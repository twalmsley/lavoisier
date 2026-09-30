//! Type-check-time compile-fail cases (R4 policy: trybuild only for
//! pre-monomorphization errors; the conservation cases are rustdoc
//! `compile_fail` doc-tests in src/lib.rs, because trybuild runs
//! `cargo check` and cannot see post-mono errors, F-003).

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
