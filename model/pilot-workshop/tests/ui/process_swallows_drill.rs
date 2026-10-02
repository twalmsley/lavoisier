// R2/F-002: a badly modelled process that swallows the drill (takes it by
// value, never returns it). NOTE the known affine-types gap: the swallowing
// process itself compiles without complaint — dropping a moved-in parameter
// is legal Rust. The error appears only at the CALLER, the first time the
// flow tries to use the drill again; rustc's "consider borrowing" fix-it must
// be ignored (R2 forbids it, F-007) — the real fix is to return the drill.
use model_core::common::Labour;
use model_core::common::boundary::{new_person, qualify};
use pilot_workshop::characteristics::DrillingCert;
use pilot_workshop::resources::boundary::{supply_drill, supply_guard, supply_sheet};
use pilot_workshop::resources::processes::{cut, drill_holes, fit_guard};
use pilot_workshop::resources::{
    Drill, DrilledPlate, DrillingOperator, FittedGuard, Plate, Swarf,
};

/// A badly modelled process: takes the drill but never gives it back. This
/// function compiles without complaint.
fn bad_drill_holes(
    operator: DrillingOperator<10_000>,
    drill: Drill,
    guard: FittedGuard,
    plate: Plate<900>,
) -> (
    DrillingOperator<8000>,
    FittedGuard,
    DrilledPlate<880>,
    Swarf<20>,
    Labour<2000>,
) {
    let (operator, _swallowed_drill, guard, drilled, swarf, labour) =
        drill_holes::<2000, 8000, 10_000, 900, 880, 20, _>(operator, drill, guard, plate);
    (operator, guard, drilled, swarf, labour)
}

fn main() {
    let sheet = supply_sheet::<2000>();
    let operator = qualify::<DrillingCert, 10_000>(new_person::<10_000>());
    let drill = supply_drill();
    let guard = fit_guard(supply_guard());
    let (p1, p2, _swarf) = cut::<2000, 900, 900, 200>(sheet);

    let (operator, guard, _d1, _s1, _l1) = bad_drill_holes(operator, drill, guard, p1);
    // The drill is gone: the next process in the flow cannot have it.
    let (_operator, _drill, _guard, _d2, _s2, _l2) =
        drill_holes::<2000, 6000, 8000, 900, 880, 20, _>(operator, drill, guard, p2);
}
