//! The downstream modelling crate: uses `model-lib`'s boundary and processes
//! end to end, but can never create a resource itself.

use model_lib::resources::FastenedAssembly;
use model_lib::{boundary, processes};

/// Run one full process chain: procure resources at the boundary, fasten,
/// return the product plus the leftover bolts (conservation: nothing vanishes).
pub fn assemble_one() -> (FastenedAssembly, [model_lib::resources::Bolt; 3]) {
    let [b0, b1, b2, b3] = boundary::procure_bolt_box().open();
    let top = boundary::procure_plate();
    let bottom = boundary::procure_plate();
    let assembly = processes::fasten(b0, top, bottom);
    (assembly, [b1, b2, b3])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// End-to-end: the user crate can run the whole process without ever
    /// constructing a resource.
    #[test]
    fn end_to_end_assembly_works() {
        let (assembly, spare) = assemble_one();
        let (_bolt, _top, _bottom) = assembly.disassemble();
        let [_s1, _s2, _s3] = spare;
    }
}
