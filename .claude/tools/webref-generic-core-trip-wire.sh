#!/usr/bin/env bash
# Layering trip-wire for the generic webref core (`.claude/tools/_webref/`
# plus the `.claude/tools/webref` entry script) — plan-memo
# 2026-07-citation-hygiene-Ai-spec-label-map.md §2 invariant K2, which that
# memo's §12(3) names as an exit criterion.
#
# `_webref/DESIGN.md` says the package "should stay generic enough to move to
# a standalone repository later" and closes with "keep new generic behavior
# free of elidex-specific file paths AND PUT ELIDEX POLICY IN ADAPTER COMMANDS
# OR DOCUMENTATION".
#
# ⚠ Those are TWO axes, and this wire reaches only the first.  Nothing
# mechanical covers the policy half — it is a judgement about wording, and the
# instrument for it is diff review.  #501 found the policy half violated in a
# commit set this wire was green on, five rounds running, so do not read a
# green here as `DESIGN.md` compliance.
#
# ONE CHECK, ABSOLUTE.  It is closed and decidable; it is not a heuristic, and
# this wire makes no un-asserted report.
#
# THE K2 PREDICATE, verbatim from the memo's §2: no `.claude/(skills|tools)/`
# plus two further path segments, anywhere in this tree.  NARROW — two fixed
# roots, a fixed segment count — so it enumerates its own population and is not
# a seed.  Measured 0 here.
#
# ⚠ It replaces a second check rather than joining one.  This wire began as a
# PIN on the one host path A-i removed (`.claude/skills/elidex-review/axes.md`,
# at two sites: `_webref/cli.py` and the `webref` entry script — measured 1 each
# at base `44cd165d`, 0 at HEAD), because at the time the K2 predicate had no
# assertion anywhere.  When #501 gate 4 restored that assertion the pin became
# strictly redundant: measured, the removed path IS a
# `.claude/(skills|tools)/<a>/<b>` string, so every input that fires the pin
# fires K2 and no input does the reverse.  Two checks where one contains the
# other is not belt-and-braces, it is a second decision surface — and it cost a
# real control: the pin's fixture was caught by the K2 arm, so killing the pin's
# verdict left the control green (#501 R70).  Collapsed to one.  The historical
# fact the pin carried is this comment, and the failure message names it.
#
#     ⚠ It was asserted once and the assertion was lost to two fixes that each
#     made sense alone.  `3aaad3cb` (R55) deleted
#     `TestSliceBoundary.test_no_elidex_file_path_in_this_package` as "a second
#     copy of a scan this wire already makes over a strictly larger range";
#     `b0912a6c` (R62) then demoted that scan to report-only, voiding the
#     deletion's ground, and nobody restored it.  Between those two commits a
#     file carrying `.claude/skills/elidex-plan-review/preflight.py` passed
#     this wire GREEN.
#
# WHAT THIS WIRE DELIBERATELY DOES NOT DO.  An earlier revision also ran a
# wide `<any top-level entry>/<something>` SEED — printed, never asserted.  It
# was widened four times, each fix opening the next hole (syntactic
# over-reach; resolving-on-disk under-reach; a string-keyed exemption that
# over-suppressed; a basename-keyed one that collided), and interpolation
# `docs/${x}/y.md` was the fifth.  This repo's recorded rule is that when a
# predicate cannot return its population it is a SEED and widening the regex
# is the wrong repair.  A seed that reports and asserts nothing is also a
# print with no consumer, which `CLAUDE.md` calls dead code.  So it is GONE,
# not demoted: what covers that open class is the diff — every line entering
# this tree passes review, and `git diff origin/main...HEAD -- .claude/` is
# finite.  Two classes it could never see either way: bare top-level names
# with no separator (`"docs"`, `"crates"`, `"CLAUDE.md"` — live at
# `cli.py`'s `--paths` default and `refresh.py`'s usage string, both
# pre-existing) and interpolation.
#
# RUNTIME: shell + grep only, bash 3.2 compatible, no toolchain.  That is a
# contract, not a coincidence: `.github/workflows/ci.yml` runs this driver
# with no setup step ("the wires are grep-only") and `CLAUDE.md` rests the
# ungated-job decision on it.  An earlier revision of this wire used `python3`
# and broke that premise for all five wires (#501 R69).  Anything needing more
# than grep belongs in a test, not here.
#
# Run from anywhere.  Exits non-zero if K2 fails or a file cannot be read.

