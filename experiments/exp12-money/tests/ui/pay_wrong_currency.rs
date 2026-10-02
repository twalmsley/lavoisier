//! Paying in the wrong currency is a type error (R19 build item 5): the
//! vendor's `Consumer` impl exists only for `Money<350>` (GBP pence), so
//! handing it `Euros<350>` fails the trait bound at type-check time, on the
//! generic-process path that carries the modeller-phrased on_unimplemented
//! message (F-015).

use exp12_money::goods::boundary::new_vendor;
// The test-support feature is enabled for test targets (dev-dependencies
// re-declaration, F-004; trybuild inherits it, F-003), so the fixture can
// mint the euros here without dragging the exchange chain into the error
// under test.
use exp12_money::money::Euros;
use model_core::boundary::send_to;

fn main() {
    let euros: Euros<350> = Euros::test_fixture();
    // The vendor takes pence, not euro cents — even at the right magnitude.
    let vendor = send_to(new_vendor(), euros);
    let _ = vendor;
}
