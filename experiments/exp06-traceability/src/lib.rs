//! EXP-06: Requirements traceability report.
//!
//! A small model in the R10 pattern: requirement traits `REQ-001`..`REQ-003`,
//! types and processes satisfying them, and tagged tests, so that `trace.sh`
//! can extract a traceability table mechanically.
//!
//! Note for the trap below: REQ-000 does not exist; inner doc comments (`//!`)
//! are module prose, not requirement definitions, and must not be picked up.

// ---------------------------------------------------------------------------
// Characteristic marker traits (R6)
// ---------------------------------------------------------------------------

pub trait M8 {}
pub trait Bolt {}
pub trait Steel {}
pub trait Length15mm {}
pub trait Plate {}
pub trait Drilled {}

/// A consumer at the system boundary (simplified from R12).
pub trait Consumer<In> {
    type Next;
    fn consume(self, item: In) -> Self::Next;
}

// ---------------------------------------------------------------------------
// Requirement traits (R10 pattern: named trait, doc comment starting with the
// ID, characteristics as supertraits, blanket impl)
// ---------------------------------------------------------------------------

/// REQ-001: Fastening bolts must be M8 steel bolts, 15 mm long.
pub trait Req001FasteningBolt: M8 + Bolt + Steel + Length15mm {}
impl<T: M8 + Bolt + Steel + Length15mm> Req001FasteningBolt for T {}

/// REQ-002: Structural plates must be drilled steel plates.
pub trait Req002StructuralPlate: Plate + Steel + Drilled {}
impl<T: Plate + Steel + Drilled> Req002StructuralPlate for T {}

/// REQ-003: Swarf produced by drilling must be handed to a waste consumer.
///
/// Deliberately left with **no verifying test**, so the report must warn.
pub trait Req003SwarfDisposal: Consumer<Swarf> {}
impl<T: Consumer<Swarf>> Req003SwarfDisposal for T {}

// ---------------------------------------------------------------------------
// Resource types. Private constructors (R1); `new` is pub(crate) so this
// crate's own tests and boundary code can build fixtures.
// ---------------------------------------------------------------------------

/// A hex-head M8 steel bolt, 15 mm long.
/// Satisfies: REQ-001
pub struct HexBoltM8(());

impl HexBoltM8 {
    #[cfg(test)]
    pub(crate) fn new() -> Self {
        HexBoltM8(())
    }
}

impl M8 for HexBoltM8 {}
impl Bolt for HexBoltM8 {}
impl Steel for HexBoltM8 {}
impl Length15mm for HexBoltM8 {}

// Compile-checked backing for the `Satisfies:` tag above: if HexBoltM8 ever
// stops satisfying REQ-001, this stops compiling.
const fn assert_req001<T: Req001FasteningBolt>() {}
const _: () = assert_req001::<HexBoltM8>();

/// A drilled steel plate.
/// Satisfies: REQ-002
pub struct DrilledSteelPlate(());

impl DrilledSteelPlate {
    #[cfg(test)]
    pub(crate) fn new() -> Self {
        DrilledSteelPlate(())
    }
}

impl Plate for DrilledSteelPlate {}
impl Steel for DrilledSteelPlate {}
impl Drilled for DrilledSteelPlate {}

const fn assert_req002<T: Req002StructuralPlate>() {}
const _: () = assert_req002::<DrilledSteelPlate>();

/// Drilling waste. See also REQ-003 for how it must be disposed of.
/// (Trap: that prose mention of the ID must not count as a definition,
/// a satisfaction, or a verification.)
pub struct Swarf(());

impl Swarf {
    #[cfg(test)]
    pub(crate) fn new() -> Self {
        Swarf(())
    }
}

/// A bin for swarf, with room for one more item (kept trivial on purpose).
/// Satisfies: REQ-003
pub struct SwarfBin(());

impl SwarfBin {
    #[cfg(test)]
    pub(crate) fn new() -> Self {
        SwarfBin(())
    }
}

/// A swarf bin that is full.
pub struct FullSwarfBin {
    _contents: Swarf,
}

impl Consumer<Swarf> for SwarfBin {
    type Next = FullSwarfBin;
    fn consume(self, item: Swarf) -> FullSwarfBin {
        FullSwarfBin { _contents: item }
    }
}

const fn assert_req003<T: Req003SwarfDisposal>() {}
const _: () = assert_req003::<SwarfBin>();

// ---------------------------------------------------------------------------
// Processes (R1): they satisfy requirements by *using* the requirement traits
// as bounds.
// ---------------------------------------------------------------------------

/// The product of fastening two plates with a bolt.
pub struct Assembly<B, P> {
    _bolt: B,
    _top: P,
    _bottom: P,
}

/// Fasten two plates together with one bolt.
/// Satisfies: REQ-001, REQ-002
pub fn fasten<B: Req001FasteningBolt, P: Req002StructuralPlate>(
    bolt: B,
    top: P,
    bottom: P,
) -> Assembly<B, P> {
    // Verifies: REQ-003 -- TRAP: an ordinary `//` comment inside a function
    // body. It must NOT count as a verification.
    Assembly {
        _bolt: bolt,
        _top: top,
        _bottom: bottom,
    }
}

//// Verifies: REQ-003 -- TRAP: four slashes is an ordinary comment in Rust,
//// not a doc comment. It must NOT count.

/* Verifies: REQ-003 -- TRAP: a block comment. It must NOT count. */

/// Hand swarf to a disposal consumer.
/// Satisfies: REQ-003
pub fn dispose_swarf<C: Req003SwarfDisposal>(swarf: Swarf, bin: C) -> C::Next {
    let _trap = "Verifies: REQ-003"; // TRAP: a string literal must NOT count.
    bin.consume(swarf)
}

// ---------------------------------------------------------------------------
// Unit tests, tagged per R10: `/// Verifies: REQ-...` in the doc comment.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Fastening accepts a conforming bolt and returns everything as an
    /// assembly.
    /// Verifies: REQ-001
    #[test]
    fn fasten_accepts_conforming_bolt() {
        let assembly = fasten(
            HexBoltM8::new(),
            DrilledSteelPlate::new(),
            DrilledSteelPlate::new(),
        );
        let _keep: Assembly<HexBoltM8, DrilledSteelPlate> = assembly;
    }

    /// Edge case: several IDs on one line, with a trailing full stop.
    /// Verifies: REQ-001, REQ-002.
    #[test]
    fn fasten_joins_two_plates() {
        let _assembly = fasten(
            HexBoltM8::new(),
            DrilledSteelPlate::new(),
            DrilledSteelPlate::new(),
        );
    }

    /// Edge case: extra whitespace around the tag and between the IDs.
    ///    Verifies:   REQ-002 ,  REQ-001
    #[test]
    fn plates_are_drilled_steel() {
        const fn is_structural<P: Req002StructuralPlate>() {}
        is_structural::<DrilledSteelPlate>();
    }

    /// Edge case: lowercase tag. The convention is case-sensitive, so this
    /// line must NOT count (REQ-002 is already verified by other tests, so
    /// the report stays correct either way; this line shows the strictness).
    /// verifies: REQ-002
    #[test]
    fn full_bin_is_a_distinct_type() {
        let bin = SwarfBin::new();
        let full: FullSwarfBin = dispose_swarf(Swarf::new(), bin);
        let _keep = full;
        // NOTE: this test exercises REQ-003's machinery but deliberately does
        // not claim `Verifies: REQ-003`, so REQ-003 stays unverified and the
        // report must warn about it.
    }
}
