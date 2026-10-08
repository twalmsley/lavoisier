# Lavoisier: critical project review

**Review date:** 8 October 2026  
**Reviewed revision:** `43d1f78014f52e01ae8bbf60fe234d821209a600` (`main` checkout)  
**Perspective:** technical design, assurance, usability, maintainability and future direction  
**Companion presentation:** [review slides](decks/critical-review.md) · [PDF](decks/critical-review.pdf)

## 1. Overall assessment

**Lavoisier is a strong, unusually well-documented research prototype for checking small, explicitly modelled resource flows. Its evidence does not yet justify treating it as a general systems-engineering assurance platform.** The most valuable results are its experimental method, the negative findings, and a working demonstration that Rust can reject useful classes of modelling mistakes. The main risk is interpreting an internally consistent, well-tested encoding as a complete or physically valid model.

The choice of Rust is defensible for the project's stated experiment: discover how far a mainstream type system can be pushed. That objective deliberately favours compile-time machinery even when it complicates the model. It is a different objective from helping engineers make reliable decisions with the least total effort. Several rules that serve the experiment—type-level inventory, exact constants everywhere, no external modelling libraries, hand-written source scanners—should be reconsidered before becoming product requirements. [E1–E4]

The recommendation is to **retain and harden the existing implementation, then compare it with a small hybrid prototype before expanding the language or case-study ladder**. Keep Rust typestate and ownership where they clarify real invariants. Evaluate an explicit resource ledger and a versioned model representation for quantities, evidence and analysis. Add a solver, scheduler or physics model only for a specific question those tools can answer better. Do not undertake a wholesale rewrite on the basis of this review.

### What deserves to survive

- The experiment → finding → rule → implementation trail, including unsuccessful approaches.
- Explicit boundary assumptions, separate resource states, and positive and negative examples.
- Integration tests that account for outputs on success, retry and failure paths.
- The distinction between specification checking and model implementation, and the decision to expose underdetermined choices as holes.
- Ordinary, inspectable generated Rust, readable diagnostics and generated documentation.

### What should change first

1. Tighten the assurance claims and cover the demonstrated loss and traceability gaps.
2. Protect hand-maintained files from accidental scaffold regeneration.
3. Make analysis coverage and verification evidence explicit and machine-checkable.
4. Establish a measurable relationship between the approved specification and the maintained implementation.
5. Validate one real process and compare alternatives with independent users before adding breadth.

## 2. Scope, method and evidence

This review inspected the root requirements, findings, plan and white paper; the model kernel; representative tea, café and bread models; the model CI and traceability scripts; the extraction/lint tooling; the specification parser, generator and tests; and the documented DSL and change-impact experiments. It is a targeted architectural and assurance review, not an exhaustive audit of every process or every historical experiment.

All existing tracked files were fingerprinted before work. Review probes, build outputs and logs were placed outside the checkout in `/workspace/review-work`. Only this report and the two new presentation files are deliverables in the repository. No existing source, tests, dependency declarations, lockfiles, documentation or deck build scripts were edited.

### Checks rerun for this review

| Check | Result | What the result establishes |
|---|---|---|
| `./model/ci.sh` | All eight gates passed | Current model workspace builds and satisfies its implemented gates on Linux with Rust 1.98.1 |
| Model test runners before doctests | 182 passed; 0 failed; 0 ignored | Includes the trybuild driver tests |
| Trybuild cases inside those drivers | 50 expected compile failures matched | Nested cases; do not add them to the driver count as if independent top-level tests |
| Model doctests | 64 passed; 0 failed; 0 ignored | Includes positive, expected-panic and expected-compile-failure examples |
| `tools/spec-gen` tests | 31 passed | Includes fixture mutations, deterministic generation and generated-crate checks |
| `tools/diagram-gen` tests | 4 passed | Existing extraction/tooling tests; not evidence of complete parser coverage |
| Analysis reports | 0 errors, 0 acknowledged errors, 4 warnings, 125 information items | Warnings and information are allowed by the existing gate |
| Specification gate | Six specifications clean; 0 warnings | Document-level rules pass; implementation equivalence is not tested by this result |

`rust-src` was installed alongside Clippy and rustfmt; the diagnostic snapshots require standard-library source excerpts. The tests above are fresh review runs, separate from the earlier environment setup. A passing negative test means the expected rejection occurred, not that the underlying invalid program succeeded. No ignored baseline tests were observed. The deliberately broken learning exercises, all historical experiments, release-profile model behaviour and scale benchmarks were not rerun. [E5–E7]

Findings below distinguish **reproduced behaviour**, **source-inspected behaviour**, **documented evidence**, and **design inference**. The isolated probes test particular mechanisms; they were not submitted to the entire repository gate as new model members. Their scope matters.

## 3. What is good

### 3.1 The research discipline is stronger than the headline claim

F-001 through F-066 preserve failures, costs and adaptations rather than presenting a polished success narrative. The project distinguishes type checking from monomorphization, records the affine/linear gap, documents feature unification, and reports compiler and generator failures. This is substantive engineering evidence, particularly valuable for others considering similar designs. The documented change-impact experiment also records the per-crate waves of diagnostics instead of claiming one complete impact report appears immediately. [E2, E8]

There is nevertheless a limit to the word “evidence”: most comparisons are within this project's own constraints, examples and authorship. They support local design choices, not superiority over alternative modelling methods.

### 3.2 Ownership and typestate catch relevant mistakes cheaply

