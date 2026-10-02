//! The workshop's sealed resource family (R1), its creation/exit boundary
//! (R12) and its processes (F-031).
//!
//! Layout per F-006/F-031: this module holds the sealed resource types, with
//! the [`boundary`] (suppliers, bin/customer constructors, waste disposal)
//! and the [`processes`] (which mint quantity-bearing values and therefore
//! live inside the privacy boundary) as child modules.
//!
//! One type per processing state (R9, F-023): a [`Plate`] is not a
//! [`DrilledPlate`], so a flow that fastens an undrilled plate is a compile
//! error reading "expected `DrilledPlate`, found `Plate`" — these plates have
//! not been drilled yet. The rule extends to **failure states** (R17): a
//! [`ScrapPlate`] is not a [`DrilledPlate`] and a [`BrokenDrillBit`] is not a
//! [`DrillBit`], so failure outputs cannot continue the success flow — and to
//! **safety states** (R18): only the [`FittedGuard`] carries the `Fitted`
//! characteristic REQ-005 requires, never the unfitted [`MachineGuard`].
//!
//! Every consumable here carries the kernel's tripwire `Drop` (R1 layer 2,
//! F-008): swarf, plates, sheets or assemblies that never reach a consumer
//! fail the test that leaked them. Reusable resources ([`Drill`], and
//! model-core's `Person` with its R15 time budget) are moved in and returned
//! by every process (R2) and stay with the caller.

use crate::catalogue::FourOf;
use crate::characteristics::DrillingCert;
// One line on purpose: the traceability grep (F-021) skips `use` lines, but
// only when the line itself starts with `use`.
use crate::requirements::{Req004CertifiedDrillingOperator, Req005FittedDrillGuard, assert_req002, assert_req003, assert_req004, assert_req005};
use core::marker::PhantomData;
use model_core::boundary::Consumer;
use model_core::common::Qualified;
use model_core::list::{Cons, Len, Nil};
use model_core::nat::{Succ, Zero};

model_core::container_resource! {
    /// Sheet steel stock, in grams (R7 base mass unit): the raw material the
    /// workshop cuts plates from. Enters the model only through
    /// `boundary::supply_sheet` (R12).
    SteelSheet,
    unit = "grams",
    must_use = "SteelSheet is a conserved resource: pass it on or hand it to a Consumer"
}

model_core::container_resource! {
    /// A cut, undrilled plate blank of `V` grams. Its own processing state
    /// (R9): no process accepts it where a drilled plate is required.
    Plate,
    unit = "grams",
    must_use = "Plate is a conserved resource: pass it on or hand it to a Consumer"
}

model_core::container_resource! {
    /// A plate of `V` grams with its bolt holes drilled — a distinct type
    /// from the undrilled blank (R9, F-023), which is what makes "fasten
    /// before drilling" a compile error.
    DrilledPlate,
    unit = "grams",
    must_use = "DrilledPlate is a conserved resource: pass it on or hand it to a Consumer"
}

// The drilled state carries the drilled characteristic (R6); through the
// blanket impl in `requirements` this is what satisfies REQ-002, and the
// assertion keeps the claim compile-checked (F-020; the greppable tag lives
// on the `fasten` process below).
impl<const G: u64> crate::characteristics::Drilled for DrilledPlate<G> {}
model_core::satisfies!(assert_req002, DrilledPlate<1>);

model_core::container_resource! {
    /// Swarf: `V` grams of metal chips from cutting or drilling. Tripwired
    /// waste (R1 layer 2, F-008): swarf that never reaches a swarf consumer
    /// panics the test that leaked it.
    Swarf,
    unit = "grams",
    must_use = "Swarf is a conserved waste product: hand it to a swarf waste consumer"
}

model_core::reusable_resource! {
    /// The workshop's pillar drill (R2): moved into every drilling step and
    /// returned to the caller, so it can be used by only one process at a
    /// time. No tripwire — a reusable resource legitimately outlives the
    /// flow.
    Drill,
    must_use = "Drill is a reusable resource: pass it on or return it to the caller"
}

// ---------------------------------------------------------------------------
// Failure states (R17), safety states (R18) and the outcome token (R17).
// ---------------------------------------------------------------------------

model_core::container_resource! {
    /// A ruined plate of `V` grams: the **failure state** of drilling (R17,
    /// extending R9's one-type-per-state rule). A scrapped plate is an
    /// output, not a disappearance (R1): its mass is conserved and it must
    /// reach the [`ScrapYard`]. No downstream process accepts it where a
    /// [`DrilledPlate`] is required — continuing the success flow with scrap
    /// is a compile error.
    ScrapPlate,
    unit = "grams",
    must_use = "ScrapPlate is a conserved failure output: hand it to the scrap yard"
}

model_core::reusable_resource! {
    /// A working drill bit for the pillar drill (R2): moved into every
    /// fallible drilling step and normally returned. On a failed step it does
    /// **not** come back — the failure bundle carries a [`BrokenDrillBit`]
    /// instead (one type per state, R9/R17).
    DrillBit,
    must_use = "DrillBit is a reusable resource: pass it on or return it to the caller"
}

