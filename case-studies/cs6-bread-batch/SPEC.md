# Model Specification — CS-6: Baking a Batch of Bread

| | |
|---|---|
| Specification version | v0.1 (draft — the first specification authored under the R22 gate from the first line) |
| Date | 2026-10-08 |
| Author | Tony (drafted by Claude) |
| Status | draft |

## 1. Purpose and scope

One baker makes two loaves of bread in a home kitchen. The case-study ladder (CS-1..CS-5) is
complete, so CS-6's stress is **the R22 pipeline itself**: this is the first specification
authored under the speccheck gate from the start (not migrated to it), and the first model
crate started from a `specgen` scaffold. The domain is chosen for emitter-path breadth per
page: a discrete supplier exhausted to its empty container, continuous draws in three units
(g, J, ms), multi-quantity states (a baked loaf carries mass *and* embodied energy), a
reusable with its own quantity-bearing state chain (the greased tin), and **fallibility
without rework** — a scorched loaf is final, the dough cannot be re-baked (where CS-2's
failures had provisioned retries).

**System boundary:** the kitchen. Ingredients, the baker, the oven, the tins and grid energy
enter it; baked loaves, scorched loaves, steam, waste heat, packaging waste and expended time
leave it.

**Out of scope:** preheating the oven (folded into the per-bake energy draw); cooling (loaves
leave warm, their embodied energy crossing the boundary with them); washing up (tins leave
`used`); proof timing and temperature (R9 — states stand in for "proved"); money, purchasing
and qualifications (CS-2/CS-5 cover those); scheduling and wall-clock time.

## 2. Requirements

**Ids:** REQ-028–REQ-031, allocated from the workspace sequence (F-053; REQ-027 is the
highest in use — the pilot owns REQ-001..005, CS-1 REQ-006..009, CS-2 REQ-010..014, and
CS-3..CS-5 REQ-015..027).

- **REQ-028:** Dough may be divided and shaped only once it is `proved`.
- **REQ-029:** The oven may bake only a loaf seated in a `greased` tin.
- **REQ-030:** The household accepts only `baked` loaves — a `scorched` loaf may not be
  handed over.
- **REQ-031:** Every `scorched` loaf must reach the compost stream.

## 3. Resources

| Resource | Kind | Characteristics (markers/parameters) | Quantity & unit | States |
|---|---|---|---|---|
| `Loaf` | product | mass and embodied energy carried in the type | 2 made | `shaped` (841 g) → `baked` (741 g, 300_000 J); or `scorched` (741 g, 300_000 J) |
| `Dough` | discrete w/ mass | mass carried in the type | 1 batch | `mixed` (1_682 g) → `kneaded` (1_682 g) → `proved` (1_682 g) |
| `Flour` | continuous | grams | 1_500 g bag, 1_000 g drawn | — |
| `Water` | continuous | grams | 650 g drawn from the tap | — |
| `Salt` | continuous | grams | 500 g jar, 18 g drawn | — |
| `Butter` | continuous | grams | 250 g block, 10 g drawn (5 g per tin) | — |
| `YeastSachet` (yeast sachet) | discrete w/ mass | 7 g yeast + 1 g wrapper | 2, boxed | `fresh` (8 g) → `spent` (1 g) |
| `YeastBox` (yeast box) | discrete container | holds the 2 sachets; box itself 30 g | 1 | `empty` (30 g) |
| `LoafTin` (loaf tin) | reusable | mass carried in the type | 2, 450 g each | `clean` (450 g) → `greased` (455 g) → `used` (450 g) |
| `Oven` | reusable | bakes one tin at a time | 1 | — |
| `Baker` (person) | reusable | time budget | 7_200_000 ms (2 h) | budget draws down |
| `GridEnergy` (oven energy) | continuous | joules | 5_000_000 J drawn from the grid | — |
| `Steam` | waste | grams | 105 g per bake | — |
| `WasteHeat` (waste heat) | waste | joules | 2_200_000 J per bake | — |
| `BakeOutcome` (bake outcome) | outcome token (R17) | sealed, boundary-injected | 2 provisioned | — |

## 4. System boundary: suppliers, consumers, sinks

**Inputs (suppliers / sources):**

| What enters | Via | Capacity | Real or placeholder? |
|---|---|---|---|
| `Flour` | pantry bag (container with remainder) | 1_500 g | placeholder: pantry |
| `Water` | tap, draw process | unbounded | placeholder |
| `Salt` | pantry jar (container with remainder) | 500 g | placeholder: pantry |
| `Butter` | pantry block (container with remainder) | 250 g | placeholder: pantry |
| `fresh` `YeastSachet`s | the `YeastBox` (discrete supplier, exhausted to empty) | 2 | placeholder: pantry |
| `GridEnergy` | grid, draw process | unbounded | placeholder |
| `Baker`, `Oven`, 2 `clean` `LoafTin`s | kitchen setup at flow start | 1/1/2 | placeholder |
| `BakeOutcome`s | boundary constructors / test fixtures | 2 | placeholder until calibrated |

