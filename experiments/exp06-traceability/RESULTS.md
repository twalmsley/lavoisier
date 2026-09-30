# EXP-06: Requirements traceability report — RESULTS

**Tests:** R10 — that requirement links can be extracted mechanically.

**Toolchain:** `rustc 1.98.1 (48a229cea 2026-09-01) (Homebrew)`, `cargo 1.98.1`.
**Dependencies:** none (trybuild was not needed — this experiment has no compile-fail cases).
**Compile time:** `cargo clean && time cargo build` three times: 1.81 s, 1.56 s, 1.46 s → median **1.6 s** (trivial crate; traceability adds no compile cost beyond one zero-cost `const` assertion per Satisfies tag).
**Status:** `cargo build` clean (no warnings), `cargo test` passes (6 tests), `trace.sh` runs with plain `sh` from any working directory.

## What was built

- `src/lib.rs`: three requirement traits in the exact R10 pattern (`Req001FasteningBolt`, `Req002StructuralPlate`, `Req003SwarfDisposal` — named trait, doc comment starting with the ID, characteristics as supertraits, blanket impl); types `HexBoltM8`, `DrilledSteelPlate`, `SwarfBin` satisfying them; processes `fasten` and `dispose_swarf` using them as bounds; four tagged unit tests. REQ-003 deliberately has no verifying test.
- `tests/traceability_edges.rs`: two tagged integration tests, so the script must sweep more than `src/`, including one tag naming a nonexistent `REQ-999`.
- `trace.sh`: POSIX sh + awk + find only. Two awk passes (definitions/tags, then trait-name uses); prints the table below; exits 1 when there are warnings, so it can gate CI.
- Deliberate traps in the code: `Verifies: REQ-003` in an ordinary `//` comment inside a function body, in a `////` (four-slash, non-doc) comment, in a `/* */` block comment, and in a string literal; a prose doc-comment mention of REQ-003 ("See also REQ-003 …"); a module-level `//!` mention of a REQ id; a lowercase `/// verifies: REQ-002` tag.

## 1. Verdicts per criterion

### C1. Report correctness — no false positives or negatives on the edge cases: **pass**

Ground truth vs. report (output reproduced in full in §2):

| # | Edge case | In the code | Expected | Reported |
|---|-----------|-------------|----------|----------|
| 1 | Several IDs on one `Verifies:` line, trailing full stop | `/// Verifies: REQ-001, REQ-002.` on `fasten_joins_two_plates` (src/lib.rs:190) | counts for both | both — correct |
| 2 | Whitespace variation | `///    Verifies:   REQ-002 ,  REQ-001` on `plates_are_drilled_steel` (src/lib.rs:201) | counts for both | both — correct |
| 3 | Lowercase tag | `/// verifies: REQ-002` on `full_bin_is_a_distinct_type` (src/lib.rs:211) | must NOT count (convention is case-sensitive), and should be flagged | not counted; flagged as "malformed tag" — correct |
| 4 | ID in ordinary `//` comment in a function body | `// Verifies: REQ-003 …` inside `fasten` (src/lib.rs:147) | must NOT count | not counted — correct |
| 5 | Four-slash `////` comment (not a doc comment in Rust) | `//// Verifies: REQ-003` (src/lib.rs:156) | must NOT count | not counted — correct |
| 6 | Block comment | `/* Verifies: REQ-003 */` (src/lib.rs:159) | must NOT count | not counted — correct |
| 7 | String literal | `let _trap = "Verifies: REQ-003";` in `dispose_swarf` | must NOT count | not counted — correct |
| 8 | Prose mention of an ID in a doc comment | `/// … See also REQ-003 for how it must be disposed of.` on `Swarf` | must NOT count as definition, satisfaction or verification | not counted — correct |
| 9 | REQ id in module doc (`//!`) | `//! … REQ-000 does not exist …` | must NOT count | not counted — correct |
| 10 | Requirement with no verifying test | REQ-003 | listed with "(no test)" + WARNING, nonzero exit | exactly that — correct |
| 11 | Tag naming an unknown requirement | `/// Verifies: REQ-999` (tests/traceability_edges.rs:16) | WARNING, no invented table row | exactly that — correct |

Zero false positives, zero false negatives against this ground truth.

### C2. "Satisfied by" is mechanically extractable from the R10 pattern alone: **pass with complications**

The complication is structural: the R10 pattern uses a **blanket impl**, so there is no `impl Req001FasteningBolt for HexBoltM8` line anywhere to grep. Satisfaction is a fact the *trait solver* knows, not a string in the source. Two mitigations were needed, and together they work well:

1. **Trait-name uses** (bounds such as `fn fasten<B: Req001FasteningBolt>`) are greppable and give "which processes demand this requirement". Reported as `[bound use]`.
2. **Which types satisfy it** needs an explicit, greppable claim: a `/// Satisfies: REQ-NNN` doc tag on the type, **backed by a compile-checked assertion** so the tag cannot silently rot:

   ```rust
   /// A hex-head M8 steel bolt, 15 mm long.
   /// Satisfies: REQ-001
   pub struct HexBoltM8(());

   const fn assert_req001<T: Req001FasteningBolt>() {}
   const _: () = assert_req001::<HexBoltM8>();
   ```

   If `HexBoltM8` ever stops satisfying REQ-001, the crate stops compiling (verbatim error in §2a). The tag itself is prose, but the assertion keeps it honest. This is a small extension to R10, adopted in the proposed convention below.

