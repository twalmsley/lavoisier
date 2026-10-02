//! Leak-surface probe (candidate R17): discarding the whole `Result` in
//! statement position is caught at compile time by `deny(unused_must_use)`
//! (std marks `Result` `#[must_use]`; the bundles carry their own R1
//! messages on top).

#![deny(unused_must_use)]

use exp10_failure_modes::model::boundary::{outcome_failure, supply_drill_bit, supply_plate};
use exp10_failure_modes::model::processes::drill_fallible;
use model_core::common::boundary::new_person;

fn main() {
    drill_fallible::<1000, 4000, 5000, 450, 440, 10, 430, 20>(
        new_person::<5000>(),
        supply_drill_bit(),
        supply_plate::<450>(),
        outcome_failure(),
    );
}
