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
# (`$SELF`, `$SCRATCH`, …), which is what this file consumes. It sets
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
# The window runs in `$_FW_DIR`, not here, so what a `PATH` entry that is not
# absolute (`bin`, `../x`, `.`, an empty one, `~`, `~login/…`) means — which
# files it names, or none — is decided THERE, by the window's own bash, and is
# outside P (docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md §1,
# "Outside P"): which such a command runs is R1's, not this file's. So the
# window does NOT emulate that lookup here (PR #527 Codex R11–R13: an
# emulation grew one case per round; draft 13 dropped it for the caller's
# `PATH`, verbatim, behind the pin). Only `git` is resolved HERE:
# - `git` is resolved by THIS shell's own lookup, and pinned in `$_FGIT_BIN`
#   as an `exec <absolute path>` wrapper, so the window, its fixtures, P-e's
#   reference and the wire itself (`$_REAL_GIT`, which the fixtures' shims
#   exec) all run the `git` this shell would run — which `git` runs is still
#   `PATH`'s (docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md §0.1,
#   R1); every command that crosses the window boundary by path is resolved by
#   `_fgit_resolve`, and by nothing else;
# - `env` is resolved the same way and used by its absolute path (window
#   launch, P-e's reference, P-f);
# - the window's `PATH` is `$_FGIT_BIN` followed by the CALLER'S `PATH`,
#   VERBATIM: no entry is made absolute, dropped or reinterpreted here.
#   `git` itself is pinned above, so what such an entry means from inside the
#   window — relative, empty, `~`, `~login/…` — decides only which OTHER tool
#   is found there, never which `git` runs: a tool only such an entry provides
#   makes the window fail, red, never a different `git`. Which of the
#   fixtures' other commands runs is outside P
#   (docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md §1, "Outside P").
#   P-j checks the shape (`$_FGIT_BIN` first, the pinned `git`, and a `PATH` without it passed on) from inside.
_fgit_resolve() { # $1 = a command name → the absolute path of the file this shell runs for it, or ""
  _fr="$(type -P "$1")" || _fr=""
  case "$_fr" in /*|"") ;; *) _fr="$PWD/$_fr" ;; esac
  printf '%s' "$_fr"
}
_FGIT_BIN="$SCRATCH/fgit-bin"
mkdir "$_FGIT_BIN" || exit 2
# ONE resolution of `git`: the wrapper below and `$_REAL_GIT` (which the
# fixtures' shims exec) are both built from `$_FGIT_GIT`, so they cannot name
# two different files.
_FGIT_GIT="$(_fgit_resolve git)"
# ⚠ EVERY EXEC-THROUGH SHIM IS BUILT HERE, BY THIS ONE FUNCTION, and each one
# removes ITS OWN directory from the front of `PATH` before it execs. A `git`
# that hands over to "the next `git` on `PATH`" (strip-own-dir, `which -a`
# skipping itself, or a re-exec under `env -i PATH=…`) would otherwise find the
# shim again — the shim's directory is first on every `PATH` that reaches it —
# and the two exec each other forever, with no output and a core spinning. A
# counter cannot see that (the `env -i` form resets it); removing the way back
# can, in any idiom. `$1` = the shim's file (its directory is the one removed),
# `$2` = shell code run before the exec (may be empty), `$3` = what is exec'd,
# already quoted (`_shq`). Reaches the window through the prelude.
_fgit_shim() {
  printf '#!/bin/sh\nPATH=${PATH#%s:}\n%s\nexec %s "$@"\n' "$(_shq "${1%/*}")" "$2" "$3" > "$1" \
    && chmod +x "$1"
}
if [ -n "$_FGIT_GIT" ]; then
  # The pin. P-j checks, from inside the window, that the `PATH` a `git` it runs
  # passes on does not hold `$_FGIT_BIN`.
  _fgit_shim "$_FGIT_BIN/git" "" "$(_shq "$_FGIT_GIT")" || exit 2
fi
_FGIT_PATH="$_FGIT_BIN:$PATH"   # the caller's PATH, verbatim, behind the pin (docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md §0.1)
_FGIT_ENVBIN="$(_fgit_resolve env)"
_FGIT_BASH="$BASH"
# `GIT_DEFAULT_REF_FORMAT=files`: a git whose COMPILED-IN default is reftable
# (git 3.0's planned default, or a breaking-changes build) would otherwise make
# every init differ from every other (`reftable/*.ref` names are random) and
# `badref` write refs a reftable repo does not read. Gits before 2.45 have no
# reftable and ignore the name.
_FGIT_ENV=("PATH=$_FGIT_PATH" "HOME=$_FGIT_VOID" GIT_CONFIG_NOSYSTEM=1 GIT_ATTR_NOSYSTEM=1 "GIT_TEMPLATE_DIR=$_FGIT_VOID" GIT_DEFAULT_REF_FORMAT=files "LC_ALL=C")
# `_FGIT_ENV_REQ` is the allowlist's own names, derived from it: P-f requires
# each of them to be PRESENT in the window's environment, and allows them plus
# what bash itself maintains for the processes it starts (`PWD OLDPWD SHLVL _`).
_FGIT_ENV_REQ=""
for _fe in "${_FGIT_ENV[@]}"; do _FGIT_ENV_REQ="$_FGIT_ENV_REQ ${_fe%%=*}"; done
# P-e's reference: the exec path the SAME git reports with only `PATH` and
# `HOME=$_FGIT_VOID` in its environment, canonical (`pwd -P`). `HOME` is there
# because a caller's `git` wrapper may read `$HOME` under `set -u`: with `PATH`
# alone it aborts, the reference is empty, and P-e reds with no cause shown. No
# record pins `HOME` here: dropping it is a false red only on a machine with
# such a wrapper, and a record runs on the caller's. Which executable runs is outside P
# (R1: it is `PATH`'s); what P-e pins is that nothing the window carries
# overrides where that git runs its commands from. The window's side is taken
# the same way, so a caller's `GIT_EXEC_PATH` or `DEVELOPER_DIR`, or one
# directory spelled two ways, cannot make the two differ.
_fgit_canon() { [ -n "$1" ] && ( cd "$1" 2>/dev/null && pwd -P ) || printf ''; }
_FGIT_WIRE_EXEC="$("$_FGIT_ENVBIN" -i "PATH=$_FGIT_PATH" "HOME=$_FGIT_VOID" git --exec-path 2>/dev/null)" || _FGIT_WIRE_EXEC=""
_FGIT_WIRE_EXEC="$(_fgit_canon "$_FGIT_WIRE_EXEC")"
# P-h's reference: the locale the wire reads in (it exports `LC_ALL=C`), so the
# build is like-for-like with the scan and the window's diagnostics are the
# ones W3 is written against.
_FGIT_WIRE_LC="${LC_ALL:-}"
# State the parent reads back — assigned before anything reads it (nothing here
# relies on `set -u`; see docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-residuals.md §5.2).
_FW_DIR="$SCRATCH/fgit-window"
_fw_rc=0; _fw_done=0; _fw_post_ok=0; _fw_post_bad=0; _fw_why="the window was never started"; _fw_diag=""; _fw2_said=0
_fw_trusted=0
_fw_seal_bad=""; _fw_limits=""
# The postconditions, run INSIDE the window after the build. Their labels
# (`$_pa_lbl` … `$_pg_lbl`, defined in the controls file) reach the window by
# name through the prelude. Returns 1 if any reported.
_fgit_postconditions() {
  _fpv=0
  # EVERY file the postconditions write and then read back (the census lists,
  # the per-link search results, `env -0`'s output, P-g's reference and
  # current listings) lives, with the references, in ONE directory made now,
  # after the build, by `mktemp -d` beside the window's directory (never under
  # `$_FW_DIR` or `$CTL`), whose name no fixture command that finished during
  # the build could know (docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md §4,
  # "The references and the working files"). `$_FW_DIR` keeps only what
  # both sides name: the two sourced files, the seal manifest `seal`, and the
  # return channel (`built`, `done`, `cause`, `post_ok`, `post_bad`, `machine_limits`,
  # `seal_failed`, `fix_failed`, `stderr`).
  _pq="$(mktemp -d "${_FW_DIR%/*}/pgrefXXXXXX")" && [ -n "$_pq" ] && [ -d "$_pq" ] \
    || { echo "!! CONTROL FAILED ($_pg_lbl): the postconditions' directory could not be made fresh" >&2; return 1; }
  # `_fgit_in <repo dir> <git args…>`: git run AT one repository and nowhere
  # else. `--git-dir=.git` turns discovery off, so a `.git` git does not
  # recognise (a garbage `HEAD`) is an error here, never a walk to an ancestor
  # repository whose configuration would then be listed in its place.
  _fgit_in() { ( cd "$1" && shift && git --git-dir=.git "$@" ); }
  # THE NUL-LIST POLICY, for every `read -d ''` reader below (the census's two
  # passes, P-f's `env -0`, `_pg_z`): `read -d ''` swallows a list written with
  # `-print` (newlines, no NUL) in silence — nothing classified, nothing red —
  # but it leaves the unterminated remainder in the loop's variable. So after
  # each loop that variable must be empty, else the list did not end in a NUL
  # byte and is red under the reader's own label. No command is run to ask.
  # THE CENSUS, BEFORE ANY POSTCONDITION RUNS GIT: git reads across git dirs
  # (`commondir`, `alternates`, `include.path`), so both passes' verdict must
  # be in before git runs anywhere, and a red census returns here — no git
  # runs on any repo. It is ONE census: the P-g/P-k loop below iterates pass
  # 1's own list (`$_pq/pre`) and nothing else, so what it acts on is what pass
  # 1 classified. Pass 1: classification and shape, over every `.git` and
  # `HEAD` entry of any type and letter case, found by ONE `find`, with no
  # `-L` and no per-directory fork. "Cannot search" is whatever `find` reports:
  # any report fails the census — and a BSD `find` (macOS's `/usr/bin/find`)
  # reports NOTHING for a directory it cannot enter (measured: exit 0, no stderr,
  # the repository beneath it unlisted), so the same `find` also lists every
  # directory whose mode is not `u+rx` (`! -perm -u=rx`) and the loop reds it.
  # (A GNU `find` reports such a directory itself, on stderr and in its exit
  # status — read from its manual, not measured here.) NOTHING UNDER `$CTL` IS
  # UNSEARCHABLE AT CENSUS TIME, BY CONSTRUCTION: the fixtures ask for their mode restrictions through
  # `_seal`, which only records them, and the window applies them AFTER this
  # check — so one the census cannot read is a new, unrequested restriction,
  # red. Names are matched without case: a case-insensitive filesystem (APFS
  # by default) lets git read `head` and `.GIT`, so a case variant is a git
  # dir too — and red, being unknown.
  _pgpre=""
  find "$CTL" \( -iname .git -print0 \) -o \( -iname HEAD -print0 \) -o \( -type d ! -perm -u=rx -print0 \) > "$_pq/pre" 2> "$_pq/pre.err" || _pgpre=" (census failed)"
  while IFS= read -r -d '' _pe; do
    case "${_pe##*/}" in
      .git)
        if [ -L "$_pe" ] || [ ! -d "$_pe" ]; then _pgpre="$_pgpre ${_pe#"$CTL"/}:[.git is not a directory]"
        else
          # Every entry under a `.git` directory must be a directory or a
          # regular file with ONE link: a symlink of any target, a FIFO, a
          # socket, a device or a hard link is red — git reads through a link
          # and blocks on a FIFO, and a hard link shares content outside the
          # git dir.
          _psh="$(find "$_pe" \( \( -type f -links +1 \) -o \( ! -type f ! -type d \) \) -print 2>&1)" || _psh="$_psh (shape scan failed)"
          [ -z "$_psh" ] || _pgpre="$_pgpre ${_pe#"$CTL"/}:[shape: $(printf '%s' "$_psh" | tr '\n' ' ')]"
        fi ;;
      [Hh][Ee][Aa][Dd])
        _pd="${_pe%/*}"
        if [ "${_pd##*/}" = .git ]; then [ "${_pe##*/}" = HEAD ] || _pgpre="$_pgpre ${_pe#"$CTL"/}:[HEAD spelled]"
        elif [ -d "$_pd/objects" ] || [ -f "$_pd/commondir" ]; then _pgpre="$_pgpre ${_pd#"$CTL"/}:[a git dir not named .git]"; fi ;;
      [.][Gg][Ii][Tt]) _pgpre="$_pgpre ${_pe#"$CTL"/}:[.git spelled]" ;;
      *) _pgpre="$_pgpre ${_pe#"$CTL"/}:[a directory the census cannot search]" ;;
    esac
  done < "$_pq/pre"
  [ -z "$_pe" ] || _pgpre="$_pgpre (the census list does not end in a NUL byte)"
  # Pass 2, the per-link search, runs only over a CLEAN pass 1 and before any
  # git: emulating git's own read set would be the anti-pattern this window
  # removed for `PATH` (docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md §0.1),
  # so this follows every symlink under `$CTL` that resolves to a
  # directory, whatever its name, and reds it if a `HEAD` or `.git` is
  # anywhere beneath. A link inside a git dir pass 1 already classified is
  # never searched (it is already red by pass 1's shape). A link that does not
  # resolve to a directory is not searched. There is no time bound on the
  # per-link search: a link to a huge tree runs until CI's job timeout ends it,
  # red — a local run has no such timeout and waits (slot
  # `#11-trip-wire-liveness-bound`,
  # docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-landing.md §9.2).
  if [ -z "$_pgpre" ]; then
    find "$CTL" -type l -print0 > "$_pq/prel" 2>> "$_pq/pre.err" || _pgpre=" (link census failed)"
    while IFS= read -r -d '' _pl; do
      [ -d "$_pl" ] || continue
      _plrc=0; find -L "$_pl" \( -iname HEAD -o -iname .git \) -print > "$_pq/prels" 2>&1 || _plrc=$?
      if [ "$_plrc" -ne 0 ]; then _pgpre="$_pgpre ${_pl#"$CTL"/}:[the search through this symlink failed (exit $_plrc)]"
      elif [ -s "$_pq/prels" ]; then _pgpre="$_pgpre ${_pl#"$CTL"/}:[a symlink to a tree holding a git dir]"; fi
    done < "$_pq/prel"
    [ -z "$_pl" ] || _pgpre="$_pgpre (the link list does not end in a NUL byte)"
  fi
  # ONE check of the census's stderr, after BOTH passes (pass 2 appends to it).
  [ ! -s "$_pq/pre.err" ] || _pgpre="$_pgpre (census stderr)"
  _pg_census="$_pgpre"
  if [ -n "$_pg_census" ]; then echo "!! CONTROL FAILED ($_pg_lbl):$_pg_census" >&2; return 1; fi
  # The REFERENCES are made in the postconditions' own directory (above), the
  # probe repos by `mkdir` without `-p`, so one that already existed is a
  # failure (a stale directory reused), not a silent success.
  mkdir "$_pq/a" "$_pq/b" \
    || { echo "!! CONTROL FAILED ($_pg_lbl): the reference repos could not be made fresh" >&2; return 1; }
  ( cd "$_pq/a" && git init -q . ) >/dev/null 2>&1 || true
  ( cd "$_pq/b" && git init -q --template="$_FGIT_VOID" . ) >/dev/null 2>&1 || true
  # `--show-origin` only (git 2.8), not `--show-scope` (git 2.26; PR #527 Codex
  # R12): the origin column already says what P-a asks — `command line:` for
  # the live `-c`, `file:.git/config` for the repo's own file, and any other
  # file for a layer outside it — so no version floor, and no limitation arm.
  _pa_live="$(_fgit_in "$_pq/a" -c a.b=c config --list --show-origin 2>/dev/null | awk -F'\t' '$1=="command line:"{print "command"; exit}')" || _pa_live=""
  _pa_bad="$(_fgit_in "$_pq/a" config --list --show-origin 2>&1 | awk -F'\t' '$1!="file:.git/config"')" || _pa_bad="(config --list failed)"
  if [ "$_pa_live" != command ]; then echo "!! CONTROL NOT EXERCISED ($_pal_lbl)" >&2; _fpv=1
  elif [ -n "$_pa_bad" ]; then echo "!! CONTROL FAILED ($_pa_lbl): $(printf '%s' "$_pa_bad" | tr '\n' ' ')" >&2; _fpv=1; fi
  # P-b asks `git var` INSIDE P-a's probe repo, so what it reads is that repo's
  # plain configuration and never the caller's (the window runs in its own
  # directory, and a probe run from wherever it happened to be would read the
  # local configuration of any repository around it).
  # ⚠ A MACHINE LIMITATION, NOT A RED: `git var` names these four only from git
  # 2.42; an older git answers 129 (usage). That is the capability rule the
  # FIFO and file-permission controls follow: reported as NOT EXERCISED on
  # this machine, and green.
  _pbd=""; _pbl=""; _pbu=""
  for _v in GIT_CONFIG_SYSTEM GIT_ATTR_SYSTEM GIT_CONFIG_GLOBAL GIT_ATTR_GLOBAL; do
    _prc=0; _fgit_in "$_pq/a" var "$_v" >/dev/null 2>&1 || _prc=$?
    [ "$_prc" -ne 129 ] || _pbu="$_pbu $_v"
  done
  if [ -n "$_pbu" ]; then
    printf 'P-b — this git'"'"'s `git var` cannot name%s (git 2.42 or later can)\n' "$_pbu" >> "$_FW_DIR/machine_limits"
  else
    for _v in GIT_CONFIG_SYSTEM GIT_ATTR_SYSTEM; do
      _o="$(GIT_CONFIG_NOSYSTEM=0 GIT_ATTR_NOSYSTEM=0 _fgit_in "$_pq/a" var "$_v" 2>/dev/null)" || _o=""
      [ -n "$_o" ] || _pbl="$_pbl $_v"
      if _o="$(_fgit_in "$_pq/a" var "$_v" 2>/dev/null)"; then _pbd="$_pbd $_v=[$_o]"; fi
    done
    for _v in GIT_CONFIG_GLOBAL GIT_ATTR_GLOBAL; do
      if ! _o="$(_fgit_in "$_pq/a" var "$_v" 2>/dev/null)"; then _pbd="$_pbd $_v:rc!=0"; continue; fi
      # The void is replaced by a token BEFORE the output is split into lines, so
      # a scratch path holding a newline cannot split one entry into two.
      _o="${_o//"$_FGIT_VOID"/@K2VOID@}"
      while IFS= read -r _l; do case "$_l" in @K2VOID@/*) : ;; *) _pbd="$_pbd $_v=[$_l]";; esac; done <<EOF_PB
$_o
EOF_PB
    done
    if [ -n "$_pbl" ]; then echo "!! CONTROL NOT EXERCISED ($_pbl_lbl):$_pbl" >&2; _fpv=1
    elif [ -n "$_pbd" ]; then echo "!! CONTROL FAILED ($_pb_lbl):$_pbd" >&2; _fpv=1; fi
  fi
  if ! diff -r "$_pq/a/.git" "$_pq/b/.git" >/dev/null 2>&1; then echo "!! CONTROL FAILED ($_pd_lbl)" >&2; _fpv=1; fi
  _o="$(git --exec-path 2>/dev/null)" || _o=""
  _o="$(_fgit_canon "$_o")"
  if [ -z "$_o" ] || [ "$_o" != "$_FGIT_WIRE_EXEC" ]; then echo "!! CONTROL FAILED ($_pe_lbl): [$_o] vs [$_FGIT_WIRE_EXEC]" >&2; _fpv=1; fi
  # P-f: every record of `env -0`, through a FILE so its status is checked; a
  # failed `env -0`, or an empty population, is NOT EXERCISED and red. ONE
  # outcome is a MACHINE LIMITATION instead (`⚠ NOT EXERCISED on this machine`,
  # green — as for the FIFO and file-permission controls and P-b), and its test
  # is the definition itself: THIS `env` runs, but refuses `-0` — `-0` fails,
  # writes nothing to stdout and says why on stderr, while the same `env` runs a
  # command. Every other outcome stays red, so an unknown one falls on the
  # fail-safe side. Both calls go through `$_pfenv`, so one record can make the
  # `env` unrunnable for both. (PR #527: the R6 probe sent "anything but one
  # exact record" to green and shared the `-i` record's anchor; the next form
  # read an `env` that could not be started at all as "no -0".)
  # No line-based fallback: a value is never split on newlines (Codex R6).
  _pf=""; _pfn=0; _pfrc=0; _pfseen=" "; _pfenv="$_FGIT_ENVBIN"
  "$_pfenv" -0 > "$_pq/env0" 2> "$_pq/env0.err" || _pfrc=$?
  if [ "$_pfrc" -ne 0 ] && [ ! -s "$_pq/env0" ] && [ -s "$_pq/env0.err" ] \
     && "$_pfenv" "$BASH" -c : > /dev/null 2>&1; then
    printf 'P-f — this `env` refuses -0 (exit %s, nothing on stdout) but runs a command\n' "$_pfrc" >> "$_FW_DIR/machine_limits"
  else
    # The NUL-list policy (above, one for every `read -d ''` here) applies.
    while IFS= read -r -d '' _rec; do
      _pfn=$((_pfn + 1)); _n="${_rec%%=*}"; _pfseen="$_pfseen$_n "
      case " PWD OLDPWD SHLVL _ $_FGIT_ENV_REQ " in *" $_n "*) : ;; *) _pf="$_pf $_n" ;; esac
    done < "$_pq/env0"
    [ -z "$_rec" ] || _pf="$_pf (the env -0 list does not end in a NUL byte)"
    # Both directions: no name outside the allowlist, and EVERY name of the
    # allowlist present — a name exported away inside the window (`export -n`)
    # is gone from `env -0` while P-h, P-i and P-j, which read shell variables,
    # still see it. (An `env` that ignored `-0` and printed lines gives no NUL,
    # so no record: the zero-population arm below, not this one, reds it.)
    _pfm=""
    for _n in $_FGIT_ENV_REQ; do case "$_pfseen" in *" $_n "*) : ;; *) _pfm="$_pfm $_n" ;; esac; done
    if [ "$_pfrc" -ne 0 ] || [ "$_pfn" -eq 0 ]; then echo "!! CONTROL NOT EXERCISED ($_pf_lbl): \`env -0\` exited $_pfrc with $_pfn record(s)" >&2; _fpv=1
    elif [ -n "$_pf" ] || [ -n "$_pfm" ]; then echo "!! CONTROL FAILED ($_pf_lbl):$_pf${_pfm:+ missing$_pfm}" >&2; _fpv=1; fi
  fi
  # P-i: the window pins the files ref format, and its plain init has it. A git
  # without `--show-ref-format` (before 2.45) has no reftable, so its answer is
  # files by construction — but such a git does not fail: `rev-parse` echoes an
  # option it does not know and exits 0, so that literal echo is what means
  # "files". Any other answer, including a failed call or a format name this
  # wire has not met, stays itself and is red unless it is `files`.
  _rf="$(_fgit_in "$_pq/a" rev-parse --show-ref-format 2>/dev/null)" || _rf="[rev-parse failed]"
  case "$_rf" in --show-ref-format) _rf=files ;; esac
  if [ "${GIT_DEFAULT_REF_FORMAT:-}" != files ] || [ "$_rf" != files ]; then
    echo "!! CONTROL FAILED ($_pi_lbl): GIT_DEFAULT_REF_FORMAT is [${GIT_DEFAULT_REF_FORMAT:-}], a plain init has [$_rf]" >&2; _fpv=1
  fi
  # P-j: the window's `PATH` is `$_FGIT_BIN` first, the caller's own entries
  # after it verbatim — and its `git` is the pinned wrapper, which takes ITSELF
  # off the `PATH` it hands on. It pins that construction the way P-h and P-i
  # pin their allowlist entries: on a machine whose first `PATH` entry is
  # already `$_FGIT_BIN`, dropping the construction changes nothing else, and
  # P-e's reference moves with it, so nothing but this sees it. Which OTHER tool
  # a caller entry finds is outside P (docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md §1 and §0.1)
  # — P-j checks only that `git` is the pinned one, that the pin comes first,
  # and that a `git` run through it sees a `PATH` without it: an alias is a shell
  # the `git` runs, and prints the `PATH` it was given. A `PATH` that still held
  # `$_FGIT_BIN` is the way back a "next `git` on `PATH`" wrapper takes into the pin.
  _pj=""
  case "$PATH" in "$_FGIT_BIN"|"$_FGIT_BIN":*) ;; *) _pj="$_pj first-entry" ;; esac
  _pjg="$(type -P git)" || _pjg=""
  [ "$_pjg" = "$_FGIT_BIN/git" ] || _pj="$_pj git:[$_pjg]"
  _pjrc=0; _pjp="$(_fgit_in "$_pq/a" -c 'alias.k2p=!printf %s "$PATH"' k2p 2>/dev/null)" || _pjrc=$?
  if [ "$_pjrc" -ne 0 ] || [ -z "$_pjp" ]; then _pj="$_pj alias-probe:[exit $_pjrc]"
  else case ":$_pjp:" in *":$_FGIT_BIN:"*) _pj="$_pj pin-on-the-PATH-it-passes-on" ;; esac; fi
  if [ -n "$_pj" ]; then echo "!! CONTROL FAILED ($_pj_lbl):$_pj" >&2; _fpv=1; fi
  # P-h: the window reads in the wire's locale.
  if [ -z "$_FGIT_WIRE_LC" ] || [ "${LC_ALL:-}" != "$_FGIT_WIRE_LC" ]; then
    echo "!! CONTROL FAILED ($_ph_lbl): LC_ALL is [${LC_ALL:-}], the wire's is [$_FGIT_WIRE_LC]" >&2; _fpv=1
  fi
  # P-g: every fixture repo's persisted configuration is exactly what a plain
  # `git init` in this window writes — P-a's probe repo, `$_pq/a`, is that init
  # — derived from git itself, per run, so no key list: anything a fixture
  # persisted beyond it is red.
  _pgref="$_pq/pgref.lines"   # the reference's LINE listing: built below, only if a message needs it
  # `_pg_z <git dir> <out>`: the `-z` listing as sorted records
  # "origin<TAB>key<LF>value", each written as ONE line by `printf %q` (a
  # value's newline becomes `$'\n'`), so record boundaries cannot be forged
  # and a plain line `sort` orders them — no `sort -z`, which not every
  # supported `sort` has (PR #527 Codex R24/R25). An odd field count (a
  # truncated listing) fails, and so does a listing that does not end in a NUL
  # byte (the NUL-list policy above).
  _pg_z() {
    _fgit_in "$1" config --list --show-origin -z > "$2.raw" || return $?
    _pgzo=""; _pgzn=0
    : > "$2.rec"
    while IFS= read -r -d '' _pgzf; do
      if [ $((_pgzn % 2)) -eq 0 ]; then _pgzo="$_pgzf"; else printf '%q\n' "$_pgzo	$_pgzf" >> "$2.rec"; fi
      _pgzn=$((_pgzn + 1))
    done < "$2.raw"
    [ -z "$_pgzf" ] || return 4
    [ $((_pgzn % 2)) -eq 0 ] && [ "$_pgzn" -gt 0 ] || return 3
    sort "$2.rec" > "$2"
  }
  _pg_z "$_pq/a" "$_pq/pgref.z" || : > "$_pq/pgref.z"
  _pg=""; _pk=""
  if [ ! -s "$_pq/pgref.z" ]; then _pg=" (no reference configuration)"; fi
  # The population is the CENSUS's own list, `$_pq/pre`: every `.git` entry the
  # fixtures produced, anywhere under $CTL (hidden and nested included, and
  # inside `.git` dirs). A clean census is what lets this loop run at all, and
  # it guarantees each `.git` entry is a real directory with a clean shape (a
  # gitfile, a symlink, a bare, separate, submodule or linked-worktree git dir
  # is UNKNOWN and red there, never skipped), so the loop only acts on the
  # `.git` entries and counts them: none is red.
  _pgn=0
  while IFS= read -r -d '' _pge; do
    [ "${_pge##*/}" = .git ] || continue
    _pgn=$((_pgn + 1)); _pgd="${_pge%/.git}"; _pgl="${_pgd#"$CTL"/}"
    # P-k: no fixture repo may read objects from a store outside it. Git's OWN
    # answer, not a file name: an `objects/info/alternates` naming another
    # store makes git read an object from there instead of writing it
    # locally, so the content the controls read is an outside input even
    # though P's ids do not move — P-g's origin comparison does not see it
    # (measured, docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md, the
    # P-k table row), but `count-objects -v`'s `alternate:` line does. A
    # failed call, or ANY stderr (a store that does not exist, or an
    # `alternates` naming a directory or a mode-000 path, gives rc 0, no
    # `alternate:` line and a warning or error on stderr), is red too — such a
    # store could appear later and be read. Runs only over the clean census,
    # on every `.git` P-g compares.
    _pkrc=0; _pko="$(_fgit_in "$_pgd" count-objects -v 2>"$_pq/pk.err")" || _pkrc=$?
    if [ "$_pkrc" -ne 0 ] || [ -s "$_pq/pk.err" ]; then
      _pk="$_pk $_pgl:[count-objects exit $_pkrc$( [ ! -s "$_pq/pk.err" ] || printf ', stderr: %s' "$(tr '\n' ' ' < "$_pq/pk.err")" )]"
    else
      # ⚠ grep's 1 is "no alternate line"; anything above it is a failed
      # comparison, which is red — not an absent line.
      _pkgrc=0; _pka="$(printf '%s\n' "$_pko" | grep '^alternate:')" || _pkgrc=$?
      if [ "$_pkgrc" -gt 1 ]; then _pk="$_pk $_pgl:[the comparison failed (grep exit $_pkgrc)]"
      elif [ -n "$_pka" ]; then _pk="$_pk $_pgl:[$(printf '%s' "$_pka" | tr '\n' ';')]"; fi
    fi
    # P-g: THE AUTHORITATIVE COMPARISON is of RECORDS, and runs first. A
    # line-based comparison can be forged by a value holding a newline shaped
    # like another `--show-origin` line (PR #527 Codex R24) and passes a line
    # written twice (R17). `_pg_z` lists `-z` (NUL-bounded origin and
    # key/value), pairs each origin with its entry into one record, and sorts;
    # the reference's and this repo's must be byte-identical. The listing must
    # EQUAL the reference as a set: an entry beyond a plain init is an input the
    # fixture persisted, and an entry a plain init writes but the fixture removed
    # (`git config --unset core.filemode`) hands that setting to the platform
    # default — either way P would depend on more than the fixture. Any non-zero
    # — a difference, or a listing/sort/cmp that could not run — is red. ⚠ BOTH
    # DIRECTIONS: a `.git` git does not recognise lists NOTHING, and an empty
    # listing is `_pg_z`'s failure (3), not "no extra key".
    _pgcrc=0
    { _pg_z "$_pgd" "$_pq/pgcur.z" && cmp -s "$_pq/pgcur.z" "$_pq/pgref.z"; } || _pgcrc=$?
    [ "$_pgcrc" -eq 0 ] && continue
    # RED, whatever follows: the line listings only build the MESSAGE, so a
    # failure of theirs cannot turn this red green.
    _pgrc=0; _pgc="$(_fgit_in "$_pgd" config --list --show-origin 2>&1)" || _pgrc=$?
    if [ "$_pgrc" -ne 0 ] || [ -z "$_pgc" ]; then
      _pg="$_pg $_pgl:[git lists no configuration here (exit $_pgrc)]"; continue
    fi
    printf '%s\n' "$_pgc" > "$_pq/pgcur"
    [ -s "$_pgref" ] || _fgit_in "$_pq/a" config --list --show-origin > "$_pgref" 2>/dev/null || : > "$_pgref"
    _pgx="$(printf '%s\n' "$_pgc" | grep -vxF -f "$_pgref")" || _pgx=""
    _pgm="$(grep -vxF -f "$_pq/pgcur" "$_pgref")" || _pgm=""
    if [ -z "$_pgx" ] && [ -z "$_pgm" ]; then
      _pg="$_pg $_pgl:[its configuration records differ from a plain init's (a repeated entry, or a value holding a newline), or could not be compared (exit $_pgcrc)]"
    else
      [ -z "$_pgx" ] || _pg="$_pg $_pgl:[$(printf '%s' "$_pgx" | tr '\t\n' ' ;')]"
      [ -z "$_pgm" ] || _pg="$_pg $_pgl:[removed: $(printf '%s' "$_pgm" | tr '\t\n' ' ;')]"
    fi
  done < "$_pq/pre"
  [ "$_pgn" -gt 0 ] || _pg="$_pg (no fixture git dir was found)"
  if [ -n "$_pg" ]; then echo "!! CONTROL FAILED ($_pg_lbl):$_pg" >&2; _fpv=1; fi
  if [ -n "$_pk" ]; then echo "!! CONTROL FAILED ($_pk_lbl):$_pk" >&2; _fpv=1; fi
  # P-k's liveness: a THIRD probe repo in the postconditions' directory (so
  # P-a's and P-d's, `a` and `b`, stay untouched), given an `alternates` file
  # naming P-a's own object store. The path is written in double-quoted C
  # form, so a path holding a newline stays one line (git(1) documents
  # C-style quoting for `GIT_ALTERNATE_OBJECT_DIRECTORIES`, and that the
  # `alternates` file accepts the same form is measured). With no machine
  # limitation arm: "prints none" cannot tell a git without the line from a
  # broken probe, so any other outcome is red.
  _pkl=""
  if mkdir "$_pq/c" && ( cd "$_pq/c" && git init -q . ) >/dev/null 2>&1; then
    mkdir -p "$_pq/c/.git/objects/info"
    printf '"%s"\n' "$(printf '%s' "$_pq/a/.git/objects" | sed 's/\\/\\\\/g; s/"/\\"/g')" > "$_pq/c/.git/objects/info/alternates"
    _pklrc=0; _pklo="$(_fgit_in "$_pq/c" count-objects -v 2>"$_pq/pkl.err")" || _pklrc=$?
    _pkln="$(printf '%s\n' "$_pklo" | grep -c '^alternate:')" || _pkln=0
    if [ "$_pklrc" -ne 0 ] || [ -s "$_pq/pkl.err" ] || [ "$_pkln" -ne 1 ]; then _pkl=bad; fi
  else
    _pkl=bad
  fi
  if [ -n "$_pkl" ]; then echo "!! CONTROL NOT EXERCISED ($_pkl_lbl)" >&2; _fpv=1; fi
  # P-c last: nothing in the window may have written into the void. Tested by
  # the shell's own globs, not by `$(ls -A …)`: command substitution strips
  # trailing newlines, so an entry whose name is only newlines read as an
  # empty listing (PR #527 Codex R14). A void the globs cannot list — not a
  # directory, or not readable and searchable — is red too: globs over an
  # unreadable directory expand to nothing, the same silent "empty" a failed
  # `ls` gave (the fix-delta review after R14).
  # The globs are this shell's, and the fixtures file ran in this shell: a
  # `set -f` or `GLOBIGNORE` it left behind would make them match nothing, so
  # both are reset here (the fix-delta re-check after R16).
  set +f; unset GLOBIGNORE
  _pcv=""
  if [ ! -d "$_FGIT_VOID" ] || [ -L "$_FGIT_VOID" ]; then _pcv=" (the void is not a directory)"
  elif [ ! -r "$_FGIT_VOID" ] || [ ! -x "$_FGIT_VOID" ]; then _pcv=" (the void cannot be listed)"; fi
  for _pce in "$_FGIT_VOID"/* "$_FGIT_VOID"/.[!.]* "$_FGIT_VOID"/..?*; do
    if [ -e "$_pce" ] || [ -L "$_pce" ]; then _pcv="$_pcv [$(printf '%s' "${_pce#"$_FGIT_VOID"/}" | tr '\n' '?')]"; fi
  done
  if [ -n "$_pcv" ]; then echo "!! CONTROL FAILED ($_pc_lbl):$_pcv" >&2; _fpv=1; fi
  return "$_fpv"
}
# Are errexit, nounset and pipefail all in force? Asked in the window before and
# after the fixtures file; one `case` per option, so no pattern depends on the
# order `$SHELLOPTS` lists them in.
_fw_opts_on() {
  case ":$SHELLOPTS:" in *:errexit:*) : ;; *) return 1 ;; esac
  case ":$SHELLOPTS:" in *:nounset:*) : ;; *) return 1 ;; esac
  case ":$SHELLOPTS:" in *:pipefail:*) : ;; *) return 1 ;; esac
}
# A mode restriction a fixture needs (an unsearchable dir, an unreadable file)
# is RECORDED here, not applied: the window applies every one after the
# postconditions, so the P-g census sees the whole tree. $1 = path, $2 = mode,
# $3 = the fixture, which is marked failed if the mode cannot be applied.
# ⚠ A PATH IS DATA, NOT PROTOCOL: the manifest is line-based, so it stores the
# path RELATIVE to `$CTL` and refuses one outside `$CTL` or holding a newline or
# a TAB — a raw path with a newline in `$CTL` would otherwise split into a line
# naming a directory outside the scratch root, and chmod it.
# A mode that is refused or cannot be applied is RED (`seal_failed`, label
# `$_fws_lbl`), not only a failed fixture: controls gated on the mode having
# taken effect would otherwise be skipped as a machine limitation.
# ⚠ AND IT CAN NEVER REACH OUTSIDE `$CTL`: a path with a `.` or `..` component
# is refused here, and `_seal_apply` refuses a path with a symlink ANYWHERE on
# it (`chmod` follows symlinks, and fixtures hold them; `_seal_apply_probe`,
# below, asserts that at load). A refusal marks the fixture failed as well.
_seal_refuse() { # $1 = fixture, $2 = why
  _fixture_failed "$1"
  printf '%s\n' "$1: $2" >> "$_FW_DIR/seal_failed"
}
_seal() {
  _sl_nl="$(printf '\nx')"; _sl_nl="${_sl_nl%x}"; _sl_tab="$(printf '\t')"
  # The manifest line is `mode TAB fixture TAB path`, so ALL THREE arguments are
  # data to check: a TAB or newline in the mode or the fixture name would shift
  # the fields and make the path something else. A bad fixture name is refused
  # under a fixed one (it would otherwise be written back as it came).
  case "$3" in ""|*[!A-Za-z0-9_-]*)
    _seal_refuse "(seal)" "the fixture name [$(printf '%s' "$3" | tr '\n\t' '??')] is not [A-Za-z0-9_-]+"; return 0 ;;
  esac
  case "$2" in ""|*[!0-7]*)
    _seal_refuse "$3" "the mode [$(printf '%s' "$2" | tr '\n\t' '??')] is not octal digits"; return 0 ;;
  esac
  case "$1" in
    "$CTL"/*) _sl_r="${1#"$CTL"/}" ;;
    *) _seal_refuse "$3" "[$1] is outside the fixture root"; return 0 ;;
  esac
  case "$_sl_r" in *"$_sl_nl"*|*"$_sl_tab"*)
    _seal_refuse "$3" "a path holding a newline or a TAB"; return 0 ;;
  esac
  case "/$_sl_r/" in */../*|*/./*|*//*)
    _seal_refuse "$3" "[$_sl_r] has a . or .. or empty component"; return 0 ;;
  esac
  printf '%s\t%s\t%s\n' "$2" "$3" "$_sl_r" >> "$_FW_DIR/seal"
}
_seal_apply() {
  [ -e "$_FW_DIR/seal" ] || return 0
  while IFS="$(printf '\t')" read -r _sm _sf _sp; do
    # A fixture whose chain already failed is reported once, by that failure.
    case " $_FIX_FAILED " in *" $_sf "*) continue ;; esac
    _sa_p="$CTL"; _sa_rest="$_sp"; _sa_link=""
    while [ -n "$_sa_rest" ]; do
      _sa_p="$_sa_p/${_sa_rest%%/*}"
      [ ! -L "$_sa_p" ] || { _sa_link="$_sa_p"; break; }
      case "$_sa_rest" in */*) _sa_rest="${_sa_rest#*/}" ;; *) _sa_rest="" ;; esac
    done
    if [ -n "$_sa_link" ]; then _seal_refuse "$_sf" "[$_sp] passes through a symlink"; continue; fi
    chmod "$_sm" "$CTL/$_sp" 2>/dev/null || _seal_refuse "$_sf" "chmod $_sm $_sp failed"
  done < "$_FW_DIR/seal"
}
# Run the fixtures file in the window. $1 = fixtures file. Sets
# _fw_rc/_fw_done/_fw_post_ok/_fw_post_bad/_fw_why/_fw_diag, _fw_seal_bad,
# _fw_limits, _FIX_FAILED and, at its end, _fw_trusted.
_fgit_window() {
  _fwf="$1"
  mkdir -p "$_FW_DIR" || { _fw_why="the window's directory could not be created"; return 0; }
  {
    printf 'set -euo pipefail\n'
    # Plain assignments, never `declare -p`: that would carry an `export`
    # attribute into the window (measured: P-f caught it).
    for _fwn in CTL CONTROL_REMOVED CONTROL_K2 CONTROL_TOOLS CONTROL_BINARY CONTROL_CLEAN \
      _REAL_GIT _REAL_GREP _fifo_ok _FGIT_VOID _FGIT_ENVBIN _FGIT_ENV_REQ _FGIT_WIRE_EXEC \
      _FGIT_WIRE_LC _FGIT_BIN _FW_DIR; do
      printf '%s=%q\n' "$_fwn" "${!_fwn:-}"
    done
    # The postcondition labels, by the names `_fgit_postconditions` USES —
    # derived from its body, not listed — each of which the controls file must
    # define non-empty; one it does not refuses the window, so a renamed label
    # cannot be forwarded empty in silence.
    _fwls="$(declare -f _fgit_postconditions | grep -o '_[A-Za-z0-9_]*_lbl' | sort -u)" || _fwls=""
    for _fwn in $_fwls; do
      if [ -z "${!_fwn:-}" ]; then
        printf 'echo %q > "$_FW_DIR/cause"; exit 1\n' "the postcondition label \$$_fwn is not defined, or is empty"
      else
        printf '%s=%q\n' "$_fwn" "${!_fwn}"
      fi
    done
    printf '_FIX_FAILED=""\n'
    declare -f _fixture_failed _shq _fgit_shim _fgit_canon _fw_opts_on _seal_refuse _seal _seal_apply _fgit_postconditions
  } > "$_FW_DIR/prelude.sh" || { _fw_why="the window prelude could not be written"; return 0; }
  # The fixtures file is sourced from a COPY in the window's own directory, by a
  # relative name: bash prefixes each diagnostic with the name it was given, so
  # W3's prefixes below are `./fixtures.sh:` and `./prelude.sh:` whatever the
  # checkout path holds (a newline in it would otherwise split the name across
  # two stderr records and hide the diagnostic — PR #527 Codex R2).
  cp "$_fwf" "$_FW_DIR/fixtures.sh" || { _fw_why="the fixtures file could not be copied into the window"; return 0; }
  # The child writes WHY it stopped into `cause` itself, as the sentence W
  # prints, so an exit status a fixtures-file command produced under errexit
  # cannot be mistaken for one of these.
  # It starts in a directory it owns, so no git it runs reads the configuration
  # of a repository the caller happened to be in.
  "$_FGIT_ENVBIN" -i "${_FGIT_ENV[@]}" "$_FGIT_BASH" -c '
    cd "$1" || { echo "the window could not enter its own directory" > "$1/cause"; exit 1; }
    . ./prelude.sh
    _fw_opts_on || { echo "the window refused to start: a prelude option (errexit, nounset or pipefail) was not in force" > "$_FW_DIR/cause"; exit 1; }
    . ./fixtures.sh
    [ -e "$_FW_DIR/built" ] || { echo "the fixtures file returned before its last line" > "$_FW_DIR/cause"; exit 1; }
    _fw_opts_on || { echo "the fixtures file switched off errexit, nounset or pipefail" > "$_FW_DIR/cause"; exit 1; }
    printf "K2-WINDOW-STDERR-CANARY\n" >&2
    _fgit_postconditions && : > "$_FW_DIR/post_ok" || : > "$_FW_DIR/post_bad"
    _seal_apply
    printf "%s" "$_FIX_FAILED" > "$_FW_DIR/fix_failed"
    : > "$_FW_DIR/done"' _ "$_FW_DIR" 2> "$_FW_DIR/stderr" || _fw_rc=$?
  # W3 reads the window's stderr only if fd 2 still pointed at the captured
  # file after the fixtures file: the canary above is written right after the
  # options re-check, so its absence from the capture means the fixtures file
  # redirected fd 2 before then (`exec 2>/dev/null`) — a window that is not
  # complete (below). The canary is taken OUT of the REPLAY only, wherever it
  # sits in a record (a fixture's stderr without a final newline appends bash's
  # own diagnostic to that same line). W3's scan below reads the raw capture:
  # the canary's text holds neither of its two prefixes, so it cannot match there.
  _fw_fd2=0
  _FW_M="K2-WINDOW-STDERR-CANARY" awk 'index($0, ENVIRON["_FW_M"]) > 0 { f = 1 } END { exit !f }' "$_FW_DIR/stderr" 2>/dev/null || _fw_fd2=1
  _FW_M="K2-WINDOW-STDERR-CANARY" awk '{ i = index($0, ENVIRON["_FW_M"]); if (i > 0) { $0 = substr($0, 1, i - 1) substr($0, i + length(ENVIRON["_FW_M"])); if ($0 == "") next } print }' "$_FW_DIR/stderr" >&2 2>/dev/null || true
  # A shell diagnostic located in the fixtures file or the prelude means a line
  # of it was skipped — an arithmetic-expansion error does not stop a sourced
  # file. The property is WHERE the line starts: bash prefixes every diagnostic
  # about a file with that file's path and `:` (`… line N:`, `… command
  # substitution: line N:`, `… eval: line N:`, in any locale), so no wording
  # is matched. The paths reach `awk` through the environment, which interprets
  # nothing, and `awk` stops after three: no pipe, so no SIGPIPE.
  _fw_dg=0
  # ANYWHERE in a record, not only at its start: a fixture that writes
  # stderr without a newline makes bash append its diagnostic to that record
  # (`x./fixtures.sh: … division by 0`; PR #527 Codex R25).
  _fw_diag="$(_FW_A="./fixtures.sh:" _FW_B="./prelude.sh:" awk '
    index($0, ENVIRON["_FW_A"]) > 0 || index($0, ENVIRON["_FW_B"]) > 0 { print; if (++n == 3) exit }
  ' "$_FW_DIR/stderr" 2>&1)" || _fw_dg=$?
  [ "$_fw_dg" -eq 0 ] || _fw_diag="(the scan of the window's stderr failed: awk exited $_fw_dg)"
  if [ -e "$_FW_DIR/done" ] && [ "$_fw_fd2" -eq 1 ]; then
    _fw_why="the fixtures file left the window's stderr redirected, so a shell diagnostic could not be read"
  elif [ -e "$_FW_DIR/done" ]; then _fw_done=1; else
    _fw_why="$(cat "$_FW_DIR/cause" 2>/dev/null)" || _fw_why=""
    [ -n "$_fw_why" ] || _fw_why="the window exited $_fw_rc before completing (the fixtures file exited or aborted, or a postcondition aborted)"
  fi
  [ ! -e "$_FW_DIR/post_ok" ] || _fw_post_ok=1
  [ ! -e "$_FW_DIR/post_bad" ] || _fw_post_bad=1
  _fw_seal_bad="$(cat "$_FW_DIR/seal_failed" 2>/dev/null)" || _fw_seal_bad=""
  _fw_limits="$(cat "$_FW_DIR/machine_limits" 2>/dev/null)" || _fw_limits=""
  _FIX_FAILED="$(cat "$_FW_DIR/fix_failed" 2>/dev/null)" || _FIX_FAILED=""
  # TRUST IS A PROPERTY OF THE WINDOW, decided here and nowhere else: complete,
  # the postconditions REACHED a clean verdict (`post_ok`, written only when
  # they returned 0 — the absence of `post_bad` is not that: a postcondition
  # that never ran to its end writes neither), none red, no shell diagnostic. The verdict function below only
  # REPORTS it, so where that call sits (or whether an exit in it survives)
  # cannot change what `_control`'s gate (`_fw_built_or_w2`) reads.
  if [ "$_fw_done" -eq 1 ] && [ "$_fw_post_ok" -eq 1 ] && [ "$_fw_post_bad" -eq 0 ] && [ -z "$_fw_diag" ]; then _fw_trusted=1; fi
  return 0
}
# THE WINDOW'S VERDICT, REPORTED IN ONE PLACE. An INCOMPLETE window is not "no
# fixture failed": nothing was built, so no control may run over it — report W
# alone and end the run "decided nothing" (2, the wire's convention; the
# number is not consumed by the driver). A complete window that is not TRUSTED
# (docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md §3, "A red
# postcondition ends the run before any control") gets its diagnostic reported
# here and the run ended at 1, before any control. This function computes
# NOTHING: `_fw_trusted` comes from `_fgit_window`; this function reads it to
# report, and `_fw_built_or_w2` reads it to gate.
_fgit_window_verdict_exit() { # $1 = the window label
  if [ "$_fw_done" -ne 1 ]; then
    echo "!! CONTROL NOT EXERCISED ($1): $_fw_why; nothing was built, so no control was run" >&2
    exit 2
  fi
  if [ "$_fw_trusted" -eq 1 ]; then return 0; fi
  # A shell diagnostic located in the fixtures file or the prelude (W3) means a
  # line was skipped: an arithmetic-expansion error does not stop a sourced file. A red
  # postcondition's own diagnostic already reached stderr, replayed above, so
  # it needs no separate report here.
  [ -z "$_fw_diag" ] || echo "!! CONTROL FAILED ($_fwd_lbl): $(printf '%s' "$_fw_diag" | tr '\n' ' ')" >&2
  # A complete window with neither `post_ok` nor `post_bad`: the postconditions
  # never reached a verdict (a statement of theirs ended the window's postcondition
  # call without returning), and nothing else has said so.
  if [ "$_fw_post_ok" -eq 0 ] && [ "$_fw_post_bad" -eq 0 ]; then
    echo "!! CONTROL FAILED ($1): the postconditions never reached a verdict" >&2
  fi
  exit 1
}

