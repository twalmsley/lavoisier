//! Type-check-time compile-fail cases (R4 policy, F-003): trybuild runs
//! `cargo check` internally, so it is used ONLY for pre-monomorphization
//! errors — requirement-bound violations (E0277: the untrained server at the
//! machine vs REQ-015, burnt milk anywhere but the drain vs REQ-016, the
//! hand-over without the receipt vs REQ-017), wrong states in process
//! positions (E0308: serving burnt milk; the wrong tender at the till — the
//! exact-amount pattern, F-051), the no-`Debug` unwrap on the steam result
//! (E0277, F-047), and **the contention counter-example** (E0382, F-025: the
//! server pulled into a branch-A step while branch B holds them — the one
//! "interleaving" R2 refuses). Conservation and overdraw violations are
//! post-monomorphization (F-001) and live as rustdoc `compile_fail`
//! doc-tests on `pull_espresso`, `steam_milk`, `take_payment`,
//! `refund_flat_white` and the crate docs instead; a trybuild case for them
//! would wrongly report success.
//!
//! trybuild also inherits dev-dependency features (F-003), so these cases see
//! the test-support fixtures — never use trybuild to "prove" the test-support
//! feature boundary (only the plain `cargo build` in ci.sh proves that,
//! F-004).

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
