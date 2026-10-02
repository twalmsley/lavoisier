//! Integration tests for the fallible drilling flows (R5, R17): downstream
//! composition using only the public API — **both arms** of the converging
//! flow and **all three paths** of the bounded repair-and-retry flow (R5's
//! per-arm rule: branch coverage is leak coverage, F-002, and the R4
//! per-instantiation asserts only run where a test instantiates the process).
//!
//! These tests sit outside the pilot's privacy boundary: every input comes
//! from a boundary supplier (the outcome tokens included — variability enters
//! only at the boundary, R17/F-042) and every output must genuinely reach a
//! consumer or stay with the caller.

#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![deny(let_underscore_drop)]
#![recursion_limit = "2048"]

use model_core::boundary::send_to;
use model_core::common::boundary::{new_person, qualify};
use model_core::history::boundary::new_history;
use model_core::nat::aliases::{N1, N2};
use pilot_workshop::characteristics::DrillingCert;
use pilot_workshop::flows::{RetryOutcome, drill_one_plate_handling_both_arms, drill_with_one_retry};
use pilot_workshop::resources::boundary::{
    dispose_bin, new_customer, new_scrap_yard, new_swarf_bin, new_tool_stores, outcome_failure,
    outcome_success, return_outcome, supply_drill_bit, supply_guard, supply_plate,
    supply_spare_parts,
};
use pilot_workshop::resources::processes::{discard_swarf, fit_guard};

/// The converging flow, success arm (R17): the product ships, the unused
/// repair kit goes back to stores, the swarf was binned, one labour event was
/// recorded — and the operator, guard and bit converge with the failure arm's
/// types.
///
/// Verifies: REQ-003, REQ-004, REQ-005
#[test]
fn converging_flow_success_arm() {
    let (operator, guard, bit, plate, parts, yard, bin, history) =
        drill_one_plate_handling_both_arms(
            qualify::<DrillingCert, 5000>(new_person::<5000>()),
            fit_guard(supply_guard()),
            supply_drill_bit(),
            supply_plate::<900>(),
            outcome_success(),
            supply_spare_parts(),
            new_scrap_yard(),
            new_swarf_bin::<N1>(),
            new_history(),
        );
    // Success arm: the product exists; ship it.
    let plate = plate.expect("the success arm must produce the drilled plate");
    let _customer = send_to(new_customer(), plate);
    // The unused reserve kit is re-accounted at the boundary (F-050).
    let parts = parts.expect("the success arm must return the unused repair kit");
    let _stores = send_to(new_tool_stores(), parts);
    dispose_bin(bin);
    assert_eq!(history.event_count(), 1);
    let _accounted = (operator, guard, bit, yard, history);
}

/// The converging flow, failure arm (R17): no product, the kit was consumed
/// by the repair, the bit comes back working (repaired), scrap went to the
/// yard, the swarf was still binned (REQ-003 covers failure swarf too), and
/// the labour was still recorded — failure costs time.
///
/// Verifies: REQ-003, REQ-004, REQ-005
#[test]
fn converging_flow_failure_arm() {
    let (operator, guard, bit, plate, parts, yard, bin, history) =
        drill_one_plate_handling_both_arms(
            qualify::<DrillingCert, 5000>(new_person::<5000>()),
            fit_guard(supply_guard()),
            supply_drill_bit(),
            supply_plate::<900>(),
            outcome_failure(),
            supply_spare_parts(),
            new_scrap_yard(),
            new_swarf_bin::<N1>(),
            new_history(),
        );
    assert!(plate.is_none(), "the failure arm produces no drilled plate");
    assert!(parts.is_none(), "the repair consumed the kit");
    dispose_bin(bin);
    assert_eq!(history.event_count(), 1);
    let _accounted = (operator, guard, bit, yard, history);
}

