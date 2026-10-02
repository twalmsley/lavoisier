//! Leak-surface probes (candidate R17, extending the F-002 matrix to
//! `Result`-shaped processes): which layer catches each way of losing the
//! resources inside an unhandled or half-handled outcome.
//!
//! Compile-time rows of the matrix live in `tests/ui/` (trybuild) and the
//! verdicts in RESULTS.md; this file holds the **test-time** rows (tripwire
//! catches) and the **not-caught** row (panic unwinding, F-002).

use exp10_failure_modes::model::boundary::{
    outcome_failure, outcome_success, supply_drill_bit, supply_plate,
};
use exp10_failure_modes::model::processes::drill_fallible;
use model_core::boundary::send_to;
use model_core::common::boundary::new_person;
use model_core::history::boundary::new_history;
use model_core::history::processes::record;

/// Probe: a `Result` that is bound but never matched. No compile-time layer
/// sees it (`must_use` is satisfied by the binding; F-002's used-then-dropped
/// gap) — the **tripwire** converts it into a test failure when the bundle
/// inside is dropped, naming the first tripwired field in declaration order.
#[test]
#[should_panic(expected = "resource leak: DrilledPlate<440> dropped without being consumed")]
fn bound_but_unmatched_result_trips_the_tripwire() {
    let result = drill_fallible::<1000, 4000, 5000, 450, 440, 10, 430, 20>(
        new_person::<5000>(),
        supply_drill_bit(),
        supply_plate::<450>(),
        outcome_success(),
    );
    // "I'll handle it later" — the binding satisfies must_use, then falls
    // out of scope: the whole success bundle is dropped.
    let _handle_it_later = result;
}

/// Probe: a `match` arm that forgets resources with a `..` pattern. No
/// compile-time layer sees struct-pattern `..` drops (F-002) — the tripwire
/// on the first forgotten field catches it at test time. (Measured drop
/// timing: fields skipped by `..` are a partial move — they stay in the
/// destructured binding's storage and drop at the end of its scope, in
/// declaration order, *after* every bound field; so the tripwire fires at
/// the end of the arm, naming the first skipped tripwired field.)
#[test]
#[should_panic(expected = "resource leak: Swarf<10> dropped without being consumed")]
fn match_arm_forgetting_fields_trips_the_tripwire() {
    match drill_fallible::<1000, 4000, 5000, 450, 440, 10, 430, 20>(
        new_person::<5000>(),
        supply_drill_bit(),
        supply_plate::<450>(),
        outcome_success(),
    ) {
        Ok(bundle) => {
            // The arm handles the product and the reusables...
            let exp10_failure_modes::model::DrillOk {
                person, bit, plate, ..
            } = bundle;
            let _customer = send_to(exp10_failure_modes::model::boundary::new_customer(), plate);
            let _handled = (person, bit);
            // ...but the `..` silently dropped the swarf and the labour.
        }
        Err(_) => unreachable!("a success token cannot produce the failure arm"),
    }
}

/// Probe (the known F-002 hole, demonstrated for the `Result` shape): a flow
/// that panics while holding the failure bundle — the moral equivalent of
/// `.unwrap()`, which itself does not compile because the bundles carry no
/// `Debug` (see `tests/ui/unwrap_needs_debug.rs`). During unwinding every
/// tripwire stands down (an unguarded tripwire would double-panic and abort
/// the whole test binary, F-008), so the five resources in the bundle are
/// lost **silently**: this test passes with only the explicit panic message,
/// and the very fact that it passes — no SIGABRT, no "resource leak:" text —
/// is the evidence that nothing reported the loss.
#[test]
#[should_panic(expected = "drilling failed and the flow gave up")]
fn panic_while_holding_the_failure_bundle_leaks_silently() {
    match drill_fallible::<1000, 4000, 5000, 450, 440, 10, 430, 20>(
        new_person::<5000>(),
        supply_drill_bit(),
        supply_plate::<450>(),
        outcome_failure(),
    ) {
        Ok(_) => unreachable!("a failure token cannot produce the success arm"),
        Err(bundle) => {
            let _held_across_the_panic = bundle;
            panic!("drilling failed and the flow gave up");
        }
    }
}

/// Control for the probes above: the fully-handled failure arm is quiet —
/// every tripwire is defused by a real consumer, nothing fires.
#[test]
fn fully_handled_failure_arm_is_quiet() {
    match drill_fallible::<1000, 4000, 5000, 450, 440, 10, 430, 20>(
        new_person::<5000>(),
        supply_drill_bit(),
        supply_plate::<450>(),
        outcome_failure(),
    ) {
        Ok(_) => unreachable!("a failure token cannot produce the success arm"),
        Err(bundle) => {
            let yard = send_to(
                exp10_failure_modes::model::boundary::new_scrap_yard(),
                bundle.scrap,
            );
            let yard = send_to(yard, bundle.swarf);
            let yard = send_to(yard, bundle.broken_bit);
            let history = record(new_history(), "drill_fallible", bundle.labour);
            assert_eq!(history.event_count(), 1);
            let _accounted = (bundle.person, yard, history);
        }
    }
}
