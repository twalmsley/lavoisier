//! Paying the wrong amount is a type error (R19 build item 3): the vendor
//! implements `Consumer` only at its exact price (`Money<350>`), so handing
//! it 300 pence fails the trait bound at type-check time — and rustc's help
//! text lists the one impl that exists, naming the right price in the error.

use exp12_money::goods::boundary::new_vendor;
use exp12_money::money::Money;
use model_core::boundary::send_to;

fn main() {
    let underpayment: Money<300> = Money::test_fixture();
    let vendor = send_to(new_vendor(), underpayment);
    let _ = vendor;
}
