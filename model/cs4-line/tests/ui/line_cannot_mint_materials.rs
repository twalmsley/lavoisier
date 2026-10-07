// REQ-019 is the crate edge (SPEC §3, EXP-08/F-005/F-006): the line may
// create NOTHING material. The struct-literal mint route fails on field
// privacy — E0451, which the error-reading guide translates as "you may not
// create this resource". (The constructor route, `Blank::mint()`, is the
// sibling case line_cannot_call_mint.rs: rustc reports only one of the two
// when both appear in one crate, aborting after the resolution-phase E0624.)
// The error is privacy vocabulary rather than REQ-phrased; REQ-019's own
// on_unimplemented phrasing appears on the bound path (fasten) instead.
use cs4_stores::resources::IssueNote;

fn main() {
    // A home-made "issue note" — forging REQ-021's evidence from the wrong
    // side of the crate edge.
    let note = IssueNote { _seal: () };
}
