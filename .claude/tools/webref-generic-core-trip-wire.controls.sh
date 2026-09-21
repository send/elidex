#!/usr/bin/env bash
# THE CONTROLS FOR `webref-generic-core-trip-wire.sh` — sourced by it, never run
# on its own.
#
# WHY IT IS A SEPARATE FILE: the wire crossed 1000 lines, and CLAUDE.md's
# touch-time split discipline says to split at a real cohesion seam when you
# touch such a file — not to defer it. This is that seam, and it was visible
# well before the line count: the wire ANSWERS about a tree, and everything here
# exists to prove the wire can reach every answer it claims. One green run of
# the scanner tells you nothing this file has not earned.
#
# THE INTERFACE, AND IT IS ASSERTED BELOW RATHER THAN DESCRIBED. What this file
# CONSUMES from the wire: `$SELF` (re-invoked per control), `$SCRATCH` (the one
# scratch root, whose trap also cleans up `$CTL`), `$_CONTROLS` (this file's own
# path, which the mutation harness copies), the five `CONTROL_*` sample strings,
# and the `_git` helper. That list — and only that list — is checked at entry.
# ⚠ WHAT THIS FILE DEFINES IS NOT AN INTERFACE, and a previous revision said it
# was: it named `$CTL`, `_fgit`, `_control`, `_ctl_env`, `$_perm_line`,
# `$_fifo_line` and `ctl_ok` "for the wire to read", and the wire reads NONE of
# them (`grep -c '_perm_line\|_fifo_line\|_ctl_env\|_fgit\|\$CTL\|ctl_ok'` over
# the wire → **0**). They are this file's own locals; `ctl_ok` is consumed three
# lines from where it is set. The data flow is one-way, and saying otherwise
# invented a contract nobody could break.
# ⚠ AND THE CONSUMES LIST HAS BEEN WRONG TWICE. It first claimed `$ROOT` and
# `_phys`, which this file did not then mention; the revision that said so also
# added a mutation harness that consumes `$_CONTROLS`, which the same revision
# left off the list and out of the guard. A list maintained by hand beside the
# code it describes is the thing that drifts — the guard below is the only
# reason this one is true, and it is why the guard enumerates rather than the
# comment.
#
# WHY SOURCED AND NOT A SEPARATE PROGRAM. The alternative the plan-review
# offered — a real entry point taking explicit parameters — costs a second copy
# of `_git`, the locale pin and the `GIT_NO_*` exports, i.e. a second statement
# of how this gate reads git. That is the decision-surface duplication this
# instrument spent four review rounds collapsing (#501 R76, R80, R81, R89), and
# it is what CLAUDE.md's "one issue, one way" forbids. The seam is real (answers
# vs. proof the answers are reachable); what was missing was the interface being
# enforced instead of narrated, and that is what this file now does.
#
# ⚠ ITS ABSENCE IS NOT A SKIPPED SELF-TEST. The wire refuses to run without it
# (exit 2, "decided nothing") rather than scanning with its controls silently
# gone — which is the one way a split like this could weaken the gate it is
# meant to keep legible.
# ⚠ THE FIXTURES ARE BUILT WITH GIT, so whoever runs this must not be able to
# change what they contain (#501 R92). A global `core.excludesFile` of `*.py`
# made `git add -A` skip the fixtures' own files, and an `init.templateDir`
# could seed `info/exclude`. `_fgit` neutralises both config layers.
# ⚠ PER CALL, NOT `export` (#501 R94). Exported, it applied to the REAL scan
# too and took `safe.directory` with it — measured: a checkout owned by
# another UID, readable only because of a global `safe.directory` entry, went
# from working to "detected dubious ownership", so the required gate could not
# run in an otherwise valid environment. The fixtures need a clean config; the
# repository needs the user's. The inventory is made machine-independent by
# `--exclude-per-directory` instead (see `_scan`), which is why nothing has to
# be stripped for the real read.
# THE CONTRACT, ASSERTED AT ENTRY. Run on its own this file has none of the
# names above, and it used to say so by failing in `mktemp` with
# `mkdtemp failed on /ctlXXXXXX: Read-only file system` — a diagnostic about the
# wrong subject entirely, which is how a reader concludes the controls are
# broken rather than misinvoked.
# ⚠ AND THIS IS THE PIN FOR "IS THIS A WIRE?". The question has ONE decider —
# `scripts/trip-wires.sh`, which diffs its `*-trip-wire.sh` glob against
# `REQUIRED_WIRES` in BOTH directions — so a rename that brought this file INTO
# the convention would run it and fail twice over: here, with the message below,
# and there, with `trip-wire(s) ran but are not registered`. Nothing rests on
# the name being one token short of the glob.
_ctl_missing=
for _n in SELF SCRATCH _CONTROLS CONTROL_REMOVED CONTROL_K2 CONTROL_TOOLS CONTROL_BINARY CONTROL_CLEAN; do
  [ -n "${!_n:-}" ] || _ctl_missing="$_ctl_missing \$$_n"
done
declare -f _git >/dev/null 2>&1 || _ctl_missing="$_ctl_missing _git()"
if [ -n "$_ctl_missing" ]; then
  echo "!! This file is the CONTROLS for \`webref-generic-core-trip-wire.sh\`. It is" >&2
  echo "   SOURCED by that wire and has no meaning on its own; missing:$_ctl_missing" >&2
  echo "   Run the wire instead — it sources this file and refuses to run without it." >&2
  exit 2
fi
_fgit() { GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_SYSTEM=/dev/null _git "$@"; }

# ONE FACT, RECORDED ONCE: DID THIS FIXTURE'S BUILD CHAIN SUCCEED?
# Every fixture below is built as `( cd … && … ) || _fixture_failed <name>`, and
# `_control` refuses to report on a fixture whose chain did not.
# ⚠ THIS REPLACES FIVE HAND-WRITTEN PRECONDITIONS, and the reason to prefer it
# is not brevity. Each fixture is built under `|| true`, so a step that fails
# leaves a TREE THAT LOOKS PLAUSIBLE — and for some fixtures that tree still
# satisfies the control's own assertion, which then passes having tested
# nothing. Five such sites were found one at a time, each fixed with a bespoke
# probe re-deriving the same fact from the fixture's end state (was the blob
# tracked? is there a replace ref? did `info/exclude` get written? is it a repo
# at all?), and the comment introducing the first two called the class closed at
# two. It was not: `cachedir`, `empty` and `lstreefail` followed, two of them
# added by the commit that wrote the sentence. The fact those probes
# reconstruct is available for free at the point of failure, for ALL of them —
# so the population is every fixture, not the five somebody noticed.
_FIX_FAILED=""
_fixture_failed() { _FIX_FAILED="$_FIX_FAILED $1"; }
# Distinguish an environment failure from a dead assertion: an empty scratch
# dir would exercise nothing and silently "pass". `mktemp -d` is checked, and
# the cleanup path is the absolute one it returned (#501 R55: an unchecked
# `mktemp` made an `rm -rf` expand to the repo root).
if ! CTL="$(mktemp -d "$SCRATCH/ctlXXXXXX")" || [ -z "$CTL" ] || [ ! -d "$CTL" ]; then
  echo "!! could not create a scratch dir for the controls (TMPDIR/disk?)," >&2
  echo "   so this run's assertions were never proved able to fire." >&2
  exit 2
fi
# CAN THIS FILESYSTEM HOLD A FIFO? Two controls need one — `odd` (an entry git
# cannot store) and `fifotracked` (a tracked path replaced by one) — and both
# used to create theirs under `|| true`, so on a filesystem without them each
# fell back to a state its own assertion could not distinguish: `odd` printed a
# note claiming "every other control ran" (false in exactly the runs where
# `fifotracked` had just failed for the same missing capability), and
# `fifotracked` reported a WIRE failure for a MACHINE limitation. Asked once,
# answered once, reported once — the shape `$_perm_line` already uses for the
# other capability this harness cannot assume.
_fifo_ok=0
if mkfifo "$CTL/.fifoprobe" 2>/dev/null; then _fifo_ok=1; command rm -f "$CTL/.fifoprobe"; fi

