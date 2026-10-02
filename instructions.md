# Using a Statically-Typed Programming Language for Model-Based Systems Engineering

## Status
The requirements below are agreed, and incorporate the amendments supported by the experiment results. The experiment plan at the end **has been run** (all eight experiments, rustc 1.98.1 stable, September 2026): results are in `experiments/exp0N-*/RESULTS.md`, and the consolidated findings log is `FINDINGS.md` at the repository root. The experiment code is exploratory — it is not the start of the real library. The real library **has been built**: the `model/` workspace holds `model-core` (the library) and `pilot-workshop` (the downstream validation model), gated end to end by `model/ci.sh`. R16 is implemented. R17–R19 are agreed from EXP-10..12; implementation is in progress. New models are written as further downstream crates from a natural-language specification following `SPEC_TEMPLATE.md`.

## Summary
This is an **experiment**. The aim is to find out how far a type system can be pushed for model-based systems engineering, and to record the limitations and complications that turn up along the way. When there is a choice, prefer the approach that puts more checking into the compiler, even if the types get complicated. Finding where that breaks down is part of the point.

Model a real-world system in Rust:
- **Types** stand for things: resources, inputs, outputs, requirements, structure, waste products.
- **Functions** stand for suppliers, consumers and processes. They link the types together.
- **Compiler errors** show where parts of the model don't fit, such as a missing or wrong resource.
- **Unit tests** show that each process really does turn its inputs into its outputs.

## Target language
- **Rust, stable toolchain only** for now. Unstable or nightly features such as `generic_const_exprs` are not used. Where stable Rust can't express something, use a stable workaround (e.g. our own type-level numbers) and record the limitation (R14).
- Rust was chosen because its ownership rules (move semantics, no implicit copying) let the compiler check much of the conservation rule below.
- Physical and consumable resources must **not** implement `Clone` or `Copy`, so they can't be duplicated.
- Known gap: Rust lets a value be dropped silently, because its types are affine, not linear. So "nothing is lost" is only partly enforced at compile time. The rest is covered by the layered conservation conventions in R1.

