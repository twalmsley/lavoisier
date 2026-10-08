# Model Specification — <system name>

> Template for natural-language specifications that are implemented as models using
> `model-core` (see `instructions.md`; R-numbers below refer to it). Copy this file,
> replace every `<angle-bracket>` item, delete guidance quotes like this one, and delete
> any section that genuinely doesn't apply (say so rather than leaving it blank).
>
> **How this is used:** the implementer (human or Claude) turns each section into model
> code mechanically — requirements become `REQ-NNN` traits, resources become sealed types,
> processes become conserving functions, the boundary becomes suppliers/consumers — and
> `ci.sh`/`trace.sh` then prove the model matches this document. `tools/spec.sh` validates
> this document mechanically (R22): balance arithmetic, waste destinations, name closure
> and Satisfies claim segments (A11) are checked at spec-review time, before any code
> exists. **Anything this
> specification leaves out does not silently default: it comes back as a numbered
> question.** Under-specification is cheap to fix here and expensive to fix later, but the
> compiler will catch it either way — a missing waste destination or an unbalanced mass
> cannot compile.

| | |
|---|---|
| Specification version | <v0.1> |
| Date | <YYYY-MM-DD> |
| Author | <name> |
| Status | draft / agreed |

## 1. Purpose and scope

<Two or three sentences: what real-world system this models, and why.>

**System boundary:** <one sentence on where the model stops — everything that crosses this
line must appear in §4.>

**Out of scope:** <what this model deliberately does not cover (e.g. scheduling, cost,
failure handling), so its absence is a decision, not an oversight.>

## 2. Requirements

> One sentence each, with `REQ-NNN` ids (three digits, sequential, never reused — R10).
> Ids are **workspace-global** (F-053): number from the next free id across the whole
> workspace (check the latest trace.sh report for the highest in use), not from 001 — a
> reused id leaves the CI gate green while the traceability report silently merges your
> requirement with another model's. Each becomes a requirement trait verbatim, so phrase each as a checkable property
> of a *thing* or a *process* ("X must be/have Y"), not as a wish. Every requirement will
> need at least one verifying test; if you can't imagine the test, rephrase the
> requirement.

**Ids:** REQ-0NN–REQ-0MM, allocated from the workspace sequence (F-053; check the latest
trace.sh report for the highest id in use).

- **REQ-0NN:** <e.g. Fastening bolts must be M8 steel, 15 mm long.>
- **REQ-0NN+1:** <…>

## 3. Resources

> One row per resource. *Kind* is one of:
> **discrete** — countable items, each its own object (R13);
> **continuous** — an amount of material/energy/time in a container (R15);
> **reusable** — moved in and returned, survives the flow (people, tools, locations — R2);
> **product** — what the model exists to produce.
> Quantities are integers in the base units (R7): g, mm, ms, mm², mm³, mK, mA, J — written
> with underscore grouping (`550_000`) or ungrouped (`550000`), never space-grouped
> (`550 000`), so machine parsing needs no lexing heuristics.
> *States*: list every processing state separately — one type per state (R9), so "plate"
> and "drilled plate" are two rows or one row listing both states. A state carrying
> quantities lists them in parentheses after the state name, one
> `quantity unit` term per const parameter in order:
> `` `filled` (1_500 g) → `boiling` (1_500 g, 500_000 J) ``. In code the quantities ride as
> const parameters on the state type (`BoilingKettle<1500, 500_000>`); free-form phrasing
> ("dry mass 3 g each") needed pattern-scraping in EXP-15 and is not machine-safe.
> **Each Resource cell opens with a backticked canonical identifier** (`` `Teapot` ``), and
> each state in the States cell is likewise backticked (`` `boiling` ``). §4, §5 and §6 refer
> to resources and states **by these identifiers verbatim**; surrounding prose is decoration.
> The validator and generator resolve names only through the identifiers — prose variants
> ("the loaded pot", "drawn from the grid") were the single biggest parsing fragility in
> EXP-15 (F-062).
> Each state name appears on **exactly one** resource row — the quantity-bearing row that
> owns the type. Do not repeat `boiling` on both the water and the kettle: name the owning
> row's state and let prose mention the rest.
> Every waste product named in §5 has its own row here — waste is a resource like any other
> (R1), and a §5 waste line with no §3 row fails speccheck.
> Two §3 phrasings are load-bearing for the generator (F-066), so write them verbatim: a
> **person row** marks itself with the parenthetical alias `(person)` after the resource
> name (`` `Baker` (person) ``) — the alias binds the row to the kernel Person budget
> machinery, and an actor row without it scaffolds as a plain reusable with **no time
> budget**; an **outcome-token row** (R17) writes the Kind `outcome token (R17)` verbatim.

| Resource | Kind | Characteristics (markers/parameters) | Quantity & unit | States |
|---|---|---|---|---|
| <`Bolt`> | discrete | <M8, Steel, 15 mm> | <count: 100 per box> | — |
| <`SteelSheet`> | continuous | <grade?> | <5000 g> | <sheet → 2 `blank`s (2_250 g)> |
| <`Person`> | reusable | <qualifications?> | <time budget: 30_000 ms> | — |
| <`Swarf`> | waste | <per-cut mass> | <500 g per sheet> | — |

## 4. System boundary: suppliers, consumers, sinks

