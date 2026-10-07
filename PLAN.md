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
| 5 | **Overview slide deck** (item 2) | steps 2–4 | 12 slides (1 intro, 10 content, 1 summary), non-technical audience (managers, team leaders): motivation, rationale, hypothesis, goals, evidence, honest costs. Format: markdown in `decks/` (GitHub-viewable) + committed PDF built by `decks/build.sh` (pandoc → beamer → xelatex). |
| 6 | **Technical features deck** (item 2.1) | steps 2–5 | ~22 slides complementing the overview: the main features, deliberately code-light — the tutorials, guide and case studies carry that depth. Same format as step 5. |
| 7 | **White paper** (item 3) | steps 2–6 | The full account: approach, method, experiments, findings F-001…, limitations, conclusions. Written as a living document (v0.x), extended as later case studies land rather than rewritten. |
| 8 | **Diagram generation** (A) | step 2 | Generated from the models at three levels: **context** (boundary crossings only, no internals), **top-level** (first-level processes linked to each other and the boundary), **detailed** (all levels including sub-processes, fully connected). GitHub-viewable, with hyperlinks from diagram elements to the relevant code. Vehicle to decide at the step: Mermaid renders natively on GitHub but sandboxes click-links there; committed SVG with embedded links is the likely answer. |
| 9 | **Process document generation** (B) | step 8 | Candidate R20 made real: a generated, human-readable document per process — inputs, outputs, internal details, embedded diagrams, well-structured text — detailed enough for real-world implementation by people. Draws on the open-questions docgen notes (signatures as work-instruction skeletons, VALUE consts as the numbers, placeholders as the open-items list, trace report as the compliance matrix). |
| 10 | **DSL** (C) | review | A comprehensive systems-engineering DSL over the library — easier for humans to write and machines to read. Needs its own experiment batch: macro front-end vs external notation compiled to Rust vs generation from SPEC documents; error-message quality through the extra layer is the make-or-break criterion. |
| 11 | **Model analysis** (D) | review | Completeness and style analysis beyond ci.sh/trace.sh: a model linter that highlights where improvement is needed — unaccounted outputs, placeholder density, requirements with thin verification, convention violations (sealing, tag discipline), oversized processes — with actionable hints. Partly exists (trace.sh warnings, sealing greps); this step makes it a single tool with a report. |
| 12+ | CS-2…CS-5, each followed by doc/deck/paper increments | review gates | Chosen one at a time at review; each updates the white paper and, where it teaches something new, the learning materials. Steps 8–11 interleave with case studies as agreed at review gates. |

## Status

| Step | State | Findings raised | Review |
|---|---|---|---|
| 0 Plan | done | — | — |
| 1 CS-1 spec | done (v0.2 agreed) | — | 2026-10-03: §8 1–3 as proposed; disposal step added; single History, merge demo deferred to a later CS |
| 2 CS-1 model | done (`model/cs1-pot-of-tea`, full gate green) | F-053, F-054 | 2026-10-03: closed — F-053 fixed via template rule (workspace-unique ids at spec time); feedback items 2/3/5/6 folded into SPEC_TEMPLATE |
| 3 Documentation | done (`docs/modellers-guide.md`, rustdoc pass, README refresh) | — (4 doc bugs found and fixed) | 2026-10-03: closed (proceed approved; step-4 shape set) |
| 4 Learning materials | done (`docs/tutorials/` 00–06 incl. Rust on-ramp and toast capstone; `learn/` exercises crate, solutions verified) | F-015 extended | 2026-10-06: closed (deck parameters set) |
| 5 Overview deck | done (`decks/overview.md` + PDF, 12 slides) | — | 2026-10-06: approved for now |
| 6 Technical deck | done (`decks/technical.md` + PDF, 22 slides) | — | 2026-10-06: approved for now |
| 7 White paper | done (`docs/white-paper.md` v0.1 + PDF, 20 pp) | — (3 doc inconsistencies found; 2 fixed, 1 noted) | 2026-10-07: approved |
| 8 Diagram generation (A) | done (`tools/diagram-gen`, `docs/diagrams/` for both crates) | F-055 (promoted); hyperlinks fixed root-relative + absolute click URLs | 2026-10-07: approved after link fixes |
| 12 CS-2 spec | done (v0.2 agreed) | — | 2026-10-07: Q1–4, 6–7 as proposed; Q5 changed — P4 gets its own 60 000 ms draw |
| 12 CS-2 model | done (`model/cs2-puncture-repair`, full gate green; diagram-gen multi-type fix) | F-050 extended; F-055 #3 resolved | 2026-10-07: closed (cement encoding approved) |
| 12 CS-3 spec | done (v0.2 agreed) | — | 2026-10-07: all six as proposed |
| 12 CS-3 model | done (`model/cs3-cafe-orders`; stranded shot via Drain as P8 per review) | F-041 and F-055 extended | 2026-10-07: closed |
| 12 CS-4 spec | done (v0.2 agreed) | — | 2026-10-07: all six as proposed |
| 12 CS-4 model | in progress | | |
| 9 Process docgen (B) | not started | | |
| 10 DSL (C) | not started | | |
| 11 Model analysis (D) | not started | | |
