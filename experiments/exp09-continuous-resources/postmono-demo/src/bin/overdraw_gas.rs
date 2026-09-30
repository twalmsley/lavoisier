// 6000 g from a 5000 g bottle: post-monomorphization E0080.
use exp09_continuous_resources::model::boundary::{fill_gas_bottle, new_depot};
use exp09_continuous_resources::model::processes::draw_gas;
use exp09_continuous_resources::boundary_traits::send_to;

fn main() {
    let bottle = fill_gas_bottle::<5000>();
    let (gas, rest) = draw_gas::<6000, 0, 5000>(bottle);
    let depot = send_to(new_depot(), rest);
    let _keep = (gas, depot);
}
