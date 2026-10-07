//! Characteristic traits (R6) for the CS-5 logistics subsystem.
//!
//! [`ContractedCourier`] is REQ-027's subject: **sealed** (F-026), so no
//! other crate can declare its own van contracted and smuggle goods past the
//! requirement — and it carries the one permit-gated conserving conversion
//! its consuming process needs (the F-054 pattern): `consign` opens the
//! contracted courier's home state through [`ContractedCourier::stand_down`]
//! (callable only with a [`crate::resources::DepartPermit`], which has no
//! public constructor) and mints the outbound state in its place, so the
//! characteristic can never be used to vanish a courier outside the process
//! that accounts for it. The attribute is single-line on purpose (F-021).

use crate::resources::DepartPermit;

/// Seals [`ContractedCourier`] (the classic sealed-trait pattern, F-026):
/// implemented only for this crate's courier.
pub(crate) mod sealed {
    /// Implemented only for the contracted courier's home state.
    pub trait Sealed {}
}

/// Characteristic (R6): the organisation holding the works' transport
/// contract, at its home site and ready to be consigned (REQ-027's subject).
/// Implemented only by [`crate::resources::CourierAtUk`] — one courier, one
/// contract, one loop.
#[diagnostic::on_unimplemented(message = "`{Self}` is not the contracted courier (REQ-027: all inter-site transport must be by the contracted courier)", label = "the contracted courier, at its home site, is required here", note = "the contracted courier enters at the boundary (`courier_reports_for_duty`) and is the only type carrying this sealed characteristic - an outside organisation cannot be declared contracted (F-026)")]
pub trait ContractedCourier: sealed::Sealed {
    /// Conserving departure: the home state is consumed and the courier
    /// continues as the outbound state `consign` mints — callable only with
    /// a [`DepartPermit`] (no public constructor, F-054), so the
    /// characteristic cannot vanish a courier outside `consign`.
    fn stand_down(self, permit: DepartPermit);
}
