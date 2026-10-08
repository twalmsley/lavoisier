//! diagram-gen — the shared extraction library behind the two generator
//! binaries: `diagram-gen` (GitHub-viewable Mermaid diagrams into
//! `docs/diagrams/<crate>/`, three levels per crate) and `docgen`
//! (human-readable per-process documents into `docs/processes/<crate>/`).
//!
//! Std-only and line-based by design: the project's R10 grep discipline
//! (requirement bounds on the `fn`-name line, conventional `boundary` /
//! `processes` child modules, rustfmt layout, the kernel resource macros)
//! is what makes this parsing reliable. Where the heuristics cannot see
//! something, the generators WARN (stderr + an HTML comment in the output)
//! rather than guessing.
//!
//! Multi-crate models (the R1 subsystem pattern, F-055 ext. 7): the scanner
//! also loads each workspace dependency's model (everything but `model-core`,
//! whose fixed helper API stays the built-in table below), so cross-crate
//! types, processes and sinks resolve instead of warning.

#![forbid(unsafe_code)]

pub mod docgen;
pub mod emit;
pub mod flow;
pub mod lint;
pub mod scan;

use std::collections::BTreeMap;
use std::path::Path;

// ---------------------------------------------------------------------------
// Data model: what the scanner extracts from a crate's source.
// ---------------------------------------------------------------------------

