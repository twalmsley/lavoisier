# EXP-13 results: a DSL as a macro front-end

**Toolchain:** `rustc 1.98.1 (48a229cea 2026-09-01) (Homebrew)`, stable. Dependencies: `model-core` by path (EXP-10 protocol update), `trybuild` as the only external dev-dependency. The proc-macro fallback was **not needed** (see "macro_rules! sufficiency").

**Layout:**

| Path | Contents |
|---|---|
| `control/` | the CS-1 slice hand-written (lifted from `model/cs1-pot-of-tea`): ColdWater/Electricity/WasteHeat containers, kettle states, mains/grid draws, `fill_kettle`, `boil` (energy assert), REQ-001 (boiling water, R10 incl. rule 8) bound on `pour_cuppa`, one flow |
| `dsl/` | the same slice authored through `model!` (`dsl/src/macros.rs`, a 294-line `macro_rules!` tt-muncher); same type names, same signatures, same flow |
| `violations/` | the two post-monomorphization canonical violations, feature-gated per (violation × arm); transcripts in `transcripts/` |
| `*/tests/ui/` | the type-check-time canonical violation (non-boiling kettle at the requirement bound), trybuild-pinned per arm |
| `trace.sh` | **verbatim copy** of `model/trace.sh`, run over both arms |
| `toolcheck/` | scratch root (symlinked `model/exp13-{control,dsl}`) over which the **real** `tools/diagram-gen` binaries (`diagram-gen`, `docgen`, `modellint`) were run; their outputs are committed as evidence |

`cargo build` is warning-free and `cargo test` passes across the workspace (unit tests, both flows, both pinned trybuild cases, both `compile_fail` doc-tests).

## Verdicts

