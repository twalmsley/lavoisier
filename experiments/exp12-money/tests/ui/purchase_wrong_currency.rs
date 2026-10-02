//! The purchase process itself is currency-typed (R19 build item 3): its
//! tendered parameter is `Money<TENDERED>` (pence), so tendering euros is a
//! plain E0308 at the call site — the two sealed cash types never mix, just
//! like the `Qty` unit types.

use exp12_money::goods::boundary::new_vendor;
use exp12_money::money::Euros;
use exp12_money::money::processes::purchase;

fn main() {
    let euros: Euros<500> = Euros::test_fixture();
    let (spanner, change, vendor) = purchase::<_, 500, 350, 150>(new_vendor(), euros);
    let _ = (spanner, change, vendor);
}
