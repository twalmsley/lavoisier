//! Integration tests: the full cut -> drill -> fasten flow, composed in two
//! different valid orders (R9: the model describes connections, not
//! sequences), with everything accounted for (R1) — swarf to the bin and on
//! to disposal, labour to the boundary ledger, the assembly to the customer,
//! reusable resources back to the caller. Plus the tripwire demonstration for
//! abandoned swarf (R1 layer 2, F-008) and the downstream fixture path
//! (F-004).
//!
//! These tests sit OUTSIDE the pilot's privacy boundary (an integration test
//! is its own crate), so nothing here can mint or defuse a resource: every
//! input comes from a boundary supplier and every output must genuinely reach
//! a consumer — exactly the discipline a downstream user of the model lives
//! under.

#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![deny(let_underscore_drop)]
#![recursion_limit = "2048"]

use model_core::boundary::send_to;
use model_core::common::Person;
use model_core::common::boundary::new_person;
use model_core::fixtures::new_test_sink;
use model_core::nat::aliases::{N1, N3, N4};
use model_core::list::{Cons, Nil};
use pilot_workshop::catalogue::boundary::full_box;
use pilot_workshop::catalogue::{Bolt, EmptyBoltBox, FasteningBolt, FourOf};
use pilot_workshop::resources::boundary::{
    dispose_bin, new_customer, new_swarf_bin, supply_drill, supply_sheet,
};
use pilot_workshop::resources::processes::{cut, discard_swarf, drill_holes, fasten};
use pilot_workshop::resources::{Assembly, Swarf};

/// Flow order A: cut, drill plate 1, drill plate 2, fasten, then account for
/// all waste at the end. Loose threading of the person and the drill (R2,
/// F-024); the time budget's running balance is restated at every call and
/// checked by the compiler (R15, F-030).
///
/// Verifies: REQ-001, REQ-002, REQ-003
#[test]
fn flow_order_a_type_checks_and_accounts_for_everything() {
    let sheet = supply_sheet::<2000>();
    let person = new_person::<10_000>();
    let drill = supply_drill();
    let bolts = full_box::<FasteningBolt, N4>();
    let bin = new_swarf_bin::<N3>();
    let sink = new_test_sink();

    let (p1, p2, cut_swarf) = cut::<2000, 900, 900, 200>(sheet);
    let (person, drill, d1, s1, l1) =
        drill_holes::<2000, 8000, 10_000, 900, 880, 20>(person, drill, p1);
    let (person, drill, d2, s2, l2) =
        drill_holes::<2000, 6000, 8000, 900, 880, 20>(person, drill, p2);
    let (assembly, rest): (Assembly<FasteningBolt, 1760>, EmptyBoltBox) = fasten(d1, d2, bolts);

    // All swarf reaches the dedicated waste consumer, which leaves the model
    // at the boundary with its contents.
    let bin = discard_swarf(bin, cut_swarf);
    let bin = discard_swarf(bin, s1);
    let bin = discard_swarf(bin, s2);
    dispose_bin(bin);

    // Labour is accounted for at the boundary ledger; the product ships.
    let sink = send_to(sink, l1);
    let _sink = send_to(sink, l2);
    let _customer = send_to(new_customer(), assembly);

    // The reusable resources come back out of the flow (R2) with the budget
    // drawn down by exactly the drilling time, and stay with the caller.
    assert_eq!(Person::<6000>::BUDGET_MS, 6000);
    let _reusables = (person, drill);
    let _empty_box: EmptyBoltBox = rest; // the exhausted box is accounted for
}

