//! The café counter's sealed resource family (R1), its creation/exit boundary
//! (R12) and its processes P1..P8 (SPEC.md §5, F-031).
//!
//! Layout per F-006/F-031: this module holds the sealed resource types, with
//! the [`boundary`] (the counter setup, the customer, the sinks and the
//! end-of-flow exits) and the [`processes`] (which mint quantity-bearing
//! values and therefore live inside the privacy boundary) as child modules.
//!
//! One type per processing state (R9, F-023), straight from SPEC.md §3:
//!
//! * milk: in the [`MilkBottle`] (300 g; 150 g per draw) → [`Milk`] (drawn)
//!   → [`SteamedMilk`] (Ok) or [`BurntMilk`] (Fail — fed to the [`Drain`]
//!   **inside** `steam_milk`, REQ-016's only exit);
//! * coffee: [`Hopper`] → [`Grounds`] (18 g dose) + [`WaterTank`] →
//!   [`MachineWater`] (40 g) → [`EspressoShot`] (36 g) + [`SpentPuck`] (22 g,
//!   fed to the [`KnockBox`] **inside** `pull_espresso`);
//! * tea: [`Urn`] → [`UrnWater`] (300 g) + [`Teabag`] (3 g, from the
//!   [`TeabagBox`]) → [`PotOfTea`] (303 g, holding its teapot and cup);
//! * drinks: [`FlatWhite`] (186 g, holding its cup); the final
//!   [`ServedOrder`] (tray + drinks + crockery, generic over the flat-white
//!   slot — see below);
//! * money (R19): [`Money`] in pence, the [`Till`] container
//!   (0 → 700 → 320 on the refund path) and the sealed [`PaymentReceipt`]
//!   evidence token (REQ-017's key, consumed at the join).
//!
//! ## The shot and its cup travel as loose values
//!
//! SPEC.md P1 produces "the espresso shot in its cup"; here the
//! [`EspressoShot`] and its [`Cup`] are threaded side by side as loose values
//! (R9/F-024 loose threading) rather than packed into a holder type. That is
//! deliberate: on path 3 the pair **splits** — the stranded shot is fed to
//! the [`Drain`] while the cup goes back to the counter stack (both inside
//! [`processes::drain_stranded_shot`], SPEC.md P8; §8 review decision 1) —
//! and a holder would need a hand-written extraction (F-040: macro payload is
//! consumption-only) for no checking gain.
//!
//! ## The stranded shot goes to the drain (SPEC.md P8)
//!
//! Path 3's already-pulled shot has no drink to join, so it is honest waste:
//! [`processes::drain_stranded_shot`] (P8, path 3 only) feeds the 36 g shot
//! to the [`Drain`] **inside the process** — the same `Consumer` machinery
//! as the burnt milk — and restacks its cup. The barista's 30 000 ms is
//! drawn by the adjacent `qualified_draw_time` in the flow and recorded to
//! `h_a` under the process name (F-048, R16), so all four of branch A's
//! path-3 events are time draws.
//!
//! Every consumable here carries the kernel's tripwire `Drop` (R1 layer 2,
//! F-008) except the crockery and teabags ([`Cup`], [`Teapot`], [`Teabag`] —
//! `no_tripwire`, F-040: they are *kept* by the box or by the drink states
//! that account for them). Reusable resources (the [`EspressoMachine`]; the
//! barista as model-core's `Qualified<MachineTraining, MS>` and the server as
//! a plain `Person`, each with the SPEC's 600 000 ms budget) are moved in and
//! returned by every process (R2) and stay with the caller.

use crate::characteristics::{
    ChangeRecipient, DrainSink, MachineTraining, OrderSlot, ProofOfPayment, sealed,
};
// One line on purpose: the traceability grep (F-021) skips `use` lines, but
// only when the line itself starts with `use`.
use crate::requirements::{assert_req015, assert_req016, assert_req017, assert_req018};
use model_core::boundary::{Consumer, Supplier};
use model_core::common::Qualified;
use model_core::list::{Cons, Len, Nil};

// ---------------------------------------------------------------------------
// Prices (R19; one currency, pence — SPEC.md §3).
// ---------------------------------------------------------------------------

/// The flat white's listed price, in pence (SPEC.md §3: 380 p). Stated once:
/// path 3's refund is exactly this amount, and the refund-in-lieu
/// [`OrderSlot`] impl exists only at it.
pub const FLAT_WHITE_PRICE_PENCE: u64 = 380;

/// The pot of tea's listed price, in pence (SPEC.md §3: 320 p).
pub const TEA_PRICE_PENCE: u64 = 320;

/// The order's total price, in pence (SPEC.md §3: 700 p = 380 + 320).
pub const ORDER_PRICE_PENCE: u64 = 700;

// The order price really is the sum of its items (R7 values are usable in
// checks).
const _: () = assert!(ORDER_PRICE_PENCE == FLAT_WHITE_PRICE_PENCE + TEA_PRICE_PENCE);

/// The note the customer tenders, in pence (SPEC.md §3: 1000 p). The tender
/// is a **concrete type** in [`processes::take_payment`]'s signature (the
/// F-051 exact-amount idea applied to the till): paying with any other note
/// is a type-check-time E0308 naming the right amount, visible to editors and
/// trybuild — unlike the post-monomorphization split assert (F-001).
pub const TENDERED_NOTE_PENCE: u64 = 1000;

// ---------------------------------------------------------------------------
// Money and the till (R19; R15 containers).
// ---------------------------------------------------------------------------

model_core::container_resource! {
    /// Cash in hand, in pence (GBP, integer minor units — the R7 money row;
    /// R19). Tendered by the customer, split for change
    /// ([`processes::split_money`]), deposited in the till, drawn back out as
    /// the path-3 refund — and conserved like any other quantity: abandoned
    /// change trips the tripwire.
    Money,
    unit = "pence",
    must_use = "Money is a conserved resource: pass it on, deposit it in the till, or hand it to a Consumer"
}

model_core::container_resource! {
    /// The till holding `V` pence (R15 quantity container applied to money,
    /// R19; SPEC.md §3: starts at 0, takes the 700 p payment, gives back the
    /// 380 p refund on path 3). The till **keeps** the money — it is a
    /// container, not a sink — and stays at the counter at flow end
    /// ([`boundary::lock_till`]). Tripwired (F-008).
    Till,
    unit = "pence",
    must_use = "Till is a conserved resource: even at close it must be accounted for (lock_till at the boundary)"
}

// ---------------------------------------------------------------------------
// Milk (SPEC.md §3: 300 g bottle, 150 g per steaming attempt).
// ---------------------------------------------------------------------------

model_core::container_resource! {
    /// The milk bottle holding `V` grams (R15; SPEC.md §3: 300 g at flow
    /// start — **deliberately exactly two** 150 g steaming attempts: the
    /// rework bound is the provisioning, F-050). `MilkBottle<0>` is the empty
    /// state, still accounted for at the boundary
    /// ([`boundary::bottle_back_to_fridge`]). Tripwired (F-008).
    MilkBottle,
    unit = "grams",
    must_use = "MilkBottle is a conserved resource: even an empty bottle must be accounted for"
}

model_core::container_resource! {
    /// Milk drawn from the bottle, in grams (R15; SPEC.md §3: 150 g per
    /// steaming attempt). Minted only by [`processes::draw_milk`] and
    /// consumed by [`processes::steam_milk`]. Tripwired (F-008).
    Milk,
    unit = "grams",
    must_use = "Milk is a conserved resource: steam it or hand it to a Consumer"
}

model_core::container_resource! {
    /// Steamed milk, in grams (SPEC.md P2 Ok arm: 150 g, in the jug). The
    /// only milk state [`processes::build_flat_white`] accepts — burnt milk
    /// in its place is a type error (REQ-016: never served). Tripwired
    /// (F-008).
    SteamedMilk,
    unit = "grams",
    must_use = "SteamedMilk is a conserved resource: build it into the flat white or hand it to a Consumer"
}

model_core::container_resource! {
    /// Burnt milk, in grams (SPEC.md P2 Fail arm: 150 g). **Its only exit is
    /// the [`Drain`]** (REQ-016 structural): no process accepts it — it can
    /// be neither re-steamed ([`processes::steam_milk`] takes [`Milk`]) nor
    /// served ([`processes::build_flat_white`] takes [`SteamedMilk`]) — and
    /// in production it never exists loose: `steam_milk`'s failure arm feeds
    /// it to the drain **inside the process**. Tripwired (F-008).
    BurntMilk,
    unit = "grams",
    must_use = "BurntMilk is a conserved waste product: it must reach the drain (REQ-016)"
}

// ---------------------------------------------------------------------------
// Coffee: hopper, tank, dose, shot, puck (SPEC.md §3).
// ---------------------------------------------------------------------------

model_core::container_resource! {
    /// The grounds hopper holding `V` grams (R15; SPEC.md §3: 500 g at flow
    /// start; 18 g per dose). Stays at the counter at flow end
    /// ([`boundary::keep_hopper`]). Tripwired (F-008).
    Hopper,
    unit = "grams",
    must_use = "Hopper is a conserved resource: even a drawn-down hopper must be accounted for"
}

