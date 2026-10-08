//! Type-check-time compile-fail cases (R4 policy, F-003): trybuild runs
//! `cargo check` internally, so it is used ONLY for pre-monomorphization
//! errors — requirement-bound violations (E0277: the unproved dough vs
//! REQ-028, the ungreased tin vs REQ-029), the single-oven contention
//! (E0382: starting the second bake before the first returns the oven),
//! the scorched loaf at the household (E0277 on the `Consumer` bound,
//! REQ-030 structural) and the no-`Debug` unwrap on the bake result (E0277,
//! F-047). Conservation and overdraw violations are post-monomorphization
//! (F-001) and live as rustdoc `compile_fail` doc-tests on the crate docs
//! instead; a trybuild case for them would wrongly report success.
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
