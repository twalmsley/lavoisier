//! # EXP-10 — Fallible processes (candidate R17)
//!
//! Tests open question 1 of `instructions.md`: can a process return *either*
//! outcome — success or failure — with **both branches conserving**, and does
//! the compiler force flows to handle the failure arm?
//!
//! Built on `model-core` (the EXP-10..12 protocol update): the kernel macros
//! define the sealed resources, `Person`/`Labour`/`draw_time` carry the time
//! budget, and `History` is the production-legal labour sink (R16, F-035).
//!
//! | Module | Contents |
//! |---|---|
//! | [`model`] | The resource family: success- and failure-state resources (`DrilledPlate` vs `ScrapPlate`, `DrillBit` vs `BrokenDrillBit`), the sealed [`model::DrillOutcome`] token, boundary sinks ([`model::ScrapYard`], [`model::Customer`], [`model::ToolStores`]), and the processes [`model::processes::drill_fallible`] / [`model::processes::repair_bit`] |
//! | [`two_token`] | The design probe rejected by this experiment: two distinct token types (`WillSucceed`/`WillFail`) selected by the flow, including the GAT form |
//! | [`flows`] | Flows handling both arms: a converging single-attempt flow and the bounded repair-and-retry composition |
//!
//! Conservation regime (R1): this crate carries the standard obligations —
//! `recursion_limit`, `forbid(unsafe_code)`, the must-use/underscore lints,
//! tripwires on every consumable (kernel-generated). Never write
//! `let _ = <resource>` (F-007).

#![recursion_limit = "2048"]
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![deny(let_underscore_drop)]

pub mod flows;
pub mod model;
pub mod two_token;