**Outputs (consumers / sinks):**

| What leaves | Via | Capacity | Real or placeholder? |
|---|---|---|---|
| `baked` `Loaf`s | the household (accepts only `baked` — REQ-030) | unbounded | placeholder |
| `scorched` `Loaf`s | compost stream | unbounded (`Next = Self`) | placeholder |
| `Steam` | atmosphere | unbounded | placeholder |
| `WasteHeat` | atmosphere | unbounded | placeholder |
| `spent` `YeastSachet`s | recycling | unbounded | placeholder |
| `empty` `YeastBox` | recycling | unbounded | placeholder |
| Expended time | History (R16) | unbounded | single History (one actor) |

## 5. Processes

> Per the template conventions: actor draws are adjacent `draw_time` steps recorded under the
> process name; waste routing is named per output; balances are marked (assert)/(structural).

### P1. Mix the dough
- **Actor(s) and reusables:** baker (draws 900_000 ms) — returned.
- **Consumes:** 1_000 g `Flour`, 650 g `Water`, 18 g `Salt`, 2 `fresh` `YeastSachet`s
  (8 g each), 1 `YeastBox` (30 g). The flour, water and salt are drawn from their §4
  containers; taking both sachets exhausts the box.
- **Produces:** the `mixed` `Dough` (1_682 g).
- **Waste:** 2 `spent` `YeastSachet`s (1 g each) → recycling (§4); 1 `empty` `YeastBox`
  (30 g) → recycling (§4).
- **Waste routing:** routed by the flow (wrappers and the empty box are loose outputs the
  flow delivers to recycling).
- **Balances:** mass 1_000 + 650 + 18 + 8 + 8 + 30 = 1_682 + 1 + 1 + 30 (assert); time →
  History (structural).
- **Satisfies:** —.

### P2. Knead
- **Actor(s) and reusables:** baker (draws 600_000 ms) — returned.
- **Consumes:** the `mixed` `Dough` (1_682 g).
- **Produces:** the `kneaded` `Dough` (1_682 g). A state change only; the mass is
  unchanged.
- **Balances:** mass 1_682 = 1_682 (structural); time → History (structural).
- **Satisfies:** —.

### P3. Prove
- **Actor(s) and reusables:** baker (draws 3_600_000 ms) — returned. The draw stands in for
  the attended proving hour; wall-clock time is out of scope (§1).
- **Consumes:** the `kneaded` `Dough` (1_682 g).
- **Produces:** the `proved` `Dough` (1_682 g).
- **Balances:** mass 1_682 = 1_682 (structural); time → History (structural).
- **Satisfies:** — (enables REQ-028).

### P4. Grease the tins
- **Actor(s) and reusables:** baker (draws 120_000 ms) — returned. Independent of P1–P3:
  the flow's only ordering freedom (§6).
- **Consumes:** 2 `clean` `LoafTin`s (450 g each), 10 g `Butter`. The butter is drawn
  from the block, 5 g per tin.
- **Produces:** 2 `greased` `LoafTin`s (455 g each).
- **Balances:** mass 450 + 450 + 10 = 455 + 455 (assert); time → History (structural).
- **Satisfies:** — (enables REQ-029).

### P5. Divide and shape
- **Actor(s) and reusables:** baker (draws 480_000 ms) — returned. **REQ-028 bounds the
  dough.**
- **Consumes:** the `proved` `Dough` (1_682 g).
- **Produces:** 2 `shaped` `Loaf`s (841 g each).
- **Balances:** mass 1_682 = 841 + 841 (assert); time → History (structural).
- **Satisfies:** REQ-028.

### P6. Bake — **fallible (R17), one loaf per bake, run twice**
- **Actor(s) and reusables:** baker (draws 300_000 ms per bake), the `Oven` — returned.
  One oven: the second bake cannot start until the first returns it. **REQ-029 bounds the
  tin.**
- **Consumes:** 1 `shaped` `Loaf` (841 g), 1 `greased` `LoafTin` (455 g), 2_500_000 J
  `GridEnergy`, 1 `BakeOutcome` token. The energy is drawn from the grid (§4).
- **Produces (Ok):** the `baked` `Loaf` (741 g, 300_000 J) + the `used` `LoafTin` (450 g).
- **Produces (Fail):** the `scorched` `Loaf` (741 g, 300_000 J) + the `used` `LoafTin`
  (450 g).
- **Waste:** 1 `Steam` (105 g) → atmosphere (§4); 1 `WasteHeat` (2_200_000 J) →
  atmosphere (§4).
