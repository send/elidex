# K2 fixture git — PR #527 CR2 round: the instruments (as run), §E.2 — the exit path's probes

Companion of `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-cr2-sweep.md` ("the sweep file"), which holds CR2's sweep, §E.1 and §E.4,
and whose §E index names the file of every instrument. §E.2 was split out of it at CR2-M (touch-time, the
1000-line rule), each block byte-identical. The plan is `…-cr2-plan.md`, with its companion
`…-cr2-plan-exitpath.md` (written there as x§6.n). **Frame `706049ad`.** How a `cr2/bin/<name>` resolves, how a
block is re-run and what it reads: the sweep file's header and §E.4. This file is excluded from CR2's sweep and
from the plan's claim rows over `docs/plans/*k2-fgit*`, as the sweep file is (the plan's §10 and §11).

## §0 Spec coverage map

**No spec surface** — as the plan's §0: this file holds a shell trip-wire's instruments, as run. `preflight.py`
exits 1 here by design, because no table follows this heading.

## §E Appendix — the instruments (as run), continued

One fenced block per file, under its file name, as in the sweep file's §E.

### §E.2 The exit path (the plan's x§6): probes, signal and fd measurements, single-record runs

#### §E.cr11.sh

`cr11.sh` (sha256 `6800bea217f7dccc`, 16 lines)

```bash
#!/bin/bash
# cr11.sh <clone> <b53|b32> <log> <SIG> [delay]: the full run in its own process group; after <delay> s (default 8)
# <SIG> goes to that group; then the run's rc, its last lines, and every process left from the run's groups
# (any process whose pgid is one the run's `_ctl_run` made: found as `sleep <_ctl_wd_s>` watchdogs and wire children).
c="$1"; L="$(cd "$(dirname "$3")" && pwd)/$(basename "$3")"; set -- "$1" "$2" "$L" "$4" "${5:-8}"; case "$2" in b53) sh=/opt/homebrew/bin/bash ;; b32) sh=/bin/bash ;; esac; log="$3"; sig="$4"; dl="${5:-8}"
cd "$c" || exit 2
perl -e '$SIG{INT}="DEFAULT"; $SIG{TERM}="DEFAULT"; $SIG{HUP}="DEFAULT"; setpgrp; alarm 300; exec @ARGV' "$sh" .claude/tools/webref-generic-core-trip-wire.sh > "$log.out" 2>&1 &
pg=$!
sleep "$dl"
before="$(ps -A -o pid=,pgid=,ppid=,command= | ME="$pg" awk '$2 != ENVIRON["ME"] && /sleep 30$|trip-wire.sh --selftest/ {print}')"
kill -"$sig" -"$pg"; rc=0; wait "$pg" || rc=$?
sleep 1
after="$(ps -A -o pid=,pgid=,ppid=,command= | awk '/sleep 30$|trip-wire.sh --selftest/ && !/awk/ {print}')"
{ echo "### sent SIG$sig to pgid $pg after ${dl}s; rc=$rc"; echo "### last lines:"; tail -n 4 "$log.out"
  echo "### other-group processes of the run just before the signal:"; printf '%s\n' "$before"
  echo "### left 1 s after the run ended:"; printf '%s\n' "$after"; } > "$log"
```

#### §E.cr12.sh

`cr12.sh` (sha256 `ee72bfa074c3eb46`, 8 lines)

```bash
#!/bin/bash
# cr12.sh <clone> <b53|b32> <log>: the wire's --selftest over a one-file clean repo, with and without a PATH-first `rm` that fails `rm -rf`.
c="$1"; case "$2" in b53) sh=/opt/homebrew/bin/bash ;; b32) sh=/bin/bash ;; esac; log="$3"; H=/Users/kazuaki/repos/send.sh/elidex-fgit-handoff/cr2
{ for v in plain shim; do
  echo "### $v"; P="$PATH"; [ $v = shim ] && P="$H/work/shim-rm:$PATH"
  rc=0; ( cd "$c" && PATH="$P" perl -e 'setpgrp; alarm 120; exec @ARGV' "$sh" .claude/tools/webref-generic-core-trip-wire.sh --selftest "$H/work/fx/clean" ) > "$log.$v.out" 2>&1 || rc=$?
  tail -n 3 "$log.$v.out"; echo "### $v rc=$rc"
done; } > "$log" 2>&1
```

#### §E.mkfx-clean.sh

`mkfx-clean.sh` (sha256 `f0c853c5998a42b9`, 8 lines)

```bash
#!/bin/bash
# mkfx-clean.sh <dir>: makes cr12.sh's second input, a one-file clean git repo (`a.py` = "a clean file", one commit by
# x <x@y>, message "c"), as cr2/work/fx/clean was made. Refuses an existing <dir>.
set -eu
d="$1"; [ ! -e "$d" ] || { echo "mkfx-clean.sh: $d exists" >&2; exit 2; }
mkdir -p "$d"; cd "$d"; git init -q .
printf 'a clean file\n' > a.py; git add a.py
git -c user.name=x -c user.email=x@y commit -q -m c
```

#### §E.trapprobe.sh

`trapprobe.sh` (sha256 `5a22fe399f77783f`, 11 lines)

```bash
#!/bin/bash
# trapprobe.sh: does an EXIT trap run, and with which $?, when SIGTERM/SIGINT reach a non-interactive bash
# (signal dispositions reset to default by perl) that is (w) in `wait` on a background job, (f) in a foreground
# external command, (c) in a command substitution? Both shells.
O=/Users/kazuaki/repos/send.sh/elidex-fgit-handoff/cr2/logs/trapprobe.txt; : > $O
for sh in /opt/homebrew/bin/bash /bin/bash; do for sig in TERM INT; do for m in w f c; do
  case $m in w) body='sleep 5 & wait $!' ;; f) body='sleep 5' ;; c) body='x=$(sleep 5)' ;; esac
  perl -e '$SIG{INT}="DEFAULT"; $SIG{TERM}="DEFAULT"; setpgrp; exec @ARGV' "$sh" -c "trap 'echo \"exit-trap rc=\$?\"' EXIT; $body; echo after" > /Users/kazuaki/repos/send.sh/elidex-fgit-handoff/cr2/logs/tp.out 2>&1 &
  pg=$!; sleep 1; kill -$sig -$pg; rc=0; wait $pg || rc=$?
  echo "$(basename $sh)-${BASH_VERSINFO:-} $($sh -c 'echo $BASH_VERSION' | cut -c1-6) SIG$sig mode=$m rc=$rc out=[$(tr '\n' ' ' < /Users/kazuaki/repos/send.sh/elidex-fgit-handoff/cr2/logs/tp.out)]" >> $O
done; done; done
```

#### §E.trapprobe2.sh

`trapprobe2.sh` (sha256 `07e7c7ca94ee9390`, 9 lines)

```bash
#!/bin/bash
# trapprobe2.sh: with an INT/TERM trap that exits 128+n, which of the trap's and the EXIT trap's stderr lines are
# seen when the signal lands in `wait $! 2>/dev/null` (the shape of ctlrun's wait), both shells.
O=/Users/kazuaki/repos/send.sh/elidex-fgit-handoff/cr2/logs/trapprobe2.txt; : > $O
for sh in /opt/homebrew/bin/bash /bin/bash; do for sig in INT TERM; do
  perl -e '$SIG{INT}="DEFAULT"; $SIG{TERM}="DEFAULT"; setpgrp; exec @ARGV' "$sh" -c 'trap "echo exit-trap rc=\$? >&2" EXIT; trap "echo sig-trap >&2; exit 130" INT; trap "echo sig-trap >&2; exit 143" TERM; sleep 5 & wait $! 2>/dev/null; echo after >&2' > /Users/kazuaki/repos/send.sh/elidex-fgit-handoff/cr2/logs/tp2.out 2>&1 &
  pg=$!; sleep 1; kill -$sig -$pg; rc=0; wait $pg || rc=$?
  echo "$($sh -c 'echo $BASH_VERSION' | cut -c1-6) SIG$sig rc=$rc out=[$(tr '\n' ' ' < /Users/kazuaki/repos/send.sh/elidex-fgit-handoff/cr2/logs/tp2.out)]" >> $O
done; done
```

#### §E.trapprobe3.sh

`trapprobe3.sh` (sha256 `5c5f5a181f5611f3`, 10 lines)

```bash
#!/bin/bash
# trapprobe3.sh: with NO signal trap (the frame's wire), is the EXIT trap's stderr line seen when TERM/INT lands in
# `wait $! 2>/dev/null` vs in `wait $!`? Both shells.
O=/Users/kazuaki/repos/send.sh/elidex-fgit-handoff/cr2/logs/trapprobe3.txt; : > $O; T=/Users/kazuaki/repos/send.sh/elidex-fgit-handoff/cr2/logs/tp3.out
for sh in /opt/homebrew/bin/bash /bin/bash; do for sig in TERM INT; do for r in redir plain; do
  w='wait $!'; [ $r = redir ] && w='wait $! 2>/dev/null'
  perl -e '$SIG{INT}="DEFAULT"; $SIG{TERM}="DEFAULT"; setpgrp; exec @ARGV' "$sh" -c "trap 'echo exit-trap rc=\$? >&2' EXIT; sleep 5 & $w" > $T 2>&1 &
  pg=$!; sleep 1; kill -$sig -$pg; rc=0; wait $pg || rc=$?
  echo "$($sh -c 'echo $BASH_VERSION' | cut -c1-6) SIG$sig $r rc=$rc out=[$(tr '\n' ' ' < $T)]" >> $O
done; done; done
```

#### §E.dispprobe.sh

`dispprobe.sh` (sha256 `b62bb22c4e486108`, 14 lines)

