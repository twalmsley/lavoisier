// REQ-012: a wheel may be refitted only with an airtight tube — patched AND
// checked, or new. The patched-but-unchecked `PatchedTube` state is not
// airtight (checking is a process, R9), so the refit refuses it (E0277, with
// REQ-012's own on_unimplemented phrasing, F-044). The two states that DO
// satisfy the bound — `CheckedTube` and `SpareTube` — are the requirement's
// deliberately multi-type satisfaction set (SPEC.md §8, review decision 7).
use cs2_puncture_repair::characteristics::Induction;
use cs2_puncture_repair::resources::boundary::{new_pump, new_workstand};
use cs2_puncture_repair::resources::processes::refit_and_inflate;
use cs2_puncture_repair::resources::{OpenWheel, PatchedTube};
use model_core::common::boundary::{new_person, qualify};

fn main() {
    let member = qualify::<Induction, 1_800_000>(new_person::<1_800_000>());
    let open = OpenWheel::test_fixture();
    let unchecked: PatchedTube<183> = PatchedTube::test_fixture(); // never checked
    let r = refit_and_inflate::<2083, _, _>(member, new_workstand(), new_pump(), open, unchecked);
}
