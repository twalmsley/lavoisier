# CS-4 change-impact exercise (SPEC §7) and scale measurements (SPEC §8)

| | |
|---|---|
| Date | 2026-10-07 |
| Toolchain | `rustc 1.98.1 (48a229cea 2026-09-01) (Homebrew)`, `cargo 1.98.1`, macOS (Darwin 24.6.0, Apple Silicon) |
| Model state | `model/` workspace with `cs4-stores` + `cs4-line` complete and `./ci.sh` green before and after the exercise |
| Probe status | **Both probes reverted exactly**; the committed code never contains them. `cargo build --workspace` and the full `./ci.sh` were re-run green after the reverts. |

The point (instructions.md, open question 3): evidence for *"change a type, and the
compiler enumerates every affected process — impact analysis for free."* CS-4 is the
first two-crate model, so this is also the first measurement of how that enumeration
behaves **across a subsystem (crate) boundary**.

## 1. Method

1. Apply one probe edit in `cs4-stores` (the upstream subsystem — both probes are
   stores-side by design, SPEC §7).
2. Run `cargo build --workspace` from `model/` and capture the full error list
   verbatim (build, not check: conservation errors are post-monomorphization, F-001).
3. Where the compiler stops at a failing crate, **acknowledge** that wave (make the
   minimal secondary edit the errors demand — itself part of the probe, reverted with
   it) and rebuild, until the workspace compiles or the radius is fully enumerated.
   This wave-by-wave shape was *discovered by* the exercise, not planned: see §4.
