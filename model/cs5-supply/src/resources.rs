//! The supply subsystem's sealed resource family (R1), its creation/exit
//! boundary (R12) and its processes (F-031) — SPEC §3–§5.
//!
//! Layout per F-006/F-031: this module holds the sealed types — the two
//! currency dimensions, the bureau, the refined vendor and its stock, the
//! component states, the packaging and the purchase order — with the
//! creation/exit [`boundary`] and the [`processes`] (which mint
//! quantity-bearing values and therefore live inside the privacy boundary)
//! as child modules.
//!
//! **The crate edge carries the seals** (SPEC §3, EXP-08/F-005): every type
//! here has private fields and a `pub(crate)` mint, so `cs5-logistics` and
//! `cs5-works` physically cannot create a euro cent, a pound of cash, a
//! component or a purchase order (E0451/E0624). What they *can* do is
//! compose the conserving processes exported by [`processes`] — the
//! cross-crate instance of R9's "each process step gets a conserving
//! conversion", exactly as CS-4's stores/line split established.
//!
//! ## The two money dimensions (R19, F-051)
//!
//! [`Money`] (pence) and [`Euros`] (euro cents) are separate sealed types:
//! no function anywhere turns one into the other except the bureau's
//! [`processes::exchange`] (REQ-024, boundary-only, F-052). GBP arithmetic
//! ([`processes::split_money`], [`processes::combine_money`]) is public so
//! the works can run its account on real supply-sealed cash held by value —
//! conserving combinators mint nothing the caller did not already hold.

use crate::characteristics::{GoodsInInspected, sealed};
// One line on purpose: the traceability grep (F-021) skips `use` lines, but
// only when the line itself starts with `use`.
use crate::requirements::{assert_req023, assert_req024};
use model_core::boundary::{Consumer, Supplier};
use model_core::list::{Cons, Len, Nil};

// ---------------------------------------------------------------------------
// Money: the two currency dimensions (R19; SPEC §3).
// ---------------------------------------------------------------------------

model_core::container_resource! {
    /// GBP cash in hand, in pence (integer minor units — the R7 money row;
    /// R19). One of CS-5's two currency dimensions: it never meets [`Euros`]
    /// in any arithmetic — the only bridge is the bureau's
    /// [`processes::exchange`] (REQ-024). Enters the model only at the
    /// boundary ([`boundary::gbp_enters_the_model`]) and leaves it only at
    /// the bureau or the boundary sink ([`boundary::gbp_leaves_the_model`]).
    /// Tripwired (F-008): abandoned cash fails the test that leaked it.
    Money,
    unit = "pence",
    must_use = "Money is a conserved resource: pass it on, bank it, or hand it to a Consumer"
}

model_core::container_resource! {
    /// Euro-cent cash in hand (EUR, integer minor units — R19). **A separate
    /// dimension from [`Money`]** (SPEC §3): the two sealed types never mix,
    /// so a cross-currency payment is a type error (E0308), like adding
    /// grams to millimetres. Minted **only** by the bureau's
    /// [`processes::exchange`] (REQ-024) and consumed **only** by the
    /// vendor at its exact price (REQ-023, F-051) — the vendor is this
    /// type's production-legal sink (F-035). Tripwired (F-008).
    Euros,
    unit = "euro cents",
    must_use = "Euros is a conserved resource: remit it to the vendor or hand it to a Consumer"
}

/// The bureau's exchange rate, stated once as an integer ratio (R19/F-052):
/// [`GBP_EUR_RATE_NUM`] euro cents per [`GBP_EUR_RATE_DEN`] pence — 117 ec
/// per 100 p (SPEC §3). A modelling input, not a computed market value;
/// every [`processes::exchange`] is const-asserted against it.
pub const GBP_EUR_RATE_NUM: u64 = 117;
/// Denominator of the stated rate: see [`GBP_EUR_RATE_NUM`].
pub const GBP_EUR_RATE_DEN: u64 = 100;

/// The vendor's listed price for one boxed precision component, in euro
/// cents (SPEC §3: 2340 ec). Stated once (F-051); the vendor's `Consumer`
/// impl exists **only** at this amount, so a wrong payment — wrong amount or
/// wrong currency — is a type-check-time error (REQ-023 structural).
pub const COMPONENT_PRICE_EC: u64 = 2340;

// The spec's amounts are chosen remainder-free (SPEC §7): the works' 2000 p
// exchange exactly at the stated rate into the vendor's exact price. Stated
// as a top-level const item, so it is editor-visible (F-001 extension).
const _: () = assert!(2000 * GBP_EUR_RATE_NUM == COMPONENT_PRICE_EC * GBP_EUR_RATE_DEN);

