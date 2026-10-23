# K2 fixture git — PR #527 CR2 round: the sweep and the instruments (as run)

Companion of `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-cr2-plan.md` ("the plan") and its companion `…-cr2-plan-exitpath.md` (the plan's §6,
written there as x§6.n). Created by CR2-M (the plan's §11, row CR2-M), on the CR round's precedent: a citation in the
repo must resolve in the repo, so the instruments the plan's claim, guard and blind-spot rows cite are kept here as
run (`…-cr-plan.md` §7, "The move", that round's R3 H1; `…-provenance.md` §E is the older precedent). **Frame
`706049ad`**, as the plan.

**How a name in the plan resolves.** Every `cr2/bin/<name>` the two plan files cite is the block `§E.<name>`, here or in the
instrument companion the §E index below names, verbatim (the list is the plan's §14, "Instruments"; each block's sha256 prefix is of the file as run, and the frozen
texts' prefixes equal those the plan companion's header, `…-cr2-plan-exitpath.md`, names in its Texts line). The logs (`cr2/logs/…`) and the clones (`cr2/work/…`) were
measured outside the repo and stay there; the plan states their figures. The scripts name the handoff directory by
its absolute path (`H=…/elidex-fgit-handoff/cr2`): to re-run one, give it a directory laid out as that one — the
blocks under `bin/`, an empty `work/` and `logs/`, the inputs of §E.4 — beside a `git clone --local` of the frame
checked out at `706049ad` and named `cr2-wt` (the `*-clone.sh` scripts clone it).

**The sweep** (the plan's §11): every slice re-runs `§E.sweep.sh` with `§E.idents.tsv` beside it. They are CR2's
copy of the CR round's (`…-cr-sweep.md` §E, which stays as run): the same file set, with CR2's own plan, its companion,
this file and its instrument companions (every `…-cr2-*.md`) excluded instead of that round's two files; CR2-0 adds the new part to the set and CR2-D the #519
memo's companion; each slice adds the identifier groups it introduces, in the commit that adds them. `idents.tsv` is
the CR round's as committed at `706049ad`, with each slice's groups appended after its last row (CR2-0: `correspondence`; CR2-B: `_mut_fresh` (word-bounded, so that its fix round's `_mut_fresh_gate` is a group of its own), `_mut_fresh_gate`, `_mf_v`, `_af_lbl` and the phrases its B2a I-2 groups re-classify, "fresh", "exactly one line" and "anchor"; CR2-A: `.mvalues` and the phrases of its B2a I-2 groups, "expand" and "spell" — its other two, `.vlabels` and `$_cen_used`, are rows already, `lbd` and `cen_vars`; CR2-C1: its B2a I-2 groups, `_rc`/`_out` (CR14-G's reader expression, `rc_out`), `.control_out` with the variable naming it, `_out_f`, the timeout marker removed beside it, `.control_timeout` and `_r_wmark`, and "RETURN CONTRACT"; CR2-D: the companion's name, `review_rounds`, and its B2a I-2 groups, "plan memo §1" (`plan_memo_s1x`), "parent … §1" (`parent_s1x`, instrument (2)'s first form) and "parent D" (`parent_d`); R29-D: `_k2_kill_gone`, `_k2_left`, `_k2_name` and the name line, "could not confirm" (`k2_name_line`); its plan delta `…-cr2-plan-liveness.md` matches the glob and the `cr2-*` exclusion, as its siblings do). The `perl -0777` wrap backstop is run as the CR round ran it
(`…-cr-plan.md` §7; `…-impl-cr.md` §S.2).

## §0 Spec coverage map

**No spec surface** — as the plan's §0: this file holds a shell trip-wire's instruments, as run. `preflight.py`
exits 1 here by design, because no table follows this heading.

## §E Appendix — the instruments (as run)

One fenced block per file, under its file name. A slice copies each block to a scratch directory under that name and
runs it from there (no extractor: the copy is the block).

### §E index — where each block is

§E was split over four files at CR2-M (touch-time, the 1000-line rule: one file held 2725 lines). A `§E.<name>`
cite resolves in the file this table names, under the heading `#### §E.<name>` (`### §E.<name>` for the sweep's
two files); each block except `§E.sweep.sh` (its exclusion widened to `…-cr2-*` at the split, the plan's §11; its file set widened by the
new part at CR2-0 and by the #519 memo's companion at CR2-D) and `§E.idents.tsv` (the groups each slice appends, from CR2-0) is byte-identical to the one it was in this file before the split. The three companions open with the same
no-spec-surface §0 and are excluded from CR2's sweep and from the plan's claim rows over `docs/plans/*k2-fgit*`, as this file is (the plan's §10 and §11).

| part | file | blocks |
|---|---|---|
| §E.sweep.sh, §E.idents.tsv | this file | `sweep.sh`, `idents.tsv` |
| §E.1 The plan's §2–§5 | this file | `oracle2.sh`, `pops.sh`, `scanV.awk`, `cr4-edit.py`, `cr4-over.py`, `cr4-parse.sh`, `cr4-proto.py`, `r4a-rec.py`, `fresh.py`, `fresh.sh`, `fresh-countonly.sh`, `cr7-edit.py`, `cr7-reanchor.py`, `f13drift.py`, `f13span.sh`, `cr9-proto.py`, `cr14-edit.py` |
| §E.2 The exit path: probes, signal and fd measurements, single-record runs | `2026-09-citation-hygiene-k2-fgit-hermetic-cr2-instruments-probes.md` | `cr11.sh`, `cr12.sh`, `mkfx-clean.sh`, `trapprobe.sh`, `trapprobe2.sh`, `trapprobe3.sh`, `dispprobe.sh`, `sigsite.sh`, `sigsite2.sh`, `sigsite3.sh`, `f2probe.sh`, `f2probe3.sh`, `f3probe.sh`, `f3fdleak.sh`, `f3fdleak3.sh`, `f3hang.sh`, `g12probe.sh`, `r2-optin.sh`, `r2-runsig.sh`, `r3-optin.sh`, `r3-runsig.sh`, `r3-tdrun.sh`, `r3-trialsig.sh`, `run.sh` |
| §E.3 (1 of 2): [r2], [r3], [r3f] | `2026-09-citation-hygiene-k2-fgit-hermetic-cr2-instruments-texts-r2-r3.md` | `r2c2-proto.py`, `r2c2-proto.fd8-rejected.py`, `r3c2-proto.py`, `r3c2-fix1.py`, `r3c2-fix2.py`, `r3-instr.py`, `r3-clone.sh`, `r3-batch.sh`, `r3-batch2.sh`, `r3f-batch.sh` |
| §E.3 (2 of 2): [r4p], [r5p] | `2026-09-citation-hygiene-k2-fgit-hermetic-cr2-instruments-texts-r4-r5.md` | `r4c2-fix3.py`, `r4-clone.sh`, `r4-batch.sh`, `r4probe.sh`, `r4race.sh`, `r4race-batch.sh`, `r5c2-fix4.py`, `r5-clone.sh`, `r5-batch.sh`, `r5probe.sh` |
| §E.4 The inputs | this file | `rm`, `ps` |

### §E.sweep.sh

`sweep.sh`

```bash
#!/bin/bash
# sweep.sh <clone> : CR2's copy of the CR round's G1 sweep (`…-cr-sweep.md` §E.sweep.sh, which stays as run).
# For each identifier row of idents.tsv (key, ERE, flags), /usr/bin/grep -rnE over the file set of the clone;
# prints key<TAB>file:line<TAB>text.
set -u
c="$1"; here="$(cd "$(dirname "$0")" && pwd)"
cd "$c" || exit 2
files=( .claude/tools/webref-generic-core-trip-wire.sh .claude/tools/webref-generic-core-trip-wire.{controls,ctlrun,fixtures,harness,mutations,correspondence,mutgen,records}.sh
        docs/plans/*k2-fgit* docs/plans/2026-07-citation-hygiene-umbrella.md docs/plans/2026-09-citation-hygiene-Ai-wire-k2-trip-wire.md
        docs/plans/2026-09-citation-hygiene-Ai-wire-k2-trip-wire-review-rounds.md
        CLAUDE.md .github/workflows/ci.yml scripts/* )
# CR2-0 added the new part (`correspondence`) to the list above; CR2-D adds the #519 memo's companion (the file its split commit creates; the plan's §11).
# Excluded (the plan's §11, on the CR round's R3 H1 precedent): CR2's plan, its companion, its plan delta (`…-cr2-plan-liveness.md`, R29-D), this file and its
# instrument companions (every `…-cr2-*.md`; `…-impl-cr2.md` and `…-impl-cr2-d.md` stay in), which match the glob above; they are the
# ledger's own statement of the identifiers, not sites. The CR round's plan and
# ledger are in the set: that round is closed, and their hits are provenance (`hist`, the plan's §11).
keep=(); for f in "${files[@]}"; do case "$f" in docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-cr2-*) ;; *) keep+=( "$f" ) ;; esac; done; files=( "${keep[@]}" )
echo "### head=$(git rev-parse --short HEAD) files=${#files[@]}"
printf '### file: %s\n' "${files[@]}"
while IFS=$'\t' read -r key re fl; do
  opt=-nE; [ "$fl" = i ] && opt=-niE
  /usr/bin/grep -H $opt -- "$re" "${files[@]}" | while IFS= read -r h; do
    f="${h%%:*}"; r="${h#*:}"; l="${r%%:*}"; t="${r#*:}"
    printf '%s\t%s:%s\t%s\n' "$key" "$f" "$l" "$t"
  done
done < "$here/idents.tsv"
```

### §E.idents.tsv

`idents.tsv`

```text
ctl_need	_ctl_need	
declined	\.declined	
cn	(^|[^A-Za-z0-9_])_cn([^A-Za-z0-9]|$)	
cndir	\$_VFY/\.cn([^A-Za-z0-9]|$)	
gopt_ok	_gopt_ok	
k2_line	_k2_line	
goptprobe	goptprobe	
fifo_ok	_fifo_ok	
perm_ok	_perm_ok	
fifo_line	_fifo_line	
perm_line	_perm_line	
machine_limits	machine_limits	
fw_limits	_fw_limits	
mskip	_mskip	
mut_trial	_mut_trial|_mrc2	
notexh	NOT EXERCISED on this machine	
cap_table	_CAP_TABLE|_cap_[a-z]|_capt?_lbl|_fifo_lbl|_perm_lbl|_pfl_lbl|WEBREF_WIRE_LACKS|goptgrep	
callsite	call site|outermost	i
standing	standing check	i
cen_lbl	_cen_lbl	
lbd	(^|[^A-Za-z0-9_])_lbd([^A-Za-z0-9]|$)|\.vlabels	
controls_v	_CONTROLS	
registry	\.registry|_ctl_reg|\.ctllabels|_mut_census_end	
ratchets	_MUT_RECORDS_MIN|_MUT_UNRECORDED_MAX|_MUT_TARGETS	
wrongreason	wrong reason	i
grepopt	GREP_OPTIONS	
family	impl-cr	
lblconv	_\*_lbl|_lbl="…"	
slot	#11-trip-wire-launch-environment	
x9	X9|route \(b\)	
resid_range	"?R1"?–"?R[0-9]+"?	
cap	(^|[^A-Za-z0-9_])_cap([^A-Za-z0-9_]|$)	
cap_fn	_cap_rows|_cap_group_of|_cap_flag_of	
cap_err	\.cap_err	
cr_vars	(^|[^A-Za-z0-9_])_cr_[a-z]	
capflags	(^|[^A-Za-z0-9_])_(zz|yy)_ok([^A-Za-z0-9_]|$)	
endmarks	\.(wm|ev|cap)_end([^A-Za-z0-9_]|$)	
lacks	WEBREF_WIRE_LACKS	
cap_chk	_cap_chk_	
cc_vars	(^|[^A-Za-z0-9_])_cc_[a-z]	
mut_skip	_mut_skip_glabel|_mut_wrong_reason|(^|[^A-Za-z0-9_])_mk_g([^A-Za-z0-9_]|$)	
cen_vars	(^|[^A-Za-z0-9_])_cen_[a-z]	
part_path	_mut_part_path|(^|[^A-Za-z0-9_])_mut_pp(_pre|_suf|_ext)?([^A-Za-z0-9_]|$)|(^|[^A-Za-z0-9_])_mut_(cp|own_pre|own_suf)([^A-Za-z0-9_]|$)|(^|[^A-Za-z0-9_])_mt_p([^A-Za-z0-9_]|$)	
target_prose	hand list|hand-list|file operand|wfile|unlisted prefix|prefix that is not listed	
correspondence	correspondence	i
mut_fresh	_mut_fresh([^A-Za-z0-9_]|$)	
mut_fresh_gate	_mut_fresh_gate	
mf_v	_mf_v	
af_lbl	_af_lbl	
fresh	fresh	i
oneline	exactly one line	i
anchor	anchor	i
mvalues	\.mvalues	
expand	expand	i
spell	spell	i
rc_out	\$\{?_(rc|out)([^A-Za-z0-9_]|$)|unset _(rc|out)	
control_out	\.control_out|_out_f([^A-Za-z0-9_]|$)	
control_timeout	\.control_timeout|_r_wmark	
retcontract	RETURN CONTRACT	i
review_rounds	Ai-wire-k2-trip-wire-review-rounds	
plan_memo_s1x	plan memo §1[01]	i
parent_s1x	[Pp]arent('s)?[^|.]{0,40}§1[01]	
parent_d	[Pp]arent D[0-9]+	
k2_reap_d	_K2_REAP_D	
k2_reap	_k2_reap([^_A-Za-z0-9]|$)	
k2_reap_add	_k2_reap_add	
k2_sig	_k2_sig	
k2_stop_text	_K2_STOP	
k2_stop	_k2_stop	
k2_end	_k2_end	
k2_age	_k2_age	
k2_traps	_k2_traps	
ctl_left	_ctl_left	
ctl_wd_pg	_ctl_wd_pg	
og_trap	_og_trap	
og_lbl	_og_lbl	
k2_orderly	_K2_ORDERLY	
exit_trap	exit trap	i
outlived	outlived	i
watchdog	watchdog	i
killed_when	killed when the child returns	i
sigkill	SIGKILL	
reap_entry	reap (set|entr)	i
three_standing	three standing	i
other_end	every other end	i
sig_exit	128\+n|stopped by SIG	
k2_kill_gone	_k2_kill_gone	
k2_left	_k2_left	
k2_name	_k2_name	
k2_name_line	could not confirm	i
```

### §E.1 The plan's §2–§5 (CR#4, CR#7, CR#9, CR#14)

#### §E.oracle2.sh

`oracle2.sh` (sha256 `ef58d7562308545a`, 135 lines)

```bash
#!/opt/homebrew/bin/bash
# oracle: bash's truth vs the shipped rule-1 cut. Run: bash oracle.sh <mutations.sh> <outdir>
export LC_ALL=C; M="$1"; O="$2"; mkdir -p "$O"
# mechanical extraction of the shipped awk program: between "awk '" and the closing "}' " before "${_cen_files"
awk '/_cen_used="\$\(awk '"'"'/{f=1; sub(/.*awk '"'"'/,"")} f{ if ($0 ~ /\}'"'"' "\$\{_cen_files/){ sub(/'"'"' "\$\{_cen_files.*/,""); print; exit } print }' "$M" > "$O/shipped.awk"
[ -s "$O/shipped.awk" ] || { echo "extract failed"; exit 2; }
TAB=$'\t'; NL=$'\n'
L=(); N=()
add(){ N+=("$1"); L+=("$2"); }
for sep in 'SP: ' "TAB:$TAB" 'SC:;' 'AMP:&' 'AND:&&' 'PIPE:|' 'OR:||' 'LP:(' 'RP:)' 'LT:<' 'GT:>' 'BOL:'; do
  nm=${sep%%:*}; c=${sep#*:}
  case $nm in
   SP) add "$nm# nosp" 'true #$_q_lbl'; add "$nm# sp" 'true # $_q_lbl';;
   TAB) add "$nm# nosp" "true ${TAB}#\$_q_lbl"; add "$nm# sp" "true ${TAB}# \$_q_lbl";;
   BOL) add "$nm# nosp" '#$_q_lbl'; add "$nm# sp" '# $_q_lbl';;
   LP) add "$nm# nosp" '(#$_q_lbl'"${NL}true)"; add "$nm# sp" '(# $_q_lbl'"${NL}true)";;
   AND|PIPE|OR) add "$nm# nosp" "true $c#\$_q_lbl${NL}true"; add "$nm# sp" "true $c# \$_q_lbl${NL}true";;
   RP) add "$nm# nosp" '(true)#$_q_lbl'; add "$nm# sp" '(true)# $_q_lbl';;
   LT) add "$nm# nosp" 'true <#$_q_lbl'; add "$nm# sp" 'true <# $_q_lbl';;
   GT) add "$nm# nosp" 'true >#$_q_lbl'; add "$nm# sp" 'true ># $_q_lbl';;
   *) add "$nm# nosp" "true $c#\$_q_lbl"; add "$nm# sp" "true $c# \$_q_lbl";;
  esac
done
add 'cmd;# sp' 'true;# $_q_lbl'
add 'cmd &# sp' 'true &# $_q_lbl'
add '$# then lbl' 'echo $# $_q_lbl'
add '${x#y} lbl' 'x=ab; echo ${x#a} $_q_lbl'
add '${x##*/} lbl' 'x=a/b; echo ${x##*/} $_q_lbl'
add '\# lbl' 'echo \# $_q_lbl'
add 'a#b lbl' 'echo a#b $_q_lbl'
add 'dq "a #b $_q_lbl"' 'echo "a #b $_q_lbl"'
add "sq 'a #b' \$_q_lbl" "echo 'a #b' \$_q_lbl"
add 'dq then # lbl' 'echo "x" # $_q_lbl'
add 'awk sq # then lbl' "echo '{ # x }' \"\$_q_lbl\""
add 'heredoc body " # lbl"' "cat <<EOF >/dev/null${NL} # \$_q_lbl${NL}EOF"
add 'heredoc body "x # lbl"' "cat <<EOF >/dev/null${NL}x # \$_q_lbl${NL}EOF"
add '$( # lbl NL )' 'echo $(# $_q_lbl'"${NL})"
add 'expansion before #' 'echo $_q_lbl # c'
add 'braced ${_q_lbl}' 'echo ${_q_lbl} # c'
add 'braced after #-sp' 'echo # ${_q_lbl}'

# ---- part B: contexts x forms, truth = output carries the marker 77 (_q_lbl=77), so ${..:-x} and $((..)) count
M=()   # method per case: u = set -u, m = marker
for i in "${!N[@]}"; do M+=(u); done
addm(){ N+=("$1"); L+=("$2"); M+=(m); }
FORMS=('$_q_lbl' '${_q_lbl}' '${_q_lbl:-x}')
for f in "${FORMS[@]}"; do
  addm "unq $f"            "echo $f"
  addm "sq $f"             "echo '$f'"
  addm "dq $f"             "echo \"$f\""
  addm "ansic \$'' $f"     "echo \$'$f'"
  addm "esc \\ $f"         "echo \\$f"
  addm "dq-esc \"\\\" $f"  "echo \"\\$f\""
  addm "cmdsub $f"         "echo \$(echo $f)"
  addm "backtick $f"       "echo \`echo $f\`"
  addm "arith $f"          "echo \$(($f*1))"
  addm "heredoc-unq $f"    "cat <<EOF${NL}$f${NL}EOF"
  addm "heredoc-q '' $f"   "cat <<'EOF'${NL}$f${NL}EOF"
  addm "heredoc-dq \"\" $f" "cat <<\"EOF\"${NL}$f${NL}EOF"
  addm "heredoc-bs \\ $f"  "cat <<\\EOF${NL}$f${NL}EOF"
  addm "heredoc-q-then-code $f" "cat <<'EOF'${NL}x${NL}EOF${NL}echo $f"
  addm "heredoc-q <<- tab $f" "cat <<-'EOF'${NL}${TAB}x${NL}${TAB}EOF${NL}echo $f"
  addm "herestring-sq $f"  "cat <<<'a' ; echo $f"
  addm "herestring-then-hd $f" "cat <<<'EOF'${NL}echo $f"
  addm "sq-hash then real $f" "echo 'a #b' $f"
  addm "sq-hash-only then real $f" "echo '#' $f"
  addm "dq-apos then real $f" "echo \"it's\" $f"
  addm "dq-apos in-expansion $f" "echo \"it's $f\" 'z'"
  addm "sq span between $f"  "echo 'a'$f'b'"
  addm "sq-bs then real $f"  "echo 'a\\' $f"
  addm "esc-bs then real $f" "echo \\\\$f"
  addm "ansic-esc-quote $f"  "echo \$'a\\'' $f"
  addm "ansic-esc-quote-in $f" "echo \$'\\'$f'"
  addm "comment after $f"    "echo $f # c"
  addm "mention in comment $f" "echo 1 # $f"
  addm "esc-then-name \$\\_ $f" "echo \$\\_q_lbl"
done
addm "arith bare name"    'echo $((_q_lbl*1))'
addm "arith bare name esc-free" 'echo $(( _q_lbl + 0 ))'
addm "escaped brace \${\\_"  'echo $\_q_lbl'
# ---- part C (CR2): GENERATED double-quote contexts. Each DQ string carries a prefix and a suffix drawn from the
# characters rule 1's steps act on (`'`, ` #`, `\"`, `$'`) around a form; then a second family places the form in
# SINGLE-QUOTED data after a DQ string holding the same characters (over-count probes), and a quoted here-document
# after a DQ string holding ` #` (the operator the cut can take).
DQP=('' "it's " ' #x ' "'a' " '\" ' "a'b'c " 'a #b' "x \$'y' ")
DQS=('' " isn't" ' #y' "'" ' \"' "'z' #")
for f in "${FORMS[@]}"; do
  for p in "${DQP[@]}"; do for s in "${DQS[@]}"; do
    addm "dq[$p|$s] $f" "echo \"$p$f$s\""
  done; done
  for p in "${DQP[@]}"; do
    addm "dq[$p] then sq-data $f" "echo \"$p\"; echo '$f'"
    addm "dq[$p] then sq-data same line, 2 spans $f" "echo \"$p\" 'a' '$f'"
    addm "dq[$p] then esc-data $f" "echo \"$p\"; echo \\$f"
    addm "dq[$p] then q-heredoc $f" ": \"$p\"; cat <<'EOF'${NL}$f${NL}EOF"
    addm "dq[$p] before q-heredoc op $f" "echo \"$p\" <<'EOF'${NL}$f${NL}EOF"
  done
done
printf '%-40s %-8s %-8s %-5s %s\n' CASE bash53 bash32 cut CLASS
declare -A cnt
for i in "${!N[@]}"; do
  line=${L[$i]}
  # cut verdict
  cut=COMMENT; [ -n "$(printf '%s\n' "$line" | awk -f "$O/shipped.awk")" ] && cut=CODE
  # NB the awk is line-based; the here-doc cases are fed whole, line by line, as the scan does
  res=()
  for sh in /opt/homebrew/bin/bash /bin/bash; do
    if [ "${M[$i]}" = m ]; then
      out=$("$sh" -c "_q_lbl=77; x=1${NL}${line}" 2>"$O/err.$$"); rc=$?; err=$(cat "$O/err.$$")
      if printf '%s' "$err" | /usr/bin/grep -qiE 'syntax error|unexpected|unterminated|here-document'; then v=SYNTAX
      elif [ $rc -ne 0 ]; then v="RC$rc"
      elif printf '%s' "$out" | /usr/bin/grep -q 77; then v=CODE
      else v=COMMENT; fi
    else
    err=$("$sh" -c "set -u${NL}${line}" 2>&1 >/dev/null); rc=$?
    if printf '%s' "$err" | /usr/bin/grep -q 'unbound variable'; then v=CODE
    elif printf '%s' "$err" | /usr/bin/grep -qiE 'syntax error|unexpected|unterminated|here-document'; then v=SYNTAX
    elif [ $rc -ne 0 ]; then v="RC$rc"
    else v=COMMENT; fi
    fi
    res+=("$v")
  done
  cls=AGREE; for v in "${res[@]}"; do
    case $v in SYNTAX|RC*) cls="SYNTAX/EXCL($v)";; esac; done
  if [ "$cls" = AGREE ]; then
    for v in "${res[@]}"; do
      if [ "$v" = CODE ] && [ "$cut" = COMMENT ]; then cls=FALSE-RED
      elif [ "$v" = COMMENT ] && [ "$cut" = CODE ]; then cls=FALSE-GREEN; fi
    done
  fi
  cnt[$cls]=$(( ${cnt[$cls]:-0} + 1 ))
  printf '%-40s %-8s %-8s %-5s %s\n' "${N[$i]}" "${res[0]}" "${res[1]}" "$cut" "$cls"
done
echo "--- counts:"; for k in "${!cnt[@]}"; do echo "$k ${cnt[$k]}"; done | sort
true
```

#### §E.pops.sh

`pops.sh` (sha256 `f5739d97d9945d66`, 15 lines)

```bash
#!/opt/homebrew/bin/bash
# pops.sh <tools dir> <outdir>: the census's two candidate populations at a tree, without running the wire.
# expansions = the shipped rule-1 awk over the wire + parts minus records (extracted mechanically, as crc1-fix3/oracle.sh does);
# mentions = `_lbd`'s grep over the wire + every part (records included).
set -eu; export LC_ALL=C
T="$1"; O="$2"; mkdir -p "$O"; W="$T/webref-generic-core-trip-wire"
M="$W.mutations.sh"; [ -f "$W.census.sh" ] && M="$W.census.sh"
awk '/_cen_used="\$\(awk '"'"'/{f=1; sub(/.*awk '"'"'/,"")} f{ if ($0 ~ /\}'"'"' "\$\{_cen_files/){ sub(/'"'"' "\$\{_cen_files.*/,""); print; exit } print }' "$M" > "$O/shipped.awk"
[ -s "$O/shipped.awk" ] || { echo "extract failed"; exit 2; }
files=("$W.sh"); for p in controls harness ctlrun mutations census fixtures mutgen; do [ -f "$W.$p.sh" ] && files+=("$W.$p.sh"); done
awk -f "$O/shipped.awk" "${files[@]}" | sort -u > "$O/expanded.txt"
/usr/bin/grep -ho '_[A-Za-z0-9_]*_lbl' "${files[@]}" "$W.records.sh" | sort -u > "$O/mentioned.txt"
echo "expanded $(wc -l < "$O/expanded.txt")  mentioned $(wc -l < "$O/mentioned.txt")"
echo "mentioned, not expanded:"; comm -13 "$O/expanded.txt" "$O/mentioned.txt"
echo "expanded, not mentioned:"; comm -23 "$O/expanded.txt" "$O/mentioned.txt"
```

#### §E.scanV.awk

`scanV.awk` (sha256 `793cf373032603bd`, 11 lines)

```awk
# MECHANISM PROTOTYPE (CR2 CR#4, variant V): the hardened rule-1 scan. Not the slice's text.
FNR == 1 { hd = "" }
hd != "" { l = $0; if (hdt) sub(/^\t+/, "", l); if (l == hd) hd = ""; next }
{ r = $0; s = $0
  if (match(r, /(^|[^<])<<-?[ \t]*[^ \t;&|()<>]+/)) { hd = substr(r, RSTART, RLENGTH); hdt = (hd ~ /<<-/); sub(/^.*<<-?[ \t]*/, "", hd); gsub(/[\047"\\]/, "", hd) }
  if (match(s, /(^|[[:space:];&|()<>`])#/)) s = substr(s, 1, RSTART - 1)
  n = gsub(/\047/, "\047", s); if (n % 2) next
  if (n) { match(s, /\047.*\047/); s = substr(s, 1, RSTART - 1) " " substr(s, RSTART + RLENGTH) }
  gsub(/\\./, " ", s); s = s " "
  while (match(s, /[$][{]?_[A-Za-z0-9_]*_lbl[^A-Za-z0-9_]/)) {
    t = substr(s, RSTART, RLENGTH - 1); sub(/^[$][{]?/, "", t); print t; s = substr(s, RSTART + 1) } }
```

#### §E.cr4-edit.py

`cr4-edit.py` (sha256 `bbaa14e8c685820e`, 16 lines)

```python
import sys
# cr4-edit.py <clone> <variant>: insert two echo-only labels after the `_ev_lbl=` definition (an inserted line, no anchor edited).
# variant "q": the finding's lines (an apostrophe pair, a ` #` inside double quotes); "plain": the same lines without them.
d, v = sys.argv[1], sys.argv[2]
p = d + "/.claude/tools/webref-generic-core-trip-wire.controls.sh"
s = open(p).read()
a = '_ev_lbl="a previous run\'s channel evidence never answers for the next"\n'
assert s.count(a) == 1, s.count(a)
if v == "q":
    l1 = 'echo "!! the window\'s own check ($_v03a_lbl) isn\'t asked" >&2'
    l2 = 'echo "!! see slot #11-x ($_v03b_lbl) …" >&2'
else:
    l1 = 'echo "!! the windows own check ($_v03a_lbl) is not asked" >&2'
    l2 = 'echo "!! see slot 11-x ($_v03b_lbl) …" >&2'
ins = '_v03a_lbl="cr2 probe a"\n_v03b_lbl="cr2 probe b"\nif false; then\n  ' + l1 + '\n  ' + l2 + '\nfi\n'
open(p, "w").write(s.replace(a, a + ins))
```

#### §E.cr4-over.py

`cr4-over.py` (sha256 `f5d89dc5fde1c1f4`, 19 lines)

```python
import sys
# cr4-over.py <clone> [optin]: the orphan check's over-count, at wire level. A new label whose ONLY "use" is the
# body of a quoted here-document whose operator follows a ` #` inside double quotes, and one record for it
# anchored on an inserted line. With "optin": the mutation run filtered to that record (the Bc-t2.sh method).
d = sys.argv[1]; T = d + "/.claude/tools/webref-generic-core-trip-wire"
def rep(p, a, b):
    s = open(p).read(); assert s.count(a) == 1, (p, s.count(a)); open(p, "w").write(s.replace(a, b))
a = '_ev_lbl="a previous run\'s channel evidence never answers for the next"\n'
rep(T + ".controls.sh", a, a + '_v03d_lbl="cr2 probe d"\n: cr2-anchor-d\nif false; then\n  echo "x #y" <<\'EOF\'\n$_v03d_lbl\nEOF\nfi\n')
rep(T + ".records.sh", "\nMUTANTS\n", "\ncontrols:s/^: cr2-anchor-d$/: cr2-anchor-dd/\tcr2 probe d\nMUTANTS\n")
if len(sys.argv) > 2:
    rec = open(T + ".records.sh").read().split("\n"); st = rec.index("  done <<'MUTANTS'")
    n = 0
    for l in rec[st + 1:]:
        if l == "MUTANTS": break
        if l and not l.startswith("#"): n += 1
    rep(T + ".mutations.sh", '      _mut_parse "$_mrec"\n', '      _mut_parse "$_mrec"\n      [ "$_mut_n" = "%d" ] || continue\n' % n)
    rep(T + ".mutations.sh", "    _mut_gen_run\n", "    :\n")
    print("record index", n)
```

#### §E.cr4-parse.sh

`cr4-parse.sh` (sha256 `5a54ba97fecdda56`, 12 lines)

```bash
#!/opt/homebrew/bin/bash
# cr4-parse.sh: the shipped rule-1 scan over the finding's shapes (no wire run). Prints what the scan counts per shape.
set -eu; export LC_ALL=C; O=logs/cr4-parse; mkdir -p $O
awk -f logs/pops-head/shipped.awk /dev/null >/dev/null   # extracted by pops.sh
t(){ printf '%s\n' "$2" > $O/case.sh; r=$(awk -f logs/pops-head/shipped.awk $O/case.sh | tr '\n' ' '); printf '%-34s counted: [%s]\n' "$1" "$r"; }
t "dq apostrophes"            'echo "!! the window'"'"'s own check ($_v03a_lbl) isn'"'"'t asked"'
t "dq space-hash"             'echo "!! see slot #11-x ($_v03b_lbl) …"'
t "same, apostrophe removed"  'echo "!! the windows own check ($_v03a_lbl) is not asked"'
t "same, hash removed"        'echo "!! see slot 11-x ($_v03b_lbl) …"'
t "dq apostrophe, then sq data" 'echo "it'"'"'s"; echo '"'"'$_v03c_lbl'"'"''
t "dq hash hides heredoc op"  "$(printf 'echo "a #b" <<'"'"'EOF'"'"'\n$_v03d_lbl\nEOF')"
t "heredoc op visible"        "$(printf 'cat <<'"'"'EOF'"'"'\n$_v03e_lbl\nEOF')"
```

#### §E.cr4-proto.py

`cr4-proto.py` (sha256 `97fada82076f2ca4`, 25 lines)

```python
import sys
# cr4-proto.py <clone>: MECHANISM PROTOTYPE (not the slice's text) of the two-population census.
# The ratchet (`.bare`) and the literal rule read every SPELLED name's value (`$_lbd_names`, over-approximate);
# the orphan check keeps the expansion scan (`.vlabels`).
p = sys.argv[1] + "/.claude/tools/webref-generic-core-trip-wire.mutations.sh"
s = open(p).read()
def rep(a, b):
    global s
    assert s.count(a) == 1, (s.count(a), a[:60]); s = s.replace(a, b)
rep('  for _cen_v in $_cen_used; do printf \'%s\\n\' "${!_cen_v}"; done | LC_ALL=C sort -u > "$_VFY/.vlabels"\n',
    '  for _cen_v in $_cen_used; do printf \'%s\\n\' "${!_cen_v}"; done | LC_ALL=C sort -u > "$_VFY/.vlabels"\n'
    '  for _cen_v in $_lbd_names; do printf \'%s\\n\' "${!_cen_v}"; done | LC_ALL=C sort -u > "$_VFY/.mvalues"  # PROTO\n')
rep('    FILENAME == ARGV[1] { isv[$0] = 1; print $0 | lc; next }\n',
    '    FILENAME == ARGV[1] { print $0 | lc; next }\n    FILENAME == ARGV[4] { isv[$0] = 1; next }  # PROTO\n')
rep('      close(lc); close(cc); exit bad }\' "$_VFY/.vlabels" "$_VFY/.registry" "$_CONTROLS" || _cen_rc=$?\n',
    '      close(lc); close(cc); exit bad }\' "$_VFY/.vlabels" "$_VFY/.registry" "$_CONTROLS" "$_VFY/.mvalues" || _cen_rc=$?\n')
rep("    { if ($0 ~ /^[[:space:]]*#/) next\n", "    FILENAME == ARGV[4] { isv[$0] = 1; next }\n    { if ($0 ~ /^[[:space:]]*#/) next\n") if False else None
# the bare population: .labels (registry + expansions) UNION the spelled values
rep('    { lb[++nl] = $0; isl[$0] = 1 }\n',
    '    FILENAME == ARGV[3] { if (!($0 in isb)) { isb[$0] = 1; bl[++nb] = $0 }; next }  # PROTO\n    { lb[++nl] = $0; isl[$0] = 1; if (!($0 in isb)) { isb[$0] = 1; bl[++nb] = $0 } }\n')
rep('      for (i = 1; i <= nl; i++) if (!(lb[i] in isn)) print lb[i] > ENVIRON["K2_BARE"]\n',
    '      for (i = 1; i <= nb; i++) if (!(bl[i] in isn)) print bl[i] > ENVIRON["K2_BARE"]  # PROTO\n')
rep('    }\' "$_VFY/.needles" "$_VFY/.labels" || _mut_arc=$?\n',
    '    }\' "$_VFY/.needles" "$_VFY/.labels" "$_VFY/.mvalues" || _mut_arc=$?\n')
open(p, "w").write(s)
```

#### §E.r4a-rec.py

`r4a-rec.py` (sha256 `2d0869f3080c138f`, 10 lines)

```python
import sys
# r4a-rec.py <clone>: R4a as a REAL record (F6): one line before `MUTANTS` in records.sh. It appends, to controls.sh's
# `_pe_lbl=` definition (a label line before `_lbd` that no record anchors), a new label and its only use inside a
# double-quoted string holding an apostrophe pair. The new name is BUILT by sed backreferences from the anchor line
# (`\1r4a\2` from `\(_pe\)\(_lbl\)`), so records.sh spells no `_*_lbl` token that is not defined; its end is anchored on
# the line's literal end (the twice rule). Needle: `$_cen_lbl`'s value.
p = sys.argv[1] + "/.claude/tools/webref-generic-core-trip-wire.records.sh"; s = open(p).read()
rec = ('controls:s/^\\(_pe\\)\\(_lbl\\)="no exec-path override reaches the fixture git"$/&; \\1r4a\\2="cr2 r4a probe"; : "it\'s $\\1r4a\\2\'s"/'
       '\tevery label the run used is named by a mutation record\n')
a = "MUTANTS\n}\n"; assert s.count(a) == 1; open(p, "w").write(s.replace(a, rec + a))
```

#### §E.fresh.py

`fresh.py` (sha256 `f6dfb74e4f6c40ae`, 71 lines)

```python
#!/usr/bin/env python3
# fresh.py <clone> [sed]: MECHANISM PROTOTYPE of CR#7's two freshness rules over the clone's CURRENT record table,
# applied with the real sed (default /usr/bin/sed; pass a GNU sed path to compare). Not the slice's text.
# Per expression: (1) lines changed (the shipped rule: must be 1); (2) TWICE: `-e x -e x` == `-e x`;
# (3) CARRY: for every char of the changed line OUTSIDE the changed span (common prefix/suffix of L and L'),
# replace it by \x01 in L: x must then either leave the line unchanged or output L' with the same \x01 carried.
import sys, subprocess, os
c = sys.argv[1]; SED = sys.argv[2] if len(sys.argv) > 2 else "/usr/bin/sed"
T = c + "/.claude/tools/webref-generic-core-trip-wire"
parts = "controls harness ctlrun mutations records fixtures mutgen".split()
targets = [p for p in parts if p != "records"]
lines = open(T + ".records.sh", encoding="latin-1").read().split("\n")
st = lines.index("  done <<'MUTANTS'")
recs = []
for i in range(st + 1, len(lines)):
    if lines[i] == "MUTANTS": break
    if lines[i] and not lines[i].startswith("#"): recs.append((i + 1, lines[i]))
def sed(args, data):
    r = subprocess.run([SED] + args, input=data, capture_output=True)
    return r.returncode, r.stdout
out = []; idx = 0
for ln, rec in recs:
    idx += 1
    f = rec.split("\t"); ex = f[:-1]; needle = f[-1]
    which = "wire"
    for t in targets:
        if ex[0].startswith(t + ":"): which = t; ex[0] = ex[0][len(t) + 1:]; break
    path = T + ".sh" if which == "wire" else T + "." + which + ".sh"
    data = open(path, "rb").read()
    src = data.split(b"\n")
    for e in ex:
        rc, o = sed(["-e", e], data)
        if rc: out.append((idx, ln, which, "INVALID", e)); continue
        ol = o.split(b"\n")
        # changed lines via difflib-free approach: use diff
        import difflib
        sm = difflib.SequenceMatcher(a=src, b=ol, autojunk=False)
        ops = [op for op in sm.get_opcodes() if op[0] != "equal"]
        dels = sum(i2 - i1 for _, i1, i2, _, _ in ops); adds = sum(j2 - j1 for _, _, _, j1, j2 in ops)
        n = max(dels, adds)
        verdict = []
        if n != 1: verdict.append("COUNT%d" % n)
        rc2, o2 = sed(["-e", e, "-e", e], data)
        if o2 != o: verdict.append("TWICE")
        if n == 1 and len(ops) == 1 and ops[0][0] == "replace":
            _, i1, i2, j1, j2 = ops[0]; L = src[i1]; Lp = ol[j1]
            P = 0
            while P < min(len(L), len(Lp)) and L[P] == Lp[P]: P += 1
            S = 0
            while S < min(len(L), len(Lp)) - P and L[-1 - S] == Lp[-1 - S]: S += 1
            pos = list(range(P)) + list(range(len(L) - S, len(L)))
            bad = []
            if pos:
                var = []; exp = []
                for p in pos:
                    v = L[:p] + b"\x01" + L[p + 1:]
                    q = p if p < P else p - len(L) + len(Lp)
                    var.append(v); exp.append(Lp[:q] + b"\x01" + Lp[q + 1:])
                newsrc = src[:i1] + var + src[i1 + 1:]
                rc3, o3 = sed(["-e", e], b"\n".join(newsrc))
                ol3 = o3.split(b"\n")
                got = ol3[i1:i1 + len(var)]
                if len(ol3) != len(newsrc): bad.append("LEN")
                else:
                    for k, p in enumerate(pos):
                        if got[k] != var[k] and got[k] != exp[k]: bad.append(p)
            if bad: verdict.append("CARRY@%s" % (",".join(map(str, bad[:5])) + ("…" if len(bad) > 5 else "")) + "/%d" % len(pos))
        out.append((idx, ln, which, " ".join(verdict) or "fresh", e))
for o in out:
    if o[3] != "fresh": print("%3d records.sh:%-4d %-9s %-28s %s" % (o[0], o[1], o[2], o[3], o[4][:110]))
print("expressions", len(out), "non-fresh", sum(1 for o in out if o[3] != "fresh"))
```

#### §E.fresh.sh

`fresh.sh` (sha256 `511591e12f9fde05`, 37 lines)

```bash
#!/bin/bash
# fresh.sh <clone>: MECHANISM PROTOTYPE of CR#7's rules in the wire's own tools (bash, sed, diff, awk via ENVIRON;
# LC_ALL=C), over the clone's current record table. Prints each non-fresh expression; the last line, the totals.
# Not the slice's text. Rule 1 = the shipped count; 2 = TWICE; 3 = CARRY (fresh.py's definition).
set -u; export LC_ALL=C
c="$1"; T="$c/.claude/tools/webref-generic-core-trip-wire"; tab=$'\t'; W="$(mktemp -d "$PWD/logs/fresh.XXXXXX")"
nx=0; nbad=0
while IFS= read -r row; do
  n="${row%%"$tab"*}"; rest="${row#*"$tab"}"; ex=()
  while :; do case "$rest" in *"$tab"*) ex+=("${rest%%"$tab"*}"); rest="${rest#*"$tab"}" ;; *) break ;; esac; done
  which=wire; for t in controls harness ctlrun fixtures mutgen mutations; do case "${ex[0]}" in "$t":*) which=$t; ex[0]="${ex[0]#"$t":}"; break ;; esac; done
  f="$T.sh"; [ "$which" = wire ] || f="$T.$which.sh"
  for e in "${ex[@]}"; do
    nx=$((nx + 1)); v=""
    sed -e "$e" "$f" > "$W/a" 2>/dev/null || { echo "$n INVALID"; continue; }
    diff "$f" "$W/a" > "$W/d"
    d=0; a=0; L=""; LP=""; N=""
    while IFS= read -r l; do case "$l" in '<'*) d=$((d+1)); L="${l#< }" ;; '>'*) a=$((a+1)); LP="${l#> }" ;; [0-9]*) N="${l%%[acd]*}" ;; esac; done < "$W/d"
    k=$(( d > a ? d : a )); [ "$k" -eq 1 ] || v="$v COUNT$k"
    sed -e "$e" -e "$e" "$f" > "$W/b" 2>/dev/null; cmp -s "$W/a" "$W/b" || v="$v TWICE"
    if [ "$k" -eq 1 ] && [ "$d" -eq 1 ] && [ "$a" -eq 1 ]; then
      K2_L="$L" K2_LP="$LP" K2_N="$N" K2_V="$W/v" awk 'BEGIN { L = ENVIRON["K2_L"]; P2 = ENVIRON["K2_LP"]; n = ENVIRON["K2_N"] + 0
          a = length(L); b = length(P2); p = 0; while (p < a && p < b && substr(L, p + 1, 1) == substr(P2, p + 1, 1)) p++
          s = 0; while (s < a - p && s < b - p && substr(L, a - s, 1) == substr(P2, b - s, 1)) s++
          printf "" > ENVIRON["K2_V"] }
        FNR == n { for (i = 1; i <= a; i++) if (i <= p || i > a - s) { print substr(L, 1, i - 1) "\001" substr(L, i + 1)
            j = (i <= p) ? i : i - a + b; print substr(P2, 1, j - 1) "\001" substr(P2, j + 1) > ENVIRON["K2_V"] } ; next }
        { print }' "$f" > "$W/c"
      sed -e "$e" "$W/c" > "$W/co" 2>/dev/null
      bad="$(K2_N="$N" awk 'FILENAME == ARGV[1] { ex[++ne] = $0; next } FILENAME == ARGV[2] { if (FNR >= ENVIRON["K2_N"] + 0 && k < ne) vi[++k] = $0; next }
          { if (FNR >= ENVIRON["K2_N"] + 0 && m < ne) { m++; if ($0 != vi[m] && $0 != ex[m]) bad++ } } END { print bad + 0 }' "$W/v" "$W/c" "$W/co")"
      [ "$bad" -eq 0 ] || v="$v CARRY$bad"
    fi
    [ -z "$v" ] || { nbad=$((nbad + 1)); printf 'records.sh:%s %s:%s%s\n' "$n" "$which" "$N" "$v"; }
  done
done < <(awk '/^  done <<.MUTANTS.$/{on=1; next} /^MUTANTS$/{on=0} on && !/^#/ && NF {print NR "\t" $0}' "$T.records.sh")
echo "expressions $nx non-fresh $nbad"; rm -rf "$W"
```

#### §E.fresh-countonly.sh

`fresh-countonly.sh` (sha256 `9794153ce3f6f9f4`, 36 lines)

```bash
#!/bin/bash
# fresh.sh <clone>: MECHANISM PROTOTYPE of CR#7's rules in the wire's own tools (bash, sed, diff, awk via ENVIRON;
# LC_ALL=C), over the clone's current record table. Prints each non-fresh expression; the last line, the totals.
# Not the slice's text. Rule 1 = the shipped count; 2 = TWICE; 3 = CARRY (fresh.py's definition).
set -u; export LC_ALL=C
c="$1"; T="$c/.claude/tools/webref-generic-core-trip-wire"; tab=$'\t'; W="$(mktemp -d "$PWD/logs/fresh.XXXXXX")"
nx=0; nbad=0
while IFS= read -r row; do
  n="${row%%"$tab"*}"; rest="${row#*"$tab"}"; ex=()
  while :; do case "$rest" in *"$tab"*) ex+=("${rest%%"$tab"*}"); rest="${rest#*"$tab"}" ;; *) break ;; esac; done
  which=wire; for t in controls harness ctlrun fixtures mutgen mutations; do case "${ex[0]}" in "$t":*) which=$t; ex[0]="${ex[0]#"$t":}"; break ;; esac; done
  f="$T.sh"; [ "$which" = wire ] || f="$T.$which.sh"
  for e in "${ex[@]}"; do
    nx=$((nx + 1)); v=""
    sed -e "$e" "$f" > "$W/a" 2>/dev/null || { echo "$n INVALID"; continue; }
    diff "$f" "$W/a" > "$W/d"
    d=0; a=0; L=""; LP=""; N=""
    while IFS= read -r l; do case "$l" in '<'*) d=$((d+1)); L="${l#< }" ;; '>'*) a=$((a+1)); LP="${l#> }" ;; [0-9]*) N="${l%%[acd]*}" ;; esac; done < "$W/d"
    k=$(( d > a ? d : a )); [ "$k" -eq 1 ] || v="$v COUNT$k"
    if false; then
      K2_L="$L" K2_LP="$LP" K2_N="$N" K2_V="$W/v" awk 'BEGIN { L = ENVIRON["K2_L"]; P2 = ENVIRON["K2_LP"]; n = ENVIRON["K2_N"] + 0
          a = length(L); b = length(P2); p = 0; while (p < a && p < b && substr(L, p + 1, 1) == substr(P2, p + 1, 1)) p++
          s = 0; while (s < a - p && s < b - p && substr(L, a - s, 1) == substr(P2, b - s, 1)) s++
          printf "" > ENVIRON["K2_V"] }
        FNR == n { for (i = 1; i <= a; i++) if (i <= p || i > a - s) { print substr(L, 1, i - 1) "\001" substr(L, i + 1)
            j = (i <= p) ? i : i - a + b; print substr(P2, 1, j - 1) "\001" substr(P2, j + 1) > ENVIRON["K2_V"] } ; next }
        { print }' "$f" > "$W/c"
      sed -e "$e" "$W/c" > "$W/co" 2>/dev/null
      bad="$(K2_N="$N" awk 'FILENAME == ARGV[1] { ex[++ne] = $0; next } FILENAME == ARGV[2] { if (FNR >= ENVIRON["K2_N"] + 0 && k < ne) vi[++k] = $0; next }
          { if (FNR >= ENVIRON["K2_N"] + 0 && m < ne) { m++; if ($0 != vi[m] && $0 != ex[m]) bad++ } } END { print bad + 0 }' "$W/v" "$W/c" "$W/co")"
      [ "$bad" -eq 0 ] || v="$v CARRY$bad"
    fi
    [ -z "$v" ] || { nbad=$((nbad + 1)); printf 'records.sh:%s %s:%s%s\n' "$n" "$which" "$N" "$v"; }
  done
