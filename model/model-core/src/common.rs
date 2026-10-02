//! Reusable common resource types (R11): [`Person`], the qualified-person
//! wrapper [`Qualified`] (R18), [`Organisation`], [`Location`], and the
//! conserved [`Labour`] output of spending a person's time.
//!
//! Layout per F-006/F-031: this module is the resource family — the sealed
//! types, with the creation [`boundary`] and the continuous [`processes`]
//! (which mint quantity-bearing values and therefore must live inside the
//! privacy boundary) as child modules. All three common types are created
//! only via [`boundary`] (production, R12) or the `test_fixture`
//! constructors (`test-support` feature, F-004).
//!
//! Adapted from `experiments/exp09-continuous-resources/src/lib.rs`
//! (`Person`/`Labour`/`draw_time`) and
//! `experiments/exp11-qualifications/src/resources.rs`
//! (`Operator`/`certify`/`operator_draw_time`, promoted here as
//! [`Qualified`]/[`boundary::qualify`]/[`processes::qualified_draw_time`]
//! per F-043).

use core::marker::PhantomData;

crate::reusable_resource! {
    /// An organisation (R11): a company, team or service that takes part in
    /// the modelled system. Reusable (R2): moved into a process and returned
    /// as part of its output.
    ///
    /// Placeholder: generic organisation — refine to the named organisation
    /// in the modelling crate.
    Organisation,
    must_use = "Organisation is a reusable resource: pass it on or return it to the caller"
}

crate::reusable_resource! {
    /// A location (R11): a site, room or position where processes happen.
    /// Reusable (R2): moved into a process and returned as part of its
    /// output, so one location is in use by only one process at a time.
    ///
    /// Placeholder: generic location — refine to the named location in the
    /// modelling crate.
    Location,
    must_use = "Location is a reusable resource: pass it on or return it to the caller"
}

crate::container_resource! {
    /// Expended labour, in person-milliseconds (the R7 base time unit): the
    /// conserved output of [`processes::draw_time`] (R15). Like every waste
    /// or by-product it must eventually reach a `Consumer` — its
    /// production-legal sink is the execution history
    /// ([`crate::history::History`], R16/F-035), which records what was
    /// consumed instead of discarding it silently.
    Labour,
    unit = "person-milliseconds",
    must_use = "Labour is a conserved resource: it must be accounted for by a Consumer (the execution History, R16)"
}

/// A person with a remaining time budget of `BUDGET_MS` person-milliseconds
/// (R11, R15).
///
/// Reusable (R2): moved in and returned by every process that uses them; no
/// tripwire `Drop` (the tripwire regime applies to consumables — a reusable
/// resource legitimately outlives the flow and remains with the caller).
/// Sealed (R1): private field, no public constructor, no
/// `Clone`/`Copy`/`Default`; created only via [`boundary::new_person`] or
/// [`Person::test_fixture`].
///
/// The budget is drawn down by [`processes::draw_time`], exactly like a gas
/// bottle (R15): a person with no budget left cannot be drawn from, by the
/// same compile-time overdraw error. Model a time budget only where that
/// time is genuinely being accounted for (R15) — the budget const parameter
/// infects every signature the person passes through, and the modeller
/// maintains the running balance by hand (F-030; a wrong balance is a
/// compile error, so the arithmetic stays compiler-checked).
#[must_use = "Person is a reusable resource: pass them on or return them to the caller"]
pub struct Person<const BUDGET_MS: u64> {
    _seal: (),
}

impl<const BUDGET_MS: u64> Person<BUDGET_MS> {
    /// The remaining time budget, in person-milliseconds (R7).
    pub const BUDGET_MS: u64 = BUDGET_MS;

    /// Test fixture: a person from nowhere, for downstream test code only
    /// (R1, F-004; `test-support` feature, dev-dependencies only).
    #[cfg(feature = "test-support")]
    pub fn test_fixture() -> Self {
        Person { _seal: () }
    }
}

// The TimeLedger placeholder that used to live here is superseded by the
// execution history (R16): `crate::history::History` is the production-legal
// boundary sink for Labour (F-035), and unlike the ledger it keeps a
// value-level record of what it consumed.

