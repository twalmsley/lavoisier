//! # cs5-works — case study CS-5, the works subsystem
//!
//! The downstream crate of the CS-5 three-crate split (SPEC §3, review
//! decision 1): the UK workshop that fulfils the customer's order.
//! Implemented from the agreed natural-language specification
//! `case-studies/cs5-two-site/SPEC.md` (v0.2, agreed), downstream of
//! `model-core`, `cs5-supply` and `cs5-logistics`. CS-5 is the
//! **boundary-refinement capstone**: two currencies with the bureau exchange
//! (R19's last unexercised clause), a placeholder refined into a finite
//! modelled supplier (R12 — the vendor, in `cs5-supply`), typed inter-site
//! transport (one type per location-state, in `cs5-logistics`), and the
//! model split across three team-shaped crates whose edges carry the seals
//! (EXP-08, F-005/F-006).
//!
//! ## Module map
//!
//! | Module | Contents |
//! |---|---|
//! | [`characteristics`] | The sealed [`characteristics::ProofOfOrder`] behind REQ-026, with its permit-gated fulfilment (F-054) |
//! | [`requirements`] | REQ-025 and REQ-026 as R10 requirement traits (F-044 messages); REQ-025 is the workspace's first **cross-crate** requirement bound — its characteristic is sealed in `cs5-supply` beside the type it marks |
//! | [`resources`] | The works' sealed family (R1): the GBP [`resources::Account`] whose balance is supply-sealed cash held by value, the housings and their 3-deep rack, the 1000 g [`resources::Instrument`] keeping its parts as real objects, the capacity-5 packaging [`resources::Bin`] (the F-034 shape), the deliberately-unrefined [`resources::Customer`] and their order token — with `boundary` and `processes` child modules (F-006, F-031) |
//! | [`flows`] | The full fulfilment in **both orderings** (housing pick-up timing; P8 placement), returning the single accounted end state [`flows::FulfilmentComplete`] — the type system proving the orderings equivalent (R9) |
//!
//! ## Cross-currency hygiene (R19 — the showcase)
//!
//! The works holds pence; the vendor sells in euro cents. Paying the vendor
//! in pence **does not compile** (`tests/ui/pay_vendor_in_pence.rs`), euro
//! cents cannot be minted on this side of the crate edge
//! (`tests/ui/works_cannot_mint_euro_cents.rs`), and the only bridge is the
//! bureau's exchange at its stated integer rate, where a non-exact amount
//! has no compiling output (F-052, pinned as `compile_fail` doc-tests in
//! `cs5-supply`). Each dimension balances separately: GBP as
//! 5000 − 2000 + 9000 = 12 000, euro cents as 2340 minted = 2340 paid.
//!
//! ## The operator's budget (R15; SPEC §3)
//!
//! One operator, 1 200 000 ms, drawn by **adjacent** `draw_time` processes
//! in the flow (F-048), each draw recorded to the single `History` under its
//! process name (R16): 120k + 60k + 60k + 60k + 180k + 120k + 30k =
//! 630 000 ms over seven attributed events (P3 has no draw), ending at
//! 570 000 ms. Overspending is the standard overdraw compile error (E0080
//! at monomorphization, invisible to `cargo check`, F-001). **Budget
//! overdraw regression** — the operator who ends the flow at 570 000 ms
//! cannot then be drawn for another 600 000:
//!
//! ```compile_fail
//! use model_core::common::boundary::new_person;
//! use model_core::common::processes::draw_time;
//!
//! let operator = new_person::<570_000>();
//! let (labour, operator) = draw_time::<600_000, 0, 570_000>(operator);
//! ```
//!
//! ## Conservation regime (R1)
//!
//! This crate carries the full obligations listed in `model-core`'s crate
//! docs: `recursion_limit = "2048"`, `forbid(unsafe_code)`, the
//! must-use/underscore lints, tripwire `Drop`s on every consumable (kernel
//! macros), and CI clippy under `-D warnings` with the workspace
//! `clippy.toml` ban list. Never write `let _ = <resource>`, and ignore
//! compiler fix-its suggesting borrows, clones, drops or `.expect(…)`
//! (F-007).

#![recursion_limit = "2048"]
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![deny(let_underscore_drop)]
#![warn(missing_docs)]

pub mod characteristics;
pub mod flows;
pub mod requirements;
pub mod resources;
