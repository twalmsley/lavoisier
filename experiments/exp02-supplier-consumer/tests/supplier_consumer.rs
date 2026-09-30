//! Integration tests: these live OUTSIDE the privacy boundary, so they can
//! only obtain bolts through the public supplier API — exactly like real
//! model code would.

use exp02_supplier_consumer::*;

/// Verifies: R12 (supply changes the type; count follows the list).
#[test]
fn supplying_shrinks_the_box() {
    let bx = full_box::<N2>();
    assert_eq!(BoltBox::<BoltsOf<N2>>::COUNT, 2);
    let (_bolt, bx) = bx.supply();
    let (_bolt2, bx) = bx.supply();
    let _empty: EmptyBoltBox = bx; // the exhausted box is a distinct type
    assert_eq!(EmptyBoltBox::COUNT, 0);
}

/// Verifies: R12 (key test, variant (a) — hand-written chained bounds).
#[test]
fn fasten_four_with_chained_bounds() {
    let bx = full_box::<N5>();
    let ([_b1, _b2, _b3, _b4], rest) = fasten_four_chained(bx);
    // One bolt left: the rest is a box of exactly one.
    let (_last, empty) = rest.supply();
    let _empty: EmptyBoltBox = empty;
}

/// Verifies: R12 (key test, variant (b) — recursive SupplyN trait).
#[test]
fn fasten_four_with_supplyn() {
    let bx = full_box::<N4>();
    let (_four_bolts, rest) = fasten_four_supplyn(bx);
    let _empty: EmptyBoltBox = rest;
}

/// Verifies: R12 (consumer with type-level space; full bag is a new type).
#[test]
fn waste_bag_fills_up() {
    let bx = full_box::<N2>();
    let bag = new_waste_bag::<N2>();
    let (bolt, bx) = bx.supply();
    let bag = bag.consume(bolt);
    let (bolt, bx) = bx.supply();
    let bag = bag.consume(bolt);
    let _empty: EmptyBoltBox = bx;
    let _full: FullWasteBag<_> = bag; // no space left: distinct type
}

/// Verifies: R12 (a supplier's output can feed a consumer, N items).
#[test]
fn supply_n_into_consume_list() {
    let bx = full_box::<N4>();
    let (taken, rest) = supply_n::<N4, _>(bx);
    let _empty: EmptyBoltBox = rest;
    let bag = new_waste_bag::<N5>();
    let bag = bag.consume_list(taken); // one space left, not full
    let _bolt_box_gone = bag;
}
