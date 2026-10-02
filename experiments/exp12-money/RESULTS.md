# EXP-12 RESULTS: Money as a conserved dimension (candidate R19)

**Toolchain:** `rustc 1.98.1 (48a229cea 2026-09-01) (Homebrew)`, stable channel, `cargo 1.98.1`,
macOS (Darwin 24.6.0, Apple Silicon).
**Dependencies:** `model-core` by path (per the EXP-10..12 protocol update; read-only, plus its
`test-support` feature under `[dev-dependencies]` only); `trybuild` as the one external
dev-dependency.
**Compile time:** `cargo clean && time cargo build` × 3 (includes building model-core from
source): 1.44 s / 1.45 s / 1.43 s — **median 1.44 s**. A non-issue.
**Suite status:** `cargo build` and `cargo clippy --all-targets` clean (0 warnings);
`cargo test` green: 7 unit tests, 1 trybuild harness (5 ui cases with pinned `.stderr`),
2 passing doc-tests, 4 `compile_fail` doc-tests. The `postmono-demo/` sub-crate holds the
deliberately-broken binaries behind the `compile_fail` cases, proving each fails for the
intended E0080 (the F-001 "fails for *any* reason" trap) and that `cargo check` passes them all.

**Headline:** money needed **zero new model-core machinery**. Every build item below is pure
downstream convention over the existing kernel: the open `Unit` kind trait, `Qty`/`split`/
`combine`, `container_resource!`, `draw_process!` (reused verbatim for `Account => Money`),
`Consumer`/`Supplier` with `Next = Self`, and the generic access processes. The deliverable for
the instructions is one R7 table row (per-currency) plus the R19 conventions (§4).

---

## 1. Verdicts per evaluation criterion

### C1. Currency units in the R7 style; mixing is a type error — **pass**

One struct + one `impl Unit` line per currency (`units::Pence`, `units::EuroCents`), downstream,
against model-core's open `Unit` kind trait. The existing unit-generic `split`/`combine` serve
money unchanged (`tests::qty_split_and_combine_work_for_currency_units`). Mixing currencies in
`combine` is a type-check-time E0308 (error (a) below), pinned by trybuild
(`tests/ui/mix_currencies.rs`). There is deliberately no shared "money" unit: each currency is
its own dimension, so cross-currency sums are unrepresentable rather than merely discouraged.

### C2. `Account<const BALANCE: u64>` with `draw_funds`; overspending is a compile error — **pass** (inherits F-001, as expected)

`Account`, `Money`, `Euros` are `container_resource!` invocations; `draw_funds` is a verbatim
`draw_process! { pub fn draw_funds: Account => Money }` — the R15 kernel generator fits money
with no adaptation. Overspending (drawing 6000 from 5000) is a genuine E0080 at
monomorphization with the custom message leading and the caller's line in the instantiation
note (error (d)); `cargo check` passes the violating program (transcript in `postmono-demo/`),
exactly the known F-001 caveat. Regressions are rustdoc `compile_fail` doc-tests per the R4
policy. `deposit` (the conserving combine back into the account) and `split_money`
(change-giving) complete the R3 set; the caller states totals per F-022. F-030 also carries
over: the modeller restates the running balance per draw, compiler-checked
(`tests::account_draws_down_and_balances`).

### C3. Payment as a process chain: dual-role `Vendor`, conserving `purchase`, change as a split — **pass**

`goods::Vendor` is one boundary object implementing **both** `Consumer<Money<350>>` (payment
sink, `Next = Self`, discards per F-029) and `Supplier` (goods source, `Item = Spanner`,
`Next = Self`) — the EXP-09 atmosphere pattern transplanted to commerce with no friction.
`purchase` is an ordinary generic process (`V: Consumer<Money<PRICE>>, V::Next: Supplier` —
depth-2 chaining, within the R12 limit): it const-asserts `TENDERED == PRICE + CHANGE`, splits
the tendered cash (R3), pays through `send_to`, takes goods through `take_one` (the F-015
generic-process discipline), and returns goods + change + vendor. Money conserves (price exits
at the boundary, change returns); goods conserve (vendor stock → caller → `SpannerRack` sink,
the F-035 production-legal consumer). `Money<0>` (exact payment, zero change) is a real empty
state that still needs accounting
(`tests::exact_payment_leaves_zero_change_which_is_still_accounted`). The whole conservation
regime covers money unchanged: abandoned change trips the tripwire naming `Money<150>`
(`tests::abandoned_change_trips_the_tripwire`, F-027/F-032 confirmed for money).

**Design result worth adopting:** implementing `Consumer` **only at the vendor's exact price**
(a concrete `Money<350>`, the price stated once as a named const) moves wrong-amount and
wrong-currency payment to **type-check time** — visible to editors and trybuild, unlike the
post-mono conservation asserts — and the errors name the correct price (errors (b), (c)).

