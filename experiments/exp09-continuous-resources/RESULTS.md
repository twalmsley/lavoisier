# EXP-09 RESULTS: Continuous resources, time budgets, and boundary sinks (R15)

**Toolchain:** `rustc 1.98.1 (48a229cea 2026-09-01) (Homebrew)`, `cargo 1.98.1`, stable channel,
macOS (Darwin 24.6.0). Dependencies: `trybuild` (dev) only.

**Compile time:** `cargo clean && time cargo build` three times: 1.06 s, 1.06 s, 1.07 s —
**median 1.06 s**. Const-generic containers carry none of the Peano depth cost (F-010/F-011);
compile time is a non-issue for continuous resources.

**Test status:** `cargo test` passes: 9 integration tests (including two `#[should_panic]`
tripwire demonstrations), 1 runnable doc-test, 7 rustdoc `compile_fail` doc-tests, and the
trybuild suite (4 ui cases, `.stderr`-pinned). `cargo clippy --all-targets` is clean.

**Layout:**
- `src/lib.rs` — the whole model: sealed tripwired quantity types (macro-generated),
  `GasBottle<REMAINING>`, `Person<BUDGET_MS>`, `Atmosphere`/`Depot`/`Ledger`/`WorkSink`
  boundary objects, `boundary` + `processes` child modules, boundary traits
  (`Supplier`, `SupplierOf<Out>`, `Consumer<In>`, `SupplyN`, `ConsumeList`) with
  `on_unimplemented`, and the 7 `compile_fail` doc-tests.
- `tests/integration.rs` — flows in two orders, budget threading, the three supplier probes,
  `ConsumeList` with `Next = Self`, tripwire `should_panic` cases.
- `tests/ui/` — trybuild type-check-time cases (wrong consumer, transposed fuel/air, and the
  two proofs that finite containers cannot implement supplier-shaped traits).
- `postmono-demo/` — a detached crate (own `Cargo.toml`, `[workspace]`) with six bins, one per
  compile-fail snippet, used to capture each error verbatim and to demonstrate the
  `cargo check` (blind) vs `cargo build` (fails) contrast. Not part of the main `cargo test`.

---

## 1. Verdicts per evaluation criterion

### C1. Does the container pattern deliver compile-checked finite capacity, and where does the overdraw error fire? — **pass** (with the known F-001 caveat)

Yes. `GasBottle<const REMAINING: u64>` + `draw_gas::<TAKE, LEFT, FULL>` with
`const { assert!(TAKE + LEFT == FULL) }` makes overdraw (6000 g from a 5000 g bottle) a
genuine `E0080` compile error: no `LEFT` exists satisfying the assert, exactly as R15 claims.
`GasBottle<0>` (`type EmptyGasBottle`) is a distinct resource type; drawing from it fails the
same way, and abandoning it is caught by the conservation conventions (§3, step 6).

Where it fires — measured directly on `postmono-demo/src/bin/overdraw_gas.rs`:

- `cargo check`: **passes** ("Finished `dev` profile") — the violation is invisible.
- `cargo build`: fails with E0080 (verbatim in §2). The primary span is in the *library's*
  assert (with the custom message leading), plus a note
  `while instantiating `fn draw_gas::<6000, 0, 5000>`` pointing at the caller's exact line.
- rustdoc `compile_fail` doc-tests: all 7 pass under `cargo test` (rustdoc fully builds
  doc-tests), confirming the R4 policy. trybuild was used only for type-check-time cases.

F-001 applies to R15 unchanged: conservation overdraw is post-monomorphization, so editor
diagnostics and `cargo check` will not show it; CI must gate on `cargo build`/`cargo test`.

One pleasant surprise vs the discrete world: every diagnostic prints the magnitudes as
**decimal numbers** (`GasBottle<0>`, `draw_gas::<6000, 0, 5000>`, `expected `Gas<300>`,
found `Air<300>``). The F-009 alias-erasure problem (uncountable `Succ` nests) does not exist
for const-generic continuous resources.

### C2. Does `Next = Self` coexist with the boundary traits, `on_unimplemented`, and repeated-use helpers? — **pass**

All of it works, with no coherence or recursion problems:

- **Multiple `Consumer<In>` impls on one boundary object:** `Atmosphere` implements
  `Consumer<ExhaustGas<M>>` and `Consumer<WasteHeat<J>>` (each generic over the magnitude,
  each `Next = Atmosphere`). Both are used in every integration flow.
