#!/usr/bin/env bash
# THE RECORD TABLE of the mutation set for `webref-generic-core-trip-wire.sh` —
# sourced by that wire's mutation set (`…trip-wire.mutations.sh`), never run on
# its own.
#
# WHY IT IS A SEPARATE FILE. The mutation set's growth is this table: a flat
# case table with no seam inside it, one record per line. The edit that would
# have taken the mutation set past 1000 lines splits the table out first
# (CLAUDE.md touch-time split), at the boundary that was already drawn there:
# the mutation set holds the READERS of the format — the one parser, the
# always-on correspondence and anchor checks, the opt-in run — and this file
# holds the DATA they read. The format itself (TAB-separated expressions and a
# needle, the `controls:` / `harness:` / `ctlrun:` / `fixtures:` / `mutgen:` / `mutations:` / `correspondence:` prefixes) is
# documented at the head of the mutation set, beside the readers that enforce
# it; the one rule kept here is the comment syntax, beside the one filter that
# drops it (below). The other: THIS FILE SPELLS ONLY DEFINED LABELS — a `_*_lbl` token the controls file does not define is "used but not defined" (exit 2); a record needing a NEW label builds it from its anchor by a sed backreference.
# WHAT IT CONSUMES: nothing — the function below expands no variable (its
# here-document is quoted). WHAT IT DEFINES: `_mutants`.
# ⚠ A FUNCTION, NOT `x="$(cat <<'EOF' … )"`. Under bash 3.2 — the stock macOS
# shell this wire commits to — a quoted here-document nested inside a command
# substitution is still parsed for expansions, and the `unset "$_v"` in one of
# the records below aborted the WHOLE FILE with `_v: unbound variable`, on every
# run, mutation mode or not. Measured: 3.2 red, 5.3 green.
# ⚠ AND THE FORMAT HAS A COMMENT SYNTAX — a `#` in column 1 — dropped HERE, at
# the single materialiser both readers go through, so one filter covers both, and
# a note can sit beside the records it is about. `^#` is safe to reserve: a
# record's expression cannot begin with a literal `#` except as a sed comment,
# which is inert. The always-on validator (in the correspondence part) stays — a line that is neither a
# comment nor a record is still an error, not a skip.
# ⚠ FILTERED IN BASH, NOT THROUGH `grep -v`. The mutation set runs under `pipefail`:
# `grep` exits 1 when it selects nothing, so a set that happened to be all
# comments — or a grep on `PATH` that answers differently — would abort the
# required gate from inside the thing reporting it. Same trap as the `wc -l`
# and `awk` notes in `…trip-wire.correspondence.sh`. Measured byte-identical to the `grep -v '^#'` pipe on
# bash 3.2.57 and 5.3, with status 0 at EOF on both.
_mutants() {
  while IFS= read -r _mu_line; do
    case "$_mu_line" in
      '#'*) : ;;
      *) printf '%s\n' "$_mu_line" ;;
    esac
  done <<'MUTANTS'
