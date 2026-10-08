//! The SPEC_TEMPLATE.md parser: markdown sections §1..§8, std-only.
//!
//! Formatting discipline it relies on (every reliance is a candidate
//! template amendment, catalogued in RESULTS.md):
//! - `## N. Title` section headings with the template's numbering;
//! - §2 bullets shaped `- **REQ-NNN:** sentence` (continuation lines indented);
//! - §3/§4 pipe tables with the template's column order;
//! - §5 blocks headed `### Pk. Title` with `- **Field:** value` bullets
//!   (continuation lines indented, no leading `-`);
//! - numbers written as digits with space- or underscore-grouped thousands
//!   ("550 000" / "550_000"): a digit group followed by 3-digit groups is one
//!   number — which is why `1500 + 9` needs the spaces around `+`.

use crate::model::*;

/// Lexes the leading number from `s` (space/underscore-grouped), returning
/// the value and the rest. "550 000 J" → (550000, "J").
pub fn lex_number(s: &str) -> Option<(u64, &str)> {
    let s = s.trim_start();
    let first_len = s.chars().take_while(|c| c.is_ascii_digit()).count();
    if first_len == 0 {
        return None;
    }
    let mut value: u64 = s[..first_len].parse().ok()?;
    let mut rest = &s[first_len..];
    // Absorb further groups: "_000", " 000" (exactly 3 digits, else stop).
    loop {
        let r = rest;
        let sep = r.chars().next();
        let after = match sep {
            Some(' ') | Some('_') => &r[1..],
            _ => break,
        };
        let g = after.chars().take_while(|c| c.is_ascii_digit()).count();
        if g == 3 {
            let grp: u64 = after[..3].parse().ok()?;
            value = value * 1000 + grp;
            rest = &after[3..];
        } else {
            break;
        }
    }
    Some((value, rest))
}

/// Parses every quantity "N unit" found in `s`. Unit tokens are the word
/// immediately after a number; "each" marks a per-item quantity which is
/// kept with unit "<unit> each".
pub fn parse_quantities(s: &str) -> Vec<Qty> {
    let mut out = Vec::new();
    let mut rest = s;
    while let Some(idx) = rest.find(|c: char| c.is_ascii_digit()) {
        // Don't treat a digit inside a word (e.g. "N3") as a number start.
        let before = &rest[..idx];
        let attached = before
            .chars()
            .last()
            .map(|c| c.is_ascii_alphanumeric() || c == '-')
            .unwrap_or(false);
        if attached {
            rest = &rest[idx + 1..];
            continue;
        }
        let (value, after) = match lex_number(&rest[idx..]) {
            Some(v) => v,
            None => break,
        };
        let mut words = after.split_whitespace();
        let unit = words.next().unwrap_or("").trim_matches(|c: char| !c.is_ascii_alphanumeric());
        let unit = unit.trim_end_matches('.').to_string();
        let each = words.next().map(|w| w.starts_with("each")).unwrap_or(false);
        if !unit.is_empty() && unit.chars().all(|c| c.is_ascii_alphabetic()) {
            out.push(Qty {
                value,
                unit: if each { format!("{unit} each") } else { unit },
            });
        } else {
            out.push(Qty {
                value,
                unit: String::new(),
            });
        }
        rest = after;
    }
    out
}

/// Strips `(...)` parentheticals (top-level, non-nested is enough for the
/// template's prose).
pub fn strip_parens(s: &str) -> String {
    let mut out = String::new();
    let mut depth = 0usize;
    for c in s.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    out
}

/// Splits on a separator only at paren depth 0.
pub fn split_top_level(s: &str, seps: &[char]) -> Vec<String> {
    let mut parts = Vec::new();
    let mut cur = String::new();
    let mut depth = 0usize;
    for c in s.chars() {
        match c {
            '(' => {
                depth += 1;
                cur.push(c);
            }
            ')' => {
                depth = depth.saturating_sub(1);
                cur.push(c);
            }
            c if depth == 0 && seps.contains(&c) => {
                parts.push(cur.trim().to_string());
                cur = String::new();
            }
            c => cur.push(c),
        }
    }
    if !cur.trim().is_empty() {
        parts.push(cur.trim().to_string());
    }
    parts
}

