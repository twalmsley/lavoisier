//! The bolt catalogue (R6, R13) and its boundary supplier (R12).
//!
//! **Catalogues are parameterized types** (R6 fixed roles): the whole
//! M6/M8 × Steel/Brass × L10/L15 catalogue is the one sealed type
//! [`Bolt<Size, Material, Length>`](Bolt) plus one unit struct per
//! characteristic *value* — LOC scales with the sum of dimension values, not
//! the product, and a wrong bolt is the clearest error of any encoding
//! (`expected Bolt<SizeM8, Steel, L15>, found Bolt<SizeM6, Brass, L15>`).
//!
//! **Every type parameter is bounded by a kind trait** ([`Size`],
//! [`Material`], [`Length`]) so transposed parameters are a clear
//! construction-site error, not silent model corruption (F-018).
//!
//! **Bridging impls are part of the catalogue** (F-019): one line per
//! characteristic value gives every parameterized bolt the marker traits its
//! parameters imply, so catalogue bolts satisfy the marker-bound requirement
//! traits (R10) unchanged. The bridge is one-way by nature; requirements are
//! therefore always marker bounds, never concrete `Bolt<…>` types in process
//! signatures.
//!
//! The [`BoltBox`] supplier holds real [`Bolt`] values in a type-level list
//! (R12): the count is the list's length, supplying one bolt shortens the
//! type by one, and an empty box cannot supply (no `Supplier` impl for
//! [`EmptyBoltBox`]). Bolts come into existence only in [`boundary::full_box`]
//! (R1); the recursive fill machinery is sealed (F-026).

use crate::requirements::assert_req001;
use core::marker::PhantomData;
use model_core::boundary::Supplier;
use model_core::list::{Cons, Len, Nil};

// ---------------------------------------------------------------------------
// Kind traits (R6, F-018): which value may appear in which parameter slot.
// ---------------------------------------------------------------------------

/// Kind trait for the size slot of [`Bolt`]. One one-line impl per size
/// value; an unbounded slot would accept transposed arguments silently
/// (F-018).
pub trait Size {}

/// Kind trait for the material slot of [`Bolt`].
pub trait Material {}

/// Kind trait for the length slot of [`Bolt`].
pub trait Length {}

// ---------------------------------------------------------------------------
// Characteristic values: one unit struct + one kind impl per value.
// ---------------------------------------------------------------------------

/// Size value: metric thread M6.
pub struct SizeM6;
impl Size for SizeM6 {}

/// Size value: metric thread M8.
pub struct SizeM8;
impl Size for SizeM8 {}

/// Material value: steel.
pub struct Steel;
impl Material for Steel {}

/// Material value: brass.
pub struct Brass;
impl Material for Brass {}

/// Length value: 10 mm (R7 base length unit is the millimetre).
pub struct L10;
impl Length for L10 {}

/// Length value: 15 mm.
pub struct L15;
impl Length for L15 {}

// ---------------------------------------------------------------------------
// The one catalogue type.
// ---------------------------------------------------------------------------

/// A bolt from the catalogue: size, material and length are type parameters
/// (R6), each bounded by its kind trait (F-018).
///
/// Sealed consumable resource (R1): private field, no public constructor, no
/// `Clone`/`Copy`/`Default`. Bolts enter the model only inside a full
/// [`BoltBox`] (R12, [`boundary::full_box`]) and are conserved from there
/// into an `Assembly`. Deliberately **not** tripwired: bolts are *kept* by
/// the containers that hold them (a box, an assembly), and a tripwired item
/// inside an abandoned container would panic from the container's own drop,
/// masking the real leak site — the same trade recorded for model-core's
/// reference fixtures. Whole-value discard is still caught by `#[must_use]`.
#[must_use = "Bolt is a conserved resource: pass it on or hand it to a Consumer"]
pub struct Bolt<S: Size, M: Material, L: Length> {
    _seal: PhantomData<(S, M, L)>,
}

