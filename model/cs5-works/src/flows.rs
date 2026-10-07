//! The two-site fulfilment flow (SPEC §6): P1 → P2 → P3 → P4 → P5 → P6 →
//! P7 → P8, composed from the three crates' public model APIs only (boundary
//! constructors, the processes, the `Consumer` sinks) — no `mint`/`defuse` —
//! so it composes exactly the way a downstream flow would.
//!
//! **The one genuine ordering freedom** (SPEC §6): the housing may be picked
//! from the rack at any point before P6, and P8 may run any time after P5.
//! [`fulfil_order`] picks the housing before the purchase even starts and
//! empties the bin straight after inspection; [`fulfil_order_bin_last`]
//! picks the housing just before assembly and empties the bin at the very
//! end. **Both compile and both end in the identical
//! [`FulfilmentComplete`] state** — the type system proving the orderings
//! equivalent (R9). The intermediate operator budgets differ between the
//! orderings (the hand-maintained F-030 balances), but the end state is one
//! type.
//!
//! **The cross-currency books** (SPEC §6): GBP and EUR never meet in one
//! assert. The GBP dimension balances as 5000 − 2000 + 9000 = 12 000 (the
//! account's asserts plus the top-level const item in
//! [`crate::resources`]); the euro-cent dimension opens at the bureau
//! (2340 ec minted against the 2000 p, REQ-024's rate assert) and closes at
//! the vendor (the exact-price impl, REQ-023). The bureau's
//! value-equivalence is the only bridge, stated at its rate.
//!
//! **Time** (R15/R16): the single operator enters with 1 200 000 ms and
//! every drawing process has an **adjacent** `draw_time` in the flow
//! (F-048), recorded into the single `History` under the process name —
//! 7 attributed events (P3 has no draw: the courier and vendor are the
//! actors), 630 000 ms drawn, operator home at 570 000 ms.

use crate::resources::boundary::{new_bin, new_customer, open_the_books, place_order, present_payment, stock_the_rack};
use crate::resources::processes::{assemble, deliver, draw_cash, empty_bin, inspect};
use crate::resources::{Account, CLOSING_BALANCE_PENCE, Customer, EmptyBin, HousingRack, INSTRUMENT_G, Instrument, TwoHousings};
use cs5_logistics::resources::CourierAtUk;
use cs5_logistics::resources::boundary::courier_reports_for_duty;
use cs5_logistics::resources::processes::{carry_to_vendor, consign, hand_over, purchase_at_vendor};
use cs5_supply::resources::boundary::{new_bureau, new_packaging_disposal, place_purchase_order, vendor_opens_for_business};
use cs5_supply::resources::{Bureau, Euros, InspectedComponent, Money, PackagingDisposal, TwoBoxedComponents, Vendor};
use model_core::boundary::take_one;
use model_core::common::Person;
use model_core::common::boundary::new_person;
use model_core::common::processes::draw_time;
use model_core::history::History;
use model_core::history::boundary::new_history;
use model_core::history::processes::record;

/// Everything the fulfilment leaves behind, fully accounted (SPEC §6): one
/// grouping with public resource fields (F-046 — a grouping, not a resource:
/// building it requires already holding the sealed values). **Both flow
/// orderings return exactly this type**, which is the compiler's statement
/// that the orderings are equivalent (R9).
#[must_use = "FulfilmentComplete bundles conserved outputs: every field must be accounted for"]
pub struct FulfilmentComplete {
    /// The account at its closing balance (5000 − 2000 + 9000 = 12 000 p);
    /// settled at the boundary by the caller.
    pub account: Account<CLOSING_BALANCE_PENCE>,
    /// The customer, instrument in hand, order fulfilled (boundary object).
    pub customer: Customer,
    /// The courier, home at the UK site and reusable (REQ-027's loop
    /// closed).
    pub courier: CourierAtUk,
    /// The vendor, paid in full in its own currency, stock at 2 — the
    /// refined placeholder's accounted end state; it withdraws to its
    /// hinterland at the boundary.
    pub vendor: Vendor<TwoBoxedComponents>,
    /// The bureau, unchanged (the deliberately unrefined placeholder).
    pub bureau: Bureau,
    /// The rack, two housings left (untripwired kept items).
    pub rack: HousingRack<TwoHousings>,
    /// The bin, emptied at P8, all five spaces restored.
    pub bin: EmptyBin,
    /// The disposal stream, holding the model's 50 g of packaging outside
    /// it (F-029).
    pub disposal: PackagingDisposal,
    /// The operator, home at 570 000 ms of the 1 200 000 ms budget
    /// (630 000 ms drawn, SPEC §6).
    pub operator: Person<570_000>,
    /// The single execution record: 7 attributed events (P3 has no draw).
    pub history: History,
}

