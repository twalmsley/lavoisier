# EXP-07 Results: Flows, exclusive resources, reusable resources

Tests **R2** (strict conservation: resources never shared, reusable resources moved in and returned) and **R9** (the model describes connections, not sequences).

- Toolchain: `rustc 1.98.1 (48a229cea 2026-09-01) (Homebrew)`, stable.
- Dependencies: `trybuild 1.0.121` (dev-dependency only, as allowed).
- Clean build time (`cargo clean && time cargo build`, three runs): 1.06 s / 1.01 s / 1.01 s — **median 1.01 s**. A model of this size is instant; nothing here stresses the compiler.
- `cargo test`: 4 unit tests + 4 trybuild compile-fail cases, all green.

## What was built

`src/lib.rs`:
- `resources`: `SteelSheet` (mass in grams), `Person`, `Drill`, `Plate`, `DrilledPlate`, `Bolt` (a handful is a plain `[Bolt; 4]` — deliberately not EXP-02's type-level-list supplier), `Assembly`, `Offcut`. All fields private (R1), no `Clone`/`Copy`, all `#[must_use]` under `#![deny(unused_must_use)]`. Conserving conversions (`cut_into_plates`, `into_drilled`, `assemble`) are `pub(crate)` methods on the resources themselves; the `boundary` child module holds placeholder suppliers/consumers (R12) and is the only creation point.
- `processes`: `cut(sheet) -> (Plate, Plate, Offcut)`, `drill_holes(person, drill, plate) -> (Person, Drill, DrilledPlate)`, `fasten(person, a, b, [Bolt; 4]) -> (Person, Assembly)`.
- `workshop`: `Workshop { person, drill }` plus aggregated wrappers `workshop::drill_holes(ws, plate) -> (Workshop, DrilledPlate)` and `workshop::fasten(ws, a, b, bolts) -> (Workshop, Assembly)`.

Tests: `flow_order_a_loose`, `flow_order_b_loose` (two different valid orders, R9), `flow_aggregated`, `processes_conserve_mass`. Compile-fail cases in `tests/compile_fail/`: `person_used_twice.rs`, `product_never_produced.rs`, `process_swallows_drill.rs`, `conjure_drilled_plate.rs`.

## 1. Verdicts per evaluation criterion

### C1. Two different valid orders of the same flow both type-check (R9) — **pass**
`tests::flow_order_a_loose` (cut → drill p1 → drill p2 → fasten, scrap offcut last) and `tests::flow_order_b_loose` (cut → scrap offcut immediately → drill p2 → drill p1 → fasten with plates in swapped positions) both compile and pass. Nothing in the process signatures fixes an order; only the data dependencies do (you cannot drill before cutting, because no `Plate` exists yet — which is exactly the R9 intent). One honest caveat: in a single linear dependency chain the "different orders" available are only the reorderings of independent steps; the type system gives sequencing freedom exactly where the data flow permits it, no more.

### C2. Same `Person` moved into two processes at once is a compile error (R2) — **pass**
`person_used_twice.rs`: two drills, one person, two `drill_holes` calls. Error E0382 (verbatim in §2) points at exactly the second process call that claims the busy person, names the resource, and explains why it cannot be shared. Because `Person` implements neither `Copy` nor `Clone`, the compiler offers no "consider cloning" escape hatch — the no-`Clone` rule directly improves the error.

### C3. A flow using a product that is never produced is a compile error (R9) — **pass**
`product_never_produced.rs`: the flow skips `drill_holes` and feeds raw `Plate`s to `fasten`. Error E0308 points at the exact arguments and says `expected `DrilledPlate`, found `Plate``. This depends entirely on the modelling discipline of giving each processing state its own type (`Plate` vs `DrilledPlate`); with a single `Plate` type the flow would compile. Backstop: `conjure_drilled_plate.rs` shows the missing product cannot be faked either — constructing `DrilledPlate` outside the resources module is E0451 "field `grams` … is private" (R1).

### C4. A process that swallows the drill — **pass with complications**
`process_swallows_drill.rs` defines `bad_drill_holes(person, drill, plate) -> (Person, DrilledPlate)`.
- **Complication (the known affine gap):** the swallowing process itself compiles cleanly. `#[must_use]` does not help — it fires on ignored return values, not on a moved-in parameter that is silently dropped inside the callee. Nothing at the definition site says "this process loses a drill".
- The error appears only at the **caller**, at the next step of the flow that needs the drill: E0382 "use of moved value: `drill`", pointing at the correct downstream call, with a note tracing the move into `bad_drill_holes` and even naming the swallowing parameter. So the caller's error is good — it identifies the guilty process — but only if the flow *does* try to reuse the drill. A flow that never needs the drill again never notices the loss (that residual gap is EXP-03's subject).
- **Misleading fix-it:** the compiler's help says "consider changing this parameter type … to borrow instead if owning the value isn't necessary" — precisely what R2 forbids. Modellers must be told to ignore borrow/clone suggestions.

