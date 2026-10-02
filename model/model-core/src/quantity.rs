//! Quantities and units (R3, R7, R8).
//!
//! Every unit is its own type and every quantity is `Qty<V, U>`: a sealed,
//! `#[must_use]` type carrying its magnitude as a const generic and exposing
//! it through [`Quantity::VALUE`] (R7). This is the **primary quantity
//! style** (R3): one generic [`split`]/[`combine`] serves every unit, unit
//! mixing is a crisp type-check-time E0308, and the conservation assert is
//! stated over the const parameters. The project writes its own units library
//! (R8); external crates such as `uom` are forbidden.
//!
//! Values are **integers in the smallest unit** of each dimension (R7); no
//! floating point. The eight base units are defined below; derived units
//! follow from them without conversion factors (area is mm², a litre is
//! 1,000,000 mm³), and temperature is millikelvin so values are never
//! negative.
//!
//! ## Where the conservation check fires (R4 caveat, F-001)
//!
//! The `const { assert!(…) }` conservation checks in [`split`] and
//! [`combine`] fire at **monomorphization, during codegen**: `cargo check`,
//! rust-analyzer diagnostics and trybuild all report violating code as fine.
//! CI therefore gates on `cargo build`/`cargo test` (never `cargo check`
//! alone), and the regression tests for violations are rustdoc
//! `compile_fail` doc-tests on the functions below (R4, F-003). A violation
//! is an E0080 whose custom message leads the output; the caller's line is
//! in the "while instantiating `fn split::<…>`" note.
//!
//! ## Outputs cannot be computed on stable (F-022)
//!
//! `Qty<{A + B}, U>` needs `generic_const_exprs` (nightly), so the caller of
//! [`combine`] states the expected total and the assert checks it. Const
//! parameters otherwise infer well: from binding annotations, through
//! chains, and via partial turbofish (`split::<_, 1500, 500>(stock)`).
//!
//! ## Role note (R15)
//!
//! `Qty` earns its keep in **unit-polymorphic** quantity code. For a model's
//! waste and material outputs, prefer the bare const-magnitude resource form
//! (`ExhaustGas<const M: u64>` via [`crate::container_resource!`]): the extra
//! `Qty` layer adds no checking there, and the bare form prints decimal
//! magnitudes in every diagnostic (F-027).
//!
//! Adapted from `experiments/exp04-quantity-conservation/src/lib.rs`
//! (trait style, adopted per R3).

use core::marker::PhantomData;

/// Kind trait for unit types (R6: every type parameter is bounded by a kind
/// trait, so a non-unit type in the unit slot is a clear construction-site
/// error).
///
/// The eight base units below are the agreed R7 table. A genuinely new
/// dimension gets a new unit type here (added to the R7 table, marked
/// "default", per R7); it is one `impl Unit for NewUnit {}` line.
pub trait Unit {}

/// Mass in grams (R7 base unit, agreed).
pub struct Grams;
impl Unit for Grams {}

/// Length in millimetres (R7 base unit, agreed).
pub struct Millimetres;
impl Unit for Millimetres {}

/// Time in milliseconds (R7 base unit, agreed). Note R9/R15: time enters the
/// model only as a costed amount (a budget drawn down), never as wall-clock
/// sequencing.
pub struct Milliseconds;
impl Unit for Milliseconds {}

/// Area in square millimetres (R7 base unit, default). Area is length ×
/// length, so mm² keeps calculations consistent with `Millimetres`.
pub struct SquareMillimetres;
impl Unit for SquareMillimetres {}

/// Volume in cubic millimetres (R7 base unit, default). A litre is
/// 1,000,000 mm³.
pub struct CubicMillimetres;
impl Unit for CubicMillimetres {}

/// Temperature in millikelvin (R7 base unit, default). Kelvin-based so
/// values are never negative and fit unsigned integers.
pub struct Millikelvin;
impl Unit for Millikelvin {}

/// Electric current in milliamperes (R7 base unit, default).
pub struct Milliamperes;
impl Unit for Milliamperes {}

/// Energy in joules (R7 base unit, default). Amounts of energy are conserved
/// and balanced per process (R15); power (a rate) stays outside the model
/// (R9).
pub struct Joules;
impl Unit for Joules {}

/// Every quantity type carries its unit as an associated type and its
/// numeric value as an associated constant (R7), so values are available for
/// calculations and tests as well as for type checking.
pub trait Quantity {
    /// The unit of this quantity.
    type Unit: Unit;
    /// The magnitude, in the base unit.
    const VALUE: u64;
}

/// A quantity of `V` in unit `U` (R7).
///
/// Sealed (R1): private fields, no public constructor, no
/// `Clone`/`Copy`/`Default`. Quantities are brought into existence only by
/// the boundary supplier in [`boundary`] (R12) or conserving processes here;
/// downstream crates cannot mint one from nothing.
#[must_use = "a quantity is a conserved resource (R1): pass it on, split or combine it, or hand it to a Consumer"]
pub struct Qty<const V: u64, U: Unit>(PhantomData<U>, ());

impl<const V: u64, U: Unit> Quantity for Qty<V, U> {
    type Unit = U;
    const VALUE: u64 = V;
}

