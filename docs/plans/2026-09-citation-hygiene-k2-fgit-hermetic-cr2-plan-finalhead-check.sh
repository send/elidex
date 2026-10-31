#!/bin/bash
# FINALHEAD-PLAN-DELTA.md (r6) / …-cr2-plan-finalhead.md evidence: static facts at T and at the frame, read with
# `git show`; the runbook's definitions file, frame guard, extraction and patch block, gate control, insert block and
# X5 build and generation lines taken from the runbook's own text and run on scratch copies; the umbrella's repo-half
# preflight count simulated on a scratch clone. Runs no wire.
# Usage: bash check.sh [memo] [output dir] [runbook]
#   defaults follow this file's location: in the handoff (H/cr2/logs/fhd) the handoff memo and runbook (the memo's §6,
#   moved out at r4) and H/cr2/logs/fhd/out; anywhere else (docs/plans/ after the move) the memo and the runbook beside
#   this file and a fresh mktemp -d.
#   An output directory is cleared only if it is empty or carries the marker .fhd-check-out; the handoff's out/ is
#   never cleared when this file runs from anywhere but the handoff.
set -u
W=/Users/kazuaki/repos/send.sh/elidex-wt-fgit-hermetic; H=/Users/kazuaki/repos/send.sh/elidex-fgit-handoff
HERE=$(cd "$(dirname "$0")" && pwd); SELFP=$HERE/$(basename "$0")
if [ "$HERE" = "$H/cr2/logs/fhd" ]; then MEMO=${1:-$H/cr2/FINALHEAD-PLAN-DELTA.md}; O=${2:-$HERE/out}; RB=${3:-$H/cr2/FINALHEAD-PLAN-DELTA-RUNBOOK.md}
else MEMO=${1:-$HERE/2026-09-citation-hygiene-k2-fgit-hermetic-cr2-plan-finalhead.md}; O=${2:-$(mktemp -d)}; RB=${3:-$HERE/2026-09-citation-hygiene-k2-fgit-hermetic-cr2-plan-finalhead-runbook.md}
  [ "$O" != "$H/cr2/logs/fhd/out" ] || { echo "refusing: the handoff's out/ is not cleared from outside the handoff"; exit 2; }; fi
if [ -e "$O" ] && [ -n "$(ls -A "$O" 2>/dev/null)" ] && [ ! -e "$O/.fhd-check-out" ]; then echo "refusing to clear $O (no .fhd-check-out marker)"; exit 2; fi
rm -rf "$O"; mkdir -p "$O/T"; : > "$O/.fhd-check-out"; G=/usr/bin/grep
echo "## checker sha256 (the file as run): $(shasum -a 256 "$SELFP" | cut -d' ' -f1)"; echo "## memo: $MEMO; runbook: $RB; out: $O"
# blk <marker>: the fenced ```sh block of the runbook that contains <marker>
blk() { python3 - "$RB" "$1" <<'PY'
import sys; s=open(sys.argv[1]).read(); i=s.index(sys.argv[2]); a=s.rindex('```sh\n',0,i)+6; b=s.index('\n```',i); print(s[a:b])
PY
}
DP=docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic
DOC() { git -C "$W" show "${2:-3b7ff6d5}:$DP$1"; }      # the runbook's frame read (runbook §6.0)
cd "$W" || exit 2
echo "## frame (want 3b7ff6d5816a…)"; git rev-parse --verify -q '3b7ff6d5^{commit}'
echo "## T (landing §9.1 command, over e8f78896..3b7ff6d5)"
_job() { git show "$1:.github/workflows/ci.yml" | sed -n '/^  trip-wires:/,/^  [a-z]/p' | $G -v '^ *#'; }
for c in $(git log --format=%H e8f78896..3b7ff6d5); do
  [ -n "$(git diff --name-only "$c^" "$c" -- .claude/tools scripts)" ] && { echo "$c"; break; }
  [ "$(_job "$c^")" = "$(_job "$c")" ] || { echo "$c"; break; }
done
echo "## tool diff 01fb2924..3b7ff6d5 (want 0)"; git diff --name-only 01fb2924 3b7ff6d5 -- .claude/tools scripts | wc -l
for p in "" .controls .harness .ctlrun .mutations .correspondence .records .fixtures .mutgen; do
  git show 01fb2924:.claude/tools/webref-generic-core-trip-wire$p.sh > "$O/T/w$p.sh"; done
