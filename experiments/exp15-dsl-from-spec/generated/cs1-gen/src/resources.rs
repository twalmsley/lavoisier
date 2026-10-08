//! GENERATED sealed resource family (R1), boundary (R12) and processes
//! (SPEC.md §3, §4, §5). Layout per F-006/F-031: types here, with `boundary`
//! and `processes` child modules.

use core::marker::PhantomData;
use model_core::boundary::{Consumer, Supplier};
use model_core::list::{Cons, Len, Nil};
use model_core::nat::{Succ, Zero};

// ── SPEC-HOLE U-07 ──────────────────────────────────────────
// The spec states one worked instance (1500 g, 550 000 J, …). The generator
// emits those literals in signatures and types; the real crate generalized most
// processes over const parameters (boil<G, DRAW_J, EMBODIED_J, HEAT_J>) with the
// spec's numbers appearing only at call sites. Where to generalize is a design
// judgement the spec does not state.
#[cfg(feature = "deny-holes")]
compile_error!("SPEC-HOLE U-07: magnitudes are generated as the spec's literal numbers, not generic const parameters");

model_core::container_resource! {
    /// Generated from SPEC.md §3 line 41: resource 'water', state 'cold'.
    ColdWater,
    unit = "grams",
    must_use = "ColdWater is a conserved resource: pass it on or hand it to a Consumer"
}

model_core::container_resource! {
    /// Generated from SPEC.md §3 line 42: resource 'electrical energy'.
    ElectricalEnergy,
    unit = "joules",
    must_use = "ElectricalEnergy is a conserved resource: pass it on or hand it to a Consumer"
}

model_core::consumable_resource! {
    /// Generated from SPEC.md §3 line 43: resource 'teabag', state 'dry'.
    /// `no_tripwire`: minted into a boundary container that keeps it (F-040 rule, inferred — see the generation report).
    DryTeabag,
    must_use = "DryTeabag is a conserved resource: pass it on or hand it to a Consumer",
    no_tripwire
}

impl DryTeabag {
    /// Mass in grams (R7), from the §3 characteristics column (line 43).
    pub const MASS_G: u64 = 3;
}

model_core::consumable_resource! {
    /// Generated from SPEC.md §3 line 43: resource 'teabag', state 'spent'.
    SpentTeabag,
    must_use = "SpentTeabag is a conserved resource: pass it on or hand it to a Consumer"
}

impl SpentTeabag {
    /// Mass in grams (R7), from the §3 characteristics column (line 43).
    pub const MASS_G: u64 = 12;
}

model_core::reusable_resource! {
    /// Generated from SPEC.md §3 line 44: resource 'kettle'.
    Kettle,
    must_use = "Kettle is a reusable resource: pass it on or return it to the caller"
}

model_core::container_resource! {
    /// Generated from SPEC.md §3 line 44: resource 'kettle', state 'filled'.
    FilledKettle,
    unit = "grams",
    must_use = "FilledKettle is a conserved resource: pass it on or hand it to a Consumer"
}

model_core::container_resource! {
    /// Generated from SPEC.md §3 line 44: resource 'kettle', state 'boiling'.
    BoilingKettle<const MASS_G: u64>,
    unit = "joules",
    must_use = "BoilingKettle is a conserved resource: pass it on or hand it to a Consumer"
}

model_core::reusable_resource! {
    /// Generated from SPEC.md §3 line 45: resource 'teapot'.
    Teapot,
    must_use = "Teapot is a reusable resource: pass it on or return it to the caller"
}

model_core::consumable_resource! {
    /// Generated from SPEC.md §3 line 45: resource 'teapot', state 'loaded'.
    LoadedTeapot {
        held: (DryTeabag, DryTeabag, DryTeabag),
    },
    must_use = "LoadedTeapot is a conserved resource: pass it on or hand it to a Consumer"
}

