//! EXP-07: Flows, exclusive resources, reusable resources.
//!
//! Tests R2 (strict conservation: resources are never shared, reusable
//! resources are moved in and returned) and R9 (the model describes
//! connections, not sequences) on a mini-model: cut a steel sheet into
//! plates, drill them, fasten them with four bolts.
//!
//! Deliberately independent of EXP-02: the bolts are a plain `[Bolt; 4]`,
//! not a type-level-list supplier.

#![deny(unused_must_use)]

pub mod resources {
    //! Resource types with private constructors (R1).
    //!
    //! All fields are private, so code outside this module cannot create a
    //! resource from nothing. The only creation points are the `boundary`
    //! child module (suppliers, R12) and the conserving conversion methods
    //! below, each of which consumes an existing resource by value.
    //!
    //! None of these types implement `Clone` or `Copy`, and all are
    //! `#[must_use]`, so silently dropping one is a lint error under
    //! `#![deny(unused_must_use)]` (the known affine-types gap remains for
    //! values moved into a swallowing function — see EXP-03).

    /// A quantity of sheet steel, mass in grams (integer, R7 base unit).
    #[must_use]
    pub struct SteelSheet {
        grams: u64,
    }

    /// A reusable resource: a person who operates processes.
    #[must_use]
    pub struct Person {
        name: &'static str,
    }

    /// A reusable resource: a drill.
    #[must_use]
    pub struct Drill {
        _private: (),
    }

    /// A cut, undrilled plate.
    #[must_use]
    pub struct Plate {
        grams: u64,
    }

    /// A plate with holes drilled in it. Distinct type from `Plate`, so a
    /// flow cannot fasten a plate that was never drilled.
    #[must_use]
    pub struct DrilledPlate {
        grams: u64,
    }

    /// One bolt. A discrete item (R13); a handful is `[Bolt; 4]`.
    #[must_use]
    pub struct Bolt {
        grams: u64,
    }

    /// The final product: two drilled plates fastened with four bolts.
    #[must_use]
    pub struct Assembly {
        grams: u64,
    }

    /// Waste product from cutting.
    #[must_use]
    pub struct Offcut {
        grams: u64,
    }

    impl SteelSheet {
        pub fn grams(&self) -> u64 {
            self.grams
        }

        /// Conserving conversion used by the `cut` process: consumes the
        /// sheet, returns two plates and the offcut. Total mass is
        /// preserved by construction (checked by a unit test as well).
        pub(crate) fn cut_into_plates(self) -> (Plate, Plate, Offcut) {
            let plate_g = (self.grams * 2) / 5; // 2000 g -> 800 + 800 + 400
            let offcut_g = self.grams - 2 * plate_g;
            (
                Plate { grams: plate_g },
                Plate { grams: plate_g },
                Offcut { grams: offcut_g },
            )
        }
    }

