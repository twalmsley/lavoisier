//! Line-based scanner for a model crate's source (R10 grep discipline).
//!
//! Extracts: kernel-macro resource declarations, hand-written boundary
//! structs, aliases, `Consumer`/`Supplier` impls, requirement definitions and
//! `satisfies!` tags, `pub fn` signatures per module (`boundary` /
//! `processes` / composite flows), and the bodies of flow functions for the
//! tracer in `flow.rs`.

use crate::{
    CrateModel, ConsumerImpl, EnumInfo, FnDef, FnKind, GenericP, Loc, ResKind, ResourceDef,
    StructInfo, SupplierImpl, TokenDef, base_name, generic_args, split_top,
};
use std::collections::BTreeMap;
use std::path::Path;

/// Strip a `//` comment (incl. `///`) outside string literals.
pub(crate) fn code_part(line: &str) -> String {
    let mut out = String::new();
    let mut in_str = false;
    let mut prev = '\0';
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if in_str {
            out.push(c);
            if c == '"' && prev != '\\' {
                in_str = false;
            }
            prev = if c == '\\' && prev == '\\' { '\0' } else { c };
            continue;
        }
        match c {
            '"' => {
                in_str = true;
                out.push(c);
            }
            '/' if chars.peek() == Some(&'/') => break,
            _ => out.push(c),
        }
        prev = c;
    }
    out
}

/// `{` and `}` counts of a comment-stripped line, outside strings.
fn open_close(code: &str) -> (i64, i64) {
    let mut o = 0i64;
    let mut c_ = 0i64;
    let mut in_str = false;
    let mut prev = '\0';
    for c in code.chars() {
        if in_str {
            if c == '"' && prev != '\\' {
                in_str = false;
            }
            prev = if c == '\\' && prev == '\\' { '\0' } else { c };
            continue;
        }
        match c {
            '"' => in_str = true,
            '{' => o += 1,
            '}' => c_ += 1,
            _ => {}
        }
        prev = c;
    }
    (o, c_)
}

/// Net `{`/`}` delta of a comment-stripped line, outside strings.
fn brace_delta(code: &str) -> i64 {
    let (o, c) = open_close(code);
    o - c
}

/// Scan model-core for the line numbers of its public helper API (legend links).
pub fn scan_core_locs(root: &Path) -> BTreeMap<String, Loc> {
    let mut out = BTreeMap::new();
    let dir = root.join("model/model-core/src");
    for fname in ["boundary.rs", "common.rs", "history.rs"] {
        let path = dir.join(fname);
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        for (idx, line) in text.lines().enumerate() {
            let t = line.trim_start();
            if let Some(rest) = t.strip_prefix("pub fn ") {
                let name: String = rest
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                out.entry(name).or_insert(Loc {
                    file: format!("model/model-core/src/{fname}"),
                    line: idx + 1,
                });
            }
        }
    }
    // The History type itself (used as the Labour sink node).
    if let Ok(text) = std::fs::read_to_string(dir.join("history.rs")) {
        for (idx, line) in text.lines().enumerate() {
            if line.trim_start().starts_with("pub struct History") {
                out.insert(
                    "History".to_string(),
                    Loc { file: "model/model-core/src/history.rs".to_string(), line: idx + 1 },
                );
                break;
            }
        }
    }
    out
}

/// Scan one crate's own source (no dependency resolution).
fn scan_crate_own(root: &Path, krate: &str) -> CrateModel {
    let mut cm = CrateModel { name: krate.to_string(), ..Default::default() };
    let src = root.join("model").join(krate).join("src");
    let mut files: Vec<_> = std::fs::read_dir(&src)
        .expect("read src dir")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|e| e == "rs").unwrap_or(false))
        .collect();
    files.sort();
    for f in &files {
        let rel = crate::rel_path(root, f);
        // A file module IS a module (Rust semantics): seed the module stack
        // with the file stem so `src/processes.rs` counts as the `processes`
        // module (the multi-crate layout splits modules into files).
        let stem = f
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .filter(|s| s != "lib" && s != "mod");
        scan_file(&mut cm, f, &rel, true, false, stem.as_deref());
    }
    let tests_flows = root.join("model").join(krate).join("tests/flows.rs");
    if tests_flows.is_file() {
        let rel = crate::rel_path(root, &tests_flows);
        scan_file(&mut cm, &tests_flows, &rel, true, true, None);
    }
    // Crate title from lib.rs's `//! # name — title` line.
    let lib = src.join("lib.rs");
    if let Ok(text) = std::fs::read_to_string(&lib) {
        for line in text.lines() {
            let t = line.trim_start();
            if let Some(rest) = t.strip_prefix("//! # ") {
                cm.title = rest.trim().to_string();
                break;
            }
        }
        cm.lib_loc = Some(Loc { file: crate::rel_path(root, &lib), line: 1 });
    }
    if cm.title.is_empty() {
        cm.title = krate.to_string();
    }
    cm
}

/// Workspace model dependencies of a crate (path deps under `model/`,
/// excluding `model-core` — the fixed kernel stays the built-in table),
/// transitively, in deterministic discovery order.
pub fn model_deps(root: &Path, krate: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut queue: Vec<String> = vec![krate.to_string()];
    while let Some(k) = queue.pop() {
        let toml = root.join("model").join(&k).join("Cargo.toml");
        let Ok(text) = std::fs::read_to_string(&toml) else {
            continue;
        };
        let mut in_deps = false;
        for line in text.lines() {
            let t = line.trim();
            if t.starts_with('[') {
                in_deps = t == "[dependencies]";
                continue;
            }
            if !in_deps || !t.contains("path") {
                continue;
            }
            let name = t.split('=').next().unwrap_or("").trim().to_string();
            if name.is_empty() || name == "model-core" {
                continue;
            }
            if !out.contains(&name) {
                out.push(name.clone());
                queue.push(name);
            }
        }
    }
    out
}

/// Scan a model crate, resolving its workspace dependencies' models into the
/// result (F-055 ext. 7): upstream types, sinks, suppliers, requirements and
/// callable processes merge in (this crate's own items always win), while
/// upstream fns stay in `dep_fns` so they are never projected as this crate's
/// own processes.
pub fn scan_crate(root: &Path, krate: &str) -> CrateModel {
    let mut cm = scan_crate_own(root, krate);
    for dep in model_deps(root, krate) {
        if !root.join("model").join(&dep).join("src").is_dir() {
            cm.warn(format!("dependency crate `{dep}` not found under model/"));
            continue;
        }
        let d = scan_crate_own(root, &dep);
        merge_dep(&mut cm, d);
    }
    cm
}

