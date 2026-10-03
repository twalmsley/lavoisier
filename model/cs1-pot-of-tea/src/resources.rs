//! The kitchen's sealed resource family (R1), its creation/exit boundary
//! (R12) and its processes P1..P5 (SPEC.md §5, F-031).
//!
//! Layout per F-006/F-031: this module holds the sealed resource types, with
//! the [`boundary`] (the kitchen setup, the mains/grid draw processes, the
//! teabag box) and the [`processes`] (which mint quantity-bearing values and
//! therefore live inside the privacy boundary) as child modules.
//!
//! One type per processing state (R9, F-023), straight from SPEC.md §3:
//!
//! * water: [`ColdWater`] → (inside the kettle) → tea in the [`PotOfTea`];
//!   *boiling* is a type, not a temperature reading — the embodied energy is
//!   carried in the type from boiling onward (SPEC.md §3, review decision 1);
//! * kettle: [`Kettle`] (empty) → [`FilledKettle`] → [`BoilingKettle`] →
//!   [`Kettle`] again after the pour;
//! * teapot: [`Teapot`] (empty) → [`LoadedPot`] (exactly 3 bags; no other
//!   count has a loaded state, review decision 2) → [`PotOfTea`];
//! * teabags: [`DryTeabag`] → [`SpentTeabag`].
//!
//! Every consumable here carries the kernel's tripwire `Drop` (R1 layer 2,
//! F-008) except the [`DryTeabag`], which is *kept* by the box and the loaded
//! pot that account for it (`no_tripwire`, F-040). Reusable resources (the
//! kettle and teapot vessels, the tap and socket, and model-core's `Person`
//! with the SPEC's 300 000 ms budget) are moved in and returned by every
//! process (R2) and stay with the caller.

use crate::characteristics::{Boiling, FoodWasteConsumer, KitchenAirSink, LoadedToBrew, sealed};
// One line on purpose: the traceability grep (F-021) skips `use` lines, but
// only when the line itself starts with `use`.
use crate::requirements::{assert_req006, assert_req007, assert_req008, assert_req009};
use core::marker::PhantomData;
use model_core::boundary::{Consumer, Supplier};
use model_core::list::{Cons, Len, Nil};
use model_core::nat::{Succ, Zero};

// ---------------------------------------------------------------------------
// Water and energy (continuous resources, R15).
// ---------------------------------------------------------------------------

model_core::container_resource! {
    /// Cold water drawn from the mains, in grams (R7 base mass unit; SPEC.md
    /// §3: 1500 g drawn). Enters the model only through
    /// [`boundary::draw_cold_water`] (R12, R15).
    ColdWater,
    unit = "grams",
    must_use = "ColdWater is a conserved resource: pass it on or hand it to a Consumer"
}

model_core::container_resource! {
    /// Electrical energy drawn from the grid, in joules (R7; SPEC.md §3:
    /// 550 000 J drawn). Enters the model only through
    /// [`boundary::draw_grid_energy`] (R12, R15).
    Electricity,
    unit = "joules",
    must_use = "Electricity is a conserved resource: pass it on or hand it to a Consumer"
}

model_core::container_resource! {
    /// Waste heat, in joules (R15: waste is an ordinary conserved output;
    /// SPEC.md §3: 50 000 J from the kettle plus 20 000 J from steeping). All
    /// of it must be accounted to the kitchen-air sink (REQ-009).
    WasteHeat,
    unit = "joules",
    must_use = "WasteHeat is a conserved waste product: account it to the kitchen-air sink (REQ-009)"
}

model_core::container_resource! {
    /// Food waste leaving the bin for the council collection, in grams
    /// (SPEC.md P5: 36 g). Minted only inside [`processes::empty_bin`], the
    /// sealed disposal path (F-039), where the bin's kept spent teabags
    /// continue as this one conserved mass.
    FoodWaste,
    unit = "grams",
    must_use = "FoodWaste is a conserved waste product: hand it to the council collection"
}

// ---------------------------------------------------------------------------
// The kettle's states (one type per state, R9).
// ---------------------------------------------------------------------------

model_core::reusable_resource! {
    /// The electric kettle, empty — the state it enters the model in and the
    /// state it is returned in after the pour (SPEC.md §3). Reusable (R2): no
    /// tripwire; it stays with the caller at flow end.
    ///
    /// Placeholder: kitchen setup at flow start — one kettle.
    Kettle,
    must_use = "Kettle is a reusable resource: pass it on or return it to the caller"
}

model_core::container_resource! {
    /// The kettle filled with `V` grams of cold water (SPEC.md P1: 1500 g).
    /// A distinct processing state (R9): it cannot be poured — only the
    /// boiling state satisfies REQ-006. Tripwired (F-008): a filled kettle
    /// that never boils (or is never otherwise accounted) fails the test
    /// that leaked its water.
    FilledKettle,
    unit = "grams of water",
    must_use = "FilledKettle is a conserved resource: boil it or hand it to a Consumer"
}

model_core::container_resource! {
    /// The kettle at the boil: `WATER_G` grams of boiling water carrying `V`
    /// joules of embodied energy (SPEC.md §3: the energy is carried in the
    /// type from boiling onward — review decision 1). The only kettle state
    /// that can be poured into the pot (REQ-006); see [`KettleAtTheBoil`]
    /// for the CS-1 quantities.
    BoilingKettle<const WATER_G: u64>,
    unit = "joules (embodied)",
    must_use = "BoilingKettle is a conserved resource: pour and brew it or hand it to a Consumer"
}

impl<const WATER_G: u64, const V: u64> BoilingKettle<WATER_G, V> {
    /// The boiling water's mass, in grams (R7). (`VALUE` is the embodied
    /// energy in joules.)
    pub const WATER_G: u64 = WATER_G;
}

/// The boiling kettle at the CS-1 quantities (SPEC.md §3: 1500 g of water,
/// 500 000 J embodied). The alias carries the satisfaction tag because a tag
/// inside the `container_resource!` invocation above would be silently
/// dropped by trace.sh (F-037).
///
/// Satisfies: REQ-006
pub type KettleAtTheBoil = BoilingKettle<1500, 500_000>;
model_core::satisfies!(assert_req006, KettleAtTheBoil);

