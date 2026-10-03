//! Exercise 06 — capstone: model making toast
//! (belongs to Tutorial 06, "Capstone: model making toast". The tutorial
//! carries the full mini-spec; the summary is repeated here.)
//!
//! THE MINI-SPEC (TOAST-1, abridged):
//! * One person makes two slices of toast in a kitchen. Boundary: the
//!   kitchen — grid electricity enters; toast, water vapour and waste heat
//!   leave.
//! * REQ-902: all waste heat must be accounted to the kitchen-air sink.
//! * Resources: bread slice (discrete, 40 g, bag of four); toast slice
//!   (36 g, carries 25 000 J embodied); toaster (reusable, needs no person
//!   while it runs); electricity (60 000 J drawn); water vapour (8 g, waste);
//!   waste heat (10 000 J, waste); person (120 000 ms budget).
//! * P1 toast two slices — consumes 2 bread slices + 60 000 J; produces
//!   2 toast slices; waste: 8 g vapour and 10 000 J heat → kitchen air.
//!   Balances: mass 40 + 40 = 36 + 36 + 8 (assert); energy
//!   60 000 = 25 000 + 25 000 + 10 000 (assert). Actor: person (draws
//!   60 000 ms, adjacent draw recorded to the History, F-048).
//! * Everything accounted at flow end: toast at the eater, vapour and heat at
//!   the kitchen air, bread bag back at 2, person back with 60 000 ms, the
//!   History holding the one recorded draw.
//!
//! GOAL: finish the model. The file has THREE deliberate holes, marked
//! `// TODO (fix N of 3)`. You will meet them **in order** — each one fails
//! at a different layer, which is the point of the whole course:
//!
//! 1. a missing characteristic impl → `error[E0277]`, the REQ-phrased
//!    requirement message (type-check time; your editor shows it);
//! 2. a wrong energy balance → `error[E0080]`, the conservation assert
//!    (monomorphization: `cargo check` and your editor will NOT show it);
//! 3. the toast never reaches the eater → the tripwire panic at TEST time
//!    ("resource leak: … dropped without being consumed") — the layer that
//!    catches what no compile-time check can (F-032).
//!
//! Fix them one at a time until `cargo test --test ex06_capstone_toast`
//! passes.

#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![deny(let_underscore_drop)]
#![recursion_limit = "2048"]

use model_core::boundary::{Consumer, Supplier, send_to, take_n};
use model_core::common::Person;
use model_core::common::boundary::new_person;
use model_core::common::processes::draw_time;
use model_core::history::boundary::new_history;
use model_core::history::processes::record;
use model_core::list::{Cons, Len, Nil};
use model_core::nat::aliases::N2;

// ---------------------------------------------------------------------------
// Resources (TOAST-1 §3): one type per processing state (R9).
// ---------------------------------------------------------------------------

model_core::consumable_resource! {
    /// A slice of bread: a discrete item, its own object (R13; 40 g, see
    /// [`BreadSlice::MASS_G`]). Untripwired (`no_tripwire`, F-040): slices
    /// are *kept* by the bread bag, and a tripwired item inside an abandoned
    /// bag would panic from the bag's own drop, masking the real leak site.
    BreadSlice,
    must_use = "BreadSlice is a conserved resource: toast it or keep it accounted for in the bag",
    no_tripwire
}

impl BreadSlice {
    /// A bread slice's mass, in grams (R7; TOAST-1 §3).
    pub const MASS_G: u64 = 40;
}

model_core::container_resource! {
    /// A slice of toast carrying `V` joules of embodied energy (TOAST-1 §3:
    /// 36 g, 25 000 J). The product; it leaves the model with the eater.
    /// Tripwired (F-008): toast left on the counter fails the test that
    /// leaked it.
    ToastSlice,
    unit = "joules (embodied)",
    must_use = "ToastSlice is the product: hand it to the eater (a Consumer)"
}

impl<const V: u64> ToastSlice<V> {
    /// A toast slice's mass, in grams (R7). (`VALUE` is the embodied energy
    /// in joules.)
    pub const MASS_G: u64 = 36;
}

model_core::container_resource! {
    /// Electrical energy drawn from the grid, in joules (R7; TOAST-1 §3:
    /// 60 000 J drawn).
    Electricity,
    unit = "joules",
    must_use = "Electricity is a conserved resource: pass it on or hand it to a Consumer"
}

