// The refined placeholder at work (R12, SPEC §4): the vendor's stock is a
// finite type-level 3-list, so a FOURTH purchase has no `Supplier` impl to
// satisfy — E0277 with the modeller-phrased F-015 message ("`Vendor<Nil>`
// cannot supply anything: it is exhausted…"). Contrast the old unbounded
// `Next = Self` placeholder vendors, which this call would never have
// refused.
use cs5_supply::resources::boundary::{place_purchase_order, vendor_opens_for_business};
use cs5_supply::resources::processes::sell_component;
use cs5_supply::resources::Euros;

fn main() {
    let vendor = vendor_opens_for_business(); // stock: 3
    let (_b1, vendor) = sell_component(vendor, place_purchase_order(), Euros::test_fixture());
    let (_b2, vendor) = sell_component(vendor, place_purchase_order(), Euros::test_fixture());
    let (_b3, vendor) = sell_component(vendor, place_purchase_order(), Euros::test_fixture());
    // The vendor is now Vendor<Nil>: exhausted, a distinct accounted state.
    let (_b4, vendor) = sell_component(vendor, place_purchase_order(), Euros::test_fixture());
}