model_core::container_resource! {
    /// A dose of ground coffee, in grams (SPEC.md §3: 18 g). Minted only by
    /// [`processes::draw_grounds`], consumed by [`processes::pull_espresso`].
    /// Tripwired (F-008).
    Grounds,
    unit = "grams",
    must_use = "Grounds is a conserved resource: pull it through the machine or hand it to a Consumer"
}

model_core::container_resource! {
    /// The espresso machine's water tank holding `V` grams (R15; SPEC.md §3:
    /// 200 g at flow start; 40 g per shot). Stays at the counter at flow end
    /// ([`boundary::keep_tank`]). Tripwired (F-008).
    WaterTank,
    unit = "grams",
    must_use = "WaterTank is a conserved resource: even a drawn-down tank must be accounted for"
}

model_core::container_resource! {
    /// Machine water drawn from the tank, in grams (SPEC.md §3: 40 g per
    /// shot). Tripwired (F-008).
    MachineWater,
    unit = "grams",
    must_use = "MachineWater is a conserved resource: pull it through the machine or hand it to a Consumer"
}

model_core::container_resource! {
    /// The pulled espresso shot, in grams (SPEC.md §3: 36 g = 18 g dose +
    /// 40 g water − 22 g puck). It travels **beside its [`Cup`]** as a loose
    /// pair (see the module docs). Its production exits: built into the flat
    /// white ([`processes::build_flat_white`]), or — stranded on path 3 —
    /// fed to the [`Drain`] inside [`processes::drain_stranded_shot`]
    /// (SPEC.md P8). Tripwired (F-008).
    EspressoShot,
    unit = "grams",
    must_use = "EspressoShot is a conserved resource: build the flat white or feed it to the drain (drain_stranded_shot)"
}

model_core::container_resource! {
    /// The spent coffee puck, in grams (SPEC.md §3: 22 g per shot). In
    /// production it never exists loose: [`processes::pull_espresso`] feeds
    /// it to the [`KnockBox`] **inside the process** (SPEC.md P1 waste
    /// routing). Tripwired (F-008).
    SpentPuck,
    unit = "grams",
    must_use = "SpentPuck is a conserved waste product: it must reach the knock box"
}

// ---------------------------------------------------------------------------
// Tea: urn, teabags, the box (SPEC.md §3).
// ---------------------------------------------------------------------------

model_core::container_resource! {
    /// The hot-water urn holding `V` grams (R15; SPEC.md §3: 1000 g at flow
    /// start; 300 g per pot). Stays at the counter at flow end
    /// ([`boundary::keep_urn`]). Tripwired (F-008).
    Urn,
    unit = "grams",
    must_use = "Urn is a conserved resource: even a drawn-down urn must be accounted for"
}

model_core::container_resource! {
    /// Urn water drawn for one pot, in grams (SPEC.md §3: 300 g per pot).
    /// Tripwired (F-008).
    UrnWater,
    unit = "grams",
    must_use = "UrnWater is a conserved resource: brew it or hand it to a Consumer"
}

model_core::consumable_resource! {
    /// A dry teabag: a discrete item, its own object (R13; SPEC.md §3: 3 g,
    /// see [`Teabag::MASS_G`]). Deliberately **not** tripwired
    /// (`no_tripwire`, F-040): teabags are *kept* by the box that holds them.
    /// Whole-value discard is still caught by `#[must_use]`.
    Teabag,
    must_use = "Teabag is a conserved resource: brew it or hand it to a Consumer",
    no_tripwire
}

impl Teabag {
    /// A teabag's mass, in grams (R7; SPEC.md §3).
    pub const MASS_G: u64 = 3;
}

/// The teabag box (SPEC.md §3: a box of 10): `Bags` is a type-level list of
/// the real [`Teabag`] objects it holds (R12, R13 — the count *is* the list's
/// length). Teabags leave one at a time through the `Supplier` impl below;
/// the box stays with the counter at flow end.
///
/// Placeholder: teabag box — brand/vendor not modelled (SPEC.md §4).
#[must_use = "TeabagBox is a boundary resource: pass it on like any other resource"]
pub struct TeabagBox<Bags> {
    bags: Bags,
    _seal: (),
}

/// A type-level list of ten teabags — the box as it enters the model.
pub type TenTeabags = Cons<
    Teabag,
    Cons<
        Teabag,
        Cons<
            Teabag,
            Cons<Teabag, Cons<Teabag, Cons<Teabag, Cons<Teabag, Cons<Teabag, Cons<Teabag, Cons<Teabag, Nil>>>>>>>,
        >,
    >,
>;

/// A type-level list of nine teabags — the box after the order's one pot.
pub type NineTeabags = Cons<
    Teabag,
    Cons<
        Teabag,
        Cons<Teabag, Cons<Teabag, Cons<Teabag, Cons<Teabag, Cons<Teabag, Cons<Teabag, Cons<Teabag, Nil>>>>>>>,
    >,
>;

/// The box as it enters the model: 10 teabags (SPEC.md §3).
pub type FreshTeabagBox = TeabagBox<TenTeabags>;

/// `Supplier` is implemented ONLY for a box with teabags left (R12); taking
/// a teabag from an empty box is a compile error with the modeller-phrased
/// `on_unimplemented` message from model-core (F-015).
impl<H, T> Supplier for TeabagBox<Cons<H, T>> {
    type Item = H;
    type Next = TeabagBox<T>;
    fn supply(self) -> (H, TeabagBox<T>) {
        let TeabagBox {
            bags: Cons(head, tail),
            _seal: (),
        } = self;
        (
            head,
            TeabagBox {
                bags: tail,
                _seal: (),
            },
        )
    }
}

impl<Bags: Len> TeabagBox<Bags> {
    /// How many teabags the box holds — the length of its contents list, so
    /// count and contents cannot disagree (R7, R12).
    pub const BAGS: u64 = Bags::LEN;
}

// ---------------------------------------------------------------------------
// Crockery (SPEC.md §3: 2 cups, 1 teapot, 1 tray — leave with the customer).
// ---------------------------------------------------------------------------

model_core::consumable_resource! {
    /// A cup (crockery, massless in this model — SPEC.md §1 scopes crockery
    /// mass out). One rides with the espresso shot and leaves inside the flat
    /// white (or goes back to the stack on path 3,
    /// [`boundary::restack_cup`]); the other leaves inside the pot-of-tea
    /// service. Deliberately **not** tripwired (`no_tripwire`, F-040): cups
    /// are kept by the drink states that account for them. Whole-value
    /// discard is still caught by `#[must_use]`.
    Cup,
    must_use = "Cup is a conserved resource: it leaves with a drink or goes back to the counter stack",
    no_tripwire
}

model_core::consumable_resource! {
    /// The teapot (crockery, massless in this model). Kept by the
    /// [`PotOfTea`] and leaves with the customer. `no_tripwire` (F-040):
    /// kept contents are untripwired. Whole-value discard is still caught by
    /// `#[must_use]`.
    Teapot,
    must_use = "Teapot is a conserved resource: it leaves inside the pot-of-tea service",
    no_tripwire
}

model_core::consumable_resource! {
    /// The serving tray (crockery, massless in this model). Loose until the
    /// join assembles the order on it; it leaves with the customer.
    /// Tripwired (F-008): a tray the flow never assembles an order on fails
    /// the test that leaked it.
    Tray,
    must_use = "Tray is a conserved resource: assemble the order on it or hand it to a Consumer"
}

// ---------------------------------------------------------------------------
// The drinks and the served order (one type per state, R9).
// ---------------------------------------------------------------------------

model_core::consumable_resource! {
    /// The flat white: `V` grams in its cup (SPEC.md P3: 186 g = 36 g shot +
    /// 150 g steamed milk; the cup is held as sealed payload, R12, and leaves
    /// with the customer when the order is consumed). Tripwired (F-008).
    FlatWhite<const V: u64> {
        cup: Cup,
    },
    must_use = "FlatWhite is a conserved resource: hand it over on the order or hand it to a Consumer"
}

impl<const V: u64> FlatWhite<V> {
    /// The drink's mass, in grams (R7; SPEC.md §3: 186 g as built).
    pub const MASS_G: u64 = V;
}

/// The flat white at the order's build (186 g) may take the flat-white slot
/// on the tray (one of [`OrderSlot`]'s two satisfying types — the other is
/// the path-3 refund in lieu). The extraction is permit-gated (F-054): only
/// [`processes::hand_over`] can consume the drink into the order.
impl OrderSlot for FlatWhite<186> {
    fn onto_tray(self, _permit: HandoverPermit) {
        // Conserving transform: the drink (and its held cup) continues onto
        // the customer's tray inside the served order (R1).
        self.defuse();
    }
}
impl sealed::Sealed for FlatWhite<186> {}

/// The refund in lieu of the flat white (SPEC.md §6: "P7 replaces the flat
/// white in P6 on path 3"): exactly the flat white's 380 p price may take its
/// slot on the tray — [`OrderSlot`]'s second satisfying type.
impl OrderSlot for Money<FLAT_WHITE_PRICE_PENCE> {
    fn onto_tray(self, _permit: HandoverPermit) {
        // Conserving transform: the refund continues onto the tray and leaves
        // with the customer inside the served order (R1, R19).
        self.defuse();
    }
}
impl sealed::Sealed for Money<FLAT_WHITE_PRICE_PENCE> {}

