//! The workshop's money (R19): one conserved dimension for GBP, held and
//! moved with the R15 machinery unchanged — no new kernel support (F-051).
//!
//! Cash in hand is the sealed const-magnitude resource [`Money`] (integer
//! minor units: pence, per the R7 base-unit table); money at rest is the
//! quantity container [`Account`], drawn down by
//! [`processes::draw_funds`], so **overspending is the standard R15 overdraw
//! compile error**. A deposit is the conserving combine back into the
//! container; change-giving is an R3 split ([`processes::split_money`]).
//!
//! **Payment is a process** (R12 strictness: a supplier or consumer step
//! takes nothing but itself and the item). The [`Vendor`] is a boundary
//! object that is both `Consumer<Money<BOLT_BOX_PRICE_PENCE>>` and a goods
//! source (`Next = Self`, F-029): it sells the pilot its boxes of fastening
//! bolts. [`processes::purchase`] splits the tendered cash into price and
//! change, pays the vendor, and takes the goods — money and goods each
//! conserved in their own dimension.
//!
//! **The vendor implements `Consumer` only at its exact price** (F-051),
//! stated once as [`BOLT_BOX_PRICE_PENCE`]: paying the wrong amount then
//! fails at **type-check time** (visible to editors and trybuild, unlike the
//! conservation asserts), and the error names the correct price —
//! `expected Money<400>, found Money<300>` (pinned by
//! `tests/ui/pay_wrong_amount.rs`).
//!
//! The pilot trades in one currency only, so there is **no exchange process**
//! here (R19 confines exchange to a boundary placeholder; a second currency
//! would be one more sealed money type in this module).
//!
//! Layout per F-006/F-031: the sealed types in this module, with the creation
//! [`boundary`] and the conserving [`processes`] (which mint quantity-bearing
//! values) as child modules. Adapted from `experiments/exp12-money/src/lib.rs`.

use crate::catalogue::BoltBox;
use crate::catalogue::boundary::{BoltsOf, full_box};
use crate::catalogue::FasteningBolt;
use model_core::boundary::{Consumer, Supplier};
use model_core::nat::aliases::N4;

model_core::container_resource! {
    /// Cash in hand, in pence (GBP, integer minor units — the R7 money row).
    /// The money dimension's in-model carrier: drawn from an [`Account`],
    /// split for change ([`processes::split_money`]), and conserved like any
    /// other quantity (R1, R15, R19) — abandoned change trips the tripwire.
    Money,
    unit = "pence",
    must_use = "Money is a conserved resource: pass it on, bank it, or hand it to a Consumer"
}

model_core::container_resource! {
    /// The workshop's bank account holding `V` pence (R15 quantity
    /// container). `Account<0>` is the empty state: a distinct resource that
    /// must still be accounted for ([`boundary::close_account`]).
    Account,
    unit = "pence remaining",
    must_use = "Account is a conserved resource: even an empty account must be passed on or closed at the boundary"
}

/// The price of a box of four fastening bolts at the [`Vendor`], in pence.
/// Stated once (F-051); the vendor's `Consumer` impl exists **only** at this
/// amount, so a wrong payment is a type-check-time error naming the right
/// price.
pub const BOLT_BOX_PRICE_PENCE: u64 = 400;

/// The box of four fastening bolts as the vendor sells it — the goods side of
/// a [`processes::purchase`].
pub type PurchasedBoltBox = BoltBox<BoltsOf<FasteningBolt, N4>>;

model_core::reusable_resource! {
    /// The fastener vendor at the system boundary (R19): consumes GBP money
    /// at its exact price and supplies boxes of fastening bolts. Both roles
    /// have `Next = Self` — an unbounded boundary object, legal only at the
    /// boundary (R15, F-029).
    ///
    /// Placeholder: fastener vendor — assumed unbounded stock and unbounded
    /// appetite for money; refine to a named organisation with finite stock.
    Vendor,
    must_use = "Vendor is a boundary object: pass it on or return it to the caller"
}