impl<S: Size, M: Material, L: Length> Bolt<S, M, L> {
    /// Mints one bolt. `pub(crate)`: only this crate's boundary (the sealed
    /// fill machinery in [`boundary`]) may call it — "no creating resources
    /// from nothing" is a compile-time check (R1).
    pub(crate) fn mint() -> Self {
        Bolt { _seal: PhantomData }
    }

    /// Test fixture: a bolt from nowhere, for downstream test code only
    /// (R1, F-004; the pilot's `test-support` feature, dev-dependencies
    /// only).
    #[cfg(feature = "test-support")]
    pub fn test_fixture() -> Self {
        Self::mint()
    }
}

// ---------------------------------------------------------------------------
// Bridging impls (R6, F-019): one line per characteristic value, added in the
// same commit as the value itself. Any new defaulted parameter on `Bolt`
// requires rewriting every one of these with an explicit parameter (F-017);
// `grep 'for Bolt<'` makes the audit mechanical.
// ---------------------------------------------------------------------------

impl<S: Size, M: Material, L: Length> crate::characteristics::IsBolt for Bolt<S, M, L> {}

impl<M: Material, L: Length> crate::characteristics::M6 for Bolt<SizeM6, M, L> {}
impl<M: Material, L: Length> crate::characteristics::M8 for Bolt<SizeM8, M, L> {}

impl<S: Size, L: Length> crate::characteristics::Steel for Bolt<S, Steel, L> {}
impl<S: Size, L: Length> crate::characteristics::Brass for Bolt<S, Brass, L> {}

impl<S: Size, M: Material> crate::characteristics::Length10mm for Bolt<S, M, L10> {}
impl<S: Size, M: Material> crate::characteristics::Length15mm for Bolt<S, M, L15> {}

/// The catalogue bolt approved for fastening: M8, steel, 15 mm.
///
/// Satisfies: REQ-001
pub type FasteningBolt = Bolt<SizeM8, Steel, L15>;

// The tag above is for grep; this assertion is for truth (R10, F-020): if the
// catalogue or bridge ever stops making FasteningBolt satisfy REQ-001, this
// line is a type-check-time compile error naming the missing characteristic.
model_core::satisfies!(assert_req001, FasteningBolt);

/// A type-level list of exactly four bolts of one catalogue type `B` — the
/// `Taken =` shape `fasten`'s `SupplyN<N4>` bound requires (F-014).
pub type FourOf<B> = Cons<B, Cons<B, Cons<B, Cons<B, Nil>>>>;

// ---------------------------------------------------------------------------
// The BoltBox supplier (R12).
// ---------------------------------------------------------------------------

/// A box of bolts: a supplier at the system boundary (R12). Its contents are
/// a type-level list of real [`Bolt`] values; the private field means the
/// contents cannot be tampered with from outside, and the count **is** the
/// list's length, so count and contents cannot disagree.
///
/// Placeholder: generic bolt supplier — refine to a named fastener supplier.
#[must_use = "BoltBox is a boundary resource: pass it on like any other resource"]
pub struct BoltBox<Items>(Items);

/// An exhausted box: a distinct resource type that must itself be accounted
/// for (R12). It cannot supply — there is no `Supplier` impl for it.
pub type EmptyBoltBox = BoltBox<Nil>;

/// `Supplier` is implemented ONLY for a non-empty box (R12); supplying from
/// an empty box is a compile error with the modeller-phrased
/// `on_unimplemented` message from model-core (F-015).
impl<H, T> Supplier for BoltBox<Cons<H, T>> {
    type Item = H;
    type Next = BoltBox<T>;
    fn supply(self) -> (H, BoltBox<T>) {
        let Cons(head, tail) = self.0;
        (head, BoltBox(tail))
    }
}

impl<Items: Len> BoltBox<Items> {
    /// How many bolts the box holds — the length of its contents list (R7,
    /// R12).
    pub const COUNT: u64 = Items::LEN;
}

/// The creation boundary of the bolt family (R12, F-006): the only production
/// code where bolts come into existence.
pub mod boundary {
    use super::{Bolt, BoltBox, Length, Material, Size};
    use model_core::list::{Cons, Nil};
    use model_core::nat::{Succ, Zero};

