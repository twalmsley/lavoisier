//! P5 — the batch as **type-level recursion** (SPEC §5): the scale
//! centrepiece of CS-4.
//!
//! [`BuildBatch<N>`] is model-core's `SupplyN` pattern (F-014) at batch
//! scale: a base case at `Zero` (building nothing changes nothing) and a
//! step case at `Succ<N>` (one cycle, then `N` more on the evolved rig). The
//! two impls do not overlap (the `N` differs), so coherence is satisfied,
//! and any batch size is one bound: `StartRig: BuildBatch<N25>`.
//!
//! **Why not a loop:** every cycle changes the types of everything threaded
//! — the rack loses a sheet, the box four bolts (100 → 96 → … → 0), the bin
//! gains three pieces of swarf (0 → 75, space 80 → 5), the operator's shift
//! clock five quanta (R15 deviation, see `cs4-stores`). Loop-carried state
//! needs one type; this state refuses one, so the repetition is recursion
//! that the compiler unrolls at monomorphization — the honest, and
//! measurably expensive, encoding (SPEC §8, F-011 at composite depth).
//!
//! **The rig aggregate is legitimate here** (F-024's own criterion): every
//! cycle uses *every* field — rack, box, bin, note, all three tools, the
//! operator and the History — so [`BatchRig`] over-claims nothing, and it is
//! what keeps the recursive impl's where-clauses to six lines instead of a
//! per-field tangle. The bin's three per-cycle consumes still cost one
//! projection alias each ([`AfterCut`], [`AfterBore`]) — the F-014
//! chained-bounds shape, tamed the way EXP-02's `NextK` aliases were.
//!
//! Draws are **adjacent** to each process in the cycle body (F-048) and
//! recorded under the process names (R16): cut 60 000 ms, each drill
//! 30 000 ms, fasten 30 000 ms — 150 000 ms per cycle, four History events
//! per cycle, 100 across the batch (SPEC §5/§6).

use crate::processes::{cut, drill_blank, fasten};
use crate::resources::{PillarDrill, Saw, Workbench};
use cs4_stores::resources::processes::{draw_effort_30k, draw_effort_60k};
use cs4_stores::resources::{ASSEMBLY_G, Assembly, CUT_SWARF_G, CutSwarf, DRILL_SWARF_G, DrillSwarf, FourOf, IssueNote, IssuedBolt, Operator, SHEET_G, Sheet};
use model_core::boundary::{Consumer, Supplier, SupplyN, take_one};
use model_core::history::History;
use model_core::history::processes::record;
use model_core::list::{Cons, Nil};
use model_core::nat::aliases::N4;
use model_core::nat::{Nat, Succ, Zero};

/// The batch's one assembly type: two 2240 g drilled plates and four issued
/// bolts, 4600 g (SPEC §3).
pub type BracketAssembly = Assembly<IssuedBolt, ASSEMBLY_G>;

/// The bin after it takes a cycle's cut swarf.
pub type AfterCut<B> = <B as Consumer<CutSwarf<CUT_SWARF_G>>>::Next;
/// The bin after it takes one blank's drill swarf.
pub type AfterBore<B> = <B as Consumer<DrillSwarf<DRILL_SWARF_G>>>::Next;
/// The bin after one full cycle: cut swarf plus two drills' swarf (3 pieces).
pub type BinAfterCycle<B> = AfterBore<AfterBore<AfterCut<B>>>;

/// Everything one batch cycle threads (SPEC §5/P5): the rack, the bolt box,
/// the bin, the issue note (REQ-021), the three tools, the operator's shift
/// clock and the execution History. An aggregate is legitimate here because
/// **every** cycle uses **every** field (F-024's criterion); its type
/// parameters are exactly the state that evolves per cycle.
///
/// A grouping constructor/decomposer pair ([`rig_up`]/[`stand_down`]) moves
/// resources in and out by value — building a rig requires already holding
/// everything in it, so the rig cannot mint (F-046).
#[must_use = "BatchRig carries the batch's conserved resources: build the batch or stand it down"]
pub struct BatchRig<Rack, Bolts, Bin, Q: Nat> {
    rack: Rack,
    bolts: Bolts,
    bin: Bin,
    note: IssueNote,
    saw: Saw,
    press: PillarDrill,
    bench: Workbench,
    operator: Operator<Q>,
    history: History,
}