/// The vendor accepts payment **only at its exact price** (R12 strictness:
/// one item, one price, one step; a purchase that needs change is a separate
/// process, [`processes::purchase`]). An unbounded sink discards its intake
/// (F-029): the money leaves the model here.
impl Consumer<Money<BOLT_BOX_PRICE_PENCE>> for Vendor {
    type Next = Vendor;
    fn consume(self, payment: Money<BOLT_BOX_PRICE_PENCE>) -> Vendor {
        payment.defuse();
        self
    }
}

/// The vendor supplies full bolt boxes: an unbounded `Next = Self` goods
/// source (a *discrete* source, so a `Supplier` impl is legal — F-028
/// restricts the trait to discrete items, which boxed goods are).
///
/// Placeholder: unbounded stock — boxes are minted at the boundary through
/// the catalogue's own sealed fill machinery (R12/R15 licence).
impl Supplier for Vendor {
    type Item = PurchasedBoltBox;
    type Next = Vendor;
    fn supply(self) -> (PurchasedBoltBox, Vendor) {
        (full_box::<FasteningBolt, N4>(), self)
    }
}

/// The creation boundary for money (R12, R19): the only production code that
/// brings money into the model or lets it leave.
pub mod boundary {
    use super::{Account, Vendor};

    /// The workshop's account enters the model holding `BALANCE` pence.
    ///
    /// Placeholder: bank — refine to a named bank/account in a real model.
    pub fn open_account<const BALANCE: u64>() -> Account<BALANCE> {
        Account::mint()
    }

    /// An account leaves the model (the bank keeps whatever it still holds).
    /// The boundary sink for [`Account`], including the empty state
    /// `Account<0>` (R15: every end-state resource is accounted for).
    ///
    /// Placeholder: bank — an unbounded sink for closed accounts.
    pub fn close_account<const V: u64>(account: Account<V>) {
        // Sanctioned boundary consumption (F-008 role).
        account.defuse();
    }

    /// A vendor enters the model (R12).
    ///
    /// Placeholder: fastener vendor.
    pub fn new_vendor() -> Vendor {
        Vendor::mint()
    }
}

/// Conserving money processes (R1, R3, R15, R19). They mint quantity-bearing
/// values, so they live inside the family's module (F-031).
pub mod processes {
    use super::{Account, Money};
    use model_core::boundary::{Consumer, Supplier};

    model_core::draw_process! {
        /// Draws `TAKE` pence from an account holding `FULL`, leaving `LEFT`
        /// (R15 pattern, caller-stated remainder, F-022/F-030).
        /// **Overspending is a compile error** (R19): no `LEFT` exists with
        /// `TAKE + LEFT == FULL` when `TAKE > FULL`. It is an E0080 at
        /// monomorphization — invisible to `cargo check` and editor
        /// diagnostics (F-001).
        ///
        /// ```
        /// use pilot_workshop::money::boundary::{close_account, open_account};
        /// use pilot_workshop::money::processes::{deposit, draw_funds};
        ///
        /// let account = open_account::<5000>();
        /// let (cash, account) = draw_funds::<300, 4700, 5000>(account);
        /// // Account for everything (R1): bank the cash again and close.
        /// let account = deposit::<300, 4700, 5000>(account, cash);
        /// close_account(account);
        /// ```
        ///
        /// Regression — overspending: drawing 600 pence from a 500 pence
        /// account must not compile (R4 policy: rustdoc `compile_fail`,
        /// because trybuild cannot see post-monomorphization errors, F-003):
        ///
        /// ```compile_fail
        /// use pilot_workshop::money::boundary::open_account;
        /// use pilot_workshop::money::processes::draw_funds;
        ///
        /// let account = open_account::<500>();
        /// let (cash, empty) = draw_funds::<600, 0, 500>(account);
        /// ```
        pub fn draw_funds: Account => Money,
        assert = "overspend: money conservation violated in draw_funds (R15/R19): TAKE + LEFT must equal FULL - is the draw larger than the account's balance?"
    }

    /// Banks cash into an account (R3 combine, caller-stated total, F-022):
    /// `Account<BALANCE>` + `Money<ADD>` → `Account<NEW>` with
    /// `BALANCE + ADD == NEW` checked at compile time. A wrong total is the
    /// same E0080 as a bad draw (F-001).
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

