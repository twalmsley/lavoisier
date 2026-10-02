//! Design probe (candidate R17): **two distinct outcome token types selected
//! by the flow**, instead of one token type with success/failure boundary
//! constructors ([`crate::model::DrillOutcome`]).
//!
//! The probe builds the design both ways it can exist on stable Rust —
//! two monomorphic process variants, and one generic process over a sealed
//! token trait with a const-generic GAT output — and demonstrates its
//! defining property: **the outcome is part of the flow's static text**.
//! A `WillSucceed` flow *is* the success flow and an `WillFail` flow *is*
//! the failure flow; there is no runtime branch, no `Result`, and therefore
//! nothing that forces a flow to handle an outcome it did not choose. The
//! design collapses "fallible process" back into "two infallible processes"
//! — it reintroduces exactly the "every process succeeds (per flow)" state
//! that candidate R17 exists to remove. See RESULTS.md for the verdict.
//!
//! The probe reuses [`crate::model`]'s resources and bundles; its tokens are
//! sealed like any resource (private field, boundary constructors) but
//! deliberately untripwired — they are probe scaffolding, not part of the
//! recommended design.

use crate::model::{DrillBit, DrillFail, DrillOk, DrilledPlate, Plate, ScrapPlate, Swarf};
use model_core::common::Person;
use model_core::common::processes::draw_time;

/// A trial known **at compile time** to succeed. Sealed (R1).
#[must_use = "WillSucceed is a boundary token: it must be consumed by exactly one process"]
pub struct WillSucceed {
    _seal: (),
}

/// A trial known **at compile time** to fail. Sealed (R1).
#[must_use = "WillFail is a boundary token: it must be consumed by exactly one process"]
pub struct WillFail {
    _seal: (),
}

/// Boundary constructors for the probe tokens (R12).
pub mod boundary {
    use super::{WillFail, WillSucceed};

    /// A statically-successful trial enters the model.
    pub fn will_succeed() -> WillSucceed {
        WillSucceed { _seal: () }
    }

    /// A statically-failing trial enters the model.
    pub fn will_fail() -> WillFail {
        WillFail { _seal: () }
    }
}

// ---------------------------------------------------------------------------
// Variant (a): two monomorphic process variants — no Result anywhere.
// ---------------------------------------------------------------------------

/// Drilling with a [`WillSucceed`] token: the outcome is in the signature, so
/// the return type is the bare success bundle — **no `Result`, no failure arm
/// to handle**. The flow that calls this has statically decided drilling
/// succeeds.
pub fn drill_known_good<
    const SPEND: u64,
    const T_LEFT: u64,
    const BUDGET: u64,
    const PLATE: u64,
    const P_LEFT: u64,
    const SW: u64,
>(
    person: Person<BUDGET>,
    bit: DrillBit,
    plate: Plate<PLATE>,
    token: WillSucceed,
) -> DrillOk<T_LEFT, P_LEFT, SW, SPEND> {
    const {
        assert!(
            P_LEFT + SW == PLATE,
            "mass conservation violated in drill_known_good (R3): the drilled plate plus its swarf must sum exactly to the plate blank"
        )
    };
    let WillSucceed { _seal: () } = token;
    let (labour, person) = draw_time::<SPEND, T_LEFT, BUDGET>(person);
    plate.defuse();
    DrillOk {
        person,
        bit,
        plate: DrilledPlate::mint(),
        swarf: Swarf::mint(),
        labour,
    }
}

/// Drilling with a [`WillFail`] token: the bare failure bundle comes back —
/// again no `Result`. Note this variant can only ever appear in a flow whose
/// author already scripted the failure.
pub fn drill_known_bad<
    const SPEND: u64,
    const T_LEFT: u64,
    const BUDGET: u64,
    const PLATE: u64,
    const SCRAP: u64,
    const FSW: u64,
>(
    person: Person<BUDGET>,
    bit: DrillBit,
    plate: Plate<PLATE>,
    token: WillFail,
) -> DrillFail<T_LEFT, SCRAP, FSW, SPEND> {
    const {
        assert!(
            SCRAP + FSW == PLATE,
            "mass conservation violated in drill_known_bad (R3): the scrap plate plus its swarf must sum exactly to the plate blank"
        )
    };
    let WillFail { _seal: () } = token;
    let (labour, person) = draw_time::<SPEND, T_LEFT, BUDGET>(person);
    plate.defuse();
    DrillFail {
        person,
        broken_bit: bit.snap(),
        scrap: ScrapPlate::mint(),
        swarf: Swarf::mint(),
        labour,
    }
}

// ---------------------------------------------------------------------------
// Variant (b): one generic process over a sealed token trait, with a
// const-generic GAT carrying the per-token output type.
// ---------------------------------------------------------------------------

mod sealed {
    /// Seals [`super::StaticOutcome`] (F-026): only the two probe tokens may
    /// implement it.
    pub trait Sealed {}
    impl Sealed for super::WillSucceed {}
    impl Sealed for super::WillFail {}
}

