# K2 fixture git — PR #527 CR2 round: the instruments (as run), §E.3, 1 of 2 — the exit path's texts [r2], [r3], [r3f]

Companion of `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-cr2-sweep.md` ("the sweep file"), which holds CR2's sweep, §E.1 and §E.4,
and whose §E index names the file of every instrument. The first half of §E.3 was split out of it at CR2-M (touch-time, the
1000-line rule), each block byte-identical. The plan is `…-cr2-plan.md`, with its companion
`…-cr2-plan-exitpath.md` (written there as x§6.n). **Frame `706049ad`.** How a `cr2/bin/<name>` resolves, how a
block is re-run and what it reads: the sweep file's header and §E.4. This file is excluded from CR2's sweep and
from the plan's claim rows over `docs/plans/*k2-fgit*`, as the sweep file is (the plan's §10 and §11).

## §0 Spec coverage map

**No spec surface** — as the plan's §0: this file holds a shell trip-wire's instruments, as run. `preflight.py`
exits 1 here by design, because no table follows this heading.

## §E Appendix — the instruments (as run), continued

One fenced block per file, under its file name, as in the sweep file's §E.

### §E.3 The exit path's texts and their batches (1 of 2): [r2], [r3], [r3f]

#### §E.r2c2-proto.py

`r2c2-proto.py` (sha256 `f5d4fe972e1202c0`, 147 lines)

```python
import sys
# r2c2-proto.py <clone> [variant ...]: MECHANISM PROTOTYPE (not the slice's text) of CR2-C2 as planned in r2:
#  wire   : stderr saved on fd 9 before any trap (F3); the reap set carried in files, one per live group, under
#           `$_K2_REAP_D` (= `$_VFY/.reap`, set by controls) (F1); every exit-path body runs `set +e` first (F2) and
#           ignores HUP/INT/TERM from its first statement; one trap line per signal (F4); `wait` after each reap (F22).
#  ctlrun : both launches close fd 9; the watchdog execs a fresh bash (no bash-saved descriptor survives in it); once both
#           are launched the groups are recorded (`_k2_reap_add`), the reap drops them. (The fd-8 form tried first:
#           bin/r2c2-proto.fd8-rejected.py, rejected by measurement, b3-f3hang-closed.*.)
#  controls: the reap directory made beside `$_VFY`; ONE call line, three rows (HUP/INT/TERM), each row's expectation
#           decided by a probe of whether that signal can reach a child of this run (ignored at entry: the run is not
#           stopped, PASSED); label literal.
#  fixtures: three PATH-first `git` shims that mark `path` and send their signal to their own group on the HEAD probe
#           (`rev-parse --verify`, called under `>/dev/null 2>&1`).
#  controls/fixtures: R12's control over `$CTL/failrm` (an `rm` that fails `rm -rf`, a `chmod` that fails `chmod -R`).
#  records: R11h / R11i / R11t (one trap line each removed), R12 (the removal's `||` dropped), R12e (`_k2_exit`'s `set +e`
#           dropped). R11f (`_k2_sig`'s line sent to fd 2) is the `r11f` variant: measured SURVIVED, not adopted.
#  mutations: the mutation run's trap string runs `set +e` first.
# Variants (measurement-only edits, applied after the prototype):
#  noreap   : `_k2_exit` does not call `_k2_reap` (CR11-R's removed arm)
#  nofdclose: both launches keep fd 9 (the fd-inheritance arm)
#  noexecwd : the watchdog is the frame's non-exec subshell (fd 9 still closed)
#  fd2      : `_k2_sig` and `_k2_exit` write to fd 2 (the frame's choice)
#  instr-ev / instr-cap: the `_ev` / `_cap` standing check's stub marks `<stub>.mark` and sleeps 3 s (a landing point)
#  norow-HUP / norow-INT / norow-TERM: that row removed from the signal control's list (the survive arms)
#  noprobe  : the rows' deliverability probe removed; nofailrm: R12's control removed (orphans R12/R12e: not a survive arm)
#  r12msg   : R12's control expects `PASSED` instead of the removal's line (R12's survive arm)
#  nochmodshim: `failrm` builds no `chmod` shim (R12e's survive arm)
d = sys.argv[1]; T = d + "/.claude/tools/webref-generic-core-trip-wire"
def rep(p, a, b, n=1):
    s = open(p).read(); assert s.count(a) == n, (p, s.count(a), a[:70]); open(p, "w").write(s.replace(a, b))
W = T + ".sh"
rep(W, '_K2_ORDERLY=0\n_k2_exit() { # $1 = the status the shell was leaving with\n'
       '  case "$SCRATCH" in /*/*) chmod -R u+rwX "$SCRATCH" 2>/dev/null || true; rm -rf "$SCRATCH";; esac\n'
       '  if [ "$1" -eq 0 ] && [ "${_K2_ORDERLY:-0}" -ne 1 ]; then\n'
       '    echo "!! CONTROL FAILED (${_ord_lbl:-the run ends only through a verdict}): the run ended before reaching its verdict" >&2\n'
       '    exit 1\n  fi\n  exit "$1"\n}\n'
       "trap '_k2_rc=$?; _k2_exit \"$_k2_rc\"' EXIT\n",
 '# R2PROTO-EXIT-BEGIN\n'
 'exec 9>&2 || exec 9>/dev/null\n'
 '_K2_ORDERLY=0; _K2_REAP_D=""\n'
 '_k2_reap() { local _kr_f _kr_p\n'
 '  [ -n "$_K2_REAP_D" ] || return 0\n'
 '  for _kr_f in "$_K2_REAP_D"/*; do _kr_p="${_kr_f##*/}"\n'
 '    case "$_kr_p" in \'\'|*[!0-9]*) continue ;; esac\n'
 '    kill -9 -"$_kr_p" 2>/dev/null; wait "$_kr_p" 2>/dev/null\n'
 '  done; }\n'
 '_k2_reap_add() { local _ka_for="$1" _ka_p; shift\n'
 '  for _ka_p in "$@"; do [ -n "$_K2_REAP_D" ] && : > "$_K2_REAP_D/$_ka_p" 2>/dev/null && continue\n'
 '    for _ka_p in "$@"; do kill -9 -"$_ka_p" 2>/dev/null; done\n'
 '    echo "!! CONTROL FAILED ($_ka_for): the reap set could not record a process group, so a signal would leave it running" >&2; return 1\n'
 '  done; }\n'
 '_k2_reap_del() { local _kd_for="$1" _kd_p; shift\n'
 '  for _kd_p in "$@"; do command rm -f "$_K2_REAP_D/$_kd_p" 2>/dev/null || {\n'
 '    echo "!! CONTROL FAILED ($_kd_for): the reap set could not drop a reaped group" >&2; return 1; }; done; }\n'
 '_k2_exit() { # $1 = the status the shell was leaving with\n'
 "  trap '' HUP INT TERM; set +e\n"
 '  _k2_reap\n'
 '  case "$SCRATCH" in /*/*) chmod -R u+rwX "$SCRATCH" 2>/dev/null\n'
 '    rm -rf "$SCRATCH" || echo "!! could not remove the scratch dir $SCRATCH (the exit status is the run\'s own)" >&9 ;; esac\n'
 '  if [ "$1" -eq 0 ] && [ "${_K2_ORDERLY:-0}" -ne 1 ]; then\n'
 '    echo "!! CONTROL FAILED (${_ord_lbl:-the run ends only through a verdict}): the run ended before reaching its verdict" >&9\n'
 '    exit 1\n  fi\n  exit "$1"\n}\n'
 "trap '_k2_rc=$?; _k2_exit \"$_k2_rc\"' EXIT\n"
 '_k2_sig() { set +e; echo "!! the run was stopped by SIG$1, so nothing was decided" >&9; exit "$2"; }\n'
 "trap '_k2_sig HUP 129' HUP\n"
 "trap '_k2_sig INT 130' INT\n"
 "trap '_k2_sig TERM 143' TERM\n"
 '# R2PROTO-EXIT-END\n')
C = T + ".ctlrun.sh"
# The launches: both close fd 9; the WATCHDOG execs a fresh bash, so no bash process of the run's children keeps the
# descriptors bash saves around a redirected group or a redirected function call (`{ … } 2>/dev/null`, and a standing
# check's `_ctl_run … > /dev/null 2> …`): bash marks those close-on-exec, and the child execs the wire already
# (bin/f3fdleak.sh). The silencing group stays as it is (its tostop evidence unchanged). The reap set records both
# groups once both are launched.
rep(C, '    < /dev/null > "$_out_f" 2>&1 & _cpid=$!; } 2>/dev/null\n', '    < /dev/null > "$_out_f" 2>&1 9>&- & _cpid=$!; } 2>/dev/null\n')
rep(C, '  { ( sleep "${_ctl_wd_s-}"\n      true > "$_r_wmark" || true\n      kill -9 -"$_cpid" ) < /dev/null > /dev/null 2>&1 & _wpid=$!; } 2>/dev/null\n',
       '  { ( exec "$BASH" -c \'sleep "$1"; true > "$2" || true; kill -9 -"$3"\' k2wd "${_ctl_wd_s-}" "$_r_wmark" "$_cpid" ) \\\n'
       '      < /dev/null > /dev/null 2>&1 9>&- & _wpid=$!; } 2>/dev/null\n')
rep(C, '  _ctl_wd_pg="$_wpid"\n', '  _ctl_wd_pg="$_wpid"; _k2_reap_add "$_r_for" "$_cpid" "$_wpid" || return 1\n')
rep(C, '  kill -9 -"$_wpid" 2>/dev/null || true; wait "$_wpid" 2>/dev/null || true\n',
       '  kill -9 -"$_wpid" 2>/dev/null || true; wait "$_wpid" 2>/dev/null || true; _k2_reap_del "$_r_for" "$_cpid" "$_wpid" || return 1\n')
K = T + ".controls.sh"
rep(K, 'if ! _VFY="$(mktemp -d "$SCRATCH/verifyXXXXXX")" || [ -z "$_VFY" ] || [ ! -d "$_VFY" ]; then\n',
       'if ! _VFY="$(mktemp -d "$SCRATCH/verifyXXXXXX")" || [ -z "$_VFY" ] || [ ! -d "$_VFY" ] || ! mkdir "$_VFY/.reap"; then\n')
rep(K, '  echo "!! could not create the verifier\'s directory (TMPDIR/disk?); decided nothing." >&2\n  exit 2\nfi\n',
       '  echo "!! could not create the verifier\'s directory (TMPDIR/disk?); decided nothing." >&2\n  exit 2\nfi\n_K2_REAP_D="$_VFY/.reap"\n')
SIGROWS = 'HUP:129 INT:130 TERM:143'
rep(K, '_control "$CTL/k2" 1 "K2: a" "a caller\'s GREP_OPTIONS cannot hide a file" "" "" "$CTL/goptgrep" "" "GREP_OPTIONS=--exclude=*" || ctl_ok=1\n',
       '_control "$CTL/k2" 1 "K2: a" "a caller\'s GREP_OPTIONS cannot hide a file" "" "" "$CTL/goptgrep" "" "GREP_OPTIONS=--exclude=*" || ctl_ok=1\n'
       'for _sg in ' + SIGROWS + '; do _sg_n="${_sg%:*}"; _sg_e="${_sg#*:}"; _sg_m="stopped by SIG$_sg_n"\n'
       '  { sh -c "kill -$_sg_n \\$\\$; exit 0"; } 2>/dev/null && { _sg_e=0; _sg_m=PASSED; }\n'
       '  _control "$CTL/clean" "$_sg_e" "$_sg_m" "a signal ends the run red and says so" "" "" "$CTL/sig$_sg_n" || ctl_ok=1\n'
       'done\n'
       '_control "$CTL/clean" 0 "could not remove the scratch dir" "a scratch dir that cannot be removed does not change the verdict" "" "" "$CTL/failrm" || ctl_ok=1\n')
F = T + ".fixtures.sh"
rep(F, '_fgit_shim "$CTL/goptgrep/grep" \': > "${K2_CTL_REACHED:?}/path"; [ -z "${GREP_OPTIONS+x}" ] || exit 1\' "$_REAL_GREP"\n',
       '_fgit_shim "$CTL/goptgrep/grep" \': > "${K2_CTL_REACHED:?}/path"; [ -z "${GREP_OPTIONS+x}" ] || exit 1\' "$_REAL_GREP"\n'
       'for _sg in HUP INT TERM; do mkdir -p "$CTL/sig$_sg"\n'
       '  _fgit_shim "$CTL/sig$_sg/git" \'case " $* " in *" rev-parse --verify "*) : > "${K2_CTL_REACHED:?}/path"; kill -\'"$_sg"\' 0 ;; esac\' "$_REAL_GIT"; done\n'
       'mkdir -p "$CTL/failrm"\n'
       '_fgit_shim "$CTL/failrm/rm" \'case "$1" in -rf) : > "${K2_CTL_REACHED:?}/path"; rm "$@"; exit 1 ;; esac\' rm\n'
       '_fgit_shim "$CTL/failrm/chmod" \'case "$1" in -R) chmod "$@"; exit 1 ;; esac\' chmod\n')
R = T + ".records.sh"
NEEDLE = "a signal ends the run red and says so"
N12 = "a scratch dir that cannot be removed does not change the verdict"
recs = ("s/^trap '_k2_sig HUP 129' HUP$/:/\t" + NEEDLE + "\n"
        "s/^trap '_k2_sig INT 130' INT$/:/\t" + NEEDLE + "\n"
        "s/^trap '_k2_sig TERM 143' TERM$/:/\t" + NEEDLE + "\n"
        "s/^    rm -rf \"[$]SCRATCH\" || echo \"!! could not remove the scratch dir [$]SCRATCH (the exit status is the run's own)\" >&9 ;; esac$/    rm -rf \"$SCRATCH\" ;; esac/\t" + N12 + "\n"
        "s/^  trap '' HUP INT TERM; set +e$/  trap '' HUP INT TERM/\t" + N12 + "\n")
R11F = "s/, so nothing was decided\" >&9; exit \"[$]2\"; }$/, so nothing was decided\" >\\&2; exit \"$2\"; }/\t" + NEEDLE + "\n"
rep(R, "MUTANTS\n}\n", recs + "MUTANTS\n}\n")
M = T + ".mutations.sh"
rep(M, "    trap '_k2_rc=$?; _mut_rm_copies; _k2_exit \"$_k2_rc\"' EXIT\n",
       "    trap '_k2_rc=$?; set +e; _mut_rm_copies; _k2_exit \"$_k2_rc\"' EXIT\n")
for v in sys.argv[2:]:
    if v == "noreap": rep(W, "  _k2_reap\n  case", "  : _k2_reap\n  case")
    elif v == "nofdclose":
        rep(C, ' 2>&1 9>&- & _cpid=$!', ' 2>&1 & _cpid=$!'); rep(C, ' 2>&1 9>&- & _wpid=$!', ' 2>&1 & _wpid=$!')
    elif v == "noexecwd":
        rep(C, '  { ( exec "$BASH" -c \'sleep "$1"; true > "$2" || true; kill -9 -"$3"\' k2wd "${_ctl_wd_s-}" "$_r_wmark" "$_cpid" ) \\\n',
               '  { ( sleep "${_ctl_wd_s-}"; true > "$_r_wmark" || true; kill -9 -"$_cpid" ) \\\n')
    elif v == "fd2":
        rep(W, ', so nothing was decided" >&9;', ', so nothing was decided" >&2;')
        rep(W, ' before reaching its verdict" >&9\n', ' before reaching its verdict" >&2\n')
    elif v == "instr-ev":
        rep(K, "printf '%s\\n' 'k2evprobe 2>/dev/null; exit 0' > \"$_VFY/.ev_run\"\n",
               "printf '%s\\n' ': > \"$0.mark\"; sleep 3; k2evprobe 2>/dev/null; exit 0' > \"$_VFY/.ev_run\"  # INSTRUMENT\n")
    elif v == "instr-cap":
        rep(K, "printf '%s\\n' 'exit 0' > \"$_VFY/.cap_run\"\n",
               "printf '%s\\n' ': > \"$0.mark\"; sleep 3; exit 0' > \"$_VFY/.cap_run\"  # INSTRUMENT\n")
    elif v == "instr-sigshow":
        rep(K, '  { sh -c "kill -$_sg_n \\$\\$; exit 0"; } 2>/dev/null && { _sg_e=0; _sg_m=PASSED; }\n',
               '  { sh -c "kill -$_sg_n \\$\\$; exit 0"; } 2>/dev/null && { _sg_e=0; _sg_m=PASSED; }; echo "SIGROW $_sg_n expects $_sg_e" >&2  # INSTRUMENT\n')
    elif v.startswith("norow-"):
        sg = v[6:]; rows = " ".join(r for r in SIGROWS.split() if not r.startswith(sg + ":"))
        rep(K, "for _sg in " + SIGROWS + "; do", "for _sg in " + rows + "; do")
    elif v == "r11f": rep(R, "MUTANTS\n}\n", R11F + "MUTANTS\n}\n")
    elif v == "noprobe":
        rep(K, '  { sh -c "kill -$_sg_n \\$\\$; exit 0"; } 2>/dev/null && { _sg_e=0; _sg_m=PASSED; }\n', '')
    elif v == "nofailrm":
        rep(K, '_control "$CTL/clean" 0 "could not remove the scratch dir" "a scratch dir that cannot be removed does not change the verdict" "" "" "$CTL/failrm" || ctl_ok=1\n', '')
    elif v == "r12msg":
        rep(K, '_control "$CTL/clean" 0 "could not remove the scratch dir" "a scratch dir', '_control "$CTL/clean" 0 "PASSED" "a scratch dir')
    elif v == "nochmodshim":
        rep(F, '_fgit_shim "$CTL/failrm/chmod" \'case "$1" in -R) chmod "$@"; exit 1 ;; esac\' chmod\n', '')
    else: raise SystemExit("unknown variant " + v)
```

