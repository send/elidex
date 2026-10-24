# K2 fixture git — the final-head runbook (§6 of the plan delta)

## §0 Spec coverage map

**No spec surface**: the declaration of the memo's §3 (a runbook of shell-harness runs). `preflight.py` exits 1 here by
design, as it does for `…-cr2-plan-exitpath.md`, the precedent of a section carved out of its plan.

**Status: accepted (`/elidex-plan-review` rounds R1–R3, R3's focused re-check after r5 and its narrow re-check after r6, 2026-10-24), the companion of `…-cr2-plan-finalhead.md` ("the memo").** It is the memo's §6,
moved out at r4 by the touch-time split, with its section numbers kept. Names, frame and evidence as in the memo's
header (`docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-cr2-plan-finalhead.md`); "memo §N" below is that file's §N. Frame `3b7ff6d5`, T =
`01fb2924`. Step 2 moved it into `docs/plans/` as
`2026-09-citation-hygiene-k2-fgit-hermetic-cr2-plan-finalhead-runbook.md`, its status and its handoff pointer
re-pointed (memo §9 item 2); it is an accepted body from then on and is not edited (memo §1.3).

---

## §6 The runbook (step 1, the RUN cells)

### §6.0 Common setup

All glue below runs under `/opt/homebrew/bin/bash`; the cell scripts run under `/bin/bash`, their own shebang.
**Script files.** Every fence below goes into one script file under `$H/cr2/work/fh-scripts/` (outside `$K`, so a
fresh `$K` keeps them), run as a file and never pasted into a shell's command line: `gate` reads every command line,
and a pasted batch names the wire. The definitions file is sourced by one line, which the fences do not repeat (the
**sourcing line**): `. /Users/kazuaki/repos/send.sh/elidex-fgit-handoff/cr2/work/fh/defs.sh || exit 1`. The **setup
script** is §6.0's fences in order: the definitions file's writer (below), the sourcing line, then the frame guard,
the gate's negative control, the clones and the scripts. Each **batch script** is the sourcing line and its section's
fences in order: B1 = §6.2's fence; B2 = §6.3's three; B3 = §6.4's; B4 = §6.5's, then §6.6's (the checker's
`x5build.sh` joins the same two, memo §7 C17); B5–B7 = §6.7's one fence, a script that runs the three batches in turn;
B8 = §6.8's. **One definitions file**, written by the setup script, holds every definition the fences use, the state
that crosses fences (`SP`, B4's directory) included:

```sh
mkdir -p /Users/kazuaki/repos/send.sh/elidex-fgit-handoff/cr2/work/fh
cat > /Users/kazuaki/repos/send.sh/elidex-fgit-handoff/cr2/work/fh/defs.sh <<'D'
# §6.0's definitions, sourced by the setup script and by every batch script (B1–B8). Definitions only: no side effect.
set -o pipefail
W=/Users/kazuaki/repos/send.sh/elidex-wt-fgit-hermetic
H=/Users/kazuaki/repos/send.sh/elidex-fgit-handoff
DP=docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic
PROV=-provenance.md
K=$H/cr2/work/fh                         # every clone, insert and log of step 1
SP=$K/x5                                 # B4's directory (§6.5, §6.6)
SH53=/opt/homebrew/bin/bash SH32=/bin/bash
DOC() { git -C "$W" show "${2:-3b7ff6d5}:$DP$1"; }     # every doc read: the frame's blob, never W's working file
xt() { [ -s "$1" ] && case "$(head -1 "$1")" in "$2"*) true ;; *) false ;; esac ||
  { echo "EXTRACTION FAILED $1 (empty, or its first line does not start: $2)"; return 1; }; }
logenv() { { echo "== $1 $(date)"; printf 'PATH=%s\n' "$PATH"; printf 'HOME=%s\n' "$HOME"
  command -v bash git mv sed find perl python3; mv --version 2>&1 | head -1; git --version
  /opt/homebrew/bin/git --version; /usr/bin/git --version
  $SH53 --version | head -1; $SH32 --version | head -1; df -h / | tail -1; } >> "$K/logs/env.txt"; }
mkclone() { mkdir -p "$(dirname "$K/$1")" && git clone -q --local "$W" "$K/$1" &&
  git -C "$K/$1" checkout -q --detach "${2:-3b7ff6d5}" &&
  { [ -n "${2:-}" ] || [ "$(git -C "$K/$1" diff --name-only 01fb2924 HEAD -- .claude/tools scripts | wc -l)" -eq 0 ]; } &&
  [ -z "$(git -C "$K/$1" status --porcelain)" ] || { echo "CLONE FAILED $1"; return 1; }; }
sweep() { local o; o=$(ps -ax -o pid=,pgid=,command= | awk -v g="$1" '$2==g' | cut -c1-150)   # cell15.sh's cleanup
  [ -z "$o" ] || { echo "    LEFT in group $1 (killed):"; echo "$o" | sed 's/^/      /'; kill -9 -"$1" 2>/dev/null; }; }
gate() { ps -ax -o pid=,pgid=,command= | /usr/bin/grep -e 'trip-wir[e]' -e 'config --lis[t]' -e 'k2w[d]'; }
stopgate() { local o; o=$(gate); [ -z "$o" ] && return 0
  printf 'STOP after %s (gate):\n%s\n' "$1" "$o" | tee -a "$K/logs/gate.txt"; return 1; }
D
```

`set -o pipefail` makes a failed `DOC` fail its pipeline, and every extraction below is checked by `xt` (non-empty,
and its first line starting as the doc's does), so an extraction that comes out empty or wrong stops the runbook;
every patch asserts its anchor and is written `python3 - … <<'P' || exit 1`, so a failed assert stops it too.

**A re-run after a stop starts from a fresh `$K`.** The fences are not idempotent (clones edited in place, logs appended
to, fixed names made with `mkdir`), so the stopped run's `$K` is moved aside whole (`mv "$K" "$K.stop<n>"`, its logs
kept as that run's record: `$K.stop<n>/logs/` and each run's own log under `$K.stop<n>/<batch>/`, §6.1). Once they
are kept, its clones and cell copies are deleted by an explicit list, printed first and checked to name no `logs/`
directory and no `*.log` or `log` file, then `chmod -R u+rwX` (a killed run leaves mode-000 scratch) and `rm -rf` of
exactly that list (MEMORY `feedback_disk-budget-for-measurement-clones`: never by an exclusion test inside a glob
loop): under `$K.stop<n>/`, the fourteen clones of the clone fence (`c0`, `x3main53`, `x3main32`, `x3plant53`,
`x3plant32`, `x3floor53`, `x3floor32`, `x11`, `u`, `a`, `stale`, `x8t`, `x5/p11`, `x8base`) and, of the batches that
ran, the cell copies `B1/cells/<tag>.<sh>/`, `B2/cells/<tag>.<sh>/` and `B2w/cells/<tag>.<sh>/` (each beside its
`.log`, which stays) and the glob `B3/crc/*.3b7ff6d5.*/co` (`crc.sh`'s clones, one per cell × shell × NOU on/off, from `$V/crc/$k.$c.$sn${NOU:+.nou}` with `V=$K/B3`; built from the actual directories, each `log` beside its clone stays). Then the setup script runs
again, and the batch that stopped and those after it follow; a batch completed before the stop stands, its logs in
the moved directory. A re-run from B6 or B7 leaves the earlier
batch lines out of B5–B7's script (its edits and `x3()` kept).

- **First, the frame guard**, a fence of the setup script; any failure stops the runbook for the user (memo §1.3).
  The frame `3b7ff6d5` resolves (every doc read below is `DOC`, the blob at `3b7ff6d5`, the lines the memo and this
  runbook cite); T, re-derived in `W` with landing :353–:360's command, is `01fb2924`; the tool code at `W`'s head is
  T's; and `origin`'s `k2-wire-fgit-hermetic` is at `3b7ff6d5`, so a move of the branch pushed from elsewhere (a move
  of T by another session included) stops here too:

```sh
cd "$W" || exit 1
[ "$(git rev-parse --verify -q '3b7ff6d5^{commit}')" = 3b7ff6d5816a7905100333ccc5b21c2f75b8b7c2 ] || { echo "FRAME MISSING"; exit 1; }
_job() { git show "$1:.github/workflows/ci.yml" | sed -n '/^  trip-wires:/,/^  [a-z]/p' | /usr/bin/grep -v '^ *#'; }   # landing :353–:360
T=$(for c in $(git log --format=%H e8f78896..HEAD); do
  [ -n "$(git diff --name-only "$c^" "$c" -- .claude/tools scripts)" ] && { echo "$c"; break; }
  [ "$(_job "$c^")" = "$(_job "$c")" ] || { echo "$c"; break; }
done)
case "$T" in 01fb2924*) ;; *) echo "T MOVED: '$T' (memo §1.3: the user decides)"; exit 1 ;; esac
[ "$(git diff --name-only 01fb2924 HEAD -- .claude/tools scripts | wc -l)" -eq 0 ] || { echo "TOOL CODE IS NOT T'S"; exit 1; }
[ "$(git ls-remote origin refs/heads/k2-wire-fgit-hermetic | cut -f1)" = 3b7ff6d5816a7905100333ccc5b21c2f75b8b7c2 ] ||
  { echo "ORIGIN'S k2-wire-fgit-hermetic IS NOT AT 3b7ff6d5 (memo §1.3: the user decides)"; exit 1; }
mkdir -p "$K/logs" "$K/ins" && cd "$K" || exit 1
echo "frame guard: passed"
```

- **The gate's negative control**, before B1 (R1 IMP-3), one probe per pattern:

```sh
[ -z "$(gate)" ] || { echo "GATE NOT CLEAN BEFORE B1"; gate; exit 1; }
( exec -a 'trip-wire-gate-probe' sleep 20 ) & p1=$!; ( exec -a 'git config --list' sleep 20 ) & p2=$!
( exec -a 'k2wd-gate-probe' sleep 20 ) & p3=$!; sleep 1
[ "$(gate | /usr/bin/grep -c -e 'trip-wire-gate-prob[e]' -e 'git config --list 2[0]' -e 'k2wd-gate-prob[e]')" -eq 3 ] || { echo "GATE BLIND"; exit 1; }
kill "$p1" "$p2" "$p3"; wait "$p1" "$p2" "$p3" 2>/dev/null; [ -z "$(gate)" ] || { echo "GATE NOT CLEAN AFTER PROBE"; exit 1; }
```

- **Clones**, each made by `mkclone` (`git clone --local`, detached at `3b7ff6d5` unless a commit is given; `W` is
  never written, A14):

```sh
for c in c0 x3main53 x3main32 x3plant53 x3plant32 x3floor53 x3floor32 x11 u a stale x8t x5/p11; do
  mkclone "$c" || exit 1; done
mkclone x8base e8f78896 || exit 1
```

  `c0` is the source tree of every `cell15.sh`/`cellw.sh` cell without a tree edit and `crc.sh`'s `K2_REPO`; `x5/p11`
  is "X5 at T"'s `p11/`, a clone of T (landing :445–:446). About 100 MB a clone and a cell copy (`W`'s tree is 34 MB
  without `target`, the object store 65 MB); with 50 cell copies, budget ≈ 7 GB for one `$K`. A stopped run's
  `$K.stop<n>` keeps its logs and small files (scripts, inserts, homes); its clones and cell copies are deleted (above), before the fresh `$K` is set
  up, so a re-run stays within the same budget. `df -h /` before each batch, recorded by `logenv` (MEMORY
  `feedback_disk-budget-for-measurement-clones`).
- **The scripts** (A13), extracted verbatim from the frame and patched by the docs' own patches, each patch asserting
  its anchor:

```sh
cd "$K" || exit 1
DOC -pr527.md     | sed -n 627,646p > cell15.sh   && xt cell15.sh   '#!/bin/bash' || exit 1   # PR527 round 13 (fence :626–:647)
DOC -pr527-r15.md | sed -n 394,414p > cellw.sh    && xt cellw.sh    '#!/bin/bash' || exit 1   # R15 round 18
DOC -pr527.md     | sed -n 333,396p > crc.sh      && xt crc.sh      '#!/bin/bash' || exit 1   # PR527 "X13: the cell script"
DOC -landing.md   | sed -n 436,441p > wdpatch.sh  && xt wdpatch.sh  "python3 - cell15.sh cellw.sh <<'P'" && /bin/bash wdpatch.sh  || exit 1   # landing :436–:441
DOC -landing.md   | sed -n 536,551p > crcpatch.sh && xt crcpatch.sh "python3 - crc.sh <<'P'" && /bin/bash crcpatch.sh || exit 1   # landing :536–:551
for f in cell15.sh cellw.sh crc.sh; do $SH32 -n $f && $SH53 -n $f || { echo "BAD $f"; exit 1; }; done
```

  `cell15.sh`'s source is PR527 :627–:646, the body of the fence :626–:647 (:648 is blank and the findings table
  follows; the runbook's "PR527:627-650" overreaches into it; `H/cell15.sh` adds only a line-2 comment), memo §7 C5. The
  checker runs this block, taken from this runbook, on a scratch directory: every extraction passes `xt`, both patches
  apply, all three scripts are clean under `bash -n` on 3.2 and 5.3 (memo §7 C5). Every runner then has `alarm 120` and a
  `kill -9 -<pgid>` watchdog at 123 s.
- **Batches**, in this order, each with `logenv <batch>` first and `stopgate <batch> || exit 1` last. A line from
  `gate` stops the runbook and is read by the driver before anything is killed (other sessions run trip-wires too,
  `…-pr527.md` :206; only a group this run started is killed). The driver attributes it by its command line and its
  working directory (`lsof -a -d cwd -p <pid>`): with a path under `$K` it is this run's, and a process not clearly
  another session's counts as this run's. Another session's: wait until it ends, then re-run the batch from a fresh
  `$K` (above). This run's: a run reached its bound, a finding in that RUN cell (memo §9). The order: B1 X6,
  X15, X11; B2 X16 and X5's `xrace`; B3 X13; B4 X5's draft-10 jobs; B5 X3 main; B6 X3 plant; B7 X3 floor; B8 X8, with
  nothing else running. B1–B3 run one cell at a time; B4 two jobs at a time (landing :526); B5–B7 b53 and b32
  together (the shape `21526e80`'s times were measured in).
- **Insert files** are written with quoted here-documents, so each is the doc's text byte for byte plus a final
  newline (`cell15.sh` puts the file's content directly before the `built` line).

### §6.1 Results, and how each cell is judged

One convention: **each batch's summary goes to `$K/logs/<batch>.txt`** (the runner's `CELL`, `X3` or `X8` line, its
first `!!` lines and any `LEFT in group` lines, or, for B4, `eval11.sh`'s verdict lines), and **each run's own log
under `$K/<batch>/`**: `cell15.sh`/`cellw.sh` write `$K2_CELLS/cells/<tag>.<sh>.log` with `K2_CELLS=$K/<batch>`
(`$K/B2w` for `cellw.sh`'s cells); `crc.sh` writes under its `K2_CELLS=$K/B3`; B4's cells write the companion's layout,
`$K/x5/c11/logs/<tag>.log` (B4's `<batch>` directory is `$K/x5`); `x3` and `x8` write `$K/<batch>/<clone>….log`.
`cell15.sh`/`cellw.sh` print `CELL <tag> <sh> rc= t= CF= NE= PASSED= …` and the first two (three) `!!` lines;
`crc.sh` prints `CELL <k> <commit> <sh> [nounset-OFF] rc= NE= CF= PASSED=` and the first four `!!` lines. A run that
stopped keeps the same layout under its moved-aside `$K.stop<n>/` (§6.0): its `logs/` (the batch summaries, `env.txt`,
`gate.txt`) and its runs' own logs. impl-d's §T.L tables the summaries from `$K/logs/` and every `$K.stop<n>/logs/`
(memo §9 item 3), a batch completed before a stop standing as recorded there.

### §6.2 B1: X6, X15, X11

```sh
cat > "$K/ins/x6tpl" <<'I'
mkdir -p "$_FGIT_VOID" && printf 'ref: refs/heads/x\n' > "$_FGIT_VOID/HEAD" && : > "$_FGIT_VOID/config"
I
cat > "$K/ins/x6inc" <<'I'
git -C "$CTL/clean" config include.path /nonexistent-k2
I
cat > "$K/ins/x15" <<'I'
git init -q "$CTL/zzx" && mkdir -p "$_FW_DIR/zzt/a" && : > "$_FW_DIR/zzt/a/HEAD" && ln -s "$_FW_DIR/zzt" "$CTL/zzx/.git/k2link"
I
: > "$K/ins/empty"
python3 - "$K/x11/.claude/tools/webref-generic-core-trip-wire.controls.sh" <<'P' || exit 1
import sys; p = sys.argv[1]; s = open(p).read()
a = '_af_lbl="a record\'s anchor is fresh only if it changes one line, once, and keeps what it does not read"\n'
assert s.count(a) == 1, a        # controls :123, the last _lbl definition
s = s.replace(a, a + '_x_lbl="an unrecorded probe"\necho "$_x_lbl" >/dev/null\n'); open(p, 'w').write(s)
P
logenv B1
for s in b53 b32; do
  for t in x6tpl x6inc x15; do K2_CELLS=$K/B1 /bin/bash "$K/cell15.sh" "$K/c0" $t $s "$K/ins/$t"; done
  K2_CELLS=$K/B1 /bin/bash "$K/cell15.sh" "$K/x11" x11 $s "$K/ins/empty"
done > "$K/logs/B1.txt" 2>&1
stopgate B1 || exit 1
```

| cell | insert or edit (verbatim; source) | expected (quoted) |
|---|---|---|
| X6 `x6tpl` | landing :397, first insert | "red: the template through **P-c**" — "Not P-d … Not P-a" (landing :397) |
| X6 `x6inc` | landing :397, second insert | "red: … the include through **P-g**" |
| X15 | landing :406 | "rc 1, P-g, whose message names `zzx` by the shape rule only; no "symlink to a tree holding a git dir" entry for `k2link`, so the search did not run behind it" |
| X11 | landing :401's two lines, after controls :123 | "red, and the ratchet lists it": rc ≠ 0 and the ratchet's report names `an unrecorded probe` |

### §6.3 B2: X16 (20 cells × 2 shells) and X5's `xrace` (1 × 2)

The twenty insert files, each the doc's text byte for byte (the checker runs this block on a scratch `K` and compares
every file with the text it extracts from the docs at the frame, memo §7 C5):

```sh
cat > "$K/ins/fifo-commondir" <<'I'   # PR527 :458, %s = commondir
rm -f "$CTL/clean/.git/commondir" && mkfifo "$CTL/clean/.git/commondir"
I
cat > "$K/ins/fifo-HEAD" <<'I'   # PR527 :458, %s = HEAD
rm -f "$CTL/clean/.git/HEAD" && mkfifo "$CTL/clean/.git/HEAD"
I
cat > "$K/ins/fifo-config" <<'I'   # PR527 :458, %s = config
rm -f "$CTL/clean/.git/config" && mkfifo "$CTL/clean/.git/config"
I
cat > "$K/ins/fifo-config.worktree" <<'I'   # PR527 :458, %s = config.worktree
rm -f "$CTL/clean/.git/config.worktree" && mkfifo "$CTL/clean/.git/config.worktree"
I
: > "$K/ins/clean"                       # PR527 :610, "nothing"
cat > "$K/ins/xcommon" <<'I'   # PR527 :611
git init -q "$CTL/zzr" && rm -f "$CTL/zzr/.git/config" && mkfifo "$CTL/zzr/.git/config" && printf '../../zzr/.git\n' > "$CTL/clean/.git/commondir"
I
cat > "$K/ins/bareh" <<'I'   # PR527 :612
git init -q --bare "$CTL/zzb.git" && ln -s / "$CTL/zzb.git/k2root"
I
cat > "$K/ins/outsidein" <<'I'   # PR527 :613
git init -q "$CTL/zzr" && ln -s / "$CTL/zzr/.git/k2root" && ln -s zzr "$CTL/zzq"
I
cat > "$K/ins/hard" <<'I'   # PR527 :614
printf x > "$_FW_DIR/zzh" && mkdir -p "$CTL/clean/.git/info" && rm -f "$CTL/clean/.git/info/exclude" && ln "$_FW_DIR/zzh" "$CTL/clean/.git/info/exclude"
I
cat > "$K/ins/xlink" <<'I'   # PR527 :615
git init -q "$_FW_DIR/zzo" && rm -f "$_FW_DIR/zzo/.git/config" && mkfifo "$_FW_DIR/zzo/.git/config" && ln -s "$_FW_DIR/zzo" "$CTL/zzl" && printf '../../zzl/.git\n' > "$CTL/clean/.git/commondir"
I
cat > "$K/ins/xincreg" <<'I'   # PR527 :616
printf '[core]\n\texcludesFile = /nonexistent-k2\n' > "$CTL/zzi" && printf '[include]\n\tpath = ../../zzi\n' >> "$CTL/clean/.git/config"
I
cat > "$K/ins/xrefpoison" <<'I'   # PR527 :720
git init -q "$_FW_DIR/pq/a" && git init -q "$_FW_DIR/pq/b" && git config --file "$_FW_DIR/pq/a/.git/config" core.excludesFile /nonexistent-k2 && git config --file "$_FW_DIR/pq/b/.git/config" core.excludesFile /nonexistent-k2 && find "$CTL" -name .git -type d -print0 > "$_FW_DIR/zzlist" && while IFS= read -r -d '' _zd; do git config --file "$_zd/config" core.excludesFile /nonexistent-k2; done < "$_FW_DIR/zzlist"
I
cat > "$K/ins/xrefctl" <<'I'   # PR527 :721
find "$CTL" -name .git -type d -print0 > "$_FW_DIR/zzlist" && while IFS= read -r -d '' _zd; do git config --file "$_zd/config" core.excludesFile /nonexistent-k2; done < "$_FW_DIR/zzlist"
I
cat > "$K/ins/lnblind2" <<'I'   # R15 :79
mkdir -p "$_FW_DIR/zzt/a/objects" && : > "$_FW_DIR/zzt/a/HEAD" && ln -s "$_FW_DIR/zzt" "$CTL/zzy" && ln -s /dev/null "$_FW_DIR/prel" && ln -s /dev/null "$_FW_DIR/pgls"
I
cat > "$K/ins/xrace" <<'I'   # R15 :80-:81: xrefctl's insert (PR527 :721), then the loop
find "$CTL" -name .git -type d -print0 > "$_FW_DIR/zzlist" && while IFS= read -r -d '' _zd; do git config --file "$_zd/config" core.excludesFile /nonexistent-k2; done < "$_FW_DIR/zzlist"
( _zi=0; while [ "$_zi" -lt 6000 ]; do _zi=$((_zi+1)); for _zr in "${_FW_DIR%/*}"/pgref*/a/.git "${_FW_DIR%/*}"/pgref*/b/.git; do if [ -f "$_zr/config" ] && ! grep -q nonexistent-k2 "$_zr/config"; then git config --file "$_zr/config" core.excludesFile /nonexistent-k2 || :; fi; done; sleep 0.005; done ) > /dev/null 2>&1 &
I
cat > "$K/ins/xbare" <<'I'   # R15 :190
ln -s /dev/null "$CTL/.bare"
I
cat > "$K/ins/xanchor" <<'I'   # R15 :190-:191
ln -s /dev/null "$CTL/.anchor"
I
cat > "$K/ins/xexecarith" <<'I'   # R15 :191
exec 2>/dev/null
: $((1/0))
I
cat > "$K/ins/gitfn" <<'I'   # R15 :417
git() { command git -c user.name=k2 -c user.email=k2@example.invalid "$@"; }
I
cat > "$K/ins/staleshim" <<'I'   # R15 :422-:423; "$S" written out below
mkdir -p "$S/prep" && ln -sfn /dev/null "$S/prep/.anchor"
printf '#!/bin/sh\ncase "$*" in *verifyXXXXXX*) echo "%s"; exit 0;; esac\nexec /usr/bin/mktemp "$@"\n' "$S/prep" > "$S/cpath/mktemp" && chmod +x "$S/cpath/mktemp"
I
```

The FIFO inserts are PR527 :458's template with `%s` the name (the R26③ cells); the round-13 inserts are the second
column of PR527 :611–:616 (round 13's, not round 12's :505–:509); `xanchor`'s text is wrapped across R15 :190–:191 in
the doc and is one line; `xrace` is two lines, `xrefctl`'s insert and the loop (R15 :80, "then").

Tree edits (memo D-F5), each in its own clone:

```sh
python3 - "$K/u/.claude/tools/webref-generic-core-trip-wire.mutations.sh" <<'P' || exit 1
import sys; p = sys.argv[1]; s = open(p).read(); a = '\n_MUT_UNRECORDED_MAX=18\n'          # mutations :271
assert s.count(a) == 1; open(p, 'w').write(s.replace(a, '\n_MUT_UNRECORDED_MAX=0\n'))
P
python3 - "$K/a/.claude/tools/webref-generic-core-trip-wire.records.sh" <<'P' || exit 1
import sys; p = sys.argv[1]; s = open(p).read()
r = 'fixtures:s/^: > "[$]_FW_DIR\\/built"$/git -C "$CTL\\/clean" config --add core.bare false; &/'   # records :196
assert s.count(r) == 1 and r.count('built"$/') == 1
open(p, 'w').write(s.replace(r, r.replace('built"$/', 'builtZZSTALE"$/')))
P
python3 - "$K/stale/.claude/tools/webref-generic-core-trip-wire.records.sh" <<'P' || exit 1
import sys; p = sys.argv[1]; s = open(p).read(); a = '\nMUTANTS\n'                          # records :595
assert s.count(a) == 1
open(p, 'w').write(s.replace(a, '\ns/K2NEVERMATCHES/x/\tthe fixture build window completed' + a))
P
python3 - "$K/ins/staleshim" "$K/B2w" <<'P' || exit 1      # R15 :418: "$S" is the cell root, written out at run time
import sys; p, root = sys.argv[1:]; s = open(p).read(); assert s.count('"$S/') == 5
open(p, 'w').write(s.replace('"$S/', '"' + root + '/'))
P
```

```sh
logenv B2
x16() { K2_CELLS=$K/B2 /bin/bash "$K/cell15.sh" "$K/$1" "$2" "$3" "$K/ins/$2"; }
for s in b53 b32; do
  for t in fifo-commondir fifo-HEAD fifo-config fifo-config.worktree clean xcommon bareh outsidein hard xlink \
           xincreg xrefpoison xrefctl lnblind2 xexecarith gitfn xrace; do x16 c0 $t $s; done
  x16 u xbare $s; x16 a xanchor $s
done > "$K/logs/B2.txt" 2>&1
mkdir -p "$K/B2w/cpath"
for s in b53 b32; do K2_CELLS=$K/B2w /bin/bash "$K/cellw.sh" "$K/stale" stalectl $s -; done >> "$K/logs/B2.txt" 2>&1
for s in b53 b32; do
  K2_CELLS=$K/B2w /bin/bash "$K/cellw.sh" "$K/stale" staleshim $s "$K/ins/staleshim" >> "$K/logs/B2.txt" 2>&1
  rm -rf "$K/B2w/cpath/mktemp" "$K/B2w/prep"          # R15 :427–:428: removed between the runs
done
stopgate B2 || exit 1
```

**Expected** (landing :403, quoted): "each red cell: rc 1, P-g, the untrusted-build stop runs no control, and the run
ends in under 40 s; `clean` PASSED" — the 40 s read from `t=` in the `CELL` line; "runs no control" read from the
cell's log by the driver. Own verdicts: "`xbare` rc 1 with the ratchet's `… labels have no mutation record, against a
ratchet of 0.`, `xanchor` rc 1 with `… its anchor is stale`, `xexecarith` rc 2 with W's sentence "the fixtures file
left the window's stderr redirected""; "`stalectl` (no insert) rc 1 with `… its anchor is stale`; `staleshim` … rc 0,
PASSED, the declared class-(c) boundary (D18-A)"; "`gitfn` rc 1 on both shells, P-a with `command line:` origins,
and the untrusted-build stop runs no control". `fifo-config.worktree`: red by M-SHAPE ("M-SHAPE reds it anyway",
PR527 :468). **`xrace`** (X5): class (c), "not owed" (corpus :276; D15-C, R15 :92), so its verdict is recorded, not
judged; it fails only by reaching the bound. Its last runs: rc 0, PASSED, both shells, on the draft-17 and draft-18
prototypes, with the window's bash and a `sleep` left in the group and killed by group (R15 :90, :211); `cell15.sh`'s
cleanup does the same here, and its `LEFT in group` lines are recorded.

### §6.4 B3: X13 (36 runs)

```sh
logenv B3
for k in clean sealfail env0 garbagehead w2rec lblrename w3ar sealdotdot reftable; do for s in b53 b32; do
  env -u NOU K2_CELLS=$K/B3 K2_REPO=$K/c0 /bin/bash "$K/crc.sh" 3b7ff6d5 $s $k
  env NOU=1  K2_CELLS=$K/B3 K2_REPO=$K/c0 /bin/bash "$K/crc.sh" 3b7ff6d5 $s $k
done; done > "$K/logs/B3.txt" 2>&1
stopgate B3 || exit 1
```

`crc.sh` clones `K2_REPO` at `3b7ff6d5` itself (one fresh clone per cell). Every cell's anchor is present once at T
(memo §7 C5: w3ar's `mkdir -p "$CTL/walk/sub"`, the `built` line, w2rec's `_fgit_window_verdict_exit` body with one
4-space `exit 2` and one 2-space `exit 1`, its `done` marker, `_pa_lbl="`, `set -euo pipefail`, and the seal of
`err/control.py` that sealfail's `chmod` shim refuses). **Expected** (landing :404): "the same exit status, NE/CF
counts and first four `!!` lines both ways, on both shells": per shell and cell, the run's block (its `CELL` line and
indented lines) with ` nounset-OFF` and `.nou` deleted equals the other run's. The per-cell values measured at
`8413a4db` (PR527 :323–:326: clean 0 PASSED · sealfail 1 · env0 0 · garbagehead 1 · w2rec 1 · lblrename 2 · w3ar 1 ·
sealdotdot 1 · reftable 0 PASSED) are recorded beside, not judged.

### §6.5 B4: X5's draft-10 jobs, the instruments

`cell9.sh` and `eval11.sh` (the companion's names: `c11/cell9.sh` and `c11/eval11.sh` are copies of `c9/cell9.sh` and
`c9/eval9.sh`, PROV §E.7), built from the verbatim scripts and the documented edit and arms (memo D-F6):

```sh
mkdir -p "$SP/c11"                                                       # SP: defs.sh
DOC $PROV ff1322bb | sed -n 832,845p > "$SP/c11/cell9.sh" && xt "$SP/c11/cell9.sh" '#!/bin/bash' || exit 1   # cell6.sh = cell8.sh (PROV@453b7b0f :811)
python3 - "$SP/c11/cell9.sh" <<'P' || exit 1                            # PROV :825: HOME="$OUT/home" after TMPDIR=…, on the env line
import sys; p=sys.argv[1]; s=open(p).read(); a='env TMPDIR="$OUT/tmp/$tag" '; assert s.count(a)==1
open(p,'w').write(s.replace(a, a+'HOME="$OUT/home" '))
P
DOC $PROV 453b7b0f | sed -n 866,889p > "$SP/c11/eval11.sh" && xt "$SP/c11/eval11.sh" '#!/bin/bash' || exit 1   # eval8.sh, 24 lines
DOC $PROV | sed -n 830,832p > "$SP/c11/arms" && xt "$SP/c11/arms" '    BEFORE-E2|BEFORE-E3|BEFORE-AR|BEFORE-E3g)' || exit 1   # the arms, fence PROV :829–:833
python3 - "$SP/c11/eval11.sh" "$SP/c11/arms" <<'P' || exit 1             # PROV :827: added before BEFORE-D2|BEFORE-D4)
import sys; p,a=sys.argv[1:]; s=open(p).read(); k='    BEFORE-D2|BEFORE-D4)'; assert s.count(k)==1
open(p,'w').write(s.replace(k, open(a).read()+k))
P
chmod +x "$SP/c11/cell9.sh" "$SP/c11/eval11.sh"
mkdir -p "$SP/c11/logs" "$SP/c11/pd" "$SP/c11/home"           # cell6.sh writes logs/ and pd/ but makes neither; HOME's dir
```

### §6.6 B4: X5's draft-10 jobs, the run (32 jobs)

"X5 at T"'s block (landing :494–:526), with its two reads taken from the frame: the `awk` of :494–:495 reads
the frame's provenance (`DOC $PROV`, saved first) instead of the working file, and :496–:525 (the re-anchoring patch, the generation, `cellT.sh`) run as
written; :526's run takes the 32 jobs of memo D-F6, not all 38:

```sh
DOC $PROV > "$SP/prov.md" || exit 1            # read whole first: the awk's early exit would make a pipe's status 141
awk '/^### §E.7/{f=1} f&&/^```python$/{p=1;next} p&&/^```$/{exit} p' "$SP/prov.md" > "$SP/c11/gen11.py"   # landing :494–:495
xt "$SP/c11/gen11.py" '#!/usr/bin/env python3' || exit 1
DOC -landing.md | sed -n 496,525p > "$SP/x5prep.sh" && xt "$SP/x5prep.sh" 'python3 - "$SP/c11/gen11.py"' || exit 1
( cd "$SP" && SP=$SP /bin/bash "$SP/x5prep.sh" ) > "$SP/gen.out" 2>&1; /usr/bin/grep -qx '38 jobs' "$SP/gen.out" || exit 1
awk -F'\037' '$3!="AFTER-E3g" && $3!="G"' "$SP/c11/jobs.tsv" > "$SP/c11/jobs.run"
[ "$(wc -l < "$SP/c11/jobs.run")" -eq 32 ] || exit 1
logenv B4
tr '\n' '\0' < "$SP/c11/jobs.run" | xargs -0 -n1 -P2 "$SP/c11/cellT.sh" > "$SP/c11/raw.tsv"   # landing :526, filtered input
/bin/bash "$SP/c11/eval11.sh" > "$K/logs/B4.txt" 2>&1
stopgate B4 || exit 1
```

**Expected** (landing :396, quoted): "every AFTER row PASS", and REF green (`eval9.sh`'s REF arm): 32 lines beginning
`PASS` in `B4.txt`, no `HARNESS-ERROR` (cellT.sh's line for a run that reached the bound, landing :523). The job
kinds and their arms: memo §7 C17. Each job's wire is `cell6.sh`'s: `PATH="$gp:/usr/bin:/bin:/opt/homebrew/bin"` (the
configuration's git first), `TMPDIR` and `HOME` under `$SP/c11`, and `K2_CORPUS_PDUMP` set to an empty file that no
part of T reads (memo §7 C17), so `same` is 0 and is read by no arm of the 32. Wall: 32 jobs of ≈ 20–35 s, two at a time,
≈ 10 min.

### §6.7 B5–B7: X3 (6 runs)

```sh
python3 - "$K/x3plant53" "$K/x3plant32" <<'P' || exit 1
import sys
for d in sys.argv[1:]:
    p = d + '/.claude/tools/webref-generic-core-trip-wire.fixtures.sh'; s = open(p).read()
    a = ': > "$_FW_DIR/built"\n'; assert s.count(a) == 1      # fixtures :705
    open(p, 'w').write(s.replace(a, 'ln -s /dev/null "$CTL/.genmutants"\n' + a))
P
python3 - "$K/x3floor53" "$K/x3floor32" <<'P' || exit 1
import sys
for d in sys.argv[1:]:
    p = d + '/.claude/tools/webref-generic-core-trip-wire.mutgen.sh'; s = open(p).read()
    a = '_mut_regex_mutants "$_gr_v" "$_gr_val" >> "$_VFY/.genmutants" ||'; assert s.count(a) == 1   # mutgen :304, in _mut_gen_run (:287–:348)
    open(p, 'w').write(s.replace(a, '_mut_regex_mutants "$_gr_v" "$_gr_val" >> /dev/null ||'))
P
x3() {  # x3 <batch> <clone> <shell path>
  local L=$K/$1/x3-$2.log; mkdir -p "$K/$1"
  ( cd "$K/$2" && exec env WEBREF_WIRE_MUTANTS=1 perl -e '$SIG{INT}="DEFAULT"; $SIG{TERM}="DEFAULT"; $SIG{HUP}="DEFAULT"; setpgrp; alarm 18000; exec @ARGV' \
      "$3" .claude/tools/webref-generic-core-trip-wire.sh < /dev/null ) > "$L" 2>&1 & local g=$!
  ( exec perl -e 'setpgrp; sleep $ARGV[0]; kill "KILL", -$ARGV[1]' 18003 "$g" ) & local wd=$!
  local s0; s0=$(date +%s); local rc=0; wait "$g" || rc=$?; kill -9 -"$wd" 2>/dev/null
  { echo "X3 $2 rc=$rc t=$(( $(date +%s) - s0 ))s"; sweep "$g"; } >> "$K/logs/$1.txt"; }
logenv B5; x3 B5 x3main53  $SH53 & x3 B5 x3main32  $SH32 & wait; stopgate B5 || exit 1
logenv B6; x3 B6 x3plant53 $SH53 & x3 B6 x3plant32 $SH32 & wait; stopgate B6 || exit 1
logenv B7; x3 B7 x3floor53 $SH53 & x3 B7 x3floor32 $SH32 & wait; stopgate B7 || exit 1
```

The command is landing :393's (`WEBREF_WIRE_MUTANTS=1 perl -e '$SIG{INT}="DEFAULT"; $SIG{TERM}="DEFAULT";
$SIG{HUP}="DEFAULT"; exec @ARGV' $SH $W`) with `setpgrp; alarm 18000` before its `exec`, a watchdog that kills the
run's group at 18003 s, and the group swept after the `wait` (memo D-F2; memo §2 IS). The `perl` resets the three signals that a
backgrounded run would otherwise inherit ignored (BS-11, landing :393). `$SH` is `/opt/homebrew/bin/bash` (5.3.20) or
`/bin/bash` (3.2.57); `PATH`, `HOME` and the rest are the driver's, logged (`PATH` holds
`/opt/homebrew/opt/coreutils/libexec/gnubin` before `/bin` in this session: GNU `mv` refuses a case-only rename on
APFS, the cause of `21526e80`'s two "not killed" records 127/128, fixed by `b3b64aa4`, `H/DECISIONS-R7.md` :825–:831;
MEMORY `reference_gnu-mv-case-only-rename-apfs`). Wall: ≈ 8.9 ks a pair estimated (memo D-F2), ≈ 7.4 h for B5–B7.

**Expected** (landing :393, quoted). Main, per shell: "three hits in every column" for `/usr/bin/grep -F -e
'entr(ies), 0 not killed as named' -e ', 0 neither killed nor argued equivalent' -e 'trip-wire PASSED'` over the run's
log, and "the run must also be PASSED". Plant: "after C11 the plant has no effect, and the run reports its normal
non-zero `N mutant(s)` and PASSED" (`generated boundary set: N mutant(s)`, N > 0; `21526e80` gave 50, and `mutgen.sh`,
`$K2RE` and `$K2RE_PATH` are unchanged since, memo §7 C7). Floor: "red, `!! the generated boundary set ran 0 mutant(s), but
the generator derives 50 …` …, and `1 neither killed nor argued equivalent`" — the wire wraps that message after
"derives", so it is read with `/usr/bin/grep -A1 'ran 0 mutant(s), but the generator derives'` (the next line starts
`50 from`), as `21526e80`'s log shows (`H/runners/logs/s8b-x3/floor-b53.log`). A run that ends at the bound (rc 137 or
142, or the watchdog's) fails its cell.

### §6.8 B8: X8 (6 runs, alone)

```sh
x8() {  # x8 <clone> <i>
  local L=$K/B8/x8-$1-$2.log; mkdir -p "$K/B8"
  ( cd "$K/$1" && exec perl -e 'setpgrp; exec @ARGV' /usr/bin/time -p bash scripts/trip-wires.sh ) > "$L" 2>&1 & local g=$!
  ( exec perl -e 'setpgrp; sleep $ARGV[0]; kill "KILL", -$ARGV[1]' 600 "$g" ) & local wd=$!
  local rc=0; wait "$g" || rc=$?; kill -9 -"$wd" 2>/dev/null
  { echo "X8 $1 $2 rc=$rc $(/usr/bin/grep '^real' "$L")"; sweep "$g"; } >> "$K/logs/B8.txt"; }
logenv B8
for i in 1 2 3; do x8 x8base $i; x8 x8t $i; done
stopgate B8 || exit 1
```

Base first, then the branch, alternated (landing :398; the runbook's B1 b1 B2 b2 B3 b3). `bash` is `PATH`'s (memo D-F3).
The bound is the watchdog, which kills the whole group (`/usr/bin/time`, the timed `bash` and its wires; R1 IMP-3):
an alarm in `time` would end `time` alone. **Expected**: "the six times recorded in the open record (§13); nothing
decided and nothing written to `ci.yml`" — the `real` lines. No measurement at T exists; a solo
`scripts/trip-wires.sh` took 28 s at `11e36a27` (impl-d :122), so 600 s is a hang bound only.
