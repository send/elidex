# K2 fixture git — provenance companion

This file is the provenance for `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md`, called "the
design memo" below. It holds:
- the measurements behind that memo's decisions (§A);
- what became of the replaced branch (§B);
- the premises found false (§C);
- every plan-review round's dispositions (§D).

The design memo keeps only its live decisions. Section references here are to the design
memo **as of the draft that the section or row belongs to** unless a file is named; a superseded draft is read with `git show <sha>:<memo path>` (drafts: 1 `bd3dc513`,
2 `0142f47a`, 3 `2b89ef7c`, 4 `57e5419f`, 5 `e8c1bdb8`, 6 `8b6a4005`).

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

---

## §D Plan-review dispositions

### §D.0 Terminators and the close-out plan

**Round 8: a Step-4.5 focused check (current).**
- **Scope:** D2 and D4 only. Ax2 checks D4 and the window's completion. Ax3 checks D2 (P-g) and D3 (the
  transient residual).
- **Converges** if and only if neither returns an IMP with a failing cell.
- **Then plan-review closes.**

**Round 7 (as it ran; not met).** Round 7 was the Step-4.5 focused check of draft 7:
- Ax2 (item 1): 0 CRIT / 2 IMP / 4 MIN;
- Ax3 (item 2): 0 CRIT / 3 IMP / 1 MIN.

The behaviour was sound, and the IMPs were in the secondary instruments.

⚠ **Draft 8 removes an instrument that round 7 was asked to check.** Seed S was draft 7's answer to
item 2, and the round-7 check measured it to be a vocabulary deny-list: it let literal spellings through
silently and red-flagged harmless ones. So D1 deletes it rather than extending it. This is recorded as
a removal, not as a fix of S.

**Round 6 (as it ran).** Round 6 was a **full five-axis review** of draft 6 `8b6a4005`, not the
focused one that draft 6 had announced:
- Ax1: 0.
- Ax2: 0/1/3.
- Ax3: 0/1/1.
- Ax4: 0/1/8.
- Ax5: 0/2/6.

In total, 0 CRIT / 5 IMP / 18 MIN. Its terminator was "no IMP with a failing command", and it was not
met, although **no finding moved the window mechanism**. Draft 6 had recorded it as "focused,
Ax2/3/5". That came from the orchestrating session's instruction before it decided on a full round,
and it is corrected here.

**Round 5 (not met).** Ax2 raised 1 IMP and Ax3 raised 3, all with cells. The root cause was that the
detector was a name list, and the design changed as a result (§D.5).

**Round 4 (not met: 5 unique IMP, each with a cell).** Round 4 converges if no IMP lands on draft-4-added text, and no IMP moves a mechanism
**without a corpus cell that shows the defect**. A finding that comes with a failing cell is fixed by
driving the prototype to pass it, and then the memo is re-derived from the result. It is not fixed in
prose first.

**Round 3 (not met).** Round 3 would have converged if no IMP had landed on draft-3-added text and no
IMP had moved a mechanism. An IMP of
either kind is a reset. If that reset is self-introduced, the next draft shrinks mechanism again rather
than adding to it. Round 2 was the first self-introduced reset: 8 of its 10 IMP landed on draft-2 text.

### §D.1 Round 1 (on draft 1 + §2.5; 0 CRIT / 13 IMP, 11 unique / 25 MIN / 14 FP) → draft 2 `0142f47a`

| F | disposition in draft 2 | fate in draft 3 |
|---|---|---|
| F1 | P-d by content, P-a by origin | kept |
| F2 | umbrella slice and memo rows | kept, pointers corrected (R2-4) |
| F3 | marker spelling; memo-row "by design"; ENVIRONMENT VARIABLES | kept |
| F4 | sites enumerated; parent not edited | **reversed** by R2-4 |
| F5 | ci.yml not edited; STOP rule | **reversed** by R2-1's cost clause: re-derive per the in-file rule |
| F6 | criterion change stated | kept; boundary added (R2-3) |
| F7 | `PATH` shim + absolute-path slot | **withdrawn** (R2-1) |
| F8 | ratchet widened to `_lbl` | kept; records for every label (R2-2) |
| F9 | separate labels for draft-2 §6.3/§6.4 | superseded: one differential, one label, one record |
| F10 | source-time snapshot; measured predictions | kept |
| F11 | one parts list, three readers | kept; guard placement fixed |

