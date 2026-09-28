#!/usr/bin/env bash
# THE FIXTURE BUILD for `webref-generic-core-trip-wire.sh` — sourced INSIDE the
# fixture build window, after the window's prelude, never run on its own. It
# builds the fixture trees under `$CTL` that the controls file then re-invokes
# the wire over.
#
# WHY IT IS A SEPARATE FILE. The controls file stood at 989 lines and this slice
# grows both of its halves, and the seam was already there: this half BUILDS the
# trees, that half ASSERTS over them (CLAUDE.md touch-time split;
# docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md §8.1).
# ⚠ IT RUNS IN THE FIXTURE BUILD WINDOW, NOT IN THE WIRE'S SHELL: the harness
# sources it in a child started with `env -i` and an allowlist
# (docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md §3), so the git it
# calls is plain `git` and inherits nothing the caller carries. What it can read
# is what the window's prelude hands it: `$CTL`, `$_FW_DIR` (for its last line
# and for `_seal`), the five `CONTROL_*` samples, `$_REAL_GIT`, `$_REAL_GREP`,
# `$_fifo_ok`, `_fixture_failed`, `_shq` and `_seal`. The controls file refuses
# to run without it, as it refuses without the harness.
# ⚠ A MODE RESTRICTION GOES THROUGH `_seal`, NEVER A BARE `chmod`: `_seal`
# records it and the window applies it after the postconditions, so the P-g
# census reads the whole tree first (a restriction applied here would make the
# census fail).
# ⚠ ITS LAST LINE IS LOAD-BEARING: it writes the `built` marker, so a window in
# which this file returned or exited early is reported as incomplete (W) and
# no control runs.
# THE ENTRY GUARD, as the other parts have: run directly — not sourced, or
# without the window's `$CTL` and `$_FW_DIR` — this file would build under `/`.
if [ "${BASH_SOURCE[0]:-$0}" = "$0" ] || [ -z "${CTL:-}" ] || [ -z "${_FW_DIR:-}" ]; then
  echo "!! This file is the FIXTURE BUILD for \`webref-generic-core-trip-wire.sh\`. It is" >&2
  echo "   SOURCED inside that wire's fixture build window and has no meaning alone." >&2
  echo "   Run the wire instead." >&2
  exit 2
fi
for d in clean pin k2 tools binary err empty walk link odd nl seg cache cachedir extra name emptyname quotename nlname rawbyte forge linkname ignored lsfail lstreefail grepfail grepfaillink nltarget linkslash staged fifotracked notcommitted inscope stagedlink nulblob committed replaced routed routeddecoy cfgkept punct suffixpath headprobe catfail phantom punctslash badref globspec orphan atclaude ancestorlink external bnd wtlsfail catkill d2red d2green d2file d3f1 d3f2 d3f3 d3f4 d3m1 d3m2 d3m3 d3m4 d3nb d5root fsmon linestart textgreen slashname pathgreen slashtext finalone interone midclass pathfirstone headlink unread; do mkdir -p "$CTL/$d"; done
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
# and now requires exit 0 with the sibling still read. The reason is the
# wording of 2026-07-citation-hygiene-Ai-spec-label-map.md §2, not convenience —
# K2 is about a path this tree *names*, i.e. stored
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
# Segments 2026-07-citation-hygiene-Ai-spec-label-map.md §2 admits that a
# `[A-Za-z0-9_.-]` class did not.
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
#      …and the rest of that exclusion set, one line per member group. ⚠ `A-Z`
#      was the member group with no line here: removing it from the class made
#      an upper-case letter a prose boundary, and every control stayed green.
{ for _c in '~' '+' '%' '-' '.' '_' '0' 'o' 'O'; do
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

# THE BOUNDARY RULES' OTHER DIRECTIONS (2026-09-citation-hygiene-Ai-wire-k2-trip-wire.md §11, D10).
# Each was shown
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
    '.claude/skills//rule.md' 'see .claude/tools/foo/ and/or more' '.claude/tools/foo//' \
    '.claude/tools/foo//bar/x'
  for _c in '}' '>' ',' ';' '"' "'" '`' ']'; do
    printf '(%s.claude/tools/foo/%s)\n' "$_c" "$_c"
  done; } > "$CTL/textgreen/ok.md"