    impl Person {
        pub fn name(&self) -> &'static str {
            self.name
        }
    }

    impl Plate {
        pub fn grams(&self) -> u64 {
            self.grams
        }

        /// Conserving conversion used by the `drill_holes` process.
        /// (A stricter model would also return swarf; the experiment brief
        /// fixes the signature to `(person, drill, drilled_plate)`.)
        pub(crate) fn into_drilled(self) -> DrilledPlate {
            DrilledPlate { grams: self.grams }
        }
    }

    impl DrilledPlate {
        pub fn grams(&self) -> u64 {
            self.grams
        }
    }

    impl Bolt {
        pub fn grams(&self) -> u64 {
            self.grams
        }
    }

    impl Assembly {
        pub fn grams(&self) -> u64 {
            self.grams
        }

        /// Conserving conversion used by the `fasten` process.
        pub(crate) fn assemble(a: DrilledPlate, b: DrilledPlate, bolts: [Bolt; 4]) -> Assembly {
            let bolt_g: u64 = bolts.iter().map(|b| b.grams).sum();
            // The bolts are consumed by value into the assembly.
            let [_, _, _, _] = bolts;
            Assembly {
                grams: a.grams + b.grams + bolt_g,
            }
        }
    }

    impl Offcut {
        pub fn grams(&self) -> u64 {
            self.grams
        }
    }

    pub mod boundary {
        //! The system boundary (R12): the only place resources are created
        //! from nothing (suppliers) or leave the model (consumers).
        //! As a child module it can see the parent's private fields.
        //! Kept as plain placeholder functions — the full `Supplier` /
        //! `Consumer` trait machinery is EXP-02's subject, not this one's.

        use super::*;

        /// Placeholder supplier: a 2000 g steel sheet enters the model.
        pub fn supply_sheet() -> SteelSheet {
            SteelSheet { grams: 2000 }
        }

        /// Placeholder supplier: a worker enters the model.
        pub fn supply_person(name: &'static str) -> Person {
            Person { name }
        }

        /// Placeholder supplier: a drill enters the model.
        pub fn supply_drill() -> Drill {
            Drill { _private: () }
        }

        /// Placeholder supplier: a handful of four 10 g bolts.
        pub fn supply_bolts() -> [Bolt; 4] {
            [(); 4].map(|_| Bolt { grams: 10 })
        }

        /// Placeholder consumer: the finished assembly leaves the model.
        /// Returns the shipped mass so tests can account for it.
        pub fn ship(assembly: Assembly) -> u64 {
            assembly.grams
        }

        /// Placeholder consumer: scrap disposal for offcuts.
        pub fn scrap(offcut: Offcut) -> u64 {
            offcut.grams
        }
    }
}

pub mod processes {
    //! Processes (R1): pure by-value transformations. Reusable resources
    //! (Person, Drill) are moved in and moved back out (R2).

    use crate::resources::*;

    /// Cut a sheet into two plates plus offcut waste.
    pub fn cut(sheet: SteelSheet) -> (Plate, Plate, Offcut) {
        sheet.cut_into_plates()
    }

    /// Drill the bolt holes in one plate. Needs a person and a drill;
    /// both are returned to the caller afterwards (R2).
    pub fn drill_holes(person: Person, drill: Drill, plate: Plate) -> (Person, Drill, DrilledPlate) {
        (person, drill, plate.into_drilled())
    }

    /// Fasten two drilled plates together with four bolts. Needs a person
    /// (but no drill); the person is returned afterwards (R2).
    pub fn fasten(
        person: Person,
        a: DrilledPlate,
        b: DrilledPlate,
        bolts: [Bolt; 4],
    ) -> (Person, Assembly) {
        (person, Assembly::assemble(a, b, bolts))
    }
}

pub mod workshop {
    //! Aggregation variant (experiment step 4): the reusable resources are
    //! carried in one `Workshop` struct instead of as loose values.

    use crate::processes;
    use crate::resources::*;

    /// The reusable resources of the shop, threaded through as one value.
    #[must_use]
    pub struct Workshop {
        pub person: Person,
        pub drill: Drill,
    }

    impl Workshop {
        pub fn new(person: Person, drill: Drill) -> Workshop {
            Workshop { person, drill }
        }
    }

    /// `drill_holes`, aggregated: one value in, one value (plus product) out.
    pub fn drill_holes(ws: Workshop, plate: Plate) -> (Workshop, DrilledPlate) {
        let (person, drill, drilled) = processes::drill_holes(ws.person, ws.drill, plate);
        (Workshop { person, drill }, drilled)
    }

