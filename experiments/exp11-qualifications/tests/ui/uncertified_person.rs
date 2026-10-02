//! A plain `Person` carries no qualification: REQ-001 refuses them at the
//! drilling process (E0277, with the REQ-phrased on_unimplemented message).

use exp11_qualifications::resources::boundary::{supply_guard, supply_plate};
use exp11_qualifications::resources::processes::{drill_plate, fit_guard};
use model_core::common::boundary::new_person;

fn main() {
    let person = new_person::<5000>();
    let guard = fit_guard(supply_guard());
    let plate = supply_plate::<900>();
    let (person, guard, drilled, swarf) = drill_plate::<900, 880, 20, _, _>(person, guard, plate);
}
