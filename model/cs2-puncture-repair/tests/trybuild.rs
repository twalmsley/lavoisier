//! Type-check-time compile-fail cases (R4 policy, F-003): trybuild runs
//! `cargo check` internally, so it is used ONLY for pre-monomorphization
//! errors — requirement-bound violations (E0277: the un-inducted member at
//! the workstand vs REQ-010, the unlocated tube vs REQ-011, the unchecked
//! tube vs REQ-012), the exact-price mismatch (E0308: paying the counter the
//! wrong amount, F-051) and the no-`Debug` unwrap on the patch result
//! (E0277, F-047). Conservation and overdraw violations are
//! post-monomorphization (F-001) and live as rustdoc `compile_fail`
//! doc-tests on `patch_tube`, `buy_spare`, `draw_cement` and the crate docs
//! instead; a trybuild case for them would wrongly report success.
//!
//! trybuild also inherits dev-dependency features (F-003), so these cases see
//! the test-support fixtures — never use trybuild to "prove" the test-support
//! feature boundary (only the plain `cargo build` in ci.sh proves that,
//! F-004).

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
