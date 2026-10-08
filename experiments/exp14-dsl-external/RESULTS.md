# EXP-14: A DSL as an external notation — RESULTS

**Toolchain:** `rustc 1.98.1 (48a229cea 2026-09-01) (Homebrew)`, stable.
**Dependencies:** none anywhere — the compiler (`lavc`) is std-only (the
diagram-gen house style); the generated crates depend only on `model-core` by
path. trybuild is permitted but was not needed (see criterion b: the
violations the generated code must catch are post-monomorphization, which
trybuild cannot see, F-003 — and the notation layer is tested by running
`lavc`, not by compiling Rust).

**What was built** (everything inside this directory):

| Piece | Where | Size |
|---|---|---|
| The `.lav` notation (CS-1 slice: water/electricity containers, mains/grid draws, `fill_kettle`, `boil` + energy assert, REQ-001 incl. R10 rule 8, one flow; `serve` as the minimal requirement-bound consumer) | `model.lav` | 113 lines (80 non-blank/non-comment) |
| The compiler: parse → check → emit | `src/{ast,parse,check,emit,lib,main}.rs`, bin `lavc` | 3 235 LOC |
| The generated model crate (buildable, tested) | `generated/cs1-slice/` | 431 LOC incl. Cargo.toml |
| The three canonical violations, authored in the notation | `violations/v-{energy,overdraw,requirement}.lav` → `generated/v-*/` | 1-line diffs from `model.lav` |
| Notation-level error fixtures | `errors/*.lav` | 4 files |
| Compiler test suite (9 tests: fixtures, determinism, breadcrumbs) | `tests/compiler.rs` | 128 LOC |
| Real-tool runs over the generated crate | `generated/cs1-slice/trace.sh` (copy of `model/trace.sh`), `lint-root/` (modellint + docgen output) | — |

Usage: `cargo run --bin lavc -- model.lav --out generated/cs1-slice --model-core ../../../../model/model-core`

---

## Verdicts

### 1. Minimal declarative notation + compiler emitting a complete, buildable crate — **pass**

The notation covers, declaratively: containers with units and extra const
parameters, reusables, unbounded boundary sources (R15 draw style) and sinks
(`Next = Self`), characteristics with carried consts and one permit-gated
extraction (the cs1 F-054 pattern), requirements (R10 incl. the rule-8
`on_unimplemented` message and the F-020 `example` alias), processes with
`in`/`out`/`balance` lines, and flows (`person`/`history`/`new`/`take`/
`spend`/`do`/`send`/`end`). The generated crate uses the kernel macros
(`container_resource!`, `reusable_resource!`, `requirement!`, `satisfies!`),
carries the mandatory crate attributes, builds with **zero warnings**, and its
generated flow integration test passes (`cargo test`: 1 passed). Caller-stated
output magnitudes (F-022) appear in the notation as `with C = N` on `do`
lines; the compiler threads every magnitude (including the person's running
budget) and emits fully-decimal turbofish —
`boil::<1500, 550_000, 500_000, 50_000>(filled, energy)` — so diagnostics
print real numbers (F-027 preserved by construction).

One emitter gotcha found: the kernel macros expand a fixture constructor
behind the *invoking* crate's `test-support` feature, so the generated
Cargo.toml must declare `test-support = []` or every resource invocation
raises an unexpected-`cfg` warning (9 of them). Fixed in the emitter.

### 2. Deterministic, idempotent, breadcrumbed generation — **pass**

Two runs are byte-identical (`diff -r` clean; also pinned by the
`generation_is_deterministic` test), and regenerating over existing output
changes nothing. Output is a pure function of the `.lav` text: no timestamps,
`BTreeMap` everywhere, declaration-order emission. Every generated item
carries `// lav: model.lav:<line> (<item>)` above its doc block (where
trace.sh ignores it — tags stay in `///`), and *flow statements and balance
asserts carry the breadcrumb on the very line a Rust diagnostic will cite*
(see criterion b: the breadcrumb is visible inside the rendered error).

### 3a. Notation-level errors (the layer the tool owns) — **pass**

All reference/structure/flow-discipline errors come with file:line:col, the
source line with a caret, modeller phrasing citing the R-rule, and
did-you-mean/`help:` lines. Error recovery (a poisoned value kind) keeps one
mistake from cascading. The three showcase transcripts, verbatim:

**(1) Unknown resource, with suggestion** (`errors/unknown-resource.lav`):

