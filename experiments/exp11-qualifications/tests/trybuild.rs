//! Type-check-time compile-fail cases (R4 policy, F-003): trybuild runs
//! `cargo check` internally, so it is used ONLY for pre-monomorphization
//! errors — requirement-bound violations (E0277: uncertified person, wrong
//! certification, unfitted guard), a missing input (E0061), and a stale
//! budget state (E0308). Overdraw through the wrapper is
//! post-monomorphization (F-001) and lives as a rustdoc `compile_fail`
//! doc-test on `operator_draw_time` instead.

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