fn merge_dep(cm: &mut CrateModel, d: CrateModel) {
    let dep_name = d.name.replace('-', "_");
    for (name, mut f) in d.fns {
        if matches!(f.kind, FnKind::Process | FnKind::Boundary | FnKind::Flow) {
            // Prefix the module path with the dep crate so fill-machinery
            // lookups stay namespaced per crate.
            f.module.insert(0, dep_name.clone());
            if !cm.fns.contains_key(&name) {
                if let Some(prev) = cm.dep_fns.get(&name) {
                    if prev.loc.file != f.loc.file {
                        cm.warn(format!(
                            "fn `{name}` is defined in two dependency crates; keeping {}",
                            prev.loc.file
                        ));
                    }
                } else {
                    cm.dep_fns.insert(name, f);
                }
            }
        }
    }
    for (k, v) in d.trait_fns {
        cm.trait_fns.entry(k).or_insert(v);
    }
    for (k, v) in d.resources {
        cm.resources.entry(k).or_insert(v);
    }
    for (k, v) in d.aliases {
        cm.aliases.entry(k).or_insert(v);
    }
    for (k, v) in d.structs {
        cm.structs.entry(k).or_insert(v);
    }
    for (k, v) in d.enums {
        cm.enums.entry(k).or_insert(v);
    }
    for c in d.consumers {
        if !cm.consumers.iter().any(|x| x.sink == c.sink && x.item == c.item) {
            cm.dep_consumers.insert(cm.consumers.len());
            cm.consumers.push(c);
        }
    }
    for s in d.suppliers {
        if !cm.suppliers.iter().any(|x| x.src == s.src && x.item == s.item) {
            cm.suppliers.push(s);
        }
    }
    for (k, v) in d.fill_items {
        cm.fill_items.entry(format!("{dep_name}::{k}")).or_insert(v);
    }
    for (k, v) in d.consts {
        cm.consts.entry(k).or_insert(v);
    }
    for (k, v) in d.reqs {
        cm.reqs.entry(k).or_insert(v);
    }
    for (k, v) in d.req_sentence {
        cm.req_sentence.entry(k).or_insert(v);
    }
    for (k, v) in d.req_by_trait {
        cm.req_by_trait.entry(k).or_insert(v);
    }
    for (k, v) in d.satisfies_types {
        let e = cm.satisfies_types.entry(k).or_default();
        for t in v {
            if !e.contains(&t) {
                e.push(t);
            }
        }
    }
    // Dep outcome tokens are the dep's own context crossings, not this
    // crate's: deliberately NOT merged.
}

struct FileCx<'a> {
    lines: Vec<&'a str>,
    code: Vec<String>,
    file: String,
    is_tests: bool,
    capture: bool,
}

fn scan_file(
    cm: &mut CrateModel,
    path: &Path,
    rel: &str,
    capture: bool,
    is_tests: bool,
    stem: Option<&str>,
) {
    let Ok(text) = std::fs::read_to_string(path) else {
        cm.warn(format!("cannot read {rel}"));
        return;
    };
    let lines: Vec<&str> = text.lines().collect();
    let code: Vec<String> = lines.iter().map(|l| code_part(l)).collect();
    let cx = FileCx { lines, code, file: rel.to_string(), is_tests, capture };

    let n = cx.lines.len();
    let mut mod_stack: Vec<String> = stem.map(|s| vec![s.to_string()]).unwrap_or_default();
    let seed_len = mod_stack.len();
    let mut pending_doc: Vec<String> = Vec::new();
    let mut pending_should_panic = false;
    let mut i = 0usize;
    while i < n {
        let t = cx.lines[i].trim_start();
        if let Some(d) = t.strip_prefix("///") {
            pending_doc.push(d.trim().to_string());
            i += 1;
            continue;
        }
        if t.starts_with("//") {
            i += 1;
            continue;
        }
        let c = cx.code[i].trim().to_string();
        if c.is_empty() {
            pending_doc.clear();
            i += 1;
            continue;
        }
        if c.starts_with("#[") || c.starts_with("#![") {
            // Attributes are single-line in this codebase (R10/F-021 keeps
            // them so); they do not detach a doc block from its item.
            if c.starts_with("#[should_panic") {
                pending_should_panic = true;
            }
            i += 1;
            continue;
        }
        // Any real item consumes the pending `#[should_panic]`.
        let took_should_panic = pending_should_panic;
        pending_should_panic = false;
        // Module open / close.
        if (c.starts_with("pub mod ") || c.starts_with("mod ") || c.starts_with("pub(crate) mod "))
            && c.ends_with('{')
        {
            let after = c.split("mod ").nth(1).unwrap_or("");
            let name: String =
                after.chars().take_while(|ch| ch.is_alphanumeric() || *ch == '_').collect();
            mod_stack.push(name);
            pending_doc.clear();
            i += 1;
            continue;
        }
        if c == "}" {
            if mod_stack.len() > seed_len {
                mod_stack.pop();
            }
            pending_doc.clear();
            i += 1;
            continue;
        }
        if c.starts_with("model_core::satisfies!") {
            parse_satisfies(cm, &c);
            pending_doc.clear();
            i += 1;
            continue;
        }
        if c.starts_with("model_core::") && c.contains('!') && c.ends_with('{') {
            let mname: String = c["model_core::".len()..]
                .chars()
                .take_while(|ch| ch.is_alphanumeric() || *ch == '_')
                .collect();
            i = scan_macro(cm, &cx, i, &mname, &mod_stack, &pending_doc);
            pending_doc.clear();
            continue;
        }
        if c.starts_with("pub use ") || c.starts_with("use ") {
            i = consume_to_semicolon(&cx, i);
            pending_doc.clear();
            continue;
        }
        if c.starts_with("pub type ") || c.starts_with("type ") {
            let (joined, end) = join_to_semicolon(&cx, i);
            parse_alias(cm, &joined);
            pending_doc.clear();
            i = end;
            continue;
        }
        if c.starts_with("pub const ") || c.starts_with("const ") {
            let (joined, end) = join_to_semicolon(&cx, i);
            parse_const_item(cm, &joined);
            pending_doc.clear();
            i = end;
            continue;
        }
        if c.contains("pub fn ")
            || c.starts_with("pub(crate) fn ")
            || (cx.is_tests && c.starts_with("fn "))
        {
            i = scan_fn(cm, &cx, i, &mod_stack, &pending_doc, took_should_panic);
            pending_doc.clear();
            continue;
        }
        if c.starts_with("pub struct ") || c.starts_with("struct ") {
            i = scan_struct(cm, &cx, i, &pending_doc);
            pending_doc.clear();
            continue;
        }
        if c.starts_with("pub enum ") || c.starts_with("enum ") {
            i = scan_enum(cm, &cx, i);
            pending_doc.clear();
            continue;
        }
        if c.starts_with("impl") {
            i = scan_impl(cm, &cx, i, &mod_stack);
            pending_doc.clear();
            continue;
        }
        if c.starts_with("pub trait ") || c.starts_with("trait ") {
            i = scan_trait(cm, &cx, i, &mod_stack);
            pending_doc.clear();
            continue;
        }
        // Anything else: skip a block if the line opens one, else skip the line.
        if brace_delta(&cx.code[i]) > 0 {
            i = skip_item(&cx, i);
        } else {
            i += 1;
        }
        pending_doc.clear();
    }
}

