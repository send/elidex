# K2 fixture git — PR #527 CR2 round: the instruments (as run), §E.3, 2 of 2 — the exit path's texts [r4p], [r5p]

Companion of `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-cr2-sweep.md` ("the sweep file"), which holds CR2's sweep, §E.1 and §E.4,
and whose §E index names the file of every instrument. The second half of §E.3 was split out of it at CR2-M (touch-time, the
1000-line rule), each block byte-identical. The plan is `…-cr2-plan.md`, with its companion
`…-cr2-plan-exitpath.md` (written there as x§6.n). **Frame `706049ad`.** How a `cr2/bin/<name>` resolves, how a
block is re-run and what it reads: the sweep file's header and §E.4. This file is excluded from CR2's sweep and
from the plan's claim rows over `docs/plans/*k2-fgit*`, as the sweep file is (the plan's §10 and §11).

## §0 Spec coverage map

**No spec surface** — as the plan's §0: this file holds a shell trip-wire's instruments, as run. `preflight.py`
exits 1 here by design, because no table follows this heading.

## §E Appendix — the instruments (as run), continued

One fenced block per file, under its file name, as in the sweep file's §E.

### §E.3 The exit path's texts and their batches (2 of 2): [r4p], [r5p]

#### §E.r4c2-fix3.py

`r4c2-fix3.py` (sha256 `d50440af1288588c`, 118 lines)

