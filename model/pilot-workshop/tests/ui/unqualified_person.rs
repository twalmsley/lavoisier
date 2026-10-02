// R18: a plain `Person` carries no qualification, so REQ-004 refuses them at
// the fallible drilling process (E0277). The top line is REQ-004's own
// on_unimplemented message (R10 rule 8, F-044) — the requirement trait's
// attribute, not the failing marker's, because the marker fails as a
// supertrait obligation of the requirement bound.
use model_core::common::boundary::new_person;
use pilot_workshop::resources::boundary::{outcome_success, supply_drill_bit, supply_guard, supply_plate};
use pilot_workshop::resources::processes::{drill_holes_fallible, fit_guard};

fn main() {
    let person = new_person::<5000>(); // never qualified
    let guard = fit_guard(supply_guard());
    let r = drill_holes_fallible::<900, 880, 20, 880, 20, _, _>(
        person,
        guard,
        supply_drill_bit(),
        supply_plate::<900>(),
        outcome_success(),
    );
}
