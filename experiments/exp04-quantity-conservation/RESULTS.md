# EXP-04: Compile-time conservation of quantities — RESULTS

Tests R3 and R7: can "mass in = mass out" be a compile error on stable Rust?

- Toolchain: `rustc 1.98.1 (48a229cea 2026-09-01) (Homebrew)`, `cargo 1.98.1`.
- Dev-dependency: `trybuild 1.0.121` (only permitted dependency).
- Clean-build time (`cargo clean && time cargo build`, three runs): 0.96 s /
  0.98 s / 0.97 s — **median 0.97 s**. (Excludes dev-deps; the first
  `cargo test` also builds trybuild and its transitive deps, ~4 s extra, once.)
- `cargo test`: **all green** — 8 unit tests, 4 trybuild compile-fail cases,
  5 `compile_fail` doc-tests.

## What was built

- `src/lib.rs`
  - `const_style` — `Grams<const V: u64>` / `Millimetres<const V: u64>`
    (private fields, `#[must_use]`, no `Clone`/`Copy`), each exposing
    `VALUE`; `AssertSum` (technique 1); `split`/`combine` (associated-const
    assert) and `split_inline`/`combine_inline` (inline `const` block,
    technique 2); placeholder boundary suppliers (R12).
  - `trait_style` — the R7 trait form: `Qty<const V: u64, U: Unit>` +
    `trait Quantity { type Unit; const VALUE: u64; }`; `AssertConserves`
    stated over `Quantity` *types* (values read from the associated const);
    unit-generic `split`/`combine`.
  - `split_grams!` — technique 3, a call-site macro planting a concrete
    `const _` item (check-time; see below).
  - `type_level_style` — technique 4, Peano naturals with `Add`, conservation
    as the trait bound `A: Add<B, Sum = In>` (check-time; magnitudes limited).
  - Doc-tests: `compile_fail` demos of the two techniques that don't work on
    stable, and of the three post-monomorphization conservation violations.
- `tests/compile_fail.rs` + `tests/ui/` — trybuild cases (see verdicts).
- `tests/ui-postmono-not-caught-by-trybuild/` — the three conservation
  violations trybuild cannot see, kept out of the glob as the minimal
  demonstration, with a README explaining why.

## Which check techniques work in a generic function on stable

| # | Technique | Compiles in a generic fn? | Violation fires | Seen by `cargo check`? | Seen by trybuild? |
|---|-----------|---------------------------|-----------------|------------------------|-------------------|
| 1 | Associated-const assert forced by use (`let _ = AssertSum::<IN, A, B>::OK;`) | yes | at **instantiation** (monomorphization, i.e. codegen) | **no** | **no** |
| 2 | Inline `const { assert!(A + B == IN) }` | yes (stable since 1.79) | at **instantiation** (same) | **no** | **no** |
| 3 | `const` item inside the fn body | **no** — E0401 | — | — | — |
| 4 | `where If<{ A + B == IN }>: True` | **no** — needs `generic_const_exprs` | — | — | — |
| 5 | Call-site macro planting `const _: () = assert!(…)` with literals | n/a (call site only, literal values) | at **type-check** of the caller | **yes** | **yes** |
| 6 | Type-level Peano bound `A: Add<B, Sum = In>` | yes | at **type-check** of the call site | **yes** | **yes** |

Definition vs instantiation, precisely: with techniques 1 and 2 the generic
*definition* always compiles — even `split::<2000, 1500, 600>` written inside
another still-generic function compiles. The error fires only when a fully
concrete instantiation is reached during code generation. That is why
`cargo check` (which stops before codegen) reports nothing; only
`cargo build` / `cargo test` reject the program. Verified directly: compiling
`tests/ui-postmono-not-caught-by-trybuild/split_not_conserving.rs` with
`--emit=metadata` (what `cargo check` does) **succeeds**; a full build fails
with E0080.

## Verdicts per evaluation criterion

### 1. Both R7 quantity styles, each exposing `VALUE` — **pass**

`const_style::Grams<V>::VALUE` (inherent const) and
`<Qty<V, U> as Quantity>::VALUE` (trait associated const) both work and are
asserted in unit tests (`const_style_split_infers_from_annotation`,
`trait_style_split_and_combine`). The trait style is strictly more useful:
one `split` serves every unit, and the conservation assert can be written
over `Quantity` types instead of raw consts.

### 2. Is a conservation violation a genuine compile error? — **pass with complications**

