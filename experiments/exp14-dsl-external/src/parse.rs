//! The `.lav` parser: line-oriented, std-only (diagram-gen house style).
//!
//! Top-level item headers start in column 1; attribute and statement lines
//! are indented under the most recent header. `#` starts a comment. The
//! parser stops at the first syntax error; reference and flow errors are
//! collected by `check` so a modeller sees them all at once.

use crate::ast::*;

/// A cursor over one line, tracking the 1-based column for diagnostics.
struct Cur<'a> {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    _src: &'a str,
}

impl<'a> Cur<'a> {
    fn new(text: &'a str, line: usize) -> Self {
        Cur {
            chars: text.chars().collect(),
            pos: 0,
            line,
            _src: text,
        }
    }
    fn col(&self) -> usize {
        self.pos + 1
    }
    fn skip_ws(&mut self) {
        while self.pos < self.chars.len() && self.chars[self.pos].is_whitespace() {
            self.pos += 1;
        }
    }
    fn at_end(&mut self) -> bool {
        self.skip_ws();
        self.pos >= self.chars.len() || self.chars[self.pos] == '#'
    }
    fn peek(&mut self) -> Option<char> {
        self.skip_ws();
        self.chars.get(self.pos).copied()
    }
    fn eat(&mut self, ch: char) -> bool {
        if self.peek() == Some(ch) {
            self.pos += 1;
            true
        } else {
            false
        }
    }
    fn expect(&mut self, ch: char, what: &str) -> Result<(), Diag> {
        if self.eat(ch) {
            Ok(())
        } else {
            Err(Diag::new(
                self.line,
                self.col(),
                format!("expected `{}` {}", ch, what),
            ))
        }
    }
    /// An identifier: letters, digits, `_`, `-` (for REQ-NNN ids).
    fn ident(&mut self, what: &str) -> Result<(String, usize), Diag> {
        self.skip_ws();
        let start = self.pos;
        while self.pos < self.chars.len()
            && (self.chars[self.pos].is_alphanumeric()
                || self.chars[self.pos] == '_'
                || self.chars[self.pos] == '-')
        {
            self.pos += 1;
        }
        if self.pos == start {
            return Err(Diag::new(
                self.line,
                self.col(),
                format!("expected {}", what),
            ));
        }
        Ok((self.chars[start..self.pos].iter().collect(), start + 1))
    }
    fn number(&mut self, what: &str) -> Result<u64, Diag> {
        let (word, col) = self.ident(what)?;
        let digits: String = word.chars().filter(|c| *c != '_').collect();
        digits.parse::<u64>().map_err(|_| {
            Diag::new(
                self.line,
                col,
                format!("expected {} (a whole number in base units, R7), found `{}`", what, word),
            )
        })
    }
    fn keyword(&mut self, kw: &str) -> Result<(), Diag> {
        let col = {
            self.skip_ws();
            self.col()
        };
        let (word, _) = self.ident(&format!("the keyword `{}`", kw))?;
        if word == kw {
            Ok(())
        } else {
            Err(Diag::new(
                self.line,
                col,
                format!("expected the keyword `{}`, found `{}`", kw, word),
            ))
        }
    }
    fn qstring(&mut self, what: &str) -> Result<String, Diag> {
        self.skip_ws();
        if self.chars.get(self.pos) != Some(&'"') {
            return Err(Diag::new(
                self.line,
                self.col(),
                format!("expected a quoted {} string", what),
            ));
        }
        self.pos += 1;
        let start = self.pos;
        while self.pos < self.chars.len() && self.chars[self.pos] != '"' {
            self.pos += 1;
        }
        if self.pos >= self.chars.len() {
            return Err(Diag::new(self.line, start, "unterminated string".to_string()));
        }
        let s: String = self.chars[start..self.pos].iter().collect();
        self.pos += 1;
        Ok(s)
    }
    /// `<A, B>` — optional angle-bracketed ident list.
    fn angle_idents(&mut self) -> Result<Vec<String>, Diag> {
        let mut out = Vec::new();
        if self.eat('<') {
            loop {
                let (id, _) = self.ident("a const parameter name")?;
                out.push(id);
                if self.eat('>') {
                    break;
                }
                self.expect(',', "between const parameters")?;
            }
        }
        Ok(out)
    }
    fn done(&mut self, item: &str) -> Result<(), Diag> {
        if self.at_end() {
            Ok(())
        } else {
            Err(Diag::new(
                self.line,
                self.col(),
                format!("unexpected text after {}", item),
            ))
        }
    }
}

