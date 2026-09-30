//! R1 backstop for "product never produced": the flow cannot dodge the
//! type mismatch by constructing the missing `DrilledPlate` from nothing,
//! because its constructor is private.

use exp07_flows::processes::fasten;
use exp07_flows::resources::{boundary, DrilledPlate};

fn main() {
    let person = boundary::supply_person("Alice");
    let bolts = boundary::supply_bolts();

    // Try to conjure the never-produced product out of thin air.
    let d1 = DrilledPlate { grams: 800 };
    let d2 = DrilledPlate { grams: 800 };

    let (_person, _assembly) = fasten(person, d1, d2, bolts);
}