# ⚠ NO SECOND `trap ... EXIT` HERE. `trap` REPLACES; a second one silently
# discarded the scratch-root cleanup and left an empty directory behind on
# every successful run (#501 R94, reproduced). `CTL` is created UNDER
# `$SCRATCH`, so the trap at the top already removes it — one owner, one
# cleanup, nothing to compose.

for d in clean pin k2 tools binary err empty walk link odd nl seg cache cachedir extra name emptyname quotename nlname rawbyte forge linkname ignored lsfail lstreefail grepfail grepfaillink nltarget linkslash staged fifotracked notcommitted inscope stagedlink nulblob committed replaced routed routeddecoy cfgkept punct suffixpath headprobe catfail phantom; do mkdir -p "$CTL/$d"; done
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
# ⚠ NOT a bare `|| mkfifo`: under the wire's `set -e` a post-probe failure
# (EEXIST, ENOSPC, a race) aborts the sourced file with NO diagnostic at all —
# no `$_fifo_line`, no `$_perm_line`, no summary — which is the "a diagnostic
# about the wrong subject" failure this file exists to avoid, in its purest
# form: no subject.
if [ "$_fifo_ok" -eq 1 ] && ! mkfifo "$CTL/odd/pipe" 2>/dev/null; then
  echo "!! mkfifo succeeded as a probe and then failed for the fixture ($CTL/odd/pipe)." >&2
  echo "   The FIFO controls cannot be built, and this run is not reporting a wire defect." >&2
  exit 2
fi
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
printf '# %s\n' "$CONTROL_CLEAN"          > "$CTL/cachedir/__pycache__/probe.txt"
# A CLEAN file whose own path is the forbidden hierarchy. A content search
# contents, so without a name pass this reads as a file with nothing in it.
mkdir -p "$CTL/name/$(dirname "$CONTROL_K2")"
printf '# %s\n' "$CONTROL_CLEAN"          > "$CTL/name/$CONTROL_K2"
printf '# %s\n' "$CONTROL_CLEAN"          > "$CTL/name/ok.py"
# …and the same shape as a SYMLINK, whose name pass is a separate arm.
mkdir -p "$CTL/linkname/$(dirname "$CONTROL_K2")"
printf '# %s\n' "$CONTROL_CLEAN"          > "$CTL/linkname/ok.py"
ln -s "$CTL/linkname/ok.py" "$CTL/linkname/$CONTROL_K2"
# An inventory that fails AFTER printing something. `git ls-files` exiting
# nonzero with nothing on stderr is the one signal a truncated population has,
# and it was discarded (#501 R85).
mkdir -p "$CTL/fakegit"
# ⚠ ONLY `ls-files` FAILS. The shim used to answer every `git` invocation,
# which made it a control for whatever the wire asked git next rather than for
# a failing inventory — and #501 R95 added a `rev-parse` preflight that the
# blanket shim then answered with the fixture's own bytes. Same shape as
# `fakegrep`: a control shims the ONE call it is about.
printf '#!/bin/sh\ncase " $* " in *" ls-files "*) printf "ok.py\\000"; exit 1;; esac\nexec %s "$@"\n' \
  "$(command -v git)" > "$CTL/fakegit/git"
chmod +x "$CTL/fakegit/git"
# …AND THE SAME FOR `ls-tree`, because the HEAD inventory is a THIRD list with a
# status check of its own and nothing exercised it: `fakegit` fails `ls-files`
# only, so the `ls-tree` arm added at #501 R95 could be deleted and every
# control here stayed green (found by this slice's review — the one mutation
# record covering all three `_ls_rc` checks hid it, because killing the first
# was enough to red the run).
mkdir -p "$CTL/fakegitls"
printf '#!/bin/sh\ncase " $* " in *" ls-tree "*) printf "x\\000"; exit 1;; esac\nexec %s "$@"\n' \
  "$(command -v git)" > "$CTL/fakegitls/git"
chmod +x "$CTL/fakegitls/git"
printf '# %s\n' "$CONTROL_CLEAN"          > "$CTL/lstreefail/ok.py"

# ⚠ THE TWO FALSE-POSITIVE FIXTURES. A required gate that reds a legitimate
# tree is the one people switch off, so both repairs the external reviewer
# found get a control asserting the wire stays GREEN — the direction a
# violation-shaped fixture can never test.
#  (a) CLOSING PUNCTUATION: prose naming a ONE-segment directory in parentheses
#      used to match, with `)` as the second segment.
printf '# See (%s) for details\n' '.claude/tools/foo/' > "$CTL/punct/ok.py"
printf '# %s\n' "$CONTROL_CLEAN"          > "$CTL/punct/other.py"
#  (b) A LEADING BOUNDARY: `.claude` as the SUFFIX of another segment is not
#      the top-level host path. The subject here is the entry's own NAME, which
#      is where the stored-path predicate reads.
mkdir -p "$CTL/suffixpath/fixtures/example.claude/skills/team"
printf '# %s\n' "$CONTROL_CLEAN" > "$CTL/suffixpath/fixtures/example.claude/skills/team/rule.md"
printf '# %s\n' "$CONTROL_CLEAN" > "$CTL/suffixpath/ok.py"
#      ⚠ …and the SAME repair in the RUNNING-TEXT predicate needs its own
#      fixture, because the two regexes are independent: the entry-name fixture
#      above exercises `$K2RE_PATH` only, so removing the boundary from `$K2RE`
#      reintroduced the URL false positive with every control still green
#      (external reviewer, R2). This one is file CONTENT.
printf '# see https://example.claude/skills/team/rule.md for the upstream note\n' \
  > "$CTL/suffixpath/url.py"

# A `git` that fails ONLY the HEAD existence probe, so "unborn" and "git could
# not answer" can be told apart. ⚠ `--verify` is the discriminator: the wire
# also runs `rev-parse --local-env-vars` at startup, and a shim matching
# `rev-parse` alone would make the run exit 2 there for a different reason.
mkdir -p "$CTL/headprobe"
printf '#!/bin/sh\ncase " $* " in *" --verify "*) exit 2;; esac\nexec %s "$@"\n' \
  "$(command -v git)" > "$CTL/headprobe/git"
chmod +x "$CTL/headprobe/git"
printf '# %s\n' "$CONTROL_CLEAN"          > "$CTL/headprobe/ok.py"

# A `cat` that always fails, for the one read whose status used to be swallowed
# by the trailing-newline sentinel.
mkdir -p "$CTL/catfail"
printf '#!/bin/sh\nexit 1\n' > "$CTL/catfail/cat"
chmod +x "$CTL/catfail/cat"
printf '# %s\n' "$CONTROL_CLEAN"          > "$CTL/catfail/ok.py"

# A `git` whose UNTRACKED inventory names a path that is not there — the
# deterministic form of "the file vanished between `ls-files` and the read".
# The real race (another process rewriting an untracked file) cannot be staged
# reliably; this shim poses the same question to the same arm.
mkdir -p "$CTL/fakegitphantom"
printf '#!/bin/sh\ncase " $* " in *" --others "*) %s "$@"; printf "phantom-gone.py\\000"; exit 0;; esac\nexec %s "$@"\n' \
  "$(command -v git)" "$(command -v git)" > "$CTL/fakegitphantom/git"
