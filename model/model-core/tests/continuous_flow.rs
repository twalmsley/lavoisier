//! Integration tests for the continuous-resource patterns (R15) and the
//! tripwire layer of the conservation regime (R1, F-008, F-032), through the
//! `test-support` fixtures (F-004).
//!
//! model-core defines no `REQ-NNN` requirements, so these tests carry plain
//! doc comments, not traceability tags (R10 applies downstream).

#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![deny(let_underscore_drop)]
#![recursion_limit = "2048"]

use model_core::boundary::send_to;
use model_core::common::boundary::new_person;
use model_core::common::processes::draw_time;
use model_core::common::{Labour, Person};
use model_core::fixtures::{
    EmptyGasBottle, Gas, GasBottle, TestSink, draw_gas, fill_gas_bottle, new_test_sink,
};

/// Order A of a small continuous flow: gas first, then time (R9: only data
/// dependencies constrain sequence).
fn flow_gas_first() -> (Person<5000>, TestSink) {
    let bottle = fill_gas_bottle::<5000>();
    let person = new_person::<10_000>();
    let sink = new_test_sink();

    let (gas, bottle) = draw_gas::<300, 4700, 5000>(bottle);
    let (labour, person) = draw_time::<5000, 5000, 10_000>(person);

    let sink = send_to(sink, gas);
    let sink = send_to(sink, labour);
    let sink = send_to(sink, bottle); // the part-empty bottle is accounted for
    (person, sink)
}

/// Order B: the same flow with the independent steps reordered; both orders
/// must type-check (R9).
fn flow_time_first() -> (Person<5000>, TestSink) {
    let person = new_person::<10_000>();
    let (labour, person) = draw_time::<5000, 5000, 10_000>(person);

    let bottle = fill_gas_bottle::<5000>();
    let (gas, bottle) = draw_gas::<300, 4700, 5000>(bottle);

    let sink = send_to(new_test_sink(), bottle);
    let sink = send_to(sink, labour);
    let sink = send_to(sink, gas);
    (person, sink)
}

/// Both orders run; the part-spent person (a reusable resource, R2) stays
/// with the caller.
#[test]
fn continuous_flow_type_checks_in_two_orders() {
    let (person_a, _sink_a) = flow_gas_first();
    let (person_b, _sink_b) = flow_time_first();
    assert_eq!(Person::<5000>::BUDGET_MS, 5000);
    let _reusable = (person_a, person_b);
}

/// Drawing conserves: the magnitudes are recoverable as constants (R7) and
/// balance by construction (the compile-time assert checked them already).
#[test]
fn draw_balances_at_the_value_level() {
    let bottle = fill_gas_bottle::<5000>();
    let (gas, rest) = draw_gas::<300, 4700, 5000>(bottle);
    assert_eq!(Gas::<300>::VALUE + GasBottle::<4700>::VALUE, 5000);
    assert_eq!(Gas::<300>::UNIT, "grams");
    let sink = send_to(new_test_sink(), gas);
    let _sink = send_to(sink, rest);
}

/// The budget draws down to the fully-spent state `Person<0>` — a distinct
/// type, like the empty bottle (R15) — with the running balance restated by
/// the modeller and checked by the compiler at every step (F-030).
#[test]
fn time_budget_draws_down_to_zero() {
    let person = new_person::<10_000>();
    let (l1, person) = draw_time::<2000, 8000, 10_000>(person);
    let (l2, person) = draw_time::<3000, 5000, 8000>(person);
    let (l3, person) = draw_time::<5000, 0, 5000>(person);
    assert_eq!(Person::<0>::BUDGET_MS, 0);
    assert_eq!(Labour::<2000>::VALUE + Labour::<3000>::VALUE + Labour::<5000>::VALUE, 10_000);
    let sink = send_to(new_test_sink(), l1);
    let sink = send_to(sink, l2);
    let _sink = send_to(sink, l3);
    let _spent_person = person;
}

/// A fully drained bottle becomes the empty state, which still reaches a
/// consumer like any other resource (R15).
#[test]
fn empty_bottle_is_a_distinct_accounted_resource() {
    let bottle = fill_gas_bottle::<300>();
    let (gas, empty) = draw_gas::<300, 0, 300>(bottle);
    let empty: EmptyGasBottle = empty; // the empty state is a named type
    let sink = send_to(new_test_sink(), gas);
    let _sink = send_to(sink, empty);
}

// ---------------------------------------------------------------------------
// Tripwire demonstrations (R1 layer 2): the should_panic cases below are the
// leaks that NO compile-time layer can catch (F-002's used-then-dropped gap);
// the tripwire Drop converts them into ordinary test failures (F-008, F-032).
// ---------------------------------------------------------------------------

/// The "empty bottle dropped silently" case: the binding is named and used,
/// so `must_use` and `unused_variables` are both satisfied — only the
/// tripwire catches the abandonment, at test time (F-032).
#[test]
#[should_panic(expected = "resource leak: GasBottle<0> dropped without being consumed")]
fn abandoned_empty_bottle_trips_the_tripwire() {
    let bottle = fill_gas_bottle::<300>();
    let (gas, empty) = draw_gas::<300, 0, 300>(bottle);
    let _sink = send_to(new_test_sink(), gas);
    // `empty` (a GasBottle<0> — still a resource, R15) goes out of scope
    // without reaching a consumer: the tripwire panics.
    let _still_bound_but_never_consumed = empty;
}

/// Drawn material that never reaches a consumer trips the tripwire, naming
/// the type and the decimal magnitude (F-027).
#[test]
#[should_panic(expected = "resource leak: Gas<300> dropped without being consumed")]
fn unconsumed_drawn_gas_trips_the_tripwire() {
    let bottle = fill_gas_bottle::<5000>();
    let (gas, rest) = draw_gas::<300, 4700, 5000>(bottle);
    let _sink = send_to(new_test_sink(), rest);
    let _still_bound_but_never_consumed = gas;
}

/// Expended labour is conserved too (R15): losing it is a leak like any
/// other.
#[test]
#[should_panic(expected = "resource leak: Labour<500> dropped without being consumed")]
fn unaccounted_labour_trips_the_tripwire() {
    let person = new_person::<1000>();
    let (labour, person) = draw_time::<500, 500, 1000>(person);
    let _person = person;
    let _still_bound_but_never_consumed = labour;
}

/// Test fixtures from the `test-support` feature construct sealed resources
/// directly (R1, F-004) — the path a downstream crate's tests use.
#[test]
fn test_fixtures_construct_sealed_resources() {
    let gas: Gas<42> = Gas::test_fixture();
    let person: Person<7> = Person::test_fixture();
    let org = model_core::common::Organisation::test_fixture();
    let loc = model_core::common::Location::test_fixture();
    assert_eq!(Gas::<42>::VALUE, 42);
    assert_eq!(Person::<7>::BUDGET_MS, 7);
    let _sink = send_to(new_test_sink(), gas);
    let _reusables_stay_with_the_caller = (person, org, loc);
}
