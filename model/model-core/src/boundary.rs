//! Suppliers and consumers at the system boundary (R12).
//!
//! Raw materials enter the model only through explicit **suppliers**; waste
//! and products leave it only through explicit **consumers**. Both are
//! project traits (not closures) whose associated `Next` type is the
//! supplier/consumer after one use — capacity is part of the type, so using
//! an exhausted supplier or a full consumer is a compile error.
//!
//! Rules (R12):
//!
//! * Suppliers and consumers are **strict**: a supplier takes nothing but
//!   itself and returns one item plus its next state; a consumer takes only
//!   itself and the item. Anything needing payment, orders or receipts is a
//!   separate process (R1).
//! * **One item per step**; repeated use goes through the recursive helper
//!   traits [`SupplyN`] and [`ConsumeList`] (F-014) — never hand-chained
//!   `S::Next: Supplier` bounds beyond depth 2.
//! * A supplier holds **real objects** in a type-level list
//!   ([`crate::list::Cons`]); the count is the list's length, so count and
//!   contents cannot disagree.
//! * A supplier or consumer is itself a resource: passed by value and
//!   returned like any other (R2). Placeholders are marked with a
//!   `/// Placeholder: …` doc line and refined later.
//! * Model code reaches suppliers and consumers **through generic
//!   processes** such as [`take_one`]/[`send_to`], never by calling
//!   `supply`/`consume` directly on a concrete value: the direct-call
//!   mistake takes the E0599 path, which bypasses the
//!   `#[diagnostic::on_unimplemented]` messages below (F-015).
//! * Unbounded boundary objects (`type Next = Self`) are legal **only at
//!   the system boundary**, are always placeholders, and an unbounded
//!   consumer necessarily discards its intake — the one sanctioned
//!   exception to "a consumer keeps what it consumes" (R15, F-029).
//!
//! Adapted from `experiments/exp02-supplier-consumer/src/lib.rs` and
//! `experiments/exp09-continuous-resources/src/lib.rs` (`boundary_traits`).

use crate::list::{Cons, Nil};
use crate::nat::{Succ, Zero};

/// A supplier at the system boundary (R12): supplies exactly one item per
/// step, by value, and becomes its next state (a different type, one item
/// shorter).
///
/// **Discrete-only** (F-028): a finite continuous container cannot implement
/// any supplier-shaped trait on stable Rust — the next state would need the
/// caller-stated remainder, which a trait impl has nowhere to receive
/// (E0207), or const arithmetic in a type (nightly-only) — and fixed-packet
/// supply forces `combine` chains and makes non-multiple amounts
/// unreachable. Continuous material is drawn by **draw-style boundary
/// processes** instead (R15, [`crate::draw_process!`]). The trybuild cases
/// `tests/ui/finite_container_*.rs` pin both failure shapes.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot supply anything: it is exhausted, or it is not a supplier of discrete items",
    label = "this supplier has nothing left to supply",
    note = "a supplier with items left is e.g. `BoltBox<Cons<..>>`; its empty state (`BoltBox<Nil>`) is a distinct resource that cannot supply (R12)",
    note = "`Supplier` is discrete-only (F-028): continuous material is drawn by a draw process (R15), not supplied through this trait"
)]
pub trait Supplier {
    /// The item supplied on each step.
    type Item;
    /// The supplier after one supply — one item shorter, a different type.
    type Next;
    /// Supplies one item, consuming this supplier state.
    fn supply(self) -> (Self::Item, Self::Next);
}

/// A consumer at the system boundary (R12): consumes exactly one item per
/// step and becomes its next state (one space fewer, the consumed item kept
/// in its contents).
///
/// A finite consumer keeps the objects it has consumed. The one sanctioned
/// exception (R15, F-029): an unbounded boundary sink (`type Next = Self`)
/// necessarily discards its intake; it must be a placeholder at the system
/// boundary.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot consume a `{In}`: it is full, or it does not accept this kind of resource",
    label = "this consumer has no space left for this resource",
    note = "a consumer with space left is e.g. `WasteBag<Succ<..>, _>`; its full state (`WasteBag<Zero, _>`) is a distinct resource that cannot consume (R12)"
)]
pub trait Consumer<In> {
    /// The consumer after consuming one item.
    type Next;
    /// Consumes one item, keeping it (finite consumers) and reducing the
    /// remaining space by one.
    fn consume(self, item: In) -> Self::Next;
}

/// Repeated supply (R12, F-014): takes `N` items from **one** supplier,
/// recursively — taking zero items is trivially possible; taking N+1 items
/// is possible when the supplier can supply once and its next state can
/// supply N more. The two impls do not overlap (the `N` differs), so
/// coherence is satisfied.
///
/// Any N costs one where-clause, and the `Taken =` equality bound doubles as
/// the item-type requirement:
/// `S: SupplyN<N4, Taken = Cons<Bolt, Cons<Bolt, Cons<Bolt, Cons<Bolt, Nil>>>>>`.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot supply this many items: it would run out partway (or it is not a supplier)",
    label = "runs out of items before the requested number is reached",
    note = "`SupplyN<N>` is only satisfied while the supplier still holds at least N items (R12)"
)]
pub trait SupplyN<N> {
    /// The items taken, as a type-level list of real values.
    type Taken;
    /// The depleted supplier.
    type Rest;
    /// Takes `N` items.
    fn supply_n(self) -> (Self::Taken, Self::Rest);
}

impl<S> SupplyN<Zero> for S {
    type Taken = Nil;
    type Rest = S;
    fn supply_n(self) -> (Nil, S) {
        (Nil, self)
    }
}