#      ⚠ THE `)` IS WHY THOSE LOOP LINES PIN ONLY THE LAST CHARACTER. The final
#      segment is `<middle>*<last>`, and a line ending `/<c>)` cannot match
#      however the MIDDLE class is widened, because the `)` after `<c>` is
#      still not a legal LAST character. Dropping `]`, `/`, `"`, `'` or a
#      backtick from the middle class therefore survived every one of them —
#      see `midclass` below, whose lines end in an ordinary character instead.
#      The `//bar/x` line above is the other half of the same blind spot, one
#      element earlier: an intermediate segment is `<seg>/`, and nothing here
#      held a `/` where that `<seg>` starts, so letting `<seg>` span `/` was
#      invisible too. Neither was found by reading the list; both came out of
#      the generator that derives the population from `$K2RE` itself.
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
#      …and the two shapes that pin `[^/]` on BOTH of `$K2RE_PATH`'s segments.
#      Widening either to "any character" lets the segment swallow a `/`, and
#      each of these is green only because it does not: an EMPTY FIRST segment
#      (`//a/b`, the first `[^/]+`) and an EMPTY SECOND one (`a//b`, the
#      second). Nothing here held either shape, so both widenings survived the
#      whole control set. Both are symlink TARGETS because a filesystem
#      collapses `//` in a name, and a stored target is kept verbatim.
ln -s '.claude/skills//a/b' "$CTL/pathgreen/emptyfirst"
ln -s '.claude/skills/a//b' "$CTL/pathgreen/emptylast"
#  (j) A reference written after a `/` — the OTHER half of the leading boundary,
#      in running text. `$K2RE_PATH` spells the same rule as `(^|/)` and
#      `slashname` pins it there; nothing pinned it here, so adding `/` to the
#      exclusion class survived the whole control set. Alone in its own fixture:
#      `bnd`'s other spellings would keep that fixture red and prove nothing
#      about this one.
printf 'see elidex/%s here\n' "$CONTROL_K2" > "$CTL/slashtext/control.md"
#  (k) A FINAL SEGMENT OF ONE CHARACTER. The final segment is `[…]*[…]`, so its
#      minimum length is one; raising that to two survived the whole control set
#      because every red fixture's last segment was longer. Alone in its own
#      fixture, for the reason (j) gives.
printf 'X = %s\n' '.claude/tools/a/b' > "$CTL/finalone/control.py"
#  (l) A ONE-CHARACTER INTERMEDIATE SEGMENT. `finalone` above pins the FINAL
#      segment's minimum length; the OTHER reading of the same group —
#      `<seg>/`, taken when the final one cannot start here — has a minimum
#      length of its own, and raising it to two survived the whole control set
#      — measured; the reds that reach this arm at all (`d3m1`…`d3m4`) name a
#      longer segment there. The `]`
#      matters: it is what stops the final-segment reading, so this line can
#      only match through the `<seg>/` arm. Alone in its own fixture, for the
#      reason (j) gives.
printf 'X = %s\n' '.claude/tools/xx/]/y' > "$CTL/interone/control.py"
#  (m) THE FINAL SEGMENT'S MIDDLE CLASS, one line per member. The final segment
#      is `<middle>*<last>`, two classes that differ only in the closing
#      punctuation `<last>` also refuses — so `textgreen`'s lines, which end in
#      `)`, discriminate `<last>` and nothing else. These end in an ORDINARY
#      character, so each is green only because its leading `<c>` is in neither
#      class, and dropping `<c>` from the middle reds exactly this line.
#      Green-direction, so the members share one fixture: any line that fires
#      reds it, which is the opposite of (j)'s red-direction case, where a
#      second line would keep the fixture red and prove nothing.
{ for _mc in ']' '/' ' ' '"' "'" '`'; do
    printf 'X = .claude/tools/foo/%sx\n' "$_mc"
  done; } > "$CTL/midclass/ok.md"