Once an appropriate resource value exists, moving it prevents a second use of the same value. Distinct states prevent a cold kettle from being used where a boiling state is required. Sealed representations prevent ordinary downstream construction. Units and currency types catch certain category errors. These are useful guarantees that survive refactoring and do not require a runtime scenario to encounter the invalid operation. [E3, E9]

The most convincing application is a small, stable process with identifiable resources and a finite set of legal states. The type signatures act as precise interfaces between subsystems. This is a credible niche even if the broader MBSE ambition is narrowed.

### 3.3 The test strategy acknowledges multiple checking layers

The eight-step gate goes beyond a nominal build: full code generation, unit/integration tests, compile-fail tests, Clippy restrictions, an ordinary production build, traceability, analysis and specification validation. The separate production build is a particularly good response to test-support feature leakage. The bread tests exercise both flow orders and all four two-token outcomes, and the café tests inspect branch histories and accounting. These are meaningful checks rather than superficial smoke tests. [E5, E10]

### 3.4 The specification gate is independently valuable

Checking arithmetic, names, waste destinations, requirement IDs and draw consistency in the document can catch errors before model code exists. This benefit does not require adopting Rust as the universal modelling medium. It could become a useful standalone review tool for teams using other engineering tools. [E11]

The one-shot scaffold is also a sensible compromise: it avoids pretending that natural-language requirements determine every modelling decision. F-066's generator defects and the new regressions demonstrate the value of field testing and of making uncertainty visible. [E2, E12]

### 3.5 Documentation is treated as part of the system

The guide, examples, tutorials, generated work instructions, diagrams and findings lower the cost of understanding an unusual approach. Shared extraction code reduces disagreement between documentation tools. This is much better than a library whose only explanation is its type signatures. However, shared extraction also creates a common failure mode; agreement among generated outputs is not independent corroboration. [E7, E13]

## 4. Critical findings and potential problems

Severity is relative to using the project for engineering assurance. **High** means a false assurance claim or loss of user work is plausible; **medium** means an important limitation on scale, maintenance or adoption. These are not claims of remote security vulnerabilities.

### R01 — Executed resource loss can survive the current lint regime

**Priority: high. Evidence: reproduced public-API behaviour, plus source inspection.**

`DryTeabag` is deliberately declared `no_tripwire`. Its box and loaded pot are intended to account for it, but the public supplier can return an individual bag. The review drew one bag from `full_teabag_box::<N1>()`, moved it into a named underscore binding and allowed it to leave scope without a consumer. The test passed. Clippy also passed with the repository's `clippy.toml`, `-D warnings`, `clippy::mem_forget` and `clippy::let_underscore_must_use`. No unsafe code, feature-enabled fixtures, forgotten memory or explicit `drop` was needed. [E3, E9; probe P1]

This is an executed loss, not merely an untested-branch or panic-unwinding gap. The `no_tripwire` design is documented; the issue is its consequence for the wider claim that executed losses are caught by the complete regime. `#[must_use]` does not require a named binding to reach a legitimate consumer. Reusable types and bare `Qty` values also lack the universal final accounting that the broad slogan can imply; those observations are source-inspected, not additional reproduced scenarios.

**Action:** enumerate every resource family's enforcement level. Either make extracted items independently accountable, constrain extraction through an accounting owner, or introduce a ledger that verifies resource disposition at flow completion. Add a negative regression for the exposed-bag case. Narrow “nothing is lost” statements immediately. Blanket prohibition of underscore-prefixed names would not solve the underlying affine semantics.

### R02 — The trusted modelling code is larger than the library kernel

**Priority: high for assurance claims. Evidence: source-inspected boundary and API design.**

Macro-generated `mint` and `defuse` methods are `pub(crate)`. The hard boundary is the defining crate, not exclusively the named boundary module. Every modelling crate that defines resources therefore contains trusted code capable of creating or retiring them. Continuous transformations commonly defuse inputs and mint outputs; their correctness depends on the author writing the right quantities, assertions and state transitions. The compiler checks the supplied encoding, not that every necessary invariant was encoded. [E3, E9]

Public boundary constructors can be called repeatedly. Unless the model separately controls entry authority or identity, two independently created `Person` values can stand for the same real technician. Rust prevents reusing one value; it does not prove that the model contains one unique value per real person. Likewise, Rust's type system does not make arbitrary process functions pure or prove that there is no hidden state or external effect.

**Action:** publish a trust-boundary map: compiler, kernel macros, each resource-defining crate, each constructor, each balance assertion, each model assumption and each generator. Where practical, narrow constructor capabilities and make transformations use reviewed accounting primitives. Use explicit identities and scoped admission for models where duplicate real-world representation matters. Do not describe privacy as a proof that modelling code cannot create fictitious resources.

### R03 — “Verified by” currently means “has a source tag”

**Priority: high. Evidence: reproduced trace-script behaviour.**

`trace.sh` associates `Verifies:` with the following named item while skipping attributes. It does not establish that the item is an enabled, executed test, that it contains a relevant assertion, or that the assertion distinguishes a violation. A copied script, run against a tiny fixture containing one `#[ignore]` test, reported “No warnings: every requirement has at least one verifying test.” The Rust test runner reported **0 passed, 1 ignored**. [E6; probe P2]


This does not show that the current 182 baseline test functions are ignored; none were. It shows that the traceability gate cannot substantiate the stronger verification claim on its own. The linter's “fewer than three verifying tests” heuristic is similarly a measure of tags, not strength of evidence. A hundred tests with the wrong oracle are weaker than one discriminating test.

