//! Exercise 02 — requirements and characteristics
//! (belongs to Tutorial 02, "Resources and requirements".)
//!
//! GOAL: a requirement (R10) is a named trait used as a *bound* on a process,
//! and a type satisfies it by carrying the right *characteristic* traits
//! (R6). Exactly one of the two water states below is at the boil — attach
//! the `Boiling` characteristic to it at the line marked `// TODO` until
//! `cargo test --test ex02_requirements` passes.
//!
//! What the broken state teaches: an unmet requirement is an `error[E0277]`
//! whose message is the requirement itself, naming its REQ id — the message
//! you wrote in `#[diagnostic::on_unimplemented]` is the one modellers see.
//! The `satisfies!` line fails with the same error: the `/// Satisfies:` doc
//! tag is for grep, and the assertion keeps the tag honest (F-020).
//!
//! (This exercise file is its own crate, so the macros expand privately here:
//! this file IS the model's privacy boundary. Requirement ids in `learn/` use
//! the 900s so they can never collide with the model/ workspace, where ids
//! are workspace-global, F-053.)

#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![deny(let_underscore_drop)]

model_core::consumable_resource! {
    /// A kettle of water straight from the tap — NOT at the boil.
    KettleOfColdWater,
    must_use = "KettleOfColdWater is a conserved resource: pass it on or hand it to a Consumer"
}

model_core::consumable_resource! {
    /// A kettle of water at a rolling boil.
    KettleOfBoilingWater,
    must_use = "KettleOfBoilingWater is a conserved resource: pass it on or hand it to a Consumer"
}

/// Characteristic (R6): the water is at the boil. One type per processing
/// state (R9): *boiling* is a type, not a temperature reading, so this trait
/// belongs on exactly one of the two states above.
#[diagnostic::on_unimplemented(message = "`{Self}` is not water at the boil", label = "boiling water is required here", note = "boiling is a process state (R9): only the boiling state of the water carries this characteristic")]
pub trait Boiling {}

// TODO: exactly one of the two water states is at the boil. Say so, with a
// one-line characteristic impl (which type satisfies REQ-901?):
// impl Boiling for <which state?> {}

model_core::requirement! {
    /// REQ-901: Tea must be brewed with boiling water.
    #[diagnostic::on_unimplemented(message = "this water may not be used to brew: `{Self}` is not at the boil (REQ-901)", label = "REQ-901: tea must be brewed with boiling water", note = "only a type carrying the `Boiling` characteristic satisfies this requirement (R6, R10)")]
    pub trait Req901BrewedAtTheBoil: (Boiling);
    assert = assert_req901;
}

/// The boiling state under its requirement-facing name. The doc tag is for
/// grep; the `satisfies!` assertion below keeps it honest — a stale tag is a
/// compile error naming the missing characteristic (F-020).
///
/// Satisfies: REQ-901
pub type BrewReadyWater = KettleOfBoilingWater;
model_core::satisfies!(assert_req901, BrewReadyWater);

/// Brewing demands the requirement, never a concrete type (F-019): any type
/// carrying the right characteristics is accepted, and the wrong one fails
/// with the REQ-phrased message above.
pub fn brew<W: Req901BrewedAtTheBoil>(water: W) -> W {
    // (A real model would transform the state here — CS-1's `pour_and_brew`
    // is the full version. This exercise is about the bound.)
    water
}

/// The boiling state passes the REQ-901 bound; the cold state never would —
/// try swapping it in once this compiles, and read the error you get.
#[test]
fn brewing_demands_the_boiling_state() {
    let water = KettleOfBoilingWater::mint();
    let water = brew(water);
    // This test sits inside the privacy boundary (the macros expanded in this
    // crate), so it may defuse the tripwire directly; a downstream flow would
    // have to hand the water to a Consumer instead.
    water.defuse();

    // The cold state exists and is a different type — that is the whole
    // "one type per state" point (R9).
    let cold = KettleOfColdWater::mint();
    cold.defuse();
}