/// Kind trait (R6, F-018) for the qualification slot of [`Qualified`] (R18):
/// every type parameter standing for a qualification is bounded by it, so a
/// transposed or nonsensical parameter is a clear construction-site error
/// instead of a silent nonsense type.
///
/// The trait is **open**: a modelling crate adds a qualification as one value
/// type plus one impl line (`pub struct DrillingCert; impl Qualification for
/// DrillingCert {}`). Qualification *markers* (`CertifiedDriller`-style
/// characteristic traits, R6) are downstream traits attached by one-line
/// blanket impls **over the budget**
/// (`impl<const MS: u64> CertifiedDriller for Qualified<DrillingCert, MS> {}`),
/// added in the same commit as the qualification value itself (F-043).
/// Markers are **never implemented on [`Person`] directly**: `Person` has no
/// qualification slot, so a direct impl would certify every person in the
/// model at once (F-043).
pub trait Qualification {}

/// A qualified person (R18): the common [`Person`] (R11) wrapped with a
/// qualification `Q` (kind trait [`Qualification`]) and carrying the person's
/// remaining time budget of `BUDGET_MS` person-milliseconds (R15).
///
/// Reusable (R2): moved in and returned by every process; no tripwire (a
/// reusable resource legitimately outlives every flow). Sealed (R1): private
/// fields, no public constructor, no `Clone`/`Copy`/`Default`; created only
/// by [`boundary::qualify`], which conserves the person by holding them
/// inside the wrapper — [`boundary::release`] gives the same person back,
/// budget intact. (The kernel macros cannot generate held contents on a
/// reusable resource, F-040, which is why this wrapper is hand-sealed.)
///
/// The wrapper's budget **is** the inner person's: the delegating draw
/// [`processes::qualified_draw_time`] opens the wrapper, calls
/// [`processes::draw_time`] and re-wraps, so the R15 overdraw error and the
/// [`Labour`]→`History` accounting (R16, F-035) are inherited, not
/// duplicated. Parallel person types that bypass `Person` fork that whole
/// machinery (~40 duplicated lines per type, F-043) — do not build them.
#[must_use = "Qualified is a reusable resource: pass them on or return them to the caller"]
pub struct Qualified<Q: Qualification, const BUDGET_MS: u64> {
    /// The person continues inside the wrapper (conservation, R1):
    /// qualification wraps, release unwraps, nothing is lost.
    person: Person<BUDGET_MS>,
    _q: PhantomData<Q>,
}

impl<Q: Qualification, const BUDGET_MS: u64> Qualified<Q, BUDGET_MS> {
    /// The remaining time budget, in person-milliseconds (R7) — the inner
    /// person's budget, exposed like [`Person::BUDGET_MS`].
    pub const BUDGET_MS: u64 = BUDGET_MS;
}

/// The creation boundary for the common types (R12): the only production
/// code allowed to create them.
pub mod boundary {
    use super::{Location, Organisation, Person, PhantomData, Qualification, Qualified};

    /// A person enters the model with a time budget (R15: model a budget
    /// only where the time is genuinely being accounted for).
    ///
    /// Placeholder: workforce — one person with a fixed shift budget.
    pub fn new_person<const BUDGET_MS: u64>() -> Person<BUDGET_MS> {
        Person { _seal: () }
    }

    /// Qualification (R18): a person becomes a [`Qualified`] person holding
    /// the qualification `Q`. This is where a qualification enters the model
    /// (R12); the person is **conserved** — they continue inside the wrapper
    /// (R1), and [`release`] gives them back, budget intact.
    ///
    /// Qualification markers (`CertifiedDriller`-style) attach downstream by
    /// blanket impls over the budget — see [`Qualification`] (F-043).
    ///
    /// Placeholder: certification body — refine to a real training process
    /// that consumes course time and produces sealed certificate evidence.
    pub fn qualify<Q: Qualification, const BUDGET_MS: u64>(
        person: Person<BUDGET_MS>,
    ) -> Qualified<Q, BUDGET_MS> {
        Qualified {
            person,
            _q: PhantomData,
        }
    }

    /// Release (R18): the wrapper opens and the same person steps out,
    /// budget intact — the conserving inverse of [`qualify`].
    ///
    /// Placeholder: certification body — refine alongside [`qualify`].
    pub fn release<Q: Qualification, const BUDGET_MS: u64>(
        qualified: Qualified<Q, BUDGET_MS>,
    ) -> Person<BUDGET_MS> {
        let Qualified { person, _q: _ } = qualified;
        person
    }

    /// An organisation enters the model.
    ///
    /// Placeholder: generic organisation.
    pub fn new_organisation() -> Organisation {
        Organisation::mint()
    }

    /// A location enters the model.
    ///
    /// Placeholder: generic location.
    pub fn new_location() -> Location {
        Location::mint()
    }
}

