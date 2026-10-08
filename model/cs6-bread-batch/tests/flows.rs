//! Integration tests: the full batch flow (SPEC.md §6), both agreed orders
//! and all four outcome combinations — both baked, first scorched, second
//! scorched, both scorched. Started from the specgen scaffold's two
//! success-token flow skeletons (R22) and hand-extended to the R17/F-050
//! outcome grouping the spec states. Each combination accounts for
//! everything at its end (R1): the baked loaves at the household (REQ-030),
//! the scorched loaves at compost (REQ-031), the steam and waste heat in the
//! atmosphere (fed inside `bake`), the spent sachets and empty box at
//! recycling, the pantry remainders (500/482/238 g) resting at the boundary,
//! both used tins (453 g), the oven back, the baker back with 900 000 ms,
//! both tokens consumed, and the single History holding one attributed event
//! per draw (R16; one actor serializes all draws).
//!
//! These tests sit OUTSIDE the crate's privacy boundary (an integration test
//! is its own crate), so nothing here can mint or defuse a resource: every
//! input comes from a boundary supplier and every output must genuinely reach
//! a consumer or a boundary rest — exactly the discipline a downstream user
//! of the model lives under.

#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![deny(let_underscore_drop)]
#![recursion_limit = "2048"]

use cs6_bread_batch::flows::{BatchOutcome, KitchenAtRest, bake_batch, bake_batch_tins_first};
use cs6_bread_batch::resources::boundary::{
    bake_outcome_will_fail, bake_outcome_will_succeed, full_yeast_box, new_atmosphere,
    new_clean_tin, new_compost_stream, new_grid, new_household, new_oven, new_pantry_bag,
    new_pantry_block, new_pantry_jar, new_recycling, new_tap, rest_pantry_bag, rest_pantry_block,
    rest_pantry_jar, rest_used_tin,
};
use cs6_bread_batch::resources::{BakedLoaf, ScorchedLoaf};
use model_core::boundary::send_to;
use model_core::common::Person;
use model_core::common::boundary::new_person;
use model_core::history::boundary::new_history;
use model_core::history::{Entry, History};
use model_core::nat::aliases::N2;

/// Checks the single History at a combination's end (SPEC.md §6): the
/// expected recorded draws, in flow order, attributed to their processes,
/// totalling the batch's drawn time (6 300 000 ms over 7 events in every
/// combination).
fn assert_history_records(history: &History, expected: &[(&str, u64)]) {
    assert_eq!(history.event_count(), expected.len());
    let mut total = 0;
    for (entry, (process, magnitude)) in history.entries().iter().zip(expected) {
        match entry {
            Entry::Event(e) => {
                assert_eq!((e.process, e.magnitude), (*process, *magnitude));
                assert_eq!(e.unit, "person-milliseconds");
                total += e.magnitude;
            }
            other => panic!("expected only plain recorded draws, got {other:?}"),
        }
    }
    let expected_total: u64 = expected.iter().map(|(_, m)| m).sum();
    assert_eq!(total, expected_total);
    assert_eq!(expected_total, 6_300_000);
}

/// The §6 order (a) draws: P1 P2 P3 P4 P5 P6 P6.
const ORDER_A_DRAWS: [(&str, u64); 7] = [
    ("mix_dough", 900_000),
    ("knead", 600_000),
    ("prove", 3_600_000),
    ("grease_tins", 120_000),
    ("divide_and_shape", 480_000),
    ("bake", 300_000),
    ("bake", 300_000),
];

/// The §6 order (b) draws: P4 P1 P2 P3 P5 P6 P6.
const ORDER_B_DRAWS: [(&str, u64); 7] = [
    ("grease_tins", 120_000),
    ("mix_dough", 900_000),
    ("knead", 600_000),
    ("prove", 3_600_000),
    ("divide_and_shape", 480_000),
    ("bake", 300_000),
    ("bake", 300_000),
];

