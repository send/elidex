#!/usr/bin/env bash
#
# Asserts that `elidex-ecs`'s `fixtures` feature is enabled from
# `[dev-dependencies]` only (C0b, D-17 (4)). `fixtures` gates the ownerless node
# creators; a normal or build edge that turned it on would put them in a product
# build.
#
# The population comes from cargo, not from manifest text: the tree below carries
# normal and build edges (a build-dependency's features are on in a product
# build) and no dev edge, so any `elidex-ecs feature "fixtures"` line in it is a
# non-dev edge.
set -euo pipefail

# A cargo failure aborts here (set -e); it must never read as "no violation".
tree="$(cargo tree --workspace -e normal,build,features -i elidex-ecs)"

# Matched against the variable, not through a pipe: under pipefail, `grep -q`
# exiting on an early match SIGPIPEs the writer and the pipeline reads as
# "no match".
#
# Non-vacuity: the tree renders feature nodes (`elidex-ecs feature "default"`
# is always there), so a flag or format change cannot make the check vacuous.
if [[ "$tree" != *'elidex-ecs feature "default"'* ]]; then
  echo 'check-fixture-boundary: cargo tree printed no `elidex-ecs feature "default"` node; refusing to pass on output without feature nodes' >&2
  exit 1
fi

if [[ "$tree" == *'elidex-ecs feature "fixtures"'* ]]; then
  echo 'check-fixture-boundary: `fixtures` is enabled outside [dev-dependencies] (normal or build edge):' >&2
  /usr/bin/grep -F 'elidex-ecs feature "fixtures"' <<<"$tree" >&2 || true
  exit 1
fi

echo 'check-fixture-boundary: ok (`fixtures` is not enabled from any normal or build edge)'