```python
import sys
# r4c2-fix3.py <clone> [variant ...]: applied AFTER bin/r3c2-fix2.py — the r4 feasibility text "[r4p]" is
# r3c2-proto.py, r3c2-fix1.py, r3c2-fix2.py, r4c2-fix3.py in sequence. A FEASIBILITY PROTOTYPE for plan-review R3's
# H1/H2/H3/H4/H17 (not the slice's text; C2 measures its own). FROZEN after its measurements (logs/r4/).
#  H1 : every reap set the run makes is the run's own (`$_K2_REAP_D`) or one directory inside it (`_og`'s is
#       `$_K2_REAP_D/og`); `_k2_reap` reads both levels, so no set exists that the run's end does not read.
#  H3 : an entry holds the `$SECONDS` of its record; `_k2_reap` acts on an entry only when its group still exists and
#       either has no live leader (an id is not reused while its group exists) or the leader's `ps -o etime=` says it
#       started no later than the record (1 s granularity); anything else (no `ps`, an unreadable entry) is skipped.
#  H2 : `_ctl_run` ends with `_k2_end`: the watchdog KILLed and waited, then each group KILLed again until
#       `kill -0 -<pgid>` fails (bounded, 20 x 0.05 s), its entry dropped only then; a group still there is listed in
#       `_ctl_left` and keeps its entry; the `_wm` check reads `_ctl_left` (the same confirmation) instead of its own poll.
#       Drop failures continue past the first (H3) and the call `exit 2`s after the loop.
#  H17: `_k2_reap_add`'s failure arm restores `set +m`, stops first, removes the recorded entries after.
#  H4 : the `_og` check gains a fourth row, `sig`: the call runs in a subshell with the wire's EXIT and TERM traps and a
#       scratch dir of its own; the stub (once both its groups are recorded) sends TERM to that subshell; the subshell
#       must end 143 with its line, its scratch dir gone and every recorded group gone. Record R11r (`_k2_exit`'s
#       `_k2_reap` -> `:`) must be killed by it. The row stands down when TERM cannot reach a child (`_wm_tg`).
#       The kill row's stub (and sig's) first waits until both groups' entries hold their content (see below).
# Variants: og-nosig (the sig row's assertions removed: R11r's survive arm), noreap (as r3's), killonce (`_k2_end`
#  without its re-KILL loop: H2's comparison arm).
d = sys.argv[1]; T = d + "/.claude/tools/webref-generic-core-trip-wire"
def rep(p, a, b, n=1):
    s = open(p).read(); assert s.count(a) == n, (p, s.count(a), a[:70]); open(p, "w").write(s.replace(a, b))
W = T + ".sh"; C = T + ".ctlrun.sh"; K = T + ".controls.sh"; R = T + ".records.sh"
OGL = "a call's process groups end with it and hold none of its caller's streams"
CEN = "every label the run used is named by a mutation record"
# --- wire: _k2_reap (H1, H3), _k2_reap_add (H17), _k2_end replaces _k2_reap_del (H2)
rep(W, '_k2_reap() { local _kr_f _kr_p; local -a _kr_g=()\n'
       '  [ -n "$_K2_REAP_D" ] || return 0\n'
       '  for _kr_f in "$_K2_REAP_D"/*; do _kr_p="${_kr_f##*/}"\n'
       '    case "$_kr_p" in \'\'|*[!0-9]*|0*|1) continue ;; esac\n'
       '    _kr_g+=("$_kr_p"); done\n',
       '_k2_reap() { local _kr_f _kr_p _kr_t; local -a _kr_g=()\n'
       '  [ -n "$_K2_REAP_D" ] || return 0\n'
       '  for _kr_f in "$_K2_REAP_D"/* "$_K2_REAP_D"/*/*; do _kr_p="${_kr_f##*/}"\n'
       '    case "$_kr_p" in \'\'|*[!0-9]*|0*|1) continue ;; esac\n'
       '    kill -0 -"$_kr_p" 2>/dev/null || continue\n'
       '    if kill -0 "$_kr_p" 2>/dev/null; then _kr_t=""; read -r _kr_t 2>/dev/null < "$_kr_f"\n'
       '      case "$_kr_t" in \'\'|*[!0-9]*) continue ;; esac\n'
       '      ps -o etime= -p "$_kr_p" 2>/dev/null | _K2_AGE=$((SECONDS - _kr_t - 1)) awk \'{ n = split($1, f, /[-:]/)\n'
       '        split("1 60 3600 86400", u, " "); for (i = n; i > 0; i--) s += f[i] * u[n - i + 1]; ok = n >= 2 && n <= 4 && $1 ~ /^[0-9:-]+$/ }\n'
       '        END { exit !(ok && s >= ENVIRON["_K2_AGE"]) }\' || continue; fi\n'
       '    _kr_g+=("$_kr_p"); done\n')
rep(W, '  local _ka_p; [ -n "$_K2_REAP_D" ] && : > "$_K2_REAP_D/$2" 2>/dev/null && return 0\n'
       '  echo "!! $1: the reap set could not record process group $2, so a signal would leave it running; decided nothing" >&9\n'
       '  for _ka_p in "${@:3}"; do command rm -f "$_K2_REAP_D/$_ka_p" 2>/dev/null; done\n'
       '  _k2_stop "${@:2}"; exit 2; }\n',
       '  local _ka_p; [ -n "$_K2_REAP_D" ] && printf \'%s\\n\' "$SECONDS" 2>/dev/null > "$_K2_REAP_D/$2" && return 0\n'
       '  set +m; echo "!! $1: the reap set could not record process group $2, so a signal would leave it running; decided nothing" >&9\n'
       '  _k2_stop "${@:2}"; for _ka_p in "${@:3}"; do command rm -f "$_K2_REAP_D/$_ka_p" 2>/dev/null; done; exit 2; }\n')
rep(W, '_k2_reap_del() { # $1 = the label, $2... = the call\'s groups, stopped\n'
       '  local _kd_p; for _kd_p in "${@:2}"; do command rm -f "$_K2_REAP_D/$_kd_p" 2>/dev/null && continue\n'
       '    echo "!! $1: the reap set could not drop process group $_kd_p, so the run\'s end could stop another\'s; decided nothing" >&9; exit 2; done; }\n',
       '_k2_end() { # $1 = the label, $2 = the child\'s group (its leader waited), $3 = the watchdog\'s: each KILLed until empty, then dropped\n'
       '  local _ke_p _ke_i _ke_f=0; _ctl_left=""; kill -9 -"$3" 2>/dev/null || true; wait "$3" 2>/dev/null || true\n'
       '  for _ke_p in "${@:2}"; do _ke_i=0\n'
       '    while kill -0 -"$_ke_p" 2>/dev/null && ! kill -0 "$_ke_p" 2>/dev/null; do\n'
       '      [ "$_ke_i" -lt 20 ] || { _ctl_left="$_ctl_left $_ke_p"; continue 2; }\n'
       '      kill -9 -"$_ke_p" 2>/dev/null || true; sleep 0.05 || sleep 1; _ke_i=$((_ke_i + 1)); done\n'
       '    command rm -f "$_K2_REAP_D/$_ke_p" 2>/dev/null && continue; _ke_f=1\n'
       '    echo "!! $1: the reap set could not drop process group $_ke_p, so the run\'s end could stop another\'s; decided nothing" >&9; done\n'
       '  [ "$_ke_f" -eq 0 ] || exit 2; }\n')
rep(C, '  kill -9 -"$_wpid" 2>/dev/null || true; wait "$_wpid" 2>/dev/null || true; _k2_reap_del "$_r_for" "$_cpid" "$_wpid"\n',
       '  _k2_end "$_r_for" "$_cpid" "$_wpid"\n')
# --- controls: `_wm` reads the call's own confirmation (H2)
rep(K, '           _wm_gone "-$_ctl_wd_pg" || _wm_bad="$_wm_bad; $_wm_row: the watchdog\'s process group outlived the call"\n',
       '           case " ${_ctl_left-} " in *" $_ctl_wd_pg "*) _wm_bad="$_wm_bad; $_wm_row: the watchdog\'s process group outlived the call" ;; esac\n')
# --- controls: `_og` (H1: its set inside the run's; H4: the sig row)
rep(K, "printf '%s\\n' 'e=\"$2/.og_reap/$$\"; while [ ! -f \"$e\" ]; do sleep 0.05; done; rm -f \"$e\"; mkdir \"$e\"' > \"$_VFY/.og_nodel\"\n",
       "printf '%s\\n' 'e=\"$2/.reap/og/$$\"; while [ ! -f \"$e\" ]; do sleep 0.05; done; rm -f \"$e\"; mkdir \"$e\"' > \"$_VFY/.og_nodel\"\n"
       "printf '%s\\n' 'd=\"$2/.reap/og\"; until n=0; for f in \"$d\"/*; do [ ! -s \"$f\" ] || n=$((n + 1)); done; [ \"$n\" -ge 2 ]; do sleep 0.05; done; kill -TERM $PPID; sleep 5' > \"$_VFY/.og_sig\"\n")
# the kill row's stub waits the same way: both groups recorded (an entry's content written) before the SIGKILL, so the
# row always has a launched, recorded watchdog to test and never kills the recorder mid-write (an empty entry is skipped).
rep(K, "printf '%s\\n' 'kill -KILL $PPID' > \"$_VFY/.og_kill\"\n",
       "printf '%s\\n' 'd=\"$2/.reap/og\"; until n=0; for f in \"$d\"/*; do [ ! -s \"$f\" ] || n=$((n + 1)); done; [ \"$n\" -ge 2 ]; do sleep 0.05; done; kill -KILL $PPID' > \"$_VFY/.og_kill\"\n")
rep(K, 'command rm -rf "$_VFY/.og_end" "$_VFY/.og_reap"\n', 'command rm -rf "$_VFY/.og_end" "$_K2_REAP_D/og" "$_VFY/.og_scr"\n')
rep(K, '         for _og_row in kill norec nodel; do\n'
       '           command rm -rf "$_VFY/.og_reap"; mkdir "$_VFY/.og_reap"; SELF="$_VFY/.og_$_og_row"; _og_t0=$SECONDS\n'
       '           _og_rd="$_VFY/.og_reap"; [ "$_og_row" != norec ] || _og_rd="$_VFY/.og_none"\n'
       '           _og_st="$( ( _ctl_wd_s=5; _K2_REAP_D="$_og_rd"; _ctl_run',
       '         _og_trap() { SCRATCH="$_VFY/.og_scr"; mkdir -p "$SCRATCH"; trap \'_k2_rc=$?; _k2_exit "$_k2_rc"\' EXIT; trap \'_k2_sig TERM 143\' TERM; }\n'
       '         for _og_row in kill norec nodel sig; do\n'
       '           [ "$_og_row" != sig ] || [ "$_wm_tg" -eq 1 ] || continue\n'
       '           command rm -rf "$_K2_REAP_D/og"; mkdir "$_K2_REAP_D/og"; SELF="$_VFY/.og_$_og_row"; _og_t0=$SECONDS\n'
       '           _og_rd="$_K2_REAP_D/og"; [ "$_og_row" != norec ] || _og_rd="$_VFY/.og_none"\n'
       '           _og_st="$( ( _ctl_wd_s=5; _K2_REAP_D="$_og_rd"; [ "$_og_row" != sig ] || _og_trap; _ctl_run')
rep(K, '               ( _K2_REAP_D="$_VFY/.og_reap"; _k2_reap )\n'
       '               for _og_f in "$_VFY/.og_reap"/*; do',
       '               ( _K2_REAP_D="$_og_rd"; _k2_reap )\n'
       '               for _og_f in "$_og_rd"/*; do')
SIG = ('             sig)\n'
       '               case "$_og_st" in *"stopped by SIGTERM"*"ended 143") ;; *) _og_bad="$_og_bad; sig: the signalled call did not end 143 with its line ($_og_st)" ;; esac\n'
       '               [ ! -e "$_VFY/.og_scr" ] || _og_bad="$_og_bad; sig: the signalled call\'s exit path left its scratch dir"\n'
       '               _og_n=0; for _og_f in "$_og_rd"/*; do _og_p="${_og_f##*/}"; case "$_og_p" in \'\'|*[!0-9]*) continue ;; esac; _og_n=$((_og_n + 1))\n'
       '                 _og_gone "-$_og_p" || _og_bad="$_og_bad; sig: the exit path left group $_og_p running"; done\n'
       '               [ "$_og_n" -eq 2 ] || _og_bad="$_og_bad; sig: the reap set held $_og_n groups, not 2" ;;\n')
rep(K, '             *) _og_bad="$_og_bad; no arm for row \\"$_og_row\\"" ;;\n',
       SIG + '             *) _og_bad="$_og_bad; no arm for row \\"$_og_row\\"" ;;\n')
rep(K, '         command rm -rf "$_VFY/.og_reap"\n         : > "$_VFY/.og_end"\n',
       '         command rm -rf "$_K2_REAP_D/og"\n         : > "$_VFY/.og_end"\n')
rep(K, '"$_VFY/.og_nodel" "$_VFY/.og_err" "$_VFY/.og_reap" "$_VFY/.control_timeout"\n',
       '"$_VFY/.og_nodel" "$_VFY/.og_sig" "$_VFY/.og_scr" "$_VFY/.og_err" "$_K2_REAP_D/og" "$_VFY/.control_timeout"\n')
# --- records: re-anchored R11a, R11x, R11g; new R11r
rep(R, 's/^  _k2_stop "[$]{@:2}"; exit 2; }$/  return 0; }/\t' + OGL + '\n',
       's/^  _k2_stop "[$]{@:2}"; for _ka_p in "[$]{@:3}"; do command rm -f "[$]_K2_REAP_D\\/[$]_ka_p" 2>\\/dev\\/null; done; exit 2; }$/  return 0; }/\t' + OGL + '\n')
rep(R, 's/; decided nothing" >&9; exit 2; done; }$/; decided nothing" >\\&9; done; }/\t' + OGL + '\n',
       's/^  \\[ "[$]_ke_f" -eq 0 \\] || exit 2; }$/  :; }/\t' + OGL + '\n')
rep(R, 'controls:s/^         for _og_row in kill norec nodel; do$/         for _og_row in; do/\t' + CEN + '\n',
       'controls:s/^         for _og_row in kill norec nodel sig; do$/         for _og_row in; do/\t' + CEN + '\n'
       's/^  _k2_reap$/  :/\t' + OGL + '\n')
for v in sys.argv[2:]:
    if v == "og-nosig": rep(K, SIG, '             sig) ;;\n')
    elif v == "noreap": rep(W, "  _k2_reap\n  case", "  : _k2_reap\n  case")
    elif v == "killonce":
        rep(W, '      kill -9 -"$_ke_p" 2>/dev/null || true; sleep 0.05 || sleep 1; _ke_i=$((_ke_i + 1)); done\n',
               '      _ke_i=20; done\n')
    else: raise SystemExit("unknown variant " + v)
```