**Action:** distinguish declared links, discovered tests, executed tests and demonstrated properties in the report. Match stable test identities to test-run outcomes, handle conditional compilation and ignored cases, and add mutation tests that remove or corrupt each important invariant. Assertions generated from the same mistaken interpretation as the implementation should not be treated as an independent oracle.

### R04 — A one-shot generator can overwrite its hand-maintained output

**Priority: high for developer workflow. Evidence: reproduced file overwrite.**

`specgen` documents promotion to hand-maintained code, but writes generated files using `std::fs::write` without refusing an existing destination. In a disposable directory, the review generated a CS-1 scaffold, appended a sentinel representing a hand edit to `src/lib.rs`, and ran the same generator command again. Both runs exited successfully and the edit disappeared. No repository file was used as the probe destination. [E12; probe P3]

**Action:** default to refusing any pre-existing output files, preflight the whole operation, generate into a fresh staging directory, and make replacement an explicit mode with a clear ownership policy. Failure halfway through writing should not leave a mixture of generations. This is a small, high-value improvement independent of the architectural direction.

### R05 — Analysis coverage depends on a fixed crate list

**Priority: high when adding models. Evidence: source inspection; no current crate omission claimed.**

`default_crates()` lists the ten current model crates explicitly. `modellint` uses it when no crate arguments are supplied; the CI wrapper invokes that default. The current list covers the current intended model crates, but adding a Cargo workspace member does not automatically add it to this analysis surface. Its report can be absent while the existing reports remain unchanged. The Rust build and analysis tool therefore do not derive their scope from the same source of truth. [E7]

`trace.sh` takes the opposite approach: it searches source/test files below its directory rather than resolving Cargo membership and active targets. That can include inactive source material. Neither approach alone establishes correspondence with the compiled workspace.

**Action:** derive membership from Cargo metadata or a checked model manifest, explicitly classify infrastructure, and fail when a member is neither analysed nor deliberately excluded. Record numbers of expected and observed resources, processes, requirements and flows so an empty or partial extraction cannot masquerade as complete coverage.

### R06 — A clean spec and a clean implementation can still disagree

**Priority: high for requirements assurance. Evidence: source-inspected pipeline; documented generator history.**

The specification checker validates the document. The Rust gate validates the implementation and selected instantiations. After one-shot scaffolding, neither establishes a general semantic equality between them. A spec quantity and the corresponding Rust quantity can each be changed to a different, internally balanced value and satisfy their respective arithmetic checks. The CI sequence contains no general comparison that would reject that divergence. This is an architectural inference from the checking boundaries, not a mutation reproduced against the entire repository. [E5, E11, E12]

F-066 is concrete evidence that a clean document did not guarantee a conserving scaffold. The recorded defects were fixed; this review does not claim they remain. Their significance is that the parser/emitter and its assumptions are part of the assurance mechanism. Generating code and tests together can reproduce the same misconception twice.

**Action:** maintain a small, versioned semantic manifest with stable process/resource/requirement IDs, units, quantities, assumptions and provenance. Compare independently extracted model contracts against the approved manifest for the subset intended to correspond. Report unsupported comparisons as unknown. Keep a human approval step for semantic changes; a textual diff or source-line breadcrumb is insufficient.

### R07 — Hand-written source parsing has become architectural infrastructure

**Priority: medium, high where generated documents are treated as complete. Evidence: source inspection and existing findings.**

The line-based scanner and flow tracer recover a great deal from disciplined source. They also constrain layout, bound placement, macro use and flow shape. F-055, F-060 and the current four scanner warnings show the cost of that choice. One current warning concerns an unresolved café order parameter; another an unresolved stores parameter. The pilot's two warnings concern an assertion-only match. They are not equivalent in meaning and should not be collapsed into one generic “warning count.” [E2, E7, E13]

The no-dependency rule has shifted work into a custom partial Rust parser. This is not automatically simpler over the project's lifetime. A syntax-tree library could improve syntactic robustness, but it would not by itself solve macro expansion, name resolution, trait semantics or conditional compilation.

**Action:** measure the supported source subset and reject unsupported constructs that threaten completeness. Prefer stable IDs and explicit contracts over adding another formatting convention for each new case. Evaluate a parser library or compiler-supported data where it reduces maintenance; retain semantic cross-checks. Fail documentation publication, rather than necessarily all local development, when a relevant process or flow is unaccounted for.

### R08 — History records are extensible declarations, not independent evidence

**Priority: medium; high if used as an audit claim. Evidence: reproduced public extension-point behaviour.**

`Recordable` is a public trait. A downstream local type can implement it and return an arbitrary `Event`; `record()` supplies the permit and accepts a caller-provided process name. The review recorded 999 person-milliseconds for a process that never ran, without consuming a `Labour` value. That test and the Clippy restrictions passed. This is a trust extension point, not a Rust memory-safety defect. [E14; probe P4]

Even honest history entries describe a model execution; they do not establish a physical event happened. The series-parallel join structure is useful, but does not automatically represent arbitrary cross-branch dependencies or a measured timeline.

**Action:** document which `Recordable` implementations are trusted. If history must support audit, use typed process identities, provenance and controlled event creation tied to accounting transitions; keep observational measurements distinct from model events. Sealing the trait would trade extensibility for stronger control, so decide that based on the product's assurance target.

### R09 — Types encode declared physics, not the validity of that physics

**Priority: high for real-world decisions; accepted research limitation. Evidence: model source and documented placeholders.**

CS-1's `boil` checks an energy sum and moves the water mass through. It does not derive a boiling condition from initial temperature, pressure, heat capacity and losses. A “boiling” state is a modelling assertion. A unit-safe energy number can still be physically wrong. Qualifications encoded as marker traits similarly express a model's declaration, not certificate validity or present human competence. [E9, E15]

