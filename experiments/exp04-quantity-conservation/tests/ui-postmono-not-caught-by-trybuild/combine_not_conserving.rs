// R3/R4: trait style — combining 1500 g + 500 g into 2100 g must not compile.
use exp04_quantity_conservation::trait_style::{boundary::supply, combine, Grams, Qty};

fn main() {
    let a = supply::<1500, Grams>();
    let b = supply::<500, Grams>();
    let total: Qty<2100, Grams> = combine(a, b);
    let _ = total;
}
