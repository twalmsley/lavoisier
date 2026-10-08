# Lavoisier

**Model-Based Systems Engineering in Rust's type system: resources, processes and
requirements as types, with conservation enforced by the compiler.**

> *"Rien ne se perd, rien ne se crée, tout se transforme."*
> — Antoine Lavoisier
>
> Nothing is lost, nothing is created, everything is transformed — and here, anything
> else is a compile error.

## What this is

An experiment in pushing a statically-typed language as far as it will go as a systems
modelling medium. A real-world system — materials, people, tools, time, energy, waste —
is modelled so that:

- **Types stand for things**: resources, products, waste, requirements, processing states.
- **Functions stand for processes**: everything a process needs is moved in by value, and
  everything it produces — products, by-products, waste, reusable resources — is returned.
  Rust's ownership rules mean a resource cannot be duplicated, cannot be used by two
  processes at once, and (with a layered lint-and-tripwire regime) cannot silently vanish.
- **Compile errors expose model mismatches**: a missing resource, a mass balance that
  doesn't add up, an exhausted supplier, an unqualified input, a product used before the
  process that makes it — none of these compile.
- **Requirements are named traits** (`REQ-001`, `REQ-002`, …) that processes take as
  bounds, with tests tagged to the requirements they verify and a shell script that turns
  the source code into a traceability report — and fails CI if a requirement has no
  verifying test.

A process signature reads as a work instruction, and the whole model reads as a
specification the compiler keeps honest:

```rust
/// Satisfies: REQ-001
/// Four REQ-001-compliant bolts are drawn from one supplier; mass is conserved.
pub fn fasten<B: Req001FasteningBolt, S: SupplyN<N4, Taken = FourBolts<B>>, /* G1, G2, OUT */>(
    person: Person<MS>, plates: (DrilledPlate<G1>, DrilledPlate<G2>), bolts: S,
) -> (Person<LEFT>, Assembly<B, OUT>, S::Next) // const { assert!(OUT == G1 + G2) } — the compiler checks the grams
```

Splitting 2000 g of steel into 1500 g + 600 g does not compile. Drawing a fifth bolt from
a box typed as holding four does not compile. Handing the same technician to two
processes at once does not compile. A box of 100 bolts is a type 100 levels deep — on
purpose; finding where this approach shines and where it collapses is the point of the
project, and both are documented.

## Repository layout