```
error: no resource called `ColdWatr` is declared - a process input must name a declared `container` or `reusable`
  --> unknown-resource.lav:72:13
   |
72 |   in water: ColdWatr<G>
   |             ^
  = help: a resource called `ColdWater` is declared - did you mean that?
```

**(2) A balance naming a magnitude the process does not carry**
(`errors/unknown-magnitude.lav`):

```
error: the balance for `boil` names `EMBODIED_X`, but no input or output of `boil` carries that magnitude
  --> unknown-magnitude.lav:82:73
   |
82 |   balance joules "energy conservation violated in boil (R15)": DRAW_J = EMBODIED_X + HEAT_J
   |                                                                         ^
  = help: a balance may only sum magnitudes that enter or leave the process (R3); `boil` carries: G, DRAW_J, EMBODIED_J, HEAT_J
```

**(3) The notation-level "use of moved value"** (`errors/moved-resource.lav`)
— R2's exclusivity in the model's own vocabulary, pre-empting the
borrow-checker phrasing F-025 had to apologise for:

```
error: `water` was already used by `fill_kettle` at line 107 - a resource can be in only one process at a time (R2)
  --> moved-resource.lav:108:35
    |
108 |   do boiling, heat = boil(filled, water) with EMBODIED_J = 500000, HEAT_J = 50000
    |                                   ^
  = help: use what that process returned instead; every process returns its reusable inputs (R2)
```

Bonus (the R1 leak check, `errors/unaccounted.lav` — a `send heat to air`
line deleted):

```
error: `heat` is never accounted for: at flow end every value must have reached a sink, a process, or this `end` line (R1)
  --> unaccounted.lav:112:1
  = help: nothing is silently lost: send it to a sink, pass it to a process, or list it in `end`
```

Judgement: these are **better than the Rust equivalents** — the moved-value
error names both processes and the rule; the leak error fires at compile
(generation) time where Rust needs the test-time tripwire (F-002/F-032).

### 3b. The three canonical Rust violations authored in the notation — **pass**

The compiler deliberately does **not** evaluate balance arithmetic, budgets
or requirement satisfaction (the stated division of labour), so a violating
`.lav` generates a violating crate and layer (b) can be measured. All three
fire exactly as in hand-written code; `cargo check` sees only the requirement
one (v-energy PASSES check, v-overdraw PASSES check, v-requirement FAILS) —
the F-001 split, unchanged by generation.

**Energy assert** (`v-energy.lav`: `HEAT_J = 60000`; plain `cargo build`,
flows are production code in `src/flows.rs`):

```
error[E0080]: evaluation panicked: energy conservation violated in boil (R15): DRAW_J must equal EMBODIED_J + HEAT_J - the amounts entering boil must sum exactly to the amounts leaving it
   --> src/resources.rs:254:17
note: the above error was encountered while instantiating `fn boil::<1500, 550000, 500000, 60000>`
  --> src/flows.rs:39:27
   |
39 |     let (boiling, heat) = boil::<1500, 550_000, 500_000, 60_000>(filled, energy); // lav: v-energy.lav:108
   |                           ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

The primary span lands on the generated assert (whose preceding line is
`// lav: v-energy.lav:82 (balance joules)`), and the instantiation note's
rendered snippet **shows the `.lav` breadcrumb inline** — the modeller is
told the notation line without opening the generated file. Judgement:
navigability is as good as hand-written, plus the breadcrumb.

**Requirement bound** (`v-requirement.lav`: the flow serves the *filled*
kettle; type-check time, editor-visible):

```
error[E0277]: this kettle may not be poured: `FilledKettle<1500>` is not at the boil (REQ-001)
   --> src/flows.rs:38:48
    |
 38 |     let (kettle, hot) = serve::<1500, 500_000, _>(filled); // lav: v-requirement.lav:106
    |                                                ^ REQ-001: water must be poured at the boil
    = note: boiling is a process state (R9): `boil` turns a `FilledKettle` into a `BoilingKettle` - only that state can be poured
help: the trait `Boiling` is implemented for `BoilingKettle<WATER_G, V>`
```

The generated R10-rule-8 message leads, the label and note survive verbatim
from the `.lav` `error`/`label`/`note` attributes, and rustc's impl list even
points at the state that would satisfy it. Judgement: **identical to the best
requirement error measured in the project (F-044)** — nothing was lost to
generation, because the generated code is ordinary source, not a macro
expansion.

