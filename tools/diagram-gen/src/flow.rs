//! Flow tracer: turns the body of a flow function (an integration-test flow
//! or a composite flow in `src/flows.rs`) into a dataflow graph, by following
//! `let` bindings through the call sequence (R9: the calls' type connections
//! ARE the model graph; the test merely fixes one valid order).
//!
//! Conservative by design: anything the line discipline cannot support is
//! skipped with a WARN rather than guessed — e.g. a `match` over a method
//! call (an assertion match) is skipped wholesale.

use crate::{
    BKind, BOut, CrateModel, FnKind, Loc, as_number, base_name, builtin, generic_args,
    split_top, subst_idents,
};
use std::collections::BTreeMap;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NKind {
    Process,
    Boundary,
    Sink,
    Helper,
    Grouping,
    FlowIo,
    /// A shared resource node (top-level projection).
    Resource,
    /// An external party (context projection).
    External,
    /// The system-as-one-box (context projection).
    System,
}

#[derive(Clone, Debug)]
pub struct FNode {
    pub id: String,
    pub label: String,
    pub fname: String,
    pub kind: NKind,
    pub loc: Option<Loc>,
}

#[derive(Clone, Debug)]
pub struct FEdge {
    pub from: usize,
    pub to: usize,
    pub label: String,
    pub dashed: bool,
}

#[derive(Default)]
pub struct FlowGraph {
    pub nodes: Vec<FNode>,
    pub edges: Vec<FEdge>,
    pub warnings: Vec<String>,
    occ: BTreeMap<String, usize>,
}

impl FlowGraph {
    pub fn add_node(&mut self, label: &str, fname: &str, kind: NKind, loc: Option<Loc>) -> usize {
        let occ = self.occ.entry(fname.to_string()).or_insert(0);
        *occ += 1;
        let base: String =
            fname.chars().map(|c| if c.is_alphanumeric() { c } else { '_' }).collect();
        let id = if *occ == 1 { format!("n_{base}") } else { format!("n_{base}_{occ}") };
        let label = if *occ == 1 { label.to_string() } else { format!("{label} ({occ})") };
        self.nodes.push(FNode { id, label, fname: fname.to_string(), kind, loc });
        self.nodes.len() - 1
    }

    pub fn edge(&mut self, from: usize, to: usize, label: String, dashed: bool) {
        if from == to {
            return;
        }
        self.edges.push(FEdge { from, to, label, dashed });
    }
}

// ---------------------------------------------------------------------------
// Expression AST (tiny: only what the flow bodies use).
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum Expr {
    Call { name: String, turbo: Vec<String>, args: Vec<Expr> },
    StructLit { path: String, fields: Vec<(String, Expr)> },
    Tuple(Vec<Expr>),
    Path(String),
    Field { base: Box<Expr>, name: String },
    MethodCall,
    Lit(Option<String>),
}

struct Cur<'a> {
    s: &'a [u8],
    i: usize,
}

