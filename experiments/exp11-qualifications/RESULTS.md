# EXP-11 Results: qualifications and safety as types (candidate R18)

**Toolchain:** `rustc 1.98.1 (48a229cea 2026-09-01) (Homebrew)`, stable channel, cargo 1.98.1,
macOS (Darwin 24.6.0, Apple Silicon). Dependencies: `model-core` by path (the EXP-10..12
protocol update; read-only), `trybuild` as the only external dev-dependency.

**Compile time:** `cargo clean && time cargo build` × 3: 1.68 s, 1.49 s, 1.54 s — **median 1.54 s**.
A non-issue.

**What was built:** a drilling fragment requiring a certified operator (REQ-001) and a machine
guard fitted to the drill (REQ-002). Qualification markers `CertifiedDriller`/`CertifiedWelder`
and safety-state marker `Fitted` (`src/qualifications.rs`); a `Qualification` kind trait with
value types `DrillingCert`/`WeldingCert`; **two** qualified-person designs probed side by side
(`src/resources.rs`): the `Operator<Q: Qualification, const BUDGET_MS: u64>` **wrapper** around
`model_core::common::Person`, and the `Driller<const SHIFT_MS: u64>` **parallel type** built
with `reusable_resource!`; `MachineGuard`/`FittedGuard` with `fit_guard`/`remove_guard`
processes; R10 requirement traits via `model_core::requirement!` (`src/requirements.rs`) bound
on `drill_plate` (and a style-B variant `drill_plate_timed`); flows in `tests/flows.rs`;
compile-fail cases in `tests/ui/` plus a `compile_fail` doc-test for overdraw. A copy of the
workspace `trace.sh` runs green over the crate (exit 0, both REQs defined/satisfied/verified).

---

## 1. Verdicts per evaluation criterion

### C1. Can the compiler refuse an unqualified person? — **pass**

`drill_plate` bounds its operator with the requirement trait
(`O: Req001CertifiedDrillingOperator`). Passing a bare `Person<5000>` or an
`Operator<WeldingCert, 5000>` is an E0277 whose top line is the REQ-phrased
`on_unimplemented` message (§2, errors A and B), pinned by trybuild
(`tests/ui/uncertified_person.rs`, `tests/ui/wrong_certification.rs`). The marker is attached
by a blanket impl over the budget (`impl<const MS: u64> CertifiedDriller for
Operator<DrillingCert, MS> {}`) — the blanket-impl-over-budgets pattern works first try, for
both the wrapper and the macro-built parallel type, and the requirement accepts **both**
designs through the same bound (`tests/flows.rs::parallel_type_driller_also_satisfies_the_requirement`).

### C2. Can the compiler refuse an unguarded machine? — **pass**

`Fitted` is implemented only by `FittedGuard`; the unfitted `MachineGuard` is a distinct type
(one type per state, R9/F-023) reachable only through the `fit_guard` process. An unfitted
guard at `drill_plate` is E0277 (§2, error C); a missing guard argument is E0061 plus — thanks
to inference pulling the plate into the guard slot — an E0277 saying a `Plate` is not `Fitted`
(§2, error D). trybuild-pinned (`tests/ui/unfitted_guard.rs`, `tests/ui/missing_guard.rs`).
"Guard present but not fitted" and "no guard at all" are thus both refused, and distinguishable.

### C3. Does the qualified person's budget still draw down? — **pass**

The wrapper's draw (`operator_draw_time`) opens the wrapper, delegates to model-core's
`draw_time`, and re-wraps — the R15 overdraw check is **inherited, not duplicated**: drawing
6000 ms from a 4000 ms operator is the same E0080 as on a bare person, with model-core's own
message and decimal magnitudes (§2, error F; post-monomorphization per F-001, so the regression
is a rustdoc `compile_fail` doc-test on `operator_draw_time`, not a trybuild case). At
type-check time, a stale-balance call is an E0308 that prints both decimal budgets
(`expected 5000, found 3000`, §2, error E; trybuild-pinned in `tests/ui/budget_drawn_down.rs`).
Multi-step draw-down through the wrapper, ending in the spent `Person<0>` stepping back out,
is `tests/flows.rs::budget_draws_down_through_the_qualification`. F-030 applies to the wrapper
exactly as to `Person` (caller-restated running balance; the budget infects signatures).

