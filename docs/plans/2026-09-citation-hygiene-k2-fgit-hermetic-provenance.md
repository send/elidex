# K2 fixture git — provenance companion

This file is the provenance for `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md`, called "the
design memo" below. It holds:
- the measurements behind that memo's decisions (§A);
- what became of the replaced branch (§B);
- the premises found false (§C).

Every plan-review round's dispositions and terminators (§D) are in
`docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-reviews.md`, split out unchanged.

The design memo keeps only its live decisions. Section references here are to the design
memo **as of the draft that the section or row belongs to** unless a file is named; a superseded draft is read with `git show <sha>:<memo path>` (drafts: 1 `bd3dc513`,
2 `0142f47a`, 3 `2b89ef7c`, 4 `57e5419f`, 5 `e8c1bdb8`, 6 `8b6a4005`, 7 `ff1322bb`, 8 `453b7b0f`, 9 `d5dacd56`).

⚠ This file follows the design memo's rule: every figure is a measurement at a named SHA or scratch
prototype, with its command. `$S` is scratch. `/usr/bin/grep` is spelled out on purpose.

---

## §0 Spec coverage map

**No spec surface** — this file records measurements and review dispositions only. It uses the same
by-design shape as the design memo's §2.5: `preflight.py` exits 1 here because no table follows this
heading. The repo-wide preflight command runs over every tracked plan memo, so this file needs the
heading like any other.

## §A Measurements

### §A.1 At the two heads (moved from draft 2 §2)

**How the runs were made.** Each run was:

```sh
( cd <checkout> && env <ASSIGNMENTS> <shell> .claude/tools/webref-generic-core-trip-wire.sh ) > log 2>&1
```

NE counts `CONTROL NOT EXERCISED` and CF counts `CONTROL FAILED`, both with `/usr/bin/grep -c`. Each
checkout was a `git clone --local`, checked with `git rev-parse HEAD`. Bash 5.3.20 and 3.2.57 agree
wherever both were run.

| caller environment | `ff6b99a3` | `e8f78896` |
|---|---|---|
| `HOME` + `.gitconfig` `core.excludesFile` = a file holding `*.py` | rc 0, NE 0 | — |
| `HOME` + empty `.config/git/ignore` (control) | rc 0, NE 0 | — |
| `HOME` + `.config/git/ignore` = `*.py` | rc 1, NE 12, CF 0 | rc 1, NE 11 (3.2) |
| `XDG_CONFIG_HOME` + `git/ignore` = `*.py` | rc 1, NE 11, CF 1 | — |
| `XDG_CONFIG_HOME` + `git/attributes` = `*.py working-tree-encoding=UTF-16LE` | rc 1, NE 17, CF 5 (3.2) | rc 1, NE 17, CF 4 (3.2) |
| `XDG_CONFIG_HOME` + `git/config` `init.templateDir` | rc 0 (3.2) | — |
| `GIT_TEMPLATE_DIR` + `info/exclude` = `*.py` | rc 0 (lane SSoT) | rc 1, NE 11 (3.2) |

**Why the HOME and XDG rows differ: 12 NE against 11 NE + 1 CF.** The cause is `ff6b99a3`'s `_envlive`,
which passes `"HOME=${HOME:-}"` into its own `env -i`. See it with:

```sh
git show ff6b99a3:.claude/tools/webref-generic-core-trip-wire.controls.sh | /usr/bin/grep -n 'HOME=\${HOME'   # line 814
```

- A hostile `HOME` contaminates the probe's baseline, so `envscrub` is reported NE.
- A hostile `XDG_CONFIG_HOME` is dropped by the probe but reaches `_fgit`, so `envscrub` is reported CF.

That the lane SSoT's "12" for XDG counted both kinds together is an **inference**: its log was not
recorded.

**Content and commit channels** (the §1 table). These ran under an otherwise empty git environment, on
git 2.55.0 and Apple 2.54.0:
- baseline: `victim.py` blob `7d4290a1…`;
- `$HOME/.config/git/attributes` `working-tree-encoding=UTF-16LE`: blob `b8b71b2a…`;
- a template `hooks/pre-commit` running `git rm -q --cached victim.py`: `victim.py` is absent from the
  index and from HEAD, and `commit` still exits 0.

### §A.2 Prototype A — `_fgit` from nothing (draft 1)

This is the base with draft-1 §3.3 applied. It gave rc 0, NE 0, CF 0 and `PASSED` on both shells, under each of:
- a clean caller;
- `HOME/.gitconfig` setting `core.excludesFile`;
- `HOME/.config/git/ignore`;
- `XDG_CONFIG_HOME` pointing at an ignore file;
- `XDG_CONFIG_HOME` pointing at an attributes file;
- `GIT_TEMPLATE_DIR` pointing at an exclude;
- the `GIT_CONFIG_COUNT` trio;
- `GIT_CONFIG_SYSTEM=<hostile>`;
- `GIT_DIR=<other>`.

Without `mkdir -p .git/info` in `notcommitted`, a clean caller gives rc 1 with 1 NE (`per-clone
info/exclude cannot hide an entry`).

### §A.3 Prototype B — postconditions and pins (draft 2; extended in draft 3)

This was a standalone script, run on both shells and on both gits.

**F1 planting (template content and `include.path`):**
- A template holding `config` (`core.excludesFile`, `core.hooksPath`) and `HEAD` (`ref: refs/heads/hostile`)
  leaves `find .git | sort` **equal** to a void init, but `diff -r` shows a difference.
- The copied keys report `local` with origin `file:.git/config`.
- A local `include.path` reports origin `file:<outside>`.
- A clean probe passes P-a and P-d.

**Allowlist mutants against P-b/d/e (draft-2 §6.2):**
- drop `HOME`: P-b red;
- source-time `HOME=$HOME`: P-b only;
- call-time `HOME=$HOME`: P-b red;
- drop NOSYSTEM: P-b red (`/opt/homebrew/etc/gitconfig`);
- drop ATTR_NOSYSTEM: P-b red (`/opt/homebrew/etc/gitattributes`, printed although the file is absent);
- drop TEMPLATE: P-d red;
- drop PATH: P-e red.

**Added in draft 3.**

| mutant | P-a | P-c | P-d |
|---|---|---|---|
| env config appended to `_FGIT_ENV` | red (scope `command`) | ok | ok |
| `_FGIT_VOID` at a non-empty directory | clean | red | ok |

**Reftable:** two `init --ref-format=reftable` runs differ under `diff -r` (`.git/reftable/*.ref`,
`tables.list`).

**Draft 1's roster E7 was dead.** Draft 1 used `init.templateDir` via XDG config. Under the constructed
environment it reported `NOT-LIVE`, because `GIT_TEMPLATE_DIR` outranks it.

### §A.4 Prototype D — the whole-build differential (draft 3; SUPERSEDED)

⚠ **The whole-wire results below are invalid, and draft 4 supersedes them.** The harness that produced
them (`fgit/mut.sh`) ran each mutant in a `cp -R` copy with `rm -rf $d/.git`, so the wire's own real-tree
scan also read nothing. Every run therefore went red for an unrelated reason as well: `read 0 stored
objects` (found by round-3 Ax3).

Only the label lines are evidence. Round 3 then measured that the differential's members were
effect-based and not universal. Draft 4 replaces the differential with the bypass detector, and the
corpus (§A.5) re-derives every whole-wire claim with a harness that keeps `.git`: it runs tagged copies
beside the originals, as the mutation runner does.

**Setup.** A `git clone --local` of `e8f78896`, with four changes:
- draft-3 §3.3;
- the `notcommitted` fix;
- base controls lines 83–733 extracted to a sourced fixtures file (651 lines; controls 346);
- a differential block after `ctl_ok=0`.

It ran on both shells and gave rc 0 and `PASSED`, with 80 repos compared.

**`linkname`.** Before its `ln -s "$CTL/…"` became `ln -s ok.py`, it was the one differing repo, because
it was position-dependent.

**Mutants** (applied to a copy of prototype D):

| mutant | result |
|---|---|
| `command env` | red, all 80 differ |
| call-time `HOME` | red |
| a `cachedir` step as `_git add -A` | red, `cachedir` only |
| `PATH="$CTL/fakegit:$PATH" git add -A` | red, `cachedir` only |
| `/opt/homebrew/bin/git add -A` | red, `cachedir` only |
| `env -i PATH="$PATH" git add -A` | **no** differential red |

**Errexit.** A P reader whose final command failed made the run end with no verdict. That is why every
reader ends in `true`.

**Cost of the P reader.** Four git calls per repo cost clearly more than two git calls plus file reads.
Timings were taken from in-script `Time::HiRes` stamps on a loaded machine. They are too noisy to quote,
so re-measure if needed.

**Driver timing.** `/usr/bin/time -p bash scripts/trip-wires.sh` was run three times each at base and at
prototype D. Head was slower. Both sides were far inside the 5-minute budget, and the spread overlapped
the difference. X8 repeats this on the real head.

### §A.5 The adversarial corpus (draft 4)

