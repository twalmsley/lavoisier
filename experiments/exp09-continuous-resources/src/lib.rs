//! EXP-09: Continuous resources, time budgets, and boundary sinks (R15).
//!
//! Tests the combination of:
//! - sealed quantity containers (`GasBottle<REMAINING>`) drawn down by R3-style
//!   split processes with the caller-stated remainder (EXP-04 pattern);
//! - a time budget on a reusable resource (`Person<BUDGET_MS>`);
//! - a two-dimension balancing process (`burn`: mass **and** energy asserts);
//! - unbounded boundary objects (`Atmosphere`) with `type Next = Self`, used as
//!   a multi-`Consumer` sink and probed as an air *source* in three styles
//!   (fixed-packet `Supplier`, parameterized `SupplierOf<Out>`, and a plain
//!   `draw_air` process).
//!
//! Conservation conventions per R1: `#[must_use]` on every resource, crate-wide
//! deny lints, and a tripwire `Drop` on every consumable resource (defused only
//! by the sealed module's `defuse`, the single allowed `mem::forget` site per
//! resource, F-008).
//!
//! # A correct end-to-end flow (runs as a doc-test)
//!
//! ```
//! use exp09_continuous_resources::model::boundary::{
//!     draw_air, fill_gas_bottle, new_depot, new_ledger, new_person, new_work_sink,
//!     the_atmosphere,
//! };
//! use exp09_continuous_resources::model::processes::{burn, draw_gas, draw_time};
//! use exp09_continuous_resources::boundary_traits::send_to;
//!
//! let bottle = fill_gas_bottle::<5000>();
//! let person = new_person::<10000>();
//! let atm = the_atmosphere();
//!
//! let (gas, bottle) = draw_gas::<300, 4700, 5000>(bottle);
//! let (air, atm) = draw_air::<300>(atm);
//! let (labour, person) = draw_time::<2000, 8000, 10000>(person);
//! let ledger = send_to(new_ledger(), labour);
//!
//! let (exhaust, heat, work) = burn::<300, 300, 600, 11000, 4000>(gas, air);
//!
//! let atm = send_to(atm, exhaust);
//! let atm = send_to(atm, heat);
//! let sink = send_to(new_work_sink(), work);
//! let depot = send_to(new_depot(), bottle); // the part-empty bottle is accounted for
//! # let _keep = (person, atm, ledger, sink, depot);
//! ```
//!
//! # Conservation-convention compile-fail cases
//!
//! Discarding a draw's entire result is caught at compile time by the
//! `#[must_use]` layer (the lint sees through the tuple):
//!
//! ```compile_fail
//! #![deny(unused_must_use)] // the project's crate-wide lint (R1)
//! use exp09_continuous_resources::model::boundary::fill_gas_bottle;
//! use exp09_continuous_resources::model::processes::draw_gas;
//! let bottle = fill_gas_bottle::<5000>();
//! draw_gas::<300, 4700, 5000>(bottle); // result discarded: error
//! ```
//!
//! Waste heat bound to a name but never consumed is caught by
//! `unused_variables` — only under the CI `-D warnings` gate (the lint is
//! warn-by-default), stood in for here by `#![deny(unused_variables)]`:
//!
//! ```compile_fail
//! #![deny(unused_variables)] // stands in for CI's `-D warnings` gate
//! use exp09_continuous_resources::model::boundary::{
//!     draw_air, fill_gas_bottle, new_work_sink, the_atmosphere,
//! };
//! use exp09_continuous_resources::model::processes::{burn, draw_gas};
//! use exp09_continuous_resources::boundary_traits::send_to;
//! let bottle = fill_gas_bottle::<300>();
//! let (gas, empty) = draw_gas::<300, 0, 300>(bottle);
//! let atm = the_atmosphere();
//! let (air, atm) = draw_air::<300>(atm);
//! let (exhaust, heat, work) = burn::<300, 300, 600, 11000, 4000>(gas, air);
//! let _atm = send_to(atm, exhaust);
//! let _sink = send_to(new_work_sink(), work);
//! # let _accounted = empty;
//! // `heat` is never sent to a consumer: error under the deny lint.
//! ```
//!
//! A waste product that is *used at least once* and then dropped is invisible
//! to every compile-time layer (F-002); the tripwire `Drop` catches it at test
//! time — see `tests/integration.rs`.