model_core::consumable_resource! {
    /// The pot of tea: `V` grams in its pot (SPEC.md P5: 303 g = 300 g water
    /// + 3 g teabag; the teapot and the service cup are held as sealed
    /// payload, R12, and leave with the customer when the order is consumed).
    /// Tripwired (F-008).
    PotOfTea<const V: u64> {
        teapot: Teapot,
        cup: Cup,
    },
    must_use = "PotOfTea is a conserved resource: hand it over on the order or hand it to a Consumer"
}

impl<const V: u64> PotOfTea<V> {
    /// The service's mass, in grams (R7; SPEC.md §3: 303 g as brewed).
    pub const MASS_G: u64 = V;
}

model_core::consumable_resource! {
    /// The served order (SPEC.md P6): the tray with the pot of tea and the
    /// flat-white slot `W` — the flat white itself, or its 380 p refund in
    /// lieu on path 3 (the deliberately two-type [`OrderSlot`]). Minted only
    /// by [`processes::hand_over`], which consumed the payment receipt
    /// (REQ-017); its only exit is the [`Customer`]. Tripwired (F-008).
    ServedOrder<W: OrderSlot>,
    must_use = "ServedOrder is a conserved resource: it must reach the customer"
}

// ---------------------------------------------------------------------------
// The payment receipt (REQ-017's key) and the permits.
// ---------------------------------------------------------------------------

model_core::consumable_resource! {
    /// The sealed payment receipt (SPEC.md §3): evidence that the till branch
    /// ran, minted only by [`processes::take_payment`] and consumed exactly
    /// once, at the join ([`processes::hand_over`], REQ-017). The one
    /// deliberate cross-branch dependency of this concurrency case study:
    /// without it, "hand over before payment" would merely be bad manners —
    /// with it, the flow does not compile. Tripwired (F-008).
    PaymentReceipt,
    must_use = "PaymentReceipt is REQ-017's key: the join consumes it - do not discard it"
}

/// The sealed proof-of-payment characteristic on the receipt (REQ-017's
/// subject): `surrender` is the permit-gated conserving extraction only
/// [`processes::hand_over`] can call (F-054).
impl ProofOfPayment for PaymentReceipt {
    fn surrender(self, _permit: HandoverPermit) {
        // The evidence is spent on exactly one hand-over (R1).
        self.defuse();
    }
}
impl sealed::Sealed for PaymentReceipt {}

/// The receipt under its requirement-facing name; the alias carries the tag
/// because a tag inside the `consumable_resource!` invocation above would be
/// silently dropped by trace.sh (F-037).
///
/// Satisfies: REQ-017
pub type OrderPaymentReceipt = PaymentReceipt;
model_core::satisfies!(assert_req017, OrderPaymentReceipt);

/// The permit gating [`ProofOfPayment::surrender`] and
/// [`OrderSlot::onto_tray`] (the F-054 permit-gated extraction pattern): a
/// private field and no public constructor, so only
/// [`processes::hand_over`] (inside this privacy boundary) can call the
/// extractions — they can never be used to vanish a receipt or a drink
/// outside the join that accounts for them (R1).
pub struct HandoverPermit {
    _seal: (),
}

// ---------------------------------------------------------------------------
// The staff and the machine.
// ---------------------------------------------------------------------------

/// The trained barista at the machine: model-core's R18 `Qualified` wrapper
/// holding the common `Person` with the [`MachineTraining`] qualification and
/// `BUDGET_MS` person-milliseconds left (SPEC.md §3: 600 000 ms at flow
/// start). The alias carries the satisfaction tag (F-037), and `Qualified` is
/// only REQ-015-fit with the machine training in its slot (F-043); the server
/// stays a plain `Person` and is refused at the machine.
///
/// Satisfies: REQ-015
pub type TrainedBarista<const BUDGET_MS: u64> = Qualified<MachineTraining, BUDGET_MS>;
model_core::satisfies!(assert_req015, TrainedBarista<0>);

model_core::reusable_resource! {
    /// The espresso machine (SPEC.md §3): the equipment REQ-015 gates — only
    /// a trained barista may operate it ([`processes::pull_espresso`]).
    /// Reusable (R2): moved in and returned; no tripwire.
    ///
    /// Placeholder: counter setup at flow start — one machine.
    EspressoMachine,
    must_use = "EspressoMachine is a reusable resource: pass it on or return it to the caller"
}

// ---------------------------------------------------------------------------
// The steam outcome token (R17, F-042) and the fallible step's bundles.
// ---------------------------------------------------------------------------

model_core::outcome_token! {
    /// One trial of the environment (R17): whether a single steaming attempt
    /// textures the milk or burns it. Consumed by exactly one process,
    /// [`processes::steam_milk`]; a flow cannot read it — it must run the
    /// process and handle both arms of the `Result`. Two are provisioned per
    /// order (SPEC.md §3), bounding the rework to two attempts alongside the
    /// bottle's two 150 g draws (F-050).
    SteamOutcome(SteamOutcomeKind),
    success = steam_goes_well,
    failure = milk_burns,
    exit = return_steam_outcome,
    must_use = "SteamOutcome is a boundary token: run it through exactly one steaming attempt or return it to the environment"
}

/// Everything a successful steaming attempt produces (R17): the steamed milk
/// plus every conserved participant — the barista comes back (the time draw
/// is an adjacent process in the flow, F-048, so the barista returns at the
/// same budget in both arms) and the drain comes back untouched (nothing
/// burnt).
///
/// A bundle is a **grouping** (R1), not a sealed resource (F-046): its fields
/// are public so the handling arm can destructure it, and building one
/// requires already *holding* the sealed resources, so it cannot mint
/// anything. It deliberately has **no `Debug` impl** (F-047), which makes
/// `Result::unwrap()`/`expect()` on the process result a compile error — see
/// `tests/ui/unwrap_steam_result.rs`.
#[must_use = "SteamOk bundles conserved outputs: every field must be accounted for"]
pub struct SteamOk<D, const B: u64, const STEAMED: u64> {
    /// The barista, back unchanged (the time draw is adjacent, R17/F-048).
    pub barista: Qualified<MachineTraining, B>,
    /// The steamed milk (SPEC.md P2: 150 g, in the jug).
    pub milk: SteamedMilk<STEAMED>,
    /// The drain, back untouched: a successful attempt burns nothing.
    pub drain: D,
}

/// Everything a failed steaming attempt produces (R17). **Failure conserves
/// too**: the barista comes back (the time was drawn before the branch —
/// failure cost it too), and the burnt milk has already been fed to the
/// drain **inside the process** (REQ-016 structural — burnt milk never
/// exists loose), so the bundle carries the drain's next state. There is no
/// milk field: the burnt state's only exit is the drain.
///
/// Like [`SteamOk`] it is a grouping with public fields and no `Debug`
/// (F-046, F-047).
#[must_use = "SteamFail bundles conserved failure outputs: every field must be accounted for"]
pub struct SteamFail<D, const B: u64> {
    /// The barista, back unchanged (the time was drawn before the branch,
    /// R17/F-048).
    pub barista: Qualified<MachineTraining, B>,
    /// The drain after taking the burnt milk (REQ-016: fed inside the
    /// process).
    pub drain: D,
}

// ---------------------------------------------------------------------------
// Boundary objects at the counter's edge (SPEC.md §4).
// ---------------------------------------------------------------------------

/// The customer at the counter (SPEC.md §4): the boundary object the order is
/// fulfilled *for*. They **supply the tender** ([`boundary::tender_cash`])
/// and **consume** the served order, the change and any refund — all with
/// `Next = Self` (an unbounded boundary object, legal only at the boundary,
/// R15, F-029). REQ-018's satisfying type: the change's only exit.
///
/// Placeholder: the customer — refine to a queue of real customers.
///
/// Satisfies: REQ-018
#[must_use = "Customer is a boundary resource: pass it on like any other resource"]
pub struct Customer {
    _seal: (),
}

// Compile-checked backing for the tag above (R10, F-020).
model_core::satisfies!(assert_req018, Customer);

impl ChangeRecipient for Customer {}

/// The customer takes delivery of the served order — with the flat white on
/// the tray, or with the 380 p refund in its place (path 3); `Next = Self`
/// (R15, F-029): an unbounded sink discards its intake, so end-state holdings
/// are assertable arithmetically only.
impl<W: OrderSlot> Consumer<ServedOrder<W>> for Customer {
    type Next = Customer;
    fn consume(self, order: ServedOrder<W>) -> Customer {
        order.defuse(); // sanctioned consumer role (F-008); unbounded sink discards (F-029)
        self
    }
}

/// The customer pockets money — the 300 p change (inside
/// [`processes::take_payment`], REQ-018 structural); `Next = Self` (R15,
/// F-029). (The path-3 refund reaches them *on the tray*, through the
/// [`OrderSlot`] in the served order.)
impl<const V: u64> Consumer<Money<V>> for Customer {
    type Next = Customer;
    fn consume(self, cash: Money<V>) -> Customer {
        cash.defuse(); // sanctioned consumer role (F-008); unbounded sink discards (F-029)
        self
    }
}

