//! GENERATED from the agreed specification "CS-1: Making a Pot of Tea" by the
//! EXP-15 spec-as-DSL generator. Every item carries a breadcrumb to the
//! SPEC.md section/line it was recovered from; every under-determined
//! decision is a numbered SPEC-HOLE (see Cargo.toml for the convention).

#![recursion_limit = "2048"]
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![deny(let_underscore_drop)]

// ── SPEC-HOLE U-01 ──────────────────────────────────────────
// The spec's §5 blocks determine signatures and balances, but not the R5 test
// fixtures, the R4 trybuild cases, or the rustdoc compile_fail regressions the
// real crate carries — all were implementation judgement.
#[cfg(feature = "deny-holes")]
compile_error!("SPEC-HOLE U-01: per-process unit tests (R5) and compile-fail regressions (R4) are not derivable from the spec");

pub mod characteristics;
pub mod requirements;
pub mod resources;
