# EXP-02 Results: Suppliers and consumers as type-level lists

**Toolchain:** `rustc 1.98.1 (48a229cea 2026-09-01) (Homebrew)`, stable. No dependencies except `trybuild` (dev-dependency).

**What was built** (all in `src/lib.rs` unless noted):

- Minimal Peano naturals (`Zero`, `Succ<N>`, `trait Nat { const VALUE: u64 }`) plus a `nat!` token-counting macro and aliases `N0`–`N5`, `N100`.
- Type-level lists holding real values: `Nil`, `Cons<H, T>` (private fields), `trait Len { const LEN: u64 }` so the count *is* the list length.
- A `boundary` module as the privacy boundary: `Bolt` (private field, no `Clone`/`Copy`), `BoltBox<Items>` (private field), `type EmptyBoltBox = BoltBox<Nil>`, `WasteBag<Space, Contents = Nil>`, `type FullWasteBag<C> = WasteBag<Zero, C>`, and the fill helper `full_box::<N>()` — the only place bolts come into existence.
- `Supplier` implemented **only** for `BoltBox<Cons<H, T>>`; `Consumer<In>` implemented **only** for `WasteBag<Succ<S>, C>`. Both carry `#[diagnostic::on_unimplemented]` messages (stable since 1.78).
- The key test both ways: `fasten_four_chained` (hand-written chained bounds) and `fasten_four_supplyn` (recursive `SupplyN<N>` trait), plus a `ConsumeList` companion so a supplier's output can feed a consumer.
- `demo_capacity_100()`: non-generic, forces full monomorphization of fill → supply×100 → consume×100 on every `cargo build`.
- 5 positive integration tests (`tests/supplier_consumer.rs`, outside the privacy boundary) and 6 trybuild compile-fail cases (`tests/ui/`). `cargo test`: **9 passed, 0 failed** (3 unit + 1 trybuild harness covering 6 cases + 5 integration).

---

## 1. Verdicts per evaluation criterion

### 1a. Does the key test (step 6) work at all on stable? How many where-clauses for 4 bolts?

**Verdict: PASS** (variant (a) pass with complications; variant (b) clean pass).

**(a) Hand-written chained bounds — works.** Four bolts need **four** where-clauses — one per bolt taken, i.e. N clauses for N bolts:

```rust
pub fn fasten_four_chained<S>(s: S) -> ([Bolt; 4], Next4<S>)
where
    S: Supplier<Item = Bolt>,
    Next1<S>: Supplier<Item = Bolt>,
    Next2<S>: Supplier<Item = Bolt>,
    Next3<S>: Supplier<Item = Bolt>,
```

This only reads acceptably because of type aliases (`type Next1<S> = <S as Supplier>::Next;` etc.). Without them the fourth bound is
`<<<S as Supplier>::Next as Supplier>::Next as Supplier>::Next: Supplier<Item = Bolt>` — nesting grows linearly and becomes unreadable fast. The return type has the same problem (`Next4<S>` hides a 4-deep projection).

**(b) Recursive `SupplyN<N>` trait — works, and is clearly better.** **One** where-clause regardless of N:

```rust
pub fn fasten_four_supplyn<S>(s: S) -> (FourBolts, S::Rest)
where
    S: SupplyN<N4, Taken = FourBolts>,
```

The trait is implemented with a base case (`SupplyN<Zero> for any S`, takes nothing) and a recursive case (`SupplyN<Succ<N>>` where `S: Supplier, S::Next: SupplyN<N>`); the two impls don't overlap because the `N` parameter differs, so coherence is satisfied with no tricks. It returns the taken items as a real Cons-list of values plus the depleted supplier, exactly as the brief asked. The `Taken = FourBolts` equality bound doubles as the "items must be bolts" requirement. Both variants pass their integration tests (`tests/supplier_consumer.rs::fasten_four_with_chained_bounds`, `::fasten_four_with_supplyn`).

### 1b. Error messages for the compile-fail cases — would a modeller understand "the box is empty"?

**Verdict: PASS WITH COMPLICATIONS.** It depends entirely on *which path* the mistake takes:

- **Through a generic bound** (passing an empty box to a process, or asking `SupplyN` for more than the box holds): **excellent**, because `#[diagnostic::on_unimplemented]` rewrites the first line to `` `BoltBox<Nil>` cannot supply anything: it is empty ``. A modeller would understand this immediately. See errors 2, 3, 4 below.
- **Direct method call on the exhausted value** (`empty.supply()`): **poor**. This takes the E0599 "no method named ..." path, which *bypasses* `on_unimplemented` entirely, never says "empty", and offers a misleading "did you mean `supply_n`" suggestion. See errors 1 and 5 below.

