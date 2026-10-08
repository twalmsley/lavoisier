//! modellint — PLAN step 11 (candidate R21): the model linter.
//!
//! One pass over every model crate that unifies the completeness and style
//! checks previously scattered across trace.sh warnings, sealing greps and
//! prose-only conventions. Reuses the shared scanner (`scan`/`flow`/`emit`)
//! that the diagram and docgen binaries use — the extraction is never forked.
//!
//! Severity ladder:
//! * **ERROR** — a hard rule (R1 sealing, F-034's diverging-consumer shape,
//!   R10 rules 7/8, F-047, F-053). Gate-fatal under `tools/lint.sh --check`.
//! * **ERROR-KNOWN** — a true ERROR-level positive in committed model code,
//!   explicitly listed in [`KNOWN`] pending the maintainer's decision.
//!   Listed first in the report, **not** gate-fatal: this tool must never
//!   silently retune a severity, and must never edit the model crates.
//! * **WARN** — a should-fix convention violation (the R20 extraction
//!   conventions, R10 rule 4) or a scanner WARN. Reported, not gate-fatal.
//! * **INFO** — visibility only (placeholder density, thin verification,
//!   wide processes, untraced crates, F-035 sink coverage). Never fails.
//!
//! Conservative like the other two binaries: every check cites the
//! instructions.md rule or FINDINGS.md finding it enforces, and where the
//! line-based extraction cannot verify something it says so (a WARN-graded
//! finding) rather than guessing.

use crate::docgen::collect_verifies;
use crate::emit::{build_top_level, collect_traced_flows, write_out};
use crate::scan::{code_part, impl_for_split, scan_core_locs, scan_crate};
use crate::{
    CrateModel, FnKind, Loc, ResKind, base_name, contains_ident, generic_args, list_shape,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::Path;

// ---------------------------------------------------------------------------
// Findings, catalogue, known-error list.
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sev {
    Error,
    ErrorKnown,
    Warn,
    Info,
}

impl Sev {
    fn name(self) -> &'static str {
        match self {
            Sev::Error => "ERROR",
            Sev::ErrorKnown => "ERROR-KNOWN",
            Sev::Warn => "WARN",
            Sev::Info => "INFO",
        }
    }
}

pub struct Finding {
    pub check: &'static str,
    pub sev: Sev,
    pub msg: String,
    pub loc: Option<Loc>,
}

struct Check {
    id: &'static str,
    sev: Sev,
    cites: &'static str,
    what: &'static str,
}

/// The check catalogue. Every check cites the rule/finding it enforces; the
/// README renders this table.
const CATALOGUE: &[Check] = &[
    // ---- ERROR: hard rules --------------------------------------------------
    Check {
        id: "E-F034",
        sev: Sev::Error,
        cites: "F-034; R12/R16",
        what: "a contents-keeping `Consumer` impl whose `Next` grows a `Cons<..>` list with no \
               decreasing `Succ<_>` space parameter in the self type — trait resolution \
               diverges; at the mandated `recursion_limit = \"2048\"` that is a rustc SIGBUS \
               with no diagnostic",
    },
    Check {
        id: "E-SEAL-UNIT",
        sev: Sev::Error,
        cites: "R1 sealing; F-005",
        what: "a `pub` unit struct used as a by-value process/boundary input or output — its \
               name is a value expression, so anyone can mint the resource",
    },
    Check {
        id: "E-SEAL-ENUM",
        sev: Sev::Error,
        cites: "R1 sealing; F-005",
        what: "a `pub enum` consumed as a process/boundary input — every variant is a public \
               constructor (outcome groupings returned by flows are legal, F-046)",
    },
    Check {
        id: "E-SEAL-DERIVE",
        sev: Sev::Error,
        cites: "R1 sealing; F-005",
        what: "`derive(Clone)`, `derive(Copy)` or `derive(Default)` on a resource type — \
               duplication or a free public constructor",
    },
    Check {
        id: "E-SEAL-MUSTUSE",
        sev: Sev::Error,
        cites: "R1 sealing; F-046/R17",
        what: "a hand-written resource type (or outcome grouping) that crosses a \
               process/boundary signature by value with no `#[must_use]` — conservation \
               layer 1 missing",
    },
    Check {
        id: "E-ATTR",
        sev: Sev::Error,
        cites: "R1; F-007; F-010",
        what: "missing crate attribute: `forbid(unsafe_code)`, `deny(unused_must_use)`, \
               `deny(let_underscore_drop)`, or `recursion_limit` where type-level numbers \
               are used",
    },
    Check {
        id: "E-DEBUG-BUNDLE",
        sev: Sev::Error,
        cites: "F-047; R17",
        what: "`Debug` on an outcome bundle or outcome grouping — it would make \
               `.unwrap()`/`.expect()` compile, reopening the panic path past the failure arm",
    },
    Check {
        id: "E-TAG-MACRO",
        sev: Sev::Error,
        cites: "F-037; R10 rule 7; F-060",
        what: "a `Satisfies:` tag inside ANY macro invocation (kernel or not — broadened from \
               the `model_core::` set after F-060's vacuous-green run) — trace.sh drops it \
               silently (the gate stays green), so this tool must catch it",
    },
    Check {
        id: "E-REQ-DIAG",
        sev: Sev::Error,
        cites: "R10 rule 8; F-044",
        what: "a requirement trait without `#[diagnostic::on_unimplemented]` — the marker's \
               own message is ignored on the supertrait path, so the modeller would see a \
               bare trait-bound error",
    },
    Check {
        id: "E-TAG-ASSERT",
        sev: Sev::Error,
        cites: "R10; F-020",
        what: "a `Satisfies:` tag on a type with no backing `satisfies!` compile-checked \
               assertion — the tag could go stale silently (unverifiable placements are \
               reported as WARN, not guessed at)",
    },
    Check {
        id: "E-REQ-DUP",
        sev: Sev::Error,
        cites: "F-053",
        what: "a REQ id defined in more than one workspace crate — trace.sh merges and \
               shadows silently (the green-but-wrong direction)",
    },
    // ---- WARN: should fix ---------------------------------------------------
    Check {
        id: "W-MACRO-MODEL",
        sev: Sev::Warn,
        cites: "F-060; R22",
        what: "a model-shaped item (`fn`, `trait`, `struct`, `enum`, `impl`) inside a \
               non-kernel macro invocation — the R20/R21 toolchain parses source, so items \
               authored through a macro front are invisible to every tool while the gate \
               stays green (the F-060 vacuous-green direction); model source is only ever \
               ordinary Rust source (R22), with macro sugar confined to model-core's kernel \
               macros. Entered as WARN per R21's catalogue-growth rule",
    },
    Check {
        id: "W-R20-DOC",
        sev: Sev::Warn,
        cites: "R20 convention 2",
        what: "a public process/boundary fn/flow whose rustdoc does not open with a one-line \
               purpose (generated documents show a gap)",
    },
    Check {
        id: "W-R20-PLACE",
        sev: Sev::Warn,
        cites: "R20 convention 1",
        what: "a process-shaped `pub fn` outside the `processes`/`boundary` modules — the \
               extraction cannot classify it, so documents and diagrams omit it",
    },
    Check {
        id: "W-R20-RECORD",
        sev: Sev::Warn,
        cites: "R20 convention 3",
        what: "a `record(history, \"<name>\", ..)` whose string is not a known process fn \
               name — History events and adjacent draws would attribute to nothing",
    },
    Check {
        id: "W-R20-SUPPLIER",
        sev: Sev::Warn,
        cites: "R20 convention 5; F-055 gap 1",
        what: "a `Supplier` impl whose item the extraction cannot recover (no concrete \
               `type Item`, `Cons` head, fill machinery or full-state alias)",
    },
    Check {
        id: "W-R10-LINE",
        sev: Sev::Warn,
        cites: "R10 rule 4; F-038/F-055 gap 2",
        what: "a requirement bound not on the `fn`-name source line — trace.sh attributes \
               line-based greps to the wrong line",
    },
    Check {
        id: "W-SCAN",
        sev: Sev::Warn,
        cites: "F-055",
        what: "a scanner/tracer WARN for this crate (unknown callee, unresolved parameter, \
               unparsed signature) — the same WARNs the diagram/docgen outputs carry",
    },
    // ---- INFO: visibility, never fails --------------------------------------
    Check {
        id: "I-PLACEHOLDER",
        sev: Sev::Info,
        cites: "R12",
        what: "a `Placeholder:`-tagged item — the crate's open-items surface, counted and \
               listed per crate",
    },
    Check {
        id: "I-THIN-VERIFY",
        sev: Sev::Info,
        cites: "R5; R10 rule 6",
        what: "a requirement verified by fewer than 3 `Verifies:`-tagged tests",
    },
    Check {
        id: "I-WIDE-PROCESS",
        sev: Sev::Info,
        cites: "R9; F-024",
        what: "a process with more than 6 value parameters — a candidate for decomposition \
               (or an honest join point)",
    },
    Check {
        id: "I-NO-FLOW",
        sev: Sev::Info,
        cites: "R20 convention 7",
        what: "a crate with no traced flow (no composite `pub fn` flow and no integration \
               test composing 2+ processes)",
    },
    Check {
        id: "I-NO-SINK",
        sev: Sev::Info,
        cites: "F-035",
        what: "a tripwired type (container/consumable/outcome token) with no in-workspace \
               production consumer — INFO because downstream crates outside this workspace \
               may consume it",
    },
];