/// The generic form of the two-token design: the output type is a GAT over
/// the drilling consts, defined per token. This *does* compile on stable
/// (GATs with const parameters, 1.65+), so the design is expressible — but
/// `drill_static`'s return type is `O::Out<…>`, an opaque projection: a flow
/// generic over the token can do **nothing** with the output except pass it
/// on, and a flow concrete in the token has statically chosen the outcome.
/// Either way, no code is ever forced to handle the arm it did not pick.
pub trait StaticOutcome: sealed::Sealed {
    /// The per-token outcome bundle.
    type Out<
        const T_LEFT: u64,
        const P_LEFT: u64,
        const SW: u64,
        const SCRAP: u64,
        const FSW: u64,
        const SPEND: u64,
    >;

    /// Realises the outcome. Each impl carries only its own branch's
    /// conservation assert — the unchosen branch's split is never checked at
    /// a call site, unlike `drill_fallible`, which proves both.
    fn realise<
        const SPEND: u64,
        const T_LEFT: u64,
        const BUDGET: u64,
        const PLATE: u64,
        const P_LEFT: u64,
        const SW: u64,
        const SCRAP: u64,
        const FSW: u64,
    >(
        self,
        person: Person<BUDGET>,
        bit: DrillBit,
        plate: Plate<PLATE>,
    ) -> Self::Out<T_LEFT, P_LEFT, SW, SCRAP, FSW, SPEND>;
}

impl StaticOutcome for WillSucceed {
    type Out<
        const T_LEFT: u64,
        const P_LEFT: u64,
        const SW: u64,
        const SCRAP: u64,
        const FSW: u64,
        const SPEND: u64,
    > = DrillOk<T_LEFT, P_LEFT, SW, SPEND>;

    fn realise<
        const SPEND: u64,
        const T_LEFT: u64,
        const BUDGET: u64,
        const PLATE: u64,
        const P_LEFT: u64,
        const SW: u64,
        const SCRAP: u64,
        const FSW: u64,
    >(
        self,
        person: Person<BUDGET>,
        bit: DrillBit,
        plate: Plate<PLATE>,
    ) -> DrillOk<T_LEFT, P_LEFT, SW, SPEND> {
        drill_known_good::<SPEND, T_LEFT, BUDGET, PLATE, P_LEFT, SW>(person, bit, plate, self)
    }
}

impl StaticOutcome for WillFail {
    type Out<
        const T_LEFT: u64,
        const P_LEFT: u64,
        const SW: u64,
        const SCRAP: u64,
        const FSW: u64,
        const SPEND: u64,
    > = DrillFail<T_LEFT, SCRAP, FSW, SPEND>;

    fn realise<
        const SPEND: u64,
        const T_LEFT: u64,
        const BUDGET: u64,
        const PLATE: u64,
        const P_LEFT: u64,
        const SW: u64,
        const SCRAP: u64,
        const FSW: u64,
    >(
        self,
        person: Person<BUDGET>,
        bit: DrillBit,
        plate: Plate<PLATE>,
    ) -> DrillFail<T_LEFT, SCRAP, FSW, SPEND> {
        drill_known_bad::<SPEND, T_LEFT, BUDGET, PLATE, SCRAP, FSW>(person, bit, plate, self)
    }
}

/// The one generic process of variant (b). Compiles and runs — but its
/// output is `O::Out<…>`, which a token-generic caller cannot inspect,
/// account for, or even name concretely: the outcome-handling obligation
/// evaporates instead of being enforced.
pub fn drill_static<
    O: StaticOutcome,
    const SPEND: u64,
    const T_LEFT: u64,
    const BUDGET: u64,
    const PLATE: u64,
    const P_LEFT: u64,
    const SW: u64,
    const SCRAP: u64,
    const FSW: u64,
>(
    person: Person<BUDGET>,
    bit: DrillBit,
    plate: Plate<PLATE>,
    token: O,
) -> O::Out<T_LEFT, P_LEFT, SW, SCRAP, FSW, SPEND> {
    token.realise::<SPEND, T_LEFT, BUDGET, PLATE, P_LEFT, SW, SCRAP, FSW>(person, bit, plate)
}

#[cfg(test)]
mod tests {
    use super::boundary::{will_fail, will_succeed};
    use super::drill_static;
    use crate::model::boundary::supply_drill_bit;
    use crate::model::boundary::supply_plate;
    use model_core::common::boundary::new_person;
    use model_core::history::boundary::new_history;
    use model_core::history::processes::record;

    /// Variant (b) type-checks and runs on stable — the probe's point is not
    /// that the design fails to compile, but that each flow statically picks
    /// its outcome: this test's text *is* the success path, and no failure
    /// arm exists anywhere in it to handle.
    #[test]
    fn static_tokens_pick_the_outcome_in_the_flow_text() {
        let ok = drill_static::<_, 1000, 4000, 5000, 450, 440, 10, 430, 20>(
            new_person::<5000>(),
            supply_drill_bit(),
            supply_plate::<450>(),
            will_succeed(),
        );
        ok.plate.defuse();
        ok.swarf.defuse();
        let _record_a = record(new_history(), "drill_static", ok.labour);
        let _reusables = (ok.person, ok.bit);

        let fail = drill_static::<_, 1000, 4000, 5000, 450, 440, 10, 430, 20>(
            new_person::<5000>(),
            supply_drill_bit(),
            supply_plate::<450>(),
            will_fail(),
        );
        fail.scrap.defuse();
        fail.swarf.defuse();
        fail.broken_bit.defuse();
        let _record_b = record(new_history(), "drill_static", fail.labour);
        let _person = fail.person;
    }
}
