//! The euro-cent currency unit (R19) — the whole quantity-level cost of a
//! new currency dimension.
//!
//! R19: money is one dimension **per currency**, in integer minor units, and
//! currency unit types never mix — cross-currency arithmetic is a type
//! error, like adding grams to millimetres. A new currency is **one unit
//! type plus one `impl Unit` line** at the quantity level (model-core's
//! `Unit` kind trait is open, F-051); the in-model cash carriers are the
//! sealed R15 container types in [`crate::resources`] ([`crate::resources::Money`]
//! in pence, [`crate::resources::Euros`] in euro cents), where the dimension
//! separation is two sealed types with no conversion function. There is
//! deliberately no shared "money" unit and no rate constant at this level:
//! conversion exists only as the R19 boundary exchange process
//! ([`crate::resources::processes::exchange`]).

use model_core::quantity::Unit;

/// EUR in euro cents (integer minor units, the R7 money row applied per
/// currency — R19). **The new currency dimension of CS-5**: this one impl
/// line is all the quantity-level machinery a currency needs (F-051), and
/// the generic `Qty`/`split`/`combine` kernel serves it unchanged:
///
/// ```
/// use cs5_supply::units::EuroCents;
/// use model_core::quantity::{Qty, boundary::supply, combine, split};
///
/// let float = supply::<234, EuroCents>();
/// let (a, b): (Qty<117, EuroCents>, Qty<117, EuroCents>) = split(float);
/// let back: Qty<234, EuroCents> = combine(a, b);
/// let _accounted = back;
/// ```
pub struct EuroCents;
impl Unit for EuroCents {}
