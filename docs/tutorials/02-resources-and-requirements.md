# Tutorial 02 — Resources and requirements

> Things become types; what must be true of them becomes traits. This
> tutorial takes you from a spec's resource table to sealed resource types,
> and from a spec's requirement sentences to compile-checked requirement
> traits. Reference: [modeller's guide §3.1–§3.2](../modellers-guide.md);
> worked code: `model/cs1-pot-of-tea/src/`. One sitting; previous:
> [01 — Setting up](01-setting-up-a-model-crate.md).

## 2.1 One type per state

CS-1's spec (`case-studies/cs1-pot-of-tea/SPEC.md`, §3) lists the kettle's
states: `empty → filled(1500 g) → boiling(1500 g, 500 000 J) → empty`. In the
model that is **three types** (`model/cs1-pot-of-tea/src/resources.rs`):

```text
Kettle  →  FilledKettle<1500>  →  BoilingKettle<1500, 500_000>  →  Kettle
```

Never reuse one type for two states (R9, F-023). The payoff is the whole
method: "poured before boiling" isn't a bug you find at runtime — it *cannot
be written*, because `pour` doesn't accept a `FilledKettle`. The quantities
ride in the type as const generics (0.8): mass in grams, embodied energy in
joules, integers in base units (R7).

## 2.2 The kernel macros write the types for you

You *could* hand-write sealed structs (0.4), but the sealing pattern is one
attribute away from silently broken — so you don't. `model-core`'s macros
generate the type with everything R1 demands: the private seal field,
`#[must_use]`, no `Clone`/`Copy`/`Default`, a crate-private `mint()`
constructor, and (where appropriate) a leak-detecting `Drop` plus a
`test_fixture()` for downstream tests. Which macro for which row of the spec:

| Spec says | Macro | CS-1 example |
|---|---|---|
| an amount of material/energy, a container, waste | `container_resource!` | `ColdWater<G>`, `WasteHeat<E>`, `FilledKettle<G>` |
| a countable item, its own object | `consumable_resource!` | `DryTeabag`, `SpentTeabag`, `LoadedPot` |
| moved in and returned by every process (people, tools) | `reusable_resource!` | `Kettle`, `Teapot`, `MainsTap` |
| "this step can fail" (not in CS-1) | `outcome_token!` | the pilot's `DrillOutcome` |

A real invocation, from `model/cs1-pot-of-tea/src/resources.rs`:

```rust
model_core::container_resource! {
    /// Cold water drawn from the mains, in grams (R7 base mass unit; SPEC.md
    /// §3: 1500 g drawn). …
    ColdWater,
    unit = "grams",
    must_use = "ColdWater is a conserved resource: pass it on or hand it to a Consumer"
}
```

The macro appends the magnitude parameter itself: this defines
`ColdWater<const V: u64>`, with `ColdWater::<1500>::VALUE == 1500` and
`UNIT == "grams"`. The `must_use` string is what the compiler prints at
anyone who discards the value — write it as an instruction to the modeller.
A state carrying *two* quantities declares the extra one:
`BoilingKettle<const WATER_G: u64>` expands to `BoilingKettle<WATER_G, V>`,
used as `BoilingKettle<1500, 500_000>`.

Two conventions worth absorbing now:

- **Constants over comments.** Masses the balances need live on the types —
  `DryTeabag::MASS_G` is 3, `SpentTeabag::MASS_G` is 12 — so tests recover
  the spec's numbers from the model instead of restating them.
- **Tripwire or not.** Consumables get a `Drop` that panics
  `resource leak: …` at test time if the value is abandoned — the only layer
  that catches a named-but-never-consumed resource. Items *kept inside* a
  container that accounts for them (the teabags in the box) are declared
  `no_tripwire` (F-040). Reusables have no tripwire — they legitimately
  outlive the flow. Tutorial 03 shows a tripwire firing.

## 2.3 Characteristics: traits carrying facts

A characteristic (R6) is a marker trait on exactly the states that have it,
with a modeller-phrased error message for everything that doesn't
(`model/cs1-pot-of-tea/src/characteristics.rs`):

```rust
#[diagnostic::on_unimplemented(message = "`{Self}` is not a kettle at the boil (REQ-006: only the boiling state can be poured)", …)]
pub trait Boiling: sealed::Sealed {
    const WATER_G: u64;      // a measured characteristic can carry its
    const EMBODIED_J: u64;   // quantities (R6/R7) — tutorial 03 uses these
    …
}

impl<const G: u64, const E: u64> Boiling for BoilingKettle<G, E> { … }
```

`{Self}` in the message is replaced by the offending type. The
`sealed::Sealed` supertrait stops outside code implementing the trait for its
own types and smuggling fake "boiling water" past a requirement — note the
pattern, use it when your characteristic guards a requirement.

## 2.4 Requirements: traits used as bounds

Each spec requirement becomes a named trait via `model_core::requirement!`
(`model/cs1-pot-of-tea/src/requirements.rs`):

```rust
model_core::requirement! {
    /// REQ-006: Tea must be brewed with boiling water (only the boiling state of the kettle can be poured into the pot).
    #[diagnostic::on_unimplemented(message = "this kettle may not be poured into the pot: `{Self}` is not at the boil (REQ-006)", …)]
    pub trait Req006PouredAtTheBoil: (Boiling);
    assert = assert_req006;
}
```

Reading it: the requirement trait's *supertrait* is the characteristic
(parenthesised — a macro parsing restriction), a hidden blanket impl makes
every type with the characteristic satisfy the requirement automatically, and
`assert_req006` is a helper you'll meet in a moment. Requirements are used
**only as bounds** in process signatures — `fn pour_and_brew<K:
Req006PouredAtTheBoil, …>` — never as concrete types (F-019).

Rules that bite, learned the hard way (each is a finding):

- **REQ ids are workspace-global (F-053).** The pilot owns REQ-001..005, so
  CS-1's spec REQ-001..004 shipped as REQ-006..009. Before numbering, check
  the latest trace.sh report for the highest id in use: a reused id keeps CI
  green while the report silently merges your requirement into another
  model's.
- The `/// REQ-NNN: <one sentence>.` line goes **literally inside the macro
  braces** — that is what keeps it visible to the traceability grep (F-021).
- The `on_unimplemented` message is **one line**, phrased as the requirement,
  naming its id. When a characteristic fails underneath a requirement bound,
  only the *requirement's* message is shown (F-044) — this message is the one
  modellers will actually see.

## 2.5 The traceability pair: tag + assertion

Which types satisfy which requirements must be reportable. The blanket impl
means no source line ever says `impl Req006… for KettleAtTheBoil`, so the
link would be invisible to grep (F-020). Hence the pair:

```rust
/// Satisfies: REQ-006
pub type KettleAtTheBoil = BoilingKettle<1500, 500_000>;
model_core::satisfies!(assert_req006, KettleAtTheBoil);
```

The doc tag is for grep (the trace.sh report); the `satisfies!` line plants a
compile-time assertion, so a **stale tag is a compile error** naming the
missing characteristic. Never put a `Satisfies:` tag *inside* a macro
invocation — trace.sh drops it silently (F-037); tag an alias beside it, as
above. Tests carry the mirror tag: one `/// Verifies: REQ-006, REQ-007` line
in the doc comment directly above each `#[test]`, and every requirement needs
at least one verifying test or the gate fails.

---

## Checkpoint

```sh
cd learn
cargo test --test ex02_requirements
```

The exercise defines two water states and requirement REQ-901, but nobody has
said which state is at the boil. You should see exactly this error, twice
(once at the `satisfies!` assertion, once at the `brew` call):

```text
error[E0277]: this water may not be used to brew: `KettleOfBoilingWater` is not at the boil (REQ-901)
```

— your own `on_unimplemented` text, with the compiler's `help:` underneath
pointing at the `Boiling` trait that has no implementations. Follow the
`// TODO` (one line fixes both errors), re-run, and you should see
`test result: ok. 1 passed`.

Then, two experiments on the fixed file:

1. In the test, hand `brew` the **cold** water instead. Read the error — the
   same REQ-901 sentence, now about `KettleOfColdWater`. This is R10 working:
   the requirement, not the type name, is the error message. Put it back.
2. Make the `Satisfies:` claim stale: move the `impl Boiling` line to the
   cold state and watch `satisfies!` catch the lie. Put it back, and confirm
   `cargo test --test ex02_requirements` passes again.

Next: [03 — Processes and conservation](03-processes-and-conservation.md).