struct Known {
    check: &'static str,
    krate: &'static str,
    ident: &'static str,
    note: &'static str,
}

/// True ERROR-level positives in committed model code, graded ERROR-KNOWN
/// (listed first, non-fatal) pending the maintainer's decision. The linter
/// never edits model crates and never silently retunes a severity: each entry
/// here names the exact finding and why it is real.
const KNOWN: &[Known] = &[
    // (empty — the three pre-CS-5 contents-keeping bins that lacked
    // `#[must_use]` were fixed on 2026-10-07 after step 11's first run
    // caught them; see FINDINGS.md F-058.)
];

// ---------------------------------------------------------------------------
// Own-source model (raw + comment-stripped lines of the crate's own files).
// ---------------------------------------------------------------------------

struct SrcFile {
    rel: String,
    lines: Vec<String>,
    code: Vec<String>,
}

fn own_files(root: &Path, krate: &str) -> Vec<SrcFile> {
    let mut out = Vec::new();
    for sub in ["src", "tests"] {
        let dir = root.join("model").join(krate).join(sub);
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        let mut v: Vec<_> = rd
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().map(|e| e == "rs").unwrap_or(false))
            .collect();
        v.sort();
        for p in v {
            let Ok(text) = std::fs::read_to_string(&p) else { continue };
            let lines: Vec<String> = text.lines().map(str::to_string).collect();
            let code: Vec<String> = lines.iter().map(|l| code_part(l)).collect();
            out.push(SrcFile { rel: crate::rel_path(root, &p), lines, code });
        }
    }
    out
}

/// The contiguous attribute block directly above a 1-based item line
/// (walking up through `///` docs and `#[..]` attributes; attributes are
/// single-line in this codebase — the scan.rs assumption).
fn attr_block(f: &SrcFile, line_1b: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut j = line_1b.saturating_sub(1); // 0-based index of the item line
    while j > 0 {
        j -= 1;
        let t = f.lines[j].trim_start();
        if t.starts_with("#[") {
            out.push(t.to_string());
        } else if t.starts_with("///") || t.starts_with("//") {
            continue;
        } else {
            break;
        }
    }
    out
}

fn has_derive(attrs: &[String], which: &str) -> bool {
    attrs.iter().any(|a| {
        a.starts_with("#[derive(")
            && a["#[derive(".len()..]
                .trim_end_matches(&[']', ')'][..])
                .split(',')
                .any(|d| d.trim() == which || d.trim().ends_with(&format!("::{which}")))
    })
}

fn has_must_use(attrs: &[String]) -> bool {
    attrs.iter().any(|a| a.starts_with("#[must_use"))
}

fn is_own(loc: &Loc, krate: &str) -> bool {
    loc.file.starts_with(&format!("model/{krate}/"))
}

/// Join comment-stripped lines from `i` until (exclusive) the first `{` or a
/// terminating `;`, returning the joined text.
fn join_header(f: &SrcFile, i: usize) -> String {
    let mut s = String::new();
    for cj in f.code.iter().skip(i) {
        for ch in cj.chars() {
            if ch == '{' || ch == ';' {
                return s;
            }
            s.push(ch);
        }
        s.push(' ');
    }
    s
}

// ---------------------------------------------------------------------------
// Per-crate lint state.
// ---------------------------------------------------------------------------

struct CrateLint {
    cm: CrateModel,
    files: Vec<SrcFile>,
    findings: Vec<Finding>,
    flow_count: usize,
    /// Bases that cross own process/boundary/flow signatures by value.
    crossing: BTreeSet<String>,
    /// Bases consumed as inputs by own process/boundary fns.
    param_bases: BTreeSet<String>,
}

impl CrateLint {
    fn push(&mut self, check: &'static str, sev: Sev, msg: String, loc: Option<Loc>) {
        self.findings.push(Finding { check, sev, msg, loc });
    }
}

fn collect_crossing(cm: &CrateModel, krate: &str) -> (BTreeSet<String>, BTreeSet<String>) {
    let mut crossing = BTreeSet::new();
    let mut params = BTreeSet::new();
    for f in cm.fns.values() {
        if !matches!(f.kind, FnKind::Process | FnKind::Boundary | FnKind::Flow)
            || !is_own(&f.loc, krate)
        {
            continue;
        }
        for (_, ty) in &f.params {
            let b = cm.resolve_alias(&base_name(ty));
            if !b.is_empty() {
                crossing.insert(b.clone());
                params.insert(b);
            }
        }
        for r in &f.ret {
            let b = base_name(r);
            if b == "Result" {
                for arm in generic_args(r) {
                    let ab = cm.resolve_alias(&base_name(&arm));
                    if !ab.is_empty() {
                        crossing.insert(ab);
                    }
                }
            } else {
                let rb = cm.resolve_alias(&b);
                if !rb.is_empty() {
                    crossing.insert(rb);
                }
            }
        }
    }
    for c in &cm.consumers {
        if is_own(&c.loc, krate) {
            crossing.insert(c.sink.clone());
        }
    }
    for s in &cm.suppliers {
        if is_own(&s.loc, krate) {
            crossing.insert(s.src.clone());
        }
    }
    (crossing, params)
}

// ---------------------------------------------------------------------------
// ERROR checks.
// ---------------------------------------------------------------------------

/// E-F034: contents-keeping `Consumer` impl without a decreasing space
/// parameter (`Next` grows a `Cons` list of the same base; self type carries
/// no `Succ<_>`).
fn check_f034(cl: &mut CrateLint) {
    let mut found: Vec<(String, Loc)> = Vec::new();
    for f in &cl.files {
        for (i, c) in f.code.iter().enumerate() {
            let t = c.trim();
            if !t.starts_with("impl") || !(t.contains(" Consumer<") || t.contains("::Consumer<")) {
                continue;
            }
            let header = join_header(f, i);
            let Some((_, rhs)) = impl_for_split(&header) else { continue };
            let self_ty = match rhs.find(" where ") {
                Some(w) => rhs[..w].trim().to_string(),
                None => rhs.trim().to_string(),
            };
            // `type Next = ...;` inside the block.
            let mut depth = 0i64;
            let mut jj = i;
            let mut next_ty: Option<String> = None;
            let mut opened = false;
            while jj < f.code.len() {
                let (o, cc) = {
                    let mut o = 0i64;
                    let mut cc = 0i64;
                    for ch in f.code[jj].chars() {
                        match ch {
                            '{' => o += 1,
                            '}' => cc += 1,
                            _ => {}
                        }
                    }
                    (o, cc)
                };
                if o > 0 {
                    opened = true;
                }
                depth += o - cc;
                if let Some(rest) = f.code[jj].trim().strip_prefix("type Next = ") {
                    next_ty = Some(rest.trim_end_matches(';').trim().to_string());
                }
                if opened && depth <= 0 {
                    break;
                }
                jj += 1;
            }
            let Some(next) = next_ty else { continue };
            let keeps = next.contains("Cons<") && base_name(&next) == base_name(&self_ty);
            if keeps && !self_ty.contains("Succ<") {
                found.push((
                    format!(
                        "`impl Consumer<..> for {}` keeps contents (`Next = {}`) with no \
                         decreasing `Succ<_>` space parameter: trait resolution diverges — at \
                         `recursion_limit = \"2048\"` this is a deterministic rustc SIGBUS",
                        self_ty.replace('|', "/"),
                        next.replace('|', "/")
                    ),
                    Loc { file: f.rel.clone(), line: i + 1 },
                ));
            }
        }
    }
    for (msg, loc) in found {
        cl.push("E-F034", Sev::Error, msg, Some(loc));
    }
}

