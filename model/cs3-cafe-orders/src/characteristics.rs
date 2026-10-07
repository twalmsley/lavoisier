//! Characteristic traits (R6) for the CS-3 café counter.
//!
//! A type's characteristics are the traits it implements; the R10 requirement
//! traits in [`crate::requirements`] are written as bounds over these, never
//! over concrete resource types (F-019). Each carries a single-line
//! `#[diagnostic::on_unimplemented]` phrased for modellers (F-015) —
//! single-line on purpose: the workspace `trace.sh` has no scope awareness,
//! and a multi-line attribute between a doc tag and its item would detach
//! them (F-021).
//!
//! ## Sealed characteristics with a conserving extraction
//!
//! [`ProofOfPayment`] and [`OrderSlot`] go beyond plain markers: each carries
//! the one conserving extraction [`crate::resources::processes::hand_over`]
//! needs, gated by the sealed permit [`crate::resources::HandoverPermit`]
//! (the F-054 permit-gated extraction pattern): downstream code can neither
//! call the extraction (no permit can be constructed) nor implement the trait
//! for its own types (the private supertrait below), so a forged "receipt"
//! cannot be smuggled past REQ-017's bound and nothing can be vanished
//! through a free extraction.
//!
//! ## The machine training qualification (R18)
//!
//! [`MachineTraining`] is the qualification value (model-core's open
//! `Qualification` kind trait); [`MachineTrained`] is the qualification
//! marker, attached by the one-line blanket impl over the time budget below
//! (F-043), **never to `model_core::common::Person` itself** — `Person`
//! carries no qualification slot, so a direct impl would train every person
//! in the model (the server included) at once.

use crate::resources::HandoverPermit;

/// Seals [`ProofOfPayment`] and [`OrderSlot`] (the classic sealed-trait
/// pattern, F-026): only this crate's receipt and order-slot types may carry
/// them. Without the seal, outside code could implement the trait for a type
/// of its own and smuggle an unconserved "receipt" past REQ-017's bound, or
/// vanish a drink through a free extraction.
pub(crate) mod sealed {
    /// Implemented only for this crate's receipt and order-slot types.
    pub trait Sealed {}
}

/// Qualification value (R18): trained on the café's espresso machine. A plain
/// public marker implementing model-core's open `Qualification` kind trait —
/// **not a resource**: it carries no sealed state and cannot mint anything.
/// What *grants* the training is the boundary process
/// `model_core::common::boundary::qualify` (a placeholder for the real
/// machine-training course, R12; SPEC.md §4).
pub struct MachineTraining;
impl model_core::common::Qualification for MachineTraining {}

/// Characteristic (R18): this person has been trained on the espresso
/// machine. Attached by the one-line blanket impl over the time budget below
/// (F-043), **never to `model_core::common::Person` itself** — a plain
/// `Person` carries no qualification slot, so a direct impl would train every
/// person in the model at once (the server must stay refusable at the
/// machine, REQ-015).
#[diagnostic::on_unimplemented(message = "`{Self}` has not been trained on the espresso machine (REQ-015: the espresso machine may be operated only by a trained barista)", label = "a trained barista is required here", note = "training is granted at the boundary (R12): `qualify::<MachineTraining, BUDGET>(person)` wraps a `Person` into a `Qualified` barista - a plain `Person` (the server) carries no training")]
pub trait MachineTrained {}

// The blanket-impl-over-budgets pattern (R6 bridging, F-043): one line per
// qualification value, blanket over the time budget, added in the same commit
// as the qualification value itself.
impl<const MS: u64> MachineTrained for model_core::common::Qualified<MachineTraining, MS> {}

/// Characteristic (R6): sealed proof that the order has been paid for —
/// REQ-017's subject. Implemented only by
/// [`crate::resources::PaymentReceipt`], which only
/// [`crate::resources::processes::take_payment`] can mint, so holding a value
/// with this characteristic means the till branch really ran: "hand over
/// before payment" is inexpressible. Carries the permit-gated conserving
/// extraction the join needs (F-054): the receipt is consumed exactly once,
/// at [`crate::resources::processes::hand_over`].
#[diagnostic::on_unimplemented(message = "`{Self}` is not proof of payment (REQ-017: an order may be handed over only after payment - the join requires the payment receipt from the till branch)", label = "the payment receipt from the till branch is required here", note = "the receipt is minted only by `take_payment` (branch B, the till): run the payment before the hand-over - the receipt is the one deliberate cross-branch dependency")]
pub trait ProofOfPayment: sealed::Sealed {
    /// Conserving extraction: the receipt is consumed at the join — its
    /// evidence is spent on exactly one hand-over. Callable only with a
    /// [`crate::resources::HandoverPermit`] (no public constructor), so the
    /// receipt cannot be vanished outside the join that accounts for it.
    fn surrender(self, permit: HandoverPermit);
}

/// Characteristic (R6): something that may take the flat white's place on the
/// order tray (SPEC.md §6: "P7 replaces the flat white in P6 on path 3").
/// Deliberately **two** satisfying types: the flat white itself
/// ([`crate::resources::FlatWhite`] at its 186 g build) on the served paths,
/// and the flat-white-price refund
/// (`Money<`[`crate::resources::FLAT_WHITE_PRICE_PENCE`]`>`) on path 3 —
/// the order is handed over either way, with the join (REQ-017) unchanged.
/// Carries the permit-gated conserving extraction `hand_over` needs (F-054).
#[diagnostic::on_unimplemented(message = "`{Self}` cannot take the flat white's place on the order tray: only the 186 g flat white or the 380 p refund in lieu may be handed over in that slot", label = "the flat white, or its refund in lieu, is required here", note = "the slot is filled by process (R9): `build_flat_white` produces the flat white, and `refund_flat_white` (path 3) draws the 380 p refund from the till")]
pub trait OrderSlot: sealed::Sealed {
    /// Conserving extraction: the slot's contents continue onto the customer's
    /// tray inside the served order that `hand_over` mints. Callable only
    /// with a [`crate::resources::HandoverPermit`] (no public constructor).
    fn onto_tray(self, permit: HandoverPermit);
}

/// Characteristic (R6): the boundary sink dedicated to burnt milk — REQ-016's
/// subject. Implemented only by [`crate::resources::Drain`], the counter's
/// unbounded placeholder sink (R15, F-029): `steam_milk` feeds burnt milk to
/// it **inside the process**, so burnt milk never exists loose — and the
/// burnt state has no other exit (no process accepts it), so re-steaming or
/// serving it is a type error.
#[diagnostic::on_unimplemented(message = "burnt milk may not go here: `{Self}` is not the drain (REQ-016: burnt milk must be discarded to the drain - it is never re-steamed or served)", label = "the drain is required here", note = "the dedicated sink is the `Drain` (an unbounded placeholder, R15); `steam_milk` feeds burnt milk to it inside the process, so the burnt state's only exit is the drain")]
pub trait DrainSink {}

/// Characteristic (R6): the boundary consumer the customer's change goes back
/// to — REQ-018's subject. Implemented only by
/// [`crate::resources::Customer`]: `take_payment` splits the tendered note
/// into the order price and the change and feeds the change to the customer
/// **inside the process** (REQ-018 structural — the split's second output has
/// only the customer exit).
#[diagnostic::on_unimplemented(message = "the change may not go here: `{Self}` is not the customer (REQ-018: the customer's change must be returned in full at the till)", label = "the customer is required here", note = "`take_payment` splits the tendered note into the order price and the change, deposits the price in the till, and returns the change to the customer inside the process")]
pub trait ChangeRecipient {}