/// Conserving processes over the common types. Continuous-resource processes
/// mint new quantity-bearing values, so they live inside the resource
/// family's module (F-031), not outside the module tree like discrete
/// processes.
pub mod processes {
    use super::{Labour, Person, PhantomData, Qualification, Qualified};

    /// Draws `SPEND` person-milliseconds from a person's time budget,
    /// leaving `LEFT` (R15 time-budget pattern; caller-stated remainder,
    /// F-022/F-030). The expended time leaves as a conserved [`Labour`]
    /// value that must reach a `Consumer` — its production-legal sink is the
    /// execution history ([`crate::history`], R16/F-035), which records it
    /// attributed to the process that spent it.
    ///
    /// ```
    /// use model_core::common::boundary::new_person;
    /// use model_core::common::processes::draw_time;
    /// use model_core::history::boundary::new_history;
    /// use model_core::history::processes::record;
    ///
    /// let person = new_person::<10_000>();
    /// let (labour, person) = draw_time::<2000, 8000, 10_000>(person);
    /// let history = record(new_history(), "walk_to_station", labour);
    /// assert_eq!(history.event_count(), 1);
    /// let _execution_record = history;
    /// let _person_keeps_8000_ms = person;
    /// ```
    ///
    /// Overspending the budget is a compile error, exactly like overdrawing
    /// a container (R15; E0080 at monomorphization, invisible to
    /// `cargo check`, F-001). Regression — only 4000 ms left, drawing
    /// another 6000 ms must not compile:
    ///
    /// ```compile_fail
    /// use model_core::common::boundary::new_person;
    /// use model_core::common::processes::draw_time;
    ///
    /// let person = new_person::<10_000>();
    /// let (l1, person) = draw_time::<6000, 4000, 10_000>(person);
    /// let (l2, person) = draw_time::<6000, 0, 4000>(person);
    /// ```
    pub fn draw_time<const SPEND: u64, const LEFT: u64, const BUDGET: u64>(
        person: Person<BUDGET>,
    ) -> (Labour<SPEND>, Person<LEFT>) {
        const {
            assert!(
                SPEND + LEFT == BUDGET,
                "time budget violated in draw_time (R15): SPEND + LEFT must equal BUDGET - is more time being spent than the person has left?"
            )
        };
        // Reusable resource: a plain conserving move (no tripwire to defuse).
        let Person { _seal: () } = person;
        (Labour::mint(), Person { _seal: () })
    }

