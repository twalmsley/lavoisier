//! Integration flows (R5, R9; candidate R17): downstream-style composition
//! using only the public API — both arms of the fallible process handled,
//! and the bounded repair-and-retry composition exercised on **all three of
//! its paths** (branch coverage is leak coverage, F-002).

use exp10_failure_modes::flows::{RetryOutcome, drill_one_plate_handling_both_arms, drill_with_one_retry};
use exp10_failure_modes::model::boundary::{
    new_customer, new_scrap_yard, new_tool_stores, outcome_failure, outcome_success,
    return_outcome, supply_drill_bit, supply_plate, supply_spare_parts,
};
use model_core::boundary::send_to;
use model_core::common::boundary::new_person;
use model_core::history::boundary::new_history;
use model_core::history::processes::record;

/// The converging flow, success arm: the product ships, the unused repair
/// kit goes back to stores, one labour event is recorded.
#[test]
fn converging_flow_success_arm() {
    let (person, bit, plate, parts, yard, history) = drill_one_plate_handling_both_arms(
        new_person::<5000>(),
        supply_drill_bit(),
        supply_plate::<450>(),
        outcome_success(),
        supply_spare_parts(),
        new_scrap_yard(),
        new_history(),
    );
    // Success arm: the product exists; ship it.
    let plate = plate.expect("the success arm must produce the drilled plate");
    let _customer = send_to(new_customer(), plate);
    // The unused reserve kit is re-accounted at the boundary.
    let parts = parts.expect("the success arm must return the unused repair kit");
    let _stores = send_to(new_tool_stores(), parts);
    assert_eq!(history.event_count(), 1);
    let _accounted = (person, bit, yard, history);
}

/// The converging flow, failure arm: no product, the kit was consumed by the
/// repair, the bit comes back working (repaired), scrap and swarf went to
/// the yard, and the labour was still recorded — failure costs time.
#[test]
fn converging_flow_failure_arm() {
    let (person, bit, plate, parts, yard, history) = drill_one_plate_handling_both_arms(
        new_person::<5000>(),
        supply_drill_bit(),
        supply_plate::<450>(),
        outcome_failure(),
        supply_spare_parts(),
        new_scrap_yard(),
        new_history(),
    );
    assert!(plate.is_none(), "the failure arm produces no drilled plate");
    assert!(parts.is_none(), "the repair consumed the kit");
    assert_eq!(history.event_count(), 1);
    let _accounted = (person, bit, yard, history);
}

/// Retry path 1 — first try succeeds: the provisioned reserves come back
/// unused and must be re-accounted (the honest cost of over-provisioning).
#[test]
fn retry_flow_first_try_success_returns_the_reserves() {
    let (out, yard, history) = drill_with_one_retry(
        new_person::<10_000>(),
        supply_drill_bit(),
        supply_plate::<450>(),
        outcome_success(),
        supply_plate::<450>(),
        outcome_failure(), // provisioned but never tried
        supply_spare_parts(),
        new_scrap_yard(),
        new_history(),
    );
    match out {
        RetryOutcome::FirstTry {
            person,
            bit,
            plate,
            reserve_plate,
            reserve_outcome,
            reserve_parts,
        } => {
            let _customer = send_to(new_customer(), plate);
            // Re-account every reserve: blank and kit to stores, the untried
            // trial back to the environment.
            let stores = send_to(new_tool_stores(), reserve_plate);
            let _stores = send_to(stores, reserve_parts);
            return_outcome(reserve_outcome);
            assert_eq!(history.event_count(), 1);
            let _accounted = (person, bit, yard, history);
        }
        _ => unreachable!("a success first trial cannot reach the retry paths"),
    }
}

/// Retry path 2 — fail then succeed: the first failure is fully accounted
/// inside the flow (scrap + swarf to the yard, labour recorded, bit
/// repaired), and the retry delivers the product from the reserve blank.
#[test]
fn retry_flow_fail_then_success_recovers() {
    let (out, yard, history) = drill_with_one_retry(
        new_person::<10_000>(),
        supply_drill_bit(),
        supply_plate::<450>(),
        outcome_failure(),
        supply_plate::<450>(),
        outcome_success(),
        supply_spare_parts(),
        new_scrap_yard(),
        new_history(),
    );
    match out {
        RetryOutcome::Retried { person, bit, plate } => {
            let _customer = send_to(new_customer(), plate);
            // Two attempts, two labour events — failure time is on the record.
            assert_eq!(history.event_count(), 2);
            let _accounted = (person, bit, yard, history);
        }
        _ => unreachable!("fail-then-success must take the Retried path"),
    }
}

/// Retry path 3 — both attempts fail: the rework budget is exhausted and the
/// second failure comes back whole; the caller accounts for every piece
/// (scrap and swarf and the now-unrepairable bit to the yard, the labour to
/// the record).
#[test]
fn retry_flow_gives_up_after_the_bounded_rework() {
    let (out, yard, history) = drill_with_one_retry(
        new_person::<10_000>(),
        supply_drill_bit(),
        supply_plate::<450>(),
        outcome_failure(),
        supply_plate::<450>(),
        outcome_failure(),
        supply_spare_parts(),
        new_scrap_yard(),
        new_history(),
    );
    match out {
        RetryOutcome::GaveUp(fail) => {
            let yard = send_to(yard, fail.scrap);
            let yard = send_to(yard, fail.swarf);
            let yard = send_to(yard, fail.broken_bit); // beyond repair: scrapped
            let history = record(history, "drill_fallible", fail.labour);
            assert_eq!(history.event_count(), 2);
            let _accounted = (fail.person, yard, history);
        }
        _ => unreachable!("fail-then-fail must take the GaveUp path"),
    }
}
