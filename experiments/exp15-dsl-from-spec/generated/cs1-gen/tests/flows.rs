//! GENERATED flow skeletons (SPEC.md §6): each agreed valid order composed
//! as an integration test, with the person's budget drawn by adjacent
//! `draw_time` steps recorded to a single History (R16, F-048).

#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![deny(let_underscore_drop)]
#![recursion_limit = "2048"]

use cs1_gen::resources::boundary::{draw_cold_water, draw_electrical_energy, full_box_of_teabags, new_council_food_waste_collection, new_drinker, new_food_waste_bin, new_grid_socket, new_kettle, new_kitchen_air, new_mains_tap, new_teapot};
use cs1_gen::resources::processes::{fill_kettle, load_pot, boil, pour_and_brew, empty_bin};
use model_core::boundary::send_to;
use model_core::common::Person;
use model_core::common::boundary::new_person;
use model_core::common::processes::draw_time;
use model_core::history::boundary::new_history;
use model_core::history::processes::record;
use model_core::nat::aliases::{N10, N40};

/// GENERATED flow order (a) from SPEC.md §6 (line 117): P1, P2, P3, P4, P5.
///
/// Verifies: REQ-006, REQ-007, REQ-008, REQ-009
#[test]
fn flow_order_a_type_checks_and_accounts_for_everything() {
    let history = new_history();
    let person = new_person::<300_000>();
    let food_waste_bin = new_food_waste_bin::<N10>();
    let drinker = new_drinker();
    let council_food_waste_collection = new_council_food_waste_collection();
    let kitchen_air = new_kitchen_air();
    // P1 — Fill the kettle (SPEC.md §5 line 71).
    let kettle = new_kettle();
    let mains_tap = new_mains_tap();
    let (cold_water, mains_tap) = draw_cold_water::<1_500>(mains_tap);
    let (person, filled_kettle) = fill_kettle(person, kettle, cold_water);
    let (labour, person) = draw_time::<30_000, 270_000, 300_000>(person);
    let history = record(history, "fill_kettle", labour);
    // P2 — Load the pot (SPEC.md §5 line 78).
    let teapot = new_teapot();
    let box_of_teabags = full_box_of_teabags::<N40>();
    let (person, loaded_teapot, box_of_teabags) = load_pot(person, teapot, box_of_teabags);
    let (labour, person) = draw_time::<20_000, 250_000, 270_000>(person);
    let history = record(history, "load_pot", labour);
    // P3 — Boil (SPEC.md §5 line 85).
    let grid_socket = new_grid_socket();
    let (electrical_energy, grid_socket) = draw_electrical_energy::<550_000>(grid_socket);
    let (boiling_kettle, waste_heat) = boil(filled_kettle, electrical_energy);
    let kitchen_air = send_to(kitchen_air, waste_heat);
    // P4 — Pour and brew (SPEC.md §5 line 96).
    let (person, pot_of_tea, spent_teabag_1, spent_teabag_2, spent_teabag_3, kettle, waste_heat) = pour_and_brew(person, boiling_kettle, loaded_teapot);
    let kitchen_air = send_to(kitchen_air, waste_heat);
    let food_waste_bin = send_to(food_waste_bin, spent_teabag_1);
    let food_waste_bin = send_to(food_waste_bin, spent_teabag_2);
    let food_waste_bin = send_to(food_waste_bin, spent_teabag_3);
    let (labour, person) = draw_time::<15_000, 235_000, 250_000>(person);
    let history = record(history, "pour_and_brew", labour);
    // P5 — Empty the bin (SPEC.md §5 line 107).
    let (person, food_waste_bin, council_food_waste_collection) = empty_bin(person, food_waste_bin, council_food_waste_collection);
    let (labour, person) = draw_time::<10_000, 225_000, 235_000>(person);
    let history = record(history, "empty_bin", labour);
    // Everything accounted at flow end (SPEC.md §6).
    let drinker = send_to(drinker, pot_of_tea);
    let _person_back: Person<225_000> = person;
    let _flow_end = (drinker, kitchen_air, mains_tap, box_of_teabags, grid_socket, kettle, food_waste_bin, council_food_waste_collection, history);
}

/// GENERATED flow order (b) from SPEC.md §6 (line 117): P1, P3, P2, P4, P5.
///
/// Verifies: REQ-006, REQ-007, REQ-008, REQ-009
#[test]
fn flow_order_b_type_checks_and_accounts_for_everything() {
    let history = new_history();
    let person = new_person::<300_000>();
    let food_waste_bin = new_food_waste_bin::<N10>();
    let drinker = new_drinker();
    let council_food_waste_collection = new_council_food_waste_collection();
    let kitchen_air = new_kitchen_air();
    // P1 — Fill the kettle (SPEC.md §5 line 71).
    let kettle = new_kettle();
    let mains_tap = new_mains_tap();
    let (cold_water, mains_tap) = draw_cold_water::<1_500>(mains_tap);
    let (person, filled_kettle) = fill_kettle(person, kettle, cold_water);
    let (labour, person) = draw_time::<30_000, 270_000, 300_000>(person);
    let history = record(history, "fill_kettle", labour);
    // P3 — Boil (SPEC.md §5 line 85).
    let grid_socket = new_grid_socket();
    let (electrical_energy, grid_socket) = draw_electrical_energy::<550_000>(grid_socket);
    let (boiling_kettle, waste_heat) = boil(filled_kettle, electrical_energy);
    let kitchen_air = send_to(kitchen_air, waste_heat);
    // P2 — Load the pot (SPEC.md §5 line 78).
    let teapot = new_teapot();
    let box_of_teabags = full_box_of_teabags::<N40>();
    let (person, loaded_teapot, box_of_teabags) = load_pot(person, teapot, box_of_teabags);
    let (labour, person) = draw_time::<20_000, 250_000, 270_000>(person);
    let history = record(history, "load_pot", labour);
    // P4 — Pour and brew (SPEC.md §5 line 96).
    let (person, pot_of_tea, spent_teabag_1, spent_teabag_2, spent_teabag_3, kettle, waste_heat) = pour_and_brew(person, boiling_kettle, loaded_teapot);
    let kitchen_air = send_to(kitchen_air, waste_heat);
    let food_waste_bin = send_to(food_waste_bin, spent_teabag_1);
    let food_waste_bin = send_to(food_waste_bin, spent_teabag_2);
    let food_waste_bin = send_to(food_waste_bin, spent_teabag_3);
    let (labour, person) = draw_time::<15_000, 235_000, 250_000>(person);
    let history = record(history, "pour_and_brew", labour);
    // P5 — Empty the bin (SPEC.md §5 line 107).
    let (person, food_waste_bin, council_food_waste_collection) = empty_bin(person, food_waste_bin, council_food_waste_collection);
    let (labour, person) = draw_time::<10_000, 225_000, 235_000>(person);
    let history = record(history, "empty_bin", labour);
    // Everything accounted at flow end (SPEC.md §6).
    let drinker = send_to(drinker, pot_of_tea);
    let _person_back: Person<225_000> = person;
    let _flow_end = (drinker, kitchen_air, mains_tap, grid_socket, box_of_teabags, kettle, food_waste_bin, council_food_waste_collection, history);
}