#### §E.r4-clone.sh

`r4-clone.sh` (sha256 `7a59002044e1ab63`, 13 lines)

```bash
#!/bin/bash
# r4-clone.sh <name> [fix3-variant ...] [-- instrument ...]: a fresh `git clone --local` of cr2-wt at 706049ad under
# work/<name> with the r4 feasibility text [r4p] = r3c2-proto.py, r3c2-fix1.py, r3c2-fix2.py, r4c2-fix3.py (+ variants);
# instruments by r3-instr.py. Both shells' `-n` over every part. Prints the clone path last.
set -eu
H=/Users/kazuaki/repos/send.sh/elidex-fgit-handoff/cr2; n="$1"; shift
v=(); i=()
while [ $# -gt 0 ]; do [ "$1" = -- ] && { shift; i=("$@"); break; }; v+=("$1"); shift; done
d=$("$H/bin/r3-clone.sh" "$n" r3f | tail -n 1)
python3 -I "$H/bin/r4c2-fix3.py" "$d" ${v[@]+"${v[@]}"}
[ ${#i[@]} -eq 0 ] || python3 -I "$H/bin/r3-instr.py" "$d" "${i[@]}"
for f in "$d"/.claude/tools/webref-generic-core-trip-wire*.sh; do /opt/homebrew/bin/bash -n "$f"; /bin/bash -n "$f"; done
echo "$d"
```

#### §E.r4-batch.sh

`r4-batch.sh` (sha256 `3da869004c59d3a6`, 21 lines)

