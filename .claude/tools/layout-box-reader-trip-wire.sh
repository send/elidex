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
# LINE comment / docstring — a `LayoutBox` mention in prose is not a reader. Matches
# ONLY `//` (covers `///`/`//!`), exactly the sibling `.claude/tools/*-trip-wire.sh`
# idiom: a bare-`*` match would drop a `*deref = LayoutBox{}` reader line (the
# dangerous under-inclusion direction); over-including a rare `/* */` block-comment
# line is the safe direction for an exhaustiveness gate.
strip_comments() { sed -E '/^[^:]*:[0-9]+:[[:space:]]*\/\//d'; }

# The wholly-test path convention — the SINGLE definition, shared by wires #1 and #3.
# `$1` is the terminator that ends a path in the caller's input: `:` for `git grep -n`
# lines (`path:line:content`), `$` for `git grep -l` bare paths. A test DIRECTORY
# segment needs no terminator; the four test-convention BASENAMES do.
# ⚠ One definition on purpose: the wires previously carried hand-synced copies of this
# regex which had already drifted (`[^/:]*` here vs `[^/]*` there). This predicate is
# what scopes the whole exhaustiveness claim, so a silent divergence between the wires
# is the same false-exhaustiveness hazard the paragraph on `live_readers` describes.
# `[^/:]*` is correct for both inputs — a tracked path never contains `:`.
test_path_re() {
  printf '(/tests?/|/tests\\.rs%s|/test_[^/:]*\\.rs%s|/tests_[^/:]*\\.rs%s|_tests\\.rs%s)' \
    "$1" "$1" "$1" "$1"
}

# The live reader set as `path<TAB>content`, line-number-insensitive (so MOVING a
# reader doesn't churn the allowlist) but content-sensitive (so EDITING one forces
# re-classification). Strips comment lines and leading indentation; excludes
# WHOLLY-TEST files only — a `/tests/` dir segment, or a test-convention BASENAME
# (`tests.rs` / `test_*.rs` / `tests_*.rs` / `*_tests.rs`). ⚠ This is a
# SEGMENT/BASENAME match, not a `test`-substring match: a substring match wrongly
# drops PRODUCTION files like `hit_test.rs` (`*_test.rs` singular, a real
# `get::<&LayoutBox>` reader), leaving a delete-enabling gate with a false
# exhaustiveness claim (the exact grep-hole design memo §4 forbids). Only the plural
# `*_tests.rs` suffix is a test convention; `*_test.rs` singular stays included.
# Inline `#[cfg(test)]` module lines in production-named files are kept and classified
# `test` in the allowlist (safe: no coverage gap, bounded noise). `|| true` keeps a
# fully-filtered (impossible-while-meaningful) result from aborting under pipefail.
live_readers() {
  cd "$ROOT"
  { git grep -nwE 'LayoutBox|BoxModel' -- 'crates/**/*.rs' || true; } \
    | strip_comments \
    | { grep -vE "$(test_path_re ':')" || true; } \
    | sed -E 's/^([^:]+):[0-9]+:[[:space:]]*(.*)$/\1\t\2/' \
    | sed -E 's/[[:space:]]+$//' \
    | sort -u
}

# The committed allowlist's `path<TAB>content` set (columns 2+; column 1 =
# classification: producer|seam|pending-migration:<slice>|type-def|import|test). `cut -f2-`
# (not `-f2,3`) keeps ALL content columns so a reader line containing a literal TAB
# round-trips identically to `live_readers` (no permanent churn).
# NB: the gate keys on unique `(path, content)` — identical-content reader lines in
# ONE file (e.g. `form.rs`'s eight `lb: &LayoutBox` helpers) collapse to one entry.
# This fires on any NOVEL-content reader and, at C-4, on any surviving
# `pending-migration` entry (full migration removes it); per-SITE precision (the ×8
# tally) is the audit doc's job. See the audit "Counting basis" note.
committed_readers() {
  grep -vE '^[[:space:]]*(#|$)' "$ALLOWLIST" | cut -f2- | sort -u
}

# The classification vocabulary — the SINGLE machine-readable definition. Wire #4 validates
# column 1 against it, which is what makes the vocabulary an enforced contract instead of a
# comment three files restate (the .tsv header, the `--regenerate` emitter, and the audit
# doc's legend had all drifted apart from each other AND from the data). It is also what
# stops `--regenerate`'s `UNCLASSIFIED` placeholder from riding into a green run: wire #1's
# own FAIL message prescribes `--regenerate`, so without this an author following the tool's
# instructions after a refactor would silently absorb a genuinely-new reader as unclassified
# and turn the gate green — a structural gate degraded to a review convention.
CLASSES='producer|seam|type-def|import|test|pending-migration:C-3[b-e]'

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
    echo "#   classification ∈ {producer, seam, pending-migration:<slice>, type-def, import, test}"
    echo "#   Machine SoT = this script's \$CLASSES (enforced by wire #4). Prose definition ="
    echo "#   the audit doc's '## Classification legend' section. A pending-migration row"
    echo "#   names its owning slice, e.g. pending-migration:C-3e."
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
#
# ⚠ Word boundaries come from `git grep -w`, NOT from `\b`. `git grep -E` is POSIX ERE and
# silently accepts-but-never-matches `\b` — the original pattern used it and this wire was
# therefore DEAD from the day it shipped, printing OK unconditionally. Since wire #2 is the
# entire reason the memo-§4 compiler check was downgraded to grep (see the header), a dead
# wire #2 means the exhaustiveness claim rests on nothing.
# ⚠ Both patterns must END on a word boundary or `-w` rejects the hit: an earlier draft
# closed BAN_ALIAS with `as[[:space:]]+[A-Za-z_]`, whose match ends mid-identifier (`… as L`
# inside `LB`), and `-w` then discarded it — the positive control below caught that too.
BAN_ALIAS='(LayoutBox|BoxModel)[[:space:]]+as'
BAN_TYPEALIAS='type[[:space:]]+[A-Za-z_][A-Za-z0-9_]*[[:space:]]*=[^;]*(LayoutBox|BoxModel)'

