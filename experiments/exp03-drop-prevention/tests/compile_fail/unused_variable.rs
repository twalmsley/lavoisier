//! Leak path 1 (variant: bound but never used) x rustc lint `unused_variables`.
//! CAUGHT AT COMPILE TIME — but only when the binding is never used at all.
//! One `format!("{bolt:?}")` and the lint goes quiet while the leak remains
//! (see tests/leak_paths.rs::end_of_scope_drop_of_used_binding_...).
#![deny(unused_variables)]

use exp03_drop_prevention::boundary;

fn main() {
    let bolt = boundary::new_bolt();
}