/// Which item the indented lines currently attach to.
enum Ctx {
    None,
    ModelHeader,
    Resource,
    Source,
    Sink,
    Characteristic,
    Requirement,
    Process,
    Flow,
}

/// Parses a `.lav` file. Returns the model or the first syntax error.
pub fn parse(src: &str) -> Result<Model, Diag> {
    let mut m = Model::default();
    let mut ctx = Ctx::None;
    let mut seen_model = false;

    for (idx, raw) in src.lines().enumerate() {
        let line_no = idx + 1;
        let trimmed = raw.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let indented = raw.starts_with(' ') || raw.starts_with('\t');
        let mut c = Cur::new(raw, line_no);
        if !indented {
            let (kw, kw_col) = c.ident("an item keyword")?;
            match kw.as_str() {
                "model" => {
                    let (name, _) = c.ident("the model name")?;
                    c.done("the model name")?;
                    m.name = name;
                    m.line = line_no;
                    seen_model = true;
                    ctx = Ctx::ModelHeader;
                }
                "container" | "reusable" => {
                    let (name, _) = c.ident("the resource name")?;
                    let extra = c.angle_idents()?;
                    let (kind, unit) = if kw == "container" {
                        c.keyword("unit")?;
                        (ResKind::Container, c.qstring("unit")?)
                    } else {
                        (ResKind::Reusable, String::new())
                    };
                    c.done("the resource declaration")?;
                    if kind == ResKind::Reusable && !extra.is_empty() {
                        return Err(Diag::new(
                            line_no,
                            kw_col,
                            "a reusable resource carries no quantity parameters - only containers do (R15)",
                        ));
                    }
                    m.resources.push(Resource {
                        name,
                        kind,
                        unit,
                        extra_consts: extra,
                        doc: String::new(),
                        placeholder: None,
                        line: line_no,
                    });
                    ctx = Ctx::Resource;
                }
                "source" => {
                    let (name, _) = c.ident("the source name")?;
                    c.keyword("draws")?;
                    let (resource, _) = c.ident("the drawn resource")?;
                    c.keyword("via")?;
                    let (draw_fn, _) = c.ident("the draw process name")?;
                    c.done("the source declaration")?;
                    m.sources.push(Source {
                        name,
                        resource,
                        draw_fn,
                        doc: String::new(),
                        placeholder: None,
                        line: line_no,
                    });
                    ctx = Ctx::Source;
                }
                "sink" => {
                    let (name, _) = c.ident("the sink name")?;
                    c.keyword("accepts")?;
                    let (accepts, _) = c.ident("the accepted resource")?;
                    c.done("the sink declaration")?;
                    m.sinks.push(Sink {
                        name,
                        accepts,
                        doc: String::new(),
                        placeholder: None,
                        line: line_no,
                    });
                    ctx = Ctx::Sink;
                }
                "characteristic" => {
                    let (name, _) = c.ident("the characteristic name")?;
                    c.keyword("on")?;
                    let (on, _) = c.ident("the carrying resource")?;
                    c.done("the characteristic declaration")?;
                    m.characteristics.push(Characteristic {
                        name,
                        on,
                        doc: String::new(),
                        carries: Vec::new(),
                        extraction: None,
                        line: line_no,
                    });
                    ctx = Ctx::Characteristic;
                }
                "requirement" => {
                    let (id, id_col) = c.ident("the requirement id (REQ-NNN)")?;
                    let num = id.strip_prefix("REQ-").unwrap_or("").to_string();
                    if num.len() != 3 || !num.chars().all(|d| d.is_ascii_digit()) {
                        return Err(Diag::new(
                            line_no,
                            id_col,
                            format!(
                                "a requirement id is `REQ-` plus exactly three digits (R10), found `{}`",
                                id
                            ),
                        ));
                    }
                    let (suffix, _) = c.ident("the requirement trait suffix (CamelCase)")?;
                    let sentence = c.qstring("requirement sentence")?;
                    c.keyword("requires")?;
                    let (requires, _) = c.ident("the required characteristic")?;
                    c.done("the requirement declaration")?;
                    m.requirements.push(Requirement {
                        id,
                        num,
                        suffix,
                        sentence,
                        requires,
                        error: String::new(),
                        label: String::new(),
                        note: String::new(),
                        example: None,
                        line: line_no,
                    });
                    ctx = Ctx::Requirement;
                }
                "process" => {
                    let (name, _) = c.ident("the process name")?;
                    c.done("the process name")?;
                    m.processes.push(Process {
                        name,
                        doc: String::new(),
                        inputs: Vec::new(),
                        outputs: Vec::new(),
                        balances: Vec::new(),
                        line: line_no,
                    });
                    ctx = Ctx::Process;
                }
                "flow" => {
                    let (name, _) = c.ident("the flow name")?;
                    c.done("the flow name")?;
                    m.flows.push(Flow {
                        name,
                        doc: String::new(),
                        verifies: Vec::new(),
                        stmts: Vec::new(),
                        line: line_no,
                    });
                    ctx = Ctx::Flow;
                }
                other => {
                    return Err(Diag::new(
                        line_no,
                        kw_col,
                        format!(
                            "`{}` is not a .lav item - items are `model`, `container`, `reusable`, `source`, `sink`, `characteristic`, `requirement`, `process` and `flow`",
                            other
                        ),
                    ));
                }
            }
            continue;
        }

        // Indented: an attribute or statement of the current item.
        if !seen_model {
            return Err(Diag::new(
                line_no,
                c.col(),
                "the file must start with a `model <name>` line",
            ));
        }
        let (kw, kw_col) = c.ident("an attribute keyword")?;
        match (&ctx, kw.as_str()) {
            (Ctx::ModelHeader, "doc") => m.doc = c.qstring("doc")?,
            (Ctx::Resource, "doc") => m.resources.last_mut().unwrap().doc = c.qstring("doc")?,
            (Ctx::Resource, "placeholder") => {
                m.resources.last_mut().unwrap().placeholder = Some(c.qstring("placeholder")?)
            }
            (Ctx::Source, "doc") => m.sources.last_mut().unwrap().doc = c.qstring("doc")?,
            (Ctx::Source, "placeholder") => {
                m.sources.last_mut().unwrap().placeholder = Some(c.qstring("placeholder")?)
            }
            (Ctx::Sink, "doc") => m.sinks.last_mut().unwrap().doc = c.qstring("doc")?,
            (Ctx::Sink, "placeholder") => {
                m.sinks.last_mut().unwrap().placeholder = Some(c.qstring("placeholder")?)
            }
            (Ctx::Characteristic, "doc") => {
                m.characteristics.last_mut().unwrap().doc = c.qstring("doc")?
            }
            (Ctx::Characteristic, "carries") => {
                let (name, _) = c.ident("the carried const name")?;
                let from_v = if c.eat('=') {
                    c.keyword("V")?;
                    true
                } else {
                    false
                };
                let doc = c.qstring("doc")?;
                m.characteristics.last_mut().unwrap().carries.push(Carry {
                    name,
                    from_v,
                    doc,
                    line: line_no,
                });
            }
            (Ctx::Characteristic, "extraction") => {
                let (fn_name, _) = c.ident("the extraction fn name")?;
                c.expect('-', "in `->`")?;
                c.expect('>', "in `->`")?;
                let (returns, _) = c.ident("the returned resource")?;
                let doc = c.qstring("doc")?;
                m.characteristics.last_mut().unwrap().extraction = Some(Extraction {
                    fn_name,
                    returns,
                    doc,
                    line: line_no,
                });
            }
            (Ctx::Requirement, "error") => {
                m.requirements.last_mut().unwrap().error = c.qstring("error message")?
            }
            (Ctx::Requirement, "label") => {
                m.requirements.last_mut().unwrap().label = c.qstring("label")?
            }
            (Ctx::Requirement, "note") => {
                m.requirements.last_mut().unwrap().note = c.qstring("note")?
            }
            (Ctx::Requirement, "example") => {
                let (alias, _) = c.ident("the satisfying alias name")?;
                c.expect('=', "after the alias name")?;
                let (resource, _) = c.ident("the satisfying resource")?;
                let mut args = Vec::new();
                if c.eat('<') {
                    loop {
                        args.push(c.number("a const magnitude")?);
                        if c.eat('>') {
                            break;
                        }
                        c.expect(',', "between const magnitudes")?;
                    }
                }
                let doc = c.qstring("doc")?;
                m.requirements.last_mut().unwrap().example = Some(Example {
                    alias,
                    resource,
                    args,
                    doc,
                    line: line_no,
                });
            }
            (Ctx::Process, "doc") => m.processes.last_mut().unwrap().doc = c.qstring("doc")?,
            (Ctx::Process, "in") => {
                let (var, _) = c.ident("the input name")?;
                c.expect(':', "after the input name")?;
                c.skip_ws();
                let ty_col = c.col();
                let (tyname, _) = c.ident("the input's type")?;
                let consts = c.angle_idents()?;
                let ty = if !c.at_end() {
                    c.keyword("requires")?;
                    let (req, _) = c.ident("the requirement id")?;
                    if !consts.is_empty() {
                        return Err(Diag::new(
                            line_no,
                            ty_col,
                            "a `requires` input is generic: write a bare type variable, not a parameterised resource",
                        ));
                    }
                    ParamTy::Generic {
                        ident: tyname,
                        requires: req,
                    }
                } else if tyname == "Person" {
                    if consts.len() != 1 {
                        return Err(Diag::new(
                            line_no,
                            ty_col,
                            "`Person` carries exactly one const: its time budget in milliseconds (R15)",
                        ));
                    }
                    ParamTy::Person {
                        budget: consts.into_iter().next().unwrap(),
                    }
                } else {
                    ParamTy::Concrete {
                        resource: tyname,
                        consts,
                    }
                };
                m.processes.last_mut().unwrap().inputs.push(Param {
                    var,
                    ty,
                    line: line_no,
                    ty_col,
                });
            }
            (Ctx::Process, "out") => {
                let (var, _) = c.ident("the output name")?;
                let out = if c.eat(':') {
                    let (resource, _) = c.ident("the output's resource")?;
                    if c.at_end() {
                        Output::Fresh {
                            var,
                            resource,
                            consts: Vec::new(),
                            line: line_no,
                        }
                    } else if c.peek() == Some('<') {
                        let consts = c.angle_idents()?;
                        Output::Fresh {
                            var,
                            resource,
                            consts,
                            line: line_no,
                        }
                    } else {
                        c.keyword("via")?;
                        let (extraction, _) = c.ident("the extraction fn")?;
                        c.expect('(', "around the extracted input")?;
                        let (from_var, _) = c.ident("the extracted input")?;
                        c.expect(')', "around the extracted input")?;
                        Output::Extracted {
                            var,
                            resource,
                            extraction,
                            from_var,
                            line: line_no,
                        }
                    }
                } else {
                    Output::PassThrough { var, line: line_no }
                };
                m.processes.last_mut().unwrap().outputs.push(out);
            }
            (Ctx::Process, "balance") => {
                let (unit, _) = c.ident("the balanced unit")?;
                let label = c.qstring("balance label")?;
                c.expect(':', "after the balance label")?;
                let lhs = parse_terms(&mut c)?;
                c.expect('=', "between the balance sides")?;
                let rhs = parse_terms(&mut c)?;
                c.done("the balance")?;
                m.processes.last_mut().unwrap().balances.push(Balance {
                    unit,
                    label,
                    lhs,
                    rhs,
                    line: line_no,
                });
            }
            (Ctx::Flow, "doc") => m.flows.last_mut().unwrap().doc = c.qstring("doc")?,
            (Ctx::Flow, "verifies") => loop {
                let (id, _) = c.ident("a requirement id")?;
                m.flows.last_mut().unwrap().verifies.push(id);
                if !c.eat(',') {
                    c.done("the verifies list")?;
                    break;
                }
            },
            (Ctx::Flow, stmt_kw) => {
                let stmt = parse_stmt(stmt_kw, kw_col, &mut c)?;
                m.flows.last_mut().unwrap().stmts.push(stmt);
            }
            (_, other) => {
                return Err(Diag::new(
                    line_no,
                    kw_col,
                    format!("`{}` is not an attribute of this item", other),
                ));
            }
        }
    }

    if !seen_model {
        return Err(Diag::new(1, 1, "the file must start with a `model <name>` line"));
    }
    Ok(m)
}