s/grep -aEn --/grep -En --/	K2 fires inside binary content, and the content is READ
s/"$_mrc" -gt 1/"$_mrc" -gt 99/	a failed NAME matcher fails closed
s#^\(K2RE_PATH='(^|/)\\\.claude/(skills|tools)/\[^/\)\(\]+/\[^/\]+'\)$#\1"\2#	a quote inside a name segment
s/^K2RE_PATH=.*/K2RE_PATH="$K2RE"/	a STAGED symlink target is a stored path
s/elif \[ "$_mode" = 120000 \]; then/elif [ "$_mode" = 120000 ] \&\& [ "$_src" = index ]; then/	a COMMITTED symlink target is a stored path
s|${1//$'\\n'/$_REC_SEP}|${1}|	a NEWLINE inside a name segment
s/^  _stored "${rel#"$_dir"\/}"/  : /	an entry's own NAME is the hierarchy
s/--exclude-per-directory=.gitignore \\$/--exclude-standard \\$/	per-clone info/exclude cannot hide an entry
s/\[ "$_hrc" -eq 0 \]/false/	a COMMITTED violation fixed only in the index still fires
s/if ! tr -d .\\000. < "\$_b"/if false/	a NUL-bearing staged symlink blob is not a path
s/readlink -n "$f"/readlink "$f"/	readlink's own newline is not read as stored content
s/export GIT_NO_LAZY_FETCH=1 GIT_NO_REPLACE_OBJECTS=1/export GIT_NO_LAZY_FETCH=1/	a replace ref cannot substitute the staged blob
s|^\(_esc() { \).*\("; }\)$|\1printf '%s' "$1\2|	a name cannot forge a verdict record
s/^export LC_ALL=C$/&.UTF-8/	the fixture build window reads in the wire's locale
s/_co="$(grep -aEn -- "$K2RE"/_co="$(LC_ALL=C.UTF-8 grep -aEn -- "$K2RE"/	a byte no UTF-8 locale can bracket
s/"$SCANNED" -eq 0/"$SCANNED" -eq -1/	an empty scope fails loudly
s/\[ "$_read" -eq 0 \] || printf/printf/	an inventoried entry that was never read is not counted as scanned
s/\[ -e "$p" \] || \[ -L "$p" \]/[ -e "$p" ]/	a symlinked EXTRA entry is scanned
s/REL_DIR="${SCOPE_DIR#"$ROOT"\/}"/REL_DIR="${SCOPE_DIR#$ROOT\/}"/	a glob character in the checkout path does not widen the scope
s/REL_FILE="${SCOPE_FILE#"$ROOT"\/}"/REL_FILE="${SCOPE_FILE#$ROOT\/}"/	a pattern character in the checkout path does not misplace the entry script
s/GIT_CONFIG\*) : ;;/GIT_CONFIG*) unset "$_v" ;;/	the caller's git CONFIGURATION survives the routing purge
s/\[ "$_rc_tracked" -eq 0 \]/true/	a failed TRACKED inventory fails closed
s/\[ "$_rc_worktree" -eq 0 \]/true/	a failed WORKTREE inventory fails closed
s/\[ "$_rc_head" -eq 0 \]/true/	a failed HEAD inventory fails closed
s/the HEAD inventory exited %d/the HEAD inventory was fine %d/	a failed HEAD inventory fails closed
s/ls-files -z --stage/ls-files -z --cached/	a STAGED symlink target is a stored path
s/\[ "$_shrc" -eq 0 \]/false/	a failed HEAD PROBE is not an unborn HEAD
s/\[ "$_shrc" -ne 1 \]/true/	an orphan branch is an unborn HEAD, not a read failure
s/\[^A-Za-z0-9_\.~@+%-\]/[^A-Za-z0-9_.~+%-]/	a PATH character before .claude is not a prose boundary
s/\[^A-Za-z0-9_.~@+%-\]/[[:space:]"]/	the leading boundary covers =, --opt=, :, a backtick, ** and ,
s/elif _ancestor_link "$rel"; then/elif false; then/	the worktree read does not traverse an ancestor symlink
s/\[ "$_catrc" -ne 0 \]/false/	a failed staged-blob read is not a clean target
s/)}>,;\]/]/g	closing punctuation is not a path segment
s/elif ! _git -C "$ROOT" --literal-pathspecs/elif false \&\& ! _git -C "$ROOT" --literal-pathspecs/	an inventoried path that vanished is not silently skipped
s/--literal-pathspecs ls-files --error-unmatch/ls-files --error-unmatch/	the vanished-path question is asked of a LITERAL path
s/\[ "$_hrc" -ne 0 \]/false/	a malformed HEAD ref is not an unborn repository
s|\[^/\[:space:\]\]+/(|[^/[:space:])}>,;]+/(|	punctuation BEFORE a slash is part of the path
s/(^|\/)\\\.claude/\\.claude/	a segment merely ENDING in .claude is not the host path
# (A RECORD WAS REMOVED HERE in CR2: it rewrote K2RE_PATH's whole value, `.*`, restating text it never read, and its mutant was byte-identical to the line above's, K2RE_PATH without `(^|/)`; the corpus §6.1 remove row for records.sh:81. The line stays because the documents cite records by their records.sh line.)
s/_SELFTEST="$2"/_SELFTEST="${WEBREF_WIRE_SELFTEST:-$2}"/	an exported SELFTEST cannot redirect the scan
s/\[ ! -d "${2:-}" \]/false/	a missing self-test root decides nothing
s/^SCRATCH="$(_phys "$_raw_scratch")"/SCRATCH="$_raw_scratch"/	a relative scratch dir is removed on exit
s/if \[ "$_ENDS" -ne 1 \] || /if false \&\& /	a walk killed mid-scan is not a verdict
s/\[ -x "$_ab_p" \] && return 0/return 0/	a tracked file under an unsearchable dir is not absent
s/|| \[ -L "$_ab_p" \] || continue/|| [ -L "$_ab_p" ] || return 1/	a tracked directory deleted wholesale stays green
s/\[ -d "$_ab_p" \] || return 0/[ -d "$_ab_p" ] || return 1/	a tracked directory replaced by a file stays green
s|\[^/\[:space:\]\]+/(|[^]/[:space:]]+/(|	a ] inside a first segment
s|\[^/\[:space:\]\]+/(|[^/[:space:]"]+/(|	a double quote inside a first segment
s|\[^/\[:space:\]\]+/(|[^/[:space:]'"'"']+/(|	a single quote inside a first segment
s|\[^/\[:space:\]\]+/(|[^/[:space:]`]+/(|	a backtick inside a first segment
s|\[^/\[:space:\]\]+/|[^]/[:space:]]+/|2	a ] inside an intermediate segment
s|\[^/\[:space:\]\]+/|[^/[:space:]"]+/|2	a double quote inside an intermediate segment
s|\[^/\[:space:\]\]+/|[^/[:space:]'"'"']+/|2	a single quote inside an intermediate segment
s|\[^/\[:space:\]\]+/|[^/[:space:]`]+/|2	a backtick inside an intermediate segment
s|\[^/\[:space:\]\]+/(|[^/[:space:]"]+/(|	a quoted one-segment reference before a slash token fails safe
s/SCRATCH="$(_phys "$_raw_scratch")" || SCRATCH=""/SCRATCH="$(_phys "$_raw_scratch")"/	a restrictive umask decides nothing
s/_root_p="$(_phys "$ROOT")" || _root_p=""/_root_p="$(_phys "$ROOT")"/	an unresolvable self-test root decides nothing
s/^unset GREP_OPTIONS$/:/	a caller's GREP_OPTIONS cannot hide a file
s/exec git -c core\.fsmonitor=false /exec git /	a caller's fsmonitor hook does not run
s/^K2RE='(^|\[/K2RE='([/	a reference at the start of a line fires
s/\[^A-Za-z0-9_\.~@+%-\]/[^A-Za-z0-9_.@+%-]/	a PATH character before .claude is not a prose boundary
s/\[^A-Za-z0-9_\.~@+%-\]/[^A-Za-z0-9_.~@]/	a PATH character before .claude is not a prose boundary
s/\[^A-Za-z0-9_.~@+%-\]/[^A-Za-z0-9_~@+%-]/	a PATH character before .claude is not a prose boundary
s/\[^A-Za-z0-9_\.~@+%-\]/[^A-Za-z0-9.~@+%-]/	a PATH character before .claude is not a prose boundary
s/\[^A-Za-z0-9_\.~@+%-\]/[^A-Za-z_.~@+%-]/	a PATH character before .claude is not a prose boundary
s/\[^A-Za-z0-9_\.~@+%-\]/[^a-z0-9_.~@+%-]/	a PATH character before .claude is not a prose boundary
s|\[^A-Za-z0-9_\.~@+%-\]|[^A-Za-z0-9_.~@+%/-]|	a reference written after a slash fires
s|`\]\*\[^\]|`]+[^]|	a ONE-character final segment fires
s#(\[^/\[:space:\]\]+/|#([^/[:space:]]{2,}/|#	a ONE-character intermediate segment fires
s#^K2RE_PATH='\(.*\)/\[^/\]+/\[^/\]+'#K2RE_PATH='\1/[^/]{2,}/[^/]+'#	a ONE-character first segment in a STORED path fires
s|\[^\]/\[:space:\]"'"'"'`\]\*|[^/[:space:]"'"'"'`]*|	the final segment's middle stops where its last character does
s|\[^\]/\[:space:\]"'"'"'`\]\*|[^][:space:]"'"'"'`]*|	the final segment's middle stops where its last character does
s|\[^\]/\[:space:\]"'"'"'`\]\*|[^]/"'"'"'`]*|	the final segment's middle stops where its last character does
s|\[^\]/\[:space:\]"'"'"'`\]\*|[^]/[:space:]'"'"'`]*|	the final segment's middle stops where its last character does
s|\[^\]/\[:space:\]"'"'"'`\]\*|[^]/[:space:]"`]*|	the final segment's middle stops where its last character does
s|\[^\]/\[:space:\]"'"'"'`\]\*|[^]/[:space:]"'"'"']*|	the final segment's middle stops where its last character does
s/^K2RE='\(.*\)(skills|tools)/K2RE='\1[a-z]+/	running text that only looks like a two-segment reference stays green
s|\[^/\[:space:\]\]+/(|[^/]+/(|	running text that only looks like a two-segment reference stays green
s|\[^/\[:space:\]\]+/(|[^/[:space:]]*/(|	running text that only looks like a two-segment reference stays green
s#(\[^/\[:space:\]\]+/|#([^/]+/|#	running text that only looks like a two-segment reference stays green
s#(\[^/\[:space:\]\]+/|#([^/[:space:]]*/|#	running text that only looks like a two-segment reference stays green
s/)}>,;\])'$/)>,;])'/	running text that only looks like a two-segment reference stays green
s/)}>,;\])'$/)},;])'/	running text that only looks like a two-segment reference stays green
s/)}>,;\])'$/)}>;])'/	running text that only looks like a two-segment reference stays green
s/)}>,;\])'$/)}>,])'/	running text that only looks like a two-segment reference stays green
s/\[^\]\/\[:space:\]"'"'"'`)}>,;\])'$/[^]\/[:space:]'"'"'`)}>,;])'/	running text that only looks like a two-segment reference stays green
s/\[^\]\/\[:space:\]"'"'"'`)}>,;\])'$/[^]\/[:space:]"`)}>,;])'/	running text that only looks like a two-segment reference stays green
s/\[^\]\/\[:space:\]"'"'"'`)}>,;\])'$/[^]\/[:space:]"'"'"')}>,;])'/	running text that only looks like a two-segment reference stays green
s/\[^\]\/\[:space:\]"'"'"'`)}>,;\])'$/[^\/[:space:]"'"'"'`)}>,;])'/	running text that only looks like a two-segment reference stays green
s/^K2RE='\(.*\)\\\.claude/K2RE='\1.claude/	running text that only looks like a two-segment reference stays green
s#^K2RE_PATH='(^|/)#K2RE_PATH='^#	a stored path naming .claude after a slash, under tools, fires
s#^K2RE_PATH='(^|/)\\\.claude/(skills|tools)#K2RE_PATH='(^|/)\\.claude/(skills)#	a stored path naming .claude after a slash, under tools, fires
s#^K2RE_PATH='(^|/)\\\.claude/(skills|tools)#K2RE_PATH='(^|/)\\.claude/[a-z]+#	a stored path that only looks like one stays green
s#^K2RE_PATH='\(.*\)/\[^/\]+/\[^/\]+'#K2RE_PATH='\1/[^/]*/[^/]+'#	a stored path that only looks like one stays green
s#^K2RE_PATH='(^|/)\\\.claude#K2RE_PATH='(^|/).claude#	a stored path that only looks like one stays green
# THE FIXTURE BUILD WINDOW's records (docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-corpus.md §6).
# One or more per label; most edit the harness, where the window's producers
# live, and the rest the fixtures file, whose own shape the window checks.
harness:s/^    [.] [.]\/prelude[.]sh$/&; exit 0/	the fixture build window completed
harness:s/printf 'set -euo pipefail\\n'/printf 'set -eo pipefail\\n'/	the fixture build window completed
harness:s/printf 'set -euo pipefail\\n'/printf 'set -eu\\n'/	the fixture build window completed
harness:s/printf 'set -euo pipefail\\n'/printf 'set -uo pipefail\\n'/	the fixture build window completed
fixtures:s/^mkdir -p "[$]CTL\/walk\/sub"$/return 0/	the fixture build window completed
fixtures:s/^: > "[$]_FW_DIR\/built"$/set +e; &/	the fixture build window completed
# W2's record removes BOTH of the verdict function's exits (the incomplete-window
# one, then the untrusted-build one that would otherwise end the run first) AND
# leaves the window incomplete (its `done` marker renamed), in three one-line
# expressions: the verdict function only reports, so with its exits gone the run
# reaches the first control, and only the gate can say W2. W2's producer is
# `_ctl_gate`, asked by `_ctl_run` alone (every `_control` and the two non-`_control` blocks run
# through it; the fsmonitor `git ls-files` follow-up, after the run, sits inside its `_ctl_run`'s branch; the three standing checks
# that read `_fw_trusted` print NOTHING: W2's text has one site); the record pins that W2 is reported when the
# exits are gone. It SURVIVES with `_fw_done` removed from `_fgit_window`'s trust
# computation. `_ctl_run`'s ask is pinned always-on, not by a record: removing it reds
# the run under `$_cap_lbl` on six of the ten rows (the two that want a decline, `M 0 1 yes` and
# `Y 1 0`, get a run; the three that want the gate's red, U, F1, F2, and the untrusted row
# `M 0 1 no` come back 0; row X, red through `_ctl_run`'s own channel validation, stays green) and
# under `$_cen_lbl` (the gate is the registry's writer: every declared member is then no `_control`
# this run made); measured, both shells (the row list re-measured after the CR-B2a review fixes).
harness:/^_fgit_window_verdict_exit()/,/^}/s/^    exit 2$/    :/	/^_fgit_window_verdict_exit()/,/^}/s/^  exit 1$/  :/	s/: > "[$]_FW_DIR\/done"'/: > "$_FW_DIR\/notdone"'/	no control runs over an incomplete or untrusted fixture build window
# C8's two records pin the SECOND clause of "complete and trusted": a red
# postcondition, or W3, must still stop the run before any control even with
# the untrusted-build exit itself removed. Each forces one of the five inputs the trust
# computation (in `_fgit_window`) reads (`_fw_post_bad`, `_fw_diag`; `_fw_done` is the W2 record's) and removes that exit in the
# same record; each survives with the clause it forces removed from the
# computation, which then writes 1 regardless
# (docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-corpus.md §6.1, C8 rows).
harness:/^_fgit_window_verdict_exit()/,/^}/s/^  exit 1$/  :/	s/\[ ! -e "[$]_FW_DIR\/post_bad" \] || _fw_post_bad=1/_fw_post_bad=1/	no control runs over an incomplete or untrusted fixture build window
harness:/^_fgit_window_verdict_exit()/,/^}/s/^  exit 1$/  :/	s/\[ "[$]_fw_dg" -eq 0 \] || _fw_diag=/false || _fw_diag=/	no control runs over an incomplete or untrusted fixture build window
fixtures:s/^mkdir -p "[$]CTL\/walk\/sub"$/&; _ar=$(( 1\/0 ))/	the fixture build window ran without a shell diagnostic
harness:s/"LC_ALL=C")$/"LC_ALL=C" GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=k2.probe GIT_CONFIG_VALUE_0=1)/	a window git whose inputs no fixtures-file command altered reads configuration only from its repo's config file
harness:s/_fgit_in "[$]_pq\/a" -c a[.]b=c config --list/_fgit_in "$_pq\/a" config --list/	this git reports a configuration origin outside the repo's file
harness:s/ GIT_CONFIG_NOSYSTEM=1 / /	the fixture git has no system or global layer outside the void
harness:s/GIT_CONFIG_NOSYSTEM=0 GIT_ATTR_NOSYSTEM=0 _fgit_in/_fgit_in/	this git names its system files through git var
harness:s/^mkdir "[$]_FGIT_VOID" || exit 2$/mkdir "$_FGIT_VOID" \&\& : > "$_FGIT_VOID\/k2plant" || exit 2/	nothing is written into the fixture git's void
harness:s/ "GIT_TEMPLATE_DIR=[$]_FGIT_VOID"//	the fixture git copies no template
harness:s/"LC_ALL=C")$/"LC_ALL=C" GIT_EXEC_PATH=\/nonexistent-k2)/	no exec-path override reaches the fixture git
harness:s/ "LC_ALL=C")$/)/	the fixture build window reads in the wire's locale
harness:s/ GIT_DEFAULT_REF_FORMAT=files / /	the fixture repos use the files ref format
harness:s/chmod "[$]_sm" "[$]CTL\/[$]_sp" 2>\/dev\/null/false/	every mode restriction a fixture sealed was applied
harness:s/^  "[$]_FGIT_ENVBIN" -i "/  "$_FGIT_ENVBIN" "/	the fixture build window's environment holds only its allowlist
harness:s/"[$]_pfenv" -0 > /{ "$_pfenv" -0; echo k2 >\&2; false; } > /	the fixture build window's environment holds only its allowlist
harness:s/"[$]_pfenv" -0 > /false > /	the fixture build window's environment holds only its allowlist
harness:s/"[$]_pfenv" -0 > /{ echo k2 >\&2; } > /	the fixture build window's environment holds only its allowlist
harness:s/_pfenv="[$]_FGIT_ENVBIN"/_pfenv=\/nonexistent-k2\/env/	the fixture build window's environment holds only its allowlist
harness:s/^  "[$]_pfenv" -0 > "[$]_pq\/env0" 2> "[$]_pq\/env0.err" || _pfrc=[$]?$/&; printf K2X=1 >> "$_pq\/env0"/	the fixture build window's environment holds only its allowlist
harness:s/^_FGIT_PATH="[$]_FGIT_BIN:/_FGIT_PATH="\/nonexistent-k2:$_FGIT_BIN:/	the fixture build window runs the pinned git first on its PATH
fixtures:s/^: > "[$]_FW_DIR\/built"$/_k2n=$(printf '\\nx'); : > "$_FGIT_VOID\/${_k2n%x}"; &/	nothing is written into the fixture git's void
fixtures:s/^: > "[$]_FW_DIR\/built"$/: > "$_FGIT_VOID\/k2plant"; chmod 300 "$_FGIT_VOID"; &/	nothing is written into the fixture git's void
fixtures:s/^: > "[$]_FW_DIR\/built"$/: > "$_FGIT_VOID\/k2plant"; chmod 600 "$_FGIT_VOID"; &/	nothing is written into the fixture git's void
fixtures:s/^: > "[$]_FW_DIR\/built"$/: > "$_FGIT_VOID\/k2plant"; set -f; &/	nothing is written into the fixture git's void
fixtures:s/^: > "[$]_FW_DIR\/built"$/( git init -q --bare "$CTL\/zzlc" \&\& cd "$CTL\/zzlc" \&\& git config core.excludesFile \/nonexistent-k2 \&\& mv HEAD zzh \&\& mv zzh head ); &/	every fixture repo persists only the configuration a plain git init writes
fixtures:s/^: > "[$]_FW_DIR\/built"$/( mkdir -p "$CTL\/zzuc" \&\& cd "$CTL\/zzuc" \&\& git init -q --separate-git-dir="$_FW_DIR\/gduc" . \&\& mv .git .zzg \&\& mv .zzg .GIT ); &/	every fixture repo persists only the configuration a plain git init writes
fixtures:s/^: > "[$]_FW_DIR\/built"$/git -C "$CTL\/clean" config --add core.bare false; &/	every fixture repo persists only the configuration a plain git init writes
fixtures:s/^: > "[$]_FW_DIR\/built"$/( git init -q --bare "$_FW_DIR\/zzout" \&\& git -C "$_FW_DIR\/zzout" config core.excludesFile \/nonexistent-k2 \&\& ln -s "$_FW_DIR\/zzout" "$CTL\/zzlink" ); &/	every fixture repo persists only the configuration a plain git init writes
fixtures:s/^: > "[$]_FW_DIR\/built"$/( mkdir -p "$_FW_DIR\/zzo2\/a" \&\& git init -q --bare "$_FW_DIR\/zzo2\/a\/r" \&\& ln -s "$_FW_DIR\/zzo2" "$CTL\/zzl2" ); &/	every fixture repo persists only the configuration a plain git init writes
fixtures:s/^: > "[$]_FW_DIR\/built"$/( git init -q --bare "$_FW_DIR\/zzdh" \&\& git -C "$_FW_DIR\/zzdh" config foo.bar 1 \&\& rm "$_FW_DIR\/zzdh\/HEAD" \&\& ln -s refs\/heads\/main "$_FW_DIR\/zzdh\/HEAD" \&\& ln -s "$_FW_DIR\/zzdh" "$CTL\/zzdhl" ); &/	every fixture repo persists only the configuration a plain git init writes
fixtures:s/^: > "[$]_FW_DIR\/built"$/( git init -q --bare "$_FW_DIR\/zzhd" \&\& git -C "$_FW_DIR\/zzhd" config foo.bar 1 \&\& mkdir -p "$CTL\/zzh" \&\& ln -s "$_FW_DIR\/zzhd" "$CTL\/zzh\/HEAD" ); &/	every fixture repo persists only the configuration a plain git init writes
fixtures:s/^: > "[$]_FW_DIR\/built"$/git -C "$CTL\/clean" config --unset core.bare; git -C "$CTL\/clean" config core.logallrefupdates "$(printf 'true\\nfile:.git\/config\\tcore.bare=false')"; &/	every fixture repo persists only the configuration a plain git init writes
fixtures:s/^: > "[$]_FW_DIR\/built"$/cp "$CTL\/clean\/.git\/config" "$_FW_DIR\/k2cfg" \&\& rm "$CTL\/clean\/.git\/config" \&\& ln -s "$_FW_DIR\/k2cfg" "$CTL\/clean\/.git\/config"; &/	every fixture repo persists only the configuration a plain git init writes
fixtures:s/^: > "[$]_FW_DIR\/built"$/( git init -q "$CTL\/zzfifo" \&\& rm "$CTL\/zzfifo\/.git\/HEAD" \&\& mkfifo "$CTL\/zzfifo\/.git\/HEAD" ); &/	every fixture repo persists only the configuration a plain git init writes
fixtures:s/^mkdir -p "[$]CTL\/walk\/sub"$/&; printf x >\&2; _k2p=$(( 1\/0 ))/	the fixture build window ran without a shell diagnostic
fixtures:s/^: > "[$]_FW_DIR\/built"$/rmdir "$_FGIT_VOID" \&\& mkdir "$CTL\/k2empty" \&\& ln -s "$CTL\/k2empty" "$_FGIT_VOID"; &/	nothing is written into the fixture git's void
harness:s/^  _fgit_shim "[$]_FGIT_BIN\/git" /  _fgit_shim "$_FGIT_BIN\/gitx" /	the fixture build window runs the pinned git first on its PATH
fixtures:s/^: > "[$]_FW_DIR\/built"$/( git init -q --bare "$CTL\/zzbare" \&\& cd "$CTL\/zzbare" \&\& git config core.excludesFile \/nonexistent-k2 \&\& _k2h=$(git symbolic-ref HEAD) \&\& rm HEAD \&\& ln -s "$_k2h" HEAD ); &/	every fixture repo persists only the configuration a plain git init writes
fixtures:s/^: > "[$]_FW_DIR\/built"$/echo '[include] path = \/nonexistent-k2' >> "$CTL\/clean\/.git\/config"; &/	every fixture repo persists only the configuration a plain git init writes
fixtures:s/^( cd "[$]CTL\/cachedir" && git init -q [.] /( cd "$CTL\/cachedir" \&\& git init -q --separate-git-dir="$CTL\/.gd-cachedir" . /	every fixture repo persists only the configuration a plain git init writes
fixtures:s/^: > "[$]_FW_DIR\/built"$/git -C "$CTL\/clean" config --unset core.filemode; &/	every fixture repo persists only the configuration a plain git init writes
fixtures:s/^: > "[$]_FW_DIR\/built"$/( mkdir -p "$CTL\/zz\/inner" \&\& cd "$CTL\/zz\/inner" \&\& git init -q . \&\& git config core.excludesFile \/nonexistent-k2 ); &/	every fixture repo persists only the configuration a plain git init writes
# THE TWO-PASS CENSUS's records (C9): every one is caught before any
# postcondition runs git, so no control runs, and each is fast (the FIFO
# forms end in seconds, because the census stops them, unlike a FIFO a
# postcondition would have read — docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-landing.md X16).
fixtures:s/^: > "[$]_FW_DIR\/built"$/( cp -R "$CTL\/clean\/.git\/objects" "$_FW_DIR\/zzobjcopy" \&\& rm -rf "$CTL\/clean\/.git\/objects" \&\& ln -s "$_FW_DIR\/zzobjcopy" "$CTL\/clean\/.git\/objects" ); &/	every fixture repo persists only the configuration a plain git init writes
fixtures:s/^: > "[$]_FW_DIR\/built"$/mkfifo "$CTL\/clean\/.git\/commondir"; &/	every fixture repo persists only the configuration a plain git init writes
harness:s/_psh="[$](find "[$]_pe"/_psh="$(false "$_pe"/	every fixture repo persists only the configuration a plain git init writes
fixtures:s/^: > "[$]_FW_DIR\/built"$/( git init -q "$CTL\/zzr" \&\& rm -f "$CTL\/zzr\/.git\/config" \&\& mkfifo "$CTL\/zzr\/.git\/config" \&\& printf '..\/..\/zzr\/.git\\n' > "$CTL\/clean\/.git\/commondir" ); &/	every fixture repo persists only the configuration a plain git init writes
fixtures:s/^: > "[$]_FW_DIR\/built"$/( printf x > "$_FW_DIR\/zzh" \&\& mkdir -p "$CTL\/clean\/.git\/info" \&\& rm -f "$CTL\/clean\/.git\/info\/exclude" \&\& ln "$_FW_DIR\/zzh" "$CTL\/clean\/.git\/info\/exclude" ); &/	every fixture repo persists only the configuration a plain git init writes
fixtures:s/^: > "[$]_FW_DIR\/built"$/( git init -q "$_FW_DIR\/zzo" \&\& rm -f "$_FW_DIR\/zzo\/.git\/config" \&\& mkfifo "$_FW_DIR\/zzo\/.git\/config" \&\& ln -s "$_FW_DIR\/zzo" "$CTL\/zzl" \&\& printf '..\/..\/zzl\/.git\\n' > "$CTL\/clean\/.git\/commondir" ); &/	every fixture repo persists only the configuration a plain git init writes
fixtures:s/^: > "[$]_FW_DIR\/built"$/( mkdir -p "$_FW_DIR\/pq\/a" "$_FW_DIR\/pq\/b" \&\& git init -q "$_FW_DIR\/pq\/a" \&\& git -C "$_FW_DIR\/pq\/a" config core.excludesFile \/nonexistent-k2 \&\& git init -q "$_FW_DIR\/pq\/b" \&\& git -C "$_FW_DIR\/pq\/b" config core.excludesFile \/nonexistent-k2 \&\& find "$CTL" -name .git -type d -print0 > "$_FW_DIR\/zzlist" \&\& while IFS= read -r -d "" _zd; do git config --file "$_zd\/config" core.excludesFile \/nonexistent-k2; done < "$_FW_DIR\/zzlist" ); &/	every fixture repo persists only the configuration a plain git init writes
fixtures:s/^: > "[$]_FW_DIR\/built"$/( mkdir -p "$_FW_DIR\/zzt\/a\/objects" \&\& : > "$_FW_DIR\/zzt\/a\/HEAD" \&\& ln -s "$_FW_DIR\/zzt" "$CTL\/zzy" \&\& ln -s \/dev\/null "$_FW_DIR\/prel" ); &/	every fixture repo persists only the configuration a plain git init writes
harness:/^  find "[$]CTL" .*-iname .git/s/2> "[$]_pq\/pre\.err" ||/2> "$_pq\/pre.err" \&\& false ||/	every fixture repo persists only the configuration a plain git init writes
harness:s/^    find "[$]CTL" -type l -print0 >/    false "$CTL" -type l -print0 >/	every fixture repo persists only the configuration a plain git init writes
harness:/^  find "[$]CTL" .*-iname .git/s/\(-iname \.git \)-print0\(.*-iname HEAD \)-print0\(.*-perm -u=rx \)-print0/\1-print\2-print\3-print/	every fixture repo persists only the configuration a plain git init writes
harness:/^    find "[$]CTL" -type l -print0/s/-print0/-print/	every fixture repo persists only the configuration a plain git init writes
harness:s/^    find "[$]CTL" -type l -print0 /    echo k2 >> "$_pq\/pre.err"; find "$CTL" -type l -print0 /	every fixture repo persists only the configuration a plain git init writes
# The NUL-list policy's tail check in `_pg_z` (the other readers' tails are pinned by the
# `-print0` records above and P-f's `K2X=1` record): a listing with an unterminated
# remainder reads as the same records, so only the tail check sees it.
harness:s/^    _fgit_in "[$]1" config --list --show-origin -z > "[$]2[.]raw" || return [$]?$/&; printf x >> "$2.raw"/	every fixture repo persists only the configuration a plain git init writes
# P-k's records (C10): the object-store check runs only over the clean
# census, on every `.git` P-g compares.
fixtures:s/^: > "[$]_FW_DIR\/built"$/( git init -q --bare "$_FW_DIR\/zzkstore" \&\& mkdir -p "$CTL\/clean\/.git\/objects\/info" \&\& printf '%s\\n' "$_FW_DIR\/zzkstore\/objects" > "$CTL\/clean\/.git\/objects\/info\/alternates" ); &/	no fixture repo reads objects from a store outside it
harness:s/"[$]_pfenv" -0 > "[$]_pq\/env0"/"$_pfenv" > "$_pq\/env0"/	the fixture build window's environment holds only its allowlist
harness:s/ GIT_CONFIG_NOSYSTEM=1 GIT_ATTR_NOSYSTEM=1 / GIT_CONFIG_NOSYSTEM=1 /	the fixture git has no system or global layer outside the void
harness:s/ "HOME=[$]_FGIT_VOID" GIT_CONFIG_NOSYSTEM/ "HOME=$SCRATCH" GIT_CONFIG_NOSYSTEM/	the fixture git has no system or global layer outside the void
# P-b's `git var` status is read by its GLOBAL check: a status of 1 on a call that succeeded reads as a failed GLOBAL call.
harness:s/_prc=0; _o="[$](_fgit_in/_prc=1; _o="$(_fgit_in/	the fixture git has no system or global layer outside the void
# P-b's machine limit is keyed on `git --version` (below 2.42, all four names), never on the shape of a refusal: on a capable git ANY non-0
# `git var` is red. Each record is one way the keyed arm goes green on a capable git; each SURVIVES with the version test forced to "old"
# (an arm case, not a record): a forced 129 / `var --` (git's usage, 129) / a misspelt name in the loop's list.
harness:s/ || _prc=[$][?]$/&; _prc=129/	the fixture git has no system or global layer outside the void
harness:s/_prc=0; _o="[$](_fgit_in "[$]_pq\/a" var "[$]_v" 2>/_prc=0; _o="$(_fgit_in "$_pq\/a" var -- "$_v" 2>/	the fixture git has no system or global layer outside the void
harness:s/ GIT_ATTR_GLOBAL; do$/ GIT_ATTR_GLOBL; do/	the fixture git has no system or global layer outside the void
fixtures:s/^: > "[$]_FW_DIR\/built"$/mkdir -p "$HOME\/.config\/git" \&\& : > "$HOME\/.config\/git\/ignore"; &/	nothing is written into the fixture git's void
fixtures:s/^: > "[$]_FW_DIR\/built"$/: > "$_FGIT_VOID\/k2plant"; GLOBIGNORE='*'; &/	nothing is written into the fixture git's void
harness:s/( cd "[$]_pq\/c" \&\& git init -q [.] )/false/	this git reports alternate object stores
harness:s/_fgit_in "[$]_pgd" count-objects -v 2>"[$]_pq\/pk.err"/false/	no fixture repo reads objects from a store outside it
# The ONE `alternate:` extraction (`_fgit_alternates`, shared by P-k and its liveness probe): broken, the liveness probe sees no line.
harness:s/alternate:[*]) _fgit_alt=/NOALT:*) _fgit_alt=/	this git reports alternate object stores
fixtures:s/^: > "[$]_FW_DIR\/built"$/mkdir -p "$CTL\/clean\/.git\/objects\/info" \&\& printf '\/nonexistent-k2-store\\n' > "$CTL\/clean\/.git\/objects\/info\/alternates"; &/	no fixture repo reads objects from a store outside it
mutgen:s/^_mut_regex_mutants() {$/& return 0/	the boundary-mutant generator derives a non-empty set from the wire's regexes
# THE WINDOW'S STDERR CANARY's record (C12): the fixtures file redirects fd 2
# before the options re-check's canary can reach it, so W ends the window
# incomplete rather than reading a diagnostic sent to /dev/null.
fixtures:s/^: > "[$]_FW_DIR\/built"$/exec 2>\/dev\/null; &/	the fixture build window completed
# W3's `./prelude.sh:` clause: an expansion error injected into
# `_fgit_postconditions` skips the rest of that top-level command in silence
# (neither `post_ok` nor `post_bad` is written, so the window is also untrusted
# under W's "never reached a verdict"), and only the prelude prefix names W3.
harness:s/^  _fpv=0$/&; : $(( 1\/0 ))/	the fixture build window ran without a shell diagnostic
# The shim builder without the line that takes the shim's own directory off the
# `PATH` it hands on (`: %s` keeps the printf's arguments aligned): the pin then
# passes `$_FGIT_BIN` on to a `git` that can find it again, and P-j's alias probe
# prints the `PATH` that git was given.
harness:s/PATH=[$]{PATH#%s:}/: %s/	the fixture build window runs the pinned git first on its PATH
# `_seal_apply`'s symlink walk, by its two halves; the standing self-check
# (`_seal_apply_probe`) is what kills them, at load.
harness:s/^      \[ ! -L "[$]_sa_p" \] || { _sa_link="[$]_sa_p"; break; }$/      :/	every mode restriction a fixture sealed was applied
harness:s/^    if \[ -n "[$]_sa_link" \]; then _seal_refuse .*$/    :/	every mode restriction a fixture sealed was applied
# `_seal` validates ALL its arguments. Each value below is one the shell would
# APPLY without complaint if it were let through (`chmod a-rwx` is `000`; a
# fixture name is only a label), so the record is killed by the refusal alone and
# not by an apply failure further down.
fixtures:s/^_seal "[$]CTL\/walk\/sub" 000 walk$/_seal "$CTL\/walk\/sub" a-rwx walk/	every mode restriction a fixture sealed was applied
fixtures:s/^_seal "[$]CTL\/walk\/sub" 000 walk$/_seal "$CTL\/walk\/sub" 000 wa.lk/	every mode restriction a fixture sealed was applied
# The err fixture's chain failing is NOT a machine that cannot enforce modes:
# the perm block is gated on its own probe (`_perm_ok`), so where `_perm_ok` is 1
# `_control err` reports the failed build, and there the err seal line, deleted, is red in the same control.
fixtures:s/^_seal "[$]CTL\/err\/control.py" 000 err$/_fixture_failed err; &/	an unreadable file fails closed
fixtures:/^_seal "[$]CTL\/err\/control.py" 000 err$/d	an unreadable file fails closed
# `_ctl_run`'s timeout clause reads a signal status as a timeout only beside the watchdog's marker
# (standing check, controls file): the marker test deleted, a signal at once is "killed after 30s".
ctlrun:s/^  if \[ "[$]_rc" -ge 128 \] \&\& \[ -e "[$]_r_wmark" \]; then$/  if [ "$_rc" -ge 128 ]; then/	only the watchdog's marker makes a signal status a timeout
# …and the check's row loop has no silent row: the hang row's arm renamed, its row is red (`*)`, "no arm").
controls:s/^             hang)$/             hng)/	only the watchdog's marker makes a signal status a timeout
fixtures:s/^: > "[$]_FW_DIR\/built"$/_fixture_failed relcwd; &/	a relative scratch dir is removed on exit
fixtures:s/^: > "[$]_FW_DIR\/built"$/_fixture_failed fsmon; &/	a caller's fsmonitor hook does not run
fixtures:s/^: > "[$]_FW_DIR\/built"$/_fixture_failed clean; &/	a restrictive umask decides nothing
# …and the canary scan failing (exit >= 2) is red, not "not found".
harness:s/END { exit !f }/END { exit 3 }/	the fixture build window ran without a shell diagnostic
# ROUND 2 of the code review (docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-corpus.md §6.1).
# The census lists a directory it cannot search: a BSD `find` says nothing about
# one, so a repository with a persisted key under a bare `chmod 0444` parent was
# invisible to it (and to P-g, which iterates the census's list).
fixtures:s/^: > "[$]_FW_DIR\/built"$/mkdir -p "$CTL\/zzm\/r" \&\& ( cd "$CTL\/zzm\/r" \&\& git init -q . \&\& git config core.excludesFile \/nonexistent-k2 ) \&\& chmod 0444 "$CTL\/zzm"; &/	every fixture repo persists only the configuration a plain git init writes
# Every label a part uses is defined: a definition in the controls file renamed,
# its producers left as they were.
controls:s/^_fws_lbl=/_fws_label=/	every label the parts use is defined
# A run ends green only through its verdict: the orderly-end flag at the PASSED line deleted.
s/^_K2_ORDERLY=1$/:/	the run ends only through a verdict
# Trust needs positive evidence: the postconditions' writer statement deleted, or
# writing a file nobody reads, leaves a complete window with neither marker.
harness:s/_fgit_postconditions \&\& : > "[$]_FW_DIR\/post_ok" || /_fgit_postconditions || /	the fixture build window completed
harness:s/ \&\& : > "[$]_FW_DIR\/post_ok" || / \&\& : > "$_FW_DIR\/post_okx" || /	the fixture build window completed
# P-f's presence direction: a name of the allowlist that the window's environment
# does not hold (exported away) is `missing`, not "nothing foreign".
fixtures:s/^: > "[$]_FW_DIR\/built"$/export -n LC_ALL; &/	the fixture build window's environment holds only its allowlist
# `_seal` refuses a path with a `..` component even when it resolves inside the
# fixture root (`walk/../walk/sub` is `walk/sub`, so the chmod would be applied).
fixtures:s/^_seal "[$]CTL\/walk\/sub" 000 walk$/_seal "$CTL\/walk\/..\/walk\/sub" 000 walk/	every mode restriction a fixture sealed was applied
# P-e's `!=` comparison: an override naming a REAL directory (the void) leaves
# `git --exec-path` non-empty, so only the comparison with the reference sees it.
harness:s/"LC_ALL=C")$/"LC_ALL=C" "GIT_EXEC_PATH=$_FGIT_VOID")/	no exec-path override reaches the fixture git
# P-k's liveness: an `alternates` that holds nothing the probe repo can read gives
# rc 0, no `alternate:` line, no stderr: only the empty-line clause's. A SECOND
# quoted store gives two `alternate:` lines: only the more-than-one-line clause's. The
# second path is C-quoted by the printf | sed form the line it extends uses, so a
# scratch path holding a backslash or a quote still reaches that clause, not the stderr one.
harness:s/> "[$]_pq\/c\/.git\/objects\/info\/alternates"$/> \/dev\/null/	this git reports alternate object stores
harness:s/^    printf '"%s"\\n' .* > "[$]_pq\/c\/.git\/objects\/info\/alternates"$/&; printf '"%s"\\n' "$(printf '%s' "$_pq\/b\/.git\/objects" | sed 's\/\\\\\/\\\\\\\\\/g; s\/"\/\\\\"\/g')" >> "$_pq\/c\/.git\/objects\/info\/alternates"/	this git reports alternate object stores
# The window's FIRST options check, alone: nounset dropped from the prelude, and
# the check after the fixtures file removed, so only the first can refuse.
harness:s/printf 'set -euo pipefail\\n'/printf 'set -eo pipefail\\n'/	/^    _fw_opts_on || { echo "the fixtures file switched off/d	the fixture build window completed
# Pass 1's NUL tail check: a duplicate record with no terminating NUL appended to
# the list after the `find` reads as the same records, so only the tail check sees it.
harness:s/^  find "[$]CTL" .*-iname .git.* || _pgpre=" (census failed)"$/&; printf %s "$CTL\/clean\/.git" >> "$_pq\/pre"/	every fixture repo persists only the configuration a plain git init writes
# The census (`$_cen_lbl`): the labels a run USED — the run registry (runtime)
# and the values of the `_*_lbl` names the wire or a part EXPANDS outside this table (the orphan check) or SPELLS, this table included (the ratchet)
# (`$_cen_used` and `$_lbd_names`, text derivations, asserted defined) — against the records, and the
# registry's sites against the call lines of the controls file (a bijection). One
# record per arm or reading, each killed by the census's own message:
#   the registry's writer in `_ctl_gate` (`ctlrun:`, range-scoped to it; the window
#   verdict, in the harness, holds the same line) → the registered labels are orphans;
#   a literal label at the window verdict, and at the fsmonitor `_ctl_run` → a
#   label no record names ("more"); a `_control` in a `( … )` after `_mut_run` →
#   the registry grew after the census (the controls file's last statement, `_mut_census_end`;
#   the subshell form, which an `exit` from the registry's writer could not red);
#   the variable half (the `$_cen_used` loop) emptied → the `_*_lbl`
#   needles are orphans; the expansion of a name deleted (the `_pkl_lbl` line) → its records are
#   orphans (a mention is no use); the recorded `forge` control deleted → its needle is an
#   orphan; the bare `ignored` control deleted → "fewer";
#   the bare `ignored` label on a second `_control` (before the census, after
#   every control) → "registered from more than one call site";
#   a second window verdict call on another controls line → the same message
#   (the site is the outermost controls-file frame: a walk that skipped the
#   harness frame but did not look for the controls one folded both calls onto
#   the verdict's own line);
#   a controls-file helper that makes the recorded `clean` control, called from
#   two lines → the same message (the site is the line that CALLED the helper,
#   not its body, where a nearest-controls-frame walk would put both);
#   a `_*_lbl` value (`$_mg_lbl`, whose producer is an `echo`) as a second
#   `_control`'s label → "a `_*_lbl` value passed to `_control`" (a `_control`
#   label is literal; the site rule cannot see a producer that is not a call).
ctlrun:/^_ctl_gate() {/,/^}/s/^  _ctl_reg "[$]1"$/  :/	every label the run used is named by a mutation record
controls:s/^_fgit_window_verdict_exit "[$]_fw_lbl"$/_fgit_window_verdict_exit "a window label spelled out"/	every label the run used is named by a mutation record
controls:s/^_mut_run$/&; ( _control "$CTL\/clean" 0 "PASSED" "a late label" ) || ctl_ok=1/	every label the run used is named by a mutation record
correspondence:s/^  for _cen_v in [$]_cen_used; do/  for _cen_v in; do/	every label the run used is named by a mutation record
controls:/^_control "[$]CTL\/forge" /d	every label the run used is named by a mutation record
controls:s/_ctl_run "[$]_fsm_lbl" /_ctl_run "a run label spelled out" /	every label the run used is named by a mutation record
controls:/^_control "[$]CTL\/ignored" /d	every label the run used is named by a mutation record
controls:s/^_mut_correspondence || ctl_ok=1$/_control "$CTL\/clean" 0 "PASSED" "an IGNORED generated artefact does not fire" || ctl_ok=1; &/	every label the run used is named by a mutation record
controls:s/^# STANDING CHECK of .*timeout clause, both ways\. A status >= 128 is a$/_fgit_window_verdict_exit "$_fw_lbl"/	every label the run used is named by a mutation record
controls:s/^_control "[$]CTL\/clean" 0 "PASSED" *"green is reachable" *|| ctl_ok=1$/_rh() { _control "$CTL\/clean" 0 "PASSED" "green is reachable"; }; _rh || ctl_ok=1/	s/^_fsm_root="[$]CTL\/fsmon"$/&; _rh || ctl_ok=1/	every label the run used is named by a mutation record
controls:s/^_control "[$]CTL\/clean" 0 "PASSED" *"green is reachable" *|| ctl_ok=1$/&; _control "$CTL\/clean" 0 "PASSED" "$_mg_lbl" || ctl_ok=1/	every label the run used is named by a mutation record
# The bijection of the registry's sites with the call lines of the controls file, and the registry's row shape:
# a name's one expansion deleted (the `_pkl_lbl` line: a mention is no use); relcwd's call behind `if false &&`
# and a `_control` behind a branch (a call line no site answers: on a machine with a FIFO the kill is the call line; on a no-FIFO declaring machine the branch runs and the kill is the ratchet, 19 against 18); each standing
# check's row loop emptied (its call line registers nothing); a two-expression record that folds a `_control`
# into a command substitution spanning two lines (a site that is not its text line); the registry's writer
# adding a fourth field (a row of the wrong shape); and the one expansion deleted with the name left in a trailing
# comment on another line (a comment is no use: the cut at a `#` that begins a word); and with the name left in a
# single-quoted span on another line (`: '$_pkl_lbl'`: a single-quoted span expands nothing, the strip after the cut).
harness:/^  if \[ -n "[$]_pkl" \]; then echo "!! CONTROL NOT EXERCISED ([$]_pkl_lbl)" >&2; _fpv=1; fi$/d	every label the run used is named by a mutation record
controls:s/^if ! _ctl_run "[$]_rel_lbl" /if false \&\& _ctl_run "$_rel_lbl" /	every label the run used is named by a mutation record
controls:s/^_cap_check || ctl_ok=1$/&; if [ "$_fifo_ok" -eq 0 ]; then _control "$CTL\/clean" 0 "PASSED" "zz new" || ctl_ok=1; fi/	every label the run used is named by a mutation record
controls:s/^         for _wm_row in abort hang; do$/         for _wm_row in; do/	every label the run used is named by a mutation record
controls:s/^         for _ev_row in ran stale; do$/         for _ev_row in; do/	every label the run used is named by a mutation record
controls:s/^         while read -r _ca_t _ca_zz _ca_yy _ca_trust _ca_chan _ca_want _ca_ran; do$/         while false; do/	every label the run used is named by a mutation record
controls:s/^\[ -z "[$]_fw_limits" \] || _limit_lines=.*\/')"$/&; _v1="$(_control "$CTL\/clean" 0 "PASSED" "green is reachable" || ctl_ok=1/	s/^_control "[$]CTL\/clean" 0 "PASSED" *"green is reachable" *|| ctl_ok=1$/)"; &/	every label the run used is named by a mutation record
ctlrun:s/^  printf '%s:%s\\t%s\\t%s\\n' /  printf '%s:%s\\t%s\\t%s\\tx\\n' /	every label the run used is named by a mutation record
harness:/^  if \[ -n "[$]_pkl" \]; then echo "!! CONTROL NOT EXERCISED ([$]_pkl_lbl)" >&2; _fpv=1; fi$/d	s/^    _pkl=bad$/& # $_pkl_lbl/	every label the run used is named by a mutation record
harness:/^  if \[ -n "[$]_pkl" \]; then echo "!! CONTROL NOT EXERCISED ([$]_pkl_lbl)" >&2; _fpv=1; fi$/d	s/^    _pkl=bad$/& ; : '$_pkl_lbl'/	every label the run used is named by a mutation record
harness:/^  if \[ -n "[$]_pkl" \]; then echo "!! CONTROL NOT EXERCISED ([$]_pkl_lbl)" >&2; _fpv=1; fi$/d	s/^    _pkl=bad$/& ; : \\\$_pkl_lbl/	every label the run used is named by a mutation record
# THE CAPABILITY TABLE (`$_cap_lbl`: the gate's capability arm, pinned by the `_cap` standing check in the
# controls file; `$_capt_lbl`: the table check; the reader and the gate's lookup are ctlrun's). Each `$_cap_lbl`
# record is killed by the `_cap` row named beside it, on every machine (the rows replace the table and the
# flags), the first red being: the lookup's equality made always-true, and made a substring match (row N);
# the decline arm (the decline rows `M 0 1 yes` and `Y 1 0`); the member's OWN group's flag (rows Y); the
# trust arm placed after the capability arm (the untrusted row); an undeclared group's red turned green (row
# U first; F1 and F2 red too); the `_cap` loop's `U` arm renamed (row U, "no arm"); a decline row made
# unparseable (that row, "did not parse": K6's shape); the `|| return $?` early return (row X, the malformed
# channel under a decline, comes back 2, not 1). One record is not killed by a row alone: the reader's
# flag-name validation made vacuous is killed by the verdict marker on bash 5.3 (the `${!x-}` abort ends
# `_cap`'s subshell) and by row F1 on bash 3.2.
ctlrun:s/^_cap_group_cb() { \[ "[$]{_cr_f\[2\]}" != "[$]_cg_l" \]/_cap_group_cb() { [ x != x ]/	the gate declines exactly the members of a group whose flag is 0
ctlrun:s/^_cap_group_cb() { \[ "[$]{_cr_f\[2\]}" != "[$]_cg_l" \]/_cap_group_cb() { [[ "${_cr_f[2]}" != *"$_cg_l"* ]]/	the gate declines exactly the members of a group whose flag is 0
ctlrun:s/^    \[ "[$]_cap_v" != 0 \] || return 2$/    [ "$_cap_v" != 0 ] || :/	the gate declines exactly the members of a group whose flag is 0
ctlrun:s/^_cap_flag_cb() { \[ "[$]{_cr_f\[1\]}" != "[$]_cf_g" \]/_cap_flag_cb() { [ x != x ]/	the gate declines exactly the members of a group whose flag is 0
ctlrun:s/^  if \[ "[$]_fw_trusted" -ne 1 \]; then$/  if false; then/	s/^  for _g_n in "[$]@"; do$/  [ "$_fw_trusted" -eq 1 ] || return 1; &/	the gate declines exactly the members of a group whose flag is 0
ctlrun:/^    if \[ -z "[$]_cap_f" \]; then$/,/^    fi$/s/^      return 1$/      return 0/	the gate declines exactly the members of a group whose flag is 0
controls:s/^             U:yes:-) case/             V:yes:-) case/	the gate declines exactly the members of a group whose flag is 0
controls:s/^M 0 1 yes - 2 -$/M 0 1 yes - 2 -x/	the gate declines exactly the members of a group whose flag is 0
ctlrun:s/^        \[0-9\]\*|\*\[!A-Za-z0-9_\]\*) _cr_bad=/        [0-9]x|x[!A-Za-z0-9_]x) _cr_bad=/	the gate declines exactly the members of a group whose flag is 0
ctlrun:s/ || _r_gs=[$]?$/ || return $?/	the gate declines exactly the members of a group whose flag is 0
# The table check's own reds, from the real table: a member row's group misspelt (the gate's red for the
# member's own label is the SECOND thing to fail, so the table check is what this needle names).
controls:s/^\(member.fif\)o\(.an unstorable entry neither hangs nor hides\)$/\1x\2/	the capability table is well formed and names every absence it is asked about
# The table check's other three clauses, each killed by it ALONE (each SURVIVES with its clause disabled):
# a probe's flag that holds 2, not 0 or 1 (the gate runs any value but 0, `_cap_lines` states the claim for
# any value but 0: only this clause reds the value); a member label listed twice (one label, one group: a
# member row's label changed to one another row lists); a group declared twice (a member row replaced by
# a copy of the `perm` group row, so the removed member simply runs here and nothing else reds).
harness:s/then _fifo_ok=1;/then _fifo_ok=2;/	the capability table is well formed and names every absence it is asked about
controls:s/^\(member.perm.\)an unsearchable dir fails closed$/\1an unreadable file fails closed/	the capability table is well formed and names every absence it is asked about
controls:/^group.perm._perm_ok/h;/^member.perm.an unresolvable self-test root decides nothing$/g	the capability table is well formed and names every absence it is asked about
# The three probes' flips (an absence nobody declared is `_cap_check`'s red, under the group's or the limit's own
# label): the FIFO probe and the permission probe answer 0 on a machine that has the capability, and P-f's definition
# test is forced to the limit branch on an `env` that accepts -0. Each is excused (SURVIVES, not counted) only where
# its caller declares the group or ID (WEBREF_WIRE_LACKS): the fifo and permission flips by the table's flag
# (`_mut_run`'s capability half), the P-f flip by the P-x half (`_pfl_lbl` is its limit label).
harness:s/then _fifo_ok=1;/then _fifo_ok=0;/	this machine holds a FIFO, or its caller declared that it cannot
harness:s/^_perm_ok=1$/_perm_ok=0/	this user cannot read a mode-000 file, or its caller declared that it can
harness:s/^  if \[ "[$]_pfrc" -ne 0 \] && \[ ! -s "[$]_pq\/env0" \] && \[ -s "[$]_pq\/env0.err" \] \\$/  if true \\/	this env lists its environment NUL-separated, or its caller declared that it cannot
# The verdict marker of each standing check: an abort inside the check's subshell before its verdict is
# a labelled red (`the check ended before its verdict`), not an unlabelled one.
controls:s/( _wm_bad=""$/( : "${_no_such_name_x}"; _wm_bad=""/	only the watchdog's marker makes a signal status a timeout
controls:s/^elif ! ( SELF="[$]_VFY\/[.]ev_run"/elif ! ( : "${_no_such_name_x}"; SELF="$_VFY\/.ev_run"/	a previous run's channel evidence never answers for the next
controls:s/^elif ! ( SELF="[$]_VFY\/[.]cap_run"/elif ! ( : "${_no_such_name_x}"; SELF="$_VFY\/.cap_run"/	the gate declines exactly the members of a group whose flag is 0
# The census's membership check (reader 4): a member row naming a label that is no `_control` of this run.
controls:s/^\(member.perm.an unreadable file fails\) closed$/\1 closeX/	every label the run used is named by a mutation record
# `_control`'s exit comparison: the `pin` control's expected exit and message swapped. `! [ … -eq … ]` reds the
# non-numeric "K2: a" ("expected exit K2: a, got 1"); a `-ne` would read `[`'s exit 2 as equal and pass.
controls:s/^_control "[$]CTL\/pin"   1 "K2: a"  /_control "$CTL\/pin"   "K2: a" 1  /	K2 fires on the path A-i removed
# THE CHANNEL EVIDENCE (`_ctl_run`: every channel a call delivers - env, umask, PATH prefix - is demanded back
# from the run itself: the first two by a recorder that runs between the delivery and the wire's first
# statement, the prefix by a mark its shims write when called). R2, R3 and R5 are KILLED by the evidence check and
# SURVIVE with that check disabled (`if [ -n "$_r_miss" ]` -> `if false`), R4 is killed by it too; R1 is killed by the control it
# makes reachable (the routed control), not by the evidence check, and survives with routed's channel
# arguments removed (S2a2R1surv): measured in the S2a-2 claim table, not enforced by the opt-in run.
# R1 - the routing purge disabled: killed by the routed control (an exported GIT_DIR redirects the scan).
s/\*) unset "[$]_v" ;; esac/*) : ;; esac/	exported GIT_DIR cannot redirect the scan
# R2 - GIT_DIR withheld between the delivery and the recorder: killed by the evidence check, not by the run.
ctlrun:s/K2_CTL_REACHED="[$]_r_reach" "[$]BASH" -c/K2_CTL_REACHED="$_r_reach" env -u GIT_DIR "$BASH" -c/	exported GIT_DIR cannot redirect the scan
# R5 - the umask withheld at the delivery AND the umask control's verdict flipped to the one it would then
# produce (both in `ctlrun`, where the verdict comparison lives): without the evidence check the control
# would pass, so only the umask clause of the check sees it.
ctlrun:s/^    \[ -z "[$]_r_umask" \] || umask "[$]_r_umask"$/    :/	s/^  \[ "[$]_cr" -eq 0 \] || return 1$/&; case "$4" in "a restrictive umask decides nothing") set -- "$1" 0 "PASSED" "$4" ;; esac/	a restrictive umask decides nothing
# R3 - the PATH prefix withheld at the delivery, for the cfgkept prefix ONLY (dropping every prefix reds lsfail
# and the rest for their own reasons, which is the wrong reason when the evidence check is what is under test):
# cfgkept still reads K2 through the real git and stays green, so only the `path` demand sees it - KILLED by the
# evidence check (`PATH prefix never reached the run`), and SURVIVES with that check disabled
# (`if [ -n "$_r_miss" ]` -> `if false`): measured in the S2b-2 claim table (S2b2R3surv), not enforced by the opt-in run.
ctlrun:s/^    export PATH="[$]{_r_pfx:+[$]_r_pfx:}[$]PATH"$/    case "$_r_pfx" in *\/fakegitcfg) _r_pfx= ;; esac; &/	the caller's git CONFIGURATION survives the routing purge
# R4 - #6, the wire's `_git` stepping past a first PATH entry ending `/fakegitcfg` (machine-independent, unlike an
# absolute git path): no `ls-files` reaches the shim, so no `path` mark - KILLED by the same evidence check. It
# SURVIVES in two directions: with that check disabled (`if [ -n "$_r_miss" ]` -> `if false`), and with cfgkept's
# mark moved back to the shim's entry, before the `case` (the bare `rev-parse --local-env-vars` at the wire's top
# still runs the shim): the latter is the F6 re-measure, S2b2R4surv.
s/^_git() { ( for _v in [$]_GIT_LOCAL_VARS; do$/_git() { ( case "${PATH%%:*}" in *\/fakegitcfg) PATH="${PATH#*:}" ;; esac; for _v in $_GIT_LOCAL_VARS; do/	the caller's git CONFIGURATION survives the routing purge
# The evidence directory made afresh (`rm -rf` then `mkdir`) weakened to `mkdir -p`: the first row of the `_ev`
# standing check leaves `path`, and its second row's prefix, which the run never calls, then reads as reached -
# KILLED by that check. With the check's rows removed it SURVIVES (no other control delivers a prefix the run
# skips after one that left `path`): measured in the CR-S2 claim table, not enforced by the opt-in run.
ctlrun:s/command rm -rf "[$]_r_reach" && mkdir "[$]_r_reach"/mkdir -p "$_r_reach"/	a previous run's channel evidence never answers for the next
s/^# Run from anywhere\./# Run from anywhere (edited by the negative control)./	!survive
# R7a-R7d - one verdict arm of `_mut_fresh` each disabled (stale, count, twice, carry): KILLED by the anchor rule's self-test
# rows for that verdict, inside the mutant; each SURVIVES only with all of them removed (R7d: every carry row, four).
correspondence:s/then _mf_v=stale; return 0; fi$/then :; fi/	a record's anchor is fresh only if it changes one line, once, and keeps what it does not read
correspondence:s/then _mf_v="count:[$]_mf_n"; return 0; fi$/then :; fi/	a record's anchor is fresh only if it changes one line, once, and keeps what it does not read
correspondence:s/then _mf_v=twice; return 0; fi$/then :; fi/	a record's anchor is fresh only if it changes one line, once, and keeps what it does not read
correspondence:s/ \[1-9\]\*) _mf_v=carry ;; / [1-9]*) ;; /	a record's anchor is fresh only if it changes one line, once, and keeps what it does not read
# R7e - `_mut_fresh_gate`'s red step dropped (`_mut_corr_bad=1`): KILLED by the self-test's red assertion; without it SURVIVES.
correspondence:s/^  _mut_corr_bad=1; return 1$/  return 1/	a record's anchor is fresh only if it changes one line, once, and keeps what it does not read
# R7f/R7g - the variant pass's exact path forced off (every form streamed in place) / its same-class probe dropped (\001 only):
# KILLED by the self-test's range row / `[a-z]` row respectively; with that row removed each SURVIVES.
correspondence:s/^    END { printf "%s %d/    END { x = 0; printf "%s %d/	a record's anchor is fresh only if it changes one line, once, and keeps what it does not read
correspondence:s/; q = "\\001" pr(substr([$]0, i, 1))$/; q = "\\001"/	a record's anchor is fresh only if it changes one line, once, and keeps what it does not read
# R7h - the variant pass's \001 probe dropped (same-class only): KILLED by the self-test's `x.ab` row; without it SURVIVES.
correspondence:s/; q = "\\001" pr(substr([$]0, i, 1))$/; q = pr(substr($0, i, 1))/	a record's anchor is fresh only if it changes one line, once, and keeps what it does not read
# R4a - a label defined and used only inside a double-quoted string holding an apostrophe pair (rule 1 counts nothing there): the ratchet reads
# the spelled names, so it counts the label bare, 19 against 18, KILLED by the census's message; with the orphan check's population it SURVIVES.
# The name is built from the anchor by backreference (`\1r4a\2`), so no new token is spelled here.
controls:s/^\(_pe\)\(_lbl\)="no exec-path override reaches the fixture git"$/&; \1r4a\2="cr2 r4a probe"; : "it's $\1r4a\2's"/	every label the run used is named by a mutation record
# R9 - the fsmonitor block's plain-git retry purges git's own list (`$_GIT_LOCAL_VARS`), not three hand-named variables: with the loop put back to
# three `unset`s and a caller's `GIT_OBJECT_DIRECTORY` exported (appended to the `_fsm_root` line), the retry cannot run the hook and the control reds, KILLED by its
# NOT EXERCISED message; with the export alone it SURVIVES (the purge is what passes). Both on the one block, so each expression is fresh alone.
controls:s/for _v in [$]_GIT_LOCAL_VARS; do unset "[$]_v"; done;/unset GIT_DIR; unset GIT_WORK_TREE; unset GIT_INDEX_FILE;/	s/^_fsm_root="[$]CTL\/fsmon"$/&; export GIT_OBJECT_DIRECTORY=\/nonexistent-k2/	a caller's fsmonitor hook does not run
MUTANTS
}
