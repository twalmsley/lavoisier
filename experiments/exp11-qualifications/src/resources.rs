//! The sealed resource family (R1) for the EXP-11 drilling fragment.
//!
//! Layout per F-006/F-031: one module holding the sealed types, with the
//! creation [`boundary`] and the conserving [`processes`] as child modules.
//! Everything is created only through [`boundary`] (production, R12) or the
//! macro-generated `test_fixture` constructors (`test-support`, F-004).
//!
//! The two qualified-person designs under probe:
//!
//! * [`Operator`] — the **wrapper** (recommended): holds the common
//!   [`Person`] as a private field; certification wraps, decertification
//!   unwraps, budget draws delegate to model-core's `draw_time`.
//! * [`Driller`] — the **parallel type** (probe): a downstream
//!   `reusable_resource!` with no `Person` inside, needing its own draw
//!   process and its own forked labour type ([`Effort`]) because model-core's
//!   `Labour` cannot be minted downstream.

use core::marker::PhantomData;

use model_core::boundary::Consumer;
use model_core::common::Person;

use crate::qualifications::{
    CertifiedDriller, CertifiedWelder, DrillingCert, Fitted, Qualification, WeldingCert,
};
use crate::requirements::{assert_req001, assert_req002};

// ---------------------------------------------------------------------------
// The wrapper: a qualified person is the common Person plus a qualification
// ---------------------------------------------------------------------------

/// A qualified person: the common [`Person`] (R11) wrapped with a
/// qualification `Q` (kind trait [`Qualification`], R6/F-018) and carrying
/// the person's remaining time budget of `BUDGET_MS` person-milliseconds
/// (R15).
///
/// Reusable (R2): moved in and returned by every process; no tripwire (it
/// legitimately outlives every flow). Sealed (R1): private fields, no public
/// constructor, no `Clone`/`Copy`/`Default`; created only by
/// [`boundary::certify`], which conserves the person by holding them inside
/// the wrapper — [`boundary::decertify`] gives the same person back.
///
/// The qualification markers attach by one-line blanket impls **over the
/// budget** (below): every `Operator<DrillingCert, MS>` is a
/// [`CertifiedDriller`] whatever its remaining budget. They are never
/// implemented on `Person` itself: `Person` has no qualification slot, so a
/// direct impl would certify every person in the model at once.
#[must_use = "Operator is a reusable resource: pass them on or return them to the caller"]
pub struct Operator<Q: Qualification, const BUDGET_MS: u64> {
    /// The person continues inside the wrapper (conservation, R1):
    /// certification wraps, decertification unwraps, nothing is lost.
    person: Person<BUDGET_MS>,
    _q: PhantomData<Q>,
}

// Bridging impls (R6: "bridging impls are part of the catalogue"): one line
// per qualification value, blanket over the time budget. Added in the same
// commit as the qualification value itself.
impl<const MS: u64> CertifiedDriller for Operator<DrillingCert, MS> {}
impl<const MS: u64> CertifiedWelder for Operator<WeldingCert, MS> {}

/// A drilling-certified operator with `BUDGET_MS` person-milliseconds left —
/// the readable name for `Operator<DrillingCert, _>`. The alias carries the
/// satisfaction tag because tags inside macro invocations are invisible to
/// trace.sh (F-037) and `Operator` itself is only REQ-001-fit with the
/// drilling qualification in its slot.
///
/// Satisfies: REQ-001
pub type DrillingOperator<const BUDGET_MS: u64> = Operator<DrillingCert, BUDGET_MS>;
model_core::satisfies!(assert_req001, DrillingOperator<0>);

// ---------------------------------------------------------------------------
// The parallel-type probe: a qualified person with no Person inside
// ---------------------------------------------------------------------------

model_core::reusable_resource! {
    /// PROBE (parallel-type variant): a drilling-certified person as a
    /// downstream type of its own, with `SHIFT_MS` person-milliseconds of
    /// shift left and **no [`Person`] inside**. Two lines to define via the
    /// kernel macro — but it forks the whole time-accounting family: it needs
    /// its own draw process ([`processes::driller_draw_time`]) and its own
    /// labour type ([`Effort`]), because model-core's `Labour` can only be
    /// minted by model-core's own `draw_time`.
    Driller<const SHIFT_MS: u64>,
    must_use = "Driller is a reusable resource: pass them on or return them to the caller"
}

