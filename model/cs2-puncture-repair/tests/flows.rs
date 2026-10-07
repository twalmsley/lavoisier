//! Integration tests: the full puncture-repair flow (SPEC.md §6), all three
//! paths — patched first try, patched on retry, spare fitted — plus the
//! wallet-readied-early ordering variant (R9: the only ordering freedoms are
//! trivia such as when the wallet is readied, and the type system proves the
//! orderings equivalent). Each path accounts for everything at its end (R1):
//! the serviceable wheel (with its path-specific mass) at the owner; the
//! member back with their remaining budget; the workstand and pump back; the
//! kit back with its remaining patches and cement; the unspent cash in the
//! wallet; the spent patches in the waste stream; untried tokens returned via
//! the boundary exit; the dead tube (path 3) at recycling; and the single
//! History holding one attributed event per draw (R16; one actor serializes
//! all draws, so the per-branch merge stays deferred to CS-3).
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

use cs2_puncture_repair::characteristics::Induction;
use cs2_puncture_repair::flows::{RepairOutcome, repair_wheel, repair_wheel_wallet_ready_early};
use cs2_puncture_repair::resources::boundary::{
    new_parts_counter, new_patch_kit, new_pump, new_rubber_recycling, new_waste_stream,
    new_wheel_owner, new_workstand, patch_will_fail, patch_will_hold, return_patch_outcome,
    take_wallet_home, wallet_with, wheel_arrives_for_service,
};
use cs2_puncture_repair::resources::processes::refit_and_inflate;
use cs2_puncture_repair::resources::{
    DryPatch, NoPatches, OnePatch, OpenWheel, PatchKit, ServiceableWheel, SpareTube, Wallet,
};
use model_core::boundary::send_to;
use model_core::common::Qualified;
use model_core::common::boundary::{new_person, qualify};
use model_core::history::boundary::new_history;
use model_core::history::processes::record;
use model_core::history::{Entry, History};

/// Checks the single History at a path's end (SPEC.md §6): the expected
/// recorded draws, in flow order, attributed to their processes, totalling
/// the path's drawn time.
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
}

/// Path 1 — patched first try: P1 P2 P3(Ok) P4 P6 (SPEC.md §6). 900 000 ms
/// drawn over 5 attributed events; wallet untouched at 1000 p; 1 spare patch
/// and 29 g of cement back in the kit; the untried second token returned to
/// the environment (F-050); the 2083 g wheel at the owner.
///
/// Verifies: REQ-010, REQ-011, REQ-012
#[test]
fn path_1_patched_first_try_accounts_for_everything() {
    let (out, counter, waste, recycling, history) = repair_wheel(
        qualify::<Induction, 1_800_000>(new_person::<1_800_000>()),
        new_workstand(),
        new_pump(),
        wheel_arrives_for_service(),
        new_patch_kit(),
        wallet_with::<1000>(),
        patch_will_hold(),
        patch_will_fail(), // provisioned but never tried
        new_parts_counter(),
        new_waste_stream(),
        new_rubber_recycling(),
        new_history(),
    );
    match out {
        RepairOutcome::PatchedFirstTry {
            member,
            workstand,
            pump,
            wheel,
            kit,
            wallet,
            reserve_outcome,
        } => {
            // The wheel, at the patched mass, goes to the owner.
            assert_eq!(ServiceableWheel::<2083>::VALUE, 2083);
            let _owner = send_to(new_wheel_owner(), wheel);
            // The member keeps 900 000 ms of the 1 800 000 ms budget.
            let _member: Qualified<Induction, 900_000> = member;
            // The kit remainder stays with the member: 1 patch, 29 g.
            let kit: PatchKit<OnePatch, 29> = kit;
            assert_eq!(PatchKit::<OnePatch, 29>::PATCHES, 1);
            assert_eq!(PatchKit::<OnePatch, 29>::CEMENT_G, 29);
            // The wallet is untouched and goes home with the member.
            let wallet: Wallet<1000> = wallet;
            take_wallet_home(wallet);
            // The untried token is re-accounted at the boundary (F-050).
            return_patch_outcome(reserve_outcome);
            // 5 attributed events totalling 900 000 ms (R16).
            assert_history_records(
                &history,
                &[
                    ("open_wheel", 240_000),
                    ("find_hole", 180_000),
                    ("patch_tube", 120_000),
                    ("check_patch", 60_000),
                    ("refit_and_inflate", 300_000),
                ],
            );
            let _reusables = (workstand, pump, counter, waste, recycling, kit);
            let _execution_record = history;
        }
        _ => unreachable!("a success first trial must take the first-try path"),
    }
}

