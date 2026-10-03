//! # learn — hands-on exercises for the type-system MBSE method
//!
//! Each file in `tests/` is one numbered exercise. Every exercise starts
//! **broken** — a comment at the top states the goal and the tutorial it
//! belongs to (`docs/tutorials/`), and `// TODO` marks the lines to fix.
//! The feedback loop is `cargo test`:
//!
//! ```text
//! cargo test --test ex01_moves        # run one exercise
//! cargo test                          # run them all (passes when you're done)
//! ```
//!
//! The exercises live in `tests/` because an integration-test file is **its
//! own crate**: each exercise is a tiny, self-contained model crate, with its
//! own sealed resources (defined by the `model-core` kernel macros, which
//! expand privately in the invoking crate), its own boundary, and its own
//! flow. See `learn/README.md` for the workflow and the exercise list, and
//! `docs/tutorials/` for the tutorials each exercise belongs to.
//!
//! This library target is intentionally empty: all the content is in
//! `tests/`.

#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![deny(let_underscore_drop)]
#![warn(missing_docs)]
