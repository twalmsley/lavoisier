//! The stores subsystem's sealed resource family (R1), its creation/exit
//! boundary (R12) and its processes (F-031) — SPEC §3–§5.
//!
//! Layout per F-006/F-031: this module holds the sealed material types, the
//! stock (the type-level sheet rack and bolt box, R12), the evidence tokens
//! and the stores-side consumers, with the creation/exit [`boundary`] and the
//! [`processes`] (which mint quantity-bearing values and therefore live
//! inside the privacy boundary) as child modules.
//!
//! **The crate edge is REQ-019** (SPEC §3): every type here is sealed —
//! private fields, `pub(crate)` mint, no `Clone`/`Default` — so the
//! downstream `cs4-line` crate physically cannot create a sheet, blank,
//! plate, bolt, swarf, assembly or issue note (E0451/privacy, EXP-08/F-005).
//! What the line *can* do is compose the conserving **material transforms**
//! exported by [`processes`] ([`processes::shear_sheet`],
//! [`processes::bore_blank`], [`processes::join_assembly`]): each one only
//! turns sealed values passed in by value into their successor states, with
//! the mass balance const-asserted inside (R3) — a cross-crate instance of
//! R9's "each process step gets a conserving conversion". The crate edge
//! thereby forces every material state change's mint site into this crate,
//! while the line owns the processes that compose them with time, tools and
//! waste routing — a structural consequence of the two-crate split worth
//! recording (SPEC §7/§8 territory).
//!
//! One type per processing state (R9, F-023): a [`Blank`] is not a
//! [`DrilledPlate`], so fastening undrilled blanks is a compile error.
//!
//! ## The operator's shift clock (deviation from R15's const-ms budget — see crate docs)
//!
//! [`Operator<Q>`] carries its remaining shift as a **type-level count of
//! 30 000 ms quanta** ([`QUANTUM_MS`]), because a `const`-generic ms budget
//! cannot descend through `cs4-line`'s recursive `BuildBatch` trait on stable
//! Rust (the F-028/E0207 shape: the recursive impl cannot name `B - 150_000`).
//! The fixed-size draws in [`processes`] each mint a conserved, tripwired
//! [`Effort`] in real milliseconds, recorded into the execution `History`
//! (R16) under the spending process's name; [`Effort`] is this crate's
//! parallel of model-core's `Labour` (`Labour`'s mint is private to
//! model-core — the measured F-043 fork, ~40 lines).

use crate::characteristics::{L15, Len15, Length, M8Thread, Material, Size, SizeM8, Steel, SteelMade, sealed};
// One line on purpose: the traceability grep (F-021) skips `use` lines, but
// only when the line itself starts with `use`.
use crate::requirements::{assert_req019, assert_req020, assert_req021, assert_req022};
use core::marker::PhantomData;
use model_core::boundary::Supplier;
use model_core::history::{Event, Permit, Recordable, UNATTRIBUTED};
use model_core::list::{Cons, Len, Nil};
use model_core::nat::{Nat, Succ, Zero};

// ---------------------------------------------------------------------------
// The batch's quantities (SPEC §3). Independent declarations on purpose: the
// conservation asserts in `processes` are real checks, not derivations — if
// one constant moves (SPEC §7 probe B changes SHEET_G 5000 → 4800), the
// compiler enumerates every balance it breaks on `cargo build` (F-001).
// ---------------------------------------------------------------------------

/// Mass of one steel sheet, in grams (R7).
pub const SHEET_G: u64 = 5000;
/// Mass of one plate blank as cut, in grams.
pub const BLANK_G: u64 = 2250;
/// Mass of one drilled plate, in grams.
pub const PLATE_G: u64 = 2240;
/// Mass of one bolt, in grams.
pub const BOLT_G: u64 = 30;
/// Cut swarf per sheet, in grams.
pub const CUT_SWARF_G: u64 = 500;
/// Drill swarf per blank, in grams.
pub const DRILL_SWARF_G: u64 = 10;
/// Mass of one finished assembly, in grams (2 × 2240 + 4 × 30).
pub const ASSEMBLY_G: u64 = 4600;
/// Total swarf mass of the 25-unit batch, in grams (25 × 500 + 50 × 10).
pub const BATCH_SWARF_G: u64 = 13_000;
/// Total swarf pieces of the batch (25 cut + 50 drill).
pub const BATCH_SWARF_PIECES: u64 = 75;
/// The operator's shift-clock quantum, in milliseconds: every draw is a
/// whole number of quanta (see [`Operator`]).
pub const QUANTUM_MS: u64 = 30_000;
/// The operator's full shift, in quanta: 150 × 30 000 ms = 4 500 000 ms
/// (75 minutes, SPEC §3).
pub const SHIFT_QUANTA: u64 = 150;

// The shift arithmetic stated once, checked at compile time.
const _: () = assert!(SHIFT_QUANTA * QUANTUM_MS == 4_500_000);
const _: () = assert!(2 * BLANK_G + CUT_SWARF_G == SHEET_G);
const _: () = assert!(PLATE_G + DRILL_SWARF_G == BLANK_G);
const _: () = assert!(2 * PLATE_G + 4 * BOLT_G == ASSEMBLY_G);
const _: () = assert!(25 * CUT_SWARF_G + 50 * DRILL_SWARF_G == BATCH_SWARF_G);

// ---------------------------------------------------------------------------
// Materials (R1): one sealed type per processing state (R9, F-023).
// ---------------------------------------------------------------------------

model_core::container_resource! {
    /// A steel sheet of `V` grams — the batch's raw material. Enters the
    /// model only inside the sheet rack that P1 ([`processes::issue_materials`])
    /// issues (R12, REQ-019).
    Sheet,
    unit = "grams",
    must_use = "Sheet is a conserved resource: pass it on or hand it to a Consumer"
}

model_core::container_resource! {
    /// A cut, undrilled plate blank of `V` grams. Its own processing state
    /// (R9): no process accepts it where a drilled plate is required.
    Blank,
    unit = "grams",
    must_use = "Blank is a conserved resource: pass it on or hand it to a Consumer"
}

model_core::container_resource! {
    /// A plate of `V` grams with its bolt holes drilled — a distinct type
    /// from the undrilled blank (R9, F-023), which is what makes "fasten
    /// before drilling" a compile error.
    DrilledPlate,
    unit = "grams",
    must_use = "DrilledPlate is a conserved resource: pass it on or hand it to a Consumer"
}

model_core::container_resource! {
    /// Cut swarf: `V` grams of offcut from shearing one sheet. Tripwired
    /// waste (R1 layer 2, F-008): swarf that never reaches the line's bin
    /// (and, at batch end, [`Disposal`] — REQ-020) panics the test that
    /// leaked it.
    CutSwarf,
    unit = "grams",
    must_use = "CutSwarf is a conserved waste product: feed it to the line's swarf bin (REQ-020)"
}

model_core::container_resource! {
    /// Drill swarf: `V` grams of chips from drilling one blank. Tripwired
    /// waste like [`CutSwarf`] (REQ-020).
    DrillSwarf,
    unit = "grams",
    must_use = "DrillSwarf is a conserved waste product: feed it to the line's swarf bin (REQ-020)"
}

model_core::consumable_resource! {
    /// A bolt from the catalogue: size, material and length are type
    /// parameters (R6), each bounded by its kind trait (F-018). Bolts enter
    /// the model only inside the 100-bolt box P1 issues (R12, REQ-019) and
    /// are conserved from there into an [`Assembly`]. Deliberately **not**
    /// tripwired (`no_tripwire`): bolts are *kept* by the containers that
    /// hold them (the box, the assembly), and a tripwired item inside an
    /// abandoned container would panic from the container's own drop,
    /// masking the real leak site (F-040). Whole-value discard is still
    /// caught by `#[must_use]`.
    Bolt<S: Size, M: Material, L: Length>,
    must_use = "Bolt is a conserved resource: pass it on or hand it to a Consumer",
    no_tripwire
}

