//! The works subsystem's sealed resource family (R1), its creation/exit
//! boundary (R12) and its processes (F-031) — SPEC §3–§5.
//!
//! Layout per F-006/F-031: this module holds the works' own sealed types —
//! the GBP account, the housings and their rack, the instrument, the
//! packaging bin, the customer and their order — with the creation/exit
//! [`boundary`] and the [`processes`] as child modules.
//!
//! ## The account holds real supply-sealed cash (the crate edge at work)
//!
//! [`Account`] is a works-owned container whose balance **is** a
//! `cs5-supply` [`Money`] value held by value: drawing and depositing are
//! compositions of supply's public conserving combinators (`split_money`,
//! `combine_money`), so the works moves money without ever being able to
//! mint it (EXP-08/F-005 — the CS-4 stores/line pattern applied to
//! currency). The balance and the held cash cannot drift: they are the same
//! const parameter.
//!
//! ## Kept items are untripwired (F-040)
//!
//! The [`Housing`] and the supply crate's `InspectedComponent` are kept-item
//! types: held by the rack, then by the [`Instrument`] that the customer
//! finally consumes. Both are `no_tripwire` per F-040 (a tripwired payload
//! would be silently defused by its container's sanctioned forget);
//! whole-value discard is still caught by `#[must_use]`, and the instrument
//! itself is tripwired.

use crate::characteristics::{ProofOfOrder, sealed};
// One line on purpose: the traceability grep (F-021) skips `use` lines, but
// only when the line itself starts with `use`.
use crate::requirements::{assert_req025, assert_req026};
use core::marker::PhantomData;
use cs5_supply::characteristics::GoodsInInspected;
use cs5_supply::resources::{Money, Packaging};
use model_core::boundary::{Consumer, Supplier};
use model_core::list::{Cons, Len, Nil};
use model_core::nat::Succ;
use model_core::nat::aliases::N5;

// ---------------------------------------------------------------------------
// The works' quantities (SPEC §3/§6). Independent declarations on purpose:
// the asserts in `processes` are real checks, not derivations, and the
// closing-balance arithmetic is stated once as a top-level const item so it
// is editor-visible (F-001 extension).
// ---------------------------------------------------------------------------

/// The works account's opening float, in pence (SPEC §3: 5000 p).
pub const OPENING_FLOAT_PENCE: u64 = 5000;
/// The pence exchanged at the bureau for the component purchase (SPEC P1).
pub const EXCHANGED_PENCE: u64 = 2000;
/// The customer's price for the instrument, in pence (SPEC §3: 9000 p,
/// taken on delivery — REQ-026).
pub const SALE_PRICE_PENCE: u64 = 9000;
/// The account's closing balance (SPEC §4: 5000 − 2000 + 9000 = 12 000 p).
pub const CLOSING_BALANCE_PENCE: u64 = 12_000;

// The GBP books stated once, checked at compile time (editor-visible).
const _: () = assert!(OPENING_FLOAT_PENCE - EXCHANGED_PENCE + SALE_PRICE_PENCE == CLOSING_BALANCE_PENCE);

/// The instrument's mass, in grams (SPEC §3: 600 g housing + 400 g
/// component = 1000 g).
pub const INSTRUMENT_G: u64 = 1000;

// ---------------------------------------------------------------------------
// Money at rest: the works' GBP account (R15/R19 over the crate edge).
// ---------------------------------------------------------------------------

/// The works' GBP account holding `PENCE` pence (R15 quantity container
/// applied to money, R19; SPEC §3: 5000 p at flow start, 12 000 p at flow
/// end). Works-sealed, but its balance **is** supply-sealed cash held by
/// value — see the module docs. No `Drop` of its own: an abandoned account
/// trips the held cash's tripwire at test time (R1 layer 2), naming the
/// amount (F-027).
#[must_use = "Account is a conserved resource: draw on it, deposit into it, or settle up at the boundary"]
pub struct Account<const PENCE: u64> {
    cash: Money<PENCE>,
    _seal: (),
}