model_core::consumable_resource! {
    /// A snapped drill bit: the **failure state** of the bit (R17). Tripwired
    /// (F-008): a broken bit that never reaches repair
    /// ([`processes::repair_bit`]) or the [`ScrapYard`] fails the test that
    /// leaked it — a failure output is accounted for, never a dead end
    /// (F-035).
    BrokenDrillBit,
    must_use = "BrokenDrillBit is a conserved failure output: repair it or hand it to the scrap yard"
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

model_core::reusable_resource! {
    /// The pillar drill's machine guard **not fitted to the drill** — the
    /// unsafe state (R18). Reusable (R2); one type per state (R9): only
    /// [`FittedGuard`] carries the `Fitted` characteristic, so an unfitted
    /// guard at a drilling process is a compile error, distinct from "no
    /// guard at all" (F-049).
    MachineGuard,
    must_use = "MachineGuard is a reusable resource: pass it on or return it to the caller"
}

model_core::reusable_resource! {
    /// The machine guard **fitted to the pillar drill** — the safe state
    /// (R18), reachable only through [`processes::fit_guard`] (fitting is a
    /// process, R9) and returned by every guarded process (R2).
    FittedGuard,
    must_use = "FittedGuard is a reusable resource: pass it on or return it to the caller"
}

// The safety characteristic lives on the safe state only (R9/F-023, R18).
impl crate::characteristics::Fitted for FittedGuard {}

/// The fitted guard under its requirement-facing name; the alias carries the
/// tag because a tag inside the `reusable_resource!` invocation above would
/// be silently dropped by trace.sh (F-037).
///
/// Satisfies: REQ-005
pub type DrillReadyGuard = FittedGuard;
model_core::satisfies!(assert_req005, DrillReadyGuard);

/// A drilling-certified operator with `BUDGET_MS` person-milliseconds left —
/// the readable name for model-core's `Qualified<DrillingCert, _>` (R18). The
/// alias carries the satisfaction tag (F-037), and `Qualified` is only
/// REQ-004-fit with the drilling qualification in its slot.
///
/// Satisfies: REQ-004
pub type DrillingOperator<const BUDGET_MS: u64> = Qualified<DrillingCert, BUDGET_MS>;
model_core::satisfies!(assert_req004, DrillingOperator<0>);

model_core::outcome_token! {
    /// One trial of the environment (R17): whether a single fallible drilling
    /// attempt succeeds (the bit survives, the plate is drilled) or fails
    /// (the bit snaps and ruins the plate). Consumed by exactly one process,
    /// [`processes::drill_holes_fallible`]; a flow cannot read it — it must
    /// run the process and handle both arms of the `Result`.
    DrillOutcome(DrillOutcomeKind),
    success = outcome_success,
    failure = outcome_failure,
    exit = return_outcome,
    must_use = "DrillOutcome is a boundary token: run it through exactly one fallible process or return it to the environment"
}

/// Everything a successful fallible drilling step produces (R17): the
/// success-state product plus every conserved by-product, the reusable
/// resources included (the operator, the guard and the bit come back).
///
/// A bundle is a **grouping** (R1 "possibly grouped into a struct"), not a
/// sealed resource (F-046): its fields are public so the handling arm can
/// destructure it, and building one requires already *holding* the sealed
/// resources, so it cannot mint anything. It deliberately has **no `Debug`
/// impl** (F-047), which makes `Result::unwrap()`/`expect()` on the process
/// result a compile error — see `tests/ui/unwrap_needs_debug.rs`.
#[must_use = "DrillOk bundles conserved outputs: every field must be accounted for"]
pub struct DrillOk<O: Req004CertifiedDrillingOperator, G: Req005FittedDrillGuard, const P_LEFT: u64, const SW: u64> {
    /// The certified operator, back unchanged (the time draw is an adjacent
    /// process in the flow, R17/F-048 — so the operator returns at the same
    /// budget in both arms).
    pub operator: O,
    /// The fitted guard, back (R2).
    pub guard: G,
    /// The bit survived and comes back working (success state).
    pub bit: DrillBit,
    /// The product: the drilled plate.
    pub plate: DrilledPlate<P_LEFT>,
    /// Waste: chips removed by drilling — REQ-003 applies to it like to any
    /// swarf.
    pub swarf: Swarf<SW>,
}

/// Everything a failed fallible drilling step produces (R17): the failure
/// states plus every conserved by-product. **Failure conserves too**: the
/// operator and guard come back, the bit comes back broken, the plate comes
/// back as scrap plus the swarf gouged before the snap — nothing disappears.
///
/// Like [`DrillOk`] it is a grouping with public fields and no `Debug`
/// (F-046, F-047).
#[must_use = "DrillFail bundles conserved failure outputs: every field must be accounted for"]
pub struct DrillFail<O: Req004CertifiedDrillingOperator, G: Req005FittedDrillGuard, const SCRAP: u64, const FSW: u64> {
    /// The certified operator, back unchanged (the time was drawn before the
    /// branch — failure cost it too, R17).
    pub operator: O,
    /// The fitted guard, back (R2).
    pub guard: G,
    /// The bit, back in its failure state (one type per state, R9/R17).
    pub broken_bit: BrokenDrillBit,
    /// The ruined plate: a conserved failure output, not a disappearance.
    pub scrap: ScrapPlate<SCRAP>,
    /// Waste: chips gouged before the snap (REQ-003 applies).
    pub swarf: Swarf<FSW>,
}

/// The scrap-metal stream at the system boundary: an unbounded sink
/// (`type Next = Self`, legal only at the boundary, R15/F-029) for the
/// drilling failure outputs — scrap plates and bits broken beyond repair.
/// Swarf does **not** go here: REQ-003 routes all swarf to the dedicated
/// [`SwarfBin`].
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
        item.defuse(); // sanctioned consumer role (F-008); unbounded sink discards (F-029)
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

/// The tool stores at the system boundary: the unbounded sink where **unused
/// rework reserves** go back when a bounded-retry flow succeeds early
/// (R17/F-050) — repair kits and reserve plate blanks. (F-035: every
/// tripwired type this crate mints has a production-legal consumer.)
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

model_core::consumable_resource! {
    /// The final product: two drilled plates fastened with four catalogue
    /// bolts. `PLATE_G` is the plates' combined mass in grams (conserved from
    /// the two inputs at compile time); the four bolts are conserved as
    /// discrete objects (R13), held inside the assembly as sealed payload —
    /// real owned objects (R12), untripwired, so the sanctioned forget in
    /// `defuse` skips no tripwire. Generated by the kernel's generic form
    /// with held contents (F-036); tripwired, defused only when a consumer
    /// such as [`Customer`] takes delivery.
    Assembly<B, const PLATE_G: u64> {
        bolts: FourOf<B>,
    },
    must_use = "Assembly is a conserved resource: pass it on or hand it to a Consumer"
}

impl<B, const PLATE_G: u64> Assembly<B, PLATE_G> {
    /// The combined mass of the two plates, in grams (R7).
    pub const PLATE_GRAMS: u64 = PLATE_G;
}

/// A bin for swarf at the system boundary (R12): the dedicated swarf waste
/// consumer that REQ-003 calls for. `Space` is the type-level number of
/// pieces it can still accept; `Contents` keeps the real swarf objects it has
/// consumed (F-016). The decreasing space parameter is a hard rule, not a
/// style choice: a contents-keeping consumer without one sends trait
/// resolution into unbounded exploration and, at this workspace's mandated
/// recursion limit, crashes the compiler (F-034).
///
/// Placeholder: generic swarf bin — refine to a named scrap-metal stream.
///
/// Satisfies: REQ-003
pub struct SwarfBin<Space, Contents = Nil> {
    contents: Contents,
    _space: PhantomData<Space>,
}

// Compile-checked backing for the tag above (R10, F-020).
model_core::satisfies!(assert_req003, SwarfBin<Zero>);

// The swarf-consumer characteristic (R6) holds in every state — REQ-003 is
// about which consumer swarf goes to; whether there is space left is the
// Consumer impl's business (R12).
impl<Space, C> crate::characteristics::SwarfConsumer for SwarfBin<Space, C> {}

/// A full bin: zero space left — a distinct resource type that must itself
/// be accounted for (R12, e.g. via [`boundary::dispose_bin`]).
pub type FullSwarfBin<Contents> = SwarfBin<Zero, Contents>;

/// `Consumer` is implemented ONLY while space remains (R12, F-034): space
/// goes down by one and the real swarf object is kept at the front of the
/// contents list (F-016). Consuming into a full bin is a compile error with
/// model-core's modeller-phrased message (F-015).
impl<S, C, const G: u64> Consumer<Swarf<G>> for SwarfBin<Succ<S>, C> {
    type Next = SwarfBin<S, Cons<Swarf<G>, C>>;
    fn consume(self, item: Swarf<G>) -> Self::Next {
        SwarfBin {
            contents: Cons(item, self.contents),
            _space: PhantomData,
        }
    }
}

impl<Space, Contents: Len> SwarfBin<Space, Contents> {
    /// How many pieces of swarf the bin holds — the length of its contents
    /// list, so count and contents cannot disagree (R7, R12).
    pub const HELD: u64 = Contents::LEN;
}

/// The customer taking delivery of finished assemblies: an unbounded boundary
/// sink (`type Next = Self`), legal only at the system boundary (R15, F-029).
/// Being unbounded, it necessarily discards what it consumes — the one
/// sanctioned exception to "a consumer keeps what it consumes" (R12).
///
/// Placeholder: customer — assumed able to take any number of assemblies.
#[must_use = "Customer is a boundary resource: pass it on like any other resource"]
pub struct Customer {
    _seal: (),
}

/// The customer accepts any assembly; `Next = Self` (R15, F-029).
impl<B, const PLATE_G: u64> Consumer<Assembly<B, PLATE_G>> for Customer {
    type Next = Customer;
    fn consume(self, item: Assembly<B, PLATE_G>) -> Customer {
        item.defuse();
        self
    }
}

/// The customer also takes delivery of loose drilled plates of any mass
/// (`Next = Self`, R15) — the production exit for the fallible drilling
/// flows' product (F-035). It accepts **only** the success state: handing it
/// a [`ScrapPlate`] is a compile error (one type per failure state, R17) —
/// and because the customer has several `Consumer` impls, that mistake takes
/// the trait-bound path and shows model-core's modeller-phrased message
/// (F-015).
impl<const G: u64> Consumer<DrilledPlate<G>> for Customer {
    type Next = Customer;
    fn consume(self, item: DrilledPlate<G>) -> Customer {
        item.defuse();
        self
    }
}

/// The creation and exit boundary of the workshop's resource family (R12,
/// F-006): the only production code where sheets, drills, bins and the
/// customer come into existence, and where full bins leave the model.
pub mod boundary {
    use super::{
        Customer, Drill, DrillBit, MachineGuard, PhantomData, Plate, ScrapYard, SpareParts,
        SteelSheet, Swarf, SwarfBin, ToolStores,
    };
    use model_core::list::{Cons, Nil};

    // The outcome token's boundary constructors and exit are generated by
    // `model_core::outcome_token!` at the family level; they are re-exported
    // here because boundary constructors live in the boundary module (R12).
    pub use super::{outcome_failure, outcome_success, return_outcome};

    /// A steel sheet of `GRAMS` grams enters the model (R12).
    ///
    /// Placeholder: steel stockholder — assumed able to deliver any sheet.
    pub fn supply_sheet<const GRAMS: u64>() -> SteelSheet<GRAMS> {
        SteelSheet::mint()
    }

    /// A drill enters the model (R12).
    ///
    /// Placeholder: tool store — one pillar drill.
    pub fn supply_drill() -> Drill {
        Drill::mint()
    }

    /// A plate blank of `GRAMS` grams enters the model directly (R12),
    /// bypassing the cutting step — the provisioning path for fallible
    /// drilling and its rework reserves (R17/F-050).
    ///
    /// Placeholder: plate stockholder — blanks bought in pre-cut.
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

    /// A machine guard enters the model, unfitted (R18).
    ///
    /// Placeholder: stores — one guard off the shelf.
    pub fn supply_guard() -> MachineGuard {
        MachineGuard::mint()
    }

    /// The scrap-metal stream enters the model (R12). An empty unbounded
    /// sink holds nothing, so this is an ordinary public boundary function.
    pub fn new_scrap_yard() -> ScrapYard {
        ScrapYard { _seal: () }
    }

    /// The tool stores enter the model (R12).
    pub fn new_tool_stores() -> ToolStores {
        ToolStores { _seal: () }
    }

    /// An empty swarf bin with `Space` slots. Creating an *empty* consumer
    /// brings no resources into existence, so this is an ordinary public
    /// boundary function (R12): `new_swarf_bin::<N3>()`.
    pub fn new_swarf_bin<Space>() -> SwarfBin<Space, Nil> {
        SwarfBin {
            contents: Nil,
            _space: PhantomData,
        }
    }

    /// The customer enters the model (R12). An empty unbounded sink holds
    /// nothing, so this too is an ordinary public boundary function.
    pub fn new_customer() -> Customer {
        Customer { _seal: () }
    }

    /// Recursively defuses a bin's kept swarf as the bin leaves the model.
    /// Private: together with [`dispose_bin`] this is the only exit for
    /// swarf (R1).
    trait Dispose {
        fn dispose(self);
    }
    impl Dispose for Nil {
        fn dispose(self) {}
    }
    impl<const G: u64, T: Dispose> Dispose for Cons<Swarf<G>, T> {
        fn dispose(self) {
            let Cons(swarf, tail) = self;
            swarf.defuse();
            tail.dispose();
        }
    }

    /// Public-in-signature but unimplementable-outside wrapper over the
    /// private disposal machinery (the sealed-trait pattern, F-026), so
    /// [`dispose_bin`] can name it without letting outside code defuse
    /// swarf.
    pub trait DisposeSealed: sealed::Sealed {
        #[doc(hidden)]
        fn dispose_all(self);
    }
    impl<L: Dispose + sealed::Sealed> DisposeSealed for L {
        fn dispose_all(self) {
            self.dispose()
        }
    }
    mod sealed {
        use super::super::Swarf;
        use model_core::list::{Cons, Nil};
        pub trait Sealed {}
        impl Sealed for Nil {}
        impl<const G: u64, T: Sealed> Sealed for Cons<Swarf<G>, T> {}
    }

    /// A bin — in any fill state — leaves the model to waste disposal (R12):
    /// the only place kept swarf is defused, which is what makes the
    /// tripwires on abandoned swarf trustworthy (R1, F-008).
    ///
    /// Placeholder: waste-disposal service — assumed able to take any bin.
    pub fn dispose_bin<Space, Contents: DisposeSealed>(bin: SwarfBin<Space, Contents>) {
        let SwarfBin {
            contents,
            _space: PhantomData,
        } = bin;
        contents.dispose_all();
    }
}

/// The workshop's processes (R1, R2): pure by-value transformations. They
/// mint quantity-bearing values (plates, swarf, assemblies), so they live
/// inside the resource family's module (F-031), not outside the module tree.
///
/// Reusable resources are threaded loosely (R9, F-024): `drill_holes` takes
/// and returns the person and the drill as loose values, so a flow that
/// needs neither can run concurrently with one that does.
pub mod processes {
    use super::{
        Assembly, BrokenDrillBit, Drill, DrillBit, DrillFail, DrillOk, DrillOutcome,
        DrillOutcomeKind, DrilledPlate, FittedGuard, MachineGuard, Plate, ScrapPlate, SpareParts,
        SteelSheet, Swarf,
    };
    use crate::catalogue::FourOf;
    use crate::characteristics::DrillingCert;
    // One line on purpose: the traceability grep (F-021) skips `use` lines,
    // but only when the line itself starts with `use`.
    use crate::requirements::{Req001FasteningBolt, Req002DrilledBeforeFastening, Req003SwarfWasteConsumer, Req004CertifiedDrillingOperator, Req005FittedDrillGuard};
    use model_core::boundary::{Consumer, SupplyN};
    use model_core::common::processes::qualified_draw_time;
    use model_core::common::{Labour, Qualified};
    use model_core::nat::aliases::N4;

    /// Cuts a steel sheet into two plate blanks plus swarf (R1, R3). Mass is
    /// conserved at compile time (`PA + PB + SW == SHEET`); the caller states
    /// the split (outputs cannot be computed on stable, F-022). The check
    /// fires at monomorphization (F-001): `cargo check` and editor
    /// diagnostics will not show a violation — `cargo build`/`cargo test` do.
    ///
    /// Regression (R4 policy: conservation violations are rustdoc
    /// `compile_fail` doc-tests, never trybuild cases, F-003): cutting a
    /// 2000 g sheet into 900 g + 900 g plates plus 500 g of swarf must not
    /// compile — 300 g would appear from nothing.
    ///
    /// ```compile_fail
    /// use pilot_workshop::resources::boundary::supply_sheet;
    /// use pilot_workshop::resources::processes::cut;
    ///
    /// let sheet = supply_sheet::<2000>();
    /// let (a, b, swarf) = cut::<2000, 900, 900, 500>(sheet);
    /// ```
    pub fn cut<const SHEET: u64, const PA: u64, const PB: u64, const SW: u64>(
        sheet: SteelSheet<SHEET>,
    ) -> (Plate<PA>, Plate<PB>, Swarf<SW>) {
        const {
            assert!(
                PA + PB + SW == SHEET,
                "mass conservation violated in cut (R3): the two plates plus the swarf must sum exactly to the sheet"
            )
        };
        // Conserving transform: the sheet's mass continues as PA + PB + SW.
        sheet.defuse();
        (Plate::mint(), Plate::mint(), Swarf::mint())
    }

    /// Fits the guard to the pillar drill (R18): fitting is a process, and
    /// the fitted state is its own type (R9/F-023) — only [`FittedGuard`]
    /// carries the `Fitted` characteristic REQ-005 requires.
    pub fn fit_guard(guard: MachineGuard) -> FittedGuard {
        // Conserving conversion: the guard continues as the fitted state.
        let MachineGuard { _seal: () } = guard;
        FittedGuard::mint()
    }

    /// Removes the guard (R18): the conserving inverse of [`fit_guard`],
    /// returning the unfitted state.
    pub fn remove_guard(fitted: FittedGuard) -> MachineGuard {
        let FittedGuard { _seal: () } = fitted;
        MachineGuard::mint()
    }

    /// Repairs a broken bit with a repair kit (R17): the failure path back to
    /// the working state. A conserving state conversion (R9): the broken bit
    /// and the kit are consumed into the working bit (the snapped cutter is
    /// assumed replaced in place; discrete objects, R13).
    pub fn repair_bit(broken: BrokenDrillBit, parts: SpareParts) -> DrillBit {
        broken.defuse();
        parts.defuse();
        DrillBit::mint()
    }

    /// Drills the bolt holes in one plate (R1, R2, R15, R18): the operator,
    /// the drill and the guard are moved in and returned; `SPEND_MS` of the
    /// operator's time budget is drawn down (model-core's
    /// `qualified_draw_time`, R15/R18) and leaves as conserved [`Labour`]
    /// that must reach a consumer; the removed material leaves as swarf
    /// (`P_LEFT + SW == PLATE`, checked at compile time).
    ///
    /// **Style B (F-048):** because this process draws the budget *inside*
    /// itself, the budget changes type, which a generic
    /// `O: Req004CertifiedDrillingOperator` bound cannot express — so the
    /// operator is the **concrete** `Qualified<DrillingCert, BUDGET>`, with
    /// REQ-004 restated as a (trivially true) where-clause for traceability.
    /// The cost: a wrong operator here is an E0308 type mismatch, not the
    /// REQ-phrased message — [`drill_holes_fallible`] shows style A, whose
    /// errors are REQ-phrased. The guard needs no budget, so it stays a
    /// generic REQ-005 bound either way.
    ///
    /// The budget const parameters infect this signature and the modeller
    /// restates the running balance at every call — the known R15 cost
    /// (F-030); a wrong balance is a compile error, so the arithmetic stays
    /// compiler-checked. The whole signature sits on one line because
    /// trace.sh attributes requirement bounds to the line they are written on
    /// (R10 rule 4, F-021).
    ///
    /// Overspending the budget is a compile error exactly like overdrawing a
    /// container (R15; E0080 at monomorphization, invisible to `cargo check`,
    /// F-001). Regression: 2000 ms of drilling cannot come out of a 1000 ms
    /// budget:
    ///
    /// ```compile_fail
    /// use model_core::common::boundary::{new_person, qualify};
    /// use pilot_workshop::characteristics::DrillingCert;
    /// use pilot_workshop::resources::boundary::{supply_drill, supply_guard, supply_sheet};
    /// use pilot_workshop::resources::processes::{cut, drill_holes, fit_guard};
    ///
    /// let operator = qualify::<DrillingCert, 1000>(new_person::<1000>());
    /// let drill = supply_drill();
    /// let guard = fit_guard(supply_guard());
    /// let sheet = supply_sheet::<2000>();
    /// let (p1, p2, swarf) = cut::<2000, 900, 900, 200>(sheet);
    /// let out = drill_holes::<2000, 0, 1000, 900, 880, 20, _>(operator, drill, guard, p1);
    /// ```
    ///
    /// Satisfies: REQ-004, REQ-005
    pub fn drill_holes<const SPEND_MS: u64, const T_LEFT: u64, const BUDGET: u64, const PLATE: u64, const P_LEFT: u64, const SW: u64, G: Req005FittedDrillGuard>(operator: Qualified<DrillingCert, BUDGET>, drill: Drill, guard: G, plate: Plate<PLATE>) -> (Qualified<DrillingCert, T_LEFT>, Drill, G, DrilledPlate<P_LEFT>, Swarf<SW>, Labour<SPEND_MS>) where Qualified<DrillingCert, BUDGET>: Req004CertifiedDrillingOperator {
        const {
            assert!(
                P_LEFT + SW == PLATE,
                "mass conservation violated in drill_holes (R3): the drilled plate plus the swarf must sum exactly to the plate blank"
            )
        };
        // The time budget draw (R15/R18): qualified_draw_time delegates to
        // draw_time, whose compile-time assert checks
        // SPEND_MS + T_LEFT == BUDGET.
        let (labour, operator) = qualified_draw_time::<SPEND_MS, T_LEFT, BUDGET, DrillingCert>(operator);
        // Conserving transform: the blank's mass continues as P_LEFT + SW.
        plate.defuse();
        (operator, drill, guard, DrilledPlate::mint(), Swarf::mint(), labour)
    }

    /// Drills one plate **fallibly** (R17): returns `Ok` with the success
    /// bundle or `Err` with the failure bundle, and **both arms conserve the
    /// same inputs** — the operator and the guard come back in both arms, the
    /// plate's mass continues either as drilled-plate-plus-swarf or as
    /// scrap-plus-swarf, and the bit comes back working or broken (one type
    /// per state, R9). The operator's time is **not** drawn here: spends
    /// shared by both arms are drawn *before* the branch, as an adjacent
    /// `qualified_draw_time` in the flow (R17/R18, F-048) — which also keeps
    /// this process generic over any certified operator (**style A**), so an
    /// unqualified person or an unfitted guard fails with the REQ-phrased
    /// `on_unimplemented` message (F-044).
    ///
    /// Per-branch conservation is **one independent compile-time assert per
    /// arm**, and both fire at every instantiation (R17/F-045, the R4
    /// caveat): the caller states the success split (`P_LEFT + SW == PLATE`)
    /// *and* the failure split (`SCRAP + FSW == PLATE`) even though only one
    /// branch runs. The outcome token is consumed (exactly one trial per
    /// attempt, R1); the `Result` is `#[must_use]` (std) on top of the
    /// bundles' own `must_use`, and the bundles implement no `Debug`, so
    /// `.unwrap()`/`.expect()` do not compile (F-047) — the flow must
    /// `match`.
    ///
    /// Success-branch conservation violation — drilling cannot *add* mass
    /// (890 + 20 ≠ 900); an E0080 at monomorphization (F-001), invisible to
    /// `cargo check`, caught by `cargo build`/`cargo test`:
    ///
    /// ```compile_fail
    /// use model_core::common::boundary::{new_person, qualify};
    /// use pilot_workshop::characteristics::DrillingCert;
    /// use pilot_workshop::resources::boundary::{outcome_success, supply_drill_bit, supply_guard, supply_plate};
    /// use pilot_workshop::resources::processes::{drill_holes_fallible, fit_guard};
    ///
    /// let operator = qualify::<DrillingCert, 5000>(new_person::<5000>());
    /// let r = drill_holes_fallible::<900, 890, 20, 880, 20, _, _>(
    ///     operator,
    ///     fit_guard(supply_guard()),
    ///     supply_drill_bit(),
    ///     supply_plate::<900>(),
    ///     outcome_success(),
    /// );
    /// ```
    ///
    /// Failure-branch conservation violation — scrapping cannot *lose* mass
    /// (850 + 20 ≠ 900), rejected even when the flow only ever runs the
    /// success branch (F-045):
    ///
    /// ```compile_fail
    /// use model_core::common::boundary::{new_person, qualify};
    /// use pilot_workshop::characteristics::DrillingCert;
    /// use pilot_workshop::resources::boundary::{outcome_success, supply_drill_bit, supply_guard, supply_plate};
    /// use pilot_workshop::resources::processes::{drill_holes_fallible, fit_guard};
    ///
    /// let operator = qualify::<DrillingCert, 5000>(new_person::<5000>());
    /// let r = drill_holes_fallible::<900, 880, 20, 850, 20, _, _>(
    ///     operator,
    ///     fit_guard(supply_guard()),
    ///     supply_drill_bit(),
    ///     supply_plate::<900>(),
    ///     outcome_success(),
    /// );
    /// ```
    ///
    /// Satisfies: REQ-004, REQ-005
    pub fn drill_holes_fallible<const PLATE: u64, const P_LEFT: u64, const SW: u64, const SCRAP: u64, const FSW: u64, O: Req004CertifiedDrillingOperator, G: Req005FittedDrillGuard>(operator: O, guard: G, bit: DrillBit, plate: Plate<PLATE>, outcome: DrillOutcome) -> Result<DrillOk<O, G, P_LEFT, SW>, DrillFail<O, G, SCRAP, FSW>> {
        const {
            assert!(
                P_LEFT + SW == PLATE,
                "success-branch mass conservation violated in drill_holes_fallible (R3, R17): the drilled plate plus its swarf must sum exactly to the plate blank"
            )
        };
        const {
            assert!(
                SCRAP + FSW == PLATE,
                "failure-branch mass conservation violated in drill_holes_fallible (R3, R17): the scrap plate plus its swarf must sum exactly to the plate blank"
            )
        };
        // Conserving transform: the blank's mass continues in whichever
        // branch is realised.
        plate.defuse();
        match outcome.consume_kind() {
            DrillOutcomeKind::Success => Ok(DrillOk {
                operator,
                guard,
                bit,
                plate: DrilledPlate::mint(),
                swarf: Swarf::mint(),
            }),
            DrillOutcomeKind::Failure => {
                // The bit is consumed by its own failure: working -> broken
                // is a conserving state conversion (R9), not a loss.
                Err(DrillFail {
                    operator,
                    guard,
                    broken_bit: bit.snap(),
                    scrap: ScrapPlate::mint(),
                    swarf: Swarf::mint(),
                })
            }
        }
    }

    /// Fastens two drilled plates into an [`Assembly`] with four bolts taken
    /// from ONE supplier in a single `SupplyN` bound (R12, F-014). Only
    /// REQ-001-approved bolts are accepted — the requirement is the trait
    /// bound on `B`, never a concrete `Bolt<…>` type (R10, F-019) — and only
    /// the drilled processing state can be passed (R9, F-023), restated as
    /// the REQ-002 bounds. Plate mass is conserved into the assembly
    /// (`PA + PB == OUT`, compile-checked; F-001 caveat applies) and the four
    /// bolts are conserved as objects, kept inside the assembly (R13).
    ///
    /// The whole signature sits on one line because the traceability grep
    /// attributes requirement bounds to the line they are written on (R10
    /// rule 4, F-021).
    ///
    /// Satisfies: REQ-002
    pub fn fasten<B: Req001FasteningBolt, S: SupplyN<N4, Taken = FourOf<B>>, const PA: u64, const PB: u64, const OUT: u64>(a: DrilledPlate<PA>, b: DrilledPlate<PB>, bolts: S) -> (Assembly<B, OUT>, S::Rest) where DrilledPlate<PA>: Req002DrilledBeforeFastening, DrilledPlate<PB>: Req002DrilledBeforeFastening {
        const {
            assert!(
                PA + PB == OUT,
                "mass conservation violated in fasten (R3): the assembly's plate mass must sum the two drilled plates exactly"
            )
        };
        let (taken, rest) = bolts.supply_n();
        // Conserving transforms: the plates' mass continues as OUT; the
        // bolts continue as objects inside the assembly (R13) — mint is the
        // conserving combinator, only wrapping values passed in by value.
        a.defuse();
        b.defuse();
        (Assembly::mint(taken), rest)
    }

    /// Hands one piece of swarf to a dedicated swarf waste consumer — the
    /// only sanctioned route for swarf out of a flow, and the process form of
    /// REQ-003. Generic over the consumer (R12, F-015): capacity mistakes
    /// produce the modeller-phrased trait-bound error, and the consumer's
    /// REQ-003 fitness is checked here, at the hand-over.
    pub fn discard_swarf<const G: u64, C: Req003SwarfWasteConsumer + Consumer<Swarf<G>>>(consumer: C, swarf: Swarf<G>) -> C::Next {
        consumer.consume(swarf)
    }
}

#[cfg(test)]
mod tests {
    //! Per-process unit tests (R5): every process turns specific inputs into
    //! the expected outputs with nothing left unaccounted for. These tests
    //! sit inside the privacy boundary, so they may mint fixtures and defuse
    //! outputs directly; downstream-style accounting is exercised by the
    //! integration tests in `tests/`.

    use super::boundary::{
        dispose_bin, new_customer, new_scrap_yard, new_swarf_bin, outcome_failure,
        outcome_success, return_outcome, supply_drill, supply_drill_bit, supply_guard,
        supply_plate, supply_sheet, supply_spare_parts,
    };
    use super::processes::{
        cut, discard_swarf, drill_holes, drill_holes_fallible, fasten, fit_guard, remove_guard,
        repair_bit,
    };
    use super::{
        Assembly, DrillFail, DrillOk, DrilledPlate, FullSwarfBin, Plate, ScrapPlate, Swarf,
        SwarfBin,
    };
    use crate::catalogue::boundary::full_box;
    use crate::catalogue::{EmptyBoltBox, FasteningBolt};
    use crate::characteristics::DrillingCert;
    use model_core::boundary::send_to;
    use model_core::common::boundary::{new_person, qualify};
    use model_core::common::{Labour, Qualified};
    use model_core::history::boundary::new_history;
    use model_core::history::processes::record;
    use model_core::list::{Cons, Nil};
    use model_core::nat::Zero;
    use model_core::nat::aliases::{N2, N4};

    /// `cut` conserves mass (R3, R5): the values are recoverable as
    /// constants and balance by construction — the compile-time assert
    /// already checked the split.
    #[test]
    fn cut_conserves_mass_and_exposes_values() {
        let sheet = supply_sheet::<1000>();
        let (a, b, swarf) = cut::<1000, 450, 450, 100>(sheet);
        assert_eq!(
            Plate::<450>::VALUE + Plate::<450>::VALUE + Swarf::<100>::VALUE,
            1000
        );
        assert_eq!(Swarf::<100>::UNIT, "grams");
        a.defuse();
        b.defuse();
        swarf.defuse();
    }

    /// `drill_holes` conserves mass and time (R3, R15): the drilled plate
    /// plus the swarf equal the blank, the budget goes down by exactly the
    /// labour that comes out, and the operator, drill and guard come back
    /// (R2) — the operator certified (R18), the guard fitted.
    ///
    /// Verifies: REQ-004, REQ-005
    #[test]
    fn drill_holes_conserves_mass_and_draws_time() {
        let operator = qualify::<DrillingCert, 5000>(new_person::<5000>());
        let drill = supply_drill();
        let guard = fit_guard(supply_guard());
        let plate: Plate<450> = Plate::mint();
        let (operator, drill, guard, drilled, swarf, labour) =
            drill_holes::<1000, 4000, 5000, 450, 440, 10, _>(operator, drill, guard, plate);
        assert_eq!(DrilledPlate::<440>::VALUE + Swarf::<10>::VALUE, 450);
        assert_eq!(
            Qualified::<DrillingCert, 4000>::BUDGET_MS + Labour::<1000>::VALUE,
            5000
        );
        drilled.defuse();
        swarf.defuse();
        // Labour is a model-core resource: this crate cannot defuse it, only
        // hand it to a consumer — its production-legal sink is the execution
        // history, which records it attributed to this process (R16, F-035).
        let history = record(new_history(), "drill_holes", labour);
        assert_eq!(history.event_count(), 1);
        let _execution_record_stays_with_the_caller = history;
        let _reusables_stay_with_the_caller = (operator, drill, guard);
    }

    /// Fitting and removing the guard round-trips the two safety states
    /// (R9/R18: one type per state).
    #[test]
    fn guard_states_round_trip() {
        let guard = supply_guard();
        let fitted = fit_guard(guard);
        let _back_on_the_shelf = remove_guard(fitted);
    }

    /// The success arm of the fallible step conserves (R17): mass balances,
    /// and the operator, guard and bit all come back working.
    ///
    /// Verifies: REQ-004, REQ-005
    #[test]
    fn fallible_success_arm_conserves_mass() {
        let operator = qualify::<DrillingCert, 5000>(new_person::<5000>());
        let r = drill_holes_fallible::<900, 880, 20, 880, 20, _, _>(
            operator,
            fit_guard(supply_guard()),
            supply_drill_bit(),
            supply_plate::<900>(),
            outcome_success(),
        );
        match r {
            Ok(DrillOk {
                operator,
                guard,
                bit,
                plate,
                swarf,
            }) => {
                assert_eq!(DrilledPlate::<880>::VALUE + Swarf::<20>::VALUE, 900);
                plate.defuse();
                swarf.defuse();
                let _reusables = (operator, guard, bit);
            }
            Err(_) => unreachable!("a success token cannot produce the failure arm"),
        }
    }

    /// The failure arm conserves the **same inputs** (R17): the plate's whole
    /// mass continues as scrap + swarf, the bit comes back broken, and the
    /// operator and guard come back unchanged.
    ///
    /// Verifies: REQ-004, REQ-005
    #[test]
    fn fallible_failure_arm_conserves_the_same_inputs() {
        let operator = qualify::<DrillingCert, 5000>(new_person::<5000>());
        let r = drill_holes_fallible::<900, 880, 20, 880, 20, _, _>(
            operator,
            fit_guard(supply_guard()),
            supply_drill_bit(),
            supply_plate::<900>(),
            outcome_failure(),
        );
        match r {
            Ok(_) => unreachable!("a failure token cannot produce the success arm"),
            Err(DrillFail {
                operator,
                guard,
                broken_bit,
                scrap,
                swarf,
            }) => {
                assert_eq!(ScrapPlate::<880>::VALUE + Swarf::<20>::VALUE, 900);
                scrap.defuse();
                swarf.defuse();
                broken_bit.defuse();
                let _reusables = (operator, guard);
            }
        }
    }

    /// Repair converts the failure state back to the working state (R9/R17),
    /// consuming the kit; the scrap yard accounts for what cannot be
    /// repaired.
    #[test]
    fn repair_restores_the_working_state_and_scrap_is_accounted() {
        let operator = qualify::<DrillingCert, 5000>(new_person::<5000>());
        let r = drill_holes_fallible::<900, 880, 20, 880, 20, _, _>(
            operator,
            fit_guard(supply_guard()),
            supply_drill_bit(),
            supply_plate::<900>(),
            outcome_failure(),
        );
        match r {
            Ok(_) => unreachable!("a failure token cannot produce the success arm"),
            Err(fail) => {
                let bit = repair_bit(fail.broken_bit, supply_spare_parts());
                let yard = send_to(new_scrap_yard(), fail.scrap);
                fail.swarf.defuse();
                let _accounted = (fail.operator, fail.guard, bit, yard);
            }
        }
    }

    /// An untried token has an accounted exit at the boundary (R12/F-050):
    /// the tripwire stays quiet.
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

    /// `fasten` takes exactly four REQ-001 bolts from one supplier (F-014),
    /// conserves the plates' mass into the assembly, and returns the
    /// exhausted box as a distinct resource (R12).
    ///
    /// Verifies: REQ-001, REQ-002
    #[test]
    fn fasten_joins_two_drilled_plates_with_four_bolts() {
        let a: DrilledPlate<440> = DrilledPlate::mint();
        let b: DrilledPlate<440> = DrilledPlate::mint();
        let bolts = full_box::<FasteningBolt, N4>();
        let (assembly, rest): (Assembly<FasteningBolt, 880>, EmptyBoltBox) = fasten(a, b, bolts);
        assert_eq!(Assembly::<FasteningBolt, 880>::PLATE_GRAMS, 880);
        let _empty_box: EmptyBoltBox = rest;
        // The product leaves through the boundary customer (R12).
        let _customer = send_to(new_customer(), assembly);
    }

    /// The swarf bin keeps what it consumes while its space counts down
    /// (R12, F-016, F-034), and a full bin leaves the model only through the
    /// boundary's waste disposal.
    ///
    /// Verifies: REQ-003
    #[test]
    fn swarf_bin_keeps_swarf_until_disposal() {
        let bin = new_swarf_bin::<N2>();
        assert_eq!(SwarfBin::<N2>::HELD, 0);
        let s1: Swarf<100> = Swarf::mint();
        let s2: Swarf<10> = Swarf::mint();
        let bin = discard_swarf(bin, s1);
        let bin = discard_swarf(bin, s2);
        assert_eq!(
            FullSwarfBin::<Cons<Swarf<10>, Cons<Swarf<100>, Nil>>>::HELD,
            2
        );
        let full: SwarfBin<Zero, _> = bin; // full: a distinct state (R12)
        dispose_bin(full);
    }
}
