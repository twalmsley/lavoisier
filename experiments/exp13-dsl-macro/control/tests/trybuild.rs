//! trybuild compile-fail tests (R4): type-check-time errors only —
//! conservation violations are post-monomorphization (F-001) and live as
//! rustdoc `compile_fail` doc-tests in src/ plus the feature-gated
//! `exp13-violations` transcripts.

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
