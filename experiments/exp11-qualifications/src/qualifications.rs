//! Qualifications and safety states as characteristics (R6).
//!
//! A person's capabilities (certifications) and a machine's safety state are
//! **characteristics**, expressed exactly like the pilot's bolt
//! characteristics: marker traits for requirement bounds (R10, F-019), plus —
//! because qualified person types are *parameterized* over their
//! qualification (the R6 catalogue encoding) — a **kind trait**
//! ([`Qualification`]) bounding the qualification slot, so a transposed or
//! nonsensical parameter is a construction-site error (F-018), and one value
//! type per qualification.
//!
//! Each marker carries a single-line `#[diagnostic::on_unimplemented]`
//! message phrased for modellers (F-015). Single-line on purpose: trace.sh
//! has no scope awareness and a multi-line attribute between a doc tag and
//! its item would detach them (F-021).
//!
//! The qualification value types (`DrillingCert`, `WeldingCert`) are plain
//! public markers, **not resources**: they are characteristic values like the
//! pilot's `SizeM8`, carry no sealed state, and cannot mint anything. What
//! *grants* a qualification is the boundary process
//! `resources::boundary::certify` (a placeholder for the real
//! training/credentialing process, R12).

/// Kind trait (R6, F-018) for the qualification slot of a qualified-person
/// type: every type parameter standing for a qualification is bounded by it,
/// so `Operator<Steel, 5000>`-style transposition is a clear
/// construction-site error instead of a silent nonsense type.
pub trait Qualification {}

/// Qualification value: certified to operate the pillar drill.
pub struct DrillingCert;
impl Qualification for DrillingCert {}

/// Qualification value: certified to weld.
pub struct WeldingCert;
impl Qualification for WeldingCert {}

/// Characteristic: this person is certified for drilling. Attached to
/// qualified person types by one-line blanket impls over the time budget
/// (`impl<const MS: u64> CertifiedDriller for Operator<DrillingCert, MS>`),
/// never to the shared `model_core::common::Person` itself — `Person` carries
/// no qualification slot, so a direct impl would certify every person in the
/// model at once.
#[diagnostic::on_unimplemented(message = "`{Self}` is not certified for drilling (REQ-001: drilling requires a certified operator)", label = "a person certified for drilling is required here", note = "certification is a boundary process (R12): wrap the person first, e.g. `certify::<DrillingCert>(person)` - a plain `Person` carries no qualification")]
pub trait CertifiedDriller {}

/// Characteristic: this person is certified for welding. Exists here as the
/// second qualification, to measure the marginal cost of adding one and to
/// serve as the wrong-certification foil in the compile-fail tests.
#[diagnostic::on_unimplemented(message = "`{Self}` is not certified for welding", label = "a person certified for welding is required here")]
pub trait CertifiedWelder {}

/// Characteristic: a machine guard **fitted to the machine** — the safe
/// state. Implemented only by `resources::FittedGuard`, never by the unfitted
/// `MachineGuard` (one type per state, R9/F-023), which is what lets REQ-002
/// distinguish "guard present but not fitted" from "guard fitted" at compile
/// time.
#[diagnostic::on_unimplemented(message = "`{Self}` is not a machine guard fitted to the drill (REQ-002: drilling requires the guard fitted)", label = "a fitted machine guard is required here", note = "fitting is a process (one type per state, R9): `fit_guard(MachineGuard) -> FittedGuard`")]
pub trait Fitted {}
