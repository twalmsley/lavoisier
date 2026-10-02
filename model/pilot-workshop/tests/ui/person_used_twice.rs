// R2: the same certified operator moved into two drilling processes at once.
// The intent is two "concurrent" drilling steps sharing one operator; the
// compiler must refuse, because the operator is already busy ("use of moved
// value" = "this resource is already in use by another process", F-025). Two
// separate drills and guards, so the contended resource in the error is
// unambiguously the operator.
use model_core::common::boundary::{new_person, qualify};
use pilot_workshop::characteristics::DrillingCert;
use pilot_workshop::resources::boundary::{supply_drill, supply_guard, supply_sheet};
use pilot_workshop::resources::processes::{cut, drill_holes, fit_guard};

fn main() {
    let operator = qualify::<DrillingCert, 10_000>(new_person::<10_000>());
    let drill_1 = supply_drill();
    let drill_2 = supply_drill();
    let guard_1 = fit_guard(supply_guard());
    let guard_2 = fit_guard(supply_guard());
    let sheet = supply_sheet::<2000>();
    let (p1, p2, _swarf) = cut::<2000, 900, 900, 200>(sheet);

    let (_operator_a, _drill_1, _guard_1, _d1, _s1, _l1) =
        drill_holes::<2000, 8000, 10_000, 900, 880, 20, _>(operator, drill_1, guard_1, p1);
    let (_operator_b, _drill_2, _guard_2, _d2, _s2, _l2) =
        drill_holes::<2000, 8000, 10_000, 900, 880, 20, _>(operator, drill_2, guard_2, p2);
}
