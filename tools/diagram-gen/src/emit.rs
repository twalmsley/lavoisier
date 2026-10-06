//! Projections and output: the static context and top-level builders, the
//! Mermaid/markdown writers, and the docs/diagrams/README.md index.

use crate::flow::{FlowGraph, NKind, Tracer};
use crate::{
    CrateModel, FnDef, FnKind, Loc, ResKind, as_number, base_name, builtin_unit, generic_args,
    list_shape, split_top,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::Path;

// ---------------------------------------------------------------------------
// Labels.
// ---------------------------------------------------------------------------

/// Human label for a type string: `ColdWater<1500>` -> "ColdWater 1500 grams",
/// `Cons<SpentTeabag, ..>` -> "3 x SpentTeabag", generic noise dropped.
pub fn render_label(cm: &CrateModel, ty: &str) -> String {
    let t = ty.trim();
    if let Some((n, item)) = list_shape(cm, t) {
        return format!("{n} × {item}");
    }
    let b = base_name(t);
    if b.is_empty() {
        return sanitize_label(t);
    }
    let nums: Vec<String> = generic_args(t).iter().filter_map(|a| as_number(a)).collect();
    let mut label = b.clone();
    if !nums.is_empty() {
        label.push(' ');
        label.push_str(&nums.join(", "));
        let unit = cm
            .resources
            .get(&b)
            .and_then(|r| r.unit.as_ref())
            .and_then(|u| u.split_whitespace().next().map(|s| s.to_string()))
            .or_else(|| builtin_unit(&b).map(|s| s.to_string()));
        if let Some(u) = unit {
            label.push(' ');
            label.push_str(&u);
        }
    }
    label
}

/// Keep Mermaid-safe characters only (labels are emitted inside quotes).
fn sanitize_label(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_alphanumeric()
            || matches!(c, ' ' | '_' | '.' | ',' | ':' | '+' | '/' | '-' | '(' | ')' | '×' | '—' | '=' | '*' | '?' | '!' | '\'')
        {
            out.push(c);
        } else {
            out.push(' ');
        }
    }
    // Collapse runs of spaces.
    let mut res = String::new();
    let mut prev_space = false;
    for c in out.trim().chars() {
        if c == ' ' {
            if !prev_space {
                res.push(c);
            }
            prev_space = true;
        } else {
            res.push(c);
            prev_space = false;
        }
    }
    res
}

// ---------------------------------------------------------------------------
// Static resolution shared by the context and top-level builders.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum PIn {
    /// A plain resource input (base type).
    Res(String),
    /// A sink fed through a `Consumer`/`ConsumeList` bound (or a REQ bound
    /// naming a sink type): process -> sink, labeled with the item.
    Sink { sink: String, label: String },
    /// Items taken from one supplier through a `SupplyN` bound.
    Supplier { src: Option<String>, label: String },
    /// A concrete boundary object threaded through (no direction claimed).
    Obj(String),
    Unknown(String),
}

fn is_threadable(cm: &CrateModel, base: &str) -> bool {
    if matches!(base, "Person" | "Qualified") {
        return true;
    }
    cm.resources
        .get(base)
        .map(|r| matches!(r.kind, ResKind::Reusable | ResKind::BoundaryObject))
        .unwrap_or(false)
}

/// Resolve a generic parameter's bound string to a base type via its
/// requirement trait and the `satisfies!` map.
fn resolve_req_base(cm: &CrateModel, bounds: &str) -> Option<String> {
    let id = cm.req_in_bounds(bounds)?;
    let tys = cm.satisfies_types.get(&id)?;
    let first = tys.first()?;
    Some(cm.resolve_alias(first))
}

/// Resolve one declared parameter type of a process into a `PIn`.
fn resolve_param(cm: &CrateModel, fdef: &FnDef, pty: &str) -> PIn {
    let t = pty.trim();
    // A bare generic parameter name?
    if let Some(g) = fdef.generics.iter().find(|g| !g.is_const && g.name == t) {
        let bounds = &g.bounds;
        for piece in split_top(bounds, '+') {
            let piece = piece.trim();
            let head = base_name(piece);
            match head.as_str() {
                "SupplyN" => {
                    let args = generic_args(piece);
                    let taken = args
                        .iter()
                        .find_map(|a| a.trim().strip_prefix("Taken").map(|r| {
                            r.trim_start_matches(|c: char| c == '=' || c.is_whitespace()).to_string()
                        }));
                    let count = args.first().and_then(|a| as_number(a));
                    let label = match taken.as_deref().and_then(|x| list_shape(cm, x)) {
                        Some((n, item)) => {
                            let item = resolve_item_base(cm, fdef, &item);
                            format!("{n} × {item}")
                        }
                        None => match (count, taken) {
                            (Some(n), Some(x)) => format!("{n} × {}", base_name(&x)),
                            _ => "items".to_string(),
                        },
                    };
                    let item = label.split('×').nth(1).map(|s| s.trim().to_string());
                    let src = item.as_deref().and_then(|it| find_supplier_for(cm, it));
                    return PIn::Supplier { src, label };
                }
                "ConsumeList" | "Consumer" => {
                    let arg = generic_args(piece).first().cloned().unwrap_or_default();
                    let label = render_label(cm, &arg);
                    let item_base = match list_shape(cm, &arg) {
                        Some((_, it)) => it,
                        None => base_name(&arg),
                    };
                    let sink = resolve_req_base(cm, bounds)
                        .filter(|b| cm.is_sink_object(b))
                        .or_else(|| {
                            cm.consumers
                                .iter()
                                .find(|c| base_name(&c.item) == item_base)
                                .map(|c| c.sink.clone())
                        });
                    if let Some(sink) = sink {
                        return PIn::Sink { sink, label };
                    }
                    return PIn::Unknown(format!("consumer of {label}"));
                }
                _ => {}
            }
        }
        if let Some(b) = resolve_req_base(cm, bounds) {
            if cm.is_sink_object(&b) {
                let items = cm.sink_items(&b).join(" / ");
                return PIn::Sink { sink: b, label: items };
            }
            return PIn::Res(b);
        }
        return PIn::Unknown(format!("{t}: {bounds}"));
    }
    // Concrete type.
    let b = cm.resolve_alias(&base_name(t));
    if b.is_empty() {
        return PIn::Unknown(t.to_string());
    }
    if cm.resources.get(&b).map(|r| r.kind == ResKind::BoundaryObject).unwrap_or(false) {
        PIn::Obj(b)
    } else {
        PIn::Res(b)
    }
}