// The blanket-impl-over-budgets pattern on a macro-built parallel type: one
// line, exactly as on the wrapper.
impl<const MS: u64> CertifiedDriller for Driller<MS> {}

/// The parallel-type driller under its requirement-facing name; the alias
/// carries the tag (F-037: tags inside the macro invocation above would be
/// silently dropped by trace.sh).
///
/// Satisfies: REQ-001
pub type ParallelDriller<const SHIFT_MS: u64> = Driller<SHIFT_MS>;
model_core::satisfies!(assert_req001, ParallelDriller<0>);

model_core::container_resource! {
    /// PROBE: the parallel type's expended shift time, in
    /// person-milliseconds — a **fork of model-core's `Labour`**, needed only
    /// because [`Driller`] holds no `Person` and `Labour::mint` is private to
    /// model-core. Conserved like any waste output: it must reach a
    /// `Consumer` ([`Site`] accepts it), or the execution history through the
    /// downstream `Recordable` impl below.
    Effort,
    unit = "person-milliseconds",
    must_use = "Effort is a conserved resource: it must be accounted for by a Consumer"
}

/// The forked labour is recordable into the execution history (R16) — but
/// only through one more hand-written impl the parallel type owes; model-core
/// grants `Labour` this for free. The `Permit` keeps `into_record` callable
/// only by `History`'s machinery (F-041), so this impl opens no disposal path
/// around the record.
impl<const MS: u64> model_core::history::Recordable for Effort<MS> {
    fn into_record(self, _permit: model_core::history::Permit) -> model_core::history::Event {
        // Boundary exit: the effort leaves the model into the record (the
        // consumer's sanctioned F-008 role, via this crate's own defuse).
        self.defuse();
        model_core::history::Event {
            process: model_core::history::UNATTRIBUTED,
            item: "Effort",
            magnitude: MS,
            unit: "person-milliseconds",
        }
    }
}

// ---------------------------------------------------------------------------
// Safety resources: the guard and its states
// ---------------------------------------------------------------------------

model_core::reusable_resource! {
    /// A machine guard **not fitted to anything** — the unsafe state.
    /// Reusable (R2); one type per state (R9): only [`FittedGuard`] carries
    /// the [`Fitted`] characteristic, so an unfitted guard at a drilling
    /// process is a compile error, distinct from "no guard at all".
    MachineGuard,
    must_use = "MachineGuard is a reusable resource: pass it on or return it to the caller"
}

model_core::reusable_resource! {
    /// A machine guard **fitted to the drill** — the safe state, reachable
    /// only through [`processes::fit_guard`] (fitting is a process, R9) and
    /// returned by every guarded process (R2).
    FittedGuard,
    must_use = "FittedGuard is a reusable resource: pass it on or return it to the caller"
}

// The safety characteristic lives on the safe state only (R9/F-023).
impl Fitted for FittedGuard {}

/// The fitted guard under its requirement-facing name; the alias carries the
/// tag (F-037: a tag inside the `reusable_resource!` invocation above would
/// be silently dropped by trace.sh).
///
/// Satisfies: REQ-002
pub type DrillReadyGuard = FittedGuard;
model_core::satisfies!(assert_req002, DrillReadyGuard);

// ---------------------------------------------------------------------------
// Work pieces and waste
// ---------------------------------------------------------------------------

model_core::container_resource! {
    /// A plate blank of `V` grams awaiting drilling (one type per processing
    /// state, R9).
    Plate,
    unit = "grams",
    must_use = "Plate is a conserved resource: pass it on or hand it to a Consumer"
}

model_core::container_resource! {
    /// A drilled plate of `V` grams — its own processing state (R9/F-023).
    DrilledPlate,
    unit = "grams",
    must_use = "DrilledPlate is a conserved resource: pass it on or hand it to a Consumer"
}

model_core::container_resource! {
    /// Swarf of `V` grams, the conserved waste of drilling (R1, R15).
    Swarf,
    unit = "grams",
    must_use = "Swarf is a conserved resource: hand it to a Consumer"
}