impl<const PENCE: u64> Account<PENCE> {
    /// The account's balance, in pence (R7) — identically the held cash's
    /// magnitude.
    pub const BALANCE_PENCE: u64 = PENCE;

    /// Test fixture: an account from nowhere (holding fixture cash), for
    /// downstream test code only (R1, F-004; `test-support` feature,
    /// dev-dependencies only — feature unification also enables supply's
    /// fixtures, which mint the held cash).
    #[cfg(feature = "test-support")]
    pub fn test_fixture() -> Self {
        Account {
            cash: Money::test_fixture(),
            _seal: (),
        }
    }
}

// ---------------------------------------------------------------------------
// The housings, their rack, and the instrument (SPEC §3).
// ---------------------------------------------------------------------------

model_core::consumable_resource! {
    /// An instrument housing (SPEC §3: 600 g, see [`Housing::MASS_G`]).
    /// Deliberately **not** tripwired (`no_tripwire`, F-040): housings are
    /// *kept* — by the rack, then by the instrument — and a tripwired item
    /// inside a consumed container would be silently defused. Whole-value
    /// discard is still caught by `#[must_use]`.
    Housing,
    must_use = "Housing is a conserved resource: fit it into an instrument or keep it racked",
    no_tripwire
}

impl Housing {
    /// The housing's mass, in grams (R7; SPEC §3).
    pub const MASS_G: u64 = 600;
}

/// The housing rack: a supplier at the system boundary (R12) holding real
/// [`Housing`] values in a type-level list — the count **is** the list's
/// length (SPEC §3: works rack, 3 housings). SPEC §6's one genuine ordering
/// freedom hangs off this rack: the housing can be picked at any point
/// before P6.
///
/// Placeholder: works housing stock — goods-in for housings is out of scope
/// (SPEC §1), the stock is simply there.
#[must_use = "HousingRack is a boundary resource: pass it on like any other resource"]
pub struct HousingRack<Items>(Items);

/// A type-level list of exactly three housings — the rack at flow start.
pub type ThreeHousings = Cons<Housing, Cons<Housing, Cons<Housing, Nil>>>;
/// Two housings — the rack after CS-5's one assembly (SPEC §6: "rack at 2").
pub type TwoHousings = Cons<Housing, Cons<Housing, Nil>>;

/// The rack as stocked: 3 housings of 600 g (SPEC §3).
pub type FullHousingRack = HousingRack<ThreeHousings>;

/// `Supplier` is implemented ONLY for a non-empty rack (R12): a fourth
/// housing is a compile error with model-core's modeller-phrased message
/// (F-015).
impl<T> Supplier for HousingRack<Cons<Housing, T>> {
    type Item = Housing;
    type Next = HousingRack<T>;
    fn supply(self) -> (Housing, HousingRack<T>) {
        let Cons(head, tail) = self.0;
        (head, HousingRack(tail))
    }
}

impl<Items: Len> HousingRack<Items> {
    /// How many housings the rack holds — the length of its contents list,
    /// so count and contents cannot disagree (R7, R12).
    pub const COUNT: u64 = Items::LEN;
}

model_core::consumable_resource! {
    /// The finished instrument (SPEC §3): a housing fitted with an inspected
    /// precision component, `G` grams in all (1000 = 600 + 400, conserved at
    /// compile time by [`processes::assemble`]). The housing and the
    /// component are conserved as real objects (R13), held inside as sealed
    /// payload — untripwired kept items (F-040). Tripwired itself; defused
    /// only when the [`Customer`] takes delivery (REQ-026).
    Instrument<C: GoodsInInspected, const G: u64> {
        parts: (Housing, C),
    },
    must_use = "Instrument is the product: deliver it to the customer against their order (REQ-026)"
}

