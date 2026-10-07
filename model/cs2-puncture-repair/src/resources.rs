//! The workshop bay's sealed resource family (R1), its creation/exit boundary
//! (R12) and its processes P1..P6 (SPEC.md §5, F-031).
//!
//! Layout per F-006/F-031: this module holds the sealed resource types, with
//! the [`boundary`] (the bay setup, the service intake, the kit fill, the
//! wallet, the counter and the sinks) and the [`processes`] (which mint
//! quantity-bearing values and therefore live inside the privacy boundary) as
//! child modules.
//!
//! One type per processing state (R9, F-023), straight from SPEC.md §3:
//!
//! * wheel: [`PuncturedWheel`] (2080 g) → [`OpenWheel`] (rim + tyre, 1900 g)
//!   → [`ServiceableWheel`] (2083 g on the patched paths, 2080 g with the
//!   spare — the mass is carried in the type);
//! * tube: [`PuncturedTube`] (180 g) → [`LocatedTube`] (180 g) →
//!   [`PatchedTube`] (183 g) → [`CheckedTube`] (183 g, the `Airtight` patched
//!   state) — or [`DeadTube`] (180 g, to rubber recycling on path 3); the new
//!   [`SpareTube`] (180 g) is `Airtight` from the counter;
//! * patch: [`DryPatch`] (2 g, kept by the kit) → [`SpentPatch`] (3 g, incl.
//!   cement — fed to the waste stream inside `patch_tube`, REQ-013).
//!
//! The **nested kit** ([`PatchKit`]) holds its two discrete patches as a
//! type-level list of real objects (R12, R13) *and* its 30 g cement tube as
//! the kit's own continuous magnitude (R15): patches leave one at a time
//! through the kit's `Supplier` impl, cement is drawn 1 g per attempt by
//! [`processes::draw_cement`] — both through the kit's boundary, so neither
//! kind of content can drift from the kit's type.
//!
//! Every consumable here carries the kernel's tripwire `Drop` (R1 layer 2,
//! F-008) except the [`DryPatch`], which is *kept* by the kit that accounts
//! for it (`no_tripwire`, F-040). Reusable resources (the workstand, the
//! pump, the member — model-core's `Qualified<Induction, MS>` with the SPEC's
//! 1 800 000 ms budget) are moved in and returned by every process (R2) and
//! stay with the caller.

use crate::characteristics::{
    Airtight, ExactPriceCounter, Induction, PunctureLocated, WasteStreamConsumer, sealed,
};
// One line on purpose: the traceability grep (F-021) skips `use` lines, but
// only when the line itself starts with `use`.
use crate::requirements::{assert_req010, assert_req011, assert_req012, assert_req013, assert_req014, Req011LocatedBeforePatching};
use model_core::boundary::{Consumer, Supplier};
use model_core::common::Qualified;
use model_core::list::{Cons, Len, Nil};

// ---------------------------------------------------------------------------
// The wheel's states (one type per state, R9; masses in the types).
// ---------------------------------------------------------------------------

model_core::consumable_resource! {
    /// The punctured rear wheel as it arrives at the bay (SPEC.md §3: 2080 g
    /// — 1900 g rim + tyre with the 180 g punctured tube inside). Enters the
    /// model only through [`boundary::wheel_arrives_for_service`] (R12).
    /// Tripwired (F-008): a wheel the flow never opens fails the test that
    /// leaked it.
    PuncturedWheel,
    must_use = "PuncturedWheel is a conserved resource: open it or hand it to a Consumer"
}

impl PuncturedWheel {
    /// The punctured wheel's mass, in grams (R7; SPEC.md §3).
    pub const MASS_G: u64 = 2080;
}

model_core::consumable_resource! {
    /// The open wheel — rim + tyre with the tube out (SPEC.md P1: 1900 g).
    /// A distinct processing state (R9): it cannot be ridden; only
    /// [`processes::refit_and_inflate`] (with an `Airtight` tube, REQ-012)
    /// turns it serviceable. Tripwired (F-008).
    OpenWheel,
    must_use = "OpenWheel is a conserved resource: refit it or hand it to a Consumer"
}

impl OpenWheel {
    /// The open wheel's mass (rim + tyre), in grams (R7; SPEC.md §3).
    pub const MASS_G: u64 = 1900;
}

model_core::container_resource! {
    /// The serviceable wheel: rim + tyre + an airtight tube, `V` grams in
    /// all (SPEC.md §6: 2083 g on the patched paths, 2080 g with the spare —
    /// the paths genuinely end at different masses, review decision 4). The
    /// final product; it leaves the model with the [`WheelOwner`]. Tripwired
    /// (F-008).
    ServiceableWheel,
    unit = "grams",
    must_use = "ServiceableWheel is the product: hand it to the owner (a Consumer)"
}

// ---------------------------------------------------------------------------
// The inner tube's states (one type per state, R9).
// ---------------------------------------------------------------------------

model_core::consumable_resource! {
    /// The punctured inner tube, straight out of the wheel (SPEC.md §3:
    /// 180 g). It cannot be patched — only the located-puncture state
    /// satisfies REQ-011 (no blind patching). Tripwired (F-008).
    PuncturedTube,
    must_use = "PuncturedTube is a conserved resource: find the hole or hand it to a Consumer"
}

impl PuncturedTube {
    /// The punctured tube's mass, in grams (R7; SPEC.md §3).
    pub const MASS_G: u64 = 180;
}

model_core::consumable_resource! {
    /// The tube with its puncture **located** (SPEC.md P2: 180 g — a state
    /// change, no mass change). The only tube state `patch_tube` accepts
    /// (REQ-011). Tripwired (F-008): a located tube that is neither patched
    /// nor retired to recycling fails the test that leaked it.
    LocatedTube,
    must_use = "LocatedTube is a conserved resource: patch it, retire it to recycling, or hand it to a Consumer"
}

/// The located-puncture characteristic (R6) lives on the located state only
/// (R9/F-023): `into_patch_site` is the permit-gated conserving extraction
/// that only [`processes::patch_tube`]'s success arm can call — the tube's
/// mass continues into the patched tube, checked by that process's
/// conservation asserts (F-054).
impl PunctureLocated for LocatedTube {
    const MASS_G: u64 = 180;
    fn into_patch_site(self, _permit: PatchPermit) {
        // Conserving transform: the tube's 180 g continue inside the patched
        // tube minted by patch_tube (R1).
        self.defuse();
    }
}
impl sealed::Sealed for LocatedTube {}

/// The located tube under its requirement-facing name; the alias carries the
/// tag because a tag inside the `consumable_resource!` invocation above would
/// be silently dropped by trace.sh (F-037).
///
/// Satisfies: REQ-011
pub type TubeReadyToPatch = LocatedTube;
model_core::satisfies!(assert_req011, TubeReadyToPatch);

model_core::container_resource! {
    /// The patched tube: `V` grams (SPEC.md P3: 183 g — the 180 g tube plus
    /// a 2 g patch and 1 g of cement). Patched but **not yet checked**: it
    /// does not satisfy REQ-012 — only [`processes::check_patch`] produces
    /// the airtight state. Tripwired (F-008).
    PatchedTube,
    unit = "grams",
    must_use = "PatchedTube is a conserved resource: check it or hand it to a Consumer"
}

model_core::container_resource! {
    /// The checked, ready tube: `V` grams (SPEC.md P4: 183 g). The
    /// `Airtight` **patched** state (REQ-012) — one of the requirement's two
    /// satisfying types, alongside the new [`SpareTube`]. Tripwired (F-008).
    CheckedTube,
    unit = "grams",
    must_use = "CheckedTube is a conserved resource: fit it to the wheel or hand it to a Consumer"
}

