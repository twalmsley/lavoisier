# FINDINGS

Findings log required by R14 of `instructions.md`. F-001…F-026 consolidated on 2026-09-30
from the `RESULTS.md` files of experiments EXP-01 through EXP-08; F-027…F-033 added the same
day from EXP-09 (`experiments/exp0N-*/RESULTS.md`); F-034 added 2026-10-02 during the
`model-core` build; F-035…F-039 added 2026-10-02 during the `pilot-workshop` build; F-036 resolved and F-040
added 2026-10-02 by the kernel-macro generics extension; F-041 added 2026-10-02 by the R16
implementation; F-042…F-052 added 2026-10-02 from the `RESULTS.md` files of experiments EXP-10
through EXP-12 (`experiments/exp1N-*/RESULTS.md`; those three ran against `model-core` by path,
with `trybuild` still the only external dev-dependency); F-046 extended 2026-10-02 by the
R17–R19 implementation; F-053…F-054 added 2026-10-03 by the CS-1 (pot of tea) build; F-015 extended 2026-10-03 by
the learning-materials build; F-055 added 2026-10-06 by the diagram-generator build; F-050 extended and F-055 point 3
resolved 2026-10-07 by the CS-2 build; F-041 and F-055 extended 2026-10-07 by the CS-3 build; F-056 added and F-001/F-010/F-011/F-055
extended 2026-10-07 by the CS-4 build; F-057 added and F-054 extended 2026-10-07 by the CS-5
build; F-055 gaps 1 and 7 (and the flow-naming WARN) resolved 2026-10-07 by the step-9
extraction work; F-058…F-059 added 2026-10-07 by the step-11 model-analysis build; F-060…F-064
added and F-001/F-037/F-040/F-044 extended 2026-10-08 from the `RESULTS.md` files of experiments
EXP-13 through EXP-15 (`experiments/exp1N-dsl-*/RESULTS.md`; EXP-13/14 ran against `model-core`
by path with `trybuild` still the only external dev-dependency; the EXP-14 and EXP-15 generators
are std-only); F-065…F-066 added 2026-10-08 by the CS-6 (bread batch) field test of the R22
spec→scaffold pipeline.
All experiments ran on the same toolchain: **`rustc 1.98.1 (48a229cea 2026-09-01) (Homebrew)`**,
stable channel, `cargo 1.98.1`, macOS (Darwin 24.6.0, Apple Silicon). Dependencies were limited
to `trybuild` as a dev-dependency.

Each entry records what could not be expressed (or only with complications), what it cost, and
the workaround adopted. Entries are ordered roughly by how much they constrain the project.
Evidence citations point at the experiment directories; the verbatim compiler output backing
each entry is in the cited `RESULTS.md`.

---

## F-001 — Compile-time conservation checks are post-monomorphization; `cargo check`, rust-analyzer and trybuild cannot see them

**What couldn't be expressed:** a conservation violation (R3: outputs must sum to the input)
as an error visible at type-check time from inside a generic function. Both stable in-function
techniques — an associated-const assert forced by use (`let _ = AssertSum::<IN, A, B>::OK;`)
and an inline `const { assert!(A + B == IN) }` (stable since 1.79) — make the violation a
genuine `E0080` compile error, but the error fires only when a fully concrete instantiation is
reached **during code generation**. Verified directly: compiling the violating program with
`--emit=metadata` (what `cargo check` does) succeeds; a full build fails.

