#!/usr/bin/env bash
# THE CONTROLS FOR `webref-generic-core-trip-wire.sh` — sourced by it, never run
# on its own.
#
# WHY IT IS A SEPARATE FILE: the wire crossed 1000 lines, and CLAUDE.md's
# touch-time split discipline says to split at a real cohesion seam when you
# touch such a file — not to defer it. This is that seam, and it was visible
# well before the line count: the wire ANSWERS about a tree, and everything here
# exists to show those answers are reachable. Not all of them are covered — the
# verdict sites with no control are a defer slot in
# 2026-09-citation-hygiene-Ai-wire-k2-trip-wire.md §8. One
# green run of the scanner tells you nothing this file has not earned.
#
# THE INTERFACE, AND IT IS ASSERTED BELOW RATHER THAN DESCRIBED. What this file
# (and the harness it sources) CONSUMES from the wire: `$SELF` (re-invoked per
# control), `$SCRATCH` (the one scratch root, whose trap also cleans up `$CTL`),
# `$_CONTROLS` (this file's own path, which the mutation set reads labels from), the
# five `CONTROL_*` sample strings, and the `_git` helper. That list — and only
# that list — is checked at entry.
# ⚠ WHAT THIS FILE DEFINES IS NOT AN INTERFACE, and a previous revision said it
# was: it named `$CTL`, `_control`, `_ctl_env`, `$_perm_line`,
# `$_fifo_line` and `ctl_ok` "for the wire to read", and the wire reads NONE of
# them (`grep -c '_perm_line\|_fifo_line\|_ctl_env\|\$CTL\|ctl_ok\|\b_control\b'`
# over the wire → **0**; the earlier spelling of this command left `_control`
# out, and it needs `\b` because a comment there names the sibling wire's
# `ban_control`). They belong to the controls — defined here or in the
# harness beside this file. The data flow is one-way, and saying otherwise
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
# THE HARNESS — how a control runs — LIVES BESIDE THIS FILE; this file is which
# controls exist. Its absence ends the run at "decided nothing", as this file's
# own absence does in the wire.
_HARNESS="${SELF%.sh}.harness.sh"
if [ ! -r "$_HARNESS" ]; then
  echo "!! the control harness beside these controls ($_HARNESS) is missing or" >&2
  echo "   unreadable, so no control here can run. This run decided nothing." >&2
  exit 2
fi
# shellcheck source=/dev/null
. "$_HARNESS"

# THE FIXTURE BUILD LIVES BESIDE THIS FILE: that one builds the trees, this one
# asserts over them. Its absence ends the run at "decided nothing", as the
# harness's does. It is NOT sourced here: the harness runs it in the fixture
# build window (`_fgit_window`), a child built from nothing, and checks the
# window's postconditions inside it.
_FIXTURES="${SELF%.sh}.fixtures.sh"
if [ ! -r "$_FIXTURES" ]; then
  echo "!! the fixture build beside these controls ($_FIXTURES) is missing or" >&2
  echo "   unreadable, so no control here has a tree to run over. This run decided nothing." >&2
  exit 2
fi
# ONE LABEL PER PRODUCER, each the text its diagnostic prints, and each named by
# a mutation record (the ratchet counts every `_lbl="…"` definition as well as
# every `_control` label). What each asserts is tabled in
# docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md §4.
_fw_lbl="the fixture build window completed"
_fw2_lbl="no control runs over an incomplete fixture build window"
_fwd_lbl="the fixtures file ran without a shell diagnostic"
_pa_lbl="a window git whose inputs no fixtures-file command altered reads configuration only from its repo's config file"
_pal_lbl="this git reports a non-local configuration scope"
_pb_lbl="the fixture git has no system or global layer outside the void"
_pbl_lbl="this git names its system files through git var"
_pc_lbl="nothing is written into the fixture git's void"
_pd_lbl="the fixture git copies no template"
_pe_lbl="no exec-path override reaches the fixture git"
_pf_lbl="the fixture build window's environment holds only its allowlist"
_pg_lbl="every fixture repo persists only the configuration a plain git init writes"
_ph_lbl="the fixture build window reads in the wire's locale"
_pi_lbl="the fixture repos use the files ref format"
_fws_lbl="every mode restriction a fixture sealed was applied"
# The `_p*_lbl` labels reach the window by name, through its prelude.
_fgit_window "$_FIXTURES"


