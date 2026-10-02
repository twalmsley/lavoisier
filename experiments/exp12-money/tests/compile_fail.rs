//! Type-check-time compile-fail cases (R4 policy: trybuild only for
//! pre-monomorphization errors — unit mismatches and trait-bound violations.
//! The conservation cases — overspending, wrong change, inexact exchange —
//! are post-monomorphization E0080s invisible to trybuild (F-001, F-003) and
//! live as rustdoc `compile_fail` doc-tests in src/lib.rs).

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
