//! The kitchen slice, hand-written (R1 sealed family + boundary + processes
//! in one module so the kernel seals support destructuring, F-031/F-006).
//!
//! One type per processing state (R9): [`ColdWater`] enters through the
//! kettle states ([`Kettle`] → [`FilledKettle`] → [`BoilingKettle`]) and
//! leaves as [`HotWater`] with the [`Drinker`]; waste heat is accounted to
//! the [`KitchenAir`] sink (R15).

use model_core::boundary::Consumer;

// ---------------------------------------------------------------------------
// Characteristic (R6) and requirement (R10).
// ---------------------------------------------------------------------------

/// Seals [`Boiling`] (the classic sealed-trait pattern, F-026): only this
/// crate's boiling state may carry it.
pub(crate) mod sealed {
    /// Implemented only for this crate's boiling kettle state.
    pub trait Sealed {}
}

/// Characteristic (R6): the kettle is **at the boil** — implemented only by
/// the boiling kettle state (one type per state, R9). Carries the boiling
/// water's mass and embodied energy (R6/R7) and the permit-gated conserving
/// pour (F-054).
#[diagnostic::on_unimplemented(message = "`{Self}` is not a kettle at the boil (REQ-001: only the boiling state can be poured)", label = "a boiling kettle is required here", note = "boiling is a process (one type per state, R9): `boil` turns a `FilledKettle` into a `BoilingKettle`")]
pub trait Boiling: sealed::Sealed {
    /// The boiling water's mass, in grams (R7).
    const WATER_G: u64;
    /// The water's embodied energy, in joules (R7).
    const EMBODIED_J: u64;
    /// Conserving extraction: the kettle is poured out and returns to its
    /// empty state; callable only with a [`BrewPermit`] so the water and
    /// energy continue into outputs a process's asserts account for (R1).
    fn pour_away(self, permit: BrewPermit) -> Kettle;
}

model_core::requirement! {
    /// REQ-001: Tea must be brewed with boiling water (only the boiling state of the kettle can be poured).
    #[diagnostic::on_unimplemented(message = "this kettle may not be poured: `{Self}` is not at the boil (REQ-001)", label = "REQ-001: tea must be brewed with boiling water", note = "boiling is a process state (R9): `boil` turns a `FilledKettle` into a `BoilingKettle` - only that state can be poured")]
    pub trait Req001BoilingWater: (Boiling);
    assert = assert_req001;
}

// ---------------------------------------------------------------------------
// Continuous resources (R15) and the kettle states (R9).
// ---------------------------------------------------------------------------

model_core::container_resource! {
    /// Cold water drawn from the mains, in grams (R7). Enters the model only
    /// through [`boundary::draw_cold_water`] (R12, R15).
    ColdWater,
    unit = "grams",
    must_use = "ColdWater is a conserved resource: pass it on or hand it to a Consumer"
}

model_core::container_resource! {
    /// Electrical energy drawn from the grid, in joules (R7). Enters the
    /// model only through [`boundary::draw_grid_energy`] (R12, R15).
    Electricity,
    unit = "joules",
    must_use = "Electricity is a conserved resource: pass it on or hand it to a Consumer"
}

model_core::container_resource! {
    /// Waste heat, in joules (R15: waste is an ordinary conserved output).
    /// All of it must be accounted to the kitchen-air sink.
    WasteHeat,
    unit = "joules",
    must_use = "WasteHeat is a conserved waste product: account it to the kitchen-air sink"
}

model_core::reusable_resource! {
    /// The electric kettle, empty — the state it enters the model in and
    /// returns to after the pour. Reusable (R2): no tripwire.
    ///
    /// Placeholder: kitchen setup at flow start — one kettle.
    Kettle,
    must_use = "Kettle is a reusable resource: pass it on or return it to the caller"
}