#### §E.r2c2-proto.fd8-rejected.py

`r2c2-proto.fd8-rejected.py` (sha256 `55be95c1ec961f5f`, 122 lines)

```python
import sys
# r2c2-proto.py <clone> [variant ...]: MECHANISM PROTOTYPE (not the slice's text) of CR2-C2 as planned in r2:
#  wire   : stderr saved on fd 9 before any trap (F3); the reap set carried in files, one per live group, under
#           `$_K2_REAP_D` (= `$_VFY/.reap`, set by controls) (F1); every exit-path body runs `set +e` first (F2) and
#           ignores HUP/INT/TERM from its first statement; one trap line per signal (F4); `wait` after each reap (F22).
#  ctlrun : the launches are silenced through an explicit save on fd 8 (not a `{ … } 2>/dev/null` group); both children
#           close fds 8 and 9; once both are launched the groups are recorded (`_k2_reap_add`), the reap drops them.
#  controls: the reap directory made beside `$_VFY`; ONE call line, three rows (HUP/INT/TERM), each row's expectation
#           decided by a probe of whether that signal can reach a child of this run (ignored at entry: the run is not
#           stopped, PASSED); label literal.
#  fixtures: three PATH-first `git` shims that mark `path` and send their signal to their own group on the HEAD probe
#           (`rev-parse --verify`, called under `>/dev/null 2>&1`).
#  records: R11h / R11i / R11t (one trap line each removed) and R11f (`_k2_sig`'s line sent to fd 2).
#  mutations: the mutation run's trap string runs `set +e` first.
# Variants (measurement-only edits, applied after the prototype):
#  noreap   : `_k2_exit` does not call `_k2_reap` (CR11-R's removed arm)
#  nofdclose: both launches keep fd 9 (the fd-inheritance arm; fd 8 still closed)
#  fd2      : `_k2_sig` and `_k2_exit` write to fd 2 (the frame's choice)
#  instr-ev / instr-cap: the `_ev` / `_cap` standing check's stub marks `<stub>.mark` and sleeps 3 s (a landing point)
#  norow-HUP / norow-INT / norow-TERM: that row removed from the signal control's list (the survive arms)
d = sys.argv[1]; T = d + "/.claude/tools/webref-generic-core-trip-wire"
def rep(p, a, b, n=1):
    s = open(p).read(); assert s.count(a) == n, (p, s.count(a), a[:70]); open(p, "w").write(s.replace(a, b))
W = T + ".sh"
rep(W, '_K2_ORDERLY=0\n_k2_exit() { # $1 = the status the shell was leaving with\n'
       '  case "$SCRATCH" in /*/*) chmod -R u+rwX "$SCRATCH" 2>/dev/null || true; rm -rf "$SCRATCH";; esac\n'
       '  if [ "$1" -eq 0 ] && [ "${_K2_ORDERLY:-0}" -ne 1 ]; then\n'
       '    echo "!! CONTROL FAILED (${_ord_lbl:-the run ends only through a verdict}): the run ended before reaching its verdict" >&2\n'
       '    exit 1\n  fi\n  exit "$1"\n}\n'
       "trap '_k2_rc=$?; _k2_exit \"$_k2_rc\"' EXIT\n",
 '# R2PROTO-EXIT-BEGIN\n'
 'exec 9>&2 || exec 9>/dev/null\n'
 '_K2_ORDERLY=0; _K2_REAP_D=""\n'
 '_k2_reap() { local _kr_f _kr_p\n'
 '  [ -n "$_K2_REAP_D" ] || return 0\n'
 '  for _kr_f in "$_K2_REAP_D"/*; do _kr_p="${_kr_f##*/}"\n'
 '    case "$_kr_p" in \'\'|*[!0-9]*) continue ;; esac\n'
 '    kill -9 -"$_kr_p" 2>/dev/null; wait "$_kr_p" 2>/dev/null\n'
 '  done; }\n'
 '_k2_reap_add() { local _ka_for="$1" _ka_p; shift\n'
 '  for _ka_p in "$@"; do [ -n "$_K2_REAP_D" ] && : > "$_K2_REAP_D/$_ka_p" 2>/dev/null && continue\n'
 '    for _ka_p in "$@"; do kill -9 -"$_ka_p" 2>/dev/null; done\n'
 '    echo "!! CONTROL FAILED ($_ka_for): the reap set could not record a process group, so a signal would leave it running" >&2; return 1\n'
 '  done; }\n'
 '_k2_reap_del() { local _kd_for="$1" _kd_p; shift\n'
 '  for _kd_p in "$@"; do command rm -f "$_K2_REAP_D/$_kd_p" 2>/dev/null || {\n'
 '    echo "!! CONTROL FAILED ($_kd_for): the reap set could not drop a reaped group" >&2; return 1; }; done; }\n'
 '_k2_exit() { # $1 = the status the shell was leaving with\n'
 "  trap '' HUP INT TERM; set +e\n"
 '  _k2_reap\n'
 '  case "$SCRATCH" in /*/*) chmod -R u+rwX "$SCRATCH" 2>/dev/null\n'
 '    rm -rf "$SCRATCH" || echo "!! could not remove the scratch dir $SCRATCH (the exit status is the run\'s own)" >&9 ;; esac\n'
 '  if [ "$1" -eq 0 ] && [ "${_K2_ORDERLY:-0}" -ne 1 ]; then\n'
 '    echo "!! CONTROL FAILED (${_ord_lbl:-the run ends only through a verdict}): the run ended before reaching its verdict" >&9\n'
 '    exit 1\n  fi\n  exit "$1"\n}\n'
 "trap '_k2_rc=$?; _k2_exit \"$_k2_rc\"' EXIT\n"
 '_k2_sig() { set +e; echo "!! the run was stopped by SIG$1, so nothing was decided" >&9; exit "$2"; }\n'
 "trap '_k2_sig HUP 129' HUP\n"
 "trap '_k2_sig INT 130' INT\n"
 "trap '_k2_sig TERM 143' TERM\n"
 '# R2PROTO-EXIT-END\n')
C = T + ".ctlrun.sh"
# The launches: the silencing moves from bash's implicit save (`{ … & } 2>/dev/null`, whose saved copy of the caller's
# stderr - fd 10 on both shells - the watchdog subshell inherits and holds: bin/f3fdleak.sh) to an explicit save on fd 8,
# closed in both children with fd 9; the reap set records both groups once both are launched.
rep(C, '  set -m\n', '  set -m; exec 8>&2 2>/dev/null || exec 8>/dev/null 2>/dev/null\n')
rep(C, '  { ( [ -z "$_r_cwd" ] || cd "$_r_cwd" || exit 97\n', '  ( [ -z "$_r_cwd" ] || cd "$_r_cwd" || exit 97\n')
rep(C, '    < /dev/null > "$_out_f" 2>&1 & _cpid=$!; } 2>/dev/null\n', '    < /dev/null > "$_out_f" 2>&1 8>&- 9>&- & _cpid=$!\n')
rep(C, '  { ( sleep "${_ctl_wd_s-}"\n', '  ( sleep "${_ctl_wd_s-}"\n')
rep(C, '      kill -9 -"$_cpid" ) < /dev/null > /dev/null 2>&1 & _wpid=$!; } 2>/dev/null\n',
       '      kill -9 -"$_cpid" ) < /dev/null > /dev/null 2>&1 8>&- 9>&- & _wpid=$!\n')
rep(C, '  set +m\n', '  set +m; exec 2>&8 8>&-\n')
rep(C, '  _ctl_wd_pg="$_wpid"\n', '  _ctl_wd_pg="$_wpid"; _k2_reap_add "$_r_for" "$_cpid" "$_wpid" || return 1\n')
rep(C, '  kill -9 -"$_wpid" 2>/dev/null || true; wait "$_wpid" 2>/dev/null || true\n',
       '  kill -9 -"$_wpid" 2>/dev/null || true; wait "$_wpid" 2>/dev/null || true; _k2_reap_del "$_r_for" "$_cpid" "$_wpid" || return 1\n')
K = T + ".controls.sh"
rep(K, 'if ! _VFY="$(mktemp -d "$SCRATCH/verifyXXXXXX")" || [ -z "$_VFY" ] || [ ! -d "$_VFY" ]; then\n',
       'if ! _VFY="$(mktemp -d "$SCRATCH/verifyXXXXXX")" || [ -z "$_VFY" ] || [ ! -d "$_VFY" ] || ! mkdir "$_VFY/.reap"; then\n')
rep(K, '  echo "!! could not create the verifier\'s directory (TMPDIR/disk?); decided nothing." >&2\n  exit 2\nfi\n',
       '  echo "!! could not create the verifier\'s directory (TMPDIR/disk?); decided nothing." >&2\n  exit 2\nfi\n_K2_REAP_D="$_VFY/.reap"\n')
SIGROWS = 'HUP:129 INT:130 TERM:143'
rep(K, '_control "$CTL/k2" 1 "K2: a" "a caller\'s GREP_OPTIONS cannot hide a file" "" "" "$CTL/goptgrep" "" "GREP_OPTIONS=--exclude=*" || ctl_ok=1\n',
       '_control "$CTL/k2" 1 "K2: a" "a caller\'s GREP_OPTIONS cannot hide a file" "" "" "$CTL/goptgrep" "" "GREP_OPTIONS=--exclude=*" || ctl_ok=1\n'
       'for _sg in ' + SIGROWS + '; do _sg_n="${_sg%:*}"; _sg_e="${_sg#*:}"; _sg_m="stopped by SIG$_sg_n"\n'
       '  { sh -c "kill -$_sg_n \\$\\$; exit 0"; } 2>/dev/null && { _sg_e=0; _sg_m=PASSED; }\n'
       '  _control "$CTL/clean" "$_sg_e" "$_sg_m" "a signal ends the run red and says so" "" "" "$CTL/sig$_sg_n" || ctl_ok=1\n'
       'done\n')
F = T + ".fixtures.sh"
rep(F, '_fgit_shim "$CTL/goptgrep/grep" \': > "${K2_CTL_REACHED:?}/path"; [ -z "${GREP_OPTIONS+x}" ] || exit 1\' "$_REAL_GREP"\n',
       '_fgit_shim "$CTL/goptgrep/grep" \': > "${K2_CTL_REACHED:?}/path"; [ -z "${GREP_OPTIONS+x}" ] || exit 1\' "$_REAL_GREP"\n'
       'for _sg in HUP INT TERM; do mkdir -p "$CTL/sig$_sg"\n'
       '  _fgit_shim "$CTL/sig$_sg/git" \'case " $* " in *" rev-parse --verify "*) : > "${K2_CTL_REACHED:?}/path"; kill -\'"$_sg"\' 0 ;; esac\' "$_REAL_GIT"; done\n')
R = T + ".records.sh"
NEEDLE = "a signal ends the run red and says so"
recs = ("s/^trap '_k2_sig HUP 129' HUP$/:/\t" + NEEDLE + "\n"
        "s/^trap '_k2_sig INT 130' INT$/:/\t" + NEEDLE + "\n"
        "s/^trap '_k2_sig TERM 143' TERM$/:/\t" + NEEDLE + "\n"
        "s/, so nothing was decided\" >&9; exit \"[$]2\"; }$/, so nothing was decided\" >\\&2; exit \"$2\"; }/\t" + NEEDLE + "\n")
rep(R, "MUTANTS\n}\n", recs + "MUTANTS\n}\n")
M = T + ".mutations.sh"
rep(M, "    trap '_k2_rc=$?; _mut_rm_copies; _k2_exit \"$_k2_rc\"' EXIT\n",
       "    trap '_k2_rc=$?; set +e; _mut_rm_copies; _k2_exit \"$_k2_rc\"' EXIT\n")
for v in sys.argv[2:]:
    if v == "noreap": rep(W, "  _k2_reap\n  case", "  : _k2_reap\n  case")
    elif v == "nofdclose":
        rep(C, ' 2>&1 8>&- 9>&- & _cpid=$!', ' 2>&1 8>&- & _cpid=$!'); rep(C, ' 2>&1 8>&- 9>&- & _wpid=$!', ' 2>&1 8>&- & _wpid=$!')
    elif v == "fd2":
        rep(W, ', so nothing was decided" >&9;', ', so nothing was decided" >&2;')
        rep(W, ' before reaching its verdict" >&9\n', ' before reaching its verdict" >&2\n')
    elif v == "instr-ev":
        rep(K, "printf '%s\\n' 'k2evprobe 2>/dev/null; exit 0' > \"$_VFY/.ev_run\"\n",
               "printf '%s\\n' ': > \"$0.mark\"; sleep 3; k2evprobe 2>/dev/null; exit 0' > \"$_VFY/.ev_run\"  # INSTRUMENT\n")
    elif v == "instr-cap":
        rep(K, "printf '%s\\n' 'exit 0' > \"$_VFY/.cap_run\"\n",
               "printf '%s\\n' ': > \"$0.mark\"; sleep 3; exit 0' > \"$_VFY/.cap_run\"  # INSTRUMENT\n")
    elif v == "instr-sigshow":
        rep(K, '  { sh -c "kill -$_sg_n \\$\\$; exit 0"; } 2>/dev/null && { _sg_e=0; _sg_m=PASSED; }\n',
               '  { sh -c "kill -$_sg_n \\$\\$; exit 0"; } 2>/dev/null && { _sg_e=0; _sg_m=PASSED; }; echo "SIGROW $_sg_n expects $_sg_e" >&2  # INSTRUMENT\n')
    elif v.startswith("norow-"):
        sg = v[6:]; rows = " ".join(r for r in SIGROWS.split() if not r.startswith(sg + ":"))
        rep(K, "for _sg in " + SIGROWS + "; do", "for _sg in " + rows + "; do")
    else: raise SystemExit("unknown variant " + v)
```