/// From line `i`, skip to the line after the construct closes: past the
/// matching `}` of the first `{` opened, or past a top-level `;` if no block
/// opens first. Handles one-liners (`pub trait Sealed {}`).
fn skip_item(cx: &FileCx, i: usize) -> usize {
    let mut depth = 0i64;
    let mut opened = false;
    let mut j = i;
    while j < cx.lines.len() {
        let (o, c) = open_close(&cx.code[j]);
        if o > 0 {
            opened = true;
        }
        depth += o - c;
        if opened && depth <= 0 {
            return j + 1;
        }
        if !opened && cx.code[j].trim_end().ends_with(';') {
            return j + 1;
        }
        j += 1;
    }
    j
}

fn consume_to_semicolon(cx: &FileCx, i: usize) -> usize {
    let mut j = i;
    while j < cx.lines.len() {
        if cx.code[j].trim_end().ends_with(';') {
            return j + 1;
        }
        j += 1;
    }
    j
}

fn join_to_semicolon(cx: &FileCx, i: usize) -> (String, usize) {
    let mut joined = String::new();
    let mut j = i;
    while j < cx.lines.len() {
        joined.push_str(cx.code[j].trim());
        joined.push(' ');
        if cx.code[j].trim_end().ends_with(';') {
            return (joined, j + 1);
        }
        j += 1;
    }
    (joined, j)
}

fn parse_alias(cm: &mut CrateModel, joined: &str) {
    // `pub type Name<..> = RHS ;`
    let Some(eq) = joined.find('=') else { return };
    let lhs = joined[..eq].trim().trim_start_matches("pub ").trim_start_matches("type ").trim();
    let name = base_name(lhs);
    let rhs = joined[eq + 1..].trim().trim_end_matches(';').trim_end().trim_end_matches(';');
    if !name.is_empty() {
        cm.aliases.insert(name, rhs.trim().to_string());
    }
}

/// `pub const NAME: u64 = <expr>;` — the model's named magnitudes (R7).
/// Evaluated where the expression is integer arithmetic over literals and
/// previously seen consts; `const _: () = ...` assertion backings are ignored.
fn parse_const_item(cm: &mut CrateModel, joined: &str) {
    let t = joined.trim().trim_start_matches("pub ").trim_start_matches("const ").trim();
    let Some(colon) = t.find(':') else { return };
    let name = t[..colon].trim().to_string();
    if name == "_" || name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return;
    }
    let rest = &t[colon + 1..];
    let Some(eq) = rest.find('=') else { return };
    let ty = rest[..eq].trim();
    if ty != "u64" {
        return;
    }
    let expr = rest[eq + 1..].trim().trim_end_matches(';').trim().to_string();
    let value = eval_const_expr(&expr, &cm.consts);
    cm.consts.insert(name, (expr, value));
}

/// Evaluate an integer expression over literals, known consts, `+ - * /` and
/// parentheses. Returns None on anything else (conservative).
pub fn eval_const_expr(
    expr: &str,
    consts: &std::collections::BTreeMap<String, (String, Option<u64>)>,
) -> Option<u64> {
    struct P<'a> {
        s: &'a [u8],
        i: usize,
        consts: &'a std::collections::BTreeMap<String, (String, Option<u64>)>,
    }
    impl<'a> P<'a> {
        fn ws(&mut self) {
            while self.s.get(self.i).map(|c| c.is_ascii_whitespace()).unwrap_or(false) {
                self.i += 1;
            }
        }
        fn atom(&mut self) -> Option<u64> {
            self.ws();
            match self.s.get(self.i)? {
                b'(' => {
                    self.i += 1;
                    let v = self.sum()?;
                    self.ws();
                    if self.s.get(self.i) == Some(&b')') {
                        self.i += 1;
                        Some(v)
                    } else {
                        None
                    }
                }
                c if c.is_ascii_digit() => {
                    let start = self.i;
                    while self
                        .s
                        .get(self.i)
                        .map(|c| c.is_ascii_alphanumeric() || *c == b'_')
                        .unwrap_or(false)
                    {
                        self.i += 1;
                    }
                    let tok = std::str::from_utf8(&self.s[start..self.i]).ok()?;
                    let clean: String = tok.chars().filter(|c| *c != '_').collect();
                    clean.parse().ok()
                }
                c if c.is_ascii_alphabetic() || *c == b'_' => {
                    let start = self.i;
                    while self
                        .s
                        .get(self.i)
                        .map(|c| c.is_ascii_alphanumeric() || *c == b'_')
                        .unwrap_or(false)
                    {
                        self.i += 1;
                    }
                    let tok = std::str::from_utf8(&self.s[start..self.i]).ok()?;
                    self.consts.get(tok).and_then(|(_, v)| *v)
                }
                _ => None,
            }
        }
        fn prod(&mut self) -> Option<u64> {
            let mut v = self.atom()?;
            loop {
                self.ws();
                match self.s.get(self.i) {
                    Some(b'*') => {
                        self.i += 1;
                        v = v.checked_mul(self.atom()?)?;
                    }
                    Some(b'/') => {
                        self.i += 1;
                        let d = self.atom()?;
                        v = v.checked_div(d)?;
                    }
                    _ => return Some(v),
                }
            }
        }
        fn sum(&mut self) -> Option<u64> {
            let mut v = self.prod()?;
            loop {
                self.ws();
                match self.s.get(self.i) {
                    Some(b'+') => {
                        self.i += 1;
                        v = v.checked_add(self.prod()?)?;
                    }
                    Some(b'-') => {
                        self.i += 1;
                        v = v.checked_sub(self.prod()?)?;
                    }
                    _ => return Some(v),
                }
            }
        }
    }
    let mut p = P { s: expr.as_bytes(), i: 0, consts };
    let v = p.sum()?;
    p.ws();
    if p.i == expr.len() { Some(v) } else { None }
}

