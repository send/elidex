# K2 wire CR#1 — step 1 at the new T: the delta to the final-head runbook (companion of the CR#1 plan)

## §0 Spec coverage map

**No spec surface**: a runbook of shell-harness runs, as `…-cr2-plan-finalhead-runbook.md` is; `preflight.py` is
not run on it. The companion of `docs/plans/2026-10-citation-hygiene-k2-wire-cr1-scratch-plan.md` ("the plan"; §N
below is the plan's unless named). "The runbook" is
`docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-cr2-plan-finalhead-runbook.md`, accepted, bound to T =
`01fb2924` and the frame `3b7ff6d5`, and **not edited** (finalhead §1.3, §9 item 2). This file is the new-T
instrument frame finalhead §1.3 requires ("new anchors; the run/retire rule re-applied cell by cell; sources that
may be gone"): it runs the runbook's fences as written except the ones it replaces or adds below, each named. **Line
numbers**: a doc or tool line is read at `67488c8d` unless it says otherwise (for the wire, its parts and every doc
cited here, `67488c8d` equals `fdded560` and `cdc7c6ec`: plan, Frame); a number marked "(r4)" is the plan's prototype tree
(`scratchpad/nw1-helper-prototype-r4.patch`, made on `fdded560`, applying to `67488c8d`) and is re-checked at T by §3.

---

## §1 The frame

- **T** is the CR#1 fix commit: the last commit on `webref-cite-audit-tool` that changes `.claude/tools/**` or
  `scripts/**` or the trip-wires job (landing :339–:341), re-derived by landing :392–:397's command (its fence
  :391–:398) in `W`; its parent is `67488c8d` (the branch's head, pushed: its merge of `origin/main` at `b9122157`, on
  top of `fdded560`, the merge of `2e7c432c`, and `cdc7c6ec`, the merge of A-iii #545). The guard (§2) refuses any other.
- **The sequence** (plan §6): `origin/main` is merged **before** the fix commit (those merges are `fdded560` and `67488c8d`;
  until the fix commit is pushed, `origin`'s branch is `67488c8d`); the fix commit is pushed (user approval) **before** step 1, because the guard needs `origin`'s `webref-cite-audit-tool` to
  equal `W`'s HEAD; X9's CI run is that same push's; the verdict commit follows step 1 and X9; then Codex. No merge of
  `origin/main` happens between the fix commit and the end of step 1, X9 and Codex: a later merge that touches
  `.claude/tools/**` or `scripts/**` moves T, and step 1 runs again at it.
- **W** = `/Users/kazuaki/repos/send.sh/elidex-wt-citeaudit`, this branch's worktree. `k2-wire-fgit-hermetic` is
  merged (#527's squash `7d0da266`) and no longer on `origin`, so the runbook's `W` and its `origin` check have no
  subject. Every clone is `git clone --local "$W"` detached at T, unless a commit is named.
- **DOC** reads the docs **at T** (they hold this slice's restatements). An extraction of an instrument is
  byte-compared with the same block at `3b7ff6d5`, the frame the instruments were reviewed at: `-pr527.md`,
  `-pr527-r15.md` and `-provenance.md` are byte-identical at `67488c8d` (`git diff --quiet 3b7ff6d5 67488c8d --
  <file>`, measured 2026-10-26), so their line ranges stand; `-landing.md` gained 42–46 lines above the three blocks
  the runbook extracts (landing's wdpatch :436–:441 → :478–:483, X5's :494–:526 → :540–:572, crcpatch :536–:551 →
  :582–:597 at `67488c8d`, each block byte-identical), so they are found by their first line (`LDOC`, §2) and compared, not
  read by number.
- **K** = `${K:-$H/cr2/work/fh2}`. The dry pass (§3) sets `K` to its own directory before sourcing the definitions.
  Disk budget as the runbook's (≈ 7 GB) plus the two N clones (§2), `df -h /` by `logenv`.
- **Scripts** live in a NEW directory, `$H/cr2/work/fh2-scripts/` (the `01fb2924` run's `fh-scripts/` is not
  overwritten), outside every `K`, and the definitions file is there too, so a fresh `K` keeps both. **Order**: the
  definitions file's writer (§2) runs first, once, as its own script `fh2-scripts/defs-writer.sh`; every other script
  (`dry.sh`, the setup script, BN, B1–B8) starts with the sourcing line, so none reads a definition before it exists.

## §2 Replaced fences of runbook §6.0

**The definitions file** (runbook §6.0's writer, as `defs-writer.sh`): written to `$H/cr2/work/fh2-scripts/defs.sh`;
the **sourcing line** becomes `. /Users/kazuaki/repos/send.sh/elidex-fgit-handoff/cr2/work/fh2-scripts/defs.sh ||
exit 1` in every script. The runbook's text (its `H=` line comes before `K=`), except these lines:

```sh
W=/Users/kazuaki/repos/send.sh/elidex-wt-citeaudit
K=${K:-$H/cr2/work/fh2}                  # every clone, insert and log of the new-T step 1 (the dry pass sets its own)
SP=$K/x5                                 # follows K
DOC() { git -C "$W" show "${2:-$(cat "$K/T")}:$DP$1"; }     # docs at T (the guard writes $K/T)
LDOC() { local n; n=$(DOC -landing.md | /usr/bin/grep -nxF -- "$1" | cut -d: -f1)   # LDOC <first line> <count> <start at 3b7ff6d5>
  [ -n "$n" ] && [ "$(printf '%s\n' "$n" | wc -l)" -eq 1 ] || { echo "LANDING ANCHOR NOT ONCE: $1" >&2; return 1; }
  DOC -landing.md | sed -n "$n,$((n + $2 - 1))p" > "$K/.ldoc" &&
  DOC -landing.md 3b7ff6d5 | sed -n "$3,$(($3 + $2 - 1))p" | cmp -s - "$K/.ldoc" ||
  { echo "LANDING BLOCK DIFFERS FROM 3b7ff6d5: $1" >&2; return 1; }; cat "$K/.ldoc"; }
mkclone() { mkdir -p "$(dirname "$K/$1")" && git clone -q --local "$W" "$K/$1" &&
  git -C "$K/$1" checkout -q --detach "${2:-$(cat "$K/T")}" &&
  [ -z "$(git -C "$K/$1" status --porcelain)" ] || { echo "CLONE FAILED $1"; return 1; }; }
```

`LDOC`'s two failure lines go to stderr, so a failed extraction leaves the extracted file empty and `xt` stops the
script on it.

**The frame guard** (replaces the fence at runbook :88–:102; any failure stops for the user, finalhead §1.3):

```sh
cd "$W" || exit 1
[ "$(git rev-parse --verify -q '3b7ff6d5^{commit}')" = 3b7ff6d5816a7905100333ccc5b21c2f75b8b7c2 ] || { echo "FRAME MISSING"; exit 1; }
_job() { git show "$1:.github/workflows/ci.yml" | sed -n '/^  trip-wires:/,/^  [a-z]/p' | /usr/bin/grep -v '^ *#'; }   # landing :392–:397
T=$(for c in $(git log --format=%H e8f78896..HEAD); do
  [ -n "$(git diff --name-only "$c^" "$c" -- .claude/tools scripts)" ] && { echo "$c"; break; }
  [ "$(_job "$c^")" = "$(_job "$c")" ] || { echo "$c"; break; }
done)
[ -n "$T" ] && [ -n "$(git diff --name-only "$T^" "$T" -- .claude/tools/webref-generic-core-trip-wire.sh)" ] ||
  { echo "T MOVED: '$T' is not a change to the wire, so not CR#1's fix (the user decides)"; exit 1; }
[ "$(git rev-parse "$T^")" = 67488c8d404c355e8f650dbd807cfb7f923f0f9c ] ||
  { echo "T'S PARENT IS NOT 67488c8d (main merged after the fix, or another commit under it: the user decides)"; exit 1; }
[ "$(git show "$T:.claude/tools/webref-generic-core-trip-wire.sh" | /usr/bin/grep -cx '_k2_own_dir() {')" -eq 1 ] ||
  { echo "T HOLDS NO OWN-DIR HELPER (not CR#1's fix)"; exit 1; }
[ "$(git diff --name-only "$T" HEAD -- .claude/tools scripts | wc -l)" -eq 0 ] || { echo "TOOL CODE IS NOT T'S"; exit 1; }
[ -z "$(git status --porcelain -- .claude/tools scripts docs/plans)" ] || { echo "W HAS LOCAL CHANGES"; exit 1; }
[ "$(git ls-remote origin refs/heads/webref-cite-audit-tool | cut -f1)" = "$(git rev-parse HEAD)" ] ||
  { echo "ORIGIN'S webref-cite-audit-tool IS NOT W'S HEAD (unpushed, or moved by another session: the user decides)"; exit 1; }
git diff --quiet 6afa7d98 67488c8d -- '.claude/tools/webref-generic-core-trip-wire*' ||
  { echo "THE BASE WIRE IS NOT THE ONE THE PLAN'S §1 MEASURED"; exit 1; }
mkdir -p "$K/logs" "$K/ins" && printf '%s\n' "$T" > "$K/T" && cd "$K" || exit 1
echo "frame guard: passed, T=$T"
```

**The clones** (replaces the fence at runbook :117–:121): `c0 x3main53 x3main32 x3plant53 x3plant32 x3floor53
x3floor32 x11 u a stale x8t x5/p11 n1` by `mkclone` (T), `mkclone x8base e8f78896`, and `mkclone nbase 67488c8d`:
the N cells' base tree. `nbase` is `67488c8d`, not `cdc7c6ec`: it is T's parent (the guard checks it), so the base
and T rows compare exactly the fix commit's change; its wire and parts are byte-identical to `cdc7c6ec`'s and
`6afa7d98`'s, where the plan's §1 was measured (the guard's last check). A stopped run's clean-up list (runbook §6.0,
"A re-run after a stop") adds `n1` and `nbase`; BN's victim directories hold that cell's `log` and stay.

**The scripts** (replaces the fence at runbook :132–:140): the two landing reads go through `LDOC`; the rest as
written.

```sh
cd "$K" || exit 1
DOC -pr527.md     | sed -n 627,646p > cell15.sh   && xt cell15.sh   '#!/bin/bash' || exit 1
DOC -pr527-r15.md | sed -n 394,414p > cellw.sh    && xt cellw.sh    '#!/bin/bash' || exit 1
DOC -pr527.md     | sed -n 333,396p > crc.sh      && xt crc.sh      '#!/bin/bash' || exit 1
for f in cell15.sh cellw.sh crc.sh; do r=$(case $f in cell15.sh) echo pr527.md 627,646;; cellw.sh) echo pr527-r15.md 394,414;; crc.sh) echo pr527.md 333,396;; esac)
  DOC "-${r% *}" 3b7ff6d5 | sed -n "${r#* }p" | cmp -s - "$f" || { echo "EXTRACTION DIFFERS FROM 3b7ff6d5: $f"; exit 1; }; done
LDOC "python3 - cell15.sh cellw.sh <<'P'" 6 436 > wdpatch.sh && xt wdpatch.sh "python3 - cell15.sh cellw.sh <<'P'" && /bin/bash wdpatch.sh || exit 1
LDOC "python3 - crc.sh <<'P'" 16 536 > crcpatch.sh && xt crcpatch.sh "python3 - crc.sh <<'P'" && /bin/bash crcpatch.sh || exit 1
python3 - crc.sh <<'P' || exit 1        # plan §6: sealdotdot re-derived for the /run level (two more `..`)
import sys; p = sys.argv[1]; s = open(p).read(); a = '\'sealdotdot\':\'_seal "$CTL/walk/../../../k2-sentinel" 000 walk'
assert s.count(a) == 1, a; open(p, 'w').write(s.replace(a, a.replace('../../../', '../../../../../')))
P
for f in cell15.sh cellw.sh crc.sh; do $SH32 -n $f && $SH53 -n $f || { echo "BAD $f"; exit 1; }; done
```

The `sealdotdot` patch is where that cell's restated definition lives (plan §6); `…-pr527.md` (closed at round 14)
and the landing memo are not edited for it.

## §3 The dry pass and the anchor re-check at T (finalhead §7 C5), before BN

After the definitions writer (§1, Order), a dry pass in its own `K`, started with the paths written out (the
invoking shell has no `$H`; the definitions define it):

```sh
K=/Users/kazuaki/repos/send.sh/elidex-fgit-handoff/cr2/work/fh2-dry /opt/homebrew/bin/bash /Users/kazuaki/repos/send.sh/elidex-fgit-handoff/cr2/work/fh2-scripts/dry.sh
```

The definitions take `K` from the environment, and `SP` follows it. `dry.sh` is the sourcing line, then: the guard;
the clones `c0`, `u`, `a`, `stale`, `x5/p11`, `x3plant53` and `x3floor53`; the scripts fence above; §6.3's insert
writers and tree edits (B2, with §4's `xrace`); §6.7's plant and floor patches, each on its one dry clone (the
anchor is the same in the other shell's clone); §6.5's instruments (`c11`); and §6.6's lines up to and including
its 32-job count, `[ "$(wc -l < "$SP/c11/jobs.run")" -eq 32 ] || exit 1`, with `x5prep.sh` through `LDOC` (§4).
**Stop point**: the line before §6.6's `logenv B4`; the dry pass runs no wire and no cell. Every `assert`, `xt` and
count on the way must hold. Then, in the dry `K`:

1. **The anchors**: the counts below, each `/usr/bin/grep -cF` (or `-cx` for a whole line) in `c0`, the clone at T.
   The X5 and X13 anchors are also asserted by their own scripts at run time.
2. The dry `K` is then deleted by its path, written out: `chmod -R u+rwX
   /Users/kazuaki/repos/send.sh/elidex-fgit-handoff/cr2/work/fh2-dry && rm -rf
   /Users/kazuaki/repos/send.sh/elidex-fgit-handoff/cr2/work/fh2-dry` (no glob).

r3's census-token grep over the generated inserts (and its loop over X5's job copies) is gone with the census
(PM decision 27; plan §4): no check reads those tokens any more.

| cell | anchor at T (file) | count |
|---|---|---|
| X6, X15, X16, `xrace`, X13 garbagehead, X3 plant | `: > "$_FW_DIR/built"` (fixtures, :705 at `67488c8d` and (r4), its last line) | 1 |
| X3 floor | `_mut_regex_mutants "$_gr_v" "$_gr_val" >> "$_VFY/.genmutants" \|\|` (mutgen :304, unedited) | 1 |
| X13 w3ar | `mkdir -p "$CTL/walk/sub"` (fixtures) | 1 |
| X13 w2rec | `_fgit_window_verdict_exit() {` (harness), its body one `\n    exit 2` and one `\n  exit 1` (crcpatch's assert) | 1 |
| X13 lblrename | `_pa_lbl="` (controls) | 1 |
| X13 sealfail | `_seal "$CTL/err/control.py" 000 err` (fixtures) | 1 |
| X13 NOU | `set -euo pipefail` as a whole line (wire) | 1 |
| X11 | `_af_lbl="a record's anchor is fresh …"` (controls), the last `_lbl="` definition | 1 |
| X16 `xbare` | `_MUT_UNRECORDED_MAX=18` (mutations) | 1 |
| X16 `xanchor` | the record `fixtures:s/^: > "[$]_FW_DIR\/built"$/git -C "$CTL\/clean" config --add core.bare false; &/` (records, by text: :196 at `67488c8d`, :207 (r4)) | 1 |
| X16 `stale*` | `MUTANTS` as a whole line (records; :595 at `67488c8d`, :606 (r4)) | 1 |
| `staleshim` | `_k2_own_dir _VFY "$SCRATCH" verify` (controls) and `"$_od_d/${3}XXXXXX"` (wire): the shim's `*verifyXXXXXX*` still matches | 1, 1 |
| `xrace` | `_k2_own_dir _pq "${_FW_DIR%/*}" pgref` (harness) and `printf -v "$1" %s "$_K2_OD_TOP/run"` (wire): the re-derived glob's level | 1, 1 |
| X5 | `gen11.py` with landing's re-anchor prints `38 jobs`; every copy clean under `bash -n` on 3.2 and 5.3 | 38 |
| X3 main | `_MUT_RECORDS_MIN=316` (mutations) and `_mutants \| wc -l` | 316, 316 |

## §4 Replaced and added fences of the batches

- **BN** (new; after the dry pass, before B1): N2, N1, N5 and N6, the plan's own cells (plan §6 (a)), one script
  whose fences follow, each run under the runner form of the runbook (`setpgrp`, an alarm, a `kill -9` of the
  group 3 s after it, `sweep` after the `wait`), two runs at a time (the two shells). The alarm is 600 s: a plain run
  takes 35–45 s here.

```sh
logenv BN; mkdir -p "$K/BN"
run1() {  # run1 <log> <cwd> <shell path> <tree> [env assignments…]: one wire run under the runner form
  local L=$1 cwd=$2 sh=$3 tr=$4; shift 4
  ( cd "$cwd" && exec env "$@" perl -e 'setpgrp; alarm 600; exec @ARGV' "$sh" "$K/$tr/.claude/tools/webref-generic-core-trip-wire.sh" < /dev/null ) > "$L" 2>&1 & local g=$!
  ( exec perl -e 'setpgrp; sleep $ARGV[0]; kill "KILL", -$ARGV[1]' 603 "$g" ) & local wd=$!
  _rc=0; wait "$g" || _rc=$?; kill -9 -"$wd" 2>/dev/null; sweep "$g" >> "$K/logs/BN.txt"; }
ls1() { [ -d "$1" ] && ls -A "$1" | tr '\n' ' ' || echo GONE; }
# N2 (no run): every part parses on both shells; the lengths
for f in "$K"/c0/.claude/tools/webref-generic-core-trip-wire*.sh; do $SH32 -n "$f" && $SH53 -n "$f" || echo "N2 BAD $f"; done >> "$K/logs/BN.txt" 2>&1
wc -l "$K"/c0/.claude/tools/webref-generic-core-trip-wire*.sh >> "$K/logs/BN.txt"
# N1: the plain run at T from the clone's root, and from an empty directory outside it
n1() { run1 "$K/BN/n1-$1.log" "$2" "$3" n1
  echo "N1 $1 rc=$_rc last=[$(tail -n 1 "$K/BN/n1-$1.log")] tree=[$(git -C "$K/n1" status --porcelain | wc -l | tr -d ' ')] cwd=[$(ls1 "$2")]" >> "$K/logs/BN.txt"; }
mkdir -p "$K/BN/out53" "$K/BN/out32"
n1 root53 "$K/n1" $SH53 & n1 root32 "$K/n1" $SH32 & wait
n1 out53 "$K/BN/out53" $SH53 & n1 out32 "$K/BN/out32" $SH32 & wait
# N5: the root site; a shim answers EVERY mktemp call with one value. In a fresh $D: the victim cwd and `old` (the
# outside directory) and `cdp/rel` each hold `sentinel`; `-` runs with OLDPWD=$D/old exported, `rel` with
# CDPATH=$D/cdp exported. $D is expanded when the shim is generated.
n5() {  # n5 <tree: nbase | c0> <tag: 53 | 32> <shell path> <answer>
  local D=$K/BN/n5/$1.$2.$4 a e=(); mkdir -p "$D/victim" "$D/old" "$D/cdp/rel" "$D/shim" &&
  touch "$D/victim/sentinel" "$D/old/sentinel" "$D/cdp/rel/sentinel" || return 1
  case $4 in empty) a='exit 0';; dot) a='printf .; exit 0';; outside) a="printf %s '$D/old'; exit 0";;
    fail) a='exit 1';; dash) a='printf %s -; exit 0'; e=("OLDPWD=$D/old");; rel) a='printf %s rel; exit 0'; e=("CDPATH=$D/cdp");; esac
  printf '#!/bin/sh\n%s\n' "$a" > "$D/shim/mktemp" && chmod +x "$D/shim/mktemp" || return 1
  run1 "$D/log" "$D/victim" "$3" "$1" ${e[@]+"${e[@]}"} PATH="$D/shim:$PATH"
  echo "N5 $1 $2 $4 rc=$_rc victim=[$(ls1 "$D/victim")] old=[$(ls1 "$D/old")] cdp/rel=[$(ls1 "$D/cdp/rel")] first=[$(/usr/bin/grep -m1 '^!!' "$D/log" | cut -c1-200)]" >> "$K/logs/BN.txt"; }
for t in nbase c0; do for a in empty dot dash rel outside fail; do n5 $t 53 $SH53 $a & n5 $t 32 $SH32 $a & wait; done; done
# N6: one inner site; the shim answers badly only for that site's template and execs the real mktemp otherwise.
n6() {  # n6 <tree> <tag> <shell path> <site: verify | ctl | pgref> <shape: empty | dot | out>
  local D=$K/BN/n6/$1.$2.$4.$5 act nd why; mkdir -p "$D/victim" "$D/out" "$D/shim" &&
  echo keep > "$D/victim/sentinel" && echo keep > "$D/out/sentinel" || return 1
  case $5 in empty) act='exit 0';; dot) act='printf .; exit 0';; out) act="printf %s '$D/out'; exit 0";; esac
  printf '#!/bin/sh\ncase "$*" in *"/%sXXXXXX"*) %s;; esac\nexec /usr/bin/mktemp "$@"\n' "$4" "$act" > "$D/shim/mktemp" && chmod +x "$D/shim/mktemp" || return 1
  run1 "$D/log" "$D/victim" "$3" "$1" PATH="$D/shim:$PATH"
  case $4 in verify) nd="could not create the verifier's directory";; ctl) nd="could not create a scratch dir for the controls";;
    pgref) nd="the postconditions' directory could not be made fresh";; esac
  case $5 in empty) why="printed no path";; *) why="not a new name inside";; esac
  echo "N6 $1 $2 $4/$5 rc=$_rc site=$(/usr/bin/grep -cF "$nd" "$D/log") why=$(/usr/bin/grep -cF "$why" "$D/log") victim=[$(ls1 "$D/victim")] out=[$(ls1 "$D/out")]" >> "$K/logs/BN.txt"; }
for t in nbase c0; do for st in verify ctl pgref; do for sh in empty dot out; do
  n6 $t 53 $SH53 $st $sh & n6 $t 32 $SH32 $st $sh & wait; done; done; done
stopgate BN || exit 1
```

  **Verdicts**, by grep over `$K/logs/BN.txt` (one line per run), against plan §6 (a)'s expected column:
  - N2: no `N2 BAD` line; the `wc -l` lines as the plan's N2 row gives them.
  - N1, four lines: each `rc=0 last=[webref generic-core layering trip-wire PASSED] tree=[0]`, and `cwd=[]` on the
    two `out` lines.
  - N5, 24 lines. `c0` per plan §4's victim rows: every line `rc=2`, `victim=[sentinel ]`, `old=[sentinel ]`,
    `cdp/rel=[sentinel ]`, and `first=` holding the row's reason (`printed no path`; `not a new name inside` for
    `dot`, `dash`, `rel`, `outside`; `no directory from mktemp` for `fail`). `nbase`, per row and shell, the values
    measured 2026-10-26 (`scratchpad/cr1r4/log/batchX.{b53,b32}.out`, the `base` rows; the same script as BN's `n5`,
    `first=` the first `^!!` line), each line `N5 nbase <53|32> <answer> rc=<rc> victim=[…] old=[…] cdp/rel=[…]
    first=[…]`; `S` = `sentinel ` (trailing space), the others are what the table says is removed (`GONE`):
    - `empty`, 5.3: `rc=2`, all three `S`, `first=[!! could not resolve the scratch dir () to a physical path, so
      nothing here was proved`; 3.2: `rc=2`, `victim=[GONE]`, `old` and `cdp/rel` `S`, `first=[!! could not create a
      scratch dir for the controls (TMPDIR/disk?),` (the victim cwd is removed).
    - `dot`, both shells: `rc=1`, `victim=[GONE]`, `old` and `cdp/rel` `S`, `first=[!! CONTROL NOT EXERCISED (this git
      reports alternate object stores)` (the victim cwd is removed).
    - `outside`, both shells: `rc=2`, `victim=[sentinel ]`, `old=[GONE]`, `cdp/rel=[sentinel ]`, `first=[!! CONTROL
      FAILED (green is reachable): exit 0 as expected, but for the wrong reason —` (the outside directory is removed).
    - `fail`, both shells: `rc=2`, all three `S`, `first=[!! no scratch dir (TMPDIR/disk?), so nothing here was
      proved` (nothing removed).
    - `dash`, 5.3: `rc=2`, all three `S`, `first=[]` (no `!!` line: the run fails at harness :54, plan §1); 3.2:
      `rc=2`, all three `S`, `first=[!! could not resolve the scratch dir (-) to a physical path, so nothing here
      was proved` (nothing removed).
    - `rel`, both shells: `rc=2`, all three `S`, `first=[]` (no `!!` line: harness :54; nothing removed).
  - N6, 36 lines. `c0`: `rc=2` (`verify`, `ctl`) or `rc=1` (`pgref`), `site=` and `why=` each at least 1,
    `victim=[sentinel ]` and `out=[sentinel ]`. `nbase`: **no measured log of the N6 base cells is among the r4
    evidence** (`cr1r4/log/` holds the N5 base rows and the N6 rows at r4 only; the plan's §1 inner table was
    "verified 2026-10-25 at `6afa7d98`", prose), so only what that table states is expected, per site and shape,
    both shells: `empty`: `verify` `rc=2`; `ctl` `rc=2`; `pgref` `rc=1`, each refused (`victim=[sentinel ]`,
    `out=[sentinel ]`); `dot`: `rc=1` at all three sites (`verify`: `victim` gains `.reap`, `.registry`, `.census`,
    `fsmhook` and 7 more, i.e. not `[sentinel ]`; `ctl` and `pgref`: `victim=[sentinel ]`, `out=[sentinel ]`);
    `out`: `rc=0` (`PASSED`) at all three sites, `out=` not `[sentinel ]` (it gains 12 files at `verify`, 97 entries
    at `ctl`, 17 files at `pgref`: `ls -A out | wc -l` − 1). The `site=`/`why=` counts and the exact listings of
    the `nbase` cells are not in the plan and are not expected here; they are recorded, not judged.
- **B1** (runbook §6.2): as written (`x11`'s patch anchors on `_af_lbl`'s text; the label block inserts its three
  new labels before it, so it stays the last definition, D-F4).
- **B2** (runbook §6.3): the insert writers as written, except `xrace`'s, which is (plan §6, a ratified change):

```sh
cat > "$K/ins/xrace" <<'I'   # R15 :80-:81 with pgref*/a, pgref*/b → pgref*/run/a, pgref*/run/b (the CR#1 plan §6)
find "$CTL" -name .git -type d -print0 > "$_FW_DIR/zzlist" && while IFS= read -r -d '' _zd; do git config --file "$_zd/config" core.excludesFile /nonexistent-k2; done < "$_FW_DIR/zzlist"
( _zi=0; while [ "$_zi" -lt 6000 ]; do _zi=$((_zi+1)); for _zr in "${_FW_DIR%/*}"/pgref*/run/a/.git "${_FW_DIR%/*}"/pgref*/run/b/.git; do if [ -f "$_zr/config" ] && ! grep -q nonexistent-k2 "$_zr/config"; then git config --file "$_zr/config" core.excludesFile /nonexistent-k2 || :; fi; done; sleep 0.005; done ) > /dev/null 2>&1 &
I
```

  The tree edits and the run fence as written (`$K` is fh2's).
- **B3** (runbook §6.4): `crc.sh "$(cat "$K/T")" $s $k` in place of `crc.sh 3b7ff6d5 $s $k`; the clone glob of a
  re-run's cleanup is `B3/crc/*.<T>.*/co`.
- **B4** (runbook §6.5–§6.6): as written, except `DOC -landing.md | sed -n 496,525p > "$SP/x5prep.sh"`, which is
  `LDOC 'python3 - "$SP/c11/gen11.py" <<'"'"'P'"'"'' 30 496 > "$SP/x5prep.sh"`.
- **B5–B7** (runbook §6.7): as written, all three: the plant and floor patches, `x3()`, and B5 (main), B6 (plant), B7
  (floor), each pair b53 and b32 together. The four plant and floor clones are made (§2): X3 plant and floor RUN at
  this T (plan §6 (a), the rule's result).
- **B8** (runbook §6.8): as written.

## §5 The cells

The cells, their counts and their expected values are the plan's §6 (a) table, "the per-cell result of finalhead's
run/retire rule", which is their one statement; this file only runs them, by the fences above, two runs at a time
at most (BN's two shells, B4's two jobs, B5–B7's two shells) and one at a time elsewhere. A red RUN cell is a
finding (finalhead §9); no expected value is rewritten to an observed one. Verdicts are read from the logs by grep
(BN's lines, §4; each cell's `CELL` line; X3's needles, runbook §6.7's **Expected**; B4's `PASS` / `HARNESS-ERROR`
lines). Results go to impl-d's new §T.N, from `$K/logs/` and every `$K.stop<n>/logs/`, with `$K/T`'s value.