# ⚠ Every _control call is an operand of `||`: `set -e` is suspended only
# inside such a command, so a bare call would abort the script on the first
# non-zero and the remaining arms would never run.
ctl_ok=0
_ctl_env=()   # per-control environment; `_control` clears it after each use
# THE WINDOW'S VERDICT, WRITTEN AFTER `ctl_ok=0`. An incomplete window built
# nothing, so it ends the run here with W alone ("decided nothing"); `_control`
# itself refuses over an unbuilt tree too (W2), wherever this line sits.
_fgit_window_incomplete_exit "$_fw_lbl"
[ "$_fw_post_bad" -eq 0 ] || ctl_ok=1
# A shell diagnostic located in the fixtures file means a line of it was
# skipped: an arithmetic-expansion error does not stop a sourced file.
if [ -n "$_fw_diag" ]; then
  echo "!! CONTROL FAILED ($_fwd_lbl): $(printf '%s' "$_fw_diag" | tr '\n' ' ')" >&2
  ctl_ok=1
fi
# A mode a fixture sealed and the window could not apply (or refused) is red
# here, with its own label: the controls gated on that mode having taken effect
# would otherwise be skipped as if this machine could not enforce it.
if [ -n "$_fw_seal_bad" ]; then
  echo "!! CONTROL FAILED ($_fws_lbl): $(printf '%s' "$_fw_seal_bad" | tr '\n' ';')" >&2
  ctl_ok=1
fi
# A postcondition this machine cannot evaluate (P-b: `git var` before git 2.42;
# P-f: an `env` without -0) is a machine limitation, reported below and not a
# red (the capability rule). The window writes one line per limitation.
_limit_lines=""
[ -z "$_fw_limits" ] || _limit_lines="$(printf '%s\n' "$_fw_limits" | sed 's/^/            ⚠ NOT EXERCISED on this machine: /')"
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
_control "$CTL/lsfail" 1 "the tracked inventory exited" "a failed TRACKED inventory fails closed" "" "" "$CTL/fakegit" || ctl_ok=1
_control "$CTL/wtlsfail" 1 "the worktree inventory exited" "a failed WORKTREE inventory fails closed" "" "" "$CTL/fakegitwt" || ctl_ok=1
# Not a `_control`: the question is what the run leaves BEHIND, which its exit
# status and output cannot say. Run from a directory of its own, so a leaked
# relative scratch lands where this block can see it.
# ⚠ Gated, like every `_control`, on the window having built the tree.
_rel_lbl="a relative scratch dir is removed on exit"
if ! _fw_built_or_w2; then ctl_ok=1
elif ( cd "$CTL/relcwd" && PATH="$CTL/fakerelmktemp:$PATH" "$BASH" "$SELF" --selftest "$CTL/clean" "" "" ) >/dev/null 2>&1; then
  if [ -n "$(ls -A "$CTL/relcwd")" ]; then
    echo "!! CONTROL FAILED ($_rel_lbl): left behind in $CTL/relcwd:" >&2
    ls -A "$CTL/relcwd" | sed 's/^/     /' >&2
    ctl_ok=1
  fi
else
  echo "!! CONTROL FAILED ($_rel_lbl): the run itself did not pass" >&2
  ctl_ok=1