Consequence for the real project: model code should reach suppliers/consumers through generic processes (which R12 requires anyway), because that is the path with the good errors.

### 1c. Compile time with capacity 100

**Verdict: PASS.** `cargo clean && time cargo build` three times (debug, and the build includes the non-generic `demo_capacity_100()`, so the compiler really does the 100-deep fill/supply/consume work): **2.33 s / 2.70 s / 2.55 s — median 2.58 s**. Indistinguishable from a trivial crate; compile time is a non-issue at this size.

Related measurements:

- **Recursion limit:** the default (128) is *sufficient at capacity 100*. A probe at capacity **130 fails** with `E0275: overflow evaluating the requirement ...: Bolts` — the error itself suggests the fix (`consider increasing the recursion limit by adding a #![recursion_limit = "256"]`). The crate now sets `#![recursion_limit = "256"]` for headroom. So the practical ceiling with the default limit sits between 100 and 130 items, and each doubling of the limit roughly doubles the ceiling.
- **Long type names:** at capacity 130 the compiler refused to print the full type inline: `the full name for the type has been written to 'target/debug/.../exp02_supplier_consumer-....long-type-12623062041893530642.txt'`. Expect this in any error involving a big box.

### 1d. The rest of the build steps (1–5, 7)

**Verdict: PASS**, with one design deviation:

- Peano numbers, `Cons`/`Nil`, private-constructor `Bolt`, private-field `BoltBox<Items>`, `EmptyBoltBox` alias, `Supplier` only for `Cons`, `Consumer` only while space remains, fill helper inside the boundary: all as specified, all compiling, all exercised by tests.
- Count-equals-contents holds by construction: `BoltBox::<Items>::COUNT` is `Items::LEN` (recursive `Len` trait per R7); verified for 0, 2, 5 and 100.
- trybuild: 6 compile-fail cases pass, including the two required ones (supply from empty, consume into full) in both method-call and generic-bound form, plus taking 5 from a box of 4, plus constructing a `Bolt` from outside the boundary (E0451: field `_private` ... is private).
- **Deviation:** the brief sketches `WasteBag<Space>`, but a bag with *only* a space parameter cannot keep the real objects it consumes (their types accumulate). It became `WasteBag<Space, Contents = Nil>` — consuming does `WasteBag<Succ<S>, C> -> WasteBag<S, Cons<In, C>>`. The default parameter keeps `new_waste_bag::<N100>()` writable. Recorded as finding F5.

---

## 2. Representative compiler errors (verbatim) with readability judgements

**Error 1 — `empty.supply()` as a direct method call** (`tests/ui/supply_from_empty_box.stderr`):

```
error[E0599]: no method named `supply` found for struct `BoltBox<Items>` in the current scope
 --> tests/ui/supply_from_empty_box.rs:8:32
  |
8 |     let (_bolt, _rest) = empty.supply();
  |                                ^^^^^^
  |
help: there is a method `supply_n` with a similar name
  |
8 |     let (_bolt, _rest) = empty.supply_n();
  |                                      ++
```

*Judgement: poor — never says the box is empty, and the `supply_n` suggestion would send a modeller in the wrong direction.*

**Error 2 — empty box passed to a generic process** (`tests/ui/supply_from_empty_box_generic.stderr`, first of two identical diagnostics):

```
error[E0277]: `BoltBox<Nil>` cannot supply anything: it is empty (or not a supplier at all)
  --> tests/ui/supply_from_empty_box_generic.rs:14:26
   |
14 |     let _ = use_one_bolt(empty);
   |             ------------ ^^^^^ this supplier has nothing left to supply
   |             |
   |             required by a bound introduced by this call
   |
   = help: the trait `exp02_supplier_consumer::Supplier` is not implemented for `BoltBox<Nil>`
   = note: a `BoltBox<Cons<..>>` still contains bolts and can supply; `EmptyBoltBox` (= `BoltBox<Nil>`) is empty and cannot
```

*Judgement: excellent — the first line literally says the box is empty; a modeller needs no Rust knowledge to read it. (Minor noise: rustc emits this diagnostic twice at the same call site.)*

**Error 3 — taking five bolts from a box of four via `SupplyN`** (`tests/ui/take_five_from_four.stderr`):

