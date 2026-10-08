//! The `model!` macro front-end (EXP-13).
//!
//! One `macro_rules!` tt-muncher over a small item grammar, each item
//! expanding to the same kernel-macro invocations and hand-written patterns
//! the control arm writes directly:
//!
//! | DSL item | expands to |
//! |---|---|
//! | `container Name[, [generics]], unit, must_use;` | `model_core::container_resource!` |
//! | `reusable Name, must_use;` | `model_core::reusable_resource!` |
//! | `source Name { enter, draw -> Out }, must_use, placeholder;` | reusable + boundary entry + R15 draw process (F-028) |
//! | `sink Name { enter, accepts [params] = Ty }, must_use, placeholder;` | sealed `Next = Self` boundary sink (R15, F-029) + entry |
//! | `entry name -> Name, placeholder;` | boundary constructor (R12) |
//! | `requirement { … }` | `model_core::requirement!` **pass-through** (keeps the `/// REQ-NNN:` and `trait` lines literal for trace.sh, F-021) |
//! | `requirement Name is "…" needs (…), assert, on_fail;` | sugared form — **generates** the doc line, measured to be invisible to trace.sh |
//! | `process fn name [generics] { threads/absorbs/consumes/mints/assert }` | a declarative conserving process: params, asserts, defuses, destructures and mints generated from the balance declaration |
//! | `process fn name [generics] { takes (…) -> (…); assert…; body {…} }` | escape hatch: hand-written body (permit-gated extractions etc.) |
//! | `flow test fn name { rust {…} step (outs) = call; send sink <- item; }` | a `#[test]` composing processes (`flow pub fn` for library flows) |
//! | `rust { … }` | verbatim item pass-through (everything the grammar cannot say) |
//!
//! **Grammar notes, all forced by stable `macro_rules!` (F-013/F-036 class):**
//! generics are written in square brackets `[const G: u64, K: Req…]` because
//! a `<`…`>` list is not one token tree and cannot be matched without a
//! tt-muncher per parameter; every generated item's name (`enter = …`,
//! `draw = …`, `assert = …`) must be spelled by the modeller because stable
//! `macro_rules!` cannot synthesize identifiers (F-013); and the `process fn`
//! / `flow test fn` keywords deliberately keep a literal `fn name` on the
//! source line so trace.sh's item detection and R10 rule 4 (bounds on the
//! fn-name line) keep working on the *invocation* text — the expansion is
//! invisible to every source-scanning tool (F-037 class).

