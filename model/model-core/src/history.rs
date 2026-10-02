//! Execution history (R16): "the past" as an explicit boundary sink.
//!
//! Consumed time — and any other event worth recording — is accounted to
//! [`History`], a sealed boundary consumer whose value doubles as the record
//! of the model execution. The design is the **value-level-records-only**
//! variant R16 mandates: `History` defuses each tripwired item it consumes
//! (the consumer's sanctioned role, F-008) and keeps an untripwired,
//! value-level [`Event`] — process name, what was consumed, magnitude, unit —
//! in a runtime list inside the sealed type.
//!
//! The **type-level-recording variant is forbidden** (R16):
//! `History<Cons<Event1, …>>` is exactly the F-034 shape — a contents-keeping
//! consumer with no decreasing space parameter — which diverges trait
//! resolution (a deterministic rustc SIGBUS at the mandated recursion limit)
//! and whose ever-growing type would infect every downstream signature.
//!
//! **Per-branch histories, merged at joins** (R16): create one `History` per
//! concurrent branch of a flow with [`boundary::new_history`] — never a
//! single global one threaded everywhere, which under strict conservation
//! (R2) would be one resource serializing the entire model and deleting the
//! concurrency R9 exists to allow. [`processes::merge`] combines branch
//! histories at join points; the merged record is a **partial order**
//! ([`Entry::Join`]) — deliberately, because concurrent branches have no
//! defined interleaving, and separate-then-merged records state that
//! truthfully.
//!
//! `History` is the **production-legal sink for [`Labour`]** (F-035: whoever
//! mints a tripwired type must also ship at least one production consumer for
//! it). It supersedes the earlier `TimeLedger` placeholder, so F-035 stays
//! satisfied: downstream production flows account for drawn time by recording
//! it here.
//!
//! Layout per F-006: the sealed type and the record types in this module,
//! with the creation [`boundary`] and the conserving [`processes`] as child
//! modules. Recording defuses tripwired resources (the sanctioned
//! `mem::forget` role reached through their sealed `defuse`), which is why
//! the machinery lives inside the privacy boundary, gated by the [`Permit`]
//! token.
//!
//! ## A worked example (runs as a doc-test)
//!
//! Two branches draw time independently (R9); each records into its own
//! history, attributed to the process that spent the time; the join merges
//! them into one partial-order record that stays with the caller:
//!
//! ```
//! use model_core::common::boundary::new_person;
//! use model_core::common::processes::draw_time;
//! use model_core::history::boundary::new_history;
//! use model_core::history::processes::{merge, record};
//! use model_core::history::Entry;
//!
//! // One history per concurrent branch (R16).
//! let h1 = new_history();
//! let h2 = new_history();
//!
//! // Branch 1: Alice drills; branch 2: Bob cuts (no shared resources, R9).
//! let alice = new_person::<10_000>();
//! let bob = new_person::<8000>();
//! let (l1, alice) = draw_time::<2000, 8000, 10_000>(alice);
//! let (l2, bob) = draw_time::<3000, 5000, 8000>(bob);
//! let h1 = record(h1, "drill_holes", l1);
//! let h2 = record(h2, "cut", l2);
//!
//! // The join: the merged record is a partial order (R16) — no
//! // interleaving is claimed between the branches.
//! let history = merge(h1, h2);
//! assert_eq!(history.event_count(), 2);
//! assert!(matches!(history.entries(), [Entry::Join(_, _)]));
//!
//! // The record legitimately survives the flow and stays with the caller.
//! let _execution_record = history;
//! let _reusables = (alice, bob);
//! ```

use crate::boundary::Consumer;
use crate::common::Labour;

/// The process name carried by events recorded through the generic
/// [`Consumer`] path ([`crate::boundary::send_to`]) rather than
/// [`processes::record`], which attributes the event to a named process.
pub const UNATTRIBUTED: &str = "(unattributed)";

