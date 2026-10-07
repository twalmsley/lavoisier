//! The full-batch integration test (SPEC §6): drives the production flow
//! `run_batch` (P1 → P5's 25 recursive cycles → P6) and asserts the end
//! state — everything accounted (R1, R9).
//!
//! History policy per the agreed SPEC review (§10): assertions on the
//! **totals** and **spot checks** of the ~102-event record, not all 102
//! events one by one.
//!
//! The recursion limit below is F-010 measured again at CS-4's scale, with a
//! twist worth recording: the attribute is per **crate**, and an integration
//! test is its own crate — naming `StockedFinishedGoods` (whose type carries
//! the 25-deep rack, 100-deep box and 25-assembly contents) here overflowed
//! the default limit (E0275) even though both library crates already carry
//! the attribute.

#![recursion_limit = "2048"]

use cs4_line::flows::{StockedFinishedGoods, run_batch};
use cs4_line::resources::EmptySwarfBin;
use cs4_stores::resources::boundary::ship_finished_goods;
use cs4_stores::resources::{BATCH_SWARF_G, Operator, QUANTUM_MS};
use model_core::history::Entry;
use model_core::nat::aliases::N17;

/// SPEC §6 — everything accounted at batch end: 25 assemblies in finished
/// goods; 13 000 g of swarf at disposal; the bin back empty; box and rack
/// back empty (consumed inside P6); the note reconciled; the operator back
/// with 510 000 ms; the tools back; and the single History holding exactly
/// 102 attributed events (2 stores draws + 25 × 4 cycle draws) totalling
/// 3 990 000 ms.
///
/// Verifies: REQ-019, REQ-020, REQ-021, REQ-022
#[test]
fn the_batch_ends_fully_accounted() {
    let out = run_batch();

    // 25 assemblies in finished goods (REQ-022): the store's contents list
    // IS the count (R12).
    assert_eq!(StockedFinishedGoods::HELD, 25);

    // The bin is back empty with the line (REQ-020)...
    assert_eq!(EmptySwarfBin::HELD, 0);
    // ...and the disposal total was const-asserted inside P6 (13 000 g);
    // restate the arithmetic the model carries (SPEC §5).
    assert_eq!(BATCH_SWARF_G, 25 * 500 + 50 * 10);

    // The operator is back with 510 000 ms:
    // 4 500 000 − 120 000 − 3 750 000 − 120 000 (SPEC §6).
    assert_eq!(Operator::<N17>::REMAINING_MS, 510_000);
    assert_eq!(Operator::<N17>::QUANTA * QUANTUM_MS, 510_000);

    // The History: exactly 102 events (2 + 25 × 4), all attributed, totals
    // and spot checks (SPEC §10 decision 6).
    assert_eq!(out.history.event_count(), 102);
    let mut total_ms = 0u64;
    let (mut issued, mut cuts, mut drills, mut fastens, mut reconciled) = (0, 0, 0, 0, 0);
    for entry in out.history.entries() {
        match entry {
            Entry::Event(e) => {
                assert_eq!(e.item, "Effort");
                assert_eq!(e.unit, "person-milliseconds");
                total_ms += e.magnitude;
                match e.process {
                    "issue_materials" => issued += 1,
                    "cut" => cuts += 1,
                    "drill_blank" => drills += 1,
                    "fasten" => fastens += 1,
                    "reconcile" => reconciled += 1,
                    other => panic!("unexpected process in the record: {other}"),
                }
            }
            Entry::Join(..) => panic!("one operator, one History: the record must be flat (R16)"),
        }
    }
    assert_eq!(total_ms, 3_990_000);
    assert_eq!((issued, cuts, drills, fastens, reconciled), (1, 25, 50, 25, 1));
    // Spot checks: the record starts at the stores window, the first cycle
    // opens with a 60 000 ms cut, and the record closes with reconciliation.
    let entries = out.history.entries();
    assert!(matches!(&entries[0], Entry::Event(e) if e.process == "issue_materials" && e.magnitude == 120_000));
    assert!(matches!(&entries[1], Entry::Event(e) if e.process == "cut" && e.magnitude == 60_000));
    assert!(matches!(&entries[4], Entry::Event(e) if e.process == "fasten" && e.magnitude == 30_000));
    assert!(matches!(&entries[101], Entry::Event(e) if e.process == "reconcile" && e.magnitude == 120_000));

    // The stocked finished goods leave the model through stores' sealed exit
    // (F-039); the rest of the end state legitimately stays with the caller.
    ship_finished_goods(out.finished_goods);
    let _stays_with_the_caller = (
        out.bin,
        out.disposal,
        out.note,
        out.saw,
        out.press,
        out.bench,
        out.operator,
        out.history,
    );
}
