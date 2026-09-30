//! Leak path 2 (`let _ = ...`) x rustc lint `let_underscore_drop`.
//! CAUGHT AT COMPILE TIME — but only for types with drop glue (here, the
//! tripwire type). `let _ = boundary::new_bolt();` would NOT trigger it,
//! because the plain Bolt has no Drop impl.
#![deny(let_underscore_drop)]

use exp03_drop_prevention::boundary;

fn main() {
    let _ = boundary::new_tracked_bolt();
}
