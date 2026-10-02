//! Type-level lists holding real values (R12, R13).
//!
//! A supplier's contents are a nested list built into its type,
//! e.g. `BoltBox<Cons<Bolt, Cons<Bolt, Nil>>>` — the count **is** the length
//! of the list, so count and contents can never disagree (R12). Discrete
//! items are separate objects (R13): the list stores the real values, not a
//! number with a "count" unit.
//!
//! `Cons`'s fields are public: the list cell itself is not a resource (the
//! items inside it are sealed), and model code must be able to destructure
//! the list a [`crate::boundary::SupplyN`] bound returns. Owning a `Cons` of
//! resources is like owning a tuple of them — building one neither creates
//! nor destroys anything.
//!
//! Adapted from `experiments/exp02-supplier-consumer/src/lib.rs`.

use crate::nat::{Nat, Succ, Zero};

/// The empty type-level list.
pub struct Nil;

/// A list cell holding a real value of type `H` and the rest of the list.
#[must_use = "this list carries conserved resources: pass them on or hand them to a Consumer"]
pub struct Cons<H, T>(pub H, pub T);

/// The length of a type-level list, exposed both as a type-level number and
/// as a constant (R7, R12).
pub trait Len {
    /// The length as a type-level [`Nat`].
    type Length: Nat;
    /// The length as a numeric constant.
    const LEN: u64;
}

impl Len for Nil {
    type Length = Zero;
    const LEN: u64 = 0;
}

impl<H, T: Len> Len for Cons<H, T> {
    type Length = Succ<T::Length>;
    const LEN: u64 = T::LEN + 1;
}

#[cfg(test)]
mod tests {
    use super::{Cons, Len, Nil};
    use crate::nat::Nat;
    use crate::nat::aliases::N3;
    use core::marker::PhantomData;

    type Three = Cons<u8, Cons<u8, Cons<u8, Nil>>>;

    /// The length comes out both as a constant and as a `Nat` type, and the
    /// two agree by construction.
    #[test]
    fn length_as_const_and_nat_agree() {
        assert_eq!(<Nil as Len>::LEN, 0);
        assert_eq!(<Three as Len>::LEN, 3);
        assert_eq!(<Three as Len>::Length::VALUE, 3);
        // Type-level equality: only compiles if Length IS N3.
        let _proof: PhantomData<<Three as Len>::Length> = PhantomData::<N3>;
    }

    /// Lists of real values destructure; nothing is copied or created.
    #[test]
    fn cons_destructures_by_value() {
        let list = Cons(1u8, Cons(2u8, Nil));
        let Cons(a, Cons(b, Nil)) = list;
        assert_eq!((a, b), (1, 2));
    }
}
