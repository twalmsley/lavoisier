# Tutorial 04 — The boundary, flows and History

> Everything enters the model through an explicit supplier or draw, and
> leaves through an explicit consumer — including expended time, whose sink
> is the execution History. Then flows wire the processes together, and the
> types decide which orders are possible. Reference:
> [modeller's guide §3.3 and §3.5](../modellers-guide.md#33-the-boundary);
> worked code: the `boundary` module in
> `model/cs1-pot-of-tea/src/resources.rs` and the flows in
> `model/cs1-pot-of-tea/tests/flows.rs`. One sitting; previous:
> [03 — Processes and conservation](03-processes-and-conservation.md).

## 4.1 Four ways across the boundary

Resources cannot be conjured (0.4) — they cross the system boundary (R12)
through exactly four shapes, all living in a `boundary` child module of the
resource family (it must be a child module: that's where `mint` is visible,
F-006):

1. **Discrete suppliers** hold the real objects they will supply —
   CS-1's `TeabagBox<Items>` holds forty actual `DryTeabag` values.
2. **Draw processes on unbounded sources** for continuous material — the
   mains tap, the grid socket: `draw_cold_water::<1500>(tap)`. (The
   `Supplier` trait is discrete-only, F-028.)
3. **Fill functions** — `full_teabag_box::<N40>()` is the one place teabags
   come into existence.
4. **Consumers / sinks** on the way out — the drinker, the bin, the kitchen
   air, the History.

Everything assumed rather than known carries a `/// Placeholder: <what needs
refining>` doc line (R12) — so the model's assumptions are a greppable list,
and refining one (a real vendor, a finite heat store) is the normal path.

## 4.2 Suppliers: the count *is* the contents

A supplier's contents are a type-level list of real values:
`TeabagBox<Cons<DryTeabag, Cons<DryTeabag, …>>>`. The count is the list's
length, so count and contents cannot disagree; capacities are counted with
type-level numbers (`N40`, aliases over `Succ<Succ<…Zero>>` — the reason for
the `recursion_limit` attribute, 1.3). `Supplier` is implemented **only for
a non-empty box**, so supplying from an empty one is a compile error with a
modeller-phrased message — capacity is in the type, exhaustion is caught
before anything runs.

Taking N items is **one where-clause**, never a hand-chained ladder of
bounds (F-014): CS-1's `load_pot` takes
`S: SupplyN<N3, Taken = ThreeDryBags>` — the `Taken =` equality doubles as
the item-type requirement.

One rule of use: reach suppliers and consumers **through the generic access
processes** — `take_one`, `take_n`, `send_to`, `send_list` from
`model_core::boundary` — never by calling `.supply()`/`.consume()` on a
concrete value. The direct call fails down a different compiler path (E0599)
that bypasses all the modeller-phrased messages (F-015).

## 4.3 Consumers and sinks

Two legal shapes:

- **Finite, contents-keeping** — CS-1's `FoodWasteBin<Space, Contents>`:
  consuming decrements `Space` and keeps the real spent bag in `Contents`.
  The decreasing space parameter is a **hard rule** (F-034): without it,
  trait resolution diverges and — at this project's mandated recursion limit
  — crashes the compiler with no diagnostic at all. (Kept tripwired contents
  also mean the bin can't simply leave the model; it is emptied through a
  sealed disposal process, CS-1's P5 — guide §3.5, F-039.)
- **Unbounded, `type Next = Self`** — the kitchen air, the drinker: legal
  *only at the boundary*, always placeholders, and they necessarily discard
  their intake (F-029). This is the one sanctioned exception to "a consumer
  keeps what it consumes".

## 4.4 History: the sink for expended time

Labour can't vanish (it's a conserved output of `draw_time`, 3.3), so it
needs a production-legal consumer. That consumer is `History` (R16): "the
past" as an explicit sink, which doubles as the execution record.

```rust
let history = new_history();
// … after each adjacent draw:
let history = record(history, "fill_kettle", labour);
// at the end: history.event_count(), history.entries()
```

Each entry carries the process name, what was consumed, magnitude and unit —
enough to generate an execution report from. Concurrency rule: **one History
per concurrent branch**, merged at joins with `merge` — a single global
History threaded everywhere would serialize the whole model (R2 + R16). CS-1
has one actor, so one History travels with the person; the merge machinery is
demonstrated in `model/model-core/src/history.rs` (module doc-test).

## 4.5 Flows: integration tests, outside the wall

A flow is just function composition, written as an integration test
(`model/cs1-pot-of-tea/tests/flows.rs`). That location is load-bearing: an
integration-test file is its own crate, *outside* your privacy boundary — it
cannot mint or defuse anything, so it lives under exactly the discipline a
downstream user does. Every input must come from a boundary supplier; every
output must genuinely reach a consumer.

Two conventions close a flow:

- **The model describes connections, not sequences** (R9). Any order that
  type-checks is valid. CS-1's kettle boils with *no person*, so "load the
  pot while the kettle boils" and "load it first" both compile — and both
  are tests, ending in the identical end state. An impossible order — using
  a product before the process that makes it — is a type error (you'll
  produce one in the checkpoint).
- **Everything accounted at flow end.** The last block of each flow names
  where every resource rests: products at their consumers, waste at its
  sinks, reusables back, empty containers accounted, the person's remaining
  budget checked, the History's entries counted. Nothing is "just done
  with"; everything is *somewhere*.

---

## Checkpoint

Two exercises, one for each half of the tutorial.

**The boundary** — a supplier that cannot cover the request:

```sh
cd learn
cargo test --test ex04_boundary
```

```text
error[E0277]: `BiscuitTin<Nil>` cannot supply anything: it is exhausted, or it is not a supplier of discrete items
```

The guest asks for three; the tin was filled with two; the compiler walks
the supply chain and finds the empty tin (`BiscuitTin<Nil>`) one step short
— note the `help:` lines naming both the empty state and the
`Cons<H, T>` state that *can* supply. Fix the `// TODO` **at the boundary**
(fill the tin properly — not by asking for less) and re-run to
`test result: ok. 1 passed`.

**The flow** — a missing step:

```sh
cargo test --test ex05_flow_order
```

```text
error[E0308]: mismatched types
   …
   expected `BoiledEgg`, found `RawEgg`
```

Read it as the model speaking: *this egg has not been boiled yet* (F-023).
Add the missing process step at the `// TODO` and re-run to
`test result: ok. 1 passed`.

Then open `model/cs1-pot-of-tea/tests/flows.rs` and read flow order (b) —
the pot loaded *while* the kettle boils — against order (a): the same five
processes, one reordering, the identical "everything accounted" block. That
file is the finished form of what you just fixed.

Next: [05 — Reading the errors](05-reading-the-errors.md).
