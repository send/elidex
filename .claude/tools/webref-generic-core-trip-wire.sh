#!/usr/bin/env bash
# Layering trip-wire for the generic webref core (`.claude/tools/_webref/`
# plus the `.claude/tools/webref` entry script) — plan-memo
# 2026-07-citation-hygiene-Ai-spec-label-map.md §2 invariant K2, which that
# memo's §12(3) names as an exit criterion.
#
# WHAT THIS WIRE ENFORCES IS K2, AND K2 IS STRICTER THAN `DESIGN.md`.  Say it
# that way round, because the reverse — deriving the predicate from
# `DESIGN.md` — does not work, and a revision of this header tried.
#
# `_webref/DESIGN.md` in full, both ends.  It OPENS (`:3-5`):
#
#     "`webref` is maintained inside elidex for now, but ITS DRIFT-DETECTION
#      CORE should stay generic enough to move to a standalone repository
#      later.  ELIDEX SPECIFIC BEHAVIOR BELONGS IN THIN ADAPTER COMMANDS."
#
# and CLOSES with:
#
#     "keep new generic behavior free of elidex-specific file paths and put
#      elidex policy in adapter commands or documentation."
#
# …and its §Architecture (`:24-47`) is stronger still: it draws a two-column
# boundary and NAMES the modules — "Generic core: upstream fetch/cache; semantic
# inventory construction; semantic diff classification; stable JSON output
# schema" versus "elidex adapter: repository citation scanning; …; impacted
# `docs/`/`crates/` path heuristics; elidex-specific agent briefs", with
# `commands/agent_brief.py` listed as the module that "scans elidex paths".
#
# ⚠ SO `DESIGN.md`'s "generic core" AND K2's ARE DIFFERENT SETS: the modules
# quoted above versus `_webref/` plus this entry script, which CONTAINS the adapter
# `DESIGN.md` deliberately put there.  A review round put that as "the predicate forbids
# something its own authority permits", a revision of this header answered it
# by quoting the opening sentence with its subject changed from "its
# drift-detection core" to "the package" and omitting the next sentence
# entirely, and that answer is WITHDRAWN.  The tension is real.
#
# WHAT RESOLVES IT is that this wire does not enforce `DESIGN.md`.  It enforces
# K2, as #501's §2 states it: no `.claude/(skills|tools)/<a>/<b>` string
# anywhere in `_webref/` plus this entry script — adapter commands and the
# package's own documentation INCLUDED.  That is a WIDENING of `DESIGN.md`'s
# rule, taken deliberately, on this ground: such a string does not move
# whoever wrote it, so the core/adapter line `DESIGN.md` draws does not help
# anyone deciding whether the package can be lifted out.  The cost is equally
# plain — a thin adapter command implementing elidex policy may not SPELL a
# two-segment host path and must reach the host some other way.  Whether
# `DESIGN.md` should be amended to say so belongs to whichever slice owns
# `_webref/`; it is not this wire's to decide.
#
# ⚠ AND THE POLICY CLAUSE IS UNREACHED EITHER WAY.  Nothing mechanical covers
# "elidex policy" — it is a judgement about wording, and its instrument is diff
# review.  #501 found that half violated in a commit set this wire was green
# on, five rounds running, so do not read a green here as `DESIGN.md`
# compliance.
#
# ONE CHECK — AND IT HAS TWO PREDICATES WITH DIFFERENT EPISTEMIC STATUS.  This
# distinction is the conclusion of four review rounds, every one of which found
# a boundary defect in the SAME half, so it is stated before anything else:
#
#   * `$K2RE_PATH`, over a STORED PATH (an entry's own name, a symlink target),
#     is CLOSED AND DECIDABLE.  git hands the value over whole, `/` is the only
#     delimiter, and a quote, a space or a newline inside a segment is data.
#     There is no judgement in it.  It is the absolute.
#   * `$K2RE`, over RUNNING TEXT, is a BOUNDED HEURISTIC and calling it an
#     absolute is what kept this file wrong.  "Does a path reference start and
#     end here?" cannot be decided without knowing the language the bytes are
#     in — prose, code, Markdown, a URL, a `.pyc` — so the predicate encodes
#     stated rules, each an approximation: a prose delimiter before it; no
#     closing punctuation ending its final segment; and a segment FOLLOWED BY
#     `/` runs to that `/`, so a quoted one-segment reference directly followed
#     by a `/`-bearing token (`".claude/tools/webref","/tmp"`) reads as a hit —
#     the safe direction, and pinned by a control.
#
# ⚠ THE CLAIM WAS THE DEFECT, NOT THE REGEX.  Four rounds ran as "the predicate
# is absolute, so this counter-example is a bug to repair", and each repair was
# aimed at the example: too loose (prose in parentheses reddened the gate), too
# tight (a comma inside a segment), too tight again (a comma before a slash),
# then wrong in kind (`@` treated as a boundary because the rule was written as
# "not these few path characters" instead of "one of these prose delimiters").
# Naming it a heuristic does not weaken what the wire DOES — it still reds on
# anything shaped like a host path — it stops the next boundary case being
# evidence that the file is broken, and makes the real requirement explicit:
# **a boundary rule needs a control in BOTH directions**, because each of those
# four was invisible to a control that only tested the other way.
#
# A NON-ZERO STATUS IS NOT A SPECIFIC NEGATIVE.  This is the invariant three
# rounds of external review kept finding violations of, one site at a time, so
# it is named here once and audited rather than rediscovered:
#
#     A command's failure may be read as a PARTICULAR negative ("there is no
#     HEAD", "this path is untracked") only when a SEPARATE, POSITIVE test
#     establishes that negative.  Otherwise it is an ERROR.
#
# The reason it kept biting is that the wrong reading is always the convenient
# one — it turns "I could not find out" into "there is nothing to find", which
# is exactly the direction that makes a gate green.
#
# ⚠ THE AUDIT — every status this file reads, and what it concludes. It is
# re-derived, not appended to: an earlier version of this table named
# `rev-list -n1 --all` as the positive test for unbornness, and a later round
# replaced that mechanism entirely while the table went on describing it, so
# the audit meant to stop site-by-site recurrence was itself stale one round
# after it was written. Derive the population rather than trusting the list:
#
#     grep -n '=\$?\|^ *if ! _git\|elif ! _git\|)" || \|cmp -s' <this file>
#
#   INFERS A PARTICULAR NEGATIVE, WITH A POSITIVE TEST FOR IT:
#     * `rev-parse --verify --quiet HEAD` non-zero -> maybe unborn. Established
#       positively by `symbolic-ref -q HEAD` naming a branch AND `show-ref
#       --verify` reporting that branch ABSENT (its documented rc 1). Anything
#       else is an `err`.
#     * `symbolic-ref -q HEAD` non-zero -> HEAD names no branch. That is not
#       read as "unborn": it is an `err`, because a malformed ref lands here
#       (measured: rc 128).
#     * `show-ref --verify --quiet` rc 1 -> the named ref is absent. ⚠ rc 1 is
#       show-ref's documented "not found"; a ref file that EXISTS but holds
#       malformed data also fails, which is why the malformed case is caught one
#       level up by `symbolic-ref` rather than here.
#     * `_ancestor_link` rc 1 -> no proper ancestor is a symlink. Pure shell,
#       no external status; the negative is established by the walk itself.
#     * `_absent` rc 0 -> the path is absent, established by a walk to the
#       nearest existing ancestor; rc 1 is "not established", i.e. an `err`.
#   INFERS A PARTICULAR NEGATIVE FROM A DOCUMENTED FAILURE, WITH NO SEPARATE
#   TEST — so the arm carries whatever else can fail that way:
#     * `ls-files --error-unmatch --literal-pathspecs` non-zero -> untracked.
#       `--literal-pathspecs` is what makes the question about THIS path. A
#       repository this call cannot read at all lands in the same arm; what
#       keeps that from becoming a green is that the arm emits an `err` either
#       way.
#     * `[ -L ]`, `[ -f ]`, `[ -e ]` on the worktree path -> "not that kind of
#       entry". Their JOINT failure is NOT read as absence (it is EACCES too);
#       `_absent` above is what answers that.
#   CONCLUDES ONLY FROM A DOCUMENTED CONTRACT (not an inference):
#     * `grep` in `_content`, `_match_path` and `_classify` — 1 = no line
#       selected, >= 2 = error, and every arm separates them.
#     * `cat` of a staged symlink blob — its status travels in the `R%d`
#       sentinel, because the substitution's own status is the sentinel's.
#   CONCLUDES NOTHING BUT "ERROR" (no negative is inferred at all):
#     * `cat-file blob`, `tr -d '\000' | cmp -s`, `readlink`, the three
#       `ls-files`/`ls-tree` inventories, `mktemp`, `rev-parse --local-env-vars`,
#       `_phys` (a `cd` that fails leaves an empty value and the run refuses),
#       and `: >` on each of the walk's temp files.
#       ⚠ The `tr | cmp` arm's MESSAGE names a NUL; any other failure of that
#       pipeline lands there too. The verdict (an `err`) holds either way.
#   NOT READ AT ALL:
#     * `$(_scan …)` — a substitution's status is its own; that the walk
#       finished is asserted from the terminal record in `_verdict`.
# A new `git` call added below joins this table or it is a defect — and the
# table is re-derived when a mechanism changes, not amended around it.
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
# WHAT THIS WIRE DOES NOT DECIDE. Anything added to this list is added HERE.
# ⚠ AND THAT RULE WAS BROKEN TWICE BEFORE IT WAS KEPT: two rounds added a
# declared non-coverage at its own site (`_match_path`'s unpinned guard, the
# ratchet's `wc -l`) and a third narrowed the leading boundary into a new
# undecided class, none of which arrived here. Items 6 and 7 below are those.
# The list is where a reader looks; a ⚠ beside the code is not the list.
# ⚠ THAT CLAIM WAS FALSE WHEN FIRST WRITTEN, and the correction is the reason
# to trust it now.  It said "A-i's memo §12(3) delegates here", citing that
# row's phrase "its header says so" — which is scoped to the OPEN part and
# whose "four failed attempts" are the deleted seed's four failed WIDENINGS,
# not these classes.  Meanwhile §12(3) restated two of them itself and §12(4)
# booked all four as defer slots.  The stacked PR that owns this header swept
# all three sites, so the delegation the sentence asserted now exists.
# Items 1-6 are the reach of a predicate, not work someone owes later; item 7
# is the part that is owed, and it is booked in the plan memo's §8.  What
# covers the rest is the diff: `git diff origin/main...HEAD -- .claude/` is
# finite, and it is what a reviewer reads.
#
#   1. THE POLICY CLAUSE of `DESIGN.md`'s closing rule (above).  Not a path
#      question at all; no grep decides it.
#   2. A SEGMENT CONTAINING WHITESPACE in running text
#      (`.claude/skills/team name/rule.md`).  §2 admits such a path; in
#      arbitrary text nothing says where it ends without quoting rules.  ⚠ A
#      STORED path has no such ambiguity and IS covered — see `$K2RE_PATH`.
#   3. A BARE TOP-LEVEL NAME with no separator (`"docs"`, `"crates"`,
#      `"CLAUDE.md"` as standalone tokens).  Two instances pre-exist at this
#      slice's base: `cli.py`'s `--paths` default and `refresh.py`'s usage
#      string.
#   4. INTERPOLATION — `docs/${x}/y.md`, where the path exists only once the
#      program runs.
#   5. A `.claude/(skills|tools)/` path with ONE further segment — of which
#      `.claude/tools/webref`, this package's own entry script, is the live
#      case.  It is LIVE, not hypothetical — derive it rather than trusting a
#      count here, since the count moves with the package:
#
#        LC_ALL=C grep -rc '\.claude/tools/webref\b' \
#          .claude/tools/_webref .claude/tools/webref | grep -v ':0$'
#
#      The bulk of it is `cli.py`'s `--help` examples; `DESIGN.md`,
#      `__init__.py` and `commands/refresh.py` each carry some too.
#
#      ⚠ THIS ONE IS DECIDABLE AND IS STILL NOT DECIDED, which is why it is
#      listed apart from 3 and 4.  §2's predicate takes TWO further segments,
#      so it cannot see these; widening it to one would red the package on
#      every mention of its own entry point, and "the tool naming how it is
#      invoked" is not the class K2 exists for.  What is NOT claimed is that
#      the wire looked and found nothing: it never looked.  Narrowing this
#      (e.g. "any one-segment path OTHER than this package's own entry") is a
#      predicate change and belongs to whoever proposes it, with its own
#      control.
#
#   6. A REFERENCE WHOSE LEADING CHARACTER IS ONE THIS PREDICATE TREATS AS PART
#      OF A COMPONENT — the exclusion class in `$K2RE` is the list, and it is
#      not restated here. `foo@.claude
#      /skills/team/rule.md` is not decided as a hit, on the reading that it
#      continues the component `foo@.claude`.  ⚠ This class exists BECAUSE the
#      leading boundary is an exclusion (see `$K2RE`): every character added to
#      the exclusion set to kill a false positive lands here.  It is the price
#      of failing safe, and it is listed rather than left implicit.
#   7. WHAT NO CONTROL PINS, named here so "the list" is the list:
#      `_match_path`'s `|| return 4` (nothing external is left in `_onerec` for
#      a shim to break); the `wc -l` in the controls' ratchet (reaching an
#      empty `.bare` needs every control to have a record); the `-a` on
#      `_verdict`'s arms (a NUL cannot reach a shell string); `-c
#      core.untrackedCache=false` in `_git`; the terminal record on `_scan`'s
#      early return (no control makes `$SCRATCH` unwritable); and the verdict
#      sites with no control of their own.  Each is recorded at its own site as
#      well.  Of these, `|| return 4`, the `wc -l` and the verdict sites are
#      booked as defer slots in the plan memo's §8; the others are accepted
#      where they stand, with the reason beside them.
#
# ⚠ 3 AND 4 ARE NOT CLOSABLE BY ANY WIRE, and saying so is the point: both are
# properties of a grep over arbitrary source text, so "later, with a better
# predicate" is not a plan, it is the seed this wire already deleted.  An
# earlier revision ran a wide `<any top-level entry>/<something>` SEED —
# printed, never asserted.  It was widened four times, each fix opening the
# next hole (syntactic over-reach; resolving-on-disk under-reach; a
# string-keyed exemption that over-suppressed; a basename-keyed one that
# collided), and interpolation was the fifth.  This repo's recorded rule is
# that when a predicate cannot return its population it is a SEED and widening
# the regex is the wrong repair.  A seed that reports and asserts nothing is
# also a print with no consumer, which `CLAUDE.md` calls dead code.  So it is
# GONE, not demoted.
#
# RUNTIME: NOTHING TO INSTALL, and bash 3.2 compatible.  That is a contract,
# not a coincidence: the property the ungated `trip-wires` job rests on is the
# ABSENCE OF A SETUP STEP — no language runtime, no package manager, no cache,
# no network — stated canonically at that job in `.github/workflows/ci.yml` and
# restated in `CLAUDE.md`.  An earlier revision of THIS wire used `python3` and
# broke that premise (#501 R69), which is the concrete thing to avoid here.
# ⚠ STATED AS A PROPERTY, NOT A LIST, because two lists have already been wrong
# at these same sites: "the wires are grep-only" (this wire calls `git`
# constantly) and then "the shell, `git` and `grep`" (both left out tools these
# files call).  The second was written by the edit retiring the first, which is
# why no third list is offered.
# ⚠ AND THIS PARAGRAPH IS ABOUT THIS WIRE, NOT THE WIRE SET.  A revision of it
# ended "anything needing more than the shell, git and grep belongs in a test,
# not here", which — sitting beside a sentence about the job — reads as a rule
# for the SET, i.e. as an answer to the open question of whether the required
# ungated wire set may require an interpreter.  That question is contended
# (PR #510) and is not settled by a comment in one wire; see the A-i-wire plan
# memo §6.
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
# ⚠ AND NO CALLER-SUPPLIED GREP ARGUMENTS. BSD grep (macOS) places
# `GREP_OPTIONS` at the start of its argument list (`man grep`, ENVIRONMENT),
# so `GREP_OPTIONS=--exclude=*` makes a named file read as "no match" (measured
# on this grep) and a local run went green over a violation (plan memo §11, D1).
unset GREP_OPTIONS
# ⚠ THE TWO GIT READING SWITCHES LIVE IN `_git`, NOT HERE, and that is the
# whole of it — see the `export` inside it. They were stated at BOTH levels
# until the mutation set (below) measured what each level is worth, and the
# answer was asymmetric in a way nobody would guess: `GIT_NO_REPLACE_OBJECTS`
# IS in `git rev-parse --local-env-vars`, so the routing purge in `_git` clears
# it and only the re-export inside survives; `GIT_NO_LAZY_FETCH` is NOT, so
# there the outer one was the live copy and the inner was redundant. One value,
# stated twice, load-bearing at a different level each time — which is why the
# mutation aimed at the outer `GIT_NO_REPLACE_OBJECTS=1` SURVIVED: the line it
# changed decided nothing. Every git call that READS THE TREE goes through
# `_git`, so after the purge is the one place that holds for all of them.

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
# ⚠ ENTERED BY ARGUMENT, NEVER BY ENVIRONMENT. Self-test mode points `ROOT` at
# a fixture AND skips every control, so it must be unreachable from an ordinary
# run. It used to be entered by exporting `WEBREF_WIRE_SELFTEST`, and every
# companion value meant to prove "the controls started this" was satisfied by a
# shell that exported both: first a literal token (it is in this file, so
# copying the lines reproduced it), then the parent's PID (a long-lived shell
# that exports `$$` IS the parent of everything it later launches, including
# `scripts/trip-wires.sh` — reproduced by the external reviewer, PR519 R8:
# every control skipped, a clean fixture scanned, `PASSED`). The environment is
# inherited by definition, so no value placed in it can tell those apart;
# arguments are not inherited, and the driver passes none. A leftover export
# of the old names is now simply not read.
# ⚠ THE RULE FOR EVERY MODE THIS WIRE CAN BE PUT INTO FROM OUTSIDE: unreachable
# or loud, never silent. This one is now unreachable; `WEBREF_WIRE_MUTANTS` is
# loud (it runs before the verdict, not instead of it — see the mutation file).
_SELFTEST=""
if [ "${1:-}" = "--selftest" ]; then
  if [ ! -d "${2:-}" ]; then
    echo "!! --selftest needs a fixture root that is a directory (got '${2:-}')." >&2
    echo "   This run decided nothing." >&2
    exit 2
  fi
  _SELFTEST="$2"
  ROOT="$_SELFTEST"
  # A fixture may name a scope SUBDIRECTORY and an EXTRA ENTRY beside it, both
  # relative to the root, so a control can reproduce the real geometry — the
  # scope directory and the `webref` entry script are SIBLINGS, and an extra
  # entry inside the scope would be caught by the directory walk anyway, proving
  # nothing about its own arm.
  SCOPE_DIR="$ROOT/${3:-.}"
  SCOPE_FILE="${4:+$ROOT/$4}"
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
# ⚠ RESOLVED BEFORE THE TRAP IS INSTALLED. GNU `mktemp -d` returns a RELATIVE
# path when `TMPDIR` is relative, and the trap's `/*/*` guard (which keeps an
# empty or root-ish value away from `rm -rf`) then matched nothing, so every
# run left its scratch — and in a normal run the whole controls fixture tree —
# behind (PR519, reproduced by the external reviewer with `TMPDIR=tmp`). The
# trap only ever sees the physical absolute path, which is also what the
# in-tree check below compares.
_phys() { ( cd "$1" 2>/dev/null && pwd -P ); }
_raw_scratch="$(mktemp -d)" || { echo "!! no scratch dir (TMPDIR/disk?), so nothing here was proved" >&2; exit 2; }
# ⚠ `|| SCRATCH=""`: under `set -e` a failing substitution exits HERE, before
# the guard below can say why (a restrictive umask does it — plan memo §11, D5).
SCRATCH="$(_phys "$_raw_scratch")" || SCRATCH=""
trap 'case "$SCRATCH" in /*/*) chmod -R u+rwX "$SCRATCH" 2>/dev/null || true; rm -rf "$SCRATCH";; esac' EXIT
if [ -z "$SCRATCH" ]; then
  rmdir "$_raw_scratch" 2>/dev/null || true
  echo "!! could not resolve the scratch dir ($_raw_scratch) to a physical path, so nothing here was proved" >&2
  exit 2