// ---------------------------------------------------------------------------
// The packaging bin (SPEC §3: capacity 5; the F-034 shape).
// ---------------------------------------------------------------------------

/// The works bin for stripped packaging: a **contents-keeping consumer**
/// with decreasing type-level space (SPEC §3: capacity 5) and the real
/// packaging kept in its contents list (F-016). The decreasing space
/// parameter is a hard rule, not style: a contents-keeping consumer without
/// one diverges trait resolution and, at this workspace's mandated recursion
/// limit, crashes the compiler (F-034). The kept packaging is `cs5-supply`'s
/// tripwired waste, so P8's disposal crosses the crate edge: the bin is
/// opened here and its contents fed to supply's disposal sink — the sealed
/// F-039 path.
///
/// (The defaulted `Contents` parameter is subject to the F-017 blanket-impl
/// audit rule: any blanket impl mentioning `Bin` must spell both
/// parameters.)
#[must_use = "Bin is a boundary resource: pass it on like any other resource"]
pub struct Bin<Space, Contents = Nil> {
    contents: Contents,
    _space: PhantomData<Space>,
}

/// The bin as it stands at flow start and after P8: full space, no contents.
pub type EmptyBin = Bin<N5, Nil>;

/// `Consumer` is implemented ONLY while space remains (R12, F-034): space
/// goes down by one and the real packaging is kept at the front of the
/// contents list (F-016). A sixth shipment's packaging takes the
/// modeller-phrased trait-bound path (F-015).
impl<S, C, const G: u64> Consumer<Packaging<G>> for Bin<Succ<S>, C> {
    type Next = Bin<S, Cons<Packaging<G>, C>>;
    fn consume(self, item: Packaging<G>) -> Self::Next {
        Bin {
            contents: Cons(item, self.contents),
            _space: PhantomData,
        }
    }
}

impl<Space, Contents: Len> Bin<Space, Contents> {
    /// How many pieces of packaging the bin holds — the length of its
    /// contents list, so count and contents cannot disagree (R7, R12).
    pub const HELD: u64 = Contents::LEN;
}

// ---------------------------------------------------------------------------
// The customer and their order (REQ-026; SPEC §3/§4).
// ---------------------------------------------------------------------------

model_core::consumable_resource! {
    /// The customer's order: the sealed evidence token REQ-026 turns on
    /// (SPEC §3: "placed → fulfilled" — fulfilment is its consumption at
    /// delivery, one type per state R9). Placed only at the boundary
    /// ([`boundary::place_order`]), consumed exactly once, by
    /// [`processes::deliver`]. Tripwired: an order that is never fulfilled
    /// fails the test that abandoned it.
    CustomerOrder,
    must_use = "CustomerOrder is REQ-026's key: deliver against it - do not discard it"
}

impl sealed::Sealed for CustomerOrder {}
impl ProofOfOrder for CustomerOrder {
    fn fulfil(self, _permit: FulfilPermit) {
        // Conserving consumption (F-054): the placed order ends as the
        // completed delivery that `deliver`'s outputs embody.
        self.defuse();
    }
}

/// The order under its requirement-facing name; the alias carries the tag
/// because a tag inside the macro invocation above would be silently dropped
/// by trace.sh (F-037).
///
/// Satisfies: REQ-026
pub type CustomerOrderToken = CustomerOrder;
model_core::satisfies!(assert_req026, CustomerOrderToken);

/// The inspected component under its works-facing, requirement-facing name
/// (the satisfying type is `cs5-supply`'s — the cross-crate R10 link; tag on
/// the alias per F-037).
///
/// Satisfies: REQ-025
pub type InspectedPrecisionComponent = cs5_supply::resources::InspectedComponent;
model_core::satisfies!(assert_req025, InspectedPrecisionComponent);

