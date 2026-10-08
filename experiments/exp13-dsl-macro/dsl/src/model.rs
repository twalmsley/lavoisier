//! The kitchen slice, authored through the `model!` front-end (the EXP-13
//! experiment arm). Same model surface as `exp13-control::model`.
//!
//! The characteristic trait, its sealed supertrait, the `BrewPermit` and the
//! `Boiling` impl ride in `rust { … }` pass-through islands: the declarative
//! grammar cannot express them (see the R-coverage table in RESULTS.md).

crate::model! {

    rust {
        /// Seals `Boiling` (the classic sealed-trait pattern, F-026): only
        /// this crate's boiling state may carry it.
        pub(crate) mod sealed {
            /// Implemented only for this crate's boiling kettle state.
            pub trait Sealed {}
        }

        /// Characteristic (R6): the kettle is **at the boil** — implemented
        /// only by the boiling kettle state (one type per state, R9). Carries
        /// the boiling water's mass and embodied energy (R6/R7) and the
        /// permit-gated conserving pour (F-054).
        #[diagnostic::on_unimplemented(message = "`{Self}` is not a kettle at the boil (REQ-002: only the boiling state can be poured)", label = "a boiling kettle is required here", note = "boiling is a process (one type per state, R9): `boil` turns a `FilledKettle` into a `BoilingKettle`")]
        pub trait Boiling: sealed::Sealed {
            /// The boiling water's mass, in grams (R7).
            const WATER_G: u64;
            /// The water's embodied energy, in joules (R7).
            const EMBODIED_J: u64;
            /// Conserving extraction: the kettle is poured out and returns to
            /// its empty state; callable only with a [`BrewPermit`] (F-054).
            fn pour_away(self, permit: BrewPermit) -> Kettle;
        }
    }

    requirement {
        /// REQ-002: Tea must be brewed with boiling water (only the boiling state of the kettle can be poured).
        #[diagnostic::on_unimplemented(message = "this kettle may not be poured: `{Self}` is not at the boil (REQ-002)", label = "REQ-002: tea must be brewed with boiling water", note = "boiling is a process state (R9): `boil` turns a `FilledKettle` into a `BoilingKettle` - only that state can be poured")]
        pub trait Req002BoilingWater: (Boiling);
        assert = assert_req002;
    }

    // The sugared requirement form, defined here to MEASURE its trace.sh
    // visibility (RESULTS.md): the REQ-003 doc line below is generated as a
    // `#[doc = …]` attribute, so it never exists as a source line.
    requirement Req003ServedHot is "REQ-003: Hot water must be served straight from the boil." needs (Boiling),
    assert = assert_req003, on_fail = "this water may not be served: `{Self}` is not at the boil (REQ-003)";

    /// Cold water drawn from the mains, in grams (R7). Enters the model only
    /// through [`boundary::draw_cold_water`] (R12, R15).
    container ColdWater, unit = "grams", must_use = "ColdWater is a conserved resource: pass it on or hand it to a Consumer";

    /// Electrical energy drawn from the grid, in joules (R7). Enters the
    /// model only through [`boundary::draw_grid_energy`] (R12, R15).
    container Electricity, unit = "joules", must_use = "Electricity is a conserved resource: pass it on or hand it to a Consumer";

    /// Waste heat, in joules (R15: waste is an ordinary conserved output).
    /// All of it must be accounted to the kitchen-air sink.
    container WasteHeat, unit = "joules", must_use = "WasteHeat is a conserved waste product: account it to the kitchen-air sink";

    /// The electric kettle, empty — the state it enters the model in and
    /// returns to after the pour. Reusable (R2): no tripwire.
    ///
    /// Placeholder: kitchen setup at flow start — one kettle.
    reusable Kettle, must_use = "Kettle is a reusable resource: pass it on or return it to the caller";

    /// The kettle filled with `V` grams of cold water. A distinct processing
    /// state (R9): it cannot be poured — only the boiling state satisfies
    /// REQ-002. Tripwired (F-008).
    container FilledKettle, unit = "grams of water", must_use = "FilledKettle is a conserved resource: boil it or hand it to a Consumer";

    /// The kettle at the boil: `WATER_G` grams of boiling water carrying `V`
    /// joules of embodied energy. The only kettle state that can be poured
    /// (REQ-002).
    container BoilingKettle [const WATER_G: u64], unit = "joules (embodied)", must_use = "BoilingKettle is a conserved resource: pour it or hand it to a Consumer";

    /// Poured hot water: `WATER_G` grams carrying `V` joules of embodied
    /// energy — the slice's product; it leaves with the `Drinker`.
    container HotWater [const WATER_G: u64], unit = "joules (embodied)", must_use = "HotWater is the product: hand it to the drinker (a Consumer)";

    rust {
        /// The boiling characteristic (R6) lives on the boiling state only
        /// (R9): `pour_away` is the permit-gated conserving extraction only
        /// `processes::pour_cuppa` can call.
        impl<const G: u64, const E: u64> Boiling for BoilingKettle<G, E> {
            const WATER_G: u64 = G;
            const EMBODIED_J: u64 = E;
            fn pour_away(self, _permit: BrewPermit) -> Kettle {
                // Conserving transform: the water and embodied energy
                // continue as the outputs minted by pour_cuppa (R1).
                self.defuse();
                Kettle::mint()
            }
        }
        impl<const G: u64, const E: u64> sealed::Sealed for BoilingKettle<G, E> {}

        /// The boiling kettle at the slice's quantities (1500 g of water,
        /// 500 000 J embodied). The alias carries the satisfaction tag
        /// (F-037).
        ///
        /// Satisfies: REQ-002
        pub type KettleAtTheBoil = BoilingKettle<1500, 500_000>;
        model_core::satisfies!(assert_req002, KettleAtTheBoil);

        /// The boil also serves straight away — the sugared requirement's
        /// satisfaction, to measure its trace.sh visibility (RESULTS.md).
        ///
        /// Satisfies: REQ-003
        pub type ServableWater = BoilingKettle<1500, 500_000>;
        model_core::satisfies!(assert_req003, ServableWater);

        /// The permit gating the pour's conserving extraction (the F-054
        /// pattern): private field, no public constructor, so only this
        /// module tree's `processes::pour_cuppa` can call
        /// [`Boiling::pour_away`].
        pub struct BrewPermit {
            pub(crate) _seal: (),
        }
    }
}

