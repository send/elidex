#!/usr/bin/env bash
# Layering trip-wire: the terminal-Z LayoutBox/BoxModel READER AUDIT gate
# (plan-memo docs/plans/2026-07-terminal-z-c3a-impl-plan.md §3 = D4).
#
# The terminal-Z C-3 program migrates every non-producer geometry reader off the
# raw `elidex_plugin::LayoutBox` component onto the C-3a `box_fragments` seam, so
# C-4 can delete `LayoutBox`. That migration's correctness rests on an EXHAUSTIVE,
# CURRENT inventory of every `LayoutBox` reader — the audit
# (docs/audits/2026-07-layoutbox-reader-inventory.md is the human record; the
# committed allowlist beside this script is the machine-checked set).
#
# WHY grep is sufficient HERE (the design memo §4 said "only the compiler can
# prove exhaustiveness — a grep cannot", and routed the method to the C-3a impl
# plan-review, which chose grep + a NAME-INTRODUCTION BAN):
#   * A `LayoutBox` / `BoxModel` reference the grep MISSES only if its token is
#     absent at the use-site. The three ways that happens —
#       (a) import aliases     `use …LayoutBox as X;`   → `X` has no token
#       (b) type aliases       `type X = …LayoutBox;`   → `X` has no token
#       (c) aliased re-exports  `pub use … as X;`       → `X` has no token
#     — are all NAME INTRODUCTIONS this wire BANS (wire #2), so with none in the
#     tree every reference carries the token.
#   * Bare trait bounds `<T: BoxModel>` / `where T: BoxModel` are NOT `dyn`/`impl`,
#     so the gate greps bare `-w BoxModel` (not `dyn|impl BoxModel`) — accepting
#     allowlist noise (the trait def + impls) in exchange for catching every
#     generic-bound reader on the stable toolchain.
#   * The one residual a compiler (dylint) would additionally catch is a macro
#     that expands to a TOKEN-LESS read. Verifiably absent today; wire #3 guards
#     against a new `macro_rules!` in a reader-token file landing unreviewed. If a
#     genuinely token-hiding macro ever lands, escalate D4 to a dylint HIR lint.
#
# Usage:
#   layout-box-reader-trip-wire.sh              # CHECK (CI / pre-push); exits non-zero on drift
#   layout-box-reader-trip-wire.sh --regenerate # rewrite the allowlist from the live tree
#                                               #   (keeps existing classification columns by path+content)
#
# Run from anywhere. Exits non-zero on any violation.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
ALLOWLIST="$ROOT/.claude/tools/layout-box-reader-allowlist.tsv"

red()   { printf '\033[31m%s\033[0m\n' "$*"; }
green() { printf '\033[32m%s\033[0m\n' "$*"; }

# Allowed `macro_rules!` in reader-token files (wire #3). Each was verified NOT to
# expand to a token-less geometry read at C-3a authoring time (both take their body
# as `$body:expr` at the call site, or carry only doc-comment tokens). A NEW macro
# in a reader-token file must be added here only after the same verification.
ALLOWED_MACROS='impl_layout_handler|impl_string_map'

# Strip grep `-n` output lines whose content (after the `path:line:` prefix) is a
# comment / docstring — a `LayoutBox` mention in prose is not a reader. Same idiom
# as the sibling `.claude/tools/*-trip-wire.sh`.
strip_comments() { sed -E '/^[^:]*:[0-9]+:[[:space:]]*(\/\/|\*|\/\*)/d'; }

# The live reader set as `path<TAB>content`, line-number-insensitive (so MOVING a
# reader doesn't churn the allowlist) but content-sensitive (so EDITING one forces
# re-classification). Strips comment lines and leading indentation; excludes
# WHOLLY-TEST files only — a `/tests/` dir segment, or a test-convention BASENAME
# (`tests.rs` / `test_*.rs` / `tests_*.rs`). ⚠ This is a SEGMENT/BASENAME match, not
# a `test`-substring match: a substring match wrongly drops PRODUCTION files like
# `hit_test.rs` (a real `get::<&LayoutBox>` reader), leaving a delete-enabling gate
# with a false exhaustiveness claim (the exact grep-hole design memo §4 forbids).
# Inline `#[cfg(test)]` module lines in production-named files are kept and classified
# `test` in the allowlist (safe: no coverage gap, bounded noise).
live_readers() {
  cd "$ROOT"
  git grep -nwE 'LayoutBox|BoxModel' -- 'crates/**/*.rs' \
    | strip_comments \
    | grep -vE '(/tests?/|/tests\.rs:|/test_[^/:]*\.rs:|/tests_[^/:]*\.rs:)' \
    | sed -E 's/^([^:]+):[0-9]+:[[:space:]]*(.*)$/\1\t\2/' \
    | sed -E 's/[[:space:]]+$//' \
    | sort -u
}

# The committed allowlist's `path<TAB>content` set (columns 2-3; column 1 =
# classification: producer|seam|pending-migration|type-def|import|test).
# NB: the gate keys on unique `(path, content)` — identical-content reader lines in
# ONE file (e.g. `form.rs`'s eight `lb: &LayoutBox` helpers) collapse to one entry.
# This fires on any NOVEL-content reader and, at C-4, on any surviving
# `pending-migration` entry (full migration removes it); per-SITE precision (the ×8
# tally) is the audit doc's job. See the audit "Counting basis" note.
committed_readers() {
  grep -vE '^[[:space:]]*(#|$)' "$ALLOWLIST" | cut -f2,3 | sort -u
}

