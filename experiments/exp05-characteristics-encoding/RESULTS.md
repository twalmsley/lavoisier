# EXP-05: Marker traits vs type parameters — RESULTS

**Tests:** R6's claim that both encodings of characteristics work and can be mixed.

**Toolchain:** `rustc 1.98.1 (48a229cea 2026-09-01) (Homebrew)`, stable. Only dev-dependency: `trybuild`.

**Compile time** (`cargo clean && time cargo build`, three runs): 2.62 s, 2.61 s, 2.13 s — **median 2.61 s**. At this catalogue size (24 combinations) compile time is a non-issue in either encoding.

**Code map:**
- `src/characteristics.rs` — marker traits, one per characteristic value.
- `src/marker.rs` — marker encoding, macro-generated (one struct per combination).
- `src/marker_by_hand.rs` — same encoding with no macro, for honest LOC counting.
- `src/param.rs` — `Bolt<Size, Material, Length>` with kind traits.
- `src/bridge.rs` — blanket impls bridging param → marker.
- `src/requirements.rs` — the same requirement in both styles (R10 pattern).
- `src/pitch_demo.rs` — adding a new dimension (thread pitch) in both styles.
- `tests/compile_fail/*` — wrong-bolt errors in both styles, verbatim in `.stderr` files.

All code compiles; `cargo test` passes (8 unit tests + 5 trybuild compile-fail cases).

---

## 1. Verdicts per evaluation criterion

### 1a. Lines of code per encoding — **pass** (with a macro caveat)

Non-blank, non-comment LOC for the 24-combination catalogue (4 sizes × 2 materials × 3 lengths):

| Encoding | LOC | Scales as |
|---|---|---|
| Marker traits, hand-written (`marker_by_hand.rs`) | 121 (+10 trait decls) | O(product): 5 lines per combination (1 struct + 4 impls) |
| Marker traits, macro (`marker.rs`) | 38 (+10 trait decls) | still O(product): 1 line per combination + 11-line macro |
| Type parameters (`param.rs`) | 33 | **O(sum)**: ~2 lines per characteristic value, fixed overhead for `Bolt` |
| Bridging impls (`bridge.rs`) | 14 | O(sum): 1 line per characteristic value + 1 for `IsBolt` |

The macro cuts the marker encoding ~3×, but **cannot eliminate the per-combination line**: stable `macro_rules!` has no identifier concatenation, so the macro cannot take three value lists and emit the cross product with synthesized names like `M8SteelBolt15mm`. Each combination must be named by hand (`bolts! { M8SteelBolt15mm: M8, Steel, Length15mm; … }`). The `paste` crate or nightly `macro_metavar_expr_concat` would fix this, but dependencies are forbidden (common protocol) and nightly is out (R16/target-language).

The parameter encoding needs no macro at all — nothing in it scales with the number of combinations.

### 1b. Cost of adding a characteristic value (M12) — **pass**

Recorded diff for experiment step 4 (both encodings kept in the final crate, marked with comments):

| Encoding | What changed |
|---|---|
| Marker | 1 new trait (`trait M12 {}` in `characteristics.rs`) + **6 new invocation lines** in `marker.rs` (one per M12 combination; 30 lines in the hand-written version), each needing a newly invented struct name |
| Parameter | 1 new struct (`pub struct SizeM12;`) + 1 kind impl (`impl Size for SizeM12 {}`) in `param.rs` — **2 lines**, regardless of how many materials/lengths exist |
| Bridge (if maintained) | +1 blanket impl (`impl<M, L> M12 for Bolt<SizeM12, M, L> {}`) |

Existing parameter-style functions that are generic over size (`fasten_m12_param`-style signatures with `S: Size`) accepted the new size with **zero** changes. In the marker encoding, every new combination is a new nominal type that no existing code mentions.

### 1c. Cost of adding a whole new dimension (thread pitch) — **pass with complications** (both styles; demonstrated in `src/pitch_demo.rs`)

| Encoding | Cost |
|---|---|
| Marker | **Multiplicative.** The struct set is the cross product, so k pitch values multiply the struct count by k (24 → 48 for coarse/fine). Every struct gains one impl line, and every struct must be *renamed* (`M8SteelBolt15mm` says nothing about its pitch), so every existing use site is touched too. |
| Parameter | **Additive but breaking** — unless a default type parameter is used. A plain fourth parameter `Bolt<S, M, L, P>` breaks every `Bolt<_, _, _>` spelling in the codebase. With a default, `Bolt<S, M, L, P = Pitch125>`, all existing spellings keep compiling unchanged (test `pitch_demo::tests::three_param_spelling_still_compiles`). |

