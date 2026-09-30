//! EXP-08: the R1 privacy boundary across a real crate boundary.
//!
//! Layout under test:
//! - `resources` defines resource types with private fields and **no** public
//!   constructors. The only code that may create resources lives in child
//!   modules of `resources` (`boundary`, `test_support`), because child
//!   modules can see the parent's private fields. Everything else — including
//!   the rest of this crate — cannot construct a resource.
//! - `processes` transforms resources by value (R1/R2) and needs no field
//!   access, so it is a sibling module.
//! - `holes` contains deliberately broken types, one per known leak in the
//!   pattern, so the downstream crate can demonstrate each hole.

pub mod resources;
pub mod processes;
pub mod holes;

// Re-export the boundary at the crate root for ergonomics; it still *lives*
// inside `resources` so the compiler, not convention, polices field access.
pub use resources::boundary;
#[cfg(feature = "test-support")]
pub use resources::test_support;
