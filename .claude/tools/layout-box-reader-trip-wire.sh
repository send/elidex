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
#     absent at the use-site. FOUR mechanisms do that:
#       (a) import aliases     `use …LayoutBox as X;`   → `X` has no token
#       (b) type aliases       `type X = …LayoutBox;`   → `X` has no token
#       (c) aliased re-exports  `pub use … as X;`       → `X` has no token
#       (d) a `LayoutBox`-TYPED BINDING (struct field or fn param) — the
#           DECLARATION carries the token, but every `v.the_binding.content…` read
#           through it does not. Live: `elidex-render` `builder/mod.rs:367-372` (via
#           `LayoutOutcome.layout_box`) and `builder/inline.rs:268…456` — six reads
#           via `InlineRunContext.lb`, declared `pub(crate) lb: &'a LayoutBox` at
#           `inline.rs:73`.
#     (a)-(c) are NAME INTRODUCTIONS wire #2 BANS, so they cannot occur.
#     (d) CANNOT be banned — such bindings legitimately exist — and this gate does
#     **NOT** bound it. What wire #1 does give is weaker but real: the *declaration*
#     line carries the token, so a NEW binding forces an allowlist row and a
#     classification. What is not machine-checked is the set of *reads through* an
#     already-allowlisted binding; the audit enumerates those by hand and slot
#     `#11-layoutbox-field-typed-reader-coverage` owns closing the gap.
#     ⚠ Do NOT restate this as "the family is bounded". A revision of this script
#     shipped a wire #5 claiming exactly that; it was withdrawn when a focused
#     re-check showed (i) its regex required the type to start right after `name:`, so
#     `InlineRunContext.lb` (`&'a LayoutBox`, listed above) went unseen, and (ii) a
#     new field in an already-listed FILE passed all
#     wires green. A gate that overclaims its bound is worse than an honest slot,
#     because C-4's delete decision is taken against the claim.
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
#   layout-box-reader-trip-wire.sh              # CHECK (local `mise run ci` / pre-push)
#                                               #   exits non-zero on drift
# ⚠ LOCAL ONLY. `.github/workflows/ci.yml` runs cargo fmt/clippy/nextest/doc/deny — it
# invokes no `mise` task, so this gate NEVER runs on a PR or on main. Its verdict is
# therefore enforced by the pre-push habit, not by CI, and a lane that skips /pre-push
# drifts the allowlist silently. Slot `#11-layoutbox-trip-wire-not-in-ci`.
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
# ⚠ The `test_*.rs` PREFIX is deliberately NOT excluded — the same reasoning given below
# for keeping singular `*_test.rs`. A `test_`-prefixed basename does not imply `cfg(test)`:
# `elidex-js/src/vm/test_helpers.rs` is `#[cfg(feature = "engine")] #[doc(hidden)] pub mod`,
# i.e. PRODUCTION-compiled shipped code, and it WRITES `LayoutBox` (a `remove_one`, an
# `insert`, a `LayoutBox { .. }` literal). Excluding it kept real writes out of the
# inventory C-4 deletes against. It is the only token-carrying `test_*.rs` in the tree, so
# including the prefix costs a few allowlist rows and closes the hole.
test_path_re() {
  printf '(/tests?/|/tests\\.rs%s|/tests_[^/:]*\\.rs%s|_tests\\.rs%s)' "$1" "$1" "$1"
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
  # `|| true` on the filter stage, exactly like `live_readers`: a comments-only allowlist
  # makes `grep -v` exit 1, which pipefail propagates into the caller's `$(…)` assignment
  # and `set -e` then aborts the script with NO diagnostic at all.
  { grep -vE '^[[:space:]]*(#|$)' "$ALLOWLIST" || true; } | cut -f2- | sort -u
}

