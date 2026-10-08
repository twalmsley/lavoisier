---
title: "Lavoisier: a critical review"
subtitle: "Evidence, assurance boundaries and the next design decision"
author: "Project review"
date: "8 October 2026"
aspectratio: 169
theme: Madrid
fontsize: 12pt
colorlinks: true
header-includes:
  - '\definecolor{ReviewNavy}{HTML}{17324D}'
  - '\definecolor{ReviewTeal}{HTML}{007F82}'
  - '\setbeamercolor{structure}{fg=ReviewNavy}'
  - '\setbeamercolor{frametitle}{bg=ReviewNavy,fg=white}'
  - '\setbeamercolor{title}{bg=ReviewNavy,fg=white}'
  - '\setbeamercolor{alerted text}{fg=ReviewTeal}'
  - '\setbeamertemplate{navigation symbols}{}'
  - '\setbeamertemplate{footline}{\hfill\insertframenumber\hspace{8mm}\vspace{3mm}}'
  - '\setbeamersize{text margin left=9mm,text margin right=9mm}'
---

# The decision in one slide

**Keep the research result. Harden the assurance boundary. Test a simpler hybrid.**

- Strong prototype for small, explicitly modelled resource flows.
- Useful compiler checks; incomplete evidence for general engineering assurance.
- The biggest next gains are trustworthy accounting, conformance and usability.
- Compare alternatives on the same problem before expanding or rewriting.

\vfill
\scriptsize Reviewed revision: 43d1f78014f52e01ae8bbf60fe234d821209a600. Full evidence: CRITICAL\_REVIEW\_REPORT.md.

# What is strong

- **Evidence trail:** experiments, negative results, findings and rule changes.
- **Useful static rejection:** wrong states, duplicate use of one token, unit mismatches.
- **Meaningful tests:** success, retry and failure accounting; negative examples.
- **Inspectable outputs:** ordinary Rust, documented assumptions, generated views.

\vfill
\scriptsize Report §3; E1–E5, E9–E13. These strengths justify continuing the project.

# The objective determines the design

| Original experiment | An engineering product |
|---|---|
| How far can Rust's types go? | Which decisions become more reliable? |
| Prefer more compiler checking | Minimise total modelling and review effort |
| Record where machinery breaks | Support real inputs, change and uncertainty |
| Type complexity can be evidence | Type complexity needs a measurable payoff |

**The present design fits the first objective better than the second.**

\vfill
\scriptsize Report §§1, 5; instructions.md, Summary and R8/R13. A change in objective should be explicit.

# Fresh validation: a green baseline

| Review run | Result |
|---|---|
| Model CI | All eight gates passed |
| Model test functions | 182 passed; 0 failed; 0 ignored |
| Nested trybuild cases | 50 expected rejections matched |
| Doctests | 64 passed; includes expected failures/panics |
| Tooling tests | 31 spec-gen + 4 diagram-gen passed |
| Specs / analysis | 6 clean specs; 0 errors, 4 warnings, 125 info items |

\vfill
\scriptsize Linux, Rust 1.98.1 with rust-src and Clippy. Nested trybuild cases are not additional top-level test functions. Report §2.

# A green gate has several meanings

| Layer | What the evidence actually supports |
|---|---|
| Rust types | Encoded state, unit and ownership constraints |
| Full compilation | Written const assertions for instantiated code |
| Executed tests | Observed behaviours and selected paths |
| Source tags and scanner | Declared links and recognised source forms |
| Measurements | Physical validity within a calibrated range |

**Passing one layer does not establish the next.**

\vfill
\scriptsize Report §§2, 4. Physical validation remains open; source tags do not establish test execution.

# Reproduced gap: an executed loss can pass

**A public supplier can expose an item with no tripwire.**

1. Draw one `DryTeabag` from a one-item box.
2. Move the bag into a named underscore binding.
3. Let it leave scope without returning it or using a consumer.

- Test passes; project Clippy restrictions also pass.
- No unsafe code, explicit drop, forget or test-support feature.
- This extends beyond the familiar untested-branch and panic gaps.

\vfill
\scriptsize R01 / P1; resource.rs, tea resources. Isolated public-API probe; not a claim that the shipped tea flow loses a bag.

# Reproduced gaps: evidence needs a trust boundary

| Probe | Observed result | Meaning |
|---|---|---|
| Ignored test with `Verifies:` | Trace script says verified; runner: 0 passed, 1 ignored | A tag is a declared link |
| Downstream `Recordable` implementation | Invented labour event enters History | Event truth trusts implementors |

**Track executed evidence and trusted event producers explicitly.**

- Baseline tests were not ignored.
- These probes test mechanisms, not the full gate for a new model crate.

\vfill
\scriptsize R03/P2 and R08/P4; trace.sh and history.rs. Reproduction details: report §9.

# Reproduced workflow defect: scaffold overwrite

**A “one-shot” generator accepts an existing output directory.**

1. Generate a CS-1 scaffold in a disposable directory.
2. Add a comment representing a hand-maintained edit.
3. Rerun the identical command.

**Observed:** exit 0; the hand edit disappears.

**Fix:** refuse existing output by default; preflight and stage writes; make replacement explicit.

\vfill
\scriptsize R04/P3; specgen.rs output loop. Probe stayed outside the repository. No existing project file was overwritten.

# Coverage and conformance are the next assurance gaps

- **Fixed analysis list:** a new workspace member does not automatically join the lint scope.
- **Separate truths:** a clean spec and clean Rust model can still describe different quantities.
- **Trusted model code:** crate-visible mint/defuse and handwritten assertions define the real boundary.
- **Shared parser:** matching diagrams and reports may share the same extraction error.

**Next:** discover membership, measure extraction coverage, and compare stable semantic contracts.