fi
_control "$CTL/lstreefail" 1 "the HEAD inventory exited" "a failed HEAD inventory fails closed" "" "" "$CTL/fakegitls" || ctl_ok=1
_control "$CTL/headprobe" 1 "that ref EXISTS, so this HEAD is not unborn" "a failed HEAD PROBE is not an unborn HEAD" "" "" "$CTL/headprobe" || ctl_ok=1
_control "$CTL/catfail" 1 "symlink blob (staged) could not be read" "a failed staged-blob read is not a clean target" "" "" "$CTL/catfail" || ctl_ok=1
_control "$CTL/phantom" 1 "the inventory listed it but it is gone" "an inventoried path that vanished is not silently skipped" "" "" "$CTL/fakegitphantom" || ctl_ok=1
_control "$CTL/globspec" 1 "the inventory listed it but it is gone" "the vanished-path question is asked of a LITERAL path" "" "" "$CTL/fakegitglob" || ctl_ok=1
_control "$CTL/badref" 1 "does not name a branch" "a malformed HEAD ref is not an unborn repository" || ctl_ok=1
_control "$CTL/punctslash" 1 "K2: a" "punctuation BEFORE a slash is part of the path" || ctl_ok=1
_control "$CTL/atclaude" 0 "PASSED" "a PATH character before .claude is not a prose boundary" || ctl_ok=1
_control "$CTL/linestart" 1 "K2: a" "a reference at the start of a line fires" || ctl_ok=1
_control "$CTL/textgreen" 0 "PASSED" "running text that only looks like a two-segment reference stays green" || ctl_ok=1
_control "$CTL/slashname" 1 "entry NAME" "a stored path naming .claude after a slash, under tools, fires" || ctl_ok=1
_control "$CTL/pathgreen" 0 "PASSED" "a stored path that only looks like one stays green" || ctl_ok=1
_control "$CTL/slashtext" 1 "K2: a" "a reference written after a slash fires" || ctl_ok=1
_control "$CTL/finalone" 1 "K2: a" "a ONE-character final segment fires" || ctl_ok=1
_control "$CTL/interone" 1 "K2: a" "a ONE-character intermediate segment fires" || ctl_ok=1
_control "$CTL/pathfirstone" 1 "entry NAME" "a ONE-character first segment in a STORED path fires" || ctl_ok=1
_control "$CTL/midclass" 0 "PASSED" "the final segment's middle stops where its last character does" || ctl_ok=1
_control "$CTL/bnd" 1 "K2: a" "the leading boundary covers =, --opt=, :, a backtick, ** and ," || ctl_ok=1
_control "$CTL/orphan" 0 "PASSED" "an orphan branch is an unborn HEAD, not a read failure" || ctl_ok=1
_control "$CTL/ancestorlink" 1 "ancestor component is a symlink" "the worktree read does not traverse an ancestor symlink" || ctl_ok=1
# ⚠ GREEN-DIRECTION CONTROLS — they prove the wire does NOT red on a legitimate
# tree, which is the failure mode that gets a required gate switched off rather
# than fixed. ⚠ An earlier version of this comment said "TWO" and added
# "every other control here proves the wire can RED"; both halves were false
# when written, and the second one still is — derive the count instead of
# reading one:
#     awk -F'"' '/^ *_control /{gsub(/ /,"",$3); print $3}' <this file> | sort | uniq -c
# (`$3` is the EXPECTED-EXIT argument: 0 = green-direction, 1 = red, 2 = the
# wire refusing to decide. ⚠ The first spelling of this command printed `$4`,
# the expected MESSAGE, which counts nothing — a derivation offered in place of
# a claim has to be run once before it is written down.)
_control "$CTL/punct" 0 "PASSED" "closing punctuation is not a path segment" || ctl_ok=1
_control "$CTL/suffixpath" 0 "PASSED" "a segment merely ENDING in .claude is not the host path" || ctl_ok=1
# …and the self-test escape hatch, which must not be reachable from the
# environment. This is PR519 R8's reproduction, exactly: both old names
# exported, the companion set to the PID that IS the wire's parent here, and
# pointing at the CLEAN fixture. The argument names a violating one; an
# environment channel would scan the clean tree and exit 0.
_ctl_env=("WEBREF_WIRE_SELFTEST=$CTL/clean" "WEBREF_WIRE_SELFTEST_PPID=$$")
_control "$CTL/committed" 1 "(in HEAD)" "an exported SELFTEST cannot redirect the scan" || ctl_ok=1
_control "$CTL/no-such-fixture" 2 "--selftest needs a fixture root" "a missing self-test root decides nothing" || ctl_ok=1
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
_control "$CTL/globextra*[e]" 1 "K2: a" "a pattern character in the checkout path does not misplace the entry script" "scope" "side/entry" || ctl_ok=1
_control "$CTL/nulblob" 1 "holds a NUL" "a NUL-bearing staged symlink blob is not a path" || ctl_ok=1
_control "$CTL/stagedlink" 1 "(staged) ->" "a STAGED symlink target is a stored path" || ctl_ok=1
_control "$CTL/headlink" 1 "(in HEAD) ->" "a COMMITTED symlink target is a stored path" || ctl_ok=1
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
# does, for this control and every other.
_control "$CTL/empty" 2 "read 0 stored objects" "an empty scope fails loudly" || ctl_ok=1
_control "$CTL/unread" 2 "read 0 stored objects" "an inventoried entry that was never read is not counted as scanned" || ctl_ok=1
# A WALK KILLED MID-SCAN is not a verdict: `POSIXLY_CORRECT` plus the failing
# `cat` ends the `_scan` subshell after it has emitted `a.py`'s record.
_ctl_env=("POSIXLY_CORRECT=1")
_control "$CTL/catkill" 2 "did not complete" "a walk killed mid-scan is not a verdict" "" "" "$CTL/catfail" || ctl_ok=1
_control "$CTL/d2green" 0 "PASSED" "a tracked directory deleted wholesale stays green" || ctl_ok=1
_control "$CTL/d2file" 0 "PASSED" "a tracked directory replaced by a file stays green" || ctl_ok=1
_control "$CTL/d3f1" 1 "K2: a" "a ] inside a first segment" || ctl_ok=1
_control "$CTL/d3f2" 1 "K2: a" "a double quote inside a first segment" || ctl_ok=1
_control "$CTL/d3f3" 1 "K2: a" "a single quote inside a first segment" || ctl_ok=1
_control "$CTL/d3f4" 1 "K2: a" "a backtick inside a first segment" || ctl_ok=1
_control "$CTL/d3m1" 1 "K2: a" "a ] inside an intermediate segment" || ctl_ok=1
_control "$CTL/d3m2" 1 "K2: a" "a double quote inside an intermediate segment" || ctl_ok=1
_control "$CTL/d3m3" 1 "K2: a" "a single quote inside an intermediate segment" || ctl_ok=1
_control "$CTL/d3m4" 1 "K2: a" "a backtick inside an intermediate segment" || ctl_ok=1
_control "$CTL/d3nb" 1 "K2: a" "a quoted one-segment reference before a slash token fails safe" || ctl_ok=1
# A caller's `GREP_OPTIONS` over a violating file. BSD grep honours it, so
# without the wire's `unset` the file reads as "no match". ⚠ GNU grep — CI's —
# does not honour it (2026-09-citation-hygiene-Ai-wire-k2-trip-wire.md §11.1;
# not measured here), and there this
# control passes whatever the wire does.
_ctl_env=("GREP_OPTIONS=--exclude=*")
_control "$CTL/k2" 1 "K2: a" "a caller's GREP_OPTIONS cannot hide a file" || ctl_ok=1
# A `core.fsmonitor` hook injected through the caller's `GIT_CONFIG*`, which the
# wire keeps. Not a `_control`: the question is whether a command RAN, which the
# verdict cannot say. ⚠ The hook is first shown to run under a plain git call
# over the same fixture, or its not running under the wire would prove nothing.
# The hook's path goes through `_shq`, as every embedded path here does.
_fsm_lbl="a caller's fsmonitor hook does not run"
if ! _fw_built_or_w2; then ctl_ok=1; else
_fsm_mark="$CTL/.fsmonitor_ran"
printf '#!/bin/sh\n: > %s\nexit 1\n' "$(_shq "$_fsm_mark")" > "$CTL/fsmhook"
chmod +x "$CTL/fsmhook"
_fsm_cfg=("GIT_CONFIG_COUNT=1" "GIT_CONFIG_KEY_0=core.fsmonitor" "GIT_CONFIG_VALUE_0=$(_shq "$CTL/fsmhook")")
( unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE; cd "$CTL/fsmon" \
  && env "${_fsm_cfg[@]}" git ls-files >/dev/null 2>&1 ) || true
