// A downstream crate must not create a Bolt from nothing:
// the tuple-struct constructor is private (private `()` field).
use model_lib::resources::Bolt;

fn main() {
    let _bolt = Bolt(());
}
