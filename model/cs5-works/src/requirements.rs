//! The works subsystem's requirements (R10) — SPEC §2/§3.
//!
//! The workspace `trace.sh` keys requirement ids globally (F-053): CS-5 owns
//! REQ-023..REQ-027, split by subsystem (SPEC §3) — REQ-023/024 in
//! `cs5-supply`, REQ-027 in `cs5-logistics`, and REQ-025/026 here. Each
//! requirement trait carries its own one-line REQ-phrased
//! `#[diagnostic::on_unimplemented]` (R10 rule 8, F-044) and is used only as
//! a bound (F-019): REQ-025 on [`crate::resources::processes::assemble`]'s
//! component, REQ-026 on [`crate::resources::processes::deliver`]'s order.
//!
//! REQ-025 is the first **cross-crate** requirement bound in the workspace:
//! its characteristic (`GoodsInInspected`) is sealed in `cs5-supply` beside
//! the type it marks, while the requirement trait lives here with its owner
//! — the R10 pattern spanning a crate edge.

use crate::characteristics::ProofOfOrder;
use cs5_supply::characteristics::GoodsInInspected;

model_core::requirement! {
    /// REQ-025: A component may be fitted only after goods-in inspection at the UK site.
    #[diagnostic::on_unimplemented(message = "this part may not be fitted: `{Self}` has not passed goods-in inspection at the UK site (REQ-025)", label = "REQ-025: a component may be fitted only after goods-in inspection", note = "inspection is a process (one type per state, R9): `inspect` turns the boxed 450 g component into the fit-ready 400 g `InspectedComponent` and routes the 50 g packaging to the bin")]
    pub trait Req025InspectedBeforeFitting: (GoodsInInspected);
    assert = assert_req025;
}

model_core::requirement! {
    /// REQ-026: Delivery happens only against the customer's order, with payment taken on delivery.
    #[diagnostic::on_unimplemented(message = "this delivery may not proceed: `{Self}` is not the customer's order token (REQ-026)", label = "REQ-026: delivery happens only against the customer's order, with payment taken on delivery", note = "the order is placed at the boundary (`place_order`) and is the only key `deliver` accepts; the 9000 p payment is consumed in the same process (payment on delivery)")]
    pub trait Req026DeliveryAgainstTheOrder: (ProofOfOrder);
    assert = assert_req026;
}
