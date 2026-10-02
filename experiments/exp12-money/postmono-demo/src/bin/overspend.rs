//! Overspending: drawing 6000 pence from a 5000-pence account (R19/R15).
//! `cargo check` passes; `cargo build` fails with E0080 (F-001).

use exp12_money::money::boundary::{close_account, open_account};
use exp12_money::money::processes::draw_funds;

fn main() {
    let account = open_account::<5000>();
    let (cash, empty) = draw_funds::<6000, 0, 5000>(account);
    let account = exp12_money::money::processes::deposit::<6000, 0, 6000>(empty, cash);
    close_account(account);
}