**Scripts.** Scratch `…/scratchpad/fgit/corpus/gen.py` and `cell.sh`, preserved at `git show 57e5419f:docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-provenance.md` (§E) so that a reviewer
can re-run them.

**Subject.** Prototype `p4`, a `git clone --local` of `e8f78896` plus exactly draft-4 design memo §3–§5:
- the `_fgit` construction;
- the `notcommitted` fix;
- controls lines 83–734 extracted to a sourced fixtures file (`wc -l` at `e58fec48`: fixtures 652, controls 397, harness 215);
- the bypass detector with its canaries;
- P-a…P-e.

It has three commits:
- `cd431e04`: the prototype;
- `9df1e47e`: a P-b reporting fix;
- `e58fec48`: one label per liveness producer.

**Run.** 780 cells, parallelism 7, from 13:55 to 15:09 JST on 2026-09-27, over `9df1e47e`.
`results.err` is empty. The call sites were chosen by regex over the fixtures file; each is identified
by its line in `p4`'s fixtures file:

| class | line |
|---|---|
| init | 427 (the loop) |
| add -A | 428 |
| add \<path\> | 466 |
| add -f | 457 |
| commit | 492 |
| checkout | 556 |
| hash-object | 515 |
| hash-object -w | 594 |
| hash-object -w --stdin | 500 |
| update-index | 502 |
| replace | 595 |
| rev-parse | 592 |
| symbolic-ref | 578 |

**Results.** From `cut -f1,2,3 results.tsv | sort | uniq -c`:

| config | G | R | P | RES |
|---|---|---|---|---|
| bash 5.3 · git 2.55 | 29 PASS | 130 PASS | 10 PASS | 26 RES-GREEN |
| bash 3.2 · git 2.55 | 29 PASS | 130 PASS | 10 PASS | 26 RES-GREEN |
| bash 5.3 · git 2.54 | 29 PASS | 130 PASS | 9 PASS, 1 FAIL | 26 RES-GREEN |
| bash 3.2 · git 2.54 | 29 PASS | 130 PASS | 9 PASS, 1 FAIL | 26 RES-GREEN |

**The two FAILs** are `P-e drop PATH` (cells c0582 and c0777, both rc 0, no control line). This was
predicted: under git 2.54, `PATH`'s git is `/usr/bin/git`, which is also `env`'s default-path git.

**Re-run at `e58fec48`.** Clean plus all 10 pins × 4 configs gave the same result: clean 4/4, pins 38/40,
and the same two P-e cells failed (`corpus2/results.tsv`).

**Examples from the logs.**
- `init × bare git` (c0030) fails 75 control lines, because the whole loop broke, and the named label is
  present.
- `init × hash -p + git` (c0036) fails exactly 1 control line: only the detector fires, because the
  hashed git works and so only the environment trace catches it.

### §A.6 Cost (draft 4)

**The rule.** From `git show e8f78896:.github/workflows/ci.yml | sed -n '/^  trip-wires:/,/^  [a-z]/p'`:
re-derive the budget with three runs per side and compare. These figures stay here and never go in
`ci.yml`, as that comment itself requires.

**Driver** (`/usr/bin/time -p bash scripts/trip-wires.sh`), three interleaved runs per side:
- base: 24.2, 20.3, 48.9 s;
- p4: 20.9, 57.5, 60.6 s.

**K2 wire alone**, five interleaved runs per side (real / user+sys CPU):
- base: 48.7/31.6, 42.5/26.2, 25.3/16.3, 36.0/25.9, 28.9/20.0;
- p4: 51.7/33.0, 48.8/32.9, 41.0/21.9, 19.3/12.0, 44.0/26.1.

The machine was heavily loaded: an unloaded base run earlier in the session was about 13–15 s. The
spreads overlap in both real and CPU time. p4 builds the fixtures **once**; its additions are two
canaries and the P probes.

⚠ Round 3's Ax3 measured non-overlapping spreads for draft 3's **two-build** prototype: base 22.2 /
19.7 / 16.3 s against head 30.8 / 26.9 / 26.8 s. That is consistent with the second build having gone.
X8 repeats the derivation on the real head.

### §A.7 The ratchet on p4

The records here are p4's `_mutants` here-document. At base, extract it with
`sed -n '/^_mutants() { cat <<.MUTANTS./,/^MUTANTS$/p'` (97 lines).

**Population:** `_control` labels plus every `_lbl="…"` definition in p4's controls file. Of those,
**30** have no record:
- the base's 21;
- the 9 new labels.

The 3 existing `_lbl` labels each have 1 record.

### §A.8 The draft-5 corpus (p5; the representative subset)

**Subject.** Prototype `p5`, a `git clone --local` of `p4`. Its commits:
- `6d2af1ff`: the env shim; the config-trace2 and poison channels; one label per producer;
  producers moved into the harness; state initialised before any read;
- `25def5a5`: the build sets `GIT_TRACE2_EVENT`;
- `9c220d30`: label wording;
- `d130ac10`: `GIT_TRACE` dropped, and the trace2 marker reset between canaries;
- `067c0c40`: each trace2 canary proves its own route.

`wc -l` over the parts at `067c0c40`: controls 366, fixtures 652, harness 306, mutations 783 (untouched in
the prototype), wire 1259 (untouched).

**Commands.** Scripts are preserved at `git show e8c1bdb8:docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-provenance.md` (§E.2); paths are relative to `…/scratchpad/fgit/`:
- `python3 corpus5/gen5.py .` — 238 jobs;
- `tr '\n' '\0' < corpus5/jobs.tsv | xargs -0 -n1 -P2 corpus5/cell.sh > corpus5/results.tsv`.

The load limit was at most 2 concurrent wire runs.

**Run 1** (aborted after 12 cells, `corpus5/results.run1-aborted.tsv`). G cell `caller GIT_TRACE2_EVENT=<abs file>`
(d0008, bash 5.3 · git 2.55) went NE on the config-route canary. A caller's `GIT_TRACE2_EVENT`
overrides `trace2.eventTarget`. Fix: the build exports its own `GIT_TRACE2_EVENT` (`25def5a5`).

**Run 2** (aborted after 60 cells, `corpus5/results.run2-aborted.tsv`). Two failures:
- R cells d0031–d0033 (`hash -p`, `eval "$_REAL_GIT"`, absolute path) went red **only** on the trace2
  label. The poison kills a config-reading git before `GIT_TRACE` writes its `built-in` line, so
  `GIT_TRACE` had no cell of its own. It was dropped (`d130ac10`).
- P cell d0048 (no global config) stayed green. The env-route canary had already written the shared
  trace2 file, so the config-route canary saw it. Fix: reset the file between canaries, and have each
  canary unset the other route (`d130ac10`, `067c0c40`).

**Run 3** (complete, 16:20–17:21 JST on 2026-09-27, `corpus5/results.tsv`, `results.err` empty),
from `cut -f1,2,3 results.tsv | sort | uniq -c`:

| config | G | R | P | RES | INFO | BEFORE (p4) |
|---|---|---|---|---|---|---|
| b53-g255 | 8 PASS | 33 PASS | 14 PASS | 2 RES-GREEN | PASS (red) | 3 FAIL, as expected |
| b32-g255 | 8 PASS | 33 PASS | 14 PASS | 2 RES-GREEN | PASS (red) | — |
| b53-g254 | 8 PASS | 33 PASS | 14 PASS | 2 RES-GREEN | FAIL (equivalent) | — |
| b32-g254 | 8 PASS | 33 PASS | 14 PASS | 2 RES-GREEN | FAIL (equivalent) | 3 FAIL, as expected |

**BEFORE, on p4 `e58fec48`** (they show the defects that U2/U3/U5 fix):

| cell | result |
|---|---|
| d0233, setup fails, b53-g255 | rc 1, no label |
| d0236, setup fails, b32-g254 | **rc 0**, no label |
| d0234 and d0237, `GIT_TRACE= <abs>` + caller HOME `*.py` ignore | rc 0, PASSED |
| d0235 and d0238, `env -i PATH=/usr/bin:/bin git` + caller HOME `*.py` ignore | rc 0, PASSED |

**Cost.** K2 wire alone, three interleaved runs per side, taken after run 3 while the machine's load
average was 24–66 (real / user+sys CPU):
- base: 26.8/20.2, 25.1/16.2, 24.6/15.1;
- p5: 24.1/15.9, 21.9/15.9, 22.8/16.9.

The spreads overlap. This is **not** X8, which re-derives on the implementing head under the ci.yml
rule.

### §A.9 The draft-6 corpus (p6: the window built from nothing)

**Subject.** Prototype `p6`, a `git clone --local` of `p5` taken at `067c0c40`:
- `50f9e31a`: the window, detector deleted, postconditions inside the window, P-f, and the corpus P-dump
  hook, which is prototype-only;
- `863158b1`: the prelude passes plain assignments.

`wc -l` at `863158b1`: controls 351, fixtures 652, harness 300, mutations 783 (untouched in the
prototype), wire 1259 (untouched).

**P-f caught its own prototype.** On the first run, the prelude used `declare -p`, which carried the
exported `K2_CORPUS_PDUMP` into the window as an exported name. P-f reported it on both shells. The fix
was plain `%q` assignments. So the complement check did its job on day one.