fi
_scratch_p="$SCRATCH"
# …and the same for the root, for the same reason.
_root_p="$(_phys "$ROOT")" || _root_p=""
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

# §2's K2 predicate over running text. Fixed ERE, `grep -E`. This and
# `$K2RE_PATH` below are what this wire asserts.
#
# A segment FOLLOWED BY `/` runs to that `/`, stopping only at whitespace: a
# `]`, a quote or a backtick inside it is part of the path, and while they ended
# it `.claude/tools/team]inc/rule.md` was missed (plan memo §11, D3). Only the
# segment that ENDS the match stops at the characters that end a path in
# running text — whitespace, `]`, quotes and a backtick. Past the first segment
# the pattern is an ALTERNATION of those two readings, so a segment that could
# be either (`…/a/]x/`) matches: the fail-safe reading.  The segment class was
# `[A-Za-z0-9_.-]` until #501 R74, which excluded segments §2
# admits — `.claude/tools/@scope/policy.md` and `.claude/skills/日本語/rule.md`
# both read GREEN.  ⚠ A segment containing WHITESPACE stays outside it — item 2 of
# WHAT THIS WIRE DOES NOT DECIDE above, stated there and not restated here.
# ⚠ TWO REPAIRS THE EXTERNAL REVIEWER FOUND, BOTH FALSE-POSITIVE DIRECTIONS —
# which in a REQUIRED gate is the direction that gets gates switched off.
#   * CLOSING PUNCTUATION MAY NOT END A SEGMENT — but it may sit INSIDE one.
#     `)]}>,;` became terminators because the harmless prose
#     `See (.claude/tools/foo/) for details` matched with `)` as the final
#     segment, rejecting a ONE-segment reference outside K2 entirely. ⚠ The
#     first repair excluded those characters from the segment ENTIRELY, which
#     bought the false positive back as a FALSE NEGATIVE: a real path
#     `.claude/tools/team,inc/rule.md` stopped at the comma and read K2 zero.
#     ⚠ …and the SECOND repair over-reached in turn, by applying the rule to
#     BOTH segments: `.claude/tools/team,/rule.md` is a valid path whose first
#     segment ends in a comma, and it went undetected. The restriction belongs
#     ONLY on the FINAL segment, because that is the only place the end of the
#     reference is ambiguous — an intermediate segment is delimited by `/`,
#     which settles it. So: first segment takes any path characters, the last
#     may not END in punctuation. Admits `team,inc` and `team,`; still refuses a
#     bare `)`. The green control pins one direction and a red control the
#     other; neither alone could have caught either of these.
#   * A LEADING BOUNDARY is required, so `.claude` must not continue another
#     path component. Without it `https://example.claude/skills/team/rule.md`
#     matched on the suffix of another host name.
#
#     ⚠ THE SHAPE OF THIS CLASS IS A SAFETY DECISION, NOT A TASTE ONE, and
#     getting that backwards is the worst thing this file has done. Neither form
#     can enumerate its complement — "which characters end prose" is open either
#     way — so the question is not which list is complete but WHICH DIRECTION
#     AN UNKNOWN CHARACTER FAILS IN:
#
#       an EXCLUSION class ("a boundary is anything that is not a path
#       character") makes the unknown character a BOUNDARY -> over-matching ->
#       a FALSE POSITIVE -> the gate reds, somebody looks, somebody fixes it.
#
#       a POSITIVE list ("a boundary is one of these") makes the unknown
#       character NOT a boundary -> under-matching -> a FALSE NEGATIVE -> the
#       gate is green and nobody ever finds out.
#
#     In a REQUIRED gate those are not symmetric, so this class is an EXCLUSION.
#     ⚠ A revision replaced it with a positive list to close ONE contrived false
#     positive (`foo@.claude/...`), and measured against the revision before it
#     that traded the safe direction for the unsafe one: `DEFAULT=.claude/...`,
#     `--paths=.claude/...`, `k:.claude/...`, `` `.claude/...` `` and
#     `**.claude/...**` all stopped matching, and no control saw it. ⚠ WHICH
#     SPELLINGS THE SCANNED TREE ACTUALLY USES IS NOT A LIST HERE — an earlier
#     revision named `--opt=` and a Markdown backtick, and neither is in the
#     tree. Derive it:
#
#       LC_ALL=C grep -rnoE '.{0,3}\.claude/(skills|tools)/' \
#         .claude/tools/_webref .claude/tools/webref | sort | uniq -c
#
#     The fixtures cover those shapes regardless, because what the tree spells
#     today is not what a contributor writes tomorrow.
#     The `@` false positive is handled where it belongs: INSIDE the exclusion
#     set, beside the other characters that continue a component (`+`, `%`,
#     `-`).
# The leading class is consumed by the match, so a record shows one extra
# character; that is cheaper than a lookbehind ERE does not have.
K2RE='(^|[^A-Za-z0-9_.~@+%-])\.claude/(skills|tools)/[^/[:space:]]+/([^/[:space:]]+/|[^]/[:space:]"'"'"'`]*[^]/[:space:]"'"'"'`)}>,;])'

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
# ⚠ AND THE SAME LEADING-BOUNDARY REPAIR. A stored path has only `/` as a
# delimiter, so the class is wider — but `.claude` still may not be the SUFFIX
# of another segment: a fixture named `_webref/fixtures/example.claude/skills/
# team/rule.md` was reported as naming the top-level host path it does not.
K2RE_PATH='(^|/)\.claude/(skills|tools)/[^/]+/[^/]+'

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
# ⚠ THE ONE RULE FOR EVERYTHING GIT CAN HOLD AND THIS READER CANNOT REPRESENT:
# it is an ERROR and the gate goes red. Not a special reader, not a silently
# narrowed value. A mode-120000 blob holding a NUL is the case that made the
# rule explicit (#501 R94): `$( )` DROPS NULs, so it arrived as a clean path and
# read GREEN — the danger being a silent misread, which is why detecting it and
# erroring is fail-closed rather than hardening.
# ⚠ THIS IS NOT A BOUND ON THE FILE, and an earlier revision claimed it was
# ("the threat model is accident, not adversary — and saying so bounds this
# file"). The file grew afterwards, and the NUL arm that sentence was written to
# justify IS a mechanism, so the claim was falsified by its own neighbourhood.
# What is true, and all that was ever needed: a contributor who wants past this
# gate edits `REQUIRED_WIRES` in `scripts/trip-wires.sh`, which that file's own
# comment names as the one edit that genuinely disables it — so this wire does
# not trade reading correctness for adversary-resistance it could not have
# anyway. Every "unknown fails closed" decision in this file is that same
# decision.
#
# A permission failure is an ERROR, not an absence: `git ls-files` reports it
# on stderr while exiting 0, so the inventories' stderr is collected and a
# non-empty one becomes an `err` record. `grep`'s own stderr is discarded; what
# carries its failures is its status above 1.
# A PATH IS DATA, NOT PROTOCOL. The records below are newline-separated and
# tab-tagged, and a tracked filename may contain both — so a file named
# `safe<LF>k2<TAB>forged` injected a synthetic K2 hit and the wire reported
# `forged` and exited 1 (#501 R80, reproduced). Every path is escaped on its
# way into a record; the escape is lossy on purpose, since what a reader needs
# is to find the entry, not to round-trip its bytes.
# EVERY `git` CALL THAT READS THE TREE GOES THROUGH HERE — the exception is the
# `rev-parse --local-env-vars` call just below, which runs before this function
# can exist, since it is what builds the list the function uses — because
# `-C "$ROOT"` does NOT
# win over the repository-routing environment: with `GIT_DIR`/`GIT_WORK_TREE`
# exported — a wrapper, a hook — the inventory described ANOTHER CHECKOUT while
# the worktree arm read files under `$ROOT`, and a fixture holding a forbidden
# path reported one clean entry and exited 0 (#501 R95, reproduced).
# ⚠ The list is git's own (`rev-parse --local-env-vars`), not a hand-written
# one: enumerating this by hand is how the next variable gets left authoritative.
# An empty or failing list means we cannot know what routes git, so the run
# decides nothing rather than guessing.
# ⚠ ROUTING ONLY — AND GIT'S LIST IS NOT ROUTING-ONLY. It also names
# `GIT_CONFIG`, `GIT_CONFIG_PARAMETERS` and `GIT_CONFIG_COUNT`, which are
# CONFIGURATION INPUTS, so unsetting the list wholesale repeated R94's defect by
# another route: a foreign-owned checkout authorised through
# `GIT_CONFIG_COUNT=1 / GIT_CONFIG_KEY_0=safe.directory / GIT_CONFIG_VALUE_0=*`
# worked under a direct `git -C <repo> ls-files` and exited 2 here, after
# reading nothing (#501 R97). The `GIT_CONFIG*` family is held back.
# ⚠ THE RISK DIRECTIONS ARE NOT SYMMETRIC, which is why the exemption is this
# narrow. Clearing too much costs the caller's configuration and the wire then
# REFUSES to run — loud. Clearing too little leaves git routed at another tree
# and the wire ANSWERS about it — silent, and wrong. So the default is to clear,
# and `GIT_CONFIG*` is the one retreat, taken against a reproduced setup.
_GIT_LOCAL_VARS="$(git rev-parse --local-env-vars 2>/dev/null)" || _GIT_LOCAL_VARS=""
if [ -z "$_GIT_LOCAL_VARS" ]; then
  echo "!! this git cannot say which variables route it (rev-parse --local-env-vars)," >&2
  echo "   so this run could not prove it read the tree it was pointed at." >&2
  exit 2
