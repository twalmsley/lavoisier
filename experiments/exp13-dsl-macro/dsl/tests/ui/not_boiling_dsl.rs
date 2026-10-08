//! Canonical violation 2 (DSL arm): a non-boiling kettle at the requirement
//! bound, authored through the `model!` flow grammar — `pour_cuppa` demands
//! REQ-002 (boiling water), and a `FilledKettle` is not at the boil.
//! Expected: E0277 with the REQ-phrased `on_unimplemented` message (R10 rule
//! 8, F-044), at type-check time.

use exp13_dsl::model::boundary::{draw_cold_water, new_kettle, new_mains_tap};
use exp13_dsl::model::processes::{fill_kettle, pour_cuppa};
use model_core::common::boundary::new_person;

exp13_dsl::model! {
    /// A broken flow: the kettle was never boiled.
    flow pub fn pour_before_the_boil {
        rust { let person = new_person::<300_000>(); }
        step (water, tap) = draw_cold_water::<1500>(new_mains_tap());
        step (person, filled) = fill_kettle(person, new_kettle(), water);
        // The kettle was never boiled: the filled state may not be poured.
        step (kettle, cuppa, loss) = pour_cuppa::<1500, 490_000, 10_000, _>(filled);
        rust { let _accounted = (person, tap, kettle, cuppa, loss); }
    }
}

fn main() {}
