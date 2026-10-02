//! Test fixtures and reference boundary implementations (`test-support`
//! feature only — R1, F-004).
//!
//! This module exists so that tests — model-core's own integration tests and
//! downstream crates' tests — have concrete sealed resources, a discrete
//! supplier/consumer pair, a continuous container family, and an unbounded
//! sink to exercise the kernel against. It is **not** part of the production
//! library: a plain `cargo build` omits it, and ci.sh proves that on every
//! run. It also serves as the reference implementation of the R12/R15
//! patterns for modelling crates to adapt.
//!
//! Layout per F-006: the sealed types with their boundary fill functions in
//! one module (this whole module is already test-only, so no separate
//! `test_support` child is needed here).
//!
//! Adapted from `experiments/exp02-supplier-consumer/src/lib.rs` (BoltBox /
//! WasteBag / fill machinery) and
//! `experiments/exp09-continuous-resources/src/lib.rs` (gas family, sinks).
//!
//! Note on tripwires: `Gas`/`GasBottle` are tripwired container resources
//! (R15); `Bolt` is deliberately **not** tripwired, matching the validated
//! EXP-02 design — a `WasteBag` keeps the bolts it consumes (R12), so a
//! tripwired bolt inside an abandoned test bag would panic the test from the
//! bag's own drop. The tripwired-discrete path is exercised by the
//! [`crate::resource`] kernel's own tests and doc-tests instead.

use crate::boundary::{Consumer, Supplier};
use crate::common::Labour;
use crate::list::{Cons, Len, Nil};
use crate::nat::{Succ, Zero};
use core::marker::PhantomData;

// ---------------------------------------------------------------------------
// A discrete resource and its supplier/consumer pair (R12, R13).
// ---------------------------------------------------------------------------

/// A physical bolt (fixture). Sealed (R1): private field, no public
/// constructor, no `Clone`/`Copy`; it cannot be created or duplicated outside
/// this crate's boundary. Not tripwired — see the module docs.
#[must_use = "Bolt is a conserved resource: pass it on or hand it to a Consumer"]
pub struct Bolt {
    _seal: (),
}

/// A box of bolts: a supplier at the system boundary (R12). Its contents are
/// a type-level list of real [`Bolt`] values
/// (`BoltBox<Cons<Bolt, Cons<Bolt, Nil>>>`); the field is private, so the
/// contents cannot be tampered with from outside.
///
/// Placeholder: generic bolt supplier — refine to a named supplier in the
/// modelling crate.
#[must_use = "BoltBox is a boundary resource: pass it on like any other resource"]
pub struct BoltBox<Items>(Items);

/// An exhausted box: a box holding the empty list — a distinct resource type
/// that must itself be accounted for (R12).
pub type EmptyBoltBox = BoltBox<Nil>;

/// `Supplier` is implemented ONLY for a non-empty box; there is no impl for
/// `BoltBox<Nil>`, so supplying from an empty box is a compile error (R12),
/// pinned by `tests/ui/empty_supplier.rs`.
impl<H, T> Supplier for BoltBox<Cons<H, T>> {
    type Item = H;
    type Next = BoltBox<T>;
    fn supply(self) -> (H, BoltBox<T>) {
        let Cons(head, tail) = self.0;
        (head, BoltBox(tail))
    }
}

impl<Items: Len> BoltBox<Items> {
    /// How many bolts the box holds — the length of its contents list, so
    /// count and contents cannot disagree (R7, R12).
    pub const COUNT: u64 = Items::LEN;
}

/// A waste bag: a consumer at the system boundary (R12). `Space` is the
/// type-level number of items it can still accept; `Contents` is the
/// type-level list of the real objects it has consumed so far (a consumer
/// keeps what it consumes; the defaulted `Contents = Nil` keeps construction
/// ergonomic, F-016).
///
/// Placeholder: generic waste bag — refine to a named waste-disposal service
/// in the modelling crate.
#[must_use = "WasteBag is a boundary resource: pass it on like any other resource"]
pub struct WasteBag<Space, Contents = Nil> {
    contents: Contents,
    _space: PhantomData<Space>,
}

/// A full bag: zero space left — a distinct resource type to be accounted
/// for (R12).
pub type FullWasteBag<Contents> = WasteBag<Zero, Contents>;

/// `Consumer` is implemented ONLY while space remains (`Succ<S>`); there is
/// no impl for `WasteBag<Zero, _>`, so consuming into a full bag is a compile
/// error (R12), pinned by `tests/ui/full_consumer.rs`. Consuming pushes the
/// real object onto the contents list and decrements the space (F-016).
impl<S, In, C> Consumer<In> for WasteBag<Succ<S>, C> {
    type Next = WasteBag<S, Cons<In, C>>;
    fn consume(self, item: In) -> Self::Next {
        WasteBag {
            contents: Cons(item, self.contents),
            _space: PhantomData,
        }
    }
}

impl<Space, Contents: Len> WasteBag<Space, Contents> {
    /// How many items the bag holds — the length of its contents list (R7).
    pub const HELD: u64 = Contents::LEN;
}