set -euo pipefail

# `$0` as given may have no slash (`bash webref-generic-core-trip-wire.sh` from
# this directory), and the controls re-invoke it — through PATH, where it is not.
# Canonicalise once, so "run from anywhere" is true of the self-invocation too
# (#501 R74: all controls returned 127 and the clean wire exited 1).
SELF="$(cd "$(dirname "$0")" && pwd)/$(basename "$0")"
ROOT="$(cd "$(dirname "$SELF")/../.." && pwd)"
SCOPE_DIR="$ROOT/.claude/tools/_webref"
SCOPE_FILE="$ROOT/.claude/tools/webref"
# SELF-TEST MODE.  The controls below re-invoke this script over a fixture tree
# and assert its EXIT STATUS.  That is the point: a control that only inspects an
# internal variable proves the classifier, not the thing the verdict does with it
# — measured at #501 R70, where killing the verdict's error arm left a control
# passing and the wire green.  The subject has to be the exit code.
if [ -n "${WEBREF_WIRE_SELFTEST:-}" ]; then
  ROOT="$WEBREF_WIRE_SELFTEST"
  # A fixture may name a scope SUBDIRECTORY and an EXTRA ENTRY beside it, both
  # relative to the root, so a control can reproduce the real geometry — the
  # scope directory and the `webref` entry script are SIBLINGS, and an extra
  # entry inside the scope would be caught by the directory walk anyway, proving
  # nothing about its own arm.
  SCOPE_DIR="$ROOT/${WEBREF_WIRE_SELFTEST_DIR:-.}"
  SCOPE_FILE="${WEBREF_WIRE_SELFTEST_EXTRA:+$ROOT/$WEBREF_WIRE_SELFTEST_EXTRA}"
fi
for p in "$SCOPE_DIR" ${SCOPE_FILE:+"$SCOPE_FILE"}; do
  # `-e` FOLLOWS a symlink, so a dangling link reads as "does not exist" — and a
  # dangling link is still an entry whose stored target this wire must read
  # (#501 R75, found by the control for the symlinked entry script).
  [ -e "$p" ] || [ -L "$p" ] || { echo "!! $p does not exist — this wire would pass over a tree it never read" >&2; exit 2; }
done
# `git grep` takes pathspecs relative to the directory it runs in.
REL_DIR="${SCOPE_DIR#$ROOT/}"; [ "$REL_DIR" != "$SCOPE_DIR" ] || REL_DIR="."
REL_FILE=""; [ -z "$SCOPE_FILE" ] || REL_FILE="${SCOPE_FILE#$ROOT/}"

# §2's K2 predicate. Fixed ERE, `grep -E`. The one thing this wire asserts.
#
# A segment is "anything up to the next separator", where the separators are `/`
# and the characters that end a path in running text: whitespace, quotes and a
# backtick.  It was `[A-Za-z0-9_.-]` until #501 R74, which excluded segments §2
# admits — `.claude/tools/@scope/policy.md` and `.claude/skills/日本語/rule.md`
# both read GREEN.  ⚠ What stays outside is a segment containing WHITESPACE
# (`.claude/skills/team name/rule.md`): in arbitrary text that is not decidable
# without quoting rules, and this repo's recorded rule is that a predicate which
# cannot return its population is a seed, so it is named in §12(3)'s open half
# rather than guessed at here.
K2RE='\.claude/(skills|tools)/[^/[:space:]"'"'"'`]+/[^/[:space:]"'"'"'`]+'