/// The airtight characteristic (R6) on the checked **patched** state
/// (REQ-012's first satisfying type): `fit_into_wheel` is the permit-gated
/// conserving extraction only [`processes::refit_and_inflate`] can call — the
/// tube's mass continues into the serviceable wheel, checked by that
/// process's per-instantiation mass assert (F-054).
impl<const V: u64> Airtight for CheckedTube<V> {
    const MASS_G: u64 = V;
    fn fit_into_wheel(self, _permit: FitPermit) {
        // Conserving transform: the tube's mass continues inside the
        // serviceable wheel minted by refit_and_inflate (R1).
        self.defuse();
    }
}
impl<const V: u64> sealed::Sealed for CheckedTube<V> {}

/// The checked patched tube at the CS-2 quantities, under its
/// requirement-facing name (REQ-012's **first** satisfying type — the
/// requirement is deliberately multi-type, SPEC.md §8 review decision 7). The
/// alias carries the tag because a tag inside the macro invocation would be
/// silently dropped by trace.sh (F-037).
///
/// Satisfies: REQ-012
pub type AirtightPatchedTube = CheckedTube<183>;
model_core::satisfies!(assert_req012, AirtightPatchedTube);

model_core::consumable_resource! {
    /// A new spare inner tube from the parts counter (SPEC.md §3: 180 g,
    /// price 650 p). `Airtight` as sold — REQ-012's **second** satisfying
    /// type. Tripwired (F-008): a bought spare that is never fitted fails the
    /// test that leaked it.
    SpareTube,
    must_use = "SpareTube is a conserved resource: fit it to the wheel or hand it to a Consumer"
}

/// The airtight characteristic (R6) on the **new** spare (REQ-012's second
/// satisfying type — the multi-type requirement, review decision 7).
impl Airtight for SpareTube {
    const MASS_G: u64 = 180;
    fn fit_into_wheel(self, _permit: FitPermit) {
        self.defuse();
    }
}
impl sealed::Sealed for SpareTube {}

/// The new spare under its requirement-facing name (REQ-012's **second**
/// satisfying type; tag on the alias per F-037).
///
/// Satisfies: REQ-012
pub type AirtightSpareTube = SpareTube;
model_core::satisfies!(assert_req012, AirtightSpareTube);

model_core::consumable_resource! {
    /// The dead tube: twice-failed and given up on (SPEC.md §3: 180 g; path
    /// 3 only). Its only exit is the [`RubberRecycling`] boundary sink.
    /// Tripwired (F-008): a dead tube that never reaches recycling fails the
    /// test that leaked it.
    DeadTube,
    must_use = "DeadTube is a conserved waste product: it must reach rubber recycling"
}

impl DeadTube {
    /// The dead tube's mass, in grams (R7; SPEC.md §3).
    pub const MASS_G: u64 = 180;
}

// ---------------------------------------------------------------------------
// Patches, cement, and the nested kit (SPEC.md §3).
// ---------------------------------------------------------------------------

model_core::consumable_resource! {
    /// A dry puncture patch: a discrete item, its own object (R13; SPEC.md
    /// §3: 2 g, see [`DryPatch::MASS_G`]). Deliberately **not** tripwired
    /// (`no_tripwire`, F-040): dry patches are *kept* by the kit that holds
    /// them, and a tripwired item inside an abandoned kit would panic from
    /// the kit's own drop, masking the real leak site. Whole-value discard is
    /// still caught by `#[must_use]`.
    DryPatch,
    must_use = "DryPatch is a conserved resource: pass it on or hand it to a Consumer",
    no_tripwire
}

impl DryPatch {
    /// A dry patch's mass, in grams (R7; SPEC.md §3).
    pub const MASS_G: u64 = 2;
}

model_core::container_resource! {
    /// A spent patch: `V` grams (SPEC.md §3: 3 g — the 2 g patch plus the
    /// 1 g of cement that failed to seal). Tripwired (F-008) — but in
    /// production it never exists loose: `patch_tube`'s failure arm feeds it
    /// to the [`WasteStream`] **inside the process** (REQ-013 structural).
    SpentPatch,
    unit = "grams",
    must_use = "SpentPatch is a conserved waste product: it must reach the workshop waste stream (REQ-013)"
}

model_core::container_resource! {
    /// Cement drawn from the kit's cement tube, in grams (R15; SPEC.md §3:
    /// 1 g per application). Minted only by [`processes::draw_cement`] — the
    /// kit's boundary — and consumed by [`processes::patch_tube`]. Tripwired
    /// (F-008).
    Cement,
    unit = "grams",
    must_use = "Cement is a conserved resource: apply it in a patch attempt or hand it to a Consumer"
}

/// The patch kit: a **nested container** (SPEC.md §3). `Patches` is a
/// type-level list of the real [`DryPatch`] objects it holds (R12, R13 — the
/// count *is* the list's length), and `CEMENT_G` is the grams remaining in
/// its cement tube (R15 — the kit's own continuous magnitude). Patches leave
/// one at a time through the `Supplier` impl below; cement is drawn through
/// [`processes::draw_cement`] — both through the kit's boundary. CS-2's kit
/// enters with 2 patches and 30 g ([`boundary::new_patch_kit`]) and the
/// remainder stays with the member at flow end.
///
/// Placeholder: patch kit — brand/vendor not modelled (SPEC.md §4).
#[must_use = "PatchKit is a boundary resource: pass it on like any other resource"]
pub struct PatchKit<Patches, const CEMENT_G: u64> {
    patches: Patches,
    _seal: (),
}

/// A type-level list of exactly two dry patches — the kit as it enters the
/// model (SPEC.md §3: the kit holds 2; the rework bound, F-050).
pub type TwoPatches = Cons<DryPatch, Cons<DryPatch, Nil>>;

/// A type-level list of one dry patch — the kit after one attempt.
pub type OnePatch = Cons<DryPatch, Nil>;

/// No patches left — the kit after both attempts. It cannot supply: there is
/// no `Supplier` impl for it (R12).
pub type NoPatches = Nil;

/// The kit as it enters the model: 2 patches, 30 g of cement (SPEC.md §3).
pub type FreshPatchKit = PatchKit<TwoPatches, 30>;

/// `Supplier` is implemented ONLY for a kit with patches left (R12); taking a
/// patch from an empty kit is a compile error with the modeller-phrased
/// `on_unimplemented` message from model-core (F-015). A third attempt is
/// therefore inexpressible: the kit holds exactly two patches (F-050).
impl<H, T, const C: u64> Supplier for PatchKit<Cons<H, T>, C> {
    type Item = H;
    type Next = PatchKit<T, C>;
    fn supply(self) -> (H, PatchKit<T, C>) {
        let PatchKit {
            patches: Cons(head, tail),
            _seal: (),
        } = self;
        (
            head,
            PatchKit {
                patches: tail,
                _seal: (),
            },
        )
    }
}

impl<Patches: Len, const CEMENT_G: u64> PatchKit<Patches, CEMENT_G> {
    /// How many patches the kit holds — the length of its contents list, so
    /// count and contents cannot disagree (R7, R12).
    pub const PATCHES: u64 = Patches::LEN;

    /// The grams of cement left in the kit's cement tube (R7, R15).
    pub const CEMENT_G: u64 = CEMENT_G;
}

// ---------------------------------------------------------------------------
// Money: cash and the wallet (R19; one currency, pence).
// ---------------------------------------------------------------------------