/// The boiling characteristic (R6) lives on the boiling state only
/// (R9/F-023): `pour_away` is the permit-gated conserving extraction that
/// only [`processes::pour_and_brew`] can call — the water and embodied energy
/// continue into the brew, checked by that process's conservation asserts.
impl<const G: u64, const E: u64> Boiling for BoilingKettle<G, E> {
    const WATER_G: u64 = G;
    const EMBODIED_J: u64 = E;
    fn pour_away(self, _permit: BrewPermit) -> Kettle {
        // Conserving transform: the water and its embodied energy continue
        // as the brew outputs minted by pour_and_brew (R1); the vessel
        // returns to its empty state.
        self.defuse();
        Kettle::mint()
    }
}
impl<const G: u64, const E: u64> sealed::Sealed for BoilingKettle<G, E> {}

// ---------------------------------------------------------------------------
// Teabags and the teapot's states (R9, R13).
// ---------------------------------------------------------------------------

model_core::consumable_resource! {
    /// A dry teabag: a discrete item, its own object (R13; SPEC.md §3: dry
    /// mass 3 g, see [`DryTeabag::MASS_G`]). Deliberately **not** tripwired
    /// (`no_tripwire`, F-040): dry bags are *kept* by the containers that
    /// hold them (the teabag box, the loaded pot), and a tripwired item
    /// inside an abandoned container would panic from the container's own
    /// drop, masking the real leak site. Whole-value discard is still caught
    /// by `#[must_use]`.
    DryTeabag,
    must_use = "DryTeabag is a conserved resource: pass it on or hand it to a Consumer",
    no_tripwire
}

impl DryTeabag {
    /// A dry teabag's mass, in grams (R7; SPEC.md §3).
    pub const MASS_G: u64 = 3;
}

model_core::consumable_resource! {
    /// A spent teabag: the teabag's state after brewing (one type per state,
    /// R9; SPEC.md §3: 12 g — the dry 3 g plus 9 g of absorbed water).
    /// Tripwired (F-008): a spent bag that never reaches the food-waste bin
    /// (REQ-008) fails the test that leaked it. Its **only** consumer is the
    /// [`FoodWasteBin`] — no other sink accepts it, so REQ-008 is structural.
    SpentTeabag,
    must_use = "SpentTeabag is a conserved waste product: it must reach the food-waste bin (REQ-008)"
}

impl SpentTeabag {
    /// A spent teabag's mass, in grams (R7; SPEC.md §3: 3 g dry plus 9 g of
    /// absorbed water).
    pub const MASS_G: u64 = 12;
}

/// A type-level list of exactly three dry teabags — the `Taken =` shape
/// [`processes::load_pot`]'s `SupplyN<N3>` bound requires (F-014).
pub type ThreeDryBags = Cons<DryTeabag, Cons<DryTeabag, Cons<DryTeabag, Nil>>>;

/// A type-level list of exactly three spent teabags — what
/// [`processes::pour_and_brew`] feeds to the food-waste bin (REQ-008).
pub type ThreeSpentBags = Cons<SpentTeabag, Cons<SpentTeabag, Cons<SpentTeabag, Nil>>>;

model_core::reusable_resource! {
    /// The teapot, empty — the state it enters the model in. Reusable (R2),
    /// but it **leaves with the product** (SPEC.md §3): once loaded and
    /// brewed it continues as the [`PotOfTea`], which the drinker takes and
    /// never gives back (pot return is out of scope, SPEC.md §7).
    ///
    /// Placeholder: kitchen setup at flow start — one teapot.
    Teapot,
    must_use = "Teapot is a reusable resource: pass it on or return it to the caller"
}

model_core::consumable_resource! {
    /// The teapot loaded with **exactly 3** dry teabags, held as real sealed
    /// payload objects (R12, R13; F-036 held contents). This state exists
    /// only at exactly 3 bags (SPEC.md §8, review decision 2) — REQ-007 is a
    /// fact about the type, not a runtime count. Tripwired (F-008): a loaded
    /// pot that never brews fails the test that leaked it.
    LoadedPot {
        bags: (DryTeabag, DryTeabag, DryTeabag),
    },
    must_use = "LoadedPot is a conserved resource: brew it or hand it to a Consumer"
}

/// The loaded pot under its requirement-facing name; the alias carries the
/// tag because a tag inside the `consumable_resource!` invocation above would
/// be silently dropped by trace.sh (F-037).
///
/// Satisfies: REQ-007
pub type BrewReadyPot = LoadedPot;
model_core::satisfies!(assert_req007, BrewReadyPot);

/// The brew-ready characteristic (R6) lives on the loaded state only
/// (R9/F-023): `steep` is the permit-gated conserving extraction that only
/// [`processes::pour_and_brew`] can call — the 3 dry bags continue as the 3
/// spent bags, the vessel as the pot of tea, checked by that process's
/// conservation asserts.
impl LoadedToBrew for LoadedPot {
    const DRY_G: u64 = 3 * DryTeabag::MASS_G;
    const SPENT_G: u64 = 3 * SpentTeabag::MASS_G;
    fn steep(self, _permit: BrewPermit) -> (SpentTeabag, SpentTeabag, SpentTeabag) {
        // Conserving transform: the pot (vessel plus its 3 held dry bags) is
        // defused; the bags continue as the spent bags below, the vessel as
        // the PotOfTea minted by pour_and_brew (R1).
        self.defuse();
        (SpentTeabag::mint(), SpentTeabag::mint(), SpentTeabag::mint())
    }
}
impl sealed::Sealed for LoadedPot {}

model_core::container_resource! {
    /// The pot of tea: the teapot carrying `TEA_G` grams of tea with `V`
    /// joules of embodied energy (SPEC.md §3: 1473 g, 480 000 J). The final
    /// product; it leaves the model with the [`Drinker`] (pot return is out
    /// of scope, SPEC.md §7). Tripwired (F-008).
    PotOfTea<const TEA_G: u64>,
    unit = "joules (embodied)",
    must_use = "PotOfTea is the product: hand it to the drinker (a Consumer)"
}

