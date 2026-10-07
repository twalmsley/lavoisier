//! Integration tests: the full order-fulfilment flow (SPEC.md §6), all three
//! paths — served first try, served on retry, tea served with the flat white
//! refunded — through **both interleavings** of the two branches (R9: the
//! branches share nothing, so both orderings compile and the type system
//! proves them equivalent by ending in identical per-path states). Each path
//! accounts for everything at its end (R1): the order (with its path's
//! flat-white slot) at the customer, the change and any refund with the
//! customer, the till at its path's balance, both staff back with their
//! remaining budgets, the stock containers at their drawn-down levels, the
//! pucks in the knock box, the burnt milk in the drain, untried tokens
//! returned, the path-3 cup restacked — and the **merged** History with the
//! caller, asserted to have the **Join shape** (R16): branch A's and branch
//! B's event sequences intact and un-interleaved, with per-branch counts
//! 3 + 3 / 4 + 3 / 4 + 4 and per-event attribution and magnitudes.
//!
//! These tests sit OUTSIDE the crate's privacy boundary (an integration test
//! is its own crate), so nothing here can mint or defuse a resource: every
//! input comes from a boundary supplier and every output must genuinely reach
//! a consumer or a boundary exit — exactly the discipline a downstream user
//! of the model lives under.

#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![deny(let_underscore_drop)]
#![recursion_limit = "2048"]

use cs3_cafe_orders::characteristics::MachineTraining;
use cs3_cafe_orders::flows::{OrderOutcome, fulfil_order, fulfil_order_branches_in_turn};
use cs3_cafe_orders::resources::boundary::{
    bottle_back_to_fridge, fill_hopper, fill_urn, fill_water_tank, keep_hopper, keep_tank,
    keep_urn, lock_till, milk_burns, new_cup, new_customer, new_drain, new_espresso_machine,
    new_knock_box, new_teabag_box, new_teapot, new_tray, open_till, return_steam_outcome,
    steam_goes_well, stock_milk_bottle,
};
use cs3_cafe_orders::resources::{
    FLAT_WHITE_PRICE_PENCE, MilkBottle, NineTeabags, ORDER_PRICE_PENCE, SteamOutcome,
    TEA_PRICE_PENCE, TENDERED_NOTE_PENCE, TeabagBox, Till,
};
use model_core::common::Person;
use model_core::common::Qualified;
use model_core::common::boundary::{new_person, qualify};
use model_core::history::boundary::new_history;
use model_core::history::{Entry, History};