/// A location in the repository, for the legend links.
#[derive(Clone, Debug)]
pub struct Loc {
    /// Repo-relative path, forward slashes.
    pub file: String,
    /// 1-based line number.
    pub line: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ResKind {
    /// `container_resource!` — continuous amount `Name<const V: u64>`.
    Container,
    /// `consumable_resource!` — discrete item.
    Consumable,
    /// `reusable_resource!` — moved in and returned.
    Reusable,
    /// `outcome_token!` — R17 boundary token.
    OutcomeToken,
    /// A hand-written `pub struct` (boundary objects, permits, bundles).
    BoundaryObject,
}

#[derive(Clone, Debug)]
pub struct ResourceDef {
    pub kind: ResKind,
    /// The `unit = "..."` string from a container macro, if any.
    pub unit: Option<String>,
    /// Whether the doc block carries a `Placeholder:` tag.
    pub placeholder: bool,
    /// The `Placeholder: ...` tag text(s), for docgen's open-items list.
    pub placeholder_text: Vec<String>,
    /// First line of the doc block (the one-sentence purpose), for docgen.
    pub doc_first: String,
    pub loc: Loc,
}

#[derive(Clone, Debug)]
pub struct GenericP {
    pub name: String,
    pub is_const: bool,
    /// Bound text after the `:` (empty if none), e.g.
    /// `SupplyN<N3, Taken = ThreeDryBags>`.
    pub bounds: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FnKind {
    /// In a `processes` module: a model process (R1).
    Process,
    /// In a `boundary` module: creation/exit boundary (R12).
    Boundary,
    /// A `pub fn` at the top of `src/flows.rs`: a composite flow.
    Flow,
    /// A test / helper fn in a tests file.
    Test,
    Other,
}

#[derive(Clone, Debug)]
pub struct FnDef {
    pub name: String,
    pub module: Vec<String>,
    pub loc: Loc,
    pub generics: Vec<GenericP>,
    /// (param name, declared type).
    pub params: Vec<(String, String)>,
    /// Top-level return components (a tuple is flattened; unit return is empty).
    pub ret: Vec<String>,
    pub kind: FnKind,
    /// Captured body text (comments stripped) for flow tracing.
    pub body: Option<String>,
    pub placeholder: bool,
    /// The item's `///` doc block (trimmed lines), for docgen.
    pub docs: Vec<String>,
    /// `#[should_panic]` test: a counterexample (tripwire demo), not a flow.
    pub should_panic: bool,
}

#[derive(Clone, Debug)]
pub struct StructInfo {
    pub generics: Vec<GenericP>,
    /// (field name, type) — named fields only.
    pub fields: Vec<(String, String)>,
    pub loc: Loc,
}

#[derive(Clone, Debug)]
pub struct EnumInfo {
    /// (variant name, fields).
    pub variants: Vec<(String, Vec<(String, String)>)>,
    pub loc: Loc,
}

#[derive(Clone, Debug)]
pub struct ConsumerImpl {
    pub sink: String,
    /// Full item type, e.g. `WasteHeat<E>` / `Money<BOLT_BOX_PRICE_PENCE>`.
    pub item: String,
    pub loc: Loc,
}

#[derive(Clone, Debug)]
pub struct SupplierImpl {
    pub src: String,
    /// `type Item = X;` when concrete; None when generic (type-level list head).
    pub item: Option<String>,
    /// The head of the impl's self-type contents list when it is concrete
    /// (`HousingRack<Cons<Housing, T>>` -> `Housing`), F-055 #1.
    pub head: Option<String>,
    pub loc: Loc,
}

/// The `outcome_token!` boundary fns (R17): token in/out crossings.
#[derive(Clone, Debug)]
pub struct TokenDef {
    pub token: String,
    pub success_fn: String,
    pub failure_fn: String,
    pub exit_fn: String,
    pub loc: Loc,
}

#[derive(Default)]
pub struct CrateModel {
    pub name: String,
    /// One-line human title from `src/lib.rs` (`//! # name — title`).
    pub title: String,
    pub lib_loc: Option<Loc>,
    pub resources: BTreeMap<String, ResourceDef>,
    pub aliases: BTreeMap<String, String>,
    pub fns: BTreeMap<String, FnDef>,
    /// Callable fns of this crate's workspace dependencies (everything but
    /// model-core), for cross-crate flow tracing (F-055 ext. 7). Never
    /// projected as this crate's own processes.
    pub dep_fns: BTreeMap<String, FnDef>,
    /// Trait-method signatures (`fn` items inside `trait` blocks), consulted
    /// only for qualified-path calls (`<T as Trait>::method(..)`).
    pub trait_fns: BTreeMap<String, FnDef>,
    pub structs: BTreeMap<String, StructInfo>,
    pub enums: BTreeMap<String, EnumInfo>,
    pub consumers: Vec<ConsumerImpl>,
    /// Indices into `consumers` that came from a dependency crate (used to
    /// keep a subsystem's context diagram to the sinks it actually touches).
    pub dep_consumers: std::collections::BTreeSet<usize>,
    pub suppliers: Vec<SupplierImpl>,
    /// module-path key (joined with "::") -> item bases minted by that
    /// module's sealed `Fill` machinery (e.g. "DryTeabag").
    pub fill_items: BTreeMap<String, Vec<String>>,
    /// `pub const NAME: u64 = <expr>;` items: name -> (raw expr, evaluated).
    pub consts: BTreeMap<String, (String, Option<u64>)>,
    /// REQ id -> (trait name, loc).
    pub reqs: BTreeMap<String, (String, Loc)>,
    /// REQ id -> the requirement's doc sentence (`REQ-NNN: <sentence>`).
    pub req_sentence: BTreeMap<String, String>,
    /// requirement trait name -> REQ id.
    pub req_by_trait: BTreeMap<String, String>,
    /// REQ id -> type names tagged by `satisfies!`.
    pub satisfies_types: BTreeMap<String, Vec<String>>,
    pub tokens: Vec<TokenDef>,
    pub warnings: Vec<String>,
}

impl CrateModel {
    pub fn warn(&mut self, msg: String) {
        eprintln!("WARN [{}]: {}", self.name, msg);
        self.warnings.push(msg);
    }

    /// Resolve an alias chain to its base type name (stops on qualified paths).
    pub fn resolve_alias(&self, name: &str) -> String {
        let mut cur = name.to_string();
        for _ in 0..8 {
            match self.aliases.get(&cur) {
                Some(rhs) if !rhs.trim_start().starts_with('<') => {
                    let b = base_name(rhs);
                    if b.is_empty() || b == cur {
                        break;
                    }
                    cur = b;
                }
                _ => break,
            }
        }
        cur
    }

    /// The full alias RHS (one hop), if any.
    pub fn alias_rhs(&self, name: &str) -> Option<&String> {
        self.aliases.get(name)
    }

    /// Is this base name a boundary object (hand-written pub struct) or a
    /// reusable resource with Consumer/Supplier impls?
    pub fn is_sink_object(&self, base: &str) -> bool {
        self.consumers.iter().any(|c| c.sink == base)
    }

    pub fn is_supplier_object(&self, base: &str) -> bool {
        self.suppliers.iter().any(|s| s.src == base)
    }

