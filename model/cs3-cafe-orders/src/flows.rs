//! The order-fulfilment flow (SPEC.md §6): two actors, two genuinely disjoint
//! branches, merged at the join — composed from the public model API only
//! (boundary constructors, the processes, the `Consumer` sinks) — no
//! `mint`/`defuse` — so it composes exactly the way a downstream flow would.
//!
//! **This is the concurrency case study (R9/R16).** The branches share *no*
//! resource — different actors (the trained barista vs the plain-`Person`
//! server), different equipment (machine vs till/urn), different stock — so
//! the compiler permits any interleaving of their steps. Two orderings are
//! given, with identical signatures and identical per-path end states:
//!
//! * [`fulfil_order`] interleaves the branches step by step (P1, P4, P2, P5,
//!   …) — the natural "both staff working at once" reading;
//! * [`fulfil_order_branches_in_turn`] runs branch A to completion and only
//!   then branch B — as if the barista finished before the server started.
//!
//! Both compile and both end in the identical per-path states (the
//! integration tests assert it), which is the type system proving the
//! orderings equivalent (R9). The counter-example is pinned:
//! `tests/ui/server_in_two_branches.rs` pulls the server into a branch-A step
//! while branch B holds them — "use of moved value" (E0382, F-025): the
//! resource is already in use by another process.
//!
//! **Per-branch histories (R16), exercised for real at last:** `h_a` records
//! the barista's draws (and path 3's drained-shot disposal), `h_b` the
//! server's, each under its process name; `hand_over` merges them into one
//! `Entry::Join` — a partial order claiming **no interleaving between the
//! branches**, which is the truthful record: the model never said which
//! branch's steps ran first, and neither does the history.
//!
//! **The one deliberate cross-branch dependency** is REQ-017's payment
//! receipt: `hand_over` (the join) consumes the receipt only `take_payment`
//! (branch B) can mint, so "hand over before payment" is inexpressible — the
//! only edge between the branches is the one the requirement demands.
//!
//! Three structural facts about the retry (R17), as in CS-2:
//!
//! 1. **Shared spends are drawn before the branch** (F-048): each steaming
//!    attempt's 60 000 ms is an adjacent `qualified_draw_time`, recorded into
//!    `h_a` before the fallible step runs — failure costs time too.
//! 2. **The three paths have three different types** (F-046/F-050): they end
//!    at different budgets, bottle levels and till balances, so the flow
//!    returns the `#[must_use]` outcome enum [`OrderOutcome`], one variant
//!    per path.
//! 3. **Bounded rework is bounded by provisioning** (F-050): the 300 g bottle
//!    allows exactly two 150 g draws and exactly two outcome tokens are
//!    provisioned, so a third steaming attempt is inexpressible — the second
//!    failure's only continuation is the refund path. On first-try success
//!    the untried token comes back in the variant and must be re-accounted at
//!    the boundary (`return_steam_outcome`).

use crate::characteristics::MachineTraining;
use crate::resources::boundary::restack_cup;
use crate::resources::processes::{
    brew_tea, build_flat_white, draw_grounds, draw_milk, draw_urn_water, draw_water, hand_over,
    pull_espresso, refund_flat_white, steam_milk, take_payment,
};
use crate::resources::{
    Cup, Customer, Drain, EspressoMachine, FreshTeabagBox, Hopper, KnockBox, MilkBottle,
    NineTeabags, SteamOutcome, TeabagBox, Teapot, Till, Tray, Urn, WaterTank,
};
use model_core::boundary::{send_to, take_one};
use model_core::common::Person;
use model_core::common::Qualified;
use model_core::common::processes::{draw_time, qualified_draw_time};
use model_core::history::History;
use model_core::history::processes::record;

