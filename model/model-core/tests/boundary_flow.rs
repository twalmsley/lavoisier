//! Integration tests for the discrete boundary machinery (R12, R13) through
//! the `test-support` fixtures — exercised exactly the way a downstream crate
//! would use them (self dev-dependency re-declaration, F-004).
//!
//! model-core defines no `REQ-NNN` requirements (it is infrastructure, R10
//! applies to the downstream modelling crates), so these tests carry plain
//! doc comments, not traceability tags.

#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![deny(let_underscore_drop)]
// Capacity tests build 100-deep types (F-010: ≈ N + 3 needed; the attribute
// is per crate, so the test crate sets it too).
#![recursion_limit = "2048"]

use model_core::boundary::{send_list, send_to, take_n, take_one};
use model_core::fixtures::{
    Bolt, BoltBox, BoltsOf, EmptyBoltBox, FullWasteBag, WasteBag, full_box, new_waste_bag,
};
use model_core::list::{Cons, Nil};
use model_core::nat::aliases::{N1, N2, N4, N100};

/// The list type of exactly four bolts (the `Taken =` bound doubles as the
/// item-type requirement, F-014).
type FourBolts = Cons<Bolt, Cons<Bolt, Cons<Bolt, Cons<Bolt, Nil>>>>;

/// A generic process taking four bolts from ONE supplier: one where-clause,
/// whatever N is (F-014) — never hand-chained `S::Next` bounds.
fn fasten_four<S>(s: S) -> (FourBolts, S::Rest)
where
    S: model_core::boundary::SupplyN<N4, Taken = FourBolts>,
{
    s.supply_n()
}

/// Supply one item through the generic access process (F-015): the box count
/// goes down by one, and the type changes with it.
#[test]
fn supplying_shortens_the_box_by_one() {
    let bx = full_box::<N2>();
    assert_eq!(BoltBox::<BoltsOf<N2>>::COUNT, 2);
    let (bolt, bx) = take_one(bx);
    assert_eq!(BoltBox::<BoltsOf<N1>>::COUNT, 1);
    // Account for everything: bolt and remainder go to a waste bag.
    let bag = new_waste_bag::<N2>();
    let bag = send_to(bag, bolt);
    let (bolt2, empty) = take_one(bx);
    let bag: FullWasteBag<_> = send_to(bag, bolt2);
    assert_eq!(FullWasteBag::<BoltsOf<N2>>::HELD, 2);
    let _empty: EmptyBoltBox = empty; // the exhausted box is a new resource
    let _full_bag_to_account_for = bag;
}

/// A generic process takes four bolts from one supplier in a single bound
/// (F-014) and the depleted supplier comes back as the empty box (R12).
#[test]
fn four_bolts_through_supply_n() {
    let bx = full_box::<N4>();
    let (taken, rest) = fasten_four(bx);
    let _empty: EmptyBoltBox = rest;
    // Feed the whole taken list to one consumer (F-014).
    let bag = new_waste_bag::<N4>();
    let full: FullWasteBag<_> = send_list(bag, taken);
    assert_eq!(FullWasteBag::<FourBolts>::HELD, 4);
    let _full_bag_to_account_for = full;
}

/// The capacity-100 round trip (R12; compile-time cost measured at ~2.6 s,
/// F-010): fill 100, supply all 100 through `SupplyN`, consume all 100 into a
/// `WasteBag<N100>`.
#[test]
fn capacity_100_round_trip() {
    let bx = full_box::<N100>();
    assert_eq!(BoltBox::<BoltsOf<N100>>::COUNT, 100);
    let (taken, rest) = take_n::<N100, _>(bx);
    let _empty: EmptyBoltBox = rest;
    let bag = new_waste_bag::<N100>();
    let full: FullWasteBag<_> = send_list(bag, taken);
    assert_eq!(FullWasteBag::<BoltsOf<N100>>::HELD, 100);
    let _full_bag_to_account_for = full;
}

/// Consuming changes the consumer's type: space goes down, contents grow
/// (F-016), and the item is genuinely kept.
#[test]
fn waste_bag_keeps_what_it_consumes() {
    let bx = full_box::<N1>();
    let (bolt, empty) = take_one(bx);
    let bag: WasteBag<N1> = new_waste_bag::<N1>();
    assert_eq!(WasteBag::<N1>::HELD, 0);
    let bag = send_to(bag, bolt);
    assert_eq!(FullWasteBag::<Cons<Bolt, Nil>>::HELD, 1);
    let _empty: EmptyBoltBox = empty;
    let _full_bag_to_account_for = bag;
}
