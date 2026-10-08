//! Resolution, validation and emission: turns a parsed [`Spec`] into
//! validation findings (`speccheck`, R22) and — strict mode only — the
//! one-shot scaffolded model crate (`specgen`), recording every
//! under-determined decision as a numbered SPEC-HOLE.
//!
//! ## The hole convention (documented contract, R22)
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
//! Nothing is ever guessed (R22): a decision the spec does not state is a
//! hole, never a default.
//!
//! ## Name resolution (A4)
//!
//! Strict resolution goes ONLY through backticked canonical identifiers:
//! a §5/§4 phrase resolves to the §3 state entry whose backticked state
//! token it contains (disambiguated by a resource token when present), else
//! to the §3 resource whose token it contains, else — for consumed/produced
//! boundary items like a works order — to a §4 input row's token. Waste
//! items resolve against §3 only (A3). Surrounding prose is decoration.
//! The EXP-15 fuzzy word-overlap lexicon survives only behind `--lenient`.

use crate::model::*;
use crate::parse::{
    backticks, is_mag_unit, item_list_part, lex_number, parse_quantities, split_top_level,
    strip_parens,
};
use std::fmt::Write as _;

const STOPWORDS: &[&str] = &[
    "the", "a", "an", "of", "and", "with", "to", "must", "be", "is", "are", "all", "only",
    "before", "exactly", "its", "from", "for", "per", "each", "one", "at", "in", "into",
];

/// Lowercase significant words of a phrase (plural-stripped) — the lenient
/// fuzzy-resolution vocabulary (EXP-15), and sink-name matching.
fn words(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '-')
        .flat_map(|w| w.split('-'))
        .map(|w| w.trim_end_matches('s').to_string())
        .filter(|w| !w.is_empty() && !w.chars().next().unwrap().is_ascii_digit())
        .filter(|w| !STOPWORDS.contains(&w.trim_end_matches('s')))
        .collect()
}

/// A per-item magnitude ("450 g each"): still a magnitude, never an item
/// count — a parenthesised magnitude cannot set a count (the D1 defect class).
pub(crate) fn is_mag_each(u: &str) -> bool {
    u.strip_suffix(" each").map(is_mag_unit).unwrap_or(false)
}

/// Magnitude in either spelling ("g" or "g each").
pub(crate) fn is_magnitude(u: &str) -> bool {
    is_mag_unit(u) || is_mag_each(u)
}

/// The item count stated at a mention: the first quantity that is NOT a
/// magnitude ("2 `clean` …" → 2, bare "3" → 3). A mention stating only
/// magnitudes ("the `mixed` `Dough` (1_682 g)") counts ONE item — a
/// parenthesised magnitude is never a count (CS-2 precedent: "the `patched`
/// `InnerTube` (183 g)" is one tube of 183 g).
fn count_of(qties: &[Qty]) -> u64 {
    qties
        .iter()
        .find(|q| !is_magnitude(&q.unit))
        .map(|q| q.value)
        .unwrap_or(1)
}

pub(crate) fn camel(s: &str) -> String {
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

pub(crate) fn snake(s: &str) -> String {
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
        "p" => "pence",
        "ec" => "euro cents",
        _ => "units",
    }
}

fn const_param_name(tok: &str) -> &'static str {
    match tok {
        "g" => "MASS_G",
        "J" => "ENERGY_J",
        "ms" => "TIME_MS",
        "p" => "PENCE",
        "ec" => "EURO_CENTS",
        _ => "MAGNITUDE",
    }
}

/// What a resolved §3 (resource, state) becomes in code.
#[derive(Debug, Clone, PartialEq)]
pub enum Role {
    /// `container_resource!`: `n_extra` const params ride before the macro's
    /// own magnitude slot (last-listed quantity).
    Container {
        extra_units: Vec<String>,
        slot_unit: String,
    },
    /// `consumable_resource!`, optionally holding real discrete contents.
    Consumable {
        held: Option<(u64, String)>,
        tripwire: bool,
        mass_g: Option<u64>,
    },
    Reusable,
    /// A sealed R17 outcome token (`model_core::outcome_token!`): the §3
    /// Kind cell says "outcome token (R17)".
    Outcome,
    /// Lives in model-core (`Person`, `History`): nothing to emit.
    Common,
}

#[derive(Debug, Clone)]
pub struct TypeEntry {
    pub ident: String,
    pub resource: String,
    pub state: Option<String>,
    /// The backticked canonical state token (A4), strict resolution's key.
    pub canonical_state: Option<String>,
    /// The resource row's backticked canonical tokens (A4).
    pub canonical_res: Vec<String>,
    /// A resolution-only alias (the bare-resource mention): never emitted.
    pub alias_only: bool,
    /// words that must all appear in a phrase for this entry to match (lenient)
    pub required: Vec<String>,
    /// resource stems: at least one must appear (lenient; containment either way)
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
    /// R17 "Produces (Ok):" items — the success arm's products.
    pub produces_ok: Vec<Item>,
    /// R17 "Produces (Fail):" items — the failure arm's products.
    pub produces_fail: Vec<Item>,
    /// The consumed R17 outcome token's ident, when the process takes one.
    pub outcome: Option<String>,
    pub waste: Vec<WasteOut>,
    /// Some(count, item) when consuming N discrete items via SupplyN —
    /// only for a genuine multi-item draw from a §4 discrete supplier.
    pub supply_n: Option<(u64, String)>,
    /// True when the supplier's own container is also listed under Consumes
    /// (the exhausted box leaves through this process): the supplier
    /// parameter IS the container, pinned to `Rest = Empty<Supplier>`.
    pub supplier_consumed: bool,
    /// Some(…) for the bin-disposal shape (the F-039 sealed path)
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

/// A §4 bounded continuous source "container with remainder" (the pantry
/// bag): a container object entering full, drawn down by an R15
/// `draw_process!`, its remainder accounted at flow end.
#[derive(Debug, Clone)]
pub struct BagDraw {
    pub obj_ident: String,
    pub fn_name: String,
    pub item: String,
    pub capacity: u64,
    pub slot_unit: String,
    pub line: usize,
}

/// A stateful object entering at flow start from a §4 setup row (e.g. the
/// `clean` tins): a §3 Container state needing a boundary constructor.
#[derive(Debug, Clone)]
pub struct StartObject {
    pub ident: String,
    pub consts: Vec<u64>,
    pub line: usize,
}

/// A declared §4 flow-end rest row (A13): a resource that comes to rest at
/// the boundary at flow end — a container remainder, a reusable that stays
/// behind — written as an output row with `(flow-end rest)` in its Via cell.
/// Each declared row gets exactly one `rest_*` boundary exit (R12), the
/// accounted counterpart of a constructor (the CS-2 `take_wallet_home`
/// shape); the generator synthesizes none (A13): an undeclared rest has no
/// exit and surfaces at the model as a tripwire panic, the designed loud
/// failure.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RestExit {
    /// The resting type's ident (a continuous item resting in its §4 entry
    /// container rests as that container, e.g. `Flour` → `PantryBag`).
    pub ident: String,
    /// Its const-parameter count.
    pub n_consts: usize,
    /// The declared count ("2 `used` `LoafTin`s" rests two).
    pub count: u64,
    /// The §4 row's line, for the emitted breadcrumb (F-061).
    pub line: usize,
}

