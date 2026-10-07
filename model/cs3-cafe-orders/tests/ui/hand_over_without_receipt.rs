// REQ-017: an order may be handed over only after payment — the join
// consumes the payment receipt, which only `take_payment` (branch B, the
// till) can mint. A flow that never took payment has nothing to put in the
// receipt position; stuffing something else there (a second tray) is the
// REQ-phrased E0277 (R10 rule 8, F-044): "`Tray` is not the payment receipt
// from the till branch (REQ-017)".
use cs3_cafe_orders::resources::boundary::{new_cup, new_teapot, new_tray};
use cs3_cafe_orders::resources::processes::hand_over;
use cs3_cafe_orders::resources::{Cup, FlatWhite, PotOfTea, Teapot};
use model_core::common::boundary::new_person;
use model_core::history::boundary::new_history;

fn main() {
    let server = new_person::<600_000>();
    let flat_white = FlatWhite::<186>::test_fixture(Cup::test_fixture());
    let tea = PotOfTea::<303>::test_fixture(Teapot::test_fixture(), Cup::test_fixture());
    let no_receipt = new_tray(); // payment never taken: no receipt exists
    let r = hand_over::<303, 600_000, _, _>(
        server,
        flat_white,
        tea,
        new_tray(),
        no_receipt,
        new_history(),
        new_history(),
    );
    let _crockery_stays = (new_teapot(), new_cup());
}
