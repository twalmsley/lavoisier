// R18: a guard that exists but has not been fitted is the unsafe state: only
// `FittedGuard` carries the `Fitted` characteristic, so the unfitted
// `MachineGuard` fails REQ-005 (E0277) — distinct from "no guard at all"
// (E0061), F-049. The top line is REQ-005's REQ-phrased message (F-044).
use model_core::common::boundary::{new_person, qualify};
use pilot_workshop::characteristics::DrillingCert;
use pilot_workshop::resources::boundary::{outcome_success, supply_drill_bit, supply_guard, supply_plate};
use pilot_workshop::resources::processes::drill_holes_fallible;

fn main() {
    let operator = qualify::<DrillingCert, 5000>(new_person::<5000>());
    let guard = supply_guard(); // never fitted
    let r = drill_holes_fallible::<900, 880, 20, 880, 20, _, _>(
        operator,
        guard,
        supply_drill_bit(),
        supply_plate::<900>(),
        outcome_success(),
    );
}