model_core::reusable_resource! {
    /// The bureau de change at the system boundary (R19): the one place
    /// currency may be exchanged (REQ-024), at its stated integer rate.
    /// Threaded by value like any boundary object and returned unchanged by
    /// [`processes::exchange`].
    ///
    /// **Deliberately unrefined** (SPEC §4): the bureau keeps the standard
    /// placeholder form so the refined [`Vendor`] has its contrast — an
    /// unbounded float in both currencies, exactly what every earlier
    /// boundary organisation looked like.
    ///
    /// Placeholder: bureau de change — assumed unbounded float in both
    /// currencies; refine to a finite float if exchange volume ever matters.
    Bureau,
    must_use = "Bureau is a boundary object: pass it on or return it to the caller"
}

impl sealed::Sealed for Bureau {}
impl crate::characteristics::ExchangeDesk for Bureau {}

/// The bureau under its requirement-facing name; the alias carries the tag
/// because a tag inside the `reusable_resource!` invocation above would be
/// silently dropped by trace.sh (F-037).
///
/// Satisfies: REQ-024
pub type ExchangeBureau = Bureau;
model_core::satisfies!(assert_req024, ExchangeBureau);

// ---------------------------------------------------------------------------
// The goods: component states (one type per state, R9) and packaging.
// ---------------------------------------------------------------------------

model_core::consumable_resource! {
    /// The precision component as the vendor ships it: boxed, 450 g in all
    /// (SPEC §3 — the 400 g component in 50 g of packaging; see
    /// [`BoxedComponent::MASS_G`]). Its own processing state (R9/F-023): a
    /// boxed component cannot be fitted — only
    /// [`processes::open_and_inspect`] produces the fit-ready state
    /// (REQ-025). Exists only inside the vendor's stock until sold (R12).
    /// Tripwired (F-008): a box lost in transit fails the test that leaked
    /// it.
    BoxedComponent,
    must_use = "BoxedComponent is a conserved resource: inspect it at goods-in or hand it to a Consumer"
}

impl BoxedComponent {
    /// The boxed component's mass, in grams (R7; SPEC §3).
    pub const MASS_G: u64 = 450;
}

model_core::consumable_resource! {
    /// The precision component after goods-in inspection at the UK site:
    /// 400 g, fit-ready — the state REQ-025 turns on (one type per state,
    /// R9). Deliberately **not** tripwired (`no_tripwire`, F-040): the
    /// inspected component is *kept* by the instrument that fits it (the
    /// works' assembly holds it as sealed payload), and a tripwired item
    /// inside a consumed instrument would be silently defused anyway.
    /// Whole-value discard is still caught by `#[must_use]`.
    InspectedComponent,
    must_use = "InspectedComponent is a conserved resource: fit it into the instrument or hand it to a Consumer",
    no_tripwire
}

impl InspectedComponent {
    /// The inspected component's mass, in grams (R7; SPEC §3).
    pub const MASS_G: u64 = 400;
}

impl sealed::Sealed for InspectedComponent {}
impl GoodsInInspected for InspectedComponent {
    const MASS_G: u64 = InspectedComponent::MASS_G;
}

model_core::container_resource! {
    /// Shipping packaging stripped at goods-in inspection: `V` grams of
    /// waste per shipment (SPEC §3: 50 g). Routed to the works bin **inside**
    /// the inspection process (SPEC P5) and emptied to [`PackagingDisposal`]
    /// at P8 (F-039). Tripwired (F-008): packaging that never reaches the
    /// bin fails the test that leaked it.
    Packaging,
    unit = "grams",
    must_use = "Packaging is a conserved waste product: feed it to the works bin (SPEC P5) and dispose of it (P8)"
}

model_core::consumable_resource! {
    /// The purchase order the vendor sells against (SPEC P2/P3): sealed
    /// paperwork raised at the supply boundary, consigned to the courier
    /// (only the order travels — the payment is remitted flow-routed,
    /// SPEC §7), presented at the vendor and consumed by the sale. Tripwired
    /// (F-008): an order that never reaches the vendor fails the test that
    /// leaked it.
    ///
    /// Note for the spec author: this token is required by SPEC §5 (P2/P3)
    /// but missing from §3's resource table.
    PurchaseOrder,
    must_use = "PurchaseOrder is the vendor's sale evidence: consign it to the courier and present it at the vendor"
}

// ---------------------------------------------------------------------------
// The vendor: THE refined placeholder (R12, SPEC §4).
// ---------------------------------------------------------------------------

