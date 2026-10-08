// R2/R9: one oven, so the two bakes are forced sequential — starting the
// second bake before the first has returned the oven is the contention
// error at the exact line (E0382 "use of moved value: `oven`"; SPEC.md §6).
// The second bake can only be written with the oven carried in the first
// attempt's outcome bundles.
use cs6_bread_batch::resources::{GreasedTin, ShapedLoaf};
use cs6_bread_batch::resources::boundary::{
    bake_outcome_will_succeed, draw_grid_energy, new_atmosphere, new_grid, new_oven,
};
use cs6_bread_batch::resources::processes::bake;
use model_core::common::boundary::new_person;

fn main() {
    let oven = new_oven();
    let (energy_1, grid) = draw_grid_energy::<2_500_000>(new_grid());
    let (energy_2, grid) = draw_grid_energy::<2_500_000>(grid);
    let attempt_1 = bake(
        new_person::<300_000>(),
        oven,
        ShapedLoaf::test_fixture(),
        GreasedTin::<456>::test_fixture(),
        energy_1,
        bake_outcome_will_succeed(),
        new_atmosphere(),
    );
    // The first bake has not returned the oven: this must not compile.
    let attempt_2 = bake(
        new_person::<300_000>(),
        oven,
        ShapedLoaf::test_fixture(),
        GreasedTin::<456>::test_fixture(),
        energy_2,
        bake_outcome_will_succeed(),
        new_atmosphere(),
    );
}
