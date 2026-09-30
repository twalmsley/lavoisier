/// Verifies: REQ-001, REQ-002 (R4 — a wrong bolt is a compile error in both
/// encodings; the .stderr files are the evidence quoted in RESULTS.md).
#[test]
fn compile_fail_cases() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
}
