//! R2: a process that swallows the drill (does not return it).
//! NOTE: the swallowing process itself compiles — Rust's types are affine,
//! so dropping a moved-in parameter is legal (the known R1 gap). The error
//! only appears at the CALLER, the first time the flow tries to use the
//! drill again.

use exp07_flows::resources::boundary;
use exp07_flows::resources::{Drill, DrilledPlate, Person, Plate};

/// A badly modelled process: takes the drill but never gives it back.
/// This function compiles without complaint.
fn bad_drill_holes(person: Person, drill: Drill, plate: Plate) -> (Person, DrilledPlate) {
    let (person, _swallowed_drill, drilled) =
        exp07_flows::processes::drill_holes(person, drill, plate);
    (person, drilled)
}

fn main() {
    let sheet = boundary::supply_sheet();
    let person = boundary::supply_person("Alice");
    let drill = boundary::supply_drill();

    let (p1, p2, _offcut) = cut_both(sheet);

    let (person, _d1) = bad_drill_holes(person, drill, p1);
    // The drill is gone: the next process in the flow cannot have it.
    let (_person, _drill, _d2) = exp07_flows::processes::drill_holes(person, drill, p2);
}

fn cut_both(sheet: exp07_flows::resources::SteelSheet) -> (Plate, Plate, exp07_flows::resources::Offcut) {
    exp07_flows::processes::cut(sheet)
}