/// The permit gating [`ProofOfOrder::fulfil`] (the F-054 permit-gated
/// consumption pattern): a private field and no public constructor, so only
/// [`processes::deliver`] (inside this privacy boundary) can close an order
/// — the characteristic can never be used to vanish an order outside the
/// delivery that accounts for it.
pub struct FulfilPermit {
    pub(crate) _seal: (),
}

/// The customer at the system boundary: places the order, pays on delivery,
/// and takes the instrument (`Next = Self` consumer — an unbounded boundary
/// object, legal only at the boundary, R15/F-029).
///
/// **Deliberately unrefined** (SPEC §4): the customer keeps the standard
/// placeholder form so the refined vendor in `cs5-supply` has its second
/// contrast.
///
/// Placeholder: the customer — their hinterland (what they do with the
/// instrument, where their money comes from) stays at the boundary.
#[must_use = "Customer is a boundary object: pass it on or return it to the caller"]
pub struct Customer {
    _seal: (),
}

/// The customer accepts the delivered instrument; `Next = Self` (R15,
/// F-029: an unbounded consumer discards — here, "takes it home"). The
/// instrument's kept parts are untripwired (F-040), so the sanctioned defuse
/// skips no tripwire.
impl<C: GoodsInInspected, const G: u64> Consumer<Instrument<C, G>> for Customer {
    type Next = Customer;
    fn consume(self, item: Instrument<C, G>) -> Customer {
        item.defuse(); // sanctioned consumer role (F-008); unbounded sink discards (F-029)
        self
    }
}

/// The creation and exit boundary of the works family (R12, F-006): the only
/// production code where the account, the rack, the bin, the customer and
/// the order come into existence, and where the account's cash and the
/// customer's payment cross to and from supply's GBP boundary.
pub mod boundary {
    use super::{Account, Bin, Cons, Customer, CustomerOrder, EmptyBin, FullHousingRack, Housing, HousingRack, Money, Nil, OPENING_FLOAT_PENCE, PhantomData, SALE_PRICE_PENCE};
    use cs5_supply::resources::boundary::{gbp_enters_the_model, gbp_leaves_the_model};

    /// The works account opens with its float (R12; SPEC §3: 5000 p). The
    /// balance is real supply-sealed cash, sourced at supply's GBP boundary
    /// and held by value — the works wraps it, it cannot mint it.
    ///
    /// Placeholder: the works' bank — the float is simply there.
    pub fn open_the_books() -> Account<OPENING_FLOAT_PENCE> {
        Account {
            cash: gbp_enters_the_model(),
            _seal: (),
        }
    }

    /// The account settles up at flow end (R12): the boundary exit for
    /// [`Account`] at any balance — the held cash leaves through supply's
    /// GBP boundary sink (F-035).
    ///
    /// Placeholder: the works' bank — an unbounded sink for banked takings.
    pub fn settle_up<const PENCE: u64>(account: Account<PENCE>) {
        let Account { cash, _seal: () } = account;
        gbp_leaves_the_model(cash);
    }

    /// The customer arrives (R12).
    ///
    /// Placeholder: the customer — deliberately unrefined (SPEC §4).
    pub fn new_customer() -> Customer {
        Customer { _seal: () }
    }

    /// The customer places their order (R12; SPEC §4: an input at the
    /// boundary) — REQ-026's key enters the model with them.
    pub fn place_order(customer: Customer) -> (CustomerOrder, Customer) {
        (CustomerOrder::mint(), customer)
    }

    /// The customer presents payment on delivery (R12; SPEC §3: 9000 p).
    /// Real supply-sealed cash from supply's GBP boundary: the works takes
    /// it, it cannot mint it.
    pub fn present_payment(customer: Customer) -> (Money<SALE_PRICE_PENCE>, Customer) {
        (gbp_enters_the_model(), customer)
    }

    /// The fill function (R12): the stocked housing rack enters the model —
    /// three real housings come into existence here and nowhere else.
    ///
    /// Placeholder: works housing stock (SPEC §1: housing procurement out of
    /// scope).
    pub fn stock_the_rack() -> FullHousingRack {
        HousingRack(Cons(
            Housing::mint(),
            Cons(Housing::mint(), Cons(Housing::mint(), Nil)),
        ))
    }