/// Resolve a `SupplyN` item that is itself a generic name (`FourOf<B>`).
fn resolve_item_base(cm: &CrateModel, fdef: &FnDef, item: &str) -> String {
    if let Some(g) = fdef.generics.iter().find(|g| !g.is_const && g.name == item) {
        if let Some(b) = resolve_req_base(cm, &g.bounds) {
            return b;
        }
    }
    cm.resolve_alias(item)
}

fn find_supplier_for(cm: &CrateModel, item: &str) -> Option<String> {
    let mut srcs: Vec<String> = cm.suppliers.iter().map(|s| s.src.clone()).collect();
    srcs.sort();
    srcs.dedup();
    srcs.into_iter().find(|s| {
        cm.supplier_item(s).map(|it| cm.resolve_alias(&it) == item || it == item).unwrap_or(false)
    })
}

/// Static outputs of a process: (base type, arm note) pairs; continuations
/// (`::Rest` / `::Next`) are skipped, `::Item` resolves through the supplier,
/// `Result<Ok, Err>` bundles are flattened to their fields.
fn resolve_outputs(cm: &CrateModel, fdef: &FnDef) -> (Vec<(String, Option<&'static str>)>, Vec<String>) {
    let mut outs: Vec<(String, Option<&'static str>)> = Vec::new();
    let mut supplier_outs: Vec<String> = Vec::new();
    let resolve_generic = |g: &str| -> Option<String> {
        fdef.generics
            .iter()
            .find(|gp| !gp.is_const && gp.name == g)
            .and_then(|gp| resolve_req_base(cm, &gp.bounds))
    };
    let push_plain = |outs: &mut Vec<(String, Option<&'static str>)>, c: &str, arm| {
        let b = cm.resolve_alias(&base_name(c));
        if !b.is_empty() {
            outs.push((b, arm));
        }
    };
    for comp in &fdef.ret {
        let c = comp.trim();
        let leading: String = c
            .trim_start_matches('<')
            .chars()
            .take_while(|ch| ch.is_alphanumeric() || *ch == '_')
            .collect();
        let is_assoc = c.starts_with('<')
            || c.strip_prefix(leading.as_str()).map(|r| r.starts_with("::")).unwrap_or(false);
        if is_assoc {
            if c.ends_with("::Rest") || c.ends_with("::Next") {
                continue; // supplier/sink continuation
            }
            if c.ends_with("::Item") || c.ends_with("::Taken") {
                // Resolve through the parameter's object: which supplier?
                if let Some(gp) = fdef.generics.iter().find(|gp| gp.name == leading) {
                    let obj = resolve_req_base(cm, &gp.bounds).or_else(|| {
                        // A Consumer<..> bound names the sink object.
                        split_top(&gp.bounds, '+').iter().find_map(|p| {
                            if base_name(p) == "Consumer" {
                                let item =
                                    generic_args(p).first().map(|a| base_name(a)).unwrap_or_default();
                                cm.consumers
                                    .iter()
                                    .find(|ci| base_name(&ci.item) == item)
                                    .map(|ci| ci.sink.clone())
                            } else {
                                None
                            }
                        })
                    });
                    if let Some(obj) = obj {
                        if let Some(item) = cm.supplier_item(&obj) {
                            let item = cm.resolve_alias(&item);
                            if cm.is_supplier_object(&item) {
                                supplier_outs.push(item);
                            } else {
                                outs.push((item, None));
                            }
                            continue;
                        }
                    }
                }
            }
            continue;
        }
        // Bare generic name: resolve via its requirement bound.
        if fdef.generics.iter().any(|g| !g.is_const && g.name == c) {
            if let Some(b) = resolve_generic(c) {
                outs.push((b, None));
            }
            continue;
        }
        let b = base_name(c);
        if b == "Result" {
            let arms = generic_args(c);
            for (k, armty) in arms.iter().enumerate() {
                let arm = if k == 0 { Some("on success") } else { Some("on failure") };
                let ab = base_name(armty);
                if let Some(sinfo) = cm.structs.get(&ab) {
                    for (_, fty) in &sinfo.fields {
                        let f = fty.trim();
                        if fdef.generics.iter().any(|g| !g.is_const && g.name == f) {
                            if let Some(rb) = resolve_generic(f) {
                                outs.push((rb, arm));
                            }
                        } else {
                            push_plain(&mut outs, f, arm);
                        }
                    }
                } else {
                    push_plain(&mut outs, armty, arm);
                }
            }
            continue;
        }
        // A supplier object produced directly.
        let rb = cm.resolve_alias(&b);
        if cm.is_supplier_object(&rb) {
            supplier_outs.push(rb);
            continue;
        }
        push_plain(&mut outs, c, None);
    }
    // Collapse duplicate (type) entries that differ only by arm.
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    let mut collapsed: Vec<(String, Option<&'static str>)> = Vec::new();
    for (b, arm) in outs {
        if let Some(&k) = seen.get(&b) {
            if collapsed[k].1 != arm {
                collapsed[k].1 = None;
            }
        } else {
            seen.insert(b.clone(), collapsed.len());
            collapsed.push((b, arm));
        }
    }
    (collapsed, supplier_outs)
}

// ---------------------------------------------------------------------------
// Top-level projection (static: the R9 connection graph of all first-level
// processes, from signatures alone).
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum NodeRef {
    Proc(String),
    BFn(String),
}

pub fn build_top_level(cm: &mut CrateModel, core_locs: &BTreeMap<String, Loc>) -> FlowGraph {
    let mut g = FlowGraph::default();
    let procs: Vec<FnDef> =
        cm.fns.values().filter(|f| f.kind == FnKind::Process).cloned().collect();
    let bfns: Vec<FnDef> =
        cm.fns.values().filter(|f| f.kind == FnKind::Boundary).cloned().collect();

    let mut prod_list: Vec<(String, NodeRef, Option<&'static str>)> = Vec::new();
    let mut cons_list: Vec<(String, NodeRef)> = Vec::new();
    let mut sink_edges: Vec<(String, String, String)> = Vec::new(); // (proc, sink, label)
    let mut supplier_edges: Vec<(String, String, String)> = Vec::new(); // (src obj, proc, label)
    let mut supplier_prod: Vec<(NodeRef, String)> = Vec::new();
    let mut threads: Vec<(String, String)> = Vec::new(); // (proc, base)

    for f in &procs {
        let mut in_bases: Vec<String> = Vec::new();
        let mut objs: Vec<String> = Vec::new();
        for (_, pty) in &f.params {
            match resolve_param(cm, f, pty) {
                PIn::Res(b) => in_bases.push(b),
                PIn::Obj(b) => objs.push(b),
                PIn::Sink { sink, label } => sink_edges.push((f.name.clone(), sink, label)),
                PIn::Supplier { src, label } => {
                    if let Some(src) = src {
                        supplier_edges.push((src, f.name.clone(), label));
                    } else {
                        cm.warn(format!(
                            "{}: no supplier object found for its SupplyN bound ({label})",
                            f.name
                        ));
                    }
                }
                PIn::Unknown(d) => {
                    cm.warn(format!("{}: unresolved parameter `{d}`", f.name));
                }
            }
        }
        let (outs, sup_outs) = resolve_outputs(cm, f);
        for s in sup_outs {
            supplier_prod.push((NodeRef::Proc(f.name.clone()), s));
        }
        let out_bases: BTreeSet<String> = outs.iter().map(|(b, _)| b.clone()).collect();
        // Threading: same base in and out, reusable/boundary-object kind.
        let mut threaded: BTreeSet<String> = BTreeSet::new();
        for b in in_bases.iter().chain(objs.iter()) {
            if out_bases.contains(b) && is_threadable(cm, b) {
                threaded.insert(b.clone());
            }
        }
        for b in &objs {
            // A boundary object passed through even without a same-base return
            // (or not returned at all) is shown as a dashed link, never a flow.
            threaded.insert(b.clone());
        }
        for b in &threaded {
            threads.push((f.name.clone(), b.clone()));
        }
        for b in in_bases {
            if !threaded.contains(&b) {
                cons_list.push((b, NodeRef::Proc(f.name.clone())));
            }
        }
        for (b, arm) in outs {
            if !threaded.contains(&b) {
                prod_list.push((b, NodeRef::Proc(f.name.clone()), arm));
            }
        }
    }

    for f in &bfns {
        let param_bases: BTreeSet<String> =
            f.params.iter().map(|(_, t)| cm.resolve_alias(&base_name(t))).collect();
        for b in &param_bases {
            if !b.is_empty() {
                cons_list.push((b.clone(), NodeRef::BFn(f.name.clone())));
            }
        }
        for comp in &f.ret {
            let b = cm.resolve_alias(&base_name(comp));
            if b.is_empty() || param_bases.contains(&b) {
                continue;
            }
            if cm.is_supplier_object(&b) {
                supplier_prod.push((NodeRef::BFn(f.name.clone()), b));
            } else if cm.is_sink_object(&b) {
                // Sink constructors: the sink node itself covers it.
            } else {
                prod_list.push((b, NodeRef::BFn(f.name.clone()), None));
            }
        }
    }

    // ---- node management ----
    let mut proc_nodes: BTreeMap<String, usize> = BTreeMap::new();
    let mut bfn_nodes: BTreeMap<String, usize> = BTreeMap::new();
    let mut obj_nodes: BTreeMap<String, usize> = BTreeMap::new();
    let mut res_nodes: BTreeMap<String, usize> = BTreeMap::new();
    let mut caller: Option<usize> = None;
    let mut history: Option<usize> = None;

    // Deterministic node order: processes first (by file/line), then the rest
    // as referenced.
    let mut sorted_procs = procs.clone();
    sorted_procs.sort_by(|a, b| (&a.loc.file, a.loc.line).cmp(&(&b.loc.file, b.loc.line)));
    for f in &sorted_procs {
        let idx = g.add_node(&f.name, &f.name, NKind::Process, Some(f.loc.clone()));
        proc_nodes.insert(f.name.clone(), idx);
    }

    macro_rules! bfn_node {
        ($g:expr, $cm:expr, $name:expr) => {{
            let name: &str = $name;
            if let Some(&i) = bfn_nodes.get(name) {
                i
            } else {
                let (kind, loc) = match $cm.fns.get(name) {
                    Some(f) => {
                        let k = if f.ret.is_empty() { NKind::Sink } else { NKind::Boundary };
                        (k, Some(f.loc.clone()))
                    }
                    None => (NKind::Boundary, None),
                };
                let i = $g.add_node(name, name, kind, loc);
                bfn_nodes.insert(name.to_string(), i);
                i
            }
        }};
    }
    macro_rules! obj_node {
        ($g:expr, $cm:expr, $name:expr, $kind:expr) => {{
            let name: &str = $name;
            if let Some(&i) = obj_nodes.get(name) {
                i
            } else {
                let loc = $cm.resources.get(name).map(|r| r.loc.clone());
                let i = $g.add_node(name, name, $kind, loc);
                obj_nodes.insert(name.to_string(), i);
                i
            }
        }};
    }
    macro_rules! node_of {
        ($g:expr, $cm:expr, $r:expr) => {
            match $r {
                NodeRef::Proc(p) => *proc_nodes.get(p.as_str()).expect("proc node"),
                NodeRef::BFn(b) => bfn_node!($g, $cm, b.as_str()),
            }
        };
    }

    // ---- per-type edge assembly ----
    let mut types: BTreeSet<String> = BTreeSet::new();
    for (t, _, _) in &prod_list {
        types.insert(t.clone());
    }
    for (t, _) in &cons_list {
        types.insert(t.clone());
    }
    let threaded_bases: BTreeSet<String> = threads.iter().map(|(_, b)| b.clone()).collect();

    for t in &types {
        let prods: Vec<(NodeRef, Option<&'static str>)> = {
            let mut v: Vec<_> = prod_list
                .iter()
                .filter(|(ty, _, _)| ty == t)
                .map(|(_, r, a)| (r.clone(), *a))
                .collect();
            v.sort();
            v.dedup();
            v
        };
        let conss: Vec<NodeRef> = {
            let mut v: Vec<_> =
                cons_list.iter().filter(|(ty, _)| ty == t).map(|(_, r)| r.clone()).collect();
            v.sort();
            v.dedup();
            v
        };
        let unit_label = t.clone();
        let hub = (prods.len() >= 2 && conss.len() >= 2) || threaded_bases.contains(t);
        if hub {
            let rn = *res_nodes.entry(t.clone()).or_insert_with(|| {
                let loc = cm
                    .resources
                    .get(t)
                    .map(|r| r.loc.clone())
                    .or_else(|| core_locs.get(t).cloned());
                g.add_node(t, t, NKind::Resource, loc)
            });
            for (p, arm) in &prods {
                let from = node_of!(g, cm, p);
                let label = match arm {
                    Some(a) => format!("{unit_label} ({a})"),
                    None => unit_label.clone(),
                };
                g.edge(from, rn, label, false);
            }
            for c in &conss {
                let to = node_of!(g, cm, c);
                g.edge(rn, to, unit_label.clone(), false);
            }
            if conss.is_empty() {
                // fall through to the sink/caller handling below, from the hub
                route_unconsumed(cm, &mut g, rn, t, &mut obj_nodes, &mut caller, &mut history);
            }
            continue;
        }
        if !conss.is_empty() {
            for (p, arm) in &prods {
                for c in &conss {
                    let from = node_of!(g, cm, p);
                    let to = node_of!(g, cm, c);
                    let label = match arm {
                        Some(a) => format!("{unit_label} ({a})"),
                        None => unit_label.clone(),
                    };
                    g.edge(from, to, label, false);
                }
            }
            if prods.is_empty() {
                // Consumed but produced nowhere in a signature: a sink object
                // leaving with its contents comes from its own node; anything
                // else comes from the caller.
                let from = if cm.is_sink_object(t) {
                    obj_node!(g, cm, t.as_str(), NKind::Sink)
                } else {
                    *caller.get_or_insert_with(|| {
                        g.add_node("caller / flow", "caller", NKind::FlowIo, None)
                    })
                };
                for c in &conss {
                    let to = node_of!(g, cm, c);
                    g.edge(from, to, unit_label.clone(), false);
                }
            }
        } else {
            for (p, arm) in &prods {
                let from = node_of!(g, cm, p);
                let n = from;
                let label = match arm {
                    Some(a) => format!("{unit_label} ({a})"),
                    None => unit_label.clone(),
                };
                let routed = route_unconsumed_from(
                    cm,
                    &mut g,
                    n,
                    t,
                    &label,
                    &mut obj_nodes,
                    &mut caller,
                    &mut history,
                );
                if !routed {
                    let cal = *caller.get_or_insert_with(|| {
                        g.add_node("caller / flow", "caller", NKind::FlowIo, None)
                    });
                    g.edge(from, cal, label, false);
                }
            }
        }
    }

    // Sink edges from Consumer/ConsumeList bounds.
    for (proc, sink, label) in &sink_edges {
        let from = *proc_nodes.get(proc.as_str()).expect("proc node");
        let to = obj_node!(g, cm, sink.as_str(), NKind::Sink);
        g.edge(from, to, label.clone(), false);
    }
    // Supplier edges (SupplyN bounds) and supplier producers.
    for (src, proc, label) in &supplier_edges {
        let from = obj_node!(g, cm, src.as_str(), NKind::Boundary);
        let to = *proc_nodes.get(proc.as_str()).expect("proc node");
        g.edge(from, to, label.clone(), false);
    }
    for (r, obj) in &supplier_prod {
        let to = obj_node!(g, cm, obj.as_str(), NKind::Boundary);
        let from = node_of!(g, cm, r);
        g.edge(from, to, obj.clone(), false);
    }
    // Threaded reusables: dashed links. A threaded sink/supplier object
    // reuses its boundary-object node; plain reusables get a resource node.
    for (proc, base) in &threads {
        let rn = if cm.is_sink_object(base) || cm.is_supplier_object(base) {
            let kind = if cm.is_sink_object(base) { NKind::Sink } else { NKind::Boundary };
            obj_node!(g, cm, base.as_str(), kind)
        } else {
            *res_nodes.entry(base.clone()).or_insert_with(|| {
                let loc = cm
                    .resources
                    .get(base)
                    .map(|r| r.loc.clone())
                    .or_else(|| core_locs.get(base).cloned());
                g.add_node(base, base, NKind::Resource, loc)
            })
        };
        let from = *proc_nodes.get(proc.as_str()).expect("proc node");
        g.edge(rn, from, String::new(), true);
    }
    // Person / Qualified providers from model-core.
    for (base, provider) in [("Person", "new_person"), ("Qualified", "qualify")] {
        if let Some(&rn) = res_nodes.get(base) {
            let has_producer = g.edges.iter().any(|e| e.to == rn && !e.dashed);
            if !has_producer {
                let loc = core_locs.get(provider).cloned();
                let i = g.add_node(
                    &format!("{provider} (model-core)"),
                    provider,
                    NKind::Boundary,
                    loc,
                );
                g.edge(i, rn, base.to_string(), false);
            }
        }
    }
    let _ = history;
    g
}

/// Try to route an unconsumed produced type to a known sink (Consumer impls)
/// or to the History bookkeeping (Labour). Returns true when routed.
#[allow(clippy::too_many_arguments)]
fn route_unconsumed_from(
    cm: &CrateModel,
    g: &mut FlowGraph,
    from: usize,
    t: &str,
    label: &str,
    obj_nodes: &mut BTreeMap<String, usize>,
    _caller: &mut Option<usize>,
    history: &mut Option<usize>,
) -> bool {
    if t == "Labour" {
        let h = *history.get_or_insert_with(|| {
            g.add_node("History — execution record (model-core, R16)", "History", NKind::Helper, None)
        });
        g.edge(from, h, label.to_string(), false);
        return true;
    }
    let sinks: Vec<String> = {
        let mut v: Vec<String> = cm
            .consumers
            .iter()
            .filter(|c| base_name(&c.item) == t)
            .map(|c| c.sink.clone())
            .collect();
        v.sort();
        v.dedup();
        v
    };
    if sinks.is_empty() {
        return false;
    }
    for s in sinks {
        let to = if let Some(&i) = obj_nodes.get(&s) {
            i
        } else {
            let loc = cm.resources.get(&s).map(|r| r.loc.clone());
            let i = g.add_node(&s, &s, NKind::Sink, loc);
            obj_nodes.insert(s.clone(), i);
            i
        };
        g.edge(from, to, label.to_string(), false);
    }
    true
}

fn route_unconsumed(
    cm: &CrateModel,
    g: &mut FlowGraph,
    from: usize,
    t: &str,
    obj_nodes: &mut BTreeMap<String, usize>,
    caller: &mut Option<usize>,
    history: &mut Option<usize>,
) {
    if !route_unconsumed_from(cm, g, from, t, t, obj_nodes, caller, history) {
        let cal = *caller
            .get_or_insert_with(|| g.add_node("caller / flow", "caller", NKind::FlowIo, None));
        g.edge(from, cal, t.to_string(), false);
    }
}

// ---------------------------------------------------------------------------
// Context projection (static boundary crossings).
// ---------------------------------------------------------------------------

pub fn build_context(cm: &mut CrateModel, flow_fns_used: &BTreeSet<String>) -> FlowGraph {
    let mut g = FlowGraph::default();
    let sys_label = format!("{} — the modelled system", cm.name);
    let sys = g.add_node(&sys_label, "system", NKind::System, cm.lib_loc.clone());

    let mut ext_nodes: BTreeMap<String, usize> = BTreeMap::new();
    let mut edges_seen: BTreeSet<(usize, usize, String)> = BTreeSet::new();
    macro_rules! ext {
        ($g:expr, $key:expr, $label:expr, $loc:expr) => {{
            let key: String = $key;
            if let Some(&i) = ext_nodes.get(&key) {
                i
            } else {
                let i = $g.add_node(&$label, &key, NKind::External, $loc);
                ext_nodes.insert(key, i);
                i
            }
        }};
    }
    macro_rules! edge_once {
        ($g:expr, $from:expr, $to:expr, $label:expr) => {{
            let l: String = $label;
            if edges_seen.insert(($from, $to, l.clone())) {
                $g.edge($from, $to, l, false);
            }
        }};
    }

    let token_fns: BTreeSet<String> = cm
        .tokens
        .iter()
        .flat_map(|t| {
            [t.success_fn.clone(), t.failure_fn.clone(), t.exit_fn.clone()].into_iter()
        })
        .collect();

    let bfns: Vec<FnDef> =
        cm.fns.values().filter(|f| f.kind == FnKind::Boundary).cloned().collect();
    // Boundary objects some draw-style boundary fn works on (e.g. the mains
    // tap): their constructors are folded into the object's external node.
    let drawn_objects: BTreeSet<String> = bfns
        .iter()
        .filter(|f| !f.ret.is_empty())
        .flat_map(|f| f.params.iter())
        .map(|(_, t)| cm.resolve_alias(&base_name(t)))
        .filter(|b| {
            cm.resources
                .get(b)
                .map(|r| matches!(r.kind, ResKind::Reusable | ResKind::BoundaryObject))
                .unwrap_or(false)
        })
        .collect();
    for f in &bfns {
        if token_fns.contains(&f.name) {
            continue;
        }
        let param_bases: Vec<String> =
            f.params.iter().map(|(_, t)| cm.resolve_alias(&base_name(t))).collect();
        if f.ret.is_empty() {
            if param_bases.is_empty() {
                continue;
            }
            // Exit crossing: resources leave the model through this fn.
            let i = ext!(g, format!("exit_{}", f.name), f.name, Some(f.loc.clone()));
            for b in &param_bases {
                if !b.is_empty() {
                    edge_once!(g, sys, i, b.clone());
                }
            }
            continue;
        }
        // Entry crossings.
        let ret_bases: Vec<String> =
            f.ret.iter().map(|r| cm.resolve_alias(&base_name(r))).collect();
        let crossing: Vec<&String> = ret_bases
            .iter()
            .filter(|b| {
                !b.is_empty()
                    && !param_bases.contains(b)
                    && !cm.is_sink_object(b)
                    && !drawn_objects.contains(*b)
            })
            .collect();
        if crossing.is_empty() {
            continue; // a sink constructor — the sink node covers it
        }
        // The external is the boundary object the fn draws from, else the fn.
        let obj_param = param_bases.iter().find(|b| cm.resources.contains_key(*b));
        let (key, label, loc) = match obj_param {
            Some(obj) => (
                obj.clone(),
                obj.clone(),
                cm.resources.get(obj).map(|r| r.loc.clone()).or(Some(f.loc.clone())),
            ),
            None => (f.name.clone(), f.name.clone(), Some(f.loc.clone())),
        };
        let i = ext!(g, key, label, loc);
        for b in crossing {
            if cm.is_supplier_object(b) {
                let item = cm
                    .supplier_item(b)
                    .map(|it| cm.resolve_alias(&it))
                    .unwrap_or_else(|| "items".to_string());
                edge_once!(g, i, sys, format!("{b} ({item} supplier)"));
            } else {
                edge_once!(g, i, sys, b.clone());
            }
        }
    }

    // Sink objects: what leaves the system, labeled by their Consumer impls.
    let mut sinks: Vec<String> = cm.consumers.iter().map(|c| c.sink.clone()).collect();
    sinks.sort();
    sinks.dedup();
    for s in &sinks {
        let loc = cm.resources.get(s).map(|r| r.loc.clone());
        let i = ext!(g, s.clone(), s.clone(), loc);
        for item in cm.sink_items(s) {
            edge_once!(g, sys, i, item);
        }
        // An object that is both sink and supplier (a vendor): goods enter.
        if cm.is_supplier_object(s) {
            if let Some(item) = cm.supplier_item(s) {
                let item = cm.resolve_alias(&item);
                edge_once!(g, i, sys, format!("{item} (goods)"));
            }
        }
    }

    // Outcome tokens: the R17 environment crossing, in and out.
    for t in &cm.tokens.clone() {
        let i = ext!(
            g,
            format!("env_{}", t.token),
            format!("environment — {} trials", t.token),
            Some(t.loc.clone())
        );
        edge_once!(g, i, sys, t.token.clone());
        edge_once!(g, sys, i, format!("{} (untried)", t.token));
    }

    // Person / time budget, and the History record (model-core crossings).
    let procs_mention_person = cm.fns.values().any(|f| {
        f.kind == FnKind::Process
            && f.params.iter().any(|(_, t)| {
                let b = base_name(t);
                b == "Person" || b == "Qualified" || {
                    f.generics
                        .iter()
                        .find(|gp| gp.name == t.trim())
                        .map(|gp| {
                            resolve_req_base(cm, &gp.bounds)
                                .map(|r| r == "Qualified" || r == "Person")
                                .unwrap_or(false)
                        })
                        .unwrap_or(false)
                }
            })
    });
    if procs_mention_person || flow_fns_used.contains("new_person") {
        let i = ext!(g, "person".to_string(), "person (model-core boundary)".to_string(), None);
        edge_once!(g, i, sys, "Person (time budget)".to_string());
    }
    let uses_time = flow_fns_used.contains("draw_time")
        || flow_fns_used.contains("qualified_draw_time")
        || cm.fns.values().any(|f| {
            f.kind == FnKind::Process && f.ret.iter().any(|r| base_name(r) == "Labour")
        });
    if uses_time {
        let i = ext!(
            g,
            "history".to_string(),
            "History — the execution record (R16)".to_string(),
            None
        );
        edge_once!(g, sys, i, "Labour (expended time)".to_string());
    }
    g
}

// ---------------------------------------------------------------------------
// Mermaid + markdown emission.
// ---------------------------------------------------------------------------

fn kind_name(k: NKind) -> &'static str {
    match k {
        NKind::Process => "process",
        NKind::Boundary => "boundary source",
        NKind::Sink => "boundary sink",
        NKind::Helper => "model-core helper",
        NKind::Grouping => "outcome grouping",
        NKind::FlowIo => "flow input/output",
        NKind::Resource => "shared resource",
        NKind::External => "external party",
        NKind::System => "the system",
    }
}

fn class_name(k: NKind) -> &'static str {
    match k {
        NKind::Process => "process",
        NKind::Boundary => "boundary",
        NKind::Sink => "sink",
        NKind::Helper => "helper",
        NKind::Grouping => "grouping",
        NKind::FlowIo => "flowio",
        NKind::Resource => "resource",
        NKind::External => "external",
        NKind::System => "system",
    }
}

fn mermaid(g: &FlowGraph, direction: &str) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "```mermaid");
    let _ = writeln!(s, "flowchart {direction}");
    let _ = writeln!(s, "  classDef process fill:#dbeafe,stroke:#1e40af,color:#1e3a8a;");
    let _ = writeln!(s, "  classDef boundary fill:#dcfce7,stroke:#166534,color:#14532d;");
    let _ = writeln!(s, "  classDef sink fill:#fee2e2,stroke:#991b1b,color:#7f1d1d;");
    let _ = writeln!(s, "  classDef helper fill:#f3e8ff,stroke:#6b21a8,color:#581c87;");
    let _ = writeln!(s, "  classDef grouping fill:#ffedd5,stroke:#9a3412,color:#7c2d12;");
    let _ = writeln!(s, "  classDef flowio fill:#e2e8f0,stroke:#334155,color:#0f172a;");
    let _ = writeln!(s, "  classDef resource fill:#fef9c3,stroke:#854d0e,color:#713f12;");
    let _ = writeln!(s, "  classDef external fill:#dcfce7,stroke:#166534,color:#14532d;");
    let _ = writeln!(s, "  classDef system fill:#dbeafe,stroke:#1e40af,color:#1e3a8a;");

    // Group nodes into subgraphs by kind (only the kinds present).
    let groups: [(NKind, &str, &str); 4] = [
        (NKind::Boundary, "sg_sources", "System boundary — sources and setup"),
        (NKind::Process, "sg_system", "Processes"),
        (NKind::Helper, "sg_helpers", "Time accounting and helpers (model-core)"),
        (NKind::Sink, "sg_sinks", "System boundary — sinks"),
    ];
    let grouped: BTreeSet<usize> = g
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| groups.iter().any(|(k, _, _)| *k == n.kind) || n.kind == NKind::Grouping)
        .map(|(i, _)| i)
        .collect();
    for (kind, sgid, title) in groups {
        let members: Vec<&crate::flow::FNode> = g
            .nodes
            .iter()
            .filter(|n| n.kind == kind || (kind == NKind::Process && n.kind == NKind::Grouping))
            .collect();
        if members.is_empty() {
            continue;
        }
        let _ = writeln!(s, "  subgraph {sgid}[\"{}\"]", sanitize_label(title));
        let _ = writeln!(s, "    direction TB");
        for n in members {
            let _ = writeln!(
                s,
                "    {}[\"{}\"]:::{}",
                n.id,
                sanitize_label(&n.label),
                class_name(n.kind)
            );
        }
        let _ = writeln!(s, "  end");
    }
    for (i, n) in g.nodes.iter().enumerate() {
        if grouped.contains(&i) {
            continue;
        }
        let _ =
            writeln!(s, "  {}[\"{}\"]:::{}", n.id, sanitize_label(&n.label), class_name(n.kind));
    }

    // Edges (deduplicated, in insertion order).
    let mut seen: BTreeSet<(usize, usize, String, bool)> = BTreeSet::new();
    for e in &g.edges {
        if e.from >= g.nodes.len() || e.to >= g.nodes.len() {
            continue;
        }
        let key = (e.from, e.to, e.label.clone(), e.dashed);
        if !seen.insert(key) {
            continue;
        }
        let from = &g.nodes[e.from].id;
        let to = &g.nodes[e.to].id;
        let label = sanitize_label(&e.label);
        let line = match (e.dashed, label.is_empty()) {
            (true, _) => format!("  {from} -.- {to}"),
            (false, true) => format!("  {from} --> {to}"),
            (false, false) => format!("  {from} -- \"{label}\" --> {to}"),
        };
        let _ = writeln!(s, "{line}");
    }

    // Click directives (sandboxed on GitHub; functional in local renderers —
    // the legend table below is the contract).
    for n in &g.nodes {
        if let Some(loc) = &n.loc {
            let _ = writeln!(s, "  click {} \"../../../{}#L{}\" \"{}\"", n.id, loc.file, loc.line, loc.file);
        }
    }
    let _ = writeln!(s, "```");
    s
}

