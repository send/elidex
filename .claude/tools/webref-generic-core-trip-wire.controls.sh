#!/usr/bin/env bash
# THE CONTROLS FOR `webref-generic-core-trip-wire.sh` — sourced by it, never run
# on its own.
#
# WHY IT IS A SEPARATE FILE: the wire crossed 1000 lines, and CLAUDE.md's
# touch-time split discipline says to split at a real cohesion seam when you
# touch such a file — not to defer it. This is that seam, and it was visible
# well before the line count: the wire ANSWERS about a tree, and everything here
# exists to show those answers are reachable. Not all of them are covered — the
# verdict sites with no control are a defer slot in the plan memo's §8. One
# green run of the scanner tells you nothing this file has not earned.
#
# THE INTERFACE, AND IT IS ASSERTED BELOW RATHER THAN DESCRIBED. What this file
# (and the harness it sources) CONSUMES from the wire: `$SELF` (re-invoked per
# control), `$SCRATCH` (the one scratch root, whose trap also cleans up `$CTL`),
# `$_CONTROLS` (this file's own path, which the mutation harness copies), the
# five `CONTROL_*` sample strings, and the `_git` helper. That list — and only
# that list — is checked at entry.
# ⚠ WHAT THIS FILE DEFINES IS NOT AN INTERFACE, and a previous revision said it
# was: it named `$CTL`, `_fgit`, `_control`, `_ctl_env`, `$_perm_line`,
# `$_fifo_line` and `ctl_ok` "for the wire to read", and the wire reads NONE of
# them (`grep -c '_perm_line\|_fifo_line\|_ctl_env\|_fgit\|\$CTL\|ctl_ok'` over
# the wire → **0**). They belong to the controls — defined here or in the
# harness beside this file; `ctl_ok` is consumed three lines from where it is
# set. The data flow is one-way, and saying otherwise invented a contract
# nobody could break.
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

for d in clean pin k2 tools binary err empty walk link odd nl seg cache cachedir extra name emptyname quotename nlname rawbyte forge linkname ignored lsfail lstreefail grepfail grepfaillink nltarget linkslash staged fifotracked notcommitted inscope stagedlink nulblob committed replaced routed routeddecoy cfgkept punct suffixpath headprobe catfail phantom punctslash badref globspec orphan atclaude ancestorlink external bnd wtlsfail catkill d2red d2green d2file d3f1 d3f2 d3f3 d3f4 d3m1 d3m2 d3m3 d3m4 d3nb d5root fsmon linestart textgreen slashname pathgreen; do mkdir -p "$CTL/$d"; done
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
# ⚠ ONLY THE TRACKED (`--stage`) INVENTORY FAILS, and the discriminator is that
# flag. This matched `" ls-files "`, which is in BOTH the tracked call and the
# worktree/untracked one — so one shim failed both, and the surviving error from
# whichever arm remained MASKED the removal of the other's status check.
# Measured by the external reviewer: deleting the worktree inventory's error
# emission left the whole run green. A shim shims the one call it is about.
# ⚠ ONLY `ls-files` FAILS. The shim used to answer every `git` invocation,
# which made it a control for whatever the wire asked git next rather than for
# a failing inventory — and #501 R95 added a `rev-parse` preflight that the
# blanket shim then answered with the fixture's own bytes. Same shape as
# `fakegrep`: a control shims the ONE call it is about.
printf '#!/bin/sh\ncase " $* " in *" --stage "*) printf "ok.py\\000"; exit 1;; esac\nexec %s "$@"\n' \
  "$_REAL_GIT" > "$CTL/fakegit/git"
chmod +x "$CTL/fakegit/git"
# …and its sibling for the WORKTREE/untracked inventory, which had no shim and
# therefore no control of its own.
mkdir -p "$CTL/fakegitwt"
printf '#!/bin/sh\ncase " $* " in *" --others "*) printf "ok.py\\000"; exit 1;; esac\nexec %s "$@"\n' \
  "$_REAL_GIT" > "$CTL/fakegitwt/git"