model_core::container_resource! {
    /// Waste heat, in joules (R15; TOAST-1 P1: 10 000 J). All of it must be
    /// accounted to the kitchen-air sink (REQ-902).
    WasteHeat,
    unit = "joules",
    must_use = "WasteHeat is a conserved waste product: account it to the kitchen-air sink (REQ-902)"
}

model_core::container_resource! {
    /// Water vapour driven out of the bread, in grams (TOAST-1 P1: 8 g —
    /// where the mass balance's missing grams go).
    WaterVapour,
    unit = "grams",
    must_use = "WaterVapour is a conserved waste product: account it to the kitchen air"
}

model_core::reusable_resource! {
    /// The toaster (R2): moved in and returned by every process that uses
    /// it; needs no person while it runs.
    ///
    /// Placeholder: kitchen setup at flow start — one toaster.
    Toaster,
    must_use = "Toaster is a reusable resource: pass it on or return it to the caller"
}

model_core::reusable_resource! {
    /// The grid socket: the unbounded boundary source energy is drawn from
    /// (R15, F-028).
    ///
    /// Placeholder: grid electricity — assumed unbounded.
    GridSocket,
    must_use = "GridSocket is a boundary resource: pass it on like any other resource"
}

// ---------------------------------------------------------------------------
// Characteristics and the requirement (TOAST-1 §2).
// ---------------------------------------------------------------------------

/// Characteristic (R6): the kitchen-air sink that REQ-902 accounts all waste
/// heat to.
#[diagnostic::on_unimplemented(message = "`{Self}` is not the kitchen-air sink (REQ-902: all waste heat must be accounted to it)", label = "the kitchen-air sink is required here", note = "the kitchen air (`KitchenAir`, a placeholder unbounded sink, R15) is where waste heat leaves the model")]
pub trait KitchenAirSink {}

model_core::requirement! {
    /// REQ-902: All waste heat must be accounted to the kitchen-air sink.
    #[diagnostic::on_unimplemented(message = "waste heat may not go here: `{Self}` is not the kitchen-air sink (REQ-902)", label = "REQ-902: all waste heat must be accounted to the kitchen-air sink", note = "the kitchen air (`KitchenAir`) is the placeholder unbounded sink for waste heat (R15)")]
    pub trait Req902HeatToKitchenAir: (KitchenAirSink);
    assert = assert_req902;
}

// ---------------------------------------------------------------------------
// Boundary objects (TOAST-1 §4).
// ---------------------------------------------------------------------------

/// The bread bag: a supplier at the system boundary (R12) holding real
/// [`BreadSlice`] objects in a type-level list — the count IS the list's
/// length. TOAST-1's bag holds four and comes back at two.
///
/// Placeholder: bread bag — brand/vendor not modelled.
#[must_use = "BreadBag is a boundary resource: pass it on like any other resource"]
pub struct BreadBag<Items>(Items);

/// `Supplier` is implemented ONLY for a non-empty bag (R12).
impl<H, T> Supplier for BreadBag<Cons<H, T>> {
    type Item = H;
    type Next = BreadBag<T>;
    fn supply(self) -> (H, BreadBag<T>) {
        let Cons(head, tail) = self.0;
        (head, BreadBag(tail))
    }
}

impl<Items: Len> BreadBag<Items> {
    /// How many slices the bag holds — the length of its contents list.
    pub const COUNT: u64 = Items::LEN;
}

/// A type-level list of exactly two bread slices — what `SupplyN<N2>` takes
/// and what is left in the bag afterwards.
pub type TwoSlices = Cons<BreadSlice, Cons<BreadSlice, Nil>>;

/// The bag of four enters the model here — the only place bread comes into
/// existence (R12; this file is its own crate, so `mint` is reachable here
/// and only here).
///
/// Placeholder: bread bag — four slices assumed on hand.
pub fn full_bread_bag() -> BreadBag<Cons<BreadSlice, Cons<BreadSlice, TwoSlices>>> {
    BreadBag(Cons(
        BreadSlice::mint(),
        Cons(
            BreadSlice::mint(),
            Cons(BreadSlice::mint(), Cons(BreadSlice::mint(), Nil)),
        ),
    ))
}

