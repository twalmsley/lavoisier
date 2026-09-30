//! Deliberate holes in the privacy boundary — one type per known leak.
//!
//! Each type here looks superficially like a resource but makes one mistake
//! that lets a downstream crate obtain (or duplicate) a value without going
//! through the boundary module. `model-user`'s tests demonstrate each hole
//! actually working. **None of these mistakes may appear on a real resource.**

/// HOLE 1: `#[derive(Default)]`. The private field does not help — the derive
/// generates a public `impl Default`, so any crate can call
/// `DefaultHole::default()` and mint a resource from nothing.
#[derive(Default)]
pub struct DefaultHole(());

/// HOLE 2: public fields. If every field is public (or the struct is a unit
/// struct), downstream code can use struct-literal syntax to build one.
pub struct PubFieldsHole {
    pub serial: u32,
}

/// HOLE 2b: a public *unit* struct. Its name alone is a value-producing
/// expression — the emptiest possible constructor, and it is public.
pub struct UnitHole;

/// HOLE 3: public enum. Every variant of a `pub enum` is public; unit and
/// tuple variants are constructors any crate may call. Privacy of fields
/// cannot be applied to enum variants at all.
pub enum EnumHole {
    Pristine,
    Stamped(u32),
}

/// HOLE 3 mitigation: `#[non_exhaustive]` on a *variant* blocks downstream
/// construction of that variant (on stable since 1.0 for enums / 1.40 for
/// this use). A unit variant marked non_exhaustive is still constructible,
/// so payload variants must carry a sealed type instead. The robust fix is a
/// struct with a private field wrapping a private enum.
#[non_exhaustive]
pub enum SealedEnumMitigation {
    #[non_exhaustive]
    Pristine {},
}

/// HOLE 4: `#[derive(Clone)]`. Not creation from nothing, but duplication —
/// one bolt becomes two, which violates conservation (R1/R2) just as badly.
#[derive(Clone)]
pub struct CloneHole(());

/// HOLE 5: a public "parse"/"deserialize"-shaped constructor. Stand-in for
/// `serde::Deserialize` (external deps are forbidden here): any public trait
/// impl or inherent fn that returns `Self` from plain data is a constructor,
/// no matter what it is called. Deriving `Deserialize` on a resource would
/// have exactly this shape.
pub struct DeserializeHole(());

impl DeserializeHole {
    pub fn from_bytes(_bytes: &[u8]) -> Self {
        DeserializeHole(())
    }
}

/// NOT a hole (control): same shape as the real resources. Private field, no
/// derives, no public constructor. Exists so `model-user`'s compile-fail
/// tests have a local, minimal target too.
pub struct SealedControl(());