// Bridging impls (R6, F-019): one line per characteristic value, added in the
// same commit as the value itself. Any new defaulted parameter on `Bolt`
// requires rewriting every one of these (F-017); `grep 'for Bolt<'` audits.
impl<M: Material, L: Length> M8Thread for Bolt<SizeM8, M, L> {}
impl<S: Size, L: Length> SteelMade for Bolt<S, Steel, L> {}
impl<S: Size, M: Material> Len15 for Bolt<S, M, L15> {}

/// The catalogue bolt stores issues for the batch: M8, steel, 15 mm
/// (SPEC §3). **SPEC §7 probe A** changes this alias's length argument to
/// [`crate::characteristics::L18`] and catalogues everything the compiler
/// reports.
///
/// Satisfies: REQ-019
pub type IssuedBolt = Bolt<SizeM8, Steel, L15>;

// The tag above is for grep; these assertions are for truth (R10, F-020): if
// the catalogue, the bridge or the sealed provenance marker ever stops making
// IssuedBolt what the batch requires, the line below is a type-check-time
// compile error naming the missing characteristic.
model_core::satisfies!(assert_req019, IssuedBolt);
/// Compile-checked backing for the issued bolt's catalogue spec (M8, steel,
/// 15 mm): probe A's first casualty.
const fn assert_issued_bolt_spec<T: M8Thread + SteelMade + Len15>() {}
const _: () = assert_issued_bolt_spec::<IssuedBolt>();

/// A type-level list of exactly four bolts of one catalogue type `B` — the
/// `Taken =` shape the line's `SupplyN<N4>` fasten bound requires (F-014).
pub type FourOf<B> = Cons<B, Cons<B, Cons<B, Cons<B, Nil>>>>;

model_core::consumable_resource! {
    /// The final product (SPEC §3): two drilled plates fastened with four
    /// bolts. `G` is the assembly's total mass in grams (4600, conserved at
    /// compile time by [`processes::join_assembly`]); the four bolts are
    /// conserved as discrete objects (R13), held inside as sealed payload —
    /// real owned objects (R12), untripwired so the sanctioned forget in
    /// `defuse` skips no tripwire (F-040). Tripwired itself; defused only
    /// when [`FinishedGoods`] takes delivery (REQ-022) or the store is
    /// shipped onward ([`boundary::ship_finished_goods`]).
    Assembly<B, const G: u64> {
        bolts: FourOf<B>,
    },
    must_use = "Assembly is a conserved resource: deliver it to finished goods (REQ-022)"
}

impl<B, const G: u64> Assembly<B, G> {
    /// The assembly's total mass, in grams (R7).
    pub const GRAMS: u64 = G;
}

model_core::consumable_resource! {
    /// The works order for the 25-unit batch (SPEC §4): consumed by P1
    /// ([`processes::issue_materials`]) when the materials are issued
    /// against it. Tripwired: an order that is never fulfilled fails the
    /// test that abandoned it.
    WorksOrder,
    must_use = "WorksOrder is a conserved resource: issue materials against it (P1)"
}

model_core::consumable_resource! {
    /// The stores issue note (SPEC §3): the sealed evidence token REQ-021
    /// turns on. Minted only by P1 ([`processes::issue_materials`]), threaded
    /// through every batch cycle by the line's `fasten`, consumed
    /// (reconciled) only by P6 ([`processes::reconcile`]). Tripwired: a note
    /// that is never reconciled fails the test that lost it.
    IssueNote,
    must_use = "IssueNote is the batch's issue evidence (REQ-021): thread it through the batch and reconcile it at the end (P6)"
}

/// The issue note under its requirement-facing name; the alias carries the
/// tag because a tag inside the macro invocation above would be silently
/// dropped by trace.sh (F-037).
///
/// Satisfies: REQ-021
pub type CurrentIssueNote = IssueNote;
model_core::satisfies!(assert_req021, CurrentIssueNote);

model_core::reusable_resource! {
    /// The reconciled issue note (SPEC §6's "note reconciled" end state):
    /// the evidence that P6 closed REQ-021 for the batch. Reusable-shaped
    /// (no tripwire): a record that legitimately stays with the caller.
    ReconciledNote,
    must_use = "ReconciledNote is the batch's reconciliation evidence: it stays with the caller"
}

model_core::container_resource! {
    /// Expended operator effort, in milliseconds: the conserved output of the
    /// shift-clock draws in [`processes`] (R15). Its production-legal sink is
    /// the execution `History` (R16, F-035), reached through the [`Recordable`]
    /// impl below. This is the F-043 parallel of model-core's `Labour`
    /// (`Labour::mint` is private to model-core), forced by the shift clock —
    /// see the crate docs.
    Effort,
    unit = "person-milliseconds",
    must_use = "Effort is a conserved resource: it must be accounted for by the execution History (R16)"
}

/// Expended effort is recordable (R16): `History` defuses it (the consumer's
/// sanctioned F-008 role, reached through the sealed `Permit`) and keeps the
/// value-level record — the production-legal sink F-035 demands.
impl<const MS: u64> Recordable for Effort<MS> {
    fn into_record(self, _permit: Permit) -> Event {
        // Boundary exit: the effort leaves the model into the record.
        self.defuse();
        Event {
            process: UNATTRIBUTED,
            item: "Effort",
            magnitude: MS,
            unit: "person-milliseconds",
        }
    }
}

// ---------------------------------------------------------------------------
// The sealed subsystem characteristics (REQ-019..022 subjects) attach to the
// family here: the seal impls and the markers, one pair per carrying type.
// ---------------------------------------------------------------------------

impl<const G: u64> sealed::Sealed for Sheet<G> {}
impl<const G: u64> crate::characteristics::StoresIssued for Sheet<G> {}
impl<const G: u64> sealed::Sealed for Blank<G> {}
impl<const G: u64> crate::characteristics::StoresIssued for Blank<G> {}
impl<const G: u64> sealed::Sealed for DrilledPlate<G> {}
impl<const G: u64> crate::characteristics::StoresIssued for DrilledPlate<G> {}
impl<const G: u64> sealed::Sealed for CutSwarf<G> {}
impl<const G: u64> crate::characteristics::StoresIssued for CutSwarf<G> {}
impl<const G: u64> sealed::Sealed for DrillSwarf<G> {}
impl<const G: u64> crate::characteristics::StoresIssued for DrillSwarf<G> {}
impl<S: Size, M: Material, L: Length> sealed::Sealed for Bolt<S, M, L> {}
impl<S: Size, M: Material, L: Length> crate::characteristics::StoresIssued for Bolt<S, M, L> {}
impl<B, const G: u64> sealed::Sealed for Assembly<B, G> {}
impl<B, const G: u64> crate::characteristics::StoresIssued for Assembly<B, G> {}
impl sealed::Sealed for IssueNote {}
impl crate::characteristics::ProofOfIssue for IssueNote {}

// ---------------------------------------------------------------------------
// Stock (R12): the 25-sheet rack and the 100-bolt box as type-level list
// suppliers — the SPEC §3/§8 scale targets (F-009/F-010 territory).
// ---------------------------------------------------------------------------

