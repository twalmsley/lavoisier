//! Exercise 05 — flows: the types order the steps
//! (belongs to Tutorial 04, "The boundary, flows and History".)
//!
//! GOAL: the model describes connections, not sequences (R9) — any order
//! that type-checks is valid, and an *impossible* order does not compile.
//! That only works because each processing state is its own type: a raw egg
//! is not a boiled egg. The flow below skips a step. Fix the lines marked
//! `// TODO` until `cargo test --test ex05_flow_order` passes.
//!
//! What the broken state teaches: `error[E0308]: mismatched types` —
//! "expected `BoiledEgg`, found `RawEgg`" — reads directly as "this egg has
//! not been boiled yet" (F-023). A missing process step is a type mismatch,
//! not a wrong answer at runtime.

#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![deny(let_underscore_drop)]

use model_core::boundary::{Consumer, send_to};

model_core::consumable_resource! {
    /// A raw egg — one type per processing state (R9).
    RawEgg,
    must_use = "RawEgg is a conserved resource: pass it on or hand it to a Consumer"
}

model_core::consumable_resource! {
    /// A boiled egg: a different type from [`RawEgg`], which is exactly what
    /// makes "peeled before boiled" a compile error instead of a wrong
    /// breakfast.
    BoiledEgg,
    must_use = "BoiledEgg is a conserved resource: pass it on or hand it to a Consumer"
}

model_core::consumable_resource! {
    /// A peeled boiled egg: the product.
    PeeledEgg,
    must_use = "PeeledEgg is the product: hand it to the eater (a Consumer)"
}

/// Boiling is a process (R1): the raw egg is moved in and continues as the
/// boiled egg — a conserving transform, nothing appears or disappears.
pub fn boil_egg(egg: RawEgg) -> BoiledEgg {
    // Conserving transform (R1): the egg's mass continues in the new state.
    egg.defuse();
    BoiledEgg::mint()
}

/// Peeling takes ONLY the boiled state — the signature is the rule.
pub fn peel_egg(egg: BoiledEgg) -> PeeledEgg {
    // Conserving transform (R1). (Shell waste is omitted to keep the
    // exercise small; a real model returns it and routes it to a bin — see
    // CS-1's spent teabags.)
    egg.defuse();
    PeeledEgg::mint()
}

/// The eater: an unbounded boundary sink (`type Next = Self`, R15, F-029).
///
/// Placeholder: the eater.
#[must_use = "Eater is a boundary resource: pass them on like any other resource"]
pub struct Eater {
    _seal: (),
}

/// The eater accepts only the finished product; `Next = Self` (R15, F-029).
impl Consumer<PeeledEgg> for Eater {
    type Next = Eater;
    fn consume(self, item: PeeledEgg) -> Eater {
        item.defuse();
        self
    }
}

/// The eater arrives at the boundary (R12).
pub fn hungry_eater() -> Eater {
    Eater { _seal: () }
}

/// The egg must be boiled before it can be peeled — the types say so.
#[test]
fn an_egg_must_be_boiled_before_it_is_peeled() {
    // The egg enters at the boundary (this file is its own crate, so `mint`
    // is reachable here and only here).
    let egg = RawEgg::mint();

    // SOLVED: a process step is missing — `peel_egg` wants a `BoiledEgg`, and
    // this egg is still raw. Add the missing step.
    let egg = boil_egg(egg);
    let peeled = peel_egg(egg);

    // The product genuinely leaves the model (R1).
    let _eater = send_to(hungry_eater(), peeled);
}
