# Using a Statically-Typed Programming Language for Model-Based Systems Engineering

## Status
Requirements are being agreed. **Do not write code yet.** Wait until the user says they are happy with these instructions.

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
- Known gap: Rust lets a value be dropped silently, because its types are affine, not linear. So "nothing is lost" is only partly enforced at compile time. The rest is covered by `#[must_use]`, conventions and unit tests.

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
  - clearly marked test helpers, available only under `#[cfg(test)]` or a dedicated test-support feature

### R2. Strict conservation: resources are never shared
- Resources are never borrowed or shared. Process signatures must not take resources as `&T` or `&mut T`, and must not use `Rc`, `Arc` or similar.
- A reusable resource is **moved in and returned**. The caller gets it back as part of the function's output.
- As a result, one resource can be in use by only one process at a time. The compiler enforces this.

### R3. Splitting and combining amounts are processes
- Changing an amount is modelled as a function, exactly like any other process. The function takes the input quantity types and returns the output quantity types. Examples:
  - splitting 2000 g of steel into a 1500 g part and 500 g of waste
  - combining several amounts into one
- These functions must conserve the amount. The total going in equals the total coming out, including waste. Where possible this is checked at compile time using the types' constant values (e.g. `const { assert!(IN::VALUE == A::VALUE + B::VALUE) }`), and otherwise by unit tests.

### R4. Compilation errors expose model mismatches
- If a process is given a missing or wrong resource, the model must fail to compile.
- The project should include compile-fail examples that show this, e.g. using `trybuild` or doc-tests marked `compile_fail`.

### R5. Unit tests verify transformations
- Each process has unit tests that show it turns specific inputs into the expected outputs, and that nothing is left unaccounted for.

### R6. Characteristics are encoded in the type system
- A type's characteristics are expressed through the traits it implements (marker traits), **or** through type parameters. Either approach is acceptable, and they can be mixed.
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

  "Default" units are sensible starting choices for the experiment and can be reworked later if they cause problems.
  - Units made from other units follow from the agreed base units, e.g. area is length × length, so it's in mm². This keeps calculations consistent without conversion factors. A litre is therefore 1,000,000 mm³.
  - Temperature is in kelvin so values are never negative and fit unsigned integers.
  - For any new dimension, pick a sensible default in the same style, add it to this table marked "default", and mention it in the findings log if the choice causes problems.
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
- Types and functions that satisfy a requirement do so by implementing or using its trait, so the link can be found in the code.
- **Tests are tagged** with the IDs of the requirements they verify, as a line in the test's doc comment:
  ```rust
  /// Verifies: REQ-001, REQ-004
  #[test]
  fn fasten_joins_two_plates() { /* ... */ }
  ```
- These tags should be easy to find mechanically (e.g. with grep or a small script), so you can report which requirements are satisfied and which are verified.

### R11. Library of reusable common types
- A shared library of common resource types, such as `Person`, `Organisation` and `Location`, plus common units and quantities.

### R12. Suppliers and consumers at the system boundary
- Raw materials and other inputs enter the model only through explicit **suppliers**. Waste products and final products leave it only through explicit **consumers**.
- Suppliers and consumers are defined by **project traits**, `Supplier<Out>` and `Consumer<In>`, not by plain closure types. Each trait has an associated type for its next state (e.g. `type Next`), because supplying or consuming changes its type. The traits can carry a name and requirement links (R10).
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
- **Suppliers hold real objects.** A supplier contains the actual item objects it will supply, e.g. a `BoltBox` contains 100 `Bolt` values. It doesn't create items from a count. Supplying hands over one of the objects it holds. Likewise, a consumer keeps the objects it has consumed. This is the starting approach; if it proves impractical, record that in the findings log (R14) and review it.
- **Capacity is checked at compile time.** The remaining capacity is part of the type. For example, a box of 100 bolts, `BoltBox<Bolts<N100>>`, supplies one bolt and becomes `BoltBox<Bolts<N99>>`.
- `Supplier` is not implemented for a supplier with no capacity left, and `Consumer` is not implemented for a consumer with no space left. Asking either one to do more is therefore a compilation error.
- Stable Rust can't do arithmetic on const generics (e.g. `N - 1`), so the project will need its own **type-level numbers**, e.g. Peano-style `Succ<Zero>` or binary encodings. They belong in the project's own library (R8), not in an external crate such as `typenum`. Each type-level number must also expose its value as a constant, as in R7.
- **Items are stored in a type-level list.** A supplier's contents are a nested list built into its type, e.g. `BoltBox<Cons<Bolt, Cons<Bolt, Nil>>>`. Supplying removes the first item and returns a supplier holding the rest. `Supplier` is implemented only for a non-empty list (`Cons<H, T>`), never for `Nil`. The count is the length of the list, so count and contents can't disagree. A recursive trait exposes that length as a constant (R7).
  - Consumers work the other way round: consuming adds an item to the front of the list. A consumer also has a type-level number for its remaining space, which goes down by one each time.
  - Large lists are written with helpers, e.g. a type alias such as `Bolts<N100>` built from the project's type-level numbers, plus a fill function that creates a full supplier (inside the privacy boundary, R1).
  - Known costs to watch for and record in the findings log (R14): very long type names in compiler errors, the compiler's recursion limit (128 by default; it can be raised with `#![recursion_limit]`), and compile time at realistic sizes.
  - Fallback, if this becomes unworkable: a fixed-size array of `Option` slots, with the remaining count held as a type-level number. It is simpler, but the compiler no longer checks that count and contents match; only encapsulated code keeps them consistent.
- Once exhausted or full, a supplier or consumer **becomes a new resource type**, e.g. `EmptyBoltBox` or `FullWasteBag`. With the type-level list this happens naturally: `BoltBox<Nil>` is already a distinct type, and a readable alias such as `type EmptyBoltBox = BoltBox<Nil>;` can name it. That new resource must also be accounted for: it goes to a consumer or into another process, just like any other output.

### R13. Discrete items are separate objects
- Each countable item, such as a bolt, is its own object with its own type. It is never an amount with a "count" unit.
- A number of items is held as a collection of those objects. Collections have a **fixed size known at compile time** wherever possible, e.g. an array `[B; N]` or a struct of named items. A variable-size collection such as `Vec<B>` is used only where a fixed size is impossible. Each use needs a comment explaining why, and it counts as a limitation to record (R14).
- Continuous material (mass, length, time) is still modelled as quantity types (R7), and split or combined by processes (R3).

### R14. Record findings and limitations
- Because this is an experiment, the project keeps a findings log in `FINDINGS.md` at the root of the repository. It records each place where the type system couldn't express something, or could express it only with significant complications, along with any workaround used.
- Examples of things to log: runtime checks used instead of compile-time ones, `Vec` used instead of a fixed-size collection, confusing compiler error messages, long compile times, and gaps in conservation checking (such as values being dropped silently).

## Open questions
None at present.