#  (n) A ONE-CHARACTER FIRST SEGMENT IN A STORED PATH. `$K2RE_PATH`'s first
#      `[^/]+` has a minimum length of one, and raising it to two survived the
#      whole control set — measured. ⚠ A fixture here DOES name a
#      one-character first segment (`linkslash`, target `.claude/skills/a/`),
#      and it cannot see this: it is green for a different reason, its final
#      segment being empty. A required gate then reads `.claude/skills/a/rule.md` — an
#      entry whose own NAME is the forbidden hierarchy — as K2 zero, and says
#      so in the line it prints. Alone in its own fixture, for the reason (j)
#      gives; `ok.py` is clean and keeps the run off the zero-read guard.
mkdir -p "$CTL/pathfirstone/.claude/skills/a"
printf '# %s\n' "$CONTROL_CLEAN" > "$CTL/pathfirstone/.claude/skills/a/rule.md"
printf '# %s\n' "$CONTROL_CLEAN" > "$CTL/pathfirstone/ok.py"

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
         slashname pathgreen slashtext finalone interone midclass pathfirstone headlink unread; do
  ( cd "$CTL/$d" 2>/dev/null && git init -q . >/dev/null 2>&1 \
    && git add -A >/dev/null 2>&1 ) || _fixture_failed "$d"
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
( cd "$CTL/cachedir" && git init -q . >/dev/null 2>&1 \
  && printf '__pycache__/\n' > .gitignore \
  && git add -A >/dev/null 2>&1 \
  && git add -f __pycache__/probe.txt >/dev/null 2>&1 \
  && printf 'RULE = "%s"\n' "$CONTROL_K2" > __pycache__/probe.txt ) || _fixture_failed cachedir
# THE WAYS THE INDEX AND THE WORKING TREE DISAGREE (#501 R92). Each is
# built AFTER the add loop above, because each needs the index to hold one
# thing while the worktree holds another.
# (1) A violation STAGED and reverted in the worktree. `git show :victim.py`
#     still carries it, so a pre-push run that reads only the worktree
#     certifies the very commit that pushes it.
( cd "$CTL/staged" && printf 'X = "%s"\n' "$CONTROL_K2" > victim.py \
  && git add victim.py >/dev/null 2>&1 && printf '# %s\n' "$CONTROL_CLEAN" > victim.py ) || _fixture_failed staged
# (2) A TRACKED path replaced by a FIFO. `--cached` still lists it, and
#     opening it blocks forever with no writer — the local gate hangs instead
#     of failing closed. Nothing here may open it.
[ "$_fifo_ok" -eq 0 ] || \
( cd "$CTL/fifotracked" && printf '# %s\n' "$CONTROL_CLEAN" > sub.py \
  && git add sub.py >/dev/null 2>&1 && command rm -f sub.py && mkfifo sub.py ) || _fixture_failed fifotracked
# (2b) A STAGED SYMLINK whose target holds a SPACE inside a segment, with the
#      worktree target since made clean. The index mode says it is a symlink,
#      so its blob is a stored path; sent through the running-text predicate
#      the space terminated the match and the wire read GREEN (#501 R93).
( cd "$CTL/stagedlink" && ln -s '.claude/skills/team name/rule.md' entry \
  && git add entry >/dev/null 2>&1 \
  && command rm -f entry && ln -s 'harmless/target' entry \
  && printf '# %s\n' "$CONTROL_CLEAN" > ok.py && git add ok.py >/dev/null 2>&1 ) || _fixture_failed stagedlink
# (2b') …and the same stored-target question asked of HEAD, which is the source
#      `stagedlink` cannot pose. The symlink is COMMITTED and then replaced, in
#      the index AND the worktree, by a clean regular file — so mode 120000
#      reaches the blob arm only from `ls-tree`, and an arm that asks for the
#      index before reading the blob as a path goes green over the commit a
#      push sends. ⚠ The target holds a SPACE for the reason (2b)'s does: it is
#      what separates the stored-path predicate from the running-text one, so a
#      mutant that routes this blob to `_content` cannot be caught by `$K2RE`
#      finding the same string anyway.
( cd "$CTL/headlink" && ln -s '.claude/skills/team name/rule.md' entry \
  && git add entry >/dev/null 2>&1 \
  && git -c user.name=w -c user.email=w@e commit -q -m c >/dev/null 2>&1 \
  && command rm -f entry && printf '# %s\n' "$CONTROL_CLEAN" > entry \
  && git add entry >/dev/null 2>&1 ) || _fixture_failed headlink