```bash
#!/bin/bash
# dispprobe.sh: (1) a signal IGNORED at entry: can the script trap it, does `trap -p` show it, and is it still ignored
# in an async child launched under `set -m`? (2) with default dispositions: is SIGINT default in an async child under
# `set -m` (yes = a TERM/INT/HUP control can be built on `_ctl_run`'s launch) and without it (POSIX XCU 2.11)?
# (3) the startup save `exec 9>&2` with stderr CLOSED: plain (errexit) vs `|| exec 9>/dev/null`. Both shells.
for sh in /opt/homebrew/bin/bash /bin/bash; do v="$($sh -c 'echo ${BASH_VERSION%%(*}')"
  echo "== $v (1) INT+HUP ignored at entry"
  perl -e '$SIG{INT}="IGNORE"; $SIG{HUP}="IGNORE"; exec @ARGV' $sh -c 'echo "  trap -p: [$(trap -p INT)|$(trap -p HUP)]"; trap "echo caught" INT; kill -INT $$; echo "  after kill -INT \$\$: survived (trap not run)"; set -m; ( echo "  set -m child: [$(trap -p INT)]"; '"$sh"' -c "kill -INT \$\$; echo \"  set -m child grandchild survived INT\"" ) & wait; set +m' 2>&1
  echo "== $v (2) default dispositions"
  perl -e '$SIG{INT}="DEFAULT"; exec @ARGV' $sh -c 'set -m; '"$sh"' -c "trap \"echo \\\"  set -m async child: INT trapped\\\"\" INT; kill -INT \$\$" & wait; set +m; '"$sh"' -c "trap \"echo \\\"  plain async child: INT trapped\\\"\" INT; kill -INT \$\$; echo \"  plain async child: INT ignored\"" & wait' 2>&1 | /usr/bin/grep -v Done
  echo "== $v (3) exec 9>&2 with stderr closed"
  $sh -c 'set -euo pipefail; exec 9>&2; echo "  plain: survived"' 2>&-; echo "  plain: rc=$?"
  $sh -c 'set -euo pipefail; exec 9>&2 || exec 9>/dev/null; echo "  or-form: survived"' 2>&-; echo "  or-form: rc=$?"
done
```

#### §E.sigsite.sh

`sigsite.sh` (sha256 `2be8cc51baf28a50`, 35 lines)

```bash
#!/bin/bash
# sigsite.sh <clone> <b53|b32> <log> <SIG> <site>: the full run, its own process group, HUP/INT/TERM reset to default,
# TMPDIR private to the run; <site> = a file name to wait for under that TMPDIR (a landing point: `.wm_hang.pid`, written
# by the `_wm` check's hang stub; `.ev_run.mark` / `.cap_run.mark`, written by the instrumented stubs) or `delay:<s>`.
# Just before the signal: every process descended from the run and its PROCESS GROUP (the run's own excluded) - the
# pgid-scoped population (F18) - and, on a prototype, the reap set's entries. After: the run's rc, its last lines, its
# b32 job notices, and every process still in one of those groups 1 s after the run ended (then killed, logged).
c="$1"; s="$2"; log="$3"; sig="$4"; site="$5"; case "$s" in b53) sh=/opt/homebrew/bin/bash ;; b32) sh=/bin/bash ;; esac
H=/Users/kazuaki/repos/send.sh/elidex-fgit-handoff/cr2; TD="$H/work/td.$(basename "$log")"; rm -rf "$TD"; mkdir -p "$TD"
cd "$c" || exit 2
TMPDIR="$TD" perl -e '$SIG{INT}="DEFAULT"; $SIG{TERM}="DEFAULT"; $SIG{HUP}="DEFAULT"; setpgrp; exec @ARGV' "$sh" .claude/tools/webref-generic-core-trip-wire.sh </dev/null > "$log.out" 2>&1 &
pg=$!
perl -e 'sleep 300; kill 9, -$ARGV[0]' "$pg" </dev/null >/dev/null 2>&1 & wd=$!
hit=""
case "$site" in
  delay:*) sleep "${site#delay:}"; hit=delay ;;
  *) i=0; while [ "$i" -lt 2400 ]; do f="$(find "$TD" -name "$site" 2>/dev/null | head -n 1)"; [ -n "$f" ] && { hit="$f"; break; }; sleep 0.05; i=$((i+1)); done ;;
esac
snap="$(ps -A -o pid=,ppid=,pgid=,command= | ROOT="$pg" awk '{ pid[NR]=$1; pp[NR]=$2; pg[NR]=$3; $1=$2=$3=""; cmd[NR]=$0 }
  END { d[ENVIRON["ROOT"]]=1; ch=1; while (ch) { ch=0; for (i=1;i<=NR;i++) if (!(pid[i] in d) && (pp[i] in d)) { d[pid[i]]=1; ch=1 } }
        for (i=1;i<=NR;i++) if ((pid[i] in d) && pg[i] != ENVIRON["ROOT"]) print pid[i], pg[i], cmd[i] }')"
reap="$(ls "$TD"/tmp.*/verify*/.reap/ 2>/dev/null | tr '\n' ' ')"
kill -"$sig" -"$pg"; rc=0; wait "$pg" || rc=$?; kill "$wd" 2>/dev/null
sleep 1
groups="$(printf '%s\n' "$snap" | awk 'NF {print $2}' | sort -u | tr '\n' ' ')"
left="$(ps -A -o pid=,pgid=,command= | G="$groups" awk 'BEGIN { n=split(ENVIRON["G"], a, " "); for (i=1;i<=n;i++) g[a[i]]=1 } ($2 in g) {print}')"
{ echo "### $s SIG$sig site=$site hit=${hit##*/} rc=$rc"
  echo "### last lines:"; tail -n 3 "$log.out" | cut -c1-200
  echo "### job notices (Killed/Terminated/Done) in the run's output: $(/usr/bin/grep -cE 'Killed|Terminated|Done' "$log.out")"
  echo "### the run's other groups just before the signal (pid pgid command):"; printf '%s\n' "$snap" | cut -c1-160
  echo "### reap set entries just before the signal: [$reap]"
  echo "### left in those groups 1 s after the run ended: $(printf '%s' "$left" | /usr/bin/grep -c .)"; printf '%s\n' "$left" | cut -c1-160
} > "$log"
for g in $groups; do kill -9 -"$g" 2>/dev/null; done
rm -rf "$TD"
```

#### §E.sigsite2.sh

`sigsite2.sh` (sha256 `943cdac8633bef65`, 35 lines)

```bash
#!/bin/bash
# sigsite2.sh (= sigsite.sh with the after-check over the snapshot groups UNION the reap entries) <clone> <b53|b32> <log> <SIG> <site>: the full run, its own process group, HUP/INT/TERM reset to default,
# TMPDIR private to the run; <site> = a file name to wait for under that TMPDIR (a landing point: `.wm_hang.pid`, written
# by the `_wm` check's hang stub; `.ev_run.mark` / `.cap_run.mark`, written by the instrumented stubs) or `delay:<s>`.
# Just before the signal: every process descended from the run and its PROCESS GROUP (the run's own excluded) - the
# pgid-scoped population (F18) - and, on a prototype, the reap set's entries. After: the run's rc, its last lines, its
# b32 job notices, and every process still in one of those groups 1 s after the run ended (then killed, logged).
c="$1"; s="$2"; log="$3"; sig="$4"; site="$5"; case "$s" in b53) sh=/opt/homebrew/bin/bash ;; b32) sh=/bin/bash ;; esac
H=/Users/kazuaki/repos/send.sh/elidex-fgit-handoff/cr2; TD="$H/work/td.$(basename "$log")"; rm -rf "$TD"; mkdir -p "$TD"
cd "$c" || exit 2
TMPDIR="$TD" perl -e '$SIG{INT}="DEFAULT"; $SIG{TERM}="DEFAULT"; $SIG{HUP}="DEFAULT"; setpgrp; exec @ARGV' "$sh" .claude/tools/webref-generic-core-trip-wire.sh </dev/null > "$log.out" 2>&1 &
pg=$!
perl -e 'sleep 300; kill 9, -$ARGV[0]' "$pg" </dev/null >/dev/null 2>&1 & wd=$!
hit=""
case "$site" in
  delay:*) sleep "${site#delay:}"; hit=delay ;;
  *) i=0; while [ "$i" -lt 2400 ]; do f="$(find "$TD" -name "$site" 2>/dev/null | head -n 1)"; [ -n "$f" ] && { hit="$f"; break; }; sleep 0.05; i=$((i+1)); done ;;
esac
snap="$(ps -A -o pid=,ppid=,pgid=,command= | ROOT="$pg" awk '{ pid[NR]=$1; pp[NR]=$2; pg[NR]=$3; $1=$2=$3=""; cmd[NR]=$0 }
  END { d[ENVIRON["ROOT"]]=1; ch=1; while (ch) { ch=0; for (i=1;i<=NR;i++) if (!(pid[i] in d) && (pp[i] in d)) { d[pid[i]]=1; ch=1 } }
        for (i=1;i<=NR;i++) if ((pid[i] in d) && pg[i] != ENVIRON["ROOT"]) print pid[i], pg[i], cmd[i] }')"
reap="$(ls "$TD"/tmp.*/verify*/.reap/ 2>/dev/null | tr '\n' ' ')"
kill -"$sig" -"$pg"; rc=0; wait "$pg" || rc=$?; kill "$wd" 2>/dev/null
sleep 1
groups="$( { printf "%s\n" "$snap" | awk "NF {print \$2}"; printf "%s\n" $reap; } | sort -u | tr "\n" " ")"
left="$(ps -A -o pid=,pgid=,command= | G="$groups" awk 'BEGIN { n=split(ENVIRON["G"], a, " "); for (i=1;i<=n;i++) g[a[i]]=1 } ($2 in g) {print}')"
{ echo "### $s SIG$sig site=$site hit=${hit##*/} rc=$rc"
  echo "### last lines:"; tail -n 3 "$log.out" | cut -c1-200
  echo "### job notices (Killed/Terminated/Done) in the run's output: $(/usr/bin/grep -cE 'Killed|Terminated|Done' "$log.out")"
  echo "### the run's other groups just before the signal (pid pgid command):"; printf '%s\n' "$snap" | cut -c1-160
  echo "### reap set entries just before the signal: [$reap]"
  echo "### left in those groups 1 s after the run ended: $(printf '%s' "$left" | /usr/bin/grep -c .)"; printf '%s\n' "$left" | cut -c1-160
} > "$log"
for g in $groups; do kill -9 -"$g" 2>/dev/null; done
rm -rf "$TD"
```