/// The fulfilment, first ordering: the housing is picked from the rack
/// **before anything else**, and the bin is emptied (P8) **straight after
/// inspection** (P5) — the earliest legal slots for both free choices
/// (SPEC §6).
pub fn fulfil_order() -> FulfilmentComplete {
    // The world at flow start (SPEC §4): boundary objects in, stock filled.
    let operator = new_person::<1_200_000>();
    let history = new_history();
    let account = open_the_books();
    let bureau = new_bureau();
    let vendor = vendor_opens_for_business();
    let courier = courier_reports_for_duty();
    let customer = new_customer();
    let rack = stock_the_rack();
    let bin = new_bin();
    let disposal = new_packaging_disposal();
    let (order_token, customer) = place_order(customer);

    // Ordering freedom, choice 1: the housing comes off the rack first.
    let (housing, rack) = take_one(rack);

    // P1 — exchange currency at the bureau (REQ-024): 120 000 ms, 2000 p
    // drawn (5000 → 3000, assert), 2340 ec minted at the stated rate.
    let (labour, operator) = draw_time::<120_000, 1_080_000, 1_200_000>(operator);
    let history = record(history, "exchange_currency", labour);
    let (sterling, account): (Money<2000>, Account<3000>) = draw_cash(account);
    let (euro_cents, bureau): (Euros<2340>, _) = cs5_supply::resources::processes::exchange(bureau, sterling);

    // P2 — consign the courier (REQ-027): 60 000 ms; only the order travels.
    let (labour, operator) = draw_time::<60_000, 1_020_000, 1_080_000>(operator);
    let history = record(history, "consign_courier", labour);
    let purchase_order = place_purchase_order();
    let courier = consign(courier, purchase_order);

    // P3 — purchase at the vendor (REQ-023 + REQ-027): no operator draw
    // (the courier and vendor are the actors); the remitted 2340 ec is
    // flow-routed from P1's output (SPEC §7).
    let courier = carry_to_vendor(courier);
    let (courier, vendor) = purchase_at_vendor(courier, vendor, euro_cents);

    // P4 — receive at the works: 60 000 ms; courier home for reuse.
    let (labour, operator) = draw_time::<60_000, 960_000, 1_020_000>(operator);
    let history = record(history, "receive_goods", labour);
    let (boxed, courier) = hand_over(courier);

    // P5 — goods-in inspection (REQ-025): 60 000 ms; 450 = 400 + 50
    // (assert); packaging fed to the bin inside the process.
    let (labour, operator) = draw_time::<60_000, 900_000, 960_000>(operator);
    let history = record(history, "goods_in_inspection", labour);
    let (component, bin) = inspect::<50, _>(boxed, bin);

    // Ordering freedom, choice 2: P8 runs straight after P5 — empty the bin
    // (30 000 ms; 50 = 50, assert via contents; F-039 disposal path).
    let (labour, operator) = draw_time::<30_000, 870_000, 900_000>(operator);
    let history = record(history, "empty_bin", labour);
    let (bin, disposal) = empty_bin::<50, _, _, _>(bin, disposal);

    // P6 — assemble the instrument (REQ-025): 180 000 ms; 600 + 400 = 1000
    // (assert).
    let (labour, operator) = draw_time::<180_000, 690_000, 870_000>(operator);
    let history = record(history, "assemble_instrument", labour);
    let instrument: Instrument<InspectedComponent, INSTRUMENT_G> = assemble(housing, component);

    // P7 — deliver against the order token with payment taken (REQ-026):
    // 120 000 ms; account 3000 → 12 000 (assert).
    let (labour, operator) = draw_time::<120_000, 570_000, 690_000>(operator);
    let history = record(history, "deliver_and_take_payment", labour);
    let (payment, customer) = present_payment(customer);
    let (account, customer): (Account<CLOSING_BALANCE_PENCE>, Customer) =
        deliver(instrument, order_token, payment, customer, account);

    FulfilmentComplete {
        account,
        customer,
        courier,
        vendor,
        bureau,
        rack,
        bin,
        disposal,
        operator,
        history,
    }
}