Exact unsigned integers are useful for reproducible bookkeeping, but do not represent measurement uncertainty, intervals, negative flows or arbitrary dynamic input. A physically measured quantity has a resolution and uncertainty, not a universal “smallest unit.” Temperature is intensive: adding two absolute temperatures is not a physical conservation law. Yet generic `split`/`combine` accepts any `Unit`, including `Millikelvin`. Dimensional compatibility and conservation semantics need separate treatment. This is an API-design observation; no temperature case study was assessed. [E16]

Money at a stated exchange rate, labour consumed, reusable availability, mass and energy should not all be described as the same conservation theorem. Rates, durations, thermodynamic state and financial rounding need distinct models and evidence.

**Action:** classify quantities as conserved amounts, state variables, capacities, costs or observations. Track units, provenance, calibration date, uncertainty and validity range for constants. Validate one representative process against measurements, with acceptance tolerances agreed before collecting results. Do not choose parameters just because they make integer arithmetic convenient without recording the resulting approximation.

### R10 — Resource exclusivity does not establish a feasible schedule

**Priority: medium; high if planning claims extend to delivery dates or throughput. Evidence: source and stated scope.**

Move semantics exclude sharing one modelled token. Time budgets bound total labour use. Neither provides start times, deadlines, calendar availability, machine capacity over intervals, travel time, queueing, fairness or deadlock analysis. Two sequential encodings ending in the same types do not prove observational equivalence or all concurrent interleavings. Café tests provide useful evidence for their selected orderings and histories, not a general concurrency theorem. [E1, E10, E14]

This is largely an intentional scope boundary, not a demand that the current project become a scheduler. However, statements about double-booking or “real concurrency” should say what is actually represented. Hidden shared constraints—power supply, floor space, common ventilation—remain absent unless explicitly modelled.

**Action:** keep budget accounting distinct from elapsed time. Export process/resource constraints to a scheduling solver or timed/coloured Petri-net model when the question becomes feasibility, throughput or contention. State environment and fairness assumptions for temporal properties.

### R11 — Complexity is being paid in author effort and compiler behaviour

**Priority: medium. Evidence: documented measurements, inspected signatures; no new scale benchmark.**

Peano capacities and heterogeneous lists make exhaustion structurally visible, but impose recursive type depth proportional to inventory. The recorded practical guidance around 100–500 items and the documented F-034 compiler failure are warning signs for industrial scale, not measurements reproduced here. Const-generic remainders make callers maintain balances; branching outcomes multiply signatures and final-state types. [E2, E8]

The choice can be worth it for a small model or teaching experiment. It is less attractive for thousands of interchangeable items, live inventory, uncertain yield, runtime quantities or frequently changing specifications. The Rust-literate modeller must also learn project-specific rules that sometimes conflict with compiler suggestions.

The 60.4% scaffold-survival result is useful, but means 697 of 1,154 generated lines survived. Those lines are approximately **27.4% of the final 2,547-line model**, and neither fraction measures engineering hours saved or correctness improved. Code density and compiler rejection counts are inadequate product-success metrics. [E2, F-066]

**Action:** benchmark authoring time, change time, diagnostic interpretation, compile time, peak memory and defect detection across increasing inventories and branches. Compare against a simpler implementation of the same contract. Let results justify the additional type machinery.

### R12 — Reproducibility and regression coverage need a product-level gate

**Priority: medium. Evidence: repository inspection and fresh validation.**

The repository states Rust 1.98.1 and includes lockfiles, but this revision contains no checked-in Rust toolchain pin. The environment is pinned externally; another clone need not be. Exact diagnostic snapshots also depend on installed components such as `rust-src`. The main CI script builds the tooling it uses but does not run the 31 specification-tool tests or four diagram-tool tests. No checked-in hosted CI workflow was found; external CI may exist and was not inspected. [E4–E7]

Rustdoc `compile_fail` examples can pass for an unrelated error, as F-003 acknowledges. A positive twin plus checks on the intended diagnostic would give stronger evidence. Analysis staleness is checked; the main gate does not similarly regenerate-and-diff all diagrams, process documents or PDFs. Growing documentation counts and repeated status summaries already create small drift: the generated analysis README refers to nine modelling crates while listing ten, and `PLAN.md` retains both “not started” and “done” DSL rows. These are secondary maintenance symptoms, not core algorithm failures.

**Action:** specify and test the supported toolchain/components, run tool suites in the normal gate, add negative-test discrimination and coverage checks, and centralize generated counts/status where feasible. Automate reproducible documentation checks according to publication needs. Preserve the current distinction between intentionally broken exercises and readiness tests.

## 5. The central design question

### Is a different approach more appropriate?

**For the original research question, the current design is appropriate. For a general engineering workflow, a hybrid is the stronger candidate, but it needs comparative evidence.** There is no single alternative that automatically supplies ownership, uncertainty, scheduling, dynamics, traceability and usability.

Choose the dominant question first:

| Engineering question | Most relevant mechanism |
|---|---|
| Can this process receive the wrong state or reuse one exclusive token? | Rust typestate/ownership, or another suitable type discipline |
| Does every resource transfer balance, including data-dependent values? | Explicit accounting ledger with invariant checks; symbolic constraints where needed |
| Can this network deadlock or reach an undesirable state? | Petri nets or an explicit transition-system model checker |
| Can work finish by Friday with these machines and calendars? | Constraint programming / scheduling optimisation |
| Is this heat, pressure or energy model physically plausible over time? | Validated equations and simulation, with empirical calibration |
| Are requirements, architecture, interfaces and evidence reviewable across disciplines? | A model repository/MBSE workflow with explicit evidence links |
| Is a property universally true over a defined formal model? | A proof or suitably scoped exhaustive analysis, with assumptions stated |