#### §E.r3c2-proto.py

`r3c2-proto.py` (sha256 `73643c0edab38a84`, 207 lines)

```python
import sys
# r3c2-proto.py <clone> [variant ...]: MECHANISM PROTOTYPE (not the slice's text) of CR2-C2 as planned in r3.
# FROZEN: the r3 measurements (logs b6-*, b7-*, b8-*, r3-*) ran on this text; it is never edited after them. A new
# mechanism text gets a new file name.
# Changes from r2 (bin/r2c2-proto.py):
#  wire   : ONE stop procedure, `$_K2_STOP` (TERM each group, poll until all are gone or the stop process is 2 s old,
#           KILL what is left), run as its own process with SHELLOPTS and BASH_ENV removed from its environment, by the
#           reap (`_k2_stop`) and by the watchdog (G2, G12); the reap set's entries accepted only as numbers > 1 (G13);
#           a failed reap-set write ends the call with exit 2 and a line on fd 9 after stopping its groups (G1, G6);
#           `_k2_sig` ignores HUP/INT/TERM first as `_k2_exit` does (X).
#  ctlrun : each group recorded right after its own launch (G6: per-launch); the watchdog execs the stop procedure after
#           its sleep and marker, through `env -u SHELLOPTS -u BASH_ENV` (G12).
#  controls: the `_wm` hang stub traps TERM (writes `<stub>.term`) and the hang row asserts it when TERM is deliverable
#           (R11w's subject); a fourth standing check, `_og` (label `_og_lbl`), rows kill / norec / nodel (G4, G6).
#  mutations: the trap string ignores HUP/INT/TERM before `_mut_rm_copies` (G11).
#  records: R11h/i/t, R12, R12e (r2's), R11w (the stop procedure's TERM step), R11d (the watchdog's exec), R11a (the add
#           failure's exit), R11x (the drop failure's exit).
# Variants (measurement-only edits applied after the prototype):
#  noreap, nofdclose, noexecwd (the watchdog a bash that stays: `( trap : USR2; env … )`, as R11d; dropping `exec` alone
#  changes nothing, both shells exec a subshell's only command), killonly (the stop procedure
#  without its TERM step), fd2, instr-sigshow, norow-<SIG>, noprobe, nofailrm, r12msg, nochmodshim, r11f,
#  wm-noterm (the `_wm` .term assertion removed: R11w's survive arm), og-nokill / og-nonorec / og-nonodel (that `_og`
#  row's assertions removed: the survive arms of R11d / R11a / R11x). Instruments for any clone: bin/r3-instr.py.
d = sys.argv[1]; T = d + "/.claude/tools/webref-generic-core-trip-wire"
def rep(p, a, b, n=1):
    s = open(p).read(); assert s.count(a) == n, (p, s.count(a), a[:70]); open(p, "w").write(s.replace(a, b))
W = T + ".sh"
STOP_TERM = 'for p; do kill -TERM -"$p"; done\n'
rep(W, '_K2_ORDERLY=0\n_k2_exit() { # $1 = the status the shell was leaving with\n'
       '  case "$SCRATCH" in /*/*) chmod -R u+rwX "$SCRATCH" 2>/dev/null || true; rm -rf "$SCRATCH";; esac\n'
       '  if [ "$1" -eq 0 ] && [ "${_K2_ORDERLY:-0}" -ne 1 ]; then\n'
       '    echo "!! CONTROL FAILED (${_ord_lbl:-the run ends only through a verdict}): the run ended before reaching its verdict" >&2\n'
       '    exit 1\n  fi\n  exit "$1"\n}\n'
       "trap '_k2_rc=$?; _k2_exit \"$_k2_rc\"' EXIT\n",
 '# R3PROTO-EXIT-BEGIN\n'
 'exec 9>&2 || exec 9>/dev/null\n'
 '_K2_ORDERLY=0; _K2_REAP_D=""\n'
 "_K2_STOP='" + STOP_TERM +
 'for p; do while kill -0 -"$p" && [ "$SECONDS" -lt 2 ]; do sleep 0.1 || sleep 1; done; done\n'
 "for p; do kill -9 -\"$p\"; done; exit 0'\n"
 '_k2_stop() { local _ks_p\n'
 '  env -u SHELLOPTS -u BASH_ENV "$BASH" -c "$_K2_STOP" k2stop "$@" < /dev/null > /dev/null 2>&1 9>&- \\\n'
 '    || for _ks_p in "$@"; do kill -9 -"$_ks_p" 2>/dev/null; done\n'
 '  for _ks_p in "$@"; do wait "$_ks_p" 2>/dev/null; done; return 0; }\n'
 '_k2_reap() { local _kr_f _kr_p; local -a _kr_g=()\n'
 '  [ -n "$_K2_REAP_D" ] || return 0\n'
 '  for _kr_f in "$_K2_REAP_D"/*; do _kr_p="${_kr_f##*/}"\n'
 '    case "$_kr_p" in \'\'|*[!0-9]*|0*|1) continue ;; esac\n'
 '    _kr_g+=("$_kr_p"); done\n'
 '  [ "${#_kr_g[@]}" -eq 0 ] || _k2_stop "${_kr_g[@]}"; }\n'
 '_k2_reap_add() { # $1 = the label, $2 = a group just launched, $3... = the call\'s groups already recorded\n'
 '  local _ka_p; [ -n "$_K2_REAP_D" ] && : > "$_K2_REAP_D/$2" 2>/dev/null && return 0\n'
 '  echo "!! $1: the reap set could not record process group $2, so a signal would leave it running; decided nothing" >&9\n'
 '  for _ka_p in "${@:3}"; do command rm -f "$_K2_REAP_D/$_ka_p" 2>/dev/null; done\n'
 '  _k2_stop "${@:2}"; exit 2; }\n'
 '_k2_reap_del() { # $1 = the label, $2... = the call\'s groups, stopped\n'
 '  local _kd_p; for _kd_p in "${@:2}"; do command rm -f "$_K2_REAP_D/$_kd_p" 2>/dev/null && continue\n'
 '    echo "!! $1: the reap set could not drop process group $_kd_p, so the run\'s end could stop another\'s; decided nothing" >&9; exit 2; done; }\n'
 '_k2_exit() { # $1 = the status the shell was leaving with\n'
 "  trap '' HUP INT TERM; set +e\n"
 '  _k2_reap\n'
 '  case "$SCRATCH" in /*/*) chmod -R u+rwX "$SCRATCH" 2>/dev/null\n'
 '    rm -rf "$SCRATCH" || echo "!! could not remove the scratch dir $SCRATCH (the exit status is the run\'s own)" >&9 ;; esac\n'
 '  if [ "$1" -eq 0 ] && [ "${_K2_ORDERLY:-0}" -ne 1 ]; then\n'
 '    echo "!! CONTROL FAILED (${_ord_lbl:-the run ends only through a verdict}): the run ended before reaching its verdict" >&9\n'
 '    exit 1\n  fi\n  exit "$1"\n}\n'
 "trap '_k2_rc=$?; _k2_exit \"$_k2_rc\"' EXIT\n"
 "_k2_sig() { trap '' HUP INT TERM; set +e; echo \"!! the run was stopped by SIG$1, so nothing was decided\" >&9; exit \"$2\"; }\n"
 "trap '_k2_sig HUP 129' HUP\n"
 "trap '_k2_sig INT 130' INT\n"
 "trap '_k2_sig TERM 143' TERM\n"
 '# R3PROTO-EXIT-END\n')
C = T + ".ctlrun.sh"
WD_EXEC = ('  { ( exec env -u SHELLOPTS -u BASH_ENV "$BASH" -c \'sleep "$1"; true > "$2" || true; shift 2; \'"$_K2_STOP" \\\n'
           '      k2wd "${_ctl_wd_s-}" "$_r_wmark" "$_cpid" ) < /dev/null > /dev/null 2>&1 9>&- & _wpid=$!; } 2>/dev/null\n')
rep(C, '    < /dev/null > "$_out_f" 2>&1 & _cpid=$!; } 2>/dev/null\n',
       '    < /dev/null > "$_out_f" 2>&1 9>&- & _cpid=$!; } 2>/dev/null; _k2_reap_add "$_r_for" "$_cpid"\n')
rep(C, '  { ( sleep "${_ctl_wd_s-}"\n      true > "$_r_wmark" || true\n      kill -9 -"$_cpid" ) < /dev/null > /dev/null 2>&1 & _wpid=$!; } 2>/dev/null\n', WD_EXEC)
rep(C, '  _ctl_wd_pg="$_wpid"\n', '  _ctl_wd_pg="$_wpid"; _k2_reap_add "$_r_for" "$_wpid" "$_cpid"\n')
rep(C, '  kill -9 -"$_wpid" 2>/dev/null || true; wait "$_wpid" 2>/dev/null || true\n',
       '  kill -9 -"$_wpid" 2>/dev/null || true; wait "$_wpid" 2>/dev/null || true; _k2_reap_del "$_r_for" "$_cpid" "$_wpid"\n')
K = T + ".controls.sh"
rep(K, 'if ! _VFY="$(mktemp -d "$SCRATCH/verifyXXXXXX")" || [ -z "$_VFY" ] || [ ! -d "$_VFY" ]; then\n',
       'if ! _VFY="$(mktemp -d "$SCRATCH/verifyXXXXXX")" || [ -z "$_VFY" ] || [ ! -d "$_VFY" ] || ! mkdir "$_VFY/.reap"; then\n')
rep(K, '  echo "!! could not create the verifier\'s directory (TMPDIR/disk?); decided nothing." >&2\n  exit 2\nfi\n',
       '  echo "!! could not create the verifier\'s directory (TMPDIR/disk?); decided nothing." >&2\n  exit 2\nfi\n_K2_REAP_D="$_VFY/.reap"\n')
OGL = "a call's process groups end with it and hold none of its caller's streams"
rep(K, '_ev_lbl="a previous run\'s channel evidence never answers for the next"\n',
       '_ev_lbl="a previous run\'s channel evidence never answers for the next"\n_og_lbl="' + OGL + '"\n')
# `_wm`: the hang stub traps TERM; the hang row asserts the TERM came first when TERM can reach a child of this run.
rep(K, "printf '%s\\n' 'sleep 5 & echo $! > \"$0.pid\"; wait' > \"$_VFY/.wm_hang\"\n",
       "printf '%s\\n' 'trap '\\'': > \"$0.term\"; exit 143'\\'' TERM; sleep 5 & echo $! > \"$0.pid\"; wait' > \"$_VFY/.wm_hang\"\n")
rep(K, 'command rm -f "$_VFY/.wm_end"\n',
       'command rm -f "$_VFY/.wm_end"\n' "{ sh -c 'kill -TERM $$; exit 0'; } 2>/dev/null && _wm_tg=0 || _wm_tg=1\n")
WM_TERM = ('               [ "$_wm_tg" -eq 0 ] || [ -e "$_VFY/.wm_hang.term" ] || _wm_bad="$_wm_bad; the hung run was sent no TERM before its KILL, so its own exit path could not run"\n')
rep(K, '               [ -e "$_VFY/.control_timeout" ] || _wm_bad="$_wm_bad; the watchdog wrote no marker"\n',
       '               [ -e "$_VFY/.control_timeout" ] || _wm_bad="$_wm_bad; the watchdog wrote no marker"\n' + WM_TERM)
rep(K, 'command rm -f "$_VFY/.wm_end" "$_VFY/.wm_abort" "$_VFY/.wm_hang" "$_VFY/.wm_hang.pid" "$_VFY/.wm_err" "$_VFY/.control_timeout"\n',
       'command rm -f "$_VFY/.wm_end" "$_VFY/.wm_abort" "$_VFY/.wm_hang" "$_VFY/.wm_hang.pid" "$_VFY/.wm_hang.term" "$_VFY/.wm_err" "$_VFY/.control_timeout"\n')
# The fourth standing check, after `_cap`'s cleanup line.
OG_KILL = ('               [ "$_og_dt" -lt 3 ] || _og_bad="$_og_bad; kill: a group of the call held its killed caller\'s stream for $_og_dt s"\n')
OG_NOREC = ('               case "$_og_st" in *"ended 2") ;; *) _og_bad="$_og_bad; norec: the call did not end 2 ($_og_st)" ;; esac\n'
            '               case "$_og_m" in *"could not record process group "*) _og_p="${_og_m##*process group }"; _og_p="${_og_p%%,*}" ;; *) _og_p="" ;; esac\n'
            '               case "$_og_p" in \'\'|*[!0-9]*) _og_bad="$_og_bad; norec: no line named the unrecorded group" ;;\n'
            '                 *) _og_gone "-$_og_p" || _og_bad="$_og_bad; norec: the unrecorded group $_og_p was left running" ;; esac ;;\n')
OG_NODEL = ('               case "$_og_st" in *"ended 2") ;; *) _og_bad="$_og_bad; nodel: the call did not end 2 ($_og_st)" ;; esac\n'
            '               case "$_og_m" in *"could not drop process group "*) ;; *) _og_bad="$_og_bad; nodel: no line named the undropped group" ;; esac ;;\n')
OG = ('# STANDING CHECK (R3 PROTO) of a call\'s process groups against its caller, three rows, ONE call line, over\n'
      '# `$_VFY` with the reap set at `$_VFY/.og_reap`: kill (the stub SIGKILLs the shell running the call, inside a\n'
      '# `$( … )`: the substitution must end at once, not when the watchdog\'s sleep ends; then the reap must stop what the\n'
      '# call left); norec (the reap set cannot record the child\'s group: the call ends 2, names the group on fd 9, the\n'
      '# group is gone); nodel (the stub turns its own entry into a directory: the drop fails, the call ends 2, named).\n'
      "printf '%s\\n' 'kill -KILL $PPID' > \"$_VFY/.og_kill\"\n"
      "printf '%s\\n' 'sleep 5 & wait' > \"$_VFY/.og_norec\"\n"
      "printf '%s\\n' 'e=\"$2/.og_reap/$$\"; while [ ! -f \"$e\" ]; do sleep 0.05; done; rm -f \"$e\"; mkdir \"$e\"' > \"$_VFY/.og_nodel\"\n"
      'command rm -rf "$_VFY/.og_end" "$_VFY/.og_reap"\n'
      'if [ "$_fw_trusted" -ne 1 ]; then ctl_ok=1\n'
      'elif ! ( _og_bad=""\n'
      '         _og_gone() { _og_i=0; while kill -0 "$1" 2>/dev/null; do\n'
      '             [ "$_og_i" -lt 40 ] || return 1; _og_i=$((_og_i + 1)); sleep 0.05; done; }\n'
      '         for _og_row in kill norec nodel; do\n'
      '           command rm -rf "$_VFY/.og_reap"; mkdir "$_VFY/.og_reap"; SELF="$_VFY/.og_$_og_row"; _og_t0=$SECONDS\n'
      '           _og_rd="$_VFY/.og_reap"; [ "$_og_row" != norec ] || _og_rd="$_VFY/.og_none"\n'
      '           _og_st="$( ( _ctl_wd_s=5; _K2_REAP_D="$_og_rd"; _ctl_run "$_og_lbl" "$_VFY" > /dev/null 2> "$_VFY/.og_err" 9> "$_VFY/.og_fd9"; echo "returned $?" ) 2>/dev/null; echo "ended $?" )"\n'
      '           _og_dt=$((SECONDS - _og_t0)); _og_m="$(cat "$_VFY/.og_fd9" 2>/dev/null)"\n'
      '           case "$_og_row" in\n'
      '             kill)\n' + OG_KILL +
      '               ( _K2_REAP_D="$_VFY/.og_reap"; _k2_reap )\n'
      '               for _og_f in "$_VFY/.og_reap"/*; do _og_p="${_og_f##*/}"; case "$_og_p" in \'\'|*[!0-9]*) continue ;; esac\n'
      '                 _og_gone "-$_og_p" || _og_bad="$_og_bad; kill: the reap left group $_og_p running"; done ;;\n'
      '             norec)\n' + OG_NOREC +
      '             nodel)\n' + OG_NODEL +
      '             *) _og_bad="$_og_bad; no arm for row \\"$_og_row\\"" ;;\n'
      '           esac\n'
      '         done\n'
      '         command rm -rf "$_VFY/.og_reap"\n'
      '         : > "$_VFY/.og_end"\n'
      '         [ -z "$_og_bad" ] || { echo "!! CONTROL FAILED ($_og_lbl)$_og_bad" >&2; exit 1; } ); then\n'
      '  [ -e "$_VFY/.og_end" ] || echo "!! CONTROL FAILED ($_og_lbl): the check ended before its verdict" >&2\n'
      '  ctl_ok=1\n'
      'fi\n'
      'command rm -rf "$_VFY/.og_end" "$_VFY/.og_kill" "$_VFY/.og_norec" "$_VFY/.og_nodel" "$_VFY/.og_err" "$_VFY/.og_fd9" "$_VFY/.og_reap" "$_VFY/.control_timeout"\n')
rep(K, 'command rm -f "$_VFY/.cap_end" "$_VFY/.cap_run" "$_VFY/.cap_err"\n',
       'command rm -f "$_VFY/.cap_end" "$_VFY/.cap_run" "$_VFY/.cap_err"\n' + OG)
SIGROWS = 'HUP:129 INT:130 TERM:143'
rep(K, '_control "$CTL/k2" 1 "K2: a" "a caller\'s GREP_OPTIONS cannot hide a file" "" "" "$CTL/goptgrep" "" "GREP_OPTIONS=--exclude=*" || ctl_ok=1\n',
       '_control "$CTL/k2" 1 "K2: a" "a caller\'s GREP_OPTIONS cannot hide a file" "" "" "$CTL/goptgrep" "" "GREP_OPTIONS=--exclude=*" || ctl_ok=1\n'
       'for _sg in ' + SIGROWS + '; do _sg_n="${_sg%:*}"; _sg_e="${_sg#*:}"; _sg_m="stopped by SIG$_sg_n"\n'
       '  { sh -c "kill -$_sg_n \\$\\$; exit 0"; } 2>/dev/null && { _sg_e=0; _sg_m=PASSED; }\n'
       '  _control "$CTL/clean" "$_sg_e" "$_sg_m" "a signal ends the run red and says so" "" "" "$CTL/sig$_sg_n" || ctl_ok=1\n'
       'done\n'
       '_control "$CTL/clean" 0 "could not remove the scratch dir" "a scratch dir that cannot be removed does not change the verdict" "" "" "$CTL/failrm" || ctl_ok=1\n')
F = T + ".fixtures.sh"
rep(F, '_fgit_shim "$CTL/goptgrep/grep" \': > "${K2_CTL_REACHED:?}/path"; [ -z "${GREP_OPTIONS+x}" ] || exit 1\' "$_REAL_GREP"\n',
       '_fgit_shim "$CTL/goptgrep/grep" \': > "${K2_CTL_REACHED:?}/path"; [ -z "${GREP_OPTIONS+x}" ] || exit 1\' "$_REAL_GREP"\n'
       'for _sg in HUP INT TERM; do mkdir -p "$CTL/sig$_sg"\n'
       '  _fgit_shim "$CTL/sig$_sg/git" \'case " $* " in *" rev-parse --verify "*) : > "${K2_CTL_REACHED:?}/path"; kill -\'"$_sg"\' 0 ;; esac\' "$_REAL_GIT"; done\n'
       'mkdir -p "$CTL/failrm"\n'
       '_fgit_shim "$CTL/failrm/rm" \'case "$1" in -rf) : > "${K2_CTL_REACHED:?}/path"; rm "$@"; exit 1 ;; esac\' rm\n'
       '_fgit_shim "$CTL/failrm/chmod" \'case "$1" in -R) chmod "$@"; exit 1 ;; esac\' chmod\n')
R = T + ".records.sh"
NEEDLE = "a signal ends the run red and says so"
N12 = "a scratch dir that cannot be removed does not change the verdict"
NWM = "only the watchdog's marker makes a signal status a timeout"
recs = ("s/^trap '_k2_sig HUP 129' HUP$/:/\t" + NEEDLE + "\n"                                   # 469 R11h
        "s/^trap '_k2_sig INT 130' INT$/:/\t" + NEEDLE + "\n"                                   # 470 R11i
        "s/^trap '_k2_sig TERM 143' TERM$/:/\t" + NEEDLE + "\n"                                 # 471 R11t
        "s/^    rm -rf \"[$]SCRATCH\" || echo \"!! could not remove the scratch dir [$]SCRATCH (the exit status is the run's own)\" >&9 ;; esac$/    rm -rf \"$SCRATCH\" ;; esac/\t" + N12 + "\n"  # 472 R12
        "s/^  trap '' HUP INT TERM; set +e$/  trap '' HUP INT TERM/\t" + N12 + "\n"             # 473 R12e
        "s/^_K2_STOP='for p; do kill -TERM -\"[$]p\"; done$/_K2_STOP=':/\t" + NWM + "\n"                             # 474 R11w
        "ctlrun:s/^  { ( exec env -u SHELLOPTS /  { ( trap : USR2; env -u SHELLOPTS /\t" + OGL + "\n"        # 475 R11d
        "s/^  _k2_stop \"[$]{@:2}\"; exit 2; }$/  return 0; }/\t" + OGL + "\n"                    # 476 R11a
        "s/; decided nothing\" >&9; exit 2; done; }$/; decided nothing\" >\\&9; done; }/\t" + OGL + "\n")  # 477 R11x
R11F = "s/, so nothing was decided\" >&9; exit \"[$]2\"; }$/, so nothing was decided\" >\\&2; exit \"$2\"; }/\t" + NEEDLE + "\n"
rep(R, "MUTANTS\n}\n", recs + "MUTANTS\n}\n")
M = T + ".mutations.sh"
rep(M, "    trap '_k2_rc=$?; _mut_rm_copies; _k2_exit \"$_k2_rc\"' EXIT\n",
       "    trap '_k2_rc=$?; trap \"\" HUP INT TERM; set +e; _mut_rm_copies; _k2_exit \"$_k2_rc\"' EXIT\n")
for v in sys.argv[2:]:
    if v == "noreap": rep(W, "  _k2_reap\n  case", "  : _k2_reap\n  case")
    elif v == "nofdclose":
        rep(C, ' 2>&1 9>&- & _cpid=$!', ' 2>&1 & _cpid=$!'); rep(C, ' 2>&1 9>&- & _wpid=$!', ' 2>&1 & _wpid=$!')
    elif v == "noexecwd": rep(C, '  { ( exec env -u SHELLOPTS ', '  { ( trap : USR2; env -u SHELLOPTS ')
    elif v == "killonly": rep(W, "_K2_STOP='" + STOP_TERM, "_K2_STOP=':\n")
    elif v == "fd2":
        rep(W, ', so nothing was decided" >&9;', ', so nothing was decided" >&2;')
        rep(W, ' before reaching its verdict" >&9\n', ' before reaching its verdict" >&2\n')
    elif v == "instr-sigshow":
        rep(K, '  { sh -c "kill -$_sg_n \\$\\$; exit 0"; } 2>/dev/null && { _sg_e=0; _sg_m=PASSED; }\n',
               '  { sh -c "kill -$_sg_n \\$\\$; exit 0"; } 2>/dev/null && { _sg_e=0; _sg_m=PASSED; }; echo "SIGROW $_sg_n expects $_sg_e" >&2  # INSTRUMENT\n')
    elif v.startswith("norow-"):
        sg = v[6:]; rows = " ".join(r for r in SIGROWS.split() if not r.startswith(sg + ":"))
        rep(K, "for _sg in " + SIGROWS + "; do", "for _sg in " + rows + "; do")
    elif v == "r11f": rep(R, "MUTANTS\n}\n", R11F + "MUTANTS\n}\n")
    elif v == "noprobe":
        rep(K, '  { sh -c "kill -$_sg_n \\$\\$; exit 0"; } 2>/dev/null && { _sg_e=0; _sg_m=PASSED; }\n', '')
    elif v == "nofailrm":
        rep(K, '_control "$CTL/clean" 0 "could not remove the scratch dir" "a scratch dir that cannot be removed does not change the verdict" "" "" "$CTL/failrm" || ctl_ok=1\n', '')
    elif v == "r12msg":
        rep(K, '_control "$CTL/clean" 0 "could not remove the scratch dir" "a scratch dir', '_control "$CTL/clean" 0 "PASSED" "a scratch dir')
    elif v == "nochmodshim":
        rep(F, '_fgit_shim "$CTL/failrm/chmod" \'case "$1" in -R) chmod "$@"; exit 1 ;; esac\' chmod\n', '')
    elif v == "wm-noterm": rep(K, WM_TERM, '')
    elif v == "og-nokill": rep(K, OG_KILL, '')
    elif v == "og-nonorec": rep(K, OG_NOREC, '               ;;\n')
    elif v == "og-nonodel": rep(K, OG_NODEL, '               ;;\n')
    else: raise SystemExit("unknown variant " + v)
```

