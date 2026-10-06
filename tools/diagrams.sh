#!/bin/sh
# diagrams.sh — regenerates docs/diagrams/ from the model crates' source.
#
# Builds and runs tools/diagram-gen (std-only Rust, outside the model
# workspace) over every model crate it knows about, writing
# docs/diagrams/<crate>/{context,top-level,detailed}.md and
# docs/diagrams/README.md. Idempotent: running it twice produces identical
# files. Run it after any model change so the committed diagrams stay true
# to the source.

set -eu
cd "$(dirname "$0")/.."

cargo run --quiet --release --manifest-path tools/diagram-gen/Cargo.toml -- "$PWD" "$@"