done < <(awk '/^  done <<.MUTANTS.$/{on=1; next} /^MUTANTS$/{on=0} on && !/^#/ && NF {print NR "\t" $0}' "$T.records.sh")
echo "expressions $nx non-fresh $nbad"; rm -rf "$W"
```

#### §E.cr7-edit.py

`cr7-edit.py` (sha256 `5b54b521a8230f1f`, 8 lines)

```python
import sys
# cr7-edit.py <clone>: the finding's scenario. The first segment's class of K2RE respelled to an EQUAL set
# (`[^/[:space:]]+/(` -> `[^[:space:]/]+/(`), and record 79 (anchored on it) re-anchored onto the new spelling.
d = sys.argv[1]; T = d + "/.claude/tools/webref-generic-core-trip-wire"
def rep(p, a, b):
    s = open(p).read(); assert s.count(a) == 1, (p, s.count(a)); open(p, "w").write(s.replace(a, b))
rep(T + ".sh", "(skills|tools)/[^/[:space:]]+/([^/[:space:]]+/|", "(skills|tools)/[^[:space:]/]+/([^/[:space:]]+/|")
rep(T + ".records.sh", "s|\\[^/\\[:space:\\]\\]+/(|[^/[:space:])}>,;]+/(|\t", "s|\\[^\\[:space:\\]/\\]+/(|[^[:space:]/)}>,;]+/(|\t")
```

#### §E.cr7-reanchor.py

`cr7-reanchor.py` (sha256 `b5cdc63ebcba0638`, 29 lines)

```python
import sys
# cr7-reanchor.py <clone>: MECHANISM PROTOTYPE of CR#7's record re-anchors (in place, no records.sh line moves).
# Each entry: records.sh line -> (old first field, new first field). Line 81 (byte-identical mutant to line 80's)
# becomes a `#` comment line (`_mutants` drops it; the line count of the file is unchanged).
d = sys.argv[1]; p = d + "/.claude/tools/webref-generic-core-trip-wire.records.sh"
L = open(p, encoding="latin-1").read().split("\n")
def fld(n, old, new):
    f = L[n - 1].split("\t"); assert f[0] == old, (n, f[0]); f[0] = new; L[n - 1] = "\t".join(f)
