//! trybuild compile-fail tests (R4): model mismatches must not compile.

#[test]
fn compile_fail() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
