#!/usr/bin/env bash
#
# `mise run doc` fast-path: rustdoc only the workspace crates whose
# source files differ from `git status`, falling back to the full
# workspace `cargo doc` when nothing matches (fresh checkout, README
# edits, etc.). See `mise.toml [tasks.doc]` for the cost / trade-off
# rationale and `mise run doc-full` for the always-comprehensive
# variant that matches GitHub Actions.
#
# POSIX-shell pipeline (set -e + pipelines + awk / grep): invoked
# explicitly via bash so Windows contributors get a clear
# "bash: command not found" message instead of a confusing
# partial-execute failure.

set -e

CHANGED_CRATES=$(git status --porcelain \
  | awk '{print $NF}' \
  | grep -oE '^crates/[^/]+/[^/]+' \
  | sort -u || true)

if [ -z "$CHANGED_CRATES" ]; then
  echo "[doc] no changed crates — running full workspace doc"
  exec cargo doc --workspace --no-deps --all-features
fi

PKGS=""
for crate_dir in $CHANGED_CRATES; do
  if [ -f "$crate_dir/Cargo.toml" ]; then
    name=$(awk -F'"' '/^name = / {print $2; exit}' "$crate_dir/Cargo.toml")
    PKGS="$PKGS -p $name"
  fi
done

if [ -z "$PKGS" ]; then
  echo "[doc] no workspace members in changed paths — running full workspace doc"
  exec cargo doc --workspace --no-deps --all-features
fi

echo "[doc] documenting changed crates:$PKGS"
exec cargo doc --no-deps --all-features $PKGS