model_core::container_resource! {
    /// Cash in hand, in pence (GBP, integer minor units — the R7 money row;
    /// R19). Drawn from the [`Wallet`], split for change
    /// ([`processes::split_money`]), and conserved like any other quantity —
    /// abandoned change trips the tripwire.
    Money,
    unit = "pence",
    must_use = "Money is a conserved resource: pass it on, put it back in the wallet, or hand it to a Consumer"
}

model_core::container_resource! {
    /// The member's wallet holding `V` pence (R15 quantity container applied
    /// to money, R19; SPEC.md §3: 1000 p at flow start). `Wallet<0>` is the
    /// empty state: a distinct resource that must still be accounted for
    /// ([`boundary::take_wallet_home`]). Tripwired (F-008).
    Wallet,
    unit = "pence remaining",
    must_use = "Wallet is a conserved resource: even an empty wallet must be passed on or taken home at the boundary"
}

/// The listed price of a spare tube at the [`PartsCounter`], in pence
/// (SPEC.md §3: 650 p). Stated once (F-051); the counter's `Consumer` impl
/// exists **only** at this amount, so a wrong payment is a type-check-time
/// error naming the right price (REQ-014 structural).
pub const SPARE_TUBE_PRICE_PENCE: u64 = 650;

// ---------------------------------------------------------------------------
// The member, the tools, and the permits.
// ---------------------------------------------------------------------------

/// The inducted member at the bay: model-core's R18 `Qualified` wrapper
/// holding the common `Person` with the [`Induction`] qualification and
/// `BUDGET_MS` person-milliseconds left (SPEC.md §3: 1 800 000 ms at flow
/// start). The alias carries the satisfaction tag (F-037), and `Qualified` is
/// only REQ-010-fit with the induction qualification in its slot (F-043).
///
/// Satisfies: REQ-010
pub type WorkshopMember<const BUDGET_MS: u64> = Qualified<Induction, BUDGET_MS>;
model_core::satisfies!(assert_req010, WorkshopMember<0>);

model_core::reusable_resource! {
    /// The workshop's workstand (SPEC.md §3): the tool REQ-010 gates — only
    /// an inducted member may use it ([`processes::open_wheel`],
    /// [`processes::refit_and_inflate`]). Reusable (R2): moved in and
    /// returned; no tripwire.
    ///
    /// Placeholder: bay setup at flow start — one workstand.
    Workstand,
    must_use = "Workstand is a reusable resource: pass it on or return it to the caller"
}

model_core::reusable_resource! {
    /// The track pump (SPEC.md §3): used to find the hole, check the patch
    /// and inflate the refitted wheel. Reusable (R2): moved in and returned;
    /// no tripwire.
    ///
    /// Placeholder: bay setup at flow start — one pump.
    Pump,
    must_use = "Pump is a reusable resource: pass it on or return it to the caller"
}

/// The permit gating [`PunctureLocated::into_patch_site`] (the F-054
/// permit-gated extraction pattern): a private field and no public
/// constructor, so only [`processes::patch_tube`] (inside this privacy
/// boundary) can call the extraction — it can never be used to vanish a tube
/// outside the process whose asserts account for it (R1).
pub struct PatchPermit {
    _seal: (),
}

/// The permit gating [`Airtight::fit_into_wheel`] (F-054): only
/// [`processes::refit_and_inflate`] can call the extraction.
pub struct FitPermit {
    _seal: (),
}

// ---------------------------------------------------------------------------
// The patch outcome token (R17, F-042) and the fallible step's bundles.
// ---------------------------------------------------------------------------

model_core::outcome_token! {
    /// One trial of the environment (R17): whether a single patch attempt
    /// seals the tube or fails. Consumed by exactly one process,
    /// [`processes::patch_tube`]; a flow cannot read it — it must run the
    /// process and handle both arms of the `Result`. Two are provisioned per
    /// repair (SPEC.md §3), bounding the rework to two attempts alongside the
    /// kit's two patches (F-050).
    PatchOutcome(PatchOutcomeKind),
    success = patch_will_hold,
    failure = patch_will_fail,
    exit = return_patch_outcome,
    must_use = "PatchOutcome is a boundary token: run it through exactly one patch attempt or return it to the environment"
}

/// Everything a successful patch attempt produces (R17): the patched tube
/// plus every conserved participant — the member comes back (the time draw is
/// an adjacent process in the flow, F-048, so the member returns at the same
/// budget in both arms) and the waste consumer comes back untouched (nothing
/// failed).
///
/// A bundle is a **grouping** (R1), not a sealed resource (F-046): its fields
/// are public so the handling arm can destructure it, and building one
/// requires already *holding* the sealed resources, so it cannot mint
/// anything. It deliberately has **no `Debug` impl** (F-047), which makes
/// `Result::unwrap()`/`expect()` on the process result a compile error — see
/// `tests/ui/unwrap_patch_result.rs`.
#[must_use = "PatchOk bundles conserved outputs: every field must be accounted for"]
pub struct PatchOk<W, const B: u64, const PATCHED_G: u64> {
    /// The member, back unchanged (the time draw is adjacent, R17/F-048).
    pub member: Qualified<Induction, B>,
    /// The patched tube (SPEC.md P3: 183 g) — not yet checked (REQ-012 needs
    /// [`processes::check_patch`] first).
    pub tube: PatchedTube<PATCHED_G>,
    /// The waste consumer, back untouched: a successful attempt produces no
    /// spent patch.
    pub waste: W,
}

/// Everything a failed patch attempt produces (R17). **Failure conserves
/// too**: the member comes back, the tube comes back still with its located
/// puncture (ready for the provisioned retry or retirement, F-050), and the
/// spent patch has already been fed to the waste stream **inside the
/// process** (REQ-013 structural — failed patches never exist loose), so the
/// bundle carries the waste consumer's next state.
///
/// Like [`PatchOk`] it is a grouping with public fields and no `Debug`
/// (F-046, F-047).
#[must_use = "PatchFail bundles conserved failure outputs: every field must be accounted for"]
pub struct PatchFail<T: Req011LocatedBeforePatching, W, const B: u64> {
    /// The member, back unchanged (the time was drawn before the branch —
    /// failure cost it too, R17).
    pub member: Qualified<Induction, B>,
    /// The tube, back in its located-puncture state (one type per state,
    /// R9/R17): ready for the second provisioned attempt, or for retirement
    /// to recycling.
    pub tube: T,
    /// The waste consumer after taking the spent patch (REQ-013: fed inside
    /// the process).
    pub waste: W,
}

// ---------------------------------------------------------------------------
// Boundary objects at the bay's edge (SPEC.md §4).
// ---------------------------------------------------------------------------

model_core::reusable_resource! {
    /// The parts counter at the system boundary (R19): consumes GBP money at
    /// its exact listed price and supplies new spare tubes. Both roles have
    /// `Next = Self` — an unbounded boundary object, legal only at the
    /// boundary (R15, F-029).
    ///
    /// Placeholder: workshop stores — assumed never to run out of spare
    /// tubes; refine to a finite stock.
    PartsCounter,
    must_use = "PartsCounter is a boundary object: pass it on or return it to the caller"
}

/// The exact-price characteristic (R6, F-051) on the counter.
impl ExactPriceCounter for PartsCounter {}

/// The counter under its requirement-facing name; the alias carries the tag
/// because a tag inside the `reusable_resource!` invocation above would be
/// silently dropped by trace.sh (F-037).
///
/// Satisfies: REQ-014
pub type ListedPriceCounter = PartsCounter;
model_core::satisfies!(assert_req014, ListedPriceCounter);