# The classification vocabulary — the SINGLE machine-readable definition. Wire #4 validates
# column 1 against it, which is what makes the vocabulary an enforced contract instead of a
# comment three files restate (the .tsv header, the `--regenerate` emitter, and the audit
# doc's legend had all drifted apart from each other AND from the data). It is also what
# stops `--regenerate`'s `UNCLASSIFIED` placeholder from riding into a green run: wire #1's
# own FAIL message prescribes `--regenerate`, so without this an author following the tool's
# instructions after a refactor would silently absorb a genuinely-new reader as unclassified
# and turn the gate green — a structural gate degraded to a review convention.
# `pending-migration:C-[0-9]+[a-z]*` keeps the slice suffix OPEN, matching the form the
# .tsv header and the audit legend document (`pending-migration:<slice>`). A closed
# `C-3[b-e]` made wire #4 reject a correctly-classified row the moment a slice split or
# was renamed, with a message telling the author to classify what they just classified.
CLASSES='producer|seam|type-def|import|test|pending-migration:C-[0-9]+[a-z]*'

if [ "${1:-}" = "--regenerate" ]; then
  # Preserve any existing classification for a (path, content) still present;
  # new lines get `UNCLASSIFIED` for the author to triage against the audit doc.
  # NB: no `declare -A`. Associative arrays need bash >= 4 and macOS ships /bin/bash 3.2,
  # so on a stock checkout the very command wire #1's FAIL message prescribes would abort
  # under `set -e` before writing anything. The prior classification is instead looked up
  # per line out of the committed file (124 rows — the O(n^2) is irrelevant here).
  PREV=""
  if [ -f "$ALLOWLIST" ]; then
    PREV="$(grep -vE '^[[:space:]]*(#|$)' "$ALLOWLIST" || true)"
  fi
  prev_class() { # $1 = path, $2 = content
    printf '%s\n' "$PREV" | awk -F'\t' -v p="$1" -v c="$2" \
      '$2 == p && substr($0, index($0, "\t") + length($2) + 2) == c { print $1; exit }'
  }
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
      cls="$(prev_class "$path" "$content")"
      printf '%s\t%s\t%s\n' "${cls:-UNCLASSIFIED}" "$path" "$content"
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
# `([[:space:]]*<[^=]*>)?` admits generic / lifetime parameters between the alias name
# and the `=`: without it `type BoxRef<'a> = &'a elidex_plugin::LayoutBox;` slipped
# through (demonstrated live — the alias plus a token-less `get::<&BoxRef>(e)` read left
# every wire green).
BAN_TYPEALIAS='type[[:space:]]+[A-Za-z_][A-Za-z0-9_]*([[:space:]]*<[^=]*>)?[[:space:]]*=[^;]*(LayoutBox|BoxModel)'
# grep is LINE-based, so an alias rustfmt wrapped across two lines carries the token on
# a line with no `type` and the `type` on a line with no token — invisible to the two
# patterns above. Rather than attempt multi-line matching, ban the wrap itself: an alias
# whose RHS starts on the next line must be written on one line so the gate can see it.
# Narrow by construction (only fires on a `type X … =` line with nothing after the `=`).
BAN_TYPEALIAS_WRAP='^[[:space:]]*type[[:space:]]+[A-Za-z_][A-Za-z0-9_]*([[:space:]]*<[^=]*>)?[[:space:]]*=[[:space:]]*$'

