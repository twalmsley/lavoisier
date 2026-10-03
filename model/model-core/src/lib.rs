//! # model-core — the type-system MBSE kernel
//!
//! The reusable library (R8, R11 of `instructions.md`) behind the project's
//! model-based systems engineering approach: **types stand for things,
//! functions stand for processes, and compiler errors show where the model
//! does not fit.** Every design decision here was validated by the
//! experiments EXP-01..EXP-12; the known limitations are recorded in
//! `FINDINGS.md` (F-001 onwards) and cited throughout.
//!
//! This crate is **infrastructure**: it defines no `REQ-NNN` requirements of
//! a modelled system itself (R10 requirement traits belong to the downstream
//! modelling crates), so the workspace `trace.sh` report notes "no
//! requirements defined" for it.
//!
//! ## Where to start
//!
//! * **The modeller's guide** — `docs/modellers-guide.md` at the repository
//!   root — is the practical how-to: setting up a model crate, the
//!   walkthrough from specification to passing CI gate, the full
//!   error-reading guide with worked examples, the must-not-break checklist,
//!   and the known limitations. Read it before modelling anything; the
//!   normative requirements it companions are `instructions.md` (R1–R19).
//! * **Worked downstream models**: `pilot-workshop` (catalogue, fallible
//!   processes, qualifications and safety, money) and `cs1-pot-of-tea` (the
//!   first spec→model round trip: continuous resources, the boundary, the
//!   execution history, the sealed bin-disposal path), both in this
//!   workspace. New models are written as further downstream crates from a
//!   specification following `SPEC_TEMPLATE.md`.
//!
//! ## Module map
//!
//! | Module | Contents |
//! |---|---|
//! | [`nat`] | Peano type-level naturals (`Zero`/`Succ`), `Nat::VALUE`, `Pred`, `Add`, `Lt`; generated aliases `N0`..`N1000` (R12) |
//! | [`list`] | Type-level lists of real values (`Cons`/`Nil`) with their length as a `Nat` and a `const` (R12, R13) |
//! | [`quantity`] | The `Unit` kind trait, the eight R7 base units, sealed `Qty<V, U>`, and conserving `split`/`combine` (R3, R7, R8) |
//! | [`resource`] | The sealing/tripwire kernel: macros defining sealed, conserved resource types, draw processes, and outcome tokens for fallible processes (R1, R15, R17) |
//! | [`boundary`] | `Supplier` (discrete-only), `Consumer`, recursive `SupplyN`/`ConsumeList`, and the generic access processes (R12) |
//! | [`common`] | Reusable common types: `Person` (with an R15 time budget), the `Qualified` person wrapper (R18), `Organisation`, `Location`, `Labour` (R11) |
//! | [`history`] | Execution history (R16): the sealed `History` boundary consumer, value-level `Event`/`Entry` records, per-branch creation and partial-order `merge` |
//! | [`requirement`] | The R10 requirement-trait pattern as macros, compatible with `trace.sh` (R10) |
//! | `fixtures` | Test fixtures and reference boundary implementations — only with the `test-support` feature (F-004) |
//!
//! ## The conservation regime (R1) — obligations on every modelling crate
//!
//! No stable mechanism makes "nothing is silently lost" a compile-time
//! guarantee (F-002); the project layers conventions instead, and **the set
//! is only sound as a whole** (F-007). Every crate in the model workspace
//! must carry:
//!
//! 1. `#![recursion_limit = "2048"]` — type-level capacities need ≈ N + 3
//!    (F-010); the attribute is per crate, not per library.
//! 2. `#![forbid(unsafe_code)]` — privacy is a safe-Rust guarantee only
//!    (F-005); `unsafe` can mint a sealed resource.
//! 3. `#![deny(unused_must_use)]` and `#![deny(let_underscore_drop)]` —
//!    layer 1 of the regime, backing the `#[must_use]` on every resource.
//! 4. CI clippy under `-D warnings` plus the restriction lints
//!    `clippy::mem_forget` and `clippy::let_underscore_must_use`, and the
//!    workspace `clippy.toml` disallowed-methods list (layer 3).
//! 5. A tripwire `Drop` on every **consumable** resource (layer 2) — the
//!    [`resource`] macros generate it. It converts silent test-time leaks
//!    into panics; it reports the `Drop` impl's line, not the leak site, so
//!    keep processes small and tests per-process (R5, F-008).
//! 6. **Never** write `let _ = <resource>` — and ignore compiler fix-its
//!    that suggest it (see the error-reading guide below).
//!
//! Residual gaps, accepted (F-002): used-then-dropped values on untested
//! branches (branch coverage *is* leak coverage, R5), losses during panic
//! unwinding, and forget-equivalents beyond the banned list (review).
//!
//! ## Modeller's error-reading guide (R4, R9)
//!
//! Recurring translations from compiler language to model language. (The
//! expanded version — each entry with a worked example from the pinned
//! compile-fail cases — is §4 of `docs/modellers-guide.md`.)
//!
//! * **"use of moved value: `x`"** — the resource is already in use by
//!   another process, or was lost upstream; get it back from that process's
//!   output (R2, R9, F-025).
//! * **"the trait `Supplier` is not implemented for `BoltBox<Nil>`"** (and
//!   other `on_unimplemented` messages) — the supplier is exhausted / the
//!   consumer is full (R12, F-015).
//! * **E0275 "overflow evaluating the requirement"** — the capacity exceeds
//!   the crate's `recursion_limit`: the box is too big for the current
//!   limit; raise the attribute (F-010).
//! * **E0080 with one of our assert messages** — a conservation violation
//!   (R3, R15). It appears only on `cargo build`/`cargo test`, **never in
//!   `cargo check` or editor diagnostics** (F-001); across crates its
//!   primary span lands in `core/src/panic.rs` — read the "while
//!   instantiating `fn split::<…>`" note for the real call site.
//! * **An E0080 naming a draw or budget assert** — a continuous **overdraw**
//!   (R15): the draw exceeds what the container or budget has left. The real
//!   numbers are in the "while instantiating `fn draw_gas::<6000, 0, 5000>`"
//!   note at the caller's line (F-027).
//! * **E0451 "field `_seal` … is private" / E0423** — you may not create
//!   this resource; resources enter the model only through boundary
//!   suppliers and fill functions (R1, R12, F-005).
//! * Aliases are erased in errors: type-level numbers print as `Succ<…>`
//!   nests with no decimal value; long types go to `long-type-*.txt` side
//!   files; the same bound error may appear twice at one call site (F-009).
//! * **"`DrillFail<…>` doesn't implement `Debug`"** (any outcome bundle, on
//!   `.unwrap()`/`.expect()`) — you may not panic past the failure arm:
//!   `match` the `Result` and account for both bundles (R17). The missing
//!   `Debug` is deliberate (F-047).
//! * **A second E0080 whose "while instantiating" note points inside the
//!   library** is a composed-process echo (e.g. `purchase` → `split_money`):
//!   read the first E0080, whose note carries your call site (F-051).
//!   Likewise, a delegating wrapper (`Qualified`'s draw) puts the note one
//!   hop inside the wrapper — the decimal magnitudes still identify the
//!   offending call (F-043).
//! * **E0107 "function takes 4 generic arguments but 3 were supplied"** on a
//!   mixed const/type turbofish — trailing generic arguments cannot be
//!   omitted; add a `_` for the inferred type parameter:
//!   `qualified_draw_time::<2000, 3000, 5000, _>(op)` (F-043).
//! * **Ignore rustc/clippy fix-it suggestions that say** `let _ = …`,
//!   `drop(…)`, "consider borrowing", "consider cloning" or "consider using
//!   `Result::expect`": each one is a conservation violation (F-007). The
//!   real fix is always to pass the resource on, return it, hand it to a
//!   Consumer — or, for a `Result`, match it and account for both bundles
//!   (R17).
//!
//! ## The `test-support` feature (R1, F-004)
//!
//! `#[cfg(test)]` helpers serve the defining crate's own unit tests **only**
//! — a dependency is always compiled without `cfg(test)`. Downstream test
//! fixtures use the `test-support` cargo feature instead, enabled
//! exclusively via a `[dev-dependencies]` re-declaration. Caveats: during
//! `cargo test`, feature unification compiles the downstream crate's
//! *production* sources against the featured library, so only the plain
//! `cargo build` in `ci.sh` proves the boundary; trybuild inherits
//! dev-dependency features and cannot prove it either (F-003); and the
//! feature must never appear under `[dependencies]` anywhere in the graph
//! (`ci.sh` greps for this).

#![recursion_limit = "2048"]
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![deny(let_underscore_drop)]
#![warn(missing_docs)]

pub mod boundary;
pub mod common;
pub mod history;
pub mod list;
pub mod nat;
pub mod quantity;
pub mod requirement;
pub mod resource;

#[cfg(feature = "test-support")]
pub mod fixtures;