model_core::container_resource! {
    /// Generated from SPEC.md §3 line 45: resource 'teapot', state 'pot of tea'.
    PotOfTea<const MASS_G: u64>,
    unit = "joules",
    must_use = "PotOfTea is a conserved resource: pass it on or hand it to a Consumer"
}

model_core::container_resource! {
    /// Generated from SPEC.md §3 line 47: resource 'waste heat'.
    WasteHeat,
    unit = "joules",
    must_use = "WasteHeat is a conserved resource: pass it on or hand it to a Consumer"
}

model_core::reusable_resource! {
    /// Generated from SPEC.md §4 line 55: boundary source drawn via [`boundary::draw_cold_water`].
    ///
    /// Placeholder: unbounded boundary source (R15).
    MainsTap,
    must_use = "MainsTap is a boundary resource: pass it on like any other resource"
}

model_core::reusable_resource! {
    /// Generated from SPEC.md §4 line 56: boundary source drawn via [`boundary::draw_electrical_energy`].
    ///
    /// Placeholder: unbounded boundary source (R15).
    GridSocket,
    must_use = "GridSocket is a boundary resource: pass it on like any other resource"
}

/// Generated from SPEC.md §4 line 57: a supplier of DryTeabag at the system
/// boundary (R12), capacity 40. Contents are a type-level list of real
/// values; the count is the list's length.
#[must_use = "BoxOfTeabags is a boundary resource: pass it on like any other resource"]
pub struct BoxOfTeabags<Items>(Items);

/// The exhausted supplier: a distinct resource that must itself be accounted for (R12).
pub type EmptyBoxOfTeabags = BoxOfTeabags<Nil>;

/// `Supplier` only for a non-empty list (R12): supplying from an empty
/// BoxOfTeabags is a compile error with model-core's `on_unimplemented` message.
impl<H, T> Supplier for BoxOfTeabags<Cons<H, T>> {
    type Item = H;
    type Next = BoxOfTeabags<T>;
    fn supply(self) -> (H, BoxOfTeabags<T>) {
        let Cons(head, tail) = self.0;
        (head, BoxOfTeabags(tail))
    }
}

impl<Items: Len> BoxOfTeabags<Items> {
    /// How many items the supplier holds — the length of its contents list (R7, R12).
    pub const COUNT: u64 = Items::LEN;
}

/// Generated from SPEC.md §4 line 65: a bounded consumer of SpentTeabag
/// (capacity 10) that KEEPS what it consumes (R12; the decreasing space
/// parameter is the F-034 hard rule).
#[must_use = "FoodWasteBin is a resource (R12): pass it on, or return it to the boundary"]
pub struct FoodWasteBin<Space, Contents = Nil> {
    contents: Contents,
    _space: PhantomData<Space>,
}

/// The full state: a distinct resource that must itself be accounted for (R12).
pub type FullFoodWasteBin<Contents> = FoodWasteBin<Zero, Contents>;

/// `Consumer` only while space remains (R12, F-034): space goes down by one
/// and the real consumed object is kept at the front of the contents list.
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
    /// How many items the consumer holds — the length of its contents list (R7, R12).
    pub const HELD: u64 = Contents::LEN;
}

/// Generated from SPEC.md §4 line 64: an unbounded boundary sink
/// (`Next = Self`, R15, F-029 — legal only at the system boundary; it
/// necessarily discards what it consumes).
///
/// Placeholder: placeholder.
#[must_use = "Drinker is a boundary resource: pass it on like any other resource"]
pub struct Drinker {
    _seal: (),
}

/// The sink accepts PotOfTea at any magnitude; `Next = Self` (R15).
impl<const C0: u64, const C1: u64> Consumer<PotOfTea<C0, C1>> for Drinker {
    type Next = Drinker;
    fn consume(self, item: PotOfTea<C0, C1>) -> Drinker {
        item.defuse(); // sanctioned consumer role (F-008); unbounded sink discards (F-029)
        self
    }
}