```bash
#!/bin/bash
# r4-batch.sh: the wire runs on the r4 feasibility text [r4p] (bin/r4-clone.sh), each item b53 ∥ b32 (two wire
# processes at a time). Logs logs/r4/<item>.<shell>.log; marker logs/r4/R4-BATCH-DONE.
H=/Users/kazuaki/repos/send.sh/elidex-fgit-handoff/cr2; B=$H/bin; L=$H/logs/r4
cl() { "$B/r4-clone.sh" "$@" | tail -n 1; }
both() { local f="$1"; shift; for s in b53 b32; do "$f" "$s" "$@" & done; wait; echo "$(date +%T) done $*" >> "$L/R4-BATCH.progress"; }
ship() { "$B/r3-runsig.sh" "$(cl "r4-$2-$1" "${@:4}")" "$1" "$L/$2.$1.log" "$3"; }          # ship <s> <item> <disp> [variants]
opt()  { "$B/r3-optin.sh" "$(cl "r4-$2-$1" "${@:5}")" "$1" "$L/$2.$1.log" "$3" "$4"; }      # opt <s> <item> <line> <disp> [variants]
: > "$L/R4-BATCH.progress"; rm -f "$L/R4-BATCH-DONE"
both ship r4-ship default
both ship r4-shipignTERM ignTERM
both ship r4-noreapship default noreap
both opt r4-optR11r 480 default
both opt r4-survR11r 480 default og-nosig
both opt r4-optR11a 476 default
both opt r4-optR11x 477 default
both opt r4-optR11g 479 default
both opt r4-optR11d 475 default
both opt r4-optR11e 478 default
both opt r4-optR11w 474 default
echo R4-BATCH-DONE > "$L/R4-BATCH-DONE"
```

#### §E.r4probe.sh

`r4probe.sh` (sha256 `f191f4bf0cc454e0`, 46 lines)

```bash
#!/bin/bash
# r4probe.sh <r4p clone> <out>: [synthetic harness, the clone's own exit block] the r4 feasibility text's reap identity
# (H3), its call-end stop (H2) and its add-failure arm (H17), one fresh shell per case under `set -euo pipefail`, both
# shells, each case its own process group with a 60 s kill -9 watchdog. Cases:
#  P1 own live group (entry = $SECONDS at its record)            -> stopped
#  P2 a live leader that started 10 s after its entry's record   -> skipped (alive after: never a wrong kill)
#  P3 a group whose leader is gone (members live; entry unreadable)-> stopped
#  P4 an unreadable entry (a directory) with a live leader       -> skipped
#  P5 P1 with no `ps` on PATH                                     -> skipped (the identity is unknown)
#  P6 _k2_end, the child's entry undroppable (a directory)        -> rc 2, that group named, the watchdog's entry dropped (the loop continues)
#  P7 _k2_end, a child whose leader exited leaving a live member  -> the member KILLed, entry dropped, _ctl_left empty
#  P8 _k2_reap_add failing under set -m, called as the wire calls it (an || operand) -> rc 2, the group gone
#  P9 five own live groups                                        -> stopped; the exit path's time
H=/Users/kazuaki/repos/send.sh/elidex-fgit-handoff/cr2; O="$2"; : > "$O"
B="$(awk '/^# R3PROTO-EXIT-BEGIN$/{on=1; next} /^# R3PROTO-EXIT-END$/{on=0} on' "$1/.claude/tools/webref-generic-core-trip-wire.sh")"
Wk="$H/work/r4probe"; mkdir -p "$Wk"
rs='_K2_REAP_D="$SCRATCH/v/.reap"; mkdir -p "$_K2_REAP_D"'
rec='printf "%s\n" "$SECONDS" > "$_K2_REAP_D/$p"'
pid='echo $p >> "'"$Wk"'/pids"'
body() { case "$1" in
  P1) echo "$rs; set -m; sleep 30 & p=\$!; set +m; $pid; $rec; exit 3" ;;
  P2) echo "$rs; set -m; sleep 30 & p=\$!; set +m; $pid; printf '%s\n' \"\$((SECONDS - 10))\" > \"\$_K2_REAP_D/\$p\"; exit 3" ;;
  P3) echo "$rs; set -m; ( sleep 30 & echo \$! >> \"$Wk/pids\"; exec sleep 30 ) & p=\$!; set +m; sleep 0.3; mkdir \"\$_K2_REAP_D/\$p\"; kill -9 \$p; wait \$p 2>/dev/null || :; exit 3" ;;
  P4) echo "$rs; set -m; sleep 30 & p=\$!; set +m; $pid; mkdir \"\$_K2_REAP_D/\$p\"; exit 3" ;;
  P5) echo "$rs; set -m; sleep 30 & p=\$!; set +m; $pid; $rec; PATH=/nonexistent-k2; exit 3" ;;
  P6) echo "$rs; set -m; ( exit 0 ) & c=\$!; sleep 30 & w=\$!; set +m; echo \$w >> \"$Wk/pids\"; mkdir \"\$_K2_REAP_D/\$c\"; p=\$w; $rec; wait \$c || :; _k2_end lbl \$c \$w; exit 0" ;;
  P7) echo "$rs; set -m; ( sleep 30 & echo \$! >> \"$Wk/pids\"; exit 0 ) & c=\$!; sleep 30 & w=\$!; set +m; echo \$w >> \"$Wk/pids\"; for p in \$c \$w; do $rec; done; wait \$c || :; sleep 0.2; _k2_end lbl \$c \$w; echo \"left=[\${_ctl_left}] entries=[\$(ls \"\$_K2_REAP_D\" | tr '\n' ' ')]\" >&2; _K2_ORDERLY=1; exit 0" ;;
  P8) echo "$rs; set -m; sleep 30 & p=\$!; $pid; _K2_REAP_D=/nonexistent-k2/r; _k2_reap_add lbl \$p || :; exit 0" ;;   # `|| :`: every caller of _ctl_run is an || / if ! operand (errexit off there)
  P9) echo "$rs; set -m; for i in 1 2 3 4 5; do sleep 30 & p=\$!; $pid; $rec; done; set +m; exit 3" ;;
esac; }
now() { perl -MTime::HiRes=time -e 'printf "%.2f\n", time'; }
for sh in /opt/homebrew/bin/bash /bin/bash; do for k in P1 P2 P3 P4 P5 P6 P7 P8 P9; do
  S="$Wk/s.$$"; rm -rf "$S" "$Wk/pids" "$Wk/out"; mkdir -p "$S"
  t0="$(now)"; rc=0
  perl -e '$SIG{INT}="DEFAULT"; $SIG{TERM}="DEFAULT"; $SIG{HUP}="DEFAULT"; setpgrp; alarm 60; exec @ARGV' "$sh" -c "set -euo pipefail
SCRATCH=\"$S\"
$B
$(body $k)" > "$Wk/out" 2>&1 & pg=$!
  perl -e 'sleep 60; kill 9, -$ARGV[0]' "$pg" </dev/null >/dev/null 2>&1 & wd=$!
  wait "$pg" || rc=$?; kill "$wd" 2>/dev/null; t1="$(now)"
  sleep 0.3; left=""
  for p in $(cat "$Wk/pids" 2>/dev/null); do kill -0 "$p" 2>/dev/null && { left="$left ALIVE:$p"; kill -9 "$p" 2>/dev/null; }; done
  printf '%s %-3s rc=%s %.2fs%s out=[%s]\n' "$($sh -c 'echo ${BASH_VERSION%%(*}')" "$k" "$rc" "$(T0="$t0" T1="$t1" awk 'BEGIN{print ENVIRON["T1"]-ENVIRON["T0"]}')" "${left:- none-alive}" "$(tr '\n' '|' < "$Wk/out" | cut -c1-220)" >> "$O"
  rm -rf "$S"
done; done
echo R4PROBE-DONE >> "$O"
```

