// REQ-025: a component may be fitted only after goods-in inspection at the
// UK site. Assembling with the still-boxed component fails with REQ-025's
// own on_unimplemented message (R10 rule 8, F-044) — only the
// `InspectedComponent` state carries the sealed `GoodsInInspected`
// characteristic (one type per state, R9/F-023), and inspection is the only
// process that produces it.
use cs5_supply::resources::BoxedComponent;
use cs5_works::resources::processes::assemble;
use cs5_works::resources::Housing;

fn main() {
    let housing = Housing::test_fixture();
    // Straight off the courier, box and all.
    let boxed = BoxedComponent::test_fixture();
    let instrument = assemble::<1050, _>(housing, boxed);
}