- **`ConsumeList` with `Next = Self`:** `send_list(atm, Cons(exhaust, Cons(heat, Nil)))`
  consumes a heterogeneous list of two different waste types into one atmosphere
  (`tests/integration.rs::atmosphere_consumes_exhaust_and_heat_as_a_list`). Recursion
  terminates because the *list* shrinks, not the consumer's state — `Next = Self` is
  irrelevant to the induction.
- **`Supplier` + `SupplyN` with `Next = Self`:** `take_n::<Succ<Succ<Succ<Zero>>>, _>(atm)`
  yields three fixed 100 g air packets and the unchanged atmosphere. Same reason: `SupplyN`
  recurses on `N`.
- **`on_unimplemented`:** asking the atmosphere to consume `Labour<500>` gives
  `` `Atmosphere` cannot consume `Labour<500>`: it is full, or it does not accept this kind of
  resource `` at type-check time, and rustc even lists the two impls that *do* exist — close
  to a model-level "the atmosphere takes exhaust and heat, not labour" (§2, error D).
- A minor honest note: an unbounded consumer's `consume` must dispose of the item
  (`defuse`), i.e. the atmosphere does **not** keep what it consumes, deviating from R12's
  "a consumer keeps the objects it has consumed". That is exactly the resource-minting/
  swallowing licence R15 already confines to the boundary; no new rule needed.

### C3. Is `Supplier` discrete-only in practice? — **yes; R15/R12 need the amendment** (probe verdict: pass — the question is answered decisively)

Three styles were built for the unbounded air source:

(a) **Fixed-packet supply** (`impl Supplier for Atmosphere { type Item = Air<100>; … }`).
Works, including through `SupplyN` — but it is the wrong tool: the unparameterized trait
allows exactly **one** packet size per boundary object; getting 300 g costs three supplies
plus a chain of `combine_air` calls (each restating totals per F-022); and amounts that are
not a multiple of the packet (250 g) are unreachable without a further splitting process.
The discrete trait forces continuous material through an item-shaped hole.

(b) **Draw-style boundary process** — `draw_air<const TAKE: u64>(atm: Atmosphere) ->
(Air<TAKE>, Atmosphere)`. One line, caller states the amount, mirrors `draw_gas` exactly, so
finite and unbounded continuous sources read identically at call sites. Fits R15 best.

(c) **Parameterized `SupplierOf<Out>`** (`impl<const G: u64> SupplierOf<Air<G>> for
Atmosphere`). Legal and usable (`let (air, atm): (Air<300>, _) = source(atm);`) — but it is
just (b) in trait clothing, and it only works *because* the source is unbounded.

The decisive fact, proven by two trybuild cases: **a finite continuous container cannot
implement any supplier-shaped trait on stable.** The next state depends on the amount drawn,
and a trait impl has nowhere to receive the caller-stated remainder:

- `impl<TAKE, LEFT, FULL> DrawSupplier<Gas<TAKE>> for GasBottle<FULL> { type Next =
  GasBottle<LEFT>; … }` → **E0207** "the const parameter `LEFT` is not constrained".
- Computing it instead, `type Next = GasBottle<{ FULL - TAKE }>` → "generic parameters may
  not be used in const operations" (needs `generic_const_exprs`, nightly).

So the trait boundary-object machinery genuinely cannot host the R15 draw pattern for finite
resources; only `Next = Self` sources escape, trivially. **Recommendation: document
`Supplier` (and any parameterized variant) as discrete-only. Continuous boundary sources are
draw-style boundary processes, not `Supplier` impls** — amendment text in §5.

### C4. Threading ergonomics of budgeted resources; error quality; anything else to change in R15 — **pass with complications**

- **The budget parameter does infect every signature the person passes through.** Even a
  process that spends nothing must be written
  `fn walk_to_station<const BUDGET: u64>(p: Person<BUDGET>) -> Person<BUDGET>`; one extra
  const parameter per budgeted resource per signature, forever. Call sites stay clean
  (the const infers from the argument).
