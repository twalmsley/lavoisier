// REQ-019 is the crate edge (SPEC §3, EXP-08/F-005/F-006): the kernel's
// generated constructors are pub(crate) to the defining crate, so the line
// calling one is E0624 "associated function `mint` is private" — material
// cannot be sourced from this side of the edge. Sibling of
// line_cannot_mint_materials.rs (the struct-literal route, E0451).
use cs4_stores::resources::Blank;

fn main() {
    // A blank from nothing — minting material downstream.
    let blank = Blank::<2250>::mint();
}