model_core::container_resource! {
    /// The kettle filled with `V` grams of cold water. A distinct processing
    /// state (R9): it cannot be poured — only the boiling state satisfies
    /// REQ-001. Tripwired (F-008).
    FilledKettle,
    unit = "grams of water",
    must_use = "FilledKettle is a conserved resource: boil it or hand it to a Consumer"
}

model_core::container_resource! {
    /// The kettle at the boil: `WATER_G` grams of boiling water carrying `V`
    /// joules of embodied energy. The only kettle state that can be poured
    /// (REQ-001).
    BoilingKettle<const WATER_G: u64>,
    unit = "joules (embodied)",
    must_use = "BoilingKettle is a conserved resource: pour it or hand it to a Consumer"
}

model_core::container_resource! {
    /// Poured hot water: `WATER_G` grams carrying `V` joules of embodied
    /// energy — the slice's product; it leaves with the [`Drinker`].
    HotWater<const WATER_G: u64>,
    unit = "joules (embodied)",
    must_use = "HotWater is the product: hand it to the drinker (a Consumer)"
}

/// The boiling characteristic (R6) lives on the boiling state only (R9):
/// `pour_away` is the permit-gated conserving extraction only
/// [`processes::pour_cuppa`] can call.
impl<const G: u64, const E: u64> Boiling for BoilingKettle<G, E> {
    const WATER_G: u64 = G;
    const EMBODIED_J: u64 = E;
    fn pour_away(self, _permit: BrewPermit) -> Kettle {
        // Conserving transform: the water and embodied energy continue as
        // the outputs minted by pour_cuppa (R1); the vessel returns empty.
        self.defuse();
        Kettle::mint()
    }
}
impl<const G: u64, const E: u64> sealed::Sealed for BoilingKettle<G, E> {}

/// The boiling kettle at the slice's quantities (1500 g of water, 500 000 J
/// embodied). The alias carries the satisfaction tag (F-037).
///
/// Satisfies: REQ-001
pub type KettleAtTheBoil = BoilingKettle<1500, 500_000>;
model_core::satisfies!(assert_req001, KettleAtTheBoil);

/// The permit gating the pour's conserving extraction (the F-054 pattern):
/// private field, no public constructor, so only this module's
/// [`processes::pour_cuppa`] can call [`Boiling::pour_away`].
pub struct BrewPermit {
    _seal: (),
}

// ---------------------------------------------------------------------------
// Boundary objects at the kitchen's edge (R12).
// ---------------------------------------------------------------------------

model_core::reusable_resource! {
    /// The mains tap: the unbounded boundary source cold water is drawn from
    /// (R15: a draw-style boundary process, never a `Supplier` impl, F-028).
    ///
    /// Placeholder: mains water — assumed unbounded.
    MainsTap,
    must_use = "MainsTap is a boundary resource: pass it on like any other resource"
}

model_core::reusable_resource! {
    /// The grid socket: the unbounded boundary source electrical energy is
    /// drawn from (R15, F-028).
    ///
    /// Placeholder: grid electricity — assumed unbounded.
    GridSocket,
    must_use = "GridSocket is a boundary resource: pass it on like any other resource"
}

/// The kitchen air: the unbounded boundary sink waste heat is accounted to.
/// `Next = Self` (R15, F-029): legal only at the system boundary.
///
/// Placeholder: kitchen air — assumed able to absorb all waste heat.
#[must_use = "KitchenAir is a boundary resource: pass it on like any other resource"]
pub struct KitchenAir {
    _seal: (),
}

/// The air absorbs waste heat of any magnitude; `Next = Self` (R15).
impl<const E: u64> Consumer<WasteHeat<E>> for KitchenAir {
    type Next = KitchenAir;
    fn consume(self, item: WasteHeat<E>) -> KitchenAir {
        item.defuse(); // sanctioned consumer role (F-008); unbounded sink discards (F-029)
        self
    }
}

/// The drinker taking delivery of the hot water: an unbounded boundary sink
/// (`type Next = Self`, R15, F-029).
///
/// Placeholder: the drinker — cup return is out of scope.
#[must_use = "Drinker is a boundary resource: pass it on like any other resource"]
pub struct Drinker {
    _seal: (),
}

