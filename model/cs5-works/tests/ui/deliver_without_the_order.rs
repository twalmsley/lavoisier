// REQ-026: delivery happens only against the customer's order. Delivering
// against the vendor-facing PurchaseOrder — real paperwork, wrong token —
// fails with REQ-026's own on_unimplemented message (R10 rule 8, F-044):
// only the sealed CustomerOrder carries ProofOfOrder (F-026), and nothing
// downstream can forge one.
use cs5_supply::resources::{InspectedComponent, Money, PurchaseOrder};
use cs5_works::resources::boundary::new_customer;
use cs5_works::resources::processes::{assemble, deliver};
use cs5_works::resources::{Account, Housing};

fn main() {
    let instrument = assemble::<1000, _>(Housing::test_fixture(), InspectedComponent::test_fixture());
    let customer = new_customer();
    let payment = Money::<9000>::test_fixture();
    let account = Account::<3000>::test_fixture();
    // The vendor's paperwork is NOT the customer's order.
    let not_the_order = PurchaseOrder::test_fixture();
    let (account, customer): (Account<12_000>, _) =
        deliver(instrument, not_the_order, payment, customer, account);
}
