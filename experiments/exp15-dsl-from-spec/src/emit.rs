//! Resolution, validation and emission: turns a parsed [`Spec`] into the
//! generated model crate, recording every under-determined decision as a
//! numbered SPEC-HOLE.
//!
//! ## The hole convention (documented contract)
//!
//! Where the spec under-determines an implementation decision, the generator
//! emits, at the nearest item site:
//! ```text
//! // ── SPEC-HOLE U-03 ────────────────────────────────────────────────
//! // <what the spec does not determine, with the SPEC.md §/line it stems from>
//! #[cfg(feature = "deny-holes")]
//! compile_error!("SPEC-HOLE U-03: <one-line summary>");
//! ```
//! A plain `cargo build` compiles everything the spec determines;
//! `cargo build --features deny-holes` turns every hole into a compile
//! error, so the full gap list is enumerable by the compiler itself.

use crate::model::*;
use crate::parse::{parse_quantities, split_top_level, strip_parens};
use std::collections::BTreeMap;
use std::fmt::Write as _;

const STOPWORDS: &[&str] = &[
    "the", "a", "an", "of", "and", "with", "to", "must", "be", "is", "are", "all", "only",
    "before", "exactly", "its", "from", "for", "per", "each", "one", "at", "in", "into",
];

/// Lowercase significant words of a phrase (plural-stripped).
fn words(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '-')
        .flat_map(|w| w.split('-'))
        .map(|w| w.trim_end_matches('s').to_string())
        .filter(|w| !w.is_empty() && !w.chars().next().unwrap().is_ascii_digit())
        .filter(|w| !STOPWORDS.contains(&w.trim_end_matches('s')))
        .collect()
}

fn camel(s: &str) -> String {
    s.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .filter(|w| !matches!(w.to_lowercase().as_str(), "the" | "a" | "an"))
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

fn snake(s: &str) -> String {
    // camel-aware: "FoodWasteBin" → food_waste_bin, "Fill the kettle" → fill_kettle
    let mut spaced = String::new();
    let mut prev_lower = false;
    for c in s.chars() {
        if c.is_ascii_uppercase() && prev_lower {
            spaced.push(' ');
        }
        prev_lower = c.is_ascii_lowercase() || c.is_ascii_digit();
        spaced.push(c);
    }
    spaced
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .filter(|w| !matches!(w.to_lowercase().as_str(), "the" | "a" | "an"))
        .map(|w| w.to_lowercase())
        .collect::<Vec<_>>()
        .join("_")
}

/// 550000 → "550_000" (Rust literal with underscore grouping).
pub fn lit(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push('_');
        }
        out.push(c);
    }
    out
}

fn unit_name(tok: &str) -> &'static str {
    match tok {
        "g" => "grams",
        "J" => "joules",
        "ms" => "milliseconds",
        "mm" => "millimetres",
        _ => "units",
    }
}

fn const_param_name(tok: &str) -> &'static str {
    match tok {
        "g" => "MASS_G",
        "J" => "ENERGY_J",
        "ms" => "TIME_MS",
        _ => "MAGNITUDE",
    }
}

/// What a resolved §3 (resource, state) becomes in code.
#[derive(Debug, Clone, PartialEq)]
pub enum Role {
    /// `container_resource!`: `n_extra` const params ride before the macro's
    /// own magnitude slot (last-listed quantity).
    Container { extra_units: Vec<String>, slot_unit: String },
    /// `consumable_resource!`, optionally holding real discrete contents.
    Consumable { held: Option<(u64, String)>, tripwire: bool, mass_g: Option<u64> },
    Reusable,
    /// Lives in model-core (`Person`, `History`): nothing to emit.
    Common,
}

