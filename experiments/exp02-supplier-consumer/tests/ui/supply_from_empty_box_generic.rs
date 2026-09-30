//! Supplying from an empty box: passing it to a generic process.
//! This path goes through the trait bound, so the
//! `#[diagnostic::on_unimplemented]` message appears.

use exp02_supplier_consumer::*;

fn use_one_bolt<S: Supplier<Item = Bolt>>(s: S) -> S::Next {
    let (_bolt, rest) = s.supply();
    rest
}

fn main() {
    let empty: EmptyBoltBox = full_box::<N0>();
    let _ = use_one_bolt(empty);
}
