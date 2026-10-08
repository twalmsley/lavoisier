//! GENERATED characteristic markers (R6) — one synthesized marker per §2
//! requirement. The spec names requirements but not the characteristic
//! machinery behind them, so each marker is a hole.

/// Synthesized characteristic for REQ-006 (SPEC.md §2, line 26).
// ── SPEC-HOLE U-02 ──────────────────────────────────────────
// REQ-006 ("Tea must be brewed with boiling water (only the boiling state of the kettle can be poured into the pot).", SPEC.md §2 line 26) becomes a requirement trait, but the
// spec does not state which type implements its characteristic, what quantities
// the characteristic carries (the real crate's `Boiling` carries WATER_G and
// EMBODIED_J for the conservation asserts), or the sealed/permit-gated
// extraction design. Attach `Req006Subject` to the right state type by hand.
#[cfg(feature = "deny-holes")]
compile_error!("SPEC-HOLE U-02: no §3 column says which resource state carries the REQ-006 characteristic");
#[diagnostic::on_unimplemented(message = "`{Self}` does not satisfy REQ-006: Tea must be brewed with boiling water .")]
pub trait Req006Subject {}

/// Synthesized characteristic for REQ-007 (SPEC.md §2, line 28).
// ── SPEC-HOLE U-03 ──────────────────────────────────────────
// REQ-007 ("The pot must be loaded with exactly 3 teabags before brewing.", SPEC.md §2 line 28) becomes a requirement trait, but the
// spec does not state which type implements its characteristic, what quantities
// the characteristic carries (the real crate's `Boiling` carries WATER_G and
// EMBODIED_J for the conservation asserts), or the sealed/permit-gated
// extraction design. Attach `Req007Subject` to the right state type by hand.
#[cfg(feature = "deny-holes")]
compile_error!("SPEC-HOLE U-03: no §3 column says which resource state carries the REQ-007 characteristic");
#[diagnostic::on_unimplemented(message = "`{Self}` does not satisfy REQ-007: The pot must be loaded with exactly 3 teabags before brewing.")]
pub trait Req007Subject {}

/// Synthesized characteristic for REQ-008 (SPEC.md §2, line 29).
// ── SPEC-HOLE U-04 ──────────────────────────────────────────
// REQ-008 ("All spent teabags must reach the food-waste bin.", SPEC.md §2 line 29) becomes a requirement trait, but the
// spec does not state which type implements its characteristic, what quantities
// the characteristic carries (the real crate's `Boiling` carries WATER_G and
// EMBODIED_J for the conservation asserts), or the sealed/permit-gated
// extraction design. Attach `Req008Subject` to the right state type by hand.
#[cfg(feature = "deny-holes")]
compile_error!("SPEC-HOLE U-04: no §3 column says which resource state carries the REQ-008 characteristic");
#[diagnostic::on_unimplemented(message = "`{Self}` does not satisfy REQ-008: All spent teabags must reach the food-waste bin.")]
pub trait Req008Subject {}

/// Synthesized characteristic for REQ-009 (SPEC.md §2, line 30).
// ── SPEC-HOLE U-05 ──────────────────────────────────────────
// REQ-009 ("All waste heat must be accounted to the kitchen-air sink.", SPEC.md §2 line 30) becomes a requirement trait, but the
// spec does not state which type implements its characteristic, what quantities
// the characteristic carries (the real crate's `Boiling` carries WATER_G and
// EMBODIED_J for the conservation asserts), or the sealed/permit-gated
// extraction design. Attach `Req009Subject` to the right state type by hand.
#[cfg(feature = "deny-holes")]
compile_error!("SPEC-HOLE U-05: no §3 column says which resource state carries the REQ-009 characteristic");
#[diagnostic::on_unimplemented(message = "`{Self}` does not satisfy REQ-009: All waste heat must be accounted to the kitchen-air sink.")]
pub trait Req009Subject {}