chmod +x "$CTL/fakegitwt/git"
printf '# %s\n' "$CONTROL_CLEAN"          > "$CTL/wtlsfail/ok.py"
# …AND THE SAME FOR `ls-tree`, because the HEAD inventory is a THIRD list with a
# status check of its own and nothing exercised it: `fakegit` fails `ls-files`
# only, so the `ls-tree` arm added at #501 R95 could be deleted and every
# control here stayed green (found by this slice's review — the one mutation
# record covering all three `_ls_rc` checks hid it, because killing the first
# was enough to red the run).
mkdir -p "$CTL/fakegitls"
printf '#!/bin/sh\ncase " $* " in *" ls-tree "*) printf "x\\000"; exit 1;; esac\nexec %s "$@"\n' \
  "$_REAL_GIT" > "$CTL/fakegitls/git"
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
# not answer" can be told apart. ⚠ BOTH TOKENS ARE NEEDED. `rev-parse` alone
# would also break the `--local-env-vars` call the wire makes at startup, and
# `--verify` alone breaks `show-ref --verify` — which is the very command that
# now establishes unbornness, so the shim silently became a control for the
# wrong arm and reported the wrong message. A shim shims the ONE call it is
# about, which here takes the verb AND the flag to pin down.
# ⚠ AS ONE ADJACENT SEQUENCE, not two globs. `*" rev-parse "*" --verify "*`
# cannot match: the first half consumes the space that the second half needs,
# so the shim matched nothing and the control silently exercised an unshimmed
# git — the failure a glob makes look like a passing fixture.
mkdir -p "$CTL/headprobe"
printf '#!/bin/sh\ncase " $* " in *" rev-parse --verify "*) exit 2;; esac\nexec %s "$@"\n' \
  "$_REAL_GIT" > "$CTL/headprobe/git"
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
  "$_REAL_GIT" "$_REAL_GIT" > "$CTL/fakegitphantom/git"
chmod +x "$CTL/fakegitphantom/git"
printf '# %s\n' "$CONTROL_CLEAN"          > "$CTL/phantom/ok.py"

#  (c) PUNCTUATION BEFORE A SLASH is not prose punctuation — the `/` settles
#      where the reference ends — so a path whose INTERMEDIATE segment ends in
#      one is a real K2 hit. This is the red-direction partner of `punct`, and
#      the two together are what pin the rule to the FINAL segment only:
#      `punct` alone admitted a predicate that missed this, and this one alone
#      admitted the predicate that reddened prose.
printf 'RULE = "%s"\n' '.claude/tools/team,/rule.md' > "$CTL/punctslash/control.py"
printf '# %s\n' "$CONTROL_CLEAN"          > "$CTL/punctslash/ok.py"

# A repository whose branch ref holds malformed data: `rev-parse --verify
# --quiet HEAD` exits 1 exactly as an unborn repository does, so this fixture is
# what separates "there is no commit" from "HEAD could not be read".
printf '# %s\n' "$CONTROL_CLEAN"          > "$CTL/badref/ok.py"

# A tracked `foo1.py` beside an untracked `foo[1].py` that the inventory names
# and the filesystem does not have. Without a literal pathspec the membership
# question matches the WRONG file and the vanished entry is passed over.
printf '# %s\n' "$CONTROL_CLEAN"          > "$CTL/globspec/foo1.py"

#  (d) `@` IS A PATH CHARACTER, NOT A PROSE BOUNDARY. `foo@.claude/...` is the
#      suffix of a component and must not match — the first boundary spelling
#      admitted it, because it was written as "not one of these few path
#      characters" instead of "one of these prose delimiters".
printf '# %s\n' 'foo@.claude/skills/team/rule.md' > "$CTL/atclaude/ok.py"
#      …and the rest of that exclusion set, one line per member group.
{ for _c in '~' '+' '%' '-' '.' '_' '0' 'o'; do
    printf '# fo%s.claude/skills/team/rule.md\n' "$_c"
  done; } >> "$CTL/atclaude/ok.py"

#  (e) THE RED DIRECTION OF THE SAME BOUNDARY RULE, which nothing tested until
#      the design re-gate measured it: the red fixtures then in place wrote
#      their paths after a SPACE or a QUOTE, so the leading class could be
#      narrowed to almost nothing and stay green. The lines below are the
#      spellings a contributor can write; which of them the scanned tree holds
#      today is derived at `$K2RE`, not claimed here.
{ printf 'DEFAULT=%s\n' "$CONTROL_K2"
  printf -- '--paths=%s\n' "$CONTROL_K2"
  printf 'key:%s\n' "$CONTROL_K2"
  printf 'see `%s` here\n' "$CONTROL_K2"
  printf '**%s**\n' "$CONTROL_K2"
  printf 'x,%s\n' "$CONTROL_K2"; } > "$CTL/bnd/control.py"

