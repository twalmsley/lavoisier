// REQ-006: tea must be brewed with boiling water — only the boiling state of
// the kettle can be poured into the pot. A merely FILLED kettle at the pour
// fails the Req006PouredAtTheBoil bound (E0277), and the top line is
// REQ-006's own on_unimplemented message (R10 rule 8, F-044) — the
// requirement trait's attribute, not the failing marker's, because the
// marker fails as a supertrait obligation of the requirement bound.
use cs1_pot_of_tea::resources::boundary::{new_food_waste_bin, new_kitchen_air};
use cs1_pot_of_tea::resources::processes::pour_and_brew;
use cs1_pot_of_tea::resources::{DryTeabag, FilledKettle, LoadedPot};
use model_core::common::boundary::new_person;
use model_core::nat::aliases::N10;

fn main() {
    let person = new_person::<300_000>();
    let filled: FilledKettle<1500> = FilledKettle::test_fixture(); // never boiled
    let pot = LoadedPot::test_fixture((
        DryTeabag::test_fixture(),
        DryTeabag::test_fixture(),
        DryTeabag::test_fixture(),
    ));
    let out = pour_and_brew::<_, 1473, 480_000, 20_000, _, _, _, _>(
        person,
        filled,
        pot,
        new_food_waste_bin::<N10>(),
        new_kitchen_air(),
    );
}