// ---------------------------------------------------------------------------
// The boundary sink
// ---------------------------------------------------------------------------

model_core::reusable_resource! {
    /// The workshop site at the system boundary: an unbounded sink
    /// (`Next = Self`, R15/F-029) for finished plates, swarf and the
    /// parallel-probe `Effort`. Legal only at the boundary; discards its
    /// intake (the sanctioned exception, F-029).
    ///
    /// Placeholder: site stores and waste skips — refine to finished-goods
    /// and waste-disposal consumers with real capacities.
    Site,
    must_use = "Site is a boundary consumer: pass it on or return it to the caller"
}

impl<const G: u64> Consumer<DrilledPlate<G>> for Site {
    type Next = Site;
    fn consume(self, item: DrilledPlate<G>) -> Site {
        item.defuse(); // sanctioned consumer role (F-008); unbounded sink discards (F-029)
        self
    }
}

impl<const G: u64> Consumer<Swarf<G>> for Site {
    type Next = Site;
    fn consume(self, item: Swarf<G>) -> Site {
        item.defuse();
        self
    }
}

impl<const MS: u64> Consumer<Effort<MS>> for Site {
    type Next = Site;
    fn consume(self, item: Effort<MS>) -> Site {
        item.defuse();
        self
    }
}

/// The creation boundary (R12): the only production code allowed to create
/// this family's resources.
pub mod boundary {
    use core::marker::PhantomData;

    use model_core::common::Person;

    use super::{Driller, MachineGuard, Operator, Plate, Site};
    use crate::qualifications::Qualification;

    /// Certification: a person becomes an operator qualified as `Q`. The
    /// person is conserved — they continue inside the wrapper (R1), and
    /// [`decertify`] gives them back. This is where a qualification enters
    /// the model (R12).
    ///
    /// Placeholder: certification body — refine to a real training process
    /// that consumes course time and produces sealed certificate evidence.
    pub fn certify<Q: Qualification, const BUDGET_MS: u64>(
        person: Person<BUDGET_MS>,
    ) -> Operator<Q, BUDGET_MS> {
        Operator {
            person,
            _q: PhantomData,
        }
    }

    /// Decertification: the wrapper opens and the same person steps out,
    /// budget intact — the conserving inverse of [`certify`].
    pub fn decertify<Q: Qualification, const BUDGET_MS: u64>(
        operator: Operator<Q, BUDGET_MS>,
    ) -> Person<BUDGET_MS> {
        let Operator { person, _q: _ } = operator;
        person
    }

    /// A parallel-type driller enters the model with a shift budget.
    ///
    /// Placeholder: workforce — one certified driller with a fixed shift.
    pub fn hire_driller<const SHIFT_MS: u64>() -> Driller<SHIFT_MS> {
        Driller::mint()
    }

    /// A machine guard enters the model, unfitted.
    ///
    /// Placeholder: stores — one guard off the shelf.
    pub fn supply_guard() -> MachineGuard {
        MachineGuard::mint()
    }

    /// A plate blank of `G` grams enters the model.
    ///
    /// Placeholder: steel stockist — refine to a finite supplier.
    pub fn supply_plate<const G: u64>() -> Plate<G> {
        Plate::mint()
    }

    /// The site boundary sink enters the model.
    ///
    /// Placeholder: site stores and waste skips.
    pub fn new_site() -> Site {
        Site::mint()
    }
}

/// Conserving processes. Continuous-resource processes mint quantity-bearing
/// values, so they live inside the resource family's module (F-031).
pub mod processes {
    use core::marker::PhantomData;

    use model_core::common::Labour;
    use model_core::common::processes::draw_time;

    use super::{DrilledPlate, Driller, Effort, FittedGuard, MachineGuard, Operator, Plate, Swarf};
    use crate::qualifications::{DrillingCert, Qualification};
    use crate::requirements::{Req001CertifiedDrillingOperator, Req002FittedDrillGuard};

