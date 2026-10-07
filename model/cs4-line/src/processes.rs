//! The line's processes P2–P4 (R1, R2; SPEC §5): pure by-value
//! transformations composing the stores material transforms with the line's
//! tools and waste routing.
//!
//! These are **discrete** processes — they only move sealed values — so they
//! live outside the resource module tree (F-006): the minting happens across
//! the crate edge, inside `cs4_stores::resources::processes`' conserving
//! transforms, whose R3 const asserts fire at every instantiation here
//! (F-001: on `cargo build`/`cargo test`, never `cargo check`).
//!
//! Waste routing per SPEC §5: cut and drill feed their swarf to the line's
//! bin **inside the process** — swarf never exists loose in a flow. The
//! operator's time draws are **adjacent** processes in the flow (F-048),
//! made by the batch recursion and recorded under these process names
//! (R16); the signatures here stay material-and-tools only.

use crate::resources::{PillarDrill, Saw, Workbench};
// One line on purpose: the traceability grep (F-021) skips `use` lines, but
// only when the line itself starts with `use`.
use cs4_stores::requirements::{Req019StoresIssuedMaterial, Req021CurrentIssueNote};
use cs4_stores::characteristics::{Len15, M8Thread, SteelMade};
use cs4_stores::resources::processes::{bore_blank, join_assembly, shear_sheet};
use cs4_stores::resources::{ASSEMBLY_G, Assembly, BLANK_G, Blank, CUT_SWARF_G, CutSwarf, DRILL_SWARF_G, DrillSwarf, DrilledPlate, FourOf, PLATE_G, SHEET_G, Sheet};
use model_core::boundary::{Consumer, SupplyN, send_to, take_n};
use model_core::nat::aliases::N4;

/// P2 — cut a sheet (SPEC §5): one 5000 g sheet becomes two 2250 g blanks,
/// with the 500 g of cut swarf fed to the line's bin **inside the process**
/// (REQ-020's collection). The saw is moved in and returned (R2). Mass
/// conservation is the const assert inside the stores transform
/// (`shear_sheet`, R3), instantiated here with the batch's stated split —
/// SPEC §7's probe B (`SHEET_G` 5000 → 4800) breaks exactly this
/// instantiation, on `cargo build` only (F-001).
pub fn cut<BinC: Consumer<CutSwarf<CUT_SWARF_G>>>(saw: Saw, sheet: Sheet<SHEET_G>, bin: BinC) -> (Saw, Blank<BLANK_G>, Blank<BLANK_G>, BinC::Next) {
    let (blank_a, blank_b, swarf) = shear_sheet::<SHEET_G, BLANK_G, BLANK_G, CUT_SWARF_G>(sheet);
    // Waste routing (SPEC §5): the swarf goes straight to the bin, through
    // the generic access process so capacity mistakes take the
    // modeller-phrased trait-bound path (F-015).
    let bin = send_to(bin, swarf);
    (saw, blank_a, blank_b, bin)
}

/// P3 — drill a blank (SPEC §5): one 2250 g blank becomes one 2240 g drilled
/// plate, with the 10 g of drill swarf fed to the bin inside the process.
/// The pillar drill is moved in and returned (R2). The state change is the
/// stores transform `bore_blank` (R9: one type per processing state — the
/// output is a [`DrilledPlate`], which is what P4 requires).
pub fn drill_blank<BinC: Consumer<DrillSwarf<DRILL_SWARF_G>>>(press: PillarDrill, blank: Blank<BLANK_G>, bin: BinC) -> (PillarDrill, DrilledPlate<PLATE_G>, BinC::Next) {
    let (plate, swarf) = bore_blank::<BLANK_G, PLATE_G, DRILL_SWARF_G>(blank);
    let bin = send_to(bin, swarf);
    (press, plate, bin)
}

/// P4 — fasten an assembly (SPEC §5): two drilled plates and four bolts
/// taken from ONE supplier in a single `SupplyN<N4>` bound (R12, F-014)
/// become one 4600 g assembly (2 × 2240 + 4 × 30, const-asserted inside the
/// stores transform `join_assembly`). Only REQ-019 stores-issued bolts of
/// the catalogued spec are accepted — the requirement is the trait bound on
/// `B`, never a concrete type (R10, F-019) — and the line may fasten only
/// against the current stores issue note, threaded through and returned
/// (REQ-021): a note-less fasten is a REQ-phrased compile error
/// (`tests/ui/fasten_without_the_note.rs`). The workbench is moved in and
/// returned (R2).
///
/// The whole signature sits on one line because trace.sh attributes
/// requirement bounds to the line they are written on (R10 rule 4, F-021).
///
/// Satisfies: REQ-019, REQ-021
pub fn fasten<B: Req019StoresIssuedMaterial + M8Thread + SteelMade + Len15, S: SupplyN<N4, Taken = FourOf<B>>, PN: Req021CurrentIssueNote>(bench: Workbench, a: DrilledPlate<PLATE_G>, b: DrilledPlate<PLATE_G>, bolts: S, note: PN) -> (Workbench, Assembly<B, ASSEMBLY_G>, S::Rest, PN) {
    let (taken, rest) = take_n::<N4, _>(bolts);
    let assembly = join_assembly::<B, PLATE_G, PLATE_G, ASSEMBLY_G>(a, b, taken);
    (bench, assembly, rest, note)
}

