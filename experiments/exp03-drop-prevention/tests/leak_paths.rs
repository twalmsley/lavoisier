//! Runtime demonstrations for the leak-path x mechanism matrix.
//!
//! - `#[should_panic(expected = "resource leak")]` tests are the evidence for
//!   "caught at TEST time" cells: the tripwire `Drop` fires.
//! - Tests that pass *silently* are the minimal demos for "NOT caught" cells:
//!   the leak completes and no mechanism notices.

#![deny(unused_must_use)]

use exp03_drop_prevention::{boundary, early_return_leak, inspect};

// ---------------------------------------------------------------------------
// Baseline: the mechanisms stay quiet when nothing leaks.
// ---------------------------------------------------------------------------

#[test]
fn well_behaved_flow_is_quiet() {
    let bolt = boundary::new_tracked_bolt();
    let bolt = inspect(bolt); // shadowing is fine when the old value MOVED
    bolt.consume();
}

// ---------------------------------------------------------------------------
// Leak path 1: silent drop at end of scope.
// ---------------------------------------------------------------------------

/// Tripwire: CAUGHT AT TEST TIME. The binding is used (so `unused_variables`
/// stays quiet) and then dropped at the end of scope.
#[test]
#[should_panic(expected = "resource leak")]
fn end_of_scope_drop_tripwire_catches() {
    let bolt = boundary::new_tracked_bolt();
    let _desc = format!("{bolt:?}"); // "use" the binding, then leak it
} // <- bolt dropped here; TrackedBolt::drop panics

/// NOT-CAUGHT demo: the same leak with the plain `Bolt` (no `Drop`).
/// `#[must_use]` does not fire on a binding going out of scope, and because
/// the binding IS used, `unused_variables` is silent too. The test passes,
/// i.e. the resource vanished and nothing noticed.
#[test]
fn end_of_scope_drop_of_used_binding_not_caught_without_tripwire() {
    let bolt = boundary::new_bolt();
    let _desc = format!("{bolt:?}");
} // <- bolt silently destroyed; compiles and passes under every lint we enable

// ---------------------------------------------------------------------------
// Leak path 2: `let _ = ...` (the underscore pattern never binds, drops now).
// ---------------------------------------------------------------------------

/// Tripwire: CAUGHT AT TEST TIME. Note this line COMPILES under
/// `#![deny(unused_must_use)]`: `let _ =` is precisely the idiom that
/// silences `must_use`. Compile-time coverage needs `let_underscore_drop`
/// (rustc) or `clippy::let_underscore_must_use` — see tests/compile_fail and
/// clippy-demos/.
#[test]
#[should_panic(expected = "resource leak")]
#[allow(clippy::let_underscore_must_use)] // deliberately demonstrating the leak
fn let_underscore_tripwire_catches() {
    let _ = boundary::new_tracked_bolt(); // dropped immediately
}

// ---------------------------------------------------------------------------
// Leak path 3: rebinding / shadowing.
// ---------------------------------------------------------------------------

/// Tripwire: CAUGHT AT TEST TIME. The first bolt becomes unreachable at the
/// shadowing `let` and is dropped at end of scope. (Shadowing does NOT drop
/// at the shadow point — the leak is deferred, which makes it easy to miss.)
///
/// Bonus compile-time finding: because the first binding is never used at
/// all, rustc's `unused_variables` DOES flag this exact case — we must
/// `allow` it to keep the demo. Shadowing after a genuine use (e.g. a
/// `format!` first) is flagged by nothing at compile time.
#[test]
#[should_panic(expected = "resource leak")]
#[allow(unused_variables)] // deliberately demonstrating the leak
fn shadowing_tripwire_catches() {
    let bolt = boundary::new_tracked_bolt();
    let bolt = boundary::new_tracked_bolt(); // first bolt now unreachable
    bolt.consume(); // only the second one is consumed
} // <- first bolt dropped here; tripwire fires

// ---------------------------------------------------------------------------
// Leak path 4: mem::forget.
// ---------------------------------------------------------------------------