/// The EU vendor organisation at the system boundary — **the refined
/// placeholder** (R12, SPEC §4): a modelled organisation whose stock is a
/// finite, type-level 3-deep supplier of real [`BoxedComponent`]s.
///
/// ## Before: the unbounded placeholder every earlier vendor was
///
/// From the pilot through CS-4, a vendor was the standard `Next = Self`
/// boundary form — correct types, no content, always `/// Placeholder:`:
///
/// ```text
/// /// Placeholder: tool vendor — assumed unbounded stock and unbounded
/// /// appetite for money; refine to a named organisation with finite
/// /// stock later.
/// impl Supplier for Vendor {
///     type Item = Spanner;
///     type Next = Self;                      // never exhausted
///     fn supply(self) -> (Spanner, Vendor) {
///         (Spanner::mint(), self)            // goods minted on demand
///     }
/// }
/// ```
///
/// That form is legal only at the boundary (R15) precisely because it mints
/// goods from nothing and swallows money without bound — an assumption worn
/// as a greppable `Placeholder:` tag.
///
/// ## After: the refined form (this type)
///
/// The R12 refinement path replaces the assumption with a model. The stock
/// is now a type-level list of **real objects the vendor holds**
/// (`Vendor<Cons<BoxedComponent, …>>`): `Supplier` is implemented only while
/// stock remains, so a **fourth purchase from the 3-stock is a compile
/// error** (the F-015 "it is exhausted" message), the count is the list's
/// length and cannot drift, and exhaustion produces a distinct empty-stock
/// state ([`ExhaustedVendor`]) that must itself be accounted for at the
/// boundary ([`boundary::vendor_returns_to_its_hinterland`]). The `supply`
/// impl hands over a held object instead of minting one. No `Placeholder:`
/// tag remains on the stock — only the vendor's *hinterland* (its own
/// sourcing, out of scope per SPEC §1) stays an assumption, carried by the
/// boundary fill and exit below. The [`Bureau`] and the works' customer
/// deliberately keep the **before** form, so both shapes sit side by side in
/// one model.
///
/// The vendor is also both money-consumer and goods-source (R19): it
/// consumes [`Euros`] **only at its exact price** (REQ-023, F-051) and
/// consumes the presented [`PurchaseOrder`]. It keeps its revenue outside
/// the model (a boundary organisation — the consumed euro cents are
/// discarded under the F-029 unbounded-sink licence, with the dimension's
/// books closed arithmetically: the flow states the vendor received exactly
/// 2340 ec).
#[must_use = "Vendor is a boundary organisation: pass it on or return it to the caller"]
pub struct Vendor<Stock> {
    stock: Stock,
    _seal: (),
}

impl<Stock> sealed::Sealed for Vendor<Stock> {}
impl<Stock> crate::characteristics::PricedInEuroCents for Vendor<Stock> {}

/// A type-level list of exactly three boxed components — the vendor's stock
/// as the model opens (SPEC §3: finite!).
pub type ThreeBoxedComponents = Cons<BoxedComponent, Cons<BoxedComponent, Cons<BoxedComponent, Nil>>>;

/// Two boxed components — the stock after CS-5's one purchase (SPEC §6:
/// "vendor stock at 2 with its state accounted").
pub type TwoBoxedComponents = Cons<BoxedComponent, Cons<BoxedComponent, Nil>>;

/// The vendor as the model opens: 3 components in stock.
///
/// Satisfies: REQ-023
pub type StockedVendor = Vendor<ThreeBoxedComponents>;
model_core::satisfies!(assert_req023, StockedVendor);

/// The exhausted vendor: a distinct resource state (R12) — it cannot supply
/// (no `Supplier` impl exists for it, so a fourth purchase is a compile
/// error) and must still be accounted for at the boundary.
pub type ExhaustedVendor = Vendor<Nil>;

/// `Supplier` is implemented ONLY while stock remains (R12): supplying hands
/// over a held object (R12 "suppliers hold real objects") — the refined
/// contrast to the `Next = Self` mint-on-demand form. A purchase from
/// [`ExhaustedVendor`] takes the modeller-phrased F-015 trait-bound path.
impl<T> Supplier for Vendor<Cons<BoxedComponent, T>> {
    type Item = BoxedComponent;
    type Next = Vendor<T>;
    fn supply(self) -> (BoxedComponent, Vendor<T>) {
        let Vendor {
            stock: Cons(head, tail),
            _seal: (),
        } = self;
        (
            head,
            Vendor {
                stock: tail,
                _seal: (),
            },
        )
    }
}

/// The vendor consumes the presented purchase order (SPEC P3) — sale
/// evidence, any stock state. The order leaves the model here (the vendor
/// files it; F-029 boundary discard).
impl<Stock> Consumer<PurchaseOrder> for Vendor<Stock> {
    type Next = Vendor<Stock>;
    fn consume(self, item: PurchaseOrder) -> Vendor<Stock> {
        item.defuse(); // sanctioned consumer role (F-008)
        self
    }
}