echo "## parts at T"; ls "$O/T" | wc -l
echo "## anchors at T (line numbers)"
$G -n -F ': > "$_FW_DIR/built"' "$O/T/w.fixtures.sh"
$G -n -F '>> "$_VFY/.genmutants"' "$O/T/w.mutgen.sh"
$G -n -E '^_mut_gen_run\(\)|^}' "$O/T/w.mutgen.sh" | $G -A1 '_mut_gen_run'
$G -n '^_MUT_UNRECORDED_MAX=\|^_MUT_RECORDS_MIN=' "$O/T/w.mutations.sh"
$G -n -F 'config --add core.bare false; &/' "$O/T/w.records.sh"
$G -n '^MUTANTS$' "$O/T/w.records.sh"
echo "## _lbl definitions in controls (count, first, last)"; $G -c '_lbl="' "$O/T/w.controls.sh"; $G -n '_lbl="' "$O/T/w.controls.sh" | sed -n '1p;$p'
echo "## _lbl= in the other parts (want 0)"; $G -n '^_[a-z0-9]*_lbl=' "$O/T"/w.sh "$O/T"/w.harness.sh "$O/T"/w.ctlrun.sh "$O/T"/w.mutations.sh "$O/T"/w.correspondence.sh "$O/T"/w.mutgen.sh "$O/T"/w.fixtures.sh | wc -l
echo "## generator inputs unchanged 21526e80..01fb2924 (want 0, 0)"
git diff --name-only 21526e80 01fb2924 -- .claude/tools/webref-generic-core-trip-wire.mutgen.sh | wc -l
git diff 21526e80 01fb2924 -- .claude/tools/webref-generic-core-trip-wire.sh | $G -c '^[-+]K2RE'
echo "## records: floor at 21526e80 / at T"; git show 21526e80:.claude/tools/webref-generic-core-trip-wire.mutations.sh | $G '^_MUT_RECORDS_MIN='; $G '^_MUT_RECORDS_MIN=' "$O/T/w.mutations.sh"
echo "## s8b-x3 wall times at 21526e80 (s)"
L=$H/runners/logs/s8b-x3; for c in main plant floor; do for s in b53 b32; do echo "$c $s $(( $(cat $L/$c-$s.end) - $(cat $L/$c-$s.start) ))"; done; done

echo "## defs: runbook §6.0's definitions file, its body taken from the runbook (sourced below, K and SP re-pointed to scratch)"
python3 - "$RB" > "$O/defs.sh" <<'PY'
import sys; s=open(sys.argv[1]).read(); i=s.index("/cr2/work/fh/defs.sh <<'D'\n")+len("/cr2/work/fh/defs.sh <<'D'\n"); j=s.index('\nD\n',i); print(s[i:j])
PY
/opt/homebrew/bin/bash -n "$O/defs.sh" && /bin/bash -n "$O/defs.sh" && echo "defs.sh: $(wc -l < "$O/defs.sh" | tr -d ' ') lines, bash -n clean, 3.2 and 5.3; defines: $(/opt/homebrew/bin/bash -c '. "$1"; declare -F | cut -d" " -f3 | tr "\n" " "' _ "$O/defs.sh")"
$G -c "k2w\[d\]" "$O/defs.sh" | sed 's/^/gate pattern k2w[d] in defs.sh: /'
echo "## frame guard: runbook §6.0's guard fence, taken from the runbook, K re-pointed to scratch (reads W and origin)"
{ cat "$O/defs.sh"; echo "K=$O/guard; SP=$O/guard/x5"; blk 'cd "$W" || exit 1'; } > "$O/guard.sh"
/opt/homebrew/bin/bash "$O/guard.sh"; echo "frame guard rc=$?"
echo "## scripts: runbook §6.0's extraction + patch block, taken from the runbook, run on a scratch K (no run)"
X=$O/scripts; mkdir -p "$X"
{ cat "$O/defs.sh"; echo "K=$X; SP=$X/x5"; blk 'DOC -pr527.md     | sed -n 627,646p > cell15.sh'; echo 'echo "script block: completed"'; } > "$O/scripts.sh"
/opt/homebrew/bin/bash "$O/scripts.sh"; echo "script block rc=$?"
cd "$X" || exit 2
for f in cell15.sh cellw.sh crc.sh; do /bin/bash -n $f && /opt/homebrew/bin/bash -n $f && echo "$f: bash -n clean, 3.2 and 5.3"; done
echo "xt negative control: an empty extraction and a wrong first line both stop"
/opt/homebrew/bin/bash -c '. "$1"; : > "$2/e"; xt "$2/e" "#!/bin/bash" && echo "NOT STOPPED (empty)"; echo x > "$2/e"; xt "$2/e" "#!/bin/bash" && echo "NOT STOPPED (wrong)"; true' _ "$O/defs.sh" "$O"
diff <(sed 2d $H/cell15.sh) <(DOC -pr527.md | sed -n 627,646p) > /dev/null && echo "cell15.sh on disk = PR527:627-646 plus its line-2 comment"
echo "fences (PR527 :626 :647 :648; landing :435 :442 :535 :552)"
for n in 626 647 648; do printf 'PR527 %s: %s\n' $n "$(DOC -pr527.md | sed -n ${n}p | cut -c1-12)"; done
for n in 435 436 441 442 535 536 551 552; do printf 'landing %s: %s\n' $n "$(DOC -landing.md | sed -n ${n}p | cut -c1-40)"; done

echo "## crc.sh cell anchors at T (want 1 each)"
python3 - "$O/T" <<'PY'
import sys; d=sys.argv[1]
r=lambda n: open(f"{d}/w{n}.sh").read()
fx,hs,ct,w=r('.fixtures'),r('.harness'),r('.controls'),r('')
i=hs.index('_fgit_window_verdict_exit() {'); b=hs[i:hs.index('\n}\n',i)]
for k,v in [('w3ar walk/sub',fx.count('mkdir -p "$CTL/walk/sub"\n')),('garbagehead/sealdotdot built',fx.count(': > "$_FW_DIR/built"\n')),
 ('w2rec exit 2 (4 sp)',b.count('\n    exit 2')),('w2rec exit 1 (2 sp)',b.count('\n  exit 1')),('w2rec done marker',hs.count(': > "$_FW_DIR/done"\'')),
 ('lblrename _pa_lbl',ct.count('_pa_lbl="')),('NOU set -euo',w.count('\nset -euo pipefail\n')),('sealfail err/control.py seal',fx.count('_seal "$CTL/err/control.py" 000 err\n'))]: print(k,v)
