//! Inexact exchange: 333 pence at 117/100 is 389.61 euro cents — neither
//! rounding direction satisfies the value-equivalence assert, so there is NO
//! `OUT` that compiles (R19 integer honesty). `cargo check` passes;
//! `cargo build` fails with E0080 (F-001).

use exp12_money::goods::boundary::new_bureau;
use exp12_money::money::boundary::{close_account, open_account, spend_abroad};
use exp12_money::money::processes::{draw_funds, exchange_gbp_to_eur};

fn main() {
    let account = open_account::<333>();
    let (cash, account) = draw_funds::<333, 0, 333>(account);
    // Round up: 390 * 100 = 39000 != 333 * 117 = 38961. (389 fails too.)
    let (euros, _bureau) = exchange_gbp_to_eur::<333, 390>(new_bureau(), cash);
    spend_abroad(euros);
    close_account(account);
}
