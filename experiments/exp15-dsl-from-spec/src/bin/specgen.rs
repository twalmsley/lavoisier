//! `specgen <SPEC.md> <out-dir>` — parses the spec, validates it, and emits
//! the generated model crate. Exit codes: 0 generated, 2 spec-validation
//! errors (nothing written), 1 usage/IO.

use exp15_dsl_from_spec::emit::{emit_crate, Generator};
use exp15_dsl_from_spec::parse::parse_spec;
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: specgen <SPEC.md> <out-dir>");
        return ExitCode::from(1);
    }
    let spec_path = Path::new(&args[1]);
    let out_dir = Path::new(&args[2]);
    let text = match std::fs::read_to_string(spec_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: cannot read {}: {e}", spec_path.display());
            return ExitCode::from(1);
        }
    };
    let (spec, mut errors) = parse_spec(&text);
    let mut generator = Generator::new(spec);
    errors.append(&mut generator.errors.clone());
    if !errors.is_empty() {
        errors.sort_by_key(|e| e.line);
        eprintln!(
            "specification errors ({}) — nothing generated; fix the specification, not the model:",
            errors.len()
        );
        for e in &errors {
            eprintln!("  error: {e}");
        }
        return ExitCode::from(2);
    }
    let pkg = out_dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "generated-model".into());
    let files = emit_crate(&mut generator, &pkg);
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
    println!();
    if !generator.warnings.is_empty() {
        println!("warnings ({}):", generator.warnings.len());
        for w in &generator.warnings {
            println!("  warning: {w}");
        }
        println!();
    }
    println!(
        "under-determined decisions ({} SPEC-HOLEs — `cargo build --features deny-holes` enumerates them in the generated crate):",
        generator.holes.len()
    );
    for h in &generator.holes {
        println!("  U-{:02}: {}", h.id, h.summary);
    }
    println!();
    println!(
        "generated: {} files, {} lines; {} requirements, {} resource types, {} boundary draws, {} suppliers, {} bounded consumers, {} sinks, {} processes, {} flow orders",
        files.len(),
        total_loc,
        generator.spec.requirements.len(),
        generator
            .lexicon
            .iter()
            .filter(|e| !matches!(e.role, exp15_dsl_from_spec::emit::Role::Common))
            .count()
            + generator.synthesized.len(),
        generator.draws.len(),
        generator.suppliers.len(),
        generator.consumers.len(),
        generator.sinks.len(),
        generator.plans.len(),
        generator.spec.flows.orders.len(),
    );
    ExitCode::SUCCESS
}