    /// Fits the guard to the drill: fitting is a process, and the fitted
    /// state is its own type (R9/F-023) — only [`FittedGuard`] carries the
    /// `Fitted` characteristic REQ-002 requires.
    pub fn fit_guard(guard: MachineGuard) -> FittedGuard {
        // Conserving conversion: the guard continues as the fitted state.
        let MachineGuard { _seal: _ } = guard;
        FittedGuard::mint()
    }

    /// Removes the guard: the conserving inverse of [`fit_guard`], returning
    /// the unfitted state.
    pub fn remove_guard(fitted: FittedGuard) -> MachineGuard {
        let FittedGuard { _seal: _ } = fitted;
        MachineGuard::mint()
    }

    /// Draws `SPEND` person-milliseconds from the **wrapped** person's budget
    /// (R15), leaving `LEFT`: the wrapper opens, model-core's `draw_time`
    /// does the draw (so the overdraw check and the [`Labour`] output are
    /// inherited, not duplicated), and the same person is re-wrapped with the
    /// qualification intact. Generic over `Q`: one draw serves every
    /// qualification.
    ///
    /// ```
    /// use exp11_qualifications::qualifications::DrillingCert;
    /// use exp11_qualifications::resources::boundary::{certify, decertify};
    /// use exp11_qualifications::resources::processes::operator_draw_time;
    /// use model_core::common::boundary::new_person;
    /// use model_core::history::boundary::new_history;
    /// use model_core::history::processes::record;
    ///
    /// let operator = certify::<DrillingCert, 5000>(new_person::<5000>());
    /// let (labour, operator) = operator_draw_time::<2000, 3000, 5000, _>(operator);
    /// let history = record(new_history(), "operator_draw_time", labour);
    /// assert_eq!(history.event_count(), 1);
    /// let _person_keeps_3000_ms = decertify(operator);
    /// let _execution_record = history;
    /// ```
    ///
    /// Overspending through the wrapper is the same compile error as
    /// overspending a bare person (R15; E0080 at monomorphization, invisible
    /// to `cargo check`, F-001). Regression — a 4000 ms operator cannot spend
    /// 6000 ms:
    ///
    /// ```compile_fail
    /// use exp11_qualifications::qualifications::DrillingCert;
    /// use exp11_qualifications::resources::processes::operator_draw_time;
    /// use exp11_qualifications::resources::boundary::certify;
    /// use model_core::common::boundary::new_person;
    ///
    /// let operator = certify::<DrillingCert, 4000>(new_person::<4000>());
    /// let (labour, operator) = operator_draw_time::<6000, 0, 4000, _>(operator);
    /// ```
    pub fn operator_draw_time<
        const SPEND: u64,
        const LEFT: u64,
        const BUDGET: u64,
        Q: Qualification,
    >(
        operator: Operator<Q, BUDGET>,
    ) -> (Labour<SPEND>, Operator<Q, LEFT>) {
        let Operator { person, _q: _ } = operator;
        let (labour, person) = draw_time::<SPEND, LEFT, BUDGET>(person);
        (
            labour,
            Operator {
                person,
                _q: PhantomData,
            },
        )
    }

    /// PROBE: the parallel type's own draw — everything model-core's
    /// `draw_time` already does, restated downstream (the duplication cost of
    /// not wrapping `Person`), with the forked [`Effort`] in place of
    /// `Labour`.
    pub fn driller_draw_time<const SPEND: u64, const LEFT: u64, const SHIFT: u64>(
        driller: Driller<SHIFT>,
    ) -> (Effort<SPEND>, Driller<LEFT>) {
        const {
            assert!(
                SPEND + LEFT == SHIFT,
                "time budget violated in driller_draw_time (R15): SPEND + LEFT must equal SHIFT - is more time being spent than the driller has left?"
            )
        };
        let Driller { _seal: _ } = driller;
        (Effort::mint(), Driller::mint())
    }