fld(45, r"""s#^K2RE_PATH=.*#K2RE_PATH='(^|/)\\.claude/(skills|tools)/[^/"]+/[^/]+'#""",
        r"""s#^\(K2RE_PATH='(^|/)\\\.claude/(skills|tools)/\[^/\)\(\]+/\[^/\]+'\)$#\1"\2#""")
fld(55, r"""s|^_esc() .*|_esc() { printf '%s' "$1"; }|""", r"""s|^\(_esc() { \).*\("; }\)$|\1printf '%s' "$1\2|""")
for n in (71, 103, 104, 106, 107, 108):
    f0 = L[n - 1].split("\t")[0]; fld(n, f0, f0.replace(r"s/\[^A-Za-z0-9_.~@+%-\]/", r"s/\[^A-Za-z0-9_\.~@+%-\]/", 1))
f0 = L[108].split("\t")[0]; fld(109, f0, f0.replace(r"s|\[^A-Za-z0-9_.~@+%-\]|", r"s|\[^A-Za-z0-9_\.~@+%-\]|", 1))
fld(80, r"s/(^|\/)\\.claude/\\.claude/", r"s/(^|\/)\\\.claude/\\.claude/")
L[80] = "# (record removed by CR2: its mutant was byte-identical to the line above's, both K2RE_PATH without `(^|/)`)"
for n in (89, 90, 91, 92, 97):
    f0 = L[n - 1].split("\t")[0]; assert f0.startswith(r"s|\[^/\[:space:\]\]+/|") and f0.endswith("+/|"), f0
    fld(n, f0, r"s|\[^/\[:space:\]\]+/(|" + f0[len(r"s|\[^/\[:space:\]\]+/|"):-1] + "(|")
