// R3/R4: THE required compile-fail — splitting 2000 g into 1500 g + 600 g.
// Via the call-site macro the violation is a check-time error, so trybuild
// (which runs `cargo check`) sees it. The raw generic `split` version of this
// same violation is in ../ui-postmono-not-caught-by-trybuild/.
use exp04_quantity_conservation::const_style::boundary::supply_grams;
use exp04_quantity_conservation::split_grams;

fn main() {
    let stock = supply_grams::<2000>();
    let (part, offcut) = split_grams!(stock, 2000 => 1500 + 600);
    let _ = (part, offcut);
}
