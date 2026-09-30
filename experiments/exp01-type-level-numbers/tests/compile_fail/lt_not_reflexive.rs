//! Less-than is strict: N42 < N42 must not hold.
use exp01_type_level_numbers::peano::{Lt, Nat};
use exp01_type_level_numbers::N42;

fn requires_lt<A: Lt<B>, B: Nat>() {}

fn main() {
    requires_lt::<N42, N42>();
}
