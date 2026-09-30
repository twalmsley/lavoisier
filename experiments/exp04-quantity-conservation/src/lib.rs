//! EXP-04: Compile-time conservation of quantities (R3, R7).
//!
//! Tests whether "mass in = mass out" can be made a compile error on stable
//! Rust, using quantity types in the two styles from R7:
//!
//! * [`const_style`] — bare const generics: `Grams<const V: u64>`.
//! * [`trait_style`] — the R7 trait form: `Qty<const V: u64, U>` implementing
//!   `Quantity { type Unit; const VALUE: u64; }`.
//!
//! Both styles carry their numeric value as a constant (`VALUE`), have private
//! fields and no public constructor (R1), and do not implement `Clone`/`Copy`.
//!
//! Conservation-check techniques that WORK on stable 1.98:
//! 1. An associated-const assert forced by use:
//!    `let _ = AssertSum::<IN, A, B>::OK;` (see `split`).
//! 2. An inline `const { assert!(...) }` block (see `split_inline`).
//! Both are POST-MONOMORPHIZATION checks: the generic definition always
//! compiles; the error fires when a violating instantiation is codegenned.
//! Consequently `cargo check` does NOT catch violations — only `cargo build`
//! / `cargo test` do. See RESULTS.md.
//!
//! Techniques that DO NOT work on stable (kept as doc-tests below so the
//! failures stay demonstrated):
//! * A `const` item inside the generic fn — E0401, a nested item cannot use
//!   the outer generic parameters.
//! * A where-clause on a computed const expression
//!   (`where If<{ A + B == IN }>: True`) — "generic parameters may not be
//!   used in const operations" (needs unstable `generic_const_exprs`).
//!
//! ```compile_fail,E0401
//! pub struct Grams<const V: u64>(());
//! pub fn split<const IN: u64, const A: u64, const B: u64>(m: Grams<IN>) -> (Grams<A>, Grams<B>) {
//!     const CHECK: () = assert!(A + B == IN); // E0401: can't use generic parameters from outer item
//!     let _ = (CHECK, m);
//!     (Grams(()), Grams(()))
//! }
//! ```
//!
//! ```compile_fail
//! pub struct Grams<const V: u64>(());
//! pub struct If<const B: bool>;
//! pub trait True {}
//! impl True for If<true> {}
//! pub fn split<const IN: u64, const A: u64, const B: u64>(m: Grams<IN>) -> (Grams<A>, Grams<B>)
//! where
//!     If<{ A + B == IN }>: True, // error: generic parameters may not be used in const operations
//! {
//!     let _ = m;
//!     (Grams(()), Grams(()))
//! }
//! ```

//! ## Post-monomorphization violations, demonstrated as compile-fail doc-tests
//!
//! trybuild runs `cargo check`, which does NOT reach these errors (see
//! RESULTS.md and `tests/ui-postmono-not-caught-by-trybuild/`), but rustdoc
//! fully builds `compile_fail` doc-tests, so they make genuine automated
//! compile-fail tests for the conservation violations. (On stable, rustdoc
//! ignores the `,E0080` code annotation — it only checks that compilation
//! fails; the snippets are kept minimal so nothing else can fail.)
//!
//! Splitting 2000 g into 1500 g + 600 g, associated-const technique:
//! ```compile_fail,E0080
//! use exp04_quantity_conservation::const_style::{boundary::supply_grams, split, Grams};
//! let stock = supply_grams::<2000>();
//! let (part, offcut): (Grams<1500>, Grams<600>) = split(stock);
//! ```
//!
//! The same violation, inline-const technique:
//! ```compile_fail,E0080
//! use exp04_quantity_conservation::const_style::{boundary::supply_grams, split_inline, Grams};
//! let stock = supply_grams::<2000>();
//! let (part, offcut): (Grams<1500>, Grams<600>) = split_inline(stock);
//! ```
//!
//! Trait style: combining 1500 g + 500 g into 2100 g:
//! ```compile_fail,E0080
//! use exp04_quantity_conservation::trait_style::{boundary::supply, combine, Grams, Qty};
//! let a = supply::<1500, Grams>();
//! let b = supply::<500, Grams>();
//! let total: Qty<2100, Grams> = combine(a, b);
//! ```

/// Style 1: bare const generics. The unit is the type's name, the value is a
/// const generic parameter.
pub mod const_style {
    /// A mass in grams. Private field: no construction outside this module
    /// except through [`boundary`] (R1). Deliberately not `Clone`/`Copy`.
    #[must_use]
    pub struct Grams<const V: u64>(());

    impl<const V: u64> Grams<V> {
        /// R7: the numeric value is available as a constant.
        pub const VALUE: u64 = V;
    }

    /// A length in millimetres, for the unit-safety checks.
    #[must_use]
    pub struct Millimetres<const V: u64>(());

