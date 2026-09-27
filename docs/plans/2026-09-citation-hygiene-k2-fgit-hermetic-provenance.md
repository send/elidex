# K2 fixture git — provenance companion

This file is the provenance for `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md`, called "the
design memo" below. It holds:
- the measurements behind that memo's decisions (§A);
- what became of the replaced branch (§B);
- the premises found false (§C);
- every plan-review round's dispositions (§D).

The design memo keeps only its live decisions. Section references here are to the design memo unless a
file is named.

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

**Scripts.** Scratch `…/scratchpad/fgit/corpus/gen.py` and `cell.sh`, reproduced in §E so that a reviewer
can re-run them.

**Subject.** Prototype `p4`, a `git clone --local` of `e8f78896` plus exactly design memo §3–§5:
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

**Commands.** Scripts are in §E; paths are relative to `…/scratchpad/fgit/`:
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

### §D.0 Terminators

**Round 5 (current).** Round 5 is a **focused** re-review of the draft 4 → 5 delta, by Ax2, Ax3 and
Ax5. It converges if and only if no IMP arrives with a failing corpus cell. A finding that does come
with a failing cell is fixed the same way as in round 4: drive p5 to pass the cell, then re-derive the
memo text from the result.

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
| Ax3 MIN: R4 ground, errexit wording, "no measured way", cost sentence | R4 is restated as a class, with no ground argued beyond the launch slot. The errexit text now describes only what p4 does. "No measured way" is deleted along with the differential. The cost sentence is rewritten from the §A.6 data | design memo §4, §5.2 |
| Ax5 IMP: C0a had no seam (parent §5 is live) | **C0a withdrawn**. Only the banner (C0b) remains; its site list is derived by rule and command, giving **seven** sites (round 3 said five) | design memo §8.2 |
| Ax5 IMP: the wire-reference convention resolves 8 references wrongly | the convention is **dropped**. A derived table resolves each reference (8 parent, 1 A-i, plus the qualified ones) | design memo §8.3 |
| Ax4/Ax5: the umbrella memo table lacked a row for the companion | the companion row was added in draft 3; draft 4 keeps it. The rounds companion is moot, since C0a is withdrawn | umbrella |
| Ax3/Ax5: `#11-k2-fgit-machine-files` held two gaps | **withdrawn at create time**: (a) becomes the declared blind spot R3; (b) is the invocation-convention slot's own gap, which that slot is narrowed to. Own new deferrals: 0 | design memo §5.2, §7 |
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
---

## §E Appendix — the corpus scripts (as run)

### §E.1 The draft-4 full corpus

`gen.py`:

```python
#!/usr/bin/env python3
"""Generate the adversarial corpus jobs for prototype p4. Usage: gen.py <p4 dir> <out dir>"""
import os, re, sys, shlex
P4, OUT = sys.argv[1], sys.argv[2]
T = os.path.join(P4, '.claude/tools')
W = 'webref-generic-core-trip-wire'
parts = ['', '.controls', '.harness', '.fixtures', '.mutations']
fx = open(os.path.join(T, W + '.fixtures.sh')).read().split('\n')
hn = open(os.path.join(T, W + '.harness.sh')).read()
CFGS = {  # name: (shell, git prefix dir, real git)
  'b53-g255': ('/opt/homebrew/bin/bash', '/opt/homebrew/bin', '/opt/homebrew/bin/git'),
  'b32-g255': ('/bin/bash', '/opt/homebrew/bin', '/opt/homebrew/bin/git'),
  'b53-g254': ('/opt/homebrew/bin/bash', '/usr/bin', '/usr/bin/git'),
  'b32-g254': ('/bin/bash', '/usr/bin', '/usr/bin/git'),
}
# ---- call-site classes: first line per class, which _fgit occurrence on it
classes = [
 ('init',            r'_fgit init'),
 ('add -A',          r'_fgit add -A'),
 ('add <path>',      r'_fgit add (?!-)'),
 ('add -f',          r'_fgit add -f'),
 ('commit',          r'_fgit -c user\.name=w -c user\.email=w@e commit'),
 ('checkout',        r'_fgit checkout'),
 ('hash-object',     r'_fgit hash-object --stdin'),
 ('hash-object -w',  r'_fgit hash-object -w (?!-)'),
 ('hash-object -w --stdin', r'_fgit hash-object -w --stdin'),
 ('update-index',    r'_fgit update-index'),
 ('replace',         r'_fgit replace'),
 ('rev-parse',       r'_fgit rev-parse'),
 ('symbolic-ref',    r'_fgit symbolic-ref'),
]
sites = []
for name, rx in classes:
    for i, l in enumerate(fx):
        if l.lstrip().startswith('#'): continue
        m = re.search(rx, l)
        if m:
            sites.append((name, i, m.start())); break
    else:
        sites.append((name, None, None))
def spellings(realgit):
    return [
     ('bare git',             'git', 'R'),
     ('_git',                 '_git', 'R'),
     ('command git',          'command git', 'R'),
     ('env git',              'env git', 'R'),
     ('env -i PATH="$PATH" git', 'env -i PATH="$PATH" git', 'R'),
     ('PATH=fakegit:$PATH git', 'PATH="$CTL/fakegit:$PATH" git', 'R'),
     ('hash -p + git',        'hash -p %s git && git' % realgit, 'R'),
     ('eval $_REAL_GIT',      'eval "$_REAL_GIT"', 'R'),
     ('absolute path',        realgit, 'R'),
     ('git -c override',      'git -c core.excludesFile=/dev/null', 'R'),
     ('env -i literal PATH',  'env -i PATH=/usr/bin:/bin git', 'RES'),
     ('env -i absolute',      'env -i %s' % realgit, 'RES'),
    ]
H = os.path.join(OUT, 'h')
g = lambda *a: os.path.join(H, *a)
GROWS = [
 ('clean', []),
 ('HOME .gitconfig excludesFile *.py', ['HOME=' + g('home_ex')]),
 ('HOME empty .config/git/ignore', ['HOME=' + g('home_empty')]),
 ('HOME .config/git/ignore *.py', ['HOME=' + g('home_ign')]),
 ('XDG git/ignore *.py', ['HOME=' + g('plain'), 'XDG_CONFIG_HOME=' + g('xdg_ign')]),
 ('XDG git/attributes wte', ['HOME=' + g('plain'), 'XDG_CONFIG_HOME=' + g('xdg_attr')]),
 ('XDG git/config excludesFile', ['HOME=' + g('plain'), 'XDG_CONFIG_HOME=' + g('xdg_cfg')]),
 ('XDG git/config init.templateDir', ['HOME=' + g('plain'), 'XDG_CONFIG_HOME=' + g('xdg_tpl')]),
 ('GIT_TEMPLATE_DIR exclude', ['GIT_TEMPLATE_DIR=' + g('tpl_ex')]),
 ('GIT_TEMPLATE_DIR config', ['GIT_TEMPLATE_DIR=' + g('tpl_cfg')]),
 ('GIT_TEMPLATE_DIR HEAD', ['GIT_TEMPLATE_DIR=' + g('tpl_head')]),
 ('GIT_TEMPLATE_DIR hooks', ['GIT_TEMPLATE_DIR=' + g('tpl_hook')]),
 ('GIT_CONFIG_GLOBAL=/dev/null', ['GIT_CONFIG_GLOBAL=/dev/null']),
 ('GIT_CONFIG_GLOBAL=<all five>', ['GIT_CONFIG_GLOBAL=' + g('all5.cfg')]),
 ('GIT_CONFIG_SYSTEM=<all five>', ['GIT_CONFIG_SYSTEM=' + g('all5.cfg')]),
 ('COUNT init.defaultBranch', ['GIT_CONFIG_COUNT=1', 'GIT_CONFIG_KEY_0=init.defaultBranch', 'GIT_CONFIG_VALUE_0=hostile']),
 ('COUNT core.excludesFile', ['GIT_CONFIG_COUNT=1', 'GIT_CONFIG_KEY_0=core.excludesFile', 'GIT_CONFIG_VALUE_0=' + g('pyglob')]),
 ('COUNT core.hooksPath', ['GIT_CONFIG_COUNT=1', 'GIT_CONFIG_KEY_0=core.hooksPath', 'GIT_CONFIG_VALUE_0=' + g('hooks')]),
 ('COUNT attributesFile+filter', ['GIT_CONFIG_COUNT=2', 'GIT_CONFIG_KEY_0=core.attributesFile', 'GIT_CONFIG_VALUE_0=' + g('attr_filter'), 'GIT_CONFIG_KEY_1=filter.zap.clean', 'GIT_CONFIG_VALUE_1=true']),
 ('COUNT commit.gpgSign+false', ['GIT_CONFIG_COUNT=2', 'GIT_CONFIG_KEY_0=commit.gpgSign', 'GIT_CONFIG_VALUE_0=true', 'GIT_CONFIG_KEY_1=gpg.program', 'GIT_CONFIG_VALUE_1=/usr/bin/false']),
 ('PARAMETERS init.defaultbranch', ["GIT_CONFIG_PARAMETERS='init.defaultbranch'='hostile'"]),
 ('PARAMETERS core.excludesfile', ["GIT_CONFIG_PARAMETERS='core.excludesfile'='%s'" % g('pyglob')]),
 ('HOME .gitconfig init.defaultBranch', ['HOME=' + g('home_br')]),
 ('HOME .gitconfig core.hooksPath', ['HOME=' + g('home_hook')]),
 ('HOME .gitconfig attributesFile+filter', ['HOME=' + g('home_filter')]),
 ('HOME .gitconfig gpgSign+false', ['HOME=' + g('home_gpg')]),
 ('GIT_DIR=<other>', ['GIT_DIR=' + g('other', '.git')]),
 ('GIT_INDEX_FILE=<other>', ['GIT_INDEX_FILE=' + g('other', 'idx')]),
 ('GIT_WORK_TREE=<other>', ['GIT_WORK_TREE=' + g('other')]),
]
PINS = [  # (name, harness old, harness new, needle)
 ('P-a env config in _FGIT_ENV', '"GIT_TEMPLATE_DIR=$_FGIT_VOID")', '"GIT_TEMPLATE_DIR=$_FGIT_VOID" GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=a.b GIT_CONFIG_VALUE_0=c)', "CONTROL FAILED (the fixture git reads configuration only from the fixture's own config file)"),
 ('P-b drop NOSYSTEM', ' GIT_CONFIG_NOSYSTEM=1 GIT_ATTR_NOSYSTEM=1', ' GIT_ATTR_NOSYSTEM=1', 'CONTROL FAILED (the fixture git has no system or global layer outside the void)'),
 ('P-b drop ATTR_NOSYSTEM', ' GIT_CONFIG_NOSYSTEM=1 GIT_ATTR_NOSYSTEM=1', ' GIT_CONFIG_NOSYSTEM=1', 'CONTROL FAILED (the fixture git has no system or global layer outside the void)'),
 ('P-b drop HOME', '"PATH=$PATH" "HOME=$_FGIT_VOID" ', '"PATH=$PATH" ', 'CONTROL FAILED (the fixture git has no system or global layer outside the void)'),
 ('P-c void non-empty', 'mkdir "$_FGIT_VOID" || exit 2', 'mkdir "$_FGIT_VOID" || exit 2; : > "$_FGIT_VOID/planted"', "CONTROL FAILED (nothing is written into the fixture git's void)"),
 ('P-d drop TEMPLATE_DIR', ' "GIT_TEMPLATE_DIR=$_FGIT_VOID")', ')', 'CONTROL FAILED (the fixture git copies no template)'),
 ('P-e drop PATH', '_FGIT_ENV=("PATH=$PATH" ', '_FGIT_ENV=(', 'CONTROL FAILED (the fixture git is the git the wire reads with)'),
 ('detect: _fgit inherits', '_fgit() { command env -i ', '_fgit() { command env ', 'CONTROL FAILED (every fixture git call goes through _fgit)'),
 ('liveness: no GIT_TRACE', '  export GIT_TRACE="$_FD_TRACE"\n', '', 'CONTROL NOT EXERCISED (the fixture-build bypass detector sees a bypass)'),
 ('liveness: no PATH shim', '  PATH="$_FD_DIR/bin:$PATH"\n', '', 'CONTROL NOT EXERCISED (the fixture-build bypass detector sees a bypass)'),
]
jobs = []
n = 0
def mk(cfg, kind, label, edits, env, expect):
    global n
    n += 1
    tag = 'c%04d' % n
    for p in parts:
        src = open(os.path.join(T, W + p + '.sh')).read()
        for (pp, old, new) in edits:
            if pp == p:
                assert src.count(old) >= 1, (label, old)
                src = src.replace(old, new, 1)
        dst = os.path.join(T, W + '.' + tag + p + '.sh')
        open(dst, 'w').write(src); os.chmod(dst, 0o755)
    sh, gp, rg = CFGS[cfg]
    jobs.append('\x1f'.join([tag, cfg, kind, label, sh, gp, ' '.join(shlex.quote(e) for e in env), expect]))
for cfg, (sh, gp, rg) in CFGS.items():
    for name, env in GROWS:
        mk(cfg, 'G', name, [], env, 'GREEN')
    for (cls, li, col) in sites:
        for sname, sp, kind in spellings(rg):
            line = fx[li]
            new = line[:col] + sp + line[col + len('_fgit'):]
            mk(cfg, kind, cls + ' × ' + sname, [('.fixtures', line, new)], [], 'CONTROL FAILED (every fixture git call goes through _fgit)')
    for name, old, new, needle in PINS:
        mk(cfg, 'P', name, [('.harness', old, new)], [], needle)
open(os.path.join(OUT, 'jobs.tsv'), 'w').write('\n'.join(jobs) + '\n')
print(len(jobs), 'jobs;', 'sites:', [(c, li + 1 if li is not None else None) for c, li, _ in sites])
```

