//! # EXP-12: Money as a conserved dimension (candidate R19)
//!
//! Tests open question 3 of `instructions.md`: can money be modelled as an
//! ordinary conserved dimension with the **existing** model-core machinery —
//! an R7 base-unit table row plus downstream conventions — or does it need
//! new kernel support?
//!
//! What is built here, all downstream of `model-core` (read-only):
//!
//! 1. **Currency units in the R7 style** ([`units`]): one unit type per
//!    currency ([`units::Pence`], [`units::EuroCents`]), integer minor units,
//!    implementing model-core's `Unit` kind trait. The generic
//!    `Qty`/`split`/`combine` machinery works for money unchanged; mixing
//!    currencies is a type-check-time E0308 (`tests/ui/mix_currencies.rs`).
//! 2. **An [`money::Account`] container** (R15 pattern) with
//!    [`money::processes::draw_funds`]; overspending is the standard overdraw
//!    compile error (E0080 at monomorphization, F-001 — regression as a
//!    rustdoc `compile_fail` doc-test, R4/F-003).
//! 3. **Payment as a process chain** (R12 strictness): [`goods::Vendor`] is a
//!    boundary object that is both `Consumer<Money<PRICE>>` and a goods
//!    source (`Supplier`); [`money::processes::purchase`] pays the vendor and
//!    obtains the goods, money and goods each conserved in their own
//!    dimension; change-giving is an R3 split
//!    ([`money::processes::split_money`]).
//! 4. **Currency exchange** ([`money::processes::exchange_gbp_to_eur`]) at a
//!    stated integer rate, const-asserted (`OUT * DEN == IN * NUM`).
//!    Documented honestly: exchange is **value-equivalence at a stated rate,
//!    not single-dimension conservation** — the GBP dimension loses `IN`
//!    while the EUR dimension gains `OUT` — so it is a boundary process
//!    (always a placeholder), like a draw. Integer honesty: there is no
//!    silent rounding anywhere — an amount whose exchange is not exact at
//!    the rate has **no** compiling `OUT`; the modeller splits off an
//!    exchangeable sub-amount first and the remainder stays conserved in the
//!    original currency.
//! 5. Compile-fail coverage: trybuild for the type-check-time errors (mixing
//!    currencies, paying in the wrong currency, paying the wrong amount);
//!    rustdoc `compile_fail` doc-tests for the post-monomorphization
//!    conservation errors (overspending, wrong change, inexact exchange).

#![recursion_limit = "2048"]
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![deny(let_underscore_drop)]

/// Currency units in the R7 style: one unit type per currency, integer minor
/// units, implementing model-core's open `Unit` kind trait.
///
/// This is the whole cost of a currency at the quantity level: **one struct
/// and one impl line per currency**, exactly like adding any other R7 base
/// unit. Each currency is its own dimension — `Qty<V, Pence>` and
/// `Qty<V, EuroCents>` never mix (E0308), just as grams never mix with
/// millimetres. There is deliberately no common "money" unit: a shared unit
/// would make cross-currency `combine` type-correct, which is precisely the
/// modelling error R19 exists to prevent.
pub mod units {
    use model_core::quantity::Unit;

    /// GBP in pence (integer minor units, R7 style). Proposed R7 table
    /// entry: Money is one dimension **per currency**, base unit the
    /// currency's minor unit.
    pub struct Pence;
    impl Unit for Pence {}

    /// EUR in euro cents (integer minor units, R7 style). The second
    /// currency, proving non-mixing is a type error.
    pub struct EuroCents;
    impl Unit for EuroCents {}
}

/// The money resource family (R15 container style): sealed cash and account
/// types, their creation boundary, and the conserving money processes.
///
/// Layout per F-006/F-031: continuous-style processes (draws, splits,
/// deposits, exchange) mint quantity-bearing values, so they live inside the
/// resource family's module tree, beside its `boundary`.
pub mod money {
    model_core::container_resource! {
        /// Cash in hand, in pence (GBP). The money dimension's in-model
        /// carrier: drawn from an [`Account`], split for change
        /// ([`processes::split_money`]), and conserved like any other
        /// quantity (R1, R15).
        Money,
        unit = "pence",
        must_use = "Money is a conserved resource: pass it on, bank it, or hand it to a Consumer"
    }