pub struct Generator {
    pub spec: Spec,
    pub strict: bool,
    pub lexicon: Vec<TypeEntry>,
    pub errors: Vec<SpecError>,
    pub warnings: Vec<SpecError>,
    pub holes: Vec<Hole>,
    pub sinks: Vec<Sink>,
    pub consumers: Vec<BoundedConsumer>,
    pub suppliers: Vec<SupplierPlan>,
    pub draws: Vec<DrawSource>,
    pub bags: Vec<BagDraw>,
    pub start_objects: Vec<StartObject>,
    /// The declared §4 flow-end rest rows (A13): filled by [`plan_boundary`]
    /// from the output rows marked `(flow-end rest)`, consumed by the flow
    /// and boundary emitters — one `rest_*` exit per declared row, never
    /// synthesized.
    pub rest_exits: Vec<RestExit>,
    pub plans: Vec<ProcPlan>,
    pub person_budget: u64,
    /// lenient only: containers synthesized from §5 waste lines with no §3
    /// row (strict mode errors instead — A3)
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

    fn warn(&mut self, line: usize, section: &str, message: String) {
        self.warnings.push(SpecError {
            line,
            section: section.to_string(),
            message,
        });
    }

    /// Resolves a phrase to a lexicon entry. Strict: through backticked
    /// canonical identifiers only (A4) — state tokens first (narrowed by a
    /// resource token when present), then resource tokens; first §3-order
    /// match wins (deterministic). Lenient: falls back to the EXP-15
    /// word-overlap scoring.
    pub fn resolve(&self, phrase: &str) -> Option<TypeEntry> {
        let toks = backticks(phrase);
        if !toks.is_empty() {
            // state entries first (more specific)
            let state_matches: Vec<&TypeEntry> = self
                .lexicon
                .iter()
                .filter(|e| {
                    e.canonical_state
                        .as_ref()
                        .map(|st| toks.iter().any(|t| t == st))
                        .unwrap_or(false)
                })
                .collect();
            if state_matches.len() == 1 {
                return Some(state_matches[0].clone());
            }
            if state_matches.len() > 1 {
                // narrow by a resource token in the same phrase
                let narrowed: Vec<&&TypeEntry> = state_matches
                    .iter()
                    .filter(|e| e.canonical_res.iter().any(|r| toks.contains(r)))
                    .collect();
                if let Some(e) = narrowed.first() {
                    return Some((**e).clone());
                }
                return Some(state_matches[0].clone());
            }
            if let Some(e) = self
                .lexicon
                .iter()
                .find(|e| e.canonical_state.is_none() && e.canonical_res.iter().any(|r| toks.contains(r)))
            {
                return Some(e.clone());
            }
        }
        if self.strict {
            return None;
        }
        self.resolve_fuzzy(phrase)
    }