/// One recorded event (R16): which process consumed what, and how much.
///
/// `Event` is a **plain data record, not a resource**, so the R1 sealing
/// rules deliberately do not apply: its fields are public and anyone may
/// build one. A record cannot mint resources — fabricating an `Event` brings
/// nothing conserved into existence — and the public fields are exactly what
/// a downstream [`Recordable`] implementor needs to describe its own
/// resource. What keeps a [`History`] trustworthy is not `Event`'s privacy
/// but `History`'s: its entry list is private, and entries are appended only
/// by this module's machinery, which demands a real resource through
/// [`Recordable`] (gated by [`Permit`]) for every event it records.
///
/// Records must carry enough information to serve as an execution record
/// usable in generated documents (R16), hence the explicit unit alongside the
/// magnitude (R7).
#[derive(Debug, PartialEq, Eq)]
pub struct Event {
    /// The process the consumption is attributed to ([`UNATTRIBUTED`] when
    /// the event arrived through the generic [`Consumer`] path).
    pub process: &'static str,
    /// What was consumed (the resource type's model name, e.g. `"Labour"`).
    pub item: &'static str,
    /// How much, in `unit` (R7: integers in the base unit).
    pub magnitude: u64,
    /// The base unit `magnitude` is counted in (R7 table).
    pub unit: &'static str,
}

/// An entry in a history: a single [`Event`], or the join of two concurrent
/// branches (R16).
///
/// A history's entries are a **series–parallel partial order** — the faithful
/// shape for R16's "the merged record is a partial order": entries in one
/// list happened in that order; the two lists inside a [`Entry::Join`] are
/// concurrent branches between which **no interleaving is claimed**. Like
/// [`Event`], `Entry` is a plain data record, not a resource, so the R1
/// sealing rules (including "never a `pub enum` resource") do not apply:
/// records cannot mint resources, and a fabricated `Entry` can never enter a
/// sealed [`History`].
#[derive(Debug, PartialEq, Eq)]
pub enum Entry {
    /// One recorded consumption.
    Event(Event),
    /// A join point (R16): the records of two branches that ran with no
    /// defined interleaving between them.
    Join(Vec<Entry>, Vec<Entry>),
}

impl Entry {
    /// Recursively counts the [`Entry::Event`]s under this entry.
    fn count(&self) -> usize {
        match self {
            Entry::Event(_) => 1,
            Entry::Join(left, right) => {
                left.iter().map(Entry::count).sum::<usize>()
                    + right.iter().map(Entry::count).sum::<usize>()
            }
        }
    }
}

/// The execution history (R16): a sealed boundary consumer that accounts for
/// conserved outputs by keeping value-level records of them — "the past" as
/// an explicit sink, doubling as the record of the model execution.
///
/// Sealed (R1): private fields, no public constructor, no
/// `Clone`/`Copy`/`Default`; created only by [`boundary::new_history`] — one
/// per concurrent branch (R16) — and passed by value like any other resource
/// (R2). **No tripwire `Drop`**, deliberately: the tripwire regime marks
/// values that must reach a consumer before the flow ends, but the history
/// *is* the record that legitimately survives the flow and stays with the
/// caller — its abandonment loses bookkeeping, not conserved resources, and
/// `#[must_use]` still catches whole-value discard.
#[must_use = "History is the execution record (R16): it stays with the caller - merge it at joins, do not discard it"]
pub struct History {
    // LIMITATION (R13, R14): a `Vec`, not a fixed-size collection — the
    // number of recorded events is flow-dependent and unknowable at compile
    // time, so a fixed size is impossible here. Explicitly sanctioned by R16
    // (value-level records only; the type-level alternative is the forbidden
    // F-034 divergence).
    entries: Vec<Entry>,
    _seal: (),
}

impl History {
    /// The recorded entries, in recording order within this branch
    /// (read-only: entries are appended only by [`processes::record`] and the
    /// [`Consumer`] impl, each demanding a real resource).
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// The total number of recorded events, recursing through every
    /// [`Entry::Join`]'s branches.
    pub fn event_count(&self) -> usize {
        self.entries.iter().map(Entry::count).sum()
    }
}

/// The recording permit: a sealed zero-size token proving the call comes from
/// [`History`]'s own machinery.
///
/// `Permit` has a private field and no constructor outside this module, so
/// [`Recordable::into_record`] — which defuses/disposes a conserved resource
/// — can only ever be invoked by [`processes::record`] and the [`Consumer`]
/// impl on [`History`]. Downstream crates can *implement* [`Recordable`] for
/// their own resources but can never call it themselves: the only way to turn
/// a resource into a record is to hand it to a `History`.
pub struct Permit {
    _seal: (),
}

