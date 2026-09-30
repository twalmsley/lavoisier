// The atmosphere is a Consumer of exhaust and heat, not of labour: a
// type-check-time E0277 carrying the on_unimplemented message (F-015).
use exp09_continuous_resources::boundary_traits::send_to;
use exp09_continuous_resources::model::boundary::{new_person, the_atmosphere};
use exp09_continuous_resources::model::processes::draw_time;

fn main() {
    let atm = the_atmosphere();
    let person = new_person::<1000>();
    let (labour, _person) = draw_time::<500, 500, 1000>(person);
    let _atm = send_to(atm, labour);
}