Yes: splitting 2000 g into 1500 g + 600 g is rejected with `E0080` and our
custom message. The complications:

- It fires at **instantiation, during codegen**. `cargo check` — and
  therefore editor diagnostics driven by check, e.g. rust-analyzer's default
  `cargo check` integration — reports **nothing**. The mistake surfaces only
  on `cargo build`/`cargo test`. A project relying on this must gate CI on
  `build`/`test`, never on `check` alone.
- trybuild runs `cargo check` internally, so it cannot see these errors at
  all (criterion 5 below).
- If nothing ever instantiates the violating call (dead generic code), it is
  never checked.

Techniques 5 and 6 avoid all three complications by moving the check to type
checking, at the cost of literals-only (macro) or unary type encodings
(Peano).

### 3. `combine` in the same style — **pass with complications**

Works identically to `split`. The complication is ergonomic: on stable the
output magnitude **cannot be computed** (`Grams<{A + B}>` needs
`generic_const_exprs`), so the caller must state the total and the assert
checks it. Exception: in the type-level style the compiler computes the
output itself (`fn combine<A: Add<B>, B: Nat>(…) -> MassG<A::Sum>`) — the
only variant where the caller writes nothing and can get nothing wrong.

### 4. Unit safety (Grams + Millimetres must not compile) — **pass**

Both styles reject it with a plain, pre-monomorphization `E0308` that
`cargo check` and trybuild both catch (`tests/ui/unit_mismatch_const.rs`,
`tests/ui/unit_mismatch_trait.rs`). In `const_style` the safety comes from
`Grams<V>` and `Millimetres<V>` being unrelated types; in `trait_style` from
`combine` forcing one `U` on both arguments.

### 5. trybuild compile-fail: splitting 2000 g into 1500 g + 600 g — **fail as literally specified; pass via two workarounds**

As literally specified — a trybuild case calling the generic
`split::<2000, 1500, 600>` — this **fails**: trybuild compiles test cases
with `cargo check`, the violation is post-monomorphization, and trybuild
reports (verbatim):

```
test tests/ui/split_not_conserving.rs ... error
Expected test case to fail to compile, but it succeeded.
```

The minimal examples are kept in `tests/ui-postmono-not-caught-by-trybuild/`.
Two working substitutes are in the crate, both green under `cargo test`:

- **rustdoc `compile_fail` doc-tests** (rustdoc fully builds doc-tests, so
  the E0080 is reached): the exact 2000 → 1500 + 600 case appears twice in
  `src/lib.rs` (both techniques), plus the trait-style combine violation.
  Caveat: on stable, rustdoc ignores the `,E0080` code annotation, so a
  `compile_fail` doc-test passes if the snippet fails for *any* reason —
  keep the snippets minimal.
- **the `split_grams!` macro** version of the same violation is a genuine
  trybuild case (`tests/ui/split_macro_not_conserving.rs`) because the
  planted `const _` item is evaluated at check time.

### 6. Caller ergonomics: do the const parameters infer? — **pass**

Yes, pleasantly. Evidence in `src/lib.rs` unit tests:

- From a binding annotation, nothing else spelled out:
  `let (part, offcut): (Grams<1500>, Grams<500>) = split(stock);` — `IN`
  comes from the argument, `A`/`B` from the annotation.
- Through a chain: `let back: Grams<100> = combine(a, b);` infers everything
  from downstream use.
- Partial turbofish with inferred const arguments works on 1.98:
  `split::<_, 1500, 500>(stock)`.
- With no annotation at all, the failure is a clear `E0284`:
  `cannot infer the value of the const parameter 'A' declared on the
  function 'split'`, with a `consider specifying the generic arguments`
  suggestion — a modeller would know what to do.

The macro is the exception: `split_grams!(stock, 2000 => 1500 + 500)` needs
the input restated as a literal because a macro cannot see the argument's
type.

## Representative compiler errors, verbatim