#### §E.r4race.sh

`r4race.sh` (sha256 `10ae75f35ac5aac2`, 34 lines)

```bash
#!/bin/bash
# r4race.sh <frame|r3|r4> <N> [load]: (from plan-review R3 Axis 2's race.sh) a `_ctl_run`-shaped launch whose child
# KILLs itself at once, then the call's end: frame/r3 = one `kill -9 -<watchdog>` + wait (the frame's and [r3f]'s);
# r4 = [r4p]'s `_k2_end` (the KILL, wait, then re-KILL each group until `kill -0 -<pgid>` fails, 20 x 0.05 s).
# Counts calls after which a process is still in the watchdog's group (r4: checked once, no grace; frame/r3: after a
# 1 s poll, as the `_wm` check's). `load` = 2 x ncpu busy loops for the run's duration.
form=$1; N=$2; d=$(mktemp -d); bad=0; ld=()
if [ "${3-}" = load ]; then for i in $(seq 1 $((2 * $(sysctl -n hw.ncpu)))); do ( while :; do :; done ) & ld+=($!); done; fi
STOP='SECONDS=0
for p; do kill -TERM -"$p"; done
for p; do while kill -0 -"$p" && [ "$SECONDS" -lt 2 ]; do sleep 0.1 || sleep 1; done; done
for p; do kill -9 -"$p"; done; exit 0'
end4() { local p i; kill -9 -"$2" 2>/dev/null || true; wait "$2" 2>/dev/null || true
  for p in "$1" "$2"; do i=0; while kill -0 -"$p" 2>/dev/null && ! kill -0 "$p" 2>/dev/null; do
    [ $i -lt 20 ] || break; kill -9 -"$p" 2>/dev/null || true; sleep 0.05; i=$((i+1)); done; done; }
for i in $(seq 1 $N); do
  set -m
  { ( exec "$BASH" -c 'kill -KILL $$' ) < /dev/null > /dev/null 2>&1 9>&- & _cpid=$!; } 2>/dev/null
  if [ "$form" = frame ]; then
    { ( sleep 3; true > "$d/m" || true; kill -9 -"$_cpid" ) < /dev/null > /dev/null 2>&1 & _wpid=$!; } 2>/dev/null
  else
    { ( exec env -u SHELLOPTS -u BASH_ENV "$BASH" -c 'sleep "$1"; true > "$2" || true; shift 2; '"$STOP" \
        k2wd 3 "$d/m" "$_cpid" ) < /dev/null > /dev/null 2>&1 9>&- & _wpid=$!; } 2>/dev/null
  fi
  set +m
  wait "$_cpid" 2>/dev/null
  if [ "$form" = r4 ]; then end4 "$_cpid" "$_wpid"
    kill -0 -"$_wpid" 2>/dev/null && { bad=$((bad+1)); echo "LEFT i=$i pg=$_wpid: $(ps -o pid,pgid,stat,command -g $_wpid 2>/dev/null | tail -n +2 | tr '\n' '|')"; kill -9 -"$_wpid"; }
  else kill -9 -"$_wpid" 2>/dev/null || true; wait "$_wpid" 2>/dev/null || true
    j=0; while kill -0 -"$_wpid" 2>/dev/null; do [ $j -lt 20 ] || { bad=$((bad+1)); echo "OUTLIVED i=$i pg=$_wpid: $(ps -o pid,pgid,stat,command -g $_wpid 2>/dev/null | tail -n +2 | tr '\n' '|')"; kill -9 -"$_wpid"; break; }; j=$((j+1)); sleep 0.05; done
  fi
done
[ ${#ld[@]} -eq 0 ] || kill -9 "${ld[@]}" 2>/dev/null
echo "form=$form N=$N load=${3-none} shell=${BASH_VERSION%%(*} left=$bad"; rm -rf "$d"
```

#### §E.r4race-batch.sh

`r4race-batch.sh` (sha256 `b853b51347226ac2`, 9 lines)

```bash
#!/bin/bash
# r4race-batch.sh: r4race.sh frame / r4 x b53 / b32 under load, N=300, one at a time; each run its own process group
# (perl setpgrp + alarm) with a kill -9 watchdog. Logs logs/r4/race-<form>.<shell>.txt; marker logs/r4/RACE-DONE.
H=/Users/kazuaki/repos/send.sh/elidex-fgit-handoff/cr2; L=$H/logs/r4; rm -f "$L/RACE-DONE"
for f in frame r4; do for s in b53 b32; do case $s in b53) sh=/opt/homebrew/bin/bash ;; b32) sh=/bin/bash ;; esac
  perl -e 'setpgrp; alarm 900; exec @ARGV' "$sh" "$H/bin/r4race.sh" "$f" 300 load > "$L/race-$f.$s.txt" 2>&1 & pg=$!
  perl -e 'sleep 900; kill 9, -$ARGV[0]' "$pg" </dev/null >/dev/null 2>&1 & wd=$!
  wait "$pg"; kill "$wd" 2>/dev/null; done; done
echo RACE-DONE > "$L/RACE-DONE"
```

#### §E.r5c2-fix4.py

`r5c2-fix4.py` (sha256 `1ce73d50492f1353`, 63 lines)

