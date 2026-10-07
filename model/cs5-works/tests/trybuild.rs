//! Type-check-time compile-fail cases (R4 policy, F-003): trybuild runs
//! `cargo check` internally, so it is used ONLY for pre-monomorphization
//! errors — the cross-currency payment (E0308: `Euros<2340>` vs
//! `Money<2340>`, REQ-023's structural half), the crate-edge mint attempt
//! (E0624: euro cents cannot be minted works-side), the un-contracted
//! transport (E0277 with REQ-027's own message, F-044), the uninspected
//! component (E0277, REQ-025), the order-less delivery (E0277, REQ-026),
//! and the fourth purchase from the refined vendor's 3-stock (E0277 with
//! the F-015 "it is exhausted" message). Conservation violations — the
//! exchange/inspection/deposit asserts and the budget overdraw — are
//! post-monomorphization (F-001) and live as rustdoc `compile_fail`
//! doc-tests in `cs5-supply` and this crate instead; a trybuild case for
//! them would wrongly report success.
//!
//! trybuild also inherits dev-dependency features (F-003), so these cases
//! see the test-support fixtures — never use trybuild to "prove" the
//! test-support feature boundary (only the plain `cargo build` in ci.sh
//! proves that, F-004).

/// Verifies: REQ-023, REQ-025, REQ-026, REQ-027
#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