# ⚠ An untracked file under the package is exactly where a violation lands
# during authoring, which is why the scan is `--no-index` (it reads working-tree
# contents, not the index) and `-a` (binary contents are searched, not skipped).
# This comment described the opposite — "a filesystem walk, not `git grep`", and
# "`-I` skips binaries" — for one round after the walk became git's (#501 R76).
# Both were the load-bearing choices, stated backwards.

# FAIL CLOSED ON A FILE IT CANNOT READ.  grep exits 0 on a match, 1 on none and
# **2 on an error** — and an unreadable file is an error, not an absence.  An
# earlier revision discarded both, so a mode-000 file carrying a host path was
# counted in "scanned N" and reported nothing: the wire printed its ABSOLUTE
# over a file it had never read (#501 R70, reproduced).  ONE arm, not two: a
# revision of this fix also ran a `-r` test, and measured, either arm alone
# catches the unreadable case, so each made the other's mutation survive.  `rc`
# is the more general of the pair — it also covers a read that fails after it
# starts, and a path that stops being a regular file between `find` and `grep`
# — so the redundant test is gone rather than kept for tidiness.
#
# AND IT SCANS BINARY CONTENT.  `grep -I` is `--binary-files=without-match`: a
# file holding a NUL is reported as *not matching* rather than as unscannable,
# so an earlier revision counted such a file in "scanned N" and certified K2
# over content it had skipped — measured with a fixture holding
# `\0.claude/skills/new-policy/rule.md\0`, which read GREEN (#501 R71).  `-a`
# treats every byte as text, so the predicate covers every file the count
# claims.  Matching is `-o`, so the report stays the matched path and not a
# binary dump.  (Two empty `__init__.py` here are already classified binary by
# `file --mime`, which is how little "binary" has to mean for this to matter.)
# THE WALK IS GIT'S, NOT A HAND-ROLLED ONE.  Six review rounds (#501 R69-R74)
# found nine ways for a `find | read | grep` walk to certify K2 over something
# it had not examined: a file it could not read, one whose content `grep -I`
# skipped, a subtree `find` could not descend into, a symlink, an entry of some
# other type, a filename holding a newline, a prune that matched by name rather
# than by type, and a segment class narrower than the invariant.  Measured, ONE
# command handles all but the symlink by construction:
#
#   git grep --no-index -anE <predicate> -- <scope>
#
# It works outside a repository (so the fixtures below need no `git init`),
# reports a newline-bearing filename as `"foo\nbar"` rather than splitting it,
# reads binary content under `-a`, descends without a prune of its own, and is
# the canonical way in this repo to ask what text a tree holds.  Writing a
# fourth variant of the hand-rolled walk when this exists is the thing
# CLAUDE.md's "既存の抽象で解決できないか考える" forbids.
#
# TWO THINGS IT DOES NOT DO, both handled explicitly below:
#   * it does not read a SYMLINK's target, and a symlink's stored content IS
#     that target (git keeps it as the blob), so links get their own pass;
#   * it exits 0 on a permission error and reports it only on stderr, so a
#     non-empty stderr is treated as a failed walk.
#
# COUNTED IS SCANNED.  The count comes from the same tool over the same scope
# (`-l` with a pattern every line matches), not from a separate traversal, so
# "counted but not scanned" stays unrepresentable.
#
# An entry git cannot store — a fifo, a socket, a device — is not listed, and
# that is the right line rather than an omission: K2 is about text that lives
# in this tree, and such an entry holds none.  (Verified: a fifo in the scope
# neither hangs nor hides a plant elsewhere.)
_scan() { # $1 = scope dir, $2 = extra file, both RELATIVE to $ROOT
  _e="$(mktemp)" || { printf 'err\twalk: no temp file for the walk errors\n'; return 0; }
  _l="$(mktemp)" || { rm -f "$_e"; printf 'err\twalk: no temp file for the file list\n'; return 0; }
  _dir="$1"; _extra="${2:-}"
  # ONE LIST DRIVES EVERYTHING — population, names, contents and the count.
  # `git ls-files` answers, in git's own terms, what this tree HOLDS:
  #   --cached   tracked, including a file force-added under an ignored path
  #   --others --exclude-standard   untracked, MINUS what .gitignore excludes
  # That second half is the fix for generated bytecode: a `.pyc` embeds the
  # absolute source path, which matches K2RE, so scanning ignored artefacts
  # turned the wire red for any contributor who had merely run the tool (#501
  # R79). It is also not text this tree holds — it is a build product.
  # `-z` because a tracked filename may contain a newline, and the list is the
  # count, so an empty file is enumerated too (#501 R79: `git grep -l` lists
  # only files with a matching line, so an empty one skipped even the NAME
  # check).
  git -C "$ROOT" ls-files -z --cached --others --exclude-standard \
      -- "$_dir" ${_extra:+"$_extra"} > "$_l" 2>>"$_e" || true
  [ -s "$_e" ] && printf 'err\tthe walk reported errors, so part of the scope went unread: %s\n' \
    "$(tr '\n' ';' < "$_e" | cut -c1-200)"
  : > "$_e"
  while IFS= read -r -d '' rel; do
    f="$ROOT/$rel"
    # THE NAME. An entry whose own path IS the forbidden hierarchy is the most
    # direct violation there is, and content search cannot see it. Matched
    # relative to the SCOPE: relative to the repo every file here would match,
    # since the generic core itself lives under `.claude/tools/`.
    printf '%s\n' "${rel#$_dir/}" | grep -aEo -- "$K2RE" | while IFS= read -r m; do
      printf 'k2\t%s: (the entry NAME is itself) %s\n' "$rel" "$m"
    done
    if [ -L "$f" ]; then
      # A symlink's stored content IS its target string; git keeps it as the blob.
      tgt="$(readlink "$f" 2>/dev/null)" || {
        printf 'err\t%s: symlink, but its target could not be read\n' "$rel"; continue; }
      printf 'ok\t%s\n' "$rel"
      printf '%s\n' "$tgt" | grep -aEo -- "$K2RE" | while IFS= read -r m; do
        printf 'k2\t%s: -> %s\n' "$rel" "$m"
      done
      continue
    fi
    # THE CONTENT. `-a` because binary content is content; `grep` rather than
    # `git grep` because `git grep -a --no-index` was measured NOT to match
    # inside a `.pyc` on this machine while plain `grep -a` does, and a wire
    # whose reach depends on which git is installed is not an absolute.
    out="$(grep -aEn -- "$K2RE" "$f" 2>/dev/null)" || [ $? -eq 1 ] || {
      printf 'err\t%s: unreadable, or the read failed\n' "$rel"; continue; }
    printf 'ok\t%s\n' "$rel"
    [ -z "$out" ] || printf '%s\n' "$out" | while IFS= read -r hit; do
      printf 'k2\t%s:%s\n' "$rel" "$hit"
    done
  done < "$_l"
  rm -f "$_l" "$_e"
  return 0
}

