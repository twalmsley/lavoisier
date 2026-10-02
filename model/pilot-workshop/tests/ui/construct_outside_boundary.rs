// R1: resources have private constructors — "no creating resources from
// nothing" is a compile-time check. This crate sits outside pilot-workshop's
// privacy boundary, so the only ways in are the boundary suppliers (R12) and
// the test fixtures (F-004); a literal construction is a privacy error
// (E0451) that reads as "you may not create this resource" (F-005).
use pilot_workshop::resources::Swarf;

fn main() {
    let _conjured = Swarf::<25> { _seal: () };
}
