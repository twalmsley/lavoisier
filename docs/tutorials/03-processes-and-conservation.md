# Tutorial 03 — Processes and conservation

> A process is a plain function that transforms inputs into outputs with
> nothing created and nothing lost — and the compiler checks the arithmetic.
> This tutorial covers the conserving-function shape, compile-time balance
> asserts, draw-down of containers and time budgets, and the two facts about
> *when* these checks fire that every modeller must internalise. Reference:
> [modeller's guide §3.4](../modellers-guide.md#34-processes); worked code:
> CS-1's P1–P5 in `model/cs1-pot-of-tea/src/resources.rs`. One sitting;
> previous: [02 — Resources and requirements](02-resources-and-requirements.md).

## 3.1 The conserving shape

Every process obeys R1: everything it needs moves in by value; everything it
produces — products, waste, and every reusable it borrowed — comes back out.
Inside, a conserving transform retires the input state and mints the output
state; CS-1's boil:

```rust
pub fn boil<const G: u64, const DRAW_J: u64, const EMBODIED_J: u64, const HEAT_J: u64>(
    kettle: FilledKettle<G>,
    energy: Electricity<DRAW_J>,
) -> (BoilingKettle<G, EMBODIED_J>, WasteHeat<HEAT_J>) {
    const { assert!(EMBODIED_J + HEAT_J == DRAW_J, "energy conservation violated in boil (R15): …") };
    kettle.defuse();          // the filled state ends; its water continues…
    energy.defuse();          // …and the energy continues as embodied + heat
    (BoilingKettle::mint(), WasteHeat::mint())
}
```

`mint()` and `defuse()` are crate-private (0.9): a process that mints
quantity-bearing values must live **inside the resource family's module**
(the `processes` child module, F-031), which is why downstream code can
compose processes but never invent or vanish resources. Note the waste: heat
is an ordinary conserved output (R15) — R1 already forces the signature to
return it, and tutorial 04 routes it to its sink.

## 3.2 The caller states every split; the assert checks it

The `const { assert!(…) }` block is the balance. Three things to know:

- **Outputs cannot be computed on stable Rust** (`Qty<{A + B}>` is a
  nightly feature, F-022). So the *caller* states every output magnitude —
  `boil::<1500, 550_000, 500_000, 50_000>` — and the assert verifies the
  books. You maintain the running balance by hand; the compiler checks every
  step (F-030). Tedious, honest, and a wrong number does not build.
- **One assert per dimension** (mass, energy, items, time — R15), each with a
  message phrased as the model rule being broken: your message *leads* the
  error output.
- **Not every balance needs an assert.** CS-1's fill is `mass 1500 = 1500`:
  the filled kettle carries the same `G` as the drawn water — true *by
  construction*. The spec marks each balance `(assert)` or `(structural)`;
  only asserts carry stated numbers on both sides.

Drawing from a container is the same pattern with a name: a split (R3). The
`draw_process!` macro generates it — `TAKE + LEFT == FULL` — and **overdraw
comes free**: when `TAKE > FULL`, no `LEFT` can satisfy the assert, so
drawing more than the container holds is the same compile error as a wrong
balance (R15). An empty container (`MilkBottle<0>`) is a distinct resource
that must still be accounted for.

## 3.3 Time is drawn like gas, next to the process

A person's time is a budget on the type — `Person<300_000>` has 300 000 ms
left — drawn down by `draw_time::<SPEND, LEFT, BUDGET>` exactly like the
bottle. Overspending is the same E0080. One project-specific convention
(F-048): a process **takes and returns the person unchanged**, and the draw
is its own *adjacent* step in the flow, recorded to the History under the
process's name:

```rust
let (person, filled) = fill_kettle(person, new_kettle(), water);
let (labour, person) = draw_time::<30_000, 270_000, 300_000>(person);
let history = record(history, "fill_kettle", labour);
```

(You met this triple in exercise 01; tutorial 04 explains `History`.) Why
not draw inside the process? Because the budget change is a const-parameter
change no generic bound can express — processes that draw internally lose
their requirement-phrased errors (F-048). Keep draws adjacent.

## 3.4 When the check fires — the fact that changes how you work

**F-001, the most consequential finding in the project:** `const` asserts
fire at *monomorphization*, during code generation. Consequences:

1. **`cargo check` passes violating code.** So do rust-analyzer and every
   editor diagnostic built on it. While you type, your editor is lying to
   you about balances. Only `cargo build` / `cargo test` give the verdict.
2. **Generic code that is never instantiated is never checked.** A process
   nobody calls with concrete numbers is unverified — which is why every
   process needs a test that instantiates it (R5).

This is why `ci.sh` gates on full builds (tutorial 01), and why conservation
regressions are pinned as rustdoc `compile_fail` doc-tests — snippets on the
process docs that must fail to build — rather than trybuild cases, which run
`cargo check` internally and never see them (F-003; see `boil`'s doc-test in
the CS-1 source).

## 3.5 The test-time layer: the tripwire

One leak shape no compile-time layer can catch: a resource that is *named
and used*, then quietly dropped at the end of scope (Rust's types are affine
— F-002). The macros' answer is the tripwire `Drop` (2.2): at test time, an
abandoned consumable panics

```text
resource leak: SpentTeabag dropped without being consumed (R1 conservation)
```

turning a silent loss into a failing test. The panic reports the `Drop`
impl's line, *not* the leak site — so keep processes small and tests
per-process, and the failing test localises the leak (R5, F-008). You will
trigger one deliberately in the capstone.

---

## Checkpoint

```sh
cd learn
cargo check --test ex03_conservation
```

You should see it finish **with no errors** — `Finished `dev` profile …` —
even though the exercise's milk balance is wrong. Now ask for the truth:

```sh
cargo test --test ex03_conservation
```

```text
error[E0080]: evaluation panicked: conservation violated in draw_milk (R15): TAKE + LEFT must equal FULL - is more milk being taken than the bottle holds, or is the stated remainder wrong?
```

with the note `evaluation of `draw_milk::<300, 1800, 2000>::{constant#0}`
failed here` — the wrong numbers, right in the error. Fix the `// TODO`
(restate what is LEFT) and re-run to `test result: ok. 1 passed`.

Then one experiment on the fixed file: change the draw to take `2_500` (and
`LEFT` to anything). Re-run `cargo test` — the *same* error: an overdraw is
just a balance no remainder can satisfy. Put it back to `300, 1_700, 2_000`
and confirm the test passes.

Next: [04 — The boundary, flows and History](04-boundary-flows-and-history.md).