    model_core::container_resource! {
        /// Cash in hand, in euro cents (EUR). A **different dimension** from
        /// [`Money`]: the two sealed types never mix, so cross-currency
        /// arithmetic is a type error, and conversion goes through the
        /// explicit boundary exchange ([`processes::exchange_gbp_to_eur`]).
        Euros,
        unit = "euro cents",
        must_use = "Euros is a conserved resource: pass it on or hand it to a Consumer"
    }

    model_core::container_resource! {
        /// A bank account holding `V` pence (R15 quantity container).
        /// `Account<0>` is the empty state: a distinct resource that must
        /// still be accounted for ([`boundary::close_account`]).
        Account,
        unit = "pence remaining",
        must_use = "Account is a conserved resource: even an empty account must be passed on or closed at the boundary"
    }

    /// The GBP→EUR exchange rate, stated as an integer ratio:
    /// `NUM` euro cents per `DEN` pence (here €1.17 per £1.00).
    ///
    /// The rate is a **modelling input** stated once as consts, not a
    /// computed market value; [`processes::exchange_gbp_to_eur`]
    /// const-asserts every exchange against it.
    pub const GBP_EUR_RATE_NUM: u64 = 117;
    /// Denominator of the stated rate: see [`GBP_EUR_RATE_NUM`].
    pub const GBP_EUR_RATE_DEN: u64 = 100;

    /// The creation boundary for money (R12): the only production code that
    /// brings money into the model or lets it leave.
    pub mod boundary {
        use super::{Account, Euros};

        /// An account enters the model holding `BALANCE` pence.
        ///
        /// Placeholder: bank — refine to a named bank/account in a real
        /// model.
        pub fn open_account<const BALANCE: u64>() -> Account<BALANCE> {
            Account::mint()
        }

        /// An account leaves the model (the bank keeps whatever it still
        /// holds). The boundary sink for [`Account`], including the empty
        /// state `Account<0>` (R15: every end-state resource is accounted
        /// for).
        ///
        /// Placeholder: bank — an unbounded sink for closed accounts.
        pub fn close_account<const V: u64>(account: Account<V>) {
            // Sanctioned boundary consumption (F-008 role).
            account.defuse();
        }

        /// Euros leave the model, spent outside it. The production-legal
        /// sink for [`Euros`] (F-035: whoever mints a tripwired type must
        /// ship a production consumer for it).
        ///
        /// Placeholder: spending abroad — an unbounded sink for exchanged
        /// currency.
        pub fn spend_abroad<const V: u64>(cash: Euros<V>) {
            cash.defuse();
        }
    }

    /// Conserving money processes (R1, R3, R15). They mint quantity-bearing
    /// values, so they live inside the family's module (F-031).
    pub mod processes {
        use super::{Account, Euros, GBP_EUR_RATE_DEN, GBP_EUR_RATE_NUM, Money};
        use crate::goods::Bureau;
        use model_core::boundary::{Consumer, Supplier};

        model_core::draw_process! {
            /// Draws `TAKE` pence from an account holding `FULL`, leaving
            /// `LEFT` (R15 pattern, caller-stated remainder, F-022/F-030).
            /// **Overspending is a compile error**: no `LEFT` exists with
            /// `TAKE + LEFT == FULL` when `TAKE > FULL`. It is an E0080 at
            /// monomorphization — invisible to `cargo check` and editor
            /// diagnostics (F-001).
            ///
            /// ```
            /// use exp12_money::money::boundary::{close_account, open_account};
            /// use exp12_money::money::processes::{deposit, draw_funds};
            ///
            /// let account = open_account::<5000>();
            /// let (cash, account) = draw_funds::<300, 4700, 5000>(account);
            /// // Account for everything (R1): bank the cash again and close.
            /// let account = deposit::<300, 4700, 5000>(account, cash);
            /// close_account(account);
            /// ```
            ///
            /// Regression — overspending: drawing 6000 pence from a 5000
            /// pence account must not compile (R4 policy: rustdoc
            /// `compile_fail`, because trybuild cannot see post-mono errors,
            /// F-003):
            ///
            /// ```compile_fail
            /// use exp12_money::money::boundary::open_account;
            /// use exp12_money::money::processes::draw_funds;
            ///
            /// let account = open_account::<5000>();
            /// let (cash, empty) = draw_funds::<6000, 0, 5000>(account);
            /// ```
            pub fn draw_funds: Account => Money,
            assert = "overspend: money conservation violated in draw_funds (R15/R19): TAKE + LEFT must equal FULL - is the draw larger than the account's balance?"
        }