#### §E.r3c2-fix1.py

`r3c2-fix1.py` (sha256 `827ea81458483789`, 30 lines)

```python
import sys
# r3c2-fix1.py <clone>: applied AFTER bin/r3c2-proto.py (and its variants) — the r3 text is the two scripts in sequence,
# "[r3f]". FROZEN after its measurements, as r3c2-proto.py is (that file is never overwritten; its own logs, logs/r3/,
# are the record of the defect fixed here). Three changes:
#  1. `$_K2_STOP` starts with `SECONDS=0`. Measured on r3c2-proto.py alone (logs/r3/r3-td-r3-wdhang.*): the watchdog's
#     stop counted its 2 s from the watchdog's own start, so after its sleep the poll ended before it began, and KILL
#     followed TERM at once (the hung child died 137 and left its scratch dir, as the frame's KILL does). The stop text
#     now bounds its own wait whoever runs it. R11w's anchor follows (the TERM line is now a line of its own).
#  2. The `_og` check's call line sends fd 9 into the substitution (`9>&1`, before the call's `> /dev/null`) instead of a file, so the kill row also sees
#     a group that keeps fd 9 (G11-3b), and the rows read the fd-9 line from the substitution's own output.
#  3. Records R11e (the watchdog's launch keeps fd 9: `9>&-` dropped) and R11g (the `_og` row loop emptied, GC-3's class).
d = sys.argv[1]; T = d + "/.claude/tools/webref-generic-core-trip-wire"
def rep(p, a, b, n=1):
    s = open(p).read(); assert s.count(a) == n, (p, s.count(a), a[:70]); open(p, "w").write(s.replace(a, b))
W = T + ".sh"; K = T + ".controls.sh"; R = T + ".records.sh"
rep(W, "_K2_STOP='", "_K2_STOP='SECONDS=0\n")
rep(K, '_ctl_run "$_og_lbl" "$_VFY" > /dev/null 2> "$_VFY/.og_err" 9> "$_VFY/.og_fd9"; echo "returned $?" ) 2>/dev/null; echo "ended $?" )"\n',
       '_ctl_run "$_og_lbl" "$_VFY" 9>&1 > /dev/null 2> "$_VFY/.og_err"; echo "returned $?" ) 2>/dev/null; echo "ended $?" )"\n')
rep(K, '_og_dt=$((SECONDS - _og_t0)); _og_m="$(cat "$_VFY/.og_fd9" 2>/dev/null)"\n',
       '_og_dt=$((SECONDS - _og_t0)); _og_m="$_og_st"\n')
rep(K, '"$_VFY/.og_err" "$_VFY/.og_fd9" "$_VFY/.og_reap"', '"$_VFY/.og_err" "$_VFY/.og_reap"')
OGL = "a call's process groups end with it and hold none of its caller's streams"
CEN = "every label the run used is named by a mutation record"
NWM = "only the watchdog's marker makes a signal status a timeout"
rep(R, "s/^_K2_STOP='for p; do kill -TERM -\"[$]p\"; done$/_K2_STOP=':/\t" + NWM + "\n",
       "s/^for p; do kill -TERM -\"[$]p\"; done$/:/\t" + NWM + "\n")
rep(R, "MUTANTS\n}\n",
    "ctlrun:s/^\\(      k2wd \"[$]{_ctl_wd_s-}\" \"[$]_r_wmark\" \"[$]_cpid\" ) < \\/dev\\/null > \\/dev\\/null 2>&1\\) 9>&- \\(& _wpid=[$]!; } 2>\\/dev\\/null\\)$/\\1 \\2/\t" + OGL + "\n"
    "controls:s/^         for _og_row in kill norec nodel; do$/         for _og_row in; do/\t" + CEN + "\n"
    "MUTANTS\n}\n")
```

