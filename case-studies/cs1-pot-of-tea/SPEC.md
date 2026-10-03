# Model Specification — CS-1: Making a Pot of Tea

| | |
|---|---|
| Specification version | v0.1 |
| Date | 2026-10-03 |
| Author | Tony (drafted by Claude for review) |
| Status | draft |

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

- **REQ-001:** Tea must be brewed with boiling water (only the boiling state of the kettle can
  be poured into the pot).
- **REQ-002:** The pot must be loaded with exactly 3 teabags before brewing.
- **REQ-003:** All spent teabags must reach the food-waste bin.
- **REQ-004:** All waste heat must be accounted to the kitchen-air sink.

## 3. Resources

| Resource | Kind | Characteristics (markers/parameters) | Quantity & unit | States |
|---|---|---|---|---|
| Water | continuous | mass (g); embodied energy (J) carried in the type from boiling onward | 1500 g drawn | cold → boiling → tea |
| Electrical energy | continuous | joules | 550 000 J drawn | — |
| Teabag | discrete | dry mass 3 g each; spent mass 12 g each (9 g absorbed water) | box of 40 | dry → spent |
| Kettle | reusable | holds water; needs no person while boiling | 1 | empty → filled(1500 g) → boiling(1500 g, 500 000 J) → empty |
| Teapot | reusable (leaves with the product) | holds teabags, then tea | 1 | empty → loaded(3 bags) → pot of tea(1473 g, 480 000 J) |
| Person | reusable | time budget | 300 000 ms (5 min) | budget draws down |
| Waste heat | continuous (waste) | joules | 50 000 + 20 000 J | — |

## 4. System boundary: suppliers, consumers, sinks

**Inputs (suppliers / sources):**

| What enters | Via | Capacity | Real or placeholder? |
|---|---|---|---|
| Cold water (g) | mains tap, draw process | unbounded | placeholder |
| Electrical energy (J) | grid socket, draw process | unbounded | placeholder |
| Teabags | box of teabags | 40 | placeholder: brand/vendor not modelled |
| Person, kettle, teapot, bin | kitchen setup at flow start | 1 each | placeholder |

**Outputs (consumers / sinks):**

| What leaves | Via | Capacity | Real or placeholder? |
|---|---|---|---|
| Pot of tea (with its mass and embodied energy) | the drinker | unbounded | placeholder |
| Spent teabags | food-waste bin | holds 10 items | real: kitchen bin (not emptied in this model) |
| Waste heat | kitchen air | unbounded | placeholder |
| Expended time | History (R16) | unbounded | per-branch, merged |

## 5. Processes

### P1. Fill the kettle
- **Actor(s) and reusables:** person (draws 30 000 ms), kettle, mains tap — returned.
- **Consumes:** 1500 g cold water (drawn from the mains).
- **Produces:** filled kettle (1500 g).
- **Balances:** mass 1500 = 1500; time 30 000 ms → History.
- **Satisfies:** —.

### P2. Load the pot
- **Actor(s) and reusables:** person (draws 20 000 ms), teapot, teabag box — returned (box at 37).
- **Consumes:** 3 teabags (one at a time from the box, R12).
- **Produces:** loaded pot (3 bags, 9 g dry).
- **Balances:** items 3 = 3; time 20 000 ms → History.
- **Satisfies:** REQ-002 (the loaded-pot state exists only at exactly 3 bags).

### P3. Boil
- **Actor(s) and reusables:** kettle only — **no person** (the kettle is automatic; this is
  what makes the ordering freedom in §6 real).
- **Consumes:** filled kettle (1500 g), 550 000 J drawn from the grid.
- **Produces:** boiling kettle (1500 g water, 500 000 J embodied).
- **Waste:** 50 000 J heat → kitchen air (kettle losses).
- **Balances:** mass 1500 = 1500; energy 550 000 = 500 000 + 50 000.
- **Satisfies:** —.
- **Failure modes:** none modelled in CS-1 (the kettle always boils; fallibility is CS-2's
  stress).

### P4. Pour and brew
- **Actor(s) and reusables:** person (draws 15 000 ms); kettle returned empty.
- **Consumes:** boiling kettle (1500 g, 500 000 J) + loaded pot (3 bags, 9 g).
- **Produces:** pot of tea (1473 g, 480 000 J embodied) and 3 spent teabags (12 g each, 36 g),
  removed from the pot at the end of brewing.
- **Waste:** 20 000 J heat → kitchen air (steeping losses); spent teabags → food-waste bin.
- **Balances:** mass 1500 + 9 = 1473 + 36 (= 1509); energy 500 000 = 480 000 + 20 000;
  time 15 000 ms → History.
- **Satisfies:** REQ-001 (accepts only the boiling kettle state), REQ-003 (spent bags exit
  only to the bin), REQ-004.

## 6. Flows

- Dependencies: P1 → P3 (boiling needs the filled kettle); P2 is independent of P1 and P3
  (it needs only the person, pot and box); P4 joins P3's and P2's outputs.
- **Concurrency:** P3 needs no person, so the person can load the pot (P2) *while the kettle
  boils* — or before filling it. At least two valid orders must compile:
  (a) P1, P2, P3, P4 and (b) P1, P3, P2, P4. The kettle/water branch and the pot/teabag
  branch each carry their own History, merged at P4 (R16).
- **Everything accounted at flow end:** pot of tea (with embodied energy) at the drinker;
  3 spent bags in the bin (bin at 7 remaining, stays in the kitchen); 70 000 J total waste
  heat at the kitchen air; kettle back, empty; person back with 235 000 ms; teabag box back
  at 37; merged History (3 recorded draws, 65 000 ms) with the caller.

## 7. Assumptions and placeholders

- Mains water and grid electricity are unbounded (placeholders).
- The kitchen air absorbs all waste heat (placeholder; the embodied 480 000 J leaves with the
  tea via the drinker, not via the air).
- Energy numbers are round-figure stand-ins (≈ what 1.5 l from 20 °C to 100 °C costs, plus
  losses), not calibrated measurements — calibration is the validation open question.
- The drinker takes the pot and never gives it back (pot return is out of scope).
- One person does everything; no qualifications needed to make tea (R18 unused in CS-1).

## 8. Open questions for the author

1. **Embodied energy in the type:** the spec carries each water state's energy as a second
   const parameter (`BoilingWater<G, J>` style) so the energy balance is compiler-checked end
   to end. The cheaper alternative keeps energy per-process only (asserted at P3 and P4 but
   not carried in types). Typed is proposed — confirm?
2. **Teabag count as a requirement:** REQ-002 is expressed by making the loaded-pot state
   exist only at exactly 3 bags. Acceptable, or should the pot accept 1–4 bags with 3 as this
   flow's choice (weaker type, more realistic pot)?
3. Quantities sanity check: 1500 g water, 3 bags (3 g dry / 12 g spent), 550 000 J drawn,
   5-minute person budget, bin capacity 10 — happy with these round numbers?
4. The bin is **not** emptied in CS-1 (it ends holding 3 spent bags). OK, or should the flow
   end with a bin-disposal boundary step (F-039 pattern) for completeness?