/// The drain (SPEC.md §4): the dedicated sink REQ-016 routes burnt milk to,
/// **inside** [`processes::steam_milk`] — and the sink path 3's stranded
/// espresso shot is fed to, inside [`processes::drain_stranded_shot`]
/// (SPEC.md P8). An unbounded sink (`type Next = Self`, R15, F-029); it
/// necessarily discards what it consumes, so end-state waste masses are
/// assertable arithmetically only.
///
/// Placeholder: the drain — assumed able to take any amount of burnt milk.
///
/// Satisfies: REQ-016
#[must_use = "Drain is a boundary resource: pass it on like any other resource"]
pub struct Drain {
    _seal: (),
}

// Compile-checked backing for the tag above (R10, F-020).
model_core::satisfies!(assert_req016, Drain);

impl DrainSink for Drain {}

/// The drain absorbs burnt milk of any mass; `Next = Self` (R15).
impl<const V: u64> Consumer<BurntMilk<V>> for Drain {
    type Next = Drain;
    fn consume(self, item: BurntMilk<V>) -> Drain {
        item.defuse(); // sanctioned consumer role (F-008); unbounded sink discards (F-029)
        self
    }
}

/// The drain absorbs path 3's stranded espresso shot (SPEC.md P8);
/// `Next = Self` (R15, F-029).
impl<const V: u64> Consumer<EspressoShot<V>> for Drain {
    type Next = Drain;
    fn consume(self, item: EspressoShot<V>) -> Drain {
        item.defuse(); // sanctioned consumer role (F-008); unbounded sink discards (F-029)
        self
    }
}

/// The knock box (SPEC.md §4): the unbounded boundary sink the spent puck is
/// fed to **inside** [`processes::pull_espresso`] (`type Next = Self`, R15,
/// F-029).
///
/// Placeholder: the knock box — assumed able to take any number of pucks.
#[must_use = "KnockBox is a boundary resource: pass it on like any other resource"]
pub struct KnockBox {
    _seal: (),
}

/// The knock box absorbs spent pucks; `Next = Self` (R15, F-029).
impl<const V: u64> Consumer<SpentPuck<V>> for KnockBox {
    type Next = KnockBox;
    fn consume(self, item: SpentPuck<V>) -> KnockBox {
        item.defuse(); // sanctioned consumer role (F-008); unbounded sink discards (F-029)
        self
    }
}

/// The creation and exit boundary of the counter's resource family (R12,
/// F-006): the only production code where the stock, the equipment, the
/// customer and the sinks come into existence, and where the drawn-down
/// containers and the restacked cup leave the model (SPEC.md §4).
pub mod boundary {
    use super::{
        Cons, Cup, Customer, Drain, EspressoMachine, FreshTeabagBox, Hopper, KnockBox, MilkBottle,
        Money, Nil, Teabag, TeabagBox, Teapot, Till, Tray, Urn, WaterTank, TENDERED_NOTE_PENCE,
    };

    // The outcome token's boundary constructors and exit are generated by
    // `model_core::outcome_token!` at the family level; they are re-exported
    // here because boundary constructors live in the boundary module (R12).
    pub use super::{milk_burns, return_steam_outcome, steam_goes_well};

    /// The espresso machine enters the model (R12).
    ///
    /// Placeholder: counter setup at flow start — one machine.
    pub fn new_espresso_machine() -> EspressoMachine {
        EspressoMachine::mint()
    }

    /// The milk bottle enters the model holding 300 g (R12; SPEC.md §3 —
    /// deliberately exactly two 150 g steaming attempts, F-050).
    ///
    /// Placeholder: counter setup at flow start — the fridge.
    pub fn stock_milk_bottle() -> MilkBottle<300> {
        MilkBottle::mint()
    }

    /// The empty or part-used bottle goes back to the fridge at flow end
    /// (R12): the boundary exit for [`MilkBottle`], whatever it still holds
    /// (R15: every end-state resource is accounted for).
    ///
    /// Placeholder: the fridge — an unbounded home for bottles.
    pub fn bottle_back_to_fridge<const V: u64>(bottle: MilkBottle<V>) {
        // Sanctioned boundary consumption (F-008 role).
        bottle.defuse();
    }

    /// The grounds hopper enters the model holding 500 g (R12; SPEC.md §3).
    ///
    /// Placeholder: counter setup at flow start.
    pub fn fill_hopper() -> Hopper<500> {
        Hopper::mint()
    }

    /// The drawn-down hopper stays at the counter at flow end (R12).
    ///
    /// Placeholder: the counter at close of flow.
    pub fn keep_hopper<const V: u64>(hopper: Hopper<V>) {
        hopper.defuse();
    }

    /// The machine's water tank enters the model holding 200 g (R12;
    /// SPEC.md §3).
    ///
    /// Placeholder: counter setup at flow start.
    pub fn fill_water_tank() -> WaterTank<200> {
        WaterTank::mint()
    }

    /// The drawn-down tank stays at the counter at flow end (R12).
    ///
    /// Placeholder: the counter at close of flow.
    pub fn keep_tank<const V: u64>(tank: WaterTank<V>) {
        tank.defuse();
    }

    /// The urn enters the model holding 1000 g (R12; SPEC.md §3).
    ///
    /// Placeholder: counter setup at flow start.
    pub fn fill_urn() -> Urn<1000> {
        Urn::mint()
    }

    /// The drawn-down urn stays at the counter at flow end (R12).
    ///
    /// Placeholder: the counter at close of flow.
    pub fn keep_urn<const V: u64>(urn: Urn<V>) {
        urn.defuse();
    }

    /// The fill function (R12): a fresh teabag box enters the model — 10
    /// real teabags held as a type-level list (SPEC.md §3).
    ///
    /// Placeholder: teabag box — brand/vendor not modelled.
    pub fn new_teabag_box() -> FreshTeabagBox {
        TeabagBox {
            bags: Cons(
                Teabag::mint(),
                Cons(
                    Teabag::mint(),
                    Cons(
                        Teabag::mint(),
                        Cons(
                            Teabag::mint(),
                            Cons(
                                Teabag::mint(),
                                Cons(
                                    Teabag::mint(),
                                    Cons(
                                        Teabag::mint(),
                                        Cons(Teabag::mint(), Cons(Teabag::mint(), Cons(Teabag::mint(), Nil))),
                                    ),
                                ),
                            ),
                        ),
                    ),
                ),
            ),
            _seal: (),
        }
    }

    /// A cup enters the model from the counter stack (R12; SPEC.md §3: 2
    /// cups per order).
    ///
    /// Placeholder: counter setup at flow start — the cup stack.
    pub fn new_cup() -> Cup {
        Cup::mint()
    }

    /// The path-3 cup goes back to the counter stack (R12; SPEC.md §8 review
    /// decision 1): the boundary exit for a [`Cup`] the stranded shot no
    /// longer needs.
    ///
    /// Placeholder: the cup stack — an unbounded home for clean cups.
    pub fn restack_cup(cup: Cup) {
        // Sanctioned boundary consumption: no_tripwire items are destructured
        // (F-040), not defused.
        let Cup { _seal: _ } = cup;
    }

    /// The teapot enters the model (R12; SPEC.md §3).
    ///
    /// Placeholder: counter setup at flow start.
    pub fn new_teapot() -> Teapot {
        Teapot::mint()
    }

    /// The serving tray enters the model (R12; SPEC.md §3).
    ///
    /// Placeholder: counter setup at flow start.
    pub fn new_tray() -> Tray {
        Tray::mint()
    }

    /// The till opens empty (R12; SPEC.md §3: the float is out of scope —
    /// change comes from the tendered note by splitting).
    ///
    /// Placeholder: counter setup at flow start.
    pub fn open_till() -> Till<0> {
        Till::mint()
    }

    /// The till stays at the counter at flow end, holding its path's balance
    /// (R12; SPEC.md §6: 700 p served, 320 p after the path-3 refund).
    ///
    /// Placeholder: the counter at close of flow.
    pub fn lock_till<const V: u64>(till: Till<V>) {
        till.defuse();
    }

    /// The customer arrives at the counter (R12).
    ///
    /// Placeholder: the customer — refine to a queue of real customers.
    pub fn new_customer() -> Customer {
        Customer { _seal: () }
    }

    /// The customer tenders their note (R12, R19; SPEC.md §3: 1000 p): a
    /// draw-style boundary source (`Supplier` is discrete-only, F-028), with
    /// `Next = Self` (F-029).
    ///
    /// Placeholder: the customer's wallet.
    pub fn tender_cash(customer: Customer) -> (Money<TENDERED_NOTE_PENCE>, Customer) {
        (Money::mint(), customer)
    }

    /// The drain enters the model (R12).
    pub fn new_drain() -> Drain {
        Drain { _seal: () }
    }

    /// The knock box enters the model (R12).
    pub fn new_knock_box() -> KnockBox {
        KnockBox { _seal: () }
    }
}