/// The counter accepts payment **only at its exact listed price** (REQ-014
/// structural, F-051; R12 strictness: one item, one price, one step — a
/// purchase that needs change is the separate process
/// [`processes::buy_spare`]). An unbounded sink discards its intake (F-029):
/// the money leaves the model here.
impl Consumer<Money<SPARE_TUBE_PRICE_PENCE>> for PartsCounter {
    type Next = PartsCounter;
    fn consume(self, payment: Money<SPARE_TUBE_PRICE_PENCE>) -> PartsCounter {
        payment.defuse(); // sanctioned consumer role (F-008); unbounded sink discards (F-029)
        self
    }
}

/// The counter supplies new spare tubes: an unbounded `Next = Self` goods
/// source (a *discrete* source, so a `Supplier` impl is legal — F-028
/// restricts the trait to discrete items).
///
/// Placeholder: unbounded stock — spares are minted at the boundary
/// (R12/R15 licence).
impl Supplier for PartsCounter {
    type Item = SpareTube;
    type Next = PartsCounter;
    fn supply(self) -> (SpareTube, PartsCounter) {
        (SpareTube::mint(), self)
    }
}

/// The workshop waste stream (SPEC.md §4): the dedicated consumer REQ-013
/// routes every failed patch to. An **unbounded** sink (`type Next = Self`,
/// R15, F-029) — deliberately the *other* sink shape from CS-1's finite bin,
/// so both shapes are demonstrated (review decision 2); it necessarily
/// discards what it consumes, and there is no disposal step this time.
///
/// Placeholder: workshop waste stream — assumed able to take any number of
/// spent patches.
///
/// Satisfies: REQ-013
#[must_use = "WasteStream is a boundary resource: pass it on like any other resource"]
pub struct WasteStream {
    _seal: (),
}

// Compile-checked backing for the tag above (R10, F-020).
model_core::satisfies!(assert_req013, WasteStream);

impl WasteStreamConsumer for WasteStream {}

/// The stream absorbs spent patches of any mass; `Next = Self` (R15).
impl<const V: u64> Consumer<SpentPatch<V>> for WasteStream {
    type Next = WasteStream;
    fn consume(self, item: SpentPatch<V>) -> WasteStream {
        item.defuse(); // sanctioned consumer role (F-008); unbounded sink discards (F-029)
        self
    }
}

/// Rubber recycling (SPEC.md §4): the unbounded boundary sink the dead tube
/// leaves to on path 3 (`type Next = Self`, R15, F-029). It accepts only the
/// [`DeadTube`] — a punctured or located tube cannot be handed to it
/// directly, so giving up is an explicit state change
/// ([`processes::retire_tube`]).
///
/// Placeholder: rubber recycling — assumed unbounded.
#[must_use = "RubberRecycling is a boundary resource: pass it on like any other resource"]
pub struct RubberRecycling {
    _seal: (),
}

/// Recycling accepts dead tubes; `Next = Self` (R15, F-029).
impl Consumer<DeadTube> for RubberRecycling {
    type Next = RubberRecycling;
    fn consume(self, item: DeadTube) -> RubberRecycling {
        item.defuse();
        self
    }
}

/// The wheel's owner taking delivery of the serviceable wheel: an unbounded
/// boundary sink (`type Next = Self`, R15, F-029; SPEC.md §4). Only the
/// finished product is accepted — an open wheel, say, has no consumer here.
///
/// Placeholder: the member's bike — wheel refitting to the bicycle is out of
/// scope (SPEC.md §1).
#[must_use = "WheelOwner is a boundary resource: pass it on like any other resource"]
pub struct WheelOwner {
    _seal: (),
}

/// The owner accepts the serviceable wheel at whichever mass its path
/// produced; `Next = Self` (R15, F-029).
impl<const G: u64> Consumer<ServiceableWheel<G>> for WheelOwner {
    type Next = WheelOwner;
    fn consume(self, item: ServiceableWheel<G>) -> WheelOwner {
        item.defuse();
        self
    }
}

/// The creation and exit boundary of the bay's resource family (R12, F-006):
/// the only production code where the wheel, the kit, the wallet, the tools,
/// the counter and the sinks come into existence, and where the wallet leaves
/// the model (SPEC.md §4).
pub mod boundary {
    use super::{
        Cons, DryPatch, FreshPatchKit, Nil, PartsCounter, Pump, PuncturedWheel, RubberRecycling,
        Wallet, WasteStream, WheelOwner, Workstand,
    };

    // The outcome token's boundary constructors and exit are generated by
    // `model_core::outcome_token!` at the family level; they are re-exported
    // here because boundary constructors live in the boundary module (R12).
    pub use super::{patch_will_fail, patch_will_hold, return_patch_outcome};

    /// The punctured wheel enters the model via the service intake (R12;
    /// SPEC.md §4).
    ///
    /// Placeholder: the member's bike — the wheel arrives off the bike
    /// (SPEC.md §1).
    pub fn wheel_arrives_for_service() -> PuncturedWheel {
        PuncturedWheel::mint()
    }

    /// The workstand enters the model (R12).
    ///
    /// Placeholder: bay setup at flow start — one workstand.
    pub fn new_workstand() -> Workstand {
        Workstand::mint()
    }

    /// The pump enters the model (R12).
    ///
    /// Placeholder: bay setup at flow start — one pump.
    pub fn new_pump() -> Pump {
        Pump::mint()
    }

    /// The fill function (R12): a fresh patch kit enters the model — 2 real
    /// dry patches held as a type-level list plus 30 g of cement as the
    /// kit's continuous magnitude (SPEC.md §3; the nested container).
    ///
    /// Placeholder: patch kit — brand/vendor not modelled.
    pub fn new_patch_kit() -> FreshPatchKit {
        super::PatchKit {
            patches: Cons(DryPatch::mint(), Cons(DryPatch::mint(), Nil)),
            _seal: (),
        }
    }

    /// The member's wallet enters the model holding `PENCE` pence (R12, R19;
    /// SPEC.md §3: 1000 p — the member arrives with cash already in hand,
    /// SPEC.md §7).
    ///
    /// Placeholder: the member's wallet.
    pub fn wallet_with<const PENCE: u64>() -> Wallet<PENCE> {
        Wallet::mint()
    }

    /// The wallet leaves the model with the member (R12): the boundary sink
    /// for [`Wallet`] at flow end, whatever it still holds — including the
    /// empty state `Wallet<0>` (R15: every end-state resource is accounted
    /// for).
    ///
    /// Placeholder: the member's pocket — an unbounded sink for wallets.
    pub fn take_wallet_home<const V: u64>(wallet: Wallet<V>) {
        // Sanctioned boundary consumption (F-008 role).
        wallet.defuse();
    }

    /// The parts counter enters the model (R12).
    ///
    /// Placeholder: workshop stores.
    pub fn new_parts_counter() -> PartsCounter {
        PartsCounter::mint()
    }

    /// The workshop waste stream enters the model (R12). An empty unbounded
    /// sink holds nothing, so this is an ordinary public boundary function.
    pub fn new_waste_stream() -> WasteStream {
        WasteStream { _seal: () }
    }

    /// Rubber recycling enters the model (R12).
    pub fn new_rubber_recycling() -> RubberRecycling {
        RubberRecycling { _seal: () }
    }

    /// The wheel's owner enters the model (R12).
    pub fn new_wheel_owner() -> WheelOwner {
        WheelOwner { _seal: () }
    }
}

