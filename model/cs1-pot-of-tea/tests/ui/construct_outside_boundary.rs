// R1: resources have private constructors — "no creating resources from
// nothing" is a compile-time check. This crate sits outside cs1-pot-of-tea's
// privacy boundary, so the only ways in are the boundary suppliers and draw
// processes (R12) and the test fixtures (F-004); a literal construction is a
// privacy error (E0451) that reads as "you may not create this resource"
// (F-005).
use cs1_pot_of_tea::resources::ColdWater;

fn main() {
    let _conjured = ColdWater::<1500> { _seal: () };
}
