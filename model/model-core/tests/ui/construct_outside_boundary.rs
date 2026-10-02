// R1: resources have private constructors — "no creating resources from
// nothing" is a compile-time check. Outside the defining crate the only ways
// in are the boundary functions (R12) and the test fixtures (F-004); a
// literal construction is a privacy error (E0451) that reads as "you may not
// create this resource" (F-005).
use model_core::common::Person;

fn main() {
    let _conjured = Person::<1000> { _seal: () };
}
