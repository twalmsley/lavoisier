// R2: the same Person moved into two drilling processes at once. The intent
// is two "concurrent" drilling steps sharing one person; the compiler must
// refuse, because the person is already busy ("use of moved value" = "this
// resource is already in use by another process", F-025). Two separate
// drills, so the contended resource in the error is unambiguously the person.
use model_core::common::boundary::new_person;
use pilot_workshop::resources::boundary::{supply_drill, supply_sheet};
use pilot_workshop::resources::processes::{cut, drill_holes};

fn main() {
    let person = new_person::<10_000>();
    let drill_1 = supply_drill();
    let drill_2 = supply_drill();
    let sheet = supply_sheet::<2000>();
    let (p1, p2, _swarf) = cut::<2000, 900, 900, 200>(sheet);

    let (_person_a, _drill_1, _d1, _s1, _l1) =
        drill_holes::<2000, 8000, 10_000, 900, 880, 20>(person, drill_1, p1);
    let (_person_b, _drill_2, _d2, _s2, _l2) =
        drill_holes::<2000, 8000, 10_000, 900, 880, 20>(person, drill_2, p2);
}