/// A conserved resource that a [`History`] may consume and record (R16).
///
/// Implementing `Recordable` for a type **declares "History may consume this
/// type"**: `into_record` is the implementor's sanctioned disposal site — the
/// implementor (who owns its type's seal) defuses or disposes of the resource
/// internally (the consumer's F-008 role, via its own sealed `defuse`) and
/// returns the value-level [`Event`] describing what was consumed. Leave the
/// event's `process` as [`UNATTRIBUTED`]: [`processes::record`] attributes it
/// to the caller's process name, and the generic [`Consumer`] path records it
/// unattributed.
///
/// The [`Permit`] argument cannot be constructed outside [`crate::history`],
/// so this method is callable only by `History`'s machinery — implementing
/// the trait never opens a disposal path that bypasses the record.
pub trait Recordable {
    /// Consumes the resource (defusing its tripwire inside its own privacy
    /// boundary) and returns its record. Only callable by [`History`]'s
    /// machinery — see [`Permit`].
    fn into_record(self, permit: Permit) -> Event;
}

/// Expended labour is recordable (R16): the history is the production-legal
/// boundary sink for [`Labour`] (F-035).
impl<const MS: u64> Recordable for Labour<MS> {
    fn into_record(self, _permit: Permit) -> Event {
        // Boundary exit: the labour leaves the model into the record;
        // defusing the tripwire here is the consumer playing its sanctioned
        // role (F-008).
        self.defuse();
        Event {
            process: UNATTRIBUTED,
            item: "Labour",
            magnitude: MS,
            unit: "person-milliseconds",
        }
    }
}

/// The generic-consumer path (R12): a `History` satisfies `Consumer` bounds,
/// so [`crate::boundary::send_to`] and processes generic over
/// `Consumer<Labour<MS>>` can account labour to it like to any boundary sink.
/// Events recorded this way carry the process name [`UNATTRIBUTED`] — the
/// trait's `consume` has nowhere to receive an attribution — so **prefer
/// [`processes::record`]**, which attributes the event to a named process
/// (R16: records must carry enough information to serve as an execution
/// record). `Next = Self`: the history is an unbounded sink, legal only at
/// the system boundary (R15); unlike other unbounded consumers it does not
/// discard its intake silently — it keeps the value-level record (F-029's
/// licence, refined by R16).
impl<const MS: u64> Consumer<Labour<MS>> for History {
    type Next = History;
    fn consume(mut self, item: Labour<MS>) -> History {
        self.entries
            .push(Entry::Event(item.into_record(Permit { _seal: () })));
        self
    }
}

/// The creation boundary for histories (R12, R16): the only production code
/// allowed to create one. Histories are created **at the system boundary,
/// one per concurrent branch** of a flow (R16) — a branch's record starts
/// empty, so creating one brings no conserved resources into existence.
pub mod boundary {
    use super::History;

    /// A fresh, empty history enters the model at the boundary (R16): create
    /// one per concurrent branch and merge them at the join
    /// ([`super::processes::merge`]).
    pub fn new_history() -> History {
        History {
            entries: Vec::new(),
            _seal: (),
        }
    }
}

/// Conserving processes over histories (R16). They consume resources into
/// records (via the sealed [`super::Permit`]), so they live inside the
/// resource family's module (F-031), not outside the module tree.
pub mod processes {
    use super::{Entry, History, Permit, Recordable};

    /// Records one conserved resource into a history, **attributed to a named
    /// process** (R16) — the primary recording API; prefer it over the
    /// generic [`super::Consumer`] path, whose events are unattributed. The
    /// resource is consumed: its implementor-provided
    /// [`Recordable::into_record`] defuses it inside its own privacy boundary
    /// (F-008), and only the value-level [`super::Event`] survives.
    pub fn record<R: Recordable>(history: History, process: &'static str, item: R) -> History {
        let mut event = item.into_record(Permit { _seal: () });
        event.process = process;
        let History {
            mut entries,
            _seal: (),
        } = history;
        entries.push(Entry::Event(event));
        History {
            entries,
            _seal: (),
        }
    }