/// Path 2 — patched on retry: P1 P2 P3(Fail) P3(Ok) P4 P6 (SPEC.md §6).
/// 1 020 000 ms drawn over 6 events (the failed attempt cost its full
/// 120 000 ms too, R17); 1 spent patch (3 g) in the waste stream; the kit
/// empty of patches with 28 g of cement; wallet untouched; the 2083 g wheel
/// at the owner.
///
/// Verifies: REQ-010, REQ-011, REQ-012, REQ-013
#[test]
fn path_2_patched_on_retry_accounts_for_everything() {
    let (out, counter, waste, recycling, history) = repair_wheel(
        qualify::<Induction, 1_800_000>(new_person::<1_800_000>()),
        new_workstand(),
        new_pump(),
        wheel_arrives_for_service(),
        new_patch_kit(),
        wallet_with::<1000>(),
        patch_will_fail(),
        patch_will_hold(),
        new_parts_counter(),
        new_waste_stream(),
        new_rubber_recycling(),
        new_history(),
    );
    match out {
        RepairOutcome::PatchedOnRetry {
            member,
            workstand,
            pump,
            wheel,
            kit,
            wallet,
        } => {
            let _owner = send_to(new_wheel_owner(), wheel);
            let _member: Qualified<Induction, 780_000> = member;
            // Kit: no patches left (a third attempt is inexpressible,
            // F-050), 28 g of cement.
            let kit: PatchKit<NoPatches, 28> = kit;
            assert_eq!(PatchKit::<NoPatches, 28>::PATCHES, 0);
            // One spent patch (2 g patch + 1 g cement = 3 g) reached the
            // waste stream inside P3 (REQ-013; the unbounded sink discards,
            // F-029, so the mass is recovered from the constants, R7).
            assert_eq!(DryPatch::MASS_G + 1, 3);
            take_wallet_home::<1000>(wallet);
            assert_history_records(
                &history,
                &[
                    ("open_wheel", 240_000),
                    ("find_hole", 180_000),
                    ("patch_tube", 120_000),
                    ("patch_tube", 120_000),
                    ("check_patch", 60_000),
                    ("refit_and_inflate", 300_000),
                ],
            );
            let _reusables = (workstand, pump, counter, waste, recycling, kit);
            let _execution_record = history;
        }
        _ => unreachable!("fail-then-success must take the retry path"),
    }
}

/// Path 3 — spare fitted: P1 P2 P3(Fail) P3(Fail) P5 P6 (SPEC.md §6).
/// 1 080 000 ms drawn over 6 events; 6 g of spent patches in the waste
/// stream; the dead tube (180 g) at rubber recycling; 350 p left in the
/// wallet after the exact-price purchase (REQ-014); the 2080 g wheel —
/// REQ-012's *other* satisfying type — at the owner.
///
/// Verifies: REQ-010, REQ-011, REQ-012, REQ-013, REQ-014
#[test]
fn path_3_spare_fitted_accounts_for_everything() {
    let (out, counter, waste, recycling, history) = repair_wheel(
        qualify::<Induction, 1_800_000>(new_person::<1_800_000>()),
        new_workstand(),
        new_pump(),
        wheel_arrives_for_service(),
        new_patch_kit(),
        wallet_with::<1000>(),
        patch_will_fail(),
        patch_will_fail(),
        new_parts_counter(),
        new_waste_stream(),
        new_rubber_recycling(),
        new_history(),
    );
    match out {
        RepairOutcome::SpareFitted {
            member,
            workstand,
            pump,
            wheel,
            kit,
            wallet,
        } => {
            // The wheel ends at the spare's mass: 1900 + 180 = 2080.
            assert_eq!(ServiceableWheel::<2080>::VALUE, 2080);
            let _owner = send_to(new_wheel_owner(), wheel);
            let _member: Qualified<Induction, 720_000> = member;
            let kit: PatchKit<NoPatches, 28> = kit;
            // Two spent patches (3 g each) reached the waste stream inside
            // the two failed attempts (REQ-013): 6 g in all (R7).
            assert_eq!(2 * (DryPatch::MASS_G + 1), 6);
            // The wallet holds the change: 1000 − 650 = 350 p (REQ-014).
            let wallet: Wallet<350> = wallet;
            assert_eq!(Wallet::<350>::VALUE, 350);
            take_wallet_home(wallet);
            assert_history_records(
                &history,
                &[
                    ("open_wheel", 240_000),
                    ("find_hole", 180_000),
                    ("patch_tube", 120_000),
                    ("patch_tube", 120_000),
                    ("buy_spare", 120_000),
                    ("refit_and_inflate", 300_000),
                ],
            );
            let _reusables = (workstand, pump, counter, waste, recycling, kit);
            let _execution_record = history;
        }
        _ => unreachable!("fail-then-fail must take the spare path"),
    }
}