- **The modeller keeps a running balance by hand.** Each spending call restates the remainder:
  `draw_time::<2000, 8000, 10000>`, then `draw_time::<3000, 5000, 8000>` — three numbers per
  call, where the previous `LEFT` must be retyped as the next `BUDGET` (the type system
  enforces the retyping: the argument type pins `BUDGET`) and the new `LEFT` must be computed
  by the modeller. A wrong balance is a compile error, so this is *checked* arithmetic — the
  cost is typing and re-deriving numbers, not a correctness risk. It is real friction: the
  integration flow spells 300/4700/5000-style triples eleven times.
- **Error quality:** the budget overdraw error is the same clean E0080 with the custom message
  ("is more time being spent than the person has left?") and the caller-site instantiation
  note. All failure-mode errors are in §2, each judged readable.
- **Both flow orders type-check and run** (R9 holds for the continuous flow), and the
  two-dimension `burn` composes cleanly: two independent `const` asserts; when both are
  violated, **both** errors are reported (verified), each with its own message.
- Two things R15 should also say — the mint-inside-the-boundary layout consequence and the
  tripwire/transform interplay — are in §3 (steps 1 and 6) and the amendments (§5).

---

## 2. Representative compiler errors (verbatim) with readability judgements

**(A) Gas overdraw — E0080 at monomorphization** (`cargo build` on `postmono-demo`;
identical shape for the time-budget overdraw):

```
error[E0080]: evaluation panicked: conservation violated in draw_gas: TAKE + LEFT must equal FULL — is the draw larger than the bottle's remaining contents?
   --> /usr/local/Cellar/rust/1.98.1/lib/rustlib/src/rust/library/core/src/panic.rs:62:9
    |
 62 |           $crate::panicking::panic_fmt($crate::const_format_args!($($t)+));
    |           ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ evaluation of `exp09_continuous_resources::model::processes::draw_gas::<6000, 0, 5000>::{constant#0}` failed here
    …
note: the above error was encountered while instantiating `fn draw_gas::<6000, 0, 5000>`
 --> src/bin/overdraw_gas.rs:8:23
  |
8 |     let (gas, rest) = draw_gas::<6000, 0, 5000>(bottle);
  |                       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

*Judgement:* good. The custom message leads and asks the right model question; the decimal
const arguments (`6000, 0, 5000`) show the violation at a glance; the final note lands on the
modeller's own line. The cross-crate primary span in `core/src/panic.rs` (exactly as F-001
warned) is noise the error-reading guide already covers.

**(B) Energy assert in `burn`** — same shape, message
`energy conservation violated in burn: the fuel's energy must equal waste heat + work`, with
`burn::<300, 300, 600, 11000, 9000>` in the instantiation note. When mass *and* energy are
both wrong, both E0080s appear. *Judgement:* good; the per-dimension messages identify which
balance failed without reading any code.

**(C) `cargo check` on every conservation violation:** `Finished `dev` profile …` —
**no error at all** (all four post-mono bins). The lint-layer cases *are* caught at check
time (E below). *Judgement:* this asymmetry is the single most important thing to teach
modellers (F-001); R15 inherits it unchanged.

**(D) Wrong consumer — type-check-time E0277 with `on_unimplemented`** (trybuild-pinned):

```
error[E0277]: `Atmosphere` cannot consume `Labour<500>`: it is full, or it does not accept this kind of resource
  --> tests/ui/atmosphere_cannot_consume_labour.rs:11:24
   |
11 |     let _atm = send_to(atm, labour);
   |                ------- ^^^ this boundary object does not take this resource
   = help: the trait `Consumer<Labour<500>>` is not implemented for `Atmosphere`
help: `Atmosphere` implements trait `Consumer<In>`
   |     impl<const M: u64> Consumer<ExhaustGas<M>> for Atmosphere {
   |     impl<const J: u64> Consumer<WasteHeat<J>> for Atmosphere {
```

*Judgement:* excellent — the message is model-language, and rustc's help listing the two
accepted waste types effectively documents the atmosphere's interface. (Emitted twice at the
one call site, consistent with F-009.)

**(E) Conservation-convention lint layers** (check-time, from `postmono-demo`):

```
error: unused `Gas` in tuple element 0 that must be used
  = note: Gas is a conserved resource: pass it on or hand it to a Consumer
error: unused `GasBottle` in tuple element 1 that must be used
  = note: GasBottle is a conserved resource: even an empty bottle must be passed on or handed to a Consumer
```
```
error: unused variable: `heat`
   |     let (exhaust, heat, work) = burn::<300, 300, 600, 11000, 4000>(gas, air);
   |                   ^^^^ help: if this is intentional, prefix it with an underscore: `_heat`
```