/// Generated from SPEC.md §4 line 65: an unbounded boundary sink
/// (`Next = Self`, R15, F-029 — legal only at the system boundary; it
/// necessarily discards what it consumes).
///
/// Placeholder: bin real; collection placeholder.
#[must_use = "CouncilFoodWasteCollection is a boundary resource: pass it on like any other resource"]
pub struct CouncilFoodWasteCollection {
    _seal: (),
}

/// The sink accepts WasteHeat at any magnitude; `Next = Self` (R15).
impl<const C0: u64> Consumer<WasteHeat<C0>> for CouncilFoodWasteCollection {
    type Next = CouncilFoodWasteCollection;
    fn consume(self, item: WasteHeat<C0>) -> CouncilFoodWasteCollection {
        item.defuse(); // sanctioned consumer role (F-008); unbounded sink discards (F-029)
        self
    }
}

/// Generated from SPEC.md §4 line 66: an unbounded boundary sink
/// (`Next = Self`, R15, F-029 — legal only at the system boundary; it
/// necessarily discards what it consumes).
///
/// Placeholder: placeholder.
#[must_use = "KitchenAir is a boundary resource: pass it on like any other resource"]
pub struct KitchenAir {
    _seal: (),
}

/// The sink accepts WasteHeat at any magnitude; `Next = Self` (R15).
impl<const C0: u64> Consumer<WasteHeat<C0>> for KitchenAir {
    type Next = KitchenAir;
    fn consume(self, item: WasteHeat<C0>) -> KitchenAir {
        item.defuse(); // sanctioned consumer role (F-008); unbounded sink discards (F-029)
        self
    }
}

/// The creation boundary (R12): the only production code where boundary
/// objects and supplied contents come into existence (SPEC.md §4).
pub mod boundary {
    use super::{BoxOfTeabags, ColdWater, CouncilFoodWasteCollection, Drinker, DryTeabag, ElectricalEnergy, FoodWasteBin, GridSocket, Kettle, KitchenAir, MainsTap, PhantomData, Teapot};
    use model_core::list::{Cons, Nil};
    use model_core::nat::{Succ, Zero};

    /// Kettle enters the model at flow start (R12).
    ///
    /// Placeholder: setup at flow start (SPEC.md §4).
    pub fn new_kettle() -> Kettle {
        Kettle::mint()
    }

    /// Teapot enters the model at flow start (R12).
    ///
    /// Placeholder: setup at flow start (SPEC.md §4).
    pub fn new_teapot() -> Teapot {
        Teapot::mint()
    }

    /// MainsTap enters the model (R12).
    ///
    /// Placeholder: unbounded boundary source (SPEC.md §4 line 55).
    pub fn new_mains_tap() -> MainsTap {
        MainsTap::mint()
    }

    /// Draws `TAKE` of ColdWater from the boundary (R15: an unbounded source of
    /// continuous material is a draw-style boundary process, never a `Supplier`
    /// impl, F-028). The source object is returned (R2).
    ///
    /// Placeholder: unbounded boundary source (SPEC.md §4 line 55).
    pub fn draw_cold_water<const TAKE: u64>(source: MainsTap) -> (ColdWater<TAKE>, MainsTap) {
        (ColdWater::mint(), source)
    }

    /// GridSocket enters the model (R12).
    ///
    /// Placeholder: unbounded boundary source (SPEC.md §4 line 56).
    pub fn new_grid_socket() -> GridSocket {
        GridSocket::mint()
    }

    /// Draws `TAKE` of ElectricalEnergy from the boundary (R15: an unbounded source of
    /// continuous material is a draw-style boundary process, never a `Supplier`
    /// impl, F-028). The source object is returned (R2).
    ///
    /// Placeholder: unbounded boundary source (SPEC.md §4 line 56).
    pub fn draw_electrical_energy<const TAKE: u64>(source: GridSocket) -> (ElectricalEnergy<TAKE>, GridSocket) {
        (ElectricalEnergy::mint(), source)
    }

