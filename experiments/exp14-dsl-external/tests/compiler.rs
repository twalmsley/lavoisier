//! Tests for `lavc` itself: the good slice compiles, generation is
//! deterministic (byte-identical across runs), and each notation-error
//! fixture produces the expected modeller-phrased diagnostic with the
//! expected line/col (the error layer the tool owns outright).

use exp14_dsl_external::{check, emit, parse};
use std::path::Path;

fn fixture(name: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("cannot read {}: {}", p.display(), e))
}

/// Parses + checks a fixture, returning the rendered diagnostics (empty when
/// it is valid).
fn diagnostics(name: &str) -> String {
    let src = fixture(name);
    match parse::parse(&src) {
        Err(d) => d.render(name, &src),
        Ok(m) => match check::check(&m) {
            Ok(_) => String::new(),
            Err(diags) => diags
                .iter()
                .map(|d| d.render(name, &src))
                .collect::<Vec<_>>()
                .join("\n"),
        },
    }
}

#[test]
fn the_good_slice_parses_checks_and_generates() {
    let src = fixture("model.lav");
    let m = parse::parse(&src).expect("model.lav must parse");
    let checked = check::check(&m).expect("model.lav must validate");
    let files = emit::generate(&checked, "model.lav", "../../../../model/model-core");
    assert_eq!(files.len(), 7, "Cargo.toml, 5 src files, tests/flows.rs");
    let lib = &files.iter().find(|(p, _)| p == "src/lib.rs").unwrap().1;
    assert!(lib.contains("#![forbid(unsafe_code)]"));
    assert!(lib.contains("#![deny(unused_must_use)]"));
}

#[test]
fn generation_is_deterministic() {
    let src = fixture("model.lav");
    let gen_once = || {
        let m = parse::parse(&src).unwrap();
        let checked = check::check(&m).unwrap();
        emit::generate(&checked, "model.lav", "../../../../model/model-core")
    };
    assert_eq!(gen_once(), gen_once(), "two runs must be byte-identical");
}

#[test]
fn breadcrumbs_link_every_item_back_to_the_notation() {
    let src = fixture("model.lav");
    let m = parse::parse(&src).unwrap();
    let checked = check::check(&m).unwrap();
    let files = emit::generate(&checked, "model.lav", "../../../../model/model-core");
    for (path, contents) in &files {
        if path.ends_with(".rs") {
            assert!(
                contents.contains("// lav: model.lav:"),
                "{} carries no breadcrumb",
                path
            );
        }
    }
    // Flow statements carry theirs on the diagnostic-bearing line itself.
    let flows = &files.iter().find(|(p, _)| p == "src/flows.rs").unwrap().1;
    assert!(flows.contains("boil::<1500, 550_000, 500_000, 50_000>(filled, energy); // lav: model.lav:"));
}

#[test]
fn unknown_resource_gets_line_col_and_a_suggestion() {
    let out = diagnostics("errors/unknown-resource.lav");
    assert!(out.contains("no resource called `ColdWatr` is declared"), "{out}");
    assert!(out.contains("unknown-resource.lav:72:13"), "{out}");
    assert!(out.contains("did you mean"), "{out}");
}

#[test]
fn balance_naming_a_foreign_magnitude_is_rejected_with_the_carried_list() {
    let out = diagnostics("errors/unknown-magnitude.lav");
    assert!(out.contains("names `EMBODIED_X`"), "{out}");
    assert!(out.contains("unknown-magnitude.lav:82:73"), "{out}");
    assert!(out.contains("`boil` carries: G, DRAW_J, EMBODIED_J, HEAT_J"), "{out}");
}

#[test]
fn reusing_a_consumed_resource_is_the_notation_level_moved_value_error() {
    let out = diagnostics("errors/moved-resource.lav");
    assert!(
        out.contains("`water` was already used by `fill_kettle` at line 107"),
        "{out}"
    );
    assert!(out.contains("moved-resource.lav:108:35"), "{out}");
    assert!(out.contains("only one process at a time (R2)"), "{out}");
}

#[test]
fn an_unaccounted_resource_at_flow_end_is_rejected() {
    let out = diagnostics("errors/unaccounted.lav");
    assert!(out.contains("`heat` is never accounted for"), "{out}");
    assert!(out.contains("(R1)"), "{out}");
}

#[test]
fn the_violation_notations_pass_the_notation_layer() {
    // The three canonical violations are structurally valid .lav: the
    // notation layer deliberately does not evaluate arithmetic or requirement
    // satisfaction - those are Rust's half of the two-layer error story.
    for v in [
        "violations/v-energy.lav",
        "violations/v-overdraw.lav",
        "violations/v-requirement.lav",
    ] {
        assert_eq!(diagnostics(v), "", "{} must pass the notation layer", v);
    }
}

#[test]
fn pretty_num_groups_like_the_house_style() {
    assert_eq!(check::pretty_num(1500), "1500");
    assert_eq!(check::pretty_num(30_000), "30_000");
    assert_eq!(check::pretty_num(550_000), "550_000");
    assert_eq!(check::pretty_num(0), "0");
}
