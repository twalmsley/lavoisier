//! The logistics subsystem's sealed resource family (R1): the contracted
//! courier's four location-states, its boundary, and the transport
//! processes — SPEC §3–§5.
//!
//! **One type per location-state** (R9, F-023; SPEC §3): at-UK → outbound →
//! at-vendor → inbound → at-UK. A flow that skips a leg — consigning a
//! courier that is already abroad, handing over goods that were never
//! collected — is a type error, read directly as "the courier is not there".
//! The cargo travels **by value inside the state type**: the purchase order
//! outbound, the boxed component inbound (both `cs5-supply`'s sealed,
//! tripwired types, so an abandoned courier trips its cargo's tripwire at
//! test time, R1 layer 2). Only the order travels — the payment is remitted
//! flow-routed to the vendor at P3 (SPEC §7).
//!
//! The courier is an **organisation** (R11 spirit): it acts without a time
//! budget (SPEC §8 review decision 6) — the works operator's draws happen
//! works-side, adjacent in the flow (F-048).

use crate::characteristics::{ContractedCourier, sealed};
// One line on purpose: the traceability grep (F-021) skips `use` lines, but
// only when the line itself starts with `use`.
use crate::requirements::assert_req027;
use cs5_supply::resources::{BoxedComponent, PurchaseOrder};

// ---------------------------------------------------------------------------
// The courier's location-states (one sealed type per state, R9).
// ---------------------------------------------------------------------------

/// The contracted courier at its UK home site, ready for a consignment —
/// the only state carrying the sealed [`ContractedCourier`] characteristic
/// (REQ-027). Reusable (R2): the loop ends back in this state and the
/// courier stays with the caller, ready for the next consignment. Hand-sealed
/// (R1): private field, no public constructor, no `Clone`/`Copy`/`Default`;
/// enters only via [`boundary::courier_reports_for_duty`].
#[must_use = "CourierAtUk is a reusable organisation: consign it or return it to the caller"]
pub struct CourierAtUk {
    _seal: (),
}

impl sealed::Sealed for CourierAtUk {}
impl ContractedCourier for CourierAtUk {
    fn stand_down(self, _permit: DepartPermit) {
        // Conserving state change (R9): the courier continues as the
        // outbound state `consign` mints in its place.
        let CourierAtUk { _seal: () } = self;
    }
}

/// The courier under its requirement-facing name; the alias carries the tag
/// because a tag on the struct above would sit apart from the blanket-less
/// characteristic impl trace.sh cannot see (F-020/F-037).
///
/// Satisfies: REQ-027
pub type TheContractedCourier = CourierAtUk;
model_core::satisfies!(assert_req027, TheContractedCourier);

/// The courier en route to the vendor, carrying the purchase order by value
/// (SPEC P2: only the order travels — payment is remitted flow-routed,
/// SPEC §7). A distinct location-state (R9): it cannot be consigned again
/// and cannot hand anything over.
#[must_use = "OutboundCourier is mid-journey: carry it to the vendor"]
pub struct OutboundCourier {
    order: PurchaseOrder,
    _seal: (),
}

/// The courier at the vendor's site, order in hand, ready to present it
/// (SPEC P3). A distinct location-state (R9).
#[must_use = "CourierAtVendor is at the vendor: present the order and purchase"]
pub struct CourierAtVendor {
    order: PurchaseOrder,
    _seal: (),
}

/// The courier on the return leg, carrying the boxed component by value
/// (SPEC P3→P4). A distinct location-state (R9): the goods can leave it only
/// through [`processes::hand_over`] at the works.
#[must_use = "InboundCourier carries the goods: hand them over at the works"]
pub struct InboundCourier {
    cargo: BoxedComponent,
    _seal: (),
}

/// The permit gating [`ContractedCourier::stand_down`] (the F-054
/// permit-gated conversion pattern): a private field and no public
/// constructor, so only [`processes::consign`] (inside this privacy
/// boundary) can retire the home state — the characteristic can never be
/// used to vanish a courier outside the process that mints its next state.
pub struct DepartPermit {
    pub(crate) _seal: (),
}