    /// An empty packaging bin with its five spaces (R12; SPEC §3). Creating
    /// an empty consumer brings no resources into existence, so this is an
    /// ordinary public boundary function.
    pub fn new_bin() -> EmptyBin {
        Bin {
            contents: Nil,
            _space: PhantomData,
        }
    }
}

/// The works' processes P1 and P4–P8 (R1, R2; SPEC §5): pure by-value
/// transformations, composing the conserving transforms the supply and
/// logistics crates export (the CS-4 cross-crate pattern). They mint
/// quantity-bearing values (the instrument; the account's successor states),
/// so they live inside the resource family's module (F-031).
///
/// The operator is threaded loosely through the flow (R9, F-024); per F-048
/// their time is drawn by **adjacent** `draw_time` processes in the flow
/// (never inside these), each draw's labour recorded into the single
/// `History` attributed to the process name (R16). P3 has no draw: the
/// courier and vendor are the actors (SPEC §5).
pub mod processes {
    use super::{Account, Bin, Consumer, EmptyBin, FulfilPermit, GoodsInInspected, Housing, Instrument, Money, Nil, Packaging, PhantomData};
    // One line on purpose: the traceability grep (F-021) skips `use` lines,
    // but only when the line itself starts with `use`.
    use crate::requirements::{Req025InspectedBeforeFitting, Req026DeliveryAgainstTheOrder};
    use cs5_supply::resources::processes::{combine_money, open_and_inspect, split_money};
    use cs5_supply::resources::{BoxedComponent, InspectedComponent, PackagingLoad};
    use model_core::boundary::{ConsumeList, send_list, send_to};

    /// P1 (account side) — draws `TAKE` pence from the account, leaving
    /// `LEFT` (R15 pattern over the crate edge: the draw opens the works'
    /// own container and splits the held supply-sealed cash with supply's
    /// conserving `split_money`, so nothing is minted here). Overspending is
    /// the standard overdraw compile error — no `LEFT` exists with
    /// `TAKE + LEFT == FULL` (E0080 at monomorphization, invisible to
    /// `cargo check`, F-001). SPEC P1 states the flow's draw: 2000 p,
    /// 5000 → 3000 (assert).
    pub fn draw_cash<const TAKE: u64, const LEFT: u64, const FULL: u64>(
        account: Account<FULL>,
    ) -> (Money<TAKE>, Account<LEFT>) {
        let Account { cash, _seal: () } = account;
        // The conservation assert (TAKE + LEFT == FULL) fires inside
        // split_money at this instantiation (R3/R19).
        let (taken, left) = split_money::<FULL, TAKE, LEFT>(cash);
        (
            taken,
            Account {
                cash: left,
                _seal: (),
            },
        )
    }

    /// P5 — goods-in inspection at the UK site (REQ-025; SPEC P5). Composes
    /// supply's conserving `open_and_inspect` (the mass balance
    /// 450 = 400 + 50 is const-asserted there, firing at this
    /// instantiation, F-001) and feeds the stripped packaging to the works
    /// bin **inside the process** (SPEC P5's waste routing) — stripped
    /// packaging never exists loose in a flow. Produces the fit-ready state
    /// REQ-025 turns on (one type per state, R9). The operator's 60 000 ms
    /// is drawn by the adjacent `draw_time` in the flow (F-048).
    ///
    /// Satisfies: REQ-025
    pub fn inspect<const PACK: u64, B: Consumer<Packaging<PACK>>>(boxed: BoxedComponent, bin: B) -> (InspectedComponent, B::Next) {
        let (component, packaging) = open_and_inspect::<PACK>(boxed);
        let bin = send_to(bin, packaging);
        (component, bin)
    }