**What it cost:**
- `cargo check` and editor diagnostics driven by it (rust-analyzer's default) report the
  violating program as fine; the mistake surfaces only on `cargo build`/`cargo test`.
- trybuild runs `cargo check` internally, so a trybuild compile-fail test for a conservation
  violation reports "Expected test case to fail to compile, but it succeeded" (see F-003).
- Dead generic code that is never instantiated is never checked at all.
- When the violating instantiation is in a downstream crate, the E0080 primary span lands in
  `library/core/src/panic.rs`, not the model library; the custom assert message and the
  "while instantiating `fn split::<2000, 1500, 600>`" note (pointing at the user's call site)
  remain, so the error is findable but noisier.
- The two techniques that would fire earlier do not compile on stable: a `const` item inside
  the generic fn (E0401: nested items can't see outer generics) and a
  `where If<{ A + B == IN }>: True` bound (needs `generic_const_exprs`).

**Workaround adopted:** gate CI and the documented workflow on `cargo build`/`cargo test`,
never on `cargo check` alone; write conservation compile-fail regressions as rustdoc
`compile_fail` doc-tests (rustdoc fully builds doc-tests) and keep the snippets minimal,
because stable rustdoc ignores the expected-error-code annotation; document prominently that
default editor diagnostics will not show conservation errors. A call-site macro that plants a
concrete `const _: () = assert!(…);` item (`split_grams!(stock, 2000 => 1500 + 500)`) moves
the check to check time and is trybuild-visible, at the cost of restating the input as a
literal; type-level (Peano) bounds also check at type-check time but do not scale (F-011).

**Evidence:** EXP-04 (`experiments/exp04-quantity-conservation/RESULTS.md`, technique table
and criterion 2; `tests/ui-postmono-not-caught-by-trybuild/`, `src/lib.rs` doc-tests).

**Extended (2026-10-07, CS-4 build):** two refinements at multi-crate scale. (1) Change impact
arrives in **per-crate waves**: cargo stops at the first failing crate, so a workspace-wide
blast radius is enumerated crate by crate, not in one list. (2) A **top-level
`const _: () = assert!(…)` item** (CS-4's batch-arithmetic check) IS visible to `cargo check`
— the post-monomorphization blindness applies to asserts inside *generic* functions, and
hoisting whole-model arithmetic into concrete const items recovers editor visibility.

**Extended (2026-10-08, EXP-15/EXP-14):** the same boundary, measured from the generation side.
A process emitted with **literal** const magnitudes (a non-generic fn) has its
`const { assert!(…) }` evaluated when the library itself is built: a conservation violation
fails plain `cargo build` of the crate with no instantiation and no test — the
post-monomorphization caveat is a cost of *generalized* (const-generic) processes, not of the
asserts per se (trade-off: a literal process serves exactly one worked instance). And for
notation-authored models the caveat is pre-emptible wholesale: every magnitude in a declarative
flow is a literal the EXP-14 compiler already tracks (it computes `draw_time`'s LEFT and the
turbofish decimals), so a notation-level `--check` pass can evaluate balances and overdraws
with notation spans at generation time, keeping the generated asserts as the Rust backstop
(F-061).

---

## F-002 — Silent resource loss is not fully closable at compile time (the affine-types gap, now quantified)

**What couldn't be expressed:** true linearity — "every resource must be consumed, checked by
the compiler". Of the eight drop/leak paths catalogued, three have **no compile-time detection
by any stable mechanism**: end-of-scope drop of a binding that was used at least once;
struct-pattern `..` silently dropping fields; and drop on an early-return path. Panic
unwinding evades everything (detecting it needs a `Drop` that panics during unwind, which
double-panics and SIGABRTs the whole test binary — worse than the leak). Separately, a process
that swallows a moved-in reusable resource (takes a `Drill`, doesn't return it) compiles
without complaint at its definition site; `#[must_use]` does not fire on moved-in parameters.
The caller's E0382 does trace the loss to the guilty function — but only if the flow tries to
reuse the resource; a flow that never needs it again never finds out.

**What it cost:** "nothing is lost" becomes a property of test coverage, not of the type
system. Every consumable resource type needs tripwire boilerplate (~10 lines: a `Drop` impl
that panics unless `thread::panicking()`, defused by an explicit `consume(self)` that
`mem::forget`s internally), and every legitimate disposal must go through an explicit
consume/Consumer call. Residual gaps even then: untested branches, unwind-path losses, and
`mem::forget`-equivalents (`ManuallyDrop::new`, `Box::leak`, `Box::into_raw`, raw-pointer
round-trips) that have no dedicated lint.

**Workaround adopted:** the layered regime of F-007/F-008 (complete lint set + tripwire
`Drop` + per-process tests), plus a signature-review convention: every reusable resource that
goes into a process must appear in its return type. Branch coverage *is* leak coverage, so R5
test discipline (every process branch tested, especially early returns and error paths) is
part of the conservation story. This confirms and bounds the known gap stated in
`instructions.md`: affine types plus conventions give compile-time protection for
discard-by-statement, `let _`, forget and explicit drop, test-time protection for everything
else except unwinding.

**Evidence:** EXP-03 (`experiments/exp03-drop-prevention/RESULTS.md`, the full leak-path ×
mechanism matrix; `tests/leak_paths.rs`, `clippy-demos/`, `src/bin/unwind_abort.rs`);
EXP-07 (`experiments/exp07-flows/RESULTS.md`, criterion C4,
`tests/compile_fail/process_swallows_drill.rs`).

---

## F-003 — trybuild cannot test post-monomorphization errors or feature-gated boundaries

**What couldn't be expressed:** the R4 instruction "compile-fail examples via trybuild" as a
universal mechanism. trybuild compiles its cases with `cargo check` and with the
dev-dependency feature set, which creates two blind spots:
1. A conservation violation inside a generic function (post-monomorphization, F-001) passes
   trybuild: "Expected test case to fail to compile, but it succeeded."
2. A compile-fail test asserting "production code can't call `test_support`" also wrongly
   succeeds at compiling, because trybuild ui tests inherit dev-dependency features
   (feature unification, F-004).

**What it cost:** trybuild results are actively misleading for these two classes unless the
check/build distinction is known. In both cases the *test* is wrong, not the boundary.

**Workaround adopted:** use trybuild only for pre-monomorphization errors (unit mismatches,
missing/wrong resource types, trait-bound violations, privacy violations) — there it works
well and pins `.stderr` files. For conservation violations, use rustdoc `compile_fail`
doc-tests or the call-site-macro variant (F-001). For the test-support feature boundary, the
only proof is a plain `cargo build`/`cargo check` of a real production target in CI; do not
write trybuild tests about it.

**Evidence:** EXP-04 (`experiments/exp04-quantity-conservation/RESULTS.md`, criterion 5,
`tests/ui-postmono-not-caught-by-trybuild/README`); EXP-08
(`experiments/exp08-privacy-boundary/RESULTS.md`, Complication B).

---

## F-004 — `#[cfg(test)]` helpers never serve downstream tests, and the `test-support` feature leaks into non-test code during `cargo test`

**What couldn't be expressed:** test-only resource constructors that are (a) visible to a
downstream crate's tests and (b) provably invisible to its production code, using `cargo test`
alone.
- `#[cfg(test)]` helpers in `model-lib` do not exist for `model-user` even in a dev/test build
  (a dependency is always compiled without `cfg(test)`); rustc's E0433 note says explicitly
  the item "was configured out". So R1's `#[cfg(test)]` option covers the defining crate's own
  tests only.
- The alternative — a `test-support` cargo feature enabled via a `[dev-dependencies]`
  re-declaration — works, but during `cargo test` Cargo unifies features across normal and dev
  dependencies, so the downstream crate's **production sources** are also compiled against the
  featured `model-lib`. Measured directly: a probe fixture call in production code passed
  `cargo test --no-run` and was rejected only by plain `cargo build`.

**What it cost:** `cargo test` alone cannot prove the privacy boundary holds in production
code. Residual risk: any crate anywhere in a production dependency graph that enables
`test-support` as a *normal* feature switches it on for everyone.

**Workaround adopted:** the `test-support` feature pattern, with three rules: it is only ever
enabled under `[dev-dependencies]`; CI runs a plain `cargo build` (or `cargo check`) of every
downstream crate in addition to `cargo test`; a one-line grep over `Cargo.toml`s polices the
feature's placement. (Note the CI consequence combines with F-001: CI needs *both* a full
build — for conservation errors — *and* a plain check/build of production targets — for the
feature boundary. `cargo build` satisfies both.)

**Evidence:** EXP-08 (`experiments/exp08-privacy-boundary/RESULTS.md`, criteria C2–C3,
`tests/ui/cfg_test_helper_invisible.rs`, verbatim probe output).

---

## F-005 — The private-constructor pattern is one attribute away from broken, and the compiler will not warn

**What couldn't be expressed:** "resources cannot be created from nothing" as a property the
compiler defends against the library author's own slips. Sealed types (private field, no
derives, no public ctor) showed **no safe-Rust hole** from downstream — but each of the
following, demonstrated by a passing test from the user crate, silently reopens the boundary:
`#[derive(Default)]` (a public constructor regardless of field privacy); all-`pub` fields;
a `pub` unit struct (its name is a value expression); a `pub enum` (every variant is a public
constructor, and variant fields cannot be private at all); `#[derive(Clone)]` (duplication —
violates conservation); any public fn or trait impl returning `Self` from plain data (the
shape of `serde::Deserialize`); and `unsafe { mem::zeroed::<Bolt>() }`, which mints even a
fully sealed resource, because privacy is a safe-Rust guarantee only.

**What it cost:** the boundary holds only under a checkable discipline, not by construction.

**Workaround adopted:** sealing rules, each mechanically greppable (a lint script is the
practical mitigation): every resource is a struct with at least one private field (a
zero-sized `(())` or `_seal: ()` on otherwise-empty types); never a `pub` unit struct or
`pub enum` resource (wrap a private enum in a sealed struct — `#[non_exhaustive]` on variants
blocks downstream construction but is fragile and its E0639 error is jargon); no `Default`/
`Clone`/`Copy`/deserialization derives or public data-to-`Self` fns on resources;
`#![forbid(unsafe_code)]` in every modelling crate (policy/CI — a library cannot impose it on
downstream crates). Boundary-violation errors are otherwise modeller-friendly: E0423
"constructor is not visible here due to private fields" reads naturally as "you may not create
this resource".

**Evidence:** EXP-08 (`experiments/exp08-privacy-boundary/RESULTS.md`, criterion C4 hole
table; `model-lib/src/holes.rs`, `model-user/tests/holes.rs`); corroborated by EXP-02
(E0451 constructing `Bolt` outside the boundary) and EXP-07 (`conjure_drilled_plate.rs`).

---

## F-006 — Field privacy stops at module boundaries, so the creation boundary must be a child module of the resource-defining module

**What couldn't be expressed:** "only the boundary module may create resources" enforced
against the rest of the defining crate, with boundary and resources in sibling modules. Rust
field privacy extends down the module tree only, so a sibling `boundary` module cannot touch
private fields — and making constructors `pub(crate)` instead reduces the rule to convention
inside the crate.

**What it cost:** resources, their boundary (suppliers/fill functions), and their
test-support fixtures must live in one module file as nested `pub mod`s, re-exported at the
crate root for ergonomics. Processes live outside the resource module tree; where a process
must build a composite output, the composite gets a `pub(crate)` combinator that only wraps
values passed in by value.

**Workaround adopted:** exactly that layout (`resources.rs` containing the sealed types plus
nested `pub mod boundary` and `#[cfg(feature = "test-support")] pub mod test_support`). With
it, the boundary is compiler-enforced even inside `model-lib`, at zero runtime and negligible
compile-time cost. The hard boundary remains the crate edge: downstream crates physically
cannot construct resources.

**Evidence:** EXP-08 (`experiments/exp08-privacy-boundary/RESULTS.md`, layout section and
recommendation §4).

---

## F-007 — The compiler's own fix-it suggestions recommend conservation violations; the lint set only works as a complete set, and shadow lints must be left out

**What couldn't be expressed:** a single lint or attribute that makes discarding a resource an
error. Worse, each mechanism's official escape hatch is the next leak path:
`deny(unused_must_use)`'s help text suggests `let _ = …` (a silent leak);
`deny(let_underscore_drop)`'s help text suggests `drop(…)` (another silent leak); and for a
process that swallows a resource, rustc suggests "consider changing this parameter type to
borrow instead" — precisely what R2 forbids — while never suggesting the real fix (return the
resource). Separately, `clippy::shadow_reuse` fires on the conservation-*correct* idiom
`let bolt = inspect(bolt);` that R2 makes ubiquitous, so the shadow lints cannot be adopted
even though `shadow_unrelated` would catch a real leak path.

**What it cost:** adopting a subset of the lints gives a false sense of safety; modellers must
be explicitly told to ignore the compiler's borrow/clone/`let _`/`drop()` suggestions.

**Workaround adopted:** the complete set, crate-wide and in CI under `-D warnings`:
`#![deny(unused_must_use)]` + `#![deny(let_underscore_drop)]` + `clippy::let_underscore_must_use`
+ `clippy::mem_forget` (both restriction lints, enabled explicitly) + `clippy::forget_non_drop`
(on by default) + `clippy::disallowed_methods` with a `clippy.toml` banning `std::mem::drop`,
`core::mem::drop`, `std::mem::ManuallyDrop::new` and `std::boxed::Box::leak`, each entry with
a `reason` string telling the modeller what to do instead (the most modeller-readable
diagnostic in the whole experiment). `#[must_use = "<type> is a conserved resource…"]` on
every resource type. Shadow lints **not** adopted; the tripwire (F-008) plus the on-by-default
`unused_variables` cover the shadowing leak instead. Never write `let _ = <resource>`.

**Evidence:** EXP-03 (`experiments/exp03-drop-prevention/RESULTS.md`, §2–§3 verbatim help
texts and matrix); EXP-07 (`process_swallows_drill.stderr`, the borrow suggestion).

**Extended (2026-10-02):** EXP-10 adds a fix-it family member: E0308 on a `Result` used as its
success bundle suggests `.expect("REASON")` — the panic path, itself a conservation violation
(and, with `Debug`-less bundles, one that does not even compile — F-047).

---

## F-008 — The tripwire `Drop` and `mem::forget` are exact complements; the tripwire reports the `Drop` impl, not the leak site

**What couldn't be expressed:** compile-time detection of used-then-dropped resources. The
panicking-`Drop` tripwire (guarded by `thread::panicking()`, defused by `consume(self)` which
`mem::forget`s internally) converts six of the eight leak paths into ordinary test failures —
every path except `mem::forget` itself (forget skips `Drop`; it is what `consume()` uses) and
panic unwinding (the guard stands down, deliberately: an unguarded tripwire double-panics and
aborts the process, exit 134/SIGABRT, burying the original failure). Meanwhile
`clippy::mem_forget` catches forget at compile time, but only fires for types *with* drop glue
— i.e. the tripwire arms the lint. The project needs exactly one commented
`#[allow(clippy::mem_forget)]` site per resource: inside `consume()`.

**What it cost:** ~10 lines of boilerplate per consumable resource type (worth
macro-generating); detection quality equals test coverage; and a fired tripwire panics at the
`Drop` impl's source line, **not the leak site** — the leak's location must be recovered from
the failing test's name or `RUST_BACKTRACE=1`. Fine at R5's one-process-per-test granularity,
weak for debugging large end-to-end flows.

**Workaround adopted:** tripwire on every consumable resource (in the real model, the R12
`Consumer` takes the `consume(self)` role); keep processes small and tests per-process so a
fired tripwire has a small search space.

**Evidence:** EXP-03 (`experiments/exp03-drop-prevention/RESULTS.md`, matrix rows 4 and 8,
`src/bin/unwind_abort.rs`, §3 tripwire output).

**Extended (2026-10-02):** EXP-10 measured the `..` path's timing: skipped fields are a partial
move that drops at the end of the destructured binding's scope, after every bound field, so a
tripwire on a forgotten field fires at the end of the match arm, not at the destructuring
statement (F-047).

---

## F-009 — Compiler diagnostics erase type aliases, never show decimal values, spill long types to side files, and duplicate bound errors

**What couldn't be expressed:** readable identity of type-level numbers in errors. A wrong
number is always caught at compile time (never wrong-but-compiles), but the diagnostic prints
the expanded structural type, never the alias: `N41` vs `N42` appears as two 40-plus-deep
`Succ` chains to count by hand, with a misleading primary line ("expected `Zero`, found
`Succ<Zero>`" — the diff after peeling common layers); the binary encoding prints compact but
binary-only `UInt` forms; Peano conservation errors show `Succ` chains with no numeric
`VALUE` anywhere. Derived numbers (results of `Pred`/`Add`, intermediate supplier states) have
no aliases at all. At capacity ~130, rustc stops printing types inline and writes them to
`target/…/long-type-….txt` side files; trait-style quantity instantiations do the same.
Minor extra noise: rustc emits the identical unsatisfied-bound E0277 twice at one generic
call site, and marker-style requirement violations emit one E0277 per missing characteristic.

**What it cost:** modellers must decode structural types by hand; there is no stable-Rust fix.

**Workaround adopted:** design around it — keep capacities small; wrap numbers in domain types
whose outer name survives in errors (`BoltBox<…>` at least says what is being counted); and
prefer trait-not-implemented error shapes over type-mismatch shapes: "`Pred`/`Supplier` is not
implemented for `Zero`/`BoltBox<Nil>`" reads almost like "the box is empty", especially with
`#[diagnostic::on_unimplemented]` (F-015). Parameter-style catalogue errors
(`expected Bolt<SizeM8, Steel, L15>, found Bolt<SizeM6, Brass, L15>`) are the most readable of
any style tested and need no mitigation.

**Evidence:** EXP-01 (`experiments/exp01-type-level-numbers/RESULTS.md`, errors 1, 2, 5);
EXP-02 (long-type file at capacity 130; duplicate E0277); EXP-04 (error (e), Peano
conservation); EXP-05 (per-characteristic duplication, §2.1).

---

## F-010 — The recursion limit caps type-level capacity, per crate; the default (128) allows ~100 but not 130

**What couldn't be expressed:** arbitrarily large type-level numbers/lists under the default
compiler configuration. Using a Peano number (evaluating `VALUE`, solving `Add`/`Lt`, or
building/consuming a `Cons` list) needs `#![recursion_limit]` ≈ **N + 3**; measured minimums
were 503 for N=500 and 1003 for N=1000. Supplier machinery at capacity 100 compiles at the
default limit; capacity 130 fails with `E0275: overflow evaluating the requirement`. Merely
*defining* aliases is lazy and free at any size — the cost lands where a number is used. The
attribute is per **crate**, so every downstream crate that touches deep numbers must set it,
not just the library defining them. The E0275 `help:` text names the exact attribute but only
suggests the next doubling.

**What it cost:** a crate-level attribute as standing convention, and an error ("recursion
limit") that a modeller has no reason to connect with "the box is too big".

**Workaround adopted:** set `#![recursion_limit = "2048"]` (or at least "256") as project
convention in every crate using type-level capacities; document E0275 as "the box is too big
for the current limit" in the modeller's troubleshooting notes. Compile time is *not* the
constraint at R12's contemplated sizes: full fill/supply×100/consume×100 monomorphization
builds in ~2.6 s.

**Evidence:** EXP-01 (`experiments/exp01-type-level-numbers/RESULTS.md`, measurement table,
error 4); EXP-02 (`experiments/exp02-supplier-consumer/RESULTS.md`, criterion 1c, error 6).

**Extended (2026-10-07, CS-4 build):** measured minima at scale — cs4-stores needs
`recursion_limit` ≈ 151 (a 100-item supplier *plus its fill machinery* now exceeds the default
128), cs4-line ≈ 143, and the integration-test crate itself needed the attribute (it overflowed
at default merely naming the end-state alias). The mandated 2048 leaves ≈ 13× headroom; no
SIGBUS, no cliff.

---

## F-011 — Type-level (unary) naturals are the only route to check-time conservation, but magnitudes are unusable; compile time goes superlinear past N≈500

**What couldn't be expressed:** check-time-visible conservation over realistic quantity
magnitudes. With Peano numbers and `A: Add<B, Sum = In>`, conservation is an ordinary trait
bound: violations fire at type-check, are visible to `cargo check`/rust-analyzer/trybuild
(avoiding all of F-001), and `combine`'s output type is computed by the compiler. But 2000 g
is a 2000-deep `Succ` chain: unusable magnitude, no custom message possible, no numeric value
in the error. Peano compile time is flat to N≈500 and superlinear after (~1 s at N=100,
1.6 s at N=500, 4.6 s at N=1000 for one `VALUE` use plus one addition); a model with many
distinct large numbers multiplies that.

**What it cost:** quantities (R3/R7) must stay const-generic and accept F-001's post-mono
checking; type-level numbers are reserved for small counts (capacities, R12).

**Workaround adopted:** const-generic quantities with `const`-assert conservation for
magnitudes; Peano for capacities ≤ a few hundred. Revisit only if the binary encoding (F-012)
is built out with comparison.

**Evidence:** EXP-04 (`experiments/exp04-quantity-conservation/RESULTS.md`, technique 6,
error (e)); EXP-01 (measurement table, F3).

**Extended (2026-10-07, CS-4 build):** composite-depth cost measured — the full 25-cycle
batch unroll (≈ 100 threaded steps, a 100-bolt box, a 75-item bin) compiles in ~1.0 s;
whole-workspace cold gate 44.9 s → 45.5 s (+1.4 %) when the two CS-4 crates joined. The flat
region holds at real composite depth.

---

## F-012 — The binary type-level encoding scales but costs ~3× the machinery: decrement needs a `Trim` normalisation pass, and less-than was not achieved

**What couldn't be expressed simply:** an alternative number encoding without canonical-form
traps. Binary naturals stay ~1 s to compile and within the default recursion limit up to
N=1000 (needs only bits(N)+3), but: decrement naively yields leading-zero forms — right
`VALUE`, wrong *type*, failing type-equality against the canonical form — so extra `Trim` +
`TrimmedCons` traits and a wrapping `Dec` were required (~25 trait impls total vs 7 for
Peano); internal helper names (`DecRaw`) leak into user-facing errors; and type-level
less-than was not implemented — with the least-significant bit outermost it needs a length
comparison plus an MSB-first lexicographic pass (typenum-style `Cmp` machinery), while Peano
`Lt` is two impls that worked first try.

**What it cost / workaround adopted:** Peano is adopted for R12 capacities — the decisive
argument being that supplier contents are a `Cons` list whose type is *already* O(N) deep, so
a binary counter alongside it saves no depth, no compile time and no recursion limit, and
Peano's structure mirrors the list one-to-one (`Succ` ↔ `Cons`). The binary encoding stays on
the shelf, revisited only if standalone numbers ≥ ~1000 with heavy arithmetic become
necessary.

**Evidence:** EXP-01 (`experiments/exp01-type-level-numbers/RESULTS.md`, F4, F5,
`tests/compile_fail/dec_raw_not_canonical.rs`, recommendation).

---

## F-013 — Stable `macro_rules!` cannot synthesize identifiers, so alias tables and marker-struct catalogues keep one hand-written line per name

**What couldn't be expressed:** a single macro invocation that *creates* names — the 1001
aliases `N0..N1000`, or the cross product of combination structs with concatenated names
(`M8SteelBolt15mm`) from per-dimension value lists. Stable `macro_rules!` has no identifier
concatenation; the `paste` crate is a forbidden dependency and `macro_metavar_expr_concat` is
nightly.

**What it cost:** every generated name must literally exist in source. The number-alias
invocation lists (1001 lines) are written by a small generator script whose output is treated
as source; the marker-trait bolt catalogue keeps 1 line per combination even with a macro, so
its LOC stays O(product of dimension sizes) — 24 lines for 4×2×3, doubling with each new
dimension.

**Workaround adopted:** for numbers, a non-recursive alias macro plus the generator script
(`exp01 tools/gen_aliases.py`), shipped with the library. For catalogues, prefer the
type-parameter encoding, which needs no macro at all and scales O(sum) (see F-019 and the R6
amendment proposal).

**Evidence:** EXP-01 (`experiments/exp01-type-level-numbers/RESULTS.md`, F1); EXP-05
(`experiments/exp05-characteristics-encoding/RESULTS.md`, F-EXP05-1, LOC table).

---

## F-014 — Using one supplier N times inside a generic process needs a recursive helper trait; chained bounds don't scale

**What couldn't be expressed cleanly:** "take four bolts from this supplier" with plain
bounds. Hand-written chained bounds work but cost one where-clause per item
(`S: Supplier, S::Next: Supplier, <S::Next as Supplier>::Next: Supplier, …`) with linearly
deepening projections — readable at 4 only via `NextK<S>` type aliases, unreadable beyond a
handful.

**What it cost:** one extra trait per "repeat this boundary action N times" pattern.

**Workaround adopted:** a recursive `SupplyN<N>` trait — base case at `Zero`, recursive case
at `Succ<N>` bounded by `S: Supplier, S::Next: SupplyN<N>`; the two impls don't overlap
because the `N` parameter differs, so coherence is satisfied with no tricks. Any N becomes
**one** where-clause (`S: SupplyN<N4, Taken = FourBolts>`), returning the items as a Cons-list
of real values plus the depleted supplier; the `Taken =` equality bound doubles as the "items
must be bolts" requirement. A matching `ConsumeList` serves consumers. Asking for more than
the supplier holds is a readable compile error (F-015).

**Evidence:** EXP-02 (`experiments/exp02-supplier-consumer/RESULTS.md`, criterion 1a,
`src/lib.rs::fasten_four_chained` / `fasten_four_supplyn`).

---

## F-015 — `#[diagnostic::on_unimplemented]` turns capacity errors into plain English, but only on the trait-bound path; direct method calls bypass it

**What couldn't be expressed:** good errors on *every* misuse path. With the attribute (stable
since 1.78) on `Supplier`/`Consumer`, passing an empty box to a generic process yields
`` `BoltBox<Nil>` cannot supply anything: it is empty `` — exactly the message a modeller
needs, at zero cost. But a direct method call on the exhausted value (`empty.supply()`) takes
the E0599 "no method named `supply`" path, which bypasses the attribute entirely, never says
"empty", and suggests an unrelated similarly-named method.

**What it cost:** nothing at compile time; the error quality depends on which path a mistake
takes.

**Workaround adopted:** put `on_unimplemented` (with messages phrased for modellers: "it is
empty", "no space left") on every boundary trait, and route model code through generic
processes — which R12 requires anyway — so mistakes hit the good path.

**Evidence:** EXP-02 (`experiments/exp02-supplier-consumer/RESULTS.md`, criterion 1b, errors
1–5).

**Extended (2026-10-03, learning-materials build):** when several `on_unimplemented`-carrying
obligations fail together, rustc surfaces the **deepest** one: a some-but-not-enough `SupplyN`
request reports the `Supplier` message on the computed empty state, not `SupplyN`'s own
"cannot supply this many items" (which leads only when the type is not a supplier at all).
Also noted: the tripwire panic's type name differs in shape between macro forms (bare name
with magnitude from the non-generic container form; fully-qualified `type_name` from the
generic forms).

---

## F-016 — A consumer with only a space parameter cannot keep what it consumes; `WasteBag` needs a contents parameter too

**What couldn't be expressed:** R12's "a consumer keeps the objects it has consumed" with the
sketched `WasteBag<Space>` — the consumed items' types accumulate as it consumes, so a
remaining-space-only type has nowhere to put them.

**What it cost:** the consumer's full type mentions everything it ever consumed, making for
long error messages.

**Workaround adopted:** `WasteBag<Space, Contents = Nil>`, where consuming maps
`WasteBag<Succ<S>, C> → WasteBag<S, Cons<In, C>>`. The default type parameter keeps
construction ergonomic (`new_waste_bag::<N100>()`); the alias
`type FullWasteBag<C> = WasteBag<Zero, C>` helps readability.

**Evidence:** EXP-02 (`experiments/exp02-supplier-consumer/RESULTS.md`, deviation note and
F5).

---

## F-017 — A default type parameter makes adding a catalogue dimension non-breaking, but silently narrows existing blanket impls

**What couldn't be expressed safely:** extending `Bolt<S, M, L>` to `Bolt<S, M, L, P>`
without either breaking every existing spelling (no default) or corrupting the bridge (with a
default). With `P = Pitch125`, all existing `Bolt<S, M, L>` spellings keep compiling — but an
existing bridging impl written as `impl<M, L> M8 for Bolt<SizeM8, M, L>` silently becomes an
impl for the *default pitch only*; bolts of every other pitch lose their `M8` marker with no
warning or error. This is quiet model corruption, exactly the class of error the project
exists to prevent.

**What it cost:** an audit obligation on every defaulted-parameter addition.

**Workaround adopted:** whenever a defaulted parameter is added, rewrite every blanket impl
mentioning the type with an explicit parameter (`impl<M, L, P> M8 for Bolt<SizeM8, M, L, P>`);
a grep for `for Bolt<` makes the audit mechanical.

**Evidence:** EXP-05 (`experiments/exp05-characteristics-encoding/RESULTS.md`, criterion 1c,
F-EXP05-3, `src/pitch_demo.rs`).

---

## F-018 — Unbounded type parameters accept transposed arguments silently; kind traits are required

**What couldn't be expressed without extra machinery:** that the slots of
`Bolt<Size, Material, Length>` only accept values of the right kind. With unbounded
parameters, `Bolt<Steel, SizeM8, L15>` compiles cleanly and the mistake surfaces far away, if
ever.

**What it cost:** one "kind" trait per dimension (`Size`, `Material`, `Length`) plus one
one-line impl per characteristic value, with the bounds repeated on the struct and its impls.

**Workaround adopted:** kind traits as bounds on the struct definition, turning transposition
into a clear construction-site error ("the trait `Size` is not implemented for `Steel`" —
reads almost like the modelling mistake itself).

**Evidence:** EXP-05 (`experiments/exp05-characteristics-encoding/RESULTS.md`, F-EXP05-2,
`tests/compile_fail/transposed_params.rs`).

---

## F-019 — Bridging the two characteristic encodings works cleanly but only one way; requirements must always be trait bounds

**What couldn't be expressed:** the reverse bridge. One-line blanket impls
(`impl<M, L> M8 for Bolt<SizeM8, M, L> {}`, O(number of characteristic values), no coherence
problems while traits and type share a crate) give every parameterized `Bolt<…>` the marker
traits its parameters imply, so parameterized types satisfy marker-style requirement traits
unchanged. But a marker struct can never satisfy a signature demanding the nominal type
`Bolt<SizeM8, Steel, L15>` — a nominal type is not a bound.

**What it cost / workaround adopted:** requirements (R10) must always be written as
marker/requirement-trait bounds, never as concrete parameterized types in process signatures;
concrete `Bolt<…>` types are implementation detail. Then both encodings satisfy every
requirement. Supporting observation on errors: a wrong bolt against a concrete parameter gives
a single diff-like E0308 showing all wrong characteristics at once (the best error of EXP-05);
against a marker bound, one E0277 per missing characteristic with a truncated "types that do
implement it" list — both readable, the parameter one better.

**Evidence:** EXP-05 (`experiments/exp05-characteristics-encoding/RESULTS.md`, criterion 1e,
F-EXP05-4, F-EXP05-5, `src/bridge.rs`).

---

## F-020 — The R10 blanket impl makes "which types satisfy REQ-NNN" invisible to grep; a compile-checked `Satisfies:` tag closes the gap

**What couldn't be expressed:** the type→requirement link as a greppable string. Because a
requirement trait is satisfied via `impl<T: A + B> ReqNNN for T {}`, no source line ever
states `impl ReqNNN for ConcreteType`; satisfaction is a fact only the trait solver knows.
Bound *uses* (`fn f<B: ReqNNN>`) remain directly greppable.

**What it cost:** "satisfied by" cannot be extracted from the R10 pattern alone.

**Workaround adopted:** an explicit `/// Satisfies: REQ-NNN` doc tag on each satisfying type
or process, kept honest by a compile-checked assertion next to it
(`const fn assert_reqNNN<T: ReqNNN>() {}` + `const _: () = assert_reqNNN::<TheType>();`), so a
stale tag is a compile error whose message names the requirement trait, the offending type and
the exact missing characteristic. Zero compile cost measured.

**Evidence:** EXP-06 (`experiments/exp06-traceability/RESULTS.md`, criterion C2, §2a).

---

## F-021 — Grep-based traceability is reliable only under a strict tag discipline; the discipline itself is mechanically lintable, but grep has no scope awareness

**What worked only with complications:** R10's mechanical extraction. `trace.sh` (~150 lines
of dependency-free sh/awk) produced a correct report — zero false positives or negatives —
across all edge cases (multiple IDs per tag line, whitespace variants, IDs in `//`/`////`/
block comments/string literals/doc prose, an unverified requirement, an unknown ID), but only
because tags follow an exact convention: case-sensitive `Verifies:`/`Satisfies:` in
three-slash `///` doc comments directly above the item, IDs as `REQ-` + exactly three digits.
Deviations are silent misses in principle. Also: grep attributes a bound in a multi-line
`where` clause to the where-clause line rather than the function name — cosmetic, locations
exact, names sometimes missing.

**Workaround adopted:** the script itself lints the convention — it warns on near-miss tags
(e.g. lowercase `verifies:`) and on unknown REQ ids, and exits nonzero on any warning
(including a requirement with no verifying test), so it doubles as a CI gate; keep requirement
bounds on the same line as the `fn` name (rustfmt does this at default width).

**Evidence:** EXP-06 (`experiments/exp06-traceability/RESULTS.md`, criterion C1 table, §2
verbatim report, §4 convention).

---

## F-022 — Output quantities cannot be computed on stable; `combine` callers must state the total

**What couldn't be expressed:** `fn combine(...) -> Grams<{A + B}>` — computing an output
magnitude from const generics needs `generic_const_exprs`; a where-clause on `A + B == IN` is
rejected ("generic parameters may not be used in const operations"); a `const` item inside the
generic fn cannot see its generics (E0401).

**What it cost:** the caller of `combine` must state the expected total, which the assert then
checks — one more place to type a number, though not a soundness hole (a wrong total is the
E0080 of F-001). Caller ergonomics are otherwise good: const parameters infer from binding
annotations, through chains, and via partial turbofish; with no annotation the failure is a
clear E0284 with a usable suggestion.

**Workaround adopted:** callers state totals; the conservation assert (inline `const` block or
`AssertSum` struct — identical behaviour; write a custom assert message, it leads the error
output) verifies them. The type-level style is the only variant where the compiler computes
the total, and it doesn't scale (F-011).

**Evidence:** EXP-04 (`experiments/exp04-quantity-conservation/RESULTS.md`, criteria 3 and 6,
error (f)).

---

## F-023 — "Product never produced" is a compile error only because every processing state is its own type

**What couldn't be expressed automatically:** flow completeness over a single reused type.
`fasten(person, p1, p2, bolts)` with undrilled plates fails with
"expected `DrilledPlate`, found `Plate`" — the most modeller-readable error of EXP-07 — but
only because `Plate` and `DrilledPlate` are distinct types; with one `Plate` type the broken
flow would compile. The backstop also held: the missing product cannot be conjured
(E0451, private field).

**What it cost:** one struct per processing state plus a conserving `pub(crate)` conversion
per process step.

**Workaround adopted (convention):** never reuse one type for two processing states.

**Evidence:** EXP-07 (`experiments/exp07-flows/RESULTS.md`, criterion C3,
`tests/compile_fail/product_never_produced.rs`, `conjure_drilled_plate.rs`).

---

## F-024 — Aggregating reusable resources (`Workshop`) saves boilerplate but over-claims resources and coarsens errors

**What worked only with complications:** threading ergonomics. Loose threading names each
reusable resource twice per call (2×N identifiers for N resources) and leans on the shadowing
idiom `let (person, drill, d1) = drill_holes(person, drill, p1);` — which is itself an EXP-03
leak path (shadowing a *different* still-live resource drops it silently). An aggregated
`Workshop { person, drill }` makes every call constant-size (`let (ws, d1) = …(ws, p1)`), but:
a process that doesn't need the drill still locks it, deleting concurrency R9 wants to allow —
a semantic distortion, not just style; contention errors degrade to "use of moved value: `ws`"
without naming the contended resource; and the destructure/rebuild boilerplate just moves into
the wrapper.

**Workaround adopted (convention):** loose threading is the default; aggregate only sets of
resources genuinely used together by *every* process that takes the aggregate.

**Evidence:** EXP-07 (`experiments/exp07-flows/RESULTS.md`, criterion C5).

---

## F-025 — Move semantics deliver R2's exclusivity with precise errors, but in borrow-checker vocabulary

**What worked, with a translation cost:** double-booking a reusable resource is E0382 "use of
moved value", marking "value moved here" on the process that holds the resource and "used here
after move" on the one trying to claim it — exactly right, but a modeller must learn that
"moved" means "handed to another process and not yet returned". Keeping resources `Clone`-free
also removes the compiler's "consider cloning" escape hatch, directly improving the error. R9
falls out for free: two valid orderings of the same flow both type-check, and only data
dependencies constrain sequence.

**Workaround adopted:** no code workaround needed; write a short error-reading guide for
modellers ("use of moved value" = "resource already busy / lost upstream"; ignore borrow/clone
fix-its per F-007).

**Evidence:** EXP-07 (`experiments/exp07-flows/RESULTS.md`, criteria C1–C2,
`tests/compile_fail/person_used_twice.rs`).

---

## F-026 — A public boundary fill function must name its recursive machinery; a sealed trait keeps it inside the boundary

**What couldn't be expressed directly:** `full_box<N>()` exposed publicly without letting
outside code implement the recursive fill trait in its where-clause (which would break "no
resources from nothing").

**What it cost:** minor boilerplate.

**Workaround adopted:** the classic sealed-trait pattern (`pub trait FillSealed:
sealed::Sealed`, with `Sealed` in a private module implemented only for bolt lists). No hole
found: constructing a `Bolt` outside the boundary fails with E0451 (trybuild-verified).

**Evidence:** EXP-02 (`experiments/exp02-supplier-consumer/RESULTS.md`, F6).

## F-027 — R15's continuous-resource pattern composes end to end on stable; overdraw diagnostics print real decimal magnitudes

**What couldn't be expressed:** nothing new — the combination R15 describes (sealed
const-generic container + caller-stated-remainder draw + budget draw-down + two-dimension
balancing + unbounded `Next = Self` sinks) composes with no new blockers. Overdraw — drawing
6000 g from a 5000 g bottle — is a genuine compile error via the conservation assert, and
inherits F-001 unchanged (`cargo check`/rust-analyzer/trybuild blind; rustdoc `compile_fail`
doc-tests are the regression vehicle; CI gates on build/test).

**What it cost:** nothing beyond F-001; clean build 1.06 s.

**Workaround adopted:** none needed. Notably, unlike Peano capacities (F-009), every
diagnostic prints magnitudes as decimals (`draw_gas::<6000, 0, 5000>`, `GasBottle<0>`) with
the caller's line in the instantiation note — the worst aspect of F-009 is absent for
continuous resources.

**Evidence:** EXP-09 (`experiments/exp09-continuous-resources/RESULTS.md` §1–2;
`src/lib.rs` doc-tests; `postmono-demo/` check-vs-build transcript).

---

## F-028 — `Supplier` is discrete-only: a finite continuous container cannot implement any supplier-shaped trait on stable

**What couldn't be expressed:** `impl SupplierOf<Gas<TAKE>> for GasBottle<FULL>` — the next
state needs the caller-stated remainder, but a trait impl has nowhere to receive it (E0207:
unconstrained const parameter), and computing it (`GasBottle<{FULL - TAKE}>`) needs
`generic_const_exprs` (nightly).

**What it cost:** fixed-packet supply through the discrete `Supplier` works for an
*unbounded* source but forces one packet size per boundary object, `combine` chains with
restated totals (F-022), and non-multiple amounts are unreachable.

**Workaround adopted:** continuous boundary sources are draw-style boundary processes
(`draw_air<const TAKE>(atm) -> (Air<TAKE>, Atmosphere)`); `Supplier` is documented as
discrete-only. `Consumer<In>` impls remain legal and useful on unbounded sinks — the consumed
amount travels in `In`, so there is no remainder problem.

**Evidence:** EXP-09 (`tests/ui/finite_container_*.rs` with pinned `.stderr`;
`tests/integration.rs` probes (a)–(c)).

---

## F-029 — `Next = Self` boundary objects coexist with all R12 machinery; an unbounded consumer necessarily discards its intake

**What couldn't be expressed:** nothing — multiple `Consumer<In>` impls on one boundary
object (exhaust + heat on `Atmosphere`), `on_unimplemented` messages, and the recursive
repeated-use traits all work: `SupplyN`/`ConsumeList` recurse on the count/list, not the
state, so a self-renewing `Next` causes no non-termination or coherence issue. rustc's E0277
help even lists the impls that do exist, documenting the boundary object's interface in the
error.

**What it cost:** an unbounded consumer necessarily *discards* what it consumes — the licence
R15 confines to the boundary — deviating from R12's "a consumer keeps the objects it has
consumed".

**Workaround adopted:** record the deviation as the one sanctioned exception, in R12.

**Evidence:** EXP-09 (`tests/integration.rs`;
`tests/ui/atmosphere_cannot_consume_labour.stderr`).

---

## F-030 — A budget parameter infects every signature its resource passes through, and the modeller maintains the running balance by hand

**What couldn't be expressed:** automatic remainder computation for a budgeted reusable
resource. Every process threading `Person<BUDGET_MS>` is generic over the budget even if it
spends nothing; each spending call restates three numbers (`draw_time::<3000, 5000, 8000>`),
with the previous remainder re-derived by the modeller as the next budget.

**What it cost:** verbosity and hand re-derivation — not correctness: a wrong balance is the
E0080 of F-027, so the arithmetic is compiler-checked.

**Workaround adopted:** none available on stable (computing remainders needs
`generic_const_exprs`). R15's "model a time budget only where that time is genuinely being
accounted for" is the right mitigation and is reaffirmed.

**Evidence:** EXP-09 (`src/lib.rs::walk_to_station`;
`tests/integration.rs::time_budget_draws_down_across_a_flow`).

---

## F-031 — Continuous-resource processes must live inside the privacy boundary

**What couldn't be expressed:** continuous processes as ordinary outside-the-module
processes. Unlike discrete processes (which only move sealed objects and can live outside
the resource module tree, F-006), R15 processes (`draw`, `combine`, `burn`-style balancers)
mint new quantity-bearing values (`Gas<TAKE>` from a bottle), which the privacy boundary
forbids to outside code.

**What it cost:** the F-006 layout rule needs a carve-out.

**Workaround adopted:** each resource family's module contains the sealed types plus
`boundary`, `test_support`, and its conserving *continuous* processes (a `processes` child
module) — or outside processes are handed sealed mint combinators.

**Evidence:** EXP-09 (`src/lib.rs::model::processes` and its module-level comment).

---

## F-032 — The tripwire `Drop` is the only layer that catches an abandoned empty container, and Drop types can't be destructured

**What couldn't be expressed:** compile-time detection of a silently abandoned
used-then-dropped container (`GasBottle<0>` left to fall out of scope). The layered regime
lands exactly as F-002/F-008 predict for continuous resources: whole-result discard →
`must_use` (compile time, sees through tuples); named-but-never-used waste →
`unused_variables` (an error only under CI `-D warnings`); used-then-dropped → tripwire
panic at test time only (`resource leak: GasBottle<0> dropped without being consumed`).

**What it cost:** because a type with `Drop` cannot be moved out of by destructuring,
F-008's "single allowed `mem::forget` site" per resource becomes a sealed `defuse(self)`
helper that every conserving transform and consumer inside the boundary must call.

**Workaround adopted:** macro-generate `mint`/`defuse`/tripwire together with each resource
type.

**Evidence:** EXP-09 (`tests/integration.rs` `should_panic` cases;
`postmono-demo/src/bin/{result_discarded,heat_unused}.rs`).

---

## F-033 — Multiple conservation dimensions per process compose cleanly

**What couldn't be expressed:** nothing — `burn` carries independent mass and energy `const`
asserts; each violation reports its own custom message, and when both are violated both
E0080s are emitted. Nothing in the pattern limits the number of balanced dimensions per
process.

**What it cost / workaround:** none.

**Evidence:** EXP-09 (`src/lib.rs::burn`; RESULTS.md §2 (B)).

**Extended (2026-10-02):** EXP-10 extends the pattern from dimensions to `Result` branches: a
fallible process carries one assert per arm and both fire at every instantiation (F-045).

---

## F-034 — A contents-keeping consumer without a decreasing space parameter diverges trait resolution; at the mandated recursion limit this is a compiler crash, not an error

**What couldn't be expressed:** a consumer that keeps its contents but has no decreasing
type-level space parameter (`impl<C> Consumer<Token> for Bin<C> { type Next = Bin<Cons<Token,
C>>; }`) used with `ConsumeList` — resolution explores `Bin<C>`, `Bin<Cons<Token, C>>`, …
without bound.

**What it cost:** at the default `recursion_limit` (128) the divergence is a graceful E0275.
At the project-mandated `#![recursion_limit = "2048"]` (F-010), rustc 1.98.1 deterministically
crashes with SIGBUS (stack overflow, no diagnostic at all). Minimal repro confirmed both ways.

**Workaround adopted:** the `WasteBag<Space, Contents>` shape (F-016) and `Next = Self`
unbounded sinks (F-029) are not just modelling conventions — a decreasing parameter (or a
self-renewing `Next`) is what keeps trait resolution terminating. Treat "every
contents-keeping consumer has a decreasing space parameter" as a hard rule, and know that the
raised recursion limit converts violations from a diagnosable error into a compiler crash.

**Evidence:** `model-core` build (LIMITATION comment in `model/model-core/src/boundary.rs`,
tests module; minimal repro described there).

---

## F-035 — Common conserved outputs need a production-legal boundary sink in the library itself

**What couldn't be expressed (as first built):** a downstream *production* flow that draws a
person's time had no way to dispose of the `Labour` it produces: `Labour`'s `defuse` is
`pub(crate)` to model-core, and the only `Consumer<Labour>` shipped was the test-support-gated
`fixtures::TestSink` — so production code could satisfy the lint regime only by never drawing
time.

**What it cost:** discovered by the pilot; its flows initially had to live in tests.

**Workaround adopted (fixed in model-core):** every library-provided conserved output ships
with a production-legal placeholder sink at the boundary — `common::TimeLedger`
(`Next = Self`, `/// Placeholder:`-marked, created by `boundary::new_time_ledger`). Rule of
thumb: whoever mints a tripwired type must also ship at least one production consumer for it,
or downstream crates cannot account for it at all.

**Evidence:** `pilot-workshop` build report; fix in `model/model-core/src/common.rs`
(`TimeLedger`, `labour_has_a_production_legal_sink`).

---

## F-036 — The resource kernel macros generate non-generic types only

**What couldn't be expressed:** defining a *parameterized* sealed resource
(`Bolt<Size, Material, Length>`, `Assembly<B, const PLATE_G: u64>`) through
`consumable_resource!`/`container_resource!` — `macro_rules!` patterns in the kernel take a
plain ident, not a generics list.

**What it cost:** the pilot's catalogue and assembly types needed hand-written sealing, and
`Assembly` a hand-rolled tripwire/`defuse`, duplicating what the macros exist to guarantee.

**Workaround adopted:** hand-write parameterized resources against the sealing-rules
checklist (R1) and keep them beside a macro-built sibling for comparison; extending the kernel
macros to accept generics is future work.

**RESOLVED (2026-10-02):** the kernel macros now accept generic parameters (type params with
one path bound each, then const params) and, for `consumable_resource!`, sealed payload
fields; the pilot's `Bolt` and `Assembly` are macro-built and their hand-written
sealing/tripwire duplication is deleted. The viable stable parsing pattern is a shared
tt-muncher with trailing-comma accumulators and callback finish macros — shaped repetitions
are impossible because `macro_rules!` reports a local ambiguity between an ident fragment and
the `const` keyword as alternatives at one position (NT-vs-token is never disambiguated by
lookahead). Residual grammar costs: types before consts, one bound per param (compound bounds
go behind a named kind/requirement trait per R6/R10), no lifetimes/defaults/where clauses,
and `container_resource!` reserves the magnitude parameter name `V`. See F-040 for the
payload-field semantics.

**Evidence:** `model/model-core/src/resource.rs` (kernel, grammar rustdoc,
`__mc_parse_generics!`); `pilot-workshop/src/catalogue.rs`, `pilot-workshop/src/resources.rs`.

---

## F-037 — A `Satisfies:` tag inside a kernel-macro invocation is silently invisible to trace.sh

**What couldn't be expressed:** tagging a macro-defined resource type directly — the doc
comment sits inside the macro invocation, so the line after the tag is not an item keyword
and trace.sh's grep drops it without a warning.

**What it cost:** a silently missing "satisfied by" row (the dangerous direction: the lint
gate stays green).

**Workaround adopted:** put `Satisfies:` tags on a type alias (`FasteningBolt`) or on the
process that uses the bound (`fasten`), never inside a macro invocation; the `satisfies!`
compile-checked assertion stays beside the macro invocation, so the *truth* half of the
R10 tag+assert pair is unaffected.

**Evidence:** `pilot-workshop/src/catalogue.rs` (`FasteningBolt`), build report.

**Extended (2026-10-08, EXP-13):** the silent direction scales beyond tags. A sugared macro
form that emits its doc line as an attribute (`#[doc = …]`) loses the requirement definition to
trace.sh — loud only when some tag references the lost id (`WARNING: unknown requirement`,
exit 1); a sugared requirement nobody tags vanishes with no warning at all. Generated
`#[doc = concat!("Placeholder: ", …)]` lines are likewise invisible to the R12/R20 placeholder
inventory. And the linter's `E-TAG-MACRO` gate (this finding's dedicated check) recognizes only
`model_core::`-prefixed invocations, so a tag inside any *other* macro passes unflagged. At
whole-model scale this failure class becomes F-060.

---

## F-038 — Grep-discipline costs of R10 in a real model (extends F-021)

**What couldn't be expressed:** tidy formatting. R10's rule 4 (requirement bounds on the
`fn`-name line) forces `fasten`'s full signature onto one ~290-character line; and trace.sh's
bound-use pass counts continuation lines of multi-line `use` statements as bound uses.

**What it cost:** cosmetic — one very long line per bounded process, and single-line imports
required near requirement names.

**Workaround adopted:** accept the long signature lines; keep imports of requirement traits
on one line. Both are lintable by the same pass that polices the tags.

**Evidence:** `pilot-workshop/src/resources.rs` (`fasten`), build report.

---

## F-039 — Tripwired contents kept by a bin need a sealed recursive disposal path (F-008 × F-016 interaction)

**What couldn't be expressed:** letting a contents-keeping consumer (the `SwarfBin`) reach
its own legitimate end-of-model exit directly — dropping the bin drops its kept tripwired
swarf, and every tripwire fires.

**What it cost:** one more sealed recursive mechanism per contents-keeping consumer.

**Workaround adopted:** a boundary disposal process (`dispose_bin`) that recursively defuses
the kept contents inside the privacy boundary before the bin itself exits — the bin's
consumer plays the sanctioned-`mem::forget` role for everything it kept.

**Evidence:** `pilot-workshop/src/resources.rs` (`dispose_bin`,
`swarf_bin_keeps_swarf_until_disposal`).

---

## F-040 — Macro-held payload contents are consumption-only and must be untripwired

**What couldn't be expressed:** general held contents in macro-built resources. The kernel's
payload fields (F-036 resolution) support exactly one ownership shape: contents enter via
`mint(field, …)` (the conserving combinator) and leave only when the whole resource is
defused at a consumer — `Assembly`'s semantics. Contents that must come back out (a bin
emptied at disposal, F-039) do not fit and stay hand-written consumers.

