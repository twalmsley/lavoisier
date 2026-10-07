//! Characteristic traits (R6) for the CS-2 workshop bay.
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
//! [`PunctureLocated`] and [`Airtight`] go beyond plain markers: per R6, a
//! measured characteristic can also carry its quantities, so each exposes the
//! tube's mass as an associated constant — which is what lets `patch_tube`
//! and `refit_and_inflate` stay **generic over their requirement bounds**
//! (style A, F-048: REQ-phrased errors) while still stating compile-time
//! conservation over the inputs' magnitudes. Each also carries the one
//! conserving extraction its consuming process needs, gated by a sealed
//! permit ([`crate::resources::PatchPermit`] /
//! [`crate::resources::FitPermit`] — the permit-gated extraction pattern,
//! F-054): downstream code can neither call the extraction (no permit can be
//! constructed) nor implement the trait for its own types (the private
//! supertrait below), so the characteristics cannot be used to mint or vanish
//! resources — a fake "airtight tube" cannot be smuggled past REQ-012's
//! bound.
//!
//! ## The induction qualification (R18)
//!
//! [`Induction`] is the qualification value (model-core's open
//! `Qualification` kind trait); [`Inducted`] is the qualification marker,
//! attached by the one-line blanket impl over the time budget below (F-043),
//! **never to `model_core::common::Person` itself** — `Person` carries no
//! qualification slot, so a direct impl would induct every person in the
//! model at once.

use crate::resources::{FitPermit, PatchPermit};

/// Seals [`PunctureLocated`] and [`Airtight`] (the classic sealed-trait
/// pattern, F-026): only this crate's tube states may carry them. Without the
/// seal, outside code could implement the trait for a type of its own and
/// smuggle an unconserved "airtight tube" past REQ-012's bound, or vanish a
/// tube through a free extraction.
pub(crate) mod sealed {
    /// Implemented only for this crate's patch-ready and wheel-ready tube
    /// states.
    pub trait Sealed {}
}

/// Qualification value (R18): inducted into the community workshop. A plain
/// public marker implementing model-core's open `Qualification` kind trait —
/// **not a resource**: it carries no sealed state and cannot mint anything.
/// What *grants* the induction is the boundary process
/// `model_core::common::boundary::qualify` (a placeholder for the real
/// induction session, R12; SPEC.md §4).
pub struct Induction;
impl model_core::common::Qualification for Induction {}

/// Characteristic (R18): this person has been inducted into the workshop.
/// Attached by the one-line blanket impl over the time budget below (F-043),
/// **never to `model_core::common::Person` itself** — a plain `Person`
/// carries no qualification slot, so a direct impl would induct every person
/// in the model at once.
#[diagnostic::on_unimplemented(message = "`{Self}` has not been inducted into the workshop (REQ-010: workshop tools may be used only by an inducted member)", label = "an inducted workshop member is required here", note = "induction is granted at the boundary (R12): `qualify::<Induction, BUDGET>(person)` wraps a `Person` into a `Qualified` member - a plain `Person` carries no induction")]
pub trait Inducted {}

// The blanket-impl-over-budgets pattern (R6 bridging, F-043): one line per
// qualification value, blanket over the time budget, added in the same commit
// as the qualification value itself.
impl<const MS: u64> Inducted for model_core::common::Qualified<Induction, MS> {}

/// Characteristic (R6): the tube's puncture has been **located** —
/// implemented only by the located-puncture state (one type per state,
/// R9/F-023), which is what lets REQ-011 refuse blind patching at compile
/// time. Carries the tube's mass (R6/R7) and the permit-gated conserving
/// extraction `patch_tube`'s success arm needs (F-054).
#[diagnostic::on_unimplemented(message = "`{Self}` is not a tube with a located puncture (REQ-011: only a located puncture may be patched)", label = "a tube with its puncture located is required here", note = "locating is a process (one type per state, R9): `find_hole` turns a `PuncturedTube` into a `LocatedTube` - no blind patching")]
pub trait PunctureLocated: sealed::Sealed {
    /// The tube's mass, in grams (R7).
    const MASS_G: u64;
    /// Conserving extraction: the located tube is consumed and its mass
    /// continues into the patched tube that `patch_tube` mints — that
    /// process's success-arm conservation assert checks exactly this, which
    /// is why the extraction is callable only with a
    /// [`crate::resources::PatchPermit`] (no public constructor): a
    /// free-standing call would vanish the tube silently.
    fn into_patch_site(self, permit: PatchPermit);
}

/// Characteristic (R6): the tube is **airtight** and may be fitted to a wheel
/// (REQ-012's subject). Deliberately **multi-type** (SPEC.md §8, review
/// decision 7): the checked-patched tube *and* the new spare both carry it —
/// the first requirement in the workspace satisfied by two types. Carries the
/// tube's mass (R6/R7; 183 g patched, 180 g spare — the paths end at
/// different wheel masses) and the permit-gated conserving extraction
/// `refit_and_inflate` needs (F-054).
#[diagnostic::on_unimplemented(message = "`{Self}` is not an airtight tube (REQ-012: a wheel may be refitted only with an airtight tube - patched and checked, or new)", label = "an airtight tube is required here", note = "airtight states are reached by process (R9): `check_patch` turns a `PatchedTube` into a `CheckedTube`, and `buy_spare` supplies a new `SpareTube`")]
pub trait Airtight: sealed::Sealed {
    /// The tube's mass, in grams (R7).
    const MASS_G: u64;
    /// Conserving extraction: the airtight tube is consumed and its mass
    /// continues into the serviceable wheel that `refit_and_inflate` mints —
    /// that process's per-instantiation mass assert checks exactly this.
    /// Callable only with a [`crate::resources::FitPermit`] (no public
    /// constructor).
    fn fit_into_wheel(self, permit: FitPermit);
}

/// Characteristic (R6): the boundary consumer dedicated to failed patches
/// (REQ-013's subject). Implemented only by
/// [`crate::resources::WasteStream`], the workshop's unbounded placeholder
/// sink (R15, F-029) — deliberately the *other* sink shape from CS-1's finite
/// bin (SPEC.md §4).
#[diagnostic::on_unimplemented(message = "failed patches may not go here: `{Self}` is not the workshop waste stream (REQ-013: all failed patches must reach it)", label = "the workshop waste stream is required here", note = "the dedicated consumer is the `WasteStream` (an unbounded placeholder sink, R15): `patch_tube` feeds the spent patch to it inside the process, so failed patches never exist loose")]
pub trait WasteStreamConsumer {}

/// Characteristic (R6): a boundary vendor that sells **only at its listed
/// price** (REQ-014's subject). Implemented only by
/// [`crate::resources::PartsCounter`], whose `Consumer` impl exists only at
/// `Money<650>` (the F-051 exact-price pattern), so a wrong payment is a
/// type-check-time error naming the right price.
#[diagnostic::on_unimplemented(message = "`{Self}` is not the parts counter selling at its listed price (REQ-014: spare tubes are sold only at the counter's listed price)", label = "the exact-price parts counter is required here", note = "the counter consumes only `Money<650>` (the listed price, F-051); `buy_spare` splits the tendered cash into price and change")]
pub trait ExactPriceCounter {}
