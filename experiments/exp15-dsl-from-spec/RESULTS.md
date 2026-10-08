# EXP-15 results: the spec as the DSL

**Tests:** step 10's third candidate — SPEC_TEMPLATE.md is already half-structured; how far is
the EXISTING spec format from being the notation?

**Toolchain:** `rustc 1.98.1 (48a229cea 2026-09-01)` (stable, Homebrew). No external
dependencies (trybuild permitted but not needed — the generator has no compile-fail surface of
its own; the generated crate's compile-fail story is measured directly below).

**What was built:** a std-only generator (`src/` library + the `specgen` bin) that parses the
REAL agreed `case-studies/cs1-pot-of-tea/SPEC.md` (v0.3, read-only corpus) — §2 requirements
(including the prose id-remap note), §3 resource table (kinds, states, quantities,
per-state masses), §4 boundary tables (draw sources, a discrete supplier, a bounded
contents-keeping consumer, unbounded sinks, via-chains), §5 process blocks (actors/draws,
consumes/produces/waste with routing, balances with arithmetic validation, satisfies), and §6
flow orders — and emits the model crate into `generated/cs1-gen/` (own path dep on
model-core). Where the spec under-determines, it emits a numbered **SPEC-HOLE**: a comment
block plus `#[cfg(feature = "deny-holes")] compile_error!(...)`, so `cargo build` compiles
everything the spec determines and `cargo build --features deny-holes` makes the compiler
enumerate the full gap list. Generation is deterministic and idempotent (verified by diff);
every emitted item carries a SPEC.md §/line breadcrumb.

**Headline result:** the generated crate **builds with zero warnings, and its two generated
flow tests (§6 orders (a) and (b)) compile and PASS** — full draw/record/History threading,
waste routed to sinks, tripwires armed, person budget type-checked to `Person<225_000>` at
flow end. The spec mechanically determines the model's *structure* (types, boundary,
process signatures with conserving bodies and asserts, flows) but almost none of its
*error-quality machinery* (requirement bounds, characteristics, permits) — which is exactly
the half the method's curated diagnostics live in.

---

## Verdicts