/// The drinker accepts the hot water, with its mass and embodied energy;
/// `Next = Self` (R15, F-029).
impl<const G: u64, const E: u64> Consumer<HotWater<G, E>> for Drinker {
    type Next = Drinker;
    fn consume(self, item: HotWater<G, E>) -> Drinker {
        item.defuse();
        self
    }
}

/// The creation boundary of the kitchen slice (R12, F-006): the only
/// production code where the setup and the mains/grid draws come into
/// existence.
pub mod boundary {
    use super::{ColdWater, Drinker, Electricity, GridSocket, Kettle, KitchenAir, MainsTap};

    /// The kettle enters the model, empty (R12).
    ///
    /// Placeholder: kitchen setup at flow start — one kettle.
    pub fn new_kettle() -> Kettle {
        Kettle::mint()
    }

    /// The mains tap enters the model (R12).
    ///
    /// Placeholder: mains water — assumed unbounded.
    pub fn new_mains_tap() -> MainsTap {
        MainsTap::mint()
    }

    /// The grid socket enters the model (R12).
    ///
    /// Placeholder: grid electricity — assumed unbounded.
    pub fn new_grid_socket() -> GridSocket {
        GridSocket::mint()
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

    /// Draws `TAKE` grams of cold water from the mains (R15: an unbounded
    /// source of continuous material is a draw-style boundary process, never
    /// a `Supplier` impl, F-028). The tap is returned (R2).
    ///
    /// Placeholder: mains water — assumed unbounded.
    pub fn draw_cold_water<const TAKE: u64>(tap: MainsTap) -> (ColdWater<TAKE>, MainsTap) {
        (ColdWater::mint(), tap)
    }

    /// Draws `TAKE` joules from the grid (R15, F-028). The socket is
    /// returned (R2).
    ///
    /// Placeholder: grid electricity — assumed unbounded.
    pub fn draw_grid_energy<const TAKE: u64>(socket: GridSocket) -> (Electricity<TAKE>, GridSocket) {
        (Electricity::mint(), socket)
    }
}

/// The slice's processes (R1, R2): pure by-value transformations. They mint
/// quantity-bearing values, so they live inside the resource family's module
/// (F-031). The person's time is drawn by **adjacent** `draw_time` processes
/// in the flow (F-048), recorded into the `History` (R16).
pub mod processes {
    use super::{
        BoilingKettle, BrewPermit, ColdWater, Electricity, FilledKettle, HotWater, Kettle,
        Req001BoilingWater, WasteHeat,
    };
    use model_core::common::Person;

    /// P1 — fills the kettle with the drawn cold water. The person is moved
    /// in and comes back with the filled kettle (R2); the water's mass
    /// continues structurally in the shared const parameter `G`.
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

    /// P3 — boils the kettle: the filled kettle plus `DRAW_J` joules from
    /// the grid become the boiling kettle (same water mass `G`, `EMBODIED_J`
    /// embodied) plus `HEAT_J` of kettle-loss waste heat. **No person** (the
    /// kettle is automatic).
    ///
    /// Energy conservation (`EMBODIED_J + HEAT_J == DRAW_J`) is checked at
    /// compile time; the check fires at monomorphization (F-001): `cargo
    /// check` and editor diagnostics will not show a violation.
    ///
    /// Regression (R4 policy, F-003): boiling must not lose energy —
    /// 500 000 embodied + 60 000 heat ≠ 550 000 drawn:
    ///
    /// ```compile_fail
    /// use exp13_control::model::boundary::{draw_cold_water, draw_grid_energy, new_grid_socket, new_kettle, new_mains_tap};
    /// use exp13_control::model::processes::{boil, fill_kettle};
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