### C5. Boilerplate per process call, loose vs aggregated — **pass with complications**
Loose threading (per call, with 2 reusable resources):
```rust
let (person, drill, d1) = drill_holes(person, drill, p1);
```
Each reusable resource is named **twice per call** (once in, once out), i.e. 2×N extra identifiers per call for N reusable resources, and the pattern relies on shadowing (`let (person, …) = …(person, …)`) to stay readable. Shadowing keeps names stable but is itself an EXP-03 leak path (shadowing a *different* still-live resource drops it silently).

Aggregated:
```rust
let (ws, d1) = workshop::drill_holes(ws, p1);
```
Constant 2 identifiers per call regardless of how many reusable resources the workshop holds; call sites scale much better. The costs:
1. **Over-claiming.** `fasten` needs no drill, but `workshop::fasten` claims the whole `Workshop`, so the drill is locked while fastening runs — the aggregation *removes* concurrency that R9 wants to allow. This is a semantic distortion of the model, not just style.
2. **Coarser errors.** A double-booking error becomes "use of moved value: `ws`", which no longer says *which* resource is contended.
3. The destructure/rebuild boilerplate does not vanish; it moves into the wrapper (`Workshop { person, drill }` rebuilt inside every aggregated process).

Verdict: loose threading is the semantically correct default; aggregation is an ergonomic optimisation that is only safe when every process in the flow genuinely needs the whole bundle.

## 2. Representative compiler errors, verbatim

### Person already busy (E0382) — `person_used_twice.rs`
```
error[E0382]: use of moved value: `person`
  --> tests/compile_fail/person_used_twice.rs:19:50
   |
10 |     let person = boundary::supply_person("Alice");
   |         ------ move occurs because `person` has type `Person`, which does not implement the `Copy` trait
...
18 |     let (_person_a, _drill_1, _d1) = drill_holes(person, drill_1, p1);
   |                                                  ------ value moved here
19 |     let (_person_b, _drill_2, _d2) = drill_holes(person, drill_2, p2);
   |                                                  ^^^^^^ value used here after move
```
*Judgement:* very good — it marks "value moved here" on the first process and "used here after move" on the second, which reads naturally as "this person is already busy in `drill_holes`; get them back from its output first". The only translation a modeller needs is the vocabulary: "moved" = "handed to another process and not yet returned".

### Product never produced (E0308) — `product_never_produced.rs`
```
error[E0308]: arguments to this function are incorrect
  --> tests/compile_fail/product_never_produced.rs:16:32
   |
16 |     let (_person, _assembly) = fasten(person, p1, p2, bolts);
   |                                ^^^^^^         --  -- expected `DrilledPlate`, found `Plate`
   |                                               |
   |                                               expected `DrilledPlate`, found `Plate`
   |
note: function defined here
  --> src/lib.rs
   |
   |     pub fn fasten(
   |            ^^^^^^
```
*Judgement:* excellent — with state-per-type naming, "expected `DrilledPlate`, found `Plate`" reads directly as "these plates have not been drilled yet". The best error of the set.