# Positive control — the STRUCTURAL fix for the class, not just the regex. A ban wire that
# cannot match is indistinguishable from a clean tree, so trusting its verdict requires first
# proving it fires. Each banned shape is matched against a synthetic violation using the SAME
# engine and flags as the real scan (`git grep --no-index -nwE`); a control that fails to hit
# means the pattern is dead and the wire aborts instead of reporting a false all-clear.
ban_control() { # $1 = pattern, $2 = sample line that MUST match, $3 = shape label
  # Run from INSIDE the scratch dir with a relative pathspec: `git grep --no-index` refuses
  # an absolute path pointing outside the enclosing repository, which would make every
  # control silently "fail" for the wrong reason and mask a pattern that is actually fine.
  local dir rc
  # Distinguish an environment failure from a dead pattern: with `dir` empty the control
  # would grep the caller's cwd for a nonexistent file, fail, and blame the regex.
  if ! dir="$(mktemp -d)" || [ -z "$dir" ] || [ ! -d "$dir" ]; then
    red "FAIL: wire #2 self-test could not create a scratch dir (TMPDIR/disk?) — the ban"
    red "      patterns were NOT validated, so this run's verdict is not trustworthy."
    return 1
  fi
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
# ⚠ Every ban_control call MUST be part of an `|| …` list. `set -e` is suspended only
# inside a command that is an operand of `||`, so a bare `ban_control …` statement would
# abort the script at the first non-zero step instead of reporting the diagnostic above.
control_ok=0
ban_control "$BAN_ALIAS" 'use elidex_plugin::LayoutBox as LB;' 'import/re-export alias' || control_ok=1
ban_control "$BAN_TYPEALIAS" 'type MyBox = elidex_plugin::LayoutBox;' 'type alias' || control_ok=1
ban_control "$BAN_TYPEALIAS" "type BoxRef<'a> = &'a elidex_plugin::LayoutBox;" 'generic/lifetime type alias' || control_ok=1
ban_control "$BAN_ALIAS" 'use elidex_plugin::{LayoutBox as LB, Rect};' 'braced alias' || control_ok=1
ban_control "$BAN_TYPEALIAS_WRAP" 'type MyBox =' 'wrapped type alias' || control_ok=1
if [ "$control_ok" -ne 0 ]; then
  fail=1
else
    # `-w` would reject the wrap ban (its match ends at `=`, a non-word char, but starts at
  # `type` which is fine — it is the ANCHORED `^…$` form that `-w` cannot help), so the
  # wrap pattern is scanned separately without `-w`.
  # The wrap ban is scoped to files that CARRY a token: a wrapped alias only hides a
  # reference if its continuation line names the type, so such a file always matches the
  # token grep. Scanning the whole tree instead fired on unrelated wrapped aliases
  # (`subtle_crypto/ops.rs` `type CipherOp =`) — a gate that reds CI on code with no
  # `LayoutBox` in it teaches people to ignore it.
  token_files="$(cd "$ROOT" && git grep -lwE 'LayoutBox|BoxModel' -- 'crates/**/*.rs' || true)"
  wrap_hits=""
  if [ -n "$token_files" ]; then
    # shellcheck disable=SC2086
    wrap_hits="$(cd "$ROOT" && grep -nH -E "$BAN_TYPEALIAS_WRAP" -- $token_files 2>/dev/null || true)"
  fi
  ban_hits="$( { cd "$ROOT" && git grep -nwE "$BAN_ALIAS|$BAN_TYPEALIAS" -- 'crates/**/*.rs'; printf '%s' "$wrap_hits"; } | strip_comments || true)"
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
    # `-H --`: without `-H`, grep omits the `path:` prefix when handed exactly ONE file,
  # and `strip_comments`' `^[^:]*:[0-9]+:` anchor then stops matching — every doc-comment
  # mention of `macro_rules!` in that file becomes a false FAIL. Reachable precisely as
  # the C-3 migration shrinks the reader-file set toward one. `--` stops a path that
  # begins with `-` being read as an option.
  macro_hits="$(cd "$ROOT" && grep -nH -E 'macro_rules!' -- $reader_files 2>/dev/null | strip_comments | grep -vE "macro_rules![[:space:]]+($ALLOWED_MACROS)\b" || true)"
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
  # A duplicate `(path, content)` key carrying a DIFFERENT classification survives wire #1
  # (`committed_readers` does `sort -u`, so the two sets compare equal) and survives the
  # membership test above (both values are legal). At C-4 the "no surviving
  # pending-migration" proof could then be satisfied by whichever copy the author reads.
  dup_keys="$( { grep -vE '^[[:space:]]*(#|$)' "$ALLOWLIST" || true; } | cut -f2- | sort | uniq -d)"
  if [ -n "$dup_keys" ]; then
    red "FAIL: duplicate (path, content) allowlist key(s) — wire #1 dedups them, so a second"
    red "      row with a different classification would be invisible. Keep exactly one row:"
    printf '%s\n' "$dup_keys" | sed 's/^/  /'
    fail=1
  else
    rows="$(grep -cvE '^[[:space:]]*(#|$)' "$ALLOWLIST" || true)"
    green "OK ($rows rows, every classification in vocabulary, no duplicate keys)"
  fi
fi

if [ "$fail" -ne 0 ]; then
  red ""; red "LayoutBox/BoxModel reader-audit trip-wire FAILED"
  exit 1
fi
green ""; green "✓ all clear"