**Position independence.** Two clean runs per shell produced byte-identical P dumps (80 repos): diff
the `c6/pd.1` and `c6/pd.2` dumps. One symlink fixture (`linkname`) stores `$CTL` in its target. The
dump normalises symlink targets; the fixture is unchanged.

⚠ **P depends on configuration by design.** A fixture repo holds a shim whose content embeds the real
git path (`$_REAL_GIT`), and that path differs between git 2.55 (`/opt/homebrew/bin/git`) and Apple
2.54 (`/usr/bin/git`). So P is compared **per configuration**.

⚠ **My first evaluation got this wrong.** `eval6.sh` looked up the reference without matching the
configuration, so every git-2.54 cell was compared against git 2.55's reference and showed a uniform
6-line "difference". That was a harness bug, not a finding. It was fixed (the lookup now matches `$2`)
and re-evaluated from the same `raw.tsv`, without re-running anything.

**Commands.** Scripts: `git show ff1322bb:<this file>` §E.3. From `…/scratchpad/fgit/`:
- `python3 c6/gen6.py .` (190 cells);
- `tr '\n' '\0' < c6/jobs.tsv | xargs -0 -n1 -P2 c6/cell6.sh > c6/raw.tsv`;
- `c6/eval6.sh > c6/results.tsv`.

The run covered 17:49 to 18:27 JST on 2026-09-27. `raw.err` is empty.

**Results.** From `cut -f1,2,3 c6/results.tsv | sort | uniq -c`:

| config | REF | G | R | RLOUD | RES | P | INFO |
|---|---|---|---|---|---|---|---|
| b53-g255 | PASS | 11 PASS | 27 PASS | PASS | 2 PASS | 9 PASS | 2 PASS |
| b32-g255 | PASS | 11 PASS | 11 PASS | PASS | 2 PASS | 9 PASS | 2 PASS |
| b53-g254 | PASS | 11 PASS | 11 PASS | PASS | 2 PASS | 9 PASS | 1 PASS, 1 FAIL (PATH drop: equivalent, e0127) |
| b32-g254 | PASS | 11 PASS | 27 PASS | PASS | 2 PASS | 9 PASS | 1 PASS, 1 FAIL (PATH drop: equivalent, e0180) |

**BEFORE, on `p5d`.** `p5d` is p5 `067c0c40` plus the same P-dump hook, `22ea58f1`. The cells are
e0182–e0185 (b53-g255) and e0187–e0190 (b32-g254). For `scrub3-sh`, `hermetic-look`, `gitsweep` and
`t2zero-countempty` under a caller HOME `*.py` ignore, all eight were **SILENT-WRONG**: rc 0 and
`PASSED`, yet P differed from p5's clean build by 50 dump lines.

**AFTER, on p6.** The same spellings under the same caller are PASS with P identical, on all four
configurations. Under the other eight hostile callers, `gitsweep` and `scrub3-absenv` are PASS with P
identical on both FULL configurations.

**Residual evidence (measured, safe).** The following facts back up the residual:
- `env -i PATH=… git config --list --show-origin --show-scope` shows
  `system	file:/opt/homebrew/etc/gitconfig	credential.helper=osxkeychain`;
- that file is owned by `kazuaki admin` and its directory is `kazuaki drwxrwxr-x`;
- the Homebrew template directory is owned by `kazuaki`;
- `env -i … git init` copies `description`, `hooks/` and `info/`.

A git that discards the allowlist therefore reads caller-writable files. Nothing was planted there.

**Cost.** K2 wire alone, three interleaved runs per side, load average about 4–6 (real / CPU):
- base: 12.19/8.98, 12.23/8.95, 12.32/8.99;
- p6: 12.26/8.82, 12.06/8.64, 12.16/8.72.

This is not X8.

### §A.10 Draft 7's focused cells (p7: round-6 items 1 and 2)

**Subject.** Prototype `p7`, a `git clone --local` of `p6` `863158b1`.

| commit | change |
|---|---|
| `6fc364f2` | an incomplete window ends the run with W alone, carrying the child's exit class; the prelude options are pinned; P-f parses every `env -0` record; seed S |
| `bf5879b6` | the options check reads `$SHELLOPTS`. The first form, `$(set -o pipefail)`, **sets** the option instead of reading it, so every run exited 3. Measured on the first run |
| `183d706e` | S's loop moved out of a command substitution. bash 3.2 misparses a `case` inside `$( )`: the first run printed "syntax error near unexpected token `newline'" and then aborted on an unbound variable |
| `d7738624` | the P-a label is scoped to "a window git that adds no input of its own" |

**Commands.** Scripts: `git show ff1322bb:<this file>` §E.4. From `…/scratchpad/fgit/`:
- `python3 c7/gen7.py .` (32 cells);
- `tr '\n' '\0' < c7/jobs.tsv | xargs -0 -n1 -P2 c7/cell7.sh > c7/raw.tsv`;
- `c7/eval7.sh > c7/results.tsv`.

`raw.err` is empty. The P-a label check ran in `c7b/`: records g00 and g10 each gave rc 1 with the new
label, and the clean cells g01 and g11 each gave rc 0.

**Results** (b53-g255 and b32-g254 alike):

| cell | p6 (before) | p7 (after) |
|---|---|---|
| `odd/pipe` pre-created (the fixtures `exit 2`) | f0002 and f0007: rc 1, **82** control lines | f0012 and f0023: **rc 2, 1 line (W)** |
| `GIT_CONFIG_GLOBAL=<file> git add -A` | f0003 and f0008: rc 0, PASSED, Pdiff 50: **SILENT-WRONG** | f0013 and f0024: rc 1, S alone |
| `git -c include.path=<file> add -A` | f0004 and f0009: **SILENT-WRONG** (Pdiff 50) | f0014 and f0025: rc 1, S |
| `GIT_TEMPLATE_DIR=<dir> git init` (in the loop) | f0005 and f0010: rc 1, loud (NE) | f0015 and f0026: rc 1, S plus NE |
| W: child aborts after the prelude | — | f0019 and f0030: rc 1, W alone |
| W: prelude without `set -euo pipefail` | — | f0020 and f0031: rc 1, W alone |
| P-f: `-i` dropped, plus a caller `BASH_FUNC_k2probe%%` | — | f0021 and f0032: P-f names `BASH_FUNC_k2probe%%` |
| G: HOME `*.py` ignore; DO cell | — | PASS, Pdiff 0 |
| R: `gitsweep` × HOME ignore | (p6 corpus: PASS, P equal) | f0018 and f0029: rc 1, S (the spelling contains `unset`/`export`/`exec`), Pdiff 0 |

**The parser difference.** Under a shell that exports a function:
- p6's `env | sed 's/^\([A-Za-z_][A-Za-z0-9_]*\)=.*/…/'` finds **0** `BASH_FUNC` names;
- `env -0 | tr '\0' '\n' | /usr/bin/grep -c '^BASH_FUNC_f%%'` finds **1**.

**Moved exit-criterion commands, re-run to show they discriminate.**

| command | results |
|---|---|
| X1 | the green log c7/f0011 gives `0` / `1`; the red log c7/f0013 gives `1` / `0` |
| X4b, new form | 2 lines on base `mutations.sh` (15, 20); 0 on a copy where both are qualified |
| X4b, old in-table form (raw `\|`, no `-i`) | **0** on base: it was vacuous |
| R7 | `runs-on: ubuntu-latest` for `trip-wires`; `runs-on: ${{ matrix.os }}` for `check` |

### §A.11 Draft 8's focused cells (p8: D1–D4)

**Subject.** Prototype `p8`, a `git clone --local` of `p7` `d7738624`:

| commit | change |
|---|---|
| `42b3d85f` | S deleted; P-g added (every fixture repo's configuration must equal a reference init's, compared line by line); `built` marker written by the fixtures file's last line; an incomplete window exits 2 with a reason (parent-side failures get their own text); W2 check |
| `563c857a` | the P-a label uses the §5.1 unit |

**Commands.** From `…/scratchpad/fgit/`:
- `python3 c8/gen8.py .` generates 40 cells;
- `tr '\n' '\0' < c8/jobs.tsv | xargs -0 -n1 -P2 c8/cell8.sh > c8/raw.tsv`;
- `c8/eval8.sh > c8/results.tsv`.

`raw.err` is empty. The P-a label cells in `c8b/` behave as expected: records g00/g10 go red with the
new label (rc 1), and the clean cells g01/g11 give rc 0.

**Results** (b53-g255 cells listed first, then b32-g254):

