//! EXP-03: How close to "nothing is silently lost"?
//!
//! Rust's types are affine: a value may be used at most once, but it may also
//! be used *zero* times and silently dropped. This crate catalogues every way
//! a resource value can leak out of a model without being accounted for, and
//! applies three counter-mechanisms to a sample resource type:
//!
//! 1. `#[must_use]` on the type, with `#![deny(unused_must_use)]`.
//! 2. A `Drop` impl that panics unless the value was explicitly consumed
//!    (a runtime "tripwire", defused by `fn consume(self)` which
//!    `mem::forget`s internally).
//! 3. Clippy lints under `-D warnings` (see the sibling `clippy-demos/`
//!    crate, which is deliberately failing and therefore excluded from this
//!    workspace).
//!
//! The matrix of leak path x mechanism is in RESULTS.md; the runtime rows are
//! demonstrated in `tests/leak_paths.rs` and the compile-time rows in
//! `tests/compile_fail/`.

#![deny(unused_must_use)]

/// A plain resource with no `Drop` impl: what R1's resources look like by
/// default. `#[must_use]` is the only protection it carries.
#[must_use = "a Bolt is a conserved resource: pass it on or consume it explicitly"]
#[derive(Debug)]
pub struct Bolt {
    _private: (),
}

/// A resource protected by the panicking-`Drop` tripwire.
///
/// Dropping it in any way other than [`TrackedBolt::consume`] panics at run
/// time, so any test that exercises a leaking code path fails. The tripwire
/// stands down while the thread is already panicking (`thread::panicking()`),
/// otherwise a leak *during unwinding* would turn every ordinary test failure
/// into a process abort (double panic). The cost of standing down is that
/// leaks on unwind paths are NOT caught — see `StrictBolt` and the matrix.
#[must_use = "a TrackedBolt is a conserved resource: pass it on or consume it explicitly"]
#[derive(Debug)]
pub struct TrackedBolt {
    _private: (),
}

impl TrackedBolt {
    /// The one blessed way to dispose of a `TrackedBolt`: hand it to the
    /// system boundary (in the real model, a `Consumer`). Defuses the
    /// tripwire with `mem::forget`, which skips the `Drop` impl.
    // The whole point of `clippy::mem_forget` is that this is the ONLY
    // place in the crate allowed to call it.
    #[allow(clippy::mem_forget)]
    pub fn consume(self) {
        std::mem::forget(self);
    }
}

impl Drop for TrackedBolt {
    fn drop(&mut self) {
        // Without this guard, a TrackedBolt dropped while unwinding from an
        // unrelated panic causes a double panic -> SIGABRT, killing the whole
        // test binary. See StrictBolt for the unguarded variant.
        if std::thread::panicking() {
            return; // leak on unwind path goes UNDETECTED
        }
        panic!("resource leak: TrackedBolt was dropped without consume()");
    }
}

/// The unguarded tripwire variant: panics in `drop` even during unwinding.
/// A leak on an unwind path is then "caught", but as a double panic that
/// aborts the whole process (SIGABRT) and takes the test harness with it.
/// Demonstrated by `src/bin/unwind_abort.rs` + the subprocess test.
#[must_use]
#[derive(Debug)]
pub struct StrictBolt {
    _private: (),
}

impl StrictBolt {
    #[allow(clippy::mem_forget)]
    pub fn consume(self) {
        std::mem::forget(self);
    }
}

impl Drop for StrictBolt {
    fn drop(&mut self) {
        panic!("resource leak: StrictBolt was dropped without consume()");
    }
}

/// A composite resource, used to demonstrate the struct-pattern `..` leak:
/// destructuring `Assembly { plate, .. }` silently drops the bolt.
#[must_use]
#[derive(Debug)]
pub struct Assembly {
    pub plate: Bolt, // stands in for a Plate; the type doesn't matter here
    pub bolt: TrackedBolt,
}

/// The system boundary (R12 stand-in): the only code allowed to create
/// resources. In the real project this would be `pub(crate)` behind a
/// supplier; for this experiment it is public so tests and demo crates can
/// build fixtures.
pub mod boundary {
    use super::{Assembly, Bolt, StrictBolt, TrackedBolt};

    pub fn new_bolt() -> Bolt {
        Bolt { _private: () }
    }

    pub fn new_tracked_bolt() -> TrackedBolt {
        TrackedBolt { _private: () }
    }

    pub fn new_strict_bolt() -> StrictBolt {
        StrictBolt { _private: () }
    }

    pub fn new_assembly() -> Assembly {
        Assembly {
            plate: new_bolt(),
            bolt: new_tracked_bolt(),
        }
    }
}

/// A well-behaved process (R1): moves the resource in and returns it. Used to
/// show that the mechanisms stay quiet when nothing leaks.
pub fn inspect(bolt: TrackedBolt) -> TrackedBolt {
    bolt
}

/// LEAK PATH 7 (early return): on the `reject` branch the bolt is dropped by
/// the implicit cleanup of the early `return`, not by any visible statement.
/// Nothing in the source names the bolt on that path.
pub fn early_return_leak(bolt: TrackedBolt, reject: bool) -> Option<TrackedBolt> {
    if reject {
        return None; // `bolt` silently dropped here
    }
    Some(bolt)
}
