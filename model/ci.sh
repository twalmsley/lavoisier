#!/bin/sh
# ci.sh — the CI gate for the model workspace (R4; FINDINGS.md F-001, F-004,
# F-007). Run from anywhere; it cd's to the workspace root. Any failing step
# fails the gate (set -e).
#
# Why each step exists:
#   1. Full `cargo build`: conservation violations are E0080s at
#      MONOMORPHIZATION (F-001) — `cargo check` alone would miss them, so the
#      gate is never allowed to rest on `cargo check`.
#   2. `cargo test`: unit + integration tests, trybuild cases, and the rustdoc
#      `compile_fail` doc-tests that are the only automated regressions for
#      conservation/overdraw violations (F-003).
#   3. clippy under -D warnings PLUS the explicit restriction lints: the
#      conservation lint set is only sound as a whole (F-007). The
#      disallowed-methods list lives in clippy.toml at the workspace root.
#   4. A PLAIN no-features `cargo build` of every member's production targets:
#      during `cargo test`, feature unification compiles production sources
#      against the test-support-featured library (F-004), so only this build
#      proves production code cannot reach fixture constructors.
#   5. trace.sh: the R10 traceability report; any warning exits nonzero. (With
#      no REQ-NNN definitions in the workspace it reports that and passes —
#      model-core is infrastructure.)
#   6. A grep proving the `test-support` feature never appears under any
#      [dependencies]-like section — [dev-dependencies] only (R1, F-004).
#   7. The model-analysis gate (PLAN step 11, candidate R21): tools/lint.sh
#      --check regenerates the linter's reports into a temp dir, diffs them
#      against the committed docs/analysis/ (stale reports fail), and fails
#      on any gate-fatal ERROR finding. ERROR-KNOWN (maintainer-acknowledged
#      true positives), WARN and INFO findings never fail the gate.

set -eu
cd "$(dirname "$0")"

echo "==> [1/7] cargo build --workspace (full build: post-monomorphization conservation errors fire here, F-001)"
cargo build --workspace

echo "==> [2/7] cargo test --workspace (tests, trybuild, and compile_fail doc-test regressions)"
cargo test --workspace

echo "==> [3/7] cargo clippy --all-targets, -D warnings + R1 restriction lints (F-007)"
cargo clippy --workspace --all-targets -- \
    -D warnings \
    -D clippy::mem_forget \
    -D clippy::let_underscore_must_use

echo "==> [4/7] plain no-features cargo build of every member's production targets (test-support boundary proof, F-004)"
# Deliberately a BUILD, not a check (R4: never gate on `cargo check` alone),
# and deliberately without features: this is the only step that can prove a
# production target does not call test-support fixtures.
cargo build --workspace

echo "==> [5/7] requirements traceability gate (R10): ./trace.sh"
./trace.sh

echo "==> [6/7] test-support must never appear under [dependencies] (R1, F-004)"
bad=$(find . -path '*/target' -prune -o -type f -name Cargo.toml -print | sort | while read -r f; do
    awk -v file="$f" '
        /^[ \t]*\[/ {
            in_dep = 0
            # Any dependencies-like section that is not dev-dependencies:
            # [dependencies], [build-dependencies], [workspace.dependencies],
            # [target.<cfg>.dependencies], and their dotted/sub-table forms.
            if ($0 ~ /dependencies[]\.]/ && $0 !~ /dev-dependencies/) in_dep = 1
        }
        in_dep && /test-support/ { print file ":" FNR ": " $0 }
    ' "$f"
done)
if [ -n "$bad" ]; then
    echo "FAIL: the test-support feature may only be enabled under [dev-dependencies] (R1, F-004):"
    echo "$bad"
    exit 1
fi
echo "    OK: test-support appears under [dev-dependencies] only."

echo "==> [7/7] model-analysis gate (candidate R21): ../tools/lint.sh --check"
../tools/lint.sh --check

echo ""
echo "CI gate passed: build, test, clippy, plain production build, traceability, feature placement, model analysis."