# THE BOUNDARY RULES' OTHER DIRECTIONS (plan memo §11, D10). Each was shown
# missing by a mutant that survived the control set as it then stood; the
# record beside each rule is that mutant.
#  (f) A reference at the very START of a line: the `^` half of the leading
#      boundary.
printf '%s\n' "$CONTROL_K2" > "$CTL/linestart/control.md"
#  (g) Running text that only LOOKS like a two-segment reference, one line per
#      rule it pins: another `.claude` root; `.claude` without its dot;
#      whitespace or an empty segment where a segment followed by `/` would
#      be; and a one-segment reference directly followed by a character the
#      final segment may not end in. The whitespace line is the shape
#      `cli.py`'s `--help` text uses.
{ printf '%s\n' '.claude/hooks/team/rule.md' 'see _claude/skills/team/rule.md' \
    '  .claude/tools/webref snapshot html --output /tmp/html-old.json' \
    '.claude/skills//rule.md' 'see .claude/tools/foo/ and/or more' '.claude/tools/foo//'
  for _c in '}' '>' ',' ';' '"' "'" '`' ']'; do
    printf '(%s.claude/tools/foo/%s)\n' "$_c" "$_c"
  done; } > "$CTL/textgreen/ok.md"
#  (h) A STORED path naming `.claude` after a `/`, under the `tools` root.
mkdir -p "$CTL/slashname/sub/.claude/tools/team"
printf '# %s\n' "$CONTROL_CLEAN" > "$CTL/slashname/sub/.claude/tools/team/rule.md"
printf '# %s\n' "$CONTROL_CLEAN" > "$CTL/slashname/ok.py"
#  (i) Stored paths that only LOOK like one: another `.claude` root, `.claude`
#      without its dot, and a symlink target whose first segment is empty.
mkdir -p "$CTL/pathgreen/.claude/hooks/team" "$CTL/pathgreen/_claude/skills/team"
printf '# %s\n' "$CONTROL_CLEAN" > "$CTL/pathgreen/.claude/hooks/team/rule.md"
printf '# %s\n' "$CONTROL_CLEAN" > "$CTL/pathgreen/_claude/skills/team/rule.md"
ln -s '.claude/skills//rule.md' "$CTL/pathgreen/entry"

# An ORPHAN branch with commits on another branch: HEAD is legitimately unborn
# while the repository is not empty, which is the case that separates "this HEAD
# has nothing committed" from "the repository has nothing committed".
printf '# %s\n' "$CONTROL_CLEAN"          > "$CTL/orphan/ok.py"

# A tracked directory replaced in the worktree by a symlink to somewhere else.
# The inventory then holds BOTH the untracked symlink and the formerly-tracked
# descendant, and reading the descendant follows the ancestor out of the tree.
printf 'RULE = "%s"\n' "$CONTROL_K2"      > "$CTL/external/a.py"
mkdir -p "$CTL/fakegitglob"
printf '#!/bin/sh\ncase " $* " in *" --others "*) %s "$@"; printf "foo[1].py\\000"; exit 0;; esac\nexec %s "$@"\n' \
  "$_REAL_GIT" "$_REAL_GIT" > "$CTL/fakegitglob/git"
chmod +x "$CTL/fakegitglob/git"
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
  "$_REAL_GIT" > "$CTL/fakegitcfg/git"
chmod +x "$CTL/fakegitcfg/git"
printf 'SRC = "%s"\n' "$CONTROL_K2"      > "$CTL/cfgkept/probe.py"
mkdir -p "$CTL/fakemktemp"
printf '#!/bin/sh\nd=%s/scratch\nmkdir -p "$d"\nprintf %%s "$d"\n' \
  "$(_shq "$CTL/inscope")" > "$CTL/fakemktemp/mktemp"
