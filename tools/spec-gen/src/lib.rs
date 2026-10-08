//! spec-gen — the specification gate and scaffolder (R22, from EXP-15/F-062).
//!
//! A std-only library parsing the agreed SPEC_TEMPLATE.md format, behind two
//! binaries:
//! - `speccheck`: validates specifications against the template's machine
//!   conventions (A1–A10) — balance arithmetic, waste-destination closure,
//!   canonical-identifier name closure, Satisfies-id closure, draw/balance
//!   time agreement, flow-order closure, workspace-unique REQ ids (F-053).
//!   This error class fires before any code exists; errors carry §/line
//!   references and are phrased at the document ("fix the specification,
//!   not the model"). It never guesses: what it cannot verify is a warning
//!   that fails the gate.
//! - `specgen`: one-shot scaffolding of the spec-determined model structure,
//!   under the generation regime (deterministic; §/line breadcrumbs; the
//!   `deny-holes` `compile_error!` convention for everything the spec
//!   under-determines). The output is promoted to hand-maintained in the
//!   same change that commits it; there is no regeneration round-trip.

#![forbid(unsafe_code)]

pub mod emit;
pub mod model;
pub mod parse;