PY
echo "## X16/X3/X11 edit anchors at T (want 1 each)"
python3 - "$O/T" <<'PY'
import sys; d=sys.argv[1]; c=lambda f,a: open(f"{d}/{f}").read().count(a)
print('plant: built line', c('w.fixtures.sh',': > "$_FW_DIR/built"\n'))
print('floor: generator append', c('w.mutgen.sh','_mut_regex_mutants "$_gr_v" "$_gr_val" >> "$_VFY/.genmutants" ||'))
print('x11: _af_lbl line', c('w.controls.sh','_af_lbl="a record\'s anchor is fresh only if it changes one line, once, and keeps what it does not read"\n'))
print('xbare U', c('w.mutations.sh','\n_MUT_UNRECORDED_MAX=18\n'))
print('xanchor A', c('w.records.sh','fixtures:s/^: > "[$]_FW_DIR\\/built"$/git -C "$CTL\\/clean" config --add core.bare false; &/'))
print('stale: here-document end', c('w.records.sh','\nMUTANTS\n'))
print('xrace: the reference dir (pgref) made beside $_FW_DIR', c('w.harness.sh','mktemp -d "${_FW_DIR%/*}/pgrefXXXXXX"'))
PY

echo "## r4 inserts: the runbook's §6.3 insert block, run on a scratch K, against the docs' text"
R=$O/rh; mkdir -p "$R/ins"
python3 - "$RB" > "$R/insblock.sh" <<'PY'
import sys; s=open(sys.argv[1]).read(); i=s.index('cat > "$K/ins/fifo-commondir"'); a=s.rindex('```sh\n',0,i)+6; b=s.index('\n```',i)
print(s[a:b])
PY
K=$R /opt/homebrew/bin/bash "$R/insblock.sh" && echo "insert block ran: $(ls "$R/ins" | wc -l | tr -d ' ') files"
DOC -pr527.md > "$R/pr527.md"; DOC -pr527-r15.md > "$R/r15.md"
python3 - "$R" <<'PY'
import sys,re,os; R=sys.argv[1]
A=open(R+'/pr527.md').read().split('\n'); Q=open(R+'/r15.md').read().split('\n'); L=lambda X,n: X[n-1]
want={}
for n in ['commondir','HEAD','config','config.worktree']:
    a=L(A,458); assert '%s' in a; want['fifo-'+n]='rm -f "$CTL/clean/.git/%s" && mkfifo "$CTL/clean/.git/%s"'%(n,n)
want['clean']=None
for n,t in zip(range(611,617),['xcommon','bareh','outsidein','hard','xlink','xincreg']):
    row=L(A,n); assert row.startswith('| '+t+' |'); want[t]=re.fullmatch(r'`(.*)`',row.split(' | ')[1].strip()).group(1)
want['xrefpoison']=L(A,720)[len('- `xrefpoison`: `'):-1]; xc=re.findall(r'`([^`]*)`',L(A,721))[-1]; want['xrefctl']=xc
want['lnblind2']=L(Q,79)[len('- `lnblind2`: `'):-1]
want['xrace']=xc+'\n'+L(Q,81).strip()[1:-1]
want['xbare']=re.search(r'`xbare` `([^`]*)`',L(Q,190)).group(1)
want['xanchor']=re.search(r'`xanchor` `([^`]*)`',L(Q,190)+'\n'+L(Q,191)).group(1).replace('\n',' ')
m=re.search(r'`xexecarith` `([^`]*)` then `([^`]*)`',L(Q,191)); want['xexecarith']=m.group(1)+'\n'+m.group(2)
want['gitfn']=re.findall(r'`(git\(\)[^`]*)`',L(Q,417))[0]
want['staleshim']=L(Q,422)+'\n'+L(Q,423)
bad=0
for t,v in want.items():
    got=open(f'{R}/ins/{t}').read()
    ok=(got=='') if v is None else (got==v+'\n')
    bad+=not ok; print(('ok  ' if ok else 'BAD ')+t)
print('inserts', len(want), 'checked,', bad, 'differ')
PY

echo "## r4 X5 draft-10: runbook §6.5's block and §6.6's lines before logenv B4, taken from the runbook, on scratch copies (generation only)"
SP=$O/x5; mkdir -p "$SP/p11"; git -C "$W" archive 3b7ff6d5 .claude/tools | tar -x -C "$SP/p11"
{ cat "$O/defs.sh"; echo "K=$O; SP=$O/x5"; blk 'DOC $PROV ff1322bb'
  blk 'DOC $PROV > "$SP/prov.md"' | sed '/^logenv B4/,$d'; echo 'echo "x5 build and generation: completed"'; } > "$O/x5build.sh"