chmod +x "$CTL/fakemktemp/mktemp"
# A `mktemp` that answers with a RELATIVE path, as GNU `mktemp -d` does under a
# relative `TMPDIR` (macOS's ignores a relative `TMPDIR`, so the real tool
# cannot pose this question here).
mkdir -p "$CTL/relcwd" "$CTL/fakerelmktemp"
printf '#!/bin/sh\nd=relscratch.$$\nmkdir "$d" && printf %%s "$d"\n' > "$CTL/fakerelmktemp/mktemp"
chmod +x "$CTL/fakerelmktemp/mktemp"
printf '# %s\n' "$CONTROL_CLEAN"          > "$CTL/inscope/ok.py"
mkdir -p "$CTL/fakegrep"
printf '#!/bin/sh\ncase " $* " in *" -aEo "*) exit 2;; esac\nexec %s "$@"\n' \
  "$_REAL_GREP" > "$CTL/fakegrep/grep"
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
# …and one carrying the TERMINAL tag, which `_verdict` requires exactly once.
printf '# %s\n' "$CONTROL_CLEAN"          > "$CTL/forge/$(printf 'safe\nend\tscan')"
# A WALK KILLED MID-SCAN: the `catfail` geometry (built below) plus a tracked
# entry that sorts BEFORE the one whose read kills the walk, so the walk has
# emitted an `ok` before it dies. Run under `POSIXLY_CORRECT=1`.
printf '# %s\n' "$CONTROL_CLEAN"          > "$CTL/catkill/ok.py"
printf '# %s\n' "$CONTROL_CLEAN"          > "$CTL/catkill/a.py"
# A clean repository for the injected-`core.fsmonitor` control.
printf '# %s\n' "$CONTROL_CLEAN"          > "$CTL/fsmon/ok.py"
# A `]`, a quote or a backtick inside a segment FOLLOWED BY `/`: as the first
# segment, and LEADING a later intermediate one (`a/team]inc/…` already matched
# before the widening, through its `a/team` prefix, so only the leading position
# discriminates there).
_d3i=0
for _d3c in ']' '"' "'" '`'; do
  _d3i=$((_d3i + 1))
  printf 'X = %s\n' ".claude/tools/team${_d3c}inc/rule.md" > "$CTL/d3f$_d3i/c.py"
  printf 'X = %s\n' ".claude/tools/a/${_d3c}inc/rule.md"  > "$CTL/d3m$_d3i/c.py"
done
# …and the boundary that widening opens: a quoted ONE-segment reference followed
# directly by a `/`-bearing token now matches. Red is the fail-safe direction.
printf 'ARGS = [".claude/tools/webref","/tmp"]\n' > "$CTL/d3nb/c.py"

# Every fixture is a repository, because the population is git's answer:
# tracked, plus untracked minus ignored. A fixture that is not a repo cannot
# reproduce that distinction — and the distinction is now load-bearing.
for d in clean pin k2 tools binary err empty walk link odd nl seg cache \
         extra name emptyname quotename nlname rawbyte forge linkname ignored lstreefail grepfail grepfaillink nltarget linkslash staged fifotracked notcommitted inscope stagedlink nulblob committed replaced routed routeddecoy cfgkept punct suffixpath headprobe catfail phantom punctslash badref globspec atclaude bnd wtlsfail lsfail \
         catkill d3f1 d3f2 d3f3 d3f4 d3m1 d3m2 d3m3 d3m4 d3nb fsmon linestart textgreen \
         slashname pathgreen; do
  ( cd "$CTL/$d" 2>/dev/null && _fgit init -q . >/dev/null 2>&1 \
    && _fgit add -A >/dev/null 2>&1 ) || _fixture_failed "$d"
done
# ⚠ `cachedir` IS ABSENT FROM THAT LIST ON PURPOSE, built below in the only
# order that makes its force-add load-bearing.
# ⚠ `lsfail` USED TO BE ABSENT TOO, on the ground that its shim failed
# `ls-files` "whatever the directory is" — which stopped being true the moment
# that shim was narrowed to the TRACKED call alone. Its worktree inventory then
# ran for real against a non-repository, both lists came back empty, and the
# control got `read 0 stored objects` (exit 2) instead of the inventory error it
# names. **An exclusion justified by another mechanism's breadth expires when
# that mechanism is narrowed**, and nothing links the two but this note.
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
# THE WAYS THE INDEX AND THE WORKING TREE DISAGREE (#501 R92). Each is
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
( cd "$CTL/catkill" && ln -s '.claude/skills/team/rule.md' entry \
  && _fgit add entry >/dev/null 2>&1 \
  && command rm -f entry && ln -s 'harmless/target' entry ) || _fixture_failed catkill
# A TRACKED file under a directory with read but NOT search permission, its
# worktree copy made violating, and no untracked sibling beside it. The
# geometry is the plan memo's (§11.2), chosen to discriminate: with an
# untracked sibling, or with the violation in the index blob, another arm reds
# the run and `_absent` is not what is under test.
( cd "$CTL/d2red" && _fgit init -q . >/dev/null 2>&1 && mkdir sub \
  && printf '# %s\n' "$CONTROL_CLEAN" > sub/a.py && printf '# %s\n' "$CONTROL_CLEAN" > ok.py \
  && _fgit add -A >/dev/null 2>&1 \
  && printf 'RULE = "%s"\n' "$CONTROL_K2" > sub/a.py && chmod 0444 sub ) || _fixture_failed d2red
