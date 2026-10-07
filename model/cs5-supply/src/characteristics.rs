//! Characteristic traits (R6) for the CS-5 supply subsystem.
//!
//! A type's characteristics are the traits it implements; the R10
//! requirement traits (this crate's [`crate::requirements`], and
//! `cs5-works`' REQ-025) are written as bounds over these, never over
//! concrete resource types (F-019). Each carries a single-line
//! `#[diagnostic::on_unimplemented]` phrased for modellers (F-015) —
//! single-line on purpose: the workspace `trace.sh` has no scope awareness,
//! and a multi-line attribute between a doc tag and its item would detach
//! them (F-021).
//!
//! All three characteristics are **sealed** (F-026): only this crate's types
//! may carry them, so a downstream crate can neither smuggle a fake
//! euro-cent vendor past REQ-023, a fake exchange desk past REQ-024, nor an
//! uninspected part past the works' REQ-025 bound. [`GoodsInInspected`] also
//! carries the component's mass (R6/R7, the F-054 associated-const pattern):
//! that is what lets the works' `assemble` stay generic over the requirement
//! bound (style A, REQ-phrased errors) while still stating compile-time mass
//! conservation. No extraction method is needed on it — the inspected
//! component is **kept** by the instrument that fits it (the F-040 kept-item
//! shape), not vanished.

/// Seals the supply characteristics (the classic sealed-trait pattern,
/// F-026): implemented only for this crate's types.
pub(crate) mod sealed {
    /// Implemented only for this crate's vendor, bureau and inspected
    /// component.
    pub trait Sealed {}
}

/// Characteristic (R6): a vendor organisation that sells **only against euro
/// cents at its exact price** (REQ-023's subject). Implemented only by
/// [`crate::resources::Vendor`], whose `Consumer` impl exists only at
/// `Euros<2340>` (the F-051 exact-price pattern), so paying in the wrong
/// currency or the wrong amount is a type-check-time error.
#[diagnostic::on_unimplemented(message = "`{Self}` is not the EU vendor selling in its own currency (REQ-023: foreign purchases must be paid in the supplier's currency)", label = "the euro-cent-priced vendor is required here", note = "the vendor consumes only `Euros<2340>` (the exact price, F-051): euro cents exist only via the bureau exchange (REQ-024), never from GBP arithmetic")]
pub trait PricedInEuroCents: sealed::Sealed {}

/// Characteristic (R6): the one place currency may be exchanged (REQ-024's
/// subject). Implemented only by [`crate::resources::Bureau`] — the
/// deliberately **unrefined** placeholder boundary object (the contrast to
/// the refined vendor, SPEC §4).
#[diagnostic::on_unimplemented(message = "`{Self}` is not the bureau (REQ-024: currency may be exchanged only at the bureau, at its stated rate)", label = "the bureau is required here", note = "exchange is a boundary process (R19/F-052): it destroys an amount in one currency dimension and mints the equivalent in the other, which only the bureau placeholder may do")]
pub trait ExchangeDesk: sealed::Sealed {}

/// Characteristic (R6): a precision component that has passed **goods-in
/// inspection at the UK site** (the subject of the works' REQ-025).
/// Implemented only by [`crate::resources::InspectedComponent`] — the state
/// produced by [`crate::resources::processes::open_and_inspect`] (one type
/// per processing state, R9/F-023), so a still-boxed component cannot be
/// fitted. Carries the component's mass (R6/R7): the works' `assemble`
/// asserts `600 + 400 = 1000` over this constant while staying generic over
/// the requirement bound (F-054). No extraction is needed: the inspected
/// component is kept inside the instrument that fits it (F-040).
#[diagnostic::on_unimplemented(message = "`{Self}` has not passed goods-in inspection (REQ-025: a component may be fitted only after goods-in inspection at the UK site)", label = "an inspected component is required here", note = "inspection is a process (one type per state, R9): `open_and_inspect` turns the 450 g `BoxedComponent` into the 400 g `InspectedComponent` and routes the 50 g packaging to the bin")]
pub trait GoodsInInspected: sealed::Sealed {
    /// The inspected component's mass, in grams (R7).
    const MASS_G: u64;
}
