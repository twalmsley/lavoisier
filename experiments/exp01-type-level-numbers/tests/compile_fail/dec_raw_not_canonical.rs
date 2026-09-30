//! FINDING demo: raw binary decrement of 1 yields `UInt<UTerm, B0>` —
//! numerically zero, but a different *type* from `BN0` (= `UTerm`).
//! Without the `Trim` normalisation pass, capacity arithmetic silently
//! stops matching the canonical aliases. The Peano encoding has no such
//! canonical-form trap.
use exp01_type_level_numbers::binary::DecRaw;
use exp01_type_level_numbers::{same, BN0, BN1};

fn main() {
    same::<<BN1 as DecRaw>::Out, BN0>();
}