/// Method signatures inside a `trait` block, consulted only for
/// qualified-path calls (`<T as Trait>::method(..)`) in traced flows.
fn scan_trait(cm: &mut CrateModel, cx: &FileCx, i: usize, mods: &[String]) -> usize {
    let end = skip_item(cx, i);
    let mut depth = 0i64;
    for j in i..end.min(cx.lines.len()) {
        let before = depth;
        depth += brace_delta(&cx.code[j]);
        let t = cx.code[j].trim();
        if before == 1 && t.starts_with("fn ") {
            let mut sig = String::new();
            for cj in cx.code.iter().take(end.min(cx.lines.len())).skip(j) {
                for ch in cj.chars() {
                    if ch == '{' || ch == ';' {
                        break;
                    }
                    sig.push(ch);
                }
                if cj.contains('{') || cj.contains(';') {
                    break;
                }
                sig.push(' ');
            }
            if let Some((name, generics, params, ret)) = parse_fn_sig(&sig) {
                cm.trait_fns.entry(name.clone()).or_insert(FnDef {
                    name,
                    module: mods.to_vec(),
                    loc: Loc { file: cx.file.clone(), line: j + 1 },
                    generics,
                    params,
                    ret,
                    kind: FnKind::Other,
                    body: None,
                    placeholder: false,
                    docs: Vec::new(),
                    should_panic: false,
                });
            }
        }
    }
    end
}

fn parse_satisfies(cm: &mut CrateModel, c: &str) {
    // `model_core::satisfies!(assert_req006, KettleAtTheBoil);`
    let Some(open) = c.find('(') else { return };
    let Some(close) = c.rfind(')') else { return };
    let args = split_top(&c[open + 1..close], ',');
    if args.len() != 2 {
        return;
    }
    let digits: String = args[0].chars().filter(|ch| ch.is_ascii_digit()).collect();
    if digits.is_empty() {
        return;
    }
    let id = format!("REQ-{digits}");
    let ty = base_name(&args[1]);
    if !ty.is_empty() {
        cm.satisfies_types.entry(id).or_default().push(ty);
    }
}

fn fn_kind(cx: &FileCx, mods: &[String], is_pub: bool) -> FnKind {
    if mods.iter().any(|m| m == "processes") {
        FnKind::Process
    } else if mods.iter().any(|m| m == "boundary") {
        FnKind::Boundary
    } else if cx.is_tests {
        FnKind::Test
    } else if cx.file.ends_with("src/flows.rs") && mods.len() <= 1 && is_pub {
        // Top-level `pub fn` in the flows module (the file stem seeds one
        // module level, so `mods` is `["flows"]` there).
        FnKind::Flow
    } else {
        FnKind::Other
    }
}

fn scan_fn(
    cm: &mut CrateModel,
    cx: &FileCx,
    i: usize,
    mods: &[String],
    docs: &[String],
    should_panic: bool,
) -> usize {
    // Join lines until the body `{` (or a `;` for a bodyless decl).
    let mut sig = String::new();
    let mut open: Option<(usize, usize)> = None; // (line, byte col of '{')
    let mut j = i;
    'outer: while j < cx.lines.len() {
        let cj = &cx.code[j];
        for (k, ch) in cj.char_indices() {
            if ch == '{' {
                sig.push_str(&cj[..k]);
                open = Some((j, k));
                break 'outer;
            }
            if ch == ';' {
                sig.push_str(&cj[..k]);
                break 'outer;
            }
        }
        sig.push_str(cj);
        sig.push(' ');
        j += 1;
    }
    let Some(def) = parse_fn_sig(&sig) else {
        cm.warn(format!("{}:{}: could not parse fn signature", cx.file, i + 1));
        return skip_item(cx, i);
    };
    let is_pub = sig.trim_start().starts_with("pub");
    let kind = fn_kind(cx, mods, is_pub);

    // Body.
    let (body, end) = match open {
        None => (None, j + 1),
        Some((jl, col)) => {
            let mut depth = 0i64;
            let mut body = String::new();
            let mut jj = jl;
            let mut end = jl + 1;
            while jj < cx.lines.len() {
                let seg: &str = if jj == jl { &cx.code[jj][col..] } else { &cx.code[jj] };
                depth += brace_delta(seg);
                if cx.capture {
                    body.push_str(seg);
                    body.push('\n');
                }
                if depth <= 0 {
                    end = jj + 1;
                    break;
                }
                jj += 1;
                end = jj;
            }
            // Trim the outer braces off the captured body.
            let body = if cx.capture {
                let b = body.trim();
                let b = b.strip_prefix('{').unwrap_or(b);
                let b = b.strip_suffix('}').unwrap_or(b);
                Some(b.to_string())
            } else {
                None
            };
            (body, end)
        }
    };

    let placeholder = docs.iter().any(|d| d.starts_with("Placeholder:"));
    let fnd = FnDef {
        name: def.0.clone(),
        module: mods.to_vec(),
        loc: Loc { file: cx.file.clone(), line: i + 1 },
        generics: def.1,
        params: def.2,
        ret: def.3,
        kind,
        body,
        placeholder,
        docs: docs.to_vec(),
        should_panic,
    };
    if cm.fns.contains_key(&def.0) {
        cm.warn(format!(
            "duplicate fn name `{}` ({}:{}); keeping the first occurrence",
            def.0,
            cx.file,
            i + 1
        ));
    } else {
        cm.fns.insert(def.0, fnd);
    }
    end
}

