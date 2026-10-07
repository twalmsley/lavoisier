// REQ-021: the line may operate only against a current stores issue note.
// Fastening with the works order in the note position fails with REQ-021's
// own on_unimplemented message (R10 rule 8, F-044) — only the sealed
// IssueNote carries ProofOfIssue, and nothing downstream can forge one
// (F-026; the mint attempt is the line_cannot_mint_materials case).
use cs4_line::processes::fasten;
use cs4_line::resources::boundary::tool_up;
use cs4_stores::resources::boundary::place_works_order;
use cs4_stores::resources::test_support::fixture_box;
use cs4_stores::resources::DrilledPlate;
use model_core::nat::aliases::N4;

fn main() {
    let (saw, press, bench) = tool_up();
    let a = DrilledPlate::<2240>::test_fixture();
    let b = DrilledPlate::<2240>::test_fixture();
    // The works order is stores paperwork, but it is NOT the issue note.
    let not_a_note = place_works_order();
    let r = fasten(bench, a, b, fixture_box::<N4>(), not_a_note);
}