/// The counter's processes P1..P8 (SPEC.md §5): pure by-value transformations
/// (R1, R2). They mint quantity-bearing values, so they live inside the
/// resource family's module (F-031).
///
/// The two actors are threaded loosely (R9, F-024) — an aggregate would
/// over-claim and delete exactly the concurrency this case study exists to
/// demonstrate. Per F-048 each actor's time is drawn by **adjacent** draw
/// processes in the flow (never inside these processes), the barista's
/// labour recorded into **`h_a`** and the server's into **`h_b`** — one
/// `History` per branch (R16), merged at the join by [`hand_over`]. The P2
/// draw happens *before* the fallible branch, so failure costs the same time
/// as success (R17).
pub mod processes {
    use super::{
        BurntMilk, Cup, EspressoMachine, EspressoShot, FlatWhite, Grounds, HandoverPermit,
        MachineTraining, MachineWater, Milk, MilkBottle, Money, PaymentReceipt, PotOfTea,
        Qualified, ServedOrder, SpentPuck, SteamFail, SteamOk, SteamOutcome, SteamOutcomeKind,
        SteamedMilk, Teabag, Teapot, Till, Tray, Urn, UrnWater, WaterTank, Hopper,
        FLAT_WHITE_PRICE_PENCE, ORDER_PRICE_PENCE, TENDERED_NOTE_PENCE,
    };
    use crate::characteristics::OrderSlot;
    // One line on purpose: the traceability grep (F-021) skips `use` lines,
    // but only when the line itself starts with `use`.
    use crate::requirements::{Req015TrainedBarista, Req016BurntMilkToTheDrain, Req017PaymentBeforeHandover, Req018ChangeInFull};
    use model_core::boundary::{Consumer, send_to};
    use model_core::common::Person;
    use model_core::history::History;
    use model_core::history::processes::merge;

    // -- Stock draws (R15; caller-stated remainder, F-022/F-030) ------------

    model_core::draw_process! {
        /// Draws `TAKE` grams of milk from a bottle holding `FULL`, leaving
        /// `LEFT` (SPEC.md §3: 150 g per steaming attempt). **Overdrawing is
        /// a compile error** (E0080 at monomorphization, invisible to
        /// `cargo check`, F-001) — which is what makes a third steaming
        /// attempt inexpressible: the 300 g bottle provisions exactly two
        /// draws (R17/F-050).
        pub fn draw_milk: MilkBottle => Milk,
        assert = "milk conservation violated in draw_milk (R15): TAKE + LEFT must equal FULL - is the draw larger than the bottle's remaining milk? (the 300 g bottle provisions exactly two 150 g attempts, F-050)"
    }

    model_core::draw_process! {
        /// Draws `TAKE` grams of ground coffee from the hopper (SPEC.md §3:
        /// 18 g per dose). Overdrawing is the standard R15 compile error
        /// (F-001).
        pub fn draw_grounds: Hopper => Grounds,
        assert = "grounds conservation violated in draw_grounds (R15): TAKE + LEFT must equal FULL - is the draw larger than the hopper's remaining grounds?"
    }

    model_core::draw_process! {
        /// Draws `TAKE` grams of water from the machine's tank (SPEC.md §3:
        /// 40 g per shot). Overdrawing is the standard R15 compile error
        /// (F-001).
        pub fn draw_water: WaterTank => MachineWater,
        assert = "water conservation violated in draw_water (R15): TAKE + LEFT must equal FULL - is the draw larger than the tank's remaining water?"
    }

    model_core::draw_process! {
        /// Draws `TAKE` grams of hot water from the urn (SPEC.md §3: 300 g
        /// per pot). Overdrawing is the standard R15 compile error (F-001).
        pub fn draw_urn_water: Urn => UrnWater,
        assert = "water conservation violated in draw_urn_water (R15): TAKE + LEFT must equal FULL - is the draw larger than the urn's remaining water?"
    }

    // -- Money processes (R19; R3 split/combine over the till) --------------

    /// Splits cash into two amounts (R3: changing an amount is a process) —
    /// the change-giving primitive. `A + B == IN` is checked at compile time;
    /// a violation is the standard E0080 (F-001).
    pub fn split_money<const IN: u64, const A: u64, const B: u64>(
        cash: Money<IN>,
    ) -> (Money<A>, Money<B>) {
        const {
            assert!(
                A + B == IN,
                "money conservation violated in split_money (R3/R19): the two output amounts must sum exactly to the input amount"
            )
        };
        cash.defuse();
        (Money::mint(), Money::mint())
    }

    /// Deposits cash in the till (R3 combine, caller-stated total, F-022):
    /// `Till<BALANCE>` + `Money<ADD>` → `Till<NEW>` with
    /// `BALANCE + ADD == NEW` checked at compile time. A wrong total is the
    /// same E0080 as a bad draw (F-001).
    pub fn deposit_till<const ADD: u64, const BALANCE: u64, const NEW: u64>(
        till: Till<BALANCE>,
        cash: Money<ADD>,
    ) -> Till<NEW> {
        const {
            assert!(
                BALANCE + ADD == NEW,
                "money conservation violated in deposit_till (R3/R19): BALANCE + ADD must equal NEW - is the stated new balance wrong?"
            )
        };
        // Conserving transform: both inputs continue as the new balance.
        till.defuse();
        cash.defuse();
        Till::mint()
    }

    // -- Branch A: the barista (records to `h_a`) ----------------------------

    /// P1 — pulls the espresso (SPEC.md P1). REQ-015 bounds the barista
    /// (style A, F-048): an untrained person — the server included — fails
    /// with the REQ-phrased `on_unimplemented` message (F-044). The machine
    /// is returned (R2); the barista's 90 000 ms is drawn by the adjacent
    /// `qualified_draw_time` in the flow and recorded to `h_a` (F-048, R16).
    /// The spent puck is fed to the knock box **inside the process**
    /// (SPEC.md P1 waste routing), and the shot leaves **beside its cup** as
    /// a loose pair (see the module docs). Mass 18 + 40 = 36 + 22 is checked
    /// at compile time, per instantiation (F-001/F-045).
    ///
    /// Espresso mass violation — a shot cannot weigh more than its inputs
    /// minus the puck (18 + 40 ≠ 37 + 22); an E0080 at monomorphization
    /// (F-001), invisible to `cargo check`, caught by
    /// `cargo build`/`cargo test`:
    ///
    /// ```compile_fail
    /// use cs3_cafe_orders::characteristics::MachineTraining;
    /// use cs3_cafe_orders::resources::boundary::{fill_hopper, fill_water_tank, new_cup, new_espresso_machine, new_knock_box};
    /// use cs3_cafe_orders::resources::processes::{draw_grounds, draw_water, pull_espresso};
    /// use model_core::common::boundary::{new_person, qualify};
    ///
    /// let barista = qualify::<MachineTraining, 600_000>(new_person::<600_000>());
    /// let (grounds, hopper) = draw_grounds::<18, 482, 500>(fill_hopper());
    /// let (water, tank) = draw_water::<40, 160, 200>(fill_water_tank());
    /// let r = pull_espresso::<18, 40, 37, 22, _, _>(barista, new_espresso_machine(), grounds, water, new_cup(), new_knock_box());
    /// ```
    ///
    /// Satisfies: REQ-015
    pub fn pull_espresso<const DOSE: u64, const WATER: u64, const SHOT: u64, const PUCK: u64, O: Req015TrainedBarista, K: Consumer<SpentPuck<PUCK>>>(barista: O, machine: EspressoMachine, grounds: Grounds<DOSE>, water: MachineWater<WATER>, cup: Cup, knock_box: K) -> (O, EspressoMachine, EspressoShot<SHOT>, Cup, K::Next) {
        const {
            assert!(
                DOSE + WATER == SHOT + PUCK,
                "mass conservation violated in pull_espresso (R3): the dose plus the water must sum exactly to the shot plus the spent puck (SPEC.md P1: 18 + 40 = 36 + 22)"
            )
        };
        // Conserving transform: the dose and water continue as the shot and
        // the puck; the puck is fed to the knock box INSIDE the process
        // (SPEC.md P1 waste routing), so it never exists loose.
        grounds.defuse();
        water.defuse();
        let knock_box = send_to(knock_box, SpentPuck::<PUCK>::mint());
        (barista, machine, EspressoShot::mint(), cup, knock_box)
    }