/opt/homebrew/bin/bash "$O/x5build.sh"; echo "x5 build rc=$?"
head -1 "$SP/x5prep.sh"; tail -1 "$SP/x5prep.sh"
echo "cell6.sh source (PROV@ff1322bb :831 fence, :832 shebang, :845 last, :846 fence):"; for n in 831 832 845 846; do printf '  %s: %s\n' $n "$(DOC -provenance.md ff1322bb | sed -n ${n}p | cut -c1-30)"; done
echo "eval8.sh source (PROV@453b7b0f :865 fence, :866 shebang, :889 last, :890 fence; anchor count):"; for n in 865 866 889 890; do printf '  %s: %s\n' $n "$(DOC -provenance.md 453b7b0f | sed -n ${n}p | cut -c1-30)"; done
DOC -provenance.md 453b7b0f | sed -n 866,889p | $G -c -F 'BEFORE-D2|BEFORE-D4)'
echo "PROV :825 :827 (sentences), :829 :833 (the arms' fence):"; for n in 825 827 829 833; do printf '  %s: %s\n' $n "$(DOC -provenance.md | sed -n ${n}p | cut -c1-40)"; done
echo "PROV@453b7b0f :811: $(DOC -provenance.md 453b7b0f | sed -n 811p)"
chmod +x "$SP/c11/cell9.sh" "$SP/c11/eval11.sh"
for f in cell9.sh eval11.sh cellT.sh; do /bin/bash -n "$SP/c11/$f" && /opt/homebrew/bin/bash -n "$SP/c11/$f" && echo "$f: bash -n clean, 3.2 and 5.3"; done
echo "jobs by kind (kind, count):"; cut -d$'\x1f' -f3 "$SP/c11/jobs.tsv" | sort | uniq -c
awk -F'\037' '$3!="AFTER-E3g" && $3!="G"' "$SP/c11/jobs.tsv" > "$SP/c11/jobs.run"; echo "jobs run (want 32): $(wc -l < "$SP/c11/jobs.run" | tr -d ' ')"
echo "part copies: $(ls "$SP/p11/.claude/tools" | $G -c '\.m0')"
n=0; for f in "$SP"/p11/.claude/tools/*.m0*.sh; do /bin/bash -n "$f" 2>/dev/null && /opt/homebrew/bin/bash -n "$f" 2>/dev/null || n=$((n+1)); done; echo "part copies failing bash -n on 3.2 or 5.3 (want 0): $n"
echo "the K2_CORPUS hook named in T's parts (want 0): $(cat "$O"/T/*.sh | $G -c 'K2_CORPUS')"
echo "eval11 arm per kind run (REF=green; AFTER-E2|E3|SE1=hit; P=hit, label not W:*):"
$G -n -E '^    (REF\|REF5|G\|R|AFTER-E2\|AFTER-E3\|AFTER-AR\|AFTER-SE1|AFTER-E3g|P)\)' "$SP/c11/eval11.sh" | cut -c1-90
echo "labels of the P jobs: $(awk -F'\037' '$3=="P"{print $4}' "$SP/c11/jobs.tsv" | sort -u)"
echo "jobs of the run set whose env names corpus/h (want 0): $(awk -F'\037' '{print $7}' "$SP/c11/jobs.run" | $G -c 'corpus/h')"
echo "part copies that name corpus/h or r9ax3 (job tags):"
$G -l 'corpus/h\|r9ax3' "$SP"/p11/.claude/tools/*.m0*.sh | sed 's/.*trip-wire\.\(m[0-9]*\)\..*/\1/' | sort -u | tr '\n' ' '; echo
for t in $($G -l 'corpus/h\|r9ax3' "$SP"/p11/.claude/tools/*.m0*.sh | sed 's/.*trip-wire\.\(m[0-9]*\)\..*/\1/' | sort -u); do
  $G -h -o -E 'git (--git-dir="[^"]*" )?config core\.excludesFile [^ ]*(corpus/h|r9ax3)[^ ]*' "$SP"/p11/.claude/tools/*.$t.*.sh | head -1 | sed "s|$SP|\$SP|; s/^/  $t: /"; done
echo "P-g reads config --list against the reference (harness):"; $G -n -F 'config --list --show-origin > "$_pgref"' "$O/T/w.harness.sh" | cut -c1-60

echo "## r4 X5 drafts 11-21: the named runs of corpus §6.1 (:234-:490)"
C=$O/corpus.md; DOC -corpus.md > "$C"
echo "lines with the word cell(s):"; sed -n 234,490p "$C" | $G -n -i -w -E 'cells?' | cut -d: -f1 | awk '{printf "%d ", $1+233}'; echo
python3 - "$C" <<'PY'
import sys,re; L=open(sys.argv[1]).read().split('\n')
for n in (262,276,277,283,285):
    l=L[n-1]; print(n, sorted(set(re.findall(r'`(Jm|xrace|lnblind2?|xbare|xanchor|xexecarith)`',l))))
PY

echo "## gate negative control (runbook §6.0's block, taken from the runbook, with defs.sh's gate), rehearsed"
{ cat "$O/defs.sh"; blk "[ -z \"\$(gate)\" ] || { echo \"GATE NOT CLEAN BEFORE B1\""; echo 'echo "gate control: passed"'; } > "$O/gatectl.sh"
/opt/homebrew/bin/bash "$O/gatectl.sh"; echo "gate control rc=$?"
echo "## gen6.py (PROV@ff1322bb §E.3) P-row anchors in T's harness"
cd "$O" || exit 2
DOC -provenance.md ff1322bb | sed -n 743,826p > gen6.py
python3 - gen6.py "$O/T" <<'PY'
import sys,re; code=open(sys.argv[1]).read(); hs=open(sys.argv[2]+'/w.harness.sh').read(); HN='.harness'
print('parts listed:', re.search(r"parts = (\[.*?\])",code).group(1))
n=0
for line in re.search(r"REC = \[(.*?)\n\]",code,re.S).group(1).strip().split('\n'):
    name,part,old,new,needle=eval(line.strip().rstrip(',')); k=hs.count(old); n+=k>0; print(f"  {k}  {name}")
print('present', n, 'of 9')
print('corpus/h fixtures used:', sorted(set(re.findall(r"g\('([a-z0-9_.]+)'", code))))
PY
find "$H" -maxdepth 4 -type d -path '*corpus/h' | wc -l
echo "## gen6 cell counts (X2 = REF+G; X5 = REF+R+RLOUD+RES+P)"
python3 - <<'PY'
cfg=4; G=11; R=11; RL=1; RES=2; P=9; REF=1; full_extra=2*8*2
print('X2', cfg*(REF+G)); print('X5 corpus-6', cfg*(REF+R+RL+RES+P)+full_extra)
PY
echo "## corpus §6.1 table (readings: table lines, adds, rows matching surviv)"
sed -n 234,490p "$C" | $G -c '^| '; sed -n 234,490p "$C" | $G -c -E '^\| [^|]*\| \*\*add'; sed -n 234,490p "$C" | $G '^| ' | $G -c -i surviv
echo "## corpus :468, :476 (C12's 305 and 211)"
sed -n 468p "$C" | $G -c -F '**211 records of this PR, 305 in all**'
sed -n 476p "$C" | $G -c -F "bash -c '. .claude/tools/webref-generic-core-trip-wire.records.sh; _mutants | wc -l'\` (305 on bash 5.3 and 3.2"
echo "## battery logs (lines, new==base)"
for s in b53 b32; do awk '{split($2,a,"=");split($3,b,"="); n++; if(a[2]==b[2])s++} END{print FILENAME, n, s}' $H/runners/logs/dg-r29e-$s.txt; done
echo "## line counts at 3b7ff6d5 (landing, corpus, impl-d, design memo)"
for f in -landing.md -corpus.md -impl-cr2-d.md .md; do echo "$(DOC $f | wc -l) ${f}"; done
echo "## landing prose width at the edited sites (max columns; :152-:200, :318-:351, :423-:433, :444-:452)"
DOC -landing.md | awk '(NR>=152&&NR<=200||NR>=318&&NR<=351||NR>=423&&NR<=433||NR>=444&&NR<=452){if(length($0)>m)m=length($0)} END{print m}'
echo "## corpus prose width at the edited sites (max columns; :54, :242-:255, :484, :486-:488)"
DOC -corpus.md | awk '(NR==54||NR>=242&&NR<=255||NR==484||NR>=486&&NR<=488){if(length($0)>m)m=length($0)} END{print m}'
echo "## quoted lines (landing :343; residuals :93-:96; impl-d :278; impl-d :255, :282 the idiom; landing :173, :191, :337, :346)"
DOC -landing.md | sed -n 343p | $G -c 'X8 is a record that cannot fail'
DOC -residuals.md | sed -n 93,96p | $G -c -e 'a residual (§5.1,' -e 'measured boundary of what the gate decides' -e 'not deferred planned work'
DOC -impl-cr2-d.md | sed -n 278p | $G -c -F '**C4**'
DOC -impl-cr2-d.md | sed -n 255p | $G -o -E '^\*\*Declared, not pinned|\*\*(Gap|Consequence|Why here, not a residual|Trigger|Owner):\*\*' | tr '\n' ' '; echo
DOC -impl-cr2-d.md | sed -n 282p | $G -o -E "^\*\*Declared, not pinned\*\* \(§T.J's idiom" ; echo
echo "landing lines (each want 1):"
for q in '152|When T moves on #501.s branch after the squash, by a fix or by a merge' '163|a finding whose fix moves T returns to 7.1' '170|update the ledger entries with #501.s merge commit' '173|Ledger text' '191|Amend.*in two ways' '318|The checks re-run only when T moves:' '337|A fix that changes the tool code or the job' '341|a fix that moves T goes back to step 1' '346|The one ledger step' '349|Terminator:.* step 3 is dry on a head whose T is unchanged since step 1 last ran' '198|The PM lane stays the owner\.' '199|The net change is −2' '339|A docs-only fix that changes a check.s definition'; do
  printf '  :%s %s\n' "${q%%|*}" "$(DOC -landing.md | sed -n "${q%%|*}p" | $G -c -E "${q#*|}")"; done
echo "PR527 :140, :144 (design memo :109's PX1, withdrawn by draft 12; each want 1):"
DOC -pr527.md | sed -n 140p | $G -c 'withdrawn in draft 12'; DOC -pr527.md | sed -n 144p | $G -c 'draft 12 withdrew that'
echo "impl-cr2 :58, #501's CR#1 in the wire (want 1):"; DOC -impl-cr2.md | sed -n 58p | $G -c -F 'CR#1 (wire :401: `mktemp` output unchecked'
echo "## the ledger slot (memory, the shared ledger): #11-trip-wire-launch-environment line, its routing, the sibling slot"
M=/Users/kazuaki/.claude/projects/-Users-kazuaki-repos-send-sh-elidex/memory
$G -n 'trip-wire-launch-environment' $M/project_open-defer-slots.md | cut -c1-60
sed -n 307p $M/project_open-defer-slots.md | $G -o -e '\*\*Re-eval\*\*: [0-9-]*' | tr '\n' ' '; echo
sed -n 308p $M/project_open-defer-slots.md | $G -o -e 'ROUTED 2026-10-24 (PM)\*\*: owner = \*\*CI/tools lane\*\*' -e 'Re-eval stays 2026-11-30' -e '`#11-k2-fixture-postcondition-coverage-at-t`' -e 'Same owner (CI/tools), same 2026-11-30 backstop' | tr '\n' '|'; echo
$G -n 'PM owns' $M/project_pm-and-lane-driver-structure.md | cut -c1-140
echo "## design memo after the :518-:519 edit: §5's new text taken from the memo, YYYY-MM-DD = 2026-10-24, wrapped at :518's width (want <= 760); and with the memo named in the stamp"
DOC .md > "$O/dm.md"
python3 - "$O/dm.md" "$MEMO" "$O" <<'PY'
import sys,re,textwrap; p,m,o=sys.argv[1:]; L=open(p).read().split('\n'); w=len(L[517])
assert L[516].startswith('- Moving the exit below a control') and L[517].startswith('  shape is pinned by the corpus RO/ROg cells (X5')
row=next(l for l in open(m).read().split('\n') if l.startswith('| design memo :518–:519'))
new=re.match(r'"(.*?)" Wrapped', row.split(' | ')[2]).group(1).replace('YYYY-MM-DD','2026-10-24')
assert '(restated 2026-10-24)' in new; print('new text:', new)
for tag,t in (('short',new),('named',new.replace('(restated 2026-10-24)','(restated 2026-10-24, `…-cr2-plan-finalhead.md`)'))):
    M=L[:]; M[517:519]=textwrap.wrap(t,width=w,initial_indent='  ',subsequent_indent='  ',break_on_hyphens=False)
    open(f'{o}/dm-{tag}.md','w').write('\n'.join(M)); print(tag, 'width', w, 'lines of the sentence', len(M)-len(L)+2)
PY
echo "short: $(wc -l < "$O/dm-short.md")  named: $(wc -l < "$O/dm-named.md")"
echo "## X2 hits in the plan docs at 3b7ff6d5 (file: lines)"
for f in $(git -C "$W" ls-tree --name-only 3b7ff6d5 docs/plans/ | $G 'k2-fgit-hermetic'); do h=$(git -C "$W" show 3b7ff6d5:$f | $G -n -E '\bX2\b' | cut -d: -f1 | tr '\n' ' '); [ -n "$h" ] && echo "${f##*/}: $h"; done
echo "## §5 searches: the two patterns over the current-stating docs (line numbers), and over the accepted plan bodies (want 0 0 each)"
V='\bX2\b|\bX5\b|\bX14\b|\bX16\b|survive check|PDUMP|step 1'; PP='shimafter|shimhash|shimprepend|mkctl|PX1|ROg|RO/ROg'
for f in .md -landing.md -corpus.md -residuals.md -census.md; do echo "${f:-.md} V: $(DOC $f | $G -n -E "$V" | cut -d: -f1 | tr '\n' ' ')| P: $(DOC $f | $G -n -E "$PP" | cut -d: -f1 | tr '\n' ' ')"; done
U=docs/plans/2026-07-citation-hygiene-umbrella.md; echo "umbrella V: $(git -C "$W" show 3b7ff6d5:$U | $G -c -E "$V") P: $(git -C "$W" show 3b7ff6d5:$U | $G -c -E "$PP")"
for s in cr2-plan cr2-plan-exitpath cr2-plan-liveness cr2-plan-liveness-2 cr-plan; do
  echo "$s $(DOC -$s.md | $G -c -E "$V") $(DOC -$s.md | $G -c -E "$PP") | X3: $(DOC -$s.md | $G -n -E '\bX3\b' | cut -d: -f1 | tr '\n' ' ')"; done