/// The vendor accepts payment **only at its exact euro-cent price**
/// (REQ-023 structural, F-051; R12 strictness: one item, one price, one
/// step). The revenue stays with the vendor outside the model (boundary
/// organisation; F-029 arithmetic-only accounting — the euro-cent dimension
/// closes with 2340 ec stated at the vendor).
impl<Stock> Consumer<Euros<COMPONENT_PRICE_EC>> for Vendor<Stock> {
    type Next = Vendor<Stock>;
    fn consume(self, payment: Euros<COMPONENT_PRICE_EC>) -> Vendor<Stock> {
        payment.defuse(); // sanctioned consumer role (F-008); revenue kept at the boundary (F-029)
        self
    }
}

impl<Stock: Len> Vendor<Stock> {
    /// How many components the vendor holds — the length of its stock list,
    /// so count and contents cannot disagree (R7, R12).
    pub const STOCK: u64 = Stock::LEN;
}

// ---------------------------------------------------------------------------
// Packaging accounting and disposal (REQ-025's waste side; F-039 across the
// crate edge, the CS-4 `SwarfLoad` shape).
// ---------------------------------------------------------------------------

/// The total mass of a list of packaging — what the works' P8 bin-emptying
/// asserts over (SPEC P8: 50 = 50, via contents). Sealed (F-026): only lists
/// of this crate's packaging carry it, so the total cannot be forged.
pub trait PackagingLoad: packaging_sealed::Sealed {
    /// Total mass of the listed packaging, in grams.
    const GRAMS: u64;
}
impl PackagingLoad for Nil {
    const GRAMS: u64 = 0;
}
impl<const G: u64, T: PackagingLoad> PackagingLoad for Cons<Packaging<G>, T> {
    const GRAMS: u64 = G + T::GRAMS;
}
mod packaging_sealed {
    use super::Packaging;
    use model_core::list::{Cons, Nil};
    pub trait Sealed {}
    impl Sealed for Nil {}
    impl<const G: u64, T: Sealed> Sealed for Cons<Packaging<G>, T> {}
}

/// The packaging disposal stream at the system boundary: the unbounded sink
/// (`type Next = Self`, legal only at the boundary, R15/F-029) that the
/// works bin is emptied into at P8 (F-039). Being unbounded, it necessarily
/// discards what it consumes — the sanctioned exception to "a consumer keeps
/// what it consumes".
///
/// Placeholder: waste-disposal service — assumed able to take any amount.
#[must_use = "PackagingDisposal is a boundary resource: pass it on like any other resource"]
pub struct PackagingDisposal {
    _seal: (),
}

/// Disposal accepts packaging of any mass; `Next = Self` (R15, F-029).
impl<const G: u64> Consumer<Packaging<G>> for PackagingDisposal {
    type Next = PackagingDisposal;
    fn consume(self, item: Packaging<G>) -> PackagingDisposal {
        item.defuse(); // sanctioned consumer role (F-008); unbounded sink discards (F-029)
        self
    }
}

/// The creation and exit boundary of the supply family (R12, F-006): the
/// only production code where money, orders, the bureau, the stocked vendor
/// and the disposal stream come into existence, and where GBP and the
/// vendor's remaining stock leave the model.
pub mod boundary {
    use super::{Bureau, Money, PackagingDisposal, PurchaseOrder, StockedVendor, Vendor};
    use model_core::list::{Cons, Nil};

    /// GBP enters the model at the system boundary (R12): the source behind
    /// the works' account float and the customer's payment. Like
    /// `model_core::quantity::boundary::supply`, this is a **boundary-only**
    /// constructor: call it exclusively from boundary objects and supplier
    /// code — everywhere else, pence must come from conserving processes.
    /// The call is greppable (`gbp_enters_the_model`) so boundary reviews
    /// are mechanical.
    ///
    /// Placeholder: the GBP economy outside the model — the works' bank and
    /// the customer's pocket.
    pub fn gbp_enters_the_model<const PENCE: u64>() -> Money<PENCE> {
        Money::mint()
    }

    /// GBP leaves the model at the system boundary (R12): the
    /// production-legal sink for [`Money`] (F-035) — the works banks its
    /// takings at flow end, whatever the amount.
    ///
    /// Placeholder: the GBP economy outside the model — an unbounded sink.
    pub fn gbp_leaves_the_model<const PENCE: u64>(cash: Money<PENCE>) {
        // Sanctioned boundary consumption (F-008 role).
        cash.defuse();
    }

    /// The bureau enters the model (R12). Deliberately the unrefined
    /// placeholder shape — see [`Bureau`].
    ///
    /// Placeholder: bureau de change.
    pub fn new_bureau() -> Bureau {
        Bureau::mint()
    }

    /// A purchase order is raised against the vendor (R12; SPEC P2). Only
    /// the order travels with the courier — the payment is remitted
    /// flow-routed (SPEC §7).
    ///
    /// Placeholder: the works' procurement paperwork — one order, one
    /// component.
    pub fn place_purchase_order() -> PurchaseOrder {
        PurchaseOrder::mint()
    }

