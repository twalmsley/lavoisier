//! Bridging blanket impls (experiment step 3): every parameterized
//! `Bolt<S, M, L>` also carries the marker traits its parameters imply, so a
//! parameterized bolt satisfies a marker-style requirement bound.
//!
//! Cost: one impl per characteristic *value* (not per combination) —
//! 4 sizes + 2 materials + 3 lengths + 1 `IsBolt` = 10 one-line impls.
//!
//! Direction: this bridge is one-way. A marker-style struct such as
//! `M8SteelBolt15mm` can never satisfy a signature demanding the concrete
//! type `Bolt<SizeM8, Steel, L15>`, because that is a nominal type, not a
//! bound. Marker traits are the common language; the parameterized type can
//! speak it, the marker structs cannot speak "parameterized".

use crate::characteristics;
use crate::param::{
    Bolt, Brass, L10, L15, L20, Length, Material, Size, SizeM10, SizeM12, SizeM6, SizeM8, Steel,
};

// Every parameterized bolt is a bolt.
impl<S: Size, M: Material, L: Length> characteristics::IsBolt for Bolt<S, M, L> {}

// Sizes.
impl<M: Material, L: Length> characteristics::M6 for Bolt<SizeM6, M, L> {}
impl<M: Material, L: Length> characteristics::M8 for Bolt<SizeM8, M, L> {}
impl<M: Material, L: Length> characteristics::M10 for Bolt<SizeM10, M, L> {}
/// Added in experiment step 4 together with `SizeM12`.
impl<M: Material, L: Length> characteristics::M12 for Bolt<SizeM12, M, L> {}

// Materials.
impl<S: Size, L: Length> characteristics::Steel for Bolt<S, Steel, L> {}
impl<S: Size, L: Length> characteristics::Brass for Bolt<S, Brass, L> {}

// Lengths.
impl<S: Size, M: Material> characteristics::Length10mm for Bolt<S, M, L10> {}
impl<S: Size, M: Material> characteristics::Length15mm for Bolt<S, M, L15> {}
impl<S: Size, M: Material> characteristics::Length20mm for Bolt<S, M, L20> {}
