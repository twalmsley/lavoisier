//! A certified welder is still not certified for drilling: the wrong
//! qualification in the slot fails REQ-001 (E0277).

use exp11_qualifications::qualifications::WeldingCert;
use exp11_qualifications::resources::boundary::{certify, supply_guard, supply_plate};
use exp11_qualifications::resources::processes::{drill_plate, fit_guard};
use model_core::common::boundary::new_person;

fn main() {
    let welder = certify::<WeldingCert, 5000>(new_person::<5000>());
    let guard = fit_guard(supply_guard());
    let plate = supply_plate::<900>();
    let (welder, guard, drilled, swarf) = drill_plate::<900, 880, 20, _, _>(welder, guard, plate);
}