#### §E.sigsite3.sh

`sigsite3.sh` (sha256 `2cc5870176c3b3b8`, 44 lines)

```bash
#!/bin/bash
# sigsite3.sh <clone> <b53|b32> <log> <SIG> <site> [group|pidonly] [settle-s]: r3's signal-site instrument (sigsite2.sh
# plus): the run's TMPDIR is its own and is NOT deleted (work/td3/<log name>), so the scratch dirs the run's children
# leave are counted (G2); `pidonly` sends the signal to the run's pid instead of its group (G20) and the seconds from
# the signal to the run's end are logged; after the run ended, the snapshot groups (just before the signal, the run's
# descendants' groups, the run's own excluded) UNION the reap set's entries are counted at +1 s, and the leftover
# scratch dirs (`tmp.*` under the run's TMPDIR) at +1 s and again once those groups are gone or <settle> s (default 1)
# passed; only then are leftovers killed. HUP/INT/TERM reset to default; a kill -9 watchdog beside the run.
c="$1"; s="$2"; log="$3"; sig="$4"; site="$5"; mode="${6:-group}"; settle="${7:-1}"
case "$s" in b53) sh=/opt/homebrew/bin/bash ;; b32) sh=/bin/bash ;; esac
H=/Users/kazuaki/repos/send.sh/elidex-fgit-handoff/cr2; TD="$H/work/td3/$(basename "$log")"; rm -rf "$TD"; mkdir -p "$TD"
now() { perl -MTime::HiRes=time -e 'printf "%.2f\n", time'; }
cd "$c" || exit 2
TMPDIR="$TD" perl -e '$SIG{INT}="DEFAULT"; $SIG{TERM}="DEFAULT"; $SIG{HUP}="DEFAULT"; setpgrp; exec @ARGV' "$sh" .claude/tools/webref-generic-core-trip-wire.sh </dev/null > "$log.out" 2>&1 &
pg=$!
perl -e 'sleep 300; kill 9, -$ARGV[0]' "$pg" </dev/null >/dev/null 2>&1 & wd=$!
hit=""
case "$site" in
  delay:*) sleep "${site#delay:}"; hit=delay ;;
  *) i=0; while [ "$i" -lt 2400 ]; do f="$(find "$TD" -name "$site" 2>/dev/null | head -n 1)"; [ -n "$f" ] && { hit="$f"; break; }; sleep 0.05; i=$((i+1)); done ;;
esac
snap="$(ps -A -o pid=,ppid=,pgid=,command= | ROOT="$pg" awk '{ pid[NR]=$1; pp[NR]=$2; pg[NR]=$3; $1=$2=$3=""; cmd[NR]=$0 }
  END { d[ENVIRON["ROOT"]]=1; ch=1; while (ch) { ch=0; for (i=1;i<=NR;i++) if (!(pid[i] in d) && (pp[i] in d)) { d[pid[i]]=1; ch=1 } }
        for (i=1;i<=NR;i++) if ((pid[i] in d) && pg[i] != ENVIRON["ROOT"]) print pid[i], pg[i], cmd[i] }')"
reap="$(ls "$TD"/tmp.*/verify*/.reap/ 2>/dev/null | tr '\n' ' ')"
t0="$(now)"; if [ "$mode" = pidonly ]; then kill -"$sig" "$pg"; else kill -"$sig" -"$pg"; fi
rc=0; wait "$pg" || rc=$?; t1="$(now)"; kill "$wd" 2>/dev/null
sleep 1
groups="$( { printf "%s\n" "$snap" | awk "NF {print \$2}"; printf "%s\n" $reap; } | sort -u | tr "\n" " ")"
lsg() { ps -A -o pid=,pgid=,command= | G="$groups" awk 'BEGIN { n=split(ENVIRON["G"], a, " "); for (i=1;i<=n;i++) g[a[i]]=1 } ($2 in g) {print}'; }
left="$(lsg)"; tmp1="$(find "$TD" -maxdepth 1 -name 'tmp.*' -type d | wc -l | tr -d ' ')"
j=0; while [ "$j" -lt $((settle * 2)) ] && [ -n "$(lsg)" ]; do sleep 0.5; j=$((j+1)); done
left2="$(lsg)"; tmp2="$(find "$TD" -maxdepth 1 -name 'tmp.*' -type d | wc -l | tr -d ' ')"
{ echo "### $s SIG$sig mode=$mode site=$site hit=${hit##*/} rc=$rc signal->end=$(T0="$t0" T1="$t1" awk 'BEGIN { printf "%.2f s", ENVIRON["T1"] - ENVIRON["T0"] }')"
  echo "### clone=$c text=$(cd "$c" && git diff --stat | tail -1)"
  echo "### last lines:"; tail -n 3 "$log.out" | cut -c1-200
  echo "### job notices (Killed/Terminated/Done) in the run's output: $(/usr/bin/grep -cE 'Killed|Terminated|Done' "$log.out")"
  echo "### the run's other groups just before the signal (pid pgid command):"; printf '%s\n' "$snap" | cut -c1-160
  echo "### reap set entries just before the signal: [$reap]"
  echo "### left in those groups 1 s after the run ended: $(printf '%s' "$left" | /usr/bin/grep -c .)"; printf '%s\n' "$left" | cut -c1-160
  echo "### scratch dirs left under the run's TMPDIR: +1 s: $tmp1; after the groups ended or ${settle} s: $tmp2 (groups still live then: $(printf '%s' "$left2" | /usr/bin/grep -c .))"
  find "$TD" -maxdepth 1 -name 'tmp.*' -type d | sed 's/^/    /'
} > "$log"
for g in $groups; do kill -9 -"$g" 2>/dev/null; done
```

#### §E.f2probe.sh

`f2probe.sh` (sha256 `516c60b13022bac6`, 57 lines)

```bash
#!/bin/bash
# f2probe.sh <proto clone>: the exit path under errexit, both shells. "r2" = the block between the R2PROTO-EXIT markers
# of the clone's wire (bin/r2c2-proto.py), run after `set -euo pipefail` as the wire runs it; "r1" = r1's §6.2 text as
# written (reap line unguarded, `rm -rf … || echo … >&2`, no `set +e`), for comparison. One fresh shell per case.
H=/Users/kazuaki/repos/send.sh/elidex-fgit-handoff/cr2; c="$1"; W="$c/.claude/tools/webref-generic-core-trip-wire.sh"
B2="$(awk '/^# R2PROTO-EXIT-BEGIN$/{on=1; next} /^# R2PROTO-EXIT-END$/{on=0} on' "$W")"
B1='
_K2_ORDERLY=0; _K2_REAP=""
_k2_exit() {
  kill -9 $_K2_REAP
  case "$SCRATCH" in /*/*) chmod -R u+rwX "$SCRATCH" 2>/dev/null || true
    rm -rf "$SCRATCH" || echo "!! could not remove the scratch dir $SCRATCH (the exit status is the run'"'"'s own)" >&2 ;; esac
  if [ "$1" -eq 0 ] && [ "${_K2_ORDERLY:-0}" -ne 1 ]; then
    echo "!! CONTROL FAILED (the run ends only through a verdict): the run ended before reaching its verdict" >&2
    exit 1
  fi
  exit "$1"
}
trap '"'"'_k2_rc=$?; _k2_exit "$_k2_rc"'"'"' EXIT
_k2_sig() { echo "!! the run was stopped by SIG$1, so nothing was decided" >&2; exit "$2"; }
trap '"'"'_k2_sig TERM 143'"'"' TERM'
Wk="$H/work/f2"
case_body() { # $1 = version (r1|r2), $2 = case id; prints the body after the block
  local reapset; [ "$1" = r2 ] && reapset='_K2_REAP_D="$SCRATCH/v/.reap"; mkdir -p "$_K2_REAP_D"' || reapset=':'
  case "$2" in
    A-empty-orderly) echo "$reapset; _K2_ORDERLY=1; exit 0" ;;
    B-unset-early) echo '_K2_REAP_D=""; exit 2' ;;
    C-stale-group) [ "$1" = r2 ] && echo "$reapset; : > \"\$_K2_REAP_D/99999\"; _K2_ORDERLY=1; exit 0" || echo '_K2_REAP="-99999"; _K2_ORDERLY=1; exit 0' ;;
    D-live-group) [ "$1" = r2 ] && echo "$reapset; set -m; sleep 30 & p=\$!; set +m; echo \$p > \"$Wk/D.pid\"; : > \"\$_K2_REAP_D/\$p\"; exit 3" \
                   || echo "set -m; sleep 30 & p=\$!; set +m; echo \$p > \"$Wk/D.pid\"; _K2_REAP=\"-\$p\"; exit 3" ;;
    E-closed-stderr) [ "$1" = r2 ] && echo "$reapset; exec 9>&- 2>&-; exit 4" || echo 'exec 2>&-; exit 4' ;;
    E2-closed-stderr-unorderly) [ "$1" = r2 ] && echo "$reapset; exec 9>&- 2>&-; exit 0" || echo 'exec 2>&-; exit 0' ;;
    F-rm-fails) echo "$reapset; rm() { return 1; }; exit 5" ;;
    G-signal-in-exit) echo "$reapset; chmod() { kill -TERM \$\$; command chmod \"\$@\"; }; exit 6" ;;
    H-term-in-wait-redirected) echo "$reapset; ( sleep 1; kill -TERM \$\$ ) & sleep 5 & wait \$! 2>/dev/null; exit 0" ;;
    I-mut-trap-rmcopies-fails) [ "$1" = r2 ] && echo "$reapset; _mut_rm_copies() { return 1; }; trap '_k2_rc=\$?; set +e; _mut_rm_copies; _k2_exit \"\$_k2_rc\"' EXIT; exit 7" \
                   || echo "_mut_rm_copies() { return 1; }; trap '_k2_rc=\$?; _mut_rm_copies; _k2_exit \"\$_k2_rc\"' EXIT; exit 7" ;;
  esac
}
for sh in /opt/homebrew/bin/bash /bin/bash; do
  for v in r1 r2; do
    for k in A-empty-orderly B-unset-early C-stale-group D-live-group E-closed-stderr E2-closed-stderr-unorderly F-rm-fails G-signal-in-exit H-term-in-wait-redirected I-mut-trap-rmcopies-fails; do
      [ "$v" = r1 ] && [ "$k" = B-unset-early ] && continue
      S="$Wk/s.$$"; rm -rf "$S" "$Wk/D.pid"; mkdir -p "$S"
      [ "$v" = r2 ] && blk="$B2" || blk="$B1"
      body="set -euo pipefail
SCRATCH=\"$S\"
$blk
$(case_body $v $k)"
      rc=0; out="$(perl -e '$SIG{INT}="DEFAULT"; $SIG{TERM}="DEFAULT"; $SIG{HUP}="DEFAULT"; exec @ARGV' "$sh" -c "$body" 2>&1)" || rc=$?
      left=""; [ -e "$S" ] && left=" SCRATCH-LEFT"
      if [ -s "$Wk/D.pid" ]; then sleep 0.3; kill -0 "$(cat "$Wk/D.pid")" 2>/dev/null && { left="$left LIVE-GROUP-LEFT"; kill -9 "$(cat "$Wk/D.pid")"; }; fi
      printf '%s %s %-28s rc=%s%s out=[%s]\n' "$($sh -c 'echo ${BASH_VERSION%%(*}')" "$v" "$k" "$rc" "$left" "$(printf '%s' "$out" | tr '\n' '|' | cut -c1-160)"
      rm -rf "$S"
    done
  done
done
```

