//! The resource sealing/tripwire kernel (R1, R15, F-032).
//!
//! The macros here generate, in **the invoking crate**, sealed resource types
//! that obey the R1 sealing rules by construction:
//!
//! * a struct with a private `_seal: ()` field — never a `pub` unit struct or
//!   `pub enum` (F-005);
//! * `#[must_use]` with an R1 message naming the type (conservation layer 1);
//! * **no** `Default`, `Clone`, `Copy` or deserialization derives, and no
//!   public fn returning `Self` from plain data;
//! * a `pub(crate)` `mint()` — only the defining crate's boundary modules
//!   (R12) and conserving processes may create the resource; downstream
//!   crates physically cannot (the hard boundary is the crate edge, F-006);
//! * for consumables, a guarded **tripwire `Drop`** (conservation layer 2,
//!   F-008): it panics `"resource leak: …"` unless the thread is already
//!   panicking, and is defused only by the sealed `defuse(self)` — the single
//!   commented `#[allow(clippy::mem_forget)]` site per resource (a type with
//!   `Drop` cannot be destructured, F-032);
//! * a `test_fixture()` constructor gated behind the invoking crate's
//!   `test-support` feature (R1, F-004) — never `#[cfg(test)]`, which can
//!   only ever serve the defining crate's own unit tests.
//!
//! Because the macros expand in the caller's crate, `mint`/`defuse` are
//! private to **that** crate: a downstream modelling crate that invokes
//! [`crate::consumable_resource!`] gets a resource only *its* boundary can
//! create. Invoke the macros inside the resource family's module, with the
//! family's `boundary`, `test_support` and continuous `processes` modules as
//! children (F-006, F-031).
//!
//! ## The three resource shapes
//!
//! | Macro | Shape | Tripwire |
//! |---|---|---|
//! | [`crate::consumable_resource!`] | plain discrete resource (`Bolt`) | yes |
//! | [`crate::container_resource!`] | const-magnitude continuous resource or container (`Gas<G>`, `GasBottle<REMAINING>`) (R15) | yes |
//! | [`crate::reusable_resource!`] | reusable resource (`Organisation`) — moved in and returned by every process (R2) | no — it legitimately outlives every flow and stays with the caller |
//!
//! [`crate::draw_process!`] generates the R15 draw process over two container
//! resources (caller-stated remainder, conservation assert).
//!
//! ## A worked example (runs as a doc-test)
//!
//! A continuous resource family, drawn down R15-style. The macros are invoked
//! here at the doc-test crate's top level, so `mint`/`defuse` are in scope;
//! in a real crate they live inside the family's module.
//!
//! ```
//! model_core::container_resource! {
//!     /// Drawn water, in grams.
//!     Water,
//!     unit = "grams",
//!     must_use = "Water is a conserved resource: pass it on or hand it to a Consumer"
//! }
//!
//! model_core::container_resource! {
//!     /// A tank holding `V` grams of water; `WaterTank<0>` is the empty
//!     /// state — a distinct resource that must still be accounted for (R15).
//!     WaterTank,
//!     unit = "grams remaining",
//!     must_use = "WaterTank is a conserved resource: even an empty tank must be passed on or handed to a Consumer"
//! }
//!
//! model_core::draw_process! {
//!     /// Draws `TAKE` grams from a tank holding `FULL`, leaving `LEFT`.
//!     pub fn draw_water: WaterTank => Water,
//!     assert = "conservation violated in draw_water (R15): TAKE + LEFT must equal FULL - is the draw larger than the tank's remaining contents?"
//! }
//!
//! fn main() {
//!     // The boundary fills a tank (mint is crate-private: this code could
//!     // not exist outside the defining crate).
//!     let tank: WaterTank<5000> = WaterTank::mint();
//!     let (water, tank) = draw_water::<300, 4700, 5000>(tank);
//!     assert_eq!(Water::<300>::VALUE, 300);
//!     assert_eq!(Water::<300>::UNIT, "grams");
//!     assert_eq!(WaterTank::<4700>::VALUE, 4700);
//!     // Account for everything: a real model hands these to Consumers;
//!     // here the boundary defuses them explicitly.
//!     water.defuse();
//!     tank.defuse();
//! }
//! ```
//!
//! **Overdraw is a compile error** (R15: finite capacity comes free from
//! conservation — no `LEFT` exists with `TAKE + LEFT == FULL` when
//! `TAKE > FULL`). It is an E0080 at monomorphization: invisible to
//! `cargo check` and editors (F-001), caught by `cargo build`/`cargo test`.
//! This regression is the same example drawing 6000 g from the 5000 g tank:
//!
//! ```compile_fail
//! model_core::container_resource! {
//!     /// Drawn water, in grams.
//!     Water,
//!     unit = "grams",
//!     must_use = "Water is a conserved resource: pass it on or hand it to a Consumer"
//! }
//!
//! model_core::container_resource! {
//!     /// A tank holding `V` grams of water.
//!     WaterTank,
//!     unit = "grams remaining",
//!     must_use = "WaterTank is a conserved resource: even an empty tank must be passed on or handed to a Consumer"
//! }
//!
//! model_core::draw_process! {
//!     /// Draws `TAKE` grams from a tank holding `FULL`, leaving `LEFT`.
//!     pub fn draw_water: WaterTank => Water,
//!     assert = "conservation violated in draw_water (R15): TAKE + LEFT must equal FULL - is the draw larger than the tank's remaining contents?"
//! }
//!
//! fn main() {
//!     let tank: WaterTank<5000> = WaterTank::mint();
//!     let (water, tank) = draw_water::<6000, 0, 5000>(tank);
//!     water.defuse();
//!     tank.defuse();
//! }
//! ```
//!
//! **The tripwire fires on an abandoned resource** (the only layer that
//! catches a used-then-dropped value, F-032):
//!
//! ```should_panic
//! model_core::consumable_resource! {
//!     /// A demo widget.
//!     Widget,
//!     must_use = "Widget is a conserved resource: pass it on or hand it to a Consumer"
//! }
//!
//! fn main() {
//!     let widget = Widget::mint();
//!     let _still_bound_but_never_consumed = widget;
//!     // falls out of scope here: "resource leak: Widget dropped without
//!     // being consumed (R1 conservation)"
//! }
//! ```