    /// Purchase as a process chain (R19; R12 strictness: payment is **not**
    /// part of a supplier/consumer step, so buying is its own process).
    ///
    /// The vendor is a boundary object that is both a money consumer and a
    /// goods source; this process splits the tendered cash into the price and
    /// the change (R3 split), pays the vendor, and takes the goods. Both
    /// dimensions conserve: **money** (`TENDERED == PRICE + CHANGE`,
    /// const-asserted; the price leaves the model at the vendor boundary, the
    /// change comes back) and **goods** (the item comes from the vendor's
    /// stock to the caller).
    ///
    /// Because the vendor implements `Consumer` **only at its exact price**
    /// (F-051), paying the wrong amount is a type-check-time error naming the
    /// right price — pinned by `tests/ui/pay_wrong_amount.rs`. A wrong
    /// `CHANGE` is the usual post-monomorphization E0080; note the second,
    /// composed-process echo E0080 whose instantiation note points at the
    /// `split_money` call below rather than your flow line — read the first
    /// error (F-051, error-reading guide).
    ///
    /// Regression — tendering 500 for a 400 purchase and claiming 50 change
    /// must not compile:
    ///
    /// ```compile_fail
    /// use pilot_workshop::money::boundary::{new_vendor, open_account};
    /// use pilot_workshop::money::processes::{draw_funds, purchase};
    ///
    /// let account = open_account::<1000>();
    /// let (tendered, account) = draw_funds::<500, 500, 1000>(account);
    /// let (bolts, change, vendor) = purchase::<_, 500, 400, 50>(new_vendor(), tendered);
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
}

#[cfg(test)]
mod tests {
    use super::boundary::{close_account, new_vendor, open_account};
    use super::processes::{deposit, draw_funds, purchase, split_money};
    use super::{Account, Money, PurchasedBoltBox};
    use model_core::boundary::send_to;

    /// Account draw-down (R15 pattern applied to money, R19): the balance
    /// falls across calls, the modeller restates the running balance, the
    /// compiler checks each step (F-030), and everything is accounted for.
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

    /// The full payment chain (R19): draw funds, purchase with change, bank
    /// the change. Money conserves (500 = 400 to the vendor + 100 change) and
    /// goods conserve (one full bolt box from the vendor to the caller).
    #[test]
    fn purchase_pays_vendor_takes_goods_and_returns_change() {
        let account = open_account::<1000>();
        let (tendered, account) = draw_funds::<500, 500, 1000>(account);
        let (bolts, change, vendor): (PurchasedBoltBox, _, _) =
            purchase::<_, 500, 400, 100>(new_vendor(), tendered);
        let account = deposit::<100, 500, 600>(account, change);
        close_account(account);
        // The goods are real objects (R12): this module's own test may keep
        // them to the end of the test, inside the privacy boundary.
        let _goods = bolts;
        let _vendor_stays_at_the_boundary = vendor;
    }

    /// Exact payment: no change needed, the split is 400 + 0 — `Money<0>` is
    /// a real (empty) resource that still has to be accounted for, like any
    /// R15 empty state.
    #[test]
    fn exact_payment_leaves_zero_change_which_is_still_accounted() {
        let account = open_account::<400>();
        let (tendered, account) = draw_funds::<400, 0, 400>(account);
        let (bolts, no_change, vendor) = purchase::<_, 400, 400, 0>(new_vendor(), tendered);
        let account = deposit::<0, 0, 0>(account, no_change);
        close_account(account);
        let _goods = bolts;
        let _vendor = vendor;
    }

    /// The conservation regime covers money like any resource (R19):
    /// abandoned change trips the tripwire at test time (R1 layer 2, F-032),
    /// naming the type and the decimal amount (F-027).
    #[test]
    #[should_panic(expected = "Money<100> dropped without being consumed")]
    fn abandoned_change_trips_the_tripwire() {
        let account = open_account::<500>();
        let (tendered, account) = draw_funds::<500, 0, 500>(account);
        let (price, change) = split_money::<500, 400, 100>(tendered);
        let vendor = send_to(new_vendor(), price);
        close_account(account);
        let _vendor = vendor;
        let _change_left_on_the_counter = change; // falls out of scope: leak
    }
}