*Judgement:* the `must_use` one is very good — the lint sees through the returned tuple and
prints the per-type custom note. The `unused_variables` one is serviceable but its fix-it
(`_heat`) is, per F-007, another leak path the guide must tell modellers to ignore.

**(F) Fuel/air transposed — type-check-time E0308:**

```
error[E0308]: arguments to this function are incorrect
   |     let _out = burn::<300, 300, 600, 11000, 4000>(air, gas);
   |                ^^^^^ ---  --- expected `Air<300>`, found `Gas<300>`
   |                      expected `Gas<300>`, found `Air<300>`
help: swap these arguments
```

*Judgement:* excellent. Distinct sealed container types make the role mix-up crisp; the
suggested fix is, for once, the conservation-correct one.

**(G) Why finite containers can't be trait suppliers** (trybuild-pinned):

```
error[E0207]: the const parameter `LEFT` is not constrained by the impl trait, self type, or predicates
```
```
error: generic parameters may not be used in const operations
   |     type Next = GasBottle<{ FULL - TAKE }>;
   |                             ^^^^ cannot perform const operation using `FULL`
```

*Judgement:* E0207 is compiler-jargon a modeller would not decode — but these errors face the
*library author* deciding a design, not a modeller using the library; as design evidence they
are exactly on point.

---

## 3. Notes per build step

1. **`GasBottle` + `draw`** — as specified; works. Layout consequence: `draw_gas`, `burn`,
   `combine_air` must live **inside** the resource module tree (child `processes` module),
   because continuous processes *construct* new quantity values (`Gas<TAKE>` out of a
   bottle's contents), unlike discrete processes, which only move existing objects. F-006's
   "processes live outside the resource module tree" needs that refinement for R15 processes.
2. **Overdraw regressions** — 7 rustdoc `compile_fail` doc-tests (gas overdraw, draw from
   empty, time overdraw, burn mass, burn energy, plus the two convention cases); all pass
   under `cargo test`; each verified to fail for the intended reason via `postmono-demo`
   (the R4 "fails for *any* reason" caveat is real, so this manual verification matters).
3. **Time budget** — works; ergonomics in C4. `Person<0>` is the spent state, analogous to
   `GasBottle<0>`. A bare `const BUDGET_MS: u64` (unit documented, exposed as a constant) was
   used rather than R15's literal `Qty<V, Milliseconds>` sketch — see amendment 4.
4. **Two-dimension `burn`** — composes cleanly; independent asserts, independent messages,
   both report when both are violated.
5. **`Atmosphere`** — all probes pass; `Supplier` is discrete-only (C3).
6. **Integration flow** — two orders type-check and run. Silent-loss cases, by layer:
   - whole draw result discarded → **`#[must_use]`/`deny(unused_must_use)`, compile time**
     (sees through tuples, prints the per-type message);
   - waste heat bound to a name but never used → **`unused_variables`, only under CI's
     `-D warnings`** (warn-by-default otherwise);
   - **empty bottle dropped silently** (named, its predecessor used — the case the brief
     asks about) → **no compile-time layer fires; the tripwire `Drop` catches it at test
     time**: `resource leak: GasBottle<0> dropped without being consumed (R1 conservation)`
     (`should_panic` test). Same for waste heat that is used once then dropped. This is
     F-002's used-then-dropped gap, reproduced exactly for continuous resources.
   - Tripwire/transform interplay: a `Drop` type cannot be destructured, so **every
     conserving transform of a tripwired container needs an internal defuse
     (`mem::forget`) site**, not just the `Consumer` — F-008's "single allowed forget site
     per resource" becomes "one `defuse` helper per resource, called by every conserving
     transform and consumer inside the boundary". Macro-generated here; no hole opened
     (defuse is `pub(in crate::model)`).

---

## 4. Candidate FINDINGS.md entries (to be numbered F-027… at consolidation)

**(1) R15's continuous-resource container pattern works end to end on stable; overdraw is a
post-monomorphization E0080 with unusually good numbers in the message.**
What couldn't be expressed: nothing new — the combination (sealed const-generic container +
caller-stated-remainder draw + budget draw-down + two-dimension balancing + unbounded
`Next = Self` sinks) composes with no new blockers. Overdraw inherits F-001 unchanged
(`cargo check`/rust-analyzer/trybuild blind; rustdoc `compile_fail` doc-tests are the
regression vehicle; CI gates on build/test). Cost: none beyond F-001; clean build 1.06 s.
Unlike Peano capacities (F-009), every diagnostic prints magnitudes as decimals
(`draw_gas::<6000, 0, 5000>`, `GasBottle<0>`) with the caller's line in the instantiation
note. Evidence: EXP-09 (`src/lib.rs` doc-tests; `postmono-demo/` check-vs-build transcript in
RESULTS.md §2).