fi
_git() { ( for _v in $_GIT_LOCAL_VARS; do
             case "$_v" in GIT_CONFIG*) : ;; *) unset "$_v" ;; esac
           done
           # ⚠ NO NETWORK. In a blobless partial clone an indexed blob may be
           # PROMISED rather than local, and `git cat-file` will then fetch it
           # on demand — measured, the run spawned
           # `git fetch origin --filter=blob:none` and `git-upload-pack` (#501
           # R94). That contradicts this gate's own contract (`.github/
           # workflows/ci.yml`: nothing to install, no cache, no network) and
           # would make a required local gate depend on credentials,
           # connectivity and an unbounded remote operation. With lazy fetching
           # off an absent blob simply fails the read and becomes the `err`
           # record that already exists — unknown fails closed, as everywhere.
           # ⚠ A git too old to know the variable ignores it, and then the
           # fetch is back. Nothing here can detect that, and saying so is the
           # honest position.
           # ⚠ AND NO REPLACEMENT OBJECTS. A local `replace` ref — history
           # repair leaves them — makes `cat-file` hand back a DIFFERENT object
           # than the one the index and the commit name. Reproduced: a staged
           # blob holding `.claude/skills/team/rule.md` replaced by a clean blob
           # read K2 zero and exited 0, while the same read with this variable
           # set showed the violation (#501 R95). What is committed is the
           # object the index names, so that is the object this wire reads.
           # ⚠ AFTER THE PURGE ABOVE, WHICH IS WHY IT IS HERE: git's own list
           # names `GIT_NO_REPLACE_OBJECTS`, so the loop clears it.
           # ⚠ AND IT IS PRESENCE-CHECKED, NOT PARSED: git disables replacement
           # if the variable is SET AT ALL, so `GIT_NO_REPLACE_OBJECTS=0` does
           # NOT turn it back on. Measured while building the mutation set —
           # the entry that sets it to 0 survives, and the one that REMOVES the
           # assignment kills. Anyone "switching this off" to debug has to
           # unset it.
           export GIT_NO_LAZY_FETCH=1 GIT_NO_REPLACE_OBJECTS=1
           # ⚠ AND NO FILESYSTEM MONITOR OR UNTRACKED CACHE. `GIT_CONFIG*`
           # survives the purge above on purpose, so it can carry
           # `core.fsmonitor`, which names a command git RUNS during this
           # wire's own `ls-files` and `cat-file` calls (measured) and whose
           # answer git takes for what the worktree holds (plan memo §11.6).
           # `-c` wins over `GIT_CONFIG_COUNT` (measured), so the caller's
           # other keys, such as `safe.directory`, still reach git. The
           # untracked cache is caller state of the same kind; it is turned off
           # here on that ground, and no control pins that half.
           exec git -c core.fsmonitor=false -c core.untrackedCache=false "$@" ); }