/// The kitchen air: the unbounded boundary sink for waste heat (REQ-902) and
/// water vapour. `Next = Self` (R15, F-029): legal only at the boundary, and
/// it necessarily discards what it consumes.
///
/// Placeholder: kitchen air — assumed able to absorb all waste heat and
/// vapour.
///
/// Satisfies: REQ-902
#[must_use = "KitchenAir is a boundary resource: pass it on like any other resource"]
pub struct KitchenAir {
    _seal: (),
}

// SOLVED (fix 1 of 3): `KitchenAir` IS the kitchen-air sink, but the model
// does not say so yet, so the `Satisfies:` tag above is a lie and REQ-902's
// bound rejects it. Attach the characteristic with a one-line impl
// (exercise 02 is the pattern):
impl KitchenAirSink for KitchenAir {}

// Compile-checked backing for the tag above (R10, F-020).
model_core::satisfies!(assert_req902, KitchenAir);

/// The air absorbs waste heat of any magnitude; `Next = Self` (R15).
impl<const E: u64> Consumer<WasteHeat<E>> for KitchenAir {
    type Next = KitchenAir;
    fn consume(self, item: WasteHeat<E>) -> KitchenAir {
        item.defuse(); // sanctioned consumer role (F-008); unbounded sink discards (F-029)
        self
    }
}

/// The air absorbs water vapour of any magnitude; `Next = Self` (R15).
impl<const G: u64> Consumer<WaterVapour<G>> for KitchenAir {
    type Next = KitchenAir;
    fn consume(self, item: WaterVapour<G>) -> KitchenAir {
        item.defuse();
        self
    }
}

/// The eater taking delivery of the toast: an unbounded boundary sink
/// (`type Next = Self`, R15, F-029).
///
/// Placeholder: the eater.
#[must_use = "Eater is a boundary resource: pass them on like any other resource"]
pub struct Eater {
    _seal: (),
}

/// The eater accepts toast, with its mass and embodied energy; `Next = Self`.
impl<const E: u64> Consumer<ToastSlice<E>> for Eater {
    type Next = Eater;
    fn consume(self, item: ToastSlice<E>) -> Eater {
        item.defuse();
        self
    }
}

/// The toaster enters the model (R12).
pub fn new_toaster() -> Toaster {
    Toaster::mint()
}

/// The grid socket enters the model (R12).
pub fn new_grid_socket() -> GridSocket {
    GridSocket::mint()
}

/// The kitchen air enters the model (R12).
pub fn new_kitchen_air() -> KitchenAir {
    KitchenAir { _seal: () }
}

/// The eater enters the model (R12).
pub fn new_eater() -> Eater {
    Eater { _seal: () }
}

/// Draws `TAKE` joules from the grid (R15: an unbounded source of continuous
/// material is a draw-style boundary process, never a `Supplier` impl,
/// F-028). The socket is returned (R2).
///
/// Placeholder: grid electricity — assumed unbounded.
pub fn draw_grid_energy<const TAKE: u64>(socket: GridSocket) -> (Electricity<TAKE>, GridSocket) {
    (Electricity::mint(), socket)
}

// ---------------------------------------------------------------------------
// Processes (TOAST-1 §5).
// ---------------------------------------------------------------------------