chmod +x "$CTL/fakegitphantom/git"
printf '# %s\n' "$CONTROL_CLEAN"          > "$CTL/phantom/ok.py"
# A `grep` that fails ONLY for the stored-path predicate's invocation, so the
# control discriminates that arm rather than every grep in the run (shadowing
# them all would abort in `_verdict` instead, for a different reason).
# A `mktemp` that hands back a directory INSIDE the tree under scan — what a
# workspace-confined `TMPDIR` does. ⚠ macOS's `/usr/bin/mktemp` ignores
# `TMPDIR` when given no template, which is why the control shims the command
# instead of setting the variable: the same defect reproduces on the machines
# that honour it, and a control that only fires on some of them is not one.
# A `git` that answers `ls-files` with nothing unless the caller's
# `GIT_CONFIG_COUNT` reached it. The fixture holds a violation, so a run that
# KEEPS the configuration reds on K2 and a run that strips it reads zero and
# exits 2 — the two verdicts differ in status AND message, which is what makes
# this control discriminate rather than merely fire.
# ⚠ Shimmed because the real case — a checkout owned by another UID, reachable
# only via `safe.directory` — cannot be built here. The variable's SURVIVAL is
# the property under test, and that is constructible; the ownership is not.
mkdir -p "$CTL/fakegitcfg"
printf '#!/bin/sh\nif [ -z "${GIT_CONFIG_COUNT:-}" ]; then case " $* " in *" ls-files "*) exit 0;; esac; fi\nexec %s "$@"\n' \
  "$(command -v git)" > "$CTL/fakegitcfg/git"
chmod +x "$CTL/fakegitcfg/git"
printf 'SRC = "%s"\n' "$CONTROL_K2"      > "$CTL/cfgkept/probe.py"
mkdir -p "$CTL/fakemktemp"
printf '#!/bin/sh\nd="${WEBREF_WIRE_SELFTEST}/scratch"\nmkdir -p "$d"\nprintf %%s "$d"\n' \
  > "$CTL/fakemktemp/mktemp"
chmod +x "$CTL/fakemktemp/mktemp"
printf '# %s\n' "$CONTROL_CLEAN"          > "$CTL/inscope/ok.py"
mkdir -p "$CTL/fakegrep"
printf '#!/bin/sh\ncase " $* " in *" -aEo "*) exit 2;; esac\nexec %s "$@"\n' \
  "$(command -v grep)" > "$CTL/fakegrep/grep"
chmod +x "$CTL/fakegrep/grep"
printf '# %s\n' "$CONTROL_CLEAN"          > "$CTL/lsfail/ok.py"
mkdir -p "$CTL/grepfail/.claude/skills/team"
printf '# %s\n' "$CONTROL_CLEAN"          > "$CTL/grepfail/.claude/skills/team/rule.md"
# SEPARATELY, because a fixture another entry can satisfy proves nothing about
# the entry it is named for (#501 R75/R80, twice). `grepfail` above would go
# red from its NAME arm alone, so it cannot show that the TARGET arm reports a
# failed matcher. Here the only entries are a clean-named symlink whose stored
# target is the forbidden hierarchy, and a clean regular file to keep the run
# off the zero-read guard.
# ⚠ What discriminates here is the MESSAGE, not the exit status: the shim
# fails `-aEo` for every subject, so each entry's NAME matcher reports a
# failure too and the run exits 1 whatever the TARGET arm does. Measured —
# reverting only the target arm still exits 1, and the control catches it on
# "the symlink TARGET went unchecked" being absent. `_control` asserts both,
# which is why that is enough; said here so the exit status is not read as
# the thing under test.
ln -s "$CONTROL_K2" "$CTL/grepfaillink/entry"
# A target whose FINAL SEGMENT IS A NEWLINE — the bytes `$( )` throws away.
ln -s $'.claude/skills/team/\n' "$CTL/nltarget/entry"
printf '# %s\n' "$CONTROL_CLEAN"          > "$CTL/nltarget/ok.py"
# The OTHER direction, and the reason `-n` is not merely tidiness: a target
# ending in `/` names a DIRECTORY, not an `<a>/<b>` path, so it must stay
# green. Without `-n`, readlink's own newline lands where the empty final
# segment was and `[^/]+` matches it — the wire would fire on a target that
# holds no violation. The sentinel alone does not catch that; this does
# (measured: dropping only `-n` leaves `nltarget` green and turns this red).
ln -s '.claude/skills/a/' "$CTL/linkslash/entry"
printf '# %s\n' "$CONTROL_CLEAN"          > "$CTL/linkslash/ok.py"
printf '# %s\n' "$CONTROL_CLEAN"          > "$CTL/grepfaillink/ok.py"
# An IGNORED generated artefact carrying a forbidden path. It must NOT fire:
# a `.pyc` embeds its source's absolute path, and scanning build products
# turned the wire red for anyone who had merely run the tool (#501 R79).
printf '# %s\n' "$CONTROL_CLEAN"          > "$CTL/ignored/ok.py"
printf 'generated/\n'                      > "$CTL/ignored/.gitignore"
mkdir -p "$CTL/ignored/generated"
printf 'SRC = "%s"\n' "$CONTROL_K2"       > "$CTL/ignored/generated/artefact.bin"
# An EMPTY tracked file whose NAME is the hierarchy, ALONE in its own fixture.
# It shared `name/` with a non-empty forbidden name until #501 R80, so the
# control passed on that one and a scanner skipping every empty file was
# green (reproduced). A fixture another entry can satisfy proves nothing
# about the entry it is named for.
# A NEWLINE inside a name segment: it is data, not a record separator.
mkdir -p "$CTL/nlname/.claude/skills/$(printf 'team\nname')"
printf '# %s\n' "$CONTROL_CLEAN" > "$CTL/nlname/.claude/skills/$(printf 'team\nname')/rule.md"
printf '# %s\n' "$CONTROL_CLEAN" > "$CTL/nlname/ok.py"
# A byte no UTF-8 locale can place in a bracket expression. ⚠ This control is
# environment-sensitive: it discriminates the `LC_ALL=C` export only where the
# INHERITED locale is multibyte, so on a C-locale machine the mutation that
# deletes the export survives it. Said here rather than left implied.
printf 'X = ".claude/skills/\377name/rule.md"\n' > "$CTL/rawbyte/probe.bin"
printf '# %s\n' "$CONTROL_CLEAN" > "$CTL/rawbyte/ok.py"
# A quote inside a NAME segment: `/` is the only delimiter a stored path has.
mkdir -p "$CTL/quotename/.claude/skills/team\"name"
printf '# %s\n' "$CONTROL_CLEAN"          > "$CTL/quotename/.claude/skills/team\"name/rule.md"
printf '# %s\n' "$CONTROL_CLEAN"          > "$CTL/quotename/ok.py"
mkdir -p "$CTL/emptyname/$(dirname "$CONTROL_K2")"
: > "$CTL/emptyname/$CONTROL_K2"
printf '# %s\n' "$CONTROL_CLEAN"          > "$CTL/emptyname/ok.py"
# A CLEAN file whose NAME carries a record separator and a classifier tag.
# Unescaped, the continuation became a synthetic K2 hit (#501 R80).
printf '# %s\n' "$CONTROL_CLEAN"          > "$CTL/forge/$(printf 'safe\nk2\tforged')"

# Every fixture is a repository, because the population is git's answer:
# tracked, plus untracked minus ignored. A fixture that is not a repo cannot
# reproduce that distinction — and the distinction is now load-bearing.
for d in clean pin k2 tools binary err empty walk link odd nl seg cache \
         extra name emptyname quotename nlname rawbyte forge linkname ignored lstreefail grepfail grepfaillink nltarget linkslash staged fifotracked notcommitted inscope stagedlink nulblob committed replaced routed routeddecoy cfgkept punct suffixpath headprobe catfail phantom; do
  ( cd "$CTL/$d" 2>/dev/null && _fgit init -q . >/dev/null 2>&1 \
    && _fgit add -A >/dev/null 2>&1 ) || _fixture_failed "$d"