| Criterion | Verdict | Evidence |
|---|---|---|
| 1. Error quality through the DSL (make-or-break) | **pass with complications** | §Error quality: all three canonical violations keep the modeller-visible head (the curated assert/REQ message) and tail (the instantiation note on the modeller's own `step` line, decimal magnitudes); degradation is confined to the middle breadcrumbs |
| 2. LOC saved per model | **pass with complications** | §LOC: −44 % on model-definition code (163 → 92); flow authoring saves **nothing** (39 → 43, worse); the macro itself costs 294 lines once (~3 slice-sized models to amortize) |
| 3. R-coverage | **pass with complications** | §R-coverage table: declarations and balance processes covered; characteristics, permits, sealed machinery, finite suppliers/consumers and all of R17/R18 fall through to `rust { }` / `body { }` escape hatches (raw Rust inside the invocation) |
| 4. Compile-time delta | **pass** | clean `cargo build -p <arm>` ×3: control 1.37/1.29/1.27 s (median 1.29), dsl 1.79/1.40/1.43 s (median 1.43): ≈ +0.14 s, ~11 % — noise at model scale |
| 5. trace.sh compatibility | **pass with complications** | §Tools: the verbatim trace.sh produces **identical row structure** for both arms — but only because the grammar was *designed* to keep literal `fn`/`trait`/`type` keywords and doc-tag lines on invocation lines; the sugared requirement form is invisible (REQ-003 → "unknown requirement" warning, exit 1) |
| 6. lint / docgen / diagrams compatibility | **fail** | §Tools, real binaries: docgen emits **0** process documents for the dsl arm vs **10** for control; context diagram 1 node/0 edges vs 8/7; "no flow to trace"; modellint raises **2 false gate-fatal ERRORs** on compliant tags and passes the sealing checks **vacuously** (it sees no resource types at all) |
| 7. macro_rules! structural sufficiency | **pass with complications** | no proc-macro needed for this grammar; the F-013/F-036 walls were avoided by design concessions (bracketed generics, modeller-spelled derived names, keyword-led sections) and by surrendering the uncoverable constructs to escape hatches |

## Error quality — the three canonical violations, side by side

All violations were authored **through** `model!`'s flow grammar (`violations/src/lib.rs`, `dsl/tests/ui/not_boiling_dsl.rs`); the control versions are plain Rust against `exp13-control`. Full transcripts: `transcripts/*.txt` and `*/tests/ui/*.stderr`.

### Violation 1 — energy assert (`boil`, 500 000 + 60 000 ≠ 550 000), E0080 at monomorphization

**Control** (`transcripts/energy-control.txt`):

```text
error[E0080]: evaluation panicked: energy conservation violated in boil (R15): the embodied energy plus the kettle's waste heat must sum exactly to the energy drawn from the grid
   --> …/library/core/src/panic.rs:62:9
    |
    |           ^^^^…^^^ evaluation of `exp13_control::model::processes::boil::<1500, 550000, 500000, 60000>::{constant#0}` failed here
    |
   ::: control/src/model.rs:305:13      ← the assert's own text, quoted in full
    …
note: erroneous constant encountered
   --> control/src/model.rs:304:9       ← the readable `const { assert!(…) }` block
    …
note: the above error was encountered while instantiating `fn boil::<1500, 550000, 500000, 60000>`
  --> violations/src/lib.rs:36:31
   |
36 |         let (boiling, heat) = boil::<1500, 550_000, 500_000, 60_000>(filled, energy);
```

**DSL** (`transcripts/energy-dsl.txt`):

```text
error[E0080]: evaluation panicked: energy conservation violated in boil (R15): the embodied energy plus the kettle's waste heat must sum exactly to the energy drawn from the grid
   --> …/library/core/src/panic.rs:62:9
    |
    |           ^^^^…^^^ evaluation of `exp13_dsl::model::processes::boil::<1500, 550000, 500000, 60000>::{constant#0}` failed here
    |
   ::: dsl/src/model.rs:167:5
    |
167 | /     crate::model! {
168 | |         /// P1 — fills the kettle with the drawn cold water. …
...   |
234 | |     }
    | |_____- in this macro invocation        ← the WHOLE 67-line invocation, not the assert
    = note: this error originates in the macro `$crate::panic::panic_2021` …
note: erroneous constant encountered
   --> dsl/src/macros.rs:206:16
    |
206 |               $( const { assert!($aexpr, $amsg) }; )*   ← macro internals, not model text
    …
note: the above error was encountered while instantiating `fn boil::<1500, 550000, 500000, 60000>`
  --> violations/src/lib.rs:57:36
   |
57 |             step (boiling, heat) = boil::<1500, 550_000, 500_000, 60_000>(filled, energy);
```

**Judgement:** what the error-reading guide tells modellers to read — the leading curated message and the trailing "while instantiating" note with decimal magnitudes **on the modeller's own line** — is intact and equal. What degrades is the middle: the control shows the assert's arithmetic at its own 2-line span; the DSL shows a 67-line whole-invocation span plus a note into `macros.rs` macro internals. A modeller following the guide is unharmed; a modeller reading top-to-bottom meets `$( const { assert!($aexpr, $amsg) }; )*` — noise, not misdirection.

### Violation 2 — non-boiling kettle at the requirement bound, E0277 at type-check time

**Control** (`control/tests/ui/not_boiling.stderr`):

```text
error[E0277]: this kettle may not be poured: `FilledKettle<1500>` is not at the boil (REQ-001)
  --> tests/ui/not_boiling.rs:15:69
   |
15 |     let (kettle, cuppa, loss) = pour_cuppa::<1500, 490_000, 10_000, _>(filled);
   |                                                                     ^ REQ-001: tea must be brewed with boiling water
   = help: the trait `Boiling` is not implemented for `FilledKettle<1500>`
   = note: boiling is a process state (R9): `boil` turns a `FilledKettle` into a `BoilingKettle` - only that state can be poured
help: the trait `Boiling` is implemented for `BoilingKettle<G, E>`
   = note: required for `FilledKettle<1500>` to implement `Req001BoilingWater`
note: required by a bound in `pour_cuppa`
   | ... OUT_E: u64, const LOSS_J: u64, K: Req001BoilingWater>(kettle: K) -> …
```

**DSL** (`dsl/tests/ui/not_boiling_dsl.stderr`):

```text
error[E0277]: this kettle may not be poured: `FilledKettle<1500>` is not at the boil (REQ-002)
  --> tests/ui/not_boiling_dsl.rs:18:74
   |
18 |         step (kettle, cuppa, loss) = pour_cuppa::<1500, 490_000, 10_000, _>(filled);
   |                                                                          ^ REQ-002: tea must be brewed with boiling water
   = help: the trait `Boiling` is not implemented for `FilledKettle<1500>`
   = note: boiling is a process state (R9): `boil` turns a `FilledKettle` into a `BoilingKettle` - only that state can be poured
help: the trait `Boiling` is implemented for `BoilingKettle<G, E>`
   = note: required for `FilledKettle<1500>` to implement `Req002BoilingWater`
note: required by a bound in `pour_cuppa`
   |         process fn pour_cuppa [const OUT_G: u64, const OUT_E: u64, const LOSS_J: u64, K: Req002BoilingWater] {
   |                                                                                          ^^^^^^^^^^^^^^^^^^ required by this bound in `pour_cuppa`
```

**Judgement:** effectively identical — the primary span lands on the modeller's own `step` line inside the invocation, the full R10-rule-8 phrasing (message, label, note, F-044) survives, and the "required by a bound" note even points at the **DSL** `process fn pour_cuppa […]` line, which reads naturally. The feared `on_unimplemented` loss did not occur.

### Violation 3 — time-budget overdraw (30 000 ms from a 20 000 ms budget), E0080

Control (`transcripts/overdraw-control.txt`) and DSL (`transcripts/overdraw-dsl.txt`) transcripts are **structurally identical**: same curated `draw_time` message, same model-core assert spans, and the instantiation note lands on the modeller's own line in both arms —

```text
note: the above error was encountered while instantiating `fn draw_time::<30000, 0, 20000>`
  --> violations/src/lib.rs:92:37                        (control: lib.rs:74:32)
   |
92 |             step (labour, person) = draw_time::<30_000, 0, 20_000>(person);
```

**Judgement:** zero degradation. When the failing assert lives in a *called* crate (model-core), the macro layer around the call site changes nothing: the span attaches to the modeller's own tokens.

**Why error quality survived (the mechanism):** every violating number and call is written by the modeller *inside* the invocation, so its tokens carry the modeller's spans; `macro_rules!` only rearranges them. Spans degrade exactly where the macro *synthesizes* code around the modeller's tokens (the generated `const { assert!(…) }` wrapper) — and even then only the middle breadcrumbs suffer. The F-013 ident-synthesis walls never arise because this grammar never synthesizes identifiers.

## LOC

Counting the model definition (everything before `#[cfg(test)]`), non-blank non-comment code lines:

| Unit | control | dsl | delta |
|---|---|---|---|
| model definition (code LOC) | 163 | 92 | **−44 %** |
| model definition (raw lines incl. docs) | 348 | 236 | −32 % |
| flow (integration test, code LOC) | 39 | 43 | **+10 %** |
| the `model!` macro itself | — | 294 lines (214 code), one-off | amortizes over ~3 slice-sized models |

Where the saving comes from: boundary constructs (`source`/`sink`/`entry` collapse a reusable + entry fn + draw fn ≈ 25 hand-written lines to 3–4), and declarative processes (`fill_kettle`/`boil` bodies — asserts, defuses, destructure, mints — are generated from the balance declaration). Where it does not: the flow grammar is isomorphic to `let`-bindings (`step (a,b) = f(x);` ⇄ `let (a,b) = f(x);`) — pure syntax tax; and ~40 lines of the dsl arm are `rust { }` islands (characteristic trait, `Boiling` impl, permit, aliases) the grammar cannot express.

## R-coverage table (what `model!` can and cannot express)

| R | Construct | Coverage |
|---|---|---|
| R1 | sealed resources, privacy boundary | **yes** — pass-through to the kernel macros; expansion is in the invoking module, so `pub(crate)` mint/defuse and field privacy work unchanged. Held contents (payload fields) not in the grammar (addable, same shape) |
| R2 | reusables moved and returned | **yes** — `reusable`, `threads`/`absorbs` sections |
| R3/R15 | balance asserts, draws, containers | **yes** — `container`, `source … draw`, `assert "msg" : expr;` per dimension; caller-stated remainders unchanged |
| R4 | compile-fail classes | **yes** — trybuild and rustdoc `compile_fail` both work *through* the DSL (pinned in this crate); the F-001 editor caveat is unchanged |
| R5 | per-process tests | **partial** — `flow test fn` covers flow-shaped tests; ordinary unit tests stay plain Rust |
| R6 | characteristics (traits, assoc consts, kind traits) | **no** — `rust { }` escape hatch; the declarative grammar has no trait vocabulary |
| R7 | quantities/units | **yes** for the kernel's const-magnitude containers; the `Qty<V, U>` polymorphic layer is untouched |
| R9 | one type per state; connection-only flows | **yes** (states are separate `container` items); the flow grammar adds **nothing** over let-bindings |
| R10 | requirements + tagging | **pass-through form: yes** (keeps `/// REQ-NNN:` + `trait` lines literal); **sugared form: breaks trace.sh** (measured below). `Satisfies:`/`Verifies:` tags attach to `process fn`/`flow test fn` lines because the grammar keeps a literal `fn name` on the line |
| R12 | finite Peano suppliers/consumers, SupplyN | **no** — not in the grammar (the slice needed only unbounded `sink`s, which are covered); fill machinery, sealed recursion = raw Rust |
| R13 | discrete items | **no arm written** (trivially addable as `consumable` pass-through) |
| R16 | history | **neutral** — `step`/`send` call `record`/`send_to` like any process |
| R17 | fallible processes (Result arms, tokens, per-branch asserts) | **no** — would need match/arm vocabulary, outcome-token grammar, doubled assert sections; left to raw Rust |
| R18 | qualifications/safety | **no** — characteristics again |
| R19 | money | same machinery as R15 → would work via `container`/`source` |
| R20 | generated documentation | **fail** — see Tools |
| R21 | the linter | **fail** — see Tools |

## Tool compatibility (the real tools, run over both arms)

**trace.sh (verbatim copy of `model/trace.sh`):** the compliant DSL arm produces rows **structurally identical** to the control — definition (from the `requirement { }` pass-through's literal `/// REQ-002:` + `trait` lines), both `Satisfies:` tags (on a `pub type` in a `rust { }` island and on the `process fn pour_cuppa …` line), the bound use (pass 2 finds `Req002BoilingWater` on the `process fn` line), and both `Verifies:` tags (unit test + the `flow test fn` line). This works **only** because the grammar deliberately embeds the literal item keywords (`fn`, `trait`, `type`) and doc-tag lines at the invocation site — trace.sh's `item_name()` matches `fn pour_cuppa` inside `process fn pour_cuppa […]` unanchored. The **sugared** requirement form (`requirement Req003ServedHot is "REQ-003: …" needs (Boiling), …`), which generates the doc line as `#[doc = …]`, is invisible: trace.sh reports `WARNING: unknown requirement REQ-003 tagged at ./dsl/src/model.rs:106` and exits 1. Loud *here* because a tag referenced it; a sugared requirement nobody tags would vanish without any warning — the F-037 silent direction.

**docgen / diagram-gen / modellint (the real `tools/diagram-gen` binaries over `toolcheck/`):**

| Measurement | control | dsl |
|---|---|---|
| docgen process documents | **10** (boil, fill_kettle, pour_cuppa, 5 boundary fns, 2 draws) | **0** (README only) |
| context diagram | 8 nodes, 7 edges | **1 node, 0 edges** |
| top-level diagram | 12 nodes, 13 edges | 1 node |
| traced flow | 15 nodes, 21 edges | **"no flow to trace"** (the flow exists — inside `model!`) |
| modellint | 0 errors, 11 info | **2 gate-fatal ERRORs (both false positives)** + 1 info |

The two modellint ERRORs are `E-TAG-ASSERT` claiming the `Satisfies: REQ-002`/`REQ-003` tags have "no backing `satisfies!` assertion" — the assertions **are** in the source, on literal `model_core::satisfies!(…)` lines, but inside the `crate::model! { }` invocation, where the scanner does not look. Meanwhile the `E-TAG-MACRO` check (F-037's dedicated gate) reports **clean** — it recognizes only `model_core::`-prefixed macro invocations (the F-055 point-5 hard-coded kernel table), so it fails to flag tags inside an unknown macro. Worse than the false positives is the **vacuous clean**: every sealing check (`E-SEAL-*`, `E-F034`, `E-DEBUG-BUNDLE`) passes because the linter sees *no resource types at all* — a macro-front-end model is unauditable by R21, with the gate green. That is the F-037 silent-direction failure at the scale of the whole model.

## macro_rules! sufficiency (the permitted proc-macro fallback was not reached)

The fallback was never needed, but only because the grammar made three concessions up front, each an F-013/F-036-class cost:

1. **Generics in square brackets** (`process fn boil [const G: u64, …]`): a `<…>` list is not one token tree and cannot be matched without a per-parameter tt-muncher (the kernel's `__mc_parse_generics!` exists for exactly this; its NT-vs-`const` local-ambiguity wall is documented in F-036). Brackets make the list one `tt` group.
2. **Every derived name is modeller-spelled** (`enter = new_mains_tap`, `draw = draw_cold_water`, `assert = assert_req002`): stable `macro_rules!` cannot synthesize identifiers (F-013), so the DSL cannot give you `new_<name>` for free.
3. **Uncoverable constructs surrender to escape hatches** (`rust { }` items, `body { }` process bodies): characteristics, permit-gated extractions, sealed recursion. This is the honest statement of structural insufficiency — a proc-macro could parse more syntax but could not *design away* the need for trait/impl semantics, so the escape hatch is the correct boundary, not a parsing workaround.

Grammar quirks found: declarative processes return a trailing-comma tuple (`(A, B,)` — identical type for n ≥ 2, but a single-output process would return a 1-tuple `(A,)`, a surface divergence); `assert` clauses use `$msg:literal : $expr:expr;` because a `tt*` expression would eat the terminator; flow `step` needed a separate single-binding arm (`step history = record(…)`) because `(x,)` is a 1-tuple pattern. Generated `#[doc = concat!("Placeholder: ", …)]` lines satisfy `missing_docs` but are **not** greppable `Placeholder:` source lines — the R12/R20 placeholder inventory loses them (same class as the trace findings).

## Candidate FINDINGS.md entries

### F-EXP13-A — A macro front-end preserves curated error quality because violations live in the modeller's own tokens

**What worked (contrary to the expected failure mode):** all three canonical violations authored through `model!` kept the modeller-visible error head (the curated assert/`on_unimplemented` message) and tail (primary span or "while instantiating" note on the modeller's own `step …` line, decimal magnitudes) identical to the hand-written control; the requirement E0277's bound note even points at the readable `process fn … K: Req002BoilingWater` DSL line. `macro_rules!` only rearranges the modeller's tokens, and spans follow tokens.

**What it cost:** the middle breadcrumbs degrade where the macro synthesizes code: the generated-assert E0080 names the whole 67-line invocation as "this macro invocation" and its "erroneous constant" note points into the macro's own `$( const { assert!($aexpr, $amsg) }; )*` line — noise a modeller following the error-reading guide skips, but hostile to top-to-bottom reading.

**Evidence:** EXP-13 (`experiments/exp13-dsl-macro/transcripts/*.txt`; `*/tests/ui/*.stderr` pairs).

### F-EXP13-B — The grep discipline survives a macro DSL only where the DSL's surface syntax converges on Rust's; every sugared form is invisible

**What couldn't be expressed:** any notation-level sugar over the R10 discipline. The `model!` grammar keeps trace.sh fully working (identical report rows to the hand-written control) **only** by design concessions: a literal `fn name` in `process fn`/`flow test fn` lines, a literal `trait` line via the `requirement { }` pass-through, literal `///` tag lines at the invocation site. The measured counter-example: the sugared `requirement Name is "REQ-NNN: …"` form generates the doc line as an attribute, and trace.sh loses the definition (`WARNING: unknown requirement REQ-003`, exit 1 — loud only because a tag referenced it; untagged, silent). Generated `Placeholder:` doc attributes are likewise invisible to the placeholder inventory.

**What it cost:** the DSL is pulled back toward Rust's own shape exactly where tooling looks, capping how far "easier to write" can go.

**Evidence:** EXP-13 (`trace.sh` verbatim-copy run; `dsl/src/model.rs` REQ-002 vs REQ-003).

### F-EXP13-C — The R20/R21 extraction tools are structurally blind to a macro-front-end model (F-055 point 5, escalated)

**What couldn't be expressed:** a `model!`-authored model the documentation generator, diagram generator or linter can see. Run over the DSL arm, the real tools produced: **0** process documents (control: 10), a 1-node context diagram (control: 8 nodes/7 edges), "no flow to trace" (the flow exists, inside the invocation), **2 false gate-fatal lint ERRORs** (`E-TAG-ASSERT` cannot see `satisfies!` lines inside the invocation), and — the dangerous direction — **vacuously clean** sealing/F-034/F-047 checks and a clean `E-TAG-MACRO` (the F-037 gate recognizes only `model_core::`-prefixed invocations), so an unauditable model passes the R21 gate green.

**What it cost / workaround:** none viable inside the tools short of teaching them the second grammar (a second parser per DSL construct — the F-055 point-5 hard-coded-kernel coupling multiplied), or constraining the DSL until it is line-shaped Rust again (F-EXP13-B).

**Evidence:** EXP-13 (`toolcheck/docs/**`, `toolcheck/analysis/exp13-dsl.md` vs `…-control.md`).

### F-EXP13-D — What a stable macro_rules! model grammar costs

**What couldn't be expressed, and the concessions:** angle-bracket generic lists (→ square-bracket `[const G: u64, K: Req…]` groups); any synthesized identifier (F-013 — every `enter =`/`draw =`/`assert =` name is modeller-spelled); trait/impl semantics (characteristics, permit-gated extractions, sealed recursion → `rust { }`/`body { }` escape hatches, ~40 of the dsl arm's 92 code lines); flow sugar with any analysis value (the `step` grammar is isomorphic to `let`-bindings — +10 % LOC over the control's plain flow). The declarative savings are real but confined to the declaration layer: −44 % code LOC (163 → 92) on this slice, against a 294-line one-off macro.

**Evidence:** EXP-13 (`dsl/src/macros.rs`; LOC table in RESULTS.md).

## Recommendation: **REJECT** the macro front-end as the model-authoring route

**Decisive reason:** the expansion is invisible to the extraction toolchain the method now depends on. R20 (generated documentation) and R21 (the linter gate) parse *source*; a `model!` model yields zero process documents, empty diagrams, false gate-fatal lint errors and — worst — vacuously green sealing checks (the silent F-037 direction at whole-model scale). Keeping trace.sh alive already forced the DSL's surface back into Rust-shaped lines (literal `fn`/`trait`/tag lines), and closing the R20/R21 gap would mean either teaching every tool a second grammar or finishing that convergence — at which point the front-end buys ~71 code lines per slice-sized model and a +10 % flow tax, while the interesting parts of any model (characteristics, permits, fallibility) still require raw Rust in escape hatches.

The error-quality result — the criterion expected to kill the route — is the surprise of the experiment and should be recorded as a *positive* precedent (F-EXP13-A): modeller-token-preserving macros do not cost curated errors. That precedent licenses the narrow **adapt** path worth keeping: grow **model-core's kernel macro family** (the existing `draw_process!` shape) one construct at a time — `source`/`sink`/`entry` generators in the kernel would bank most of this experiment's declaration-layer savings while staying inside the `model_core::`-prefixed invocation set the tools already special-case (F-055 point 5), rather than introducing a second, tool-invisible grammar. A whole-model `model! { }` front-end should not be pursued; EXP-14's external-notation route (generating real, tool-visible source) is the structurally sound way to buy authoring ergonomics.
