//! Variability enters only at the boundary (candidate R17, R12 spirit): an
//! outcome token cannot be forged by downstream code — its kind is a private
//! field of a sealed struct, so the only outcome sources are the boundary
//! constructors and test-support fixtures.

use exp10_failure_modes::model::DrillOutcome;

fn main() {
    let _forged = DrillOutcome {
        kind: todo!(),
        _seal: (),
    };
}
