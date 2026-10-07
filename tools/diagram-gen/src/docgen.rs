//! docgen — candidate R20 made real: a generated, human-readable document
//! per process, detailed enough for real-world implementation by people.
//!
//! Everything factual comes from the scanned model (the same extraction the
//! diagram generator uses, F-055-hardened): process signatures are the
//! work-instruction skeletons, VALUE/const parameters supply the numbers,
//! `Placeholder:` tags are the open-items list, the requirement bounds plus
//! `Verifies:` tags are the compliance matrix (the trace.sh data, regenerated
//! not shelled), and the R9 connection graph states where each process fits
//! as needs/feeds — a dependency graph, not a fixed procedure.
//!
//! Conservative like diagram-gen: where the extraction cannot see something
//! it says so (stderr WARN + an HTML comment in the affected file) rather
//! than guessing. Idempotent; GitHub-viewable; root-relative links plus
//! absolute Mermaid click urls via `DIAGRAM_REPO_BLOB_BASE`.

use crate::emit::{
    PIn, build_top_level, collect_traced_flows, kind_name, legend, mermaid,
    resolve_param, resolve_outputs, sanitize_label, write_out,
};
use crate::flow::FlowGraph;
use crate::scan::{eval_const_expr, scan_crate};
use crate::{
    CrateModel, FnDef, FnKind, Loc, ResKind, as_number, base_name, contains_ident, generic_args,
    idents_in, split_top,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::Path;

// ---------------------------------------------------------------------------
// Per-crate collected data.
// ---------------------------------------------------------------------------

#[allow(dead_code)]
struct CrateData {
    cm: CrateModel,
    top: FlowGraph,
    /// (section title, graph, loc) — the same flows the detailed diagrams use.
    flows: Vec<(String, FlowGraph, Loc)>,
    /// REQ id -> verifying tests (name, loc), from `/// Verifies:` tags.
    verifies: BTreeMap<String, Vec<(String, Loc)>>,
    /// process name -> adjacent draw magnitudes (ms) observed in flows
    /// (the F-048 adjacent-draw convention: `draw_time::<T, ..>` followed by
    /// `record(history, "<process>", labour)`).
    draws: BTreeMap<String, BTreeSet<u64>>,
    /// fn name -> deduped edge labels observed in traced flows (in, out).
    in_labels: BTreeMap<String, BTreeSet<String>>,
    out_labels: BTreeMap<String, BTreeSet<String>>,
    /// fn name -> observed const-generic instantiations (deduped).
    calls: BTreeMap<String, Vec<BTreeMap<String, String>>>,
    /// Raw signature line per own fn (R10 rule 4: one line), for REQ greps.
    sig_lines: BTreeMap<String, String>,
    warnings: Vec<String>,
}

/// The crate a repo-relative source path belongs to (`model/<crate>/...`).
fn crate_of(file: &str) -> String {
    let mut it = file.split('/');
    match (it.next(), it.next()) {
        (Some("model"), Some(c)) => c.to_string(),
        _ => String::new(),
    }
}

/// Strip string literals from captured body text (process names quoted in
/// History records are not calls).
fn strip_strings(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut in_str = false;
    let mut prev = '\0';
    for c in body.chars() {
        if in_str {
            if c == '"' && prev != '\\' {
                in_str = false;
            }
            prev = if c == '\\' && prev == '\\' { '\0' } else { c };
            continue;
        }
        if c == '"' {
            in_str = true;
        } else {
            out.push(c);
        }
        prev = c;
    }
    out
}

/// `/// Verifies: REQ-...` tags across a crate's src/ and tests/ top-level
/// .rs files (the trace.sh discipline, R10 rules 2–3), regenerated here.
fn collect_verifies(root: &Path, krate: &str) -> BTreeMap<String, Vec<(String, Loc)>> {
    let mut out: BTreeMap<String, Vec<(String, Loc)>> = BTreeMap::new();
    let mut files: Vec<std::path::PathBuf> = Vec::new();
    for sub in ["src", "tests"] {
        let dir = root.join("model").join(krate).join(sub);
        if let Ok(rd) = std::fs::read_dir(&dir) {
            let mut v: Vec<_> = rd
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().map(|e| e == "rs").unwrap_or(false))
                .collect();
            v.sort();
            files.extend(v);
        }
    }
    for f in &files {
        let Ok(text) = std::fs::read_to_string(f) else { continue };
        let rel = crate::rel_path(root, f);
        let mut pending: Vec<String> = Vec::new();
        for (idx, line) in text.lines().enumerate() {
            let t = line.trim_start();
            if let Some(d) = t.strip_prefix("///") {
                let d = d.trim();
                if let Some(ids) = d.strip_prefix("Verifies:") {
                    for id in ids.split(',') {
                        let id = id.trim().to_string();
                        if id.starts_with("REQ-") {
                            pending.push(id);
                        }
                    }
                }
                continue;
            }
            if t.starts_with("//") || t.starts_with("#[") {
                continue;
            }
            if let Some(p) = t.find("fn ") {
                if p == 0 || t[..p].trim_end().ends_with("pub") || t[..p].trim_end().is_empty() {
                    let name: String = t[p + 3..]
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '_')
                        .collect();
                    if !name.is_empty() && !pending.is_empty() {
                        for id in pending.drain(..) {
                            out.entry(id).or_default().push((
                                name.clone(),
                                Loc { file: rel.clone(), line: idx + 1 },
                            ));
                        }
                        continue;
                    }
                }
            }
            if !t.is_empty() {
                pending.clear();
            }
        }
    }
    out
}

