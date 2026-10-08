//! Regression tests for the specification gate (R22) and the scaffolder.
//!
//! - The real, migrated CS-1 SPEC.md parses strict-clean and completely.
//! - The ported EXP-15 mutation fixtures (tests/fixtures/*.md) stay red, with
//!   the modeller-phrased, §/line-referenced errors (F-062's error class).
//! - Every A1–A10 convention check fires on an in-memory mutation of the real
//!   CS-1 document, so the checks track the live corpus.
//! - speccheck is red across files on a duplicated REQ id (F-053).
//! - specgen obeys the generation regime: deterministic (two runs
//!   byte-identical), and the pinned CS-1 scaffold builds clean, passes its
//!   generated flow tests, and enumerates exactly its SPEC-HOLEs under
//!   `--features deny-holes`.

use spec_gen::emit::{emit_crate, Generator};
use spec_gen::parse::parse_spec;

fn load(rel: &str) -> String {
    let path = format!("{}/{rel}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {path}: {e}"))
}

fn cs1() -> String {
    load("../../case-studies/cs1-pot-of-tea/SPEC.md")
}

fn analyse(text: &str, strict: bool) -> Generator {
    let (spec, parse_errors) = parse_spec(text, strict);
    let mut g = Generator::new(spec, strict);
    for e in parse_errors {
        g.errors.push(e);
    }
    g
}