Residual roughness (cosmetic, not correctness): the `[bound use]` rows show the matching source line, not the enclosing item's name, because grep has no scope awareness; and a bound placed in a multi-line `where` clause would surface as the where-clause line rather than the `fn` line. Acceptable for a report; noted in the convention.

### C3. The needed discipline can be written down as a workable convention: **pass**

See §4 — proposed convention. Its checkability was demonstrated: the one deliberately non-conforming tag in the crate (lowercase `verifies:`) is mechanically flagged by the script's near-miss lint, so drift from the convention is caught by the same tool that produces the report.

## 2. Sample `trace.sh` output (verbatim)

```
Requirements traceability report
================================

REQ-001  Fastening bolts must be M8 steel bolts, 15 mm long.
    defined at    src/lib.rs:32                      trait Req001FasteningBolt
    satisfied by  src/lib.rs:53                      HexBoltM8  [Satisfies tag]
    satisfied by  src/lib.rs:142                     fasten  [Satisfies tag]
    satisfied by  src/lib.rs:69                      const fn assert_req001<T: Req001FasteningBolt>() {}  [bound use]
    satisfied by  src/lib.rs:142                     pub fn fasten<B: Req001FasteningBolt, P: Req002StructuralPlate>(  [bound use]
    satisfied by  tests/traceability_edges.rs:10     const fn takes<T: Req001FasteningBolt>() {}  [bound use]
    verified by   src/lib.rs:180                     fasten_accepts_conforming_bolt
    verified by   src/lib.rs:192                     fasten_joins_two_plates
    verified by   src/lib.rs:203                     plates_are_drilled_steel
    verified by   tests/traceability_edges.rs:9      req001_bound_holds_downstream

REQ-002  Structural plates must be drilled steel plates.
    defined at    src/lib.rs:36                      trait Req002StructuralPlate
    satisfied by  src/lib.rs:74                      DrilledSteelPlate  [Satisfies tag]
    satisfied by  src/lib.rs:142                     fasten  [Satisfies tag]
    satisfied by  src/lib.rs:87                      const fn assert_req002<T: Req002StructuralPlate>() {}  [bound use]
    satisfied by  src/lib.rs:142                     pub fn fasten<B: Req001FasteningBolt, P: Req002StructuralPlate>(  [bound use]
    satisfied by  src/lib.rs:204                     const fn is_structural<P: Req002StructuralPlate>() {}  [bound use]
    satisfied by  tests/traceability_edges.rs:19     const fn takes<T: Req002StructuralPlate>() {}  [bound use]
    verified by   src/lib.rs:192                     fasten_joins_two_plates
    verified by   src/lib.rs:203                     plates_are_drilled_steel

REQ-003  Swarf produced by drilling must be handed to a waste consumer.
    defined at    src/lib.rs:40                      trait Req003SwarfDisposal
    satisfied by  src/lib.rs:104                     SwarfBin  [Satisfies tag]
    satisfied by  src/lib.rs:163                     dispose_swarf  [Satisfies tag]
    satisfied by  src/lib.rs:125                     const fn assert_req003<T: Req003SwarfDisposal>() {}  [bound use]
    satisfied by  src/lib.rs:163                     pub fn dispose_swarf<C: Req003SwarfDisposal>(swarf: Swarf, bin:...  [bound use]
    verified by   (no test)

WARNING: REQ-003 has no verifying test (defined at src/lib.rs:40)
WARNING: unknown requirement REQ-999 tagged at tests/traceability_edges.rs:16 (Verifies on unknown_requirement_id_is_flagged)
WARNING: malformed tag (not canonical Verifies:/Satisfies:) at src/lib.rs:211: verifies: REQ-002
```

Exit code: 1 (warnings present), suitable as a CI gate.

### 2a. Representative compiler error

Removing `impl Length15mm for HexBoltM8` (i.e. the type no longer meets the requirement) while the Satisfies-backing assertion is in place:

```
error[E0277]: the trait bound `HexBoltM8: Req001FasteningBolt` is not satisfied
  --> src/lib.rs:70:31
   |
70 | const _: () = assert_req001::<HexBoltM8>();
   |                               ^^^^^^^^^ unsatisfied trait bound
   |
help: the trait `Length15mm` is not implemented for `HexBoltM8`
  --> src/lib.rs:53:1
   |
53 | pub struct HexBoltM8(());
   | ^^^^^^^^^^^^^^^^^^^^
note: required for `HexBoltM8` to implement `Req001FasteningBolt`
  --> src/lib.rs:34:41
   |
34 | impl<T: M8 + Bolt + Steel + Length15mm> Req001FasteningBolt for T {}
   |                             ----------  ^^^^^^^^^^^^^^^^^^^     ^
   |                             |
   |                             unsatisfied trait bound introduced here
```