# …and its green partners: a tracked directory deleted wholesale (every
# ancestor below the root is missing), and one replaced by a regular FILE (the
# nearest existing ancestor is not a directory).
( cd "$CTL/d2green" && _fgit init -q . >/dev/null 2>&1 && mkdir -p dir/deep \
  && printf '# %s\n' "$CONTROL_CLEAN" > dir/deep/a.py && printf '# %s\n' "$CONTROL_CLEAN" > ok.py \
  && _fgit add -A >/dev/null 2>&1 && command rm -rf dir ) || _fixture_failed d2green
( cd "$CTL/d2file" && _fgit init -q . >/dev/null 2>&1 && mkdir dir \
  && printf '# %s\n' "$CONTROL_CLEAN" > dir/a.py && printf '# %s\n' "$CONTROL_CLEAN" > ok.py \
  && _fgit add -A >/dev/null 2>&1 && command rm -rf dir \
  && printf '# %s\n' "$CONTROL_CLEAN" > dir ) || _fixture_failed d2file
# A `--selftest` root that IS a directory but cannot be resolved to a physical
# path.
chmod 000 "$CTL/d5root" || _fixture_failed d5root
# …the orphan-branch fixture: commit on one branch, then check out an orphan.
( cd "$CTL/orphan" && _fgit init -q . >/dev/null 2>&1 \
  && printf 'x\n' > seed.txt && _fgit add seed.txt >/dev/null 2>&1 \
  && _fgit -c user.name=w -c user.email=w@e commit -q -m c >/dev/null 2>&1 \
  && _fgit checkout -q --orphan fresh >/dev/null 2>&1 \
  && command rm -f seed.txt && _fgit add -A >/dev/null 2>&1 ) || _fixture_failed orphan
# …and the ancestor-symlink fixture: commit `dir/a.py` clean, then replace `dir`
# with a link to a directory outside the tree whose `a.py` is NOT clean.
( cd "$CTL/ancestorlink" && _fgit init -q . >/dev/null 2>&1 \
  && mkdir -p dir && printf '# %s\n' "$CONTROL_CLEAN" > dir/a.py \
  && _fgit add -A >/dev/null 2>&1 \
  && _fgit -c user.name=w -c user.email=w@e commit -q -m c >/dev/null 2>&1 \
  && command rm -rf dir && ln -s "$CTL/external" dir ) || _fixture_failed ancestorlink
# ⚠ THE HEAD-PROBE FIXTURE NEEDS A COMMIT, and did not have one until the rule
# it encodes was corrected. R2 asserted "a failed probe is an error"; R3 showed
# the property is really "exit 1 is not by itself absence", so the fixture must
# be a repository that DOES have a commit — otherwise the probe failing and the
# repository being unborn are the same situation and the control cannot tell
# the two apart. A control written against a rule outlives the rule.
( cd "$CTL/headprobe" && _fgit add -A >/dev/null 2>&1 \
  && _fgit -c user.name=w -c user.email=w@e commit -q -m c >/dev/null 2>&1 ) || _fixture_failed headprobe