#### §E.f2probe3.sh

`f2probe3.sh` (sha256 `eab65a523a298634`, 52 lines)

```bash
#!/bin/bash
# f2probe3.sh <r2 clone> <r3 clone> <out>: the exit path under errexit, both shells, one fresh shell per case, the block
# between the R2PROTO-/R3PROTO-EXIT markers of each clone's wire run after `set -euo pipefail` as the wire runs it.
# Cases A-I as bin/f2probe.sh (r2's), plus: J (X x M: a TERM to the run during `_mut_rm_copies` in the mutation run's
# trap string, each version's own string), L (r3 only, `_k2_stop` stubbed to print its arguments: entries "", 0, 00,
# 01, 1, 12a, 5 -> only 5 passes the filter; never run against a real kill), M (a live group that ignores TERM: the
# exit path's time and the group gone), N (a live group of the shell's own child: the time), O (N with SHELLOPTS
# exported, so every bash the exit path starts would import errexit/nounset).
H=/Users/kazuaki/repos/send.sh/elidex-fgit-handoff/cr2; O="$3"; : > "$O"
blk() { awk '/^# R[23]PROTO-EXIT-BEGIN$/{on=1; next} /^# R[23]PROTO-EXIT-END$/{on=0} on' "$1/.claude/tools/webref-generic-core-trip-wire.sh"; }
B2="$(blk "$1")"; B3="$(blk "$2")"; Wk="$H/work/f2r3"; mkdir -p "$Wk"
mtrap() { [ "$1" = r3 ] && echo "trap '_k2_rc=\$?; trap \"\" HUP INT TERM; set +e; _mut_rm_copies; _k2_exit \"\$_k2_rc\"' EXIT" \
                         || echo "trap '_k2_rc=\$?; set +e; _mut_rm_copies; _k2_exit \"\$_k2_rc\"' EXIT"; }
case_body() { local rs='_K2_REAP_D="$SCRATCH/v/.reap"; mkdir -p "$_K2_REAP_D"'
  case "$2" in
    A-empty-orderly) echo "$rs; _K2_ORDERLY=1; exit 0" ;;
    B-unset-early) echo '_K2_REAP_D=""; exit 2' ;;
    C-stale-group) echo "$rs; : > \"\$_K2_REAP_D/99999\"; _K2_ORDERLY=1; exit 0" ;;
    D-live-group) echo "$rs; set -m; sleep 30 & p=\$!; set +m; echo \$p > \"$Wk/D.pid\"; : > \"\$_K2_REAP_D/\$p\"; exit 3" ;;
    E-closed-stderr) echo "$rs; exec 9>&- 2>&-; exit 4" ;;
    E2-closed-stderr-unorderly) echo "$rs; exec 9>&- 2>&-; exit 0" ;;
    F-rm-fails) echo "$rs; rm() { return 1; }; exit 5" ;;
    G-signal-in-exit) echo "$rs; chmod() { kill -TERM \$\$; command chmod \"\$@\"; }; exit 6" ;;
    H-term-in-wait-redirected) echo "$rs; ( sleep 1; kill -TERM \$\$ ) & sleep 5 & wait \$! 2>/dev/null; exit 0" ;;
    I-mut-trap-rmcopies-fails) echo "$rs; _mut_rm_copies() { return 1; }; $(mtrap "$1"); exit 7" ;;
    J-term-during-rmcopies) echo "$rs; _mut_rm_copies() { kill -TERM \$\$; sleep 0.3; return 0; }; $(mtrap "$1"); exit 7" ;;
    L-filter) echo "$rs; _k2_stop() { echo \"STOP [\$*]\" >&2; }; for e in 0 00 01 1 12a 5; do : > \"\$_K2_REAP_D/\$e\"; done; _K2_ORDERLY=1; exit 0" ;;
    M-term-ignored-group) echo "$rs; set -m; ( trap '' TERM; exec sleep 30 ) & p=\$!; set +m; echo \$p > \"$Wk/D.pid\"; : > \"\$_K2_REAP_D/\$p\"; exit 3" ;;
    N-live-child-timed) echo "$rs; set -m; sleep 30 & p=\$!; set +m; echo \$p > \"$Wk/D.pid\"; : > \"\$_K2_REAP_D/\$p\"; exit 3" ;;
    O-shellopts-exported) echo "$rs; export SHELLOPTS; set -m; sleep 30 & p=\$!; set +m; echo \$p > \"$Wk/D.pid\"; : > \"\$_K2_REAP_D/\$p\"; exit 3" ;;
  esac
}
now() { perl -MTime::HiRes=time -e 'printf "%.2f\n", time'; }
for sh in /opt/homebrew/bin/bash /bin/bash; do
  for v in r2 r3; do
    for k in A-empty-orderly B-unset-early C-stale-group D-live-group E-closed-stderr E2-closed-stderr-unorderly F-rm-fails G-signal-in-exit H-term-in-wait-redirected I-mut-trap-rmcopies-fails J-term-during-rmcopies L-filter M-term-ignored-group N-live-child-timed O-shellopts-exported; do
      [ "$v" = r2 ] && [ "$k" = L-filter ] && continue   # r2's filter passes 0 and 1: never run it against kill
      S="$Wk/s.$$"; rm -rf "$S" "$Wk/D.pid"; mkdir -p "$S"
      [ "$v" = r2 ] && blk="$B2" || blk="$B3"
      body="set -euo pipefail
SCRATCH=\"$S\"
$blk
$(case_body $v $k)"
      t0="$(now)"; rc=0; out="$(perl -e '$SIG{INT}="DEFAULT"; $SIG{TERM}="DEFAULT"; $SIG{HUP}="DEFAULT"; setpgrp; exec @ARGV' "$sh" -c "$body" 2>&1)" || rc=$?; t1="$(now)"
      left=""; [ -e "$S" ] && left=" SCRATCH-LEFT"
      if [ -s "$Wk/D.pid" ]; then sleep 0.3; p="$(cat "$Wk/D.pid")"; kill -0 -"$p" 2>/dev/null && { left="$left LIVE-GROUP-LEFT"; kill -9 -"$p"; }; fi
      printf '%s %s %-28s rc=%s %.2fs%s out=[%s]\n' "$($sh -c 'echo ${BASH_VERSION%%(*}')" "$v" "$k" "$rc" "$(T0="$t0" T1="$t1" awk 'BEGIN{print ENVIRON["T1"]-ENVIRON["T0"]}')" "$left" "$(printf '%s' "$out" | tr '\n' '|' | cut -c1-200)" >> "$O"
      rm -rf "$S"
    done
  done
done
echo F2PROBE3-DONE >> "$O"
```

#### §E.f3probe.sh

`f3probe.sh` (sha256 `8f3ec52241ab5438`, 20 lines)

