//! EXP-02: Suppliers and consumers as type-level lists.
//!
//! Tests the R12 design end to end: a `BoltBox` whose contents are a
//! type-level Cons-list of real `Bolt` values, a `WasteBag` with type-level
//! remaining space, `Supplier`/`Consumer` traits implemented only while
//! capacity remains, and a generic process that takes four bolts from one
//! supplier (the hard part).
//!
//! Everything that may create resources lives inside the `boundary` module
//! (the privacy boundary, R1). Code outside it — including this crate's
//! integration tests — can only obtain a `Bolt` from a supplier.

// The default recursion_limit (128) is enough at capacity 100, but fails at
// capacity 130 with E0275 "overflow evaluating the requirement". Raised here
// so the design has headroom; see RESULTS.md for the measurement.
#![recursion_limit = "256"]

use core::marker::PhantomData;

// ---------------------------------------------------------------------------
// Type-level Peano naturals (minimal copy — deliberately not shared with
// EXP-01, per the common protocol).
// ---------------------------------------------------------------------------

/// Type-level zero.
pub struct Zero;

/// Type-level successor: `Succ<N>` is N + 1.
pub struct Succ<N>(PhantomData<N>);

/// Every type-level natural exposes its value as a constant (R7).
pub trait Nat {
    const VALUE: u64;
}

impl Nat for Zero {
    const VALUE: u64 = 0;
}

impl<N: Nat> Nat for Succ<N> {
    const VALUE: u64 = N::VALUE + 1;
}

/// Builds a Peano number from a sequence of tokens: `nat!(x x x)` = 3.
macro_rules! nat {
    () => { Zero };
    ($head:tt $($rest:tt)*) => { Succ<nat!($($rest)*)> };
}

pub type N0 = nat!();
pub type N1 = nat!(x);
pub type N2 = nat!(x x);
pub type N3 = nat!(x x x);
pub type N4 = nat!(x x x x);
pub type N5 = nat!(x x x x x);
#[rustfmt::skip]
pub type N100 = nat!(
    x x x x x x x x x x  x x x x x x x x x x
    x x x x x x x x x x  x x x x x x x x x x
    x x x x x x x x x x  x x x x x x x x x x
    x x x x x x x x x x  x x x x x x x x x x
    x x x x x x x x x x  x x x x x x x x x x
);

// ---------------------------------------------------------------------------
// Type-level lists holding real values (R12: suppliers hold real objects).
// ---------------------------------------------------------------------------

/// The empty list.
pub struct Nil;

/// A list cell holding a real value of type `H` and the rest of the list.
/// The fields are private: only the boundary module builds these directly.
pub struct Cons<H, T>(H, T);

/// The length of a type-level list, exposed as a constant (R7). Because the
/// count is literally the length of the list, count and contents can never
/// disagree.
pub trait Len {
    const LEN: u64;
}

impl Len for Nil {
    const LEN: u64 = 0;
}

impl<H, T: Len> Len for Cons<H, T> {
    const LEN: u64 = T::LEN + 1;
}

// ---------------------------------------------------------------------------
// The privacy boundary (R1). Only code in this module can create a `Bolt`
// or reach inside a `BoltBox` / `WasteBag`.
// ---------------------------------------------------------------------------

mod boundary {
    use super::{Cons, Nil, Succ, Zero};

    /// A physical bolt. Private field, no public constructor, no
    /// `Clone`/`Copy`: it cannot be created or duplicated outside this
    /// module (R1).
    pub struct Bolt {
        _private: (),
    }

    /// A box of bolts. Its contents are a type-level list of real `Bolt`
    /// values, e.g. `BoltBox<Cons<Bolt, Cons<Bolt, Nil>>>`. The field is
    /// private, so the contents cannot be tampered with from outside.
    pub struct BoltBox<Items>(Items);

    /// An exhausted box is just a box with an empty list — a distinct type
    /// that must itself be accounted for (R12).
    pub type EmptyBoltBox = BoltBox<Nil>;