/// Assembles the rig from its nine threaded resources (by value — the rig is
/// a grouping, not a resource, F-046).
// Nine arguments: the rig groups exactly the state SPEC §5/P5 threads, and
// loose arguments keep each resource's hand-over visible (R9, F-024).
#[allow(clippy::too_many_arguments)]
pub fn rig_up<Rack, Bolts, Bin, Q: Nat>(rack: Rack, bolts: Bolts, bin: Bin, note: IssueNote, saw: Saw, press: PillarDrill, bench: Workbench, operator: Operator<Q>, history: History) -> BatchRig<Rack, Bolts, Bin, Q> {
    BatchRig {
        rack,
        bolts,
        bin,
        note,
        saw,
        press,
        bench,
        operator,
        history,
    }
}

/// Decomposes the rig back into its nine resources after the batch, for the
/// reconciliation hand-over (P6).
// The nine-element tuple mirrors rig_up's arguments one-to-one; naming a
// struct for a single decompose site would only move the same list (R9).
#[allow(clippy::type_complexity)]
pub fn stand_down<Rack, Bolts, Bin, Q: Nat>(rig: BatchRig<Rack, Bolts, Bin, Q>) -> (Rack, Bolts, Bin, IssueNote, Saw, PillarDrill, Workbench, Operator<Q>, History) {
    let BatchRig {
        rack,
        bolts,
        bin,
        note,
        saw,
        press,
        bench,
        operator,
        history,
    } = rig;
    (rack, bolts, bin, note, saw, press, bench, operator, history)
}

/// P5 — builds `N` bracket assemblies by type-level recursion (SPEC §5): the
/// `SupplyN` pattern (F-014) at batch scale. Implemented for a rig whose
/// rack, box, bin and shift clock can cover one cycle *and* whose evolved
/// rig can build `N − 1` more; a rig that would run out partway implements
/// nothing, so an oversized batch fails at type-check time (the 26th-cycle
/// probe in the crate docs).
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot build a batch of this size: the sheet rack, bolt box, swarf bin or shift clock would run out partway (P5)",
    label = "this rig cannot cover every cycle of the requested batch",
    note = "the issued rig covers exactly 25 cycles (SPEC: 25 sheets, 100 bolts, bin space 80 for 75 pieces, 125 shift quanta after issue) - a 26th cycle finds the rack empty (R12, F-015)"
)]
pub trait BuildBatch<N>: Sized {
    /// The assemblies built, as a type-level list of real values (R13) —
    /// `Cons<BracketAssembly, …>` `N` deep.
    type Assemblies;
    /// The rig after the batch, every threaded type evolved.
    type Rest;
    /// Builds the batch: `N` cycles of cut → drill ×2 → fasten, 150 000 ms
    /// drawn per cycle in four process-named draws (R16).
    fn build_batch(self) -> (Self::Assemblies, Self::Rest);
}

/// Base case: a batch of zero builds nothing and leaves the rig untouched.
impl<Rack, Bolts, Bin, Q: Nat> BuildBatch<Zero> for BatchRig<Rack, Bolts, Bin, Q> {
    type Assemblies = Nil;
    type Rest = Self;
    fn build_batch(self) -> (Nil, Self) {
        (Nil, self)
    }
}

