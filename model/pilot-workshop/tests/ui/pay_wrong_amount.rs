// R19/F-051: paying the wrong amount is a type-check-time error that names
// the right price. The vendor implements `Consumer` only at its exact price
// (`Money<400>`), so handing it 300 pence fails inference against that single
// impl: "expected `Money<400>`, found `Money<300>`" (E0308) — visible to
// editors and trybuild, unlike the post-monomorphization conservation
// asserts.
use model_core::boundary::send_to;
use pilot_workshop::money::boundary::{new_vendor, open_account};
use pilot_workshop::money::processes::draw_funds;

fn main() {
    let account = open_account::<500>();
    let (underpayment, account) = draw_funds::<300, 200, 500>(account);
    let vendor = send_to(new_vendor(), underpayment);
    let _accounted = (vendor, account);
}
