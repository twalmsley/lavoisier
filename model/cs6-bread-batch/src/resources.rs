//! The kitchen's sealed resource family (R1), its creation/exit boundary
//! (R12) and its processes P1..P6 (SPEC.md §3, §4, §5). Layout per
//! F-006/F-031: the sealed types here, with the [`boundary`] and the
//! [`processes`] (which mint quantity-bearing values and therefore live
//! inside the privacy boundary) as child modules.
//!
//! Started from the specgen scaffold of SPEC.md (R22) and hand-maintained
//! since; the `Generated from SPEC.md` breadcrumbs below are kept where they
//! remain true. The scaffold's SPEC-HOLE U-07 (literal magnitudes vs generic
//! const parameters) is resolved by **keeping the spec's literal numbers** in
//! the process signatures — the house style for a single worked instance —
//! generalizing only where a requirement bound forces it (`divide_and_shape`
//! over the proved dough, `bake` over the greased tin, each recovering its
//! mass through the characteristic's `MASS_G`, R6/R7).
//!
//! One type per processing state (R9, F-023), straight from SPEC.md §3:
//!
//! * dough: [`MixedDough`] (1 682 g) → [`KneadedDough`] → [`ProvedDough`]
//!   (the `Proved` state REQ-028 gates);
//! * loaf: [`ShapedLoaf`] (841 g) → [`BakedLoaf`] **or** [`ScorchedLoaf`]
//!   (744 g each, carrying 300 000 J of embodied energy — the multi-quantity
//!   states, both constants in the type);
//! * tin: [`CleanTin`] (450 g) → [`GreasedTin`] (456 g, the `Greased` state
//!   REQ-029 gates) → [`UsedTin`] (453 g, butter residue);
//! * yeast: [`FreshSachet`] (8 g, kept by the box, F-040) → [`SpentSachet`]
//!   (1 g), with the [`YeastBox`] supplier exhausted to [`EmptyYeastBox`]
//!   and its 30 g shell continuing as the [`EmptyBox`] output (SPEC.md §3).
//!
//! Every consumable here carries the kernel's tripwire `Drop` (R1 layer 2,
//! F-008) except the [`FreshSachet`], which is *kept* by the box that
//! accounts for it (`no_tripwire`, F-040). Reusable resources (the
//! [`Oven`], the baker — model-core's `Person` with the SPEC's 7 200 000 ms
//! budget) are moved in and returned by every process (R2) and stay with the
//! caller.

use crate::characteristics::{
    BakedOnlyHousehold, Greased, Proved, ScorchedLoafCompost, sealed,
};
// One line on purpose: the traceability grep (F-021) skips `use` lines, but
// only when the line itself starts with `use`.
use crate::requirements::{assert_req028, assert_req029, assert_req030, assert_req031};
use model_core::boundary::{Consumer, Supplier};
use model_core::common::Person;
use model_core::list::{Cons, Len, Nil};

model_core::consumable_resource! {
    /// Generated from SPEC.md §3 line 47: resource ''loaf'', state 'shaped'.
    /// Half the proved dough, seated in nothing yet (841 g): only
    /// [`processes::bake`] — with a greased tin, REQ-029 — moves it on.
    /// Tripwired (F-008): a shaped loaf the flow never bakes fails the test
    /// that leaked it.
    ShapedLoaf,
    must_use = "ShapedLoaf is a conserved resource: pass it on or hand it to a Consumer"
}

impl ShapedLoaf {
    /// The shaped loaf's mass, in grams (R7; SPEC.md §3).
    pub const MASS_G: u64 = 841;
}

model_core::consumable_resource! {
    /// Generated from SPEC.md §3 line 47: resource ''loaf'', state 'baked'.
    /// The product: a multi-quantity state (SPEC.md §1) carrying mass *and*
    /// embodied energy in the type's constants. Its exit is the
    /// [`Household`] (REQ-030). Tripwired (F-008).
    BakedLoaf,
    must_use = "BakedLoaf is a conserved resource: pass it on or hand it to a Consumer"
}

impl BakedLoaf {
    /// The baked loaf's mass, in grams (R7; SPEC.md §3: 744 g — 841 g shaped
    /// − 100 g steam + 3 g of crust butter from the tin).
    pub const MASS_G: u64 = 744;
    /// The baked loaf's embodied energy, in joules (R7; SPEC.md §3): it
    /// crosses the boundary with the loaf (cooling is out of scope, §1).
    pub const EMBODIED_J: u64 = 300_000;
}

model_core::consumable_resource! {
    /// Generated from SPEC.md §3 line 47: resource ''loaf'', state 'scorched'.
    /// The failure state (R17): same mass and embodied energy as the baked
    /// state, but **final** — the dough is consumed, no rework exists
    /// (SPEC.md §5 P6), and its only exit is the [`CompostStream`]
    /// (REQ-031). Tripwired (F-008): a scorched loaf that never reaches
    /// compost fails the test that leaked it.
    ScorchedLoaf,
    must_use = "ScorchedLoaf is a conserved waste product: it must reach the compost stream (REQ-031)"
}

impl ScorchedLoaf {
    /// The scorched loaf's mass, in grams (R7; SPEC.md §3).
    pub const MASS_G: u64 = 744;
    /// The scorched loaf's embodied energy, in joules (R7; SPEC.md §3).
    pub const EMBODIED_J: u64 = 300_000;
}

model_core::consumable_resource! {
    /// Generated from SPEC.md §3 line 48: resource ''dough'', state 'mixed'.
    /// The whole 1 682 g batch straight out of P1. Tripwired (F-008).
    MixedDough,
    must_use = "MixedDough is a conserved resource: pass it on or hand it to a Consumer"
}

impl MixedDough {
    /// The mixed dough's mass, in grams (R7; SPEC.md §3).
    pub const MASS_G: u64 = 1_682;
}

model_core::consumable_resource! {
    /// Generated from SPEC.md §3 line 48: resource ''dough'', state 'kneaded'.
    /// A state change only (SPEC.md §5 P2): the mass is unchanged. It cannot
    /// be shaped — only the proved state satisfies REQ-028. Tripwired
    /// (F-008).
    KneadedDough,
    must_use = "KneadedDough is a conserved resource: pass it on or hand it to a Consumer"
}

impl KneadedDough {
    /// The kneaded dough's mass, in grams (R7; SPEC.md §3).
    pub const MASS_G: u64 = 1_682;
}

model_core::consumable_resource! {
    /// Generated from SPEC.md §3 line 48: resource ''dough'', state 'proved'.
    /// The only dough state `divide_and_shape` accepts (REQ-028). Tripwired
    /// (F-008): proved dough that is never shaped fails the test that leaked
    /// it.
    ProvedDough,
    must_use = "ProvedDough is a conserved resource: pass it on or hand it to a Consumer"
}

/// The proved characteristic (R6) lives on the proved state only (R9/F-023):
/// `into_loaves` is the permit-gated conserving extraction that only
/// [`processes::divide_and_shape`] can call — the dough's mass continues into
/// the two shaped loaves, checked by that process's conservation assert
/// (F-054).
impl Proved for ProvedDough {
    const MASS_G: u64 = 1_682;
    fn into_loaves(self, _permit: ShapePermit) {
        // Conserving transform: the dough's 1 682 g continue inside the two
        // shaped loaves minted by divide_and_shape (R1).
        self.defuse();
    }
}
impl sealed::Sealed for ProvedDough {}

