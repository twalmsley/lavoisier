//! Marker characteristic traits (R6).
//!
//! A type's characteristics are the traits it implements. These markers are
//! the common language both characteristic encodings speak (F-019): the
//! parameterized catalogue types in [`crate::catalogue`] acquire them through
//! one-line bridging impls, and the R10 requirement traits in
//! [`crate::requirements`] are written as bounds over them — never over
//! concrete `Bolt<…>` types.
//!
//! Each marker carries a single-line `#[diagnostic::on_unimplemented]`
//! message phrased for modellers (F-015). Single-line on purpose: the
//! workspace `trace.sh` has no scope awareness, and a multi-line attribute
//! between a doc tag and its item would detach them (F-021).

/// Characteristic: the item is a bolt (any size, material or length).
#[diagnostic::on_unimplemented(message = "`{Self}` is not a bolt", label = "a bolt is required here")]
pub trait IsBolt {}

/// Characteristic: metric thread size M6.
#[diagnostic::on_unimplemented(message = "`{Self}` is not an M6-sized bolt", label = "an M6 thread is required here")]
pub trait M6 {}

/// Characteristic: metric thread size M8.
#[diagnostic::on_unimplemented(message = "`{Self}` is not an M8-sized bolt (REQ-001 requires M8 for fastening)", label = "an M8 thread is required here")]
pub trait M8 {}

/// Characteristic: made of steel.
#[diagnostic::on_unimplemented(message = "`{Self}` is not made of steel (REQ-001 requires steel for fastening)", label = "steel is required here")]
pub trait Steel {}

/// Characteristic: made of brass.
#[diagnostic::on_unimplemented(message = "`{Self}` is not made of brass", label = "brass is required here")]
pub trait Brass {}

/// Characteristic: 10 mm long.
#[diagnostic::on_unimplemented(message = "`{Self}` is not 10 mm long", label = "a 10 mm length is required here")]
pub trait Length10mm {}

/// Characteristic: 15 mm long.
#[diagnostic::on_unimplemented(message = "`{Self}` is not 15 mm long (REQ-001 requires 15 mm for fastening)", label = "a 15 mm length is required here")]
pub trait Length15mm {}

/// Characteristic: this plate has had its bolt holes drilled. Implemented
/// only by the drilled processing state (one type per processing state, R9),
/// which is what lets REQ-002 be stated as a trait bound.
#[diagnostic::on_unimplemented(message = "`{Self}` has not been drilled (REQ-002: plates must be drilled before fastening)", label = "a drilled plate is required here")]
pub trait Drilled {}

/// Characteristic: a boundary consumer dedicated to swarf (REQ-003's
/// subject). Implemented by [`crate::resources::SwarfBin`] in every state.
#[diagnostic::on_unimplemented(message = "`{Self}` is not a dedicated swarf waste consumer (REQ-003: all swarf must reach one)", label = "a swarf waste consumer is required here")]
pub trait SwarfConsumer {}