`cell.sh`:

```sh
#!/bin/bash
# one corpus cell: tag cfg kind label shell gitprefix env expect
IFS=$'\x1f' read -r tag cfg kind label sh gp envs expect <<< "$1"
P4=/private/tmp/claude-501/-Users-kazuaki-repos-send-sh-elidex/d366a8a7-1cd3-4517-9fcf-9e5d66cdc77f/scratchpad/fgit/p4
OUT=/private/tmp/claude-501/-Users-kazuaki-repos-send-sh-elidex/d366a8a7-1cd3-4517-9fcf-9e5d66cdc77f/scratchpad/fgit/corpus
[ -n "$tag" ] && [ -x "$sh" ] && [ -n "$expect" ] || { printf "HARNESS-ERROR\t%s\n" "$1"; exit 0; }
log=$OUT/logs/$tag.log; mkdir -p $OUT/tmp/$tag
eval "set -- $envs"
( cd $P4 && env TMPDIR=$OUT/tmp/$tag PATH="$gp:/usr/bin:/bin:/opt/homebrew/bin" "$@" "$sh" ".claude/tools/webref-generic-core-trip-wire.$tag.sh" ) > $log 2>&1
rc=$?
[ $rc -ne 127 ] && [ $rc -ne 126 ] || { printf "HARNESS-ERROR\t%s\trc=%s\n" "$tag" "$rc"; exit 0; }
ncf=$(/usr/bin/grep -c 'CONTROL FAILED\|CONTROL NOT EXERCISED' $log); pass=$(/usr/bin/grep -c 'trip-wire PASSED' $log)
if [ "$expect" = GREEN ]; then
  if [ $rc -eq 0 ] && [ $ncf -eq 0 ] && [ $pass -eq 1 ]; then v=PASS; else v=FAIL; fi
else
  if [ $rc -ne 0 ] && /usr/bin/grep -qF "$expect" $log; then v=PASS; else v=FAIL; fi
fi
[ "$kind" = RES ] && { [ "$v" = PASS ] && v=RES-RED || v=RES-GREEN; }
printf '%s\t%s\t%s\t%s\t%s\trc=%s\tctl=%s\n' "$v" "$cfg" "$kind" "$label" "$tag" "$rc" "$ncf"
rm -rf $OUT/tmp/$tag
```