/// The bay's processes P1..P6 (SPEC.md §5): pure by-value transformations
/// (R1, R2). They mint quantity-bearing values, so they live inside the
/// resource family's module (F-031).
///
/// The member is threaded loosely through every process (R9, F-024) — one
/// actor, a linear repair (SPEC.md §6: CS-2's stress is fallibility, not
/// concurrency). Per F-048 the member's time is drawn by **adjacent**
/// `qualified_draw_time` processes in the flow (never inside these
/// processes), each draw's labour recorded into the single `History`
/// attributed to the process name (R16); each P3 attempt's draw happens
/// *before* the fallible branch, so failure costs the same time as success
/// (R17).
pub mod processes {
    use super::{
        Cement, CheckedTube, DeadTube, DryPatch, FitPermit, LocatedTube, Money, OpenWheel,
        PatchFail, PatchKit, PatchOk, PatchOutcome, PatchOutcomeKind, PatchPermit, PatchedTube,
        PuncturedTube, PuncturedWheel, Pump, Qualified, SPARE_TUBE_PRICE_PENCE, ServiceableWheel,
        SpareTube, SpentPatch, Wallet, Workstand,
    };
    use crate::characteristics::Induction;
    // One line on purpose: the traceability grep (F-021) skips `use` lines,
    // but only when the line itself starts with `use`.
    use crate::requirements::{Req010InductedAtTheWorkstand, Req011LocatedBeforePatching, Req012AirtightTubeOnly, Req013FailedPatchesToWasteStream, Req014ListedPriceOnly};
    use model_core::boundary::{Consumer, Supplier, send_to, take_one};

    // -- Money processes (R19; R3 split/combine over the wallet) ------------

    model_core::draw_process! {
        /// Draws `TAKE` pence from a wallet holding `FULL`, leaving `LEFT`
        /// (R15 pattern applied to money, R19; caller-stated remainder,
        /// F-022/F-030). **Overspending is a compile error**: no `LEFT`
        /// exists with `TAKE + LEFT == FULL` when `TAKE > FULL` — an E0080 at
        /// monomorphization, invisible to `cargo check` (F-001).
        pub fn draw_cash: Wallet => Money,
        assert = "overspend: money conservation violated in draw_cash (R15/R19): TAKE + LEFT must equal FULL - is the draw larger than the wallet's balance?"
    }

    /// Puts cash back into the wallet (R3 combine, caller-stated total,
    /// F-022): `Wallet<BALANCE>` + `Money<ADD>` → `Wallet<NEW>` with
    /// `BALANCE + ADD == NEW` checked at compile time. A wrong total is the
    /// same E0080 as a bad draw (F-001).
    pub fn deposit<const ADD: u64, const BALANCE: u64, const NEW: u64>(
        wallet: Wallet<BALANCE>,
        cash: Money<ADD>,
    ) -> Wallet<NEW> {
        const {
            assert!(
                BALANCE + ADD == NEW,
                "money conservation violated in deposit (R3/R19): BALANCE + ADD must equal NEW - is the stated new balance wrong?"
            )
        };
        // Conserving transform: both inputs continue as the new balance.
        wallet.defuse();
        cash.defuse();
        Wallet::mint()
    }

    /// Splits cash into two amounts (R3: changing an amount is a process) —
    /// the change-giving primitive. `A + B == IN` is checked at compile time;
    /// a violation is the standard E0080 (F-001).
    pub fn split_money<const IN: u64, const A: u64, const B: u64>(
        cash: Money<IN>,
    ) -> (Money<A>, Money<B>) {
        const {
            assert!(
                A + B == IN,
                "money conservation violated in split_money (R3/R19): the two output amounts must sum exactly to the input amount"
            )
        };
        cash.defuse();
        (Money::mint(), Money::mint())
    }

    // -- The kit's cement draw (R15, through the nested kit's boundary) -----

    /// Draws `TAKE` grams of cement from the kit's cement tube holding
    /// `FULL`, leaving `LEFT` (R15 caller-stated remainder, F-022/F-030) —
    /// the kit's **continuous** boundary, beside its discrete `Supplier`
    /// side (the nested container, SPEC.md §3). SPEC.md P3 draws 1 g per
    /// attempt. Hand-written rather than `draw_process!` because the kit is
    /// generic over its held patches.
    ///
    /// Overdrawing the cement tube is a compile error (R15; E0080 at
    /// monomorphization, invisible to `cargo check`, F-001). Regression —
    /// a 30 g tube cannot give 31 g:
    ///
    /// ```compile_fail
    /// use cs2_puncture_repair::resources::boundary::new_patch_kit;
    /// use cs2_puncture_repair::resources::processes::draw_cement;
    ///
    /// let kit = new_patch_kit();
    /// let (cement, kit) = draw_cement::<31, 0, 30, _>(kit);
    /// ```
    pub fn draw_cement<const TAKE: u64, const LEFT: u64, const FULL: u64, Patches>(
        kit: PatchKit<Patches, FULL>,
    ) -> (Cement<TAKE>, PatchKit<Patches, LEFT>) {
        const {
            assert!(
                TAKE + LEFT == FULL,
                "cement conservation violated in draw_cement (R15): TAKE + LEFT must equal FULL - is the draw larger than the kit's remaining cement?"
            )
        };
        // Conserving transform: the kit's cement continues as TAKE + LEFT;
        // the held patches pass through untouched.
        let PatchKit {
            patches,
            _seal: (),
        } = kit;
        (
            Cement::mint(),
            PatchKit {
                patches,
                _seal: (),
            },
        )
    }

    // -- P1..P6 --------------------------------------------------------------

    /// P1 — opens the wheel on the workstand and removes the tube
    /// (SPEC.md P1). REQ-010 bounds the member (style A, F-048): an
    /// un-inducted person fails with the REQ-phrased `on_unimplemented`
    /// message (F-044). The workstand is returned (R2); the member's
    /// 240 000 ms is drawn by the adjacent `qualified_draw_time` in the flow
    /// (F-048). Mass 2080 = 1900 + 180 is checked at compile time
    /// (SPEC.md P1).
    ///
    /// Satisfies: REQ-010
    pub fn open_wheel<O: Req010InductedAtTheWorkstand>(member: O, workstand: Workstand, wheel: PuncturedWheel) -> (O, Workstand, OpenWheel, PuncturedTube) {
        const {
            assert!(
                PuncturedWheel::MASS_G == OpenWheel::MASS_G + PuncturedTube::MASS_G,
                "mass conservation violated in open_wheel (R3): the punctured wheel must split exactly into the open wheel (rim + tyre) and the punctured tube"
            )
        };
        // Conserving transform: the wheel's 2080 g continue as 1900 + 180.
        wheel.defuse();
        (member, workstand, OpenWheel::mint(), PuncturedTube::mint())
    }

    /// P2 — finds the hole by inflating and listening/feeling (SPEC.md P2):
    /// a state change, no mass change (180 = 180, structural — both states
    /// carry the same fixed mass). The pump is returned (R2); the member's
    /// 180 000 ms is drawn by the adjacent `qualified_draw_time` in the flow
    /// (F-048). This process is what *enables* REQ-011: only its output
    /// state can be patched.
    pub fn find_hole<const B: u64>(
        member: Qualified<Induction, B>,
        pump: Pump,
        tube: PuncturedTube,
    ) -> (Qualified<Induction, B>, Pump, LocatedTube) {
        // Conserving state change (R9): the tube's 180 g continue unchanged.
        tube.defuse();
        (member, pump, LocatedTube::mint())
    }