# …and the malformed-ref fixture: commit, then overwrite the branch ref with
# data git cannot resolve. ⚠ Built after the add loop, because it needs a HEAD
# to break.
( cd "$CTL/badref" && _fgit add -A >/dev/null 2>&1 \
  && _fgit -c user.name=w -c user.email=w@e commit -q -m c >/dev/null 2>&1 \
  && _br="$(_fgit symbolic-ref --short HEAD)" \
  && printf 'deadbeef\n' > ".git/refs/heads/$_br" ) || _fixture_failed badref
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
_control "$CTL/lsfail" 1 "the tracked inventory exited" "a failed TRACKED inventory fails closed" "" "" "$CTL/fakegit" || ctl_ok=1
_control "$CTL/wtlsfail" 1 "the worktree inventory exited" "a failed WORKTREE inventory fails closed" "" "" "$CTL/fakegitwt" || ctl_ok=1
# Not a `_control`: the question is what the run leaves BEHIND, which its exit
# status and output cannot say. Run from a directory of its own, so a leaked
# relative scratch lands where this block can see it.
_rel_lbl="a relative scratch dir is removed on exit"
if ( cd "$CTL/relcwd" && PATH="$CTL/fakerelmktemp:$PATH" "$SELF" --selftest "$CTL/clean" "" "" ) >/dev/null 2>&1; then
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
_control "$CTL/catfail" 1 "staged symlink blob could not be read" "a failed staged-blob read is not a clean target" "" "" "$CTL/catfail" || ctl_ok=1
_control "$CTL/phantom" 1 "the inventory listed it but it is gone" "an inventoried path that vanished is not silently skipped" "" "" "$CTL/fakegitphantom" || ctl_ok=1
_control "$CTL/globspec" 1 "the inventory listed it but it is gone" "the vanished-path question is asked of a LITERAL path" "" "" "$CTL/fakegitglob" || ctl_ok=1
_control "$CTL/badref" 1 "does not name a branch" "a malformed HEAD ref is not an unborn repository" || ctl_ok=1
_control "$CTL/punctslash" 1 "K2: a" "punctuation BEFORE a slash is part of the path" || ctl_ok=1
_control "$CTL/atclaude" 0 "PASSED" "a PATH character before .claude is not a prose boundary" || ctl_ok=1
_control "$CTL/linestart" 1 "K2: a" "a reference at the start of a line fires" || ctl_ok=1
_control "$CTL/textgreen" 0 "PASSED" "running text that only looks like a two-segment reference stays green" || ctl_ok=1
_control "$CTL/slashname" 1 "entry NAME" "a stored path naming .claude after a slash, under tools, fires" || ctl_ok=1
_control "$CTL/pathgreen" 0 "PASSED" "a stored path that only looks like one stays green" || ctl_ok=1
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
# does, for this control and every other.
_control "$CTL/empty" 2 "read 0 stored objects" "an empty scope fails loudly" || ctl_ok=1
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
# does not honour it (plan memo §11.1; not measured here), and there this
# control passes whatever the wire does.
_ctl_env=("GREP_OPTIONS=--exclude=*")
_control "$CTL/k2" 1 "K2: a" "a caller's GREP_OPTIONS cannot hide a file" || ctl_ok=1
# A `core.fsmonitor` hook injected through the caller's `GIT_CONFIG*`, which the
# wire keeps. Not a `_control`: the question is whether a command RAN, which the
# verdict cannot say. ⚠ The hook is first shown to run under a plain git call
# over the same fixture, or its not running under the wire would prove nothing.
# The hook's path goes through `_shq`, as every embedded path here does.
_fsm_lbl="a caller's fsmonitor hook does not run"
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
  env "${_fsm_cfg[@]}" "$SELF" --selftest "$CTL/fsmon" "" "" > "$CTL/.fsm_out" 2>&1 || _fsm_rc=$?
  if [ "$_fsm_rc" -ne 0 ] || [ -e "$_fsm_mark" ]; then
    echo "!! CONTROL FAILED ($_fsm_lbl): exit $_fsm_rc; the hook ran: $([ -e "$_fsm_mark" ] && echo yes || echo no)" >&2
    sed 's/^/     /' "$CTL/.fsm_out" >&2
    ctl_ok=1
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
  ( umask 777; "$SELF" --selftest "$CTL/clean" "" "" ) > "$CTL/.umask_out" 2>&1 || _um_rc=$?
  if [ "$_um_rc" -ne 2 ] || ! grep -q "could not resolve the scratch dir" "$CTL/.umask_out"; then
    echo "!! CONTROL FAILED ($_umask_lbl): expected exit 2 naming the scratch dir, got $_um_rc" >&2
    sed 's/^/     /' "$CTL/.umask_out" >&2
    ctl_ok=1
  fi
fi
# ---- THE MUTATION SET, WHICH LIVES BESIDE THIS FILE -------------------------
# Two subjects, two files: this one proves the wire can reach every verdict;
# that one proves each control is about the arm it names. Its absence is not a
# skipped check — the correspondence between the two lists is what keeps either
# list honest, so the run ends at "decided nothing" rather than without it.
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

[ "$ctl_ok" -eq 0 ] || exit 1

_mut_run

echo "  controls: green reachable; K2 fires under both roots, on the removed path,"
echo "            inside binary content, on a symlink's stored target, on an entry's"
echo "            own NAME, and on a symlinked entry script beside the scope; an empty"
echo "            scope fails closed"
printf '%s\n' "$_fifo_line"
printf '%s\n' "$_perm_line"
echo "            (each asserted on this script's own exit status, over a fixture tree)"