#### §E.r3c2-fix2.py

`r3c2-fix2.py` (sha256 `3bb517c9c67accc6`, 19 lines)

```python
import sys
# r3c2-fix2.py <clone>: applied AFTER bin/r3c2-fix1.py — the final r3 text "[r3f]" is r3c2-proto.py, r3c2-fix1.py,
# r3c2-fix2.py in sequence. FROZEN after its measurements. One change: `_k2_stop`'s stop process, its KILL fallback and
# its `wait`s run inside `{ … } 2>/dev/null`, so the job notices bash 3.2 prints while it waits on the stop process
# ("Killed: 9", "Terminated: 15" for the run's own stopped jobs) go nowhere. Measured before this fix: f2probe3 case M on
# b32 printed "Killed: 9" for the TERM-ignoring group (logs/r3f-aborted/f2probe3.proto+fix1.txt, run on proto + fix1), and the
# top-level `killonly` site printed two (logs/r3/r3-s-top-killonly-TERM.b32.log); a synthetic test of the grouped form
# printed none on either shell. The `wait` after the stop alone (r2's form) does not reach a notice printed during the
# stop process's own foreground wait.
d = sys.argv[1]; T = d + "/.claude/tools/webref-generic-core-trip-wire"
def rep(p, a, b, n=1):
    s = open(p).read(); assert s.count(a) == n, (p, s.count(a), a[:70]); open(p, "w").write(s.replace(a, b))
W = T + ".sh"
rep(W, '  env -u SHELLOPTS -u BASH_ENV "$BASH" -c "$_K2_STOP" k2stop "$@" < /dev/null > /dev/null 2>&1 9>&- \\\n'
       '    || for _ks_p in "$@"; do kill -9 -"$_ks_p" 2>/dev/null; done\n'
       '  for _ks_p in "$@"; do wait "$_ks_p" 2>/dev/null; done; return 0; }\n',
       '  { env -u SHELLOPTS -u BASH_ENV "$BASH" -c "$_K2_STOP" k2stop "$@" < /dev/null > /dev/null 2>&1 9>&- \\\n'
       '      || for _ks_p in "$@"; do kill -9 -"$_ks_p"; done\n'
       '    for _ks_p in "$@"; do wait "$_ks_p"; done; } 2>/dev/null; return 0; }\n')
```