/// The ordering variant (R9, SPEC.md §6): the wallet is readied before the
/// repair starts and the dead tube is retired before the purchase — and path
/// 3 still ends in the identical state (same wheel mass, same budget, same
/// wallet, same 6 events), which is the type system proving the orderings
/// equivalent.
///
/// Verifies: REQ-010, REQ-011, REQ-012, REQ-013, REQ-014
#[test]
fn ordering_variant_path_3_ends_in_the_identical_state() {
    let (out, counter, waste, recycling, history) = repair_wheel_wallet_ready_early(
        qualify::<Induction, 1_800_000>(new_person::<1_800_000>()),
        new_workstand(),
        new_pump(),
        wheel_arrives_for_service(),
        new_patch_kit(),
        wallet_with::<1000>(),
        patch_will_fail(),
        patch_will_fail(),
        new_parts_counter(),
        new_waste_stream(),
        new_rubber_recycling(),
        new_history(),
    );
    match out {
        RepairOutcome::SpareFitted {
            member,
            workstand,
            pump,
            wheel,
            kit,
            wallet,
        } => {
            let _owner = send_to(new_wheel_owner(), wheel);
            let _member: Qualified<Induction, 720_000> = member;
            let _kit: PatchKit<NoPatches, 28> = kit;
            take_wallet_home::<350>(wallet);
            assert_history_records(
                &history,
                &[
                    ("open_wheel", 240_000),
                    ("find_hole", 180_000),
                    ("patch_tube", 120_000),
                    ("patch_tube", 120_000),
                    ("buy_spare", 120_000),
                    ("refit_and_inflate", 300_000),
                ],
            );
            let _reusables = (workstand, pump, counter, waste, recycling);
            let _execution_record = history;
        }
        _ => unreachable!("fail-then-fail must take the spare path"),
    }
}

/// The ordering variant's patched path: the early-drawn cash is deposited
/// back whole (1000 = 0 + 1000, the conserving combine), so path 1 too ends
/// in the identical state — wallet at 1000 p, untried token returned.
///
/// Verifies: REQ-010, REQ-011, REQ-012
#[test]
fn ordering_variant_path_1_redeposits_the_unspent_cash() {
    let (out, counter, waste, recycling, history) = repair_wheel_wallet_ready_early(
        qualify::<Induction, 1_800_000>(new_person::<1_800_000>()),
        new_workstand(),
        new_pump(),
        wheel_arrives_for_service(),
        new_patch_kit(),
        wallet_with::<1000>(),
        patch_will_hold(),
        patch_will_fail(), // provisioned but never tried
        new_parts_counter(),
        new_waste_stream(),
        new_rubber_recycling(),
        new_history(),
    );
    match out {
        RepairOutcome::PatchedFirstTry {
            member,
            workstand,
            pump,
            wheel,
            kit,
            wallet,
            reserve_outcome,
        } => {
            let _owner = send_to(new_wheel_owner(), wheel);
            let _member: Qualified<Induction, 900_000> = member;
            let _kit: PatchKit<OnePatch, 29> = kit;
            take_wallet_home::<1000>(wallet);
            return_patch_outcome(reserve_outcome);
            assert_eq!(history.event_count(), 5);
            let _reusables = (workstand, pump, counter, waste, recycling);
            let _execution_record = history;
        }
        _ => unreachable!("a success first trial must take the first-try path"),
    }
}

/// The downstream fixture path (R1, F-004): the crate's own `test-support`
/// feature — enabled only through the `[dev-dependencies]` self
/// re-declaration — lets this test conjure a sealed spare tube and an open
/// wheel without running the flow. The fixtures still have to be accounted
/// for like real resources: the refit consumes them under the REQ-010 and
/// REQ-012 bounds and the wheel genuinely reaches the owner, with the labour
/// drawn adjacently and recorded (R16).
///
/// Verifies: REQ-010, REQ-012
#[test]
fn fixtures_construct_sealed_resources_for_downstream_tests() {
    let spare = SpareTube::test_fixture();
    let open = OpenWheel::test_fixture();
    let member = qualify::<Induction, 400_000>(new_person::<400_000>());
    let (member, stand, pump, wheel) =
        refit_and_inflate::<2080, _, _>(member, new_workstand(), new_pump(), open, spare);
    let (labour, member) =
        model_core::common::processes::qualified_draw_time::<300_000, 100_000, 400_000, _>(member);
    let history = record(new_history(), "refit_and_inflate", labour);
    assert_eq!(history.event_count(), 1);
    let _owner = send_to(new_wheel_owner(), wheel);
    let _reusables = (member, stand, pump);
    let _execution_record = history;
}

/// The tripwire demonstration (R1 layer 2, F-008, F-032) from outside the
/// privacy boundary: a fixture spare tube that is never fitted is named and
/// used, so `must_use` and `unused_variables` are both satisfied — no
/// compile-time layer can catch this leak; the tripwire `Drop` converts it
/// into this test failure.
///
/// Verifies: REQ-012
#[test]
#[should_panic(expected = "resource leak: SpareTube dropped without being consumed")]
fn abandoned_spare_tube_trips_the_tripwire_downstream() {
    let spare = SpareTube::test_fixture();
    let _never_fitted = spare;
}