# (2c) A mode-120000 index entry whose BLOB HOLDS A NUL. git will store and
#      commit it; no filesystem can realise it as a symlink. It must red the
#      gate as unreadable, not be quietly shortened into something clean.
( cd "$CTL/nulblob" \
  && printf '.claude/skills/\000/rule.md' > raw.bin \
  && _sha="$(git hash-object -w --stdin < raw.bin)" \
  && command rm -f raw.bin \
  && git update-index --add --cacheinfo "120000,$_sha,entry" >/dev/null 2>&1 \
  && printf '# %s\n' "$CONTROL_CLEAN" > ok.py && git add ok.py >/dev/null 2>&1 ) || _fixture_failed nulblob
# (2c') A SCOPE WHOSE EVERY ENTRY GOES UNREAD, which is the state that tells
#      `SCANNED` apart from the size of the inventory. The one entry's index
#      blob is a sha that was COMPUTED BUT NEVER WRITTEN, so `cat-file` fails;
#      it has no worktree copy, so the worktree pass answers for it with no
#      record either. Both sources therefore leave `_read` at 0 and the run
#      must stop at "read 0" — whereas an `ok` printed for every entry
#      regardless makes `SCANNED` the inventory's LENGTH, and the zero-read
#      guard stops guarding anything.
#      ⚠ `empty` cannot pose this: with no entries at all `SCANNED` is 0 either
#      way. The entries have to EXIST and go unread.
( cd "$CTL/unread" \
  && _sha="$(printf 'computed, never written\n' | git hash-object --stdin)" \
  && git update-index --add --cacheinfo "100644,$_sha,victim.py" >/dev/null 2>&1 ) || _fixture_failed unread
# The `ls-tree` shim only reaches its arm if the fixture HAS a HEAD — an unborn
# HEAD skips the whole block, which would make the control green over a branch
# it never took.
( cd "$CTL/lstreefail" && git add -A >/dev/null 2>&1 \
  && git -c user.name=w -c user.email=w@e commit -q -m c >/dev/null 2>&1 ) || _fixture_failed lstreefail
# …and the same staged-symlink geometry for the `cat`-failure control: the blob
# is what `cat` reads, so the fixture must HAVE one.
( cd "$CTL/catfail" && ln -s '.claude/skills/team/rule.md' entry \
  && git add entry >/dev/null 2>&1 \
  && command rm -f entry && ln -s 'harmless/target' entry ) || _fixture_failed catfail
( cd "$CTL/catkill" && ln -s '.claude/skills/team/rule.md' entry \
  && git add entry >/dev/null 2>&1 \
  && command rm -f entry && ln -s 'harmless/target' entry ) || _fixture_failed catkill
# A TRACKED file under a directory with read but NOT search permission, its
# worktree copy made violating, and no untracked sibling beside it. The
# geometry is 2026-09-citation-hygiene-Ai-wire-k2-trip-wire.md §11.2's, chosen
# to discriminate: with an
# untracked sibling, or with the violation in the index blob, another arm reds
# the run and `_absent` is not what is under test.
( cd "$CTL/d2red" && git init -q . >/dev/null 2>&1 && mkdir sub \
  && printf '# %s\n' "$CONTROL_CLEAN" > sub/a.py && printf '# %s\n' "$CONTROL_CLEAN" > ok.py \
  && git add -A >/dev/null 2>&1 \
  && printf 'RULE = "%s"\n' "$CONTROL_K2" > sub/a.py ) || _fixture_failed d2red
_seal "$CTL/d2red/sub" 0444 d2red
# …and its green partners: a tracked directory deleted wholesale (every
# ancestor below the root is missing), and one replaced by a regular FILE (the
# nearest existing ancestor is not a directory).
( cd "$CTL/d2green" && git init -q . >/dev/null 2>&1 && mkdir -p dir/deep \
  && printf '# %s\n' "$CONTROL_CLEAN" > dir/deep/a.py && printf '# %s\n' "$CONTROL_CLEAN" > ok.py \
  && git add -A >/dev/null 2>&1 && command rm -rf dir ) || _fixture_failed d2green
