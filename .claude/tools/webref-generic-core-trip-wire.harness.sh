#!/usr/bin/env bash
# THE CONTROL HARNESS for `webref-generic-core-trip-wire.sh` — sourced by that
# wire's controls file, never run on its own.
#
# WHY IT IS A SEPARATE FILE: CLAUDE.md's touch-time split, taken when the
# controls file was about to pass the line threshold. The seam is HOW a control
# runs versus WHICH controls exist: this file holds the scratch root the
# fixtures live in, the window they are built in and the postconditions checked
# inside it, the shim-quoting helper, the FIFO capability probe and `_control`
# itself; the fixtures file builds each fixture, and the controls file holds the
# assertion made over it.
# ⚠ SOURCED ONLY AFTER THE CONTROLS FILE HAS ASSERTED WHAT THE WIRE HANDS IT
# (`$SELF`, `$SCRATCH`, `_git`, …), which is what this file consumes. It sets
# `_HARNESS` immediately before sourcing, so an unset one means this file was
# reached some other way.
if [ -z "${_HARNESS:-}" ]; then
  echo "!! This file is the control HARNESS for \`webref-generic-core-trip-wire.sh\`. It is" >&2
  echo "   SOURCED by that wire's controls file and has no meaning alone." >&2
  echo "   Run the wire instead." >&2
  exit 2
