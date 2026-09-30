//! Binary type-level natural numbers (typenum-style, but written from
//! scratch — external crates are forbidden by R8).
//!
//! A number is a list of bits with the most significant bit innermost:
//! `UTerm` is the empty number (0), `UInt<U, B>` is `2 * U + B`.
//! So 6 = 0b110 = `UInt<UInt<UInt<UTerm, B1>, B1>, B0>`.
//! Depth grows with log2(N) instead of N, which is the whole point.

use core::marker::PhantomData;

use crate::peano::Nat;

/// A single bit.
pub trait Bit {
    const BIT: u64;
}

/// The bit 0.
#[derive(Debug, Default)]
pub struct B0;

/// The bit 1.
#[derive(Debug, Default)]
pub struct B1;

impl Bit for B0 {
    const BIT: u64 = 0;
}

impl Bit for B1 {
    const BIT: u64 = 1;
}

/// The terminator: the empty bit string, i.e. 0.
#[derive(Debug, Default)]
pub struct UTerm;

/// `UInt<U, B>` is the number `2 * U + B` (append bit `B` to `U`).
#[derive(Debug)]
pub struct UInt<U, B>(PhantomData<(U, B)>);

impl<U, B> Default for UInt<U, B> {
    fn default() -> Self {
        UInt(PhantomData)
    }
}

// The same `Nat` trait as the Peano encoding, so both expose VALUE
// the same way.
impl Nat for UTerm {
    const VALUE: u64 = 0;
}

impl<U: Nat, B: Bit> Nat for UInt<U, B> {
    const VALUE: u64 = 2 * U::VALUE + B::BIT;
}

/// Well-formedness marker: a canonical (no leading `B0` on `UTerm`)
/// unsigned binary number. Keeps `Inc`/`Add` outputs canonical.
pub trait Unsigned: Nat {}
impl Unsigned for UTerm {}
impl<U: Unsigned, B: Bit> Unsigned for UInt<U, B> where UInt<U, B>: Nat {}

/// Type-level increment: `<N as Inc>::Out` is N + 1.
pub trait Inc: Nat {
    type Out: Nat;
}

impl Inc for UTerm {
    // 0 + 1 = 1
    type Out = UInt<UTerm, B1>;
}

impl<U: Nat> Inc for UInt<U, B0> {
    // ...0 + 1 = ...1
    type Out = UInt<U, B1>;
}

impl<U: Inc> Inc for UInt<U, B1> {
    // ...1 + 1 = carry: (U+1)0
    type Out = UInt<U::Out, B0>;
}

/// Raw type-level decrement: `<N as DecRaw>::Out` is N - 1, but the result
/// may carry leading zero bits (e.g. 0b1000 - 1 = 0b0111, and 1 - 1 = 0b0).
/// Not implemented for `UTerm` (zero), so decrementing zero is a compile
/// error, as R12 needs.
pub trait DecRaw: Nat {
    type Out: Nat;
}

impl<U: Nat> DecRaw for UInt<U, B1> {
    // ...1 - 1 = ...0
    type Out = UInt<U, B0>;
}

impl<U: DecRaw> DecRaw for UInt<U, B0> {
    // ...0 - 1 = borrow: (U-1)1
    type Out = UInt<U::Out, B1>;
}

/// Strips leading zeros so numbers stay in canonical form. Needed because
/// `DecRaw` of a power of two leaves a leading `B0` (0b1000 - 1 = 0b0111),
/// which has the right VALUE but is a *different type* from 0b111.
pub trait Trim: Nat {
    type Out: Nat;
}

/// Helper for `Trim`: appends bit `B` to an already-trimmed prefix `Self`,
/// dropping the bit's slot if it would create a leading zero.
pub trait TrimmedCons<B: Bit>: Nat {
    type Out: Nat;
}

impl TrimmedCons<B0> for UTerm {
    // A zero appended to nothing is still nothing.
    type Out = UTerm;
}

impl TrimmedCons<B1> for UTerm {
    type Out = UInt<UTerm, B1>;
}

impl<U: Nat, B2: Bit, B: Bit> TrimmedCons<B> for UInt<U, B2> {
    type Out = UInt<UInt<U, B2>, B>;
}

impl Trim for UTerm {
    type Out = UTerm;
}