/// Retry path 1 — first try succeeds (R17/F-050): the provisioned reserves
/// come back unused and must be re-accounted — blank and kit to the stores,
/// the untried trial back to the environment. The honest cost of
/// over-provisioning.
///
/// Verifies: REQ-003, REQ-004, REQ-005
#[test]
fn retry_flow_first_try_success_returns_the_reserves() {
    let (out, yard, history) = drill_with_one_retry(
        qualify::<DrillingCert, 10_000>(new_person::<10_000>()),
        fit_guard(supply_guard()),
        supply_drill_bit(),
        supply_plate::<900>(),
        outcome_success(),
        supply_plate::<900>(),
        outcome_failure(), // provisioned but never tried
        supply_spare_parts(),
        new_scrap_yard(),
        new_swarf_bin::<N2>(),
        new_history(),
    );
    match out {
        RetryOutcome::FirstTry {
            operator,
            guard,
            bit,
            plate,
            reserve_plate,
            reserve_outcome,
            reserve_parts,
            bin,
        } => {
            let _customer = send_to(new_customer(), plate);
            // Re-account every reserve: blank and kit to stores, the untried
            // trial back to the environment (F-050).
            let stores = send_to(new_tool_stores(), reserve_plate);
            let _stores = send_to(stores, reserve_parts);
            return_outcome(reserve_outcome);
            dispose_bin(bin);
            assert_eq!(history.event_count(), 1);
            let _accounted = (operator, guard, bit, yard, history);
        }
        _ => unreachable!("a success first trial cannot reach the retry paths"),
    }
}

/// Retry path 2 — fail then succeed (R17/F-050): the first failure is fully
/// accounted inside the flow (scrap to the yard, swarf binned, labour
/// recorded, bit repaired with the kit), and the retry delivers the product
/// from the reserve blank at the cost of a second recorded labour event.
///
/// Verifies: REQ-003, REQ-004, REQ-005
#[test]
fn retry_flow_fail_then_success_recovers() {
    let (out, yard, history) = drill_with_one_retry(
        qualify::<DrillingCert, 10_000>(new_person::<10_000>()),
        fit_guard(supply_guard()),
        supply_drill_bit(),
        supply_plate::<900>(),
        outcome_failure(),
        supply_plate::<900>(),
        outcome_success(),
        supply_spare_parts(),
        new_scrap_yard(),
        new_swarf_bin::<N2>(),
        new_history(),
    );
    match out {
        RetryOutcome::Retried {
            operator,
            guard,
            bit,
            plate,
            bin,
        } => {
            let _customer = send_to(new_customer(), plate);
            dispose_bin(bin);
            // Two attempts, two labour events — failure time is on the
            // record (R16).
            assert_eq!(history.event_count(), 2);
            let _accounted = (operator, guard, bit, yard, history);
        }
        _ => unreachable!("fail-then-success must take the Retried path"),
    }
}

/// Retry path 3 — both attempts fail (R17/F-050): the rework budget is
/// exhausted and the second failure comes back whole; the caller accounts for
/// every piece (scrap and the now-unrepairable bit to the yard, the swarf to
/// the bin's last slot, both labour events already on the record).
///
/// Verifies: REQ-003, REQ-004, REQ-005
#[test]
fn retry_flow_gives_up_after_the_bounded_rework() {
    let (out, yard, history) = drill_with_one_retry(
        qualify::<DrillingCert, 10_000>(new_person::<10_000>()),
        fit_guard(supply_guard()),
        supply_drill_bit(),
        supply_plate::<900>(),
        outcome_failure(),
        supply_plate::<900>(),
        outcome_failure(),
        supply_spare_parts(),
        new_scrap_yard(),
        new_swarf_bin::<N2>(),
        new_history(),
    );
    match out {
        RetryOutcome::GaveUp { fail, bin } => {
            let yard = send_to(yard, fail.scrap);
            let bin = discard_swarf(bin, fail.swarf);
            let yard = send_to(yard, fail.broken_bit); // beyond repair: scrapped
            dispose_bin(bin);
            assert_eq!(history.event_count(), 2);
            let _accounted = (fail.operator, fail.guard, yard, history);
        }
        _ => unreachable!("fail-then-fail must take the GaveUp path"),
    }
}