done
# ⚠ TWO NAMES ARE ABSENT FROM THAT LIST ON PURPOSE, and both are now stated —
# one of them was not, and "is this deliberate or a dropped word?" is exactly
# what a hand-kept second list cannot answer. `cachedir` is built below, in the
# only order that makes its force-add load-bearing. `lsfail` is NOT A REPOSITORY
# by design: its control runs under a `git` shim that fails `ls-files` whatever
# the directory is, so making it a repo would add a state the control does not
# read.
# …and the cache fixture's probe is FORCE-added under an ignored path, which
# is the case `--cached` exists to keep (#501 R77).
# ⚠ BUILT OUTSIDE THE LOOP ABOVE, AND THE ORDER IS THE WHOLE CONTROL. Inside
# it, `add -A` ran BEFORE `.gitignore` existed, so the probe was already tracked
# and the force-add changed nothing: measured, deleting `add -f` outright left
# this control GREEN and the wire exit 0. `.gitignore` first, then the ordinary
# add — which must now SKIP the probe — then the force-add as the only thing
# that can track it.
# ⚠ THE VIOLATION IS IN THE WORKTREE ONLY, AND THE STAGED BLOB IS CLEAN. With
# the forbidden content in both, this control passed from the INDEX arm alone —
# so a regression that stopped scanning the worktree copy of a tracked file
# beneath an ignored directory stayed green, which is the exact combination the
# fixture exists to cover (external reviewer, P2). Order: `.gitignore` first (so
# the ordinary add skips the probe), then the force-add of the CLEAN blob (the
# only thing that can track it), then the worktree copy is made violating.
( cd "$CTL/cachedir" && _fgit init -q . >/dev/null 2>&1 \
  && printf '__pycache__/\n' > .gitignore \
  && _fgit add -A >/dev/null 2>&1 \
  && _fgit add -f __pycache__/probe.txt >/dev/null 2>&1 \
  && printf 'RULE = "%s"\n' "$CONTROL_K2" > __pycache__/probe.txt ) || _fixture_failed cachedir
# THE THREE WAYS THE INDEX AND THE WORKING TREE DISAGREE (#501 R92). Each is
# built AFTER the add loop above, because each needs the index to hold one
# thing while the worktree holds another.
# (1) A violation STAGED and reverted in the worktree. `git show :victim.py`
#     still carries it, so a pre-push run that reads only the worktree
#     certifies the very commit that pushes it.
( cd "$CTL/staged" && printf 'X = "%s"\n' "$CONTROL_K2" > victim.py \
  && _fgit add victim.py >/dev/null 2>&1 && printf '# %s\n' "$CONTROL_CLEAN" > victim.py ) || _fixture_failed staged
# (2) A TRACKED path replaced by a FIFO. `--cached` still lists it, and
#     opening it blocks forever with no writer — the local gate hangs instead
#     of failing closed. Nothing here may open it.
[ "$_fifo_ok" -eq 0 ] || \
( cd "$CTL/fifotracked" && printf '# %s\n' "$CONTROL_CLEAN" > sub.py \
  && _fgit add sub.py >/dev/null 2>&1 && command rm -f sub.py && mkfifo sub.py ) || _fixture_failed fifotracked
# (2b) A STAGED SYMLINK whose target holds a SPACE inside a segment, with the
#      worktree target since made clean. The index mode says it is a symlink,
#      so its blob is a stored path; sent through the running-text predicate
#      the space terminated the match and the wire read GREEN (#501 R93).
( cd "$CTL/stagedlink" && ln -s '.claude/skills/team name/rule.md' entry \
  && _fgit add entry >/dev/null 2>&1 \
  && command rm -f entry && ln -s 'harmless/target' entry \
  && printf '# %s\n' "$CONTROL_CLEAN" > ok.py && _fgit add ok.py >/dev/null 2>&1 ) || _fixture_failed stagedlink
# (2c) A mode-120000 index entry whose BLOB HOLDS A NUL. git will store and
#      commit it; no filesystem can realise it as a symlink. It must red the
#      gate as unreadable, not be quietly shortened into something clean.
( cd "$CTL/nulblob" \
  && printf '.claude/skills/\000/rule.md' > raw.bin \
  && _sha="$(_fgit hash-object -w --stdin < raw.bin)" \
  && command rm -f raw.bin \
  && _fgit update-index --add --cacheinfo "120000,$_sha,entry" >/dev/null 2>&1 \
  && printf '# %s\n' "$CONTROL_CLEAN" > ok.py && _fgit add ok.py >/dev/null 2>&1 ) || _fixture_failed nulblob
# The `ls-tree` shim only reaches its arm if the fixture HAS a HEAD — an unborn
# HEAD skips the whole block, which would make the control green over a branch
# it never took.
( cd "$CTL/lstreefail" && _fgit add -A >/dev/null 2>&1 \
  && _fgit -c user.name=w -c user.email=w@e commit -q -m c >/dev/null 2>&1 ) || _fixture_failed lstreefail
# …and the same staged-symlink geometry for the `cat`-failure control: the blob
# is what `cat` reads, so the fixture must HAVE one.
( cd "$CTL/catfail" && ln -s '.claude/skills/team/rule.md' entry \
  && _fgit add entry >/dev/null 2>&1 \
  && command rm -f entry && ln -s 'harmless/target' entry ) || _fixture_failed catfail
# (2d) A violation COMMITTED and then fixed only in the index and worktree. A
#      push sends the commit, so a gate that reads the index alone calls this
#      clean while `git show HEAD:victim.py` still carries it (#501 R95).
( cd "$CTL/committed" && printf 'X = "%s"\n' "$CONTROL_K2" > victim.py \
  && _fgit add victim.py >/dev/null 2>&1 \
  && _fgit -c user.name=w -c user.email=w@e commit -q -m c >/dev/null 2>&1 \
  && printf '# %s\n' "$CONTROL_CLEAN" > victim.py \
  && _fgit add victim.py >/dev/null 2>&1 ) || _fixture_failed committed
# (2e) A local `replace` ref pointing the staged blob at a clean one. What is
#      committed is the object the index NAMES, so that is what must be read.
( cd "$CTL/replaced" && printf 'X = "%s"\n' "$CONTROL_K2" > victim.py \
  && _fgit add victim.py >/dev/null 2>&1 \
  && _bad="$(_fgit rev-parse :0:victim.py)" \
  && printf '# %s\n' "$CONTROL_CLEAN" > victim.py \
  && _good="$(_fgit hash-object -w victim.py)" \
  && _fgit replace "$_bad" "$_good" >/dev/null 2>&1 ) || _fixture_failed replaced
