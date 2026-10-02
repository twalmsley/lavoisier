//! Flows handling **both arms** of a fallible process (candidate R17), plus
//! the bounded repair-and-retry composition.
//!
//! These flows use only the public model API (boundary constructors, the
//! processes, the `Consumer` sinks) — no `mint`/`defuse` — so they compose
//! exactly the way a downstream flow would.
//!
//! Two structural facts about fallible flows, demonstrated here and discussed
//! in RESULTS.md:
//!
//! 1. **Converging after a fallible step needs equal types in both arms.**
//!    Reusable resources converge naturally when both arms restore the same
//!    state (the bit comes back working in the success arm, and *repaired* in
//!    the failure arm). Products that exist in only one arm cannot converge;
//!    they come back as `Option<…>` (value-level presence, type-level
//!    conservation — a dropped `Some(resource)` still trips the tripwire).
//! 2. **Bounded rework is bounded by provisioning.** Under strict
//!    conservation a retry consumes a provisioned reserve (a second blank, a
//!    second trial token, a repair kit), so the retry count is fixed by the
//!    resources passed in — unbounded rework is inexpressible without
//!    unbounded inputs. When the first attempt succeeds, the reserves come
//!    back and must be re-accounted (returned to [`crate::model::ToolStores`]
//!    / the environment), which is the honest cost of over-provisioning.

use crate::model::processes::{drill_fallible, repair_bit};
use crate::model::{
    DrillBit, DrillFail, DrillOutcome, DrilledPlate, Plate, ScrapYard, SpareParts,
};
use model_core::boundary::send_to;
use model_core::common::Person;
use model_core::history::History;
use model_core::history::processes::record;

/// One drilling attempt with **both arms handled and converged** (candidate
/// R17): scrap and swarf go to the yard, labour is recorded (R16), a broken
/// bit is repaired on the spot — so both arms return the *same* person type,
/// the *same* bit type and the updated sinks. The per-arm differences are
/// value-level: the product and the unused repair kit come back as `Option`s.
///
/// The `match` is forced: the result is a `Result` (std `#[must_use]`), the
/// bundles have no `Debug` (no `unwrap()`), and the only way to reach the
/// sealed resources inside is to destructure both arms.
#[allow(clippy::type_complexity)] // the tuple is the flow's full account (R1)
pub fn drill_one_plate_handling_both_arms(
    person: Person<5000>,
    bit: DrillBit,
    plate: Plate<450>,
    outcome: DrillOutcome,
    parts: SpareParts,
    yard: ScrapYard,
    history: History,
) -> (
    Person<4000>,
    DrillBit,
    Option<DrilledPlate<440>>,
    Option<SpareParts>,
    ScrapYard,
    History,
) {
    match drill_fallible::<1000, 4000, 5000, 450, 440, 10, 430, 20>(person, bit, plate, outcome) {
        Ok(ok) => {
            let yard = send_to(yard, ok.swarf);
            let history = record(history, "drill_fallible", ok.labour);
            (
                ok.person,
                ok.bit,
                Some(ok.plate),
                Some(parts),
                yard,
                history,
            )
        }
        Err(fail) => {
            let yard = send_to(yard, fail.scrap);
            let yard = send_to(yard, fail.swarf);
            let history = record(history, "drill_fallible", fail.labour);
            let bit = repair_bit(fail.broken_bit, parts);
            (fail.person, bit, None, None, yard, history)
        }
    }
}