“Move checks into the compiler” is a means. The desired outcome is an engineer making a correct decision with comprehensible, reviewable evidence.

## 6. Design options and tradeoffs

The following are design assessments, not benchmark rankings. External tools were not installed or compared experimentally in this review. The reference sites in §10 were blocked by this environment's egress policy, so they are supplied as further-reading entry points, not as a newly verified literature survey or current-version compatibility claim.

### Option A — Harden the present Rust design

**Approach:** retain type-level quantities and inventory, constrain the supported model subset, fix R01–R08, improve CI and declare the trusted model boundary.

**Best fit:** research, teaching, small fixed processes, software-literate teams and stable interfaces.

**Benefits:** least migration; preserves working tests, curated diagnostics and evidence; produces a credible, bounded method quickly.

**Costs:** runtime variability, uncertainty, inventory scale and authoring burden remain. A longer lint list does not turn affine Rust into linear Rust. More macros would increase the scanner's maintenance burden.

**Decision trigger:** choose this as the long-term product only if target users' models fit the supported envelope and user trials show the complexity is worth the errors caught.

### Option B — Hybrid Rust typestate plus explicit accounting (recommended experiment)

**Approach:** keep state/capability types and move-only handles, but store varying amounts and inventory in a transaction-scoped ledger. A process consumes identified inputs and creates outputs through checked transitions. Closing a run reports unaccounted resources, boundary transfers and failed invariants. Represent optional static facts in types without requiring every quantity or item count to be a type parameter.

**Best fit:** practical resource-flow checking with dynamic quantities and larger inventories.

**Benefits:** better arithmetic ergonomics; scalable homogeneous inventory; an explicit final-accounting oracle; structured event data; a natural place for uncertainty and provenance. A ledger can reveal loss even when a token's destructor has no tripwire.

**Costs:** many checks move to validation or execution time. Atomicity, rollback, identity, concurrency and ledger bypass become trusted implementation concerns. A ledger proves only the runs or symbolic states it checks. The design needs transaction abort semantics rather than assuming every panic can be safely continued.

**Migration:** first implement one CS-1 or CS-6 slice beside the current model in a separate experimental crate. Reuse the accepted specification and independently written violation cases; compare outputs and diagnostics. Leave the current corpus as the control. Do not silently reinterpret R8/R13; this would be an explicit change in design objectives.

### Option C — Versioned model representation with a small DSL and generated views

**Approach:** make a typed, versioned intermediate representation the machine-readable contract. Give every resource, process, requirement and assumption a stable ID. Generate readable Rust, reports and analysis inputs, with extensions outside generated files.

**Best fit:** multiple analysis backends, non-Rust authoring, and durable spec/model traceability.

**Benefits:** conservation and name checks can occur before Rust code generation; reduces repeated semantic inference from source text; supports editor diagnostics and structured provenance. This builds on the existing spec parser, `Spec`/generator data and EXP-14/15, rather than pretending the project currently has no internal representation.

**Costs:** a language, schema migration and editor/tooling product must be maintained. A common representation can propagate one mistake to every output. Independent validators remain necessary. Escape hatches and lossless round-tripping are difficult.

**Decision trigger:** proceed only after a bounded semantic contract demonstrates enough benefit. Do not expand Markdown conventions into a full programming language by accident. Adopt strict generated-file ownership, or keep one-shot scaffolding with explicit conformance checks; avoid an ambiguous mixture.

### Option D — Constraint solving / optimisation for quantities and planning

**Approach:** express balances, bounds, assignments and selected scheduling constraints symbolically; use an SMT solver such as Z3 or a constraint-programming solver such as OR-Tools CP-SAT according to the problem class.

**Best fit:** inferring feasible amounts instead of hand-writing remainders, finding counterexamples, calendars/capacities, and optimising a stated objective.

**Benefits:** constraints can state what must hold without spelling every intermediate type; solver output can identify infeasible combinations or candidate plans.

**Costs:** solver encodings and result interpretation are trusted. Nonlinear equations, huge search spaces and weak explanations can be difficult. Bounded analysis, timeouts and unknown results must stay distinct from proof. It does not automatically guarantee that an implementation follows a solver's model.

**Decision trigger:** use it for a concrete constraint question with a known validation oracle. A solver backend is a complement to resource identities and evidence management, not their replacement.

### Option E — Coloured/timed Petri nets or transition-system model checking

**Approach:** represent resources as tokens, processes as transitions, and relevant time or state explicitly. Use a Petri-net tool for resource-flow analysis, TLA+ for temporal/state-machine reasoning, or Alloy for scoped structural/relational analysis where appropriate; these are different formalisms, not interchangeable engines.

**Best fit:** reachability, contention, deadlock, bounded retry, concurrency and protocol properties.

**Benefits:** resource movement is close to the problem domain; explicit state exploration can examine behaviours beyond two hand-written orderings. Invariants and counterexample traces make the assurance target visible.

**Costs:** state explosion, modelling expertise, bound/fairness assumptions and correspondence with Rust code. A Petri net does not automatically model realistic physics; a TLA+ specification does not automatically yield a proof; Alloy's scoped checks do not establish arbitrary unbounded claims.