type ParsedSig = (String, Vec<GenericP>, Vec<(String, String)>, Vec<String>);

pub fn parse_fn_sig(sig: &str) -> Option<ParsedSig> {
    let fnpos = sig.find("fn ")?;
    let rest = &sig[fnpos + 3..];
    let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
    if name.is_empty() {
        return None;
    }
    let after = &rest[name.len()..];
    let mut idx = 0usize;
    let bytes: Vec<char> = after.chars().collect();
    // Generics.
    let mut generics = Vec::new();
    while idx < bytes.len() && bytes[idx].is_whitespace() {
        idx += 1;
    }
    if idx < bytes.len() && bytes[idx] == '<' {
        let mut depth = 0i64;
        let start = idx + 1;
        let mut endg = start;
        for (k, &ch) in bytes.iter().enumerate().skip(idx) {
            match ch {
                '<' => depth += 1,
                '>' => {
                    depth -= 1;
                    if depth == 0 {
                        endg = k;
                        break;
                    }
                }
                _ => {}
            }
        }
        let gtext: String = bytes[start..endg].iter().collect();
        for g in split_top(&gtext, ',') {
            let g = g.trim();
            let (is_const, g2) = match g.strip_prefix("const ") {
                Some(r) => (true, r),
                None => (false, g),
            };
            let (gname, bounds) = match g2.find(':') {
                Some(p) => (g2[..p].trim().to_string(), g2[p + 1..].trim().to_string()),
                None => (g2.trim().to_string(), String::new()),
            };
            generics.push(GenericP { name: gname, is_const, bounds });
        }
        idx = endg + 1;
    }
    // Params.
    while idx < bytes.len() && bytes[idx] != '(' {
        idx += 1;
    }
    if idx >= bytes.len() {
        return Some((name, generics, Vec::new(), Vec::new()));
    }
    let mut depth = 0i64;
    let pstart = idx + 1;
    let mut pend = pstart;
    for (k, &ch) in bytes.iter().enumerate().skip(idx) {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    pend = k;
                    break;
                }
            }
            _ => {}
        }
    }
    let ptext: String = bytes[pstart..pend].iter().collect();
    let mut params = Vec::new();
    for p in split_top(&ptext, ',') {
        if let Some(cpos) = find_top_colon(&p) {
            params.push((p[..cpos].trim().to_string(), p[cpos + 1..].trim().to_string()));
        }
    }
    // Return type and where clause.
    let tail: String = bytes[pend + 1..].iter().collect();
    let mut ret = Vec::new();
    let mut where_text = String::new();
    if let Some(arrow) = tail.find("->") {
        let mut rtext = tail[arrow + 2..].trim().to_string();
        if let Some(wp) = find_top_where(&rtext) {
            where_text = rtext[wp + 5..].trim().to_string();
            rtext = rtext[..wp].trim().to_string();
        }
        if rtext.starts_with('(') && rtext.ends_with(')') {
            ret = split_top(&rtext[1..rtext.len() - 1], ',');
        } else if !rtext.is_empty() && rtext != "()" {
            ret.push(rtext);
        }
    } else if let Some(wp) = find_top_where(&tail) {
        where_text = tail[wp + 5..].trim().to_string();
    }
    // Merge where-clause bounds on bare generic names into the generics list
    // (e.g. `purchase`'s `where V: Consumer<Money<PRICE>>`).
    if !where_text.is_empty() {
        for clause in split_top(&where_text, ',') {
            if let Some(cpos) = find_top_colon(&clause) {
                let lhs = clause[..cpos].trim();
                if lhs.chars().all(|c| c.is_alphanumeric() || c == '_') {
                    if let Some(g) = generics.iter_mut().find(|g| g.name == lhs) {
                        if g.bounds.is_empty() {
                            g.bounds = clause[cpos + 1..].trim().to_string();
                        } else {
                            g.bounds =
                                format!("{} + {}", g.bounds, clause[cpos + 1..].trim());
                        }
                    }
                }
            }
        }
    }
    Some((name, generics, params, ret))
}

/// First `:` at bracket depth 0 (skipping `::`).
fn find_top_colon(s: &str) -> Option<usize> {
    let b: Vec<char> = s.chars().collect();
    let mut depth = 0i64;
    let mut k = 0usize;
    while k < b.len() {
        match b[k] {
            '(' | '[' | '{' | '<' => depth += 1,
            ')' | ']' | '}' | '>' => depth -= 1,
            ':' if depth == 0 => {
                if k + 1 < b.len() && b[k + 1] == ':' {
                    k += 2;
                    continue;
                }
                return Some(k);
            }
            _ => {}
        }
        k += 1;
    }
    None
}

/// Position of a top-level ` where ` in a return-type tail.
fn find_top_where(s: &str) -> Option<usize> {
    let mut depth = 0i64;
    let b = s.as_bytes();
    let mut k = 0usize;
    while k + 6 <= s.len() {
        match b[k] {
            b'(' | b'[' | b'{' | b'<' => depth += 1,
            b')' | b']' | b'}' | b'>' => depth -= 1,
            b'w' if depth == 0 && s[k..].starts_with("where ") => {
                let before_ok = k == 0 || !(b[k - 1].is_ascii_alphanumeric() || b[k - 1] == b'_');
                if before_ok {
                    return Some(k);
                }
            }
            _ => {}
        }
        k += 1;
    }
    None
}