/// The proved dough under its requirement-facing name; the alias carries the
/// tag because a tag inside the `consumable_resource!` invocation above would
/// be silently dropped by trace.sh (F-037).
///
/// Satisfies: REQ-028
pub type DoughReadyToShape = ProvedDough;
model_core::satisfies!(assert_req028, DoughReadyToShape);

model_core::container_resource! {
    /// Generated from SPEC.md §3 line 49: resource ''flour''.
    Flour,
    unit = "grams",
    must_use = "Flour is a conserved resource: pass it on or hand it to a Consumer"
}

model_core::container_resource! {
    /// Generated from SPEC.md §3 line 50: resource ''water''.
    Water,
    unit = "grams",
    must_use = "Water is a conserved resource: pass it on or hand it to a Consumer"
}

model_core::container_resource! {
    /// Generated from SPEC.md §3 line 51: resource ''salt''.
    Salt,
    unit = "grams",
    must_use = "Salt is a conserved resource: pass it on or hand it to a Consumer"
}

model_core::container_resource! {
    /// Generated from SPEC.md §3 line 52: resource ''butter''.
    Butter,
    unit = "grams",
    must_use = "Butter is a conserved resource: pass it on or hand it to a Consumer"
}

model_core::consumable_resource! {
    /// Generated from SPEC.md §3 line 53: resource ''yeastsachet' (yeast sachet)', state 'fresh'.
    /// `no_tripwire`: minted into a boundary container that keeps it (the F-040 rule).
    FreshSachet,
    must_use = "FreshSachet is a conserved resource: pass it on or hand it to a Consumer",
    no_tripwire
}

impl FreshSachet {
    /// A fresh sachet's mass, in grams (R7; SPEC.md §3: 7 g yeast + 1 g
    /// wrapper).
    pub const MASS_G: u64 = 8;
}

model_core::consumable_resource! {
    /// Generated from SPEC.md §3 line 53: resource ''yeastsachet' (yeast sachet)', state 'spent'.
    /// The 1 g wrapper after P1 empties the yeast into the dough; its exit is
    /// [`Recycling`] (SPEC.md §4). Tripwired (F-008).
    SpentSachet,
    must_use = "SpentSachet is a conserved waste product: it must reach recycling"
}

impl SpentSachet {
    /// A spent sachet's mass, in grams (R7; SPEC.md §3: the wrapper).
    pub const MASS_G: u64 = 1;
}

model_core::consumable_resource! {
    /// Generated from SPEC.md §3 line 54: resource ''yeastbox' (yeast box)', state 'empty'.
    /// The §3 empty state of the yeast box (30 g): minted by
    /// [`processes::mix_dough`] as the exhausted supplier's shell continues
    /// (SPEC.md §5 P1); its exit is [`Recycling`] (SPEC.md §4). Tripwired
    /// (F-008).
    EmptyBox,
    must_use = "EmptyBox is a conserved waste product: it must reach recycling"
}

impl EmptyBox {
    /// The empty box's mass, in grams (R7; SPEC.md §3).
    pub const MASS_G: u64 = 30;
}

model_core::container_resource! {
    /// Generated from SPEC.md §3 line 55: resource ''loaftin' (loaf tin)', state 'clean'.
    CleanTin,
    unit = "grams",
    must_use = "CleanTin is a conserved resource: pass it on or hand it to a Consumer"
}

model_core::container_resource! {
    /// Generated from SPEC.md §3 line 55: resource ''loaftin' (loaf tin)', state 'greased'.
    /// `V` grams in all (456 at the SPEC quantities: 450 g tin + 6 g butter).
    /// The `Greased` state REQ-029 gates: the only tin state
    /// [`processes::bake`] accepts. Tripwired (F-008).
    GreasedTin,
    unit = "grams",
    must_use = "GreasedTin is a conserved resource: pass it on or hand it to a Consumer"
}

/// The greased characteristic (R6) on the greased state (REQ-029's
/// satisfying type): `into_oven` is the permit-gated conserving extraction
/// only [`processes::bake`] can call — the tin's mass continues into the
/// loaf's crust and the used tin, checked by that process's per-arm mass
/// asserts (F-054).
impl<const V: u64> Greased for GreasedTin<V> {
    const MASS_G: u64 = V;
    fn into_oven(self, _permit: BakePermit) {
        // Conserving transform: the tin's mass continues inside the used tin
        // and the loaf's crust minted by bake (R1).
        self.defuse();
    }
}
impl<const V: u64> sealed::Sealed for GreasedTin<V> {}

/// The greased tin at the CS-6 quantities, under its requirement-facing name
/// (REQ-029's satisfying type). The alias carries the tag because a tag
/// inside the macro invocation would be silently dropped by trace.sh (F-037).
///
/// Satisfies: REQ-029
pub type TinReadyToBake = GreasedTin<456>;
model_core::satisfies!(assert_req029, TinReadyToBake);

model_core::container_resource! {
    /// Generated from SPEC.md §3 line 55: resource ''loaftin' (loaf tin)', state 'used'.
    UsedTin,
    unit = "grams",
    must_use = "UsedTin is a conserved resource: pass it on or hand it to a Consumer"
}

model_core::reusable_resource! {
    /// Generated from SPEC.md §3 line 56: resource ''oven''.
    Oven,
    must_use = "Oven is a reusable resource: pass it on or return it to the caller"
}

model_core::container_resource! {
    /// Generated from SPEC.md §3 line 58: resource ''gridenergy' (oven energy)'.
    GridEnergy,
    unit = "joules",
    must_use = "GridEnergy is a conserved resource: pass it on or hand it to a Consumer"
}

model_core::container_resource! {
    /// Generated from SPEC.md §3 line 59: resource ''steam''.
    Steam,
    unit = "grams",
    must_use = "Steam is a conserved resource: pass it on or hand it to a Consumer"
}

model_core::container_resource! {
    /// Generated from SPEC.md §3 line 60: resource ''wasteheat' (waste heat)'.
    WasteHeat,
    unit = "joules",
    must_use = "WasteHeat is a conserved resource: pass it on or hand it to a Consumer"
}

model_core::outcome_token! {
    /// Generated from SPEC.md §3 line 61: resource ''bakeoutcome' (bake outcome)' — one trial of the
    /// environment (R17, F-042): sealed, injected only at the boundary (the
    /// constructors below stay placeholders until calibrated), realised by
    /// exactly one fallible process. A flow cannot read it: the only way to
    /// learn the outcome is to run the process and handle both `Result` arms.
    BakeOutcome(BakeOutcomeKind),
    success = bake_outcome_will_succeed,
    failure = bake_outcome_will_fail,
    exit = return_bake_outcome,
    must_use = "BakeOutcome is a boundary token: run it through exactly one fallible process or return it to the environment"
}

model_core::reusable_resource! {
    /// Generated from SPEC.md §4 line 70: boundary source drawn via [`boundary::draw_water`].
    ///
    /// Placeholder: unbounded boundary source (R15).
    Tap,
    must_use = "Tap is a boundary resource: pass it on like any other resource"
}

model_core::reusable_resource! {
    /// Generated from SPEC.md §4 line 74: boundary source drawn via [`boundary::draw_grid_energy`].
    ///
    /// Placeholder: unbounded boundary source (R15).
    Grid,
    must_use = "Grid is a boundary resource: pass it on like any other resource"
}