/// The creation boundary of the kitchen slice (R12, F-006), authored through
/// the `model!` boundary constructs.
pub mod boundary {
    use super::{ColdWater, Electricity, Kettle, WasteHeat};

    crate::model! {

        /// The mains tap: the unbounded boundary source cold water is drawn
        /// from (R15, F-028).
        source MainsTap { enter = new_mains_tap, draw = draw_cold_water -> ColdWater },
        must_use = "MainsTap is a boundary resource: pass it on like any other resource",
        placeholder = "mains water - assumed unbounded.";

        /// The grid socket: the unbounded boundary source electrical energy
        /// is drawn from (R15, F-028).
        source GridSocket { enter = new_grid_socket, draw = draw_grid_energy -> Electricity },
        must_use = "GridSocket is a boundary resource: pass it on like any other resource",
        placeholder = "grid electricity - assumed unbounded.";

        /// The kitchen air: the unbounded boundary sink waste heat is
        /// accounted to (R15, F-029).
        sink KitchenAir { enter = new_kitchen_air, accepts [const E: u64] = WasteHeat<E> },
        must_use = "KitchenAir is a boundary resource: pass it on like any other resource",
        placeholder = "kitchen air - assumed able to absorb all waste heat.";

        /// The drinker taking delivery of the hot water: an unbounded
        /// boundary sink (R15, F-029).
        sink Drinker { enter = new_drinker, accepts [const G: u64, const E: u64] = super::HotWater<G, E> },
        must_use = "Drinker is a boundary resource: pass it on like any other resource",
        placeholder = "the drinker - cup return is out of scope.";

        entry new_kettle -> Kettle, placeholder = "kitchen setup at flow start - one kettle.";
    }
}

/// The slice's processes (R1, R2), authored through the `model!` process
/// constructs. The person's time is drawn by **adjacent** `draw_time`
/// processes in the flow (F-048), recorded into the `History` (R16).
pub mod processes {
    // (The Boiling methods need no import: on a generic parameter they
    // resolve through the requirement bound's supertrait.)
    use super::{
        BoilingKettle, BrewPermit, ColdWater, Electricity, FilledKettle, HotWater, Kettle,
        Req002BoilingWater, WasteHeat,
    };
    use model_core::common::Person;