/// Maps a type-level count to the list type of that many `T`s:
/// `ListOf<Sheet<5000>, N25>` is the rack's full contents.
pub trait Replicate<T> {
    /// The list type holding `Self`-many items of type `T`.
    type List;
}
impl<T> Replicate<T> for Zero {
    type List = Nil;
}
impl<T, N: Replicate<T>> Replicate<T> for Succ<N> {
    type List = Cons<T, N::List>;
}

/// Shorthand for [`Replicate::List`].
pub type ListOf<T, N> = <N as Replicate<T>>::List;

/// The sheet rack: a supplier at the system boundary (R12) holding real
/// [`Sheet`] values in a type-level list — the count **is** the list's
/// length. Issued full (25 sheets) by P1; returned empty at reconciliation.
///
/// Placeholder: stores sheet stock — goods-in and procurement are out of
/// scope (SPEC §1), the stock is simply there.
#[must_use = "SheetRack is a boundary resource: pass it on like any other resource"]
pub struct SheetRack<Items>(Items);

/// The rack as issued: 25 sheets of 5000 g (SPEC §3).
pub type FullSheetRack = SheetRack<ListOf<Sheet<SHEET_G>, model_core::nat::aliases::N25>>;

/// The exhausted rack: a distinct resource type that must itself be
/// accounted for — it returns to stores at reconciliation (P6, SPEC §4).
pub type EmptySheetRack = SheetRack<Nil>;

/// `Supplier` is implemented ONLY for a non-empty rack (R12): taking a 26th
/// sheet is a compile error with model-core's modeller-phrased message
/// (F-015) — the edge the line's 26th-cycle probe pushes on.
impl<H, T> Supplier for SheetRack<Cons<H, T>> {
    type Item = H;
    type Next = SheetRack<T>;
    fn supply(self) -> (H, SheetRack<T>) {
        let Cons(head, tail) = self.0;
        (head, SheetRack(tail))
    }
}

impl<Items: Len> SheetRack<Items> {
    /// How many sheets the rack holds — the length of its contents list, so
    /// count and contents cannot disagree (R7, R12).
    pub const COUNT: u64 = Items::LEN;
}

/// The bolt box: a supplier at the system boundary (R12) holding real
/// [`Bolt`] values in a type-level list. Issued full (100 bolts — the F-010
/// measured frontier) by P1; returned empty at reconciliation.
///
/// Placeholder: stores fastener stock — goods-in out of scope (SPEC §1).
#[must_use = "BoltBox is a boundary resource: pass it on like any other resource"]
pub struct BoltBox<Items>(Items);

/// The box as issued: 100 [`IssuedBolt`]s (SPEC §3).
pub type FullBoltBox = BoltBox<ListOf<IssuedBolt, model_core::nat::aliases::N100>>;

/// The exhausted box: a distinct resource type that must itself be accounted
/// for — it returns to stores at reconciliation (P6, SPEC §4).
pub type EmptyBoltBox = BoltBox<Nil>;

/// `Supplier` is implemented ONLY for a non-empty box (R12): a 101st bolt is
/// a compile error with the modeller-phrased message (F-015).
impl<H, T> Supplier for BoltBox<Cons<H, T>> {
    type Item = H;
    type Next = BoltBox<T>;
    fn supply(self) -> (H, BoltBox<T>) {
        let Cons(head, tail) = self.0;
        (head, BoltBox(tail))
    }
}

impl<Items: Len> BoltBox<Items> {
    /// How many bolts the box holds (R7, R12).
    pub const COUNT: u64 = Items::LEN;
}

/// Public-in-signature but unimplementable-outside wrapper over the private
/// fill machinery (the sealed-trait pattern, F-026): the generic fixture
/// constructors in [`test_support`] can name it, but outside code cannot
/// implement it to mint materials.
pub trait FillSealed: fill_sealed::Sealed {
    #[doc(hidden)]
    fn fill_sealed() -> Self;
}

/// Builds a list of real stock items. Private: together with the `mint`s,
/// this is the only place sheets and bolts come into existence (R1),
/// reachable only through P1's fill calls and the feature-gated fixtures.
trait Fill {
    fn fill() -> Self;
}
impl Fill for Nil {
    fn fill() -> Nil {
        Nil
    }
}
impl<const G: u64, T: Fill> Fill for Cons<Sheet<G>, T> {
    fn fill() -> Self {
        Cons(Sheet::mint(), T::fill())
    }
}
impl<S: Size, M: Material, L: Length, T: Fill> Fill for Cons<Bolt<S, M, L>, T> {
    fn fill() -> Self {
        Cons(Bolt::mint(), T::fill())
    }
}

impl<L2: Fill + fill_sealed::Sealed> FillSealed for L2 {
    fn fill_sealed() -> Self {
        L2::fill()
    }
}
mod fill_sealed {
    use super::{Bolt, Sheet};
    use crate::characteristics::{Length, Material, Size};
    use model_core::list::{Cons, Nil};
    pub trait Sealed {}
    impl Sealed for Nil {}
    impl<const G: u64, T: Sealed> Sealed for Cons<Sheet<G>, T> {}
    impl<S: Size, M: Material, L: Length, T: Sealed> Sealed for Cons<Bolt<S, M, L>, T> {}
}

// ---------------------------------------------------------------------------
// The operator's shift clock (R2 reusable; the R15 deviation — crate docs).
// ---------------------------------------------------------------------------

/// The batch's operator (R2, R11): a reusable resource whose remaining shift
/// is the **type-level count `Q` of 30 000 ms quanta** ([`QUANTUM_MS`]).
/// Hand-sealed (R1): private fields, no public constructor, no
/// `Clone`/`Copy`/`Default`; enters only via [`boundary::clock_in`] (a full
/// [`SHIFT_QUANTA`]-quantum shift) or the feature-gated fixture. No tripwire:
/// a reusable resource legitimately outlives the flow (R2).
///
/// The quantum clock exists because a `Person<const BUDGET_MS: u64>` budget
/// cannot descend through the line's recursive `BuildBatch` trait on stable
/// Rust (crate docs; the F-028/E0207 shape). Drawing more quanta than remain
/// is a **type-check-time** error (the Peano route, F-011) rather than the
/// R15 overdraw E0080 — one honest trade recorded with the deviation.
#[must_use = "Operator is a reusable resource: pass them on or return them to the caller"]
pub struct Operator<Q: Nat> {
    _seal: (),
    _q: PhantomData<Q>,
}

impl<Q: Nat> Operator<Q> {
    /// The remaining shift, in quanta.
    pub const QUANTA: u64 = Q::VALUE;

    /// The remaining shift, in milliseconds (R7): `QUANTA × QUANTUM_MS`.
    pub const REMAINING_MS: u64 = Q::VALUE * QUANTUM_MS;

    /// Test fixture: an operator from nowhere with `Q` quanta left, for
    /// downstream test code only (R1, F-004; `test-support` feature,
    /// dev-dependencies only).
    #[cfg(feature = "test-support")]
    pub fn test_fixture() -> Self {
        Operator {
            _seal: (),
            _q: PhantomData,
        }
    }
}

// ---------------------------------------------------------------------------
// Stores-side consumers (R12): finished goods (REQ-022) and disposal
// (REQ-020).
// ---------------------------------------------------------------------------

/// The finished-goods store (SPEC §4): the consumer completed assemblies
/// must reach (REQ-022). `Space` is the type-level number of assemblies it
/// can still accept (≥ 25 for the batch); `Contents` keeps the real
/// assemblies it has taken delivery of (F-016). The decreasing space
/// parameter is a hard rule, not style: a contents-keeping consumer without
/// one diverges trait resolution and, at this workspace's mandated recursion
/// limit, crashes the compiler (F-034).
///
/// Placeholder: finished-goods stores — onward delivery out of scope.
///
/// Satisfies: REQ-022
pub struct FinishedGoods<Space, Contents = Nil> {
    contents: Contents,
    _space: PhantomData<Space>,
}

