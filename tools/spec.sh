#!/bin/sh
# spec.sh — the specification gate (R22): validates model specifications
# against SPEC_TEMPLATE.md's machine conventions (A1–A13).
#
# Builds and runs the `speccheck` binary of tools/spec-gen (std-only Rust,
# outside the model workspace — it parses SPEC documents, never model
# source), checking balance arithmetic, waste-destination closure (§5 ↔ §4),
# canonical-identifier name closure (§3 ↔ §4/§5/§6), Satisfies-id and
# claim-segment closure (A11), draw/balance time agreement, flow-order
# closure and workspace-unique REQ ids (F-053). Errors carry §/line references and are phrased at the
# document: fix the specification, not the model (F-062). This error class
# fires BEFORE any code exists; without it an unbalanced spec line surfaces
# days later as an E0080 in the implemented crate.
#
# `spec.sh --check` is the CI mode (model/ci.sh step 8): it validates every
# case-studies/*/SPEC.md and exits nonzero on any error OR warning — what
# speccheck cannot verify fails the gate, like trace.sh's warnings (R22).
#
# Plain mode takes explicit spec files or directories (default:
# case-studies/), plus `--lenient` as a migration aid only: the pre-A1..A13
# EXP-15 fuzzy conventions, failing on errors alone.

set -eu
cd "$(dirname "$0")/.."

if [ "${1:-}" = "--check" ]; then
    exec cargo run --quiet --release --manifest-path tools/spec-gen/Cargo.toml --bin speccheck -- \
        case-studies
fi

if [ "$#" -eq 0 ]; then
    set -- case-studies
fi
exec cargo run --quiet --release --manifest-path tools/spec-gen/Cargo.toml --bin speccheck -- "$@"
