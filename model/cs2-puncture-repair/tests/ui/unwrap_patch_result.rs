// R17/F-047: `.unwrap()` on the fallible patch attempt's result does not
// compile, because the failure bundle carries sealed resources and therefore
// no `Debug` impl. The panicking shortcut past the failure arm is closed at
// type-check time: the flow must `match` and account for both bundles (see
// the error-reading guide).
use cs2_puncture_repair::characteristics::Induction;
use cs2_puncture_repair::resources::boundary::{
    new_patch_kit, new_waste_stream, patch_will_hold,
};
use cs2_puncture_repair::resources::processes::{draw_cement, patch_tube};
use cs2_puncture_repair::resources::LocatedTube;
use model_core::boundary::take_one;
use model_core::common::boundary::{new_person, qualify};

fn main() {
    let member = qualify::<Induction, 1_800_000>(new_person::<1_800_000>());
    let located = LocatedTube::test_fixture();
    let (patch, kit) = take_one(new_patch_kit());
    let (cement, kit) = draw_cement::<1, 29, 30, _>(kit);
    let result = patch_tube::<1, 183, 3, 1_800_000, _, _>(
        member,
        located,
        patch,
        cement,
        patch_will_hold(),
        new_waste_stream(),
    );
    let _ok = result.unwrap();
}