/// Flow order B: the same processes in a different valid sequence (R9) —
/// plate 2 drilled before plate 1, each piece of swarf binned as soon as it
/// exists, and the assembly shipped before the bin leaves. Only data
/// dependencies constrain the order; both compositions type-check.
///
/// Verifies: REQ-001, REQ-002, REQ-003
#[test]
fn flow_order_b_type_checks_and_accounts_for_everything() {
    let person = new_person::<10_000>();
    let drill = supply_drill();
    let bin = new_swarf_bin::<N3>();

    let sheet = supply_sheet::<2000>();
    let (p1, p2, cut_swarf) = cut::<2000, 900, 900, 200>(sheet);
    let bin = discard_swarf(bin, cut_swarf);

    let (person, drill, d2, s2, l2) =
        drill_holes::<2000, 8000, 10_000, 900, 880, 20>(person, drill, p2);
    let bin = discard_swarf(bin, s2);
    let (person, drill, d1, s1, l1) =
        drill_holes::<2000, 6000, 8000, 900, 880, 20>(person, drill, p1);
    let bin = discard_swarf(bin, s1);

    let bolts = full_box::<FasteningBolt, N4>();
    let (assembly, rest): (Assembly<FasteningBolt, 1760>, EmptyBoltBox) = fasten(d2, d1, bolts);
    let _customer = send_to(new_customer(), assembly);

    dispose_bin(bin);
    let sink = send_to(new_test_sink(), l2);
    let _sink = send_to(sink, l1);

    assert_eq!(Person::<6000>::BUDGET_MS, 6000);
    let _reusables = (person, drill);
    let _empty_box: EmptyBoltBox = rest;
}

/// The tripwire demonstration (R1 layer 2, F-008, F-032): the same flow as
/// order A, except one piece of drilling swarf never reaches the bin. The
/// binding is named and used, so `must_use` and `unused_variables` are both
/// satisfied — NO compile-time layer can catch this leak; the tripwire
/// `Drop` converts it into this test failure, naming the type and the
/// decimal magnitude (F-027).
///
/// Verifies: REQ-003
#[test]
#[should_panic(expected = "resource leak: Swarf<20> dropped without being consumed")]
fn abandoned_swarf_trips_the_tripwire() {
    let sheet = supply_sheet::<2000>();
    let person = new_person::<10_000>();
    let drill = supply_drill();
    let bolts = full_box::<FasteningBolt, N4>();
    let bin = new_swarf_bin::<N3>();

    let (p1, p2, cut_swarf) = cut::<2000, 900, 900, 200>(sheet);
    let (person, drill, d1, s1, l1) =
        drill_holes::<2000, 8000, 10_000, 900, 880, 20>(person, drill, p1);
    let (person, drill, d2, s2, l2) =
        drill_holes::<2000, 6000, 8000, 900, 880, 20>(person, drill, p2);
    let (assembly, rest): (Assembly<FasteningBolt, 1760>, EmptyBoltBox) = fasten(d1, d2, bolts);

    let bin = discard_swarf(bin, cut_swarf);
    let bin = discard_swarf(bin, s1);
    // s2 is never binned: it falls out of scope at the end of the test and
    // its tripwire panics.
    let _never_reaches_the_bin = s2;
    dispose_bin(bin);

    let sink = send_to(new_test_sink(), l1);
    let _sink = send_to(sink, l2);
    let _customer = send_to(new_customer(), assembly);
    let _reusables = (person, drill);
    let _empty_box: EmptyBoltBox = rest;
}

/// The downstream fixture path (R1, F-004): the pilot's own `test-support`
/// feature — enabled only through the `[dev-dependencies]` self
/// re-declaration — lets this test conjure a sealed `Swarf` without running
/// a flow. The fixture still has to be accounted for like any real resource.
///
/// Verifies: REQ-003
#[test]
fn fixtures_construct_sealed_resources_for_downstream_tests() {
    let swarf: Swarf<25> = Swarf::test_fixture();
    assert_eq!(Swarf::<25>::VALUE, 25);
    let bin = discard_swarf(new_swarf_bin::<N1>(), swarf);
    dispose_bin(bin);
}

/// The generic fixture path (F-036, F-004): macro-generated `test_fixture()`s
/// on parameterized resources work downstream exactly like non-generic ones —
/// including the payload-holding `Assembly`, whose fixture takes by value the
/// real bolts it keeps (R12). The fixtures still have to be accounted for:
/// the assembly ships to the customer like any real one.
#[test]
fn generic_fixtures_construct_sealed_resources_for_downstream_tests() {
    let bolts: FourOf<FasteningBolt> = Cons(
        Bolt::test_fixture(),
        Cons(
            Bolt::test_fixture(),
            Cons(Bolt::test_fixture(), Cons(Bolt::test_fixture(), Nil)),
        ),
    );
    let assembly: Assembly<FasteningBolt, 42> = Assembly::test_fixture(bolts);
    assert_eq!(Assembly::<FasteningBolt, 42>::PLATE_GRAMS, 42);
    let _customer = send_to(new_customer(), assembly);
}