    /// Items a sink object consumes (deduped base names, in impl order).
    pub fn sink_items(&self, sink: &str) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for c in &self.consumers {
            if c.sink == sink {
                let b = base_name(&c.item);
                if !out.contains(&b) {
                    out.push(b);
                }
            }
        }
        out
    }

    /// A callable fn by bare name: this crate's own first, then a workspace
    /// dependency's (F-055 ext. 7).
    pub fn callable(&self, name: &str) -> Option<&FnDef> {
        self.fns.get(name).or_else(|| self.dep_fns.get(name))
    }

    /// The item a supplier object hands out (F-055 #1): a concrete
    /// `type Item`; else the impl self-type's concrete contents head; else the
    /// item minted by the sealed fill machinery of the module whose boundary
    /// fn returns this object (disambiguated by the contents alias when the
    /// module fills several item kinds); else a resource named inside a
    /// full-state alias or constructor return type for this object.
    pub fn supplier_item(&self, src: &str) -> Option<String> {
        for s in &self.suppliers {
            if s.src == src {
                if let Some(it) = &s.item {
                    let b = base_name(it);
                    if !b.is_empty() && b != "H" && b != "B" {
                        return Some(self.resolve_alias(&b));
                    }
                }
                // The impl's own contents head, when concrete
                // (`HousingRack<Cons<Housing, T>>` names the item).
                if let Some(h) = &s.head {
                    return Some(self.resolve_alias(h));
                }
            }
        }
        // Candidate item bases named inside this object's full-state aliases
        // (`FullBoltBox = BoltBox<ListOf<IssuedBolt, N100>>`) and constructor
        // return types — used both to disambiguate a multi-item fill module
        // and as the final fallback.
        let mut named: Vec<String> = Vec::new();
        for rhs in self.aliases.values() {
            if base_name(rhs) == src {
                for id in idents_in(rhs) {
                    let b = self.resolve_alias(&id);
                    if b != src && self.resources.contains_key(&b) && !named.contains(&b) {
                        named.push(b);
                    }
                }
            }
        }
        // The fill machinery: a fn returning this object names the module
        // whose sealed `Fill` impls mint the items.
        for f in self.fns.values().chain(self.dep_fns.values()) {
            if f.ret.first().map(|r| self.resolve_alias(&base_name(r))) == Some(src.to_string()) {
                let key = f.module.join("::");
                if let Some(items) = self.fill_items.get(&key) {
                    if items.len() == 1 {
                        return Some(items[0].clone());
                    }
                    // Several item kinds filled in one module: pick the one
                    // the object's own contents name (deterministic).
                    for it in items {
                        let b = self.resolve_alias(it);
                        if named.contains(&b) || named.contains(it) {
                            return Some(it.clone());
                        }
                    }
                }
                // No fill machinery: a resource named in the constructor's
                // return type arguments is the held item.
                if let Some(r0) = f.ret.first() {
                    for id in idents_in(r0) {
                        let b = self.resolve_alias(&id);
                        if b != src && self.resources.contains_key(&b) {
                            return Some(b);
                        }
                    }
                }
            }
        }
        named.into_iter().next()
    }

