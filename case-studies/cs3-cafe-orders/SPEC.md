# Model Specification — CS-3: Café Order Fulfilment

| | |
|---|---|
| Specification version | v0.3 (implemented) |
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

> Workspace ids: REQ-001..014 are taken (pilot, CS-1, CS-2); CS-3 starts at REQ-015.

- **REQ-015:** The espresso machine may be operated only by a trained barista.
- **REQ-016:** Burnt milk must be discarded to the drain — it is never re-steamed or served.
- **REQ-017:** An order may be handed over only after payment (the join requires the payment
  receipt from the till branch).
- **REQ-018:** The customer's change must be returned in full at the till.

## 3. Resources

| Resource | Kind | Characteristics (markers/parameters) | Quantity & unit | States |
|---|---|---|---|---|
| Milk | continuous | grams | 300 g bottle; 150 g per steaming attempt | in bottle → steamed(150 g) or burnt(150 g) |
| Ground coffee | continuous | grams | hopper 500 g; 18 g per dose | hopper → dosed → (in drink / in puck) |
| Machine water | continuous | grams | tank 200 g; 40 g per shot | — |
| Espresso shot | product part | 36 g (18 g dose + 40 g water − 22 g puck) | 1 | — |
| Spent puck | waste | 22 g | per shot | → knock box |
| Flat white | product | mass in type | 186 g (36 + 150) in cup | — |
| Urn water | continuous | grams | urn 1000 g; 300 g per pot | — |
| Teabag | discrete | 3 g | box of 10 | dry → in pot |
| Pot of tea | product | mass in type | 303 g in pot | — |
| Cups / teapot / tray | discrete crockery | — | 2 cups, 1 pot, 1 tray | leave with the customer |
| Cash | continuous (money) | pence | customer tenders 1000 p; order 700 p (flat white 380 + tea 320) | split 700 + 300 |
| Till | container (money) | pence | starts 0 | 0 → 700 → (320 on refund path) |
| Payment receipt | evidence token | sealed; REQ-017's key | 1 | — |
| Barista | reusable | time budget; `MachineTrained` (R18) | 600 000 ms | draws down |
| Server | reusable | time budget | 600 000 ms | draws down |
| Steam outcome | outcome token (R17) | sealed, boundary-injected | 2 provisioned | — |

## 4. System boundary: suppliers, consumers, sinks

**Inputs (suppliers / sources):**

| What enters | Via | Capacity | Real or placeholder? |
|---|---|---|---|
| Customer's cash (1000 p) | the customer (boundary object) | — | placeholder |
| Stock (milk bottle, hopper, tank, urn, teabag box, crockery) | counter setup at flow start | per §3 | placeholder |
| Staff (barista trained via `qualify`, server) | shift start | 1 + 1 | placeholder: training body |
| Steam outcomes | boundary constructors / test fixtures | 2 | placeholder until calibrated |

**Outputs (consumers / sinks):**

| What leaves | Via | Capacity | Real or placeholder? |
|---|---|---|---|
| The served order (tray, drinks, crockery), change, any refund | the customer (`Next = Self`) | unbounded | placeholder |
| Burnt milk | the drain | unbounded | placeholder |
| Spent pucks | knock box | unbounded | placeholder |
| Payment (700 p) | the till (kept: container, not a sink) | — | real |
| Expended time | **two** Histories (R16) — one per actor — **merged at the join** | — | the per-branch merge, exercised for real at last |
| Untried steam tokens | boundary exit | — | returned (R17) |

## 5. Processes

> Draws are adjacent `draw_time` steps recorded under the process name; waste routing named;
> balances marked (assert)/(structural).

### Branch A — the barista (own History `h_a`)

### P1. Pull the espresso
- **Actor(s) and reusables:** barista (draws 90 000 ms), espresso machine — returned.
  **REQ-015 bounds the barista.**
- **Consumes:** 18 g grounds (hopper draw), 40 g water (tank draw), 1 cup.
- **Produces:** espresso shot in cup (36 g).
- **Waste routing:** spent puck (22 g) fed to the knock box **inside the process**.
- **Balances:** mass 18 + 40 = 36 + 22 (assert); time → `h_a` (structural).
- **Satisfies:** REQ-015.