#### §E.r3-instr.py

`r3-instr.py` (sha256 `cb135107ff6b3e46`, 29 lines)

```python
import sys
# r3-instr.py <clone> <instrument ...>: measurement-only edits that apply to the frame, the r2 prototype and the r3
# prototype alike (they edit lines every one of them keeps):
#  instr-ev  : the `_ev` standing check's stub marks `<stub>.mark` and sleeps 3 s (a subshell landing point)
#  instr-cap : the same for `_cap`'s stub
#  instr-top : the `k2` control's `grep` shim (a top-level `_control`, its child a real selftest wire with a scratch dir)
#              marks `top_run.mark` in its evidence directory and sleeps 3 s, on its first call only
#  wdhang    : after the `k2` control, a control whose child (a real selftest wire) hangs in a PATH-first `git` shim
#              (`sleep 60` on the HEAD probe) under a 3 s watchdog: the watchdog's timeout kill, measured
d = sys.argv[1]; T = d + "/.claude/tools/webref-generic-core-trip-wire"
def rep(p, a, b, n=1):
    s = open(p).read(); assert s.count(a) == n, (p, s.count(a), a[:70]); open(p, "w").write(s.replace(a, b))
K = T + ".controls.sh"; F = T + ".fixtures.sh"
K2 = '_control "$CTL/k2" 1 "K2: a" "a caller\'s GREP_OPTIONS cannot hide a file" "" "" "$CTL/goptgrep" "" "GREP_OPTIONS=--exclude=*" || ctl_ok=1\n'
GOPT = '_fgit_shim "$CTL/goptgrep/grep" \': > "${K2_CTL_REACHED:?}/path"; [ -z "${GREP_OPTIONS+x}" ] || exit 1\' "$_REAL_GREP"\n'
for v in sys.argv[2:]:
    if v == "instr-ev":
        rep(K, "printf '%s\\n' 'k2evprobe 2>/dev/null; exit 0' > \"$_VFY/.ev_run\"\n",
               "printf '%s\\n' ': > \"$0.mark\"; sleep 3; k2evprobe 2>/dev/null; exit 0' > \"$_VFY/.ev_run\"  # INSTRUMENT\n")
    elif v == "instr-cap":
        rep(K, "printf '%s\\n' 'exit 0' > \"$_VFY/.cap_run\"\n",
               "printf '%s\\n' ': > \"$0.mark\"; sleep 3; exit 0' > \"$_VFY/.cap_run\"  # INSTRUMENT\n")
    elif v == "instr-top":
        rep(F, GOPT, GOPT.replace("': > \"${K2_CTL_REACHED:?}/path\"; ",
            "': > \"${K2_CTL_REACHED:?}/path\"; [ -e \"$K2_CTL_REACHED/top_run.mark\" ] || { : > \"$K2_CTL_REACHED/top_run.mark\"; sleep 3; }; ")[:-1] + "  # INSTRUMENT\n")
    elif v == "wdhang":
        rep(K, K2, K2 + '_ctl_wd_s=3; _control "$CTL/clean" 0 "PASSED" "wdhang probe" "" "" "$CTL/hangsh" || ctl_ok=1; _ctl_wd_s=30  # INSTRUMENT\n')
        rep(F, 'mkdir -p "$CTL/goptgrep"\n', 'mkdir -p "$CTL/goptgrep"\n' + 'mkdir -p "$CTL/hangsh"; _fgit_shim "$CTL/hangsh/git" \'case " $* " in *" rev-parse --verify "*) : > "${K2_CTL_REACHED:?}/path"; sleep 60 ;; esac\' "$_REAL_GIT"  # INSTRUMENT\n')
    else: raise SystemExit("unknown instrument " + v)
```

