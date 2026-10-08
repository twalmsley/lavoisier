//! The `.lav` notation's abstract syntax (EXP-14).
//!
//! The notation is deliberately minimal and declarative: resources with
//! quantities/units, boundary sources and sinks, characteristics,
//! requirements (the R10 pattern), processes with balances (R3/R15), and
//! flows (R9). Everything carries its `.lav` line so the generator can emit
//! breadcrumb comments and the validator can point at the source.

/// A parsed `.lav` model.
#[derive(Debug, Default)]
pub struct Model {
    /// Crate name from the `model <name>` line.
    pub name: String,
    /// One-sentence crate description (`doc` attribute of `model`).
    pub doc: String,
    /// Line of the `model` header.
    pub line: usize,
    /// Containers and reusables, in declaration order.
    pub resources: Vec<Resource>,
    /// Unbounded boundary sources (R15 draw-style).
    pub sources: Vec<Source>,
    /// Unbounded boundary sinks (`Next = Self`, R15/F-029).
    pub sinks: Vec<Sink>,
    /// Characteristic traits (R6).
    pub characteristics: Vec<Characteristic>,
    /// Requirements (R10).
    pub requirements: Vec<Requirement>,
    /// Processes (R1/R3).
    pub processes: Vec<Process>,
    /// Flows (R9), each generated as a `pub fn` of `src/flows.rs` (R20).
    pub flows: Vec<Flow>,
}

/// `container <Name>[<EXTRA..>] unit "<unit>"` or `reusable <Name>`.
#[derive(Debug)]
pub struct Resource {
    pub name: String,
    pub kind: ResKind,
    /// The magnitude's unit string (containers only).
    pub unit: String,
    /// Extra const parameters before the implicit magnitude `V`.
    pub extra_consts: Vec<String>,
    pub doc: String,
    pub placeholder: Option<String>,
    pub line: usize,
}

/// Container (sealed quantity, R15) vs reusable (R2).
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ResKind {
    Container,
    Reusable,
}

impl Resource {
    /// Total const parameters: the extras plus the magnitude `V` for
    /// containers; none for reusables.
    pub fn const_count(&self) -> usize {
        match self.kind {
            ResKind::Container => self.extra_consts.len() + 1,
            ResKind::Reusable => 0,
        }
    }
}

/// `source <Name> draws <Resource> via <fn>` — an unbounded boundary source
/// drawn from by a draw-style process (R15, F-028).
#[derive(Debug)]
pub struct Source {
    pub name: String,
    pub resource: String,
    pub draw_fn: String,
    pub doc: String,
    pub placeholder: Option<String>,
    pub line: usize,
}

/// `sink <Name> accepts <Resource>` — an unbounded boundary consumer
/// (`Next = Self`, R15/F-029).
#[derive(Debug)]
pub struct Sink {
    pub name: String,
    pub accepts: String,
    pub doc: String,
    pub placeholder: Option<String>,
    pub line: usize,
}

/// `characteristic <Name> on <Resource>` with carried quantities (R6/R7)
/// and at most one permit-gated conserving extraction (the cs1 pattern).
#[derive(Debug)]
pub struct Characteristic {
    pub name: String,
    pub on: String,
    pub doc: String,
    pub carries: Vec<Carry>,
    pub extraction: Option<Extraction>,
    pub line: usize,
}

/// `carries <CONST> "<doc>"` (maps to the resource's extra const of the same
/// name) or `carries <CONST> = V "<doc>"` (maps to the magnitude).
#[derive(Debug)]
pub struct Carry {
    pub name: String,
    pub from_v: bool,
    pub doc: String,
    pub line: usize,
}

/// `extraction <fn> -> <Resource> "<doc>"`.
#[derive(Debug)]
pub struct Extraction {
    pub fn_name: String,
    pub returns: String,
    pub doc: String,
    pub line: usize,
}

/// `requirement REQ-NNN <Suffix> "<sentence>" requires <Characteristic>`.
#[derive(Debug)]
pub struct Requirement {
    /// Full id, e.g. `REQ-001`.
    pub id: String,
    /// The three digits, e.g. `001`.
    pub num: String,
    /// CamelCase trait-name suffix, e.g. `PouredAtTheBoil`.
    pub suffix: String,
    pub sentence: String,
    pub requires: String,
    /// `#[diagnostic::on_unimplemented]` message / label / note (R10 rule 8).
    pub error: String,
    pub label: String,
    pub note: String,
    /// `example <Alias> = <Resource><n, ..>`: the concrete satisfying type,
    /// generated as a tagged alias plus `satisfies!` assertion (F-020/F-037).
    pub example: Option<Example>,
    pub line: usize,
}

impl Requirement {
    /// The generated trait name, e.g. `Req001PouredAtTheBoil`.
    pub fn trait_name(&self) -> String {
        format!("Req{}{}", self.num, self.suffix)
    }
    /// The generated assert helper, e.g. `assert_req001`.
    pub fn assert_name(&self) -> String {
        format!("assert_req{}", self.num)
    }
}

/// The concrete satisfying type named by a requirement's `example` line.
#[derive(Debug)]
pub struct Example {
    pub alias: String,
    pub resource: String,
    pub args: Vec<u64>,
    pub doc: String,
    pub line: usize,
}

/// `process <name>` with `in`/`out` params and `balance` lines.
#[derive(Debug)]
pub struct Process {
    pub name: String,
    pub doc: String,
    pub inputs: Vec<Param>,
    pub outputs: Vec<Output>,
    pub balances: Vec<Balance>,
    pub line: usize,
}

/// One `in` line.
#[derive(Debug)]
pub struct Param {
    pub var: String,
    pub ty: ParamTy,
    pub line: usize,
    /// Column of the type name, for diagnostics.
    pub ty_col: usize,
}