if [ "${1:-}" = "--regenerate" ]; then
  # Preserve any existing classification for a (path, content) still present;
  # new lines get `UNCLASSIFIED` for the author to triage against the audit doc.
  declare -A CLASS
  if [ -f "$ALLOWLIST" ]; then
    while IFS=$'\t' read -r cls path content; do
      [ -z "${cls:-}" ] && continue
      case "$cls" in \#*) continue;; esac
      CLASS["$path"$'\t'"$content"]="$cls"
    done < <(grep -vE '^[[:space:]]*(#|$)' "$ALLOWLIST" || true)
  fi
  {
    echo "# terminal-Z LayoutBox/BoxModel reader allowlist — machine-checked sibling of"
    echo "# docs/audits/2026-07-layoutbox-reader-inventory.md (the human record)."
    echo "# Format: <classification>\t<path>\t<content>"
    echo "#   classification ∈ {producer, seam, pending-migration, type-def}"
    echo "# Regenerate: .claude/tools/layout-box-reader-trip-wire.sh --regenerate"
    while IFS=$'\t' read -r path content; do
      key="$path"$'\t'"$content"
      printf '%s\t%s\t%s\n' "${CLASS[$key]:-UNCLASSIFIED}" "$path" "$content"
    done < <(live_readers)
  } > "$ALLOWLIST"
  green "regenerated $ALLOWLIST ($(committed_readers | wc -l | tr -d ' ') readers)"
  exit 0
fi

test -f "$ALLOWLIST" || { red "FAIL: allowlist missing at $ALLOWLIST (run --regenerate)"; exit 1; }

fail=0

echo "wire #1: LayoutBox/BoxModel reader inventory is exhaustive + current (allowlist diff)"
live="$(live_readers)"
committed="$(committed_readers)"
added="$(comm -23 <(printf '%s\n' "$live") <(printf '%s\n' "$committed") || true)"
removed="$(comm -13 <(printf '%s\n' "$live") <(printf '%s\n' "$committed") || true)"
if [ -n "$added" ]; then
  red "FAIL: NEW unclassified LayoutBox/BoxModel reader(s) — add to the allowlist + the audit inventory (classify by the 8 axes):"
  printf '%s\n' "$added" | sed 's/^/  + /'
  fail=1
fi
if [ -n "$removed" ]; then
  red "FAIL: allowlist entr(y/ies) no longer present (reader migrated/moved) — sync the allowlist (--regenerate) + the audit inventory:"
  printf '%s\n' "$removed" | sed 's/^/  - /'
  fail=1
fi
[ "$fail" -eq 0 ] && green "OK ($(printf '%s\n' "$live" | grep -c . || true) readers, all classified)"

echo "wire #2: no LayoutBox/BoxModel NAME-INTRODUCTION (keeps the grep exhaustive)"
# Bans: `X as` aliases, `type X = …Token`, aliased re-exports. The canonical
# `pub use …{…, LayoutBox, …}` re-export (no `as`) keeps the token, so it is fine.
ban_hits="$(cd "$ROOT" && git grep -nE '\b(LayoutBox|BoxModel)[[:space:]]+as[[:space:]]|type[[:space:]]+[A-Za-z_][A-Za-z0-9_]*[[:space:]]*=[^;]*\b(LayoutBox|BoxModel)\b' -- 'crates/**/*.rs' | strip_comments || true)"
if [ -n "$ban_hits" ]; then
  red "FAIL: a LayoutBox/BoxModel alias / type-alias was introduced — it drops the token at use-sites and defeats the grep. Use the type directly:"
  printf '%s\n' "$ban_hits" | sed 's/^/  /'
  fail=1
else
  green "OK (no aliases)"
fi

echo "wire #3: no unreviewed macro_rules! in a reader-token file (token-hiding-macro guard)"
reader_files="$(cd "$ROOT" && git grep -lwE 'LayoutBox|BoxModel' -- 'crates/**/*.rs' | grep -vE '(/tests?/|/tests\.rs$|/test_[^/]*\.rs$|/tests_[^/]*\.rs$)' || true)"
macro_hits=""
if [ -n "$reader_files" ]; then
  # shellcheck disable=SC2086
  macro_hits="$(cd "$ROOT" && grep -nE 'macro_rules!' $reader_files 2>/dev/null | grep -vE "macro_rules![[:space:]]+($ALLOWED_MACROS)\b" || true)"
fi
if [ -n "$macro_hits" ]; then
  red "FAIL: a new macro_rules! landed in a LayoutBox/BoxModel-reading file. Verify it does NOT expand to a token-less geometry read, then add it to ALLOWED_MACROS (or escalate D4 to a dylint HIR lint):"
  printf '%s\n' "$macro_hits" | sed 's/^/  /'
  fail=1
else
  green "OK (only the verified non-hiding macros)"
fi

if [ "$fail" -ne 0 ]; then
  red ""; red "LayoutBox/BoxModel reader-audit trip-wire FAILED"
  exit 1
fi
green ""; green "✓ all clear"
