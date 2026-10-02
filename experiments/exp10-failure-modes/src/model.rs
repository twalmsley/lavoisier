//! The drilling model's sealed resource family (R1), including the
//! **failure-state resources** and the **outcome token**, with the creation
//! boundary (R12) and the processes (F-031) as child modules.
//!
//! One type per processing state (R9, F-023) extends naturally to failure
//! states: a [`ScrapPlate`] is not a [`DrilledPlate`], so a flow that tries to
//! ship a scrapped plate is a compile error; a [`BrokenDrillBit`] is not a
//! [`DrillBit`], so a flow that tries to drill with a broken bit is a compile
//! error. Failure outputs are ordinary conserved outputs: scrap goes to the
//! [`ScrapYard`], a broken bit is either repaired
//! ([`processes::repair_bit`]) or scrapped — never a dead end (F-035).

use model_core::boundary::Consumer;
use model_core::common::{Labour, Person};

// ---------------------------------------------------------------------------
// Success-state and failure-state resources (R9 "one type per state").
// ---------------------------------------------------------------------------

model_core::container_resource! {
    /// A cut, undrilled plate blank of `V` grams: the input state.
    Plate,
    unit = "grams",
    must_use = "Plate is a conserved resource: pass it on or hand it to a Consumer"
}

model_core::container_resource! {
    /// A plate of `V` grams with its holes drilled: the **success state** of
    /// drilling (R9) — a distinct type from the blank and from scrap.
    DrilledPlate,
    unit = "grams",
    must_use = "DrilledPlate is a conserved resource: pass it on or hand it to a Consumer"
}

model_core::container_resource! {
    /// A ruined plate of `V` grams: the **failure state** of drilling (R9,
    /// candidate R17). A scrapped plate is an output, not a disappearance
    /// (R1): its mass is conserved and it must reach a scrap consumer
    /// ([`ScrapYard`]). No downstream process accepts it where a
    /// [`DrilledPlate`] is required — continuing the success flow with scrap
    /// is a compile error.
    ScrapPlate,
    unit = "grams",
    must_use = "ScrapPlate is a conserved failure output: hand it to a scrap consumer"
}

model_core::container_resource! {
    /// Swarf: `V` grams of metal chips. Both branches of a fallible drilling
    /// step produce it (the failure branch gouges more).
    Swarf,
    unit = "grams",
    must_use = "Swarf is a conserved waste product: hand it to a scrap consumer"
}

model_core::reusable_resource! {
    /// A working drill bit (R2): moved into every drilling step and normally
    /// returned. On a failed step it does **not** come back — the failure
    /// bundle carries a [`BrokenDrillBit`] instead (one type per state, R9).
    DrillBit,
    must_use = "DrillBit is a reusable resource: pass it on or return it to the caller"
}

model_core::consumable_resource! {
    /// A snapped drill bit: the **failure state** of the bit (R9, candidate
    /// R17). Tripwired (F-008): a broken bit that never reaches repair
    /// ([`processes::repair_bit`]) or the [`ScrapYard`] fails the test that
    /// leaked it. A failure output is accounted for, not a dead end.
    BrokenDrillBit,
    must_use = "BrokenDrillBit is a conserved failure output: repair it or hand it to a scrap consumer"
}

impl DrillBit {
    /// The conserving working→broken state conversion (R9's `pub(crate)`
    /// per-step conversion idiom): the bit is consumed by its own failure,
    /// not lost. Crate-private: only processes may realise a failure.
    pub(crate) fn snap(self) -> BrokenDrillBit {
        let DrillBit { _seal: () } = self;
        BrokenDrillBit::mint()
    }
}

model_core::consumable_resource! {
    /// A drill-bit repair kit (spare cutter and collet), consumed by
    /// [`processes::repair_bit`]. Tripwired (F-008); its production-legal
    /// sinks are the repair process and [`ToolStores`] (unused kits go back
    /// to stores — F-035: whoever mints a tripwired type must ship a
    /// production consumer for it).
    SpareParts,
    must_use = "SpareParts is a conserved resource: use it in a repair or return it to the tool stores"
}

// ---------------------------------------------------------------------------
// The outcome token (candidate R17): variability enters only at the boundary.
// ---------------------------------------------------------------------------

