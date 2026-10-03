# Tutorials — model your first system

A numbered sequence, each one sitting long, each ending with a runnable
checkpoint. The hands-on counterpart is the exercises crate
[`learn/`](../../learn/) — most checkpoints *are* one of its exercises, run
with `cargo test`. The destination is the
[modeller's guide](../modellers-guide.md): after the series you should be
able to use it, and read CS-1's code, unaided.

| # | Tutorial | You come out able to | Exercise |
|---|---|---|---|
| 00 | [The Rust on-ramp](00-rust-on-ramp.md) | read moves, structs, traits, bounds and const generics as conservation, resources, characteristics, requirements and quantities | `ex01_moves` |
| 01 | [Setting up a model crate](01-setting-up-a-model-crate.md) | explain every line of a model crate's manifest and every step of `ci.sh` | — (run `./ci.sh`) |
| 02 | [Resources and requirements](02-resources-and-requirements.md) | define sealed resource states with the kernel macros, and requirements as compile-checked traits | `ex02_requirements` |
| 03 | [Processes and conservation](03-processes-and-conservation.md) | write conserving processes with compile-time balance asserts — and know when those asserts do and don't fire | `ex03_conservation` |
| 04 | [The boundary, flows and History](04-boundary-flows-and-history.md) | bring resources in through suppliers and draws, out through consumers and the History, and compose flows | `ex04_boundary`, `ex05_flow_order` |
| 05 | [Reading the errors](05-reading-the-errors.md) | translate every error family back into model language | — (read the pinned `.stderr` catalogue) |
| 06 | [Capstone: model making toast](06-capstone-toast.md) | build a small model from a mini-spec, through all three checking layers | `ex06_capstone_toast` |

Prerequisites: a systems-engineering head, stable Rust 1.98.1, no prior Rust.
Start at [00](00-rust-on-ramp.md); if you already write Rust, skim 00 and
start at [01](01-setting-up-a-model-crate.md).
