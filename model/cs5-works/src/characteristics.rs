//! Characteristic traits (R6) for the CS-5 works subsystem.
//!
//! [`ProofOfOrder`] is REQ-026's subject: **sealed** (F-026), so nothing —
//! not even the supply crate's own paperwork (the vendor-facing
//! `PurchaseOrder`) — can stand in for the customer's order at delivery, and
//! it carries the one permit-gated conserving consumption its process needs
//! (the F-054 pattern): `deliver` closes the order through
//! [`ProofOfOrder::fulfil`], callable only with a
//! [`crate::resources::FulfilPermit`] (no public constructor), so the
//! characteristic can never be used to vanish an order outside the delivery
//! that accounts for it. The attribute is single-line on purpose (F-021).
//!
//! The works' other requirement, REQ-025, is bounded over `cs5-supply`'s
//! sealed `GoodsInInspected` characteristic — the inspected state is a
//! supply-sealed type, so its characteristic lives with its seal (F-026) and
//! the requirement trait lives with its owner (SPEC §3).

use crate::resources::FulfilPermit;

/// Seals [`ProofOfOrder`] (the classic sealed-trait pattern, F-026): only
/// this crate's customer order carries it.
pub(crate) mod sealed {
    /// Implemented only for the customer's order token.
    pub trait Sealed {}
}

/// Characteristic (R6): the customer's placed order — the evidence token
/// REQ-026 turns on (SPEC §3: "sealed; REQ-026's key"). Implemented only by
/// [`crate::resources::CustomerOrder`] (one type per state, R9: the placed
/// order; fulfilment is its consumption at delivery).
#[diagnostic::on_unimplemented(message = "`{Self}` is not the customer's order (REQ-026: delivery happens only against the customer's order, with payment taken on delivery)", label = "the customer's order token is required here", note = "the order is placed at the boundary (`place_order`) and consumed exactly once, by `deliver` - the vendor-facing `PurchaseOrder` is different paperwork and does not satisfy this requirement")]
pub trait ProofOfOrder: sealed::Sealed {
    /// Conserving fulfilment: the placed order is consumed and its
    /// fulfilment continues as the delivery `deliver` completes — callable
    /// only with a [`FulfilPermit`] (no public constructor, F-054), so the
    /// characteristic cannot vanish an order outside the delivery that
    /// accounts for it.
    fn fulfil(self, permit: FulfilPermit);
}
