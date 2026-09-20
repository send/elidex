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

# BYTES, NOT CHARACTERS.  Every predicate here is a byte pattern over content
# git may hold, and under a UTF-8 locale an invalid byte cannot participate in a
# bracket expression — measured, a file holding `.claude/skills/\xffname/rule.md`
# read GREEN under `LC_ALL=C.UTF-8` and RED under `LC_ALL=C` (#501 R88).  `-a`
# governs binary-file *handling*; it does not change multibyte regex semantics.
# So the locale is pinned for the whole run rather than per call site.
export LC_ALL=C
# ⚠ AND NO NETWORK. In a blobless partial clone an indexed blob may be PROMISED
# rather than local, and `git cat-file` will then fetch it on demand — measured,
# the run spawned `git fetch origin --filter=blob:none` and `git-upload-pack`
# (#501 R94). That contradicts this gate's own contract (`.github/workflows/
# ci.yml`: no toolchain, no cache, no network) and would make a required local
# gate depend on credentials, connectivity and an unbounded remote operation.
# With lazy fetching off, an absent blob simply fails the read and becomes the
# `err` record that already exists — unknown fails closed, as everywhere else.
# ⚠ A git too old to know this variable ignores it, and then the fetch is back.
# Nothing here can detect that, and saying so is the honest position.
export GIT_NO_LAZY_FETCH=1
# ⚠ AND NO REPLACEMENT OBJECTS. A local `replace` ref — history-repair work
# leaves them — makes `cat-file` hand back a DIFFERENT object than the one the
# index and the commit name. Reproduced: a staged blob holding
# `.claude/skills/team/rule.md` replaced by a clean blob read K2 zero and
# exited 0, while the same read with this variable set showed the violation
# (#501 R95). What is committed is the object the index names, so that is the
# object this wire reads.
export GIT_NO_REPLACE_OBJECTS=1

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
# ONE SCRATCH ROOT, AND IT MUST NOT BE INSIDE WHAT WE ARE ABOUT TO SCAN.
# `mktemp` follows `TMPDIR`, and a workspace-confined sandbox may reasonably put
# it under the tree — at which point this wire scans the fixture repositories
# and temp files IT created, and answers about itself: a clean checkout went red
# on its own control tree, and in self-test mode an empty scope reported the
# scan's own temp files as entries and exited 0 (#501 R93).
# Refuse rather than guess: there is no portable directory known to be both
# writable and outside an arbitrary root, and "decided nothing" is this wire's
# answer whenever it cannot trust its own reading.
# ⚠ Physical paths on both sides (`pwd -P`), or a symlinked `TMPDIR` walks
# straight past a textual prefix test.
SCRATCH="$(mktemp -d)" || { echo "!! no scratch dir (TMPDIR/disk?), so nothing here was proved" >&2; exit 2; }
trap 'case "$SCRATCH" in /*/*) chmod -R u+rwX "$SCRATCH" 2>/dev/null || true; rm -rf "$SCRATCH";; esac' EXIT
_phys() { ( cd "$1" 2>/dev/null && pwd -P ); }
_scratch_p="$(_phys "$SCRATCH")"
_root_p="$(_phys "$ROOT")"
if [ -z "$_scratch_p" ] || [ -z "$_root_p" ]; then
  echo "!! could not resolve the scratch dir or the root to a physical path," >&2
  echo "   so this run could not prove it was not scanning its own workings." >&2
  exit 2