# Positive control — the STRUCTURAL fix for the class, not just the regex. A ban wire that
# cannot match is indistinguishable from a clean tree, so trusting its verdict requires first
# proving it fires. Each banned shape is matched against a synthetic violation using the SAME
# engine and flags as the real scan (`git grep --no-index -nwE`); a control that fails to hit
# means the pattern is dead and the wire aborts instead of reporting a false all-clear.
ban_control() { # $1 = pattern, $2 = sample line that MUST match, $3 = shape label
  # Run from INSIDE the scratch dir with a relative pathspec: `git grep --no-index` refuses
  # an absolute path pointing outside the enclosing repository, which would make every
  # control silently "fail" for the wrong reason and mask a pattern that is actually fine.
  local dir rc; dir="$(mktemp -d)"
  printf '%s\n' "$2" > "$dir/control.rs"
  ( cd "$dir" && git grep --no-index -qnwE "$1" -- control.rs ) 2>/dev/null
  rc=$?
  rm -rf "$dir"
  if [ "$rc" -ne 0 ]; then
    red "FAIL: wire #2 self-test — the '$3' ban pattern matched nothing on a known violation."
    red "      The wire cannot fire, so its OK verdict would be meaningless. Fix the pattern"
    red "      (POSIX ERE has no \\b; and with -w the match must END on a word boundary)."
    return 1
  fi
  return 0
}
control_ok=0
ban_control "$BAN_ALIAS" 'use elidex_plugin::LayoutBox as LB;' 'import/re-export alias' || control_ok=1
ban_control "$BAN_TYPEALIAS" 'type MyBox = elidex_plugin::LayoutBox;' 'type alias' || control_ok=1
if [ "$control_ok" -ne 0 ]; then
  fail=1
else
  ban_hits="$(cd "$ROOT" && git grep -nwE "$BAN_ALIAS|$BAN_TYPEALIAS" -- 'crates/**/*.rs' | strip_comments || true)"
  if [ -n "$ban_hits" ]; then
    red "FAIL: a LayoutBox/BoxModel alias / type-alias was introduced — it drops the token at use-sites and defeats the grep. Use the type directly:"
    printf '%s\n' "$ban_hits" | sed 's/^/  /'
    fail=1
  else
    green "OK (no aliases; both ban patterns verified live against a positive control)"
  fi
fi

echo "wire #3: no unreviewed macro_rules! in a reader-token file (token-hiding-macro guard)"
reader_files="$(cd "$ROOT" && git grep -lwE 'LayoutBox|BoxModel' -- 'crates/**/*.rs' | grep -vE "$(test_path_re '$')" || true)"
macro_hits=""
if [ -n "$reader_files" ]; then
  # shellcheck disable=SC2086
  # strip_comments (like wires #1/#2) so a prose mention of `macro_rules!` in a
  # doc-comment is not a false positive.
  macro_hits="$(cd "$ROOT" && grep -nE 'macro_rules!' $reader_files 2>/dev/null | strip_comments | grep -vE "macro_rules![[:space:]]+($ALLOWED_MACROS)\b" || true)"
fi
if [ -n "$macro_hits" ]; then
  red "FAIL: a new macro_rules! landed in a LayoutBox/BoxModel-reading file. Verify it does NOT expand to a token-less geometry read, then add it to ALLOWED_MACROS (or escalate D4 to a dylint HIR lint):"
  printf '%s\n' "$macro_hits" | sed 's/^/  /'
  fail=1
else
  green "OK (only the verified non-hiding macros)"
fi

echo "wire #4: every allowlist row carries a classification from the declared vocabulary"
# Without this the classification column is never read by anything (`committed_readers`
# does `cut -f2-`), so it is documentation the machine ignores — including
# `--regenerate`'s `UNCLASSIFIED`, the one value that means "nobody has triaged this yet".
bad_class="$(grep -vE '^[[:space:]]*(#|$)' "$ALLOWLIST" | grep -vE "^($CLASSES)"$'\t' || true)"
if [ -n "$bad_class" ]; then
  red "FAIL: allowlist row(s) whose classification is outside the declared vocabulary"
  red "      ($CLASSES). An UNCLASSIFIED row means --regenerate found a reader nobody has"
  red "      triaged — classify it against docs/audits/2026-07-layoutbox-reader-inventory.md:"
  printf '%s\n' "$bad_class" | sed 's/^/  /'
  fail=1
else
  green "OK ($(committed_readers | wc -l | tr -d ' ') rows, every classification in vocabulary)"
fi

if [ "$fail" -ne 0 ]; then
  red ""; red "LayoutBox/BoxModel reader-audit trip-wire FAILED"
  exit 1
fi
green ""; green "✓ all clear"
