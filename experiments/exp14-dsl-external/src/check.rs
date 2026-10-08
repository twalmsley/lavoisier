//! The notation-level validator: the error layer the tool owns outright.
//!
//! The division of labour between the two error layers is deliberate:
//!
//! * the NOTATION checks names, references, arity, structure, and the R1/R2
//!   flow discipline (a resource used twice, a resource never accounted for)
//!   - with line/col spans and modeller phrasing;
//! * RUST checks what the types check: requirement satisfaction (R10, via the
//!   generated `on_unimplemented` messages) and conservation arithmetic
//!   (R3/R15 const asserts, post-monomorphization per F-001).
//!
//! In particular, `balance` lines and flow magnitudes are NOT evaluated here:
//! a violating `.lav` model generates a violating crate, so the Rust error
//! layer can be measured (the point of EXP-14). A production `lavc` could
//! evaluate the literal arithmetic and pre-empt every E0080 at the notation
//! layer - recorded in RESULTS.md as the decisive adaptation.

use crate::ast::*;
use std::collections::BTreeMap;

/// A process's resolved generic signature, in declaration order.
#[derive(Debug, Default)]
pub struct ProcSig {
    /// Const parameter names: inputs' consts first (order of appearance),
    /// then the fresh (caller-stated, F-022) output consts.
    pub const_params: Vec<String>,
    /// How many of `const_params` are bound by inputs (inferable at calls).
    pub input_bound: usize,
    /// Generic type parameters: (ident, requirement id).
    pub generics: Vec<(String, String)>,
}

/// A checked model plus everything codegen needs.
pub struct Checked<'m> {
    pub model: &'m Model,
    pub sigs: BTreeMap<String, ProcSig>,
    pub flows: Vec<FlowEval>,
}

/// One evaluated flow: statements with all magnitudes resolved.
pub struct FlowEval {
    pub stmts: Vec<GenStmt>,
    /// The `end` tuple: (variable, generated Rust type).
    pub end: Vec<(String, String)>,
}

/// A flow statement annotated for generation.
pub enum GenStmt {
    Person { var: String, budget: u64, lav: usize },
    History { var: String, lav: usize },
    NewReusable { var: String, ty: String, lav: usize },
    NewSource { var: String, ty: String, lav: usize },
    NewSink { var: String, ty: String, lav: usize },
    Take { var: String, amount: u64, from: String, draw_fn: String, lav: usize },
    Spend {
        person: String,
        take: u64,
        left: u64,
        budget: u64,
        process: String,
        history: String,
        lav: usize,
    },
    Do {
        outs: Vec<String>,
        process: String,
        /// Turbofish entries: decimal literals for consts, `_` for generics.
        turbofish: Vec<String>,
        args: Vec<String>,
        lav: usize,
    },
    Send { var: String, to: String, lav: usize },
}

/// What a flow variable holds during evaluation.
#[derive(Clone, Debug)]
enum VKind {
    Person { budget: u64 },
    History,
    Source { ty: String },
    Sink { ty: String },
    Res { resource: String, consts: Vec<u64> },
    /// Error recovery: the value of a statement that already failed. It
    /// satisfies every later check silently, so one mistake is reported once
    /// instead of cascading into "not defined" follow-ons.
    Unknown,
}

#[derive(Clone, Debug)]
struct VarState {
    kind: VKind,
    live: bool,
    consumed_at: Option<(usize, String)>,
}

/// Levenshtein distance for did-you-mean suggestions.
fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut cur = vec![i];
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            cur.push((prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost));
        }
        prev = cur;
    }
    prev[b.len()]
}

fn suggest<'a, I: Iterator<Item = &'a str>>(name: &str, candidates: I) -> Option<&'a str> {
    candidates
        .map(|c| (edit_distance(name, c), c))
        .filter(|(d, _)| *d <= 2)
        .min_by_key(|(d, _)| *d)
        .map(|(_, c)| c)
}