    /// Maps a type-level count to the list type of that many DryTeabags.
    pub trait Replicate {
        /// The list type holding `Self`-many items.
        type List;
    }
    impl Replicate for Zero {
        type List = Nil;
    }
    impl<N: Replicate> Replicate for Succ<N> {
        type List = Cons<DryTeabag, N::List>;
    }

    /// Shorthand for [`Replicate::List`].
    pub type ItemsOf<N> = <N as Replicate>::List;

    /// Builds a list of real items. Private: the only place DryTeabags come
    /// into existence (R1), reachable only through [`full_box_of_teabags`].
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
        use super::super::DryTeabag;
        use model_core::list::{Cons, Nil};
        pub trait Sealed {}
        impl Sealed for Nil {}
        impl<T: Sealed> Sealed for Cons<DryTeabag, T> {}
    }

    /// The fill function (R12): a full supplier of `N` real items enters the
    /// model here (SPEC.md §4 line 57: capacity 40).
    ///
    /// Placeholder: vendor not modelled (SPEC.md §4).
    pub fn full_box_of_teabags<N: Replicate>() -> BoxOfTeabags<ItemsOf<N>>
    where
        ItemsOf<N>: FillSealed,
    {
        BoxOfTeabags(FillSealed::fill_sealed())
    }

    /// An empty FoodWasteBin with `Space` slots enters the model (SPEC.md §4 line 65).
    /// Creating an empty consumer brings no resources into existence (R12).
    pub fn new_food_waste_bin<Space>() -> FoodWasteBin<Space, Nil> {
        FoodWasteBin {
            contents: Nil,
            _space: PhantomData,
        }
    }

    /// Drinker enters the model (R12). An empty unbounded sink holds nothing.
    pub fn new_drinker() -> Drinker {
        Drinker { _seal: () }
    }

    /// CouncilFoodWasteCollection enters the model (R12). An empty unbounded sink holds nothing.
    pub fn new_council_food_waste_collection() -> CouncilFoodWasteCollection {
        CouncilFoodWasteCollection { _seal: () }
    }

    /// KitchenAir enters the model (R12). An empty unbounded sink holds nothing.
    pub fn new_kitchen_air() -> KitchenAir {
        KitchenAir { _seal: () }
    }

}

/// GENERATED processes (SPEC.md §5): by-value conserving transformations
/// (R1, R2), living inside the resource family's privacy boundary (F-031).
pub mod processes {
    use super::{BoilingKettle, ColdWater, CouncilFoodWasteCollection, DryTeabag, ElectricalEnergy, FilledKettle, FoodWasteBin, Kettle, LoadedTeapot, PhantomData, PotOfTea, SpentTeabag, Teapot, WasteHeat};
    use model_core::boundary::{SupplyN, send_to};
    use model_core::common::Person;
    use model_core::list::{Cons, Nil};
    use model_core::nat::aliases::{N10, N3, N7};

    /// P1 — Fill the kettle (SPEC.md §5 P1, line 71).
    /// The person's 30 000 ms is drawn by the **adjacent** `draw_time` in the flow,
    /// recorded to the History under this process's name (F-048 — the template's
    /// stated convention); the process takes and returns the person unchanged.
    pub fn fill_kettle<const B: u64>(person: Person<B>, kettle: Kettle, cold_water: ColdWater<1_500>) -> (Person<B>, FilledKettle<1_500>) {
        // mass balance 'mass 1500 = 1500' (SPEC.md line 75): structural — carried by the shared magnitudes, no assert needed.
        cold_water.defuse(); // conserving transform: the magnitude continues in the outputs
        let Kettle { _seal: () } = kettle; // the vessel continues as the produced state
        (
            person,
            FilledKettle::mint(),
        )
    }