echo "## stamps: 'restated 20' in the edited files (want 0 each) and in residuals"
for f in -landing.md -corpus.md .md -cr2-sweep.md; do echo "$f $(DOC $f | $G -c 'restated 20')"; done
echo "umbrella $(git -C "$W" show 3b7ff6d5:$U | $G -c 'restated 20')"; echo "residuals $(DOC -residuals.md | $G -c 'restated 20')"
echo "## X14 callers' last runs (corpus :192-:193; PR527 :50-:51, :93, :100, :130, :290-:296) and ancestry"
sed -n 192,193p "$C" | $G -o -E 'Codex R23.s shape|red on `cb8b0e09`, PASSED after' | sed 's/^/  corpus :192-:193: /'
DOC -pr527.md | sed -n 50,51p | $G -o -E 'P-e red on `da2730d4` and PASSED after|saw 97 calls' | sed 's/^/  PR527 :50: /'
DOC -pr527.md | sed -n 93p | $G -o 'Codex R23\*\* (on `cb8b0e09`)' | sed 's/^/  PR527 :93: /'; DOC -pr527.md | sed -n 100p | $G -o 'Codex R24\*\* (on `0116209d`)' | sed 's/^/  PR527 :100: /'
DOC -pr527.md | sed -n 130p | $G -o 'draft 11: M-PATH' | sed 's/^/  PR527 :130: /'; DOC -pr527.md | sed -n 290,291p | $G -o -E 'In `\$A/sb` \(M-PATH\)|rc 0, PASSED' | sed 's/^/  PR527 :290-:291: /'
for c in 0116209d f8a5f3e6; do echo "  subject $c: $(git -C "$W" log -1 --format=%s $c)"; done
for c in cb8b0e09 da2730d4 0116209d f8a5f3e6; do git -C "$W" merge-base --is-ancestor $c 3b7ff6d5 && echo "$c ancestor of 3b7ff6d5"; done
echo "## wire groups at T (ctlrun :348 :352 :372; controls :551 :559; each want 1)"
sed -n 348p "$O/T/w.ctlrun.sh" | $G -c '^  set -m$'; sed -n 352p "$O/T/w.ctlrun.sh" | $G -c -F '"$BASH" "$SELF" --selftest'; sed -n 372p "$O/T/w.ctlrun.sh" | $G -c 'k2wd'
sed -n 551p "$O/T/w.controls.sh" | $G -c -F "( trap '' TERM; : > \"\$_VFY/.og_dr\"; exec sleep 10 )"; sed -n 559p "$O/T/w.controls.sh" | $G -c -E 'sleep 0\.2 &.*sleep 5 &.*exec sleep 5'
echo "## sweep.sh's file list: does the glob take, and the case drop, the final-head files? (…-cr2-sweep.md :63-:72)"
DOC -cr2-sweep.md | sed -n 64p | $G -o 'docs/plans/\*k2-fgit\*'
DOC -cr2-sweep.md | sed -n 72p | $G -o 'case "$f" in [^)]*)'
python3 - <<'PY'
from fnmatch import fnmatch
for f in ('docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-cr2-plan-finalhead.md','docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-cr2-plan-finalhead-runbook.md','docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-cr2-plan-finalhead-check.sh'):
    print(f.split('/')[-1], 'glob', fnmatch(f,'docs/plans/*k2-fgit*'), 'dropped', fnmatch(f,'docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-cr2-*'))
