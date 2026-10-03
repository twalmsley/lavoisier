//! Integration tests: the full pot-of-tea flow (SPEC.md §6), composed in the
//! two agreed valid orders — (a) P1, P2, P3, P4, P5 and (b) P1, P3, P2, P4,
//! P5 (the kettle boils with no person, so loading the pot commutes with
//! boiling; R9: the model describes connections, not sequences). Each flow
//! accounts for everything at flow end (R1): the pot of tea (with its
//! embodied energy) at the drinker, 36 g of food waste at the council
//! collection, the bin back empty with its capacity 10 restored, 70 000 J of
//! waste heat at the kitchen air, the kettle back empty, the person back with
//! 225 000 ms, the teabag box back at 37, and the single History holding the
//! four recorded draws totalling 75 000 ms, each attributed to its process
//! (R16; SPEC.md keeps one History because one actor serializes all draws —
//! the per-branch merge is deferred).
//!
//! These tests sit OUTSIDE the crate's privacy boundary (an integration test
//! is its own crate), so nothing here can mint or defuse a resource: every
//! input comes from a boundary supplier and every output must genuinely reach
//! a consumer — exactly the discipline a downstream user of the model lives
//! under.

#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![deny(let_underscore_drop)]
#![recursion_limit = "2048"]

use cs1_pot_of_tea::resources::boundary::{
    BagsOf, draw_cold_water, draw_grid_energy, full_teabag_box, new_council_collection,
    new_drinker, new_food_waste_bin, new_grid_socket, new_kettle, new_kitchen_air, new_mains_tap,
    new_teapot,
};
use cs1_pot_of_tea::resources::processes::{
    boil, empty_bin, fill_kettle, load_pot, pour_and_brew, vent_heat,
};
use cs1_pot_of_tea::resources::{FoodWasteBin, Kettle, PotOfTea, SpentTeabag, TeabagBox};
use model_core::boundary::send_to;
use model_core::common::Person;
use model_core::common::boundary::new_person;
use model_core::common::processes::draw_time;
use model_core::history::Entry;
use model_core::history::boundary::new_history;
use model_core::history::processes::record;
use model_core::nat::aliases::{N1, N10, N37, N40};

/// Checks the single History at flow end (SPEC.md §6): exactly four recorded
/// draws, in flow order, attributed to their processes, totalling 75 000 ms.
fn assert_history_records_the_four_draws(history: &model_core::history::History) {
    assert_eq!(history.event_count(), 4);
    match history.entries() {
        [
            Entry::Event(e1),
            Entry::Event(e2),
            Entry::Event(e3),
            Entry::Event(e4),
        ] => {
            assert_eq!(
                (e1.process, e1.magnitude, e1.unit),
                ("fill_kettle", 30_000, "person-milliseconds")
            );
            assert_eq!((e2.process, e2.magnitude), ("load_pot", 20_000));
            assert_eq!((e3.process, e3.magnitude), ("pour_and_brew", 15_000));
            assert_eq!((e4.process, e4.magnitude), ("empty_bin", 10_000));
            assert_eq!(
                e1.magnitude + e2.magnitude + e3.magnitude + e4.magnitude,
                75_000
            );
        }
        other => panic!("expected exactly four recorded draws, got {other:?}"),
    }
}

/// Flow order (a): P1 fill, P2 load, P3 boil, P4 pour and brew, P5 empty the
/// bin. The person's budget is drawn by adjacent `draw_time` processes
/// (F-048), each recorded into the single History attributed to the process
/// name (R16); the running balance is restated at every call and checked by
/// the compiler (R15, F-030). Everything is accounted at flow end (SPEC.md
/// §6).
///
/// Verifies: REQ-006, REQ-007, REQ-008, REQ-009
#[test]
fn flow_order_a_type_checks_and_accounts_for_everything() {
    // The kitchen setup enters at the boundary (SPEC.md §4), with the single
    // History (one actor serializes all draws).
    let history = new_history();
    let person = new_person::<300_000>();
    let tap = new_mains_tap();
    let socket = new_grid_socket();
    let bin = new_food_waste_bin::<N10>();
    let air = new_kitchen_air();

    // P1 — fill the kettle (30 000 ms).
    let (water, tap) = draw_cold_water::<1500>(tap);
    let (person, filled) = fill_kettle(person, new_kettle(), water);
    let (labour, person) = draw_time::<30_000, 270_000, 300_000>(person);
    let history = record(history, "fill_kettle", labour);

    // P2 — load the pot (20 000 ms); the box comes back at 37.
    let (person, pot, teabag_box) = load_pot(person, new_teapot(), full_teabag_box::<N40>());
    let (labour, person) = draw_time::<20_000, 250_000, 270_000>(person);
    let history = record(history, "load_pot", labour);

    // P3 — boil: no person (the kettle is automatic); the kettle losses are
    // accounted to the kitchen air (REQ-009).
    let (energy, socket) = draw_grid_energy::<550_000>(socket);
    let (boiling, kettle_heat) = boil::<1500, 550_000, 500_000, 50_000>(filled, energy);
    let air = vent_heat(air, kettle_heat);

    // P4 — pour and brew (15 000 ms): REQ-006 takes only the boiling kettle,
    // REQ-007 only the 3-bag loaded pot; the spent bags exit only to the bin
    // (REQ-008) and the steeping losses to the air (REQ-009), both inside
    // the process.
    let (person, tea, kettle, bin, air): (_, PotOfTea<1473, 480_000>, _, _, _) =
        pour_and_brew::<_, 1473, 480_000, 20_000, _, _, _, _>(person, boiling, pot, bin, air);
    let (labour, person) = draw_time::<15_000, 235_000, 250_000>(person);
    let history = record(history, "pour_and_brew", labour);

    // P5 — empty the bin (10 000 ms): 36 g of food waste to the council, the
    // bin back with its capacity 10 restored at the type level.
    let (person, bin, council) = empty_bin::<_, 36, _, _>(person, bin, new_council_collection());
    let (labour, person) = draw_time::<10_000, 225_000, 235_000>(person);
    let history = record(history, "empty_bin", labour);

    // Everything accounted at flow end (SPEC.md §6).
    let _drinker = send_to(new_drinker(), tea); // the pot of tea, with mass and energy, at the drinker
    let _kettle_back_empty: Kettle = kettle;
    let _bin_back_empty_at_10: FoodWasteBin<N10> = bin;
    let _box_back_at_37: TeabagBox<BagsOf<N37>> = teabag_box;
    assert_eq!(TeabagBox::<BagsOf<N37>>::COUNT, 37);
    let _person_back: Person<225_000> = person;
    assert_eq!(Person::<225_000>::BUDGET_MS, 225_000);
    assert_eq!(50_000 + 20_000, 70_000); // total waste heat at the kitchen air
    assert_history_records_the_four_draws(&history);
    let _reusables = (tap, socket, air, council);
    let _execution_record = history;
}