#[derive(Debug, Clone)]
pub struct TypeEntry {
    pub ident: String,
    pub resource: String,
    pub state: Option<String>,
    /// words that must all appear in a phrase for this entry to match
    pub required: Vec<String>,
    /// resource stems: at least one must appear (containment either way)
    pub stems: Vec<String>,
    pub quantities: Vec<Qty>,
    pub role: Role,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct Hole {
    pub id: u32,
    pub summary: String,
    pub detail: String,
}

/// A resolved item in a process's consumes/produces/waste list.
#[derive(Debug, Clone)]
pub struct Item {
    pub ident: String,
    /// const arguments, in declaration order
    pub consts: Vec<u64>,
    /// discrete count (1 for continuous)
    pub count: u64,
    pub discrete: bool,
}

#[derive(Debug, Clone)]
pub struct WasteOut {
    pub item: Option<Item>,
    pub destination: String,
    /// snake name of the §4 sink object this routes to (resolved)
    pub sink_var: Option<String>,
    /// true when the same item already appears under Produces (CS-1 P4)
    pub duplicate_of_produce: bool,
}

#[derive(Debug, Clone)]
pub struct ProcPlan {
    pub fn_name: String,
    pub p: Process,
    pub person: bool,
    pub reusable_params: Vec<String>,  // base reusable idents moved in/out
    pub reusable_returns: Vec<String>, // base idents returned (e.g. kettle back empty)
    pub consumes: Vec<Item>,
    pub produces: Vec<Item>,
    pub waste: Vec<WasteOut>,
    /// Some(count, item, supplier-needed) when consuming N discrete items
    pub supply_n: Option<(u64, String)>,
    /// Some((space_left, cap, item, count)) for the bin-disposal shape (P5)
    pub bin_disposal: Option<BinDisposal>,
}

#[derive(Debug, Clone)]
pub struct BinDisposal {
    pub bin_ident: String,
    pub cap: u64,
    pub count: u64,
    pub item: String,
    pub waste_ident: String,
    pub waste_mass: u64,
    pub sink_ident: String,
}

#[derive(Debug, Clone)]
pub struct Sink {
    pub ident: String,
    /// words of the §4 phrase this sink was named from (for destination matching)
    pub name_words: Vec<String>,
    /// (consumed ident, number of const params on it)
    pub consumes: Vec<(String, usize)>,
    pub placeholder_note: String,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct BoundedConsumer {
    pub ident: String,
    pub name_words: Vec<String>,
    pub item: String,
    pub capacity: u64,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct SupplierPlan {
    pub ident: String,
    pub item: String,
    pub capacity: u64,
    pub fill_fn: String,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct DrawSource {
    pub obj_ident: String,
    pub fn_name: String,
    pub item: String,
    pub line: usize,
}

pub struct Generator {
    pub spec: Spec,
    pub lexicon: Vec<TypeEntry>,
    pub errors: Vec<SpecError>,
    pub warnings: Vec<String>,
    pub holes: Vec<Hole>,
    pub sinks: Vec<Sink>,
    pub consumers: Vec<BoundedConsumer>,
    pub suppliers: Vec<SupplierPlan>,
    pub draws: Vec<DrawSource>,
    pub plans: Vec<ProcPlan>,
    pub person_budget: u64,
    /// containers synthesized from §5 waste lines with no §3 row
    pub synthesized: Vec<TypeEntry>,
}

impl Generator {
    fn hole(&mut self, summary: &str, detail: &str) -> u32 {
        let id = self.holes.len() as u32 + 1;
        self.holes.push(Hole {
            id,
            summary: summary.to_string(),
            detail: detail.to_string(),
        });
        id
    }

    fn err(&mut self, line: usize, section: &str, message: String) {
        self.errors.push(SpecError {
            line,
            section: section.to_string(),
            message,
        });
    }

    /// Resolves a prose phrase to a lexicon entry (best word-overlap score;
    /// every `required` word must appear; at least one stem must match).
    pub fn resolve(&self, phrase: &str) -> Option<TypeEntry> {
        let ws = words(&strip_parens(phrase));
        let matches_word = |w: &str, k: &str| -> bool {
            w == k || (w.len() >= 3 && k.contains(w)) || (k.len() >= 3 && w.contains(k))
        };
        let mut best: Option<(usize, &TypeEntry)> = None;
        let mut tie = false;
        for e in self.lexicon.iter().chain(self.synthesized.iter()) {
            if !e.required.iter().all(|r| ws.iter().any(|w| matches_word(w, r))) {
                continue;
            }
            if !e.stems.is_empty() && !e.stems.iter().any(|s| ws.iter().any(|w| matches_word(w, s))) {
                continue;
            }
            let score = e
                .required
                .iter()
                .chain(e.stems.iter())
                .filter(|k| ws.iter().any(|w| matches_word(w, k)))
                .count()
                + e.required.len(); // prefer more-specific (state) entries

            match &best {
                Some((s, b)) if *s == score && b.ident != e.ident => tie = true,
                Some((s, _)) if *s >= score => {}
                _ => {
                    best = Some((score, e));
                    tie = false;
                }
            }
        }
        if tie {
            return None;
        }
        best.map(|(_, e)| e.clone())
    }

    pub fn new(spec: Spec) -> Generator {
        let mut g = Generator {
            spec,
            lexicon: Vec::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
            holes: Vec::new(),
            sinks: Vec::new(),
            consumers: Vec::new(),
            suppliers: Vec::new(),
            draws: Vec::new(),
            plans: Vec::new(),
            person_budget: 0,
            synthesized: Vec::new(),
        };
        g.build_lexicon();
        g.plan_boundary();
        g.plan_processes();
        g.validate();
        g
    }

    // ── §3 → lexicon ────────────────────────────────────────────────────

    fn build_lexicon(&mut self) {
        let resources = self.spec.resources.clone();
        // Cross-resource state-name collisions: the state with quantities
        // wins (it is the carrying vessel); the other becomes a note.
        let mut owned: BTreeMap<String, (usize, bool)> = BTreeMap::new(); // state word -> (res idx, has qty)
        for (i, r) in resources.iter().enumerate() {
            for s in &r.states {
                let key = s.name.to_lowercase();
                let has = !s.quantities.is_empty();
                match owned.get(&key) {
                    Some((_, true)) if !has => {}
                    Some((j, false)) if has => {
                        let loser = resources[*j].name.clone();
                        self.warnings.push(format!(
                            "§3: state '{}' appears on both '{}' and '{}'; the quantity-bearing row ('{}') owns the type — the other is treated as narrative",
                            s.name, loser, r.name, r.name
                        ));
                        owned.insert(key, (i, has));
                    }
                    Some(_) => {}
                    None => {
                        owned.insert(key, (i, has));
                    }
                }
            }
        }
        // Also treat subset-named states as collisions ("tea" vs "pot of tea").
        let keys: Vec<String> = owned.keys().cloned().collect();
        for k in &keys {
            for k2 in &keys {
                if k != k2 && k2.split(' ').any(|w| w == k) {
                    // k is a one-word subset of k2: the longer, quantity-bearing name owns it
                    let (i2, q2) = owned[k2];
                    let (_, q1) = owned[k];
                    if q2 && !q1 {
                        owned.insert(k.clone(), (i2, true));
                    }
                }
            }
        }

        for (i, r) in resources.iter().enumerate() {
            let res_words = words(&r.name);
            let res_last = res_words.last().cloned().unwrap_or_default();
            let low = r.name.to_lowercase();
            // model-core common resources
            if low == "person" {
                self.person_budget = parse_quantities(&r.quantity_raw)
                    .first()
                    .map(|q| q.value)
                    .unwrap_or(0);
                self.lexicon.push(TypeEntry {
                    ident: "Person".into(),
                    resource: low.clone(),
                    state: None,
                    required: vec!["person".into()],
                    stems: vec!["person".into()],
                    quantities: parse_quantities(&r.quantity_raw),
                    role: Role::Common,
                    line: r.line,
                });
                continue;
            }
            match &r.kind {
                Kind::Continuous { .. } if r.states.is_empty() => {
                    // single container type named after the resource
                    let unit = parse_quantities(&r.quantity_raw)
                        .into_iter()
                        .find(|q| !q.unit.is_empty())
                        .map(|q| q.unit)
                        .unwrap_or_default();
                    self.lexicon.push(TypeEntry {
                        ident: camel(&r.name),
                        resource: low.clone(),
                        state: None,
                        required: vec![],
                        stems: res_words.clone(),
                        quantities: parse_quantities(&r.quantity_raw),
                        role: Role::Container {
                            extra_units: vec![],
                            slot_unit: unit,
                        },
                        line: r.line,
                    });
                }
                Kind::Continuous { .. } => {
                    // one container per owned state
                    for s in &r.states {
                        let key = s.name.to_lowercase();
                        if owned.get(&key).map(|(j, _)| *j) != Some(i) {
                            continue; // another resource's vessel carries this state
                        }
                        let ident = state_ident(&s.name, &res_last);
                        let unit = s
                            .quantities
                            .last()
                            .map(|q| q.unit.clone())
                            .or_else(|| parse_quantities(&r.quantity_raw).first().map(|q| q.unit.clone()))
                            .unwrap_or_default();
                        self.lexicon.push(TypeEntry {
                            ident,
                            resource: low.clone(),
                            state: Some(key.clone()),
                            required: words(&s.name),
                            stems: res_words.clone(),
                            quantities: s.quantities.clone(),
                            role: Role::Container {
                                extra_units: vec![],
                                slot_unit: unit,
                            },
                            line: r.line,
                        });
                    }
                }
                Kind::Discrete => {
                    for s in &r.states {
                        let key = s.name.to_lowercase();
                        if owned.get(&key).map(|(j, _)| *j) != Some(i) {
                            continue;
                        }
                        let ident = state_ident(&s.name, &res_last);
                        // per-state mass from the characteristics column:
                        // "<state> mass N g"
                        let mass = find_state_mass(&r.characteristics, &s.name);
                        self.lexicon.push(TypeEntry {
                            ident,
                            resource: low.clone(),
                            state: Some(key.clone()),
                            required: words(&s.name),
                            stems: res_words.clone(),
                            quantities: s.quantities.clone(),
                            role: Role::Consumable {
                                held: None,
                                tripwire: true, // refined in plan_boundary (F-040 rule)
                                mass_g: mass,
                            },
                            line: r.line,
                        });
                    }
                    if r.states.is_empty() {
                        self.lexicon.push(TypeEntry {
                            ident: camel(&r.name),
                            resource: low.clone(),
                            state: None,
                            required: vec![],
                            stems: res_words.clone(),
                            quantities: parse_quantities(&r.quantity_raw),
                            role: Role::Consumable {
                                held: None,
                                tripwire: true,
                                mass_g: None,
                            },
                            line: r.line,
                        });
                    } else {
                        // bare-resource alias: unqualified mention = initial state
                        let init = state_ident(&r.states[0].name, &res_last);
                        self.lexicon.push(TypeEntry {
                            ident: init,
                            resource: low.clone(),
                            state: None,
                            required: vec![],
                            stems: res_words.clone(),
                            quantities: vec![],
                            role: Role::Common, // alias only: nothing extra to emit
                            line: r.line,
                        });
                    }
                }
                Kind::Reusable { .. } => {
                    // base type: the resource itself (its "empty" state)
                    self.lexicon.push(TypeEntry {
                        ident: camel(&r.name),
                        resource: low.clone(),
                        state: None,
                        required: vec![],
                        stems: res_words.clone(),
                        quantities: vec![],
                        role: Role::Reusable,
                        line: r.line,
                    });
                    for s in &r.states {
                        let key = s.name.to_lowercase();
                        if key == "empty" || s.name.trim() == "—" {
                            continue; // the base type is the empty state
                        }
                        if owned.get(&key).map(|(j, _)| *j) != Some(i) {
                            continue;
                        }
                        let ident = state_ident(&s.name, &res_last);
                        // held discrete contents? quantity like "3 bags"
                        let held = s.quantities.iter().find_map(|q| {
                            if q.unit.is_empty() || matches!(q.unit.as_str(), "g" | "J" | "ms" | "mm") {
                                return None;
                            }
                            self.lexicon
                                .iter()
                                .find(|e| {
                                    e.state.is_none()
                                        && e.stems.iter().any(|st| {
                                            st.contains(q.unit.trim_end_matches('s'))
                                                || q.unit.trim_end_matches('s').contains(st.as_str())
                                        })
                                })
                                .map(|e| (q.value, e.ident.clone()))
                        });
                        let numeric: Vec<&Qty> = s
                            .quantities
                            .iter()
                            .filter(|q| matches!(q.unit.as_str(), "g" | "J" | "ms" | "mm"))
                            .collect();
                        let role = if let Some(h) = held {
                            Role::Consumable {
                                held: Some(h),
                                tripwire: true,
                                mass_g: None,
                            }
                        } else if !numeric.is_empty() {
                            Role::Container {
                                extra_units: numeric[..numeric.len() - 1]
                                    .iter()
                                    .map(|q| q.unit.clone())
                                    .collect(),
                                slot_unit: numeric.last().unwrap().unit.clone(),
                            }
                        } else {
                            Role::Consumable {
                                held: None,
                                tripwire: true,
                                mass_g: None,
                            }
                        };
                        self.lexicon.push(TypeEntry {
                            ident,
                            resource: low.clone(),
                            state: Some(key.clone()),
                            required: words(&s.name),
                            stems: res_words.clone(),
                            quantities: s.quantities.clone(),
                            role,
                            line: r.line,
                        });
                    }
                }
                Kind::Product => {
                    self.lexicon.push(TypeEntry {
                        ident: camel(&r.name),
                        resource: low.clone(),
                        state: None,
                        required: vec![],
                        stems: res_words.clone(),
                        quantities: parse_quantities(&r.quantity_raw),
                        role: Role::Consumable {
                            held: None,
                            tripwire: true,
                            mass_g: None,
                        },
                        line: r.line,
                    });
                }
            }
        }
    }

    // ── §4 → boundary plans ─────────────────────────────────────────────

    fn plan_boundary(&mut self) {
        let inputs = self.spec.inputs.clone();
        for row in &inputs {
            let via_low = row.via.to_lowercase();
            if via_low.contains("draw process") {
                let obj = camel(split_top_level(&row.via, &[','])[0].as_str());
                let what = strip_parens(&row.what);
                let item = match self.resolve(&what) {
                    Some(e) => e.ident,
                    None => {
                        self.err(
                            row.line,
                            "§4",
                            format!("input '{}' does not match any §3 resource", row.what),
                        );
                        continue;
                    }
                };
                self.draws.push(DrawSource {
                    obj_ident: obj,
                    fn_name: format!("draw_{}", snake(&what)),
                    item,
                    line: row.line,
                });
            } else if via_low.contains("history") {
                // common machinery
            } else if let Some((cap, _)) = crate::parse::lex_number(row.capacity.trim()) {
                // discrete supplier with numeric capacity — only when the
                // supplied thing is a §3 DISCRETE resource (a "kitchen setup"
                // row of reusables also has a numeric capacity)
                let discrete = self
                    .resolve(&row.what)
                    .and_then(|e| {
                        self.spec
                            .resources
                            .iter()
                            .find(|r| r.name.to_lowercase() == e.resource)
                            .map(|r| matches!(r.kind, Kind::Discrete))
                    })
                    .unwrap_or(false);
                if !discrete {
                    continue; // reusables entering at flow start: constructors are emitted anyway
                }
                if let Some(e) = self.resolve(&row.what) {
                    let ident = camel(&row.via);
                    self.suppliers.push(SupplierPlan {
                        ident: ident.clone(),
                        item: e.ident.clone(),
                        capacity: cap,
                        fill_fn: format!("full_{}", snake(&row.via)),
                        line: row.line,
                    });
                    // F-040: items minted into a boundary supplier are kept
                    // by containers — no_tripwire.
                    let item = e.ident.clone();
                    for l in self.lexicon.iter_mut() {
                        if l.ident == item {
                            if let Role::Consumable { tripwire, .. } = &mut l.role {
                                *tripwire = false;
                            }
                        }
                    }
                } else {
                    self.err(
                        row.line,
                        "§4",
                        format!("input '{}' does not match any §3 resource", row.what),
                    );
                }
            } else {
                // "kitchen setup" style row: reusables enter here; the
                // reusable types already exist, constructors are emitted for
                // every reusable. Nothing further to plan.
            }
        }
        let outputs = self.spec.outputs.clone();
        for row in &outputs {
            let via_low = row.via.to_lowercase();
            if via_low.contains("history") {
                continue; // R16 common machinery
            }
            let hops: Vec<String> = row
                .via
                .split(" then ")
                .map(|h| strip_parens(h).trim().trim_end_matches(',').to_string())
                .collect();
            let caps: Vec<String> = split_top_level(&row.capacity, &[';']);
            for (i, hop) in hops.iter().enumerate() {
                let hop_clean = hop.trim_end_matches(',').trim();
                let cap_cell = caps.get(i).map(|s| s.as_str()).unwrap_or(&row.capacity);
                let bounded = crate::parse::lex_number(
                    cap_cell.trim_start_matches(|c: char| !c.is_ascii_digit()),
                );
                if cap_cell.contains("unbounded") || bounded.is_none() {
                    // Next = Self sink
                    let ident = camel(hop_clean);
                    let consumed: Vec<(String, usize)> = if i == 0 {
                        match self.resolve(&row.what) {
                            Some(e) => vec![(e.ident.clone(), n_consts(&e.role))],
                            None => vec![],
                        }
                    } else {
                        vec![] // later hop: wired from §5 waste lines afterwards
                    };
                    if !self.sinks.iter().any(|s| s.ident == ident) {
                        self.sinks.push(Sink {
                            ident,
                            name_words: words(hop_clean),
                            consumes: consumed,
                            placeholder_note: row.status.clone(),
                            line: row.line,
                        });
                    }
                } else if let Some((cap, _)) = bounded {
                    let ident = camel(hop_clean);
                    match self.resolve(&row.what) {
                        Some(e) => self.consumers.push(BoundedConsumer {
                            ident,
                            name_words: words(hop_clean),
                            item: e.ident.clone(),
                            capacity: cap,
                            line: row.line,
                        }),
                        None => self.err(
                            row.line,
                            "§4",
                            format!("output '{}' does not match any §3 resource", row.what),
                        ),
                    }
                }
            }
        }
    }

    // ── §5 → process plans ──────────────────────────────────────────────

    fn item_from_phrase(&mut self, phrase: &str, line: usize, section: &str) -> Option<Item> {
        let qties = parse_quantities(phrase);
        // trailing participial prose ("removed from the pot at the end of
        // brewing") is narrative, not an item
        let first_word = strip_parens(phrase);
        let first_word = first_word.split_whitespace().next().unwrap_or("");
        if qties.is_empty() && first_word.to_lowercase().ends_with("ed") {
            self.warnings.push(format!(
                "{section} (line {line}): narrative clause skipped in an item list: '{}'",
                phrase.trim()
            ));
            return None;
        }
        let entry = self.resolve(phrase).or_else(|| {
            // boundary fallback: "drawn from the grid" → §4 via words
            let ws = words(&strip_parens(phrase));
            let inputs = self.spec.inputs.clone();
            for row in &inputs {
                let via_ws = words(&row.via);
                if ws.iter().any(|w| via_ws.contains(w)) {
                    if let Some(e) = self.resolve(&strip_parens(&row.what)) {
                        return Some(e);
                    }
                }
            }
            None
        });
        let Some(entry) = entry else {
            let has_number = !qties.is_empty();
            if has_number {
                self.err(
                    line,
                    section,
                    format!(
                        "'{}' names no §3 resource or state: every consumed/produced item must be a §3 row (or a state listed there)",
                        phrase.trim()
                    ),
                );
            } else {
                self.warnings.push(format!(
                    "{section} (line {line}): unparsed prose left in an item list: '{}'",
                    phrase.trim()
                ));
            }
            return None;
        };
        match &entry.role {
            Role::Container { extra_units, slot_unit } => {
                // const args in declaration order: prefer the magnitudes
                // written at THIS mention; fall back to the §3 declaration
                let mut consts: Vec<u64> = qties
                    .iter()
                    .filter(|q| matches!(q.unit.as_str(), "g" | "J" | "ms" | "mm"))
                    .map(|q| q.value)
                    .collect();
                if consts.is_empty() {
                    consts = entry
                        .quantities
                        .iter()
                        .filter(|q| matches!(q.unit.as_str(), "g" | "J" | "ms" | "mm"))
                        .map(|q| q.value)
                        .collect();
                }
                let want = extra_units.len() + usize::from(!slot_unit.is_empty());
                if want > 0 && consts.len() != want {
                    self.warnings.push(format!(
                        "{section} (line {line}): '{}' resolved to {} which carries {} magnitude(s), but {} were stated here",
                        phrase.trim(),
                        entry.ident,
                        want,
                        consts.len()
                    ));
                }
                Some(Item {
                    ident: entry.ident.clone(),
                    consts,
                    count: 1,
                    discrete: false,
                })
            }
            Role::Consumable { .. } | Role::Common if entry.ident == "Person" => None,
            Role::Consumable { .. } | Role::Common => {
                let mut count = qties
                    .iter()
                    .find(|q| !matches!(q.unit.as_str(), "g" | "J" | "ms" | "mm") || q.unit.is_empty())
                    .map(|q| q.value)
                    .or_else(|| qties.first().map(|q| q.value))
                    .unwrap_or(1);
                // "loaded pot (3 bags, …)": the 3 counts the HELD contents,
                // not three pots
                if let Role::Consumable { held: Some((h, _)), .. } = &entry.role {
                    if count == *h {
                        count = 1;
                    }
                }
                Some(Item {
                    ident: entry.ident.clone(),
                    consts: vec![],
                    count,
                    discrete: true,
                })
            }
            Role::Reusable => Some(Item {
                ident: entry.ident.clone(),
                consts: vec![],
                count: 1,
                discrete: false,
            }),
        }
    }

    fn plan_processes(&mut self) {
        let procs = self.spec.processes.clone();
        for p in &procs {
            let section = format!("§5 P{}", p.id);
            let fn_name = snake(&p.title);
            let mut plan = ProcPlan {
                fn_name,
                p: p.clone(),
                person: !p.no_person && p.actors_raw.to_lowercase().contains("person"),
                reusable_params: Vec::new(),
                reusable_returns: Vec::new(),
                consumes: Vec::new(),
                produces: Vec::new(),
                waste: Vec::new(),
                supply_n: None,
                bin_disposal: None,
            };

            // -- the P5 shape: consuming a bounded consumer's kept contents
            let consumes_low = p.consumes_raw.to_lowercase();
            let bin = self
                .consumers
                .iter()
                .find(|c| {
                    c.name_words
                        .iter()
                        .any(|w| consumes_low.contains(w.as_str()) || p.actors_raw.to_lowercase().contains(w.as_str()))
                })
                .cloned();
            if consumes_low.contains("kept contents") {
                if let Some(b) = bin {
                    // item + count from the rest of the Consumes line
                    let item = self.item_from_phrase(&p.consumes_raw, p.consumes_line, &section);
                    let (count, item_ident) = item
                        .map(|i| (i.count, i.ident))
                        .unwrap_or((0, b.item.clone()));
                    // the waste line: "<N g> <name> → <sink>"
                    let (waste_ident, waste_mass, sink_ident) =
                        self.disposal_waste(p, &section);
                    plan.bin_disposal = Some(BinDisposal {
                        bin_ident: b.ident.clone(),
                        cap: b.capacity,
                        count,
                        item: item_ident,
                        waste_ident,
                        waste_mass,
                        sink_ident,
                    });
                    self.plans.push(plan);
                    continue;
                }
            }

            // -- consumes
            for part in item_parts(&p.consumes_raw) {
                if let Some(it) = self.item_from_phrase(&part, p.consumes_line, &section) {
                    if it.discrete && it.count > 1 {
                        plan.supply_n = Some((it.count, it.ident.clone()));
                    }
                    plan.consumes.push(it);
                }
            }
            // -- produces
            if p.produces_raw.trim() != "—" && !p.produces_raw.trim().is_empty() {
                for part in item_parts(&p.produces_raw) {
                    if let Some(it) = self.item_from_phrase(&part, p.produces_line, &section) {
                        plan.produces.push(it);
                    }
                }
            }
            // -- waste
            if let Some(w) = &p.waste_raw.clone() {
                for clause in split_top_level(w, &[';']) {
                    let halves: Vec<&str> = clause.split('→').collect();
                    if halves.len() != 2 {
                        self.warnings.push(format!(
                            "{section}: waste clause without a '→ destination': '{clause}'"
                        ));
                        continue;
                    }
                    let item = self.item_from_phrase(halves[0], p.waste_line, &section);
                    let dest = strip_parens(halves[1]).trim().to_string();
                    let dup = item
                        .as_ref()
                        .map(|i| plan.produces.iter().any(|pr| pr.ident == i.ident))
                        .unwrap_or(false);
                    let sink_var = self.sink_for_destination(&dest);
                    if sink_var.is_none() {
                        self.err(
                            p.waste_line,
                            &section,
                            format!(
                                "waste '{}' routes to '{dest}', which matches no §4 output row (every waste product needs a destination row there — SPEC_TEMPLATE §4)",
                                strip_parens(halves[0]).trim()
                            ),
                        );
                    }
                    plan.waste.push(WasteOut {
                        item,
                        destination: dest,
                        sink_var,
                        duplicate_of_produce: dup,
                    });
                }
            }
            // -- reusables from the Actor line
            let actor_low = strip_parens(&p.actors_raw).to_lowercase();
            let reusables: Vec<TypeEntry> = self
                .lexicon
                .iter()
                .filter(|e| matches!(e.role, Role::Reusable))
                .cloned()
                .collect();
            for e in reusables {
                let mentioned = e.stems.iter().any(|s| actor_low.contains(s.as_str()));
                if !mentioned {
                    continue;
                }
                // a §4 draw-source object belongs to the adjacent draw, not
                // to this process (F-048 analogue for material draws)
                if self.draws.iter().any(|d| d.obj_ident == e.ident) {
                    continue;
                }
                let state_consumed = plan
                    .consumes
                    .iter()
                    .any(|c| c.ident != e.ident && self.ident_resource(&c.ident) == Some(e.resource.clone()));
                let state_produced = plan
                    .produces
                    .iter()
                    .any(|c| c.ident != e.ident && self.ident_resource(&c.ident) == Some(e.resource.clone()));
                // The vessel enters unless one of its states is already the
                // consumed input; it returns (bare) unless it continues as a
                // produced state ("kettle returned empty" in the Actor line).
                if !state_consumed {
                    plan.reusable_params.push(e.ident.clone());
                }
                if !state_produced {
                    plan.reusable_returns.push(e.ident.clone());
                }
            }
            self.plans.push(plan);
        }
    }

    fn ident_resource(&self, ident: &str) -> Option<String> {
        self.lexicon
            .iter()
            .chain(self.synthesized.iter())
            .find(|e| e.ident == ident)
            .map(|e| e.resource.clone())
    }

    fn sink_for_destination(&self, dest: &str) -> Option<String> {
        let dws = words(dest);
        let mut best: Option<(f64, String)> = None;
        let mut consider = |name_words: &[String], ident: &str| {
            if name_words.is_empty() {
                return;
            }
            let matched = dws.iter().filter(|w| name_words.contains(w)).count();
            let need = name_words.len().clamp(1, 2);
            if matched >= need {
                let frac = matched as f64 / name_words.len() as f64;
                if best.as_ref().map(|(f, _)| frac > *f).unwrap_or(true) {
                    best = Some((frac, snake(ident)));
                }
            }
        };
        for s in &self.sinks {
            consider(&s.name_words, &s.ident);
        }
        for c in &self.consumers {
            consider(&c.name_words, &c.ident);
        }
        best.map(|(_, v)| v)
    }

    /// P5's waste: resolves (or synthesizes) the disposal mass type and its
    /// sink, returning (waste ident, mass, sink ident).
    fn disposal_waste(&mut self, p: &Process, section: &str) -> (String, u64, String) {
        let w = p.waste_raw.clone().unwrap_or_default();
        let clause = split_top_level(&w, &[';'])
            .into_iter()
            .next()
            .unwrap_or_default();
        let halves: Vec<&str> = clause.split('→').collect();
        let (left, right) = (halves.first().copied().unwrap_or(""), halves.get(1).copied().unwrap_or(""));
        let qty = parse_quantities(left).into_iter().find(|q| q.unit == "g");
        let mass = qty.as_ref().map(|q| q.value).unwrap_or(0);
        let name_part: String = words(&strip_parens(left))
            .join(" ");
        let ident = match self.resolve(left) {
            Some(e) => e.ident,
            None => {
                let ident = camel(&name_part);
                self.warnings.push(format!(
                    "{section} (line {}): waste '{}' has no §3 row; a container type {} ({}) was synthesized from this line — the template should require a §3 row for every waste product",
                    p.waste_line, left.trim(), ident, unit_name(qty.as_ref().map(|q| q.unit.as_str()).unwrap_or("g"))
                ));
                self.synthesized.push(TypeEntry {
                    ident: ident.clone(),
                    resource: name_part.clone(),
                    state: None,
                    required: vec![],
                    stems: words(&name_part),
                    quantities: vec![Qty { value: mass, unit: "g".into() }],
                    role: Role::Container {
                        extra_units: vec![],
                        slot_unit: "g".into(),
                    },
                    line: p.waste_line,
                });
                ident
            }
        };
        let sink = {
            let dws = words(&strip_parens(right));
            self.sinks
                .iter_mut()
                .find(|s| dws.iter().filter(|w| s.name_words.contains(w)).count() >= 2)
                .map(|s| {
                    if s.consumes.is_empty() {
                        s.consumes.push((ident.clone(), 1));
                    }
                    s.ident.clone()
                })
        };
        let sink = match sink {
            Some(s) => s,
            None => {
                self.err(
                    p.waste_line,
                    section,
                    format!(
                        "waste '{}' routes to '{}', which matches no §4 output row (every waste product needs a destination row there)",
                        left.trim(),
                        right.trim()
                    ),
                );
                String::new()
            }
        };
        (ident, mass, sink)
    }

    // ── validation ──────────────────────────────────────────────────────

    fn validate(&mut self) {
        let procs = self.spec.processes.clone();
        for p in &procs {
            let section = format!("§5 P{}", p.id);
            // balance arithmetic — the spec's own conservation check
            for b in &p.balances {
                let l: u64 = b.lhs.iter().sum();
                let r: u64 = b.rhs.iter().sum();
                if l != r {
                    let fmt_side = |v: &Vec<u64>| {
                        v.iter().map(|n| lit(*n).replace('_', " ")).collect::<Vec<_>>().join(" + ")
                    };
                    self.err(
                        b.line,
                        &section,
                        format!(
                            "the {} balance does not balance: {} ≠ {} (left totals {}, right totals {}; a model built from this line cannot compile — fix the specification, not the model)",
                            b.dimension,
                            fmt_side(&b.lhs),
                            fmt_side(&b.rhs),
                            lit(l).replace('_', " "),
                            lit(r).replace('_', " ")
                        ),
                    );
                }
                if b.mark == BalanceMark::Unmarked {
                    self.warnings.push(format!(
                        "{section} (line {}): balance '{}' is not marked (assert) or (structural) — the generator guesses: identical single terms on both sides ⇒ structural, otherwise assert (SPEC.md §8 feedback item 5 asked for the mark)",
                        b.line, b.raw
                    ));
                }
            }
            // time clause vs Actor draws
            if let (Some(td), Some(draw)) = (&p.time_draw, p.draws_ms) {
                if td.ms != draw {
                    self.err(
                        td.line,
                        &section,
                        format!(
                            "the Balances time clause says {} ms but the Actor line draws {} ms — the two must agree",
                            lit(td.ms).replace('_', " "),
                            lit(draw).replace('_', " ")
                        ),
                    );
                }
            }
            // Satisfies ids must exist in §2
            for id in &p.satisfies {
                let known = self
                    .spec
                    .requirements
                    .iter()
                    .any(|r| r.spec_id == *id || r.impl_id == *id);
                if !known {
                    self.err(
                        p.line,
                        &section,
                        format!("Satisfies names REQ-{id:03}, which §2 does not define"),
                    );
                }
            }
            // no person but a time draw?
            if p.no_person && p.draws_ms.is_some() {
                self.err(
                    p.line,
                    &section,
                    "the Actor line says 'no person' but also draws person-time".into(),
                );
            }
            // missing Waste routing field where waste has a destination
            if p.waste_raw.is_some() && p.waste_routing.is_none() {
                self.warnings.push(format!(
                    "{section}: no 'Waste routing' field — the generator defaults to the weak reading (the flow routes loose outputs); SPEC.md §8 feedback item 3 asked the author to choose (strong reading: the process takes its consumers as requirement-bounded parameters)"
                ));
            }
        }
        // flow orders reference real processes
        let flows = self.spec.flows.clone();
        for order in &flows.orders {
            for id in order {
                if !procs.iter().any(|p| p.id == *id) {
                    self.err(
                        flows.line,
                        "§6",
                        format!("flow order names P{id}, which §5 does not define"),
                    );
                }
            }
        }
    }
}

fn n_consts(role: &Role) -> usize {
    match role {
        Role::Container { extra_units, slot_unit } => {
            extra_units.len() + usize::from(!slot_unit.is_empty())
        }
        _ => 0,
    }
}

/// "pot of tea" + "teapot" → PotOfTea; "filled" + "kettle" → FilledKettle.
fn state_ident(state: &str, res_last: &str) -> String {
    let sws = words(state);
    let overlap = sws.iter().any(|w| {
        (w.len() >= 3 && res_last.contains(w.as_str())) || w.contains(res_last)
    });
    if overlap {
        camel(state)
    } else {
        format!("{}{}", camel(state), camel(res_last))
    }
}

/// "dry mass 3 g each; spent mass 12 g each (9 g absorbed water)"
fn find_state_mass(characteristics: &str, state: &str) -> Option<u64> {
    let low = characteristics.to_lowercase();
    let key = format!("{} mass", state.to_lowercase());
    let i = low.find(&key)?;
    parse_quantities(&characteristics[i + key.len()..])
        .first()
        .map(|q| q.value)
}

/// Splits a Consumes/Produces line into item parts: top-level ',' and '+',
/// plus the " and " conjunction.
fn item_parts(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    for p in split_top_level(s, &[',', '+']) {
        // split on " and " outside parens
        let mut rest = p.as_str();
        let mut depth_safe = Vec::new();
        loop {
            match find_top_level(rest, " and ") {
                Some(i) => {
                    depth_safe.push(rest[..i].to_string());
                    rest = &rest[i + 5..];
                }
                None => {
                    depth_safe.push(rest.to_string());
                    break;
                }
            }
        }
        for q in depth_safe {
            let t = q.trim();
            if !t.is_empty() && t != "—" {
                parts.push(t.to_string());
            }
        }
    }
    parts
}

fn find_top_level(s: &str, pat: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut depth = 0usize;
    let mut i = 0;
    while i + pat.len() <= s.len() {
        match bytes[i] {
            b'(' => depth += 1,
            b')' => depth = depth.saturating_sub(1),
            _ => {}
        }
        if depth == 0 && s[i..].starts_with(pat) {
            return Some(i);
        }
        i += 1;
    }
    None
}

// ═══════════════════════════════════════════════════════════════════════
// Emission
// ═══════════════════════════════════════════════════════════════════════

mod emit_files;
pub use emit_files::emit_crate;