/// NOT-CAUGHT demo (the tripwire's blind spot): `mem::forget` skips `Drop`
/// entirely, so the tripwire cannot fire — `forget` is exactly what
/// `consume()` uses to defuse it. `#[must_use]` is satisfied (the value IS
/// used). This test passing is the demo. Only `clippy::mem_forget` catches
/// this, at compile time — see clippy-demos/. `ManuallyDrop::new` and
/// `Box::leak` are equivalent uncaught-by-Drop escape hatches.
#[test]
#[allow(clippy::mem_forget)] // deliberately demonstrating the leak
fn mem_forget_defeats_tripwire_not_caught() {
    std::mem::forget(boundary::new_tracked_bolt());
    // no panic: the resource is gone and no runtime mechanism noticed
}

// ---------------------------------------------------------------------------
// Leak path 5: mem::drop (or the prelude `drop`).
// ---------------------------------------------------------------------------

/// Tripwire: CAUGHT AT TEST TIME. `drop(x)` is an ordinary use, so
/// `#[must_use]` is satisfied; but it runs `Drop`, so the tripwire fires.
#[test]
#[should_panic(expected = "resource leak")]
fn mem_drop_tripwire_catches() {
    let bolt = boundary::new_tracked_bolt();
    drop(bolt);
}

// ---------------------------------------------------------------------------
// Leak path 6: struct pattern with `..`.
// ---------------------------------------------------------------------------

/// Tripwire: CAUGHT AT TEST TIME. Destructuring with `..` drops the unbound
/// fields immediately. No compile-time mechanism fires: `#[must_use]` on the
/// field's type is ignored in patterns, and no rustc/clippy lint flags a `..`
/// that genuinely omits fields (`clippy::rest_pat_in_fully_bound_structs`
/// only fires when `..` hides nothing).
#[test]
#[should_panic(expected = "resource leak")]
fn struct_rest_pattern_tripwire_catches() {
    let assembly = boundary::new_assembly();
    let exp03_drop_prevention::Assembly { plate: _plate, .. } = assembly;
    // `bolt` (a TrackedBolt) was dropped by the `..`; tripwire fires here
}

// ---------------------------------------------------------------------------
// Leak path 7: early return.
// ---------------------------------------------------------------------------

/// Tripwire: CAUGHT AT TEST TIME — but only because a test exercises the
/// rejecting branch. Coverage of this leak is exactly test coverage of the
/// early-return path; no compile-time mechanism sees it at all.
#[test]
#[should_panic(expected = "resource leak")]
fn early_return_tripwire_catches_when_path_is_exercised() {
    let bolt = boundary::new_tracked_bolt();
    let _ = early_return_leak(bolt, true); // reject branch drops the bolt
}

/// The same function on the happy path: passes, proving the leak in the
/// other branch is invisible unless a test happens to take it.
#[test]
fn early_return_leak_invisible_on_untested_branch() {
    let bolt = boundary::new_tracked_bolt();
    match early_return_leak(bolt, false) {
        Some(b) => b.consume(),
        None => unreachable!(),
    }
}

// ---------------------------------------------------------------------------
// Leak path 8: panic unwinding.
// ---------------------------------------------------------------------------

/// NOT-CAUGHT demo for the panicking-aware tripwire: an unrelated panic
/// unwinds past a live TrackedBolt. The tripwire stands down (it checks
/// `thread::panicking()`), so the leak goes undetected — the price of not
/// aborting the process. The caught panic is the unrelated one.
#[test]
fn unwinding_guarded_tripwire_misses_leak() {
    let result = std::panic::catch_unwind(|| {
        let _bolt = boundary::new_tracked_bolt();
        panic!("some unrelated failure");
    });
    let msg = *result.unwrap_err().downcast::<&str>().unwrap();
    assert_eq!(msg, "some unrelated failure"); // not the tripwire's message
    // The bolt is gone; nothing flagged it.
}

/// The unguarded tripwire (`StrictBolt`) DOES fire during unwinding, but a
/// panic inside a destructor that runs during unwinding aborts the whole
/// process (SIGABRT). We demonstrate in a subprocess so the abort doesn't
/// kill this test binary.
#[test]
fn unwinding_vs_strict_tripwire_aborts_process() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_unwind_abort"))
        .output()
        .expect("failed to run unwind_abort binary");
    assert!(!output.status.success());
    // On unix the abort shows up as a signal, not an exit code.
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(output.status.signal(), Some(libc_sigabrt()));
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("panic in a destructor during cleanup"),
        "unexpected stderr: {stderr}"
    );
}

#[cfg(unix)]
fn libc_sigabrt() -> i32 {
    6 // SIGABRT on macOS and Linux
}