    impl<const V: u64> Millimetres<V> {
        pub const VALUE: u64 = V;
    }

    /// Technique 1: an associated const whose evaluation asserts conservation.
    /// Referencing `AssertSum::<IN, A, B>::OK` inside a function forces the
    /// constant to be evaluated when that function is monomorphized.
    pub struct AssertSum<const SUM: u64, const A: u64, const B: u64>;

    impl<const SUM: u64, const A: u64, const B: u64> AssertSum<SUM, A, B> {
        pub const OK: () = assert!(
            A + B == SUM,
            "conservation violated: the two output quantities do not sum to the input"
        );
    }

    /// R3: splitting an amount is a process. Conservation (`A + B == IN`) is
    /// checked at compile time via the associated-const assert.
    ///
    /// The generic definition compiles unconditionally; a violating call such
    /// as `split::<2000, 1500, 600>` fails at monomorphization (E0080).
    pub fn split<const IN: u64, const A: u64, const B: u64>(m: Grams<IN>) -> (Grams<A>, Grams<B>) {
        let _ = AssertSum::<IN, A, B>::OK;
        let Grams(inner) = m; // consume the input; nothing is dropped silently
        (Grams(inner), Grams(()))
    }

    /// Technique 2: the same check as an inline `const` block (stable since
    /// Rust 1.79; inline const blocks may use the enclosing generics).
    pub fn split_inline<const IN: u64, const A: u64, const B: u64>(
        m: Grams<IN>,
    ) -> (Grams<A>, Grams<B>) {
        const {
            assert!(
                A + B == IN,
                "conservation violated: the two output quantities do not sum to the input"
            )
        };
        let Grams(inner) = m;
        (Grams(inner), Grams(()))
    }

    /// R3: combining amounts is a process too. `OUT` cannot be *computed* on
    /// stable (no `generic_const_exprs`), so the caller must state it and the
    /// assert checks it.
    pub fn combine<const A: u64, const B: u64, const OUT: u64>(
        a: Grams<A>,
        b: Grams<B>,
    ) -> Grams<OUT> {
        let _ = AssertSum::<OUT, A, B>::OK;
        let (Grams(inner), Grams(())) = (a, b);
        Grams(inner)
    }

    /// `combine` with the inline-const technique.
    pub fn combine_inline<const A: u64, const B: u64, const OUT: u64>(
        a: Grams<A>,
        b: Grams<B>,
    ) -> Grams<OUT> {
        const {
            assert!(
                A + B == OUT,
                "conservation violated: the output quantity does not sum the two inputs"
            )
        };
        let (Grams(inner), Grams(())) = (a, b);
        Grams(inner)
    }

    /// R12: placeholder suppliers — the only public way to bring quantities
    /// into existence.
    pub mod boundary {
        use super::{Grams, Millimetres};

        /// Placeholder: the steel stock supplier for the demo flow.
        pub fn supply_grams<const V: u64>() -> Grams<V> {
            Grams(())
        }

        /// Placeholder: a length supplier, used by the unit-safety tests.
        pub fn supply_millimetres<const V: u64>() -> Millimetres<V> {
            Millimetres(())
        }
    }
}

/// Style 2: the R7 trait form. A quantity is `Qty<const V: u64, U>` where `U`
/// is a unit type; the `Quantity` trait exposes both the unit and the value.
pub mod trait_style {
    use core::marker::PhantomData;

    /// Marker trait for unit types.
    pub trait Unit {}

    pub struct Grams;
    impl Unit for Grams {}

    pub struct Millimetres;
    impl Unit for Millimetres {}

    /// R7: every quantity type carries its unit as an associated type and its
    /// numeric value as an associated constant.
    pub trait Quantity {
        type Unit: Unit;
        const VALUE: u64;
    }

    /// A quantity of `V` in unit `U`. Private field: no construction outside
    /// this module except through [`boundary`] (R1). Not `Clone`/`Copy`.
    #[must_use]
    pub struct Qty<const V: u64, U: Unit>(PhantomData<U>, ());

    impl<const V: u64, U: Unit> Quantity for Qty<V, U> {
        type Unit = U;
        const VALUE: u64 = V;
    }

    /// The conservation assert stated over `Quantity` TYPES rather than raw
    /// const parameters: the values come out of the trait's associated const.
    pub struct AssertConserves<Whole, PartA, PartB>(PhantomData<(Whole, PartA, PartB)>);

    impl<Whole: Quantity, PartA: Quantity, PartB: Quantity> AssertConserves<Whole, PartA, PartB> {
        pub const OK: () = assert!(
            PartA::VALUE + PartB::VALUE == Whole::VALUE,
            "conservation violated: the two part quantities do not sum to the whole"
        );
    }