#### §E.r3-clone.sh

`r3-clone.sh` (sha256 `db2cfb6027912299`, 15 lines)

```bash
#!/bin/bash
# r3-clone.sh <name> <frame|r2|r3|r3f> [variant ...] [-- instrument ...]: a fresh `git clone --local` of cr2-wt at 706049ad
# under work/<name>; r2 = bin/r2c2-proto.py, r3 = bin/r3c2-proto.py (FROZEN) with the variants; r3f = r3 then bin/r3c2-fix1.py then bin/r3c2-fix2.py (the final r3 text); the instruments after
# `--` by bin/r3-instr.py (any text). Both shells' `-n` over every part. Prints the clone path last.
set -eu
H=/Users/kazuaki/repos/send.sh/elidex-fgit-handoff/cr2; n="$1"; m="$2"; shift 2
v=(); i=()
while [ $# -gt 0 ]; do [ "$1" = -- ] && { shift; i=("$@"); break; }; v+=("$1"); shift; done
d="$H/work/$n"; rm -rf "$d"
git clone -q --local "$H/../cr2-wt" "$d"; git -C "$d" checkout -q 706049ad
case "$m" in r2) python3 -I "$H/bin/r2c2-proto.py" "$d" ${v[@]+"${v[@]}"} ;; r3) python3 -I "$H/bin/r3c2-proto.py" "$d" ${v[@]+"${v[@]}"} ;;
  r3f) python3 -I "$H/bin/r3c2-proto.py" "$d" ${v[@]+"${v[@]}"} && python3 -I "$H/bin/r3c2-fix1.py" "$d" && python3 -I "$H/bin/r3c2-fix2.py" "$d" ;; frame) : ;; *) echo bad mode; exit 2 ;; esac
[ ${#i[@]} -eq 0 ] || python3 -I "$H/bin/r3-instr.py" "$d" "${i[@]}"
for f in "$d"/.claude/tools/webref-generic-core-trip-wire*.sh; do /opt/homebrew/bin/bash -n "$f"; /bin/bash -n "$f"; done
echo "$d"
```

#### §E.r3-batch.sh

`r3-batch.sh` (sha256 `6c75c2cd2f742976`, 55 lines)

