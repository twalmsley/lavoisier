//! The resource sealing/tripwire kernel (R1, R15, F-032, F-036).
//!
//! The macros here generate, in **the invoking crate**, sealed resource types
//! that obey the R1 sealing rules by construction:
//!
//! * a struct with a private seal field — never a `pub` unit struct or
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
//! ## The four resource shapes
//!
//! | Macro | Shape | Tripwire |
//! |---|---|---|
//! | [`crate::consumable_resource!`] | plain discrete resource (`Bolt`) | yes (unless `no_tripwire`) |
//! | [`crate::container_resource!`] | const-magnitude continuous resource or container (`Gas<G>`, `GasBottle<REMAINING>`) (R15) | yes |
//! | [`crate::reusable_resource!`] | reusable resource (`Organisation`) — moved in and returned by every process (R2) | no — it legitimately outlives every flow and stays with the caller |
//! | [`crate::outcome_token!`] | sealed outcome token for a fallible process (`DrillOutcome`) (R17, F-042) | yes |
//!
//! [`crate::draw_process!`] generates the R15 draw process over two container
//! resources (caller-stated remainder, conservation assert).
//!
//! ## Generic resources (F-036)
//!
//! Each resource macro also accepts a **generic parameter list** — type
//! parameters (each with at most one kind-trait bound, R6/F-018) followed by
//! const parameters — and [`crate::consumable_resource!`] additionally
//! accepts **held contents** (sealed payload fields owned as real objects,
//! R12) and a trailing `no_tripwire` marker for items that are *kept* by a
//! container which accounts for them. The full grammar, the parsing pattern
//! and its limits are documented on [`crate::consumable_resource!`]; the
//! generic forms carry every guarantee the non-generic forms do.
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

