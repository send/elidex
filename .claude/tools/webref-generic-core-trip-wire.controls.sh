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
# WHAT IT MAY ASSUME: `$SELF`, `$SCRATCH`, `$ROOT`, the `CONTROL_*` strings, and
# the `_git` / `_fgit` / `_phys` helpers — all defined by the wire before it
# sources this. It defines `$CTL`, `_control`, `_ctl_env` and `$_perm_line`, and
# leaves `ctl_ok` for the wire to read.
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
_fgit() { GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_SYSTEM=/dev/null _git "$@"; }
# Distinguish an environment failure from a dead assertion: an empty scratch
# dir would exercise nothing and silently "pass". `mktemp -d` is checked, and
# the cleanup path is the absolute one it returned (#501 R55: an unchecked
# `mktemp` made an `rm -rf` expand to the repo root).
if ! CTL="$(mktemp -d "$SCRATCH/ctlXXXXXX")" || [ -z "$CTL" ] || [ ! -d "$CTL" ]; then
  echo "!! could not create a scratch dir for the controls (TMPDIR/disk?)," >&2
  echo "   so this run's assertions were never proved able to fire." >&2
  exit 2
fi
# ⚠ NO SECOND `trap ... EXIT` HERE. `trap` REPLACES; a second one silently
# discarded the scratch-root cleanup and left an empty directory behind on
# every successful run (#501 R94, reproduced). `CTL` is created UNDER
# `$SCRATCH`, so the trap at the top already removes it — one owner, one
# cleanup, nothing to compose.

for d in clean pin k2 tools binary err empty walk link odd nl seg cache cachedir extra name emptyname quotename nlname rawbyte forge linkname ignored lsfail grepfail grepfaillink nltarget linkslash staged fifotracked notcommitted inscope stagedlink nulblob committed replaced routed routeddecoy cfgkept; do mkdir -p "$CTL/$d"; done
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
         cachedir extra name emptyname quotename nlname rawbyte forge linkname ignored grepfail grepfaillink nltarget linkslash staged fifotracked notcommitted inscope stagedlink nulblob committed replaced routed routeddecoy cfgkept; do
  ( cd "$CTL/$d" 2>/dev/null && _fgit init -q . >/dev/null 2>&1 \
    && _fgit add -A >/dev/null 2>&1 ) || true
done
# …and the cache fixture's probe is FORCE-added under an ignored path, which
# is the case `--cached` exists to keep (#501 R77).
( cd "$CTL/cachedir" && printf '__pycache__/\n' > .gitignore \
  && _fgit add -A >/dev/null 2>&1 \
  && _fgit add -f __pycache__/probe.txt >/dev/null 2>&1 ) || true
# THE THREE WAYS THE INDEX AND THE WORKING TREE DISAGREE (#501 R92). Each is
# built AFTER the add loop above, because each needs the index to hold one
# thing while the worktree holds another.
# (1) A violation STAGED and reverted in the worktree. `git show :victim.py`
#     still carries it, so a pre-push run that reads only the worktree
#     certifies the very commit that pushes it.
( cd "$CTL/staged" && printf 'X = "%s"\n' "$CONTROL_K2" > victim.py \
  && _fgit add victim.py >/dev/null 2>&1 && printf '# %s\n' "$CONTROL_CLEAN" > victim.py ) || true
# (2) A TRACKED path replaced by a FIFO. `--cached` still lists it, and
#     opening it blocks forever with no writer — the local gate hangs instead
#     of failing closed. Nothing here may open it.
( cd "$CTL/fifotracked" && printf '# %s\n' "$CONTROL_CLEAN" > sub.py \
  && _fgit add sub.py >/dev/null 2>&1 && command rm -f sub.py && mkfifo sub.py ) || true