    /// P2 — steams the milk: **fallible** (R17, SPEC.md P2). Takes the drawn
    /// milk, one outcome token (the only way variability enters, F-042), and
    /// the drain (REQ-016, style A: anything that is not the drain fails
    /// with the REQ-phrased message, F-044): on failure the burnt milk is
    /// fed to the drain **inside this process**, so burnt milk never exists
    /// loose and the burnt state's only exit is the drain — REQ-016 is
    /// structural. The barista returns in both arms at the same budget (the
    /// 60 000 ms per attempt is drawn by the adjacent `qualified_draw_time`
    /// *before* the branch, F-048).
    ///
    /// Per-branch conservation is **one independent compile-time assert per
    /// arm**, and both fire at every instantiation (R17/F-045, the R4
    /// caveat): the caller states the success mass (`M == STEAMED`) *and*
    /// the failure mass (`M == BURNT`) even though only one branch runs. The
    /// `Result` is `#[must_use]` (std) on top of the bundles' own
    /// `must_use`, and the bundles implement no `Debug`, so
    /// `.unwrap()`/`.expect()` do not compile (F-047) — the flow must
    /// `match`.
    ///
    /// Success-branch conservation violation — steaming cannot add mass
    /// (150 ≠ 160); an E0080 at monomorphization (F-001):
    ///
    /// ```compile_fail
    /// use cs3_cafe_orders::characteristics::MachineTraining;
    /// use cs3_cafe_orders::resources::boundary::{new_drain, steam_goes_well, stock_milk_bottle};
    /// use cs3_cafe_orders::resources::processes::{draw_milk, steam_milk};
    /// use model_core::common::boundary::{new_person, qualify};
    ///
    /// let barista = qualify::<MachineTraining, 600_000>(new_person::<600_000>());
    /// let (milk, bottle) = draw_milk::<150, 150, 300>(stock_milk_bottle());
    /// let r = steam_milk::<150, 160, 150, 600_000, _>(barista, milk, steam_goes_well(), new_drain());
    /// ```
    ///
    /// Failure-branch conservation violation — a failed attempt cannot lose
    /// milk down the drain's ledger (150 ≠ 140), rejected even when the flow
    /// only ever runs the success branch (F-045):
    ///
    /// ```compile_fail
    /// use cs3_cafe_orders::characteristics::MachineTraining;
    /// use cs3_cafe_orders::resources::boundary::{new_drain, steam_goes_well, stock_milk_bottle};
    /// use cs3_cafe_orders::resources::processes::{draw_milk, steam_milk};
    /// use model_core::common::boundary::{new_person, qualify};
    ///
    /// let barista = qualify::<MachineTraining, 600_000>(new_person::<600_000>());
    /// let (milk, bottle) = draw_milk::<150, 150, 300>(stock_milk_bottle());
    /// let r = steam_milk::<150, 150, 140, 600_000, _>(barista, milk, steam_goes_well(), new_drain());
    /// ```
    ///
    /// A third steaming attempt is inexpressible (R17/F-050): the 300 g
    /// bottle provisions exactly two 150 g draws —
    ///
    /// ```compile_fail
    /// use cs3_cafe_orders::resources::boundary::stock_milk_bottle;
    /// use cs3_cafe_orders::resources::processes::draw_milk;
    ///
    /// let (m1, bottle) = draw_milk::<150, 150, 300>(stock_milk_bottle());
    /// let (m2, bottle) = draw_milk::<150, 0, 150>(bottle);
    /// let (m3, bottle) = draw_milk::<150, 0, 0>(bottle);
    /// ```
    ///
    /// Satisfies: REQ-016
    pub fn steam_milk<const M: u64, const STEAMED: u64, const BURNT: u64, const B: u64, D: Req016BurntMilkToTheDrain + Consumer<BurntMilk<BURNT>>>(barista: Qualified<MachineTraining, B>, milk: Milk<M>, outcome: SteamOutcome, drain: D) -> Result<SteamOk<D, B, STEAMED>, SteamFail<<D as Consumer<BurntMilk<BURNT>>>::Next, B>> {
        const {
            assert!(
                M == STEAMED,
                "success-branch mass conservation violated in steam_milk (R3, R17): the steamed milk must carry exactly the drawn milk's mass"
            )
        };
        const {
            assert!(
                M == BURNT,
                "failure-branch mass conservation violated in steam_milk (R3, R17): the burnt milk fed to the drain must carry exactly the drawn milk's mass"
            )
        };
        match outcome.consume_kind() {
            SteamOutcomeKind::Success => {
                // Conserving state change: the milk's mass continues as the
                // steamed milk (R1).
                milk.defuse();
                Ok(SteamOk {
                    barista,
                    milk: SteamedMilk::mint(),
                    drain,
                })
            }
            SteamOutcomeKind::Failure => {
                // The milk continues as burnt milk, which is fed to the drain
                // INSIDE the process (REQ-016 structural): burnt milk never
                // exists loose.
                milk.defuse();
                let drain = send_to(drain, BurntMilk::<BURNT>::mint());
                Err(SteamFail { barista, drain })
            }
        }
    }

    /// P3 — builds the flat white (SPEC.md P3): the shot (with its cup) and
    /// the steamed milk become the 186 g drink, in the cup. Burnt milk in
    /// the milk position is a type error (REQ-016: never served). The
    /// barista's 30 000 ms is drawn by the adjacent `qualified_draw_time` in
    /// the flow and recorded to `h_a` (F-048, R16). Mass 36 + 150 = 186 is
    /// checked at compile time, per instantiation (F-001).
    pub fn build_flat_white<const SHOT: u64, const MILK: u64, const OUT: u64, const B: u64>(
        barista: Qualified<MachineTraining, B>,
        shot: EspressoShot<SHOT>,
        cup: Cup,
        milk: SteamedMilk<MILK>,
    ) -> (Qualified<MachineTraining, B>, FlatWhite<OUT>) {
        const {
            assert!(
                SHOT + MILK == OUT,
                "mass conservation violated in build_flat_white (R3): the shot plus the steamed milk must sum exactly to the flat white (SPEC.md P3: 36 + 150 = 186)"
            )
        };
        // Conserving transform: the shot and milk continue as the drink; the
        // cup continues inside it as held payload (R12).
        shot.defuse();
        milk.defuse();
        (barista, FlatWhite::mint(cup))
    }

    /// P8 — drains the stranded shot (SPEC.md P8; **path 3 only**): when both
    /// steams fail there is no drink for the already-pulled shot to join, so
    /// it is honest waste — the shot is fed to the drain **inside the
    /// process** (the same `Consumer` machinery as the burnt milk; SPEC.md §8
    /// review decision 1) and its cup goes back to the counter stack
    /// ([`super::boundary::restack_cup`]). The barista's 30 000 ms is drawn
    /// by the adjacent `qualified_draw_time` in the flow and recorded to
    /// `h_a` under the process name (F-048, R16). Mass is conserved
    /// structurally: the shot is fed whole, so there is nothing to assert.
    pub fn drain_stranded_shot<const SHOT: u64, const B: u64, D: Consumer<EspressoShot<SHOT>>>(
        barista: Qualified<MachineTraining, B>,
        shot: EspressoShot<SHOT>,
        cup: Cup,
        drain: D,
    ) -> (Qualified<MachineTraining, B>, D::Next) {
        // The shot is fed to the drain INSIDE the process (SPEC.md P8 waste
        // routing), so the stranded state never exists past the process.
        let drain = send_to(drain, shot);
        // The cup goes back to the counter stack through its boundary exit.
        super::boundary::restack_cup(cup);
        (barista, drain)
    }

    // -- Branch B: the server (records to `h_b`) -----------------------------

    /// P4 — takes payment at the till (R19, SPEC.md P4). The tender is the
    /// **concrete** `Money<1000>` (the F-051 exact-amount pattern applied to
    /// the till): paying with any other note is a type-check-time E0308
    /// naming the right amount. The note is split 700 + 300 (R3, assert),
    /// the 700 p order price is deposited in the till (assert), and the
    /// 300 p change goes back to the customer **inside the process** —
    /// REQ-018 structural: the split's second output has only the customer
    /// exit, bounded here (style A). The **payment receipt** — REQ-017's key,
    /// the one deliberate cross-branch dependency — is minted here and
    /// consumed only at the join. The server's 60 000 ms is drawn by the
    /// adjacent `draw_time` in the flow and recorded to `h_b` (F-048, R16).
    ///
    /// Payment split violation — claiming 400 p change from the 1000 p note
    /// against the 700 p order (1000 ≠ 700 + 400) must not compile; an E0080
    /// at monomorphization (F-001 — note the composed-process echo whose
    /// instantiation note points at the `split_money` call inside, F-051):
    ///
    /// ```compile_fail
    /// use cs3_cafe_orders::resources::boundary::{new_customer, open_till, tender_cash};
    /// use cs3_cafe_orders::resources::processes::take_payment;
    /// use model_core::common::boundary::new_person;
    ///
    /// let server = new_person::<600_000>();
    /// let (cash, customer) = tender_cash(new_customer());
    /// let r = take_payment::<400, 0, 700, 600_000, _>(server, open_till(), cash, customer);
    /// ```
    ///
    /// Satisfies: REQ-017, REQ-018
    pub fn take_payment<const CHANGE: u64, const TILL: u64, const NEW: u64, const B: u64, C: Req018ChangeInFull + Consumer<Money<CHANGE>>>(server: Person<B>, till: Till<TILL>, cash: Money<TENDERED_NOTE_PENCE>, customer: C) -> (Person<B>, Till<NEW>, PaymentReceipt, C::Next) {
        const {
            assert!(
                TENDERED_NOTE_PENCE == ORDER_PRICE_PENCE + CHANGE,
                "money conservation violated in take_payment (R1/R19): the tendered note must equal the order price plus the change - is the stated change wrong for the 700 p order?"
            )
        };
        let (price, change) = split_money::<TENDERED_NOTE_PENCE, ORDER_PRICE_PENCE, CHANGE>(cash);
        let till = deposit_till::<ORDER_PRICE_PENCE, TILL, NEW>(till, price);
        // The change goes straight back to the customer INSIDE the process
        // (REQ-018 structural): the split's second output has only the
        // customer exit.
        let customer = send_to(customer, change);
        (server, till, PaymentReceipt::mint(), customer)
    }