fn legend(g: &FlowGraph) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "| Node | Kind | Defined at |");
    let _ = writeln!(s, "|---|---|---|");
    for n in &g.nodes {
        let link = match &n.loc {
            Some(loc) => format!("[{}#L{}](../../../{}#L{})", loc.file, loc.line, loc.file, loc.line),
            None => "—".to_string(),
        };
        let _ = writeln!(s, "| `{}` ({}) | {} | {} |", n.id, sanitize_label(&n.label), kind_name(n.kind), link);
    }
    s
}

/// Resources named on the edges, with links to their sealed definitions.
fn resource_table(cm: &CrateModel, graphs: &[&FlowGraph]) -> String {
    let mut bases: BTreeSet<String> = BTreeSet::new();
    for g in graphs {
        for e in &g.edges {
            // First token of the label is the base type (or a count for lists).
            for tok in e.label.split([' ', '(', '/']) {
                let tok = tok.trim().trim_end_matches(',');
                if cm.resources.contains_key(tok) {
                    bases.insert(tok.to_string());
                }
            }
        }
    }
    if bases.is_empty() {
        return String::new();
    }
    let mut s = String::new();
    let _ = writeln!(s, "\n## Resources on the edges\n");
    let _ = writeln!(s, "| Resource | Kind | Unit | Defined at |");
    let _ = writeln!(s, "|---|---|---|---|");
    for b in bases {
        let r = &cm.resources[&b];
        let kind = match r.kind {
            ResKind::Container => "continuous (container)",
            ResKind::Consumable => "discrete (consumable)",
            ResKind::Reusable => "reusable",
            ResKind::OutcomeToken => "outcome token (R17)",
            ResKind::BoundaryObject => "boundary object",
        };
        let unit = r.unit.clone().unwrap_or_else(|| "—".to_string());
        let ph = if r.placeholder { " (placeholder)" } else { "" };
        let _ = writeln!(
            s,
            "| `{}` | {}{} | {} | [{}#L{}](../../../{}#L{}) |",
            b, kind, ph, unit, r.loc.file, r.loc.line, r.loc.file, r.loc.line
        );
    }
    s
}

