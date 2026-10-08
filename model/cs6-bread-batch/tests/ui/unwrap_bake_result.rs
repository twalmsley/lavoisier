// R17/F-047: `.unwrap()` on the fallible bake's result does not compile,
// because the failure bundle carries sealed resources and therefore no
// `Debug` impl. The panicking shortcut past the failure arm is closed at
// type-check time: the flow must `match` and account for both bundles (see
// the error-reading guide).
use cs6_bread_batch::resources::{GreasedTin, ShapedLoaf};
use cs6_bread_batch::resources::boundary::{
    bake_outcome_will_succeed, draw_grid_energy, new_atmosphere, new_grid, new_oven,
};
use cs6_bread_batch::resources::processes::bake;
use model_core::common::boundary::new_person;

fn main() {
    let (energy, grid) = draw_grid_energy::<2_500_000>(new_grid());
    let result = bake(
        new_person::<300_000>(),
        new_oven(),
        ShapedLoaf::test_fixture(),
        GreasedTin::<456>::test_fixture(),
        energy,
        bake_outcome_will_succeed(),
        new_atmosphere(),
    );
    let _ok = result.unwrap();
}
