// Fuel and air transposed: distinct sealed quantity types make the unit/role
// mix-up a crisp type-check-time E0308.
use exp09_continuous_resources::model::boundary::{draw_air, fill_gas_bottle, the_atmosphere};
use exp09_continuous_resources::model::processes::{burn, draw_gas};

fn main() {
    let bottle = fill_gas_bottle::<1000>();
    let (gas, _bottle) = draw_gas::<300, 700, 1000>(bottle);
    let (air, _atm) = draw_air::<300>(the_atmosphere());
    let _out = burn::<300, 300, 600, 11000, 4000>(air, gas);
}
