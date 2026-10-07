//! diagram-gen — generates GitHub-viewable Mermaid diagrams from the model
//! crates' source, at three levels per crate (context / top-level /
//! detailed), into `docs/diagrams/<crate>/`.
//!
//! Usage: `diagram-gen <repo-root> [crate-name ...]`
//! (normally driven by `tools/diagrams.sh`). The extraction lives in the
//! `diagram_gen` library, shared with the `docgen` binary.

#![forbid(unsafe_code)]

use diagram_gen::{default_crates, emit, scan};
use std::path::PathBuf;

fn main() {
    let mut args = std::env::args().skip(1);
    let root = PathBuf::from(args.next().unwrap_or_else(|| ".".to_string()));
    let mut crates: Vec<String> = args.collect();
    if crates.is_empty() {
        crates = default_crates();
    }

    // Line locations for the fixed model-core helper API, for legend links.
    let core_locs = scan::scan_core_locs(&root);

    let mut index_rows: Vec<(String, String)> = Vec::new();
    let mut had_warnings = false;

    for krate in &crates {
        let crate_dir = root.join("model").join(krate);
        if !crate_dir.is_dir() {
            eprintln!("ERROR: no such model crate: {}", crate_dir.display());
            std::process::exit(1);
        }
        let mut cm = scan::scan_crate(&root, krate);
        let out_dir = root.join("docs").join("diagrams").join(krate);
        std::fs::create_dir_all(&out_dir).expect("create output dir");

        emit::emit_crate(&root, &out_dir, &mut cm, &core_locs);
        had_warnings |= !cm.warnings.is_empty();
        index_rows.push((krate.clone(), cm.title.clone()));
    }

    emit::emit_index(&root, &index_rows);
    if had_warnings {
        eprintln!("(generation completed with warnings — see the HTML comments in the output)");
    }
}