```bash
#!/bin/bash
# r3-batch.sh: every (M) the memo's §6 cites for r3, on the FROZEN bin/r3c2-proto.py (digest 73643c0edab38a84) unless
# the item names the frame or r2 as its comparison arm; each item runs b53 ∥ b32 (two wire processes at a time).
# Logs: logs/r3/<item>.<shell>.log. Marker: logs/r3/R3-BATCH-DONE.
H=/Users/kazuaki/repos/send.sh/elidex-fgit-handoff/cr2; B=$H/bin; L=$H/logs/r3
cl() { "$B/r3-clone.sh" "$@" | tail -n 1; }
both() { local f="$1"; shift; for s in b53 b32; do "$f" "$s" "$@" & done; wait; echo "$(date +%T) done $*" >> "$L/R3-BATCH.progress"; }
ship()  { "$B/r3-runsig.sh" "$(cl "b-$2-$1" r3 "${@:4}")" "$1" "$L/$2.$1.log" "$3"; }          # ship <s> <item> <disp> [variants]
opt()   { "$B/r3-optin.sh" "$(cl "b-$2-$1" r3 "${@:5}")" "$1" "$L/$2.$1.log" "$3" "$4"; }      # opt <s> <item> <line> <disp> [variants]
site()  { local s=$1 item=$2 mode=$3 sig=$4 at=$5 how=$6 settle=$7; shift 7
          "$B/sigsite3.sh" "$(cl "b-$item-$s" "$mode" "$@")" "$s" "$L/$item.$s.log" "$sig" "$at" "$how" "$settle"; }
hang()  { "$B/f3hang.sh" "$(cl "b-$2-$1" r3 "${@:3}" -- instr-ev)" "$1" "$L/$2.$1.log"; }
tdrun() { local s=$1 item=$2 mode=$3; shift 3; "$B/r3-tdrun.sh" "$(cl "b-$item-$s" "$mode" "$@")" "$s" "$L/$item.$s.log"; }
trial() { "$B/r3-trialsig.sh" "$(cl "b-$2-$1" r3)" "$1" "$L/$2.$1.log" "$3" "$4"; }
: > "$L/R3-BATCH.progress"
# A: ship and dispositions
both ship r3-ship default instr-sigshow
both ship r3-shipignINT ignINT instr-sigshow
both ship r3-shipignHUP ignHUP instr-sigshow
both ship r3-shipignTERM ignTERM instr-sigshow
both ship r3-noprobe-ignINT ignINT noprobe instr-sigshow
both ship r3-noprobe-ignHUP ignHUP noprobe instr-sigshow
both ship r3-noprobe-ignTERM ignTERM noprobe instr-sigshow
both ship r3-noreapship default noreap
# B: records (opt-in, default dispositions) and their survive arms
for x in "469 R11h norow-HUP" "470 R11i norow-INT" "471 R11t norow-TERM" "472 R12 r12msg" "473 R12e nochmodshim" \
         "474 R11w wm-noterm" "475 R11d og-nokill" "476 R11a og-nonorec" "477 R11x og-nonodel"; do
  set -- $x; both opt "r3-opt$2" "$1" default; both opt "r3-surv$2" "$1" default "$3"
done
both opt r3-optR11f 478 default r11f
both opt r3-optR11i-ignINT 470 ignINT
both opt r3-optR11t-ignTERM 471 ignTERM
# C: signal sites (run-scoped TMPDIR kept; scratch dirs counted)
for sig in TERM INT; do for arm in reap noreap; do
  v=(); [ $arm = noreap ] && v=(noreap)
  both site "r3-s-ev-$arm-$sig" r3 $sig .ev_run.mark group 1 ${v[@]+"${v[@]}"} -- instr-ev
  both site "r3-s-cap-$arm-$sig" r3 $sig .cap_run.mark group 1 ${v[@]+"${v[@]}"} -- instr-cap
  both site "r3-s-top-$arm-$sig" r3 $sig top_run.mark group 1 ${v[@]+"${v[@]}"} -- instr-top
done; done
both site r3-s-top-frame-TERM frame TERM top_run.mark group 40 -- instr-top
both site r3-s-top-r2-TERM r2 TERM top_run.mark group 1 -- instr-top
both site r3-s-top-killonly-TERM r3 TERM top_run.mark group 1 killonly -- instr-top
both site r3-s-ev-pidonly-TERM r3 TERM .ev_run.mark pidonly 1 -- instr-ev
# D: the caller's stream after a SIGKILL of the run
both hang r3-f3hang-final
both hang r3-f3hang-keep9 nofdclose
both hang r3-f3hang-noexecwd noexecwd
# E: the watchdog's timeout kill, scratch dirs counted; the negative control (frame, no instrument, no signal)
both tdrun r3-td-frame-plain frame
both tdrun r3-td-frame-wdhang frame -- wdhang
both tdrun r3-td-r3-wdhang r3 -- wdhang
both tdrun r3-td-killonly-wdhang r3 killonly -- wdhang
# F: a signal during an opt-in trial
both trial r3-trial-TERM 472 TERM
echo R3-BATCH-DONE >> "$L/R3-BATCH.progress"; echo R3-BATCH-DONE > "$L/R3-BATCH-DONE"
```

#### §E.r3-batch2.sh

`r3-batch2.sh` (sha256 `6a7b2ece3db2a5f4`, 13 lines)

```bash
#!/bin/bash
# r3-batch2.sh: follow-ups to r3-batch.sh, started after its marker (two wire processes at a time, never beside it).
# The probe-removed ship runs (r3-batch.sh's ran with a conflicting instrument variant and never started: x-invalid-*).
H=/Users/kazuaki/repos/send.sh/elidex-fgit-handoff/cr2; B=$H/bin; L=$H/logs/r3
while [ ! -e "$L/R3-BATCH-DONE" ]; do sleep 20; done
cl() { "$B/r3-clone.sh" "$@" | tail -n 1; }
both() { local f="$1"; shift; for s in b53 b32; do "$f" "$s" "$@" & done; wait; echo "$(date +%T) done $*" >> "$L/R3-BATCH2.progress"; }
ship()  { "$B/r3-runsig.sh" "$(cl "b-$2-$1" r3 "${@:4}")" "$1" "$L/$2.$1.log" "$3"; }
: > "$L/R3-BATCH2.progress"
both ship r3-noprobe-ignINT ignINT noprobe
both ship r3-noprobe-ignHUP ignHUP noprobe
both ship r3-noprobe-ignTERM ignTERM noprobe
echo R3-BATCH2-DONE > "$L/R3-BATCH2-DONE"
```

#### §E.r3f-batch.sh

`r3f-batch.sh` (sha256 `e1575ce34c712fa4`, 57 lines)

```bash
#!/bin/bash
# r3f-batch.sh: every (M) the memo's §6 cites for r3, on the FROZEN final text [r3f] = bin/r3c2-proto.py (73643c0edab38a84)
# then bin/r3c2-fix1.py (frozen; digest in R3-PROGRESS.md), unless
# the item names the frame or r2 as its comparison arm; each item runs b53 ∥ b32 (two wire processes at a time).
# Logs: logs/r3/<item>.<shell>.log. Marker: logs/r3/R3F-BATCH-DONE.
H=/Users/kazuaki/repos/send.sh/elidex-fgit-handoff/cr2; B=$H/bin; L=$H/logs/r3f
cl() { "$B/r3-clone.sh" "$@" | tail -n 1; }
both() { local f="$1"; shift; for s in b53 b32; do "$f" "$s" "$@" & done; wait; echo "$(date +%T) done $*" >> "$L/R3F-BATCH.progress"; }
ship()  { "$B/r3-runsig.sh" "$(cl "f-$2-$1" r3f "${@:4}")" "$1" "$L/$2.$1.log" "$3"; }          # ship <s> <item> <disp> [variants]
opt()   { "$B/r3-optin.sh" "$(cl "f-$2-$1" r3f "${@:5}")" "$1" "$L/$2.$1.log" "$3" "$4"; }      # opt <s> <item> <line> <disp> [variants]
site()  { local s=$1 item=$2 mode=$3 sig=$4 at=$5 how=$6 settle=$7; shift 7
          "$B/sigsite3.sh" "$(cl "f-$item-$s" "$mode" "$@")" "$s" "$L/$item.$s.log" "$sig" "$at" "$how" "$settle"; }
hang()  { "$B/f3hang.sh" "$(cl "f-$2-$1" r3f "${@:3}" -- instr-ev)" "$1" "$L/$2.$1.log"; }
tdrun() { local s=$1 item=$2 mode=$3; shift 3; "$B/r3-tdrun.sh" "$(cl "f-$item-$s" "$mode" "$@")" "$s" "$L/$item.$s.log"; }
trial() { "$B/r3-trialsig.sh" "$(cl "f-$2-$1" r3f)" "$1" "$L/$2.$1.log" "$3" "$4"; }
: > "$L/R3F-BATCH.progress"
# A: ship and dispositions
both ship r3-ship default instr-sigshow
both ship r3-shipignINT ignINT instr-sigshow
both ship r3-shipignHUP ignHUP instr-sigshow
both ship r3-shipignTERM ignTERM instr-sigshow
both ship r3-noprobe-ignINT ignINT noprobe
both ship r3-noprobe-ignHUP ignHUP noprobe
both ship r3-noprobe-ignTERM ignTERM noprobe
both ship r3-noreapship default noreap
# B: records (opt-in, default dispositions) and their survive arms
for x in "469 R11h norow-HUP" "470 R11i norow-INT" "471 R11t norow-TERM" "472 R12 r12msg" "473 R12e nochmodshim" \
         "474 R11w wm-noterm" "475 R11d og-nokill" "476 R11a og-nonorec" "477 R11x og-nonodel" "478 R11e og-nokill"; do
  set -- $x; both opt "r3-opt$2" "$1" default; both opt "r3-surv$2" "$1" default "$3"
done
both opt r3-optR11f 478 default r11f
both opt r3-optR11g 479 default
both opt r3-optR11i-ignINT 470 ignINT
both opt r3-optR11t-ignTERM 471 ignTERM
# C: signal sites (run-scoped TMPDIR kept; scratch dirs counted)
for sig in TERM INT; do for arm in reap noreap; do
  v=(); [ $arm = noreap ] && v=(noreap)
  both site "r3-s-ev-$arm-$sig" r3f $sig .ev_run.mark group 1 ${v[@]+"${v[@]}"} -- instr-ev
  both site "r3-s-cap-$arm-$sig" r3f $sig .cap_run.mark group 1 ${v[@]+"${v[@]}"} -- instr-cap
  both site "r3-s-top-$arm-$sig" r3f $sig top_run.mark group 1 ${v[@]+"${v[@]}"} -- instr-top
done; done
both site r3-s-top-frame-TERM frame TERM top_run.mark group 40 -- instr-top
both site r3-s-top-r2-TERM r2 TERM top_run.mark group 1 -- instr-top
both site r3-s-top-killonly-TERM r3f TERM top_run.mark group 1 killonly -- instr-top
both site r3-s-ev-pidonly-TERM r3f TERM .ev_run.mark pidonly 1 -- instr-ev
# D: the caller's stream after a SIGKILL of the run
both hang r3-f3hang-final
both hang r3-f3hang-keep9 nofdclose
both hang r3-f3hang-noexecwd noexecwd
# E: the watchdog's timeout kill, scratch dirs counted; the negative control (frame, no instrument, no signal)
both tdrun r3-td-frame-plain frame
both tdrun r3-td-frame-wdhang frame -- wdhang
both tdrun r3-td-r3-wdhang r3f -- wdhang
both tdrun r3-td-killonly-wdhang r3f killonly -- wdhang
# F: a signal during an opt-in trial
both trial r3-trial-TERM 472 TERM
echo R3F-BATCH-DONE >> "$L/R3F-BATCH.progress"; echo R3F-BATCH-DONE > "$L/R3F-BATCH-DONE"
```