/// The type position of an `in` line.
#[derive(Debug)]
pub enum ParamTy {
    /// `Person<B>` — model-core's common person with a budget const (R11/R15).
    Person { budget: String },
    /// `<Resource>` or `<Resource><C, ..>` — a declared resource with const
    /// idents binding its parameters.
    Concrete {
        resource: String,
        consts: Vec<String>,
    },
    /// `<T> requires REQ-NNN` — generic, bounded by the requirement trait
    /// (R10/F-019). Requirement satisfaction is checked by RUST, not by the
    /// notation: that is the agreed division of labour between the two error
    /// layers.
    Generic { ident: String, requires: String },
}

/// One `out` line.
#[derive(Debug)]
pub enum Output {
    /// `out <var>` — a pass-through of the input of the same name (R2).
    PassThrough { var: String, line: usize },
    /// `out <var>: <Resource><C, ..>` — a fresh conserved output; consts not
    /// bound by any input become caller-stated const parameters (F-022).
    Fresh {
        var: String,
        resource: String,
        consts: Vec<String>,
        line: usize,
    },
    /// `out <var>: <Resource> via <extraction>(<in-var>)` — produced by a
    /// characteristic's permit-gated conserving extraction.
    Extracted {
        var: String,
        resource: String,
        extraction: String,
        from_var: String,
        line: usize,
    },
}

/// `balance <unit> "<label>": <terms> = <terms>` — a per-dimension
/// conservation statement (R3/R15), generated as a `const` assert. The
/// notation does NOT evaluate it: arithmetic truth is Rust's half of the
/// two-layer error story (F-001).
#[derive(Debug)]
pub struct Balance {
    pub unit: String,
    pub label: String,
    pub lhs: Vec<Term>,
    pub rhs: Vec<Term>,
    pub line: usize,
}

/// A balance term.
#[derive(Debug)]
pub struct Term {
    /// `K.WATER_G` has `param = Some("K")`; a bare const has `None`.
    pub param: Option<String>,
    pub konst: String,
    pub col: usize,
}

/// `flow <name>` with statements, generated as a `pub fn` plus one
/// integration test tagged `Verifies:` (R10).
#[derive(Debug)]
pub struct Flow {
    pub name: String,
    pub doc: String,
    pub verifies: Vec<String>,
    pub stmts: Vec<Stmt>,
    pub line: usize,
}

/// One flow statement.
#[derive(Debug)]
pub enum Stmt {
    /// `person <var> budget <N>` — model-core `new_person::<N>()`.
    Person { var: String, budget: u64, line: usize },
    /// `history <var>` — the flow's single R16 record.
    History { var: String, line: usize },
    /// `new <var> <Name>` — a reusable, source or sink enters at the boundary.
    New { var: String, ty: String, ty_col: usize, line: usize },
    /// `take <var> <N> from <source-var>` — the source's draw process (R15).
    Take {
        var: String,
        amount: u64,
        from: String,
        from_col: usize,
        line: usize,
    },
    /// `spend <person-var> <N> on <process>` — the F-048 adjacent
    /// `draw_time`, recorded to the flow's history (R16).
    Spend {
        person: String,
        amount: u64,
        process: String,
        line: usize,
    },
    /// `do <outs..> = <process>(<args..>) [with C = N, ..]`.
    Do {
        outs: Vec<String>,
        process: String,
        proc_col: usize,
        args: Vec<(String, usize)>,
        with: Vec<(String, u64)>,
        line: usize,
    },
    /// `send <var> to <sink-var>`.
    Send {
        var: String,
        var_col: usize,
        to: String,
        to_col: usize,
        line: usize,
    },
    /// `end <vars..>` — flow-end accounting (R1): every live value must be
    /// listed; everything listed must be live.
    End { vars: Vec<(String, usize)>, line: usize },
}

impl Stmt {
    /// The statement's source line.
    pub fn line(&self) -> usize {
        match self {
            Stmt::Person { line, .. }
            | Stmt::History { line, .. }
            | Stmt::New { line, .. }
            | Stmt::Take { line, .. }
            | Stmt::Spend { line, .. }
            | Stmt::Do { line, .. }
            | Stmt::Send { line, .. }
            | Stmt::End { line, .. } => *line,
        }
    }
}

/// A notation-level diagnostic: the error layer the tool owns outright.
#[derive(Debug)]
pub struct Diag {
    pub line: usize,
    pub col: usize,
    pub msg: String,
    pub help: Option<String>,
}

impl Diag {
    pub fn new(line: usize, col: usize, msg: impl Into<String>) -> Self {
        Diag {
            line,
            col,
            msg: msg.into(),
            help: None,
        }
    }
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Renders the diagnostic in the familiar rustc shape, but with modeller
    /// phrasing and a `.lav` span.
    pub fn render(&self, file: &str, src: &str) -> String {
        let mut out = String::new();
        out.push_str(&format!("error: {}\n", self.msg));
        out.push_str(&format!("  --> {}:{}:{}\n", file, self.line, self.col));
        if let Some(text) = src.lines().nth(self.line.saturating_sub(1)) {
            let n = format!("{}", self.line);
            let pad = " ".repeat(n.len());
            out.push_str(&format!("{} |\n", pad));
            out.push_str(&format!("{} | {}\n", n, text));
            let mut caret = String::new();
            for ch in text.chars().take(self.col.saturating_sub(1)) {
                caret.push(if ch == '\t' { '\t' } else { ' ' });
            }
            out.push_str(&format!("{} | {}^\n", pad, caret));
        }
        if let Some(h) = &self.help {
            out.push_str(&format!("  = help: {}\n", h));
        }
        out
    }
}
