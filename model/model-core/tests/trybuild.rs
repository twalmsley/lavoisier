//! Type-check-time compile-fail cases (R4 policy, F-003): trybuild runs
//! `cargo check` internally, so it is used ONLY for pre-monomorphization
//! errors — trait-bound violations (empty supplier, full consumer, the F-028
//! supplier-shape dead ends), unit mismatches (E0308) and privacy violations
//! (E0451). Conservation/overdraw violations are post-monomorphization
//! (F-001) and live as rustdoc `compile_fail` doc-tests in the library
//! instead; a trybuild test for them would wrongly report success.
//!
//! trybuild also inherits dev-dependency features (F-003), so these cases see
//! the `test-support` fixtures — and, for the same reason, trybuild must
//! never be used to "prove" the test-support feature boundary (only the plain
//! `cargo build` in ci.sh proves that, F-004).

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
