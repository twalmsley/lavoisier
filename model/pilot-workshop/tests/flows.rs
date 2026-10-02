//! Integration tests: the full cut -> drill -> fasten flow, composed in two
//! different valid orders (R9: the model describes connections, not
//! sequences), with everything accounted for (R1) — swarf to the bin and on
//! to disposal, labour recorded into per-branch execution histories merged at
//! the join (R16), the assembly to the customer, reusable resources back to
//! the caller. Plus the tripwire demonstration for abandoned swarf (R1
//! layer 2, F-008) and the downstream fixture path (F-004).
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
use model_core::history::Entry;
use model_core::history::boundary::new_history;
use model_core::history::processes::{merge, record};
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
    // One execution history per branch of the flow, created at the boundary
    // (R16): never a single global history threaded everywhere, which under
    // R2 would serialize the whole model.
    let h1 = new_history();
    let h2 = new_history();

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

    // Labour is accounted to the execution histories, attributed to the
    // process that spent it (R16, F-035); the branch records are merged at
    // the join into one partial-order record. The product ships.
    let h1 = record(h1, "drill_holes", l1);
    let h2 = record(h2, "drill_holes", l2);
    let history = merge(h1, h2);
    let _customer = send_to(new_customer(), assembly);

    // The merged record is a partial order (R16): one Join holding the two
    // branches, with no interleaving claimed between them.
    assert_eq!(history.event_count(), 2);
    match history.entries() {
        [Entry::Join(left, right)] => match (&left[..], &right[..]) {
            ([Entry::Event(e1)], [Entry::Event(e2)]) => {
                assert_eq!(e1.process, "drill_holes");
                assert_eq!(e1.item, "Labour");
                assert_eq!(e1.magnitude, 2000);
                assert_eq!(e1.unit, "person-milliseconds");
                assert_eq!(e2.magnitude, 2000);
            }
            other => panic!("expected one recorded event per branch, got {other:?}"),
        },
        other => panic!("expected a single Join at the merge point, got {other:?}"),
    }

    // The reusable resources come back out of the flow (R2) with the budget
    // drawn down by exactly the drilling time, and stay with the caller —
    // as does the merged execution record (R16).
    assert_eq!(Person::<6000>::BUDGET_MS, 6000);
    let _reusables = (person, drill);
    let _execution_record = history;
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

    // In this order each branch's history is created at the boundary and
    // written as soon as its labour exists (R16): only data dependencies
    // constrain when recording happens (R9).
    let (person, drill, d2, s2, l2) =
        drill_holes::<2000, 8000, 10_000, 900, 880, 20>(person, drill, p2);
    let bin = discard_swarf(bin, s2);
    let h2 = record(new_history(), "drill_holes", l2);
    let (person, drill, d1, s1, l1) =
        drill_holes::<2000, 6000, 8000, 900, 880, 20>(person, drill, p1);
    let bin = discard_swarf(bin, s1);
    let h1 = record(new_history(), "drill_holes", l1);

    let bolts = full_box::<FasteningBolt, N4>();
    let (assembly, rest): (Assembly<FasteningBolt, 1760>, EmptyBoltBox) = fasten(d2, d1, bolts);
    let _customer = send_to(new_customer(), assembly);

    dispose_bin(bin);
    // The join merges the branch records into one partial order (R16); the
    // merged record stays with the caller.
    let history = merge(h2, h1);
    assert_eq!(history.event_count(), 2);
    assert!(matches!(history.entries(), [Entry::Join(_, _)]));
    let _execution_record = history;

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

    let history = record(new_history(), "drill_holes", l1);
    let history = record(history, "drill_holes", l2);
    let _execution_record = history;
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
