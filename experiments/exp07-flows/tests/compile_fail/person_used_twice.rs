//! R2: the same Person moved into two processes at once.
//! The intent is two "concurrent" drilling steps sharing one person —
//! the compiler must refuse, because the person is already busy.

use exp07_flows::processes::{cut, drill_holes};
use exp07_flows::resources::boundary;

fn main() {
    let sheet = boundary::supply_sheet();
    let person = boundary::supply_person("Alice");
    let drill_1 = boundary::supply_drill();
    let drill_2 = boundary::supply_drill();

    let (p1, p2, _offcut) = cut(sheet);

    // Two drills, but only one person: these two process instances
    // both claim `person`, which R2 forbids.
    let (_person_a, _drill_1, _d1) = drill_holes(person, drill_1, p1);
    let (_person_b, _drill_2, _d2) = drill_holes(person, drill_2, p2);
}