model_core::satisfies!(assert_req022, FinishedGoods<Zero>);
impl<Space, C> sealed::Sealed for FinishedGoods<Space, C> {}
impl<Space, C> crate::characteristics::FinishedGoodsStore for FinishedGoods<Space, C> {}

/// `Consumer` is implemented ONLY while space remains (R12, F-034): space
/// goes down by one and the real assembly is kept at the front of the
/// contents list (F-016).
impl<S, C, B, const G: u64> model_core::boundary::Consumer<Assembly<B, G>> for FinishedGoods<Succ<S>, C> {
    type Next = FinishedGoods<S, Cons<Assembly<B, G>, C>>;
    fn consume(self, item: Assembly<B, G>) -> Self::Next {
        FinishedGoods {
            contents: Cons(item, self.contents),
            _space: PhantomData,
        }
    }
}

impl<Space, Contents: Len> FinishedGoods<Space, Contents> {
    /// How many assemblies the store holds — the length of its contents
    /// list, so count and contents cannot disagree (R7, R12).
    pub const HELD: u64 = Contents::LEN;
}

/// Stores' swarf disposal stream (SPEC §4): the unbounded boundary sink
/// (`type Next = Self`, legal only at the system boundary, R15/F-029) that
/// REQ-020 routes all batch swarf to at reconciliation. Being unbounded, it
/// necessarily discards what it consumes — the one sanctioned exception to
/// "a consumer keeps what it consumes" (R12, F-029).
///
/// Placeholder: waste-disposal service — assumed able to take any amount.
///
/// Satisfies: REQ-020
#[must_use = "Disposal is a boundary resource: pass it on like any other resource"]
pub struct Disposal {
    _seal: (),
}

model_core::satisfies!(assert_req020, Disposal);
impl sealed::Sealed for Disposal {}
impl crate::characteristics::DisposalSink for Disposal {}

/// Disposal accepts cut swarf of any mass; `Next = Self` (R15, F-029).
impl<const G: u64> model_core::boundary::Consumer<CutSwarf<G>> for Disposal {
    type Next = Disposal;
    fn consume(self, item: CutSwarf<G>) -> Disposal {
        item.defuse(); // sanctioned consumer role (F-008); unbounded sink discards (F-029)
        self
    }
}

/// Disposal accepts drill swarf of any mass; `Next = Self` (R15, F-029).
impl<const G: u64> model_core::boundary::Consumer<DrillSwarf<G>> for Disposal {
    type Next = Disposal;
    fn consume(self, item: DrillSwarf<G>) -> Disposal {
        item.defuse();
        self
    }
}

// ---------------------------------------------------------------------------
// The subsystem interface traits P6 reconciles across the crate edge.
// ---------------------------------------------------------------------------

/// The total mass and piece count of a swarf list — what P6's REQ-020
/// reconciliation asserts over (SPEC §5: 13 000 g, 75 pieces). Sealed
/// (F-026): only lists of this crate's swarf types carry it, so the totals
/// cannot be forged.
pub trait SwarfLoad: swarf_sealed::Sealed {
    /// Total mass of the listed swarf, in grams.
    const GRAMS: u64;
    /// Number of swarf pieces in the list.
    const PIECES: u64;
}
impl SwarfLoad for Nil {
    const GRAMS: u64 = 0;
    const PIECES: u64 = 0;
}
impl<const G: u64, T: SwarfLoad> SwarfLoad for Cons<CutSwarf<G>, T> {
    const GRAMS: u64 = G + T::GRAMS;
    const PIECES: u64 = T::PIECES + 1;
}
impl<const G: u64, T: SwarfLoad> SwarfLoad for Cons<DrillSwarf<G>, T> {
    const GRAMS: u64 = G + T::GRAMS;
    const PIECES: u64 = T::PIECES + 1;
}
mod swarf_sealed {
    use super::{CutSwarf, DrillSwarf};
    use model_core::list::{Cons, Nil};
    pub trait Sealed {}
    impl Sealed for Nil {}
    impl<const G: u64, T: Sealed> Sealed for Cons<CutSwarf<G>, T> {}
    impl<const G: u64, T: Sealed> Sealed for Cons<DrillSwarf<G>, T> {}
}

/// The crate-edge interface P6 empties the line's swarf bin through
/// (REQ-020): stores defines the trait, the line implements it for its bin
/// (stores cannot name the line's types — the dependency points the other
/// way). `open` hands back the kept swarf for disposal and the emptied bin
/// for return to the line (SPEC §5/P6). Implementing it mints nothing: the
/// implementor must already hold the sealed swarf it hands over.
pub trait SwarfReturn {
    /// The swarf contents handed over for disposal.
    type Contents;
    /// The emptied bin, returned to the line.
    type Emptied;
    /// Opens the bin: contents out for disposal, the emptied bin back.
    fn open(self) -> (Self::Contents, Self::Emptied);
}

/// The creation and exit boundary of the stores family (R12, F-006): the
/// only production code where orders, operators, the stores-side consumers —
/// and, inside P1's fills, materials — come into existence, and where the
/// stocked finished goods leave the model.
pub mod boundary {
    use super::{Assembly, Disposal, FillSealed, FinishedGoods, FullBoltBox, FullSheetRack, Operator, PhantomData, WorksOrder};
    use model_core::list::{Cons, Nil};
    use model_core::nat::aliases::N150;

    /// The works order for the 25-unit batch enters the model (R12, SPEC §4).
    ///
    /// Placeholder: production planning — one order, quantity fixed by the
    /// batch.
    pub fn place_works_order() -> WorksOrder {
        WorksOrder::mint()
    }

    /// The operator clocks in at shift start with the full
    /// [`super::SHIFT_QUANTA`]-quantum budget (R12, R15; SPEC §4).
    ///
    /// Placeholder: workforce — one operator, one 75-minute shift.
    pub fn clock_in() -> Operator<N150> {
        Operator {
            _seal: (),
            _q: PhantomData,
        }
    }

    /// An empty finished-goods store with `Space` slots (REQ-022). Creating
    /// an *empty* consumer brings no resources into existence, so this is an
    /// ordinary public boundary function (R12): `new_finished_goods::<N25>()`.
    pub fn new_finished_goods<Space>() -> FinishedGoods<Space, Nil> {
        FinishedGoods {
            contents: Nil,
            _space: PhantomData,
        }
    }

    /// The disposal stream enters the model (R12). An empty unbounded sink
    /// holds nothing, so this too is an ordinary public boundary function.
    pub fn new_disposal() -> Disposal {
        Disposal { _seal: () }
    }

    /// A full 25-sheet rack comes into existence — P1's fill (R1, R12).
    /// Crate-private: materials are released only by `issue_materials`.
    pub(crate) fn full_rack() -> FullSheetRack {
        super::SheetRack(FillSealed::fill_sealed())
    }

    /// A full 100-bolt box comes into existence — P1's fill (R1, R12).
    pub(crate) fn full_box() -> FullBoltBox {
        super::BoltBox(FillSealed::fill_sealed())
    }

    /// Recursively defuses a shipped store's kept assemblies as they leave
    /// the model. Private: together with [`ship_finished_goods`] this is the
    /// only exit for stocked assemblies (R1, F-039).
    trait Dispose {
        fn dispose(self);
    }
    impl Dispose for Nil {
        fn dispose(self) {}
    }
    impl<B, const G: u64, T: Dispose> Dispose for Cons<Assembly<B, G>, T> {
        fn dispose(self) {
            let Cons(assembly, tail) = self;
            assembly.defuse();
            tail.dispose();
        }
    }

