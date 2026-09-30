# EXP-08 Results: The privacy boundary in practice

**Tests:** R1's private-constructor rule across real crate boundaries.
**Toolchain:** `rustc 1.98.1 (48a229cea 2026-09-01) (Homebrew)`, `cargo 1.98.1`. Stable only.
**Dependencies:** `trybuild 1.0.121` as dev-dependency of `model-user` only.
**Clean build time:** `cargo clean && time cargo build` three runs: 0.94 s, 0.97 s, 0.91 s → median **0.94 s**. Privacy costs nothing at compile time; the seal field `()` is zero-sized, so it costs nothing at runtime either.
**Test status:** `cargo test` — 13 tests pass, including 1 trybuild test covering 5 compile-fail cases. All code compiles; the deliberate holes are demonstrated by *passing* tests in `model-user/tests/holes.rs` (each passing test is a boundary breach succeeding).

## Layout built

```
exp08-privacy-boundary/            (workspace)
├── model-lib/
│   └── src/
│       ├── lib.rs                 re-exports resources::boundary (and, feature-gated, resources::test_support)
│       ├── resources.rs           sealed types + child modules: boundary, test_support (#[cfg(feature)]),
│       │                          test_helpers (#[cfg(test)])
│       ├── processes.rs           fasten(bolt, plate, plate) -> FastenedAssembly; no field access needed
│       └── holes.rs               one deliberately leaky type per known hole
└── model-user/
    ├── src/lib.rs                 assemble_one(): end-to-end process, boundary-only
    └── tests/
        ├── boundary.rs            end-to-end + test-support fixture tests
        ├── holes.rs               working demo of every hole
        ├── compile_fail.rs        trybuild driver
        └── ui/*.rs + *.stderr     5 compile-fail cases
```

The load-bearing design decision: **`boundary` and `test_support` are child modules of `resources`**, re-exported at the crate root. Rust field privacy extends to child modules, so they can construct resources while *every other module of model-lib itself* cannot. The boundary is compiler-enforced even inside the defining crate, not just across it. `processes::fasten` sits outside the module tree and needs one `pub(crate)` combinator (`FastenedAssembly::assemble`), which can only wrap resources it is handed, never create them.

## 1. Verdicts per evaluation criterion

### C1. `model-user` runs a process end to end but cannot create a `Bolt` from nothing — **pass**
- End to end: `model-user/src/lib.rs::assemble_one` procures a `BoltBox` and two `Plate`s through `model_lib::boundary`, runs `processes::fasten`, and the test `user_crate_runs_process_end_to_end` passes.
- Cannot create: trybuild cases `create_bolt_tuple_ctor.rs` (E0423), `create_plate_literal.rs` (E0451) and `bolt_has_no_default.rs` (E0599) all fail to compile from the user crate's side with the expected errors (verbatim below). A single private zero-sized field (`Bolt(())` / `_seal: ()`) is sufficient and free.

### C2. `#[cfg(test)]` helpers in model-lib are NOT visible to model-user's tests — **pass**
- `model-lib` has `#[cfg(test)] pub mod test_helpers` which its own unit test uses successfully (`internal_helper_makes_a_bolt`).
- From `model-user`, even in a dev/test build, the module does not exist: trybuild case `cfg_test_helper_invisible.rs` fails with E0433, and rustc's note explicitly says the item was configured out by `#[cfg(test)]`. This is by construction: a dependency is always compiled without `cfg(test)`; only the crate under test gets it.
- Conclusion: `#[cfg(test)]` helpers are for the defining crate's own tests **only**. They can never serve downstream fixtures.

### C3. `test-support` cargo feature as the downstream alternative — **pass with complications**
The pattern:

```toml
# model-user/Cargo.toml
[dependencies]
model-lib = { path = "../model-lib" }                              # no feature
[dev-dependencies]
model-lib = { path = "../model-lib", features = ["test-support"] } # tests only
```