# ⚠ `-a` ON EVERY ARM. A matched line can carry a NUL (the scan reads binary
# content deliberately, and #501 R77 removed the cache exclusion that had kept
# most of it out), and `grep` without `-a` answers "binary file matches" on
# stdin and prints NOTHING — leaving `K2_HITS` empty and the run green over a
# violation it had already found (#501 R78, reproduced: the same pipeline
# returns rc 1 and no output without `-a`, and the record with it).
# ⚠ NO FIXTURE ASSERTS THIS, and saying so is the point: `_verdict` takes its
# input as a shell STRING, and `read` and `$( )` both drop NULs, so in this
# shape a NUL cannot reach here and reverting `-a` is not a caught mutation.
# It is kept for the shape one refactor away — streaming `_scan`'s output
# instead of round-tripping it through a variable — where it becomes
# load-bearing silently.
_verdict() { # $1 = _scan output; sets K2_HITS / ERR_HITS / SCANNED
  K2_HITS="$(printf '%s\n' "$1" | grep -a '^k2	' || true)"
  ERR_HITS="$(printf '%s\n' "$1" | grep -a '^err	' || true)"
  SCANNED="$(printf '%s\n' "$1" | grep -ac '^ok	' || true)"
}

# ---- CONTROLS: re-invoke THIS script over fixtures, assert the exit code ----
# The samples are spelled out a SECOND time on purpose. They are the independent
# subject each check is tested against, exactly as `layout-box-reader-trip-wire.sh`'s
# `ban_control` passes a hand-written sample line beside the pattern. If someone
# edits $PIN or $K2RE to a different spelling, the two stop agreeing and THAT is
# the signal — the failure a pattern-derived fixture cannot see.
# Two fixtures for the one predicate, on purpose: the path A-i actually removed
# (the case the wire exists for) and a path that never existed here (the general
# case). If $K2RE is edited to a different shape, both stop agreeing with it.
CONTROL_REMOVED='.claude/skills/elidex-review/axes.md'
CONTROL_K2='.claude/skills/new-policy/rule.md'
# $K2RE names TWO roots. A fixture for only one leaves the other unasserted —
# measured: narrowing the pattern to `skills` alone survived every control
# until this fixture existed (#501 R70).
CONTROL_TOOLS='.claude/tools/some-other-lane/artifact.tsv'
# A host path wrapped in NULs. `grep -I` calls such a file "no match"; the
# fixture exists so that answer can never be mistaken for a clean one again.
CONTROL_BINARY='.claude/skills/new-policy/rule.md'
CONTROL_CLEAN='a label map and nothing that looks like a host path'