impl<const TEA_G: u64, const V: u64> PotOfTea<TEA_G, V> {
    /// The tea's mass, in grams (R7). (`VALUE` is the embodied energy in
    /// joules.)
    pub const TEA_G: u64 = TEA_G;
}

// ---------------------------------------------------------------------------
// Boundary objects at the kitchen's edge (SPEC.md §4).
// ---------------------------------------------------------------------------

model_core::reusable_resource! {
    /// The mains tap: the unbounded boundary source cold water is drawn from
    /// (R15: a draw-style boundary process, never a `Supplier` impl, F-028).
    ///
    /// Placeholder: mains water — assumed unbounded (SPEC.md §7).
    MainsTap,
    must_use = "MainsTap is a boundary resource: pass it on like any other resource"
}

model_core::reusable_resource! {
    /// The grid socket: the unbounded boundary source electrical energy is
    /// drawn from (R15, F-028).
    ///
    /// Placeholder: grid electricity — assumed unbounded (SPEC.md §7).
    GridSocket,
    must_use = "GridSocket is a boundary resource: pass it on like any other resource"
}

/// The permit gating the brew's conserving extractions (the R16 `Permit`
/// pattern): [`crate::characteristics::Boiling::pour_away`] and
/// [`crate::characteristics::LoadedToBrew::steep`] each demand one, and it
/// has a private field with no public constructor, so only
/// [`processes::pour_and_brew`] (inside this privacy boundary) can call them
/// — the extractions can never be used to vanish water, energy or bag mass
/// outside the process whose asserts account for them (R1).
pub struct BrewPermit {
    _seal: (),
}

/// A box of teabags: a supplier at the system boundary (R12). Its contents
/// are a type-level list of real [`DryTeabag`] values; the count **is** the
/// list's length, so count and contents cannot disagree. CS-1's box holds 40
/// (SPEC.md §4) and is returned at 37 after P2.
///
/// Placeholder: teabag box — brand/vendor not modelled (SPEC.md §4).
#[must_use = "TeabagBox is a boundary resource: pass it on like any other resource"]
pub struct TeabagBox<Items>(Items);

/// An exhausted teabag box: a distinct resource type that must itself be
/// accounted for (R12). It cannot supply — there is no `Supplier` impl for
/// it.
pub type EmptyTeabagBox = TeabagBox<Nil>;

/// `Supplier` is implemented ONLY for a non-empty box (R12); supplying from
/// an empty box is a compile error with the modeller-phrased
/// `on_unimplemented` message from model-core (F-015).
impl<H, T> Supplier for TeabagBox<Cons<H, T>> {
    type Item = H;
    type Next = TeabagBox<T>;
    fn supply(self) -> (H, TeabagBox<T>) {
        let Cons(head, tail) = self.0;
        (head, TeabagBox(tail))
    }
}

impl<Items: Len> TeabagBox<Items> {
    /// How many teabags the box holds — the length of its contents list (R7,
    /// R12).
    pub const COUNT: u64 = Items::LEN;
}

/// The food-waste bin (SPEC.md §4): the dedicated consumer REQ-008 routes
/// every spent teabag to. `Space` is the type-level number of bags it can
/// still accept (CS-1: 10); `Contents` keeps the real spent-teabag objects
/// it has consumed (F-016). The decreasing space parameter is a hard rule,
/// not a style choice: a contents-keeping consumer without one sends trait
/// resolution into unbounded exploration and, at this workspace's mandated
/// recursion limit, crashes the compiler (F-034). The bin is emptied — and
/// its kept tripwired contents defused — only through
/// [`processes::empty_bin`], the sealed disposal path (F-039).
///
/// Satisfies: REQ-008
pub struct FoodWasteBin<Space, Contents = Nil> {
    contents: Contents,
    _space: PhantomData<Space>,
}

// Compile-checked backing for the tag above (R10, F-020).
model_core::satisfies!(assert_req008, FoodWasteBin<Zero>);

// The food-waste-consumer characteristic (R6) holds in every fill state —
// REQ-008 is about which consumer the bags go to; whether there is space
// left is the Consumer impl's business (R12).
impl<Space, C> FoodWasteConsumer for FoodWasteBin<Space, C> {}

/// A full bin: zero space left — a distinct resource type that must itself
/// be accounted for (R12), e.g. via [`processes::empty_bin`].
pub type FullFoodWasteBin<Contents> = FoodWasteBin<Zero, Contents>;

/// `Consumer` is implemented ONLY while space remains (R12, F-034): space
/// goes down by one and the real spent-teabag object is kept at the front of
/// the contents list (F-016). Consuming into a full bin is a compile error
/// with model-core's modeller-phrased message (F-015).
impl<S, C> Consumer<SpentTeabag> for FoodWasteBin<Succ<S>, C> {
    type Next = FoodWasteBin<S, Cons<SpentTeabag, C>>;
    fn consume(self, item: SpentTeabag) -> Self::Next {
        FoodWasteBin {
            contents: Cons(item, self.contents),
            _space: PhantomData,
        }
    }
}

impl<Space, Contents: Len> FoodWasteBin<Space, Contents> {
    /// How many spent teabags the bin holds — the length of its contents
    /// list, so count and contents cannot disagree (R7, R12).
    pub const HELD: u64 = Contents::LEN;
}

/// The kitchen air: the unbounded boundary sink every joule of waste heat is
/// accounted to (REQ-009; SPEC.md §4). `Next = Self` (R15, F-029): legal
/// only at the system boundary, and it necessarily discards what it consumes
/// — the one sanctioned exception to "a consumer keeps what it consumes".
///
/// Placeholder: kitchen air — assumed able to absorb all waste heat
/// (SPEC.md §7; the embodied 480 000 J leaves with the tea, not via the air).
///
/// Satisfies: REQ-009
#[must_use = "KitchenAir is a boundary resource: pass it on like any other resource"]
pub struct KitchenAir {
    _seal: (),
}

// Compile-checked backing for the tag above (R10, F-020).
model_core::satisfies!(assert_req009, KitchenAir);

impl KitchenAirSink for KitchenAir {}

/// The air absorbs waste heat of any magnitude; `Next = Self` (R15).
impl<const E: u64> Consumer<WasteHeat<E>> for KitchenAir {
    type Next = KitchenAir;
    fn consume(self, item: WasteHeat<E>) -> KitchenAir {
        item.defuse(); // sanctioned consumer role (F-008); unbounded sink discards (F-029)
        self
    }
}

