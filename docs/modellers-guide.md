# The Modeller's Guide

> The practical companion to `instructions.md`. That document is the **normative
> specification** of the modelling approach (requirements R1–R19, each backed by experiment
> evidence); this one is the **how-to**: what you actually type, in what order, and what the
> compiler says when you get it wrong. Everything here is illustrated from real code in this
> repository — mostly the CS-1 case study (`model/cs1-pot-of-tea/`, built from
> `case-studies/cs1-pot-of-tea/SPEC.md`), with the pilot (`model/pilot-workshop/`) covering
> what CS-1 doesn't: fallible processes, qualifications and safety, and money. File paths are
> given throughout so you can open the worked example next to this guide.
>
> F-numbers refer to entries in `FINDINGS.md`; R-numbers to `instructions.md`.

## Contents

1. [The mental model](#1-the-mental-model)
2. [Setting up a model crate](#2-setting-up-a-model-crate)
3. [Modelling walkthrough (CS-1)](#3-modelling-walkthrough-cs-1)
4. [Reading compiler errors](#4-reading-compiler-errors)
5. [Rules you must not break](#5-rules-you-must-not-break)
6. [Known limitations](#6-known-limitations)

---

## 1. The mental model

**Things are types.** Every resource in the modelled system — a teabag, 1500 g of water, a
person's remaining time, a kettle — is its own Rust type. Quantities are integers in base
units (grams, millimetres, milliseconds, joules, …; the R7 table) carried as const generics:
`ColdWater<1500>` is fifteen hundred grams of cold water, and `ColdWater<1400>` is a
*different type*. Processing states are different types too: a `FilledKettle<1500>` is not a
`BoilingKettle<1500, 500_000>`, so "pour before boiling" is not a runtime bug — it cannot be
written (R9).

**Resources are sealed.** Every resource type has a private field and no public constructor,
so code outside its defining crate physically cannot create one (R1). Resources enter the
model only at the **system boundary** (R12): suppliers holding real objects, draw processes
on unbounded sources (the mains tap, the grid), and fill functions. They leave only through
**consumers** (the drinker, the bin, the kitchen air, the execution `History`).

**Processes conserve.** A process is a plain function: everything it needs moves in by value,
everything it produces — products, waste, by-products, the reusable resources it borrowed —
comes back out (R1, R2). Amounts balance per dimension, checked by `const { assert!(…) }`
blocks at compile time: CS-1's brew asserts `water + dry bags == tea + spent bags` for mass
and `embodied in == embodied out + steeping heat` for energy. A balance that doesn't add up
does not build.

**The compiler is the auditor.** Because resources can't be cloned, can't be created from
nothing, and must be moved, the compiler enforces most of the bookkeeping: a resource used
by two processes at once is "use of moved value"; an exhausted supplier fails a trait bound
with a plain-English message; an unmet requirement (`REQ-006: tea must be brewed with
boiling water`) is an E0277 naming the requirement. What the compiler can't fully enforce —
Rust's types are affine, so a value *can* be silently dropped — is covered by a layered
regime of lints and a test-time tripwire `Drop` (section 5); the residual gaps are stated
honestly in section 6.

**The model describes connections, not sequences** (R9). A flow is just function
composition; any order that type-checks is valid. CS-1's kettle boils with no person, so
"load the pot while the kettle boils" and "load it first" are both compiled and both tested
(`model/cs1-pot-of-tea/tests/flows.rs`) — the type system proves the two orders reach the
identical end state.

---

## 2. Setting up a model crate

Every model is its own crate in the `model/` workspace, downstream of `model-core`. The
reference layouts are `model/cs1-pot-of-tea/` (small, continuous-resource-heavy) and
`model/pilot-workshop/` (catalogue, fallibility, qualifications, money).

### 2.1 Workspace membership

Add the crate to `model/Cargo.toml`:

```toml
[workspace]
resolver = "2"
members = ["model-core", "pilot-workshop", "cs1-pot-of-tea"]   # + your crate
```

### 2.2 The crate's `Cargo.toml`

Copy the shape of `model/cs1-pot-of-tea/Cargo.toml` exactly — every line of it is
load-bearing:

```toml
[features]
# Your own resources' test fixtures (F-004): the kernel macros expand a
# `test_fixture()` constructor behind the INVOKING crate's feature.
test-support = []

[dependencies]
# Production dependency: NO features. The plain `cargo build` in ci.sh is
# what proves production code cannot reach fixture constructors (F-004).
model-core = { path = "../model-core" }

[dev-dependencies]
# Re-declarations for tests only (R1, F-004).
model-core = { path = "../model-core", features = ["test-support"] }
your-crate = { path = ".", features = ["test-support"] }
trybuild = "1"   # the one allowed dev-dependency (R4)
```

The `test-support` feature must **never** appear under any `[dependencies]`-like section
anywhere in the workspace — `ci.sh` step 6 greps for this and fails the gate. `#[cfg(test)]`
helpers are *not* a substitute: a dependency is always compiled without `cfg(test)`, so they
can never serve downstream fixtures (F-004).

### 2.3 Crate attributes

Every modelling crate's `lib.rs` — and every integration-test file under `tests/` — carries
the full set (see `model/cs1-pot-of-tea/src/lib.rs` and `tests/flows.rs`):

```rust
#![recursion_limit = "2048"]   // type-level capacities need ≈ N + 3 (F-010); per crate, not per library
#![forbid(unsafe_code)]        // privacy is a safe-Rust guarantee only (F-005)
#![deny(unused_must_use)]      // conservation layer 1 (R1)
#![deny(let_underscore_drop)]  // conservation layer 1 (R1)
#![warn(missing_docs)]         // every public item documented (lib.rs only)
```

Do **not** add `clippy::shadow_*` lints: `shadow_reuse` fires on the conservation-correct
idiom `let kettle = boil(kettle, …)` (F-007). The rest of the lint regime (clippy
restriction lints, the `clippy.toml` disallowed-methods ban list) is supplied by the
workspace and `ci.sh`; adopt it whole or not at all — the set is only sound as a whole
(section 5).

### 2.4 The CI gate

```sh
cd model
./ci.sh
```

`model/ci.sh` is the whole verification story, six steps, any failure fails the gate:

1. **`cargo build --workspace`** — conservation asserts are E0080s at *monomorphization*
   (F-001): `cargo check` cannot see them, so the gate never rests on `check`.
2. **`cargo test --workspace`** — unit and integration tests, trybuild cases, the rustdoc
   `compile_fail` doc-tests (the only automated regressions for conservation/overdraw
   violations, F-003), and the tripwire `should_panic` demos.
3. **clippy, `-D warnings`** plus the explicit restriction lints `clippy::mem_forget` and
   `clippy::let_underscore_must_use`, with the workspace `clippy.toml` ban list.
4. **A plain no-features `cargo build`** — during `cargo test`, feature unification compiles
   your *production* sources against the featured library (F-004); only this build proves
   production code can't call `test_fixture()`.
5. **`./trace.sh`** — the R10 traceability report; any warning (a requirement with no
   verifying test, an unknown ID, a near-miss tag) exits nonzero.
6. **The feature-placement grep** — `test-support` under `[dev-dependencies]` only.

Run it before claiming anything works. Run `cargo build` (not just your editor) before
believing a conservation-sensitive change compiles — see section 4.

---

## 3. Modelling walkthrough (CS-1)

The order that works in practice mirrors the SPEC_TEMPLATE sections: requirements → resources
→ boundary → processes → flows. CS-1's spec (`case-studies/cs1-pot-of-tea/SPEC.md`) maps
onto four source files:

| File | Contents |
|---|---|
| `model/cs1-pot-of-tea/src/requirements.rs` | SPEC §2 as R10 requirement traits |
| `model/cs1-pot-of-tea/src/characteristics.rs` | the R6 characteristic traits the requirements bound over |
| `model/cs1-pot-of-tea/src/resources.rs` | SPEC §3–§5: the sealed resource family, its `boundary` and `processes` child modules |
| `model/cs1-pot-of-tea/tests/flows.rs` | SPEC §6: the flows, as integration tests |

### 3.1 Requirements

Each spec requirement becomes a named trait via `model_core::requirement!`, used **only as a
bound** in process signatures, never as a concrete type (F-019):

```rust
model_core::requirement! {
    /// REQ-006: Tea must be brewed with boiling water (only the boiling state of the kettle can be poured into the pot).
    #[diagnostic::on_unimplemented(message = "this kettle may not be poured into the pot: `{Self}` is not at the boil (REQ-006)", label = "REQ-006: tea must be brewed with boiling water", note = "boiling is a process state (R9): `boil` turns a `FilledKettle` into a `BoilingKettle` - only that state can be poured")]
    pub trait Req006PouredAtTheBoil: (Boiling);
    assert = assert_req006;
}
```

(`model/cs1-pot-of-tea/src/requirements.rs`.) Things to know:

- **IDs are workspace-unique (F-053).** `trace.sh` keys requirement ids globally across the
  workspace. The pilot owns REQ-001..005, so CS-1's spec REQ-001..004 were implemented as
  REQ-006..009 with the mapping documented at the definitions. Check the latest trace.sh
  report for the highest id in use *before* numbering; a reused id leaves CI green while the
  report silently merges your requirement with another model's — the worst failure mode,
  because nothing warns.
- The `/// REQ-NNN: <one sentence>.` line is written **literally inside the macro braces**,
  directly above the `trait` line — that is what keeps it visible to `trace.sh` (F-021). The
  supertrait bounds are parenthesised (a `macro_rules!` parsing restriction).
- **Every requirement trait carries a one-line `#[diagnostic::on_unimplemented]`** phrased as
  the requirement and naming its id (R10 rule 8). When a marker fails as a supertrait
  obligation of a requirement bound, the *marker's* message is ignored and only the
  requirement trait's is shown (F-044) — so this message is the one modellers will see. It
  must be a single line: trace.sh has no scope awareness, and a multi-line attribute between
  a doc tag and its item detaches them (F-021).
- **`Satisfies:` tags come in pairs.** The blanket impl inside `requirement!` means no source
  line ever states `impl Req006… for KettleAtTheBoil`, so the type→requirement link is
  invisible to grep (F-020). Therefore each satisfying type carries a
  `/// Satisfies: REQ-006` doc line (for grep) *plus* a `model_core::satisfies!(assert_req006,
  KettleAtTheBoil);` assertion (for truth — a stale tag becomes a type-check-time compile
  error naming the missing characteristic). See `KettleAtTheBoil` in
  `model/cs1-pot-of-tea/src/resources.rs`.
- **Never put a `Satisfies:` tag inside a kernel-macro invocation** — trace.sh drops it
  *silently* (F-037). Tag a type alias beside the invocation instead, exactly as CS-1 does:

  ```rust
  /// Satisfies: REQ-006
  pub type KettleAtTheBoil = BoilingKettle<1500, 500_000>;
  model_core::satisfies!(assert_req006, KettleAtTheBoil);
  ```
- **Tests are tagged** `/// Verifies: REQ-006, REQ-007` — one `Verifies:` line per test,
  case-sensitive, in the `///` doc block directly above the `#[test]`. Every requirement
  needs at least one verifying test or trace.sh fails CI.
- Requirement bounds stay **on the same line as the `fn` name** (R10 rule 4) — which is why
  CS-1's `pour_and_brew` signature is one very long line. Grep-based traceability has no
  scope awareness; this is the price (F-038).

### 3.2 Resources — the kernel macros

Resources are generated by the `model-core` kernel macros, which expand *in your crate* so
the generated `mint()`/`defuse()` are private to it (`pub(crate)`): only your own boundary
and processes can create or dispose of your resources. The macros produce the full R1
sealing by construction — private seal field, `#[must_use]`, no
`Clone`/`Copy`/`Default`, a tripwire `Drop` where appropriate, and a feature-gated
`test_fixture()`. Which macro for which thing:

| Macro | For | CS-1 / pilot example |
|---|---|---|
| `container_resource!` | continuous amounts, containers, waste — `Name<const V: u64>` in a fixed base unit (R15) | `ColdWater`, `Electricity`, `WasteHeat`, `FilledKettle` (`model/cs1-pot-of-tea/src/resources.rs`) |
| `consumable_resource!` | discrete items, their own objects (R13); optional held contents; optional `no_tripwire` | `DryTeabag`, `SpentTeabag`, `LoadedPot` |
| `reusable_resource!` | things moved in and returned by every process (R2); no tripwire | `Kettle`, `Teapot`, `MainsTap`, `GridSocket` |
| `outcome_token!` | the boundary token of a fallible process (R17) | `DrillOutcome` (`model/pilot-workshop/src/resources.rs`) |
| `draw_process!` | the R15 draw over two of your container types | `draw_funds` (`model/pilot-workshop/src/money.rs`) |

Each macro's rustdoc in `model/model-core/src/resource.rs` carries its grammar and a
compiling example. Conventions that matter:

- **One type per processing state** (R9, F-023). CS-1's kettle is four types: `Kettle`
  (empty, reusable) → `FilledKettle<G>` → `BoilingKettle<G, E>` → `Kettle` again. That is
  what turns "poured before boiling" into a compile error instead of a wrong answer. Never
  reuse one type for two states.
- **States carrying more than one quantity**: `container_resource!` appends one magnitude
  parameter `const V: u64` itself (always last); a second dimension rides as a *declared*
  const parameter. `BoilingKettle<const WATER_G: u64>` therefore expands to
  `BoilingKettle<WATER_G, V>`, used as `BoilingKettle<1500, 500_000>` — 1500 g of water, V =
  500 000 J embodied. A hand-written inherent impl exposes the extra constant
  (`BoilingKettle::WATER_G`).
- **Tripwire or not:** consumables and containers get a tripwire `Drop` that panics
  `"resource leak: …"` at test time if the value is dropped without being consumed — the only
  layer that catches a named-but-abandoned resource (F-032). Items *kept* by a container that
  accounts for them (CS-1's `DryTeabag`, kept by the box and the loaded pot) are declared
  `no_tripwire`: a tripwired item inside an abandoned container would panic from the
  container's own drop and mask the real leak site (F-040). Reusable resources have no
  tripwire — they legitimately outlive the flow.
- **Held contents are real objects** (R12): `LoadedPot { bags: (DryTeabag, DryTeabag,
  DryTeabag) }` owns its three bags as sealed payload. `mint` takes them by value (a
  conserving combinator), and they leave only when the whole resource is defused at a
  consumer. Held contents must be untripwired (F-040).
- **Constants over comments:** masses and counts that the asserts need live as associated
  consts on the types (`DryTeabag::MASS_G`, `SpentTeabag::MASS_G`, `TeabagBox::COUNT`), so
  tests recover the spec's numbers from the model rather than restating them.

### 3.3 The boundary

Everything in SPEC §4 becomes a `boundary` child module of the resource module
(`model/cs1-pot-of-tea/src/resources.rs`, `pub mod boundary`) — it has to be a child module,
because field privacy stops at module boundaries and this is the only production code allowed
to mint (F-006). Four shapes:

- **Discrete suppliers hold real objects in a type-level list.** CS-1's
  `TeabagBox<Items>` implements `model_core::boundary::Supplier` *only* for a non-empty list
  (`TeabagBox<Cons<H, T>>`); `TeabagBox<Nil>` (aliased `EmptyTeabagBox`) has no impl, so
  supplying from an empty box is a trait-bound error with a plain-English
  `on_unimplemented` message. The count **is** the list's length (`Items::LEN`), so count and
  contents cannot disagree. A sealed fill function (`full_teabag_box::<N40>()`) is the one
  place teabags come into existence; its recursive machinery is sealed with the classic
  sealed-trait pattern so outside code cannot implement it (F-026).
- **Continuous sources are draw processes, not suppliers** (F-028 — the `Supplier` trait is
  discrete-only; a finite continuous container cannot implement a supplier-shaped trait on
  stable). CS-1's mains tap and grid socket are reusable boundary objects with draw
  functions: `draw_cold_water::<1500>(tap) -> (ColdWater<1500>, MainsTap)`.
- **Sinks are consumers.** Finite, contents-keeping: the `FoodWasteBin<Space, Contents>` —
  `Consumer<SpentTeabag>` is implemented only while `Space` is `Succ<…>`, consuming
  decrements space and keeps the real bag at the front of the contents list (F-016). The
  decreasing space parameter is a **hard rule**, not style — without it trait resolution
  diverges and, at the mandated recursion limit, crashes rustc with no diagnostic (F-034).
  Unbounded (`type Next = Self`): `KitchenAir`, `Drinker`, `CouncilCollection` — legal only
  at the boundary, always placeholders, and they necessarily discard their intake (F-029).
- **Placeholders are tagged.** Every assumed boundary object carries a
  `/// Placeholder: <what needs refining>` doc line (R12) — greppable, so the model's
  unbounded assumptions are a mechanical list, and refining one later (a real vendor, a
  finite heat store) is the normal path.

Reach suppliers and consumers **through the generic access processes**
(`model_core::boundary::{take_one, take_n, send_to, send_list}`) or your own generic
processes — never by calling `.supply()`/`.consume()` on a concrete value. The direct call
takes the E0599 path, which bypasses the modeller-phrased diagnostics entirely (F-015).

Taking N items from one supplier is **one where-clause** with the recursive `SupplyN` trait,
never hand-chained `S::Next: Supplier` bounds (F-014): CS-1's `load_pot` takes
`S: SupplyN<N3, Taken = ThreeDryBags>` — the `Taken =` equality doubles as the item-type
requirement. `ConsumeList` is the mirror for feeding a list to one consumer.

### 3.4 Processes

Processes live in the `processes` child module of the resource family **when they mint
quantity-bearing values** (F-031 — continuous processes must be inside the privacy boundary,
since they call the `pub(crate)` `mint`/`defuse`). Each SPEC §5 block becomes one function;
CS-1's P1–P5 are all in `model/cs1-pot-of-tea/src/resources.rs`. The recurring moves:

**Const-assert balances.** One inline `const { assert!(…, "modeller-phrased message") }` per
dimension (R3, R15, F-033). The custom message leads the error output, so phrase it as the
model rule being broken:

```rust
const {
    assert!(
        EMBODIED_J + HEAT_J == DRAW_J,
        "energy conservation violated in boil (R15): the embodied energy plus the kettle's \
         waste heat must sum exactly to the energy drawn from the grid"
    )
};
```

Outputs cannot be computed on stable (`Qty<{A + B}>` needs nightly, F-022), so the **caller
states every split** and the assert checks it — the modeller maintains the running balance by
hand, and a wrong balance is a compile error (F-030). Balances that are true *by
construction* (CS-1's fill: the filled kettle carries the same `G` as the drawn water; the
load: `SupplyN<N3>` takes exactly three) are "structural" and need no assert — the SPEC marks
each balance line `(assert)` or `(structural)`.

**Time draws are adjacent, not internal** (F-048). A process that needs a person takes and
returns `Person<B>` *unchanged*; the budget draw is its own step in the flow, right next to
it, recorded to the `History` under the process's name:

```rust
let (person, filled) = fill_kettle(person, new_kettle(), water);
let (labour, person) = draw_time::<30_000, 270_000, 300_000>(person);
let history = record(history, "fill_kettle", labour);
```

Why: a process that draws internally must take the concrete `Qualified<Cert, BUDGET>`/
`Person<BUDGET>` type (the budget change is a const-parameter change no generic bound can
express) and its wrong-operator errors degrade from REQ-phrased E0277s to raw E0308s
(F-048). Keep draws adjacent and both problems vanish. Overspending is the same
compile error as overdrawing a gas bottle (R15).

**Waste routing — say which of the two shapes you're using.** Either the flow routes a loose
waste output to its sink (CS-1's P3 kettle heat: `boil` returns `WasteHeat<50_000>`, and the
flow calls `vent_heat(air, kettle_heat)` — `vent_heat` carries the REQ-009 bound), or the
process takes its consumers as requirement-bounded parameters and feeds them internally so
the waste never exists loose (CS-1's P4: `pour_and_brew` takes the bin and the air, and the
spent bags and steeping heat never escape it). The second is the strong reading — the
requirement becomes structural — at the cost of a wider signature.

**The F-054 permit pattern** (how `pour_and_brew` stays fully generic). A
requirement-bounded ("style A") process would normally be unable to *consume* a resource
whose magnitudes it must assert over. The pattern that fixes it, in brief:

1. the magnitudes go on the **sealed characteristic trait** as associated consts
   (`Boiling::WATER_G`, `Boiling::EMBODIED_J` — usable inside `const { assert!(…) }`);
2. the trait carries a **conserving extraction** (`pour_away`, `steep`) gated by a **permit**
   — a zero-size sealed token (`BrewPermit`) with no public constructor, so only the one
   process inside the privacy boundary can call the extraction;
3. the characteristic trait is **sealed** (a private supertrait), so outside code can
   neither implement it to smuggle fake "boiling water" past the bound nor call the
   extraction to vanish resources.

Result: all four of `pour_and_brew`'s inputs are requirement-trait bounds with REQ-phrased
errors, *and* the asserts still state compile-time conservation. See
`model/cs1-pot-of-tea/src/characteristics.rs` (the traits and seal) and `BrewPermit` in
`src/resources.rs`. Style B — taking the concrete type — is only needed when the *returned*
type's const parameters must vary with the input's.

**Fallible processes** (R17; pilot, not CS-1). A process that can fail returns
`Result<OkBundle, FailBundle>` where both bundles conserve the same inputs; variability
enters only as a sealed **outcome token** (`outcome_token!`) injected at the boundary, so the
process stays deterministic and both arms are testable. Bundles are `#[must_use]` groupings
with public resource fields and **no `Debug`** — which makes `.unwrap()`/`.expect()` a
compile error and forces the `match` (F-047). Failure states are their own types
(`ScrapPlate`, `BrokenDrillBit`), each with a production-legal exit (repair process or scrap
consumer). Rework is bounded by provisioning: a retry consumes provisioned reserves, and
unused reserves are re-accounted at the boundary (F-050). Worked examples:
`drill_holes_fallible` in `model/pilot-workshop/src/resources.rs`, both-arms handling and
the one-retry composition in `model/pilot-workshop/src/flows.rs`, per-arm tests in
`model/pilot-workshop/tests/fallible_flows.rs`.

**Qualifications** (R18; pilot). A qualified person is
`model_core::common::Qualified<Q, BUDGET_MS>` — a sealed wrapper that conserves the `Person`
inside (boundary `qualify`/`release`). Qualification markers are downstream traits attached
by one-line blanket impls over the budget, **never implemented on `Person` directly**
(F-043). `qualified_draw_time` delegates to `draw_time`, so the overdraw error and the
Labour→History accounting are inherited.

**Money** (R19; pilot). No new machinery: each currency is its own dimension,
`Money<const PENCE: u64>` is a container resource, `Account` is drawn down by a
`draw_process!`, vendors implement `Consumer` only at their exact price (so a wrong payment
is a type-check-time error naming the right price, F-051), and change-giving is an R3 split.
See `model/pilot-workshop/src/money.rs`.

### 3.5 Flows, History, and disposal

**Flows are integration tests** (`model/cs1-pot-of-tea/tests/flows.rs`). An integration test
is its own crate, *outside* your privacy boundary — it cannot mint or defuse anything, so it
lives under exactly the discipline a downstream user does: every input from a boundary
supplier, every output genuinely reaching a consumer. Compose at least the two valid orders
the SPEC names, and end each flow with the "everything accounted" block — every product at
its consumer, every reusable back, the empty containers accounted, the budget balance
checked.

**History** (R16) is the sink for expended time: `new_history()` at the boundary,
`record(history, "process_name", labour)` after each draw, and the value doubles as the
execution record (`event_count()`, `entries()`). One `History` per concurrent branch, merged
at joins with `merge` — a single global History threaded everywhere would serialize the whole
model (R16). CS-1 has one actor, so one History travels with the person and the merge
demonstration is deferred; the merge machinery itself is worked in
`model/model-core/src/history.rs` (module doc-test).

**Disposal** (F-039): a contents-keeping consumer's kept items are tripwired, so the bin
cannot simply exit the model — every kept tripwire would fire. The bin is emptied through a
sealed recursive disposal process (`empty_bin`, CS-1's P5) that defuses the contents *inside*
the privacy boundary, conserves their summed mass into one `FoodWaste<G>` handed to the
council collection, and returns the bin with its capacity restored at the type level
(`Space + Contents::Length` via type-level `Add`). The recursion is sealed (F-026) so outside
code cannot defuse spent teabags.

### 3.6 Tests

- **Per-process unit tests** (R5) live in `#[cfg(test)] mod tests` inside the resource
  module — inside the boundary, so they may mint fixtures and defuse outputs directly. One
  test per process; for a fallible process, one per *arm* plus one per retry path (branch
  coverage is leak coverage, F-002).
- **Type-check-time compile-fail cases** go in `tests/ui/*.rs` under trybuild (CS-1:
  pouring a non-boiling kettle, brewing with an unloaded pot, a spent bag handed straight to
  the council, constructing a sealed resource outside the boundary).
- **Conservation/overdraw violations** cannot be tested with trybuild — it runs
  `cargo check` internally and never sees them (F-003). They are rustdoc **`compile_fail`
  doc-tests** on the process itself (see `boil` and `pour_and_brew` in
  `model/cs1-pot-of-tea/src/resources.rs`). Keep the snippets minimal: stable rustdoc
  ignores the expected-error-code, so the test passes if the snippet fails for *any* reason.
- **Tripwire demos** are `#[should_panic(expected = "resource leak: …")]` tests — one from
  inside the boundary and one downstream via `test_fixture()`.

---

## 4. Reading compiler errors

The compiler speaks borrow-checker; the model speaks conservation. This table is the
translation, with a worked example for each entry. (This is the error-reading guide that
R4/R9 require the library documentation to include; a condensed copy lives on
`model-core`'s crate front page, `model/model-core/src/lib.rs`.)

### 4.1 "use of moved value: `person`" (E0382)

The resource is already in use by another process, or was lost upstream. Get it back from
that process's output: every process returns its reusables, so the fix is to rebind —
`let (person, filled) = fill_kettle(person, …)` — and pass the *returned* person onward.
Pinned example: `model/pilot-workshop/tests/ui/person_used_twice.rs` (the same person moved
into two processes at once — the error means "this person is already busy"). **Ignore
rustc's "consider borrowing" / "consider cloning" fix-its**: resources are never borrowed or
cloned (R2); following them is a conservation violation (F-007).

### 4.2 An `on_unimplemented` sentence (E0277) — capacity, requirements, qualifications

Trait-bound failures are where this project's own messages appear. Three families:

- **Exhausted supplier / full consumer:**

  ```text
  error[E0277]: `BoltBox<Nil>` cannot supply anything: it is exhausted, or it is
                not a supplier of discrete items
  ```

  (`model/model-core/tests/ui/empty_supplier.stderr`.) The box is empty — refill or enlarge
  it at the boundary. The mirror case, a full consumer, is
  `model/model-core/tests/ui/full_consumer.rs`. `SupplyN`'s version reads "cannot supply
  this many items: it would run out partway".

- **An unmet requirement**, phrased as the requirement and naming its id:

  ```text
  error[E0277]: this kettle may not be poured into the pot: `FilledKettle<1500>`
                is not at the boil (REQ-006)
  ```

  (`model/cs1-pot-of-tea/tests/ui/pour_non_boiling_kettle.stderr`.) The `help:` line names
  the type that *does* satisfy it (`BoilingKettle<G, E>`), i.e. the missing process step.
  The unloaded-pot case (`brew_with_unloaded_pot.stderr`) reads the same way for REQ-007.

- **Qualifications and safety** (pilot): "this person is not certified …" /
  the fitted-guard message — `model/pilot-workshop/tests/ui/unqualified_person.rs`,
  `unfitted_guard.rs`.

These messages appear **only on the trait-bound path**. If you call `.supply()` directly on a
concrete empty box you get a bare E0599 "no method named…" with unrelated suggestions —
which is why model code goes through generic processes (`take_one`, `send_to`, …; F-015).

### 4.3 "expected `X`, found `Y`" (E0308) — the wrong state, the wrong thing

A plain type mismatch is the model saying a processing step is missing or a resource is
routed to the wrong place:

```text
error[E0308]: mismatched types
   = note: expected struct `FoodWaste<_>`
              found struct `SpentTeabag`
```

(`model/cs1-pot-of-tea/tests/ui/spent_bag_straight_to_council.stderr` — the council takes
only the bin's bagged `FoodWaste`, so a loose spent teabag's only route out is through the
bin: REQ-008 is structural.) Likewise "expected `DrilledPlate`, found `Plate`" reads as
"these plates have not been drilled yet" (F-023). Note that a wrong *catalogue* item usually
surfaces as 4.2's E0277 instead, because catalogue bolts are reached through requirement
bounds: the pilot's wrong-bolt case
(`model/pilot-workshop/tests/ui/wrong_bolt.stderr`) reads "this bolt may not be used for
fastening: `Bolt<SizeM6, Brass, L15>` is not an M8 steel 15 mm bolt (REQ-001)", with the
approved `Bolt<SizeM8, Steel, L15>` named in the notes. You only get the bare
expected/found E0308 form where a signature takes the concrete type.

### 4.4 E0080 with one of our assert messages — a conservation violation

```text
error[E0080]: evaluation panicked: energy conservation violated in boil (R15): the
              embodied energy plus the kettle's waste heat must sum exactly to the
              energy drawn from the grid
  ...
  |_______^ evaluation of `boil::<1500, 550000, 500000, 60000>::{constant#0}` failed here
```

(rustc 1.98.1; this is CS-1's `boil` given 60 000 J of heat instead of 50 000.) The custom
message leads, and the instantiation line carries the decimal magnitudes — the wrong numbers
are right there. When the violating call crosses a crate boundary (your model calling
model-core's `split` or `draw_time`), the primary span lands in `core/src/panic.rs` instead:
read the **"while instantiating `fn draw_time::<30000, 0, 20000>`"** note, which points at
your call site. Three crucial facts (F-001):

- It fires at **monomorphization, during codegen**: `cargo check`, rust-analyzer and
  editor diagnostics report the violating code as *fine*. Only `cargo build`/`cargo test`
  show it. Your editor saying "no errors" means nothing for balances.
- Generic code that is never instantiated is never checked — which is why every process
  needs a test that instantiates it (R5).
- An E0080 naming a **draw or budget assert** is a continuous overdraw (R15): the draw
  exceeds what the container or budget has left. Same reading: the numbers are in the
  instantiation note, e.g. `fn draw_time::<30_000, 0, 20_000>`.

Two echo patterns: a **composed process** (pilot's `purchase` → `split_money`) produces a
second E0080 whose note points *inside the library* — read the first one, whose note carries
your call site (F-051). A **delegating wrapper** (`qualified_draw_time`) puts the note one
hop inside the wrapper — the decimal magnitudes still identify the offending call (F-043).

### 4.5 E0275 "overflow evaluating the requirement"

The type-level capacity exceeds the crate's `recursion_limit`: the box is too big for the
current limit. Peano needs ≈ N + 3; the mandated `#![recursion_limit = "2048"]` covers
capacities to ~2000, and the attribute is **per crate** — a new crate (including each
integration-test file) that forgets it gets this error at capacity ~130 (F-010). Keep
capacities ≤ ~500 where convenient; compile time goes superlinear past that (F-011).

### 4.6 E0451 "field `_seal` … is private" / E0423

You tried to create a resource from nothing:

```text
error[E0451]: field `_seal` of struct `Person` is private
```

(`model/model-core/tests/ui/construct_outside_boundary.rs`; CS-1's version is
`model/cs1-pot-of-tea/tests/ui/construct_outside_boundary.rs`.) Resources enter only through
boundary suppliers, draws and fill functions (R1, R12) — or, in tests, `test_fixture()`
under the `test-support` feature.

### 4.7 "`DrillFail<…>` doesn't implement `Debug`" — on `.unwrap()` / `.expect()`

You may not panic past the failure arm of a fallible process. Outcome bundles implement no
`Debug` **deliberately** (F-047), so `unwrap`/`expect` do not compile
(`model/pilot-workshop/tests/ui/unwrap_needs_debug.rs`): `match` the `Result` and account
for both bundles' resources (R17). Do not derive `Debug` to "fix" it, and do not follow the
`.expect("REASON")` fix-it.

### 4.8 E0107 "function takes 4 generic arguments but 3 were supplied"

A mixed const/type turbofish: trailing generic arguments cannot be omitted, so write `_` for
the inferred type parameter — `qualified_draw_time::<2000, 3000, 5000, _>(op)`, or CS-1's
`pour_and_brew::<_, 1473, 480_000, 20_000, _, _, _, _>(…)` (F-043). Const parameters
otherwise infer well — from binding annotations, through chains, and via partial turbofish.

### 4.9 What errors look like, generally (F-009)

- **Aliases are erased**: type-level numbers print as raw `Succ<Succ<…>>` nests with no
  decimal value (`N37` never appears in an error). Wrap numbers in domain types whose outer
  name survives (`TeabagBox<…>` does; a bare `N37` doesn't).
- **Long types spill to side files**: from capacity ~130, rustc writes the full type to
  `target/…/long-type-*.txt` and the error cites the file.
- **The same bound error may appear twice** at one call site (as in
  `empty_supplier.stderr`) — it is one mistake, not two.

### 4.10 Fix-its that are always wrong here

Ignore any rustc/clippy suggestion that says `let _ = …`, `drop(…)`, "consider borrowing",
"consider cloning", or "consider using `Result::expect`". Each one is a conservation
violation dressed as a fix (F-007): each mechanism's official suggestion is precisely the
next leak path. The real fix is always to pass the resource on, return it, hand it to a
Consumer — or, for a `Result`, match it and account for both bundles (R17).

### 4.11 And one failure that isn't an error at all

A tripwire panic at *test* time — `resource leak: SpentTeabag dropped without being
consumed (R1 conservation)` — means a resource was named and used but never reached a
consumer. No compile-time layer can catch that shape (F-032). Note the panic reports the
`Drop` impl's line, **not** the leak site: keep processes small and tests per-process (R5)
so the failing test localises the leak.

---

## 5. Rules you must not break

Each of these looks optional until you know the finding behind it. Breaking one either
silently unsounds the model or crashes the compiler.

1. **The sealing rules, all of them (F-005).** Every resource is a struct with at least one
   private field — never a `pub` unit struct, never a `pub enum` (every variant is a public
   constructor); no `Default`/`Clone`/`Copy`/deserialize; `#[must_use]` on every resource;
   `#![forbid(unsafe_code)]` in every modelling crate (`unsafe { mem::zeroed() }` mints a
   sealed resource). The pattern is one attribute away from broken and **the compiler will
   not warn** — use the kernel macros, which generate it correctly, and hand-seal only where
   they can't (see `Qualified` in `model/model-core/src/common.rs`, F-040).
2. **The lint set is complete or it is nothing (F-007).** `deny(unused_must_use)` +
   `deny(let_underscore_drop)` + clippy `-D warnings` with `clippy::mem_forget`,
   `clippy::let_underscore_must_use` and the `clippy.toml` ban list. Each mechanism's own
   fix-it suggests the next leak path, so a subset gives *false* safety. Never write
   `let _ = <resource>`. And do not add `clippy::shadow_*` — it fires on the
   conservation-correct rebinding idiom.
3. **Outcome bundles never implement `Debug` (F-047).** The missing impl is what makes
   `.unwrap()`/`.expect()` on a fallible process a compile error. Deriving `Debug` to
   silence the error reopens the panic-past-the-failure-arm hole.
4. **Every contents-keeping consumer has a decreasing space parameter (F-034).** An impl
   that applies to every state and only grows
   (`impl<C> Consumer<T> for Bin<C> { type Next = Bin<Cons<T, C>>; }`) diverges trait
   resolution: at the default recursion limit that is a graceful E0275, but at the mandated
   `recursion_limit = "2048"` it is a **deterministic rustc SIGBUS — a crash with no
   diagnostic**. The decreasing parameter (or an honest `Next = Self` unbounded sink) is
   what keeps resolution terminating. The same finding forbids type-level history recording
   (R16).
5. **REQ ids are workspace-unique (F-053).** Number from the next free id across the whole
   workspace (check the latest trace.sh report), not from 001. A reused id keeps CI green
   while the traceability report silently merges your requirement into another model's.
6. **`Satisfies:` tags never go inside a macro invocation (F-037)**, and every tag is paired
   with its `satisfies!` assertion (F-020). Requirement bounds stay on the `fn`-name line
   (F-021). All three protect the same thing: trace.sh's grep is the traceability report,
   and these are the shapes it silently misses.
7. **`test-support` only ever under `[dev-dependencies]` (F-004)** — and remember that
   `cargo test` alone cannot prove the boundary (feature unification); only ci.sh's plain
   build does. Never gate on `cargo check` alone (F-001).
8. **Whoever mints a tripwired type ships a production-legal consumer for it (F-035).** A
   conserved output whose only consumer is test-gated makes every production use a dead end
   — this is why `History` exists as `Labour`'s sink, and why CS-1's spent teabags have the
   bin and the bin has `empty_bin`.
9. **One type per state (F-023); markers never on `Person` (F-043); `Supplier` is
   discrete-only (F-028); unbounded `Next = Self` objects only at the boundary, always
   placeholders (F-029).**

---

## 6. Known limitations

Stated plainly, because finding them was the point of the project (R14). Full entries in
`FINDINGS.md`.

- **Conservation errors are invisible to editors (F-001, F-003).** The `const` asserts fire
  at monomorphization. `cargo check`, rust-analyzer and trybuild all pass violating code;
  generic code never instantiated is never checked. Mitigation: ci.sh gates on full builds,
  every process has an instantiating test, violations are pinned as rustdoc `compile_fail`
  doc-tests. But while you type, your editor is lying to you about balances.
- **"Nothing is silently lost" is not fully compile-time enforceable (F-002).** Rust's types
  are affine: end-of-scope drops of used bindings, `..` field drops and early-return drops
  have no compile-time detection. The layered regime (must_use → tripwire → clippy) converts
  most leak paths into compile or test failures, but the residue is real: a leak on a branch
  no test runs is caught by nothing — branch coverage *is* leak coverage — and **panic
  unwinding evades everything**: a panicking model run loses whatever was in flight,
  silently (the tripwires deliberately stand down during unwinding to avoid double-panic
  aborts). A panicking run has already failed; its accounting is void.
- **Recursion and compile-time ceilings (F-009, F-010, F-011).** Type-level capacity costs
  `recursion_limit` ≈ N + 3 per crate (hence the mandated 2048) and compile time goes
  superlinear past N ≈ 500; magnitudes (2000 g) are unusable as unary types — that is what
  the const-generic quantities are for. Long types spill to side files from ~130; aliases
  are erased in every diagnostic.
- **The F-034 crash.** The one known shape that takes rustc down with no diagnostic
  (SIGBUS at the mandated recursion limit): a contents-keeping consumer without a
  decreasing space parameter. If the compiler dies with no error at all, look for this
  shape first.
- **Outputs cannot be computed; balances are maintained by hand (F-022, F-030).** Every
  split's remainder and every combine's total is caller-stated; a budget parameter infects
  every signature its resource passes through, and the modeller restates the running
  balance at every call. The compiler checks each step, but the arithmetic burden is yours.
- **Traceability is grep with a discipline (F-021, F-037, F-038, F-053).** trace.sh has no
  scope awareness: one-line signatures for bounded processes, one-line
  `on_unimplemented` attributes, tags outside macros, workspace-global ids. The discipline
  is mechanical but real, and two of its failure modes (F-037, F-053) are silently green.
- **Exact integers only — no tolerances.** "1500 g ± 10 g" is not expressible; every
  quantity is one exact value (instructions.md, open question 4). Likewise nothing yet
  checks the model against *reality* (does boiling really take 550 000 J?) — validation vs
  verification is open question 1, and every outcome-token constructor stays a
  `Placeholder:` until calibrated.
- **Time and rates stay outside the model (R9, R15).** Only amounts are conserved; there is
  no scheduling, no duration, no power. The History records what was consumed, attributed
  to processes, as a partial order — not a timeline.
