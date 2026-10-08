//! EXP-13 canonical conservation violations (E0080, post-monomorphization,
//! F-001), one feature-gated module per (violation × arm) so verbatim
//! transcripts can be captured without breaking the default build.
//!
//! * `energy-*`: the boil energy assert — 500 000 embodied + 60 000 heat
//!   ≠ 550 000 drawn.
//! * `overdraw-*`: the person's time budget — drawing 30 000 ms from a
//!   20 000 ms budget (`model_core::common::processes::draw_time`).
//!
//! The `*-control` modules author the violating flow in plain Rust against
//! `exp13-control`; the `*-dsl` modules author it through `exp13_dsl::model!`'s
//! flow grammar. The flows are `pub fn`s of this library crate, so they are
//! monomorphization roots: `cargo build` (never `cargo check`) surfaces the
//! E0080.

#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![deny(let_underscore_drop)]
#![recursion_limit = "2048"]

/// Violation 1, control arm: the boil energy assert, plain Rust.
#[cfg(feature = "energy-control")]
pub mod energy_control {
    use exp13_control::model::boundary::{
        draw_cold_water, draw_grid_energy, new_grid_socket, new_kettle, new_mains_tap,
    };
    use exp13_control::model::processes::{boil, fill_kettle};
    use model_core::common::boundary::new_person;

    /// Boiling must not lose energy: 500 000 + 60 000 ≠ 550 000.
    pub fn violation_energy_control() {
        let person = new_person::<300_000>();
        let (water, tap) = draw_cold_water::<1500>(new_mains_tap());
        let (person, filled) = fill_kettle(person, new_kettle(), water);
        let (energy, socket) = draw_grid_energy::<550_000>(new_grid_socket());
        let (boiling, heat) = boil::<1500, 550_000, 500_000, 60_000>(filled, energy);
        let _accounted = (person, tap, socket, boiling, heat);
    }
}

/// Violation 1, DSL arm: the boil energy assert, authored through `model!`.
#[cfg(feature = "energy-dsl")]
pub mod energy_dsl {
    use exp13_dsl::model::boundary::{
        draw_cold_water, draw_grid_energy, new_grid_socket, new_kettle, new_mains_tap,
    };
    use exp13_dsl::model::processes::{boil, fill_kettle};
    use model_core::common::boundary::new_person;

    exp13_dsl::model! {
        /// Boiling must not lose energy: 500 000 + 60 000 ≠ 550 000.
        flow pub fn violation_energy_dsl {
            rust { let person = new_person::<300_000>(); }
            step (water, tap) = draw_cold_water::<1500>(new_mains_tap());
            step (person, filled) = fill_kettle(person, new_kettle(), water);
            step (energy, socket) = draw_grid_energy::<550_000>(new_grid_socket());
            step (boiling, heat) = boil::<1500, 550_000, 500_000, 60_000>(filled, energy);
            rust { let _accounted = (person, tap, socket, boiling, heat); }
        }
    }
}

/// Violation 3, control arm: the time-budget overdraw, plain Rust.
#[cfg(feature = "overdraw-control")]
pub mod overdraw_control {
    use model_core::common::boundary::new_person;
    use model_core::common::processes::draw_time;
    use model_core::history::boundary::new_history;
    use model_core::history::processes::record;

    /// 30 000 ms of kettle-filling cannot come out of a 20 000 ms budget.
    pub fn violation_overdraw_control() {
        let person = new_person::<20_000>();
        let (labour, person) = draw_time::<30_000, 0, 20_000>(person);
        let history = record(new_history(), "fill_kettle", labour);
        let _accounted = (person, history);
    }
}

/// Violation 3, DSL arm: the time-budget overdraw, authored through `model!`.
#[cfg(feature = "overdraw-dsl")]
pub mod overdraw_dsl {
    use model_core::common::boundary::new_person;
    use model_core::common::processes::draw_time;
    use model_core::history::boundary::new_history;
    use model_core::history::processes::record;

    exp13_dsl::model! {
        /// 30 000 ms of kettle-filling cannot come out of a 20 000 ms budget.
        flow pub fn violation_overdraw_dsl {
            rust { let person = new_person::<20_000>(); }
            step (labour, person) = draw_time::<30_000, 0, 20_000>(person);
            step history = record(new_history(), "fill_kettle", labour);
            rust { let _accounted = (person, history); }
        }
    }
}
