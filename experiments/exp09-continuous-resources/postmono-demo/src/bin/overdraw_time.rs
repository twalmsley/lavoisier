// Spending 6000 ms from a 4000 ms remaining budget: post-mono E0080.
use exp09_continuous_resources::model::boundary::{new_ledger, new_person};
use exp09_continuous_resources::model::processes::draw_time;
use exp09_continuous_resources::boundary_traits::send_to;

fn main() {
    let person = new_person::<10000>();
    let (l1, person) = draw_time::<6000, 4000, 10000>(person);
    let (l2, person) = draw_time::<6000, 0, 4000>(person);
    let ledger = send_to(new_ledger(), l1);
    let _ledger = send_to(ledger, l2);
    let _person = person;
}