# (2b) A STAGED SYMLINK whose target holds a SPACE inside a segment, with the
#      worktree target since made clean. The index mode says it is a symlink,
#      so its blob is a stored path; sent through the running-text predicate
#      the space terminated the match and the wire read GREEN (#501 R93).
( cd "$CTL/stagedlink" && ln -s '.claude/skills/team name/rule.md' entry \
  && _fgit add entry >/dev/null 2>&1 \
  && command rm -f entry && ln -s 'harmless/target' entry \
  && printf '# %s\n' "$CONTROL_CLEAN" > ok.py && _fgit add ok.py >/dev/null 2>&1 ) || true
# (2c) A mode-120000 index entry whose BLOB HOLDS A NUL. git will store and
#      commit it; no filesystem can realise it as a symlink. It must red the
#      gate as unreadable, not be quietly shortened into something clean.
( cd "$CTL/nulblob" \
  && printf '.claude/skills/\000/rule.md' > raw.bin \
  && _sha="$(_fgit hash-object -w --stdin < raw.bin)" \
  && command rm -f raw.bin \
  && _fgit update-index --add --cacheinfo "120000,$_sha,entry" >/dev/null 2>&1 \
  && printf '# %s\n' "$CONTROL_CLEAN" > ok.py && _fgit add ok.py >/dev/null 2>&1 ) || true
# (2d) A violation COMMITTED and then fixed only in the index and worktree. A
#      push sends the commit, so a gate that reads the index alone calls this
#      clean while `git show HEAD:victim.py` still carries it (#501 R95).
( cd "$CTL/committed" && printf 'X = "%s"\n' "$CONTROL_K2" > victim.py \
  && _fgit add victim.py >/dev/null 2>&1 \
  && _fgit -c user.name=w -c user.email=w@e commit -q -m c >/dev/null 2>&1 \
  && printf '# %s\n' "$CONTROL_CLEAN" > victim.py \
  && _fgit add victim.py >/dev/null 2>&1 ) || true
# (2e) A local `replace` ref pointing the staged blob at a clean one. What is
#      committed is the object the index NAMES, so that is what must be read.
( cd "$CTL/replaced" && printf 'X = "%s"\n' "$CONTROL_K2" > victim.py \
  && _fgit add victim.py >/dev/null 2>&1 \
  && _bad="$(_fgit rev-parse :0:victim.py)" \
  && printf '# %s\n' "$CONTROL_CLEAN" > victim.py \
  && _good="$(_fgit hash-object -w victim.py)" \
  && _fgit replace "$_bad" "$_good" >/dev/null 2>&1 ) || true