    /// Public-in-signature but unimplementable-outside wrapper over the
    /// private disposal machinery (the sealed-trait pattern, F-026), so
    /// [`ship_finished_goods`] can name it without letting outside code
    /// defuse assemblies.
    pub trait ShipSealed: ship_sealed::Sealed {
        #[doc(hidden)]
        fn ship_all(self);
    }
    impl<L: Dispose + ship_sealed::Sealed> ShipSealed for L {
        fn ship_all(self) {
            self.dispose()
        }
    }
    mod ship_sealed {
        use super::super::Assembly;
        use model_core::list::{Cons, Nil};
        pub trait Sealed {}
        impl Sealed for Nil {}
        impl<B, const G: u64, T: Sealed> Sealed for Cons<Assembly<B, G>, T> {}
    }

    /// The stocked finished goods leave the model onward (R12, F-039): the
    /// only place kept assemblies are defused, which is what makes the
    /// tripwires on abandoned assemblies trustworthy (R1, F-008).
    ///
    /// Placeholder: onward delivery from finished goods — out of scope
    /// (SPEC §1).
    pub fn ship_finished_goods<Space, Contents: ShipSealed>(store: FinishedGoods<Space, Contents>) {
        let FinishedGoods {
            contents,
            _space: PhantomData,
        } = store;
        contents.ship_all();
    }
}

/// Downstream-test fixtures (R1, F-004): resources from nowhere, for test
/// code only, enabled exclusively via a `[dev-dependencies]` re-declaration
/// with the `test-support` feature (ci.sh polices the placement). The
/// macro-generated per-type `test_fixture()` constructors live on the types;
/// the stock builders that need the fill machinery live here.
///
/// Positive control for this crate's `compile_fail` doc-tests (the R4
/// caveat: a compile_fail snippet passes if it fails for *any* reason, so
/// this proves the fixtures those snippets lean on really are reachable from
/// doc-test builds — `no_run` because the outputs are deliberately left
/// unaccounted, which only the tripwires at run time would object to):
///
/// ```no_run
/// use cs4_stores::resources::Sheet;
/// use cs4_stores::resources::processes::shear_sheet;
///
/// let sheet = Sheet::<5000>::test_fixture();
/// let (a, b, swarf) = shear_sheet::<5000, 2250, 2250, 500>(sheet);
/// ```
#[cfg(feature = "test-support")]
pub mod test_support {
    use super::{BoltBox, EmptyBoltBox, EmptySheetRack, FillSealed, IssuedBolt, ListOf, Replicate, SHEET_G, Sheet, SheetRack};
    use model_core::list::Nil;

    /// A rack of `N` sheets from nowhere: `fixture_rack::<N1>()`.
    pub fn fixture_rack<N: Replicate<Sheet<SHEET_G>>>() -> SheetRack<ListOf<Sheet<SHEET_G>, N>>
    where
        ListOf<Sheet<SHEET_G>, N>: FillSealed,
    {
        SheetRack(FillSealed::fill_sealed())
    }

    /// A box of `N` issued bolts from nowhere: `fixture_box::<N4>()`.
    pub fn fixture_box<N: Replicate<IssuedBolt>>() -> BoltBox<ListOf<IssuedBolt, N>>
    where
        ListOf<IssuedBolt, N>: FillSealed,
    {
        BoltBox(FillSealed::fill_sealed())
    }

    /// An already-exhausted box, as the batch returns it.
    pub fn fixture_empty_box() -> EmptyBoltBox {
        BoltBox(Nil)
    }

    /// An already-exhausted rack, as the batch returns it.
    pub fn fixture_empty_rack() -> EmptySheetRack {
        SheetRack(Nil)
    }

    /// Test-only settlement for a fixture issue note (F-004): downstream
    /// tests cannot defuse the tripwired note (its seal is this crate's) and
    /// the production exit is P6's full reconciliation, so a per-process test
    /// that only *threads* the note settles it here instead of running the
    /// whole batch.
    pub fn fixture_settle_note(note: super::IssueNote) {
        note.defuse();
    }
}

/// The stores subsystem's processes (R1, R2): pure by-value transformations.
/// They mint quantity-bearing values (blanks, swarf, assemblies, effort), so
/// they live inside the resource family's module (F-031).
///
/// The **material transforms** ([`shear_sheet`], [`bore_blank`],
/// [`join_assembly`]) are the cross-crate conserving conversions the line's
/// processes compose (R9; see the module docs): each demands its sealed
/// inputs by value and const-asserts the mass balance (R3), so the line can
/// change material states without ever being able to mint material. The
/// **subsystem processes** P1 ([`issue_materials`]) and P6 ([`reconcile`])
/// are the stores window the batch starts and ends at (SPEC §5).
pub mod processes {
    use super::{Assembly, BOLT_G, Blank, BoltBox, CutSwarf, DrillSwarf, DrilledPlate, Effort, EmptyBoltBox, EmptySheetRack, FourOf, FullBoltBox, FullSheetRack, IssueNote, Len15, M8Thread, Nat, Operator, PhantomData, ReconciledNote, Sheet, SheetRack, SteelMade, Succ, SwarfLoad, SwarfReturn, WorksOrder, boundary};
    // One line on purpose: the traceability grep (F-021) skips `use` lines,
    // but only when the line itself starts with `use`.
    use crate::requirements::{Req019StoresIssuedMaterial, Req020SwarfToDisposal, Req021CurrentIssueNote, Req022ToFinishedGoods};
    use model_core::boundary::{ConsumeList, send_list};
    use model_core::history::History;
    use model_core::history::processes::record;
    use model_core::list::Nil;

    /// Shears one sheet into two plate blanks plus cut swarf — the material
    /// transform inside the line's P2 (R1, R3, R9). Mass is conserved at
    /// compile time (`A + B + SW == SHEET`); the caller states the split
    /// (outputs cannot be computed on stable, F-022). The check fires at
    /// monomorphization (F-001): `cargo check` and editor diagnostics will
    /// not show a violation — `cargo build`/`cargo test` do, which is what
    /// SPEC §7's probe B (5000 → 4800) demonstrates.
    ///
    /// Regression (R4 policy: conservation violations are rustdoc
    /// `compile_fail` doc-tests, never trybuild cases, F-003): shearing a
    /// 5000 g sheet into 2250 g + 2250 g blanks plus only 400 g of swarf
    /// must not compile — 100 g would vanish:
    ///
    /// ```compile_fail
    /// use cs4_stores::resources::Sheet;
    /// use cs4_stores::resources::processes::shear_sheet;
    ///
    /// let sheet = Sheet::<5000>::test_fixture();
    /// let (a, b, swarf) = shear_sheet::<5000, 2250, 2250, 400>(sheet);
    /// ```
    pub fn shear_sheet<const SHEET: u64, const A: u64, const B: u64, const SW: u64>(
        sheet: Sheet<SHEET>,
    ) -> (Blank<A>, Blank<B>, CutSwarf<SW>) {
        const {
            assert!(
                A + B + SW == SHEET,
                "mass conservation violated in shear_sheet (R3, P2): the two blanks plus the cut swarf must sum exactly to the sheet"
            )
        };
        // Conserving transform: the sheet's mass continues as A + B + SW.
        sheet.defuse();
        (Blank::mint(), Blank::mint(), CutSwarf::mint())
    }