```bash
#!/bin/bash
# f3probe.sh: is the exit path's line seen, by landing point, when it is written to fd 2 (as the frame does) or to a
# stderr saved on fd 9 at startup (r2)? trapprobe3's landings (`wait $! 2>/dev/null`, `wait $!`) plus the wire's other
# redirected landings: a launch block `{ … & wait; } 2>/dev/null`, a function call under `>/dev/null 2>&1` whose body
# is a subshell (the `_git … >/dev/null 2>&1` shape), a command substitution. TERM and INT (dispositions reset to
# default), both shells. Each line names which of SIG (the signal trap's line) and EXIT (the EXIT trap's line) was seen.
O=/Users/kazuaki/repos/send.sh/elidex-fgit-handoff/cr2/logs/f3probe.txt; : > "$O"; T=/Users/kazuaki/repos/send.sh/elidex-fgit-handoff/cr2/logs/f3.out
for sh in /opt/homebrew/bin/bash /bin/bash; do for fd in 2 9; do for sig in TERM INT; do
for land in waitredir waitplain launchblock fnredir cmdsub; do
  case $land in
    waitredir) b='sleep 5 & wait $! 2>/dev/null' ;; waitplain) b='sleep 5 & wait $!' ;;
    launchblock) b='{ sleep 5 & wait $!; } 2>/dev/null' ;; fnredir) b='f() { ( sleep 5 ); }; f >/dev/null 2>&1' ;;
    cmdsub) b='x=$(sleep 5)' ;;
  esac
  pre='exec 9>&2 || exec 9>/dev/null; '
  tr="trap 'echo EXIT >&$fd' EXIT; s() { echo SIG >&$fd; exit \$2; }; trap 's TERM 143' TERM; trap 's INT 130' INT; "
  perl -e '$SIG{INT}="DEFAULT"; $SIG{TERM}="DEFAULT"; $SIG{HUP}="DEFAULT"; setpgrp; exec @ARGV' "$sh" -c "$pre$tr$b; echo after >&2" > "$T" 2>&1 &
  pg=$!; sleep 1; kill -$sig -$pg; rc=0; wait $pg || rc=$?
  printf '%s fd=%s SIG%-4s %-11s rc=%s seen=[%s]\n' "$($sh -c 'echo ${BASH_VERSION%%(*}')" "$fd" "$sig" "$land" "$rc" "$(tr '\n' ' ' < "$T")" >> "$O"
done; done; done; done
```

#### §E.f3fdleak.sh

`f3fdleak.sh` (sha256 `201501760cf4a312`, 30 lines)

```bash
#!/bin/bash
# f3fdleak.sh: which descriptors does `_ctl_run`'s launch shape leave open in the WATCHDOG SUBSHELL (a bash that does
# not exec)? The launch is `set -m; { ( sleep N; true ) </dev/null >/dev/null 2>&1 [9>&-] & w=$!; } 2>/dev/null`, run
# by a shell whose stdout+stderr are a FIFO read by `cat`; lsof lists the subshell's numbered descriptors. Both shells,
# with and without `9>&-`, with the launch moved out of the `{ … } 2>/dev/null` group (the group's saved stderr), with the
# silencing done by an explicit save on fd 8 closed in the child (`explicit8`), and with the watchdog exec'ing a new bash;
# the `fnredir-*` variants put the launch inside a function called as `f >/dev/null 2>/dev/null` (the standing checks'
# `_ctl_run … > /dev/null 2> …` shape), whose saved descriptors the non-exec watchdog subshell inherits too.
H=/Users/kazuaki/repos/send.sh/elidex-fgit-handoff/cr2; D="$H/work/fdleak"; mkdir -p "$D"
for sh in /opt/homebrew/bin/bash /bin/bash; do for v in grp grp-close9 nogrp-close9 grp-close9-close10plus explicit8 grp-close9-execwd fnredir-explicit8 fnredir-grp-close9 fnredir-grp-close9-execwd; do
  f="$D/fifo.$$.$v"; [ -p "$f" ] || mkfifo "$f"
  case "$v" in
    grp) l='{ ( sleep 3; true ) </dev/null >/dev/null 2>&1 & w=$!; } 2>/dev/null' ;;
    grp-close9) l='{ ( sleep 3; true ) </dev/null >/dev/null 2>&1 9>&- & w=$!; } 2>/dev/null' ;;
    nogrp-close9) l='( sleep 3; true ) </dev/null >/dev/null 2>&1 9>&- & w=$!' ;;
    grp-close9-close10plus) l='{ ( exec 10>&- 11>&- 12>&- 255>&-; sleep 3; true ) </dev/null >/dev/null 2>&1 9>&- & w=$!; } 2>/dev/null' ;;
    explicit8) l='exec 8>&2 2>/dev/null || exec 8>/dev/null 2>/dev/null; ( sleep 3; true ) </dev/null >/dev/null 2>&1 8>&- 9>&- & w=$!; exec 2>&8 8>&-' ;;
    grp-close9-execwd) l='{ ( exec "$BASH" -c "sleep 3; true" ) </dev/null >/dev/null 2>&1 9>&- & w=$!; } 2>/dev/null' ;;
    fnredir-explicit8) l='f() { exec 8>&2 2>/dev/null || exec 8>/dev/null 2>/dev/null; ( sleep 3; true ) </dev/null >/dev/null 2>&1 8>&- 9>&- & w=$!; exec 2>&8 8>&-; }; f >/dev/null 2>/dev/null' ;;
    fnredir-grp-close9) l='f() { { ( sleep 3; true ) </dev/null >/dev/null 2>&1 9>&- & w=$!; } 2>/dev/null; }; f >/dev/null 2>/dev/null' ;;
    fnredir-grp-close9-execwd) l='f() { { ( exec "$BASH" -c "sleep 3; true" ) </dev/null >/dev/null 2>&1 9>&- & w=$!; } 2>/dev/null; }; f >/dev/null 2>/dev/null' ;;
  esac
  ( cat "$f" > /dev/null; perl -MTime::HiRes=time -e 'printf "%.2f\n", time' > "$f.eof" ) &
  r=$!
  t0="$(perl -MTime::HiRes=time -e 'printf "%.2f\n", time')"
  "$sh" -c 'exec 9>&2; set -m; '"$l"'; set +m; sleep 0.3; lsof -p "$w" 2>/dev/null | awk "\$4 ~ /^[0-9]+[urw]?\$/ {print \"    fd \" \$4, \$5, \$NF}" > "'"$f"'.lsof"; exit 0' > "$f" 2>&1
  wait "$r"
  printf '%s %-24s writer exit->EOF %.2f s; watchdog subshell fds:\n' "$("$sh" -c 'echo ${BASH_VERSION%%(*}')" "$v" "$(T0="$t0" T1="$(cat "$f.eof")" awk 'BEGIN { print ENVIRON["T1"] - ENVIRON["T0"] }')"
  cat "$f.lsof"
done; done
```

#### §E.f3fdleak3.sh

`f3fdleak3.sh` (sha256 `95454d00e20aad5d`, 35 lines)

