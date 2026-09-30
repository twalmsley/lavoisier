//! Consuming into a full waste bag must not compile.

use exp02_supplier_consumer::*;

fn main() {
    let bx = full_box::<N2>();
    let bag = new_waste_bag::<N1>(); // space for one item only
    let (bolt, bx) = bx.supply();
    let bag = bag.consume(bolt); // fills the bag
    let (bolt, _bx) = bx.supply();
    let _bag = bag.consume(bolt); // no space left
}