# ONE FACT, RECORDED ONCE: DID THIS FIXTURE'S BUILD CHAIN SUCCEED?
# Every fixture in `…trip-wire.fixtures.sh` is built as
# `( cd … && … ) || _fixture_failed <name>`, and
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
# `$_FIX_FAILED` itself lives in the window (the prelude starts it empty) and
# comes back through a file, which `_fgit_window` reads whatever the window's
# outcome; the gate is at the consumer: `_control` reads it only after
# `_fw_built_or_w2` has passed, i.e. over a complete window.
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
# answered once, reported once — the shape `_perm_ok` (below) uses for the
# other capability this harness cannot assume.
_fifo_ok=0
if mkfifo "$SCRATCH/fifoprobe" 2>/dev/null; then _fifo_ok=1; command rm -f "$SCRATCH/fifoprobe"; fi

# CAN THIS USER BE KEPT FROM READING A FILE BY ITS MODE? The perm controls
# (`err`, `walk`, `d2red`, `d5root`, umask) need a mode-000 file to be unreadable
# — not true of root. ASKED OF A FILE OF ITS OWN, like `_fifo_ok`, never of a
# fixture: the fixture's seal can fail, and a capability read off it turned
# "the err fixture did not build" into "this machine cannot enforce modes", and
# the five controls were skipped under a green PASSED.
_perm_ok=1
{ : > "$SCRATCH/permprobe" && chmod 000 "$SCRATCH/permprobe"; } || { echo "!! the permission probe could not be made in $SCRATCH; this run decided nothing." >&2; exit 2; }
if [ -r "$SCRATCH/permprobe" ]; then _perm_ok=0; fi

