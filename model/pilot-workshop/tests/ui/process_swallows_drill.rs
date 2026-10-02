// R2/F-002: a badly modelled process that swallows the drill (takes it by
// value, never returns it). NOTE the known affine-types gap: the swallowing
// process itself compiles without complaint — dropping a moved-in parameter
// is legal Rust. The error appears only at the CALLER, the first time the
// flow tries to use the drill again; rustc's "consider borrowing" fix-it must
// be ignored (R2 forbids it, F-007) — the real fix is to return the drill.
use model_core::common::{Labour, Person};
use model_core::common::boundary::new_person;
use pilot_workshop::resources::boundary::{supply_drill, supply_sheet};
use pilot_workshop::resources::processes::{cut, drill_holes};
use pilot_workshop::resources::{Drill, DrilledPlate, Plate, Swarf};

/// A badly modelled process: takes the drill but never gives it back. This
/// function compiles without complaint.
fn bad_drill_holes(
    person: Person<10_000>,
    drill: Drill,
    plate: Plate<900>,
) -> (Person<8000>, DrilledPlate<880>, Swarf<20>, Labour<2000>) {
    let (person, _swallowed_drill, drilled, swarf, labour) =
        drill_holes::<2000, 8000, 10_000, 900, 880, 20>(person, drill, plate);
    (person, drilled, swarf, labour)
}

fn main() {
    let sheet = supply_sheet::<2000>();
    let person = new_person::<10_000>();
    let drill = supply_drill();
    let (p1, p2, _swarf) = cut::<2000, 900, 900, 200>(sheet);

    let (person, _d1, _s1, _l1) = bad_drill_holes(person, drill, p1);
    // The drill is gone: the next process in the flow cannot have it.
    let (_person, _drill, _d2, _s2, _l2) =
        drill_holes::<2000, 6000, 8000, 900, 880, 20>(person, drill, p2);
}