    /// A supplier at the system boundary (R12). Supplying consumes the
    /// supplier by value and returns one item plus the supplier's next
    /// state, which is a *different type* (one item shorter).
    #[diagnostic::on_unimplemented(
        message = "`{Self}` cannot supply anything: it is empty (or not a supplier at all)",
        label = "this supplier has nothing left to supply",
        note = "a `BoltBox<Cons<..>>` still contains bolts and can supply; `EmptyBoltBox` (= `BoltBox<Nil>`) is empty and cannot"
    )]
    pub trait Supplier {
        type Item;
        type Next;
        fn supply(self) -> (Self::Item, Self::Next);
    }

    /// `Supplier` is implemented ONLY for a non-empty box. There is no impl
    /// for `BoltBox<Nil>`, so supplying from an empty box is a compile error.
    impl<H, T> Supplier for BoltBox<Cons<H, T>> {
        type Item = H;
        type Next = BoltBox<T>;
        fn supply(self) -> (H, BoltBox<T>) {
            let Cons(head, tail) = self.0;
            (head, BoltBox(tail))
        }
    }

    /// A consumer at the system boundary (R12). `Space` is the type-level
    /// number of items it can still accept; `Contents` is the type-level
    /// list of the real objects it has consumed so far (a consumer keeps
    /// what it consumes).
    ///
    /// Placeholder: represents a generic waste bag; refine to a named
    /// waste-disposal service later.
    pub struct WasteBag<Space, Contents = Nil> {
        contents: Contents,
        _space: core::marker::PhantomData<Space>,
    }

    /// A full bag has zero space left — a distinct type to be accounted for.
    pub type FullWasteBag<Contents> = WasteBag<Zero, Contents>;

    /// Creating an *empty* waste bag creates no resources, so this is public.
    pub fn new_waste_bag<Space>() -> WasteBag<Space, Nil> {
        WasteBag {
            contents: Nil,
            _space: core::marker::PhantomData,
        }
    }

    #[diagnostic::on_unimplemented(
        message = "`{Self}` cannot consume a `{In}`: it is full (or not a consumer of `{In}`)",
        label = "this consumer has no space left",
        note = "a `WasteBag<Succ<..>, _>` still has space; `WasteBag<Zero, _>` is full and cannot consume"
    )]
    pub trait Consumer<In> {
        type Next;
        fn consume(self, item: In) -> Self::Next;
    }

    /// `Consumer` is implemented ONLY while space remains (`Succ<S>`).
    /// There is no impl for `WasteBag<Zero, _>`, so consuming into a full
    /// bag is a compile error. Consuming pushes the real object onto the
    /// contents list and decrements the space.
    impl<S, In, C> Consumer<In> for WasteBag<Succ<S>, C> {
        type Next = WasteBag<S, Cons<In, C>>;
        fn consume(self, item: In) -> Self::Next {
            WasteBag {
                contents: Cons(item, self.contents),
                _space: core::marker::PhantomData,
            }
        }
    }

    impl<Space, Contents: super::Len> WasteBag<Space, Contents> {
        /// How many items the bag holds — the length of its contents list.
        pub const HELD: u64 = Contents::LEN;
    }

    // -- Filling (inside the privacy boundary) ------------------------------

    /// Maps a type-level count to the list type of that many bolts:
    /// `BoltsOf<N100>` = `Cons<Bolt, Cons<Bolt, ...>>`, 100 deep.
    pub trait Bolts {
        type List;
    }
    impl Bolts for Zero {
        type List = Nil;
    }
    impl<N: Bolts> Bolts for Succ<N> {
        type List = Cons<Bolt, N::List>;
    }
    pub type BoltsOf<N> = <N as Bolts>::List;

    /// Builds a list of real bolts. Private: this is the ONLY place bolts
    /// come into existence, and it is reachable only through `full_box`.
    trait Fill {
        fn fill() -> Self;
    }
    impl Fill for Nil {
        fn fill() -> Nil {
            Nil
        }
    }
    impl<T: Fill> Fill for Cons<Bolt, T> {
        fn fill() -> Self {
            Cons(Bolt { _private: () }, T::fill())
        }
    }

    /// The fill function (R12): creates a full box of N real bolts. This is
    /// the system boundary where bolts enter the model.
    pub fn full_box<N: Bolts>() -> BoltBox<BoltsOf<N>>
    where
        BoltsOf<N>: FillSealed,
    {
        BoltBox(FillSealed::fill_sealed())
    }

    /// Public-in-signature but unimplementable-outside wrapper over `Fill`,
    /// so `full_box`'s where-clause can name it without exposing `Fill`.
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
        pub trait Sealed {}
        impl Sealed for super::Nil {}
        impl<T: Sealed> Sealed for super::Cons<super::Bolt, T> {}
    }

    impl<Items: super::Len> BoltBox<Items> {
        /// How many bolts the box holds — the length of its list (R7).
        pub const COUNT: u64 = Items::LEN;
    }
}

pub use boundary::{
    Bolt, BoltBox, Bolts, BoltsOf, Consumer, EmptyBoltBox, FullWasteBag, Supplier, WasteBag,
    full_box, new_waste_bag,
};

// ---------------------------------------------------------------------------
// Step 6(b): a recursive SupplyN trait — take N items in one bound.
// ---------------------------------------------------------------------------

/// Supplies `N` items from a supplier, returning them as a Cons-list plus
/// the depleted supplier. Implemented recursively: taking zero items is
/// trivially possible; taking N+1 items is possible when the supplier can
/// supply once and its next state can supply N more.
#[diagnostic::on_unimplemented(
    message = "cannot take that many items from `{Self}`: it does not hold enough (or is not a supplier)",
    label = "runs out of items before the requested number is reached",
    note = "`SupplyN<N>` is only satisfied while the supplier still holds at least N items"
)]
pub trait SupplyN<N> {
    /// The items taken, as a type-level list of real values.
    type Taken;
    /// The depleted supplier.
    type Rest;
    fn supply_n(self) -> (Self::Taken, Self::Rest);
}