Judgement: very readable for a modeller — it names the requirement trait, the offending type, and the exact missing characteristic (`Length15mm`), i.e. "this bolt is not 15 mm".

## 3. Candidate FINDINGS.md entries (ready to paste)

> **The R10 blanket impl makes "which types satisfy REQ-NNN" invisible to grep.** Because a requirement trait is satisfied via `impl<T: A + B> ReqNNN for T {}`, no source line ever states `impl ReqNNN for ConcreteType`, so text search cannot recover the type→requirement link — only the trait solver knows it. Cost: "satisfied by" cannot be extracted from the R10 pattern alone. Workaround (adopted, EXP-06): an explicit `/// Satisfies: REQ-NNN` doc tag on each satisfying type or process, kept honest by a compile-checked assertion next to it (`const fn assert_reqNNN<T: ReqNNN>() {}` + `const _: () = assert_reqNNN::<TheType>();`), so a stale tag is a compile error. Bound *uses* (`fn f<B: ReqNNN>`) remain directly greppable.

> **Grep-based traceability is reliable only under a strict tag discipline; the discipline itself is mechanically lintable.** EXP-06's `trace.sh` produced a correct report (no false positives/negatives) across the edge cases — multiple IDs per tag line, whitespace variations, IDs in `//` / `////` / block comments, string literals and doc-comment prose, an unverified requirement, an unknown ID — but only because tags follow the exact convention in EXP-06 RESULTS.md §4 (three-slash doc comments only, case-sensitive `Verifies:` / `Satisfies:`, doc block directly above the item). Cost: the convention must be kept; deviations are silent misses in principle. Workaround: the script warns on near-miss tags (e.g. lowercase `verifies:`) and on unknown REQ ids, and exits nonzero on any warning, so CI catches drift with the same tool that makes the report.

> **Grep has no scope awareness, so "satisfied by" rows for bound uses show the source line, not the enclosing item.** A requirement bound in a multi-line `where` clause is attributed to the where-clause line rather than the function name. Cost: cosmetic — locations are exact, names sometimes missing. Workaround: keep the requirement bound on the same line as `fn name<...>` (rustfmt does this at default width for these signatures); a future version could use `cargo rustdoc --output-format json` (nightly-only today) or a small parser for exact attribution.

## 4. Recommendation: **adopt** (with one addition to R10)

Adopt R10's greppable-traceability claim: it holds. The report was correct on every edge case, the script is ~150 lines of dependency-free sh/awk, and mis-formatted tags are themselves detectable. The one addition needed to R10 is the `Satisfies:` tag + backing assertion (below), because the blanket impl otherwise hides type→requirement links from text search.

### Proposed tagging convention (the discipline the code must keep)

**Requirement definitions**
1. Every requirement is a named trait `ReqNNN<Name>`; its doc comment's **first line** is `/// REQ-NNN: <one-sentence statement>.` — ID at the start of the line, exactly three slashes, colon after the ID. Further doc lines may follow.
2. The `trait` line follows the doc block directly (attributes may intervene; blank lines may not).
3. IDs are `REQ-` plus exactly three digits, sequential, never reused. `REQ-NNN` outside this pattern is treated as prose.

**Satisfaction**
4. Every type or process that is *claimed* to satisfy a requirement carries `/// Satisfies: REQ-NNN` (comma-separated for several) as its own line in its doc block, directly above the item.
5. Every `Satisfies:` tag on a **type** is backed by a compile-checked assertion in the same file: `const _: () = assert_reqNNN::<TheType>();` with `const fn assert_reqNNN<T: ReqNNN>() {}` defined once per requirement. The tag is for grep; the assertion is for truth.
6. Processes link to requirements only through the named requirement trait as a bound (`fn f<B: Req001FasteningBolt>`), never by re-spelling the characteristic list; keep the bound on the same line as the `fn` name.
7. No direct `impl ReqNNN for Type` — the blanket impl is the only impl (the script filters `impl<…> ReqNNN for …` lines as the blanket).

**Verification**
8. Every test that verifies requirements carries `/// Verifies: REQ-NNN[, REQ-MMM…]` as its own doc line directly above the `#[test]` fn. One tag line per test; several IDs go on that one line, comma-separated.
9. Tags are **case-sensitive** (`Verifies:`, `Satisfies:`) and live only in three-slash `///` doc comments. Anything in `//`, `////`, `//!`, `/* */` or string literals is ignored by design — so requirement IDs may be mentioned freely in prose and ordinary comments without polluting the report.
10. `trace.sh` runs in CI and must exit 0: an unverified requirement, an unknown REQ id in a tag, or a near-miss tag (wrong case/format) fails the build.

## Files

- `/Users/tonywalmsley/work/systems_engineering/claude/experiments/exp06-traceability/src/lib.rs`
- `/Users/tonywalmsley/work/systems_engineering/claude/experiments/exp06-traceability/tests/traceability_edges.rs`
- `/Users/tonywalmsley/work/systems_engineering/claude/experiments/exp06-traceability/trace.sh`