/// The fulfilment, second ordering: the housing is picked from the rack
/// **just before assembly**, and the bin is emptied (P8) **last of all** —
/// the latest legal slots for both free choices (SPEC §6). Returns the
/// identical [`FulfilmentComplete`] end state as [`fulfil_order`]: the type
/// system proving the two orderings equivalent (R9).
pub fn fulfil_order_bin_last() -> FulfilmentComplete {
    let operator = new_person::<1_200_000>();
    let history = new_history();
    let account = open_the_books();
    let bureau = new_bureau();
    let vendor = vendor_opens_for_business();
    let courier = courier_reports_for_duty();
    let customer = new_customer();
    let rack = stock_the_rack();
    let bin = new_bin();
    let disposal = new_packaging_disposal();
    let (order_token, customer) = place_order(customer);

    // P1 (120 000 ms; 5000 → 3000; 2000 p → 2340 ec).
    let (labour, operator) = draw_time::<120_000, 1_080_000, 1_200_000>(operator);
    let history = record(history, "exchange_currency", labour);
    let (sterling, account): (Money<2000>, Account<3000>) = draw_cash(account);
    let (euro_cents, bureau): (Euros<2340>, _) = cs5_supply::resources::processes::exchange(bureau, sterling);

    // P2 (60 000 ms).
    let (labour, operator) = draw_time::<60_000, 1_020_000, 1_080_000>(operator);
    let history = record(history, "consign_courier", labour);
    let courier = consign(courier, place_purchase_order());

    // P3 (no draw).
    let courier = carry_to_vendor(courier);
    let (courier, vendor) = purchase_at_vendor(courier, vendor, euro_cents);

    // P4 (60 000 ms).
    let (labour, operator) = draw_time::<60_000, 960_000, 1_020_000>(operator);
    let history = record(history, "receive_goods", labour);
    let (boxed, courier) = hand_over(courier);

    // P5 (60 000 ms; 450 = 400 + 50; packaging to the bin inside).
    let (labour, operator) = draw_time::<60_000, 900_000, 960_000>(operator);
    let history = record(history, "goods_in_inspection", labour);
    let (component, bin) = inspect::<50, _>(boxed, bin);

    // Ordering freedom, choice 1 (late): the housing comes off the rack
    // only now, just before P6.
    let (housing, rack) = take_one(rack);

    // P6 (180 000 ms; 600 + 400 = 1000).
    let (labour, operator) = draw_time::<180_000, 720_000, 900_000>(operator);
    let history = record(history, "assemble_instrument", labour);
    let instrument: Instrument<InspectedComponent, INSTRUMENT_G> = assemble(housing, component);

    // P7 (120 000 ms; 3000 → 12 000).
    let (labour, operator) = draw_time::<120_000, 600_000, 720_000>(operator);
    let history = record(history, "deliver_and_take_payment", labour);
    let (payment, customer) = present_payment(customer);
    let (account, customer): (Account<CLOSING_BALANCE_PENCE>, Customer) =
        deliver(instrument, order_token, payment, customer, account);

    // Ordering freedom, choice 2 (late): P8 runs last (30 000 ms; 50 = 50).
    let (labour, operator) = draw_time::<30_000, 570_000, 600_000>(operator);
    let history = record(history, "empty_bin", labour);
    let (bin, disposal) = empty_bin::<50, _, _, _>(bin, disposal);

    FulfilmentComplete {
        account,
        customer,
        courier,
        vendor,
        bureau,
        rack,
        bin,
        disposal,
        operator,
        history,
    }
}