model_core::container_resource! {
    /// Generated from SPEC.md §4 line 69: a bounded source of Flour with
    /// remainder — enters full (capacity 1_500), drawn down via [`boundary::draw_flour`],
    /// and its remainder is accounted at flow end (SPEC.md §6).
    PantryBag,
    unit = "grams remaining",
    must_use = "PantryBag is a conserved resource: even an exhausted container must be accounted for"
}

model_core::container_resource! {
    /// Generated from SPEC.md §4 line 71: a bounded source of Salt with
    /// remainder — enters full (capacity 500), drawn down via [`boundary::draw_salt`],
    /// and its remainder is accounted at flow end (SPEC.md §6).
    PantryJar,
    unit = "grams remaining",
    must_use = "PantryJar is a conserved resource: even an exhausted container must be accounted for"
}

model_core::container_resource! {
    /// Generated from SPEC.md §4 line 72: a bounded source of Butter with
    /// remainder — enters full (capacity 250), drawn down via [`boundary::draw_butter`],
    /// and its remainder is accounted at flow end (SPEC.md §6).
    PantryBlock,
    unit = "grams remaining",
    must_use = "PantryBlock is a conserved resource: even an exhausted container must be accounted for"
}

/// The permit gating [`Proved::into_loaves`] (the F-054 permit-gated
/// extraction pattern): a private field and no public constructor, so only
/// [`processes::divide_and_shape`] (inside this privacy boundary) can call
/// the extraction — it can never be used to vanish dough outside the process
/// whose assert accounts for it (R1).
pub struct ShapePermit {
    _seal: (),
}

/// The permit gating [`Greased::into_oven`] (F-054): only
/// [`processes::bake`] can call the extraction.
pub struct BakePermit {
    _seal: (),
}

/// Everything P6's Ok arm produces (R17, SPEC.md §5 'Produces (Ok)',
/// line 150): the arm's products plus every conserved participant — the
/// person returns at the same budget in both arms (the time draw is adjacent,
/// F-048), reusables return in both arms (R2), and the atmosphere comes back
/// having taken the bake's steam and waste heat **inside the process**
/// (consumer-parameter waste routing, SPEC.md §5 P6: the waste never exists
/// loose).
///
/// A grouping (R1), not a sealed resource (F-046): public fields the handling
/// arm destructures, buildable only from already-held resources. Deliberately
/// **no `Debug`** (F-047): `.unwrap()`/`.expect()` on the process `Result` do
/// not compile — the flow must `match` both arms.
#[must_use = "BakeOk bundles conserved outputs: every field must be accounted for"]
pub struct BakeOk<const B: u64, A> {
    /// The baker, back unchanged (the time draw is adjacent, R17/F-048).
    pub person: Person<B>,
    /// The baked loaf (SPEC.md §5 P6: 744 g, 300 000 J) — the product, bound
    /// for the household (REQ-030).
    pub baked_loaf: BakedLoaf,
    /// The used tin (453 g: its grease split 50/50 with the crust, SPEC.md
    /// §8 item 4).
    pub used_tin: UsedTin<453>,
    /// The oven, back (R2): only its return lets the second bake start.
    pub oven: Oven,
    /// The atmosphere after taking the bake's 100 g of steam and 2 200 000 J
    /// of waste heat inside the process (SPEC.md §5 P6 waste routing).
    pub atmosphere: A,
}

/// Everything P6's Fail arm produces (R17, SPEC.md §5 'Produces (Fail)',
/// line 151). **Failure conserves too**: the person and the oven come back
/// at the same budget as the Ok arm (the time draw is adjacent, F-048), the
/// tin is just as used, the same steam and waste heat reached the atmosphere
/// inside the process — only the loaf differs, and it is **final**: the
/// dough is consumed, so no rework exists (SPEC.md §5 P6) and the scorched
/// loaf is a Fail-arm product the flow routes to the compost stream
/// (REQ-031).
///
/// A grouping (R1), not a sealed resource (F-046): public fields the handling
/// arm destructures, buildable only from already-held resources. Deliberately
/// **no `Debug`** (F-047): `.unwrap()`/`.expect()` on the process `Result` do
/// not compile — the flow must `match` both arms.
#[must_use = "BakeFail bundles conserved outputs: every field must be accounted for"]
pub struct BakeFail<const B: u64, A> {
    /// The baker, back unchanged (the time was drawn before the branch —
    /// failure cost it too, R17).
    pub person: Person<B>,
    /// The scorched loaf (744 g, 300 000 J — same split as the Ok arm):
    /// final, its only exit the compost stream (REQ-031).
    pub scorched_loaf: ScorchedLoaf,
    /// The used tin (453 g) — the tin survives a scorched bake.
    pub used_tin: UsedTin<453>,
    /// The oven, back (R2): a scorch does not break the oven.
    pub oven: Oven,
    /// The atmosphere after taking the bake's steam and waste heat inside
    /// the process — a scorched bake vents exactly like a good one.
    pub atmosphere: A,
}

/// Generated from SPEC.md §4 line 73: a supplier of FreshSachet at the system
/// boundary (R12), capacity 2. Contents are a type-level list of real
/// values; the count is the list's length.
#[must_use = "YeastBox is a boundary resource: pass it on like any other resource"]
pub struct YeastBox<Items>(Items);

/// The exhausted supplier: a distinct resource that must itself be accounted for (R12).
pub type EmptyYeastBox = YeastBox<Nil>;

/// A type-level list of exactly two fresh sachets — the box as it enters the
/// model (SPEC.md §3: the box holds 2; P1 takes both, exhausting it).
pub type TwoSachets = Cons<FreshSachet, Cons<FreshSachet, Nil>>;

/// The yeast box as it enters the model: 2 real fresh sachets (SPEC.md §3).
pub type FullYeastBox = YeastBox<TwoSachets>;

/// `Supplier` only for a non-empty list (R12): supplying from an empty
/// YeastBox is a compile error with model-core's `on_unimplemented` message.
impl<H, T> Supplier for YeastBox<Cons<H, T>> {
    type Item = H;
    type Next = YeastBox<T>;
    fn supply(self) -> (H, YeastBox<T>) {
        let Cons(head, tail) = self.0;
        (head, YeastBox(tail))
    }
}

impl<Items: Len> YeastBox<Items> {
    /// How many items the supplier holds — the length of its contents list (R7, R12).
    pub const COUNT: u64 = Items::LEN;
}

/// Generated from SPEC.md §4 line 82: the household taking delivery of the
/// finished loaves — an unbounded boundary sink (`Next = Self`, R15, F-029 —
/// legal only at the system boundary; it necessarily discards what it
/// consumes). Its `Consumer` impl exists for the **baked** state alone
/// (REQ-030 structural): handing it a scorched loaf is a type-check-time
/// error.
///
/// Placeholder: the household — assumed able to take any number of baked
/// loaves.
///
/// Satisfies: REQ-030
#[must_use = "Household is a boundary resource: pass it on like any other resource"]
pub struct Household {
    _seal: (),
}

// Compile-checked backing for the tag above (R10, F-020).
model_core::satisfies!(assert_req030, Household);

impl BakedOnlyHousehold for Household {}

/// The sink accepts baked loaves only; `Next = Self` (R15; REQ-030).
impl Consumer<BakedLoaf> for Household {
    type Next = Household;
    fn consume(self, item: BakedLoaf) -> Household {
        item.defuse(); // sanctioned consumer role (F-008); unbounded sink discards (F-029)
        self
    }
}