#### Modeller's error-reading guide (documentation requirement)
The real library's documentation must include a short error-reading guide covering these recurring translations (referenced from R4 and R9):
- "use of moved value: `x`" → the resource is already in use by another process, or was lost upstream; get it back from that process's output.
- "the trait `Supplier` is not implemented for `BoltBox<Nil>`" (and other `on_unimplemented` messages) → the supplier is exhausted / the consumer is full.
- E0275 "overflow evaluating the requirement" → the capacity exceeds the crate's `recursion_limit`; raise it per convention (R12).
- E0080 with our assert message → a conservation violation; it appears only on `cargo build`/`cargo test`, never in `cargo check`/editor diagnostics, and across crates its primary span lands in `core/src/panic.rs` — read the "while instantiating `fn split::<…>`" note for the real call site.
- An E0080 naming a draw or budget assert → a continuous **overdraw** (R15): the draw exceeds what the container or budget has left. The real numbers are in the "while instantiating `fn draw_gas::<6000, 0, 5000>`" note at the caller's line; `cargo check` and editors will not show it.
- Aliases are erased in errors: type-level numbers print as `Succ<…>` nests with no decimal value; long types go to `long-type-*.txt` side files; the same bound error may appear twice at one call site.
- "`DrillFail<…>` doesn't implement `Debug`" (any outcome bundle, on `.unwrap()`/`.expect()`) → you may not panic past the failure arm: `match` the `Result` and account for both bundles (R17). The missing `Debug` is deliberate (F-047).
- A second E0080 whose "while instantiating" note points **inside the library** is a composed-process echo (e.g. `purchase` → `split_money`): read the first E0080, whose note carries your call site (F-051). Likewise, a delegating wrapper (`Qualified`'s draw) puts the note one hop inside the wrapper; the decimal magnitudes still identify the offending call (F-043).
- E0107 "function takes 4 generic arguments but 3 were supplied" on a mixed const/type turbofish → trailing generic arguments cannot be omitted; add a `_` for the inferred type parameter: `qualified_draw_time::<2000, 3000, 5000, _>(op)` (F-043).
- **Ignore rustc/clippy fix-it suggestions that say** `let _ = …`, `drop(…)`, "consider borrowing", "consider cloning" or "consider using `Result::expect`": each one is a conservation violation (F-007). The real fix is always to pass the resource on, return it, hand it to a Consumer — or, for a `Result`, match it and account for both bundles (R17).

## Requirements

### R1. Processes conserve resources
- A process function transforms its inputs into its outputs **without side effects**. It must not:
  - create resources from nothing
  - consume an input without turning it into an output (a product or a waste product)
- Every resource the process needs is passed in **by value** as an argument of the correct type, possibly grouped into a struct.
- Everything the process produces is returned, possibly grouped into a struct. That includes:
  - products
  - waste products and by-products
  - reusable resources (people, tools, locations)
- The only exceptions are suppliers and consumers at the system boundary (R12). Suppliers are where resources enter the model, and consumers are where they leave it.
- **Resources have private constructors.** Resource types have private fields and no public constructor, so code outside their defining module (or crate) can't create them. That makes "no creating resources from nothing" a compile-time check, not just a convention. The only code allowed to create resources is:
  - the code that builds suppliers at the system boundary (R12), e.g. filling a `BoltBox` with its bolts
  - clearly marked test helpers (see **Test helpers** below — `#[cfg(test)]` and the `test-support` feature are *not* interchangeable)
- **Layout.** Resources live in `model-lib`; downstream modelling crates depend on it. The hard boundary is the crate edge. Inside `model-lib`, field privacy stops at module boundaries, so the boundary must be a **child module of the resource-defining module**: each resource family is one module containing the sealed types plus nested `pub mod boundary` (suppliers/fill functions — the only production constructors) and `#[cfg(feature = "test-support")] pub mod test_support` (fixtures), re-exported at the crate root. Processes live outside the resource module tree — except continuous-resource processes (`draw`, `combine`, `burn`-style balancers), which mint new quantity-bearing values and therefore live *inside* the resource family's module, in a `processes` child module beside `boundary` (F-031); a composite output gets a `pub(crate)` combinator that only wraps values passed in by value.
- **Sealing rules** (each greppable; enforce with a lint script — the pattern is one attribute away from broken and the compiler will not warn):
  - every resource is a struct with at least one private field (a zero-sized `_seal: ()` on otherwise-empty types); never a `pub` unit struct;
  - never a `pub enum` resource (variant fields cannot be private; every variant is a public constructor) — wrap a private enum in a sealed struct;
  - no `Default`, `Clone`, `Copy` or deserialization derives/impls, and no public fn returning `Self` from plain data, on any resource;
  - `#[must_use]` on every resource (feeds the conservation conventions below);
  - `#![forbid(unsafe_code)]` in every modelling crate (privacy is a safe-Rust guarantee only: `unsafe { mem::zeroed() }` mints a sealed resource);
  - a public fill function's recursive machinery is sealed with the classic sealed-trait pattern so outside code cannot implement it.
- **Test helpers — the two options are not equivalent.** `#[cfg(test)]` helpers serve the defining crate's own unit tests **only**: a dependency is always compiled without `cfg(test)`, so they can never serve downstream fixtures. Downstream tests use the `test-support` cargo feature, enabled exclusively via a `[dev-dependencies]` re-declaration of `model-lib`. Caveats: during `cargo test`, feature unification compiles the downstream crate's *production* sources against the featured library, so a fixture call in production code passes `cargo test` and is only rejected by plain `cargo build` (hence the R4 CI gate rule); trybuild inherits dev-dependency features and cannot prove the feature boundary; and the feature must never appear under `[dependencies]` anywhere in the graph — police with a grep over `Cargo.toml`s in CI.

#### Conservation conventions (the layered regime)
No stable mechanism or combination makes "nothing is silently lost" a compile-time guarantee: end-of-scope drop of a used binding, struct-pattern `..` field drops, and early-return drops have no compile-time detection at all, and panic unwinding evades everything (FINDINGS.md F-002). The project therefore layers:
1. `#[must_use = "<Type> is a conserved resource: pass it on or hand it to a Consumer"]` on every resource type; crate-wide `#![deny(unused_must_use)]` and `#![deny(let_underscore_drop)]`.
2. A **tripwire `Drop`** on every consumable resource: panics with `"resource leak: <Type> …"` unless `thread::panicking()`, defused only by explicit consumption (the R12 `Consumer` plays the `consume(self)` role; the `mem::forget` inside it is the single allowed site, with a commented `#[allow(clippy::mem_forget)]`; macro-generate the boilerplate). It converts six of the eight known leak paths into ordinary test failures; note it reports the `Drop` impl's line, not the leak site — keep processes small and tests per-process (R5). Because a type with `Drop` cannot be destructured, each tripwired resource gets a sealed `defuse(self)` helper (the internal `mem::forget` site) that every conserving transform and consumer inside the boundary calls; macro-generate `mint`/`defuse`/tripwire together with the type. An abandoned end-state resource (e.g. an empty `GasBottle<0>` left to fall out of scope) is caught **only** by this tripwire at test time: `must_use` catches whole-result discard at compile time, and `unused_variables` catches named-but-unused waste only under CI `-D warnings` (F-032).
3. Clippy in CI under `-D warnings`, plus explicitly (restriction lints): `clippy::mem_forget`, `clippy::let_underscore_must_use`; and a `clippy.toml` `disallowed-methods` list banning `std::mem::drop`, `core::mem::drop`, `std::mem::ManuallyDrop::new` and `std::boxed::Box::leak`, each with a `reason` string telling the modeller what to do instead.
4. **The set is only sound as a whole**: each mechanism's official fix-it suggests the next leak path (`unused_must_use` suggests `let _ =`; `let_underscore_drop` suggests `drop(…)`; for swallowed parameters rustc suggests borrowing, which R2 forbids). Adopting a subset gives false safety. Never write `let _ = <resource>`.
5. **Do not enable `clippy::shadow_*`**: `shadow_reuse` fires on the conservation-correct R2 idiom `let bolt = inspect(bolt);`. The tripwire plus the default `unused_variables` cover the shadowing leak instead.
6. **Whoever mints a tripwired resource type must also ship at least one production-legal consumer for it** (F-035). A conserved output whose only consumer is test-support-gated cannot be accounted for by downstream production code at all — the library's `Labour` needed a `TimeLedger` placeholder sink at the boundary for exactly this reason. Shipping the type without its sink makes every production use a dead end.
7. **Residual gaps, accepted:** used-then-dropped values on untested branches (branch coverage is leak coverage — R5 must test every process branch, especially early returns and error paths); losses during panic unwinding (a panicking model run has already failed); and forget-equivalents beyond the banned list, covered by review.

### R2. Strict conservation: resources are never shared
- Resources are never borrowed or shared. Process signatures must not take resources as `&T` or `&mut T`, and must not use `Rc`, `Arc` or similar.
- A reusable resource is **moved in and returned**. The caller gets it back as part of the function's output.
- As a result, one resource can be in use by only one process at a time. The compiler enforces this.

### R3. Splitting and combining amounts are processes
- Changing an amount is modelled as a function, exactly like any other process. The function takes the input quantity types and returns the output quantity types. Examples:
  - splitting 2000 g of steel into a 1500 g part and 500 g of waste
  - combining several amounts into one
- These functions must conserve the amount. The total going in equals the total coming out, including waste. Where possible this is checked at compile time using the types' constant values (e.g. `const { assert!(IN::VALUE == A::VALUE + B::VALUE) }`), and otherwise by unit tests.
- The `const`-assert technique works on stable inside generic functions (inline `const { … }` block, or an `AssertSum` helper struct — identical behaviour). Write a custom assert message; it leads the error output. The check fires at instantiation, not definition — see the R4 caveat.
- **Outputs cannot be computed on stable** (`Grams<{A + B}>` needs `generic_const_exprs`), so the caller of `combine` states the expected total and the assert checks it. Const parameters otherwise infer well (from binding annotations, through chains, via partial turbofish).
- **Primary quantity style: the R7 trait form** (`Qty<const V: u64, U: Unit>` + `trait Quantity`), not one bare struct per unit: one generic `split`/`combine` serves every unit, unit mixing is a crisp type-check-time E0308, and the conservation assert is stated over `Quantity` types. Type-level (Peano) magnitudes would move the check to type-check time and compute totals, but are unusable at realistic magnitudes (2000 g is a 2000-deep `Succ` chain); they stay reserved for R12 capacities.

### R4. Compilation errors expose model mismatches
- If a process is given a missing or wrong resource, the model must fail to compile.
- The project should include compile-fail examples that show this, e.g. using `trybuild` or doc-tests marked `compile_fail`.
- **Caveat — two classes of compile error.** Type errors (missing/wrong resource, unit mismatch, trait-bound and privacy violations) fire at type-check time and are visible everywhere. Conservation asserts (R3's `const { assert!(…) }` and equivalents) fire only at **monomorphization, during codegen**: `cargo check`, rust-analyzer diagnostics driven by it, and trybuild (which runs `cargo check` internally) all report violating code as fine, and generic code that is never instantiated is never checked. Modellers must be told that editor diagnostics will not show conservation errors (see the error-reading guide under Target language).
- **Compile-fail test policy:** use trybuild only for type-check-time errors; write conservation-violation regressions as rustdoc `compile_fail` doc-tests (rustdoc fully builds them). Keep those snippets minimal: stable rustdoc ignores the expected-error-code annotation, so the doc-test passes if the snippet fails for *any* reason.
- **CI gate rule:** CI must run `cargo build` and `cargo test` for every crate, and a plain `cargo build`/`cargo check` of every downstream crate's production targets. `cargo check` alone misses conservation errors; `cargo test` alone misses test-support feature misuse in production code (feature unification, R1). Never gate on `cargo check` alone.

### R5. Unit tests verify transformations
- Each process has unit tests that show it turns specific inputs into the expected outputs, and that nothing is left unaccounted for.
- For a fallible process (R17), "each process" means each **arm**: at minimum one test per `Result` arm, and one per path of any retry composition (a one-retry flow has three paths). Branch coverage is leak coverage (F-002), and R4's per-instantiation asserts only run where a test instantiates the process.

### R6. Characteristics are encoded in the type system
- A type's characteristics are expressed through the traits it implements (marker traits), **or** through type parameters. Both are legal and compose via bridging, but they have fixed roles (below).
- A concrete type is the intersection of the sets its characteristics describe. Marker-trait example:
  ```rust
  trait M8 {}  trait Bolt {}  trait Length15mm {}  trait Steel {}

  struct M8SteelBolt15mm;
  impl M8 for M8SteelBolt15mm {}
  impl Bolt for M8SteelBolt15mm {}
  impl Length15mm for M8SteelBolt15mm {}
  impl Steel for M8SteelBolt15mm {}
  ```
- Type-parameter example: `Bolt<M8, Steel, Length<15, Millimetres>>`.
- A requirement is written as a generic bound, so any type with the right characteristics satisfies it:
  ```rust
  fn fasten<B: M8 + Bolt + Length15mm + Steel>(bolt: B, /* ... */) -> /* ... */
  ```
  In a real model, the bound is wrapped in a named requirement trait (R10).
- A marker trait for a measured characteristic, such as `Length15mm`, can also carry the matching quantity type (R7), e.g. `trait Length15mm: HasLength<Length = Length<15, Millimetres>> {}`. The value is then available for calculations as well as for type checking.
- **Fixed roles, by experiment:**
  - **Catalogues are parameterized types** (`Bolt<Size, Material, Length>`): LOC scales with the sum, not the product, of dimension values (33 vs 121 hand-written lines at 24 combinations); adding a value is 2–3 lines; no macro machinery is needed (stable `macro_rules!` cannot synthesize the cross-product struct names anyway); and wrong-bolt errors are the clearest of any style (`expected Bolt<SizeM8, Steel, L15>, found Bolt<SizeM6, Brass, L15>`).
  - **Every type parameter is bounded by a kind trait** (`Size`, `Material`, `Length`; one one-line impl per value). Unbounded parameters accept transposed arguments silently (`Bolt<Steel, SizeM8, L15>` compiles); kind traits turn transposition into a clear construction-site error.
  - **Requirements are always written as marker/requirement-trait bounds (the R10 pattern), never as concrete `Bolt<…>` types in process signatures.** Bridging is one-way: blanket impls (`impl<M, L> M8 for Bolt<SizeM8, M, L> {}`) let parameterized types satisfy marker bounds, but a marker struct can never satisfy a nominal-type signature. Bounds are the one language both encodings speak; concrete parameterized types are implementation detail.
  - **Bridging impls are part of the catalogue**: one line per characteristic value, added in the same commit as the value itself.
  - **Default-type-parameter trap:** adding a dimension with a default (`Bolt<S, M, L, P = Pitch125>`) keeps existing spellings compiling, but existing blanket impls written over `Bolt<SizeM8, M, L>` silently narrow to the default value only — quiet model corruption with no warning. Whenever a defaulted parameter is added, every blanket impl mentioning the type must be rewritten with an explicit parameter (`impl<M, L, P> M8 for Bolt<SizeM8, M, L, P>`); a grep for `for Bolt<` makes the audit mechanical.
  - **One-struct-per-combination marker types are reserved for small, irregular sets** (a handful of named, non-orthogonal things), never for orthogonal catalogues.

### R7. Quantities and units are types that also carry values
- Every quantity value and every unit is its own type, e.g. `Millimetres`, `Grams`, `Length15mm`. This lets the compiler catch unit and quantity mismatches.
- The same types also hold their numeric values in constants (associated `const`s and/or const generics), so the values can be used in calculations and in tests.
- Values are **integers in the smallest unit** of each dimension. Floating point is not used for now. The base units are:

  | Dimension        | Base unit           | Status  |
  |------------------|---------------------|---------|
  | Mass             | grams               | agreed  |
  | Length           | millimetres         | agreed  |
  | Time             | milliseconds        | agreed  |
  | Area             | square millimetres  | default |
  | Volume           | cubic millimetres   | default |
  | Temperature      | millikelvin         | default |
  | Electric current | milliamperes        | default |
  | Energy           | joules              | default |
  | Money            | minor currency unit | default |

  "Default" units are sensible starting choices for the experiment and can be reworked later if they cause problems.
  - Units made from other units follow from the agreed base units, e.g. area is length × length, so it's in mm². This keeps calculations consistent without conversion factors. A litre is therefore 1,000,000 mm³.
  - Temperature is in kelvin so values are never negative and fit unsigned integers.
  - For any new dimension, pick a sensible default in the same style, add it to this table marked "default", and mention it in the findings log if the choice causes problems.
  - Money is one dimension **per currency** (R19): each currency is its own unit type in integer minor units (pence, euro cents), and currency unit types never mix — cross-currency arithmetic is a type error, like adding grams to millimetres. There is no shared "money" unit and no conversion factor between currencies; conversion is only the R19 boundary exchange process.
- Example:
  ```rust
  struct Millimetres;
  struct Length<const V: u64, U>(PhantomData<U>);
  impl<const V: u64, U> Quantity for Length<V, U> { type Unit = U; const VALUE: u64 = V; }
  // Length<15, Millimetres>::VALUE == 15
  ```

### R8. Our own units library
- The project writes its own units and quantities library. It does not depend on existing crates such as `uom`.

### R9. The model describes connections, not sequences
- Process functions describe only **which inputs and outputs connect**. They don't model time, duration or a fixed order.
- Process functions can then be composed into flows, in any sequence or concurrently, as long as the types match:
  - **Sequence:** one process's outputs are passed as the next process's inputs.
  - **Concurrency:** two processes whose inputs don't overlap can run independently. Strict conservation (R2) makes sure they can't both claim the same resource.
- An impossible flow, such as one that needs a resource already used elsewhere or never produced, is a compilation error.
- **One type per processing state.** "Product never produced" is a compile error only because each state is its own type (`Plate` vs `DrilledPlate` gives "expected `DrilledPlate`, found `Plate`" — read directly as "these plates have not been drilled yet"); with one reused type the broken flow compiles. Never reuse one type for two processing states; each process step gets a conserving `pub(crate)` conversion.
- **Loose threading is the default.** Thread reusable resources as loose values (`let (person, drill, d1) = drill_holes(person, drill, p1);`) even though it names each resource twice per call. Aggregates (`Workshop { person, drill }`) are allowed only for resource sets genuinely used together by *every* process that takes the aggregate: otherwise the aggregate over-claims (a `fasten` that needs no drill locks it anyway), deleting the concurrency this requirement exists to allow, and contention errors degrade to "use of moved value: `ws`" without naming the contended resource.
- **Vocabulary note:** "use of moved value" means "this resource is already in use by another process (or was lost upstream)"; rustc's borrow/clone fix-it suggestions must be ignored — following them violates R2. See the error-reading guide under Target language.

### R10. Requirements traceability
- Requirements exist as named artifacts in the code, and model elements link back to them.
- This is about requirements *of the system being modelled*. It is separate from the R-numbered list in this document, which lists requirements of the modelling approach itself.
- Every requirement has an ID of the form `REQ-NNN`, e.g. `REQ-001`, numbered sequentially with three digits.
- Each requirement is a **named trait**, and its doc comment starts with the requirement ID. Trait aliases aren't available on stable Rust, so a requirement made of several characteristics is a trait with those characteristics as supertraits, plus a blanket impl:
  ```rust
  /// REQ-001: Fastening bolts must be M8 steel, 15 mm long.
  trait Req001FasteningBolt: M8 + Bolt + Length15mm + Steel {}
  impl<T: M8 + Bolt + Length15mm + Steel> Req001FasteningBolt for T {}

  fn fasten<B: Req001FasteningBolt>(bolt: B, /* ... */) -> /* ... */
  ```
- Types and functions that satisfy a requirement do so through its trait — but note the blanket impl means **no source line ever states `impl ReqNNN for ConcreteType`**, so the type→requirement link is invisible to text search. Therefore:
  - Every type or process claimed to satisfy a requirement carries `/// Satisfies: REQ-NNN` (comma-separated for several) as its own line in its doc block.
  - Every `Satisfies:` tag on a **type** is backed by a compile-checked assertion in the same file: `const fn assert_reqNNN<T: ReqNNN>() {}` (once per requirement) plus `const _: () = assert_reqNNN::<TheType>();`. The tag is for grep; the assertion is for truth — a stale tag is a compile error naming the exact missing characteristic. (This assertion is a trait-bound check, so it fires at type-check time, unaffected by the R4 post-monomorphization caveat.)
- **Tests are tagged** with the IDs of the requirements they verify, as a line in the test's doc comment:
  ```rust
  /// Verifies: REQ-001, REQ-004
  #[test]
  fn fasten_joins_two_plates() { /* ... */ }
  ```
- **The tagging convention** (kept by a `trace.sh` report that runs as a CI gate and exits nonzero on violations), so that which requirements are satisfied and verified is mechanically reportable:
  1. A requirement trait's doc comment **first line** is `/// REQ-NNN: <one sentence>.`; IDs are `REQ-` + exactly three digits, sequential, never reused.
  2. Tags are **case-sensitive** (`Verifies:`, `Satisfies:`) and live only in three-slash `///` doc comments directly above the item; anything in `//`, `////`, `//!`, `/* */` or string literals is ignored by design, so IDs may be mentioned freely in prose.
  3. One `Verifies:` line per test, several IDs comma-separated on that line.
  4. Requirement bounds stay on the same line as the `fn` name (grep has no scope awareness; multi-line `where` clauses are attributed to the wrong line).
  5. No direct `impl ReqNNN for Type` — the blanket impl is the only impl.
  6. The report warns on near-miss tags (e.g. lowercase `verifies:`), unknown REQ ids, and requirements with no verifying test; any warning fails CI.
  7. **`Satisfies:` tags never go inside a macro invocation** (F-037): the line after the tag is not an item keyword there, so trace.sh drops the tag **silently** — the dangerous direction, since the gate stays green. Tag a type alias or the process that uses the bound instead; the `satisfies!` compile-checked assertion stays beside the macro invocation, so the truth half of the tag+assert pair is unaffected.
  8. **Every requirement trait carries a one-line `#[diagnostic::on_unimplemented]`** phrased as the requirement and naming its REQ id, passed through `requirement!`'s meta slot. A failing marker's own attribute is ignored when the marker fails as a supertrait obligation of a requirement bound — only the root obligation's message is shown (F-044) — so the requirement trait's message is the one modellers will see; keep marker-level attributes too, for direct marker bounds.

### R11. Library of reusable common types
- A shared library of common resource types, such as `Person`, `Organisation` and `Location`, plus common units and quantities.

### R12. Suppliers and consumers at the system boundary
- Raw materials and other inputs enter the model only through explicit **suppliers**. Waste products and final products leave it only through explicit **consumers**.
- Suppliers and consumers are defined by **project traits**, `Supplier<Out>` and `Consumer<In>`, not by plain closure types. Each trait has an associated type for its next state (e.g. `type Next`), because supplying or consuming changes its type. The traits can carry a name and requirement links (R10). The `Supplier` trait is **discrete-only** (F-028): continuous material is drawn by boundary processes instead (R15).
- At first, suppliers and consumers may be **placeholders**. A placeholder has the correct types, but will be refined later to represent the real supplier or consumer, such as a named organisation or a waste-disposal service. Placeholders are marked with a `/// Placeholder: <what needs refining>` line in the doc comment.
- A process that needs suppliers or consumers takes them as **generic parameters** (e.g. `S: Supplier<Bolt>`), possibly grouped into a struct, and does not call specific ones directly. Callers can then pass in any implementation whose types match, so a placeholder can later be swapped for a real one without changing the process.
- A supplier or consumer is itself a resource. It is passed by value and returned like any other resource (R2).

#### Suppliers and consumers are strict
- A supplier takes nothing but itself, and returns only what it supplies plus its own next state.
- A consumer takes only itself and the item it consumes, and returns only its own next state.
- Its "next state" is itself with reduced capacity. Because capacity is part of the type, this is a different type (see below).
- If supplying or consuming needs anything else, such as payment, an order, a receipt or a fee, it is **not** a supplier or consumer. It is modelled as a separate process (R1), and that process will probably need its own suppliers and consumers.

#### Suppliers and consumers have finite capacity
- A supplier can supply only a limited number of items before it is **exhausted**. Example: a box of 100 bolts.
- A consumer can accept only a limited number of items before it is **full**. Example: a waste bag.
- Once exhausted or full, it can't supply or consume any more.
- **One item per step.** A supplier supplies exactly one item each time it is used, and a consumer consumes exactly one. Several items are handled by using it repeatedly, e.g. through a process that calls it N times. This is meant to mimic how things happen in reality.
- **Repeated use goes through a recursive helper trait.** A process that takes N items from one supplier does not chain `Supplier` bounds by hand (`S: Supplier, S::Next: Supplier, …` — one where-clause per item, with linearly deepening projections; forbidden beyond depth 2). Instead the library provides a recursive `SupplyN<N>` trait: base case implemented for every `S` at `Zero` (takes nothing), recursive case at `Succ<N>` bounded by `S: Supplier, S::Next: SupplyN<N>`. The two impls do not overlap (the `N` differs), so coherence is satisfied. It returns the taken items as a Cons-list of real values plus the depleted supplier, and any N costs one where-clause: `S: SupplyN<N4, Taken = FourBolts>` (the `Taken =` bound doubles as the item-type requirement). A matching `ConsumeList` trait serves consumers. Expect one such recursive trait per "repeat this boundary action N times" pattern.
- **Suppliers hold real objects.** A supplier contains the actual item objects it will supply, e.g. a `BoltBox` contains 100 `Bolt` values. It doesn't create items from a count. Supplying hands over one of the objects it holds. Likewise, a consumer keeps the objects it has consumed. This is the starting approach; if it proves impractical, record that in the findings log (R14) and review it. One sanctioned exception: an unbounded `Next = Self` boundary consumer (R15) necessarily *discards* its intake (F-029).
- **Capacity is checked at compile time.** The remaining capacity is part of the type. For example, a box of 100 bolts, `BoltBox<Bolts<N100>>`, supplies one bolt and becomes `BoltBox<Bolts<N99>>`.
- `Supplier` is not implemented for a supplier with no capacity left, and `Consumer` is not implemented for a consumer with no space left. Asking either one to do more is therefore a compilation error.
- Every boundary trait (`Supplier`, `Consumer`, `SupplyN`, …) carries `#[diagnostic::on_unimplemented]` (stable since Rust 1.78) with a message phrased for modellers, e.g. `` `BoltBox<Nil>` cannot supply anything: it is empty ``. This only works on the trait-bound path: a direct method call on an exhausted value takes the E0599 "no method named …" path, which bypasses the attribute and suggests unrelated methods. Model code must therefore reach suppliers and consumers through generic processes (already required above), never by calling `supply`/`consume` directly on a concrete value.
- Stable Rust can't do arithmetic on const generics (e.g. `N - 1`), so the project will need its own **type-level numbers**, **Peano-style (`Succ<Zero>`), adopted by experiment**. They belong in the project's own library (R8), not in an external crate such as `typenum`. Each type-level number must also expose its value as a constant, as in R7.
  - **Encoding: Peano.** Supplier contents are a `Cons` list whose type is already O(N) deep, so a binary counter saves no depth, compile time or recursion limit; Peano's structure mirrors the list one-to-one (`Succ` ↔ `Cons`), decrement is 2 impls with no canonical-form traps, and less-than is trivial. The binary encoding works at N=1000 but costs ~3× the machinery, needs a `Trim` normalisation pass for decrement, and has no comparison; keep it on the shelf unless standalone numbers ≥ ~1000 with heavy arithmetic become necessary.
  - **Every crate that uses type-level numbers sets `#![recursion_limit = "2048"]`** (the attribute is per crate, not per library; Peano needs ≈ N + 3, and the default 128 allows capacity ~100 but not 130). Document E0275 "overflow evaluating the requirement" as "the box is too big for the current limit" in the modeller's guide.
  - **Capacity guidance:** capacities up to ~100 are free (full monomorphization ~2.6 s); compile time is flat to N≈500 and superlinear after (~4.6 s per use-site cluster at N=1000). Keep capacities ≤ ~500 where convenient. Aliases `N0`…`N1000` must be generated by a small script (stable `macro_rules!` cannot synthesize identifiers); ship the generator with the library and treat its output as source.
- **Items are stored in a type-level list.** A supplier's contents are a nested list built into its type, e.g. `BoltBox<Cons<Bolt, Cons<Bolt, Nil>>>`. Supplying removes the first item and returns a supplier holding the rest. `Supplier` is implemented only for a non-empty list (`Cons<H, T>`), never for `Nil`. The count is the length of the list, so count and contents can't disagree. A recursive trait exposes that length as a constant (R7).
  - Consumers work the other way round: a consumer carries **two** type parameters — remaining space (a type-level number) and contents (a type-level list), e.g. `WasteBag<Space, Contents = Nil>`. Consuming maps `WasteBag<Succ<S>, C> -> WasteBag<S, Cons<In, C>>`: space goes down by one and the consumed item is kept at the front of the contents list. Give the contents parameter a default of `Nil` so construction stays ergonomic (`new_waste_bag::<N100>()`), and name the full state with an alias (`type FullWasteBag<C> = WasteBag<Zero, C>;`). Note the cost: a consumer's full type mentions everything it has ever consumed, which lengthens error messages. (The defaulted `Contents` parameter is subject to the blanket-impl audit rule in R6.) **Hard rule (F-034): every contents-keeping consumer has a decreasing space parameter.** Without one (`impl<C> Consumer<T> for Bin<C> { type Next = Bin<Cons<T, C>>; }`), trait resolution diverges: at the default recursion limit that is a graceful E0275, but at the mandated `recursion_limit = "2048"` it is a deterministic rustc SIGBUS **crash with no diagnostic**. The decreasing parameter (or a `Next = Self` unbounded sink, R15) is what keeps resolution terminating — it is not just a modelling convention.
  - Large lists are written with helpers, e.g. a type alias such as `Bolts<N100>` built from the project's type-level numbers, plus a fill function that creates a full supplier (inside the privacy boundary, R1).
  - Costs, now measured (FINDINGS.md F-009, F-010): long type names appear from capacity ~130, where rustc spills them to `target/…/long-type-….txt` side files; the default recursion limit allows capacity ~100 but not 130 (hence `#![recursion_limit = "2048"]` above); compile time at capacity 100 is ~2.6 s — a non-issue. The array-of-`Option` fallback considered earlier is **not needed**: the type-level-list design works end to end on stable, and count cannot drift from contents because the count *is* the list's recursive length.
- Once exhausted or full, a supplier or consumer **becomes a new resource type**, e.g. `EmptyBoltBox` or `FullWasteBag`. With the type-level list this happens naturally: `BoltBox<Nil>` is already a distinct type, and a readable alias such as `type EmptyBoltBox = BoltBox<Nil>;` can name it. That new resource must also be accounted for: it goes to a consumer or into another process, just like any other output.
- **A contents-keeping consumer needs a sealed recursive disposal path at the boundary** (F-039, an F-008 × F-016 interaction): its kept contents are tripwired, so letting the bin reach its own legitimate end-of-model exit directly would fire every kept tripwire. A boundary disposal process (e.g. `dispose_bin`) recursively defuses the contents inside the privacy boundary before the bin itself exits — the disposal is the sanctioned-`mem::forget` role for everything the bin kept.

### R13. Discrete items are separate objects
- Each countable item, such as a bolt, is its own object with its own type. It is never an amount with a "count" unit.
- A number of items is held as a collection of those objects. Collections have a **fixed size known at compile time** wherever possible, e.g. an array `[B; N]` or a struct of named items. A variable-size collection such as `Vec<B>` is used only where a fixed size is impossible. Each use needs a comment explaining why, and it counts as a limitation to record (R14).
- Continuous material (mass, length, time, energy) is still modelled as quantity types (R7), split or combined by processes (R3), and held at the boundary in quantity containers (R15) — never as Peano-list suppliers.

### R14. Record findings and limitations
- Because this is an experiment, the project keeps a findings log in `FINDINGS.md` at the root of the repository. It records each place where the type system couldn't express something, or could express it only with significant complications, along with any workaround used.
- Examples of things to log: runtime checks used instead of compile-time ones, `Vec` used instead of a fixed-size collection, confusing compiler error messages, long compile times, and gaps in conservation checking (such as values being dropped silently).
- Findings entries carry stable IDs (`F-NNN`) and cite the evidencing experiment(s) and file(s), so instruction amendments and code comments can reference them.

### R15. Continuous resources, time, and boundary sinks
- **The model conserves amounts, not rates.** "Electric power" (watts) and flow rates are rates, and rates stay outside the model with time and ordering (R9). What is modelled and conserved is the **amount**: energy in joules, gas in grams, labour in person-milliseconds. Rates would only enter the model if R9 is ever revisited.
- **Continuous resources are quantity containers, not item suppliers.** The R12 Peano-list design is for discrete items (R13); magnitudes are unusable as unary types (FINDINGS.md F-011 — 5 kg would be a 5000-deep type). A continuous resource is a **sealed container type wrapping a quantity** (R7 const-generic form): `GasBottle<const REMAINING: u64>` in grams, `Battery<const E: u64>` in joules, `WaterTank<…>`, and so on. Drawing from one is an R3-style split process with the caller-stated remainder:
  ```rust
  // draw 300 g from a 5000 g bottle; const { assert!(TAKE + LEFT == FULL) }
  fn draw<const TAKE: u64, const LEFT: u64, const FULL: u64>(
      b: GasBottle<FULL>,
  ) -> (Gas<TAKE>, GasBottle<LEFT>)
  ```
- **Finite capacity comes free from conservation.** Overdrawing is a conservation violation — no `LEFT` exists with `TAKE + LEFT == FULL` when `TAKE > FULL` — so it is a compile error (the R4 post-monomorphization caveat applies). `GasBottle<0>` is the empty state: a distinct resource type that must still be accounted for, exactly like `EmptyBoltBox` (R12).
- **Time budgets.** Wall-clock and scheduling time stay out of the model (R9). Time as a **costed input** — a person's labour, machine-hours — is a quantity budget carried by the reusable resource (e.g. `Person<const BUDGET_MS: u64>`), drawn down like the gas bottle; a resource with no budget left cannot be drawn from, by the same overdraw error. Model a time budget only where that time is genuinely being accounted for, not on every resource by default.
- **Waste is an ordinary conserved output.** Exhaust gas, waste heat, swarf and the like are sealed quantity-bearing outputs returned by every process that produces them (R1 already forces this). **Prefer the bare const-magnitude form** — `ExhaustGas<const M: u64>`, `WasteHeat<const E: u64>`, with the unit fixed by the sealed type and exposed as an associated constant — over a wrapped `ExhaustGas<Qty<M, Grams>>`: the extra `Qty` layer adds no checking (distinct sealed types already prevent cross-dimension and cross-role mixing — a fuel/air swap is a crisp E0308), and the bare form prints decimal magnitudes in every diagnostic (F-027). `Qty<V, U>` keeps its role in unit-polymorphic quantity code (R3/R7 `split`/`combine`), where it earns its keep. Conservation asserts balance each dimension per process: fuel + air in = exhaust out (mass); energy in = useful work + waste heat (energy). Every waste output must eventually reach a consumer.
- **Unbounded boundary sources and sinks.** Some environments are practically unbounded — the atmosphere, mains water, the electricity grid as a first approximation. These are suppliers/consumers whose next state is themselves (`type Next = Self`). Rules:
  - They are legal **only at the system boundary** (R12). `Next = Self` mints resources (supplier) or swallows them without bound (consumer) — exactly what R1 forbids inside the model; that is why it is confined to the boundary.
  - They are **always placeholders** (R12), e.g. `/// Placeholder: atmosphere — assumed unbounded sink for exhaust and heat.` The unbounded assumption thereby becomes a named, greppable artifact of the model, and refining it later — a scrubber, a radiator, a finite heat store — is the normal placeholder-refinement path.
  - One boundary object may be both source and sink: the atmosphere supplies air *and* accepts exhaust, and the mass balance across combustion needs both.
  - An unbounded source of continuous material is a **draw-style boundary process** (`draw_air<const TAKE>(atm: Atmosphere) -> (Air<TAKE>, Atmosphere)`), not a `Supplier` impl. The `Supplier` trait (R12) is **discrete-only**: a finite continuous container cannot implement any supplier-shaped trait on stable — the next state would need the caller-stated remainder, which a trait impl cannot receive (E0207), or const arithmetic in a type, which is nightly-only — and fixed-packet supply forces `combine` chains and makes non-multiple amounts unreachable (FINDINGS.md F-028). `Consumer<In>` impls remain legal and useful on unbounded sinks: the consumed amount travels in `In`, so there is no remainder problem.

### R16. Execution history
*Design agreed and implemented 2026-10-02 (`model-core::history`); the implementation served as the validation — no design deviations were needed (see F-041). `History` superseded the `TimeLedger` placeholder as the production-legal sink for `Labour` (F-035).*
- Consumed time — and any other event worth recording — is accounted to **`History`**, a sealed boundary consumer: "the past" as an explicit sink. A `History` value doubles as the record of the model execution.
- **Value-level records only (the sound design).** `History` defuses each tripwired item it consumes (the consumer's sanctioned role, F-008) and keeps an **untripwired, value-level record** of it — process name, what was consumed, magnitudes — in a runtime list inside the sealed type. This is a sanctioned `Vec` under R13 (record count is flow-dependent); comment and log it per R13/R14.
- **The type-level-recording variant is forbidden.** `History<Cons<Event1, Cons<Event2, …>>>` — keeping events in the type so the final type is the trace — is exactly the F-034 shape: a contents-keeping consumer with no decreasing space parameter. It diverges trait resolution (SIGBUS at the mandated recursion limit) and its ever-growing type infects every downstream signature. Do not build it.
- **Per-branch histories, merged at joins.** One `History` per concurrent branch of a flow, never a single global one threaded everywhere: under strict conservation (R2) a global `History` is one resource and serializes the entire model, deleting the concurrency R9 exists to allow. A `merge` process combines branch histories at join points. The merged record is a **partial order** — deliberately: concurrent branches have no defined interleaving, and separate-then-merged records state that truthfully.
- Records must carry enough information for the history to serve as an execution record usable in generated documents (see the documentation open question below).

### R17. Fallible processes
- A process that can fail returns **`Result<OkBundle, FailBundle>`**: success and failure are *both* ordinary conserving outcomes (R1). **Both branches conserve the same inputs** — a failure output (a scrapped part, a snapped tool) is a product, not a disappearance. Every reusable resource comes back in both arms; inputs spent in both arms (a person's time) are drawn **before** the branch — by default as an adjacent draw process in the flow (R18, F-048) — so the reusable returns at the same magnitude either way.
- **One type per failure state** (extends R9's one-type-per-processing-state rule): a `ScrapPlate` is not a `DrilledPlate`, a `BrokenDrillBit` is not a `DrillBit` — so a failure output continuing the success flow is a compile error (E0308, or E0277 with `on_unimplemented` phrasing on multi-impl sinks). Every failure-state resource ships with at least one production-legal exit: a repair process back to the working state, and/or a boundary scrap consumer (the F-035 rule applies to failure outputs too).
- **Variability enters only at the boundary, as a sealed outcome token** (F-042) — one token type per fallible-process kind, **never** two token types selected by the flow: two types make the outcome part of the flow's static text, with no `Result` and no code ever forced to handle the arm it did not pick. The token wraps a private enum (R1 sealing), is tripwired, is created only by boundary constructors (always `/// Placeholder:` until calibrated — see open question 1, validation vs verification) and test-support fixtures, and is consumed by exactly one process; an untried token is returned to the environment through a boundary exit function. Flows cannot read a token: the only way to learn the outcome is to run the process and handle the `Result`.
- **Per-branch conservation is one const assert per branch**, each with a branch-naming message, and **both are checked at every instantiation** (the R4 post-monomorphization caveat applies): callers state the success split *and* the failure split even though only one branch runs (F-045). Expect a fallible process to carry roughly twice the const parameters of its infallible form (F-022/F-030).
- **Outcome bundles are groupings, not resources** (F-046): one `#[must_use]` struct per arm with public resource fields, destructured by the handling arm. Bundles implement **no `Debug`** — deliberately, since that makes `.unwrap()`/`.expect()` a compile error and forces a `match` (F-047). A grouping may also be a `pub` enum over bundles (one variant per flow path); a grouping cannot mint resources — building one requires already holding them — so the R1 sealing rules bind resources, not groupings. Prefer named `#[must_use]` groupings over bare `Option<resource>` returns: `unused_must_use` sees through tuples but **not** through `Option`, which leaves only the tripwire (F-046).
- **Flows handle both arms.** Enforcement is layered and measured: at compile time, std's `must_use` on `Result` (discarded result), `deny(let_underscore_drop)` (`let _ =`), E0308 (result used as if it were the Ok bundle; failure state in a success-flow position) and the missing `Debug` (unwrap/expect); at test time, the tripwires (a bound-but-unmatched result; a match arm that forgets fields — `..`-skipped fields are a partial move and drop at the end of the destructured binding's scope, F-008). The residual hole is F-002's: a panic while holding a bundle leaks its resources silently under unwinding. Ignore the fix-its on this path — `let _ = …`, `drop(…)` and `.expect("REASON")` are each a conservation violation (F-007); see the error-reading guide.
- **Rework is bounded by provisioning** (F-050). A retry consumes provisioned reserves (another blank, another outcome token, a repair kit), so the rework bound is the resources passed in — unbounded retry is inexpressible, truthfully. Retry paths that spend different amounts have different types, so a retried flow returns a `#[must_use]` outcome enum (one variant per path); on early success the unused reserves are re-accounted at the boundary (returned to stores / the environment). Failure's labour is recorded in the `History` like any other consumption (R16).

### R18. Qualifications and safety as types
- A person's capabilities (certifications, authorisations) and a machine's safety states (a fitted guard, worn PPE) are **characteristics** (R6): marker traits with modeller-phrased `on_unimplemented` messages, required through R10 requirement traits bound on each process that needs them. There are no runtime qualification checks.
- **A qualified person is a sealed wrapper around the common `Person`** (R11): `Qualified<Q: Qualification, const BUDGET_MS: u64>` in `model-core::common` holds the person as a private field (the kernel macros cannot generate held contents on a reusable, F-040). `Qualification` is the kind trait (R6) over qualification value types (`DrillingCert`); qualification markers (`CertifiedDriller`) are downstream traits attached by one-line blanket impls **over the budget** (`impl<const MS: u64> CertifiedDriller for Qualified<DrillingCert, MS> {}`), added in the same commit as the qualification value itself (F-043).
  - Markers are **never implemented on `Person` directly**: `Person` has no qualification slot, so a direct impl certifies every person in the model at once (F-043).
  - Certification enters the model as a **boundary process** (R12): `qualify` wraps, `release` unwraps the same person, budget intact — the person is conserved through both. It is a placeholder (`/// Placeholder: certification body …`) until refined into a real training process producing certificate evidence.
  - The wrapper's budget is the inner person's: one delegating draw process opens the wrapper, calls `draw_time` and re-wraps, so the R15 overdraw error and the `Labour`→`History` accounting (R16, F-035) are inherited, not duplicated. Parallel person types that bypass `Person` are not used: they fork the draw/labour/sink/record machinery (~40 duplicated lines per type, F-043). The delegation costs one diagnostic hop — the overdraw's instantiation note lands on the wrapper's internal `draw_time` call (see the error-reading guide).
- **A safety resource is a reusable resource** (R2) with **one type per safety state** (R9): `MachineGuard` vs `FittedGuard`, converted only by explicit `fit`/`remove` processes. The safe characteristic (`Fitted`) is implemented by the safe state only, so "guard present but not fitted" (E0277) and "no guard at all" (E0061) are both compile errors, and distinguishable (F-049).
- A guarded, qualification-checked process takes the person and the safety resources as **generic parameters bounded by requirement traits** (F-019) and returns them (R2). A requirement sentence spanning several participants decomposes into **one R10 trait per constrained parameter** (e.g. REQ-001 operator + REQ-002 guard), all bound on the same `fn`-name line (R10 rule 4); trace.sh reports the one process under each id (F-049). A composite "station" aggregate is not used: it over-claims (F-024).
- **Each requirement trait carries its own one-line `#[diagnostic::on_unimplemented]`**, phrased as the requirement and naming the REQ id, passed through `requirement!`'s meta slot (R10 rule 8, F-044): a marker's own message is ignored when the marker fails as a supertrait obligation of a requirement bound, so the requirement trait's message is the one modellers will see.
- Qualification-checked processes do **not** draw time budgets themselves (F-048): the draw is its own adjacent process in the flow (R15). A process that must account its own time takes the concrete `Qualified<Cert, BUDGET>` instead — the budget change is a const-parameter change a generic bound cannot express (the F-028/E0207 shape) — restates its requirement as a where-clause bound on the concrete type for traceability, and accepts that its wrong-operator errors are E0308 type mismatches rather than the REQ-phrased messages.

### R19. Money as a conserved dimension
- **Each currency is its own dimension** (F-051). Money follows R7: integer minor units (pence, euro cents), one unit type per currency, currencies never mixing — cross-currency arithmetic is a type error, like adding grams to millimetres. A new currency is one unit type plus one `impl Unit` line (quantity level) or one sealed money resource type (R15 container level) in the modelling crate; no library machinery is needed.
- **Money is held and moved with the R15 machinery unchanged.** Cash in hand is a sealed const-magnitude resource (`Money<const PENCE: u64>`); money at rest is a quantity container (`Account<const BALANCE: u64>`) drawn down by a draw process, so overspending is the standard overdraw compile error (the R4 post-monomorphization caveat applies). A deposit is the conserving combine back into the container; change-giving is an R3 split.
- **Payment is a process** (reaffirming R12 strictness: a supplier or consumer step takes nothing but itself and the item). A vendor is a boundary object that is both `Consumer<money>` and a goods source (`Next = Self`, F-029); a purchase process splits the tendered cash into price and change (`TENDERED == PRICE + CHANGE`, const-asserted), pays the vendor, and takes the goods, with money and goods each conserved in their own dimension.
- **Vendors implement `Consumer` only at their exact price**, stated once as a named const (F-051). Paying the wrong amount or the wrong currency then fails at **type-check time** (visible to editors and trybuild, unlike conservation asserts), and the error names the correct price: a single impl makes inference report `expected Money<350>, found Money<300>`, and a priced generic bound makes E0277's help list the implemented price. Cost: one impl per (vendor, price point).
- **Currency exchange is value-equivalence at a stated integer rate, not single-dimension conservation** (F-052). An exchange process destroys an amount in one currency dimension and mints the equivalent in another — what R1 forbids inside the model — so it is legal **only as a boundary process**, always a placeholder (R12), threading its boundary object by value. The rate is stated as integer consts `NUM`/`DEN` and every exchange is const-asserted `OUT * DEN == IN * NUM`. The assert is exact multiplication, so nothing ever rounds silently: an amount with no exact exchange at the rate has no `OUT` that compiles — split off an exchangeable sub-amount (R3) and the remainder stays conserved in the original currency. A model that wants a rounding loss or spread must model it as an explicit fee output.

## Experiment plan

The requirements above make claims that stable Rust may or may not support well. Each experiment below tests one claim in isolation. They are written so that independent sub-agents can run and evaluate them **in parallel**: no experiment depends on another's code or results, and each one works entirely inside its own directory.

### Common protocol (applies to every experiment)
- **Read `instructions.md` in full before starting.** The R-numbers referenced below are its requirements.
- **Location:** each experiment is its own cargo crate (or workspace) in `experiments/expNN-<slug>/`, created with `cargo new --lib`. Work only inside your own experiment directory. In particular, do **not** edit `instructions.md` and do **not** create or edit `FINDINGS.md` — consolidation happens later, in one place, to avoid conflicts between parallel agents.
- **Toolchain:** stable Rust only. Record the exact `rustc --version` in your RESULTS.md. If something turns out to need nightly, that is a finding to record, not a licence to use nightly.
- **Dependencies:** none, with one exception: `trybuild` is allowed as a dev-dependency for compile-fail tests (R4). Crates like `typenum` and `uom` are forbidden (R8).
- **Duplication over sharing:** if you need a helper that another experiment also builds (e.g. type-level numbers), write your own minimal copy inside your crate. Sharing code would serialize the experiments.
- **Compile-time measurements:** run `cargo clean && time cargo build` three times and report the median. We care about orders of magnitude ("2 s vs 2 min"), not precise benchmarks.
- **Timebox:** if one sub-task resists three genuinely different approaches, stop; record the blocker and the best error message verbatim, and move to the next sub-task.
- **Deliverable:** a `RESULTS.md` in your experiment directory containing:
  1. A verdict for each evaluation criterion: **pass** / **pass with complications** / **fail**, each backed by evidence (a code reference, verbatim compiler output, or a measurement).
  2. Representative compiler error messages, verbatim, each with a one-line judgement of how understandable it would be to a modeller.
  3. Candidate findings, each written as a ready-to-paste `FINDINGS.md` entry: what couldn't be expressed (or only with complications), what it cost, and the workaround used (R14).
  4. A recommendation: **adopt** / **adapt** (say how) / **reject** (say why) for the technique tested.
- Unless a criterion is marked fail, all experiment code must compile and `cargo test` must pass, including trybuild compile-fail tests. A fail is demonstrated by a minimal example left in the crate plus the verbatim error.

### EXP-01: Type-level natural numbers — `experiments/exp01-type-level-numbers/`
**Tests:** R12's assumption that stable Rust can express naturals in types, with decrement and constant values, at useful sizes.
**Build:**
1. Peano naturals: `Zero`, `Succ<N>`, and `trait Nat { const VALUE: u64; }`.
2. A `macro_rules!` macro generating aliases `N0`…`N1000`.
3. Type-level addition (`trait Add<B: Nat>: Nat { type Sum: Nat; }`); comparison (less-than) as a stretch goal.
4. The same again with a **binary encoding** (types for bits), for comparison.
**Evaluate:**
- Compile time and the `#![recursion_limit]` needed at N = 10, 100, 500, 1000, for both encodings.
- The error message when `N42` is supplied where `N41` is required — quote it and judge its readability.
- Ergonomics: how bad is it to write and read these numbers with and without the aliases?

### EXP-02: Suppliers and consumers as type-level lists — `experiments/exp02-supplier-consumer/`
**Tests:** the R12 design end to end, including the hardest part — using a supplier several times from inside one generic process.
**Build:**
1. A minimal copy of Peano numbers (do not share with EXP-01).
2. `Cons<H, T>` / `Nil`; `struct Bolt` with a private constructor; `struct BoltBox<Items>` with a private field; alias `type EmptyBoltBox = BoltBox<Nil>;`.
3. `trait Supplier { type Item; type Next; fn supply(self) -> (Self::Item, Self::Next); }`, implemented only for `BoltBox<Cons<H, T>>`.
4. `WasteBag<Space>` with type-level remaining space, and `trait Consumer<In> { type Next; fn consume(self, item: In) -> Self::Next; }`, implemented only while space remains.
5. A fill function or macro (inside the privacy boundary) that builds a full `BoltBox` of N bolts.
6. **Key test:** a generic process that takes four bolts from one supplier, e.g. `fasten_four<S>(s: S, …)`. This needs `S::Next` to also be a supplier, recursively. Try at least: (a) hand-written chained bounds (`S: Supplier, S::Next: Supplier, …`), and (b) a recursive `SupplyN<N>` trait that returns the bolts as a Cons-list plus the depleted supplier. Record exactly how far each gets.
7. trybuild compile-fail tests: supplying from `EmptyBoltBox`; consuming into a full `WasteBag`.
**Evaluate:**
- Does step 6 work at all on stable Rust, and how many where-clauses does a 4-bolt process need?
- Error messages for the two compile-fail cases — would a modeller understand "the box is empty" from them?
- Compile time with capacity 100.

### EXP-03: How close to "nothing is silently lost"? — `experiments/exp03-drop-prevention/`
**Tests:** the known gap in R1/target-language: Rust types are affine, so values can be dropped silently.
**Build:**
1. A catalogue of leak paths as small examples: silent drop at end of scope; `let _ = …`; rebinding/shadowing; `mem::forget`; `mem::drop`; struct-pattern fields dropped with `..`; early `return`; panic unwinding.
2. Counter-mechanisms, each applied to a sample resource type: `#[must_use]` with `#![deny(unused_must_use)]`; a `Drop` impl that panics unless the value was explicitly consumed (a runtime tripwire, defused by a `fn consume(self)` that `mem::forget`s internally); relevant clippy lints (`clippy::mem_forget` and friends) under `-D warnings`.
3. A matrix: leak path × mechanism → caught at compile time / caught at test time / not caught, with a minimal demo for every "not caught" cell.
**Evaluate:**
- The completed matrix is the main result.
- A recommended set of conventions for the real project (which lints, which attributes, whether the panicking-Drop tripwire is worth its noise), with the residual gaps stated plainly.

### EXP-04: Compile-time conservation of quantities — `experiments/exp04-quantity-conservation/`
**Tests:** R3 and R7 — can "mass in = mass out" be a compile error on stable Rust?
**Build:**
1. Quantity types in both styles from R7: `struct Grams<const V: u64>;` (const generics) and the trait-based form; each exposing `VALUE`.
2. `fn split<const IN: u64, const A: u64, const B: u64>(m: Grams<IN>) -> (Grams<A>, Grams<B>)` with a compile-time check that `A + B == IN`. Stable techniques to try, in order: an associated-const assert forced by use (`struct AssertSum<const IN: u64, const A: u64, const B: u64>; impl … { const OK: () = assert!(A + B == IN); }` referenced with `let _ = AssertSum::<IN, A, B>::OK;`); an inline `const { … }` block; anything else found. Record which work inside a generic function on stable, and whether the error fires at definition or at instantiation.
3. `combine` in the same style, and a unit-safety check: adding `Grams` to `Millimetres` must not compile.
4. trybuild compile-fail: splitting 2000 g into 1500 g + 600 g.
**Evaluate:**
- Is a conservation violation a genuine compile error? At what point does it fire, and what does the message look like?
- Caller ergonomics: what does calling `split` correctly look like — do the const parameters infer, or must they all be spelled out?

### EXP-05: Marker traits vs type parameters — `experiments/exp05-characteristics-encoding/`
**Tests:** R6's claim that both encodings work and can be mixed.
**Build:**
1. A catalogue — sizes {M6, M8, M10}, materials {Steel, Brass}, lengths {10 mm, 15 mm, 20 mm} — encoded both ways: (a) marker traits with one struct per combination (macro-generate the 18 structs and impls); (b) a single `Bolt<Size, Material, Length>` with the characteristics as type parameters.
2. The same requirement in both styles: a bound `M8 + Steel + Length15mm` vs accepting `Bolt<M8, Steel, L15>` (and a generic-over-length version of the latter).
3. Bridging: blanket impls such as `impl<M, L> M8 for Bolt<SizeM8, M, L> {}` so parameterized types satisfy marker bounds — do the two styles compose?
4. Add one new size (M12) to each encoding and record everything that had to change.
**Evaluate:**
- Lines of code per encoding (with and without macros); cost of adding a characteristic value and of adding a whole new dimension (e.g. thread pitch).
- Error messages when the wrong bolt is passed, in both styles.
- Whether bridging works cleanly; a recommendation for when to use which style.

### EXP-06: Requirements traceability report — `experiments/exp06-traceability/`
**Tests:** R10 — that requirement links can be extracted mechanically.
**Build:**
1. Three sample requirement traits (`REQ-001`…`REQ-003`) in the R10 pattern, several types and processes satisfying them, and tests tagged `/// Verifies: REQ-…` — including one requirement deliberately left with no verifying test.
2. `trace.sh` — plain grep/awk/sh, no dependencies — producing a table: requirement ID → where defined, satisfied by, verified by; plus a warning list of requirements with no verifying test.
3. Edge cases: several IDs in one `Verifies:` line; an ID mentioned in an ordinary comment (should not count); formatting variations.
**Evaluate:**
- Does the report come out correct — no false positives or negatives on the edge cases?
- What discipline does the code need for grep to stay reliable (exact tag format, one convention for placement)? Write that discipline down as the proposed convention.

### EXP-07: Flows, exclusive resources, reusable resources — `experiments/exp07-flows/`
**Tests:** R2 and R9 — sequencing freedom, compiler-enforced exclusivity, and the ergonomics of threading reusable resources.
**Build:**
1. A mini-model with private constructors: a steel sheet quantity, `Person`, `Drill`, plates, and a fixed handful of bolts (keep it simple — e.g. `[Bolt; 4]` — so this experiment does not depend on EXP-02's hard parts).
2. Processes `cut`, `drill_holes(person, drill, plate) -> (person, drill, drilled_plate)`, `fasten`. Compose the full flow in two different valid orders and show both type-check.
3. trybuild compile-fail tests: the same `Person` moved into two processes at once; a flow using a product that is never produced; a process that swallows the drill (doesn't return it) — show what the caller's error looks like.
4. Aggregation: a `Workshop { person, drill }` struct threaded through instead of loose values; compare.
**Evaluate:**
- Boilerplate per process call when threading reusable resources, loose vs aggregated.
- Do the compile-fail errors point at the right place, and would a modeller understand them as "this person is already busy"?

### EXP-08: The privacy boundary in practice — `experiments/exp08-privacy-boundary/`
**Tests:** R1's private-constructor rule across real crate boundaries.
**Build:**
1. A cargo workspace with two crates: `model-lib` (resource types with private constructors, a boundary module that may create them, test helpers) and `model-user` (depends on `model-lib`, runs processes).
2. Demonstrate that `model-user` can run a process end to end but cannot create a `Bolt` from nothing: trybuild compile-fail from the user crate's side.
3. Test-helper mechanics: show that `#[cfg(test)]` helpers in `model-lib` are **not** visible to `model-user`'s tests, and evaluate a `test-support` cargo feature as the alternative. Which pattern keeps the boundary tight while letting downstream tests construct fixtures?
**Evaluate:**
- The recommended crate/module layout and helper pattern, stated precisely enough to adopt in the real project.
- Any hole found in the boundary (e.g. a way to obtain a resource without the boundary module), with a demo.

### EXP-09: Continuous resources, time budgets, and boundary sinks — `experiments/exp09-continuous-resources/`
**Tests:** R15 — the only requirement with no experiment behind it. Its pieces are individually validated (EXP-04's split pattern, EXP-02's boundary traits); this tests the combination.
**Build:**
1. A sealed quantity container `GasBottle<const REMAINING: u64>` (grams) with a `draw` process using the caller-stated-remainder pattern; the empty state `GasBottle<0>` as a distinct resource that must be accounted for.
2. Overdraw regression tests: drawing 6000 g from a 5000 g bottle must fail to compile. Per the R4 policy this is a post-monomorphization error, so use rustdoc `compile_fail` doc-tests, not trybuild.
3. A time budget on a reusable resource: a `Person` carrying a `Qty<V, Milliseconds>` budget, drawn down across several process calls in one flow; overdraw fails like the bottle. Record the threading ergonomics — does the budget parameter infect every signature the person passes through?
4. A two-dimension balancing process: `burn(fuel, air) -> (exhaust, heat, work)` with a mass assert (fuel + air = exhaust) **and** an energy assert in the same process. Does the two-assert pattern compose cleanly, and what does each violation's error look like?
5. An unbounded boundary object `Atmosphere` with `type Next = Self`:
   - as a **consumer** of two different things (exhaust and heat) — do multiple `Consumer<In>` impls on one boundary object work, and does `ConsumeList`-style repeated use tolerate `Next = Self`?
   - as a **source** of air. **Key design probe:** the R12 `Supplier` trait supplies one fixed `Item` per step, which fits discrete items but not "draw an arbitrary amount of air". Try both: (a) fixed-packet supply (`Item = Air<Qty<PACKET, Grams>>`), and (b) a draw-style process (`draw_air<const TAKE>(atm: Atmosphere) -> (Air<TAKE>, Atmosphere)`) bypassing the `Supplier` trait. Record which fits R15 better and whether the `Supplier` trait should be documented as discrete-only.
6. Integration flow: draw gas, draw the person's time, burn, send exhaust and heat to the atmosphere, return the part-empty bottle and part-spent person. Compose it in two valid orders; both must type-check. Include compile-fail (or `compile_fail` doc-test) cases: waste heat never consumed; the empty bottle dropped silently (should trip the conservation conventions — record which layer catches it).
**Evaluate:**
- Does the container pattern deliver compile-checked finite capacity as R15 claims, and where exactly does the overdraw error fire (with verbatim message)?
- Does `Next = Self` coexist with the boundary traits, `on_unimplemented`, and repeated-use helper traits?
- Is `Supplier` discrete-only in practice? If so, R15 needs an amendment saying continuous boundary sources are draw processes, not `Supplier` impls.
- Threading ergonomics of budgeted resources; error quality for each failure mode; anything else that should change in R15.

### EXP-10: Fallible processes — `experiments/exp10-failure-modes/`
**Tests:** open question 1 (candidate R17). Can a process return *either* outcome — success or failure — with both branches conserving, and does the compiler force flows to handle the failure arm?
**Protocol update for EXP-10..12:** unlike EXP-01..09, these experiments MAY depend on `model-core` by path (`path = "../../model/model-core"`, plus its `test-support` feature under dev-dependencies) — the library exists now, and testing against it is the point. model-core itself stays read-only; `trybuild` remains the only external dev-dependency.
**Build:**
1. Failure-state resources per "one type per state" (R9): `ScrapPlate<const G: u64>`, `BrokenDrillBit`, built with the kernel macros; a scrap consumer and a repair process (`BrokenDrillBit` + parts → working bit) so failure outputs are accounted, not dead ends.
2. An **outcome token** design: variability enters only at the boundary (R12 spirit) — a sealed token type whose values are injected by boundary/test-support constructors, so processes stay deterministic and both outcomes are testable. Probe at least: one token type with success/failure constructors vs two distinct token types selected by the flow.
3. A fallible process `drill_fallible(person, drill, plate, outcome) -> Result<OkBundle, FailBundle>` where both bundles conserve the same inputs (per-branch const asserts; the person comes back in both arms).
4. Flows handling both arms; a repair-and-retry (bounded rework) composition on the failure path.
5. Leak-surface probes: what happens on `.unwrap()` (panic unwinding is the known F-002 gap — demonstrate); a dropped `Result` (which layer catches it); a `match` that forgets resources in one arm.
**Evaluate:** does `Result` + `must_use` + the tripwires force failure handling at compile/test time, and where are the holes (verbatim evidence); per-branch conservation ergonomics; error-message quality; a precise recommendation for the R17 wording.

### EXP-11: Qualifications and safety as types — `experiments/exp11-qualifications/`
**Tests:** open question 2 (candidate R18). Can the compiler refuse an unqualified person or an unguarded machine, with modeller-grade errors? Same protocol update as EXP-10 (model-core by path, read-only).
**Build:**
1. Qualification marker traits (`CertifiedDriller`, `CertifiedWelder`) with a `Qualification`-style kind trait; qualified person types carrying time budgets (probe: markers implemented on downstream `reusable_resource!`/hand-sealed person types generic over the budget, e.g. `impl<const MS: u64> CertifiedDriller for Driller<MS>`), interop with `model_core::common::Person` stated honestly.
2. Safety resources: a `MachineGuard` (reusable) required and returned by the guarded process; a `FittedGuard` state if fitting is a process (one type per state).
3. R10 requirement traits combining both: REQ-style "drilling requires a certified operator and a fitted guard", bound on the process, traced by trace.sh.
4. trybuild: unqualified person rejected (with `on_unimplemented` phrasing like "this person is not certified for drilling"); missing guard rejected; the same person's budget still draws down through a qualified process.
**Evaluate:** ergonomics (how much boilerplate per qualification/per person type); error quality; whether anything belongs in model-core (e.g. a `qualification!` helper or Person redesign) vs pure downstream convention; a precise recommendation for the R18 wording.

### EXP-12: Money as a conserved dimension — `experiments/exp12-money/`
**Tests:** open question 3 (candidate R19). Same protocol update as EXP-10 (model-core by path, read-only).
**Build:**
1. Currency units in the R7 style: one unit type per currency, integer minor units (`Pence` as the working default, plus a second currency to prove non-mixing is a type error).
2. An `Account<const BALANCE: u64>` container (R15 pattern) with `draw_funds`; overspending must be a compile error like any overdraw.
3. **Payment as a process chain** (R12 strictness): a `Vendor` boundary object that is both `Consumer<Money>` and a goods source; a `purchase` process that pays the vendor and obtains the goods, with money and goods each conserved. Change-giving as a split.
4. A currency `exchange` process GBP→second currency with a stated integer rate, const-asserted (`out * DEN == in * NUM` style); document honestly that exchange is value-equivalence at a stated rate, not single-dimension conservation.
5. trybuild/compile-fail: mixing currencies; overspending; paying with the wrong currency.
**Evaluate:** does money need ANY new model-core machinery, or only a base-unit table row plus downstream conventions; rounding/remainder honesty in exchange (integer arithmetic); error quality; a precise recommendation for the R19 wording and the R7 table entry.

### Consolidation (later — not part of the parallel runs)
After all experiments have RESULTS.md files, a single follow-up pass (one agent, not parallel) will:
1. Read every RESULTS.md and create the real `FINDINGS.md` (R14) from the candidate entries.
2. Propose updates to this document: techniques to adopt, defaults to rework, requirements that need weakening (e.g. if compile-time conservation fails) or strengthening.
3. Propose what to build first for the real library.
This pass waits for the user's go-ahead, and its instruction changes are agreed with the user before being committed.

## Open questions / future requirements
Raised 2026-10-02. The original items 1–3 (failure modes, qualifications and safety, money) were resolved by EXP-10..12 into R17–R19 on 2026-10-02. Ordered roughly by how well each fits the type system.
1. **Validation vs verification.** Tests (R5) prove the model self-consistent; nothing yet checks it against reality (does cutting really take 3 s and produce 500 g of swarf?). Needs calibration data and deserves its own requirement so the two are never confused.
2. **Document generation.** Converting the model back to natural-language work instructions for real-world implementation: process signatures are work-instruction skeletons; `VALUE` constants supply the numbers (R7); `Placeholder:` tags are the open-items list; trace.sh output is the compliance matrix; fill functions are the bill of materials; R9 flows emit a dependency graph, not a fixed procedure. Costs only conventions we nearly have: structured greppable doc-comment fields (`/// Actor:`, `/// Duration:`) and real-world naming on every public item. Candidate R20 plus a `docgen` tool over rustdoc JSON. The forward direction (natural-language spec → model) is already adopted: see `SPEC_TEMPLATE.md`.
3. **Change impact and subsystem decomposition** — two strengths worth documenting: changing a type makes the compiler enumerate every affected process (impact analysis for free), and crate boundaries are natural subsystem interfaces for multi-team work (R1 layout already points this way).
4. **Tolerances.** Exact integers cannot say "1500 g ± 10 g". Min/max const pairs could, at real complexity cost — logged as a known limitation rather than solved now.
