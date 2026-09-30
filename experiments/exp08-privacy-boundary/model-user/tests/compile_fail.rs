//! trybuild compile-fail tests (R4): each `tests/ui/*.rs` file must fail to
//! compile with the error recorded in the matching `.stderr` file.

#[test]
fn boundary_violations_do_not_compile() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