/// Generated from SPEC.md §4 line 83: the compost stream — an unbounded
/// boundary sink (`Next = Self`, R15, F-029 — legal only at the system
/// boundary; it necessarily discards what it consumes). It is the **only**
/// consumer of the scorched state in the model (REQ-031 structural): a
/// scorched loaf's sole exit is this stream, and the loaf's tripwire catches
/// one that never arrives.
///
/// Placeholder: the compost stream — assumed able to take any number of
/// scorched loaves.
///
/// Satisfies: REQ-031
#[must_use = "CompostStream is a boundary resource: pass it on like any other resource"]
pub struct CompostStream {
    _seal: (),
}

// Compile-checked backing for the tag above (R10, F-020).
model_core::satisfies!(assert_req031, CompostStream);

impl ScorchedLoafCompost for CompostStream {}

/// The sink accepts scorched loaves — their only consumer (REQ-031);
/// `Next = Self` (R15).
impl Consumer<ScorchedLoaf> for CompostStream {
    type Next = CompostStream;
    fn consume(self, item: ScorchedLoaf) -> CompostStream {
        item.defuse(); // sanctioned consumer role (F-008); unbounded sink discards (F-029)
        self
    }
}

/// Generated from SPEC.md §4 line 84: an unbounded boundary sink
/// (`Next = Self`, R15, F-029 — legal only at the system boundary; it
/// necessarily discards what it consumes). [`processes::bake`] takes it as a
/// bounded consumer parameter and feeds it the steam and waste heat
/// **inside the process** (SPEC.md §5 P6 waste routing), so neither waste
/// ever exists loose.
///
/// Placeholder: the atmosphere — assumed an unbounded sink for steam and
/// waste heat.
#[must_use = "Atmosphere is a boundary resource: pass it on like any other resource"]
pub struct Atmosphere {
    _seal: (),
}

/// The sink accepts Steam at any magnitude; `Next = Self` (R15).
impl<const C0: u64> Consumer<Steam<C0>> for Atmosphere {
    type Next = Atmosphere;
    fn consume(self, item: Steam<C0>) -> Atmosphere {
        item.defuse(); // sanctioned consumer role (F-008); unbounded sink discards (F-029)
        self
    }
}

/// The sink accepts WasteHeat at any magnitude; `Next = Self` (R15).
impl<const C0: u64> Consumer<WasteHeat<C0>> for Atmosphere {
    type Next = Atmosphere;
    fn consume(self, item: WasteHeat<C0>) -> Atmosphere {
        item.defuse(); // sanctioned consumer role (F-008); unbounded sink discards (F-029)
        self
    }
}

/// Generated from SPEC.md §4 line 86: an unbounded boundary sink
/// (`Next = Self`, R15, F-029 — legal only at the system boundary; it
/// necessarily discards what it consumes). The flow routes P1's loose waste
/// here: the two spent sachets and the empty yeast box (SPEC.md §5 P1 waste
/// routing).
///
/// Placeholder: recycling — assumed able to take any number of wrappers and
/// boxes.
#[must_use = "Recycling is a boundary resource: pass it on like any other resource"]
pub struct Recycling {
    _seal: (),
}

/// The sink accepts SpentSachet at any magnitude; `Next = Self` (R15).
impl Consumer<SpentSachet> for Recycling {
    type Next = Recycling;
    fn consume(self, item: SpentSachet) -> Recycling {
        item.defuse(); // sanctioned consumer role (F-008); unbounded sink discards (F-029)
        self
    }
}

/// The sink accepts EmptyBox at any magnitude; `Next = Self` (R15).
impl Consumer<EmptyBox> for Recycling {
    type Next = Recycling;
    fn consume(self, item: EmptyBox) -> Recycling {
        item.defuse(); // sanctioned consumer role (F-008); unbounded sink discards (F-029)
        self
    }
}

/// The creation boundary (R12): the only production code where boundary
/// objects and supplied contents come into existence (SPEC.md §4).
pub mod boundary {
    use super::{Atmosphere, CleanTin, CompostStream, FreshSachet, Grid, GridEnergy, Household, Oven, PantryBag, PantryBlock, PantryJar, Recycling, Tap, UsedTin, Water, YeastBox};
    use model_core::list::{Cons, Nil};
    use model_core::nat::{Succ, Zero};

    /// The outcome tokens' boundary constructors and exits (R17), generated by
    /// `model_core::outcome_token!` at the family level and re-exported here so the
    /// creation boundary stays the single entry point (R12).
    pub use super::{bake_outcome_will_fail, bake_outcome_will_succeed, return_bake_outcome};

    /// Oven enters the model at flow start (R12).
    ///
    /// Placeholder: setup at flow start (SPEC.md §4).
    pub fn new_oven() -> Oven {
        Oven::mint()
    }

    /// CleanTin enters the model at flow start, at its §3-declared magnitudes
    /// (R12; SPEC.md §4 line 75).
    ///
    /// Placeholder: setup at flow start (SPEC.md §4).
    pub fn new_clean_tin() -> CleanTin<450> {
        CleanTin::mint()
    }

    /// Tap enters the model (R12).
    ///
    /// Placeholder: unbounded boundary source (SPEC.md §4 line 70).
    pub fn new_tap() -> Tap {
        Tap::mint()
    }

    /// Draws `TAKE` of Water from the boundary (R15: an unbounded source of
    /// continuous material is a draw-style boundary process, never a `Supplier`
    /// impl, F-028). The source object is returned (R2).
    ///
    /// Placeholder: unbounded boundary source (SPEC.md §4 line 70).
    pub fn draw_water<const TAKE: u64>(source: Tap) -> (Water<TAKE>, Tap) {
        (Water::mint(), source)
    }

    /// Grid enters the model (R12).
    ///
    /// Placeholder: unbounded boundary source (SPEC.md §4 line 74).
    pub fn new_grid() -> Grid {
        Grid::mint()
    }

    /// Draws `TAKE` of GridEnergy from the boundary (R15: an unbounded source of
    /// continuous material is a draw-style boundary process, never a `Supplier`
    /// impl, F-028). The source object is returned (R2).
    ///
    /// Placeholder: unbounded boundary source (SPEC.md §4 line 74).
    pub fn draw_grid_energy<const TAKE: u64>(source: Grid) -> (GridEnergy<TAKE>, Grid) {
        (GridEnergy::mint(), source)
    }

    /// The PantryBag enters the model full (R12): a bounded continuous source with
    /// remainder, capacity 1_500 (SPEC.md §4 line 69).
    ///
    /// Placeholder: vendor/stock not modelled (SPEC.md §4).
    pub fn new_pantry_bag() -> PantryBag<1_500> {
        PantryBag::mint()
    }

    /// The PantryJar enters the model full (R12): a bounded continuous source with
    /// remainder, capacity 500 (SPEC.md §4 line 71).
    ///
    /// Placeholder: vendor/stock not modelled (SPEC.md §4).
    pub fn new_pantry_jar() -> PantryJar<500> {
        PantryJar::mint()
    }

    /// The PantryBlock enters the model full (R12): a bounded continuous source with
    /// remainder, capacity 250 (SPEC.md §4 line 72).
    ///
    /// Placeholder: vendor/stock not modelled (SPEC.md §4).
    pub fn new_pantry_block() -> PantryBlock<250> {
        PantryBlock::mint()
    }

    /// Maps a type-level count to the list type of that many FreshSachets.
    pub trait Replicate {
        /// The list type holding `Self`-many items.
        type List;
    }
    impl Replicate for Zero {
        type List = Nil;
    }
    impl<N: Replicate> Replicate for Succ<N> {
        type List = Cons<FreshSachet, N::List>;
    }