**(2) `Supplier` is discrete-only: no supplier-shaped trait can be implemented by a finite
continuous container on stable; continuous boundary sources must be draw-style processes.**
What couldn't be expressed: `impl SupplierOf<Gas<TAKE>> for GasBottle<FULL>` — the next
state needs the caller-stated remainder, but a trait impl has nowhere to receive it
(E0207 unconstrained const parameter) and computing it (`GasBottle<{FULL - TAKE}>`) needs
`generic_const_exprs` (nightly). Fixed-packet supply through the discrete `Supplier` works
for an *unbounded* source but forces one packet size per boundary object, `combine` chains
(restated totals, F-022) and unreachable non-multiple amounts. Workaround adopted: continuous
boundary sources are draw-style boundary processes (`draw_air<const TAKE>(atm) ->
(Air<TAKE>, Atmosphere)`); `Supplier`/`Consumer` trait impls on continuous resources are
reserved for unbounded (`Next = Self`) boundary objects, where they remain useful as the
generic *sink* interface (`Consumer<In>` has no remainder problem — the consumed amount is
in `In`). Evidence: EXP-09 (`tests/ui/finite_container_*.rs` + pinned `.stderr`;
`tests/integration.rs` probes (a)–(c)).

**(3) `Next = Self` boundary objects coexist with all R12 machinery.** Multiple
`Consumer<In>` impls on one object (exhaust + heat on `Atmosphere`), `on_unimplemented`
messages, and the recursive repeated-use traits all work: `SupplyN`/`ConsumeList` recurse on
the count/list, so a self-renewing `Next` causes no non-termination or coherence issue.
rustc's E0277 help even lists the impls that do exist ("`Atmosphere` implements
`Consumer<ExhaustGas<M>>`, `Consumer<WasteHeat<J>>`"), documenting the boundary object's
interface in the error. Note: an unbounded consumer necessarily *discards* what it consumes
(the licence R15 confines to the boundary), deviating from R12's "a consumer keeps the
objects it has consumed". Evidence: EXP-09 (`tests/integration.rs`,
`tests/ui/atmosphere_cannot_consume_labour.stderr`).

**(4) A budgeted reusable resource's budget parameter infects every signature it passes
through, and the modeller maintains the running balance by hand — checked, but manual.**
Every process threading `Person<BUDGET_MS>` is generic over the budget even if it spends
nothing; each spending call restates three numbers (`draw_time::<3000, 5000, 8000>`), the
previous remainder re-derived by the modeller as the next budget. A wrong balance is the
E0080 of finding (1), so the arithmetic is compiler-checked; the cost is verbosity and
re-derivation, not correctness. Workaround: none available on stable (computing remainders
needs `generic_const_exprs`); R15's "model a time budget only where genuinely accounted for"
is the right mitigation and is reaffirmed. Evidence: EXP-09 (`walk_to_station`,
`tests/integration.rs::time_budget_draws_down_across_a_flow`).

**(5) Continuous-resource processes must live inside the privacy boundary.** Unlike discrete
processes (which only move sealed objects and can live outside the resource module tree,
F-006), R15 processes (`draw`, `combine`, `burn`) mint new quantity-bearing values
(`Gas<TAKE>` from a bottle), so they must sit in a child module of the resource-defining
module (or be handed sealed mint combinators). The F-006 layout rule needs this carve-out:
each resource family's module contains sealed types + `boundary` + `test_support` + its
conserving *continuous* processes. Evidence: EXP-09 (`src/lib.rs::model::processes` and the
module-level comment).