**Trap found (parameter + default + bridging):** after adding `P = Pitch125`, an existing bridging impl written as `impl<M, L> M8 for Bolt<SizeM8, M, L>` now *silently* means `impl … for Bolt<SizeM8, M, L, Pitch125>` — bolts of every other pitch lose their `M8` marker with no warning. Every bridging impl must be rewritten with an explicit `P` parameter. This is exactly the kind of quiet model corruption the project exists to prevent, so it belongs in FINDINGS.md (entry F3 below).

### 1d. Error messages when the wrong bolt is passed — **pass** (both readable; different flavours — verbatim quotes in §2)

- Parameter style produces a diff-like `E0308: mismatched types` with `expected Bolt<SizeM8, Steel, L15>, found Bolt<SizeM6, Brass, L15>` — the best error of the experiment; wrong characteristics are visible side by side.
- Marker style produces `E0277` naming the *first missing characteristic trait* (`the trait M8 is not implemented for M6BrassBolt15mm`) plus a help list of types that *do* implement it — effectively a catalogue listing of valid bolts. Downsides: one duplicated error per missing characteristic, and the help list is truncated (`and $N others`).
- The bridged case (wrong `Bolt<…>` into a marker-bound function) is arguably the most instructive: it names the missing marker trait *and* points at the bridging impl (`the trait M8 is implemented for Bolt<SizeM8, M, L>`), i.e. it tells the modeller which size would have satisfied the requirement.

### 1e. Does bridging work cleanly — **pass**

Ten one-line blanket impls (`src/bridge.rs`) make every `Bolt<S, M, L>` carry the marker traits its parameters imply. No coherence/orphan problems (traits and type live in the same crate — in the real project they would live in the same library crate, which R11 already plans). Test `param_bolt_satisfies_marker_requirement_via_bridge` shows `fasten_marker(Bolt::<SizeM8, Steel, L15>::new())` compiling against the marker-style requirement trait `Req001FasteningBolt` unchanged.

**But the bridge is one-way.** A marker struct (`M8SteelBolt15mm`) can never be passed where the nominal type `Bolt<SizeM8, Steel, L15>` is demanded; a nominal type is not a bound. Marker traits are the common language: parameterized types can speak it (via the bridge), marker structs cannot speak "parameterized". Consequence: **requirements should always be written as marker/requirement-trait bounds** (which the R10 pattern already dictates); concrete `Bolt<…>` signatures should be treated as implementation details, not requirement statements.

One extra hazard found and fenced: with unbounded type parameters, a transposed `Bolt<Steel, SizeM8, L15>` compiles silently and only errors far from the mistake. Kind traits (`Size`, `Material`, `Length` bounds on `Bolt`) turn transposition into a construction-site error (trybuild test `transposed_params.rs`, quoted in §2). Cost: one trait per dimension plus one impl per value.

---

## 2. Representative compiler errors (verbatim) with readability judgements

All are stored under `tests/compile_fail/*.stderr` and reproduced by `cargo test --test trybuild`.

### 2.1 Wrong bolt, marker style (`wrong_bolt_marker_style.rs`)

```
error[E0277]: the trait bound `exp05_characteristics_encoding::marker::M6BrassBolt15mm: Req001FasteningBolt` is not satisfied
 --> tests/compile_fail/wrong_bolt_marker_style.rs:7:27
  |
7 |     let _ = fasten_marker(M6BrassBolt15mm);
  |             ------------- ^^^^^^^^^^^^^^^ the trait `M8` is not implemented for `exp05_characteristics_encoding::marker::M6BrassBolt15mm`
  |             |
  |             required by a bound introduced by this call
  |
  = help: the following other types implement trait `M8`:
            Bolt<SizeM8, M, L>
            M8SteelBolt15mmCoarse
            M8SteelBolt15mmFine
            exp05_characteristics_encoding::marker::M8BrassBolt10mm
            exp05_characteristics_encoding::marker::M8BrassBolt15mm
            ...
          and $N others
  = note: required for `exp05_characteristics_encoding::marker::M6BrassBolt15mm` to implement `Req001FasteningBolt`
note: required by a bound in `fasten_marker`
```

