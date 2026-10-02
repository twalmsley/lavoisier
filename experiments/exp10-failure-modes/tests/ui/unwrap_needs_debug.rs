//! Leak-surface probe (candidate R17): `.unwrap()` on a fallible process's
//! result does not compile, because the failure bundle carries sealed
//! resources and therefore no `Debug` impl. The panicking shortcut past the
//! failure arm is closed at type-check time.

use exp10_failure_modes::model::boundary::{outcome_success, supply_drill_bit, supply_plate};
use exp10_failure_modes::model::processes::drill_fallible;
use model_core::common::boundary::new_person;

fn main() {
    let result = drill_fallible::<1000, 4000, 5000, 450, 440, 10, 430, 20>(
        new_person::<5000>(),
        supply_drill_bit(),
        supply_plate::<450>(),
        outcome_success(),
    );
    let _ok = result.unwrap();
}
