//! The repair flow (SPEC.md §6): a once-retried fallible patch with the
//! spare-tube fallback, composed from the public model API only (boundary
//! constructors, the processes, the `Consumer` sinks) — no `mint`/`defuse` —
//! so it composes exactly the way a downstream flow would.
//!
//! Three structural facts about this flow (R17):
//!
//! 1. **Shared spends are drawn before the branch** (F-048): each attempt's
//!    labour is an adjacent `qualified_draw_time` in the flow, recorded into
//!    the `History` (R16) before the fallible step runs — so the member
//!    returns at the same budget in both arms and failure costs time too.
//! 2. **The three paths have three different types** (F-046/F-050): patched
//!    first try, patched on retry and spare fitted end with different wheel
//!    masses (2083/2083/2080 — the first two differ in budget and kit),
//!    different budgets and different wallet states, so the flow returns the
//!    `#[must_use]` outcome enum [`RepairOutcome`], one variant per path.
//! 3. **Bounded rework is bounded by provisioning** (F-050): the kit holds
//!    exactly two patches and exactly two outcome tokens are provisioned, so
//!    a third patch attempt is inexpressible — the second failure's only
//!    continuation is the spare-tube purchase. On first-try success the
//!    untried token comes back in the variant and must be re-accounted at
//!    the boundary (`return_patch_outcome`).
//!
//! **Ordering freedom is deliberately minimal** (SPEC.md §6: one actor, a
//! linear repair — CS-2's stress is fallibility, not concurrency). The
//! statically expressible freedoms are trivia such as when the wallet is
//! readied: [`repair_wheel`] draws the cash only inside the second-failure
//! arm (and retires the dead tube after the purchase), while
//! [`repair_wheel_wallet_ready_early`] draws the cash before the repair even
//! starts and retires the dead tube before the purchase — both compile and
//! both end in the identical per-path states, which is the type system
//! proving the orderings equivalent (R9). Buying the spare *before* the
//! second attempt has failed is **not** statically expressible as a complete
//! flow: if that attempt then succeeded, the unused spare's only consumer is
//! `refit_and_inflate`, which the patched tube already claims — the spare
//! would be a dead end, so the type system rejects the pessimistic purchase,
//! truthfully.

use crate::characteristics::Induction;
use crate::resources::processes::{
    buy_spare, check_patch, deposit, draw_cash, draw_cement, find_hole, open_wheel, patch_tube,
    refit_and_inflate, retire_tube,
};
use crate::resources::{
    FreshPatchKit, NoPatches, OnePatch, PartsCounter, PatchKit, PatchOutcome, Pump,
    PuncturedWheel, RubberRecycling, ServiceableWheel, Wallet, WasteStream, Workstand,
};
use model_core::boundary::{send_to, take_one};
use model_core::common::Qualified;
use model_core::common::processes::qualified_draw_time;
use model_core::history::History;
use model_core::history::processes::record;

/// The outcome of [`repair_wheel`] (R17/F-050): the three paths of SPEC.md §6
/// produce **three different resource combinations** (different time spent,
/// different kit remainder, different wallet, different wheel mass), so the
/// flow's result is necessarily a three-way sum — there is no single
/// converged type (F-046). This is a grouping over sealed resources (a `pub`
/// enum is legal for groupings, R17): holding a variant means holding the
/// resources, and every tripwired field is still leak-protected.
#[must_use = "RepairOutcome bundles conserved outputs: every field must be accounted for"]
pub enum RepairOutcome {
    /// Path 1 — patched first try (SPEC.md §6): 900 000 ms drawn (5
    /// attributed events); the wallet untouched; 1 spare patch and 29 g of
    /// cement left in the kit; the untried second token comes back and the
    /// caller must return it to the environment (F-050).
    PatchedFirstTry {
        /// The member after 900 000 ms of the 1 800 000 ms budget.
        member: Qualified<Induction, 900_000>,
        /// The workstand, back (R2).
        workstand: Workstand,
        /// The pump, back (R2).
        pump: Pump,
        /// The serviceable wheel at the patched mass (1900 + 183).
        wheel: ServiceableWheel<2083>,
        /// The kit, back with its spare patch and 29 g of cement.
        kit: PatchKit<OnePatch, 29>,
        /// The wallet, untouched.
        wallet: Wallet<1000>,
        /// The provisioned-but-untried second trial — back to the
        /// environment via `return_patch_outcome` (F-050).
        reserve_outcome: PatchOutcome,
    },
    /// Path 2 — patched on retry (SPEC.md §6): 1 020 000 ms drawn (6
    /// events); 1 spent patch (3 g) in the waste stream; the kit empty of
    /// patches with 28 g of cement; the wallet untouched.
    PatchedOnRetry {
        /// The member after 1 020 000 ms.
        member: Qualified<Induction, 780_000>,
        /// The workstand, back (R2).
        workstand: Workstand,
        /// The pump, back (R2).
        pump: Pump,
        /// The serviceable wheel at the patched mass (1900 + 183).
        wheel: ServiceableWheel<2083>,
        /// The kit, out of patches, with 28 g of cement.
        kit: PatchKit<NoPatches, 28>,
        /// The wallet, untouched.
        wallet: Wallet<1000>,
    },
    /// Path 3 — spare fitted (SPEC.md §6): 1 080 000 ms drawn (6 events);
    /// 6 g of spent patches in the waste stream; the dead tube at rubber
    /// recycling; 350 p left in the wallet; the wheel at the spare's mass.
    SpareFitted {
        /// The member after 1 080 000 ms.
        member: Qualified<Induction, 720_000>,
        /// The workstand, back (R2).
        workstand: Workstand,
        /// The pump, back (R2).
        pump: Pump,
        /// The serviceable wheel at the spare's mass (1900 + 180).
        wheel: ServiceableWheel<2080>,
        /// The kit, out of patches, with 28 g of cement.
        kit: PatchKit<NoPatches, 28>,
        /// The wallet after the purchase: 1000 − 650 = 350 p.
        wallet: Wallet<350>,
    },
}