fn scan_struct(cm: &mut CrateModel, cx: &FileCx, i: usize, docs: &[String]) -> usize {
    // Join header until `{`, `(`-tuple, or `;`.
    let mut header = String::new();
    let mut body_open = false;
    let mut j = i;
    'outer: while j < cx.lines.len() {
        let cj = &cx.code[j];
        for (k, ch) in cj.char_indices() {
            if ch == '{' {
                header.push_str(&cj[..k]);
                body_open = true;
                break 'outer;
            }
            if ch == ';' || ch == '(' {
                header.push_str(&cj[..k]);
                break 'outer;
            }
        }
        header.push_str(cj);
        header.push(' ');
        j += 1;
    }
    let name_part = header
        .trim_start()
        .trim_start_matches("pub ")
        .trim_start_matches("struct ")
        .trim();
    let name = base_name(name_part);
    let generics: Vec<GenericP> = generic_args(name_part)
        .into_iter()
        .map(|g| {
            let g = g.trim().to_string();
            let (is_const, g2) =
                match g.strip_prefix("const ") { Some(r) => (true, r.to_string()), None => (false, g) };
            let (gname, bounds) = match g2.find(':') {
                Some(p) => (g2[..p].trim().to_string(), g2[p + 1..].trim().to_string()),
                None => {
                    // Strip a default (`Contents = Nil`).
                    let gg = g2.split('=').next().unwrap_or(&g2).trim().to_string();
                    (gg, String::new())
                }
            };
            GenericP { name: gname, is_const, bounds }
        })
        .collect();

    let mut fields = Vec::new();
    let end;
    if body_open {
        // Fields until the block closes.
        let mut depth = 0i64;
        let mut jj = j;
        loop {
            if jj >= cx.lines.len() {
                end = jj;
                break;
            }
            let seg = &cx.code[jj];
            let before = depth;
            depth += brace_delta(seg);
            if jj > j && before == 1 {
                let t = seg.trim().trim_start_matches("pub ").trim();
                if let Some(cpos) = find_top_colon(t) {
                    let fname = t[..cpos].trim().to_string();
                    let fty = t[cpos + 1..].trim().trim_end_matches(',').trim().to_string();
                    if !fname.starts_with('_') && !fname.is_empty() {
                        fields.push((fname, fty));
                    }
                }
            }
            if depth <= 0 {
                end = jj + 1;
                break;
            }
            jj += 1;
        }
    } else {
        end = consume_to_semicolon(cx, j.max(i));
    }

    if !name.is_empty() {
        let placeholder = docs.iter().any(|d| d.starts_with("Placeholder:"));
        cm.structs
            .insert(name.clone(), StructInfo { generics, fields, loc: Loc { file: cx.file.clone(), line: i + 1 } });
        cm.resources.entry(name).or_insert(ResourceDef {
            kind: ResKind::BoundaryObject,
            unit: None,
            placeholder,
            placeholder_text: placeholder_tags(docs),
            doc_first: docs.first().cloned().unwrap_or_default(),
            loc: Loc { file: cx.file.clone(), line: i + 1 },
        });
    }
    end
}

/// The `Placeholder: ...` tag lines of a doc block (tag text without the tag).
fn placeholder_tags(docs: &[String]) -> Vec<String> {
    docs.iter()
        .filter_map(|d| d.strip_prefix("Placeholder:").map(|r| r.trim().to_string()))
        .collect()
}

fn scan_enum(cm: &mut CrateModel, cx: &FileCx, i: usize) -> usize {
    // Header until '{'.
    let header = cx.code[i].trim().to_string();
    let name = base_name(
        header.trim_start_matches("pub ").trim_start_matches("enum ").trim_end_matches('{').trim(),
    );
    let mut variants: Vec<(String, Vec<(String, String)>)> = Vec::new();
    let mut depth = 0i64;
    let mut j = i;
    let mut cur: Option<(String, Vec<(String, String)>)> = None;
    loop {
        if j >= cx.lines.len() {
            break;
        }
        let seg = cx.code[j].trim().to_string();
        let before = depth;
        depth += brace_delta(&cx.code[j]);
        if j > i && !seg.is_empty() && !seg.starts_with("#[") {
            if before == 1 {
                // Variant line.
                if seg.starts_with('}') {
                    // end of enum handled below
                } else if seg.ends_with('{') {
                    let vname = base_name(seg.trim_end_matches('{').trim());
                    cur = Some((vname, Vec::new()));
                } else {
                    let vname = base_name(seg.trim_end_matches(',').trim());
                    if !vname.is_empty() {
                        variants.push((vname, Vec::new()));
                    }
                }
            } else if before == 2 {
                if seg.starts_with('}') {
                    if let Some(v) = cur.take() {
                        variants.push(v);
                    }
                } else if let Some(v) = cur.as_mut() {
                    let t = seg.trim_start_matches("pub ").trim();
                    if let Some(cpos) = find_top_colon(t) {
                        let fname = t[..cpos].trim().to_string();
                        let fty = t[cpos + 1..].trim().trim_end_matches(',').trim().to_string();
                        v.1.push((fname, fty));
                    }
                }
            }
        }
        if depth <= 0 && j > i {
            j += 1;
            break;
        }
        j += 1;
    }
    if !name.is_empty() {
        cm.enums.insert(name, EnumInfo { variants, loc: Loc { file: cx.file.clone(), line: i + 1 } });
    }
    j
}