/// The creation boundary of the logistics family (R12, F-006): the only
/// production code where the courier comes into existence.
pub mod boundary {
    use super::CourierAtUk;

    /// The contracted courier reports for duty at the UK site (R12;
    /// SPEC §4: courier at start).
    ///
    /// Placeholder: the courier organisation's own fleet and staffing — out
    /// of scope (SPEC §1); one courier, one contract.
    pub fn courier_reports_for_duty() -> CourierAtUk {
        CourierAtUk { _seal: () }
    }
}

/// The transport processes (R1, R2): pure by-value conversions between
/// location-states. They mint the next state's sealed values, so they live
/// inside the resource family's module (F-031).
pub mod processes {
    use super::{BoxedComponent, CourierAtUk, CourierAtVendor, DepartPermit, InboundCourier, OutboundCourier, PurchaseOrder};
    // One line on purpose: the traceability grep (F-021) skips `use` lines,
    // but only when the line itself starts with `use`.
    use crate::requirements::Req027ContractedTransportOnly;
    use cs5_supply::requirements::Req023PaidInEuroCents;
    use cs5_supply::resources::{COMPONENT_PRICE_EC, Euros};
    use model_core::boundary::{Consumer, Supplier};

    /// P2 — consigns the purchase order to the contracted courier (REQ-027;
    /// SPEC P2). Style A (F-048): the courier parameter is bounded by the
    /// requirement, so an un-contracted carrier fails with the REQ-phrased
    /// message (F-044) — pinned by
    /// `cs5-works/tests/ui/uncontracted_transport.rs`. The courier's home
    /// state continues as the outbound state (a conserving state change
    /// through the permit-gated `stand_down`, F-054); **only the order is
    /// consigned** — payment does not travel (SPEC §7). The operator's
    /// 60 000 ms is drawn by the adjacent `draw_time` in the works' flow
    /// (F-048); courier state at-UK → outbound (structural).
    ///
    /// Satisfies: REQ-027
    pub fn consign<C: Req027ContractedTransportOnly>(courier: C, order: PurchaseOrder) -> OutboundCourier {
        // Conserving state change (R9/F-054): the home state is retired and
        // the courier continues as the outbound state, order aboard.
        courier.stand_down(DepartPermit { _seal: () });
        OutboundCourier { order, _seal: () }
    }

    /// Carries the consignment to the vendor's site (SPEC P3's first leg):
    /// a pure location-state change (R9/SPEC §7 — transit never fails and
    /// takes no modelled duration), the order still aboard.
    pub fn carry_to_vendor(courier: OutboundCourier) -> CourierAtVendor {
        let OutboundCourier { order, _seal: () } = courier;
        CourierAtVendor { order, _seal: () }
    }

    /// P3 — the purchase at the vendor (SPEC P3; REQ-023 + REQ-027). No
    /// operator time: the courier and vendor are the actors (SPEC §5). The
    /// courier presents the order; the **remitted** 2340 ec (flow-routed
    /// from the bureau, SPEC §7) pays the vendor at its exact price; one
    /// boxed component leaves the finite stock; the courier turns for home
    /// with the box aboard (at-vendor → inbound, structural). The money and
    /// goods balances are structural: 2340 = 2340 by the exact-price impl,
    /// 1 = 1 by the supply step (SPEC P3). A fourth purchase fails on the
    /// vendor's exhausted stock — the refined placeholder (SPEC §4), pinned
    /// by `cs5-works/tests/ui/fourth_purchase_from_the_3_stock.rs`.
    ///
    /// Satisfies: REQ-027
    pub fn purchase_at_vendor<V: Req023PaidInEuroCents + Consumer<PurchaseOrder>>(courier: CourierAtVendor, vendor: V, payment: Euros<COMPONENT_PRICE_EC>) -> (InboundCourier, <<V::Next as Consumer<Euros<COMPONENT_PRICE_EC>>>::Next as Supplier>::Next) where V::Next: Consumer<Euros<COMPONENT_PRICE_EC>>, <V::Next as Consumer<Euros<COMPONENT_PRICE_EC>>>::Next: Supplier<Item = BoxedComponent> {
        let CourierAtVendor { order, _seal: () } = courier;
        let (cargo, vendor) = cs5_supply::resources::processes::sell_component(vendor, order, payment);
        (InboundCourier { cargo, _seal: () }, vendor)
    }

