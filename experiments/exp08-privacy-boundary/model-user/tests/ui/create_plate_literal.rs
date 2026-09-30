// Struct-literal syntax must also fail: the seal field is private.
use model_lib::resources::Plate;

fn main() {
    let _plate = Plate { _seal: () };
}
