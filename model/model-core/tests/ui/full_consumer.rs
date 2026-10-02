// R12: `Consumer` is implemented only while space remains, so consuming into
// a full consumer is a type-check-time E0277 carrying the modeller-phrased
// on_unimplemented message (F-015): the bag is full.
use model_core::boundary::{send_to, take_one};
use model_core::fixtures::{full_box, new_waste_bag};
use model_core::nat::aliases::{N1, N2};

fn main() {
    let bx = full_box::<N2>();
    let (bolt1, bx) = take_one(bx);
    let (bolt2, _empty) = take_one(bx);
    let bag = new_waste_bag::<N1>();
    let bag = send_to(bag, bolt1); // the bag is now full...
    let _bag = send_to(bag, bolt2); // ...so this cannot compile
}
