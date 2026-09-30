# EXP-03 Results: How close to "nothing is silently lost"?

**Toolchain:** `rustc 1.98.1 (48a229cea 2026-09-01) (Homebrew)`, cargo 1.98.1, clippy 0.1.98. Dev-dependency: trybuild 1.0.121.
**Build time:** `cargo clean && time cargo build`, three runs: 1.33 s / 1.68 s / 1.30 s → **median 1.33 s** (all mechanisms are effectively free at this scale).

**Crate layout**

| Where | What |
|---|---|
| `src/lib.rs` | Sample resources: `Bolt` (plain, `#[must_use]` only), `TrackedBolt` (tripwire `Drop`, guarded by `thread::panicking()`), `StrictBolt` (unguarded tripwire), `Assembly` (composite), plus `early_return_leak` |
| `tests/leak_paths.rs` | 12 runtime demos: every "caught at test time" cell as `#[should_panic(expected = "resource leak")]`, every runtime "not caught" cell as a test that passes silently |
| `tests/compile_fail/` | trybuild evidence for the rustc-caught cells (`unused_must_use`, `let_underscore_drop`, `unused_variables`) |
| `src/bin/unwind_abort.rs` | The unguarded-tripwire-during-unwind demo; run as a subprocess by a test that asserts SIGABRT |
| `clippy-demos/` | Deliberately failing crate (workspace-excluded): one function per clippy-caught cell, plus three functions clippy says nothing about — the compile-time "not caught" evidence |

`cargo test` passes: 12 runtime tests + 3 trybuild compile-fail tests. The main crate is clean under `cargo clippy --all-targets -- -D warnings -D clippy::mem_forget -D clippy::let_underscore_must_use` (the deliberate leak demos carry commented `#[allow]`s).

---

## 1. The matrix (main result)

Mechanisms: **A** = `#[must_use]` on the type + `#![deny(unused_must_use)]`; **B** = tripwire `Drop` (panics unless defused by `consume(self)`, which `mem::forget`s internally); **C** = lints under `-D warnings` (rustc's on-by-default lints, `let_underscore_drop`, and clippy's `let_underscore_must_use`, `mem_forget`, `forget_non_drop`, `shadow_*`, `disallowed_methods` via clippy.toml).

**COMPILE** = caught at compile time, **TEST** = caught at test time, **—** = not caught. Every — cell has a minimal demo (rightmost column; test names from `tests/leak_paths.rs`, functions from `clippy-demos/src/lib.rs`).