**Decision trigger:** add this when an engineering requirement is temporal or about all reachable states, rather than trying to encode it as another marker trait.

### Option F — Established MBSE and physical simulation tools

**Approach:** use a SysML/Capella-style workflow for architecture, interfaces and requirements, and Modelica or a suitable simulation tool for continuous physical behaviour. Connect Lavoisier through explicit contracts or exports where it provides useful resource checks.

**Best fit:** multidisciplinary systems, established enterprise review practices, physical dynamics and integration with existing engineering data.

**Benefits:** richer domain vocabulary and workflows; subject-matter experts need not read long Rust types; physical equations and observations can be first-class.

**Costs:** tool integration, possible licensing/vendor constraints, configuration governance and consistency between representations. Diagrams alone prove little. Simulation validates sampled scenarios under assumptions; uncertainty and calibration still matter.

**Decision trigger:** make Lavoisier a focused checker within an existing workflow when replacing the team's architecture/evidence system would cost more than the additional checking provides.

### Option G — Linear/dependent types or deductive verification

**Approach:** investigate a genuinely linear discipline, dependent proofs, or a verification layer that can express stronger contracts than ordinary Rust types.

**Best fit:** the research objective becomes proving complete resource use or universal arithmetic properties within a precisely defined formal model.

**Benefits:** potentially stronger statements with explicit proof obligations; avoids some ad hoc lint/tripwire conventions.

**Costs:** substantial rewrite or annotation effort, specialist skills and narrower ecosystems. Linearity alone does not prove balanced quantities, correct boundary assumptions or realistic physics. Exceptions, unrestricted values, foreign code and effects require careful treatment. Haskell's linear-arrow extension, for example, should not be treated as automatic whole-program linearity.

**Decision trigger:** revisit only when the assurance requirement warrants proof engineering and a small prototype demonstrates better total cost. Nightly Rust arithmetic would address part of the ergonomics problem, not the loss, truth or traceability gaps.

### Comparative decision table

| Option | Main gain | Main sacrifice / new obligation | Relative migration | Recommendation |
|---|---|---|---|---|
| A. Harden Rust | Preserve static checks and existing evidence | Existing modelling envelope remains narrow | Low | Do now |
| B. Hybrid ledger | Dynamic data, explicit accounting, inventory scale | More runtime checking; ledger correctness | Medium | Compare next |
| C. Versioned representation | Traceability and multiple backends | Language/schema/tool ownership | Medium–high | Start with a small contract |
| D. Solver | Feasibility, inferred quantities, schedules | Encoding, boundedness, explainability | Medium for a slice | Add for a concrete question |
| E. Nets/model checking | Reachability and temporal behaviour | State-space and correspondence costs | Medium–high | Use when concurrency is the question |
| F. MBSE/simulation | Domain workflow and physical behaviour | Integration and tool governance | Context-dependent | Integrate for industrial adoption |
| G. Stronger formal host | Stronger proof obligations | Highest expertise and migration burden | High | Research option, not default rewrite |

## 7. A possible architecture worth testing

The recommended experiment is a small extension of existing ideas, not an immediate platform build:

1. **Approved contract:** resource/process IDs, states, units, balances, requirement links, boundary assumptions and source provenance in a versioned representation.
2. **Early validator:** schema, accounting, dimensional and completeness checks; every unsupported statement becomes an explicit unresolved obligation.
3. **Execution/checking backend:** Rust typestate for discrete protocol states, plus ledger transactions for variable quantities and inventory.
4. **Optional analysis adapters:** solver for feasibility; temporal model for liveness/contention; simulation for physical dynamics.
5. **Evidence record:** exact model revision, toolchain, assumptions, executed checks, counterexamples and measured-data provenance.
6. **Generated views:** prose, diagrams and reports annotated with coverage and unresolved assumptions.

The semantic contract must say what each backend preserves. Two backends agreeing is not independent evidence if they share the same faulty translation. Use hand-calculated cases, mutation fixtures and measured data as separate oracles. Start with one process family; resist building an all-purpose ontology before establishing value.

## 8. Prioritised path forward

These are proposed, acceptance-gated stages rather than promises about delivery dates. Roles describe responsibility, not staffing commitments.

| Stage | Work | Suggested owner | Exit evidence |
|---|---|---|---|
| 1. Correct assurance boundaries | Resolve or explicitly bound R01; relabel traceability; document trusted constructors/history; guard `specgen` output | Kernel/tooling maintainer | P1–P4 become permanent regression or documented contract tests; generated output refuses overwrite; claims match enforcement |
| 2. Make coverage observable | Discover/classify workspace members; join requirement links to executed tests; include tooling suites; pin reproducible compiler/components | Tooling/CI maintainer | Added model cannot silently escape analysis; ignored test cannot satisfy an execution claim; fresh clone passes documented gate |
| 3. Test comparative value | Same CS-6 slice in existing Rust and a ledger-based alternative; independent mutation corpus; two or more independent modellers if available | Review lead + external modellers | Authoring/change time, error interpretation, defect detection and performance recorded for both approaches |
| 4. Validate reality | Measure one actual process and register assumptions, uncertainty and acceptance tolerances | Domain engineer | Held-out measurements satisfy the agreed acceptance criteria or trigger explicit model revision |
| 5. Choose the product boundary | Decide research toolkit, specialist checker, or broader modelling product | Project owner | Decision record naming users, excluded problems, supported scale, maintenance budget and rejected options |

### Suggested comparison protocol

