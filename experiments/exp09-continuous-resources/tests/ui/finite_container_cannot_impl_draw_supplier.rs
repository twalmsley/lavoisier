// Why `Supplier` (in any shape) is discrete-only for finite containers: the
// next state depends on the amount drawn, and a trait impl has nowhere to
// receive the caller-stated remainder — the leftover const parameter is
// unconstrained (E0207). A local mirror of `SupplierOf` is used because the
// orphan rule would otherwise add an unrelated error.
use exp09_continuous_resources::model::{Gas, GasBottle};

trait DrawSupplier<Out> {
    type Next;
    fn supply_of(self) -> (Out, Self::Next);
}

impl<const TAKE: u64, const LEFT: u64, const FULL: u64> DrawSupplier<Gas<TAKE>>
    for GasBottle<FULL>
{
    type Next = GasBottle<LEFT>;
    fn supply_of(self) -> (Gas<TAKE>, GasBottle<LEFT>) {
        unimplemented!()
    }
}

fn main() {}
