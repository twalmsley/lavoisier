# Model Specification — CS-3: Café Order Fulfilment

| | |
|---|---|
| Specification version | v0.5 (implemented; format migrated to the R22 machine-checked conventions, 2026-10-08 — no semantic change) |
| Date | 2026-10-07 |
| Author | Tony (drafted by Claude; reviewed and agreed 2026-10-07) |
| Status | agreed |

## 1. Purpose and scope

A small café fulfils one customer order — a flat white and a pot of tea — with **two staff
working genuinely in parallel**: the barista makes the flat white (with fallible milk
steaming) while the server takes payment and brews the tea. This is the case study that
finally exercises R16 for real: **two actors, two branches, per-branch Histories merged at
the join into an honest partial order.** Money flows at the till (R19), including a refund on
the worst path, and the burnt-milk remake is R17 rework with provisioned milk.

**System boundary:** the counter area. The customer (a boundary object) tenders cash and
receives the order, change and any refund; stock and equipment are set up at flow start;
waste goes to the drain and knock box.

**Out of scope:** energy and temperature (CS-1 covered energy; here states stand in —
"steamed", "hot"); other customers and queueing; the till float (change comes from the
tendered note by splitting); food; crockery return (the tray leaves with the customer);
cleaning; staff scheduling.

## 2. Requirements

**Ids:** REQ-015–REQ-018, allocated from the workspace sequence (F-053; REQ-001..014 are
taken by the pilot, CS-1 and CS-2).

- **REQ-015:** The espresso machine may be operated only by a trained barista.
- **REQ-016:** Burnt milk must be discarded to the drain — it is never re-steamed or served.
- **REQ-017:** An order may be handed over only after payment (the join requires the payment
  receipt from the till branch).
- **REQ-018:** The customer's change must be returned in full at the till.

## 3. Resources

| Resource | Kind | Characteristics (markers/parameters) | Quantity & unit | States |
|---|---|---|---|---|
| `Milk` | continuous | grams | 300 g bottle; 150 g per steaming attempt | in bottle → `steamed` (150 g) or `burnt` (150 g) |
| `GroundCoffee` (ground coffee) | continuous | grams | hopper 500 g; 18 g per dose | hopper → dosed → (in drink / in puck) |
| `MachineWater` (machine water) | continuous | grams | tank 200 g; 40 g per shot | — |
| `EspressoShot` (espresso shot) | product part | 36 g (18 g dose + 40 g water − 22 g puck) | 1 | — |
| `SpentPuck` (spent puck) | waste | 22 g | per shot | → knock box |
| `FlatWhite` (flat white) | product | mass in type | 186 g (36 + 150) in cup | — |
| `UrnWater` (urn water) | continuous | grams | urn 1000 g; 300 g per pot | — |
| `Teabag` | discrete | 3 g | box of 10 | dry → in pot |
| `PotOfTea` (pot of tea) | product | mass in type | 303 g in pot | — |
| `Cup`s / `Teapot` / `Tray` | discrete crockery | — | 2 cups, 1 pot, 1 tray | leave with the customer |
| `Cash` | continuous (money) | pence | customer tenders 1000 p; order 700 p (flat white 380 + tea 320) | split 700 + 300 |
| `Till` | container (money) | pence | starts 0 | 0 → 700 → (320 on refund path) |
| `PaymentReceipt` (payment receipt) | evidence token | sealed; REQ-017's key | 1 | — |
| `Barista` | reusable | time budget; `MachineTrained` (R18) | 600_000 ms | draws down |
| `Server` | reusable | time budget | 600_000 ms | draws down |
| `SteamOutcome` (steam outcome) | outcome token (R17) | sealed, boundary-injected | 2 provisioned | — |

## 4. System boundary: suppliers, consumers, sinks

**Inputs (suppliers / sources):**