fn scan_impl(cm: &mut CrateModel, cx: &FileCx, i: usize, mods: &[String]) -> usize {
    // Join header until '{' or ';'.
    let mut header = String::new();
    let mut has_block = false;
    let mut j = i;
    'outer: while j < cx.lines.len() {
        let cj = &cx.code[j];
        for (k, ch) in cj.char_indices() {
            if ch == '{' {
                header.push_str(&cj[..k]);
                has_block = true;
                break 'outer;
            }
            if ch == ';' {
                header.push_str(&cj[..k]);
                break 'outer;
            }
        }
        header.push_str(cj);
        header.push(' ');
        j += 1;
    }
    let loc = Loc { file: cx.file.clone(), line: i + 1 };

    // The generic parameter names declared by the impl itself
    // (`impl<H, T, const G: u64> ...`), to tell a concrete contents head
    // from a generic one (F-055 #1).
    let impl_generics: Vec<String> = {
        let after = header.trim_start().strip_prefix("impl").unwrap_or("");
        if after.trim_start().starts_with('<') {
            generic_args(&format!("X{}", after.trim_start()))
                .iter()
                .map(|g| {
                    let g = g.trim().trim_start_matches("const ").trim();
                    g.split(':').next().unwrap_or(g).trim().to_string()
                })
                .collect()
        } else {
            Vec::new()
        }
    };

    // Split the header into the implemented-trait side and the self-type
    // side at the top-level ` for ` — trait names mentioned in a where
    // clause (`where Bin: Consumer<..>`) must NOT register an impl.
    let (impl_lhs, impl_rhs) = match impl_for_split(&header) {
        Some((l, r)) => (l, r),
        None => (header.clone(), String::new()),
    };
    let self_ty_full = match find_top_where(&impl_rhs) {
        Some(w) => impl_rhs[..w].trim().to_string(),
        None => impl_rhs.trim().to_string(),
    };

    // `impl<..> Consumer<Item> for Sink` (possibly path-qualified,
    // `model_core::boundary::Consumer<..>`).
    if let Some(cpos) = find_trait_use(&impl_lhs, "Consumer") {
        if !self_ty_full.is_empty() {
            let item_ty = &impl_lhs[cpos..];
            let item = generic_args(item_ty).first().cloned().unwrap_or_default();
            let sink = base_name(&self_ty_full);
            if !sink.is_empty() && !item.is_empty() {
                cm.consumers.push(ConsumerImpl { sink, item, loc: loc.clone() });
            }
        }
    } else if impl_lhs.trim_end().ends_with(" Supplier")
        || impl_lhs.trim_end().ends_with("::Supplier")
    {
        {
            let self_ty = self_ty_full.as_str();
            let src = base_name(self_ty);
            // The contents head when the self type is `Src<Cons<Head, ..>>`
            // and the head is concrete (not one of the impl's own generics).
            let head = generic_args(self_ty)
                .first()
                .filter(|a| base_name(a) == "Cons")
                .and_then(|a| generic_args(a).first().map(|h| base_name(h)))
                .filter(|h| !h.is_empty() && !impl_generics.contains(h));
            // Look for `type Item = ...;` inside the block.
            let mut item = None;
            if has_block {
                let mut depth = 0i64;
                let mut jj = j;
                while jj < cx.lines.len() {
                    depth += brace_delta(&cx.code[jj]);
                    let t = cx.code[jj].trim();
                    if let Some(rest) = t.strip_prefix("type Item = ") {
                        item = Some(rest.trim_end_matches(';').trim().to_string());
                    }
                    if depth <= 0 && jj >= j {
                        break;
                    }
                    jj += 1;
                }
            }
            if !src.is_empty() {
                cm.suppliers.push(SupplierImpl { src, item, head, loc: loc.clone() });
            }
        }
    } else if impl_lhs.trim_end().ends_with(" Fill") && self_ty_full.starts_with("Cons<") {
        let cons_ty = self_ty_full.as_str();
        if let Some(arg) = generic_args(cons_ty).first() {
            let item = base_name(arg);
            if !item.is_empty() && item != "T" {
                let e = cm.fill_items.entry(mods.join("::")).or_default();
                if !e.contains(&item) {
                    e.push(item);
                }
            }
        }
    }

    if has_block { skip_item(cx, j) } else { j + 1 }
}

/// First top-level `" for "` in an impl header (outside any brackets):
/// splits the implemented-trait side from the self-type side.
pub(crate) fn impl_for_split(header: &str) -> Option<(String, String)> {
    let b = header.as_bytes();
    let mut depth = 0i64;
    let mut k = 0usize;
    while k + 5 <= header.len() {
        match b[k] {
            b'(' | b'[' | b'{' | b'<' => depth += 1,
            b')' | b']' | b'}' | b'>' => depth -= 1,
            b' ' if depth == 0 && header[k..].starts_with(" for ") => {
                return Some((header[..k].to_string(), header[k + 5..].to_string()));
            }
            _ => {}
        }
        k += 1;
    }
    None
}

/// Position just past the opening of `TraitName<` in an impl header, matched
/// as ` TraitName<` or `::TraitName<` (path-qualified impls, F-055 ext. 7),
/// returning the index of `TraitName` (so the `<..>` can be read).
fn find_trait_use(header: &str, trait_name: &str) -> Option<usize> {
    for pat in [format!(" {trait_name}<"), format!("::{trait_name}<")] {
        if let Some(p) = header.find(&pat) {
            return Some(p + pat.len() - trait_name.len() - 1);
        }
    }
    None
}