    /// P3 — patches the tube: **fallible** (R17, SPEC.md P3). Takes one
    /// located tube (REQ-011, style A: a wrong tube state fails with the
    /// REQ-phrased message, F-044/F-054), one dry patch from the kit, the
    /// 1 g of cement drawn through the kit's boundary, one outcome token
    /// (the only way variability enters, F-042), and the waste consumer
    /// (REQ-013): on failure the spent patch is fed to the waste stream
    /// **inside this process**, so failed patches never exist loose —
    /// REQ-013 is structural. The member returns in both arms at the same
    /// budget (the 120 000 ms per attempt is drawn by the adjacent
    /// `qualified_draw_time` *before* the branch, F-048).
    ///
    /// Per-branch conservation is **one independent compile-time assert per
    /// arm**, and both fire at every instantiation (R17/F-045, the R4
    /// caveat): the caller states the success split
    /// (`tube + patch + cement == PATCHED_G`) *and* the failure split
    /// (`tube + patch + cement == tube + SPENT_G`) even though only one
    /// branch runs. The `Result` is `#[must_use]` (std) on top of the
    /// bundles' own `must_use`, and the bundles implement no `Debug`, so
    /// `.unwrap()`/`.expect()` do not compile (F-047) — the flow must
    /// `match`.
    ///
    /// Success-branch conservation violation — patching cannot add mass
    /// beyond its inputs (180 + 2 + 1 ≠ 184); an E0080 at monomorphization
    /// (F-001), invisible to `cargo check`, caught by
    /// `cargo build`/`cargo test`:
    ///
    /// ```compile_fail
    /// use cs2_puncture_repair::characteristics::Induction;
    /// use cs2_puncture_repair::resources::boundary::{new_patch_kit, new_pump, new_waste_stream, new_workstand, patch_will_hold, wheel_arrives_for_service};
    /// use cs2_puncture_repair::resources::processes::{draw_cement, find_hole, open_wheel, patch_tube};
    /// use model_core::boundary::take_one;
    /// use model_core::common::boundary::{new_person, qualify};
    ///
    /// let member = qualify::<Induction, 1_800_000>(new_person::<1_800_000>());
    /// let (member, stand, open, tube) = open_wheel(member, new_workstand(), wheel_arrives_for_service());
    /// let (member, pump, located) = find_hole(member, new_pump(), tube);
    /// let (patch, kit) = take_one(new_patch_kit());
    /// let (cement, kit) = draw_cement::<1, 29, 30, _>(kit);
    /// let r = patch_tube::<1, 184, 3, 1_800_000, _, _>(member, located, patch, cement, patch_will_hold(), new_waste_stream());
    /// ```
    ///
    /// Failure-branch conservation violation — a failed attempt cannot lose
    /// the cement (the spent patch is 2 + 1 = 3 g, not 4 g), rejected even
    /// when the flow only ever runs the success branch (F-045):
    ///
    /// ```compile_fail
    /// use cs2_puncture_repair::characteristics::Induction;
    /// use cs2_puncture_repair::resources::boundary::{new_patch_kit, new_pump, new_waste_stream, new_workstand, patch_will_hold, wheel_arrives_for_service};
    /// use cs2_puncture_repair::resources::processes::{draw_cement, find_hole, open_wheel, patch_tube};
    /// use model_core::boundary::take_one;
    /// use model_core::common::boundary::{new_person, qualify};
    ///
    /// let member = qualify::<Induction, 1_800_000>(new_person::<1_800_000>());
    /// let (member, stand, open, tube) = open_wheel(member, new_workstand(), wheel_arrives_for_service());
    /// let (member, pump, located) = find_hole(member, new_pump(), tube);
    /// let (patch, kit) = take_one(new_patch_kit());
    /// let (cement, kit) = draw_cement::<1, 29, 30, _>(kit);
    /// let r = patch_tube::<1, 183, 4, 1_800_000, _, _>(member, located, patch, cement, patch_will_hold(), new_waste_stream());
    /// ```
    ///
    /// Satisfies: REQ-011, REQ-013
    pub fn patch_tube<const CEMENT_G: u64, const PATCHED_G: u64, const SPENT_G: u64, const B: u64, T: Req011LocatedBeforePatching, W: Req013FailedPatchesToWasteStream + Consumer<SpentPatch<SPENT_G>>>(member: Qualified<Induction, B>, tube: T, patch: DryPatch, cement: Cement<CEMENT_G>, outcome: PatchOutcome, waste: W) -> Result<PatchOk<W, B, PATCHED_G>, PatchFail<T, <W as Consumer<SpentPatch<SPENT_G>>>::Next, B>> {
        const {
            assert!(
                T::MASS_G + DryPatch::MASS_G + CEMENT_G == PATCHED_G,
                "success-branch mass conservation violated in patch_tube (R3, R17): the patched tube must sum the tube, the patch and the cement exactly"
            )
        };
        const {
            assert!(
                T::MASS_G + DryPatch::MASS_G + CEMENT_G == T::MASS_G + SPENT_G,
                "failure-branch mass conservation violated in patch_tube (R3, R17): the returned tube plus the spent patch must sum the tube, the patch and the cement exactly"
            )
        };
        match outcome.consume_kind() {
            PatchOutcomeKind::Success => {
                // The permit-gated conserving extraction (F-054): the tube's
                // mass continues into the patched tube minted below; the
                // patch and cement continue with it.
                tube.into_patch_site(PatchPermit { _seal: () });
                let DryPatch { _seal: _ } = patch;
                cement.defuse();
                Ok(PatchOk {
                    member,
                    tube: PatchedTube::mint(),
                    waste,
                })
            }
            PatchOutcomeKind::Failure => {
                // The patch and cement continue as the spent patch, which is
                // fed to the waste stream INSIDE the process (REQ-013
                // structural): a failed patch never exists loose.
                let DryPatch { _seal: _ } = patch;
                cement.defuse();
                let waste = send_to(waste, SpentPatch::<SPENT_G>::mint());
                Err(PatchFail {
                    member,
                    tube,
                    waste,
                })
            }
        }
    }

    /// P4 — checks the patch by inflating and watching it hold
    /// (SPEC.md P4): a state change, no mass change (`G` flows through
    /// structurally). The pump is returned (R2); the member's 60 000 ms is
    /// drawn by the adjacent `qualified_draw_time` in the flow (F-048;
    /// SPEC.md §8 review decision 5: P4 gets its own draw). This process is
    /// what *enables* REQ-012's patched route: only its output state is
    /// `Airtight`.
    pub fn check_patch<const G: u64, const B: u64>(
        member: Qualified<Induction, B>,
        pump: Pump,
        tube: PatchedTube<G>,
    ) -> (Qualified<Induction, B>, Pump, CheckedTube<G>) {
        // Conserving state change (R9): the tube's mass continues unchanged.
        tube.defuse();
        (member, pump, CheckedTube::mint())
    }

