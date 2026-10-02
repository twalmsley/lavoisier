//! Flows handling **both arms** of the fallible drilling step (R17), plus the
//! bounded repair-and-retry composition.
//!
//! These flows use only the public model API (boundary constructors, the
//! processes, the `Consumer` sinks) — no `mint`/`defuse` — so they compose
//! exactly the way a downstream flow would.
//!
//! Three structural facts about fallible flows, demonstrated here (R17):
//!
//! 1. **Shared spends are drawn before the branch** (F-048): each attempt's
//!    labour is an adjacent `qualified_draw_time` in the flow, recorded into
//!    the `History` (R16) before the fallible step runs — so the operator
//!    returns at the same budget in both arms and failure costs time too.
//! 2. **Converging after a fallible step needs equal types in both arms**
//!    (F-046). Reusable resources converge when both arms restore the same
//!    state (the bit comes back working in the success arm and *repaired* in
//!    the failure arm; both arms bin swarf of the same magnitude, so even the
//!    contents-keeping `SwarfBin` converges). Products that exist in only one
//!    arm cannot converge; they come back as `Option<…>` (value-level
//!    presence, type-level conservation — a dropped `Some(resource)` still
//!    trips the tripwire; `unused_must_use` does not see through `Option`,
//!    F-046).
//! 3. **Bounded rework is bounded by provisioning** (F-050). A retry consumes
//!    a provisioned reserve (a second blank, a second trial token, a repair
//!    kit), so the retry count is fixed by the resources passed in —
//!    unbounded rework is inexpressible without unbounded inputs. On early
//!    success the reserves come back and must be re-accounted (to
//!    [`crate::resources::ToolStores`] / the environment), which is the
//!    honest cost of over-provisioning. Retry paths spend different amounts,
//!    so the retried flow returns the `#[must_use]` outcome enum
//!    [`RetryOutcome`], one variant per path.

use crate::characteristics::DrillingCert;
use crate::resources::processes::{discard_swarf, drill_holes_fallible, repair_bit};
use crate::resources::{
    DrillBit, DrillFail, DrillOutcome, DrilledPlate, FittedGuard, Plate, ScrapYard, SpareParts,
    Swarf, SwarfBin,
};
use model_core::boundary::send_to;
use model_core::common::Qualified;
use model_core::common::processes::qualified_draw_time;
use model_core::history::History;
use model_core::history::processes::record;
use model_core::list::{Cons, Nil};
use model_core::nat::aliases::{N1, N2};
use model_core::nat::{Succ, Zero};

/// One fallible drilling attempt with **both arms handled and converged**
/// (R17): the attempt's time is drawn and recorded *before* the branch
/// (F-048), swarf goes to the dedicated bin in both arms (REQ-003; both arms
/// gouge the same 20 g, so even the contents-keeping bin converges, F-046),
/// scrap goes to the yard, and a broken bit is repaired on the spot — so both
/// arms return the *same* operator, guard and bit types and the updated
/// sinks. The per-arm differences are value-level: the product and the unused
/// repair kit come back as `Option`s.
///
/// The `match` is forced: the result is a `Result` (std `#[must_use]`), the
/// bundles have no `Debug` (no `unwrap()`, F-047), and the only way to reach
/// the sealed resources inside is to destructure both arms.
///
/// Verifies nothing by itself — the R5 per-arm tests live in
/// `tests/fallible_flows.rs`.
#[allow(clippy::type_complexity)] // the tuple is the flow's full account (R1)
#[allow(clippy::too_many_arguments)] // loose threading (R9/F-024): an aggregate would over-claim
pub fn drill_one_plate_handling_both_arms<S, C>(
    operator: Qualified<DrillingCert, 5000>,
    guard: FittedGuard,
    bit: DrillBit,
    plate: Plate<900>,
    outcome: DrillOutcome,
    parts: SpareParts,
    yard: ScrapYard,
    bin: SwarfBin<Succ<S>, C>,
    history: History,
) -> (
    Qualified<DrillingCert, 4000>,
    FittedGuard,
    DrillBit,
    Option<DrilledPlate<880>>,
    Option<SpareParts>,
    ScrapYard,
    SwarfBin<S, Cons<Swarf<20>, C>>,
    History,
) {
    // The spend shared by both arms is drawn before the branch (R17/F-048)
    // and recorded (R16): failure costs the same labour as success.
    let (labour, operator) = qualified_draw_time::<1000, 4000, 5000, _>(operator);
    let history = record(history, "drill_holes_fallible", labour);
    match drill_holes_fallible::<900, 880, 20, 880, 20, _, _>(operator, guard, bit, plate, outcome)
    {
        Ok(ok) => {
            let bin = discard_swarf(bin, ok.swarf);
            (
                ok.operator,
                ok.guard,
                ok.bit,
                Some(ok.plate),
                Some(parts),
                yard,
                bin,
                history,
            )
        }
        Err(fail) => {
            let yard = send_to(yard, fail.scrap);
            let bin = discard_swarf(bin, fail.swarf);
            let bit = repair_bit(fail.broken_bit, parts);
            (
                fail.operator,
                fail.guard,
                bit,
                None,
                None,
                yard,
                bin,
                history,
            )
        }
    }
}