### §E.2 The draft-5 subset (as run in run 3)

`corpus5/gen5.py`:

```python
#!/usr/bin/env python3
"""Representative corpus for draft 5 (prototype p5) + before-cells on p4.
Usage: gen5.py <scratch fgit dir>. Writes corpus5/jobs.tsv."""
import os, re, sys, shlex
SP = sys.argv[1]; OUT = os.path.join(SP, 'corpus5'); H = os.path.join(OUT, 'h')
W = 'webref-generic-core-trip-wire'; parts = ['', '.controls', '.harness', '.fixtures', '.mutations']
CFGS = {'b53-g255': ('/opt/homebrew/bin/bash', '/opt/homebrew/bin'), 'b32-g255': ('/bin/bash', '/opt/homebrew/bin'),
        'b53-g254': ('/opt/homebrew/bin/bash', '/usr/bin'), 'b32-g254': ('/bin/bash', '/usr/bin')}
L = dict(setup="the fixture-build bypass detector is installed",
 gitl="the build's PATH shim for git records a call", envl="the build's PATH shim for env records a call",
 trl="the build's GIT_TRACE2_EVENT records an inheriting git", cfgl="the build's trace2 target, set through its global configuration, records a git that drops the trace variables",
 poil="the build's poisoned configuration stops a git that drops the trace variables and the global configuration",
 git="no fixture calls git through PATH instead of _fgit", env="no fixture calls env through PATH instead of _fgit",
 cfg="no fixture git writes the build's trace2 events",
 pa="the fixture git reads configuration only from the fixture's own config file", pal="this git reports a non-local configuration scope",
 pb="the fixture git has no system or global layer outside the void", pbl="this git names its system files through git var",
 pc="nothing is written into the fixture git's void", pd="the fixture git copies no template", pe="the fixture git is the git the wire reads with")
F = lambda k: 'CONTROL FAILED (%s)' % L[k]
N = lambda k: 'CONTROL NOT EXERCISED (%s)' % L[k]
g = lambda *a: os.path.join(H, *a)
jobs = []; n = [0]
def mk(tree, cfg, kind, label, edits, env, expect):
    n[0] += 1; tag = 'd%04d' % n[0]; T = os.path.join(tree, '.claude/tools')
    for p in parts:
        src = open(os.path.join(T, W + p + '.sh')).read()
        for (pp, old, new) in edits:
            if pp == p:
                assert src.count(old) >= 1, (label, p, old[:60]); src = src.replace(old, new, 1)
        dst = os.path.join(T, W + '.' + tag + p + '.sh'); open(dst, 'w').write(src); os.chmod(dst, 0o755)
    sh, gp = CFGS[cfg]
    jobs.append('\x1f'.join([tag, cfg, kind, label, sh, gp, ' '.join(shlex.quote(e) for e in env), expect, tree]))
P5 = os.path.join(SP, 'p5'); P4 = os.path.join(SP, 'p4')
fx = open(os.path.join(P5, '.claude/tools', W + '.fixtures.sh')).read().split('\n')
classes = [('init', r'_fgit init'), ('add -A', r'_fgit add -A'), ('add <path>', r'_fgit add (?!-)'), ('add -f', r'_fgit add -f'),
 ('commit', r'_fgit -c user\.name=w -c user\.email=w@e commit'), ('checkout', r'_fgit checkout'), ('hash-object', r'_fgit hash-object --stdin'),
 ('hash-object -w', r'_fgit hash-object -w (?!-)'), ('hash-object -w --stdin', r'_fgit hash-object -w --stdin'), ('update-index', r'_fgit update-index'),
 ('replace', r'_fgit replace'), ('rev-parse', r'_fgit rev-parse'), ('symbolic-ref', r'_fgit symbolic-ref')]
sites = {}
for name, rx in classes:
    for i, l in enumerate(fx):
        if l.lstrip().startswith('#'): continue
        m = re.search(rx, l)
        if m: sites[name] = (i, m.start()); break
def site_edit(cls, sp):
    i, col = sites[cls]; line = fx[i]
    return [('.fixtures', line, line[:col] + sp + line[col + len('_fgit'):])]
RG = '"$_REAL_GIT_BIN"'
SPELL = [  # (name, spelling at the add -A site, expected needle)
 ('bare git', 'git', F('git')), ('_git', '_git', F('git')), ('command git', 'command git', F('git')),
 ('env git', 'env git', F('env')), ('env -i PATH="$PATH" git', 'env -i PATH="$PATH" git', F('env')),
 ('env - PATH=/usr/bin:/bin git', 'env - PATH=/usr/bin:/bin git', F('env')),
 ('env -i PATH=/usr/bin:/bin git', 'env -i PATH=/usr/bin:/bin git', F('env')),
 ('env -i <abs git>', 'env -i ' + RG, F('env')),
 ('PATH=fakegit:$PATH git', 'PATH="$CTL/fakegit:$PATH" git', 'CONTROL FAILED (no fixture '),
 ('hash -p <abs> git && git', 'hash -p ' + RG + ' git && git', F('cfg')), ('eval "$_REAL_GIT"', 'eval "$_REAL_GIT"', F('cfg')),
 ('absolute path', RG, F('cfg')), ('git -c core.excludesFile=/dev/null', 'git -c core.excludesFile=/dev/null', F('git')),
 ('GIT_TRACE= <abs>', 'GIT_TRACE= ' + RG, F('cfg')), ('GIT_TRACE=0 eval "$_REAL_GIT"', 'GIT_TRACE=0 eval "$_REAL_GIT"', F('cfg')),
 ('GIT_TRACE=/dev/null <abs>', 'GIT_TRACE=/dev/null ' + RG, F('cfg')),
 ('env -u GIT_TRACE <abs>', 'env -u GIT_TRACE ' + RG, F('env')),
 ('sh -c unset GIT_TRACE GIT_TRACE2_EVENT; exec <abs>', 'sh -c \'unset GIT_TRACE GIT_TRACE2_EVENT; exec "$0" "$@"\' ' + RG, F('cfg')),
 ('sh -c unset GIT_TRACE GIT_TRACE2_EVENT GIT_CONFIG_GLOBAL; exec <abs> (poison)', 'sh -c \'unset GIT_TRACE GIT_TRACE2_EVENT GIT_CONFIG_GLOBAL; exec "$0" "$@"\' ' + RG, 'CONTROL NOT EXERCISED ('),
]
RES = [('/usr/bin/env -i PATH=/usr/bin:/bin git', '/usr/bin/env -i PATH=/usr/bin:/bin git'), ('/usr/bin/env -i <abs git>', '/usr/bin/env -i ' + RG)]
HN = '.harness'
REC = [  # the representative record set: (name, part, old, new, needle)
 ('setup fails (U5)', HN, '_FD_DIR="$SCRATCH/fgit-detect"', '_FD_DIR="/nonexistent-k2/fgit-detect"', N('setup')),
 ('git shim marks nothing', HN, '''printf '#!/bin/sh\\n: >> %s\\nexit 97\\n' "$(_shq "$_FD_GITMARK")" > "$_FD_DIR/bin/git"''', '''printf '#!/bin/sh\\nexit 97\\n' > "$_FD_DIR/bin/git"''', N('gitl')),
 ('env shim marks nothing', HN, '''printf '#!/bin/sh\\n: >> %s\\nexit 97\\n' "$(_shq "$_FD_ENVMARK")" > "$_FD_DIR/bin/env"''', '''printf '#!/bin/sh\\nexit 97\\n' > "$_FD_DIR/bin/env"''', N('envl')),
 ('no GIT_TRACE2_EVENT', HN, '  export GIT_TRACE2_EVENT="$_FD_T2"\n', '', N('trl')),
 ('no global config', HN, '  export GIT_CONFIG_GLOBAL="$_FD_CFG"\n', '', N('cfgl')),
 ('no poison', HN, '  export GIT_CONFIG_COUNT=bogus\n', '', N('poil')),
 ('_fgit inherits (-i dropped)', HN, '_fgit() { "$_FGIT_ENVBIN" -i ', '_fgit() { "$_FGIT_ENVBIN" ', F('cfg')),
 ('P-a env config in _FGIT_ENV', HN, '"GIT_TEMPLATE_DIR=$_FGIT_VOID")', '"GIT_TEMPLATE_DIR=$_FGIT_VOID" GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=a.b GIT_CONFIG_VALUE_0=c)', F('pa')),
 ('P-a liveness probe loses -c', HN, '_fgit -c a.b=c config --list', '_fgit config --list', N('pal')),
 ('P-b drop NOSYSTEM', HN, ' GIT_CONFIG_NOSYSTEM=1 GIT_ATTR_NOSYSTEM=1 "GIT', ' GIT_ATTR_NOSYSTEM=1 "GIT', F('pb')),
 ('P-b liveness probe keeps NOSYSTEM', HN, 'GIT_CONFIG_NOSYSTEM=0 GIT_ATTR_NOSYSTEM=0 git var', 'git var', N('pbl')),
 ('P-c void non-empty', HN, 'mkdir "$_FGIT_VOID" || exit 2', 'mkdir "$_FGIT_VOID" || exit 2; : > "$_FGIT_VOID/planted"', F('pc')),
 ('P-d drop TEMPLATE_DIR', HN, ' "GIT_TEMPLATE_DIR=$_FGIT_VOID")', ')', F('pd')),
 ('P-e GIT_EXEC_PATH injected', HN, '_FGIT_ENV=("PATH=$PATH" ', '_FGIT_ENV=("PATH=$PATH" GIT_EXEC_PATH=/nonexistent-k2 ', F('pe')),
]
GROWS = [('clean', []), ('HOME .config/git/ignore *.py', ['HOME=' + g('home_ign')]), ('GIT_CONFIG_GLOBAL=/dev/null', ['GIT_CONFIG_GLOBAL=/dev/null']),
 ('COUNT core.excludesFile', ['GIT_CONFIG_COUNT=1', 'GIT_CONFIG_KEY_0=core.excludesFile', 'GIT_CONFIG_VALUE_0=' + g('pyglob')]),
 ('GIT_CONFIG_GLOBAL=<all five>', ['GIT_CONFIG_GLOBAL=' + g('all5.cfg')]), ('COUNT init.defaultBranch', ['GIT_CONFIG_COUNT=1', 'GIT_CONFIG_KEY_0=init.defaultBranch', 'GIT_CONFIG_VALUE_0=hostile']),
 ('caller GIT_TRACE=<abs file>', ['GIT_TRACE=' + g('callertrace')]), ('caller GIT_TRACE2_EVENT=<abs file>', ['GIT_TRACE2_EVENT=' + g('callertrace2')])]
for cfg in CFGS:
    for name, env in GROWS: mk(P5, cfg, 'G', name, [], env, 'GREEN')
    for cls in sites: mk(P5, cfg, 'R', cls + ' × bare git', site_edit(cls, 'git'), [], F('git'))
    for name, sp, needle in SPELL: mk(P5, cfg, 'R', 'add -A × ' + name, site_edit('add -A', sp), [], needle)
    mk(P5, cfg, 'R', 'add -A × GIT_TRACE= <abs> + caller HOME ignore *.py', site_edit('add -A', 'GIT_TRACE= ' + RG), ['HOME=' + g('home_ign')], F('cfg'))
    for name, sp in RES: mk(P5, cfg, 'RES', 'add -A × ' + name, site_edit('add -A', sp), [], F('env'))
    for name, part, old, new, needle in REC: mk(P5, cfg, 'P', name, [(part, old, new)], [], needle)
    mk(P5, cfg, 'INFO', 'P-e drop PATH (machine-conditional)', [(HN, '_FGIT_ENV=("PATH=$PATH" ', '_FGIT_ENV=(')], [], F('pe'))
# before-cells on p4 (the draft-4 prototype), two configs
fx4 = open(os.path.join(P4, '.claude/tools', W + '.fixtures.sh')).read().split('\n')
i, col = sites['add -A']; line4 = fx4[i]
for cfg in ('b53-g255', 'b32-g254'):
    mk(P4, cfg, 'BEFORE', 'p4: setup fails (U5)', [(HN, '_FD_DIR="$SCRATCH/fgit-detect"', '_FD_DIR="/nonexistent-k2/fgit-detect"')], [], 'CONTROL NOT EXERCISED (the fixture-build bypass detector sees a bypass)')
    for name, sp in [('GIT_TRACE= <abs>', 'GIT_TRACE= ' + RG), ('env -i PATH=/usr/bin:/bin git', 'env -i PATH=/usr/bin:/bin git')]:
        mk(P4, cfg, 'BEFORE', 'p4: add -A × ' + name + ' + caller HOME ignore *.py', [('.fixtures', line4, line4[:col] + sp + line4[col + 5:])], ['HOME=' + g('home_ign')], 'CONTROL FAILED (every fixture git call goes through _fgit)')
open(os.path.join(OUT, 'jobs.tsv'), 'w').write('\n'.join(jobs) + '\n')
print(len(jobs), 'jobs')
```

