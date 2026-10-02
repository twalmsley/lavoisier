//! The qualified person's budget really draws down: after spending 2000 ms
//! the operator is `Operator<_, 3000>`, and a call that claims the budget is
//! still 5000 is a type mismatch (E0308) showing both decimal budgets
//! (F-027). (Overdraw itself is post-monomorphization, F-001, and lives as a
//! `compile_fail` doc-test on `operator_draw_time`.)

use exp11_qualifications::qualifications::DrillingCert;
use exp11_qualifications::resources::boundary::certify;
use exp11_qualifications::resources::processes::operator_draw_time;
use model_core::common::boundary::new_person;

fn main() {
    let operator = certify::<DrillingCert, 5000>(new_person::<5000>());
    let (l1, operator) = operator_draw_time::<2000, 3000, 5000, _>(operator);
    // Stale balance: only 3000 ms remain, but this claims 5000 again.
    let (l2, operator) = operator_draw_time::<2000, 3000, 5000, _>(operator);
}