/// The swarf bin after the retry flow's first attempt: one 20 g piece kept,
/// one slot left.
pub type BinAfterOneSwarf = SwarfBin<N1, Cons<Swarf<20>, Nil>>;

/// The swarf bin after both of the retry flow's attempts: full.
pub type BinAfterTwoSwarf = SwarfBin<Zero, Cons<Swarf<20>, Cons<Swarf<20>, Nil>>>;

/// The outcome of [`drill_with_one_retry`] (R17/F-050): the three paths of a
/// once-retried fallible step produce **three different resource
/// combinations** (different time spent, different reserves left, different
/// bin fill), so the flow's result is necessarily a three-way sum — there is
/// no single converged type (F-046). This is a grouping over sealed resources
/// (a `pub` enum is legal for groupings, R17): holding a variant means
/// holding the resources, and every tripwired field is still leak-protected.
#[must_use = "RetryOutcome bundles conserved outputs: every field must be accounted for"]
pub enum RetryOutcome {
    /// The first attempt succeeded: 1000 ms spent, and the provisioned
    /// reserves (second blank, second trial, repair kit) come back unused —
    /// the caller must re-account them (blank and kit to
    /// [`crate::resources::ToolStores`], the untried trial through
    /// [`crate::resources::boundary::return_outcome`]).
    FirstTry {
        /// The operator after one attempt's labour.
        operator: Qualified<DrillingCert, 9000>,
        /// The fitted guard, back (R2).
        guard: FittedGuard,
        /// The surviving bit.
        bit: DrillBit,
        /// The product.
        plate: DrilledPlate<880>,
        /// Unused reserve blank — back to stores.
        reserve_plate: Plate<900>,
        /// Unused reserve trial — back to the environment.
        reserve_outcome: DrillOutcome,
        /// Unused repair kit — back to stores.
        reserve_parts: SpareParts,
        /// The bin with the one attempt's swarf kept (REQ-003).
        bin: BinAfterOneSwarf,
    },
    /// The first attempt failed (scrap to the yard, swarf binned, labour
    /// recorded, the bit repaired with the kit) and the retry succeeded:
    /// 2000 ms spent, reserves consumed.
    Retried {
        /// The operator after two attempts' labour.
        operator: Qualified<DrillingCert, 8000>,
        /// The fitted guard, back (R2).
        guard: FittedGuard,
        /// The repaired bit, back from the second attempt.
        bit: DrillBit,
        /// The product, from the reserve blank.
        plate: DrilledPlate<880>,
        /// The bin, full with both attempts' swarf (REQ-003).
        bin: BinAfterTwoSwarf,
    },
    /// Both attempts failed: the rework budget is exhausted and the second
    /// failure comes back whole for the caller to account (bit to scrap or a
    /// further repair, scrap to the yard, swarf to the bin's last slot).
    GaveUp {
        /// The second attempt's failure bundle, untouched (R17: the caller
        /// handles the arm).
        fail: DrillFail<Qualified<DrillingCert, 8000>, FittedGuard, 880, 20>,
        /// The bin with the first attempt's swarf kept; one slot left for the
        /// bundle's swarf (REQ-003).
        bin: BinAfterOneSwarf,
    },
}

