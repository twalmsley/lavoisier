//! # cs4-stores — case study CS-4, the stores subsystem
//!
//! The fourth spec→model round trip, and the **first multi-crate model**:
//! CS-4 is the **scale and structure** case study, implemented downstream of
//! `model-core` from the agreed natural-language specification
//! `case-studies/cs4-batch-run/SPEC.md` (v0.2, agreed) as **two crates, two
//! subsystems** (SPEC §3). This crate is **stores**: it owns every material
//! type and its seal, the stock, the issue note, the issue/reconcile
//! processes and the finished-goods/disposal consumers. The sibling crate
//! `cs4-line` (the production line) depends on it and **may create nothing
//! material**: the crate edge is REQ-019 (provenance), carried by Rust's
//! privacy across crates (EXP-08, F-005/F-006) — the hard boundary the pilot
//! only had at module level.
//!
//! ## Module map
//!
//! | Module | Contents |
//! |---|---|
//! | [`characteristics`] | The bolt catalogue's kind traits and values (R6, F-018), the characteristic markers with bridging impls declared beside the catalogue (F-019), and the **sealed** subsystem characteristics behind REQ-019..022 ([`characteristics::StoresIssued`], [`characteristics::ProofOfIssue`], [`characteristics::DisposalSink`], [`characteristics::FinishedGoodsStore`]) |
//! | [`requirements`] | The four modelled-system requirements of SPEC §2 as R10 requirement traits, each with a REQ-phrased `on_unimplemented` (R10 rule 8, F-044). The workspace trace namespace is global (F-053): the pilot owns REQ-001..005 and CS-1..3 own REQ-006..018, so CS-4 starts at REQ-019 exactly as SPEC §2 allocates |
//! | [`resources`] | The sealed material family (R1): sheet/blank/drilled-plate/bolt/swarf/assembly, the 25-sheet rack and 100-bolt box (type-level stock, R12), the issue note, works order, operator shift clock and `Effort`, finished goods and disposal, the `boundary` and `processes` child modules (F-006, F-031) — including the conserving material transforms the line composes (`shear_sheet`, `bore_blank`, `join_assembly`) and the subsystem processes P1 (`issue_materials`) and P6 (`reconcile`) |
//!
//! ## Scale targets carried by this crate (SPEC §3, §8)
//!
//! * the **sheet rack**: 25 sheets as a type-level list supplier;
//! * the **bolt box**: 100 bolts as a type-level list supplier — right at the
//!   F-010 measured frontier (the default recursion limit allows ~100; this
//!   workspace mandates `recursion_limit = "2048"`);
//! * **finished goods**: a contents-keeping consumer (decreasing space,
//!   F-034) that ends the batch holding 25 assemblies.
//!
//! ## The operator's shift clock (a deliberate deviation from R15's const-ms budget)
//!
//! SPEC §5/P5 threads the operator through a 25-cycle **type-level
//! recursion** with the budget descending. A `Person<const BUDGET_MS: u64>`
//! budget **cannot ride a recursive trait on stable Rust**: the recursive
//! impl would have to name the next cycle's budget (`B - 150_000`), which is
//! const arithmetic in a type (`generic_const_exprs`, nightly), and an
//! inferred extra const parameter on the impl is rejected as unconstrained
//! (E0207) — the F-028/F-030 shape at batch scale. The stable encoding is a
//! **type-level quantum clock**: [`resources::Operator<Q>`] carries its
//! remaining budget as a Peano count of 30 000 ms quanta
//! ([`resources::QUANTUM_MS`]; the 75-minute shift is
//! [`resources::SHIFT_QUANTA`] = 150 quanta), drawn down by the fixed-size
//! draws in [`resources::processes`], each of which mints a conserved
//! [`resources::Effort`] in real milliseconds that must reach the execution
//! `History` (R16). The cost is a parallel effort family beside model-core's
//! `Person`/`Labour` (~40 lines, the F-043 measured price) and overdraw
//! errors that are type-check-time trait/type errors rather than E0080s —
//! both recorded as a CS-4 finding candidate.
//!
//! ## Conservation regime (R1)
//!
//! This crate carries the full obligations listed in `model-core`'s crate
//! docs: `recursion_limit = "2048"`, `forbid(unsafe_code)`, the
//! must-use/underscore lints, tripwire `Drop`s on every consumable (kernel
//! macros), and CI clippy under `-D warnings` with the workspace
//! `clippy.toml` ban list. Never write `let _ = <resource>`, and ignore
//! compiler fix-its suggesting borrows, clones, drops or `.expect(…)`
//! (F-007).
//!
//! ## Change-impact probes (SPEC §7)
//!
//! This crate is the target of the two documented (and reverted) probes:
//! **probe A** changes [`resources::IssuedBolt`]'s length characteristic
//! (`L15` → `L18` — the catalogue carries both values so the probe is a
//! one-token edit), and **probe B** changes [`resources::SHEET_G`]
//! (5000 → 4800). The catalogues of everything the compiler reports are in
//! `case-studies/cs4-batch-run/CHANGE-IMPACT.md`.

#![recursion_limit = "2048"]
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![deny(let_underscore_drop)]
#![warn(missing_docs)]

pub mod characteristics;
pub mod requirements;
pub mod resources;
