//! Resource types with private constructors (R1).
//!
//! Every type here has at least one private field, implements neither `Clone`
//! nor `Copy` nor `Default`, and offers no public constructor function. The
//! private field is a zero-sized `()` "seal"; it costs nothing at runtime but
//! makes literal construction a privacy error outside this module tree.

/// A single M8 steel bolt. A physical resource: cannot be created, copied or
/// cloned outside the boundary.
#[must_use]
pub struct Bolt(());

/// A steel plate with two holes, ready for fastening.
#[must_use]
pub struct Plate {
    _seal: (),
}

/// Two plates joined by a bolt — the product of `processes::fasten`.
#[must_use]
pub struct FastenedAssembly {
    bolt: Bolt,
    top: Plate,
    bottom: Plate,
}

impl FastenedAssembly {
    /// Crate-internal constructor for `processes::fasten`. `pub(crate)` is
    /// deliberate: `fasten` does not create resources from nothing — it can
    /// only wrap resources it was given by value. Conservation is preserved.
    pub(crate) fn assemble(bolt: Bolt, top: Plate, bottom: Plate) -> Self {
        FastenedAssembly { bolt, top, bottom }
    }

    /// Disassembly is also a process-shaped move: everything comes back out.
    pub fn disassemble(self) -> (Bolt, Plate, Plate) {
        (self.bolt, self.top, self.bottom)
    }
}

/// A box of bolts: a supplier at the system boundary (R12, simplified — no
/// type-level capacity here; that is EXP-02's problem).
#[must_use]
pub struct BoltBox {
    bolts: [Bolt; 4],
}

impl BoltBox {
    /// Hand over the contents. The box holds real `Bolt` objects; opening it
    /// moves them out. Consumes the box (it becomes packaging waste,
    /// simplified away here).
    pub fn open(self) -> [Bolt; 4] {
        self.bolts
    }
}

/// The system boundary: the only production code allowed to create resources.
///
/// This is a *child* module of `resources`, so it can see the private fields
/// above. If it were a sibling module it could not construct anything — the
/// boundary is enforced by the compiler even against the rest of this crate.
pub mod boundary {
    use super::{Bolt, BoltBox, Plate};

    /// Supplier: procure a full box of four bolts. This is where `Bolt`
    /// values enter the model (R12).
    pub fn procure_bolt_box() -> BoltBox {
        BoltBox {
            bolts: [Bolt(()), Bolt(()), Bolt(()), Bolt(())],
        }
    }

    /// Supplier: procure one pre-drilled plate.
    pub fn procure_plate() -> Plate {
        Plate { _seal: () }
    }
}

/// Test fixtures for *downstream* crates, behind the `test-support` feature.
///
/// Also a child module of `resources` for the same reason as `boundary`.
/// A downstream crate enables it only for its tests:
///
/// ```toml
/// [dependencies]
/// model-lib = { path = "..." }
/// [dev-dependencies]
/// model-lib = { path = "...", features = ["test-support"] }
/// ```
#[cfg(feature = "test-support")]
pub mod test_support {
    use super::{Bolt, Plate};

    /// Fixture: a bolt from nowhere. Tests only.
    pub fn bolt_fixture() -> Bolt {
        Bolt(())
    }

    /// Fixture: a plate from nowhere. Tests only.
    pub fn plate_fixture() -> Plate {
        Plate { _seal: () }
    }
}

/// Helpers for *this crate's own* tests. `#[cfg(test)]` means this module is
/// compiled only when `model-lib` itself is the test target; it is never part
/// of the library a downstream crate links against, so downstream code cannot
/// name it (demonstrated by a trybuild test in `model-user`).
#[cfg(test)]
pub mod test_helpers {
    use super::Bolt;

    pub fn bolt() -> Bolt {
        Bolt(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The library's own tests can use `test_helpers` directly.
    #[test]
    fn internal_helper_makes_a_bolt() {
        let bolt = test_helpers::bolt();
        let plate_a = boundary::procure_plate();
        let plate_b = boundary::procure_plate();
        let assembly = FastenedAssembly::assemble(bolt, plate_a, plate_b);
        let (_bolt, _a, _b) = assembly.disassemble();
    }
}