**Overdraw** (`v-overdraw.lav`: budget 40 000, spends 30 000 + 15 000; the
compiler tracks the running budget and emits the overdraw as
`draw_time::<15_000, 0, 10_000>` — saturating LEFT, so the canonical R15
shape is reproduced):

```
error[E0080]: evaluation panicked: time budget violated in draw_time (R15): SPEND + LEFT must equal BUDGET - is more time being spent than the person has left?
   --> .../library/core/src/panic.rs:62:9
   ::: .../model/model-core/src/common.rs:246:13
note: the above error was encountered while instantiating `fn draw_time::<15000, 0, 10000>`
  --> src/flows.rs:41:23
   |
41 |     let (labour, p) = draw_time::<15_000, 0, 10_000>(p); // lav: v-overdraw.lav:110 (spend on serve)
   |                       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

The cross-crate primary span lands in `core/src/panic.rs` exactly as the
error-reading guide documents (F-001), and the instantiation note again
renders the `.lav` breadcrumb — including *which spend on which process*.
Judgement: same quality as hand-written, breadcrumb on top.

### 4. Notation LOC vs generated LOC vs hand-written control — **pass**

| Layer | LOC |
|---|---|
| `.lav` notation | **113** raw, 80 non-blank/non-comment |
| Generated crate | **431** (resources 281, flows 46, characteristics 29, lib 23, requirements 17, test 15, Cargo.toml 20) |
| Hand-written control (the same slice counted out of cs1-pot-of-tea by item regions: the six resources, two sources + draws, two sinks, permit, Boiling, one requirement, fill_kettle/boil/serve-equivalent, boundary fns, lib/doc shares, one flow test) | **≈ 455** |
| The compiler | 3 235 (one-off) |

So the notation is **~4× denser than the Rust it replaces**, and the
generated crate is the same size and style as the hand-written control (the
emitter reproduces the cs1 house style, docs and all). The one-off compiler
costs ~7× the hand-written control — it amortises only if several models (or
many iterations of one) are written in the notation. Timings: clean build of
the generated crate 1.47 s median of 3 (incl. model-core), incremental
0.21 s; `lavc` itself regenerates in < 0.3 s; the compiler builds in 2.6 s
median of 3 (clean).

### 5. R-coverage — **pass with complications** (the table IS the complication)

| R | Covered by the notation? |
|---|---|
| R1 | ✓ sealing + conservation regime via the kernel macros; crate attrs; private-field sinks; permit-gated extractions; **plus** a notation-level leak check at flow end (stronger than Rust: compile-time, not tripwire-time) |
| R2 | ✓ by-value signatures; person pass-through enforced; **plus** the notation-level moved-value check |
| R3 | ✓ `balance` → `const` asserts (arithmetic truth deliberately left to Rust — see Adapt) |
| R4 | partial — violations are expressible and fire (above), but compile-fail *regression tests* (trybuild/`compile_fail` doc-tests) are not generated |
| R5 | partial — one integration test per flow is generated (passes, `Verifies:`-tagged); per-process unit tests are not |
| R6 | ✓ minimal: characteristics with carried consts + one extraction; no marker-only or parameterized-catalogue (R6 fixed-roles) support |
| R7 | ✓ integer magnitudes in base units; unit strings on containers |
| R8 | ✓ std-only compiler; generated code depends only on model-core |
| R9 | ✓ flows are connection compositions; a second valid order is a second `flow` block (same as the hand-written control's two tests) |
| R10 | ✓ full: `requirement!` with rule-8 message, `Satisfies:` + `satisfies!` via `example`, `Verifies:` on flow tests, one-line signatures — **the real `trace.sh` runs green over the output** (report below) |
| R11 | partial — reuses model-core `Person`/`History`; declares no new common types |
| R12 | partial — unbounded sources/sinks only; **no finite Peano suppliers/consumers, no `SupplyN`, no contents-keeping bins (F-016/F-034/F-039)** |
| R13 | ✗ discrete items not expressible |
| R15 | ✓ draws, waste outputs, time budgets (`spend` = the F-048 adjacent draw) |
| R16 | partial — one `history` per flow, every spend recorded with the process name; no merge/branches |
| R17/R18/R19 | ✗ not expressible (fallible processes, qualifications, money) |
| R20 | ✓ **the real `docgen` produced 11 process documents over the generated crate with zero WARN comments**, including observed instantiations recovered from the generated flow |
| R21 | ✓ **the real `modellint` reports 0 ERROR, 0 WARN** (13 INFO: 12 placeholders — all declared in the notation — and one thin-verify) |

The uncovered rows are the finding: the notation expresses the
continuous-resource core beautifully, and each further R-feature means
growing the notation (and its checker, and its emitter) toward the
complexity of the language it fronts.

### 6. Regeneration vs hand-edit — **pass** (discipline proposed, mechanism already emitted)

Every generated file opens with
`GENERATED by lavc - DO NOT EDIT. Edit the notation and regenerate (the
committed crate is gated by regenerate-and-diff, so hand edits fail CI).`

Proposed discipline (the R20/R21 staleness-gate pattern, reused unchanged):

1. **The `.lav` file is source; the generated crate is committed output.**
   CI regenerates into a temp dir and diffs against the committed crate —
   byte-identical generation (criterion 2) makes the gate exact; any hand
   edit fails it. (`lavc` is fast enough to run on every CI pass: < 0.3 s.)
2. **Hand-written code goes beside, not inside:** anything the notation
   cannot express (an R13 supplier, an R17 fallible process) lives in a
   *separate hand-maintained crate or module* depending on the generated one
   — never in a generated file. The generated crate is a normal upstream
   dependency (the R1/F-055-ext-7 multi-crate layout already supports this).
3. **Promotion is one-way and explicit:** if a model outgrows the notation,
   delete the `.lav` and the headers in one commit; the crate becomes
   hand-maintained and the gate stops applying. There is no "edit and keep
   regenerating" middle state — merge-on-regenerate is exactly the
   silent-corruption surface R20 exists to prevent.

### 7. Tool compatibility / greppable-discipline survival — **pass**

All three real tools were run over the generated crate (via a copied
`trace.sh` and a staged `lint-root/model/cs1-slice`, tools built with
`CARGO_TARGET_DIR` inside this directory so nothing outside was written):

* **trace.sh: exit 0, no warnings.** REQ-001 defined at
  `src/requirements.rs:13`; satisfied by `KettleAtTheBoil` [Satisfies tag],
  `serve` [Satisfies tag + bound use]; verified by
  `flow_make_tea_accounts_for_everything`.
* **modellint: 0 ERROR, 0 ERROR-KNOWN, 0 WARN, 13 INFO** (I-PLACEHOLDER ×12,
  I-THIN-VERIFY ×1 — both honest properties of the model, not of the
  generation).
* **docgen: 11 process documents, zero WARN comments**, flow traced from the
  generated `src/flows.rs` `pub fn`, balances restated with observed
  instantiations (`550000 == 500000 + 50000 → holds`).

Grep spot-checks: `/// REQ-001:` first line literal at the `requirement!`
invocation site; `Satisfies:` tags only on items *outside* macro invocations
(the `example` alias + the bound-carrying process) each backed by
`satisfies!`; `Verifies:` on the generated test; the requirement bound on the
one-line `fn` signature; 12 `Placeholder:` doc lines.

