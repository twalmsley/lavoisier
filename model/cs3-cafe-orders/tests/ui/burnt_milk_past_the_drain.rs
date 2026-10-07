// REQ-016: burnt milk must be discarded to the drain — the burnt state's
// ONLY consumer is the `Drain` (REQ-016 structural). Handing it to the
// customer instead is an E0277: `Customer` implements no
// `Consumer<BurntMilk<150>>`, and rustc's help enumerates the sinks the
// customer does accept (the served order and money) — the boundary object's
// interface, documented in the error (F-029).
use cs3_cafe_orders::resources::BurntMilk;
use cs3_cafe_orders::resources::boundary::new_customer;
use model_core::boundary::send_to;

fn main() {
    let burnt = BurntMilk::<150>::test_fixture();
    let customer = send_to(new_customer(), burnt);
}