impl<S> SupplyN<Zero> for S {
    type Taken = Nil;
    type Rest = S;
    fn supply_n(self) -> (Nil, S) {
        (Nil, self)
    }
}

impl<S, N> SupplyN<Succ<N>> for S
where
    S: Supplier,
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

/// Free-function form so the count can be given by turbofish:
/// `supply_n::<N4, _>(box)`.
pub fn supply_n<N, S: SupplyN<N>>(supplier: S) -> (S::Taken, S::Rest) {
    supplier.supply_n()
}

// ---------------------------------------------------------------------------
// Consuming a whole list into a consumer (companion to SupplyN, so a
// supplier's output can be fed straight into a consumer).
// ---------------------------------------------------------------------------

/// Consumes every item of a Cons-list, one `consume` per item (R12: one
/// item per step — this is just the repetition written as recursion).
pub trait ConsumeList<L> {
    type Next;
    fn consume_list(self, items: L) -> Self::Next;
}

impl<C> ConsumeList<Nil> for C {
    type Next = C;
    fn consume_list(self, _items: Nil) -> C {
        self
    }
}

impl<C, H, T> ConsumeList<Cons<H, T>> for C
where
    C: Consumer<H>,
    C::Next: ConsumeList<T>,
{
    type Next = <C::Next as ConsumeList<T>>::Next;
    fn consume_list(self, items: Cons<H, T>) -> Self::Next {
        let Cons(head, tail) = items;
        self.consume(head).consume_list(tail)
    }
}

// ---------------------------------------------------------------------------
// Step 6: the KEY test — a generic process taking four bolts from ONE
// supplier. Two attempts, per the experiment brief.
// ---------------------------------------------------------------------------

/// Shorthand for "the supplier after one more supply".
pub type Next1<S> = <S as Supplier>::Next;
pub type Next2<S> = Next1<Next1<S>>;
pub type Next3<S> = Next1<Next2<S>>;
pub type Next4<S> = Next1<Next3<S>>;

/// (a) Hand-written chained bounds: one where-clause per bolt taken.
///
/// Four bolts need FOUR where-clauses, and without the `NextK` aliases the
/// fourth reads
/// `<<<S as Supplier>::Next as Supplier>::Next as Supplier>::Next: Supplier<Item = Bolt>`.
pub fn fasten_four_chained<S>(s: S) -> ([Bolt; 4], Next4<S>)
where
    S: Supplier<Item = Bolt>,
    Next1<S>: Supplier<Item = Bolt>,
    Next2<S>: Supplier<Item = Bolt>,
    Next3<S>: Supplier<Item = Bolt>,
{
    let (b1, s) = s.supply();
    let (b2, s) = s.supply();
    let (b3, s) = s.supply();
    let (b4, s) = s.supply();
    ([b1, b2, b3, b4], s)
}

/// The list type of exactly four bolts.
pub type FourBolts = Cons<Bolt, Cons<Bolt, Cons<Bolt, Cons<Bolt, Nil>>>>;

/// (b) The recursive `SupplyN` trait: ONE where-clause, whatever N is.
/// The `Taken = FourBolts` equality bound also pins the item type to `Bolt`,
/// so this process only accepts suppliers of bolts.
pub fn fasten_four_supplyn<S>(s: S) -> (FourBolts, S::Rest)
where
    S: SupplyN<N4, Taken = FourBolts>,
{
    s.supply_n()
}

// ---------------------------------------------------------------------------
// Capacity-100 stress: non-generic, so building the library forces the
// compiler through the full 100-deep instantiation (fill 100 bolts, supply
// all 100 through SupplyN<N100>, consume all 100 into a WasteBag<N100>).
// ---------------------------------------------------------------------------

/// Returns (bolts the box held, bolts the bag now holds, space left in bag).
pub fn demo_capacity_100() -> (u64, u64, u64) {
    let bx: BoltBox<BoltsOf<N100>> = full_box::<N100>();
    let held = BoltBox::<BoltsOf<N100>>::COUNT;
    let (taken, rest) = supply_n::<N100, _>(bx);
    let _empty: EmptyBoltBox = rest; // the exhausted box is a new resource
    let bag = new_waste_bag::<N100>();
    let full: FullWasteBag<_> = bag.consume_list(taken);
    let in_bag = FullWasteBag::<BoltsOf<N100>>::HELD;
    let _to_account_for = full; // a full bag is itself a resource (R12)
    (held, in_bag, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies: peano constants (R7).
    #[test]
    fn nat_values() {
        assert_eq!(N0::VALUE, 0);
        assert_eq!(N4::VALUE, 4);
        assert_eq!(N100::VALUE, 100);
    }

    /// Verifies: count is the list length, so count and contents agree (R12).
    #[test]
    fn box_count_is_list_length() {
        assert_eq!(BoltBox::<BoltsOf<N5>>::COUNT, 5);
        assert_eq!(EmptyBoltBox::COUNT, 0);
    }

    /// Verifies: the capacity-100 round trip (R12).
    #[test]
    fn capacity_100_round_trip() {
        assert_eq!(demo_capacity_100(), (100, 100, 0));
    }
}
