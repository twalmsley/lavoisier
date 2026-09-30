//! LEAK PATH 8 (panic unwinding) vs the UNGUARDED tripwire (`StrictBolt`).
//!
//! An unrelated panic unwinds while a StrictBolt is live in scope. Unwinding
//! drops the bolt, its `Drop` panics too -> double panic -> the process
//! ABORTS (SIGABRT). The leak is "caught", but by killing the process, which
//! inside `cargo test` would take the whole test binary down. The integration
//! test `unwinding_vs_strict_tripwire_aborts_process` runs this binary as a
//! subprocess and asserts it died abnormally.

use exp03_drop_prevention::boundary;

// Note: rustc's `unused_variables` flags `bolt` (a use after the panic! is
// unreachable and doesn't count), so even this demo needed an `allow` —
// evidence that the lint catches some never-really-used bindings.
#[allow(unused_variables)]
fn main() {
    let bolt = boundary::new_strict_bolt();
    // Unrelated failure elsewhere in the process:
    panic!("some unrelated failure");
    // unreachable: unwinding drops `bolt`, StrictBolt::drop panics -> abort
}