if [ ! -e "$_fsm_mark" ]; then
  echo "!! CONTROL NOT EXERCISED ($_fsm_lbl): the hook did not run under a plain" >&2
  echo "   git call either, so its not running under the wire would prove nothing." >&2
  ctl_ok=1
else
  command rm -f "$_fsm_mark"
  _fsm_rc=0
  env "${_fsm_cfg[@]}" "$BASH" "$SELF" --selftest "$CTL/fsmon" "" "" > "$CTL/.fsm_out" 2>&1 || _fsm_rc=$?
  if [ "$_fsm_rc" -ne 0 ] || [ -e "$_fsm_mark" ]; then
    echo "!! CONTROL FAILED ($_fsm_lbl): exit $_fsm_rc; the hook ran: $([ -e "$_fsm_mark" ] && echo yes || echo no)" >&2
    sed 's/^/     /' "$CTL/.fsm_out" >&2
    ctl_ok=1
  fi
fi
fi
# ⚠ The line is built HERE, beside the decision that produces it. An earlier
# shape decided here and described it in the summary below, so the two could
# disagree — and an unconditional summary claimed exit-status evidence the run
# had not obtained (#501 R78). One site, one truth.
_perm_line="            ⚠ NOT EXERCISED on this machine: the controls that need file
          permissions enforced (this user can read a mode-000 file), so this
          run carries no evidence for them"