    /// P2 — Load the pot (SPEC.md §5 P2, line 78).
    /// The person's 20 000 ms is drawn by the **adjacent** `draw_time` in the flow,
    /// recorded to the History under this process's name (F-048 — the template's
    /// stated convention); the process takes and returns the person unchanged.
    ///
    /// Satisfies: REQ-007
    // ── SPEC-HOLE U-08 ──────────────────────────────────────────
    // SPEC.md §5 P2 says this process satisfies REQ-007, but the generated signature
    // uses concrete types: the requirement-trait bounds, the characteristic consts
    // they carry, and the REQ-phrased wrong-resource errors (F-044) are all
    // implementation design the spec does not determine. Wrong-resource errors here
    // are plain E0308 type mismatches.
    #[cfg(feature = "deny-holes")]
    compile_error!("SPEC-HOLE U-08: load_pot: requirement bounds (R10 style A) are not derivable from the spec");
    pub fn load_pot<const B: u64, S: SupplyN<N3, Taken = Cons<DryTeabag, Cons<DryTeabag, Cons<DryTeabag, Nil>>>>>(person: Person<B>, teapot: Teapot, box_of_teabags: S) -> (Person<B>, LoadedTeapot, S::Rest) {
        // items balance 'items 3 = 3' (SPEC.md line 82): structural — carried by the shared magnitudes, no assert needed.
        let (Cons(i1, Cons(i2, Cons(i3, Nil))), rest) = box_of_teabags.supply_n(); // one at a time via SupplyN (R12, F-014)
        let Teapot { _seal: () } = teapot; // the vessel continues as the produced state
        (
            person,
            LoadedTeapot::mint((i1, i2, i3)) /* holds the 3 real DryTeabags */,
            rest,
        )
    }

    /// P3 — Boil (SPEC.md §5 P3, line 85).
    /// **No person** (SPEC.md Actor line): this is what creates the §6 ordering freedom.
    pub fn boil(filled_kettle: FilledKettle<1_500>, electrical_energy: ElectricalEnergy<550_000>) -> (BoilingKettle<1_500, 500_000>, WasteHeat<50_000>) {
        // mass balance 'mass 1500 = 1500' (SPEC.md line 91): structural — carried by the shared magnitudes, no assert needed.
        const {
            assert!(
                550_000 == 500_000 + 50_000,
                "energy conservation violated in boil (SPEC.md §5 P3 line 91): energy 550 000 = 500 000 + 50 000"
            )
        };
        filled_kettle.defuse(); // conserving transform: the magnitude continues in the outputs
        electrical_energy.defuse(); // conserving transform: the magnitude continues in the outputs
        (
            BoilingKettle::mint(),
            WasteHeat::mint(),
        )
    }

