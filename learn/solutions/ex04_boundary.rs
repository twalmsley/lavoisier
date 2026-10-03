//! Exercise 04 — the boundary: a supplier holds real objects
//! (belongs to Tutorial 04, "The boundary, flows and History".)
//!
//! GOAL: discrete items enter the model only through a supplier at the
//! system boundary (R12). A supplier holds the *actual objects* it will
//! supply in a type-level list — the count IS the list's length, so count
//! and contents cannot disagree. The guest wants three biscuits, but the tin
//! below was only filled with two. Fix the line marked `// TODO` until
//! `cargo test --test ex04_boundary` passes.
//!
//! What the broken state teaches: asking a supplier for more than it holds is
//! an `error[E0277]` with a modeller-phrased message ("it would run out
//! partway") — capacity is part of the type, so exhaustion is a compile
//! error, not a runtime surprise. Note the fix is at the BOUNDARY (fill the
//! tin properly), not in the process that asks.

#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![deny(let_underscore_drop)]
#![recursion_limit = "2048"]

use model_core::boundary::{Consumer, Supplier, send_list, take_n};
use model_core::list::{Cons, Len, Nil};
use model_core::nat::aliases::N3;

model_core::consumable_resource! {
    /// A biscuit: a discrete item, its own object (R13). Tripwired: a
    /// biscuit that never reaches a consumer fails the test that leaked it.
    Biscuit,
    must_use = "Biscuit is a conserved resource: pass it on or hand it to a Consumer"
}

/// A biscuit tin: a supplier at the system boundary (R12). Its contents are
/// a type-level list of real [`Biscuit`] values.
///
/// Placeholder: biscuit tin — brand/vendor not modelled.
#[must_use = "BiscuitTin is a boundary resource: pass it on like any other resource"]
pub struct BiscuitTin<Items>(Items);

/// `Supplier` is implemented ONLY for a non-empty tin (R12): supplying from
/// an empty tin is a compile error with model-core's modeller-phrased
/// message (F-015).
impl<H, T> Supplier for BiscuitTin<Cons<H, T>> {
    type Item = H;
    type Next = BiscuitTin<T>;
    fn supply(self) -> (H, BiscuitTin<T>) {
        let Cons(head, tail) = self.0;
        (head, BiscuitTin(tail))
    }
}

impl<Items: Len> BiscuitTin<Items> {
    /// How many biscuits the tin holds — the length of its contents list.
    pub const COUNT: u64 = Items::LEN;
}

/// A type-level list of exactly three biscuits — the `Taken =` shape the
/// guest's `SupplyN<N3>` request produces (F-014).
pub type ThreeBiscuits = Cons<Biscuit, Cons<Biscuit, Cons<Biscuit, Nil>>>;

/// The guest eating the biscuits: an unbounded boundary sink
/// (`type Next = Self`, R15, F-029) — legal only at the boundary, always a
/// placeholder, and it necessarily discards what it consumes.
///
/// Placeholder: the guest — appetite assumed unbounded.
#[must_use = "Guest is a boundary resource: pass them on like any other resource"]
pub struct Guest {
    _seal: (),
}

/// The guest accepts biscuits; `Next = Self` (R15, F-029).
impl Consumer<Biscuit> for Guest {
    type Next = Guest;
    fn consume(self, item: Biscuit) -> Guest {
        item.defuse(); // the sanctioned consumer role (F-008)
        self
    }
}

/// The guest arrives at the boundary (R12).
pub fn arriving_guest() -> Guest {
    Guest { _seal: () }
}

/// Three biscuits leave the tin one at a time (R12: one item per step, via
/// one `SupplyN<N3>` bound) and reach the guest; the emptied tin is a
/// distinct resource that is itself accounted for.
#[test]
fn three_biscuits_for_the_guest() {
    // The boundary fill: the only place biscuits come into existence (R12) —
    // this file is its own crate, so `mint` is reachable here and only here.
    // SOLVED: the guest wants three biscuits, but the tin was only filled with
    // two. Fill it properly.
    let tin = BiscuitTin(Cons(
        Biscuit::mint(),
        Cons(Biscuit::mint(), Cons(Biscuit::mint(), Nil)),
    ));

    // One where-clause takes all three (F-014); the annotation pins that the
    // guest really gets three, not fewer.
    let (taken, tin): (ThreeBiscuits, _) = take_n::<N3, _>(tin);

    // The exhausted tin is a new resource (R12) that must be accounted for.
    let _empty_tin: BiscuitTin<Nil> = tin;

    // Every biscuit genuinely reaches the consumer (R1).
    let _guest = send_list(arriving_guest(), taken);
}
