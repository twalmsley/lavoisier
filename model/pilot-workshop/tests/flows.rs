//! Integration tests: the full cut -> drill -> fasten flow, composed in two
//! different valid orders (R9: the model describes connections, not
//! sequences), with everything accounted for (R1) — swarf to the bin and on
//! to disposal, labour recorded into per-branch execution histories merged at
//! the join (R16), the assembly to the customer, reusable resources back to
//! the caller. The drilling steps require a certified operator and a fitted
//! guard (R18: REQ-004, REQ-005), and flow order A buys its bolt box from the
//! vendor with money drawn from the workshop's account, conserved end to end
//! (R19). Plus the tripwire demonstration for abandoned swarf (R1 layer 2,
//! F-008) and the downstream fixture path (F-004).
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
use model_core::common::Qualified;
use model_core::common::boundary::{new_person, qualify, release};
use model_core::history::Entry;
use model_core::history::boundary::new_history;
use model_core::history::processes::{merge, record};
use model_core::list::{Cons, Nil};
use model_core::nat::aliases::{N1, N3, N4};
use pilot_workshop::catalogue::boundary::full_box;
use pilot_workshop::catalogue::{Bolt, EmptyBoltBox, FasteningBolt, FourOf};
use pilot_workshop::characteristics::DrillingCert;
use pilot_workshop::money::boundary::{close_account, new_vendor, open_account};
use pilot_workshop::money::processes::{deposit, draw_funds, purchase};
use pilot_workshop::resources::boundary::{
    dispose_bin, new_customer, new_swarf_bin, supply_drill, supply_guard, supply_sheet,
};
use pilot_workshop::resources::processes::{
    cut, discard_swarf, drill_holes, fasten, fit_guard, remove_guard,
};
use pilot_workshop::resources::{Assembly, Swarf};

/// Flow order A: buy the bolts, cut, drill plate 1, drill plate 2, fasten,
/// then account for all waste at the end. Money is conserved end to end
/// (R19): 500 pence drawn, 400 to the vendor, 100 change banked, the account
/// closed at the boundary. The operator is certified and the guard fitted
/// before drilling (R18), and both are returned to their unwrapped states
/// afterwards — qualification and fitting are conserving processes. Loose
/// threading of the reusables (R2, F-024); the time budget's running balance
/// is restated at every call and checked by the compiler (R15, F-030).
///
/// Verifies: REQ-001, REQ-002, REQ-003, REQ-004, REQ-005
#[test]
fn flow_order_a_type_checks_and_accounts_for_everything() {
    // Money in, goods in (R19): the vendor sells the pilot its bolt box.
    let account = open_account::<1000>();
    let (cash, account) = draw_funds::<500, 500, 1000>(account);
    let (bolts, change, vendor) = purchase::<_, 500, 400, 100>(new_vendor(), cash);
    let account = deposit::<100, 500, 600>(account, change);

    let sheet = supply_sheet::<2000>();
    let operator = qualify::<DrillingCert, 10_000>(new_person::<10_000>());
    let drill = supply_drill();
    let guard = fit_guard(supply_guard());
    let bin = new_swarf_bin::<N3>();
    // One execution history per branch of the flow, created at the boundary
    // (R16): never a single global history threaded everywhere, which under
    // R2 would serialize the whole model.
    let h1 = new_history();
    let h2 = new_history();

    let (p1, p2, cut_swarf) = cut::<2000, 900, 900, 200>(sheet);
    let (operator, drill, guard, d1, s1, l1) =
        drill_holes::<2000, 8000, 10_000, 900, 880, 20, _>(operator, drill, guard, p1);
    let (operator, drill, guard, d2, s2, l2) =
        drill_holes::<2000, 6000, 8000, 900, 880, 20, _>(operator, drill, guard, p2);
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
    // drawn down by exactly the drilling time; the qualification and the
    // fitting are undone by their conserving inverses (R18), the account
    // closes at the boundary (R19), and the merged execution record stays
    // with the caller.
    assert_eq!(Qualified::<DrillingCert, 6000>::BUDGET_MS, 6000);
    let person = release(operator);
    let guard = remove_guard(guard);
    close_account(account);
    let _reusables = (person, drill, guard, vendor);
    let _execution_record = history;
    let _empty_box: EmptyBoltBox = rest; // the exhausted box is accounted for
}

/// Flow order B: the same processes in a different valid sequence (R9) —
/// plate 2 drilled before plate 1, each piece of swarf binned as soon as it
/// exists, the bolt box taken straight from the catalogue's boundary
/// placeholder instead of the vendor, and the assembly shipped before the bin
/// leaves. Only data dependencies constrain the order; both compositions
/// type-check. The operator and the fitted guard stay with the caller in
/// their working states.
///
/// Verifies: REQ-001, REQ-002, REQ-003, REQ-004, REQ-005
#[test]
fn flow_order_b_type_checks_and_accounts_for_everything() {
    let operator = qualify::<DrillingCert, 10_000>(new_person::<10_000>());
    let drill = supply_drill();
    let guard = fit_guard(supply_guard());
    let bin = new_swarf_bin::<N3>();

    let sheet = supply_sheet::<2000>();
    let (p1, p2, cut_swarf) = cut::<2000, 900, 900, 200>(sheet);
    let bin = discard_swarf(bin, cut_swarf);

    // In this order each branch's history is created at the boundary and
    // written as soon as its labour exists (R16): only data dependencies
    // constrain when recording happens (R9).
    let (operator, drill, guard, d2, s2, l2) =
        drill_holes::<2000, 8000, 10_000, 900, 880, 20, _>(operator, drill, guard, p2);
    let bin = discard_swarf(bin, s2);
    let h2 = record(new_history(), "drill_holes", l2);
    let (operator, drill, guard, d1, s1, l1) =
        drill_holes::<2000, 6000, 8000, 900, 880, 20, _>(operator, drill, guard, p1);
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

    assert_eq!(Qualified::<DrillingCert, 6000>::BUDGET_MS, 6000);
    let _reusables = (operator, drill, guard);
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
    let operator = qualify::<DrillingCert, 10_000>(new_person::<10_000>());
    let drill = supply_drill();
    let guard = fit_guard(supply_guard());
    let bolts = full_box::<FasteningBolt, N4>();
    let bin = new_swarf_bin::<N3>();

    let (p1, p2, cut_swarf) = cut::<2000, 900, 900, 200>(sheet);
    let (operator, drill, guard, d1, s1, l1) =
        drill_holes::<2000, 8000, 10_000, 900, 880, 20, _>(operator, drill, guard, p1);
    let (operator, drill, guard, d2, s2, l2) =
        drill_holes::<2000, 6000, 8000, 900, 880, 20, _>(operator, drill, guard, p2);
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
    let _reusables = (operator, drill, guard);
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