        /// Banks cash into an account (R3 combine, caller-stated total,
        /// F-022): `Account<BALANCE>` + `Money<ADD>` → `Account<NEW>` with
        /// `BALANCE + ADD == NEW` checked at compile time.
        ///
        /// A wrong total is the same E0080 as a bad draw. Regression —
        /// depositing 300 into 4700 and claiming 5100 must not compile:
        ///
        /// ```compile_fail
        /// use exp12_money::money::boundary::open_account;
        /// use exp12_money::money::processes::{deposit, draw_funds};
        ///
        /// let account = open_account::<5000>();
        /// let (cash, account) = draw_funds::<300, 4700, 5000>(account);
        /// let account = deposit::<300, 4700, 5100>(account, cash);
        /// ```
        pub fn deposit<const ADD: u64, const BALANCE: u64, const NEW: u64>(
            account: Account<BALANCE>,
            cash: Money<ADD>,
        ) -> Account<NEW> {
            const {
                assert!(
                    BALANCE + ADD == NEW,
                    "money conservation violated in deposit (R3/R19): BALANCE + ADD must equal NEW - is the stated new balance wrong?"
                )
            };
            // Conserving transform: both inputs continue as the new balance.
            account.defuse();
            cash.defuse();
            Account::mint()
        }

        /// Splits cash into two amounts (R3: changing an amount is a
        /// process) — the change-giving primitive. `A + B == IN` is checked
        /// at compile time; a violation is the standard E0080 (F-001).
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

        /// Purchase as a process chain (R12 strictness: payment is **not**
        /// part of a supplier/consumer step, so buying is its own process).
        ///
        /// The vendor is a boundary object that is both a money consumer and
        /// a goods source; this process splits the tendered cash into the
        /// price and the change (R3 split), pays the vendor, and takes the
        /// goods. Both dimensions conserve:
        ///
        /// * **money**: `TENDERED == PRICE + CHANGE` (const-asserted here);
        ///   the price leaves the model at the vendor boundary, the change
        ///   comes back;
        /// * **goods**: the item comes from the vendor's held stock (here a
        ///   `Next = Self` placeholder source) and is returned to the caller.
        ///
        /// Because the vendor implements `Consumer` **only at its exact
        /// price**, paying the wrong amount or the wrong currency is a
        /// type-check-time E0277 with the modeller-phrased
        /// `on_unimplemented` message (F-015) — pinned by trybuild
        /// (`tests/ui/pay_wrong_amount.rs`, `tests/ui/pay_wrong_currency.rs`).
        /// A wrong `CHANGE` is the usual post-mono E0080. Regression —
        /// tendering 500 for a 350 purchase and claiming 200 change must not
        /// compile:
        ///
        /// ```compile_fail
        /// use exp12_money::goods::boundary::new_vendor;
        /// use exp12_money::money::boundary::open_account;
        /// use exp12_money::money::processes::{draw_funds, purchase};
        ///
        /// let account = open_account::<1000>();
        /// let (tendered, account) = draw_funds::<500, 500, 1000>(account);
        /// let (spanner, change, vendor) = purchase::<_, 500, 350, 200>(new_vendor(), tendered);
        /// ```
        pub fn purchase<V, const TENDERED: u64, const PRICE: u64, const CHANGE: u64>(
            vendor: V,
            tendered: Money<TENDERED>,
        ) -> (
            <V::Next as Supplier>::Item,
            Money<CHANGE>,
            <V::Next as Supplier>::Next,
        )
        where
            V: Consumer<Money<PRICE>>,
            V::Next: Supplier,
        {
            const {
                assert!(
                    TENDERED == PRICE + CHANGE,
                    "money conservation violated in purchase (R1/R19): TENDERED must equal PRICE + CHANGE - is the stated change wrong for this price?"
                )
            };
            let (price, change) = split_money::<TENDERED, PRICE, CHANGE>(tendered);
            // Pay, then take the goods, through the generic access processes
            // (F-015: the trait-bound path carries the on_unimplemented
            // messages; never call supply/consume on a concrete value).
            let vendor = model_core::boundary::send_to(vendor, price);
            let (goods, vendor) = model_core::boundary::take_one(vendor);
            (goods, change, vendor)
        }