fi
#
# ⚠ THE FIXTURES ARE BUILT WITH GIT, so whoever runs this must not be able to
# change what they contain (#501 R92): a global `core.excludesFile` of `*.py`
# made `git add -A` skip the fixtures' own files, and an `init.templateDir`
# could seed `info/exclude`. Name lists failed at this three times — #519 R7's
# `GIT_CONFIG_COUNT`, the unlisted `GIT_TEMPLATE_DIR`, and a `GIT_*` sweep that
# missed the DEFAULT `core.excludesFile` under `$HOME` — so nothing here
# enumerates what the caller carries. THE FIXTURE BUILD RUNS IN A WINDOW BUILT
# FROM NOTHING (docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md §3): one child shell,
# started with `env -i` and the allowlist below, sources the fixtures file.
# Every git it starts inherits only that environment, unless a fixtures-file
# command itself altered its inputs (classes b and c of
# docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md §0.3), so the fixtures call plain `git`
# and there is no per-call helper.
# ⚠ A CHILD PROCESS, NOT `export` (#501 R94). Exported, a clean configuration
# applied to the REAL scan too and took `safe.directory` with it — measured: a
# checkout owned by another UID, readable only because of a global
# `safe.directory` entry, went from working to "detected dubious ownership", so
# the required gate could not run in an otherwise valid environment. The
# fixtures need a clean configuration; the repository needs the user's. So
# `_git` PRESERVES the caller's configuration, `GIT_CONFIG*` included (#501
# R97), and the window, being a child, cannot touch it. The inventory is made
# machine-independent by `--exclude-per-directory` instead (see `_scan`), which
# is why nothing has to be stripped for the real read.
# What each allowlist entry buys is tabled in docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md §3;
# the postconditions below check, from inside the window, that the build saw
# nothing else.
_FGIT_VOID="$SCRATCH/fgit-void"
mkdir "$_FGIT_VOID" || exit 2
_FGIT_ENVBIN="$(command -v env)"
_FGIT_BASH="$BASH"
_FGIT_ENV=("PATH=$PATH" "HOME=$_FGIT_VOID" GIT_CONFIG_NOSYSTEM=1 GIT_ATTR_NOSYSTEM=1 "GIT_TEMPLATE_DIR=$_FGIT_VOID" "LC_ALL=C")
# Names the window's environment may hold: the allowlist above, plus what bash
# itself maintains for the processes it starts.
_FGIT_ENV_NAMES="PATH HOME GIT_CONFIG_NOSYSTEM GIT_ATTR_NOSYSTEM GIT_TEMPLATE_DIR LC_ALL PWD OLDPWD SHLVL _"
_FGIT_WIRE_EXEC="$(git --exec-path 2>/dev/null)" || _FGIT_WIRE_EXEC=""
# State the parent reads back — assigned before anything reads it (nothing here
# relies on `set -u`; see docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md §5.2).
_FW_DIR="$SCRATCH/fgit-window"
_fw_rc=0; _fw_done=0; _fw_post_bad=0; _fw_why="the window was never started"; _fw_diag=""; _fw2_said=0
# The postconditions, run INSIDE the window after the build. $1..$9 = labels:
# P-a, P-a-live, P-b, P-b-live, P-c, P-d, P-e, P-f, P-g. Returns 1 if any
# reported.
_fgit_postconditions() {
  _fpv=0
  _pq="$_FW_DIR/pq"; mkdir -p "$_pq/a" "$_pq/b" || return 1
  ( cd "$_pq/a" && git init -q . ) >/dev/null 2>&1 || true
  ( cd "$_pq/b" && git init -q --template="$_FGIT_VOID" . ) >/dev/null 2>&1 || true
  _pa_live="$(cd "$_pq/a" && git -c a.b=c config --list --show-scope --show-origin 2>/dev/null | awk -F'\t' '$1=="command"{print "command"; exit}')" || _pa_live=""
  _pa_bad="$(cd "$_pq/a" && git config --list --show-scope --show-origin 2>&1 | awk -F'\t' '!($1=="local" && $2=="file:.git/config")')" || _pa_bad="(config --list failed)"
  if [ "$_pa_live" != command ]; then echo "!! CONTROL NOT EXERCISED ($2)" >&2; _fpv=1
  elif [ -n "$_pa_bad" ]; then echo "!! CONTROL FAILED ($1): $(printf '%s' "$_pa_bad" | tr '\n' ' ')" >&2; _fpv=1; fi
  _pbd=""; _pbl=""
  for _v in GIT_CONFIG_SYSTEM GIT_ATTR_SYSTEM; do
    _o="$(GIT_CONFIG_NOSYSTEM=0 GIT_ATTR_NOSYSTEM=0 git var "$_v" 2>/dev/null)" || _o=""
    [ -n "$_o" ] || _pbl="$_pbl $_v"
    if _o="$(git var "$_v" 2>/dev/null)"; then _pbd="$_pbd $_v=[$_o]"; fi
  done
  for _v in GIT_CONFIG_GLOBAL GIT_ATTR_GLOBAL; do
    if ! _o="$(git var "$_v" 2>/dev/null)"; then _pbd="$_pbd $_v:rc!=0"; continue; fi
    while IFS= read -r _l; do case "$_l" in "$_FGIT_VOID"/*) : ;; *) _pbd="$_pbd $_v=[$_l]";; esac; done <<EOF_PB
$_o
EOF_PB
  done
  if [ -n "$_pbl" ]; then echo "!! CONTROL NOT EXERCISED ($4):$_pbl" >&2; _fpv=1
  elif [ -n "$_pbd" ]; then echo "!! CONTROL FAILED ($3):$_pbd" >&2; _fpv=1; fi
  if ! diff -r "$_pq/a/.git" "$_pq/b/.git" >/dev/null 2>&1; then echo "!! CONTROL FAILED ($6)" >&2; _fpv=1; fi
  _o="$(git --exec-path 2>/dev/null)" || _o=""
  if [ -z "$_o" ] || [ "$_o" != "$_FGIT_WIRE_EXEC" ]; then echo "!! CONTROL FAILED ($7): [$_o] vs [$_FGIT_WIRE_EXEC]" >&2; _fpv=1; fi
  _pf=""
  while IFS= read -r -d '' _rec; do
    _n="${_rec%%=*}"
    case " $_FGIT_ENV_NAMES " in *" $_n "*) : ;; *) _pf="$_pf $_n" ;; esac
  done < <("$_FGIT_ENVBIN" -0)
  if [ -n "$_pf" ]; then echo "!! CONTROL FAILED ($8):$_pf" >&2; _fpv=1; fi
  # P-g: every fixture repo's persisted configuration is exactly what a
  # reference `git init` in this window writes — derived from git itself, per
  # run, so no key list: anything a fixture persisted beyond it is red.
  _pgr="$_FW_DIR/pgref"; mkdir -p "$_pgr" && ( cd "$_pgr" && git init -q . ) >/dev/null 2>&1 || true
  _pgref="$_FW_DIR/pgref.lines"
  git -C "$_pgr" config --list --show-scope --show-origin > "$_pgref" 2>/dev/null || : > "$_pgref"
  _pg=""
  if [ ! -s "$_pgref" ]; then _pg=" (no reference configuration)"; fi
  # Population by property: EVERY git dir the fixtures produced, anywhere under
  # $CTL (hidden and nested included, and inside `.git` dirs): each `.git`
  # entry of any type, and each directory shaped like a git dir (a HEAD file
  # plus an `objects` dir or a `commondir` file). Only a `.git` that is a real
  # directory has its config compared; every other shape (a gitfile, a symlink,
  # a bare, separate, submodule or linked-worktree git dir) is UNKNOWN and red,
  # never skipped. "Cannot search" is whatever `find` reports: any report fails
  # the census, and a failed census is red, as is one that finds no git dir.
  # The fixtures make three directories unsearchable ON PURPOSE. Each must have
  # exactly its declared mode (octal, and as `ls -ld` prints it); it is opened
  # (u+rx) for the census and restored to that mode after, so what is inside it
  # is examined too.
  _pg_declared="walk/sub:000:d--------- d5root:000:d--------- d2red/sub:444:dr--r--r--"
  _pg_mode_is() { # $1 = path, $2 = the ls -ld mode string
    _pgs="$(ls -ld "$1" 2>/dev/null)" || return 1
    [ "${_pgs:0:10}" = "$2" ]
  }
  _pg_opened=""
  for _pgm in $_pg_declared; do
    _pgp="$CTL/${_pgm%%:*}"; [ -e "$_pgp" ] || continue
    if _pg_mode_is "$_pgp" "${_pgm##*:}" && chmod u+rx "$_pgp" 2>/dev/null; then
      _pg_opened="$_pg_opened $_pgm"
    else
      _pg="$_pg ${_pgm%%:*}:[not at its declared mode ${_pgm##*:}, or could not be opened]"
    fi
  done
  _pgpop="$_FW_DIR/pgpop"; _pgn=0
  find "$CTL" \( -name .git -print0 \) -o \( -type f -name HEAD -print0 \) > "$_pgpop" 2>"$_FW_DIR/pgpop.err" \
    || : > "$_FW_DIR/pgpop.failed"
  if [ -e "$_FW_DIR/pgpop.failed" ] || [ -s "$_FW_DIR/pgpop.err" ]; then
    _pg="$_pg (the git-dir census under the fixture root failed: $(head -3 "$_FW_DIR/pgpop.err" | tr '\n' ';'))"
  fi
  while IFS= read -r -d '' _pge; do
    if [ "${_pge##*/}" = HEAD ]; then
      _pgd="${_pge%/HEAD}"
      [ "${_pgd##*/}" != .git ] || continue            # the ordinary git dir, counted by its `.git` entry
      if [ -d "$_pgd/objects" ] || [ -f "$_pgd/commondir" ]; then _pg="$_pg ${_pgd#"$CTL"/}:[a git dir not named .git]"; fi
      continue
    fi
    _pgn=$((_pgn + 1)); _pgd="${_pge%/.git}"; _pgl="${_pgd#"$CTL"/}"
    if [ -L "$_pge" ] || [ ! -d "$_pge" ]; then _pg="$_pg $_pgl:[.git is not a directory]"; continue; fi
    _pgx="$(git -C "$_pgd" config --list --show-scope --show-origin 2>&1 | grep -vxF -f "$_pgref")" || true
    [ -z "$_pgx" ] || _pg="$_pg $_pgl:[$(printf '%s' "$_pgx" | tr '\t\n' ' ;')]"
  done < "$_pgpop"
  for _pgm in $_pg_opened; do
    _pgp="$CTL/${_pgm%%:*}"
    _pgo="${_pgm#*:}"; _pgo="${_pgo%%:*}"
    { chmod "$_pgo" "$_pgp" 2>/dev/null && _pg_mode_is "$_pgp" "${_pgm##*:}"; } \
      || _pg="$_pg ${_pgm%%:*}:[could not be restored to its declared mode $_pgo]"
  done
  [ "$_pgn" -gt 0 ] || _pg="$_pg (no fixture git dir was found)"
  if [ -n "$_pg" ]; then echo "!! CONTROL FAILED (${9}):$_pg" >&2; _fpv=1; fi
  # P-c last: nothing in the window may have written into the void.
  if [ -n "$(ls -A "$_FGIT_VOID" 2>/dev/null)" ]; then echo "!! CONTROL FAILED ($5): $(ls -A "$_FGIT_VOID" | tr '\n' ' ')" >&2; _fpv=1; fi
  return "$_fpv"
}
# Run the fixtures file in the window. $1 = fixtures file, $2..$10 = the
# postcondition labels. Sets _fw_rc/_fw_done/_fw_post_bad/_fw_why/_fw_diag and
# _FIX_FAILED.
_fgit_window() {
  _fwf="$1"; shift
  mkdir -p "$_FW_DIR" || { _fw_why="the window's directory could not be created"; return 0; }
  {
    printf 'set -euo pipefail\n'
    # Plain assignments, never `declare -p`: that would carry an `export`
    # attribute into the window (measured: P-f caught it).
    for _fwn in CTL CONTROL_REMOVED CONTROL_K2 CONTROL_TOOLS CONTROL_BINARY CONTROL_CLEAN \
      _REAL_GIT _REAL_GREP _fifo_ok _FGIT_VOID _FGIT_ENVBIN _FGIT_ENV_NAMES _FGIT_WIRE_EXEC _FW_DIR; do
      printf '%s=%q\n' "$_fwn" "${!_fwn:-}"
    done
    printf '_FIX_FAILED=""\n'
    declare -f _fixture_failed _shq _fgit_postconditions
  } > "$_FW_DIR/prelude.sh" || { _fw_why="the window prelude could not be written"; return 0; }
  _fw_rc=0
  "$_FGIT_ENVBIN" -i "${_FGIT_ENV[@]}" "$_FGIT_BASH" -c '
    . "$1"
    case ":$SHELLOPTS:" in *:errexit:*) : ;; *) echo 3 > "$_FW_DIR/cause"; exit 3 ;; esac
    case ":$SHELLOPTS:" in *:nounset:*) : ;; *) echo 3 > "$_FW_DIR/cause"; exit 3 ;; esac
    case ":$SHELLOPTS:" in *:pipefail:*) : ;; *) echo 3 > "$_FW_DIR/cause"; exit 3 ;; esac
    . "$2"; shift 2
    [ -e "$_FW_DIR/built" ] || { echo 4 > "$_FW_DIR/cause"; exit 4; }
    case ":$SHELLOPTS:" in *:errexit:*:nounset:*|*:nounset:*:errexit:*) : ;; *) echo 5 > "$_FW_DIR/cause"; exit 5 ;; esac
    case ":$SHELLOPTS:" in *:pipefail:*) : ;; *) echo 5 > "$_FW_DIR/cause"; exit 5 ;; esac
    printf "%s" "$_FIX_FAILED" > "$_FW_DIR/fix_failed"
    _fgit_postconditions "$@" || : > "$_FW_DIR/post_bad"
    : > "$_FW_DIR/done"' _ "$_FW_DIR/prelude.sh" "$_fwf" "$@" 2> "$_FW_DIR/stderr" || _fw_rc=$?
  cat "$_FW_DIR/stderr" >&2 2>/dev/null || true
  # A shell diagnostic located in the fixtures file (bash's own "<file>: line N:"
  # form, English under the window's LC_ALL=C) means a top-level command was
  # skipped — an arithmetic-expansion error does not stop a sourced file.
  # No pipe into `head`: under pipefail its early exit SIGPIPEs grep (141) once
  # the output is long, which must not read as "no diagnostic". grep rc 1 is
  # "none"; any other non-zero status is itself red.
  _fw_dg=0; _fw_diag="$(grep -m 3 -F "$_fwf: line " "$_FW_DIR/stderr" 2>&1)" || _fw_dg=$?
  case "$_fw_dg" in
    0) : ;;
    1) _fw_diag="" ;;
    *) _fw_diag="(the scan of the window's stderr failed: grep exited $_fw_dg)" ;;
  esac
  if [ -e "$_FW_DIR/done" ]; then _fw_done=1; else
    # The cause comes from a marker the child writes, not from the exit status,
    # which a fixtures-file command can produce on its own under errexit.
    _fw_cause="$(cat "$_FW_DIR/cause" 2>/dev/null)" || _fw_cause=""
    case "$_fw_cause" in
      3) _fw_why="the window refused to start: a prelude option (errexit, nounset or pipefail) was not in force" ;;
      4) _fw_why="the fixtures file returned before its last line" ;;
      5) _fw_why="the fixtures file switched off errexit, nounset or pipefail" ;;
      *) _fw_why="the window exited $_fw_rc before completing (the fixtures file exited or aborted, or a postcondition aborted)" ;;
    esac
  fi
  [ ! -e "$_FW_DIR/post_bad" ] || _fw_post_bad=1
  _FIX_FAILED="$(cat "$_FW_DIR/fix_failed" 2>/dev/null)" || _FIX_FAILED=""
  return 0
}
# An INCOMPLETE window is not "no fixture failed": nothing was built, so no
# control may run over it. Report W alone and end the run "decided nothing"
# (2, the wire's convention; the number is not consumed by the driver).
_fgit_window_incomplete_exit() { # $1 = the window label
  [ "$_fw_done" -ne 1 ] || return 0
  echo "!! CONTROL NOT EXERCISED ($1): $_fw_why; nothing was built, so no control was run" >&2
  exit 2
}
_fgit_window_verdict() { [ "$_fw_post_bad" -eq 0 ]; }

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
# the cleanup path is the physical absolute one the wire resolves it to (#501
# R55: an unchecked `mktemp` made an `rm -rf` expand to the repo root).
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