impl<S: Supplier, N> SupplyN<Succ<N>> for S
where
    S::Next: SupplyN<N>,
{
    type Taken = Cons<S::Item, <S::Next as SupplyN<N>>::Taken>;
    type Rest = <S::Next as SupplyN<N>>::Rest;
    fn supply_n(self) -> (Self::Taken, Self::Rest) {
        let (item, next) = self.supply();
        let (rest_taken, rest) = next.supply_n();
        (Cons(item, rest_taken), rest)
    }
}

/// Repeated consumption (R12, F-014): feeds a whole [`Cons`] list of items —
/// possibly of different types — to **one** consumer, one `consume` per item.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot consume this list of items: it fills up partway, or it does not accept one of them",
    label = "cannot take every item in this list",
    note = "`ConsumeList` needs a `Consumer` impl (with space left) for each item type in the list, in order (R12)"
)]
pub trait ConsumeList<L> {
    /// The consumer after consuming the whole list.
    type Next;
    /// Consumes every item of the list, in order.
    fn consume_list(self, items: L) -> Self::Next;
}

impl<C> ConsumeList<Nil> for C {
    type Next = C;
    fn consume_list(self, Nil: Nil) -> C {
        self
    }
}

impl<C: Consumer<H>, H, T> ConsumeList<Cons<H, T>> for C
where
    C::Next: ConsumeList<T>,
{
    type Next = <C::Next as ConsumeList<T>>::Next;
    fn consume_list(self, items: Cons<H, T>) -> Self::Next {
        let Cons(head, tail) = items;
        self.consume(head).consume_list(tail)
    }
}

// ---------------------------------------------------------------------------
// Generic access processes (F-015): model code reaches boundary objects
// through these bounds, never by direct method calls — the E0599 path of a
// direct call on an exhausted value bypasses `on_unimplemented` entirely.
// ---------------------------------------------------------------------------

/// Takes one item from a supplier (R12). Use this (or a process generic over
/// [`Supplier`]) instead of calling `.supply()` on a concrete value, so
/// capacity mistakes produce the modeller-phrased trait-bound error (F-015).
pub fn take_one<S: Supplier>(supplier: S) -> (S::Item, S::Next) {
    supplier.supply()
}

/// Takes `N` items from one supplier (R12, F-014):
/// `take_n::<N4, _>(boltbox)`.
pub fn take_n<N, S: SupplyN<N>>(supplier: S) -> (S::Taken, S::Rest) {
    supplier.supply_n()
}

/// Hands one item to a consumer (R12). Use this (or a process generic over
/// [`Consumer`]) instead of calling `.consume()` on a concrete value
/// (F-015).
pub fn send_to<I, C: Consumer<I>>(consumer: C, item: I) -> C::Next {
    consumer.consume(item)
}

/// Hands a whole list of items to one consumer (R12, F-014).
pub fn send_list<L, C: ConsumeList<L>>(consumer: C, items: L) -> C::Next {
    consumer.consume_list(items)
}

#[cfg(test)]
mod tests {
    use super::{Consumer, Supplier, send_list, take_n};
    use crate::list::{Cons, Nil};
    use crate::nat::aliases::N3;

    /// A minimal in-module supplier: a stack of numbered tokens held in a
    /// type-level list (the sealed reference fixtures live behind the
    /// `test-support` feature; this one only exercises the trait machinery).
    struct Stack<Items>(Items);

    struct Token(u64);

    impl<T> Supplier for Stack<Cons<Token, T>> {
        type Item = Token;
        type Next = Stack<T>;
        fn supply(self) -> (Token, Stack<T>) {
            let Cons(head, tail) = self.0;
            (head, Stack(tail))
        }
    }

    /// A minimal in-module consumer in the unbounded-sink shape
    /// (`Next = Self`, R15/F-029).
    ///
    /// LIMITATION(candidate finding): a finite consumer that keeps its
    /// contents MUST also carry a decreasing type-level space parameter (the
    /// F-016 `WasteBag<Space, Contents>` shape). An impl that applies to
    /// every state and only grows, such as
    /// `impl<C> Consumer<Token> for Bin<C> { type Next = Bin<Cons<Token, C>>; }`,
    /// sends `ConsumeList` trait resolution into unbounded candidate
    /// exploration: at the default `recursion_limit` (128) that is a graceful
    /// E0275, but at the project-mandated `recursion_limit = "2048"` (F-010)
    /// rustc 1.98.1 exhausts its stack and dies with SIGBUS instead of
    /// reporting an error. Reproduced deterministically while building this
    /// module's tests; minimal repro in the workspace report.
    struct Bin;

    impl Consumer<Token> for Bin {
        type Next = Bin;
        fn consume(self, item: Token) -> Bin {
            let Token(_) = item; // an unbounded sink discards (F-029)
            self
        }
    }

    /// `SupplyN` takes N real items in one bound and returns the depleted
    /// supplier (F-014); the taken list carries the real values in supply
    /// order, and `ConsumeList` feeds them onward one `consume` per item.
    #[test]
    fn supply_n_and_consume_list_round_trip() {
        let stack = Stack(Cons(Token(1), Cons(Token(2), Cons(Token(3), Nil))));
        let (taken, rest) = take_n::<N3, _>(stack);
        let _empty: Stack<Nil> = rest;
        let Cons(Token(a), tail) = taken;
        assert_eq!(a, 1);
        let _bin: Bin = send_list(Bin, tail);
    }
}
