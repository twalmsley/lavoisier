# Model Specification — CS-4: Small-Batch Production Run

| | |
|---|---|
| Specification version | v0.1 |
| Date | 2026-10-07 |
| Author | Tony (drafted by Claude for review) |
| Status | draft |

## 1. Purpose and scope

A production line builds a batch of **25 bracket assemblies** (two drilled plates fastened
with four bolts each) from materials issued by a stores subsystem. This is the **scale and
structure** case study: capacities near the measured limits (a 100-bolt box, a 75-item swarf
bin, 25-deep batch recursion — F-010/F-011 territory), repetition expressed honestly at the
type level, **two crates as two subsystems** with the crate edge as the interface (the first
multi-crate model), and a documented **change-impact exercise** — alter one resource type and
catalogue everything the compiler reports, turning the white paper's "impact analysis for
free" claim into evidence.

**System boundary:** the factory's stores-plus-line. Raw materials and the works order enter
stores; finished assemblies leave to finished goods; swarf leaves via disposal.

**Out of scope:** concurrency (CS-3 owns it — one operator, one History); fallibility (CS-2
owns it — every step succeeds); money and procurement (stores' stock is simply there);
energy; machine maintenance; scheduling.

## 2. Requirements

> Workspace ids: REQ-001..018 are taken; CS-4 starts at REQ-019.

- **REQ-019:** Assemblies may be built only from stores-issued materials (provenance: the
  line cannot mint or source materials itself — enforced by the crate boundary).
- **REQ-020:** All swarf from a batch must be collected in the line's bin and returned to
  stores' disposal at batch end.
- **REQ-021:** The line may operate only against a current stores issue note (the subsystem
  interface evidence, threaded through the batch and reconciled at the end).
- **REQ-022:** Completed assemblies must be delivered to finished-goods stores.

## 3. Resources

| Resource | Kind | Characteristics (markers/parameters) | Quantity & unit | States |
|---|---|---|---|---|
| Steel sheet | discrete w/ mass | 5000 g each | stores stock: 25 (type-level list) | sheet → 2 blanks + swarf |
| Plate blank | product part | 2250 g | 2 per sheet | blank → drilled (2240 g) |
| Bolt | discrete | M8 steel, 30 g | one box of 100 (type-level list) | — |
| Swarf (cut) | waste | 500 g per sheet | 25 pieces | → line bin |
| Swarf (drill) | waste | 10 g per plate | 50 pieces | → line bin |
| Swarf bin | contents-keeping consumer | decreasing space (F-034) | capacity 80; ends holding 75 | emptied via stores disposal (F-039) |
| Assembly | product | mass in type | 4600 g (2 × 2240 + 4 × 30) | → finished goods |
| Issue note | evidence token | sealed; REQ-021's key | 1 per batch | issued → reconciled |
| Operator | reusable | time budget | 4 500 000 ms (75 min) | draws down |
| Workbench, drill, saw | reusable | — | 1 each | — |

**Subsystem ownership (the two crates):**

| | `cs4-stores` (library crate) | `cs4-line` (depends on stores) |
|---|---|---|
| Owns | all material types and their seals; the stock (sheet rack, bolt box); issue/return/reconcile processes; finished-goods and disposal consumers; the issue note | the line processes (cut, drill, fasten, batch recursion); the swarf bin; the operator's flow |
| May create | materials (its boundary only) | nothing material — REQ-019 is the crate edge (EXP-08/F-006) |

## 4. System boundary: suppliers, consumers, sinks

**Inputs (suppliers / sources):**

| What enters | Via | Capacity | Real or placeholder? |
|---|---|---|---|
| Works order for 25 | stores intake | 1 | placeholder |
| Sheet rack (25 × 5000 g), bolt box (100) | stores stock at batch start | per §3 | placeholder: goods-in not modelled |
| Operator, tools | shift start | 1 + 3 | placeholder |

**Outputs (consumers / sinks):**

| What leaves | Via | Capacity | Real or placeholder? |
|---|---|---|---|
| 25 assemblies (4600 g each) | finished-goods stores (in `cs4-stores`) | ≥ 25 | placeholder |
| 75 swarf pieces (13 000 g total) | line bin → stores disposal at batch end (REQ-020, F-039) | bin 80 | disposal placeholder |
| Empty bolt box, empty sheet rack, reconciled note | returned to stores | — | real (accounted) |
| Expended time | single History (one operator) | — | per-cycle events, ~100 total |

## 5. Processes

> Draws adjacent and recorded under process names; waste routing named; balances marked.

### P1. Issue materials against the works order — `cs4-stores`
- **Actor(s) and reusables:** operator (draws 120 000 ms at the stores window).
- **Consumes:** the works order.
- **Produces:** the sheet rack (25), the bolt box (100), and the **issue note** (REQ-021's
  key), all handed to the line.
- **Balances:** items out = stock (structural); time → History (structural).
- **Satisfies:** REQ-019 (only this boundary creates/releases materials), REQ-021.

### P2. Cut a sheet — `cs4-line`, 25×
- **Actor(s) and reusables:** operator (draws 60 000 ms per sheet), saw — returned.
- **Consumes:** 1 sheet (5000 g).
- **Produces:** 2 blanks (2250 g each).
- **Waste routing:** cut swarf (500 g) fed to the line bin **inside the process**.
- **Balances:** mass 5000 = 2250 + 2250 + 500 (assert); time → History (structural).

### P3. Drill a blank — `cs4-line`, 50×
- **Actor(s) and reusables:** operator (draws 30 000 ms per blank), drill — returned.
- **Consumes:** 1 blank (2250 g).
- **Produces:** 1 drilled plate (2240 g).
- **Waste routing:** drill swarf (10 g) fed to the line bin **inside the process**.
- **Balances:** mass 2250 = 2240 + 10 (assert); time → History (structural).

### P4. Fasten an assembly — `cs4-line`, 25×
- **Actor(s) and reusables:** operator (draws 30 000 ms per assembly), workbench — returned.
  **REQ-021 bounds the flow context (note threaded); REQ-019 is structural (stores types).**
- **Consumes:** 2 drilled plates + 4 bolts (`SupplyN<N4>` from the one box).
- **Produces:** 1 assembly (4600 g, bolts conserved inside as payload).
- **Balances:** mass 2240 × 2 + 30 × 4 = 4600 (assert); time → History (structural).

### P5. The batch — `cs4-line`, the scale centrepiece
- **Repetition is type-level recursion** (a `BuildBatch<N>`-style recursive trait, the
  `SupplyN` pattern at batch scale): each cycle takes 1 sheet from the rack, runs P2, P3×2,
  P4, drawing 150 000 ms per cycle; the box, rack, bin, note, tools, operator and History
  thread through all 25 cycles with their types evolving (box 100→96→…→0). A Rust loop
  cannot express this — the types change every cycle — which is exactly the honesty being
  probed at scale.
- **Balances:** batch totals (structural from the 25 cycles): 25 sheets in; 25 assemblies,
  75 swarf pieces, empty rack and box out; 25 × 150 000 = 3 750 000 ms drawn.

### P6. Return and reconcile — `cs4-stores`
- **Actor(s) and reusables:** operator (draws 120 000 ms).
- **Consumes:** the 25 assemblies (→ finished goods, REQ-022), the full bin (→ disposal,
  REQ-020, the F-039 sealed path; bin returned empty to the line), the empty box and rack,
  and the issue note (reconciled — REQ-021 closed).
- **Balances:** swarf mass 25 × 500 + 50 × 10 = 13 000 g disposed (assert); items 25 = 25
  (structural); time → History (structural).
- **Satisfies:** REQ-020, REQ-021, REQ-022.

## 6. Flows

- P1 → P5 (the 25 recursive cycles) → P6. Within a cycle: cut → drill, drill → fasten; the
  only ordering freedom is trivial within-cycle reordering of the two drills — stated, not
  celebrated (concurrency is out of scope).
- **Everything accounted at batch end:** 25 assemblies in finished goods; 13 000 g of swarf
  at disposal; bin back empty; box and rack back empty; note reconciled; operator back with
  510 000 ms (4 500 000 − 120k − 3 750k − 120k); tools back; the History holding ~102
  attributed events (2 stores draws + 25 × 4 cycle draws).

## 7. The change-impact exercise (required deliverable)

The point: evidence for "change a type, and the compiler enumerates every affected process".
Method (results written to `case-studies/cs4-batch-run/CHANGE-IMPACT.md`; the probe changes
are **reverted**, never committed):

1. **Probe A (characteristic change):** in `cs4-stores`, change the bolt's length
   characteristic (L15 → L18). Run `cargo build --workspace`. Catalogue every error: file,
   process, message — the blast radius through requirements, catalogue bridging and the line
   crate.
2. **Probe B (quantity change):** change the sheet mass constant (5000 → 4800). Catalogue
   again — this blast radius runs through conservation asserts instead (and only appears on
   `cargo build`, F-001 — state this in the document).
3. Compare the two radii; conclusions on completeness (did the compiler find every place a
   human would need to touch?) and on error quality at scale.

## 8. Scale measurements (required findings)

Record in FINDINGS (or RESULTS feedback) with numbers: cold `ci.sh` wall time before vs after
CS-4 joins the workspace; the 100-bolt box and 75-item bin against F-010's recursion-limit
headroom (2048) and F-009's long-type side files; the 25-cycle recursion's compile-time cost
(F-011's curve at a real composite depth); any new limit found.

## 9. Assumptions and placeholders

- Stores stock is simply present (goods-in, procurement and money out of scope).
- One operator does everything; the 75-minute budget is sized to the 4 005 000 ms of draws
  with headroom.
- Masses are round-number stand-ins; drill swarf at 10 g keeps the bin arithmetic exact.

## 10. Open questions for the author

1. **Repetition as type-level recursion** (`BuildBatch<N>`, compile-time unrolled 25×) is the
   honest encoding — a loop cannot type-check because every cycle changes the types. Confirm
   this is the intended scale probe (it is the expensive part worth measuring)?
2. **The two-crate split** and ownership table in §3 — OK? (REQ-019 becomes a *crate-edge*
   guarantee, the first time EXP-08's result carries a real model.)
3. **Change-impact probes** (one characteristic, one quantity; documented in
   CHANGE-IMPACT.md; reverted, never committed) — OK? Anything else you want probed?
4. **Scale targets** (box 100, bin 80 holding 75, recursion 25 cycles ≈ 100 threaded steps)
   and the required measurements in §8 — ambitious enough, or push harder (e.g. a 50-unit
   batch) given F-011 says compile time is flat to ~500?
5. Quantities/budgets sanity: §3's masses, 150 000 ms per cycle, 75-minute budget. Happy?
6. **History at ~102 events** stays a single value-level record (R16); assertions on totals
   and spot-checks rather than all 102 events. OK?