# (2f) `GIT_DIR`/`GIT_WORK_TREE` exported at another checkout. `-C` does not
#      win over them, so the inventory described the decoy while the worktree
#      arm read files here — one clean entry, exit 0, over a violation (#501
#      R95). The decoy is a real repo holding nothing forbidden.
printf 'SRC = "%s"\n' "$CONTROL_K2"      > "$CTL/routed/probe.py"
printf '# %s\n' "$CONTROL_CLEAN"         > "$CTL/routeddecoy/ok.py"
( cd "$CTL/routed" && _fgit add -A >/dev/null 2>&1 ) || _fixture_failed routed
( cd "$CTL/routeddecoy" && _fgit add -A >/dev/null 2>&1 ) || _fixture_failed routeddecoy
# (2g) A checkout path holding a GLOB CHARACTER. Built outside the fixture
#      loop on purpose: the loop's word list would itself glob the name.
#      The violation sits BESIDE the scope, so a widened `REL_DIR` — the
#      defect — turns this green fixture red.
mkdir -p "$CTL/glob[1]/scope"
printf '# %s\n' "$CONTROL_CLEAN"         > "$CTL/glob[1]/scope/ok.py"
printf 'SRC = "%s"\n' "$CONTROL_K2"      > "$CTL/glob[1]/outside.py"
( cd "$CTL/glob[1]" && _fgit init -q . >/dev/null 2>&1 && _fgit add -A >/dev/null 2>&1 ) || _fixture_failed "glob[1]"
# (3) An untracked violation hidden by `$GIT_DIR/info/exclude` — per-clone,
#     uncommitted state that `--exclude-standard` honours and no other clone
#     of the same commit shares. (The machine-wide `core.excludesFile` is the
#     same code path; it is reproduced in #501 R92's thread rather than given
#     a fixture, because neutralising the config layer above would also
#     neutralise the fixture that tried to set it.)
( cd "$CTL/notcommitted" && printf '# %s\n' "$CONTROL_CLEAN" > ok.py \
  && _fgit add ok.py >/dev/null 2>&1 \
  && printf 'SRC = "%s"\n' "$CONTROL_K2" > probe.txt \
  && printf 'probe.txt\n' > .git/info/exclude ) || _fixture_failed notcommitted

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
             # $5 = optional scope subdir (relative), $6 = optional extra entry (relative),
             # $7 = optional PATH prefix (to shadow a tool the wire calls),
             # and `_ctl_env` (an ARRAY, set by the caller) for environment
             # assignments. ⚠ NOT a string: `env ${8:-}` word-split them, so a
             # `TMPDIR` holding a space broke the routed control with
             # `env: 'dir/.../.git': No such file or directory` and the whole
             # gate exited 1 for a contributor whose temp path has a space
             # (#501 R96). A path is data here too.
  # ⚠ WITH A WATCHDOG, because a hang is a verdict this harness could not
  # otherwise report (#501 R92). The FIFO finding's whole harm was that the
  # local gate BLOCKED instead of failing closed — and a control for it, run
  # plainly, would block too: it would never return to say so. Reverting that
  # fix now produces a reported control failure rather than a wire that waits
  # forever. Backgrounded and `wait`ed rather than polled, so a healthy
  # control costs nothing; the killer is itself killed when the child returns.
  # ⚠ THE FIXTURE'S BUILD STATUS, ASKED BEFORE ANYTHING ELSE. Every fixture is
  # built under `|| _fixture_failed <name>`, so "did the tree this control is
  # about actually get built?" is one lookup rather than a probe re-derived per
  # site from the fixture's end state. Without it a broken build leaves a
  # plausible-looking tree, and for several fixtures that tree still satisfies
  # the assertion below — a control green over a question never posed.
  case " $_FIX_FAILED " in
    *" ${1##*/} "*)
      echo "!! CONTROL NOT EXERCISED ($4): its fixture did not build, so this control" >&2
      echo "   would be asserting over a tree that never posed the question." >&2
      return 1 ;;
  esac
  _out_f="$CTL/.control_out"
  # ⚠ THE TOKEN IS WHAT MAKES SELF-TEST MODE REACHABLE. The wire refuses to
  # enter it without this, so an inherited `WEBREF_WIRE_SELFTEST` left exported
  # in somebody's shell can no longer redirect the required gate at a fixture.
  # ⚠ `set -m` SO THE CHILD IS ITS OWN PROCESS GROUP. Without it the watchdog's
  # `kill -9 "$_cpid"` reaches only the wire's own bash; a command substitution
  # or a `grep` BLOCKED beneath it — precisely the FIFO hang this watchdog
  # exists to catch — is reparented and stays blocked after the gate reports
  # the timeout, so repeated local runs accumulate permanent orphans. Measured
  # on bash 5.3 and 3.2: group kill reaps the descendant, top-PID kill does not.
  set -m
  WEBREF_WIRE_SELFTEST="$1" WEBREF_WIRE_SELFTEST_TOKEN="$_SELFTEST_TOKEN" \
    WEBREF_WIRE_SELFTEST_DIR="${5:-}" \
    WEBREF_WIRE_SELFTEST_EXTRA="${6:-}" PATH="${7:+$7:}$PATH" \
    env ${_ctl_env[@]+"${_ctl_env[@]}"} "$SELF" > "$_out_f" 2>&1 & _cpid=$!
  set +m
  # ⚠ THE TIMER IS A SEPARATE PROCESS FROM THE SHELL THAT FORKED IT. `$!` is
  # the subshell; killing only that reparents the `sleep` to PID 1, where it
  # runs out its 30 s — one orphan per control, dozens per local gate run
  # (#501 R93, observed). The subshell traps TERM and takes its own children
  # with it, so the pair is reaped as a unit with shell builtins only.
  ( trap 'kill $(jobs -p) 2>/dev/null; exit 0' TERM
    sleep 30 & wait
    kill -9 -"$_cpid" 2>/dev/null || kill -9 "$_cpid" 2>/dev/null ) & _wpid=$!
  wait "$_cpid"; _rc=$?
  kill -TERM "$_wpid" 2>/dev/null || true; wait "$_wpid" 2>/dev/null || true
  # ⚠ CLEARED HERE, NOT BY THE CALLER. `_ctl_env` is file-scope (bash has no
  # array parameter), and it used to be reset by a `_ctl_env=()` line after each
  # control that set one — a positional convention, so a control inserted
  # between a set and its clear would silently inherit the previous control's
  # environment. One owner: whoever sets it gets exactly the next control.
  _ctl_env=()
  _out="$(cat "$_out_f" 2>/dev/null)"
  if [ "$_rc" -ge 128 ]; then
    echo "!! CONTROL FAILED ($4): the wire did not finish — killed after 30s (signal" >&2
    echo "   status $_rc). A gate that blocks is worse than one that reds: it never" >&2
    echo "   reaches a verdict at all. Something on this path opened what it must not." >&2
    return 1
  fi
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
_ctl_env=()   # per-control environment; `_control` clears it after each use
_control "$CTL/clean" 0 "PASSED"                  "green is reachable"   || ctl_ok=1
_control "$CTL/pin"   1 "K2: a"  "K2 fires on the path A-i removed" || ctl_ok=1
_control "$CTL/k2"    1 "K2: a"  "K2 fires on a path never here"    || ctl_ok=1
_control "$CTL/tools" 1 "K2: a"  "K2 fires under the tools root too" || ctl_ok=1
# ⚠ THE NEEDLE IS THE RECORD, NOT THE HEADLINE. `"K2: a"` passes with `-a`
# removed from `_content`'s grep too: this grep answers `Binary file … matches`
# on stdout and exits 0, so the run still reds. Measured as a surviving mutant.
# ⚠ BOTH ARMS STILL EMIT A RECORD in that state — the staged one names a TEMP
# BLOB PATH that no longer exists, the worktree one names the real entry — so
# "it names a temp path" is only half the reason, and not the half that
# discriminates. What discriminates is `:1:`, the LINE NUMBER, which only a
# read produces; `(worktree)` is chosen over `(staged)` because its path half is
# stable. Getting this reason wrong is how the needle drifts back.
_control "$CTL/binary" 1 "control.dat (worktree):1:" "K2 fires inside binary content, and the content is READ" || ctl_ok=1
_control "$CTL/link"   1 "K2: a" "K2 fires on a symlink's target"    || ctl_ok=1
_control "$CTL/nl"     1 "K2: a" "a newline in a filename is not a split" || ctl_ok=1
_control "$CTL/seg"    1 "K2: a" "K2 covers @ and non-ASCII segments"     || ctl_ok=1
_control "$CTL/cache"  1 "K2: a" "a regular file named __pycache__ is read" || ctl_ok=1
_control "$CTL/cachedir" 1 "K2: a" "a file UNDER a cache directory is read"  || ctl_ok=1
_control "$CTL/name"   1 "entry NAME" "an entry's own NAME is the hierarchy"  || ctl_ok=1
_control "$CTL/emptyname" 1 "entry NAME" "an EMPTY entry's name is the hierarchy" || ctl_ok=1
_control "$CTL/quotename" 1 "entry NAME" "a quote inside a name segment"          || ctl_ok=1
_control "$CTL/nlname"  1 "entry NAME" "a NEWLINE inside a name segment"          || ctl_ok=1
_control "$CTL/rawbyte" 1 "K2: a"      "a byte no UTF-8 locale can bracket"       || ctl_ok=1
_control "$CTL/forge"  0 "PASSED" "a name cannot forge a verdict record"      || ctl_ok=1
_control "$CTL/linkname" 1 "entry NAME" "a SYMLINK's own name is the hierarchy" || ctl_ok=1
_control "$CTL/ignored" 0 "PASSED" "an IGNORED generated artefact does not fire" || ctl_ok=1
_control "$CTL/lsfail" 1 "population is incomplete" "a failed inventory fails closed" "" "" "$CTL/fakegit" || ctl_ok=1
_control "$CTL/lstreefail" 1 "the HEAD inventory exited" "a failed HEAD inventory fails closed" "" "" "$CTL/fakegitls" || ctl_ok=1
_control "$CTL/headprobe" 1 "the HEAD probe exited" "a failed HEAD PROBE is not an unborn HEAD" "" "" "$CTL/headprobe" || ctl_ok=1
_control "$CTL/catfail" 1 "staged symlink blob could not be read" "a failed staged-blob read is not a clean target" "" "" "$CTL/catfail" || ctl_ok=1
_control "$CTL/phantom" 1 "the inventory listed it but it is gone" "an inventoried path that vanished is not silently skipped" "" "" "$CTL/fakegitphantom" || ctl_ok=1
# ⚠ TWO GREEN-DIRECTION CONTROLS. Every other control here proves the wire can
# RED; these two prove it does not red on a legitimate tree, which is the
# failure mode that gets a required gate switched off rather than fixed.
_control "$CTL/punct" 0 "PASSED" "closing punctuation is not a path segment" || ctl_ok=1
_control "$CTL/suffixpath" 0 "PASSED" "a segment merely ENDING in .claude is not the host path" || ctl_ok=1
# …and the self-test escape hatch, which must not be reachable from an
# inherited environment. `_ctl_env` overrides the token the harness passes, so
# this is the one control that asks the wire to REFUSE to run.
_ctl_env=("WEBREF_WIRE_SELFTEST_TOKEN=stale-from-somebody-s-shell")
_control "$CTL/clean" 2 "by the controls beside this wire" "an inherited SELFTEST export cannot redirect the gate" || ctl_ok=1
_control "$CTL/grepfail" 1 "the entry NAME went unchecked" "a failed NAME matcher fails closed" "" "" "$CTL/fakegrep" || ctl_ok=1
_control "$CTL/grepfaillink" 1 "the symlink TARGET went unchecked" "a failed TARGET matcher fails closed" "" "" "$CTL/fakegrep" || ctl_ok=1
_control "$CTL/nltarget" 1 "K2: a" "a NEWLINE-terminated symlink target is not truncated" || ctl_ok=1
_control "$CTL/linkslash" 0 "PASSED" "readlink's own newline is not read as stored content" || ctl_ok=1
_control "$CTL/staged" 1 "(staged)" "a STAGED violation reverted in the worktree still fires" || ctl_ok=1
# ⚠ THE TWO FIFO CONTROLS SHARE ONE PRECONDITION AND ONE REPORT LINE, built
# HERE beside the decision that produces it (the `$_perm_line` shape, and for
# the same reason: a summary written apart from its decision can disagree with
# it). A machine that cannot hold a FIFO is a capability limit, not a wire
# defect — but it is also not something this run may pass over in silence.
_fifo_line="            an entry git cannot store neither hangs nor hides a verdict, and a
            tracked path replaced by one is not opened"
