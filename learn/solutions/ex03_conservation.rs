//! Exercise 03 — conservation is checked by the compiler (and your editor
//! will NOT warn you)
//! (belongs to Tutorial 03, "Processes and conservation".)
//!
//! GOAL: drawing from a container is a split process (R3, R15): the caller
//! states every part of the balance — TAKE, what is LEFT, and the FULL amount
//! — and a `const { assert!(…) }` checks it at compile time. The balance
//! below is wrong. Fix the line marked `// TODO` until
//! `cargo test --test ex03_conservation` passes.
//!
//! What the broken state teaches (F-001, the most important finding in the
//! project): conservation errors fire at MONOMORPHIZATION, during code
//! generation. Try both of these on the broken file:
//!
//! ```text
//! cargo check --test ex03_conservation   # reports NO errors — it is lying
//! cargo test  --test ex03_conservation   # error[E0080]: the real verdict
//! ```
//!
//! `cargo check`, rust-analyzer and your editor's diagnostics never see a
//! conservation violation. Only `cargo build` / `cargo test` do — which is
//! why the model workspace's ci.sh gates on full builds, never on check.

#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![deny(let_underscore_drop)]

model_core::container_resource! {
    /// Milk drawn from the bottle, in grams (R7 base mass unit).
    Milk,
    unit = "grams",
    must_use = "Milk is a conserved resource: pass it on or hand it to a Consumer"
}

model_core::container_resource! {
    /// The milk bottle holding `V` grams; `MilkBottle<0>` is the empty state
    /// — a distinct resource that must still be accounted for (R15).
    MilkBottle,
    unit = "grams remaining",
    must_use = "MilkBottle is a conserved resource: even an empty bottle must be passed on or handed to a Consumer"
}

model_core::draw_process! {
    /// Draws `TAKE` grams from a bottle holding `FULL`, leaving `LEFT`.
    pub fn draw_milk: MilkBottle => Milk,
    assert = "conservation violated in draw_milk (R15): TAKE + LEFT must equal FULL - is more milk being taken than the bottle holds, or is the stated remainder wrong?"
}

/// 300 g of milk goes into the tea; the rest stays in the bottle. The
/// compiler checks the arithmetic — a wrong balance does not build.
#[test]
fn the_bottle_balance_must_add_up() {
    let bottle: MilkBottle<2_000> = MilkBottle::mint();

    // SOLVED: 300 g is drawn from the 2 000 g bottle, but the stated remainder
    // is wrong — the milk would not balance. Restate what is LEFT.
    // (Notice that `cargo check` accepts the wrong number: see the file docs.)
    let (milk, bottle) = draw_milk::<300, 1_700, 2_000>(bottle);

    // The tea needs exactly 300 g — this annotation pins the draw.
    let milk: Milk<300> = milk;
    assert_eq!(Milk::<300>::VALUE, 300);
    assert_eq!(Milk::<300>::UNIT, "grams");

    // This test sits inside the privacy boundary, so it may defuse directly;
    // a real flow hands both to Consumers (R12).
    milk.defuse();
    bottle.defuse();
}
