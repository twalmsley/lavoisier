# Model Specification — <system name>

> Template for natural-language specifications that are implemented as models using
> `model-core` (see `instructions.md`; R-numbers below refer to it). Copy this file,
> replace every `<angle-bracket>` item, delete guidance quotes like this one, and delete
> any section that genuinely doesn't apply (say so rather than leaving it blank).
>
> **How this is used:** the implementer (human or Claude) turns each section into model
> code mechanically — requirements become `REQ-NNN` traits, resources become sealed types,
> processes become conserving functions, the boundary becomes suppliers/consumers — and
> `ci.sh`/`trace.sh` then prove the model matches this document. **Anything this
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

> One sentence each, numbered `REQ-001` upward (three digits, sequential, never reused —
> R10). Each becomes a requirement trait verbatim, so phrase each as a checkable property
> of a *thing* or a *process* ("X must be/have Y"), not as a wish. Every requirement will
> need at least one verifying test; if you can't imagine the test, rephrase the
> requirement.

- **REQ-001:** <e.g. Fastening bolts must be M8 steel, 15 mm long.>
- **REQ-002:** <…>

## 3. Resources

> One row per resource. *Kind* is one of:
> **discrete** — countable items, each its own object (R13);
> **continuous** — an amount of material/energy/time in a container (R15);
> **reusable** — moved in and returned, survives the flow (people, tools, locations — R2);
> **product** — what the model exists to produce.
> Quantities are integers in the base units (R7): g, mm, ms, mm², mm³, mK, mA, J.
> *States*: list every processing state separately — one type per state (R9), so "plate"
> and "drilled plate" are two rows or one row listing both states.

| Resource | Kind | Characteristics (markers/parameters) | Quantity & unit | States |
|---|---|---|---|---|
| <Bolt> | discrete | <M8, Steel, 15 mm> | <count: 100 per box> | — |
| <Steel sheet> | continuous | <grade?> | <5000 g> | <sheet → blanks> |
| <Person> | reusable | <qualifications?> | <time budget: 30 000 ms> | — |

## 4. System boundary: suppliers, consumers, sinks

> Everything enters through a supplier and leaves through a consumer (R12). Discrete
> suppliers hand over one item per step; continuous sources are draw processes (R15).
> **Every waste product in §5 must have a destination row here.** Mark a row *placeholder*
> if the real supplier/consumer isn't decided yet — that's allowed and visible (R12).
> Capacity "unbounded" is legal only at the boundary and is always a placeholder (R15).

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
| <Expended time> | History (R16) | unbounded | per-branch, merged at joins |

## 5. Processes

> One block per process. A process takes everything it needs by value and returns
> everything it produces — nothing appears or disappears (R1). **The balances must
> actually balance**; the compiler checks them, so wrong numbers here fail the build.
> If a process needs payment, orders or receipts, it is a process chain, not a supplier
> (R12) — spell out the chain.

### P1. <cut>
- **Actor(s) and reusables:** <1 person (≥ 5000 ms of their budget), the saw> — returned.
- **Consumes:** <steel sheet, 5000 g>.
- **Produces:** <2 plate blanks (2 × 2250 g)>.
- **Waste:** <swarf, 500 g → swarf bin (§4)>.
- **Balances:** <mass: 5000 = 2250 + 2250 + 500>; <time drawn: 5000 ms → History>.
- **Satisfies:** <REQ-00N, if any>.
- **Failure modes:** <none modelled / describe — see instructions.md open question 1>.

### P2. <…>

## 6. Flows

> Only connections, never a fixed order (R9): which process outputs feed which process
> inputs. State what may run concurrently — independent branches each carry their own
> History (R16). The implementer will compose at least two valid orders and prove both
> compile.

- <P1's blanks feed P2 and P3; P2 and P3 are independent (separate people) and may run
  concurrently; P4 joins their outputs and merges their histories.>
- **Everything accounted:** <at flow end, name where every resource rests: products at the
  customer, swarf bin disposed, empty containers consumed, reusables returned, histories
  merged.>

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
