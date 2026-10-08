//! `speccheck [--lenient] <SPEC.md | dir>...` — the specification gate (R22).
//!
//! Validates each given specification (a directory argument means every
//! `*/SPEC.md` below it, the `case-studies/` layout) against the
//! SPEC_TEMPLATE.md machine conventions, and REQ-id uniqueness ACROSS the
//! given files (F-053). Errors and warnings both carry file, line and §
//! references, phrased at the document: fix the specification, not the model
//! (F-062).
//!
//! Exit codes: 0 clean; 2 findings (any error — or, in strict mode, any
//! warning: what the tool cannot verify fails the gate, like trace.sh's);
//! 1 usage/IO. `--lenient` (migration aid only) keeps the EXP-15 fuzzy
//! lexicon and pre-A1..A10 conventions, and fails only on errors.

use spec_gen::emit::Generator;
use spec_gen::parse::parse_spec;
use std::path::PathBuf;
use std::process::ExitCode;

fn collect_specs(arg: &str, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let p = PathBuf::from(arg);
    if p.is_dir() {
        let mut subs: Vec<PathBuf> = std::fs::read_dir(&p)
            .map_err(|e| format!("cannot read directory {}: {e}", p.display()))?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .collect();
        subs.sort();
        let mut found = false;
        for sub in subs {
            let spec = sub.join("SPEC.md");
            if spec.is_file() {
                out.push(spec);
                found = true;
            }
        }
        if !found {
            return Err(format!("no */SPEC.md found under {}", p.display()));
        }
        Ok(())
    } else if p.is_file() {
        out.push(p);
        Ok(())
    } else {
        Err(format!("no such file or directory: {arg}"))
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut lenient = false;
    let mut specs: Vec<PathBuf> = Vec::new();
    for a in &args {
        match a.as_str() {
            "--lenient" => lenient = true,
            "--strict" => lenient = false,
            _ => {
                if let Err(e) = collect_specs(a, &mut specs) {
                    eprintln!("error: {e}");
                    return ExitCode::from(1);
                }
            }
        }
    }
    if specs.is_empty() {
        eprintln!("usage: speccheck [--lenient] <SPEC.md | dir>...");
        return ExitCode::from(1);
    }
    let strict = !lenient;
    let mut total_errors = 0usize;
    let mut total_warnings = 0usize;
    // (impl REQ id, file, line) across all files, for F-053 uniqueness
    let mut req_ids: Vec<(u32, String, usize)> = Vec::new();

    for path in &specs {
        let display = path.display().to_string();
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("error: cannot read {display}: {e}");
                return ExitCode::from(1);
            }
        };
        let (spec, parse_errors) = parse_spec(&text, strict);
        let mut g = Generator::new(spec, strict);
        for e in parse_errors {
            g.errors.push(e);
        }
        g.errors.sort_by_key(|e| e.line);
        g.warnings.sort_by_key(|e| e.line);
        for r in &g.spec.requirements {
            req_ids.push((r.impl_id, display.clone(), r.line));
        }
        for e in &g.errors {
            // SpecError displays as "SPEC.md:<line> §x: …" — prefix the real path
            println!("error: {}", e.to_string().replacen("SPEC.md", &display, 1));
        }
        for w in &g.warnings {
            println!("warning: {}", w.to_string().replacen("SPEC.md", &display, 1));
        }
        total_errors += g.errors.len();
        total_warnings += g.warnings.len();
    }

    // REQ ids are workspace-unique (F-053): a reused id leaves the CI gate
    // green while the traceability report silently merges requirements.
    req_ids.sort();
    for pair in req_ids.windows(2) {
        if pair[0].0 == pair[1].0 {
            println!(
                "error: {}:{} §2: REQ-{:03} is also defined in {} (line {}) — requirement ids are workspace-unique (F-053); allocate from the workspace sequence (A6)",
                pair[1].1, pair[1].2, pair[1].0, pair[0].1, pair[0].2
            );
            total_errors += 1;
        }
    }

    let gate_fails = total_errors > 0 || (strict && total_warnings > 0);
    if gate_fails {
        println!(
            "speccheck: {} error(s), {} warning(s) over {} specification(s) — fix the specification, not the model (R22, F-062){}",
            total_errors,
            total_warnings,
            specs.len(),
            if strict && total_errors == 0 {
                " [warnings fail the gate: what speccheck cannot verify is never silently passed]"
            } else {
                ""
            }
        );
        return ExitCode::from(2);
    }
    println!(
        "speccheck: {} specification(s) clean ({} warning(s))",
        specs.len(),
        total_warnings
    );
    ExitCode::SUCCESS
}
