//! GENERATED requirement traits (R10) from SPEC.md §2, via
//! `model_core::requirement!` (blanket impl + per-requirement assert fn,
//! with a REQ-phrased `#[diagnostic::on_unimplemented]`, R10 rule 8).

use crate::characteristics::{Req006Subject, Req007Subject, Req008Subject, Req009Subject};

// ── SPEC-HOLE U-06 ──────────────────────────────────────────
// The spec says which PROCESSES satisfy which requirements (§5 Satisfies lines,
// emitted as doc tags on the process fns), but not which TYPES do, nor the
// alias-carries-the-tag placement the real crate uses to dodge F-037. The
// `satisfies!` compile-checked assertions therefore cannot be generated.
#[cfg(feature = "deny-holes")]
compile_error!("SPEC-HOLE U-06: `Satisfies:` doc tags and their backing `satisfies!` assertions cannot be placed");

model_core::requirement! {
    /// REQ-006: Tea must be brewed with boiling water (only the boiling state of the kettle can be poured into the pot).
    /// (SPEC.md §2 line 26 numbers this requirement REQ-001; ids remapped per the §2 implementation note, F-053.)
    #[diagnostic::on_unimplemented(message = "`{Self}` does not satisfy REQ-006: Tea must be brewed with boiling water .", label = "REQ-006: Tea must be brewed with boiling water .")]
    pub trait Req006TeaBrewedBoilingWater: (Req006Subject);
    assert = assert_req006;
}

model_core::requirement! {
    /// REQ-007: The pot must be loaded with exactly 3 teabags before brewing.
    /// (SPEC.md §2 line 28 numbers this requirement REQ-002; ids remapped per the §2 implementation note, F-053.)
    #[diagnostic::on_unimplemented(message = "`{Self}` does not satisfy REQ-007: The pot must be loaded with exactly 3 teabags before brewing.", label = "REQ-007: The pot must be loaded with exactly 3 teabags before brewing.")]
    pub trait Req007PotLoadedTeabagBrewing: (Req007Subject);
    assert = assert_req007;
}

model_core::requirement! {
    /// REQ-008: All spent teabags must reach the food-waste bin.
    /// (SPEC.md §2 line 29 numbers this requirement REQ-003; ids remapped per the §2 implementation note, F-053.)
    #[diagnostic::on_unimplemented(message = "`{Self}` does not satisfy REQ-008: All spent teabags must reach the food-waste bin.", label = "REQ-008: All spent teabags must reach the food-waste bin.")]
    pub trait Req008SpentTeabagReachFood: (Req008Subject);
    assert = assert_req008;
}

model_core::requirement! {
    /// REQ-009: All waste heat must be accounted to the kitchen-air sink.
    /// (SPEC.md §2 line 30 numbers this requirement REQ-004; ids remapped per the §2 implementation note, F-053.)
    #[diagnostic::on_unimplemented(message = "`{Self}` does not satisfy REQ-009: All waste heat must be accounted to the kitchen-air sink.", label = "REQ-009: All waste heat must be accounted to the kitchen-air sink.")]
    pub trait Req009WasteHeatAccountedKitchen: (Req009Subject);
    assert = assert_req009;
}