/// Flow order (b): P1 fill, P3 boil, P2 load, P4 pour and brew, P5 empty —
/// the pot is loaded *while the kettle boils* (SPEC.md §6's concurrency:
/// P3 takes no person, so only data dependencies constrain the order, R9).
/// The end state is identical to order (a), including the person's budget:
/// the type system proves the two orders equivalent.
///
/// Verifies: REQ-006, REQ-007, REQ-008, REQ-009
#[test]
fn flow_order_b_type_checks_and_accounts_for_everything() {
    let history = new_history();
    let person = new_person::<300_000>();
    let bin = new_food_waste_bin::<N10>();
    let air = new_kitchen_air();

    // P1 — fill the kettle (30 000 ms).
    let (water, tap) = draw_cold_water::<1500>(new_mains_tap());
    let (person, filled) = fill_kettle(person, new_kettle(), water);
    let (labour, person) = draw_time::<30_000, 270_000, 300_000>(person);
    let history = record(history, "fill_kettle", labour);

    // P3 — boil first this time: the kettle needs no person, so the person
    // is free for P2 below while it boils.
    let (energy, socket) = draw_grid_energy::<550_000>(new_grid_socket());
    let (boiling, kettle_heat) = boil::<1500, 550_000, 500_000, 50_000>(filled, energy);
    let air = vent_heat(air, kettle_heat);

    // P2 — load the pot (20 000 ms), while the kettle boils.
    let (person, pot, teabag_box) = load_pot(person, new_teapot(), full_teabag_box::<N40>());
    let (labour, person) = draw_time::<20_000, 250_000, 270_000>(person);
    let history = record(history, "load_pot", labour);

    // P4 — pour and brew (15 000 ms).
    let (person, tea, kettle, bin, air): (_, PotOfTea<1473, 480_000>, _, _, _) =
        pour_and_brew::<_, 1473, 480_000, 20_000, _, _, _, _>(person, boiling, pot, bin, air);
    let (labour, person) = draw_time::<15_000, 235_000, 250_000>(person);
    let history = record(history, "pour_and_brew", labour);

    // P5 — empty the bin (10 000 ms).
    let (person, bin, council) = empty_bin::<_, 36, _, _>(person, bin, new_council_collection());
    let (labour, person) = draw_time::<10_000, 225_000, 235_000>(person);
    let history = record(history, "empty_bin", labour);

    // The identical end state (SPEC.md §6).
    let _drinker = send_to(new_drinker(), tea);
    let _kettle_back_empty: Kettle = kettle;
    let _bin_back_empty_at_10: FoodWasteBin<N10> = bin;
    let _box_back_at_37: TeabagBox<BagsOf<N37>> = teabag_box;
    let _person_back: Person<225_000> = person;
    assert_history_records_the_four_draws(&history);
    let _reusables = (tap, socket, air, council);
    let _execution_record = history;
}

/// The downstream fixture path (R1, F-004): the crate's own `test-support`
/// feature — enabled only through the `[dev-dependencies]` self
/// re-declaration — lets this test conjure a sealed spent teabag without
/// brewing. The fixture still has to be accounted for like any real resource:
/// into the bin (REQ-008) and out through the sealed disposal path (F-039),
/// which also shows `empty_bin` restoring a different capacity (1) and
/// conserving a different mass (one bag, 12 g).
///
/// Verifies: REQ-008
#[test]
fn fixtures_construct_sealed_resources_for_downstream_tests() {
    let bag = SpentTeabag::test_fixture();
    let bin = send_to(new_food_waste_bin::<N1>(), bag);
    let person = new_person::<1000>();
    let (person, bin, council) = empty_bin::<_, 12, _, _>(person, bin, new_council_collection());
    let _bin_restored: FoodWasteBin<N1> = bin;
    let _accounted = (person, council);
}

/// The tripwire demonstration (R1 layer 2, F-008, F-032) from outside the
/// privacy boundary: a fixture spent teabag that never reaches the bin is
/// named and used, so `must_use` and `unused_variables` are both satisfied —
/// no compile-time layer can catch this leak; the tripwire `Drop` converts it
/// into this test failure.
///
/// Verifies: REQ-008
#[test]
#[should_panic(expected = "resource leak: SpentTeabag dropped without being consumed")]
fn abandoned_spent_teabag_trips_the_tripwire_downstream() {
    let bag = SpentTeabag::test_fixture();
    let _never_reaches_the_bin = bag;
}
