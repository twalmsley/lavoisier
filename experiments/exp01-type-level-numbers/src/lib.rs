//! EXP-01: Type-level natural numbers on stable Rust.
//!
//! Tests R12's assumption that stable Rust can express naturals in types,
//! with decrement and constant values, at useful sizes. Two encodings:
//! unary (Peano) in `peano`, binary (typenum-style) in `binary`, plus
//! generated aliases `N0..N1000` and `BN0..BN1000`.
//!
//! The recursion limit below is what the *library plus its tests* needs;
//! per-size minimums are measured separately in `measure/` and reported
//! in RESULTS.md.
#![recursion_limit = "2048"]

pub mod binary;
pub mod peano;

mod binary_aliases;
mod peano_aliases;

pub use binary_aliases::*;
pub use peano_aliases::*;

/// Asserts at compile time that two types are equal — the workhorse for
/// checking type-level arithmetic. `same::<A, B>()` only compiles when the
/// compiler can prove `A == B` after normalising associated types.
pub fn same<A, B>()
where
    A: SameAs<B>,
{
}

pub trait SameAs<T> {}
impl<T> SameAs<T> for T {}
