//! A guard that exists but has not been fitted is the unsafe state: only
//! `FittedGuard` carries the `Fitted` characteristic, so the unfitted
//! `MachineGuard` fails REQ-002 (E0277).

use exp11_qualifications::qualifications::DrillingCert;
use exp11_qualifications::resources::boundary::{certify, supply_guard, supply_plate};
use exp11_qualifications::resources::processes::drill_plate;
use model_core::common::boundary::new_person;

fn main() {
    let operator = certify::<DrillingCert, 5000>(new_person::<5000>());
    let guard = supply_guard(); // never fitted
    let plate = supply_plate::<900>();
    let (operator, guard, drilled, swarf) = drill_plate::<900, 880, 20, _, _>(operator, guard, plate);
}
