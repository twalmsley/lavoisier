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
| [`instructions.md`](instructions.md) | The agreed requirements of the modelling approach itself (R1–R16): conservation rules, type-level capacity, traceability conventions, the lint/tripwire regime, boundary design, open questions. Every rule is backed by experiment evidence. |
| [`FINDINGS.md`](FINDINGS.md) | The findings log (F-001–F-040): everything stable Rust could not express, what each limitation cost, and the workaround adopted — from "conservation errors are invisible to `cargo check`" to a deterministic rustc SIGBUS. |
| [`experiments/`](experiments/) | Ten self-contained experiment crates (EXP-01–EXP-09), each testing one risky claim in isolation, with per-criterion verdicts and verbatim compiler output in each `RESULTS.md`. |
| [`model/`](model/) | The real library: **`model-core`** (type-level numbers, units and quantities, the sealed-resource kernel, boundary traits, requirement macros) and **`pilot-workshop`** (a complete small model — cut → drill → fasten — with live requirements traceability). Gated by `ci.sh`. |
| [`SPEC_TEMPLATE.md`](SPEC_TEMPLATE.md) | The natural-language specification template new models are written from: each section maps mechanically onto model elements, and anything a spec leaves out comes back as a numbered question rather than a guess. |

## Getting started

Stable Rust only (developed on 1.98.1); the single dev-dependency is `trybuild`.

```sh
cd model
./ci.sh
```

`ci.sh` is the whole verification story in one gate: full build (conservation asserts fire
at monomorphization, so `cargo check` alone is *not* enough), the test suites (unit,
integration, pinned compile-fail cases, doc-test `compile_fail` regressions, tripwire
`should_panic` demos), clippy under the project's restriction lints, a plain no-features
build proving test fixtures can't leak into production code, the requirements
traceability report, and a feature-placement audit.

## Method

Everything here was built evidence-first: requirements were agreed in writing, each risky
claim became an isolated experiment with pass/fail criteria, the findings were
consolidated into `FINDINGS.md`, and only then did each design decision — with its
documented cost — get written back into `instructions.md` and implemented in the library.
Where the type system lost (silent drops are only partly preventable; conservation errors
are invisible to editor diagnostics; one trait-resolution shape crashes the compiler
outright), the loss is recorded as a finding and mitigated, not papered over.

## Status

Working and verified: the core library, the pilot model, and the full CI gate. Agreed but
not yet implemented: execution history (R16). Open questions — failure modes, cost as a
conserved dimension, qualifications and safety as types, generating work-instruction
documents back out of the model — are listed at the end of `instructions.md`.
