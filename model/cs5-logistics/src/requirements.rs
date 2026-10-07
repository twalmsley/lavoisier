//! The logistics subsystem's requirement (R10) — SPEC §2/§3.
//!
//! The workspace `trace.sh` keys requirement ids globally (F-053): CS-5 owns
//! REQ-023..REQ-027, split by subsystem (SPEC §3) — REQ-023/024 in
//! `cs5-supply`, REQ-025/026 in `cs5-works`, and REQ-027 here. The
//! requirement trait carries its own one-line REQ-phrased
//! `#[diagnostic::on_unimplemented]` (R10 rule 8, F-044) and is used only as
//! a bound ([`crate::resources::processes::consign`], F-019).

use crate::characteristics::ContractedCourier;

model_core::requirement! {
    /// REQ-027: All inter-site transport must be by the contracted courier.
    #[diagnostic::on_unimplemented(message = "goods may not be moved between sites by `{Self}`: it is not the contracted courier (REQ-027)", label = "REQ-027: all inter-site transport must be by the contracted courier", note = "only the sealed `CourierAtUk` carries the contract (F-026): consign it (`consign`), and the location-states carry the cargo by value from there")]
    pub trait Req027ContractedTransportOnly: (ContractedCourier);
    assert = assert_req027;
}