    /// The vendor opens for business holding its finite stock — the fill
    /// function of the **refined** placeholder (R12, SPEC §4): three real
    /// boxed components come into existence here, inside the privacy
    /// boundary, and nowhere else.
    ///
    /// Placeholder: the vendor's hinterland — its own sourcing is out of
    /// scope (SPEC §1); the stock is simply there. (The stock itself is no
    /// longer a placeholder: that is the refinement.)
    pub fn vendor_opens_for_business() -> StockedVendor {
        Vendor {
            stock: Cons(
                super::BoxedComponent::mint(),
                Cons(
                    super::BoxedComponent::mint(),
                    Cons(super::BoxedComponent::mint(), Nil),
                ),
            ),
            _seal: (),
        }
    }

    /// Recursively defuses a withdrawing vendor's remaining stock as it
    /// leaves the model. Private: together with
    /// [`vendor_returns_to_its_hinterland`] this is the only exit for
    /// unsold stock (R1, F-039).
    trait Restock {
        fn restock(self);
    }
    impl Restock for Nil {
        fn restock(self) {}
    }
    impl<T: Restock> Restock for Cons<super::BoxedComponent, T> {
        fn restock(self) {
            let Cons(component, tail) = self;
            component.defuse();
            tail.restock();
        }
    }

    /// Public-in-signature but unimplementable-outside wrapper over the
    /// private restocking machinery (the sealed-trait pattern, F-026), so
    /// [`vendor_returns_to_its_hinterland`] can name it without letting
    /// outside code defuse components.
    pub trait RestockSealed: restock_sealed::Sealed {
        #[doc(hidden)]
        fn restock_all(self);
    }
    impl<L: Restock + restock_sealed::Sealed> RestockSealed for L {
        fn restock_all(self) {
            self.restock()
        }
    }
    mod restock_sealed {
        use super::super::BoxedComponent;
        use model_core::list::{Cons, Nil};
        pub trait Sealed {}
        impl Sealed for Nil {}
        impl<T: Sealed> Sealed for Cons<BoxedComponent, T> {}
    }

    /// The vendor withdraws to its hinterland with its remaining stock
    /// (R12, F-039): the boundary exit that accounts for the vendor's
    /// end-of-model state (SPEC §6: "vendor stock at 2 with its state
    /// accounted") — the only place unsold stock is defused, which is what
    /// makes the tripwires on abandoned components trustworthy (R1, F-008).
    ///
    /// Placeholder: the vendor's hinterland — restocking and onward trade
    /// out of scope (SPEC §1).
    pub fn vendor_returns_to_its_hinterland<Stock: RestockSealed>(vendor: Vendor<Stock>) {
        let Vendor { stock, _seal: () } = vendor;
        stock.restock_all();
    }

    /// The packaging disposal stream enters the model (R12). An empty
    /// unbounded sink holds nothing, so this is an ordinary public boundary
    /// function.
    pub fn new_packaging_disposal() -> PackagingDisposal {
        PackagingDisposal { _seal: () }
    }
}

/// The supply subsystem's processes (R1, R2): pure by-value transformations.
/// They mint quantity-bearing values (cash, euro cents, components,
/// packaging), so they live inside the resource family's module (F-031).
///
/// [`split_money`] and [`combine_money`] are the **cross-crate conserving
/// GBP combinators** the works' account runs on (the CS-4 stores/line
/// pattern): each only rearranges sealed values passed in by value, with the
/// pence balance const-asserted, so the works can move money without ever
/// being able to mint it.
pub mod processes {
    use super::{BoxedComponent, COMPONENT_PRICE_EC, Consumer, Euros, GBP_EUR_RATE_DEN, GBP_EUR_RATE_NUM, InspectedComponent, Money, Packaging, PurchaseOrder, Supplier};
    // One line on purpose: the traceability grep (F-021) skips `use` lines,
    // but only when the line itself starts with `use`.
    use crate::requirements::{Req023PaidInEuroCents, Req024ExchangeAtTheBureau};
    use model_core::boundary::{send_to, take_one};

    /// Splits GBP cash into two amounts (R3: changing an amount is a
    /// process). `A + B == IN` is checked at compile time; a violation is
    /// the standard E0080 at monomorphization (F-001).
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

    /// Combines two GBP amounts into one (R3; the deposit primitive the
    /// works' account is built on). The total **cannot be computed on
    /// stable** (F-022), so the caller states it and the assert checks it —
    /// a wrong total is the same E0080 as a bad split (F-001).
    pub fn combine_money<const A: u64, const B: u64, const OUT: u64>(
        a: Money<A>,
        b: Money<B>,
    ) -> Money<OUT> {
        const {
            assert!(
                A + B == OUT,
                "money conservation violated in combine_money (R3/R19): the stated total must sum the two input amounts exactly"
            )
        };
        a.defuse();
        b.defuse();
        Money::mint()
    }