/// Accounts for everything every combination ends with identically
/// (SPEC.md §6 "everything accounted"): the pantry remainders rest at the
/// boundary, both 453 g used tins rest, the baker is back at 900 000 ms, and
/// the oven, tap, grid, atmosphere and recycling stay with the caller.
fn account_for_the_kitchen(rest: KitchenAtRest, expected_draws: &[(&str, u64)]) {
    let KitchenAtRest {
        baker,
        tin_1,
        tin_2,
        oven,
        pantry_bag,
        pantry_jar,
        pantry_block,
        tap,
        grid,
        atmosphere,
        recycling,
        history,
    } = rest;
    // The baker keeps 900 000 ms of the 2-hour budget (typed, R15).
    let _baker_back: Person<900_000> = baker;
    // Container remainders (500/482/238 g) rest at the boundary (R12).
    rest_pantry_bag(pantry_bag);
    rest_pantry_jar(pantry_jar);
    rest_pantry_block(pantry_block);
    // Both tins come back used at 453 g (butter residue, SPEC.md §8 item 4).
    rest_used_tin(tin_1);
    rest_used_tin(tin_2);
    // 7 attributed events totalling 6 300 000 ms (R16) — the same drawn time
    // in every combination (SPEC.md §6).
    assert_history_records(&history, expected_draws);
    let _reusables = (oven, tap, grid, atmosphere, recycling);
    let _execution_record = history;
}

/// Combination 1 — both baked, order (a): 2 baked loaves to the household,
/// compost empty, and the whole kitchen at rest.
///
/// Verifies: REQ-028, REQ-029, REQ-030
#[test]
fn order_a_both_baked_accounts_for_everything() {
    let (out, rest) = bake_batch(
        new_person::<7_200_000>(),
        new_pantry_bag(),
        new_pantry_jar(),
        new_pantry_block(),
        new_tap(),
        new_grid(),
        full_yeast_box::<N2>(),
        new_clean_tin(),
        new_clean_tin(),
        new_oven(),
        bake_outcome_will_succeed(),
        bake_outcome_will_succeed(),
        new_household(),
        new_compost_stream(),
        new_atmosphere(),
        new_recycling(),
        new_history(),
    );
    match out {
        BatchOutcome::BothBaked { household, compost } => {
            let _sinks = (household, compost);
        }
        _ => unreachable!("two success tokens must realise the both-baked combination"),
    }
    account_for_the_kitchen(rest, &ORDER_A_DRAWS);
}

/// Combination 2 — first scorched, order (a): 1 baked loaf to the household,
/// 1 scorched to compost (REQ-031), same time and energy as every other
/// combination (a scorch costs the full bake, R17).
///
/// Verifies: REQ-028, REQ-029, REQ-030, REQ-031
#[test]
fn order_a_first_scorched_accounts_for_everything() {
    let (out, rest) = bake_batch(
        new_person::<7_200_000>(),
        new_pantry_bag(),
        new_pantry_jar(),
        new_pantry_block(),
        new_tap(),
        new_grid(),
        full_yeast_box::<N2>(),
        new_clean_tin(),
        new_clean_tin(),
        new_oven(),
        bake_outcome_will_fail(),
        bake_outcome_will_succeed(),
        new_household(),
        new_compost_stream(),
        new_atmosphere(),
        new_recycling(),
        new_history(),
    );
    match out {
        BatchOutcome::FirstScorched { household, compost } => {
            let _sinks = (household, compost);
        }
        _ => unreachable!("fail-then-success must realise the first-scorched combination"),
    }
    account_for_the_kitchen(rest, &ORDER_A_DRAWS);
}

/// Combination 3 — second scorched, order (a): as combination 2 with the
/// bake order swapped (SPEC.md §6).
///
/// Verifies: REQ-028, REQ-029, REQ-030, REQ-031
#[test]
fn order_a_second_scorched_accounts_for_everything() {
    let (out, rest) = bake_batch(
        new_person::<7_200_000>(),
        new_pantry_bag(),
        new_pantry_jar(),
        new_pantry_block(),
        new_tap(),
        new_grid(),
        full_yeast_box::<N2>(),
        new_clean_tin(),
        new_clean_tin(),
        new_oven(),
        bake_outcome_will_succeed(),
        bake_outcome_will_fail(),
        new_household(),
        new_compost_stream(),
        new_atmosphere(),
        new_recycling(),
        new_history(),
    );
    match out {
        BatchOutcome::SecondScorched { household, compost } => {
            let _sinks = (household, compost);
        }
        _ => unreachable!("success-then-fail must realise the second-scorched combination"),
    }
    account_for_the_kitchen(rest, &ORDER_A_DRAWS);
}