        /// Exchanges GBP for EUR at the stated integer rate
        /// ([`GBP_EUR_RATE_NUM`]/[`GBP_EUR_RATE_DEN`]: 117 euro cents per
        /// 100 pence), const-asserted as `OUT * DEN == IN * NUM`.
        ///
        /// **Honesty notes (R19):**
        ///
        /// * Exchange is **value-equivalence at a stated rate, not
        ///   single-dimension conservation**: the GBP dimension loses `IN`
        ///   and the EUR dimension gains `OUT`. That mints and destroys
        ///   per-currency amounts — exactly what R1 forbids inside the model
        ///   — so exchange is legal **only as a boundary process** (the
        ///   bureau is a `Next = Self`-style placeholder boundary object,
        ///   threaded by value like any resource).
        /// * **Integer honesty**: the assert is exact multiplication — no
        ///   division, no rounding, anywhere. An amount whose exchange is
        ///   not exact at the rate has **no** `OUT` that compiles; the
        ///   modeller first splits off an exchangeable sub-amount
        ///   ([`split_money`]) and the remainder stays conserved in GBP.
        ///
        /// ```
        /// use exp12_money::goods::boundary::new_bureau;
        /// use exp12_money::money::boundary::{close_account, open_account, spend_abroad};
        /// use exp12_money::money::processes::{deposit, draw_funds, exchange_gbp_to_eur, split_money};
        ///
        /// let account = open_account::<333>();
        /// let (cash, account) = draw_funds::<333, 0, 333>(account);
        /// // 333 pence has no exact EUR value at 117/100: split off 300.
        /// let (exchangeable, remainder) = split_money::<333, 300, 33>(cash);
        /// let (euros, bureau) = exchange_gbp_to_eur::<300, 351>(new_bureau(), exchangeable);
        /// // Account for everything (R1): euros leave at the boundary, the
        /// // GBP remainder goes back in the account, the account closes.
        /// spend_abroad(euros);
        /// let account = deposit::<33, 0, 33>(account, remainder);
        /// close_account(account);
        /// let _bureau_stays_at_the_boundary = bureau;
        /// ```
        ///
        /// Regression — a rounded (inexact) exchange must not compile: 333
        /// pence at 117/100 would be 389.61 euro cents, and **neither**
        /// rounding direction satisfies the assert (389 × 100 = 38 900 ≠
        /// 333 × 117 = 38 961; 390 × 100 = 39 000 ≠ 38 961):
        ///
        /// ```compile_fail
        /// use exp12_money::goods::boundary::new_bureau;
        /// use exp12_money::money::boundary::open_account;
        /// use exp12_money::money::processes::{draw_funds, exchange_gbp_to_eur};
        ///
        /// let account = open_account::<333>();
        /// let (cash, empty) = draw_funds::<333, 0, 333>(account);
        /// let (euros, bureau) = exchange_gbp_to_eur::<333, 390>(new_bureau(), cash);
        /// ```
        pub fn exchange_gbp_to_eur<const IN: u64, const OUT: u64>(
            bureau: Bureau,
            gbp: Money<IN>,
        ) -> (Euros<OUT>, Bureau) {
            const {
                assert!(
                    OUT * GBP_EUR_RATE_DEN == IN * GBP_EUR_RATE_NUM,
                    "exchange is not exact at the stated rate (R19): OUT euro cents * DEN must equal IN pence * NUM - integer exchange never rounds; split off an exchangeable amount first and keep the remainder in GBP"
                )
            };
            // Value-equivalence at the boundary: the GBP amount leaves the
            // model here (sanctioned boundary consumption)...
            gbp.defuse();
            // ...and the equivalent EUR amount enters it.
            (Euros::mint(), bureau)
        }
    }
}

/// The goods side of a purchase: the vendor boundary object (money consumer
/// **and** goods source), the goods themselves, and their sinks.
pub mod goods {
    use model_core::boundary::{Consumer, Supplier};

    use crate::money::Money;

    model_core::consumable_resource! {
        /// A spanner: the purchased goods (discrete, R13). Tripwired like any
        /// consumable; its production-legal sink is [`SpannerRack`] (F-035).
        Spanner,
        must_use = "Spanner is a conserved resource: pass it on or hand it to a Consumer"
    }

    /// The price of a spanner at [`Vendor`], in pence. Stated once; the
    /// vendor's `Consumer` impl exists **only** at this amount, so a wrong
    /// payment is a type-check-time error naming the right price in its
    /// help text.
    pub const SPANNER_PRICE_PENCE: u64 = 350;