/// One expected recorded event: (process, item, magnitude, unit).
type Expected = (&'static str, &'static str, u64, &'static str);

/// Checks that one branch's record inside the join is a **flat, in-order
/// sequence of plain events** — no invented interleaving, no nested joins —
/// with the expected attribution, items, magnitudes and units (R16).
fn assert_branch(entries: &[Entry], expected: &[Expected], branch: &str) {
    assert_eq!(
        entries.len(),
        expected.len(),
        "branch {branch}: wrong event count"
    );
    for (entry, (process, item, magnitude, unit)) in entries.iter().zip(expected) {
        match entry {
            Entry::Event(e) => {
                assert_eq!(
                    (e.process, e.item, e.magnitude, e.unit),
                    (*process, *item, *magnitude, *unit),
                    "branch {branch}: wrong event"
                );
            }
            other => panic!(
                "branch {branch}: expected a flat, un-interleaved event sequence, got {other:?}"
            ),
        }
    }
}

/// Checks the merged History at a path's end (R16, SPEC.md §6): exactly one
/// top-level `Entry::Join` — the partial order — holding branch A's and
/// branch B's event sequences intact, in their own within-branch order, with
/// **no interleaving claimed between them**.
fn assert_join_shape(history: &History, branch_a: &[Expected], branch_b: &[Expected]) {
    assert_eq!(history.event_count(), branch_a.len() + branch_b.len());
    match history.entries() {
        [Entry::Join(a, b)] => {
            assert_branch(a, branch_a, "A (barista)");
            assert_branch(b, branch_b, "B (server)");
        }
        other => panic!("expected exactly one top-level Join (R16), got {other:?}"),
    }
}

/// Branch A's expected record on the served paths' first attempt prefix.
const MS: &str = "person-milliseconds";

/// Runs one of the two flow orderings over fresh boundary state with the
/// given steam outcomes, so every test and the interleaving-equivalence
/// comparisons drive identical inputs.
#[allow(clippy::type_complexity)] // the flow's public signature, spelled once
fn run(
    flow: fn(
        Qualified<MachineTraining, 600_000>,
        Person<600_000>,
        cs3_cafe_orders::resources::EspressoMachine,
        MilkBottle<300>,
        cs3_cafe_orders::resources::Hopper<500>,
        cs3_cafe_orders::resources::WaterTank<200>,
        cs3_cafe_orders::resources::Urn<1000>,
        cs3_cafe_orders::resources::FreshTeabagBox,
        cs3_cafe_orders::resources::Cup,
        cs3_cafe_orders::resources::Cup,
        cs3_cafe_orders::resources::Teapot,
        cs3_cafe_orders::resources::Tray,
        Till<0>,
        cs3_cafe_orders::resources::Customer,
        SteamOutcome,
        SteamOutcome,
        cs3_cafe_orders::resources::Drain,
        cs3_cafe_orders::resources::KnockBox,
        History,
        History,
    ) -> (OrderOutcome, cs3_cafe_orders::flows::CounterAtClose),
    first: SteamOutcome,
    second: SteamOutcome,
) -> (OrderOutcome, cs3_cafe_orders::flows::CounterAtClose) {
    flow(
        qualify::<MachineTraining, 600_000>(new_person::<600_000>()),
        new_person::<600_000>(),
        new_espresso_machine(),
        stock_milk_bottle(),
        fill_hopper(),
        fill_water_tank(),
        fill_urn(),
        new_teabag_box(),
        new_cup(),
        new_cup(),
        new_teapot(),
        new_tray(),
        open_till(),
        new_customer(),
        first,
        second,
        new_drain(),
        new_knock_box(),
        // One History per concurrent branch (R16), created at the boundary.
        new_history(),
        new_history(),
    )
}

/// Disposes of the common counter state every path ends with (R1): the
/// drawn-down containers go back through their boundary exits, the reusables
/// and boundary objects stay with the caller, and the merged History is
/// returned for the join-shape assertions.
fn close_counter(counter: cs3_cafe_orders::flows::CounterAtClose) -> History {
    let (machine, hopper, tank, urn, teabags, customer, drain, knock_box, history) = counter;
    // Stock at its drawn-down levels (SPEC.md §6): hopper 500 − 18, tank
    // 200 − 40, urn 1000 − 300 — the types say so; the exits account for
    // them.
    keep_hopper::<482>(hopper);
    keep_tank::<160>(tank);
    keep_urn::<700>(urn);
    let _nine_left: TeabagBox<NineTeabags> = teabags;
    let _boundary_objects_stay = (machine, customer, drain, knock_box);
    history
}

/// Path 1 — served, first-try steam (SPEC.md §6): barista 180 000 ms drawn,
/// server 210 000 ms; bottle at 150 g; one untried token returned; till at
/// 700 p; the order (flat white + tea + tray) and the 300 p change with the
/// customer. Merged record: Join of 3 + 3 attributed events.
///
/// Verifies: REQ-015, REQ-016, REQ-017, REQ-018
#[test]
fn path_1_served_first_try_accounts_for_everything() {
    let (out, counter) = run(fulfil_order, steam_goes_well(), milk_burns());
    match out {
        OrderOutcome::ServedFirstTry {
            barista,
            server,
            bottle,
            till,
            reserve_outcome,
        } => {
            // Staff back with their remaining budgets (the types say so).
            let _barista: Qualified<MachineTraining, 420_000> = barista;
            let _server: Person<390_000> = server;
            // The bottle kept its second provisioned draw (F-050).
            bottle_back_to_fridge::<150>(bottle);
            // The till holds exactly the order price (R19).
            assert_eq!(Till::<700>::VALUE, ORDER_PRICE_PENCE);
            lock_till::<700>(till);
            // The untried token is re-accounted at the boundary (F-050).
            return_steam_outcome(reserve_outcome);
            // The customer holds the order and their 300 p change
            // (unbounded sinks discard, F-029: asserted arithmetically, R7).
            assert_eq!(TENDERED_NOTE_PENCE - ORDER_PRICE_PENCE, 300);
            // The merged record is a partial order (R16): one Join, each
            // branch's events intact and un-interleaved — 3 + 3.
            let history = close_counter(counter);
            assert_join_shape(
                &history,
                &[
                    ("pull_espresso", "Labour", 90_000, MS),
                    ("steam_milk", "Labour", 60_000, MS),
                    ("build_flat_white", "Labour", 30_000, MS),
                ],
                &[
                    ("take_payment", "Labour", 60_000, MS),
                    ("brew_tea", "Labour", 120_000, MS),
                    ("hand_over", "Labour", 30_000, MS),
                ],
            );
            let _execution_record = history;
        }
        _ => unreachable!("a success first trial must take the first-try path"),
    }
}

/// Path 2 — served, re-steamed (SPEC.md §6): barista 240 000 ms (the failed
/// attempt cost its full 60 000 ms too, R17), server 210 000 ms; bottle
/// empty; 150 g of burnt milk in the drain; both tokens used; till at 700 p.
/// Merged record: Join of 4 + 3 attributed events.
///
/// Verifies: REQ-015, REQ-016, REQ-017, REQ-018
#[test]
fn path_2_served_on_retry_accounts_for_everything() {
    let (out, counter) = run(fulfil_order, milk_burns(), steam_goes_well());
    match out {
        OrderOutcome::ServedOnRetry {
            barista,
            server,
            bottle,
            till,
        } => {
            let _barista: Qualified<MachineTraining, 360_000> = barista;
            let _server: Person<390_000> = server;
            // Both provisioned draws spent: a third attempt is
            // inexpressible (F-050).
            bottle_back_to_fridge::<0>(bottle);
            // 150 g of burnt milk reached the drain inside P2 (REQ-016; the
            // unbounded sink discards, F-029, so the mass is recovered from
            // the constants, R7).
            assert_eq!(MilkBottle::<300>::VALUE - 150, 150);
            lock_till::<700>(till);
            let history = close_counter(counter);
            assert_join_shape(
                &history,
                &[
                    ("pull_espresso", "Labour", 90_000, MS),
                    ("steam_milk", "Labour", 60_000, MS),
                    ("steam_milk", "Labour", 60_000, MS),
                    ("build_flat_white", "Labour", 30_000, MS),
                ],
                &[
                    ("take_payment", "Labour", 60_000, MS),
                    ("brew_tea", "Labour", 120_000, MS),
                    ("hand_over", "Labour", 30_000, MS),
                ],
            );
            let _execution_record = history;
        }
        _ => unreachable!("fail-then-success must take the retry path"),
    }
}

/// Path 3 — tea served, flat white refunded (SPEC.md §6): both steams fail —
/// barista 210 000 ms (no P3), server 240 000 ms (P7 runs); 300 g of burnt
/// milk drained; the stranded shot drained with its cup restacked (review
/// decision 1); till at 320 p; the customer holds the tea, their 300 p
/// change **and** the 380 p refund (on the tray, in the flat white's slot).
/// Merged record: Join of 4 + 4 — branch A's fourth event is the drained
/// shot (36 g), branch B's is P7.
///
/// Verifies: REQ-015, REQ-016, REQ-017, REQ-018
#[test]
fn path_3_refund_accounts_for_everything() {
    let (out, counter) = run(fulfil_order, milk_burns(), milk_burns());
    match out {
        OrderOutcome::TeaServedFlatWhiteRefunded {
            barista,
            server,
            bottle,
            till,
        } => {
            let _barista: Qualified<MachineTraining, 390_000> = barista;
            let _server: Person<360_000> = server;
            bottle_back_to_fridge::<0>(bottle);
            // The till gave the flat white's price back (REQ-018's change
            // was already returned in full at P4): 700 = 320 + 380.
            assert_eq!(Till::<320>::VALUE + FLAT_WHITE_PRICE_PENCE, ORDER_PRICE_PENCE);
            lock_till::<320>(till);
            // The customer's holdings (arithmetic, F-029): the tea service,
            // 300 p change, 380 p refund — and the tea's price is what the
            // till kept.
            assert_eq!(ORDER_PRICE_PENCE - FLAT_WHITE_PRICE_PENCE, TEA_PRICE_PENCE);
            // 300 g of burnt milk reached the drain across the two failed
            // attempts (REQ-016), and the stranded 36 g shot was drained
            // with its cup restacked — its disposal is branch A's fourth
            // recorded event, below.
            let history = close_counter(counter);
            assert_join_shape(
                &history,
                &[
                    ("pull_espresso", "Labour", 90_000, MS),
                    ("steam_milk", "Labour", 60_000, MS),
                    ("steam_milk", "Labour", 60_000, MS),
                    ("drain_stranded_shot", "EspressoShot", 36, "grams"),
                ],
                &[
                    ("take_payment", "Labour", 60_000, MS),
                    ("brew_tea", "Labour", 120_000, MS),
                    ("refund_flat_white", "Labour", 30_000, MS),
                    ("hand_over", "Labour", 30_000, MS),
                ],
            );
            let _execution_record = history;
        }
        _ => unreachable!("fail-then-fail must take the refund path"),
    }
}

/// **The concurrency demonstration** (R9/R16): the other interleaving —
/// branch A run to completion before branch B even starts — compiles and
/// ends path 1 in the **identical** state: the same variant, the same typed
/// budgets, bottle and till, and the same merged record (each branch's
/// events in the same within-branch order). The compiler permitted both
/// orderings because the branches share nothing; the histories claim no
/// order between them either.
///
/// Verifies: REQ-015, REQ-017
#[test]
fn interleavings_are_equivalent_on_path_1() {
    let (out_a, counter_a) = run(fulfil_order, steam_goes_well(), milk_burns());
    let (out_b, counter_b) = run(fulfil_order_branches_in_turn, steam_goes_well(), milk_burns());
    let expected_a: &[Expected] = &[
        ("pull_espresso", "Labour", 90_000, MS),
        ("steam_milk", "Labour", 60_000, MS),
        ("build_flat_white", "Labour", 30_000, MS),
    ];
    let expected_b: &[Expected] = &[
        ("take_payment", "Labour", 60_000, MS),
        ("brew_tea", "Labour", 120_000, MS),
        ("hand_over", "Labour", 30_000, MS),
    ];
    for out in [out_a, out_b] {
        match out {
            OrderOutcome::ServedFirstTry {
                barista,
                server,
                bottle,
                till,
                reserve_outcome,
            } => {
                // Identical end state: the same types check in both runs.
                let _barista: Qualified<MachineTraining, 420_000> = barista;
                let _server: Person<390_000> = server;
                bottle_back_to_fridge::<150>(bottle);
                lock_till::<700>(till);
                return_steam_outcome(reserve_outcome);
            }
            _ => unreachable!("both orderings must take the first-try path"),
        }
    }
    for counter in [counter_a, counter_b] {
        let history = close_counter(counter);
        // Identical merged records: same Join shape, same per-branch
        // sequences — the partial order never claimed a cross-branch order,
        // so reordering the branches changed nothing (R16).
        assert_join_shape(&history, expected_a, expected_b);
        let _execution_record = history;
    }
}

/// The interleaving equivalence on the worst path (R9): both orderings of
/// path 3 — P7 and the drained shot fall at different places in the source,
/// yet the end states and the merged records are identical.
///
/// Verifies: REQ-016, REQ-017, REQ-018
#[test]
fn interleavings_are_equivalent_on_path_3() {
    let (out_a, counter_a) = run(fulfil_order, milk_burns(), milk_burns());
    let (out_b, counter_b) = run(fulfil_order_branches_in_turn, milk_burns(), milk_burns());
    for out in [out_a, out_b] {
        match out {
            OrderOutcome::TeaServedFlatWhiteRefunded {
                barista,
                server,
                bottle,
                till,
            } => {
                let _barista: Qualified<MachineTraining, 390_000> = barista;
                let _server: Person<360_000> = server;
                bottle_back_to_fridge::<0>(bottle);
                lock_till::<320>(till);
            }
            _ => unreachable!("both orderings must take the refund path"),
        }
    }
    for counter in [counter_a, counter_b] {
        let history = close_counter(counter);
        assert_join_shape(
            &history,
            &[
                ("pull_espresso", "Labour", 90_000, MS),
                ("steam_milk", "Labour", 60_000, MS),
                ("steam_milk", "Labour", 60_000, MS),
                ("drain_stranded_shot", "EspressoShot", 36, "grams"),
            ],
            &[
                ("take_payment", "Labour", 60_000, MS),
                ("brew_tea", "Labour", 120_000, MS),
                ("refund_flat_white", "Labour", 30_000, MS),
                ("hand_over", "Labour", 30_000, MS),
            ],
        );
        let _execution_record = history;
    }
}

/// The downstream fixture path (R1, F-004): the crate's own `test-support`
/// feature — enabled only through the `[dev-dependencies]` self
/// re-declaration — lets this test conjure a sealed till without running the
/// payment. The fixture still has to be accounted for like a real resource:
/// the refund draws it down under the standard R15 conservation assert and
/// the till leaves through its boundary exit.
#[test]
fn fixtures_construct_sealed_resources_for_downstream_tests() {
    let till = Till::<700>::test_fixture();
    let server = new_person::<600_000>();
    let (server, till, refund) =
        cs3_cafe_orders::resources::processes::refund_flat_white::<320, 700, 600_000>(server, till);
    // The refund is real money (R19) and must be accounted for: the customer
    // is its only exit (REQ-018's shape).
    let customer = model_core::boundary::send_to(new_customer(), refund);
    lock_till::<320>(till);
    let _accounted = (server, customer);
}

/// The tripwire demonstration (R1 layer 2, F-008, F-032) from outside the
/// privacy boundary: a fixture espresso shot that is neither built into the
/// flat white nor recorded as drained is named and used, so `must_use` and
/// `unused_variables` are both satisfied — no compile-time layer can catch
/// this leak; the tripwire `Drop` converts it into this test failure.
#[test]
#[should_panic(expected = "dropped without being consumed")]
fn abandoned_espresso_shot_trips_the_tripwire_downstream() {
    let shot = cs3_cafe_orders::resources::EspressoShot::<36>::test_fixture();
    let _never_served_never_drained = shot;
}