/// The F-048 adjacent-draw convention, read back from the captured flow
/// bodies: `draw_time::<TAKE, ..>` (or `qualified_draw_time::<TAKE, ..>`)
/// followed by `record(history, "<process>", labour)`.
fn collect_draws(cm: &CrateModel) -> BTreeMap<String, BTreeSet<u64>> {
    let mut out: BTreeMap<String, BTreeSet<u64>> = BTreeMap::new();
    for f in cm.fns.values() {
        let Some(body) = &f.body else { continue };
        let mut from = 0usize;
        while let Some(p) = body[from..].find("record(") {
            let at = from + p;
            // Second argument: the quoted process name.
            let args_start = at + "record(".len();
            let Some(q1) = body[args_start..].find('"') else { break };
            let q1 = args_start + q1 + 1;
            let Some(q2rel) = body[q1..].find('"') else { break };
            let process = body[q1..q1 + q2rel].to_string();
            // Nearest preceding draw turbofish.
            let before = &body[..at];
            let draw_pos = before
                .rfind("draw_time::<")
                .map(|x| x + "draw_time::<".len())
                .or_else(|| {
                    before
                        .rfind("qualified_draw_time::<")
                        .map(|x| x + "qualified_draw_time::<".len())
                });
            if let Some(dp) = draw_pos {
                let take: String = before[dp..]
                    .chars()
                    .take_while(|c| c.is_ascii_digit() || *c == '_')
                    .collect();
                let clean: String = take.chars().filter(|c| *c != '_').collect();
                if let Ok(ms) = clean.parse::<u64>() {
                    if !process.is_empty() {
                        out.entry(process).or_default().insert(ms);
                    }
                }
            }
            from = q1 + q2rel + 1;
        }
    }
    out
}

fn collect_crate_data(root: &Path, krate: &str, core_locs: &BTreeMap<String, Loc>) -> CrateData {
    let mut cm = scan_crate(root, krate);
    let flows = collect_traced_flows(&mut cm, core_locs);
    let top = build_top_level(&mut cm, core_locs);

    let mut in_labels: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut out_labels: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut calls: BTreeMap<String, Vec<BTreeMap<String, String>>> = BTreeMap::new();
    for (_, g, _) in &flows {
        for e in &g.edges {
            if e.from >= g.nodes.len() || e.to >= g.nodes.len() || e.label.is_empty() {
                continue;
            }
            out_labels
                .entry(g.nodes[e.from].fname.clone())
                .or_default()
                .insert(e.label.clone());
            in_labels
                .entry(g.nodes[e.to].fname.clone())
                .or_default()
                .insert(e.label.clone());
        }
        for (name, map) in &g.calls {
            let entry = calls.entry(name.clone()).or_default();
            if !map.is_empty() && !entry.contains(map) {
                entry.push(map.clone());
            }
        }
    }
    let draws = collect_draws(&cm);
    let verifies = collect_verifies(root, krate);

    // Raw signature lines (R10 rule 4 keeps requirement bounds on this line).
    let mut sig_lines: BTreeMap<String, String> = BTreeMap::new();
    let mut file_cache: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for f in cm.fns.values() {
        let lines = file_cache.entry(f.loc.file.clone()).or_insert_with(|| {
            std::fs::read_to_string(root.join(&f.loc.file))
                .map(|t| t.lines().map(str::to_string).collect())
                .unwrap_or_default()
        });
        if let Some(l) = lines.get(f.loc.line.saturating_sub(1)) {
            sig_lines.insert(f.name.clone(), l.clone());
        }
    }

    CrateData {
        cm,
        top,
        flows,
        verifies,
        draws,
        in_labels,
        out_labels,
        calls,
        sig_lines,
        warnings: Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// Rendering helpers.
// ---------------------------------------------------------------------------

fn link(loc: &Loc) -> String {
    format!("[{}#L{}](/{}#L{})", loc.file, loc.line, loc.file, loc.line)
}

/// Kind text for a base type, with the model-core reusables covered.
fn kind_text(cm: &CrateModel, base: &str) -> String {
    if matches!(base, "Person" | "Qualified") {
        return "reusable (model-core, R2/R15)".to_string();
    }
    if base == "History" {
        return "execution record (model-core, R16)".to_string();
    }
    cm.resources
        .get(base)
        .map(|r| res_kind_name(r.kind).to_string())
        .unwrap_or_else(|| "—".to_string())
}

fn res_kind_name(k: ResKind) -> &'static str {
    match k {
        ResKind::Container => "continuous (container, R15)",
        ResKind::Consumable => "discrete (consumable, R13)",
        ResKind::Reusable => "reusable (R2)",
        ResKind::OutcomeToken => "outcome token (R17)",
        ResKind::BoundaryObject => "boundary object (R12)",
    }
}

fn fn_kind_name(k: FnKind) -> &'static str {
    match k {
        FnKind::Process => "process (R1)",
        FnKind::Boundary => "boundary function (R12)",
        FnKind::Flow => "composite flow (R9)",
        _ => "function",
    }
}

/// Decimal magnitude text for one type's generic arguments: numeric literals
/// and evaluable named consts become numbers; const-generic names stay
/// caller-stated.
fn magnitudes(cm: &CrateModel, ty: &str, fdef: &FnDef) -> String {
    let args = generic_args(ty);
    let mut parts: Vec<String> = Vec::new();
    for a in &args {
        let a = a.trim();
        if let Some(n) = as_number(a) {
            parts.push(n);
        } else if let Some((_, Some(v))) = cm.consts.get(a) {
            parts.push(format!("{a} = {v}"));
        } else if fdef.generics.iter().any(|g| g.is_const && g.name == a) {
            parts.push(format!("`{a}` (caller-stated)"));
        }
    }
    if parts.is_empty() { "—".to_string() } else { parts.join(", ") }
}