### C4. Currency `exchange` at a stated integer rate, const-asserted — **pass**, documented as value-equivalence

`exchange_gbp_to_eur` states the rate as integer consts (`GBP_EUR_RATE_NUM = 117`,
`GBP_EUR_RATE_DEN = 100`) and const-asserts `OUT * DEN == IN * NUM`. Honesty, both halves
documented on the process:

- **Not single-dimension conservation.** The GBP dimension loses `IN` while the EUR dimension
  gains `OUT`; the process mints and destroys per-currency amounts — exactly what R1 forbids
  inside the model — so it is a **boundary process** threading a `Bureau` placeholder object,
  the same licence as a draw from the atmosphere (R15/F-031).
- **Integer honesty: nothing ever rounds.** The assert is exact multiplication — no division
  anywhere — so an amount with no exact exchange at the rate has **no `OUT` that compiles**
  (333 pence × 117/100 = 389.61: both 389 and 390 fail, error (e)). The working idiom is
  split-then-exchange: split off the largest exchangeable sub-amount (here 300 → 351 cents) and
  the remainder stays conserved in GBP
  (`tests::inexact_amounts_split_first_and_keep_the_remainder`). Rounding, if a model ever
  wants it, would have to be modelled explicitly as a fee/spread output — it cannot happen
  silently.

### C5. Compile-fail coverage: mixing, overspending, wrong currency — **pass**

Per the R4/F-003 split:

| Mistake | Fires at | Vehicle |
|---|---|---|
| mixing currencies in `combine` | type check (E0308) | trybuild `mix_currencies.rs` |
| paying the wrong currency | type check (E0308) | trybuild `pay_wrong_currency.rs`, `purchase_wrong_currency.rs` |
| paying the wrong amount | type check (E0308) | trybuild `pay_wrong_amount.rs` |
| purchase bound states wrong price | type check (E0277 + on_unimplemented) | trybuild `purchase_wrong_price.rs` |
| overspending the account | monomorphization (E0080) | rustdoc `compile_fail` on `draw_funds` + `postmono-demo/overspend` |
| wrong change stated | monomorphization (E0080) | rustdoc `compile_fail` on `purchase` + `postmono-demo/wrong_change` |
| wrong deposit total | monomorphization (E0080) | rustdoc `compile_fail` on `deposit` |
| inexact exchange | monomorphization (E0080) | rustdoc `compile_fail` on `exchange_gbp_to_eur` + `postmono-demo/inexact_exchange` |

### Key question: does money need ANY new model-core machinery? — **No.**

Nothing was added, worked around, or wished for in the kernel. Specifically: the `Unit` trait
is open to downstream currencies (C1); `container_resource!` + `draw_process!` fit accounts and
cash verbatim (C2); `Consumer`/`Supplier`/`send_to`/`take_one` fit vendors (C3); the exchange
process is 15 lines of downstream boundary code (C4). Money is **an R7 table row plus
downstream conventions** — the conventions being exactly what R19 should state (recommendation,
§4).

---

## 2. Representative compiler errors, verbatim, with readability judgements

**(a) Mixing currencies in `combine` (E0308, type check — trybuild-pinned):**

```
error[E0308]: mismatched types
  --> tests/ui/mix_currencies.rs:13:47
   |
13 |     let total: Qty<217, Pence> = combine(gbp, eur);
   |                                  -------      ^^^ expected `Qty<_, Pence>`, found `Qty<117, EuroCents>`
   |                                  |
   |                                  arguments to this function are incorrect
   |
   = note: expected struct `Qty<_, Pence>`
              found struct `Qty<117, EuroCents>`
```

*Judgement:* excellent — reads directly as "you cannot add euro cents to pence", with the
magnitude visible as a decimal; same shape as the known-good unit-mismatch errors (F-027).

**(b) Paying the wrong amount (E0308, type check — the single-impl inference path):**

```
error[E0308]: mismatched types
  --> tests/ui/pay_wrong_amount.rs:12:40
   |
12 |     let vendor = send_to(new_vendor(), underpayment);
   |                  -------               ^^^^^^^^^^^^ expected `350`, found `300`
   |
   = note: expected struct `Money<350>`
              found struct `Money<300>`
```

*Judgement:* the best money error in the experiment — because `Vendor` has exactly one
`Consumer` impl, inference resolves the expected payment from it and the error **names the
correct price**: "expected `Money<350>`, found `Money<300>`" is "the spanner costs 350, you
offered 300". Wrong currency takes the identical shape ("expected `Money<350>`, found
`Euros<350>`") — see `tests/ui/pay_wrong_currency.rs`.

**(c) Purchase bound stating the wrong price (E0277 + on_unimplemented, type check):**