if [ "$_fifo_ok" -eq 1 ]; then
  _control "$CTL/fifotracked" 1 "NOT opened" "a tracked path replaced by a FIFO is not opened" || ctl_ok=1
else
  _fifo_line="            ⚠ NOT EXERCISED on this machine: BOTH FIFO controls (this
            filesystem holds no fifo), so this run carries no evidence that an
            entry git cannot store neither hangs nor hides a verdict, NOR that a
            tracked path replaced by one is not opened — the second being the
            control whose original finding was a local gate that HUNG"
fi
_control "$CTL/notcommitted" 1 "K2: a" "per-clone info/exclude cannot hide an entry" || ctl_ok=1
_control "$CTL/committed" 1 "(in HEAD)" "a COMMITTED violation fixed only in the index still fires" || ctl_ok=1
# ⚠ A FIXTURE THAT DID NOT BUILD MUST NOT PASS, and `_fixture_failed` is where
# that is now decided — for every control, not for the five somebody noticed.
# The history is worth keeping because it is what the general mechanism costs
# to NOT have: a failed `git replace` left this control green while testing
# nothing (#501 R96, reproduced with a `git` wrapper failing only `replace`);
# the fix was a bespoke `replace -l` probe here; the comment introducing it
# called the class closed at two sites; `cachedir`, `empty` and `lstreefail`
# then turned up in the same state, two of them added by the commit that wrote
# that sentence. Five hand-written probes, each re-deriving from the fixture's
# END STATE a fact the BUILD already knew.
# ⚠ AND ONE OF THOSE PROBES HAD TO BE SUBTLE, which is the other argument for
# not writing them: this run exports `GIT_NO_REPLACE_OBJECTS=1`, so reading the
# blob returns the violating bytes whether or not the replacement took — the
# obvious probe would have passed vacuously for the very reason the control
# exists, and only `replace -l` worked. A build status has no such trap.
_control "$CTL/replaced" 1 "(staged)" "a replace ref cannot substitute the staged blob" || ctl_ok=1
_ctl_env=("GIT_CONFIG_COUNT=1" "GIT_CONFIG_KEY_0=safe.directory" "GIT_CONFIG_VALUE_0=*")
_control "$CTL/cfgkept" 1 "K2: a" "the caller's git CONFIGURATION survives the routing purge" "" "" "$CTL/fakegitcfg" || ctl_ok=1
_ctl_env=("GIT_DIR=$CTL/routeddecoy/.git" "GIT_WORK_TREE=$CTL/routeddecoy")
_control "$CTL/routed" 1 "K2: a" "exported GIT_DIR cannot redirect the scan" || ctl_ok=1
_control "$CTL/glob[1]" 0 "PASSED" "a glob character in the checkout path does not widen the scope" "scope" || ctl_ok=1
_control "$CTL/nulblob" 1 "holds a NUL" "a NUL-bearing staged symlink blob is not a path" || ctl_ok=1
_control "$CTL/stagedlink" 1 "(staged) ->" "a STAGED symlink target is a stored path" || ctl_ok=1
_control "$CTL/inscope" 2 "INSIDE the tree" "scratch inside the scanned tree decides nothing" "" "" "$CTL/fakemktemp" || ctl_ok=1
_control "$CTL/extra"  1 "K2: a" "a symlinked EXTRA entry is scanned" "sub" "entry" || ctl_ok=1
# …the second of the pair `$_fifo_line` above reports on.
if [ "$_fifo_ok" -eq 1 ]; then
  _control "$CTL/odd" 0 "PASSED" "an unstorable entry neither hangs nor hides" || ctl_ok=1
