//! Mixing currencies is a type error (R19 build item 1): each currency is
//! its own unit type, so `combine` over `Qty<_, Pence>` and
//! `Qty<_, EuroCents>` fails at type-check time (E0308), exactly like adding
//! grams to millimetres.

use exp12_money::units::{EuroCents, Pence};
use model_core::quantity::{Qty, boundary::supply, combine};

fn main() {
    let gbp = supply::<100, Pence>();
    let eur = supply::<117, EuroCents>();
    // 100 pence + 117 euro cents is not a sum in any currency.
    let total: Qty<217, Pence> = combine(gbp, eur);
    let _accounted = total;
}
