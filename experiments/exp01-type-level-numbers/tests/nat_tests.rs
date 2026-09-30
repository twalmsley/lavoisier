//! Unit tests for both encodings: constant values, addition, decrement,
//! and the less-than stretch goal.
#![recursion_limit = "2048"]

use exp01_type_level_numbers::binary::{self, Dec, DecRaw, Inc, UInt, UTerm, B0};
use exp01_type_level_numbers::peano::{self, Lt, Nat, Pred, Succ, Zero};
use exp01_type_level_numbers::{same, *};

// ---------- Constant values (R7: types carry values) ----------

#[test]
fn peano_values() {
    assert_eq!(N0::VALUE, 0);
    assert_eq!(N1::VALUE, 1);
    assert_eq!(N42::VALUE, 42);
    assert_eq!(N100::VALUE, 100);
    assert_eq!(N1000::VALUE, 1000);
}

#[test]
fn binary_values() {
    assert_eq!(BN0::VALUE, 0);
    assert_eq!(BN1::VALUE, 1);
    assert_eq!(BN42::VALUE, 42);
    assert_eq!(BN100::VALUE, 100);
    assert_eq!(BN1000::VALUE, 1000);
}

#[test]
fn values_agree_across_encodings() {
    assert_eq!(N500::VALUE, BN500::VALUE);
}

// ---------- Addition ----------

#[test]
fn peano_addition_small() {
    // 2 + 3 == 5, proven by the type checker.
    same::<<N2 as peano::Add<N3>>::Sum, N5>();
    // 0 is the identity.
    same::<<N0 as peano::Add<N42>>::Sum, N42>();
    same::<<N42 as peano::Add<N0>>::Sum, N42>();
}

#[test]
fn peano_addition_large() {
    // 500 + 500 == 1000 — forces the solver 500 levels deep.
    same::<<N500 as peano::Add<N500>>::Sum, N1000>();
    assert_eq!(<N500 as peano::Add<N500>>::Sum::VALUE, 1000);
}

#[test]
fn binary_addition_small() {
    same::<<BN2 as binary::Add<BN3>>::Sum, BN5>();
    // Carry chains: 1 + 1 = 2, 3 + 5 = 8, 7 + 1 = 8.
    same::<<BN1 as binary::Add<BN1>>::Sum, BN2>();
    same::<<BN3 as binary::Add<BN5>>::Sum, BN8>();
    same::<<BN7 as binary::Add<BN1>>::Sum, BN8>();
    same::<<BN0 as binary::Add<BN42>>::Sum, BN42>();
    same::<<BN42 as binary::Add<BN0>>::Sum, BN42>();
}

#[test]
fn binary_addition_large() {
    same::<<BN500 as binary::Add<BN500>>::Sum, BN1000>();
    same::<<BN499 as binary::Add<BN501>>::Sum, BN1000>();
    assert_eq!(<BN500 as binary::Add<BN500>>::Sum::VALUE, 1000);
}

// ---------- Decrement (the operation R12 actually needs) ----------

#[test]
fn peano_decrement() {
    same::<<N42 as Pred>::Out, N41>();
    same::<<N1 as Pred>::Out, N0>();
    assert_eq!(<N1000 as Pred>::Out::VALUE, 999);
}

#[test]
fn binary_decrement() {
    same::<<BN42 as Dec>::Out, BN41>();
    same::<<BN1000 as Dec>::Out, BN999>();
    // Decrementing across a borrow chain: 512 - 1 = 511.
    same::<<BN512 as Dec>::Out, BN511>();
}

/// FINDING (see RESULTS.md): *raw* binary decrement leaves leading zeros —
/// 1 - 1 is `UInt<UTerm, B0>`, and 512 - 1 is a 10-bit 0b0111111111. Both
/// have the right VALUE but are different types from the canonical forms,
/// so type-level comparisons fail. The public `Dec` therefore has to run a
/// `Trim` normalisation pass; the compile-fail test dec_raw_not_canonical.rs
/// shows what happens without it.
#[test]
fn binary_raw_decrement_is_noncanonical() {
    // The values are right...
    assert_eq!(<BN1 as DecRaw>::Out::VALUE, 0);
    assert_eq!(<BN512 as DecRaw>::Out::VALUE, 511);
    // ...but the raw types keep a leading zero:
    same::<<BN1 as DecRaw>::Out, UInt<UTerm, B0>>();
    // The canonical Dec fixes both:
    same::<<BN1 as Dec>::Out, BN0>();
    same::<<BN512 as Dec>::Out, BN511>();
}

#[test]
fn binary_increment() {
    same::<<BN41 as Inc>::Out, BN42>();
    same::<<BN511 as Inc>::Out, BN512>();
    same::<<BN999 as Inc>::Out, BN1000>();
}

// ---------- Less-than (stretch goal) ----------

/// A function only callable when A < B — the shape a capacity bound
/// would take in the real model.
fn requires_lt<A: Lt<B>, B: Nat>() -> (u64, u64) {
    (A::VALUE, B::VALUE)
}

#[test]
fn peano_less_than() {
    assert_eq!(requires_lt::<N41, N42>(), (41, 42));
    assert_eq!(requires_lt::<N0, N1>(), (0, 1));
    assert_eq!(requires_lt::<N0, N1000>(), (0, 1000));
    assert_eq!(requires_lt::<N999, N1000>(), (999, 1000));
    // requires_lt::<N42, N42>() and requires_lt::<N42, N41>() do not
    // compile — demonstrated in tests/compile_fail/lt_not_reflexive.rs.
}

// ---------- Ergonomics without aliases (kept small on purpose) ----------

#[test]
fn spelled_out_forms() {
    // Peano 4, written by hand: already unwieldy.
    same::<Succ<Succ<Succ<Succ<Zero>>>>, N4>();
    // Binary 4 (0b100), written by hand: shallow but inside-out.
    same::<UInt<UInt<UInt<UTerm, binary::B1>, B0>, B0>, BN4>();
}
