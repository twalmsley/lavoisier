//! Regression tests: the real CS-1 SPEC.md parses cleanly and completely,
//! and the three mutated fixtures produce the modeller-phrased
//! spec-validation errors with section and line references (the error story's
//! generation-time half).

use exp15_dsl_from_spec::emit::Generator;
use exp15_dsl_from_spec::parse::parse_spec;

fn load(rel: &str) -> String {
    let path = format!("{}/{rel}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {path}: {e}"))
}

fn analyse(text: &str) -> Generator {
    let (spec, parse_errors) = parse_spec(text);
    let mut g = Generator::new(spec);
    for e in parse_errors {
        g.errors.push(e);
    }
    g
}

/// The corpus — the REAL agreed spec — parses with zero validation errors
/// and the expected shape: 4 requirements (remapped to REQ-006..009 per the
/// §2 implementation note, F-053), 5 processes, 2 flow orders, 1 supplier,
/// 1 bounded consumer, 3 sinks, 2 draw sources.
#[test]
fn real_cs1_spec_parses_cleanly_and_completely() {
    let g = analyse(&load("../../case-studies/cs1-pot-of-tea/SPEC.md"));
    assert!(
        g.errors.is_empty(),
        "expected no validation errors, got: {:#?}",
        g.errors
    );
    assert_eq!(g.spec.requirements.len(), 4);
    assert_eq!(
        g.spec.requirements.iter().map(|r| r.impl_id).collect::<Vec<_>>(),
        vec![6, 7, 8, 9],
        "the §2 implementation note remaps ids workspace-globally (F-053)"
    );
    assert_eq!(g.spec.processes.len(), 5);
    assert_eq!(g.spec.flows.orders, vec![vec![1, 2, 3, 4, 5], vec![1, 3, 2, 4, 5]]);
    assert_eq!(g.suppliers.len(), 1, "the teabag box");
    assert_eq!(g.consumers.len(), 1, "the food-waste bin");
    assert_eq!(g.sinks.len(), 3, "drinker, council collection, kitchen air");
    assert_eq!(g.draws.len(), 2, "mains tap and grid socket");
    assert_eq!(g.person_budget, 300_000);
    // the known under-determinations surface as holes once emission runs —
    // here we check the planning-time ambiguity warnings instead
    assert!(
        g.warnings.iter().any(|w| w.contains("state 'boiling' appears on both")),
        "the Water/Kettle state collision is reported: {:#?}",
        g.warnings
    );
}

/// Spec-validation error 1: an unbalanced Balances line is caught at
/// generation time with the §/line reference and both totals.
#[test]
fn unbalanced_balance_is_a_generation_time_error() {
    let g = analyse(&load("tests/fixtures/bad-balance.md"));
    let msg = g.errors.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n");
    assert!(
        msg.contains("SPEC.md:91 §5 P3")
            && msg.contains("550 000 ≠ 500 000 + 60 000")
            && msg.contains("right totals 560 000"),
        "got: {msg}"
    );
}

/// Spec-validation error 2: a §5 waste product whose destination has no §4
/// output row is caught at generation time, for every process that routes to
/// the missing sink.
#[test]
fn waste_without_a_boundary_destination_is_a_generation_time_error() {
    let g = analyse(&load("tests/fixtures/missing-sink.md"));
    let msgs: Vec<String> = g.errors.iter().map(|e| e.to_string()).collect();
    assert!(
        msgs.iter().any(|m| m.contains("§5 P3") && m.contains("'kitchen air'") && m.contains("no §4 output row")),
        "got: {msgs:#?}"
    );
    assert!(
        msgs.iter().any(|m| m.contains("§5 P4") && m.contains("'kitchen air'")),
        "P4's steeping losses route to the same missing sink: {msgs:#?}"
    );
}

/// Spec-validation error 3: a consumed item that names no §3 resource is
/// caught at generation time with the offending phrase.
#[test]
fn unknown_resource_is_a_generation_time_error() {
    let g = analyse(&load("tests/fixtures/unknown-resource.md"));
    let msg = g.errors.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n");
    assert!(
        msg.contains("SPEC.md:80 §5 P2")
            && msg.contains("oolong pearls")
            && msg.contains("names no §3 resource"),
        "got: {msg}"
    );
}

/// The number lexer handles the spec's space-grouped thousands, and the
/// grouping rule is what disambiguates "1500 + 9" from "1 500".
#[test]
fn space_grouped_numbers_lex_correctly() {
    use exp15_dsl_from_spec::parse::lex_number;
    assert_eq!(lex_number("550 000 J"), Some((550_000, " J")));
    assert_eq!(lex_number("1500 + 9"), Some((1500, " + 9")));
    assert_eq!(lex_number("75 000 ms"), Some((75_000, " ms")));
    assert_eq!(lex_number("300 000 ms (5 min)"), Some((300_000, " ms (5 min)")));
    // a group that is not exactly 3 digits does not merge
    assert_eq!(lex_number("10 20"), Some((10, " 20")));
}
