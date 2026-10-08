//! trybuild compile-fail tests (R4): type-check-time errors only. The
//! canonical requirement-bound violation is authored THROUGH the `model!`
//! flow grammar, so the pinned .stderr shows exactly what a DSL modeller
//! sees.

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