    model_core::reusable_resource! {
        /// A vendor at the system boundary: consumes GBP money at its exact
        /// price and supplies spanners. Both roles have `Next = Self` — an
        /// unbounded boundary object, legal only at the boundary (R15).
        ///
        /// Placeholder: tool vendor — assumed unbounded stock and unbounded
        /// appetite for money; refine to a named organisation with finite
        /// stock later.
        Vendor,
        must_use = "Vendor is a boundary object: pass it on or return it to the caller"
    }

    /// The vendor accepts payment **only at its exact price** (R12
    /// strictness: one item, one price, one step; a purchase that needs
    /// change is a separate process, `money::processes::purchase`).
    ///
    /// An unbounded sink discards its intake (F-029): the money leaves the
    /// model here.
    impl Consumer<Money<SPANNER_PRICE_PENCE>> for Vendor {
        type Next = Vendor;
        fn consume(self, payment: Money<SPANNER_PRICE_PENCE>) -> Vendor {
            payment.defuse();
            self
        }
    }

    /// The vendor supplies spanners: an unbounded `Next = Self` goods source
    /// (a *discrete* source, so a `Supplier` impl is legal — F-028 restricts
    /// the trait to discrete items, which goods are).
    ///
    /// Placeholder: unbounded stock — mints at the boundary (R12/R15
    /// licence).
    impl Supplier for Vendor {
        type Item = Spanner;
        type Next = Vendor;
        fn supply(self) -> (Spanner, Vendor) {
            (Spanner::mint(), self)
        }
    }

    model_core::reusable_resource! {
        /// Where bought spanners end up: an unbounded `Next = Self` boundary
        /// sink, the production-legal consumer for [`Spanner`] (F-035).
        ///
        /// Placeholder: the buyer's toolbox — assumed unbounded.
        SpannerRack,
        must_use = "SpannerRack is a boundary object: pass it on or return it to the caller"
    }

    /// The rack accepts spanners without bound (F-029: an unbounded consumer
    /// discards — here, "stores outside the model").
    impl Consumer<Spanner> for SpannerRack {
        type Next = SpannerRack;
        fn consume(self, spanner: Spanner) -> SpannerRack {
            spanner.defuse();
            self
        }
    }

    model_core::reusable_resource! {
        /// A bureau de change at the system boundary: consumes one currency
        /// and sources the other at the stated rate
        /// (`money::processes::exchange_gbp_to_eur`). Threaded by value like
        /// any boundary object.
        ///
        /// Placeholder: bureau de change — assumed unbounded float in both
        /// currencies.
        Bureau,
        must_use = "Bureau is a boundary object: pass it on or return it to the caller"
    }

    /// The goods/vendor boundary (R12).
    pub mod boundary {
        use super::{Bureau, SpannerRack, Vendor};

        /// A vendor enters the model. Placeholder: tool vendor.
        pub fn new_vendor() -> Vendor {
            Vendor::mint()
        }

        /// A spanner rack (goods sink) enters the model. Placeholder:
        /// the buyer's toolbox.
        pub fn new_rack() -> SpannerRack {
            SpannerRack::mint()
        }