**(6) The tripwire `Drop` on quantity containers works and is the only layer that catches
the abandoned empty bottle, but Drop types can't be destructured, so every conserving
transform needs an internal forget site.** The layered regime lands exactly as in F-002/F-008
for continuous resources: whole-result discard → `must_use` (compile time, sees through
tuples, custom per-type note); named-but-never-used waste → `unused_variables` (error only
under CI `-D warnings`); used-then-dropped (the silently abandoned `GasBottle<0>`, or heat
passed through one inspection) → tripwire panic at test time only
(`resource leak: GasBottle<0> dropped without being consumed`). Because a type with `Drop`
cannot be moved out of by destructuring, F-008's "single allowed `mem::forget` site" per
resource becomes a sealed `defuse(self)` helper that every conserving transform and consumer
inside the boundary calls — macro-generate it with the type. Evidence: EXP-09
(`tests/integration.rs` `should_panic` cases; `postmono-demo/src/bin/{result_discarded,heat_unused}.rs`).

**(7) Two conservation dimensions per process compose cleanly.** `burn` carries independent
mass and energy `const` asserts; each violation reports its own custom message, and when both
are violated both E0080s are emitted. Nothing in the pattern limits the number of balanced
dimensions per process. Evidence: EXP-09 (`src/lib.rs::burn`, RESULTS.md §2 (B)).

---

## 5. Recommendation: **ADAPT** — adopt R15 with the following amendments

The core of R15 is validated end to end: quantity containers, caller-stated-remainder draws,
compile-checked finite capacity (post-mono), time budgets, two-dimension balancing, waste as
conserved outputs, and unbounded `Next = Self` boundary objects all work together on stable
with modeller-grade error messages. R15 needs these precise amendments:

1. **Continuous boundary sources are draw processes, not `Supplier` impls.** Add to the
   "Unbounded boundary sources and sinks" bullet: *"An unbounded source of continuous
   material is a draw-style boundary process (`draw_air<const TAKE>(atm) -> (Air<TAKE>,
   Atmosphere)`), not a `Supplier` impl. The `Supplier` trait (R12) is discrete-only: a
   finite continuous container cannot implement any supplier-shaped trait on stable (the next
   state would need the caller-stated remainder, which a trait impl cannot receive — E0207 —
   or const arithmetic in a type, which is nightly-only), and fixed-packet supply forces
   combine chains and unreachable non-multiple amounts. `Consumer<In>` impls remain legal and
   useful on unbounded sinks: the consumed amount travels in `In`, so there is no remainder
   problem."* Add the same one-line cross-reference to R12.
2. **Layout carve-out (R1/F-006):** continuous-resource processes (`draw`, `combine`,
   `burn`-style balancers) mint new quantity values and therefore live *inside* the resource
   family's module (a `processes` child module beside `boundary`), unlike discrete processes.
3. **Tripwire mechanics for containers (R1 conventions):** on tripwired quantity types, the
   per-resource `defuse(self)` (internal `mem::forget`, sealed visibility) is called by every
   conserving transform and consumer inside the boundary — Drop types cannot be destructured.
   Macro-generate `mint`/`defuse`/tripwire with the type. Record that the abandoned
   `GasBottle<0>`/`Person<0>`-successor case is caught **only** by the tripwire at test time;
   the compile-time layers catch whole-result discard (`must_use`) and, under CI
   `-D warnings` only, named-but-unused waste (`unused_variables`).
4. **Simplify the sketched types:** allow (and prefer) the bare const-magnitude form —
   `ExhaustGas<const M: u64>` with the unit fixed by the sealed container type and exposed as
   an associated constant — over R15's `ExhaustGas<Qty<M, Grams>>` sketch. The extra `Qty`
   layer adds no checking (distinct sealed containers already prevent cross-dimension and
   cross-role mixing — the fuel/air swap is a crisp E0308) and the bare form prints decimal
   magnitudes in every diagnostic. Reserve `Qty<V, U>` for unit-polymorphic quantity code
   (R3/R7 split/combine), where it earns its keep.
5. **Error-reading guide:** add the continuous overdraw translation — an E0080 whose message
   names a draw/budget assert means "the draw exceeds what the container or budget has left";
   the real numbers are in the `while instantiating `fn draw_gas::<6000, 0, 5000>`` note at
   the caller's line; `cargo check` and editors will not show it (existing F-001 entry).
6. *(Optional, small)* Note in R12 that an unbounded consumer discards rather than keeps its
   intake — the one sanctioned exception to "a consumer keeps the objects it has consumed",
   already implied by R15's boundary-licence wording.