    /// Drills one blank into a drilled plate plus drill swarf — the material
    /// transform inside the line's P3 (R1, R3, R9). Mass is conserved at
    /// compile time (`P + SW == BLANK`; fires at monomorphization, F-001).
    ///
    /// Regression — drilling cannot *add* mass (2245 + 10 ≠ 2250):
    ///
    /// ```compile_fail
    /// use cs4_stores::resources::Blank;
    /// use cs4_stores::resources::processes::bore_blank;
    ///
    /// let blank = Blank::<2250>::test_fixture();
    /// let (plate, swarf) = bore_blank::<2250, 2245, 10>(blank);
    /// ```
    pub fn bore_blank<const BLANK: u64, const P: u64, const SW: u64>(
        blank: Blank<BLANK>,
    ) -> (DrilledPlate<P>, DrillSwarf<SW>) {
        const {
            assert!(
                P + SW == BLANK,
                "mass conservation violated in bore_blank (R3, P3): the drilled plate plus the drill swarf must sum exactly to the blank"
            )
        };
        // Conserving transform: the blank's mass continues as P + SW.
        blank.defuse();
        (DrilledPlate::mint(), DrillSwarf::mint())
    }

    /// Joins two drilled plates and four stores-issued bolts into one
    /// assembly — the material transform inside the line's P4 (R1, R3, R13).
    /// Only REQ-019 stores-issued bolts of the catalogued spec (M8, steel,
    /// 15 mm) are accepted — the requirement is the trait bound on `B`,
    /// never a concrete `Bolt<…>` type (R10, F-019). Mass is conserved at
    /// compile time (`PA + PB + 4 × 30 == OUT`, SPEC §5/P4's
    /// 2 × 2240 + 4 × 30 = 4600); the bolts are conserved as objects, kept
    /// inside the assembly as sealed payload (R13, F-040).
    ///
    /// The whole signature sits on one line because trace.sh attributes
    /// requirement bounds to the line they are written on (R10 rule 4,
    /// F-021).
    ///
    /// Regression — an assembly cannot weigh more than its parts
    /// (2240 + 2240 + 120 ≠ 4700):
    ///
    /// ```compile_fail
    /// use cs4_stores::resources::processes::join_assembly;
    /// use cs4_stores::resources::{Bolt, DrilledPlate, IssuedBolt};
    /// use model_core::list::{Cons, Nil};
    ///
    /// let a = DrilledPlate::<2240>::test_fixture();
    /// let b = DrilledPlate::<2240>::test_fixture();
    /// let bolts = Cons(IssuedBolt::test_fixture(), Cons(IssuedBolt::test_fixture(), Cons(IssuedBolt::test_fixture(), Cons(IssuedBolt::test_fixture(), Nil))));
    /// let assembly = join_assembly::<_, 2240, 2240, 4700>(a, b, bolts);
    /// ```
    ///
    /// Satisfies: REQ-019
    pub fn join_assembly<B: Req019StoresIssuedMaterial + M8Thread + SteelMade + Len15, const PA: u64, const PB: u64, const OUT: u64>(a: DrilledPlate<PA>, b: DrilledPlate<PB>, bolts: FourOf<B>) -> Assembly<B, OUT> {
        const {
            assert!(
                PA + PB + 4 * BOLT_G == OUT,
                "mass conservation violated in join_assembly (R3, P4): the assembly must weigh exactly its two plates plus its four bolts (2 x 2240 + 4 x 30 = 4600)"
            )
        };
        // Conserving transforms: the plates' mass continues inside OUT; the
        // bolts continue as objects inside the assembly (R13) — mint is the
        // conserving combinator, only wrapping values passed in by value.
        a.defuse();
        b.defuse();
        Assembly::mint(bolts)
    }

    // The fixed-size shift-clock draws (R15 deviation — see the crate docs):
    // each spends a whole number of quanta and mints the Effort in real
    // milliseconds; the quantum arithmetic is stated and compile-checked
    // once, here.
    const _: () = assert!(30_000 == super::QUANTUM_MS);
    const _: () = assert!(60_000 == 2 * super::QUANTUM_MS);
    const _: () = assert!(120_000 == 4 * super::QUANTUM_MS);

    /// Draws one 30 000 ms quantum from the operator's shift clock (R15):
    /// the per-blank drilling and per-assembly fastening spend (SPEC §5).
    /// Spending a quantum the operator does not have is a type-check-time
    /// error (the clock's Peano route; contrast F-001's build-only E0080s).
    ///
    /// Regression — a clocked-out operator cannot work:
    ///
    /// ```compile_fail
    /// use cs4_stores::resources::Operator;
    /// use cs4_stores::resources::processes::draw_effort_30k;
    /// use model_core::nat::aliases::N0;
    ///
    /// let operator = Operator::<N0>::test_fixture();
    /// let (effort, operator) = draw_effort_30k(operator);
    /// ```
    pub fn draw_effort_30k<Q: Nat>(operator: Operator<Succ<Q>>) -> (Effort<30_000>, Operator<Q>) {
        // Reusable resource: a plain conserving move (no tripwire to defuse).
        let Operator { _seal: (), _q: _ } = operator;
        (
            Effort::mint(),
            Operator {
                _seal: (),
                _q: PhantomData,
            },
        )
    }

    /// Draws two quanta (60 000 ms): the per-sheet cutting spend (SPEC §5).
    pub fn draw_effort_60k<Q: Nat>(operator: Operator<Succ<Succ<Q>>>) -> (Effort<60_000>, Operator<Q>) {
        let Operator { _seal: (), _q: _ } = operator;
        (
            Effort::mint(),
            Operator {
                _seal: (),
                _q: PhantomData,
            },
        )
    }

    /// Draws four quanta (120 000 ms): the stores-window spend of P1 and P6
    /// (SPEC §5). **Budget overdraw regression** — four quanta cannot come
    /// out of a three-quantum remainder (type-check-time on the clock's
    /// Peano route):
    ///
    /// ```compile_fail
    /// use cs4_stores::resources::Operator;
    /// use cs4_stores::resources::processes::draw_effort_120k;
    /// use model_core::nat::aliases::N3;
    ///
    /// let operator = Operator::<N3>::test_fixture();
    /// let (effort, operator) = draw_effort_120k(operator);
    /// ```
    pub fn draw_effort_120k<Q: Nat>(operator: Operator<Succ<Succ<Succ<Succ<Q>>>>>) -> (Effort<120_000>, Operator<Q>) {
        let Operator { _seal: (), _q: _ } = operator;
        (
            Effort::mint(),
            Operator {
                _seal: (),
                _q: PhantomData,
            },
        )
    }

    /// P1 — issue materials against the works order (SPEC §5). The operator
    /// draws 120 000 ms at the stores window (recorded under this process's
    /// name, R16); the order is consumed; the full sheet rack, the full bolt
    /// box and the **issue note** (REQ-021's key) are handed to the line.
    /// Balances: items out = stock (structural — the fills are the stock);
    /// time → History (structural).
    ///
    /// Satisfies: REQ-019, REQ-021
    pub fn issue_materials<Q: Nat>(order: WorksOrder, operator: Operator<Succ<Succ<Succ<Succ<Q>>>>>, history: History) -> (FullSheetRack, FullBoltBox, IssueNote, Operator<Q>, History) {
        let (effort, operator) = draw_effort_120k(operator);
        let history = record(history, "issue_materials", effort);
        // The order is fulfilled by this issue: its exit from the model.
        order.defuse();
        (
            boundary::full_rack(),
            boundary::full_box(),
            IssueNote::mint(),
            operator,
            history,
        )
    }

