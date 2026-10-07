// The crate edge (EXP-08/F-005/F-006): the kernel's generated constructors
// are pub(crate) to the defining crate, so the works calling cs5-supply's
// `mint` is E0624 "associated function `mint` is private" — euro cents
// cannot be sourced on this side of the edge. The only production mint site
// for `Euros` is the bureau's `exchange` (REQ-024), which demands real GBP
// in return.
use cs5_supply::resources::Euros;

fn main() {
    // The vendor's price from nothing — minting currency downstream.
    let euro_cents = Euros::<2340>::mint();
}
