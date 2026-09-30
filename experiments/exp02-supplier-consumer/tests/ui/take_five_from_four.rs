//! Asking a four-bolt box for five bolts through the recursive SupplyN
//! trait: the box runs out one short.

use exp02_supplier_consumer::*;

fn main() {
    let bx = full_box::<N4>();
    let (_taken, _rest) = supply_n::<N5, _>(bx);
}
