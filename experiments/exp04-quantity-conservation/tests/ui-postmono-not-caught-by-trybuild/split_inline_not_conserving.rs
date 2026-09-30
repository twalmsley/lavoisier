// R3/R4: the same violation through the inline-const technique.
use exp04_quantity_conservation::const_style::{boundary::supply_grams, split_inline, Grams};

fn main() {
    let stock = supply_grams::<2000>();
    let (part, offcut): (Grams<1500>, Grams<600>) = split_inline(stock);
    let _ = (part, offcut);
}
