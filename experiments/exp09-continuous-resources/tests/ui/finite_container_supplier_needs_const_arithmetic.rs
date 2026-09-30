// The other way a finite container might implement a supplier shape —
// computing the next state — needs const arithmetic in a type, which is
// nightly-only (`generic_const_exprs`).
use exp09_continuous_resources::model::{Gas, GasBottle};

trait DrawSupplier<Out> {
    type Next;
    fn supply_of(self) -> (Out, Self::Next);
}

impl<const TAKE: u64, const FULL: u64> DrawSupplier<Gas<TAKE>> for GasBottle<FULL> {
    type Next = GasBottle<{ FULL - TAKE }>;
    fn supply_of(self) -> (Gas<TAKE>, Self::Next) {
        unimplemented!()
    }
}

fn main() {}