/// The full repair (SPEC.md §6): P1 → P2 → P3; P3-Ok → P4 → P6;
/// P3-Fail → second P3; second-P3-Fail → P5 → P6. Every path ends with P6
/// and a serviceable wheel. Each attempt's 120 000 ms is drawn and recorded
/// *before* its branch (R17/F-048), so the member's budget is honest in both
/// arms; the rework bound is the provisioning — two patches in the kit, two
/// tokens (F-050). The wallet is readied only when path 3 needs it; see
/// [`repair_wheel_wallet_ready_early`] for the other statically expressible
/// ordering.
#[allow(clippy::too_many_arguments)] // loose threading (R9/F-024): an aggregate would over-claim
pub fn repair_wheel(
    member: Qualified<Induction, 1_800_000>,
    workstand: Workstand,
    pump: Pump,
    wheel: PuncturedWheel,
    kit: FreshPatchKit,
    wallet: Wallet<1000>,
    first: PatchOutcome,
    second: PatchOutcome,
    counter: PartsCounter,
    waste: WasteStream,
    recycling: RubberRecycling,
    history: History,
) -> (
    RepairOutcome,
    PartsCounter,
    WasteStream,
    RubberRecycling,
    History,
) {
    // P1 — open the wheel (240 000 ms, REQ-010 on the workstand).
    let (member, workstand, open, p_tube) = open_wheel(member, workstand, wheel);
    let (labour, member) = qualified_draw_time::<240_000, 1_560_000, 1_800_000, _>(member);
    let history = record(history, "open_wheel", labour);

    // P2 — find the hole (180 000 ms) — enables REQ-011.
    let (member, pump, located) = find_hole(member, pump, p_tube);
    let (labour, member) = qualified_draw_time::<180_000, 1_380_000, 1_560_000, _>(member);
    let history = record(history, "find_hole", labour);

    // P3, attempt 1 (120 000 ms, drawn BEFORE the branch — failure costs it
    // too, R17/F-048). The attempt consumes one patch and 1 g of cement,
    // both through the kit's boundary, and one outcome token.
    let (labour, member) = qualified_draw_time::<120_000, 1_260_000, 1_380_000, _>(member);
    let history = record(history, "patch_tube", labour);
    let (patch, kit) = take_one(kit);
    let (cement, kit) = draw_cement::<1, 29, 30, _>(kit);
    match patch_tube::<1, 183, 3, 1_260_000, _, _>(member, located, patch, cement, first, waste) {
        Ok(ok) => {
            // P4 — check the patch (60 000 ms) — enables REQ-012.
            let (member, pump, checked) = check_patch(ok.member, pump, ok.tube);
            let (labour, member) = qualified_draw_time::<60_000, 1_200_000, 1_260_000, _>(member);
            let history = record(history, "check_patch", labour);

            // P6 — refit and inflate (300 000 ms; REQ-010 + REQ-012):
            // 1900 + 183 = 2083.
            let (member, workstand, pump, wheel) =
                refit_and_inflate::<2083, _, _>(member, workstand, pump, open, checked);
            let (labour, member) = qualified_draw_time::<300_000, 900_000, 1_200_000, _>(member);
            let history = record(history, "refit_and_inflate", labour);

            (
                RepairOutcome::PatchedFirstTry {
                    member,
                    workstand,
                    pump,
                    wheel,
                    kit,
                    wallet,
                    reserve_outcome: second,
                },
                counter,
                ok.waste,
                recycling,
                history,
            )
        }
        Err(fail) => {
            // P3, attempt 2: consumes the provisioned reserves — the second
            // patch, another 1 g of cement, the second token (the rework
            // bound, F-050) — and costs its own drawn-and-recorded labour.
            let (labour, member) = qualified_draw_time::<120_000, 1_140_000, 1_260_000, _>(fail.member);
            let history = record(history, "patch_tube", labour);
            let (patch, kit) = take_one(kit);
            let (cement, kit) = draw_cement::<1, 28, 29, _>(kit);
            match patch_tube::<1, 183, 3, 1_140_000, _, _>(
                member, fail.tube, patch, cement, second, fail.waste,
            ) {
                Ok(ok) => {
                    // P4 — check the patch (60 000 ms).
                    let (member, pump, checked) = check_patch(ok.member, pump, ok.tube);
                    let (labour, member) =
                        qualified_draw_time::<60_000, 1_080_000, 1_140_000, _>(member);
                    let history = record(history, "check_patch", labour);

                    // P6 — refit and inflate (300 000 ms): 1900 + 183 = 2083.
                    let (member, workstand, pump, wheel) =
                        refit_and_inflate::<2083, _, _>(member, workstand, pump, open, checked);
                    let (labour, member) =
                        qualified_draw_time::<300_000, 780_000, 1_080_000, _>(member);
                    let history = record(history, "refit_and_inflate", labour);

                    (
                        RepairOutcome::PatchedOnRetry {
                            member,
                            workstand,
                            pump,
                            wheel,
                            kit,
                            wallet,
                        },
                        counter,
                        ok.waste,
                        recycling,
                        history,
                    )
                }
                Err(fail2) => {
                    // P5 — buy a spare (120 000 ms; REQ-014): the wallet is
                    // readied here, the cash splits 1000 = 650 + 350, the
                    // change goes straight back in.
                    let (cash, wallet) = draw_cash::<1000, 0, 1000>(wallet);
                    let (member, spare, change, counter) =
                        buy_spare::<1000, 350, 1_140_000, _>(fail2.member, counter, cash);
                    let (labour, member) =
                        qualified_draw_time::<120_000, 1_020_000, 1_140_000, _>(member);
                    let history = record(history, "buy_spare", labour);
                    let wallet = deposit::<350, 0, 350>(wallet, change);

                    // The dead tube goes to rubber recycling (flow-routed,
                    // SPEC.md P5): giving up is an explicit state change.
                    let recycling = send_to(recycling, retire_tube(fail2.tube));

                    // P6 — refit and inflate (300 000 ms): 1900 + 180 = 2080.
                    let (member, workstand, pump, wheel) =
                        refit_and_inflate::<2080, _, _>(member, workstand, pump, open, spare);
                    let (labour, member) =
                        qualified_draw_time::<300_000, 720_000, 1_020_000, _>(member);
                    let history = record(history, "refit_and_inflate", labour);

                    (
                        RepairOutcome::SpareFitted {
                            member,
                            workstand,
                            pump,
                            wheel,
                            kit,
                            wallet,
                        },
                        counter,
                        fail2.waste,
                        recycling,
                        history,
                    )
                }
            }
        }
    }
}