# ⚠ A PATH EMBEDDED IN A GENERATED SCRIPT IS QUOTED, BY ONE HELPER. A shim that
# embeds a path — a real tool's, or a fixture's — writes it into `/bin/sh`
# source; spliced in bare, a path
# holding a space or a shell metacharacter split into words and every shim went
# invalid, so the controls using them exited for the wrong reason (PR519,
# reproduced by the external reviewer with git at `/tmp/tool space/git`). Same
# lesson as #501 R96 on `_ctl_env`: a path is data. Single quotes are the one
# `/bin/sh` quoting with no expansion inside; an embedded `'` is closed, escaped
# and reopened.
_shq() { printf "'%s'" "$(printf '%s' "$1" | sed "s/'/'\\\\''/g")"; }
_REAL_GIT="$(_shq "$(command -v git)")"
_REAL_GREP="$(_shq "$(command -v grep)")"
# ⚠ ASSERTED, NOT A `_control`: this is the harness's own part, not an arm of the
# wire. (The mutation set can now aim a record at this file with `harness:`;
# this check does not depend on one.) It is checked by round-tripping a path
# that holds each thing that broke it.
_shq_probe="/tool dir/it's \$HOME \`x\`"
if [ "$(sh -c "printf %s $(_shq "$_shq_probe")")" != "$_shq_probe" ]; then
  echo "!! the shim-quoting helper does not round-trip a path through /bin/sh, so every" >&2
  echo "   shim built with it may exec the wrong thing. This run decided nothing." >&2
  exit 2
