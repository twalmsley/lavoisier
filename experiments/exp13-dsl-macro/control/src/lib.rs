//! # exp13-control — the CS-1 slice, hand-written (the control arm)
//!
//! The water/electricity/kettle/boil/requirement/flow slice of
//! `model/cs1-pot-of-tea`, lifted and trimmed per the EXP-13 brief: the
//! ColdWater/Electricity containers, the mains/grid boundary draws,
//! `fill_kettle` and `boil` with the energy assert, one requirement (boiling
//! water, R10 pattern including rule 8) used by `pour_cuppa`, and one flow
//! (`tests/flows.rs`). Everything here is the direct-Rust baseline the `model!`
//! macro front-end (the `exp13-dsl` crate) is measured against.

#![recursion_limit = "2048"]
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![deny(let_underscore_drop)]
#![warn(missing_docs)]

pub mod model;