fld(101, "s/exec git -c core.fsmonitor=false /exec git /", r"s/exec git -c core\.fsmonitor=false /exec git /")
for n in (134, 135, 137):
    f0 = L[n - 1].split("\t")[0]; fld(n, f0, f0.replace(r"#^K2RE_PATH='(^|/)\\.claude", r"#^K2RE_PATH='(^|/)\\\.claude", 1))
f0 = L[223].split("\t")[0]; fld(224, f0, f0.replace(r'"[$]_pq\/pre.err" ||/', r'"[$]_pq\/pre\.err" ||/', 1))
fld(226, r'harness:/^  find "[$]CTL" .*-iname .git/s/-print0/-print/g',
         r'harness:/^  find "[$]CTL" .*-iname .git/s/\(-iname \.git \)-print0\(.*-iname HEAD \)-print0\(.*-perm -u=rx \)-print0/\1-print\2-print\3-print/')
f0 = L[322].split("\t")[0]; fld(323, f0, f0.replace(r"""s/^    printf '"%s"\\n' .*> "[$]_pq""", r"""s/^    printf '"%s"\\n' .* > "[$]_pq""", 1))
f0 = L[328].split("\t")[0]; fld(329, f0, f0.replace(r's/^  find "[$]CTL" .*-iname .git.*$/', r's/^  find "[$]CTL" .*-iname .git.* || _pgpre=" (census failed)"$/', 1))
f0 = L[381].split("\t")[0]; fld(382, f0, f0.replace(r'_limit_lines=.*$/&;', r"""_limit_lines=.*\/')"$/&;""", 1))
open(p, "w", encoding="latin-1").write("\n".join(L))
```

#### §E.f13drift.py

`f13drift.py` (sha256 `a56cea594a1c8002`, 8 lines)

```python
import sys
# f13drift.py <clone>: a CONSTRUCTED drift of the address-range-end shape for record records.sh:403
# (`ctlrun:/^    if \[ -z "[$]_cap_f" \]; then$/,/^    fi$/s/^      return 1$/      return 0/`): the range's end line and
# the line the record was written for are both respelled (a trailing comment each, behaviour unchanged), so the range
# runs on to the next `    fi` and the record now edits `_ctl_gate`'s `return 1` after `if _fixture_did_fail …` (ctlrun.sh:205) instead.
p = sys.argv[1] + "/.claude/tools/webref-generic-core-trip-wire.ctlrun.sh"; s = open(p).read()
a = '      return 1\n    fi\n    _cap_flag_of "$_cap_g"\n'; assert s.count(a) == 1
open(p, "w").write(s.replace(a, '      return 1 # undeclared group\n    fi # undeclared group\n    _cap_flag_of "$_cap_g"\n'))
```

#### §E.f13span.sh

`f13span.sh` (sha256 `197d5f6df2c16198`, 10 lines)

```bash
#!/bin/bash
# f13span.sh: clause 3 (faithful) is blind INSIDE the changed span. A one-line fake target `k=abc` and two records run
# through bin/fresh.sh (the bash prototype of the three clauses): A `s/^k=a.c$/k=aXc/` (its wildcard reads the character
# it replaces, inside the span) and B `s/^k=.b/k=aX/` (its wildcard reads `a`, outside the span, and restates it). Then
# the target respelled to `k=aYc`: what A writes there.
H=/Users/kazuaki/repos/send.sh/elidex-fgit-handoff/cr2; F="$H/work/f13span"; rm -rf "$F"; mkdir -p "$F/.claude/tools"
T="$F/.claude/tools/webref-generic-core-trip-wire"; printf 'k=abc\n' > "$T.sh"
printf '%s\n' '_mutants() {' "  done <<'MUTANTS'" 's/^k=a.c$/k=aXc/	A' 's/^k=.b/k=aX/	B' 'MUTANTS' '}' > "$T.records.sh"
cd "$H" && bin/fresh.sh "$F"
printf 'k=aYc\n' > "$F/respelled"; echo "A on the respelled line: $(/usr/bin/sed -e 's/^k=a.c$/k=aXc/' "$F/respelled")"
```

#### §E.cr9-proto.py

`cr9-proto.py` (sha256 `5b52e19192bee7aa`, 10 lines)

```python
import sys
# cr9-proto.py <clone>: MECHANISM PROTOTYPE (not the slice's text). The fsmonitor block's plain-git call purges
# git's own list (`$_GIT_LOCAL_VARS`, the wire's, configuration included) instead of three hand-named variables;
# the one configuration it asks about is given back through `env`.
d = sys.argv[1]; p = d + "/.claude/tools/webref-generic-core-trip-wire.controls.sh"
s = open(p).read()
a = 'elif ( unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE; cd "$_fsm_root" \\\n'
assert s.count(a) == 1
s = s.replace(a, 'elif ( for _v in $_GIT_LOCAL_VARS; do unset "$_v"; done; cd "$_fsm_root" \\\n')
open(p, "w").write(s)
```

#### §E.cr14-edit.py

`cr14-edit.py` (sha256 `43df055004ab94cc`, 14 lines)

```python
import sys
# cr14-edit.py <clone> <ro|rw> [fix]: a control whose run, if it ran, would fail (clean fixture, expected 1 "K2: a"),
# placed right after the previous control left "K2: a" in `$_VFY/.control_out`; "ro" makes that file read-only
# first, so the child's redirect cannot open it (non-root). "fix" = MECHANISM PROTOTYPE of the root fix in ctlrun
# (the output file removed with the timeout marker, before the launch, fail-closed).
d, v = sys.argv[1], sys.argv[2]; T = d + "/.claude/tools/webref-generic-core-trip-wire"
def rep(p, a, b):
    s = open(p).read(); assert s.count(a) == 1, (p, s.count(a)); open(p, "w").write(s.replace(a, b))