/// Validates the model and evaluates its flows. All errors are collected so
/// the modeller sees every problem in one run.
pub fn check(model: &Model) -> Result<Checked<'_>, Vec<Diag>> {
    let mut diags: Vec<Diag> = Vec::new();
    let res = |name: &str| model.resources.iter().find(|r| r.name == name);
    let chr = |name: &str| model.characteristics.iter().find(|c| c.name == name);
    let req = |id: &str| model.requirements.iter().find(|r| r.id == id);

    let missing_resource = |name: &str, line: usize, col: usize, role: &str| {
        let mut d = Diag::new(
            line,
            col,
            format!(
                "no resource called `{}` is declared - {} must name a declared `container` or `reusable`",
                name, role
            ),
        );
        if let Some(s) = suggest(name, model.resources.iter().map(|r| r.name.as_str())) {
            d = d.with_help(format!("a resource called `{}` is declared - did you mean that?", s));
        }
        d
    };

    // --- duplicate names across the model's type namespace -----------------
    {
        let mut seen: BTreeMap<&str, usize> = BTreeMap::new();
        let all: Vec<(&str, usize)> = model
            .resources
            .iter()
            .map(|r| (r.name.as_str(), r.line))
            .chain(model.sources.iter().map(|s| (s.name.as_str(), s.line)))
            .chain(model.sinks.iter().map(|s| (s.name.as_str(), s.line)))
            .chain(model.characteristics.iter().map(|c| (c.name.as_str(), c.line)))
            .collect();
        for (name, line) in all {
            if let Some(first) = seen.insert(name, line) {
                diags.push(Diag::new(
                    line,
                    1,
                    format!("`{}` is declared twice (first at line {})", name, first),
                ));
            }
        }
        let mut req_seen: BTreeMap<&str, usize> = BTreeMap::new();
        for r in &model.requirements {
            if let Some(first) = req_seen.insert(r.id.as_str(), r.line) {
                diags.push(Diag::new(
                    r.line,
                    1,
                    format!(
                        "requirement id `{}` is used twice (first at line {}) - ids are never reused (R10)",
                        r.id, first
                    ),
                ));
            }
        }
    }

    // --- boundary references ------------------------------------------------
    for s in &model.sources {
        match res(&s.resource) {
            None => diags.push(missing_resource(&s.resource, s.line, 1, "the drawn resource")),
            Some(r) => {
                if r.kind != ResKind::Container || !r.extra_consts.is_empty() {
                    diags.push(Diag::new(
                        s.line,
                        1,
                        format!(
                            "a source draws a plain quantity container (R15); `{}` is not one",
                            s.resource
                        ),
                    ));
                }
            }
        }
    }
    for s in &model.sinks {
        if res(&s.accepts).is_none() {
            diags.push(missing_resource(&s.accepts, s.line, 1, "the accepted resource"));
        }
    }

    // --- characteristics ----------------------------------------------------
    for ch in &model.characteristics {
        match res(&ch.on) {
            None => diags.push(missing_resource(&ch.on, ch.line, 1, "the carrying resource")),
            Some(r) => {
                for carry in &ch.carries {
                    if carry.from_v {
                        if r.kind != ResKind::Container {
                            diags.push(Diag::new(
                                carry.line,
                                1,
                                format!(
                                    "`carries {} = V` needs a container magnitude, but `{}` is reusable",
                                    carry.name, r.name
                                ),
                            ));
                        }
                    } else if !r.extra_consts.contains(&carry.name) {
                        diags.push(Diag::new(
                            carry.line,
                            1,
                            format!(
                                "`{}` carries no const called `{}` - its consts are <{}> plus the magnitude `V`",
                                r.name,
                                carry.name,
                                r.extra_consts.join(", ")
                            ),
                        ));
                    }
                }
            }
        }
        if let Some(ex) = &ch.extraction {
            match res(&ex.returns) {
                None => diags.push(missing_resource(&ex.returns, ex.line, 1, "the extraction's result")),
                Some(r) if r.const_count() != 0 => diags.push(Diag::new(
                    ex.line,
                    1,
                    format!(
                        "an extraction returns the bare vessel state; `{}` carries quantities, which must instead continue as process outputs checked by a balance (R1)",
                        ex.returns
                    ),
                )),
                _ => {}
            }
        }
    }

    // --- requirements -------------------------------------------------------
    for r in &model.requirements {
        if chr(&r.requires).is_none() {
            let mut d = Diag::new(
                r.line,
                1,
                format!(
                    "no characteristic called `{}` is declared - a requirement requires a declared `characteristic` (R10)",
                    r.requires
                ),
            );
            if let Some(s) = suggest(&r.requires, model.characteristics.iter().map(|c| c.name.as_str())) {
                d = d.with_help(format!("did you mean `{}`?", s));
            }
            diags.push(d);
        }
        if let Some(ex) = &r.example {
            match res(&ex.resource) {
                None => diags.push(missing_resource(&ex.resource, ex.line, 1, "the satisfying type")),
                Some(res_decl) => {
                    if ex.args.len() != res_decl.const_count() {
                        diags.push(Diag::new(
                            ex.line,
                            1,
                            format!(
                                "`{}` takes {} const magnitude(s), but the example states {}",
                                ex.resource,
                                res_decl.const_count(),
                                ex.args.len()
                            ),
                        ));
                    }
                }
            }
        }
        if r.error.is_empty() {
            diags.push(Diag::new(
                r.line,
                1,
                format!(
                    "requirement {} has no `error` message - every requirement carries a modeller-phrased diagnostic (R10 rule 8, F-044)",
                    r.id
                ),
            ));
        }
    }

    // --- processes ----------------------------------------------------------
    let mut sigs: BTreeMap<String, ProcSig> = BTreeMap::new();
    for p in &model.processes {
        let mut sig = ProcSig::default();
        let push_const = |sig: &mut ProcSig, name: &str| {
            if !sig.const_params.iter().any(|c| c == name) {
                sig.const_params.push(name.to_string());
            }
        };
        for input in &p.inputs {
            match &input.ty {
                ParamTy::Person { budget } => push_const(&mut sig, budget),
                ParamTy::Concrete { resource, consts } => {
                    match res(resource) {
                        None => diags.push(missing_resource(
                            resource,
                            input.line,
                            input.ty_col,
                            "a process input",
                        )),
                        Some(r) => {
                            if consts.len() != r.const_count() {
                                diags.push(Diag::new(
                                    input.line,
                                    input.ty_col,
                                    format!(
                                        "`{}` carries {} const parameter(s), but {} are written here",
                                        resource,
                                        r.const_count(),
                                        consts.len()
                                    ),
                                ));
                            }
                        }
                    }
                    for c in consts {
                        push_const(&mut sig, c);
                    }
                }
                ParamTy::Generic { ident, requires } => {
                    if req(requires).is_none() {
                        diags.push(Diag::new(
                            input.line,
                            input.ty_col,
                            format!("no requirement `{}` is declared", requires),
                        ));
                    }
                    sig.generics.push((ident.clone(), requires.clone()));
                }
            }
        }
        sig.input_bound = sig.const_params.len();
        for out in &p.outputs {
            match out {
                Output::PassThrough { var, line } => {
                    if !p.inputs.iter().any(|i| &i.var == var) {
                        diags.push(Diag::new(
                            *line,
                            1,
                            format!(
                                "`out {}` passes an input through, but `{}` has no input of that name",
                                var, p.name
                            ),
                        ));
                    }
                }
                Output::Fresh { resource, consts, line, .. } => {
                    match res(resource) {
                        None => diags.push(missing_resource(resource, *line, 1, "a process output")),
                        Some(r) => {
                            if consts.len() != r.const_count() {
                                diags.push(Diag::new(
                                    *line,
                                    1,
                                    format!(
                                        "`{}` carries {} const parameter(s), but {} are written here",
                                        resource,
                                        r.const_count(),
                                        consts.len()
                                    ),
                                ));
                            }
                        }
                    }
                    for c in consts {
                        push_const(&mut sig, c);
                    }
                }
                Output::Extracted { resource, extraction, from_var, line, .. } => {
                    let from = p.inputs.iter().find(|i| &i.var == from_var);
                    match from {
                        None => diags.push(Diag::new(
                            *line,
                            1,
                            format!("`{}` extracts from `{}`, but there is no such input", p.name, from_var),
                        )),
                        Some(Param { ty: ParamTy::Generic { requires, .. }, .. }) => {
                            let ch_name = req(requires).map(|r| r.requires.clone());
                            let found = ch_name
                                .as_deref()
                                .and_then(chr)
                                .and_then(|c| c.extraction.as_ref())
                                .filter(|e| e.fn_name == *extraction && e.returns == *resource);
                            if found.is_none() {
                                diags.push(Diag::new(
                                    *line,
                                    1,
                                    format!(
                                        "`{}` is not an extraction of the characteristic required of `{}`",
                                        extraction, from_var
                                    ),
                                ));
                            }
                        }
                        Some(_) => diags.push(Diag::new(
                            *line,
                            1,
                            format!(
                                "extractions belong to characteristics: `{}` must be a `requires` input to extract from it",
                                from_var
                            ),
                        )),
                    }
                }
            }
        }
        // Person inputs must come back (R2).
        for input in &p.inputs {
            if matches!(input.ty, ParamTy::Person { .. })
                && !p
                    .outputs
                    .iter()
                    .any(|o| matches!(o, Output::PassThrough { var, .. } if var == &input.var))
            {
                diags.push(Diag::new(
                    input.line,
                    1,
                    format!(
                        "`{}` takes the person `{}` but never returns them - a reusable resource is moved in and returned (R2)",
                        p.name, input.var
                    ),
                ));
            }
        }
        // Balance terms must name magnitudes of the process's signature (R3).
        for b in &p.balances {
            for t in b.lhs.iter().chain(b.rhs.iter()) {
                match &t.param {
                    None => {
                        if !sig.const_params.iter().any(|c| c == &t.konst) {
                            diags.push(
                                Diag::new(
                                    b.line,
                                    t.col,
                                    format!(
                                        "the balance for `{}` names `{}`, but no input or output of `{}` carries that magnitude",
                                        p.name, t.konst, p.name
                                    ),
                                )
                                .with_help(format!(
                                    "a balance may only sum magnitudes that enter or leave the process (R3); `{}` carries: {}",
                                    p.name,
                                    sig.const_params.join(", ")
                                )),
                            );
                        }
                    }
                    Some(param) => {
                        let generic = sig.generics.iter().find(|(g, _)| g == param);
                        match generic {
                            None => diags.push(Diag::new(
                                b.line,
                                t.col,
                                format!(
                                    "`{}.{}` reads a carried const, but `{}` is not a `requires` input of `{}`",
                                    param, t.konst, param, p.name
                                ),
                            )),
                            Some((_, req_id)) => {
                                let carried = req(req_id)
                                    .and_then(|r| chr(&r.requires))
                                    .map(|c| c.carries.iter().any(|k| k.name == t.konst))
                                    .unwrap_or(true);
                                if !carried {
                                    diags.push(Diag::new(
                                        b.line,
                                        t.col,
                                        format!(
                                            "the characteristic required of `{}` does not carry a const called `{}`",
                                            param, t.konst
                                        ),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
        sigs.insert(p.name.clone(), sig);
    }

    // --- flows --------------------------------------------------------------
    let mut flow_evals = Vec::new();
    for f in &model.flows {
        for v in &f.verifies {
            if req(v).is_none() {
                diags.push(Diag::new(
                    f.line,
                    1,
                    format!("`verifies {}` names an unknown requirement", v),
                ));
            }
        }
        match eval_flow(model, &sigs, f) {
            Ok(fe) => flow_evals.push(fe),
            Err(mut e) => diags.append(&mut e),
        }
    }

    if diags.is_empty() {
        Ok(Checked {
            model,
            sigs,
            flows: flow_evals,
        })
    } else {
        diags.sort_by_key(|d| (d.line, d.col));
        Err(diags)
    }
}

fn eval_flow(
    model: &Model,
    sigs: &BTreeMap<String, ProcSig>,
    f: &Flow,
) -> Result<FlowEval, Vec<Diag>> {
    let mut diags = Vec::new();
    let mut vars: BTreeMap<String, VarState> = BTreeMap::new();
    let mut order: Vec<String> = Vec::new(); // declaration order for liveness reports
    let mut stmts = Vec::new();
    let mut end: Vec<(String, String)> = Vec::new();
    let mut history_var: Option<String> = None;
    let mut ended = false;

    let res = |name: &str| model.resources.iter().find(|r| r.name == name);

    macro_rules! bail {
        ($d:expr) => {{
            diags.push($d);
            continue;
        }};
    }

    for stmt in &f.stmts {
        let line = stmt.line();
        if ended {
            diags.push(Diag::new(
                line,
                1,
                "no statements may follow `end` - it is the flow's final accounting (R1)",
            ));
            break;
        }
        // Bind a new variable; rebinding a live value would silently lose it.
        macro_rules! bind {
            ($var:expr, $kind:expr) => {{
                let var: &String = $var;
                if let Some(old) = vars.get(var) {
                    if old.live {
                        diags.push(Diag::new(
                            line,
                            1,
                            format!(
                                "rebinding `{}` would lose the value it still holds - account for it first (R1)",
                                var
                            ),
                        ));
                    }
                } else {
                    order.push(var.clone());
                }
                vars.insert(
                    var.clone(),
                    VarState {
                        kind: $kind,
                        live: true,
                        consumed_at: None,
                    },
                );
            }};
        }
        // Consume a live variable.
        macro_rules! consume {
            ($var:expr, $col:expr, $by:expr) => {{
                let var: &str = $var;
                match vars.get_mut(var) {
                    None => {
                        diags.push(Diag::new(
                            line,
                            $col,
                            format!("`{}` is not defined in this flow", var),
                        ));
                        None
                    }
                    Some(st) if !st.live => {
                        let (at, by) = st.consumed_at.clone().unwrap_or((0, String::new()));
                        diags.push(
                            Diag::new(
                                line,
                                $col,
                                format!(
                                    "`{}` was already used by `{}` at line {} - a resource can be in only one process at a time (R2)",
                                    var, by, at
                                ),
                            )
                            .with_help(
                                "use what that process returned instead; every process returns its reusable inputs (R2)".to_string(),
                            ),
                        );
                        None
                    }
                    Some(st) => {
                        st.live = false;
                        st.consumed_at = Some((line, $by.to_string()));
                        Some(st.kind.clone())
                    }
                }
            }};
        }

        match stmt {
            Stmt::Person { var, budget, line } => {
                bind!(var, VKind::Person { budget: *budget });
                stmts.push(GenStmt::Person {
                    var: var.clone(),
                    budget: *budget,
                    lav: *line,
                });
            }
            Stmt::History { var, line } => {
                history_var = Some(var.clone());
                bind!(var, VKind::History);
                stmts.push(GenStmt::History {
                    var: var.clone(),
                    lav: *line,
                });
            }
            Stmt::New { var, ty, ty_col, line } => {
                if let Some(r) = res(ty) {
                    if r.kind == ResKind::Container {
                        bail!(Diag::new(
                            *line,
                            *ty_col,
                            format!(
                                "`{}` is a conserved quantity container: it cannot be created with `new` - draw it from a source with `take` (R1, R15)",
                                ty
                            ),
                        ));
                    }
                    bind!(var, VKind::Res { resource: ty.clone(), consts: Vec::new() });
                    stmts.push(GenStmt::NewReusable {
                        var: var.clone(),
                        ty: ty.clone(),
                        lav: *line,
                    });
                } else if model.sources.iter().any(|s| &s.name == ty) {
                    bind!(var, VKind::Source { ty: ty.clone() });
                    stmts.push(GenStmt::NewSource {
                        var: var.clone(),
                        ty: ty.clone(),
                        lav: *line,
                    });
                } else if model.sinks.iter().any(|s| &s.name == ty) {
                    bind!(var, VKind::Sink { ty: ty.clone() });
                    stmts.push(GenStmt::NewSink {
                        var: var.clone(),
                        ty: ty.clone(),
                        lav: *line,
                    });
                } else {
                    let mut d = Diag::new(
                        *line,
                        *ty_col,
                        format!(
                            "no reusable resource, source or sink called `{}` is declared",
                            ty
                        ),
                    );
                    let candidates = model
                        .resources
                        .iter()
                        .filter(|r| r.kind == ResKind::Reusable)
                        .map(|r| r.name.as_str())
                        .chain(model.sources.iter().map(|s| s.name.as_str()))
                        .chain(model.sinks.iter().map(|s| s.name.as_str()));
                    if let Some(s) = suggest(ty, candidates) {
                        d = d.with_help(format!("did you mean `{}`?", s));
                    }
                    bail!(d);
                }
            }
            Stmt::Take { var, amount, from, from_col, line } => {
                let src_ty = match vars.get(from) {
                    Some(VarState { kind: VKind::Source { ty }, live: true, .. }) => ty.clone(),
                    Some(_) => bail!(Diag::new(
                        *line,
                        *from_col,
                        format!("`take` draws from a boundary source, but `{}` is not one (R15)", from),
                    )),
                    None => bail!(Diag::new(
                        *line,
                        *from_col,
                        format!("`{}` is not defined in this flow", from),
                    )),
                };
                let source = model.sources.iter().find(|s| s.name == src_ty).unwrap();
                bind!(var, VKind::Res { resource: source.resource.clone(), consts: vec![*amount] });
                stmts.push(GenStmt::Take {
                    var: var.clone(),
                    amount: *amount,
                    from: from.clone(),
                    draw_fn: source.draw_fn.clone(),
                    lav: *line,
                });
            }
            Stmt::Spend { person, amount, process, line } => {
                if !model.processes.iter().any(|p| &p.name == process) {
                    diags.push(Diag::new(
                        *line,
                        1,
                        format!(
                            "`spend ... on {}` names an unknown process - the spend is attributed to a declared process in the History (R16)",
                            process
                        ),
                    ));
                }
                let Some(hist) = history_var.clone() else {
                    bail!(Diag::new(
                        *line,
                        1,
                        "`spend` records labour into the flow's History - declare one with `history <var>` first (R16)",
                    ));
                };
                let budget = match vars.get(person) {
                    Some(VarState { kind: VKind::Person { budget }, live: true, .. }) => *budget,
                    Some(VarState { kind: VKind::Unknown, live: true, .. }) => 0,
                    _ => bail!(Diag::new(
                        *line,
                        1,
                        format!("`spend` draws a person's time, but `{}` is not a live person here (R15)", person),
                    )),
                };
                // Deliberately NOT an error when amount > budget: the
                // generated `draw_time` instantiation carries the overdraw to
                // the Rust layer (R15/F-001) - the arithmetic is Rust's half
                // of the two-layer error story.
                let left = budget.saturating_sub(*amount);
                vars.get_mut(person).unwrap().kind = VKind::Person { budget: left };
                stmts.push(GenStmt::Spend {
                    person: person.clone(),
                    take: *amount,
                    left,
                    budget,
                    process: process.clone(),
                    history: hist,
                    lav: *line,
                });
            }
            Stmt::Do { outs, process, proc_col, args, with, line } => {
                // Error recovery: when the statement fails, its outputs are
                // bound as Unknown so one mistake does not cascade.
                macro_rules! poison_outs {
                    () => {
                        for out in outs {
                            bind!(out, VKind::Unknown);
                        }
                    };
                }
                let Some(p) = model.processes.iter().find(|p| &p.name == process) else {
                    let mut d = Diag::new(
                        *line,
                        *proc_col,
                        format!("no process called `{}` is declared", process),
                    );
                    if let Some(s) = suggest(process, model.processes.iter().map(|p| p.name.as_str())) {
                        d = d.with_help(format!("did you mean `{}`?", s));
                    }
                    poison_outs!();
                    bail!(d);
                };
                let sig = &sigs[process];
                if args.len() != p.inputs.len() {
                    poison_outs!();
                    bail!(Diag::new(
                        *line,
                        *proc_col,
                        format!(
                            "`{}` takes {} input(s) ({}), but {} argument(s) are passed",
                            process,
                            p.inputs.len(),
                            p.inputs.iter().map(|i| i.var.as_str()).collect::<Vec<_>>().join(", "),
                            args.len()
                        ),
                    ));
                }
                if outs.len() != p.outputs.len() {
                    poison_outs!();
                    bail!(Diag::new(
                        *line,
                        1,
                        format!(
                            "`{}` returns {} output(s), but {} are bound - every output must be accounted for (R1)",
                            process,
                            p.outputs.len(),
                            outs.len()
                        ),
                    ));
                }
                // Bind const idents to magnitudes from the arguments.
                let mut binding: BTreeMap<String, u64> = BTreeMap::new();
                let mut arg_kinds: BTreeMap<String, VKind> = BTreeMap::new();
                let mut ok = true;
                for ((arg, col), input) in args.iter().zip(&p.inputs) {
                    let Some(kind) = consume!(arg.as_str(), *col, process) else {
                        ok = false;
                        continue;
                    };
                    arg_kinds.insert(input.var.clone(), kind.clone());
                    match (&input.ty, &kind) {
                        (_, VKind::Unknown) => {}
                        (ParamTy::Person { budget }, VKind::Person { budget: b }) => {
                            binding.insert(budget.clone(), *b);
                        }
                        (ParamTy::Person { .. }, _) => {
                            diags.push(Diag::new(
                                *line,
                                *col,
                                format!("`{}`'s `{}` input is a person, but `{}` is not one", process, input.var, arg),
                            ));
                            ok = false;
                        }
                        (ParamTy::Concrete { resource, consts }, VKind::Res { resource: have, consts: vals }) => {
                            if res(resource).is_none() {
                                // Already reported at the declaration; do not cascade.
                            } else if resource != have {
                                diags.push(Diag::new(
                                    *line,
                                    *col,
                                    format!(
                                        "`{}`'s `{}` input is {}, but `{}` holds {}",
                                        process, input.var, resource, arg, have
                                    ),
                                ));
                                ok = false;
                            } else {
                                for (ident, val) in consts.iter().zip(vals) {
                                    if let Some(prev) = binding.insert(ident.clone(), *val) {
                                        if prev != *val {
                                            diags.push(Diag::new(
                                                *line,
                                                *col,
                                                format!(
                                                    "`{}` binds `{}` to both {} and {} - the shared magnitude must agree",
                                                    process, ident, prev, val
                                                ),
                                            ));
                                            ok = false;
                                        }
                                    }
                                }
                            }
                        }
                        (ParamTy::Concrete { resource, .. }, _) => {
                            diags.push(Diag::new(
                                *line,
                                *col,
                                format!("`{}`'s `{}` input is {}, but `{}` is not a resource", process, input.var, resource, arg),
                            ));
                            ok = false;
                        }
                        // A `requires` input accepts any resource here: whether
                        // it satisfies the requirement is Rust's half of the
                        // error story (R10/F-044 on_unimplemented messages).
                        (ParamTy::Generic { .. }, VKind::Res { .. }) => {}
                        (ParamTy::Generic { .. }, _) => {
                            diags.push(Diag::new(
                                *line,
                                *col,
                                format!("`{}`'s `{}` input must be a resource", process, input.var),
                            ));
                            ok = false;
                        }
                    }
                }
                // `with` must state exactly the caller-stated consts (F-022).
                let fresh: Vec<&String> = sig.const_params[sig.input_bound..].iter().collect();
                for (name, v) in with {
                    if !fresh.iter().any(|f| *f == name) {
                        diags.push(Diag::new(
                            *line,
                            1,
                            format!(
                                "`with {} = {}` states a magnitude `{}` does not ask the caller for - its caller-stated magnitudes are: {}",
                                name,
                                v,
                                process,
                                if fresh.is_empty() { "(none)".to_string() } else { fresh.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ") }
                            ),
                        ));
                        ok = false;
                    } else {
                        binding.insert(name.clone(), *v);
                    }
                }
                for name in &fresh {
                    if !binding.contains_key(*name) {
                        diags.push(Diag::new(
                            *line,
                            1,
                            format!(
                                "`{}` needs the caller to state `{}` (outputs cannot be computed on stable Rust, F-022): add `with {} = <magnitude>`",
                                process, name, name
                            ),
                        ));
                        ok = false;
                    }
                }
                if !ok {
                    poison_outs!();
                    continue;
                }
                // Bind the outputs.
                for (out_var, out_decl) in outs.iter().zip(&p.outputs) {
                    let kind = match out_decl {
                        Output::PassThrough { var, .. } => {
                            arg_kinds.get(var).cloned().unwrap_or(VKind::Unknown)
                        }
                        Output::Fresh { resource, consts, .. } => {
                            match consts.iter().map(|c| binding.get(c).copied()).collect::<Option<Vec<u64>>>() {
                                Some(vals) => VKind::Res {
                                    resource: resource.clone(),
                                    consts: vals,
                                },
                                None => VKind::Unknown,
                            }
                        }
                        Output::Extracted { resource, .. } => VKind::Res {
                            resource: resource.clone(),
                            consts: Vec::new(),
                        },
                    };
                    bind!(out_var, kind);
                }
                // The turbofish: decimals for consts, `_` per generic.
                let mut tf: Vec<String> = sig
                    .const_params
                    .iter()
                    .map(|c| binding.get(c).map(|v| pretty_num(*v)).unwrap_or_else(|| "_".to_string()))
                    .collect();
                for _ in &sig.generics {
                    tf.push("_".to_string());
                }
                stmts.push(GenStmt::Do {
                    outs: outs.clone(),
                    process: process.clone(),
                    turbofish: tf,
                    args: args.iter().map(|(a, _)| a.clone()).collect(),
                    lav: *line,
                });
            }
            Stmt::Send { var, var_col, to, to_col, line } => {
                let sink_ty = match vars.get(to) {
                    Some(VarState { kind: VKind::Sink { ty }, live: true, .. }) => ty.clone(),
                    _ => bail!(Diag::new(
                        *line,
                        *to_col,
                        format!("`send` hands a resource to a boundary sink, but `{}` is not a live sink (R12)", to),
                    )),
                };
                let accepts = model.sinks.iter().find(|s| s.name == sink_ty).unwrap().accepts.clone();
                let Some(kind) = consume!(var.as_str(), *var_col, &format!("the {} sink", sink_ty)) else {
                    continue;
                };
                match kind {
                    VKind::Unknown => {}
                    VKind::Res { ref resource, .. } if *resource == accepts => {}
                    VKind::Res { ref resource, .. } => bail!(Diag::new(
                        *line,
                        *var_col,
                        format!(
                            "the `{}` sink accepts {}, not {} - every waste and product needs its own sanctioned exit (R12)",
                            sink_ty, accepts, resource
                        ),
                    )),
                    _ => bail!(Diag::new(
                        *line,
                        *var_col,
                        format!("only resources can be sent to a sink; `{}` is not one", var),
                    )),
                }
                stmts.push(GenStmt::Send {
                    var: var.clone(),
                    to: to.clone(),
                    lav: *line,
                });
            }
            Stmt::End { vars: listed, line } => {
                ended = true;
                for (v, col) in listed {
                    match vars.get(v) {
                        None => diags.push(Diag::new(
                            *line,
                            *col,
                            format!("`{}` is not defined in this flow", v),
                        )),
                        Some(st) if !st.live => {
                            let (at, by) = st.consumed_at.clone().unwrap_or((0, String::new()));
                            diags.push(Diag::new(
                                *line,
                                *col,
                                format!(
                                    "`{}` cannot be returned at flow end: it was already used by `{}` at line {} (R2)",
                                    v, by, at
                                ),
                            ));
                        }
                        Some(st) => {
                            let ty = match &st.kind {
                                VKind::Unknown => "()".to_string(),
                                VKind::Person { budget } => format!("Person<{}>", pretty_num(*budget)),
                                VKind::History => "History".to_string(),
                                VKind::Source { ty } | VKind::Sink { ty } => ty.clone(),
                                VKind::Res { resource, consts } => {
                                    let r = res(resource).unwrap();
                                    if r.kind == ResKind::Container {
                                        diags.push(Diag::new(
                                            *line,
                                            *col,
                                            format!(
                                                "`{}` holds {}, a conserved quantity: it cannot just be kept at flow end - hand it to a sink or a further process (R1)",
                                                v, resource
                                            ),
                                        ));
                                    }
                                    if consts.is_empty() {
                                        resource.clone()
                                    } else {
                                        format!(
                                            "{}<{}>",
                                            resource,
                                            consts.iter().map(|c| pretty_num(*c)).collect::<Vec<_>>().join(", ")
                                        )
                                    }
                                }
                            };
                            end.push((v.clone(), ty));
                        }
                    }
                }
                // Everything still live must be listed (R1).
                for v in &order {
                    let st = &vars[v];
                    if st.live && !listed.iter().any(|(l, _)| l == v) {
                        diags.push(
                            Diag::new(
                                *line,
                                1,
                                format!(
                                    "`{}` is never accounted for: at flow end every value must have reached a sink, a process, or this `end` line (R1)",
                                    v
                                ),
                            )
                            .with_help(
                                "nothing is silently lost: send it to a sink, pass it to a process, or list it in `end`".to_string(),
                            ),
                        );
                    }
                }
            }
        }
    }
    if !ended {
        diags.push(Diag::new(
            f.line,
            1,
            format!("flow `{}` has no `end` line - the flow's final accounting is mandatory (R1)", f.name),
        ));
    }

    if diags.is_empty() {
        Ok(FlowEval { stmts, end })
    } else {
        Err(diags)
    }
}

/// `550000` → `550_000`; numbers of up to four digits stay plain (the
/// workspace house style).
pub fn pretty_num(n: u64) -> String {
    let s = n.to_string();
    if s.len() <= 4 {
        return s;
    }
    let bytes: Vec<char> = s.chars().collect();
    let mut out = String::new();
    for (i, ch) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i) % 3 == 0 {
            out.push('_');
        }
        out.push(*ch);
    }
    out
}