```
error[E0277]: `BoltBox<Nil>` cannot supply anything: it is empty (or not a supplier at all)
 --> tests/ui/take_five_from_four.rs:8:27
  |
8 |     let (_taken, _rest) = supply_n::<N5, _>(bx);
  |                           ^^^^^^^^^^^^^^^^^^^^^ this supplier has nothing left to supply
  ...
  = note: required for `BoltBox<Nil>` to implement `SupplyN<Succ<exp02_supplier_consumer::Zero>>`
```

*Judgement: very good — it even shows the arithmetic: the box would be down to `BoltBox<Nil>` with `Succ<Zero>` (one) request still to satisfy.*

**Error 4 — consuming into a full bag through a generic process** (`tests/ui/consume_into_full_bag_generic.stderr`, abridged to one of the two identical diagnostics):

```
error[E0277]: `WasteBag<exp02_supplier_consumer::Zero, Cons<exp02_supplier_consumer::Bolt, Nil>>` cannot consume a `exp02_supplier_consumer::Bolt`: it is full (or not a consumer of `exp02_supplier_consumer::Bolt`)
  --> tests/ui/consume_into_full_bag_generic.rs:16:24
   |
16 |     let _bag = discard(bag, bolt); // no space left
   |                ------- ^^^ this consumer has no space left
   |
   = note: a `WasteBag<Succ<..>, _>` still has space; `WasteBag<Zero, _>` is full and cannot consume
help: the trait `exp02_supplier_consumer::Consumer<...>` is not implemented for `WasteBag<exp02_supplier_consumer::Zero, Cons<..., Nil>>`
      but it is implemented for `WasteBag<Succ<_>, _>`
```

*Judgement: good — "it is full" and "no space left" are plain English; the fully-qualified type paths (`exp02_supplier_consumer::Zero`) add clutter a modeller must skip over.*

**Error 5 — consuming into a full bag as a direct method call** (`tests/ui/consume_into_full_bag.stderr`):

```
error[E0599]: no method named `consume` found for struct `WasteBag<Space, Contents>` in the current scope
  --> tests/ui/consume_into_full_bag.rs:11:20
   |
11 |     let _bag = bag.consume(bolt); // no space left
   |                    ^^^^^^^
   |
note: there's an earlier shadowed binding `bag` of type `WasteBag<Succ<exp02_supplier_consumer::Zero>>` that has method `consume` available
...
 9 |     let bag = bag.consume(bolt); // fills the bag
   |         --- earlier `bag` shadowed here with type `WasteBag<exp02_supplier_consumer::Zero, Cons<Bolt, Nil>>`
help: there is a method `consume_list` with a similar name
```

*Judgement: mediocre — the shadowed-binding note actually shows the bag going from `Succ<Zero>` space to `Zero` space, which a trained eye can decode as "it filled up", but the headline ("no method named `consume`") and the `consume_list` suggestion mislead.*

**Error 6 — capacity 130 with the default recursion limit** (probe, not left in the crate):

```
error[E0275]: overflow evaluating the requirement `Succ<Succ<Succ<Zero>>>: Bolts`
    = help: consider increasing the recursion limit by adding a `#![recursion_limit = "256"]` attribute to your crate (`exp02_supplier_consumer`)
    = note: 126 redundant requirements hidden
    = note: the full name for the type has been written to 'target/debug/deps/exp02_supplier_consumer-....long-type-....txt'
```

*Judgement: fine for a developer (the fix is stated verbatim); baffling for a modeller, who has no reason to connect "recursion limit" with "the box is too big".*

---

## 3. Candidate FINDINGS.md entries (ready to paste)

- **F1 — Multi-use suppliers need a recursive helper trait; chained bounds don't scale.** Using one supplier N times inside a generic process works on stable Rust, but hand-written chained bounds cost one where-clause per item (`S: Supplier, S::Next: Supplier, <S::Next as Supplier>::Next: Supplier, …`) with linearly deepening projections — 4 clauses for 4 bolts, unreadable beyond a handful without `NextK<S>` type aliases. Workaround (adopted): a recursive `SupplyN<N>` trait (base case at `Zero`, recursive case at `Succ<N>` bounded by `S: Supplier, S::Next: SupplyN<N>`) reduces any N to **one** where-clause and returns the items as a Cons-list of real values plus the depleted supplier. Cost: one extra trait per "repeat this boundary action N times" pattern (a matching `ConsumeList` was needed for consumers). (EXP-02, R12.)

- **F2 — `#[diagnostic::on_unimplemented]` (stable since 1.78) turns capacity errors into plain English — but only on the trait-bound path.** With the attribute on `Supplier`/`Consumer`, passing an empty box to a process yields `` `BoltBox<Nil>` cannot supply anything: it is empty `` — exactly the "box is empty" message a modeller needs. However, a *direct method call* on the exhausted value (`empty.supply()`) takes the E0599 "no method named `supply`" path, which bypasses the attribute, never mentions emptiness, and suggests an unrelated similarly-named method. Cost: none at compile time. Convention to adopt: put `on_unimplemented` on every boundary trait, and route model code through generic processes (required by R12 anyway) so mistakes hit the good path. (EXP-02, R4/R12.)

