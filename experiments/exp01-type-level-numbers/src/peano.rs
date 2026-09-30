//! Peano (unary) type-level natural numbers.
//!
//! `Zero` is 0, `Succ<N>` is N + 1. Every number is a distinct type, and the
//! `Nat` trait exposes the numeric value as a constant (R7/R12).

use core::marker::PhantomData;

/// The natural number 0.
#[derive(Debug)]
pub struct Zero;

/// The successor of `N`: the natural number N + 1.
#[derive(Debug)]
pub struct Succ<N>(PhantomData<N>);

/// A type-level natural number that exposes its value as a constant.
pub trait Nat {
    const VALUE: u64;
}

impl Nat for Zero {
    const VALUE: u64 = 0;
}

impl<N: Nat> Nat for Succ<N> {
    const VALUE: u64 = N::VALUE + 1;
}

// The numbers are zero-sized; giving them `Default` lets tests and demos
// construct values without a public field. (In the real model, resources
// would carry these only as phantom parameters.)
impl Default for Zero {
    fn default() -> Self {
        Zero
    }
}

impl<N> Default for Succ<N> {
    fn default() -> Self {
        Succ(PhantomData)
    }
}

/// Type-level addition: `<A as Add<B>>::Sum` is A + B.
pub trait Add<B: Nat>: Nat {
    type Sum: Nat;
}

impl<B: Nat> Add<B> for Zero {
    type Sum = B;
}

impl<A: Add<B>, B: Nat> Add<B> for Succ<A> {
    type Sum = Succ<A::Sum>;
}

/// Type-level predecessor (decrement): `<Succ<N> as Pred>::Out` is N.
/// Deliberately not implemented for `Zero`, so decrementing zero is a
/// compile error — exactly what R12 needs for an exhausted supplier.
pub trait Pred: Nat {
    type Out: Nat;
}

impl<N: Nat> Pred for Succ<N> {
    type Out = N;
}

/// Stretch goal — type-level less-than: `A: Lt<B>` holds iff A < B.
pub trait Lt<B: Nat>: Nat {}

impl<B: Nat> Lt<Succ<B>> for Zero {}

impl<A: Lt<B>, B: Nat> Lt<Succ<B>> for Succ<A> {}

/// Less-than-or-equal, useful for capacity bounds.
pub trait Le<B: Nat>: Nat {}

impl<B: Nat> Le<B> for Zero {}

impl<A: Le<B>, B: Nat> Le<Succ<B>> for Succ<A> {}
