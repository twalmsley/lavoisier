// Waste heat bound but never consumed: caught by unused_variables (lint layer),
// which is an error only under deny / CI's -D warnings.
#![deny(unused_variables)]
use exp09_continuous_resources::model::boundary::{
    draw_air, fill_gas_bottle, new_depot, new_work_sink, the_atmosphere,
};
use exp09_continuous_resources::model::processes::{burn, draw_gas};
use exp09_continuous_resources::boundary_traits::send_to;

fn main() {
    let bottle = fill_gas_bottle::<300>();
    let (gas, empty) = draw_gas::<300, 0, 300>(bottle);
    let atm = the_atmosphere();
    let (air, atm) = draw_air::<300>(atm);
    let (exhaust, heat, work) = burn::<300, 300, 600, 11000, 4000>(gas, air);
    let _atm = send_to(atm, exhaust);
    let _sink = send_to(new_work_sink(), work);
    let _depot = send_to(new_depot(), empty);
    // `heat` is never sent anywhere.
}
