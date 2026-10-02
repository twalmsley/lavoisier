//! The core R17 enforcement: a flow cannot treat the fallible process's
//! result as if it were the success bundle — the `Result` must be matched,
//! so the failure arm must be written. (trybuild inherits the test-support
//! feature, F-003/F-004, so fixtures may appear in these cases.)

use exp10_failure_modes::model::boundary::{outcome_success, supply_drill_bit, supply_plate};
use exp10_failure_modes::model::processes::drill_fallible;
use exp10_failure_modes::model::DrillOk;
use model_core::common::boundary::new_person;

fn main() {
    let ok: DrillOk<4000, 440, 10, 1000> =
        drill_fallible::<1000, 4000, 5000, 450, 440, 10, 430, 20>(
            new_person::<5000>(),
            supply_drill_bit(),
            supply_plate::<450>(),
            outcome_success(),
        );
    let _ = ok;
}