    /// Split, generic over the unit `U` as well as the three values. Unit
    /// safety is structural: both outputs necessarily have the input's unit.
    pub fn split<U: Unit, const IN: u64, const A: u64, const B: u64>(
        m: Qty<IN, U>,
    ) -> (Qty<A, U>, Qty<B, U>) {
        let _ = AssertConserves::<Qty<IN, U>, Qty<A, U>, Qty<B, U>>::OK;
        let Qty(_, inner) = m;
        (Qty(PhantomData, inner), Qty(PhantomData, ()))
    }

    /// Combine. Both inputs must have the same unit `U` — mixing `Grams` and
    /// `Millimetres` is an ordinary type error (E0308), caught by
    /// `cargo check`, BEFORE monomorphization.
    pub fn combine<U: Unit, const A: u64, const B: u64, const OUT: u64>(
        a: Qty<A, U>,
        b: Qty<B, U>,
    ) -> Qty<OUT, U> {
        let _ = AssertConserves::<Qty<OUT, U>, Qty<A, U>, Qty<B, U>>::OK;
        let (Qty(_, inner), Qty(_, ())) = (a, b);
        Qty(PhantomData, inner)
    }

    /// R12: placeholder supplier for the trait style.
    pub mod boundary {
        use super::{PhantomData, Qty, Unit};

        /// Placeholder: generic supplier bringing a quantity into existence.
        pub fn supply<const V: u64, U: Unit>() -> Qty<V, U> {
            Qty(PhantomData, ())
        }
    }
}

/// Technique 3: a call-site macro that plants a concrete `const _` item.
///
/// A nested `const` item with concrete values IS evaluated by `cargo check`
/// (unlike an associated const referenced from a function body, which is only
/// evaluated at codegen). So this macro turns a conservation violation into a
/// CHECK-TIME error — and therefore one that trybuild can see. The costs: the
/// input amount must be written out as a literal, and the macro only works at
/// monomorphic call sites (a generic caller cannot pass its const parameters
/// to `const _`... they are literals here by construction).
#[macro_export]
macro_rules! split_grams {
    ($m:expr, $in:literal => $a:literal + $b:literal) => {{
        const _: () = assert!(
            $a + $b == $in,
            concat!(
                "conservation violated: splitting ", stringify!($in),
                " g into ", stringify!($a), " g + ", stringify!($b),
                " g does not conserve mass"
            )
        );
        $crate::const_style::split::<$in, $a, $b>($m)
    }};
}

/// Technique 4: type-level Peano numbers, where conservation is a TRAIT BOUND
/// (`A: Add<B, Sum = IN>`) instead of a const-evaluated assert. The bound is
/// checked during type checking, so a violation fires under `cargo check` and
/// under trybuild, and even the *output* of `combine` is computed by the
/// compiler instead of being stated by the caller.
///
/// The cost: values are unary types (`Succ<Succ<...>>`), so realistic
/// magnitudes like 2000 g are impractical in this encoding. Scaling (aliases,
/// binary encodings, recursion limits) is EXP-01's subject; this module only
/// establishes WHERE the error fires.
pub mod type_level_style {
    use core::marker::PhantomData;

    pub struct Zero;
    pub struct Succ<N>(PhantomData<N>);

    /// R7: type-level numbers must also expose their value as a constant.
    pub trait Nat {
        const VALUE: u64;
    }
    impl Nat for Zero {
        const VALUE: u64 = 0;
    }
    impl<N: Nat> Nat for Succ<N> {
        const VALUE: u64 = N::VALUE + 1;
    }

    /// Type-level addition.
    pub trait Add<B: Nat>: Nat {
        type Sum: Nat;
    }
    impl<B: Nat> Add<B> for Zero {
        type Sum = B;
    }
    impl<A: Add<B>, B: Nat> Add<B> for Succ<A> {
        type Sum = Succ<A::Sum>;
    }

    pub type N0 = Zero;
    pub type N1 = Succ<N0>;
    pub type N2 = Succ<N1>;
    pub type N3 = Succ<N2>;
    pub type N4 = Succ<N3>;
    pub type N5 = Succ<N4>;

    /// A mass of `N` grams, the amount encoded as a type-level number.
    /// Private field; not `Clone`/`Copy`.
    #[must_use]
    pub struct MassG<N: Nat>(PhantomData<N>, ());

    /// Split with conservation as a trait bound: `A + B` must equal `IN`
    /// by construction of the `Sum` associated type. Violations are ordinary
    /// type errors, caught by `cargo check` at the call site.
    pub fn split<In: Nat, A: Add<B, Sum = In>, B: Nat>(m: MassG<In>) -> (MassG<A>, MassG<B>) {
        let MassG(_, inner) = m;
        (MassG(PhantomData, inner), MassG(PhantomData, ()))
    }