- Use the same accepted inputs, processes, outcomes and accounting obligations; do not make the simpler alternative solve a weaker problem.
- Seed wrong units, wrong states, duplicate allocation, missing disposal, overdraw, incorrect failure accounting, spec/model disagreement and unsupported syntax. Record **which checking phase** catches each, and any false positives.
- Separate valid examples used to develop the tool from unseen examples used to evaluate it.
- Measure initial modelling time and at least two requirement changes; include correction time and whether a domain reviewer can explain the diagnostic.
- Evaluate inventory sizes such as 10, 100, 1,000 and 10,000 where meaningful, recording build time, peak memory and runtime checking cost. Stop failing runs explicitly; do not count timeout as rejection of the intended invariant.
- Count required manual balances and annotations, but give more weight to time and correctness than lines of code.
- Agree the decision thresholds before running the comparison. A reasonable starting requirement is detection of every seeded high-priority accounting error, no unexplained false-green cases, and a measurable reduction in change effort without unacceptable feedback latency. These are proposed criteria, not achieved results.

### Opportunities worth pursuing

**A narrowly scoped verification product.** A resource-flow checker for stable procedures, conservation-heavy transformations or subsystem interfaces may be more useful and supportable than a universal MBSE language.

**A specification review tool.** `speccheck` can be useful independently. Structured diagnostics, editor integration and explicit unresolved obligations could provide value before a team writes Rust.

**A research benchmark and educational corpus.** The experiments and counterexamples could become a comparative corpus for type systems, modelling tools and compiler diagnostics. Publish reproducible parameters and independent replications rather than only success narratives.

**Evidence-aware generated documents.** Work instructions and diagrams that clearly mark checked facts, assumptions, unsupported extraction and calibration sources would be more defensible than polished outputs that look uniformly authoritative.

### Work to defer

Defer another near-neighbour case study unless it attacks an unresolved risk. Defer a whole-model macro language, a wholesale host-language rewrite and a broad MBSE platform until the comparison justifies them. Do not spend the next increment merely increasing test counts, generated pages or rule counts; those can grow without improving assurance or usability.

## 9. Reproduced probes and how to repeat them

These probes deliberately construct counterexamples to broad interpretations of the API. They do not demonstrate that the shipped tea flow currently loses a bag, that existing histories are fabricated, or that baseline tests are ignored. They establish where enforcement ends.

Create a disposable directory **outside the repository**, with a standalone Rust crate depending by path on `model/model-core` and `model/cs1-pot-of-tea`. Use edition 2024, no `test-support` features, and an empty `[workspace]` section. Copy `model/clippy.toml` into that disposable crate. Activate Rust 1.98.1 and its Clippy component. The dependency paths must point to the reviewed checkout.

### P1: an extracted, untripwired item can disappear

Use the following in the disposable crate's `src/lib.rs`:

```rust
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![deny(let_underscore_drop)]
#![recursion_limit = "2048"]

#[cfg(test)]
mod tests {
    #[test]
    fn extracted_item_is_not_accounted_for() {
        use cs1_pot_of_tea::resources::boundary::full_teabag_box;
        use model_core::boundary::take_one;
        use model_core::nat::aliases::N1;

        let (bag, empty_box) = take_one(full_teabag_box::<N1>());
        let _lost_bag = bag;
        let _retained_empty_box = empty_box;
    }
}
```

Run `cargo test`, then `cargo clippy --all-targets -- -D warnings -D clippy::mem_forget -D clippy::let_underscore_must_use`. **Observed: both exited 0.** The bag reaches neither a returned result nor a consumer. This is a positive execution of the review counterexample, not an expected-failure regression already in the project.

### P2: an ignored test satisfies the textual trace gate

In another disposable directory, copy `model/trace.sh` unchanged and create `src/lib.rs`:

```rust
/// REQ-999: this requirement has no executed verification.
pub trait Req999 {}
/// Verifies: REQ-999
#[test]
#[ignore]
fn never_runs() { panic!("not executed"); }
```

Run the copied script. Compile the fixture with `rustc --edition=2024 --test src/lib.rs -o tests` and execute `./tests`. **Observed:** trace exit 0, “every requirement has at least one verifying test”; runner exit 0, **0 passed, 1 ignored**. This directly tests `trace.sh`, not the full model-analysis gate.

### P3: scaffold generation replaces a hand edit

Run the built `specgen` against `case-studies/cs1-pot-of-tea/SPEC.md`, using a fresh disposable output directory and `--model-core` pointing to the existing kernel. Append a unique comment to the generated `src/lib.rs`. Rerun the identical command. **Observed:** both exits 0; the comment disappeared; the file returned to its original generated content. Never use an existing project directory to repeat this probe.

### P4: history trusts downstream record implementations

Inside the disposable crate's test module, the following test passed, as did the same Clippy checks:

```rust
#[test]
fn recordable_is_a_trusted_extension() {
    use model_core::history::{Event, Permit, Recordable, UNATTRIBUTED};
    use model_core::history::{boundary::new_history, processes::record, Entry};
    struct InventedRecord;
    impl Recordable for InventedRecord {
        fn into_record(self, _: Permit) -> Event {
            Event {
                process: UNATTRIBUTED,
                item: "Labour",
                magnitude: 999,
                unit: "person-milliseconds",
            }
        }
    }
    let h = record(new_history(), "never_executed", InventedRecord);
    assert_eq!(h.event_count(), 1);
    assert!(matches!(&h.entries()[0], Entry::Event(e) if e.magnitude == 999));
}
```

`Permit` controls access to recording machinery; it does not certify the truth of a downstream implementation's event. That distinction should be part of the public contract.

## 10. Evidence index and further reading