    /// Maps a type-level count to the list type of that many `B` bolts:
    /// `BoltsOf<FasteningBolt, N4>` is four `FasteningBolt`s.
    pub trait Replicate<B> {
        /// The list type holding `Self`-many bolts of type `B`.
        type List;
    }
    impl<B> Replicate<B> for Zero {
        type List = Nil;
    }
    impl<B, N: Replicate<B>> Replicate<B> for Succ<N> {
        type List = Cons<B, N::List>;
    }

    /// Shorthand for [`Replicate::List`].
    pub type BoltsOf<B, N> = <N as Replicate<B>>::List;

    /// Builds a list of real bolts. Private: together with `Bolt::mint`,
    /// this is the only place bolts come into existence (R1), reachable only
    /// through [`full_box`].
    trait Fill {
        fn fill() -> Self;
    }
    impl Fill for Nil {
        fn fill() -> Nil {
            Nil
        }
    }
    impl<S: Size, M: Material, L: Length, T: Fill> Fill for Cons<Bolt<S, M, L>, T> {
        fn fill() -> Self {
            Cons(Bolt::mint(), T::fill())
        }
    }

    /// Public-in-signature but unimplementable-outside wrapper over the
    /// private fill machinery, so [`full_box`]'s where-clause can name it
    /// without letting outside code implement it (the classic sealed-trait
    /// pattern, F-026).
    pub trait FillSealed: sealed::Sealed {
        #[doc(hidden)]
        fn fill_sealed() -> Self;
    }
    impl<L2: Fill + sealed::Sealed> FillSealed for L2 {
        fn fill_sealed() -> Self {
            L2::fill()
        }
    }
    mod sealed {
        use super::{Bolt, Length, Material, Size};
        use model_core::list::{Cons, Nil};
        pub trait Sealed {}
        impl Sealed for Nil {}
        impl<S: Size, M: Material, L: Length, T: Sealed> Sealed for Cons<Bolt<S, M, L>, T> {}
    }

    /// The fill function (R12): a full box of `N` real catalogue bolts of
    /// type `B` enters the model here — `full_box::<FasteningBolt, N4>()`.
    ///
    /// Placeholder: generic bolt supplier — refine to a named fastener
    /// supplier.
    pub fn full_box<B, N: Replicate<B>>() -> BoltBox<BoltsOf<B, N>>
    where
        BoltsOf<B, N>: FillSealed,
    {
        BoltBox(FillSealed::fill_sealed())
    }
}

#[cfg(test)]
mod tests {
    use super::boundary::{BoltsOf, full_box};
    use super::{Bolt, BoltBox, Brass, EmptyBoltBox, FasteningBolt, L10, SizeM6};
    use model_core::boundary::take_one;
    use model_core::nat::aliases::{N1, N2};

    /// The count is the list's recursive length (R7, R12): supplying one
    /// bolt shortens the box type by one.
    #[test]
    fn supplying_shortens_the_box_by_one() {
        let bx = full_box::<FasteningBolt, N2>();
        assert_eq!(BoltBox::<BoltsOf<FasteningBolt, N2>>::COUNT, 2);
        let (bolt, bx) = take_one(bx);
        assert_eq!(BoltBox::<BoltsOf<FasteningBolt, N1>>::COUNT, 1);
        let (bolt2, empty) = take_one(bx);
        let _empty: EmptyBoltBox = empty; // the exhausted box is a new resource
        // Inside the privacy boundary this crate's own tests may account for
        // bolts directly by keeping them to the end of the test.
        let _accounted = (bolt, bolt2);
    }

    /// The catalogue covers the whole M6/M8 x Steel/Brass x L10/L15 cross
    /// product without one struct per combination (R6): any combination
    /// fills a box.
    #[test]
    fn any_catalogue_combination_fills_a_box() {
        let bx = full_box::<Bolt<SizeM6, Brass, L10>, N1>();
        let (bolt, empty) = take_one(bx);
        let _empty: EmptyBoltBox = empty;
        let _accounted = bolt;
    }
}