**(a) Conservation violation, associated-const technique, same crate**
(probe equivalent of `split::<2000, 1500, 600>`; identical shape in this
crate's doc-tests):

```
error[E0080]: evaluation panicked: conservation violated: outputs do not sum to input
 --> src/main.rs:7:24
  |
7 |     pub const OK: () = assert!(A + B == IN, "conservation violated: outputs do not sum to input");
  |                        ^^^^^^^^^^^^^^^^^^^^ ... evaluation of `AssertSum::<2000, 1500, 600>::OK` failed here

note: erroneous constant encountered
  --> src/main.rs:12:13
   |
12 |     let _ = AssertSum::<IN, A, B>::OK;

note: the above error was encountered while instantiating `fn split_a::<2000, 1500, 600>`
  --> src/main.rs:29:45
   |
29 |     let (c, d): (Grams<1500>, Grams<600>) = split_a(Grams::<2000>(()));
```

Judgement: **very readable** — the custom assert message leads,
`AssertSum::<2000, 1500, 600>` shows the offending numbers, and the final
note points at the exact call site.

**(b) The same violation across a crate boundary** (model user depends on the
library; from building `tests/ui-postmono-not-caught-by-trybuild/split_not_conserving.rs`):

```
error[E0080]: evaluation panicked: conservation violated: the two output quantities do not sum to the input
   --> /usr/local/Cellar/rust/1.98.1/lib/rustlib/src/rust/library/core/src/panic.rs:62:9
    |
 62 |           $crate::panicking::panic_fmt($crate::const_format_args!($($t)+));
    |           ^^^^ ... evaluation of `exp04_quantity_conservation::const_style::AssertSum::<2000, 1500, 600>::OK` failed here
...
note: the above error was encountered while instantiating `fn exp04_quantity_conservation::const_style::split::<2000, 1500, 600>`
 --> tests/ui-postmono-not-caught-by-trybuild/split_not_conserving.rs:7:53
  |
7 |     let (part, offcut): (Grams<1500>, Grams<600>) = split(stock);
```

Judgement: **readable but degraded** — the primary span lands inside
`core/src/panic.rs`, which will puzzle a modeller; the custom message and the
"while instantiating" note still carry the day. In the trait style the
instantiation name balloons (fully-qualified `Qty<…>` three times over) and
rustc dumps the full type name to a side file
(`note: the full name for the type has been written to '….long-type-….txt'`).

**(c) trybuild on the post-mono case** — see criterion 5: trybuild declares
the violating program compiles. Judgement: actively **misleading** if you
don't know the check/build distinction.

**(d) Unit mismatch, trait style** (`tests/ui/unit_mismatch_trait.stderr`):

```
error[E0308]: mismatched types
 --> tests/ui/unit_mismatch_trait.rs:8:49
  |
8 |     let total: Qty<2000, Grams> = combine(mass, length);
  |                                   -------       ^^^^^^ expected `Qty<_, Grams>`, found `Qty<500, Millimetres>`
```

Judgement: **excellent** — "expected `Qty<_, Grams>`, found
`Qty<500, Millimetres>`" reads exactly as the unit error it is. The
const-style version (`expected 'Grams<_>', found 'Millimetres<500>'`) is
equally clear.

**(e) Type-level (Peano) conservation violation**
(`tests/ui/split_type_level_not_conserving.stderr`):

```
error[E0271]: type mismatch resolving `<Succ<Succ<Succ<Zero>>> as Add<Succ<Succ<Succ<Zero>>>>>::Sum == Succ<Succ<Succ<Succ<Succ<Zero>>>>>`
 --> tests/ui/split_type_level_not_conserving.rs:8:42
  |
8 |     let (a, b): (MassG<N3>, MassG<N3>) = split(five);
  |                                          ^^^^^^^^^^^ expected `Succ<Succ<Succ<Succ<Succ<Zero>>>>>`, found `Succ<Succ<Succ<Succ<Succ<Succ<Zero>>>>>>`
```

Judgement: **decipherable at 5, hopeless at 2000** — "expected 5, found 6" is
in there, but only by counting `Succ`s; no custom message is possible, and no
numeric `VALUE` appears. (Alias-preserving output and binary encodings are
EXP-01's territory.)

**(f) The two techniques that don't work on stable** (kept as `compile_fail`
doc-tests). A `const` item inside the generic fn:

```
error[E0401]: can't use generic parameters from outer item
5 |     const CHECK: () = assert!(A + B == IN);
  |           -----               ^ use of generic parameter from outer item
  = note: a `const` is a separate item from the item that contains it
```

and the where-clause form:

```
error: generic parameters may not be used in const operations
9 |     If<{ A + B == IN }>: True,
  |          ^ cannot perform const operation using `A`
  = help: const parameters may only be used as standalone arguments here, i.e. `A`
```

Judgement: both **clear** about the language limitation (though neither names
`generic_const_exprs` as the missing feature on 1.98).

## Candidate FINDINGS.md entries (ready to paste)

- **Compile-time conservation checks are post-monomorphization on stable
  Rust.** Both stable in-function techniques (an associated-const assert
  forced by `let _ = AssertSum::<IN, A, B>::OK;`, and an inline
  `const { assert!(A + B == IN) }`) make a conservation violation a genuine
  `E0080` compile error — but only during code generation. `cargo check`,
  rust-analyzer diagnostics based on it, and trybuild (which checks, not
  builds) all report the violating program as fine, and dead generic code is
  never checked. Cost: violations surface later than a modeller expects.
  Workaround adopted in EXP-04: gate CI on `cargo build`/`cargo test`; write
  compile-fail regression tests as rustdoc `compile_fail` doc-tests (rustdoc
  fully builds them) instead of trybuild; keep those snippets minimal because
  stable rustdoc ignores the expected-error-code annotation.

- **trybuild cannot test post-monomorphization errors.** The R4 instruction
  "compile-fail examples via trybuild" silently passes violating code when
  the error is const-eval inside a generic function. trybuild remains right
  for pre-monomorphization errors (unit mismatches, trait-bound violations).
  Workarounds: `compile_fail` doc-tests, or a call-site macro
  (`split_grams!(stock, 2000 => 1500 + 600)`) that plants a concrete
  `const _: () = assert!(…);` item — concrete `const` items are evaluated by
  `cargo check`, so that variant trybuild does catch. The macro costs
  restating the input as a literal and only works at monomorphic call sites.

- **Output quantities cannot be computed on stable.** `combine` cannot return
  `Grams<{A + B}>` (needs `generic_const_exprs`), and a where-clause bound on
  `A + B == IN` is rejected with "generic parameters may not be used in const
  operations". Cost: the caller of `combine` must state the expected total,
  which the assert then checks — one more place to type a number, though not
  a soundness hole. A `const` item inside the generic fn is also unusable
  (E0401: nested items can't see outer generics); the assert must live in an
  inline `const` block or on a separate generic struct.

- **Type-level naturals move the conservation check to `cargo check`, but
  don't scale in unary.** With Peano numbers and `A: Add<B, Sum = In>`,
  conservation is an ordinary trait bound: violations fire at type-check, are
  visible to check/rust-analyzer/trybuild, and `combine`'s output type is
  computed by the compiler rather than stated by the caller. Cost: 2000 g is
  a 2000-deep `Succ` chain — unusable magnitudes, and the error message is a
  `Succ` chain with no numeric value shown ("expected
  `Succ<Succ<Succ<Succ<Succ<Zero>>>>>`, found `Succ<…6 deep…>`"). Only viable
  for quantities if EXP-01's binary encoding proves out.

- **Cross-crate const-eval errors point into `core`.** When the violating
  instantiation is in a downstream crate (the model user's position), the
  E0080 primary span is `library/core/src/panic.rs`, not the assert in the
  model library, and trait-style instantiations dump their full type names to
  a `….long-type-….txt` side file. The custom assert message and the "while
  instantiating `fn split::<2000, 1500, 600>`" note (which points at the
  user's call site) remain, so the error is findable but noisier than the
  same-crate case.

## Recommendation: **adapt**

Adopt compile-time conservation, with these adaptations:

1. **Primary style: the R7 trait form** (`Qty<const V: u64, U: Unit>` +
   `Quantity`), not bare `Grams<const V>`: one generic `split`/`combine`
   serves every unit, unit mixing is a crisp pre-mono `E0308`, and the
   conservation assert can be stated over `Quantity` types so it keeps
   working for quantity types that aren't `Qty` (e.g. named characteristic
   types per R6/R7).
2. **Check technique: inline `const { assert!(…) }`** (or the `AssertSum`
   struct — behaviour is identical; the struct form prints the offending
   numbers more prominently, the inline form needs no extra item). Write a
   custom assert message; it leads the error output.
3. **Accept and manage the post-mono nature**: CI and the documented workflow
   must use `cargo build`/`cargo test`; record prominently that `cargo check`
   and default editor diagnostics will not show conservation errors.
4. **Compile-fail tests**: use trybuild only for pre-mono errors (unit
   safety, missing/wrong resource types); use `compile_fail` doc-tests for
   conservation violations.
5. **`combine` callers state the total** (checked by the assert). If EXP-01's
   binary type-level numbers scale to realistic magnitudes with tolerable
   errors, revisit: they are the only route to check-time conservation with
   compiler-computed totals in fully generic code.