```python
import sys
# r5c2-fix4.py <clone> [variant ...]: applied AFTER bin/r4c2-fix3.py — the r5 text "[r5p]" is r3c2-proto.py,
# r3c2-fix1.py, r3c2-fix2.py, r4c2-fix3.py, r5c2-fix4.py in sequence. A FEASIBILITY PROTOTYPE for plan-review R4's
# J1/J2/J8/J11 (not the slice's text; C2 measures its own). FROZEN after its measurements (logs/r5/).
#  J1 : the identity read is one helper, `_k2_age <pid> <seconds>` (ps -o etime= through awk); the controls probe it
#       on the run's own pid right after the reap directory is made: unusable -> "decided nothing", exit 2.
#  J2 : the `_og` call line nests: an outer subshell (the sig row's traps, the run's `_K2_REAP_D`) around an inner
#       `( _K2_REAP_D=<og set>; _ctl_run … )`; the sig stub TERMs the outer (its pid written by `_og_trap`), then the
#       inner (`$PPID`, dies by the default action), so the outer's exit path must read the nested set. Record R11n
#       (the reap's glob shrunk to one level) must be killed by the sig row. The outer's fd 9 is the substitution
#       (`exec 9>&1`, its line); the inner starts with fd 9 closed (`9>&-`): bash 3.2 saves a redirected fd 9 (the
#       call's `9>&1`) at fd 10 without close-on-exec, and the `exec`ing watchdog kept that copy of the substitution
#       (the sig row then ended after the watchdog's 5 s: logs/r5/pre/b32-sig-hold.*).
#  J8 : `_k2_stop`'s `wait` is `|| :` (no errexit precondition on its callers).
#  J11: one trap installer, `_k2_traps` (EXIT, HUP, INT, TERM), defined and called by the wire, called by `_og_trap`.
d = sys.argv[1]; T = d + "/.claude/tools/webref-generic-core-trip-wire"
def rep(p, a, b, n=1):
    s = open(p).read(); assert s.count(a) == n, (p, s.count(a), a[:70]); open(p, "w").write(s.replace(a, b))
W = T + ".sh"; K = T + ".controls.sh"; R = T + ".records.sh"
OGL = "a call's process groups end with it and hold none of its caller's streams"
# --- wire: J1 helper
rep(W, '      ps -o etime= -p "$_kr_p" 2>/dev/null | _K2_AGE=$((SECONDS - _kr_t - 1)) awk \'{ n = split($1, f, /[-:]/)\n'
       '        split("1 60 3600 86400", u, " "); for (i = n; i > 0; i--) s += f[i] * u[n - i + 1]; ok = n >= 2 && n <= 4 && $1 ~ /^[0-9:-]+$/ }\n'
       '        END { exit !(ok && s >= ENVIRON["_K2_AGE"]) }\' || continue; fi\n',
       '      _k2_age "$_kr_p" $((SECONDS - _kr_t - 1)) || continue; fi\n')
rep(W, '_k2_reap() { local _kr_f _kr_p _kr_t; local -a _kr_g=()\n',
       '_k2_age() { ps -o etime= -p "$1" 2>/dev/null | _K2_AGE="$2" awk \'{ n = split($1, f, /[-:]/)\n'
       '    split("1 60 3600 86400", u, " "); for (i = n; i > 0; i--) s += f[i] * u[n - i + 1]; ok = n >= 2 && n <= 4 && $1 ~ /^[0-9:-]+$/ }\n'
       '    END { exit !(ok && s >= ENVIRON["_K2_AGE"]) }\'; }\n'
       '_k2_reap() { local _kr_f _kr_p _kr_t; local -a _kr_g=()\n')
# --- wire: J8
rep(W, '    for _ks_p in "$@"; do wait "$_ks_p"; done; } 2>/dev/null; return 0; }\n',
       '    for _ks_p in "$@"; do wait "$_ks_p" || :; done; } 2>/dev/null; return 0; }\n')
# --- wire: J11 one installer
rep(W, 'trap \'_k2_rc=$?; _k2_exit "$_k2_rc"\' EXIT\n'
       '_k2_sig() { trap \'\' HUP INT TERM; set +e; echo "!! the run was stopped by SIG$1, so nothing was decided" >&9; exit "$2"; }\n'
       'trap \'_k2_sig HUP 129\' HUP\n'
       'trap \'_k2_sig INT 130\' INT\n'
       'trap \'_k2_sig TERM 143\' TERM\n',
       '_k2_sig() { trap \'\' HUP INT TERM; set +e; echo "!! the run was stopped by SIG$1, so nothing was decided" >&9; exit "$2"; }\n'
       '_k2_traps() { trap \'_k2_rc=$?; _k2_exit "$_k2_rc"\' EXIT\n'
       'trap \'_k2_sig HUP 129\' HUP\n'
       'trap \'_k2_sig INT 130\' INT\n'
       'trap \'_k2_sig TERM 143\' TERM\n'
       '}; _k2_traps\n')
# --- controls: J1 probe
rep(K, '_K2_REAP_D="$_VFY/.reap"\n',
       '_K2_REAP_D="$_VFY/.reap"\n'
       '_k2_age $$ 0 || { echo "!! decided nothing: the reap\'s identity check needs \\`ps -o etime=\\` (read through awk), which this machine does not give" >&2; exit 2; }\n')
# --- controls: J2 nesting, J11 installer
rep(K, '         _og_trap() { SCRATCH="$_VFY/.og_scr"; mkdir -p "$SCRATCH"; trap \'_k2_rc=$?; _k2_exit "$_k2_rc"\' EXIT; trap \'_k2_sig TERM 143\' TERM; }\n',
       '         _og_trap() { SCRATCH="$_VFY/.og_scr"; mkdir -p "$SCRATCH"; exec 9>&1; sh -c \'echo $PPID\' > "$_VFY/.og_outer"; _k2_traps; }\n')
rep(K, '           _og_st="$( ( _ctl_wd_s=5; _K2_REAP_D="$_og_rd"; [ "$_og_row" != sig ] || _og_trap; _ctl_run "$_og_lbl" "$_VFY" 9>&1 > /dev/null 2> "$_VFY/.og_err"; echo "returned $?" ) 2>/dev/null; echo "ended $?" )"\n',
       '           _og_st="$( ( _ctl_wd_s=5; [ "$_og_row" != sig ] || _og_trap; ( _K2_REAP_D="$_og_rd"; _ctl_run "$_og_lbl" "$_VFY" 9>&1 > /dev/null 2> "$_VFY/.og_err"; echo "returned $?" ) 9>&- ) 2>/dev/null; echo "ended $?" )"\n')
rep(K, 'kill -TERM $PPID; sleep 5\' > "$_VFY/.og_sig"\n',
       'kill -TERM "$(cat "$2/.og_outer")"; kill -TERM $PPID; sleep 5\' > "$_VFY/.og_sig"\n')
rep(K, '"$_VFY/.og_sig" "$_VFY/.og_scr" "$_VFY/.og_err"', '"$_VFY/.og_sig" "$_VFY/.og_scr" "$_VFY/.og_outer" "$_VFY/.og_err"')
# --- records: R11n (the reap reads one level only)
rep(R, 's/^  _k2_reap$/  :/\t' + OGL + '\n',
       's/^  _k2_reap$/  :/\t' + OGL + '\n'
       's/^\\(  for _kr_f in "[$]_K2_REAP_D"\\/\\*\\) "[$]_K2_REAP_D"\\/\\*\\/\\*;/\\1;/\t' + OGL + '\n')
for v in sys.argv[2:]:
    raise SystemExit("unknown variant " + v)
```