/// E-SEAL-UNIT / E-SEAL-ENUM / E-SEAL-DERIVE / E-SEAL-MUSTUSE / E-DEBUG-BUNDLE.
fn check_sealing(cl: &mut CrateLint, krate: &str) {
    // Outcome bundle/grouping bases: Result arms + enums returned by own fns.
    let mut bundle_bases: BTreeSet<String> = BTreeSet::new();
    let mut ret_enum_bases: BTreeSet<String> = BTreeSet::new();
    for f in cl.cm.fns.values() {
        if !is_own(&f.loc, krate) {
            continue;
        }
        for r in &f.ret {
            if base_name(r) == "Result" {
                for arm in generic_args(r) {
                    let ab = cl.cm.resolve_alias(&base_name(&arm));
                    if !ab.is_empty() {
                        bundle_bases.insert(ab);
                    }
                }
            } else if matches!(f.kind, FnKind::Process | FnKind::Boundary | FnKind::Flow) {
                let rb = cl.cm.resolve_alias(&base_name(r));
                if cl.cm.enums.contains_key(&rb) {
                    ret_enum_bases.insert(rb);
                }
            }
        }
    }

    let mut findings: Vec<(&'static str, String, Loc)> = Vec::new();
    for f in &cl.files {
        if !f.rel.contains("/src/") {
            continue; // sealing checks apply to the crate's production source
        }
        for (i, c) in f.code.iter().enumerate() {
            let t = c.trim();
            // Unit structs: `pub struct Name;` (possibly generic, no fields).
            if t.starts_with("pub struct ") && t.ends_with(';') && !t.contains('(') {
                let name = base_name(t.trim_start_matches("pub struct ").trim_end_matches(';'));
                if cl.crossing.contains(&name) {
                    findings.push((
                        "E-SEAL-UNIT",
                        format!(
                            "`pub struct {name};` is a unit struct used by value in a \
                             process/boundary signature — its name is a public constructor \
                             expression; give it a private `_seal: ()` field"
                        ),
                        Loc { file: f.rel.clone(), line: i + 1 },
                    ));
                }
            }
            // Pub enums consumed as process inputs.
            if t.starts_with("pub enum ") {
                let name = base_name(t.trim_start_matches("pub enum ").trim_end_matches('{'));
                if cl.param_bases.contains(&name) {
                    findings.push((
                        "E-SEAL-ENUM",
                        format!(
                            "`pub enum {name}` is consumed as a process/boundary input — every \
                             variant is a public constructor; wrap a private enum in a sealed \
                             struct (outcome groupings returned by flows are legal, F-046)"
                        ),
                        Loc { file: f.rel.clone(), line: i + 1 },
                    ));
                }
            }
        }
    }

    // Derive / must_use / Debug checks over the scanned type tables.
    let structs: Vec<(String, Loc)> = cl
        .cm
        .structs
        .iter()
        .filter(|(_, s)| is_own(&s.loc, krate))
        .map(|(n, s)| (n.clone(), s.loc.clone()))
        .collect();
    let enums: Vec<(String, Loc)> = cl
        .cm
        .enums
        .iter()
        .filter(|(_, e)| is_own(&e.loc, krate))
        .map(|(n, e)| (n.clone(), e.loc.clone()))
        .collect();
    let file_of = |rel: &str, files: &[SrcFile]| files.iter().position(|f| f.rel == rel);

    for (name, loc) in structs.iter().chain(enums.iter()) {
        let Some(fi) = file_of(&loc.file, &cl.files) else { continue };
        let attrs = attr_block(&cl.files[fi], loc.line);
        let kind = cl.cm.resources.get(name).map(|r| r.kind);
        let is_resource_like = cl.crossing.contains(name)
            || matches!(
                kind,
                Some(ResKind::Container | ResKind::Consumable | ResKind::Reusable | ResKind::OutcomeToken)
            );
        if is_resource_like {
            for d in ["Clone", "Copy", "Default"] {
                if has_derive(&attrs, d) {
                    findings.push((
                        "E-SEAL-DERIVE",
                        format!(
                            "`derive({d})` on resource type `{name}` — {} (R1 sealing)",
                            if d == "Default" {
                                "a public constructor regardless of field privacy"
                            } else {
                                "duplication violates conservation"
                            }
                        ),
                        loc.clone(),
                    ));
                }
            }
        }
        // must_use: hand-written types crossing signatures by value (macro
        // resources get theirs from the kernel macros).
        let hand_written = matches!(kind, Some(ResKind::BoundaryObject))
            || (kind.is_none() && cl.cm.enums.contains_key(name));
        let crosses = cl.crossing.contains(name) || ret_enum_bases.contains(name);
        if hand_written && crosses && !has_must_use(&attrs) {
            findings.push((
                "E-SEAL-MUSTUSE",
                format!(
                    "`{name}` crosses a process/boundary signature by value with no \
                     `#[must_use]` — conservation layer 1 (R1) is missing on this type; \
                     whole-value discard is caught only by tripwires at test time"
                ),
                loc.clone(),
            ));
        }
        // Debug on outcome bundles/groupings (F-047).
        if bundle_bases.contains(name) || ret_enum_bases.contains(name) {
            let mut debug = has_derive(&attrs, "Debug");
            if !debug {
                let pat = format!("Debug for {name}");
                debug = cl
                    .files
                    .iter()
                    .any(|f| f.code.iter().any(|c| c.contains(&pat)));
            }
            if debug {
                findings.push((
                    "E-DEBUG-BUNDLE",
                    format!(
                        "outcome bundle/grouping `{name}` implements `Debug` — \
                         `.unwrap()`/`.expect()` past the failure arm would compile (F-047); \
                         remove the derive/impl"
                    ),
                    loc.clone(),
                ));
            }
        }
    }

    for (check, msg, loc) in findings {
        cl.push(check, Sev::Error, msg, Some(loc));
    }
}

/// E-ATTR: required crate attributes on lib.rs.
fn check_crate_attrs(cl: &mut CrateLint, krate: &str) {
    let lib_rel = format!("model/{krate}/src/lib.rs");
    let Some(lib) = cl.files.iter().find(|f| f.rel == lib_rel) else {
        cl.push("E-ATTR", Sev::Error, format!("no src/lib.rs found for `{krate}`"), None);
        return;
    };
    let attrs: Vec<String> = lib
        .code
        .iter()
        .filter(|c| c.trim_start().starts_with("#!["))
        .map(|c| c.trim().to_string())
        .collect();
    let has = |needle: &str| attrs.iter().any(|a| a.contains(needle));
    let mut missing: Vec<&str> = Vec::new();
    if !has("forbid(unsafe_code)") {
        missing.push("#![forbid(unsafe_code)]");
    }
    if !has("deny(unused_must_use)") {
        missing.push("#![deny(unused_must_use)]");
    }
    if !has("deny(let_underscore_drop)") {
        missing.push("#![deny(let_underscore_drop)]");
    }
    let uses_nats = cl.files.iter().any(|f| {
        f.rel.contains("/src/")
            && f.code
                .iter()
                .any(|c| c.contains("Succ<") || c.contains("SupplyN") || c.contains("Cons<"))
    });
    if uses_nats && !has("recursion_limit") {
        missing.push("#![recursion_limit = \"2048\"] (type-level numbers are used, F-010)");
    }
    let loc = Some(Loc { file: lib_rel, line: 1 });
    for m in missing {
        cl.push("E-ATTR", Sev::Error, format!("missing crate attribute: `{m}`"), loc.clone());
    }
}

/// A block-form macro invocation opener: a comment-stripped line that is
/// exactly `<path>! {` (or `<path>!{`). Returns (full path, last segment).
/// A `macro_rules! name {` DEFINITION is not an invocation and is not
/// matched: its template lines are patterns, not model items; the items a
/// definition can smuggle in only become real at an invocation site, which
/// this check does see (F-060).
fn macro_invocation(code: &str) -> Option<(String, String)> {
    let head = code.strip_suffix('{')?.trim_end();
    let path = head.strip_suffix('!')?.trim_end();
    if path.is_empty()
        || !path
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == ':')
        || path.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(true)
    {
        return None;
    }
    let mname = path.rsplit("::").next().unwrap_or(path).to_string();
    Some((path.to_string(), mname))
}

/// A model-shaped item line (F-060): the shapes the extraction toolchain
/// recovers model structure from — `fn`, `trait`, `struct`, `enum`, `impl` —
/// including sugared forms whose keyword is not first (`process fn boil`).
fn is_model_shaped(code: &str) -> bool {
    let t = code.trim_start_matches("pub(crate)").trim_start_matches("pub ").trim_start();
    t.split_whitespace()
        .take(3)
        .any(|w| matches!(w, "fn" | "trait" | "struct" | "enum" | "impl"))
        || t.starts_with("impl<")
}

