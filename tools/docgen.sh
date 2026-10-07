#!/bin/sh
# docgen.sh — regenerates docs/processes/ from the model crates' source.
#
# Builds and runs the `docgen` binary of tools/diagram-gen (std-only Rust,
# outside the model workspace) over every model crate it knows about,
# writing docs/processes/<crate>/<process>.md (one document per process,
# boundary function and composite flow), a per-crate index, and
# docs/processes/README.md. Idempotent: running it twice produces identical
# files. Run it after any model change so the committed documents stay true
# to the source.

set -eu
cd "$(dirname "$0")/.."

# Absolute blob base for Mermaid click directives (same convention as
# tools/diagrams.sh): GitHub renders Mermaid in a sandboxed iframe where
# relative urls 404, so clicks need the full blob url. Derived from the git
# remote; override by exporting DIAGRAM_REPO_BLOB_BASE.
if [ -z "${DIAGRAM_REPO_BLOB_BASE:-}" ]; then
  remote="$(git remote get-url origin 2>/dev/null || true)"
  branch="$(git symbolic-ref --short HEAD 2>/dev/null || echo main)"
  case "$remote" in
    git@github.com:*) repo="${remote#git@github.com:}" ;;
    https://github.com/*) repo="${remote#https://github.com/}" ;;
    *) repo="" ;;
  esac
  repo="${repo%.git}"
  if [ -n "$repo" ]; then
    DIAGRAM_REPO_BLOB_BASE="https://github.com/$repo/blob/$branch"
  fi
fi
export DIAGRAM_REPO_BLOB_BASE="${DIAGRAM_REPO_BLOB_BASE:-}"

cargo run --quiet --release --manifest-path tools/diagram-gen/Cargo.toml --bin docgen -- "$PWD" "$@"
