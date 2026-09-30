//! EXP-05: Marker traits vs type parameters (R6).
//!
//! A bolt catalogue — sizes {M6, M8, M10, (M12 added later)}, materials
//! {Steel, Brass}, lengths {10, 15, 20 mm} — encoded two ways:
//!
//! * [`marker`]  — one struct per combination, characteristics as marker
//!   traits ([`characteristics`]), structs macro-generated.
//! * [`marker_by_hand`] — the same encoding written out with no macro,
//!   kept only so lines of code can be counted honestly.
//! * [`param`]   — a single `Bolt<Size, Material, Length>` with the
//!   characteristics as type parameters.
//! * [`bridge`]  — blanket impls so parameterized bolts satisfy the marker
//!   traits, testing whether the two styles compose.
//! * [`requirements`] — the same requirement expressed in both styles.
//! * [`pitch_demo`]   — what adding a whole new dimension (thread pitch)
//!   costs in each encoding.

pub mod characteristics;
pub mod marker;
pub mod marker_by_hand;
pub mod param;
pub mod bridge;
pub mod requirements;
pub mod pitch_demo;

#[cfg(test)]
mod tests {
    use crate::marker::*;
    use crate::param::{Bolt, L10, L15, SizeM10, SizeM12, SizeM8, Steel};
    use crate::requirements::*;

    /// Verifies: REQ-001 (marker style — the macro-generated struct with the
    /// right characteristics satisfies the marker-trait bound).
    #[test]
    fn marker_bolt_satisfies_marker_requirement() {
        let bolt = M8SteelBolt15mm;
        let _bolt = fasten_marker(bolt);
    }

    /// Verifies: REQ-001 (marker style, hand-written struct — macro and
    /// hand-written encodings are interchangeable).
    #[test]
    fn handwritten_bolt_satisfies_marker_requirement() {
        let bolt = crate::marker_by_hand::M8SteelBolt15mm;
        let _bolt = fasten_marker(bolt);
    }

    /// Verifies: REQ-001 (parameter style — exact concrete type accepted).
    #[test]
    fn param_bolt_satisfies_param_requirement() {
        let bolt = Bolt::<SizeM8, Steel, L15>::new();
        let _bolt = fasten_param(bolt);
    }

    /// Verifies: REQ-002 (parameter style, generic over length).
    #[test]
    fn param_requirement_generic_over_length_accepts_both_lengths() {
        let _b = fasten_param_any_length(Bolt::<SizeM8, Steel, L15>::new());
        let _b = fasten_param_any_length(Bolt::<SizeM8, Steel, L10>::new());
    }

    /// Verifies: REQ-001 (bridging — a parameterized bolt satisfies the
    /// marker-trait requirement through the blanket impls in `bridge`).
    #[test]
    fn param_bolt_satisfies_marker_requirement_via_bridge() {
        let bolt = Bolt::<SizeM8, Steel, L15>::new();
        let _bolt = fasten_marker(bolt);
    }

    /// Verifies: REQ-003 (the M12 size added later works in both styles).
    #[test]
    fn m12_added_to_both_encodings() {
        let _m = fasten_m12_marker(M12BrassBolt20mm);
        let _p = fasten_m12_marker(Bolt::<SizeM12, crate::param::Brass, crate::param::L20>::new());
        let _q = fasten_m12_param(Bolt::<SizeM12, Steel, L15>::new());
    }

    /// Bridging is one-way: this test documents that the generic-over-length
    /// param requirement still works for every size via a marker bound too.
    #[test]
    fn marker_bound_accepts_any_bridged_size() {
        fn is_steel<B: crate::characteristics::Steel>(b: B) -> B {
            b
        }
        let _b = is_steel(Bolt::<SizeM10, Steel, L10>::new());
        let _b = is_steel(M10SteelBolt10mm);
    }
}