    /// P1 (bureau side) — exchanges GBP for euro cents at the bureau's
    /// stated integer rate (REQ-024; SPEC P1: 2000 p → 2340 ec),
    /// const-asserted `OUT × 100 == IN × 117`.
    ///
    /// **Honesty notes (R19, F-052):** exchange is value-equivalence at a
    /// stated rate, **not** single-dimension conservation — the GBP
    /// dimension loses `IN` and the euro-cent dimension gains `OUT`, which
    /// is minting and destroying per-currency amounts, exactly what R1
    /// forbids inside the model. It is therefore legal **only as a boundary
    /// process**, threading the placeholder [`super::Bureau`] by value. And
    /// the assert is exact multiplication — no division, no rounding,
    /// anywhere: an amount with no exact exchange at the rate has **no**
    /// `OUT` that compiles; split off an exchangeable sub-amount
    /// ([`split_money`]) and the remainder stays conserved in GBP.
    ///
    /// The supply-side chain end to end (runs as a doc-test):
    ///
    /// ```
    /// use cs5_supply::resources::boundary::{gbp_enters_the_model, new_bureau, new_packaging_disposal, place_purchase_order, vendor_opens_for_business, vendor_returns_to_its_hinterland};
    /// use cs5_supply::resources::processes::{exchange, open_and_inspect, sell_component};
    /// use cs5_supply::resources::{Euros, Money};
    /// use model_core::boundary::send_to;
    ///
    /// let bureau = new_bureau();
    /// let sterling: Money<2000> = gbp_enters_the_model();
    /// let (euro_cents, bureau): (Euros<2340>, _) = exchange(bureau, sterling);
    /// let (boxed, vendor) = sell_component(vendor_opens_for_business(), place_purchase_order(), euro_cents);
    /// let (component, packaging) = open_and_inspect::<50>(boxed);
    /// let disposal = send_to(new_packaging_disposal(), packaging);
    /// vendor_returns_to_its_hinterland(vendor);
    /// let _stay_at_the_boundary = (bureau, disposal);
    /// let _kept_for_fitting = component;
    /// ```
    ///
    /// **The exchange assert** (R4 policy: conservation violations are
    /// rustdoc `compile_fail` doc-tests, F-001/F-003) — claiming 2341 ec for
    /// 2000 p must not compile (2341 × 100 ≠ 2000 × 117):
    ///
    /// ```compile_fail
    /// use cs5_supply::resources::boundary::{gbp_enters_the_model, new_bureau};
    /// use cs5_supply::resources::processes::exchange;
    /// use cs5_supply::resources::Euros;
    ///
    /// let (euro_cents, bureau): (Euros<2341>, _) =
    ///     exchange(new_bureau(), gbp_enters_the_model::<2000>());
    /// ```
    ///
    /// **A non-exact amount has no compiling OUT** (F-052): 1999 p at
    /// 117/100 would be 2338.83 ec, and *neither* rounding direction
    /// satisfies the assert (2338 × 100 = 233 800 ≠ 1999 × 117 = 233 883;
    /// 2339 × 100 = 233 900 ≠ 233 883) — stated as a feature (SPEC §7):
    ///
    /// ```compile_fail
    /// use cs5_supply::resources::boundary::{gbp_enters_the_model, new_bureau};
    /// use cs5_supply::resources::processes::exchange;
    /// use cs5_supply::resources::Euros;
    ///
    /// let (euro_cents, bureau): (Euros<2339>, _) =
    ///     exchange(new_bureau(), gbp_enters_the_model::<1999>());
    /// ```
    ///
    /// Satisfies: REQ-024
    pub fn exchange<const IN: u64, const OUT: u64, B: Req024ExchangeAtTheBureau>(bureau: B, sterling: Money<IN>) -> (Euros<OUT>, B) {
        const {
            assert!(
                OUT * GBP_EUR_RATE_DEN == IN * GBP_EUR_RATE_NUM,
                "exchange is not exact at the stated rate (R19/F-052, REQ-024): OUT euro cents x 100 must equal IN pence x 117 - integer exchange never rounds; split off an exchangeable amount first and keep the remainder in GBP"
            )
        };
        // Value-equivalence at the boundary (F-052): the GBP amount leaves
        // the model here...
        sterling.defuse();
        // ...and the equivalent euro-cent amount enters it.
        (Euros::mint(), bureau)
    }

