// REQ-029: the oven may bake only a loaf seated in a greased tin — a clean
// tin straight from the kitchen setup cannot go in the oven (E0277, with
// REQ-029's own on_unimplemented phrasing, F-044). Greasing is a process
// (one type per state, R9): only `grease_tins`' output state can be baked
// in.
use cs6_bread_batch::resources::ShapedLoaf;
use cs6_bread_batch::resources::boundary::{
    bake_outcome_will_succeed, draw_grid_energy, new_atmosphere, new_clean_tin, new_grid,
    new_oven,
};
use cs6_bread_batch::resources::processes::bake;
use model_core::common::boundary::new_person;

fn main() {
    let baker = new_person::<300_000>();
    let loaf = ShapedLoaf::test_fixture();
    let (energy, grid) = draw_grid_energy::<2_500_000>(new_grid());
    // The tin was never greased (no `grease_tins`): this must not compile.
    let attempt = bake(
        baker,
        new_oven(),
        loaf,
        new_clean_tin(),
        energy,
        bake_outcome_will_succeed(),
        new_atmosphere(),
    );
}