/// What the flow hands back besides the per-path [`OrderOutcome`]: the
/// resources whose end states are the same on every path — the machine, the
/// drawn-down stock containers, the teabag box, the boundary objects and the
/// **merged** History (R16). A plain tuple alias to keep the two flow
/// signatures identical and comparable.
pub type CounterAtClose = (
    EspressoMachine,
    Hopper<482>,
    WaterTank<160>,
    Urn<700>,
    TeabagBox<NineTeabags>,
    Customer,
    Drain,
    KnockBox,
    History,
);

/// The outcome of [`fulfil_order`] (R17/F-050): the three paths of SPEC.md §6
/// produce **three different resource combinations** (different time spent,
/// different bottle level, different till balance), so the flow's result is
/// necessarily a three-way sum — there is no single converged type (F-046).
/// This is a grouping over sealed resources (a `pub` enum is legal for
/// groupings, R17): holding a variant means holding the resources, and every
/// tripwired field is still leak-protected. The served order itself went to
/// the customer inside the flow (SPEC.md P6).
#[must_use = "OrderOutcome bundles conserved outputs: every field must be accounted for"]
pub enum OrderOutcome {
    /// Path 1 — served, first-try steam (SPEC.md §6): barista 180 000 ms
    /// drawn (3 events on `h_a`), server 210 000 ms (3 on `h_b`); the bottle
    /// at 150 g; the till at 700 p; the untried second token comes back and
    /// the caller must return it to the environment (F-050).
    ServedFirstTry {
        /// The barista after 180 000 ms of the 600 000 ms budget.
        barista: Qualified<MachineTraining, 420_000>,
        /// The server after 210 000 ms of the 600 000 ms budget.
        server: Person<390_000>,
        /// The bottle after one 150 g draw.
        bottle: MilkBottle<150>,
        /// The till holding the 700 p payment.
        till: Till<700>,
        /// The provisioned-but-untried second trial — back to the
        /// environment via `return_steam_outcome` (F-050).
        reserve_outcome: SteamOutcome,
    },
    /// Path 2 — served, re-steamed (SPEC.md §6): barista 240 000 ms drawn
    /// (4 events on `h_a` — the failed attempt cost its full 60 000 ms too,
    /// R17), server 210 000 ms (3 on `h_b`); the bottle empty; 150 g of
    /// burnt milk in the drain; both tokens used; the till at 700 p.
    ServedOnRetry {
        /// The barista after 240 000 ms.
        barista: Qualified<MachineTraining, 360_000>,
        /// The server after 210 000 ms.
        server: Person<390_000>,
        /// The bottle, empty after both provisioned draws (F-050).
        bottle: MilkBottle<0>,
        /// The till holding the 700 p payment.
        till: Till<700>,
    },
    /// Path 3 — tea served, flat white refunded (SPEC.md §6): both steams
    /// fail — barista 210 000 ms, no P3 (4 events on `h_a`: three draws plus
    /// the drained stranded shot, 36 g); server 240 000 ms (4 on `h_b`: P7
    /// runs); 300 g of burnt milk in the drain; the shot's cup restacked;
    /// the till at 320 p after the 380 p refund, which left **on the tray**
    /// in the flat white's slot; the customer also holds their 300 p change.
    TeaServedFlatWhiteRefunded {
        /// The barista after 210 000 ms.
        barista: Qualified<MachineTraining, 390_000>,
        /// The server after 240 000 ms.
        server: Person<360_000>,
        /// The bottle, empty after both provisioned draws (F-050).
        bottle: MilkBottle<0>,
        /// The till after the refund: 700 − 380 = 320 p.
        till: Till<320>,
    },
}