struct Section {
    number: u32,
    start: usize, // line index (0-based) of first content line
    lines: Vec<(usize, String)>, // (1-based line no, text)
}

fn sections(text: &str) -> Vec<Section> {
    let mut secs: Vec<Section> = Vec::new();
    for (i, raw) in text.lines().enumerate() {
        if let Some(rest) = raw.strip_prefix("## ") {
            let num: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(n) = num.parse::<u32>() {
                secs.push(Section {
                    number: n,
                    start: i + 1,
                    lines: Vec::new(),
                });
                continue;
            }
        }
        if let Some(cur) = secs.last_mut() {
            cur.lines.push((i + 1, raw.to_string()));
        }
    }
    let _ = secs.iter().map(|s| s.start).count();
    secs
}

/// Joins `- ...` bullets with their indented continuation lines.
fn bullets(lines: &[(usize, String)]) -> Vec<(usize, String)> {
    let mut out: Vec<(usize, String)> = Vec::new();
    for (no, l) in lines {
        let t = l.trim_start();
        if t.starts_with("- ") || t.starts_with("* ") {
            out.push((*no, t[2..].trim().to_string()));
        } else if !t.is_empty()
            && !t.starts_with('#')
            && !t.starts_with('>')
            && !t.starts_with('|')
            && l.starts_with(' ')
        {
            if let Some(last) = out.last_mut() {
                last.1.push(' ');
                last.1.push_str(t);
            }
        } else if !t.is_empty() && !t.starts_with('#') && !t.starts_with('>') && !t.starts_with('|')
        {
            // Unindented prose continues the previous bullet too (CS-1 wraps
            // at the margin without indentation inside §5/§6 bullets).
            if let Some(last) = out.last_mut() {
                last.1.push(' ');
                last.1.push_str(t);
            }
        }
    }
    out
}

/// Parses a markdown pipe-table's body rows into cell vectors.
fn table_rows(lines: &[(usize, String)]) -> Vec<(usize, Vec<String>)> {
    let mut rows = Vec::new();
    for (no, l) in lines {
        let t = l.trim();
        if !t.starts_with('|') {
            continue;
        }
        // Skip header separator rows |---|---|
        if t.chars().all(|c| matches!(c, '|' | '-' | ' ' | ':')) {
            continue;
        }
        let cells: Vec<String> = t
            .trim_matches('|')
            .split('|')
            .map(|c| c.trim().to_string())
            .collect();
        rows.push((*no, cells));
    }
    rows
}