### Process swallows the drill (E0382 at the caller) — `process_swallows_drill.rs`
```
error[E0382]: use of moved value: `drill`
  --> tests/compile_fail/process_swallows_drill.rs:27:78
   |
21 |     let drill = boundary::supply_drill();
   |         ----- move occurs because `drill` has type `Drill`, which does not implement the `Copy` trait
...
25 |     let (person, _d1) = bad_drill_holes(person, drill, p1);
   |                                                 ----- value moved here
26 |     // The drill is gone: the next process in the flow cannot have it.
27 |     let (_person, _drill, _d2) = exp07_flows::processes::drill_holes(person, drill, p2);
   |                                                                              ^^^^^ value used here after move
   |
note: consider changing this parameter type in function `bad_drill_holes` to borrow instead if owning the value isn't necessary
  --> tests/compile_fail/process_swallows_drill.rs:12:43
   |
12 | fn bad_drill_holes(person: Person, drill: Drill, plate: Plate) -> (Person, DrilledPlate) {
   |    --------------- in this function       ^^^^^ this parameter takes ownership of the value
```
*Judgement:* good location (it fingers `bad_drill_holes` as where the drill disappeared), but the "consider … borrow instead" suggestion is model-hostile: following it would violate R2. The real fix — "return the drill from `bad_drill_holes`" — is never suggested.

### Conjuring a never-produced product (E0451) — `conjure_drilled_plate.rs`
```
error[E0451]: field `grams` of struct `DrilledPlate` is private
  --> tests/compile_fail/conjure_drilled_plate.rs:13:29
   |
13 |     let d1 = DrilledPlate { grams: 800 };
   |                             ^^^^^ private field
```
*Judgement:* adequate — "private field" says "you may not create this", though not *why*; a doc comment on each resource type ("obtained only from process X or supplier Y") would supply the missing why.

## 3. Candidate FINDINGS.md entries (ready to paste)

- **Move semantics deliver R2's "one resource, one process" with precise errors, but in borrow-checker vocabulary.** Double-booking a reusable resource is E0382 "use of moved value", pointing at both the process that holds the resource and the one trying to claim it. A modeller must learn one translation: "moved" = "already in use by another process". No workaround needed; consider a short error-reading guide in the project docs. Keeping resources `Clone`-free also keeps the compiler from suggesting `.clone()` as a fix. (EXP-07, R2)

- **"Product never produced" is only caught because every processing state is its own type.** `fasten(person, p1, p2, bolts)` with undrilled plates fails with `expected `DrilledPlate`, found `Plate`` — the most modeller-readable error in the experiment. Cost: one struct per state (`Plate`/`DrilledPlate`) and a conserving `pub(crate)` conversion for each process step. Convention to adopt: never reuse one type for two processing states. (EXP-07, R9/R1)

- **A process that swallows a reusable resource compiles without complaint; the error surfaces only at the caller's next use.** Rust's affine types allow `fn bad(p: Person, d: Drill, …) -> (Person, DrilledPlate)` to drop the drill silently; `#[must_use]` does not fire on moved-in parameters. The caller's E0382 does trace the loss to the guilty function, but only when the flow tries to reuse the drill — a flow that never needs it again never finds out. Worse, rustc's fix-it advises borrowing ("consider changing this parameter type … to borrow"), which is exactly what R2 forbids. Mitigations: signature-review convention (every reusable resource that goes in must appear in the return type), plus EXP-03's drop-tripwire mechanisms. (EXP-07, R1/R2 gap)

- **Loose threading costs 2 identifiers per reusable resource per call and leans on shadowing; aggregation (`Workshop`) is constant-size but over-claims resources.** `let (person, drill, d1) = drill_holes(person, drill, p1)` vs `let (ws, d1) = workshop::drill_holes(ws, p1)`. The aggregate reads better and scales with the resource count, but a `Workshop`-taking `fasten` locks the drill it doesn't need, deleting concurrency the model should allow (R9), and contention errors degrade to "use of moved value: `ws`" without naming the contended resource. Recommendation: aggregate only sets of resources that are genuinely always used together; otherwise thread loose values. Also note the shadowing idiom that makes loose threading bearable is itself a silent-drop leak path (EXP-03). (EXP-07, R2/R9)

## 4. Recommendation

**Adopt** — move-in/move-out threading of reusable resources works on stable Rust exactly as R2 hopes, with compile errors that land on the right line, and R9's "connections, not sequences" falls out of data dependencies with no extra machinery. Adopt with three riders:
1. One type per processing state (that is what makes "never produced" a readable type error).
2. Loose threading as the default; a `Workshop`-style aggregate only for resource sets used together by *every* process that takes it — otherwise it over-claims and serialises the flow.
3. Document the two vocabulary traps for modellers: "use of moved value" means "resource already busy / lost upstream", and rustc's borrow/clone fix-it suggestions must be ignored, since following them breaks R2.