fi
# An empty scope must be an ERROR, not a pass: "no violations" and "nothing
# read" are different answers and only one of them is green.
# ⚠ THIS ONE IS WHY THE BUILD STATUS IS RECORDED RATHER THAN PROBED. A `git
# init` that never ran produces a BYTE-IDENTICAL verdict here: the
# `SCANNED -eq 0` guard short-circuits before the `ERR_HITS` block, so a
# non-repository and an empty repository both print
# `!! read 0 stored objects or files` and exit 2. No end-state probe over the
# fixture can tell them apart cheaply; the build's own exit status can, and
# does, for this control and the other thirty-nine.
_control "$CTL/empty" 2 "read 0 stored objects" "an empty scope fails loudly" || ctl_ok=1
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
# ---- THE MUTATION SET ------------------------------------------------------
# WHY IT IS HERE AND NOT IN A COMMIT LOG. Every control above proves a verdict
# is REACHABLE. What it does not prove is that the verdict is PRODUCED BY THE
# CODE THE CONTROL IS ABOUT — a control can pass over an arm someone deleted, if
# another arm happens to red the same fixture (measured twice: #501 R70, where
# killing the verdict's error arm left a control green, and this slice's own
# `cachedir`, green with its force-add removed entirely).
#
# The answer is a mutation set, and the only question was where it lives. The
# population used to be PROSE — "each fix this wire's history names", scattered
# across review commits on #501, which that PR's squash merge ERASES. A
# criterion whose population disappears when the parent lands is not a
# criterion. So the set lives HERE, machine-readably, in the file that ships
# with the thing it is about:
#
#   one record per line, TAB-separated:  <sed expression>  <substring the run must print>
#
# The expression is applied to a COPY of the wire; the copy must (a) differ from
# the original — a stale anchor that matches nothing is a FAILED entry, not a
# passing one — (b) exit non-zero, and (c) print the named control's own
# diagnostic, so an entry that reds for an unrelated reason is caught too.
#
# RUN IT — it is NOT part of the gate (it costs one full control pass per entry,
# minutes rather than seconds), and a required check nobody can afford to run is
# how gates get switched off:
#
#     WEBREF_WIRE_MUTANTS=1 bash .claude/tools/webref-generic-core-trip-wire.sh
#
# ⚠ ADDING AN ARM MEANS ADDING A RECORD. Nothing here can detect an arm that
# never had one — this set is a floor, not a census. ⚠ AND A DECLARED FLOOR IS
# NOT A DETECTOR: this file used to say exactly the sentence above and assert
# nothing, so deleting a record shrank the set in silence and the run stayed
# green. Three things now hold it up, and each is checked below rather than
# described:
#   * a CORRESPONDENCE, IN BOTH DIRECTIONS AND ALWAYS-ON. Every record's needle
#     must name an actual `_control` label in this file (a stale anchor would
#     otherwise report "wrong reason" forever), AND the number of controls with
#     NO record is ratcheted: `_MUT_UNRECORDED_MAX` may only come down.
#     ⚠ THE SECOND DIRECTION IS THE ONE THAT CATCHES ANYTHING. The first has
#     never had a violation; the second is where both real gaps lived — the
#     `ls-tree` arm with no record, and a control whose fixture made its
#     mutation inert. A count-only floor was tried first and is gone: it could
#     not distinguish "one deleted, one added" from "unchanged", and it said
#     nothing about WHICH control had been left bare. The gap is the quantity
#     with meaning, so the gap is what is ratcheted.
#   * a STANDING NEGATIVE CONTROL — one record whose expected outcome is
#     SURVIVAL (`!survive`), so a harness that cannot distinguish a real kill
#     from an environment failure reds. That distinction is not hypothetical:
#     an inert entry run ONCE is what caught this harness reporting 18/18 when
#     every mutant was in fact exiting 126 on a permission bit, and a control
#     that is not standing cannot catch it twice.
#     ⚠ ITS EDIT IS A COMMENT, AND THAT IS A REQUIREMENT, not a convenience.
#     The first version appended a space to an `rm -f` argument list — inert,
#     but only ACCIDENTALLY so, and that particular `rm` is itself dead (the
#     scratch root's trap removes the whole directory). A negative control whose
#     inertness depends on the current behaviour of the code it edits stops
#     being a negative control the moment that code changes, silently. Comment
#     text cannot alter a verdict by construction.
_MUT_UNRECORDED_MAX=21
# ⚠ A FUNCTION, NOT `x="$(cat <<'EOF' … )"`. Under bash 3.2 — the stock macOS
# shell this wire commits to — a quoted here-document nested inside a command
# substitution is still parsed for expansions, and the `unset "$_v"` in one of
# the records below aborted the WHOLE FILE with `_v: unbound variable`, on every
# run, mutation mode or not. Measured: 3.2 red, 5.3 green.
_mutants() { cat <<'MUTANTS'
s/grep -aEn --/grep -En --/	K2 fires inside binary content, and the content is READ
s/"$_mrc" -gt 1/"$_mrc" -gt 99/	a failed NAME matcher fails closed
s/^K2RE_PATH=.*/K2RE_PATH="$K2RE"/	a quote inside a name segment
s|${1//$'\\n'/$_REC_SEP}|${1}|	a NEWLINE inside a name segment
s/^  _stored "${rel#"$_dir"\/}"/  : /	an entry's own NAME is the hierarchy
s/--exclude-per-directory=.gitignore/--exclude-standard/	per-clone info/exclude cannot hide an entry
s/\[ "$_hrc" -eq 0 \]/false/	a COMMITTED violation fixed only in the index still fires
s/if ! tr -d .\\000. < "\$_b"/if false/	a NUL-bearing staged symlink blob is not a path
s/readlink -n "$f"/readlink "$f"/	readlink's own newline is not read as stored content
s/export GIT_NO_LAZY_FETCH=1 GIT_NO_REPLACE_OBJECTS=1/export GIT_NO_LAZY_FETCH=1/	a replace ref cannot substitute the staged blob
s|^_esc() .*|_esc() { printf '%s' "$1"; }|	a name cannot forge a verdict record
s/^export LC_ALL=C$/export LC_ALL=C.UTF-8/	a byte no UTF-8 locale can bracket
s/"$SCANNED" -eq 0/"$SCANNED" -eq -1/	an empty scope fails loudly
s/\[ -e "$p" \] || \[ -L "$p" \]/[ -e "$p" ]/	a symlinked EXTRA entry is scanned
s/REL_DIR="${SCOPE_DIR#"$ROOT"\/}"/REL_DIR="${SCOPE_DIR#$ROOT\/}"/	a glob character in the checkout path does not widen the scope
s/GIT_CONFIG\*) : ;;/GIT_CONFIG*) unset "$_v" ;;/	the caller's git CONFIGURATION survives the routing purge
s/\[ "$_ls_rc" -eq 0 \]/[ 0 -eq 0 ]/	a failed inventory fails closed
s/the HEAD inventory exited %d/the HEAD inventory was fine %d/	a failed HEAD inventory fails closed
s/ls-files -z --stage/ls-files -z --cached/	a STAGED symlink target is a stored path
s/\[ "$_hrc" -gt 1 \]/false/	a failed HEAD PROBE is not an unborn HEAD
s/\[ "$_catrc" -ne 0 \]/false/	a failed staged-blob read is not a clean target
s/)}>,;\]/]/g	closing punctuation is not a path segment
s/elif ! _git -C "$ROOT" ls-files --error-unmatch/elif false \&\& ! _git -C "$ROOT" ls-files --error-unmatch/	an inventoried path that vanished is not silently skipped
s/(^|\/)\\.claude/\\.claude/	a segment merely ENDING in .claude is not the host path
s/(^|\[^A-Za-z0-9_.~-\])\\.claude/\\.claude/	a segment merely ENDING in .claude is not the host path
s/\[ "${WEBREF_WIRE_SELFTEST_TOKEN:-}" != "$_SELFTEST_TOKEN" \]/false/	an inherited SELFTEST export cannot redirect the gate
s/^# Run from anywhere\./# Run from anywhere (edited by the negative control)./	!survive
MUTANTS
}

# ---- THE CORRESPONDENCE, CHECKED ON EVERY RUN ---------------------------------
# ⚠ THESE TWO USED TO LIVE INSIDE THE MUTATION BLOCK, which is opt-in and costs
# minutes — so a PR that deleted a record or renamed a control stayed green
# until somebody happened to run the harness by hand. They are static
# properties of this shipped file and cost milliseconds, so they belong where
# every run sees them. (The mutation RUN stays opt-in; only its bookkeeping
# moved.)
_mutants > "$CTL/.mutants"
_mut_orphan=0
while IFS="$(printf '\t')" read -r _ _mwant; do
  [ -n "${_mwant:-}" ] || continue
  # `!survive` is the standing negative control; it names no control by design.
  [ "$_mwant" != '!survive' ] || continue
  grep -qF -- "\"$_mwant\"" "$_CONTROLS" || {
    echo "!! mutation record needle \"$_mwant\" matches no _control label in this file." >&2
    echo "   A record whose control was renamed reports \"wrong reason\" forever." >&2
    _mut_orphan=1; }
done < "$CTL/.mutants"
[ "$_mut_orphan" -eq 0 ] || ctl_ok=1
# …and the direction that actually finds things: controls with NO record.
# The label is the third quoted argument of a `_control` call.
_mut_bare=0
awk -F'"' '/^ *_control /{print $6}' "$_CONTROLS" | sed '/^$/d' | while IFS= read -r _lbl; do
  grep -qF -- "	$_lbl" "$CTL/.mutants" || printf '%s\n' "$_lbl"