- Works as intended: `test_support_fixtures_work_in_downstream_tests` builds a `Bolt` and two `Plate`s from fixtures and runs the process on them. Production `cargo build` of `model-user` cannot see the module (verbatim E0433 below, with rustc naming the gating feature).
- **Complication A (feature unification):** during `cargo test`, Cargo unifies features across normal and dev dependencies, so the *entire* build graph — including `model-user`'s own library code — is compiled against a `model-lib` that has `test-support` on. A fixture call accidentally placed in production code compiles fine under `cargo test` and is only rejected by `cargo build`/`cargo check`. Measured directly: with a probe `pub fn probe()` calling `test_support::bolt_fixture()` in `model-user/src/lib.rs`, `cargo build` failed with E0433 while `cargo test --no-run` finished successfully. CI must therefore run a plain `cargo build` (or `cargo check`) as well as `cargo test`.
- **Complication B (trybuild blind spot):** trybuild ui tests are compiled with the dev-dependency feature set, so a compile-fail test asserting "production code can't call `test_support`" wrongly *succeeds at compiling* ("Expected test case to fail to compile, but it succeeded."). The feature boundary cannot be proven by trybuild; only the plain `cargo build` of a real production target proves it. (Probe removed after capture; error verbatim below.)
- **Complication C (viral feature):** if any crate anywhere in a production dependency graph enables `test-support` as a *normal* feature, unification switches it on for everyone. Mitigation is convention: the feature must only ever appear under `[dev-dependencies]`, which a one-line grep over Cargo.tomls can police.

### C4. Holes in the boundary — **pass with complications** (boundary is sound only under discipline; every hole demonstrated)
Sealed types themselves (private field, no derives, no public ctor) showed **no safe-Rust hole**: literal syntax, tuple-ctor call, `Default`, and functional-update syntax are all privacy errors, and pattern-destructuring a sealed type is equally rejected. But the pattern is one attribute away from broken. Each hole below is demonstrated by a passing test in `model-user/tests/holes.rs` against a deliberately leaky type in `model-lib/src/holes.rs`:

| # | Hole | Demo | Severity |
|---|------|------|----------|
| 1 | `#[derive(Default)]` — the derive is a public constructor regardless of field privacy | `DefaultHole::default()` compiles downstream | Creates from nothing |
| 2 | All fields `pub` — struct literal works downstream | `PubFieldsHole { serial: 42 }` | Creates from nothing |
| 2b | `pub` **unit struct** — its name is a value expression | `let _ = UnitHole;` | Creates from nothing |
| 3 | `pub enum` — every variant is a public constructor; variant fields **cannot** be private | `EnumHole::Pristine`, `EnumHole::Stamped(7)` | Creates from nothing |
| 4 | `#[derive(Clone)]` — duplication, not creation | `fn duplicate(one) -> (one, copy)` compiles | Violates conservation |
| 5 | Any public fn/trait-impl returning `Self` from data (`serde::Deserialize` has exactly this shape; simulated as `from_bytes` since deps are forbidden) | `DeserializeHole::from_bytes(b"...")` | Creates from nothing |
| 6 | `unsafe` — privacy is a safe-Rust guarantee only | `unsafe { std::mem::zeroed::<Bolt>() }` mints a real sealed `Bolt` | Creates from nothing |

Mitigations verified or identified:
- Holes 1, 4, 5: **never** derive/impl `Default`, `Clone`, `Copy`, `Deserialize` (or write any public data-to-`Self` fn) on a resource. `bolt_has_no_default.rs` shows the closed state (E0599). Mechanically greppable (`derive(.*Default|Clone|Copy|Deserialize)` next to resource definitions).
- Hole 2/2b: every resource must carry a private seal field. A unit struct can never be a resource.
- Hole 3: a `pub enum` can never be a resource type. `#[non_exhaustive]` on a struct-like variant does block downstream construction (verified: trybuild case `non_exhaustive_variant.rs`, E0639) but is fragile (works per-variant, does not restrict the defining crate, unit variants are trickier); the robust form is a struct with a private field wrapping a private enum.
- Hole 6: `#![forbid(unsafe_code)]` in every modelling crate, as policy/CI — the library cannot impose it on downstream crates.

### C5. Recommended layout stated precisely enough to adopt — **pass** (see §4)

## 2. Representative compiler errors (verbatim) with readability judgements

Tuple-struct constructor (`create_bolt_tuple_ctor.stderr`):

