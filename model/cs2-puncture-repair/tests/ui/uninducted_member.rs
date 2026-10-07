// R18: a plain `Person` carries no induction, so REQ-010 refuses them at the
// workstand (E0277). The top line is REQ-010's own on_unimplemented message
// (R10 rule 8, F-044) — the requirement trait's attribute, not the failing
// marker's, because the marker fails as a supertrait obligation of the
// requirement bound.
use cs2_puncture_repair::resources::boundary::{new_workstand, wheel_arrives_for_service};
use cs2_puncture_repair::resources::processes::open_wheel;
use model_core::common::boundary::new_person;

fn main() {
    let person = new_person::<1_800_000>(); // never inducted
    let r = open_wheel(person, new_workstand(), wheel_arrives_for_service());
}
