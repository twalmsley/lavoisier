// R7: units are types, so mixing them is a crisp type-check-time E0308 —
// grams cannot be combined with millimetres.
use model_core::quantity::boundary::supply;
use model_core::quantity::{Grams, Millimetres, combine};

fn main() {
    let mass = supply::<100, Grams>();
    let length = supply::<50, Millimetres>();
    let _total = combine::<Grams, 100, 50, 150>(mass, length);
}