fn unit_of(cm: &CrateModel, base: &str) -> String {
    cm.resources
        .get(base)
        .and_then(|r| r.unit.clone())
        .or_else(|| crate::builtin_unit(base).map(str::to_string))
        .unwrap_or_else(|| "—".to_string())
}

/// `const { assert!(EXPR, "MSG") }` occurrences in a captured body.
fn extract_asserts(body: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut from = 0usize;
    while let Some(p) = body[from..].find("const {") {
        let at = from + p + "const {".len();
        let rest = body[at..].trim_start();
        if let Some(a) = rest.strip_prefix("assert!(") {
            // Balanced to the closing paren.
            let mut depth = 1i64;
            let mut end = a.len();
            let mut in_str = false;
            let mut prev = '\0';
            for (k, c) in a.char_indices() {
                if in_str {
                    if c == '"' && prev != '\\' {
                        in_str = false;
                    }
                    prev = c;
                    continue;
                }
                match c {
                    '"' => in_str = true,
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            end = k;
                            break;
                        }
                    }
                    _ => {}
                }
                prev = c;
            }
            let args = &a[..end];
            let parts = split_top(args, ',');
            let expr = parts.first().cloned().unwrap_or_default();
            let msg = parts
                .get(1)
                .map(|m| m.trim().trim_matches('"').to_string())
                .unwrap_or_default();
            let expr: String = expr.split_whitespace().collect::<Vec<_>>().join(" ");
            out.push((expr, msg));
        }
        from = at;
    }
    out
}

/// Try to evaluate `LHS == RHS` numerically under a substitution map.
fn numeric_check(cm: &CrateModel, expr: &str, map: &BTreeMap<String, String>) -> Option<String> {
    let (lhs, rhs) = expr.split_once("==")?;
    let subst = |side: &str| -> Option<(String, u64)> {
        let s = crate::subst_idents(side.trim(), map);
        if s.contains("::") {
            return None; // associated consts: not statically resolvable here
        }
        let v = eval_const_expr(&s, &cm.consts)?;
        Some((s.split_whitespace().collect::<Vec<_>>().join(" "), v))
    };
    let (lt, lv) = subst(lhs)?;
    let (rt, rv) = subst(rhs)?;
    let holds = if lv == rv { "holds" } else { "DOES NOT HOLD" };
    Some(format!("`{lt} == {rt}` → {lv} = {rv} ({holds})"))
}

// ---------------------------------------------------------------------------
// The per-process document.
// ---------------------------------------------------------------------------

fn stamp(krate: &str) -> String {
    format!(
        "> Generated by `tools/docgen.sh` from `model/{krate}` — **do not hand-edit**; \
         regenerate after any model change. Links are into the defining source lines; \
         Mermaid nodes carry absolute click urls, the legend table the same links as \
         plain markdown.\n"
    )
}

