#!/usr/bin/env bash
# THE CONTROL HARNESS for `webref-generic-core-trip-wire.sh` — sourced by that
# wire's controls file, never run on its own.
#
# WHY IT IS A SEPARATE FILE: CLAUDE.md's touch-time split, taken when the
# controls file was about to pass the line threshold. The seam is HOW a control
# runs versus WHICH controls exist: this file holds the scratch root the
# fixtures live in, the git helper that builds them, the shim-quoting helper,
# the FIFO capability probe and `_control` itself; the controls file holds each
# fixture and the assertion made over it.
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
# change what they contain (#501 R92). A global `core.excludesFile` of `*.py`
# made `git add -A` skip the fixtures' own files, and a template directory seeds
# `.git/info/exclude` at `init` to the same effect. `_fgit` is what the fixtures
# are built with, and it builds them from a KNOWN-EMPTY git environment.
# ⚠ PER CALL, NOT `export` (#501 R94). Exported, it applied to the REAL scan
# too and took `safe.directory` with it — measured: a checkout owned by
# another UID, readable only because of a global `safe.directory` entry, went
# from working to "detected dubious ownership", so the required gate could not
# run in an otherwise valid environment. The fixtures need a clean config; the
# repository needs the user's. The inventory is made machine-independent by
# `--exclude-per-directory` instead (see `_scan`), which is why nothing has to
# be stripped for the real read.
# ⚠ AND IT SCRUBS THE CALLER'S GIT ENVIRONMENT, which `_git` deliberately does
# NOT. `_git` keeps `GIT_CONFIG*` because clearing it broke a checkout that was
# only readable through a caller's `safe.directory` (#501 R97) — right for the
# REAL scan, wrong for the fixtures, which must be built from a known
# configuration whatever the caller carries. The asymmetry is the point and it is
# why there are two helpers: PRESERVE for the repository, SCRUB for the fixtures.
# ⚠ AND THE SCRUB IS THE COMPLEMENT OF A KEEP-SET, NOT A LIST OF NAMES TO DROP.
# It is the SECOND failure of that enumeration that says it has to be. Pointing
# `GIT_CONFIG_GLOBAL`/`_SYSTEM` at `/dev/null` did not reach
# `GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=core.excludesFile
#  GIT_CONFIG_VALUE_0=<file holding *.py>`, which made the fixtures' `git add`
# skip their own `.py` inputs on a clean checkout (#501 R97) — so three names
# were then unset by hand. That list did not hold `GIT_TEMPLATE_DIR`, which is in
# neither it nor `git rev-parse --local-env-vars`, so a caller's template reached
# every `_fgit init` (PR501, external reviewer; reproduced here):
#
#     mkdir -p /tmp/tpl/info && printf '*.py\n' > /tmp/tpl/info/exclude
#     GIT_TEMPLATE_DIR=/tmp/tpl bash .claude/tools/webref-generic-core-trip-wire.sh
#
# A list that leaves the next variable authoritative twice is not a list to
# extend a third time. In a REQUIRED gate an unknown must fall on the side that
# is NEUTRALISED, not the side that leaks in — so every `GIT_*` the caller
# carries goes, and what the fixtures NEED is what gets enumerated. The
# assignment below IS the keep-set; the arm over it is the control named
# `a hostile caller git environment cannot change what a fixture holds`.
# ⚠ AND THE KEEP-SET IS GROUNDED IN GIT'S CONFIGURATION LAYERS, NOT IN A
# PER-VARIABLE MEASUREMENT. It used to say "MEASURED, NOT REASONED", which makes
# membership empirical one name at a time — and a member with no available test
# then has to be written up as an exception. An exception inside a keep-set is the
# enumeration shape the paragraph above just abandoned.
# The layers are a closed list (`git help config`, FILES and ENVIRONMENT; read at
# git 2.55): system `$(prefix)/etc/gitconfig`; global `$XDG_CONFIG_HOME/git/config`
# and `~/.gitconfig`; local `$GIT_DIR/config`; worktree `$GIT_DIR/config.worktree`;
# `-c` on the command line; and the env-carried pairs `GIT_CONFIG_COUNT` /
# `GIT_CONFIG_KEY_<n>` / `GIT_CONFIG_VALUE_<n>`. Of those, the sweep reaches the
# env-carried layer and only that one: local and worktree belong to the repository
# `_fgit` itself creates, `-c` is this file's own argv, and system and global are
# named by PATHS OUTSIDE THE ENVIRONMENT — reachable only through git's own two
# overrides. So the keep-set is exactly the overrides for the layers the sweep
# cannot reach, which is why it has two members and not a third.
# ⚠ AND ONE UNIFORM METHOD CONFIRMS BOTH: SUBSTITUTION. Point either at a
# `.gitconfig` whose `core.excludesFile` names a file holding `*.py`, instead of
# at `/dev/null`, and the controls red — measured separately for each, 12 control
# failures and exit 1 both times. REMOVAL is not the method and cannot be for the
# system layer: `/etc/gitconfig` needs root to make hostile, so a removal test
# there decides nothing. Substitution shows git reads the variable and that the
# layer is live during a fixture build, which is the whole claim being made.
# ⚠ AND A LAYER GIT ADDS LATER HAS A RULE, NOT A PRECEDENT: its override joins
# the keep-set iff the sweep cannot reach that layer, and is confirmed the same
# way. The two members here are not older or better-tested than a third would be.
# ⚠ `${!GIT_@}` IS THE ENUMERATOR, on both shells this wire is run under rather
# than on whichever one is to hand: bash 3.2.57 — the stock macOS shell it commits
# to — supports the prefix form, and the QUOTED spelling is the one used, because
# it yields one word per name whatever `IFS` holds and zero words under `set -u`
# when nothing matches.
# ⚠ `2>/dev/null || true` PER NAME. bash 3.2 LISTS an entry like `GIT_X-Y` and
# then refuses to `unset` it ("not a valid identifier"); without this the refusal
# printed five times — once per `_fgit` call whose stderr is not redirected — and
# the run still exited 0, because `set -e` is suspended inside the `&&` chains the
# fixtures are built as. Whether such an entry is present is not this sweep's
# question to answer; it is the probe's, below, and the probe must survive the
# sweep in order to answer it.
# ⚠ In a subshell (`( … )`), so neither the unsets nor the assignments can leak
# into the real scan.
_fgit_scrub() { for _fv in "${!GIT_@}"; do unset "$_fv" 2>/dev/null || true; done; }
_fgit() ( _fgit_scrub; GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_SYSTEM=/dev/null _git "$@" )