    /// Shorthand for [`Replicate::List`].
    pub type ItemsOf<N> = <N as Replicate>::List;

    /// Builds a list of real items. Private: the only place FreshSachets come
    /// into existence (R1), reachable only through [`full_yeast_box`].
    trait Fill {
        fn fill() -> Self;
    }
    impl Fill for Nil {
        fn fill() -> Nil {
            Nil
        }
    }
    impl<T: Fill> Fill for Cons<FreshSachet, T> {
        fn fill() -> Self {
            Cons(FreshSachet::mint(), T::fill())
        }
    }

    /// Public-in-signature but unimplementable-outside (sealed-trait pattern, F-026).
    pub trait FillSealed: sealed::Sealed {
        #[doc(hidden)]
        fn fill_sealed() -> Self;
    }
    impl<L: Fill + sealed::Sealed> FillSealed for L {
        fn fill_sealed() -> Self {
            L::fill()
        }
    }
    mod sealed {
        use super::super::FreshSachet;
        use model_core::list::{Cons, Nil};
        pub trait Sealed {}
        impl Sealed for Nil {}
        impl<T: Sealed> Sealed for Cons<FreshSachet, T> {}
    }

    /// The fill function (R12): a full supplier of `N` real items enters the
    /// model here (SPEC.md §4 line 73: capacity 2).
    ///
    /// Placeholder: vendor not modelled (SPEC.md §4).
    pub fn full_yeast_box<N: Replicate>() -> YeastBox<ItemsOf<N>>
    where
        ItemsOf<N>: FillSealed,
    {
        YeastBox(FillSealed::fill_sealed())
    }

    /// Household enters the model (R12). An empty unbounded sink holds nothing.
    pub fn new_household() -> Household {
        Household { _seal: () }
    }

    /// CompostStream enters the model (R12). An empty unbounded sink holds nothing.
    pub fn new_compost_stream() -> CompostStream {
        CompostStream { _seal: () }
    }

    /// Atmosphere enters the model (R12). An empty unbounded sink holds nothing.
    pub fn new_atmosphere() -> Atmosphere {
        Atmosphere { _seal: () }
    }

    /// Recycling enters the model (R12). An empty unbounded sink holds nothing.
    pub fn new_recycling() -> Recycling {
        Recycling { _seal: () }
    }

    /// Flow-end rest for PantryBag (R12 exit): the §6 account leaves it resting at
    /// the boundary ("everything accounted"), and an integration test cannot hold a
    /// tripwired resource past its end — the accounted counterpart of a constructor
    /// (the CS-2 `take_wallet_home` shape).
    pub fn rest_pantry_bag<const C0: u64>(resting: PantryBag<C0>) {
        resting.defuse(); // the sanctioned boundary exit (R12, F-008)
    }

    /// Flow-end rest for PantryBlock (R12 exit): the §6 account leaves it resting at
    /// the boundary ("everything accounted"), and an integration test cannot hold a
    /// tripwired resource past its end — the accounted counterpart of a constructor
    /// (the CS-2 `take_wallet_home` shape).
    pub fn rest_pantry_block<const C0: u64>(resting: PantryBlock<C0>) {
        resting.defuse(); // the sanctioned boundary exit (R12, F-008)
    }

    /// Flow-end rest for PantryJar (R12 exit): the §6 account leaves it resting at
    /// the boundary ("everything accounted"), and an integration test cannot hold a
    /// tripwired resource past its end — the accounted counterpart of a constructor
    /// (the CS-2 `take_wallet_home` shape).
    pub fn rest_pantry_jar<const C0: u64>(resting: PantryJar<C0>) {
        resting.defuse(); // the sanctioned boundary exit (R12, F-008)
    }

    /// Flow-end rest for UsedTin (R12 exit): the §6 account leaves it resting at
    /// the boundary ("everything accounted"), and an integration test cannot hold a
    /// tripwired resource past its end — the accounted counterpart of a constructor
    /// (the CS-2 `take_wallet_home` shape).
    pub fn rest_used_tin<const C0: u64>(resting: UsedTin<C0>) {
        resting.defuse(); // the sanctioned boundary exit (R12, F-008)
    }

}

/// The kitchen's processes P1..P6 (SPEC.md §5): pure by-value conserving
/// transformations (R1, R2). They mint quantity-bearing values, so they live
/// inside the resource family's module (F-031).
///
/// The baker is threaded loosely through every process (R9, F-024) — one
/// actor, so the only ordering freedom is when the tins are greased
/// (SPEC.md §6). Per F-048 the baker's time is drawn by **adjacent**
/// `draw_time` processes in the flow (never inside these processes), each
/// draw's labour recorded into the single `History` attributed to the
/// process name (R16); P6's draw happens *before* the fallible branch, so a
/// scorched bake costs the same time as a good one (R17).
pub mod processes {
    use super::{BakeFail, BakeOk, BakeOutcome, BakeOutcomeKind, BakePermit, BakedLoaf, Butter, CleanTin, EmptyBox, EmptyYeastBox, Flour, FreshSachet, GreasedTin, GridEnergy, KneadedDough, MixedDough, Oven, PantryBag, PantryBlock, PantryJar, ProvedDough, Salt, ScorchedLoaf, ShapePermit, ShapedLoaf, SpentSachet, Steam, UsedTin, WasteHeat, Water, YeastBox};
    // One line on purpose: the traceability grep (F-021) skips `use` lines,
    // but only when the line itself starts with `use`.
    use crate::requirements::{Req028ProvedBeforeShaping, Req029GreasedTinInTheOven};
    use model_core::boundary::{Consumer, SupplyN, send_to};
    use model_core::common::Person;
    use model_core::list::{Cons, Nil};
    use model_core::nat::aliases::N2;

    // -- Pantry draws (R15; caller-stated remainder, F-022/F-030) ----------

    model_core::draw_process! {
        /// Draws `TAKE` of Flour from the PantryBag (R15), leaving `LEFT` of `FULL`
        /// (SPEC.md §4 line 69); the remainder is caller-stated and checked at
        /// compile time (F-022, F-030).
        pub fn draw_flour: PantryBag => Flour,
        assert = "conservation violated in draw_flour (R15): TAKE + LEFT must equal FULL — is the draw larger than the container's remaining contents?"
    }

    model_core::draw_process! {
        /// Draws `TAKE` of Salt from the PantryJar (R15), leaving `LEFT` of `FULL`
        /// (SPEC.md §4 line 71); the remainder is caller-stated and checked at
        /// compile time (F-022, F-030).
        pub fn draw_salt: PantryJar => Salt,
        assert = "conservation violated in draw_salt (R15): TAKE + LEFT must equal FULL — is the draw larger than the container's remaining contents?"
    }

    model_core::draw_process! {
        /// Draws `TAKE` of Butter from the PantryBlock (R15), leaving `LEFT` of `FULL`
        /// (SPEC.md §4 line 72); the remainder is caller-stated and checked at
        /// compile time (F-022, F-030).
        pub fn draw_butter: PantryBlock => Butter,
        assert = "conservation violated in draw_butter (R15): TAKE + LEFT must equal FULL — is the draw larger than the container's remaining contents?"
    }