/// P1 — toasts two slices (TOAST-1 P1): two bread slices plus `DRAW_J`
/// joules from the grid become two toast slices (each `SLICE_E` embodied),
/// `VAPOUR_G` grams of water vapour and `HEAT_J` of waste heat. **No person
/// while it runs** (the toaster is automatic); the person's 60 000 ms of
/// loading and unloading is an adjacent `draw_time` in the flow (F-048).
///
/// Conservation is one compile-time assert per dimension (R3, R15), with the
/// caller stating the splits (F-022). Both fire at monomorphization (F-001):
/// invisible to `cargo check`, caught by `cargo build`/`cargo test`.
pub fn toast_two_slices<const DRAW_J: u64, const SLICE_E: u64, const HEAT_J: u64, const VAPOUR_G: u64>(
    toaster: Toaster,
    slices: TwoSlices,
    energy: Electricity<DRAW_J>,
) -> (
    Toaster,
    (ToastSlice<SLICE_E>, ToastSlice<SLICE_E>),
    WaterVapour<VAPOUR_G>,
    WasteHeat<HEAT_J>,
) {
    const {
        assert!(
            SLICE_E + SLICE_E + HEAT_J == DRAW_J,
            "energy conservation violated in toast_two_slices (R15): the two slices' embodied energy plus the waste heat must sum exactly to the energy drawn from the grid"
        )
    };
    const {
        assert!(
            BreadSlice::MASS_G + BreadSlice::MASS_G
                == ToastSlice::<SLICE_E>::MASS_G + ToastSlice::<SLICE_E>::MASS_G + VAPOUR_G,
            "mass conservation violated in toast_two_slices (R3): the bread going in must sum exactly to the toast plus the water vapour driven out"
        )
    };
    // Conserving transforms (R1): each slice's mass continues as toast plus
    // vapour; the drawn energy continues as embodied energy plus heat.
    let Cons(s1, Cons(s2, Nil)) = slices;
    let BreadSlice { _seal: _ } = s1;
    let BreadSlice { _seal: _ } = s2;
    energy.defuse();
    (
        toaster,
        (ToastSlice::mint(), ToastSlice::mint()),
        WaterVapour::mint(),
        WasteHeat::mint(),
    )
}

/// Hands one quantity of waste heat to the kitchen-air sink — the process
/// form of REQ-902. Generic over the consumer (R12, F-015): a wrong sink
/// produces the REQ-phrased trait-bound error.
///
/// Satisfies: REQ-902
pub fn vent_heat<const E_J: u64, C: Req902HeatToKitchenAir + Consumer<WasteHeat<E_J>>>(sink: C, heat: WasteHeat<E_J>) -> C::Next {
    sink.consume(heat)
}

// ---------------------------------------------------------------------------
// The flow (TOAST-1 §6).
// ---------------------------------------------------------------------------

/// The toast flow, with everything accounted at flow end.
///
/// Verifies: REQ-902
#[test]
fn the_toast_flow_accounts_for_everything() {
    // The kitchen setup enters at the boundary (TOAST-1 §4).
    let history = new_history();
    let person = new_person::<120_000>();
    let air = new_kitchen_air();

    // Two slices leave the bag of four, one at a time, in one SupplyN bound
    // (R12, F-014); the bag comes back at two.
    let (slices, bag): (TwoSlices, _) = take_n::<N2, _>(full_bread_bag());

    // P1 — toast (no person while it runs): 60 000 J drawn; the balances are
    // stated by the caller and checked by the compiler.
    // SOLVED (fix 2 of 3): the stated energy split is wrong — 2 × 25 000 J of
    // embodied energy plus this much heat is NOT the 60 000 J drawn. Restate
    // the waste heat (TOAST-1 P1 has the numbers). Note `cargo check` will
    // not catch this (F-001) — only `cargo test`/`cargo build` do.
    let (energy, socket) = draw_grid_energy::<60_000>(new_grid_socket());
    let (toaster, (t1, t2), vapour, heat) =
        toast_two_slices::<60_000, 25_000, 10_000, 8>(new_toaster(), slices, energy);

    // The person's loading/unloading time: an adjacent draw, recorded to the
    // History under the process's name (F-048, R16).
    let (labour, person) = draw_time::<60_000, 60_000, 120_000>(person);
    let history = record(history, "toast_two_slices", labour);

    // Waste routing: the heat goes through the REQ-902-bounded vent; the
    // vapour to the same air sink.
    let air = vent_heat(air, heat);
    let air = send_to(air, vapour);

    // SOLVED (fix 3 of 3): the toast must LEAVE the model through the eater —
    // left on the counter, its tripwire fails this test at runtime ("resource
    // leak"). Hand both slices to the eater (`send_to`, twice).
    let eater = send_to(new_eater(), t1);
    let _eater = send_to(eater, t2);

    // Everything accounted at flow end (TOAST-1 §6).
    let _bag_back_at_two: BreadBag<TwoSlices> = bag;
    assert_eq!(BreadBag::<TwoSlices>::COUNT, 2);
    let _person_back: Person<60_000> = person;
    assert_eq!(history.event_count(), 1);
    let _reusables = (toaster, socket, air);
    let _execution_record = history;
}