PY
echo "## cr2-plan banner: the CR2-Z row at 3b7ff6d5 and as accepted (b85b67e3); the banner's first and last line"
git -C "$W" show 3b7ff6d5:$DP-cr2-plan.md | $G -n -F 'X3 and the rest of' | cut -d: -f1
git -C "$W" show b85b67e3:$DP-cr2-plan.md | $G -n -F 'X3 and the rest of' | cut -d: -f1
git -C "$W" show 3b7ff6d5:$DP-cr2-plan.md | sed -n 18p | cut -c1-24; printf '  :33 empty: %s; notes :19-:32: %s\n' "$(git -C "$W" show 3b7ff6d5:$DP-cr2-plan.md | sed -n 33p | wc -c | tr -d ' ')" "$(git -C "$W" show 3b7ff6d5:$DP-cr2-plan.md | sed -n 19,32p | $G -c '^- \*\*:')"
echo "## umbrella count (umbrella :176's command on a scratch clone at 3b7ff6d5 with the memo, runbook and checker added; per file exit code)"
UC=$O/um; git clone -q --local "$W" "$UC" && git -C "$UC" checkout -q --detach 3b7ff6d5 || echo "UMBRELLA CLONE FAILED"
git -C "$UC" show 3b7ff6d5:docs/plans/2026-07-citation-hygiene-umbrella.md | sed -n 176p | $G -o -F "git ls-files 'docs/plans/*.md' | xargs -n1 python3 .claude/skills/elidex-plan-review/preflight.py"
cp "$MEMO" "$UC/$DP-cr2-plan-finalhead.md"; cp "$RB" "$UC/$DP-cr2-plan-finalhead-runbook.md"; cp "$SELFP" "$UC/$DP-cr2-plan-finalhead-check.sh"
git -C "$UC" add docs/plans
( cd "$UC" && git ls-files 'docs/plans/*.md' | while read -r f; do python3 .claude/skills/elidex-plan-review/preflight.py "$f" > /dev/null 2>&1; echo "$? $f"; done ) > "$O/umbrella-exits.txt"
echo "all tracked plan memos: $(wc -l < "$O/umbrella-exits.txt" | tr -d ' '); the checker taken: $($G -c 'check\.sh' "$O/umbrella-exits.txt")"
$G 'k2-fgit-hermetic' "$O/umbrella-exits.txt" > "$O/umbrella-k2.txt"
echo "k2-fgit-hermetic files: $(wc -l < "$O/umbrella-k2.txt" | tr -d ' '); exit 1: $($G -c '^1 ' "$O/umbrella-k2.txt"); exit 0: $($G -c '^0 ' "$O/umbrella-k2.txt"); other: $($G -v -c '^[01] ' "$O/umbrella-k2.txt")"
echo "exit 0:"; $G '^0 ' "$O/umbrella-k2.txt" | sed 's|.*/||; s/^/  /'
echo "the runbook: $($G 'finalhead-runbook' "$O/umbrella-k2.txt" | cut -d' ' -f1)"
rm -rf "$UC"
echo "## residual rows R24-R28 cited by the memo (want 0)"
$G -v '^| C15 ' "$MEMO" | $G -c -E '\*\*R2[4-8] |residuals.md` R2[4-8]|residuals? R2[4-8]|\(R2[4-8]\)|R24–R28'   # C15's own row names the patterns
echo "## accounting sites: the search over the k2-fgit-hermetic docs and the umbrella at 3b7ff6d5 (file: lines)"
for f in $(git -C "$W" ls-tree --name-only 3b7ff6d5 docs/plans/ | $G -E 'k2-fgit-hermetic|2026-07-citation-hygiene-umbrella\.md'); do
  h=$(git -C "$W" show 3b7ff6d5:$f | $G -n -i -E 'own deferral|own slots|nets? to|net change|−2([^0-9]|$)' | cut -d: -f1 | tr '\n' ' '); [ -n "$h" ] && echo "${f##*/}: $h"; done
