#!/usr/bin/env bash
#
# `mise run ci-sweep`: cap target/ size via cargo-sweep so the
# incremental cache cannot blow past ~30GB across long sessions. See
# `mise.toml [tasks.ci-sweep]` for the cap-vs-`--time` rationale.
#
# `cargo-sweep` is optional — fresh checkouts without it skip the
# prune with an install hint rather than failing `mise run ci`.

set -e

# `cargo sweep --help` succeeds whenever cargo can resolve the
# subcommand — this matches cargo's own `$CARGO_HOME/bin` lookup
# rather than relying on PATH (which may not include
# `~/.cargo/bin` in restricted shells / CI sandboxes even when the
# binary is installed).
if ! cargo sweep --help >/dev/null 2>&1; then
  echo "[ci-sweep] cargo-sweep not installed — skipping cache prune"
  echo "[ci-sweep] install with: cargo install cargo-sweep"
  exit 0
fi

cargo sweep --installed
cargo sweep --maxsize 30000