( cd "$CTL/d2file" && git init -q . >/dev/null 2>&1 && mkdir dir \
  && printf '# %s\n' "$CONTROL_CLEAN" > dir/a.py && printf '# %s\n' "$CONTROL_CLEAN" > ok.py \
  && git add -A >/dev/null 2>&1 && command rm -rf dir \
  && printf '# %s\n' "$CONTROL_CLEAN" > dir ) || _fixture_failed d2file
# A `--selftest` root that IS a directory but cannot be resolved to a physical
# path.
_seal "$CTL/d5root" 000 d5root
# …the orphan-branch fixture: commit on one branch, then check out an orphan.
( cd "$CTL/orphan" && git init -q . >/dev/null 2>&1 \
  && printf 'x\n' > seed.txt && git add seed.txt >/dev/null 2>&1 \
  && git -c user.name=w -c user.email=w@e commit -q -m c >/dev/null 2>&1 \
  && git checkout -q --orphan fresh >/dev/null 2>&1 \
  && command rm -f seed.txt && git add -A >/dev/null 2>&1 ) || _fixture_failed orphan
# …and the ancestor-symlink fixture: commit `dir/a.py` clean, then replace `dir`
# with a link to a directory outside the tree whose `a.py` is NOT clean.
( cd "$CTL/ancestorlink" && git init -q . >/dev/null 2>&1 \
  && mkdir -p dir && printf '# %s\n' "$CONTROL_CLEAN" > dir/a.py \
  && git add -A >/dev/null 2>&1 \
  && git -c user.name=w -c user.email=w@e commit -q -m c >/dev/null 2>&1 \
  && command rm -rf dir && ln -s "$CTL/external" dir ) || _fixture_failed ancestorlink
# ⚠ THE HEAD-PROBE FIXTURE NEEDS A COMMIT, and did not have one until the rule
# it encodes was corrected. R2 asserted "a failed probe is an error"; R3 showed
# the property is really "exit 1 is not by itself absence", so the fixture must
# be a repository that DOES have a commit — otherwise the probe failing and the
# repository being unborn are the same situation and the control cannot tell
# the two apart. A control written against a rule outlives the rule.
( cd "$CTL/headprobe" && git add -A >/dev/null 2>&1 \
  && git -c user.name=w -c user.email=w@e commit -q -m c >/dev/null 2>&1 ) || _fixture_failed headprobe
# …and the malformed-ref fixture: commit, then overwrite the branch ref with
# data git cannot resolve. ⚠ Built after the add loop, because it needs a HEAD
# to break.
( cd "$CTL/badref" && git add -A >/dev/null 2>&1 \
  && git -c user.name=w -c user.email=w@e commit -q -m c >/dev/null 2>&1 \
  && _br="$(git symbolic-ref --short HEAD)" \
  && printf 'deadbeef\n' > ".git/refs/heads/$_br" ) || _fixture_failed badref
# (2d) A violation COMMITTED and then fixed only in the index and worktree. A
#      push sends the commit, so a gate that reads the index alone calls this
#      clean while `git show HEAD:victim.py` still carries it (#501 R95).
( cd "$CTL/committed" && printf 'X = "%s"\n' "$CONTROL_K2" > victim.py \
  && git add victim.py >/dev/null 2>&1 \
  && git -c user.name=w -c user.email=w@e commit -q -m c >/dev/null 2>&1 \
  && printf '# %s\n' "$CONTROL_CLEAN" > victim.py \
  && git add victim.py >/dev/null 2>&1 ) || _fixture_failed committed
# (2e) A local `replace` ref pointing the staged blob at a clean one. What is
#      committed is the object the index NAMES, so that is what must be read.
( cd "$CTL/replaced" && printf 'X = "%s"\n' "$CONTROL_K2" > victim.py \
  && git add victim.py >/dev/null 2>&1 \
  && _bad="$(git rev-parse :0:victim.py)" \
  && printf '# %s\n' "$CONTROL_CLEAN" > victim.py \
  && _good="$(git hash-object -w victim.py)" \
  && git replace "$_bad" "$_good" >/dev/null 2>&1 ) || _fixture_failed replaced
