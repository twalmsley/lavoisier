# EXP-10 — Fallible processes (candidate R17): results

**Toolchain:** `rustc 1.98.1 (48a229cea 2026-09-01) (Homebrew)`, `cargo 1.98.1`, macOS (Darwin 24.6.0, Apple Silicon).
**Dependencies:** `model-core` by path (per the EXP-10..12 protocol update; `test-support` enabled only under `[dev-dependencies]`), `trybuild` as the one external dev-dependency.
**Compile time:** `cargo clean && time cargo build` three times: 1.49 s / 1.54 s / 1.51 s — **median 1.51 s** (model-core + this crate). A non-issue.
**Test suite:** 18 green — 6 unit, 5 flow, 4 leak-probe, 7 trybuild ui cases, 2 `compile_fail` doc-tests.

Layout: `src/model.rs` (the family: failure-state resources, outcome token, bundles, sinks, boundary, processes), `src/two_token.rs` (the rejected design probe), `src/flows.rs` (converging both-arms flow, bounded retry), `tests/{flows,leaks,trybuild}.rs`, `tests/ui/*`.

---

## 1. Verdicts per criterion

### C1. Failure-state resources via the kernel macros, with a scrap consumer and a repair process — **pass**

`ScrapPlate<const G: u64>` (`container_resource!`), `BrokenDrillBit` (`consumable_resource!`, tripwired) built by the macros with zero friction; the working→broken conversion is a `pub(crate)` `DrillBit::snap()` exactly in R9's per-step-conversion idiom. Both failure outputs have production-legal exits (F-035 honoured): `ScrapYard` (an unbounded `Next = Self` boundary sink for scrap, swarf **and** bits broken beyond repair) and `repair_bit(BrokenDrillBit, SpareParts) -> DrillBit`. Nothing new was needed from model-core. Evidence: `src/model.rs`; unit tests `failure_arm_conserves_the_same_inputs`, `repair_restores_the_working_state`.

### C2. The outcome-token design probe — **pass**; recommend ONE token type

Both designs were built end to end (`src/model.rs::DrillOutcome` vs `src/two_token.rs`):

- **One sealed token type, runtime-valued (`DrillOutcome`)** — a private enum wrapped in a sealed struct (the R1 "never a pub enum resource" rule), tripwired, minted only by boundary constructors (`outcome_success`/`outcome_failure`, both `/// Placeholder:`) and test-support fixtures, consumed by exactly one process, with a boundary exit (`return_outcome`) for untried trials. **A flow cannot read the token** — the kind is private — so the only way to learn the outcome is to run the process and handle the `Result`. Processes stay deterministic functions of their inputs; the variability is in the token's *value*, injected at the boundary (R12 spirit). Both outcomes of one process are testable by injecting either token.
- **Two distinct token types selected by the flow (`WillSucceed`/`WillFail`)** — expressible two ways, and both compile on stable: (a) two monomorphic process variants returning the bare bundles, and (b) one generic process over a sealed trait with a **const-generic GAT** output (`type Out<const …: u64, …>`, stable since GATs landed) — `drill_static<O: StaticOutcome, …>(…) -> O::Out<…>` builds and runs. The design's defect is semantic, not technical: **the outcome is part of the flow's static text.** There is no `Result`, no runtime branch, and nothing anywhere that forces a flow to handle an arm it did not choose — "fallible process" collapses back into two infallible processes, reintroducing per-flow "every process succeeds". A token-generic flow is no escape: its result is the opaque projection `O::Out<…>`, which it can only pass along, so the handling obligation evaporates rather than being enforced. Each impl also carries only its own branch's conservation assert, so the unchosen branch's split is never checked at any call site.

**Recommendation: one token type.** Two-token machinery is legitimate only as test scaffolding, and even there the single runtime token plus two boundary constructors does the same job with less machinery.

### C3. `drill_fallible(...) -> Result<OkBundle, FailBundle>` with per-branch conservation and the person returned in both arms — **pass with complications** (ergonomics)

