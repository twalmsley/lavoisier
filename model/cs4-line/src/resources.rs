//! The line's own sealed resources (R1): the reusable tools (R2) and the
//! swarf bin, with the line's creation [`boundary`] as a child module
//! (F-006).
//!
//! Everything *material* is a `cs4_stores` type — the line owns only tools
//! and the bin (SPEC §3's ownership table). The bin is the line's
//! contents-keeping waste consumer (REQ-020's collection half): capacity 80,
//! ending the batch holding 75 pieces (SPEC §3), with the **decreasing space
//! parameter the F-034 hard rule demands** — without one, trait resolution
//! diverges, and at this workspace's mandated `recursion_limit = "2048"`
//! that is a deterministic compiler crash, not an error.

use core::marker::PhantomData;
use cs4_stores::resources::{CutSwarf, DrillSwarf, SwarfReturn};
use model_core::boundary::Consumer;
use model_core::list::{Cons, Len, Nil};
use model_core::nat::Succ;
use model_core::nat::aliases::N80;

model_core::reusable_resource! {
    /// The line's power saw (R2): moved into every cutting step and
    /// returned. No tripwire — a reusable resource legitimately outlives the
    /// flow.
    Saw,
    must_use = "Saw is a reusable resource: pass it on or return it to the caller"
}

model_core::reusable_resource! {
    /// The line's pillar drill (R2): moved into every drilling step and
    /// returned.
    PillarDrill,
    must_use = "PillarDrill is a reusable resource: pass it on or return it to the caller"
}

model_core::reusable_resource! {
    /// The line's workbench (R2): moved into every fastening step and
    /// returned.
    Workbench,
    must_use = "Workbench is a reusable resource: pass it on or return it to the caller"
}

/// The line's swarf capacity, in pieces (SPEC §3): the batch's 75 pieces fit
/// with five slots spare.
pub const BIN_CAPACITY: u64 = 80;

/// The line's swarf bin (SPEC §3): the dedicated collection consumer REQ-020
/// routes every piece of batch swarf through. `Space` is the type-level
/// number of pieces it can still accept; `Contents` keeps the real swarf
/// objects it has consumed (F-016). The decreasing space parameter is a hard
/// rule, not style (F-034). The bin cannot defuse what it keeps — swarf is
/// sealed in `cs4-stores` — so its only emptying path is the crate-edge
/// [`SwarfReturn`] hand-over to stores' disposal at reconciliation (P6,
/// REQ-020, the F-039 shape across crates).
///
/// Placeholder: line-side swarf bin — refine to a named waste stream.
#[must_use = "SwarfBin is the line's swarf collection (REQ-020): pass it on and return it at reconciliation"]
pub struct SwarfBin<Space, Contents = Nil> {
    contents: Contents,
    _space: PhantomData<Space>,
}

/// The bin as the line starts (and, after P6 hands back the emptied bin,
/// ends) the batch: all 80 slots free, nothing kept.
pub type EmptySwarfBin = SwarfBin<N80, Nil>;

/// `Consumer` is implemented ONLY while space remains (R12, F-034): space
/// goes down by one and the real cut-swarf object is kept at the front of
/// the contents list (F-016). An 81st piece is a compile error with
/// model-core's modeller-phrased message (F-015) —
/// `tests/ui/bin_cannot_take_an_81st_piece.rs` pins it.
impl<S, C, const G: u64> Consumer<CutSwarf<G>> for SwarfBin<Succ<S>, C> {
    type Next = SwarfBin<S, Cons<CutSwarf<G>, C>>;
    fn consume(self, item: CutSwarf<G>) -> Self::Next {
        SwarfBin {
            contents: Cons(item, self.contents),
            _space: PhantomData,
        }
    }
}

/// Drill swarf is kept the same way (F-016, F-034).
impl<S, C, const G: u64> Consumer<DrillSwarf<G>> for SwarfBin<Succ<S>, C> {
    type Next = SwarfBin<S, Cons<DrillSwarf<G>, C>>;
    fn consume(self, item: DrillSwarf<G>) -> Self::Next {
        SwarfBin {
            contents: Cons(item, self.contents),
            _space: PhantomData,
        }
    }
}

