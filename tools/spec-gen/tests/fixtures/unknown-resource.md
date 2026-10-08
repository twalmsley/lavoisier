# Model Specification — CS-1: Making a Pot of Tea

| | |
|---|---|
| Specification version | v0.4 (implemented; format migrated to the R22 machine-checked conventions, 2026-10-08 — no semantic change) |
| Date | 2026-10-03 |
| Author | Tony (drafted by Claude; reviewed and agreed 2026-10-03) |
| Status | agreed |

## 1. Purpose and scope

The smallest complete everyday system: one person makes one pot of tea in a kitchen, using an
electric kettle. Chosen as the first case study because continuous resources (water, energy,
time) dominate, the flow has one genuine ordering freedom, and the whole model fits in a
walkthrough presentation.

**System boundary:** the kitchen. Mains water and grid electricity enter it; the finished pot
of tea, waste heat and spent teabags leave it (or rest in boundary consumers).

**Out of scope:** milk, sugar and serving; warming the pot; washing up; the pot's return after
drinking; temperature as a modelled dimension (states stand in for it: *boiling* water is a
type, not a reading); cost (no money in CS-1).

## 2. Requirements

**Ids:** REQ-006–REQ-009, allocated from the workspace sequence (F-053; the pilot owns
REQ-001..005). The mapping is recorded at the definitions in
`model/cs1-pot-of-tea/src/requirements.rs`.

- **REQ-006:** Tea must be brewed with boiling water (only the boiling state of the kettle can
  be poured into the pot).
- **REQ-007:** The pot must be loaded with exactly 3 teabags before brewing.
- **REQ-008:** All spent teabags must reach the food-waste bin.
- **REQ-009:** All waste heat must be accounted to the kitchen-air sink.

## 3. Resources

| Resource | Kind | Characteristics (markers/parameters) | Quantity & unit | States |
|---|---|---|---|---|
| `Water` | continuous | mass (g); embodied energy (J) carried in the type from boiling onward | 1_500 g drawn | `cold` → boiling (the kettle row owns that state) → tea (leaves in the pot) |
| `ElectricalEnergy` (electrical energy) | continuous | joules | 550_000 J drawn | — |
| `Teabag` | discrete | dry mass 3 g each; spent mass 12 g each (9 g absorbed water) | box of 40 | `dry` → `spent` |
| `Kettle` | reusable | holds water; needs no person while boiling | 1 | empty → `filled` (1_500 g) → `boiling` (1_500 g, 500_000 J) → empty |
| `Teapot` | reusable (leaves with the product) | holds teabags, then tea | 1 | empty → `loaded` (3 bags) → `pot of tea` (1_473 g, 480_000 J) |
| `Person` | reusable | time budget | 300_000 ms (5 min) | budget draws down |
| `WasteHeat` (waste heat) | continuous (waste) | joules | 50_000 + 20_000 J | — |
| `FoodWaste` (food waste) | continuous (waste) | grams — the spent teabags' mass leaving via disposal (R1: waste is a resource like any other) | 36 g | — |

## 4. System boundary: suppliers, consumers, sinks

**Inputs (suppliers / sources):**

| What enters | Via | Capacity | Real or placeholder? |
|---|---|---|---|
| `cold` `Water` (g) | mains tap, draw process | unbounded | placeholder |
| `ElectricalEnergy` (J) | grid socket, draw process | unbounded | placeholder |
| `Teabag`s | box of teabags | 40 | placeholder: brand/vendor not modelled |
| `Person`, `Kettle`, `Teapot`, bin | kitchen setup at flow start | 1 each | placeholder |

**Outputs (consumers / sinks):**

| What leaves | Via | Capacity | Real or placeholder? |
|---|---|---|---|
| `pot of tea` (with its mass and embodied energy) | the drinker | unbounded | placeholder |
| `spent` `Teabag`s | food-waste bin, then council food-waste collection (P5) | bin holds 10; collection unbounded | bin real; collection placeholder |
| `WasteHeat` | kitchen air | unbounded | placeholder |
| Expended time | History (R16) | unbounded | a single History threaded with the person (one actor serializes all draws); the per-branch merge demonstration is deferred to a later case study |