fi

# No control may run over an unbuilt tree, WHEREVER the incomplete-window exit
# sits (docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md §3): every `_control` asks first. `$_fw2_lbl` is defined in
# the controls file.
_fw_built_or_w2() {
  [ "$_fw_done" -ne 1 ] || return 0
  [ "$_fw2_said" -eq 1 ] || echo "!! CONTROL FAILED ($_fw2_lbl): a control was reached over an unbuilt tree" >&2
  _fw2_said=1; return 1
}
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
  _fw_built_or_w2 || return 1
  case " $_FIX_FAILED " in
    *" ${1##*/} "*)
      echo "!! CONTROL NOT EXERCISED ($4): its fixture did not build, so this control" >&2
      echo "   would be asserting over a tree that never posed the question." >&2
      return 1 ;;
  esac
  _out_f="$CTL/.control_out"
  # ⚠ SELF-TEST MODE IS AN ARGUMENT (see the wire, beside `_SELFTEST`): it is
  # the one channel a shell cannot leave behind for a later ordinary run.
  # ⚠ `set -m` SO THE CHILD IS ITS OWN PROCESS GROUP. Without it the watchdog's
  # `kill -9 "$_cpid"` reaches only the wire's own bash; a command substitution
  # or a `grep` BLOCKED beneath it — precisely the FIFO hang this watchdog
  # exists to catch — is reparented and stays blocked after the gate reports
  # the timeout, so repeated local runs accumulate permanent orphans. Measured
  # on bash 5.3 and 3.2: group kill reaps the descendant, top-PID kill does not.
  set -m
  PATH="${7:+$7:}$PATH" \
    env ${_ctl_env[@]+"${_ctl_env[@]}"} "$SELF" --selftest "$1" "${5:-}" "${6:-}" \
    > "$_out_f" 2>&1 & _cpid=$!
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