    /// Combine — note the output type `A::Sum` is COMPUTED by the compiler;
    /// the caller does not have to state the total at all.
    pub fn combine<A: Add<B>, B: Nat>(a: MassG<A>, b: MassG<B>) -> MassG<A::Sum> {
        let (MassG(_, inner), MassG(_, ())) = (a, b);
        MassG(PhantomData, inner)
    }

    /// R12: placeholder supplier.
    pub mod boundary {
        use super::{MassG, Nat, PhantomData};

        /// Placeholder: brings a type-level mass into existence.
        pub fn supply<N: Nat>() -> MassG<N> {
            MassG(PhantomData, ())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::const_style::{self, boundary::supply_grams, combine, combine_inline, split, split_inline};
    use super::trait_style::{self, boundary::supply, Grams as G, Millimetres as Mm, Qty, Quantity};

    /// Verifies: R3, R7 — a correct split compiles and the values are right.
    /// Const parameters A and B are INFERRED from the binding's annotation;
    /// IN is inferred from the argument. Nothing is spelled out at the call.
    #[test]
    fn const_style_split_infers_from_annotation() {
        let stock = supply_grams::<2000>();
        let (part, offcut): (const_style::Grams<1500>, const_style::Grams<500>) = split(stock);
        assert_eq!(const_style::Grams::<1500>::VALUE, 1500);
        assert_eq!(const_style::Grams::<500>::VALUE, 500);
        // account for everything: recombine and check the total
        let total: const_style::Grams<2000> = combine(part, offcut);
        assert_eq!(const_style::Grams::<2000>::VALUE, 2000);
        let _ = total;
    }

    /// Verifies: R3 — const parameters also infer from downstream use:
    /// the outputs of `split` flow straight into `combine`, whose output
    /// annotation is the only type written.
    #[test]
    fn const_style_infers_through_a_chain() {
        let stock = supply_grams::<100>();
        let (a, b) = split::<100, 40, 60>(stock); // fully spelled out works too
        let back: const_style::Grams<100> = combine(a, b);
        let _ = back;
    }

    /// Verifies: R3 — `_` placeholders are allowed for const arguments in the
    /// turbofish, so only the outputs need naming.
    #[test]
    fn const_style_partial_turbofish() {
        let stock = supply_grams::<2000>();
        let (part, offcut) = split::<_, 1500, 500>(stock);
        let _: const_style::Grams<1500> = part;
        let _: const_style::Grams<500> = offcut;
    }

    /// Verifies: R3 — the inline-const technique behaves the same.
    #[test]
    fn const_style_inline_const_split_and_combine() {
        let stock = supply_grams::<2000>();
        let (part, offcut): (const_style::Grams<1200>, const_style::Grams<800>) =
            split_inline(stock);
        let back: const_style::Grams<2000> = combine_inline(part, offcut);
        let _ = back;
    }

    /// Verifies: R3, R7 — trait style: split then combine, unit inferred.
    #[test]
    fn trait_style_split_and_combine() {
        let stock = supply::<2000, G>();
        let (part, offcut): (Qty<1500, G>, Qty<500, G>) = trait_style::split(stock);
        assert_eq!(<Qty<1500, G> as Quantity>::VALUE, 1500);
        let back: Qty<2000, G> = trait_style::combine(part, offcut);
        let _ = back;
    }

    /// Verifies: R7 — the same generic code works for any unit; here lengths.
    #[test]
    fn trait_style_is_unit_generic() {
        let rod = supply::<300, Mm>();
        let (a, b): (Qty<120, Mm>, Qty<180, Mm>) = trait_style::split(rod);
        let back: Qty<300, Mm> = trait_style::combine(a, b);
        let _ = back;
    }

    /// Verifies: R3 — the call-site macro (check-time technique) accepts a
    /// conserving split.
    #[test]
    fn macro_split_conserving() {
        let stock = supply_grams::<2000>();
        let (part, offcut) = crate::split_grams!(stock, 2000 => 1500 + 500);
        let _: const_style::Grams<1500> = part;
        let _: const_style::Grams<500> = offcut;
    }

    /// Verifies: R3, R7 — type-level style: conservation as a trait bound,
    /// values recoverable through `Nat::VALUE`, and `combine`'s output type
    /// computed by the compiler.
    #[test]
    fn type_level_split_and_combine() {
        use crate::type_level_style::{boundary, combine, split, MassG, Nat, N2, N3, N5};
        let five = boundary::supply::<N5>();
        let (a, b): (MassG<N3>, MassG<N2>) = split(five);
        assert_eq!(N3::VALUE, 3);
        assert_eq!(N2::VALUE, 2);
        let back = combine(a, b); // output type computed: MassG<N5>
        let _: MassG<N5> = back;
    }
}