#[allow(clippy::too_many_lines)]
fn scan_macro(
    cm: &mut CrateModel,
    cx: &FileCx,
    i: usize,
    mname: &str,
    mods: &[String],
    pending_doc: &[String],
) -> usize {
    // Collect inner lines until the macro's block closes.
    let mut depth = 0i64;
    let mut inner_code: Vec<(usize, String)> = Vec::new();
    let mut inner_docs: Vec<String> = Vec::new();
    let mut j = i;
    let end;
    loop {
        if j >= cx.lines.len() {
            end = j;
            break;
        }
        let raw = cx.lines[j].trim_start();
        if j > i {
            if let Some(d) = raw.strip_prefix("///") {
                inner_docs.push(d.trim().to_string());
            } else if !raw.starts_with("//") {
                let c = cx.code[j].trim().to_string();
                if !c.is_empty() && !c.starts_with("#[") {
                    inner_code.push((j, c));
                }
            }
        }
        depth += brace_delta(&cx.code[j]);
        if depth <= 0 && j > i {
            end = j + 1;
            break;
        }
        j += 1;
    }
    // Drop the macro's own closing `}` line from inner_code if captured.
    if let Some(last) = inner_code.last() {
        if last.1 == "}" {
            inner_code.pop();
        }
    }

    let placeholder = inner_docs
        .iter()
        .chain(pending_doc.iter())
        .any(|d| d.starts_with("Placeholder:"));

    match mname {
        "container_resource" | "consumable_resource" | "reusable_resource" => {
            let kind = match mname {
                "container_resource" => ResKind::Container,
                "consumable_resource" => ResKind::Consumable,
                _ => ResKind::Reusable,
            };
            // The declaration is the first inner code line (doc lines and
            // attributes were already filtered out; `unit = ...` etc. follow).
            let decl = inner_code.first();
            if let Some((dl, dcode)) = decl {
                let name = base_name(
                    dcode.trim_end_matches(',').trim_end_matches('{').trim(),
                );
                let unit = inner_code.iter().find_map(|(_, c)| {
                    c.trim().strip_prefix("unit = ").map(|r| {
                        r.trim().trim_end_matches(',').trim_matches('"').to_string()
                    })
                });
                if !name.is_empty() {
                    let all_docs: Vec<String> =
                        pending_doc.iter().chain(inner_docs.iter()).cloned().collect();
                    cm.resources.insert(
                        name,
                        ResourceDef {
                            kind,
                            unit,
                            placeholder,
                            placeholder_text: placeholder_tags(&all_docs),
                            doc_first: inner_docs
                                .first()
                                .or(pending_doc.first())
                                .cloned()
                                .unwrap_or_default(),
                            loc: Loc { file: cx.file.clone(), line: dl + 1 },
                        },
                    );
                } else {
                    cm.warn(format!(
                        "{}:{}: could not read the resource name inside {}!",
                        cx.file,
                        i + 1,
                        mname
                    ));
                }
                // Held contents (generic form `Name { field: ty }`): record
                // them as struct fields for field-access typing.
                let mut fields = Vec::new();
                for (_, c) in &inner_code {
                    let t = c.trim();
                    if t.ends_with(',') || t.ends_with("},") {
                        if let Some(cpos) = find_top_colon(t) {
                            let fname = t[..cpos].trim().trim_start_matches("pub ").to_string();
                            if fname.chars().all(|ch| ch.is_alphanumeric() || ch == '_')
                                && !fname.is_empty()
                                && !t.contains(" = ")
                            {
                                fields.push((
                                    fname,
                                    t[cpos + 1..].trim().trim_end_matches(',').to_string(),
                                ));
                            }
                        }
                    }
                }
                if !fields.is_empty() {
                    let name2 = base_name(dcode.trim_end_matches(',').trim_end_matches('{').trim());
                    cm.structs.entry(name2).or_insert(StructInfo {
                        generics: Vec::new(),
                        fields,
                        loc: Loc { file: cx.file.clone(), line: *dl + 1 },
                    });
                }
            }
        }
        "outcome_token" => {
            let decl = inner_code.first();
            if let Some((dl, dcode)) = decl {
                let token = base_name(dcode.split('(').next().unwrap_or(""));
                let get = |key: &str| -> String {
                    inner_code
                        .iter()
                        .find_map(|(_, c)| {
                            c.trim()
                                .strip_prefix(&format!("{key} = "))
                                .map(|r| r.trim_end_matches(',').trim().to_string())
                        })
                        .unwrap_or_default()
                };
                let (s, f, e) = (get("success"), get("failure"), get("exit"));
                let loc = Loc { file: cx.file.clone(), line: dl + 1 };
                if !token.is_empty() {
                    let all_docs: Vec<String> =
                        pending_doc.iter().chain(inner_docs.iter()).cloned().collect();
                    cm.resources.insert(
                        token.clone(),
                        ResourceDef {
                            kind: ResKind::OutcomeToken,
                            unit: None,
                            placeholder,
                            placeholder_text: placeholder_tags(&all_docs),
                            doc_first: inner_docs
                                .first()
                                .or(pending_doc.first())
                                .cloned()
                                .unwrap_or_default(),
                            loc: loc.clone(),
                        },
                    );
                    for (fname, params, ret) in [
                        (s.clone(), vec![], vec![token.clone()]),
                        (f.clone(), vec![], vec![token.clone()]),
                        (e.clone(), vec![("token".to_string(), token.clone())], vec![]),
                    ] {
                        if fname.is_empty() {
                            continue;
                        }
                        cm.fns.insert(
                            fname.clone(),
                            FnDef {
                                name: fname,
                                module: {
                                    let mut m = mods.to_vec();
                                    m.push("boundary".to_string());
                                    m
                                },
                                loc: loc.clone(),
                                generics: Vec::new(),
                                params,
                                ret,
                                kind: FnKind::Boundary,
                                body: None,
                                placeholder: true,
                                docs: inner_docs.clone(),
                                should_panic: false,
                            },
                        );
                    }
                    cm.tokens.push(TokenDef {
                        token,
                        success_fn: s,
                        failure_fn: f,
                        exit_fn: e,
                        loc,
                    });
                }
            }
        }
        "draw_process" => {
            // `pub fn draw_funds: Account => Money,`
            if let Some((dl, dcode)) =
                inner_code.iter().find(|(_, c)| c.trim_start().starts_with("pub fn "))
            {
                let t = dcode.trim().trim_start_matches("pub fn ").trim();
                let name: String =
                    t.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
                let rest = t[name.len()..].trim_start_matches(':').trim();
                let mut halves = rest.splitn(2, "=>");
                let input = base_name(halves.next().unwrap_or("").trim());
                let output = base_name(
                    halves.next().unwrap_or("").trim().trim_end_matches(',').trim(),
                );
                if !name.is_empty() && !input.is_empty() && !output.is_empty() {
                    cm.fns.insert(
                        name.clone(),
                        FnDef {
                            name,
                            module: mods.to_vec(),
                            loc: Loc { file: cx.file.clone(), line: dl + 1 },
                            generics: vec![
                                GenericP { name: "TAKE".into(), is_const: true, bounds: String::new() },
                                GenericP { name: "LEFT".into(), is_const: true, bounds: String::new() },
                                GenericP { name: "FULL".into(), is_const: true, bounds: String::new() },
                            ],
                            params: vec![("container".to_string(), format!("{input}<FULL>"))],
                            ret: vec![format!("{output}<TAKE>"), format!("{input}<LEFT>")],
                            kind: if mods.iter().any(|m| m == "processes") {
                                FnKind::Process
                            } else {
                                FnKind::Other
                            },
                            body: None,
                            placeholder,
                            docs: inner_docs.clone(),
                            should_panic: false,
                        },
                    );
                }
            }
        }
        "requirement" => {
            let id = inner_docs.iter().find_map(|d| {
                let p = d.find("REQ-")?;
                let digits: String =
                    d[p + 4..].chars().take_while(|c| c.is_ascii_digit()).collect();
                if digits.len() == 3 { Some(format!("REQ-{digits}")) } else { None }
            });
            let trait_line = inner_code.iter().find(|(_, c)| c.contains("pub trait "));
            if let (Some(id), Some((tl, tcode))) = (id, trait_line) {
                let after = tcode.split("pub trait ").nth(1).unwrap_or("");
                let tname: String =
                    after.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
                if !tname.is_empty() {
                    // The requirement's doc sentence (`REQ-NNN: <sentence>`),
                    // for docgen's compliance section.
                    if let Some(sentence) = inner_docs
                        .iter()
                        .find_map(|d| d.strip_prefix(&format!("{id}:")).map(|r| r.trim().to_string()))
                    {
                        cm.req_sentence.insert(id.clone(), sentence);
                    }
                    cm.req_by_trait.insert(tname.clone(), id.clone());
                    cm.reqs.insert(id, (tname, Loc { file: cx.file.clone(), line: tl + 1 }));
                }
            }
        }
        _ => {}
    }
    end
}