```bash
#!/bin/bash
# f3fdleak3.sh (= f3fdleak.sh plus r3's watchdog shapes: `execenv` = `exec env -u SHELLOPTS -u BASH_ENV "$BASH" -c`,
# and `trapstay` = R11d's mutant `( trap : USR2; env … )`, plain and under a redirected function call): which descriptors does `_ctl_run`'s launch shape leave open in the WATCHDOG SUBSHELL (a bash that does
# not exec)? The launch is `set -m; { ( sleep N; true ) </dev/null >/dev/null 2>&1 [9>&-] & w=$!; } 2>/dev/null`, run
# by a shell whose stdout+stderr are a FIFO read by `cat`; lsof lists the subshell's numbered descriptors. Both shells,
# with and without `9>&-`, with the launch moved out of the `{ … } 2>/dev/null` group (the group's saved stderr), with the
# silencing done by an explicit save on fd 8 closed in the child (`explicit8`), and with the watchdog exec'ing a new bash;
# the `fnredir-*` variants put the launch inside a function called as `f >/dev/null 2>/dev/null` (the standing checks'
# `_ctl_run … > /dev/null 2> …` shape), whose saved descriptors the non-exec watchdog subshell inherits too.
H=/Users/kazuaki/repos/send.sh/elidex-fgit-handoff/cr2; D="$H/work/fdleak3"; mkdir -p "$D"
for sh in /opt/homebrew/bin/bash /bin/bash; do for v in grp grp-close9 nogrp-close9 grp-close9-close10plus explicit8 grp-close9-execwd fnredir-explicit8 fnredir-grp-close9 fnredir-grp-close9-execwd grp-close9-execenv fnredir-grp-close9-execenv grp-close9-trapstay fnredir-grp-close9-trapstay; do
  f="$D/fifo.$$.$v"; [ -p "$f" ] || mkfifo "$f"
  case "$v" in
    grp) l='{ ( sleep 3; true ) </dev/null >/dev/null 2>&1 & w=$!; } 2>/dev/null' ;;
    grp-close9) l='{ ( sleep 3; true ) </dev/null >/dev/null 2>&1 9>&- & w=$!; } 2>/dev/null' ;;
    nogrp-close9) l='( sleep 3; true ) </dev/null >/dev/null 2>&1 9>&- & w=$!' ;;
    grp-close9-close10plus) l='{ ( exec 10>&- 11>&- 12>&- 255>&-; sleep 3; true ) </dev/null >/dev/null 2>&1 9>&- & w=$!; } 2>/dev/null' ;;
    explicit8) l='exec 8>&2 2>/dev/null || exec 8>/dev/null 2>/dev/null; ( sleep 3; true ) </dev/null >/dev/null 2>&1 8>&- 9>&- & w=$!; exec 2>&8 8>&-' ;;
    grp-close9-execwd) l='{ ( exec "$BASH" -c "sleep 3; true" ) </dev/null >/dev/null 2>&1 9>&- & w=$!; } 2>/dev/null' ;;
    fnredir-explicit8) l='f() { exec 8>&2 2>/dev/null || exec 8>/dev/null 2>/dev/null; ( sleep 3; true ) </dev/null >/dev/null 2>&1 8>&- 9>&- & w=$!; exec 2>&8 8>&-; }; f >/dev/null 2>/dev/null' ;;
    fnredir-grp-close9) l='f() { { ( sleep 3; true ) </dev/null >/dev/null 2>&1 9>&- & w=$!; } 2>/dev/null; }; f >/dev/null 2>/dev/null' ;;
    fnredir-grp-close9-execwd) l='f() { { ( exec "$BASH" -c "sleep 3; true" ) </dev/null >/dev/null 2>&1 9>&- & w=$!; } 2>/dev/null; }; f >/dev/null 2>/dev/null' ;;
    grp-close9-execenv) l='{ ( exec env -u SHELLOPTS -u BASH_ENV "$BASH" -c "sleep 3; true" ) </dev/null >/dev/null 2>&1 9>&- & w=$!; } 2>/dev/null' ;;
    fnredir-grp-close9-execenv) l='f() { { ( exec env -u SHELLOPTS -u BASH_ENV "$BASH" -c "sleep 3; true" ) </dev/null >/dev/null 2>&1 9>&- & w=$!; } 2>/dev/null; }; f >/dev/null 2>/dev/null' ;;
    grp-close9-trapstay) l='{ ( trap : USR2; env -u SHELLOPTS -u BASH_ENV "$BASH" -c "sleep 3; true" ) </dev/null >/dev/null 2>&1 9>&- & w=$!; } 2>/dev/null' ;;
    fnredir-grp-close9-trapstay) l='f() { { ( trap : USR2; env -u SHELLOPTS -u BASH_ENV "$BASH" -c "sleep 3; true" ) </dev/null >/dev/null 2>&1 9>&- & w=$!; } 2>/dev/null; }; f >/dev/null 2>/dev/null' ;;
  esac
  ( cat "$f" > /dev/null; perl -MTime::HiRes=time -e 'printf "%.2f\n", time' > "$f.eof" ) &
  r=$!
  t0="$(perl -MTime::HiRes=time -e 'printf "%.2f\n", time')"
  "$sh" -c 'exec 9>&2; set -m; '"$l"'; set +m; sleep 0.3; lsof -p "$w" 2>/dev/null | awk "\$4 ~ /^[0-9]+[urw]?\$/ {print \"    fd \" \$4, \$5, \$NF}" > "'"$f"'.lsof"; exit 0' > "$f" 2>&1
  wait "$r"
  printf '%s %-24s writer exit->EOF %.2f s; watchdog subshell fds:\n' "$("$sh" -c 'echo ${BASH_VERSION%%(*}')" "$v" "$(T0="$t0" T1="$(cat "$f.eof")" awk 'BEGIN { print ENVIRON["T1"] - ENVIRON["T0"] }')"
  cat "$f.lsof"
done; done
```

#### §E.f3hang.sh

`f3hang.sh` (sha256 `0a3f55e4c701289f`, 22 lines)

```bash
#!/bin/bash
# f3hang.sh <clone> <b53|b32> <log>: the fd-inheritance hang check (F3). The run's stdout+stderr go through a FIFO to a
# reader (`cat`); when the instrumented `_ev` stub has started (`.ev_run.mark`: a `_ctl_run` child and its 30 s watchdog
# are live), the run's own group gets SIGKILL (no trap runs, so nothing is reaped). Measured: seconds from the kill to
# the reader's EOF, i.e. how long a process the run left holds the caller's stream open; then the leftovers (killed).
c="$1"; s="$2"; log="$3"; case "$s" in b53) sh=/opt/homebrew/bin/bash ;; b32) sh=/bin/bash ;; esac
H=/Users/kazuaki/repos/send.sh/elidex-fgit-handoff/cr2; TD="$H/work/td.$(basename "$log")"; rm -rf "$TD"; mkdir -p "$TD"; mkfifo "$TD/fifo"
{ cat "$TD/fifo" > "$log.out"; perl -MTime::HiRes=time -e 'printf "%.2f\n", time' > "$TD/eof"; } &
rd=$!
cd "$c" || exit 2
TMPDIR="$TD" perl -e '$SIG{INT}="DEFAULT"; $SIG{TERM}="DEFAULT"; $SIG{HUP}="DEFAULT"; setpgrp; exec @ARGV' "$sh" .claude/tools/webref-generic-core-trip-wire.sh </dev/null > "$TD/fifo" 2>&1 &
pg=$!
i=0; while [ "$i" -lt 2400 ] && [ -z "$(find "$TD" -name .ev_run.mark 2>/dev/null | head -n 1)" ]; do sleep 0.05; i=$((i+1)); done
snap="$(ps -A -o pid=,ppid=,pgid=,command= | ROOT="$pg" awk '{ pid[NR]=$1; pp[NR]=$2; pg[NR]=$3 } END { d[ENVIRON["ROOT"]]=1; ch=1
  while (ch) { ch=0; for (i=1;i<=NR;i++) if (!(pid[i] in d) && (pp[i] in d)) { d[pid[i]]=1; ch=1 } } for (i=1;i<=NR;i++) if ((pid[i] in d) && pg[i] != ENVIRON["ROOT"]) print pg[i] }' | sort -u | tr '\n' ' ')"
t0="$(perl -MTime::HiRes=time -e 'printf "%.2f\n", time')"; kill -9 -"$pg"; wait "$pg" 2>/dev/null
j=0; while [ "$j" -lt 900 ] && [ ! -s "$TD/eof" ]; do sleep 0.1; j=$((j+1)); done
t1="$(cat "$TD/eof" 2>/dev/null)"
held="$(for g in $snap; do lsof -a -g "$g" -d 9 2>/dev/null | tail -n +2; done | cut -c1-120)"
{ echo "### $s mark-wait=${i}x0.05s groups=[$snap] kill->EOF=$(T0="$t0" T1="$t1" awk 'BEGIN { if (ENVIRON["T1"] == "") print "none in 90 s"; else printf "%.2f s", ENVIRON["T1"] - ENVIRON["T0"] }')"
  echo "### fd 9 still open in the run's groups after EOF (lsof):"; printf '%s\n' "$held"; } > "$log"
for g in $snap; do kill -9 -"$g" 2>/dev/null; done; kill "$rd" 2>/dev/null; rm -rf "$TD"
```

#### §E.g12probe.sh

`g12probe.sh` (sha256 `b55573995d59c836`, 28 lines)

```bash
#!/bin/bash
# g12probe.sh <out>: the watchdog's environment (G12). A shell under `set -euo pipefail` with SHELLOPTS exported (as a
# caller's `export SHELLOPTS` makes it) and BASH_ENV naming a file that marks it was read, launches a target group
# (`sleep 30`) and a watchdog in each form, with the bound UNSET (`${_ctl_wd_s-}` = "", case R8nowd): the watchdog must
# still write its marker and kill the target. Forms: frame (a subshell: sleep; true > mark; kill -9), r2 (`exec "$BASH"
# -c` the same three steps), r3 (`exec env -u SHELLOPTS -u BASH_ENV "$BASH" -c` the steps then the stop procedure, the
# text of bin/r3c2-proto.py). The launch is inside a function called as `L || true`, as every `_ctl_run` caller is an
# `||`/`if !` operand (errexit off there); BASH_ENV is exported after the probe shell started. Both shells; each cell
# with and without the export.
O="$1"; : > "$O"; D=/Users/kazuaki/repos/send.sh/elidex-fgit-handoff/cr2/work/g12; mkdir -p "$D"
printf ': > "%s/benv.read"\n' "$D" > "$D/benv"
STOP='SECONDS=0
for p; do kill -TERM -"$p"; done
for p; do while kill -0 -"$p" && [ "$SECONDS" -lt 2 ]; do sleep 0.1 || sleep 1; done; done
for p; do kill -9 -"$p"; done; exit 0'
for sh in /opt/homebrew/bin/bash /bin/bash; do for ex in no yes; do for f in frame r2 r3; do
  rm -f "$D/mark" "$D/benv.read"
  case $f in
    frame) w='( sleep "${_ctl_wd_s-}"; true > "$_r_wmark" || true; kill -9 -"$_cpid" ) </dev/null >/dev/null 2>&1 & _wpid=$!' ;;
    r2) w='( exec "$BASH" -c '"'"'sleep "$1"; true > "$2" || true; kill -9 -"$3"'"'"' k2wd "${_ctl_wd_s-}" "$_r_wmark" "$_cpid" ) </dev/null >/dev/null 2>&1 & _wpid=$!' ;;
    r3) w='( exec env -u SHELLOPTS -u BASH_ENV "$BASH" -c '"'"'sleep "$1"; true > "$2" || true; shift 2; '"'"'"$_K2_STOP" k2wd "${_ctl_wd_s-}" "$_r_wmark" "$_cpid" ) </dev/null >/dev/null 2>&1 & _wpid=$!' ;;
  esac
  pre='set -euo pipefail; '; [ $ex = yes ] && pre="$pre"'export SHELLOPTS; '
  body="$pre"'export BASH_ENV=$3; _K2_STOP=$1; _r_wmark=$2; L() { set -m; sleep 30 & _cpid=$!; '"$w"'; set +m; }; L || true; sleep 1.5; kill -0 -"$_cpid" 2>/dev/null && { echo TARGET-ALIVE; kill -9 -"$_cpid"; } || echo target-killed; exit 0'
  out="$(perl -e 'setpgrp; exec @ARGV' "$sh" -c "$body" g12 "$STOP" "$D/mark" "$D/benv" 2>&1)"
  printf '%s export=%-3s %-5s mark=%s bash_env_read=%s %s\n' "$($sh -c 'echo ${BASH_VERSION%%(*}')" $ex $f "$([ -e "$D/mark" ] && echo yes || echo NO)" "$([ -e "$D/benv.read" ] && echo YES || echo no)" "$(printf '%s' "$out" | tr '\n' ' ')" >> "$O"