| Leak path | A: must_use | B: tripwire Drop | C: lints | Demo of the — cells |
|---|---|---|---|---|
| 1a. Unused expression statement `new_bolt();` | **COMPILE** | TEST | COMPILE (A is itself a lint) | — |
| 1b. End of scope, binding never used | — | TEST | **COMPILE** (`unused_variables`, on by default) | (1b×A subsumed by 1c demo) |
| 1c. End of scope, binding used then dropped | — | **TEST** | — | `end_of_scope_drop_of_used_binding_not_caught_without_tripwire` (passes silently); `leak_at_end_of_scope_not_caught` (clippy silent) |
| 2. `let _ = expr` | — (this idiom is must_use's official silencer) | **TEST** | **COMPILE** (`clippy::let_underscore_must_use` for any must_use type; rustc `let_underscore_drop` for Drop types only) | compiles under `deny(unused_must_use)` in `leak_paths.rs` |
| 3. Rebinding / shadowing | — | **TEST** (drop is deferred to end of scope, not the shadow point) | **COMPILE**, with caveats: `unused_variables` iff the shadowed binding was never used; `clippy::shadow_unrelated` otherwise, but the `shadow_*` lints also flag conservation-*correct* rebinding like `let bolt = inspect(bolt);` | `shadowing_tripwire_catches` needed `#[allow(unused_variables)]` — evidence for the iff |
| 4. `mem::forget` | — (forget *is* a use) | **—** (forget skips Drop; it is exactly what `consume()` uses) | **COMPILE** (`clippy::mem_forget` for Drop types — restriction; `clippy::forget_non_drop` for the rest — on by default) | `mem_forget_defeats_tripwire_not_caught` (passes silently) |
| 5. `mem::drop` / prelude `drop()` | — (drop is a use) | **TEST** | **COMPILE** (`clippy::disallowed_methods` + clippy.toml entry; catches bare `drop(bolt)` as `core::mem::drop`) | — |
| 6. Struct pattern `..` dropping fields | — (must_use ignored in patterns) | **TEST** | — (`rest_pat_in_fully_bound_structs` only fires when `..` hides nothing) | `leak_by_rest_pattern_not_caught` (clippy silent) |
| 7. Early `return` | — | **TEST — only if a test exercises that branch** | — | `early_return_leak_invisible_on_untested_branch` (passes); `leak_by_early_return_not_caught` (clippy silent) |
| 8. Panic unwinding | — | guarded tripwire: **—** (stands down via `thread::panicking()`); unguarded: **TEST, catastrophically** — double panic → SIGABRT kills the whole test binary | — | `unwinding_guarded_tripwire_misses_leak` (leak invisible behind the unrelated panic); `unwinding_vs_strict_tripwire_aborts_process` (subprocess, exit 134) |

**Headline reading.** No mechanism, and no combination, makes "nothing is silently lost" a compile-time guarantee: paths 1c, 6 and 7 have **no compile-time detection at all**, and path 8 has none by any mechanism. Layered, though, the coverage is nearly complete at test time: the tripwire catches every path except `mem::forget` (its own escape hatch — which is precisely the path clippy fully catches at compile time) and unwinding (a secondary loss that only happens when a test is already failing). The mechanisms are complementary, not redundant: the tripwire also *arms* `clippy::mem_forget`, which only fires for Drop types.

---

## 2. Per-criterion verdicts

**C1 — Leak-path catalogue complete, each as a small compiling example: pass.**
All eight paths from the brief are demonstrated (`tests/leak_paths.rs`, `src/lib.rs::early_return_leak`, `src/bin/unwind_abort.rs`, `clippy-demos/src/lib.rs`). One addition surfaced while building: path 1 splits into three genuinely different cases (unused statement / never-used binding / used-then-dropped binding) with different detection.

**C2 — Counter-mechanisms applied and behaving as designed: pass with complications.**
- `#[must_use]` + `deny(unused_must_use)`: works, but covers exactly one cell (1a). Complication: rustc's *suggested fix* for the error is `let _ = ...` — i.e. the compiler officially recommends leak path 2 (see §3).
- Tripwire `Drop` + `consume(self)`/`mem::forget`: works and catches 6 of 8 paths at test time. Complications: (i) it must check `thread::panicking()` or any ordinary test failure with a live resource becomes a process abort (demonstrated: exit 134/SIGABRT), and that guard is what surrenders path 8; (ii) `consume` needs a commented `#[allow(clippy::mem_forget)]` — one blessed hole in the lint wall; (iii) detection is only as good as test coverage (path 7).
- Clippy under `-D warnings`: works for paths 2, 4, 5 and partially 3. Complications: `mem_forget`, `let_underscore_must_use` and `shadow_*` are in the `restriction` group (opt-in); `disallowed_methods` needs a clippy.toml entry because no "don't drop this type" lint exists; the `shadow_*` lints tax the idiomatic R2 style (`let bolt = inspect(bolt);` trips `shadow_reuse`), so they were **not** adopted for the main crate.

**C3 — Matrix complete, minimal demo for every "not caught" cell: pass.**
Table above; every — cell names its demo. The "not caught" demos are tests that *pass silently* — the strongest form of the evidence, and they keep `cargo test` green as the protocol requires.

**C4 — The claim under test ("nothing is silently lost" via these mechanisms): fail at compile time, pass with complications as a layered compile+test regime.**
This is the expected outcome — the experiment quantifies the known affine-types gap rather than contradicting it. Compile-time-only residue: paths 1c, 6, 7 (plus 8). With the tripwire and tests added, the residue shrinks to: untested branches (7), unwind-path losses (8), and `ManuallyDrop`/`Box::leak`-style escapes (see gaps).

---

## 3. Representative verbatim output, with readability judgements

**1a × must_use (compile time, trybuild snapshot `tests/compile_fail/unused_must_use.stderr`):**
```text
error: unused `TrackedBolt` that must be used
 --> tests/compile_fail/unused_must_use.rs:9:5
  |
9 |     boundary::new_tracked_bolt(); // result dropped on the spot
  |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  |
  = note: a TrackedBolt is a conserved resource: pass it on or consume it explicitly
help: use `let _ = ...` to ignore the resulting value
  |
9 |     let _ = boundary::new_tracked_bolt(); // result dropped on the spot
  |     +++++++
```
*Judgement:* excellent for a modeller — the `#[must_use = "..."]` reason string puts the conservation rule in the error itself. **But** the `help:` line recommends `let _ =`, which is leak path 2: without `clippy::let_underscore_must_use` also enabled, the compiler steers users from a caught leak into an uncaught one.

**2 × rustc `let_underscore_drop` (compile time, `let_underscore_drop.stderr`):**
```text
error: non-binding let on a type that has a destructor
  --> tests/compile_fail/let_underscore_drop.rs:10:5
   |
10 |     let _ = boundary::new_tracked_bolt();
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
...
help: consider immediately dropping the value
   |
10 +     drop(boundary::new_tracked_bolt());
```
*Judgement:* understandable, though "has a destructor" is compiler-speak; a modeller needs the convention "destructor = tripwire = conserved resource". Its help text in turn suggests `drop(...)` — leak path 5 — so the lints keep handing users the next leak down the list; only the full set closes the loop.

**4 × clippy `mem_forget` (compile time, from `clippy-demos`):**
```text
error: usage of `mem::forget` on `Drop` type
  --> src/lib.rs:31:5
   |
31 |     std::mem::forget(boundary::new_tracked_bolt());
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = note: argument has type `exp03_drop_prevention::TrackedBolt`
   = note: `-D clippy::mem-forget` implied by `-D warnings`
```
*Judgement:* clear and points at the exact expression; fine for a modeller once "why is forget banned" is documented.

**5 × clippy `disallowed_methods` with clippy.toml (compile time):**
```text
error: use of a disallowed method `core::mem::drop`
  --> src/lib.rs:39:5
   |
39 |     drop(bolt);
   |     ^^^^
   |
   = note: resources are conserved: hand them to a Consumer or call consume(), never drop them
```
*Judgement:* the best message of the lot, because the `reason` string from clippy.toml is written by us — it tells the modeller what to do instead. It also catches the bare prelude `drop`, not just the spelled-out path.

**Tripwire firing (test time; same format as any `should_panic` evidence, here from the unwind demo binary):**
```text
thread 'main' (13995996) panicked at src/lib.rs:87:9:
resource leak: StrictBolt was dropped without consume()
```
*Judgement:* the message is ours, so it is as readable as we make it; the reported location is the `Drop` impl, **not the leak site** — the modeller must read the backtrace (or the failing test's name) to find where the resource was lost. Acceptable in tests, weak for debugging large flows.

**Unguarded tripwire during unwinding (the reason for the `thread::panicking()` guard):**
```text
thread 'main' (13995996) panicked at src/bin/unwind_abort.rs:19:5:
some unrelated failure
...
thread 'main' (13995996) panicked at src/lib.rs:87:9:
resource leak: StrictBolt was dropped without consume()
...
thread 'main' (13995996) panicked at library/core/src/panicking.rs:233:5:
panic in a destructor during cleanup
thread caused non-unwinding panic. aborting.
```
Exit status 134 (SIGABRT). *Judgement:* actively harmful in a test suite — the abort kills the whole test binary and buries the original failure; hence the guard, and hence the surrendered unwind cell in the matrix.

---

## 4. Candidate FINDINGS.md entries (ready to paste)

> **Silent loss is not fully closable at compile time (R1, target-language known gap — now quantified).** Of the eight drop/leak paths, three have no compile-time detection by any stable mechanism: end-of-scope drop of a binding that was used at least once; struct-pattern `..` silently dropping fields; and drop on an early-return path. Panic unwinding additionally evades everything (detecting it requires a `Drop` that panics during unwind, which double-panics and aborts the process — worse than the leak). Workaround: the panicking-`Drop` tripwire pattern catches all of these at *test* time except unwinding, turning "nothing is lost" into a property of test coverage. Cost: every resource type needs the tripwire boilerplate (`Drop` impl guarded by `thread::panicking()`, plus a `consume(self)` defuser), and every legitimate disposal must go through an explicit consume/Consumer call. Evidence: `experiments/exp03-drop-prevention/` matrix and `tests/leak_paths.rs`.

> **The compiler's own fix-it suggestions form a chain of leaks (R1).** `deny(unused_must_use)`'s help text suggests `let _ = ...` (a silent leak), and `deny(let_underscore_drop)`'s help text suggests `drop(...)` (another silent leak). Each mechanism's official escape hatch is the next leak path, so the lint set only works as a *complete* set: `unused_must_use` + `let_underscore_drop` + `clippy::let_underscore_must_use` + `clippy::disallowed_methods(mem::drop)` + `clippy::mem_forget`/`forget_non_drop`. Adopting a subset gives a false sense of safety. Evidence: verbatim help texts in `experiments/exp03-drop-prevention/RESULTS.md` §3.

> **`mem::forget` and the Drop tripwire are exact complements (R1).** The tripwire catches every leak path at test time except `mem::forget` (forget skips `Drop`; it is what `consume()` itself uses), while `clippy::mem_forget` catches forget at compile time but only fires for types *with* drop glue — i.e. the tripwire arms the lint (`clippy::forget_non_drop`, on by default, covers non-Drop types). The project needs exactly one `#[allow(clippy::mem_forget)]` site per resource: inside `consume()`. Residual: `ManuallyDrop::new`, `Box::leak` and raw-pointer round-trips are forget-equivalents with no dedicated lint; they must be banned by convention/`disallowed_methods` entries or code review.

> **Clippy's `shadow_*` lints conflict with the R2 move-in/move-out style.** Shadowing a resource binding leaks the old value (deferred to end of scope), and `clippy::shadow_unrelated` catches exactly that — but `clippy::shadow_reuse` also fires on the conservation-*correct* idiom `let bolt = inspect(bolt);` that R2 makes ubiquitous. Adopting shadow lints therefore means either renaming through every process step (`bolt2`, `bolt3`, …) or per-line allows. Not adopted; the tripwire covers the shadowing leak at test time instead. (`unused_variables`, on by default, still catches the shadowed-and-never-used case at compile time — it even flagged two of this experiment's deliberate demos.)

> **Tripwire panics point at the Drop impl, not the leak site (R1/R5).** A fired tripwire reports `panicked at src/lib.rs:<line of Drop impl>`; the leak's location must be recovered from the failing test's name or `RUST_BACKTRACE=1`. Fine for unit-test granularity (R5's one-process-per-test keeps the search space small), poor for end-to-end flows. Mitigation: keep processes small and tests per-process, as R5 already requires.

---

## 5. Recommendation: **adapt**

Adopt the layered regime, with adjustments ("adapt" rather than "adopt" because no single tested mechanism suffices and one — shadow lints — should be rejected):

**Conventions for the real project**

1. Every resource type: `#[must_use = "<type> is a conserved resource: pass it on or hand it to a Consumer"]`. Crate-wide `#![deny(unused_must_use)]` and `#![deny(let_underscore_drop)]`.
2. Every *consumable* resource type gets the tripwire: a `Drop` impl that panics with `"resource leak: <Type> ..."` unless `thread::panicking()`, defused only by explicit consumption (in the real model, the R12 `Consumer` takes the role of `consume(self)`; `mem::forget` inside it is the single allowed site, macro-generate the boilerplate). The tripwire is worth its noise: it converts six of eight leak paths into ordinary test failures for ~10 lines per type, and R5 already demands the tests that make it bite.
3. CI runs clippy with `-D warnings` plus, enabled explicitly (they are restriction lints): `clippy::mem_forget`, `clippy::let_underscore_must_use`; and a `clippy.toml` disallowing `std::mem::drop`, `core::mem::drop`, and (belt-and-braces, no dedicated lint exists) `std::mem::ManuallyDrop::new` and `std::boxed::Box::leak` via `disallowed-methods`, each with a `reason` telling the modeller what to do instead — the reason string is the most modeller-readable diagnostic in the whole experiment.
4. Do **not** enable `clippy::shadow_*`: it fights the R2 rebinding idiom. Rely on the tripwire plus default `unused_variables` for shadowing.
5. Never write `let _ = <resource>` or underscore-prefixed resource bindings; the lints in (1) and (3) make the first an error, the second is convention only.
6. R5 test discipline is part of the conservation story: every process branch (especially early returns and error paths) needs a test, because branch coverage *is* leak coverage for path 7.

**Residual gaps, stated plainly**
- A resource bound, genuinely used, and then allowed to fall off the end of scope (or dropped by a struct `..` pattern, or on an untested early-return branch) is caught by nothing at compile time — only by the tripwire *when a test runs that path*.
- Leaks during panic unwinding are caught by nothing, by design: the alternative is process abort. In this project that is acceptable — a panicking model run has already failed — but "nothing is lost" formally does not hold on failure paths.
- `ManuallyDrop`, `Box::leak`, `Box::into_raw` and friends can defeat the tripwire like `mem::forget`; only `disallowed_methods` entries and review cover them.

Stable Rust verdict for the instructions' known-gap paragraph: confirmed and now bounded — affine types plus this convention set give **compile-time** protection for discard-by-statement, `let _`, forget and explicit drop, and **test-time** protection for everything else except unwinding. True linearity ("every resource must be consumed, checked by the compiler") is not expressible on stable Rust; no third approach beyond types-with-Drop and lints exists to try, so this is a bounded limitation, not a blocker.