# ⚠ PARAMETER EXPANSION, NOT A PIPELINE, and the reason is the always-run job.
# These two are the hottest things in the file — called per entry per source —
# and as `printf | sed | tr` / `printf | tr` they were **306 of the real scan's
# 592 process spawns**. In-shell they cost none, and the scan measured ~25%
# faster with byte-identical output (verified by diff over the whole record
# stream, and by the `forge` / `nlname` / `quotename` controls, which exist
# precisely to exercise these characters).
# ⚠ THE REPLACEMENT MUST COME FROM A VARIABLE, UNQUOTED, and that is not
# stylistic. A literal `${v//$'\n'/~}` is TILDE-EXPANDED — it substitutes the
# home directory — and the obvious repair, quoting it as `"$_T"`, emits LITERAL
# QUOTE CHARACTERS under bash 3.2, which this wire commits to. `_T=$'~'` used
# unquoted is the one form correct on both; verified on 3.2.57 and 5.x over
# `a\b`, an embedded LF, `safe<LF>k2<TAB>forged`, a TAB, `~home` and the empty
# string.
_ESC_T=$'~'
_REC_SEP=$'\001'
# Is any PROPER ancestor component of $1 (relative to $ROOT) a symlink? The
# entry itself is not tested — a symlinked entry is a case the arms above own.
_ancestor_link() {
  _al_rest="$1"; _al_pre=""
  while [ "${_al_rest#*/}" != "$_al_rest" ]; do
    _al_head="${_al_rest%%/*}"; _al_rest="${_al_rest#*/}"
    _al_pre="${_al_pre:+$_al_pre/}$_al_head"
    [ ! -L "$ROOT/$_al_pre" ] || return 0
  done
  return 1
}