if [ -r "$CTL/err/control.py" ]; then
  :
else
  _perm_line="            the controls that need file permissions enforced ran too"
  _control "$CTL/err"  1 "could not be read" "an unreadable file fails closed" || ctl_ok=1
  _control "$CTL/walk" 1 "could not be read" "an unsearchable dir fails closed" || ctl_ok=1
  _control "$CTL/d2red" 1 "not provably absent" "a tracked file under an unsearchable dir is not absent" || ctl_ok=1
  _control "$CTL/d5root" 2 "or the root to a physical path" "an unresolvable self-test root decides nothing" || ctl_ok=1
  # Not a `_control`: `_control` has no umask channel. A umask that leaves the
  # scratch dir unsearchable makes it unresolvable to a physical path.
  _umask_lbl="a restrictive umask decides nothing"
  _um_rc=0
  if ! _fw_built_or_w2; then ctl_ok=1
  else ( umask 777; "$BASH" "$SELF" --selftest "$CTL/clean" "" "" ) > "$CTL/.umask_out" 2>&1 || _um_rc=$?
  fi
  if [ "$_fw_done" -eq 1 ] && { [ "$_um_rc" -ne 2 ] || ! grep -q "could not resolve the scratch dir" "$CTL/.umask_out"; }; then
    echo "!! CONTROL FAILED ($_umask_lbl): expected exit 2 naming the scratch dir, got $_um_rc" >&2
    sed 's/^/     /' "$CTL/.umask_out" >&2
    ctl_ok=1
  fi
fi
# ---- THE MUTATION SET, WHICH LIVES BESIDE THIS FILE -------------------------
# Two subjects, two files: this one runs the wire over a fixture and asserts the
# exit status; that one proves each control is about the arm it names. Its
# absence is not a skipped check — the correspondence between the two lists is
# what keeps either list honest, so the run ends at "decided nothing" rather
# than without it.
_MUTATIONS="${SELF%.sh}.mutations.sh"
if [ ! -r "$_MUTATIONS" ]; then
  echo "!! the mutation set beside these controls ($_MUTATIONS) is missing or" >&2
  echo "   unreadable, so nothing here was shown to be about the arm it names." >&2
  echo "   This run decided nothing." >&2
  exit 2
fi
# shellcheck source=/dev/null
. "$_MUTATIONS"
_mut_correspondence || ctl_ok=1

# (No check after the controls: every `_control` and every block that is not one
# asks `_fw_built_or_w2` before it runs, so none runs over an unbuilt tree.)
[ "$ctl_ok" -eq 0 ] || exit 1

_mut_run

echo "  controls: green reachable; K2 fires under both roots, on the removed path,"
echo "            inside binary content, on a symlink's stored target, on an entry's"
echo "            own NAME, and on a symlinked entry script beside the scope; an empty"
echo "            scope fails closed"
printf '%s\n' "$_fifo_line"
printf '%s\n' "$_perm_line"
[ -z "$_limit_lines" ] || printf '%s\n' "$_limit_lines"
echo "            (each asserted on this script's own exit status, over a fixture tree)"
