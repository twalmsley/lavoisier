// REQ-028: dough may be divided and shaped only once it is proved — kneaded
// dough that was never proved cannot be shaped (E0277, with REQ-028's own
// on_unimplemented phrasing, F-044). Proving is a process (one type per
// state, R9): only `prove`'s output state can be shaped.
use cs6_bread_batch::resources::boundary::{
    draw_water, full_yeast_box, new_pantry_bag, new_pantry_jar, new_tap,
};
use cs6_bread_batch::resources::processes::{
    divide_and_shape, draw_flour, draw_salt, knead, mix_dough,
};
use model_core::common::boundary::new_person;
use model_core::nat::aliases::N2;

fn main() {
    let baker = new_person::<7_200_000>();
    let (flour, bag) = draw_flour::<1_000, 500, 1_500>(new_pantry_bag());
    let (water, tap) = draw_water::<650>(new_tap());
    let (salt, jar) = draw_salt::<18, 482, 500>(new_pantry_jar());
    let (baker, mixed, spent_1, spent_2, empty_box) =
        mix_dough(baker, flour, water, salt, full_yeast_box::<N2>());
    let (baker, kneaded) = knead(baker, mixed);
    // The dough was never proved (no `prove`): this must not compile.
    let (baker, loaf_1, loaf_2) = divide_and_shape(baker, kneaded);
}