done > "$CTL/.bare"
_mut_bare="$(grep -c . "$CTL/.bare")"
if [ "$_mut_bare" -gt "$_MUT_UNRECORDED_MAX" ]; then
  echo "!! $_mut_bare controls have no mutation record, against a ratchet of $_MUT_UNRECORDED_MAX." >&2
  echo "   Either the new control needs a record, or a record was deleted. The bare ones:" >&2
  sed 's/^/     /' "$CTL/.bare" >&2
  echo "   If a control genuinely cannot have one, say why and RAISE the ratchet in the" >&2
  echo "   same edit — so widening the gap is a visible decision rather than a silence." >&2
  ctl_ok=1
fi
command rm -f "$CTL/.bare"

[ "$ctl_ok" -eq 0 ] || exit 1


if [ -n "${WEBREF_WIRE_MUTANTS:-}" ]; then
  # The copy must sit BESIDE the wire: `$ROOT` is derived from `$0`, and the
  # scope it reads is this repository's real generic core. Its own controls file
  # has to be beside it too, under the name the wire derives (`${SELF%.sh}`), or
  # the copy refuses to run — which is criterion 2 working, not a harness bug.
  # ⚠ PER-RUN NAMES, BECAUSE THE WORKING TREE IS SHARED BY DESIGN. A fixed
  # `${SELF%.sh}.mutant.sh` is one path per checkout, and CLAUDE.md's
  # parallel-session rule makes two runs in one tree the EXPECTED case — so the
  # second run's `sed >` and its `rm -f` overwrote and deleted the first's
  # files mid-flight. The victim's copy then lost its controls and took the
  # wire's own `exit 2` arm, which this harness reported as
  # `killed for the WRONG REASON`: **it blamed the mutation set for an
  # environment collision.** Reproduced independently three times during this
  # slice's review, at `16`, `11` and `10 not killed as named` on a head that
  # measures `0` when run alone.
  # ⚠ AND THE COPY MUST SIT BESIDE THE WIRE, so `$SCRATCH` is not an option:
  # `$ROOT` is derived from `$0`, and the scope it must read is this
  # repository's real generic core. `$$` is what makes the two runs disjoint.
  _mut_wire="${SELF%.sh}.mutant.$$.sh"
  _mut_ctl="${SELF%.sh}.mutant.$$.controls.sh"
  # ⚠ A LEFTOVER IS A REPORT, NOT A FILE TO CLEAN UP. `trap` does not run on
  # SIGKILL, so a killed run leaves mode-755 artifacts in `.claude/tools/`
  # where a `git add -A` would stage them. Say so; do not delete another run's.
  for _stale in "${SELF%.sh}".mutant.*.sh; do
    case "$_stale" in *'.mutant.*.sh') break ;; esac
    case "$_stale" in "$_mut_wire"|"$_mut_ctl") continue ;; esac
    echo "  note: a previous mutation run left $_stale behind (SIGKILL?); it is" >&2
    echo "        not this run's to remove. Delete it once no run is using it." >&2
  done
  trap 'command rm -f "$_mut_wire" "$_mut_ctl"; case "$SCRATCH" in /*/*) chmod -R u+rwX "$SCRATCH" 2>/dev/null || true; rm -rf "$SCRATCH";; esac' EXIT
  cp "$_CONTROLS" "$_mut_ctl"
  # Through a FILE, not a pipe: the counters below must survive the loop, and a
  # `_mutants | while` runs the body in a subshell that discards them.
  _mutants > "$CTL/.mutants"
  _mut_n=0; _mut_bad=0
  while IFS="$(printf '\t')" read -r _mx _mwant; do
    [ -n "$_mx" ] || continue
    _mut_n=$((_mut_n + 1))
    if ! sed "$_mx" "$SELF" > "$_mut_wire" 2>/dev/null; then
      echo "!! MUTANT $_mut_n: the expression is not a valid sed script: $_mx" >&2
      _mut_bad=$((_mut_bad + 1)); continue
    fi
    # ⚠ AND IT MUST BE EXECUTABLE. `_control` invokes `"$SELF"` DIRECTLY, not
    # through `bash`, so a copy written by `sed` (mode 644) exits 126
    # "Permission denied" for EVERY control — which reds the run, prints every
    # control's diagnostic, and therefore satisfies a "did it name the right
    # control?" test VACUOUSLY. Measured: all 18 entries below "passed" that
    # way, and a deliberately inert entry (a comment-only edit that cannot
    # change any verdict) was the negative control that exposed it. The probe's
    # subject was the permission bit, not the mutation.
    chmod +x "$_mut_wire"
    if cmp -s "$_mut_wire" "$SELF"; then
      echo "!! MUTANT $_mut_n ($_mwant): the expression MATCHED NOTHING, so this entry" >&2
      echo "   tested a copy identical to the wire: $_mx" >&2
      _mut_bad=$((_mut_bad + 1)); continue
    fi
    _mrc2=0
    _mout="$(env -u WEBREF_WIRE_MUTANTS bash "$_mut_wire" 2>&1)" || _mrc2=$?
    if [ "$_mwant" = '!survive' ]; then
      # THE STANDING NEGATIVE CONTROL. Its edit is to COMMENT TEXT, so it
      # cannot change any verdict by construction and the wire MUST still be
      # green. If it reds, the harness is failing mutants for a reason that has
      # nothing to do with the mutation — a missing execute bit, a clobbered
      # copy, a full disk — and every "killed" above is unearned.
      [ "$_mrc2" -eq 0 ] || {
        echo "!! THE NEGATIVE CONTROL DIED (exit $_mrc2). An edit that changes no verdict" >&2
        echo "   reddened the wire, so this run cannot tell a real kill from a broken" >&2
        echo "   harness, and every kill reported above is unearned. Output:" >&2
        printf '%s\n' "$_mout" | sed 's/^/     /' >&2
        _mut_bad=$((_mut_bad + 1)); }
      continue
    fi
    if [ "$_mrc2" -eq 0 ]; then
      echo "!! MUTANT $_mut_n ($_mwant) SURVIVED: the wire still exited 0 with this" >&2
      echo "   applied, so nothing above is testing it: $_mx" >&2
      _mut_bad=$((_mut_bad + 1)); continue
    fi
    case "$_mout" in
      *"$_mwant"*) ;;
      *) echo "!! MUTANT $_mut_n killed for the WRONG REASON (exit $_mrc2): the output" >&2
         echo "   does not name \"$_mwant\", so another check masked the one under test." >&2
         _mut_bad=$((_mut_bad + 1)) ;;
    esac
  done < "$CTL/.mutants"
  command rm -f "$_mut_wire" "$_mut_ctl"
  echo "  mutation set: $_mut_n entr(ies), $_mut_bad not killed as named"
  [ "$_mut_bad" -eq 0 ] || exit 1
  echo "  every entry above was shown to red, and to red for its own reason"
  # ⚠ NO `exit 0` HERE, AND THAT IS THE WHOLE POINT. This block used to end the
  # run — so `WEBREF_WIRE_MUTANTS` merely PRESENT IN THE ENVIRONMENT (exported
  # once, in a shell that later runs `mise run trip-wires`) made the required
  # gate exit 0 having never scanned `_webref/`, with the driver recording it as
  # a wire that ran. Every other path in this file refuses to be green over
  # something it did not read; this was the one that did. The mutation set is
  # now something the run does BEFORE its verdict, not INSTEAD of it.
fi
echo "  controls: green reachable; K2 fires under both roots, on the removed path,"
echo "            inside binary content, on a symlink's stored target, on an entry's"
echo "            own NAME, and on a symlinked entry script beside the scope; an empty"
echo "            scope fails closed"
printf '%s\n' "$_fifo_line"
printf '%s\n' "$_perm_line"
echo "            (each asserted on this script's own exit status, over a fixture tree)"