### C4. Interop with `model_core::common::Person` — **pass with complications** (the honest answer)

- **Markers cannot go on `Person` itself.** Coherence would allow
  `impl<const MS: u64> CertifiedDriller for Person<MS>` (local trait), but `Person` has no
  qualification slot, so the impl would certify *every* person in the model at once. The rule
  is semantic, not technical — it must be written down (it is, in the proposed R18).
- **Wrapper (adopted):** `Operator<Q, BUDGET_MS>` holds the `Person` as a private field.
  `certify` (a `/// Placeholder:` boundary process standing in for real training) wraps;
  `decertify` unwraps the *same* person, budget intact — the person is conserved through both,
  verified end to end (`Person<3000>` comes back out after a 2000 ms draw from 5000).
  Budget draws delegate to model-core's `draw_time`, so `Labour` and its `History` sink
  (F-035/R16) are reused unchanged.
- **Parallel type (rejected):** `Driller<SHIFT_MS>` via `reusable_resource!` costs 2 lines to
  declare and 1 line to certify — but it holds no `Person`, so it forks the entire
  time-accounting family: its own draw process (restating what `draw_time` does), its own
  labour type (`Effort` — model-core's `Labour` cannot be minted downstream), a `Consumer`
  sink for it, and a hand-written `Recordable` impl before it can reach the execution history
  (model-core grants `Labour` that for free). ≈ 40 lines of duplicated machinery per person
  type, all of it re-reviewable conservation code.
- **Complication:** the wrapper adds one indirection hop to overdraw diagnostics — the
  "while instantiating" note (F-027) lands on `operator_draw_time`'s internal `draw_time`
  call, not the modeller's line (§2, error F). The decimal magnitudes
  (`draw_time::<6000, 0, 4000>`) still identify the offending call uniquely.
- **Complication:** `reusable_resource!` cannot hold contents (payload fields are
  `consumable_resource!`-only, F-040), so the wrapper is **hand-sealed** against the R1
  checklist (~34 lines once, including `certify`/`decertify`/`operator_draw_time`).

### C5. Requirement traits + trace.sh — **pass**

One R10 trait per constrained participant: the sentence "drilling requires a certified
operator and a fitted guard" spans two parameters, and a single trait cannot bound both — it
decomposes into REQ-001 (operator) and REQ-002 (guard), both bound on the same one-line
`drill_plate` signature (R10 rule 4). `trace.sh` (workspace copy, run from the crate) exits 0
and reports both REQs defined, satisfied (tags on the `DrillingOperator`/`ParallelDriller`/
`DrillReadyGuard` aliases per F-037, backed by `satisfies!` assertions; bound uses on both
drill processes) and verified (4 tagged tests). A macro-built resource again cannot carry its
own tag (F-037 reconfirmed): the aliases exist precisely to carry them.

### C6. Ergonomics / boilerplate — **pass** (numbers in §3)

---

## 2. Representative compiler errors, verbatim

**(A) Uncertified person at a drilling process** (`tests/ui/uncertified_person.stderr`):

```text
error[E0277]: this person may not drill: `Person<5000>` is not a certified drilling operator (REQ-001)
  --> tests/ui/uncertified_person.rs:12:71
   |
12 |     let (person, guard, drilled, swarf) = drill_plate::<900, 880, 20, _, _>(person, guard, plate);
   |                                                                       ^ REQ-001: drilling requires a certified drilling operator
   |
   = help: the trait `CertifiedDriller` is not implemented for `Person<5000>`
   = note: qualifications are granted at the boundary (R12): `certify::<DrillingCert>(person)` wraps a `Person` into a certified `Operator`
help: the following other types implement trait `CertifiedDriller`
  --> src/resources.rs
   |
   | impl<const MS: u64> CertifiedDriller for Operator<DrillingCert, MS> {}
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ `Operator<DrillingCert, MS>`
...
   | impl<const MS: u64> CertifiedDriller for Driller<MS> {}
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ `Driller<MS>`
   = note: required for `Person<5000>` to implement `Req001CertifiedDrillingOperator`
note: required by a bound in `drill_plate`
```

*Judgement:* the best requirement error in the project so far — top line states the modelling
mistake in REQ language, the note says how to fix it, and rustc's impl list enumerates every
type that *would* qualify. **Caveat that produced it:** the `#[diagnostic::on_unimplemented]`
sits on the **requirement trait** (passed through `requirement!`'s meta slot), not only on the
marker — see error C for what happens otherwise.

**(B) Wrong certification** (`tests/ui/wrong_certification.stderr`), same shape:

```text
error[E0277]: this person may not drill: `Operator<WeldingCert, 5000>` is not a certified drilling operator (REQ-001)
```

*Judgement:* excellent — the type name itself says "this is a welder".

**(C) Unfitted guard** (`tests/ui/unfitted_guard.stderr`) — REQ-002 deliberately carries
**no** `on_unimplemented` (the probe's control arm):

```text
error[E0277]: the trait bound `MachineGuard: Req002FittedDrillGuard` is not satisfied
  --> tests/ui/unfitted_guard.rs:14:76
   |
14 |     let (operator, guard, drilled, swarf) = drill_plate::<900, 880, 20, _, _>(operator, guard, plate);
   |                                                                            ^ the trait `Fitted` is not implemented for `MachineGuard`
   |
help: the trait `Fitted` is implemented for `FittedGuard`
   = note: required for `MachineGuard` to implement `Req002FittedDrillGuard`
```

*Judgement:* still readable ("`Fitted` not implemented for `MachineGuard`", and the help names
`FittedGuard`), but generic-rustc phrased. **Key finding:** the marker trait `Fitted` *does*
carry a modeller-phrased `on_unimplemented`, and it is **ignored here** — a nested obligation
never surfaces its own attribute; only the root obligation's (the requirement trait's) message
is used. The pilot's `wrong_bolt.stderr` has the same shape (its `M8` message is also unused).
The attribute must therefore go on the **requirement trait**; marker-level messages only ever
show where a marker is itself the direct bound.

**(D) Missing guard entirely** (`tests/ui/missing_guard.stderr`): E0061
("this function takes 3 arguments but 2 arguments were supplied … argument #3 of type
`Plate<900>` is missing") plus an E0277 saying `Plate<900>` is not `Fitted` (inference slid
the plate into the guard slot). *Judgement:* blunt but unambiguous — the process signature is
the checklist of required inputs, and omitting one cannot compile.

**(E) Stale budget state** (`tests/ui/budget_drawn_down.stderr`):

```text
error[E0308]: mismatched types
16 |     let (l2, operator) = operator_draw_time::<2000, 3000, 5000, _>(operator);
   |                          ----------------------------------------- ^^^^^^^^ expected `5000`, found `3000`
   = note: expected struct `Operator<_, 5000>`
              found struct `Operator<DrillingCert, 3000>`
```

*Judgement:* excellent — decimal budgets on both sides (F-027 holds through the wrapper);
reads directly as "this operator has 3000 ms left, not 5000".

**(F) Overdraw through the wrapper** (captured from a temporary example;
`cargo check` passes, `cargo build` fails — F-001 exactly as on a bare person):

```text
error[E0080]: evaluation panicked: time budget violated in draw_time (R15): SPEND + LEFT must equal BUDGET - is more time being spent than the person has left?
   --> .../library/core/src/panic.rs:62:9
    |
    |  evaluation of `model_core::common::processes::draw_time::<6000, 0, 4000>::{constant#0}` failed here
...
note: the above error was encountered while instantiating `fn draw_time::<6000, 0, 4000>`
   --> src/resources.rs:367:32
    |
367 |         let (labour, person) = draw_time::<SPEND, LEFT, BUDGET>(person);
```

*Judgement:* the message and decimals are model-core's, inherited for free; **complication:**
the instantiation note points at the wrapper's internal `draw_time` call, not the modeller's
flow line — one indirection hop worse than F-027's bare-container case.

**(G) Turbofish ergonomics note:** mixing const and type parameters means call sites cannot
omit a trailing inferred type parameter — Rust's all-or-nothing rule yields E0107
("function takes 4 generic arguments but 3 were supplied") until the modeller writes a `_`:
`operator_draw_time::<2000, 3000, 5000, _>(operator)`. Harmless but must be documented.

---

## 3. Boilerplate measured

Whole crate: 141 lines of `src/` excluding comments/blanks (627 raw lines incl. docs).

| Item | Cost |
|---|---|
| **Per qualification** (adding "welding" once the wrapper exists) | **5 code lines**: value type + `Qualification` impl (2), marker trait + one-line `on_unimplemented` (2), bridging blanket impl over budgets (1). Plus, when a requirement needs it: one `requirement!` invocation (4 lines) and a tagged alias + `satisfies!` (3). |
| **Per safety resource with a fitted state** | 2 `reusable_resource!` invocations (4 lines each), 1 marker impl, `fit`/`remove` processes (~10), tagged alias + `satisfies!` (3) ≈ **22 lines**. |
| **Once per model: the `Operator` wrapper** | ~34 code lines hand-sealed (struct 6, `certify` 8, `decertify` 6, `operator_draw_time` 14) — hand-written because `reusable_resource!` takes no held contents (F-040). |
| **Per person type, parallel design (rejected)** | 2-line declaration + 1-line marker, **then ~40 lines of forked time accounting** (own draw with its own const assert, own `Effort` labour type, own `Consumer` sink, own `Recordable` impl to reach `History`). |

---

## 4. Candidate FINDINGS entries (ready to paste)

### F-NEW-1 — Qualifications attach to wrapper types, never to the shared `Person`; the wrapper inherits the R15 budget machinery, at one diagnostic hop

**What couldn't be expressed:** a qualification as a marker on `model_core::common::Person`
itself. Coherence permits the impl (local trait), but `Person<BUDGET_MS>` has no qualification
slot, so `impl<const MS: u64> CertifiedDriller for Person<MS>` would certify every person in
the model at once — quiet model corruption of the F-017 class, with no compiler warning
possible. A parallel downstream person type avoids this but holds no `Person`, so it forks the
whole time-accounting family (its own draw process, its own labour type — `Labour::mint` is
private to model-core — its own sink, and its own `Recordable` impl; ~40 duplicated lines per
person type, measured).

**What it cost:** a hand-sealed wrapper `Operator<Q: Qualification, const BUDGET_MS: u64>`
holding the `Person` as a private field (~34 lines once per model: the kernel's
`reusable_resource!` takes no held contents, F-040). Certification/decertification are
conserving boundary processes (wrap/unwrap the same person); the budget draw opens the
wrapper, delegates to `draw_time`, and re-wraps, so the R15 overdraw E0080 and the
`Labour`→`History` path are inherited unchanged. One diagnostic degradation: the overdraw's
"while instantiating" note (F-027) lands on the wrapper's internal `draw_time` call, one hop
from the modeller's flow line (decimal magnitudes still identify the call). One ergonomic
footnote: with consts before the type parameter, callers must write a trailing `_`
(`operator_draw_time::<2000, 3000, 5000, _>(op)`) — trailing generic arguments cannot be
omitted from a turbofish.

**Workaround adopted:** the wrapper, with the rule "qualification markers go on qualified
wrapper types via blanket impls over the budget (`impl<const MS: u64> CertifiedDriller for
Operator<DrillingCert, MS>`), never on `Person`". The blanket-impl-over-budgets pattern worked
first try on both the hand-sealed wrapper and a macro-built type.

**Evidence:** EXP-11 (`experiments/exp11-qualifications/RESULTS.md` §1 C4;
`src/resources.rs::{Operator, Driller, Effort}`, `tests/flows.rs`).

### F-NEW-2 — `on_unimplemented` on a supertrait/marker is ignored at a requirement bound; the attribute must go on the requirement trait itself

**What couldn't be expressed:** one modeller-phrased message per characteristic that surfaces
everywhere. When a process bounds `O: Req001…` (the R10 pattern) and the blanket impl's
supertrait obligation fails, rustc surfaces only the **root** obligation's
`#[diagnostic::on_unimplemented]`; the failing *marker's* own attribute (`CertifiedDriller`,
`Fitted`) is never shown in that position — the error falls back to "the trait bound
`MachineGuard: Req002FittedDrillGuard` is not satisfied". The pilot's `wrong_bolt.stderr` has
the same shape (its `M8` message is likewise unused). Marker-level messages appear only where
the marker is itself the direct bound.

**What it cost:** one more attribute per requirement trait, and a convention to know.

**Workaround adopted:** `model_core::requirement!`'s meta slot already passes attributes
through, so each `requirement!` invocation carries its own single-line `on_unimplemented`
phrased as the requirement ("this person may not drill: `{Self}` is not a certified drilling
operator (REQ-001)"). Result (trybuild-pinned): the top line of the E0277 states the REQ, the
label repeats it at the argument, the note says how to certify, and rustc's own impl list
enumerates the types that would qualify — the best requirement error measured in the project.
Keep marker-level attributes too, for direct marker bounds.

**Evidence:** EXP-11 (`tests/ui/uncertified_person.stderr` vs `tests/ui/unfitted_guard.stderr`;
`src/requirements.rs`); corroborated by `model/pilot-workshop/tests/ui/wrong_bolt.stderr`.

### F-NEW-3 — A requirement sentence spanning several participants decomposes into one R10 trait per constrained parameter

**What couldn't be expressed:** "drilling requires a certified operator **and** a fitted
guard" as a single requirement trait — a trait bound constrains one type, and no type is both
the operator and the guard. (A composite `DrillingStation` aggregate would over-claim, F-024.)

**What it cost:** two REQ ids for one sentence of intent (REQ-001 operator, REQ-002 guard),
both bound on the same one-line process signature.

**Workaround adopted:** accept the decomposition; trace.sh then reports the one process under
both ids, which reads correctly in the traceability table. The safety-state half additionally
relies on "one type per state" (R9/F-023): `Fitted` is implemented only by `FittedGuard`, so
"guard present but not fitted" is refused distinctly from "no guard at all" (E0277 vs E0061).

**Evidence:** EXP-11 (`src/requirements.rs` design note; `src/resources.rs::drill_plate`;
`tests/ui/unfitted_guard.rs`, `tests/ui/missing_guard.rs`; trace.sh output in RESULTS §1 C5).

### F-NEW-4 — A process that draws a budget inside itself must take the concrete wrapper and loses the REQ-phrased error; keep draws as separate processes

**What couldn't be expressed:** a drilling process that is simultaneously (a) generic over
"any certified operator" and (b) draws the operator's time budget down inside itself. The
budget change is a const-parameter change, which a generic `O:
Req001CertifiedDrillingOperator` cannot express (the F-028/E0207 remainder problem in
different clothes). Style B (`drill_plate_timed`, the pilot `drill_holes` shape) therefore
takes the concrete `Operator<DrillingCert, BUDGET>`, restating REQ-001 as a trivially-true
where-clause for greppability — and a wrong operator there is an E0308 type mismatch, not the
REQ-phrased E0277.

**What it cost:** a composition convention rather than machinery.

**Workaround adopted:** style A as default — qualification-checked processes are generic with
requirement bounds and do **not** draw budgets; the draw is its own adjacent process in the
flow (`operator_draw_time` then `drill_plate`), which is R9/R15-idiomatic anyway. Style B
stays legal where a single process must account its own time, with the where-clause convention.

**Evidence:** EXP-11 (`src/resources.rs::{drill_plate, drill_plate_timed}`;
`tests/flows.rs`, both styles exercised).

---

## 5. Recommendation

**Adopt** qualifications-and-safety-as-types as requirement R18 — the machinery is R6+R10+R15
composed, it worked with zero new blockers, and the error quality (with F-NEW-2's attribute
placement) is the best in the project.

**What belongs in model-core vs downstream convention:**

- **model-core (small, worth doing):** move the qualified-person wrapper into
  `model_core::common` as `Qualified<Q: Qualification, const BUDGET_MS: u64>` (holding
  `Person<BUDGET_MS>`), with the `Qualification` kind trait, boundary `qualify`/`release`
  (placeholder-tagged) and a delegating `qualified_draw_time`. It is ~34 lines every model
  would otherwise hand-seal (the kernel macro cannot generate it — held contents on a
  reusable, F-040), it has exactly one sensible shape, and downstream coherence is unaffected
  (markers are downstream traits, so `impl<const MS> CertifiedDriller for
  Qualified<DrillingCert, MS>` is legal downstream). No `qualification!` macro is needed: a
  qualification is 5 ordinary lines (value type, kind impl, marker trait + message, bridging
  impl), below macro-worthy.
- **Pure downstream convention:** the markers, cert value types, requirement traits,
  guard-state types and fit/remove processes — all ordinary R6/R9/R10 code.
- **Documentation (error-reading guide / R10):** F-NEW-2's rule — the `on_unimplemented`
  goes on the requirement trait, via `requirement!`'s existing meta slot.

### Proposed R18 wording (slots into instructions.md after R16)

> ### R18. Qualifications and safety as types
> - A person's capabilities (certifications, authorisations) and a machine's safety states
>   (a fitted guard, worn PPE) are **characteristics** (R6): marker traits with
>   modeller-phrased `on_unimplemented` messages, required through R10 requirement traits
>   bound on each process that needs them. There are no runtime qualification checks.
> - **A qualified person is a sealed wrapper around the common `Person`** (R11):
>   `Qualified<Q: Qualification, const BUDGET_MS: u64>` holds the person as a private field.
>   `Qualification` is the kind trait (R6) over sealed-free qualification value types
>   (`DrillingCert`); qualification markers (`CertifiedDriller`) attach by one-line blanket
>   impls **over the budget** (`impl<const MS: u64> CertifiedDriller for
>   Qualified<DrillingCert, MS> {}`), added with the qualification value itself.
>   - Markers are **never implemented on `Person` directly**: `Person` has no qualification
>     slot, so a direct impl certifies every person in the model at once.
>   - Certification enters the model as a **boundary process** (R12): `qualify` wraps,
>     `release` unwraps the same person — the person is conserved through both. It is a
>     placeholder (`/// Placeholder: certification body…`) until refined into a real
>     training process producing certificate evidence.
>   - The wrapper's budget is the inner person's: one generic draw process opens the
>     wrapper, delegates to `draw_time` and re-wraps, so the R15 overdraw error and the
>     `Labour`→`History` accounting (R16) are inherited, not duplicated. Parallel person
>     types that bypass `Person` are not used: they fork the draw/labour/sink/record
>     machinery (~40 duplicated lines per type, EXP-11).
> - **A safety resource is a reusable resource** (R2) with **one type per safety state**
>   (R9): `MachineGuard` vs `FittedGuard`, converted only by explicit `fit`/`remove`
>   processes. The safe characteristic (`Fitted`) is implemented by the safe state only, so
>   "guard present but not fitted" (E0277) and "no guard at all" (E0061) are both compile
>   errors, and distinguishable.
> - A guarded, qualification-checked process takes the person and the safety resources as
>   **generic parameters bounded by requirement traits** (F-019) and returns them (R2). A
>   requirement sentence spanning several participants decomposes into **one R10 trait per
>   constrained parameter** (e.g. REQ-001 operator + REQ-002 guard), all bound on the same
>   `fn`-name line (R10 rule 4); trace.sh reports the one process under each id.
> - **Each requirement trait carries its own one-line `#[diagnostic::on_unimplemented]`**,
>   phrased as the requirement and naming the REQ id, passed through `requirement!`'s meta
>   slot. A marker's own message is ignored when the marker fails as a supertrait obligation
>   of a requirement bound — only the root obligation's attribute is shown — so the
>   requirement trait's message is the one modellers will see.
> - Qualification-checked processes do **not** draw time budgets themselves: the draw is its
>   own adjacent process in the flow (R15). A process that must account its own time takes
>   the concrete `Qualified<Cert, BUDGET>` instead (the budget is a const-parameter change a
>   generic bound cannot express), restates its requirement as a where-clause bound on the
>   concrete type for traceability, and accepts that its wrong-operator errors are E0308
>   type mismatches rather than the REQ-phrased messages.

---

## 6. Artefact map

| File | Contents |
|---|---|
| `src/qualifications.rs` | `Qualification` kind trait, cert value types, markers with `on_unimplemented` |
| `src/requirements.rs` | REQ-001 (with the on_unimplemented probe arm) and REQ-002 (control arm) |
| `src/resources.rs` | `Operator` wrapper + `Driller` parallel probe + `Effort` fork + guards + plates/swarf + `Site` sink; `boundary` and `processes` child modules; unit tests |
| `tests/flows.rs` | Both process styles, parallel-type flow, multi-draw budget flow; `Verifies:` tags |
| `tests/ui/*` | 5 trybuild cases with pinned `.stderr` |
| `operator_draw_time` doc-tests | Positive draw + History record; `compile_fail` overdraw regression (F-001 policy) |
| `trace.sh` | Workspace copy, runs green over this crate (exit 0) |