    /// P1 — Mix the dough (SPEC.md §5 P1, line 95): the drawn flour, water
    /// and salt plus both sachets' yeast become the 1_682 g mixed batch, and
    /// taking both sachets exhausts the box.
    /// The person's 900_000 ms is drawn by the **adjacent** `draw_time` in the flow,
    /// recorded to the History under this process's name (F-048 — the template's
    /// stated convention); the process takes and returns the person unchanged.
    ///
    /// **The exhausted-supplier convention** (the scaffold's SPEC-HOLE U-08,
    /// resolved as the agreed reading): the Consumes list names the
    /// `YeastBox` that SPEC.md §4 also names as this process's discrete
    /// supplier, and the two are **one object** — the supplier parameter IS
    /// the container (no second input), `Rest` is pinned to
    /// [`EmptyYeastBox`] so exhaustion is total, and the exhausted shell is
    /// destructured inside so its 30 g continue as the §3 'empty' state
    /// ([`EmptyBox`]), the declared output the flow routes to recycling
    /// (SPEC.md §5 P1 waste routing).
    pub fn mix_dough<const B: u64, S: SupplyN<N2, Taken = Cons<FreshSachet, Cons<FreshSachet, Nil>>, Rest = EmptyYeastBox>>(person: Person<B>, flour: Flour<1_000>, water: Water<650>, salt: Salt<18>, yeast_box: S) -> (Person<B>, MixedDough, SpentSachet, SpentSachet, EmptyBox) {
        const {
            assert!(
                Flour::<1_000>::VALUE
                    + Water::<650>::VALUE
                    + Salt::<18>::VALUE
                    + 2 * FreshSachet::MASS_G
                    + EmptyBox::MASS_G
                    == MixedDough::MASS_G + 2 * SpentSachet::MASS_G + EmptyBox::MASS_G,
                "mass conservation violated in mix_dough (SPEC.md §5 P1 line 105): mass 1_000 + 650 + 18 + 8 + 8 + 30 = 1_682 + 1 + 1 + 30 (assert)"
            )
        };
        let (Cons(i1, Cons(i2, Nil)), rest) = yeast_box.supply_n(); // one at a time via SupplyN (R12, F-014)
        let FreshSachet { _seal: _ } = i1; // conserving transform (untripwired, F-040): its magnitudes continue in the outputs (checked by the asserts above)
        let FreshSachet { _seal: _ } = i2; // conserving transform (untripwired, F-040): its magnitudes continue in the outputs (checked by the asserts above)
        let YeastBox(Nil) = rest; // the exhausted supplier's shell (Rest = EmptyYeastBox): its mass continues as the empty-state output
        flour.defuse(); // conserving transform: its magnitudes continue in the outputs (checked by the asserts above)
        water.defuse(); // conserving transform: its magnitudes continue in the outputs (checked by the asserts above)
        salt.defuse(); // conserving transform: its magnitudes continue in the outputs (checked by the asserts above)
        (
            person,
            MixedDough::mint(),
            SpentSachet::mint(),
            SpentSachet::mint(),
            EmptyBox::mint(),
        )
    }

    /// P2 — Knead (SPEC.md §5 P2, line 109): a pure state change — the
    /// 1_682 g batch is unchanged in mass (structural).
    /// The person's 600 000 ms is drawn by the **adjacent** `draw_time` in the flow,
    /// recorded to the History under this process's name (F-048 — the template's
    /// stated convention); the process takes and returns the person unchanged.
    pub fn knead<const B: u64>(person: Person<B>, mixed_dough: MixedDough) -> (Person<B>, KneadedDough) {
        // mass balance 'mass 1_682 = 1_682 (structural)' (SPEC.md line 114): structural — carried by the shared magnitudes, no assert needed.
        mixed_dough.defuse(); // conserving state change (R9): the batch's 1_682 g continue unchanged
        (
            person,
            KneadedDough::mint(),
        )
    }

    /// P3 — Prove (SPEC.md §5 P3, line 117): the attended proving hour as a
    /// pure state change — the mass is unchanged (structural); the person's
    /// 3_600_000 ms draw (adjacent in the flow, F-048, recorded to the
    /// History under this process's name) stands in for the elapsed hour —
    /// wall-clock time is out of scope (SPEC.md §1).
    ///
    /// SPEC.md §5 P3 deliberately claims nothing here (its Satisfies line
    /// reads em-dash): this process *enables* REQ-028 — only its output
    /// state can be shaped — and the scaffold's SPEC-HOLE U-09 is resolved
    /// by placing the requirement bound on `divide_and_shape`, not here.
    pub fn prove<const B: u64>(person: Person<B>, kneaded_dough: KneadedDough) -> (Person<B>, ProvedDough) {
        // mass balance 'mass 1_682 = 1_682 (structural)' (SPEC.md line 122): structural — carried by the shared magnitudes, no assert needed.
        kneaded_dough.defuse(); // conserving state change (R9): the batch's 1_682 g continue unchanged
        (
            person,
            ProvedDough::mint(),
        )
    }

    /// P4 — Grease the tins (SPEC.md §5 P4, line 125): 12 g of butter, 6 g
    /// per tin, turns the two clean tins into greased ones — independent of
    /// P1–P3, the flow's only ordering freedom (SPEC.md §6).
    /// The person's 120_000 ms is drawn by the **adjacent** `draw_time` in
    /// the flow, recorded to the History under this process's name (F-048);
    /// the process takes and returns the person unchanged. Mass
    /// 450 + 450 + 12 = 456 + 456 is checked at compile time (SPEC.md §5 P4).
    ///
    /// SPEC.md §5 P4 deliberately claims nothing here (its Satisfies line
    /// reads em-dash): this process *enables* REQ-029 — only its output
    /// state can go in the oven — and the scaffold's SPEC-HOLE U-10 is
    /// resolved by placing the requirement bound on `bake`, not here.
    pub fn grease_tins<const B: u64>(person: Person<B>, clean_tin_1: CleanTin<450>, clean_tin_2: CleanTin<450>, butter: Butter<12>) -> (Person<B>, GreasedTin<456>, GreasedTin<456>) {
        const {
            assert!(
                2 * CleanTin::<450>::VALUE + Butter::<12>::VALUE
                    == 2 * GreasedTin::<456>::VALUE,
                "mass conservation violated in grease_tins (SPEC.md §5 P4 line 131): mass 450 + 450 + 12 = 456 + 456 (assert)"
            )
        };
        clean_tin_1.defuse(); // conserving transform: its magnitudes continue in the outputs (checked by the asserts above)
        clean_tin_2.defuse(); // conserving transform: its magnitudes continue in the outputs (checked by the asserts above)
        butter.defuse(); // conserving transform: its magnitudes continue in the outputs (checked by the asserts above)
        (
            person,
            GreasedTin::mint(),
            GreasedTin::mint(),
        )
    }

    /// P5 — Divide and shape (SPEC.md §5 P5, line 134): the proved batch is
    /// divided and shaped into the two 841 g loaves. **REQ-028 bounds the
    /// dough** (style A, F-048): any unproved state fails with the
    /// REQ-phrased `on_unimplemented` message (F-044) — the scaffold's
    /// SPEC-HOLE U-11 resolved per CS-2.
    /// The person's 480_000 ms is drawn by the **adjacent** `draw_time` in
    /// the flow, recorded to the History under this process's name (F-048);
    /// the process takes and returns the person unchanged. Mass
    /// 1_682 = 841 + 841 is checked at compile time over the bound's
    /// `MASS_G` (SPEC.md §5 P5).
    ///
    /// Satisfies: REQ-028
    pub fn divide_and_shape<const B: u64, D: Req028ProvedBeforeShaping>(person: Person<B>, dough: D) -> (Person<B>, ShapedLoaf, ShapedLoaf) {
        const {
            assert!(
                D::MASS_G == ShapedLoaf::MASS_G + ShapedLoaf::MASS_G,
                "mass conservation violated in divide_and_shape (SPEC.md §5 P5 line 139): mass 1_682 = 841 + 841 (assert)"
            )
        };
        // The permit-gated conserving extraction (F-054): the dough's mass
        // continues into the two shaped loaves minted below (checked by the
        // assert above).
        dough.into_loaves(ShapePermit { _seal: () });
        (
            person,
            ShapedLoaf::mint(),
            ShapedLoaf::mint(),
        )
    }