**What it cost:** held contents must be **untripwired**: `defuse`'s single sanctioned
`mem::forget` silently defuses any tripwired payload along with the wrapper, which would erase
the payload's own leak protection. The new `no_tripwire` marker covers kept item types
(e.g. `Bolt`), omitting both `Drop` and `defuse` (a `forget` on a `Drop`-less type is itself a
leak path, `clippy::forget_non_drop`). Also, the generic tripwire message now reports
`core::any::type_name::<Self>()` — full paths with decimal const magnitudes (F-027) instead of
the old bare name.

**Workaround adopted:** the documented rule — tripwired wrappers hold untripwired contents;
extractable contents are a `Consumer` with a sealed disposal path (F-039).

**Evidence:** `model/model-core/src/resource.rs` (payload grammar rustdoc and tests);
`pilot-workshop/src/resources.rs` (`Assembly`, `SwarfBin` contrast).

**Extended (2026-10-08, EXP-15):** the tripwire/`no_tripwire` choice is mechanically derivable
from the boundary tables — an item minted into a §4 supplier is untripwired held contents; a
process-created output is tripwired — and the spec generator's rule reproduced CS-1's
hand-made choices exactly (`DryTeabag` untripwired, `SpentTeabag` tripwired). Candidate
template/linter rule rather than per-model judgement; until stated somewhere, it is one of the
F-062 silent inferences.

