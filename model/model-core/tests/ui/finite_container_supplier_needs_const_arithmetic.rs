// F-028 pinned, the other shape: computing the container's next state in the
// impl needs const arithmetic in a type (`GasBottle<{ FULL - TAKE }>`), which
// is nightly-only (`generic_const_exprs`). Together with the E0207 case this
// is why `Supplier` is documented as discrete-only (R12/R15).
use model_core::fixtures::{Gas, GasBottle};

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
