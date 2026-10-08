//! Canonical violation 2 (control arm): a non-boiling kettle at the
//! requirement bound — `pour_cuppa` demands REQ-001 (boiling water), and a
//! `FilledKettle` is not at the boil. Expected: E0277 with the REQ-phrased
//! `on_unimplemented` message (R10 rule 8, F-044), at type-check time.

use exp13_control::model::boundary::{draw_cold_water, new_kettle, new_mains_tap};
use exp13_control::model::processes::{fill_kettle, pour_cuppa};
use model_core::common::boundary::new_person;

fn main() {
    let person = new_person::<300_000>();
    let (water, tap) = draw_cold_water::<1500>(new_mains_tap());
    let (person, filled) = fill_kettle(person, new_kettle(), water);
    // The kettle was never boiled: the filled state may not be poured.
    let (kettle, cuppa, loss) = pour_cuppa::<1500, 490_000, 10_000, _>(filled);
    let _ = (person, tap, kettle, cuppa, loss);
}