    /// P5 — brews the pot of tea (SPEC.md P5): the urn water and the teabag
    /// become the 303 g pot, with the teapot and the service cup held inside
    /// the service (R12). The server's 120 000 ms is drawn by the adjacent
    /// `draw_time` in the flow and recorded to `h_b` (F-048, R16). Mass
    /// 300 + 3 = 303 is checked at compile time, per instantiation (F-001).
    pub fn brew_tea<const WATER: u64, const OUT: u64, const B: u64>(
        server: Person<B>,
        water: UrnWater<WATER>,
        teabag: Teabag,
        teapot: Teapot,
        cup: Cup,
    ) -> (Person<B>, PotOfTea<OUT>) {
        const {
            assert!(
                WATER + Teabag::MASS_G == OUT,
                "mass conservation violated in brew_tea (R3): the urn water plus the teabag must sum exactly to the pot of tea (SPEC.md P5: 300 + 3 = 303)"
            )
        };
        // Conserving transform: the water and teabag continue as the tea; the
        // crockery continues inside the service as held payload (R12).
        water.defuse();
        let Teabag { _seal: _ } = teabag;
        (server, PotOfTea::mint(teapot, cup))
    }

    // -- The join -------------------------------------------------------------

    /// P6 — assembles the order on the tray and hands it over (SPEC.md P6):
    /// the pot of tea, the flat-white slot (the drink itself, or its 380 p
    /// refund in lieu on path 3 — [`OrderSlot`]'s two satisfying types), the
    /// tray, and the **payment receipt** (REQ-017, style A: hand-over
    /// without it is the REQ-phrased error, F-044) — and **merges `h_a` and
    /// `h_b`** (R16): the merged record is a partial order (`Entry::Join`),
    /// claiming no interleaving between the branches. The server's 30 000 ms
    /// is drawn by the adjacent `draw_time` and recorded to `h_b` **before**
    /// the merge (SPEC.md P6, structural). Items: 2 drinks (or drink +
    /// refund) + tray = 1 order (structural — crockery is massless here).
    ///
    /// Satisfies: REQ-017
    pub fn hand_over<const TEA_G: u64, const B: u64, W: OrderSlot, R: Req017PaymentBeforeHandover>(server: Person<B>, slot: W, tea: PotOfTea<TEA_G>, tray: Tray, receipt: R, h_a: History, h_b: History) -> (Person<B>, ServedOrder<W>, History) {
        // The permit-gated conserving extractions (F-054): the slot's
        // contents and the receipt's evidence continue into the served order;
        // the tea and tray continue onto it directly.
        slot.onto_tray(HandoverPermit { _seal: () });
        receipt.surrender(HandoverPermit { _seal: () });
        tea.defuse();
        tray.defuse();
        // The join (R16): the two branch records merge into one partial
        // order; the merged History stays with the caller.
        let history = merge(h_a, h_b);
        (server, ServedOrder::mint(), history)
    }

    /// P7 — refunds the flat white (SPEC.md P7; path 3 only): the server
    /// draws the flat white's 380 p price back out of the till (R15/R19
    /// caller-stated remainder, F-022/F-030); the refund then **takes the
    /// flat white's place on the order tray** (SPEC.md §6: "P7 replaces the
    /// flat white in P6") through the refund-in-lieu [`OrderSlot`]. The
    /// server's 30 000 ms is drawn by the adjacent `draw_time` in the flow
    /// and recorded to `h_b` (F-048, R16).
    ///
    /// Till-refund violation — refunding 380 p cannot leave 330 p in a 700 p
    /// till (380 + 330 ≠ 700); an E0080 at monomorphization (F-001),
    /// invisible to `cargo check`, caught by `cargo build`/`cargo test`:
    ///
    /// ```compile_fail
    /// use cs3_cafe_orders::resources::boundary::{new_customer, open_till, tender_cash};
    /// use cs3_cafe_orders::resources::processes::{refund_flat_white, take_payment};
    /// use model_core::common::boundary::new_person;
    ///
    /// let server = new_person::<600_000>();
    /// let (cash, customer) = tender_cash(new_customer());
    /// let (server, till, receipt, customer) = take_payment::<300, 0, 700, 600_000, _>(server, open_till(), cash, customer);
    /// let (server, till, refund) = refund_flat_white::<330, 700, 600_000>(server, till);
    /// ```
    pub fn refund_flat_white<const LEFT: u64, const TILL: u64, const B: u64>(
        server: Person<B>,
        till: Till<TILL>,
    ) -> (Person<B>, Till<LEFT>, Money<FLAT_WHITE_PRICE_PENCE>) {
        const {
            assert!(
                FLAT_WHITE_PRICE_PENCE + LEFT == TILL,
                "money conservation violated in refund_flat_white (R15/R19): the 380 p refund plus LEFT must equal the till's balance - is the stated remainder wrong? (SPEC.md P7: 700 = 320 + 380)"
            )
        };
        // Conserving transform: the till's balance continues as the refund
        // plus the remainder.
        till.defuse();
        (server, Till::mint(), Money::mint())
    }
}

#[cfg(test)]
mod tests {
    //! Per-process unit tests (R5): every process turns specific inputs into
    //! the expected outputs with nothing left unaccounted for — one test per
    //! arm for the fallible P2 (branch coverage is leak coverage, F-002).
    //! These tests sit inside the privacy boundary, so they may mint fixtures
    //! and defuse outputs directly; downstream-style accounting is exercised
    //! by the integration tests in `tests/flows.rs`.

    use super::boundary::{
        fill_hopper, fill_urn, fill_water_tank, lock_till, milk_burns, new_cup, new_customer,
        new_drain, new_espresso_machine, new_knock_box, new_teabag_box, new_teapot, new_tray,
        open_till, restack_cup, steam_goes_well, stock_milk_bottle, tender_cash,
    };
    use super::processes::{
        brew_tea, build_flat_white, deposit_till, drain_stranded_shot, draw_grounds, draw_milk,
        draw_urn_water, draw_water, hand_over, pull_espresso, refund_flat_white, split_money,
        steam_milk, take_payment,
    };
    use super::{
        Cup, EspressoShot, FlatWhite, FreshTeabagBox, Milk, Money, NineTeabags, PotOfTea,
        SteamedMilk, Teabag, TeabagBox, Till, Tray, FLAT_WHITE_PRICE_PENCE, ORDER_PRICE_PENCE,
        TEA_PRICE_PENCE, TENDERED_NOTE_PENCE,
    };
    use crate::characteristics::MachineTraining;
    use model_core::boundary::take_one;
    use model_core::common::boundary::{new_person, qualify};
    use model_core::history::boundary::new_history;
    use model_core::history::processes::record;
    use model_core::history::Entry;

    /// P1: the dose and water split exactly into the shot and the puck
    /// (SPEC.md P1: 18 + 40 = 36 + 22), the puck reaches the knock box
    /// inside the process, the machine comes back (R2), and only the trained
    /// barista could do it.
    ///
    /// Verifies: REQ-015
    #[test]
    fn pull_espresso_balances_the_shot_and_puck() {
        let barista = qualify::<MachineTraining, 600_000>(new_person::<600_000>());
        let (grounds, hopper) = draw_grounds::<18, 482, 500>(fill_hopper());
        let (water, tank) = draw_water::<40, 160, 200>(fill_water_tank());
        let (barista, machine, shot, cup, knock_box) = pull_espresso::<18, 40, 36, 22, _, _>(
            barista,
            new_espresso_machine(),
            grounds,
            water,
            new_cup(),
            new_knock_box(),
        );
        assert_eq!(EspressoShot::<36>::VALUE + 22, 18 + 40);
        // Inside the privacy boundary this test may defuse directly.
        shot.defuse();
        restack_cup(cup);
        hopper.defuse();
        tank.defuse();
        let _reusables = (barista, machine, knock_box);
    }

    /// P2 success arm (R17): the drawn milk continues as steamed milk
    /// (SPEC.md P2: 150 = 150), the barista comes back, and the drain is
    /// untouched.
    #[test]
    fn steam_milk_success_arm_balances() {
        let barista = qualify::<MachineTraining, 600_000>(new_person::<600_000>());
        let (milk, bottle) = draw_milk::<150, 150, 300>(stock_milk_bottle());
        match steam_milk::<150, 150, 150, 600_000, _>(barista, milk, steam_goes_well(), new_drain())
        {
            Ok(ok) => {
                assert_eq!(SteamedMilk::<150>::VALUE, Milk::<150>::VALUE);
                ok.milk.defuse();
                let _accounted = (ok.barista, ok.drain);
            }
            Err(_fail) => panic!("a success token must realise the success arm"),
        }
        bottle.defuse();
    }

