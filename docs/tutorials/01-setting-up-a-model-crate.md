# Tutorial 01 — Setting up a model crate

> Where a model lives, what its `Cargo.toml` must say, which crate attributes
> are mandatory, and what the CI gate actually checks. This tutorial teaches
> *why each line exists*; the copy-paste reference is
> [modeller's guide §2](../modellers-guide.md#2-setting-up-a-model-crate).
> One sitting; previous: [00 — Rust on-ramp](00-rust-on-ramp.md).

## 1.1 The shape of the repository

Three layers matter to a modeller:

- **`model/model-core/`** — the kernel library: type-level numbers, units and
  quantities, the sealed-resource macros, the boundary traits, `Person` and
  `History`, the requirement macros. You *use* it; you never edit it.
- **`model/<your-model>/`** — each model is its own crate, downstream of
  `model-core`. The crate edge is the privacy wall (0.9): your resources are
  constructible only inside your crate. Reference layouts:
  `model/cs1-pot-of-tea/` (small, continuous resources) and
  `model/pilot-workshop/` (fallibility, qualifications, money).
- **a natural-language spec** in `case-studies/<name>/SPEC.md`, written from
  `SPEC_TEMPLATE.md`. The model implements the spec *mechanically*; tutorial
  06 walks the full round trip on a mini-spec.

A new model crate is added as a workspace member in `model/Cargo.toml`
(`members = ["model-core", "pilot-workshop", "cs1-pot-of-tea"]` + yours) so
the single CI gate covers it. (The exercises crate `learn/` sits deliberately
*outside* the workspace — a sandbox that cannot break the gate — but its
manifest has exactly the shape yours needs.)

## 1.2 The `Cargo.toml`, line by line

From `model/cs1-pot-of-tea/Cargo.toml` (abridged); every line is
load-bearing:

```toml
[features]
test-support = []                                # (1)

[dependencies]
model-core = { path = "../model-core" }          # (2) NO features

[dev-dependencies]
model-core = { path = "../model-core", features = ["test-support"] }   # (3)
cs1-pot-of-tea = { path = ".", features = ["test-support"] }           # (4)
trybuild = "1"                                   # (5)
```

1. The kernel macros generate a `test_fixture()` constructor for every
   resource — a sealed resource from nowhere, for tests — gated behind *your*
   crate's `test-support` feature, so you declare it (F-004).
2. The production dependency takes **no features**. This is what makes it
   provable that production code cannot conjure fixtures.
3. Tests re-declare the dependency *with* the feature. Features declared
   under `[dev-dependencies]` apply only to test builds.
4. The same trick on your own crate, so your integration tests can use your
   own fixtures.
5. `trybuild` is the one allowed dev-dependency (R4): it pins compile-fail
   tests — files that *must not* compile, with their expected errors.

Why not `#[cfg(test)]` helpers instead of a feature? Because a dependency is
always compiled without `cfg(test)` — a downstream crate could never see
them. And why does CI need a special step to police this? Because during
`cargo test`, cargo *unifies* features: your production sources get compiled
against the featured library, so `cargo test` passing does **not** prove
production code avoids fixtures — only a plain `cargo build` does (F-004).
You don't need to memorise this; you need to not "simplify" the manifest.

The feature must never appear under `[dependencies]` anywhere — the gate
greps for it.

## 1.3 The five crate attributes

Top of every modelling crate's `src/lib.rs` — and of **every file in
`tests/`**, because each of those is its own crate (0.9):

```rust
#![recursion_limit = "2048"]   // type-level capacities need ≈ N + 3 (F-010)
#![forbid(unsafe_code)]        // `unsafe` could mint a sealed resource (F-005)
#![deny(unused_must_use)]      // discarding a resource-bearing result: error
#![deny(let_underscore_drop)]  // `let _ = resource`: error
#![warn(missing_docs)]         // every public item documented (lib.rs only)
```

The middle three are the compile-time layer of the conservation regime
(tutorial 03 adds the test-time layer). Forgetting `recursion_limit` in a new
test file is a classic: everything works until a supplier holds ~130 items,
then `E0275 overflow evaluating the requirement` — the number is too big for
the current limit (F-010).

One lint you must *not* add: `clippy::shadow_*` — it fires on the
`let kettle = boil(kettle, …)` rebinding idiom, which is the house style
(0.3, F-007).

## 1.4 The CI gate: `ci.sh`

```sh
cd model
./ci.sh
```

Six steps, any failure fails the gate — and each exists because of a measured
finding, not ceremony:

| Step | What | Why |
|---|---|---|
| 1 | `cargo build --workspace` | conservation asserts fire only in a *full* build (F-001) — never trust `cargo check` |
| 2 | `cargo test --workspace` | unit + flow tests, trybuild cases, `compile_fail` doc-tests, tripwire demos |
| 3 | clippy, `-D warnings` + restriction lints | the leak-prevention lint set is only sound as a whole (F-007) |
| 4 | a **plain no-features build** | the only step that proves production code can't call `test_fixture()` (F-004) |
| 5 | `./trace.sh` | the requirements traceability report; a requirement with no verifying test fails CI |
| 6 | a feature-placement grep | `test-support` under `[dev-dependencies]` only |

The habit to build now: **run `./ci.sh` before claiming anything works**, and
run a real `cargo build`/`cargo test` (not just your editor) before believing
a conservation-sensitive change compiles.

---

## Checkpoint

Run the gate on the untouched repository:

```sh
cd model
./ci.sh
```

It takes a minute or two. You should see each step announce itself
(`==> [1/6] cargo build --workspace …`) and the run end with exactly:

```text
CI gate passed: build, test, clippy, plain production build, traceability, feature placement.
```

Scroll back to step 5's output: that is the traceability report — every
requirement in the workspace with the processes satisfying it and the tests
verifying it, generated from the source by grep conventions you'll learn in
tutorial 02.

Next: [02 — Resources and requirements](02-resources-and-requirements.md).