/// The private outcome value inside a [`DrillOutcome`]. A value-level record
/// (like R16's `Event`), not a resource: `Copy` is fine here because the
/// *token* is the conserved thing, and the token is sealed and tripwired.
#[derive(Clone, Copy)]
pub(crate) enum OutcomeKind {
    /// The drilling attempt will succeed.
    Success,
    /// The bit will snap and ruin the plate.
    Failure,
}

/// One trial of the environment: a sealed token deciding whether a single
/// drilling attempt succeeds or fails (candidate R17).
///
/// The token is how **variability enters the model without making processes
/// nondeterministic**: [`processes::drill_fallible`] is a pure function of
/// its inputs — the token included — and the *value* inside the token is
/// injected only at the system boundary ([`boundary::outcome_success`] /
/// [`boundary::outcome_failure`], both placeholders) or by test-support
/// fixtures. A flow holding a `DrillOutcome` cannot read it (the kind is
/// private): the only way to learn the outcome is to run the process and
/// handle the `Result`, so both arms must be written.
///
/// Sealed per the R1 rules: the kind is a **private enum wrapped in a sealed
/// struct** (never a `pub enum` resource, F-005), no public constructor, no
/// `Clone`/`Copy`. Tripwired (F-008): a provisioned trial that is neither run
/// nor returned to the environment ([`boundary::return_outcome`]) fails the
/// test that leaked it. Exactly one process consumes it (R1).
#[must_use = "DrillOutcome is a boundary token: run it through exactly one fallible process or return it to the environment"]
pub struct DrillOutcome {
    kind: OutcomeKind,
    _seal: (),
}

impl DrillOutcome {
    /// Mints one token. Reachable only from this module tree: the boundary
    /// (R12) and the processes are the only production code that can create
    /// or read an outcome.
    fn mint(kind: OutcomeKind) -> Self {
        DrillOutcome { kind, _seal: () }
    }

    /// Defuses the tripwire and lets the token go — the single allowed forget
    /// site for this resource (R1, F-008), called only by the consuming
    /// process and the boundary return.
    fn defuse(self) {
        // The one sanctioned mem::forget for this type (F-008).
        #[allow(clippy::mem_forget)]
        core::mem::forget(self);
    }

    /// Consumes the token and reveals its kind — callable only inside this
    /// module tree, i.e. only by a process that is about to realise the
    /// outcome. (A type with `Drop` cannot be destructured, F-032, hence
    /// read-then-defuse.)
    pub(crate) fn consume_kind(self) -> OutcomeKind {
        let kind = self.kind;
        self.defuse();
        kind
    }

    /// Test fixture: a success token from nowhere (R1, F-004;
    /// `test-support` feature, dev-dependencies only).
    #[cfg(feature = "test-support")]
    pub fn test_success() -> Self {
        DrillOutcome::mint(OutcomeKind::Success)
    }

    /// Test fixture: a failure token from nowhere (R1, F-004).
    #[cfg(feature = "test-support")]
    pub fn test_failure() -> Self {
        DrillOutcome::mint(OutcomeKind::Failure)
    }
}