        /// A bureau de change enters the model. Placeholder: bureau de
        /// change.
        pub fn new_bureau() -> Bureau {
            Bureau::mint()
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::goods::boundary::{new_bureau, new_rack, new_vendor};
    use crate::money::boundary::{close_account, open_account, spend_abroad};
    use crate::money::processes::{
        deposit, draw_funds, exchange_gbp_to_eur, purchase, split_money,
    };
    use crate::money::{Account, Euros, Money};
    use crate::units::{EuroCents, Pence};
    use model_core::boundary::send_to;
    use model_core::quantity::{Qty, Quantity, boundary::supply, combine, split};

    /// R7-style currency quantities: the existing unit-generic machinery
    /// (`Qty`/`split`/`combine`) serves money unchanged — no new model-core
    /// machinery. Verifies build item 1.
    #[test]
    fn qty_split_and_combine_work_for_currency_units() {
        let float = supply::<2000, Pence>();
        let (a, b): (Qty<1500, Pence>, Qty<500, Pence>) = split(float);
        assert_eq!(<Qty<1500, Pence> as Quantity>::VALUE, 1500);
        let back: Qty<2000, Pence> = combine(a, b);
        let _accounted = back;

        // The second currency is just another unit (one impl line).
        let eur = supply::<117, EuroCents>();
        let (x, y): (Qty<100, EuroCents>, Qty<17, EuroCents>) = split(eur);
        let back: Qty<117, EuroCents> = combine(x, y);
        let _accounted = back;
    }

    /// Account draw-down (R15 pattern): the balance falls across calls, the
    /// modeller restates the running balance, the compiler checks each step
    /// (F-030), and everything is accounted for.
    #[test]
    fn account_draws_down_and_balances() {
        let account = open_account::<10_000>();
        let (c1, account) = draw_funds::<2000, 8000, 10_000>(account);
        let (c2, account) = draw_funds::<3000, 5000, 8000>(account);
        assert_eq!(Account::<5000>::VALUE, 5000);
        assert_eq!(Money::<2000>::UNIT, "pence");
        // Bank it all again: deposits are the conserving combine.
        let account = deposit::<2000, 5000, 7000>(account, c1);
        let account = deposit::<3000, 7000, 10_000>(account, c2);
        close_account(account);
    }

    /// The full payment chain (build item 3): draw funds, purchase with
    /// change, bank the change, store the goods. Money conserves
    /// (500 = 350 to the vendor + 150 change) and goods conserve (one
    /// spanner from the vendor to the rack).
    #[test]
    fn purchase_pays_vendor_takes_goods_and_returns_change() {
        let account = open_account::<1000>();
        let (tendered, account) = draw_funds::<500, 500, 1000>(account);
        let (spanner, change, vendor) =
            purchase::<_, 500, 350, 150>(new_vendor(), tendered);
        let rack = send_to(new_rack(), spanner);
        let account = deposit::<150, 500, 650>(account, change);
        close_account(account);
        // Boundary objects stay with the caller (no tripwire, R2-style).
        let (_vendor, _rack) = (vendor, rack);
    }

    /// Exact payment: no change needed, the split is 350 + 0 — `Money<0>` is
    /// a real (empty) resource that still has to be accounted for, like any
    /// R15 empty state.
    #[test]
    fn exact_payment_leaves_zero_change_which_is_still_accounted() {
        let account = open_account::<350>();
        let (tendered, account) = draw_funds::<350, 0, 350>(account);
        let (spanner, no_change, vendor) =
            purchase::<_, 350, 350, 0>(new_vendor(), tendered);
        let rack = send_to(new_rack(), spanner);
        let account = deposit::<0, 0, 0>(account, no_change);
        close_account(account);
        let (_vendor, _rack) = (vendor, rack);
    }

    /// Exchange at the stated rate (build item 4): 200 pence × 117/100 =
    /// 234 euro cents, exactly. Value-equivalence, not conservation: the
    /// pence leave the GBP dimension, the cents enter the EUR dimension, at
    /// the boundary only.
    #[test]
    fn exchange_is_exact_at_the_stated_rate() {
        let account = open_account::<200>();
        let (cash, account) = draw_funds::<200, 0, 200>(account);
        let (euros, bureau) = exchange_gbp_to_eur::<200, 234>(new_bureau(), cash);
        assert_eq!(Euros::<234>::VALUE, 234);
        assert_eq!(Euros::<234>::UNIT, "euro cents");
        spend_abroad(euros);
        close_account(account);
        let _bureau = bureau;
    }

    /// The split-then-exchange idiom (integer honesty): 333 pence has no
    /// exact EUR value at 117/100, so the modeller splits off 300 and the
    /// 33-pence remainder stays conserved in GBP. No rounding happened
    /// anywhere.
    #[test]
    fn inexact_amounts_split_first_and_keep_the_remainder() {
        let account = open_account::<333>();
        let (cash, account) = draw_funds::<333, 0, 333>(account);
        let (exchangeable, remainder) = split_money::<333, 300, 33>(cash);
        let (euros, bureau) = exchange_gbp_to_eur::<300, 351>(new_bureau(), exchangeable);
        spend_abroad(euros);
        let account = deposit::<33, 0, 33>(account, remainder);
        close_account(account);
        let _bureau = bureau;
    }

    /// The conservation regime covers money like any resource: abandoned
    /// change trips the tripwire at test time (R1 layer 2, F-032), naming
    /// the type and the decimal amount (F-027).
    #[test]
    #[should_panic(expected = "Money<150> dropped without being consumed")]
    fn abandoned_change_trips_the_tripwire() {
        let account = open_account::<500>();
        let (tendered, account) = draw_funds::<500, 0, 500>(account);
        let (price, change) = split_money::<500, 350, 150>(tendered);
        let vendor = send_to(new_vendor(), price);
        close_account(account);
        let _vendor = vendor;
        let _change_left_on_the_counter = change; // falls out of scope: leak
    }
}
