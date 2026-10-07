// REQ-016: burnt milk is never served — `build_flat_white` accepts only the
// `SteamedMilk` state (one type per state, R9/F-023), so burnt milk in the
// milk position is a crisp E0308: "expected `SteamedMilk<150>`, found
// `BurntMilk<150>`". (In production burnt milk never even exists loose —
// `steam_milk` feeds it to the drain inside the process — so this case needs
// the test-support fixture to conjure one at all.)
use cs3_cafe_orders::characteristics::MachineTraining;
use cs3_cafe_orders::resources::BurntMilk;
use cs3_cafe_orders::resources::EspressoShot;
use cs3_cafe_orders::resources::boundary::new_cup;
use cs3_cafe_orders::resources::processes::build_flat_white;
use model_core::common::boundary::{new_person, qualify};

fn main() {
    let barista = qualify::<MachineTraining, 600_000>(new_person::<600_000>());
    let shot = EspressoShot::<36>::test_fixture();
    let burnt = BurntMilk::<150>::test_fixture();
    let (barista, drink) =
        build_flat_white::<36, 150, 186, 600_000>(barista, shot, new_cup(), burnt);
}