---

## Candidate FINDINGS entries

### F-EXP14-1 — An external notation preserves the curated error surface completely; breadcrumbs ride the diagnostic lines

**What worked:** EXP-14's std-only `.lav` → crate compiler shows that the
generate-ordinary-source route has **none of the macro-front-end failure
modes**: generated code is plain Rust, so conservation E0080s land on the
generated assert with the custom message leading, requirement E0277s carry
the R10-rule-8 `on_unimplemented` phrasing verbatim from the notation's
`error`/`label`/`note` attributes, and overdraws reproduce the canonical R15
shape because the compiler threads every literal magnitude (including the
person's running budget, saturating at zero so an overdraw instantiates
`draw_time::<SPEND, 0, BUDGET>`). Emitting breadcrumb comments **on the same
line as each flow call** means rustc's "while instantiating" note renders the
`.lav` line inside the error itself — the modeller never opens the generated
file. The real trace.sh/modellint/docgen all run green over the output
(0 warnings), i.e. the greppable discipline survives generation because the
emitter *is* a codification of the house style.
**What it cost:** a ~3.2 kLOC one-off std-only compiler for a notation
covering only the continuous-resource core (see F-EXP14-3); a generated-crate
Cargo.toml must declare `test-support = []` because the kernel macros expand
a fixture behind the invoking crate's feature (else 9 unexpected-`cfg`
warnings).
**Workaround used:** none needed.
**Evidence:** `experiments/exp14-dsl-external/RESULTS.md` criteria 2/3b/7;
`generated/v-*/` builds; `lint-root/` tool output.