    /// P6 — assembles the instrument (REQ-025; SPEC P6): a housing from the
    /// rack plus an **inspected** component (style A: the requirement bounds
    /// the component, so a still-boxed part fails with the REQ-phrased
    /// message, F-044 — pinned by
    /// `tests/ui/fit_uninspected_component.rs`). Mass is conserved at
    /// compile time (`600 + 400 == G`, SPEC P6's 1000 g); the housing and
    /// component continue as real kept objects inside the instrument (R13,
    /// F-040). The operator's 180 000 ms is drawn by the adjacent
    /// `draw_time` in the flow (F-048).
    ///
    /// The assembly mass assert — an instrument cannot weigh more than its
    /// parts (600 + 400 ≠ 1100); an E0080 at monomorphization (F-001):
    ///
    /// ```compile_fail
    /// use cs5_supply::resources::InspectedComponent;
    /// use cs5_works::resources::Housing;
    /// use cs5_works::resources::processes::assemble;
    ///
    /// let housing = Housing::test_fixture();
    /// let component = InspectedComponent::test_fixture();
    /// let instrument = assemble::<1100, _>(housing, component);
    /// ```
    ///
    /// Satisfies: REQ-025
    pub fn assemble<const G: u64, C: Req025InspectedBeforeFitting>(housing: Housing, component: C) -> Instrument<C, G> {
        const {
            assert!(
                Housing::MASS_G + <C as GoodsInInspected>::MASS_G == G,
                "mass conservation violated in assemble (R3, P6): the instrument must weigh exactly its housing plus its inspected component (600 + 400 = 1000)"
            )
        };
        // Conserving transform: both parts continue as real objects inside
        // the instrument (R13) — mint is the conserving combinator, only
        // wrapping values passed in by value.
        Instrument::mint((housing, component))
    }

    /// P7 — delivers the instrument and takes payment (REQ-026; SPEC P7):
    /// consumes the instrument, the customer's **order token** (REQ-026's
    /// key — style A: the requirement bounds the order, so any other
    /// paperwork fails with the REQ-phrased message, F-044, pinned by
    /// `tests/ui/deliver_without_the_order.rs`) and the customer's `PAID`
    /// pence; the instrument goes to the customer and the payment is
    /// deposited (the flow states 3000 → 12 000). The operator's 120 000 ms
    /// is drawn by the adjacent `draw_time` in the flow (F-048).
    ///
    /// **The account deposit assert** — claiming 12 001 p after banking
    /// 9000 p into 3000 p must not compile (an E0080 at monomorphization,
    /// F-001; note the composed-process echo whose instantiation note points
    /// at the `combine_money` call inside, F-051):
    ///
    /// ```compile_fail
    /// use cs5_supply::resources::{InspectedComponent, Money};
    /// use cs5_works::resources::boundary::{new_customer, place_order};
    /// use cs5_works::resources::processes::{assemble, deliver};
    /// use cs5_works::resources::{Account, Housing};
    ///
    /// let instrument = assemble::<1000, _>(Housing::test_fixture(), InspectedComponent::test_fixture());
    /// let (order, customer) = place_order(new_customer());
    /// let account = Account::<3000>::test_fixture();
    /// let (account, customer): (Account<12_001>, _) =
    ///     deliver(instrument, order, Money::<9000>::test_fixture(), customer, account);
    /// ```
    ///
    /// Satisfies: REQ-026
    pub fn deliver<const PAID: u64, const BAL: u64, const NEW: u64, const IG: u64, C: GoodsInInspected, O: Req026DeliveryAgainstTheOrder, K: Consumer<Instrument<C, IG>>>(instrument: Instrument<C, IG>, order: O, payment: Money<PAID>, customer: K, account: Account<BAL>) -> (Account<NEW>, K::Next) {
        const {
            assert!(
                BAL + PAID == NEW,
                "money conservation violated in deliver (R3/R19, P7): the new balance must equal the old balance plus the payment taken on delivery - the flow states 3000 + 9000 = 12 000"
            )
        };
        // REQ-026: the delivery is keyed on the customer's order, consumed
        // here through its permit-gated fulfilment (F-054).
        order.fulfil(FulfilPermit { _seal: () });
        // The instrument goes to the customer through the generic access
        // process (F-015).
        let customer = send_to(customer, instrument);
        // Payment on delivery: deposited by supply's conserving combine —
        // the BAL + PAID == NEW assert fires there too (R3/R19).
        let Account { cash, _seal: () } = account;
        let cash = combine_money::<BAL, PAID, NEW>(cash, payment);
        (
            Account {
                cash,
                _seal: (),
            },
            customer,
        )
    }