    /// P5 — buys a spare tube at the parts counter (R19, SPEC.md P5; path 3
    /// only). REQ-014 bounds the counter (style A): the counter consumes
    /// only `Money<650>` — the F-051 exact-price pattern — so paying any
    /// other amount is a type-check-time E0308 naming the right price. The
    /// tendered cash is split into price and change (R3), the price leaves
    /// the model at the counter, the change stays with the member, and the
    /// new spare comes back; money and goods each conserve in their own
    /// dimension. The member's 120 000 ms is drawn by the adjacent
    /// `qualified_draw_time` in the flow (F-048). The dead tube's trip to
    /// recycling is flow-routed (SPEC.md P5), via [`retire_tube`].
    ///
    /// Money split violation — tendering 1000 for a 650 purchase and
    /// claiming 300 change must not compile (1000 ≠ 650 + 300; an E0080 at
    /// monomorphization, F-001 — note the composed-process echo whose
    /// instantiation note points at the `split_money` call inside, F-051):
    ///
    /// ```compile_fail
    /// use cs2_puncture_repair::characteristics::Induction;
    /// use cs2_puncture_repair::resources::boundary::{new_parts_counter, wallet_with};
    /// use cs2_puncture_repair::resources::processes::{buy_spare, draw_cash};
    /// use model_core::common::boundary::{new_person, qualify};
    ///
    /// let member = qualify::<Induction, 1_800_000>(new_person::<1_800_000>());
    /// let (cash, wallet) = draw_cash::<1000, 0, 1000>(wallet_with::<1000>());
    /// let (member, spare, change, counter) = buy_spare::<1000, 300, 1_800_000, _>(member, new_parts_counter(), cash);
    /// ```
    ///
    /// Satisfies: REQ-014
    pub fn buy_spare<const TENDERED: u64, const CHANGE: u64, const B: u64, V: Req014ListedPriceOnly + Consumer<Money<SPARE_TUBE_PRICE_PENCE>>>(member: Qualified<Induction, B>, counter: V, cash: Money<TENDERED>) -> (Qualified<Induction, B>, SpareTube, Money<CHANGE>, <<V as Consumer<Money<SPARE_TUBE_PRICE_PENCE>>>::Next as Supplier>::Next) where <V as Consumer<Money<SPARE_TUBE_PRICE_PENCE>>>::Next: Supplier<Item = SpareTube> {
        const {
            assert!(
                TENDERED == SPARE_TUBE_PRICE_PENCE + CHANGE,
                "money conservation violated in buy_spare (R1/R19): TENDERED must equal the listed price plus CHANGE - is the stated change wrong for the 650 p price?"
            )
        };
        let (price, change) = split_money::<TENDERED, SPARE_TUBE_PRICE_PENCE, CHANGE>(cash);
        // Pay, then take the goods, through the generic access processes
        // (F-015: the trait-bound path carries the on_unimplemented
        // messages; never call supply/consume on a concrete value).
        let counter = send_to(counter, price);
        let (spare, counter) = take_one(counter);
        (member, spare, change, counter)
    }

    /// Gives up on the twice-failed tube (SPEC.md §3's "dead" state; path 3
    /// only): a conserving state change (R9), no mass change (180 = 180,
    /// structural — both states carry the same fixed mass). The dead tube's
    /// only exit is the [`super::RubberRecycling`] sink, to which the flow
    /// routes it (SPEC.md P5's waste routing).
    pub fn retire_tube(tube: LocatedTube) -> DeadTube {
        // Conserving state change (R9): the tube's 180 g continue unchanged.
        tube.defuse();
        DeadTube::mint()
    }

    /// P6 — refits an airtight tube and inflates (SPEC.md P6). REQ-010
    /// bounds the member (the workstand again) and REQ-012 bounds the tube —
    /// **deliberately multi-type** (review decision 7): the checked patched
    /// tube (183 g) and the new spare (180 g) both satisfy the bound, and
    /// the serviceable wheel's mass is checked **per instantiation**
    /// (`rim+tyre + tube == WHEEL_G`: 1900 + 183 = 2083 on the patched
    /// paths, 1900 + 180 = 2080 with the spare — an E0080 at
    /// monomorphization for any other claim, F-001/F-045). The workstand and
    /// pump are returned (R2); the member's 300 000 ms is drawn by the
    /// adjacent `qualified_draw_time` in the flow (F-048).
    ///
    /// Satisfies: REQ-010, REQ-012
    pub fn refit_and_inflate<const WHEEL_G: u64, O: Req010InductedAtTheWorkstand, T: Req012AirtightTubeOnly>(member: O, workstand: Workstand, pump: Pump, wheel: OpenWheel, tube: T) -> (O, Workstand, Pump, ServiceableWheel<WHEEL_G>) {
        const {
            assert!(
                OpenWheel::MASS_G + T::MASS_G == WHEEL_G,
                "mass conservation violated in refit_and_inflate (R3): the rim + tyre plus the airtight tube must sum exactly to the stated serviceable-wheel mass"
            )
        };
        // Conserving transforms: the open wheel and the tube continue as the
        // serviceable wheel (the tube through its permit-gated extraction,
        // F-054).
        wheel.defuse();
        tube.fit_into_wheel(FitPermit { _seal: () });
        (member, workstand, pump, ServiceableWheel::mint())
    }
}

#[cfg(test)]
mod tests {
    //! Per-process unit tests (R5): every process turns specific inputs into
    //! the expected outputs with nothing left unaccounted for — one test per
    //! arm for the fallible P3 (branch coverage is leak coverage, F-002).
    //! These tests sit inside the privacy boundary, so they may mint fixtures
    //! and defuse outputs directly; downstream-style accounting is exercised
    //! by the integration tests in `tests/flows.rs`.

    use super::boundary::{
        new_parts_counter, new_patch_kit, new_pump, new_rubber_recycling, new_waste_stream,
        new_wheel_owner, new_workstand, patch_will_fail, patch_will_hold, take_wallet_home,
        wallet_with, wheel_arrives_for_service,
    };
    use super::processes::{
        buy_spare, check_patch, deposit, draw_cash, draw_cement, find_hole, open_wheel,
        patch_tube, refit_and_inflate, retire_tube, split_money,
    };
    use super::{
        Cement, CheckedTube, DeadTube, DryPatch, FreshPatchKit, LocatedTube, Money, NoPatches,
        OnePatch, OpenWheel, PatchKit, PatchedTube, PuncturedTube, PuncturedWheel,
        SPARE_TUBE_PRICE_PENCE, ServiceableWheel, SpareTube, Wallet,
    };
    use crate::characteristics::{Airtight, Induction, PunctureLocated};
    use model_core::boundary::{send_to, take_one};
    use model_core::common::boundary::{new_person, qualify};

    /// P1: the wheel splits exactly into rim + tyre and the tube
    /// (SPEC.md P1: 2080 = 1900 + 180), the workstand comes back (R2), and
    /// only the inducted member could do it.
    ///
    /// Verifies: REQ-010
    #[test]
    fn open_wheel_splits_the_wheel_exactly() {
        let member = qualify::<Induction, 1_800_000>(new_person::<1_800_000>());
        let (member, stand, open, tube) =
            open_wheel(member, new_workstand(), wheel_arrives_for_service());
        assert_eq!(
            PuncturedWheel::MASS_G,
            OpenWheel::MASS_G + PuncturedTube::MASS_G
        );
        // Inside the privacy boundary this test may defuse directly.
        open.defuse();
        tube.defuse();
        let _reusables = (member, stand);
    }

    /// P2: a pure state change (SPEC.md P2: 180 = 180, structural), with the
    /// pump returned (R2).
    #[test]
    fn find_hole_is_a_state_change_with_no_mass_change() {
        let member = qualify::<Induction, 1_800_000>(new_person::<1_800_000>());
        let (member, pump, located) = find_hole(member, new_pump(), PuncturedTube::mint());
        assert_eq!(
            PuncturedTube::MASS_G,
            <LocatedTube as PunctureLocated>::MASS_G
        );
        located.defuse();
        let _reusables = (member, pump);
    }

    /// P3 success arm (R17): tube + patch + cement continue as the patched
    /// tube (SPEC.md P3: 180 + 2 + 1 = 183), the member comes back, and the
    /// waste stream is untouched.
    ///
    /// Verifies: REQ-011
    #[test]
    fn patch_tube_success_arm_balances() {
        let member = qualify::<Induction, 1_800_000>(new_person::<1_800_000>());
        let (cement, kit) = draw_cement::<1, 29, 30, _>(new_patch_kit());
        let (patch, kit) = take_one(kit);
        match patch_tube::<1, 183, 3, 1_800_000, _, _>(
            member,
            LocatedTube::mint(),
            patch,
            cement,
            patch_will_hold(),
            new_waste_stream(),
        ) {
            Ok(ok) => {
                assert_eq!(
                    <LocatedTube as PunctureLocated>::MASS_G + DryPatch::MASS_G + 1,
                    PatchedTube::<183>::VALUE
                );
                ok.tube.defuse();
                let _accounted = (ok.member, ok.waste);
            }
            Err(_fail) => panic!("a success token must realise the success arm"),
        }
        let _kit: PatchKit<OnePatch, 29> = kit;
    }

