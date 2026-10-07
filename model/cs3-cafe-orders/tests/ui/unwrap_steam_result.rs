// R17/F-047: `.unwrap()` on the fallible steaming attempt's result does not
// compile, because the failure bundle carries sealed resources and therefore
// no `Debug` impl. The panicking shortcut past the failure arm is closed at
// type-check time: the flow must `match` and account for both bundles (see
// the error-reading guide).
use cs3_cafe_orders::characteristics::MachineTraining;
use cs3_cafe_orders::resources::boundary::{new_drain, steam_goes_well, stock_milk_bottle};
use cs3_cafe_orders::resources::processes::{draw_milk, steam_milk};
use model_core::common::boundary::{new_person, qualify};

fn main() {
    let barista = qualify::<MachineTraining, 600_000>(new_person::<600_000>());
    let (milk, bottle) = draw_milk::<150, 150, 300>(stock_milk_bottle());
    let result = steam_milk::<150, 150, 150, 600_000, _>(barista, milk, steam_goes_well(), new_drain());
    let _ok = result.unwrap();
}
