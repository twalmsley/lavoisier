// R12/F-014: fasten takes four bolts from ONE supplier in a single
// `SupplyN<N4>` bound. A box holding only three bolts would run out partway,
// so the bound is unsatisfied — E0277 with the modeller-phrased SupplyN /
// Supplier messages (F-015; the deepest failing obligation's message leads,
// per F-015's extension).
use cs4_line::processes::fasten;
use cs4_line::resources::boundary::tool_up;
use cs4_stores::resources::test_support::fixture_box;
use cs4_stores::resources::{DrilledPlate, IssueNote};
use model_core::nat::aliases::N3;

fn main() {
    let (saw, press, bench) = tool_up();
    let a = DrilledPlate::<2240>::test_fixture();
    let b = DrilledPlate::<2240>::test_fixture();
    let note = IssueNote::test_fixture();
    // Three bolts cannot fasten a four-bolt assembly.
    let r = fasten(bench, a, b, fixture_box::<N3>(), note);
}