echo "the ASCII spelling, -2 after a space, ( , * or a line start (file: lines; each hit an identifier, a list suffix or head -2, read by hand):"
for f in $(git -C "$W" ls-tree --name-only 3b7ff6d5 docs/plans/ | $G -E 'k2-fgit-hermetic|2026-07-citation-hygiene-umbrella\.md'); do
  h=$(git -C "$W" show 3b7ff6d5:$f | $G -n -E '(^|[ (*,`])-2([^0-9]|$)' | cut -d: -f1 | tr '\n' ' '); [ -n "$h" ] && echo "${f##*/}: $h"; done
echo "residuals :90 / :92, landing :271 (each want 1):"
DOC -residuals.md | sed -n 90p | $G -c -F '**Accounting:** 1 own deferral, `#11-trip-wire-liveness-bound`'
DOC -residuals.md | sed -n 92p | $G -c -F 'the ledger nets to **−2**.'
DOC -landing.md | sed -n 271p | $G -c -x -F -e "- **Accounting**: #527's own deferrals, 1."
echo "residuals line count at 3b7ff6d5 (want 152): $(DOC -residuals.md | wc -l | tr -d ' ')"
DOC -residuals.md > "$O/res.md"
python3 - "$O/res.md" "$MEMO" <<'PY'
import sys,re,itertools; L=open(sys.argv[1]).read().split('\n'); m=open(sys.argv[2]).read().split('\n')
w=max(len(L[i]) for i in range(89,96)); print('paragraph :90-:96 max width', w)
row=next(l for l in m if l.startswith('| residuals :90–:93 '))
new=re.match(r'"(.*?)" Wrapped', row.split(' | ')[2]).group(1).replace('YYYY-MM-DD','2026-10-24')
assert '(restated 2026-10-24)' in new and 'nets to **−1**' in new and '2 own deferrals' in new
t=(new+' '+L[92]).split(' '); n=len(t); best=None
for c in itertools.combinations(range(1,n),3):
    parts=[' '.join(t[a:b]) for a,b in zip((0,)+c,c+(n,))]; mx=max(map(len,parts))
    if best is None or mx<best[0]: best=(mx,parts)