# IS $1 (relative to $ROOT) PROVABLY ABSENT? rc 0 = yes, 1 = not established.
# `[ -L ] / [ -f ] / [ -e ]` all answer "no" when the lookup fails with EACCES,
# so their joint failure is not a negative (plan memo §11, D2: a tracked file
# under a directory with read but no search permission read as tracked-and-gone,
# with no record, green). The negative is established here instead: walk up to
# the NEAREST EXISTING ancestor. If it is a directory it must be searchable, and
# then the component below it failing lstat is a real absence; if it exists and
# is NOT a directory (a tracked directory replaced by a file), the lookup below
# it is ENOTDIR, which is an absence too. Walking past missing ancestors is what
# keeps a tracked directory deleted wholesale — a routine unstaged state — green.
_absent() {
  _ab_p="$ROOT/$1"
  while _ab_up="${_ab_p%/*}"; [ "$_ab_up" != "$_ab_p" ] && [ -n "$_ab_up" ]; do
    _ab_p="$_ab_up"
    [ -e "$_ab_p" ] || [ -L "$_ab_p" ] || continue
    [ -d "$_ab_p" ] || return 0
    [ -x "$_ab_p" ] && return 0
    return 1
  done
  return 1
}

_esc() { _e="${1//\\/\\\\}"; _e="${_e//$'\n'/$_ESC_T}"; printf '%s' "${_e//$'\t'/$_ESC_T}"; }