done; done; done
echo G12-DONE >> "$O"
```

#### §E.r2-optin.sh

`r2-optin.sh` (sha256 `7533a0117f296de2`, 37 lines)

```bash
#!/bin/bash
# r2-optin.sh <clone> <b53|b32> <log> <records.sh line> [sigdefault]: the real opt-in path filtered to ONE record (the
# crc2-fix/Bc-t2.sh method: `[ "$_mut_n" = idx ] || continue` after `_mut_parse "$_mrec"`, the generated set stubbed),
# idx computed from the clone's own records.sh; `sigdefault` runs the wire with HUP/INT/TERM reset to default (perl);
# `ignINT` with SIGINT ignored at entry (HUP/TERM default).
set -eu
H=/Users/kazuaki/repos/send.sh/elidex-fgit-handoff/cr2; c="$1"; s="$2"; log="$3"; ln="$4"; mode="${5:-}"
T="$c/.claude/tools/webref-generic-core-trip-wire"
idx="$(python3 -I - "$T.records.sh" "$ln" <<'PY'
import sys
rec=open(sys.argv[1]).read().split('\n'); ln=int(sys.argv[2])
start=next(i for i,l in enumerate(rec) if l=="  done <<'MUTANTS'")
assert rec[ln-1] and not rec[ln-1].startswith('#'), rec[ln-1]
print(1+sum(1 for i in range(start+1,ln-1) if rec[i] and not rec[i].startswith('#')))
PY
)"
python3 -I - "$T.mutations.sh" "$idx" <<'PY'
import sys
p=sys.argv[1]; idx=sys.argv[2]; s=open(p).read()
a='      _mut_parse "$_mrec"\n'; assert s.count(a)==1
s=s.replace(a,a+'      [ "$_mut_n" = "'+idx+'" ] || continue\n')
b='    _mut_gen_run\n'; assert s.count(b)==1
s=s.replace(b,'    :\n'); open(p,'w').write(s)
PY
case "$s" in b53) sh=/opt/homebrew/bin/bash ;; b32) sh=/bin/bash ;; esac
{ echo "### clone=$c shell=$s record line=$ln idx=$idx mode=$mode"; echo "### record: $(sed -n "${ln}p" "$T.records.sh" | cut -c1-200)"; } > "$log"
cd "$c"; rc=0; set +e
if [ "$mode" = ignINT ]; then
  env WEBREF_WIRE_MUTANTS=1 perl -e '$SIG{INT}="IGNORE"; $SIG{TERM}="DEFAULT"; $SIG{HUP}="DEFAULT"; setpgrp; alarm 900; exec @ARGV' "$sh" .claude/tools/webref-generic-core-trip-wire.sh </dev/null >> "$log" 2>&1 & pg=$!
elif [ "$mode" = sigdefault ]; then
  env WEBREF_WIRE_MUTANTS=1 perl -e '$SIG{INT}="DEFAULT"; $SIG{TERM}="DEFAULT"; $SIG{HUP}="DEFAULT"; setpgrp; alarm 900; exec @ARGV' "$sh" .claude/tools/webref-generic-core-trip-wire.sh </dev/null >> "$log" 2>&1 & pg=$!
else
  env WEBREF_WIRE_MUTANTS=1 perl -e 'setpgrp; alarm 900; exec @ARGV' "$sh" .claude/tools/webref-generic-core-trip-wire.sh </dev/null >> "$log" 2>&1 & pg=$!
fi
perl -e 'sleep 900; kill 9, -$ARGV[0]' "$pg" </dev/null >/dev/null 2>&1 & wd=$!  # the kill -9 watchdog (bash 3.2 ignores the alarm)
wait "$pg" || rc=$?; kill "$wd" 2>/dev/null
echo "### end RC=$rc" >> "$log"
```

#### §E.r2-runsig.sh

`r2-runsig.sh` (sha256 `4907970ece8fcbff`, 12 lines)

```bash
#!/bin/bash
# r2-runsig.sh <clone> <b53|b32> <log> <dispositions: default|ignINT|ignHUP>: one ship run, its own process group,
# with HUP/INT/TERM reset to default, or with INT (resp. HUP) IGNORED at entry (the nohup / non-interactive `&` case).
c="$1"; s="$2"; log="$3"; m="$4"; case "$s" in b53) sh=/opt/homebrew/bin/bash ;; b32) sh=/bin/bash ;; esac
case "$m" in default) pl='$SIG{INT}="DEFAULT"; $SIG{TERM}="DEFAULT"; $SIG{HUP}="DEFAULT";' ;;
  ignINT) pl='$SIG{INT}="IGNORE"; $SIG{TERM}="DEFAULT"; $SIG{HUP}="DEFAULT";' ;;
  ignHUP) pl='$SIG{INT}="DEFAULT"; $SIG{TERM}="DEFAULT"; $SIG{HUP}="IGNORE";' ;; esac
echo "### clone=$c shell=$s dispositions=$m" > "$log"; cd "$c"; rc=0
perl -e "$pl"' setpgrp; alarm 900; exec @ARGV' "$sh" .claude/tools/webref-generic-core-trip-wire.sh </dev/null >> "$log" 2>&1 & pg=$!
perl -e 'sleep 900; kill 9, -$ARGV[0]' "$pg" </dev/null >/dev/null 2>&1 & wd=$!  # the kill -9 watchdog (bash 3.2 ignores the alarm)
wait "$pg" || rc=$?; kill "$wd" 2>/dev/null
echo "### end RC=$rc" >> "$log"
```

#### §E.r3-optin.sh

`r3-optin.sh` (sha256 `723ba6ef9d9c9f89`, 33 lines)

```bash
#!/bin/bash
# r3-optin.sh <clone> <b53|b32> <log> <records.sh line> <default|ignHUP|ignINT|ignTERM>: (= r2-optin.sh with every disposition mode) the real opt-in path filtered to ONE record (the
# crc2-fix/Bc-t2.sh method: `[ "$_mut_n" = idx ] || continue` after `_mut_parse "$_mrec"`, the generated set stubbed),
# idx computed from the clone's own records.sh; `sigdefault` runs the wire with HUP/INT/TERM reset to default (perl);
# `ignINT` with SIGINT ignored at entry (HUP/TERM default).
set -eu
H=/Users/kazuaki/repos/send.sh/elidex-fgit-handoff/cr2; c="$1"; s="$2"; log="$3"; ln="$4"; mode="${5:-}"
T="$c/.claude/tools/webref-generic-core-trip-wire"
idx="$(python3 -I - "$T.records.sh" "$ln" <<'PY'
import sys
rec=open(sys.argv[1]).read().split('\n'); ln=int(sys.argv[2])
start=next(i for i,l in enumerate(rec) if l=="  done <<'MUTANTS'")
assert rec[ln-1] and not rec[ln-1].startswith('#'), rec[ln-1]
print(1+sum(1 for i in range(start+1,ln-1) if rec[i] and not rec[i].startswith('#')))
PY
)"
python3 -I - "$T.mutations.sh" "$idx" <<'PY'
import sys
p=sys.argv[1]; idx=sys.argv[2]; s=open(p).read()
a='      _mut_parse "$_mrec"\n'; assert s.count(a)==1
s=s.replace(a,a+'      [ "$_mut_n" = "'+idx+'" ] || continue\n')
b='    _mut_gen_run\n'; assert s.count(b)==1
s=s.replace(b,'    :\n'); open(p,'w').write(s)
PY
case "$s" in b53) sh=/opt/homebrew/bin/bash ;; b32) sh=/bin/bash ;; esac
{ echo "### clone=$c shell=$s record line=$ln idx=$idx mode=$mode"; echo "### record: $(sed -n "${ln}p" "$T.records.sh" | cut -c1-200)"; } > "$log"
cd "$c"; rc=0; set +e
pl='$SIG{INT}="DEFAULT"; $SIG{TERM}="DEFAULT"; $SIG{HUP}="DEFAULT";'
case "$mode" in ign*) pl="$pl \$SIG{${mode#ign}}=\"IGNORE\";" ;; default|sigdefault) : ;; *) echo "bad mode $mode"; exit 2 ;; esac
env WEBREF_WIRE_MUTANTS=1 perl -e "$pl"' setpgrp; alarm 900; exec @ARGV' "$sh" .claude/tools/webref-generic-core-trip-wire.sh </dev/null >> "$log" 2>&1 & pg=$!
perl -e 'sleep 900; kill 9, -$ARGV[0]' "$pg" </dev/null >/dev/null 2>&1 & wd=$!  # the kill -9 watchdog (bash 3.2 ignores the alarm)
wait "$pg" || rc=$?; kill "$wd" 2>/dev/null
echo "### end RC=$rc" >> "$log"
```

#### §E.r3-runsig.sh

`r3-runsig.sh` (sha256 `f50ff501695689ab`, 11 lines)

```bash
#!/bin/bash
# r3-runsig.sh <clone> <b53|b32> <log> <default|ignHUP|ignINT|ignTERM>: one ship run, its own process group, HUP/INT/TERM
# default, or the named one IGNORED at entry (the nohup / non-interactive `&` / `trap "" TERM` caller); kill -9 watchdog.
c="$1"; s="$2"; log="$3"; m="$4"; case "$s" in b53) sh=/opt/homebrew/bin/bash ;; b32) sh=/bin/bash ;; esac
pl='$SIG{INT}="DEFAULT"; $SIG{TERM}="DEFAULT"; $SIG{HUP}="DEFAULT";'
case "$m" in ign*) pl="$pl \$SIG{${m#ign}}=\"IGNORE\";" ;; default) : ;; *) echo "bad mode"; exit 2 ;; esac
echo "### clone=$c shell=$s dispositions=$m text=$(cd "$c" && git diff --stat | tail -1)" > "$log"; cd "$c"; rc=0
perl -e "$pl"' setpgrp; alarm 900; exec @ARGV' "$sh" .claude/tools/webref-generic-core-trip-wire.sh </dev/null >> "$log" 2>&1 & pg=$!
perl -e 'sleep 900; kill 9, -$ARGV[0]' "$pg" </dev/null >/dev/null 2>&1 & wd=$!  # the kill -9 watchdog (bash 3.2 ignores the alarm)
wait "$pg" || rc=$?; kill "$wd" 2>/dev/null
echo "### end RC=$rc" >> "$log"
```

#### §E.r3-tdrun.sh

`r3-tdrun.sh` (sha256 `fbea3f5ae00c09a6`, 14 lines)

```bash
#!/bin/bash
# r3-tdrun.sh <clone> <b53|b32> <log>: one ship run in its own process group (HUP/INT/TERM default, kill -9 watchdog),
# TMPDIR its own and NOT deleted (work/td3/<log name>); after the run: the scratch dirs (`tmp.*`) left there (G2's
# watchdog-timeout count, with bin/r3-instr.py wdhang).
c="$1"; s="$2"; log="$3"; case "$s" in b53) sh=/opt/homebrew/bin/bash ;; b32) sh=/bin/bash ;; esac
H=/Users/kazuaki/repos/send.sh/elidex-fgit-handoff/cr2; TD="$H/work/td3/$(basename "$log")"; rm -rf "$TD"; mkdir -p "$TD"
cd "$c" || exit 2; rc=0
TMPDIR="$TD" perl -e '$SIG{INT}="DEFAULT"; $SIG{TERM}="DEFAULT"; $SIG{HUP}="DEFAULT"; setpgrp; exec @ARGV' "$sh" .claude/tools/webref-generic-core-trip-wire.sh </dev/null > "$log.out" 2>&1 & pg=$!
perl -e 'sleep 600; kill 9, -$ARGV[0]' "$pg" </dev/null >/dev/null 2>&1 & wd=$!
wait "$pg" || rc=$?; kill "$wd" 2>/dev/null; sleep 1
{ echo "### $s rc=$rc clone=$c text=$(cd "$c" && git diff --stat | tail -1)"
  echo "### wdhang lines:"; /usr/bin/grep -n 'wdhang probe' "$log.out" | cut -c1-200
  echo "### scratch dirs left under the run's TMPDIR: $(find "$TD" -maxdepth 1 -name 'tmp.*' -type d | wc -l | tr -d ' ')"
  find "$TD" -maxdepth 1 -name 'tmp.*' -type d | sed 's/^/    /'; } > "$log"
