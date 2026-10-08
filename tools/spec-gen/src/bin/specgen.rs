//! `specgen <SPEC.md> <out-dir> [--model-core <path>]` — one-shot scaffolding
//! (R22). Parses the spec (strict: the A1–A10 conventions are the input
//! contract), validates it, and emits the generated model crate. Nothing is
//! written unless the spec is completely clean — speccheck's gate is the
//! precondition, not a suggestion.
//!
//! The output obeys the generation regime (R22): deterministic and
//! idempotent (two runs byte-identical); every emitted item carries a
//! SPEC.md §/line breadcrumb placed so Rust diagnostics render it (F-061);
//! everything the spec under-determines is an enumerable SPEC-HOLE
//! (`cargo build --features deny-holes` lists them all), never a guess.
//! The emitted crate is ONE-SHOT: promoted to hand-maintained in the same
//! change that commits it; there is no regeneration round-trip (F-062 —
//! mechanical naming drift makes one fatal).
//!
//! `--model-core` sets the path written into the generated Cargo.toml's
//! model-core dependency (default: ../../model/model-core, the layout of a
//! crate directory beside the model/ workspace).
//!
//! Exit codes: 0 generated, 2 spec findings (nothing written), 1 usage/IO.

use spec_gen::emit::{emit_crate, Generator};
use spec_gen::parse::parse_spec;
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut positional: Vec<&str> = Vec::new();
    let mut model_core = "../../model/model-core".to_string();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--model-core" => {
                i += 1;
                match args.get(i) {
                    Some(p) => model_core = p.clone(),
                    None => {
                        eprintln!("error: --model-core needs a path");
                        return ExitCode::from(1);
                    }
                }
            }
            a => positional.push(a),
        }
        i += 1;
    }
    if positional.len() != 2 {
        eprintln!("usage: specgen <SPEC.md> <out-dir> [--model-core <path>]");
        return ExitCode::from(1);
    }
    let spec_path = Path::new(positional[0]);
    let out_dir = Path::new(positional[1]);
    let text = match std::fs::read_to_string(spec_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: cannot read {}: {e}", spec_path.display());
            return ExitCode::from(1);
        }
    };
    let (spec, parse_errors) = parse_spec(&text, true);
    let mut generator = Generator::new(spec, true);
    for e in parse_errors {
        generator.errors.push(e);
    }
    if !generator.errors.is_empty() || !generator.warnings.is_empty() {
        let mut errors = generator.errors.clone();
        errors.sort_by_key(|e| e.line);
        let mut warnings = generator.warnings.clone();
        warnings.sort_by_key(|e| e.line);
        eprintln!(
            "specification findings ({} error(s), {} warning(s)) — nothing generated; fix the specification, not the model:",
            errors.len(),
            warnings.len()
        );
        for e in &errors {
            eprintln!("  error: {e}");
        }
        for w in &warnings {
            eprintln!("  warning: {w}");
        }
        return ExitCode::from(2);
    }
    let pkg = out_dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "generated-model".into());
    let files = emit_crate(&mut generator, &pkg, &model_core);
    let mut total_loc = 0usize;
    for (rel, contents) in &files {
        let path = out_dir.join(rel);
        if let Some(dir) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(dir) {
                eprintln!("error: cannot create {}: {e}", dir.display());
                return ExitCode::from(1);
            }
        }
        if let Err(e) = std::fs::write(&path, contents) {
            eprintln!("error: cannot write {}: {e}", path.display());
            return ExitCode::from(1);
        }
        let loc = contents.lines().count();
        total_loc += loc;
        println!("wrote {} ({} lines)", path.display(), loc);
    }
    println!(
        "generated {} files, {} lines, {} SPEC-HOLE(s) — enumerate them with `cargo build --features deny-holes`; this scaffold is ONE-SHOT: hand-maintained from the moment it is committed (R22)",
        files.len(),
        total_loc,
        generator.holes.len()
    );
    ExitCode::SUCCESS
}