#[allow(clippy::too_many_lines)]
fn process_doc(
    d: &CrateData,
    fdef: &FnDef,
    used_by: &BTreeMap<String, BTreeSet<String>>,
    warnings: &mut Vec<String>,
) -> String {
    let cm = &d.cm;
    let name = &fdef.name;
    let mut s = String::new();
    let mut doc_warns: Vec<String> = Vec::new();

    let _ = writeln!(s, "# `{}` — {}\n", name, fn_kind_name(fdef.kind));
    s.push_str(&stamp(&cm.name));

    // Purpose: the first rustdoc sentence (no invented prose — absence is
    // stated). Doc lines are hard-wrapped, so join up to the sentence end.
    let mut purpose = String::new();
    for l in &fdef.docs {
        if l.is_empty() && !purpose.is_empty() {
            break;
        }
        if !purpose.is_empty() {
            purpose.push(' ');
        }
        purpose.push_str(l);
        if let Some(p) = purpose.find(". ") {
            purpose.truncate(p + 1);
            break;
        }
        if purpose.ends_with('.') || purpose.ends_with(':') {
            break;
        }
    }
    if purpose.is_empty() {
        doc_warns.push(format!("`{name}` has no rustdoc purpose line"));
        let _ = writeln!(s, "\n*(No rustdoc purpose line on this item.)*\n");
    } else {
        let _ = writeln!(s, "\n**{}**\n", purpose.replace('|', "/"));
    }

    let short_title = cm
        .title
        .strip_prefix(cm.name.as_str())
        .map(|r| r.trim_start_matches([' ', '\u{2014}', '-']).to_string())
        .unwrap_or_else(|| cm.title.clone());
    let _ = writeln!(s, "- **Crate / subsystem:** `{}` — {}", cm.name, short_title);
    let _ = writeln!(s, "- **Source:** {}", link(&fdef.loc));
    let sig_reqs = signature_reqs(d, fdef);
    if !sig_reqs.is_empty() {
        let _ = writeln!(s, "- **Requirements bound on the signature:** {}", sig_reqs.join(", "));
    }
    for ph in fdef.docs.iter().filter_map(|l| l.strip_prefix("Placeholder:")) {
        let _ = writeln!(s, "- **Placeholder:** {}", ph.trim());
    }

    // ---- who and with what -------------------------------------------------
    let _ = writeln!(s, "\n## Who and with what\n");
    let mut actor_rows: Vec<String> = Vec::new();
    let mut threaded: Vec<String> = Vec::new();
    let mut boundary_lines: Vec<String> = Vec::new();
    let mut waste_inside: Vec<String> = Vec::new();
    let out_bases: BTreeSet<String> = resolve_outputs(cm, fdef).0.into_iter().map(|(b, _)| b).collect();
    for (pname, pty) in &fdef.params {
        match resolve_param(cm, fdef, pty) {
            PIn::Res(bases) => {
                for b in &bases {
                    if b == "Person" || b == "Qualified" {
                        let budget = magnitudes(cm, pty, fdef);
                        let draw = d
                            .draws
                            .get(name)
                            .map(|ms| {
                                ms.iter()
                                    .map(|m| format!("{m} ms"))
                                    .collect::<Vec<_>>()
                                    .join(" / ")
                            })
                            .unwrap_or_else(|| "— (no adjacent draw found in the traced flows)".to_string());
                        actor_rows.push(format!(
                            "| `{pname}` | `{b}` | {budget} | {draw} |",
                            pname = pname,
                        ));
                    } else if out_bases.contains(b)
                        && cm
                            .resources
                            .get(b)
                            .map(|r| matches!(r.kind, ResKind::Reusable))
                            .unwrap_or(false)
                    {
                        threaded.push(format!("`{pname}: {b}`"));
                    }
                }
            }
            PIn::Obj(b) => {
                let k = cm.resources.get(&b).map(|r| res_kind_name(r.kind)).unwrap_or("object");
                boundary_lines.push(format!("- `{pname}`: `{b}` ({k}), threaded by value (R2) — {}",
                    cm.resources.get(&b).map(|r| link(&r.loc)).unwrap_or_else(|| "—".into())));
            }
            PIn::Sink { sink, label } => {
                waste_inside.push(format!(
                    "- `{pname}` is a consumer parameter: **{label}** is handed to `{sink}` \
                     **inside this process** (Consumer/ConsumeList bound, R12) — {}",
                    cm.resources.get(&sink).map(|r| link(&r.loc)).unwrap_or_else(|| "—".into())
                ));
            }
            PIn::Supplier { src, label } => {
                let src_txt = match &src {
                    Some(x) => format!(
                        "`{x}` — {}",
                        cm.resources.get(x).map(|r| link(&r.loc)).unwrap_or_else(|| "—".into())
                    ),
                    None => "(supplier object not identified)".to_string(),
                };
                boundary_lines.push(format!(
                    "- `{pname}` supplies **{label}** from {src_txt} (SupplyN bound, R12)"
                ));
            }
            PIn::Unknown(u) => {
                doc_warns.push(format!("`{name}`: parameter `{pname}` not resolved ({u})"));
            }
        }
    }
    if actor_rows.is_empty() {
        let _ = writeln!(
            s,
            "No person in this signature: any time cost is drawn by an **adjacent** draw \
             process in the flow (R15/F-048), or the step needs no actor."
        );
        if let Some(ms) = d.draws.get(name) {
            let list = ms.iter().map(|m| format!("{m} ms")).collect::<Vec<_>>().join(" / ");
            let _ = writeln!(
                s,
                "Adjacent draw observed in the traced flows for this step: **{list}**."
            );
        }
    } else {
        let _ = writeln!(s, "| Actor | Type | Budget at entry | This step's adjacent draw (F-048) |");
        let _ = writeln!(s, "|---|---|---|---|");
        for r in &actor_rows {
            let _ = writeln!(s, "{r}");
        }
    }
    if !threaded.is_empty() {
        let _ = writeln!(
            s,
            "\nReusables threaded — moved in and returned (R2): {}.",
            threaded.join(", ")
        );
    }
    if !boundary_lines.is_empty() {
        let _ = writeln!(s, "\nBoundary objects used:\n");
        for l in &boundary_lines {
            let _ = writeln!(s, "{l}");
        }
    }

    // ---- inputs ------------------------------------------------------------
    let _ = writeln!(s, "\n## Inputs\n");
    let _ = writeln!(s, "| Parameter | Declared type | Resource kind | Unit | Magnitude |");
    let _ = writeln!(s, "|---|---|---|---|---|");
    for (pname, pty) in &fdef.params {
        let (shown, bases): (String, Vec<String>) = match resolve_param(cm, fdef, pty) {
            PIn::Res(bases) => (
                if bases.len() > 1 {
                    format!("`{pty}` → any of {}", bases.iter().map(|b| format!("`{b}`")).collect::<Vec<_>>().join(", "))
                } else if bases.len() == 1
                    && fdef.generics.iter().any(|g| !g.is_const && g.name == pty.trim())
                {
                    // A requirement-bound generic: name the satisfying type.
                    format!("`{pty}` → `{}` (via the requirement bound)", bases[0])
                } else {
                    format!("`{pty}`")
                },
                bases,
            ),
            PIn::Obj(b) => (format!("`{pty}`"), vec![b]),
            PIn::Sink { sink, label } => (
                format!("consumer of {label} (sink `{sink}`)"),
                vec![sink],
            ),
            PIn::Supplier { src, label } => (
                format!("supplier of {label}"),
                src.map(|x| vec![x]).unwrap_or_default(),
            ),
            PIn::Unknown(u) => (format!("`{pty}` — unresolved: {}", sanitize_label(&u)), Vec::new()),
        };
        let base = bases.first().cloned().unwrap_or_default();
        let kind = if base.is_empty() { "—".to_string() } else { kind_text(cm, &base) };
        let unit = if base.is_empty() { "—".to_string() } else { unit_of(cm, &base) };
        let mag = magnitudes(cm, pty, fdef);
        let _ = writeln!(
            s,
            "| `{pname}` | {} | {kind} | {unit} | {mag} |",
            shown.replace('|', "/")
        );
    }
    if let Some(labels) = d.in_labels.get(name) {
        let shown: Vec<String> = labels.iter().map(|l| format!("`{l}`")).collect();
        let _ = writeln!(s, "\nObserved entering this step in the traced flows: {}.", shown.join(", "));
    }

    // ---- outputs -----------------------------------------------------------
    let _ = writeln!(s, "\n## Outputs\n");
    let _ = writeln!(s, "| Output | Resource kind | Unit | Magnitude | Routing (who takes it) |");
    let _ = writeln!(s, "|---|---|---|---|---|");
    // Routing from the crate's top-level R9 graph.
    let mut routing: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (i, n) in d.top.nodes.iter().enumerate() {
        if n.fname != *name {
            continue;
        }
        for e in &d.top.edges {
            if e.from == i && !e.dashed && e.to < d.top.nodes.len() {
                let first = e.label.split(' ').next().unwrap_or("").to_string();
                let tgt = &d.top.nodes[e.to];
                routing
                    .entry(first)
                    .or_default()
                    .insert(format!("`{}` ({})", tgt.label, kind_name(tgt.kind)));
            }
        }
    }
    let mut listed: BTreeSet<String> = BTreeSet::new();
    for comp in &fdef.ret {
        let c = comp.trim();
        if c.ends_with("::Rest") || c.ends_with("::Next") {
            let _ = writeln!(
                s,
                "| `{}` | — | — | — | the same boundary object, returned in its advanced state (R2/R12) |",
                c.replace('|', "/")
            );
            continue;
        }
        let b = cm.resolve_alias(&base_name(c));
        if base_name(c) == "Result" {
            let arms = generic_args(c);
            for (k, armty) in arms.iter().enumerate() {
                let tag = if k == 0 { "on success" } else { "on failure" };
                let ab = base_name(armty);
                let _ = writeln!(
                    s,
                    "| `{}` ({tag}) | outcome bundle (R17 grouping, F-046) | — | — | destructured by the handling arm |",
                    ab.replace('|', "/")
                );
                if let Some(sinfo) = cm.structs.get(&ab) {
                    for (fname2, fty) in &sinfo.fields {
                        let fb = cm.resolve_alias(&base_name(fty));
                        let kind = kind_text(cm, &fb);
                        let unit = unit_of(cm, &fb);
                        let mag = magnitudes(cm, fty, fdef);
                        let route = routing
                            .get(&fb)
                            .map(|v| v.iter().cloned().collect::<Vec<_>>().join("; "))
                            .unwrap_or_else(|| "returned in the bundle".to_string());
                        let _ = writeln!(
                            s,
                            "| `{ab}.{fname2}: {fb}` ({tag}) | {kind} | {unit} | {mag} | {route} |"
                        );
                    }
                }
            }
            continue;
        }
        if b.is_empty() || listed.contains(&b) && fdef.ret.iter().filter(|r| cm.resolve_alias(&base_name(r)) == b).count() == 1 {
            if b.is_empty() {
                let _ = writeln!(s, "| `{}` | — | — | — | — |", c.replace('|', "/"));
                continue;
            }
        }
        listed.insert(b.clone());
        let kind = kind_text(cm, &b);
        let unit = unit_of(cm, &b);
        let mag = magnitudes(cm, c, fdef);
        let threaded_back = fdef
            .params
            .iter()
            .any(|(_, t)| cm.resolve_alias(&base_name(t)) == b || base_name(t) == b);
        let route = routing
            .get(&b)
            .map(|v| v.iter().cloned().collect::<Vec<_>>().join("; "))
            .unwrap_or_else(|| {
                if threaded_back {
                    "moved in and returned (R2)".to_string()
                } else {
                    "returned to the flow (the caller must account for it, R1)".to_string()
                }
            });
        let _ = writeln!(s, "| `{}` | {kind} | {unit} | {mag} | {route} |", c.replace('|', "/"));
    }
    if !waste_inside.is_empty() {
        let _ = writeln!(s, "\nWaste routed **inside** this process (never loose in a flow):\n");
        for l in &waste_inside {
            let _ = writeln!(s, "{l}");
        }
    }
    if let Some(labels) = d.out_labels.get(name) {
        let shown: Vec<String> = labels.iter().map(|l| format!("`{l}`")).collect();
        let _ = writeln!(s, "\nObserved leaving this step in the traced flows: {}.", shown.join(", "));
    }

    // ---- balances ----------------------------------------------------------
    let asserts = fdef.body.as_deref().map(extract_asserts).unwrap_or_default();
    // Conservation inherited by composition: a called process's own const
    // asserts fire at every instantiation here (F-001), including across the
    // crate edge (the multi-crate R1 layout).
    let mut inherited: Vec<(String, Loc, Vec<(String, String)>)> = Vec::new();
    if let Some(body) = &fdef.body {
        let code = strip_strings(body);
        for (cname, cdef) in cm.fns.iter().chain(cm.dep_fns.iter()) {
            if cname == name
                || !matches!(cdef.kind, FnKind::Process | FnKind::Flow | FnKind::Boundary)
            {
                continue;
            }
            if !(code.contains(&format!("{cname}(")) || code.contains(&format!("{cname}::<"))) {
                continue;
            }
            let subs = cdef.body.as_deref().map(extract_asserts).unwrap_or_default();
            if !subs.is_empty() {
                inherited.push((cname.clone(), cdef.loc.clone(), subs));
            }
        }
    }
    let structural: Vec<String> = fdef
        .generics
        .iter()
        .filter(|g| g.is_const)
        .filter(|g| {
            fdef.params.iter().any(|(_, t)| contains_ident(t, &g.name))
                && fdef.ret.iter().any(|r| contains_ident(r, &g.name))
        })
        .map(|g| g.name.clone())
        .collect();
    if !asserts.is_empty() || !structural.is_empty() || !inherited.is_empty() {
        let _ = writeln!(s, "\n## Balances (R1/R3/R15)\n");
        for (expr, msg) in &asserts {
            let _ = writeln!(s, "- **Compile-time assert:** `{expr}`");
            if !msg.is_empty() {
                let _ = writeln!(s, "  - message: *{}*", msg.replace('|', "/"));
            }
            let empty = BTreeMap::new();
            let mut maps: Vec<&BTreeMap<String, String>> =
                d.calls.get(name).map(|v| v.iter().collect()).unwrap_or_default();
            if maps.is_empty() {
                maps.push(&empty);
            }
            let mut checks: BTreeSet<String> = BTreeSet::new();
            for m in maps.iter().take(3) {
                if let Some(c) = numeric_check(cm, expr, m) {
                    checks.insert(c);
                }
            }
            for c in checks {
                let _ = writeln!(s, "  - in numbers (observed instantiation): {c}");
            }
            let _ = writeln!(
                s,
                "  - fires at monomorphization (F-001): `cargo build`/`cargo test` catch a \
                 violation; `cargo check` and editor diagnostics do not."
            );
        }
        for g in &structural {
            let _ = writeln!(
                s,
                "- **Structural:** the const parameter `{g}` appears in an input and an output \
                 — the magnitude flows through unchanged, checked by the shared type parameter \
                 (no assert needed)."
            );
        }
        for (cname, cloc, subs) in &inherited {
            let _ = writeln!(
                s,
                "- **Inherited by composition:** this step calls [`{cname}`](/{}#L{}), whose \
                 conservation asserts fire at every instantiation here (F-001):",
                cloc.file, cloc.line
            );
            for (expr, _) in subs {
                let _ = writeln!(s, "  - `{expr}`");
            }
        }
    }

    // ---- requirements ------------------------------------------------------
    if !sig_reqs.is_empty() || fdef.docs.iter().any(|l| l.starts_with("Satisfies:")) {
        let _ = writeln!(s, "\n## Requirements (R10 — the compliance matrix)\n");
        let _ = writeln!(s, "| Requirement | Statement | Defined at | Satisfied by | Verified by |");
        let _ = writeln!(s, "|---|---|---|---|---|");
        for id in &sig_reqs {
            let sentence = cm
                .req_sentence
                .get(id)
                .cloned()
                .unwrap_or_else(|| "—".to_string());
            let def = cm.reqs.get(id).map(|(_, l)| link(l)).unwrap_or_else(|| "—".to_string());
            let sat = cm
                .satisfies_types
                .get(id)
                .map(|ts| ts.iter().map(|t| format!("`{t}`")).collect::<Vec<_>>().join(", "))
                .unwrap_or_else(|| "—".to_string());
            let ver = d
                .verifies
                .get(id)
                .map(|ts| {
                    ts.iter()
                        .map(|(n, l)| format!("[`{n}`](/{}#L{})", l.file, l.line))
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_else(|| "— (no `Verifies:` tag found in this crate)".to_string());
            let _ = writeln!(
                s,
                "| {id} | {} | {def} | {sat} | {ver} |",
                sentence.replace('|', "/")
            );
        }
        let tags: Vec<String> = fdef
            .docs
            .iter()
            .filter_map(|l| l.strip_prefix("Satisfies:"))
            .map(|r| r.trim().to_string())
            .collect();
        if !tags.is_empty() {
            let _ = writeln!(s, "\nThis item is doc-tagged `Satisfies: {}`.", tags.join("; "));
        }
    }

    // ---- where it fits -----------------------------------------------------
    let _ = writeln!(s, "\n## Where it fits (R9: needs / feeds)\n");
    let _ = writeln!(
        s,
        "The model states **connections, not sequences**: any order satisfying these \
         dependencies is valid (R9).\n"
    );
    let mut needs: BTreeSet<String> = BTreeSet::new();
    let mut feeds: BTreeSet<String> = BTreeSet::new();
    for (i, n) in d.top.nodes.iter().enumerate() {
        if n.fname != *name {
            continue;
        }
        for e in &d.top.edges {
            if e.to == i && e.from < d.top.nodes.len() {
                let src = &d.top.nodes[e.from];
                if e.dashed {
                    needs.insert(format!(
                        "`{}` ({}) — threaded through, returned (R2)",
                        src.label,
                        kind_name(src.kind)
                    ));
                } else {
                    needs.insert(format!(
                        "**{}** from `{}` ({})",
                        e.label,
                        src.label,
                        kind_name(src.kind)
                    ));
                }
            }
            if e.from == i && e.to < d.top.nodes.len() && !e.dashed {
                let tgt = &d.top.nodes[e.to];
                feeds.insert(format!(
                    "**{}** to `{}` ({})",
                    e.label,
                    tgt.label,
                    kind_name(tgt.kind)
                ));
            }
        }
    }
    if needs.is_empty() && feeds.is_empty() {
        let _ = writeln!(
            s,
            "This function does not appear in the crate's top-level connection graph \
             (boundary setup and exits connect through the resources they mint or retire)."
        );
    }
    if !needs.is_empty() {
        let _ = writeln!(s, "**Needs:**\n");
        for x in &needs {
            let _ = writeln!(s, "- {x}");
        }
    }
    if !feeds.is_empty() {
        let _ = writeln!(s, "\n**Feeds:**\n");
        for x in &feeds {
            let _ = writeln!(s, "- {x}");
        }
    }
    if let Some(users) = used_by.get(&format!("{}::{}", cm.name, name)) {
        let _ = writeln!(s, "\n**Used across the crate edge by (R1 subsystem decomposition):**\n");
        for u in users {
            let _ = writeln!(s, "- {u}");
        }
    }

    // ---- neighbourhood diagram ----------------------------------------------
    let hood = neighbourhood(d, name);
    if let Some(g) = hood {
        let _ = writeln!(s, "\n## Neighbourhood (one hop)\n");
        s.push_str(&mermaid(&g, "LR"));
        let _ = writeln!(s, "\n### Legend — node → source\n");
        s.push_str(&legend(&g));
    }

    // ---- open items ---------------------------------------------------------
    let mut touched: BTreeSet<String> = BTreeSet::new();
    for (_, t) in &fdef.params {
        for id in idents_in(t) {
            touched.insert(cm.resolve_alias(&id));
        }
    }
    for r in &fdef.ret {
        for id in idents_in(r) {
            touched.insert(cm.resolve_alias(&id));
        }
    }
    for g in &fdef.generics {
        for id in idents_in(&g.bounds) {
            touched.insert(cm.resolve_alias(&id));
            if let Some(reqid) = cm.req_by_trait.get(&id) {
                if let Some(tys) = cm.satisfies_types.get(reqid) {
                    for t in tys {
                        touched.insert(cm.resolve_alias(t));
                    }
                }
            }
        }
    }
    let mut open: Vec<String> = Vec::new();
    for b in &touched {
        if let Some(r) = cm.resources.get(b) {
            if r.placeholder {
                let texts = if r.placeholder_text.is_empty() {
                    "(no tag text)".to_string()
                } else {
                    r.placeholder_text.join("; ")
                };
                open.push(format!("- `{b}` — {} ({})", texts.replace('|', "/"), link(&r.loc)));
            }
        }
    }
    if fdef.placeholder || !open.is_empty() {
        let _ = writeln!(s, "\n## Open items (`Placeholder:` tags, R12)\n");
        if fdef.placeholder {
            let _ = writeln!(s, "- this function is itself a placeholder (see the header above)");
        }
        for o in &open {
            let _ = writeln!(s, "{o}");
        }
    }

    // ---- warnings ----------------------------------------------------------
    if !doc_warns.is_empty() {
        s.push('\n');
        for w in &doc_warns {
            let _ = writeln!(s, "<!-- WARN (docgen): {} -->", w.replace("--", "–"));
            warnings.push(w.clone());
        }
    }
    s
}

/// REQ ids bound on this fn's signature line (the trace.sh method: R10 rule 4
/// keeps requirement bounds on the `fn`-name line).
fn signature_reqs(d: &CrateData, fdef: &FnDef) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let line = d.sig_lines.get(&fdef.name).cloned().unwrap_or_default();
    for (trait_name, id) in &d.cm.req_by_trait {
        let in_line = contains_ident(&line, trait_name);
        let in_bounds = fdef.generics.iter().any(|g| contains_ident(&g.bounds, trait_name));
        if (in_line || in_bounds) && !out.contains(id) {
            out.push(id.clone());
        }
    }
    out.sort();
    out
}

/// One-hop neighbourhood of the fn's node(s) in the top-level graph; None
/// when the fn does not appear there.
fn neighbourhood(d: &CrateData, name: &str) -> Option<FlowGraph> {
    let centers: Vec<usize> = d
        .top
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.fname == name)
        .map(|(i, _)| i)
        .collect();
    if centers.is_empty() {
        return None;
    }
    let mut keep: BTreeSet<usize> = centers.iter().cloned().collect();
    for e in &d.top.edges {
        if e.from >= d.top.nodes.len() || e.to >= d.top.nodes.len() {
            continue;
        }
        if centers.contains(&e.from) {
            keep.insert(e.to);
        }
        if centers.contains(&e.to) {
            keep.insert(e.from);
        }
    }
    let mut g = FlowGraph::default();
    let mut map: BTreeMap<usize, usize> = BTreeMap::new();
    for i in keep {
        let n = &d.top.nodes[i];
        let kind = if centers.contains(&i) { n.kind } else { n.kind };
        let ni = g.add_node(&n.label, &n.fname, kind, n.loc.clone());
        map.insert(i, ni);
    }
    for e in &d.top.edges {
        if e.from >= d.top.nodes.len() || e.to >= d.top.nodes.len() {
            continue;
        }
        if centers.contains(&e.from) || centers.contains(&e.to) {
            if let (Some(&f), Some(&t)) = (map.get(&e.from), map.get(&e.to)) {
                g.edge(f, t, e.label.clone(), e.dashed);
            }
        }
    }
    Some(g)
}