#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![deny(let_underscore_drop)]

/// The resource family module (R1/F-006 layout): sealed types, with the
/// creation boundary and the conserving processes as child modules.
///
/// Note a layout consequence of R15 recorded in RESULTS.md: unlike discrete
/// processes (which only move existing objects), continuous-resource processes
/// (`draw_gas`, `burn`, `combine_air`) must *construct* new quantity-bearing
/// values, so they must live inside the privacy boundary (or be handed sealed
/// `pub(crate)` mint helpers). They cannot live outside the resource module
/// tree as F-006 prescribes for discrete processes.
pub mod model {
    use crate::boundary_traits::{Consumer, Supplier, SupplierOf};

    /// Energy content of the modelled gas, in joules per gram (model constant).
    pub const GAS_ENERGY_J_PER_G: u64 = 50;

    /// Mass of one fixed air packet supplied by the `Supplier` impl on
    /// [`Atmosphere`], in grams (design probe (a) of EXP-09 step 5).
    pub const AIR_PACKET_G: u64 = 100;

    /// Generates a sealed, tripwired, consumable quantity-bearing resource:
    /// a struct with a private field, `#[must_use]`, an associated `VALUE`
    /// constant (R7), a `pub(in crate::model)` `mint`, and a tripwire `Drop`
    /// defused only by `defuse` (the single allowed `mem::forget` site for the
    /// type, F-008).
    macro_rules! conserved_quantity {
        ($(#[$doc:meta])* $name:ident, $unit:literal, $must_use_msg:literal) => {
            $(#[$doc])*
            #[must_use = $must_use_msg]
            pub struct $name<const V: u64> {
                _seal: (),
            }

            impl<const V: u64> $name<V> {
                /// The magnitude, in the type's base unit.
                pub const VALUE: u64 = V;

                /// The base unit of this resource's magnitude.
                pub const UNIT: &'static str = $unit;

                pub(in crate::model) fn mint() -> Self {
                    $name { _seal: () }
                }

                /// Defuses the tripwire. The single allowed forget site for
                /// this resource (F-008); callable only inside the boundary.
                pub(in crate::model) fn defuse(self) {
                    #[allow(clippy::mem_forget)]
                    core::mem::forget(self);
                }
            }

            impl<const V: u64> Drop for $name<V> {
                fn drop(&mut self) {
                    if !std::thread::panicking() {
                        panic!(
                            "resource leak: {}<{}> dropped without being consumed (R1 conservation)",
                            stringify!($name),
                            V
                        );
                    }
                }
            }
        };
    }

    conserved_quantity!(
        /// Drawn gas (fuel), in grams.
        Gas,
        "grams",
        "Gas is a conserved resource: pass it on or hand it to a Consumer"
    );
    conserved_quantity!(
        /// Drawn air, in grams.
        Air,
        "grams",
        "Air is a conserved resource: pass it on or hand it to a Consumer"
    );
    conserved_quantity!(
        /// Exhaust gas (waste), in grams.
        ExhaustGas,
        "grams",
        "ExhaustGas is a conserved waste output: it must reach a Consumer"
    );
    conserved_quantity!(
        /// Waste heat, in joules.
        WasteHeat,
        "joules",
        "WasteHeat is a conserved waste output: it must reach a Consumer"
    );
    conserved_quantity!(
        /// Useful work, in joules.
        Work,
        "joules",
        "Work is a conserved output: pass it on or hand it to a Consumer"
    );
    conserved_quantity!(
        /// Expended labour, in person-milliseconds.
        Labour,
        "person-milliseconds",
        "Labour is a conserved resource: it must be accounted for (e.g. a Ledger)"
    );
    conserved_quantity!(
        /// A gas bottle holding `REMAINING` grams of gas. `GasBottle<0>` is the
        /// empty state: a distinct resource type that must still be accounted
        /// for, exactly like `EmptyBoltBox` (R12/R15).
        GasBottle,
        "grams remaining",
        "GasBottle is a conserved resource: even an empty bottle must be passed on or handed to a Consumer"
    );

    /// The empty state of the gas bottle — a distinct resource type (R15).
    pub type EmptyGasBottle = GasBottle<0>;

    /// A person with a remaining time budget of `BUDGET_MS` person-milliseconds
    /// (R15 time budgets). Reusable (R2): moved in and returned by every
    /// process that uses them; no tripwire `Drop` (the tripwire regime applies
    /// to consumables — a reusable resource legitimately outlives the flow and
    /// remains with the caller).
    #[must_use = "Person is a reusable resource: pass them on or return them to the caller"]
    pub struct Person<const BUDGET_MS: u64> {
        _seal: (),
    }

    impl<const BUDGET_MS: u64> Person<BUDGET_MS> {
        /// Remaining time budget, in person-milliseconds.
        pub const BUDGET_MS: u64 = BUDGET_MS;
    }

    /// The atmosphere: an unbounded boundary object, `type Next = Self` in
    /// every boundary-trait impl.
    ///
    /// Placeholder: atmosphere — assumed unbounded source of air and unbounded
    /// sink for exhaust gas and waste heat.
    #[must_use = "Atmosphere is a boundary resource: pass it on like any other resource"]
    pub struct Atmosphere {
        _seal: (),
    }

    /// A depot taking back gas bottles (any fill state).
    ///
    /// Placeholder: bottle depot — assumed unbounded sink for returned bottles.
    #[must_use = "Depot is a boundary resource: pass it on like any other resource"]
    pub struct Depot {
        _seal: (),
    }

    /// A ledger accounting for expended labour.
    ///
    /// Placeholder: labour ledger — assumed unbounded sink for spent time.
    #[must_use = "Ledger is a boundary resource: pass it on like any other resource"]
    pub struct Ledger {
        _seal: (),
    }

    /// A sink for delivered useful work (e.g. the customer).
    ///
    /// Placeholder: work customer — assumed unbounded sink for delivered work.
    #[must_use = "WorkSink is a boundary resource: pass it on like any other resource"]
    pub struct WorkSink {
        _seal: (),
    }

    // ------------------------------------------------------------------
    // Boundary-trait impls for the unbounded boundary objects (Next = Self).
    // ------------------------------------------------------------------

    /// The atmosphere consumes exhaust gas of any mass; `Next = Self`.
    impl<const M: u64> Consumer<ExhaustGas<M>> for Atmosphere {
        type Next = Atmosphere;
        fn consume(self, item: ExhaustGas<M>) -> Atmosphere {
            item.defuse();
            self
        }
    }

    /// The atmosphere also consumes waste heat of any magnitude; `Next = Self`.
    /// (Multiple `Consumer<In>` impls on one boundary object — EXP-09 step 5.)
    impl<const J: u64> Consumer<WasteHeat<J>> for Atmosphere {
        type Next = Atmosphere;
        fn consume(self, item: WasteHeat<J>) -> Atmosphere {
            item.defuse();
            self
        }
    }

    /// Design probe (a): fixed-packet supply. The R12 `Supplier` trait has one
    /// `Item` per impl, and the trait is not parameterized, so the atmosphere
    /// can supply air only in fixed packets of [`AIR_PACKET_G`] grams.
    impl Supplier for Atmosphere {
        type Item = Air<AIR_PACKET_G>;
        type Next = Atmosphere;
        fn supply(self) -> (Air<AIR_PACKET_G>, Atmosphere) {
            (Air::mint(), self)
        }
    }

    /// Design probe (c): a parameterized `SupplierOf<Out>` — "draw in trait
    /// clothing". Legal for an *unbounded* source only: a finite container
    /// cannot implement this shape on stable (the next state would have to be
    /// computed from the amount drawn — see `tests/ui/`).
    impl<const G: u64> SupplierOf<Air<G>> for Atmosphere {
        type Next = Atmosphere;
        fn supply_of(self) -> (Air<G>, Atmosphere) {
            (Air::mint(), self)
        }
    }

    /// The depot accepts a returned bottle in any fill state; `Next = Self`.
    impl<const R: u64> Consumer<GasBottle<R>> for Depot {
        type Next = Depot;
        fn consume(self, item: GasBottle<R>) -> Depot {
            item.defuse();
            self
        }
    }

    /// The ledger accounts for any amount of expended labour; `Next = Self`.
    impl<const MS: u64> Consumer<Labour<MS>> for Ledger {
        type Next = Ledger;
        fn consume(self, item: Labour<MS>) -> Ledger {
            item.defuse();
            self
        }
    }

    /// The work sink accepts any amount of delivered work; `Next = Self`.
    impl<const J: u64> Consumer<Work<J>> for WorkSink {
        type Next = WorkSink;
        fn consume(self, item: Work<J>) -> WorkSink {
            item.defuse();
            self
        }
    }

    /// The creation boundary (R12): the only production code allowed to create
    /// resources, plus the boundary-source draw process for the unbounded
    /// atmosphere (design probe (b)).
    pub mod boundary {
        use super::{Atmosphere, Air, Depot, GasBottle, Ledger, Person, WorkSink};

        /// Fills a gas bottle with `FULL` grams of gas at the system boundary.
        ///
        /// Placeholder: gas supplier — assumed able to deliver a full bottle.
        pub fn fill_gas_bottle<const FULL: u64>() -> GasBottle<FULL> {
            GasBottle::mint()
        }

        /// A person enters the model with a time budget (R15: model a budget
        /// only where the time is genuinely being accounted for).
        ///
        /// Placeholder: workforce — one person with a fixed shift budget.
        pub fn new_person<const BUDGET_MS: u64>() -> Person<BUDGET_MS> {
            Person { _seal: () }
        }

        /// Placeholder: atmosphere — assumed unbounded source of air and
        /// unbounded sink for exhaust and heat.
        pub fn the_atmosphere() -> Atmosphere {
            Atmosphere { _seal: () }
        }

        /// Placeholder: bottle depot — assumed unbounded sink.
        pub fn new_depot() -> Depot {
            Depot { _seal: () }
        }

        /// Placeholder: labour ledger — assumed unbounded sink.
        pub fn new_ledger() -> Ledger {
            Ledger { _seal: () }
        }

        /// Placeholder: work customer — assumed unbounded sink.
        pub fn new_work_sink() -> WorkSink {
            WorkSink { _seal: () }
        }

        /// Design probe (b): a draw-style boundary process for the unbounded
        /// air source, bypassing the `Supplier` trait. The caller states the
        /// amount; the atmosphere's next state is itself.
        ///
        /// Placeholder: atmosphere — assumed unbounded source of air.
        pub fn draw_air<const TAKE: u64>(atm: Atmosphere) -> (Air<TAKE>, Atmosphere) {
            (Air::mint(), atm)
        }
    }

    /// The conserving processes over the resource family (R1/R3). Inside the
    /// module tree because they must mint the new quantity-bearing values they
    /// return (see the module-level note).
    pub mod processes {
        use super::{
            Air, ExhaustGas, Gas, GasBottle, Labour, Person, WasteHeat, Work, GAS_ENERGY_J_PER_G,
        };

        /// Draws `TAKE` grams from a bottle holding `FULL` grams, leaving
        /// `LEFT`. R3 caller-stated-remainder pattern; the conservation assert
        /// fires at monomorphization (R4 caveat: invisible to `cargo check`).
        ///
        /// Overdrawing is a compile error — no `LEFT` satisfies
        /// `TAKE + LEFT == FULL` when `TAKE > FULL`:
        ///
        /// ```compile_fail
        /// use exp09_continuous_resources::model::boundary::fill_gas_bottle;
        /// use exp09_continuous_resources::model::processes::draw_gas;
        /// let bottle = fill_gas_bottle::<5000>();
        /// // 6000 g from a 5000 g bottle: E0080 at monomorphization.
        /// let (gas, rest) = draw_gas::<6000, 0, 5000>(bottle);
        /// ```
        ///
        /// The empty state is unusable the same way:
        ///
        /// ```compile_fail
        /// use exp09_continuous_resources::model::boundary::fill_gas_bottle;
        /// use exp09_continuous_resources::model::processes::draw_gas;
        /// let empty = fill_gas_bottle::<0>();
        /// let (gas, still_empty) = draw_gas::<1, 0, 0>(empty);
        /// ```
        pub fn draw_gas<const TAKE: u64, const LEFT: u64, const FULL: u64>(
            bottle: GasBottle<FULL>,
        ) -> (Gas<TAKE>, GasBottle<LEFT>) {
            const {
                assert!(
                    TAKE + LEFT == FULL,
                    "conservation violated in draw_gas: TAKE + LEFT must equal FULL — is the draw larger than the bottle's remaining contents?"
                )
            };
            bottle.defuse();
            (Gas::mint(), GasBottle::mint())
        }

        /// Draws `SPEND` person-milliseconds from a person's time budget,
        /// leaving `LEFT`. Same pattern as [`draw_gas`]; the expended time
        /// leaves as a conserved `Labour` value that must be accounted for.
        ///
        /// Overspending the budget fails exactly like the bottle:
        ///
        /// ```compile_fail
        /// use exp09_continuous_resources::model::boundary::new_person;
        /// use exp09_continuous_resources::model::processes::draw_time;
        /// let person = new_person::<10000>();
        /// let (l1, person) = draw_time::<6000, 4000, 10000>(person);
        /// // Only 4000 ms left; drawing another 6000 ms is E0080.
        /// let (l2, person) = draw_time::<6000, 0, 4000>(person);
        /// ```
        pub fn draw_time<const SPEND: u64, const LEFT: u64, const BUDGET: u64>(
            person: Person<BUDGET>,
        ) -> (Labour<SPEND>, Person<LEFT>) {
            const {
                assert!(
                    SPEND + LEFT == BUDGET,
                    "time budget violated in draw_time: SPEND + LEFT must equal BUDGET — is more time being spent than the person has left?"
                )
            };
            let Person { _seal: () } = person; // reusable resource: plain move, no tripwire
            (Labour::mint(), Person { _seal: () })
        }

        /// A process that takes the person but spends none of their time —
        /// demonstrates that the budget const parameter still infects the
        /// signature (RESULTS.md, threading ergonomics).
        pub fn walk_to_station<const BUDGET: u64>(person: Person<BUDGET>) -> Person<BUDGET> {
            person
        }

        /// Combines two amounts of air (R3); the caller states the total
        /// (F-022: outputs cannot be computed on stable).
        pub fn combine_air<const A: u64, const B: u64, const TOTAL: u64>(
            a: Air<A>,
            b: Air<B>,
        ) -> Air<TOTAL> {
            const {
                assert!(
                    A + B == TOTAL,
                    "conservation violated in combine_air: A + B must equal TOTAL"
                )
            };
            a.defuse();
            b.defuse();
            Air::mint()
        }

        /// Burns fuel in air: the two-dimension balancing process (EXP-09
        /// step 4). Two independent conservation asserts in one process:
        /// mass (fuel + air = exhaust) and energy (fuel energy = heat + work).
        ///
        /// A mass violation fails to compile:
        ///
        /// ```compile_fail
        /// use exp09_continuous_resources::model::boundary::{draw_air, fill_gas_bottle, the_atmosphere};
        /// use exp09_continuous_resources::model::processes::{burn, draw_gas};
        /// let bottle = fill_gas_bottle::<1000>();
        /// let (gas, bottle) = draw_gas::<300, 700, 1000>(bottle);
        /// let (air, atm) = draw_air::<400>(the_atmosphere());
        /// // 300 g + 400 g != 600 g: the mass assert fails (E0080).
        /// let (exhaust, heat, work) = burn::<300, 400, 600, 11000, 4000>(gas, air);
        /// ```
        ///
        /// An energy violation fails to compile independently (mass correct):
        ///
        /// ```compile_fail
        /// use exp09_continuous_resources::model::boundary::{draw_air, fill_gas_bottle, the_atmosphere};
        /// use exp09_continuous_resources::model::processes::{burn, draw_gas};
        /// let bottle = fill_gas_bottle::<1000>();
        /// let (gas, bottle) = draw_gas::<300, 700, 1000>(bottle);
        /// let (air, atm) = draw_air::<300>(the_atmosphere());
        /// // 300 g of gas carries 15000 J, but 11000 + 9000 = 20000 J: E0080.
        /// let (exhaust, heat, work) = burn::<300, 300, 600, 11000, 9000>(gas, air);
        /// ```
        pub fn burn<
            const FUEL_G: u64,
            const AIR_G: u64,
            const EXHAUST_G: u64,
            const HEAT_J: u64,
            const WORK_J: u64,
        >(
            fuel: Gas<FUEL_G>,
            air: Air<AIR_G>,
        ) -> (ExhaustGas<EXHAUST_G>, WasteHeat<HEAT_J>, Work<WORK_J>) {
            const {
                assert!(
                    FUEL_G + AIR_G == EXHAUST_G,
                    "mass conservation violated in burn: fuel + air must equal exhaust"
                )
            };
            const {
                assert!(
                    FUEL_G * GAS_ENERGY_J_PER_G == HEAT_J + WORK_J,
                    "energy conservation violated in burn: the fuel's energy must equal waste heat + work"
                )
            };
            fuel.defuse();
            air.defuse();
            (ExhaustGas::mint(), WasteHeat::mint(), Work::mint())
        }

        /// Identity pass-through used by the tests to "use" a value at least
        /// once before (wrongly) dropping it — demonstrating the F-002 gap
        /// that only the tripwire catches.
        pub fn inspect_heat<const J: u64>(heat: WasteHeat<J>) -> WasteHeat<J> {
            heat
        }
    }
}

/// The R12 boundary traits, with `#[diagnostic::on_unimplemented]` messages
/// phrased for modellers (F-015), plus the repeated-use helper traits
/// (`SupplyN`, `ConsumeList`, F-014) and the generic access processes that
/// model code must use instead of direct method calls (F-015).
pub mod boundary_traits {
    use core::marker::PhantomData;

    /// R12 supplier: supplies exactly one fixed `Item` per step.
    #[diagnostic::on_unimplemented(
        message = "`{Self}` cannot supply anything: it is exhausted, or it is not a supplier of discrete items",
        label = "not a usable supplier here"
    )]
    pub trait Supplier {
        type Item;
        type Next;
        fn supply(self) -> (Self::Item, Self::Next);
    }

