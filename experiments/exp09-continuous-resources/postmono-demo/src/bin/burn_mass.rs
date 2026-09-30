// 300 g fuel + 400 g air declared to make 600 g exhaust: mass assert fails.
use exp09_continuous_resources::model::boundary::{draw_air, fill_gas_bottle, the_atmosphere};
use exp09_continuous_resources::model::processes::{burn, draw_gas};

fn main() {
    let bottle = fill_gas_bottle::<1000>();
    let (gas, _bottle) = draw_gas::<300, 700, 1000>(bottle);
    let (air, _atm) = draw_air::<400>(the_atmosphere());
    let _out = burn::<300, 400, 600, 11000, 4000>(gas, air);
}