### P2. Steam the milk — **fallible (R17)**
- **Actor(s) and reusables:** barista (draws 60 000 ms per attempt) — returned both arms.
- **Consumes:** 150 g milk (bottle draw), 1 steam outcome token.
- **Produces (Ok):** steamed milk (150 g, in the jug).
- **Produces (Fail):** burnt milk (150 g).
- **Waste routing:** burnt milk fed to the drain **inside the process** (REQ-016 structural —
  the burnt state's only exit).
- **Balances (both arms):** mass 150 = 150 (assert each arm); time → `h_a` (structural).
- **Satisfies:** REQ-016.
- **Failure modes:** burnt milk. Provisioned rework: the 300 g bottle allows exactly two
  attempts; a third is inexpressible (R17/F-050).

### P3. Build the flat white
- **Actor(s) and reusables:** barista (draws 30 000 ms) — returned.
- **Consumes:** espresso shot (36 g, in its cup) + steamed milk (150 g).
- **Produces:** flat white (186 g, in cup).
- **Balances:** mass 36 + 150 = 186 (assert); time → `h_a` (structural).

### Branch B — the server (own History `h_b`)

### P4. Take payment at the till
- **Actor(s) and reusables:** server (draws 60 000 ms), till — till keeps the money.
- **Consumes:** customer's 1000 p, split 700 + 300 (assert); 700 p deposited to the till.
- **Produces:** payment receipt (the REQ-017 evidence); 300 p change returned to the
  customer (REQ-018 structural — the split's second output has only the customer exit).
- **Balances:** money 1000 = 700 + 300 (assert); till 0 + 700 = 700 (assert); time → `h_b` (structural).
- **Satisfies:** REQ-017 (produces its key), REQ-018.

### P5. Brew the pot of tea
- **Actor(s) and reusables:** server (draws 120 000 ms), urn — returned.
- **Consumes:** 300 g urn water (draw), 1 teabag (3 g), the teapot + 1 cup.
- **Produces:** pot of tea (303 g, in pot, with cup).
- **Balances:** mass 300 + 3 = 303 (assert); time → `h_b` (structural).

### The join

### P6. Assemble and hand over
- **Actor(s) and reusables:** server (draws 30 000 ms) — returned.
- **Consumes:** flat white + pot of tea + **payment receipt** (REQ-017) + tray; **merges
  `h_a` and `h_b`** (R16).
- **Produces:** the served order (tray + drinks + crockery) to the customer; the merged
  History stays with the caller.
- **Balances:** items 2 drinks + tray = 1 order (structural); time → `h_b` before the merge (structural).
- **Satisfies:** REQ-017.

### P7. Refund the flat white — path 3 only
- **Actor(s) and reusables:** server (draws 30 000 ms), till.
- **Consumes:** till draw 380 p (till 700 → 320).
- **Produces:** 380 p refund to the customer; the partial order (tea only) handed over with
  the receipt.
- **Balances:** money 700 = 320 + 380 (assert); time → `h_b` (structural).

## 6. Flows

- Dependencies: P1, P2 (then P2 again on retry) and P3 form branch A; P4 and P5 form branch
  B; P6 joins them (and needs P4's receipt — the one deliberate cross-branch dependency,
  REQ-017); P7 replaces the flat white in P6 on path 3.
- **This is the concurrency case study.** The branches share **no** resource — different
  actors, different equipment, different stock — so the compiler permits any interleaving
  (R9/R2). The model must demonstrate it: at least two interleavings of branch A and branch
  B steps compile and produce identical end states, and a trybuild case proves the contention
  counter-example (the server pulled into a branch-A step while their own branch holds them —
  "use of moved value").
- **Histories:** `h_a` and `h_b` are created per branch and merged at P6. The merged record
  is asserted to have the **Join shape** — branch A's events and branch B's events as
  parallel sequences with no invented interleaving (the first real exercise of R16's partial
  order). Event counts: path 1 → 3 + 3; path 2 → 4 + 3; path 3 → 4 + 4 (P7 on `h_b`).
- **Three paths** (a `#[must_use]` outcome grouping, one variant per path):
  1. **Served, first-try steam:** barista 180 000 ms drawn, server 210 000 ms; bottle at
     150 g; one untried token returned; till 700 p; customer: order + 300 p change.
  2. **Served, re-steamed:** barista 240 000 ms; bottle empty; 150 g burnt milk in the drain;
     both tokens used; till 700 p.
  3. **Tea served, flat white refunded:** both steams fail — barista 210 000 ms (no P3);
     300 g burnt milk drained; P7 runs: till 320 p; customer: tea, 300 p change **and** 380 p
     refund; the espresso shot (path 3 waste) goes to the drain with its cup returned to the
     counter stack — see §8 Q1.
- **Everything accounted at every path's end:** drinks/tray with the customer; change and any
  refund with the customer; till holding its path's balance; staff back with their remaining
  budgets; stock containers at their drawn-down levels; pucks in the knock box; burnt milk in
  the drain; untried tokens returned; the **merged** History with the caller.

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