    /// Drills a plate behind a fitted guard — **style A, the recommended
    /// form**: the operator and the guard are generic parameters bounded by
    /// the requirement traits (R10, F-019), so an unqualified person or an
    /// unfitted guard fails with the modeller-phrased `on_unimplemented`
    /// message, and any certified type (wrapper or parallel) is accepted.
    /// Both reusables come back (R2); plate mass is conserved into the
    /// drilled plate plus swarf (R3; the F-001 caveat applies). The time draw
    /// is its own process (R15): run [`operator_draw_time`] in the flow.
    ///
    /// The whole signature sits on one line because trace.sh attributes
    /// requirement bounds to the line they are written on (R10 rule 4,
    /// F-021).
    ///
    /// Satisfies: REQ-001, REQ-002
    pub fn drill_plate<const PLATE: u64, const P_LEFT: u64, const SW: u64, O: Req001CertifiedDrillingOperator, G: Req002FittedDrillGuard>(operator: O, guard: G, plate: Plate<PLATE>) -> (O, G, DrilledPlate<P_LEFT>, Swarf<SW>) {
        const {
            assert!(
                P_LEFT + SW == PLATE,
                "mass conservation violated in drill_plate (R3): the drilled plate plus the swarf must sum exactly to the plate blank"
            )
        };
        // Conserving transform: the blank's mass continues as P_LEFT + SW.
        plate.defuse();
        (operator, guard, DrilledPlate::mint(), Swarf::mint())
    }

    /// Drills a plate and draws the operator's time **inside** the process —
    /// style B, the pilot `drill_holes` shape. Because the budget changes
    /// type, the operator must be the **concrete** wrapper
    /// `Operator<DrillingCert, BUDGET>` (a generic `O` cannot change its own
    /// const parameter); REQ-001 is restated as a where-clause bound on the
    /// concrete type so the link stays greppable, but a wrong operator here
    /// is an E0308 type mismatch, not the REQ-phrased `on_unimplemented`
    /// error — which is why style A is preferred where error quality matters.
    ///
    /// Satisfies: REQ-001, REQ-002
    pub fn drill_plate_timed<const SPEND: u64, const T_LEFT: u64, const BUDGET: u64, const PLATE: u64, const P_LEFT: u64, const SW: u64, G: Req002FittedDrillGuard>(operator: Operator<DrillingCert, BUDGET>, guard: G, plate: Plate<PLATE>) -> (Operator<DrillingCert, T_LEFT>, G, DrilledPlate<P_LEFT>, Swarf<SW>, Labour<SPEND>) where Operator<DrillingCert, BUDGET>: Req001CertifiedDrillingOperator {
        const {
            assert!(
                P_LEFT + SW == PLATE,
                "mass conservation violated in drill_plate_timed (R3): the drilled plate plus the swarf must sum exactly to the plate blank"
            )
        };
        let (labour, operator) =
            operator_draw_time::<SPEND, T_LEFT, BUDGET, DrillingCert>(operator);
        plate.defuse();
        (operator, guard, DrilledPlate::mint(), Swarf::mint(), labour)
    }
}

#[cfg(test)]
mod tests {
    use model_core::boundary::send_to;

    use super::boundary::{new_site, supply_guard, supply_plate};
    use super::processes::{fit_guard, remove_guard};

    /// Fitting and removing the guard round-trips the two safety states
    /// (R9: one type per state).
    #[test]
    fn guard_states_round_trip() {
        let guard = supply_guard();
        let fitted = fit_guard(guard);
        let _back_on_the_shelf = remove_guard(fitted);
    }

    /// An abandoned plate is caught by the tripwire (R1 layer 2, F-032) —
    /// qualification machinery does not weaken the conservation regime.
    #[test]
    #[should_panic(expected = "Plate<900> dropped without being consumed")]
    fn abandoned_plate_trips_the_tripwire() {
        let plate = supply_plate::<900>();
        let _still_bound_but_never_consumed = plate;
    }

    /// The site sink accounts for drilled plates and swarf (F-029 unbounded
    /// consumer), keeping the family's conserved outputs accountable in
    /// production code (F-035).
    #[test]
    fn site_accounts_for_outputs() {
        let plate = supply_plate::<900>();
        let (drilled, swarf) = {
            // In-module shortcut: this test sits inside the privacy boundary,
            // so it may run the conserving transform by hand.
            plate.defuse();
            (super::DrilledPlate::<880>::mint(), super::Swarf::<20>::mint())
        };
        let site = new_site();
        let site = send_to(site, drilled);
        let site = send_to(site, swarf);
        let _site = site;
    }
}