# ⚠ NO SECOND `trap ... EXIT` HERE. `trap` REPLACES; a second one silently
# discarded the scratch-root cleanup and left an empty directory behind on
# every successful run (#501 R94, reproduced). `CTL` is created UNDER
# `$SCRATCH`, so the trap at the top already removes it — one owner, one
# cleanup, nothing to compose.

_REAL_GIT="$(_shq "$_FGIT_GIT")"
_REAL_GREP="$(_shq "$(_fgit_resolve grep)")"
# ⚠ ASSERTED, NOT A `_control`: this is the harness's own part, not an arm of the
# wire. It is checked by round-tripping a path that holds each thing that broke
# it.
_shq_probe="/tool dir/it's \$HOME \`x\`"
if [ "$(sh -c "printf %s $(_shq "$_shq_probe")")" != "$_shq_probe" ]; then
  echo "!! the shim-quoting helper does not round-trip a path through /bin/sh, so every" >&2
  echo "   shim built with it may exec the wrong thing. This run decided nothing." >&2
  exit 2
fi

# ⚠ ASSERTED, NOT A `_control`, LIKE THE ONE ABOVE: the walk in `_seal_apply` is
# the only thing keeping a deferred `chmod` inside `$CTL` (it follows symlinks,
# and fixtures hold them), and no fixture's seal passes through a link, so
# nothing else exercises it. The REAL `_seal_apply` is run, in a subshell, over a
# probe tree of its own in `$SCRATCH` (its own `CTL` and `_FW_DIR`) whose
# manifest names a path through a symlink; it must refuse it, naming it, and
# leave the target's mode alone. The walk is local to `_seal_apply` on purpose:
# the wire's `_ancestor_link` skips the leaf and is the scanner's logic.
_seal_apply_probe() (
  CTL="$SCRATCH/sealprobe/ctl"; _FW_DIR="$SCRATCH/sealprobe/win"; _FIX_FAILED=""
  mkdir -p "$CTL/real" "$_FW_DIR" && : > "$CTL/real/f" && chmod 644 "$CTL/real/f" \
    && ln -s real "$CTL/lnk" || return 1
  printf '%s\t%s\t%s\n' 000 sealprobe lnk/f > "$_FW_DIR/seal"
  _seal_apply
  _sp_mode="$(ls -ld "$CTL/real/f")"
  case "${_sp_mode%% *}" in -rw-r--r--*) ;; *) return 1 ;; esac
  case "$(<"$_FW_DIR/seal_failed")" in *"[lnk/f] passes through a symlink"*) ;; *) return 1 ;; esac
)
if ! _seal_apply_probe; then
  echo "!! CONTROL FAILED ($_fws_lbl): _seal_apply followed a symlink on a sealed path or did not" >&2
  echo "   refuse it, so a deferred chmod can reach outside the fixture root. This run decided nothing." >&2
  exit 2
fi

# No control may run over a build that is not COMPLETE AND TRUSTED, WHEREVER
# the verdict exit sits (docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md §3):
# every `_control` asks first, by `_fw_trusted` alone — the one flag
# `_fgit_window` writes. `$_fw2_lbl` is defined in the controls
# file.
_fw_built_or_w2() {
  [ "$_fw_trusted" -ne 1 ] || return 0
  [ "$_fw2_said" -eq 1 ] || echo "!! CONTROL FAILED ($_fw2_lbl): a control was reached over a build that was not complete and trusted" >&2
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
  _out_f="$_VFY/.control_out"
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
    env ${_ctl_env[@]+"${_ctl_env[@]}"} "$BASH" "$SELF" --selftest "$1" "${5:-}" "${6:-}" \
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
