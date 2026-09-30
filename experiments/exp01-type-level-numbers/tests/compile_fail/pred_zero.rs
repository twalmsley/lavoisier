//! Decrementing zero must not compile (Peano): this is the "supplier is
//! exhausted" error shape from R12.
use exp01_type_level_numbers::peano::Pred;
use exp01_type_level_numbers::N0;

fn main() {
    let _decremented: <N0 as Pred>::Out = Default::default();
}