\vfill
\scriptsize R02, R05–R07. Source-inspected architectural risks; no current model-crate omission claimed.

# Where the modelling promise ends

| Useful property | Further evidence required |
|---|---|
| Balanced energy | Validated thermal model |
| One token used once | Unique real-resource identity |
| Labour budget conserved | Feasible calendar and deadlines |
| Same final types | Behavioural / temporal analysis |
| Compatible units | Meaningful physical operations |

**Consistency does not establish physical validity.**

\vfill
\scriptsize R09–R10. Temperature is not an additive conserved amount; a boiling state is a declared model state.

# The cost is not just compilation time

- Unary inventory and recursive lists have a documented scale ceiling.
- Callers maintain exact balances; failure paths multiply types and signatures.
- Source conventions and custom parsing create continuing maintenance work.
- New users must learn Rust and the project's exceptions to normal idioms.

**Scaffold survival is not effort saved:** 697 / 1,154 generated lines survived (60.4%); that is about 27.4% of the final model's lines.

\vfill
\scriptsize R11; F-010–012, F-030, F-034, F-066. Historical scale measurements were not rerun in this review.

# Options: retain, simplify, or formalise the contract

| Option | Strongest benefit | Principal cost |
|---|---|---|
| A. Harden current Rust | Preserve static checks and evidence | Existing scale/ergonomic limits |
| B. Typestate + ledger | Dynamic quantities, explicit final accounting | More runtime checks; trusted ledger |
| C. Versioned model representation | Conformance, diagnostics, multiple views | Schema and language/tool ownership |

**Recommendation:** do A now; compare a small B prototype; introduce only the portion of C the comparison needs.

\vfill
\scriptsize Report §6. These are design assessments, not measured benchmark rankings.

# Options: use the tool that answers the question

| Question | Candidate approach | Main limit |
|---|---|---|
| Feasible amounts or schedules? | SMT / constraint programming | Encoding and search cost |
| Deadlock or reachable bad state? | Petri nets / model checking | State explosion and bounds |
| Physical dynamics or domain review? | Simulation / MBSE workflow | Calibration and integration |
| Stronger universal guarantees? | Linear/dependent types / proofs | Expertise and migration |

**No alternative automatically supplies all of these guarantees.**

\vfill
\scriptsize Options D–G, report §6. TLA+, Alloy, Petri nets and optimisation solve different classes of problem.

# A hybrid worth testing

**Approved contract** → **early checks** → **Rust states + accounting ledger**

- Stable IDs, units, balances, requirements and assumption provenance.
- Checked transactions for resource creation, transfer and final disposition.
- Optional solver, temporal or simulation adapters for specific questions.
- Evidence record: revision, tools, executed checks and unresolved obligations.
- Generated views expose assumptions and extraction coverage.

\vfill
\scriptsize Report §7. A ledger adds obligations: identity, atomicity, rollback, bypass prevention and independent validation.

# What to improve first

| Change, in priority order | Acceptance evidence |
|---|---|
| Account for extracted items | Permanent P1 regression/contract |
| Protect scaffold output | Refuse existing files |
| Track executed verification | Ignored tests do not qualify |
| Discover model membership | New models cannot escape analysis |
| Broaden reproducible gate | Tools, compiler and conformance |

\vfill
\scriptsize Report §8. No fixes to existing files were made during this review.

# Make the next experiment comparative

**Same CS-6 slice. Same obligations. Independent violation cases.**

- Compare current Rust with a small ledger-based version.
- Include unseen errors: loss, overdraw, wrong state, failure accounting and spec drift.
- Measure authoring/change time, diagnostic comprehension and false positives.
- Measure build time, memory and runtime cost as inventory grows.
- Use independent modellers; agree decision thresholds before running.

\vfill
\scriptsize Report §8. Do not compare against a weaker specification or use lines of code as the primary success measure.

# Put one real process under measurement

- Separate conserved amounts, state variables, capacities, costs and observations.
- Give constants a source, uncertainty, calibration date and validity range.
- Agree acceptance tolerances before gathering validation data.
- Use held-out observations to test the physical model.

**A passed conservation check is one input to engineering confidence.**

\vfill
\scriptsize R09; report §§4 and 8. Physical validation is a proposed next step, not a completed review result.

# Choose a bounded future

**Near term:** a dependable specialist resource-flow checker and research corpus.

**Expand when:** independent users and comparative results demonstrate value.

**Integrate when:** scheduling, physical simulation or enterprise evidence workflows dominate the need.

**Defer:** more similar case studies, a whole-model macro language, or a host-language rewrite without a discriminating experiment.

\vfill
\scriptsize Report §§8 and 11. Keep the current implementation as the control and preserve its negative findings.

# Review basis and limits

- Fresh baseline: eight CI gates, 182 test functions, 64 doctests and 35 tooling tests passed.
- Four disposable probes: resource loss, ignored verification, output overwrite and trusted history extension.
- Source inspection: kernel, representative models, gates, scanner and generator.
- Not performed: exhaustive audit, new scale benchmarks, physical calibration or alternative-tool benchmarks.
- External reference sites were blocked; option descriptions are not a current tool-compatibility survey.

\vfill
\scriptsize Report §§2, 9–10 contain commands, scope limits, evidence links and further reading.

# Recommended next decision

**Continue, with a precise promise and a measured comparison.**

1. Harden accounting, evidence reporting and generation safety.
2. Compare a small hybrid on the same specification.
3. Validate one physical process with independent evidence.
4. Select the product boundary from those results.

\vfill
Full report: [CRITICAL_REVIEW_REPORT.md](../CRITICAL_REVIEW_REPORT.md).\newline
The next success metric should be better engineering decisions per unit of modelling effort.