    /// P4 — Pour and brew (SPEC.md §5 P4, line 96).
    /// The person's 15 000 ms is drawn by the **adjacent** `draw_time` in the flow,
    /// recorded to the History under this process's name (F-048 — the template's
    /// stated convention); the process takes and returns the person unchanged.
    ///
    /// Satisfies: REQ-006, REQ-008, REQ-009
    // ── SPEC-HOLE U-09 ──────────────────────────────────────────
    // SPEC.md §5 P4 says this process satisfies REQ-006, REQ-008, REQ-009, but the generated signature
    // uses concrete types: the requirement-trait bounds, the characteristic consts
    // they carry, and the REQ-phrased wrong-resource errors (F-044) are all
    // implementation design the spec does not determine. Wrong-resource errors here
    // are plain E0308 type mismatches.
    #[cfg(feature = "deny-holes")]
    compile_error!("SPEC-HOLE U-09: pour_and_brew: requirement bounds (R10 style A) are not derivable from the spec");
    // ── SPEC-HOLE U-10 ──────────────────────────────────────────
    // SPEC.md §5 P4 lists the same items under Produces and Waste with no
    // 'Waste routing' field (the §8 feedback-item-3 ambiguity). Generated: the WEAK
    // reading — loose outputs routed by the flow. The real crate chose the STRONG
    // reading (the process takes its consumers as requirement-bounded parameters and
    // feeds them internally, so the waste never exists loose).
    #[cfg(feature = "deny-holes")]
    compile_error!("SPEC-HOLE U-10: pour_and_brew: waste routing is ambiguous (weak reading generated)");
    // ── SPEC-HOLE U-11 ──────────────────────────────────────────
    // The body below defuses consumed states and mints produced ones — conservation
    // holds only via the stated asserts, not by construction. The real crate routes
    // the magnitudes through sealed, permit-gated characteristic extractions
    // (`pour_away`/`steep` with a BrewPermit) so a free-standing transform cannot
    // vanish them; that machinery is pure implementation design.
    #[cfg(feature = "deny-holes")]
    compile_error!("SPEC-HOLE U-11: pour_and_brew: conserving extraction design is not derivable");
    pub fn pour_and_brew<const B: u64>(person: Person<B>, boiling_kettle: BoilingKettle<1_500, 500_000>, loaded_teapot: LoadedTeapot) -> (Person<B>, PotOfTea<1_473, 480_000>, SpentTeabag, SpentTeabag, SpentTeabag, Kettle, WasteHeat<20_000>) {
        const {
            assert!(
                1_500 + 9 == 1_473 + 36,
                "mass conservation violated in pour_and_brew (SPEC.md §5 P4 line 102): mass 1500 + 9 = 1473 + 36 (= 1509)"
            )
        };
        const {
            assert!(
                500_000 == 480_000 + 20_000,
                "energy conservation violated in pour_and_brew (SPEC.md §5 P4 line 102): energy 500 000 = 480 000 + 20 000"
            )
        };
        boiling_kettle.defuse(); // conserving transform: the magnitude continues in the outputs
        loaded_teapot.defuse(); // conserving transform: its magnitudes continue in the outputs (checked by the asserts above)
        (
            person,
            PotOfTea::mint(),
            SpentTeabag::mint(),
            SpentTeabag::mint(),
            SpentTeabag::mint(),
            Kettle::mint(),
            WasteHeat::mint(),
        )
    }

    /// P5 — Empty the bin (SPEC.md §5 P5, line 107).
    /// The person's 10 000 ms is drawn by the **adjacent** `draw_time` in the flow,
    /// recorded to the History under this process's name (F-048 — the template's
    /// stated convention); the process takes and returns the person unchanged.
    ///
    /// Satisfies: REQ-008
    // ── SPEC-HOLE U-12 ──────────────────────────────────────────
    // SPEC.md §5 P5 says this process satisfies REQ-008, but the generated signature
    // uses concrete types: the requirement-trait bounds, the characteristic consts
    // they carry, and the REQ-phrased wrong-resource errors (F-044) are all
    // implementation design the spec does not determine. Wrong-resource errors here
    // are plain E0308 type mismatches.
    #[cfg(feature = "deny-holes")]
    compile_error!("SPEC-HOLE U-12: empty_bin: requirement bounds (R10 style A) are not derivable from the spec");
    pub fn empty_bin<const B: u64>(person: Person<B>, food_waste_bin: FoodWasteBin<N7, Cons<SpentTeabag, Cons<SpentTeabag, Cons<SpentTeabag, Nil>>>>, council_food_waste_collection: CouncilFoodWasteCollection) -> (Person<B>, FoodWasteBin<N10>, CouncilFoodWasteCollection) {
        // mass balance 'mass 36 = 36' (SPEC.md line 113): structural — carried by the shared magnitudes, no assert needed.
        let FoodWasteBin {
            contents: Cons(i1, Cons(i2, Cons(i3, Nil))),
            _space: PhantomData,
        } = food_waste_bin;
        i1.defuse(); // sealed disposal path (F-039): the kept mass continues as WasteHeat below
        i2.defuse(); // sealed disposal path (F-039): the kept mass continues as WasteHeat below
        i3.defuse(); // sealed disposal path (F-039): the kept mass continues as WasteHeat below
        let council_food_waste_collection = send_to(council_food_waste_collection, WasteHeat::<36>::mint());
        (
            person,
            FoodWasteBin {
                contents: Nil,
                _space: PhantomData,
            },
            council_food_waste_collection,
        )
    }

}
