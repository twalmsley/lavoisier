//! Cost of adding a whole new dimension — thread pitch — to each encoding
//! (evaluation criterion). Kept in its own module so the main catalogue stays
//! three-dimensional; this is a scaled-down but compiling demonstration.
//!
//! Marker style: the struct set is the cross product of all dimensions, so a
//! new dimension with k values multiplies the struct count by k (24 structs
//! become 48 for coarse/fine pitch) and every struct gains one impl line.
//! Names must also change (`M8SteelBolt15mm` says nothing about pitch), so
//! every existing use site is touched. Demonstrated below with two structs.
//!
//! Parameter style: two options.
//!   (a) Add a fourth parameter `Bolt<S, M, L, P>` — every existing use site
//!       of `Bolt<_, _, _>` breaks and must be edited.
//!   (b) Add it with a *default*: `Bolt<S, M, L, P = Pitch125>` — existing
//!       use sites keep compiling unchanged (demonstrated by
//!       `three_param_spelling_still_compiles` below). One trap: existing
//!       bridging impls written as `impl<..> M8 for Bolt<SizeM8, M, L>` now
//!       silently cover only the default pitch; they must be rewritten with
//!       an explicit `P` parameter or bolts of other pitches lose their
//!       marker traits.

use core::marker::PhantomData;

// --- The new characteristic ------------------------------------------------
pub trait Pitch {}
/// Coarse pitch, 1.25 mm (value ×100 in the name; integers only, R7).
pub struct Pitch125;
/// Fine pitch, 1.00 mm.
pub struct Pitch100;
impl Pitch for Pitch125 {}
impl Pitch for Pitch100 {}

pub trait CoarsePitch {}
pub trait FinePitch {}

// --- Marker style: every combination struct doubles --------------------------
// (Two of the 48 are shown; the other 46 follow the same pattern.)
pub struct M8SteelBolt15mmCoarse;
impl crate::characteristics::IsBolt for M8SteelBolt15mmCoarse {}
impl crate::characteristics::M8 for M8SteelBolt15mmCoarse {}
impl crate::characteristics::Steel for M8SteelBolt15mmCoarse {}
impl crate::characteristics::Length15mm for M8SteelBolt15mmCoarse {}
impl CoarsePitch for M8SteelBolt15mmCoarse {}

pub struct M8SteelBolt15mmFine;
impl crate::characteristics::IsBolt for M8SteelBolt15mmFine {}
impl crate::characteristics::M8 for M8SteelBolt15mmFine {}
impl crate::characteristics::Steel for M8SteelBolt15mmFine {}
impl crate::characteristics::Length15mm for M8SteelBolt15mmFine {}
impl FinePitch for M8SteelBolt15mmFine {}

// --- Parameter style, option (b): default type parameter ----------------------
// A local copy of the parameterized bolt so `param::Bolt` itself is untouched.
pub struct PBolt<S: crate::param::Size, M: crate::param::Material, L: crate::param::Length, P: Pitch = Pitch125>(
    PhantomData<(S, M, L, P)>,
);

impl<S: crate::param::Size, M: crate::param::Material, L: crate::param::Length, P: Pitch>
    PBolt<S, M, L, P>
{
    pub fn new() -> Self {
        PBolt(PhantomData)
    }
}

impl<S: crate::param::Size, M: crate::param::Material, L: crate::param::Length, P: Pitch> Default
    for PBolt<S, M, L, P>
{
    fn default() -> Self {
        Self::new()
    }
}

// Bridge for the new dimension: note the explicit `P` — see the trap above.
impl<S: crate::param::Size, M: crate::param::Material, L: crate::param::Length> CoarsePitch
    for PBolt<S, M, L, Pitch125>
{
}
impl<S: crate::param::Size, M: crate::param::Material, L: crate::param::Length> FinePitch
    for PBolt<S, M, L, Pitch100>
{
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::param::{L15, SizeM8, Steel};

    /// The default type parameter keeps three-parameter spellings compiling
    /// after the fourth dimension is added.
    #[test]
    fn three_param_spelling_still_compiles() {
        let _bolt: PBolt<SizeM8, Steel, L15> = PBolt::new();
        let _same: PBolt<SizeM8, Steel, L15, Pitch125> = PBolt::new();
        fn coarse<B: CoarsePitch>(b: B) -> B {
            b
        }
        let _b = coarse(PBolt::<SizeM8, Steel, L15>::new());
        let _b = coarse(M8SteelBolt15mmCoarse);
    }
}
