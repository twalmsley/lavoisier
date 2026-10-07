//! Integration tests for the CS-5 fulfilment (SPEC §6): both orderings run
//! end to end with **everything accounted** at the far side — downstream
//! style, through public APIs only (this crate cannot defuse anything of
//! `cs5-supply`'s; the account settles through the boundary, the vendor
//! withdraws through its own exit).

use cs5_supply::resources::boundary::vendor_returns_to_its_hinterland;
use cs5_supply::resources::{TwoBoxedComponents, Vendor};
use cs5_works::flows::{FulfilmentComplete, fulfil_order, fulfil_order_bin_last};
use cs5_works::resources::boundary::settle_up;
use cs5_works::resources::{Account, EmptyBin, HousingRack, TwoHousings};
use model_core::history::Entry;
use model_core::common::Person;

/// Checks a finished fulfilment against SPEC §6's "everything accounted"
/// list, then retires the end state through the boundaries: instrument with
/// the customer (consumed inside the flow); account at 12 000 p; vendor paid
/// and at stock 2; courier home; packaging at disposal; bin empty; order
/// consumed; rack at 2; operator at 570 000 ms; History with the caller,
/// holding the 7 attributed events in flow order.
fn assert_everything_accounted(done: FulfilmentComplete, expected_events: [(&str, u64); 7]) {
    let FulfilmentComplete {
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
    } = done;

    // The books (SPEC §4/§6): each currency dimension balances separately.
    assert_eq!(Account::<12_000>::BALANCE_PENCE, 12_000);
    // The refined vendor's accounted end state (SPEC §6): stock at 2.
    assert_eq!(Vendor::<TwoBoxedComponents>::STOCK, 2);
    // The rack and the bin (SPEC §6): rack at 2, bin empty.
    assert_eq!(HousingRack::<TwoHousings>::COUNT, 2);
    assert_eq!(EmptyBin::HELD, 0);
    // The operator (SPEC §6): 630 000 ms drawn of 1 200 000.
    assert_eq!(Person::<570_000>::BUDGET_MS, 570_000);

    // The single History: 7 attributed events (P3 has no draw), flat and in
    // flow order — one actor, no concurrency, no joins (R16).
    assert_eq!(history.event_count(), 7);
    let entries = history.entries();
    assert_eq!(entries.len(), 7);
    for (entry, (process, ms)) in entries.iter().zip(expected_events) {
        match entry {
            Entry::Event(event) => {
                assert_eq!(event.process, process);
                assert_eq!(event.magnitude, ms);
                assert_eq!(event.unit, "person-milliseconds");
            }
            other => panic!("expected a flat attributed event, got {other:?}"),
        }
    }

    // Retire the end state through the boundaries (R1): the takings bank,
    // the vendor withdraws with its remaining stock; the reusables and the
    // untripwired kept stock stay with the caller to scope end.
    settle_up(account);
    vendor_returns_to_its_hinterland(vendor);
    let _stay_with_the_caller = (customer, courier, bureau, rack, bin, disposal, operator, history);
}

/// Ordering 1 (SPEC §6): housing picked first, bin emptied straight after
/// inspection — compiles and accounts for everything.
///
/// Verifies: REQ-023, REQ-024, REQ-025, REQ-026, REQ-027
#[test]
fn fulfilment_with_early_housing_and_early_bin_accounts_for_everything() {
    assert_everything_accounted(
        fulfil_order(),
        [
            ("exchange", 120_000),
            ("consign", 60_000),
            ("hand_over", 60_000),
            ("inspect", 60_000),
            ("empty_bin", 30_000),
            ("assemble", 180_000),
            ("deliver", 120_000),
        ],
    );
}

/// Ordering 2 (SPEC §6): housing picked just before assembly, bin emptied
/// last — compiles to the **same end state type** as ordering 1, which is
/// the type system proving the orderings equivalent (R9).
///
/// Verifies: REQ-023, REQ-024, REQ-025, REQ-026, REQ-027
#[test]
fn fulfilment_with_late_housing_and_late_bin_reaches_the_same_end_state() {
    assert_everything_accounted(
        fulfil_order_bin_last(),
        [
            ("exchange", 120_000),
            ("consign", 60_000),
            ("hand_over", 60_000),
            ("inspect", 60_000),
            ("assemble", 180_000),
            ("deliver", 120_000),
            ("empty_bin", 30_000),
        ],
    );
}