```rust
pub fn drill_fallible<const SPEND: u64, const T_LEFT: u64, const BUDGET: u64,
                      const PLATE: u64, const P_LEFT: u64, const SW: u64,
                      const SCRAP: u64, const FSW: u64>(
    person: Person<BUDGET>, bit: DrillBit, plate: Plate<PLATE>, outcome: DrillOutcome,
) -> Result<DrillOk<T_LEFT, P_LEFT, SW, SPEND>, DrillFail<T_LEFT, SCRAP, FSW, SPEND>>
```

Two independent `const { assert!(…) }` blocks — `P_LEFT + SW == PLATE` (success) and `SCRAP + FSW == PLATE` (failure) — and **both fire at every instantiation**, so every call site proves both arms conserve even if the flow only ever realises one (F-033 extended to branches). The time draw happens before the branch, so the person comes back in both arms with the same `T_LEFT` — failure costs labour too, and that labour is recorded into `History` like any other (R16). Both branch-violation regressions are rustdoc `compile_fail` doc-tests (R4 policy; post-monomorphization per F-001 — verified: `cargo check --tests` passes the violation, `cargo build --tests` rejects it, verbatim in §2a).

The complication is const-parameter load: **eight const parameters, all stated by the caller** — a fallible process carries the splits of *both* branches (F-022/F-030 roughly double). Not a correctness problem (a wrong number is the E0080 of §2a), but every call reads `drill_fallible::<1000, 4000, 5000, 450, 440, 10, 430, 20>(…)`.

### C4. Flows handling both arms + bounded repair-and-retry — **pass**, with two structural lessons