/// The drinker taking delivery of the pot of tea: an unbounded boundary sink
/// (`type Next = Self`, R15, F-029; SPEC.md §4). The drinker takes the pot
/// and never gives it back (SPEC.md §7).
///
/// Placeholder: the drinker — pot return is out of scope.
#[must_use = "Drinker is a boundary resource: pass it on like any other resource"]
pub struct Drinker {
    _seal: (),
}

/// The drinker accepts the pot of tea, with its mass and embodied energy;
/// `Next = Self` (R15, F-029). Only the finished product is accepted — a
/// spent teabag, say, has no consumer here (REQ-008 routes it to the bin).
impl<const G: u64, const E: u64> Consumer<PotOfTea<G, E>> for Drinker {
    type Next = Drinker;
    fn consume(self, item: PotOfTea<G, E>) -> Drinker {
        item.defuse();
        self
    }
}

/// The council food-waste collection: the unbounded boundary sink the bin's
/// contents leave to when the bin is emptied (SPEC.md P5). `Next = Self`
/// (R15, F-029). It accepts only the bin's [`FoodWaste`] — a loose spent
/// teabag cannot be handed to it directly, so the bags' only route out of
/// the model is through the bin (REQ-008).
///
/// Placeholder: council food-waste collection — assumed unbounded.
#[must_use = "CouncilCollection is a boundary resource: pass it on like any other resource"]
pub struct CouncilCollection {
    _seal: (),
}

/// The collection accepts bagged food waste of any mass; `Next = Self`
/// (R15, F-029).
impl<const G: u64> Consumer<FoodWaste<G>> for CouncilCollection {
    type Next = CouncilCollection;
    fn consume(self, item: FoodWaste<G>) -> CouncilCollection {
        item.defuse();
        self
    }
}

/// The creation boundary of the kitchen (R12, F-006): the only production
/// code where the kitchen setup, the mains/grid draws and the teabag box
/// come into existence (SPEC.md §4).
pub mod boundary {
    use super::{
        ColdWater, CouncilCollection, Drinker, DryTeabag, Electricity, FoodWasteBin, GridSocket,
        KitchenAir, MainsTap, PhantomData, Teapot, TeabagBox,
    };
    use model_core::list::{Cons, Nil};
    use model_core::nat::{Succ, Zero};

    /// The kettle enters the model, empty (R12).
    ///
    /// Placeholder: kitchen setup at flow start — one kettle.
    pub fn new_kettle() -> super::Kettle {
        super::Kettle::mint()
    }

    /// The teapot enters the model, empty (R12).
    ///
    /// Placeholder: kitchen setup at flow start — one teapot.
    pub fn new_teapot() -> Teapot {
        Teapot::mint()
    }

    /// The mains tap enters the model (R12).
    ///
    /// Placeholder: mains water — assumed unbounded (SPEC.md §7).
    pub fn new_mains_tap() -> MainsTap {
        MainsTap::mint()
    }

    /// The grid socket enters the model (R12).
    ///
    /// Placeholder: grid electricity — assumed unbounded (SPEC.md §7).
    pub fn new_grid_socket() -> GridSocket {
        GridSocket::mint()
    }

    /// Draws `TAKE` grams of cold water from the mains (R15: an unbounded
    /// source of continuous material is a draw-style boundary process, never
    /// a `Supplier` impl, F-028). The tap is returned (R2).
    ///
    /// Placeholder: mains water — assumed unbounded (SPEC.md §7).
    pub fn draw_cold_water<const TAKE: u64>(tap: MainsTap) -> (ColdWater<TAKE>, MainsTap) {
        (ColdWater::mint(), tap)
    }

    /// Draws `TAKE` joules from the grid (R15, F-028). The socket is
    /// returned (R2).
    ///
    /// Placeholder: grid electricity — assumed unbounded (SPEC.md §7).
    pub fn draw_grid_energy<const TAKE: u64>(socket: GridSocket) -> (Electricity<TAKE>, GridSocket) {
        (Electricity::mint(), socket)
    }

    /// Maps a type-level count to the list type of that many dry teabags:
    /// `BagsOf<N40>` is forty [`DryTeabag`]s.
    pub trait Replicate {
        /// The list type holding `Self`-many dry teabags.
        type List;
    }
    impl Replicate for Zero {
        type List = Nil;
    }
    impl<N: Replicate> Replicate for Succ<N> {
        type List = Cons<DryTeabag, N::List>;
    }

    /// Shorthand for [`Replicate::List`].
    pub type BagsOf<N> = <N as Replicate>::List;

    /// Builds a list of real teabags. Private: together with
    /// `DryTeabag::mint`, this is the only place teabags come into existence
    /// (R1), reachable only through [`full_teabag_box`].
    trait Fill {
        fn fill() -> Self;
    }
    impl Fill for Nil {
        fn fill() -> Nil {
            Nil
        }
    }
    impl<T: Fill> Fill for Cons<DryTeabag, T> {
        fn fill() -> Self {
            Cons(DryTeabag::mint(), T::fill())
        }
    }

    /// Public-in-signature but unimplementable-outside wrapper over the
    /// private fill machinery, so [`full_teabag_box`]'s where-clause can name
    /// it without letting outside code implement it (the classic sealed-trait
    /// pattern, F-026).
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
        use super::super::DryTeabag;
        use model_core::list::{Cons, Nil};
        pub trait Sealed {}
        impl Sealed for Nil {}
        impl<T: Sealed> Sealed for Cons<DryTeabag, T> {}
    }

    /// The fill function (R12): a full box of `N` real dry teabags enters
    /// the model here — CS-1's box is `full_teabag_box::<N40>()`
    /// (SPEC.md §4: capacity 40).
    ///
    /// Placeholder: teabag box — brand/vendor not modelled.
    pub fn full_teabag_box<N: Replicate>() -> TeabagBox<BagsOf<N>>
    where
        BagsOf<N>: FillSealed,
    {
        TeabagBox(FillSealed::fill_sealed())
    }

    /// An empty food-waste bin with `Space` slots enters the model (CS-1:
    /// `new_food_waste_bin::<N10>()`, SPEC.md §4). Creating an *empty*
    /// consumer brings no resources into existence, so this is an ordinary
    /// public boundary function (R12).
    pub fn new_food_waste_bin<Space>() -> FoodWasteBin<Space, Nil> {
        FoodWasteBin {
            contents: Nil,
            _space: PhantomData,
        }
    }

    /// The kitchen air enters the model (R12). An empty unbounded sink holds
    /// nothing, so this is an ordinary public boundary function.
    pub fn new_kitchen_air() -> KitchenAir {
        KitchenAir { _seal: () }
    }

    /// The drinker enters the model (R12).
    pub fn new_drinker() -> Drinker {
        Drinker { _seal: () }
    }

    /// The council food-waste collection enters the model (R12).
    pub fn new_council_collection() -> CouncilCollection {
        CouncilCollection { _seal: () }
    }
}

