//! Marker traits for bolt characteristics (R6).
//!
//! One trait per characteristic *value*. A concrete type "is" the
//! intersection of the traits it implements.

/// The thing itself is a bolt.
pub trait IsBolt {}

// --- Size ---------------------------------------------------------------
pub trait M6 {}
pub trait M8 {}
pub trait M10 {}
/// Added in step 4 of the experiment (the new size).
pub trait M12 {}

// --- Material -----------------------------------------------------------
pub trait Steel {}
pub trait Brass {}

// --- Length -------------------------------------------------------------
pub trait Length10mm {}
pub trait Length15mm {}
pub trait Length20mm {}
