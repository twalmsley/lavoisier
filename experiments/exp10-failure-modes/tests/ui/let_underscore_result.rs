//! Leak-surface probe (candidate R17): the `let _ = …` escape hatch that
//! `unused_must_use`'s own fix-it suggests (F-007) is caught by
//! `deny(let_underscore_drop)` — the bundles have drop glue through their
//! tripwired fields.

#![deny(let_underscore_drop)]

use exp10_failure_modes::model::boundary::{outcome_failure, supply_drill_bit, supply_plate};
use exp10_failure_modes::model::processes::drill_fallible;
use model_core::common::boundary::new_person;

fn main() {
    let _ = drill_fallible::<1000, 4000, 5000, 450, 440, 10, 430, 20>(
        new_person::<5000>(),
        supply_drill_bit(),
        supply_plate::<450>(),
        outcome_failure(),
    );
}