/// Defines a sealed, tripwired, **consumable discrete resource** (R1).
///
/// Input: doc comment(s), the type name, and the `#[must_use]` message (a
/// literal, because attribute values cannot be concatenated on stable). See
/// the [module docs](crate::resource) for everything the expansion contains
/// and a worked example.
///
/// The generated `mint`/`defuse` are `pub(crate)`: only the defining crate's
/// boundary (R12) creates the resource, and only its conserving transforms
/// and consumers defuse the tripwire. The generated `test_fixture()` is
/// available to downstream tests through the `test-support` feature (F-004).
#[macro_export]
macro_rules! consumable_resource {
    (
        $(#[$meta:meta])*
        $Name:ident,
        must_use = $msg:literal
    ) => {
        $(#[$meta])*
        ///
        /// Sealed consumable resource (R1): private field, no public
        /// constructor, no `Clone`/`Copy`/`Default`; carries a tripwire
        /// `Drop` (F-008) defused only by explicit consumption inside the
        /// defining crate's boundary.
        #[must_use = $msg]
        pub struct $Name {
            _seal: (),
        }

        impl $Name {
            /// Mints one resource. `pub(crate)`: only this crate's boundary
            /// modules (R12 suppliers and fill functions) and conserving
            /// processes may call it — "no creating resources from nothing"
            /// is a compile-time check (R1).
            // Generated kernel API: whether a given resource's mint/defuse
            // is called in a plain (no test-support) build depends on which
            // boundary/processes exist yet, so dead_code is allowed here.
            #[allow(dead_code)]
            pub(crate) fn mint() -> Self {
                $Name { _seal: () }
            }

            /// Defuses the tripwire and lets the resource go. This is the
            /// single allowed forget site for this resource (R1, F-008):
            /// every conserving transform and consumer inside the boundary
            /// calls it *after* accounting for the value. A type with `Drop`
            /// cannot be destructured (F-032), which is why this helper
            /// exists.
            #[allow(dead_code)]
            pub(crate) fn defuse(self) {
                // The one sanctioned mem::forget per resource (F-008): it is
                // exactly the complement of the tripwire Drop below.
                #[allow(clippy::mem_forget)]
                ::core::mem::forget(self);
            }

            /// Test fixture: a resource from nowhere, for downstream test
            /// code only (R1, F-004). Enable via a `[dev-dependencies]`
            /// re-declaration with the `test-support` feature — never under
            /// `[dependencies]` (ci.sh polices this).
            #[cfg(feature = "test-support")]
            #[allow(dead_code)] // fixture API surface; unused in private invocations
            pub fn test_fixture() -> Self {
                Self::mint()
            }
        }

        impl ::core::ops::Drop for $Name {
            /// Conservation tripwire (R1 layer 2, F-008): dropping this
            /// resource without explicit consumption is a leak; the panic
            /// turns it into a test failure. It stands down while the thread
            /// is already panicking (an unguarded tripwire double-panics and
            /// aborts the test binary). Note it reports this impl's line, not
            /// the leak site — keep processes small and tests per-process
            /// (R5).
            fn drop(&mut self) {
                if !::std::thread::panicking() {
                    panic!(
                        "resource leak: {} dropped without being consumed (R1 conservation)",
                        stringify!($Name)
                    );
                }
            }
        }
    };
}

/// Defines a sealed, tripwired, **const-magnitude container resource**
/// (R15): `Name<const V: u64>`, the magnitude in a fixed base unit exposed as
/// `VALUE` (R7) and the unit's name as `UNIT`.
///
/// This is the bare const-magnitude form R15 prefers for continuous
/// resources, containers and waste outputs (`Gas<G>`,
/// `GasBottle<REMAINING>`, `ExhaustGas<M>`, `WasteHeat<E>`): distinct sealed
/// types already prevent cross-dimension and cross-role mixing, and the bare
/// form prints decimal magnitudes in every diagnostic (F-027). `Name<0>` is
/// the empty state: a distinct resource type that must still be accounted
/// for, exactly like an empty bolt box (R12, R15).
///
/// Everything else matches [`crate::consumable_resource!`] (sealing, tripwire,
/// `pub(crate)` mint/defuse, feature-gated `test_fixture`). Pair with
/// [`crate::draw_process!`] for R15 draw-downs. See the
/// [module docs](crate::resource) for a worked example.
#[macro_export]
macro_rules! container_resource {
    (
        $(#[$meta:meta])*
        $Name:ident,
        unit = $unit:literal,
        must_use = $msg:literal
    ) => {
        $(#[$meta])*
        ///
        /// Sealed const-magnitude container resource (R1, R15): private
        /// field, no public constructor, no `Clone`/`Copy`/`Default`; carries
        /// a tripwire `Drop` (F-008) defused only by explicit consumption
        /// inside the defining crate's boundary. The magnitude `V` is in the
        /// type's fixed base unit (`UNIT`).
        #[must_use = $msg]
        pub struct $Name<const V: u64> {
            _seal: (),
        }

        impl<const V: u64> $Name<V> {
            /// The magnitude, in this type's base unit (R7).
            pub const VALUE: u64 = V;

            /// The base unit of this resource's magnitude (R7 table).
            pub const UNIT: &'static str = $unit;

            /// Mints one resource of magnitude `V`. `pub(crate)`: only this
            /// crate's boundary modules (R12) and conserving processes
            /// (which, for continuous resources, live inside the privacy
            /// boundary, F-031) may call it.
            // Generated kernel API: whether a given resource's mint/defuse
            // is called in a plain (no test-support) build depends on which
            // boundary/processes exist yet, so dead_code is allowed here.
            #[allow(dead_code)]
            pub(crate) fn mint() -> Self {
                $Name { _seal: () }
            }

            /// Defuses the tripwire and lets the resource go. The single
            /// allowed forget site for this resource (R1, F-008); see
            /// [`crate::consumable_resource!`].
            #[allow(dead_code)]
            pub(crate) fn defuse(self) {
                // The one sanctioned mem::forget per resource (F-008).
                #[allow(clippy::mem_forget)]
                ::core::mem::forget(self);
            }

            /// Test fixture: a resource from nowhere, for downstream test
            /// code only (R1, F-004; `test-support` feature, dev-dependencies
            /// only).
            #[cfg(feature = "test-support")]
            #[allow(dead_code)] // fixture API surface; unused in private invocations
            pub fn test_fixture() -> Self {
                Self::mint()
            }
        }

        impl<const V: u64> ::core::ops::Drop for $Name<V> {
            /// Conservation tripwire (R1 layer 2, F-008, F-032); stands down
            /// during unwinding. Reports this impl's line, not the leak site
            /// (R5: keep tests per-process).
            fn drop(&mut self) {
                if !::std::thread::panicking() {
                    panic!(
                        "resource leak: {}<{}> dropped without being consumed (R1 conservation)",
                        stringify!($Name),
                        V
                    );
                }
            }
        }
    };
}

/// Defines a sealed **reusable resource** (R2): moved into every process that
/// uses it and returned as part of the output, so the compiler enforces that
/// it is in use by only one process at a time.
///
/// No tripwire `Drop`: a reusable resource legitimately outlives every flow
/// and remains with the caller, and a `Drop` impl would forbid the
/// destructuring its conserving processes rely on (F-032). Sealing, the
/// `pub(crate)` `mint` and the feature-gated `test_fixture` match
/// [`crate::consumable_resource!`]. For a reusable resource carrying an R15
/// budget (`Person<const BUDGET_MS: u64>`), see [`crate::common::Person`] —
/// the budget parameter and its draw process are written by hand.
#[macro_export]
macro_rules! reusable_resource {
    (
        $(#[$meta:meta])*
        $Name:ident,
        must_use = $msg:literal
    ) => {
        $(#[$meta])*
        ///
        /// Sealed reusable resource (R1, R2): private field, no public
        /// constructor, no `Clone`/`Copy`/`Default`. Moved in and returned by
        /// every process that uses it; no tripwire (it stays with the
        /// caller).
        #[must_use = $msg]
        pub struct $Name {
            _seal: (),
        }

        impl $Name {
            /// Mints one resource. `pub(crate)`: only this crate's boundary
            /// modules (R12) may call it.
            // Generated kernel API: whether a given resource's mint/defuse
            // is called in a plain (no test-support) build depends on which
            // boundary/processes exist yet, so dead_code is allowed here.
            #[allow(dead_code)]
            pub(crate) fn mint() -> Self {
                $Name { _seal: () }
            }

            /// Test fixture, for downstream test code only (R1, F-004;
            /// `test-support` feature, dev-dependencies only).
            #[cfg(feature = "test-support")]
            #[allow(dead_code)] // fixture API surface; unused in private invocations
            pub fn test_fixture() -> Self {
                Self::mint()
            }
        }
    };
}

/// Defines an R15 **draw process** over two [`crate::container_resource!`]
/// types: `fn name<const TAKE, const LEFT, const FULL>(Container<FULL>) ->
/// (Out<TAKE>, Container<LEFT>)` with the caller-stated-remainder
/// conservation assert (`TAKE + LEFT == FULL`).
///
/// * Overdrawing is a compile error — no `LEFT` satisfies the assert when
///   `TAKE > FULL` (R15); it fires at monomorphization (F-001), so write
///   regressions as rustdoc `compile_fail` doc-tests (R4).
/// * The generated function mints the drawn amount and the container's next
///   state, so it must live **inside the resource family's defining crate**
///   (continuous processes live inside the privacy boundary, F-031) — it
///   calls the `pub(crate)` `mint`/`defuse`.
/// * The remainder is caller-stated because outputs cannot be computed on
///   stable (F-022); the modeller re-derives the running balance by hand and
///   the compiler checks every step (F-030).
///
/// See the [module docs](crate::resource) for a worked example, the overdraw
/// `compile_fail` regression, and the message `assert = …` should carry (it
/// leads the E0080 output, so phrase it for modellers).
#[macro_export]
macro_rules! draw_process {
    (
        $(#[$meta:meta])*
        $vis:vis fn $fname:ident: $Container:ident => $Out:ident,
        assert = $msg:literal
    ) => {
        $(#[$meta])*
        ///
        /// R15 draw process (generated): draws `TAKE` out of a container
        /// holding `FULL`, leaving `LEFT`; `TAKE + LEFT == FULL` is checked
        /// at compile time (at monomorphization — invisible to `cargo check`
        /// and editor diagnostics, F-001).
        $vis fn $fname<const TAKE: u64, const LEFT: u64, const FULL: u64>(
            container: $Container<FULL>,
        ) -> ($Out<TAKE>, $Container<LEFT>) {
            const {
                assert!(TAKE + LEFT == FULL, $msg)
            };
            // Conserving transform: the old container state is defused (its
            // contents continue as TAKE + LEFT), never silently dropped.
            container.defuse();
            ($Out::mint(), $Container::mint())
        }
    };
}

#[cfg(test)]
mod tests {
    // The macros expand here, in the defining crate's own test module, so
    // mint/defuse are callable and the kernel is exercised without the
    // test-support feature.
    crate::consumable_resource! {
        /// A demo discrete resource.
        Widget,
        must_use = "Widget is a conserved resource: pass it on or hand it to a Consumer"
    }

    crate::container_resource! {
        /// Demo drawn material, in grams.
        Stuff,
        unit = "grams",
        must_use = "Stuff is a conserved resource: pass it on or hand it to a Consumer"
    }

    crate::container_resource! {
        /// A demo container of `Stuff`, in grams remaining.
        StuffBin,
        unit = "grams remaining",
        must_use = "StuffBin is a conserved resource: even an empty bin must be passed on or handed to a Consumer"
    }

    crate::draw_process! {
        /// Draws `TAKE` grams of stuff from a bin holding `FULL`.
        pub fn draw_stuff: StuffBin => Stuff,
        assert = "conservation violated in draw_stuff (R15): TAKE + LEFT must equal FULL"
    }

    crate::reusable_resource! {
        /// A demo reusable resource.
        Jig,
        must_use = "Jig is a reusable resource: pass it on or return it to the caller"
    }

    /// Minting and explicitly defusing a consumable is quiet: the tripwire
    /// and `defuse` are exact complements (F-008).
    #[test]
    fn defused_consumable_is_quiet() {
        let widget = Widget::mint();
        widget.defuse();
    }

    /// The tripwire converts an abandoned consumable into a test failure
    /// (R1 layer 2) and names the type.
    #[test]
    #[should_panic(expected = "resource leak: Widget dropped without being consumed")]
    fn abandoned_consumable_trips_the_tripwire() {
        let widget = Widget::mint();
        let _still_bound_but_never_consumed = widget;
    }

    /// The container tripwire names the type AND the magnitude as a decimal
    /// (F-027) — including the empty state, which is still a resource (R15).
    #[test]
    #[should_panic(expected = "resource leak: StuffBin<0> dropped without being consumed")]
    fn abandoned_empty_container_trips_the_tripwire() {
        let bin: StuffBin<300> = StuffBin::mint();
        let (stuff, empty) = draw_stuff::<300, 0, 300>(bin);
        stuff.defuse(); // the drawn stuff is accounted for...
        let _still_bound_but_never_consumed = empty; // ...the empty bin is not
    }

    /// A draw conserves: magnitudes and units are recoverable (R7), and the
    /// remainder is exactly the difference.
    #[test]
    fn draw_conserves_and_exposes_values() {
        let bin: StuffBin<5000> = StuffBin::mint();
        let (stuff, bin) = draw_stuff::<300, 4700, 5000>(bin);
        assert_eq!(Stuff::<300>::VALUE, 300);
        assert_eq!(Stuff::<300>::UNIT, "grams");
        assert_eq!(StuffBin::<4700>::VALUE, 4700);
        assert_eq!(StuffBin::<4700>::UNIT, "grams remaining");
        stuff.defuse();
        bin.defuse();
    }

    /// A reusable resource has no tripwire: it stays with the caller at the
    /// end of a flow (R2) and may simply go out of scope.
    #[test]
    fn reusable_resource_outlives_the_flow_quietly() {
        let jig = Jig::mint();
        let _stays_with_the_caller = jig;
    }
}