#### §E.r5-clone.sh

`r5-clone.sh` (sha256 `cdd82249e6388348`, 9 lines)

```bash
#!/bin/bash
# r5-clone.sh <name> [fix3-variant ...] [-- instrument ...]: the r5 text [r5p] = r4-clone.sh's [r4p] (+ fix3 variants)
# then bin/r5c2-fix4.py; both shells' `-n` over every part. Prints the clone path last.
set -eu
H=/Users/kazuaki/repos/send.sh/elidex-fgit-handoff/cr2
d=$("$H/bin/r4-clone.sh" "$@" | tail -n 1)
python3 -I "$H/bin/r5c2-fix4.py" "$d"
for f in "$d"/.claude/tools/webref-generic-core-trip-wire*.sh; do /opt/homebrew/bin/bash -n "$f"; /bin/bash -n "$f"; done
echo "$d"
```

#### §E.r5-batch.sh

`r5-batch.sh` (sha256 `99a458d9dfe85be4`, 25 lines)

```bash
#!/bin/bash
# r5-batch.sh: the wire runs on the r5 text [r5p] (bin/r5-clone.sh), each item b53 ∥ b32 (two wire processes at a
# time). Logs logs/r5/<item>.<shell>.log; marker logs/r5/R5-BATCH-DONE. `shipps`: the ship with a `ps` that fails
# (work/r5-ps-shim/ps, `exit 1`) first on PATH (J1).
H=/Users/kazuaki/repos/send.sh/elidex-fgit-handoff/cr2; B=$H/bin; L=$H/logs/r5
cl() { "$B/r5-clone.sh" "$@" | tail -n 1; }
both() { local f="$1"; shift; for s in b53 b32; do "$f" "$s" "$@" & done; wait; echo "$(date +%T) done $*" >> "$L/R5-BATCH.progress"; }
ship() { "$B/r3-runsig.sh" "$(cl "r5-$2-$1" "${@:4}")" "$1" "$L/$2.$1.log" "$3"; }          # ship <s> <item> <disp> [variants]
shipps() { local c; c="$(cl "r5-$2-$1")"; env PATH="$H/work/r5-ps-shim:$PATH" "$B/r3-runsig.sh" "$c" "$1" "$L/$2.$1.log" "$3"; }
opt()  { "$B/r3-optin.sh" "$(cl "r5-$2-$1" "${@:5}")" "$1" "$L/$2.$1.log" "$3" "$4"; }      # opt <s> <item> <line> <disp> [variants]
: > "$L/R5-BATCH.progress"; rm -f "$L/R5-BATCH-DONE"
both ship r5-ship default
both shipps r5-shipps default
both ship r5-noreapship default noreap
both opt r5-optR11n 481 default
both opt r5-survR11n 481 default og-nosig
both opt r5-optR11r 480 default
both opt r5-optR11h 469 default
both opt r5-optR11i 470 default
both opt r5-optR11t 471 default
both opt r5-optR11d 475 default
both opt r5-optR11e 478 default
both opt r5-optR11a 476 default
both opt r5-optR11x 477 default
echo R5-BATCH-DONE > "$L/R5-BATCH-DONE"
```

#### §E.r5probe.sh

`r5probe.sh` (sha256 `671415b837ea0bfa`, 52 lines)