**Judgement: good.** A modeller reads "M6BrassBolt15mm does not satisfy Req001FasteningBolt because it is not M8", and the help list is effectively "here are the valid bolts". Noise: the same call also emits a second, near-identical E0277 for the missing `Steel` trait (one error per missing characteristic), and the help list is truncated.

### 2.2 Wrong bolt, parameter style (`wrong_bolt_param_style.rs`)

```
error[E0308]: mismatched types
 --> tests/compile_fail/wrong_bolt_param_style.rs:7:26
  |
7 |     let _ = fasten_param(Bolt::<SizeM6, Brass, L15>::new());
  |             ------------ ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ expected `Bolt<SizeM8, Steel, L15>`, found `Bolt<SizeM6, Brass, L15>`
  |             |
  |             arguments to this function are incorrect
  |
  = note: expected struct `Bolt<SizeM8, exp05_characteristics_encoding::param::Steel, L15>`
             found struct `Bolt<SizeM6, exp05_characteristics_encoding::param::Brass, L15>`
```

**Judgement: excellent** — the best of the experiment. Expected vs found shows *every* wrong characteristic at once, in one error. (The length-generic variant, `wrong_length_param_generic.stderr`, is the same shape with `_` in the generic slot: `expected Bolt<SizeM8, Steel, _>, found Bolt<SizeM8, Brass, L20>`.)

### 2.3 Wrong parameterized bolt into the marker-style requirement (`wrong_bolt_bridged.rs`)

```
error[E0277]: the trait bound `Bolt<SizeM6, exp05_characteristics_encoding::param::Brass, L15>: Req001FasteningBolt` is not satisfied
 --> tests/compile_fail/wrong_bolt_bridged.rs:8:27
  |
8 |     let _ = fasten_marker(Bolt::<SizeM6, Brass, L15>::new());
  |             ------------- ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ the trait `M8` is not implemented for `Bolt<SizeM6, exp05_characteristics_encoding::param::Brass, L15>`
  |
help: the trait `M8` is implemented for `Bolt<SizeM8, M, L>`
 --> src/bridge.rs
  |
  | impl<M: Material, L: Length> characteristics::M8 for Bolt<SizeM8, M, L> {}
  = note: required for `Bolt<SizeM6, ..., L15>` to implement `Req001FasteningBolt`
```

**Judgement: good, arguably the most instructive** — it names the missing characteristic *and* shows which size would satisfy it (`M8 is implemented for Bolt<SizeM8, M, L>`). Same duplication noise as 2.1 (a second E0277 for `Steel`).

### 2.4 Transposed type parameters caught by kind traits (`transposed_params.rs`)

```
error[E0277]: the trait bound `exp05_characteristics_encoding::param::Steel: Size` is not satisfied
 --> tests/compile_fail/transposed_params.rs:7:13
  |
7 |     let _ = Bolt::<Steel, SizeM8, L15>::new();
  |             ^^^^^^^^^^^^^^^^^^^^^^^^^^ the trait `Size` is not implemented for `exp05_characteristics_encoding::param::Steel`
  |
help: the following other types implement trait `Size`
  | impl Size for SizeM6 {}
  | impl Size for SizeM8 {}
  ...
```

**Judgement: very clear** — "Steel is not a Size" reads almost like the modelling mistake itself. Without the kind traits this program compiles silently; that hazard is why the kind traits exist.

---

## 3. Candidate FINDINGS.md entries (ready to paste)

> **F-EXP05-1 — Stable `macro_rules!` cannot synthesize struct names, so the marker-trait encoding keeps one hand-written line per combination.**
> What couldn't be expressed: a macro taking the value lists of each dimension and generating the full cross product of combination structs with concatenated names (`M8SteelBolt15mm`). Stable Rust has no identifier concatenation in `macro_rules!` (the `paste` crate is a forbidden dependency; `macro_metavar_expr_concat` is nightly).
> Cost: the macro reduces the marker catalogue from ~5 LOC to 1 LOC per combination but the per-combination line remains, so marker-encoding LOC stays O(product of dimension sizes) — 24 lines for 4×2×3, and every new value or dimension multiplies it.
> Workaround: macro takes an explicit `Name: Size, Material, Length;` line per combination (`exp05` `src/marker.rs`); or prefer the type-parameter encoding, which needs no macro at all.