# DOES THAT SWEEP ACTUALLY EMPTY THE ENVIRONMENT BLOCK? Asked once, before a
# fixture is built, because the sweep above is worth exactly what the answer is.
# ⚠ `env` IS THE AUTHORITY, NOT `${!GIT_@}`. This used to be a precondition that
# walked bash's own list and asked whether each member could be unset — i.e. it
# verified the sweep against THE VERY ENUMERATOR UNDER TEST, which is the defect
# the sweep above was written to fix, one level up. Measured with
#
#     env 'GIT_X-Y=1' GIT_TEMPLATE_DIR=/tmp/tpl bash -c 'echo "${!GIT_@}"; command env'
#
# bash 3.2.57 lists `GIT_EDITOR GIT_TEMPLATE_DIR GIT_X-Y` and the old block fired;
# bash 5.3.20 lists only the first two and it was BLIND — blind on the shell where
# the residual survives. `env` reports all three on both. So the question is put
# the other way round: sweep, then ask the environment block what is left.
# ⚠ AND IT ASKS IN BASH, NOT THROUGH `grep`. `grep` on `PATH` is whatever `PATH`
# resolves (the wire's D12 row), this file runs under `pipefail` where a `grep`
# selecting nothing exits 1, and hard-coding `/usr/bin/grep` would start a second
# absolute-path inventory of external tools. A `case` needs none of them.
# ⚠ AND `env` PRODUCING NOTHING IS "DECIDED NOTHING", not "nothing leaked": a
# probe whose observer failed must not read as a clean sweep.
# ⚠ AND THE PROBE KEEPS ITS OWN COPY OF THE LOOP rather than calling
# `_fgit_scrub`, which is the one thing it would be tidier to do. Measured: with
# the probe calling `_fgit_scrub`, rewriting that function to the three-name
# `unset` it replaced makes THIS BLOCK exit 2 at the top of the run — so the
# mutation record that pins this sweep reported `killed for the WRONG REASON`
# instead of naming its control, and whether it did so depended on whether the
# caller's environment happened to carry a removable `GIT_*` (`GIT_EDITOR=true`
# here). A check that is a detector for the payload of the thing it checks
# pre-empts the control that proves that payload end to end, nondeterministically.
# The loop below is therefore the ENUMERATOR UNDER TEST, not a call into
# production; the two are kept identical, and the `harness:` mutation record's
# anchor IS `_fgit_scrub`'s body, so editing the sweep without editing the probe
# reds as `MATCHED NOTHING`.
# ⚠ ASSERTED, NOT A `_control`, for the same reason `_shq`'s probe below is: the
# question is about THIS process's own environment, and a `_control` can only set
# the environment of a wire it RE-INVOKES. Each re-invocation runs this block
# again over its own environment, which is how the hostile fixture's child is
# covered too.
# ⚠ AND IT REFUSES WITHOUT CLAIMING THE RESIDUAL IS DANGEROUS — that distinction
# is load-bearing, because "it reaches git's environment" invites the next reader
# to try to HANDLE `GIT_X-Y`, and there is nothing there to handle. A name bash
# cannot `unset` is not a valid shell identifier, and every name git reads IS one:
# measured at git 2.55, every `GIT*` environment variable named in git(1), in
# git-config(1) and by `git rev-parse --local-env-vars` is spelled `[A-Z0-9_]+`,
# with no exception in the complement (the only non-conforming token either page
# holds is the man-page title `GIT-CONFIG`). So the two sets are DISJOINT and the
# live exposure is nil. What is nevertheless unproven is the sweep's own claim to
# be complete over the environment block — and reporting a pass over an unproven
# sweep is the defect this helper is about, one level up. So the run stops.
_fgit_block="$( ( for _fv in "${!GIT_@}"; do unset "$_fv" 2>/dev/null || true; done
                  command env ) )" || _fgit_block=""
