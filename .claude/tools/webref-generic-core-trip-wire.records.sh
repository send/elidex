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
# needle, the `controls:` / `harness:` / `fixtures:` / `mutgen:` prefixes) is
# documented at the head of the mutation set, beside the readers that enforce
# it; the one rule kept here is the comment syntax, beside the one filter that
# drops it (below).
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
# which is inert. The always-on validator (in the mutation set) stays — a line that is neither a
# comment nor a record is still an error, not a skip.
# ⚠ FILTERED IN BASH, NOT THROUGH `grep -v`. The mutation set runs under `pipefail`:
# `grep` exits 1 when it selects nothing, so a set that happened to be all
# comments — or a grep on `PATH` that answers differently — would abort the
# required gate from inside the thing reporting it. Same trap as the `wc -l`
# and `awk` notes in the mutation set. Measured byte-identical to the `grep -v '^#'` pipe on
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
s#^K2RE_PATH=.*#K2RE_PATH='(^|/)\\.claude/(skills|tools)/[^/"]+/[^/]+'#	a quote inside a name segment
s/^K2RE_PATH=.*/K2RE_PATH="$K2RE"/	a STAGED symlink target is a stored path
s/elif \[ "$_mode" = 120000 \]; then/elif [ "$_mode" = 120000 ] \&\& [ "$_src" = index ]; then/	a COMMITTED symlink target is a stored path
s|${1//$'\\n'/$_REC_SEP}|${1}|	a NEWLINE inside a name segment
s/^  _stored "${rel#"$_dir"\/}"/  : /	an entry's own NAME is the hierarchy
s/--exclude-per-directory=.gitignore \\$/--exclude-standard \\$/	per-clone info/exclude cannot hide an entry
s/\[ "$_hrc" -eq 0 \]/false/	a COMMITTED violation fixed only in the index still fires
s/if ! tr -d .\\000. < "\$_b"/if false/	a NUL-bearing staged symlink blob is not a path
s/readlink -n "$f"/readlink "$f"/	readlink's own newline is not read as stored content
s/export GIT_NO_LAZY_FETCH=1 GIT_NO_REPLACE_OBJECTS=1/export GIT_NO_LAZY_FETCH=1/	a replace ref cannot substitute the staged blob
s|^_esc() .*|_esc() { printf '%s' "$1"; }|	a name cannot forge a verdict record
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
s/\[^A-Za-z0-9_.~@+%-\]/[^A-Za-z0-9_.~+%-]/	a PATH character before .claude is not a prose boundary
s/\[^A-Za-z0-9_.~@+%-\]/[[:space:]"]/	the leading boundary covers =, --opt=, :, a backtick, ** and ,
s/elif _ancestor_link "$rel"; then/elif false; then/	the worktree read does not traverse an ancestor symlink
s/\[ "$_catrc" -ne 0 \]/false/	a failed staged-blob read is not a clean target
s/)}>,;\]/]/g	closing punctuation is not a path segment
s/elif ! _git -C "$ROOT" --literal-pathspecs/elif false \&\& ! _git -C "$ROOT" --literal-pathspecs/	an inventoried path that vanished is not silently skipped
s/--literal-pathspecs ls-files --error-unmatch/ls-files --error-unmatch/	the vanished-path question is asked of a LITERAL path
s/\[ "$_hrc" -ne 0 \]/false/	a malformed HEAD ref is not an unborn repository
s|\[^/\[:space:\]\]+/(|[^/[:space:])}>,;]+/(|	punctuation BEFORE a slash is part of the path
s/(^|\/)\\.claude/\\.claude/	a segment merely ENDING in .claude is not the host path
s#^K2RE_PATH=.*#K2RE_PATH='\\.claude/(skills|tools)/[^/]+/[^/]+'#	a segment merely ENDING in .claude is not the host path
s/_SELFTEST="$2"/_SELFTEST="${WEBREF_WIRE_SELFTEST:-$2}"/	an exported SELFTEST cannot redirect the scan
s/\[ ! -d "${2:-}" \]/false/	a missing self-test root decides nothing
s/^SCRATCH="$(_phys "$_raw_scratch")"/SCRATCH="$_raw_scratch"/	a relative scratch dir is removed on exit
s/if \[ "$_ENDS" -ne 1 \] || /if false \&\& /	a walk killed mid-scan is not a verdict
s/\[ -x "$_ab_p" \] && return 0/return 0/	a tracked file under an unsearchable dir is not absent
s/|| \[ -L "$_ab_p" \] || continue/|| [ -L "$_ab_p" ] || return 1/	a tracked directory deleted wholesale stays green
s/\[ -d "$_ab_p" \] || return 0/[ -d "$_ab_p" ] || return 1/	a tracked directory replaced by a file stays green
s|\[^/\[:space:\]\]+/|[^]/[:space:]]+/|	a ] inside a first segment
s|\[^/\[:space:\]\]+/|[^/[:space:]"]+/|	a double quote inside a first segment
s|\[^/\[:space:\]\]+/|[^/[:space:]'"'"']+/|	a single quote inside a first segment
s|\[^/\[:space:\]\]+/|[^/[:space:]`]+/|	a backtick inside a first segment
s|\[^/\[:space:\]\]+/|[^]/[:space:]]+/|2	a ] inside an intermediate segment
s|\[^/\[:space:\]\]+/|[^/[:space:]"]+/|2	a double quote inside an intermediate segment
s|\[^/\[:space:\]\]+/|[^/[:space:]'"'"']+/|2	a single quote inside an intermediate segment
s|\[^/\[:space:\]\]+/|[^/[:space:]`]+/|2	a backtick inside an intermediate segment
s|\[^/\[:space:\]\]+/|[^/[:space:]"]+/|	a quoted one-segment reference before a slash token fails safe
s/SCRATCH="$(_phys "$_raw_scratch")" || SCRATCH=""/SCRATCH="$(_phys "$_raw_scratch")"/	a restrictive umask decides nothing
s/_root_p="$(_phys "$ROOT")" || _root_p=""/_root_p="$(_phys "$ROOT")"/	an unresolvable self-test root decides nothing
s/^unset GREP_OPTIONS$/:/	a caller's GREP_OPTIONS cannot hide a file
s/exec git -c core.fsmonitor=false /exec git /	a caller's fsmonitor hook does not run
s/^K2RE='(^|\[/K2RE='([/	a reference at the start of a line fires
s/\[^A-Za-z0-9_.~@+%-\]/[^A-Za-z0-9_.@+%-]/	a PATH character before .claude is not a prose boundary
s/\[^A-Za-z0-9_.~@+%-\]/[^A-Za-z0-9_.~@]/	a PATH character before .claude is not a prose boundary
s/\[^A-Za-z0-9_.~@+%-\]/[^A-Za-z0-9_~@+%-]/	a PATH character before .claude is not a prose boundary
s/\[^A-Za-z0-9_.~@+%-\]/[^A-Za-z0-9.~@+%-]/	a PATH character before .claude is not a prose boundary
s/\[^A-Za-z0-9_.~@+%-\]/[^A-Za-z_.~@+%-]/	a PATH character before .claude is not a prose boundary
s/\[^A-Za-z0-9_.~@+%-\]/[^a-z0-9_.~@+%-]/	a PATH character before .claude is not a prose boundary
s|\[^A-Za-z0-9_.~@+%-\]|[^A-Za-z0-9_.~@+%/-]|	a reference written after a slash fires
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
s#^K2RE_PATH='(^|/)\\.claude/(skills|tools)#K2RE_PATH='(^|/)\\.claude/(skills)#	a stored path naming .claude after a slash, under tools, fires
s#^K2RE_PATH='(^|/)\\.claude/(skills|tools)#K2RE_PATH='(^|/)\\.claude/[a-z]+#	a stored path that only looks like one stays green
s#^K2RE_PATH='\(.*\)/\[^/\]+/\[^/\]+'#K2RE_PATH='\1/[^/]*/[^/]+'#	a stored path that only looks like one stays green
s#^K2RE_PATH='(^|/)\\.claude#K2RE_PATH='(^|/).claude#	a stored path that only looks like one stays green
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
# `_ctl_gate`, asked by `_ctl_run` (every `_control` and the three non-`_control`
# blocks run through it) and once more by the fsmonitor block before its own plain-git
# preflight; the record pins that W2 is reported when the exits are gone. It SURVIVES
# with `_fw_done` removed from `_fgit_window`'s trust computation. `_ctl_run`'s ask is
# pinned by the relcwd and umask records (removing it, they survive). ⚠ A DECLARED GAP:
# the fsmonitor block's ask is pinned only together with `_ctl_run`'s (either alone is
# covered by the other); alone, it is pinned by the traced `w2rec` cell (companion §A.14).
harness:/^_fgit_window_verdict_exit()/,/^}/s/^    exit 2$/    :/	/^_fgit_window_verdict_exit()/,/^}/s/^  exit 1$/  :/	s/: > "[$]_FW_DIR\/done"'/: > "$_FW_DIR\/notdone"'/	no control runs over an incomplete or untrusted fixture build window
# C8's two records pin the SECOND clause of "complete and trusted": a red
# postcondition, or W3, must still stop the run before any control even with
# the untrusted-build exit itself removed. Each forces one of two of its three inputs
# (the third, `_fw_done`, is the W2 record's) that the trust computation (in `_fgit_window`) reads and removes that exit in the
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
fixtures:s/^: > "[$]_FW_DIR\/built"$/( git init -q --bare "$CTL\/zzlc" \&\& cd "$CTL\/zzlc" \&\& git config core.excludesFile \/nonexistent-k2 \&\& mv HEAD head ); &/	every fixture repo persists only the configuration a plain git init writes
fixtures:s/^: > "[$]_FW_DIR\/built"$/( mkdir -p "$CTL\/zzuc" \&\& cd "$CTL\/zzuc" \&\& git init -q --separate-git-dir="$_FW_DIR\/gduc" . \&\& mv .git .GIT ); &/	every fixture repo persists only the configuration a plain git init writes
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
harness:/^  find "[$]CTL" .*-iname .git/s/2> "[$]_pq\/pre.err" ||/2> "$_pq\/pre.err" \&\& false ||/	every fixture repo persists only the configuration a plain git init writes
harness:s/^    find "[$]CTL" -type l -print0 >/    false "$CTL" -type l -print0 >/	every fixture repo persists only the configuration a plain git init writes
harness:/^  find "[$]CTL" .*-iname .git/s/-print0/-print/g	every fixture repo persists only the configuration a plain git init writes
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
# P-b's ONE `git var` per name keeps its status beside its value: a status stored wrong reads as a failed GLOBAL call.
harness:s/printf -v "_pbr_[$]_v" '%s' "[$]_prc"/printf -v "_pbr_$_v" '%s' 1/	the fixture git has no system or global layer outside the void
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
# the perm block is gated on its own probe (`_perm_ok`), so `_control err` reports
# the failed build; and the err seal line, deleted, is red in the same control.
fixtures:s/^_seal "[$]CTL\/err\/control.py" 000 err$/_fixture_failed err; &/	an unreadable file fails closed
fixtures:/^_seal "[$]CTL\/err\/control.py" 000 err$/d	an unreadable file fails closed
# `_ctl_run` (commit `fd85e29f`) clears `_ctl_env` before its gate (standing check, controls file), and its ONE gate
# refuses a failed fixture among the root and the cwd of every run (the three non-`_control` blocks too).
harness:/^  _ctl_env=()$/d	a refused control hands its environment to no other control
# …and its timeout clause reads a signal status as a timeout only beside the watchdog's marker
# (standing check, controls file): the marker test deleted, a signal at once is "killed after 30s".
harness:s/^  if \[ "[$]_rc" -ge 128 \] \&\& \[ -e "[$]_r_wmark" \]; then$/  if [ "$_rc" -ge 128 ]; then/	only the watchdog's marker makes a signal status a timeout
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
harness:s/^    printf '"%s"\\n' .*> "[$]_pq\/c\/.git\/objects\/info\/alternates"$/&; printf '"%s"\\n' "$(printf '%s' "$_pq\/b\/.git\/objects" | sed 's\/\\\\\/\\\\\\\\\/g; s\/"\/\\\\"\/g')" >> "$_pq\/c\/.git\/objects\/info\/alternates"/	this git reports alternate object stores
# The window's FIRST options check, alone: nounset dropped from the prelude, and
# the check after the fixtures file removed, so only the first can refuse.
harness:s/printf 'set -euo pipefail\\n'/printf 'set -eo pipefail\\n'/	/^    _fw_opts_on || { echo "the fixtures file switched off/d	the fixture build window completed
# Pass 1's NUL tail check: a duplicate record with no terminating NUL appended to
# the list after the `find` reads as the same records, so only the tail check sees it.
harness:s/^  find "[$]CTL" .*-iname .git.*$/&; printf %s "$CTL\/clean\/.git" >> "$_pq\/pre"/	every fixture repo persists only the configuration a plain git init writes
s/^# Run from anywhere\./# Run from anywhere (edited by the negative control)./	!survive
MUTANTS
}
