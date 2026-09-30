//! Type-parameter encoding: a single `Bolt<Size, Material, Length>`
//! (R6, experiment step 1b).
//!
//! The whole 18-combination catalogue (24 with M12) is this one struct plus
//! one unit struct per characteristic *value*. No macro is needed because
//! nothing scales with the number of combinations.
//!
//! The `Size` / `Material` / `Length` kind traits are not strictly required,
//! but without them `Bolt<Steel, SizeM8, L15>` (parameters transposed)
//! compiles silently and only fails far away at first use. The kind bounds
//! turn transposition into an error at the construction site
//! (see tests/compile_fail/transposed_params.rs).

use core::marker::PhantomData;

// --- Kind traits: which struct may appear in which parameter slot ---------
pub trait Size {}
pub trait Material {}
pub trait Length {}

// --- Size values ----------------------------------------------------------
pub struct SizeM6;
pub struct SizeM8;
pub struct SizeM10;
/// Added in experiment step 4: the new size is these two lines
/// (plus one bridging impl in `bridge`).
pub struct SizeM12;

impl Size for SizeM6 {}
impl Size for SizeM8 {}
impl Size for SizeM10 {}
impl Size for SizeM12 {}

// --- Material values --------------------------------------------------------
pub struct Steel;
pub struct Brass;

impl Material for Steel {}
impl Material for Brass {}

// --- Length values ----------------------------------------------------------
pub struct L10;
pub struct L15;
pub struct L20;

impl Length for L10 {}
impl Length for L15 {}
impl Length for L20 {}

// --- The one bolt type ------------------------------------------------------
pub struct Bolt<S: Size, M: Material, L: Length>(PhantomData<(S, M, L)>);

impl<S: Size, M: Material, L: Length> Bolt<S, M, L> {
    /// Public here because this experiment is about encoding characteristics,
    /// not the privacy boundary (that is EXP-08).
    pub fn new() -> Self {
        Bolt(PhantomData)
    }
}

impl<S: Size, M: Material, L: Length> Default for Bolt<S, M, L> {
    fn default() -> Self {
        Self::new()
    }
}
