//! The full batch flow (SPEC §6): P1 → P5 (25 recursive cycles) → P6, as a
//! **production** function — so the plain `cargo build` in ci.sh
//! monomorphizes all 25 cycles and every conservation assert in them
//! (F-001), and SPEC §7's change-impact probes blast through real production
//! instantiations, not just tests.
//!
//! Within a cycle the only ordering freedom is the trivial reordering of the
//! two drills (stated in [`crate::batch`]'s cycle body); between processes
//! the data dependencies fix P1 → P5 → P6 (R9). Everything is accounted at
//! batch end in the returned [`BatchOutcome`]; the integration test
//! `tests/flows.rs` asserts SPEC §6's end state over it.

use crate::batch::{BatchRig, BuildBatch, rig_up, stand_down};
use crate::resources::boundary::{new_swarf_bin, tool_up};
use crate::resources::{EmptySwarfBin, PillarDrill, Saw, Workbench};
use cs4_stores::resources::boundary::{clock_in, new_disposal, new_finished_goods, place_works_order};
use cs4_stores::resources::processes::{issue_materials, reconcile};
use cs4_stores::resources::{BATCH_SWARF_G, BATCH_SWARF_PIECES, Disposal, FinishedGoods, FullBoltBox, FullSheetRack, Operator, ReconciledNote};
use model_core::boundary::ConsumeList;
use model_core::history::History;
use model_core::history::boundary::new_history;
use model_core::nat::aliases::{N17, N25, N80, N146};

/// The rig as P1 hands it to the line: the full 25-sheet rack, the full
/// 100-bolt box, the empty 80-slot bin, and the operator four quanta
/// (120 000 ms) into the 150-quantum shift.
pub type StartRig = BatchRig<FullSheetRack, FullBoltBox, EmptySwarfBin, N146>;

/// The batch's product: 25 [`crate::batch::BracketAssembly`]s as a
/// type-level list of real values (R13) — the compiler computes it from the
/// recursion.
pub type BatchAssemblies = <StartRig as BuildBatch<N25>>::Assemblies;

/// The rig after the 25 cycles: rack and box empty, bin holding 75 pieces
/// (space 5 of 80), clock at 21 quanta.
pub type SpentRig = <StartRig as BuildBatch<N25>>::Rest;

/// The finished-goods store after P6 delivers the batch: space `Zero`,
/// contents the 25 assemblies — SPEC §6's "25 assemblies in finished goods"
/// as a type.
pub type StockedFinishedGoods = <FinishedGoods<N25> as ConsumeList<BatchAssemblies>>::Next;

/// Everything the batch ends with (SPEC §6), fully accounted: a grouping
/// with public fields (F-046) — building one requires already holding the
/// resources, so it cannot mint.
#[must_use = "BatchOutcome carries the batch's conserved end state: every field must be accounted for"]
pub struct BatchOutcome {
    /// 25 assemblies in finished goods (REQ-022).
    pub finished_goods: StockedFinishedGoods,
    /// The bin, back empty with the line (REQ-020).
    pub bin: EmptySwarfBin,
    /// The disposal stream, 13 000 g of swarf heavier (REQ-020).
    pub disposal: Disposal,
    /// The issue note, reconciled (REQ-021 closed).
    pub note: ReconciledNote,
    /// The saw, returned (R2).
    pub saw: Saw,
    /// The pillar drill, returned (R2).
    pub press: PillarDrill,
    /// The workbench, returned (R2).
    pub bench: Workbench,
    /// The operator, back with 17 quanta = 510 000 ms
    /// (4 500 000 − 120 000 − 3 750 000 − 120 000).
    pub operator: Operator<N17>,
    /// The single execution record (R16): ~102 attributed events
    /// (2 stores draws + 25 × 4 cycle draws).
    pub history: History,
}

/// The whole batch (SPEC §6): issue against the works order (P1), build the
/// 25 assemblies by type-level recursion (P5), return and reconcile (P6).
pub fn run_batch() -> BatchOutcome {
    // One operator, one History (SPEC §1: concurrency is CS-3's).
    let history = new_history();
    // P1 — stores issues the batch's materials against the works order.
    let (rack, bolt_box, note, operator, history) =
        issue_materials(place_works_order(), clock_in(), history);
    // The line tools up and rigs the batch.
    let (saw, press, bench) = tool_up();
    let rig: StartRig = rig_up(rack, bolt_box, new_swarf_bin::<N80>(), note, saw, press, bench, operator, history);
    // P5 — the 25 cycles, unrolled by the compiler.
    let (assemblies, rig) = <StartRig as BuildBatch<N25>>::build_batch(rig);
    let (rack, bolt_box, bin, note, saw, press, bench, operator, history) = stand_down(rig);
    // P6 — return and reconcile: assemblies to finished goods (REQ-022),
    // the bin's 75 pieces / 13 000 g to disposal (REQ-020, const-asserted),
    // empties and note back to stores (REQ-021 closed).
    let (finished_goods, bin, disposal, note, operator, history) = reconcile::<BATCH_SWARF_G, BATCH_SWARF_PIECES, _, _, _, _, _>(assemblies, new_finished_goods::<N25>(), bin, new_disposal(), bolt_box, rack, note, operator, history);
    BatchOutcome {
        finished_goods,
        bin,
        disposal,
        note,
        saw,
        press,
        bench,
        operator,
        history,
    }
}