    /// P3 (vendor side) — the vendor sells one boxed component against the
    /// courier-presented purchase order and the **remitted** exact price
    /// (REQ-023; SPEC P3). The order and the 2340 ec are consumed by the
    /// vendor (each through its own `Consumer` impl — R12 strictness: a
    /// consumer step takes nothing but itself and the item, so the sale is
    /// this separate process); one boxed component leaves the finite stock
    /// through the `Supplier` bound, so a **fourth purchase is a compile
    /// error** with the F-015 "it is exhausted" message (the refined
    /// placeholder at work). Money and goods each balance structurally:
    /// 2340 = 2340 by the exact-price impl, 1 = 1 by the supply step.
    ///
    /// Paying in pence does not compile: `Euros<2340>` and `Money<2340>` are
    /// different dimensions (R19/F-051), so a GBP payment is an E0308 —
    /// pinned by `cs5-works/tests/ui/pay_vendor_in_pence.rs`.
    ///
    /// Satisfies: REQ-023
    pub fn sell_component<V: Req023PaidInEuroCents + Consumer<PurchaseOrder>>(vendor: V, order: PurchaseOrder, payment: Euros<COMPONENT_PRICE_EC>) -> (BoxedComponent, <<V::Next as Consumer<Euros<COMPONENT_PRICE_EC>>>::Next as Supplier>::Next) where V::Next: Consumer<Euros<COMPONENT_PRICE_EC>>, <V::Next as Consumer<Euros<COMPONENT_PRICE_EC>>>::Next: Supplier<Item = BoxedComponent> {
        // Present the order, remit the payment, take the goods — through the
        // generic access processes (F-015: the trait-bound path carries the
        // on_unimplemented messages; never call supply/consume directly).
        let vendor = send_to(vendor, order);
        let vendor = send_to(vendor, payment);
        take_one(vendor)
    }

    /// P5 (material half) — opens the box and inspects the component at
    /// goods-in (SPEC P5): the conserving transform the works' inspection
    /// process composes (the CS-4 cross-crate pattern — the mint site stays
    /// in this crate). One type per state (R9): only this process produces
    /// the [`InspectedComponent`] that REQ-025 demands. Mass is conserved at
    /// compile time (`400 + PACK == 450`); the caller states the packaging
    /// split (outputs cannot be computed on stable, F-022), and the check
    /// fires at monomorphization (F-001).
    ///
    /// **The inspection mass assert** — claiming 49 g of packaging must not
    /// compile (400 + 49 ≠ 450; a gram would vanish):
    ///
    /// ```compile_fail
    /// use cs5_supply::resources::BoxedComponent;
    /// use cs5_supply::resources::processes::open_and_inspect;
    ///
    /// let boxed = BoxedComponent::test_fixture();
    /// let (component, packaging) = open_and_inspect::<49>(boxed);
    /// ```
    pub fn open_and_inspect<const PACK: u64>(
        boxed: BoxedComponent,
    ) -> (InspectedComponent, Packaging<PACK>) {
        const {
            assert!(
                InspectedComponent::MASS_G + PACK == BoxedComponent::MASS_G,
                "mass conservation violated in open_and_inspect (R3, P5): the inspected component plus the packaging must sum exactly to the boxed component (450 = 400 + 50)"
            )
        };
        // Conserving transform: the box's 450 g continue as 400 + PACK.
        boxed.defuse();
        (InspectedComponent::mint(), Packaging::mint())
    }
}

#[cfg(test)]
mod tests {
    //! Per-process unit tests (R5): every supply process turns specific
    //! inputs into the expected outputs with nothing left unaccounted for.
    //! These tests sit inside the privacy boundary, so they may mint
    //! fixtures and defuse outputs directly; downstream-style accounting is
    //! exercised by `cs5-logistics`' and `cs5-works`' tests.

    use super::boundary::{gbp_enters_the_model, gbp_leaves_the_model, new_bureau, new_packaging_disposal, place_purchase_order, vendor_opens_for_business, vendor_returns_to_its_hinterland};
    use super::processes::{combine_money, exchange, open_and_inspect, sell_component, split_money};
    use super::{BoxedComponent, Euros, ExhaustedVendor, GBP_EUR_RATE_DEN, GBP_EUR_RATE_NUM, InspectedComponent, Money, Packaging, StockedVendor, TwoBoxedComponents, Vendor};
    use crate::characteristics::GoodsInInspected;
    use model_core::boundary::send_to;
    use model_core::list::Nil;

    /// The bureau exchange is exact at the stated rate (REQ-024; SPEC P1:
    /// 2000 p × 117/100 = 2340 ec) and the euro cents reach their
    /// production-legal sink, the vendor (F-035).
    ///
    /// Verifies: REQ-023, REQ-024
    #[test]
    fn exchange_is_exact_and_the_vendor_takes_the_remittance() {
        let sterling: Money<2000> = gbp_enters_the_model();
        let (euro_cents, bureau): (Euros<2340>, _) = exchange(new_bureau(), sterling);
        assert_eq!(Euros::<2340>::VALUE * GBP_EUR_RATE_DEN, 2000 * GBP_EUR_RATE_NUM);
        assert_eq!(Euros::<2340>::UNIT, "euro cents");
        let vendor = send_to(vendor_opens_for_business(), euro_cents);
        vendor_returns_to_its_hinterland(vendor);
        let _bureau_stays_at_the_boundary = bureau;
    }

