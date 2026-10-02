//! Type-check-time compile-fail cases (R4 policy, F-003): trybuild is used
//! ONLY for errors visible to `cargo check` — type errors, trait-bound
//! violations, privacy violations and deny-level lints. Conservation
//! violations are post-monomorphization and live as rustdoc `compile_fail`
//! doc-tests on `drill_fallible` instead.

#[test]
fn compile_fail_cases() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