fi
case "$_scratch_p/" in
  "$_root_p"/*)
    echo "!! the scratch directory ($SCRATCH) is INSIDE the tree this wire scans" >&2
    echo "   ($ROOT). It would read the fixtures and temp files it created itself" >&2
    echo "   and report a verdict about its own workings. Set TMPDIR outside the" >&2
    echo "   scanned tree and run again; this run decided nothing." >&2
    exit 2 ;;
esac

for p in "$SCOPE_DIR" ${SCOPE_FILE:+"$SCOPE_FILE"}; do
  # `-e` FOLLOWS a symlink, so a dangling link reads as "does not exist" — and a
  # dangling link is still an entry whose stored target this wire must read
  # (#501 R75, found by the control for the symlinked entry script).
  [ -e "$p" ] || [ -L "$p" ] || { echo "!! $p does not exist — this wire would pass over a tree it never read" >&2; exit 2; }
done
# `git ls-files` takes pathspecs relative to the directory it runs in.
# ⚠ `${var#"$prefix"/}` — QUOTED. Unquoted, the operand after `#` is a PATTERN,
# so a checkout path holding a glob character stops matching itself: under
# `/work/repo[1]` the removal failed, `REL_DIR` fell back to `.`, and the wire
# inventoried the WHOLE REPOSITORY instead of the generic core — turning every
# ordinary host-path reference elsewhere in the tree into a K2 hit (#501 R95,
# reproduced under bash 5.3; note zsh does not re-interpret the operand, so this
# only shows under the shell the wire actually runs). Same lesson as R80 one
# layer down: a path is data, not protocol.
REL_DIR="${SCOPE_DIR#"$ROOT"/}"; [ "$REL_DIR" != "$SCOPE_DIR" ] || REL_DIR="."
REL_FILE=""; [ -z "$SCOPE_FILE" ] || REL_FILE="${SCOPE_FILE#"$ROOT"/}"

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

# …and the SAME invariant over a STORED PATH — an entry's own name, or a
# symlink's target — where the only delimiter is `/`.
#
# ⚠ Which means the VALUE must reach `grep` whole.  `printf | grep` makes a
# newline a record separator, so `.claude/skills/team<LF>name/rule.md` was split
# into two lines and matched neither (#501 R88, reproduced for a name and for a
# symlink target).  `_onerec` maps newline to a byte that no predicate mentions,
# so the segment stays one value and `[^/]+` spans it.
#
# In running text a quote or a space ends a path and nothing decides where;
# that is why $K2RE stops at them, and why a whitespace segment is named in
# §12(3)'s open half. A stored path has no such ambiguity: git hands it over
# whole. `.claude/skills/team"name/rule.md` is ONE entry with two segments, and
# the content predicate's terminators cut it in half — measured, the wire read
# that entry, reported K2 zero and exited 0 (#501 R87). Using the text
# predicate on a name was not conservatism, it was the wrong predicate.
K2RE_PATH='\.claude/(skills|tools)/[^/]+/[^/]+'

# ⚠ Three superseded accounts of the scanner lived here until #501 R81 — one
# describing `git grep --no-index` and `-a`/`-I`, one the `rc >= 2` arm, one
# the binary handling.  Each was true when written and wrong within a round or
# two of the next change, and each produced a finding of its own (R76, R80,
# R81).  They are deleted rather than corrected: the block below is the one
# account, the same collapse §12(3) made for the memo.
# THE WALK IS GIT'S, AND SO ARE THE BYTES.  Eight review rounds (#501 R69-R79)
# found ten ways for a hand-rolled walk to certify K2 over something it had not
# examined — a file it could not read, one whose content `grep -I` skipped, a
# subtree `find` could not descend into, a symlink, an entry of some other type,
# a filename holding a newline, a prune matching by name rather than type, a
# segment class narrower than the invariant, an entry's own NAME, and an empty
# file.  One authority answers all of it, and it is git:
#
#   * THE POPULATION is what git says this tree holds: tracked — INCLUDING a
#     file force-added under an ignored path (#501 R77) — plus untracked minus
#     what the tree's OWN `.gitignore` excludes (a `.pyc` embeds its source's
#     absolute path and would otherwise turn this wire red for anyone who had
#     merely run the tool, #501 R79).  NOT the "standard" git exclusions:
#     `$GIT_DIR/info/exclude` and the machine's `core.excludesFile` are
#     uncommitted, and a gate that calls itself absolute cannot read differently
#     on two machines from one commit (#501 R92).
#   * THE CONTENT is what git holds for each entry.  A tracked entry is read
#     from its INDEX BLOB — the bytes a commit would carry — AND from the
#     worktree, because the two differ in both directions and an unstaged
#     violation is one `git add` from being carried too.  An untracked entry has
#     no blob, so only the worktree.  Anything present that is neither a regular
#     file nor a symlink is an ERROR, never an open(): a tracked path replaced
#     by a fifo IS listed, and opening it blocked forever (#501 R92).
#   * A STORED PATH — an entry's own name, and a symlink's target from either
#     source — goes through the stored-path predicate, never the running-text
#     one.  `/` is its only delimiter; a quote, a space or a newline inside a
#     segment is data (#501 R87, R90, R93).
#   * THE COUNT is that same enumeration, so "counted but not scanned" is not
#     representable rather than merely checked — three rounds each found a
#     different way for two separately-derived quantities to disagree.
#
# ⚠ THE COMMANDS LIVE IN `_scan` AND `_entry`, NOT HERE.  This block used to
# spell out the `git ls-files` invocation; R92 changed that invocation and this
# account went on naming the very flags R92 had rejected, pointing the next
# maintainer back at the machine-dependent behaviour it had just removed (#501
# R93).  Three earlier rounds (R76, R80, R81) deleted DUPLICATE ACCOUNTS in
# favour of this one — and a restated COMMAND is that same duplication one level
# down.  What lives here is the property; how it is obtained lives next to the
# code that obtains it.
#
# ⚠ THE THREAT MODEL IS ACCIDENT, NOT ADVERSARY — and saying so bounds this
# file (#501 R94). A contributor who wants past this gate edits `REQUIRED_WIRES`
# in `scripts/trip-wires.sh`, which that file's own comment names as the one
# edit that genuinely disables it. Hardening against a crafted index while
# conceding a one-line edit to the registration would be incoherent. So a
# construction git can store but no filesystem can realise — a symlink blob
# holding a NUL — is not met with a NUL-safe reader: it is an ERROR, and the
# gate goes red. Every "unknown fails closed" decision in this file is the same
# decision, and anything outside the model gets that answer rather than a new
# mechanism.
#
# A permission failure is an ERROR, not an absence: `git ls-files` and `grep`
# both report it on stderr while exiting 0, so a non-empty stderr and a `grep`
# status above 1 each fail the run.
# A PATH IS DATA, NOT PROTOCOL. The records below are newline-separated and
# tab-tagged, and a tracked filename may contain both — so a file named
# `safe<LF>k2<TAB>forged` injected a synthetic K2 hit and the wire reported
# `forged` and exited 1 (#501 R80, reproduced). Every path is escaped on its
# way into a record; the escape is lossy on purpose, since what a reader needs
# is to find the entry, not to round-trip its bytes.
# EVERY `git` CALL IN THIS FILE GOES THROUGH HERE, because `-C "$ROOT"` does NOT
# win over the repository-routing environment: with `GIT_DIR`/`GIT_WORK_TREE`
# exported — a wrapper, a hook — the inventory described ANOTHER CHECKOUT while
# the worktree arm read files under `$ROOT`, and a fixture holding a forbidden
# path reported one clean entry and exited 0 (#501 R95, reproduced).
# ⚠ The list is git's own (`rev-parse --local-env-vars`), not a hand-written
# one: enumerating this by hand is how the next variable gets left authoritative.
# An empty or failing list means we cannot know what routes git, so the run
# decides nothing rather than guessing.
# ⚠ It unsets ONLY routing, never configuration: R94 exported a clean config
# process-wide and took `safe.directory` with it.
_GIT_LOCAL_VARS="$(git rev-parse --local-env-vars 2>/dev/null)" || _GIT_LOCAL_VARS=""
if [ -z "$_GIT_LOCAL_VARS" ]; then
  echo "!! this git cannot say which variables route it (rev-parse --local-env-vars)," >&2
  echo "   so this run could not prove it read the tree it was pointed at." >&2
  exit 2
fi
_git() { ( for _v in $_GIT_LOCAL_VARS; do unset "$_v"; done
           export GIT_NO_LAZY_FETCH=1 GIT_NO_REPLACE_OBJECTS=1
           exec git "$@" ); }

_esc() { printf '%s' "$1" | sed -e 's/\\/\\\\/g' | tr '\n\t' '~~'; }

# A stored path as ONE record: newline is data inside a segment, not a
# separator. `\001` is chosen because no predicate here mentions it.
_onerec() { printf '%s' "$1" | tr '\n' '\001'; }

# THE ONE PLACE A STORED PATH IS MATCHED.  Both stored-path subjects — an
# entry's own name and a symlink's target — run this, so "did the matcher
# fail?" is decided once instead of per arm.  Prints the matches, one per line.
# Status: 0 = matched, 1 = did not, **2 or more = the matcher failed**.
# ⚠ THAT LAST CASE IS THE WHOLE POINT (#501 R89).  These arms used to be bare
# `_onerec … | grep … | while …` pipelines inside a `_scan` the caller invokes
# under `|| true`, so a `grep` that exited 2 produced no match, no record and no
# trace: the entry still emitted `ok`, K2 stayed empty and the wire exited 0
# over a name it had never actually read (reproduced with a clean tracked
# `.claude/skills/team/rule.md` and a shim making only `grep -aEo` exit 2).
# `grep` defines status 2 as an error, and the content arm and `_verdict` both
# already separated it; these two were the arms that did not.
_match_path() { _onerec "$1" | grep -aEo -- "$K2RE_PATH"; }

# …AND THE ONE PLACE ITS ANSWER IS TURNED INTO RECORDS.  Three subjects reach
# it: an entry's own name, a symlink's target in the worktree, and — since #501
# R92 added the index arm — a symlink's STAGED blob, whose bytes are the target
# string.  R93 found that third one going through the running-text predicate
# instead: a staged target `.claude/skills/team name/rule.md` with a clean
# worktree target read GREEN, because a space terminates a path in running text
# and nothing terminates it in a stored one (#501 R87's rule, at an arm added
# after it).  A third copy of the status test was how that happened, so there
# is now one.
# $1 = the stored value, $2 = the entry (for records), $3 = what it is,
# $4 = how a hit is shown.
_stored() {
  _mrc=0; _m="$(_match_path "$1")" || _mrc=$?
  if [ "$_mrc" -gt 1 ]; then
    printf 'err\t%s: the %s went unchecked (its matcher exited %d)\n' \
      "$(_esc "$2")" "$3" "$_mrc"
    return 0
  fi
  [ -z "$_m" ] || printf '%s\n' "$_m" | while IFS= read -r m; do
    printf 'k2\t%s: %s %s\n' "$(_esc "$2")" "$4" "$m"
  done
  return 0
}

# CONTENT, from one file, emitting the records. $1 = file to read, $2 = the
# entry's path (for the record), $3 = which of its two sources this is.
# Returns 1 when the READ failed, as opposed to finding nothing (#501 R89's
# class: "no match" and "could not read" must not be the same answer).
# `-a` because binary content is content; `grep` rather than `git grep` because
# `git grep -a --no-index` was measured NOT to match inside a `.pyc` on this
# machine while plain `grep -a` does, and a wire whose reach depends on which
# git is installed is not an absolute.
_content() {
  _co="$(grep -aEn -- "$K2RE" "$1" 2>/dev/null)" || [ $? -eq 1 ] || return 1
  [ -z "$_co" ] || printf '%s\n' "$_co" | while IFS= read -r hit; do
    printf 'k2\t%s %s:%s\n' "$(_esc "$2")" "$3" "$hit"
  done
  return 0
}

# ONE ENTRY, and EVERY PLACE ITS BYTES CAN LIVE.
# ⚠ THE POPULATION IS GIT'S, SO THE CONTENT HAS TO BE GIT'S TOO (#501 R92).
# Taking the list from the index and the bytes from the working tree made the
# two disagree, and three separate reproductions came out of that one seam:
#   * a path staged with a violation and reverted in the worktree read GREEN,
#     while `git show :victim` still held it — a local pre-push run certifying
#     the very commit that pushes it;
#   * a tracked path replaced by a FIFO made `grep` block forever on a file git
#     never has to open — the local gate hung rather than failing closed;
#   * and the worktree half brought in whatever the machine's git was
#     configured to ignore (see the inventory below).
# So: a tracked entry is read from its INDEX BLOB — the bytes a commit would
# carry — AND from the worktree, because the two differ and an unstaged
# violation is one `git add` from being carried too. An untracked entry has no
# blob, so only the worktree. Anything present that is neither a regular file
# nor a symlink is an ERROR, never an open().
# ⚠ `:0:$rel`, not `:$rel`: a tracked file named `0:x.py` makes the short form
# name stage 0 of `x.py` instead — measured, it returned the OTHER file's
# content.
_entry() { # $1 = source (index|head|tree), $2 = its MODE there (empty for tree), $3 = path
  _src="$1"; _mode="$2"; rel="$3"; f="$ROOT/$rel"; _read=0
  # THE NAME. An entry whose own path IS the forbidden hierarchy is the most
  # direct violation there is, and content search cannot see it. Matched
  # relative to the SCOPE: relative to the repo every file here would match,
  # since the generic core itself lives under `.claude/tools/`.
  _stored "${rel#"$_dir"/}" "$rel" "entry NAME" "(the entry NAME is itself)"
  # (A) A STORED OBJECT — the index's, or HEAD's.
  if [ "$_src" != tree ]; then
    if [ "$_src" = index ]; then _spec=":0:$rel"; _tag="(staged)"; else _spec="HEAD:$rel"; _tag="(in HEAD)"; fi
    _brc=0
    _git -C "$ROOT" cat-file blob "$_spec" > "$_b" 2>/dev/null || _brc=$?
    if [ "$_brc" -ne 0 ]; then
      printf 'err\t%s: git lists it in %s, but its blob could not be read (exit %d)\n' \
        "$(_esc "$rel")" "$_src" "$_brc"
    elif [ "$_mode" = 120000 ]; then
      # A STAGED SYMLINK's blob IS its target string — a stored path, so it
      # takes the stored-path predicate (#501 R93). Read with a sentinel
      # because `$( )` strips trailing newlines (#501 R90's lesson, one arm on).
      # ⚠ A NUL FIRST. `$( )` drops NUL bytes, so a blob holding
      # `.claude/skills/<NUL>/rule.md` reached `_stored` as
      # `.claude/skills//rule.md` and read GREEN (#501 R94, reproduced via
      # `git hash-object` + `git update-index --cacheinfo`). No NUL-safe reader
      # is built for it: **no path can contain a NUL**, so such a blob is not a
      # symlink target at all, and "cannot be read as a path" is an ERROR — the
      # same fail-closed answer every other unreadable thing here gets. The
      # threat model is accident, not adversary (see the header); a blob crafted
      # to be unreadable reds the gate rather than passing it.
      if ! tr -d '\000' < "$_b" | cmp -s - "$_b"; then
        printf 'err\t%s: its staged symlink target holds a NUL, which no path can, so it was not read as one\n' \
          "$(_esc "$rel")"
      else
        _sb="$(cat "$_b"; printf 'R')"; _sb="${_sb%R}"
        _stored "$_sb" "$rel" "staged symlink TARGET" "$_tag ->"
      fi
      _read=1
    elif _content "$_b" "$rel" "$_tag"; then
      _read=1
    else
      printf 'err\t%s: its %s blob could not be searched\n' "$(_esc "$rel")" "$_src"
    fi
  fi
  # (B) THE WORKING TREE.
  if [ "$_src" != tree ]; then :
  elif [ -L "$f" ]; then
    # A symlink's stored content IS its target string; git keeps it as the blob.
    # ⚠ AND `$( )` STRIPS TRAILING NEWLINES, so the plain substitution truncated
    # the stored value: a target `.claude/skills/team/<LF>` arrived as
    # `.claude/skills/team/`, whose final segment is empty, so `[^/]+` could not
    # match and the wire exited 0 over a violation git stores verbatim (#501 R90,
    # reproduced). `-n` stops `readlink` adding its own newline, and the `R%d`
    # sentinel keeps the substitution from ending in one — so nothing is stripped
    # and the exit status still reaches us.
    # ⚠ Every OTHER stored value here avoids `$( )` already: the entry name comes
    # from `read -r -d ''`, and the two match captures hold `grep -o` output whose
    # records cannot end in a newline because `_onerec` removed them. This was the
    # one raw stored value that went through a substitution.
    # If some `readlink` lacks `-n` it exits non-zero here and the entry becomes an
    # `err` record: unknown fails closed, which is the only safe direction for a
    # portability question inside an absolute.
    tgt="$(readlink -n "$f" 2>/dev/null; printf 'R%d' "$?")"
    _rlrc="${tgt##*R}"; tgt="${tgt%R*}"
    if [ "$_rlrc" -ne 0 ]; then
      printf 'err\t%s: symlink, but its target could not be read\n' "$(_esc "$rel")"
    else
      _read=1
      _stored "$tgt" "$rel" "symlink TARGET" "->"
    fi
  elif [ -f "$f" ]; then
    if _content "$f" "$rel" "(worktree)"; then _read=1
    else printf 'err\t%s: unreadable, or the read failed\n' "$(_esc "$rel")"; fi
  elif [ -e "$f" ]; then
    printf 'err\t%s: in the worktree but neither a regular file nor a symlink, so it was NOT opened\n' \
      "$(_esc "$rel")"
  fi
  # A tracked path DELETED from the worktree reaches none of those arms; the
  # index pass answered for it with its own record.
  [ "$_read" -eq 0 ] || printf 'ok\t%s\n' "$(_esc "$rel")"
}

_scan() { # $1 = scope dir, $2 = extra file, both RELATIVE to $ROOT
  _e="$(mktemp "$SCRATCH/errXXXXXX")" || { printf 'err\twalk: no temp file for the walk errors\n'; return 0; }
  _lc="$(mktemp "$SCRATCH/lcXXXXXX")" || { rm -f "$_e"; printf 'err\twalk: no temp file for the tracked list\n'; return 0; }
  _lo="$(mktemp "$SCRATCH/loXXXXXX")" || { rm -f "$_e" "$_lc"; printf 'err\twalk: no temp file for the worktree list\n'; return 0; }
  _lh="$(mktemp "$SCRATCH/lhXXXXXX")" || { rm -f "$_e" "$_lc" "$_lo"; printf 'err\twalk: no temp file for the HEAD list\n'; return 0; }
  _b="$(mktemp "$SCRATCH/blobXXXXXX")"  || { rm -f "$_e" "$_lc" "$_lo"; printf 'err\twalk: no temp file for the staged blob\n'; return 0; }
  _dir="$1"; _extra="${2:-}"
  # THE LISTS ARE GIT'S ANSWER to what this tree HOLDS, in two halves because
  # the halves keep their bytes in different places (see `_entry`):
  #   --cached   tracked, including a file force-added under an ignored path
  #   --others   untracked
  # ⚠ `--exclude-per-directory=.gitignore`, NOT `--exclude-standard` (#501 R92).
  # "Standard" means the tree's `.gitignore` PLUS `$GIT_DIR/info/exclude` PLUS
  # the machine's `core.excludesFile` — two sources that are not committed and
  # not the repository's statement about what it carries. Reproduced: a global
  # `*.py` rule emptied the fixture repositories and the required local gate
  # exited 1 before it ever scanned this repo, and the untracked population
  # became a property of whoever ran it. A gate that calls itself ABSOLUTE
  # cannot read differently on two machines from the same commit.
  # The tree's own `.gitignore` stays honoured, and it is load-bearing: a `.pyc`
  # embeds its source's absolute path, which matches the predicate, so scanning
  # build products turned the wire red for anyone who had merely run the tool
  # (#501 R79).
  # `-z` because a tracked filename may contain a newline, and the list is the
  # count, so an empty file is enumerated too (#501 R79: `git grep -l` lists
  # only files with a matching line, so an empty one skipped even the NAME
  # check).
  # ⚠ ITS STATUS, NOT JUST ITS STDERR. `|| true` discarded the only signal an
  # inventory has when it fails silently — a partial list then gives SCANNED > 0,
  # no error record, and a green verdict over a population that was cut short
  # (#501 R85, reproduced with a `git` that printed one entry and exited 1).
  _ls_rc=0
  # `--stage`, not a bare `--cached`: the INDEX MODE is what says a staged blob
  # is a symlink target rather than file content, and asking the worktree
  # instead gets it wrong exactly when the two disagree (#501 R93).
  # ⚠ A conflicted path has stages 1/2/3 and no stage 0, so it appears more than
  # once and `:0:` cannot resolve it — each copy becomes an `err`, which is the
  # right answer: nothing here can say what such a tree would commit.
  _git -C "$ROOT" ls-files -z --stage \
      -- "$_dir" ${_extra:+"$_extra"} > "$_lc" 2>>"$_e" || _ls_rc=$?
  [ "$_ls_rc" -eq 0 ] || \
    printf 'err\tthe tracked inventory exited %d, so the population is incomplete\n' "$_ls_rc"
  # THE WORKING TREE's half is tracked AND untracked together: every path that
  # has bytes on disk right now, whichever list git files it under.
  _ls_rc=0
  _git -C "$ROOT" ls-files -z --cached --others --exclude-per-directory=.gitignore \
      -- "$_dir" ${_extra:+"$_extra"} > "$_lo" 2>>"$_e" || _ls_rc=$?
  [ "$_ls_rc" -eq 0 ] || \
    printf 'err\tthe worktree inventory exited %d, so the population is incomplete\n' "$_ls_rc"
  # …AND HEAD, because a push sends the COMMIT, not the index (#501 R95). A
  # violation committed and then fixed only in the index read green while
  # `git show HEAD:victim` still held it. Bounded at the TIP and no further:
  # elidex squash-merges, so what lands on main is the tip's tree, and the
  # commits below it are not what this gate is about. An unborn HEAD — every
  # fixture here, and a fresh clone before its first commit — is not an error:
  # there is simply nothing committed to read.
  if _git -C "$ROOT" rev-parse --verify --quiet HEAD >/dev/null 2>&1; then
    _ls_rc=0
    _git -C "$ROOT" ls-tree -r -z HEAD \
        -- "$_dir" ${_extra:+"$_extra"} > "$_lh" 2>>"$_e" || _ls_rc=$?
    [ "$_ls_rc" -eq 0 ] || \
      printf 'err\tthe HEAD inventory exited %d, so the population is incomplete\n' "$_ls_rc"
  fi
  [ -s "$_e" ] && printf 'err\tthe walk reported errors, so part of the scope went unread: %s\n' \
    "$(tr '\n' ';' < "$_e" | cut -c1-200)"
  : > "$_e"
  # `--stage` records are `<mode> <sha> <stage><TAB><path>`; the path may hold a
  # tab of its own, so strip up to the FIRST one only.
  # THREE PASSES, ONE SOURCE EACH — so "counted" and "scanned" stay the same
  # quantity per source, and a path living in all three is read three times
  # rather than once with two of its versions assumed.
  # `--stage` and `ls-tree -r` both emit `<mode> …<TAB><path>`; the path may hold
  # a tab of its own, so strip up to the FIRST one only.
  while IFS= read -r -d '' _rec; do
    _entry index "${_rec%% *}" "${_rec#*$'\t'}"
  done < "$_lc"
  while IFS= read -r -d '' _rec; do
    _entry head "${_rec%% *}" "${_rec#*$'\t'}"
  done < "$_lh"
  while IFS= read -r -d '' rel; do _entry tree "" "$rel"; done < "$_lo"
  rm -f "$_lc" "$_lo" "$_lh" "$_b" "$_e"
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
# ⚠ AND ITS OWN STATUS.  `grep` exits 1 for "no line selected" and **2 for an
# error**; `|| true` collapsed both, so an operational failure here reported an
# empty `K2_HITS` over a violation `_scan` had already found (#501 R88). Each
# arm now distinguishes them and a status above 1 aborts rather than answering.
_verdict() { # $1 = _scan output; sets K2_HITS / ERR_HITS / SCANNED
  _vrc=0; K2_HITS="$(printf '%s\n' "$1" | grep -a '^k2	')" || _vrc=$?
  [ "$_vrc" -le 1 ] || { echo "!! the K2 classifier failed (grep exit $_vrc); this run decided nothing" >&2; exit 2; }
  _vrc=0; ERR_HITS="$(printf '%s\n' "$1" | grep -a '^err	')" || _vrc=$?
  [ "$_vrc" -le 1 ] || { echo "!! the error classifier failed (grep exit $_vrc); this run decided nothing" >&2; exit 2; }
  _vrc=0; SCANNED="$(printf '%s\n' "$1" | grep -ac '^ok	')" || _vrc=$?
  [ "$_vrc" -le 1 ] || { echo "!! the count classifier failed (grep exit $_vrc); this run decided nothing" >&2; exit 2; }
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
  # THE CONTROLS LIVE BESIDE THIS FILE. Split out when the wire crossed 1000
  # lines, at the seam that was already there: this file ANSWERS, that one
  # proves the answers are reachable (CLAUDE.md touch-time split; #501 R96).
  # ⚠ A MISSING CONTROLS FILE IS NOT A SKIPPED SELF-TEST. Sourcing it is the
  # only way this wire earns the word PASSED, so its absence ends the run at
  # "decided nothing" rather than producing a verdict nothing stands behind.
  _CONTROLS="${SELF%.sh}.controls.sh"
  if [ ! -r "$_CONTROLS" ]; then
    echo "!! the controls beside this wire ($_CONTROLS) are missing or unreadable," >&2
    echo "   so nothing here was shown able to fail. This run decided nothing." >&2
    exit 2
  fi
  # shellcheck source=/dev/null
  . "$_CONTROLS"
fi

# ---- THE REAL TREE ----------------------------------------------------------
# No `|| true` here. It was redundant — `_scan` returns 0 unconditionally and
# reports every failure as an `err` record — but it is exactly the token that
# made the stored-path arms' statuses look deliberately discarded (#501 R89),
# and a swallow that currently swallows nothing is the one that stops being
# noticed when it starts to.
_verdict "$(_scan "$REL_DIR" ${REL_FILE:+"$REL_FILE"})"
if [ "$SCANNED" -eq 0 ]; then
  echo "!! read 0 stored objects or files; this wire would report no violation for a reason that is not 'there are none'" >&2
  exit 2
fi
# The population is git's: tracked, plus untracked minus ignored. An EMPTY file
# is in it -- its name is still part of the tree (#501 R79).
echo "  read $SCANNED stored object(s)/file(s) under the generic core — the index, HEAD and the working tree, in full"
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
