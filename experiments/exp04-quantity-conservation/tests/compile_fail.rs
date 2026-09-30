/// Verifies: R3, R4 — conservation violations and unit mixing must not compile.
#[test]
fn compile_fail_cases() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