    /// P3 failure arm (R17): the tube comes back still located (ready for
    /// the provisioned retry), the spent patch (SPEC.md P3: 2 + 1 = 3 g) is
    /// fed to the waste stream INSIDE the process (REQ-013 structural), and
    /// the member comes back — failure conserves too.
    ///
    /// Verifies: REQ-011, REQ-013
    #[test]
    fn patch_tube_failure_arm_feeds_the_waste_stream_inside() {
        let member = qualify::<Induction, 1_800_000>(new_person::<1_800_000>());
        let (cement, kit) = draw_cement::<1, 29, 30, _>(new_patch_kit());
        let (patch, kit) = take_one(kit);
        match patch_tube::<1, 183, 3, 1_800_000, _, _>(
            member,
            LocatedTube::mint(),
            patch,
            cement,
            patch_will_fail(),
            new_waste_stream(),
        ) {
            Ok(_ok) => panic!("a failure token must realise the failure arm"),
            Err(fail) => {
                // The tube is back in its located state at its full mass.
                assert_eq!(<LocatedTube as PunctureLocated>::MASS_G, 180);
                fail.tube.defuse();
                let _accounted = (fail.member, fail.waste);
            }
        }
        let _kit: PatchKit<OnePatch, 29> = kit;
    }

    /// P4: a pure state change (SPEC.md P4: 183 = 183, structural) producing
    /// the airtight patched state, with the pump returned (R2).
    #[test]
    fn check_patch_is_a_state_change_with_no_mass_change() {
        let member = qualify::<Induction, 1_800_000>(new_person::<1_800_000>());
        let tube: PatchedTube<183> = PatchedTube::mint();
        let (member, pump, checked) = check_patch(member, new_pump(), tube);
        assert_eq!(<CheckedTube<183> as Airtight>::MASS_G, 183);
        checked.defuse();
        let _reusables = (member, pump);
    }

    /// P5: money splits 1000 = 650 + 350 (SPEC.md P5, assert), the counter
    /// takes exactly its listed price, the spare comes back with the change,
    /// and the change is banked back into the wallet.
    ///
    /// Verifies: REQ-014
    #[test]
    fn buy_spare_splits_the_money_and_pays_the_listed_price() {
        let member = qualify::<Induction, 1_800_000>(new_person::<1_800_000>());
        let (cash, wallet) = draw_cash::<1000, 0, 1000>(wallet_with::<1000>());
        let (member, spare, change, counter) =
            buy_spare::<1000, 350, 1_800_000, _>(member, new_parts_counter(), cash);
        assert_eq!(SPARE_TUBE_PRICE_PENCE + Money::<350>::VALUE, 1000);
        assert_eq!(<SpareTube as Airtight>::MASS_G, 180);
        let wallet = deposit::<350, 0, 350>(wallet, change);
        assert_eq!(Wallet::<350>::VALUE, 350);
        take_wallet_home(wallet);
        spare.defuse();
        let _accounted = (member, counter);
    }

    /// The money dimension conserves through a bare split too (R3/R19).
    #[test]
    fn split_money_conserves_the_amount() {
        let (cash, wallet) = draw_cash::<1000, 0, 1000>(wallet_with::<1000>());
        let (a, b) = split_money::<1000, 650, 350>(cash);
        assert_eq!(Money::<650>::VALUE + Money::<350>::VALUE, 1000);
        a.defuse();
        b.defuse();
        take_wallet_home(wallet);
    }

    /// P6 with the checked **patched** tube (REQ-012's first satisfying
    /// type): 1900 + 183 = 2083 (SPEC.md P6, assert per instantiation), the
    /// workstand and pump back (R2), the wheel to the owner.
    ///
    /// Verifies: REQ-010, REQ-012
    #[test]
    fn refit_with_the_checked_patched_tube_gives_a_2083_g_wheel() {
        let member = qualify::<Induction, 1_800_000>(new_person::<1_800_000>());
        let checked: CheckedTube<183> = CheckedTube::mint();
        let (member, stand, pump, wheel) = refit_and_inflate::<2083, _, _>(
            member,
            new_workstand(),
            new_pump(),
            OpenWheel::mint(),
            checked,
        );
        assert_eq!(ServiceableWheel::<2083>::VALUE, 2083);
        let _owner = send_to(new_wheel_owner(), wheel);
        let _reusables = (member, stand, pump);
    }

    /// P6 with the **new spare** (REQ-012's second satisfying type — the
    /// multi-type requirement, review decision 7): 1900 + 180 = 2080, a
    /// different per-instantiation assert from the patched route.
    ///
    /// Verifies: REQ-012
    #[test]
    fn refit_with_the_new_spare_gives_a_2080_g_wheel() {
        let member = qualify::<Induction, 1_800_000>(new_person::<1_800_000>());
        let (member, stand, pump, wheel) = refit_and_inflate::<2080, _, _>(
            member,
            new_workstand(),
            new_pump(),
            OpenWheel::mint(),
            SpareTube::mint(),
        );
        assert_eq!(ServiceableWheel::<2080>::VALUE, 2080);
        let _owner = send_to(new_wheel_owner(), wheel);
        let _reusables = (member, stand, pump);
    }

    /// The dead tube's state change conserves (180 = 180, structural) and
    /// recycling is its only exit (SPEC.md P5 waste routing).
    #[test]
    fn retire_tube_reaches_rubber_recycling() {
        let dead = retire_tube(LocatedTube::mint());
        assert_eq!(DeadTube::MASS_G, 180);
        let _recycling = send_to(new_rubber_recycling(), dead);
    }

    /// The nested kit (SPEC.md §3): patches leave one at a time through the
    /// `Supplier` side, cement through the draw — and both contents are
    /// facts about the kit's type, so count and remainder cannot drift.
    #[test]
    fn the_kit_supplies_patches_and_draws_cement_through_its_boundary() {
        let kit = new_patch_kit();
        assert_eq!(FreshPatchKit::PATCHES, 2);
        assert_eq!(FreshPatchKit::CEMENT_G, 30);
        let (p1, kit) = take_one(kit);
        let (cement, kit) = draw_cement::<1, 29, 30, _>(kit);
        assert_eq!(Cement::<1>::VALUE, 1);
        assert_eq!(PatchKit::<OnePatch, 29>::PATCHES, 1);
        let (p2, kit) = take_one(kit);
        let (cement2, kit) = draw_cement::<1, 28, 29, _>(kit);
        assert_eq!(PatchKit::<NoPatches, 28>::PATCHES, 0);
        assert_eq!(PatchKit::<NoPatches, 28>::CEMENT_G, 28);
        // Inside the privacy boundary the test accounts directly.
        cement.defuse();
        cement2.defuse();
        let _patches_kept_to_test_end = (p1, p2);
        let _kit_stays_with_the_member = kit;
    }

    /// An abandoned dead tube is caught by its tripwire at test time (R1
    /// layer 2, F-008): no compile-time layer sees a named-and-used binding
    /// that never reaches recycling.
    #[test]
    #[should_panic(expected = "resource leak: DeadTube dropped without being consumed")]
    fn abandoned_dead_tube_trips_the_tripwire() {
        let dead = retire_tube(LocatedTube::mint());
        let _never_reaches_recycling = dead;
    }
}