### F-EXP14-2 — The notation layer owns flow-discipline errors outright, and could own the conservation arithmetic too

**What worked:** the notation-level checker reports R2 violations as
"`water` was already used by `fill_kettle` at line 107 - a resource can be in
only one process at a time (R2)" with file:line:col, caret and a help line —
strictly better than the borrow-checker vocabulary F-025 documents — and
reports R1 leaks ("`heat` is never accounted for") at **generation time**,
where hand-written Rust only catches them with the test-time tripwire
(F-002/F-032). A poisoned-value recovery kind keeps one mistake from
cascading.
**What couldn't be expressed / the deliberate gap:** `lavc` does *not*
evaluate balances, budgets or requirement satisfaction, so those violations
pass through to Rust (the experiment's measurement). But in a declarative
flow every magnitude is a literal the compiler already tracks — it computes
`draw_time`'s LEFT and the turbofish decimals — so a production `lavc
--check` could evaluate every literal balance and overdraw at the notation
layer with perfect spans, **pre-empting the entire F-001
editor-invisibility problem for notation-authored models** (the generated
`const` asserts remain as the Rust-side backstop). Requirement satisfaction
should stay on the Rust side: its E0277 is type-check-time, editor-visible,
and already REQ-phrased.
**Evidence:** `errors/*.lav` transcripts in RESULTS.md; `cargo check`
passes on v-energy/v-overdraw and fails on v-requirement (the F-001 split,
unchanged by generation).

### F-EXP14-3 — Cost and coverage: ~4× denser than Rust over the continuous-resource core; every further R-feature grows the notation toward the language it fronts

**What worked:** 113 notation lines generate a 431-line crate
(hand-written control ≈ 455 lines of cs1), deterministically and
idempotently, in < 0.3 s; the slice covers R1–R3, R6 (minimal), R7–R10, R12
(unbounded boundary only), R15, R16 (single history) — with R10 traceability
and R20/R21 tooling fully intact.
**What couldn't be expressed:** discrete items and finite Peano
suppliers/consumers (R12/R13: `SupplyN`, contents-keeping bins,
F-016/F-034/F-039 machinery), fallible processes (R17), qualifications
(R18), money (R19), per-process unit tests and compile-fail regressions
(R4/R5), multi-characteristic requirement composition, catalogue
characteristics (R6 fixed roles). Each is expressible only by growing the
grammar, checker and emitter — the notation's economy comes precisely from
hard-coding the kernel's semantics (the F-055-point-5 coupling, now on the
generating side).
**Workaround:** the F-EXP14-1 discipline: notation-generated crates for the
continuous core, ordinary hand-written downstream crates beside them for
everything else, under a regenerate-and-diff CI gate (the R20/R21 staleness
pattern); promotion to hand-maintained is one-way (delete the `.lav` and the
headers in one commit — no merge-on-regenerate middle state).
**Evidence:** RESULTS.md criteria 4/5/6.

---

## Recommendation: **ADAPT**

The decisive reason: **the external notation preserves the project's curated
error story end to end** — generated code is ordinary source, so the three
canonical violations fire with hand-written-quality diagnostics *plus* an
inline `.lav` breadcrumb in the rendered error, and trace.sh, modellint and
docgen all run green over the output — while the tool's own error layer is
*better* than rustc's for the R1/R2 flow discipline (named processes, R-rule
phrasing, generation-time leak detection). That is the make-or-break
criterion, and it is met outright (where a macro front-end risks losing
exactly this).

Adapt, not adopt, because of two conditions:

1. **Close the F-001 hole at the notation layer** before production use: add
   the `--check` pass that evaluates literal balances and budget draws
   (everything in a flow is a literal the compiler already tracks), so
   conservation violations in notation-authored models are caught with
   notation spans at generation time, editor-visible, with the generated
   `const` asserts kept as the Rust backstop. This is small (the checker
   already computes every number) and removes the route's one inherited
   weakness.
2. **Scope honesty:** adopt the notation only for the continuous-resource
   core it covers (the R-coverage table), as *one generated crate among
   hand-written ones* under the regenerate-and-diff gate and the one-way
   promotion rule — do not chase full R13/R17–R19 coverage, which would
   regrow Rust's complexity inside the notation and dilute the kernel-macro
   single-source-of-truth.