---

## F-041 — R16 implementation notes: the sanctioned `Vec`, records vs resources, and a refinement of F-029

**What couldn't be expressed:** nothing — the agreed R16 design survived the compiler
unchanged. Three notes from its implementation (which doubled as its validation):

- **The sanctioned `Vec` (R13/R14 logging duty):** `History`'s entries are a runtime
  `Vec<Entry>` because record count is flow-dependent; this is the R16-sanctioned use,
  commented at the field per R13. The partial order is kept faithfully by
  `Entry::Join(Vec<Entry>, Vec<Entry>)` — a series-parallel shape, claiming no interleaving
  between merged branches.
- **Records are not resources:** `Event` and `Entry` carry `Debug`/`PartialEq`/`Eq` derives
  and public fields, which the R1 sealing rules forbid on resources — legal here precisely
  because they are value-level records that cannot mint resources; the record's integrity
  rests on `History`'s privacy (only `record`/`merge` can append), not on the record types'.
  The conservation-safe consumption path is the sealed `Permit` token:
  `Recordable::into_record` is implementable downstream but callable only by `History`'s
  machinery, so the trait cannot be used to vanish a resource outside a consumer.
- **F-029 refined:** `History` is a `Next = Self` unbounded boundary consumer that does
  *not* discard its intake outright — it discards the resource (defusing it in its
  sanctioned consumer role) but keeps its record. F-029's "necessarily discards" applies to
  the resource, not to information about it.

