//! R9: a flow that uses a product that is never produced.
//! `fasten` needs `DrilledPlate`s, but this flow skips the drilling
//! process, so only undrilled `Plate`s exist.

use exp07_flows::processes::{cut, fasten};
use exp07_flows::resources::boundary;

fn main() {
    let sheet = boundary::supply_sheet();
    let person = boundary::supply_person("Alice");
    let bolts = boundary::supply_bolts();

    let (p1, p2, _offcut) = cut(sheet);

    // No drill_holes step: the DrilledPlate product is never produced.
    let (_person, _assembly) = fasten(person, p1, p2, bolts);
}
