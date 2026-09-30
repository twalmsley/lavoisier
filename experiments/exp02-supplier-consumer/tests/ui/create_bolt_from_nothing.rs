//! The privacy boundary (R1): a bolt cannot be created from nothing
//! outside the boundary module.

use exp02_supplier_consumer::*;

fn main() {
    let _bolt = Bolt { _private: () };
}