/// E-TAG-MACRO, E-REQ-DIAG, E-TAG-ASSERT (plus the WARN-graded unverifiable
/// tag placements): one pass over the raw lines with macro-block tracking.
fn check_tags_and_requirements(cl: &mut CrateLint) {
    struct Tag {
        ids: Vec<String>,
        loc: Loc,
    }
    let mut findings: Vec<(&'static str, Sev, String, Loc)> = Vec::new();

    for f in &cl.files {
        let n = f.lines.len();
        let mut i = 0usize;
        let mut pending: Option<Tag> = None;
        while i < n {
            let raw = f.lines[i].trim_start();
            let code = f.code[i].trim();

            // Macro invocation block: `<path>! {` — ANY macro, not just the
            // `model_core::` kernel set (F-060: the pre-broadening check
            // reported a whole-model macro front vacuously clean).
            if let Some((mpath, mname)) = macro_invocation(code) {
                let kernel = mpath.starts_with("model_core::");
                let start = i;
                let mut depth = 0i64;
                let mut has_diag = false;
                while i < n {
                    let cj = &f.code[i];
                    for ch in cj.chars() {
                        match ch {
                            '{' => depth += 1,
                            '}' => depth -= 1,
                            _ => {}
                        }
                    }
                    let rj = f.lines[i].trim_start();
                    if i > start && rj.starts_with("///") && rj.contains("Satisfies:") {
                        findings.push((
                            "E-TAG-MACRO",
                            Sev::Error,
                            format!(
                                "`Satisfies:` tag inside the `{mname}!` invocation — trace.sh \
                                 drops it silently (F-037); move the tag to a type alias or \
                                 the using process"
                            ),
                            Loc { file: f.rel.clone(), line: i + 1 },
                        ));
                    }
                    // F-060: model-shaped items inside a NON-kernel macro
                    // invocation are invisible to the whole R20/R21 toolchain
                    // while its gates stay green. WARN per R21's
                    // catalogue-growth rule (new checks enter as WARN).
                    if i > start && !kernel {
                        let cl_ = f.code[i].trim();
                        if is_model_shaped(cl_) {
                            findings.push((
                                "W-MACRO-MODEL",
                                Sev::Warn,
                                format!(
                                    "model-shaped item inside the non-kernel `{mname}!` \
                                     invocation (`{}`) — the extraction toolchain parses \
                                     source, so this item is invisible to trace.sh, the \
                                     diagram/document generators and this linter's \
                                     type-level checks (F-060); model source is only ever \
                                     ordinary Rust source (R22)",
                                    cl_.chars().take(60).collect::<String>().replace('|', "/")
                                ),
                                Loc { file: f.rel.clone(), line: i + 1 },
                            ));
                        }
                    }
                    if rj.starts_with("#[diagnostic::on_unimplemented") {
                        has_diag = true;
                    }
                    if depth <= 0 && i > start {
                        break;
                    }
                    i += 1;
                }
                if kernel && mname == "requirement" && !has_diag {
                    findings.push((
                        "E-REQ-DIAG",
                        Sev::Error,
                        "requirement trait defined without `#[diagnostic::on_unimplemented]` \
                         (R10 rule 8): modellers would see a bare trait-bound error instead \
                         of the REQ-phrased message"
                            .to_string(),
                        Loc { file: f.rel.clone(), line: start + 1 },
                    ));
                }
                pending = None;
                i += 1;
                continue;
            }

            // Doc lines: collect Satisfies tags.
            if raw.starts_with("///") {
                let body = raw.trim_start_matches('/').trim();
                if let Some(rest) = body.strip_prefix("Satisfies:") {
                    let ids: Vec<String> = rest
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| s.starts_with("REQ-"))
                        .collect();
                    if !ids.is_empty() {
                        pending = Some(Tag { ids, loc: Loc { file: f.rel.clone(), line: i + 1 } });
                    }
                }
                i += 1;
                continue;
            }
            if raw.starts_with("//") || raw.starts_with("#[") || code.is_empty() {
                i += 1;
                continue; // attributes/blanks keep a pending tag (trace.sh behaviour)
            }

            // A hand-written requirement trait outside the macro.
            if code.starts_with("pub trait Req")
                && code
                    .trim_start_matches("pub trait Req")
                    .chars()
                    .take(3)
                    .all(|c| c.is_ascii_digit())
            {
                let attrs = attr_block(f, i + 1);
                if !attrs.iter().any(|a| a.starts_with("#[diagnostic::on_unimplemented")) {
                    findings.push((
                        "E-REQ-DIAG",
                        Sev::Error,
                        "hand-written requirement trait without \
                         `#[diagnostic::on_unimplemented]` (R10 rule 8)"
                            .to_string(),
                        Loc { file: f.rel.clone(), line: i + 1 },
                    ));
                }
            }

            // Resolve a pending Satisfies tag against this item.
            if let Some(tag) = pending.take() {
                let is_fn = code.starts_with("pub fn ")
                    || code.starts_with("fn ")
                    || code.starts_with("pub(crate) fn ")
                    || code.contains(" fn ");
                let type_kw = ["pub struct ", "struct ", "pub enum ", "enum ", "pub type ", "type "]
                    .iter()
                    .find_map(|k| code.strip_prefix(k).map(|r| (k.trim(), r)));
                if is_fn {
                    // Tags on processes are grep-only by design (R10): fine.
                } else if let Some((_, rest)) = type_kw {
                    let name: String = rest
                        .trim_start()
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '_')
                        .collect();
                    let resolved = cl.cm.resolve_alias(&name);
                    for id in &tag.ids {
                        let backed = cl
                            .cm
                            .satisfies_types
                            .get(id)
                            .map(|ts| {
                                ts.iter().any(|t| {
                                    t == &name
                                        || cl.cm.resolve_alias(t) == resolved
                                        || cl.cm.resolve_alias(t) == name
                                })
                            })
                            .unwrap_or(false);
                        if !backed {
                            findings.push((
                                "E-TAG-ASSERT",
                                Sev::Error,
                                format!(
                                    "`Satisfies: {id}` on type `{name}` has no backing \
                                     `satisfies!` assertion in this crate — the tag can go \
                                     stale silently (R10/F-020)"
                                ),
                                tag.loc.clone(),
                            ));
                        }
                    }
                } else {
                    // Not a type or process: the tag cannot be compile-checked.
                    findings.push((
                        "E-TAG-ASSERT",
                        Sev::Warn,
                        format!(
                            "`Satisfies: {}` sits on an item the linter cannot verify (not a \
                             type or fn): a tag here has no `satisfies!` backing — move it to \
                             a type alias or the using process (R10/F-020)",
                            tag.ids.join(", ")
                        ),
                        tag.loc,
                    ));
                }
            }
            i += 1;
        }
    }
    for (check, sev, msg, loc) in findings {
        cl.push(check, sev, msg, Some(loc));
    }
}

// ---------------------------------------------------------------------------
// WARN checks.
// ---------------------------------------------------------------------------