/// The outcome of [`drill_with_one_retry`]: the three paths of a
/// once-retried fallible step produce **three different resource
/// combinations** (different time spent, different reserves left), so the
/// flow's result is necessarily a three-way sum — there is no single
/// converged type. This is a grouping over sealed resources, like the
/// bundles: holding a variant means holding the resources, and every
/// tripwired field is still leak-protected.
#[must_use = "RetryOutcome bundles conserved outputs: every field must be accounted for"]
pub enum RetryOutcome {
    /// The first attempt succeeded: 1000 ms spent, and the provisioned
    /// reserves (second blank, second trial, repair kit) come back unused —
    /// the caller must re-account them (to [`crate::model::ToolStores`] and
    /// [`crate::model::boundary::return_outcome`]).
    FirstTry {
        /// The person after one attempt.
        person: Person<9000>,
        /// The surviving bit.
        bit: DrillBit,
        /// The product.
        plate: DrilledPlate<440>,
        /// Unused reserve blank.
        reserve_plate: Plate<450>,
        /// Unused reserve trial.
        reserve_outcome: DrillOutcome,
        /// Unused repair kit.
        reserve_parts: SpareParts,
    },
    /// The first attempt failed (scrap and swarf went to the yard, the
    /// labour was recorded, the bit was repaired with the kit) and the retry
    /// succeeded: 2000 ms spent, reserves consumed.
    Retried {
        /// The person after two attempts.
        person: Person<8000>,
        /// The repaired bit, back from the second attempt.
        bit: DrillBit,
        /// The product, from the reserve blank.
        plate: DrilledPlate<440>,
    },
    /// Both attempts failed: the rework budget is exhausted and the second
    /// failure comes back whole for the caller to account (bit to scrap or a
    /// further repair, scrap and swarf to the yard, labour to the history).
    GaveUp(DrillFail<8000, 430, 20, 1000>),
}

/// Bounded repair-and-retry (candidate R17): drill a plate, and on failure
/// scrap it, repair the bit and try once more with the provisioned reserve
/// blank. **The bound is the provisioning**: exactly one reserve blank, one
/// reserve trial and one repair kit go in, so at most one retry can happen —
/// a third attempt has nothing left to consume.
///
/// First-attempt bookkeeping (scrap, swarf, labour, repair) happens inside
/// the flow; the final state comes back as a [`RetryOutcome`] because the
/// three paths spend different amounts of time and reserves and therefore
/// have three different types (see the module docs).
#[allow(clippy::type_complexity)]
pub fn drill_with_one_retry(
    person: Person<10_000>,
    bit: DrillBit,
    plate: Plate<450>,
    first: DrillOutcome,
    reserve_plate: Plate<450>,
    second: DrillOutcome,
    parts: SpareParts,
    yard: ScrapYard,
    history: History,
) -> (RetryOutcome, ScrapYard, History) {
    match drill_fallible::<1000, 9000, 10_000, 450, 440, 10, 430, 20>(person, bit, plate, first) {
        Ok(ok) => {
            let yard = send_to(yard, ok.swarf);
            let history = record(history, "drill_fallible", ok.labour);
            (
                RetryOutcome::FirstTry {
                    person: ok.person,
                    bit: ok.bit,
                    plate: ok.plate,
                    reserve_plate,
                    reserve_outcome: second,
                    reserve_parts: parts,
                },
                yard,
                history,
            )
        }
        Err(fail) => {
            // Account the first failure: scrap and swarf to the yard, labour
            // to the history, the broken bit into the repair.
            let yard = send_to(yard, fail.scrap);
            let yard = send_to(yard, fail.swarf);
            let history = record(history, "drill_fallible", fail.labour);
            let bit = repair_bit(fail.broken_bit, parts);
            // The retry consumes the reserves (the bound).
            match drill_fallible::<1000, 8000, 9000, 450, 440, 10, 430, 20>(
                fail.person,
                bit,
                reserve_plate,
                second,
            ) {
                Ok(ok) => {
                    let yard = send_to(yard, ok.swarf);
                    let history = record(history, "drill_fallible", ok.labour);
                    (
                        RetryOutcome::Retried {
                            person: ok.person,
                            bit: ok.bit,
                            plate: ok.plate,
                        },
                        yard,
                        history,
                    )
                }
                Err(fail2) => (RetryOutcome::GaveUp(fail2), yard, history),
            }
        }
    }
}
