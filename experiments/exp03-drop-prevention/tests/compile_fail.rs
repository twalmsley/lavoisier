//! trybuild harness: the evidence for every "caught at COMPILE time" cell
//! that rustc itself (not clippy) can produce. Clippy-caught cells are
//! demonstrated in `clippy-demos/` because trybuild drives rustc, not clippy.

#[test]
fn compile_fail() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
}