    /// The EXP-15 CS-1 fuzzy lexicon (lenient only): best word-overlap score;
    /// every `required` word must appear; at least one stem must match.
    fn resolve_fuzzy(&self, phrase: &str) -> Option<TypeEntry> {
        let ws = words(&strip_parens(phrase));
        let matches_word = |w: &str, k: &str| -> bool {
            w == k || (w.len() >= 3 && k.contains(w)) || (k.len() >= 3 && w.contains(k))
        };
        let mut best: Option<(usize, &TypeEntry)> = None;
        let mut tie = false;
        for e in self.lexicon.iter().chain(self.synthesized.iter()) {
            if !e
                .required
                .iter()
                .all(|r| ws.iter().any(|w| matches_word(w, r)))
            {
                continue;
            }
            if !e.stems.is_empty() && !e.stems.iter().any(|s| ws.iter().any(|w| matches_word(w, s)))
            {
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

    pub fn new(spec: Spec, strict: bool) -> Generator {
        let mut g = Generator {
            spec,
            strict,
            lexicon: Vec::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
            holes: Vec::new(),
            sinks: Vec::new(),
            consumers: Vec::new(),
            suppliers: Vec::new(),
            draws: Vec::new(),
            bags: Vec::new(),
            start_objects: Vec::new(),
            rest_exits: Vec::new(),
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

    /// One state name appears on exactly one resource row (A7). Strict: a
    /// duplicate is an error. Lenient: the EXP-15 quantity-bearing-row-wins
    /// heuristic, reported as a warning.
    fn state_owner_map(&mut self) -> std::collections::BTreeMap<String, usize> {
        let resources = self.spec.resources.clone();
        let mut owned: std::collections::BTreeMap<String, (usize, bool)> =
            std::collections::BTreeMap::new();
        for (i, r) in resources.iter().enumerate() {
            for s in &r.states {
                let key = if self.strict {
                    s.name.clone()
                } else {
                    s.name.to_lowercase()
                };
                let has = !s.quantities.is_empty();
                match owned.get(&key) {
                    Some((j, prev_has)) => {
                        if self.strict {
                            self.errors.push(SpecError {
                                line: r.line,
                                section: "§3".into(),
                                message: format!(
                                    "state `{}` appears on both '{}' (line {}) and '{}' — each state name appears on exactly one resource row (A7): name the owning row's state and let prose mention the rest",
                                    s.name,
                                    resources[*j].name,
                                    resources[*j].line,
                                    r.name
                                ),
                            });
                        } else if has && !prev_has {
                            let loser = resources[*j].name.clone();
                            self.warnings.push(SpecError {
                                line: r.line,
                                section: "§3".into(),
                                message: format!(
                                    "state '{}' appears on both '{}' and '{}'; the quantity-bearing row ('{}') owns the type — the other is treated as narrative",
                                    s.name, loser, r.name, r.name
                                ),
                            });
                            owned.insert(key, (i, has));
                        }
                    }
                    None => {
                        owned.insert(key, (i, has));
                    }
                }
            }
        }
        if !self.strict {
            // subset-named collisions ("tea" vs "pot of tea"): the longer,
            // quantity-bearing name owns it (EXP-15 heuristic)
            let keys: Vec<String> = owned.keys().cloned().collect();
            for k in &keys {
                for k2 in &keys {
                    if k != k2 && k2.split(' ').any(|w| w == k) {
                        let (i2, q2) = owned[k2];
                        let (_, q1) = owned[k];
                        if q2 && !q1 {
                            owned.insert(k.clone(), (i2, true));
                        }
                    }
                }
            }
        }
        owned.into_iter().map(|(k, (i, _))| (k, i)).collect()
    }

    fn build_lexicon(&mut self) {
        let strict = self.strict;
        let owned = self.state_owner_map();
        let resources = self.spec.resources.clone();
        let state_key = |name: &str| -> String {
            if strict {
                name.to_string()
            } else {
                name.to_lowercase()
            }
        };

        for (i, r) in resources.iter().enumerate() {
            let res_words = words(&r.name);
            let res_last = res_words.last().cloned().unwrap_or_default();
            let low = r.name.to_lowercase();
            let canon: Vec<String> = if strict {
                r.idents.clone()
            } else {
                backticks(&r.name)
            };
            // model-core common resources: the `Person` row, or a row whose
            // parenthesised prose alias says "(person)" (CS-2's `Member`,
            // CS-6's `Baker` — the actor rides model-core's Person budget).
            if low.contains("person")
                && (low.starts_with("person") || canon.iter().any(|c| c == "Person"))
                || canon.iter().any(|c| c == "Person")
                || low.contains("(person)")
            {
                self.person_budget = parse_quantities(&r.quantity_raw, strict)
                    .first()
                    .map(|q| q.value)
                    .unwrap_or(0);
                self.lexicon.push(TypeEntry {
                    ident: "Person".into(),
                    resource: low.clone(),
                    state: None,
                    canonical_state: None,
                    canonical_res: if canon.is_empty() {
                        vec!["Person".into()]
                    } else {
                        canon.clone()
                    },
                    alias_only: true,
                    required: vec!["person".into()],
                    stems: vec!["person".into()],
                    quantities: parse_quantities(&r.quantity_raw, strict),
                    role: Role::Common,
                    line: r.line,
                });
                continue;
            }
            // the row's base ident: the first canonical token, else camel(name)
            let base_ident = canon
                .first()
                .map(|c| camel(c))
                .unwrap_or_else(|| camel(&r.name));
            let owned_by_me = |name: &str| owned.get(&state_key(name)).copied() == Some(i);

            match &r.kind {
                Kind::Continuous { .. } if r.states.is_empty() => {
                    let unit = parse_quantities(&r.quantity_raw, strict)
                        .into_iter()
                        .find(|q| !q.unit.is_empty())
                        .map(|q| q.unit)
                        .unwrap_or_default();
                    self.lexicon.push(TypeEntry {
                        ident: base_ident.clone(),
                        resource: low.clone(),
                        state: None,
                        canonical_state: None,
                        canonical_res: canon.clone(),
                        alias_only: false,
                        required: vec![],
                        stems: res_words.clone(),
                        quantities: parse_quantities(&r.quantity_raw, strict),
                        role: Role::Container {
                            extra_units: vec![],
                            slot_unit: unit,
                        },
                        line: r.line,
                    });
                }
                Kind::Continuous { .. } => {
                    // one container per owned state, plus a resolution-only
                    // base alias for unqualified mentions ("150 g `Milk`")
                    for s in &r.states {
                        if !owned_by_me(&s.name) {
                            continue;
                        }
                        let ident = state_ident(&s.name, &res_last);
                        let unit = s
                            .quantities
                            .last()
                            .map(|q| q.unit.clone())
                            .or_else(|| {
                                parse_quantities(&r.quantity_raw, strict)
                                    .first()
                                    .map(|q| q.unit.clone())
                            })
                            .unwrap_or_default();
                        self.lexicon.push(TypeEntry {
                            ident,
                            resource: low.clone(),
                            state: Some(s.name.to_lowercase()),
                            canonical_state: Some(s.name.clone()),
                            canonical_res: canon.clone(),
                            alias_only: false,
                            required: words(&s.name),
                            stems: res_words.clone(),
                            quantities: s.quantities.clone(),
                            role: Role::Container {
                                extra_units: s.quantities[..s.quantities.len().saturating_sub(1)]
                                    .iter()
                                    .filter(|q| is_mag_unit(&q.unit))
                                    .map(|q| q.unit.clone())
                                    .collect(),
                                slot_unit: unit,
                            },
                            line: r.line,
                        });
                    }
                    let unit = parse_quantities(&r.quantity_raw, strict)
                        .into_iter()
                        .find(|q| !q.unit.is_empty())
                        .map(|q| q.unit)
                        .unwrap_or_default();
                    self.lexicon.push(TypeEntry {
                        ident: base_ident.clone(),
                        resource: low.clone(),
                        state: None,
                        canonical_state: None,
                        canonical_res: canon.clone(),
                        alias_only: true,
                        required: vec![],
                        stems: res_words.clone(),
                        quantities: parse_quantities(&r.quantity_raw, strict),
                        role: Role::Container {
                            extra_units: vec![],
                            slot_unit: unit,
                        },
                        line: r.line,
                    });
                }
                // R17 outcome tokens: the §3 Kind cell says so (CS-2's
                // `PatchOutcome`, CS-6's `BakeOutcome`) — a sealed
                // `model_core::outcome_token!`, injected at the boundary.
                Kind::Other(k) if k.to_lowercase().contains("outcome token") => {
                    self.lexicon.push(TypeEntry {
                        ident: base_ident.clone(),
                        resource: low.clone(),
                        state: None,
                        canonical_state: None,
                        canonical_res: canon.clone(),
                        alias_only: false,
                        required: vec![],
                        stems: res_words.clone(),
                        quantities: parse_quantities(&r.quantity_raw, strict),
                        role: Role::Outcome,
                        line: r.line,
                    });
                }
                Kind::Discrete | Kind::Product | Kind::Other(_) => {
                    for s in &r.states {
                        if !owned_by_me(&s.name) {
                            continue;
                        }
                        let ident = state_ident(&s.name, &res_last);
                        // per-state mass from the characteristics column:
                        // "<state> mass N g"
                        let mass = find_state_mass(&r.characteristics, &s.name, strict);
                        let role = Role::Consumable {
                            held: None,
                            tripwire: true, // refined in plan_boundary (F-040 rule)
                            mass_g: mass,
                        };
                        self.lexicon.push(TypeEntry {
                            ident,
                            resource: low.clone(),
                            state: Some(s.name.to_lowercase()),
                            canonical_state: Some(s.name.clone()),
                            canonical_res: canon.clone(),
                            alias_only: false,
                            required: words(&s.name),
                            stems: res_words.clone(),
                            quantities: s.quantities.clone(),
                            role,
                            line: r.line,
                        });
                    }
                    if r.states.is_empty() {
                        self.lexicon.push(TypeEntry {
                            ident: base_ident.clone(),
                            resource: low.clone(),
                            state: None,
                            canonical_state: None,
                            canonical_res: canon.clone(),
                            alias_only: false,
                            required: vec![],
                            stems: res_words.clone(),
                            quantities: parse_quantities(&r.quantity_raw, strict),
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
                            canonical_state: None,
                            canonical_res: canon.clone(),
                            alias_only: true,
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
                        ident: base_ident.clone(),
                        resource: low.clone(),
                        state: None,
                        canonical_state: None,
                        canonical_res: canon.clone(),
                        alias_only: false,
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
                        if !owned_by_me(&s.name) {
                            continue;
                        }
                        let ident = state_ident(&s.name, &res_last);
                        // held discrete contents? quantity like "3 bags"
                        let held = s.quantities.iter().find_map(|q| {
                            if q.unit.is_empty() || is_mag_unit(&q.unit) {
                                return None;
                            }
                            let key = q.unit.trim_end_matches('s').to_lowercase();
                            self.lexicon
                                .iter()
                                .find(|e| {
                                    e.state.is_none()
                                        && (e
                                            .canonical_res
                                            .iter()
                                            .any(|c| {
                                                let cl = c.to_lowercase();
                                                cl.contains(&key) || key.contains(&cl)
                                            })
                                            || e.stems.iter().any(|st| {
                                                st.contains(&key) || key.contains(st.as_str())
                                            }))
                                })
                                .map(|e| (q.value, e.ident.clone()))
                        });
                        let numeric: Vec<&Qty> = s
                            .quantities
                            .iter()
                            .filter(|q| is_mag_unit(&q.unit))
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
                            canonical_state: Some(s.name.clone()),
                            canonical_res: canon.clone(),
                            alias_only: false,
                            required: words(&s.name),
                            stems: res_words.clone(),
                            quantities: s.quantities.clone(),
                            role,
                            line: r.line,
                        });
                    }
                }
            }
        }
    }

    // ── §4 → boundary plans ─────────────────────────────────────────────

    fn kind_of_entry(&self, e: &TypeEntry) -> Option<Kind> {
        self.spec
            .resources
            .iter()
            .find(|r| r.name.to_lowercase() == e.resource)
            .map(|r| r.kind.clone())
    }

    fn plan_boundary(&mut self) {
        let strict = self.strict;
        let inputs = self.spec.inputs.clone();
        for row in &inputs {
            let via_low = row.via.to_lowercase();
            if via_low.contains("draw process") {
                let obj = camel(split_top_level(&row.via, &[','])[0].as_str());
                let what = strip_parens(&row.what);
                let item = match self.resolve(&row.what) {
                    Some(e) => e.ident,
                    None => {
                        self.err(
                            row.line,
                            "§4",
                            format!(
                                "input '{}' names no §3 resource by its canonical identifier (A4)",
                                row.what
                            ),
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
            } else if let Some((cap, _)) = lex_number(row.capacity.trim(), strict) {
                // Bounded continuous source "container with remainder" (the
                // CS-6 pantry rows): a container object enters full, an R15
                // draw_process! draws it down, the remainder is accounted at
                // flow end (§6 "back as container remainders").
                let resolved = self.resolve(&row.what);
                if via_low.contains("container") {
                    if let Some(e) = &resolved {
                        let continuous = matches!(
                            self.kind_of_entry(e),
                            Some(Kind::Continuous { .. })
                        );
                        if continuous {
                            if let Role::Container { slot_unit, .. } = &e.role {
                                let obj = camel(
                                    split_top_level(&strip_parens(&row.via), &[','])[0].as_str(),
                                );
                                self.bags.push(BagDraw {
                                    obj_ident: obj,
                                    fn_name: format!(
                                        "draw_{}",
                                        snake(&strip_parens(&row.what))
                                    ),
                                    item: e.ident.clone(),
                                    capacity: cap,
                                    slot_unit: slot_unit.clone(),
                                    line: row.line,
                                });
                                continue;
                            }
                        }
                    }
                }
                let discrete = resolved
                    .as_ref()
                    .and_then(|e| self.kind_of_entry(e))
                    .map(|k| matches!(k, Kind::Discrete))
                    .unwrap_or(false);
                if !discrete {
                    // reusables entering at flow start: constructors are
                    // emitted for every reusable anyway, but a stateful start
                    // object (the `clean` tins) needs its own constructor
                    self.plan_start_objects(row);
                    continue;
                }
                if let Some(e) = resolved {
                    let ident = camel(&strip_parens(&row.via));
                    self.suppliers.push(SupplierPlan {
                        ident: ident.clone(),
                        item: e.ident.clone(),
                        capacity: cap,
                        fill_fn: format!("full_{}", snake(&strip_parens(&row.via))),
                        line: row.line,
                    });
                    // F-040 rule: items minted into a boundary supplier are
                    // kept by containers — no_tripwire.
                    let item = e.ident.clone();
                    for l in self.lexicon.iter_mut() {
                        if l.ident == item {
                            if let Role::Consumable { tripwire, .. } = &mut l.role {
                                *tripwire = false;
                            }
                        }
                    }
                }
            } else {
                // "kitchen setup" style row: reusables enter here; the
                // reusable types already exist, constructors are emitted for
                // every reusable. A stateful start object still needs one.
                self.plan_start_objects(row);
            }
        }
        let outputs = self.spec.outputs.clone();
        for row in &outputs {
            let via_low = row.via.to_lowercase();
            if via_low.contains("history") || via_low.contains("histories") {
                continue; // R16 common machinery
            }
            if via_low.contains("flow-end rest") {
                // A13: a declared flow-end rest row — one `rest_*` boundary
                // exit per row (R12), the accounted counterpart of a
                // constructor; nothing here is a sink or consumer.
                match self.resolve(&row.what) {
                    Some(e) => {
                        let count = count_of(&parse_quantities(&row.what, strict));
                        // a continuous item resting in its §4 entry container
                        // rests as that container (the pantry-bag remainder)
                        let (ident, n) = match self.bags.iter().find(|b| b.item == e.ident) {
                            Some(b) => (b.obj_ident.clone(), 1),
                            None => (e.ident.clone(), n_consts(&e.role)),
                        };
                        if !self.rest_exits.iter().any(|r| r.ident == ident) {
                            self.rest_exits.push(RestExit {
                                ident,
                                n_consts: n,
                                count,
                                line: row.line,
                            });
                        }
                    }
                    None => self.err(
                        row.line,
                        "§4",
                        format!(
                            "flow-end rest '{}' names no §3 resource by its canonical identifier (A4, A13)",
                            row.what
                        ),
                    ),
                }
                continue;
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
                let bounded = lex_number(
                    cap_cell.trim_start_matches(|c: char| !c.is_ascii_digit()),
                    strict,
                );
                if cap_cell.contains("unbounded") || bounded.is_none() {
                    // Next = Self sink
                    let ident = camel(&strip_parens(hop_clean));
                    let consumed: Vec<(String, usize)> = if i == 0 {
                        match self.resolve(&row.what) {
                            Some(e) => vec![(e.ident.clone(), n_consts(&e.role))],
                            None => vec![],
                        }
                    } else {
                        vec![] // later hop: wired from §5 waste lines afterwards
                    };
                    if let Some(existing) = self.sinks.iter_mut().find(|s| s.ident == ident) {
                        // Two §4 rows naming one sink (steam AND waste heat →
                        // the atmosphere): the sink consumes both — dropping
                        // the second row's item left the sink unable to take
                        // it at all.
                        for c in consumed {
                            if !existing.consumes.contains(&c) {
                                existing.consumes.push(c);
                            }
                        }
                    } else {
                        self.sinks.push(Sink {
                            ident,
                            name_words: words(hop_clean),
                            consumes: consumed,
                            placeholder_note: row.status.clone(),
                            line: row.line,
                        });
                    }
                } else if let Some((cap, _)) = bounded {
                    let ident = camel(&strip_parens(hop_clean));
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
                            format!(
                                "output '{}' names no §3 resource by its canonical identifier (A4)",
                                row.what
                            ),
                        ),
                    }
                }
            }
        }
    }

    /// A §4 setup row ("kitchen setup at flow start") listing a stateful
    /// start object — a §3 Container STATE like the `clean` tins — plans a
    /// boundary constructor for it; bare reusables already get one.
    fn plan_start_objects(&mut self, row: &BoundaryRow) {
        for part in split_top_level(&row.what, &[',']) {
            let Some(e) = self.resolve(&part) else { continue };
            if e.alias_only {
                continue;
            }
            if let Role::Container { .. } = e.role {
                let consts: Vec<u64> = e
                    .quantities
                    .iter()
                    .filter(|q| is_magnitude(&q.unit))
                    .map(|q| q.value)
                    .collect();
                if !self.start_objects.iter().any(|s| s.ident == e.ident) {
                    self.start_objects.push(StartObject {
                        ident: e.ident.clone(),
                        consts,
                        line: row.line,
                    });
                }
            }
        }
    }

    // ── §5 → process plans ──────────────────────────────────────────────

    /// Strict §4-input fallback: a boundary-entering item (a works order, an
    /// outcome token box) may resolve through a §4 input row's backticked
    /// token; deterministic exact match, no guessing. Waste items never use
    /// it (A3: waste products are §3 rows).
    fn resolve_via_inputs(&self, phrase: &str) -> bool {
        let toks = backticks(phrase);
        self.spec.inputs.iter().any(|row| {
            backticks(&row.what)
                .iter()
                .any(|t| toks.iter().any(|p| p == t))
        })
    }

    fn item_from_phrase(
        &mut self,
        phrase: &str,
        line: usize,
        section: &str,
        is_waste: bool,
    ) -> Option<Item> {
        let qties = parse_quantities(phrase, self.strict);
        if !self.strict {
            // trailing participial prose is narrative, not an item (lenient)
            let first_word = strip_parens(phrase);
            let first_word = first_word.split_whitespace().next().unwrap_or("");
            if qties.is_empty() && first_word.to_lowercase().ends_with("ed") {
                self.warn(
                    line,
                    section,
                    format!("narrative clause skipped in an item list: '{}'", phrase.trim()),
                );
                return None;
            }
        }
        let entry = self.resolve(phrase).or_else(|| {
            if self.strict {
                return None;
            }
            // lenient boundary fallback: "drawn from the grid" → §4 via words
            let ws = words(&strip_parens(phrase));
            let inputs = self.spec.inputs.clone();
            for row in &inputs {
                let via_ws = words(&row.via);
                if ws.iter().any(|w| via_ws.contains(w)) {
                    if let Some(e) = self.resolve_fuzzy(&strip_parens(&row.what)) {
                        return Some(e);
                    }
                }
            }
            None
        });
        let Some(entry) = entry else {
            if self.strict && !is_waste && self.resolve_via_inputs(phrase) {
                return None; // a §4 boundary item: legal, nothing to plan
            }
            if self.strict {
                self.err(
                    line,
                    section,
                    format!(
                        "'{}' names no §3 resource or state by a backticked canonical identifier (A4/A9): every item is quantity + canonical identifier (+ an optional parenthesised reference); explanatory prose goes in its own sentence after the list{}",
                        phrase.trim(),
                        if is_waste {
                            " — and every waste product has its own §3 row (A3)"
                        } else {
                            ""
                        }
                    ),
                );
            } else if !qties.is_empty() {
                self.err(
                    line,
                    section,
                    format!(
                        "'{}' names no §3 resource or state: every consumed/produced item must be a §3 row (or a state listed there)",
                        phrase.trim()
                    ),
                );
            } else {
                self.warn(
                    line,
                    section,
                    format!("unparsed prose left in an item list: '{}'", phrase.trim()),
                );
            }
            return None;
        };
        match &entry.role {
            Role::Container {
                extra_units,
                slot_unit,
            } => {
                // const args in declaration order: prefer the magnitudes
                // written at THIS mention ("456 g" and "456 g each" both
                // magnitudes); fall back to the §3 declaration
                let mut consts: Vec<u64> = qties
                    .iter()
                    .filter(|q| is_magnitude(&q.unit))
                    .map(|q| q.value)
                    .collect();
                if consts.is_empty() {
                    consts = entry
                        .quantities
                        .iter()
                        .filter(|q| is_magnitude(&q.unit))
                        .map(|q| q.value)
                        .collect();
                }
                let want = extra_units.len() + usize::from(!slot_unit.is_empty());
                if want > 0 && consts.len() != want {
                    self.warn(
                        line,
                        section,
                        format!(
                            "'{}' resolved to {} which carries {} magnitude(s), but {} were stated here",
                            phrase.trim(),
                            entry.ident,
                            want,
                            consts.len()
                        ),
                    );
                }
                Some(Item {
                    ident: entry.ident.clone(),
                    consts,
                    // "2 `clean` `LoafTin`s (450 g each)" is TWO items of
                    // 450 g, never one (the D3 defect class)
                    count: count_of(&qties),
                    discrete: false,
                })
            }
            Role::Consumable { .. } | Role::Common if entry.ident == "Person" => None,
            Role::Consumable { .. } | Role::Common => {
                // Count = the first stated NON-magnitude quantity; with none,
                // ONE item — never a magnitude pressed into a count (the D1
                // defect: "the `mixed` `Dough` (1_682 g)" is one 1_682 g
                // dough, not 1_682 doughs).
                let mut count = count_of(&qties);
                // "loaded pot (3 bags, …)": the 3 counts the HELD contents,
                // not three pots
                if let Role::Consumable {
                    held: Some((h, _)), ..
                } = &entry.role
                {
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
                // respect a stated count here too (D3): "2 `LoafTin`s" is
                // two parameters/returns, not one
                count: count_of(&qties),
                discrete: false,
            }),
            Role::Outcome => Some(Item {
                ident: entry.ident.clone(),
                consts: vec![],
                count: count_of(&qties),
                discrete: true,
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
                person: !p.no_person
                    && (p.actors_raw.to_lowercase().contains("person") || p.draws_ms.is_some()),
                reusable_params: Vec::new(),
                reusable_returns: Vec::new(),
                consumes: Vec::new(),
                produces: Vec::new(),
                produces_ok: Vec::new(),
                produces_fail: Vec::new(),
                outcome: None,
                waste: Vec::new(),
                supply_n: None,
                supplier_consumed: false,
                bin_disposal: None,
            };

            let consumes_list = item_list_part(&p.consumes_raw).to_string();
            // -- the disposal shape: consuming a bounded consumer's kept contents
            let consumes_low = consumes_list.to_lowercase();
            let bin = self
                .consumers
                .iter()
                .find(|c| {
                    c.name_words.iter().any(|w| {
                        consumes_low.contains(w.as_str())
                            || p.actors_raw.to_lowercase().contains(w.as_str())
                    })
                })
                .cloned();
            if consumes_low.contains("kept contents") {
                if let Some(b) = bin {
                    // item + count from the rest of the Consumes line
                    let item =
                        self.item_from_phrase(&consumes_list, p.consumes_line, &section, false);
                    let (count, item_ident) = item
                        .map(|i| (i.count, i.ident))
                        .unwrap_or((0, b.item.clone()));
                    // the waste line: "<N g> <name> → <sink>"
                    let (waste_ident, waste_mass, sink_ident) = self.disposal_waste(p, &section);
                    plan.bin_disposal = Some(BinDisposal {
                        bin_ident: b.ident.clone(),
                        cap: b.capacity,
                        count,
                        item: item_ident,
                        waste_ident,
                        waste_mass,
                        sink_ident,
                    });
                    self.check_waste_destinations(p, &section);
                    self.plans.push(plan);
                    continue;
                }
            }

            // -- consumes
            if consumes_list.trim() != "—" && !consumes_list.trim().is_empty() {
                for part in item_parts(&consumes_list) {
                    if let Some(it) =
                        self.item_from_phrase(&part, p.consumes_line, &section, false)
                    {
                        // SupplyN only for a genuine multi-item draw from a
                        // §4 discrete supplier; any other consumed item —
                        // whatever its count — stays a by-value parameter
                        // (the D2 defect: items routed to a supplier that
                        // does not exist vanished from the signature).
                        if it.discrete
                            && it.count > 1
                            && self.suppliers.iter().any(|sp| sp.item == it.ident)
                        {
                            plan.supply_n = Some((it.count, it.ident.clone()));
                        }
                        plan.consumes.push(it);
                    }
                }
            }
            // The supplier's own container listed under Consumes (CS-6:
            // taking both sachets exhausts the box, and the box leaves as the
            // `empty` state): the supplier parameter IS that container — no
            // second input parameter, and `Rest` is pinned to the exhausted
            // supplier so the §3 `empty` state can continue from it.
            if let Some((_, supply_item)) = &plan.supply_n.clone() {
                if let Some(sp) = self
                    .suppliers
                    .iter()
                    .find(|sp| &sp.item == supply_item)
                    .cloned()
                {
                    let lexicon = self.lexicon.clone();
                    let mut consumed_box = false;
                    plan.consumes.retain(|c| {
                        let is_box = lexicon.iter().any(|e| {
                            e.ident == c.ident
                                && e.canonical_res.iter().any(|t| camel(t) == sp.ident)
                        });
                        if is_box {
                            consumed_box = true;
                        }
                        !is_box
                    });
                    plan.supplier_consumed = consumed_box;
                }
            }
            // -- the consumed R17 outcome token, if any
            for c in &plan.consumes {
                let is_outcome = self
                    .lexicon
                    .iter()
                    .any(|e| e.ident == c.ident && matches!(e.role, Role::Outcome));
                if is_outcome {
                    plan.outcome = Some(c.ident.clone());
                }
            }
            // -- produces (plus R17 variant fields, checked for closure)
            let produces_list = item_list_part(&p.produces_raw).to_string();
            if produces_list.trim() != "—" && !produces_list.trim().is_empty() {
                for part in item_parts(&produces_list) {
                    if let Some(it) =
                        self.item_from_phrase(&part, p.produces_line, &section, false)
                    {
                        plan.produces.push(it);
                    }
                }
            }
            for (label, value, vline) in &p.produces_variants.clone() {
                let list = item_list_part(value).to_string();
                if list.trim() == "—" || list.trim().is_empty() {
                    continue;
                }
                for part in item_parts(&list) {
                    // R17 arms: keep the resolved items — discarding them
                    // scaffolded a fallible process that returned NOTHING in
                    // either arm (the D4 defect).
                    let it = self.item_from_phrase(&part, *vline, &section, false);
                    if let Some(it) = it {
                        if label.contains("(ok") {
                            plan.produces_ok.push(it);
                        } else if label.contains("(fail") {
                            plan.produces_fail.push(it);
                        }
                    }
                }
            }
            // -- waste
            if let Some(w) = &p.waste_raw.clone() {
                let wlist = item_list_part(w).to_string();
                for clause in split_top_level(&wlist, &[';']) {
                    let halves: Vec<&str> = clause.split('→').collect();
                    if halves.len() != 2 {
                        if self.strict {
                            self.err(
                                p.waste_line,
                                &section,
                                format!(
                                    "waste clause without a '→ destination': '{clause}' (every §5 waste output names its §4 destination)"
                                ),
                            );
                        } else {
                            self.warn(
                                p.waste_line,
                                &section,
                                format!("waste clause without a '→ destination': '{clause}'"),
                            );
                        }
                        continue;
                    }
                    let item = self.item_from_phrase(halves[0], p.waste_line, &section, true);
                    let dest = strip_parens(halves[1]).trim().to_string();
                    let dup = item
                        .as_ref()
                        .map(|i| plan.produces.iter().any(|pr| pr.ident == i.ident))
                        .unwrap_or(false);
                    let sink_var = self.sink_for_destination(&dest);
                    plan.waste.push(WasteOut {
                        item,
                        destination: dest,
                        sink_var,
                        duplicate_of_produce: dup,
                    });
                }
            }
            self.check_waste_destinations(p, &section);
            // -- reusables from the Actor line
            let actor_low = strip_parens(&p.actors_raw).to_lowercase();
            let reusables: Vec<TypeEntry> = self
                .lexicon
                .iter()
                .filter(|e| matches!(e.role, Role::Reusable))
                .cloned()
                .collect();
            for e in reusables {
                let mentioned = e.stems.iter().any(|s| actor_low.contains(s.as_str()))
                    || e.canonical_res
                        .iter()
                        .any(|c| actor_low.contains(&c.to_lowercase()));
                if !mentioned {
                    continue;
                }
                // a §4 draw-source object belongs to the adjacent draw, not
                // to this process (F-048 analogue for material draws)
                if self.draws.iter().any(|d| d.obj_ident == e.ident) {
                    continue;
                }
                let state_consumed = plan.consumes.iter().any(|c| {
                    c.ident != e.ident && self.ident_resource(&c.ident) == Some(e.resource.clone())
                });
                let state_produced = plan
                    .produces
                    .iter()
                    .chain(plan.produces_ok.iter())
                    .chain(plan.produces_fail.iter())
                    .any(|c| {
                        c.ident != e.ident
                            && self.ident_resource(&c.ident) == Some(e.resource.clone())
                    });
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

    /// Normalizes a destination/row phrase for the §5→§4 waste-destination
    /// closure: lowercase, parens stripped, punctuation collapsed.
    fn norm_dest(s: &str) -> String {
        strip_parens(s)
            .to_lowercase()
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() { c } else { ' ' }
            })
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Every §5 waste output has a §4 destination row (R22): the stated
    /// destination (after '→', parens stripped) must appear within some §4
    /// output row's What/Via cells — exact normalized containment, never a
    /// similarity score.
    fn check_waste_destinations(&mut self, p: &Process, section: &str) {
        let Some(w) = &p.waste_raw else { return };
        let wlist = item_list_part(w).to_string();
        let rows: Vec<String> = self
            .spec
            .outputs
            .iter()
            .map(|r| Self::norm_dest(&format!("{} {}", r.what, r.via)))
            .collect();
        for clause in split_top_level(&wlist, &[';']) {
            let halves: Vec<&str> = clause.split('→').collect();
            if halves.len() != 2 {
                continue; // already reported
            }
            let dest = Self::norm_dest(halves[1]);
            if dest.is_empty() {
                continue;
            }
            let found = rows.iter().any(|r| r.contains(&dest) || dest.contains(r.as_str()));
            if !found {
                self.err(
                    p.waste_line,
                    section,
                    format!(
                        "waste '{}' routes to '{}', which matches no §4 output row (every waste product needs a destination row there — SPEC_TEMPLATE §4)",
                        strip_parens(halves[0]).trim(),
                        strip_parens(halves[1]).trim()
                    ),
                );
            }
        }
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

    /// The disposal waste: resolves the disposal mass type and its sink,
    /// returning (waste ident, mass, sink ident). Strict mode never
    /// synthesizes a type — a §5 waste product with no §3 row is an error
    /// (A3); lenient synthesizes it with a warning (the EXP-15 behaviour).
    fn disposal_waste(&mut self, p: &Process, section: &str) -> (String, u64, String) {
        let w = p.waste_raw.clone().unwrap_or_default();
        let clause = split_top_level(item_list_part(&w), &[';'])
            .into_iter()
            .next()
            .unwrap_or_default();
        let halves: Vec<&str> = clause.split('→').collect();
        let (left, right) = (
            halves.first().copied().unwrap_or(""),
            halves.get(1).copied().unwrap_or(""),
        );
        let qty = parse_quantities(left, self.strict)
            .into_iter()
            .find(|q| q.unit == "g");
        let mass = qty.as_ref().map(|q| q.value).unwrap_or(0);
        let name_part: String = words(&strip_parens(left)).join(" ");
        let ident = match self.resolve(left) {
            Some(e) => e.ident,
            None => {
                if self.strict {
                    self.err(
                        p.waste_line,
                        section,
                        format!(
                            "waste '{}' has no §3 row: waste is a resource like any other (R1), and a §5 waste line with no §3 row fails speccheck (A3)",
                            left.trim()
                        ),
                    );
                    camel(&name_part)
                } else {
                    let ident = camel(&name_part);
                    self.warn(
                        p.waste_line,
                        section,
                        format!(
                            "waste '{}' has no §3 row; a container type {} ({}) was synthesized from this line — the template requires a §3 row for every waste product (A3)",
                            left.trim(),
                            ident,
                            unit_name(qty.as_ref().map(|q| q.unit.as_str()).unwrap_or("g"))
                        ),
                    );
                    self.synthesized.push(TypeEntry {
                        ident: ident.clone(),
                        resource: name_part.clone(),
                        state: None,
                        canonical_state: None,
                        canonical_res: vec![],
                        alias_only: false,
                        required: vec![],
                        stems: words(&name_part),
                        quantities: vec![Qty {
                            value: mass,
                            unit: "g".into(),
                        }],
                        role: Role::Container {
                            extra_units: vec![],
                            slot_unit: "g".into(),
                        },
                        line: p.waste_line,
                    });
                    ident
                }
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
        let sink = sink.unwrap_or_default();
        (ident, mass, sink)
    }

    // ── validation ──────────────────────────────────────────────────────

    fn validate(&mut self) {
        let strict = self.strict;
        // §2: the Ids field (A6) and in-file id discipline (F-053)
        let reqs = self.spec.requirements.clone();
        if strict {
            match &self.spec.ids_field {
                None => {
                    self.err(
                        self.spec.section2_line.max(1),
                        "§2",
                        "no '**Ids:** REQ-0NN–REQ-0MM' field: ids are allocated from the workspace sequence and stated here (A6, F-053; check the latest trace.sh report for the highest id in use)"
                            .into(),
                    );
                }
                Some(f) => {
                    let first = reqs.first().map(|r| r.spec_id);
                    let last = reqs.last().map(|r| r.spec_id);
                    if first != Some(f.first) || last != Some(f.last) {
                        self.err(
                            f.line,
                            "§2",
                            format!(
                                "the Ids field says REQ-{:03}–REQ-{:03} but the bullets run REQ-{}–REQ-{} — the field states exactly the allocated range (A6)",
                                f.first,
                                f.last,
                                first.map(|n| format!("{n:03}")).unwrap_or("?".into()),
                                last.map(|n| format!("{n:03}")).unwrap_or("?".into())
                            ),
                        );
                    }
                }
            }
            for pair in reqs.windows(2) {
                if pair[1].spec_id != pair[0].spec_id + 1 {
                    self.err(
                        pair[1].line,
                        "§2",
                        format!(
                            "REQ-{:03} follows REQ-{:03}: ids are sequential, never reused (R10, F-053)",
                            pair[1].spec_id, pair[0].spec_id
                        ),
                    );
                }
            }
        }
        // duplicate ids within the file (both modes)
        for (i, r) in reqs.iter().enumerate() {
            if reqs[..i].iter().any(|o| o.spec_id == r.spec_id) {
                self.err(
                    r.line,
                    "§2",
                    format!("REQ-{:03} is defined twice in this specification (F-053)", r.spec_id),
                );
            }
        }

        let procs = self.spec.processes.clone();
        for p in &procs {
            let section = format!("§5 P{}", p.id);
            // balance arithmetic — the spec's own conservation check
            for b in &p.balances {
                if !b.symbolic && !b.lhs.is_empty() && !b.rhs.is_empty() {
                    let l: u64 = b.lhs.iter().sum();
                    let r: u64 = b.rhs.iter().sum();
                    if l != r {
                        let fmt_side = |v: &Vec<u64>| {
                            v.iter()
                                .map(|n| lit(*n).replace('_', " "))
                                .collect::<Vec<_>>()
                                .join(" + ")
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
                }
                match b.mark {
                    BalanceMark::Unmarked if strict => {
                        self.err(
                            b.line,
                            &section,
                            format!(
                                "balance '{}' is not marked: every Balances clause ends in (assert) or (structural) — an unmarked clause is a speccheck error, not a judgement call (A1, F-062)",
                                b.raw
                            ),
                        );
                    }
                    BalanceMark::Unmarked => {
                        self.warn(
                            b.line,
                            &section,
                            format!(
                                "balance '{}' is not marked (assert) or (structural) — the generator guesses: identical single terms on both sides ⇒ structural, otherwise assert (A1)",
                                b.raw
                            ),
                        );
                    }
                    BalanceMark::Assert if b.symbolic && b.note.is_empty() => {
                        self.warn(
                            b.line,
                            &section,
                            format!(
                                "assert balance '{}' has terms with no stated numbers, so it cannot be verified at the document — only asserts carry stated numbers on both sides (A1); if the assert lives in code, say where: '(assert, per instantiation)'",
                                b.raw
                            ),
                        );
                    }
                    _ => {}
                }
            }
            // time clause vs Actor draws
            if let (Some(td), Some(draw)) = (&p.time_draw, p.draws_ms) {
                if let Some(ms) = td.ms {
                    if ms != draw {
                        self.err(
                            td.line,
                            &section,
                            format!(
                                "the Balances time clause says {} ms but the Actor line draws {} ms — the two must agree",
                                lit(ms).replace('_', " "),
                                lit(draw).replace('_', " ")
                            ),
                        );
                    }
                }
            }
            // Satisfies ids must exist in §2
            for id in &p.satisfies {
                let known = reqs.iter().any(|r| r.spec_id == *id || r.impl_id == *id);
                if !known {
                    self.err(
                        if p.satisfies_line > 0 { p.satisfies_line } else { p.line },
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
            // Waste routing (A2): mandatory where waste exists, naming one
            // of the two shapes
            if p.waste_raw.is_some() {
                match &p.waste_routing {
                    None if strict => {
                        self.err(
                            p.waste_line,
                            &section,
                            "no 'Waste routing:' field: every process with waste carries one, naming 'consumer parameter' or 'routed by the flow' (A2, hole U-10, F-062)"
                                .into(),
                        );
                    }
                    None => {
                        self.warn(
                            p.waste_line,
                            &section,
                            "no 'Waste routing' field — the generator defaults to the weak reading (the flow routes loose outputs); the template asks the author to choose (A2)"
                                .into(),
                        );
                    }
                    Some(r) if strict => {
                        let low = r.to_lowercase();
                        if !low.contains("consumer parameter")
                            && !low.contains("routed by the flow")
                            && !low.contains("flow-routed")
                        {
                            self.err(
                                p.waste_routing_line,
                                &section,
                                format!(
                                    "Waste routing '{r}' names neither shape: write 'consumer parameter' (the process takes its consumer as a requirement-bounded parameter) or 'routed by the flow' (A2)"
                                ),
                            );
                        }
                    }
                    Some(_) => {}
                }
            }
        }
        // §6: flow orders (A8) reference real processes
        let flows = self.spec.flows.clone();
        if strict && !flows.has_orders_field && !procs.is_empty() {
            self.err(
                flows.line.max(1),
                "§6",
                "no '**Orders:**' field: §6 states at least two valid orders as comma-separated lists of §5 process ids, and the implementer proves both compile (A8, R9)"
                    .into(),
            );
        }
        for order in &flows.orders {
            for id in order {
                if !procs.iter().any(|p| p.id == *id) {
                    self.err(
                        if flows.orders_line > 0 { flows.orders_line } else { flows.line },
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
        Role::Container {
            extra_units,
            slot_unit,
        } => extra_units.len() + usize::from(!slot_unit.is_empty()),
        _ => 0,
    }
}

/// "pot of tea" + "teapot" → PotOfTea; "filled" + "kettle" → FilledKettle.
/// Mechanical naming, documented as such (F-062): a human implementer may
/// choose differently; one-shot scaffolding never round-trips.
fn state_ident(state: &str, res_last: &str) -> String {
    let sws = words(state);
    let overlap = sws
        .iter()
        .any(|w| (w.len() >= 3 && res_last.contains(w.as_str())) || w.contains(res_last));
    if overlap {
        camel(state)
    } else {
        format!("{}{}", camel(state), camel(res_last))
    }
}

/// "dry mass 3 g each; spent mass 12 g each (9 g absorbed water)"
fn find_state_mass(characteristics: &str, state: &str, strict: bool) -> Option<u64> {
    let low = characteristics.to_lowercase();
    let key = format!("{} mass", state.to_lowercase());
    let i = low.find(&key)?;
    parse_quantities(&characteristics[i + key.len()..], strict)
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
    // byte-wise (pat is ASCII), so multi-byte prose never splits a char
    let bytes = s.as_bytes();
    let p = pat.as_bytes();
    let mut depth = 0usize;
    let mut i = 0;
    while i + p.len() <= bytes.len() {
        match bytes[i] {
            b'(' => depth += 1,
            b')' => depth = depth.saturating_sub(1),
            _ => {}
        }
        if depth == 0 && &bytes[i..i + p.len()] == p {
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