/// Defines a sealed, tripwired, **consumable discrete resource** (R1) —
/// non-generic or generic (F-036), optionally holding sealed payload fields
/// (held contents, R12), optionally untripwired (`no_tripwire`).
///
/// See the [module docs](crate::resource) for everything the expansion
/// contains and the non-generic worked example. The generated `mint`/`defuse`
/// are `pub(crate)`: only the defining crate's boundary (R12) creates the
/// resource, and only its conserving transforms and consumers defuse the
/// tripwire. The generated `test_fixture()` is available to downstream tests
/// through the `test-support` feature (F-004).
///
/// ## Grammar
///
/// ```text
/// consumable_resource! {
///     /// docs…
///     Name [< params >] [{ held contents }] ,
///     must_use = "…"            (a literal: attribute values cannot be
///                                concatenated on stable)
///     [, no_tripwire]
/// }
///
/// params        :=  ty_params  |  const_params  |  ty_params , const_params
/// ty_params     :=  Ident [: KindBound] , …     (one `path` bound per
///                                                parameter; compound bounds
///                                                go behind a named kind or
///                                                requirement trait, R6/R10)
/// const_params  :=  const IDENT : Type , …
/// held contents :=  field : Type , …            (sealed payload fields the
///                                                resource owns as real
///                                                objects, R12)
/// ```
///
/// Limits of the accepted grammar, all deliberate:
///
/// * **The parameter list is parsed by a tt-muncher** (one rule per
///   parameter shape, accumulating into bracketed lists), not by shaped
///   repetitions: `macro_rules!` cannot alternate parameter shapes inside
///   one repetition — at the point where either another type parameter
///   (an `ident` fragment) or the `const` keyword may follow, the matcher
///   reports *"local ambiguity: multiple parsing options"* rather than
///   disambiguating by token. **Type parameters come before const
///   parameters**, and each type parameter takes at most one bound, written
///   as a plain path: `+` may not follow a `path` fragment (the same
///   follow-set family that shaped [`crate::requirement!`]'s parenthesised
///   bounds), and compound bounds belong behind a named kind/requirement
///   trait anyway (R6, R10).
/// * **No lifetimes** (resources are never borrowed, R2), no parameter
///   defaults (the F-017 blanket-impl trap), no `where` clauses, and no
///   trailing comma in the parameter list.
/// * **Held contents are consumption-only**: they go in through `mint` (a
///   conserving combinator — it only wraps values passed in by value, R1) and
///   leave only when the whole resource is defused at a consumer. `defuse`'s
///   sanctioned `mem::forget` skips the payload's destructors, so held
///   contents must themselves be **untripwired** (`no_tripwire` items, or
///   types with no `Drop`) — a tripwired payload would be silently defused.
///   Contents that must come back *out* (a contents-keeping consumer's
///   disposal path, F-039) are a `Consumer`, not a resource: keep those
///   hand-written.
/// * **`no_tripwire`** omits the tripwire `Drop` and `defuse` (nothing to
///   defuse): for items *kept* by a container that accounts for them (a box,
///   an assembly), where a tripwired item inside an abandoned container would
///   panic from the container's own drop and mask the real leak site.
///   Whole-value discard is still caught by `#[must_use]`.
/// * The generic forms' tripwire reports the **full type name** (via
///   `core::any::type_name`), so const magnitudes print as decimals in the
///   panic message (F-027); the non-generic form reports the bare name, as
///   before.
///
/// ## Generic worked example (runs as a doc-test)
///
/// A parameterized catalogue item kept (untripwired) by a payload-holding
/// assembly — the two F-036 target shapes end to end:
///
/// ```
/// /// Kind trait for the size slot (R6, F-018).
/// pub trait Size {}
/// /// Size value: metric thread M8.
/// pub struct M8;
/// impl Size for M8 {}
///
/// model_core::consumable_resource! {
///     /// A catalogue bolt of size `S`. Kept by the assembly below, which
///     /// accounts for it — hence `no_tripwire`.
///     Bolt<S: Size>,
///     must_use = "Bolt is a conserved resource: pass it on or hand it to a Consumer",
///     no_tripwire
/// }
///
/// model_core::consumable_resource! {
///     /// Two plates of `MASS_G` grams combined, fastened with two real
///     /// bolts of type `B` held as sealed payload (R12).
///     Assembly<B, const MASS_G: u64> {
///         bolts: (B, B),
///     },
///     must_use = "Assembly is a conserved resource: pass it on or hand it to a Consumer"
/// }
///
/// fn main() {
///     // mint takes ownership of the held contents, in declaration order.
///     let assembly: Assembly<Bolt<M8>, 880> =
///         Assembly::mint((Bolt::mint(), Bolt::mint()));
///     // In a real model a boundary Consumer plays this role.
///     assembly.defuse();
/// }
/// ```
#[macro_export]
macro_rules! consumable_resource {
    // ---- 1. Plain non-generic form (the original grammar, unchanged) ----
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

    // ---- 1b. Non-generic with held contents and/or `no_tripwire` ----
    // (A plain invocation matches arm 1 first; this arm is reached only when
    // payload fields or the flag are present.)
    (
        $(#[$meta:meta])*
        $Name:ident
        $({ $( $field:ident : $fty:ty ),* $(,)? })?,
        must_use = $msg:literal
        $(, $flag:ident)?
    ) => {
        $crate::__mc_consumable_resource! {
            meta = [ $(#[$meta])* ],
            name = $Name,
            tys = [ ],
            consts = [ ],
            payload = [ $($( $field : $fty , )*)? ],
            must_use = $msg,
            flag = [ $($flag)? ]
        }
    };

    // ---- 2. Generic forms (F-036) ----
    // Everything after `Name<` goes to the shared parameter tt-muncher
    // (`__mc_parse_generics!`), which accumulates the type and const
    // parameters up to the closing `>` and then hands the remainder (held
    // contents, must_use, flag) to `__mc_consumable_finish!`.
    (
        $(#[$meta:meta])*
        $Name:ident < $($rest:tt)+
    ) => {
        $crate::__mc_parse_generics! {
            cb = __mc_consumable_finish,
            args = [ meta = [ $(#[$meta])* ], name = $Name ],
            tys = [ ],
            consts = [ ],
            rest = [ $($rest)+ ]
        }
    };
}

/// Internal parameter-list tt-muncher shared by the three resource macros'
/// generic forms (F-036). One rule per parameter shape — `macro_rules!`
/// cannot alternate shapes inside one repetition without a local-ambiguity
/// error — accumulating the declarations (with trailing commas) into `tys`
/// and `consts`, then invoking the named finish macro on the closing `>`.
/// Not part of the public grammar.
#[doc(hidden)]
#[macro_export]
macro_rules! __mc_parse_generics {
    // const parameter, more parameters follow
    (
        cb = $cb:ident,
        args = [ $($args:tt)* ],
        tys = [ $($tys:tt)* ],
        consts = [ $($consts:tt)* ],
        rest = [ const $C:ident : $CT:ty , $($r:tt)* ]
    ) => {
        $crate::__mc_parse_generics! {
            cb = $cb,
            args = [ $($args)* ],
            tys = [ $($tys)* ],
            consts = [ $($consts)* $C : $CT , ],
            rest = [ $($r)* ]
        }
    };
    // const parameter, closing the list
    (
        cb = $cb:ident,
        args = [ $($args:tt)* ],
        tys = [ $($tys:tt)* ],
        consts = [ $($consts:tt)* ],
        rest = [ const $C:ident : $CT:ty > $($r:tt)* ]
    ) => {
        $crate::$cb! {
            args = [ $($args)* ],
            tys = [ $($tys)* ],
            consts = [ $($consts)* $C : $CT , ],
            rest = [ $($r)* ]
        }
    };
    // bounded type parameter, more parameters follow
    (
        cb = $cb:ident,
        args = [ $($args:tt)* ],
        tys = [ $($tys:tt)* ],
        consts = [ $($consts:tt)* ],
        rest = [ $T:ident : $TB:path , $($r:tt)* ]
    ) => {
        $crate::__mc_parse_generics! {
            cb = $cb,
            args = [ $($args)* ],
            tys = [ $($tys)* $T : $TB , ],
            consts = [ $($consts)* ],
            rest = [ $($r)* ]
        }
    };
    // bounded type parameter, closing the list
    (
        cb = $cb:ident,
        args = [ $($args:tt)* ],
        tys = [ $($tys:tt)* ],
        consts = [ $($consts:tt)* ],
        rest = [ $T:ident : $TB:path > $($r:tt)* ]
    ) => {
        $crate::$cb! {
            args = [ $($args)* ],
            tys = [ $($tys)* $T : $TB , ],
            consts = [ $($consts)* ],
            rest = [ $($r)* ]
        }
    };
    // unbounded type parameter, more parameters follow
    (
        cb = $cb:ident,
        args = [ $($args:tt)* ],
        tys = [ $($tys:tt)* ],
        consts = [ $($consts:tt)* ],
        rest = [ $T:ident , $($r:tt)* ]
    ) => {
        $crate::__mc_parse_generics! {
            cb = $cb,
            args = [ $($args)* ],
            tys = [ $($tys)* $T , ],
            consts = [ $($consts)* ],
            rest = [ $($r)* ]
        }
    };
    // unbounded type parameter, closing the list
    (
        cb = $cb:ident,
        args = [ $($args:tt)* ],
        tys = [ $($tys:tt)* ],
        consts = [ $($consts:tt)* ],
        rest = [ $T:ident > $($r:tt)* ]
    ) => {
        $crate::$cb! {
            args = [ $($args)* ],
            tys = [ $($tys)* $T , ],
            consts = [ $($consts)* ],
            rest = [ $($r)* ]
        }
    };
}

/// Internal finish step of [`crate::consumable_resource!`]'s generic forms:
/// parses the remainder after the parameter list (held contents, `must_use`,
/// optional `no_tripwire`) and dispatches to the emitter. Not part of the
/// public grammar.
#[doc(hidden)]
#[macro_export]
macro_rules! __mc_consumable_finish {
    // Tripwired (the default).
    (
        args = [ meta = [ $(#[$meta:meta])* ], name = $Name:ident ],
        tys = [ $($tys:tt)* ],
        consts = [ $($consts:tt)* ],
        rest = [ $({ $( $field:ident : $fty:ty ),* $(,)? })? , must_use = $msg:literal ]
    ) => {
        $crate::__mc_consumable_resource! {
            meta = [ $(#[$meta])* ],
            name = $Name,
            tys = [ $($tys)* ],
            consts = [ $($consts)* ],
            payload = [ $($( $field : $fty , )*)? ],
            must_use = $msg,
            flag = [ ]
        }
    };
    // Untripwired (`no_tripwire`).
    (
        args = [ meta = [ $(#[$meta:meta])* ], name = $Name:ident ],
        tys = [ $($tys:tt)* ],
        consts = [ $($consts:tt)* ],
        rest = [ $({ $( $field:ident : $fty:ty ),* $(,)? })? , must_use = $msg:literal , no_tripwire ]
    ) => {
        $crate::__mc_consumable_resource! {
            meta = [ $(#[$meta])* ],
            name = $Name,
            tys = [ $($tys)* ],
            consts = [ $($consts)* ],
            payload = [ $($( $field : $fty , )*)? ],
            must_use = $msg,
            flag = [ no_tripwire ]
        }
    };
}

/// Internal expansion target of [`crate::consumable_resource!`]'s generic and
/// extended forms (F-036). Not part of the public grammar — invoke the
/// public macro instead.
#[doc(hidden)]
#[macro_export]
macro_rules! __mc_consumable_resource {
    // Tripwired (the default): flag list is empty.
    (
        meta = [ $(#[$meta:meta])* ],
        name = $Name:ident,
        tys = [ $( $T:ident $(: $TB:path)? , )* ],
        consts = [ $( $C:ident : $CT:ty , )* ],
        payload = [ $( $field:ident : $fty:ty , )* ],
        must_use = $msg:literal,
        flag = [ ]
    ) => {
        $(#[$meta])*
        ///
        /// Sealed consumable resource (R1): private fields, no public
        /// constructor, no `Clone`/`Copy`/`Default`; carries a tripwire
        /// `Drop` (F-008) defused only by explicit consumption inside the
        /// defining crate's boundary.
        #[must_use = $msg]
        pub struct $Name<$( $T $(: $TB)? ,)* $( const $C: $CT ,)*> {
            $(
                // Held contents (R12 "suppliers hold real objects"): sealed
                // payload, reachable only inside the privacy boundary. Never
                // read back out — it leaves only when the whole resource is
                // defused at a consumer — so dead_code is allowed.
                #[allow(dead_code)]
                $field: $fty,
            )*
            _seal: ::core::marker::PhantomData<($( $T, )*)>,
        }

        impl<$( $T $(: $TB)? ,)* $( const $C: $CT ,)*> $Name<$( $T, )* $( $C, )*> {
            /// Mints one resource, taking ownership of its held contents
            /// (payload fields, in declaration order). `pub(crate)`: only
            /// this crate's boundary modules (R12) and conserving processes
            /// may call it — a conserving combinator only ever wraps values
            /// passed in by value (R1).
            // Generated kernel API: whether a given resource's mint/defuse
            // is called in a plain (no test-support) build depends on which
            // boundary/processes exist yet, so dead_code is allowed here.
            #[allow(dead_code)]
            pub(crate) fn mint($( $field: $fty ),*) -> Self {
                $Name {
                    $( $field, )*
                    _seal: ::core::marker::PhantomData,
                }
            }

            /// Defuses the tripwire and lets the resource go. The single
            /// allowed forget site for this resource (R1, F-008); see
            /// [`crate::consumable_resource!`]. The forget also skips the
            /// held contents' destructors, which is why held contents must
            /// be untripwired (F-036).
            #[allow(dead_code)]
            pub(crate) fn defuse(self) {
                // The one sanctioned mem::forget per resource (F-008): it is
                // exactly the complement of the tripwire Drop below.
                #[allow(clippy::mem_forget)]
                ::core::mem::forget(self);
            }

            /// Test fixture: a resource from nowhere (its held contents, if
            /// any, still arrive by value), for downstream test code only
            /// (R1, F-004; `test-support` feature, dev-dependencies only).
            #[cfg(feature = "test-support")]
            #[allow(dead_code)] // fixture API surface; unused in private invocations
            pub fn test_fixture($( $field: $fty ),*) -> Self {
                Self::mint($( $field ),*)
            }
        }

        impl<$( $T $(: $TB)? ,)* $( const $C: $CT ,)*> ::core::ops::Drop
            for $Name<$( $T, )* $( $C, )*>
        {
            /// Conservation tripwire (R1 layer 2, F-008, F-032); stands down
            /// during unwinding. Reports the full type name, so const
            /// magnitudes print as decimals (F-027) — but this impl's line,
            /// not the leak site (R5: keep tests per-process).
            fn drop(&mut self) {
                if !::std::thread::panicking() {
                    panic!(
                        "resource leak: {} dropped without being consumed (R1 conservation)",
                        ::core::any::type_name::<Self>()
                    );
                }
            }
        }
    };

    // Untripwired (`no_tripwire`): kept by a container that accounts for it.
    // No `Drop`, and no `defuse` — there is no tripwire to defuse, and a
    // sanctioned `mem::forget` on a `Drop`-less type would itself be a leak
    // path (clippy::forget_non_drop).
    (
        meta = [ $(#[$meta:meta])* ],
        name = $Name:ident,
        tys = [ $( $T:ident $(: $TB:path)? , )* ],
        consts = [ $( $C:ident : $CT:ty , )* ],
        payload = [ $( $field:ident : $fty:ty , )* ],
        must_use = $msg:literal,
        flag = [ no_tripwire ]
    ) => {
        $(#[$meta])*
        ///
        /// Sealed consumable resource (R1): private fields, no public
        /// constructor, no `Clone`/`Copy`/`Default`. Deliberately
        /// **untripwired** (`no_tripwire`): it is kept by a container that
        /// accounts for it, and a tripwired item inside an abandoned
        /// container would panic from the container's own drop, masking the
        /// real leak site. Whole-value discard is still caught by
        /// `#[must_use]`.
        #[must_use = $msg]
        pub struct $Name<$( $T $(: $TB)? ,)* $( const $C: $CT ,)*> {
            $(
                // Held contents (R12): sealed payload, reachable only inside
                // the privacy boundary.
                #[allow(dead_code)]
                $field: $fty,
            )*
            _seal: ::core::marker::PhantomData<($( $T, )*)>,
        }

        impl<$( $T $(: $TB)? ,)* $( const $C: $CT ,)*> $Name<$( $T, )* $( $C, )*> {
            /// Mints one resource, taking ownership of its held contents
            /// (payload fields, in declaration order). `pub(crate)`: only
            /// this crate's boundary modules (R12) and conserving processes
            /// may call it (R1).
            // Generated kernel API; see the tripwired arm for the rationale.
            #[allow(dead_code)]
            pub(crate) fn mint($( $field: $fty ),*) -> Self {
                $Name {
                    $( $field, )*
                    _seal: ::core::marker::PhantomData,
                }
            }

            /// Test fixture: a resource from nowhere (its held contents, if
            /// any, still arrive by value), for downstream test code only
            /// (R1, F-004; `test-support` feature, dev-dependencies only).
            #[cfg(feature = "test-support")]
            #[allow(dead_code)] // fixture API surface; unused in private invocations
            pub fn test_fixture($( $field: $fty ),*) -> Self {
                Self::mint($( $field ),*)
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
///
/// ## Grammar
///
/// ```text
/// container_resource! {
///     /// docs…
///     Name [< params >] ,
///     unit = "…",
///     must_use = "…"
/// }
/// ```
///
/// `params` follows the F-036 grammar documented on
/// [`crate::consumable_resource!`] (type parameters with at most one path
/// bound each, then const parameters; no lifetimes, defaults, `where`
/// clauses or trailing commas). The macro **appends the magnitude parameter
/// `const V: u64` itself**, always last — so `Tank<F: Fluid>` generates
/// `Tank<F, const V: u64>`, used as `Tank<Water, 5000>` — and no declared
/// parameter may be named `V`. A generic container takes no held contents
/// (its contents are its magnitude) and no `no_tripwire` (an abandoned
/// container, even empty, is exactly what the tripwire exists to catch,
/// F-032). The generic forms' tripwire reports the full type name via
/// `core::any::type_name`, magnitude included.
///
/// ## Worked example (runs as a doc-test)
///
/// A declared const parameter rides as a second quantity beside the appended
/// magnitude `V` — the multi-quantity-state shape CS-1's
/// `BoilingKettle<WATER_G, V>` uses (`model/cs1-pot-of-tea/src/resources.rs`):
///
/// ```
/// model_core::container_resource! {
///     /// A kettle at the boil: `WATER_G` grams of water carrying `V`
///     /// joules of embodied energy (the magnitude slot).
///     BoilingKettle<const WATER_G: u64>,
///     unit = "joules (embodied)",
///     must_use = "BoilingKettle is a conserved resource: pour it or hand it to a Consumer"
/// }
///
/// fn main() {
///     // mint/defuse are pub(crate): this code only exists inside the
///     // defining crate's boundary (here, the doc-test crate).
///     let kettle: BoilingKettle<1500, 500_000> = BoilingKettle::mint();
///     assert_eq!(BoilingKettle::<1500, 500_000>::VALUE, 500_000);
///     assert_eq!(BoilingKettle::<1500, 500_000>::UNIT, "joules (embodied)");
///     // A real model hands this to a process or Consumer; the boundary
///     // defuses it here to keep the example self-contained.
///     kettle.defuse();
/// }
/// ```
#[macro_export]
macro_rules! container_resource {
    // ---- 1. Plain non-generic form (the original grammar, unchanged) ----
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

    // ---- 2. Generic forms (F-036) ----
    // Everything after `Name<` goes to the shared parameter tt-muncher; the
    // remainder (`, unit = …, must_use = …`) is parsed by
    // `__mc_container_finish!`.
    (
        $(#[$meta:meta])*
        $Name:ident < $($rest:tt)+
    ) => {
        $crate::__mc_parse_generics! {
            cb = __mc_container_finish,
            args = [ meta = [ $(#[$meta])* ], name = $Name ],
            tys = [ ],
            consts = [ ],
            rest = [ $($rest)+ ]
        }
    };
}

/// Internal finish step of [`crate::container_resource!`]'s generic forms.
/// Not part of the public grammar.
#[doc(hidden)]
#[macro_export]
macro_rules! __mc_container_finish {
    (
        args = [ meta = [ $(#[$meta:meta])* ], name = $Name:ident ],
        tys = [ $($tys:tt)* ],
        consts = [ $($consts:tt)* ],
        rest = [ , unit = $unit:literal , must_use = $msg:literal ]
    ) => {
        $crate::__mc_container_resource! {
            meta = [ $(#[$meta])* ],
            name = $Name,
            tys = [ $($tys)* ],
            consts = [ $($consts)* ],
            unit = $unit,
            must_use = $msg
        }
    };
}

/// Internal expansion target of [`crate::container_resource!`]'s generic
/// forms (F-036). Not part of the public grammar — invoke the public macro
/// instead.
#[doc(hidden)]
#[macro_export]
macro_rules! __mc_container_resource {
    (
        meta = [ $(#[$meta:meta])* ],
        name = $Name:ident,
        tys = [ $( $T:ident $(: $TB:path)? , )* ],
        consts = [ $( $C:ident : $CT:ty , )* ],
        unit = $unit:literal,
        must_use = $msg:literal
    ) => {
        $(#[$meta])*
        ///
        /// Sealed const-magnitude container resource (R1, R15): private
        /// fields, no public constructor, no `Clone`/`Copy`/`Default`;
        /// carries a tripwire `Drop` (F-008) defused only by explicit
        /// consumption inside the defining crate's boundary. The magnitude
        /// parameter `V` (appended last by the macro) is in the type's fixed
        /// base unit (`UNIT`).
        #[must_use = $msg]
        pub struct $Name<$( $T $(: $TB)? ,)* $( const $C: $CT ,)* const V: u64> {
            _seal: ::core::marker::PhantomData<($( $T, )*)>,
        }

        impl<$( $T $(: $TB)? ,)* $( const $C: $CT ,)* const V: u64>
            $Name<$( $T, )* $( $C, )* V>
        {
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
                $Name {
                    _seal: ::core::marker::PhantomData,
                }
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

        impl<$( $T $(: $TB)? ,)* $( const $C: $CT ,)* const V: u64> ::core::ops::Drop
            for $Name<$( $T, )* $( $C, )* V>
        {
            /// Conservation tripwire (R1 layer 2, F-008, F-032); stands down
            /// during unwinding. Reports the full type name, magnitude
            /// included as a decimal (F-027) — but this impl's line, not the
            /// leak site (R5: keep tests per-process).
            fn drop(&mut self) {
                if !::std::thread::panicking() {
                    panic!(
                        "resource leak: {} dropped without being consumed (R1 conservation)",
                        ::core::any::type_name::<Self>()
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
/// budget drawn down by a hand-written process (`Person<const BUDGET_MS:
/// u64>`), see [`crate::common::Person`] — its draw process destructures the
/// struct, which the macro-generated seal supports only inside the defining
/// module, so it stays hand-written.
///
/// ## Grammar
///
/// ```text
/// reusable_resource! {
///     /// docs…
///     Name [< params >] ,
///     must_use = "…"
/// }
/// ```
///
/// `params` follows the F-036 grammar documented on
/// [`crate::consumable_resource!`] (type parameters with at most one path
/// bound each, then const parameters; no lifetimes, defaults, `where`
/// clauses or trailing commas). A reusable resource takes no held contents
/// and no flags.
///
/// ## Worked example (runs as a doc-test)
///
/// A reusable tool is moved into a process and returned as part of its
/// output (R2), then legitimately stays with the caller — no tripwire fires:
///
/// ```
/// model_core::reusable_resource! {
///     /// A spanner: moved in and returned by every process that uses it.
///     Spanner,
///     must_use = "Spanner is a reusable resource: pass it on or return it to the caller"
/// }
///
/// /// A process takes the tool by value and gives it back (R1, R2).
/// fn tighten(spanner: Spanner) -> Spanner {
///     spanner
/// }
///
/// fn main() {
///     // mint is pub(crate): only the defining crate's boundary (here, the
///     // doc-test crate) creates the resource.
///     let spanner = Spanner::mint();
///     let spanner = tighten(spanner);
///     // Reusable resources outlive the flow and stay with the caller.
///     let _stays_with_the_caller = spanner;
/// }
/// ```
#[macro_export]
macro_rules! reusable_resource {
    // ---- 1. Plain non-generic form (the original grammar, unchanged) ----
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

    // ---- 2. Generic forms (F-036) ----
    // Everything after `Name<` goes to the shared parameter tt-muncher; the
    // remainder (`, must_use = …`) is parsed by `__mc_reusable_finish!`.
    (
        $(#[$meta:meta])*
        $Name:ident < $($rest:tt)+
    ) => {
        $crate::__mc_parse_generics! {
            cb = __mc_reusable_finish,
            args = [ meta = [ $(#[$meta])* ], name = $Name ],
            tys = [ ],
            consts = [ ],
            rest = [ $($rest)+ ]
        }
    };
}

/// Internal finish step of [`crate::reusable_resource!`]'s generic forms.
/// Not part of the public grammar.
#[doc(hidden)]
#[macro_export]
macro_rules! __mc_reusable_finish {
    (
        args = [ meta = [ $(#[$meta:meta])* ], name = $Name:ident ],
        tys = [ $($tys:tt)* ],
        consts = [ $($consts:tt)* ],
        rest = [ , must_use = $msg:literal ]
    ) => {
        $crate::__mc_reusable_resource! {
            meta = [ $(#[$meta])* ],
            name = $Name,
            tys = [ $($tys)* ],
            consts = [ $($consts)* ],
            must_use = $msg
        }
    };
}

/// Internal expansion target of [`crate::reusable_resource!`]'s generic forms
/// (F-036). Not part of the public grammar — invoke the public macro instead.
#[doc(hidden)]
#[macro_export]
macro_rules! __mc_reusable_resource {
    (
        meta = [ $(#[$meta:meta])* ],
        name = $Name:ident,
        tys = [ $( $T:ident $(: $TB:path)? , )* ],
        consts = [ $( $C:ident : $CT:ty , )* ],
        must_use = $msg:literal
    ) => {
        $(#[$meta])*
        ///
        /// Sealed reusable resource (R1, R2): private fields, no public
        /// constructor, no `Clone`/`Copy`/`Default`. Moved in and returned by
        /// every process that uses it; no tripwire (it stays with the
        /// caller).
        #[must_use = $msg]
        pub struct $Name<$( $T $(: $TB)? ,)* $( const $C: $CT ,)*> {
            _seal: ::core::marker::PhantomData<($( $T, )*)>,
        }

        impl<$( $T $(: $TB)? ,)* $( const $C: $CT ,)*> $Name<$( $T, )* $( $C, )*> {
            /// Mints one resource. `pub(crate)`: only this crate's boundary
            /// modules (R12) may call it.
            // Generated kernel API; see the consumable macro for the
            // dead_code rationale.
            #[allow(dead_code)]
            pub(crate) fn mint() -> Self {
                $Name {
                    _seal: ::core::marker::PhantomData,
                }
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
/// * The container and output are **plain non-generic idents**: a *generic*
///   container family (the F-036 forms) hand-writes its draw inside the
///   privacy boundary, where it already lives beside the family's
///   `pub(crate)` `mint`/`defuse` (F-031). No generic container with a draw
///   exists yet; extending this generator is deliberate future work, not a
///   parsing limitation.
///
/// See the [module docs](crate::resource) for the overdraw `compile_fail`
/// regression and the message `assert = …` should carry (it leads the E0080
/// output, so phrase it for modellers).
///
/// ## Worked example (runs as a doc-test)
///
/// ```
/// model_core::container_resource! {
///     /// Drawn fuel, in grams.
///     Fuel,
///     unit = "grams",
///     must_use = "Fuel is a conserved resource: pass it on or hand it to a Consumer"
/// }
///
/// model_core::container_resource! {
///     /// A fuel can holding `V` grams; `FuelCan<0>` is the empty state.
///     FuelCan,
///     unit = "grams remaining",
///     must_use = "FuelCan is a conserved resource: even an empty can must be accounted for"
/// }
///
/// model_core::draw_process! {
///     /// Draws `TAKE` grams from a can holding `FULL`, leaving `LEFT`.
///     pub fn draw_fuel: FuelCan => Fuel,
///     assert = "conservation violated in draw_fuel (R15): TAKE + LEFT must equal FULL - is the draw larger than the can's remaining contents?"
/// }
///
/// fn main() {
///     let can: FuelCan<800> = FuelCan::mint(); // boundary fill (pub(crate) mint)
///     let (fuel, can) = draw_fuel::<300, 500, 800>(can);
///     assert_eq!(Fuel::<300>::VALUE + FuelCan::<500>::VALUE, 800);
///     // A real model hands these on; the boundary defuses them here.
///     fuel.defuse();
///     can.defuse();
/// }
/// ```
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

/// Defines a sealed, tripwired **outcome token** (R17, F-042): the boundary
/// object through which variability enters a model with a fallible process,
/// without making any process nondeterministic.
///
/// One token type per fallible-process kind, runtime-valued — **never two
/// token types selected by the flow**: two types make the outcome part of the
/// flow's static text, with no `Result` and no code ever forced to handle the
/// arm it did not pick (F-042). The token wraps a **private enum in a sealed
/// struct** (R1's "never a `pub enum` resource"); its value is injected only
/// at the system boundary (the generated constructors, placeholders until
/// calibrated) or by test-support fixtures. A flow holding a token cannot
/// read it: the only way to learn the outcome is to run the consuming process
/// and handle the `Result`, so both arms must be written.
///
/// ## What the expansion contains
///
/// * a `pub(crate)` outcome-kind enum (`Success`/`Failure`), `Copy` — a
///   value-level record, not a resource; invisible outside the defining
///   crate, so flows cannot read tokens;
/// * the sealed `pub` token struct: private fields, no public constructor,
///   no `Clone`/`Copy`/`Default`, `#[must_use]`, **tripwired** (F-008): a
///   provisioned trial that is neither run nor returned to the environment
///   fails the test that leaked it;
/// * a `pub(crate)` `consume_kind(self)` — the sealed way for **the owning
///   process** (exactly one per token type, R1) to consume the token and
///   realise the outcome;
/// * `pub` boundary constructors (one success, one failure), each tagged
///   `/// Placeholder:` — refine to a calibrated outcome source (validation,
///   open question 1);
/// * a `pub` boundary **exit** for an untried token (R12): the accounted
///   path for a provisioned retry trial a flow never needed (F-050);
/// * `test_fixture_success()`/`test_fixture_failure()` constructors gated
///   behind the invoking crate's `test-support` feature (F-004).
///
/// ## Grammar
///
/// ```text
/// outcome_token! {
///     /// docs…
///     Name ( KindName ) ,          (KindName: the pub(crate) outcome enum)
///     success = success_ctor,      (boundary constructor names)
///     failure = failure_ctor,
///     exit = exit_fn,              (boundary exit for an untried token)
///     must_use = "…"
/// }
/// ```
///
/// The token is non-generic by design: a fallible-process *kind* has one
/// token type, and per-call variation travels in the token's value, not its
/// type (F-042).
///
/// ## Worked example (runs as a doc-test)
///
/// A fallible welding step: the process consumes the token through
/// `consume_kind` (callable only inside the defining crate) and returns a
/// `Result`; an untried token leaves through the boundary exit.
///
/// ```
/// model_core::outcome_token! {
///     /// One trial of the environment: whether a single welding attempt
///     /// succeeds or the torch flames out.
///     WeldOutcome(WeldOutcomeKind),
///     success = weld_goes_well,
///     failure = weld_flames_out,
///     exit = return_weld_outcome,
///     must_use = "WeldOutcome is a boundary token: run it through exactly one fallible process or return it to the environment"
/// }
///
/// /// The one process that consumes the token (R17): deterministic in its
/// /// inputs — the variability is the token's value.
/// fn weld(outcome: WeldOutcome) -> Result<(), ()> {
///     match outcome.consume_kind() {
///         WeldOutcomeKind::Success => Ok(()),
///         WeldOutcomeKind::Failure => Err(()),
///     }
/// }
///
/// fn main() {
///     assert!(weld(weld_goes_well()).is_ok());
///     assert!(weld(weld_flames_out()).is_err());
///     // An untried token has an accounted boundary exit (F-050).
///     return_weld_outcome(weld_goes_well());
/// }
/// ```
#[macro_export]
macro_rules! outcome_token {
    (
        $(#[$meta:meta])*
        $Name:ident ( $Kind:ident ),
        success = $success_fn:ident,
        failure = $failure_fn:ident,
        exit = $exit_fn:ident,
        must_use = $msg:literal
    ) => {
        /// The private outcome value inside the token: a value-level record
        /// (like R16's `Event`), not a resource — `Copy` is fine here because
        /// the *token* is the conserved thing, and the token is sealed and
        /// tripwired. `pub(crate)`: only the defining crate's owning process
        /// can name it, so flows cannot read outcomes.
        #[derive(Clone, Copy)]
        pub(crate) enum $Kind {
            /// The trial will succeed.
            Success,
            /// The trial will fail.
            Failure,
        }

        $(#[$meta])*
        ///
        /// Sealed outcome token (R17, F-042): a private enum wrapped in a
        /// sealed struct, tripwired (F-008). Its value is injected only at
        /// the system boundary or by test-support fixtures; a flow cannot
        /// read it — the only way to learn the outcome is to run the
        /// consuming process and handle both arms of its `Result`.
        #[must_use = $msg]
        pub struct $Name {
            kind: $Kind,
            _seal: (),
        }

        impl $Name {
            /// Mints one token. Private: only the generated boundary
            /// constructors and fixtures may create an outcome (R1, R12).
            #[allow(dead_code)]
            fn mint(kind: $Kind) -> Self {
                $Name { kind, _seal: () }
            }

            /// Defuses the tripwire and lets the token go — the single
            /// allowed forget site for this resource (R1, F-008), called only
            /// by `consume_kind` and the boundary exit.
            #[allow(dead_code)]
            fn defuse(self) {
                // The one sanctioned mem::forget for this type (F-008).
                #[allow(clippy::mem_forget)]
                ::core::mem::forget(self);
            }

            /// Consumes the token and reveals its kind — callable only
            /// inside the defining crate (`pub(crate)`), i.e. only by the
            /// one process that realises the outcome (R17). (A type with
            /// `Drop` cannot be destructured, F-032, hence read-then-defuse.)
            #[allow(dead_code)]
            pub(crate) fn consume_kind(self) -> $Kind {
                let kind = self.kind;
                self.defuse();
                kind
            }

            /// Test fixture: a success token from nowhere, for downstream
            /// test code only (R1, F-004; `test-support` feature,
            /// dev-dependencies only).
            #[cfg(feature = "test-support")]
            #[allow(dead_code)] // fixture API surface; unused in private invocations
            pub fn test_fixture_success() -> Self {
                Self::mint($Kind::Success)
            }

            /// Test fixture: a failure token from nowhere (R1, F-004) — both
            /// outcomes of one process are testable by injecting either
            /// token.
            #[cfg(feature = "test-support")]
            #[allow(dead_code)] // fixture API surface; unused in private invocations
            pub fn test_fixture_failure() -> Self {
                Self::mint($Kind::Failure)
            }
        }

        impl ::core::ops::Drop for $Name {
            /// Conservation tripwire (R1 layer 2, F-008): a provisioned
            /// trial that is dropped without being run (or returned to the
            /// environment) is a leak. Stands down while the thread is
            /// already panicking.
            fn drop(&mut self) {
                if !::std::thread::panicking() {
                    panic!(
                        "resource leak: {} dropped without being consumed (R1 conservation)",
                        stringify!($Name)
                    );
                }
            }
        }

        /// A trial that will succeed enters the model (R17): the
        /// environment's variability, injected at the boundary (R12) so
        /// processes stay deterministic. A flow cannot read the token; it
        /// must run the consuming process and handle both arms of the
        /// `Result`.
        ///
        /// Placeholder: the environment — refine to a calibrated outcome
        /// source (validation, open question 1) or use the test fixtures.
        #[allow(dead_code)]
        pub fn $success_fn() -> $Name {
            $Name::mint($Kind::Success)
        }

        /// A trial that will fail enters the model (R17).
        ///
        /// Placeholder: the environment — refine to a calibrated outcome
        /// source (validation, open question 1) or use the test fixtures.
        #[allow(dead_code)]
        pub fn $failure_fn() -> $Name {
            $Name::mint($Kind::Failure)
        }

        /// An untried outcome token returns to the environment (R12 exit):
        /// the accounted path for a provisioned retry trial a flow never
        /// needed (F-050).
        #[allow(dead_code)]
        pub fn $exit_fn(token: $Name) {
            token.defuse();
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

    // ---- Generic forms (F-036) ----

    /// Kind trait bounding the demo generic resources' parameter slot
    /// (R6/F-018 style).
    pub trait Kind {}
    /// A kind value for the demo generic resources.
    pub struct Red;
    impl Kind for Red {}

    crate::consumable_resource! {
        /// A demo untripwired part (`no_tripwire`, non-generic extended
        /// form): kept by the carton below, which accounts for it.
        Part,
        must_use = "Part is a conserved resource: pass it on or hand it to a Consumer",
        no_tripwire
    }

    crate::consumable_resource! {
        /// A demo generic carton of kind `K` keeping one real [`Part`] as
        /// held contents (R12, F-036).
        Carton<K: Kind> { part: Part },
        must_use = "Carton is a conserved resource: pass it on or hand it to a Consumer"
    }

    crate::consumable_resource! {
        /// A demo batch of `N` units (const-parameter generic shape).
        Batch<const N: u64>,
        must_use = "Batch is a conserved resource: pass it on or hand it to a Consumer"
    }

    crate::container_resource! {
        /// A demo generic tank of fluid kind `K`; the magnitude parameter
        /// `V` is appended by the macro, so this is `Tank<K, const V: u64>`.
        Tank<K: Kind>,
        unit = "grams remaining",
        must_use = "Tank is a conserved resource: even an empty tank must be passed on or handed to a Consumer"
    }

    crate::reusable_resource! {
        /// A demo generic rig of kind `K` with `SLOTS` fixture slots (mixed
        /// type + const generic shape).
        Rig<K: Kind, const SLOTS: u64>,
        must_use = "Rig is a reusable resource: pass it on or return it to the caller"
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

    /// A generic consumable with held contents mints (taking the payload by
    /// value) and defuses quietly; the payload is untripwired, so the
    /// sanctioned `mem::forget` in `defuse` skips no tripwire (F-036, F-008).
    #[test]
    fn generic_consumable_with_payload_defuses_quietly() {
        let carton: Carton<Red> = Carton::mint(Part::mint());
        carton.defuse();
    }

    /// The generic tripwire reports the full type name, parameters included.
    #[test]
    #[should_panic(expected = "Carton<")]
    fn abandoned_generic_consumable_trips_the_tripwire() {
        let carton: Carton<Red> = Carton::mint(Part::mint());
        let _still_bound_but_never_consumed = carton;
    }

    /// A const-parameter consumable (consts-only generic shape) carries the
    /// same kernel guarantees, and the tripwire prints the magnitude as a
    /// decimal (F-027).
    #[test]
    #[should_panic(expected = "Batch<9> dropped without being consumed")]
    fn abandoned_const_generic_consumable_trips_the_tripwire() {
        let batch: Batch<9> = Batch::mint();
        let _still_bound_but_never_consumed = batch;
    }

    /// A generic container exposes `VALUE`/`UNIT` with the macro-appended
    /// magnitude parameter last, and defuses quietly.
    #[test]
    fn generic_container_exposes_values() {
        assert_eq!(Tank::<Red, 500>::VALUE, 500);
        assert_eq!(Tank::<Red, 500>::UNIT, "grams remaining");
        let tank: Tank<Red, 500> = Tank::mint();
        tank.defuse();
    }

    /// The generic container tripwire reports the decimal magnitude (F-027)
    /// inside the full type name.
    #[test]
    #[should_panic(expected = "Red, 300> dropped without being consumed")]
    fn abandoned_generic_container_trips_the_tripwire() {
        let tank: Tank<Red, 300> = Tank::mint();
        let _still_bound_but_never_consumed = tank;
    }

    /// A generic reusable resource (mixed type + const shape) has no
    /// tripwire and stays with the caller (R2).
    #[test]
    fn generic_reusable_outlives_the_flow_quietly() {
        let rig: Rig<Red, 4> = Rig::mint();
        let _stays_with_the_caller = rig;
    }

    // ---- Outcome tokens (R17, F-042) ----

    crate::outcome_token! {
        /// A demo trial: whether a single stamping attempt succeeds.
        StampOutcome(StampOutcomeKind),
        success = stamp_goes_well,
        failure = stamp_jams,
        exit = return_stamp_outcome,
        must_use = "StampOutcome is a boundary token: run it through exactly one fallible process or return it to the environment"
    }

    /// The one process consuming the demo token (R17): both arms are
    /// ordinary conserving outcomes.
    fn stamp(outcome: StampOutcome) -> Result<(), ()> {
        match outcome.consume_kind() {
            StampOutcomeKind::Success => Ok(()),
            StampOutcomeKind::Failure => Err(()),
        }
    }

    /// The injected token value decides the arm (R17): processes stay
    /// deterministic, both outcomes are testable, and the consumed token is
    /// quiet.
    #[test]
    fn outcome_token_realises_the_injected_outcome() {
        assert!(stamp(stamp_goes_well()).is_ok());
        assert!(stamp(stamp_jams()).is_err());
    }

    /// An untried token has an accounted exit at the boundary (R12, F-050):
    /// the tripwire stays quiet.
    #[test]
    fn untried_outcome_token_returns_to_the_environment() {
        let token = stamp_goes_well();
        return_stamp_outcome(token);
    }

    /// An abandoned token is caught by its tripwire at test time (R1 layer 2,
    /// F-008): a provisioned trial must be run or returned, never dropped.
    #[test]
    #[should_panic(expected = "resource leak: StampOutcome dropped without being consumed")]
    fn abandoned_outcome_token_trips_the_tripwire() {
        let token = stamp_jams();
        let _still_bound_but_never_run = token;
    }

    /// Outcome-token fixtures are gated exactly like resource fixtures (R1,
    /// F-004): both outcomes can be conjured by downstream tests.
    #[cfg(feature = "test-support")]
    #[test]
    fn outcome_token_fixtures_are_feature_gated() {
        assert!(stamp(StampOutcome::test_fixture_success()).is_ok());
        assert!(stamp(StampOutcome::test_fixture_failure()).is_err());
    }

    /// Generic `test_fixture()`s are gated exactly like non-generic ones
    /// (R1, F-004): they exist only under the `test-support` feature, which
    /// reaches this unit-test build through the `[dev-dependencies]` self
    /// re-declaration. (The downstream positive path is exercised by
    /// pilot-workshop's integration tests.)
    #[cfg(feature = "test-support")]
    #[test]
    fn generic_test_fixtures_are_feature_gated() {
        let carton: Carton<Red> = Carton::test_fixture(Part::test_fixture());
        carton.defuse();
        let tank: Tank<Red, 5> = Tank::test_fixture();
        tank.defuse();
        let batch: Batch<3> = Batch::test_fixture();
        batch.defuse();
        let _rig: Rig<Red, 1> = Rig::test_fixture();
    }
}
