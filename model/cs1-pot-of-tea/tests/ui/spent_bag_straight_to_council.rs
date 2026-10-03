// REQ-008, the bypass route: the council collection accepts only the bin's
// bagged FoodWaste, never a loose spent teabag — the bags' only exit from
// the model is through the bin and its sealed disposal path (F-039). Handing
// one to the council directly fails at type-check time: the council's single
// Consumer impl makes inference name the one thing it does accept (E0308
// "expected FoodWaste<_>, found SpentTeabag" — the F-051 effect).
use cs1_pot_of_tea::resources::SpentTeabag;
use cs1_pot_of_tea::resources::boundary::new_council_collection;
use model_core::boundary::send_to;

fn main() {
    let bag = SpentTeabag::test_fixture();
    let council = send_to(new_council_collection(), bag);
}