print('residuals :90-:92 new text with :93 rewrapped, 4 lines, max width', best[0], '(want <=', w, ')'); print('\n'.join('  '+x for x in best[1]))
N=L[:89]+best[1]+L[93:]; print('residuals after the edit:', len(N)-1 if N[-1]=='' else len(N), 'lines; :93 ends as before:', N[92].endswith('a residual (§5.1,') and L[92].endswith('a residual (§5.1,'), '; :94 onward unchanged:', N[93:]==L[93:])
row=next(l for l in m if l.startswith('| landing :271 '))
new=re.match(r'"(.*?)" One line', row.split(' | ')[2]).group(1).replace('YYYY-MM-DD','2026-10-24')
print('landing :271 new text:', new); print('  one line,', len(new), 'columns')
PY
echo "## L3 trigger clauses: corpus §6.1 rows naming W and P-a/P-b liveness; the records at T"
python3 - "$C" <<'PY'
import sys; L=open(sys.argv[1]).read().split('\n')
for n in (285,307,312): print(f'corpus :{n} clause column:', L[n-1].split(' | ')[-3])
print('corpus :288 names P-a liveness, P-b liveness:', 'P-a liveness, P-b liveness' in L[287], '; clause column:', L[287].split(' | ')[-3])
PY
echo "records labelled 'the fixture build window completed' at T: $($G -n -E $'\tthe fixture build window completed$' "$O/T/w.records.sh" | cut -d: -f1 | tr '\n' ' ')"
echo "controls _fw_lbl/_pal_lbl/_pbl_lbl:"; $G -n -E '^_(fw|pal|pbl)_lbl=' "$O/T/w.controls.sh"
for n in 174 176; do printf 'records :%s label: %s\n' $n "$(sed -n ${n}p "$O/T/w.records.sh" | awk -F'\t' '{print $NF}')"; done
echo "## memo line count"; wc -l < "$MEMO"
echo "## runbook line count"; wc -l < "$RB"
true
