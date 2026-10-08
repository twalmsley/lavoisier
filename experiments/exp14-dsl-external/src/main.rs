//! `lavc <model.lav> --out <dir> [--model-core <path>]` — compiles the `.lav`
//! notation to a model crate. Std-only (the diagram-gen house style).

use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let mut input: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;
    // Default: generated/<name> sits two levels below the exp14 crate, which
    // sits two levels below the repository root.
    let mut model_core = "../../../../model/model-core".to_string();
    while let Some(a) = args.next() {
        match a.as_str() {
            "--out" => out = args.next().map(PathBuf::from),
            "--model-core" => {
                if let Some(p) = args.next() {
                    model_core = p;
                }
            }
            _ if input.is_none() => input = Some(PathBuf::from(a)),
            other => {
                eprintln!("error: unexpected argument `{}`", other);
                return ExitCode::FAILURE;
            }
        }
    }
    let Some(input) = input else {
        eprintln!("usage: lavc <model.lav> --out <dir> [--model-core <path>]");
        return ExitCode::FAILURE;
    };
    let Some(out) = out else {
        eprintln!("error: an output directory is required (--out <dir>)");
        return ExitCode::FAILURE;
    };
    match exp14_dsl_external::compile(&input, &out, &model_core) {
        Ok(n) => {
            println!(
                "generated {} files into {} from {}",
                n,
                out.display(),
                input.display()
            );
            ExitCode::SUCCESS
        }
        Err(msg) => {
            eprint!("{}", msg);
            ExitCode::FAILURE
        }
    }
}