# (2f) `GIT_DIR`/`GIT_WORK_TREE` exported at another checkout. `-C` does not
#      win over them, so the inventory described the decoy while the worktree
#      arm read files here — one clean entry, exit 0, over a violation (#501
#      R95). The decoy is a real repo holding nothing forbidden.
printf 'SRC = "%s"\n' "$CONTROL_K2"      > "$CTL/routed/probe.py"
printf '# %s\n' "$CONTROL_CLEAN"         > "$CTL/routeddecoy/ok.py"
( cd "$CTL/routed" && _fgit add -A >/dev/null 2>&1 ) || true
( cd "$CTL/routeddecoy" && _fgit add -A >/dev/null 2>&1 ) || true
# (2g) A checkout path holding a GLOB CHARACTER. Built outside the fixture
#      loop on purpose: the loop's word list would itself glob the name.
#      The violation sits BESIDE the scope, so a widened `REL_DIR` — the
#      defect — turns this green fixture red.
mkdir -p "$CTL/glob[1]/scope"
printf '# %s\n' "$CONTROL_CLEAN"         > "$CTL/glob[1]/scope/ok.py"
printf 'SRC = "%s"\n' "$CONTROL_K2"      > "$CTL/glob[1]/outside.py"
( cd "$CTL/glob[1]" && _fgit init -q . >/dev/null 2>&1 && _fgit add -A >/dev/null 2>&1 ) || true
# (3) An untracked violation hidden by `$GIT_DIR/info/exclude` — per-clone,
#     uncommitted state that `--exclude-standard` honours and no other clone
#     of the same commit shares. (The machine-wide `core.excludesFile` is the
#     same code path; it is reproduced in #501 R92's thread rather than given
#     a fixture, because neutralising the config layer above would also
#     neutralise the fixture that tried to set it.)
( cd "$CTL/notcommitted" && printf '# %s\n' "$CONTROL_CLEAN" > ok.py \
  && _fgit add ok.py >/dev/null 2>&1 \
  && printf 'SRC = "%s"\n' "$CONTROL_K2" > probe.txt \
  && printf 'probe.txt\n' > .git/info/exclude ) || true

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
  _out_f="$CTL/.control_out"
  WEBREF_WIRE_SELFTEST="$1" WEBREF_WIRE_SELFTEST_DIR="${5:-}" \
    WEBREF_WIRE_SELFTEST_EXTRA="${6:-}" PATH="${7:+$7:}$PATH" \
    env ${_ctl_env[@]+"${_ctl_env[@]}"} "$SELF" > "$_out_f" 2>&1 & _cpid=$!
  # ⚠ THE TIMER IS A SEPARATE PROCESS FROM THE SHELL THAT FORKED IT. `$!` is
  # the subshell; killing only that reparents the `sleep` to PID 1, where it
  # runs out its 30 s — one orphan per control, dozens per local gate run
  # (#501 R93, observed). The subshell traps TERM and takes its own children
  # with it, so the pair is reaped as a unit with shell builtins only.
  ( trap 'kill $(jobs -p) 2>/dev/null; exit 0' TERM
    sleep 30 & wait
    kill -9 "$_cpid" 2>/dev/null ) & _wpid=$!
  wait "$_cpid"; _rc=$?
  kill -TERM "$_wpid" 2>/dev/null || true; wait "$_wpid" 2>/dev/null || true
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
_ctl_env=()   # per-control environment; reset after any control that sets it
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
_control "$CTL/emptyname" 1 "entry NAME" "an EMPTY entry's name is the hierarchy" || ctl_ok=1
_control "$CTL/quotename" 1 "entry NAME" "a quote inside a name segment"          || ctl_ok=1
_control "$CTL/nlname"  1 "entry NAME" "a NEWLINE inside a name segment"          || ctl_ok=1
_control "$CTL/rawbyte" 1 "K2: a"      "a byte no UTF-8 locale can bracket"       || ctl_ok=1
_control "$CTL/forge"  0 "PASSED" "a name cannot forge a verdict record"      || ctl_ok=1
_control "$CTL/linkname" 1 "entry NAME" "a SYMLINK's own name is the hierarchy" || ctl_ok=1
_control "$CTL/ignored" 0 "PASSED" "an IGNORED generated artefact does not fire" || ctl_ok=1
_control "$CTL/lsfail" 1 "population is incomplete" "a failed inventory fails closed" "" "" "$CTL/fakegit" || ctl_ok=1
_control "$CTL/grepfail" 1 "the entry NAME went unchecked" "a failed NAME matcher fails closed" "" "" "$CTL/fakegrep" || ctl_ok=1
_control "$CTL/grepfaillink" 1 "the symlink TARGET went unchecked" "a failed TARGET matcher fails closed" "" "" "$CTL/fakegrep" || ctl_ok=1
_control "$CTL/nltarget" 1 "K2: a" "a NEWLINE-terminated symlink target is not truncated" || ctl_ok=1
_control "$CTL/linkslash" 0 "PASSED" "readlink's own newline is not read as stored content" || ctl_ok=1
_control "$CTL/staged" 1 "(staged)" "a STAGED violation reverted in the worktree still fires" || ctl_ok=1
_control "$CTL/fifotracked" 1 "NOT opened" "a tracked path replaced by a FIFO is not opened" || ctl_ok=1
if [ -s "$CTL/notcommitted/.git/info/exclude" ]; then
  _control "$CTL/notcommitted" 1 "K2: a" "per-clone info/exclude cannot hide an entry" || ctl_ok=1