    /// P8 — empties the bin to disposal (SPEC P8; the F-039 sealed path
    /// across the crate edge): the bin is opened here (its contents are
    /// supply's tripwired packaging) and every piece is fed to supply's
    /// disposal sink through the recursive `ConsumeList`; the emptied bin
    /// returns with its full five spaces. The mass books are asserted via
    /// the contents (SPEC P8: 50 = 50) against supply's sealed
    /// [`PackagingLoad`], so the stated disposal total cannot be forged. The
    /// operator's 30 000 ms is drawn by the adjacent `draw_time` in the
    /// flow (F-048).
    ///
    /// The bin-emptying mass assert — the bin cannot come back light
    /// (claiming 40 g for the held 50 g):
    ///
    /// ```compile_fail
    /// use cs5_supply::resources::boundary::new_packaging_disposal;
    /// use cs5_supply::resources::Packaging;
    /// use cs5_works::resources::boundary::new_bin;
    /// use cs5_works::resources::processes::empty_bin;
    /// use model_core::boundary::send_to;
    ///
    /// let bin = send_to(new_bin(), Packaging::<50>::test_fixture());
    /// let (bin, disposal) = empty_bin::<40, _, _, _>(bin, new_packaging_disposal());
    /// ```
    pub fn empty_bin<const STATED_G: u64, Space, Contents: PackagingLoad, D: ConsumeList<Contents>>(bin: Bin<Space, Contents>, disposal: D) -> (EmptyBin, D::Next) {
        const {
            assert!(
                <Contents as PackagingLoad>::GRAMS == STATED_G,
                "packaging mass reconciliation violated in empty_bin (R3, P8): the bin's contents must weigh exactly the stated disposal total (the flow states 50 g)"
            )
        };
        let Bin {
            contents,
            _space: PhantomData,
        } = bin;
        // F-039: the kept, tripwired packaging reaches its sealed disposal
        // exit; the recursive feed is one consume per piece (F-014).
        let disposal = send_list(disposal, contents);
        (
            Bin {
                contents: Nil,
                _space: PhantomData,
            },
            disposal,
        )
    }
}

#[cfg(test)]
mod tests {
    //! Per-process unit tests (R5): every works process turns specific
    //! inputs into the expected outputs with nothing left unaccounted for.
    //! These tests sit inside the privacy boundary, so they may mint
    //! fixtures and defuse outputs directly; the full flows (both orderings)
    //! are exercised in `tests/flows.rs`.

    use super::boundary::{new_bin, new_customer, open_the_books, place_order, present_payment, settle_up, stock_the_rack};
    use super::processes::{assemble, deliver, draw_cash, empty_bin, inspect};
    use super::{Account, Bin, Customer, EmptyBin, FullHousingRack, HousingRack, Instrument, TwoHousings};
    use cs5_supply::resources::boundary::new_packaging_disposal;
    use cs5_supply::resources::{BoxedComponent, InspectedComponent, Money, Packaging};
    use model_core::boundary::take_one;
    use model_core::list::{Cons, Nil};
    use model_core::nat::aliases::N4;