impl<Space, Contents: Len> SwarfBin<Space, Contents> {
    /// How many pieces of swarf the bin holds — the length of its contents
    /// list, so count and contents cannot disagree (R7, R12).
    pub const HELD: u64 = Contents::LEN;
}

/// The crate-edge interface (REQ-020): stores' P6 opens the bin through the
/// trait **stores defines** and the line implements — stores cannot name the
/// line's types (the dependency points the other way), and the line cannot
/// defuse the swarf it kept (sealed across the edge), so the hand-over is
/// the only emptying path. The kept swarf goes over for disposal and a fresh
/// empty bin comes back to the line (SPEC §5/P6: "bin returned empty").
impl<Space, C> SwarfReturn for SwarfBin<Space, C> {
    type Contents = C;
    type Emptied = EmptySwarfBin;
    fn open(self) -> (C, EmptySwarfBin) {
        (
            self.contents,
            SwarfBin {
                contents: Nil,
                _space: PhantomData,
            },
        )
    }
}

/// The line's creation boundary (R12, F-006): tools and the empty bin enter
/// the model here. Nothing material: materials come only from stores' P1
/// (REQ-019).
pub mod boundary {
    use super::{Nil, PhantomData, PillarDrill, Saw, SwarfBin, Workbench};

    /// The line's three reusable tools enter the model at shift start
    /// (R2, R12; SPEC §4).
    ///
    /// Placeholder: tool stores — one saw, one pillar drill, one workbench.
    pub fn tool_up() -> (Saw, PillarDrill, Workbench) {
        (Saw::mint(), PillarDrill::mint(), Workbench::mint())
    }

    /// An empty swarf bin with `Space` slots (REQ-020's collection half).
    /// Creating an *empty* consumer brings no resources into existence, so
    /// this is an ordinary public boundary function (R12):
    /// `new_swarf_bin::<N80>()`.
    pub fn new_swarf_bin<Space>() -> SwarfBin<Space, Nil> {
        SwarfBin {
            contents: Nil,
            _space: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    //! Per-resource unit tests (R5). The line's tests sit outside stores'
    //! privacy boundary, so every material here is a `test-support` fixture
    //! (F-004) and every piece of swarf must leave through the real disposal
    //! path — exactly like production code.

    use super::boundary::{new_swarf_bin, tool_up};
    use super::{EmptySwarfBin, SwarfBin};
    use cs4_stores::resources::boundary::new_disposal;
    use cs4_stores::resources::{CutSwarf, DrillSwarf, SwarfReturn};
    use model_core::boundary::{send_list, send_to};
    use model_core::list::{Cons, Nil};
    use model_core::nat::Zero;
    use model_core::nat::aliases::N2;

    /// The bin keeps what it consumes while its space counts down (R12,
    /// F-016, F-034), and its only emptying path hands the kept swarf across
    /// the crate edge for disposal, returning the bin empty (REQ-020).
    ///
    /// Verifies: REQ-020
    #[test]
    fn bin_keeps_swarf_until_the_crate_edge_hand_over() {
        let bin = new_swarf_bin::<N2>();
        assert_eq!(SwarfBin::<N2>::HELD, 0);
        let bin = send_to(bin, CutSwarf::<500>::test_fixture());
        let bin = send_to(bin, DrillSwarf::<10>::test_fixture());
        assert_eq!(SwarfBin::<Zero, Cons<DrillSwarf<10>, Cons<CutSwarf<500>, Nil>>>::HELD, 2);
        // The full state is distinct (R12); opening it is the P6 hand-over.
        let full: SwarfBin<Zero, _> = bin;
        let (swarf, emptied) = full.open();
        let _disposal = send_list(new_disposal(), swarf);
        assert_eq!(EmptySwarfBin::HELD, 0);
        let _bin_back_with_the_line: EmptySwarfBin = emptied;
    }

    /// Tools are reusable (R2): created at the boundary, they stay with the
    /// caller; no tripwire fires when they go out of scope.
    #[test]
    fn tools_outlive_the_flow_quietly() {
        let (_saw, _press, _bench) = tool_up();
    }
}