    /// P6 — Bake (SPEC.md §5 P6, line 142): **fallible (R17), one loaf per
    /// bake, run twice** — one shaped loaf seated in a greased tin goes into
    /// the oven with 2_500_000 J of grid energy and one outcome token (the
    /// only way variability enters, F-042), and comes out baked, or —
    /// finally, with no rework (the dough is consumed) — scorched. **REQ-029
    /// bounds the tin** (style A, F-048): a clean or used tin fails with the
    /// REQ-phrased message (F-044) — the scaffold's SPEC-HOLE U-12 resolved
    /// per CS-2. The single oven is moved in and returned in both arms (R2):
    /// the second bake cannot start until the first returns it — attempting
    /// both at once is the contention error at the exact line (R9).
    ///
    /// **Waste routing is a consumer parameter** (SPEC.md §5 P6 — the
    /// scaffold's SPEC-HOLE U-13, resolved as the spec's strong reading):
    /// the atmosphere comes in as a requirement-free bounded parameter and
    /// the bake's 100 g of steam and 2_200_000 J of waste heat are fed to it
    /// **inside the process**, in both arms, so neither waste ever exists
    /// loose. The scorched loaf stays a Fail-arm product the flow routes to
    /// the compost stream (REQ-031 — its only consumer).
    ///
    /// The person's 300_000 ms per bake is drawn by the **adjacent**
    /// `draw_time` in the flow *before* the fallible call (R17/F-048), so a
    /// scorch costs the same time as a good bake; the process takes and
    /// returns the person unchanged. Per-arm mass conservation is one
    /// independent compile-time assert per arm, both firing at every
    /// instantiation (R17/F-045); the energy split is common to both arms.
    /// The `Result` is `#[must_use]` (std) on top of the bundles' own
    /// `must_use`, and the bundles implement no `Debug`, so
    /// `.unwrap()`/`.expect()` do not compile (F-047) — the flow must
    /// `match` both arms (the scaffold's SPEC-HOLE U-14, resolved per CS-2
    /// with the four-combination grouping in [`crate::flows`]).
    ///
    /// Satisfies: REQ-029, REQ-031
    pub fn bake<const B: u64, T: Req029GreasedTinInTheOven, A: Consumer<Steam<100>, Next = A> + Consumer<WasteHeat<2_200_000>, Next = A>>(person: Person<B>, oven: Oven, shaped_loaf: ShapedLoaf, greased_tin: T, grid_energy: GridEnergy<2_500_000>, bake_outcome: BakeOutcome, atmosphere: A) -> Result<BakeOk<B, A>, BakeFail<B, A>> {
        const {
            assert!(
                ShapedLoaf::MASS_G + T::MASS_G == BakedLoaf::MASS_G + 453 + 100,
                "success-arm mass conservation violated in bake (SPEC.md §5 P6 line 158): the baked loaf, the used tin and the steam must sum the shaped loaf and the greased tin exactly (841 + 456 = 744 + 453 + 100)"
            )
        };
        const {
            assert!(
                ShapedLoaf::MASS_G + T::MASS_G == ScorchedLoaf::MASS_G + 453 + 100,
                "failure-arm mass conservation violated in bake (SPEC.md §5 P6 line 158): the scorched loaf, the used tin and the steam must sum the shaped loaf and the greased tin exactly (841 + 456 = 744 + 453 + 100)"
            )
        };
        const {
            assert!(
                2_500_000 == BakedLoaf::EMBODIED_J + 2_200_000,
                "energy conservation violated in bake (SPEC.md §5 P6 line 158): energy 2_500_000 = 300_000 + 2_200_000 (assert)"
            )
        };
        shaped_loaf.defuse(); // conserving transform: its magnitudes continue in the outputs (checked by the asserts above)
        // The permit-gated conserving extraction (F-054): the tin's mass
        // continues into the used tin and the loaf's crust minted below
        // (checked by the per-arm asserts above).
        greased_tin.into_oven(BakePermit { _seal: () });
        grid_energy.defuse(); // conserving transform: its magnitudes continue in the outputs (checked by the asserts above)
        // Consumer-parameter waste routing (SPEC.md §5 P6): both arms vent
        // the same steam and waste heat, fed to the atmosphere INSIDE the
        // process so the waste never exists loose.
        let atmosphere = send_to(atmosphere, Steam::<100>::mint());
        let atmosphere = send_to(atmosphere, WasteHeat::<2_200_000>::mint());
        match bake_outcome.consume_kind() {
            BakeOutcomeKind::Success => Ok(BakeOk {
                person,
                baked_loaf: BakedLoaf::mint(),
                used_tin: UsedTin::mint(),
                oven,
                atmosphere,
            }),
            BakeOutcomeKind::Failure => Err(BakeFail {
                person,
                scorched_loaf: ScorchedLoaf::mint(),
                used_tin: UsedTin::mint(),
                oven,
                atmosphere,
            }),
        }
    }

}

#[cfg(test)]
mod tests {
    //! Per-process unit tests (R5): every process turns specific inputs into
    //! the expected outputs with nothing left unaccounted for — one test per
    //! arm for the fallible P6 (branch coverage is leak coverage, F-002).
    //! These tests sit inside the privacy boundary, so they may mint fixtures
    //! and defuse outputs directly; downstream-style accounting is exercised
    //! by the integration tests in `tests/flows.rs`.

    use super::boundary::{
        bake_outcome_will_fail, bake_outcome_will_succeed, draw_grid_energy, draw_water,
        full_yeast_box, new_atmosphere, new_clean_tin, new_compost_stream, new_grid,
        new_household, new_oven, new_pantry_bag, new_pantry_block, new_pantry_jar, new_recycling,
        new_tap, rest_pantry_bag, rest_pantry_block, rest_pantry_jar, rest_used_tin,
    };
    use super::processes::{
        bake, divide_and_shape, draw_butter, draw_flour, draw_salt, grease_tins, knead, mix_dough,
        prove,
    };
    use super::{
        BakedLoaf, Butter, CleanTin, EmptyBox, EmptyYeastBox, Flour, FreshSachet, FullYeastBox,
        GreasedTin, KneadedDough, MixedDough, ProvedDough, Salt, ScorchedLoaf, ShapedLoaf,
        SpentSachet, Water,
    };
    use crate::characteristics::{Greased, Proved};
    use model_core::boundary::{send_to, take_one};
    use model_core::common::boundary::new_person;
    use model_core::nat::aliases::N2;