# A stored path as ONE record: newline is data inside a segment, not a
# separator. `\001` is chosen because no predicate here mentions it.
_onerec() { printf '%s' "${1//$'\n'/$_REC_SEP}"; }

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
# ⚠ NO PIPELINE, AND THE ASSIGNMENT'S STATUS IS RETURNED. It was
# `_onerec "$1" | grep …`, and under `pipefail` a pipeline's status is its
# RIGHTMOST non-zero one — so a pre-processing failure arrived as `grep`'s 1,
# i.e. as an ordinary "no match", and an entry whose own name is a forbidden
# path passed the gate.
# ⚠ REMOVING THE PIPELINE WAS NOT ENOUGH, and the first repair stopped there.
# `_match_path` is called beneath `||` in `_stored`, where `errexit` is
# suspended, so a failing assignment simply fell through to `grep`, which
# returned 1 for the empty value — the same misclassification by another route.
# Measured by the external reviewer: replacing `_onerec` with `return 2` let an
# entry named `.claude/skills/team/rule.md` report `K2: 0` and exit 0. `|| return 4`
# is what makes the failure reach `_stored`'s `> 1` arm.
# ⚠ AND NO CONTROL PINS IT, which is worth saying rather than leaving to be
# discovered. `_onerec` is parameter expansion now — there is no external
# command left in it for a PATH shim to break — so the only way to reach this
# arm is to edit the function, which is a mutation rather than an input. The
# reviewer's reproduction did exactly that. Deleting `|| return 4` therefore
# reds nothing today; it is kept for the shape one refactor away (an `_onerec`
# that shells out again), where it becomes load-bearing silently. Same standing
# as the `-a` on `_verdict`'s arms below, and recorded in the same words.
_match_path() { _mp="$(_onerec "$1")" || return 4; grep -aEo -- "$K2RE_PATH" <<<"$_mp"; }

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
        # ⚠ THE SENTINEL PRESERVES TRAILING NEWLINES; IT DOES NOT PRESERVE THE
        # STATUS, and an earlier revision read the second from the first. A
        # `cat` that fails makes the substitution succeed with an empty or
        # partial value, which then goes to `_stored` as a successfully-read
        # target — measured by the external reviewer: a staged forbidden target
        # plus a clean worktree target plus a failing `cat` reported K2 zero and
        # exited 0. `R%d` carries both.
        _sb="$(cat "$_b" 2>/dev/null; printf 'R%d' "$?")"
        _catrc="${_sb##*R}"; _sb="${_sb%R*}"
        if [ "$_catrc" -ne 0 ]; then
          printf 'err\t%s: its staged symlink blob could not be read (exit %d)\n' \
            "$(_esc "$rel")" "$_catrc"
        else
          _stored "$_sb" "$rel" "staged symlink TARGET" "$_tag ->"
        fi
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
  elif _ancestor_link "$rel"; then
    # ⚠ BEFORE THE LEAF-TYPE ARMS, not after them. Placed after `[ -L "$f" ]`
    # this guard lost whenever the external leaf was ITSELF a symlink:
    # `dir -> /tmp/external` with `external/a.py -> .claude/skills/team/rule.md`
    # was reported as a K2 hit on the DESCENDANT — a verdict about bytes outside
    # this tree, dressed as a verdict about it. The ancestor question is about
    # whether the path is reachable at all, so it is asked first.
    # An ancestor being a symlink means what is on disk here is NOT what git
    # would carry. The entry is not skipped — that is how things get passed over
    # in silence — it is an `err`, because this run cannot say what the tree
    # holds there.
    printf 'err\t%s: an ancestor component is a symlink, so the worktree bytes here are not this tree'"'"'s\n' \
      "$(_esc "$rel")"
  elif [ -L "$f" ]; then
    # A symlink's stored content IS its target string; git keeps it as the blob.
    # ⚠ AND `$( )` STRIPS TRAILING NEWLINES, so the plain substitution truncated
    # the stored value: a target `.claude/skills/team/<LF>` arrived as
    # `.claude/skills/team/`, whose final segment is empty, so `[^/]+` could not
    # match and the wire exited 0 over a violation git stores verbatim (#501 R90,
    # reproduced). `-n` stops `readlink` adding its own newline, and the `R%d`
    # sentinel keeps the substitution from ending in one — so nothing is stripped
    # and the exit status still reaches us.
    # ⚠ The other stored values reach the predicate by other routes: the entry
    # name comes from `read -r -d ''`, the two match captures hold `grep -o`
    # output whose records cannot end in a newline because `_onerec` removed
    # them, and the staged symlink blob goes through the same `R%d` sentinel
    # above.
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
  elif ! _absent "$rel"; then
    # Neither a file, a link, nor PROVABLY absent (see `_absent`).
    printf 'err\t%s: not a file or a link here, and not provably absent (its nearest existing ancestor is not a searchable directory), so this run never read it\n' \
      "$(_esc "$rel")"
  elif ! _git -C "$ROOT" --literal-pathspecs ls-files --error-unmatch -- "$rel" >/dev/null 2>&1; then
    # ⚠ INVENTORIED, THEN GONE — AND UNTRACKED, so nothing else answers for it.
    # The arms above all test the path as it is NOW, and a path that vanished
    # between `ls-files` and this read matched none of them: no `ok`, no `err`,
    # no record at all. ⚠ THAT IT IS GONE IS `_absent`'s answer, not these
    # arms' joint failure — which is EACCES as readily as absence, and is why
    # the arm above exists. The final guard only requires the AGGREGATE `SCANNED`
    # to be non-zero, so its siblings carried the run to green. Reproduced by
    # the external reviewer: a forbidden untracked file removed immediately
    # after the inventory and restored afterwards gave exit 0.
    # ⚠ A TRACKED path deleted from the worktree is NOT this case — the index
    # pass answered for it with its own record — which is why the membership
    # question is asked of git rather than assumed from absence.
    # ⚠ AND `--literal-pathspecs`, because `$rel` is DATA, not a pattern. Without
    # it git reads the name as a PATHSPEC: a vanished untracked `foo[1].py`
    # matched a tracked `foo1.py` and was reported as tracked, so the entry that
    # nothing had answered for was passed over in silence — the very hole this
    # arm was added to close. Same lesson as `${var#"$prefix"/}` one layer up.
    printf 'err\t%s: the inventory listed it but it is gone, so this run never read it\n' \
      "$(_esc "$rel")"
  fi
  # A tracked path DELETED from the worktree reaches none of those arms; the
  # index pass answered for it with its own record.
  [ "$_read" -eq 0 ] || printf 'ok\t%s\n' "$(_esc "$rel")"
}