if [ -z "$_fgit_block" ]; then
  echo "!! \`env\` reported nothing about this process's own environment block, so the" >&2
  echo "   fixture git scrub could not be checked at all and this run decided nothing." >&2
  exit 2
fi
# ⚠ `(GIT_*)`, WITH THE LEADING PAREN. Inside `$( … )` bash 3.2.57 takes the `)`
# that closes a bare `case` pattern as the one that closes the substitution:
# "syntax error near unexpected token `newline'", then `_fl: unbound variable`.
# `bash -n` does NOT catch it — the substitution is parsed at execution — so it
# reached a run, the same class as the `_mutants` note in the mutation set.
# The balanced form parses on both; measured 3.2.57 and 5.3.20.
_fgit_leak="$(printf '%s\n' "$_fgit_block" | while IFS= read -r _fl; do
  case "$_fl" in (GIT_*) printf '%s\n' "$_fl" ;; esac
done)"
if [ -n "$_fgit_leak" ]; then
  echo "!! THIS RUN CANNOT PROVE THE FIXTURE GIT SCRUB DID WHAT IT CLAIMS. After" >&2
  echo "   sweeping every name bash lists as \`GIT_*\`, \`env\` still reports:" >&2
  printf '%s\n' "$_fgit_leak" | sed 's/^/     /' >&2
  echo "   Those entries are NOT a channel into git and there is nothing to handle in" >&2
  echo "   them: a name bash cannot unset is not a shell identifier, and every variable" >&2
  echo "   git reads IS one (measured — see the note above this block). What fails is" >&2
  echo "   the sweep's own claim — to be COMPLETE over the environment block — and" >&2
  echo "   reporting a pass over an unproven sweep is the defect this helper is about," >&2
  echo "   one level up. So the run ends at \"decided nothing\" instead. (An entry that" >&2
  echo "   is not a \`GIT_*\` but whose VALUE holds a line beginning \`GIT_\` lands here" >&2
  echo "   too: an unknown falls on the side that stops.)" >&2
  exit 2
fi

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
# wire, so there is no fixture to pose the question over. (The reason used to be
# that the mutation set edits only the wire; it is no longer wire-only — see its
# `harness:` prefix — but a record has to name a control, and this is not one.) It
# is checked by round-tripping a path that holds each thing that broke it.
_shq_probe="/tool dir/it's \$HOME \`x\`"
if [ "$(sh -c "printf %s $(_shq "$_shq_probe")")" != "$_shq_probe" ]; then
  echo "!! the shim-quoting helper does not round-trip a path through /bin/sh, so every" >&2
  echo "   shim built with it may exec the wrong thing. This run decided nothing." >&2
  exit 2
fi

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