    /// Merges two branch histories at a join point (R16). The merged record
    /// is a **partial order**: the two branches' entries are kept separate
    /// under one [`Entry::Join`] — deliberately, because concurrent branches
    /// have no defined interleaving, and separate-then-merged records state
    /// that truthfully (no event order is claimed between the branches; the
    /// order within each branch is preserved).
    pub fn merge(a: History, b: History) -> History {
        let History {
            entries: left,
            _seal: (),
        } = a;
        let History {
            entries: right,
            _seal: (),
        } = b;
        History {
            entries: vec![Entry::Join(left, right)],
            _seal: (),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::boundary::new_history;
    use super::processes::{merge, record};
    use super::{Entry, UNATTRIBUTED};
    use crate::boundary::send_to;
    use crate::common::boundary::new_person;
    use crate::common::processes::draw_time;

    /// `record` consumes the labour and appends one event attributed to the
    /// named process, with the item, magnitude and unit recorded (R16).
    #[test]
    fn record_appends_an_attributed_event() {
        let person = new_person::<5000>();
        let (labour, person) = draw_time::<2000, 3000, 5000>(person);
        let history = record(new_history(), "drill_holes", labour);
        assert_eq!(history.event_count(), 1);
        match history.entries() {
            [Entry::Event(e)] => {
                assert_eq!(e.process, "drill_holes");
                assert_eq!(e.item, "Labour");
                assert_eq!(e.magnitude, 2000);
                assert_eq!(e.unit, "person-milliseconds");
            }
            other => panic!("expected exactly one Event entry, got {other:?}"),
        }
        // The record and the part-spent person stay with the caller.
        let _execution_record = history;
        let _person_keeps_3000_ms = person;
    }

    /// The generic `Consumer` path (R12) works through `send_to` and records
    /// the event unattributed — the trait has nowhere to receive a process
    /// name; `processes::record` is the attributed API.
    #[test]
    fn consumer_path_records_unattributed() {
        let person = new_person::<1000>();
        let (labour, person) = draw_time::<500, 500, 1000>(person);
        let history = send_to(new_history(), labour);
        assert_eq!(history.event_count(), 1);
        match history.entries() {
            [Entry::Event(e)] => {
                assert_eq!(e.process, UNATTRIBUTED);
                assert_eq!(e.item, "Labour");
                assert_eq!(e.magnitude, 500);
            }
            other => panic!("expected exactly one Event entry, got {other:?}"),
        }
        let _execution_record = history;
        let _person = person;
    }

    /// `merge` produces a single `Join` holding the two branches' entries
    /// unchanged (the partial order, R16), and `event_count` recurses through
    /// nested joins.
    #[test]
    fn merge_joins_branches_and_event_count_recurses() {
        // Three "branches": two with one event each, one still empty.
        let person = new_person::<9000>();
        let (l1, person) = draw_time::<1000, 8000, 9000>(person);
        let (l2, person) = draw_time::<2000, 6000, 8000>(person);
        let h1 = record(new_history(), "cut", l1);
        let h2 = record(new_history(), "drill_holes", l2);
        let h3 = new_history();
        assert_eq!(h3.event_count(), 0);

        // First join, then a nested join: counting must recurse.
        let inner = merge(h1, h2);
        assert_eq!(inner.event_count(), 2);
        let outer = merge(inner, h3);
        assert_eq!(outer.event_count(), 2);

        match outer.entries() {
            [Entry::Join(left, right)] => {
                // The left branch is the inner join; the right one is empty.
                assert!(right.is_empty());
                match &left[..] {
                    [Entry::Join(a, b)] => {
                        assert!(
                            matches!(&a[..], [Entry::Event(e)] if e.process == "cut" && e.magnitude == 1000)
                        );
                        assert!(
                            matches!(&b[..], [Entry::Event(e)] if e.process == "drill_holes" && e.magnitude == 2000)
                        );
                    }
                    other => panic!("expected the inner Join, got {other:?}"),
                }
            }
            other => panic!("expected a single outer Join, got {other:?}"),
        }
        let _execution_record = outer;
        let _person = person;
    }
}
