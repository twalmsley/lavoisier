// REQ-027: all inter-site transport must be by the contracted courier.
// Consigning the purchase order to a home-made van fails with REQ-027's own
// on_unimplemented message (R10 rule 8, F-044) — the `ContractedCourier`
// characteristic is sealed in cs5-logistics (F-026), so no outside
// organisation can be declared contracted.
use cs5_logistics::resources::processes::consign;
use cs5_supply::resources::PurchaseOrder;

/// An un-contracted carrier: right shape, no contract.
struct WhiteVan;

fn main() {
    let order = PurchaseOrder::test_fixture();
    let courier = consign(WhiteVan, order);
}
