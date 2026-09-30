//! Integration tests in a separate file, so `trace.sh` has to sweep more
//! than `src/`.

use exp06_traceability::{DrilledSteelPlate, HexBoltM8, Req001FasteningBolt, Req002StructuralPlate};

/// The requirement trait is usable as a bound from outside the crate.
/// Verifies: REQ-001
#[test]
fn req001_bound_holds_downstream() {
    const fn takes<T: Req001FasteningBolt>() {}
    takes::<HexBoltM8>();
}

/// Edge case: a tag naming a requirement that does not exist. The report
/// should flag REQ-999 as unknown rather than silently inventing a row.
/// Verifies: REQ-999
#[test]
fn unknown_requirement_id_is_flagged() {
    const fn takes<T: Req002StructuralPlate>() {}
    takes::<DrilledSteelPlate>();
}
