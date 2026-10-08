//! Integration test: the slice's flow (experiment arm), authored through the
//! `model!` flow grammar, outside the privacy boundary — nothing here can
//! mint or defuse a resource (R1). Mirrors `exp13-control/tests/flows.rs`
//! step for step.

#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![deny(let_underscore_drop)]
#![recursion_limit = "2048"]

use exp13_dsl::model::boundary::{
    draw_cold_water, draw_grid_energy, new_drinker, new_grid_socket, new_kettle, new_kitchen_air,
    new_mains_tap,
};
use exp13_dsl::model::processes::{boil, fill_kettle, pour_cuppa};
use model_core::common::Person;
use model_core::common::boundary::new_person;
use model_core::common::processes::draw_time;
use model_core::history::boundary::new_history;
use model_core::history::processes::record;

exp13_dsl::model! {
    /// The flow: draw water, fill (30 000 ms drawn adjacently, F-048), boil
    /// (no person), vent the kettle loss, pour at the boil (REQ-002), hot
    /// water to the drinker, pour loss to the air. Everything is accounted
    /// at flow end.
    ///
    /// Verifies: REQ-002
    flow test fn flow_makes_hot_water_and_accounts_for_everything {
        // The kitchen setup enters at the boundary (R12).
        rust {
            let history = new_history();
            let person = new_person::<300_000>();
            let tap = new_mains_tap();
            let socket = new_grid_socket();
            let air = new_kitchen_air();
            let drinker = new_drinker();
        }

        // P1 — fill the kettle (30 000 ms drawn adjacently, F-048).
        step (water, tap) = draw_cold_water::<1500>(tap);
        step (person, filled) = fill_kettle(person, new_kettle(), water);
        step (labour, person) = draw_time::<30_000, 270_000, 300_000>(person);
        step history = record(history, "fill_kettle", labour);

        // P3 — boil: no person; the kettle losses go to the kitchen air.
        step (energy, socket) = draw_grid_energy::<550_000>(socket);
        step (boiling, kettle_heat) = boil::<1500, 550_000, 500_000, 50_000>(filled, energy);
        send air <- kettle_heat;

        // P4 — pour at the boil (REQ-002): hot water to the drinker, the
        // pour loss to the air, the kettle back empty.
        step (kettle, cuppa, pour_loss) = pour_cuppa::<1500, 490_000, 10_000, _>(boiling);
        send drinker <- cuppa;
        send air <- pour_loss;

        // Flow end: everything accounted (R1).
        rust {
            assert_eq!(history.event_count(), 1);
            let _person_keeps_270_000: Person<270_000> = person;
            let _reusables = (tap, socket, kettle);
            let _boundary = (air, drinker);
            let _execution_record = history;
        }
    }
}