fn warnings_block(warns: &[String]) -> String {
    if warns.is_empty() {
        return String::new();
    }
    let mut s = String::from("\n");
    for w in warns {
        let _ = writeln!(s, "<!-- WARN (diagram-gen): {} -->", w.replace("--", "–"));
    }
    s
}

fn stamp(cm: &CrateModel) -> String {
    format!(
        "> Generated by `tools/diagram-gen` from `model/{}` — **do not hand-edit**; regenerate \
         with `tools/diagrams.sh`. GitHub sandboxes Mermaid `click` links, so the legend table \
         under the diagram carries the hyperlinks into the source.\n",
        cm.name
    )
}

// ---------------------------------------------------------------------------
// Per-crate orchestration.
// ---------------------------------------------------------------------------

pub fn emit_crate(
    _root: &Path,
    out_dir: &Path,
    cm: &mut CrateModel,
    core_locs: &BTreeMap<String, Loc>,
) {
    // ---- detailed: trace the canonical flow (+ composite flows) ----
    let main_flow: Option<String> = cm
        .fns
        .keys()
        .find(|n| n.starts_with("flow_order_a"))
        .cloned();
    let composite_flows: Vec<String> = cm
        .fns
        .values()
        .filter(|f| f.kind == FnKind::Flow && f.body.is_some())
        .map(|f| f.name.clone())
        .collect();

    let mut traced: Vec<(String, FlowGraph, Loc)> = Vec::new();
    let mut flow_fns_used: BTreeSet<String> = BTreeSet::new();
    if let Some(mf) = &main_flow {
        let loc = cm.fns[mf].loc.clone();
        if let Some(g) = Tracer::trace(cm, core_locs, mf) {
            for n in &g.nodes {
                flow_fns_used.insert(n.fname.clone());
            }
            traced.push((format!("Main flow — `{mf}`"), g, loc));
        }
    } else {
        cm.warn("no `flow_order_a*` integration-test flow found".to_string());
    }
    for cf in &composite_flows {
        let loc = cm.fns[cf].loc.clone();
        if let Some(g) = Tracer::trace(cm, core_locs, cf) {
            traced.push((format!("Composite fallible flow — `{cf}`"), g, loc));
        }
    }

    // ---- static projections ----
    let before = cm.warnings.len();
    let top = build_top_level(cm, core_locs);
    let top_warns: Vec<String> = cm.warnings[before..].to_vec();
    let before = cm.warnings.len();
    let ctx = build_context(cm, &flow_fns_used);
    let ctx_warns: Vec<String> = cm.warnings[before..].to_vec();

    // ---- context.md ----
    let mut s = String::new();
    let _ = writeln!(s, "# {} — context diagram\n", cm.name);
    s.push_str(&stamp(cm));
    let _ = writeln!(
        s,
        "\nThe system as one box — only what crosses the boundary (R12/R15): inputs from \
         suppliers and sources on the left-hand edges, outputs to consumers and sinks on the \
         right. Extracted statically from the crate's `boundary` module functions, its \
         `Supplier`/`Consumer` impls and its `outcome_token!` declarations. *{}*\n",
        cm.title
    );
    s.push_str(&mermaid(&ctx, "LR"));
    let _ = writeln!(s, "\n## Legend — node → source\n");
    s.push_str(&legend(&ctx));
    s.push_str(&resource_table(cm, &[&ctx]));
    s.push_str(&warnings_block(&ctx_warns));
    write_out(out_dir, "context.md", &s);
    println!(
        "{} context: {} nodes, {} edges",
        cm.name,
        ctx.nodes.len(),
        count_edges(&ctx)
    );

    // ---- top-level.md ----
    let mut s = String::new();
    let _ = writeln!(s, "# {} — top-level diagram\n", cm.name);
    s.push_str(&stamp(cm));
    let _ = writeln!(
        s,
        "\nThe first level of processes with their inputs and outputs, linked to each other and \
         to the boundary. This is the **R9 connection graph**, extracted statically from the \
         process signatures (the model describes connections, not sequences): an edge `A — T → B` \
         means A outputs a resource of type T that B's signature accepts. Requirement-trait \
         bounds are resolved through their `satisfies!`-tagged types; `Consumer`/`SupplyN` \
         bounds become sink/supplier edges. Dashed links are reusable resources or boundary \
         objects **threaded** through a process (moved in and returned, R2) — no direction is \
         claimed. Hand-overs that happen inside a process (e.g. a sealed disposal minting its \
         output) are not visible in signatures and appear only in the detailed diagram's flow \
         trace.\n"
    );
    s.push_str(&mermaid(&top, "LR"));
    let _ = writeln!(s, "\n## Legend — node → source\n");
    s.push_str(&legend(&top));
    s.push_str(&resource_table(cm, &[&top]));
    s.push_str(&warnings_block(&top_warns));
    write_out(out_dir, "top-level.md", &s);
    println!(
        "{} top-level: {} nodes, {} edges",
        cm.name,
        top.nodes.len(),
        count_edges(&top)
    );

    // ---- detailed.md ----
    let mut s = String::new();
    let _ = writeln!(s, "# {} — detailed diagram\n", cm.name);
    s.push_str(&stamp(cm));
    let _ = writeln!(
        s,
        "\nAll levels, fully connected: every call in the flow — boundary setup, the processes, \
         the adjacent time draws and their `History` records (F-048/R16), internal waste \
         routing, and the disposal steps — traced from the flow function's `let`-binding \
         sequence, with the const magnitudes from each call site. Edge labels carry the resource \
         and its magnitude in the base unit (R7).\n"
    );
    let mut graph_refs: Vec<&FlowGraph> = Vec::new();
    for (title, g, loc) in &traced {
        let _ = writeln!(s, "\n## {title}\n");
        let _ = writeln!(
            s,
            "Source: [{}#L{}](../../../{}#L{})\n",
            loc.file, loc.line, loc.file, loc.line
        );
        s.push_str(&mermaid(g, "LR"));
        let _ = writeln!(s, "\n### Legend — node → source\n");
        s.push_str(&legend(g));
        s.push_str(&warnings_block(&g.warnings));
        println!(
            "{} detailed [{}]: {} nodes, {} edges",
            cm.name,
            title,
            g.nodes.len(),
            count_edges(g)
        );
    }
    for (_, g, _) in &traced {
        graph_refs.push(g);
    }
    s.push_str(&resource_table(cm, &graph_refs));
    write_out(out_dir, "detailed.md", &s);
}

