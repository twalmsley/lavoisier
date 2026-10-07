// REQ-011: no blind patching — a tube whose puncture has not been located
// (the `PuncturedTube` state, straight out of the wheel) cannot be patched
// (E0277, with REQ-011's own on_unimplemented phrasing, F-044). Locating is
// a process (one type per state, R9): only `find_hole`'s output state can be
// patched.
use cs2_puncture_repair::characteristics::Induction;
use cs2_puncture_repair::resources::boundary::{
    new_patch_kit, new_waste_stream, new_workstand, patch_will_hold, wheel_arrives_for_service,
};
use cs2_puncture_repair::resources::processes::{draw_cement, open_wheel, patch_tube};
use model_core::boundary::take_one;
use model_core::common::boundary::{new_person, qualify};

fn main() {
    let member = qualify::<Induction, 1_800_000>(new_person::<1_800_000>());
    let (member, stand, open, punctured_tube) =
        open_wheel(member, new_workstand(), wheel_arrives_for_service());
    let (patch, kit) = take_one(new_patch_kit());
    let (cement, kit) = draw_cement::<1, 29, 30, _>(kit);
    // The hole was never found (no `find_hole`): this must not compile.
    let r = patch_tube::<1, 183, 3, 1_800_000, _, _>(
        member,
        punctured_tube,
        patch,
        cement,
        patch_will_hold(),
        new_waste_stream(),
    );
}
