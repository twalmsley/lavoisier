//! # cs5-logistics — case study CS-5, the logistics subsystem
//!
//! The middle crate of the CS-5 three-crate split (SPEC §3, review decision
//! 1): it owns the **contracted courier organisation** and everything about
//! moving goods between the two sites. Implemented downstream of
//! `model-core` and `cs5-supply` from the agreed specification
//! `case-studies/cs5-two-site/SPEC.md` (v0.2, agreed); `cs5-works` depends
//! on it for the transport half of the flow.
//!
//! ## Module map
//!
//! | Module | Contents |
//! |---|---|
//! | [`characteristics`] | The sealed [`characteristics::ContractedCourier`] characteristic behind REQ-027, with its permit-gated conserving departure (F-054) |
//! | [`requirements`] | REQ-027 as an R10 requirement trait with a REQ-phrased `on_unimplemented` (R10 rule 8, F-044); the workspace sequence puts it after the works' REQ-025/026 numerically, but ownership is by subsystem (SPEC §3) |
//! | [`resources`] | One sealed type per location-state (R9; SPEC §3): [`resources::CourierAtUk`] → [`resources::OutboundCourier`] → [`resources::CourierAtVendor`] → [`resources::InboundCourier`] → back home — plus the `boundary` and the transport `processes` (consign / carry / purchase-at-vendor / hand-over) |
//!
//! ## Transit is a pure state change (R9; SPEC §7)
//!
//! Nothing is lost or delayed in carriage: each leg is a conserving
//! conversion between location-state types, with the cargo — the purchase
//! order outbound, the boxed component inbound — held **by value** inside
//! the state type, so a courier abandoned mid-journey trips its cargo's
//! tripwire at test time (R1 layer 2). **Only the order travels** (SPEC §8
//! review decision 2): the payment is abstracted — remitted flow-routed from
//! the bureau's output to the vendor's consumer at P3 — so no money type
//! appears in any transit state.
//!
//! ## Conservation regime (R1)
//!
//! This crate carries the full obligations listed in `model-core`'s crate
//! docs: `recursion_limit = "2048"`, `forbid(unsafe_code)`, the
//! must-use/underscore lints, and CI clippy under `-D warnings` with the
//! workspace `clippy.toml` ban list. Never write `let _ = <resource>`, and
//! ignore compiler fix-its suggesting borrows, clones, drops or `.expect(…)`
//! (F-007).

#![recursion_limit = "2048"]
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![deny(let_underscore_drop)]
#![warn(missing_docs)]

pub mod characteristics;
pub mod requirements;
pub mod resources;