## 5. Processes

### P1. Fill the kettle
- **Actor(s) and reusables:** person (draws 30_000 ms), kettle, mains tap — returned.
- **Consumes:** 1_500 g `cold` `Water` (drawn from the mains).
- **Produces:** `filled` kettle (1_500 g).
- **Balances:** mass 1500 = 1500 (structural); time 30_000 ms → History (structural).
- **Satisfies:** —.

### P2. Load the pot
- **Actor(s) and reusables:** person (draws 20_000 ms), teapot, teabag box — returned (box at 37).
- **Consumes:** 3 `OolongPearl`s (one at a time from the box, R12).
- **Produces:** `loaded` pot (3 bags, 9 g dry).
- **Balances:** items 3 = 3 (structural); time 20_000 ms → History (structural).
- **Satisfies:** REQ-007 (the loaded-pot state exists only at exactly 3 bags).

### P3. Boil
- **Actor(s) and reusables:** kettle only — **no person** (the kettle is automatic; this is
  what makes the ordering freedom in §6 real).
- **Consumes:** `filled` kettle (1_500 g), 550_000 J `ElectricalEnergy` (drawn from the grid).
- **Produces:** `boiling` kettle (1_500 g, 500_000 J embodied).
- **Waste:** 50_000 J `WasteHeat` → kitchen air (kettle losses).
- **Waste routing:** routed by the flow (the flow's `vent_heat` carries the REQ bound — §8
  feedback item 7).
- **Balances:** mass 1500 = 1500 (structural); energy 550_000 = 500_000 + 50_000 (assert).
- **Satisfies:** —.
- **Failure modes:** none modelled in CS-1 (the kettle always boils; fallibility is CS-2's
  stress).

### P4. Pour and brew
- **Actor(s) and reusables:** person (draws 15_000 ms); kettle returned empty.
- **Consumes:** `boiling` kettle (1_500 g, 500_000 J) + `loaded` pot (3 bags, 9 g).
- **Produces:** `pot of tea` (1_473 g, 480_000 J embodied) and 3 `spent` `Teabag`s
  (12 g each, 36 g). The spent bags are removed from the pot at the end of brewing.
- **Waste:** 20_000 J `WasteHeat` → kitchen air (steeping losses); 3 `spent` `Teabag`s
  (36 g) → food-waste bin.
- **Waste routing:** consumer parameter (the process takes the bin and air as
  requirement-bounded consumer parameters and feeds them internally — §8 feedback item 3,
  the strong reading as implemented).
- **Balances:** mass 1500 + 9 = 1473 + 36 (= 1509) (assert); energy 500_000 = 480_000 + 20_000
  (assert); time 15_000 ms → History (structural).
- **Satisfies:** REQ-006 (accepts only the boiling kettle state), REQ-008 (spent bags exit
  only to the bin), REQ-009.

### P5. Empty the bin
- **Actor(s) and reusables:** person (draws 10_000 ms); the bin — returned empty.
- **Consumes:** the bin's kept contents: 3 `spent` `Teabag`s (36 g), released only through
  this sealed disposal path (F-039).
- **Produces:** —.
- **Waste:** 36 g `FoodWaste` → council food-waste collection (boundary consumer).
- **Waste routing:** consumer parameter (the disposal feeds the collection inside the sealed
  F-039 path).
