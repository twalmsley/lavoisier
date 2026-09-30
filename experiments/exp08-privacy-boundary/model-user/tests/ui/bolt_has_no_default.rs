// Real resources derive neither Default nor Clone, so the derive holes are
// closed on them: there is no `Bolt::default()`.
use model_lib::resources::Bolt;

fn main() {
    let _bolt = Bolt::default();
}
