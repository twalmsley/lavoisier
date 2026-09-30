//! Consuming into a full waste bag: through a generic process bound, so the
//! `#[diagnostic::on_unimplemented]` message ("it is full") appears.

use exp02_supplier_consumer::*;

fn discard<C: Consumer<Bolt>>(c: C, bolt: Bolt) -> C::Next {
    c.consume(bolt)
}

fn main() {
    let bx = full_box::<N2>();
    let bag = new_waste_bag::<N1>(); // space for one item only
    let (bolt, bx) = bx.supply();
    let bag = discard(bag, bolt); // fills the bag
    let (bolt, _bx) = bx.supply();
    let _bag = discard(bag, bolt); // no space left
}
