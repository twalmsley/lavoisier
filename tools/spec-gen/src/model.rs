//! The parsed form of a SPEC_TEMPLATE.md specification — only what the
//! validator and generator consume. Every node keeps its 1-based source line
//! for the error story (R22: errors are phrased at the document, with §/line
//! references — "fix the specification, not the model", F-062).

/// A spec-validation finding. `speccheck` reports two grades and both fail
/// the gate (R22: what the tool cannot verify is a warning that fails the
/// gate, like trace.sh's); only errors fail in `--lenient` migration mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecError {
    /// 1-based line in the spec file.
    pub line: usize,
    /// Which section the finding belongs to ("§5 P3", "§2", …).
    pub section: String,
    /// Modeller-phrased message.
    pub message: String,
}

impl std::fmt::Display for SpecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SPEC.md:{} {}: {}", self.line, self.section, self.message)
    }
}

/// A §2 requirement bullet.
#[derive(Debug, Clone)]
pub struct Requirement {
    /// The id as numbered in the spec (REQ-006 → 6). Since amendment A6 the
    /// bullets carry the workspace-global ids directly.
    pub spec_id: u32,
    /// The id the implementation must use. Equals `spec_id` in strict mode;
    /// in `--lenient` mode a pre-A6 prose remap note may shift it (F-053).
    pub impl_id: u32,
    /// The requirement sentence, verbatim (without the id).
    pub text: String,
    pub line: usize,
}

/// §3 resource kinds. The four template kinds drive generation; anything
/// else (evidence token, outcome token, nested container, organisation, …)
/// is kept verbatim as `Other` — valid in a specification, registered for
/// name closure, but scaffolded only as far as the spec determines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    Discrete,
    Continuous { waste: bool },
    Reusable { note: Option<String> },
    Product,
    Other(String),
}

/// One parsed quantity, e.g. "1500 g" or "550_000 J".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Qty {
    pub value: u64,
    /// Base-unit token as written: "g", "J", "ms", "p", "ec", "bags", …
    pub unit: String,
}

/// One state in a §3 "States" cell. In strict mode a state is exactly a
/// backticked canonical identifier (A4), optionally followed by its
/// parenthesised quantities (A10): `` `boiling` (1_500 g, 500_000 J) ``.
#[derive(Debug, Clone)]
pub struct ResState {
    pub name: String,
    pub quantities: Vec<Qty>,
}

/// A §3 table row.
#[derive(Debug, Clone)]
pub struct Resource {
    /// The Resource cell's full text (prose allowed around the identifier).
    pub name: String,
    /// The backticked canonical identifiers the cell opens with (A4).
    /// Usually one; a shared row ("`Cup` / `Teapot` / `Tray`") lists several.
    pub idents: Vec<String>,
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

/// How a Balances clause is checked (template A1: every clause ends in
/// "(assert…)" or "(structural…)").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BalanceMark {
    Assert,
    Structural,
    /// The spec did not say — an error in strict mode (A1, F-062).
    Unmarked,
}

/// One clause of a §5 Balances line.
#[derive(Debug, Clone)]
pub struct Balance {
    pub dimension: String,
    /// Each side as evaluated term values; a term with no number at all is
    /// recorded in `symbolic` instead.
    pub lhs: Vec<u64>,
    pub rhs: Vec<u64>,
    /// True when either side contained a term with no parsable number
    /// ("mass 1900 + tube = wheel").
    pub symbolic: bool,
    pub mark: BalanceMark,
    /// Whatever followed the mark inside its parenthetical ("per
    /// instantiation", "kit draw", "Ok arm") — the spec's own statement of
    /// where a document-unverifiable assert lives.
    pub note: String,
    pub raw: String,
    pub line: usize,
}

/// A "time … → History" (or "→ `h_a`") clause of a Balances line.
#[derive(Debug, Clone)]
pub struct TimeDraw {
    /// The stated amount, when the clause carries one.
    pub ms: Option<u64>,
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
    /// R17 variant fields ("Produces (Ok):", "Produces (Fail):"), each kept
    /// for its own name-closure check.
    pub produces_variants: Vec<(String, String, usize)>,
    pub waste_raw: Option<String>,
    /// The template's "Waste routing" field (mandatory where waste exists, A2).
    pub waste_routing: Option<String>,
    pub waste_routing_line: usize,
    pub balances: Vec<Balance>,
    pub balances_line: usize,
    pub time_draw: Option<TimeDraw>,
    /// Requirement ids named on the Satisfies line.
    pub satisfies: Vec<u32>,
    pub satisfies_line: usize,
    pub consumes_line: usize,
    pub produces_line: usize,
    pub waste_line: usize,
}

/// §6, kept mostly raw: the Orders field is the mechanical part (A8).
#[derive(Debug, Clone, Default)]
pub struct Flows {
    /// Each stated valid order as a list of process ids, from the
    /// "**Orders:**" field (A8), e.g. [1,2,3,4,5].
    pub orders: Vec<Vec<u32>>,
    /// True when §6 carries the "**Orders:**" field.
    pub has_orders_field: bool,
    pub orders_line: usize,
    pub raw: Vec<String>,
    pub line: usize,
}

/// The §2 "**Ids:**" allocation field (A6): the stated workspace-global
/// range, e.g. REQ-006–REQ-009.
#[derive(Debug, Clone)]
pub struct IdsField {
    pub first: u32,
    pub last: u32,
    pub line: usize,
}

/// The whole parsed spec.
#[derive(Debug, Clone, Default)]
pub struct Spec {
    pub system_name: String,
    pub requirements: Vec<Requirement>,
    pub ids_field: Option<IdsField>,
    pub section2_line: usize,
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
