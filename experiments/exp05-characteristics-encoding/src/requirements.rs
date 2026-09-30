//! The same requirement expressed in both styles (experiment step 2),
//! using the R10 named-requirement-trait pattern.

use crate::characteristics::{IsBolt, Length15mm, M12, M8, Steel};
use crate::param::{Bolt, L15, Length, SizeM12, SizeM8};

// --------------------------------------------------------------------------
// Marker style: a requirement is a bound (wrapped in a named trait, R10).
// Anything with the right characteristics satisfies it — macro-generated
// structs, hand-written structs, and (via `bridge`) parameterized bolts.
// --------------------------------------------------------------------------

/// REQ-001: Fastening bolts must be M8 steel, 15 mm long.
pub trait Req001FasteningBolt: IsBolt + M8 + Steel + Length15mm {}
impl<T: IsBolt + M8 + Steel + Length15mm> Req001FasteningBolt for T {}

/// Marker-style process: accepts any type satisfying REQ-001.
pub fn fasten_marker<B: Req001FasteningBolt>(bolt: B) -> B {
    bolt
}

// --------------------------------------------------------------------------
// Parameter style: a requirement is a concrete type (or a partially generic
// one). Only `Bolt<...>` can ever satisfy it.
// --------------------------------------------------------------------------

/// REQ-001 again, parameter style: exactly `Bolt<SizeM8, Steel, L15>`.
pub fn fasten_param(bolt: Bolt<SizeM8, crate::param::Steel, L15>) -> Bolt<SizeM8, crate::param::Steel, L15> {
    bolt
}

/// REQ-002: an M8 steel bolt of *any* catalogue length — the parameter style
/// relaxes one dimension by making just that slot generic.
pub fn fasten_param_any_length<L: Length>(
    bolt: Bolt<SizeM8, crate::param::Steel, L>,
) -> Bolt<SizeM8, crate::param::Steel, L> {
    bolt
}

// --------------------------------------------------------------------------
// REQ-003: processes for the M12 size added in experiment step 4.
// --------------------------------------------------------------------------

/// REQ-003: an M12 bolt, any material, any length (marker style — also
/// accepts bridged parameterized bolts).
pub fn fasten_m12_marker<B: IsBolt + M12>(bolt: B) -> B {
    bolt
}

/// REQ-003, parameter style, generic over material and length.
pub fn fasten_m12_param<M: crate::param::Material, L: Length>(
    bolt: Bolt<SizeM12, M, L>,
) -> Bolt<SizeM12, M, L> {
    bolt
}