    /// P6 — return and reconcile at batch end (SPEC §5). The operator draws
    /// 120 000 ms; the batch's assemblies are delivered to finished goods
    /// (REQ-022); the line's full bin is opened across the crate edge
    /// ([`SwarfReturn`]) and its swarf consumed by [`Disposal`] (REQ-020),
    /// with the totals const-asserted (`SWARF_G` grams, `SWARF_PIECES`
    /// pieces — the flow states 13 000 g / 75, SPEC §5; fires at
    /// monomorphization, F-001); the emptied bin goes back to the line; the
    /// empty box and rack return to stores (destructured — accounted); and
    /// the issue note is reconciled (REQ-021 closed), leaving the
    /// [`ReconciledNote`] evidence.
    ///
    /// Call shape (F-043's turbofish note): the two stated totals lead and
    /// the type parameters infer —
    /// `reconcile::<13_000, 75, _, _, _, _, _>(…)`.
    ///
    /// Reconciliation-total regression — the batch's bin cannot come back
    /// light (one 500 g piece is not the stated 510 g / 2 pieces):
    ///
    /// ```compile_fail
    /// use cs4_stores::resources::boundary::{new_disposal, new_finished_goods};
    /// use cs4_stores::resources::processes::reconcile;
    /// use cs4_stores::resources::test_support::{fixture_empty_box, fixture_empty_rack};
    /// use cs4_stores::resources::{CutSwarf, IssueNote, Operator, SwarfReturn};
    /// use model_core::history::boundary::new_history;
    /// use model_core::list::{Cons, Nil};
    /// use model_core::nat::aliases::{N1, N4};
    ///
    /// struct TestBin(Cons<CutSwarf<500>, Nil>);
    /// impl SwarfReturn for TestBin {
    ///     type Contents = Cons<CutSwarf<500>, Nil>;
    ///     type Emptied = ();
    ///     fn open(self) -> (Self::Contents, ()) {
    ///         (self.0, ())
    ///     }
    /// }
    ///
    /// let bin = TestBin(Cons(CutSwarf::<500>::test_fixture(), Nil));
    /// let (fg, bin, disposal, note, operator, history) = reconcile::<510, 2, _, _, _, _, _>(Nil, new_finished_goods::<N1>(), bin, new_disposal(), fixture_empty_box(), fixture_empty_rack(), IssueNote::test_fixture(), Operator::<N4>::test_fixture(), new_history());
    /// ```
    ///
    /// Satisfies: REQ-020, REQ-021, REQ-022
    // Nine arguments: P6 is the batch's join point and takes everything the
    // batch returns, threaded loosely on purpose (R9, F-024 — an aggregate
    // here would over-claim nothing, but the spec's P6 interface lists these
    // nine and the loose threading keeps each return visible).
    #[allow(clippy::too_many_arguments)]
    pub fn reconcile<const SWARF_G: u64, const SWARF_PIECES: u64, AS, FG: Req022ToFinishedGoods + ConsumeList<AS>, FB: SwarfReturn, DS: Req020SwarfToDisposal + ConsumeList<FB::Contents>, Q: Nat>(assemblies: AS, finished_goods: FG, bin: FB, disposal: DS, bolt_box: EmptyBoltBox, sheet_rack: EmptySheetRack, note: IssueNote, operator: Operator<Succ<Succ<Succ<Succ<Q>>>>>, history: History) -> (FG::Next, FB::Emptied, DS::Next, ReconciledNote, Operator<Q>, History) where FB::Contents: SwarfLoad, IssueNote: Req021CurrentIssueNote {
        const {
            assert!(
                <FB::Contents as SwarfLoad>::GRAMS == SWARF_G,
                "swarf mass reconciliation violated in reconcile (R3, REQ-020): the returned bin's swarf must weigh exactly the stated disposal total (the batch states 13 000 g)"
            )
        };
        const {
            assert!(
                <FB::Contents as SwarfLoad>::PIECES == SWARF_PIECES,
                "swarf piece-count reconciliation violated in reconcile (REQ-020): the returned bin must hold exactly the stated number of pieces (the batch states 75)"
            )
        };
        let (effort, operator) = draw_effort_120k(operator);
        let history = record(history, "reconcile", effort);
        // REQ-022: the batch's assemblies are delivered to finished goods —
        // through the generic access process, so capacity mistakes take the
        // modeller-phrased trait-bound path (F-015).
        let finished_goods = send_list(finished_goods, assemblies);
        // REQ-020: the bin is opened across the crate edge and its swarf is
        // consumed by disposal; the emptied bin goes back to the line.
        let (swarf, emptied_bin) = bin.open();
        let disposal = send_list(disposal, swarf);
        // The empty box and rack return to stores: destructured, accounted.
        let BoltBox(Nil) = bolt_box;
        let SheetRack(Nil) = sheet_rack;
        // REQ-021 closed: the note is reconciled — its exit from the model —
        // and the reconciliation evidence takes its place.
        note.defuse();
        (
            finished_goods,
            emptied_bin,
            disposal,
            ReconciledNote::mint(),
            operator,
            history,
        )
    }
}