- **Waste routing:** consumer parameter (steam and waste heat are fed to the atmosphere
  inside the process and never exist loose); the `scorched` `Loaf` is a Fail-arm product
  routed by the flow to the compost stream (REQ-031).
- **Balances:** mass 841 + 455 = 741 + 450 + 105 (assert, both arms); energy 2_500_000 =
  300_000 + 2_200_000 (assert); time → History (structural).
- **Satisfies:** REQ-029, REQ-031.
- **Failure modes:** the bake scorches. **No rework is possible** — the dough is consumed,
  so a failed bake ends that loaf's path at the compost stream. 2 tokens are provisioned,
  one per bake; a third bake is inexpressible.

## 6. Flows

- Dependencies: P1 → P2 → P3 → P5; P4 is independent of P1–P3 and must precede P6; P5 and
  P4 feed P6, which runs twice (one loaf each) and sequentially (one `Oven`).
- **Orders:** (a) P1, P2, P3, P4, P5, P6, P6; (b) P4, P1, P2, P3, P5, P6, P6.
- **Four outcome combinations** (the flow returns a `#[must_use]` outcome grouping per
  R17/F-050 — the arms end with different loaf counts at the household and compost):
  1. **Both baked:** 2 `baked` `Loaf`s to the household; compost empty.
  2. **First scorched:** 1 `baked` to the household, 1 `scorched` to compost.
  3. **Second scorched:** as combination 2 with the bake order swapped.
  4. **Both scorched:** 2 `scorched` to compost; the household receives nothing.
  All four draw the same time (6_300_000 ms, 7 attributed events) and energy (5_000_000 J).
- **Ordering freedom is deliberately small** (one actor): P4 may run anywhere before P6.
  The two bakes are forced sequential by the single `Oven` — attempting both at once is the
  contention error at the exact line (R9).
- **Everything accounted:** at every combination's end — `baked` loaves at the household
  (each carrying 741 g and 300_000 J across the boundary), `scorched` loaves at compost,
  steam (210 g) and waste heat (4_400_000 J) in the atmosphere, 2 `spent` `YeastSachet`s
  and the `empty` `YeastBox` at recycling, the flour bag (500 g), salt jar (482 g) and
  butter block (240 g) back as container remainders, both `used` `LoafTin`s and the `Oven`
  back in the kitchen, the `Baker` back with 900_000 ms remaining, both tokens consumed,
  and the single History holding one attributed event per draw (P1, P2, P3, P4, P5, P6, P6).

## 7. Assumptions and placeholders

- The atmosphere, compost stream and recycling are unbounded placeholder sinks; the tap and
  grid are unbounded placeholder sources.
- All quantities are round-number stand-ins, not calibrated measurements (notably the
  741 + 105 g bake split and the 300_000 J embodied / 2_200_000 J waste-heat split).
- Steam carries no embodied energy (simplification; the energy split is accounted wholly to
  the loaf and the waste heat).
- The baker's 3_600_000 ms prove draw stands in for the elapsed proving hour (attended
  prove); wall-clock time remains out of scope.
- Scorch probability is uncalibrated: `BakeOutcome` tokens are boundary-injected (R17), one
  per bake.
- The oven bakes one tin at a time and is not preheated separately (the per-bake draw
  includes it).

## 8. Open questions for the author

1. **Domain:** the ladder is complete, so CS-6's stress is the pipeline, and bread was
   chosen for emitter-path breadth per page (discrete exhaustion + container disposal,
   three draw units, multi-quantity states, a stateful reusable, rework-free fallibility).
   OK, or would you rather field-test specgen on a different domain?
2. **Fallibility without rework:** a scorched loaf is final (the dough is consumed), so the
   four outcome combinations ride the flow's outcome grouping and the "both scorched" arm
   delivers nothing to the household. OK?
3. **Single oven, single History:** the two bakes are forced sequential by oven contention
   (the demo), and one actor means one History again. OK?
4. **The butter's journey:** greasing adds 5 g to each tin; the bake sends that butter out
   with the steam (841 + 455 = 741 + 450 + 105), so the tin returns to its clean mass as
   `used`. OK, or should the butter go into the loaf's crust instead?
5. **Quantities sanity check:** 1_000 g flour / 650 g water / 18 g salt / 14 g yeast →
   1_682 g dough; 841 g shaped → 741 g loaf + 105 g steam (incl. butter); 2_500_000 J per
   bake split 300_000 embodied + 2_200_000 waste heat; 2-hour budget with 900_000 ms spare.
   Happy?
6. **Scaffold regime (the field test):** per R22 I will run `specgen` once against this
   agreed spec, record the scaffold's measurements (items emitted, holes enumerated, any
   inference it needed) in FINDINGS, and commit the finished crate with the scaffold
   promoted to hand-maintained in that same change. OK?
