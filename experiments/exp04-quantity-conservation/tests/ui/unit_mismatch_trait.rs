// R7: unit safety, trait style — a Grams quantity and a Millimetres quantity
// must not combine, even though both are Qty<_, _> with matching values.
use exp04_quantity_conservation::trait_style::{boundary::supply, combine, Grams, Millimetres, Qty};

fn main() {
    let mass = supply::<1500, Grams>();
    let length = supply::<500, Millimetres>();
    let total: Qty<2000, Grams> = combine(mass, length);
    let _ = total;
}
