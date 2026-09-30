//! Decrementing zero must not compile (binary encoding).
use exp01_type_level_numbers::binary::Dec;
use exp01_type_level_numbers::BN0;

fn main() {
    let _decremented: <BN0 as Dec>::Out = Default::default();
}