// In-crate tests may construct the operator directly; this tiny helper keeps
// the field expression in one place without exposing anything.
#[cfg(test)]
impl<Q: Nat> Operator<Q> {
    pub(crate) fn test_fixture_free() -> Self {
        Operator {
            _seal: (),
            _q: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    //! Per-process unit tests (R5): every stores process turns specific
    //! inputs into the expected outputs with nothing left unaccounted for.
    //! These tests sit inside the privacy boundary, so they may mint
    //! fixtures and defuse outputs directly; downstream-style accounting is
    //! exercised by `cs4-line`'s tests.

    use super::boundary::{clock_in, new_disposal, new_finished_goods, place_works_order, ship_finished_goods};
    use super::processes::{bore_blank, draw_effort_30k, draw_effort_60k, draw_effort_120k, issue_materials, join_assembly, reconcile, shear_sheet};
    use super::{Assembly, Blank, BoltBox, CutSwarf, DrillSwarf, DrilledPlate, Disposal, FinishedGoods, FullBoltBox, FullSheetRack, IssueNote, IssuedBolt, Operator, Sheet, SheetRack, SwarfReturn};
    use model_core::boundary::send_to;
    use model_core::history::boundary::new_history;
    use model_core::history::Entry;
    use model_core::list::{Cons, Nil};
    use model_core::nat::aliases::{N1, N4, N143, N146, N150};
    use model_core::nat::Zero;

    /// Test-only drain for stock a test ends up holding: sheets are
    /// tripwired and must be defused inside the privacy boundary; bolts are
    /// untripwired (kept-item types, F-040) and are simply let go.
    trait Drain {
        fn drain(self);
    }
    impl Drain for Nil {
        fn drain(self) {}
    }
    impl<const G: u64, T: Drain> Drain for Cons<Sheet<G>, T> {
        fn drain(self) {
            let Cons(sheet, tail) = self;
            sheet.defuse();
            tail.drain();
        }
    }
    impl<S: crate::characteristics::Size, M: crate::characteristics::Material, L: crate::characteristics::Length, T: Drain> Drain for Cons<super::Bolt<S, M, L>, T> {
        fn drain(self) {
            let Cons(bolt, tail) = self;
            let _untripwired_kept_item = bolt;
            tail.drain();
        }
    }

    /// `shear_sheet` conserves mass (R3, R5): the values are recoverable as
    /// constants and balance by construction — the compile-time assert
    /// already checked the split.
    #[test]
    fn shear_conserves_mass_and_exposes_values() {
        let sheet = Sheet::<5000>::mint();
        let (a, b, swarf) = shear_sheet::<5000, 2250, 2250, 500>(sheet);
        assert_eq!(Blank::<2250>::VALUE + Blank::<2250>::VALUE + CutSwarf::<500>::VALUE, 5000);
        assert_eq!(CutSwarf::<500>::UNIT, "grams");
        a.defuse();
        b.defuse();
        swarf.defuse();
    }

    /// `bore_blank` conserves mass (R3, R5) and converts the processing
    /// state (R9): the output is a `DrilledPlate`, not a `Blank`.
    #[test]
    fn bore_conserves_mass_and_changes_state() {
        let blank = Blank::<2250>::mint();
        let (plate, swarf) = bore_blank::<2250, 2240, 10>(blank);
        assert_eq!(DrilledPlate::<2240>::VALUE + DrillSwarf::<10>::VALUE, 2250);
        plate.defuse();
        swarf.defuse();
    }

    /// `join_assembly` conserves the plates' mass plus the four bolts into
    /// the 4600 g assembly (R3, R13) and the finished-goods store takes
    /// delivery (REQ-022), keeping what it consumes (F-016).
    ///
    /// Verifies: REQ-019, REQ-022
    #[test]
    fn join_conserves_and_finished_goods_keeps_the_assembly() {
        let a = DrilledPlate::<2240>::mint();
        let b = DrilledPlate::<2240>::mint();
        let bolts = Cons(IssuedBolt::mint(), Cons(IssuedBolt::mint(), Cons(IssuedBolt::mint(), Cons(IssuedBolt::mint(), Nil))));
        let assembly: Assembly<IssuedBolt, 4600> = join_assembly(a, b, bolts);
        assert_eq!(Assembly::<IssuedBolt, 4600>::GRAMS, 4600);
        let store = new_finished_goods::<N1>();
        assert_eq!(FinishedGoods::<N1>::HELD, 0);
        let store = send_to(store, assembly);
        assert_eq!(FinishedGoods::<Zero, Cons<Assembly<IssuedBolt, 4600>, Nil>>::HELD, 1);
        // The stocked store leaves the model through its sealed exit
        // (F-039): the only place kept assemblies are defused.
        ship_finished_goods(store);
    }

    /// The shift clock draws down in whole quanta and every draw's effort is
    /// real milliseconds that reach the execution history attributed to the
    /// spending process (R15, R16).
    #[test]
    fn clock_draws_down_and_effort_reaches_history() {
        let operator = clock_in();
        assert_eq!(Operator::<N150>::REMAINING_MS, 4_500_000);
        let (e1, operator) = draw_effort_60k(operator);
        let (e2, operator) = draw_effort_30k(operator);
        let (e3, operator) = draw_effort_120k(operator);
        assert_eq!(Operator::<N143>::QUANTA, 143);
        let history = model_core::history::processes::record(new_history(), "cut", e1);
        let history = model_core::history::processes::record(history, "drill_blank", e2);
        let history = model_core::history::processes::record(history, "reconcile", e3);
        assert_eq!(history.event_count(), 3);
        match history.entries() {
            [Entry::Event(a), Entry::Event(b), Entry::Event(c)] => {
                assert_eq!((a.process, a.magnitude), ("cut", 60_000));
                assert_eq!((b.process, b.magnitude), ("drill_blank", 30_000));
                assert_eq!((c.process, c.magnitude), ("reconcile", 120_000));
                assert_eq!(a.unit, "person-milliseconds");
            }
            other => panic!("expected three flat events, got {other:?}"),
        }
        let _execution_record = history;
        let _operator_keeps_143_quanta = operator;
    }

    /// P1 issues the full stock against the works order: 25 sheets, 100
    /// bolts, the issue note, 120 000 ms drawn and recorded (SPEC §5).
    ///
    /// Verifies: REQ-019, REQ-021
    #[test]
    fn issue_materials_issues_the_batch_stock() {
        let (rack, bolt_box, note, operator, history) =
            issue_materials(place_works_order(), clock_in(), new_history());
        assert_eq!(FullSheetRack::COUNT, 25);
        assert_eq!(FullBoltBox::COUNT, 100);
        assert_eq!(Operator::<N146>::QUANTA, 146);
        assert_eq!(history.event_count(), 1);
        // Inside the privacy boundary the test drains the stock it holds;
        // the real batch consumes it (cs4-line's tests).
        let SheetRack(sheets) = rack;
        sheets.drain();
        let BoltBox(bolts) = bolt_box;
        bolts.drain();
        note.defuse();
        let _execution_record = history;
        let _operator = operator;
    }

    /// P6 reconciles a (small, test-scale) return: assemblies delivered,
    /// swarf disposed with the stated totals, box and rack destructured,
    /// note reconciled, emptied bin handed back — exercised through a local
    /// bin so the crate-edge `SwarfReturn` interface is proven
    /// implementation-agnostic.
    ///
    /// Verifies: REQ-020, REQ-021, REQ-022
    #[test]
    fn reconcile_accounts_for_everything_returned() {
        struct TestBin(Cons<CutSwarf<500>, Cons<DrillSwarf<10>, Nil>>);
        struct TestBinEmptied;
        impl SwarfReturn for TestBin {
            type Contents = Cons<CutSwarf<500>, Cons<DrillSwarf<10>, Nil>>;
            type Emptied = TestBinEmptied;
            fn open(self) -> (Self::Contents, TestBinEmptied) {
                (self.0, TestBinEmptied)
            }
        }

        let a = DrilledPlate::<2240>::mint();
        let b = DrilledPlate::<2240>::mint();
        let bolts = Cons(IssuedBolt::mint(), Cons(IssuedBolt::mint(), Cons(IssuedBolt::mint(), Cons(IssuedBolt::mint(), Nil))));
        let assemblies = Cons(join_assembly::<_, 2240, 2240, 4600>(a, b, bolts), Nil);
        let bin = TestBin(Cons(CutSwarf::mint(), Cons(DrillSwarf::mint(), Nil)));
        let (finished_goods, emptied, disposal, reconciled, operator, history) = reconcile::<510, 2, _, _, _, _, _>(assemblies, new_finished_goods::<N1>(), bin, new_disposal(), BoltBox(Nil), SheetRack(Nil), IssueNote::mint(), Operator::<N4>::test_fixture_free(), new_history());
        assert_eq!(FinishedGoods::<Zero, Cons<Assembly<IssuedBolt, 4600>, Nil>>::HELD, 1);
        assert_eq!(Operator::<Zero>::QUANTA, 0);
        assert_eq!(history.event_count(), 1);
        let TestBinEmptied = emptied;
        let Disposal { _seal: () } = disposal;
        ship_finished_goods(finished_goods);
        let _kept = (reconciled, operator, history);
    }

    /// Disposal accepts both swarf kinds without bound (`Next = Self`,
    /// R15/F-029).
    ///
    /// Verifies: REQ-020
    #[test]
    fn disposal_accepts_both_swarf_kinds() {
        let disposal = new_disposal();
        let disposal = send_to(disposal, CutSwarf::<500>::mint());
        let disposal = send_to(disposal, DrillSwarf::<10>::mint());
        let Disposal { _seal: () } = disposal;
    }

    /// An abandoned issue note is caught by its tripwire at test time (R1
    /// layer 2, F-008): the batch's evidence must be reconciled, never
    /// dropped.
    #[test]
    #[should_panic(expected = "resource leak: IssueNote dropped without being consumed")]
    fn abandoned_issue_note_trips_the_tripwire() {
        let note = IssueNote::mint();
        let _still_bound_but_never_reconciled = note;
    }
}
