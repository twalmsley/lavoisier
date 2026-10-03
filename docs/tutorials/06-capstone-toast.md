# Tutorial 06 — Capstone: model making toast

> Everything in one exercise. Below is a small specification in the project's
> own template shape ([`SPEC_TEMPLATE.md`](../../SPEC_TEMPLATE.md), abridged);
> `learn/tests/ex06_capstone_toast.rs` is that spec, implemented — with three
> deliberate holes, one per checking layer. You will close them in order, and
> in doing so meet the full method end to end. One sitting; previous:
> [05 — Reading the errors](05-reading-the-errors.md).

## 6.1 The specification — TOAST-1: making toast

### 1. Purpose and scope

One person makes two slices of toast in a kitchen, using an electric
toaster. **System boundary:** the kitchen — grid electricity and the bread
bag's contents enter; toast, water vapour and waste heat leave (or rest in
boundary consumers). **Out of scope:** butter and serving; burnt toast
(fallibility is the pilot's stress, not this model's); cost.

### 2. Requirements

- **REQ-902:** All waste heat must be accounted to the kitchen-air sink.

(One requirement only — ids in `learn/` use the 900s so they can never
collide with the `model/` workspace, where requirement ids are
workspace-global, F-053.)

### 3. Resources

| Resource | Kind | Quantity & unit | States |
|---|---|---|---|
| Bread slice | discrete | 40 g each; bag of 4 | bread → toast |
| Toast slice | product | 36 g; 25 000 J embodied | — |
| Electrical energy | continuous | 60 000 J drawn | — |
| Water vapour | continuous (waste) | 8 g | — |
| Waste heat | continuous (waste) | 10 000 J | — |
| Toaster | reusable | 1; needs no person while it runs | — |
| Person | reusable | time budget 120 000 ms | budget draws down |

### 4. System boundary

Inputs: bread (bread bag, capacity 4, placeholder); electrical energy (grid
socket, draw process, unbounded, placeholder). Outputs: toast (the eater,
unbounded, placeholder); water vapour and waste heat (kitchen air, unbounded,
placeholder); expended time (History, R16).

### 5. Processes

**P1 — toast two slices.** Actor: person (draws 60 000 ms — an adjacent
draw, recorded to the History; the toaster itself needs no person while it
runs). Consumes: 2 bread slices, 60 000 J. Produces: 2 toast slices
(25 000 J embodied each). Waste: 8 g water vapour and 10 000 J heat → kitchen
air; the heat routing carries REQ-902. Balances: mass
40 + 40 = 36 + 36 + 8 **(assert)**; energy 60 000 = 25 000 + 25 000 + 10 000
**(assert)**; time 60 000 ms → History (structural).

### 6. Flows

Take 2 slices from the bag → P1 → toast to the eater, vapour and heat to the
air. **Everything accounted at flow end:** toast at the eater; vapour and
heat at the air; bread bag back at 2; toaster and socket back; person back
with 60 000 ms; the History holding one recorded draw.

## 6.2 How the exercise is built — and the three holes

Open `learn/tests/ex06_capstone_toast.rs` and skim top to bottom: resources
(§3 as macro invocations), the characteristic and requirement (§2), the
boundary (§4 — bag, draws, sinks, all `Placeholder:`-tagged), the process
(§5, with both asserts), the flow (§6, as a test, ending in the
"everything accounted" block). It is CS-1 in miniature; every section follows
a tutorial you have done.

The three `// TODO (fix N of 3)` holes fail at three different layers, and
you will meet them **in order** — type-check first, then monomorphization,
then test time:

**Fix 1 — the missing characteristic (tutorial 02).**

```sh
cd learn
cargo test --test ex06_capstone_toast
```

```text
error[E0277]: waste heat may not go here: `KitchenAir` is not the kitchen-air sink (REQ-902)
```

(twice: once at the `satisfies!` assertion, once at the `vent_heat` call —
the tag and the bound failing together). The air *is* the sink; the model
just doesn't say so. One line.

**Fix 2 — the wrong balance (tutorial 03).** Re-run, and now:

```text
error[E0080]: evaluation panicked: energy conservation violated in toast_two_slices (R15): the two slices' embodied energy plus the waste heat must sum exactly to the energy drawn from the grid
```

Notice your editor showed you nothing between fix 1 and fix 2 — fix 1 was a
type-check error (visible everywhere), this one fires at monomorphization
(F-001). The spec's P1 has the right numbers; restate the split.

**Fix 3 — the leak (tutorial 03/05).** Re-run: it *compiles* now, and the
test fails at runtime:

```text
resource leak: ToastSlice<25000> dropped without being consumed (R1 conservation)
```

The toast was made and then left on the counter — named, used, never
consumed: the shape only the tripwire catches (F-032). Route both slices to
the eater and re-run:

```text
test result: ok. 1 passed; 0 failed; …
```

## 6.3 Prove the whole course

```sh
cargo test
```

All six exercises green is the course complete. If any other exercise is
still red, its tutorial is listed at the top of the file.

## 6.4 Where to go from here

- **Extend TOAST-1** in place (it's a sandbox): add butter — a new container
  resource, a `butter_toast` process with a mass assert, a `ButteredToast`
  state the eater accepts (one type per state!). Or make the bag run dry:
  change the flow to toast four slices and watch the boundary refuse at six.
- **Read CS-1 end to end**, spec first:
  `case-studies/cs1-pot-of-tea/SPEC.md`, then
  `model/cs1-pot-of-tea/src/` in the order requirements →
  characteristics → resources → `tests/flows.rs`. After this course, nothing
  in it should surprise you.
- **Model something real.** Copy `SPEC_TEMPLATE.md`, fill it in, and work
  with the [modeller's guide](../modellers-guide.md) open — §3 is the
  walkthrough, §4 the error table, §5 the rules you must not break. Remember
  to take your requirement ids from the next free workspace number (F-053),
  and to finish with `cd model && ./ci.sh`.

That is the course. The compiler is your auditor now — argue with it; it is
usually right about your model.