    /// Draws `SPEND` person-milliseconds from a **qualified** person's budget
    /// (R15, R18), leaving `LEFT`: the wrapper opens, [`draw_time`] does the
    /// draw — so the overdraw check and the [`Labour`] output (with its
    /// `History` sink, R16/F-035) are **inherited, not duplicated** — and the
    /// same person is re-wrapped with the qualification intact. Generic over
    /// `Q`: one draw serves every qualification.
    ///
    /// **Turbofish note (F-043):** the type parameter `Q` comes after the
    /// const parameters, and trailing generic arguments cannot be omitted —
    /// write a `_` for it: `qualified_draw_time::<2000, 3000, 5000, _>(op)`.
    /// Leaving it off is an E0107 "function takes 4 generic arguments but 3
    /// were supplied".
    ///
    /// **Diagnostic note (F-043):** because this draw delegates, an
    /// overdraw's E0080 "while instantiating" note lands on the internal
    /// `draw_time` call below — one hop from the modeller's flow line; the
    /// decimal magnitudes in the note still identify the offending call.
    ///
    /// ```
    /// use model_core::common::boundary::{new_person, qualify, release};
    /// use model_core::common::processes::qualified_draw_time;
    /// use model_core::common::Qualification;
    /// use model_core::history::boundary::new_history;
    /// use model_core::history::processes::record;
    ///
    /// /// Qualification value: certified to operate the pillar drill.
    /// pub struct DrillingCert;
    /// impl Qualification for DrillingCert {}
    ///
    /// let operator = qualify::<DrillingCert, 5000>(new_person::<5000>());
    /// let (labour, operator) = qualified_draw_time::<2000, 3000, 5000, _>(operator);
    /// let history = record(new_history(), "drill_holes", labour);
    /// assert_eq!(history.event_count(), 1);
    /// let _person_keeps_3000_ms = release(operator);
    /// let _execution_record = history;
    /// ```
    ///
    /// Overspending through the wrapper is the same compile error as
    /// overspending a bare person (R15; E0080 at monomorphization, invisible
    /// to `cargo check`, F-001). Regression — a 4000 ms operator cannot
    /// spend 6000 ms:
    ///
    /// ```compile_fail
    /// use model_core::common::boundary::{new_person, qualify};
    /// use model_core::common::processes::qualified_draw_time;
    /// use model_core::common::Qualification;
    ///
    /// pub struct DrillingCert;
    /// impl Qualification for DrillingCert {}
    ///
    /// let operator = qualify::<DrillingCert, 4000>(new_person::<4000>());
    /// let (labour, operator) = qualified_draw_time::<6000, 0, 4000, _>(operator);
    /// ```
    pub fn qualified_draw_time<
        const SPEND: u64,
        const LEFT: u64,
        const BUDGET: u64,
        Q: Qualification,
    >(
        qualified: Qualified<Q, BUDGET>,
    ) -> (Labour<SPEND>, Qualified<Q, LEFT>) {
        let Qualified { person, _q: _ } = qualified;
        let (labour, person) = draw_time::<SPEND, LEFT, BUDGET>(person);
        (
            labour,
            Qualified {
                person,
                _q: PhantomData,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::boundary::{new_location, new_organisation, new_person, qualify, release};
    use super::processes::{draw_time, qualified_draw_time};
    use super::{Person, Qualification, Qualified};
    use crate::history::boundary::new_history;
    use crate::history::processes::record;

    /// A test qualification value (the kind trait is open, R18).
    struct DrillingCert;
    impl Qualification for DrillingCert {}

    /// The production path for labour: drawn time is accounted for at the
    /// boundary by the execution history (R16, F-035), with no test-support
    /// fixture involved (R12).
    #[test]
    fn labour_has_a_production_legal_sink() {
        let person = new_person::<5000>();
        let (labour, person) = draw_time::<2000, 3000, 5000>(person);
        let history = record(new_history(), "draw_time", labour);
        assert_eq!(history.event_count(), 1);
        let _execution_record_stays_with_the_caller = history;
        let _person_keeps_3000_ms = person;
    }

    /// The budget is drawn down across several calls; the modeller restates
    /// the running balance and the compiler checks every step (R15, F-030).
    #[test]
    fn budget_draws_down_across_calls() {
        let person = new_person::<10_000>();
        let (l1, person) = draw_time::<2000, 8000, 10_000>(person);
        let (l2, person) = draw_time::<3000, 5000, 8000>(person);
        let (l3, person) = draw_time::<5000, 0, 5000>(person); // fully spent
        assert_eq!(Person::<0>::BUDGET_MS, 0); // Person<0> is the spent state
        // Account for the labour (this module's own tests may defuse: they
        // sit inside the privacy boundary).
        assert_eq!((l1.defuse(), l2.defuse(), l3.defuse()), ((), (), ()));
        let _spent_person_stays_with_the_caller = person;
    }

    /// Qualification conserves the person (R18): qualify wraps, release
    /// unwraps the same person, budget intact — and the wrapper exposes the
    /// inner budget like `Person` does.
    #[test]
    fn qualify_and_release_conserve_the_person() {
        let person = new_person::<5000>();
        let operator = qualify::<DrillingCert, 5000>(person);
        assert_eq!(Qualified::<DrillingCert, 5000>::BUDGET_MS, 5000);
        let person = release(operator);
        assert_eq!(Person::<5000>::BUDGET_MS, 5000);
        let _person_stays_with_the_caller = person;
    }

    /// The delegating draw inherits the whole R15/R16 machinery (F-043): the
    /// budget draws down through the wrapper, the labour comes out as
    /// model-core's own `Labour` and reaches its production sink, and the
    /// qualification survives the draw.
    #[test]
    fn qualified_budget_draws_down_and_labour_reaches_history() {
        let operator = qualify::<DrillingCert, 5000>(new_person::<5000>());
        let (labour, operator) = qualified_draw_time::<2000, 3000, 5000, _>(operator);
        let history = record(new_history(), "qualified_draw_time", labour);
        assert_eq!(history.event_count(), 1);
        // Still qualified, at the reduced budget.
        let person = release::<DrillingCert, 3000>(operator);
        let _execution_record = history;
        let _person_keeps_3000_ms = person;
    }

    /// Reusable resources are created at the boundary and stay with the
    /// caller; no tripwire fires when they go out of scope (R2).
    #[test]
    fn common_reusables_outlive_the_flow_quietly() {
        let _org = new_organisation();
        let _loc = new_location();
        let _person = new_person::<1000>();
    }
}