```

#### §E.r3-trialsig.sh

`r3-trialsig.sh` (sha256 `cb55e2386c863371`, 44 lines)

```bash
#!/bin/bash
# r3-trialsig.sh <clone> <b53|b32> <log> <records.sh line> <SIG>: G8's trial-nesting edge. The real opt-in path filtered to
# one record (as bin/r2-optin.sh), HUP/INT/TERM default, its own process group and TMPDIR (kept); once the trial's wire
# has made its own verifier directory (a second `verify*` under the TMPDIR: the trial is inside its controls) plus 2 s,
# <SIG> goes to the run's group. Logged: rc, the last lines, the processes left in the run's descendants' groups 1 s
# after the end, the scratch dirs and the `.mutant.` copies in .claude/tools left.
set -u
H=/Users/kazuaki/repos/send.sh/elidex-fgit-handoff/cr2; c="$1"; s="$2"; log="$3"; ln="$4"; sig="$5"
T="$c/.claude/tools/webref-generic-core-trip-wire"
idx="$(python3 -I - "$T.records.sh" "$ln" <<'PY'
import sys
rec=open(sys.argv[1]).read().split('\n'); ln=int(sys.argv[2])
start=next(i for i,l in enumerate(rec) if l=="  done <<'MUTANTS'")
print(1+sum(1 for i in range(start+1,ln-1) if rec[i] and not rec[i].startswith('#')))
PY
)"
python3 -I - "$T.mutations.sh" "$idx" <<'PY'
import sys
p=sys.argv[1]; idx=sys.argv[2]; s=open(p).read()
a='      _mut_parse "$_mrec"\n'; assert s.count(a)==1
s=s.replace(a,a+'      [ "$_mut_n" = "'+idx+'" ] || continue\n')
b='    _mut_gen_run\n'; assert s.count(b)==1
s=s.replace(b,'    :\n'); open(p,'w').write(s)
PY
case "$s" in b53) sh=/opt/homebrew/bin/bash ;; b32) sh=/bin/bash ;; esac
TD="$H/work/td3/$(basename "$log")"; rm -rf "$TD"; mkdir -p "$TD"; cd "$c"
TMPDIR="$TD" WEBREF_WIRE_MUTANTS=1 perl -e '$SIG{INT}="DEFAULT"; $SIG{TERM}="DEFAULT"; $SIG{HUP}="DEFAULT"; setpgrp; exec @ARGV' "$sh" .claude/tools/webref-generic-core-trip-wire.sh </dev/null > "$log.out" 2>&1 & pg=$!
perl -e 'sleep 600; kill 9, -$ARGV[0]' "$pg" </dev/null >/dev/null 2>&1 & wd=$!
i=0; while [ "$i" -lt 6000 ] && [ "$(find "$TD" -maxdepth 3 -name 'verify*' -type d 2>/dev/null | wc -l)" -lt 2 ]; do sleep 0.05; i=$((i+1)); done
sleep 2
snapg="$(ps -A -o pid=,ppid=,pgid= | ROOT="$pg" awk '{ pid[NR]=$1; pp[NR]=$2; pg[NR]=$3 } END { d[ENVIRON["ROOT"]]=1; ch=1
  while (ch) { ch=0; for (i=1;i<=NR;i++) if (!(pid[i] in d) && (pp[i] in d)) { d[pid[i]]=1; ch=1 } } for (i=1;i<=NR;i++) if (pid[i] in d) print pg[i] }' | sort -u | tr '\n' ' ')"
nproc="$(ps -A -o pid=,ppid= | ROOT="$pg" awk '{ pid[NR]=$1; pp[NR]=$2 } END { d[ENVIRON["ROOT"]]=1; ch=1; n=0
  while (ch) { ch=0; for (i=1;i<=NR;i++) if (!(pid[i] in d) && (pp[i] in d)) { d[pid[i]]=1; ch=1 } } for (i=1;i<=NR;i++) if (pid[i] in d) n++; print n }')"
kill -"$sig" -"$pg"; rc=0; wait "$pg" || rc=$?; kill "$wd" 2>/dev/null; sleep 1
left="$(ps -A -o pid=,pgid=,command= | G="$snapg" awk 'BEGIN { n=split(ENVIRON["G"], a, " "); for (i=1;i<=n;i++) g[a[i]]=1 } ($2 in g) {print}')"
{ echo "### $s SIG$sig record line=$ln idx=$idx rc=$rc (verify dirs seen after ${i}x0.05 s + 2 s; $nproc processes in the run's tree at the signal)"
  echo "### clone=$c text=$(git diff --stat | tail -1)"
  echo "### last lines:"; tail -n 3 "$log.out" | cut -c1-200
  echo "### groups of the run's tree at the signal: [$snapg]"
  echo "### left in those groups 1 s after: $(printf '%s' "$left" | /usr/bin/grep -c .)"; printf '%s\n' "$left" | cut -c1-160
  echo "### scratch dirs left: $(find "$TD" -maxdepth 1 -name 'tmp.*' -type d | wc -l | tr -d ' ')"
  echo "### mutant copies left in .claude/tools: $(ls .claude/tools | /usr/bin/grep -c 'mutant\.')"; } > "$log"
for g in $snapg; do [ "$g" = "$pg" ] || kill -9 -"$g" 2>/dev/null; done
```

#### §E.run.sh

`run.sh` (sha256 `ab8f01ebc23f9da8`, 16 lines)

```bash
#!/bin/bash
# run.sh <clone dir> <b53|b32> <log path> [env assignments...]
# Runs the wire once in its own process group with a 900 s alarm; header = clone HEAD, tool digest, shell.
set -u
c="$(cd "$1" && pwd)"; s="$2"; log="$(cd "$(dirname "$3")" && pwd)/$(basename "$3")"; shift 3
case "$s" in b53) sh=/opt/homebrew/bin/bash ;; b32) sh=/bin/bash ;; *) echo bad shell; exit 2 ;; esac
{
  echo "### clone=$c head=$(git -C "$c" rev-parse --short HEAD) shell=$s ($("$sh" -c 'echo $BASH_VERSION'))"
  echo "### tool digest=$(cd "$c/.claude/tools" && cat webref-generic-core-trip-wire*.sh | shasum -a 256 | cut -c1-12) diff-vs-HEAD=$(git -C "$c" diff --stat | tail -1)"
  echo "### env: $*"
  echo "### start $(date +%s)"
} > "$log"
cd "$c" || exit 2
rc=0
env "$@" perl -e 'setpgrp; alarm 900; exec @ARGV' "$sh" .claude/tools/webref-generic-core-trip-wire.sh </dev/null >> "$log" 2>&1 || rc=$?
echo "### end $(date +%s) RC=$rc" >> "$log"
```