/// The kitchen's processes P1..P5 (SPEC.md §5): pure by-value
/// transformations (R1, R2). They mint quantity-bearing values, so they live
/// inside the resource family's module (F-031).
///
/// The person is threaded loosely through P1, P2, P4 and P5 (R9, F-024) and
/// pointedly **not** through P3 — the kettle needs no person while boiling,
/// which is what makes SPEC.md §6's ordering freedom real. Per F-048 the
/// person's time is drawn by **adjacent** `draw_time` processes in the flow
/// (never inside these processes), and each draw's labour is recorded into
/// the single `History` attributed to the process name (R16).
pub mod processes {
    use super::{
        BoilingKettle, BrewPermit, ColdWater, CouncilCollection, Electricity, FilledKettle,
        FoodWaste, FoodWasteBin, Kettle, LoadedPot, PhantomData, PotOfTea, SpentTeabag, Teapot,
        ThreeDryBags, ThreeSpentBags, WasteHeat,
    };
    // (The Boiling/LoadedToBrew methods need no import here: on a generic
    // parameter they resolve through the requirement bound's supertrait.)
    // One line on purpose: the traceability grep (F-021) skips `use` lines,
    // but only when the line itself starts with `use`.
    use crate::requirements::{Req006PouredAtTheBoil, Req007LoadedWithThreeBags, Req008FoodWasteBinOnly, Req009HeatToKitchenAir};
    use model_core::boundary::{ConsumeList, Consumer, SupplyN, send_list, send_to};
    use model_core::common::Person;
    use model_core::list::{Cons, Len, Nil};
    use model_core::nat::Add;
    use model_core::nat::aliases::N3;

    /// P1 — fills the kettle with the drawn cold water (SPEC.md P1). The
    /// person and the kettle are moved in and the person comes back with the
    /// filled kettle (R2); the water's mass continues structurally — the
    /// filled kettle carries the same `G` (mass 1500 = 1500, checked by the
    /// shared const parameter rather than an assert). The person's 30 000 ms
    /// is drawn by the adjacent `draw_time` in the flow (F-048).
    pub fn fill_kettle<const B: u64, const G: u64>(
        person: Person<B>,
        kettle: Kettle,
        water: ColdWater<G>,
    ) -> (Person<B>, FilledKettle<G>) {
        // Conserving transform: the water's mass continues inside the kettle.
        water.defuse();
        let Kettle { _seal: () } = kettle;
        (person, FilledKettle::mint())
    }

    /// P3 — boils the kettle (SPEC.md P3): the filled kettle plus `DRAW_J`
    /// joules from the grid become the boiling kettle (same water mass `G`,
    /// `EMBODIED_J` embodied) plus `HEAT_J` of kettle-loss waste heat.
    /// **No person** (the kettle is automatic) — this is what makes the
    /// ordering freedom in SPEC.md §6 real.
    ///
    /// Energy conservation (`EMBODIED_J + HEAT_J == DRAW_J`; SPEC.md P3:
    /// 550 000 = 500 000 + 50 000) is checked at compile time; the check
    /// fires at monomorphization (F-001): `cargo check` and editor
    /// diagnostics will not show a violation — `cargo build`/`cargo test`
    /// do. Mass is structural (the same `G` flows through).
    ///
    /// Regression (R4 policy: conservation violations are rustdoc
    /// `compile_fail` doc-tests, never trybuild cases, F-003): boiling must
    /// not lose energy — 500 000 embodied + 60 000 heat ≠ 550 000 drawn:
    ///
    /// ```compile_fail
    /// use cs1_pot_of_tea::resources::boundary::{draw_cold_water, draw_grid_energy, new_grid_socket, new_kettle, new_mains_tap};
    /// use cs1_pot_of_tea::resources::processes::{boil, fill_kettle};
    /// use model_core::common::boundary::new_person;
    ///
    /// let person = new_person::<300_000>();
    /// let (water, tap) = draw_cold_water::<1500>(new_mains_tap());
    /// let (person, filled) = fill_kettle(person, new_kettle(), water);
    /// let (energy, socket) = draw_grid_energy::<550_000>(new_grid_socket());
    /// let (boiling, heat) = boil::<1500, 550_000, 500_000, 60_000>(filled, energy);
    /// ```
    pub fn boil<const G: u64, const DRAW_J: u64, const EMBODIED_J: u64, const HEAT_J: u64>(
        kettle: FilledKettle<G>,
        energy: Electricity<DRAW_J>,
    ) -> (BoilingKettle<G, EMBODIED_J>, WasteHeat<HEAT_J>) {
        const {
            assert!(
                EMBODIED_J + HEAT_J == DRAW_J,
                "energy conservation violated in boil (R15): the embodied energy plus the kettle's waste heat must sum exactly to the energy drawn from the grid"
            )
        };
        // Conserving transforms: the water continues at the same mass with
        // the embodied energy; the drawn energy continues as embodied + heat.
        kettle.defuse();
        energy.defuse();
        (BoilingKettle::mint(), WasteHeat::mint())
    }