fn check_r20_conventions(cl: &mut CrateLint, krate: &str) {
    let mut findings: Vec<(&'static str, String, Option<Loc>)> = Vec::new();

    // W-R20-DOC: missing first-line rustdoc purpose.
    for f in cl.cm.fns.values() {
        if !matches!(f.kind, FnKind::Process | FnKind::Boundary | FnKind::Flow)
            || !is_own(&f.loc, krate)
        {
            continue;
        }
        let first = f.docs.iter().find(|l| !l.is_empty());
        let missing = match first {
            None => true,
            Some(l) => {
                l.starts_with("Placeholder:") || l.starts_with("Satisfies:") || l.starts_with("Verifies:")
            }
        };
        if missing {
            findings.push((
                "W-R20-DOC",
                format!(
                    "`{}` has no one-line rustdoc purpose as its first doc line (R20 \
                     convention 2) — its generated document opens with a stated gap",
                    f.name
                ),
                Some(f.loc.clone()),
            ));
        }
    }

    // W-R20-PLACE: process-shaped pub fn outside processes/boundary.
    for f in cl.cm.fns.values() {
        if f.kind != FnKind::Other || !is_own(&f.loc, krate) || !f.loc.file.contains("/src/") {
            continue;
        }
        if f.module.iter().any(|m| m == "test_support" || m == "fixtures" || m == "tests") {
            continue;
        }
        // Only `pub fn` (pub(crate) conversions are internal by design, R9).
        let is_pub = cl
            .files
            .iter()
            .find(|sf| sf.rel == f.loc.file)
            .and_then(|sf| sf.lines.get(f.loc.line.saturating_sub(1)))
            .map(|l| l.trim_start().starts_with("pub fn "))
            .unwrap_or(false);
        if !is_pub || f.ret.is_empty() {
            continue;
        }
        let touches_resource = f.params.iter().any(|(_, ty)| {
            let b = cl.cm.resolve_alias(&base_name(ty));
            matches!(
                cl.cm.resources.get(&b).map(|r| r.kind),
                Some(ResKind::Container | ResKind::Consumable | ResKind::Reusable | ResKind::OutcomeToken)
            )
        });
        // A grouping constructor (F-046: the fn just builds a `#[must_use]`
        // grouping struct literal from values it already holds) is not a
        // process and legitimately lives beside its struct.
        let is_grouping_ctor = f.ret.len() == 1
            && f.body
                .as_deref()
                .map(|b| {
                    let rb = base_name(&f.ret[0]);
                    let t = b.trim();
                    !rb.is_empty()
                        && cl.cm.structs.contains_key(&rb)
                        && (t.starts_with(&format!("{rb} {{")) || t.starts_with("Self {"))
                })
                .unwrap_or(false);
        if touches_resource && !is_grouping_ctor {
            findings.push((
                "W-R20-PLACE",
                format!(
                    "`{}` is a process-shaped `pub fn` outside the `processes`/`boundary` \
                     modules (R20 convention 1) — the extraction cannot classify it, so \
                     diagrams and documents omit it",
                    f.name
                ),
                Some(f.loc.clone()),
            ));
        }
    }

    // W-R20-RECORD: record() strings that are not known fn names.
    let mut seen_records: BTreeSet<(String, String)> = BTreeSet::new();
    for f in cl.cm.fns.values() {
        let Some(body) = &f.body else { continue };
        let mut from = 0usize;
        while let Some(p) = body[from..].find("record(") {
            let at = from + p + "record(".len();
            let Some(q1r) = body[at..].find('"') else { break };
            let q1 = at + q1r + 1;
            let Some(q2r) = body[q1..].find('"') else { break };
            let name = body[q1..q1 + q2r].to_string();
            from = q1 + q2r + 1;
            if name.is_empty() || cl.cm.callable(&name).is_some() {
                continue;
            }
            if seen_records.insert((f.name.clone(), name.clone())) {
                findings.push((
                    "W-R20-RECORD",
                    format!(
                        "`record(.., \"{name}\", ..)` in `{}` names no known process fn (R20 \
                         convention 3) — the History event and any adjacent draw attribute \
                         to nothing in generated documents",
                        f.name
                    ),
                    Some(f.loc.clone()),
                ));
            }
        }
    }

    // W-R20-SUPPLIER: supplier item not recoverable.
    let own_suppliers: Vec<(String, Loc)> = cl
        .cm
        .suppliers
        .iter()
        .filter(|s| is_own(&s.loc, krate))
        .map(|s| (s.src.clone(), s.loc.clone()))
        .collect();
    let mut warned: BTreeSet<String> = BTreeSet::new();
    for (src, loc) in own_suppliers {
        // Recoverable the way the tracer recovers it: `supplier_item` (the
        // F-055 gap-1 paths), or a full-state alias whose contents argument
        // is a recognizable type-level list (one alias hop, `list_shape`).
        let mut recoverable = cl.cm.supplier_item(&src).is_some();
        if !recoverable {
            for rhs in cl.cm.aliases.values() {
                if base_name(rhs) == src
                    && generic_args(rhs)
                        .iter()
                        .any(|arg| list_shape(&cl.cm, arg).is_some())
                {
                    recoverable = true;
                    break;
                }
            }
        }
        if !recoverable && warned.insert(src.clone()) {
            findings.push((
                "W-R20-SUPPLIER",
                format!(
                    "the `Supplier` impl for `{src}` does not make its item recoverable (no \
                     concrete `type Item`, `Cons` head, fill machinery or full-state alias) \
                     — R20 convention 5 / F-055 gap 1"
                ),
                Some(loc),
            ));
        }
    }

    for (check, msg, loc) in findings {
        cl.push(check, Sev::Warn, msg, loc);
    }
}

/// W-R10-LINE: a requirement bound not on the `fn`-name source line.
fn check_req_bound_line(cl: &mut CrateLint, krate: &str) {
    let mut findings: Vec<(String, Option<Loc>)> = Vec::new();
    for f in cl.cm.fns.values() {
        if !matches!(f.kind, FnKind::Process | FnKind::Boundary | FnKind::Flow)
            || !is_own(&f.loc, krate)
        {
            continue;
        }
        let Some(sf) = cl.files.iter().find(|sf| sf.rel == f.loc.file) else { continue };
        let i = f.loc.line.saturating_sub(1);
        if i >= sf.lines.len() {
            continue;
        }
        let first_line = &sf.lines[i];
        // Macro-generated fns (outcome_token/draw_process) have no real
        // signature at their loc — the check only applies to literal `fn`s.
        if !contains_ident(first_line, "fn") || !contains_ident(first_line, &f.name) {
            continue;
        }
        let sig = join_header(sf, i);
        for (trait_name, id) in &cl.cm.req_by_trait {
            if contains_ident(&sig, trait_name) && !contains_ident(first_line, trait_name) {
                findings.push((
                    format!(
                        "`{}` binds requirement {id} (`{trait_name}`) off the `fn`-name source \
                         line (R10 rule 4): trace.sh's line-based grep attributes the use to \
                         the wrong line",
                        f.name
                    ),
                    Some(f.loc.clone()),
                ));
            }
        }
    }
    for (msg, loc) in findings {
        cl.push("W-R10-LINE", Sev::Warn, msg, loc);
    }
}

// ---------------------------------------------------------------------------
// INFO checks.
// ---------------------------------------------------------------------------

fn check_info(cl: &mut CrateLint, krate: &str, root: &Path) {
    let mut findings: Vec<(&'static str, String, Option<Loc>)> = Vec::new();

    // I-PLACEHOLDER: the open-items surface.
    for (name, r) in &cl.cm.resources {
        if r.placeholder && is_own(&r.loc, krate) {
            let text = if r.placeholder_text.is_empty() {
                "(no tag text)".to_string()
            } else {
                r.placeholder_text.join("; ")
            };
            findings.push((
                "I-PLACEHOLDER",
                format!("`{name}` — {}", text.replace('|', "/")),
                Some(r.loc.clone()),
            ));
        }
    }
    let mut seen_fn_ph: BTreeSet<(String, usize)> = BTreeSet::new();
    for f in cl.cm.fns.values() {
        if f.placeholder
            && is_own(&f.loc, krate)
            && matches!(f.kind, FnKind::Process | FnKind::Boundary | FnKind::Flow)
            && seen_fn_ph.insert((f.loc.file.clone(), f.loc.line))
        {
            let text = f
                .docs
                .iter()
                .filter_map(|l| l.strip_prefix("Placeholder:"))
                .map(|r| r.trim())
                .collect::<Vec<_>>()
                .join("; ");
            findings.push((
                "I-PLACEHOLDER",
                format!(
                    "fn `{}` — {}",
                    f.name,
                    if text.is_empty() { "(no tag text)" } else { &text }.replace('|', "/")
                ),
                Some(f.loc.clone()),
            ));
        }
    }

    // I-THIN-VERIFY: own requirements with < 3 verifying tests.
    let verifies = collect_verifies(root, krate);
    for (id, (trait_name, loc)) in &cl.cm.reqs {
        if !is_own(loc, krate) {
            continue;
        }
        let n = verifies.get(id).map(|v| v.len()).unwrap_or(0);
        if n < 3 {
            findings.push((
                "I-THIN-VERIFY",
                format!(
                    "{id} (`{trait_name}`) is verified by {n} tagged test{} (<3) — thin \
                     verification",
                    if n == 1 { "" } else { "s" }
                ),
                Some(loc.clone()),
            ));
        }
    }

    // I-WIDE-PROCESS: > 6 value parameters.
    for f in cl.cm.fns.values() {
        if f.kind == FnKind::Process && is_own(&f.loc, krate) && f.params.len() > 6 {
            findings.push((
                "I-WIDE-PROCESS",
                format!(
                    "process `{}` takes {} value parameters (>6) — a decomposition candidate \
                     or an honest join point (R9/F-024)",
                    f.name,
                    f.params.len()
                ),
                Some(f.loc.clone()),
            ));
        }
    }

    // I-NO-FLOW.
    if cl.flow_count == 0 {
        findings.push((
            "I-NO-FLOW",
            "no traced flow: no composite `pub fn` flow in src/flows.rs and no integration \
             test composing 2+ processes (R20 convention 7) — generated documents have no \
             observed instantiations"
                .to_string(),
            None,
        ));
    }

    for (check, msg, loc) in findings {
        cl.push(check, Sev::Info, msg, loc);
    }
}

/// Concrete types that can stand in for a generic with these bounds: the
/// satisfying types of any requirement trait named (R10), plus the self
/// types of local trait impls (sealed slot/characteristic traits).
fn bound_types(
    cm: &CrateModel,
    local_impls: &BTreeMap<String, Vec<String>>,
    bounds: &str,
) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for (trait_name, id) in &cm.req_by_trait {
        if contains_ident(bounds, trait_name) {
            for t in cm.satisfies_types.get(id).into_iter().flatten() {
                let tb = cm.resolve_alias(t);
                if !tb.is_empty() && !out.contains(&tb) {
                    out.push(tb);
                }
            }
        }
    }
    for id in crate::idents_in(bounds) {
        if let Some(tys) = local_impls.get(&id) {
            for t in tys {
                if !out.contains(t) {
                    out.push(t.clone());
                }
            }
        }
    }
    out
}

/// I-NO-SINK (F-035), cross-crate: a tripwired type with no in-workspace
/// production consumer. Evidence of consumption, anywhere in the workspace:
/// a `Consumer` impl for it; a production process/boundary fn that takes it
/// and does not return it (a transform or exit); a struct keeping it as
/// payload (F-040); or a `Consumer<..>`/`ConsumeList<..>` bound naming it.
fn check_sinks(lints: &mut [CrateLint], crates: &[String]) {
    // Build the workspace-wide consumption pool.
    let mut consumed: BTreeSet<String> = BTreeSet::new();
    for cl in lints.iter() {
        let cm = &cl.cm;
        for c in &cm.consumers {
            let b = cm.resolve_alias(&base_name(&c.item));
            if !b.is_empty() {
                consumed.insert(b);
            }
        }
        // Local trait impls (`impl OrderSlot for FlatWhite<186>`): a sealed
        // slot/characteristic impl is how a type enters a multi-type-bound
        // process (the F-055 conservative-omission shape), and model-core
        // documents `impl Recordable for X` as "History may consume this".
        let mut local_impls: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for f in &cl.files {
            for (i, c) in f.code.iter().enumerate() {
                let t = c.trim();
                if !t.starts_with("impl") || !t.contains(" for ") {
                    continue;
                }
                let header = join_header(f, i);
                let Some((lhs, rhs)) = impl_for_split(&header) else { continue };
                let trait_base = base_name(
                    lhs.rsplit(|ch: char| ch == ' ' || ch == '>')
                        .find(|s| !s.trim().is_empty())
                        .unwrap_or("")
                        .trim(),
                );
                let ty = cm.resolve_alias(&base_name(rhs.trim()));
                if trait_base.is_empty() || ty.is_empty() {
                    continue;
                }
                if trait_base == "Recordable" {
                    consumed.insert(ty.clone());
                }
                local_impls.entry(trait_base).or_default().push(ty);
            }
        }
        for s in cm.structs.values() {
            for (_, fty) in &s.fields {
                let b = cm.resolve_alias(&base_name(fty));
                if !b.is_empty() {
                    consumed.insert(b);
                }
            }
        }
        for f in cm.fns.values() {
            if !matches!(f.kind, FnKind::Process | FnKind::Boundary | FnKind::Flow) {
                continue;
            }
            // Expand returns one level through bundles and Result arms.
            let mut ret_bases: BTreeSet<String> = BTreeSet::new();
            let expand = |ty: &str, ret_bases: &mut BTreeSet<String>| {
                let b = cm.resolve_alias(&base_name(ty));
                if b.is_empty() {
                    return;
                }
                ret_bases.insert(b.clone());
                if let Some(si) = cm.structs.get(&b) {
                    for (_, fty) in &si.fields {
                        let fb = cm.resolve_alias(&base_name(fty));
                        if !fb.is_empty() {
                            ret_bases.insert(fb);
                        }
                    }
                }
                if let Some(ei) = cm.enums.get(&b) {
                    for (_, fields) in &ei.variants {
                        for (_, fty) in fields {
                            let fb = cm.resolve_alias(&base_name(fty));
                            if !fb.is_empty() {
                                ret_bases.insert(fb);
                            }
                        }
                    }
                }
            };
            for r in &f.ret {
                if base_name(r) == "Result" {
                    for arm in generic_args(r) {
                        expand(&arm, &mut ret_bases);
                    }
                } else {
                    expand(r, &mut ret_bases);
                }
            }
            for (_, pty) in &f.params {
                let b = cm.resolve_alias(&base_name(pty));
                if b.is_empty() {
                    continue;
                }
                if !ret_bases.contains(&b) {
                    consumed.insert(b.clone());
                }
                // A generic param consumes the types that can stand in for it
                // (the R10 pattern and sealed slot traits hide the literal
                // type name): `K: Req006PouredAtTheBoil` consumes
                // `BoilingKettle`; `W: OrderSlot` carries `FlatWhite` into a
                // product another sink consumes.
                if let Some(g) = f.generics.iter().find(|g| g.name == b) {
                    for t in bound_types(cm, &local_impls, &g.bounds) {
                        if !ret_bases.contains(&t) {
                            consumed.insert(t);
                        }
                    }
                }
            }
            // Supplier-taken items enter the process by value and must be
            // accounted there: `SupplyN<N, Taken = X>` / `Supplier<Item = X>`.
            for g in &f.generics {
                for key in ["Taken = ", "Item = "] {
                    let mut from = 0usize;
                    while let Some(p) = g.bounds[from..].find(key) {
                        let at = from + p + key.len();
                        let rest: String = g.bounds[at..]
                            .chars()
                            .take_while(|c| *c != ',' && *c != '>')
                            .collect();
                        let rest = rest.trim();
                        // Candidate item bases: the list item (one alias
                        // hop), the bare type, and the instantiating generic
                        // arguments (`FourOf<IssuedBolt>` names the item in
                        // its argument; `FourOf<B>`'s item is the generic
                        // `B`, resolved through its requirement bound).
                        let mut cands: Vec<String> = Vec::new();
                        if let Some((_, item)) = list_shape(cm, rest) {
                            cands.push(item);
                        }
                        cands.push(base_name(rest));
                        for a in generic_args(rest) {
                            cands.push(base_name(&a));
                        }
                        for cand in cands {
                            let b = cm.resolve_alias(&cand);
                            if b.is_empty() {
                                continue;
                            }
                            if cm.resources.contains_key(&b) {
                                consumed.insert(b);
                            } else if let Some(g2) =
                                f.generics.iter().find(|g2| g2.name == b)
                            {
                                for t in bound_types(cm, &local_impls, &g2.bounds) {
                                    consumed.insert(t);
                                }
                            }
                        }
                        from = at;
                    }
                }
            }
            // Consumer/ConsumeList bounds.
            for g in &f.generics {
                let bounds = &g.bounds;
                let mut from = 0usize;
                while let Some(p) = bounds[from..].find("Consumer<") {
                    let at = from + p;
                    let arg = generic_args(&bounds[at..]).first().cloned().unwrap_or_default();
                    let b = cm.resolve_alias(&base_name(&arg));
                    if !b.is_empty() {
                        consumed.insert(b);
                    }
                    from = at + "Consumer<".len();
                }
                if let Some(p) = bounds.find("ConsumeList<") {
                    if let Some(arg) = generic_args(&bounds[p..]).first() {
                        if let Some((_, item)) = list_shape(cm, arg) {
                            consumed.insert(cm.resolve_alias(&item));
                        }
                    }
                }
            }
        }
    }

    for (k, cl) in lints.iter_mut().enumerate() {
        let krate = &crates[k];
        let mut findings: Vec<(String, Loc)> = Vec::new();
        for (name, r) in &cl.cm.resources {
            if !is_own(&r.loc, krate)
                || !matches!(r.kind, ResKind::Container | ResKind::Consumable | ResKind::OutcomeToken)
            {
                continue;
            }
            if !consumed.contains(name) {
                findings.push((
                    format!(
                        "tripwired type `{name}` has no in-workspace production consumer \
                         (F-035): no `Consumer` impl, consuming process, payload keeper or \
                         consumer bound names it — INFO because a downstream crate outside \
                         this workspace may consume it"
                    ),
                    r.loc.clone(),
                ));
            }
        }
        for (msg, loc) in findings {
            cl.push("I-NO-SINK", Sev::Info, msg, Some(loc));
        }
    }
}

/// E-REQ-DUP (F-053), cross-crate: a REQ id defined in more than one crate.
fn check_req_dups(lints: &mut [CrateLint], crates: &[String]) {
    let mut by_id: BTreeMap<String, Vec<(usize, String, Loc)>> = BTreeMap::new();
    for (k, cl) in lints.iter().enumerate() {
        for (id, (trait_name, loc)) in &cl.cm.reqs {
            if is_own(loc, &crates[k]) {
                by_id.entry(id.clone()).or_default().push((k, trait_name.clone(), loc.clone()));
            }
        }
    }
    for (id, defs) in by_id {
        if defs.len() < 2 {
            continue;
        }
        let places: Vec<String> = defs
            .iter()
            .map(|(k, t, l)| format!("`{}` (`{t}`, {}:{})", crates[*k], l.file, l.line))
            .collect();
        for (k, _, loc) in &defs {
            lints[*k].push(
                "E-REQ-DUP",
                Sev::Error,
                format!(
                    "{id} is defined in {} workspace crates — trace.sh merges and shadows \
                     silently (F-053): {}",
                    defs.len(),
                    places.join("; ")
                ),
                Some(loc.clone()),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// ERROR-KNOWN downgrade.
// ---------------------------------------------------------------------------

fn apply_known(cl: &mut CrateLint, krate: &str) {
    for f in &mut cl.findings {
        if f.sev != Sev::Error {
            continue;
        }
        if let Some(k) = KNOWN
            .iter()
            .find(|k| k.check == f.check && k.krate == krate && f.msg.contains(k.ident))
        {
            f.sev = Sev::ErrorKnown;
            f.msg = format!("{} — **ERROR-KNOWN**: {}", f.msg, k.note);
        }
    }
}

// ---------------------------------------------------------------------------
// Report emission.
// ---------------------------------------------------------------------------

fn stamp(krate: &str) -> String {
    format!(
        "> Generated by `tools/lint.sh` (`tools/diagram-gen`'s `modellint` binary) from \
         `model/{krate}` — **do not hand-edit**; regenerate after any model change. Every \
         check cites the instructions.md rule or FINDINGS.md finding it enforces; the full \
         catalogue and severity ladder are in [README.md](README.md).\n"
    )
}

fn link(loc: &Option<Loc>) -> String {
    match loc {
        Some(l) => format!("[{}#L{}](/{}#L{})", l.file, l.line, l.file, l.line),
        None => "—".to_string(),
    }
}

fn cites_of(check: &str) -> &'static str {
    CATALOGUE.iter().find(|c| c.id == check).map(|c| c.cites).unwrap_or("")
}

fn counts(findings: &[Finding]) -> (usize, usize, usize, usize) {
    let c = |s: Sev| findings.iter().filter(|f| f.sev == s).count();
    (c(Sev::Error), c(Sev::ErrorKnown), c(Sev::Warn), c(Sev::Info))
}

fn section(s: &mut String, title: &str, blurb: &str, rows: &[&Finding]) {
    if rows.is_empty() {
        return;
    }
    let _ = writeln!(s, "\n## {title}\n");
    if !blurb.is_empty() {
        let _ = writeln!(s, "{blurb}\n");
    }
    let _ = writeln!(s, "| Check | Cites | Finding | Source |");
    let _ = writeln!(s, "|---|---|---|---|");
    for f in rows {
        let _ = writeln!(
            s,
            "| `{}` | {} | {} | {} |",
            f.check,
            cites_of(f.check),
            f.msg.replace('|', "/").replace('\n', " "),
            link(&f.loc)
        );
    }
}

fn crate_page(krate: &str, title: &str, findings: &[Finding]) -> String {
    let (ne, nk, nw, ni) = counts(findings);
    let mut s = String::new();
    let short = title
        .strip_prefix(krate)
        .map(|r| r.trim_start_matches([' ', '\u{2014}', '-']).to_string())
        .unwrap_or_else(|| title.to_string());
    let _ = writeln!(s, "# `{krate}` — model analysis\n");
    s.push_str(&stamp(krate));
    let _ = writeln!(s, "\n{short}\n");
    let _ = writeln!(
        s,
        "**{ne} error{} (gate-fatal) · {nk} error-known · {nw} warning{} · {ni} info item{}** \
         — the gate (`tools/lint.sh --check`, model/ci.sh step 7) fails only on gate-fatal \
         errors or stale reports.",
        if ne == 1 { "" } else { "s" },
        if nw == 1 { "" } else { "s" },
        if ni == 1 { "" } else { "s" },
    );

    let by = |s: Sev| findings.iter().filter(|f| f.sev == s).collect::<Vec<_>>();
    section(
        &mut s,
        "ERROR-KNOWN — true positives pending a maintainer decision",
        "These are real violations of the ERROR set found in committed model code. The \
         linter never edits model crates and never silently retunes a severity: each one is \
         explicitly listed in the tool (`tools/diagram-gen/src/lint.rs`, `KNOWN`), graded \
         **non-fatal**, and stays here — listed first — until the maintainer fixes the model \
         or amends the rule.",
        &by(Sev::ErrorKnown),
    );
    section(&mut s, "Errors (gate-fatal)", "", &by(Sev::Error));
    section(
        &mut s,
        "Warnings (should fix)",
        "Convention violations the extraction tooling depends on (R20, R10 rule 4) and \
         scanner WARNs. Reported, not gate-fatal.",
        &by(Sev::Warn),
    );
    section(
        &mut s,
        "Info (visibility — never gate-fatal)",
        "The improvement surface: open placeholders, thin verification, wide processes, \
         missing sinks.",
        &by(Sev::Info),
    );

    // Checks-run appendix.
    let _ = writeln!(s, "\n## Checks run\n");
    let _ = writeln!(s, "| Check | Severity | Cites | Result |");
    let _ = writeln!(s, "|---|---|---|---|");
    for c in CATALOGUE {
        let n = findings.iter().filter(|f| f.check == c.id).count();
        let known = findings
            .iter()
            .filter(|f| f.check == c.id && f.sev == Sev::ErrorKnown)
            .count();
        let result = if n == 0 {
            "clean".to_string()
        } else if known > 0 {
            format!("{n} finding{} ({known} error-known)", if n == 1 { "" } else { "s" })
        } else {
            format!("{n} finding{}", if n == 1 { "" } else { "s" })
        };
        let _ = writeln!(s, "| `{}` | {} | {} | {result} |", c.id, c.sev.name(), c.cites);
    }
    s
}

fn readme(rows: &[(String, String, usize, usize, usize, usize)]) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "# Model analysis reports\n");
    let _ = writeln!(
        s,
        "> Generated by `tools/lint.sh` (`tools/diagram-gen`'s `modellint` binary) — **do \
         not hand-edit**; regenerate after any model change.\n"
    );
    let _ = writeln!(
        s,
        "The model linter (PLAN step 11, candidate R21): one pass over every model crate \
         that unifies the completeness and style checks previously scattered across trace.sh \
         warnings, sealing greps and prose-only conventions. Everything factual is recovered \
         from source by the same shared scanner the diagram and docgen binaries use; where \
         the extraction cannot verify something it says so rather than guessing.\n"
    );
    let _ = writeln!(s, "**The severity ladder:**\n");
    let _ = writeln!(
        s,
        "- **ERROR** — a hard rule (R1 sealing, F-034, R10 rules 7/8, F-047, F-053). \
         Gate-fatal: `tools/lint.sh --check` (model/ci.sh step 7) exits nonzero."
    );
    let _ = writeln!(
        s,
        "- **ERROR-KNOWN** — a true ERROR-level positive in committed model code, explicitly \
         listed in the tool and graded non-fatal pending the maintainer's decision; it is \
         flagged at the top of the crate's page. The linter never edits model crates and \
         never silently retunes a severity."
    );
    let _ = writeln!(
        s,
        "- **WARN** — a should-fix convention violation (the R20 extraction conventions, R10 \
         rule 4) or a scanner WARN. Reported, not gate-fatal."
    );
    let _ = writeln!(
        s,
        "- **INFO** — visibility only: placeholder density, thin verification, wide \
         processes, untraced crates, F-035 sink coverage. Never fails.\n"
    );
    let _ = writeln!(s, "| Model crate | Errors | Error-known | Warnings | Info | Report |");
    let _ = writeln!(s, "|---|---|---|---|---|---|");
    let (mut te, mut tk, mut tw, mut ti) = (0, 0, 0, 0);
    for (name, title, ne, nk, nw, ni) in rows {
        let short = title
            .strip_prefix(name.as_str())
            .map(|r| r.trim_start_matches([' ', '\u{2014}', '-']).to_string())
            .unwrap_or_else(|| title.clone());
        let _ = writeln!(
            s,
            "| `{name}` — {} | {ne} | {nk} | {nw} | {ni} | [report]({name}.md) |",
            short.replace('|', "/")
        );
        te += ne;
        tk += nk;
        tw += nw;
        ti += ni;
    }
    let _ = writeln!(s, "| **workspace** | **{te}** | **{tk}** | **{tw}** | **{ti}** | |");

    let _ = writeln!(s, "\n## The check catalogue\n");
    let _ = writeln!(
        s,
        "Each check cites the instructions.md rule or FINDINGS.md finding it enforces. New \
         ERROR checks are added here when findings harden into hard rules.\n"
    );
    let _ = writeln!(s, "| Check | Severity | Cites | What it checks |");
    let _ = writeln!(s, "|---|---|---|---|");
    for c in CATALOGUE {
        let _ = writeln!(s, "| `{}` | {} | {} | {} |", c.id, c.sev.name(), c.cites, c.what);
    }

    let _ = writeln!(s, "\n## What the linter cannot see (stated, not guessed)\n");
    let _ = writeln!(
        s,
        "- **Conservation truth** — the const asserts themselves are checked by the compiler \
         at monomorphization (F-001), by ci.sh steps 1–2, never by this tool."
    );
    let _ = writeln!(
        s,
        "- **Tripwire presence** — the kernel macros generate the tripwire `Drop`s (F-008); \
         the linter trusts the macros rather than re-verifying their expansion."
    );
    let _ = writeln!(
        s,
        "- **model-core itself** — kernel infrastructure, not a model: its sealing is \
         macro-generated and it is gated by ci.sh steps 1–6; the linter covers the nine \
         modelling crates."
    );
    let _ = writeln!(
        s,
        "- **Downstream-implemented trait bounds seen from upstream**, F-056 quantum-clock \
         actors, and recursion internals — the known scanner limits (F-055 resolution notes); \
         they surface as `W-SCAN` warnings, never as silent gaps."
    );
    let _ = writeln!(
        s,
        "- **A `Satisfies:` tag on an item that is not a type or fn** cannot be \
         compile-checked; it is reported as a WARN under `E-TAG-ASSERT`, not guessed at."
    );

    let _ = writeln!(s, "\n## The gate and staleness\n");
    let _ = writeln!(
        s,
        "`model/ci.sh` step 7 runs `tools/lint.sh --check`: the reports are regenerated into \
         a temporary directory and diffed against this committed directory (stale reports \
         fail), then the gate fails on any gate-fatal ERROR. ERROR-KNOWN findings and \
         WARN/INFO never fail the gate.\n\nRegenerate after a model change:\n\n```sh\n\
         ./tools/lint.sh\n```\n\nThe sibling generators: diagrams in \
         [docs/diagrams](../diagrams/README.md) (`tools/diagrams.sh`), process documents in \
         [docs/processes](../processes/README.md) (`tools/docgen.sh`)."
    );
    s
}

// ---------------------------------------------------------------------------
// Orchestration.
// ---------------------------------------------------------------------------

/// Lint every crate, write the reports into `out_dir`, print a summary, and
/// return the number of gate-fatal ERROR findings across the workspace.
pub fn run(root: &Path, out_dir: &Path, crates: &[String]) -> usize {
    let core_locs = scan_core_locs(root);
    let mut lints: Vec<CrateLint> = Vec::new();

    for krate in crates {
        if !root.join("model").join(krate).is_dir() {
            eprintln!("ERROR: no such model crate: {krate}");
            std::process::exit(2);
        }
        let mut cm = scan_crate(root, krate);
        let flows = collect_traced_flows(&mut cm, &core_locs);
        let top = build_top_level(&mut cm, &core_locs);
        let mut scan_warnings: Vec<String> = cm.warnings.clone();
        for (title, g, _) in &flows {
            for w in &g.warnings {
                scan_warnings.push(format!("[flow {title}] {w}"));
            }
        }
        for w in &top.warnings {
            scan_warnings.push(format!("[top-level] {w}"));
        }
        let flow_count = flows.len();
        let files = own_files(root, krate);
        let (crossing, param_bases) = collect_crossing(&cm, krate);
        let mut cl =
            CrateLint { cm, files, findings: Vec::new(), flow_count, crossing, param_bases };

        check_f034(&mut cl);
        check_sealing(&mut cl, krate);
        check_crate_attrs(&mut cl, krate);
        check_tags_and_requirements(&mut cl);
        check_r20_conventions(&mut cl, krate);
        check_req_bound_line(&mut cl, krate);
        // W-SCAN: the scanner/tracer WARNs, deduped, surfaced per crate.
        // "no flow to trace" is covered by I-NO-FLOW (the agreed INFO grade).
        let mut seen = BTreeSet::new();
        for w in scan_warnings {
            if w.starts_with("no flow to trace") {
                continue;
            }
            if seen.insert(w.clone()) {
                cl.push("W-SCAN", Sev::Warn, format!("scanner: {w}"), None);
            }
        }
        check_info(&mut cl, krate, root);
        lints.push(cl);
    }

    check_req_dups(&mut lints, crates);
    check_sinks(&mut lints, crates);
    for (k, cl) in lints.iter_mut().enumerate() {
        apply_known(cl, &crates[k]);
    }

    std::fs::create_dir_all(out_dir).expect("create analysis output dir");
    let mut rows: Vec<(String, String, usize, usize, usize, usize)> = Vec::new();
    let mut fatal = 0usize;
    for (k, cl) in lints.iter().enumerate() {
        let krate = &crates[k];
        let (ne, nk, nw, ni) = counts(&cl.findings);
        fatal += ne;
        let page = crate_page(krate, &cl.cm.title, &cl.findings);
        write_out(out_dir, &format!("{krate}.md"), &page);
        println!(
            "{krate}: {ne} error(s), {nk} error-known, {nw} warning(s), {ni} info item(s)"
        );
        rows.push((krate.clone(), cl.cm.title.clone(), ne, nk, nw, ni));
    }
    write_out(out_dir, "README.md", &readme(&rows));
    if fatal > 0 {
        eprintln!("modellint: {fatal} gate-fatal ERROR finding(s) — see the reports");
    }
    fatal
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Block-form macro invocations are recognized for ANY path (the F-060
    /// broadening), and ordinary blocks are not.
    #[test]
    fn macro_invocation_detection() {
        assert_eq!(
            macro_invocation("model_core::requirement! {"),
            Some(("model_core::requirement".into(), "requirement".into()))
        );
        assert_eq!(
            macro_invocation("crate::model! {"),
            Some(("crate::model".into(), "model".into()))
        );
        assert_eq!(macro_invocation("lavoisier::model!{"), Some(("lavoisier::model".into(), "model".into())));
        assert_eq!(macro_invocation("macro_rules! model {"), None); // two tokens, not a path
        assert_eq!(macro_invocation("const {"), None);
        assert_eq!(macro_invocation("match outcome {"), None);
        assert_eq!(macro_invocation("pub struct Foo {"), None);
    }

    /// The model-shaped line heuristic catches plain and sugared item forms.
    #[test]
    fn model_shaped_detection() {
        assert!(is_model_shaped("pub trait Boiling: sealed::Sealed {"));
        assert!(is_model_shaped("fn pour_away(self, permit: BrewPermit) -> Kettle;"));
        assert!(is_model_shaped("pub struct BrewPermit {"));
        assert!(is_model_shaped("process fn boil [const G: u64] {"));
        assert!(is_model_shaped("flow test fn pour_cuppa_balances {"));
        assert!(is_model_shaped("impl Boiling for KettleAtTheBoil {"));
        assert!(!is_model_shaped("rust {"));
        assert!(!is_model_shaped("let kettle = fill_kettle(kettle, water);"));
        assert!(!is_model_shaped("assert = \"conservation violated\";"));
        assert!(!is_model_shaped("send drain <- water;"));
    }

    /// The F-060 negative fixture, read-only: EXP-13's `model!`-authored dsl
    /// arm — the whole-model macro front the old checks passed vacuously
    /// green — is flagged by the broadened E-TAG-MACRO (its `Satisfies:` tags
    /// sit inside the invocation) and by W-MACRO-MODEL (its model-shaped
    /// items do too).
    #[test]
    fn exp13_dsl_arm_is_flagged() {
        let path = format!(
            "{}/../../experiments/exp13-dsl-macro/dsl/src/model.rs",
            env!("CARGO_MANIFEST_DIR")
        );
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read the F-060 fixture {path}: {e}"));
        let lines: Vec<String> = text.lines().map(str::to_string).collect();
        let code: Vec<String> = lines.iter().map(|l| code_part(l)).collect();
        let mut cl = CrateLint {
            cm: CrateModel::default(),
            files: vec![SrcFile {
                rel: "experiments/exp13-dsl-macro/dsl/src/model.rs".into(),
                lines,
                code,
            }],
            findings: Vec::new(),
            flow_count: 0,
            crossing: BTreeSet::new(),
            param_bases: BTreeSet::new(),
        };
        check_tags_and_requirements(&mut cl);
        let tag_errors = cl
            .findings
            .iter()
            .filter(|f| f.check == "E-TAG-MACRO" && f.sev == Sev::Error)
            .count();
        let model_shaped = cl
            .findings
            .iter()
            .filter(|f| f.check == "W-MACRO-MODEL" && f.sev == Sev::Warn)
            .count();
        assert!(
            tag_errors >= 2,
            "the dsl arm's in-invocation Satisfies: tags must be flagged (F-037/F-060), got {tag_errors}"
        );
        assert!(
            model_shaped >= 5,
            "the dsl arm's in-invocation fn/trait/struct items must be flagged (F-060), got {model_shaped}"
        );
    }
}