if [ -z "${WEBREF_WIRE_SELFTEST:-}" ]; then
  # Distinguish an environment failure from a dead assertion: an empty scratch
  # dir would exercise nothing and silently "pass". `mktemp -d` is checked, and
  # the cleanup path is the absolute one it returned (#501 R55: an unchecked
  # `mktemp` made an `rm -rf` expand to the repo root).
  if ! CTL="$(mktemp -d)" || [ -z "$CTL" ] || [ ! -d "$CTL" ]; then
    echo "!! could not create a scratch dir for the controls (TMPDIR/disk?)," >&2
    echo "   so this run's assertions were never proved able to fire." >&2
    exit 2
  fi
  trap 'chmod -R u+rwX "$CTL" 2>/dev/null || true; case "$CTL" in /*/*) rm -rf "$CTL";; esac' EXIT

  for d in clean pin k2 tools binary err empty walk link odd nl seg cache cachedir extra name linkname ignored; do mkdir -p "$CTL/$d"; done
  mkdir -p "$CTL/walk/sub"
  printf '# %s\n' "$CONTROL_CLEAN" > "$CTL/walk/top.py"
  printf '# %s\n' "$CONTROL_CLEAN"  > "$CTL/clean/control.py"
  printf 'AXES = "%s"\n' "$CONTROL_REMOVED" > "$CTL/pin/control.py"
  printf 'RULE = "%s"\n' "$CONTROL_K2"      > "$CTL/k2/control.py"
  printf 'ART  = "%s"\n' "$CONTROL_TOOLS"   > "$CTL/tools/control.py"
  printf 'x\000%s\000y\n' "$CONTROL_BINARY"  > "$CTL/binary/control.dat"
  printf '# %s\n' "$CONTROL_CLEAN"          > "$CTL/link/ok.py"
  ln -s "$CONTROL_K2" "$CTL/link/forbidden-target"
  # An entry git cannot store. ⚠ This fixture's expected verdict CHANGED at
  # #501 R74: it used to require exit 1 ("the wire cannot say what this holds"),
  # and now requires exit 0 with the sibling still read. The reason is §2's
  # wording, not convenience — K2 is about a path this tree *names*, i.e. stored
  # text, and a fifo holds none: it cannot be committed and cannot survive a
  # checkout. What the control still pins is that such an entry neither hangs
  # the walk nor suppresses the verdict over its siblings.
  printf '# %s\n' "$CONTROL_CLEAN"          > "$CTL/odd/ok.py"
  mkfifo "$CTL/odd/pipe" 2>/dev/null || true
  # A filename holding a newline: `-print` plus `read` would split it into
  # fragments and scan those instead of the file (#501 R74).
  printf '# %s\n' "$CONTROL_CLEAN"          > "$CTL/nl/foo"
  printf '# %s\n' "$CONTROL_CLEAN"          > "$CTL/nl/bar"
  printf 'RULE = "%s"\n' "$CONTROL_K2"      > "$CTL/nl/$(printf 'foo\nbar')"
  # Segments §2 admits that a `[A-Za-z0-9_.-]` class did not.
  printf 'A = "%s"\nB = "%s"\n' '.claude/tools/@scope/policy.md' '.claude/skills/日本語/rule.md' \
                                             > "$CTL/seg/control.py"
  # A REGULAR FILE named `__pycache__`: pruning by name alone skipped it.
  printf '# %s\n' "$CONTROL_CLEAN"          > "$CTL/cache/ok.py"
  printf 'RULE = "%s"\n' "$CONTROL_K2"      > "$CTL/cache/__pycache__"
  # A file under a REAL cache directory. Excluding it by location hid it from
  # both passes even when tracked (#501 R77).
  mkdir -p "$CTL/cachedir/__pycache__"
  printf '# %s\n' "$CONTROL_CLEAN"          > "$CTL/cachedir/ok.py"
  printf 'RULE = "%s"\n' "$CONTROL_K2"      > "$CTL/cachedir/__pycache__/probe.txt"
  # A CLEAN file whose own path is the forbidden hierarchy. `git grep` searches
  # contents, so without a name pass this reads as a file with nothing in it.
  mkdir -p "$CTL/name/$(dirname "$CONTROL_K2")"
  printf '# %s\n' "$CONTROL_CLEAN"          > "$CTL/name/$CONTROL_K2"
  printf '# %s\n' "$CONTROL_CLEAN"          > "$CTL/name/ok.py"
  # …and the same shape as a SYMLINK, whose name pass is a separate arm.
  mkdir -p "$CTL/linkname/$(dirname "$CONTROL_K2")"
  printf '# %s\n' "$CONTROL_CLEAN"          > "$CTL/linkname/ok.py"
  ln -s "$CTL/linkname/ok.py" "$CTL/linkname/$CONTROL_K2"
  # An IGNORED generated artefact carrying a forbidden path. It must NOT fire:
  # a `.pyc` embeds its source's absolute path, and scanning build products
  # turned the wire red for anyone who had merely run the tool (#501 R79).
  printf '# %s\n' "$CONTROL_CLEAN"          > "$CTL/ignored/ok.py"
  printf 'generated/\n'                      > "$CTL/ignored/.gitignore"
  mkdir -p "$CTL/ignored/generated"
  printf 'SRC = "%s"\n' "$CONTROL_K2"       > "$CTL/ignored/generated/artefact.bin"
  # An EMPTY tracked file whose NAME is the hierarchy: the list must enumerate
  # it even though no content matches (#501 R79).
  mkdir -p "$CTL/name/$(dirname "$CONTROL_K2")"
  : > "$CTL/name/empty-$(basename "$CONTROL_K2")"

  # Every fixture is a repository, because the population is git's answer:
  # tracked, plus untracked minus ignored. A fixture that is not a repo cannot
  # reproduce that distinction — and the distinction is now load-bearing.
  for d in clean pin k2 tools binary err empty walk link odd nl seg cache \
           cachedir extra name linkname ignored; do
    ( cd "$CTL/$d" 2>/dev/null && git init -q . >/dev/null 2>&1 \
      && git add -A >/dev/null 2>&1 ) || true
  done
  # …and the cache fixture's probe is FORCE-added under an ignored path, which
  # is the case `--cached` exists to keep (#501 R77).
  ( cd "$CTL/cachedir" && printf '__pycache__/\n' > .gitignore \
    && git add -A >/dev/null 2>&1 \
    && git add -f __pycache__/probe.txt >/dev/null 2>&1 ) || true
  # The real geometry: a scope directory and, BESIDE it, an entry script that
  # is itself a symlink holding a forbidden target.
  mkdir -p "$CTL/extra/sub"
  printf '# %s\n' "$CONTROL_CLEAN"          > "$CTL/extra/sub/ok.py"
  ln -s "$CONTROL_K2" "$CTL/extra/entry"
  printf 'AXES = "%s"\n' "$CONTROL_REMOVED" > "$CTL/err/control.py"
  # A readable sibling, so the run reaches the ERROR verdict instead of stopping
  # at the zero-read guard — the fixture must exercise the arm it names.
  printf '# %s\n' "$CONTROL_CLEAN"          > "$CTL/err/readable.py"
  printf 'RULE = "%s"\n' "$CONTROL_K2"      > "$CTL/walk/sub/hidden.py"
  chmod 000 "$CTL/err/control.py" "$CTL/walk/sub"

  _control() { # $1 = root, $2 = expected exit, $3 = expected message, $4 = label,
               # $5 = optional scope subdir (relative), $6 = optional extra entry (relative)
    _out="$(WEBREF_WIRE_SELFTEST="$1" WEBREF_WIRE_SELFTEST_DIR="${5:-}" \
            WEBREF_WIRE_SELFTEST_EXTRA="${6:-}" "$SELF" 2>&1)"; _rc=$?
    if [ "$_rc" -ne "$2" ]; then
      echo "!! CONTROL FAILED ($4): expected exit $2, got $_rc. A green below would" >&2
      echo "   mean nothing — this wire was not shown able to reach that verdict." >&2
      printf '%s\n' "$_out" | sed 's/^/     /' >&2
      return 1
    fi
    case "$_out" in *"$3"*) : ;; *)
      echo "!! CONTROL FAILED ($4): exit $2 as expected, but for the wrong reason —" >&2
      echo "   the output does not contain \"$3\"." >&2
      printf '%s\n' "$_out" | sed 's/^/     /' >&2
      return 1 ;;
    esac
    return 0
  }

  # ⚠ Every _control call is an operand of `||`: `set -e` is suspended only
  # inside such a command, so a bare call would abort the script on the first
  # non-zero and the remaining arms would never run.
  ctl_ok=0
  _control "$CTL/clean" 0 "PASSED"                  "green is reachable"   || ctl_ok=1
  _control "$CTL/pin"   1 "K2: a"  "K2 fires on the path A-i removed" || ctl_ok=1
  _control "$CTL/k2"    1 "K2: a"  "K2 fires on a path never here"    || ctl_ok=1
  _control "$CTL/tools" 1 "K2: a"  "K2 fires under the tools root too" || ctl_ok=1
  _control "$CTL/binary" 1 "K2: a" "K2 fires inside binary content"    || ctl_ok=1
  _control "$CTL/link"   1 "K2: a" "K2 fires on a symlink's target"    || ctl_ok=1
  _control "$CTL/nl"     1 "K2: a" "a newline in a filename is not a split" || ctl_ok=1
  _control "$CTL/seg"    1 "K2: a" "K2 covers @ and non-ASCII segments"     || ctl_ok=1
  _control "$CTL/cache"  1 "K2: a" "a regular file named __pycache__ is read" || ctl_ok=1
  _control "$CTL/cachedir" 1 "K2: a" "a file UNDER a cache directory is read"  || ctl_ok=1
  _control "$CTL/name"   1 "entry NAME" "an entry's own NAME is the hierarchy"  || ctl_ok=1
  _control "$CTL/linkname" 1 "entry NAME" "a SYMLINK's own name is the hierarchy" || ctl_ok=1
  _control "$CTL/ignored" 0 "PASSED" "an IGNORED generated artefact does not fire" || ctl_ok=1
  _control "$CTL/extra"  1 "K2: a" "a symlinked EXTRA entry is scanned" "sub" "entry" || ctl_ok=1
  if [ -p "$CTL/odd/pipe" ]; then
    _control "$CTL/odd" 0 "PASSED" "an unstorable entry neither hangs nor hides" || ctl_ok=1
  else
    echo "  note: the odd-entry control could not be exercised here (no fifo);"
    echo "        every other control ran"
  fi
  # An empty scope must be an ERROR, not a pass: "no violations" and "nothing
  # read" are different answers and only one of them is green.
  _control "$CTL/empty" 2 "read 0 entries" "an empty scope fails loudly" || ctl_ok=1
  # ⚠ The line is built HERE, beside the decision that produces it. An earlier
  # shape decided here and described it in the summary below, so the two could
  # disagree — and an unconditional summary claimed exit-status evidence the run
  # had not obtained (#501 R78). One site, one truth.
  _perm_line="            ⚠ NOT EXERCISED on this machine: the unreadable-file and
            unsearchable-directory controls (this user can read a mode-000 file),
            so this run carries no evidence for those two"
  if [ -r "$CTL/err/control.py" ]; then
    :
  else
    _perm_line="            an unreadable file and an unsearchable directory also fail closed"
    _control "$CTL/err"  1 "could not be read" "an unreadable file fails closed" || ctl_ok=1
    _control "$CTL/walk" 1 "could not be read" "an unsearchable dir fails closed" || ctl_ok=1
  fi
  [ "$ctl_ok" -eq 0 ] || exit 1
  echo "  controls: green reachable; K2 fires under both roots, on the removed path,"
  echo "            inside binary content, on a symlink's stored target, on an entry's"
  echo "            own NAME, and on a symlinked entry script beside the scope; an empty"
  echo "            scope fails closed; an entry git cannot store neither hangs nor hides"
  echo "            a verdict"
  printf '%s\n' "$_perm_line"
  echo "            (each asserted on this script's own exit status, over a fixture tree)"