/// Splits a quantity into two parts (R3: changing an amount is a process).
///
/// Conservation (`A + B == IN`) is checked at compile time; the caller
/// usually states the parts through binding annotations and everything else
/// infers:
///
/// ```
/// use model_core::quantity::{Grams, Qty, boundary::supply, split};
///
/// let stock = supply::<2000, Grams>();
/// let (part, offcut): (Qty<1500, Grams>, Qty<500, Grams>) = split(stock);
/// ```
///
/// A violation is a genuine compile error (E0080 at monomorphization — not
/// visible to `cargo check` or editors, F-001). Regression: splitting 2000 g
/// into 1500 g + 600 g must not compile:
///
/// ```compile_fail
/// use model_core::quantity::{Grams, Qty, boundary::supply, split};
///
/// let stock = supply::<2000, Grams>();
/// let (part, offcut): (Qty<1500, Grams>, Qty<600, Grams>) = split(stock);
/// ```
///
/// Unit safety is structural: both outputs necessarily have the input's
/// unit, and mixing units elsewhere is an ordinary type error (E0308),
/// pinned by the trybuild case `tests/ui/unit_mismatch.rs`.
pub fn split<U: Unit, const IN: u64, const A: u64, const B: u64>(
    m: Qty<IN, U>,
) -> (Qty<A, U>, Qty<B, U>) {
    const {
        assert!(
            A + B == IN,
            "conservation violated in split (R3): the two output quantities must sum exactly to the input quantity"
        )
    };
    // Consume the input by destructuring; nothing is dropped silently.
    let Qty(_, seal) = m;
    (Qty(PhantomData, seal), Qty(PhantomData, ()))
}

/// Combines two quantities of the same unit into one (R3).
///
/// The total **cannot be computed on stable** (`generic_const_exprs`,
/// F-022), so the caller states it — usually through a binding annotation —
/// and the conservation assert checks it:
///
/// ```
/// use model_core::quantity::{Joules, Qty, boundary::supply, combine};
///
/// let a = supply::<1500, Joules>();
/// let b = supply::<500, Joules>();
/// let total: Qty<2000, Joules> = combine(a, b);
/// ```
///
/// A wrong total is the same E0080 as a bad split (F-001 applies).
/// Regression: combining 1500 + 500 into 2100 must not compile:
///
/// ```compile_fail
/// use model_core::quantity::{Grams, Qty, boundary::supply, combine};
///
/// let a = supply::<1500, Grams>();
/// let b = supply::<500, Grams>();
/// let total: Qty<2100, Grams> = combine(a, b);
/// ```
pub fn combine<U: Unit, const A: u64, const B: u64, const OUT: u64>(
    a: Qty<A, U>,
    b: Qty<B, U>,
) -> Qty<OUT, U> {
    const {
        assert!(
            A + B == OUT,
            "conservation violated in combine (R3): the stated total must sum the two input quantities exactly"
        )
    };
    let (Qty(_, seal), Qty(_, ())) = (a, b);
    Qty(PhantomData, seal)
}

/// The quantity boundary (R12): the only production code that brings a bare
/// `Qty` into existence.
pub mod boundary {
    use super::{PhantomData, Qty, Unit};

    /// Brings a quantity into the model at the system boundary (R12).
    ///
    /// Placeholder: generic quantity source — refine to a named supplier
    /// (organisation, stockline) in the modelling crate. Call this only in
    /// boundary/supplier code; everywhere else quantities must come from
    /// conserving processes. The call is greppable
    /// (`quantity::boundary::supply`) so boundary reviews are mechanical.
    pub fn supply<const V: u64, U: Unit>() -> Qty<V, U> {
        Qty(PhantomData, ())
    }
}

#[cfg(test)]
mod tests {
    use super::boundary::supply;
    use super::{Grams, Millimetres, Qty, Quantity, combine, split};

    /// A correct split conserves and the values are recoverable (R3, R7).
    /// The const parameters A and B are inferred from the binding annotation;
    /// IN is inferred from the argument.
    #[test]
    fn split_infers_from_annotation_and_conserves() {
        let stock = supply::<2000, Grams>();
        let (part, offcut): (Qty<1500, Grams>, Qty<500, Grams>) = split(stock);
        assert_eq!(<Qty<1500, Grams> as Quantity>::VALUE, 1500);
        assert_eq!(<Qty<500, Grams> as Quantity>::VALUE, 500);
        // Account for everything: recombine and check the total (R1).
        let total: Qty<2000, Grams> = combine(part, offcut);
        assert_eq!(<Qty<2000, Grams> as Quantity>::VALUE, 2000);
        let _accounted = total;
    }

    /// Const parameters also infer through chains and via partial turbofish
    /// (F-022: ergonomics are good even though the total must be stated).
    #[test]
    fn split_partial_turbofish_and_chain() {
        let stock = supply::<2000, Grams>();
        let (part, offcut) = split::<_, _, 1200, 800>(stock);
        let back: Qty<2000, Grams> = combine(part, offcut);
        let _accounted = back;
    }

    /// The same generic code works for any unit (R7): here lengths.
    #[test]
    fn split_is_unit_generic() {
        let rod = supply::<300, Millimetres>();
        let (a, b): (Qty<120, Millimetres>, Qty<180, Millimetres>) = split(rod);
        let back: Qty<300, Millimetres> = combine(a, b);
        let _accounted = back;
    }
}