fn errors_of(g: &Generator) -> String {
    g.errors
        .iter()
        .map(|e| e.to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Mutates the live CS-1 text: the needle must exist (so mutations can never
/// silently rot when the spec changes).
fn mutate(needle: &str, replacement: &str) -> String {
    let text = cs1();
    assert!(
        text.contains(needle),
        "mutation needle no longer in CS-1 SPEC.md: '{needle}'"
    );
    text.replacen(needle, replacement, 1)
}

// ── the corpus ──────────────────────────────────────────────────────────

/// The real, migrated CS-1 spec is strict-clean and parses completely:
/// 4 requirements at the workspace-global ids (A6/F-053), 5 processes, the
/// two §6 orders (A8), 1 supplier, 1 bounded consumer, 3 sinks, 2 draw
/// sources, and the person's budget.
#[test]
fn real_cs1_spec_is_strict_clean_and_complete() {
    let g = analyse(&cs1(), true);
    assert!(g.errors.is_empty(), "expected no errors, got: {:#?}", g.errors);
    assert!(
        g.warnings.is_empty(),
        "expected no warnings (they fail the gate), got: {:#?}",
        g.warnings
    );
    assert_eq!(g.spec.requirements.len(), 4);
    assert_eq!(
        g.spec.requirements.iter().map(|r| r.impl_id).collect::<Vec<_>>(),
        vec![6, 7, 8, 9],
        "the §2 bullets carry the workspace-global ids directly (A6, F-053)"
    );
    assert_eq!(g.spec.processes.len(), 5);
    assert_eq!(
        g.spec.flows.orders,
        vec![vec![1, 2, 3, 4, 5], vec![1, 3, 2, 4, 5]],
        "the §6 Orders field is the mechanical order source (A8)"
    );
    assert_eq!(g.suppliers.len(), 1, "the teabag box");
    assert_eq!(g.consumers.len(), 1, "the food-waste bin");
    assert_eq!(g.sinks.len(), 3, "drinker, council collection, kitchen air");
    assert_eq!(g.draws.len(), 2, "mains tap and grid socket");
    assert_eq!(g.person_budget, 300_000);
}

/// All six agreed case-study specs are strict-clean (the stage-2 gate kept
/// as a regression).
#[test]
fn all_agreed_case_studies_are_strict_clean() {
    for cs in [
        "cs1-pot-of-tea",
        "cs2-puncture-repair",
        "cs3-cafe-orders",
        "cs4-batch-run",
        "cs5-two-site",
        "cs6-bread-batch",
    ] {
        let g = analyse(&load(&format!("../../case-studies/{cs}/SPEC.md")), true);
        assert!(g.errors.is_empty(), "{cs}: {:#?}", g.errors);
        assert!(g.warnings.is_empty(), "{cs}: {:#?}", g.warnings);
    }
}

// ── the ported EXP-15 mutation fixtures (committed files) ───────────────

/// Fixture 1: an unbalanced Balances line is caught at the document with the
/// §/line reference and both totals.
#[test]
fn fixture_bad_balance_is_red() {
    let g = analyse(&load("tests/fixtures/bad-balance.md"), true);
    let msg = errors_of(&g);
    assert!(
        msg.contains("§5 P3")
            && msg.contains("does not balance")
            && msg.contains("left totals 550 000")
            && msg.contains("right totals 560 000"),
        "got: {msg}"
    );
}

/// Fixture 2: a §5 waste product whose destination has no §4 output row is
/// caught, for every process that routes to the missing sink.
#[test]
fn fixture_missing_sink_is_red() {
    let g = analyse(&load("tests/fixtures/missing-sink.md"), true);
    let msgs: Vec<String> = g.errors.iter().map(|e| e.to_string()).collect();
    assert!(
        msgs.iter()
            .any(|m| m.contains("§5 P3") && m.contains("kitchen air") && m.contains("no §4 output row")),
        "got: {msgs:#?}"
    );
    assert!(
        msgs.iter().any(|m| m.contains("§5 P4") && m.contains("kitchen air")),
        "P4's steeping losses route to the same missing sink: {msgs:#?}"
    );
}

/// Fixture 3: a consumed item that names no §3 resource is caught with the
/// offending phrase (A4 closure).
#[test]
fn fixture_unknown_resource_is_red() {
    let g = analyse(&load("tests/fixtures/unknown-resource.md"), true);
    let msg = errors_of(&g);
    assert!(
        msg.contains("§5 P2")
            && msg.contains("OolongPearl")
            && msg.contains("names no §3 resource"),
        "got: {msg}"
    );
}

// ── the A1–A10 convention checks (in-memory mutations of the live CS-1) ──

/// A1: an unmarked Balances clause is an error, not a judgement call.
#[test]
fn a1_unmarked_balance_is_red() {
    let t = mutate(
        "energy 550_000 = 500_000 + 50_000 (assert)",
        "energy 550_000 = 500_000 + 50_000",
    );
    let g = analyse(&t, true);
    assert!(
        errors_of(&g).contains("is not marked: every Balances clause ends in (assert) or (structural)"),
        "got: {}",
        errors_of(&g)
    );
}

/// A2: a process with waste and no Waste routing field is an error…
#[test]
fn a2_missing_waste_routing_is_red() {
    let t = mutate(
        "- **Waste routing:** routed by the flow (the flow's `vent_heat` carries the REQ bound — §8\n  feedback item 7).\n",
        "",
    );
    let g = analyse(&t, true);
    assert!(
        errors_of(&g).contains("no 'Waste routing:' field"),
        "got: {}",
        errors_of(&g)
    );
}

/// …and a routing field naming neither shape is too.
#[test]
fn a2_routing_naming_neither_shape_is_red() {
    let t = mutate(
        "- **Waste routing:** routed by the flow (the flow's `vent_heat` carries the REQ bound — §8\n  feedback item 7).",
        "- **Waste routing:** somehow.",
    );
    let g = analyse(&t, true);
    assert!(
        errors_of(&g).contains("names neither shape"),
        "got: {}",
        errors_of(&g)
    );
}

/// A3: a §5 waste product with no §3 row is an error (never synthesized in
/// strict mode).
#[test]
fn a3_waste_without_section3_row_is_red() {
    let t = mutate(
        "| `FoodWaste` (food waste) | continuous (waste) | grams — the spent teabags' mass leaving via disposal (R1: waste is a resource like any other) | 36 g | — |\n",
        "",
    );
    let g = analyse(&t, true);
    let msg = errors_of(&g);
    assert!(
        msg.contains("FoodWaste") && (msg.contains("A3") || msg.contains("no §3")),
        "got: {msg}"
    );
}

/// A5: a space-grouped number is an error in strict mode.
#[test]
fn a5_space_grouped_number_is_red() {
    let t = mutate("550_000 = 500_000 + 50_000", "550 000 = 500 000 + 50 000");
    let g = analyse(&t, true);
    assert!(
        errors_of(&g).contains("space-grouped number"),
        "got: {}",
        errors_of(&g)
    );
}

/// A6: a missing §2 Ids field is an error; so is a field whose range does
/// not match the bullets.
#[test]
fn a6_ids_field_is_required_and_checked() {
    let t = mutate("**Ids:** REQ-006–REQ-009", "Ids were allocated informally");
    let g = analyse(&t, true);
    assert!(
        errors_of(&g).contains("no '**Ids:**"),
        "got: {}",
        errors_of(&g)
    );
    let t = mutate("**Ids:** REQ-006–REQ-009", "**Ids:** REQ-006–REQ-010");
    let g = analyse(&t, true);
    assert!(
        errors_of(&g).contains("the field states exactly the allocated range"),
        "got: {}",
        errors_of(&g)
    );
}

/// A7: the same backticked state on two §3 rows is an error — one owner row
/// per state.
#[test]
fn a7_state_owned_by_two_rows_is_red() {
    let t = mutate(
        "| `Water` | continuous | mass (g); embodied energy (J) carried in the type from boiling onward | 1_500 g drawn | `cold` → boiling (the kettle row owns that state) → tea (leaves in the pot) |",
        "| `Water` | continuous | mass (g); embodied energy (J) carried in the type from boiling onward | 1_500 g drawn | `cold` → `boiling` → tea (leaves in the pot) |",
    );
    let g = analyse(&t, true);
    assert!(
        errors_of(&g).contains("appears on both") && errors_of(&g).contains("exactly one resource row"),
        "got: {}",
        errors_of(&g)
    );
}

/// A8: a missing §6 Orders field is an error; an order naming an unknown
/// process is too.
#[test]
fn a8_orders_field_is_required_and_closed() {
    let t = mutate("- **Orders:** (a) P1, P2, P3, P4, P5; (b) P1, P3, P2, P4, P5", "");
    let g = analyse(&t, true);
    assert!(
        errors_of(&g).contains("no '**Orders:**' field"),
        "got: {}",
        errors_of(&g)
    );
    let t = mutate("(b) P1, P3, P2, P4, P5", "(b) P1, P3, P2, P4, P9");
    let g = analyse(&t, true);
    assert!(
        errors_of(&g).contains("names P9, which §5 does not define"),
        "got: {}",
        errors_of(&g)
    );
}

/// The Balances time clause must agree with the Actor draw.
#[test]
fn time_clause_disagreeing_with_actor_draw_is_red() {
    let t = mutate("time 30_000 ms → History (structural)", "time 25_000 ms → History (structural)");
    let g = analyse(&t, true);
    assert!(
        errors_of(&g).contains("says 25 000 ms but the Actor line draws 30 000 ms"),
        "got: {}",
        errors_of(&g)
    );
}

/// A "no person" process must draw no person-time.
#[test]
fn no_person_process_drawing_time_is_red() {
    let t = mutate(
        "kettle only — **no person** (the kettle is automatic",
        "kettle only — **no person** (draws 5_000 ms; the kettle is automatic",
    );
    let g = analyse(&t, true);
    assert!(
        errors_of(&g).contains("says 'no person' but also draws person-time"),
        "got: {}",
        errors_of(&g)
    );
}

/// A Satisfies id §2 does not define is an error.
#[test]
fn satisfies_naming_unknown_req_is_red() {
    let t = mutate("REQ-007 (the loaded-pot state", "REQ-099 (the loaded-pot state");
    let g = analyse(&t, true);
    assert!(
        errors_of(&g).contains("Satisfies names REQ-099, which §2 does not define"),
        "got: {}",
        errors_of(&g)
    );
}

/// Warnings fail the gate too (R22): an assert whose terms carry no numbers
/// and no note ("where does this assert live?") is at least a warning.
#[test]
fn unverifiable_assert_without_note_warns() {
    let t = mutate(
        "mass 1500 + 9 = 1473 + 36 (= 1509) (assert)",
        "mass poured + dry = tea + bags (assert)",
    );
    let g = analyse(&t, true);
    assert!(
        g.warnings
            .iter()
            .any(|w| w.message.contains("cannot be verified at the document")),
        "got: {:#?} / {:#?}",
        g.errors,
        g.warnings
    );
}

// ── number lexing (A5) ──────────────────────────────────────────────────

/// Strict lexing takes underscore grouping and plain digits only; lenient
/// keeps the EXP-15 space-grouping (and its "1500 + 9" disambiguation).
#[test]
fn number_lexing_strict_vs_lenient() {
    use spec_gen::parse::lex_number;
    assert_eq!(lex_number("550_000 J", true), Some((550_000, " J")));
    assert_eq!(lex_number("550 000 J", true), Some((550, " 000 J")));
    assert_eq!(lex_number("550 000 J", false), Some((550_000, " J")));
    assert_eq!(lex_number("1500 + 9", false), Some((1500, " + 9")));
    assert_eq!(lex_number("300 000 ms (5 min)", false), Some((300_000, " ms (5 min)")));
    // a group that is not exactly 3 digits does not merge
    assert_eq!(lex_number("10 20", false), Some((10, " 20")));
}

// ── cross-file REQ uniqueness, through the real binary (F-053) ──────────

/// Two individually-clean specs sharing a REQ id fail speccheck together.
#[test]
fn duplicate_req_across_files_fails_the_binary() {
    let bin = env!("CARGO_BIN_EXE_speccheck");
    let fixtures = format!("{}/tests/fixtures/dup-req", env!("CARGO_MANIFEST_DIR"));
    // each alone: clean
    for sub in ["alpha", "beta"] {
        let out = std::process::Command::new(bin)
            .arg(format!("{fixtures}/{sub}/SPEC.md"))
            .output()
            .expect("run speccheck");
        assert!(out.status.success(), "{sub} should be clean alone");
    }
    // together: the duplicate id fails the gate
    let out = std::process::Command::new(bin)
        .arg(&fixtures)
        .output()
        .expect("run speccheck");
    assert!(!out.status.success(), "duplicate REQ ids must fail");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("REQ-101") && stdout.contains("workspace-unique"),
        "got: {stdout}"
    );
}

// ── the CS-6 field-test regressions (the four silent mis-derivations) ────
//
// CS-6 was specgen's first real field test and exposed four generator
// defects, every one of them SILENT (the scaffold compiled or nearly
// compiled while minting or vanishing resources). Each test below pins the
// fixed behaviour against the live CS-6 document, asserting first that the
// triggering phrasing is still in the spec (the mutation-needle discipline),
// then that the emitted crate carries the right shape.

fn cs6() -> String {
    load("../../case-studies/cs6-bread-batch/SPEC.md")
}

/// Emits the CS-6 scaffold in memory: (generator, file → contents).
fn emit_cs6() -> (Generator, std::collections::BTreeMap<String, String>) {
    let (spec, errs) = parse_spec(&cs6(), true);
    assert!(errs.is_empty(), "{errs:#?}");
    let mut g = Generator::new(spec, true);
    assert!(g.errors.is_empty() && g.warnings.is_empty(), "{:#?} / {:#?}", g.errors, g.warnings);
    let files = emit_crate(&mut g, "cs6-scaffold", "../../model/model-core");
    (g, files.into_iter().collect())
}

fn needle(text: &str, n: &str) {
    assert!(text.contains(n), "regression needle no longer in CS-6 SPEC.md: '{n}'");
}

/// D1 — a parenthesised magnitude is never an item count: "the `mixed`
/// `Dough` (1_682 g)" is ONE 1_682 g dough, not 1_682 doughs (the defective
/// scaffold emitted a 1_682-element tuple and a nonexistent `N1682` alias).
#[test]
fn d1_parenthesised_magnitude_is_one_item_not_a_count() {
    needle(&cs6(), "the `mixed` `Dough` (1_682 g)");
    let (_, files) = emit_cs6();
    let res = &files["src/resources.rs"];
    assert!(
        res.contains("pub fn mix_dough") && !res.contains("MixedDough, MixedDough"),
        "mix_dough must produce exactly one MixedDough"
    );
    assert!(!res.contains("N1682"), "no magnitude-sized nat alias may appear");
    assert!(
        res.contains("pub fn knead<const B: u64>(person: Person<B>, mixed_dough: MixedDough) -> (Person<B>, KneadedDough)"),
        "knead takes the one dough by value and returns the one kneaded dough:\n{res}"
    );
}

/// D2 — a consumed item may never vanish from the signature: every §5
/// Consumes item is a by-value parameter, and SupplyN is reserved for a
/// genuine multi-item draw from a §4 discrete supplier (the defective
/// scaffold emitted `knead(person, baker)` with no dough anywhere).
#[test]
fn d2_consumed_items_are_by_value_parameters() {
    needle(&cs6(), "**Consumes:** the `kneaded` `Dough` (1_682 g)");
    needle(&cs6(), "2 `fresh` `YeastSachet`s");
    let (g, files) = emit_cs6();
    let res = &files["src/resources.rs"];
    assert!(
        res.contains("pub fn prove<const B: u64>(person: Person<B>, kneaded_dough: KneadedDough) -> (Person<B>, ProvedDough)"),
        "prove must consume the kneaded dough by value:\n{res}"
    );
    assert!(
        res.contains("proved_dough: ProvedDough") && res.contains("pub fn divide_and_shape"),
        "divide_and_shape must consume the proved dough by value"
    );
    assert!(
        res.contains("flour: Flour<1_000>, water: Water<650>, salt: Salt<18>"),
        "mix_dough must take every continuous draw by value"
    );
    // SupplyN only for the genuine supplier draw (the 2 sachets from the box)
    let p1 = g.plans.iter().find(|p| p.p.id == 1).unwrap();
    assert_eq!(p1.supply_n, Some((2, "FreshSachet".into())));
    for p in &g.plans {
        if p.p.id != 1 {
            assert_eq!(p.supply_n, None, "P{} must not route items via SupplyN", p.p.id);
        }
    }
    assert!(res.contains("Taken = Cons<FreshSachet, Cons<FreshSachet, Nil>>"));
}

/// D3 — a stated count on a reusable's state is respected: "2 `clean`
/// `LoafTin`s (450 g each)" is two parameters, "2 `greased` `LoafTin`s
/// (456 g each)" two returns (the defective scaffold hardcoded one).
#[test]
fn d3_counts_on_reusable_states_are_respected() {
    needle(&cs6(), "2 `clean` `LoafTin`s (450 g each)");
    needle(&cs6(), "2 `greased` `LoafTin`s (456 g each)");
    let (_, files) = emit_cs6();
    let res = &files["src/resources.rs"];
    assert!(
        res.contains("clean_tin_1: CleanTin<450>, clean_tin_2: CleanTin<450>"),
        "grease_tins must take both clean tins:\n{res}"
    );
    assert!(
        res.contains("-> (Person<B>, GreasedTin<456>, GreasedTin<456>)"),
        "grease_tins must return both greased tins"
    );
}

/// D4 — a fallible (R17) process scaffolds the CS-2 dual-arm shape: a
/// `Result` of two conserving bundles, the outcome token realised in a match
/// with BOTH arms constructed, and the generated flows handling both arms
/// (the defective scaffold returned no loaf in any arm and defused the
/// token).
#[test]
fn d4_fallible_process_scaffolds_both_arms() {
    needle(&cs6(), "**Produces (Ok):** the `baked` `Loaf` (744 g, 300_000 J)");
    needle(&cs6(), "**Produces (Fail):** the `scorched` `Loaf` (744 g, 300_000 J)");
    let (_, files) = emit_cs6();
    let res = &files["src/resources.rs"];
    assert!(
        res.contains("bake_outcome: BakeOutcome) -> Result<BakeOk<B>, BakeFail<B>>"),
        "bake must return the dual-arm Result:\n{res}"
    );
    assert!(res.contains("match bake_outcome.consume_kind()"));
    assert!(res.contains("BakeOutcomeKind::Success => Ok(BakeOk"));
    assert!(res.contains("BakeOutcomeKind::Failure => Err(BakeFail"));
    // each arm carries its loaf, the used tin, the reusables and the waste
    assert!(res.contains("pub baked_loaf: BakedLoaf"));
    assert!(res.contains("pub scorched_loaf: ScorchedLoaf"));
    assert!(res.contains("pub used_tin: UsedTin<453>"));
    assert!(res.contains("pub steam: Steam<100>"));
    assert!(res.contains("pub waste_heat: WasteHeat<2_200_000>"));
    // the outcome token is an outcome_token!, not a consumable
    assert!(res.contains("model_core::outcome_token!"));
    // the generated flows handle BOTH arms, routing each arm's loaf
    let flows = &files["tests/flows.rs"];
    assert!(flows.contains("Ok(BakeOk { person, baked_loaf, used_tin, oven, steam, waste_heat })"));
    assert!(flows.contains("Err(BakeFail { person, scorched_loaf, used_tin, oven, steam, waste_heat })"));
    assert!(flows.contains("send_to(household, baked_loaf)"));
    assert!(flows.contains("send_to(compost_stream, scorched_loaf)"));
}

/// The supplier-box consume: "1 `YeastBox` (30 g)" under Consumes names the
/// same object §4 names as the sachets' supplier — the scaffold reads them
/// as ONE object (the supplier parameter IS the box, pinned to
/// `Rest = EmptyYeastBox`), the exhausted shell continues as the `empty`
/// state, and the decision is surfaced as a SPEC-HOLE (the defective
/// scaffold represented the box three times).
#[test]
fn supplier_box_consume_is_one_object_with_a_hole() {
    needle(&cs6(), "1 `YeastBox` (30 g)");
    needle(&cs6(), "the `YeastBox` (discrete supplier, exhausted to empty)");
    let (g, files) = emit_cs6();
    let res = &files["src/resources.rs"];
    assert!(
        !res.contains("empty_box: EmptyBox"),
        "the box must not be a second input parameter"
    );
    assert!(res.contains("Rest = EmptyYeastBox"));
    assert!(res.contains("let YeastBox(Nil) = rest;"));
    assert!(
        res.contains("-> (Person<B>, MixedDough, SpentSachet, SpentSachet, EmptyBox)"),
        "the empty box leaves exactly once, as the declared waste output:\n{res}"
    );
    assert!(
        g.holes.iter().any(|h| h.summary.contains("exhausted supplier")),
        "the one-object reading is an enumerated SPEC-HOLE"
    );
}

/// Two §4 output rows naming one sink: the sink consumes BOTH items (the
/// defective emitter dropped every row after the first, leaving the
/// atmosphere unable to take the waste heat at all).
#[test]
fn shared_sink_consumes_every_routed_item() {
    needle(&cs6(), "| `Steam` | atmosphere |");
    needle(&cs6(), "| `WasteHeat` | atmosphere |");
    let (_, files) = emit_cs6();
    let res = &files["src/resources.rs"];
    assert!(res.contains("Consumer<Steam<C0>> for Atmosphere"));
    assert!(res.contains("Consumer<WasteHeat<C0>> for Atmosphere"));
    assert!(res.contains("Consumer<SpentSachet> for Recycling"));
    assert!(res.contains("Consumer<EmptyBox> for Recycling"));
}

/// The pinned CS-6 scaffold regression (the field-test result, kept): the
/// scaffold builds clean against model-core, both generated flow tests pass
/// (every input a parameter, every output accounted, both P6 arms handled),
/// and `--features deny-holes` fires exactly the enumerated SPEC-HOLEs.
#[test]
fn pinned_cs6_scaffold_builds_tests_and_enumerates_holes() {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let out_dir = format!("{manifest}/target/pinned-cs6-scaffold");
    let model_core = format!("{manifest}/../../model/model-core");
    let _ = std::fs::remove_dir_all(&out_dir);

    let (spec, errs) = parse_spec(&cs6(), true);
    assert!(errs.is_empty(), "{errs:#?}");
    let mut g = Generator::new(spec, true);
    assert!(g.errors.is_empty() && g.warnings.is_empty());
    let files = emit_crate(&mut g, "pinned-cs6-scaffold", &model_core);
    assert_eq!(
        g.holes.len(),
        14,
        "the pinned CS-6 scaffold enumerates exactly 14 SPEC-HOLEs; holes now: {:#?}",
        g.holes.iter().map(|h| &h.summary).collect::<Vec<_>>()
    );
    for (rel, contents) in &files {
        let path = std::path::Path::new(&out_dir).join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, contents).unwrap();
    }

    let cargo = |args: &[&str]| {
        std::process::Command::new("cargo")
            .args(args)
            .current_dir(&out_dir)
            .output()
            .expect("run cargo")
    };
    let build = cargo(&["build"]);
    assert!(
        build.status.success(),
        "scaffold must build clean:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let test = cargo(&["test"]);
    assert!(
        test.status.success(),
        "the generated flow tests must pass:\n{}",
        String::from_utf8_lossy(&test.stderr)
    );
    let deny = cargo(&["build", "--features", "deny-holes"]);
    assert!(!deny.status.success(), "deny-holes must fail the build");
    let stderr = String::from_utf8_lossy(&deny.stderr);
    let fired = (1..=14)
        .filter(|i| stderr.contains(&format!("SPEC-HOLE U-{i:02}")))
        .count();
    assert_eq!(fired, 14, "exactly the 14 enumerated holes fire:\n{stderr}");
}

// ── the generation regime (R22): determinism + the pinned CS-1 scaffold ──

/// Two generations from the same spec are byte-identical (deterministic and
/// idempotent — the regeneration-gate precondition), for both scaffolded
/// case studies.
#[test]
fn generation_is_deterministic() {
    for (text, pkg) in [(cs1(), "cs1-scaffold"), (cs6(), "cs6-scaffold")] {
        let emit_once = || {
            let (spec, errs) = parse_spec(&text, true);
            assert!(errs.is_empty());
            let mut g = Generator::new(spec, true);
            assert!(g.errors.is_empty() && g.warnings.is_empty());
            emit_crate(&mut g, pkg, "../../model/model-core")
        };
        let a = emit_once();
        let b = emit_once();
        assert_eq!(a, b, "{pkg}: two runs must be byte-identical (R22)");
    }
}

/// The pinned CS-1 scaffold regression (the EXP-15 result, kept): generated
/// into an ignored target/ directory, the scaffold builds clean against
/// model-core, its generated flow tests pass, and `--features deny-holes`
/// fires exactly the enumerated SPEC-HOLEs.
#[test]
fn pinned_cs1_scaffold_builds_tests_and_enumerates_holes() {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let out_dir = format!("{manifest}/target/pinned-cs1-scaffold");
    let model_core = format!("{manifest}/../../model/model-core");
    let _ = std::fs::remove_dir_all(&out_dir);

    let text = cs1();
    let (spec, errs) = parse_spec(&text, true);
    assert!(errs.is_empty(), "{errs:#?}");
    let mut g = Generator::new(spec, true);
    assert!(g.errors.is_empty() && g.warnings.is_empty());
    let files = emit_crate(&mut g, "pinned-cs1-scaffold", &model_core);
    assert_eq!(
        g.holes.len(),
        12,
        "the pinned CS-1 scaffold enumerates exactly 12 SPEC-HOLEs; holes now: {:#?}",
        g.holes.iter().map(|h| &h.summary).collect::<Vec<_>>()
    );
    for (rel, contents) in &files {
        let path = std::path::Path::new(&out_dir).join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, contents).unwrap();
    }

    let cargo = |args: &[&str]| {
        std::process::Command::new("cargo")
            .args(args)
            .current_dir(&out_dir)
            .output()
            .expect("run cargo")
    };
    let build = cargo(&["build"]);
    assert!(
        build.status.success(),
        "scaffold must build clean:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let test = cargo(&["test"]);
    assert!(
        test.status.success(),
        "the generated flow tests must pass:\n{}",
        String::from_utf8_lossy(&test.stderr)
    );
    let deny = cargo(&["build", "--features", "deny-holes"]);
    assert!(!deny.status.success(), "deny-holes must fail the build");
    let stderr = String::from_utf8_lossy(&deny.stderr);
    let fired = (1..=12)
        .filter(|i| stderr.contains(&format!("SPEC-HOLE U-{i:02}")))
        .count();
    assert_eq!(fired, 12, "exactly the 12 enumerated holes fire:\n{stderr}");
}
