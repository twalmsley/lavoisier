// R9/F-023: a flow that uses a product that is never produced. `fasten`
// needs the drilled processing state, and the plates were never drilled —
// `Plate` and `DrilledPlate` are distinct types, so the broken flow is a
// compile error reading "expected `DrilledPlate`, found `Plate`": these
// plates have not been drilled yet.
use model_core::nat::aliases::N4;
use pilot_workshop::catalogue::boundary::full_box;
use pilot_workshop::catalogue::{EmptyBoltBox, FasteningBolt};
use pilot_workshop::resources::Assembly;
use pilot_workshop::resources::boundary::supply_sheet;
use pilot_workshop::resources::processes::{cut, fasten};

fn main() {
    let sheet = supply_sheet::<2000>();
    let (p1, p2, _swarf) = cut::<2000, 900, 900, 200>(sheet);
    let bolts = full_box::<FasteningBolt, N4>();

    // The drilling step is missing from this flow:
    let (_assembly, _rest): (Assembly<FasteningBolt, 1800>, EmptyBoltBox) = fasten(p1, p2, bolts);
}
