# Programme plan: case studies and deliverables

> The working plan for the case-study and publication phase. This document is **programme
> management, not rules**: the binding requirements live in `instructions.md` (R1–R19), the
> findings log in `FINDINGS.md`, and the specification format in `SPEC_TEMPLATE.md`. Update the
> status table as steps complete; every step ends with a quick review before the next begins.

## Working method

- Each case study starts as a **filled-in `SPEC_TEMPLATE.md`** (drafted by the implementer from
  a prose brief, reviewed by the author), lives as its **own downstream crate** in the `model/`
  workspace, and must pass the full `ci.sh` gate including zero trace.sh warnings.
- **Findings tracking:** anything a case study reveals goes into `FINDINGS.md` with the next
  F-number; when a finding is later resolved, its entry gains a dated **RESOLVED** note (the
  F-036 pattern) rather than being rewritten. The status table below links each step to the
  findings it raised.
- **Review gates:** after every step, a short review with the author: what was produced, what
  was learned, what (if anything) should change in `instructions.md` before the next step.
  Instruction changes remain agreed-before-committed.
- Deliverable formats (slide tooling, doc rendering) are decided at the step that produces
  them, not up front.

## Case-study ladder

Each case study is chosen to stress something no earlier one did. Only CS-1 is committed;
the rest are suggestions to revisit at each review gate.

| # | Case study | New stress on the method |
|---|---|---|
| CS-1 | **Making a pot of tea** | Smallest complete system. Continuous resources dominate (water in g, energy in J, time in ms); kettle as a tool with energy draw; teabags as a discrete supplier; waste heat and spent teabags to boundary sinks; History records the brew. First real spec→model round trip through `SPEC_TEMPLATE.md`, and small enough to be the walkthrough-deck example. |
| CS-2 | **Bicycle puncture repair** | Fallible processes in anger (find the hole, patch fails → second patch or new tube — provisioned rework, R17); a repair kit as nested containers; purchasing parts (R19); a qualified repairer (R18). A household-recognisable domain for the learning materials. |
| CS-3 | **Café order fulfilment** | Genuine concurrency: two staff, parallel orders, per-branch History merged at the join (R16's partial order finally exercised for real); money at the till with change; fallible steps (burnt milk → remake); multiple products in flight. |
| CS-4 | **Small-batch production run** (e.g. 25 assemblies) | Scale: capacities near the measured limits (F-010 recursion, F-011 compile-time curve), repetition via `SupplyN` at real N, stores/line as separate crates (subsystem decomposition), and a change-impact demonstration (alter one resource type, catalogue what the compiler reports). |
| CS-5 | **Two-site fulfilment with procurement** | Boundary refinement at full stretch: placeholders replaced by real suppliers, two currencies with boundary exchange (R19's only unexercised clause), transport between Locations, organisations as actors, and the model split across team-shaped crates. |

## Deliverables and order

Ordered so that each deliverable builds on what exists; a quick review follows every step.

| Step | Deliverable | Depends on | Notes |
|---|---|---|---|
| 0 | This plan | — | Done when committed. |
| 1 | **CS-1 spec** (filled `SPEC_TEMPLATE.md`, `case-studies/cs1-pot-of-tea/SPEC.md`) | template | Drafted for review; open questions back to the author per the template's §8. |
| 2 | **CS-1 model** (crate `cs1-pot-of-tea` in `model/`) | step 1 | Full gate green; findings logged; review decides instruction changes. |
| 3 | **Documentation** (item 4) | step 2 | The modeller's guide as a real document: assembled from the error-reading guide, the conventions scattered through R1–R19, and CS-1 examples; plus a rustdoc polish pass over model-core and a README refresh. Docs come before teaching materials because everything later cites them. |
| 4 | **Learning materials** (item 1) | step 3 | A tutorial sequence ("model your first system"): from blank crate to a passing gate, using CS-1; exercises with compile-error answers (the errors are the pedagogy). |
| 5 | **Overview slide deck** (item 2) | steps 2–4 | Motivation, rationale, hypothesis, goals — and the evidence so far (experiments, findings, what the compiler catches). |
| 6 | **Walkthrough slide deck** (item 2.1) | steps 2–5 | How to model, end to end, on CS-1: spec → types → processes → boundary → gate → traceability report. |
| 7 | **White paper** (item 3) | steps 2–6 | The full account: approach, method, experiments, findings F-001…, limitations, conclusions. Written as a living document (v0.x), extended as later case studies land rather than rewritten. |
| 8+ | CS-2…CS-5, each followed by doc/deck/paper increments | review gates | Chosen one at a time at review; each updates the white paper and, where it teaches something new, the learning materials. |

## Status

| Step | State | Findings raised | Review |
|---|---|---|---|
| 0 Plan | done | — | — |
| 1 CS-1 spec | drafted (`case-studies/cs1-pot-of-tea/SPEC.md` v0.1) — awaiting review | — | pending: §8 questions 1–4 |
| 2 CS-1 model | not started | | |
| 3 Documentation | not started | | |
| 4 Learning materials | not started | | |
| 5 Overview deck | not started | | |
| 6 Walkthrough deck | not started | | |
| 7 White paper | not started | | |