    /// `fasten`, aggregated. Note the cost: `fasten` does not need the
    /// drill, but the Workshop claims it anyway, so nothing else can use
    /// the drill while fastening is "running" in the flow.
    pub fn fasten(
        ws: Workshop,
        a: DrilledPlate,
        b: DrilledPlate,
        bolts: [Bolt; 4],
    ) -> (Workshop, Assembly) {
        let (person, assembly) = processes::fasten(ws.person, a, b, bolts);
        (
            Workshop {
                person,
                drill: ws.drill,
            },
            assembly,
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::processes::{cut, drill_holes, fasten};
    use crate::resources::boundary;
    use crate::workshop;

    /// Flow order A: cut, drill plate 1, drill plate 2, fasten.
    /// Loose threading of person and drill. Type-checks and conserves mass.
    #[test]
    fn flow_order_a_loose() {
        let sheet = boundary::supply_sheet();
        let mass_in = sheet.grams() + 4 * 10;
        let person = boundary::supply_person("Alice");
        let drill = boundary::supply_drill();
        let bolts = boundary::supply_bolts();

        let (p1, p2, offcut) = cut(sheet);
        let (person, drill, d1) = drill_holes(person, drill, p1);
        let (person, drill, d2) = drill_holes(person, drill, p2);
        let (person, assembly) = fasten(person, d1, d2, bolts);

        // Everything is accounted for: product + waste = inputs.
        let mass_out = boundary::ship(assembly) + boundary::scrap(offcut);
        assert_eq!(mass_in, mass_out);

        // The reusable resources come back out of the flow.
        assert_eq!(person.name(), "Alice");
        drop(person); // explicit disposal at end of test (affine gap, EXP-03)
        drop(drill);
    }

    /// Flow order B: the drilling steps swapped (plate 2 before plate 1),
    /// and the offcut scrapped before fastening instead of after. Same
    /// processes, different sequence — both orders type-check (R9).
    #[test]
    fn flow_order_b_loose() {
        let sheet = boundary::supply_sheet();
        let mass_in = sheet.grams() + 4 * 10;
        let person = boundary::supply_person("Alice");
        let drill = boundary::supply_drill();
        let bolts = boundary::supply_bolts();

        let (p1, p2, offcut) = cut(sheet);
        let scrapped = boundary::scrap(offcut);
        let (person, drill, d2) = drill_holes(person, drill, p2);
        let (person, drill, d1) = drill_holes(person, drill, p1);
        let (person, assembly) = fasten(person, d2, d1, bolts);

        let mass_out = boundary::ship(assembly) + scrapped;
        assert_eq!(mass_in, mass_out);

        drop(person);
        drop(drill);
    }

    /// The same flow with the `Workshop` aggregation threaded through.
    #[test]
    fn flow_aggregated() {
        let sheet = boundary::supply_sheet();
        let mass_in = sheet.grams() + 4 * 10;
        let ws = workshop::Workshop::new(
            boundary::supply_person("Alice"),
            boundary::supply_drill(),
        );
        let bolts = boundary::supply_bolts();

        let (p1, p2, offcut) = cut(sheet);
        let (ws, d1) = workshop::drill_holes(ws, p1);
        let (ws, d2) = workshop::drill_holes(ws, p2);
        let (ws, assembly) = workshop::fasten(ws, d1, d2, bolts);

        let mass_out = boundary::ship(assembly) + boundary::scrap(offcut);
        assert_eq!(mass_in, mass_out);

        assert_eq!(ws.person.name(), "Alice");
        drop(ws);
    }

    /// Each process conserves mass on its own.
    #[test]
    fn processes_conserve_mass() {
        let sheet = boundary::supply_sheet();
        let sheet_g = sheet.grams();
        let (p1, p2, offcut) = cut(sheet);
        assert_eq!(sheet_g, p1.grams() + p2.grams() + offcut.grams());

        let person = boundary::supply_person("Alice");
        let drill = boundary::supply_drill();
        let p1_g = p1.grams();
        let (person, drill, d1) = drill_holes(person, drill, p1);
        assert_eq!(p1_g, d1.grams());
        let (person, drill, d2) = drill_holes(person, drill, p2);

        let bolts = boundary::supply_bolts();
        let bolts_g: u64 = bolts.iter().map(|b| b.grams()).sum();
        let plates_g = d1.grams() + d2.grams();
        let (person, assembly) = fasten(person, d1, d2, bolts);
        assert_eq!(plates_g + bolts_g, assembly.grams());

        let _ = boundary::ship(assembly);
        let _ = boundary::scrap(offcut);
        drop(person);
        drop(drill);
    }
}
