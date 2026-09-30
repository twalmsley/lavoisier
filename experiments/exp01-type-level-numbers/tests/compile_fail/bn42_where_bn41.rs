//! The same mismatch in the binary encoding: BN42 where BN41 is required.
use exp01_type_level_numbers::{BN41, BN42};

fn requires_bn41(_count: BN41) {}

fn main() {
    let wrong: BN42 = BN42::default();
    requires_bn41(wrong);
}
