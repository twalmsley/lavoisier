//! # exp13-dsl — the CS-1 slice authored through the `model!` macro front-end
//!
//! The same water/electricity/kettle/boil/requirement/flow slice as the
//! `exp13-control` crate, but authored through [`model!`] (defined in
//! [`macros`]): the EXP-13 experiment arm. The two crates expose the same
//! model surface (same type names, same process signatures, same flow), so
//! LOC, compile time, error transcripts and tool compatibility compare
//! like-for-like.

#![recursion_limit = "2048"]
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![deny(let_underscore_drop)]
#![warn(missing_docs)]

pub mod macros;
pub mod model;
