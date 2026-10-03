# learn/ — hands-on exercises

Rustlings-style exercises for the type-system MBSE method. Every file in
`tests/` is one exercise; every exercise **starts broken** and tells you, at
the top, what it teaches, which tutorial it belongs to
([`docs/tutorials/`](../docs/tutorials/)), and what to fix (the `// TODO`
markers). Your feedback loop is the compiler:

```sh
cd learn
cargo test --test ex01_moves    # work on one exercise
cargo test                      # the lot — passes when you have finished
```

Fix the `// TODO` lines until the test passes, then move to the next file.
Read the compiler errors — **the errors are the lesson**: each exercise is
built so that the broken state fails with exactly the error the matching
tutorial teaches you to read.

This crate deliberately sits **outside** the `model/` workspace: it is a
sandbox. Nothing you do here can break the real models or their CI gate, and
its `Cargo.toml` doubles as a worked example of how a downstream model crate
depends on `model-core` (the `test-support` feature under `[dev-dependencies]`
only — see `docs/modellers-guide.md` §2.2).

Each exercise file is an integration test, which means it is **its own
crate**: the model-core kernel macros expand privately inside it, so each file
is a complete, self-contained model — sealed resources, a boundary, processes
and a flow — in miniature.

## The exercises

| Exercise | Tutorial | It starts broken with | What it teaches |
|---|---|---|---|
| `ex01_moves` | 00 | `error[E0382]: use of moved value` | a resource is in one process at a time; take it from the previous process's output (R2) |
| `ex02_requirements` | 02 | `error[E0277]` phrased as REQ-901 | characteristics are traits, requirements are bounds, and the `Satisfies:` tag + `satisfies!` assertion keep each other honest (R6, R10) |
| `ex03_conservation` | 03 | `error[E0080]` — but **`cargo check` passes** | conservation asserts fire at monomorphization; your editor will not show them (R3, R15, F-001) |
| `ex04_boundary` | 04 | `error[E0277]` — the tin "cannot supply" | suppliers hold real objects; capacity is part of the type; fix the boundary, not the asker (R12) |
| `ex05_flow_order` | 04 | `error[E0308]: expected BoiledEgg, found RawEgg` | one type per state: a missing process step is a type mismatch (R9, F-023) |
| `ex06_capstone_toast` | 06 | three failures, in order: E0277 → E0080 → a tripwire panic | build a whole small model from a mini-spec; meet all three checking layers |

## Solutions

`solutions/` mirrors every exercise, solved (the fixed lines are marked
`// SOLVED`). Compare when you are stuck:

```sh
diff tests/ex01_moves.rs solutions/ex01_moves.rs
```

`./check-solutions.sh` proves the solutions: it copies them over the
exercises in a temporary copy of this crate (your files are untouched) and
runs `cargo test` there.

## If you get lost

Start from the tutorials — [`docs/tutorials/00-rust-on-ramp.md`](../docs/tutorials/00-rust-on-ramp.md)
assumes no Rust at all — and keep the error-translation table
(`docs/modellers-guide.md` §4) open next to the exercises. To reset an
exercise to its broken state, restore it from git:
`git checkout -- tests/<file>`.