/// Creates an empty waste bag with `Space` slots. Creating an *empty*
/// consumer brings no resources into existence, so this is an ordinary
/// public boundary function (R12): `new_waste_bag::<N100>()`.
pub fn new_waste_bag<Space>() -> WasteBag<Space, Nil> {
    WasteBag {
        contents: Nil,
        _space: PhantomData,
    }
}

// -- Filling (inside the privacy boundary, R1/F-026) -------------------------

/// Maps a type-level count to the list type of that many bolts:
/// `BoltsOf<N4>` = `Cons<Bolt, Cons<Bolt, Cons<Bolt, Cons<Bolt, Nil>>>>`.
pub trait Bolts {
    /// The list type holding that many bolts.
    type List;
}
impl Bolts for Zero {
    type List = Nil;
}
impl<N: Bolts> Bolts for Succ<N> {
    type List = Cons<Bolt, N::List>;
}
/// Shorthand for [`Bolts::List`].
pub type BoltsOf<N> = <N as Bolts>::List;

/// Builds a list of real bolts. Private: this is the ONLY place bolts come
/// into existence (R1), reachable only through [`full_box`].
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
        Cons(Bolt { _seal: () }, T::fill())
    }
}

/// Public-in-signature but unimplementable-outside wrapper over the private
/// fill machinery, so [`full_box`]'s where-clause can name it without letting
/// outside code implement it (the classic sealed-trait pattern, F-026).
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
    impl Sealed for crate::list::Nil {}
    impl<T: Sealed> Sealed for crate::list::Cons<super::Bolt, T> {}
}

/// The fill function (R12): creates a full box of N real bolts. This is the
/// system boundary where bolts enter the model: `full_box::<N4>()`.
///
/// Placeholder: generic bolt supplier.
pub fn full_box<N: Bolts>() -> BoltBox<BoltsOf<N>>
where
    BoltsOf<N>: FillSealed,
{
    BoltBox(FillSealed::fill_sealed())
}

// ---------------------------------------------------------------------------
// A continuous container family (R15), built with the resource kernel macros.
// ---------------------------------------------------------------------------

crate::container_resource! {
    /// Drawn gas (fuel), in grams (fixture).
    Gas,
    unit = "grams",
    must_use = "Gas is a conserved resource: pass it on or hand it to a Consumer"
}

crate::container_resource! {
    /// A gas bottle holding `V` grams of gas (fixture). `GasBottle<0>` is the
    /// empty state: a distinct resource type that must still be accounted
    /// for, exactly like an empty bolt box (R12, R15).
    GasBottle,
    unit = "grams remaining",
    must_use = "GasBottle is a conserved resource: even an empty bottle must be passed on or handed to a Consumer"
}

/// The empty state of the gas bottle — a distinct resource type (R15).
pub type EmptyGasBottle = GasBottle<0>;

crate::draw_process! {
    /// Draws `TAKE` grams from a bottle holding `FULL` grams, leaving `LEFT`
    /// (R15 caller-stated-remainder pattern). Overdrawing is a compile error;
    /// the regression lives in the kernel's doc-tests
    /// ([`crate::resource`]).
    pub fn draw_gas: GasBottle => Gas,
    assert = "conservation violated in draw_gas (R15): TAKE + LEFT must equal FULL - is the draw larger than the bottle's remaining contents?"
}

/// Fills a gas bottle with `FULL` grams at the system boundary (R12).
///
/// Placeholder: gas supplier — assumed able to deliver a full bottle.
pub fn fill_gas_bottle<const FULL: u64>() -> GasBottle<FULL> {
    GasBottle::mint()
}

// ---------------------------------------------------------------------------
// An unbounded boundary sink (R15, F-029).
// ---------------------------------------------------------------------------

/// An unbounded test sink: `type Next = Self` in every `Consumer` impl, the
/// pattern R15 licenses **only at the system boundary**. One boundary object
/// may consume several different resources (F-029); being unbounded, it
/// necessarily discards its intake — the one sanctioned exception to "a
/// consumer keeps what it consumes" (R12).
///
/// Placeholder: generic unbounded sink — in a real model this is e.g. the
/// atmosphere, a bottle depot, or a labour ledger, each refined separately.
#[must_use = "TestSink is a boundary resource: pass it on like any other resource"]
pub struct TestSink {
    _seal: (),
}

/// Creates the unbounded test sink. Creating an empty sink brings no
/// resources into existence, so this is an ordinary public boundary function
/// (R12).
pub fn new_test_sink() -> TestSink {
    TestSink { _seal: () }
}

/// The sink accepts drawn gas of any magnitude; `Next = Self` (R15).
impl<const G: u64> Consumer<Gas<G>> for TestSink {
    type Next = TestSink;
    fn consume(self, item: Gas<G>) -> TestSink {
        item.defuse();
        self
    }
}

/// The sink accepts a returned bottle in any fill state; `Next = Self`
/// (R15: even the empty bottle must be accounted for).
impl<const R: u64> Consumer<GasBottle<R>> for TestSink {
    type Next = TestSink;
    fn consume(self, item: GasBottle<R>) -> TestSink {
        item.defuse();
        self
    }
}

/// The sink accounts for any amount of expended labour; `Next = Self`.
impl<const MS: u64> Consumer<Labour<MS>> for TestSink {
    type Next = TestSink;
    fn consume(self, item: Labour<MS>) -> TestSink {
        item.defuse();
        self
    }
}
