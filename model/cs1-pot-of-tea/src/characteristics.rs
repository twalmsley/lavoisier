//! Characteristic traits (R6) for the CS-1 kitchen.
//!
//! A type's characteristics are the traits it implements; the R10 requirement
//! traits in [`crate::requirements`] are written as bounds over these, never
//! over concrete resource types (F-019). Each carries a single-line
//! `#[diagnostic::on_unimplemented]` phrased for modellers (F-015) —
//! single-line on purpose: the workspace `trace.sh` has no scope awareness,
//! and a multi-line attribute between a doc tag and its item would detach
//! them (F-021).
//!
//! ## Measured characteristics with a conserving extraction
//!
//! [`Boiling`] and [`LoadedToBrew`] go beyond plain markers: per R6, a
//! measured characteristic can also carry its quantities, so they expose the
//! boiling water's mass and embodied energy, and the loaded pot's dry and
//! spent bag masses, as associated constants — which is what lets
//! `pour_and_brew` stay **generic over its requirement bounds** (style A,
//! F-048: REQ-phrased errors) while still stating compile-time conservation
//! over the inputs' magnitudes. Each also carries the one conserving
//! extraction its consuming process needs, gated by the sealed
//! [`crate::resources::BrewPermit`] (the R16 `Permit` pattern): downstream
//! code can neither call the extraction (no permit can be constructed) nor
//! implement the trait for its own types (the private supertrait below), so
//! the characteristics cannot be used to mint or vanish resources.

use crate::resources::{BrewPermit, Kettle, SpentTeabag};

/// Seals [`Boiling`] and [`LoadedToBrew`] (the classic sealed-trait pattern,
/// F-026): only this crate's resource states may carry them. Without the
/// seal, outside code could implement the trait for a type of its own and
/// smuggle unconserved "boiling water" past REQ-006's bound.
pub(crate) mod sealed {
    /// Implemented only for this crate's brew-ready resource states.
    pub trait Sealed {}
}

/// Characteristic (R6): the kettle is **at the boil** — implemented only by
/// the boiling kettle state (one type per state, R9: *boiling* is a type,
/// not a temperature reading, SPEC.md §1). Carries the boiling water's mass
/// and embodied energy (R6/R7) and the permit-gated conserving pour.
#[diagnostic::on_unimplemented(message = "`{Self}` is not a kettle at the boil (REQ-006: only the boiling state can be poured)", label = "a boiling kettle is required here", note = "boiling is a process (one type per state, R9): `boil` turns a `FilledKettle` into a `BoilingKettle`")]
pub trait Boiling: sealed::Sealed {
    /// The boiling water's mass, in grams (R7).
    const WATER_G: u64;
    /// The water's embodied energy, in joules (R7; carried in the type from
    /// boiling onward, SPEC.md §3).
    const EMBODIED_J: u64;
    /// Conserving extraction: the kettle is poured out and returns to its
    /// empty state. The water and its embodied energy continue into the brew
    /// — `pour_and_brew`'s conservation asserts check exactly that — which is
    /// why this is callable only with a [`BrewPermit`] (no public
    /// constructor): a free-standing pour would vanish them silently.
    fn pour_away(self, permit: BrewPermit) -> Kettle;
}

/// Characteristic (R6): the pot is **loaded with exactly 3 teabags** and
/// ready to brew — implemented only by the loaded-pot state, which exists
/// only at exactly 3 bags (SPEC.md §8 review decision 2). Carries the bags'
/// dry and spent masses (R6/R7) and the permit-gated conserving steep.
#[diagnostic::on_unimplemented(message = "`{Self}` is not a pot loaded with exactly 3 teabags (REQ-007)", label = "a pot loaded with exactly 3 teabags is required here", note = "loading is a process (one type per state, R9): `load_pot` takes 3 bags from the teabag box and turns a `Teapot` into a `LoadedPot` - no other bag count has a loaded state")]
pub trait LoadedToBrew: sealed::Sealed {
    /// The loaded dry teabags' total mass, in grams (R7): 3 bags at 3 g.
    const DRY_G: u64;
    /// The spent teabags' total mass after brewing, in grams (R7): 3 bags at
    /// 12 g (each absorbs 9 g of water, SPEC.md §3).
    const SPENT_G: u64;
    /// Conserving extraction: the pot steeps and its 3 dry bags continue as
    /// 3 spent bags; the vessel continues as the pot of tea that
    /// `pour_and_brew` mints. Callable only with a [`BrewPermit`] — a
    /// free-standing steep would vanish the pot's mass unchecked.
    fn steep(self, permit: BrewPermit) -> (SpentTeabag, SpentTeabag, SpentTeabag);
}

/// Characteristic (R6): a boundary consumer dedicated to food waste
/// (REQ-008's subject). Implemented by [`crate::resources::FoodWasteBin`] in
/// every fill state — REQ-008 is about *which* consumer the spent bags go
/// to; whether there is space left is the `Consumer` impl's business (R12).
#[diagnostic::on_unimplemented(message = "`{Self}` is not the food-waste bin (REQ-008: all spent teabags must reach it)", label = "the food-waste bin is required here", note = "the dedicated consumer is the `FoodWasteBin` (any fill state); it is emptied only through `empty_bin`, the sealed disposal path to the council collection (F-039)")]
pub trait FoodWasteConsumer {}

/// Characteristic (R6): the kitchen-air sink that REQ-009 accounts all waste
/// heat to. Implemented only by [`crate::resources::KitchenAir`], the
/// placeholder unbounded boundary sink (R15).
#[diagnostic::on_unimplemented(message = "`{Self}` is not the kitchen-air sink (REQ-009: all waste heat must be accounted to it)", label = "the kitchen-air sink is required here", note = "the kitchen air (`KitchenAir`, a placeholder unbounded sink, R15) is where waste heat leaves the model")]
pub trait KitchenAirSink {}