    /// P2 — loads the pot with **exactly 3** teabags, taken one at a time
    /// from the box in a single `SupplyN<N3>` bound (R12, F-014; SPEC.md P2:
    /// the box comes back at 37). Items 3 = 3 is structural: the bound takes
    /// exactly three and the loaded state holds exactly three. The person's
    /// 20 000 ms is drawn by the adjacent `draw_time` in the flow (F-048).
    /// The where-clause restates REQ-007 for traceability (the loaded state
    /// this process produces is what satisfies it); the whole signature sits
    /// on one line because trace.sh attributes requirement bounds to the
    /// line they are written on (R10 rule 4, F-021).
    ///
    /// Satisfies: REQ-007
    pub fn load_pot<const B: u64, S: SupplyN<N3, Taken = ThreeDryBags>>(person: Person<B>, pot: Teapot, teabag_box: S) -> (Person<B>, LoadedPot, S::Rest) where LoadedPot: Req007LoadedWithThreeBags {
        let (Cons(b1, Cons(b2, Cons(b3, Nil))), rest) = teabag_box.supply_n();
        // Conserving transform: the vessel and the three real bags continue
        // inside the loaded pot (mint is the conserving combinator — it only
        // wraps values passed in by value, R1).
        let Teapot { _seal: () } = pot;
        (person, LoadedPot::mint((b1, b2, b3)), rest)
    }

    /// P4 — pours the boiling kettle into the loaded pot and brews
    /// (SPEC.md P4). Style A throughout (F-048): every constrained input is
    /// a **requirement-trait bound**, so the wrong resource fails with the
    /// REQ-phrased `on_unimplemented` message (F-044) — REQ-006 on the
    /// kettle (only the boiling state pours), REQ-007 on the pot (only the
    /// 3-bag loaded state brews), REQ-008 on the bin (the spent bags exit
    /// only to the food-waste bin, fed inside this process), REQ-009 on the
    /// air (the steeping losses are accounted to the kitchen air inside this
    /// process). The quantities the asserts need travel on the
    /// characteristic traits as associated consts (R6/R7), and the
    /// extractions are permit-gated ([`BrewPermit`]) so only this process
    /// can realise them. The person's 15 000 ms is drawn by the adjacent
    /// `draw_time` in the flow (F-048).
    ///
    /// Conservation is one compile-time assert per dimension (R3, R15,
    /// F-033), with the caller stating the splits (F-022):
    /// * mass: boiling water + dry bags = tea + spent bags
    ///   (SPEC.md P4: 1500 + 9 = 1473 + 36);
    /// * energy: embodied in = embodied in the tea + steeping heat
    ///   (SPEC.md P4: 500 000 = 480 000 + 20 000).
    ///
    /// Both fire at monomorphization (F-001): invisible to `cargo check`,
    /// caught by `cargo build`/`cargo test`. Regression (R4 policy, F-003):
    /// brewing cannot keep the water's full mass in the pot — the bags
    /// absorb 27 g, so 1500 + 9 ≠ 1500 + 36:
    ///
    /// ```compile_fail
    /// use cs1_pot_of_tea::resources::boundary::{draw_cold_water, draw_grid_energy, full_teabag_box, new_food_waste_bin, new_grid_socket, new_kettle, new_kitchen_air, new_mains_tap, new_teapot};
    /// use cs1_pot_of_tea::resources::processes::{boil, fill_kettle, load_pot, pour_and_brew};
    /// use model_core::common::boundary::new_person;
    /// use model_core::nat::aliases::{N10, N40};
    ///
    /// let person = new_person::<300_000>();
    /// let (water, tap) = draw_cold_water::<1500>(new_mains_tap());
    /// let (person, filled) = fill_kettle(person, new_kettle(), water);
    /// let (energy, socket) = draw_grid_energy::<550_000>(new_grid_socket());
    /// let (boiling, kettle_heat) = boil::<1500, 550_000, 500_000, 50_000>(filled, energy);
    /// let (person, pot, rest) = load_pot(person, new_teapot(), full_teabag_box::<N40>());
    /// let out = pour_and_brew::<_, 1500, 480_000, 20_000, _, _, _, _>(person, boiling, pot, new_food_waste_bin::<N10>(), new_kitchen_air());
    /// ```
    ///
    /// Satisfies: REQ-006, REQ-008, REQ-009
    pub fn pour_and_brew<const B: u64, const TEA_G: u64, const TEA_E: u64, const HEAT_J: u64, K: Req006PouredAtTheBoil, P: Req007LoadedWithThreeBags, BinC: Req008FoodWasteBinOnly + ConsumeList<ThreeSpentBags>, Air: Req009HeatToKitchenAir + Consumer<WasteHeat<HEAT_J>>>(person: Person<B>, kettle: K, pot: P, bin: BinC, air: Air) -> (Person<B>, PotOfTea<TEA_G, TEA_E>, Kettle, <BinC as ConsumeList<ThreeSpentBags>>::Next, <Air as Consumer<WasteHeat<HEAT_J>>>::Next) {
        const {
            assert!(
                K::WATER_G + P::DRY_G == TEA_G + P::SPENT_G,
                "mass conservation violated in pour_and_brew (R3): the boiling water plus the dry teabags must sum exactly to the pot of tea plus the spent teabags"
            )
        };
        const {
            assert!(
                K::EMBODIED_J == TEA_E + HEAT_J,
                "energy conservation violated in pour_and_brew (R15): the kettle's embodied energy must sum exactly to the tea's embodied energy plus the steeping waste heat"
            )
        };
        // The permit-gated conserving extractions (R1): the kettle returns
        // to its empty state, its water and energy continuing as the tea and
        // heat minted below; the pot's 3 dry bags continue as 3 spent bags.
        let empty_kettle = kettle.pour_away(BrewPermit { _seal: () });
        let (bag1, bag2, bag3) = pot.steep(BrewPermit { _seal: () });
        // The spent bags exit ONLY to the food-waste bin (REQ-008), one
        // consume per bag (R12, F-014), and the steeping losses go to the
        // kitchen air (REQ-009), both inside this process per SPEC.md P4.
        let bin = send_list(bin, Cons(bag1, Cons(bag2, Cons(bag3, Nil))));
        let air = send_to(air, WasteHeat::mint());
        (person, PotOfTea::mint(), empty_kettle, bin, air)
    }

