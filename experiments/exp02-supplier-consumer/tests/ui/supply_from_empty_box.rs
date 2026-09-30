//! Supplying from an empty box: direct method call.
//! A modeller should be able to read "the box is empty" out of this error.

use exp02_supplier_consumer::*;

fn main() {
    let empty: EmptyBoltBox = full_box::<N0>();
    let (_bolt, _rest) = empty.supply();
}
