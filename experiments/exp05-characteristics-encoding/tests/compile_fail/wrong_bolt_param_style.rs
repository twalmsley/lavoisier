//! An M6 brass bolt passed where REQ-001 demands exactly
//! Bolt<SizeM8, Steel, L15>, parameter style.
use exp05_characteristics_encoding::param::{Bolt, Brass, L15, SizeM6};
use exp05_characteristics_encoding::requirements::fasten_param;

fn main() {
    let _ = fasten_param(Bolt::<SizeM6, Brass, L15>::new());
}