**What it cost / workaround:** none.

**Evidence:** `model/model-core/src/history.rs`; pilot per-branch merge assertions in
`model/pilot-workshop/tests/flows.rs`.

**Extended (2026-10-07, CS-3 build):** the merge machinery passed its first real two-actor
exercise with no gaps: `Entry::Join`'s public shape made the "no invented interleaving"
assertions direct (top-level `[Entry::Join(a, b)]`, every entry inside each branch a flat
in-order `Event`), and downstream `Recordable` impls let a stranded resource's disposal be
recorded as a branch event — exercised during the build and then replaced at the 2026-10-07
review by a Drain-consumer process (CS-3's P8), the `Recordable` route having worked as
designed. No model-core changes were needed.

---

## F-042 — A fallible process needs one runtime-valued outcome token; selecting the outcome with types collapses fallibility

**What couldn't be expressed:** compiler-forced failure handling with the outcome chosen by
types. Two sealed token types selected by the flow are fully expressible on stable — as two
monomorphic process variants, or as one generic process whose output is a const-generic GAT
projection (`type Out<const …: u64, …>`) — but the outcome then becomes part of the flow's
static text: there is no `Result`, no runtime branch, and no site anywhere is forced to handle
the arm the flow did not pick, so "fallible process" collapses back into two infallible
processes. A token-generic flow is no escape (its result is the opaque projection `O::Out<…>`,
which it can only pass along), and each impl carries only its own branch's conservation assert,
so the unchosen branch's split is never checked at any call site.

**What it cost:** a design rule rather than machinery (~70 hand-written lines per token type —
see the `outcome_token!` proposal).

**Workaround adopted:** **one sealed token type per fallible-process kind**, runtime-valued: a
private enum wrapped in a sealed struct (R1's "never a pub enum resource"), tripwired, minted
only by boundary constructors (`/// Placeholder:` until calibrated) and test-support fixtures,
unreadable by flows — the only way to learn the outcome is to run the process and handle the
`Result` — consumed by exactly one process, with a boundary exit function for untried tokens.
Processes stay deterministic (the variability is the token's *value*, injected at the boundary,
R12 spirit), and both outcomes of one process are testable by injecting either token.

**Evidence:** EXP-10 (`experiments/exp10-failure-modes/RESULTS.md` §1 C2;
`src/model.rs::DrillOutcome`, `src/two_token.rs`, `tests/ui/outcome_cannot_be_minted.stderr`).

---

## F-043 — Qualification markers go on a sealed wrapper around `Person`, never on `Person`; a parallel person type forks the whole time-accounting family

**What couldn't be expressed:** a qualification as a marker on `model_core::common::Person`
itself. Coherence permits `impl<const MS: u64> CertifiedDriller for Person<MS>` (local trait),
but `Person` has no qualification slot, so the impl certifies **every person in the model at
once** — quiet model corruption of the F-017 class, with no compiler warning possible. The
alternative, a parallel downstream person type (`reusable_resource!`, 2 lines to declare), holds
no `Person` and so forks the entire time-accounting family: its own draw process, its own labour
type (`Labour`'s mint is private to model-core), its own `Consumer` sink, and its own
`Recordable` impl before it can reach `History` — ~40 duplicated lines of re-reviewable
conservation code per person type, measured.

**What it cost:** a hand-sealed wrapper `Operator<Q: Qualification, const BUDGET_MS: u64>`
holding the `Person` as a private field (~34 lines once per model — `reusable_resource!` takes
no held contents, F-040). Two diagnostic footnotes: the wrapper's delegating draw puts the
overdraw E0080's "while instantiating" note (F-027) on its internal `draw_time` call, one hop
from the modeller's flow line (the decimal magnitudes still identify the call); and with consts
before a type parameter, callers must write a trailing `_` in the turbofish
(`operator_draw_time::<2000, 3000, 5000, _>(op)` — E0107 otherwise).

**Workaround adopted:** the wrapper, with the rule "qualification markers attach by one-line
blanket impls **over the budget** (`impl<const MS: u64> CertifiedDriller for
Operator<DrillingCert, MS>`), never on `Person`" — the blanket-impl-over-budgets pattern worked
first try on both a hand-sealed and a macro-built type. Certify/decertify are conserving
boundary processes (wrap/unwrap the same person, budget intact); the budget draw opens the
wrapper, delegates to `draw_time`, re-wraps — the R15 overdraw error, F-030's hand-maintained
balance, and the `Labour`→`History` path (F-035/R16) are inherited, not duplicated. The wrapper
is proposed for promotion into model-core as `common::Qualified` (candidate R18). `Satisfies:`
tags again went on aliases, per F-037.

**Evidence:** EXP-11 (`experiments/exp11-qualifications/RESULTS.md` §1 C4, §3;
`src/resources.rs::{Operator, Driller, Effort}`, `tests/flows.rs`).

---

## F-044 — `on_unimplemented` on a marker is ignored when the marker fails as a supertrait obligation; the attribute must go on the requirement trait, and its message is fixed per trait, not per impl

**What couldn't be expressed:** one modeller-phrased message per characteristic that surfaces
everywhere. When a process bounds `O: Req001…` (the R10 pattern) and the blanket impl's
supertrait obligation fails, rustc surfaces only the **root** obligation's
`#[diagnostic::on_unimplemented]`; the failing *marker's* own attribute (`CertifiedDriller`,
`Fitted`) is never shown in that position — the error falls back to "the trait bound
`MachineGuard: Req002FittedDrillGuard` is not satisfied". The pilot's `wrong_bolt.stderr` has
the same shape (its `M8` message is likewise unused). A related granularity limit (EXP-12): the
attribute's message is fixed at the **trait**, not per impl — model-core's `Consumer` note,
phrased for waste bags, appears verbatim when the failing consumer is a vendor.

**What it cost:** one more attribute per requirement trait, a convention to know, and a
retrofit of the pilot's existing requirement traits.

**Workaround adopted:** every `requirement!` invocation carries its own one-line
`on_unimplemented` phrased as the requirement and naming the REQ id (the macro's meta slot
already passes attributes through). Result, trybuild-pinned: the E0277's top line states the
REQ ("this person may not drill: `Person<5000>` is not a certified drilling operator
(REQ-001)"), the note says how to qualify, and rustc's impl list enumerates the types that
would qualify — the best requirement error measured in the project. Keep marker-level
attributes too, for direct marker bounds.

**Evidence:** EXP-11 (`tests/ui/uncertified_person.stderr` vs `tests/ui/unfitted_guard.stderr`;
`src/requirements.rs`); corroborated by `model/pilot-workshop/tests/ui/wrong_bolt.stderr`;
EXP-12 (`tests/ui/purchase_wrong_price.stderr`).

**Clarification (2026-10-06):** the certified-driller message appears as "(REQ-001)" here
(EXP-11's crate-local numbering) and as "(REQ-004)" in the pilot and the technical deck — both
are real pinned outputs from different crates, not a contradiction. An F-053-adjacent reminder
that requirement ids are per-workspace, and experiment crates number independently.

**Extended (2026-10-08, EXP-13/EXP-14):** the requirement-trait message survives both viable
DSL layers unchanged. Through a token-preserving macro front the E0277 is effectively identical
to hand-written — message, label and note intact, the "required by a bound" note even citing
the readable DSL `process fn … K: Req002BoilingWater` line (the feared `on_unimplemented` loss
did not occur). Through generation it survives verbatim: the message/label/note travel as
notation attributes into `requirement!`'s meta slot, and the rendered error equals the best
requirement error measured in the project.

---

## F-045 — Per-branch conservation is one const assert per branch, both checked at every instantiation; the cost is doubled const-parameter load

**What worked:** a fallible process carries an independent `const { assert!(…) }` per `Result`
arm (success split, failure split), each with a branch-naming message, and **both fire at every
instantiation** (post-monomorphization, F-001 unchanged; decimal magnitudes and the caller's
line in the instantiation note, F-027) — so a call site that only ever realises the success
branch is still rejected for an unbalanced failure branch. The model proves both arms conserve
everywhere. (Extends F-033 from dimensions to branches.)

**What it cost:** the caller states **both** branches' splits — eight const parameters on one
process (`drill_fallible::<1000, 4000, 5000, 450, 440, 10, 430, 20>`), roughly doubling the
F-022/F-030 load. Not a correctness risk: a wrong number is the E0080. Drawing shared spends
(time) before the branch keeps the reusable resource's return type equal in both arms.

**Workaround adopted:** accept the load; violation regressions are rustdoc `compile_fail`
doc-tests per the R4 policy (verified: `cargo check --tests` passes the violation,
`cargo build --tests` rejects it).

**Evidence:** EXP-10 (`src/model.rs::drill_fallible`; RESULTS §2a).

---

## F-046 — Converging after a fallible step requires equal types in both arms; `unused_must_use` sees through tuples but not `Option`, so one-arm outputs need named `#[must_use]` groupings

**What couldn't be expressed:** a single return type for a flow whose arms produce different
resources or different magnitudes. Reusable resources converge only when the failure arm
restores the same state at the same magnitude (repair-on-the-spot; equal time spent in both
arms); paths that spend different amounts (a retried flow: 1000 vs 2000 ms) have no common type
at all. One-arm products can come back as `Option<resource>`, but — measured —
`deny(unused_must_use)` fires per **tuple element** with each resource's own R1 message and
does **not** look inside `Option<…>`: an Option-wrapped resource keeps only tripwire protection
against discard.

**What it cost:** per-flow outcome types instead of tuples containing `Option`s.

**Workaround adopted:** converge what can be converged (repair in the failure arm); return
per-path outcomes as a named `#[must_use]` grouping — one pub struct per arm, or a pub enum
with one variant per flow path. **Groupings are not resources**: building one requires already
holding the sealed resources, so a grouping cannot mint, and the R1 sealing rules (including
"never a pub enum") bind resources, not groupings. Outcome bundles deliberately derive no
`Debug` (F-047).

**Evidence:** EXP-10 (`src/flows.rs`; RESULTS §2f).

**Extended (2026-10-02, R17–R19 implementation):** convergence across fallible arms has a
further constraint with contents-keeping consumers (F-016): the consumer's type records what
it has kept, so both arms must feed it **identical item types and magnitudes** for the flow to
converge — the pilot's `SwarfBin` converges only because both arms yield 20 g of swarf, and
across a retry flow's unequal paths it cannot converge at all and travels inside each
`RetryOutcome` variant. Unbounded (`Next = Self`) sinks have no such constraint.

---

## F-047 — `Debug`-less outcome bundles make `.unwrap()`/`.expect()` a compile error; the remaining panic path is F-002's hole, demonstrated for the `Result` shape

**What worked (unplanned):** `Result::unwrap`/`expect` require `E: Debug`; outcome bundles hold
sealed resources (which implement no `Debug`) and derive none, so the panicking shortcut past
the failure arm **does not compile** (E0277 "doesn't implement `Debug`") — keep `Debug` off
outcome bundles deliberately. Everything between the ends of the leak surface is caught:
discard and `let _ =` at compile time by the lint pair (whose fix-its are F-007 verbatim, plus
the new `.expect("REASON")` member), the `Result` used as its Ok bundle at type-check time
(E0308), a failure state continuing the success flow at type-check time (E0308, or E0277 with
`on_unimplemented` on multi-impl sinks — F-023 extended to failure states), and
bound-but-unmatched results / forgetful match arms at test time by the tripwires (F-008,
including the `..` partial-move timing).

**What it cost / residual:** the `Debug` error is phrased as a formatting problem and needs an
error-reading-guide entry ("you may not panic past the failure arm: match and account for both
bundles"). The runtime unwrap-equivalent — an explicit panic while holding a bundle — remains
F-002's hole: unwinding stands every tripwire down, and the bundle's five resources vanished
with no abort and no leak report.

**Workaround adopted:** no `Debug` on any outcome bundle (a candidate-R17 convention) plus the
guide entries.

**Evidence:** EXP-10 (`tests/ui/unwrap_needs_debug.stderr`,
`tests/ui/result_is_not_the_bundle.stderr`,
`tests/leaks.rs::panic_while_holding_the_failure_bundle_leaks_silently`; RESULTS §3 matrix).

---

## F-048 — A process that draws a budget inside itself must take the concrete type and loses the REQ-phrased error; keep draws as separate adjacent processes

**What couldn't be expressed:** a process simultaneously (a) generic over "any certified
operator" and (b) drawing the operator's time budget down inside itself. The budget change is a
const-parameter change, which a generic `O: Req001…` bound cannot express (the F-028/E0207
remainder problem in different clothes). Taking the concrete `Qualified<DrillingCert, BUDGET>`
works, but a wrong operator there is an E0308 type mismatch, not the REQ-phrased E0277 of
F-044.

**What it cost:** a composition convention rather than machinery.

**Workaround adopted:** default style — qualification-checked processes are generic with
requirement bounds and do **not** draw budgets; the draw is its own adjacent process in the
flow (R9/R15-idiomatic anyway). A process that must account its own time takes the concrete
type and restates its requirement as a trivially-true where-clause for greppability (R10).

**Evidence:** EXP-11 (`src/resources.rs::{drill_plate, drill_plate_timed}`; both styles
exercised in `tests/flows.rs`).

---

## F-049 — A requirement sentence spanning several participants decomposes into one R10 trait per constrained parameter

**What couldn't be expressed:** "drilling requires a certified operator **and** a fitted guard"
as a single requirement trait — a trait bound constrains one type, and no type is both the
operator and the guard (a composite aggregate would over-claim, F-024).

**What it cost:** two REQ ids for one sentence of intent, both bound on the same one-line
process signature (R10 rule 4).

**Workaround adopted:** accept the decomposition; trace.sh then reports the one process under
both ids, which reads correctly in the traceability table. The safety half additionally relies
on one type per state (R9/F-023): `Fitted` is implemented only by `FittedGuard`, so "guard
present but not fitted" (E0277) and "no guard at all" (E0061, with inference pulling the plate
into the guard slot) are refused distinctly.

**Evidence:** EXP-11 (`src/requirements.rs`; `tests/ui/unfitted_guard.rs`,
`tests/ui/missing_guard.rs`; trace.sh green in RESULTS §1 C5).

---

## F-050 — Bounded rework is bounded by provisioning; early success returns reserves that must be re-accounted

**What worked:** under strict conservation a retry consumes provisioned reserves (a second
blank, a second outcome token, a repair kit), so the rework bound **is** the resources passed
in — unbounded retry is inexpressible without unbounded inputs, which is the honest statement
of real rework. Retry paths spend different amounts, so a retried flow returns a `#[must_use]`
outcome enum, one variant per path (F-046). The cost surfaces symmetrically: on first-try
success the unused reserves come back and must be re-accounted at the boundary (stores take
back the blank and the kit; the untried token leaves through its boundary exit) —
over-provisioning becomes an explicit, typed cost. Failure's labour is recorded in `History`
like any other consumption (R16).

**What it cost / workaround:** none beyond R5 discipline — every path tested (three tests for
a one-retry flow).

**Evidence:** EXP-10 (`src/flows.rs::drill_with_one_retry`, `tests/flows.rs`).

**Extended (2026-10-07, CS-2 build):** provisioning bounds more than retry counts — it also
polices *when* contingency resources may be acquired. A pessimistic early purchase (buying the
spare before the second failure is known) is statically inexpressible as a complete flow: on
the success arm the unused spare's only consumer is already claimed by the patched tube, a
dead end. Contingency purchases therefore sit on the failure arm by construction.

---

## F-051 — Money needs no new kernel machinery: one dimension per currency, and an exact-price `Consumer` impl turns wrong payments into type-check-time errors that name the right price

**What couldn't be expressed:** nothing — EXP-12 added **zero** model-core machinery. A
currency is one downstream `impl Unit` line (the kind trait is open); `container_resource!` +
`draw_process!` fit cash and accounts verbatim (`draw_funds: Account => Money`), so overspending
is the standard R15 overdraw E0080 (F-001/F-027/F-030 unchanged); a vendor is one boundary
object implementing both `Consumer<money>` and `Supplier` (goods), `Next = Self` (F-029);
purchase is an ordinary conserving process with change-giving as an R3 split, and the whole
conservation regime covers money unchanged (abandoned change trips the tripwire naming
`Money<150>`). There is deliberately no shared "money" unit: cross-currency sums are
unrepresentable (E0308), like adding grams to millimetres.

**Design result:** implement `Consumer` **only at the vendor's exact price** (a concrete
`Money<350>`, stated once as a named const). Wrong amount and wrong currency then fail at
**type-check time** — editor- and trybuild-visible, unlike the conservation asserts — and the
error names the correct price: a single impl makes inference report `expected Money<350>,
found Money<300>`, and through a priced generic bound the E0277 help lists the implemented
impl (F-029's observation, now serving prices). Cost: one impl per (vendor, price point).

**What it cost:** minor noise only: a violated assert inside a composed process (`purchase` →
`split_money`) emits a second E0080 whose instantiation note points inside the library, not at
the user's line — an echo to learn to skip, kin to F-009's duplicates; and F-044's per-trait
message granularity shows here as waste-bag phrasing on a vendor error.

**Workaround adopted:** none needed; the conventions become candidate R19 plus one R7 base-unit
table row.

**Evidence:** EXP-12 (`experiments/exp12-money/RESULTS.md` §1–§2; `tests/ui/*.rs` with pinned
`.stderr`; `postmono-demo/`).

---

## F-052 — Currency exchange is value-equivalence at a stated integer rate, confined to the boundary; silent rounding is unrepresentable

**What couldn't be expressed:** exchange as a conserving in-model process — correctly so. An
exchange destroys an amount in one currency dimension and mints the equivalent in another,
which R1 forbids inside the model; it is therefore a **boundary process** (the R15/F-031
licence, like drawing from the atmosphere), threading a placeholder boundary object (`Bureau`),
with the rate stated once as integer consts and every exchange const-asserted
`OUT * DEN == IN * NUM` (E0080 on violation, F-001 visibility).

**What it cost / gained:** the assert is exact multiplication — no division anywhere — so
nothing ever rounds silently: an amount with no exact exchange at the rate has **no `OUT` that
compiles** (333 pence at 117/100 rejects both 389 and 390 euro cents). The working idiom is
split-then-exchange: split off the largest exchangeable sub-amount (R3) and the remainder stays
conserved in the original currency; the modeller computes that sub-amount by hand (the
F-022/F-030 cost, unchanged). A model that wants a rounding loss or spread must model it as an
explicit fee output — a feature, not a limitation.

**Workaround adopted:** none; documented as the candidate-R19 exchange convention.

**Evidence:** EXP-12 (`src/lib.rs::exchange_gbp_to_eur` and its `compile_fail` doc-test;
`postmono-demo/src/bin/inexact_exchange.rs`; RESULTS §2 (e)).

---

## F-053 — trace.sh's requirement-id namespace is workspace-global: a second crate reusing REQ-001 stays silently green while the report merges and shadows

**What couldn't be expressed:** per-model requirement numbering. CS-1's agreed spec assigned
REQ-001..REQ-004, but the pilot already owns REQ-001..REQ-005 in the same workspace:
implementing the spec literally left the CI gate **green** while the traceability report
silently merged CS-1's tags and tests under the pilot's requirements and shadowed CS-1's
definitions (file-sort-order dependent) — the dangerous green-but-wrong direction, like F-037.

**What it cost:** requirement ids must currently be allocated workspace-uniquely at
specification time; CS-1 shipped as REQ-006..REQ-009 with the spec mapping documented at the
definitions.

**Workaround adopted:** continue the workspace sequence per case study and record the
spec↔code id mapping in the crate. Fix candidates for the project: scope trace.sh's ids per
crate, or make SPEC_TEMPLATE allocate from the workspace sequence.

**Evidence:** `model/cs1-pot-of-tea/src/requirements.rs` (mapping note); CS-1 build report.

---

## F-054 — A requirement-bounded process can consume a state-changing resource and keep REQ-phrased errors: associated-const magnitudes plus a permit-gated extraction

**What couldn't be expressed (previously, F-048):** style-A (generic, requirement-bounded)
processes were thought unable to consume resources whose magnitudes they must assert over,
degrading their errors to E0308 via style-B concrete types.

**What it cost / the pattern:** when the state change lands on a **fixed output type**, the
magnitudes go on the sealed characteristic trait as associated consts (R6), usable inside
inline `const { assert!(…) }` blocks, and the resource is opened by a **permit-gated
conserving extraction** on that trait (the R16 `Permit` pattern). Sealing the characteristic
is mandatory: an open trait would let outside impls smuggle a fake "boiling water" past the
bound or vanish resources through a free extraction (F-026). CS-1's `pour_and_brew` keeps
REQ-phrased E0277s for all four requirements this way.

**Workaround adopted:** none needed — this is a positive pattern, narrowing F-048: style-B is
only required when the *returned* type's const parameters must vary with the input's.

**Evidence:** `model/cs1-pot-of-tea/src/characteristics.rs` (`Boiling`, `BrewPermit`),
`src/resources.rs` (`pour_and_brew`).

**Extended (2026-10-07, CS-5 build):** permit-gated extractions are **same-crate only** — a
permit type is unconstructible downstream, so a crate consuming another crate's sealed
resource cannot extract from it. The working cross-crate shapes: consume by **keeping** (the
resource as untripwired payload inside the product, F-040), and the R10 split that goes with
it — the sealed characteristic (with its magnitude consts) lives beside the type in the
owning crate, while the requirement trait lives with whoever states the requirement.

---

## F-055 — Machine-reading the models: five gaps in the grep discipline (extends F-021/F-038)

**What couldn't be expressed:** building the diagram generator (`tools/diagram-gen`) against
the R10 grep discipline showed the discipline covers human-and-grep traceability but is not
yet sufficient for *tools* that must recover the full model structure from source:

1. **A generic `Supplier` impl hides what a supplier supplies** (`type Item = H`): "what does
   this box hold" has no greppable single line — the generator recovers it from the sealed
   fill machinery (`Fill for Cons<DryTeabag, T>`), which is incidental, not contractual.
2. **Plain (non-requirement) bounds live in `where` clauses.** R10 rule 4 pins only
   requirement bounds to the `fn`-name line, so one-line signature parsing is insufficient
   (`purchase`); tools must parse `where` clauses — a candidate extension to the discipline.
3. **REQ→type resolution via `satisfies!` is unique today but many-to-many in principle:** a
   second satisfying type per requirement would make bound-to-type resolution ambiguous; the
   generator picks the first, deterministically.
4. **Path-qualified return types** (`fn new_kettle() -> super::Kettle`) complicate line
   parsing; bare return-type names would keep it trivial.
5. **Kernel-helper semantics are hard-coded coupling:** `draw_time`'s TAKE/LEFT/FULL turbofish
   order and the `send_to`/`record` roles had to be built into the tool as a fixed table —
   acceptable for a fixed kernel, but any kernel change silently breaks the tooling.

**What it cost:** heuristics plus in-file WARN comments where parsing is not certain.

**Workaround adopted:** conservative parsing that warns rather than guesses; the gaps are
candidate convention extensions to consider before the process-docgen step (PLAN step 9),
which will lean on the same extraction.

**Evidence:** `tools/diagram-gen/src/{scan,flow}.rs`; the WARN comments in
`docs/diagrams/pilot-workshop/*.md`.

**Point 3 RESOLVED (2026-10-07, CS-2 build):** the predicted ambiguity became a real, silent
error — CS-2's multi-type REQ-012 made the top-level diagram route the spare tube to "caller"
instead of into `refit_and_inflate`, with no warning. Fixed in `tools/diagram-gen`: a
requirement bound now resolves to **one input edge per satisfying type**, and the remaining
single-pick contexts (sink/boundary-object positions) WARN instead of silently taking the
first. trace.sh itself handled the many-to-many case correctly from the start.

**Extended (2026-10-07, CS-3 build):** a sixth gap — R9's statement-order freedom can hide
flow structure from tools: a fallible call whose `Result` is stored in a binding and matched
later is skipped by the flow tracer as an "assertion-only match". Convention adopted: `match`
directly on the fallible call expression. Also noted: a multi-type *non-requirement* bound
(CS-3's sealed two-type order slot) is omitted from the top-level graph with a WARN — the
conservative F-055 behaviour working as designed.
**Extended (2026-10-07, CS-4 build):** a seventh gap — the generator scans **per crate**, so
cross-crate flows are invisible: cs4-line's top-level diagram collapsed to 2 nodes (its types
are defined upstream in cs4-stores) and the traced batch flow WARNs on every stores-defined
callee. Multi-crate models (the R1 subsystem pattern) need the generator to resolve across a
dependency, or a merged-workspace mode — queue for step 9.

**Gaps 1 and 7 RESOLVED, flow discovery generalized (2026-10-07, step-9 build):** the shared
scanner now loads workspace path-dependencies transitively and resolves upstream types,
processes and sinks (gap 7: all 24 unknown-callee WARNs died; cs4/cs5 diagrams come out whole
with cross-crate edges, single-crate output unchanged), recovers supplier items through
concrete `Item`s, `Cons` heads, fill machinery or full-state aliases (gap 1), and traces every
composite flow and ≥2-process integration test rather than one magic test name. Stderr WARNs
fell 42 → 6. Still open, by design or pending: the multi-type non-REQ bound (conservative
omission), gap 6's convention (match on the call), a downstream-implemented trait bound seen
from the upstream crate, the F-056 quantum clock not recognized as an actor, recursion
internals shown as one step, and associated-const asserts restated symbolically only.

---

## F-056 — A const budget cannot descend through type-level recursion; the quantum-clock encoding

**What couldn't be expressed:** threading `Person<const BUDGET_MS>` through a recursive batch
trait with the budget descending per cycle — `BUDGET_MS − 150_000` in the recursive impl's
associated types needs `generic_const_exprs` (nightly), and introducing the decremented value
as an inferred impl const is E0207 (the F-028 shape again, now on the consumer side of
recursion).

**What it cost:** CS-4's operator is a **type-level quantum clock** instead:
`Operator<Q: Nat>` holding time as 150 quanta of 30 000 ms, decremented structurally
(`Succ<Q> → Q`) per fixed-size draw, each draw minting a conserved `Effort<MS>` in real
milliseconds recorded to the History. Costs: draws come in fixed quanta (fine for a
fixed-cadence batch; wrong for ad-hoc draws), the ~40-line F-043 parallel-labour fork
(model-core's `Labour::mint` is crate-private), and overdraw errors become trait-resolution
failures at check time rather than the R15 E0080 (arguably an upgrade — editor-visible).

**Workaround adopted:** the quantum clock for recursion-threaded budgets; plain
`Person<BUDGET>` with caller-stated remainders everywhere else. Candidate convention: models
choose per actor — ad-hoc draws (R15 style) or recursion-compatible quanta — and say which.

**Evidence:** `model/cs4-line/src/batch.rs` (`Operator`, `BuildBatch`),
`case-studies/cs4-batch-run/CHANGE-IMPACT.md` §6.

---

## F-057 — Cross-crate money: the holder wraps, the currency owner mints

**What couldn't be expressed:** a downstream crate holding or moving another crate's sealed
currency without new machinery. Two precedented moves were needed:

- **The holder wraps:** the works' `Account<P>` holds the supply crate's sealed `Money<P>` by
  value as payload, and its draws/deposits are compositions of supply's public conserving
  `split`/`combine`. The balance const parameter and the held cash are the same number by
  construction, so they cannot drift.
- **The currency owner ships generic boundary entries:** downstream boundary objects (the
  works' float, the customer's payment) cannot mint sealed cash, so a currency-owning crate
  must export parameterized boundary in/out functions (`gbp_enters/leaves_the_model::<P>`) —
  the same licence as model-core's `quantity::boundary`, now a stated obligation of owning a
  currency (extends F-035's "whoever mints ships a sink" to "…and the boundary entries").

**What it cost:** nothing beyond the pattern; a type-level revenue counter for the vendor is
inexpressible on stable (the F-028/E0207 const-arithmetic shape), so boundary organisations'
takings stay arithmetic-only (F-029).

**Evidence:** `model/cs5-works/src/` (`Account`), `model/cs5-supply/src/` (boundary fns);
CS-5 build report.

---

## F-058 — The one ungated sealing rule was the one broken: `#[must_use]` on hand-written contents-keeping bins

**What couldn't be expressed:** nothing — this is a process finding. R1's sealing rules held
across all ten crates on their first mechanical audit, with exactly one systematic exception:
every contents-keeping bin written before CS-5 (`FoodWasteBin`, `FinishedGoods`, `SwarfBin`)
lacked `#[must_use]`, while CS-5's `Bin` carried it. A bin is a consumer and a consumer is a
resource (R12); without the attribute, whole-value discard of an **empty** bin was caught by
nothing (the kept contents' tripwires cover only a non-empty one, F-032).

**What it cost:** three latent layer-1 gaps that nine CI steps, 206 tests and five case-study
reviews never surfaced — R1's own text said "enforce with a lint script", and `must_use` was
the one rule no grep gated until step 11's linter (E-SEAL-MUSTUSE) ran.

**Workaround adopted:** the three structs fixed (one attribute each) the day the linter found
them; the linter's ERROR-KNOWN mechanism exists for any future true positive — named in the
tool's `KNOWN` list with justification, listed first, non-fatal, never silently retuned.

**Evidence:** `docs/analysis/` first-run reports; the fix commit; `tools/diagram-gen/src/lint.rs`.

---

## F-059 — Sink-coverage analysis must resolve through bounds (extends F-055)

**What couldn't be expressed:** a name-level answer to "does every tripwired type have a
production consumer?" (the F-035 audit). The R10/R12 bounds-first discipline hides literal
types: a kettle is consumed as `K: Req006…`, bolts as `Taken = FourOf<B>`, a drink via a
sealed slot-trait impl, labour via `impl Recordable` (which model-core documents as "History
may consume this").

**What it cost:** a naive name-level grep reported 13 orphaned types; resolving consumption
through requirement satisfaction, supplier `Taken` lists, local trait impls and `Recordable`
impls found **zero** real orphans. Any F-035 audit — human or tool — that does not follow the
bounds reports false positives.

**Workaround adopted:** step 11's I-NO-SINK check resolves through the four bound paths above
(INFO-graded: downstream crates outside the workspace may legitimately be the consumer).

**Evidence:** `tools/diagram-gen/src/lint.rs` (sink resolution); `docs/analysis/README.md`.

---

## F-060 — A whole-model macro front-end is invisible to the R20/R21 extraction toolchain; the lint gate passes vacuously green

**What couldn't be expressed:** a `model!`-authored model that the documentation generator,
diagram generator or linter can audit. Run over EXP-13's compliant DSL arm, the real tools
produced: **0** process documents (hand-written control: 10), a 1-node/0-edge context diagram
(control: 8/7), "no flow to trace" (the flow exists — inside the invocation), and **two false
gate-fatal lint ERRORs** (`E-TAG-ASSERT` cannot see the `satisfies!` lines inside the
invocation). Worse than the false positives is the vacuous clean: every sealing/F-034/F-047
check passed because the linter sees *no resource types at all*, and `E-TAG-MACRO` (F-037's
gate) reported clean because it recognizes only `model_core::`-prefixed invocations (the F-055
point-5 hard-coded kernel table). An unauditable model passes the R21 gate **green** — the
F-037 silent direction at the scale of the whole model, in the gate built to prevent it.

**What it cost:** the macro-front-end route is rejected. Closing the gap would mean teaching
every tool a second grammar (the F-055 point-5 coupling multiplied) or converging the DSL's
surface back onto Rust-shaped lines — which trace.sh compatibility had *already* forced
(literal `fn`/`trait`/`///`-tag lines at the invocation site; every sugared form is invisible,
see the F-037 extension) — at which point the front-end buys ~71 code lines per slice-sized
model and a +10 % flow tax while the interesting constructs still live in escape hatches.

**Workaround adopted (hard rule, candidate R22):** model source is authored as — or generated
into — ordinary Rust source; a whole-model macro front is never built. The two salvage items:
grow **model-core's kernel macro family** one declaration-layer construct at a time
(source/sink/entry generators), staying inside the `model_core::` invocation set the tools
already parse and taught to the shared scanner in the same change; and the token-preserving
error precedent (F-063).

**Evidence:** EXP-13 (`experiments/exp13-dsl-macro/RESULTS.md` §Tools and recommendation;
`toolcheck/docs/**`, `toolcheck/analysis/exp13-dsl.md` vs `…-control.md`).

---

## F-061 — Generating ordinary source preserves both error layers and the whole tool surface; the notation layer owns flow-discipline errors Rust catches only at test time

**What worked:** EXP-14's std-only `.lav` → crate compiler (`lavc`, diagram-gen house style)
has none of the macro-front failure modes, because generated code is plain committed source.
Measured on the three canonical violations: conservation E0080s land on the generated assert
with the custom message leading; requirement E0277s carry the R10-rule-8 phrasing verbatim
from notation attributes (F-044 ext.); overdraws reproduce the canonical R15 shape because the
compiler threads every literal magnitude (including the person's running budget, saturating at
zero). Breadcrumb comments emitted **on the same line as each flow call** make rustc's "while
instantiating" note render the `.lav` line inside the error itself — the modeller never opens
the generated file. The real trace.sh (exit 0), modellint (0 ERROR/0 WARN) and docgen (11
documents, zero WARN comments) all run green over the output: the emitter *is* a codification
of the house style. Above the Rust layer, the notation checker reports R2 double-use
("`water` was already used by `fill_kettle` at line 107 - a resource can be in only one
process at a time (R2)") and R1 leaks ("`heat` is never accounted for") at **generation time**
with file:line:col, caret and help lines — strictly better than F-025's borrow-checker
vocabulary, and earlier than the test-time tripwire (F-002/F-032).

**What it cost:** a ~3.2 kLOC one-off std-only compiler covering only the continuous-resource
core (F-064); plus one emitter gotcha — a generated crate's Cargo.toml must declare
`test-support = []`, because the kernel macros expand a fixture constructor behind the
*invoking* crate's feature (else 9 unexpected-`cfg` warnings).

**Workaround adopted (the regeneration discipline):** the `.lav` file is source and the
generated crate is committed output under a regenerate-and-diff CI gate (byte-identical
generation makes the gate exact; any hand edit fails it); hand-written code goes **beside, not
inside** — a separate crate depending on the generated one (the R1/F-055-ext-7 layout);
promotion to hand-maintained is one-way and explicit (delete the `.lav` and headers in one
commit) — no merge-on-regenerate middle state. Condition before production use: a
`lavc --check` pass evaluating literal balances and budget draws at the notation layer,
pre-empting F-001 for notation-authored models (see the F-001 extension).

**Evidence:** EXP-14 (`experiments/exp14-dsl-external/RESULTS.md` criteria 2/3a/3b/6/7;
`generated/v-*/` builds; `errors/*.lav` transcripts; `lint-root/` tool output).

---

## F-062 — The agreed spec format mechanically determines ~half a model — all of its structure and none of its error-quality machinery; the enumerated holes are the required syntax of any fuller notation

**What worked / the split:** EXP-15's std-only generator parsed the real CS-1 `SPEC.md`
unchanged and emitted **51 % of the crate by LOC, 63 % of model items fully** (75 % incl.
partial): every sealed state type (incl. multi-quantity const parameters), all five boundary
impls (incl. the F-034 decreasing-space bin and a `SupplyN<N3>` discrete supplier with fill
machinery), all boundary fns, process signatures with conserving bodies and balance asserts,
and two **passing** §6 flow tests — full draw/record/History threading, budget type-checked at
flow end. What the spec cannot determine, emitted as 12 compiler-enumerable **SPEC-HOLE**s
(`cargo build --features deny-holes` → one `compile_error!` each): requirement bounds on
signatures (wrong-state errors degrade from the REQ-phrased E0277 to a bare E0308),
characteristic consts and permit-gated extractions, `Satisfies:` placement, literal-vs-generic
magnitudes, waste-routing strength, and **all tests** — exactly the half the method's curated
diagnostics live in. Four further decisions were silent inferences the generator had to invent
(tripwire choice — now the F-040 extension; state-ownership collisions; "unqualified mention =
initial state"; assert-vs-structural guessing), and mechanical naming drifts from the
implementer's (`LoadedTeapot` vs `LoadedPot`) — harmless in a fresh crate, fatal to any
regeneration round-trip against a hand-touched one.

**Spec validation is its own error class, adoptable independently of generation:** balance
arithmetic, waste-destination closure (§5 ↔ §4), §3↔§5↔§6 name closure, Satisfies-id closure
and draw/balance time agreement are all checkable against the *document* in milliseconds, with
§/line references and modeller phrasing ("a model built from this line cannot compile — fix
the specification, not the model"). Today an unbalanced spec line surfaces days later as an
E0080 in the implemented crate. CS-1's §8 feedback items 2/3/5/6 were each re-discovered
mechanically — the feedback loop captures real under-determination.

**What it cost:** parsing the current template needed ~10 invented conventions — the candidate
template amendments A1–A10 (headline: every Balances clause marked, mandatory waste routing,
canonical backticked identifiers, underscore-grouped numbers, structured id allocation).

**Workaround adopted:** the spec stays the human contract and becomes machine-checked; the
generator's output is **one-shot scaffolding** the implementer completes by filling the
enumerated holes (promoted to hand-maintained immediately), or the front half of an
EXP-14-grade notation. Closing the 12 holes *inside* SPEC.md would turn the spec into that
notation with worse syntax — rejected.

**Evidence:** EXP-15 (`experiments/exp15-dsl-from-spec/RESULTS.md` §1–§4;
`generated/cs1-gen/`; `tests/fixtures/` + `tests/validation.rs`).

---

## F-063 — Token-preserving macro layers keep the curated error surface; the costs are fixed by stable `macro_rules!` grammar limits

**What worked (contrary to the expected failure mode):** all three canonical violations
authored through EXP-13's `model!` kept the modeller-visible error **head** (the curated
assert / `on_unimplemented` message, F-044 ext.) and **tail** (primary span or "while
instantiating" note on the modeller's own `step` line, decimal magnitudes) identical to the
hand-written control. The mechanism: `macro_rules!` only *rearranges* the modeller's tokens,
and spans follow tokens; every violating number and call is written by the modeller inside the
invocation. Degradation is confined to the middle breadcrumbs where the macro *synthesizes*
code: the generated-assert E0080 names the whole 67-line invocation as its span and notes into
the macro's own `$( const { assert!($aexpr, $amsg) }; )*` line — noise a modeller following
the error-reading guide skips, hostile only to top-to-bottom reading.

**What it cost (the F-013/F-036 walls, designed around rather than hit):** generics in square
brackets (`process fn boil [const G: u64, …]` — a `<…>` list is not one token tree); every
derived name modeller-spelled (`enter =`/`draw =`/`assert =` — no identifier synthesis,
F-013); trait/impl semantics (characteristics, permits, sealed recursion, all of R17/R18)
surrendered to `rust { }`/`body { }` escape hatches (~40 of the dsl arm's 92 code lines); flow
sugar isomorphic to `let`-bindings (+10 % LOC — pure syntax tax); and grammar quirks
(declarative processes return trailing-comma tuples, so a single-output process returns a
1-tuple `(A,)`; `assert` clauses need `$msg:literal : $expr:expr;`). Net: −44 % declaration-
layer code LOC (163 → 92) against a 294-line one-off macro.

**Workaround adopted:** the precedent licenses growing model-core's kernel macro family
(declaration-layer constructs, F-060's salvage path) with confidence that curated errors
survive — never a second whole-model grammar.

**Evidence:** EXP-13 (`experiments/exp13-dsl-macro/transcripts/*.txt`, `*/tests/ui/*.stderr`
pairs, `dsl/src/macros.rs`; RESULTS.md LOC and R-coverage tables).

---

## F-064 — The external notation is ~4× denser than the Rust it replaces over the continuous-resource core; every further R-feature grows it toward the language it fronts

**What worked:** 113 notation lines generate a 431-line crate (hand-written control ≈ 455
lines of cs1), deterministically and idempotently, in < 0.3 s, covering R1–R3, minimal R6,
R7–R10, unbounded-boundary R12, R15 and single-history R16 — with R10 traceability and the
R20/R21 tooling fully intact (F-061).

**What couldn't be expressed:** discrete items and finite Peano suppliers/consumers
(`SupplyN`, contents-keeping bins — the F-016/F-034/F-039 machinery), fallible processes
(R17), qualifications (R18), money (R19), per-process unit tests and compile-fail regressions
(R4/R5), and catalogue characteristics (R6 fixed roles). Each is expressible only by growing
the grammar, checker *and* emitter together — the notation's economy comes precisely from
hard-coding the kernel's semantics (the F-055 point-5 coupling, now on the generating side).
Contrast with F-062: one-shot scaffolding *can* emit the discrete/bin structure because its
gaps are handed to an implementer as enumerated holes; a regenerate-forever notation must
fully own everything it covers, because nobody may edit its output.

**What it cost / workaround adopted:** scope honesty — the notation route is adopted (if at
all) only for the continuous-resource core it covers, as one generated crate among
hand-written ones under the F-061 regeneration discipline; full R13/R17–R19 coverage is not
chased, since that would regrow Rust's complexity inside the notation and dilute the
kernel-macro single source of truth.

**Evidence:** EXP-14 (`experiments/exp14-dsl-external/RESULTS.md` criteria 4/5/6; the
R-coverage table).


## F-065 — Authoring a specification under the R22 gate catches the item-grammar error class at the document, on the first pass, at zero downstream cost

**What worked:** CS-6 is the first specification *authored* under the speccheck gate from the
first line (CS-1..CS-5 were migrated to it after agreement). A careful first pass, written in
the house style by an author who knows the conventions, still drew **5 errors and 1 warning**:
prose trailing inside Consumes/Produces item lists three times (A9 — "which is exhausted",
"no mass change", "drawn from the grid"), a §4/§5 energy input with no §3 canonical-identifier
row (A4 — `GridEnergy` was referenced before it existed), a space-grouped number in §2's prose
(A5), and a parenthesised aside ("drawn from the block, 5 g per tin") parsed as a second
magnitude on a one-magnitude resource. Every one is a real document defect of exactly the
class that previously surfaced days later as implementation round-trip feedback (the CS-1 §8
items behind F-062, CS-2's "a state change must have an owning process"). All were fixed at
the document in minutes; the second pass was clean, before any reviewer saw the draft.

**What it cost:** the A9 item-purity discipline genuinely fights natural prose — the instinct
to qualify an item inline ("taken from the box, which is exhausted") loses every time, and
the explanation moves to its own sentence. That is the trade R22 bought deliberately: the
items are machine-resolved, the prose is decoration.

**Evidence:** the CS-6 draft commit (`d565256`) records the first-pass error list verbatim;
`./tools/spec.sh case-studies/cs6-bread-batch/SPEC.md` before and after.

## F-066 — A speccheck-clean specification can still scaffold silently wrong code: where specgen's item grammar met unexercised phrasings it guessed instead of holing — four mis-derivation classes found, fixed and pinned at CS-6

**What happened:** the first real `specgen` run (CS-6, the R22 pipeline's designated field
test) produced a 5,913-line scaffold that compiled-shaped code violating conservation — the
F-037/F-060 green-but-wrong class, inside the very tool the generation regime governs. Four
silent mis-derivation classes:

1. **A parenthesised magnitude became an item count** ("the `mixed` `Dough` (1_682 g)" →
   a tuple of 1,682 `MixedDough::mint()` calls, and an import of the nonexistent `N1682`):
   the count fallback took the first stated quantity even when it carried a magnitude unit.
2. **Consumed items vanished from signatures** (`knead`, `prove`, `divide_and_shape` took no
   dough at all while minting their outputs from nothing): a count>1 consume was routed to
   `SupplyN` even with no §4 supplier, then silently skipped at signature emission.
3. **Counts on reusables were hardcoded to 1** ("2 `clean` `LoafTin`s (450 g each)" → one
   parameter, one return).
4. **Fallible (R17) Produces (Ok)/(Fail) arms were parsed, name-checked and discarded** —
   the scaffolded `bake` returned no loaf in any arm and defused the outcome token.

Six further latent gaps had to be closed for a compiling, conserving scaffold: shared §4
sinks dropped every row after the first; the "`Baker` (person)" alias fell through to a plain
reusable (person budget 0, underflowed draw consts); "container with remainder" inputs had no
emission at all; stateful start objects lacked boundary constructors; §6 flow-end rests had
no exits (tripwire panics); repeated processes (P6 ×2) broke flow emission.

**Why the gate missed it:** speccheck validates the *document* (it was clean, and stayed
clean — F-065); the generator's "never guess" obligation was implemented only for constructs
it *recognized* as under-determined (the enumerated holes). Unrecognized phrasings fell
through to wrong code instead of holes. EXP-15's pinned regression covered CS-1 alone, which
exercises none of these phrasings — fallibility, multi-count reusables, article-plus-magnitude
items and person aliases all arrived with CS-6. The field test did exactly what it was for.

**What it cost / workaround adopted:** fixes in `tools/spec-gen` (emit layer), with the
regression suite grown 21 → 28: one pinned fixture per defect class in the CS-6 phrasing
(mutation-needle discipline), a pinned CS-6 scaffold regression (builds clean, generated flow
tests pass, exactly 14 holes enumerate) beside the CS-1 pin, and determinism extended to
CS-6. The regenerated scaffold is 1,154 lines (from 5,913 defective), builds with zero
warnings, passes its 2 generated flow tests, and both `bake` arms conserve. Holes grew 12 →
14 (U-08: the exhausted supplier continuing as the declared empty state is a generator
convention surfaced to the author; U-14: R17 refinements beyond the scaffolded dual-arm
shape). The regime rule this sharpens: **what the generator does not positively recognize
must become a hole, never code** — silence is the failure mode, not conservatism.

**Template-rule candidate (for review, agreed-before-committed):** the house idiom
"Satisfies: — (enables REQ-0NN)" is scraped by the §5 parser as a *real* Satisfies entry
(CS-2's P2/P4 carry it too), seeding wrong traceability claims in scaffold doc tags — an
F-053-class green-but-wrong hazard at the generator surface. Either the template rules the
phrasing ("a Satisfies value beginning '—' claims nothing; put 'enables' notes in prose") or
the parser is taught the idiom; deciding which changes agreed documents, so it goes to the
review gate.

**Evidence:** the defective scaffold's signatures (quoted in the fix commit);
`tools/spec-gen/tests/validation.rs` (`d1_…`…`d4_…`, `supplier_box_consume…`,
`shared_sink…`, `pinned_cs6_scaffold…`); `cargo test` 28/28 green; speccheck output over all
six specifications byte-identical before and after the fix.

**Extension (2026-10-08, the CS-6 hand-finish):** the fixed scaffold's survival through
promotion measures the step-10 scope prediction directly: **697 of 1,154 scaffold lines
(60.4 %) survived verbatim** into the finished 2,547-line crate, concentrated exactly where
predicted — all 25 resource-kernel macro invocations were essentially commit-ready, the
sealed `SupplyN` yeast-box machinery was kept untouched, and the flow skeletons' budget and
remainder arithmetic was perfect for both §6 orders — while the requirement machinery,
process semantics and flow architecture (the error-quality "soul") were hand-written, as the
regime says they must be. Hand-finish also surfaced four categories of **emitted-code debt**,
none silent (each caught by an existing gate layer — the layered defence working as designed)
but all specgen backlog: (1) the scaffold's pure-literal const asserts
(`assert!(1_000 + 650 + … == 1_682 + …)`) fail the workspace clippy gate
(`assertions_on_constants`/`eq_op`) — the scaffold compiles but would not pass ci.sh step 3
unmodified; (2) `draw_process!` invocations emitted into `boundary` where the house
convention is `processes` (caught as W-R20-PLACE); (3) a dead parallel `LoafTin` reusable
object scaffolded beside the Clean/Greased/Used state chain that fully models the tin; and
(4) U+202F narrow no-break spaces in generated prose numbers, which defeat grep and
exact-string editing. The generators' hardcoded `default_crates()` list is a fifth, older
item of the same F-055-point-5 class: a new model crate is invisible to diagrams/docgen/lint
until the list is extended by hand (one line, done for cs6 in the landing commit).
