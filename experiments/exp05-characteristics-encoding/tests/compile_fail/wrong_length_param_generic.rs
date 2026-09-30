//! Wrong material passed to the length-generic parameter-style requirement
//! (REQ-002: M8 steel, any length).
use exp05_characteristics_encoding::param::{Bolt, Brass, L20, SizeM8};
use exp05_characteristics_encoding::requirements::fasten_param_any_length;

fn main() {
    let _ = fasten_param_any_length(Bolt::<SizeM8, Brass, L20>::new());
}
