//! EXP-15: the spec as the DSL. A std-only generator that parses the agreed
//! SPEC_TEMPLATE.md format (corpus: the real CS-1 SPEC.md) and emits as much
//! of the model crate as the spec mechanically determines, with numbered
//! SPEC-HOLEs where it under-determines.

pub mod model;
pub mod parse;
pub mod emit;
