// REQ-008: all spent teabags must reach the food-waste bin. The spent bags
// never leave pour_and_brew loose — the process feeds them to its bin
// parameter itself — so "sending them anywhere but the bin" means putting
// something else in the bin slot: the kitchen air here fails the
// Req008FoodWasteBinOnly bound (E0277) with REQ-008's own on_unimplemented
// message (R10 rule 8, F-044), and also fails ConsumeList (no consumer for
// spent teabags) with model-core's modeller-phrased message (F-015).
use cs1_pot_of_tea::resources::boundary::new_kitchen_air;
use cs1_pot_of_tea::resources::processes::pour_and_brew;
use cs1_pot_of_tea::resources::{BoilingKettle, DryTeabag, LoadedPot};
use model_core::common::boundary::new_person;

fn main() {
    let person = new_person::<300_000>();
    let boiling: BoilingKettle<1500, 500_000> = BoilingKettle::test_fixture();
    let pot = LoadedPot::test_fixture((
        DryTeabag::test_fixture(),
        DryTeabag::test_fixture(),
        DryTeabag::test_fixture(),
    ));
    let out = pour_and_brew::<_, 1473, 480_000, 20_000, _, _, _, _>(
        person,
        boiling,
        pot,
        new_kitchen_air(), // the air is not the food-waste bin (REQ-008)
        new_kitchen_air(),
    );
}