    /// P4 (transport half) — the courier arrives back at the UK site,
    /// surrenders the boxed component at goods-in, and returns home for
    /// reuse (inbound → at-UK; SPEC P4). The works' receiving process
    /// composes this with the operator's 60 000 ms draw (F-048).
    ///
    /// Satisfies: REQ-027
    pub fn hand_over(courier: InboundCourier) -> (BoxedComponent, CourierAtUk) {
        let InboundCourier { cargo, _seal: () } = courier;
        (cargo, CourierAtUk { _seal: () })
    }
}

#[cfg(test)]
mod tests {
    //! Per-process unit tests (R5): the courier loop leg by leg, with every
    //! resource accounted for through production paths — the goods continue
    //! into inspection and the packaging into disposal, because this crate
    //! cannot defuse `cs5-supply`'s sealed types (the crate edge, EXP-08).

    use super::boundary::courier_reports_for_duty;
    use super::processes::{carry_to_vendor, consign, hand_over, purchase_at_vendor};
    use super::{CourierAtUk, TheContractedCourier};
    use cs5_supply::resources::boundary::{new_packaging_disposal, place_purchase_order, vendor_opens_for_business, vendor_returns_to_its_hinterland};
    use cs5_supply::resources::processes::open_and_inspect;
    use cs5_supply::resources::{Euros, TwoBoxedComponents, Vendor};
    use model_core::boundary::send_to;

    /// The full courier loop (SPEC P2→P4): consign with the order only,
    /// carry out, purchase against order + remitted exact price, hand the
    /// box over at the works, courier home and reusable — every state a
    /// distinct type, every cargo conserved.
    ///
    /// Verifies: REQ-023, REQ-027
    #[test]
    fn the_courier_loop_moves_order_out_and_goods_home() {
        let courier = consign(courier_reports_for_duty(), place_purchase_order());
        let courier = carry_to_vendor(courier);
        let (courier, vendor) = purchase_at_vendor(
            courier,
            vendor_opens_for_business(),
            Euros::test_fixture(),
        );
        assert_eq!(Vendor::<TwoBoxedComponents>::STOCK, 2);
        let (boxed, courier) = hand_over(courier);
        // The courier is home and reusable: a second consignment type-checks.
        let courier: CourierAtUk = courier;
        let courier = consign(courier, place_purchase_order());
        // Account for everything through production paths (R1): the box
        // continues into inspection, the packaging into disposal, the
        // vendor withdraws with its remaining stock.
        let (component, packaging) = open_and_inspect::<50>(boxed);
        let disposal = send_to(new_packaging_disposal(), packaging);
        vendor_returns_to_its_hinterland(vendor);
        let courier = carry_to_vendor(courier);
        let (courier, vendor) = purchase_at_vendor(
            courier,
            vendor_opens_for_business(),
            Euros::test_fixture(),
        );
        let (boxed, courier) = hand_over(courier);
        let (component2, packaging2) = open_and_inspect::<50>(boxed);
        let disposal = send_to(disposal, packaging2);
        vendor_returns_to_its_hinterland(vendor);
        let _home_again: TheContractedCourier = courier;
        let _kept_untripwired_items = (component, component2);
        let _disposal_stays_at_the_boundary = disposal;
    }

    /// A courier abandoned mid-journey trips its cargo's tripwire at test
    /// time (R1 layer 2, F-008): the consigned order is conserved cargo.
    #[test]
    #[should_panic(expected = "PurchaseOrder dropped without being consumed")]
    fn an_abandoned_outbound_courier_trips_the_order_tripwire() {
        let courier = consign(courier_reports_for_duty(), place_purchase_order());
        let _parked_in_a_layby_forever = courier; // falls out of scope: leak
    }
}