# (2f) `GIT_DIR`/`GIT_WORK_TREE` exported at another checkout. `-C` does not
#      win over them, so the inventory described the decoy while the worktree
#      arm read files here — one clean entry, exit 0, over a violation (#501
#      R95). The decoy is a real repo holding nothing forbidden.
printf 'SRC = "%s"\n' "$CONTROL_K2"      > "$CTL/routed/probe.py"
printf '# %s\n' "$CONTROL_CLEAN"         > "$CTL/routeddecoy/ok.py"
( cd "$CTL/routed" && git add -A >/dev/null 2>&1 ) || _fixture_failed routed
( cd "$CTL/routeddecoy" && git add -A >/dev/null 2>&1 ) || _fixture_failed routeddecoy
# (2g) A checkout path holding a GLOB CHARACTER. Built outside the fixture
#      loop on purpose: the loop's word list would itself glob the name.
#      The violation sits BESIDE the scope, so a widened `REL_DIR` — the
#      defect — turns this green fixture red.
mkdir -p "$CTL/glob[1]/scope"
printf '# %s\n' "$CONTROL_CLEAN"         > "$CTL/glob[1]/scope/ok.py"
printf 'SRC = "%s"\n' "$CONTROL_K2"      > "$CTL/glob[1]/outside.py"
( cd "$CTL/glob[1]" && git init -q . >/dev/null 2>&1 && git add -A >/dev/null 2>&1 ) || _fixture_failed "glob[1]"
# (2g') …and the SECOND site that strips `$ROOT` from a scope path: the EXTRA
#      ENTRY's. `glob[1]` passes no extra entry, so nothing asked this half —
#      and `[1]` is the wrong shape for it anyway: the strip then fails
#      outright and leaves an ABSOLUTE pathspec, which git resolves to the same
#      entry, so the mutant survives. Measured on the pre-fix wire, with an
#      extra entry added to `glob[1]`'s own geometry: exit 0, PASSED.
#      ⚠ THE NAME IS THE FIXTURE. What discriminates is a pattern that matches
#      a LONGER prefix than the literal it came from: `*[e]` cannot match
#      `*[e]`, but `*` can absorb up to `sid` and `[e]` take the `e` of
#      `side/` — so the unquoted strip removes `…/side/` and hands git `entry`,
#      a path that does not exist. The entry is then absent from the inventory
#      and its violation is never read: silently green, which is why the
#      fixture's violation lives THERE and not beside it.
mkdir -p "$CTL/globextra*[e]/scope" "$CTL/globextra*[e]/side"
printf '# %s\n' "$CONTROL_CLEAN"         > "$CTL/globextra*[e]/scope/ok.py"
printf 'SRC = "%s"\n' "$CONTROL_K2"      > "$CTL/globextra*[e]/side/entry"
( cd "$CTL/globextra*[e]" && git init -q . >/dev/null 2>&1 && git add -A >/dev/null 2>&1 ) || _fixture_failed "globextra*[e]"
# (3) An untracked violation hidden by `$GIT_DIR/info/exclude` — per-clone,
#     uncommitted state that `--exclude-standard` honours and no other clone
#     of the same commit shares. (The machine-wide `core.excludesFile` is the
#     same code path; it is reproduced in #501 R92's thread rather than given
#     a fixture, because neutralising the config layer above would also
#     neutralise the fixture that tried to set it.)
( cd "$CTL/notcommitted" && printf '# %s\n' "$CONTROL_CLEAN" > ok.py \
  && git add ok.py >/dev/null 2>&1 \
  && printf 'SRC = "%s"\n' "$CONTROL_K2" > probe.txt \
  && mkdir -p .git/info && printf 'probe.txt\n' > .git/info/exclude ) || _fixture_failed notcommitted

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
_seal "$CTL/err/control.py" 000 err
_seal "$CTL/walk/sub" 000 walk
# THE LAST LINE: the window certifies that THIS FILE ran to its end, not merely
# that the child shell did (a top-level `return` would otherwise complete it).
# The fixtures' own `notcommitted` needs `mkdir -p .git/info` above because the
# window's template directory is empty, so `git init` no longer creates it.
: > "$_FW_DIR/built"
