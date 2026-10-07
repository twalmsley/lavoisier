//! # cs4-line — case study CS-4, the production-line subsystem
//!
//! The downstream half of the first **multi-crate model** (SPEC §3): this
//! crate owns the line's processes (P2 cut, P3 drill, P4 fasten), the swarf
//! bin, and **P5 — the 25-unit batch as type-level recursion**, the scale
//! centrepiece of CS-4. It depends on `cs4-stores` and **may create nothing
//! material**: every sheet, blank, plate, bolt, swarf piece, assembly and
//! issue note is sealed in the stores crate, so the crate edge *is* REQ-019
//! (provenance) — `tests/ui/line_cannot_mint_materials.rs` pins the privacy
//! errors a line that tries gets (E0451/E0624, EXP-08/F-005).
//!
//! ## Module map
//!
//! | Module | Contents |
//! |---|---|
//! | [`resources`] | The line's own sealed resources: the reusable tools (saw, pillar drill, workbench — R2) and the **swarf bin** (capacity 80, contents-keeping with a decreasing space parameter — the F-034 hard rule), plus the line's `boundary` and the cross-crate `SwarfReturn` impl that lets stores' P6 empty the bin |
//! | [`processes`] | P2 [`processes::cut`], P3 [`processes::drill_blank`], P4 [`processes::fasten`] — each composes a stores material transform (the conserving mint sites live across the crate edge) with the line's tools and waste routing; swarf is fed to the bin **inside** cut and drill (SPEC §5); fasten takes four bolts in one `SupplyN<N4>` bound (F-014) and threads the REQ-021 issue note |
//! | [`batch`] | **P5**: the [`batch::BatchRig`] aggregate and the recursive [`batch::BuildBatch`]`<N>` trait — the `SupplyN` pattern at batch scale. Each cycle takes one sheet, runs cut → drill ×2 → fasten, draws 150 000 ms in four adjacent, process-named draws, and recurses with every threaded type evolved (rack −1 sheet, box −4 bolts, bin +3 swarf, clock −5 quanta). A Rust loop cannot express this — the types change every cycle |
//! | [`flows`] | The full batch (SPEC §6): P1 → P5 (25 cycles) → P6 as the production flow [`flows::run_batch`], returning the fully-accounted [`flows::BatchOutcome`] the integration test asserts SPEC §6's end state over |
//!
//! ## Why the batch is a recursive trait, not a loop
//!
//! Every cycle changes the types of everything it threads: the rack is one
//! sheet shorter, the box four bolts emptier (100 → 96 → … → 0), the bin's
//! space one–three smaller and its contents list longer (0 → 75 pieces), the
//! operator's shift clock five quanta down. A `for` loop needs a single type
//! for its loop-carried state, so the honest encoding is a recursive trait
//! ([`batch::BuildBatch`]) with a base case at `Zero` and a step case at
//! `Succ<N>` — exactly model-core's `SupplyN` pattern (F-014) at batch
//! scale. The compiler unrolls all 25 cycles at monomorphization, which is
//! the compile-time cost SPEC §8 measures (F-011 at a real composite depth).
//!
//! One deviation forced by stable Rust, recorded in `cs4-stores`' crate docs
//! and the CS-4 findings: the operator's budget rides the recursion as a
//! **type-level quantum clock** (30 000 ms quanta), because a
//! `Person<const BUDGET_MS>` cannot descend through a recursive trait
//! (`B - 150_000` is nightly-only const arithmetic; an inferred impl const
//! is E0207 — the F-028/F-030 shape).
//!
//! ## The 26th cycle is inexpressible (SPEC §6: the rack is the binding stock)
//!
//! Asking the issued rig for 26 assemblies fails at type-check time: at
//! cycle 26 the rack is `SheetRack<Nil>`, which supplies nothing (R12,
//! F-015):
//!
//! ```compile_fail
//! use cs4_line::batch::BuildBatch;
//! use cs4_line::flows::StartRig;
//! use model_core::nat::aliases::N26;
//!
//! fn probe<T: BuildBatch<N26>>() {}
//! probe::<StartRig>();
//! ```
//!
//! The same probe at the real batch size compiles (the positive control —
//! `flows::run_batch` instantiates `BuildBatch<N25>` in production code, so
//! the plain `cargo build` in ci.sh monomorphizes all 25 cycles and every
//! conservation assert in them, F-001).
//!
//! ## Conservation regime (R1)
//!
//! This crate carries the full obligations listed in `model-core`'s crate
//! docs: `recursion_limit = "2048"`, `forbid(unsafe_code)`, the
//! must-use/underscore lints, and CI clippy under `-D warnings` with the
//! workspace `clippy.toml` ban list. Never write `let _ = <resource>`, and
//! ignore compiler fix-its suggesting borrows, clones, drops or `.expect(…)`
//! (F-007).

#![recursion_limit = "2048"]
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![deny(let_underscore_drop)]
#![warn(missing_docs)]

pub mod batch;
pub mod flows;
pub mod processes;
pub mod resources;