fn count_edges(g: &FlowGraph) -> usize {
    let mut seen: BTreeSet<(usize, usize, String, bool)> = BTreeSet::new();
    for e in &g.edges {
        seen.insert((e.from, e.to, e.label.clone(), e.dashed));
    }
    seen.len()
}

fn write_out(dir: &Path, name: &str, content: &str) {
    let path = dir.join(name);
    std::fs::write(&path, content).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
}

pub fn emit_index(root: &Path, rows: &[(String, String)]) {
    let mut s = String::new();
    let _ = writeln!(s, "# Generated model diagrams\n");
    let _ = writeln!(
        s,
        "> Generated by `tools/diagram-gen` — **do not hand-edit**; regenerate with \
         `tools/diagrams.sh`.\n"
    );
    let _ = writeln!(
        s,
        "Three GitHub-viewable Mermaid diagrams per model crate, generated from the crate's \
         source (the R10 grep discipline is what makes the extraction reliable):\n"
    );
    let _ = writeln!(
        s,
        "- **Context** — the system as one box; only what crosses the boundary (R12/R15).\n\
         - **Top-level** — the first level of processes with their inputs and outputs, linked \
         to each other and to the boundary: the R9 connection graph from the process signatures.\n\
         - **Detailed** — all levels, fully connected: the flow trace with adjacent time draws, \
         History records, internal waste routing and disposal steps, plus the composite \
         fallible flows where the crate has them.\n"
    );
    let _ = writeln!(
        s,
        "GitHub sandboxes Mermaid `click` directives, so **the legend table under each diagram \
         carries the hyperlinks** from every node into the defining source line; the `click` \
         directives still work in local Mermaid renderers.\n"
    );
    let _ = writeln!(s, "| Model crate | Context | Top-level | Detailed |");
    let _ = writeln!(s, "|---|---|---|---|");
    for (name, title) in rows {
        // The lib.rs title usually starts with the crate name — drop it.
        let short = title
            .strip_prefix(name.as_str())
            .map(|r| r.trim_start_matches([' ', '—', '-']).to_string())
            .unwrap_or_else(|| title.clone());
        let _ = writeln!(
            s,
            "| `{name}` — {} | [context]({name}/context.md) | [top-level]({name}/top-level.md) | [detailed]({name}/detailed.md) |",
            short.replace('|', "/")
        );
    }
    let _ = writeln!(
        s,
        "\nRegenerate after a model change:\n\n```sh\n./tools/diagrams.sh\n```\n\nThe generator \
         is `tools/diagram-gen` (std-only Rust, outside the model workspace). Where its \
         line-based heuristics cannot see something it emits a WARN to stderr and an HTML \
         comment in the affected file rather than guessing."
    );
    let dir = root.join("docs/diagrams");
    std::fs::create_dir_all(&dir).expect("create docs/diagrams");
    write_out(&dir, "README.md", &s);
}
