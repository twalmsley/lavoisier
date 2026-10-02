//! Type-check-time compile-fail cases (R4 policy, F-003): trybuild runs
//! `cargo check` internally, so it is used ONLY for pre-monomorphization
//! errors — exclusivity violations (E0382: person used twice, drill
//! swallowed), wrong processing state (E0308: product never produced),
//! requirement-bound violations (E0277: wrong bolt vs REQ-001) and privacy
//! violations (E0451: outside-boundary construction). Conservation and
//! overdraw violations are post-monomorphization (F-001) and live as rustdoc
//! `compile_fail` doc-tests on `cut` and `drill_holes` instead; a trybuild
//! case for them would wrongly report success.
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
