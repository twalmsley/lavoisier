// R12: `Supplier` is implemented only for a non-empty box, so supplying from
// an exhausted supplier is a type-check-time E0277 carrying the
// modeller-phrased on_unimplemented message (F-015): the box is empty.
use model_core::boundary::take_one;
use model_core::fixtures::full_box;
use model_core::nat::aliases::N1;

fn main() {
    let bx = full_box::<N1>();
    let (_bolt, empty) = take_one(bx);
    let (_bolt2, _rest) = take_one(empty);
}
