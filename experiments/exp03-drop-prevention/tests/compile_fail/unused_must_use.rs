//! Leak path 1 (variant: unused expression statement) x #[must_use].
//! CAUGHT AT COMPILE TIME: calling a resource-returning function and ignoring
//! the result is an error under #![deny(unused_must_use)].
#![deny(unused_must_use)]

use exp03_drop_prevention::boundary;

fn main() {
    boundary::new_tracked_bolt(); // result dropped on the spot
    boundary::new_bolt(); // same for the plain (no-Drop) resource
}
