// REQ-023's structural half (R19/F-051): the vendor's `Consumer` impl exists
// only at `Euros<2340>`, and pence and euro cents are separate dimensions —
// so paying the vendor in GBP is a type-check-time E0308 ("expected
// `Euros<2340>`, found `Money<2340>`"), exactly like handing grams where
// millimetres are required. The only way to euro cents is the bureau
// exchange (REQ-024).
use cs5_supply::resources::boundary::{place_purchase_order, vendor_opens_for_business};
use cs5_supply::resources::processes::sell_component;
use cs5_supply::resources::Money;

fn main() {
    let vendor = vendor_opens_for_business();
    let order = place_purchase_order();
    // The right amount in the wrong currency: 2340 PENCE.
    let pence = Money::<2340>::test_fixture();
    let (boxed, vendor) = sell_component(vendor, order, pence);
}