| Criterion | Verdict |
|---|---|
| 1. Determined ratio (LOC + items) | **measured** — 51% by LOC; 63% of model items fully, 75% incl. partial (tables below) |
| 2. Under-determination list | **delivered** — 12 compiler-enumerable SPEC-HOLEs + 4 silent inferences (the experiment's main product, below) |
| 3. Parseability of the current template | **pass with complications** — the real spec parses with zero errors, but only after ~10 invented formatting conventions; amendments A1–A10 |
| 4. Error story | **pass** — 3 modeller-phrased generation-time errors with §/line refs; Rust-side errors degrade from REQ-phrased E0277 to E0308, but generated E0080 asserts carry a SPEC.md breadcrumb *and fire on plain `cargo build`* (no F-001 caveat — see finding C2) |
| 5. Complement or replacement of EXP-13/14 | **complement** — the front half of EXP-14's pipeline, plus an immediately adoptable spec *validator* |

**Recommendation: ADAPT** (see end).

---

## 1. Determined ratio

### LOC

| | lines |
|---|---|
| Real `model/cs1-pot-of-tea` (src 1244 incl. 161 unit-test lines, tests/flows.rs 219, trybuild 20) | 1483 |
| Generated `generated/cs1-gen` (src 645, tests/flows.rs 113) | 758 |
| **Mechanically emitted volume** | **51%** |

### Item counts (every item in the real crate, classified)

D = fully determined (generated functional equivalent); P = partial (skeleton/signature
generated, substance is a hole); U = undetermined (nothing generated).

| Category | D | P | U | Notes |
|---|---|---|---|---|
| Sealed types, aliases, accessor consts (32) | 27 | 0 | 5 | U: `BrewPermit`, the `KettleAtTheBoil`/`BrewReadyPot` tag-carrying aliases, the `WATER_G`/`TEA_G` accessors |
| Boundary trait impls — Supplier ×1, Consumer ×4 (5) | 5 | 0 | 0 | incl. the F-034 decreasing-space bin and three `Next = Self` sinks |
| Characteristic machinery (11) | 0 | 4 | 7 | 4 bare markers synthesized (unattached); `Boiling`/`LoadedToBrew` consts + permit-gated extractions + sealing all U |
| Requirements: 4 `requirement!` + 4 `satisfies!` (8) | 0 | 4 | 4 | trait/id/F-053 remap/blanket/assert-fn generated; supertrait binding and `satisfies!` placement are holes |
| Boundary fns + fill machinery (16) | 16 | 0 | 0 | draws, constructors, Replicate/Fill/FillSealed, bin/sink constructors |
| Processes (9) | 3 | 2 | 4 | D: `fill_kettle`, `boil`, `load_pot` (incl. `SupplyN<N3>`); P: `pour_and_brew`, `empty_bin`; U: `vent_heat`, the sealed `Dispose` machinery ×3 |
| Tests (17) | 0 | 2 | 15 | P: the two §6 flow tests (pass, but no History-content assertions); U: 9 unit tests, trybuild, compile_fail doctests, fixture + tripwire tests |
| **Total (98)** | **51 (52%)** | **12 (12%)** | **35 (36%)** | |
| **Model items only, excl. tests (81)** | **51 (63%)** | **10 (12%)** | **20 (25%)** | |

Generation itself: `specgen` run < 0.1 s; generator clean build median 2.85 s; generated
crate clean build (incl. model-core) median 1.52 s.

---

## 2. The under-determination list (the main product)

### Compiler-enumerable holes (`cargo build --features deny-holes` → 12 × `compile_error!`)

| Hole | What the spec does not determine | Real-crate decision it stands in for |
|---|---|---|
| U-01 | Per-process unit tests (R5), trybuild cases, rustdoc `compile_fail` regressions (R4) | 9 unit tests, 2 doctest regressions, trybuild + ui |
| U-02..05 | Which resource state carries each requirement's characteristic, what quantity consts it exposes, the sealed/permit design | `Boiling` on `BoilingKettle` (WATER_G, EMBODIED_J), `LoadedToBrew` on `LoadedPot` (DRY_G, SPENT_G), sink markers on the bin/air |
| U-06 | `Satisfies:` tag placement on TYPES + backing `satisfies!` assertions (the F-037 alias workaround) | `KettleAtTheBoil`/`BrewReadyPot` aliases carrying tags |
| U-07 | Literal magnitudes vs generic const parameters | real `boil<G, DRAW_J, EMBODIED_J, HEAT_J>` vs generated `boil(FilledKettle<1_500>, …)` |
| U-08, U-09, U-12 | Requirement-trait bounds on process signatures (R10 style A, F-048) | REQ-phrased `on_unimplemented` wrong-resource errors; generated signatures give E0308 instead |
| U-10 | Waste routing strong vs weak (§8 feedback item 3 verbatim) | real: bin + air fed INSIDE `pour_and_brew` (requirement-bounded consumer params); generated: loose outputs routed by the flow |
| U-11 | The conserving-extraction design (permit-gated `pour_away`/`steep`) | generated bodies defuse + mint, so conservation holds only via the asserts, not by construction |

### Silent inferences (defensible rules the generator had to invent; each a latent hole)

1. **Tripwire choice (F-040):** items minted into a boundary supplier → `no_tripwire`
   (DryTeabag); items created by processes → tripwired (SpentTeabag). Reproduces the real
   crate's choice, but the rule is nowhere in the spec.
2. **State ownership collisions:** "boiling" appears on both the Water and Kettle rows, "tea"
   inside "pot of tea" — rule: the quantity-bearing row owns the type (warned, not erred).
3. **Unqualified resource mention = initial state** ("3 teabags" → `DryTeabag`).
4. **assert-vs-structural guess** for unmarked Balances lines: identical single terms ⇒
   structural, multi-term ⇒ assert (reproduces the real crate's split; §8 feedback item 5).

### Cosmetic-but-real naming drift

Mechanical naming gives `ElectricalEnergy`/`LoadedTeapot`/`BoxOfTeabags`/
`CouncilFoodWasteCollection`/`draw_electrical_energy` where the implementer chose
`Electricity`/`LoadedPot`/`TeabagBox`/`CouncilCollection`/`draw_grid_energy`. Harmless inside
one crate; fatal to regeneration-round-tripping against a hand-touched crate.

Also lost: all curated prose — crate/module docs, the three-field `on_unimplemented`
messages (generated: mechanical message+label from the §2 sentence), and `vent_heat` as a
named REQ-009 process (the generated flow calls `send_to` directly, so the REQ's process
identity evaporates).

---

## 3. Parseability of the current template (candidate amendments)

The REAL spec parses with **zero errors**, but only because the parser grew ~10 conventions.
Each is a candidate SPEC_TEMPLATE amendment:

- **A1 — mark every Balances clause `(assert)` or `(structural)`** (template already
  suggests it; CS-1 doesn't do it — 8 guess warnings). Confirms §8 feedback item 5.
- **A2 — mandatory `Waste routing:` field** on any process with waste (strong/weak choice;
  §8 feedback item 3). CS-1's absence produced hole U-10.
- **A3 — every waste product gets a §3 row.** P5's "36 g food waste" has none; the generator
  synthesized `FoodWaste` from the waste line and warned.
- **A4 — canonical names.** The single biggest fragility: §4/§5 refer to §3 resources by
  prose variants ("loaded pot" vs resource "Teapot"; "550 000 J drawn from the grid" resolved
  only through the §4 via-words). Amendment: backticked canonical identifiers in §3, used
  verbatim in §4–§6; prose stays as decoration.
- **A5 — number format.** Space-grouped thousands ("550 000") forced an invented lexing rule
  (merge following exactly-3-digit groups) that is only unambiguous because `+` is always
  space-separated. Amendment: underscores (`550_000`) or no grouping.
- **A6 — id allocation as a structured field**, not a prose blockquote: the F-053 remap note
  ("implemented as **REQ-006..REQ-009**") needed a phrase-specific scrape across wrapped
  blockquote lines.
- **A7 — one owner row per state** (don't repeat "boiling" on Water and Kettle).
- **A8 — explicit `Orders:` field in §6**; the "(a) P1, P2, …" pattern parses, but only by a
  lucky convention.
- **A9 — item lists stay pure**: trailing participial prose inside Consumes/Produces
  ("…, removed from the pot at the end of brewing") needed a skip-heuristic.
- **A10 — structured characteristics** for per-state quantities ("dry mass 3 g each" worked
  via pattern-scraping; a `state: quantity` shape would be robust).

Verdict: **the template is ~10 small amendments away from being a parseable notation for the
structural half of a model** — and exactly those amendments move it toward being EXP-14's
notation.

## 4. Error story

### Generation-time (spec-validation) errors — the layer this route owns

Three demonstrations against mutated fixture specs (all in `tests/fixtures/`, asserted by
`tests/validation.rs`):

1. **Unbalanced balance** (P3 energy changed to `500 000 + 60 000`):
   ```
   error: SPEC.md:91 §5 P3: the energy balance does not balance: 550 000 ≠ 500 000 + 60 000
   (left totals 550 000, right totals 560 000; a model built from this line cannot compile —
   fix the specification, not the model)
   ```
2. **Waste with no §4 destination** (kitchen-air output row deleted) — one error per routing
   process:
   ```
   error: SPEC.md:89 §5 P3: waste '50 000 J heat' routes to 'kitchen air', which matches no
   §4 output row (every waste product needs a destination row there — SPEC_TEMPLATE §4)
   error: SPEC.md:100 §5 P4: waste '20 000 J heat' routes to 'kitchen air', …
   ```
3. **Unknown resource** (P2 consumes "3 oolong pearls"):
   ```
   error: SPEC.md:80 §5 P2: '3 oolong pearls (one at a time from the box, R12)' names no §3
   resource or state: every consumed/produced item must be a §3 row (or a state listed there)
   ```

These are phrased at the *document*, with section + line, before any Rust exists — a class of
error the method has never had (today an unbalanced spec line surfaces days later as an E0080
in the implemented crate). Also validated: Satisfies ids exist in §2; the Balances time
clause equals the Actor draw; "no person" processes draw no time; flow orders name real
processes.

### Rust errors in the generated code

- **Wrong processing state** (a `FilledKettle` passed to `pour_and_brew`): plain **E0308**
  — `expected BoilingKettle<1500, 500000>, found FilledKettle<1500>` at the call site.
  Readable, but a quality **loss** vs the real crate's REQ-phrased
  `on_unimplemented` ("this kettle may not be poured into the pot: … is not at the boil
  (REQ-006)") — the direct cost of hole U-08/09/12.
- **Hand-broken conservation assert** in generated `boil`: **E0080 whose message carries the
  spec breadcrumb** — `energy conservation violated in boil (SPEC.md §5 P3 line 91): energy
  550 000 = 500 000 + 50 000` — and, because generated processes are non-generic literals, it
  fires on **plain `cargo build` of the library, with no instantiation needed**: the F-001
  post-monomorphization caveat does not apply to literal processes (finding C2).
- The hole convention itself: `cargo build --features deny-holes` lists all 12 gaps as
  compile errors with their summaries — the gap list is compiler-enumerable.

## 5. Complement or replacement?

**Complement — specifically, the front half of EXP-14's pipeline, plus a standalone spec
validator.**

- What the spec determines mechanically is the model's **structure**: §3/§4 give every sealed
  type, supplier, consumer and sink (including the F-034 bin shape); §5 gives signatures,
  conserving bodies and the asserts; §6 gives working flows. That is ~63% of model items and
  it genuinely runs.
- What it cannot determine is the **error-quality machinery** the method depends on —
  requirement bounds, characteristic consts, permit-gated extractions, curated
  `on_unimplemented` prose, tests. Those 12 holes are precisely the things an EXP-14-style
  notation would have to carry as explicit syntax. A spec amended until the holes close (A1–
  A10 plus characteristic/bound syntax) *is* a notation — at which point it stops reading as
  a natural-language document, which is the spec's whole point.
- So the honest division of labour: **SPEC.md stays the human contract and becomes
  machine-checked** (the validation layer above runs at spec-review time, in CI against the
  document); the generator's output is a first-cut scaffold the implementer completes by
  filling the enumerated holes — or the front end that feeds a `.lav`-grade notation where
  one exists. It does not replace EXP-13/14's role of preserving curated errors, and it is
  strictly better than both at catching errors *before* any code exists.

## Candidate FINDINGS entries

- **C1 (spec-determinism split).** The agreed SPEC_TEMPLATE format mechanically determines
  ~51% of a model crate by LOC and ~63% of model items (CS-1 corpus): all structure (sealed
  state types incl. multi-quantity const params, suppliers/consumers/sinks, boundary fns,
  process signatures + conserving bodies + balance asserts, passing §6 flow tests) — and none
  of the error-quality machinery (requirement bounds, characteristic consts, permit-gated
  extractions, tests). The 12-item hole list in exp15's RESULTS is the exact gap between
  spec-as-document and spec-as-DSL. Evidence: `experiments/exp15-dsl-from-spec/`.
- **C2 (literal processes dodge F-001).** A process generated with literal const magnitudes
  (non-generic fn) has its `const { assert!(…) }` evaluated when the library itself is built:
  conservation violations fail plain `cargo build` of the crate with no instantiation and no
  test. The F-001 post-monomorphization caveat is a cost of *generalized* (const-generic)
  processes, not of conservation asserts per se. Trade-off: literal processes serve exactly
  one worked instance.
- **C3 (spec-validation error class).** Balance arithmetic, waste-destination closure,
  §3↔§5 name closure, Satisfies-id closure and draw/balance agreement are all checkable
  against the DOCUMENT in milliseconds, with §/line references and modeller phrasing
  ("fix the specification, not the model"). This error class is adoptable independently of
  any code generation.
- **C4 (naming drift is the parsing fragility).** Cross-section prose naming ("loaded pot" /
  "Teapot"; "drawn from the grid" / "Electrical energy") forced a fuzzy resolution lexicon;
  canonical backticked identifiers in §3 would remove the whole class. Same lesson for
  machine-read prose: the §2 id-remap note, §6 orders and characteristics column each needed a
  phrase-specific scrape (template amendments A1–A10).
- **C5 (the §8 loop closes).** CS-1's round-trip feedback items 2 (adjacent draws), 3 (waste
  routing), 5 (assert/structural), 6 (multi-quantity states) were each re-discovered
  mechanically by the generator as warnings or holes — evidence the §8 feedback mechanism
  captures real under-determination, and that those four belong in the template.
- **C6 (F-040 is derivable).** The tripwire/no_tripwire choice followed mechanically from
  the boundary tables: supplier-held items untripwired, process-created waste tripwired —
  reproducing the real crate's choice. Candidate rule for the template/linter rather than
  per-model judgement.

## Protocol compliance

Everything inside `experiments/exp15-dsl-from-spec/`; model/, case-studies/ and model-core
read-only (consumed by path); no git commands; stable rustc 1.98.1; std-only generator, no
external deps; generated crate depends on model-core by path and builds clean (`cargo build`
0 warnings, `cargo test` 2/2 flow tests pass, 5/5 generator regression tests pass);
timebox respected (no sub-task needed a third approach).

## Recommendation

**ADAPT.** Do not adopt spec-as-DSL as the model's source of truth (the 12 holes are the
method's soul — curated errors, requirement bounds, conservation-by-construction — and
closing them inside SPEC.md would just turn the spec into EXP-14's notation with worse
syntax). Adopt instead, in this order:
1. **The validator** (C3) as a spec-review/CI gate over SPEC.md documents — immediately, it
   needs nothing else.
2. **Template amendments A1–A10** (headline: mark balances, mandatory Waste-routing field,
   canonical backticked names) so the document stays one mechanical parse away from code.
3. **The generator as scaffolding** for new case studies: emit the structural 63%, let the
   implementer fill the enumerated deny-holes — pairing naturally with EXP-14 (this is its
   pipeline's front half) rather than competing with it.
