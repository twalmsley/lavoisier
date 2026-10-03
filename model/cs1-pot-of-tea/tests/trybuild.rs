//! Type-check-time compile-fail cases (R4 policy, F-003): trybuild runs
//! `cargo check` internally, so it is used ONLY for pre-monomorphization
//! errors — requirement-bound violations (E0277: a non-boiling kettle at the
//! pour vs REQ-006, an unloaded pot at the brew vs REQ-007, spent bags
//! offered anywhere but the bin vs REQ-008) and privacy violations (E0451:
//! outside-boundary construction). Conservation and overdraw violations are
//! post-monomorphization (F-001) and live as rustdoc `compile_fail`
//! doc-tests on `boil`, `pour_and_brew` and the crate docs instead; a
//! trybuild case for them would wrongly report success.
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