    /// Hands one quantity of waste heat to the kitchen-air sink — the only
    /// sanctioned route for waste heat out of a flow, and the process form
    /// of REQ-009 (SPEC.md P3's kettle losses go through it; P4's steeping
    /// losses are vented inside `pour_and_brew`). Generic over the consumer
    /// (R12, F-015): a wrong sink produces the REQ-phrased trait-bound
    /// error.
    ///
    /// Satisfies: REQ-009
    pub fn vent_heat<const E_J: u64, C: Req009HeatToKitchenAir + Consumer<WasteHeat<E_J>>>(sink: C, heat: WasteHeat<E_J>) -> C::Next {
        sink.consume(heat)
    }

    /// Recursively defuses the bin's kept spent teabags as they leave to the
    /// council collection, summing their mass. Private: together with
    /// [`empty_bin`] this is the only exit for spent teabags (R1, F-039).
    trait Dispose {
        /// The total mass of the kept bags, in grams (R7).
        const MASS_G: u64;
        fn dispose(self);
    }
    impl Dispose for Nil {
        const MASS_G: u64 = 0;
        fn dispose(self) {}
    }
    impl<T: Dispose> Dispose for Cons<SpentTeabag, T> {
        const MASS_G: u64 = SpentTeabag::MASS_G + T::MASS_G;
        fn dispose(self) {
            let Cons(bag, tail) = self;
            bag.defuse();
            tail.dispose();
        }
    }

    /// Public-in-signature but unimplementable-outside wrapper over the
    /// private disposal machinery (the sealed-trait pattern, F-026), so
    /// [`empty_bin`] can name it without letting outside code defuse spent
    /// teabags.
    pub trait DisposeSealed: sealed_dispose::Sealed {
        /// The total mass of the bin's kept spent teabags, in grams (R7).
        const MASS_G: u64;
        #[doc(hidden)]
        fn dispose_all(self);
    }
    impl<L: Dispose + sealed_dispose::Sealed> DisposeSealed for L {
        const MASS_G: u64 = L::MASS_G;
        fn dispose_all(self) {
            self.dispose()
        }
    }
    mod sealed_dispose {
        use super::super::SpentTeabag;
        use model_core::list::{Cons, Nil};
        pub trait Sealed {}
        impl Sealed for Nil {}
        impl<T: Sealed> Sealed for Cons<SpentTeabag, T> {}
    }

    /// P5 — empties the bin (SPEC.md P5, the F-039 pattern): the bin's kept
    /// contents (CS-1: 3 spent bags, 36 g) are released only through this
    /// sealed disposal path — their tripwires are defused inside the privacy
    /// boundary and their mass continues as one [`FoodWaste`] handed to the
    /// council collection. The bin comes back **empty with its capacity
    /// restored** (type-level `Space + Contents::Length`, R12); the person's
    /// 10 000 ms is drawn by the adjacent `draw_time` in the flow (F-048).
    /// Mass conservation (`WASTE_G` = the kept bags' total; SPEC.md P5:
    /// 36 = 36) is a compile-time assert over the contents list's summed
    /// mass; the caller states the total (F-022). The where-clause restates
    /// REQ-008 for traceability.
    ///
    /// Satisfies: REQ-008
    pub fn empty_bin<const B: u64, const WASTE_G: u64, Space: Add<<C as Len>::Length>, C: Len + DisposeSealed>(person: Person<B>, bin: FoodWasteBin<Space, C>, council: CouncilCollection) -> (Person<B>, FoodWasteBin<<Space as Add<<C as Len>::Length>>::Sum>, CouncilCollection) where FoodWasteBin<Space, C>: Req008FoodWasteBinOnly {
        const {
            assert!(
                C::MASS_G == WASTE_G,
                "mass conservation violated in empty_bin (R3): the stated food-waste mass must equal the total mass of the spent teabags the bin kept"
            )
        };
        let FoodWasteBin {
            contents,
            _space: PhantomData,
        } = bin;
        // The sealed disposal (F-039): the kept bags' tripwires are defused
        // here, inside the privacy boundary, and their mass continues as the
        // food waste handed to the council below (R1).
        contents.dispose_all();
        let council = send_to(council, FoodWaste::<WASTE_G>::mint());
        (
            person,
            FoodWasteBin {
                contents: Nil,
                _space: PhantomData,
            },
            council,
        )
    }
}

#[cfg(test)]
mod tests {
    //! Per-process unit tests (R5): every process turns specific inputs into
    //! the expected outputs with nothing left unaccounted for. These tests
    //! sit inside the privacy boundary, so they may mint fixtures and defuse
    //! outputs directly; downstream-style accounting is exercised by the
    //! integration tests in `tests/flows.rs`.

    use super::boundary::{
        BagsOf, draw_cold_water, draw_grid_energy, full_teabag_box, new_council_collection,
        new_drinker, new_food_waste_bin, new_grid_socket, new_kettle, new_kitchen_air,
        new_mains_tap, new_teapot,
    };
    use super::processes::{boil, empty_bin, fill_kettle, load_pot, pour_and_brew, vent_heat};
    use super::{
        BoilingKettle, ColdWater, DryTeabag, EmptyTeabagBox, FilledKettle, FoodWasteBin, Kettle,
        LoadedPot, PotOfTea, SpentTeabag, TeabagBox, ThreeSpentBags, WasteHeat,
    };
    use crate::characteristics::LoadedToBrew;
    use model_core::boundary::{send_to, take_one};
    use model_core::common::boundary::new_person;
    use model_core::nat::aliases::{N1, N7, N10, N37, N39, N40};

    /// P1: the drawn water's mass continues structurally into the filled
    /// kettle (SPEC.md P1: mass 1500 = 1500), and the person and tap come
    /// back (R2).
    #[test]
    fn fill_kettle_conserves_the_drawn_water() {
        let person = new_person::<300_000>();
        let (water, tap) = draw_cold_water::<1500>(new_mains_tap());
        assert_eq!(ColdWater::<1500>::VALUE, 1500);
        assert_eq!(ColdWater::<1500>::UNIT, "grams");
        let (person, filled) = fill_kettle(person, new_kettle(), water);
        assert_eq!(FilledKettle::<1500>::VALUE, 1500);
        // Inside the privacy boundary this test may defuse directly.
        filled.defuse();
        let _reusables = (person, tap);
    }