### §D.2 Round 2 (on draft 2 `0142f47a`; 0 CRIT / 10 IMP / 19 MIN / 17 FP) → draft 3

| id | disposition | verifying command / evidence |
|---|---|---|
| R2-1 | the guard, canary, marker, absolute-path slot, `envscrub` and 12 probes are replaced by **one** whole-build differential. Three universal members, each proven live alone before the hostile build. Verdict order after `ctl_ok=0`. Errexit specified. Refs are in P. Cost re-derived by the ci.yml rule, with STOP on change | prototype D (§A.4): unmutated green; `command env`, call-time `HOME`, `_git`, `PATH=… git` and absolute path all red; `env -i git` green, booked. `git show e8f78896:.github/workflows/ci.yml \| sed -n '/^  trip-wires:/,/^  [a-z]/p'` for the rule. Driver timing, three runs per side |
| R2-2 | six labels, six records; `_MUT_UNRECORDED_MAX` unchanged; X11 | P-a/P-c records measured (§A.3). Existing `_lbl` records: `/usr/bin/grep -cF` → 1 each |
| R2-3 | §0.1 boundary: P governs git's inputs, not executable or platform (R1, R2, R5). R3 booked as `#11-k2-fgit-machine-files`, which is also R3(b). Own deferrals: **1** | design memo §0.1, §5 |
| R2-4 | parent touched: C0a (move §5 and §10 to a rounds companion, stub headings) + C0b (banner), under CLAUDE.md's quoted text. Umbrella pointers name files and sections exactly. The code-reference convention is stated; references are qualified in edited files | `/usr/bin/grep -n '^## '` over the parent (section map); the 17-line reference grep |
| R2-5 | this companion | `wc -l` of both files |
| R2-6 | C3's "required" claim withdrawn and re-grounded. Full clause answered, with Q1 | design memo §0.2 |

**MINs:**
- draft-2 §6.4's "already live": a false ordering, fixed by draft-3 §6.3's liveness order.
- The union shadowing: handled by proving each member live alone.
- Hostile files are written before the build.
- The dedicated E8 repo: moot, since there are no per-channel probes.
- The guard marker reset: moot.
- C1 states the fixtures file is sourced.
- R8 now names P-d's reftable failure.
- Refs are in P.
- The non-git commands are named as the class R4 plus draft-3 §10 row 15.
- P is narrowed to what draft-3 §6.3 compares.
- C6 is withdrawn.
- The Stage-5 coverage claim is corrected (§B).
- draft-2 §8.1's attribution is fixed.
- The test-suite claim is removed.
- "Recipe" is narrowed to ingredients.
- The GNU sed claim is stated as unverified.
- The slot quotes are verbatim.
- The SSoT "12" is marked as inference.
- R1 is stated as this slice's own residual.
- The ledger step and the landing SHA are corrected.
- The land order gains user approvals, the #501 P2 thread, the status-cell update, and the #501
  decision.
- The sibling guard lives only in `_mut_run`.
- P-c runs last.
- `#501 R97` is corrected to `#519 R7`.
- The `wire` qualifier is restored.
- The draft-3 §7 closure claim is corrected.
- The umbrella pointers are fixed.

### §D.3 Round 3 (on draft 3 `2b89ef7c`; 0 CRIT / 7 IMP / 22 MIN / 4 FP) → draft 4

Round 3 did not meet its terminator: all 7 IMP landed on draft-3 text, and 4 of them moved the
mechanism. The process changed as a result: the corpus came first (§A.5), and draft 4 was written from
it.

