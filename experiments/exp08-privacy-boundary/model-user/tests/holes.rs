//! Demonstrations that each deliberate hole in `model_lib::holes` really is a
//! hole: this downstream crate obtains (or duplicates) a "resource" without
//! ever touching a boundary module. Every passing test here is a way the
//! privacy boundary can be silently broken by one wrong attribute or `pub`.

use model_lib::holes::{
    CloneHole, DefaultHole, DeserializeHole, EnumHole, PubFieldsHole, UnitHole,
};

/// HOLE 1: `#[derive(Default)]` mints a resource from nothing, despite the
/// private field.
#[test]
fn derive_default_creates_from_nothing() {
    let _minted: DefaultHole = DefaultHole::default();
    let _minted_again: DefaultHole = Default::default();
}

/// HOLE 2: all-public fields allow struct-literal construction downstream.
#[test]
fn pub_fields_allow_literal_construction() {
    let _minted = PubFieldsHole { serial: 42 };
}

/// HOLE 2b: a pub unit struct's name is itself a constructor expression.
#[test]
fn unit_struct_is_its_own_constructor() {
    let _minted = UnitHole;
}

/// HOLE 3: every variant of a pub enum is a public constructor; variant
/// fields cannot be made private.
#[test]
fn enum_variants_are_public_constructors() {
    let _minted = EnumHole::Pristine;
    let _minted_with_payload = EnumHole::Stamped(7);
}

/// HOLE 4: `#[derive(Clone)]` duplicates a resource — conservation broken
/// even though creation from nothing is still impossible.
#[test]
fn derive_clone_duplicates_a_resource() {
    // We can't create a CloneHole here (private field, no Default)...
    fn duplicate(one: CloneHole) -> (CloneHole, CloneHole) {
        // ...but given one, we can turn it into two.
        let copy = one.clone();
        (one, copy)
    }
    let _ = duplicate; // The signature compiling is the demonstration.
}

/// HOLE 5: any public function returning `Self` from plain data is a
/// constructor — `serde::Deserialize` on a resource would be exactly this.
#[test]
fn deserialize_shaped_fn_creates_from_bytes() {
    let _minted = DeserializeHole::from_bytes(b"anything at all");
}

/// HOLE 6: `unsafe` defeats privacy entirely. Even the properly sealed `Bolt`
/// can be minted with `mem::zeroed` (or `transmute`, or `MaybeUninit`).
/// Privacy is a safe-Rust guarantee only; the mitigation is policy:
/// `#![forbid(unsafe_code)]` in every modelling crate, enforced by review/CI
/// (the library cannot force it on downstream crates).
#[test]
fn unsafe_mints_a_sealed_bolt() {
    let _minted: model_lib::resources::Bolt = unsafe { std::mem::zeroed() };
}
