//! No guard at all: the process signature demands one, so omitting it is an
//! argument-count error (E0061) — blunter than the unfitted-guard E0277, but
//! it still stops the model compiling.

use exp11_qualifications::qualifications::DrillingCert;
use exp11_qualifications::resources::boundary::{certify, supply_plate};
use exp11_qualifications::resources::processes::drill_plate;
use model_core::common::boundary::new_person;

fn main() {
    let operator = certify::<DrillingCert, 5000>(new_person::<5000>());
    let plate = supply_plate::<900>();
    let (operator, guard, drilled, swarf) = drill_plate::<900, 880, 20, _, _>(operator, plate);
}