`flows::drill_one_plate_handling_both_arms` handles and **converges** both arms: scrap/swarf to the yard, labour recorded, and — the key move — the broken bit **repaired on the spot**, so both arms return the same `DrillBit` type and the same `Person<4000>`. Per-arm differences come back value-level as `Option<DrilledPlate<440>>`/`Option<SpareParts>`. Lesson 1: **convergence requires equal types in both arms**; reusable resources converge only when the failure arm restores the same state, and magnitudes must match (both arms spend the same time here — had they differed, the person types would differ and no single return type would exist). Measured caveat: `unused_must_use` fires *per tuple element* (with each resource's own R1 message — excellent) but does **not** look inside `Option<…>` (§2f), so Option-wrapped resources keep only tripwire protection against discard; prefer a named `#[must_use]` outcome grouping.

`flows::drill_with_one_retry` is the bounded rework composition. Lesson 2: **the bound is the provisioning** — a retry consumes a provisioned reserve blank, reserve trial token and repair kit, so the retry count is fixed by the resources passed in; unbounded retry is inexpressible without unbounded inputs (strict conservation working as designed). Its three paths spend different time (1000 vs 2000 ms) and different reserves, so there is **no single converged return type**; the flow returns a `#[must_use]` three-variant outcome grouping (`RetryOutcome::{FirstTry, Retried, GaveUp}`). First-try success returns the unused reserves, which the caller must re-account (`ToolStores`, `return_outcome`) — over-provisioning becomes an explicit, typed cost. All three paths are tested (R5: branch coverage is leak coverage).

Note on groupings: `DrillOk`/`DrillFail` (pub-field structs) and `RetryOutcome` (a pub enum over resources) are **groupings, not resources** — building one requires already holding the sealed resources, so they cannot mint; the R1 sealing rules (incl. "never a pub enum resource") apply to resources, not to groupings. Deliberately **no `Debug`** on any bundle (see C5).

### C5. Leak-surface probes — **pass**; the matrix is §3, the one hole is F-002's

Headline: **`.unwrap()` and `.expect()` do not compile.** `Result::unwrap` requires `E: Debug`; the failure bundle holds sealed resources (no `Debug` anywhere) and derives none, so the panicking shortcut past the failure arm is closed at type-check time (§2b) — an unplanned bonus of the sealing conventions, worth making deliberate. The runtime unwrap-*equivalent* (an explicit panic while holding the bundle) remains F-002's hole: during unwinding every tripwire stands down, and the five resources in the bundle vanish silently (§2g — the probe test passing at all, with no SIGABRT and no "resource leak:" text, is the evidence of silence). Everything between those ends is caught: discard and `let _` at compile time by the lint pair, misuse of the `Result` as the bundle at type-check time, and bound-but-unmatched results / forgetful match arms at test time by the tripwires.

---

## 2. Verbatim compiler/test output, with readability judgements

### (a) Per-branch conservation violation — E0080 at monomorphization (temporary probe, kept as `compile_fail` doc-tests)

`cargo check --tests`: **passes** (`Finished `dev` profile … in 4.63s`). `cargo build --tests`:

```
error[E0080]: evaluation panicked: failure-branch mass conservation violated in drill_fallible (R3, candidate R17): the scrap plate plus its swarf must sum exactly to the plate blank
   --> /usr/local/Cellar/rust/1.98.1/lib/rustlib/src/rust/library/core/src/panic.rs:62:9
    |
 62 |           $crate::panicking::panic_fmt($crate::const_format_args!($($t)+));
    |           ^^^^^^^^…^^^^^ evaluation of `exp10_failure_modes::model::processes::drill_fallible::<1000, 4000, 5000, 450, 440, 10, 400, 20>::{constant#1}` failed here
…
note: the above error was encountered while instantiating `fn drill_fallible::<1000, 4000, 5000, 450, 440, 10, 400, 20>`
  --> tests/e0080probe.rs:8:13
```

*Judgement:* good — the custom message leads and names the branch; the instantiation note carries the decimal magnitudes (F-027) and the caller's line. The primary span in `core/src/panic.rs` is the known F-001 noise. A modeller reads "failure-branch mass conservation violated … scrap plus swarf must sum to the blank" directly. Checked **both asserts fire per instantiation**: a flow that only ever runs the success branch is still rejected for a bad failure split.

### (b) `.unwrap()` — compile error because bundles have no `Debug` (`tests/ui/unwrap_needs_debug.stderr`)

```
error[E0277]: `DrillFail<4000, 430, 20, 1000>` doesn't implement `Debug`
  --> tests/ui/unwrap_needs_debug.rs:17:22
   |
17 |     let _ok = result.unwrap();
   |                      ^^^^^^ the trait `Debug` is not implemented for `DrillFail<4000, 430, 20, 1000>`
   |
note: required by a bound in `Result::<T, E>::unwrap`
```

*Judgement:* fair — correct and final, but phrased as a formatting problem; a modeller needs the translation "you may not panic past the failure arm — match and account for it". One for the error-reading guide.

### (c) `Result` discarded / `let _ =` — the lint pair, each suggesting the next leak (F-007 confirmed for the Result shape)

`tests/ui/result_discarded.stderr` (deny `unused_must_use`):

```
error: unused `Result` that must be used
   = note: this `Result` may be an `Err` variant, which should be handled
help: use `let _ = ...` to ignore the resulting value
```

`tests/ui/let_underscore_result.stderr` (deny `let_underscore_drop`):

```
error: non-binding let on a type that has a destructor
help: consider binding to an unused variable to avoid immediately dropping the value
help: consider immediately dropping the value
   |
13 ~     drop(drill_fallible::<…>(
```

*Judgement:* the primary lines are excellent ("may be an `Err` variant, which should be handled" is practically the R17 rule). The fix-its are F-007 verbatim: `unused_must_use` recommends `let _ =` (the next leak), `let_underscore_drop` recommends `drop(…)` (a banned method) or an unused binding (a test-time tripwire). The lint set only works as a set, and the modeller guide's "ignore these fix-its" applies unchanged to fallible processes.

### (d) `Result` used as if it were the success bundle (`tests/ui/result_is_not_the_bundle.stderr`)

```
error[E0308]: mismatched types
   = note: expected struct `DrillOk<4000, 440, 10, 1000>`
                found enum `Result<DrillOk<4000, 440, 10, 1000>, DrillFail<4000, 430, 20, 1000>>`
help: consider using `Result::expect` to unwrap the `Result<…>` value, panicking if the value is a `Result::Err`
   |
18 |         ).expect("REASON");
```

*Judgement:* the error itself is the core R17 enforcement and reads perfectly ("you have an unhandled failure arm"). The fix-it is a **new F-007 family member**: rustc recommends `.expect("REASON")` — the panic path — which here does not even compile (no `Debug`, (b)), and would be a conservation violation if it did. Add to the error-reading guide.

### (e) Failure state cannot continue the success flow

Single-`Consumer` sink → E0308 (`tests/ui/scrap_cannot_be_shipped.stderr`):

```
error[E0308]: mismatched types
12 |     let _customer = send_to(new_customer(), scrap);
   |                     -------                 ^^^^^ expected `DrilledPlate<_>`, found `ScrapPlate<430>`
```

Multi-`Consumer` sink → the trait-bound path with model-core's `on_unimplemented` (`tests/ui/scrap_cannot_return_to_stores.stderr`):

```
error[E0277]: `ToolStores` cannot consume a `ScrapPlate<430>`: it is full, or it does not accept this kind of resource
   |                   ------- ^^^^^^^^^^^^^^^^^ this consumer has no space left for this resource
help: `ToolStores` implements trait `Consumer<In>`
   | impl Consumer<SpareParts> for ToolStores {
   | impl<const G: u64> Consumer<Plate<G>> for ToolStores {
```

*Judgement:* both are modeller-grade. The E0308 reads directly as "these are scrap, not drilled plates" (F-023 extended to failure states); the E0277 adds the sink's actual interface in the help (F-029's observation), and is duplicated once per F-009. Which shape appears depends on how many `Consumer` impls the sink has — single-impl sinks get type inference and E0308, multi-impl sinks get the `on_unimplemented` message.

### (f) `unused_must_use` and the converged flow's return (temporary probe)

Discarding the whole `(Person, DrillBit, Option<DrilledPlate>, Option<SpareParts>, ScrapYard, History)`:

```
error: unused `Person` in tuple element 0 that must be used
   = note: Person is a reusable resource: pass them on or return them to the caller
error: unused `DrillBit` in tuple element 1 that must be used
error: unused `ScrapYard` in tuple element 4 that must be used
error: unused `History` in tuple element 5 that must be used
```

*Judgement:* per-element errors with each resource's own `must_use` message are excellent — **but elements 2 and 3 are missing**: the lint does not look inside `Option<…>`, so an `Option`-wrapped resource loses the compile-time discard layer (tripwire still covers a dropped `Some`). Design consequence in §4.

### (g) The test-time and not-caught rows (`tests/leaks.rs`, all green)

```
test bound_but_unmatched_result_trips_the_tripwire - should panic ... ok
      (panic: resource leak: DrilledPlate<440> dropped without being consumed (R1 conservation))
test match_arm_forgetting_fields_trips_the_tripwire - should panic ... ok
      (panic: resource leak: Swarf<10> dropped without being consumed (R1 conservation))
test panic_while_holding_the_failure_bundle_leaks_silently - should panic ... ok
      (panic: drilling failed and the flow gave up — and nothing else)
```

*Judgement:* the tripwire messages name type and decimal magnitude (F-027) but the `Drop` impl's line, not the leak site (F-008, unchanged). The third test is the F-002 hole demonstrated for the `Result` shape: it exits cleanly with **only** the flow's own panic — no abort, no leak report — although five resources (person, broken bit, scrap, swarf, labour) were lost during unwinding. Drop-order detail, measured with a side probe: fields skipped by a `..` pattern are a **partial move** — they stay in the destructured binding's storage and drop at the end of its scope, in declaration order, *after* every bound field — so the tripwire fires at the end of the match arm, naming the first skipped tripwired field.

---

## 3. The leak-surface matrix for `Result`-shaped processes

| # | Leak path | Caught by | When |
|---|---|---|---|
| 1 | `.unwrap()` / `.expect()` on the result | bundles have no `Debug` → E0277/E0599 | **compile (type-check)** |
| 2 | result discarded in statement position | std `#[must_use]` on `Result` + `deny(unused_must_use)` | **compile (lint)** |
| 3 | `let _ = <result>` | `deny(let_underscore_drop)` (bundles have drop glue) | **compile (lint)** |
| 4 | result assigned/passed as if it were the Ok bundle | E0308 | **compile (type-check)** |
| 5 | failure state continues the success flow (`ScrapPlate` shipped) | E0308, or E0277 + `on_unimplemented` (multi-impl sink) | **compile (type-check)** |
| 6 | result bound, never matched, dropped at scope end | tripwire on the first bundle field | test time |
| 7 | match arm forgets fields (`..`, or bind-then-drop) | tripwire on the first skipped field, at arm end | test time |
| 8 | panic while holding a bundle (the unwrap-equivalent) | **nothing** — tripwires stand down during unwinding | not caught (F-002) |
| 9 | outcome token minted/forged downstream | E0451 private fields + private type | **compile** |
| 10 | outcome token provisioned but never run or returned | tripwire on `DrillOutcome` | test time |

Rows 2 and 3 carry misleading fix-its (each suggests the next row or a banned method — F-007); row 4's fix-it suggests `.expect()` (blocked by row 1). `Option<resource>` is **not** covered by row 2's lint (§2f): only rows 6/7-style tripwires protect it.

---

## 4. Candidate FINDINGS entries

### F-EXP10-1 — A fallible process needs ONE runtime-valued outcome token; two token types statically collapse fallibility

**What couldn't be expressed (with two token types):** compiler-forced failure handling. Two sealed token types selected by the flow are fully expressible on stable — as two monomorphic process variants, or as one generic process whose output is a const-generic GAT (`type Out<const …: u64, …>`, `exp10 src/two_token.rs::StaticOutcome`) — but the outcome becomes part of the flow's static text: no `Result`, no runtime branch, and no site anywhere is forced to handle the arm the flow did not pick; each impl checks only its own branch's conservation. A token-generic flow returns the opaque projection `O::Out<…>` it can only pass along.
**Workaround adopted (the design):** one sealed token type (`DrillOutcome`) wrapping a **private enum** (R1: never a pub enum resource), tripwired, minted only by boundary constructors (placeholders) and test-support fixtures, unreadable by flows, consumed by exactly one process, with a boundary exit for untried tokens. Processes stay deterministic (the token's value is an input); both outcomes of one process are testable; `Result` + the lint/tripwire layers force handling.
**Evidence:** EXP-10 (`src/model.rs::DrillOutcome`, `src/two_token.rs`, `tests/ui/outcome_cannot_be_minted.stderr`).

### F-EXP10-2 — Per-branch conservation is independent const asserts, both checked at every instantiation; the cost is doubled const-parameter load

**What worked:** `drill_fallible` carries a success-split assert and a failure-split assert; **both fire at every instantiation** (post-monomorphization, F-001 unchanged), so a call site that only ever realises the success branch is still rejected for an unbalanced failure branch — the model proves both arms conserve everywhere. Each violation's E0080 leads with its own branch-naming message and decimal magnitudes (F-027).
**What it cost:** the caller states **both** branches' splits: eight const parameters on one process (`drill_fallible::<1000, 4000, 5000, 450, 440, 10, 430, 20>`), extending F-022/F-030; drawing shared inputs (time) *before* the branch keeps the reusable's return type equal in both arms.
**Evidence:** EXP-10 (`src/model.rs::drill_fallible`, the two `compile_fail` doc-tests, RESULTS §2a).

### F-EXP10-3 — Outcome bundles without `Debug` make `.unwrap()`/`.expect()` a compile error; rustc's fix-its recommend the panic path anyway

**What worked (unplanned):** `Result::unwrap`/`expect` require `E: Debug`; outcome bundles hold sealed resources (which implement no `Debug`) and derive none, so the panicking shortcut past the failure arm **does not compile** (E0277). Keep `Debug` off outcome bundles deliberately.
**What it cost / residual:** the error is phrased as a formatting problem (needs an error-reading-guide entry), and the F-007 fix-it family grows: E0308 on a mishandled `Result` suggests `.expect("REASON")`, `unused_must_use` suggests `let _ =`, `let_underscore_drop` suggests `drop(…)`. The runtime unwrap-equivalent — panicking while holding a bundle — remains F-002's hole: unwinding defuses every tripwire and the bundle's resources are lost silently.
**Evidence:** EXP-10 (`tests/ui/unwrap_needs_debug.stderr`, `tests/ui/result_is_not_the_bundle.stderr`, `tests/leaks.rs::panic_while_holding_the_failure_bundle_leaks_silently`).

### F-EXP10-4 — Converging after a fallible step requires equal types in both arms; `unused_must_use` does not look inside `Option`, so one-arm outputs need named `#[must_use]` groupings

**What couldn't be expressed:** a single return type for a flow whose arms produce different resources or different magnitudes. Reusable resources converge only when the failure arm restores the same state at the same magnitude (repair-on-the-spot; equal time spent in both arms). One-arm products can come back as `Option<resource>`, but — measured — `unused_must_use` fires per **tuple element** (with each resource's own message) and **not** through `Option<…>`, leaving Option-wrapped resources tripwire-only against discard. Paths that spend different amounts (a retried flow: 1000 vs 2000 ms) have no common type at all.
**Workaround adopted:** converge what can be converged (repair in the failure arm), return per-path outcomes as a named `#[must_use]` grouping — a pub struct per arm, or a pub enum with one variant per path (`RetryOutcome`). Groupings (pub fields/variants over sealed resources) are legal: they cannot mint, so the R1 sealing rules — including "never a pub enum" — apply to resources, not groupings.
**Evidence:** EXP-10 (`src/flows.rs`, RESULTS §2f).

### F-EXP10-5 — Bounded rework is bounded by provisioning; early success returns reserves that must be re-accounted

**What worked:** under strict conservation a retry consumes provisioned reserves (a second blank, a second trial token, a repair kit), so the rework bound is the resources passed in — **unbounded retry is inexpressible without unbounded inputs**, which is the honest statement of real rework. The cost surfaces symmetrically: when the first attempt succeeds, the unused reserves come back and must be re-accounted at the boundary (stores take back the blank and the kit; the environment takes back the untried token via a boundary exit function). Failure's labour is recorded in the `History` like any other consumption — the execution record shows what failure cost (R16).
**Evidence:** EXP-10 (`src/flows.rs::drill_with_one_retry`, `tests/flows.rs`, all three paths tested).

### F-EXP10-6 — `..`-skipped fields are a partial move: they drop at the end of the destructured binding's scope, after every bound field

**What was measured:** in `let Bundle { a, c, .. } = bundle;` the skipped fields do **not** drop at the destructuring statement; they stay in `bundle`'s storage and drop where `bundle` would have dropped — end of its scope, declaration order, after all bound (and later-declared) locals. A tripwire on a forgotten field therefore fires at the end of the match arm, and if a *bound* tripwired resource is also dropped, the bound one fires first.
**What it cost:** none — one more "the tripwire reports a place, not the leak" datum for the modeller guide (F-008).
**Evidence:** EXP-10 (`tests/leaks.rs::match_arm_forgetting_fields_trips_the_tripwire` doc comment; side probe transcript in the experiment log).

---

## 5. Recommendation: **adopt** — proposed R17 wording

Adopt the single-runtime-token, `Result`-of-bundles design. Proposed text, written to slot into `instructions.md` after R16 (F-references to be renumbered at consolidation):

> ### R17. Fallible processes
> - A process that can fail returns **`Result<OkBundle, FailBundle>`**: success and failure are *both* ordinary conserving outcomes (R1). **Both branches conserve the same inputs** — a failure output (a scrapped part, a snapped tool) is a product, not a disappearance. Every reusable resource comes back in both arms; inputs spent in both arms (a person's time) are drawn **before** the branch, so the reusable returns at the same magnitude either way.
> - **One type per failure state** (extends R9): `ScrapPlate` is not `DrilledPlate`, a `BrokenDrillBit` is not a `DrillBit` — so a failure output continuing the success flow is a compile error. Every failure-state resource ships with at least one production-legal exit: a repair process back to the working state, and/or a boundary scrap consumer (the F-035 rule applies to failure outputs too).
> - **Variability enters only at the boundary, as a sealed outcome token** — one token type per fallible process kind, **never** two token types selected by the flow (two types make the outcome part of the flow's static text: no `Result`, and no code is ever forced to handle the arm it did not pick). The token wraps a private enum (R1 sealing), is tripwired, is created only by boundary constructors (always `/// Placeholder:` until calibrated — see the validation open question) and test-support fixtures, and is consumed by exactly one process; an untried token is returned to the environment through a boundary exit function. Flows cannot read a token: the only way to learn the outcome is to run the process and handle the `Result`.
> - **Per-branch conservation is one const assert per branch**, each with a branch-naming message, and **both are checked at every instantiation** (the R4 post-monomorphization caveat applies): callers state the success split *and* the failure split even though only one branch runs. Expect a fallible process to carry roughly twice the const parameters of its infallible form (F-022/F-030).
> - **Outcome bundles are groupings, not resources**: one `#[must_use]` struct per arm with public resource fields, destructured by the handling arm. Bundles implement **no `Debug`** — deliberately, since that makes `.unwrap()`/`.expect()` a compile error and forces a `match`. A grouping may also be a `pub` enum over bundles (one variant per flow path); groupings cannot mint resources, so the R1 sealing rules apply to resources, not groupings. Prefer named `#[must_use]` groupings over bare `Option<resource>` returns: `unused_must_use` sees through tuples but **not** through `Option`, which leaves only the tripwire.
> - **Flows handle both arms.** Enforcement is layered and measured: at compile time, std's `must_use` on `Result` (discarded result), `deny(let_underscore_drop)` (`let _ =`), E0308 (result used as the bundle; failure state in a success-flow position, with `on_unimplemented` phrasing on multi-impl sinks), and the missing `Debug` (unwrap); at test time, the tripwires (a bound-but-unmatched result; a match arm that forgets fields — `..`-skipped fields drop at the end of the destructured binding's scope). The residual hole is F-002's: a panic while holding a bundle leaks its resources silently under unwinding. Ignore the fix-its on this path — `let _ = …`, `drop(…)`, and `.expect("REASON")` are each a conservation violation (F-007).
> - **Rework is bounded by provisioning.** A retry consumes provisioned reserves (another blank, another trial token, a repair kit), so the rework bound is the resources passed in — unbounded retry is inexpressible, truthfully. Retry paths that spend different amounts have different types, so a retried flow returns a `#[must_use]` outcome enum (one variant per path); on early success the unused reserves are re-accounted at the boundary (returned to stores / the environment). Failure's labour is recorded in the `History` like any other consumption (R16).

Supporting changes proposed alongside R17:
1. **Error-reading guide additions:** "`DrillFail<…>` doesn't implement `Debug`" → "you may not panic past the failure arm: match the `Result` and account for both bundles"; "consider using `Result::expect`" → ignore (F-007 family).
2. **R5 note:** for a fallible process, "every branch" means at minimum one test per `Result` arm per flow path (EXP-10's retry flow needed three).
3. No model-core changes required — the kernel macros, `Person`/`Labour`/`draw_time`, `Consumer`, and `History` served the whole experiment unmodified. A future `outcome_token!` kernel macro (generating the sealed enum-wrapper token, its tripwire, boundary constructors and fixtures, ~70 hand-written lines here) is worthwhile but optional.
