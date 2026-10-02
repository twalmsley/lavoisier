//! Type-level Peano natural numbers (R12).
//!
//! `Zero` is 0 and `Succ<N>` is N + 1; every number is a distinct type, and
//! [`Nat`] exposes the numeric value as a constant (R7). Stable Rust cannot do
//! arithmetic on const generics (`N - 1`), so supplier/consumer capacities
//! (R12) are counted with these instead. The Peano encoding was adopted by
//! experiment (F-012): supplier contents are a `Cons` list whose type is
//! already O(N) deep, so a binary counter would save nothing, and `Succ`
//! mirrors `Cons` one-to-one.
//!
//! Adapted from `experiments/exp01-type-level-numbers/src/peano.rs`, minus the
//! `Default` impls: in the real library these numbers appear only as phantom
//! type parameters, never as values.
//!
//! ## Costs, measured (F-009, F-010, F-011)
//!
//! * Every crate *using* these numbers needs `#![recursion_limit = "2048"]`
//!   (Peano needs ≈ N + 3; the default 128 allows capacity ~100 but not 130).
//!   E0275 "overflow evaluating the requirement" means "the number is too big
//!   for the current limit".
//! * Keep capacities ≤ ~500 where convenient: compile time is flat to N≈500
//!   and superlinear after.
//! * Diagnostics erase the [`aliases`] and print raw `Succ<…>` chains with no
//!   decimal value (F-009); wrap numbers in domain types whose outer name
//!   survives in errors.
//! * Magnitudes (2000 g) are **not** encoded this way — that is what the
//!   const-generic quantities in [`crate::quantity`] are for (F-011).
//!
//! Decrementing zero is a dead end by design, with a modeller-phrased
//! diagnostic (R12): an exhausted capacity cannot be counted down further.
//!
//! ```compile_fail
//! use model_core::nat::{Pred, aliases::N0};
//! // N0 has no predecessor: "cannot count `Zero` down" (E0277).
//! fn dec<N: Pred>() {}
//! fn main() { dec::<N0>(); }
//! ```

use core::marker::PhantomData;

pub mod aliases;

/// The type-level natural number 0.
pub struct Zero;

/// The successor of `N`: the type-level natural number N + 1.
pub struct Succ<N>(PhantomData<N>);

/// A type-level natural number that exposes its value as a constant (R7,
/// R12).
pub trait Nat {
    /// The numeric value of this type-level number.
    const VALUE: u64;
}

impl Nat for Zero {
    const VALUE: u64 = 0;
}

impl<N: Nat> Nat for Succ<N> {
    const VALUE: u64 = N::VALUE + 1;
}

/// Type-level predecessor (decrement): `<Succ<N> as Pred>::Out` is N.
///
/// Deliberately **not** implemented for [`Zero`], so decrementing zero is a
/// compile error — exactly what R12 needs for an exhausted supplier or a full
/// consumer.
#[diagnostic::on_unimplemented(
    message = "cannot count `{Self}` down: there is nothing left (type-level zero has no predecessor)",
    label = "this capacity is already exhausted",
    note = "a supplier or consumer with no capacity left cannot be used again (R12): refill or replace it at the system boundary"
)]
pub trait Pred: Nat {
    /// The predecessor (this number minus one).
    type Out: Nat;
}

impl<N: Nat> Pred for Succ<N> {
    type Out = N;
}

/// Type-level addition: `<A as Add<B>>::Sum` is A + B.
pub trait Add<B: Nat>: Nat {
    /// The sum of the two numbers.
    type Sum: Nat;
}

impl<B: Nat> Add<B> for Zero {
    type Sum = B;
}

impl<A: Add<B>, B: Nat> Add<B> for Succ<A> {
    type Sum = Succ<A::Sum>;
}

/// Type-level less-than: `A: Lt<B>` holds iff A < B.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not less than `{B}`: the count exceeds the available capacity",
    label = "too many for the capacity here",
    note = "capacity bounds are type-checked (R12): use a bigger supplier/consumer at the boundary, or ask for fewer items"
)]
pub trait Lt<B: Nat>: Nat {}

impl<B: Nat> Lt<Succ<B>> for Zero {}

impl<A: Lt<B>, B: Nat> Lt<Succ<B>> for Succ<A> {}

#[cfg(test)]
mod tests {
    use super::aliases::{N0, N1, N2, N3, N4, N5, N100, N1000};
    use super::{Add, Lt, Nat, Pred};
    use core::marker::PhantomData;

    /// The values of the generated aliases are their indices (R7), up to and
    /// including N1000 (which exercises the `recursion_limit = "2048"`
    /// headroom, F-010).
    #[test]
    fn alias_values_match_their_indices() {
        assert_eq!(N0::VALUE, 0);
        assert_eq!(N1::VALUE, 1);
        assert_eq!(N4::VALUE, 4);
        assert_eq!(N100::VALUE, 100);
        assert_eq!(N1000::VALUE, 1000);
    }

    /// `Pred` counts down by exactly one and exposes the value.
    #[test]
    fn pred_counts_down() {
        assert_eq!(<N5 as Pred>::Out::VALUE, 4);
        assert_eq!(<N1 as Pred>::Out::VALUE, 0);
    }

    /// `Add` computes the sum both as a type and as a value.
    #[test]
    fn add_computes_sum_type_and_value() {
        assert_eq!(<N2 as Add<N3>>::Sum::VALUE, 5);
        // Type-level equality: the annotation only compiles if the computed
        // Sum type IS N5.
        let _proof: PhantomData<<N2 as Add<N3>>::Sum> = PhantomData::<N5>;
    }

    fn requires_lt<A: Lt<B>, B: Nat>() {}

    /// `Lt` holds for strictly smaller numbers (its negative direction is a
    /// compile error, shown as a `compile_fail` doc-test in the module docs).
    #[test]
    fn lt_holds_for_smaller_numbers() {
        requires_lt::<N0, N1>();
        requires_lt::<N3, N100>();
    }
}