| cell | p7 (before) | p8 (after) |
|---|---|---|
| `printf '[include]\n\tpath = <file>' >> .git/config` (cachedir chain) | h0002, h0008: SILENT-WRONG (rc 0, Pdiff 1) | h0014, h0028: rc 1, P-g |
| `printf '[core]\n\texcludesFile = <file>' >> .git/config` | h0003, h0009: SILENT-WRONG | h0015, h0029: rc 1, P-g |
| `git -C . config core.excludesFile <file>` | h0004, h0010: SILENT-WRONG | h0016, h0030: rc 1, P-g |
| `git config include.path <file>` | h0005, h0011: red via S only | h0017, h0031: rc 1, P-g |
| top-level `return 0` after the fixtures file's first line | h0006, h0012: rc 1, **81** control lines | h0018, h0032: **rc 2, W alone** |
| `odd/pipe` pre-created | — | h0022, h0036: rc 2, W alone |
| W2 record (incomplete-window `exit` removed, plus `odd/pipe`) | — | h0023, h0037: rc 1, W2 present |
| prelude drops nounset / pipefail / errexit | — | h0024–26, h0038–40: rc 2, W alone each |
| transient `git -C . -c include.path=<file> add -A` (declared residual) | — | h0019, h0033: rc 0, PASSED, Pdiff 50: **silent**, as declared |
| G: HOME `*.py` ignore; DO cell | — | PASS, Pdiff 0 |

**Census.** On a clean p8 run, every fixture repo's configuration equals the reference init's: P-g
stays green on both shells.

**Cost of P-g.** Eighty `git config --list --show-scope --show-origin` calls under `env -i` took 0.6 s
total, standalone, at load about 2. The whole p8 wire took 14.6 s on bash 5.3 and 12.3 s on bash 3.2.
This is **not** X8.

