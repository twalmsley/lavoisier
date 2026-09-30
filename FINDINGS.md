# FINDINGS

Findings log required by R14 of `instructions.md`. Consolidated on 2026-09-30 from the
`RESULTS.md` files of experiments EXP-01 through EXP-08 (`experiments/exp0N-*/RESULTS.md`).
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