    crate::model! {

        /// P1 — fills the kettle with the drawn cold water. The person is
        /// moved in and comes back with the filled kettle (R2); the water's
        /// mass continues structurally in the shared const parameter `G`.
        process fn fill_kettle [const B: u64, const G: u64] {
            threads { person: Person<B> }
            absorbs { kettle: Kettle }
            consumes { water: ColdWater<G> }
            mints { FilledKettle<G> }
        }

        /// P3 — boils the kettle: the filled kettle plus `DRAW_J` joules
        /// from the grid become the boiling kettle (same water mass `G`,
        /// `EMBODIED_J` embodied) plus `HEAT_J` of kettle-loss waste heat.
        /// **No person** (the kettle is automatic).
        ///
        /// The energy assert fires at monomorphization (F-001): `cargo
        /// check` and editor diagnostics will not show a violation.
        ///
        /// Regression (R4 policy, F-003), authored through the DSL flow
        /// grammar — 500 000 embodied + 60 000 heat ≠ 550 000 drawn:
        ///
        /// ```compile_fail
        /// exp13_dsl::model! {
        ///     flow pub fn broken_boil {
        ///         rust {
        ///             use exp13_dsl::model::boundary::{draw_cold_water, draw_grid_energy, new_grid_socket, new_kettle, new_mains_tap};
        ///             use exp13_dsl::model::processes::{boil, fill_kettle};
        ///             use model_core::common::boundary::new_person;
        ///             let person = new_person::<300_000>();
        ///         }
        ///         step (water, tap) = draw_cold_water::<1500>(new_mains_tap());
        ///         step (person, filled) = fill_kettle(person, new_kettle(), water);
        ///         step (energy, socket) = draw_grid_energy::<550_000>(new_grid_socket());
        ///         step (boiling, heat) = boil::<1500, 550_000, 500_000, 60_000>(filled, energy);
        ///         rust { let _accounted = (person, tap, socket, boiling, heat); }
        ///     }
        /// }
        /// broken_boil(); // force the instantiation (the assert fires at monomorphization, F-001)
        /// ```
        process fn boil [const G: u64, const DRAW_J: u64, const EMBODIED_J: u64, const HEAT_J: u64] {
            consumes { kettle: FilledKettle<G>, energy: Electricity<DRAW_J> }
            mints { BoilingKettle<G, EMBODIED_J>, WasteHeat<HEAT_J> }
            assert "energy conservation violated in boil (R15): the embodied energy plus the kettle's waste heat must sum exactly to the energy drawn from the grid" : EMBODIED_J + HEAT_J == DRAW_J;
        }

        /// P4 — pours the kettle at the boil into the cup: REQ-002 on the
        /// kettle (only the boiling state pours, F-044). The quantities
        /// travel on the `Boiling` characteristic's associated consts
        /// (R6/R7); the extraction is permit-gated, so the declarative form
        /// cannot express this process — raw `body` escape hatch (RESULTS.md
        /// R-coverage).
        ///
        /// Satisfies: REQ-002
        process fn pour_cuppa [const OUT_G: u64, const OUT_E: u64, const LOSS_J: u64, K: Req002BoilingWater] {
            takes (kettle: K) -> (Kettle, HotWater<OUT_G, OUT_E>, WasteHeat<LOSS_J>);
            assert "mass conservation violated in pour_cuppa (R3): the hot water poured must carry exactly the boiling water's mass" : K::WATER_G == OUT_G;
            assert "energy conservation violated in pour_cuppa (R15): the kettle's embodied energy must sum exactly to the hot water's embodied energy plus the pouring loss" : K::EMBODIED_J == OUT_E + LOSS_J;
            body {
                // The permit-gated conserving extraction (R1): the kettle
                // returns to its empty state; its water and energy continue
                // as the outputs.
                let empty_kettle = kettle.pour_away(BrewPermit { _seal: () });
                (empty_kettle, HotWater::mint(), WasteHeat::mint())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    //! Per-process unit tests (R5), inside the privacy boundary. The first
    //! two are plain Rust (as in the control); the flow-shaped one is
    //! authored through the DSL's `flow test fn` grammar.

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

    crate::model! {
        /// P4: mass and energy balance (1500 = 1500; 500 000 = 490 000 +
        /// 10 000) and the kettle comes back empty — authored through the
        /// DSL flow grammar.
        ///
        /// Verifies: REQ-002
        flow test fn pour_cuppa_balances_mass_and_energy {
            rust { let boiling: BoilingKettle<1500, 500_000> = BoilingKettle::mint(); }
            step (kettle, cuppa, loss) = pour_cuppa::<1500, 490_000, 10_000, _>(boiling);
            rust {
                assert_eq!(HotWater::<1500, 490_000>::VALUE + WasteHeat::<10_000>::VALUE, 500_000);
                let _kettle_back_empty: Kettle = kettle;
                cuppa.defuse();
                loss.defuse();
            }
        }
    }
}