⚠ **These cells ran under the real `HOME`** (round 8, Ax2). `c8/cell8.sh` passes `HOME` only when a
cell's own environment names it, so every other cell inherited the caller's. The fixture build was not
exposed, because the window sets its own `HOME`, but the controls' reads were. **Re-run with `HOME`
set** (§A.12's `c9/cell9.sh`, into `c8h/`): all 40 cells gave the same verdict, rc and Pdiff as the
table above. `raw.err` was empty, and the cell `HOME` stayed empty.

### §A.12 Draft 9's focused cells (p10: E2, E3, AR, SE1)

**Subjects.** `p9` (`4ca92c38`) and `p10` (`ba68371b`), each a `git clone --local` of its predecessor:

| commit | change |
|---|---|
| `4ca92c38` | `_control` calls `_fw_built_or_w2` first (W2 wherever the exit sits); W3 from the child's stderr; options re-checked after the fixtures file (exit 5); `_fw_why` names 3/4/5; the two comment fixes; a top-level P-g shape check |
| `ba68371b` | P-g's census: one `find` over `$CTL` for every `.git` of any type, every `HEAD`+`objects` dir and every unsearchable dir; the three deliberate unsearchable dirs declared; census failure or empty population red |

Clean runs of p10 with `HOME` set to an empty scratch dir: rc 0 and `PASSED` on bash 5.3 and on bash
3.2. The first p10 census run was red on the clean tree: `find` exited non-zero on the fixtures'
deliberately unsearchable `walk/sub` and `d5root`. That is where the declared list came from. With the
list emptied, exactly `d2red/sub`, `walk/sub` and `d5root` go red, so each entry is necessary.

**The cell harness now sets `HOME`.** `c9/cell9.sh` is `cell8.sh` plus `HOME="$OUT/home"`, placed
before the cell's own environment, so a G cell's `HOME=` still wins. After each run, `ls -A c9/home`
is empty.

**Commands.** From `…/scratchpad/fgit/`:
- `python3 c9/gen9.py "$PWD"` generates 44 cells (the path must be absolute: the hostile files are
  named by it, and a relative one breaks every cell that names one);
- `tr '\n' '\0' < c9/jobs.tsv | xargs -0 -n1 -P2 c9/cell9.sh > c9/raw.tsv`;
- `c9/eval9.sh > c9/results.tsv`.

`raw.err` is empty. All 44 cells gave the same verdict on both configurations; the tags are listed
b53-g255 first, then b32-g254.

| cell | p8 (before) | p10 (after) |
|---|---|---|
| clean | k0001, k0010: PASS | k0019, k0032: PASS |
| `cachedir` `--separate-git-dir` + persisted `core.excludesFile` | k0002, k0011: SILENT-WRONG (rc 0, Pdiff 3) | k0020, k0033: rc 1, P-g `cachedir:[.git is not a directory] .gd-cachedir:[a git dir not named .git]` |
| `cachedir` `--separate-git-dir` only | k0003, k0012: SILENT-WRONG (rc 0, Pdiff 2) | k0021, k0034: rc 1, P-g, same entries |
| nested `zz/inner`, persisted key | k0004, k0013: GREEN-SAME | k0022, k0035: rc 1, P-g `zz/inner:[local file:.git/config core.excludesfile=…]` |
| hidden `.hid`, persisted key | k0005, k0014: GREEN-SAME | k0023, k0036: rc 1, P-g |
| bare `zbare`, persisted key | k0006, k0015: GREEN-SAME | k0024, k0037: rc 1, P-g `zbare:[a git dir not named .git]` |
| RO: exit below the first control + `odd/pipe` | k0007, k0016: rc 2; `green is reachable` failed over the unbuilt tree, then W; no W2 | k0025, k0038: rc 2; W2, then W |
| ROg: exit below the first control | k0008, k0017: GREEN-SAME | k0026, k0039: PASS (green, P equal) |
| AR: `_ar=$(( 1/0 ))` after line 1 | k0009, k0018: GREEN-SAME (rc 0, PASSED) | k0027, k0040: rc 1, W3 `…fixtures.sh: line 2: 1/0 : division by 0 …` |
| SE1: `set +e` before the `built` line | — | k0028, k0041: rc 2, W alone, cause 5 |
| G: HOME `*.py` ignore; DO cell | — | k0029/30, k0042/43: PASS, Pdiff 0 |
| W2 record | — | k0031, k0044: rc 1, W2 |

The nested, hidden and bare cells change no fixture's P (Pdiff 0), because each is a new repo that no
control reads. They are red because P-g's population is every git dir, not because P moved.

**p9, the intermediate, missed two shapes.** The same generator pointed at p9 (`c9/p9x/`, bash 5.3 only)
gave: `--separate-git-dir` ×2 and bare, red with P-g; **nested and hidden, GREEN-SAME**. p9 walked
`$CTL/*/`, which skips hidden names and does not descend. p10 replaces that walk.

**A first E2 run that did not isolate P-g** (`c9/run1/`). `--separate-git-dir` on **every** loop-built
fixture was already red on p8 (rc 1, ctl 10 or 2). Fixtures that write `.git/…` directly failed to
build ("its fixture did not build"), and one such write left a shell diagnostic. So the collateral, not
P-g, was red. The single-fixture `cachedir` cells above replace it, and they are silent on p8.

### §A.13 Draft 10's cells (p11: round 9's A1–A3)

**Subject.** `p11`, a `git clone --local` of `p10`, branch `p11`:

| commit | change |
|---|---|
| `e422e2f2` | W3 by `grep -m 3` (rc 1 = none, other rc red); the census opens the declared dirs at their declared mode and restores it; git-dir shape `HEAD` + `objects`/`commondir`; no per-directory fork, `.git` not pruned; any `find` report fails the census; the W cause through a marker file |
| `d7e7c826` | one census-failure message instead of two |

The first p11 run was red on the clean tree: the mode check used `find <dir> -prune -perm 000`, and BSD
`find` reports "Permission denied" on an unreadable **starting** directory even with `-prune`, so all
three declared directories read as off-mode. (Run standalone from a relative path, the same `find`
matched; it was not investigated further.) The check now reads `ls -ld`. Clean runs of `d7e7c826` with
`HOME` set to an empty scratch dir: rc 0 and `PASSED` on bash 5.3 and on bash 3.2.

**Commands.** From `…/scratchpad/fgit/`:
- `python3 c11/gen11.py "$PWD"` generates 56 cells;
- `tr '\n' '\0' < c11/jobs.tsv | K2_CORPUS_OUT=$PWD/c11 xargs -0 -n1 -P2 c11/cell9.sh > c11/raw.tsv`;
- `K2_CORPUS_OUT=$PWD/c11 c11/eval11.sh > c11/results.tsv` (`eval11.sh` = `c9/eval9.sh`).

The cells ran on `e422e2f2`. `d7e7c826` changes only the census-failure text; its clean runs are above.
`raw.err` is empty, and the cell `HOME` stayed empty. Tags are b53-g255, then b32-g254:

| cell | p10 (before) | p11 (after) |
|---|---|---|
| arnoise (AR + 200 `[` diagnostics) | m0002, m0009: GREEN-SAME | m0020, m0027: rc 1, W3 |
| noise1k (1000 `[` diagnostics) | m0003, m0010: GREEN-SAME | m0021, m0028: rc 1, W3 |
| ar (AR alone) | m0004, m0011: rc 1, W3 | m0022, m0029: rc 1, W3 |
| rc5 (`sh -c 'exit 5'`) | m0005, m0012: rc 2, W "switched off errexit, nounset or pipefail" | m0023, m0030: rc 2, W "the window exited 5 before completing" |
| nr (`chmod 300` dir) | m0006, m0013: rc 1, P-g (census) | m0024, m0031: rc 1, P-g "census … failed: find: walk/nr: Permission denied" |
| wtmeta | m0007, m0014: SILENT-WRONG (Pdiff 5) | m0025, m0032: rc 1, P-g `zzw/.git/worktrees/zzw2:[a git dir not named .git]` |
| m2h, b53-g255 / b32-g254 | m0015, m0016: SILENT-WRONG (Pdiff 2) | m0033, m0034: rc 1, P-g `walk/sub/inner:[… core.excludesfile=…]` |
| m2h, b53-g254 / b32-g255 | m0017, m0018: rc 0, PASSED (no reference cell for these configs, so P is not compared) | m0035, m0036: rc 1, P-g |
| draft 9's set on p11 | — | m0037–m0056: all PASS (`--separate-git-dir`, nested, hidden, bare, RO, ROg, SE1, G, DO, W2 record) |

m2h is Ax3's cell (`…/scratchpad/r9ax3/sn/m2h-py`), with its index dump redirected to `/dev/null`.
arnoise, noise1k, ar and rc5 are Ax2's (`…/scratchpad/r9ax2/ed_*.py`).

**Census cost** (MIN-3). Instrumented copies print the census section's wall time, and the clean
trees were used. On p10: 1.447 s (bash 5.3) and 1.478 s (bash 3.2). On p11: 0.760 s and 0.773 s. The
copies' runs did not print `PASSED`, which was not investigated: the figure is the census section
only. Ax3 measured a fork-free `find` alone at 0.029 s; p11's remaining time is the per-git-dir `git
config --list`.


### §A.14 Implementation (C0b–C5) — history and deviations

**Subjects.** Commits as in `…-reviews.md` §13. Verification ran on fresh `git clone --local` copies at
each commit under `…/scratchpad/impl/` (`x1.sh`, `x1c.sh`, `x3.sh`, `rectest.sh`, `xmisc.sh`, `x8.sh`,
`norm.sh`; X5 in `xh/`, a clone of the head carrying the prototype's P-dump hook in a local commit
`7c07415c` that is not on the branch).

**Timing.** Base X3 took 2463 s (bash 5.3) and 2467 s (bash 3.2); C2 2433 s; C3 2972 s; C4 3163 s;
C5 3550 s. The C3/C4 runs overlapped other work on the machine; they are not a cost measurement.

**Deviations from the memo, in the order found:**
1. **W2's record is one `harness:` expression with two substitutions**, not "harness + fixtures". A
   record's sed expression edits one file. It removes the `exit` and renames the `done` marker so the
   window stays incomplete. It pins the same property: W2 is reported when the exit is gone.
2. **X6's producers were misnamed.** A template planted in the void reds through P-c, not P-d: P-d
   diffs `init` against `init --template="$_FGIT_VOID"`, and with `GIT_TEMPLATE_DIR` already the void
   both sides copy the plant. The local `include.path` reds through P-g, not P-a, whose probe repo is
   not a fixture.
3. **C4 also rewrites the harness's `_shq` comment**, which said "the mutation set (which edits the
   wire) has nothing to aim at". C4's `harness:` target makes that false, so C4 corrects it; C5
   carries the same text.
4. **C5 was recommitted once with `git reset --soft`** (not amend). The first C5, `d7812ffe`, had
   changed the harness's mode 100755 → 100644 (a `shutil.copy` from a 644 scratch file). The final
   `386febc8` restores the mode and adds X8's `ci.yml` paragraph; the stat is in memo §13. X3, the
   per-record runs and X5 are on `d7812ffe`; X1 is on `386febc8`. The harness is sourced, so the mode
   changes no behaviour.
5. **X3 was not run at C1 alone.** C1's sibling guard and parts list run inside every later X3.
6. **The X5 cell generator needed `mutgen` in its copied parts.** The first run gave eight false FAILs
   ("missing … mutgen.sh"); rerun: 38/38 PASS.
7. **The first per-record queue was invalid**: both shells shared one clone directory. It was killed
   and rerun with per-shell directories: 40/40.

**Independent check by the orchestrating session** (reported to this author): on `386febc8`, both
shells, scratch `HOME`, seven caller conditions gave rc 0 and PASSED — clean, the original Codex P2
(`GIT_TEMPLATE_DIR` with `info/exclude *.py`), a `.gitconfig` `excludesFile`, a home ignore, an XDG
ignore, XDG/home attributes UTF-16LE, and `GIT_CONFIG_GLOBAL=/dev/null`. On base `e8f78896` the P2
condition gives rc 1 / NE 11 on both shells.


**The `/simplify` pass — verification** (`…-reviews.md` §13; `…/scratchpad/impl/`, both shells,
scratch `HOME`, at most two wire runs at once; run on the pass's content before it was folded into
one commit, whose code is byte-identical):

```text
X1       rc=0 CTL=0 PASSED=1 (bash 5.3, bash 3.2)
X3       "114 entr(ies), 0 not killed as named", "50 mutant(s) … 0 neither killed nor argued
         equivalent", PASSED, 0 `!!` lines — both shells (4508 s / 4509 s)
X11      _x_lbl added: rc 1, "22 labels have no mutation record, against a ratchet of 21", lists it
callers  cc.sh, 7 conditions × 2 shells: clean; GIT_TEMPLATE_DIR with info/exclude *.py (the P2);
         .gitconfig core.excludesFile; home ignore; XDG ignore; home attributes UTF-16LE;
         GIT_CONFIG_GLOBAL=/dev/null — every run rc=0 NE=0 CF=0 PASSED=1
negative base e8f78896 with the P2 condition: rc=1 NE=11, both shells
anchors  every record's expression, applied to its target: changes 1–2 lines, none matches nothing
bash -n  every part, both shells: clean; parts 383/683/379/645/328 lines (controls/fixtures/
         harness/mutations/mutgen), the wire 1259 untouched
```


**The `/code-review` pass — verification** (`…-reviews.md` §13; `…/scratchpad/impl/`, scratch `HOME`,
at most two wire runs at once; run on the fix's content before it was folded into one commit, whose
code is byte-identical). b53 = `/opt/homebrew/bin/bash`; b32 = `/bin/bash` with `PATH=/bin:/usr/bin:…`,
so `bash` is 3.2 too.

```text
X1        b53, b32: rc=0 CTL=0 PASSED=1
X3        b53 (2942 s), b32 (3818 s): "116 entr(ies), 0 not killed as named", "50 mutant(s) … 0
          neither killed nor argued equivalent", PASSED, 0 `!!` lines
probe12   /bin/bash driver, mutation run cut to 2 records, each process logging $BASH_VERSION:
          172 mutant processes and 86 wire/control processes, every one 3.2.57(1)-release
X11       b53, b32: rc 1, "22 labels have no mutation record, against a ratchet of 21", lists it
callers   7 conditions (clean; GIT_TEMPLATE_DIR info/exclude *.py; .gitconfig excludesFile; home
          ignore; XDG ignore; home attributes UTF-16LE; GIT_CONFIG_GLOBAL=/dev/null) × b53, b32:
          every run rc=0 NE=0 CF=0 PASSED=1
tripwires bash scripts/trip-wires.sh from the repo root, b53 and b32: rc=0, generic-core PASSED
records   the new or changed records (P-b-live, P-e, P-h, W4), each alone, b53 and b32: 8/8 killed
          with their needle
```

Cells (`crc.sh`; "before" = the `/simplify` head `db2b0af1`, "after" = the fix), both shells unless
noted. Every after-state is the intended one:

| # | cell | before | after |
|---|---|---|---|
| 1 | `cwdrepo`: caller cwd in another repo with a local `core.attributesFile` and a bad include | rc 1, P-b-live NE | rc 0, PASSED |
| 2 | `gitvar129`: a `git` shim answering 129 for the four `git var` names | rc 1, NE | rc 0, PASSED, P-b "NOT EXERCISED on this machine" |
| 3 | `pe_opt` / `pe_dev` / `pe_alias`: `GIT_EXEC_PATH` opt spelling, `DEVELOPER_DIR` with `/usr/bin/git`, an exported `GIT_EXEC_PATH` | `pe_dev` red (Xcode vs CommandLineTools); the other two green here | all green |
| 4 | `sealfail`: a `chmod` shim failing only `*/err/control.py` | PASSED, false "NOT EXERCISED on this machine" | rc 1, W4 |
| 5 | `w3cmdsub`: `: "$(case a in a) echo y;; esac)"` (b32) | green | rc 1, W3 (b53: no diagnostic exists, green) |
| 5 | `w3ar`: `$(( 1/0 ))` | red | red |
| 6 | `env0`: an `env` shim without `-0` | green | rc 1, P-f NE — **superseded** at `1fe79a26`: an `env` that runs a command but refuses `-0` is a machine limitation (green, `⚠ NOT EXERCISED on this machine: P-f`); see `…-reviews.md` §13, PR #527 |
| 7 | `garbagehead`: persisted `core.excludesFile` plus a garbage `HEAD` | green | rc 1, P-g "git lists no configuration here" |
| 8 | `nltmp`: a `mktemp` shim returning a scratch path holding a newline | a directory OUTSIDE the scratch root chmodded to `d---------` (and P-b red) | the outside directory unchanged; rc 0, PASSED |
| 9 | `longopts` / `tzutc`: `set -o errexit -o nounset -o pipefail`; `"TZ=UTC"` appended | green | rc 1, "no longer matches … its anchor is stale" |
| 10 | `nomutgen`: `mutgen.sh` removed | — | rc 2, "…is missing or unreadable … decided nothing" |
| 11 | `w2rec` (traced): the W2 record's edit | a non-`_control` block executed over the unbuilt tree | none executed; W and W2 |
| 13 | `mutpath`: a clone at `…/wt.mutant.k2/sb` | green (guard was opt-in) | green (guard always on) |
| 15 | `lblrename`: `_pa_lbl` renamed | green, silently | rc 2, W "…label $_pa_lbl is not defined, or is empty" |

#14's cell: `bash …fixtures.sh` run directly prints the entry-guard refusal and exits 2. #12's is
probe12 above. The first `nltmp` run of the fix showed a false P-b red, the void path split at the
newline; fixed before this verification by replacing the void with a token before splitting lines.
Not taken, as decided: P-g's per-repo `git config` forks and `_mut_correspondence`'s greps.


**The `/elidex-review` pass — verification** (code at `4357b963`, folded unchanged into the fix
commit; scratch `HOME`; at most two wire runs at once; b53/b32 as above).

```text
X1/clean   b53, b32: rc=0 PASSED; the 7 caller conditions × b53, b32: every run rc=0 NE=0 CF=0 PASSED=1
X11        b53, b32: rc 1, "22 labels have no mutation record, against a ratchet of 21", lists it
P-i record alone (drop GIT_DEFAULT_REF_FORMAT=files): b53, b32 killed with its needle
tripwires  bash scripts/trip-wires.sh from the repo root, b53 and b32: rc=0, generic-core PASSED
X8         /usr/bin/time -p bash scripts/trip-wires.sh, alternated, base e8f78896 vs 4357b963:
           base 17.64 17.13 17.12 s, head 18.91 19.83 18.19 s; the 5-min budget is unchanged
nounset    NOU=1 (the wire's `set -euo pipefail` → `set -eo pipefail`) vs normal, b53, at 4357b963:
           clean, sealfail, env0, garbagehead, w2rec, lblrename, w3ar, sealdotdot, reftable — every pair
           has the same rc, NE/CF counts and verdict lines
```

| cell | before (`44ae6c57`) | after (`4357b963`) |
|---|---|---|
| `reftable`: a `git` that picks reftable unless the caller pins a format | rc 1: P-d, W3 (the fixtures' ref writes), `badref` NOT EXERCISED | rc 0, PASSED (b53, b32) |
| `sealdotdot`: `_seal "$CTL/walk/../../../k2-sentinel"` | rc 1, W4 "chmod … failed" | rc 1, W4 "has a . or .. or empty component" (b53, b32) |
| `sealsymlink`: a symlink under `walk` pointing out of the scratch root, then sealed | rc 1, W4 "chmod … failed" | rc 1, W4 "passes through a symlink" (b53, b32) |

⚠ **What the seal cells did not show.** Both "before" runs failed their `chmod` instead of changing
anything outside the scratch root: in this sandbox `mktemp -d` ignores `TMPDIR` (measured:
`TMPDIR=<dir> mktemp -d` returned a `/var/folders/…` path), so the cells' sentinel path did not exist.
The escape the review describes is therefore reasoned, not reproduced here; what the cells show is that
the fix refuses both paths before any `chmod`, on both shells, with the sentinel unchanged.
The one-failure-reported-once change (`_seal_apply` skipping a fixture already failed, `d2red`'s seal
inside its chain) has no separate cell.

---

## §B Fate of `ff6b99a3`'s 14 commits (moved from draft 2 §9.1)

| commit | subject | fate |
|---|---|---|
| `c8f52724` | scrub the whole `GIT_*` environment | mechanism superseded; target plumbing → C4 |
| `4ad051ea` | one target list and a resolver | → C4 |
| `c5820bd5` | comment syntax for records | → C3 |
| `a1007bfb` | keep-set grounded in config layers | superseded (premise false: a template seeds the local layer) |
| `162edf98` | prove the scrub against `env` | superseded |
| `328dc22a` | measure the disjointness | superseded |
| `c39c7989` | bound the unknown-prefix claim | → C4 |
| `f8c74c6d` | two comments | harness half superseded; mutations half → C4 |
| `6b4064fa` | `env -0` per record | superseded |
| `009e93db` | liveness before asserting | principle survives in draft-3 §6.3; code superseded (it leaked `HOME`) |
| `4aced879` | restore/resolve order | → C4 |
| `2dea82ff` | retire the hostile-build count | superseded |
| `6ffe2d5d` | one spelling for pattern and target | principle survives |
| `ff6b99a3` | order-measurement note | → C4 |

**Review coverage for the code carried forward.** `ff6b99a3`'s Stage 5 reported 15 IMP / 8 MIN. The
lane SSoT does **not** list the 15 individually. What it records (`project_citation-hygiene-program.md`,
the block headed "(superseded 2026-09-27 後続) ▶▶▶ NEXT SESSION STARTS HERE (2026-09-27)", sub-heading "🔴 併発する 3 件") is the XDG hole plus three co-occurring items, all
measured:
- no control pinned the keep-set;
- the 1000-line split was not taken;
- two carve slots had false premises.

⚠ Draft 2 said "not itemised", which was too strong. All four recorded items are addressed:
- the XDG hole, by draft-3 §3;
- the pins, by draft-3 §6.2;
- the split, by draft-3 §8;
- the slots, by draft-3 §7.

The remaining, unrecorded findings cannot be recovered. C3/C4 are therefore reviewed fresh by the
rebuild's `/pre-push` Stage 4 over the whole range.

---

## §C Premises found false

**From the original brief (draft 1):**
1. "The paths `git add -A` stages" is too narrow.
2. "Nulling global config opens the channel" overstates it: base is already red.
3. The XDG "12" is 11 NE + 1 CF.
4. `_envlive`'s 5 lines call git through `env`.
5. `ff6b99a3`'s claim that the local layer "belongs to the repo `_fgit` creates" is false.
6. The `env -i` shape changed four ways (draft-3 §3.2).
7. Base `_fgit` already nulled the global and system layers.

**From round 1 (draft 2):**

8. F1 was true, and refined: origin checks cannot see template-copied config.
9. F10 was true.
10. Draft 1's roster E7 was dead.
11. F7's "shadow `_git`" was unnecessary.
12. Three misattributions: draft-2 §8.1's rule, the "§10.x" row (§10.6 R7), and the man-page section.

**From round 2 (draft 3):**

13. Draft 2's claim that "no in-process mechanism intercepts an absolute exec" was false. A whole-build
    output differential catches it (§A.4).
14. Draft 2's `PATH`-shim guard missed eight non-absolute spellings. That was measured by Ax3, and the
    realistic case is `PATH="$CTL/fakegit:$PATH" git`, which is the fixtures' own shim idiom. **Not
    re-measured here**: the guard is withdrawn, so no fix depends on it.
15. Draft 2 said C3 was required. It was not (§0.2).
16. Draft 2 said the parent need not be touched because the touch was "avoidable". CLAUDE.md contains no
    such rule (draft-3 §8.3).
17. Draft 2 attributed a finding to "#501 R97". It was #519's Codex R7 (parent §10.6).
18. Draft 2 dropped the qualifier on `wire`. `wire:` parses as sed `w`, so `wire` is the default target
    and is never stripped.
19. Draft 2 said draft-2 §3.3 was the dissolved slot's "structural closure". The slot named `_git_exec`.
20. Draft 2 said the ledger step cited a "landing SHA". That SHA is unreachable from `main` after #501's
    squash.

**From round 8 (draft 9):**

21. Draft 8's cells were described as run in a scratch `HOME`. `c8/cell8.sh` never set it, so they ran
    under the real one. Re-run with it set, all 40 were unchanged (§A.11).
22. "RO leaves W2 silent" was half true. On p8 the RO run was already red (rc 2, W), but the control it
    reached was misreported as `green is reachable`, and W2 did not fire. p10 reports W2 (§A.12).
23. This author's first E2 cell (every loop fixture with `--separate-git-dir`) did not isolate P-g,
    because it was red on p8 through collateral (§A.12).
24. This author's p9 claimed P-g's population was "every fixture git dir" but walked only `$CTL/*/`. A
    nested and a hidden repo passed (§A.12).
25. A census over every directory cannot search the fixtures' deliberate `chmod 000` directories. An
    unconditional "unresolvable is red" rule reds the clean tree, so those three are declared (§A.12).

**From round 9 (draft 10):**

26. Draft 9's W3 was said to catch any diagnostic. It failed open under SIGPIPE once stderr was long
    (§A.13).
27. Draft 9's census was said to examine every git dir. It pruned the declared directories, so a
    nested repo inside one passed (§A.13).
28. The W cause was read from the exit status. A fixtures-file command can produce any status under
    errexit (§A.13, rc5).
29. This author's first p11 mode check assumed `find <dir> -prune -perm 000` reads an unreadable
    directory's mode. On BSD it reports "Permission denied" instead (§A.13).

---


## §E Appendix — the corpus scripts (as run)

To keep this file bounded, superseded scripts live only in the history:

| scripts | location |
|---|---|
| draft 4's full corpus | `git show 57e5419f:docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-provenance.md` (§E) |
| draft 5's subset | `git show e8c1bdb8:…` (§E.2) |
| draft 6's recast corpus (`c6/gen6.py`, `cell6.sh`, `eval6.sh`) | `git show ff1322bb:…` (§E.3) |
| draft 7's cells (`c7/gen7.py`, `eval7.sh`) | `git show ff1322bb:…` (§E.4) |
| draft 8's cells (`c8/gen8.py`, `eval8.sh`; `cell8.sh` = `cell6.sh`) | `git show 453b7b0f:…` (§E.5) |

Scratch copies are under `…/scratchpad/fgit/`. Draft 9's scripts follow.

### §E.6 Draft 9's focused cells (as run)

`c9/cell9.sh` is `c8/cell8.sh` with one change, on the `env` line: `HOME="$OUT/home"` after `TMPDIR=…`.

`c9/eval9.sh` is `c8/eval8.sh` with three arms added before `BEFORE-D2|BEFORE-D4)`:

```sh
    BEFORE-E2|BEFORE-E3|BEFORE-AR|BEFORE-E3g) if [ $green -eq 1 ] && [ $same -eq 0 ]; then v=SILENT-WRONG; elif [ $green -eq 1 ]; then v=GREEN-SAME; elif [ "$hit" = hit=1 ]; then v="RED-WITH-LABEL"; else v="RED-OTHER(ctl=${ctl#ctl=})"; fi ;;
    AFTER-E2|AFTER-E3|AFTER-AR|AFTER-SE1) [ "$hit" = hit=1 ] && v=PASS || v=FAIL ;;
    AFTER-E3g) [ $green -eq 1 ] && [ $same -eq 1 ] && v=PASS || v=FAIL ;;
```

`c9/gen9.py` (`LOOPINIT`/`SEP`/`SEP0` are the unused remains of run 1, §A.12):

```python
#!/usr/bin/env python3
"""Draft-9 focused cells (E2, E3, AR, SE1) on p8 (before) and p10 (after). Usage: gen9.py <scratch fgit dir>."""
import os, sys, shlex
SP = sys.argv[1]; OUT = os.path.join(SP, 'c9'); H = os.path.join(SP, 'corpus', 'h')
W = 'webref-generic-core-trip-wire'; parts = ['', '.controls', '.harness', '.fixtures', '.mutations']
CFGS = {'b53-g255': ('/opt/homebrew/bin/bash', '/opt/homebrew/bin'), 'b32-g254': ('/bin/bash', '/usr/bin')}
g = lambda *a: os.path.join(H, *a)
jobs = []; n = [0]
def mk(tree, cfg, kind, label, edits, env, expect, ref):
    n[0] += 1; tag = 'k%04d' % n[0]; T = os.path.join(tree, '.claude/tools')
    for p in parts:
        src = open(os.path.join(T, W + p + '.sh')).read()
        for (pp, old, new) in edits:
            if pp == p:
                assert src.count(old) >= 1, (label, p, old[:60]); src = src.replace(old, new, 1)
        dst = os.path.join(T, W + '.' + tag + p + '.sh'); open(dst, 'w').write(src); os.chmod(dst, 0o755)
    sh, gp = CFGS[cfg]
    jobs.append('\x1f'.join([tag, cfg, kind, label, sh, gp, ' '.join(shlex.quote(e) for e in env), expect, tree, ref]))
PYG = g('pyglob')
LOOPINIT = '( cd "$CTL/$d" 2>/dev/null && git init -q . >/dev/null 2>&1 \\'
SEP = '( cd "$CTL/$d" 2>/dev/null && git init -q --separate-git-dir="$CTL/.gd-$d" . >/dev/null 2>&1 && git config core.excludesFile %s \\' % PYG
SEP0 = '( cd "$CTL/$d" 2>/dev/null && git init -q --separate-git-dir="$CTL/.gd-$d" . >/dev/null 2>&1 \\'
CDINIT = '( cd "$CTL/cachedir" && git init -q . >/dev/null 2>&1 \\'
CD_SEP = '( cd "$CTL/cachedir" && git init -q --separate-git-dir="$CTL/.gd-cachedir" . >/dev/null 2>&1 && git config core.excludesFile %s \\' % PYG
CD_SEP0 = '( cd "$CTL/cachedir" && git init -q --separate-git-dir="$CTL/.gd-cachedir" . >/dev/null 2>&1 \\'
NEST = 'mkdir -p "$CTL/zz/inner" && ( cd "$CTL/zz/inner" && git init -q . && git config core.excludesFile %s )\n' % PYG
HID = 'mkdir -p "$CTL/.hid" && ( cd "$CTL/.hid" && git init -q . && git config core.excludesFile %s )\n' % PYG
BARE = 'git init -q --bare "$CTL/zbare" && git --git-dir="$CTL/zbare" config core.excludesFile %s\n' % PYG
FIFO = 'if [ "$_fifo_ok" -eq 1 ] && ! mkfifo "$CTL/odd/pipe" 2>/dev/null; then'
PIPE = ('.fixtures', FIFO, ': > "$CTL/odd/pipe"\n' + FIFO)
EXITL = '_fgit_window_incomplete_exit "$_fw_lbl"\n'
FIRSTCTL = '_control "$CTL/clean" 0 "PASSED"                  "green is reachable"   || ctl_ok=1\n'
MOVE = [('.controls', EXITL, ''), ('.controls', FIRSTCTL, FIRSTCTL + EXITL)]
PGL = 'CONTROL FAILED (every fixture repo persists only the configuration a plain git init writes)'
W2L = 'CONTROL FAILED (no control runs over an incomplete fixture build window)'
DIAGL = 'CONTROL FAILED (the fixtures file ran without a shell diagnostic)'
WL = 'CONTROL NOT EXERCISED (the fixture build window completed)'
BUILT = ': > "$_FW_DIR/built"\n'
for tree, phase in ((os.path.join(SP, 'p8'), 'BEFORE'), (os.path.join(SP, 'p10'), 'AFTER')):
    fx = open(os.path.join(tree, '.claude/tools', W + '.fixtures.sh')).read().split('\n')
    for cfg in CFGS:
        ref = 'REF-' + cfg
        mk(tree, cfg, 'REF', phase + ' clean', [], [], 'GREEN', ref)
        for lab, add in (('cachedir: --separate-git-dir + persisted core.excludesFile', CD_SEP), ('cachedir: --separate-git-dir only (gitfile shape)', CD_SEP0)):
            mk(tree, cfg, phase + '-E2', lab, [('.fixtures', CDINIT, add)], [], PGL, ref)
        for lab, add in (('nested repo zz/inner with persisted core.excludesFile', NEST), ('hidden top-level repo .hid with persisted core.excludesFile', HID), ('bare git dir zbare with persisted core.excludesFile', BARE)):
            mk(tree, cfg, phase + '-E2', lab, [('.fixtures', BUILT, add + BUILT)], [], PGL, ref)
        mk(tree, cfg, phase + '-E3', 'RO: exit moved below the first control + odd/pipe', MOVE + [PIPE], [], W2L, ref)
        mk(tree, cfg, phase + '-E3g', 'ROg: exit moved below the first control (window complete)', MOVE, [], 'GREEN', ref)
        mk(tree, cfg, phase + '-AR', 'AR: $(( 1/0 )) at the fixtures file top level', [('.fixtures', fx[0] + '\n', fx[0] + '\n_ar=$(( 1/0 ))\n')], [], DIAGL, ref)
        if phase == 'AFTER':
            mk(tree, cfg, 'AFTER-SE1', 'SE1: set +e left at the end of the fixtures file', [('.fixtures', BUILT, 'set +e\n' + BUILT)], [], WL, ref)
            mk(tree, cfg, 'G', 'HOME .config/git/ignore *.py', [], ['HOME=' + g('home_ign')], 'GREEN', ref)
            mk(tree, cfg, 'G', 'DO cell', [], ['GIT_CONFIG_GLOBAL=' + g('do', 'safe.cfg'), 'GIT_TEST_ASSUME_DIFFERENT_OWNER=1'], 'GREEN', ref)
            mk(tree, cfg, 'P', 'W2 record: incomplete-window exit removed + odd/pipe', [('.harness', '  echo "!! CONTROL NOT EXERCISED ($1): $_fw_why; nothing was built, so no control was run" >&2\n  exit 2\n', '  echo "!! CONTROL NOT EXERCISED ($1): $_fw_why; nothing was built, so no control was run" >&2\n'), PIPE], [], W2L, ref)
open(os.path.join(OUT, 'jobs.tsv'), 'w').write('\n'.join(jobs) + '\n'); print(len(jobs), 'jobs')
```

### §E.7 Draft 10's cells (as run)

`c11/cell9.sh` and `c11/eval11.sh` are copies of `c9/cell9.sh` and `c9/eval9.sh`. `c11/gen11.py`:

```python
#!/usr/bin/env python3
"""Draft-10 cells (A1 W3 under SIGPIPE, A2 census inside declared dirs, A3 shapes/causes) on p10 (before) and p11 (after), plus the draft-9 set on p11. Usage: gen11.py <abs scratch fgit dir>."""
import os, sys, shlex
SP = sys.argv[1]; OUT = os.path.join(SP, 'c11'); H = os.path.join(SP, 'corpus', 'h'); OUTSIDE = os.path.join(SP, '..', 'r9ax3', 'outside')
W = 'webref-generic-core-trip-wire'; parts = ['', '.controls', '.harness', '.fixtures', '.mutations']
CFGS = {'b53-g255': ('/opt/homebrew/bin/bash', '/opt/homebrew/bin'), 'b32-g254': ('/bin/bash', '/usr/bin'),
        'b53-g254': ('/opt/homebrew/bin/bash', '/usr/bin'), 'b32-g255': ('/bin/bash', '/opt/homebrew/bin')}
g = lambda *a: os.path.join(H, *a)
jobs = []; n = [0]
def mk(tree, cfg, kind, label, edits, env, expect, ref):
    n[0] += 1; tag = 'm%04d' % n[0]; T = os.path.join(tree, '.claude/tools')
    for p in parts:
        src = open(os.path.join(T, W + p + '.sh')).read()
        for (pp, old, new) in edits:
            if pp == p:
                assert src.count(old) == 1, (label, p, old[:60]); src = src.replace(old, new, 1)
        dst = os.path.join(T, W + '.' + tag + p + '.sh'); open(dst, 'w').write(src); os.chmod(dst, 0o755)
    sh, gp = CFGS[cfg]
    jobs.append('\x1f'.join([tag, cfg, kind, label, sh, gp, ' '.join(shlex.quote(e) for e in env), expect, tree, ref]))
PYG = g('pyglob')
L1 = lambda tree: open(os.path.join(tree, '.claude/tools', W + '.fixtures.sh')).read().split('\n')[0] + '\n'
BUILT = ': > "$_FW_DIR/built"\n'
CHM = 'chmod 000 "$CTL/err/control.py" "$CTL/walk/sub"\n'
M2H = ('( mkdir -p "$CTL/walk/sub/inner" && cd "$CTL/walk/sub/inner" && git init -q . >/dev/null 2>&1 && git config core.excludesFile %s'
       ' && printf \'x\\n\' > a.py && printf \'y\\n\' > b.txt && git add -A && GIT_AUTHOR_DATE=\'2000-01-01T00:00:00Z\' GIT_COMMITTER_DATE=\'2000-01-01T00:00:00Z\''
       ' git -c user.name=w -c user.email=w@e commit -q -m c && cd "$CTL/walk" && git add -A >/dev/null 2>&1 ) || _fixture_failed walkinner\n') % os.path.join(OUTSIDE, 'ign-py')
NR = 'mkdir -p "$CTL/walk/nr" && chmod 300 "$CTL/walk/nr"\n'
WTM = ('( mkdir -p "$CTL/zzw" && cd "$CTL/zzw" && git init -q . && git -c user.name=w -c user.email=w@e commit -q --allow-empty -m c'
       ' && git worktree add -q ../zzw2 >/dev/null 2>&1 && rm -rf ../zzw2 )\n')
AR = '_ar=$(( 1/0 ))\n'
NOISE = lambda k: 'for _k in $(seq 1 %d); do [ "$_k" -eq x ] || :; done\n' % k
PGL = 'CONTROL FAILED (every fixture repo persists only the configuration a plain git init writes)'
DIAGL = 'CONTROL FAILED (the fixtures file ran without a shell diagnostic)'
W2L = 'CONTROL FAILED (no control runs over an incomplete fixture build window)'
WL = 'CONTROL NOT EXERCISED (the fixture build window completed)'
FIFO = 'if [ "$_fifo_ok" -eq 1 ] && ! mkfifo "$CTL/odd/pipe" 2>/dev/null; then'
PIPE = ('.fixtures', FIFO, ': > "$CTL/odd/pipe"\n' + FIFO)
EXITL = '_fgit_window_incomplete_exit "$_fw_lbl"\n'
FIRSTCTL = '_control "$CTL/clean" 0 "PASSED"                  "green is reachable"   || ctl_ok=1\n'
MOVE = [('.controls', EXITL, ''), ('.controls', FIRSTCTL, FIRSTCTL + EXITL)]
CDINIT = '( cd "$CTL/cachedir" && git init -q . >/dev/null 2>&1 \\'
CD_SEP = '( cd "$CTL/cachedir" && git init -q --separate-git-dir="$CTL/.gd-cachedir" . >/dev/null 2>&1 && git config core.excludesFile %s \\' % PYG
NEST = 'mkdir -p "$CTL/zz/inner" && ( cd "$CTL/zz/inner" && git init -q . && git config core.excludesFile %s )\n' % PYG
HID = 'mkdir -p "$CTL/.hid" && ( cd "$CTL/.hid" && git init -q . && git config core.excludesFile %s )\n' % PYG
BARE = 'git init -q --bare "$CTL/zbare" && git --git-dir="$CTL/zbare" config core.excludesFile %s\n' % PYG
P10, P11 = os.path.join(SP, 'p10'), os.path.join(SP, 'p11')
for tree, ph in ((P10, 'BEFORE'), (P11, 'AFTER')):
    for cfg in ('b53-g255', 'b32-g254'):
        ref = 'REF-' + cfg
        mk(tree, cfg, 'REF', ph + ' clean', [], [], 'GREEN', ref)
        mk(tree, cfg, ph + '-E2', 'arnoise: AR + 200 [ diagnostics', [('.fixtures', L1(tree), L1(tree) + AR + NOISE(200))], [], DIAGL, ref)
        mk(tree, cfg, ph + '-E2', 'noise1k: 1000 [ diagnostics', [('.fixtures', L1(tree), L1(tree) + NOISE(1000))], [], DIAGL, ref)
        mk(tree, cfg, ph + '-E2', 'ar: AR alone', [('.fixtures', L1(tree), L1(tree) + AR)], [], DIAGL, ref)
        mk(tree, cfg, ph + '-E2', "rc5: sh -c 'exit 5' at the top level (cause named generically)", [('.fixtures', L1(tree), L1(tree) + "sh -c 'exit 5'\n")], [], 'the window exited 5 before completing', ref)
        mk(tree, cfg, ph + '-E2', 'nr: a chmod 300 dir under walk', [('.fixtures', BUILT, NR + BUILT)], [], PGL, ref)
        mk(tree, cfg, ph + '-E2', 'wtmeta: a linked worktree removed, its .git/worktrees entry left', [('.fixtures', BUILT, WTM + BUILT)], [], PGL, ref)
    for cfg in CFGS:
        mk(tree, cfg, ph + '-E2', 'm2h: nested repo under walk/sub with an outside core.excludesFile, gitlinked into walk', [('.fixtures', CHM, M2H + CHM)], [], PGL, 'REF-' + cfg if cfg in ('b53-g255', 'b32-g254') else 'NOREF')
# draft-9 regression set on p11
for cfg in ('b53-g255', 'b32-g254'):
    ref = 'REF-' + cfg
    for lab, ed in (('cachedir --separate-git-dir + key', [('.fixtures', CDINIT, CD_SEP)]), ('nested zz/inner', [('.fixtures', BUILT, NEST + BUILT)]),
                    ('hidden .hid', [('.fixtures', BUILT, HID + BUILT)]), ('bare zbare', [('.fixtures', BUILT, BARE + BUILT)])):
        mk(P11, cfg, 'AFTER-E2', 'd9 ' + lab, ed, [], PGL, ref)
    mk(P11, cfg, 'AFTER-E3', 'd9 RO', MOVE + [PIPE], [], W2L, ref)
    mk(P11, cfg, 'AFTER-E3g', 'd9 ROg', MOVE, [], 'GREEN', ref)
    mk(P11, cfg, 'AFTER-SE1', 'd9 SE1 set +e', [('.fixtures', BUILT, 'set +e\n' + BUILT)], [], 'the fixtures file switched off errexit, nounset or pipefail', ref)
    mk(P11, cfg, 'G', 'd9 G HOME ignore', [], ['HOME=' + g('home_ign')], 'GREEN', ref)
    mk(P11, cfg, 'G', 'd9 G DO', [], ['GIT_CONFIG_GLOBAL=' + g('do', 'safe.cfg'), 'GIT_TEST_ASSUME_DIFFERENT_OWNER=1'], 'GREEN', ref)
    mk(P11, cfg, 'P', 'd9 W2 record', [('.harness', '  echo "!! CONTROL NOT EXERCISED ($1): $_fw_why; nothing was built, so no control was run" >&2\n  exit 2\n', '  echo "!! CONTROL NOT EXERCISED ($1): $_fw_why; nothing was built, so no control was run" >&2\n'), PIPE], [], W2L, ref)
open(os.path.join(OUT, 'jobs.tsv'), 'w').write('\n'.join(jobs) + '\n'); print(len(jobs), 'jobs')
```
