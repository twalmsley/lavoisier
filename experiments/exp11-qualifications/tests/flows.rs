//! End-to-end flows (R5, R9): both qualified-person designs drill a plate
//! behind a fitted guard, budgets draw down across the flow, and every
//! conserved output is accounted for. All construction goes through
//! production boundary functions — no test-support fixtures are needed for
//! the happy paths.

use exp11_qualifications::qualifications::DrillingCert;
use exp11_qualifications::resources::boundary::{
    certify, decertify, hire_driller, new_site, supply_guard, supply_plate,
};
use exp11_qualifications::resources::processes::{
    drill_plate, drill_plate_timed, driller_draw_time, fit_guard, operator_draw_time, remove_guard,
};
use model_core::boundary::send_to;
use model_core::common::Person;
use model_core::common::boundary::new_person;
use model_core::history::boundary::new_history;
use model_core::history::processes::record;

/// Style A: certification, fitting and the time draw are their own processes;
/// `drill_plate` takes the operator and guard as requirement-trait bounds and
/// returns both (R2). The person comes back out of the wrapper with the spent
/// budget.
///
/// Verifies: REQ-001, REQ-002
#[test]
fn certified_operator_with_fitted_guard_drills_a_plate() {
    let operator = certify::<DrillingCert, 5000>(new_person::<5000>());
    let guard = fit_guard(supply_guard());
    let plate = supply_plate::<900>();

    // The time draw is its own process (R15), through the wrapper.
    let (labour, operator) = operator_draw_time::<2000, 3000, 5000, _>(operator);
    let (operator, guard, drilled, swarf) = drill_plate::<900, 880, 20, _, _>(operator, guard, plate);

    // Account for everything (R1): plates and swarf to the site sink, labour
    // to the execution history (R16, F-035).
    let site = new_site();
    let site = send_to(site, drilled);
    let site = send_to(site, swarf);
    let history = record(new_history(), "drill_plate", labour);
    assert_eq!(history.event_count(), 1);

    // Reusables stay with the caller (R2); the same person steps back out of
    // the wrapper with 3000 ms left — the interop round trip.
    let person: Person<3000> = decertify(operator);
    let _person = person;
    let _guard_back_on_the_shelf = remove_guard(guard);
    let _site = site;
    let _execution_record = history;
}

/// Style B: the pilot `drill_holes` shape — the draw happens inside
/// `drill_plate_timed`, so the operator parameter is the concrete wrapper and
/// the budget threads through the process signature (F-030).
///
/// Verifies: REQ-001, REQ-002
#[test]
fn timed_drilling_draws_the_budget_inside_the_process() {
    let operator = certify::<DrillingCert, 5000>(new_person::<5000>());
    let guard = fit_guard(supply_guard());
    let plate = supply_plate::<900>();

    let (operator, guard, drilled, swarf, labour) =
        drill_plate_timed::<2000, 3000, 5000, 900, 880, 20, _>(operator, guard, plate);

    let site = new_site();
    let site = send_to(send_to(site, drilled), swarf);
    let history = record(new_history(), "drill_plate_timed", labour);
    assert_eq!(history.event_count(), 1);

    let person: Person<3000> = decertify(operator);
    let _person = person;
    let _guard = remove_guard(guard);
    let _site = site;
    let _history = history;
}

/// The parallel-type probe: `Driller` satisfies the same REQ-001 bound as the
/// wrapper (the requirement trait is the one language both designs speak,
/// F-019), but its time accounting is the forked `Effort`, not `Labour` — it
/// reaches the execution history only through the crate's own hand-written
/// `Recordable` impl.
///
/// Verifies: REQ-001, REQ-002
#[test]
fn parallel_type_driller_also_satisfies_the_requirement() {
    let driller = hire_driller::<8000>();
    let guard = fit_guard(supply_guard());
    let plate = supply_plate::<900>();

    let (effort, driller) = driller_draw_time::<2500, 5500, 8000>(driller);
    let (driller, guard, drilled, swarf) = drill_plate::<900, 880, 20, _, _>(driller, guard, plate);

    let site = new_site();
    let site = send_to(send_to(site, drilled), swarf);
    let history = record(new_history(), "driller_draw_time", effort);
    assert_eq!(history.event_count(), 1);
    let _history = history;

    let _driller_keeps_5500_ms = driller;
    let _guard = remove_guard(guard);
    let _site = site;
}

/// The wrapped budget draws down across several calls exactly like a bare
/// person (R15, F-030): the modeller restates the running balance, the
/// compiler checks every step, and the qualification rides along unchanged.
///
/// Verifies: REQ-001
#[test]
fn budget_draws_down_through_the_qualification() {
    let operator = certify::<DrillingCert, 10_000>(new_person::<10_000>());
    let (l1, operator) = operator_draw_time::<2000, 8000, 10_000, _>(operator);
    let (l2, operator) = operator_draw_time::<3000, 5000, 8000, _>(operator);
    let (l3, operator) = operator_draw_time::<5000, 0, 5000, _>(operator);

    let history = record(new_history(), "draw_1", l1);
    let history = record(history, "draw_2", l2);
    let history = record(history, "draw_3", l3);
    assert_eq!(history.event_count(), 3);

    // The spent state is a distinct type; the person inside is Person<0>.
    let spent: Person<0> = decertify(operator);
    let _spent = spent;
    let _history = history;
}