/// The front-end. See the module docs for the grammar. Items expand in the
/// module where the macro is invoked, so the kernel's `pub(crate)`
/// `mint`/`defuse` and module-scoped field privacy work exactly as in a
/// hand-written model crate.
#[macro_export]
macro_rules! model {
    () => {};

    // ---- containers (R15) ---------------------------------------------
    (
        $(#[$meta:meta])*
        container $Name:ident, unit = $unit:literal, must_use = $msg:literal;
        $($rest:tt)*
    ) => {
        model_core::container_resource! {
            $(#[$meta])*
            $Name,
            unit = $unit,
            must_use = $msg
        }
        $crate::model! { $($rest)* }
    };
    (
        $(#[$meta:meta])*
        container $Name:ident [ $($gen:tt)* ], unit = $unit:literal, must_use = $msg:literal;
        $($rest:tt)*
    ) => {
        model_core::container_resource! {
            $(#[$meta])*
            $Name<$($gen)*>,
            unit = $unit,
            must_use = $msg
        }
        $crate::model! { $($rest)* }
    };

    // ---- reusable resources (R2) ---------------------------------------
    (
        $(#[$meta:meta])*
        reusable $Name:ident, must_use = $msg:literal;
        $($rest:tt)*
    ) => {
        model_core::reusable_resource! {
            $(#[$meta])*
            $Name,
            must_use = $msg
        }
        $crate::model! { $($rest)* }
    };

    // ---- an unbounded boundary source with its draw process (R15, F-028) -
    (
        $(#[$meta:meta])*
        source $Name:ident { enter = $enter:ident, draw = $draw:ident -> $Out:ident },
        must_use = $msg:literal, placeholder = $ph:literal;
        $($rest:tt)*
    ) => {
        model_core::reusable_resource! {
            $(#[$meta])*
            #[doc = ""]
            #[doc = concat!("Placeholder: ", $ph)]
            $Name,
            must_use = $msg
        }

        #[doc = concat!("The boundary source `", stringify!($Name), "` enters the model (R12).")]
        #[doc = ""]
        #[doc = concat!("Placeholder: ", $ph)]
        pub fn $enter() -> $Name {
            $Name::mint()
        }

        #[doc = concat!("Draws `TAKE` of `", stringify!($Out), "` from the unbounded `", stringify!($Name), "` (R15: a draw-style boundary process, never a `Supplier` impl, F-028). The source is returned (R2).")]
        #[doc = ""]
        #[doc = concat!("Placeholder: ", $ph)]
        pub fn $draw<const TAKE: u64>(src: $Name) -> ($Out<TAKE>, $Name) {
            ($Out::mint(), src)
        }
        $crate::model! { $($rest)* }
    };

    // ---- an unbounded boundary sink (R15, F-029) ------------------------
    (
        $(#[$meta:meta])*
        sink $Name:ident { enter = $enter:ident, accepts [ $($ig:tt)* ] = $In:ty },
        must_use = $msg:literal, placeholder = $ph:literal;
        $($rest:tt)*
    ) => {
        $(#[$meta])*
        #[doc = ""]
        #[doc = concat!("Placeholder: ", $ph)]
        #[must_use = $msg]
        pub struct $Name {
            _seal: (),
        }

        /// Unbounded boundary sink (R15): `Next = Self`, legal only at the
        /// system boundary; it necessarily discards what it consumes (F-029).
        impl< $($ig)* > model_core::boundary::Consumer<$In> for $Name {
            type Next = $Name;
            fn consume(self, item: $In) -> $Name {
                item.defuse(); // sanctioned consumer role (F-008)
                self
            }
        }

        #[doc = concat!("The boundary sink `", stringify!($Name), "` enters the model (R12). An empty unbounded sink holds nothing, so this is an ordinary public boundary function.")]
        #[doc = ""]
        #[doc = concat!("Placeholder: ", $ph)]
        pub fn $enter() -> $Name {
            $Name { _seal: () }
        }
        $crate::model! { $($rest)* }
    };

    // ---- a plain boundary entry for a reusable (R12) ---------------------
    (
        entry $enter:ident -> $Name:ident, placeholder = $ph:literal;
        $($rest:tt)*
    ) => {
        #[doc = concat!("`", stringify!($Name), "` enters the model (R12).")]
        #[doc = ""]
        #[doc = concat!("Placeholder: ", $ph)]
        pub fn $enter() -> $Name {
            $Name::mint()
        }
        $crate::model! { $($rest)* }
    };

    // ---- requirement, pass-through form (R10; trace.sh-compatible) -------
    ( requirement { $($body:tt)* } $($rest:tt)* ) => {
        model_core::requirement! { $($body)* }
        $crate::model! { $($rest)* }
    };

    // ---- requirement, sugared form (measured: the generated doc line is
    //      invisible to trace.sh — see RESULTS.md) -------------------------
    (
        requirement $Req:ident is $sentence:literal needs ( $($bound:tt)+ ),
        assert = $afn:ident, on_fail = $fail:literal;
        $($rest:tt)*
    ) => {
        model_core::requirement! {
            #[doc = $sentence]
            #[diagnostic::on_unimplemented(message = $fail)]
            pub trait $Req: ( $($bound)+ );
            assert = $afn;
        }
        $crate::model! { $($rest)* }
    };

    // ---- declarative conserving process (R1/R2/R3/R15) -------------------
    // threads: reusables moved in and returned unchanged (R2)
    // absorbs:  this module's reusables consumed into a new state (destructured)
    // consumes: tripwired inputs whose magnitudes continue into the outputs
    // mints:    the outputs (inference fills their const arguments)
    // assert:   one compile-time conservation assert per dimension (R3)
    (
        $(#[$meta:meta])*
        process fn $f:ident [ $($gen:tt)* ] {
            $( threads { $( $tn:ident : $TT:ty ),+ $(,)? } )?
            $( absorbs { $( $an:ident : $AT:ident ),+ $(,)? } )?
            $( consumes { $( $cn:ident : $CT:ty ),+ $(,)? } )?
            mints { $( $OT:ty ),+ $(,)? }
            $( assert $amsg:literal : $aexpr:expr; )*
        }
        $($rest:tt)*
    ) => {
        $(#[$meta])*
        pub fn $f< $($gen)* >(
            $($( $tn : $TT, )+)?
            $($( $an : $AT, )+)?
            $($( $cn : $CT, )+)?
        ) -> ( $($( $TT, )+)? $( $OT, )+ ) {
            $( const { assert!($aexpr, $amsg) }; )*
            // Conserving transforms (R1): consumed inputs are defused (their
            // magnitudes continue in the minted outputs), absorbed reusables
            // are destructured into their next state.
            $($( $cn.defuse(); )+)?
            $($( let $AT { _seal: () } = $an; )+)?
            ( $($( $tn, )+)? $( <$OT>::mint(), )+ )
        }
        $crate::model! { $($rest)* }
    };

    // ---- raw-body process: the escape hatch for anything the declarative
    //      form cannot say (permit-gated extractions, characteristic consts) -
    (
        $(#[$meta:meta])*
        process fn $f:ident [ $($gen:tt)* ] {
            takes ( $( $pn:ident : $PT:ty ),* $(,)? ) -> ( $($RT:tt)* );
            $( assert $amsg:literal : $aexpr:expr; )*
            body { $($body:tt)* }
        }
        $($rest:tt)*
    ) => {
        $(#[$meta])*
        pub fn $f< $($gen)* >( $( $pn : $PT ),* ) -> ( $($RT)* ) {
            $( const { assert!($aexpr, $amsg) }; )*
            $($body)*
        }
        $crate::model! { $($rest)* }
    };

    // ---- flows (R9): a #[test] (or pub fn) composing processes -----------
    (
        $(#[$meta:meta])*
        flow test fn $f:ident { $($body:tt)* }
        $($rest:tt)*
    ) => {
        $(#[$meta])*
        #[test]
        fn $f() {
            $crate::__model_flow! { $($body)* }
        }
        $crate::model! { $($rest)* }
    };
    (
        $(#[$meta:meta])*
        flow pub fn $f:ident { $($body:tt)* }
        $($rest:tt)*
    ) => {
        $(#[$meta])*
        pub fn $f() {
            $crate::__model_flow! { $($body)* }
        }
        $crate::model! { $($rest)* }
    };

    // ---- verbatim pass-through: everything the grammar cannot say --------
    ( rust { $($t:tt)* } $($rest:tt)* ) => {
        $($t)*
        $crate::model! { $($rest)* }
    };
}

/// Internal statement muncher for `model!`'s flow bodies. Not part of the
/// public grammar.
#[doc(hidden)]
#[macro_export]
macro_rules! __model_flow {
    () => {};
    // verbatim statements
    ( rust { $($t:tt)* } $($rest:tt)* ) => {
        $($t)*
        $crate::__model_flow! { $($rest)* }
    };
    // one process step: named outputs from one call
    ( step ( $($out:ident),+ $(,)? ) = $call:expr; $($rest:tt)* ) => {
        let ( $($out),+ ) = $call;
        $crate::__model_flow! { $($rest)* }
    };
    // one process step with a single output
    ( step $out:ident = $call:expr; $($rest:tt)* ) => {
        let $out = $call;
        $crate::__model_flow! { $($rest)* }
    };
    // hand an item to a boundary consumer, rebinding the sink (R12)
    ( send $sink:ident <- $item:expr; $($rest:tt)* ) => {
        let $sink = model_core::boundary::send_to($sink, $item);
        $crate::__model_flow! { $($rest)* }
    };
}
