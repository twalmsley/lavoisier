//! Characteristic traits (R6) for the CS-6 kitchen.
//!
//! Started from the specgen scaffold of SPEC.md (R22) and hand-maintained
//! since: the scaffold emitted one synthesized marker per §2 requirement and
//! flagged each as a SPEC-HOLE (U-02..U-05) because the spec does not say
//! which resource state carries which characteristic; the resolutions below
//! follow the CS-2 pattern.
//!
//! A type's characteristics are the traits it implements; the R10 requirement
//! traits in [`crate::requirements`] are written as bounds over these, never
//! over concrete resource types (F-019). Each carries a single-line
//! `#[diagnostic::on_unimplemented]` phrased for modellers (F-015) —
//! single-line on purpose: the workspace `trace.sh` has no scope awareness,
//! and a multi-line attribute between a doc tag and its item would detach
//! them (F-021).
//!
//! ## Measured characteristics with a conserving extraction
//!
//! [`Proved`] and [`Greased`] go beyond plain markers: per R6, a measured
//! characteristic can also carry its quantities, so each exposes its mass as
//! an associated constant — which is what lets `divide_and_shape` and `bake`
//! stay **generic over their requirement bounds** (style A, F-048:
//! REQ-phrased errors) while still stating compile-time conservation over the
//! inputs' magnitudes. Each also carries the one conserving extraction its
//! consuming process needs, gated by a sealed permit
//! ([`crate::resources::ShapePermit`] / [`crate::resources::BakePermit`] —
//! the permit-gated extraction pattern, F-054): downstream code can neither
//! call the extraction (no permit can be constructed) nor implement the trait
//! for its own types (the sealed supertrait below), so the characteristics
//! cannot be used to mint or vanish resources — a fake "proved dough" cannot
//! be smuggled past REQ-028's bound.
//!
//! ## The boundary-sink markers
//!
//! [`BakedOnlyHousehold`] and [`ScorchedLoafCompost`] are the REQ-030 and
//! REQ-031 subjects: plain markers on the two §4 sinks, bound on the flow's
//! consumer parameters so a transposed sink fails with the REQ-phrased
//! message (F-044). The requirements themselves are **structural** too:
//! [`crate::resources::Household`] has a `Consumer` impl only for the baked
//! state, and the scorched state's only `Consumer` in the workspace is
//! [`crate::resources::CompostStream`].

use crate::resources::{BakePermit, ShapePermit};

/// Seals [`Proved`] and [`Greased`] (the classic sealed-trait pattern,
/// F-026): only this crate's dough and tin states may carry them. Without the
/// seal, outside code could implement the trait for a type of its own and
/// smuggle an unconserved "proved dough" past REQ-028's bound, or vanish a
/// tin through a free extraction.
pub(crate) mod sealed {
    /// Implemented only for this crate's shape-ready and bake-ready states.
    pub trait Sealed {}
}

/// Characteristic (R6): the dough is **proved** — implemented only by the
/// proved state (one type per state, R9/F-023), which is what lets REQ-028
/// refuse shaping unproved dough at compile time. Carries the dough's mass
/// (R6/R7) and the permit-gated conserving extraction `divide_and_shape`
/// needs (F-054).
#[diagnostic::on_unimplemented(message = "`{Self}` is not proved dough (REQ-028: dough may be divided and shaped only once it is proved)", label = "proved dough is required here", note = "proving is a process (one type per state, R9): `prove` turns a `KneadedDough` into a `ProvedDough` - only that state can be shaped")]
pub trait Proved: sealed::Sealed {
    /// The dough's mass, in grams (R7).
    const MASS_G: u64;
    /// Conserving extraction: the proved dough is consumed and its mass
    /// continues into the two shaped loaves that `divide_and_shape` mints —
    /// that process's conservation assert checks exactly this, which is why
    /// the extraction is callable only with a
    /// [`crate::resources::ShapePermit`] (no public constructor): a
    /// free-standing call would vanish the dough silently.
    fn into_loaves(self, permit: ShapePermit);
}

/// Characteristic (R6): the tin is **greased** and may seat a loaf in the
/// oven (REQ-029's subject). Implemented only by the greased state (R9):
/// greasing is a process, so a clean or used tin cannot reach the oven.
/// Carries the tin's mass (R6/R7; 456 g — 450 g tin + 6 g butter) and the
/// permit-gated conserving extraction `bake` needs (F-054).
#[diagnostic::on_unimplemented(message = "`{Self}` is not a greased tin (REQ-029: the oven may bake only a loaf seated in a greased tin)", label = "a greased tin is required here", note = "greasing is a process (one type per state, R9): `grease_tins` turns two `CleanTin`s into `GreasedTin`s - only that state can go in the oven")]
pub trait Greased: sealed::Sealed {
    /// The tin's mass including its grease, in grams (R7).
    const MASS_G: u64;
    /// Conserving extraction: the greased tin is consumed and its mass
    /// continues into the baked (or scorched) loaf's crust and the used tin
    /// that `bake` mints — that process's per-arm mass asserts check exactly
    /// this. Callable only with a [`crate::resources::BakePermit`] (no
    /// public constructor).
    fn into_oven(self, permit: BakePermit);
}

/// Characteristic (R6): the boundary consumer that accepts **only `baked`
/// loaves** (REQ-030's subject). Implemented only by
/// [`crate::resources::Household`], the unbounded placeholder sink (R15,
/// F-029) whose `Consumer` impl exists for the baked state alone — handing it
/// a scorched loaf is a type-check-time error (SPEC.md §4).
#[diagnostic::on_unimplemented(message = "baked loaves may not go here: `{Self}` is not the household (REQ-030: the household accepts only baked loaves)", label = "the household is required here", note = "the household is the boundary sink for `BakedLoaf` only (R15): a scorched loaf has no consumer there - its only exit is the compost stream (REQ-031)")]
pub trait BakedOnlyHousehold {}

/// Characteristic (R6): the boundary consumer dedicated to scorched loaves
/// (REQ-031's subject). Implemented only by
/// [`crate::resources::CompostStream`], the unbounded placeholder sink (R15,
/// F-029) that is the **only** `Consumer` of the scorched state in the
/// workspace — so every scorched loaf's sole exit is the compost stream, and
/// the tripwire catches one that never gets there (SPEC.md §4).
#[diagnostic::on_unimplemented(message = "scorched loaves may not go here: `{Self}` is not the compost stream (REQ-031: every scorched loaf must reach the compost stream)", label = "the compost stream is required here", note = "the compost stream is the only consumer of `ScorchedLoaf` (R15): a scorched bake is final (no rework, SPEC.md §5 P6) and composting is its one exit")]
pub trait ScorchedLoafCompost {}