impl Drop for DrillOutcome {
    /// Conservation tripwire (R1 layer 2, F-008): a provisioned trial that is
    /// dropped without being run (or returned to the environment) is a leak.
    /// Stands down while the thread is already panicking.
    fn drop(&mut self) {
        if !std::thread::panicking() {
            panic!(
                "resource leak: DrillOutcome dropped without being consumed (R1 conservation)"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// The outcome bundles (candidate R17): one grouping struct per arm.
// ---------------------------------------------------------------------------

/// Everything a successful drilling step produces (candidate R17): the
/// success-state product plus every conserved by-product, the reusable
/// resources included (the person and the bit come back).
///
/// A bundle is a **grouping** (R1 "possibly grouped into a struct"), not a
/// sealed resource: its fields are public so the handling arm can
/// destructure it, and building one requires already *holding* the sealed
/// resources, so it cannot mint anything. It deliberately has **no `Debug`
/// impl** (sealed resources have none), which makes `Result::unwrap()` and
/// `expect()` on the process result a compile error — see
/// `tests/ui/unwrap_needs_debug.rs`.
#[must_use = "DrillOk bundles conserved outputs: every field must be accounted for"]
pub struct DrillOk<const T_LEFT: u64, const P_LEFT: u64, const SW: u64, const SPEND: u64> {
    /// The person, back with the remaining budget (returned in both arms).
    pub person: Person<T_LEFT>,
    /// The bit survived and comes back (success state).
    pub bit: DrillBit,
    /// The product: the drilled plate.
    pub plate: DrilledPlate<P_LEFT>,
    /// Waste: chips removed by drilling.
    pub swarf: Swarf<SW>,
    /// The expended time, conserved (R15); record it into a `History` (R16).
    pub labour: Labour<SPEND>,
}

/// Everything a failed drilling step produces (candidate R17): the failure
/// states plus every conserved by-product. **Failure conserves too**: the
/// person comes back (with the time genuinely spent), the bit comes back
/// broken, the plate comes back as scrap plus the swarf gouged before the
/// snap — nothing disappears.
///
/// Like [`DrillOk`] it is a grouping with public fields and no `Debug`.
#[must_use = "DrillFail bundles conserved failure outputs: every field must be accounted for"]
pub struct DrillFail<const T_LEFT: u64, const SCRAP: u64, const FSW: u64, const SPEND: u64> {
    /// The person, back with the remaining budget — failure still spent their
    /// time (returned in both arms).
    pub person: Person<T_LEFT>,
    /// The bit, back in its failure state (one type per state, R9).
    pub broken_bit: BrokenDrillBit,
    /// The ruined plate: a conserved failure output, not a disappearance.
    pub scrap: ScrapPlate<SCRAP>,
    /// Waste: chips gouged before the snap.
    pub swarf: Swarf<FSW>,
    /// The expended time — failure costs time too (R15).
    pub labour: Labour<SPEND>,
}

// ---------------------------------------------------------------------------
// Boundary sinks (R12, R15): where products, scrap and reserves leave.
// ---------------------------------------------------------------------------

/// The scrap-metal stream at the system boundary: an unbounded sink
/// (`type Next = Self`, legal only at the boundary, R15/F-029) for every
/// metallic failure output and waste: scrap plates, swarf, and drill bits
/// broken beyond repair.
///
/// Placeholder: scrap-metal merchant — assumed able to take any amount.
#[must_use = "ScrapYard is a boundary resource: pass it on like any other resource"]
pub struct ScrapYard {
    _seal: (),
}

/// The yard accepts ruined plates of any mass; `Next = Self` (R15).
impl<const G: u64> Consumer<ScrapPlate<G>> for ScrapYard {
    type Next = ScrapYard;
    fn consume(self, item: ScrapPlate<G>) -> ScrapYard {
        item.defuse();
        self
    }
}

/// The yard accepts swarf of any mass; `Next = Self` (R15).
impl<const G: u64> Consumer<Swarf<G>> for ScrapYard {
    type Next = ScrapYard;
    fn consume(self, item: Swarf<G>) -> ScrapYard {
        item.defuse();
        self
    }
}

/// The yard accepts bits broken beyond repair; `Next = Self` (R15). Together
/// with [`processes::repair_bit`] this gives [`BrokenDrillBit`] two accounted
/// exits — a failure output is never a dead end (F-035).
impl Consumer<BrokenDrillBit> for ScrapYard {
    type Next = ScrapYard;
    fn consume(self, item: BrokenDrillBit) -> ScrapYard {
        item.defuse();
        self
    }
}

/// The customer taking delivery of drilled plates: an unbounded boundary sink
/// (R15, F-029). It accepts **only** the success state — handing it a
/// [`ScrapPlate`] is a compile error with model-core's modeller-phrased
/// `Consumer` message (F-015), pinned by `tests/ui/scrap_cannot_be_shipped.rs`.
///
/// Placeholder: customer — assumed able to take any number of plates.
#[must_use = "Customer is a boundary resource: pass it on like any other resource"]
pub struct Customer {
    _seal: (),
}

/// The customer accepts drilled plates of any mass; `Next = Self` (R15).
impl<const G: u64> Consumer<DrilledPlate<G>> for Customer {
    type Next = Customer;
    fn consume(self, item: DrilledPlate<G>) -> Customer {
        item.defuse();
        self
    }
}

/// The tool stores at the system boundary: the unbounded sink where
/// **unused rework reserves** go back when a bounded-retry flow succeeds
/// early — spare parts and reserve plate blanks (F-035: every tripwired type
/// this crate mints has a production-legal consumer).
///
/// Placeholder: tool stores — assumed able to take back any reserve.
#[must_use = "ToolStores is a boundary resource: pass it on like any other resource"]
pub struct ToolStores {
    _seal: (),
}

/// Stores take back an unused repair kit; `Next = Self` (R15).
impl Consumer<SpareParts> for ToolStores {
    type Next = ToolStores;
    fn consume(self, item: SpareParts) -> ToolStores {
        item.defuse();
        self
    }
}

/// Stores take back an unused reserve plate blank; `Next = Self` (R15).
impl<const G: u64> Consumer<Plate<G>> for ToolStores {
    type Next = ToolStores;
    fn consume(self, item: Plate<G>) -> ToolStores {
        item.defuse();
        self
    }
}

// ---------------------------------------------------------------------------
// The creation boundary (R12).
// ---------------------------------------------------------------------------

/// The creation and exit boundary of the family (R12, F-006): the only
/// production code where plates, bits, kits, outcome tokens and the sinks
/// come into existence, and where an untried outcome token leaves the model.
pub mod boundary {
    use super::{
        Customer, DrillBit, DrillOutcome, OutcomeKind, Plate, ScrapYard, SpareParts, ToolStores,
    };

    /// A plate blank of `GRAMS` grams enters the model (R12).
    ///
    /// Placeholder: plate stockholder.
    pub fn supply_plate<const GRAMS: u64>() -> Plate<GRAMS> {
        Plate::mint()
    }

    /// A working drill bit enters the model (R12).
    ///
    /// Placeholder: tool stores — one bit.
    pub fn supply_drill_bit() -> DrillBit {
        DrillBit::mint()
    }

    /// A repair kit enters the model (R12).
    ///
    /// Placeholder: tool stores — one spare cutter and collet.
    pub fn supply_spare_parts() -> SpareParts {
        SpareParts::mint()
    }

    /// A trial that will succeed enters the model (candidate R17): the
    /// environment's variability, injected at the boundary so processes stay
    /// deterministic. A flow cannot read the token; it must run the process
    /// and handle both arms of the `Result`.
    ///
    /// Placeholder: the environment — refine to a calibrated failure source
    /// (open question 4, validation) or a test fixture.
    pub fn outcome_success() -> DrillOutcome {
        DrillOutcome::mint(OutcomeKind::Success)
    }

    /// A trial that will fail enters the model (candidate R17).
    ///
    /// Placeholder: the environment — see [`outcome_success`].
    pub fn outcome_failure() -> DrillOutcome {
        DrillOutcome::mint(OutcomeKind::Failure)
    }

    /// An untried outcome token returns to the environment (R12 exit): the
    /// accounted path for a provisioned retry trial a flow never needed.
    pub fn return_outcome(outcome: DrillOutcome) {
        outcome.defuse();
    }

    /// The scrap-metal stream enters the model (R12). An empty unbounded
    /// sink holds nothing, so this is an ordinary public boundary function.
    pub fn new_scrap_yard() -> ScrapYard {
        ScrapYard { _seal: () }
    }

    /// The customer enters the model (R12).
    pub fn new_customer() -> Customer {
        Customer { _seal: () }
    }

    /// The tool stores enter the model (R12).
    pub fn new_tool_stores() -> ToolStores {
        ToolStores { _seal: () }
    }
}

// ---------------------------------------------------------------------------
// Processes (R1, F-031).
// ---------------------------------------------------------------------------

/// The family's processes (R1): pure by-value transformations. They mint
/// quantity-bearing values and state conversions, so they live inside the
/// resource family's module (F-031).
pub mod processes {
    use super::{
        BrokenDrillBit, DrillBit, DrillFail, DrillOk, DrillOutcome, DrilledPlate, OutcomeKind,
        Plate, ScrapPlate, SpareParts, Swarf,
    };
    use model_core::common::Person;
    use model_core::common::processes::draw_time;

    /// Drills one plate, **fallibly** (candidate R17): returns `Ok` with the
    /// success bundle or `Err` with the failure bundle, and **both arms
    /// conserve the same inputs** — the person comes back in both arms with
    /// the time genuinely spent, the plate's mass continues either as
    /// drilled-plate-plus-swarf or as scrap-plus-swarf, and the bit comes
    /// back working or broken (one type per state, R9).
    ///
    /// Per-branch conservation is **two independent compile-time asserts**,
    /// and both fire at every instantiation (R3, F-001/F-033): the caller
    /// states the success split (`P_LEFT + SW == PLATE`) *and* the failure
    /// split (`SCRAP + FSW == PLATE`) even though only one branch happens at
    /// run time — the model proves both arms conserve for every call site.
    /// The cost is const-parameter load: eight parameters, all stated by the
    /// caller (F-022/F-030).
    ///
    /// The outcome token is consumed (exactly one trial per attempt, R1);
    /// the `Result` is `#[must_use]` (std) on top of the bundles' own
    /// `must_use`, and the bundles implement no `Debug`, so `.unwrap()` /
    /// `.expect()` do not compile — the flow must `match`.
    ///
    /// Success-branch conservation violation — drilling cannot *add* mass
    /// (440 + 20 ≠ 450); an E0080 at monomorphization (F-001), invisible to
    /// `cargo check`, caught by `cargo build`/`cargo test`:
    ///
    /// ```compile_fail
    /// use exp10_failure_modes::model::boundary::{outcome_success, supply_drill_bit, supply_plate};
    /// use exp10_failure_modes::model::processes::drill_fallible;
    /// use model_core::common::boundary::new_person;
    ///
    /// let person = new_person::<5000>();
    /// let r = drill_fallible::<1000, 4000, 5000, 450, 440, 20, 430, 20>(
    ///     person,
    ///     supply_drill_bit(),
    ///     supply_plate::<450>(),
    ///     outcome_success(),
    /// );
    /// ```
    ///
    /// Failure-branch conservation violation — scrapping cannot *lose* mass
    /// (400 + 20 ≠ 450), rejected even when the flow only ever runs the
    /// success branch:
    ///
    /// ```compile_fail
    /// use exp10_failure_modes::model::boundary::{outcome_success, supply_drill_bit, supply_plate};
    /// use exp10_failure_modes::model::processes::drill_fallible;
    /// use model_core::common::boundary::new_person;
    ///
    /// let person = new_person::<5000>();
    /// let r = drill_fallible::<1000, 4000, 5000, 450, 440, 10, 400, 20>(
    ///     person,
    ///     supply_drill_bit(),
    ///     supply_plate::<450>(),
    ///     outcome_success(),
    /// );
    /// ```
    pub fn drill_fallible<
        const SPEND: u64,
        const T_LEFT: u64,
        const BUDGET: u64,
        const PLATE: u64,
        const P_LEFT: u64,
        const SW: u64,
        const SCRAP: u64,
        const FSW: u64,
    >(
        person: Person<BUDGET>,
        bit: DrillBit,
        plate: Plate<PLATE>,
        outcome: DrillOutcome,
    ) -> Result<DrillOk<T_LEFT, P_LEFT, SW, SPEND>, DrillFail<T_LEFT, SCRAP, FSW, SPEND>> {
        const {
            assert!(
                P_LEFT + SW == PLATE,
                "success-branch mass conservation violated in drill_fallible (R3, candidate R17): the drilled plate plus its swarf must sum exactly to the plate blank"
            )
        };
        const {
            assert!(
                SCRAP + FSW == PLATE,
                "failure-branch mass conservation violated in drill_fallible (R3, candidate R17): the scrap plate plus its swarf must sum exactly to the plate blank"
            )
        };
        // Both arms spend the time (R15): failure costs labour too.
        let (labour, person) = draw_time::<SPEND, T_LEFT, BUDGET>(person);
        // Conserving transform: the blank's mass continues in whichever
        // branch is realised.
        plate.defuse();
        match outcome.consume_kind() {
            OutcomeKind::Success => Ok(DrillOk {
                person,
                bit,
                plate: DrilledPlate::mint(),
                swarf: Swarf::mint(),
                labour,
            }),
            OutcomeKind::Failure => {
                // The bit is consumed by its own failure: working -> broken
                // is a conserving state conversion (R9), not a loss.
                Err(DrillFail {
                    person,
                    broken_bit: bit.snap(),
                    scrap: ScrapPlate::mint(),
                    swarf: Swarf::mint(),
                    labour,
                })
            }
        }
    }

    /// Repairs a broken bit with a repair kit (candidate R17): the failure
    /// path back to the success state. A conserving state conversion (R9):
    /// the broken bit and the kit are consumed into the working bit (the
    /// snapped cutter is assumed replaced in place; discrete objects, R13).
    pub fn repair_bit(broken: BrokenDrillBit, parts: SpareParts) -> DrillBit {
        broken.defuse();
        parts.defuse();
        DrillBit::mint()
    }
}

#[cfg(test)]
mod tests {
    //! Per-process unit tests (R5): **every branch of every process**,
    //! especially the failure arm — branch coverage is leak coverage (F-002).
    //! These tests sit inside the privacy boundary, so they may mint fixtures
    //! and defuse outputs directly; downstream-style accounting is exercised
    //! by the integration tests in `tests/`.

    use super::boundary::{outcome_failure, outcome_success, return_outcome, supply_drill_bit, supply_plate, supply_spare_parts};
    use super::processes::{drill_fallible, repair_bit};
    use super::{DrillFail, DrillOk, DrilledPlate, ScrapPlate, Swarf};
    use model_core::common::boundary::new_person;
    use model_core::common::{Labour, Person};
    use model_core::history::boundary::new_history;
    use model_core::history::processes::record;

    /// The success arm conserves: mass, time and the reusables all come back
    /// accounted (R3, R15; candidate R17).
    #[test]
    fn success_arm_conserves_mass_and_time() {
        let person = new_person::<5000>();
        let r = drill_fallible::<1000, 4000, 5000, 450, 440, 10, 430, 20>(
            person,
            supply_drill_bit(),
            supply_plate::<450>(),
            outcome_success(),
        );
        match r {
            Ok(DrillOk {
                person,
                bit,
                plate,
                swarf,
                labour,
            }) => {
                assert_eq!(DrilledPlate::<440>::VALUE + Swarf::<10>::VALUE, 450);
                assert_eq!(Person::<4000>::BUDGET_MS + Labour::<1000>::VALUE, 5000);
                plate.defuse();
                swarf.defuse();
                // Labour is model-core's resource: this crate cannot defuse
                // it, only hand it to its production sink (R16, F-035).
                let history = record(new_history(), "drill_fallible", labour);
                assert_eq!(history.event_count(), 1);
                let _execution_record = history;
                let _reusables = (person, bit);
            }
            Err(_) => unreachable!("a success token cannot produce the failure arm"),
        }
    }

    /// The failure arm conserves the **same inputs** (candidate R17): the
    /// person comes back with the time genuinely spent, the plate's whole
    /// mass continues as scrap + swarf, and the bit comes back broken.
    #[test]
    fn failure_arm_conserves_the_same_inputs() {
        let person = new_person::<5000>();
        let r = drill_fallible::<1000, 4000, 5000, 450, 440, 10, 430, 20>(
            person,
            supply_drill_bit(),
            supply_plate::<450>(),
            outcome_failure(),
        );
        match r {
            Ok(_) => unreachable!("a failure token cannot produce the success arm"),
            Err(DrillFail {
                person,
                broken_bit,
                scrap,
                swarf,
                labour,
            }) => {
                assert_eq!(ScrapPlate::<430>::VALUE + Swarf::<20>::VALUE, 450);
                assert_eq!(Person::<4000>::BUDGET_MS + Labour::<1000>::VALUE, 5000);
                scrap.defuse();
                swarf.defuse();
                broken_bit.defuse();
                // Failure costs time too, and it is recorded like any other
                // consumption (R16).
                let history = record(new_history(), "drill_fallible", labour);
                assert_eq!(history.event_count(), 1);
                let _execution_record = history;
                let _person_stays = person;
            }
        }
    }

    /// Repair converts the failure state back to the working state (R9),
    /// consuming the kit.
    #[test]
    fn repair_restores_the_working_state() {
        let person = new_person::<5000>();
        let r = drill_fallible::<1000, 4000, 5000, 450, 440, 10, 430, 20>(
            person,
            supply_drill_bit(),
            supply_plate::<450>(),
            outcome_failure(),
        );
        match r {
            Ok(_) => unreachable!("a failure token cannot produce the success arm"),
            Err(fail) => {
                let bit = repair_bit(fail.broken_bit, supply_spare_parts());
                fail.scrap.defuse();
                fail.swarf.defuse();
                let _execution_record = record(new_history(), "drill_fallible", fail.labour);
                let _reusables = (fail.person, bit);
            }
        }
    }

    /// An untried token has an accounted exit at the boundary (R12): the
    /// tripwire stays quiet.
    #[test]
    fn untried_outcome_returns_to_the_environment() {
        let token = outcome_success();
        return_outcome(token);
    }

    /// An abandoned token is caught by its tripwire at test time (R1 layer 2,
    /// F-008): a provisioned trial must be run or returned, never dropped.
    #[test]
    #[should_panic(expected = "resource leak: DrillOutcome dropped without being consumed")]
    fn abandoned_outcome_trips_the_tripwire() {
        let token = outcome_failure();
        let _still_bound_but_never_run = token;
    }
}
