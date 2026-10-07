//! modellint — the model linter (PLAN step 11, candidate R21): analyses the
//! model crates for completeness and style violations and writes one report
//! per crate plus a workspace summary into `docs/analysis/`.
//!
//! Usage: `modellint <repo-root> [--out <dir>] [--gate] [crate-name ...]`
//! (normally driven by `tools/lint.sh`). The extraction lives in the
//! `diagram_gen` library, shared with the `diagram-gen` and `docgen` binaries.
//!
//! `--out <dir>` writes the reports somewhere other than `docs/analysis/`
//! (the staleness check regenerates into a temp dir and diffs). `--gate`
//! exits nonzero when any gate-fatal ERROR finding exists (ERROR-KNOWN,
//! WARN and INFO never fail).

#![forbid(unsafe_code)]

use diagram_gen::{default_crates, lint};
use std::path::PathBuf;

fn main() {
    let mut root: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;
    let mut gate = false;
    let mut crates: Vec<String> = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--out" => match args.next() {
                Some(d) => out = Some(PathBuf::from(d)),
                None => {
                    eprintln!("ERROR: --out needs a directory argument");
                    std::process::exit(2);
                }
            },
            "--gate" => gate = true,
            _ if root.is_none() => root = Some(PathBuf::from(a)),
            _ => crates.push(a),
        }
    }
    let root = root.unwrap_or_else(|| PathBuf::from("."));
    let out = out.unwrap_or_else(|| root.join("docs").join("analysis"));
    if crates.is_empty() {
        crates = default_crates();
    }

    let fatal = lint::run(&root, &out, &crates);
    if gate && fatal > 0 {
        std::process::exit(1);
    }
}