fn parse_terms(c: &mut Cur) -> Result<Vec<Term>, Diag> {
    let mut terms = Vec::new();
    loop {
        c.skip_ws();
        let col = c.col();
        let (first, _) = c.ident("a magnitude name")?;
        let term = if c.eat('.') {
            let (konst, _) = c.ident("the carried const name")?;
            Term {
                param: Some(first),
                konst,
                col,
            }
        } else {
            Term {
                param: None,
                konst: first,
                col,
            }
        };
        terms.push(term);
        if !c.eat('+') {
            break;
        }
    }
    Ok(terms)
}

fn parse_stmt(kw: &str, kw_col: usize, c: &mut Cur) -> Result<Stmt, Diag> {
    let line = c.line;
    let stmt = match kw {
        "person" => {
            let (var, _) = c.ident("the person variable")?;
            c.keyword("budget")?;
            let budget = c.number("the time budget in milliseconds")?;
            Stmt::Person { var, budget, line }
        }
        "history" => {
            let (var, _) = c.ident("the history variable")?;
            Stmt::History { var, line }
        }
        "new" => {
            let (var, _) = c.ident("the variable name")?;
            c.skip_ws();
            let ty_col = c.col();
            let (ty, _) = c.ident("the boundary object's type")?;
            Stmt::New {
                var,
                ty,
                ty_col,
                line,
            }
        }
        "take" => {
            let (var, _) = c.ident("the variable name")?;
            let amount = c.number("the drawn amount")?;
            c.keyword("from")?;
            c.skip_ws();
            let from_col = c.col();
            let (from, _) = c.ident("the source variable")?;
            Stmt::Take {
                var,
                amount,
                from,
                from_col,
                line,
            }
        }
        "spend" => {
            let (person, _) = c.ident("the person variable")?;
            let amount = c.number("the spent time in milliseconds")?;
            c.keyword("on")?;
            let (process, _) = c.ident("the process the time is spent on")?;
            Stmt::Spend {
                person,
                amount,
                process,
                line,
            }
        }
        "do" => {
            let mut outs = Vec::new();
            loop {
                let (v, _) = c.ident("an output variable")?;
                outs.push(v);
                if !c.eat(',') {
                    break;
                }
            }
            c.expect('=', "between the outputs and the process call")?;
            c.skip_ws();
            let proc_col = c.col();
            let (process, _) = c.ident("the process name")?;
            c.expect('(', "around the arguments")?;
            let mut args = Vec::new();
            if !c.eat(')') {
                loop {
                    c.skip_ws();
                    let col = c.col();
                    let (a, _) = c.ident("an argument variable")?;
                    args.push((a, col));
                    if c.eat(')') {
                        break;
                    }
                    c.expect(',', "between arguments")?;
                }
            }
            let mut with = Vec::new();
            if !c.at_end() {
                c.keyword("with")?;
                loop {
                    let (name, _) = c.ident("a stated const name")?;
                    c.expect('=', "after the stated const name")?;
                    let v = c.number("the stated magnitude")?;
                    with.push((name, v));
                    if !c.eat(',') {
                        break;
                    }
                }
            }
            Stmt::Do {
                outs,
                process,
                proc_col,
                args,
                with,
                line,
            }
        }
        "send" => {
            c.skip_ws();
            let var_col = c.col();
            let (var, _) = c.ident("the sent variable")?;
            c.keyword("to")?;
            c.skip_ws();
            let to_col = c.col();
            let (to, _) = c.ident("the sink variable")?;
            Stmt::Send {
                var,
                var_col,
                to,
                to_col,
                line,
            }
        }
        "end" => {
            let mut vars = Vec::new();
            loop {
                c.skip_ws();
                let col = c.col();
                let (v, _) = c.ident("a variable")?;
                vars.push((v, col));
                if !c.eat(',') {
                    break;
                }
            }
            Stmt::End { vars, line }
        }
        other => {
            return Err(Diag::new(
                line,
                kw_col,
                format!(
                    "`{}` is not a flow statement - statements are `person`, `history`, `new`, `take`, `spend`, `do`, `send` and `end`",
                    other
                ),
            ));
        }
    };
    c.done("the statement")?;
    Ok(stmt)
}