    /// Requirement id (e.g. "REQ-006") mentioned in a bound string, if any.
    pub fn req_in_bounds(&self, bounds: &str) -> Option<String> {
        for (trait_name, id) in &self.req_by_trait {
            if contains_ident(bounds, trait_name) {
                return Some(id.clone());
            }
        }
        None
    }
}

// ---------------------------------------------------------------------------
// Small parsing utilities shared by scan/flow/emit.
// ---------------------------------------------------------------------------

/// Leading base identifier of a type: `Foo<Bar, 3>` -> `Foo`,
/// `crate::x::Foo<..>` -> `Foo`, `&Foo` -> `Foo`. Qualified paths
/// (`<X as T>::Assoc`) and generic projections return "".
pub fn base_name(ty: &str) -> String {
    let t = ty.trim().trim_start_matches('&').trim();
    if t.starts_with('<') {
        return String::new();
    }
    // Last path segment before any `<`.
    let head: &str = match t.find('<') {
        Some(i) => &t[..i],
        None => t,
    };
    let seg = head.rsplit("::").next().unwrap_or(head).trim();
    if seg.chars().all(|c| c.is_alphanumeric() || c == '_') {
        seg.to_string()
    } else {
        String::new()
    }
}

/// Generic arguments inside the outermost `<...>` of a type, split at top level.
pub fn generic_args(ty: &str) -> Vec<String> {
    let t = ty.trim();
    let Some(start) = t.find('<') else {
        return Vec::new();
    };
    let mut depth = 0i32;
    let bytes = t.as_bytes();
    let mut end = t.len();
    for (i, &b) in bytes.iter().enumerate().skip(start) {
        match b {
            b'<' => depth += 1,
            b'>' => {
                depth -= 1;
                if depth == 0 {
                    end = i;
                    break;
                }
            }
            _ => {}
        }
    }
    if end <= start + 1 {
        return Vec::new();
    }
    split_top(&t[start + 1..end], ',')
}

/// Split at top level, respecting `()`, `[]`, `{}`, `<>`.
pub fn split_top(s: &str, sep: char) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut cur = String::new();
    let mut prev = '\0';
    for c in s.chars() {
        match c {
            '(' | '[' | '{' | '<' => {
                // `->` is not an angle open; `<` after `-` cannot occur here.
                depth += 1;
                cur.push(c);
            }
            ')' | ']' | '}' => {
                depth -= 1;
                cur.push(c);
            }
            '>' => {
                if prev == '-' || prev == '=' {
                    // `->` / `=>`: the `-`/`=` was already pushed; not a close.
                    cur.push(c);
                } else {
                    depth -= 1;
                    cur.push(c);
                }
            }
            _ if c == sep && depth == 0 => {
                let p = cur.trim().to_string();
                if !p.is_empty() {
                    parts.push(p);
                }
                cur = String::new();
            }
            _ => cur.push(c),
        }
        prev = c;
    }
    let p = cur.trim().to_string();
    if !p.is_empty() {
        parts.push(p);
    }
    parts
}

/// All identifier tokens in a type string, in order (no dedup of case).
pub fn idents_in(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for c in s.chars().chain(std::iter::once('\0')) {
        if c.is_alphanumeric() || c == '_' {
            cur.push(c);
        } else {
            if !cur.is_empty() && cur.chars().next().map(|f| f.is_alphabetic()).unwrap_or(false) {
                out.push(cur.clone());
            }
            cur.clear();
        }
    }
    out
}

/// Whole-word identifier containment.
pub fn contains_ident(hay: &str, ident: &str) -> bool {
    let bytes = hay.as_bytes();
    let mut from = 0;
    while let Some(pos) = hay[from..].find(ident) {
        let i = from + pos;
        let before_ok = i == 0 || {
            let c = bytes[i - 1] as char;
            !(c.is_alphanumeric() || c == '_')
        };
        let j = i + ident.len();
        let after_ok = j >= hay.len() || {
            let c = bytes[j] as char;
            !(c.is_alphanumeric() || c == '_')
        };
        if before_ok && after_ok {
            return true;
        }
        from = i + ident.len();
    }
    false
}

/// Substitute whole-word identifiers in a type string by a map
/// (used to apply turbofish const arguments to return types).
pub fn subst_idents(ty: &str, map: &BTreeMap<String, String>) -> String {
    let mut out = String::new();
    let mut ident = String::new();
    for c in ty.chars().chain(std::iter::once('\0')) {
        if c.is_alphanumeric() || c == '_' {
            ident.push(c);
        } else {
            if !ident.is_empty() {
                match map.get(&ident) {
                    Some(v) => out.push_str(v),
                    None => out.push_str(&ident),
                }
                ident.clear();
            }
            if c != '\0' {
                out.push(c);
            }
        }
    }
    out
}

/// Normalize a numeric literal: `550_000` -> `550000`. Returns None if the
/// token is not a number. Also maps Peano aliases `N40` -> `40`.
pub fn as_number(tok: &str) -> Option<String> {
    let t = tok.trim();
    let t2 = t.strip_prefix('N').unwrap_or(t);
    let cleaned: String = t2.chars().filter(|c| *c != '_').collect();
    if !cleaned.is_empty() && cleaned.chars().all(|c| c.is_ascii_digit()) {
        // Require the `N` prefix form to have had a digit after N.
        if t.starts_with('N') && t2 == t {
            return None;
        }
        Some(cleaned)
    } else {
        None
    }
}