| Path | What it is |
|---|---|
| [`instructions.md`](instructions.md) | The agreed requirements of the modelling approach itself (R1–R22): conservation rules, type-level capacity, traceability conventions, the lint/tripwire regime, boundary design, fallible processes, qualifications, money, the generated-documentation and analysis rules, the specification gate, open questions. Every rule is backed by experiment evidence. |
| [`FINDINGS.md`](FINDINGS.md) | The findings log (F-001–F-064): everything stable Rust could not express, what each limitation cost, and the workaround adopted — from "conservation errors are invisible to `cargo check`" to a deterministic rustc SIGBUS. |
| [`experiments/`](experiments/) | Fifteen self-contained experiment crates (EXP-01–EXP-15), each testing one risky claim in isolation, with per-criterion verdicts and verbatim compiler output in each `RESULTS.md` — the last three (EXP-13–EXP-15) are the DSL candidates whose verdicts became R22. |
| [`model/`](model/) | The real workspace, ten crates gated by `ci.sh`: **`model-core`** (type-level numbers, units and quantities, the sealed-resource kernel, boundary traits, execution history, requirement macros), **`pilot-workshop`** (a complete small model — cut → drill → fasten — with fallible drilling, a qualified operator, money, and live requirements traceability), and the eight case-study crates: **`cs1-pot-of-tea`**, **`cs2-puncture-repair`**, **`cs3-cafe-orders`**, the two-crate **`cs4-stores`**/**`cs4-line`** subsystem pair, and the three-crate **`cs5-supply`**/**`cs5-logistics`**/**`cs5-works`** split. |
| [`docs/`](docs/) | Project documentation, led by the **[modeller's guide](docs/modellers-guide.md)**: the practical companion to `instructions.md` — setting up a model crate, the spec-to-model walkthrough on CS-1, the full error-reading guide, the must-not-break checklist, and the known limitations stated plainly. |
| [`docs/tutorials/`](docs/tutorials/) + [`learn/`](learn/) | The learning materials: a numbered tutorial sequence (from a Rust on-ramp for systems engineers to a capstone model), paired with a hands-on exercises crate — broken files you fix until `cargo test` passes, with solutions. |
| [`docs/white-paper.md`](docs/white-paper.md) | The white paper (v0.3, living): the full account — motivation, method, approach, experiments, the case-study ladder, the toolchain, findings, limitations, conclusions — with a committed PDF rebuilt by `docs/build-paper.sh`. |
| [`docs/diagrams/`](docs/diagrams/) | Generated diagrams for each model crate at three levels (context, top-level, detailed): GitHub-rendered Mermaid with legend tables hyperlinking every node into the source. Regenerate with `tools/diagrams.sh`. |
| [`docs/processes/`](docs/processes/) | Generated work instructions (R20): one human-readable document per process, boundary function and flow — 166 documents across the nine model crates, everything factual recovered from source, nothing invented. Regenerate with `tools/docgen.sh`. |
| [`docs/analysis/`](docs/analysis/) | Generated model-analysis reports (R21): one lint report per crate from a 23-check catalogue (sealing hard rules, extraction conventions, improvement surface), diffed for staleness by `ci.sh`. Regenerate with `tools/lint.sh`. |
| [`tools/`](tools/) | The std-only toolchain, outside the model workspace: `diagram-gen` (one shared source-extraction library behind three binaries — `diagram-gen`, `docgen`, `modellint`) and `spec-gen` (two binaries: `speccheck`, the R22 specification gate, and `specgen`, the one-shot model scaffolder), driven by `diagrams.sh`, `docgen.sh`, `lint.sh` and `spec.sh`. |
| [`decks/`](decks/) | Slide decks: the non-technical overview (12 slides) and the technical features tour (22 slides) — markdown sources rendered by GitHub, with committed PDFs rebuilt by `decks/build.sh`. |
| [`SPEC_TEMPLATE.md`](SPEC_TEMPLATE.md) | The natural-language specification template new models are written from: each section maps mechanically onto model elements, anything a spec leaves out comes back as a numbered question rather than a guess, and its conventions (A1–A10) are machine-validated by the `speccheck` gate on every CI run. |
| [`case-studies/`](case-studies/) | The agreed case-study specifications (filled-in copies of the template), all five implemented: [CS-1 making a pot of tea](case-studies/cs1-pot-of-tea/SPEC.md), [CS-2 bicycle puncture repair](case-studies/cs2-puncture-repair/SPEC.md) (fallibility, rework, money), [CS-3 café order fulfilment](case-studies/cs3-cafe-orders/SPEC.md) (real concurrency, merged Histories), [CS-4 small-batch production run](case-studies/cs4-batch-run/SPEC.md) (scale, two subsystem crates, and the measured [change-impact exercise](case-studies/cs4-batch-run/CHANGE-IMPACT.md)) and [CS-5 two-site fulfilment](case-studies/cs5-two-site/SPEC.md) (two currencies, transport, three team-shaped crates). |
| [`PLAN.md`](PLAN.md) | The programme plan for the case-study and publication phase: the case-study ladder (CS-1–CS-5, each stressing something no earlier one did), the deliverables in order, and the live status table. |

## Getting started

Stable Rust only (developed on 1.98.1); the single dev-dependency is `trybuild`.

```sh
cd model
./ci.sh
```

`ci.sh` is the whole verification story in one eight-step gate: full build (conservation
asserts fire at monomorphization, so `cargo check` alone is *not* enough), the test suites
(unit, integration, pinned compile-fail cases, doc-test `compile_fail` regressions,
tripwire `should_panic` demos), clippy under the project's restriction lints, a plain
no-features build proving test fixtures can't leak into production code, the requirements
traceability report, a feature-placement audit, the model-analysis gate (the lint reports
regenerated and diffed against the committed `docs/analysis/` — stale reports fail), and
the specification gate (every case-study spec machine-validated, so an unbalanced spec
line fails before any code exists).

To model a system yourself, start with the
**[modeller's guide](docs/modellers-guide.md)** — it walks from a blank crate to a passing
gate on the CS-1 example and translates the compiler's errors back into model language.

## Method

Everything here was built evidence-first: requirements were agreed in writing, each risky
claim became an isolated experiment with pass/fail criteria, the findings were
consolidated into `FINDINGS.md`, and only then did each design decision — with its
documented cost — get written back into `instructions.md` and implemented in the library.
Where the type system lost (silent drops are only partly preventable; conservation errors
are invisible to editor diagnostics; one trait-resolution shape crashes the compiler
outright), the loss is recorded as a finding and mitigated, not papered over.

## Status

Working and verified: the core library with all agreed requirements implemented (R1–R22,
including execution history, fallible processes, qualifications and money), the pilot
model, and the full eight-step CI gate. **Programme steps 0–11 are complete**
([`PLAN.md`](PLAN.md)): the case-study ladder CS-1 through CS-5 ran the spec→model round
trip end to end, and the toolchain followed — generated diagrams, generated work
instructions, the model linter (whose first run caught three latent sealing gaps, F-058),
and the step-10 DSL experiments, whose verdict is the R22 pipeline: specifications
machine-checked before implementation, new models started as `specgen` scaffolds with
every open decision an enumerable compile error, model source always ordinary Rust. The
full external notation is validated (EXP-14) and deliberately **parked on the
experimental track**. Findings are logged through F-064; the white paper (v0.3) is the
full account. Next: case studies born as specgen scaffolds, and the remaining open
questions — validation vs verification above all — listed at the end of
`instructions.md`; change impact has its measured demonstration in
[CS-4's change-impact exercise](case-studies/cs4-batch-run/CHANGE-IMPACT.md).
