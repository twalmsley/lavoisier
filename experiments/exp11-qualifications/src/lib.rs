//! # exp11-qualifications — EXP-11: qualifications and safety as types
//!
//! Tests open question 2 (candidate R18): can the compiler refuse an
//! unqualified person or an unguarded machine, with modeller-grade errors?
//!
//! The modelled fragment: a workshop drilling process that requires
//! **a certified drilling operator** (REQ-001) and **a machine guard fitted
//! to the drill** (REQ-002). Qualifications are characteristics (R6):
//! marker traits attached to budget-carrying person types; the guard's safe
//! state is its own type (R9, one type per state), reachable only through a
//! fitting process.
//!
//! ## Module map
//!
//! | Module | Contents |
//! |---|---|
//! | [`qualifications`] | The `Qualification` kind trait, qualification value types (`DrillingCert`, `WeldingCert`), and the marker characteristic traits (`CertifiedDriller`, `CertifiedWelder`, `Fitted`) |
//! | [`requirements`] | REQ-001 and REQ-002 as R10 requirement traits via `model_core::requirement!` |
//! | [`resources`] | The sealed resource family: the `Operator<Q, BUDGET_MS>` wrapper around `model_core::common::Person` (the recommended interop), the `Driller<SHIFT_MS>` parallel-type probe, `MachineGuard`/`FittedGuard`, plates and swarf, the `Site` boundary sink, with `boundary` and `processes` child modules (F-006, F-031) |
//!
//! ## The two person-type probes (interop with `model_core::common::Person`)
//!
//! * **Wrapper (recommended):** [`resources::Operator`] holds the common
//!   `Person` as a private field. Certification is a boundary process
//!   (`certify` wraps, `decertify` unwraps — the person is conserved through
//!   both), and the budget draw delegates to model-core's `draw_time`, so the
//!   R15 overdraw check is inherited, not duplicated.
//! * **Parallel type (probe):** [`resources::Driller`] is a downstream
//!   `reusable_resource!` type with no `Person` inside. It needs its own draw
//!   process and — because model-core's `Labour` cannot be minted downstream —
//!   its own forked labour type ([`resources::Effort`]) and sink. The
//!   duplication is the finding.
//!
//! This crate carries the full R1 conservation regime obligations listed in
//! model-core's crate docs.

#![recursion_limit = "2048"]
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![deny(let_underscore_drop)]
#![warn(missing_docs)]

pub mod qualifications;
pub mod requirements;
pub mod resources;
