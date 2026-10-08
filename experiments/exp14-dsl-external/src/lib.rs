//! EXP-14: `lavc`, a std-only compiler from the `.lav` external notation to a
//! complete, buildable model crate downstream of `model-core`.
//!
//! Pipeline: [`parse`] (syntax; first error wins) → [`check`] (references,
//! arity, flow discipline; all errors collected, with line/col and modeller
//! phrasing) → [`gen`] (deterministic, idempotent crate emission with `.lav`
//! breadcrumbs). The two-layer error story is deliberate: the notation owns
//! structure and the R1/R2 flow discipline; Rust owns requirement
//! satisfaction and conservation arithmetic (F-001).

pub mod ast;
pub mod check;
pub mod emit;
pub mod parse;

use std::fs;
use std::io::Write as _;
use std::path::Path;

/// Compiles one `.lav` file to a crate under `out_dir`. Returns the number of
/// files written, or the rendered diagnostics.
pub fn compile(
    lav_path: &Path,
    out_dir: &Path,
    model_core: &str,
) -> Result<usize, String> {
    let src = fs::read_to_string(lav_path)
        .map_err(|e| format!("error: cannot read {}: {}", lav_path.display(), e))?;
    let lav_name = lav_path
        .file_name()
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or_else(|| lav_path.display().to_string());
    let model = parse::parse(&src).map_err(|d| d.render(&lav_name, &src))?;
    let checked = check::check(&model).map_err(|diags| {
        let mut out = String::new();
        for d in &diags {
            out.push_str(&d.render(&lav_name, &src));
            out.push('\n');
        }
        out.push_str(&format!(
            "error: could not compile `{}` ({} notation error{})\n",
            lav_name,
            diags.len(),
            if diags.len() == 1 { "" } else { "s" }
        ));
        out
    })?;
    let files = emit::generate(&checked, &lav_name, model_core);
    let n = files.len();
    for (rel, contents) in files {
        let path = out_dir.join(&rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("error: cannot create {}: {}", parent.display(), e))?;
        }
        let mut f = fs::File::create(&path)
            .map_err(|e| format!("error: cannot write {}: {}", path.display(), e))?;
        f.write_all(contents.as_bytes())
            .map_err(|e| format!("error: cannot write {}: {}", path.display(), e))?;
    }
    Ok(n)
}
