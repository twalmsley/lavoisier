//! Wrong change: tendering 500 pence for a 350-pence purchase and claiming
//! 200 pence change (R19). `cargo check` passes; `cargo build` fails with
//! E0080 (F-001).

use exp12_money::goods::boundary::{new_rack, new_vendor};
use exp12_money::money::boundary::{close_account, open_account};
use exp12_money::money::processes::{deposit, draw_funds, purchase};
use model_core::boundary::send_to;

fn main() {
    let account = open_account::<1000>();
    let (tendered, account) = draw_funds::<500, 500, 1000>(account);
    let (spanner, change, _vendor) = purchase::<_, 500, 350, 200>(new_vendor(), tendered);
    let _rack = send_to(new_rack(), spanner);
    let account = deposit::<200, 500, 700>(account, change);
    close_account(account);
}