/// The full order, **branches interleaved** (SPEC.md §6): P1, P4, P2, P5,
/// (P2 again on retry), P3, (P7 on path 3), P6 — the two staff genuinely
/// working at once, each step recorded to its own branch History under its
/// process name (R16). Every path ends with P6: the join consumes the
/// receipt (REQ-017) and merges `h_a` and `h_b` into the partial order that
/// stays with the caller.
#[allow(clippy::too_many_arguments)] // loose threading (R9/F-024): an aggregate would over-claim and delete the concurrency
pub fn fulfil_order(
    barista: Qualified<MachineTraining, 600_000>,
    server: Person<600_000>,
    machine: EspressoMachine,
    bottle: MilkBottle<300>,
    hopper: Hopper<500>,
    tank: WaterTank<200>,
    urn: Urn<1000>,
    teabags: FreshTeabagBox,
    coffee_cup: Cup,
    tea_cup: Cup,
    teapot: Teapot,
    tray: Tray,
    till: Till<0>,
    customer: Customer,
    first: SteamOutcome,
    second: SteamOutcome,
    drain: Drain,
    knock_box: KnockBox,
    h_a: History,
    h_b: History,
) -> (OrderOutcome, CounterAtClose) {
    // Branch A: P1 — pull the espresso (90 000 ms, REQ-015 at the machine).
    let (grounds, hopper) = draw_grounds::<18, 482, 500>(hopper);
    let (water, tank) = draw_water::<40, 160, 200>(tank);
    let (barista, machine, shot, coffee_cup, knock_box) =
        pull_espresso::<18, 40, 36, 22, _, _>(barista, machine, grounds, water, coffee_cup, knock_box);
    let (labour, barista) = qualified_draw_time::<90_000, 510_000, 600_000, _>(barista);
    let h_a = record(h_a, "pull_espresso", labour);

    // Branch B: P4 — take payment at the till (60 000 ms; REQ-017's key is
    // minted here; REQ-018's change goes back inside the process).
    let (cash, customer) = crate::resources::boundary::tender_cash(customer);
    let (server, till, receipt, customer) =
        take_payment::<300, 0, 700, 600_000, _>(server, till, cash, customer);
    let (labour, server) = draw_time::<60_000, 540_000, 600_000>(server);
    let h_b = record(h_b, "take_payment", labour);

    // Branch A: P2's inputs and its 60 000 ms, drawn BEFORE the branch —
    // failure costs it too (R17/F-048). The attempt consumes one 150 g
    // bottle draw and one outcome token.
    let (milk, bottle) = draw_milk::<150, 150, 300>(bottle);
    let (labour, barista) = qualified_draw_time::<60_000, 450_000, 510_000, _>(barista);
    let h_a = record(h_a, "steam_milk", labour);

    // Branch B: P5 — brew the pot of tea (120 000 ms) — while branch A's
    // milk waits at the wand: the interleaving the disjoint branches permit.
    let (teabag, teabags) = take_one(teabags);
    let (tea_water, urn) = draw_urn_water::<300, 700, 1000>(urn);
    let (server, tea) = brew_tea::<300, 303, 540_000>(server, tea_water, teabag, teapot, tea_cup);
    let (labour, server) = draw_time::<120_000, 420_000, 540_000>(server);
    let h_b = record(h_b, "brew_tea", labour);

    // Branch A: P2, attempt 1.
    match steam_milk::<150, 150, 150, 450_000, _>(barista, milk, first, drain) {
        Ok(ok) => {
            // Branch A: P3 — build the flat white (30 000 ms).
            let (barista, flat_white) =
                build_flat_white::<36, 150, 186, 450_000>(ok.barista, shot, coffee_cup, ok.milk);
            let (labour, barista) = qualified_draw_time::<30_000, 420_000, 450_000, _>(barista);
            let h_a = record(h_a, "build_flat_white", labour);

            // The join: P6 (30 000 ms, recorded to h_b BEFORE the merge).
            let (labour, server) = draw_time::<30_000, 390_000, 420_000>(server);
            let h_b = record(h_b, "hand_over", labour);
            let (server, order, history) =
                hand_over::<303, 390_000, _, _>(server, flat_white, tea, tray, receipt, h_a, h_b);
            let customer = send_to(customer, order);

            (
                OrderOutcome::ServedFirstTry {
                    barista,
                    server,
                    bottle,
                    till,
                    reserve_outcome: second,
                },
                (machine, hopper, tank, urn, teabags, customer, ok.drain, knock_box, history),
            )
        }
        Err(fail) => {
            // Branch A: P2, attempt 2 — the provisioned rework (F-050): the
            // second (and last) 150 g draw, the second token, its own
            // drawn-and-recorded labour.
            let (milk, bottle) = draw_milk::<150, 0, 150>(bottle);
            let (labour, barista) = qualified_draw_time::<60_000, 390_000, 450_000, _>(fail.barista);
            let h_a = record(h_a, "steam_milk", labour);
            match steam_milk::<150, 150, 150, 390_000, _>(barista, milk, second, fail.drain) {
                Ok(ok) => {
                    // Branch A: P3 (30 000 ms).
                    let (barista, flat_white) = build_flat_white::<36, 150, 186, 390_000>(
                        ok.barista, shot, coffee_cup, ok.milk,
                    );
                    let (labour, barista) =
                        qualified_draw_time::<30_000, 360_000, 390_000, _>(barista);
                    let h_a = record(h_a, "build_flat_white", labour);

                    // The join: P6 (30 000 ms).
                    let (labour, server) = draw_time::<30_000, 390_000, 420_000>(server);
                    let h_b = record(h_b, "hand_over", labour);
                    let (server, order, history) = hand_over::<303, 390_000, _, _>(
                        server, flat_white, tea, tray, receipt, h_a, h_b,
                    );
                    let customer = send_to(customer, order);

                    (
                        OrderOutcome::ServedOnRetry {
                            barista,
                            server,
                            bottle,
                            till,
                        },
                        (machine, hopper, tank, urn, teabags, customer, ok.drain, knock_box, history),
                    )
                }
                Err(fail2) => {
                    // Branch B: P7 — refund the flat white (30 000 ms;
                    // till 700 → 320, SPEC.md P7).
                    let (labour, server) = draw_time::<30_000, 390_000, 420_000>(server);
                    let h_b = record(h_b, "refund_flat_white", labour);
                    let (server, till, refund) = refund_flat_white::<320, 700, 390_000>(server, till);

                    // Branch A's stranded shot is drained — the disposal is
                    // branch A's fourth recorded event (R16/F-041; SPEC.md §8
                    // review decision 1) — and its cup goes back to the stack.
                    let h_a = record(h_a, "drain_stranded_shot", shot);
                    restack_cup(coffee_cup);

                    // The join: P6 (30 000 ms) with the refund in the flat
                    // white's slot (SPEC.md §6: "P7 replaces the flat white
                    // in P6").
                    let (labour, server) = draw_time::<30_000, 360_000, 390_000>(server);
                    let h_b = record(h_b, "hand_over", labour);
                    let (server, order, history) =
                        hand_over::<303, 360_000, _, _>(server, refund, tea, tray, receipt, h_a, h_b);
                    let customer = send_to(customer, order);

                    (
                        OrderOutcome::TeaServedFlatWhiteRefunded {
                            barista: fail2.barista,
                            server,
                            bottle,
                            till,
                        },
                        (machine, hopper, tank, urn, teabags, customer, fail2.drain, knock_box, history),
                    )
                }
            }
        }
    }
}