impl<U: Trim, B: Bit> Trim for UInt<U, B>
where
    U::Out: TrimmedCons<B>,
{
    type Out = <U::Out as TrimmedCons<B>>::Out;
}

/// Type-level decrement, canonical: `<N as Dec>::Out` is N - 1 with no
/// leading zeros. `DecRaw` + `Trim` under one name.
pub trait Dec: Nat {
    type Out: Nat;
}

impl<N: DecRaw> Dec for N
where
    N::Out: Trim,
{
    type Out = <<N as DecRaw>::Out as Trim>::Out;
}

/// Type-level addition with ripple carry: `<A as Add<B>>::Sum` is A + B.
/// (A separate trait from `peano::Add` so the two encodings stay
/// independent, as they would be in separate experiments.)
pub trait Add<Rhs: Nat>: Nat {
    type Sum: Nat;
}

/// Helper: addition with an incoming carry bit already folded in.
/// `<A as AddCarry<B>>::Sum` is A + B + 1.
pub trait AddCarry<Rhs: Nat>: Nat {
    type Sum: Nat;
}

// --- Add, no carry in ---
impl Add<UTerm> for UTerm {
    type Sum = UTerm;
}

impl<U: Nat, B: Bit> Add<UInt<U, B>> for UTerm {
    type Sum = UInt<U, B>;
}

impl<U: Nat, B: Bit> Add<UTerm> for UInt<U, B> {
    type Sum = UInt<U, B>;
}

impl<Ul: Add<Ur>, Ur: Nat> Add<UInt<Ur, B0>> for UInt<Ul, B0> {
    // ...0 + ...0 = (U+V)0
    type Sum = UInt<Ul::Sum, B0>;
}

impl<Ul: Add<Ur>, Ur: Nat> Add<UInt<Ur, B1>> for UInt<Ul, B0> {
    // ...0 + ...1 = (U+V)1
    type Sum = UInt<Ul::Sum, B1>;
}

impl<Ul: Add<Ur>, Ur: Nat> Add<UInt<Ur, B0>> for UInt<Ul, B1> {
    // ...1 + ...0 = (U+V)1
    type Sum = UInt<Ul::Sum, B1>;
}

impl<Ul: AddCarry<Ur>, Ur: Nat> Add<UInt<Ur, B1>> for UInt<Ul, B1> {
    // ...1 + ...1 = carry: (U+V+1)0
    type Sum = UInt<Ul::Sum, B0>;
}

// --- AddCarry: A + B + 1 ---
impl AddCarry<UTerm> for UTerm {
    type Sum = UInt<UTerm, B1>;
}

impl<U: Inc> AddCarry<UInt<U, B0>> for UTerm {
    type Sum = UInt<U, B1>;
}

impl<U: Inc> AddCarry<UInt<U, B1>> for UTerm {
    type Sum = UInt<U::Out, B0>;
}

impl<U: Inc> AddCarry<UTerm> for UInt<U, B0> {
    type Sum = UInt<U, B1>;
}

impl<U: Inc> AddCarry<UTerm> for UInt<U, B1> {
    type Sum = UInt<U::Out, B0>;
}

impl<Ul: Add<Ur>, Ur: Nat> AddCarry<UInt<Ur, B0>> for UInt<Ul, B0> {
    // ...0 + ...0 + 1 = (U+V)1
    type Sum = UInt<Ul::Sum, B1>;
}

impl<Ul: AddCarry<Ur>, Ur: Nat> AddCarry<UInt<Ur, B1>> for UInt<Ul, B0> {
    // ...0 + ...1 + 1 = (U+V+1)0
    type Sum = UInt<Ul::Sum, B0>;
}

impl<Ul: AddCarry<Ur>, Ur: Nat> AddCarry<UInt<Ur, B0>> for UInt<Ul, B1> {
    // ...1 + ...0 + 1 = (U+V+1)0
    type Sum = UInt<Ul::Sum, B0>;
}

impl<Ul: AddCarry<Ur>, Ur: Nat> AddCarry<UInt<Ur, B1>> for UInt<Ul, B1> {
    // ...1 + ...1 + 1 = (U+V+1)1
    type Sum = UInt<Ul::Sum, B1>;
}