- **F3 — The recursion limit caps supplier capacity; the default (128) allows 100 but not 130.** `full_box`/`SupplyN` at capacity 100 compile with the default `recursion_limit`; capacity 130 fails with `E0275: overflow evaluating the requirement` (the error text itself names the fix). Workaround: `#![recursion_limit = "256"]` in the crate root (adopted); each doubling roughly doubles the ceiling. Also at that size the compiler stops printing types inline and writes them to a `long-type-….txt` side file, so expect opaque type names in errors involving large boxes. Compile time is *not* the constraint: a cold debug build that fully monomorphizes fill/supply×100/consume×100 takes a median 2.58 s. (EXP-02, R12/R14.)

- **F4 — Peano lists are fine at catalogue sizes but the aliases need generating.** Writing `N100` required a token-counting `macro_rules!` (`nat!(x x … x)` with 100 tokens); anything beyond a few hundred should generate aliases mechanically (EXP-01's job). Count-equals-contents genuinely holds by construction: the box's `COUNT` is the list's recursive `LEN`, so they cannot disagree. (EXP-02, R7/R12.)

- **F5 — `WasteBag<Space>` alone cannot keep what it consumes; it needs a contents parameter too.** R12 says a consumer keeps the real objects it has consumed, but their types accumulate as it consumes, so remaining-space-only (`WasteBag<Space>`) can't store them. Adopted shape: `WasteBag<Space, Contents = Nil>`, where consuming maps `WasteBag<Succ<S>, C> → WasteBag<S, Cons<In, C>>`. The default type parameter keeps construction ergonomic (`new_waste_bag::<N100>()`). Consequence: a consumer's *full* type mentions everything it ever consumed, which makes for long error messages; `type FullWasteBag<C> = WasteBag<Zero, C>` helps readability. (EXP-02, R12.)

- **F6 — Exposing a boundary fill function publicly needs a sealed helper trait.** `full_box<N>()` must name the recursive fill machinery in its public where-clause without letting outside code implement it (which would break "no resources from nothing"). Workaround: the classic sealed-trait pattern (`pub trait FillSealed: sealed::Sealed`, with `Sealed` in a private module implemented only for bolt lists). Minor boilerplate, no hole found: constructing a `Bolt` outside the boundary fails with `E0451: field `_private` … is private` (trybuild-verified). (EXP-02, R1/R12.)

- **F7 — Minor diagnostic noise: rustc emits the unsatisfied-bound error twice at one call site.** Each compile-fail through a generic call produced the identical E0277 twice (once for the argument, once for the call expression). Harmless but adds bulk; no workaround. (EXP-02, R4.)

---

## 4. Recommendation

**ADOPT, with these adaptations:**

1. Adopt the type-level-list supplier/consumer design itself: it works end to end on stable, the count cannot drift from the contents, exhausted/full states are genuinely distinct types, and compile time at capacity 100 is negligible. The fallback design (array of `Option` slots) is not needed.
2. Make the recursive helper trait (`SupplyN`, `ConsumeList`) the standard way processes use a supplier/consumer more than once; forbid chained `S::Next` bounds beyond depth ~2 in project conventions (F1).
3. Require `#[diagnostic::on_unimplemented]` on every boundary trait, with messages phrased for modellers ("it is empty", "no space left"), and require model code to go through generic processes so errors hit that path (F2).
4. Set `#![recursion_limit]` explicitly in every model crate and document the capacity ceiling it buys; treat E0275 as "the box is too big for the current limit" in the project's troubleshooting notes (F3).
5. Amend the R12 sketch of consumers from `WasteBag<Space>` to `WasteBag<Space, Contents = Nil>` (F5).
