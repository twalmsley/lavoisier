// REQ-007: the pot must be loaded with exactly 3 teabags before brewing —
// and the loaded state exists ONLY at exactly 3 bags (SPEC.md §8), so a
// wrongly-loaded pot is not constructible at all: the expressible wrong bag
// count is an unloaded pot (0 bags) at the brew, which fails the
// Req007LoadedWithThreeBags bound (E0277) with REQ-007's own
// on_unimplemented message (R10 rule 8, F-044).
use cs1_pot_of_tea::resources::boundary::{new_food_waste_bin, new_kitchen_air, new_teapot};
use cs1_pot_of_tea::resources::processes::pour_and_brew;
use cs1_pot_of_tea::resources::BoilingKettle;
use model_core::common::boundary::new_person;
use model_core::nat::aliases::N10;

fn main() {
    let person = new_person::<300_000>();
    let boiling: BoilingKettle<1500, 500_000> = BoilingKettle::test_fixture();
    let unloaded_pot = new_teapot(); // never loaded: 0 bags, not 3
    let out = pour_and_brew::<_, 1473, 480_000, 20_000, _, _, _, _>(
        person,
        boiling,
        unloaded_pot,
        new_food_waste_bin::<N10>(),
        new_kitchen_air(),
    );
}
