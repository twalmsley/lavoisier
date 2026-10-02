// F-028 pinned: `Supplier` is discrete-only. A finite continuous container
// cannot implement any supplier-shaped trait on stable: the next state needs
// the caller-stated remainder, and a trait impl has nowhere to receive it —
// the leftover const parameter is unconstrained (E0207). A local mirror of a
// parameterized supplier trait is used because the orphan rule would
// otherwise add an unrelated error. Continuous boundary sources are
// draw-style processes instead (R15).
use model_core::fixtures::{Gas, GasBottle};

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