    /// P2 failure arm (R17): the burnt milk (150 g) is fed to the drain
    /// INSIDE the process (REQ-016 structural — the burnt state's only
    /// exit), and the barista comes back — failure conserves too.
    ///
    /// Verifies: REQ-016
    #[test]
    fn steam_milk_failure_arm_feeds_the_drain_inside() {
        let barista = qualify::<MachineTraining, 600_000>(new_person::<600_000>());
        let (milk, bottle) = draw_milk::<150, 150, 300>(stock_milk_bottle());
        match steam_milk::<150, 150, 150, 600_000, _>(barista, milk, milk_burns(), new_drain()) {
            Ok(_ok) => panic!("a failure token must realise the failure arm"),
            Err(fail) => {
                // The drain discarded the 150 g (F-029): the mass is
                // recovered from the constants (R7).
                assert_eq!(Milk::<150>::VALUE, 150);
                let _accounted = (fail.barista, fail.drain);
            }
        }
        bottle.defuse();
    }

    /// P3: the shot and the steamed milk become the flat white
    /// (SPEC.md P3: 36 + 150 = 186), in the cup, with the barista back (R2).
    #[test]
    fn build_flat_white_balances() {
        let barista = qualify::<MachineTraining, 600_000>(new_person::<600_000>());
        let shot: EspressoShot<36> = EspressoShot::mint();
        let milk: SteamedMilk<150> = SteamedMilk::mint();
        let (barista, flat_white) = build_flat_white::<36, 150, 186, 600_000>(
            barista,
            shot,
            new_cup(),
            milk,
        );
        assert_eq!(FlatWhite::<186>::MASS_G, 36 + 150);
        flat_white.defuse();
        let _reusable = barista;
    }

    /// P4: the 1000 p note splits 700 + 300 (SPEC.md P4, assert), the till
    /// takes exactly the order price (0 + 700 = 700, assert), the change
    /// reaches the customer inside the process (REQ-018 structural), and the
    /// receipt — REQ-017's key — is produced.
    ///
    /// Verifies: REQ-017, REQ-018
    #[test]
    fn take_payment_splits_deposits_and_returns_the_change() {
        let server = new_person::<600_000>();
        let (cash, customer) = tender_cash(new_customer());
        let (server, till, receipt, customer) =
            take_payment::<300, 0, 700, 600_000, _>(server, open_till(), cash, customer);
        assert_eq!(ORDER_PRICE_PENCE + Money::<300>::VALUE, TENDERED_NOTE_PENCE);
        assert_eq!(Till::<700>::VALUE, 700);
        assert_eq!(ORDER_PRICE_PENCE, FLAT_WHITE_PRICE_PENCE + TEA_PRICE_PENCE);
        receipt.defuse();
        lock_till(till);
        let _accounted = (server, customer);
    }

    /// The money dimension conserves through a bare split too (R3/R19).
    #[test]
    fn split_money_conserves_the_amount() {
        let (cash, customer) = tender_cash(new_customer());
        let (a, b) = split_money::<1000, 700, 300>(cash);
        assert_eq!(Money::<700>::VALUE + Money::<300>::VALUE, 1000);
        a.defuse();
        b.defuse();
        let _customer = customer;
    }

    /// The deposit combine conserves (R3/R19): 320 + 380 back to 700.
    #[test]
    fn deposit_till_conserves_the_amount() {
        let till: Till<320> = Till::mint();
        let refund: Money<380> = Money::mint();
        let till = deposit_till::<380, 320, 700>(till, refund);
        assert_eq!(Till::<700>::VALUE, 700);
        lock_till(till);
    }

    /// P5: the urn water and the teabag become the pot of tea
    /// (SPEC.md P5: 300 + 3 = 303), holding its teapot and cup, with the
    /// server back (R2).
    #[test]
    fn brew_tea_balances() {
        let server = new_person::<600_000>();
        let (water, urn) = draw_urn_water::<300, 700, 1000>(fill_urn());
        let (teabag, teabags) = take_one(new_teabag_box());
        let (server, tea) = brew_tea::<300, 303, 600_000>(server, water, teabag, new_teapot(), new_cup());
        assert_eq!(PotOfTea::<303>::MASS_G, 300 + Teabag::MASS_G);
        let _teabags: TeabagBox<NineTeabags> = teabags;
        tea.defuse();
        urn.defuse();
        let _reusable = server;
    }

    /// P6 with the flat white in its slot (the served paths): the order is
    /// assembled only with the receipt (REQ-017), and the merge produces a
    /// single `Entry::Join` holding each branch's records un-interleaved
    /// (R16).
    ///
    /// Verifies: REQ-017
    #[test]
    fn hand_over_consumes_the_receipt_and_merges_the_branches() {
        // A receipt can only come from the payment (REQ-017's key).
        let server = new_person::<600_000>();
        let (cash, customer) = tender_cash(new_customer());
        let (server, till, receipt, customer) =
            take_payment::<300, 0, 700, 600_000, _>(server, open_till(), cash, customer);

        // Two tiny branch histories to see the Join shape through P6.
        let barista = qualify::<MachineTraining, 600_000>(new_person::<600_000>());
        let (labour_a, barista) =
            model_core::common::processes::qualified_draw_time::<90_000, 510_000, 600_000, _>(barista);
        let h_a = record(new_history(), "pull_espresso", labour_a);
        let (labour_b, server) =
            model_core::common::processes::draw_time::<60_000, 540_000, 600_000>(server);
        let h_b = record(new_history(), "take_payment", labour_b);

        let flat_white: FlatWhite<186> = FlatWhite::mint(new_cup());
        let tea: PotOfTea<303> = PotOfTea::mint(new_teapot(), new_cup());
        let (server, order, history) =
            hand_over::<303, 540_000, _, _>(server, flat_white, tea, new_tray(), receipt, h_a, h_b);
        assert_eq!(history.event_count(), 2);
        assert!(matches!(history.entries(), [Entry::Join(a, b)] if a.len() == 1 && b.len() == 1));
        order.defuse();
        lock_till(till);
        let _accounted = (barista, server, customer, history);
    }

    /// P6 with the refund in the flat white's slot (path 3, SPEC.md §6:
    /// "P7 replaces the flat white in P6"): the 380 p refund is handed over
    /// on the tray, with the receipt still required (REQ-017).
    ///
    /// Verifies: REQ-017
    #[test]
    fn hand_over_takes_the_refund_in_the_flat_whites_slot() {
        let server = new_person::<600_000>();
        let (cash, customer) = tender_cash(new_customer());
        let (server, till, receipt, customer) =
            take_payment::<300, 0, 700, 600_000, _>(server, open_till(), cash, customer);
        let (server, till, refund) = refund_flat_white::<320, 700, 600_000>(server, till);
        let tea: PotOfTea<303> = PotOfTea::mint(new_teapot(), new_cup());
        let (server, order, history) = hand_over::<303, 600_000, _, _>(
            server,
            refund,
            tea,
            new_tray(),
            receipt,
            new_history(),
            new_history(),
        );
        assert_eq!(Till::<320>::VALUE + FLAT_WHITE_PRICE_PENCE, 700);
        order.defuse();
        lock_till(till);
        let _accounted = (server, customer, history);
    }

    /// P7: the refund draw conserves the till (SPEC.md P7:
    /// 700 = 320 + 380), with the server back (R2).
    #[test]
    fn refund_flat_white_draws_the_till_down() {
        let server = new_person::<600_000>();
        let till: Till<700> = Till::mint();
        let (server, till, refund) = refund_flat_white::<320, 700, 600_000>(server, till);
        assert_eq!(Money::<FLAT_WHITE_PRICE_PENCE>::VALUE + Till::<320>::VALUE, 700);
        refund.defuse();
        lock_till(till);
        let _reusable = server;
    }

    /// P8 (path 3 only): the stranded shot is fed to the drain INSIDE the
    /// process (SPEC.md P8 waste routing — the drain discarded the 36 g,
    /// F-029, so the mass is recovered from the constants, R7), the cup goes
    /// back to the stack, and the barista comes back (R2; the 30 000 ms is
    /// drawn by the adjacent process in the flow, F-048).
    #[test]
    fn drain_stranded_shot_feeds_the_drain_and_restacks_the_cup() {
        let barista = qualify::<MachineTraining, 600_000>(new_person::<600_000>());
        let shot: EspressoShot<36> = EspressoShot::mint();
        let cup: Cup = Cup::mint();
        let (barista, drain) = drain_stranded_shot::<36, 600_000, _>(barista, shot, cup, new_drain());
        assert_eq!(EspressoShot::<36>::VALUE, 36);
        let _accounted = (barista, drain);
    }

    /// The teabag box supplies through its boundary (R12): the count is the
    /// list's length, so count and contents cannot drift.
    #[test]
    fn the_teabag_box_supplies_through_its_boundary() {
        let teabags = new_teabag_box();
        assert_eq!(FreshTeabagBox::BAGS, 10);
        let (bag, teabags) = take_one(teabags);
        assert_eq!(TeabagBox::<NineTeabags>::BAGS, 9);
        let Teabag { _seal: _ } = bag;
        let _box_stays_at_the_counter = teabags;
    }

    /// An abandoned tray is caught by its tripwire at test time (R1 layer 2,
    /// F-008): no compile-time layer sees a named-and-used binding that never
    /// reaches the order.
    #[test]
    #[should_panic(expected = "resource leak: Tray dropped without being consumed")]
    fn abandoned_tray_trips_the_tripwire() {
        let tray: Tray = Tray::mint();
        let _never_assembled = tray;
    }
}