> Everything enters through a supplier and leaves through a consumer (R12). Discrete
> suppliers hand over one item per step; continuous sources are draw processes (R15).
> **Every waste product in §5 must have a destination row here.** Mark a row *placeholder*
> if the real supplier/consumer isn't decided yet — that's allowed and visible (R12).
> Capacity "unbounded" is legal only at the boundary and is always a placeholder (R15).
> A continuous input whose container enters and leaves with a remainder writes
> `(container with remainder)` in its Via cell — the phrasing is load-bearing for the
> generator (F-066): from that cell it emits the container type, its draw process and the
> remainder arithmetic.
> **Flow-end rests are boundary exits like any other (A13):** every resource that comes to
> rest at flow end — a container remainder, a reusable that stays behind — gets one output
> row here, with `(flow-end rest)` in its Via cell. The generator emits one `rest_*` exit
> per declared rest row and synthesizes none: an undeclared rest has no exit, and the model
> surfaces it as a tripwire panic — the designed loud failure.

**Inputs (suppliers / sources):**

| What enters | Via | Capacity | Real or placeholder? |
|---|---|---|---|
| <Bolts> | <box of bolts> | <100> | <placeholder: actual vendor TBD> |
| <Air> | <atmosphere, draw process> | unbounded | placeholder |

**Outputs (consumers / sinks):**

| What leaves | Via | Capacity | Real or placeholder? |
|---|---|---|---|
| <Finished assembly> | <customer> | unbounded | placeholder |
| <Swarf> | <swarf bin> | <holds 10> | <real: workshop bin, emptied by disposal> |
| <The saw> | <workshop rack (flow-end rest)> | — | <real: stays in the workshop> |
| <Expended time> | History (R16) | unbounded | per-branch, merged at joins |

## 5. Processes

> One block per process. A process takes everything it needs by value and returns
> everything it produces — nothing appears or disappears (R1). **The balances must
> actually balance**; the compiler checks them, so wrong numbers here fail the build.
> If a process needs payment, orders or receipts, it is a process chain, not a supplier
> (R12) — spell out the chain. Conventions the implementation relies on:
> - "Actor: person (draws N ms)" means the draw is an **adjacent** `draw_time` step,
>   recorded to the History under this process's name; the process itself takes and returns
>   the person unchanged (F-048). A process needing no person should say so — that is what
>   creates ordering freedom (§6).
> - **Every process with waste carries a `Waste routing:` field** naming one of the two
>   shapes: *consumer parameter* (the process takes its consumer as a requirement-bounded
>   parameter and feeds it internally — the waste never exists loose and the requirement
>   becomes structural) or *routed by the flow* (the loose output is the flow's to deliver).
>   A waste line with no routing field is a speccheck error (hole U-10, F-062).
> - **Every Balances clause ends in `(assert)`** — checked by a `const` assert — **or
>   `(structural)`** — true by construction (shared consts, a fixed item count). Both are
>   conservation; only asserts carry stated numbers on both sides. An unmarked clause is a
>   speccheck error, not a judgement call: CS-1's unmarked lines each forced the generator to
>   guess (F-062).
> - Consumes/Produces/Waste lists contain only items — quantity, canonical identifier,
>   optional parenthesised R-reference. Explanatory prose ("removed from the pot at the end
>   of brewing") goes in its own sentence after the list, never trailing inside an item.
> - **A `Satisfies:` value's claim segment is the text before the first full stop** (A11):
>   either a comma-separated list of REQ ids (each with an optional parenthesised note) or
>   the single mark `—`, which claims nothing. Anything after the full stop is plain prose —
>   a REQ id there is informative, never a claim. A claim segment that starts with `—` and
>   still contains a REQ id is a speccheck error (the scraped id seeded false traceability
>   tags in generated scaffolds, F-066); write the sanctioned form instead:
>   `**Satisfies:** —. Enables REQ-011 (P6's bound).`

### P1. <cut>
- **Actor(s) and reusables:** <1 person (draws 5_000 ms), the saw> — returned.
- **Consumes:** <1 `SteelSheet` (5000 g)>.
- **Produces:** <2 `blank`s (2250 g each)>.
- **Waste:** <1 `Swarf` (500 g) → swarf bin (§4)>.
- **Waste routing:** <consumer parameter / routed by the flow>.
- **Balances:** <mass 5000 = 2250 + 2250 + 500 (assert)>; <time 5_000 ms → History (structural)>.
- **Satisfies:** <REQ-0NN, if any>.
- **Failure modes:** <none modelled / describe — see instructions.md open question 1>.

### P2. <…>

## 6. Flows

> Only connections, never a fixed order (R9): which process outputs feed which process
> inputs. State what may run concurrently — independent branches each carry their own
> History (R16). The implementer will compose at least two valid orders and prove both
> compile.

- <P1's blanks feed P2 and P3; P2 and P3 are independent (separate people) and may run
  concurrently; P4 joins their outputs and merges their histories.>
- **Orders:** (a) <P1, P2, P4>; (b) <P1, P3, P4> — at least two valid orders, each a
  comma-separated list of §5 process ids; the implementer proves both compile (R9).
- **Everything accounted:** <at flow end, name where every resource rests, naming only
  destinations declared in §4 (A13: a flow-end rest needs its own §4 output row): products
  at the customer, swarf bin disposed, empty containers consumed, reusables returned,
  histories merged.>

## 7. Assumptions and placeholders

> Everything assumed rather than known. Each becomes a greppable `Placeholder:` tag or a
> findings-log entry, so nothing here gets lost.

- <e.g. The atmosphere absorbs all exhaust (unbounded sink).>
- <e.g. One person does all drilling; qualification not modelled.>

## 8. Open questions for the author

> Left deliberately for the implementer to fill in during implementation: every point
> where this specification was found to under-specify will be added here as a numbered
> question and sent back, rather than guessed at.

- *(filled in during implementation)*