Repository references describe the reviewed revision. Paths are relative to this report; symbol names and finding IDs make the references usable after modest line movement.

| ID | Source | Review relevance |
|---|---|---|
| E1 | [instructions.md](instructions.md), Summary and R1–R22 | Experimental objective; binding modelling conventions and admitted limits |
| E2 | [FINDINGS.md](FINDINGS.md), especially F-001–008, F-010–012, F-030, F-034, F-040, F-053–066 | Empirical constraints, generator defects, scale and scanner limits |
| E3 | [resource.rs](model/model-core/src/resource.rs), `consumable_resource!`, generic `no_tripwire` emission, `mint`/`defuse`, reusable macros | Actual resource enforcement and trusted scope |
| E4 | [README.md](README.md), Getting started; [white paper](docs/white-paper.md), Motivation, Limitations, Related work | Published claims, scope and toolchain |
| E5 | [model/ci.sh](model/ci.sh) | Implemented eight-step gate and its scope |
| E6 | [model/trace.sh](model/trace.sh), `item_name`, pass 1, report | Source-tag interpretation and executed-test gap |
| E7 | [tool library](tools/diagram-gen/src/lib.rs), `default_crates`; [modellint](tools/diagram-gen/src/bin/modellint.rs); [lint wrapper](tools/lint.sh); [analysis catalogue](docs/analysis/README.md) | Fixed coverage list, severity policy and current counts |
| E8 | [CS-4 change-impact exercise](case-studies/cs4-batch-run/CHANGE-IMPACT.md) | Recorded change propagation and scale evidence, not rerun here |
| E9 | [tea resources](model/cs1-pot-of-tea/src/resources.rs), `DryTeabag`, `full_teabag_box`, `fill_kettle`, `boil` | Exposed untripwired item and explicit physical assumptions |
| E10 | [café flow tests](model/cs3-cafe-orders/tests/flows.rs), [café flows](model/cs3-cafe-orders/src/flows.rs), [bread flow tests](model/cs6-bread-batch/tests/flows.rs) | Strong path accounting; limits of ordering evidence |
| E11 | [spec wrapper](tools/spec.sh), [speccheck](tools/spec-gen/src/bin/speccheck.rs), [parser](tools/spec-gen/src/parse.rs), [template](SPEC_TEMPLATE.md) | Document gate versus implementation conformance |
| E12 | [specgen](tools/spec-gen/src/bin/specgen.rs), output-writing loop; [validation tests](tools/spec-gen/tests/validation.rs) | One-shot promise, overwriting behaviour, generator regression corpus |
| E13 | [scanner](tools/diagram-gen/src/scan.rs), [flow tracer](tools/diagram-gen/src/flow.rs), [linter](tools/diagram-gen/src/lint.rs) | Shared extraction and its blind spots |
| E14 | [history.rs](model/model-core/src/history.rs), `Recordable`, `Permit`, `record`, `merge` | Trust in event implementations; series-parallel histories |
| E15 | [tea specification](case-studies/cs1-pot-of-tea/SPEC.md), [bread specification](case-studies/cs6-bread-batch/SPEC.md) | Boundary assumptions, quantities and validation needs |
| E16 | [quantity.rs](model/model-core/src/quantity.rs), `Unit`, `Qty`, `split`, `combine`, `boundary::supply` | Arithmetic/unit checking versus physical quantity semantics |
| E17 | [EXP-13 results](experiments/exp13-dsl-macro/RESULTS.md), [EXP-14 results](experiments/exp14-dsl-external/RESULTS.md), [EXP-15 results](experiments/exp15-dsl-from-spec/RESULTS.md) | Existing alternatives explored by the project; no need to restart that work blindly |
| E18 | [PLAN.md](PLAN.md), [deck build script](decks/build.sh), [Clippy configuration](model/clippy.toml) | Delivery history, presentation convention and probe lint policy |

Further-reading entry points for evaluating the alternatives—not implementation or compatibility endorsements:

- [OMG SysML specification](https://www.omg.org/spec/SysML/2.0)
- [TLA+ resources](https://lamport.azurewebsites.net/tla/tla.html)
- [Alloy documentation](https://alloytools.org/documentation.html)
- [Z3 arithmetic guide](https://microsoft.github.io/z3guide/docs/logic/Arithmetic/)
- [OR-Tools job-shop scheduling](https://developers.google.com/optimization/scheduling/job_shop)
- [Modelica](https://modelica.org/)
- [GHC LinearTypes documentation](https://ghc.gitlab.haskell.org/ghc/doc/users_guide/exts/linear_types.html)
- [CPN Tools](https://cpntools.org/)

## 11. Recommended decision

Continue Lavoisier, with a more precise promise: **it makes selected resource-flow contracts executable and rejects specific classes of inconsistency under explicit modelling conventions**. That is already useful. The next increment should make its evidence harder to misinterpret and its workflow safer, then test whether the same value can be delivered with less modelling machinery.

Preserve the present design as a research result and a control implementation. Choose a broader architecture only after independent modelling, adversarial checks and physical validation demonstrate a benefit. A smaller, clearly bounded tool with honest evidence would be a stronger engineering contribution than a larger system whose green gate is mistaken for a complete assurance argument.

### Presentation rebuild

The new deck uses the repository's Pandoc/Beamer/XeLaTeX convention with its own presentation metadata. From the repository root:

```sh
pandoc decks/critical-review.md -t beamer --pdf-engine=xelatex \
  -o decks/critical-review.pdf
```

The existing `decks/build.sh` remains unchanged and still builds only its original two decks.
