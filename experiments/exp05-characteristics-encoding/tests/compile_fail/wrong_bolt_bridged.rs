//! A parameterized M6 brass bolt passed to the *marker-style* requirement:
//! the bridge makes the call legal in form, so the error reports the missing
//! marker traits on the Bolt<...> type.
use exp05_characteristics_encoding::param::{Bolt, Brass, L15, SizeM6};
use exp05_characteristics_encoding::requirements::fasten_marker;

fn main() {
    let _ = fasten_marker(Bolt::<SizeM6, Brass, L15>::new());
}