    /// P1 (account side): the draw splits the held cash exactly
    /// (5000 = 2000 + 3000, assert inside supply's split) and the balance is
    /// the type (SPEC P1).
    #[test]
    fn draw_cash_draws_down_the_account() {
        let account = open_the_books();
        assert_eq!(Account::<5000>::BALANCE_PENCE, 5000);
        let (cash, account): (Money<2000>, Account<3000>) = draw_cash(account);
        assert_eq!(Money::<2000>::VALUE + Account::<3000>::BALANCE_PENCE, 5000);
        cs5_supply::resources::boundary::gbp_leaves_the_model(cash);
        settle_up(account);
    }

    /// P5: inspection strips the packaging into the bin inside the process
    /// (450 = 400 + 50, assert inside supply's transform) and produces the
    /// fit-ready state (REQ-025).
    ///
    /// Verifies: REQ-025
    #[test]
    fn inspect_feeds_the_bin_inside_the_process() {
        let (component, bin) = inspect::<50, _>(BoxedComponent::test_fixture(), new_bin());
        assert_eq!(Bin::<N4, Cons<Packaging<50>, Nil>>::HELD, 1);
        let _fit_ready_kept_item = component;
        // The kept packaging is tripwired supply waste: even a per-process
        // test must route it through the sealed disposal path (F-039).
        let (bin, disposal) = empty_bin::<50, _, _, _>(bin, new_packaging_disposal());
        let _stay_at_the_boundary = (bin, disposal);
    }

    /// P6: assembly conserves mass (600 + 400 = 1000, assert) and keeps both
    /// parts as real objects inside the instrument (R13, F-040); P7 delivers
    /// it against the order with payment taken (REQ-026; 3000 → 12 000,
    /// assert).
    ///
    /// Verifies: REQ-025, REQ-026
    #[test]
    fn assemble_and_deliver_balance_mass_and_money() {
        let rack = stock_the_rack();
        assert_eq!(FullHousingRack::COUNT, 3);
        let (housing, rack) = take_one(rack);
        let instrument: Instrument<InspectedComponent, 1000> =
            assemble(housing, InspectedComponent::test_fixture());
        let (order, customer) = place_order(new_customer());
        let (payment, customer) = present_payment(customer);
        let account = open_the_books();
        let (cash, account): (Money<2000>, Account<3000>) = draw_cash(account);
        cs5_supply::resources::boundary::gbp_leaves_the_model(cash);
        let (account, customer): (Account<12_000>, Customer) =
            deliver(instrument, order, payment, customer, account);
        assert_eq!(Account::<12_000>::BALANCE_PENCE, 12_000);
        assert_eq!(HousingRack::<TwoHousings>::COUNT, 2);
        settle_up(account);
        let Customer { _seal: () } = customer;
        let _rack_keeps_two_untripwired_housings = rack;
    }

    /// P8: the bin is opened, its packaging reaches disposal across the
    /// crate edge (50 = 50, assert via contents), and the emptied bin comes
    /// back with all five spaces.
    #[test]
    fn empty_bin_disposes_the_packaging_and_restores_the_space() {
        let (component, bin) = inspect::<50, _>(BoxedComponent::test_fixture(), new_bin());
        let (bin, disposal) = empty_bin::<50, _, _, _>(bin, new_packaging_disposal());
        let empty: EmptyBin = bin;
        assert_eq!(EmptyBin::HELD, 0);
        let _kept = component;
        let _stay_at_the_boundary = (empty, disposal);
    }

    /// An abandoned customer order is caught by its tripwire at test time
    /// (R1 layer 2, F-008): REQ-026's key must be fulfilled, never dropped.
    #[test]
    #[should_panic(expected = "CustomerOrder dropped without being consumed")]
    fn an_abandoned_order_trips_the_tripwire() {
        let (order, customer) = place_order(new_customer());
        let Customer { _seal: () } = customer;
        let _placed_but_never_fulfilled = order;
    }
}