| finding | disposition | evidence |
|---|---|---|
| Ax2/Ax3 IMP-1: the members are not universal across subcommands and fixture shapes | the differential and its members are **replaced** by an invocation-based detector (`GIT_TRACE` plus a `PATH` shim, around the one build) | §A.5 R: 130/130 × 4 configs, across all 13 call-site classes and 10 spellings, including `commit`, `update-index`, `hash-object -w`, `add -f` and `-c` overrides |
| Ax3: poison members, plus a `PATH` shim for `env -i PATH="$PATH" git` | the `PATH` shim is adopted. Poison was **not** needed: `GIT_TRACE` catches every environment-inheriting git regardless of subcommand or `-c`, and it does not break the build it observes | R rows `env -i PATH="$PATH" git`, `-c override`, `hash -p` |
| Ax2 IMP-2: canaries inherit the caller's environment, so an ordinary caller kills a member | moot: the detector's canaries test channels the gate itself sets. The caller's git configuration cannot switch `GIT_TRACE` or `PATH` off | §A.5 G: 29/29 × 4, including `GIT_CONFIG_GLOBAL=/dev/null`, `GIT_CONFIG_GLOBAL`/`GIT_CONFIG_COUNT`/`GIT_CONFIG_PARAMETERS` with `init.defaultBranch` |
| Ax2 MIN: ref values | P now includes deterministic ref values; commits are compared by tree | design memo §1 |
| Ax2 MIN: the hostile `_FIX_FAILED` has no reader; errexit differs between 5.3 and 3.2 in `( . fixtures ) \|\| rc` | moot: there is no second build. The one build is sourced in the calling shell exactly as at base | p4 |
| Ax2 MIN: two producers share one label | one label per producer for the P-a/P-b liveness halves (`e58fec48`). ⚠ **Draft 4's claim that this gave one label per producer was false**: the detector's labels still had several producers (`_fdl_lbl`: setup, PATH canary, trace canary; `_fd_lbl`: PATH arm, trace arm). Round 4 found this, and draft 5 fixes it (§D.4) | §A.5 re-run |
| Ax2 MIN: d1 did not match the design | p4 carries §3–§5 exactly, P-a…P-e included | §A.5 |
| Ax3 harness defect (`rm -rf $d/.git`) | §A.4 is marked invalid; the corpus harness keeps `.git` | §A.4 note |
| Ax3 MIN: R4 ground, errexit wording, "no measured way", cost sentence | R4 is restated as a class, with no ground argued beyond the launch slot. The errexit text now describes only what p4 does. "No measured way" is deleted along with the differential. The cost sentence is rewritten from the §A.6 data | draft-4 design memo §4, §5.2 |
| Ax5 IMP: C0a had no seam (parent §5 is live) | **C0a withdrawn**. Only the banner (C0b) remains; its site list is derived by rule and command, giving **seven** sites (round 3 said five) | design memo §8.2 |
| Ax5 IMP: the wire-reference convention resolves 8 references wrongly | the convention is **dropped**. A derived table resolves each reference (8 parent, 1 A-i, plus the qualified ones) | design memo §8.3 |
| Ax4/Ax5: the umbrella memo table lacked a row for the companion | the companion row was added in draft 3; draft 4 keeps it. The rounds companion is moot, since C0a is withdrawn | umbrella |
| Ax3/Ax5: `#11-k2-fgit-machine-files` held two gaps | **withdrawn at create time**: (a) becomes the declared blind spot R3; (b) is the invocation-convention slot's own gap, which that slot is narrowed to. Own new deferrals: 0 | draft-4 design memo §5.2, §7 |
| R2-2 carried: ratchet | measured: 30 bare labels with a widened population; 7 of the 9 new labels get records; +2 declared raise for the two version guards | §A.7 |

⚠ **Not addressed individually.** Ax4's 7 and Ax5's 7 MINs were not in this session's brief as texts,
and the review agents' output files were unreachable (dangling links under `tasks/`). Their stated
subjects were handled where named: wording/pointer fixes, memory references anchored by heading text,
and the umbrella rows. Any MIN not listed above is **open**, and it should be re-raised against draft 4.


### §D.4 Round 4 (on draft 4 `57e5419f`; 0 CRIT / 9 IMP, 5 unique / 20 MIN / 22 FP) → draft 5

Round 4 did not meet its terminator: 5 unique IMP, each with a failing cell. Nobody broke the detector
core.