fn parse_req_remap(lines: &[(usize, String)]) -> Option<u32> {
    // The CS-1 implementation note: "... are implemented as **REQ-006..REQ-009** in spec order."
    // (blockquote lines wrap, so strip the "> " markers before joining)
    let joined: String = lines
        .iter()
        .map(|(_, l)| l.trim_start().trim_start_matches('>').trim())
        .collect::<Vec<_>>()
        .join(" ");
    let key = "implemented as **REQ-";
    let idx = joined.find(key)?;
    let after = &joined[idx + key.len()..];
    let digits: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

fn parse_requirements(sec: &Section, errors: &mut Vec<SpecError>) -> Vec<Requirement> {
    let mut reqs = Vec::new();
    for (no, b) in bullets(&sec.lines) {
        let Some(rest) = b.strip_prefix("**REQ-") else {
            continue;
        };
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        let Ok(id) = digits.parse::<u32>() else {
            errors.push(SpecError {
                line: no,
                section: "§2".into(),
                message: format!("requirement bullet has no parsable REQ id: '{b}'"),
            });
            continue;
        };
        let text = rest[digits.len()..]
            .trim_start_matches(":**")
            .trim_start_matches(":** ")
            .trim()
            .to_string();
        if text.is_empty() {
            errors.push(SpecError {
                line: no,
                section: "§2".into(),
                message: format!("REQ-{id:03} has an empty requirement sentence"),
            });
        }
        reqs.push(Requirement {
            spec_id: id,
            impl_id: id,
            text,
            line: no,
        });
    }
    // Workspace-global id remap (F-053), if §2 carries the implementation note.
    if let Some(base) = parse_req_remap(&sec.lines) {
        for (i, r) in reqs.iter_mut().enumerate() {
            r.impl_id = base + i as u32;
        }
    }
    reqs
}

fn parse_kind(cell: &str, line: usize, errors: &mut Vec<SpecError>) -> Kind {
    let c = cell.to_lowercase();
    if c.starts_with("discrete") {
        Kind::Discrete
    } else if c.starts_with("continuous") {
        Kind::Continuous {
            waste: c.contains("waste"),
        }
    } else if c.starts_with("reusable") {
        let note = c
            .find('(')
            .map(|i| c[i..].trim_matches(|ch| ch == '(' || ch == ')').to_string());
        Kind::Reusable { note }
    } else if c.starts_with("product") {
        Kind::Product
    } else {
        errors.push(SpecError {
            line,
            section: "§3".into(),
            message: format!(
                "unknown resource kind '{cell}': expected discrete / continuous / reusable / product"
            ),
        });
        Kind::Discrete
    }
}

fn parse_states(cell: &str) -> Vec<ResState> {
    if cell.trim() == "—" || cell.trim() == "-" || cell.trim().is_empty() {
        return Vec::new();
    }
    cell.split('→')
        .map(|part| {
            let part = part.trim();
            let (name, quantities) = match part.find('(') {
                Some(i) => {
                    let inner = part[i..].trim_matches(|c| c == '(' || c == ')');
                    (part[..i].trim().to_string(), parse_quantities(inner))
                }
                None => (part.to_string(), Vec::new()),
            };
            ResState { name, quantities }
        })
        .filter(|s| !s.name.is_empty())
        .collect()
}

fn parse_resources(sec: &Section, errors: &mut Vec<SpecError>) -> Vec<Resource> {
    let mut out = Vec::new();
    for (no, cells) in table_rows(&sec.lines) {
        if cells.len() < 5 {
            errors.push(SpecError {
                line: no,
                section: "§3".into(),
                message: format!(
                    "resource row has {} columns, the template needs 5 (Resource | Kind | Characteristics | Quantity & unit | States)",
                    cells.len()
                ),
            });
            continue;
        }
        if cells[0].eq_ignore_ascii_case("resource") {
            continue; // header
        }
        out.push(Resource {
            name: cells[0].clone(),
            kind: parse_kind(&cells[1], no, errors),
            characteristics: cells[2].clone(),
            quantity_raw: cells[3].clone(),
            states: parse_states(&cells[4]),
            line: no,
        });
    }
    out
}

fn parse_boundary(sec: &Section, errors: &mut Vec<SpecError>) -> (Vec<BoundaryRow>, Vec<BoundaryRow>) {
    // Two tables, keyed by the "**Inputs" / "**Outputs" headings.
    let mut inputs = Vec::new();
    let mut outputs = Vec::new();
    let mut in_inputs = false;
    let mut in_outputs = false;
    for (no, l) in &sec.lines {
        let t = l.trim();
        if t.starts_with("**Inputs") {
            in_inputs = true;
            in_outputs = false;
            continue;
        }
        if t.starts_with("**Outputs") {
            in_outputs = true;
            in_inputs = false;
            continue;
        }
        if !t.starts_with('|') {
            continue;
        }
        if t.chars().all(|c| matches!(c, '|' | '-' | ' ' | ':')) {
            continue;
        }
        let cells: Vec<String> = t
            .trim_matches('|')
            .split('|')
            .map(|c| c.trim().to_string())
            .collect();
        if cells.len() < 4 {
            errors.push(SpecError {
                line: *no,
                section: "§4".into(),
                message: format!(
                    "boundary row has {} columns, the template needs 4 (What | Via | Capacity | Real or placeholder?)",
                    cells.len()
                ),
            });
            continue;
        }
        if cells[0].to_lowercase().starts_with("what ") {
            continue; // header
        }
        let row = BoundaryRow {
            what: cells[0].clone(),
            via: cells[1].clone(),
            capacity: cells[2].clone(),
            status: cells[3].clone(),
            line: *no,
        };
        if in_inputs {
            inputs.push(row);
        } else if in_outputs {
            outputs.push(row);
        } else {
            errors.push(SpecError {
                line: *no,
                section: "§4".into(),
                message: "boundary row appears before the **Inputs**/**Outputs** heading".into(),
            });
        }
    }
    let _ = (in_inputs, in_outputs);
    (inputs, outputs)
}

/// Removes a "(= N)" running-total parenthetical from a balance clause.
fn strip_running_total(s: &str) -> String {
    match s.find("(=") {
        Some(i) => {
            let close = s[i..].find(')').map(|j| i + j + 1).unwrap_or(s.len());
            format!("{}{}", &s[..i], &s[close..])
        }
        None => s.to_string(),
    }
}

fn parse_balance_clause(
    clause: &str,
    line: usize,
    pid: u32,
    errors: &mut Vec<SpecError>,
) -> (Option<Balance>, Option<TimeDraw>) {
    let clause = clause.trim().trim_end_matches('.').trim();
    if clause.is_empty() {
        return (None, None);
    }
    let mark = if clause.contains("(assert)") {
        BalanceMark::Assert
    } else if clause.contains("(structural)") {
        BalanceMark::Structural
    } else {
        BalanceMark::Unmarked
    };
    let cleaned = clause.replace("(assert)", "").replace("(structural)", "");
    if cleaned.contains("→ History") || cleaned.contains("-> History") {
        // "time 30 000 ms → History"
        let qs = parse_quantities(&cleaned);
        match qs.first() {
            Some(q) => return (None, Some(TimeDraw { ms: q.value, line })),
            None => {
                errors.push(SpecError {
                    line,
                    section: format!("§5 P{pid}"),
                    message: format!("time clause has no parsable amount: '{clause}'"),
                });
                return (None, None);
            }
        }
    }
    let Some(eq) = cleaned.find('=') else {
        errors.push(SpecError {
            line,
            section: format!("§5 P{pid}"),
            message: format!(
                "balance clause has neither '=' nor '→ History': '{clause}' (template shape: '<dimension> a + b = c + d (assert|structural)')"
            ),
        });
        return (None, None);
    };
    let lhs_raw = strip_running_total(&cleaned[..eq]);
    let rhs_raw = strip_running_total(&cleaned[eq + 1..]);
    let dimension = lhs_raw
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_string();
    let side = |raw: &str| -> Vec<u64> {
        split_top_level(raw, &['+'])
            .iter()
            .filter_map(|term| lex_number(term.trim_start_matches(|c: char| !c.is_ascii_digit())).map(|(v, _)| v))
            .collect()
    };
    let lhs = side(&lhs_raw);
    let rhs = side(&rhs_raw);
    if lhs.is_empty() || rhs.is_empty() {
        errors.push(SpecError {
            line,
            section: format!("§5 P{pid}"),
            message: format!("balance clause has an empty side: '{clause}'"),
        });
        return (None, None);
    }
    (
        Some(Balance {
            dimension,
            lhs,
            rhs,
            mark,
            raw: clause.to_string(),
            line,
        }),
        None,
    )
}

fn parse_processes(sec: &Section, errors: &mut Vec<SpecError>) -> Vec<Process> {
    let mut procs: Vec<Process> = Vec::new();
    // Split into blocks on "### P<k>." headings, then bullet-parse each.
    let mut blocks: Vec<(u32, String, usize, Vec<(usize, String)>)> = Vec::new();
    for (no, l) in &sec.lines {
        let t = l.trim();
        if let Some(rest) = t.strip_prefix("### P") {
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(id) = digits.parse::<u32>() {
                let title = rest[digits.len()..].trim_start_matches('.').trim().to_string();
                blocks.push((id, title, *no, Vec::new()));
                continue;
            }
        }
        if let Some(b) = blocks.last_mut() {
            b.3.push((*no, l.clone()));
        }
    }
    for (id, title, hline, lines) in blocks {
        let mut p = Process {
            id,
            title,
            line: hline,
            ..Default::default()
        };
        for (no, b) in bullets(&lines) {
            let Some(colon) = b.find(":**") else { continue };
            let field = b[..colon].trim_start_matches("**").to_lowercase();
            let value = b[colon + 3..].trim().trim_end_matches('.').trim().to_string();
            match field.as_str() {
                f if f.starts_with("actor") => {
                    p.actors_raw = value.clone();
                    let low = value.to_lowercase();
                    if low.contains("no person") {
                        p.no_person = true;
                    }
                    if let Some(i) = low.find("draws") {
                        if let Some(q) = parse_quantities(&value[i..]).first() {
                            p.draws_ms = Some(q.value);
                        }
                    }
                }
                "consumes" => {
                    p.consumes_raw = value;
                    p.consumes_line = no;
                }
                "produces" => {
                    p.produces_raw = value;
                    p.produces_line = no;
                }
                "waste" => {
                    p.waste_raw = Some(value);
                    p.waste_line = no;
                }
                "waste routing" => p.waste_routing = Some(value),
                "balances" => {
                    for clause in split_top_level(&value, &[';']) {
                        let (bal, td) = parse_balance_clause(&clause, no, id, errors);
                        if let Some(b) = bal {
                            p.balances.push(b);
                        }
                        if let Some(t) = td {
                            p.time_draw = Some(t);
                        }
                    }
                }
                "satisfies" => {
                    let mut rest = value.as_str();
                    while let Some(i) = rest.find("REQ-") {
                        let digits: String = rest[i + 4..]
                            .chars()
                            .take_while(|c| c.is_ascii_digit())
                            .collect();
                        if let Ok(n) = digits.parse::<u32>() {
                            p.satisfies.push(n);
                        }
                        rest = &rest[i + 4..];
                    }
                }
                _ => {} // "failure modes" and unknown fields: prose, not consumed
            }
        }
        procs.push(p);
    }
    procs
}

fn parse_flows(sec: &Section) -> Flows {
    let mut flows = Flows {
        line: sec.lines.first().map(|(n, _)| *n).unwrap_or(0),
        ..Default::default()
    };
    for (_, b) in bullets(&sec.lines) {
        flows.raw.push(b.clone());
        // Orders: "(a) P1, P2, P3, P4, P5 and (b) P1, P3, P2, P4, P5"
        let mut rest = b.as_str();
        while let Some(i) = rest.find("(") {
            let after = &rest[i + 1..];
            // marker like (a) / (b)
            let is_marker = after.len() > 1
                && after.chars().next().map(|c| c.is_ascii_lowercase()).unwrap_or(false)
                && after[1..].starts_with(')');
            if is_marker {
                let tail = &after[2..];
                let mut order = Vec::new();
                let mut t = tail;
                loop {
                    let Some(pi) = t.find('P') else { break };
                    let digits: String =
                        t[pi + 1..].chars().take_while(|c| c.is_ascii_digit()).collect();
                    if digits.is_empty() {
                        break;
                    }
                    let gap = &t[..pi];
                    // Stop at the first P that isn't part of the comma list.
                    if !order.is_empty() && !gap.trim().trim_start_matches(',').trim().is_empty()
                        && !gap.contains(',')
                    {
                        break;
                    }
                    order.push(digits.parse::<u32>().unwrap());
                    t = &t[pi + 1 + digits.len()..];
                }
                if order.len() >= 2 {
                    flows.orders.push(order);
                }
                rest = tail;
            } else {
                rest = &rest[i + 1..];
            }
        }
    }
    flows
}

/// Parses the whole spec. Collects every error rather than stopping at the
/// first (the modeller fixes a batch per run, like a compiler).
pub fn parse_spec(text: &str) -> (Spec, Vec<SpecError>) {
    let mut errors = Vec::new();
    let mut spec = Spec::default();
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("# Model Specification —") {
            spec.system_name = rest.trim().to_string();
            break;
        }
        if let Some(rest) = line.strip_prefix("# Model Specification -") {
            spec.system_name = rest.trim().to_string();
            break;
        }
    }
    for sec in sections(text) {
        match sec.number {
            2 => spec.requirements = parse_requirements(&sec, &mut errors),
            3 => spec.resources = parse_resources(&sec, &mut errors),
            4 => {
                let (i, o) = parse_boundary(&sec, &mut errors);
                spec.inputs = i;
                spec.outputs = o;
            }
            5 => spec.processes = parse_processes(&sec, &mut errors),
            6 => spec.flows = parse_flows(&sec),
            _ => {}
        }
    }
    (spec, errors)
}
