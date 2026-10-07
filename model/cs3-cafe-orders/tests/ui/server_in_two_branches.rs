// R2/R9 — THE CONTENTION COUNTER-EXAMPLE of the concurrency case study. The
// two branches share no resource, which is exactly why the compiler permits
// any interleaving of their steps. This broken flow makes them share one:
// branch B's P4 (take_payment) holds the server, and branch A then pulls the
// SAME server binding into its own adjacent time draw (P2's 60 000 ms, as if
// the server could steam the milk while taking payment). The compiler
// refuses: E0382 "use of moved value: `server`" — "this resource is already
// in use by another process" (F-025). Ignore rustc's borrow/clone fix-its
// (F-007): the real fix is to give each branch its own actor.
use cs3_cafe_orders::resources::boundary::{new_customer, open_till, tender_cash};
use cs3_cafe_orders::resources::processes::take_payment;
use model_core::common::boundary::new_person;
use model_core::common::processes::draw_time;

fn main() {
    let server = new_person::<600_000>();

    // Branch B claims the server: P4 at the till.
    let (cash, customer) = tender_cash(new_customer());
    let (server_at_the_till, till, receipt, customer) =
        take_payment::<300, 0, 700, 600_000, _>(server, open_till(), cash, customer);

    // Branch A tries to draw the SAME server's time for the milk steaming:
    // the server is already busy in branch B.
    let (steam_labour, server) = draw_time::<60_000, 540_000, 600_000>(server);

    let _accounted = (server_at_the_till, till, receipt, customer, steam_labour, server);
}
