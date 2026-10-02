//! Stating the wrong price in a purchase is a trait-bound error (R19 build
//! item 3): `purchase` requires `V: Consumer<Money<PRICE>>`, and the vendor
//! implements `Consumer` only at its exact price (350 pence). Asking it to
//! accept 300 fails the bound at type-check time with the modeller-phrased
//! on_unimplemented message (F-015), and rustc's help lists the one impl
//! that exists — naming the right price in the error.

use exp12_money::goods::boundary::new_vendor;
use exp12_money::money::Money;
use exp12_money::money::processes::purchase;

fn main() {
    let tendered: Money<500> = Money::test_fixture();
    // The spanner costs 350, not 300.
    let (spanner, change, vendor) = purchase::<_, 500, 300, 200>(new_vendor(), tendered);
    let _ = (spanner, change, vendor);
}