```bash
#!/bin/bash
# r5probe.sh <r5p clone> <r4p clone> <out>: [synthetic harness, each clone's own exit block, as r4probe.sh] one fresh
# shell per case under `set -euo pipefail`, both shells, each case its own process group with a 60 s kill -9 watchdog.
# Cases P1-P7, P9 as r4probe.sh, on [r5p]; then:
#  P8  _k2_reap_add failing under set -m, called BARE (not an || operand)  -> rc 2, the line, the group gone (J8)
#  P8r P8 on [r4p]'s block (its `wait` without `|| :`)                       -> the comparison arm
#  P10 the child recorded, the watchdog's record fails (its entry path is a directory): `_k2_stop` wrapped to say
#      whether the child's entry still exists when the stop starts         -> rc 2, the watchdog named, both groups
#      gone, "at-stop: child entry present", the child's entry removed after (J7: stop, then remove)
#  P10s P10 on [r5p]'s block with the arm's order swapped (remove, then stop) -> "at-stop: child entry absent"
H=/Users/kazuaki/repos/send.sh/elidex-fgit-handoff/cr2; O="$3"; : > "$O"
ext() { awk '/^# R3PROTO-EXIT-BEGIN$/{on=1; next} /^# R3PROTO-EXIT-END$/{on=0} on' "$1/.claude/tools/webref-generic-core-trip-wire.sh"; }
B5="$(ext "$1")"; B4="$(ext "$2")"
a='  _k2_stop "${@:2}"; for _ka_p in "${@:3}"; do command rm -f "$_K2_REAP_D/$_ka_p" 2>/dev/null; done; exit 2; }'
b='  for _ka_p in "${@:3}"; do command rm -f "$_K2_REAP_D/$_ka_p" 2>/dev/null; done; _k2_stop "${@:2}"; exit 2; }'
B5s="$(PA="$a" PB="$b" PS="$B5" python3 -I -c 'import os; s=os.environ["PS"]; a=os.environ["PA"]; assert s.count(a)==1; print(s.replace(a, os.environ["PB"]))')" || { echo "P10s: arm not found" >> "$O"; exit 1; }
Wk="$H/work/r5probe"; mkdir -p "$Wk"
rs='_K2_REAP_D="$SCRATCH/v/.reap"; mkdir -p "$_K2_REAP_D"'
rs10='_K2_REAP_D="'"$Wk"'/r10"; rm -rf "$_K2_REAP_D"; mkdir -p "$_K2_REAP_D"'
rec='printf "%s\n" "$SECONDS" > "$_K2_REAP_D/$p"'
pid='echo $p >> "'"$Wk"'/pids"'
wrap='eval "$(declare -f _k2_stop | /usr/bin/sed "1s/^_k2_stop/_k2_stop_real/")"; _k2_stop() { if [ -e "$_K2_REAP_D/$c" ]; then echo "at-stop: child entry present" >&2; else echo "at-stop: child entry absent" >&2; fi; _k2_stop_real "$@"; }'
body() { case "$1" in
  P1) echo "$rs; set -m; sleep 30 & p=\$!; set +m; $pid; $rec; exit 3" ;;
  P2) echo "$rs; set -m; sleep 30 & p=\$!; set +m; $pid; printf '%s\n' \"\$((SECONDS - 10))\" > \"\$_K2_REAP_D/\$p\"; exit 3" ;;
  P3) echo "$rs; set -m; ( sleep 30 & echo \$! >> \"$Wk/pids\"; exec sleep 30 ) & p=\$!; set +m; sleep 0.3; mkdir \"\$_K2_REAP_D/\$p\"; kill -9 \$p; wait \$p 2>/dev/null || :; exit 3" ;;
  P4) echo "$rs; set -m; sleep 30 & p=\$!; set +m; $pid; mkdir \"\$_K2_REAP_D/\$p\"; exit 3" ;;
  P5) echo "$rs; set -m; sleep 30 & p=\$!; set +m; $pid; $rec; PATH=/nonexistent-k2; exit 3" ;;
  P6) echo "$rs; set -m; ( exit 0 ) & c=\$!; sleep 30 & w=\$!; set +m; echo \$w >> \"$Wk/pids\"; mkdir \"\$_K2_REAP_D/\$c\"; p=\$w; $rec; wait \$c || :; _k2_end lbl \$c \$w; exit 0" ;;
  P7) echo "$rs; set -m; ( sleep 30 & echo \$! >> \"$Wk/pids\"; exit 0 ) & c=\$!; sleep 30 & w=\$!; set +m; echo \$w >> \"$Wk/pids\"; for p in \$c \$w; do $rec; done; wait \$c || :; sleep 0.2; _k2_end lbl \$c \$w; echo \"left=[\${_ctl_left}] entries=[\$(ls \"\$_K2_REAP_D\" | tr '\n' ' ')]\" >&2; _K2_ORDERLY=1; exit 0" ;;
  P8|P8r) echo "$rs; set -m; sleep 30 & p=\$!; $pid; _K2_REAP_D=/nonexistent-k2/r; _k2_reap_add lbl \$p; exit 0" ;;
  P9) echo "$rs; set -m; for i in 1 2 3 4 5; do sleep 30 & p=\$!; $pid; $rec; done; set +m; exit 3" ;;
  P10|P10s) echo "$rs10; set -m; sleep 30 & c=\$!; sleep 30 & w=\$!; echo \$c >> \"$Wk/pids\"; echo \$w >> \"$Wk/pids\"; p=\$c; $rec; set +m; mkdir \"\$_K2_REAP_D/\$w\"; $wrap; _k2_reap_add lbl \$w \$c; exit 0" ;;
esac; }
now() { perl -MTime::HiRes=time -e 'printf "%.2f\n", time'; }
for sh in /opt/homebrew/bin/bash /bin/bash; do for k in P1 P2 P3 P4 P5 P6 P7 P8 P8r P9 P10 P10s; do
  case "$k" in P8r) B="$B4" ;; P10s) B="$B5s" ;; *) B="$B5" ;; esac
  S="$Wk/s.$$"; rm -rf "$S" "$Wk/pids" "$Wk/out" "$Wk/r10"; mkdir -p "$S"
  t0="$(now)"; rc=0
  perl -e '$SIG{INT}="DEFAULT"; $SIG{TERM}="DEFAULT"; $SIG{HUP}="DEFAULT"; setpgrp; alarm 60; exec @ARGV' "$sh" -c "set -euo pipefail
SCRATCH=\"$S\"
$B
$(body $k)" > "$Wk/out" 2>&1 & pg=$!
  perl -e 'sleep 60; kill 9, -$ARGV[0]' "$pg" </dev/null >/dev/null 2>&1 & wd=$!
  wait "$pg" || rc=$?; kill "$wd" 2>/dev/null; t1="$(now)"
  sleep 0.3; left=""
  for p in $(cat "$Wk/pids" 2>/dev/null); do kill -0 "$p" 2>/dev/null && { left="$left ALIVE:$p"; kill -9 "$p" 2>/dev/null; }; done
  ent=""; [ ! -d "$Wk/r10" ] || ent=" r10-entries=[$(ls "$Wk/r10" | tr '\n' ' ')] (watchdog=$(/usr/bin/sed -n 2p "$Wk/pids"))"
  printf '%s %-4s rc=%s %.2fs%s%s out=[%s]\n' "$($sh -c 'echo ${BASH_VERSION%%(*}')" "$k" "$rc" "$(T0="$t0" T1="$t1" awk 'BEGIN{print ENVIRON["T1"]-ENVIRON["T0"]}')" "${left:- none-alive}" "$ent" "$(tr '\n' '|' < "$Wk/out" | cut -c1-220)" >> "$O"
  rm -rf "$S" "$Wk/r10"
done; done
echo R5PROBE-DONE >> "$O"
```