/// The other interleaving (R9, SPEC.md §6): **branch A runs to completion
/// first, then branch B** — as if the barista finished every coffee step
/// before the server lifted a finger. Nothing forces this order and nothing
/// forbids it: the branches share no resource, so both orderings type-check,
/// and every path ends in **exactly the same state** as [`fulfil_order`] —
/// the identical [`OrderOutcome`] variants and the identical merged record
/// (each branch's events in the same within-branch order), which is the
/// compiler proving the two interleavings equivalent (R9/R16).
#[allow(clippy::too_many_arguments)] // loose threading (R9/F-024): an aggregate would over-claim and delete the concurrency
pub fn fulfil_order_branches_in_turn(
    barista: Qualified<MachineTraining, 600_000>,
    server: Person<600_000>,
    machine: EspressoMachine,
    bottle: MilkBottle<300>,
    hopper: Hopper<500>,
    tank: WaterTank<200>,
    urn: Urn<1000>,
    teabags: FreshTeabagBox,
    coffee_cup: Cup,
    tea_cup: Cup,
    teapot: Teapot,
    tray: Tray,
    till: Till<0>,
    customer: Customer,
    first: SteamOutcome,
    second: SteamOutcome,
    drain: Drain,
    knock_box: KnockBox,
    h_a: History,
    h_b: History,
) -> (OrderOutcome, CounterAtClose) {
    // ---- Branch A, start to finish ----
    // P1 — pull the espresso (90 000 ms).
    let (grounds, hopper) = draw_grounds::<18, 482, 500>(hopper);
    let (water, tank) = draw_water::<40, 160, 200>(tank);
    let (barista, machine, shot, coffee_cup, knock_box) =
        pull_espresso::<18, 40, 36, 22, _, _>(barista, machine, grounds, water, coffee_cup, knock_box);
    let (labour, barista) = qualified_draw_time::<90_000, 510_000, 600_000, _>(barista);
    let h_a = record(h_a, "pull_espresso", labour);

    // P2, attempt 1 (60 000 ms, drawn before the branch, R17/F-048).
    let (milk, bottle) = draw_milk::<150, 150, 300>(bottle);
    let (labour, barista) = qualified_draw_time::<60_000, 450_000, 510_000, _>(barista);
    let h_a = record(h_a, "steam_milk", labour);
    match steam_milk::<150, 150, 150, 450_000, _>(barista, milk, first, drain) {
        Ok(ok) => {
            // P3 — build the flat white (30 000 ms): branch A done.
            let (barista, flat_white) =
                build_flat_white::<36, 150, 186, 450_000>(ok.barista, shot, coffee_cup, ok.milk);
            let (labour, barista) = qualified_draw_time::<30_000, 420_000, 450_000, _>(barista);
            let h_a = record(h_a, "build_flat_white", labour);

            // ---- Branch B, only now ----
            let (cash, customer) = crate::resources::boundary::tender_cash(customer);
            let (server, till, receipt, customer) =
                take_payment::<300, 0, 700, 600_000, _>(server, till, cash, customer);
            let (labour, server) = draw_time::<60_000, 540_000, 600_000>(server);
            let h_b = record(h_b, "take_payment", labour);
            let (teabag, teabags) = take_one(teabags);
            let (tea_water, urn) = draw_urn_water::<300, 700, 1000>(urn);
            let (server, tea) =
                brew_tea::<300, 303, 540_000>(server, tea_water, teabag, teapot, tea_cup);
            let (labour, server) = draw_time::<120_000, 420_000, 540_000>(server);
            let h_b = record(h_b, "brew_tea", labour);

            // The join: P6 (30 000 ms).
            let (labour, server) = draw_time::<30_000, 390_000, 420_000>(server);
            let h_b = record(h_b, "hand_over", labour);
            let (server, order, history) =
                hand_over::<303, 390_000, _, _>(server, flat_white, tea, tray, receipt, h_a, h_b);
            let customer = send_to(customer, order);

            (
                OrderOutcome::ServedFirstTry {
                    barista,
                    server,
                    bottle,
                    till,
                    reserve_outcome: second,
                },
                (machine, hopper, tank, urn, teabags, customer, ok.drain, knock_box, history),
            )
        }
        Err(fail) => {
            // P2, attempt 2 (the provisioned rework, F-050).
            let (milk, bottle) = draw_milk::<150, 0, 150>(bottle);
            let (labour, barista) = qualified_draw_time::<60_000, 390_000, 450_000, _>(fail.barista);
            let h_a = record(h_a, "steam_milk", labour);
            match steam_milk::<150, 150, 150, 390_000, _>(barista, milk, second, fail.drain) {
                Ok(ok) => {
                    // P3: branch A done.
                    let (barista, flat_white) = build_flat_white::<36, 150, 186, 390_000>(
                        ok.barista, shot, coffee_cup, ok.milk,
                    );
                    let (labour, barista) =
                        qualified_draw_time::<30_000, 360_000, 390_000, _>(barista);
                    let h_a = record(h_a, "build_flat_white", labour);

                    // ---- Branch B, only now ----
                    let (cash, customer) = crate::resources::boundary::tender_cash(customer);
                    let (server, till, receipt, customer) =
                        take_payment::<300, 0, 700, 600_000, _>(server, till, cash, customer);
                    let (labour, server) = draw_time::<60_000, 540_000, 600_000>(server);
                    let h_b = record(h_b, "take_payment", labour);
                    let (teabag, teabags) = take_one(teabags);
                    let (tea_water, urn) = draw_urn_water::<300, 700, 1000>(urn);
                    let (server, tea) =
                        brew_tea::<300, 303, 540_000>(server, tea_water, teabag, teapot, tea_cup);
                    let (labour, server) = draw_time::<120_000, 420_000, 540_000>(server);
                    let h_b = record(h_b, "brew_tea", labour);

                    // The join: P6 (30 000 ms).
                    let (labour, server) = draw_time::<30_000, 390_000, 420_000>(server);
                    let h_b = record(h_b, "hand_over", labour);
                    let (server, order, history) = hand_over::<303, 390_000, _, _>(
                        server, flat_white, tea, tray, receipt, h_a, h_b,
                    );
                    let customer = send_to(customer, order);

                    (
                        OrderOutcome::ServedOnRetry {
                            barista,
                            server,
                            bottle,
                            till,
                        },
                        (machine, hopper, tank, urn, teabags, customer, ok.drain, knock_box, history),
                    )
                }
                Err(fail2) => {
                    // Branch A winds up: the stranded shot is drained (the
                    // fourth h_a event, R16/F-041) and its cup restacked.
                    let h_a = record(h_a, "drain_stranded_shot", shot);
                    restack_cup(coffee_cup);

                    // ---- Branch B, only now ----
                    let (cash, customer) = crate::resources::boundary::tender_cash(customer);
                    let (server, till, receipt, customer) =
                        take_payment::<300, 0, 700, 600_000, _>(server, till, cash, customer);
                    let (labour, server) = draw_time::<60_000, 540_000, 600_000>(server);
                    let h_b = record(h_b, "take_payment", labour);
                    let (teabag, teabags) = take_one(teabags);
                    let (tea_water, urn) = draw_urn_water::<300, 700, 1000>(urn);
                    let (server, tea) =
                        brew_tea::<300, 303, 540_000>(server, tea_water, teabag, teapot, tea_cup);
                    let (labour, server) = draw_time::<120_000, 420_000, 540_000>(server);
                    let h_b = record(h_b, "brew_tea", labour);

                    // P7 — refund (30 000 ms; till 700 → 320).
                    let (labour, server) = draw_time::<30_000, 390_000, 420_000>(server);
                    let h_b = record(h_b, "refund_flat_white", labour);
                    let (server, till, refund) =
                        refund_flat_white::<320, 700, 390_000>(server, till);

                    // The join: P6 (30 000 ms) with the refund in the slot.
                    let (labour, server) = draw_time::<30_000, 360_000, 390_000>(server);
                    let h_b = record(h_b, "hand_over", labour);
                    let (server, order, history) =
                        hand_over::<303, 360_000, _, _>(server, refund, tea, tray, receipt, h_a, h_b);
                    let customer = send_to(customer, order);

                    (
                        OrderOutcome::TeaServedFlatWhiteRefunded {
                            barista: fail2.barista,
                            server,
                            bottle,
                            till,
                        },
                        (machine, hopper, tank, urn, teabags, customer, fail2.drain, knock_box, history),
                    )
                }
            }
        }
    }
}
