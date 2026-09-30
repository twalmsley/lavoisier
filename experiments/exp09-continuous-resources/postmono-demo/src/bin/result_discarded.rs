// The whole draw result discarded: caught by the must_use layer at check time.
#![deny(unused_must_use)]
use exp09_continuous_resources::model::boundary::fill_gas_bottle;
use exp09_continuous_resources::model::processes::draw_gas;

fn main() {
    let bottle = fill_gas_bottle::<5000>();
    draw_gas::<300, 4700, 5000>(bottle);
}