    /// Design probe (c): a supplier parameterized by what it supplies, so one
    /// boundary object can supply a caller-chosen amount. See RESULTS.md —
    /// only unbounded (`Next = Self`) sources can implement this shape.
    #[diagnostic::on_unimplemented(
        message = "`{Self}` cannot supply `{Out}`: it is exhausted, or it does not supply this resource"
    )]
    pub trait SupplierOf<Out> {
        type Next;
        fn supply_of(self) -> (Out, Self::Next);
    }

    /// R12 consumer: consumes exactly one item per step.
    #[diagnostic::on_unimplemented(
        message = "`{Self}` cannot consume `{In}`: it is full, or it does not accept this kind of resource",
        label = "this boundary object does not take this resource"
    )]
    pub trait Consumer<In> {
        type Next;
        fn consume(self, item: In) -> Self::Next;
    }

    /// Type-level zero (minimal Peano copy, per the no-sharing protocol).
    pub struct Zero;
    /// Type-level successor.
    pub struct Succ<N>(PhantomData<N>);

    /// A heterogeneous list of taken/consumed items (value-carrying).
    #[must_use = "this list carries conserved resources: pass them on or hand them to a Consumer"]
    pub struct Cons<H, T>(pub H, pub T);
    /// The empty list.
    pub struct Nil;

    /// Repeated supply (F-014): take `N` items from one supplier.
    #[diagnostic::on_unimplemented(
        message = "`{Self}` cannot supply this many items: it would be exhausted partway"
    )]
    pub trait SupplyN<N> {
        type Taken;
        type Rest;
        fn supply_n(self) -> (Self::Taken, Self::Rest);
    }

    impl<S> SupplyN<Zero> for S {
        type Taken = Nil;
        type Rest = S;
        fn supply_n(self) -> (Nil, S) {
            (Nil, self)
        }
    }

    impl<S: Supplier, N> SupplyN<Succ<N>> for S
    where
        S::Next: SupplyN<N>,
    {
        type Taken = Cons<S::Item, <S::Next as SupplyN<N>>::Taken>;
        type Rest = <S::Next as SupplyN<N>>::Rest;
        fn supply_n(self) -> (Self::Taken, Self::Rest) {
            let (item, next) = self.supply();
            let (rest_taken, rest) = next.supply_n();
            (Cons(item, rest_taken), rest)
        }
    }

    /// Repeated consumption (F-014): feed a whole list to one consumer.
    #[diagnostic::on_unimplemented(
        message = "`{Self}` cannot consume this list of items: it is full partway, or it does not accept one of them"
    )]
    pub trait ConsumeList<L> {
        type Next;
        fn consume_list(self, items: L) -> Self::Next;
    }

    impl<C> ConsumeList<Nil> for C {
        type Next = C;
        fn consume_list(self, Nil: Nil) -> C {
            self
        }
    }

    impl<C: Consumer<H>, H, T> ConsumeList<Cons<H, T>> for C
    where
        C::Next: ConsumeList<T>,
    {
        type Next = <C::Next as ConsumeList<T>>::Next;
        fn consume_list(self, items: Cons<H, T>) -> Self::Next {
            let Cons(head, tail) = items;
            self.consume(head).consume_list(tail)
        }
    }

    // Generic access processes: model code reaches boundary objects through
    // these bounds, never by direct method calls (F-015 — the E0599 path
    // bypasses `on_unimplemented`).

    /// Hand one item to a consumer.
    pub fn send_to<I, C: Consumer<I>>(consumer: C, item: I) -> C::Next {
        consumer.consume(item)
    }

    /// Hand a whole list of items to one consumer.
    pub fn send_list<L, C: ConsumeList<L>>(consumer: C, items: L) -> C::Next {
        consumer.consume_list(items)
    }

    /// Take one item from a supplier.
    pub fn take_one<S: Supplier>(supplier: S) -> (S::Item, S::Next) {
        supplier.supply()
    }

    /// Take `N` items from one supplier.
    pub fn take_n<N, S: SupplyN<N>>(supplier: S) -> (S::Taken, S::Rest) {
        supplier.supply_n()
    }

    /// Draw a caller-chosen output from a parameterized supplier (probe (c)).
    pub fn source<O, S: SupplierOf<O>>(supplier: S) -> (O, S::Next) {
        supplier.supply_of()
    }
}