```
error[E0277]: `Vendor` cannot consume a `Money<300>`: it is full, or it does not accept this kind of resource
  --> tests/ui/purchase_wrong_price.rs:15:48
   |
15 |     let (spanner, change, vendor) = purchase::<_, 500, 300, 200>(new_vendor(), tendered);
   |                                                ^ this consumer has no space left for this resource
   |
   = note: a consumer with space left is e.g. `WasteBag<Succ<..>, _>`; its full state (`WasteBag<Zero, _>`) is a distinct resource that cannot consume (R12)
help: the trait `Consumer<Money<300>>` is not implemented for `Vendor`
      but trait `Consumer<Money<350>>` is implemented for it
```

*Judgement:* good — model-core's on_unimplemented message leads, and rustc's help **lists the
one impl that exists, so the right price is in the error** (the F-029 observation, now serving
prices). Two blemishes: the trait's generic `WasteBag` note is irrelevant to a vendor (the
attribute's message is fixed at the trait, not per impl), and the identical E0277 is emitted
twice at the call site (known F-009 noise).

**(d) Overspending the account (E0080 at monomorphization — `cargo check` passes this program):**

```
error[E0080]: evaluation panicked: overspend: money conservation violated in draw_funds (R15/R19): TAKE + LEFT must equal FULL - is the draw larger than the account's balance?
   --> .../library/core/src/panic.rs:62:9
    |
    = ... evaluation of `exp12_money::money::processes::draw_funds::<6000, 0, 5000>::{constant#0}` failed here
...
note: the above error was encountered while instantiating `fn draw_funds::<6000, 0, 5000>`
 --> src/bin/overspend.rs:9:25
  |
9 |     let (cash, empty) = draw_funds::<6000, 0, 5000>(account);
```

*Judgement:* the custom message leads and the instantiation note lands on the modeller's own
line with decimal magnitudes — F-027 quality, F-001 visibility (primary span in
`core/src/panic.rs`; invisible to `cargo check`/editors — verified: `cargo check` of
`postmono-demo` passes, `cargo build` fails).

**(e) Inexact exchange (E0080 — integer honesty):**

```
error[E0080]: evaluation panicked: exchange is not exact at the stated rate (R19): OUT euro cents * DEN must equal IN pence * NUM - integer exchange never rounds; split off an exchangeable amount first and keep the remainder in GBP
...
note: the above error was encountered while instantiating `fn exchange_gbp_to_eur::<333, 390>`
  --> src/bin/inexact_exchange.rs:14:28
```

*Judgement:* good — the message says what to do (split first, keep the remainder), and the note
carries the offending numbers. The modeller must still compute the exchangeable sub-amount by
hand (the F-022/F-030 cost, unchanged).

**(f) Wrong change in a composed process (E0080 cascade — minor noise):** stating
`purchase::<_, 500, 350, 200>` emits **two** E0080s: the `purchase` assert (instantiation note
at the user's line) and then the inner `split_money::<500, 350, 200>` assert, whose
instantiation note points at `purchase`'s body **inside the library**, not the user's call.
*Judgement:* the first error is exactly right; the second is a confusing trailer a modeller
must learn to skip — a composed-process echo of F-009's duplicate-error noise.

---

## 3. Candidate FINDINGS entries

### F-042 (candidate) — Money needs no kernel support: each currency is its own dimension, and an exact-price `Consumer` impl turns wrong payments into type-check-time errors that name the right price

**What couldn't be expressed:** nothing — EXP-12 added **zero** model-core machinery. A
currency at the quantity level is one downstream `Unit` impl (the trait is open); at the
resource level, `container_resource!` + `draw_process!` fit cash and accounts verbatim
(`draw_funds: Account => Money`), so overspending is the standard R15 overdraw E0080 (F-001/
F-027 apply unchanged). A vendor is one boundary object implementing both `Consumer<money>`
and `Supplier` (goods), `Next = Self` (F-029); purchase is an ordinary conserving process with
change-giving as an R3 split.

**Design result:** implement `Consumer` **only at the vendor's exact price** (a concrete
`Money<350>`, the price stated once as a named const). Wrong amount and wrong currency then
fail at **type-check time** (editor- and trybuild-visible, unlike conservation asserts): with a
single impl, inference yields `expected Money<350>, found Money<300>` — the right price in the
error — and through a priced generic bound, the E0277 help lists the implemented impl, again
naming the price. Cost: one impl per (vendor, priced item); a multi-product vendor is one impl
per price point.

**What it cost:** nothing beyond known findings. Minor noise: a violated assert inside a
composed process (`purchase` → `split_money`) emits a second E0080 whose instantiation note
points inside the library, not at the user's line; and the `Consumer` trait's
on_unimplemented note (phrased for waste bags) is irrelevant when the consumer is a vendor —
the attribute's message is fixed at the trait, not per impl.

**Workaround adopted:** none needed; the per-currency-dimension rule and exact-price-impl
pattern become R19 conventions.

**Evidence:** `experiments/exp12-money/` (`src/lib.rs`; `tests/ui/*.rs` + pinned `.stderr`;
RESULTS.md §2 (a)–(c), (f)).

### F-043 (candidate) — Currency exchange is value-equivalence at a stated integer rate, confined to the boundary; integer arithmetic makes silent rounding unrepresentable

**What couldn't be expressed:** exchange as a conserving in-model process — correctly so. An
exchange destroys an amount in one currency dimension and mints the equivalent in another,
which is R1-forbidden inside the model; it is therefore a **boundary process** (R15/F-031
licence, like drawing from the atmosphere), threading a placeholder boundary object (`Bureau`),
with the rate stated once as integer consts and every exchange const-asserted
`OUT * DEN == IN * NUM` (E0080 on violation, F-001 visibility).

**What it cost / gained:** the assert is exact multiplication — no division, so **no rounding
can happen silently**: an amount with no exact exchange at the rate has no `OUT` that compiles
(333 pence at 117/100 rejects both 389 and 390 euro cents). The working idiom is
split-then-exchange: split off the largest exchangeable sub-amount (R3) and the remainder stays
conserved in the original currency; the modeller computes that sub-amount by hand (the
F-022/F-030 cost). A model that wants rounding/spread must make it an explicit fee output — a
feature, not a limitation.

**Workaround adopted:** none; documented as the R19 exchange convention.

**Evidence:** `experiments/exp12-money/src/lib.rs` (`exchange_gbp_to_eur`, its `compile_fail`
doc-test); `postmono-demo/src/bin/inexact_exchange.rs`; RESULTS.md §2 (e).

---

## 4. Recommendation: **adopt** — R19 as downstream conventions plus one R7 table row; no model-core changes

### Proposed R7 table entry

Add one row to the base-unit table (R7):

```
| Money            | minor currency unit   | default |
```

with this bullet added to the notes under the table:

- Money is one dimension **per currency** (R19): each currency is its own unit type in integer
  minor units (pence, euro cents), and currency unit types never mix — cross-currency
  arithmetic is a type error, like adding grams to millimetres. There is no shared "money"
  unit and no conversion factor between currencies; conversion is only the R19 boundary
  exchange process.

### Proposed R19 wording

> ### R19. Money as a conserved dimension
> - **Each currency is its own dimension.** Money follows R7: integer minor units (pence, euro
>   cents), one unit type per currency, currencies never mixing. A new currency is one unit
>   type plus one `impl Unit` line (quantity level) or one sealed money resource type (R15
>   container level) in the modelling crate; no library machinery is needed.
> - **Money is held and moved with the R15 machinery unchanged.** Cash in hand is a sealed
>   const-magnitude resource (`Money<const PENCE: u64>`); money at rest is a quantity container
>   (`Account<const BALANCE: u64>`) drawn down by a draw process, so overspending is the
>   standard overdraw compile error (the R4 post-monomorphization caveat applies). A deposit is
>   the conserving combine back into the container; change-giving is an R3 split.
> - **Payment is a process** (reaffirming R12 strictness: a supplier or consumer step takes
>   nothing but itself and the item). A vendor is a boundary object that is both
>   `Consumer<money>` and a goods source; a purchase process splits the tendered cash into
>   price and change, pays the vendor, and takes the goods, with money and goods each conserved
>   in their own dimension (`TENDERED == PRICE + CHANGE`, const-asserted).
> - **Vendors implement `Consumer` only at their exact price**, stated once as a named const.
>   Paying the wrong amount or the wrong currency then fails at **type-check time** (visible to
>   editors and trybuild, unlike conservation asserts), and the error names the correct price:
>   a single impl makes inference report `expected Money<350>, found Money<300>`, and a priced
>   generic bound makes E0277's help list the implemented price.
> - **Currency exchange is value-equivalence at a stated integer rate, not single-dimension
>   conservation.** An exchange process destroys an amount in one currency dimension and mints
>   the equivalent in another — what R1 forbids inside the model — so it is legal **only as a
>   boundary process**, always a placeholder (R12), threading its boundary object by value. The
>   rate is stated as integer consts `NUM`/`DEN` and every exchange is const-asserted
>   `OUT * DEN == IN * NUM`. The assert is exact multiplication, so nothing ever rounds
>   silently: an amount with no exact exchange at the rate has no `OUT` that compiles — split
>   off an exchangeable sub-amount (R3) and the remainder stays conserved in the original
>   currency. A model that wants a rounding loss or spread must model it as an explicit fee
>   output.
