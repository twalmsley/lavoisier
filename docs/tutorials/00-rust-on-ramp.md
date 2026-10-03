# Tutorial 00 — The Rust on-ramp (for systems engineers)

> You know systems: resources, processes, boundaries, conservation,
> requirements. This tutorial is the minimal Rust you need to *read and write
> models in this project* — nothing more. It is not a Rust course; every
> concept below is here because the method uses it for something specific,
> and that something is named as we go. One sitting.
>
> The series: 00 (this) → [01 Setting up](01-setting-up-a-model-crate.md) →
> [02 Resources and requirements](02-resources-and-requirements.md) →
> [03 Processes and conservation](03-processes-and-conservation.md) →
> [04 Boundary, flows and History](04-boundary-flows-and-history.md) →
> [05 Reading the errors](05-reading-the-errors.md) →
> [06 Capstone: toast](06-capstone-toast.md). The destination is the
> [modeller's guide](../modellers-guide.md) — after the series you should be
> able to read it, and CS-1's code, unaided.

## 0.1 Why this language

The project's premise (`instructions.md`, R1–R19): if **types stand for
things** and **functions stand for processes**, the compiler becomes the
model auditor. Rust was chosen for one property — *ownership*: every value
has exactly one owner, and passing a value to a function **moves** it there.
That is resource conservation, built into the language. Most of what follows
is ordinary Rust; what's unusual is only how literally this project takes it.

## 0.2 The toolchain, and where you will type things

You need stable Rust 1.98.1 (installed via [rustup](https://rustup.rs) or a
package manager). Three commands matter:

- `cargo build` — compile. In this project, the *real* verdict (see 0.10).
- `cargo test` — compile **and** run the tests. Your main feedback loop.
- `cargo check` — a fast half-compile that editors use. Useful, but it
  **cannot see conservation errors** — tutorial 03 demonstrates this; until
  then, just remember: green editor ≠ green model.

You will type things in `learn/`, the exercises crate at the repository root.
Each exercise is one file in `learn/tests/`, it starts broken, and
`cargo test --test <name>` tells you how. `learn/README.md` has the list.

## 0.3 Values move — this is the conservation rule

A `let` binds a name to a value. Passing the value somewhere **moves** it:
the old name becomes unusable, enforced at compile time.

```rust
let person = new_person::<10_000>();          // a person enters the model
let (labour, person_back) = draw_time::<2_000, 8_000, 10_000>(person);
// `person` is GONE — it moved into draw_time. Using it again is an error:
// "use of moved value: `person`". The person you have now is `person_back`.
```

Read moves as the model rule they encode: **one resource is in one process at
a time** (R2 in `instructions.md`). A process that uses a reusable resource
returns it, and you continue with the returned one. The idiomatic spelling
re-uses the same name — called *shadowing*, and in this project it is the
house style, not a trick:

```rust
let (person, filled) = fill_kettle(person, kettle, water);  // person: the returned one
```

Two things ordinary Rust programs do that **this project never does**:

- `.clone()` / `Copy` — duplicating a value. A duplicated resource is matter
  from nothing; resource types here implement neither, so the compiler
  refuses.
- `&` / `&mut` *borrowing* — lending a value instead of moving it. Process
  signatures here never borrow resources (R2). When the compiler's error
  *suggests* borrowing or cloning, the suggestion is wrong for this project
  — always (see §4.10 of the [modeller's guide](../modellers-guide.md)).

## 0.4 Structs are resources

A `struct` is a named record type. Resources are structs whose fields are
**private**, which is the whole trick for "no creating resources from
nothing" (R1):

```rust
pub struct Kettle {
    _seal: (),      // a private, zero-size field: the seal
}
```

`()` is the empty "unit" type — the field holds no data; it exists so the
struct has *a private field*, because a struct with one can only be
constructed where that field is visible. Outside the defining crate,
`Kettle { _seal: () }` is a compile error (`E0451: field `_seal` … is
private`). Resources therefore come into existence only where the model says
they may: at the boundary (tutorial 04).

You will rarely write these structs by hand — macros generate them correctly
(tutorial 02) — but you will read them everywhere, e.g.
`model/cs1-pot-of-tea/src/resources.rs`.

## 0.5 Functions are processes

A function takes parameters and returns a value; several values travel as a
*tuple* `(a, b, c)`, unpacked by *destructuring*:

```rust
pub fn fill_kettle<const B: u64, const G: u64>(   // B, G: numbers in the types — see 0.8
    person: Person<B>,
    kettle: Kettle,
    water: ColdWater<G>,
) -> (Person<B>, FilledKettle<G>) { /* … */ }

let (person, filled) = fill_kettle(person, kettle, water);
```

(That is CS-1's real P1, `model/cs1-pot-of-tea/src/resources.rs`.)

Read the signature as a work instruction: everything the process needs moves
in by value; everything it produces — product, waste, and every reusable it
borrowed — comes back out (R1). There is no global state and no side door:
if a process's signature doesn't mention a resource, the process cannot touch
it.

## 0.6 Traits are characteristics

A `trait` is a named capability a type may have; `impl Trait for Type`
declares that it has it. The method uses *marker traits* — no methods, pure
declaration — as **characteristics** (R6):

```rust
pub trait Boiling { /* … */ }                      // the characteristic
impl Boiling for BoilingKettle<G, E> { /* … */ }   // only this state has it
```

One line attaches a characteristic to a type. The kettle's *filled* state has
no `Boiling` impl — and that absence is checkable, which is the next step.

## 0.7 Generics with bounds are requirements

A generic function takes a *type parameter*, and a **bound** restricts which
types are acceptable:

```rust
pub fn pour_and_brew<K: Req006PouredAtTheBoil, /* … */>(kettle: K, /* … */)
```

`K: Req006PouredAtTheBoil` reads: "any kettle, *provided* it satisfies
REQ-006". Requirements in this project are exactly this — named traits used
as bounds (R10) — and passing a non-boiling kettle produces a compile error
*phrased as the requirement*, REQ id and all (you'll see one verbatim in
tutorial 02).

One syntax note: `::<…>` after a function name (the "turbofish") supplies
generic arguments explicitly — `draw_time::<2_000, 8_000, 10_000>(person)` —
used when the compiler can't infer them, which with numbers is most of the
time.

## 0.8 Const generics are quantities

A type parameter can be a **number**: `ColdWater<const V: u64>` (a `u64` is a
non-negative integer). Then `ColdWater<1500>` — fifteen hundred grams of cold
water — is a *different type* from `ColdWater<1400>`. Quantities live in the
types (R7: integers, base units — grams, millimetres, milliseconds, joules),
so quantity mistakes are type mistakes, and the compiler can check a mass
balance before anything runs (tutorial 03).

## 0.9 Modules and privacy are the boundary

Code lives in modules (`mod boundary { … }`); each crate (= one compilation
unit, one `Cargo.toml`) is the outermost privacy wall. `pub` makes an item
visible outside its module; `pub(crate)` means "public inside this crate,
invisible outside it".

The method leans on this hard: a resource's constructor is `pub(crate)`, so
**only the defining crate's own boundary and processes can create it** —
downstream code physically cannot (F-006). That is why each model is its own
crate, and why the exercises put each mini-model in its own file (an
integration-test file compiles as its own crate — `learn/README.md`
explains).

## 0.10 `Result` and `match` are fallible processes

A process that can fail returns `Result<OkBundle, FailBundle>` — a value that
is *either* outcome — and `match` forces you to write both arms:

```rust
match drill_holes_fallible(/* … */) {
    Ok(ok)    => { /* account for every resource in the success bundle */ }
    Err(fail) => { /* account for every resource in the failure bundle */ }
}
```

Both arms conserve the same inputs: a failure output (a scrapped part) is a
product, not a disappearance (R17). The usual Rust shortcut `.unwrap()`
("crash if it failed") is deliberately made a compile error here. CS-1 has no
fallible processes — they live in `model/pilot-workshop/` — so this is all
you need for now.

## 0.11 Where tests live

Two kinds, both run by `cargo test`:

```rust
/// Verifies: REQ-006           <- traceability tag (tutorial 02)
#[test]
fn boil_conserves_energy() { assert_eq!(2 + 2, 4); /* … */ }
```

- **unit tests** in `#[cfg(test)] mod tests` inside a source file — inside
  the privacy wall;
- **integration tests** in `tests/*.rs` — each file its own crate, *outside*
  the wall, living under exactly the discipline a downstream user does. The
  model's flows are written here (tutorial 04).

`assert_eq!(a, b)` fails the test if the two differ; `#[should_panic]` tests
expect a crash — the project uses them to *demonstrate* its leak-detection
tripwires.

---

## Checkpoint

1. Toolchain:

   ```sh
   rustc --version
   ```

   You should see `rustc 1.98.1` (the hash and date after it vary by
   install).

2. Your first broken model. From the repository root:

   ```sh
   cd learn
   cargo test --test ex01_moves
   ```

   You should see exactly this error (plus a hint-carrying warning):

   ```text
   error[E0382]: use of moved value: `person`
   ```

   with the compiler pointing at the second `draw_time` call: the person was
   moved into job 1, and job 2 is claiming the stale name. Open
   `learn/tests/ex01_moves.rs`, follow the `// TODO`, and re-run until you
   see:

   ```text
   test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; …
   ```

   While fixing it, notice what you may *not* do: cloning the person, or
   following any "consider borrowing" suggestion. The fix is always to take
   the resource from the previous process's output.

Next: [01 — Setting up a model crate](01-setting-up-a-model-crate.md).