`corpus5/cell.sh`:

```sh
#!/bin/bash
# one corpus cell: tag cfg kind label shell gitprefix env expect [tree]
IFS=$'\x1f' read -r tag cfg kind label sh gp envs expect tree <<< "$1"
OUT=/private/tmp/claude-501/-Users-kazuaki-repos-send-sh-elidex/d366a8a7-1cd3-4517-9fcf-9e5d66cdc77f/scratchpad/fgit/corpus5
[ -n "$tag" ] && [ -x "$sh" ] && [ -n "$expect" ] && [ -d "$tree" ] || { printf "HARNESS-ERROR\t%s\n" "$1"; exit 0; }
log=$OUT/logs/$tag.log; mkdir -p $OUT/tmp/$tag
eval "set -- $envs"
( cd "$tree" && env TMPDIR=$OUT/tmp/$tag PATH="$gp:/usr/bin:/bin:/opt/homebrew/bin" "$@" "$sh" ".claude/tools/webref-generic-core-trip-wire.$tag.sh" ) > $log 2>&1
rc=$?
[ $rc -ne 127 ] && [ $rc -ne 126 ] || { printf "HARNESS-ERROR\t%s\trc=%s\n" "$tag" "$rc"; exit 0; }
ncf=$(/usr/bin/grep -c 'CONTROL FAILED\|CONTROL NOT EXERCISED' $log); pass=$(/usr/bin/grep -c 'trip-wire PASSED' $log)
green=0; [ $rc -eq 0 ] && [ $ncf -eq 0 ] && [ $pass -eq 1 ] && green=1
hit=0; [ $rc -ne 0 ] && /usr/bin/grep -qF -- "$expect" $log && hit=1
case "$kind" in
  G) [ $green -eq 1 ] && v=PASS || v=FAIL ;;
  RES) if [ $green -eq 1 ]; then v=RES-GREEN; elif [ $hit -eq 1 ]; then v=RES-RED; else v=FAIL; fi ;;
  *) [ $hit -eq 1 ] && v=PASS || v=FAIL ;;
esac
arms=$(/usr/bin/grep -oE 'CONTROL [A-Z ]*\((no fixture|the build.s|the fixture-build|the fixture git|this git|nothing is written)[^)]*\)' $log | sort -u | tr '\n' ';')
printf '%s\t%s\t%s\t%s\t%s\trc=%s\tctl=%s\t%s\n' "$v" "$cfg" "$kind" "$label" "$tag" "$rc" "$ncf" "$arms"
rm -rf $OUT/tmp/$tag
```
