//! Exercise 01 — one resource, one process at a time
//! (belongs to Tutorial 00, "The Rust on-ramp", §"Values move".)
//!
//! GOAL: a resource can be in use by only one process at a time (R2). A
//! process takes the person by value and **returns them**; the next process
//! must be given the person who came back, not the original binding — the
//! original was moved. Fix the line marked `// TODO` until
//! `cargo test --test ex01_moves` passes.
//!
//! What the broken state teaches: `error[E0382]: use of moved value: `person``
//! is the compiler saying "this person is already busy in another process".
//! Ignore the compiler's "consider cloning" style suggestions — resources are
//! never cloned or borrowed (R2, F-007). The fix is always to take the
//! resource from the previous process's output.

#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![deny(let_underscore_drop)]

use model_core::common::Person;
use model_core::common::boundary::new_person;
use model_core::common::processes::draw_time;
use model_core::history::boundary::new_history;
use model_core::history::processes::record;

/// One person fills the kettle (2 000 ms) and then loads the pot (3 000 ms).
/// Both draws come out of the same 10 000 ms budget, and each draw's labour
/// is recorded into the History (R16).
#[test]
fn the_same_person_cannot_do_two_jobs_at_once() {
    let history = new_history();
    let person = new_person::<10_000>();

    // Job 1 — fill the kettle: the person is moved in, and a person with
    // 8 000 ms left comes back out.
    let (labour, person_back) = draw_time::<2_000, 8_000, 10_000>(person);
    let history = record(history, "fill_kettle", labour);

    // Job 2 — load the pot.
    // SOLVED: this line claims the ORIGINAL `person` again — but that binding
    // was moved into job 1. Use the person who came back (`person_back`),
    // and restate their balance: they have 8 000 ms now, and this 3 000 ms
    // draw leaves 5 000.
    let (labour, person) = draw_time::<3_000, 5_000, 8_000>(person_back);
    let history = record(history, "load_pot", labour);

    // Everything accounted at the end: the person is back with 5 000 ms, and
    // the History holds both draws, attributed to their processes.
    let _person_back_with_5000_ms: Person<5_000> = person;
    assert_eq!(history.event_count(), 2);
    match history.entries() {
        [
            model_core::history::Entry::Event(e1),
            model_core::history::Entry::Event(e2),
        ] => {
            assert_eq!((e1.process, e1.magnitude), ("fill_kettle", 2_000));
            assert_eq!((e2.process, e2.magnitude), ("load_pot", 3_000));
        }
        other => panic!("expected exactly two recorded draws, got {other:?}"),
    }
    let _execution_record = history;
}