fi

# ---- THE REAL TREE ----------------------------------------------------------
_verdict "$(_scan "$REL_DIR" ${REL_FILE:+"$REL_FILE"} || true)"
if [ "$SCANNED" -eq 0 ]; then
  echo "!! read 0 entries; this wire would report no violation for a reason that is not 'there are none'" >&2
  exit 2
fi
# The population is git's: tracked, plus untracked minus ignored. An EMPTY file
# is in it -- its name is still part of the tree (#501 R79).
echo "  read $SCANNED entr(y/ies) under the generic core, in full"
failed=0

if [ -n "$K2_HITS" ]; then
  echo "!! K2: a \`.claude/(skills|tools)/<a>/<b>\` host path is named in the generic core."
  echo "   (If one of these is \`.claude/skills/elidex-review/axes.md\`, it is the path A-i"
  echo "    removed from \`_webref/cli.py\` and the \`webref\` entry script, come back.)"
  printf '%s\n' "$K2_HITS" | sed 's/^k2	/     /'
  failed=1
else
  echo "  K2: 0 \`.claude/(skills|tools)/<a>/<b>\` paths named here -- ABSOLUTE"
fi

if [ -n "$ERR_HITS" ]; then
  echo "!! part of the generic core could not be read, so the verdict above does not"
  echo "   cover it -- this wire does not report a green over what it never read:"
  printf '%s\n' "$ERR_HITS" | sed 's/^err	/     /'
  failed=1
fi

[ "$failed" -eq 0 ] || exit 1
echo "webref generic-core layering trip-wire PASSED"
