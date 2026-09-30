//! Processes (R1): pure transformations, resources in by value, out by value.
//! Note that this module needs **no** access to private fields — it only moves
//! resources around — so it sits outside the `resources` module tree. It uses
//! the one `pub(crate)` wrapper (`FastenedAssembly::assemble`), which cannot
//! create resources, only combine ones it is given.

use crate::resources::{Bolt, FastenedAssembly, Plate};

/// Fasten two plates with one bolt. Consumes all three, returns the assembly.
pub fn fasten(bolt: Bolt, top: Plate, bottom: Plate) -> FastenedAssembly {
    FastenedAssembly::assemble(bolt, top, bottom)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::boundary;

    #[test]
    fn fasten_conserves_everything() {
        let [b0, _b1, _b2, _b3] = boundary::procure_bolt_box().open();
        let assembly = fasten(b0, boundary::procure_plate(), boundary::procure_plate());
        let (_bolt, _top, _bottom) = assembly.disassemble();
        // (_b1.._b3 are silently dropped here — the affine-types gap, EXP-03's topic.)
    }
}