    /// P4 — pours the kettle at the boil into the cup: REQ-001 on the kettle
    /// (only the boiling state pours; the wrong state fails with the
    /// REQ-phrased `on_unimplemented` message, F-044). The quantities travel
    /// on the [`super::Boiling`] characteristic's associated consts (R6/R7);
    /// the extraction is permit-gated so only this process can realise it.
    ///
    /// Conservation is one assert per dimension (R3, R15, F-033), the caller
    /// stating the splits (F-022): mass `K::WATER_G == OUT_G`; energy
    /// `K::EMBODIED_J == OUT_E + LOSS_J`. Both fire at monomorphization
    /// (F-001).
    ///
    /// Satisfies: REQ-001
    pub fn pour_cuppa<const OUT_G: u64, const OUT_E: u64, const LOSS_J: u64, K: Req001BoilingWater>(kettle: K) -> (Kettle, HotWater<OUT_G, OUT_E>, WasteHeat<LOSS_J>) {
        const {
            assert!(
                K::WATER_G == OUT_G,
                "mass conservation violated in pour_cuppa (R3): the hot water poured must carry exactly the boiling water's mass"
            )
        };
        const {
            assert!(
                K::EMBODIED_J == OUT_E + LOSS_J,
                "energy conservation violated in pour_cuppa (R15): the kettle's embodied energy must sum exactly to the hot water's embodied energy plus the pouring loss"
            )
        };
        // The permit-gated conserving extraction (R1): the kettle returns to
        // its empty state; its water and energy continue as the outputs.
        let empty_kettle = kettle.pour_away(BrewPermit { _seal: () });
        (empty_kettle, HotWater::mint(), WasteHeat::mint())
    }
}

#[cfg(test)]
mod tests {
    //! Per-process unit tests (R5), inside the privacy boundary so fixtures
    //! may be minted and outputs defused directly.

    use super::boundary::{draw_cold_water, draw_grid_energy, new_grid_socket, new_mains_tap};
    use super::processes::{boil, fill_kettle, pour_cuppa};
    use super::{BoilingKettle, ColdWater, FilledKettle, HotWater, Kettle, WasteHeat};
    use model_core::common::boundary::new_person;

    /// P1: the drawn water's mass continues structurally into the filled
    /// kettle, and the person and tap come back (R2).
    #[test]
    fn fill_kettle_conserves_the_drawn_water() {
        let person = new_person::<300_000>();
        let (water, tap) = draw_cold_water::<1500>(new_mains_tap());
        assert_eq!(ColdWater::<1500>::VALUE, 1500);
        let (person, filled) = fill_kettle(person, Kettle::mint(), water);
        assert_eq!(FilledKettle::<1500>::VALUE, 1500);
        filled.defuse();
        let _reusables = (person, tap);
    }

    /// P3: energy balances (550 000 = 500 000 + 50 000), the water's mass
    /// flows through unchanged, and no person is involved.
    #[test]
    fn boil_conserves_energy_and_needs_no_person() {
        let (energy, socket) = draw_grid_energy::<550_000>(new_grid_socket());
        let filled: FilledKettle<1500> = FilledKettle::mint();
        let (boiling, heat) = boil::<1500, 550_000, 500_000, 50_000>(filled, energy);
        assert_eq!(
            BoilingKettle::<1500, 500_000>::VALUE + WasteHeat::<50_000>::VALUE,
            550_000
        );
        boiling.defuse();
        heat.defuse();
        let _socket = socket;
    }

    /// P4: mass and energy balance (1500 = 1500; 500 000 = 490 000 +
    /// 10 000) and the kettle comes back empty.
    ///
    /// Verifies: REQ-001
    #[test]
    fn pour_cuppa_balances_mass_and_energy() {
        let boiling: BoilingKettle<1500, 500_000> = BoilingKettle::mint();
        let (kettle, cuppa, loss) = pour_cuppa::<1500, 490_000, 10_000, _>(boiling);
        assert_eq!(HotWater::<1500, 490_000>::VALUE + WasteHeat::<10_000>::VALUE, 500_000);
        let _kettle_back_empty: Kettle = kettle;
        cuppa.defuse();
        loss.defuse();
    }
}