4. Also run `cargo check --workspace` under each probe, to record which waves are
   visible to editors (F-001's distinction).
5. Revert every probe edit exactly; re-run `./ci.sh` green.

Scope note: `cargo build --workspace` monomorphizes **production targets**. CS-4's
full batch flow (`cs4_line::flows::run_batch`, non-generic, P1 → P5×25 → P6) is
production code precisely so the probes blast through real instantiations; the same
probes under `cargo test` would add each unit test's instantiations to the catalogue
(more sites, same shapes).

## 2. Probe A — characteristic change: the issued bolt's length, L15 → L18

**The edit** (`model/cs4-stores/src/resources.rs`): the issued-bolt alias's length
argument —

```rust
pub type IssuedBolt = Bolt<SizeM8, Steel, L15>;
// probe A:
pub type IssuedBolt = Bolt<SizeM8, Steel, crate::characteristics::L18>;
```

`L18` is a legal catalogue value (kind-trait impl only, no `Len15` bridge), so the
entire fallout is the compiler's enumeration, not a missing-name error.

### Wave 1 — `cs4-stores` (1 error; build stops before `cs4-line` is attempted)

| # | File | Model element (process/requirement) | Error |
|---|---|---|---|
| A1 | `cs4-stores/src/resources.rs:175` | the stores-side compile-checked catalogue-spec assertion on `IssuedBolt` (the F-020 tag+assert discipline) | E0277, REQ-phrased via the marker's `on_unimplemented` |

Verbatim (complete):

```text
error[E0277]: `Bolt<SizeM8, Steel, L18>` is not a 15 mm part: the batch's assemblies take 15 mm bolts only
   --> cs4-stores/src/resources.rs:175:41
    |
175 | const _: () = assert_issued_bolt_spec::<IssuedBolt>();
    |                                         ^^^^^^^^^^ a 15 mm bolt is required here
    |
help: the trait `Len15` is not implemented for `Bolt<SizeM8, Steel, L18>`
   --> cs4-stores/src/resources.rs:137:1
    |
137 | / model_core::consumable_resource! {
...   |
149 | |     no_tripwire
150 | | }
    | |_^
    = note: the issued catalogue bolt is `Bolt<SizeM8, Steel, L15>` (`IssuedBolt`); `L18` stock is catalogued but not approved for this assembly
help: the trait `Len15` is implemented for `Bolt<S, M, L15>`
   --> cs4-stores/src/resources.rs:157:1
    |
157 | impl<S: Size, M: Material> Len15 for Bolt<S, M, L15> {}
    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
note: required by a bound in `assert_issued_bolt_spec`
   --> cs4-stores/src/resources.rs:174:60
    |
174 | const fn assert_issued_bolt_spec<T: M8Thread + SteelMade + Len15>() {}
    |                                                            ^^^^^ required by this bound in `assert_issued_bolt_spec`
error: could not compile `cs4-stores` (lib) due to 1 previous error
```

`cargo check --workspace`: **exit 101, same error** — a characteristic change is a
trait-bound (type-check-time) failure, fully visible to editors (contrast probe B).

### Wave 2 — `cs4-line` (1 error)

Acknowledging wave 1 (commenting the stores-side spec assertion — the edit a modeller
deliberately migrating to L18 would make) and rebuilding:

| # | File | Model element | Error |
|---|---|---|---|
| A2 | `cs4-line/src/batch.rs:193` — the `fasten` call inside the `BuildBatch` cycle body (P4 inside P5) | the line's P4 bolt-spec bound `B: … + Len15` | E0277, same marker message, with the note naming the bound in `fasten` at `cs4-line/src/processes.rs:69` |

Verbatim (complete):

```text
error[E0277]: `Bolt<SizeM8, Steel, L18>` is not a 15 mm part: the batch's assemblies take 15 mm bolts only
   --> cs4-line/src/batch.rs:193:46
    |
193 |         let (bench, assembly, bolts, note) = fasten(bench, plate_a, plate_b, bolts, note);
    |                                              ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ a 15 mm bolt is required here
    |
    = help: the trait `Len15` is not implemented for `Bolt<SizeM8, Steel, L18>`
    = note: the issued catalogue bolt is `Bolt<SizeM8, Steel, L15>` (`IssuedBolt`); `L18` stock is catalogued but not approved for this assembly
help: the trait `Len15` is implemented for `Bolt<S, M, L15>`
   --> cs4-stores/src/resources.rs:157:1
    |
157 | impl<S: Size, M: Material> Len15 for Bolt<S, M, L15> {}
    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
note: required by a bound in `fasten`
   --> cs4-line/src/processes.rs:69:70
    |
 69 | pub fn fasten<B: Req019StoresIssuedMaterial + M8Thread + SteelMade + Len15, S: SupplyN<N4, Taken = FourOf<B>>, PN: Req021CurrentIss...
    |                                                                      ^^^^^ required by this bound in `fasten`
error: could not compile `cs4-line` (lib) due to 1 previous error
```

**Probe A radius: 2 errors, 2 waves, 2 crates** — the stores-side catalogue-spec
assertion, then the line-side process bound at the exact call inside the batch cycle.
Notably: `join_assembly` (the stores transform carrying the same `Len15` bound) does
**not** error separately — it is called from inside generic `fasten`, where `B: Len15`
is assumed by `fasten`'s own bound, so the obligation surfaces once, at the outermost
place a concrete `L18` bolt meets the bound. And the 25 unrolled cycles produce **one**
error, not 25: enumeration is per *source location*, not per monomorphized repetition.

## 3. Probe B — quantity change: the sheet mass constant, 5000 → 4800

**The edit** (`model/cs4-stores/src/resources.rs`):

```rust
pub const SHEET_G: u64 = 5000;
// probe B:
pub const SHEET_G: u64 = 4800;
```

### Wave 1 — `cs4-stores` (1 error), and it is check-visible

The first casualty is a stores-side **top-level** batch-arithmetic assertion:

```text
error[E0080]: evaluation panicked: assertion failed: 2 * BLANK_G + CUT_SWARF_G == SHEET_G
  --> cs4-stores/src/resources.rs:84:15
   |
84 | const _: () = assert!(2 * BLANK_G + CUT_SWARF_G == SHEET_G);
   |               ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ evaluation of `resources::_` failed here
error: could not compile `cs4-stores` (lib) due to 1 previous error
```

`cargo check --workspace` under this wave: **exit 101** — an unexpected and useful
nuance of F-001: a *non-generic* `const _` item is evaluated at check time, so batch
arithmetic stated as top-level const items **is** editor-visible, unlike the same
arithmetic inside generic processes. (This is the zero-cost cousin of F-001's
call-site-macro mitigation.)

### Wave 2 — `cs4-line` (1 error), and it is the F-001 shape exactly

Acknowledging wave 1 (commenting that one const item) and rebuilding:

```text
error[E0080]: evaluation panicked: mass conservation violated in shear_sheet (R3, P2): the two blanks plus the cut swarf must sum exactly to the sheet
   --> /usr/local/Cellar/rust/1.98.1/lib/rustlib/src/rust/library/core/src/panic.rs:62:9
    |
 62 |           $crate::panicking::panic_fmt($crate::const_format_args!($($t)+));
    |           ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ evaluation of `cs4_stores::resources::processes::shear_sheet::<4800, 2250, 2250, 500>::{constant#0}` failed here
    |
   ::: cs4-stores/src/resources.rs:802:13   (the const { assert!(A + B + SW == SHEET, …) } in shear_sheet)
note: the above error was encountered while instantiating `fn shear_sheet::<4800, 2250, 2250, 500>`
  --> cs4-line/src/processes.rs:35:37
   |
35 |     let (blank_a, blank_b, swarf) = shear_sheet::<SHEET_G, BLANK_G, BLANK_G, CUT_SWARF_G>(sheet);
   |                                     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
error: could not compile `cs4-line` (lib) due to 1 previous error
```

`cargo check --workspace` under this wave: **exit 0 — the violation is invisible to
`cargo check` and to editor diagnostics** (demonstrated directly, as SPEC §7 asks;
this is F-001). The primary span lands in `core/src/panic.rs` (the documented
cross-crate E0080 shape); the real site is the "while instantiating
`fn shear_sheet::<4800, 2250, 2250, 500>`" note, which points at the **cut process**
(P2) in `cs4-line` with the real decimal magnitudes (F-027).

**Probe B radius: 2 errors, 2 waves, 2 crates** — the stores batch-arithmetic item,
then the one conservation assert the changed quantity unbalances, located at P2's
instantiation. Again one error for 25 unrolled cuts: `shear_sheet::<4800, 2250, 2250,
500>` is a single instantiation however many cycles use it.

## 4. The two radii, compared

| | Probe A (characteristic) | Probe B (quantity) |
|---|---|---|
| Errors (total / per wave) | 2 (1 + 1) | 2 (1 + 1) |
| Error class | E0277 trait bound, REQ-/characteristic-phrased | E0080 const-eval, assert-message-phrased |
| Where the radius runs | requirements & catalogue bridging: the F-020 spec assertion, then the P4 process bound | conservation asserts: the batch-arithmetic const item, then P2's mass balance |
| `cargo check` / editor visibility | **visible** (type-check time) | wave 1 visible (non-generic const item); wave 2 **invisible — build only** (F-001) |
| Error location quality | the exact bound and the exact call in the batch cycle, with the "which bolt would satisfy this" impl listed | primary span in `core/src/panic.rs`; the instantiation note carries the true site and the real numbers |
| What the human still re-derives | whether the spec itself moves to 18 mm (bounds + marker bridge + prose) | the new split (2250/2250/500 does not divide 4800) — the compiler rejects, the human proposes |

**Two structural results, both first visible at two-crate scale:**

1. **The enumeration is iterative, in per-crate waves.** `cargo build` stops at the
   first failing crate, so the downstream half of the radius only appears after the
   upstream wave is acknowledged. That is not a weakness so much as the shape of real
   change propagation — each wave's error list is exactly the next set of places to
   touch, and nothing downstream can be silently missed because the downstream crate
   *cannot compile* until its wave is addressed — but "the compiler enumerates every
   affected process" must be read as "per wave", not "in one listing".
2. **Generic processes collapse repetition out of the radius.** The 25-cycle
   recursion contributes one error per distinct source location/instantiation, not
   per cycle. The catalogue is short because the model is DRY, not because the
   impact is small.

**Completeness:** within what the type system states, yes — every affected *process*
was named (A: fasten/P4, via its bound; B: shear_sheet inside cut/P2, via its
assert). Each wave's `cargo build --workspace` log contained exactly the one error
listed and nothing else (every other workspace crate kept compiling), so for these
probes the per-wave lists are the complete production-target radius; resolving the
final errors is a *design* decision (move the spec to 18 mm; choose a split of
4800), not a further enumeration. What the compiler cannot enumerate: prose (doc
comments saying "15 mm" or "5000 g"), and those design re-derivations — it rejects
wrong numbers rather than proposing right ones (F-022/F-030, unchanged at scale).

**Error quality at scale:** probe A's errors are the best in the case study — the
marker's `on_unimplemented` sentence leads, the "implemented for `Bolt<S, M, L15>`"
help names the fix, and the bound note names the process. Probe B's wave-2 E0080
needs the error-reading guide (span in `core`, real site in the note) but carries
exact decimal magnitudes. Neither probe produced a single long-type side file or a
`Succ`-chain in an error (F-009's costs did not surface on these paths).

## 5. Candidate FINDINGS entries from §7

- **(extends F-001/F-006) Cross-crate change impact arrives in per-crate waves**, and
  generic processes collapse repetition: a probe's full radius is enumerated
  iteratively (acknowledge wave *n* to see wave *n*+1), one error per source
  location regardless of how many monomorphized cycles share it.
- **(extends F-001, positive) Non-generic `const _: () = assert!(…)` items are
  check-time-visible**: batch arithmetic stated once over the workspace constants
  gives editors the quantity-change signal that in-process generic asserts cannot
  (probe B wave 1 vs wave 2).

## 6. Scale measurements (SPEC §8)

All on the toolchain above, same machine, `model/` workspace. "Cold" = after
`cargo clean` (which itself takes 0.07 s).

### 6.1 Cold CI and build wall time, before vs after CS-4 joins the workspace

| Measurement | Before CS-4 (5 crates) | After CS-4 (7 crates) | Delta |
|---|---|---|---|
| Cold `./ci.sh` (build + test + trybuild + doc-tests + clippy + plain build + trace) | **44.91 s** (user 111.97 s) | **45.52 s** (user 115.84 s) | +0.61 s (+1.4 %) |
| Cold `cargo build --workspace` (production targets) | **1.60 s** / 1.64 s (two runs) | **2.43 / 2.48 / 2.48 s** (three runs, median 2.48 s) | +~0.85 s |
| Non-incremental compile of `cs4-line` alone (deps ready) — includes the full 25-cycle `BuildBatch` monomorphization and trait solving | — | **1.04 s** | |
| Non-incremental compile of `cs4-stores` alone — includes the 100-bolt box and 25-sheet rack fill machinery | — | **0.42 s** | |
| Incremental touch-rebuild (`touch src/lib.rs`) | — | cs4-line 0.30 s, cs4-stores 0.26 s | |

**Conclusion:** the 25-cycle composite recursion (≈100 threaded steps, each step
resolving `Supplier`/`SupplyN<N4>`/three `Consumer` obligations over types carrying a
25-deep rack, 100-deep box and a bin whose contents grow to 75) costs about one
second — squarely on F-011's flat region, now confirmed at a *real composite* depth
rather than a synthetic single-trait one. Compile time is a non-issue at SPEC §3's
scale targets; nothing suggests the 50-unit variant (SPEC §10 question 4) would be
either.

### 6.2 Long-type side files (F-009)

A green build/test/CI run produces **zero** `long-type-*.txt` files (checked across
`target/` after the full cold `ci.sh`). Side files appeared only on **diagnostic**
paths during development, exactly as F-009 predicts for types past ~130: the E0275
recursion-limit probes below each wrote one (e.g.
`target/debug/deps/flows-….long-type-….txt` carrying the full `BatchRig` with the
25-sheet rack, 100-bolt box and contents list). F-009 confirmed, with the refinement
that the side-file cost is paid only when something is already wrong.

### 6.3 Recursion-limit headroom at the mandated 2048 (F-010)

Measured by lowering `#![recursion_limit]` per crate until E0275 (then restoring
2048):

| Crate | Minimum passing limit | Fails at | Driver |
|---|---|---|---|
| `cs4-stores` lib | **151** | 150 | the 100-bolt `ListOf<IssuedBolt, N100>` fill/`Replicate` chains (≈ N + ~50 — noticeably more than F-010's bare ≈ N + 3, because `Replicate`, the sealed `Fill` and `Len` stack on the same chain) |
| `cs4-line` lib | **143** (fails at 141) | 141 | the `BuildBatch<N25>` end-state types (box 100-list inside the rig) |
| `cs4-line` integration test (`tests/flows.rs`) | needs the attribute too — **overflowed at the default 128** merely *naming* `StockedFinishedGoods` | | |

Headroom at the mandated 2048: ≈ **13×** — comfortable, and the F-034 SIGBUS edge was
never approached (every contents-keeping consumer here has its decreasing space
parameter). **New wrinkle for F-010:** the attribute is per crate *including test
crates* — CS-4 is the first model whose integration test itself needed
`#![recursion_limit = "2048"]`, because the end-state type aliases walk the full
25/100-deep types. The default limit no longer covers a 100-item supplier once fill
and length machinery share the chain: F-010's "~100 is fine at the default" should be
read as "~75–80" for this crate shape.

### 6.4 New limits found (and the one design casualty)

- **A `const`-generic time budget cannot ride a recursive type-level batch** — the
  recursive `BuildBatch` impl would have to name the next cycle's budget
  (`B - 150_000`: `generic_const_exprs`, nightly) and an extra inferred impl const is
  rejected as unconstrained (E0207) — the F-028/F-030 shape at batch scale. The
  stable encoding adopted: the operator's shift as a **type-level quantum clock**
  (`Operator<Q: Nat>`, 150 × 30 000 ms quanta), with fixed-size draws minting a
  conserved `Effort<MS>` in real milliseconds. Costs, measured: a parallel
  effort family beside model-core's `Person`/`Labour` (the ~40-line F-043 fork,
  since `Labour`'s mint is private to model-core), and overdraw errors that are
  type-check-time trait/type errors on the Peano route (editor-visible, unlike R15's
  E0080s — arguably an upgrade, at the price of quantized time). **Candidate
  FINDINGS entry; R15 deserves a note.**
- No compile-time cliff, no solver divergence, no SIGBUS, no new E-code: at CS-4's
  mandated sizes the scale story is bounded by the recursion limit and by error
  ergonomics, not by time or stability.

### 6.5 Candidate FINDINGS entries from §8

1. (extends F-010) Recursion-limit minima at CS-4 scale: 151 for a 100-item supplier
   with fill/`Replicate`/`Len` machinery, 143 for the 25-cycle batch rig; **test
   crates need the attribute too**; 2048 leaves ≈13× headroom.
2. (extends F-011) A real composite recursion — 25 cycles × (1 supply + 1 `SupplyN<N4>`
   + 3 consumes + 5 clock decrements) over 100-deep stock types — compiles in ~1 s;
   the flat region holds at composite depth.
3. (extends F-028/F-030/F-043) The const-budget-cannot-recurse result and the
   quantum-clock workaround (§6.4).
4. (extends F-055) The diagram generator is per-crate: in the two-crate model it
   renders each crate alone, WARNs on every cross-crate callee in a traced flow
   ("unknown callee `issue_materials` … shown as a plain step"), and the line's
   top-level connection graph collapses to 2 nodes because every edge type it would
   draw is defined upstream. Cross-crate (subsystem-interface) diagramming is a real
   gap for multi-crate models.
