//! Type-check-time compile-fail cases (R4 policy, F-003): trybuild runs
//! `cargo check` internally, so it is used ONLY for pre-monomorphization
//! errors — **the crate edge itself** (E0451/E0624: the line trying to mint
//! stores materials, which is REQ-019's structural half, EXP-08/F-005), a
//! note-less fasten (E0277 with REQ-021's own on_unimplemented message, R10
//! rule 8/F-044), a fasten against a box that runs short (E0277, the
//! `SupplyN` message, F-014/F-015), and an 81st piece into the full bin
//! (E0277, the `Consumer` message, F-015/F-034). Conservation and
//! reconciliation-total violations are post-monomorphization (F-001) and
//! live as rustdoc `compile_fail` doc-tests in `cs4-stores` instead; a
//! trybuild case for them would wrongly report success.
//!
//! trybuild also inherits dev-dependency features (F-003), so these cases
//! see the test-support fixtures — never use trybuild to "prove" the
//! test-support feature boundary (only the plain `cargo build` in ci.sh
//! proves that, F-004).

/// Verifies: REQ-019, REQ-021
#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
