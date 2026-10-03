#!/bin/sh
# check-solutions.sh — proves the solutions are real: it copies solutions/
# over the exercise files in a TEMPORARY COPY of this crate (your working
# files in tests/ are untouched) and runs `cargo test` there. Every exercise,
# solved, must pass.
#
# Use it as the answer key: diff any exercise against its solution with
#   diff tests/ex01_moves.rs solutions/ex01_moves.rs

set -eu
cd "$(dirname "$0")"
REPO_ROOT=$(cd .. && pwd)

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

mkdir -p "$TMP/learn/src" "$TMP/learn/tests"
cp Cargo.toml "$TMP/learn/Cargo.toml"
cp src/lib.rs "$TMP/learn/src/lib.rs"

# The solutions ARE the exercises, solved: they stand in for the test files.
for f in solutions/*.rs; do
    cp "$f" "$TMP/learn/tests/$(basename "$f")"
done

# The temp copy lives outside the repository, so point the path dependencies
# at the real model-core by absolute path.
sed -i.bak "s|\.\./model/model-core|$REPO_ROOT/model/model-core|g" "$TMP/learn/Cargo.toml"
rm -f "$TMP/learn/Cargo.toml.bak"

echo "==> running cargo test on the solved exercises (temp copy: $TMP/learn)"
cargo test --manifest-path "$TMP/learn/Cargo.toml"

echo ""
echo "check-solutions: all solutions pass."