```
error[E0423]: cannot initialize a tuple struct which contains private fields
 --> tests/ui/create_bolt_tuple_ctor.rs:6:17
  |
6 |     let _bolt = Bolt(());
  |                 ^^^^
  |
note: constructor is not visible here due to private fields
 --> $WORKSPACE/model-lib/src/resources.rs
  |
  | pub struct Bolt(());
  |                 ^^ private field
```
Judgement: excellent — a modeller reads "constructor is not visible here" as "you are not allowed to make bolts", and the note points at the sealed definition.

Struct literal (`create_plate_literal.stderr`):

```
error[E0451]: field `_seal` of struct `Plate` is private
 --> tests/ui/create_plate_literal.rs:5:26
  |
5 |     let _plate = Plate { _seal: () };
  |                          ^^^^^ private field
```
Judgement: clear, though "field `_seal` is private" is one step less direct than E0423; naming the field `_seal` makes the intent self-describing.

`#[cfg(test)]` helper from downstream (`cfg_test_helper_invisible.stderr`):

```
error[E0433]: cannot find `test_helpers` in `resources`
 --> tests/ui/cfg_test_helper_invisible.rs:5:39
  |
5 |     let _bolt = model_lib::resources::test_helpers::bolt();
  |                                       ^^^^^^^^^^^^ could not find `test_helpers` in `resources`
  |
note: found an item that was configured out
 --> $WORKSPACE/model-lib/src/resources.rs
  |
  | #[cfg(test)]
  |       ---- the item is gated here
  | pub mod test_helpers {
  |         ^^^^^^^^^^^^
```
Judgement: very good — rustc explicitly says the item exists but "is gated here" with the `#[cfg(test)]` shown; nothing to explain.

Production code touching the `test-support` feature (captured from the transient probe, `cargo build` in model-user):

```
error[E0433]: cannot find `test_support` in `model_lib`
  --> model-user/src/lib.rs:33:16
   |
33 |     model_lib::test_support::bolt_fixture()
   |                ^^^^^^^^^^^^ could not find `test_support` in `model_lib`
   |
note: found an item that was configured out
  --> model-lib/src/lib.rs:22:20
   |
21 | #[cfg(feature = "test-support")]
   |       ------------------------ the item is gated behind the `test-support` feature
22 | pub use resources::test_support;
   |                    ^^^^^^^^^^^^
```
Judgement: excellent — names the exact feature; a modeller immediately sees this is test-only machinery.
(The same probe under `cargo test --no-run`: `Finished 'test' profile ... target(s) in 1.09s` — i.e. **no error**. That silent success is Complication A.)

trybuild blind spot (Complication B), verbatim from the removed probe ui test:

```
test tests/ui/probe_test_support.rs ... error
Expected test case to fail to compile, but it succeeded.
```
Judgement: misleading if you don't know about feature unification — the test is wrong, not the boundary.

Non-exhaustive variant mitigation (`non_exhaustive_variant.stderr`):

```
error[E0639]: cannot create non-exhaustive variant using struct expression
 --> tests/ui/non_exhaustive_variant.rs:6:19
  |
6 |     let _minted = SealedEnumMitigation::Pristine {};
  |                   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```
Judgement: terse and jargon-heavy ("non-exhaustive variant" means nothing to a modeller) — one reason to prefer sealed structs over mitigated enums.

## 3. Candidate FINDINGS.md entries (ready to paste)

- **Privacy boundary works across crates, but only inside a module tree within the defining crate.** Field privacy stops at module boundaries, not crate boundaries, so "only the boundary module may create resources" is compiler-enforced against the rest of `model-lib` only if the boundary (and `test_support`) are *child modules of the module defining the resource types*. Cost: resources, boundary and test-support must share one module file (or use `pub(crate)` constructors and accept convention-only enforcement inside the crate). Workaround used: `resources.rs` contains the types plus nested `pub mod boundary` / `pub mod test_support`, re-exported at the crate root for ergonomics. (EXP-08)

- **`#[cfg(test)]` helpers can never serve downstream tests.** A dependency is always compiled without `cfg(test)`; only the crate currently under test gets it. Verified by trybuild: `model_lib::resources::test_helpers` is E0433 from `model-user` even in a dev build, with rustc noting "found an item that was configured out". Workaround: a `test-support` cargo feature, enabled by downstream crates via `[dev-dependencies]` only. (EXP-08)

