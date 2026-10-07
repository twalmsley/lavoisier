// R19/F-051 (the exact-amount pattern applied to the till): `take_payment`
// accepts the tender as the CONCRETE `Money<1000>` — the note the customer
// tenders (SPEC.md §3) — so paying with any other amount is a
// type-check-time error that names the right amount: "expected `Money<1000>`,
// found `Money<900>`" (E0308) — visible to editors and trybuild, unlike the
// post-monomorphization split assert (F-001).
use cs3_cafe_orders::resources::Money;
use cs3_cafe_orders::resources::boundary::{new_customer, open_till};
use cs3_cafe_orders::resources::processes::take_payment;
use model_core::common::boundary::new_person;

fn main() {
    let server = new_person::<600_000>();
    let short_tender = Money::<900>::test_fixture();
    let r = take_payment::<300, 0, 700, 600_000, _>(server, open_till(), short_tender, new_customer());
}
