// R19/F-051 (REQ-014): paying the wrong amount is a type-check-time error
// that names the right price. The counter implements `Consumer` only at its
// exact listed price (`Money<650>`), so handing it 600 pence fails inference
// against that single impl: "expected `Money<650>`, found `Money<600>`"
// (E0308) — visible to editors and trybuild, unlike the
// post-monomorphization conservation asserts.
use cs2_puncture_repair::resources::boundary::{new_parts_counter, wallet_with};
use cs2_puncture_repair::resources::processes::draw_cash;
use model_core::boundary::send_to;

fn main() {
    let wallet = wallet_with::<600>();
    let (underpayment, wallet) = draw_cash::<600, 0, 600>(wallet);
    let counter = send_to(new_parts_counter(), underpayment);
    let _accounted = (counter, wallet);
}