// ---------------------------------------------------------------------------
// Orchestration.
// ---------------------------------------------------------------------------

pub fn run(root: &Path, crates: &[String]) {
    let core_locs = crate::scan::scan_core_locs(root);
    let mut data: Vec<CrateData> = Vec::new();
    for krate in crates {
        if !root.join("model").join(krate).is_dir() {
            eprintln!("ERROR: no such model crate: {krate}");
            std::process::exit(1);
        }
        data.push(collect_crate_data(root, krate, &core_locs));
    }

    // Cross-crate usage: who calls whose processes across the crate edge.
    let mut used_by: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for d in &data {
        for f in d.cm.fns.values() {
            let Some(body) = &f.body else { continue };
            let code = strip_strings(body);
            for (dep_fn, df) in &d.cm.dep_fns {
                if code.contains(&format!("{dep_fn}(")) || code.contains(&format!("{dep_fn}::<")) {
                    let target_crate = crate_of(&df.loc.file);
                    if target_crate.is_empty() {
                        continue;
                    }
                    used_by
                        .entry(format!("{target_crate}::{dep_fn}"))
                        .or_default()
                        .insert(format!(
                            "`{}`'s `{}` ([{}#L{}](/{}#L{}))",
                            d.cm.name, f.name, f.loc.file, f.loc.line, f.loc.file, f.loc.line
                        ));
                }
            }
        }
    }

    let mut index_rows: Vec<(String, String, usize)> = Vec::new();
    let mut had_warnings = false;
    for d in &data {
        let out_dir = root.join("docs").join("processes").join(&d.cm.name);
        std::fs::create_dir_all(&out_dir).expect("create output dir");
        let mut documented: Vec<&FnDef> = d
            .cm
            .fns
            .values()
            .filter(|f| matches!(f.kind, FnKind::Process | FnKind::Boundary | FnKind::Flow))
            .collect();
        documented.sort_by(|a, b| (&a.loc.file, a.loc.line).cmp(&(&b.loc.file, b.loc.line)));

        let mut warnings: Vec<String> = d.cm.warnings.clone();
        for f in &documented {
            let doc = process_doc(d, f, &used_by, &mut warnings);
            write_out(&out_dir, &format!("{}.md", f.name), &doc);
        }

        // Per-crate index.
        let mut s = String::new();
        let _ = writeln!(s, "# `{}` — process documents\n", d.cm.name);
        s.push_str(&stamp(&d.cm.name));
        let _ = writeln!(s, "\n{} — {} documented functions. Diagrams: [context](../../diagrams/{}/context.md) · [top-level](../../diagrams/{}/top-level.md) · [detailed](../../diagrams/{}/detailed.md)\n",
            d.cm.title, documented.len(), d.cm.name, d.cm.name, d.cm.name);
        let _ = writeln!(s, "| Document | Kind | Purpose (first rustdoc line) | Source |");
        let _ = writeln!(s, "|---|---|---|---|");
        for f in &documented {
            let purpose = f
                .docs
                .first()
                .cloned()
                .unwrap_or_else(|| "—".to_string())
                .replace('|', "/");
            let _ = writeln!(
                s,
                "| [`{}`]({}.md) | {} | {} | {} |",
                f.name,
                f.name,
                fn_kind_name(f.kind),
                purpose,
                link(&f.loc)
            );
        }
        if !warnings.is_empty() {
            s.push('\n');
            let mut seen = BTreeSet::new();
            for w in &warnings {
                if seen.insert(w.clone()) {
                    let _ = writeln!(s, "<!-- WARN (docgen): {} -->", w.replace("--", "–"));
                }
            }
            had_warnings = true;
        }
        write_out(&out_dir, "README.md", &s);
        println!("{}: {} process documents", d.cm.name, documented.len());
        index_rows.push((d.cm.name.clone(), d.cm.title.clone(), documented.len()));
    }

    // Top index.
    let mut s = String::new();
    let _ = writeln!(s, "# Generated process documents\n");
    let _ = writeln!(
        s,
        "> Generated by `tools/docgen.sh` (`tools/diagram-gen`'s `docgen` binary) — **do not \
         hand-edit**; regenerate after any model change.\n"
    );
    let _ = writeln!(
        s,
        "One human-readable document per process, boundary function and composite flow, \
         generated from each model crate's source (candidate R20): the signature is the \
         work-instruction skeleton, the `VALUE`/const parameters supply the numbers, \
         `Placeholder:` tags are the open-items list, the requirement bounds with their \
         `Verifies:` tags are the compliance matrix, and the R9 connection graph states where \
         each process fits as **needs / feeds** — a dependency graph, not a fixed procedure. \
         Where the extraction cannot see something, the document says so (HTML comment) rather \
         than guessing.\n"
    );
    let _ = writeln!(s, "| Model crate | Documents | Index |");
    let _ = writeln!(s, "|---|---|---|");
    for (name, title, count) in &index_rows {
        let short = title
            .strip_prefix(name.as_str())
            .map(|r| r.trim_start_matches([' ', '—', '-']).to_string())
            .unwrap_or_else(|| title.clone());
        let _ = writeln!(
            s,
            "| `{name}` — {} | {count} | [index]({name}/README.md) |",
            short.replace('|', "/")
        );
    }
    let _ = writeln!(
        s,
        "\nRegenerate after a model change:\n\n```sh\n./tools/docgen.sh\n```\n\nThe diagrams \
         these documents link to live in [docs/diagrams](../diagrams/README.md) and are \
         regenerated by `tools/diagrams.sh`."
    );
    let dir = root.join("docs/processes");
    std::fs::create_dir_all(&dir).expect("create docs/processes");
    write_out(&dir, "README.md", &s);
    if had_warnings {
        eprintln!("(docgen completed with warnings — see the HTML comments in the output)");
    }
}
