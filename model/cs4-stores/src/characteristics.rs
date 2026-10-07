//! Characteristic traits (R6) for the CS-4 stores subsystem.
//!
//! A type's characteristics are the traits it implements; the R10 requirement
//! traits in [`crate::requirements`] are written as bounds over these, never
//! over concrete resource types (F-019). Each marker carries a single-line
//! `#[diagnostic::on_unimplemented]` phrased for modellers (F-015) —
//! single-line on purpose: the workspace `trace.sh` has no scope awareness
//! (F-021).
//!
//! ## The bolt catalogue's kinds and values (R6 fixed roles, F-018)
//!
//! The catalogue is the one parameterized type
//! [`crate::resources::Bolt`]`<Size, Material, Length>`; every parameter slot
//! is bounded by a kind trait below so a transposed argument is a clear
//! construction-site error, not silent model corruption. The value set
//! deliberately includes **two lengths** — [`L15`] (the issued bolt) and
//! [`L18`] (catalogued but unused) — so SPEC §7's probe A ("change the
//! bolt's length characteristic L15 → L18 in stores") is a one-token edit
//! whose fallout is entirely the compiler's enumeration, not a missing-name
//! error.
//!
//! ## Sealed subsystem characteristics (F-026, F-054 style)
//!
//! [`StoresIssued`], [`ProofOfIssue`], [`DisposalSink`] and
//! [`FinishedGoodsStore`] are **sealed**: only this crate's types may carry
//! them. They are the subjects of REQ-019..022, and the seal is what makes
//! each requirement's bound unforgeable — `cs4-line` (or anything else
//! downstream) cannot implement `ProofOfIssue` for a type of its own and
//! smuggle a home-made "issue note" past REQ-021, any more than it can mint
//! the sealed [`crate::resources::IssueNote`] itself (the crate edge,
//! REQ-019/F-005).

/// Seals the subsystem characteristics (the classic sealed-trait pattern,
/// F-026): only this crate's resource types may carry [`StoresIssued`],
/// [`ProofOfIssue`], [`DisposalSink`] and [`FinishedGoodsStore`]. The impls
/// live beside the resource types in [`crate::resources`].
pub(crate) mod sealed {
    /// Implemented only for this crate's sealed resource types.
    pub trait Sealed {}
}

// ---------------------------------------------------------------------------
// Kind traits for the bolt catalogue (R6, F-018).
// ---------------------------------------------------------------------------

/// Kind trait for the size slot of [`crate::resources::Bolt`]. One one-line
/// impl per size value; an unbounded slot would accept transposed arguments
/// silently (F-018).
pub trait Size {}

/// Kind trait for the material slot of [`crate::resources::Bolt`].
pub trait Material {}

/// Kind trait for the length slot of [`crate::resources::Bolt`].
pub trait Length {}

// ---------------------------------------------------------------------------
// Characteristic values: one unit struct + one kind impl per value.
// ---------------------------------------------------------------------------

/// Size value: metric thread M8 (the batch's only size).
pub struct SizeM8;
impl Size for SizeM8 {}

/// Material value: steel.
pub struct Steel;
impl Material for Steel {}

/// Length value: 15 mm (R7 base length unit is the millimetre) — the issued
/// bolt's length.
pub struct L15;
impl Length for L15 {}

/// Length value: 18 mm — catalogued but **not issued**. It exists so SPEC
/// §7's change-impact probe A (L15 → L18 on
/// [`crate::resources::IssuedBolt`]) is a legal one-token edit whose entire
/// fallout is the compiler's blast-radius enumeration.
pub struct L18;
impl Length for L18 {}

// ---------------------------------------------------------------------------
// Characteristic markers (R6). Bridging impls (one line per value, F-019)
// live beside the catalogue type in `crate::resources`.
// ---------------------------------------------------------------------------

/// Characteristic (R6): an M8-threaded fastener.
#[diagnostic::on_unimplemented(message = "`{Self}` is not an M8-threaded fastener: the batch's assemblies take M8 bolts only", label = "an M8 bolt is required here", note = "the issued catalogue bolt is `Bolt<SizeM8, Steel, L15>` (`IssuedBolt`); it enters the model only inside the stores-issued bolt box (REQ-019)")]
pub trait M8Thread {}

