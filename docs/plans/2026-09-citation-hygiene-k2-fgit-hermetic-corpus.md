# K2 fixture git — the corpus and the mutation records

This file holds the corpus — the evidence behind the design — and the mutation records derived from it,
for `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md` ("the design memo"). It was split out of
the design memo's §6 as a touch-time split (`35dc1153`), before the design memo grew past 1000 lines.
At the split the text was unchanged, except that five references to other sections of the design memo
were made to name it. Later drafts changed it: §6.1 (draft 11's record plan, revised by drafts 12–21, then the fix round's and the later review rounds' record changes) and one
sentence under "Records" were added after the split. The section number §6 is kept, so an earlier
reference to "memo §6" resolves here.

"The companion" below is the provenance companion,
`docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-provenance.md`; `…-reviews.md` is the review
record, `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-reviews.md`, and `…-pr527.md` is PR #527's
record, `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-pr527.md`.

---

## §0 Spec coverage map

**No spec surface** — this file records corpus cells and mutation records only. It has the same
by-design shape as the design memo's §2.5: `preflight.py` exits 1 here because no table follows this
heading, and the repo-wide preflight command runs over every tracked plan memo.

---

## §6 The corpus — evidence, and the source of the records

**Oracle, recast.** Under a hostile caller, a bypass spelling no longer has to go red. It has to go
**green with P identical to the clean build's P**. P is dumped per fixture repo by a **prototype-only**
hook (`K2_CORPUS_PDUMP`), which is not part of the design. It is passed into the window as a plain
assignment, so P-f does not see it. Symlink targets are normalised for `$CTL` (companion §A.9).

**Subjects.** All three are `git clone --local`s of `e8f78896`.
- **p6** carries design memo §3–§4 as in draft 6. It shows the window's own guarantee under hostile callers.
- **p7** is p6 plus draft 7's fixes, including S, which is now deleted.
- **p8** is p7 minus S, plus P-g, the `built` marker, the W reason text, W2 and the per-option checks.
  Draft 8's claims are measured on p8.
- **p9** is p8 plus `_control`'s W2 guard, W3, the options re-check, the named exit causes, the comment
  fixes, and a top-level P-g shape check. It is superseded by p10 on P-g only.
- **p10** is p9 with P-g's census over every git dir (design memo §4). Draft 9's claims are measured on p10.
- **p11** is p10 plus round 9's fixes: W3 without a pipe into `head`, the census opening the declared
  directories, the `HEAD` + `objects`/`commondir` shape, no per-directory fork, and the cause marker.
  Draft 10's claims, and the implementation, are measured against p11.

**Subset and re-run recipe.** Scripts: `git show ff1322bb:docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-provenance.md` §E.3 (scratch `c6/`). Run them from a directory holding the
tree under test as `p6/` and, for the before-cells, `p5d/`:

```sh
python3 c6/gen6.py <dir>                                         # 190 cells
tr '\n' '\0' < c6/jobs.tsv | xargs -0 -n1 -P2 c6/cell6.sh > c6/raw.tsv   # K2_CORPUS_OUT overrides the output dir
c6/eval6.sh > c6/results.tsv                                     # verdicts, P compared per config
```

For X2/X5 on the implementing head, clone that head as `<dir>/p6`. The scripts take no other paths.

**Draft 6's recast corpus** covered 190 cells on p6, across all four configurations. The cell list is
in companion §A.9. It contains:
- **G:** 11 hostile callers, including the DO cell;
- **R:** 11 bypass spellings under a HOME `*.py` ignore, with two of them also run × every hostile
  caller;
- **RLOUD:** `_git`;
- **RES:** 2 cells;
- **P:** the records;
- **INFO:** 2 cells;
- **BEFORE:** cells on p5d.

**Results.** Draft 6's corpus results are in companion §A.9:
- G, R and RES were green with P equal on all four configurations;
- the DO cell was green with P equal, so the read side still sees the caller's configuration;
- round 5's four bypass spellings went from silently wrong on p5 to P-equal on p6;
- the one INFO cell that stayed green is PATH-drop on git 2.54, which is equivalent there, as
  predicted.

**Draft 8's focused cells.** They ran on p7 (before) and p8 (after), on bash 5.3·git 2.55 and bash
3.2·git 2.54. The scripts are in the history (companion §E, `453b7b0f` §E.5), and the full table is
in companion §A.11, which also records their re-run with `HOME` set: unchanged.

| cell | p7 (before) | p8 (after) |
|---|---|---|
| D2: `printf '[include]\n\tpath = <file>' >> .git/config` | rc 0, PASSED, P differs: **silently wrong** | rc 1, P-g |
| D2: `printf '[core]\n\texcludesFile = <file>' >> .git/config` | **silently wrong** | rc 1, P-g |
| D2: `git -C . config core.excludesFile <file>` | **silently wrong** | rc 1, P-g |
| D2: `git config include.path <file>` | red only via S | rc 1, P-g |
| D4: a top-level `return 0` early in the fixtures file | rc 1, **81** control lines: `done` certified the child, not the file | **rc 2, W alone** |
| D4: `$CTL/odd/pipe` pre-created (the fixtures exit 2) | (draft 7: rc 2, W alone) | rc 2, W alone |
| D4 record W2: incomplete-window `exit` removed, plus `odd/pipe` | — | rc 1, W2 printed |
| D4 records: the prelude drops `nounset` / `pipefail` / `errexit` (one each) | — | each rc 2, W alone |
| D3 residual, now a class-(c) example (design memo §0.3): `git -C . -c include.path=<file> add -A` | — | rc 0, PASSED, Pdiff 50: **silent, declared** |
| G: HOME `*.py` ignore; the DO cell | — | green, P equal |
| P-a record with the design memo §5.1-unit label | — | red with the label |

**Draft 9's focused cells.** They ran on p8 (before) and p10 (after), on bash 5.3·git 2.55 and bash
3.2·git 2.54, **with `HOME` set for every cell**. Every row is the same on both configurations. The
scripts are in companion §E.6, and the full table is in companion §A.12.

| cell | p8 (before) | p10 (after) |
|---|---|---|
| E2: `cachedir` built with `--separate-git-dir` plus a persisted `core.excludesFile` | rc 0, PASSED, P differs: **silently wrong** | rc 1, P-g: `cachedir:[.git is not a directory] .gd-cachedir:[a git dir not named .git]` |
| E2: `cachedir` built with `--separate-git-dir` only (a gitfile) | rc 0, PASSED, P differs: **silently wrong** | rc 1, P-g, same two entries |
| E2: a nested repo `zz/inner` with a persisted `core.excludesFile` | green: unseen | rc 1, P-g: `zz/inner:[local file:.git/config core.excludesfile=…]` |
| E2: a hidden top-level repo `.hid`, same key | green: unseen | rc 1, P-g |
| E2: a bare git dir `zbare`, same key | green: unseen | rc 1, P-g: `zbare:[a git dir not named .git]` |
| E3 RO: the exit moved below the first control, plus `odd/pipe` | rc 2: W, plus the first control **misreported** as `green is reachable`; no W2 | **rc 2: W2, then W** |
| E3 ROg: the exit moved below the first control, window complete | green, P equal | green, P equal |
| AR: `_ar=$(( 1/0 ))` after the fixtures file's first line | rc 0, PASSED: **silent** | rc 1, W3 |
| SE1: `set +e` before the fixtures file's `built` line | — | rc 2, W alone: "the fixtures file switched off errexit, nounset or pipefail" |
| W2 record: the incomplete-window `exit` removed, plus `odd/pipe` | — | rc 1, W2 |
| G: HOME `*.py` ignore; the DO cell | — | green, P equal |
| clean | green | green, P equal |

**Draft 10's cells.** They ran on p10 (before) and p11 (after), with `HOME` set for every cell. The
scripts are in companion §E.7, and the full table is in companion §A.13.

| cell | p10 (before) | p11 (after) |
|---|---|---|
| arnoise: `$(( 1/0 ))` plus 200 `[` diagnostics | green, PASSED: **W3 failed open** | rc 1, W3 |
| noise1k: 1000 `[` diagnostics | green, PASSED: **W3 failed open** | rc 1, W3 |
| ar: `$(( 1/0 ))` alone | rc 1, W3 | rc 1, W3 |
| m2h: nested repo under `walk/sub`, outside `core.excludesFile`, gitlinked into `walk` (4 shell × git configs) | rc 0, PASSED, P differs where a reference exists: **silently wrong** | rc 1, P-g `walk/sub/inner:[local file:.git/config core.excludesfile=…]` |
| wtmeta: a linked worktree removed, its `.git/worktrees/` entry left | rc 0, P differs: **silently wrong** | rc 1, P-g `zzw/.git/worktrees/zzw2:[a git dir not named .git]` |
| nr: a `chmod 300` directory under `walk` | rc 1, P-g (census failed) | rc 1, P-g (census failed) |
| rc5: `sh -c 'exit 5'` at the fixtures file's top level | rc 2, W **misnamed** "switched off errexit…" | rc 2, W "the window exited 5 before completing" |
| draft 9's set: `--separate-git-dir`, nested, hidden, bare, RO, ROg, SE1, G, DO, W2 record | — | as draft 9: all PASS |

Both shells gave the same verdict in every row; m2h also ran on bash 5.3·git 2.54 and bash 3.2·git 2.55.

**Records: representative only.** The table and the totals below are the set at `8413a4db`; C6–C12,
the fix round and the review rounds after it change them as §6.1 says.

| label | record | target |
|---|---|---|
| W | the child exits right after the prelude | harness |
| W | the prelude drops `nounset` (`set -eo pipefail`) | harness |
| W | the prelude drops `pipefail` (`set -eu`) | harness |
| W | the prelude drops `errexit` (`set -uo pipefail`) | harness |
| W | a top-level `return 0` early in the fixtures file | **fixtures** |
| W2 | one `harness:` expression with two substitutions: the `exit` removed from `_fgit_window_incomplete_exit`, and the child's `done` marker renamed so the window stays incomplete. A record edits one file, so the draft's "plus the fixtures exit 2" is not expressible (`…-reviews.md` §13) | harness |
| P-a | config appended to `_FGIT_ENV` | harness |
| P-a liveness | the probe loses `-c a.b=c` | harness |
| P-b | drop `GIT_CONFIG_NOSYSTEM` | harness |
| P-b liveness | the probe loses `…NOSYSTEM=0` | harness |
| P-c | plant a file in the void | harness |
| P-d | drop `GIT_TEMPLATE_DIR` | harness |
| P-e | inject `GIT_EXEC_PATH=/nonexistent-k2` | harness |
| P-h | drop `"LC_ALL=C"` from `_FGIT_ENV` | harness |
| W4 | `_seal_apply`'s `chmod` replaced by `false` | harness |
| P-i | drop `GIT_DEFAULT_REF_FORMAT=files` from `_FGIT_ENV` | harness |
| P-f | drop `-i` from the window's `env` | harness |
| P-f | `env -0` fails **with** output and a line on stderr (`{ env -0; echo k2 >&2; false; }`): NOT EXERCISED, red — pins "stdout empty" alone (the fix-delta re-check #2 showed the earlier `{ env -0; false; }` also died on the stderr clause, so it pinned nothing of its own) | harness |
| P-f | `env -0` fails silently (`false`): NOT EXERCISED, red — pins "says why on stderr" in the limitation predicate | harness |
| P-f | `env -0` exits 0 with nothing on stdout and a line on stderr: NOT EXERCISED, red — pins "fails" | harness |
| P-f | the `env` P-f runs is `/nonexistent-k2/env`, which cannot be started (measured exit 127 on bash 5.3, 1 on bash 3.2 inside the wire): NOT EXERCISED, red — pins "runs a command" | harness |
| P-f | `env -0`'s output gains a last record with no NUL after it (`K2X=1`): red — pins reading that record | harness |
| P-j | a relative entry after `$_FGIT_BIN` in the window's `PATH` | harness |
| P-j | the window's `PATH` starts at `/usr/bin` instead of `$_FGIT_BIN` | harness |
| P-g | the extra-lines `grep` reads a missing pattern file (exit 2): red — pins the status check | harness |
| P-g | the missing-lines `grep` reads a missing pattern file (exit 2): red — pins the status check | harness |
| P-c | a fixture writes an entry named only by a newline into the void | **fixtures** |
| P-c | a fixture writes into the void, then makes it unreadable (`chmod 300`) | **fixtures** |
| P-c | a fixture replaces the void with a symlink to an empty directory | **fixtures** |
| P-c | a fixture writes into the void, then makes it readable but not searchable (`chmod 600`) | **fixtures** |
| P-c | a fixture writes into the void and leaves `set -f` on | **fixtures** |
| P-j | the wrapper is written as `gitx`, so the window's `git` is not the pinned one | harness |
| P-g | a bare repo with a persisted `core.excludesFile` whose `HEAD` is a symlink to its branch ref (a form git reads) | **fixtures** |
| P-g | a bare repo with a persisted `core.excludesFile` whose `HEAD` is renamed `head` | **fixtures** |
| P-g | a gitfile named `.GIT` (separate git dir outside the fixture root) | **fixtures** |
| P-g | a fixture adds a second `core.bare = false` (a repeated line) | **fixtures** |
| P-g | a symlink under the fixture root to a bare repo (with a persisted key) outside it | **fixtures** |
| P-g | a symlink under the fixture root to a directory with a bare repo two levels down | **fixtures** |
| P-g | a symlink to a bare repo whose unborn `HEAD` is a dangling symlink (a form git accepts) | **fixtures** |
| P-g | a symlink NAMED `HEAD` to a bare repo outside the fixture root | **fixtures** |
| P-g | a value holding a newline shaped like another listing line (`core.bare` unset, `core.logallrefupdates` forged) | **fixtures** |
| P-g | `.git/config` replaced by a symlink to a copy outside the fixture root | **fixtures** |
| P-g | a fixture repo whose `.git/HEAD` is a FIFO (git is not run on it; it would block) | **fixtures** |
| W3 | stderr without a newline, then a non-fatal diagnostic appended to that record | **fixtures** |
| W | `set +e` before the fixtures file's `built` line | **fixtures** |
| W3 | `_ar=$(( 1/0 ))` after the fixtures file's first line | **fixtures** |
| P-g | `printf '[include]…' >> .git/config` in a fixture | **fixtures** |
| P-g | one fixture's `git init` gains `--separate-git-dir` (a gitfile) | **fixtures** |
| P-g | a nested repo with a persisted `core.excludesFile` | **fixtures** |
| P-g | a fixture unsets a key a plain init writes (`git config --unset core.filemode`) | **fixtures** |

- **Totals at `8413a4db`:** **16 labels and 50 records** (how each came to be: `…-pr527.md`). `_MUT_TARGETS` was `"harness fixtures"` there (the prefix parts; now §6.1's rows); a record
  with no prefix edits the wire.
- **The `fixtures:` prefix:** it is a BSD `sed` error ("invalid command code f"); GNU is unmeasured.
- **The ratchet at `8413a4db`:** `_MUT_UNRECORDED_MAX` stayed at 21, and `_MUT_RECORDS_MIN` rose by exactly 50 (95 → 145).
- **What is not a record:**
  - RES cells, because the runner requires the `!survive` needle exactly once (`mutations.sh:658–662` at `e8f78896`);
  - the exit number, which is unpinnable (design memo §3);
  - the two INFO cells, which are informative;
  - resolving relative `PATH` entries: it changes something only for a caller whose `PATH` has one, and a
    record's run uses the caller's `PATH`. Measured by hand (Codex R23's shape, both shells): a `tools/bin`
    `git` wrapper with a `#!/usr/bin/env` interpreter beside it — red on `cb8b0e09`, PASSED after.
- **Records on a machine that cannot evaluate their postcondition** (a `machine_limits` line, a fact or a declared
  limit: P-b on git before 2.42, a fact row of the capability table, needing no declaration; P-f on an `env` without `-0`, a limit
  row, reached only where the caller declared it, `WEBREF_WIRE_LACKS=P-f`: an undeclared limit is red under `$_pfl_lbl`
  before any record runs, and so is an undeclared absent group, under the group's label): the mutation run still
  runs them — a record pinning the limitation test's own narrowness dies there — and the outcome an excuse accepts is
  reported as not exercisable on that machine instead of counted (the summary says how many). Two sources, both
  read by `_mut_run`:
  - **P-x** (any `P-x — …` line of `$_fw_limits`, a `limit` or a `fact` row's ID): the labels are derived from the ID,
    exactly `_p<x>_lbl` (its postcondition's label) and `_p<x>l_lbl` (the ID's limitation-test label; for P-f,
    `_pfl_lbl`, also its limit row's label), in `$_lbd_names`; **a survival (1) only** is excused. A renamed label falls out
    of the excuse and the opt-in run reds on a limited machine (the fail-safe side).
  - **The capability table** (CR-B2c): a record whose needle is a member of a group whose flag is 0, or that
    group's own label, on a machine that declared the absence (a flag-0 group the caller did not declare in
    `WEBREF_WIRE_LACKS` is red first, by the table check `_cap_check`, and never reaches `_mut_run`; a needle whose
    group no `group` row declares, or whose flag is unset, is no 0, so nothing is excused). **A survival (1) or a wrong-reason kill whose mutant exited 1
    (a 3) is accepted**: a declined member's label never stands in a label slot on that machine. The excuse reads the
    member's own group flag, so `WEBREF_WIRE_LACKS=fifo` excuses nothing in `perm`.
  - **A 2 is never excused** (a mutant that exited 2 ended at "decided nothing", which a harness break also reports:
    the four records whose needle is `$_pg_lbl` plant a FIFO and are red on a no-FIFO declaring machine, BS-7), and **a 3
    that nothing excuses is red** (a record whose needle is no member of an absent group, killed for the wrong reason
    by a mutant that exited 1). The excuse is per label, so on such a machine a regression in a record that would die
    there is excused too — a machine with the capability (CI's GNU `env`) runs them all; a leaking caller can still
    make a false red (the fail-safe side). A record whose anchor no longer matches is red before `_mut_run` is
    reached, by the always-on anchor check (measured, planted on an excused needle); the apply-time `MATCHED NOTHING`
    path under an excused needle is not separately measured.
  - Measured at the CR-B2c code commit `4df27614`, both shells, one-record trials on a root-simulating `PATH` shim with
    `WEBREF_WIRE_LACKS=perm` (the `err`-fixture seal records, the self-test-root, `$SCRATCH` and
    ctlrun umask records, the `-x` record and the permission probe's flip: each survived, excused, `1 not exercisable`; the
    `_fixture_failed clean` umask record: "was killed for another reason", excused) and on a no-FIFO `PATH` shim with `WEBREF_WIRE_LACKS=fifo` (the FIFO probe's
    flip: excused). Where the capability is present (no shim, no declaration) the `err` seal record and the
    `_fixture_failed clean` umask record are killed as named, silent; the record whose needle is the table check's own
    label (`$_capt_lbl`) is killed as named on the declaring root shim too, never excused. Not measured: a real root user, a real no-FIFO filesystem (simulations by behaviour only).
- **What counts as a kill.** `_mut_trial` counts a record killed only where its needle stands in a diagnostic's label
  slot: `CONTROL FAILED (<needle>)` or `CONTROL NOT EXERCISED (<needle>)` (the `!kill` arm's `CONTROL FAILED` form, plus `CONTROL NOT EXERCISED`). A label that is
  printed on a green summary line therefore cannot let an unrelated red kill a record (K15, R7.13).

**Mutation-mode cost (X3).** Each run of X3 is one record trial per record (the count is §6.1's Current totals) plus the generated
population, one control pass each. X3 prints the counts, and X8 gives the per-pass time. This is
opt-in and does not add to the always-run gate.

### §6.1 Drafts 11–21, the fix round and the review rounds after it: the record changes (`…-landing.md` §9.1; draft 21 added no record of its own -- D19-A's run-time layer is pinned by X3's floor cell, not a record)

Each change is listed with the commit that makes it. The needle of every record is its label, so
renaming a label renames the needle of each of its records in the same commit. "This PR's" counts the
records this PR adds (the table above); "all" is every entry the runner counts, `!survive` included
(`awk -F'\t' '!/^#/ && NF>1'` over the `MUTANTS` here-document: 145 at `8413a4db`). Draft 12's C7 and
its two records were withdrawn with the time bound.

Each record is planned to be **killed** when X3 runs at T (`…-landing.md` §9.1), and to **survive**
with the clause it names removed; that **survive check** is run by hand as part of X3 (`…-landing.md`
§11). Per commit, the ordinary run reads the records in three places, and none of them runs a record
(X1): the always-on anchor check in `_mut_correspondence` (every record's expression must change its
target; `mutations.sh:537–560` at `8413a4db`), the records floor (`_MUT_RECORDS_MIN`, line 574) and the
label ratchet (`_MUT_UNRECORDED_MAX`, line 595). A record that plants a FIFO is the exception to the
survive check, because without its clause git blocks on the FIFO and the run waits (no time bound,
design memo §5.2 R9). For those, the removed-clause run is a **necessary-condition check** only. It
runs by `cell15.sh`'s method (`…-pr527.md`, round 13): its own process group under
`perl -e 'setpgrp; alarm 120; exec @ARGV'`, with a `kill -9 -<pgid>` watchdog 3 s after it, because
bash 3.2 does not end a hung wire at the alarm (`…-landing.md` §11, "Cell runners at T"). It is
expected to reach that bound with
`ps -ax -o pid=,pgid=,command=` showing the P-g `git … config --list` of that group blocked; then that
group alone is killed with `kill -9 -<pgid>`, so nothing is left behind.

| commit | change | label | target | this PR's / all |
|---|---|---|---|---|
| — | at `8413a4db` | | | 50 / 145 |
| C6 | **delete**: "a relative entry after `$_FGIT_BIN` in the window's `PATH`". There is no per-entry clause left to pin | P-j | harness | 49 / 144 |
| C6 | **re-anchor**: "the window's `PATH` starts at `/usr/bin`" becomes "a directory with no `git` placed before `$_FGIT_BIN`" (`/nonexistent-k2:` in front), which pins the first-entry clause alone. The `gitx` record still pins the `git` clause. Measured (`…-pr527-r15.md` §Q, the `Jm` cells, bash 5.3): the expression applied to the draft-19 prototype, which has the clause, gives rc 1, P-j `first-entry`; applied to the draft-18 prototype, which lacked it, rc 0, PASSED, the survive check | P-j | harness | — |
| C6 | **relabel**: P-j's two records take the label `the fixture build window runs the pinned git first on its PATH` | P-j | — | — |
| C8 | **add**: one harness expression inside `_fgit_window_verdict_exit` that forces a postcondition red (`post_bad` written unconditionally) and removes the untrusted-build exit. W2 is reported by `_control`'s gate, `_fw_built_or_w2`, which decides by `_fw_trusted` alone. It survives with the `post_bad` clause removed from `_fgit_window_verdict_exit`'s computation of `_fw_trusted` (the writer), which then writes 1. The fix round moved that computation into `_fgit_window` (below) | W2 | harness | 50 / 145 |
| C8 | **add**: the same exit removed, with W3 forced (`_fw_diag` set) instead. It survives with the W3 clause removed from the same computation of `_fw_trusted` | W2 | harness | 51 / 146 |
| C8 | **re-anchor**: W2's existing record (`harness:/^_fgit_window_incomplete_exit()/,/^}/…`, `mutations.sh:343` at `8413a4db`) addresses the function C8 renames, so its range becomes `/^_fgit_window_verdict_exit()/,/^}/` | W2 | harness | — |
| C8 | **relabel** (D18-B; the draft-19 open item, closed): the record `s/^export LC_ALL=C$/export LC_ALL=C.UTF-8/` (`mutations.sh` line 244 on the prototype) takes P-h's label, `the fixture build window reads in the wire's locale`. After C8 it cannot be killed as `a byte no UTF-8 locale can bracket`: the wire's locale becomes `C.UTF-8`, the window's stays `C` (its allowlist), P-h reds, and the untrusted-build stop runs no control, so that needle never prints (`MUTANT 14 … killed for the WRONG REASON`, both draft-19 prototype mutation runs). Under P-h's label it is killed as named (`m14lc`, `…-pr527-r15.md` §Q), and it pins the wire side of P-h, which no record did: the existing P-h record changes the window's locale. As relayed from round 18 (the reviewer's directory `/tmp/elidex-plan-review.7e5b6f5f-cec2-4f20-9e7a-b25f0c4752cf/r18/ax23/sweep/`, not re-measured here), the reviewer probed every record with the controls cut after the prototype's C8 stand-in, and this was the only one stopped before its needle; the prototype's C8 is a one-line stand-in (the `K2PROTO` stop in the controls file), not `_fgit_window_verdict_exit`, so X3 re-measures that at T | P-h | — | — |
| C8 | **add**: the scan's locale alone, `s/_co="$(grep -aEn -- "$K2RE"/_co="$(LC_ALL=C.UTF-8 grep -aEn -- "$K2RE"/` (a wire target), which keeps the `rawbyte` control's question after the relabel. Killed as named, rc 1 on both shells, the one CONTROL FAILED `a byte no UTF-8 locale can bracket`, no P-h, no stop (`m14grep`); it survives with the `rawbyte` control removed (`m14surv`, bash 5.3; `…-pr527-r15.md` §Q). It needs a UTF-8 `C.UTF-8` locale, as the relabelled record did | rawbyte | wire | 52 / 147 |
| C8 | **relabel**: W2's existing record takes the label `no control runs over an incomplete or untrusted fixture build window` | W2 | — | — |
| C9 | **add**: `clean`'s `.git/objects` replaced by a symlink to a copy outside the fixture root (PR #527 Codex R26②: PASSED at `8413a4db`). It survives without the shape rule | P-g | **fixtures** | 53 / 148 |
| C9 | **add**: a FIFO `.git/commondir` in `clean` (Codex R26③). Without the shape rule the run waits (the FIFO procedure above) | P-g | **fixtures** | 54 / 149 |
| C9 | **add**: the shape scan replaced by a command that fails with no output. It pins that a failed scan is red, not an empty result, and survives with the status check removed | P-g | harness | 55 / 150 |
| C9 | **add**: `xcommon`, a FIFO `zzr/.git/config` with `clean/.git/commondir` naming `../../zzr/.git` (round 12). It pins the census-before-any-git order: with the shape verdict taken per repo, git runs on `clean` and waits (the FIFO procedure above) | P-g | **fixtures** | 56 / 151 |
| C9 | **add**: `clean/.git/info/exclude` replaced by a hard link to a file outside the fixture root (round 12). It survives with the one-link clause removed | P-g | **fixtures** | 57 / 152 |
| C9 | **add**: `xlink` (round 13), a link `$CTL/zzl` to a repo outside the fixture root whose `.git/config` is a FIFO, with `clean/.git/commondir` naming `../../zzl/.git`. Only pass 2 reds it. It pins "one census verdict after both passes": with pass 2 run inside the per-repo loop, git runs on `clean` first and waits (the FIFO procedure above) | P-g | **fixtures** | 58 / 153 |
| C9 | **add**: `xrefpoison` (round 14): the fixtures make `$_FW_DIR/pq/a` and `$_FW_DIR/pq/b` in advance, the old reference path, as repos with `core.excludesFile /nonexistent-k2`, and write the same key into every fixture repo's `.git/config` (as shipped, the record wrote `clean`'s alone, which kills it whatever the reference path; review R2 restored every repo, below). Killed by P-g against the references made after the build (`…-pr527.md` round 14: rc 1 in 5–7 s on both shells). It survives with the references made at the old fixed path (`_pq="$_FW_DIR/pq"; mkdir -p "$_pq/a" "$_pq/b"`): measured on the draft-15 prototype, rc 0, PASSED on both shells. No record removes the `mkdir`-without-`-p` freshness assertion: the directory `mktemp -d` has just made holds nothing, so no run can present a stale repo there, and that mutant is equivalent **within class (b)**: a fixture command still running after the build can place one there (`xrace`, round 15), which is class (c) and not owed (design memo §0.3) | P-g | **fixtures** | 59 / 154 |
| C9 | **add**: `lnblind2` (round 15): a link `$CTL/zzy` to a tree in `$_FW_DIR` holding a git dir (`zzt/a` with `objects` and `HEAD`), and `$_FW_DIR/prel` and `$_FW_DIR/pgls`, draft 16's pass-2 link list and in-loop per-link result, made links to `/dev/null`. Killed by P-g's pass 2 once the working files are in the postconditions' directory (`K2PRE zzy:[a symlink to a tree holding a git dir]`, rc 1 in 3–5 s on both shells, `…-pr527-r15.md` §Q). It survives with the working files back in `$_FW_DIR`: the draft-16 prototype gives rc 0, PASSED on both shells. C9 deletes the in-loop search; the record still planted `pgls` too, which nothing read, until the fix round dropped that half (below), so it now plants the pass-2 list alone (round 15's `lnblind` insert, verbatim with its source in `…-pr527-r15.md` §Q, which the draft-16 prototype reds only through that search: `zzy:[a symlink to a tree holding a git dir: a/HEAD]`, bash 5.3). That it survives on the implementation with the working files back in `$_FW_DIR` is expected, not measured; X3's survive check measures it | P-g | **fixtures** | 60 / 155 |
| C9 | **re-anchor**: the P-f record "`env -0`'s output gains a last record with no NUL after it" (`mutations.sh:360` at `8413a4db`) and the P-g record "the missing-lines `grep` reads a missing pattern file" (line 364) name `$_FW_DIR/env0` and `$_FW_DIR/pgcur`, which C9 moves into the postconditions' directory (`$_pq`); both expressions name the new path | P-f, P-g | harness | — |
| C10 | **add**: `clean` gets an `objects/info/alternates` naming an object store outside the fixture root. It survives with the `alternate:` clause removed | P-k | **fixtures** | 61 / 156 |
| C10 | **add**: the liveness probe loses its `alternates` file, so it prints no `alternate:` line: NOT EXERCISED, red. It survives with the liveness check removed. With no machine-limitation arm, no machine excuses it. (As shipped, the record made the probe's `git init` fail, the `else` arm, and is kept for it; this row's form is review R2's record, below) | P-k liveness | harness | 62 / 157 |
| C10 | **add**: `count-objects -v` replaced by a command that fails with no output. A failed count is red, and the record survives with the status check removed (the empty output then has no `alternate:` line, so it reads green) | P-k | harness | 63 / 158 |
| C10 | **add**: `clean` gets an `alternates` naming a store that does not exist: rc 0, no `alternate:` line, an error on stderr. It survives with the stderr clause removed | P-k | **fixtures** | 64 / 159 |
| C11 | **no change** for the move: the parent's working files move out of `$CTL` (`…-landing.md` §9.1), and no record anchors their paths: over the `MUTANTS` here-document (`mutations.sh` lines 230–387 on the draft-18 prototype), `/usr/bin/grep -n` with the alternatives `-e 'CTL/\.' -e 'CTL\\/\.' -e fsmhook -e fsmonitor_ran -e control_out -e fifoprobe` finds one line, the `.gd-cachedir` record, a fixture's path. `xbare` and `xanchor` are X16 cells, not records: each needs a tree edit (the ratchet at 0, a stale record) besides its insert, and a record is one expression. Draft 19 put the empty-generated-set check (D17-D) in the opt-in run, where no record could reach it (a trial runs the wire with `env -u WEBREF_WIRE_MUTANTS`); draft 20 makes it always on, and the next row pins it | — | — | 64 / 159 |
| C11 | **add** (D18-B): `mutgen:s/^_mut_regex_mutants() {$/_mut_regex_mutants() { return 0/`, a `mutgen` target (`_MUT_TARGETS` gains it), labelled `the boundary-mutant generator derives a non-empty set from the wire's regexes`. The generator then emits nothing, and the always-on `_mut_gen_floor` reds under that label, rc 1 on both shells (`floorkill`); it survives with the `_mut_gen_floor` call removed from `_mut_correspondence`, rc 0, PASSED (`floorsurv`, bash 5.3; `…-pr527-r15.md` §Q). Draft 21's run-time layer (`_mut_gen_n` against `_mut_gen_floor_n` in `_mut_run`, D19-A) adds **no record**: it runs only in the opt-in run, which no trial reaches; X3's floor cell pins it | generator floor | mutgen | 65 / 160 |
| C12 | **add**: `exec 2>/dev/null` just before the fixtures' `built` line. Killed by W ("the fixtures file left the window's stderr redirected"): on the draft-18 prototype this insert alone gave rc 2 (bash 5.3), and with `: $((1/0))` after it (`xexecarith`) rc 2 on both shells (`…-pr527-r15.md` §Q). It survives with the canary check removed: without the check (the draft-16 prototype) the insert alone gave rc 0, PASSED (bash 3.2), and `xexecarith` rc 0, PASSED on both shells on the draft-17 prototype (round 16) | W | **fixtures** | 66 / 161 |
| fix `976e1430` | **re-shape**, all records: a record is `expr1 [TAB expr2 …] TAB needle`, and the always-on anchor check applies each expression alone and requires it to change exactly one line. The three W2 records' compound `range s;s` expressions become one expression per line; the wire record on `--exclude-per-directory=.gitignore` is anchored on the code line, not on a comment as well; the P-f `-i` record on the window's `env` call alone | W2, P-f, wire | — | — |
| fix `15d3e704` | **remove**: the two P-g line-`grep` failure records (extra-lines, missing-lines). The record comparison runs first and decides; the line greps only name the entries on a red path, so no failure of theirs can turn a red green, and the arm they pinned is gone | P-g | harness | 64 / 159 |
| fix `15d3e704` | **retarget**: C9's "shape scan replaced by a failing command" addressed the P-g loop's duplicate scan, which this commit deletes (one census); it now replaces pass 1's shape scan. **Re-anchor** onto `_fgit_in` (git run in the repo with `--git-dir=.git`, no discovery): P-a liveness, P-b liveness, P-k status. P-b's `GIT_CONFIG_NOSYSTEM` record now drops that name alone. `lnblind2` drops its `pgls` half | P-g, P-a, P-b, P-k | harness | — |
| fix `15d3e704` | **add**: pass 1's `find` fails after writing its list (`&& false`), and pass 2's `find` fails. Killed by P-g. **Each record pins its arm** (corrected in review R2; this row first said errexit backs the arms): `_fgit_postconditions` runs as the left operand of an and-or list, so errexit is off inside it, and with the arm removed the failed `find` is ignored and the record survives (measured: no CONTROL FAILED, no W, `r422noarm`/`r423noarm`, both shells); pass 1's `&& false` form keeps the list complete, so `(no fixture git dir was found)` does not fire either | P-g | harness | 66 / 161 |
| fix `15d3e704` | **add**: `-print0` → `-print` in pass 1, and in pass 2 (`… does not end in a NUL byte`). Pass 2's survives with the NUL check (since the /simplify round, the loop-remainder test after the reader) removed; pass 1's is still killed then, by the zero-population check | P-g | harness | 68 / 163 |
| fix `15d3e704` | **add**: a line on the census's stderr written by pass 2 (`(census stderr)`): one check after both passes. It survives with the check back before pass 2 | P-g | harness | 69 / 164 |
| fix `15d3e704` | **add**: `env` without `-0` (one record): `missing PATH HOME …`. It survived without the presence check until the /simplify round, whose NUL-list policy reads that output as no record, so it is now killed by the zero-population arm (`NOT EXERCISED … 0 record(s)`) and pins `-0` alone; the presence check is review R2's `export -n LC_ALL` record's (below) | P-f | harness | 70 / 165 |
| fix `15d3e704` | **add**: `GIT_ATTR_NOSYSTEM` dropped alone (`GIT_ATTR_SYSTEM=[…]`); the window's `HOME` set to `$SCRATCH` (`GIT_CONFIG_GLOBAL=[…]`; anchored on the `_FGIT_ENV` line since `6ed414b2`, when P-e's reference gained `HOME` too) | P-b | harness | 72 / 167 |
| fix `15d3e704` | **add**: the fixtures write `$HOME/.config/git/ignore` (`[.config]`, a dot name); the fixtures plant a void entry and set `GLOBIGNORE='*'` (`[k2plant]`) | P-c | **fixtures** | 74 / 169 |
| fix `15d3e704` | **add**: P-k's `grep` for `alternate:` exits 2 (`[the comparison failed (grep exit 2)]`). It survives with the exit test removed. **Removed** in `2b56c33c`, with the arm (`_fgit_alternates` is a builtin loop) | P-k | harness | 75 / 170 |
| fix `6ed414b2` | **re-shape**: the incomplete-window W2 record removes both of `_fgit_window_verdict_exit`'s exits, `done` renamed. All three W2 records are killed with W2's label and each survives with its clause (`done`, `post_bad`, W3) removed from `_fgit_window`'s trust computation. **Relabel**: W3 becomes `the fixture build window ran without a shell diagnostic` (both W3 records' needles) | W2, W3 | — | — |
| fix `6ed414b2` | **add**: `: $(( 1/0 ))` at the start of `_fgit_postconditions` (`./prelude.sh: line …: division by 0`). It survives with W3's `./prelude.sh:` clause removed | W3 | harness | 76 / 171 |
| fix `6ed414b2` | **add**: the pin's re-entry limit set to 0. It survives with the depth check neutralised (a removed guard would wait, the liveness slot's class). **Removed in review R2** with the counter (below) | pin | harness | 77 / 172 |
| fix `6ed414b2` | **add**: `_seal_apply`'s symlink walk, and its refusal, each replaced by `:`. Killed at load by the standing self-check `_seal_apply_probe` (exit 2); each survives with the probe call disabled | W4 | harness | 79 / 174 |
| fix `6ed414b2` | **add**: a `_seal` with the mode `a-rwx`, and one with the fixture name `wa.lk` (values `chmod` would apply), refused by the validation alone. Each survives with its validation removed | W4 | **fixtures** | 81 / 176 |
| fix `6ed414b2` | **add**: the `err` fixture marked failed before its seal (`CONTROL NOT EXERCISED (an unreadable file fails closed)`), and the `err` `_seal` line deleted (the err control's wrong-reason report). Both survive with the permission block gated on `[ -r err/control.py ]` again instead of on the `_perm_ok` probe | `an unreadable file fails closed` | **fixtures** | 83 / 178 |
| /simplify round | **add**: `_pg_z` given an unterminated remainder (`printf x >> "$2.raw"`): killed by the loop-remainder tail check, survives without it. Pass 2's and P-f's tails are pinned by pass 2's `-print0` record and P-f's `K2X=1` record (each survives with its own tail check removed); pass 1's `-print0` record is killed by the zero-population check even then, so pass 1's tail is review R2's record (below) | P-g | harness | 84 / 179 |
| review R2 `96eea58c` | **replace**: the pin's limit-0 record goes with the counter; the shim builder `_fgit_shim` without its `PATH=${PATH#…:}` line takes its place: P-j's alias probe sees `$_FGIT_BIN` on the `PATH` the pin hands on (`pin-on-the-PATH-it-passes-on`). It survives with the alias check removed. **Re-anchor**: the `gitx` record onto the `_fgit_shim "$_FGIT_BIN/git"` call | P-j | harness | 84 / 179 |
| review R2 `96eea58c` | **re-shape**: `xrefpoison` writes the key into every `$CTL` `.git` again (a `find` loop), as the C9 row says; it survives against the old fixed-path reference form | P-g | **fixtures** | — |
| review R2 `96eea58c` | **add**: a repo with a persisted key under a bare `chmod 0444` parent (`zzm`): `[a directory the census cannot search]`. It survives with the census's `! -perm -u=rx` clause removed (BSD `find`, this macOS) | P-g | **fixtures** | 85 / 180 |
| review R2 `96eea58c` | **add**: `_fws_lbl=` renamed in the controls file (a `controls` target, new in `_MUT_TARGETS`): `used but not defined, or empty: _fws_lbl`, exit 2, before the harness loads. Before it, 3.2 gave rc 0 (a false green) when a seal also failed | `every label the parts use is defined` | **controls** | 86 / 181 |
| review R2 `96eea58c` | **add**: the `post_ok` writer deleted, and its file misspelled (two records): `the postconditions never reached a verdict`. Each survives with `post_ok` removed from the trust computation | W | harness | 88 / 183 |
| review R2 `96eea58c` | **add**: `export -n LC_ALL` before `built`: `missing LC_ALL`. It survives with the presence loop removed | P-f | **fixtures** | 89 / 184 |
| review R2 `96eea58c` | **add**: `_seal "$CTL/walk/../walk/sub" 000 walk` (a path `chmod` would apply): refused, `..` component. It survives with `_seal`'s component check removed | W4 | **fixtures** | 90 / 185 |
| review R2 `96eea58c` | **add**: `GIT_EXEC_PATH=$_FGIT_VOID`, a real directory, in the window's allowlist: P-e's `!=` comparison (`[void] vs [git-core]`). It survives with the `!=` clause removed (the old `/nonexistent-k2` record dies by the empty-answer clause) | P-e | harness | 91 / 186 |
| review R2 `96eea58c` | **add**: the liveness probe's `alternates` written to `/dev/null` (rc 0, no `alternate:` line, no stderr). It survives with the `-z` clause removed (the more-than-one-line clause has its own record, review of `fd85e29f`) | P-k liveness | harness | 92 / 187 |
| review R2 `96eea58c` | **add**: two expressions, `nounset` dropped from the prelude and the options re-check after the fixtures file deleted: W's "refused to start". It survives with the check before the fixtures file removed | W | harness | 93 / 188 |
| review R2 `96eea58c` | **add**: pass 1's list given an unterminated duplicate record after the `find`: `(the census list does not end in a NUL byte)`. It survives with pass 1's tail check removed (rc 0, PASSED) | P-g | harness | 94 / 189 |
| Stage 5 `8f4ad3c8` | **add**: the orderly-end flag's assignment before the PASSED line replaced by `:` (`s/^_K2_ORDERLY=1$/:/`, no prefix: the wire itself): every run that would end green ends `the run ended before reaching its verdict`, rc 1. Killed with its needle on both shells. Removing the check itself has no record: bash 5 exits 1 on a `set -u` abort by itself, so such a record could die only on 3.2 (`…-landing.md` §9.2, the exit-trap closure). E-3's unreadable-marker check has none either: its trigger is a fixtures file that chmods the window's own marker, class (c); measured by hand | `the run ends only through a verdict` | **wire** | 95 / 190 |
| `2b56c33c` | **remove**: P-k's `grep '^alternate:'` exit-above-1 record (`[the comparison failed (grep exit 2)]`; the `15d3e704` row above notes it). The extraction is now `_fgit_alternates`, a builtin loop, so no external `grep` can exit above 1: the arm goes with its record, and no clause is left for the survive check | P-k | harness | — |
| `2b56c33c` | **add**: `_ctl_env=()` deleted from `_control` (D-1; from `fd85e29f` it is `_ctl_run`'s). Killed by the controls file's standing check `$_ce_lbl` (a refused control in a subshell leaves `_ctl_env` empty), with its label's needle. The record's comment names no survive clause, as for the `_fws_lbl` record (review R2 row above) | `a refused control hands its environment to no other control` | harness | — |
| `2b56c33c` | **add**: `_fixture_failed relcwd`, `_fixture_failed fsmon` and `_fixture_failed clean` before the `built` line (D-8: the three blocks that are not a `_control` read the fixture-failure list through `_ctl_gate`). Each is killed with its block's label. At `2b56c33c` each survived with that block's `_ctl_gate` call removed (history: no block calls `_ctl_gate` now; `fd85e29f` moves the gate into `_ctl_run`). At `20615255` each survives with `_ctl_built`'s fixture-failure test disabled (`if _fixture_did_fail "$_b_n"; then` becomes `if false; then`): rc 0, PASSED on both shells (cases `hFFrelcwd`, `hFFfsmon`, `hFFclean`); each record alone KILLED | `a relative scratch dir is removed on exit`, `a caller's fsmonitor hook does not run`, `a restrictive umask decides nothing` | **fixtures** | — |
| `2b56c33c` | **add**: P-b's `git var` status stored as 1 (`printf -v "_pbr_$_v" '%s' 1`; from `b164f3ae` the loop's `_prc=1`, below): a status of 1 on a call that succeeded reads as a failed GLOBAL call (`GIT_CONFIG_GLOBAL:rc!=0 GIT_ATTR_GLOBAL:rc!=0`). It survives with the GLOBAL arm's red for a non-zero status removed (the `… _pbd="$_pbd $_v:rc!=0"; fi` line, its `continue` kept): measured at `20615255` on both shells as a `.mutant.99` copy (`S4bPbsurv`), the record alone KILLED with its needle, rc 1; with that line removed too, rc 0, PASSED | P-b | harness | — |
| `2b56c33c` | **add**: the `alternate:` extraction broken (`alternate:*)` becomes `NOALT:*)`; re-anchored in `fd85e29f`, below): the liveness probe sees no `alternate:` line, NOT EXERCISED. It survives with the liveness check's no-line clause (`[ -z "$_fgit_alt" ]`) removed: measured at `20615255` on both shells as a `.mutant.99` copy (`S4bNoaltsurv`), the record alone KILLED with its needle, rc 1; with that clause removed too, rc 0, PASSED, so P-k, which reads the same extraction, does not red either | P-k liveness | harness | — |
| `2b56c33c` | **re-shape**, 34 `built`-anchored fixtures records (D-7): the expression spelled `: > "$_FW_DIR\/built"` twice (the anchor and the replacement) now writes it once, as `…; &`. Each yields the same fixtures file as before (the old and the new expressions applied to `fixtures.sh` at `2b56c33c`: the 34 outputs are identical). 22 are P-g's, 7 P-c's, 2 P-k's, 2 W's and 1 P-f's; the three records added above are written in the new form | P-g, P-c, P-k, W, P-f | **fixtures** | 100 / 195 |
| `fd85e29f` | **add**: the canary scan fails (`END { exit !f }` becomes `END { exit 3 }`): a scan that cannot run is red, not "not found". Killed with W3's label. It survives with the fold of the scan's failure into `_fw_diag` at the trust site removed (measured on `116a9b8b` and again on `aa0a501b`: a `.mutant.99` copy, PASSED, needle absent) | W3 | harness | — |
| `fd85e29f` | **re-shape**, 6 records (S-5): the anchor is written once, as `&`. P-h's `s/^export LC_ALL=C$/&.UTF-8/`; W's `&; exit 0` after the `prelude.sh` source line; W3's `_fpv=0` record and its two `mkdir -p "$CTL\/walk\/sub"` records; the generator floor's `_mut_regex_mutants() {`. Each yields the same file as before (6 identical outputs at `fd85e29f`) | P-h, W, W3, `the boundary-mutant generator derives a non-empty set from the wire's regexes` | wire, harness, fixtures, mutgen | — |
| `fd85e29f` | **add**: `_fixture_failed fakerelmktemp` before the `built` line (its comment: the PATH-prefix dir of a run is a tree it reads, which the gate then derived). Its survive clause was never stated, and it was never measured, while it lived; `116a9b8b` removed it (below) | `a relative scratch dir is removed on exit` | **fixtures** | — |
| `fd85e29f` | **re-anchor**: the record that breaks the `alternate:` extraction (`s/alternate:[*]) printf/NOALT:*) printf/`) addresses `_fgit_alternates`' arm, which now assigns `_fgit_alt`: `s/alternate:[*]) _fgit_alt=/NOALT:*) _fgit_alt=/` | P-k liveness | harness | 102 / 197 |
| `116a9b8b` | **remove**: `_fixture_failed fakerelmktemp` (added in `fd85e29f`). The gate no longer derives names from the PATH-prefix shim dirs (they are built bare, and errexit covers a failed build), so a failed `fakerelmktemp` fixture is an impossible state and the record has no clause to pin | `a relative scratch dir is removed on exit` | **fixtures** | — |
| `116a9b8b` | **re-shape**, three records from a `\(…\)` … `\1` group to `&`: P-f's `K2X=1` record (the `env -0` output gains a last record), P-g's `printf x` record (the `config --list` raw tail) and P-g's pass-1 tail record (`printf %s "$CTL\/clean\/.git" >> "$_pq\/pre"`). Each yields the same file as before (3 identical outputs at `116a9b8b`). With the record below, this commit has three records re-shaped to `&`, one new record that uses `&`, and one removed | P-f, P-g | harness | — |
| `116a9b8b` | **add**: P-k liveness's second store: a second quoted path appended to the probe repo's `alternates` (written with `&`), so git prints two `alternate:` lines with no stderr. Killed with P-k liveness's label by the more-than-one-line clause; it survives with that clause removed (rc 0, needle absent; the shipped tree is killed on both shells) | P-k liveness | harness | 102 / 197 |
| `aa0a501b` | **add**: `_ctl_run`'s timeout clause with the watchdog-marker test deleted (a status of 128 or more alone makes a timeout), so a script that signals itself at once is reported "killed after 30s". A `harness` record then (`ctlrun` after `16efb423`). Killed on both shells by the controls file's standing check `$_wm_lbl`. The record's comment names no survive clause, as for the `_fws_lbl` record | `only the watchdog's marker makes a signal status a timeout` | harness | 103 / 198 |
| `2d5c393f` | **re-shape**: the `err` seal record writes `&` instead of `\(…\)$` … `\1` (`s/^_seal "$CTL\/err\/control.py" 000 err$/_fixture_failed err; &/`). The sed output on the shipped fixtures file is byte-identical (the old and the new expressions applied at `2d5c393f`) | `an unreadable file fails closed` | **fixtures** | — |
| `2d5c393f` | **re-shape**: P-k's second-store record C-quotes the appended path in the form of the line it extends (`\` and `"` escaped by a `sed` the `printf` is piped to). Under a backslash `TMPDIR` the old record was killed by the stderr clause (git could not normalise the raw path); the new one is killed by the more-than-one-line clause it is about (two `alternate:` lines, empty stderr), on bash 5.3 and 3.2 (the commit message). Its survive clause is the one in the `116a9b8b` row; measured at `20615255` on both shells (case `hPk2`): that clause (` \|\| [ "$_fgit_alt" != "${_fgit_alt%%$'\n'*}" ]`) removed plus this record, rc 0, PASSED; the record alone KILLED | P-k liveness | harness | 103 / 198 |
| `e271cb02` | **add**: two `controls` records, one per control, each replacing the shim's PATH-prefix argument by the empty string (the relcwd control's `fakerelmktemp` shim, cfgkept's `fakegitcfg` shim; tail-anchored, so the expression does not spell the needle). Each shim is built under `$_VFY` and touches a mark whenever it runs; an absent mark is `CONTROL NOT EXERCISED` under the control's label. Killed with the label on both shells; each survives with the mark check removed (measured, both shells) | `a relative scratch dir is removed on exit`, `the caller's git CONFIGURATION survives the routing purge` | controls | 105 / 200 |
| `16efb423` | **re-aim**: the two `harness` records anchored in the text the prereq split moves to `ctlrun.sh` (`_ctl_env=()` deleted; the watchdog-marker test deleted) become `ctlrun` records; `ctlrun` joins `_MUT_PARTS` and `_MUT_TARGETS`. The expressions and needles are unchanged; both are killed with their needles on both shells (the commit message) | `a refused control hands its environment to no other control`, `only the watchdog's marker makes a signal status a timeout` | ctlrun | 105 / 200 |
| `d976f239` | **add**, the census (`$_cen_lbl`, a new label: the labels a run used — the run registry and the values of the `_*_lbl` names the parts spell, `$_lbd_names` — against the records; each record is killed by the census's own message), its two populations: the registry's writer in `_ctl_gate` replaced by `:` (`ctlrun:`, range-scoped to the function), and the `$_lbd_names` loop emptied (`mutations:`, which joins `_MUT_TARGETS` here): every registered, and every `_*_lbl`, needle is an orphan. Neither names a survive clause, and none was measured: each record removes one population itself (the slice's record plan: "is the removal") | `every label the run used is named by a mutation record` | ctlrun, mutations | — |
| `d976f239` | **add**, the census's "more" and "fewer" readings and its orphan arm: a literal label at the window verdict, and at the fsmonitor `_ctl_run` (a label no record names, "more"); the recorded `forge` control deleted (its needle an orphan); the bare `ignored` control deleted ("fewer"). They survive, in order, with the window verdict's registration removed, with the "more" arm's red removed, with the orphan arm's `bad = 1` removed, and with the "fewer" arm's red removed (measured on the slice's first draft, both shells, each a `.mutant.99` copy: rc 0, PASSED; the fsmonitor record was then anchored on a `_ctl_gate` call, and re-anchored to the `_ctl_run` one in its first review round, killed there on both shells) | `every label the run used is named by a mutation record` | controls | — |
| `d976f239` | **add**, the census's later readings: a `_control` in a `( … )` after `_mut_run` (the registry grew after the census, `_mut_census_end`, the controls file's last statement). It survives with `_mut_census_end` made to return 0, and with that last statement removed (`S12noend`, `S15noendline`, both shells: rc 0, PASSED) | `every label the run used is named by a mutation record` | controls | — |
| `d976f239` | **add**, the site rule (one label, one site; the site is the outermost controls-file frame): the bare `ignored` label on a second `_control`; a second window verdict call on another controls line; a controls-file helper that makes the recorded `clean` control, called from two lines. Each is killed as "registered from more than one call site". The first survives with the site rule's `bad = 1` removed; the second under a walk that skips only ctlrun's frames (`while [ "${BASH_SOURCE[$_rg_i]}" = "${BASH_SOURCE[0]}" ]`), so both verdict calls fold to the harness frame (case `hsurvA`, re-aimed at `20615255`; it was `survA_skipctl`), and again with the window verdict's registry writer removed (case `hR3nvr`, re-aimed at `20615255`; it was `R3noverdictreg`); the third under a nearest-controls-frame walk (case `hsurvB`). All four survive cases (`hsvR8` for the first, `hsurvA`, `hR3nvr`, `hsurvB`) were measured at `20615255` on both shells as `.mutant.99` copies: rc 0, PASSED; the first and the third were first measured in the slice's review rounds | `every label the run used is named by a mutation record` | controls | — |
| `d976f239` | **add**, the kind rule (a `_control` label is literal): `$_mg_lbl`'s value as a second `_control`'s label, "a `_*_lbl` value passed to `_control`". It survives with `_ctl_reg`'s kind detection removed, and with the census's kind arm removed (`S10nokind`, `S11nokindrule`, both shells: rc 0, PASSED) | `every label the run used is named by a mutation record` | controls | — |
| `d976f239` | **add**, `_ctl_need`'s two records (`$_cn_lbl`, a new label): the `fifotracked` caller's value replaced by 0 ("declined while this machine's capability flag is 1"; it dies only where `_fifo_ok` is 1), and a declined direct `_ctl_run "$_mg_lbl"` on the census's line ("declined after the last capability group's ask", on every machine). The first survives with `_cn_declined`'s growth arm, or its red arm, removed (`S13nogrowth`, `S14nored`); the second with the last ask removed (`S16nolastask`); both shells, rc 0, PASSED | `_ctl_need declines exactly what this machine cannot do` | controls | 118 / 213 |
| `288c857c` | **re-anchor**: cfgkept's prefix-drop record (`e271cb02`) onto `"$_VFY/fakegitcfg"; then ctl_ok=1`, the mark check now being the `elif` of its control's `if`. Killed alone; it survives with that `elif` made `elif false` (`S3cfgnoelif330`, both shells: rc 0, PASSED; measured at `288c857c`, its subject went in `fd90b4b1`) | `the caller's git CONFIGURATION survives the routing purge` | controls | 118 / 213 |
| `b164f3ae` | **re-anchor**: P-b's status record (`2b56c33c`) onto the one loop's status line, `_prc=0; _o="$(_fgit_in` becoming `_prc=1; …`. Killed with its needle; it survives with its GLOBAL status clause removed (`S5arecsurv`, both shells: rc 0, PASSED), and the P-b record of the `2b56c33c` row survives at `20615255` under a different clause (`S4bPbsurv` removes only the inner `_pbd=` line; `S5arecsurv` removes the whole `if [ "$_prc" -ne 0 ] … continue; fi`): both survive | P-b | harness | 118 / 213 |
| `c8eb9ab4` | **add**, four records on P-b's definition test (a name's 129 is a machine limit only if stdout is empty, stderr's first line begins `usage: git var ` and the same git answers `git var GIT_PAGER`). Each forces a 129 on every name on a capable git, so one clause turns it red: stdout non-empty; stderr a hangup line before git's usage; the control failing (`var NOSUCH_K2`); stderr git's top-level usage (`usage: git [-v \| --version] …`). Each is killed alone, and survives with its clause removed — the `-z "$_o"` test, the first-line prefix test, the control test, and the prefix narrowed to `usage: ` (`S5bsurvA`, `S5bsurvD`, `S5bsurvC`, `RVusageonly245`, both shells: rc 0, PASSED, with the machine-limit line) | P-b | harness | 122 / 217 |
| `8c952d6d` | **remove**: `_ctl_env=()` deleted (`2b56c33c`): the global went; a run's environment travels as `_ctl_run`'s trailing `NAME=VALUE` arguments, and `$_ce_lbl`'s standing check with it | `a refused control hands its environment to no other control` | ctlrun | — |
| `8c952d6d` | **re-aim**: the census's second-window-verdict record onto `# STANDING CHECK of .*timeout clause, both ways\. A status >= 128 is a$` (the comment it addressed went with `$_ce_lbl`'s check); killed with the same message on both shells, its survive clauses as in the `d976f239` row. **Re-anchor**: cfgkept's prefix-drop record onto the call's continuation line (`"$_VFY/fakegitcfg" "" \\$`), whose old line end moved; killed on both shells | `every label the run used is named by a mutation record`, `the caller's git CONFIGURATION survives the routing purge` | controls | — |
| `8c952d6d` | **add**, the channel evidence (`_ctl_run` demands every delivered channel back from the run itself): R1, the wire's routing purge disabled (`*) unset "$_v" ;; esac` becomes `*) : ;; esac`), killed by the routed control, survives with routed's channel arguments removed (`S2a2R1surv`); R2, `GIT_DIR` withheld between the delivery and the recorder (`env -u GIT_DIR`), and R5, the umask withheld at the delivery with the umask control's verdict flipped to the one it then gives (both expressions `ctlrun:`), each killed by the evidence check (NOT EXERCISED) and surviving with that check disabled (`if [ -n "$_r_miss" ]` becomes `if false`; `S2a2R2surv`, `S2a2R5surv`). Both shells: rc 0, PASSED | `exported GIT_DIR cannot redirect the scan` (R1, R2), `a restrictive umask decides nothing` (R5) | wire (R1), ctlrun | 124 / 219 |
| `fd90b4b1` | **remove**: the relcwd and cfgkept prefix-drop records (`e271cb02`): the marks and their checks went, replaced by the evidence check's PATH-prefix demand; a call-site drop is the opt-in records' class | `a relative scratch dir is removed on exit`, `the caller's git CONFIGURATION survives the routing purge` | controls | — |
| `fd90b4b1` | **add**: R3, cfgkept's PATH prefix withheld at the delivery (that prefix only; `ctlrun:`), and R4, the wire's `_git` stepping past a first `PATH` entry ending `/fakegitcfg`. Each is killed by the evidence check (`PATH prefix never reached the run`) and survives with that check disabled (R3: `S2b2R3surv`); R4 also survives with cfgkept's mark moved back to the shim's entry (`S2b2R4surv`). Both shells: rc 0, PASSED | `the caller's git CONFIGURATION survives the routing purge` | ctlrun (R3), wire (R4) | 124 / 219 |
| `0d411df2` | **add**: k2's `_ctl_need="$_gopt_ok"` replaced by 0 (the `fifotracked` record's shape; `_gopt_ok` is the harness's probe of the grep the run resolves): "declined while this machine's capability flag is 1", so it dies only where `_gopt_ok` is 1; where it is 0 the decline is the right answer and the record survives (NOT EXERCISED, rc 0). It survives with `_cn_declined`'s red arm removed: measured at `20615255` on both shells as a `.mutant.99` copy (`S4bGoptsurv`), the record alone KILLED with its needle, rc 1; with that arm removed too, rc 0, PASSED, k2 NOT EXERCISED | `_ctl_need declines exactly what this machine cannot do` | controls | 125 / 220 |
| CR-B1 | **remove**: k2's `_ctl_need` prefix record (`0d411df2`: the `_ctl_need` prefix's value replaced by 0): k2 is posed on every machine through the `goptgrep` shim (`…-cr-plan.md` §2.1), so it has no `_ctl_need` prefix and its call has nothing to decline; the opt-in record that pins k2 is the `unset GREP_OPTIONS` record (needle `a caller's GREP_OPTIONS cannot hide a file`), killed on a grep that honours the variable and on one that does not, and its survive clause is the shim's `GREP_OPTIONS` arm removed on a grep that ignores the variable | `a caller's GREP_OPTIONS cannot hide a file` | controls | 127 / 222 |
| CR-B2a | **remove**: the two `controls` records of `_ctl_need` (`d976f239`: the `fifotracked` caller's value replaced by 0; a declined direct `_ctl_run "$_mg_lbl"` on the census's line). Their subject, the `_ctl_need` prefix, the `_cn` standing check and the asks that read `.declined`, is replaced by the capability table (`…-cr-plan.md` §2.2): a caller no longer states a need, so there is no value to replace by 0 and no decline after a last ask | `_ctl_need declines exactly what this machine cannot do` (the label itself is gone) | controls | — |
| CR-B2a | **re-aim**: the record that takes the gate's answer as an early return again before `_ctl_run`'s channel validation (`|| return $?`; `d976f239`/CR-S2, needle `_ctl_need declines exactly what this machine cannot do`) now has the needle `$_cap_lbl` (`the gate declines exactly the members of a group whose flag is 0`). The expression is unchanged; it dies at `_cap`'s row `M 0 1 yes X`, the malformed channel under a decline, which comes back 2 instead of 1 | `the gate declines exactly the members of a group whose flag is 0` | ctlrun | — |
| CR-B2a | **add**, the capability table's gate and lookup, nine records on `$_cap_lbl`, eight killed with their needle by the `_cap` row that names it as the first red (`_cap` replaces the table and the flags per row, so each dies on every machine; the ninth is the reader's validation, below): the lookup `_cap_group_cb`'s equality made always-true (`[ x != x ]`) and made a substring match (row N); `_cap_flag_cb`'s equality made always-true (the member's OWN group's flag, rows Y); the decline arm's `return 2` made `:` (the decline rows `M 0 1 yes` and `Y 1 0`); the trust test made `if false` with the trust check placed after the capability arm (the untrusted row); the no-flag arm's `return 1` made `return 0` (an undeclared group: row U is the first red, F1 and F2 red too); the reader's flag-name validation made vacuous (`[0-9]x\|x[!A-Za-z0-9_]x`: killed on bash 5.3 not by a row but through the verdict marker, because its subject is the `${!x-}` abort that ends `_cap`'s subshell, which the standing checks' end marker turns into a labelled red, design memo §3, W2's paragraph; on bash 3.2 there is no abort and row F1 kills it); `_cap`'s own arms (`U:yes:-)` renamed, so row U has no arm) and the parse guard (see the review-fix row below for its record). Killed with the label on both shells (CR-B2a code half, 26 records re-trialed, 52 trials) | `the gate declines exactly the members of a group whose flag is 0` | ctlrun, controls | — |
| CR-B2a | **add**, the table check's own red, from the real table: a `member` row's group misspelt (`member<TAB>fifo<TAB>an unstorable entry neither hangs nor hides` becomes `fifx`). The gate's red for that member's label is the second thing to fail, so the table check is what the needle names | `the capability table is well formed and names every absence it is asked about` | controls | — |
| CR-B2a | **add**, the census's membership check (`_mut_cap_member`, through `_cap_rows member`): a `member` row whose label no `_control` of this run made (`an unreadable file fails closed` becomes `closeX`), red under `$_cen_lbl` ("declared capability member … is no `_control` this run made") | `every label the run used is named by a mutation record` | controls | 136 / 231 at the code half |
| CR-B2a review fixes | **re-aim**, the parse guard's record: the row `M 0 1 yes - 2 -` of `_cap`'s table made `M 0 1 yes - 2 -x` (a last field the loop's pattern does not match), where the code half renamed the untrusted row `M 0 1 no` to `nop`. The renamed row still ran as a malformed row once the guard's arm was removed and came back 2, not 1, so the guard was not what killed it; the re-aimed row's malformed text can only be reported by the guard (its survive clause, the guard's `*)` arm made `*) ;;`, is green on both shells) | `the gate declines exactly the members of a group whose flag is 0` | controls | — |
| CR-B2a review fixes | **add**, the table check's three other clauses, each killed under `$_capt_lbl` alone (each survives with its own clause disabled, both shells): a probe's flag holding 2 (`harness:s/then _fifo_ok=1;/then _fifo_ok=2;/`: the gate runs any value but 0, so only this clause reds the value); a member label listed twice (the `perm` group's `an unsearchable dir fails closed` row relabelled `an unreadable file fails closed`); a group declared twice (the `perm` group row copied over a member row, in one expression: the wire's anchor check wants each expression to change a line, and the removed member then simply runs, so nothing else reds) | `the capability table is well formed and names every absence it is asked about` | harness, controls | — |
| CR-B2a review fixes | **add**, the verdict marker of each standing check, one record each: an abort (`: "${_no_such_name_x}"`) injected at the start of the `_wm`, `_ev` and `_cap` subshells, so the check ends before its verdict and the `then` branch must red `the check ended before its verdict` under the check's own label (the marker arm itself is not pinned apart from them: with its `then` line removed each record dies for the wrong reason) | `only the watchdog's marker makes a signal status a timeout`, `a previous run's channel evidence never answers for the next`, `the gate declines exactly the members of a group whose flag is 0` | controls | 142 / 237 |
| CR-B2b | **add**, the FIFO probe's flip (`_cap_check`'s absence check, its group clause): `harness:s/then _fifo_ok=1;/then _fifo_ok=0;/`, the probe answering 0 on a machine that holds a FIFO, so the `fifo` group's flag is 0 and nobody declared it. Killed with `$_fifo_lbl` by `_cap_check`'s one red line (`group "fifo" is absent on this machine … and its caller did not declare it`) on both shells; it survives with the group's absence clause removed (rc 0, needle absent, both shells; measured at the code commit `567b22e8`). A machine that really lacks the capability and declares it excuses this record by the table's flag (CR-B2c, `…-cr-plan.md` §2.4: it survives there and is reported as not exercisable, the `fifo` group named; killed as named where the declaration is cut from the mutant's environment, measured on a no-FIFO shim, both shells) | `this machine holds a FIFO, or its caller declared that it cannot` | harness | — |
| CR-B2b | **add**, the permission probe's flip: `harness:s/^_perm_ok=1$/_perm_ok=0/`, the probe answering 0 where a mode-000 file is unreadable. Killed with `$_perm_lbl` by the same red line on both shells; it survives with the group's absence clause removed (both shells). On a declaring machine that CAN read a mode-000 file (root: the probe answers 0 there) it is excused by the table's flag (CR-B2c, root simulation, both shells: reported as not exercisable, the `perm` group named) | `this user cannot read a mode-000 file, or its caller declared that it can` | harness | — |
| CR-B2b | **add**, P-f's definition test forced to the limit branch (the `if [ "$_pfrc" -ne 0 ] && [ ! -s … ] && [ -s … ]` line replaced by `if true`) on an `env` that accepts `-0`, so the window reports the limit `P-f — …` and nobody declared it. Killed with `$_pfl_lbl` by `_cap_check`'s limit clause on both shells; it survives with that clause removed (both shells). On an `env` that lacks `-0` with `P-f` declared it is excused by the P-x half (`_pfl_lbl` is its limit label; CR-B2c, simulation, both shells); on a capable machine the declaration excuses nothing, because the parent's limit list has no `P-f` | `this env lists its environment NUL-separated, or its caller declared that it cannot` | harness | 145 / 240 |
| CR-C1 | **re-aim**: the census's variable-half record onto `mutations:s/^  for _cen_v in [$]_cen_used; do/  for _cen_v in; do/` (rule 1: the loop reads the names the text EXPANDS, `$_cen_used`, no longer `$_lbd_names`); same needle. Killed as named, both shells (one-record trial at the code commit `2ac70030`) | `every label the run used is named by a mutation record` | mutations | — |
| CR-C1 | **add**, a name's one expansion deleted (claim row GC-1 of `…-cr-plan.md`; GC-2…5 below likewise): the harness line that expands `$_pkl_lbl` (`if [ -n "$_pkl" ]; then echo "!! CONTROL NOT EXERCISED ($_pkl_lbl)" …`) removed, so the name is only defined and mentioned; rule 1 counts no use, and the `_pkl_lbl` records are orphans. Killed as named, both shells (one-record trial at `2ac70030`) | `every label the run used is named by a mutation record` | harness | — |
| CR-C1 | **add**, two calls the bijection sees and the registry alone could not (GC-2): relcwd's `if ! _ctl_run "$_rel_lbl" …` made `if false && _ctl_run …`, and a `_control` behind a branch (a `zz new` control after `_cap_check`, run only where `_fifo_ok` is 0). Relcwd's is a call line of the controls file with no registered site on every machine. The `zz new` one is that on a machine with a FIFO (the branch is not taken: a call line with no site); on a no-FIFO declaring machine the branch runs, the site exists, and the kill is the ratchet (19 labels against 18; measured at the fix-round head: one-record trial on P4, `LACKS=fifo` + the no-FIFO shim, both shells, `0 not killed as named`). Killed as named, both shells (one-record trials at `2ac70030`, P1) | `every label the run used is named by a mutation record` | controls | — |
| CR-C1 | **add**, the three standing checks' row loops emptied (GC-3: `for _wm_row in abort hang`, `for _ev_row in ran stale` and `_cap`'s `while read -r _ca_t …`): the check's one call line registers nothing. Killed as named, both shells (one-record trials at `2ac70030`) | `every label the run used is named by a mutation record` | controls | — |
| CR-C1 | **add**, a `_control` folded into a command substitution that spans two lines (GC-4, a two-expression record: the call lands on a site that is not its text line), and the registry's writer adding a fourth field (GC-5, a `ctlrun:` record: a row of the wrong shape). Killed as named, both shells (one-record trials at `2ac70030`) | `every label the run used is named by a mutation record` | controls (GC-4), ctlrun (GC-5) | 153 / 248 |
| CR-C1 | **add**, a name's one expansion deleted with the name left in a trailing comment on another line (the fix round, I-1; a two-expression `harness:` record: the `_pkl_lbl` NOT EXERCISED line deleted, and `  # $_pkl_lbl` appended to the un-anchored `    _pkl=bad` line). Rule 1 cuts the text from a `#` that begins a word before matching, so the name has no expansion and its records are orphans. Killed as named under `$_cen_lbl`, both shells (one-record trial at the fix-round head); with the cut removed (`if (match(s, /(^|[[:space:]])#/)) s = substr(…)` deleted) the record SURVIVES, both shells | `every label the run used is named by a mutation record` | harness | 154 / 249 |
| CR-C1 | **add**, a name's one expansion deleted with the name left in a single-quoted span on another line (fix round 3; a two-expression `harness:` record: the `_pkl_lbl` NOT EXERCISED line deleted, and ` ; : '$_pkl_lbl'` appended to the un-anchored `    _pkl=bad` line, inert). Rule 1 drops each single-quoted span after the comment cut, so the name has no expansion and its records are orphans. Killed as named under `$_cen_lbl`, both shells (one-record trial at the fix-round-3 head); with the span strip removed the record SURVIVES, both shells (the oracle: 131 cases, 0 false green, bash 5.3 and 3.2, measured outside the repo; the figures are stated here, the logs are not copied) | `every label the run used is named by a mutation record` | harness | 155 / 250 |

- **Unchanged, re-attributed:** the records "`.git/config` replaced by a symlink" and "`.git/HEAD` is a
  FIFO" keep their text and target. After C9 they die by the census's shape pass rather than by the
  deleted case split and two-name guard.
- **Current totals** (the one statement; earlier per-commit totals are in the table above and in git):
  **155 records of this PR, 250 in all**, measured at the CR-C1 slice's fix-round-3 commit (which adds the single-quoted-span record above, 249 before; 249 was measured at the fix-round commit, which adds the trailing-comment record, 248 before; 248 was measured at the CR-C1 slice's code commit, which adds the eight records above, 240 before, and re-aims the variable-half record; 240 was measured at the CR-B2b slice's commit, which adds the three probe-flip records, 237 before; 237 was measured at the CR-B2a slice's commit with its review fixes, which removes the two `_ctl_need` records, adds seventeen and
  re-aims two: the capability table, eleven at the code half, 231, and six in the review fixes, the table check's three clauses and the three verdict markers; 222 at the CR-B1 slice's commit, which removes k2's `_ctl_need` prefix record:
  k2 is posed on every machine through a shim, so it has no `_ctl_need` prefix; 223 at the CR-S3 slice's commit, which removes P-b's four
  definition-test records, whose test went with K2's version-keyed limit, and adds the `var --`, the misspelt-name
  and the forced-129 records; 224 at the CR-S2 slice's commit, which added the `pin`
  control's swapped exit/message record and the record that restores the gate's early return before
  `_ctl_run`'s channel validation, and the record that weakens the evidence directory's fresh `rm -rf … && mkdir …`
  to `mkdir -p …`; 221 at the CR-S1 slice's commit, 220 at `0d411df2`) with
  `bash -c '. .claude/tools/webref-generic-core-trip-wire.records.sh; _mutants | wc -l'` (250; 240 at the CR-C1 slice's base) against
  the same count of `e8f78896`'s here-document (95). `_MUT_RECORDS_MIN=250` is a **floor**: fewer
  records is red, more is not; its comment in `…mutations.sh` names every record this PR removed, by
  commit. `_MUT_UNRECORDED_MAX=18` (21 at the CR-B2a slice's commit; 18 at the CR-B1 slice's commit; 19 before the CR-S2 slice's `pin` record): the three labels CR-B2a had left bare are the capability table's group and limit labels, `$_fifo_lbl`, `$_perm_lbl` and `$_pfl_lbl`, which the census then counted as `_*_lbl` names the parts spell (since CR-C1 it counts the names the wire or a part expands: the three are still counted, through the table's expansion); since CR-B2b each has a record (the three probe-flip records above, which pin the absence check that arrived with the declarations), which brought the figure back to 18, and CR-C1 left it at 18 (the bijection fixes which sites exist; the rows and labels per site are the same because every call site passes a fixed label, so the figure is the same on every machine; measured 18 at the code commit `2ac70030` under three PATH profiles). The ratchet is **exact**: fewer bare labels than that is red
  too ("LOWER the ratchet").
- **ROg** (§6, E3: the exit moved below the first control, window complete) went red with C8, because
  the verdict function then set `_fw_trusted`; with trust decided by the window it is green again on
  both shells at `6ed414b2` (`…-pr527-r15.md` §Q).
- **Not records, by the caller's `PATH`:** the X14 outcomes. A record's run uses the caller's `PATH`, so
  no record can pose them.
- **Cost (X3).** No new record runs to a bound. The FIFO records end in seconds, because the census
  stops them before git runs (`…-landing.md` X16). Only X3's survive check runs a FIFO record with its
  clause removed; there it waits, by the FIFO procedure above.
- **Retired with C6:** the "resolving relative `PATH` entries" bullet under "What is not a record"
  above describes the normaliser C6 deletes.