/// A type-level `Cons` list (possibly behind one alias hop):
/// returns (count, item base) if recognizable.
pub fn list_shape(cm: &CrateModel, ty: &str) -> Option<(usize, String)> {
    let mut t = ty.trim().to_string();
    let b = base_name(&t);
    if b != "Cons" {
        if let Some(rhs) = cm.alias_rhs(&b) {
            t = rhs.clone();
        } else {
            return None;
        }
    }
    if base_name(&t) != "Cons" {
        return None;
    }
    let count = t.matches("Cons").count();
    let args = generic_args(&t);
    let item = args.first().map(|a| base_name(a))?;
    if item.is_empty() {
        return None;
    }
    Some((count, item))
}

// ---------------------------------------------------------------------------
// Built-in knowledge of the fixed model-core boundary/helper API.
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub enum BOut {
    /// A fresh output; `{0}`/`{1}`... are turbofish argument slots.
    Fresh(&'static str),
    /// The same resource as input slot N, passed through.
    Pass(usize),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BKind {
    /// model-core boundary constructor (resources enter/leave the model).
    Boundary,
    /// Time/history bookkeeping (dropped from the top-level projection).
    Helper,
    /// `send_to` / `send_list`: collapses into an edge to the sink.
    RoutingToSink,
    /// `take_one` / `take_n`: supply from a supplier object.
    TakeFromSupplier,
}

pub struct Builtin {
    pub kind: BKind,
    pub outs: &'static [BOut],
    /// model-core file the legend links to (resolved to a line by scanning).
    pub file_hint: &'static str,
}

pub fn builtin(name: &str) -> Option<Builtin> {
    use BOut::{Fresh, Pass};
    let b = |kind, outs: &'static [BOut], file_hint| Builtin { kind, outs, file_hint };
    Some(match name {
        "new_person" => b(BKind::Boundary, &[Fresh("Person<{0}>")], "common.rs"),
        "qualify" => b(BKind::Boundary, &[Fresh("Qualified<{0}, {1}>")], "common.rs"),
        "release" => b(BKind::Boundary, &[Fresh("Person<{1}>")], "common.rs"),
        // The person comes back at the post-draw budget (`LEFT`, slot {1}).
        "draw_time" => b(BKind::Helper, &[Fresh("Labour<{0}>"), Fresh("Person<{1}>")], "common.rs"),
        "qualified_draw_time" => {
            b(BKind::Helper, &[Fresh("Labour<{0}>"), Fresh("Qualified<{3}, {1}>")], "common.rs")
        }
        "new_history" => b(BKind::Helper, &[Fresh("History")], "history.rs"),
        "record" => b(BKind::Helper, &[Pass(0)], "history.rs"),
        "merge" => b(BKind::Helper, &[Fresh("History")], "history.rs"),
        "send_to" => b(BKind::RoutingToSink, &[Pass(0)], "boundary.rs"),
        "send_list" => b(BKind::RoutingToSink, &[Pass(0)], "boundary.rs"),
        "take_one" => b(BKind::TakeFromSupplier, &[Fresh("item"), Pass(0)], "boundary.rs"),
        "take_n" => b(BKind::TakeFromSupplier, &[Fresh("items"), Pass(0)], "boundary.rs"),
        "new_organisation" => b(BKind::Boundary, &[Fresh("Organisation")], "common.rs"),
        "new_location" => b(BKind::Boundary, &[Fresh("Location")], "common.rs"),
        _ => return None,
    })
}

/// Units for model-core resources (crate resources carry their own).
pub fn builtin_unit(base: &str) -> Option<&'static str> {
    match base {
        "Person" | "Qualified" | "Labour" => Some("ms"),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Orchestration helpers shared by the two binaries.
// ---------------------------------------------------------------------------

/// The model crates both generators cover, in ladder order.
pub fn default_crates() -> Vec<String> {
    [
        "cs1-pot-of-tea",
        "cs2-puncture-repair",
        "cs3-cafe-orders",
        "cs4-stores",
        "cs4-line",
        "cs5-supply",
        "cs5-logistics",
        "cs5-works",
        "cs6-bread-batch",
        "pilot-workshop",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

/// Repo-relative forward-slash path of a file under the root.
pub fn rel_path(root: &Path, file: &Path) -> String {
    file.strip_prefix(root)
        .unwrap_or(file)
        .to_string_lossy()
        .replace('\\', "/")
}
