//! The SPEC_TEMPLATE.md parser: markdown sections §1..§8, std-only.
//!
//! Two modes (R22):
//! - **strict** (the default): the template's machine conventions A1–A13 —
//!   backticked canonical identifiers (A4), underscore-grouped or ungrouped
//!   numbers only (A5), the §2 `**Ids:**` field (A6), the §6 `**Orders:**`
//!   field (A8), the Satisfies claim segment (A11) — are relied on and
//!   enforced.
//! - **lenient** (`--lenient`, migration aid only): the EXP-15 fuzzy
//!   conventions survive — space-grouped thousands, prose remap notes, and
//!   downstream fuzzy name resolution (F-062).

use crate::model::*;

/// Magnitude units (R7 base units plus money minor units, R19): quantities
/// in these units ride as const parameters; anything else counts items.
pub const MAG_UNITS: &[&str] = &["g", "J", "ms", "mm", "mK", "mA", "p", "ec"];

/// True for a magnitude unit token.
pub fn is_mag_unit(u: &str) -> bool {
    MAG_UNITS.contains(&u)
}

/// Lexes the leading number from `s`, returning the value and the rest.
/// Underscore grouping is always absorbed ("550_000 J" → (550000, " J"));
/// space grouping ("550 000") only in lenient mode (A5).
pub fn lex_number(s: &str, strict: bool) -> Option<(u64, &str)> {
    let s = s.trim_start();
    let first_len = s.chars().take_while(|c| c.is_ascii_digit()).count();
    if first_len == 0 {
        return None;
    }
    let mut value: u64 = s[..first_len].parse().ok()?;
    let mut rest = &s[first_len..];
    // Absorb further groups: "_000" always, " 000" (exactly 3 digits) only
    // in lenient mode.
    loop {
        let r = rest;
        let after = match r.chars().next() {
            Some('_') => &r[1..],
            Some(' ') if !strict => &r[1..],
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
pub fn parse_quantities(s: &str, strict: bool) -> Vec<Qty> {
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
        let (value, after) = match lex_number(&rest[idx..], strict) {
            Some(v) => v,
            None => break,
        };
        let mut words = after.split_whitespace();
        let unit = words
            .next()
            .unwrap_or("")
            .trim_matches(|c: char| !c.is_ascii_alphanumeric());
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

/// Every backticked token in `s`, in order: `` 1 `cold` `Water` `` →
/// ["cold", "Water"]. The canonical-identifier mechanism of A4.
pub fn backticks(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = s;
    while let Some(i) = rest.find('`') {
        let after = &rest[i + 1..];
        match after.find('`') {
            Some(j) => {
                let tok = after[..j].trim();
                if !tok.is_empty() {
                    out.push(tok.to_string());
                }
                rest = &after[j + 1..];
            }
            None => break,
        }
    }
    out
}

/// A9's sentence rule: an item list runs to the first top-level ". " (or a
/// trailing "."); explanatory prose lives in its own sentence after it.
/// Returns the item-list half.
pub fn item_list_part(s: &str) -> &str {
    let bytes = s.as_bytes();
    let mut depth = 0usize;
    let mut in_tick = false;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'`' => in_tick = !in_tick,
            b'(' if !in_tick => depth += 1,
            b')' if !in_tick => depth = depth.saturating_sub(1),
            b'.' if !in_tick && depth == 0 => {
                // a sentence boundary is ". " or a final "."
                if i + 1 == bytes.len() || bytes[i + 1] == b' ' {
                    // but not a decimal / "e.g." style abbreviation
                    let prev_digit =
                        i > 0 && (bytes[i - 1] as char).is_ascii_digit() && i + 1 < bytes.len();
                    if !prev_digit || i + 1 == bytes.len() {
                        return s[..i].trim_end();
                    }
                }
            }
            _ => {}
        }
        i += 1;
    }
    s.trim_end()
}

pub(crate) struct Section {
    pub number: u32,
    pub lines: Vec<(usize, String)>, // (1-based line no, text)
}

pub(crate) fn sections(text: &str) -> Vec<Section> {
    let mut secs: Vec<Section> = Vec::new();
    for (i, raw) in text.lines().enumerate() {
        if let Some(rest) = raw.strip_prefix("## ") {
            let num: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(n) = num.parse::<u32>() {
                secs.push(Section {
                    number: n,
                    lines: Vec::new(),
                });
                continue;
            }
        }
        if let Some(cur) = secs.last_mut() {
            cur.lines.push((i + 1, raw.to_string()));
        }
    }
    secs
}

/// Joins `- ...` bullets with their (indented or wrapped) continuation lines.
fn bullets(lines: &[(usize, String)]) -> Vec<(usize, String)> {
    let mut out: Vec<(usize, String)> = Vec::new();
    for (no, l) in lines {
        let t = l.trim_start();
        if t.starts_with("- ") || t.starts_with("* ") {
            out.push((*no, t[2..].trim().to_string()));
        } else if !t.is_empty() && !t.starts_with('#') && !t.starts_with('>') && !t.starts_with('|')
        {
            // Indented or margin-wrapped prose continues the previous bullet.
            if let Some(last) = out.last_mut() {
                last.1.push(' ');
                last.1.push_str(t);
            }
        }
    }
    out
}

/// Parses a markdown pipe-table's body rows into cell vectors — only the
/// FIRST contiguous table in the slice (a later ownership/subsystem table in
/// the same section is prose to this parser).
fn first_table_rows(lines: &[(usize, String)]) -> Vec<(usize, Vec<String>)> {
    let mut rows = Vec::new();
    let mut seen_table = false;
    for (no, l) in lines {
        let t = l.trim();
        if !t.starts_with('|') {
            if seen_table && !t.is_empty() {
                break; // first table ended
            }
            continue;
        }
        seen_table = true;
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

/// Lenient only: the pre-A6 prose remap note ("… are implemented as
/// **REQ-006..REQ-009** in spec order", F-053).
fn parse_req_remap(lines: &[(usize, String)]) -> Option<u32> {
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

/// The §2 "**Ids:** REQ-0NN–REQ-0MM, …" field (A6).
fn parse_ids_field(lines: &[(usize, String)]) -> Option<IdsField> {
    for (no, l) in lines {
        let t = l.trim_start().trim_start_matches("- ").trim_start();
        if let Some(rest) = t.strip_prefix("**Ids:**") {
            let mut ids = Vec::new();
            let mut r = rest;
            while let Some(i) = r.find("REQ-") {
                let digits: String = r[i + 4..]
                    .chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect();
                if let Ok(n) = digits.parse::<u32>() {
                    ids.push(n);
                }
                r = &r[i + 4..];
            }
            let first = *ids.first()?;
            let last = *ids.get(1).unwrap_or(&first);
            return Some(IdsField {
                first,
                last,
                line: *no,
            });
        }
    }
    None
}

fn parse_requirements(
    sec: &Section,
    strict: bool,
    errors: &mut Vec<SpecError>,
) -> (Vec<Requirement>, Option<IdsField>) {
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
    let ids_field = parse_ids_field(&sec.lines);
    if !strict {
        // Pre-A6 prose remap note (F-053), lenient only.
        if let Some(base) = parse_req_remap(&sec.lines) {
            for (i, r) in reqs.iter_mut().enumerate() {
                r.impl_id = base + i as u32;
            }
        }
    }
    (reqs, ids_field)
}

fn parse_kind(cell: &str) -> Kind {
    let c = cell.to_lowercase();
    if c.starts_with("discrete") {
        Kind::Discrete
    } else if c.starts_with("continuous") {
        Kind::Continuous {
            waste: c.contains("waste"),
        }
    } else if c.starts_with("waste") {
        Kind::Continuous { waste: true }
    } else if c.starts_with("reusable") {
        let note = c
            .find('(')
            .map(|i| c[i..].trim_matches(|ch| ch == '(' || ch == ')').to_string());
        Kind::Reusable { note }
    } else if c.starts_with("product") {
        Kind::Product
    } else {
        Kind::Other(cell.to_string())
    }
}

/// Strict states (A4/A10): each backticked token in the States cell is a
/// state; a '(' immediately after it carries its quantities, one term per
/// const parameter. Everything unbackticked is narrative.
fn parse_states_strict(cell: &str) -> Vec<ResState> {
    let mut out: Vec<ResState> = Vec::new();
    let mut rest = cell;
    while let Some(i) = rest.find('`') {
        let after = &rest[i + 1..];
        let Some(j) = after.find('`') else { break };
        let name = after[..j].trim().to_string();
        let mut tail = &after[j + 1..];
        let mut quantities = Vec::new();
        let t = tail.trim_start();
        if let Some(stripped) = t.strip_prefix('(') {
            if let Some(close) = stripped.find(')') {
                quantities = parse_quantities(&stripped[..close], true);
                tail = &stripped[close + 1..];
            }
        }
        if !name.is_empty() && !out.iter().any(|s: &ResState| s.name == name) {
            out.push(ResState { name, quantities });
        }
        rest = tail;
    }
    out
}

/// Lenient states: the pre-A4 "cold → boiling(1500 g) → tea" chain.
fn parse_states_lenient(cell: &str) -> Vec<ResState> {
    if cell.trim() == "—" || cell.trim() == "-" || cell.trim().is_empty() {
        return Vec::new();
    }
    cell.split('→')
        .map(|part| {
            let part = part.trim();
            let (name, quantities) = match part.find('(') {
                Some(i) => {
                    let inner = part[i..].trim_matches(|c| c == '(' || c == ')');
                    (
                        part[..i].trim().to_string(),
                        parse_quantities(inner, false),
                    )
                }
                None => (part.to_string(), Vec::new()),
            };
            ResState { name, quantities }
        })
        .filter(|s| !s.name.is_empty())
        .collect()
}

fn parse_resources(sec: &Section, strict: bool, errors: &mut Vec<SpecError>) -> Vec<Resource> {
    let mut out = Vec::new();
    for (no, cells) in first_table_rows(&sec.lines) {
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
        let idents = backticks(&cells[0]);
        if strict && idents.is_empty() {
            errors.push(SpecError {
                line: no,
                section: "§3".into(),
                message: format!(
                    "resource cell '{}' has no backticked canonical identifier — each Resource cell opens with one (template §3, A4); §4/§5/§6 resolve names only through these identifiers",
                    cells[0]
                ),
            });
        }
        let states = if strict {
            parse_states_strict(&cells[4])
        } else {
            parse_states_lenient(&cells[4])
        };
        out.push(Resource {
            name: cells[0].clone(),
            idents,
            kind: parse_kind(&cells[1]),
            characteristics: cells[2].clone(),
            quantity_raw: cells[3].clone(),
            states,
            line: no,
        });
    }
    out
}

fn parse_boundary(
    sec: &Section,
    errors: &mut Vec<SpecError>,
) -> (Vec<BoundaryRow>, Vec<BoundaryRow>) {
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

/// The clause's trailing conservation mark (A1): the last parenthetical
/// whose first word is "assert" or "structural". Returns (mark, note,
/// clause-without-the-marker).
fn split_mark(clause: &str) -> (BalanceMark, String, String) {
    // walk parentheticals from the end
    let mut best: Option<(usize, usize)> = None;
    let mut depth = 0usize;
    let mut open = 0usize;
    for (i, c) in clause.char_indices() {
        match c {
            '(' => {
                if depth == 0 {
                    open = i;
                }
                depth += 1;
            }
            ')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    best = Some((open, i));
                }
            }
            _ => {}
        }
    }
    if let Some((a, b)) = best {
        let inner = clause[a + 1..b].trim();
        let low = inner.to_lowercase();
        let mark = if low.starts_with("assert") {
            Some(BalanceMark::Assert)
        } else if low.starts_with("structural") {
            Some(BalanceMark::Structural)
        } else {
            None
        };
        if let Some(mark) = mark {
            let key_len = if mark == BalanceMark::Assert { 6 } else { 10 };
            let note = inner[key_len..]
                .trim_start_matches([',', ' ', '—', '-', ':'])
                .trim()
                .to_string();
            let without = format!("{}{}", &clause[..a], &clause[b + 1..]);
            return (mark, note, without.trim().to_string());
        }
    }
    (BalanceMark::Unmarked, String::new(), clause.to_string())
}

/// Evaluates one side of a balance equation: terms split on top-level '+',
/// each term the PRODUCT of its numbers ("25 × 500" → 12500; "2000 p × 117"
/// → 234000). A term with no number is symbolic.
fn eval_side(raw: &str, strict: bool) -> (Vec<u64>, bool) {
    let mut vals = Vec::new();
    let mut symbolic = false;
    for term in split_top_level(raw, &['+']) {
        let mut product: Option<u64> = None;
        let mut rest = term.as_str();
        loop {
            let Some(idx) = rest.find(|c: char| c.is_ascii_digit()) else {
                break;
            };
            let attached = rest[..idx]
                .chars()
                .last()
                .map(|c| c.is_ascii_alphanumeric() || c == '-')
                .unwrap_or(false);
            if attached {
                rest = &rest[idx + 1..];
                continue;
            }
            match lex_number(&rest[idx..], strict) {
                Some((v, after)) => {
                    product = Some(product.map(|p| p.saturating_mul(v)).unwrap_or(v));
                    rest = after;
                }
                None => break,
            }
        }
        match product {
            Some(v) => vals.push(v),
            None => {
                if !term.trim().is_empty() {
                    symbolic = true;
                }
            }
        }
    }
    (vals, symbolic)
}

fn parse_balance_clause(
    clause: &str,
    line: usize,
    pid: u32,
    strict: bool,
    errors: &mut Vec<SpecError>,
) -> (Option<Balance>, Option<TimeDraw>) {
    let clause = clause.trim().trim_end_matches('.').trim();
    if clause.is_empty() {
        return (None, None);
    }
    let (mark, note, cleaned) = split_mark(clause);
    // Time clause: routes to a History (R16) — "time 30_000 ms → History" or
    // "time → `h_a`".
    let is_time = cleaned.contains("→ History")
        || cleaned.contains("-> History")
        || cleaned.contains("→ `h")
        || cleaned.contains("-> `h");
    if is_time && !cleaned.contains('=') {
        let head = cleaned
            .split('→')
            .next()
            .unwrap_or("")
            .split("->")
            .next()
            .unwrap_or("");
        let ms = parse_quantities(head, strict).first().map(|q| q.value);
        return (None, Some(TimeDraw { ms, line }));
    }
    let Some(eq) = cleaned.find('=') else {
        // A marked clause stating where its check lives ("(assert, kit
        // draw)") is the spec's own statement — accepted with a note.
        if mark == BalanceMark::Unmarked {
            errors.push(SpecError {
                line,
                section: format!("§5 P{pid}"),
                message: format!(
                    "balance clause has neither '=' nor '→ History': '{clause}' (template shape: '<dimension> a + b = c + d (assert|structural)')"
                ),
            });
        }
        return (
            Some(Balance {
                dimension: cleaned
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_string(),
                lhs: Vec::new(),
                rhs: Vec::new(),
                symbolic: true,
                mark,
                note,
                raw: clause.to_string(),
                line,
            }),
            if is_time { Some(TimeDraw { ms: None, line }) } else { None },
        );
    };
    let lhs_raw = strip_running_total(&cleaned[..eq]);
    let rhs_raw = strip_running_total(&cleaned[eq + 1..]);
    let dimension = lhs_raw.split_whitespace().next().unwrap_or("").to_string();
    let (lhs, sym_l) = eval_side(&lhs_raw, strict);
    let (rhs, sym_r) = eval_side(&rhs_raw, strict);
    if lhs.is_empty() && rhs.is_empty() && !sym_l && !sym_r {
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
            symbolic: sym_l || sym_r,
            mark,
            note,
            raw: clause.to_string(),
            line,
        }),
        if is_time { Some(TimeDraw { ms: None, line }) } else { None },
    )
}

fn parse_processes(sec: &Section, strict: bool, errors: &mut Vec<SpecError>) -> Vec<Process> {
    let mut procs: Vec<Process> = Vec::new();
    // Split into blocks on "### P<k>." headings (other ### headings — branch
    // banners, "The join" — are narrative), then bullet-parse each.
    let mut blocks: Vec<(u32, String, usize, Vec<(usize, String)>)> = Vec::new();
    for (no, l) in &sec.lines {
        let t = l.trim();
        if let Some(rest) = t.strip_prefix("### P") {
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(id) = digits.parse::<u32>() {
                let title = rest[digits.len()..]
                    .trim_start_matches('.')
                    .trim()
                    .to_string();
                blocks.push((id, title, *no, Vec::new()));
                continue;
            }
        }
        if t.starts_with("### ") {
            continue; // narrative sub-heading
        }
        if let Some(b) = blocks.last_mut() {
            b.3.push((*no, l.clone()));
        }
    }
    for (id, title, hline, lines) in blocks {
        // The title may carry a crate marker or emphasis ("Patch the tube —
        // **fallible (R17)**"): keep the head before an em-dash for naming.
        let short_title = title
            .split('—')
            .next()
            .unwrap_or(&title)
            .trim()
            .trim_end_matches(|c: char| c == '*' || c.is_whitespace())
            .to_string();
        let mut p = Process {
            id,
            title: short_title,
            line: hline,
            ..Default::default()
        };
        for (no, b) in bullets(&lines) {
            let Some(colon) = b.find(":**") else { continue };
            let field = b[..colon].trim_start_matches("**").to_lowercase();
            let value = b[colon + 3..].trim().to_string();
            match field.as_str() {
                f if f.starts_with("actor") => {
                    p.actors_raw = value.clone();
                    let low = value.to_lowercase();
                    if low.contains("no person") || low.contains("no operator") {
                        p.no_person = true;
                    }
                    if let Some(i) = low.find("draws") {
                        if let Some(q) = parse_quantities(&value[i..], strict).first() {
                            p.draws_ms = Some(q.value);
                        }
                    }
                }
                "consumes" => {
                    p.consumes_raw = value.trim_end_matches('.').trim().to_string();
                    p.consumes_line = no;
                }
                "produces" => {
                    p.produces_raw = value.trim_end_matches('.').trim().to_string();
                    p.produces_line = no;
                }
                f if f.starts_with("produces") => {
                    // R17 variants: "Produces (Ok):", "Produces (Fail):"
                    p.produces_variants.push((
                        f.to_string(),
                        value.trim_end_matches('.').trim().to_string(),
                        no,
                    ));
                    if p.produces_line == 0 {
                        p.produces_line = no;
                    }
                }
                "waste" => {
                    p.waste_raw = Some(value.trim_end_matches('.').trim().to_string());
                    p.waste_line = no;
                }
                "waste routing" => {
                    p.waste_routing = Some(value.trim_end_matches('.').trim().to_string());
                    p.waste_routing_line = no;
                }
                f if f.starts_with("balances") => {
                    p.balances_line = no;
                    for clause in split_top_level(&value, &[';']) {
                        let (bal, td) = parse_balance_clause(&clause, no, id, strict, errors);
                        if let Some(b) = bal {
                            p.balances.push(b);
                        }
                        if let Some(t) = td {
                            p.time_draw = Some(t);
                        }
                    }
                }
                "satisfies" => {
                    p.satisfies_line = no;
                    // A11: the claim segment is the text before the first full
                    // stop — a comma-separated list of REQ ids (each with an
                    // optional parenthesised note) or the single mark '—',
                    // which claims nothing. Anything after the full stop is
                    // plain prose: REQ ids there are informative, never
                    // claims. Lenient mode keeps the pre-A11 whole-line scrape.
                    let claim = if strict {
                        item_list_part(&value)
                    } else {
                        value.as_str()
                    };
                    let dash_claim = claim.trim_start().starts_with('—');
                    if strict && dash_claim && claim.contains("REQ-") {
                        errors.push(SpecError {
                            line: no,
                            section: format!("§5 P{id}"),
                            message: format!(
                                "the Satisfies claim segment '{}' starts with '—' but still names a REQ id: '—' claims nothing, so move the informative id after the full stop ('**Satisfies:** —. Enables REQ-011 (P6's bound).') — a REQ id scraped from a '—' segment seeded false traceability tags in generated scaffolds (A11, F-066)",
                                claim.trim()
                            ),
                        });
                    }
                    let mut rest = if strict && dash_claim { "" } else { claim };
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

fn parse_flows(sec: &Section, strict: bool) -> Flows {
    let mut flows = Flows {
        line: sec.lines.first().map(|(n, _)| *n).unwrap_or(0),
        ..Default::default()
    };
    for (no, b) in bullets(&sec.lines) {
        flows.raw.push(b.clone());
        let is_orders = b.trim_start().starts_with("**Orders:**");
        if is_orders {
            flows.has_orders_field = true;
            flows.orders_line = no;
            let value = b.trim_start().trim_start_matches("**Orders:**").trim();
            for seg in split_top_level(value, &[';']) {
                let mut order = Vec::new();
                let mut t = seg.as_str();
                loop {
                    let Some(pi) = t.find('P') else { break };
                    let digits: String = t[pi + 1..]
                        .chars()
                        .take_while(|c| c.is_ascii_digit())
                        .collect();
                    if digits.is_empty() {
                        t = &t[pi + 1..];
                        continue;
                    }
                    order.push(digits.parse::<u32>().unwrap());
                    t = &t[pi + 1 + digits.len()..];
                }
                if !order.is_empty() {
                    flows.orders.push(order);
                }
            }
            continue;
        }
        if strict {
            continue; // strict mode reads orders only from the Orders field (A8)
        }
        // Lenient: the pre-A8 "(a) P1, P2, P3" marker parsing.
        let mut rest = b.as_str();
        while let Some(i) = rest.find('(') {
            let after = &rest[i + 1..];
            let is_marker = after.len() > 1
                && after
                    .chars()
                    .next()
                    .map(|c| c.is_ascii_lowercase())
                    .unwrap_or(false)
                && after[1..].starts_with(')');
            if is_marker {
                let tail = &after[2..];
                let mut order = Vec::new();
                let mut t = tail;
                loop {
                    let Some(pi) = t.find('P') else { break };
                    let digits: String = t[pi + 1..]
                        .chars()
                        .take_while(|c| c.is_ascii_digit())
                        .collect();
                    if digits.is_empty() {
                        break;
                    }
                    let gap = &t[..pi];
                    if !order.is_empty()
                        && !gap.trim().trim_start_matches(',').trim().is_empty()
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

/// A5: scans §2–§6 content for space-grouped thousands ("550 000"), which
/// strict number lexing deliberately does not absorb.
fn scan_space_grouped(secs: &[Section], errors: &mut Vec<SpecError>) {
    for sec in secs {
        if !(2..=6).contains(&sec.number) {
            continue;
        }
        for (no, l) in &sec.lines {
            let b = l.as_bytes();
            let mut i = 0;
            while i + 4 < b.len() {
                if (b[i] as char).is_ascii_digit()
                    && b[i + 1] == b' '
                    && (b[i + 2] as char).is_ascii_digit()
                    && (b[i + 3] as char).is_ascii_digit()
                    && (b[i + 4] as char).is_ascii_digit()
                    && (i + 5 == b.len() || !(b[i + 5] as char).is_ascii_digit())
                {
                    errors.push(SpecError {
                        line: *no,
                        section: format!("§{}", sec.number),
                        message: format!(
                            "space-grouped number in '{}': write underscore grouping (550_000) or ungrouped (550000) — machine parsing takes no lexing heuristics (template §3, A5)",
                            l.trim()
                        ),
                    });
                    break; // one report per line is enough
                }
                i += 1;
            }
        }
    }
}

/// Parses the whole spec. Collects every error rather than stopping at the
/// first (the modeller fixes a batch per run, like a compiler).
pub fn parse_spec(text: &str, strict: bool) -> (Spec, Vec<SpecError>) {
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
    let secs = sections(text);
    if strict {
        scan_space_grouped(&secs, &mut errors);
    }
    for sec in &secs {
        match sec.number {
            2 => {
                spec.section2_line = sec.lines.first().map(|(n, _)| *n).unwrap_or(0);
                let (reqs, ids) = parse_requirements(sec, strict, &mut errors);
                spec.requirements = reqs;
                spec.ids_field = ids;
            }
            3 => spec.resources = parse_resources(sec, strict, &mut errors),
            4 => {
                let (i, o) = parse_boundary(sec, &mut errors);
                spec.inputs = i;
                spec.outputs = o;
            }
            5 => spec.processes = parse_processes(sec, strict, &mut errors),
            6 => spec.flows = parse_flows(sec, strict),
            _ => {}
        }
    }
    (spec, errors)
}