a = '_control "$CTL/k2" 1 "K2: a" "a caller\'s GREP_OPTIONS cannot hide a file" "" "" "$CTL/goptgrep" "" "GREP_OPTIONS=--exclude=*" || ctl_ok=1\n'
ro = 'chmod 444 "$_VFY/.control_out"\n' if v == "ro" else ''
rep(T + ".controls.sh", a, a + ro + '_control "$CTL/clean" 1 "K2: a" "cr2 stale probe" || ctl_ok=1\nchmod 644 "$_VFY/.control_out" 2>/dev/null || :\n')
if len(sys.argv) > 3:
    rep(T + ".ctlrun.sh", '  _r_wmark="$_VFY/.control_timeout"; command rm -f "$_r_wmark"\n',
        '  _r_wmark="$_VFY/.control_timeout"; command rm -f "$_r_wmark" "$_out_f" || { echo "!! CONTROL FAILED ($_r_for): a previous run\'s output or timeout marker could not be cleared" >&2; return 1; }  # PROTO\n')
```

### §E.4 The inputs

`cr12.sh` reads `work/shim-rm/rm` (below) and `work/fx/clean`, a one-file clean repository that `§E.mkfx-clean.sh`
makes (`mkfx-clean.sh <H>/work/fx/clean`); CR11-J's ship run puts `work/r5-ps-shim/ps` (below) first on `PATH`.
`oracle2.sh` carries `oracle.sh`'s text, and `r2-optin.sh` and `r3-optin.sh` carry the single-record filter (the
`Bc-t2.sh` method) inline: they read no other file. The frame clone is `706049ad` (above). Nothing else is read.

#### §E.rm

`rm` (sha256 `6b7ff8e6f0f092a0`, 4 lines) — `work/shim-rm/rm`, mode 755

```sh
#!/bin/sh
# CR#12 repro shim: fails `rm -rf …` (the EXIT trap's scratch removal), passes every other rm through.
[ "${1:-}" = -rf ] && { echo "shim rm: refusing $*" >&2; exit 1; }
exec /bin/rm "$@"
```

#### §E.ps

`ps` (sha256 `275239824e00e61b`, 2 lines) — `work/r5-ps-shim/ps`, mode 755

```sh
#!/bin/sh
exit 1
```
