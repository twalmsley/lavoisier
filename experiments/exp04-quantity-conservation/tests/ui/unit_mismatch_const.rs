// R7: unit safety, const style — Grams and Millimetres must not combine.
use exp04_quantity_conservation::const_style::{boundary, combine, Grams};

fn main() {
    let mass = boundary::supply_grams::<1500>();
    let length = boundary::supply_millimetres::<500>();
    let total: Grams<2000> = combine(mass, length);
    let _ = total;
}