else
  echo "!! CONTROL NOT EXERCISED (per-clone info/exclude cannot hide an entry): the" >&2
  echo "   fixture's info/exclude is empty, so the probe is visible either way." >&2
  ctl_ok=1
fi
_control "$CTL/committed" 1 "(in HEAD)" "a COMMITTED violation fixed only in the index still fires" || ctl_ok=1
# ⚠ A FIXTURE THAT DID NOT BUILD MUST NOT PASS. Both of these are built with
# `|| true`, and for both the FALLBACK STATE still satisfies the control's own
# assertion — so a failed `git replace` (or a failed `info/exclude` write) left
# the control green while testing nothing, and the mutation it exists to catch
# would have survived it too (#501 R96, reproduced with a `git` wrapper failing
# only `replace`). Every other fixture here falls back to a state its control
# REJECTS; these two were the exceptions, and each now asserts the precondition
# that makes it meaningful.
# ⚠ Asked as `replace -l`, not by reading the blob: this run exports
# `GIT_NO_REPLACE_OBJECTS=1`, so a read here returns the violating bytes
# whether or not the replacement took — the check would have passed vacuously
# for the very reason the control exists. (Caught by the precondition firing
# on a correctly-built fixture.)
if [ -n "$(cd "$CTL/replaced" && _fgit replace -l 2>/dev/null)" ]; then
  _control "$CTL/replaced" 1 "(staged)" "a replace ref cannot substitute the staged blob" || ctl_ok=1
else
  echo "!! CONTROL NOT EXERCISED (a replace ref cannot substitute the staged blob):" >&2
  echo "   the fixture's replacement never took, so the staged blob is the violating one" >&2
  echo "   either way and this control would pass without testing anything." >&2
  ctl_ok=1
fi
_ctl_env=("GIT_CONFIG_COUNT=1" "GIT_CONFIG_KEY_0=safe.directory" "GIT_CONFIG_VALUE_0=*")
_control "$CTL/cfgkept" 1 "K2: a" "the caller's git CONFIGURATION survives the routing purge" "" "" "$CTL/fakegitcfg" || ctl_ok=1
_ctl_env=()
_ctl_env=("GIT_DIR=$CTL/routeddecoy/.git" "GIT_WORK_TREE=$CTL/routeddecoy")
_control "$CTL/routed" 1 "K2: a" "exported GIT_DIR cannot redirect the scan" || ctl_ok=1
_ctl_env=()
_control "$CTL/glob[1]" 0 "PASSED" "a glob character in the checkout path does not widen the scope" "scope" || ctl_ok=1
_control "$CTL/nulblob" 1 "holds a NUL" "a NUL-bearing staged symlink blob is not a path" || ctl_ok=1
_control "$CTL/stagedlink" 1 "(staged) ->" "a STAGED symlink target is a stored path" || ctl_ok=1
_control "$CTL/inscope" 2 "INSIDE the tree" "scratch inside the scanned tree decides nothing" "" "" "$CTL/fakemktemp" || ctl_ok=1
_control "$CTL/extra"  1 "K2: a" "a symlinked EXTRA entry is scanned" "sub" "entry" || ctl_ok=1
if [ -p "$CTL/odd/pipe" ]; then
  _control "$CTL/odd" 0 "PASSED" "an unstorable entry neither hangs nor hides" || ctl_ok=1
else
  echo "  note: the odd-entry control could not be exercised here (no fifo);"
  echo "        every other control ran"
fi
# An empty scope must be an ERROR, not a pass: "no violations" and "nothing
# read" are different answers and only one of them is green.
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
[ "$ctl_ok" -eq 0 ] || exit 1
echo "  controls: green reachable; K2 fires under both roots, on the removed path,"
echo "            inside binary content, on a symlink's stored target, on an entry's"
echo "            own NAME, and on a symlinked entry script beside the scope; an empty"
echo "            scope fails closed; an entry git cannot store neither hangs nor hides"
echo "            a verdict"
printf '%s\n' "$_perm_line"
echo "            (each asserted on this script's own exit status, over a fixture tree)"
