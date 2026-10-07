#!/bin/sh
# lint.sh — regenerates docs/analysis/ (the model linter's reports) from the
# model crates' source.
#
# Builds and runs the `modellint` binary of tools/diagram-gen (std-only Rust,
# outside the model workspace) over every model crate it knows about, writing
# docs/analysis/<crate>.md (one analysis report per crate) and
# docs/analysis/README.md (the workspace summary, severity ladder and check
# catalogue). Idempotent: running it twice produces identical files. Run it
# after any model change so the committed reports stay true to the source.
#
# `lint.sh --check` is the CI mode (model/ci.sh step 7): it regenerates the
# full report set into a temporary directory, diffs it against the committed
# docs/analysis/ (stale reports fail), and exits nonzero on any gate-fatal
# ERROR finding. ERROR-KNOWN, WARN and INFO findings never fail the gate.
# --check takes no crate arguments: staleness is a whole-directory property.

set -eu
cd "$(dirname "$0")/.."

if [ "${1:-}" = "--check" ]; then
    tmp="$(mktemp -d)"
    trap 'rm -rf "$tmp"' EXIT INT TERM
    rc=0
    cargo run --quiet --release --manifest-path tools/diagram-gen/Cargo.toml --bin modellint -- \
        "$PWD" --out "$tmp" --gate || rc=$?
    if ! diff -r "$tmp" docs/analysis >/dev/null 2>&1; then
        echo "FAIL: docs/analysis/ is stale — regenerate with ./tools/lint.sh and commit:" >&2
        diff -r "$tmp" docs/analysis >&2 || true
        exit 1
    fi
    if [ "$rc" -ne 0 ]; then
        echo "FAIL: gate-fatal ERROR findings — see docs/analysis/ (ERROR-KNOWN/WARN/INFO never fail)" >&2
    fi
    exit "$rc"
fi

cargo run --quiet --release --manifest-path tools/diagram-gen/Cargo.toml --bin modellint -- "$PWD" "$@"
