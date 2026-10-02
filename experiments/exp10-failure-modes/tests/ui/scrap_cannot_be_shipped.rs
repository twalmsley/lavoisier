//! One type per state (R9) extended to failure states (candidate R17): the
//! failure output cannot continue the success flow — the customer consumes
//! `DrilledPlate`s, never `ScrapPlate`s. Fixture-built scrap (test-support;
//! trybuild inherits the feature, F-003/F-004).

use exp10_failure_modes::model::boundary::new_customer;
use exp10_failure_modes::model::ScrapPlate;
use model_core::boundary::send_to;

fn main() {
    let scrap: ScrapPlate<430> = ScrapPlate::test_fixture();
    let _customer = send_to(new_customer(), scrap);
}