/// Combination 4 — both scorched, order (a): the household receives nothing,
/// both loaves reach compost (REQ-031), and the kitchen still ends at the
/// identical rest state — failure conserves too (R17).
///
/// Verifies: REQ-028, REQ-029, REQ-031
#[test]
fn order_a_both_scorched_accounts_for_everything() {
    let (out, rest) = bake_batch(
        new_person::<7_200_000>(),
        new_pantry_bag(),
        new_pantry_jar(),
        new_pantry_block(),
        new_tap(),
        new_grid(),
        full_yeast_box::<N2>(),
        new_clean_tin(),
        new_clean_tin(),
        new_oven(),
        bake_outcome_will_fail(),
        bake_outcome_will_fail(),
        new_household(),
        new_compost_stream(),
        new_atmosphere(),
        new_recycling(),
        new_history(),
    );
    match out {
        BatchOutcome::BothScorched { household, compost } => {
            let _sinks = (household, compost);
        }
        _ => unreachable!("two failure tokens must realise the both-scorched combination"),
    }
    account_for_the_kitchen(rest, &ORDER_A_DRAWS);
}

/// The ordering variant (R9, SPEC.md §6 order b): the tins are greased
/// before the dough is mixed, and the both-baked combination still ends in
/// the identical state (same budgets, same remainders, same 7 events in the
/// order-b sequence) — the type system proving the orderings equivalent.
///
/// Verifies: REQ-028, REQ-029, REQ-030
#[test]
fn order_b_both_baked_ends_in_the_identical_state() {
    let (out, rest) = bake_batch_tins_first(
        new_person::<7_200_000>(),
        new_pantry_bag(),
        new_pantry_jar(),
        new_pantry_block(),
        new_tap(),
        new_grid(),
        full_yeast_box::<N2>(),
        new_clean_tin(),
        new_clean_tin(),
        new_oven(),
        bake_outcome_will_succeed(),
        bake_outcome_will_succeed(),
        new_household(),
        new_compost_stream(),
        new_atmosphere(),
        new_recycling(),
        new_history(),
    );
    match out {
        BatchOutcome::BothBaked { household, compost } => {
            let _sinks = (household, compost);
        }
        _ => unreachable!("two success tokens must realise the both-baked combination"),
    }
    account_for_the_kitchen(rest, &ORDER_B_DRAWS);
}

/// The ordering variant through a failure path: order (b) with both bakes
/// scorched ends in the identical both-scorched state as order (a) — the
/// ordering freedom and the fallibility compose (R9, R17).
///
/// Verifies: REQ-028, REQ-029, REQ-031
#[test]
fn order_b_both_scorched_ends_in_the_identical_state() {
    let (out, rest) = bake_batch_tins_first(
        new_person::<7_200_000>(),
        new_pantry_bag(),
        new_pantry_jar(),
        new_pantry_block(),
        new_tap(),
        new_grid(),
        full_yeast_box::<N2>(),
        new_clean_tin(),
        new_clean_tin(),
        new_oven(),
        bake_outcome_will_fail(),
        bake_outcome_will_fail(),
        new_household(),
        new_compost_stream(),
        new_atmosphere(),
        new_recycling(),
        new_history(),
    );
    match out {
        BatchOutcome::BothScorched { household, compost } => {
            let _sinks = (household, compost);
        }
        _ => unreachable!("two failure tokens must realise the both-scorched combination"),
    }
    account_for_the_kitchen(rest, &ORDER_B_DRAWS);
}

/// The downstream fixture path (R1, F-004): the crate's own `test-support`
/// feature — enabled only through the `[dev-dependencies]` self
/// re-declaration — lets this test conjure sealed loaves without running the
/// flow. The fixtures still have to be accounted for like real resources:
/// the baked loaf genuinely reaches the household (its only sink, REQ-030)
/// and the scorched loaf genuinely reaches the compost stream (its only
/// sink, REQ-031).
///
/// Verifies: REQ-030, REQ-031
#[test]
fn fixtures_construct_sealed_loaves_for_downstream_tests() {
    let baked = BakedLoaf::test_fixture();
    let scorched = ScorchedLoaf::test_fixture();
    let _household = send_to(new_household(), baked);
    let _compost = send_to(new_compost_stream(), scorched);
}

/// The tripwire demonstration (R1 layer 2, F-008, F-032) from outside the
/// privacy boundary: a fixture baked loaf that is never handed over is named
/// and used, so `must_use` and `unused_variables` are both satisfied — no
/// compile-time layer can catch this leak; the tripwire `Drop` converts it
/// into this test failure.
///
/// Verifies: REQ-030
#[test]
#[should_panic(expected = "resource leak: BakedLoaf dropped without being consumed")]
fn abandoned_baked_loaf_trips_the_tripwire_downstream() {
    let baked = BakedLoaf::test_fixture();
    let _never_handed_over = baked;
}