_scan() { # $1 = scope dir, $2 = extra file, both RELATIVE to $ROOT
  # ⚠ FIXED NAMES UNDER `$SCRATCH`, NOT `mktemp`. `$SCRATCH` is already this
  # PROCESS's own `mktemp -d` (and the run refuses to start if it sits inside
  # the scanned tree), and `_scan` is called exactly once per process — so five
  # `mktemp` spawns per invocation bought uniqueness that was already
  # guaranteed, once per control.
  # ⚠ The creation is still CHECKED, because "could not write here" must not
  # become a silent empty list — it becomes the same `err` record as before.
  for _t in e lc lo lh b; do
    # ⚠ The terminal record here too (see `_verdict`). No control reaches this
    # arm — nothing here makes `$SCRATCH` unwritable — and both of its shapes
    # exit 2, differing only in the message.
    : > "$SCRATCH/scan.$_t" || { printf 'err\twalk: no temp file (%s) for the walk\n' "$_t"; printf 'end\tscan\n'; return 0; }
  done
  _e="$SCRATCH/scan.e"; _lc="$SCRATCH/scan.lc"; _lo="$SCRATCH/scan.lo"
  _lh="$SCRATCH/scan.lh"; _b="$SCRATCH/scan.b"
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
  _rc_tracked=0
  # `--stage`, not a bare `--cached`: the INDEX MODE is what says a staged blob
  # is a symlink target rather than file content, and asking the worktree
  # instead gets it wrong exactly when the two disagree (#501 R93).
  # ⚠ A conflicted path has stages 1/2/3 and no stage 0, so it appears more than
  # once and `:0:` cannot resolve it — each copy becomes an `err`, which is the
  # right answer: nothing here can say what such a tree would commit.
  _git -C "$ROOT" ls-files -z --stage \
      -- "$_dir" ${_extra:+"$_extra"} > "$_lc" 2>>"$_e" || _rc_tracked=$?
  [ "$_rc_tracked" -eq 0 ] || \
    printf 'err\tthe tracked inventory exited %d, so the population is incomplete\n' "$_rc_tracked"
  # THE WORKING TREE's half is tracked AND untracked together: every path that
  # has bytes on disk right now, whichever list git files it under.
  _rc_worktree=0
  _git -C "$ROOT" ls-files -z --cached --others --exclude-per-directory=.gitignore \
      -- "$_dir" ${_extra:+"$_extra"} > "$_lo" 2>>"$_e" || _rc_worktree=$?
  [ "$_rc_worktree" -eq 0 ] || \
    printf 'err\tthe worktree inventory exited %d, so the population is incomplete\n' "$_rc_worktree"
  # …AND HEAD, because a push sends the COMMIT, not the index (#501 R95). A
  # violation committed and then fixed only in the index read green while
  # `git show HEAD:victim` still held it. Bounded at the TIP and no further:
  # elidex squash-merges, so what lands on main is the tip's tree, and the
  # commits below it are not what this gate is about. An unborn HEAD — a fresh
  # clone before its first commit, and any fixture here that was never
  # committed to — is not an error: there is nothing committed to read.
  # ⚠ EXIT 1 IS "UNBORN"; ANYTHING ELSE IS A FAILURE, and collapsing the two
  # made an operational error silently disable the whole HEAD pass. `--quiet`
  # exits 1 for an unborn HEAD and 128 when git cannot answer at all (measured),
  # so `if <probe>` alone read "no HEAD, nothing committed to check" for both —
  # and a committed forbidden path cleaned in the index and worktree then read
  # green. Found by the external reviewer, who reproduced it with a PATH wrapper
  # returning 2 for this probe only.
  _hrc=0
  _git -C "$ROOT" rev-parse --verify --quiet HEAD >/dev/null 2>&1 || _hrc=$?
  if [ "$_hrc" -ne 0 ]; then
    # ⚠ EXIT 1 IS NOT "UNBORN" EITHER, and treating it as such was the second
    # version of this bug: a repository whose branch ref holds malformed data
    # also exits 1, and the wire then skipped the HEAD inventory and exited 0
    # over a clean index and worktree while HEAD lookup had actually FAILED.
    # So the absence is established POSITIVELY — a repository with no commits
    # at all — and everything else is an error. Measured: truly unborn gives
    # `rev-list -n1 --all` exit 0 with empty output; a malformed ref gives 128.
    # ⚠ AND THE SUBJECT IS *THIS HEAD*, NOT THE REPOSITORY. The first version of
    # this test asked whether the repository held any commit at all
    # (`rev-list -n 1 --all`), which is a different question: on an ORPHAN
    # BRANCH with commits on another branch, HEAD is legitimately unborn and
    # `--all` still returns one, so the gate reported a read error over a tree
    # it had correctly nothing to read. Measured, all three cases:
    #   orphan branch, commits elsewhere : symbolic-ref 0 -> ref absent  = unborn
    #   empty repository                 : symbolic-ref 0 -> ref absent  = unborn
    #   malformed branch ref             : symbolic-ref **128**          = ERROR
    # So: HEAD must name a branch, and that branch must be absent. Anything
    # else is a failure to read, which is what the invariant above demands.
    _srf="$(_git -C "$ROOT" symbolic-ref -q HEAD 2>/dev/null)" || _srf=""
    if [ -z "$_srf" ]; then
      printf 'err\tHEAD could not be resolved and does not name a branch, so the commit was never read\n'
    else
      _shrc=0; _git -C "$ROOT" show-ref --verify --quiet -- "$_srf" || _shrc=$?
      if [ "$_shrc" -eq 0 ]; then
        printf 'err\tHEAD names %s and that ref EXISTS, so this HEAD is not unborn and the commit was never read\n' \
          "$(_esc "$_srf")"
      elif [ "$_shrc" -ne 1 ]; then
        printf 'err\tHEAD names %s but that ref could not be read (show-ref exit %d)\n' \
          "$(_esc "$_srf")" "$_shrc"
      fi
    fi
  fi
  if [ "$_hrc" -eq 0 ]; then
    _rc_head=0
    _git -C "$ROOT" ls-tree -r -z HEAD \
        -- "$_dir" ${_extra:+"$_extra"} > "$_lh" 2>>"$_e" || _rc_head=$?
    [ "$_rc_head" -eq 0 ] || \
      printf 'err\tthe HEAD inventory exited %d, so the population is incomplete\n' "$_rc_head"
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
  # THE TERMINAL RECORD, after the last source. A path cannot forge it: `_esc`
  # maps LF and TAB, so no entry can put `end<TAB>` at the start of a line —
  # the `forge` fixture holds one named `safe<LF>end<TAB>scan`.
  printf 'end\tscan\n'
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
# ONE IMPLEMENTATION OF THE THREE-WAY STATUS RULE, called three times — rather
# than the rule written out three times, which is what `_verdict` used to be.
# The rule is this file's most-cited invariant ("1 is no match, 2 or more is a
# failure, and a failure may not be answered as an absence"), and an invariant
# with three edit sites is three chances to fix two of them. The `-ac` variant
# was also invisible without diffing the copies.
# $1 = what failed (for the diagnostic), $2 = extra grep flags, $3 = pattern.
_classify() {
  _crc=0; _cout="$(printf '%s\n' "$_VERDICT_IN" | grep -a $2 -- "$3")" || _crc=$?
  [ "$_crc" -le 1 ] || { echo "!! the $1 classifier failed (grep exit $_crc); this run decided nothing" >&2; exit 2; }
  printf '%s' "$_cout"
}

_verdict() { # $1 = _scan output; sets K2_HITS / ERR_HITS / SCANNED
  _VERDICT_IN="$1"
  # ⚠ DID THE WALK COMPLETE? `_scan` runs inside `$( )`, so its status is
  # lost, and a subshell killed mid-walk (reproduced with `POSIXLY_CORRECT=1`
  # and a failing `cat`) handed over only what it had emitted before dying —
  # which read as a verdict (plan memo §11, D4). Exactly one terminal record,
  # in last position, or this run decided nothing. Asked BEFORE anything is
  # counted, so a walk killed before its first record says "did not complete"
  # rather than "read 0".
  _ENDS="$(_classify terminal -c '^end	')"
  _last="${1##*$'\n'}"
  if [ "$_ENDS" -ne 1 ] || [ "$_last" != "end	scan" ]; then
    echo "!! the scan did not complete (terminal records: $_ENDS; last record: '${_last%%	*}'), so this run decided nothing" >&2
    exit 2
  fi
  K2_HITS="$(_classify K2 '' '^k2	')"
  ERR_HITS="$(_classify error '' '^err	')"
  SCANNED="$(_classify count -c '^ok	')"
}

# ---- CONTROLS: re-invoke THIS script over fixtures, assert the exit code ----
# The samples are spelled out a SECOND time on purpose. They are the independent
# subject each check is tested against, exactly as `layout-box-reader-trip-wire.sh`'s
# `ban_control` passes a hand-written sample line beside the pattern. If someone
# edits $K2RE to a different spelling, the two stop agreeing and THAT is
# the signal — the failure a pattern-derived fixture cannot see.
# Two fixtures for the content predicate, on purpose: the path A-i actually removed
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

if [ -z "$_SELFTEST" ]; then
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
# No `|| true` here: it is exactly the token that made the stored-path arms'
# statuses look deliberately discarded (#501 R89). `_scan`'s status does not
# survive the substitution either way; whether the walk completed is what
# `_verdict` asserts, from the terminal record.
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
  # ⚠ THE WORD THE HEADER RETRACTED MAY NOT SURVIVE IN THE LINE THE GATE PRINTS.
  # This said `-- ABSOLUTE` over a count that is the UNION of both predicates,
  # i.e. over the half the header calls a bounded heuristic — so a CI reader saw
  # the exact claim that kept five rounds patching boundary examples. The
  # retraction had been applied only where reviewers read.
  echo "  K2: 0 \`.claude/(skills|tools)/<a>/<b>\` paths named here."
  echo "     stored paths (entry names, symlink targets): ABSOLUTE — closed and decidable."
  echo "     file content: a bounded heuristic over running text (see this wire's header)."
fi

if [ -n "$ERR_HITS" ]; then
  echo "!! part of the generic core could not be read, so the verdict above does not"
  echo "   cover it -- this wire does not report a green over what it never read:"
  printf '%s\n' "$ERR_HITS" | sed 's/^err	/     /'
  failed=1
fi

[ "$failed" -eq 0 ] || exit 1
echo "webref generic-core layering trip-wire PASSED"