    /// P1: the drawn ingredients and both sachets become the 1_682 g batch
    /// (SPEC.md P1: 1_000 + 650 + 18 + 8 + 8 + 30 = 1_682 + 1 + 1 + 30), the
    /// box is exhausted to its 30 g empty state, and the wrappers and box
    /// reach recycling.
    #[test]
    fn mix_dough_balances_and_exhausts_the_box() {
        let person = new_person::<900_000>();
        let (flour, bag) = draw_flour::<1_000, 500, 1_500>(new_pantry_bag());
        let (water, tap) = draw_water::<650>(new_tap());
        let (salt, jar) = draw_salt::<18, 482, 500>(new_pantry_jar());
        let (person, dough, s1, s2, empty_box) =
            mix_dough(person, flour, water, salt, full_yeast_box::<N2>());
        assert_eq!(
            Flour::<1_000>::VALUE
                + Water::<650>::VALUE
                + Salt::<18>::VALUE
                + 2 * FreshSachet::MASS_G
                + EmptyBox::MASS_G,
            MixedDough::MASS_G + 2 * SpentSachet::MASS_G + EmptyBox::MASS_G
        );
        let recycling = send_to(new_recycling(), s1);
        let recycling = send_to(recycling, s2);
        let _recycling = send_to(recycling, empty_box);
        // Inside the privacy boundary this test may defuse directly.
        dough.defuse();
        rest_pantry_bag(bag);
        rest_pantry_jar(jar);
        let _reusables = (person, tap);
    }

    /// The yeast box (SPEC.md §3): a discrete supplier of its two real
    /// sachets, exhausted to the distinct empty shell — count and contents
    /// are the same type-level list, so they cannot disagree (R7, R12).
    #[test]
    fn the_yeast_box_supplies_its_two_real_sachets_then_is_exhausted() {
        let yeast_box = full_yeast_box::<N2>();
        assert_eq!(FullYeastBox::COUNT, 2);
        let (s1, yeast_box) = take_one(yeast_box);
        let (s2, yeast_box) = take_one(yeast_box);
        assert_eq!(EmptyYeastBox::COUNT, 0);
        let _exhausted_shell: EmptyYeastBox = yeast_box;
        // Fresh sachets are untripwired (F-040): the box accounts for them.
        let _sachets_kept_to_test_end = (s1, s2);
    }

    /// P2: a pure state change (SPEC.md P2: 1_682 = 1_682, structural).
    #[test]
    fn knead_is_a_state_change_with_no_mass_change() {
        let person = new_person::<600_000>();
        let (person, kneaded) = knead(person, MixedDough::mint());
        assert_eq!(MixedDough::MASS_G, KneadedDough::MASS_G);
        kneaded.defuse();
        let _reusable = person;
    }

    /// P3: a pure state change (SPEC.md P3: 1_682 = 1_682, structural)
    /// producing the proved state. This process is what *enables* REQ-028:
    /// only its output can be shaped.
    #[test]
    fn prove_is_a_state_change_producing_the_shapeable_state() {
        let person = new_person::<3_600_000>();
        let (person, proved) = prove(person, KneadedDough::mint());
        assert_eq!(KneadedDough::MASS_G, <ProvedDough as Proved>::MASS_G);
        proved.defuse();
        let _reusable = person;
    }

    /// P4: the butter continues onto the tins (SPEC.md P4:
    /// 450 + 450 + 12 = 456 + 456, assert), producing the greased state
    /// REQ-029 needs — this process is what *enables* that requirement.
    #[test]
    fn grease_tins_balances_the_butter_onto_the_tins() {
        let person = new_person::<120_000>();
        let (butter, block) = draw_butter::<12, 238, 250>(new_pantry_block());
        let (person, tin_1, tin_2) = grease_tins(person, new_clean_tin(), new_clean_tin(), butter);
        assert_eq!(
            2 * CleanTin::<450>::VALUE + Butter::<12>::VALUE,
            2 * <GreasedTin<456> as Greased>::MASS_G
        );
        tin_1.defuse();
        tin_2.defuse();
        rest_pantry_block(block);
        let _reusable = person;
    }

    /// P5: the proved batch splits exactly into the two shaped loaves
    /// (SPEC.md P5: 1_682 = 841 + 841, assert), and only the proved state
    /// could enter.
    ///
    /// Verifies: REQ-028
    #[test]
    fn divide_and_shape_splits_the_proved_dough_exactly() {
        let person = new_person::<480_000>();
        let (person, loaf_1, loaf_2) = divide_and_shape(person, ProvedDough::mint());
        assert_eq!(
            <ProvedDough as Proved>::MASS_G,
            ShapedLoaf::MASS_G + ShapedLoaf::MASS_G
        );
        loaf_1.defuse();
        loaf_2.defuse();
        let _reusable = person;
    }

    /// P6 success arm (R17): the shaped loaf and the greased tin continue as
    /// the baked loaf, the used tin and the steam (841 + 456 = 744 + 453 +
    /// 100); the steam and waste heat reached the atmosphere inside the
    /// process; the baked loaf goes to the household — the only sink that
    /// accepts it.
    ///
    /// Verifies: REQ-029, REQ-030
    #[test]
    fn bake_success_arm_balances_and_the_loaf_reaches_the_household() {
        let person = new_person::<300_000>();
        let (energy, grid) = draw_grid_energy::<2_500_000>(new_grid());
        match bake(
            person,
            new_oven(),
            ShapedLoaf::mint(),
            GreasedTin::<456>::mint(),
            energy,
            bake_outcome_will_succeed(),
            new_atmosphere(),
        ) {
            Ok(ok) => {
                assert_eq!(
                    ShapedLoaf::MASS_G + <GreasedTin<456> as Greased>::MASS_G,
                    BakedLoaf::MASS_G + 453 + 100
                );
                let _household = send_to(new_household(), ok.baked_loaf);
                rest_used_tin(ok.used_tin);
                let _accounted = (ok.person, ok.oven, ok.atmosphere, grid);
            }
            Err(_fail) => panic!("a success token must realise the success arm"),
        }
    }

    /// P6 failure arm (R17): failure conserves too — same masses, same
    /// energy split, same steam and waste heat to the atmosphere inside the
    /// process; only the loaf differs, and it is final: its one exit is the
    /// compost stream.
    ///
    /// Verifies: REQ-029, REQ-031
    #[test]
    fn bake_failure_arm_balances_and_the_scorched_loaf_reaches_compost() {
        let person = new_person::<300_000>();
        let (energy, grid) = draw_grid_energy::<2_500_000>(new_grid());
        match bake(
            person,
            new_oven(),
            ShapedLoaf::mint(),
            GreasedTin::<456>::mint(),
            energy,
            bake_outcome_will_fail(),
            new_atmosphere(),
        ) {
            Ok(_ok) => panic!("a failure token must realise the failure arm"),
            Err(fail) => {
                assert_eq!(
                    ShapedLoaf::MASS_G + <GreasedTin<456> as Greased>::MASS_G,
                    ScorchedLoaf::MASS_G + 453 + 100
                );
                assert_eq!(ScorchedLoaf::EMBODIED_J, BakedLoaf::EMBODIED_J);
                let _compost = send_to(new_compost_stream(), fail.scorched_loaf);
                rest_used_tin(fail.used_tin);
                let _accounted = (fail.person, fail.oven, fail.atmosphere, grid);
            }
        }
    }

    /// An abandoned scorched loaf is caught by its tripwire at test time (R1
    /// layer 2, F-008): no compile-time layer sees a named-and-used binding
    /// that never reaches compost — the tripwire is REQ-031's backstop.
    ///
    /// Verifies: REQ-031
    #[test]
    #[should_panic(expected = "resource leak: ScorchedLoaf dropped without being consumed")]
    fn abandoned_scorched_loaf_trips_the_tripwire() {
        let scorched = ScorchedLoaf::mint();
        let _never_reaches_compost = scorched;
    }
}
