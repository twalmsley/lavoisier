# Tutorial 05 — Reading the errors

> The compiler speaks borrow-checker; the model speaks conservation. You have
> already met every major error family in the exercises — this tutorial makes
> the translation explicit and permanent. It is a guided reading of
> [modeller's guide §4](../modellers-guide.md#4-reading-compiler-errors),
> which is the full table with worked examples: learn the shapes here, keep
> §4 open while you model. One sitting; previous:
> [04 — Boundary, flows and History](04-boundary-flows-and-history.md).

## 5.1 The five you have already produced

| You saw it in | Error | The model is saying |
|---|---|---|
| ex01 | `E0382` use of moved value: `person` | this resource is already in use by another process (or was lost upstream) — take it from that process's output |
| ex02 | `E0277` phrased as a REQ sentence | an unmet requirement; the `help:` names the characteristic — and usually the missing process step |
| ex03 | `E0080` evaluation panicked, with one of *our* messages | a conservation violation or an overdraw; the wrong numbers are in the `draw_milk::<300, 1800, 2000>`-style note |
| ex04 | `E0277` "cannot supply anything: it is exhausted…" | a capacity problem; fix the boundary (refill, enlarge) — not the asker |
| ex05 | `E0308` expected `BoiledEgg`, found `RawEgg` | a process step is missing, or a resource is routed to the wrong place |

Add one you haven't: `E0451 field `_seal` … is private` (or E0423) — you
tried to create a resource from nothing; resources enter only through the
boundary (0.4, R1/R12).

## 5.2 Where to look inside an error

- **In an E0277**, read three lines: the top message (the requirement, in
  your own words), the `help:` ("the trait `Boiling` is implemented for
  `BoilingKettle<G, E>`" — i.e. *the state you should have produced*), and
  the `note: required by a bound in `pour_and_brew`` (the process that is
  refusing).
- **In an E0080**, your assert message leads, and the instantiation note
  carries the decimal magnitudes — the actual wrong numbers. When the
  violating call crosses a crate boundary (your flow calling model-core's
  `draw_time`), the primary span lands in Rust's own `core/src/panic.rs`:
  ignore that, read the **"while instantiating `fn draw_time::<30000, 0,
  20000>`"** note, which points at your call site.
- **In an E0308**, the expected/found pair *is* the diagnosis: expected the
  processed state, found the unprocessed one.

## 5.3 The three habits

1. **Never trust `cargo check` or your editor about balances** (F-001). An
   E0080 only appears on `cargo build`/`cargo test`. You proved this to
   yourself in exercise 03; it stays true forever.
2. **Ignore the compiler's repair suggestions on resources** (F-007). Every
   fix-it that says `let _ = …`, `drop(…)`, "consider borrowing", "consider
   cloning", or "consider using `Result::expect`" is a conservation
   violation dressed as a fix — each mechanism's official suggestion is
   precisely the next leak path. The real fix is always: pass the resource
   on, return it, or hand it to a Consumer.
3. **Expect type-level numbers to look ugly** (F-009). Aliases are erased:
   `N37` prints as a raw `Succ<Succ<…>>` nest with no decimal value; from
   capacity ~130 rustc spills long types to `target/…/long-type-*.txt` side
   files; the same bound error can appear twice at one call site (one
   mistake, not two). Domain wrappers (`TeabagBox<…>`) keep their outer name
   — bare numbers don't.

## 5.4 One failure that isn't a compile error

A tripwire panic at *test* time —

```text
resource leak: SpentTeabag dropped without being consumed (R1 conservation)
```

— means a resource was named and used but never reached a consumer. No
compile-time layer can catch that shape (F-032). The panic reports the
`Drop` impl's line, not the leak site: the failing *test* localises the
leak, which is why tests stay per-process. You will trigger one on purpose
in the capstone.

## 5.5 The pinned errors are documentation

Every error family in this project is pinned as a compile-fail test with its
expected output stored next to it — the `.stderr` files under
`model/*/tests/ui/` are a browsable catalogue of "what it looks like when
you get it wrong", kept honest by CI. When you meet an unfamiliar error,
grep the `.stderr` files before anything else.

---

## Checkpoint

Read two pinned errors straight from the repository. First, the unmet
requirement:

```sh
cat model/cs1-pot-of-tea/tests/ui/pour_non_boiling_kettle.stderr
```

The first line should be exactly:

```text
error[E0277]: this kettle may not be poured into the pot: `FilledKettle<1500>` is not at the boil (REQ-006)
```

Find, further down, the `help:` naming `BoilingKettle<G, E>` — the missing
process step (`boil`) — and the `note:` naming `pour_and_brew` — the process
that refused. Then the wrong-routing case:

```sh
cat model/cs1-pot-of-tea/tests/ui/spent_bag_straight_to_council.stderr
```

```text
error[E0308]: mismatched types
```

with `expected `FoodWaste<_>`, found `SpentTeabag`` — the council takes only
the bin's bagged waste, so a loose spent teabag's only route out is through
the bin: requirement REQ-008 enforced *structurally*, by the shape of the
model rather than by a named bound.

If both read like sentences to you now, you are ready to build a model of
your own. Next: [06 — Capstone: model making toast](06-capstone-toast.md).