| What enters | Via | Capacity | Real or placeholder? |
|---|---|---|---|
| The customer's `Cash` (1000 p) | the customer (boundary object) | — | placeholder |
| Stock (`Milk` bottle, hopper, tank, urn, `Teabag` box, crockery) | counter setup at flow start | per §3 | placeholder |
| Staff (`Barista` trained via `qualify`, `Server`) | shift start | 1 + 1 | placeholder: training body |
| `SteamOutcome`s | boundary constructors / test fixtures | 2 | placeholder until calibrated |

**Outputs (consumers / sinks):**

| What leaves | Via | Capacity | Real or placeholder? |
|---|---|---|---|
| The served order (`Tray`, drinks, crockery), change, any refund | the customer (`Next = Self`) | unbounded | placeholder |
| `burnt` `Milk`; path 3's stranded `EspressoShot` | the drain | unbounded | placeholder |
| `SpentPuck`s | knock box | unbounded | placeholder |
| `Cash` payment (700 p) | the till (kept: container, not a sink) | — | real |
| Expended time | **two** Histories (R16) — one per actor — **merged at the join** | — | the per-branch merge, exercised for real at last |
| Untried `SteamOutcome` tokens | boundary exit | — | returned (R17) |

## 5. Processes

> Draws are adjacent `draw_time` steps recorded under the process name; waste routing named;
> balances marked (assert)/(structural).

### Branch A — the barista (own History `h_a`)

### P1. Pull the espresso
- **Actor(s) and reusables:** barista (draws 90_000 ms), espresso machine — returned.
  **REQ-015 bounds the barista.**
- **Consumes:** 18 g `GroundCoffee` (hopper draw), 40 g `MachineWater` (tank draw), 1 `Cup`.
- **Produces:** the `EspressoShot` in its cup (36 g).
- **Waste:** 1 `SpentPuck` (22 g) → knock box (§4).
- **Waste routing:** consumer parameter (the spent puck is fed to the knock box **inside the
  process**).
- **Balances:** mass 18 + 40 = 36 + 22 (assert); time → `h_a` (structural).
- **Satisfies:** REQ-015.