/// Characteristic (R6): made of steel.
#[diagnostic::on_unimplemented(message = "`{Self}` is not a steel part: the batch's assemblies take steel bolts only", label = "a steel bolt is required here", note = "the issued catalogue bolt is `Bolt<SizeM8, Steel, L15>` (`IssuedBolt`)")]
pub trait SteelMade {}

/// Characteristic (R6): 15 mm long. Bridged only for bolts whose length slot
/// is [`L15`] — a bolt of any other catalogued length (such as [`L18`]) does
/// not carry it, which is exactly the edge SPEC §7's probe A pushes on.
#[diagnostic::on_unimplemented(message = "`{Self}` is not a 15 mm part: the batch's assemblies take 15 mm bolts only", label = "a 15 mm bolt is required here", note = "the issued catalogue bolt is `Bolt<SizeM8, Steel, L15>` (`IssuedBolt`); `L18` stock is catalogued but not approved for this assembly")]
pub trait Len15 {}

// ---------------------------------------------------------------------------
// Sealed subsystem characteristics: the subjects of REQ-019..022.
// ---------------------------------------------------------------------------

/// Characteristic (R6, sealed): a material created and released by the stores
/// subsystem — REQ-019's subject. Implemented only for this crate's sealed
/// material types; downstream code can neither construct those types (the
/// crate edge, F-005/F-006) nor implement this trait for its own (the private
/// supertrait), so "built only from stores-issued materials" is structural.
#[diagnostic::on_unimplemented(message = "`{Self}` is not a stores-issued material (REQ-019: assemblies may be built only from stores-issued materials)", label = "REQ-019: only stores-issued materials may be built in", note = "materials enter the model only through stores' issue process P1 (`issue_materials`) against a works order; the line cannot mint or source materials itself - that is the crate edge (EXP-08, F-006)")]
pub trait StoresIssued: sealed::Sealed {}

/// Characteristic (R6, sealed): evidence that stores issued materials for the
/// current batch — REQ-021's subject. Implemented only by
/// [`crate::resources::IssueNote`], which only
/// [`crate::resources::processes::issue_materials`] can mint, so holding a
/// value with this characteristic means P1 really ran: a note-less line
/// operation is inexpressible, not merely untested.
#[diagnostic::on_unimplemented(message = "`{Self}` is not a current stores issue note (REQ-021: the line may operate only against a current stores issue note)", label = "REQ-021: the stores issue note is required here", note = "the note is minted only by stores' issue process P1 (`issue_materials`) and reconciled by P6 (`reconcile`); thread it through the batch - nothing else can take its place")]
pub trait ProofOfIssue: sealed::Sealed {}

/// Characteristic (R6, sealed): stores' swarf disposal stream — REQ-020's
/// subject. Implemented only by [`crate::resources::Disposal`], the one exit
/// for the batch's swarf (the line's bin is collection, not disposal).
#[diagnostic::on_unimplemented(message = "`{Self}` is not stores' swarf disposal (REQ-020: all swarf from a batch must be collected in the line's bin and returned to stores' disposal at batch end)", label = "REQ-020: stores' disposal stream is required here", note = "the batch's swarf is collected in the line's bin during the cycles and the full bin is returned at reconciliation (P6), where its contents are consumed by the `Disposal` placeholder sink")]
pub trait DisposalSink: sealed::Sealed {}

/// Characteristic (R6, sealed): stores' finished-goods store — REQ-022's
/// subject. Implemented only by [`crate::resources::FinishedGoods`], the
/// consumer completed assemblies must reach.
#[diagnostic::on_unimplemented(message = "`{Self}` is not the finished-goods store (REQ-022: completed assemblies must be delivered to finished-goods stores)", label = "REQ-022: the finished-goods store is required here", note = "assemblies leave the model only into stores' `FinishedGoods` consumer at reconciliation (P6); it keeps what it consumes, so the batch's 25 delivered assemblies are counted by its contents list")]
pub trait FinishedGoodsStore: sealed::Sealed {}