> **F-EXP05-2 — Unbounded type parameters accept transposed arguments silently.**
> What couldn't be expressed (without extra machinery): that the slots of `Bolt<Size, Material, Length>` only accept values of the right kind. `Bolt<Steel, SizeM8, L15>` compiles cleanly if the parameters are unbounded, and the mistake only surfaces far away, if ever.
> Cost: one "kind" trait per dimension (`Size`, `Material`, `Length`) plus one one-line impl per characteristic value, and the kind bounds must be repeated on the struct and its impls.
> Workaround adopted: kind traits as bounds on the struct definition, turning transposition into a clear construction-site error ("the trait `Size` is not implemented for `Steel`") — `exp05` `src/param.rs`, `tests/compile_fail/transposed_params.rs`.

> **F-EXP05-3 — A default type parameter makes adding a dimension non-breaking, but silently narrows existing blanket impls.**
> What worked: adding thread pitch as `Bolt<S, M, L, P = Pitch125>` keeps every existing `Bolt<S, M, L>` spelling compiling (`exp05` `src/pitch_demo.rs`).
> The complication: existing bridging impls written as `impl<M, L> M8 for Bolt<SizeM8, M, L>` silently become impls for the *default pitch only*; bolts of other pitches lose their marker traits with no warning or error. This is quiet model corruption, not a compile error.
> Workaround: whenever a defaulted parameter is added, audit every blanket impl mentioning the type and rewrite it with an explicit parameter (`impl<M, L, P> M8 for Bolt<SizeM8, M, L, P>`). A grep for `for Bolt<` makes the audit mechanical.

> **F-EXP05-4 — Bridging the two encodings works cleanly but only one way.**
> What worked: ten one-line blanket impls give every `Bolt<S, M, L>` the marker traits its parameters imply, so parameterized bolts satisfy marker-style requirement traits unchanged (`exp05` `src/bridge.rs`). Cost is O(number of characteristic values), and no coherence problems arise while traits and type share a crate.
> The limitation: the reverse direction does not exist — a marker struct can never satisfy a signature demanding the nominal type `Bolt<SizeM8, Steel, L15>`.
> Consequence adopted: requirements (R10) must always be written as trait bounds, never as concrete parameterized types in signatures; concrete `Bolt<…>` types are implementation detail. Then both encodings satisfy every requirement.

> **F-EXP05-5 — Marker-style trait errors are per-characteristic and duplicated; parameter-style errors are single and diff-like.**
> Not a blocker, an ergonomics observation: a wrong bolt against a marker bound emits one E0277 per missing characteristic (each restating the whole requirement trait), with a truncated "types that do implement it" list; a wrong bolt against a concrete `Bolt<…>` parameter emits a single E0308 with expected/found showing all wrong characteristics at once. Both are readable by a modeller; the parameter one is better. See `exp05` `tests/compile_fail/*.stderr`.

---

## 4. Recommendation: **adapt** — use both, with fixed roles (mix, as R6 allows, but not freely)

R6's claim holds: both encodings work on stable Rust, and they compose via one-way bridging. But they are not equal, and the experiment supports fixed roles rather than free choice:

1. **Encode catalogues as parameterized types** (`Bolt<Size, Material, Length>` with kind traits). LOC scales with the sum, not the product, of dimension values; adding a value is 2–3 lines; adding a dimension can be non-breaking via a default parameter; no macro machinery is needed; wrong-bolt errors are the clearest of any style tested.
2. **Express requirements only as marker/requirement-trait bounds** (the R10 named-trait pattern), never as concrete `Bolt<…>` parameter types in process signatures. Bounds are the one language both encodings speak, so this keeps every type — parameterized or bespoke — eligible to satisfy a requirement.
3. **Maintain the bridging blanket impls as part of the catalogue**: one line per characteristic value, added in the same commit as the value itself (M12 cost exactly one bridge line here). When a defaulted dimension is added, rewrite all bridges with an explicit parameter (F-EXP05-3).
4. **Reserve one-struct-per-combination marker types for small, irregular sets** (a handful of named, non-orthogonal things) where the cross-product explosion can't bite and a bespoke name per thing aids readability. Do not use it for orthogonal catalogues: 24 combinations already cost 121 hand-written or 24+macro lines, and a new dimension doubles the set and renames everything.