### P2. Steam the milk — **fallible (R17)**
- **Actor(s) and reusables:** barista (draws 60_000 ms per attempt) — returned both arms.
- **Consumes:** 150 g `Milk` (bottle draw), 1 `SteamOutcome` token.
- **Produces (Ok):** the `steamed` `Milk` (150 g, in the jug).
- **Produces (Fail):** the `burnt` `Milk` (150 g).
- **Waste:** 150 g `burnt` `Milk` → the drain (§4).
- **Waste routing:** consumer parameter (the burnt milk is fed to the drain **inside the
  process** — REQ-016 structural: the burnt state's only exit).
- **Balances:** mass 150 = 150 (assert, each arm); time → `h_a` (structural).
- **Satisfies:** REQ-016.
- **Failure modes:** burnt milk. Provisioned rework: the 300 g bottle allows exactly two
  attempts; a third is inexpressible (R17/F-050).

### P3. Build the flat white
- **Actor(s) and reusables:** barista (draws 30_000 ms) — returned.
- **Consumes:** the `EspressoShot` (36 g, in its cup) + the `steamed` `Milk` (150 g).
- **Produces:** the `FlatWhite` (186 g, in cup).
- **Balances:** mass 36 + 150 = 186 (assert); time → `h_a` (structural).

### Branch B — the server (own History `h_b`)

### P4. Take payment at the till
- **Actor(s) and reusables:** server (draws 60_000 ms), till — till keeps the money.
- **Consumes:** the customer's 1000 p `Cash`. The tender is split 700 + 300; 700 p is
  deposited to the `Till`.
- **Produces:** 1 `PaymentReceipt` (the REQ-017 evidence) and 300 p `Cash` (change, returned
  to the customer — REQ-018 structural: the split's second output has only the customer exit).
- **Balances:** money 1000 = 700 + 300 (assert); till 0 + 700 = 700 (assert); time → `h_b` (structural).
- **Satisfies:** REQ-017 (produces its key), REQ-018.

### P5. Brew the pot of tea
- **Actor(s) and reusables:** server (draws 120_000 ms), urn — returned.
- **Consumes:** 300 g `UrnWater` (urn draw), 1 `Teabag` (3 g), the `Teapot` + 1 `Cup`.
- **Produces:** the `PotOfTea` (303 g, in pot, with cup).
- **Balances:** mass 300 + 3 = 303 (assert); time → `h_b` (structural).

### The join

### P6. Assemble and hand over
- **Actor(s) and reusables:** server (draws 30_000 ms) — returned.
- **Consumes:** the `FlatWhite` + the `PotOfTea` + the `PaymentReceipt` (REQ-017) + the
  `Tray`. Merges `h_a` and `h_b` (R16).
- **Produces:** the served order (the `Tray` + drinks + crockery) to the customer. The merged
  History stays with the caller.
- **Balances:** items 2 drinks + tray = 1 order (structural); time → `h_b` before the merge (structural).
- **Satisfies:** REQ-017.

### P7. Refund the flat white — path 3 only
- **Actor(s) and reusables:** server (draws 30_000 ms), till.
- **Consumes:** 380 p `Cash` drawn from the `Till` (700 → 320).
- **Produces:** 380 p `Cash` (the refund) to the customer. The partial order (tea only) is
  handed over with the receipt.
- **Balances:** money 700 = 320 + 380 (assert); time → `h_b` (structural).

### P8. Drain the stranded shot — path 3 only (branch A)
- **Actor(s) and reusables:** barista (draws 30_000 ms) — returned.
- **Consumes:** the stranded `EspressoShot` (36 g) and its `Cup`.
- **Produces:** the `Cup` (back to the counter stack via the boundary exit).
- **Waste:** the stranded `EspressoShot` (36 g) → the drain (§4).
- **Waste routing:** consumer parameter (the shot is fed to the drain **inside the process**
  — the same Consumer machinery as the burnt milk).
- **Balances:** mass 36 = 36 (structural — the shot is fed whole); time → `h_a` (structural).

## 6. Flows

- Dependencies: P1, P2 (then P2 again on retry) and P3 form branch A; P4 and P5 form branch
  B; P6 joins them (and needs P4's receipt — the one deliberate cross-branch dependency,
  REQ-017); P7 replaces the flat white in P6 on path 3, and P8 winds up branch A there
  (the stranded shot to the drain).
- **Orders:** (a) P1, P2, P3, P4, P5, P6 (branch A, then branch B); (b) P4, P5, P1, P2, P3,
  P6 (branch B first — the branches share nothing, so any interleaving compiles, R9/R2).
- **This is the concurrency case study.** The branches share **no** resource — different
  actors, different equipment, different stock — so the compiler permits any interleaving
  (R9/R2). The model must demonstrate it: at least two interleavings of branch A and branch
  B steps compile and produce identical end states, and a trybuild case proves the contention
  counter-example (the server pulled into a branch-A step while their own branch holds them —
  "use of moved value").
- **Histories:** `h_a` and `h_b` are created per branch and merged at P6. The merged record
  is asserted to have the **Join shape** — branch A's events and branch B's events as
  parallel sequences with no invented interleaving (the first real exercise of R16's partial
  order). Event counts: path 1 → 3 + 3; path 2 → 4 + 3; path 3 → 4 + 4 (P8 on `h_a`,
  P7 on `h_b`).
- **Three paths** (a `#[must_use]` outcome grouping, one variant per path):
  1. **Served, first-try steam:** barista 180_000 ms drawn, server 210_000 ms; bottle at
     150 g; one untried token returned; till 700 p; customer: order + 300 p change.
  2. **Served, re-steamed:** barista 240_000 ms; bottle empty; 150 g burnt milk in the drain;
     both tokens used; till 700 p.
  3. **Tea served, flat white refunded:** both steams fail — barista 240_000 ms
     (90k + 60k + 60k + P8's 30k; no P3); 300 g burnt milk drained; P7 runs: till 320 p;
     customer: tea, 300 p change **and** 380 p refund; the espresso shot (path 3 waste,
     36 g) goes to the drain in P8 with its cup returned to the counter stack — see §8 Q1.
- **Everything accounted at every path's end:** drinks/tray with the customer; change and any
  refund with the customer; till holding its path's balance; staff back with their remaining
  budgets; stock containers at their drawn-down levels; pucks in the knock box; burnt milk
  (and path 3's stranded shot) in the drain; untried tokens returned; the **merged** History
  with the caller.

## 7. Assumptions and placeholders

- The customer, drain and knock box are unbounded placeholder boundary objects (F-029: they
  discard; end-state waste masses are assertable arithmetically only).
- Barista training is a placeholder `qualify` boundary process (R18).
- The 300 g milk bottle deliberately provisions exactly two steaming attempts.
- Prices and masses are round-number stand-ins (flat white 380 p, tea 320 p, tender 1000 p).
- Concurrency is **compile-time** concurrency (R9): the model proves the branches *may*
  interleave freely by sharing nothing; it does not execute threads.

## 8. Open questions for the author

Review decisions (2026-10-07): **all six agreed as proposed** — the stranded shot is drained
with its cup restacked; partial refund (tea still served); the receipt token carries REQ-017;
energy/temperature out of scope; quantities as drafted; merged-History assertions as drafted
(Join shape + per-branch counts).

Implementation round-trip feedback (2026-10-07):
1. **Path 3's fourth branch-A event** is the drained stranded shot's *disposal record*:
   `record(h_a, "drain_stranded_shot", shot)` — the budget arithmetic (210 000 ms = three
   draws) rules out a fourth time draw, and R16/F-041 allows recording any consumption. The
   shot's model sink is therefore the History record (the physical drain is documented); if
   the drain-as-consumer had been intended, the count would read 3+4. Future specs should
   name the source of every expected History event.
   *Review 2026-10-07: reworked — the shot now exits via the Drain consumer as P8 with its
   own 30 000 ms draw (barista 240 000 ms on path 3); all four branch-A events are time
   draws.*
2. **"Shot in cup" is a loose pair**, not a holder type: macro payloads are consumption-only
   (F-040) and the pair splits on path 3 (shot drained, cup restacked) — R9 loose threading.
3. **The 1000 p tender is a fixed note** (concrete `Money<1000>` at the till), making a wrong
   tender a pinned E0308 — load-bearing for a test; worth stating in §5 next time.
4. **"P7 replaces the flat white in P6"** is realized as a sealed two-type order slot
   (`FlatWhite<186>` or `Money<380>` refund), so P6 runs on all three paths — consistent with
   the server's 240 000 ms and h_b's four events on path 3.
5. **Generator convention (F-055 extension):** `match` directly on the fallible call — storing
   the `Result` in a binding and matching later hides the flow structure from diagram-gen
   ("assertion-only match" skip). R9's statement-order freedom can obscure diagrams.

Original questions, for the record:

1. **Path 3's stranded espresso:** if both steams fail, the already-pulled shot has no drink
   to join. Proposed: it is drained (36 g to the drain) and its cup goes back to the counter
   stack — honest waste. Alternatives: serve it as a free espresso (scope creep), or pull the
   shot only after the milk succeeds (removes the branch-A ordering freedom worth having).
   OK as proposed?
2. **Refund shape:** path 3 refunds the flat-white price (380 p) and still serves the tea.
   OK, or should the whole order be refunded/aborted?
3. **REQ-017 via a receipt token** consumed at the join — the cross-branch dependency that
   makes "hand over before payment" inexpressible. OK?
4. **Energy/temperature out of scope** (states stand in; CS-1 owns the energy demonstration). OK?
5. Quantities/prices sanity: §3's numbers, 10-minute budgets, 60–120 k ms draws. Happy?
6. **The merged-History assertions** (Join shape, per-branch event counts, no invented
   interleaving) are the heart of this case study — anything else you want asserted about
   the record (e.g. per-event magnitudes, as CS-2 did)?

*(Format note, 2026-10-08: migrated mechanically to the R22 machine-checked conventions —
backticked canonical identifiers, underscore-grouped numbers, the §2 Ids and §6 Orders fields,
split Waste/Waste routing fields. No semantic change; §8 kept verbatim.)*