/// The other statically expressible ordering (SPEC.md §6: "trivia such as
/// when the wallet is readied"): the cash is drawn **before the repair even
/// starts** and carried through the flow, and on path 3 the dead tube is
/// retired to recycling **before** the purchase. On the patched paths the
/// un-spent cash is deposited back whole (1000 = 0 + 1000, the conserving
/// combine), so every path ends in **exactly the same state** as
/// [`repair_wheel`] — the identical [`RepairOutcome`] variants type-check,
/// which is the compiler proving the two orderings equivalent (R9).
#[allow(clippy::too_many_arguments)] // loose threading (R9/F-024): an aggregate would over-claim
pub fn repair_wheel_wallet_ready_early(
    member: Qualified<Induction, 1_800_000>,
    workstand: Workstand,
    pump: Pump,
    wheel: PuncturedWheel,
    kit: FreshPatchKit,
    wallet: Wallet<1000>,
    first: PatchOutcome,
    second: PatchOutcome,
    counter: PartsCounter,
    waste: WasteStream,
    recycling: RubberRecycling,
    history: History,
) -> (
    RepairOutcome,
    PartsCounter,
    WasteStream,
    RubberRecycling,
    History,
) {
    // The wallet is readied first (the ordering trivium): the cash travels
    // with the flow and is re-deposited whole if path 3 never happens.
    let (cash, wallet) = draw_cash::<1000, 0, 1000>(wallet);

    // P1 — open the wheel (240 000 ms).
    let (member, workstand, open, p_tube) = open_wheel(member, workstand, wheel);
    let (labour, member) = qualified_draw_time::<240_000, 1_560_000, 1_800_000, _>(member);
    let history = record(history, "open_wheel", labour);

    // P2 — find the hole (180 000 ms).
    let (member, pump, located) = find_hole(member, pump, p_tube);
    let (labour, member) = qualified_draw_time::<180_000, 1_380_000, 1_560_000, _>(member);
    let history = record(history, "find_hole", labour);

    // P3, attempt 1 (120 000 ms, drawn before the branch, R17/F-048).
    let (labour, member) = qualified_draw_time::<120_000, 1_260_000, 1_380_000, _>(member);
    let history = record(history, "patch_tube", labour);
    let (patch, kit) = take_one(kit);
    let (cement, kit) = draw_cement::<1, 29, 30, _>(kit);
    match patch_tube::<1, 183, 3, 1_260_000, _, _>(member, located, patch, cement, first, waste) {
        Ok(ok) => {
            // The unspent cash goes back in the wallet whole (R3 combine).
            let wallet = deposit::<1000, 0, 1000>(wallet, cash);

            // P4 — check (60 000 ms), then P6 — refit (300 000 ms).
            let (member, pump, checked) = check_patch(ok.member, pump, ok.tube);
            let (labour, member) = qualified_draw_time::<60_000, 1_200_000, 1_260_000, _>(member);
            let history = record(history, "check_patch", labour);
            let (member, workstand, pump, wheel) =
                refit_and_inflate::<2083, _, _>(member, workstand, pump, open, checked);
            let (labour, member) = qualified_draw_time::<300_000, 900_000, 1_200_000, _>(member);
            let history = record(history, "refit_and_inflate", labour);

            (
                RepairOutcome::PatchedFirstTry {
                    member,
                    workstand,
                    pump,
                    wheel,
                    kit,
                    wallet,
                    reserve_outcome: second,
                },
                counter,
                ok.waste,
                recycling,
                history,
            )
        }
        Err(fail) => {
            // P3, attempt 2 (the provisioned retry, F-050).
            let (labour, member) = qualified_draw_time::<120_000, 1_140_000, 1_260_000, _>(fail.member);
            let history = record(history, "patch_tube", labour);
            let (patch, kit) = take_one(kit);
            let (cement, kit) = draw_cement::<1, 28, 29, _>(kit);
            match patch_tube::<1, 183, 3, 1_140_000, _, _>(
                member, fail.tube, patch, cement, second, fail.waste,
            ) {
                Ok(ok) => {
                    let wallet = deposit::<1000, 0, 1000>(wallet, cash);
                    let (member, pump, checked) = check_patch(ok.member, pump, ok.tube);
                    let (labour, member) =
                        qualified_draw_time::<60_000, 1_080_000, 1_140_000, _>(member);
                    let history = record(history, "check_patch", labour);
                    let (member, workstand, pump, wheel) =
                        refit_and_inflate::<2083, _, _>(member, workstand, pump, open, checked);
                    let (labour, member) =
                        qualified_draw_time::<300_000, 780_000, 1_080_000, _>(member);
                    let history = record(history, "refit_and_inflate", labour);

                    (
                        RepairOutcome::PatchedOnRetry {
                            member,
                            workstand,
                            pump,
                            wheel,
                            kit,
                            wallet,
                        },
                        counter,
                        ok.waste,
                        recycling,
                        history,
                    )
                }
                Err(fail2) => {
                    // The other ordering trivium: the dead tube is retired
                    // BEFORE the purchase this time.
                    let recycling = send_to(recycling, retire_tube(fail2.tube));

                    // P5 — buy the spare with the cash readied at the start.
                    let (member, spare, change, counter) =
                        buy_spare::<1000, 350, 1_140_000, _>(fail2.member, counter, cash);
                    let (labour, member) =
                        qualified_draw_time::<120_000, 1_020_000, 1_140_000, _>(member);
                    let history = record(history, "buy_spare", labour);
                    let wallet = deposit::<350, 0, 350>(wallet, change);

                    // P6 — refit with the spare: 1900 + 180 = 2080.
                    let (member, workstand, pump, wheel) =
                        refit_and_inflate::<2080, _, _>(member, workstand, pump, open, spare);
                    let (labour, member) =
                        qualified_draw_time::<300_000, 720_000, 1_020_000, _>(member);
                    let history = record(history, "refit_and_inflate", labour);

                    (
                        RepairOutcome::SpareFitted {
                            member,
                            workstand,
                            pump,
                            wheel,
                            kit,
                            wallet,
                        },
                        counter,
                        fail2.waste,
                        recycling,
                        history,
                    )
                }
            }
        }
    }
}