| id | disposition | evidence (companion §A.8) |
|---|---|---|
| U1: the record plan | Producers move into harness functions; labels stay in the controls file and are passed in. Targets become `wire harness fixtures`: BSD `sed 'fixtures:p'` gives rc 1; GNU is unmeasured. Records come from representative cells only: 16 new labels, 16 records. `_MUT_UNRECORDED_MAX` stays 21. RES cells are not records, because of the `!survive` exactly-once rule. X2 and X5 now use the subset | P column 14/14 × 4. The record table is in design memo §6 |
| U2+U3: the escape class | Two channels are added. (a) An env shim, with `_fgit` using an absolute `env` captured at source time. (b) Trace2 via `GIT_TRACE2_EVENT` and via `GIT_CONFIG_GLOBAL` (config route), plus poison `GIT_CONFIG_COUNT=bogus`. `GIT_TRACE` is dropped, because it was measured to be shadowed. The residual shrinks to a git started from nothing by an absolute-path program. The slot is narrowed with a property-defined trigger | R column 33/33 × 4, including the round-4 escapes and `env -`; RES green; BEFORE d0234/5/7/8 silent on p4 |
| U4: P-e | The record injects `GIT_EXEC_PATH`, which reds on both gits. The `PATH`-drop mutant becomes an INFO cell and is not a record. No runner mechanism was added | P-e record PASS × 4; INFO red on git 2.55, green on 2.54 |
| U5: uninitialised state | Every detector name is assigned at harness top level. A setup failure is a labelled verdict. Nothing relies on `set -u` | setup record PASS × 4, against BEFORE d0233 (rc 1, unlabelled) and d0236 (**rc 0**) |
| Pre-existing: bash 3.2 EXIT trap plus `set -u` exits 0 | Recorded in design memo §5.2 and not fixed, because the wire is untouched. The orchestrator books the slot | `…/scratchpad/r4ax2/u.sh` (the orchestrator's) |

**MINs:**
- **Ax4, C1 range:** stated exactly as base controls lines 83–734; lines 735–737 are `ctl_ok=0`'s
  comment. The §7 census now uses `NR<=734`.
- **Ax4, the D-label collision:** labels renamed to F…, R2-…, R3-…, U….
- **Ax4, X4 cannot see comment-only qualification:** added X4b, a grep that requires a memo filename on
  every `§N` / "plan memo" line.
- **Ax4, umbrella wording:** now "one slice row, one memo-table row, one status cell".
- **Ax4, bare § references:** qualified as "draft-N §x" throughout §A–§D.
- **Ax4, the RES classifier:** now requires rc 0, 0 control lines and PASSED for RES-GREEN
  (`corpus5/cell.sh`).
- **Ax5, banner scope:** re-derived by property over C1–C5, giving nine sites, including L683 (C4) and
  L866–868 (§9, present tense).
- **Ax5, the §8.3 grep:** now covers every `§[0-9]`, which gives 9 A-i and 8 parent unqualified wire
  references. Draft 4's "A-i for 1" was false.
- **Ax5, stale header:** the pointer now reads §8.2.
- **Ax5, X8:** now names #510, with its head via `gh pr view 510 --json headRefOid`.
- **Ax5, `env -`:** closed by the env shim.
- **Ax5, in-file mutations statements** (`:44–46`, `:136–138`, `:684`): listed as C2/C4/C5
  obligations.
- **Ax5, §8.2's ground:** now states both §5 (live) and §10 (cited by live §11: `§10.4`, `§10.8`).
- **Ax2, detector labels with several producers:** now one per producer; the e58fec48 claim is
  corrected in §D.3.
- **Ax2, caller `GIT_TRACE=1`:** noted as pre-existing on the read side (R6, parent D7).
- **Ax3, §4 cost prose standing in for X8:** that sentence now points to X8.
- **Ax3, mutation-mode cost:** stated as a formula over X3's own counts.

⚠ Round 4's six Ax4 MINs and ten Ax5 MINs are addressed only where this session's brief named their
subject, which was every item listed above. Anything else stays open for round 5.

### §D.5 Round 5 (focused; on draft 5 `e8c1bdb8`; Ax2 0/1/2, Ax3 0/3/4, Ax5 0/0/6) → draft 6

| id | disposition | evidence (§A.9) |
|---|---|---|
| V1, Ax3 IMP-1: the detector is a name list (scrub3, hermetic-look, gitsweep, t2zero-countempty) | **Design change** (the orchestrating session's decision): the whole fixture-build window is built from nothing, and the detector is deleted | BEFORE 8/8 silent-wrong on p5d; AFTER, R PASS with P identical × 4 configs |
| V2, Ax3 IMP-2: a wrapper git false-reds (x005/x010) | **moot**: there is no `PATH` shim | — |
| V3, Ax3 IMP-3: the system prefix is caller-writable | stated as the residual. The claim "reads no caller input" is withdrawn | ownership measurement (§A.9) |
| V4, Ax2 IMP: the restore of `GIT_CONFIG_GLOBAL` is unpinned | **moot**: there is no restore; the window is a child. The DO cell is green with P equal × 4 | G DO cell |
| V5, the decision's own premise: "the one spelling that can still read caller state is `env -i` / `env -u GIT_CONFIG_NOSYSTEM`" | **refined**. The residual is any fixture command that removes or overrides an allowlist entry: `env -i`, `env -`, `env -u`, `NAME=… git`, `unset NAME`. RES cells pass here only because this machine's system file and template are benign | RES 2/2 × 4 with P equal; residual stated by property |
| V6, the decision's own premise: "`_fgit`'s per-call `env -i` may be redundant" | **confirmed and removed**. After the move, no use of `_fgit` remains outside the window (the postconditions moved inside), so the fixtures call `git` | `/usr/bin/grep -c '_fgit' p6/…fixtures.sh` → 0 |

**MINs:**
- **Ax5, X4b case:** now `grep -i` (it catches mutations:20 "The memo").
- **Ax5, "P (14)" vs a 16-row table:** moot; the table and count are now 9 and 9.
- **Ax5, banner seed:** the seed is widened (`.bare|mutation record|every control|unreachable or loud`), giving eleven sites, including L816 and L1212–1214. The memo now says the seed is a seed.
- **Ax5, in-file mutations statements:** now listed as `:44–46`, `:120`, `:136–138`, `:672–673` and `:684–688`, found by a stated grep.
- **Ax5, §5.2:** names `#11-k2-wire-exit-trap-masks-set-u-abort`, and nothing new relies on `set -u`.
- **Ax5, X2/X5 re-run recipe:** the scripts take a directory argument and `K2_CORPUS_OUT`, with the tree under test at `<dir>/p6`.
- **Ax2/Ax3 MINs tied to the detector** (M1 `cmd_name` absent under poison; M2/M3 detector messages and the canary order): **moot**, deleted with the detector.
- ⚠ Any round-5 MIN not named in this session's brief remains open for round 6.

### §D.6 Round 6 (full; on draft 6 `8b6a4005`; 0 CRIT / 5 IMP / 18 MIN) → draft 7

| id | disposition | evidence |
|---|---|---|
| W1 (item 1, Ax2): an incomplete window was read as "no fixture failed", and the controls cascaded | fixed on p7: W alone, the child's exit class carried (2 stays 2), no control over unbuilt trees. The W records now require exactly one control line | §A.10: `odd/pipe` went from rc 1 with 82 lines to rc 2 with 1, on both shells |
| W2 (item 2, Ax3 plus Ax2 MIN-3): add-input spellings silently diverge | the gap is re-stated by property (remove, override or add an input). S is adopted as a seed; P-a over every repo is declined; the over-claims are scoped (§3 (iii), §4, §10 rows 1–2, the P-a label); the slot trigger is honest | §A.10: SILENT-WRONG on p6 becomes S red on p7 |
| W3 (item 3, Ax4): `\|` inside table cells | every command containing a pipe now sits in a fenced block with `-e` alternatives; each was re-run | §A.10, moved commands |
| W4 (item 4, Ax5): X9 cannot run on a stacked PR | both routes, (a) a temporary draft PR to `main` and (b) #501's CI with a stop condition, are written down as a push-time user decision | design memo §9 |
| W5 (item 5, Ax5): statements that become false after `_fgit`'s deletion | harness:22–47, harness:48–49 and controls:20–22 are listed, with the grep that finds them | design memo §8.1 |

**MINs:**
- **Ax2 MIN-1:**
  - the census now has a derivation command, and it matches the prelude by static read-set;
  - `set -u` is pinned by the options check plus a W record;
  - the three sentences now say "nothing in the parent relies on `set -u`".
- **Ax2 MIN-2:** P-f now parses every `env -0` record.
- **Ax4 MIN-1:** the mutations list is `:133–138`, and the grep's full return is listed (93 and 526 are
  read and kept).
- **Ax4 MIN-2:** banner lines corrected to L747–748 and L1281–1282; L279 is noted as changing under C5.
- **Ax4 MIN-3:** 0 of 207 man pages mention `GIT_ATTR_NOSYSTEM`, with a positive control.
- **Ax4 MIN-4:** "in §E" pointers now use the history route.
- **Ax4 MIN-5:** drifted "design memo §" references are qualified by draft, with a stated convention in
  the header.
- **Ax4 MIN-6:** the ledger premise is quoted verbatim.
- **Ax4 MIN-7:** the owner is the citation-hygiene lane, and the correct replacement text for the
  exit-trap ledger entry is given.
- **Ax4 MIN-8:** A-ii's recogniser is described as line-anchored, fence-aware and §3-scoped.
- **Ax4 false premise and Ax5 MIN, round-6 scope:** recorded as it ran (§D.0).
- **Ax5, "0 new own deferrals":** corrected to 1 own deferral under a re-scoped name, with the ledger
  net at −1.
- **Ax5, ledger timing:** the ledger text is planned now (design memo §9) and written at PR creation.
- **Ax5, fixtures:20–28 `exit 2`:** subsumed by W1.
- **Ax5, X8:** where the record goes, and the interaction with #510 (head `94281cd7` at writing).
- **Ax5, umbrella "§5.1: the residual":** now points to §5.1 for the gap and §5.2 for the other
  residuals.

### §D.7 Round 7 (Step 4.5; on draft 7 `ff1322bb`; Ax2 0/2/4, Ax3 0/3/1) → draft 8

| id | disposition | evidence (§A.11) |
|---|---|---|
| D1 (Ax3 IMP-1): S is a vocabulary deny-list, with silent-wrong literal spellings and false reds | **S deleted**, along with its label, record and census, because extending it is the wrong repair | Ax3's r7ax3 cells; design memo §5.1 |
| D2 (Ax3 IMP-2): the P-a-over-every-repo decline rested on a false premise (persisting does not need `git config`) | **P-g adopted**: every fixture repo's configuration must equal a reference init's, line by line. The key set is derived from git per run, not from a path-key list | before p7: silently wrong ×3 per shell; after p8: P-g red ×4 per shell; census clean |
| D3 (Ax3 IMP-3): over-claim scope and the residual | the unit is "every git whose inputs no fixtures-file command altered". The residual is re-scoped to **transient** inputs, with a trigger of reviewer attention | transient cell h0019/h0033: silent, as declared |
| D4 (Ax2 IMPs): pinning | W2 added; one record per prelude option; `built` from the fixtures file's last line; the exit-number claim is dropped as unconsumed (driver `set -euo pipefail` plus `bash "$w"`, lines 22/204) | W2 and option records PASS; early `return 0` goes from 81 lines to W alone |

**MINs:**
- **Ax2 MIN-1 (a top-level `return 0`):** fixed by the `built` marker.
- **Ax2 MIN-2 (the class decided by value):** moot, because the number is not claimed.
- **Ax2 MIN-3 (wording, and parent-side failure mislabelled):** fixed with reason text via `_fw_why`.
- **Ax2 MIN-4 (W alone suppresses static checks):** moot, because S is deleted.
- **Ax3 MIN-1:** moot, because S is deleted.
- **The claim in W1 (§D.6) that "records require exactly one control line":** it was **false** for the
  runner. It holds only for the corpus evaluator (`eval7`/`eval8` check `ctl=1`). The mutation runner
  sees only rc ≠ 0 and a needle. Draft 8 therefore pins W2 as a separate label.
---

## §E Appendix — the corpus scripts (as run)

To keep this file bounded, superseded scripts live only in the history:

| scripts | location |
|---|---|
| draft 4's full corpus | `git show 57e5419f:docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-provenance.md` (§E) |
| draft 5's subset | `git show e8c1bdb8:…` (§E.2) |
| draft 6's recast corpus (`c6/gen6.py`, `cell6.sh`, `eval6.sh`) | `git show ff1322bb:…` (§E.3) |
| draft 7's cells (`c7/gen7.py`, `eval7.sh`) | `git show ff1322bb:…` (§E.4) |

Scratch copies are under `…/scratchpad/fgit/`. Draft 8's scripts follow.

### §E.5 Draft 8's focused cells (as run)

`c8/cell8.sh` is `c6/cell6.sh` unchanged.

`c8/gen8.py`:

```python
#!/usr/bin/env python3
"""Draft-8 focused cells (D2, D4) on p7 (before) and p8 (after). Usage: gen8.py <scratch fgit dir>."""
import os, sys, shlex
SP = sys.argv[1]; OUT = os.path.join(SP, 'c8'); H = os.path.join(SP, 'corpus', 'h')
W = 'webref-generic-core-trip-wire'; parts = ['', '.controls', '.harness', '.fixtures', '.mutations']
CFGS = {'b53-g255': ('/opt/homebrew/bin/bash', '/opt/homebrew/bin'), 'b32-g254': ('/bin/bash', '/usr/bin')}
g = lambda *a: os.path.join(H, *a)
jobs = []; n = [0]
def mk(tree, cfg, kind, label, edits, env, expect, ref):
    n[0] += 1; tag = 'h%04d' % n[0]; T = os.path.join(tree, '.claude/tools')
    for p in parts:
        src = open(os.path.join(T, W + p + '.sh')).read()
        for (pp, old, new) in edits:
            if pp == p:
                assert src.count(old) >= 1, (label, p, old[:60]); src = src.replace(old, new, 1)
        dst = os.path.join(T, W + '.' + tag + p + '.sh'); open(dst, 'w').write(src); os.chmod(dst, 0o755)
    sh, gp = CFGS[cfg]
    jobs.append('\x1f'.join([tag, cfg, kind, label, sh, gp, ' '.join(shlex.quote(e) for e in env), expect, tree, ref]))
CD = '( cd "$CTL/cachedir" && git init -q . >/dev/null 2>&1 \\\n'
INC = g('home_ex', '.gitconfig'); PYG = g('pyglob')
PERSIST = [('printf [include] path >> .git/config', "  && printf '[include]\\n\\tpath = %s\\n' >> .git/config \\\n" % INC),
           ('printf [core] excludesFile >> .git/config', "  && printf '[core]\\n\\texcludesFile = %s\\n' >> .git/config \\\n" % PYG),
           ('git -C . config core.excludesFile <file>', "  && git -C . config core.excludesFile %s \\\n" % PYG),
           ('git config include.path <file>', "  && git config include.path %s \\\n" % INC)]
PGL = 'CONTROL FAILED (every fixture repo persists only the configuration a plain git init writes)'
WL = 'CONTROL NOT EXERCISED (the fixture build window completed)'
FIFO = 'if [ "$_fifo_ok" -eq 1 ] && ! mkfifo "$CTL/odd/pipe" 2>/dev/null; then'
for tree, phase in ((os.path.join(SP, 'p7'), 'BEFORE'), (os.path.join(SP, 'p8'), 'AFTER')):
    fx = open(os.path.join(tree, '.claude/tools', W + '.fixtures.sh')).read().split('\n')
    li = next(i for i, l in enumerate(fx) if not l.lstrip().startswith('#') and 'git add -A' in l); col = fx[li].index('git add -A')
    for cfg in CFGS:
        ref = 'REF-' + cfg
        mk(tree, cfg, 'REF', phase + ' clean', [], [], 'GREEN', ref)
        for name, ins in PERSIST:
            mk(tree, cfg, phase + '-D2', name, [('.fixtures', CD, CD + ins)], [], PGL, ref)
        mk(tree, cfg, phase + '-D4', 'fixtures file: top-level return 0 after line 1', [('.fixtures', fx[0] + '\n', fx[0] + '\nreturn 0\n')], [], WL, ref)
        if phase == 'AFTER':
            mk(tree, cfg, 'RES', 'transient: git -C . -c include.path=<file> add -A (declared residual)', [('.fixtures', fx[li], fx[li][:col] + 'git -C . -c include.path=' + INC + fx[li][col + 3:])], [], 'GREEN', ref)
            mk(tree, cfg, 'G', 'HOME .config/git/ignore *.py', [], ['HOME=' + g('home_ign')], 'GREEN', ref)
            mk(tree, cfg, 'G', 'DO cell', [], ['GIT_CONFIG_GLOBAL=' + g('do', 'safe.cfg'), 'GIT_TEST_ASSUME_DIFFERENT_OWNER=1'], 'GREEN', ref)
            mk(tree, cfg, 'AFTER-1', 'odd/pipe pre-created (fixtures exit 2)', [('.fixtures', FIFO, ': > "$CTL/odd/pipe"\n' + FIFO)], [], WL, ref)
            mk(tree, cfg, 'P', 'W2: incomplete-window exit removed + odd/pipe', [('.harness', '  echo "!! CONTROL NOT EXERCISED ($1): $_fw_why; nothing was built, so no control was run" >&2\n  exit 2\n', '  echo "!! CONTROL NOT EXERCISED ($1): $_fw_why; nothing was built, so no control was run" >&2\n'), ('.fixtures', FIFO, ': > "$CTL/odd/pipe"\n' + FIFO)], [], 'CONTROL FAILED (no control runs over an incomplete fixture build window)', ref)
            for nm, new in (('nounset', 'set -eo pipefail'), ('pipefail', 'set -eu'), ('errexit', 'set -uo pipefail')):
                mk(tree, cfg, 'P', 'W: prelude drops ' + nm, [('.harness', "    printf 'set -euo pipefail\\n'\n", "    printf '%s\\n'\n" % new)], [], WL, ref)
open(os.path.join(OUT, 'jobs.tsv'), 'w').write('\n'.join(jobs) + '\n'); print(len(jobs), 'jobs')
```

`c8/eval8.sh`:

```sh
#!/bin/bash
# Verdicts from c6/raw.tsv: G/R/RES need rc 0, no control line, PASSED and P equal to the config's reference.
OUT="${K2_CORPUS_OUT:-$(cd "$(dirname "$0")" && pwd)}"
while IFS=$'\t' read -r tag cfg kind label rc ctl passed hit ref; do
  [ "$tag" != HARNESS-ERROR ] || { echo "HARNESS-ERROR	$cfg"; continue; }
  reftag=$(awk -F'\t' -v r="${ref#ref=}" -v c="$cfg" '$2==c && (($3=="REF"&&r=="REF-"c)||($3=="REF5"&&r=="REF5-"c)){print $1}' "$OUT/raw.tsv" | head -1)
  green=0; [ "$rc" = rc=0 ] && [ "$ctl" = ctl=0 ] && [ "$passed" = passed=1 ] && green=1
  same=0; [ -n "$reftag" ] && cmp -s "$OUT/pd/$tag" "$OUT/pd/$reftag" && [ -s "$OUT/pd/$tag" ] && same=1
  nd=$(diff "$OUT/pd/$reftag" "$OUT/pd/$tag" 2>/dev/null | /usr/bin/grep -c '^[<>]')
  case "$kind" in
    REF|REF5) [ $green -eq 1 ] && v=PASS || v=FAIL ;;
    G|R) [ $green -eq 1 ] && [ $same -eq 1 ] && v=PASS || v=FAIL ;;
    BEFORE|BEFORE-2) if [ $green -eq 1 ] && [ $same -eq 0 ]; then v=SILENT-WRONG; elif [ $green -eq 1 ]; then v=GREEN-SAME; else v=RED; fi ;;
    BEFORE-1) v="OBSERVED" ;;
    BEFORE-D2|BEFORE-D4) if [ $green -eq 1 ] && [ $same -eq 0 ]; then v=SILENT-WRONG; elif [ $green -eq 1 ]; then v=GREEN-SAME; else v="RED(ctl=${ctl#ctl=})"; fi ;;
    AFTER-D2) [ "$hit" = hit=1 ] && v=PASS || v=FAIL ;;
    AFTER-D4) [ "$hit" = hit=1 ] && [ "$ctl" = ctl=1 ] && v=PASS || v=FAIL ;;
    AFTER-1) [ "$hit" = hit=1 ] && [ "$rc" = rc=2 ] && [ "$ctl" = ctl=1 ] && v=PASS || v=FAIL ;;
    P) if [ "$hit" = hit=1 ] && { case "$label" in W:*) [ "$ctl" = ctl=1 ];; *) true;; esac; }; then v=PASS; else v=FAIL; fi ;;
    RES) if [ $green -eq 1 ] && [ $same -eq 0 ]; then v=RES-SILENT-WRONG; elif [ $green -eq 1 ]; then v=RES-GREEN-SAME; else v=RES-RED; fi ;;
    *) [ "$hit" = hit=1 ] && v=PASS || v=FAIL ;;
  esac
  printf '%s\t%s\t%s\t%s\t%s\t%s\tPdiff=%s\n' "$v" "$cfg" "$kind" "$label" "$tag" "$rc" "$nd"
done < "$OUT/raw.tsv"
```