impl<'a> Cur<'a> {
    fn new(s: &'a str) -> Self {
        Cur { s: s.as_bytes(), i: 0 }
    }
    fn peek(&self) -> Option<u8> {
        self.s.get(self.i).copied()
    }
    fn peek2(&self) -> Option<u8> {
        self.s.get(self.i + 1).copied()
    }
    fn bump(&mut self) -> Option<u8> {
        let c = self.peek();
        if c.is_some() {
            self.i += 1;
        }
        c
    }
    fn skip_ws(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_ascii_whitespace() {
                self.i += 1;
            } else {
                break;
            }
        }
    }
    fn eat(&mut self, c: u8) -> bool {
        self.skip_ws();
        if self.peek() == Some(c) {
            self.i += 1;
            true
        } else {
            false
        }
    }
    fn at_kw(&mut self, kw: &str) -> bool {
        self.skip_ws();
        let end = self.i + kw.len();
        if end > self.s.len() || &self.s[self.i..end] != kw.as_bytes() {
            return false;
        }
        match self.s.get(end) {
            Some(&c) if c.is_ascii_alphanumeric() || c == b'_' => false,
            _ => true,
        }
    }
    fn kw(&mut self, kw: &str) -> bool {
        if self.at_kw(kw) {
            self.i += kw.len();
            true
        } else {
            false
        }
    }
    fn ident(&mut self) -> String {
        self.skip_ws();
        let start = self.i;
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || c == b'_' {
                self.i += 1;
            } else {
                break;
            }
        }
        String::from_utf8_lossy(&self.s[start..self.i]).to_string()
    }
    fn done(&mut self) -> bool {
        self.skip_ws();
        self.i >= self.s.len()
    }
    /// Consume a balanced `<...>` (cursor at `<`); returns the inner text.
    fn balanced_angles(&mut self) -> String {
        let mut depth = 0i64;
        let start = self.i + 1;
        let mut end = start;
        while let Some(c) = self.bump() {
            match c {
                b'<' => depth += 1,
                b'>' => {
                    depth -= 1;
                    if depth == 0 {
                        end = self.i - 1;
                        break;
                    }
                }
                _ => {}
            }
        }
        String::from_utf8_lossy(&self.s[start..end]).to_string()
    }
    /// Consume a balanced delimiter pair starting at the current char.
    fn skip_balanced(&mut self) {
        let open = match self.peek() {
            Some(c) => c,
            None => return,
        };
        let close = match open {
            b'(' => b')',
            b'[' => b']',
            b'{' => b'}',
            _ => return,
        };
        let mut depth = 0i64;
        let mut in_str = false;
        let mut prev = 0u8;
        while let Some(c) = self.bump() {
            if in_str {
                if c == b'"' && prev != b'\\' {
                    in_str = false;
                }
                prev = if c == b'\\' && prev == b'\\' { 0 } else { c };
                continue;
            }
            if c == b'"' {
                in_str = true;
            } else if c == open {
                depth += 1;
            } else if c == close {
                depth -= 1;
                if depth == 0 {
                    return;
                }
            }
            prev = c;
        }
    }
    /// Text up to (not including) `=>` at bracket depth 0.
    fn until_fat_arrow(&mut self) -> String {
        let start = self.i;
        let mut depth = 0i64;
        while let Some(c) = self.peek() {
            match c {
                b'(' | b'[' | b'{' | b'<' => depth += 1,
                b')' | b']' | b'}' | b'>' => depth -= 1,
                b'=' if depth == 0 && self.peek2() == Some(b'>') => {
                    let text = String::from_utf8_lossy(&self.s[start..self.i]).to_string();
                    self.i += 2;
                    return text;
                }
                _ => {}
            }
            self.i += 1;
        }
        String::from_utf8_lossy(&self.s[start..self.i]).to_string()
    }
    /// Type-annotation text up to `=` at bracket depth 0 (skipping `==`, `=>`).
    fn until_eq(&mut self) -> String {
        let start = self.i;
        let mut depth = 0i64;
        while let Some(c) = self.peek() {
            match c {
                b'(' | b'[' | b'{' | b'<' => depth += 1,
                b')' | b']' | b'}' | b'>' => depth -= 1,
                b'=' if depth == 0 && self.peek2() != Some(b'=') && self.peek2() != Some(b'>') => {
                    let text = String::from_utf8_lossy(&self.s[start..self.i]).to_string();
                    self.i += 1;
                    return text;
                }
                _ => {}
            }
            self.i += 1;
        }
        String::from_utf8_lossy(&self.s[start..self.i]).to_string()
    }

    fn parse_expr(&mut self) -> Expr {
        let mut e = self.parse_primary();
        loop {
            self.skip_ws();
            if self.peek() == Some(b'.') && self.peek2().map(|c| c.is_ascii_alphabetic() || c == b'_') == Some(true)
            {
                self.i += 1;
                let name = self.ident();
                self.skip_ws();
                if self.peek() == Some(b'(') {
                    self.skip_balanced();
                    e = Expr::MethodCall;
                } else {
                    e = Expr::Field { base: Box::new(e), name };
                }
            } else {
                break;
            }
        }
        e
    }

    fn parse_primary(&mut self) -> Expr {
        self.skip_ws();
        match self.peek() {
            Some(b'&') => {
                self.i += 1;
                self.parse_expr()
            }
            Some(b'(') => {
                self.i += 1;
                let mut elems = Vec::new();
                loop {
                    self.skip_ws();
                    if self.peek() == Some(b')') {
                        self.i += 1;
                        break;
                    }
                    elems.push(self.parse_expr());
                    self.skip_ws();
                    if self.peek() == Some(b',') {
                        self.i += 1;
                    }
                }
                if elems.len() == 1 { elems.pop().unwrap() } else { Expr::Tuple(elems) }
            }
            Some(b'"') => {
                self.i += 1;
                let start = self.i;
                let mut prev = 0u8;
                while let Some(c) = self.bump() {
                    if c == b'"' && prev != b'\\' {
                        break;
                    }
                    prev = c;
                }
                let text = String::from_utf8_lossy(&self.s[start..self.i.saturating_sub(1)]).to_string();
                Expr::Lit(Some(text))
            }
            Some(c) if c.is_ascii_digit() => {
                while let Some(c) = self.peek() {
                    if c.is_ascii_alphanumeric() || c == b'_' {
                        self.i += 1;
                    } else {
                        break;
                    }
                }
                Expr::Lit(None)
            }
            Some(c) if c.is_ascii_alphabetic() || c == b'_' => {
                let mut parts = vec![self.ident()];
                let mut turbo: Vec<String> = Vec::new();
                loop {
                    if self.peek() == Some(b':') && self.peek2() == Some(b':') {
                        self.i += 2;
                        if self.peek() == Some(b'<') {
                            let inner = self.balanced_angles();
                            turbo = split_top(&inner, ',');
                        } else {
                            parts.push(self.ident());
                        }
                    } else {
                        break;
                    }
                }
                // Macro invocation: `assert_eq!(..)`, `panic!(..)`, ...
                if self.peek() == Some(b'!') {
                    self.i += 1;
                    self.skip_ws();
                    self.skip_balanced();
                    return Expr::Lit(None);
                }
                self.skip_ws();
                if self.peek() == Some(b'(') {
                    self.i += 1;
                    let mut args = Vec::new();
                    loop {
                        self.skip_ws();
                        if self.peek() == Some(b')') {
                            self.i += 1;
                            break;
                        }
                        args.push(self.parse_expr());
                        self.skip_ws();
                        if self.peek() == Some(b',') {
                            self.i += 1;
                        }
                    }
                    Expr::Call { name: parts.last().cloned().unwrap_or_default(), turbo, args }
                } else if self.peek() == Some(b'{') && parts.len() > 1 {
                    // Struct/variant literal (`RetryOutcome::FirstTry { .. }`).
                    self.i += 1;
                    let mut fields = Vec::new();
                    loop {
                        self.skip_ws();
                        if self.peek() == Some(b'}') {
                            self.i += 1;
                            break;
                        }
                        let fname = self.ident();
                        self.skip_ws();
                        let fexpr = if self.peek() == Some(b':') && self.peek2() != Some(b':') {
                            self.i += 1;
                            self.parse_expr()
                        } else {
                            Expr::Path(fname.clone())
                        };
                        fields.push((fname, fexpr));
                        self.skip_ws();
                        if self.peek() == Some(b',') {
                            self.i += 1;
                        }
                    }
                    Expr::StructLit { path: parts.join("::"), fields }
                } else {
                    Expr::Path(parts.join("::"))
                }
            }
            _ => {
                self.i += 1;
                Expr::Lit(None)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tracing.
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct Var {
    node: usize,
    ty: Option<String>,
}

#[derive(Debug)]
enum Val {
    /// Result of a call: one producing node with typed output slots.
    Slots { node: usize, tys: Vec<Option<String>>, fallible: bool },
    /// A single existing value (variable, field access).
    One { node: Option<usize>, ty: Option<String>, name: Option<String> },
    Tup(Vec<Val>),
    None,
}

pub struct Tracer<'a> {
    cm: &'a mut CrateModel,
    core_locs: &'a BTreeMap<String, Loc>,
    pub g: FlowGraph,
    env: BTreeMap<String, Var>,
    flow_end: Option<usize>,
    flow_in: Option<usize>,
    flow_loc: Loc,
}

impl<'a> Tracer<'a> {
    pub fn trace(
        cm: &'a mut CrateModel,
        core_locs: &'a BTreeMap<String, Loc>,
        flow_fn: &str,
    ) -> Option<FlowGraph> {
        let fdef = cm.fns.get(flow_fn)?.clone();
        let body = fdef.body.clone()?;
        let mut tr = Tracer {
            cm,
            core_locs,
            g: FlowGraph::default(),
            env: BTreeMap::new(),
            flow_end: None,
            flow_in: None,
            flow_loc: fdef.loc.clone(),
        };
        // A composite flow's params enter through a single "flow inputs" node.
        if !fdef.params.is_empty() {
            let idx = tr.add_node(
                "flow inputs (provisioned by the caller)",
                "flow_inputs",
                NKind::FlowIo,
                Some(fdef.loc.clone()),
            );
            tr.flow_in = Some(idx);
            for (pname, pty) in &fdef.params {
                tr.env.insert(pname.clone(), Var { node: idx, ty: Some(pty.clone()) });
            }
        }
        let mut cur = Cur::new(&body);
        tr.parse_stmts(&mut cur, true);
        let g = std::mem::take(&mut tr.g);
        Some(g)
    }

    fn warn(&mut self, msg: String) {
        self.g.warnings.push(msg.clone());
        self.cm.warn(msg);
    }

    fn add_node(&mut self, label: &str, fname: &str, kind: NKind, loc: Option<Loc>) -> usize {
        self.g.add_node(label, fname, kind, loc)
    }

    fn edge(&mut self, from: usize, to: usize, label: String) {
        self.g.edge(from, to, label, false);
    }

    fn flow_end_node(&mut self) -> usize {
        if let Some(i) = self.flow_end {
            return i;
        }
        let loc = self.flow_loc.clone();
        let i = self.add_node("flow end — everything accounted", "flow_end", NKind::FlowIo, Some(loc));
        self.flow_end = Some(i);
        i
    }

    // -- statements ---------------------------------------------------------

    fn parse_stmts(&mut self, cur: &mut Cur, top: bool) {
        loop {
            cur.skip_ws();
            if cur.done() {
                break;
            }
            if cur.peek() == Some(b'}') {
                if !top {
                    cur.i += 1;
                }
                break;
            }
            self.parse_stmt(cur);
        }
    }

    fn parse_stmt(&mut self, cur: &mut Cur) {
        cur.skip_ws();
        if cur.kw("let") {
            let (idents, all_underscore) = parse_pattern(cur);
            cur.skip_ws();
            let ann: Option<Vec<Option<String>>> = if cur.peek() == Some(b':') {
                cur.i += 1;
                let text = cur.until_eq();
                Some(parse_annotation(&text))
            } else {
                cur.skip_ws();
                if cur.peek() == Some(b'=') {
                    cur.i += 1;
                }
                None
            };
            let was_path_like = {
                cur.skip_ws();
                matches!(cur.peek(), Some(c) if c.is_ascii_alphabetic() || c == b'_' || c == b'(')
            };
            let rhs = cur.parse_expr();
            let rhs_is_binding_move =
                was_path_like && matches!(rhs, Expr::Path(_) | Expr::Tuple(_) | Expr::Field { .. });
            cur.eat(b';');
            let val = self.eval(&rhs);
            self.bind(&idents, all_underscore, ann, val, rhs_is_binding_move);
        } else if cur.kw("match") {
            let scrut = cur.parse_expr();
            cur.skip_ws();
            let val = match &scrut {
                Expr::Call { .. } => self.eval(&scrut),
                _ => Val::None,
            };
            if cur.peek() != Some(b'{') {
                return;
            }
            match val {
                Val::Slots { node, tys, fallible } => {
                    cur.i += 1; // '{'
                    self.parse_arms(cur, node, &tys, fallible);
                }
                _ => {
                    self.warn(
                        "skipped a `match` whose scrutinee is not a direct process call \
                         (assertion-only match)"
                            .to_string(),
                    );
                    cur.skip_balanced();
                }
            }
            cur.eat(b';');
        } else {
            let e = cur.parse_expr();
            cur.eat(b';');
            let v = self.eval(&e);
            self.expr_statement_val(v);
        }
    }

    fn expr_statement_val(&mut self, v: Val) {
        if let Val::Tup(vals) = v {
            // A bare tuple at the end of a flow/arm: its outputs.
            let end = self.flow_end_node();
            for val in vals {
                if let Val::One { node: Some(n), ty, name } = val {
                    let label = self.label_for(ty.as_deref(), name.as_deref());
                    self.edge(n, end, label);
                } else if let Val::Slots { node, tys, .. } = val {
                    let label = self.label_for(tys.first().and_then(|t| t.as_deref()), None);
                    self.edge(node, end, label);
                }
            }
        }
    }

    fn parse_arms(&mut self, cur: &mut Cur, node: usize, tys: &[Option<String>], fallible: bool) {
        loop {
            cur.skip_ws();
            if cur.done() {
                break;
            }
            if cur.peek() == Some(b'}') {
                cur.i += 1;
                break;
            }
            let pat = cur.until_fat_arrow();
            self.bind_arm_pattern(pat.trim(), node, tys, fallible);
            cur.skip_ws();
            if cur.peek() == Some(b'{') {
                cur.i += 1;
                self.parse_stmts(cur, false);
            } else {
                let e = cur.parse_expr();
                let v = self.eval(&e);
                self.expr_statement_val(v);
            }
            cur.eat(b',');
        }
    }

    fn bind_arm_pattern(&mut self, pat: &str, node: usize, tys: &[Option<String>], fallible: bool) {
        let pat = pat.trim();
        if pat == "_" || pat.is_empty() {
            return;
        }
        // `Ok(x)` / `Err(x)` (possibly `Ok(_)`).
        for (tag, slot) in [("Ok", 0usize), ("Err", 1usize)] {
            if let Some(rest) = pat.strip_prefix(tag) {
                let rest = rest.trim();
                if rest.starts_with('(') && fallible {
                    let inner = rest.trim_start_matches('(').trim_end_matches(')').trim();
                    if inner != "_" && inner.chars().all(|c| c.is_alphanumeric() || c == '_') {
                        self.env.insert(
                            inner.to_string(),
                            Var { node, ty: tys.get(slot).cloned().flatten() },
                        );
                    }
                    return;
                }
            }
        }
        // `Enum::Variant { a, b, .. }`.
        if let Some(brace) = pat.find('{') {
            let path = pat[..brace].trim();
            let enum_name = path.split("::").next().unwrap_or("").trim().to_string();
            let variant = path.split("::").last().unwrap_or("").trim().to_string();
            let fields_text = pat[brace + 1..].trim_end_matches('}').trim();
            let field_tys: BTreeMap<String, String> = self
                .cm
                .enums
                .get(&enum_name)
                .and_then(|e| e.variants.iter().find(|(v, _)| *v == variant))
                .map(|(_, fs)| fs.iter().cloned().collect())
                .unwrap_or_default();
            for f in split_top(fields_text, ',') {
                let f = f.trim();
                if f == ".." || f.is_empty() {
                    continue;
                }
                let fname = f.split(':').next().unwrap_or(f).trim();
                let bind = f.split(':').nth(1).map(|s| s.trim()).unwrap_or(fname);
                if bind.chars().all(|c| c.is_alphanumeric() || c == '_') && !bind.is_empty() {
                    self.env.insert(
                        bind.to_string(),
                        Var { node, ty: field_tys.get(fname).cloned() },
                    );
                }
            }
            return;
        }
        // A bare binding (`other`): bind to the node, untyped.
        if pat.chars().all(|c| c.is_alphanumeric() || c == '_') {
            self.env.insert(pat.to_string(), Var { node, ty: tys.first().cloned().flatten() });
        }
    }

    fn bind(
        &mut self,
        idents: &[Option<String>],
        all_underscore: bool,
        ann: Option<Vec<Option<String>>>,
        val: Val,
        rhs_is_binding_move: bool,
    ) {
        let ann_slot = |k: usize| -> Option<String> {
            ann.as_ref().and_then(|a| a.get(k).cloned().flatten())
        };
        match val {
            Val::Slots { node, tys, .. } => {
                if idents.len() == 1 {
                    if let Some(Some(name)) = idents.first().map(|x| x.as_ref()) {
                        self.env.insert(
                            name.clone(),
                            Var { node, ty: ann_slot(0).or_else(|| tys.first().cloned().flatten()) },
                        );
                    }
                } else {
                    for (k, ident) in idents.iter().enumerate() {
                        if let Some(name) = ident {
                            self.env.insert(
                                name.clone(),
                                Var { node, ty: ann_slot(k).or_else(|| tys.get(k).cloned().flatten()) },
                            );
                        }
                    }
                    if idents.len() != tys.len() && !tys.is_empty() {
                        self.warn(format!(
                            "pattern arity {} != output arity {} at a traced call",
                            idents.len(),
                            tys.len()
                        ));
                    }
                }
            }
            Val::One { node, ty, name } => {
                if all_underscore && rhs_is_binding_move {
                    if let Some(n) = node {
                        let label =
                            self.label_for(ann_slot(0).or(ty.clone()).as_deref(), name.as_deref());
                        let end = self.flow_end_node();
                        self.edge(n, end, label);
                    }
                } else if let (Some(Some(id)), Some(n)) = (idents.first().map(|x| x.as_ref()), node)
                {
                    self.env.insert(id.clone(), Var { node: n, ty: ann_slot(0).or(ty) });
                }
            }
            Val::Tup(vals) => {
                if all_underscore && rhs_is_binding_move {
                    let end = self.flow_end_node();
                    for v in vals {
                        if let Val::One { node: Some(n), ty, name } = v {
                            let label = self.label_for(ty.as_deref(), name.as_deref());
                            self.edge(n, end, label);
                        }
                    }
                } else {
                    for (k, v) in vals.into_iter().enumerate() {
                        if let (Some(Some(id)), Val::One { node: Some(n), ty, .. }) =
                            (idents.get(k).map(|x| x.as_ref()), v)
                        {
                            self.env.insert(id.clone(), Var { node: n, ty: ann_slot(k).or(ty) });
                        }
                    }
                }
            }
            Val::None => {}
        }
    }

    // -- evaluation ---------------------------------------------------------

    fn label_for(&self, ty: Option<&str>, name: Option<&str>) -> String {
        match ty {
            Some(t) => crate::emit::render_label(self.cm, t),
            None => name.unwrap_or("").to_string(),
        }
    }

    fn eval(&mut self, e: &Expr) -> Val {
        match e {
            Expr::Lit(_) => Val::None,
            Expr::Path(p) => {
                if let Some(v) = self.env.get(p.as_str()).cloned() {
                    Val::One { node: Some(v.node), ty: v.ty, name: Some(p.clone()) }
                } else {
                    Val::None
                }
            }
            Expr::Field { base, name } => {
                let b = self.eval(base);
                if let Val::One { node, ty, .. } = b {
                    let fty = ty
                        .as_deref()
                        .and_then(|t| self.field_type(t, name))
                        // A leftover bare generic (`O`, `G`) says nothing:
                        // fall back to the field name.
                        .filter(|t| {
                            t.contains('<') || t.len() > 2 || self.cm.resources.contains_key(t)
                        });
                    Val::One { node, ty: fty, name: Some(name.clone()) }
                } else {
                    Val::None
                }
            }
            Expr::MethodCall => Val::None,
            Expr::Tuple(elems) => Val::Tup(elems.iter().map(|x| self.eval(x)).collect()),
            Expr::StructLit { path, fields } => {
                let enum_name = path.split("::").next().unwrap_or("").to_string();
                let variant = path.split("::").last().unwrap_or("").to_string();
                let loc = self
                    .cm
                    .enums
                    .get(&enum_name)
                    .map(|e| e.loc.clone())
                    .or_else(|| self.cm.structs.get(&enum_name).map(|s| s.loc.clone()));
                let field_tys: BTreeMap<String, String> = self
                    .cm
                    .enums
                    .get(&enum_name)
                    .and_then(|e| e.variants.iter().find(|(v, _)| *v == variant))
                    .map(|(_, fs)| fs.iter().cloned().collect())
                    .or_else(|| {
                        self.cm.structs.get(&enum_name).map(|s| s.fields.iter().cloned().collect())
                    })
                    .unwrap_or_default();
                let node = self.add_node(path, path, NKind::Grouping, loc);
                let fvals: Vec<(String, Val)> =
                    fields.iter().map(|(fname, fe)| (fname.clone(), self.eval(fe))).collect();
                for (fname, v) in fvals {
                    if let Val::One { node: Some(n), ty, name } = v {
                        let label = self
                            .label_for(field_tys.get(&fname).map(|s| s.as_str()).or(ty.as_deref()), name.as_deref());
                        self.edge(n, node, label);
                    }
                }
                Val::One { node: Some(node), ty: Some(enum_name), name: Some(path.clone()) }
            }
            Expr::Call { name, turbo, args } => self.eval_call(name, turbo, args),
        }
    }

    fn field_type(&self, ty: &str, field: &str) -> Option<String> {
        let b = base_name(ty);
        let sinfo = self.cm.structs.get(&b)?;
        let fty = sinfo.fields.iter().find(|(f, _)| f == field)?.1.clone();
        // Substitute the struct's generics by the concrete arguments.
        let args = generic_args(ty);
        let mut map = BTreeMap::new();
        for (g, a) in sinfo.generics.iter().zip(args.iter()) {
            let a = a.trim();
            if a != "_" {
                map.insert(g.name.clone(), as_number(a).unwrap_or_else(|| a.to_string()));
            }
        }
        Some(subst_idents(&fty, &map))
    }

    fn eval_call(&mut self, name: &str, turbo: &[String], args: &[Expr]) -> Val {
        // Assertion helpers and the like: evaluate nothing.
        if name.starts_with("assert") || name == "panic" || name == "unreachable" {
            return Val::None;
        }
        let argvals: Vec<Val> = args.iter().map(|a| self.eval(a)).collect();

        // Transparent wrappers: `Some(x)` / `Ok(x)` / `Err(x)` are the value.
        if matches!(name, "Some" | "Ok" | "Err") {
            return argvals.into_iter().next().unwrap_or(Val::None);
        }

        // send_to / send_list collapse into an edge to the sink object's node.
        if name == "send_to" || name == "send_list" {
            let sink = match argvals.first() {
                Some(Val::One { node: Some(n), ty, .. }) => Some((*n, ty.clone())),
                Some(Val::Slots { node, tys, .. }) => Some((*node, tys.first().cloned().flatten())),
                _ => None,
            };
            let Some((sink_node, sink_ty)) = sink else {
                self.warn(format!("{name}: could not resolve the sink argument"));
                return Val::None;
            };
            // A sink handed in as a flow parameter gets its own node (the
            // "flow inputs" node is not a consumer).
            let sink_node = if Some(sink_node) == self.flow_in {
                let b = sink_ty.as_deref().map(base_name).unwrap_or_default();
                if b.is_empty() {
                    sink_node
                } else {
                    let loc = self.cm.resources.get(&b).map(|r| r.loc.clone());
                    self.add_node(&b, &b, NKind::Sink, loc)
                }
            } else {
                sink_node
            };
            if let Some(v) = argvals.get(1) {
                match v {
                    Val::One { node: Some(n), ty, name: vname } => {
                        let label = self.label_for(ty.as_deref(), vname.as_deref());
                        self.edge(*n, sink_node, label);
                    }
                    Val::Slots { node, tys, .. } => {
                        let label = self.label_for(tys.first().and_then(|t| t.as_deref()), None);
                        self.edge(*node, sink_node, label);
                    }
                    _ => {}
                }
            }
            return Val::Slots { node: sink_node, tys: vec![sink_ty], fallible: false };
        }

        // Resolve the callee.
        if let Some(fdef) = self.cm.fns.get(name).cloned() {
            if fdef.kind == FnKind::Test {
                return Val::None; // a tests-file helper (assertions)
            }
            let kind = match fdef.kind {
                FnKind::Process | FnKind::Flow => NKind::Process,
                FnKind::Boundary => {
                    let ret_sink = fdef
                        .ret
                        .first()
                        .map(|r| self.cm.is_sink_object(&base_name(r)))
                        .unwrap_or(false);
                    if fdef.ret.is_empty() || ret_sink { NKind::Sink } else { NKind::Boundary }
                }
                _ => NKind::Helper,
            };
            let node = self.add_node(name, name, kind, Some(fdef.loc.clone()));
            self.wire_args(node, &fdef.params, &argvals);
            let (tys, fallible) = self.crate_fn_outputs(&fdef, turbo, &argvals);
            return Val::Slots { node, tys, fallible };
        }
        if let Some(b) = builtin(name) {
            let kind = match b.kind {
                BKind::Boundary => NKind::Boundary,
                _ => NKind::Helper,
            };
            let loc = self.core_locs.get(name).cloned();
            // `record` nodes carry the recorded process name for readability.
            let label = if name == "record" {
                if let Some(Expr::Lit(Some(s))) = args.get(1) {
                    format!("record {s}")
                } else {
                    name.to_string()
                }
            } else {
                name.to_string()
            };
            let node = self.add_node(&label, name, kind, loc);
            self.wire_args(node, &[], &argvals);
            let tys: Vec<Option<String>> = b
                .outs
                .iter()
                .map(|o| match o {
                    BOut::Fresh(tpl) => {
                        let mut t = (*tpl).to_string();
                        for (k, a) in turbo.iter().enumerate() {
                            let v = as_number(a).unwrap_or_else(|| a.trim().to_string());
                            t = t.replace(&format!("{{{k}}}"), &v);
                        }
                        if t.contains('{') {
                            // No turbofish at the call site: keep the base type.
                            let b = base_name(tpl);
                            if b.is_empty() { None } else { Some(b) }
                        } else {
                            Some(t)
                        }
                    }
                    BOut::Pass(k) => match argvals.get(*k) {
                        Some(Val::One { ty, .. }) => ty.clone(),
                        Some(Val::Slots { tys, .. }) => tys.first().cloned().flatten(),
                        _ => None,
                    },
                })
                .collect();
            return Val::Slots { node, tys, fallible: false };
        }
        self.warn(format!("unknown callee `{name}` in a traced flow — shown as a plain step"));
        let node = self.add_node(name, name, NKind::Helper, None);
        self.wire_args(node, &[], &argvals);
        Val::Slots { node, tys: Vec::new(), fallible: false }
    }

    fn wire_args(&mut self, node: usize, params: &[(String, String)], argvals: &[Val]) {
        for (k, v) in argvals.iter().enumerate() {
            match v {
                Val::One { node: Some(n), ty, name } => {
                    // Fall back to the callee's declared parameter type unless
                    // it is a bare generic name (which says nothing).
                    let fallback = params
                        .get(k)
                        .map(|(_, t)| t.as_str())
                        .filter(|t| t.contains('<') || t.len() > 2);
                    let label = self.label_for(ty.as_deref().or(fallback), name.as_deref());
                    self.edge(*n, node, label);
                }
                Val::Slots { node: n, tys, .. } => {
                    let label = self.label_for(tys.first().and_then(|t| t.as_deref()), None);
                    self.edge(*n, node, label);
                }
                _ => {}
            }
        }
    }

    /// Output slot types of a crate fn call: turbofish consts substituted,
    /// `G::Rest` / `<G as ..>::Next` continuations inherited from the matching
    /// argument, `::Item` resolved through the supplier object.
    fn crate_fn_outputs(
        &mut self,
        fdef: &crate::FnDef,
        turbo: &[String],
        argvals: &[Val],
    ) -> (Vec<Option<String>>, bool) {
        let mut map: BTreeMap<String, String> = BTreeMap::new();
        if !turbo.is_empty() && turbo.len() == fdef.generics.len() {
            for (g, a) in fdef.generics.iter().zip(turbo.iter()) {
                let a = a.trim();
                if a != "_" {
                    map.insert(g.name.clone(), as_number(a).unwrap_or_else(|| a.to_string()));
                }
            }
        } else if !turbo.is_empty() {
            self.warn(format!(
                "turbofish arity {} != generics arity {} on `{}`",
                turbo.len(),
                fdef.generics.len(),
                fdef.name
            ));
        }
        let arg_ty = |k: usize| -> Option<String> {
            match argvals.get(k) {
                Some(Val::One { ty, .. }) => ty.clone(),
                Some(Val::Slots { tys, .. }) => tys.first().cloned().flatten(),
                _ => None,
            }
        };
        // A generic type-parameter name -> the argument type passed for it.
        let param_for_generic = |g: &str| -> Option<usize> {
            fdef.params.iter().position(|(_, t)| t.trim() == g)
        };

        let resolve_component = |tr: &Self, comp: &str| -> Option<String> {
            let mut c = comp.trim();
            // `super::Kettle` / `crate::x::Kettle` are plain types.
            while let Some(rest) = c.strip_prefix("super::").or_else(|| c.strip_prefix("crate::")) {
                c = rest;
            }
            // Continuations: `S::Rest`, `S::Next`, `<G as Trait<..>>::Next`.
            let leading: String = c
                .trim_start_matches('<')
                .chars()
                .take_while(|ch| ch.is_alphanumeric() || *ch == '_')
                .collect();
            // Associated-type projection: `<..>::X` or `G::X` — but NOT a
            // plain type whose generic arguments merely contain `::`.
            let is_assoc = c.starts_with('<')
                || c.strip_prefix(leading.as_str()).map(|r| r.starts_with("::")).unwrap_or(false);
            if is_assoc {
                if let Some(k) = param_for_generic(&leading) {
                    if c.ends_with("::Rest") || c.ends_with("::Next") {
                        // Continuation of the same object: keep the base name
                        // only (its const/list state has changed).
                        return arg_ty(k).map(|t| base_name(&t)).filter(|b| !b.is_empty());
                    }
                    if c.ends_with("::Item") || c.ends_with("::Taken") {
                        let obj = arg_ty(k).map(|t| base_name(&t)).unwrap_or_default();
                        return tr.cm.supplier_item(&obj).or(Some("item".to_string()));
                    }
                }
                // `<V::Next as Supplier>::Item` and friends.
                if let Some(k) = param_for_generic(&leading) {
                    return arg_ty(k);
                }
                return None;
            }
            // A bare generic name: the argument's type.
            if let Some(k) = param_for_generic(c) {
                return arg_ty(k).or(Some(c.to_string()));
            }
            Some(subst_idents(c, &map))
        };

        // Fallible: a single `Result<Ok, Err>` return.
        if fdef.ret.len() == 1 && base_name(&fdef.ret[0]) == "Result" {
            let subst = subst_idents(&fdef.ret[0], &map);
            let arms = generic_args(&subst);
            let tys: Vec<Option<String>> = arms.iter().map(|a| Some(a.clone())).collect();
            return (tys, true);
        }
        let tys: Vec<Option<String>> =
            fdef.ret.iter().map(|r| resolve_component(self, r)).collect();
        (tys, false)
    }
}

// ---------------------------------------------------------------------------
// Pattern helpers.
// ---------------------------------------------------------------------------

/// `let` patterns: `x`, `_x`, `(a, b, _)`. Returns (slot idents, all-underscore?).
fn parse_pattern(cur: &mut Cur) -> (Vec<Option<String>>, bool) {
    cur.skip_ws();
    let mut idents: Vec<Option<String>> = Vec::new();
    if cur.peek() == Some(b'(') {
        cur.i += 1;
        loop {
            cur.skip_ws();
            if cur.peek() == Some(b')') {
                cur.i += 1;
                break;
            }
            let id = cur.ident();
            idents.push(if id.is_empty() || id == "_" { None } else { Some(id) });
            cur.skip_ws();
            if cur.peek() == Some(b',') {
                cur.i += 1;
            }
        }
    } else {
        let id = cur.ident();
        idents.push(if id.is_empty() || id == "_" { None } else { Some(id) });
    }
    let all_underscore = idents
        .iter()
        .all(|x| x.as_ref().map(|s| s.starts_with('_')).unwrap_or(true));
    (idents, all_underscore)
}

/// A `let` type annotation: single type or tuple; `_` components become None.
fn parse_annotation(text: &str) -> Vec<Option<String>> {
    let t = text.trim();
    let comps: Vec<String> = if t.starts_with('(') && t.ends_with(')') {
        split_top(&t[1..t.len() - 1], ',')
    } else {
        vec![t.to_string()]
    };
    comps
        .into_iter()
        .map(|c| {
            let c = c.trim().to_string();
            if c == "_" || c.is_empty() { None } else { Some(c) }
        })
        .collect()
}
