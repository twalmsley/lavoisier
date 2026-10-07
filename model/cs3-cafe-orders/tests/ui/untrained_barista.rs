// R18 (REQ-015): the server — a plain `Person` — carries no machine training,
// so REQ-015 refuses them at the espresso machine (E0277). The top line is
// REQ-015's own on_unimplemented message (R10 rule 8, F-044) — the
// requirement trait's attribute, not the failing marker's, because the marker
// fails as a supertrait obligation of the requirement bound.
use cs3_cafe_orders::resources::boundary::{
    fill_hopper, fill_water_tank, new_cup, new_espresso_machine, new_knock_box,
};
use cs3_cafe_orders::resources::processes::{draw_grounds, draw_water, pull_espresso};
use model_core::common::boundary::new_person;

fn main() {
    let server = new_person::<600_000>(); // never trained on the machine
    let (grounds, hopper) = draw_grounds::<18, 482, 500>(fill_hopper());
    let (water, tank) = draw_water::<40, 160, 200>(fill_water_tank());
    let r = pull_espresso::<18, 40, 36, 22, _, _>(
        server,
        new_espresso_machine(),
        grounds,
        water,
        new_cup(),
        new_knock_box(),
    );
}
