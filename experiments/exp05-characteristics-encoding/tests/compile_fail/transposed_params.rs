//! Type parameters written in the wrong order. Without the Size/Material/
//! Length kind traits this would compile silently; with them it is an error
//! at the construction site.
use exp05_characteristics_encoding::param::{Bolt, L15, SizeM8, Steel};

fn main() {
    let _ = Bolt::<Steel, SizeM8, L15>::new();
}