- **Balances:** mass 36 = 36 (structural); time 10_000 ms → History (structural).
- **Satisfies:** REQ-008 (completes the spent bags' journey out of the system).

## 6. Flows

- Dependencies: P1 → P3 (boiling needs the filled kettle); P2 is independent of P1 and P3
  (it needs only the person, pot and box); P4 joins P3's and P2's outputs; P5 follows P4.
- **Concurrency:** P3 needs no person, so the person can load the pot (P2) *while the kettle
  boils* — or before filling it. A single History travels with the person (one actor, so the
  draws are serialized through them); the per-branch merge is deliberately not exercised here
  (deferred to a later case study with two actors).
- **Orders:** (a) P1, P2, P3, P4, P5; (b) P1, P3, P2, P4, P5 — at least two valid orders;
  the implementer proves both compile (R9).
- **Everything accounted at flow end:** pot of tea (with embodied energy) at the drinker;
  36 g of food waste at the council collection; the bin back, empty (capacity 10 restored);
  70_000 J total waste heat at the kitchen air; kettle back, empty; person back with
  225_000 ms; teabag box back at 37; the History (4 recorded draws, 75_000 ms) with the
  caller.

## 7. Assumptions and placeholders

- Mains water and grid electricity are unbounded (placeholders).
- The kitchen air absorbs all waste heat (placeholder; the embodied 480 000 J leaves with the
  tea via the drinker, not via the air).
- Energy numbers are round-figure stand-ins (≈ what 1.5 l from 20 °C to 100 °C costs, plus
  losses), not calibrated measurements — calibration is the validation open question.
- The drinker takes the pot and never gives it back (pot return is out of scope).
- One person does everything; no qualifications needed to make tea (R18 unused in CS-1).

## 8. Open questions for the author

Review decisions (2026-10-03): (1) embodied energy carried in the types — **agreed as
proposed**; (2) loaded pot exists only at exactly 3 bags — **agreed as proposed**; (3) round
quantities — **agreed as proposed**; (4) a disposal step **added** (P5, the F-039 pattern).
Additionally agreed: CS-1 keeps a single History (R16 requires one wherever time is drawn) and
defers the per-branch merge demonstration to a later case study.

Implementation round-trip feedback (2026-10-03) — items for the template and tooling:
1. **Requirement-id scope (F-053):** ids are workspace-global; allocate workspace-unique ids
   at spec time (CS-1 shipped as REQ-006..009) or scope the tooling per crate.
2. **Where a time draw lives:** "person (draws N ms)" was implemented per F-048 as an adjacent
   `draw_time` recorded to the History under the process's name, with the process itself
   taking/returning the person unchanged. The template should state this convention.
3. **Produces vs Waste routing:** P4 listed the spent bags under both; implemented the strong
   reading (the process takes the bin and air as requirement-bounded consumer parameters and
   feeds them internally, so spent bags never exist loose). The template should ask the author
   to say whether a process takes its consumers or the flow routes its outputs.
4. **REQ-002's observable failures:** because the loaded state exists only at exactly 3 bags,
   the only writable violation is brewing with an unloaded pot; an under-filled box fails at
   `load_pot` with the capacity message, not a REQ-phrased one.
5. **Assert vs structural balances:** P1's 1500 = 1500 and P2's 3 = 3 are structural (shared
   consts / `SupplyN<N3>`), not asserts; §5's Balances rows could distinguish the two.
6. **Multi-quantity states:** `container_resource!` fixes one magnitude slot, so the second
   dimension rides as a declared const parameter (`BoilingKettle<1500, 500_000>`); worth a
   template note for states carrying more than one quantity.
7. **P3's heat routing:** kettle losses reach the air via the flow's `vent_heat` (which
   carries the REQ bound), not inside P3; the spec's "Satisfies: —" on P3 is therefore
   correct as implemented — venting is the flow's job.
8. **Single-History decision:** implemented exactly as agreed; no friction.

*(Format note, 2026-10-08: this document was migrated mechanically to the R22 machine-checked
conventions — backticked canonical identifiers, underscore-grouped numbers, (assert)/(structural)
markers, the §2 Ids and §6 Orders fields, Waste routing fields, and a §3 row for the food
waste; §2's spec-local ids REQ-001..004 were renumbered to the implemented REQ-006..009 that
the field above states. The §8 record above is kept verbatim, so its id mentions predate the
renumbering. No semantic change.)*
