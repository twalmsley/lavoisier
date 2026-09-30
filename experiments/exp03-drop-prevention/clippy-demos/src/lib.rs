//! One function per clippy-caught (or clippy-missed) matrix cell.
//! Run `cargo clippy -- -D warnings` in this directory and compare the errors
//! against the function list: the *absence* of an error for the last three
//! functions is the "not caught" evidence.
//!
//! The lints below are the candidate set for the real project. All are in
//! clippy's `restriction` group except `disallowed_methods` (`style`, driven
//! by clippy.toml).

#![warn(
    clippy::let_underscore_must_use,
    clippy::mem_forget,
    clippy::shadow_unrelated,
    clippy::shadow_reuse,
    clippy::shadow_same
)]

use exp03_drop_prevention::{Assembly, boundary, early_return_leak};

/// Leak path 2: CAUGHT by clippy::let_underscore_must_use (compile time).
/// (rustc's let_underscore_drop also catches it, but only for Drop types;
/// this clippy lint fires for every #[must_use] type, Drop or not.)
pub fn leak_by_let_underscore() {
    let _ = boundary::new_bolt();
}

/// Leak path 4: CAUGHT by clippy::mem_forget (compile time).
/// This is the tripwire's one blind spot, and clippy is the only mechanism
/// that sees it.
pub fn leak_by_mem_forget() {
    std::mem::forget(boundary::new_tracked_bolt());
}

/// Leak path 5: CAUGHT by clippy::disallowed_methods via clippy.toml
/// (compile time). There is no built-in "don't drop this type" lint, so the
/// project must disallow std::mem::drop/core::mem::drop wholesale.
pub fn leak_by_mem_drop() {
    let bolt = boundary::new_tracked_bolt();
    drop(bolt);
}

/// Leak path 3: CAUGHT by clippy::shadow_unrelated (compile time) — but the
/// shadow_* lints also fire on perfectly conservation-correct rebinding
/// (`let bolt = inspect(bolt);` trips shadow_reuse), so adopting them taxes
/// the R2 move-in/move-out style. See RESULTS.md.
pub fn leak_by_shadowing() {
    let bolt = boundary::new_tracked_bolt();
    let bolt = boundary::new_tracked_bolt();
    bolt.consume();
}

// ---------------------------------------------------------------------------
// NOT caught by any clippy lint (with everything above enabled):
// ---------------------------------------------------------------------------

/// Leak path 1: end-of-scope drop of a used binding. No lint output.
pub fn leak_at_end_of_scope_not_caught() {
    let bolt = boundary::new_bolt();
    let _desc = format!("{bolt:?}");
}

/// Leak path 6: struct pattern `..` dropping fields. No lint output.
/// (clippy::rest_pat_in_fully_bound_structs only fires when `..` hides
/// nothing at all.)
pub fn leak_by_rest_pattern_not_caught() {
    let Assembly { plate, .. } = boundary::new_assembly();
    // `bolt` field silently dropped (tripwire would fire at run time)
    let _desc = format!("{plate:?}");
}

/// Leak path 7: early return. No lint output.
pub fn leak_by_early_return_not_caught(reject: bool) {
    let bolt = boundary::new_tracked_bolt();
    if let Some(b) = early_return_leak(bolt, reject) {
        b.consume();
    }
}

/// Leak path 4 variant: mem::forget of a NO-Drop resource (plain Bolt).
/// `clippy::mem_forget` (restriction) only fires for types with drop glue,
/// but `clippy::forget_non_drop` (warn BY DEFAULT) fires here instead — so
/// under `-D warnings` both variants of forget are caught at compile time.
pub fn leak_by_mem_forget_no_drop_type() {
    std::mem::forget(boundary::new_bolt());
}