#[cfg(test)]
mod tests {
    //! Per-process unit tests (R5), downstream-style: materials are
    //! `test-support` fixtures (F-004), and everything tripwired leaves
    //! through a real consumer or the fixtures' settlement helpers.

    use super::{cut, drill_blank, fasten};
    use crate::resources::boundary::{new_swarf_bin, tool_up};
    use cs4_stores::resources::boundary::{new_disposal, new_finished_goods, ship_finished_goods};
    use cs4_stores::resources::test_support::{fixture_box, fixture_settle_note};
    use cs4_stores::resources::{Assembly, Blank, DrilledPlate, EmptyBoltBox, IssueNote, IssuedBolt, Sheet, SwarfReturn};
    use model_core::boundary::{send_list, send_to};
    use model_core::nat::aliases::{N1, N3, N4};

    /// P2 cuts one sheet into two blanks with the cut swarf fed to the bin
    /// inside the process, P3 drills both blanks likewise, and P4 fastens
    /// the resulting plates — one full cycle's material path, everything
    /// accounted at the end (R1, R5; SPEC §5).
    ///
    /// Verifies: REQ-020, REQ-022
    #[test]
    fn one_cycle_of_processes_accounts_for_everything() {
        let (saw, press, bench) = tool_up();
        let bin = new_swarf_bin::<N3>();
        let (saw, blank_a, blank_b, bin) = cut(saw, Sheet::<5000>::test_fixture(), bin);
        assert_eq!(Blank::<2250>::VALUE + Blank::<2250>::VALUE, 4500);
        let (press, plate_a, bin) = drill_blank(press, blank_a, bin);
        let (press, plate_b, bin) = drill_blank(press, blank_b, bin);
        let (bench, assembly, empty_box, note): (_, Assembly<IssuedBolt, 4600>, EmptyBoltBox, IssueNote) = fasten(bench, plate_a, plate_b, fixture_box::<N4>(), IssueNote::test_fixture());
        // The assembly leaves to finished goods (REQ-022), which is then
        // shipped so its kept contents are accounted (F-039).
        let fg = send_to(new_finished_goods::<N1>(), assembly);
        ship_finished_goods(fg);
        // The cycle's three swarf pieces leave through the real disposal
        // path (REQ-020), and the bin comes back empty.
        let (swarf, emptied) = bin.open();
        let _disposal = send_list(new_disposal(), swarf);
        let _bin_back_with_the_line = emptied;
        // The note would be reconciled by P6 in the real batch; the fixture
        // settlement stands in for it in this per-process test (F-004).
        fixture_settle_note(note);
        let _returned_to_stores = empty_box;
        let _reusables = (saw, press, bench);
    }

    /// P4 takes exactly four bolts from one supplier in one `SupplyN<N4>`
    /// bound (F-014): the four-bolt box fixture comes back empty — a
    /// distinct resource (R12) — and the issue note is threaded through
    /// unchanged (REQ-021).
    ///
    /// Verifies: REQ-019, REQ-021
    #[test]
    fn fasten_takes_four_bolts_and_returns_the_note() {
        let (saw, press, bench) = tool_up();
        let a = DrilledPlate::<2240>::test_fixture();
        let b = DrilledPlate::<2240>::test_fixture();
        let (bench, assembly, rest, note): (_, Assembly<IssuedBolt, 4600>, EmptyBoltBox, IssueNote) = fasten(bench, a, b, fixture_box::<N4>(), IssueNote::test_fixture());
        assert_eq!(Assembly::<IssuedBolt, 4600>::GRAMS, 4600);
        let _empty_box: EmptyBoltBox = rest;
        let fg = send_to(new_finished_goods::<N1>(), assembly);
        ship_finished_goods(fg);
        fixture_settle_note(note);
        let _reusables = (saw, press, bench);
    }
}