/// Step case: one cycle — take a sheet, cut, drill both blanks, fasten —
/// then recurse on the evolved rig. The clock type on `Self` spells the
/// cycle's five quanta (2 + 1 + 1 + 1 = 150 000 ms); the where-clauses spell
/// the cycle's stock and bin needs; the final bound hands the evolved rig to
/// `BuildBatch<N>`.
impl<Rack, Bolts, Bin, Q: Nat, N> BuildBatch<Succ<N>> for BatchRig<Rack, Bolts, Bin, Succ<Succ<Succ<Succ<Succ<Q>>>>>>
where
    Rack: Supplier<Item = Sheet<SHEET_G>>,
    Bolts: SupplyN<N4, Taken = FourOf<IssuedBolt>>,
    Bin: Consumer<CutSwarf<CUT_SWARF_G>>,
    AfterCut<Bin>: Consumer<DrillSwarf<DRILL_SWARF_G>>,
    AfterBore<AfterCut<Bin>>: Consumer<DrillSwarf<DRILL_SWARF_G>>,
    BatchRig<Rack::Next, Bolts::Rest, BinAfterCycle<Bin>, Q>: BuildBatch<N>,
{
    type Assemblies = Cons<BracketAssembly, <BatchRig<Rack::Next, Bolts::Rest, BinAfterCycle<Bin>, Q> as BuildBatch<N>>::Assemblies>;
    type Rest = <BatchRig<Rack::Next, Bolts::Rest, BinAfterCycle<Bin>, Q> as BuildBatch<N>>::Rest;

    fn build_batch(self) -> (Self::Assemblies, Self::Rest) {
        let BatchRig {
            rack,
            bolts,
            bin,
            note,
            saw,
            press,
            bench,
            operator,
            history,
        } = self;
        // P2 — cut one sheet (adjacent draw, recorded under the process
        // name: 60 000 ms, R16/F-048).
        let (effort, operator) = draw_effort_60k(operator);
        let history = record(history, "cut", effort);
        let (sheet, rack) = take_one(rack);
        let (saw, blank_a, blank_b, bin) = cut(saw, sheet, bin);
        // P3 — drill both blanks (30 000 ms each; the only ordering freedom
        // in a cycle is swapping these two draws+drills — stated, not
        // celebrated: concurrency is out of scope, SPEC §6).
        let (effort, operator) = draw_effort_30k(operator);
        let history = record(history, "drill_blank", effort);
        let (press, plate_a, bin) = drill_blank(press, blank_a, bin);
        let (effort, operator) = draw_effort_30k(operator);
        let history = record(history, "drill_blank", effort);
        let (press, plate_b, bin) = drill_blank(press, blank_b, bin);
        // P4 — fasten (30 000 ms; four bolts in one SupplyN bound, F-014;
        // the issue note threaded, REQ-021).
        let (effort, operator) = draw_effort_30k(operator);
        let history = record(history, "fasten", effort);
        let (bench, assembly, bolts, note) = fasten(bench, plate_a, plate_b, bolts, note);
        // Recurse on the evolved rig: every threaded type one cycle further
        // on. The compiler unrolls this at monomorphization (SPEC §8).
        let rig = BatchRig {
            rack,
            bolts,
            bin,
            note,
            saw,
            press,
            bench,
            operator,
            history,
        };
        let (rest_of_batch, spent_rig) = rig.build_batch();
        (Cons(assembly, rest_of_batch), spent_rig)
    }
}

#[cfg(test)]
mod tests {
    //! P5 unit test (R5): one recursion step in isolation — the full 25-cycle
    //! batch is the integration test in `tests/flows.rs`.

    use super::{BuildBatch, rig_up, stand_down};
    use crate::resources::boundary::{new_swarf_bin, tool_up};
    use cs4_stores::resources::boundary::{new_disposal, new_finished_goods, ship_finished_goods};
    use cs4_stores::resources::test_support::{fixture_box, fixture_rack, fixture_settle_note};
    use cs4_stores::resources::{IssueNote, Operator, SwarfReturn};
    use model_core::boundary::{send_list, send_to};
    use model_core::history::boundary::new_history;
    use model_core::nat::aliases::{N0, N1, N3, N4, N5};

    /// One cycle through the recursion: a one-sheet rack, a four-bolt box, a
    /// three-slot bin and a five-quantum clock cover exactly `BuildBatch<N1>`
    /// — and everything the cycle touched is accounted at the end (R1, R5).
    ///
    /// Verifies: REQ-019, REQ-020, REQ-021, REQ-022
    #[test]
    fn one_cycle_of_the_recursion_builds_one_assembly() {
        let (saw, press, bench) = tool_up();
        let rig = rig_up(fixture_rack::<N1>(), fixture_box::<N4>(), new_swarf_bin::<N3>(), IssueNote::test_fixture(), saw, press, bench, Operator::<N5>::test_fixture(), new_history());
        let (assemblies, rig) = BuildBatch::<N1>::build_batch(rig);
        let (rack, bolts, bin, note, saw, press, bench, operator, history) = stand_down(rig);
        // The cycle's four draws were recorded under the process names (R16).
        assert_eq!(history.event_count(), 4);
        // Everything accounted: the assembly to finished goods (shipped,
        // F-039), the swarf to disposal, the note settled, the empties and
        // reusables kept to the end.
        let model_core::list::Cons(assembly, model_core::list::Nil) = assemblies;
        let fg = send_to(new_finished_goods::<N1>(), assembly);
        ship_finished_goods(fg);
        let (swarf, emptied) = bin.open();
        let _disposal = send_list(new_disposal(), swarf);
        fixture_settle_note(note);
        let _spent_clock: Operator<N0> = operator;
        let _empties_and_reusables = (rack, bolts, emptied, saw, press, bench, history);
    }
}
