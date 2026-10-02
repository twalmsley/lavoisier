// R6/R10: the wrong bolt is rejected by REQ-001. The box is full of M6 brass
// bolts; `fasten`'s bound requires REQ-001 fastening bolts (M8 steel, 15 mm),
// so the flow fails at the fasten call with one E0277 per missing
// characteristic. The top line of each is REQ-001's own on_unimplemented
// message (R10 rule 8, F-044): the failing marker's message is NOT shown when
// the marker fails as a supertrait obligation — only the root requirement
// trait's is.
use model_core::common::boundary::{new_person, qualify};
use model_core::nat::aliases::N4;
use pilot_workshop::catalogue::boundary::full_box;
use pilot_workshop::catalogue::{Bolt, Brass, L15, SizeM6};
use pilot_workshop::characteristics::DrillingCert;
use pilot_workshop::resources::Assembly;
use pilot_workshop::resources::boundary::{supply_drill, supply_guard, supply_sheet};
use pilot_workshop::resources::processes::{cut, drill_holes, fasten, fit_guard};

fn main() {
    let sheet = supply_sheet::<2000>();
    let operator = qualify::<DrillingCert, 10_000>(new_person::<10_000>());
    let drill = supply_drill();
    let guard = fit_guard(supply_guard());
    let (p1, p2, _swarf) = cut::<2000, 900, 900, 200>(sheet);
    let (operator, drill, guard, d1, _s1, _l1) =
        drill_holes::<2000, 8000, 10_000, 900, 880, 20, _>(operator, drill, guard, p1);
    let (_operator, _drill, _guard, d2, _s2, _l2) =
        drill_holes::<2000, 6000, 8000, 900, 880, 20, _>(operator, drill, guard, p2);

    // M6 brass is not what REQ-001 calls for:
    let wrong_bolts = full_box::<Bolt<SizeM6, Brass, L15>, N4>();
    let (_assembly, _rest): (Assembly<Bolt<SizeM6, Brass, L15>, 1760>, _) =
        fasten(d1, d2, wrong_bolts);
}