/// Bounded repair-and-retry (R17/F-050): drill a plate, and on failure scrap
/// it, repair the bit and try once more with the provisioned reserve blank.
/// **The bound is the provisioning**: exactly one reserve blank, one reserve
/// trial and one repair kit go in, so at most one retry can happen — a third
/// attempt has nothing left to consume.
///
/// Each attempt's time is drawn adjacently and recorded before its branch
/// (F-048/R16), so the retry paths genuinely cost more labour. First-attempt
/// bookkeeping (scrap, swarf, labour, repair) happens inside the flow; the
/// final state comes back as a [`RetryOutcome`] because the three paths spend
/// different amounts and therefore have three different types.
#[allow(clippy::too_many_arguments)] // loose threading (R9/F-024): an aggregate would over-claim
pub fn drill_with_one_retry(
    operator: Qualified<DrillingCert, 10_000>,
    guard: FittedGuard,
    bit: DrillBit,
    plate: Plate<900>,
    first: DrillOutcome,
    reserve_plate: Plate<900>,
    second: DrillOutcome,
    parts: SpareParts,
    yard: ScrapYard,
    bin: SwarfBin<N2, Nil>,
    history: History,
) -> (RetryOutcome, ScrapYard, History) {
    // Attempt 1: draw and record its labour before the branch (R17/F-048).
    let (labour, operator) = qualified_draw_time::<1000, 9000, 10_000, _>(operator);
    let history = record(history, "drill_holes_fallible", labour);
    match drill_holes_fallible::<900, 880, 20, 880, 20, _, _>(operator, guard, bit, plate, first) {
        Ok(ok) => {
            let bin = discard_swarf(bin, ok.swarf);
            (
                RetryOutcome::FirstTry {
                    operator: ok.operator,
                    guard: ok.guard,
                    bit: ok.bit,
                    plate: ok.plate,
                    reserve_plate,
                    reserve_outcome: second,
                    reserve_parts: parts,
                    bin,
                },
                yard,
                history,
            )
        }
        Err(fail) => {
            // Account the first failure: scrap to the yard, swarf to the
            // bin, the broken bit into the repair.
            let yard = send_to(yard, fail.scrap);
            let bin = discard_swarf(bin, fail.swarf);
            let bit = repair_bit(fail.broken_bit, parts);
            // Attempt 2 consumes the reserves (the bound, F-050) and costs
            // its own drawn-and-recorded labour.
            let (labour, operator) = qualified_draw_time::<1000, 8000, 9000, _>(fail.operator);
            let history = record(history, "drill_holes_fallible", labour);
            match drill_holes_fallible::<900, 880, 20, 880, 20, _, _>(
                operator,
                fail.guard,
                bit,
                reserve_plate,
                second,
            ) {
                Ok(ok) => {
                    let bin = discard_swarf(bin, ok.swarf);
                    (
                        RetryOutcome::Retried {
                            operator: ok.operator,
                            guard: ok.guard,
                            bit: ok.bit,
                            plate: ok.plate,
                            bin,
                        },
                        yard,
                        history,
                    )
                }
                Err(fail2) => (RetryOutcome::GaveUp { fail: fail2, bin }, yard, history),
            }
        }
    }
}
