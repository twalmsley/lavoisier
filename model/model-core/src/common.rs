//! Reusable common resource types (R11): [`Person`], [`Organisation`],
//! [`Location`], and the conserved [`Labour`] output of spending a person's
//! time.
//!
//! Layout per F-006/F-031: this module is the resource family — the sealed
//! types, with the creation [`boundary`] and the continuous [`processes`]
//! (which mint quantity-bearing values and therefore must live inside the
//! privacy boundary) as child modules. All three common types are created
//! only via [`boundary`] (production, R12) or the `test_fixture`
//! constructors (`test-support` feature, F-004).
//!
//! Adapted from `experiments/exp09-continuous-resources/src/lib.rs`
//! (`Person`/`Labour`/`draw_time`).

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
    /// or by-product it must eventually reach a `Consumer` (e.g. a ledger at
    /// the system boundary).
    Labour,
    unit = "person-milliseconds",
    must_use = "Labour is a conserved resource: it must be accounted for by a Consumer (e.g. a ledger)"
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

/// The creation boundary for the common types (R12): the only production
/// code allowed to create them.
pub mod boundary {
    use super::{Location, Organisation, Person};

    /// A person enters the model with a time budget (R15: model a budget
    /// only where the time is genuinely being accounted for).
    ///
    /// Placeholder: workforce — one person with a fixed shift budget.
    pub fn new_person<const BUDGET_MS: u64>() -> Person<BUDGET_MS> {
        Person { _seal: () }
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
    use super::{Labour, Person};

    /// Draws `SPEND` person-milliseconds from a person's time budget,
    /// leaving `LEFT` (R15 time-budget pattern; caller-stated remainder,
    /// F-022/F-030). The expended time leaves as a conserved [`Labour`]
    /// value that must reach a `Consumer`.
    ///
    /// ```
    /// use model_core::boundary::send_to;
    /// use model_core::common::boundary::new_person;
    /// use model_core::common::processes::draw_time;
    /// use model_core::fixtures::new_test_sink; // test-support feature
    ///
    /// let person = new_person::<10_000>();
    /// let (labour, person) = draw_time::<2000, 8000, 10_000>(person);
    /// let _ledger = send_to(new_test_sink(), labour);
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
}

#[cfg(test)]
mod tests {
    use super::boundary::{new_location, new_organisation, new_person};
    use super::processes::draw_time;
    use super::Person;

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

    /// Reusable resources are created at the boundary and stay with the
    /// caller; no tripwire fires when they go out of scope (R2).
    #[test]
    fn common_reusables_outlive_the_flow_quietly() {
        let _org = new_organisation();
        let _loc = new_location();
        let _person = new_person::<1000>();
    }
}