    /// The split-then-exchange idiom (F-052 integer honesty): 2050 p has no
    /// exact euro value at 117/100, so 2000 p is split off and the 50 p
    /// remainder stays conserved in GBP — no rounding happened anywhere.
    ///
    /// Verifies: REQ-024
    #[test]
    fn inexact_amounts_split_first_and_keep_the_remainder_in_gbp() {
        let cash: Money<2050> = gbp_enters_the_model();
        let (exchangeable, remainder) = split_money::<2050, 2000, 50>(cash);
        let (euro_cents, bureau): (Euros<2340>, _) = exchange(new_bureau(), exchangeable);
        let vendor = send_to(vendor_opens_for_business(), euro_cents);
        vendor_returns_to_its_hinterland(vendor);
        gbp_leaves_the_model(remainder);
        let _bureau = bureau;
    }

    /// GBP arithmetic conserves through split and combine (R3/R19) — the
    /// combinators the works' account is built on.
    #[test]
    fn split_and_combine_conserve_the_pence() {
        let cash: Money<5000> = gbp_enters_the_model();
        let (a, b) = split_money::<5000, 2000, 3000>(cash);
        assert_eq!(Money::<2000>::VALUE + Money::<3000>::VALUE, 5000);
        let back: Money<5000> = combine_money(a, b);
        gbp_leaves_the_model(back);
    }

    /// P3: the vendor sells one component from its finite stock against the
    /// order and the exact price; the stock drops 3 → 2 with the count read
    /// off the type (the refined placeholder, SPEC §4).
    ///
    /// Verifies: REQ-023
    #[test]
    fn sell_component_takes_the_exact_price_and_depletes_the_stock() {
        assert_eq!(StockedVendor::STOCK, 3);
        let (boxed, vendor) = sell_component(
            vendor_opens_for_business(),
            place_purchase_order(),
            Euros::<2340>::mint(),
        );
        assert_eq!(Vendor::<TwoBoxedComponents>::STOCK, 2);
        assert_eq!(BoxedComponent::MASS_G, 450);
        boxed.defuse();
        vendor_returns_to_its_hinterland(vendor);
    }

    /// Three sales exhaust the 3-stock: the empty state is a distinct,
    /// accounted resource type (R12) — and a fourth purchase is a compile
    /// error, pinned in `cs5-works/tests/ui/fourth_purchase_from_the_3_stock.rs`.
    ///
    /// Verifies: REQ-023
    #[test]
    fn three_sales_exhaust_the_vendor() {
        let vendor = vendor_opens_for_business();
        let (b1, vendor) = sell_component(vendor, place_purchase_order(), Euros::mint());
        let (b2, vendor) = sell_component(vendor, place_purchase_order(), Euros::mint());
        let (b3, vendor) = sell_component(vendor, place_purchase_order(), Euros::mint());
        let exhausted: ExhaustedVendor = vendor;
        assert_eq!(ExhaustedVendor::STOCK, 0);
        b1.defuse();
        b2.defuse();
        b3.defuse();
        // The empty-stock state is still a resource: it exits at the
        // boundary like any other (R12).
        let Vendor { stock: Nil, _seal: () } = exhausted;
    }

    /// P5's material half conserves mass (450 = 400 + 50, assert) and
    /// changes the processing state (R9): the output is fit-ready, the
    /// packaging is real conserved waste.
    #[test]
    fn open_and_inspect_balances_and_changes_state() {
        let (component, packaging) = open_and_inspect::<50>(BoxedComponent::mint());
        assert_eq!(
            <InspectedComponent as GoodsInInspected>::MASS_G + Packaging::<50>::VALUE,
            BoxedComponent::MASS_G
        );
        let disposal = send_to(new_packaging_disposal(), packaging);
        // The inspected component is a kept-item type (untripwired, F-040):
        // in the real flow the instrument keeps it; this in-boundary test
        // simply retains it to scope end.
        let _untripwired_kept_item = component;
        let _disposal_stays_at_the_boundary = disposal;
    }

    /// Abandoned euro cents are caught by the tripwire at test time (R1
    /// layer 2, F-008/F-032), naming the type and the decimal amount
    /// (F-027).
    #[test]
    #[should_panic(expected = "Euros<2340> dropped without being consumed")]
    fn abandoned_euro_cents_trip_the_tripwire() {
        let (euro_cents, bureau): (Euros<2340>, _) =
            exchange(new_bureau(), gbp_enters_the_model::<2000>());
        let _bureau = bureau;
        let _left_on_the_counter = euro_cents; // falls out of scope: leak
    }
}
