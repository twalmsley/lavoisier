//! The parsed form of a SPEC_TEMPLATE.md specification — only what the
//! generator consumes. Every node keeps its 1-based source line for the
//! error story.

/// A spec-validation error: the modeller-facing error class the generator
/// owns outright (vs Rust errors inside emitted holes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecError {
    /// 1-based line in the spec file.
    pub line: usize,
    /// Which section the error belongs to ("§5 P3", "§2", …).
    pub section: String,
    /// Modeller-phrased message.
    pub message: String,
}

impl std::fmt::Display for SpecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SPEC.md:{} {}: {}",
            self.line, self.section, self.message
        )
    }
}

/// A §2 requirement bullet.
#[derive(Debug, Clone)]
pub struct Requirement {
    /// The id as numbered in the spec (REQ-001 → 1).
    pub spec_id: u32,
    /// The id the implementation must use (workspace-global remap, F-053);
    /// equals `spec_id` unless §2 carries an implementation note.
    pub impl_id: u32,
    /// The requirement sentence, verbatim (without the id).
    pub text: String,
    pub line: usize,
}

/// §3 resource kinds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    Discrete,
    Continuous { waste: bool },
    Reusable { note: Option<String> },
    Product,
}

/// One parsed quantity, e.g. "1500 g" or "500 000 J".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Qty {
    pub value: u64,
    /// Base-unit token as written: "g", "J", "ms", "bags", …
    pub unit: String,
}

/// One state in a §3 "States" chain, e.g. `boiling(1500 g, 500 000 J)`.
#[derive(Debug, Clone)]
pub struct ResState {
    pub name: String,
    pub quantities: Vec<Qty>,
}

/// A §3 table row.
#[derive(Debug, Clone)]
pub struct Resource {
    pub name: String,
    pub kind: Kind,
    pub characteristics: String,
    pub quantity_raw: String,
    pub states: Vec<ResState>,
    pub line: usize,
}

/// A §4 boundary table row (inputs or outputs).
#[derive(Debug, Clone)]
pub struct BoundaryRow {
    pub what: String,
    pub via: String,
    pub capacity: String,
    pub status: String,
    pub line: usize,
}

/// How a Balances clause is checked (template: "(assert)"/"(structural)").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BalanceMark {
    Assert,
    Structural,
    /// The spec did not say — the generator must guess (a template gap:
    /// CS-1 §8 feedback item 5).
    Unmarked,
}

/// One clause of a §5 Balances line.
#[derive(Debug, Clone)]
pub struct Balance {
    pub dimension: String,
    pub lhs: Vec<u64>,
    pub rhs: Vec<u64>,
    pub mark: BalanceMark,
    pub raw: String,
    pub line: usize,
}

/// A "time N ms → History" clause of a Balances line.
#[derive(Debug, Clone)]
pub struct TimeDraw {
    pub ms: u64,
    pub line: usize,
}

/// A §5 process block.
#[derive(Debug, Clone, Default)]
pub struct Process {
    pub id: u32,
    pub title: String,
    pub line: usize,
    pub actors_raw: String,
    /// "person (draws N ms)" parsed out of the Actor line.
    pub draws_ms: Option<u64>,
    /// The Actor line says "no person".
    pub no_person: bool,
    pub consumes_raw: String,
    pub produces_raw: String,
    pub waste_raw: Option<String>,
    /// The template's optional "Waste routing" field (absent in CS-1).
    pub waste_routing: Option<String>,
    pub balances: Vec<Balance>,
    pub time_draw: Option<TimeDraw>,
    /// Impl-side requirement ids (already remapped).
    pub satisfies: Vec<u32>,
    pub consumes_line: usize,
    pub produces_line: usize,
    pub waste_line: usize,
}

/// §6, kept mostly raw: the orders are the mechanical part.
#[derive(Debug, Clone, Default)]
pub struct Flows {
    /// Each stated valid order as a list of process ids, e.g. [1,2,3,4,5].
    pub orders: Vec<Vec<u32>>,
    pub raw: Vec<String>,
    pub line: usize,
}

/// The whole parsed spec.
#[derive(Debug, Clone, Default)]
pub struct Spec {
    pub system_name: String,
    pub requirements: Vec<Requirement>,
    pub resources: Vec<Resource>,
    pub inputs: Vec<BoundaryRow>,
    pub outputs: Vec<BoundaryRow>,
    pub processes: Vec<Process>,
    pub flows: Flows,
}

impl Default for Kind {
    fn default() -> Self {
        Kind::Discrete
    }
}
