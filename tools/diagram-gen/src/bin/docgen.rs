//! docgen — generates the per-process documents in `docs/processes/<crate>/`
//! from the model crates' source (candidate R20 made real).
//!
//! Usage: `docgen <repo-root> [crate-name ...]`
//! (normally driven by `tools/docgen.sh`). The extraction lives in the
//! `diagram_gen` library, shared with the `diagram-gen` binary.

#![forbid(unsafe_code)]

use diagram_gen::{default_crates, docgen};
use std::path::PathBuf;

fn main() {
    let mut args = std::env::args().skip(1);
    let root = PathBuf::from(args.next().unwrap_or_else(|| ".".to_string()));
    let mut crates: Vec<String> = args.collect();
    if crates.is_empty() {
        crates = default_crates();
    }
    docgen::run(&root, &crates);
}