    /// P3: energy balances (SPEC.md P3: 550 000 = 500 000 + 50 000), the
    /// water's mass flows through unchanged, and no person is involved.
    #[test]
    fn boil_conserves_energy_and_needs_no_person() {
        let (energy, socket) = draw_grid_energy::<550_000>(new_grid_socket());
        let filled: FilledKettle<1500> = FilledKettle::mint();
        let (boiling, heat) = boil::<1500, 550_000, 500_000, 50_000>(filled, energy);
        assert_eq!(
            BoilingKettle::<1500, 500_000>::VALUE + WasteHeat::<50_000>::VALUE,
            550_000
        );
        assert_eq!(BoilingKettle::<1500, 500_000>::WATER_G, 1500);
        boiling.defuse();
        heat.defuse();
        let _socket = socket;
    }

    /// P3's kettle losses reach the kitchen air through the REQ-009 process.
    ///
    /// Verifies: REQ-009
    #[test]
    fn vent_heat_accounts_heat_to_the_kitchen_air() {
        let heat: WasteHeat<50_000> = WasteHeat::mint();
        let _air = vent_heat(new_kitchen_air(), heat);
    }

    /// P2: exactly three bags leave the box one at a time (R12, F-014), the
    /// box comes back at 37 (SPEC.md P2), and the loaded state carries the
    /// bags' masses (9 g dry in, 36 g spent out, SPEC.md §3).
    ///
    /// Verifies: REQ-007
    #[test]
    fn load_pot_takes_exactly_three_bags_and_returns_the_box_at_37() {
        let person = new_person::<300_000>();
        let the_box = full_teabag_box::<N40>();
        assert_eq!(TeabagBox::<BagsOf<N40>>::COUNT, 40);
        let (person, pot, rest) = load_pot(person, new_teapot(), the_box);
        let _box_at_37: TeabagBox<BagsOf<N37>> = rest;
        assert_eq!(TeabagBox::<BagsOf<N37>>::COUNT, 37);
        assert_eq!(<LoadedPot as LoadedToBrew>::DRY_G, 9);
        assert_eq!(<LoadedPot as LoadedToBrew>::SPENT_G, 36);
        pot.defuse();
        let _person = person;
    }

    /// The box supplier shortens by one per supply (R12) and its empty state
    /// is a distinct resource.
    #[test]
    fn supplying_shortens_the_box_by_one() {
        let the_box = full_teabag_box::<N1>();
        let (bag, empty) = take_one(the_box);
        let _empty: EmptyTeabagBox = empty; // the exhausted box is a new resource
        assert_eq!(TeabagBox::<BagsOf<N39>>::COUNT, 39);
        // Inside the privacy boundary the untripwired bag can be kept to the
        // end of the test as its accounting.
        let _accounted = bag;
    }

    /// P4: mass and energy balance (SPEC.md P4: 1500 + 9 = 1473 + 36 and
    /// 500 000 = 480 000 + 20 000), the kettle comes back empty, the spent
    /// bags land in the bin (space 10 → 7, contents kept, F-016), the
    /// steeping heat reaches the air, and the tea ships to the drinker.
    ///
    /// Verifies: REQ-006, REQ-007, REQ-008, REQ-009
    #[test]
    fn pour_and_brew_balances_mass_and_energy() {
        let person = new_person::<300_000>();
        let boiling: BoilingKettle<1500, 500_000> = BoilingKettle::mint();
        let pot = LoadedPot::mint((DryTeabag::mint(), DryTeabag::mint(), DryTeabag::mint()));
        let bin = new_food_waste_bin::<N10>();
        let air = new_kitchen_air();
        let (person, tea, kettle, bin, air) =
            pour_and_brew::<_, 1473, 480_000, 20_000, _, _, _, _>(person, boiling, pot, bin, air);
        // The spec's P4 balances, recovered from the constants (R7).
        assert_eq!(1500 + 9, PotOfTea::<1473, 480_000>::TEA_G + 36);
        assert_eq!(
            PotOfTea::<1473, 480_000>::VALUE + WasteHeat::<20_000>::VALUE,
            500_000
        );
        // The bin kept the three real spent bags (F-016): space down to 7.
        let full_bin: FoodWasteBin<N7, ThreeSpentBags> = bin;
        assert_eq!(FoodWasteBin::<N7, ThreeSpentBags>::HELD, 3);
        let _kettle_back_empty: Kettle = kettle;
        let _drinker = send_to(new_drinker(), tea);
        // The bin's kept tripwired bags leave through the sealed disposal
        // path (F-039).
        let (person, bin, council) =
            empty_bin::<_, 36, _, _>(person, full_bin, new_council_collection());
        let _bin_restored: FoodWasteBin<N10> = bin;
        let _accounted = (person, air, council);
    }

    /// P5: the sealed disposal path (F-039) defuses the kept bags, ships
    /// their mass to the council (SPEC.md P5: mass 36 = 36), and restores
    /// the bin's capacity at the type level.
    ///
    /// Verifies: REQ-008
    #[test]
    fn empty_bin_restores_capacity_and_ships_the_waste() {
        let bin = new_food_waste_bin::<N10>();
        let bin = send_to(bin, SpentTeabag::mint());
        let bin = send_to(bin, SpentTeabag::mint());
        let bin = send_to(bin, SpentTeabag::mint());
        assert_eq!(FoodWasteBin::<N7, ThreeSpentBags>::HELD, 3);
        let person = new_person::<300_000>();
        let (person, bin, council) =
            empty_bin::<_, 36, _, _>(person, bin, new_council_collection());
        let bin: FoodWasteBin<N10> = bin; // capacity 10 restored (SPEC.md §6)
        assert_eq!(FoodWasteBin::<N10>::HELD, 0);
        let _accounted = (person, bin, council);
    }

    /// An abandoned spent teabag is caught by its tripwire at test time (R1
    /// layer 2, F-008): no compile-time layer sees a named-and-used binding
    /// that never reaches the bin.
    #[test]
    #[should_panic(expected = "resource leak: SpentTeabag dropped without being consumed")]
    fn abandoned_spent_teabag_trips_the_tripwire() {
        let bag = SpentTeabag::mint();
        let _never_reaches_the_bin = bag;
    }
}
