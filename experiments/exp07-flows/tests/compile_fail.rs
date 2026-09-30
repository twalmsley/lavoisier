//! trybuild harness for the R2/R9 compile-fail demonstrations (R4).

#[test]
fn compile_fail() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
}
