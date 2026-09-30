//! The headline error required by the experiment brief: supplying N42
//! where N41 is required, Peano encoding.
use exp01_type_level_numbers::{N41, N42};

/// A process step that needs a supplier with exactly 41 items left.
fn requires_n41(_count: N41) {}

fn main() {
    let wrong: N42 = N42::default();
    requires_n41(wrong);
}
