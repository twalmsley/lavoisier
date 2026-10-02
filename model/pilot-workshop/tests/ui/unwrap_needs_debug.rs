// R17/F-047: `.unwrap()` on the fallible process's result does not compile,
// because the failure bundle carries sealed resources and therefore no
// `Debug` impl. The panicking shortcut past the failure arm is closed at
// type-check time: the flow must `match` and account for both bundles (see
// the error-reading guide).
use model_core::common::boundary::{new_person, qualify};
use pilot_workshop::characteristics::DrillingCert;
use pilot_workshop::resources::boundary::{outcome_success, supply_drill_bit, supply_guard, supply_plate};
use pilot_workshop::resources::processes::{drill_holes_fallible, fit_guard};

fn main() {
    let operator = qualify::<DrillingCert, 5000>(new_person::<5000>());
    let result = drill_holes_fallible::<900, 880, 20, 880, 20, _, _>(
        operator,
        fit_guard(supply_guard()),
        supply_drill_bit(),
        supply_plate::<900>(),
        outcome_success(),
    );
    let _ok = result.unwrap();
}