- **The `test-support` feature leaks into non-test code during `cargo test` (feature unification).** With `model-lib = { features = ["test-support"] }` under `[dev-dependencies]`, `cargo test` compiles the whole graph — including the downstream crate's production sources — against the featured `model-lib`. A fixture call in production code passes `cargo test` and is only rejected by plain `cargo build`/`cargo check` (verified both ways with a probe). Cost: CI must run `cargo build` in addition to `cargo test`; also, trybuild ui tests inherit dev-dependency features, so trybuild *cannot* be used to prove the feature is off ("Expected test case to fail to compile, but it succeeded."). Residual risk: any crate enabling `test-support` as a normal dependency feature turns it on for the whole graph — policed only by a grep over `Cargo.toml`s. (EXP-08)

- **The private-constructor pattern is one attribute away from broken; the compiler will not warn.** Demonstrated from the downstream crate: `#[derive(Default)]` mints resources despite private fields; `#[derive(Clone)]` duplicates them; all-`pub` fields, `pub` unit structs and every variant of a `pub enum` are public constructors (enum variant fields cannot be private at all); any public data-to-`Self` fn (the shape of `serde::Deserialize`) is a constructor by another name; and `unsafe { mem::zeroed() }` mints even a fully sealed resource. Cost: the boundary holds only under a checkable discipline — every resource is a struct with a private seal field; no `Default`/`Clone`/`Copy`/`Deserialize` or public `-> Self` fns on resources; no `pub enum` resources (wrap a private enum in a sealed struct); `#![forbid(unsafe_code)]` in all modelling crates. Each rule is mechanically greppable; a lint script is the practical mitigation. (EXP-08)

- **Boundary-violation compiler errors are modeller-friendly.** E0423 "cannot initialize a tuple struct which contains private fields / constructor is not visible here due to private fields" and the cfg/feature notes ("the item is gated behind the `test-support` feature") read naturally as "you may not create this resource here". Exception: E0639 for `#[non_exhaustive]` variants is jargon — another reason to avoid enum-based resources. (EXP-08)

## 4. Recommendation: **adopt**, with the following precise layout

Adopt the two-crate privacy boundary; it does exactly what R1 asks, at zero runtime and negligible compile-time cost, with errors a modeller can read. Precisely:

1. **Crates.** `model-lib` (resource types, boundary, processes, test support) and one or more downstream modelling crates. The hard boundary is the crate edge; downstream crates physically cannot construct resources.
2. **Module layout inside `model-lib`.** One module (or module subtree) per resource family, each containing: the sealed types, a nested `pub mod boundary` (suppliers/fill functions — the only production constructors), and a nested `#[cfg(feature = "test-support")] pub mod test_support` (fixtures). Nesting is mandatory: it is what makes the boundary compiler-enforced against the rest of `model-lib`. Re-export `boundary`/`test_support` at the crate root. Processes live outside the resource module tree; where a process must build a composite output, give the composite a `pub(crate)` combinator that only wraps values passed in by value (e.g. `FastenedAssembly::assemble`).
3. **Sealing rules for every resource type** (each rule greppable; consider a lint script):
   - a struct with at least one private field — use a zero-sized `(())` or `_seal: ()` field on otherwise-empty types;
   - never a `pub` unit struct, never a `pub enum` (wrap a private enum in a sealed struct);
   - no `Default`, `Clone`, `Copy`, or deserialization derives/impls; no public fn returning `Self` from plain data;
   - `#[must_use]` on every resource (feeds EXP-03's drop story);
   - `#![forbid(unsafe_code)]` in every crate of the project.
4. **Test helpers.** `#[cfg(test)]` modules for `model-lib`'s own unit tests only. For downstream tests, the `test-support` feature, enabled exclusively via `[dev-dependencies]` re-declaration of `model-lib`. Never enable it in `[dependencies]`; police with a grep in CI.
5. **CI must run `cargo build` (or `cargo check`) for every downstream crate in addition to `cargo test`**, because feature unification makes `cargo test` blind to test-support misuse in production code; and do not write trybuild tests about the feature boundary — trybuild inherits dev-dependency features and will report false success.
