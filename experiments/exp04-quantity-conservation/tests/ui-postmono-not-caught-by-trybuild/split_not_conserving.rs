// R3/R4: splitting 2000 g into 1500 g + 600 g must not compile (100 g created
// from nothing). Uses the associated-const assert technique.
use exp04_quantity_conservation::const_style::{boundary::supply_grams, split, Grams};

fn main() {
    let stock = supply_grams::<2000>();
    let (part, offcut): (Grams<1500>, Grams<600>) = split(stock);
    let _ = (part, offcut);
}
