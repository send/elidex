# K2 fixture git — the corpus and the mutation records

This file holds the corpus — the evidence behind the design — and the mutation records derived from it,
for `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md` ("the design memo"). It was split out of
the design memo's §6 as a touch-time split (`35dc1153`), before the design memo grew past 1000 lines.
At the split the text was unchanged, except that five references to other sections of the design memo
were made to name it. Later drafts changed it: §6.1 (draft 11's record plan, revised by drafts 12–20) and one
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

**Records: representative only.** The table and the totals below are the set at `8413a4db`; drafts
11–15 change them as §6.1 says.

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

- **Totals:** **16 labels and 50 records** (how each came to be: `…-pr527.md`). `_MUT_TARGETS="harness fixtures"`: the prefix parts; a record
  with no prefix edits the wire.
- **The `fixtures:` prefix:** it is a BSD `sed` error ("invalid command code f"); GNU is unmeasured.
- **The ratchet:** **`_MUT_UNRECORDED_MAX` stays at 21**, and `_MUT_RECORDS_MIN` rises by exactly 50 (95 → 145).
- **What is not a record:**
  - RES cells, because the runner requires the `!survive` needle exactly once (`mutations.sh:658–662` at `e8f78896`);
  - the exit number, which is unpinnable (design memo §3);
  - the two INFO cells, which are informative;
  - resolving relative `PATH` entries: it changes something only for a caller whose `PATH` has one, and a
    record's run uses the caller's `PATH`. Measured by hand (Codex R23's shape, both shells): a `tools/bin`
    `git` wrapper with a `#!/usr/bin/env` interpreter beside it — red on `cb8b0e09`, PASSED after.
- **Records on a machine that cannot evaluate their postcondition** (a `machine_limits` line: P-b on git
  before 2.42, P-f on an `env` without `-0`): the mutation run still runs them — a record pinning the
  limitation test's own narrowness dies there — and a **survival** of one is reported as not exercisable
  on that machine instead of counted (and the summary says how many); the labels are derived from the limitation's
  ID. The excuse is per label, so on such a machine a regression in a record that would die there is excused
  too — a machine with the capability (CI's GNU `env`) runs them all. A "killed for the wrong reason" (2)
  is never excused: it is also how a harness break reports; a leaking caller can make it a false red.

**Mutation-mode cost (X3).** Each run of X3 is (95 base records + 50) record trials plus the generated
population, one control pass each. X3 prints the counts, and X8 gives the per-pass time. This is
opt-in and does not add to the always-run gate.

### §6.1 Drafts 11–20: the record changes (planned, `…-landing.md` §9.1)

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
`perl -e 'setpgrp; alarm 120; exec @ARGV'`. It is expected to reach the alarm with
`ps -ax -o pid=,pgid=,command=` showing the P-g `git … config --list` of that group blocked; then that
group alone is killed with `kill -9 -<pgid>`, so nothing is left behind.

| commit | change | label | target | this PR's / all |
|---|---|---|---|---|
| — | at `8413a4db` | | | 50 / 145 |
| C6 | **delete**: "a relative entry after `$_FGIT_BIN` in the window's `PATH`". There is no per-entry clause left to pin | P-j | harness | 49 / 144 |
| C6 | **re-anchor**: "the window's `PATH` starts at `/usr/bin`" becomes "a directory with no `git` placed before `$_FGIT_BIN`" (`/nonexistent-k2:` in front), which pins the first-entry clause alone. The `gitx` record still pins the `git` clause. Measured (`…-pr527-r15.md` §Q, the `Jm` cells, bash 5.3): the expression applied to the draft-19 prototype, which has the clause, gives rc 1, P-j `first-entry`; applied to the draft-18 prototype, which lacked it, rc 0, PASSED, the survive check | P-j | harness | — |
| C6 | **relabel**: P-j's two records take the label `the fixture build window runs the pinned git first on its PATH` | P-j | — | — |
| C8 | **add**: one harness expression inside `_fgit_window_verdict_exit` that forces a postcondition red (`post_bad` written unconditionally) and removes the untrusted-build exit. W2 is reported by `_control`'s gate, `_fw_built_or_w2`, which decides by `_fw_trusted` alone. It survives with the `post_bad` clause removed from `_fgit_window_verdict_exit`'s computation of `_fw_trusted` (the writer), which then writes 1 | W2 | harness | 50 / 145 |
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
| C9 | **add**: `xrefpoison` (round 14): the fixtures make `$_FW_DIR/pq/a` and `$_FW_DIR/pq/b` in advance, the old reference path, as repos with `core.excludesFile /nonexistent-k2`, and write the same key into every fixture repo's `.git/config`. Killed by P-g against the references made after the build (`…-pr527.md` round 14: rc 1 in 5–7 s on both shells). It survives with the references made at the old fixed path (`_pq="$_FW_DIR/pq"; mkdir -p "$_pq/a" "$_pq/b"`): measured on the draft-15 prototype, rc 0, PASSED on both shells. No record removes the `mkdir`-without-`-p` freshness assertion: the directory `mktemp -d` has just made holds nothing, so no run can present a stale repo there, and that mutant is equivalent **within class (b)**: a fixture command still running after the build can place one there (`xrace`, round 15), which is class (c) and not owed (design memo §0.3) | P-g | **fixtures** | 59 / 154 |
| C9 | **add**: `lnblind2` (round 15): a link `$CTL/zzy` to a tree in `$_FW_DIR` holding a git dir (`zzt/a` with `objects` and `HEAD`), and `$_FW_DIR/prel` and `$_FW_DIR/pgls`, draft 16's pass-2 link list and in-loop per-link result, made links to `/dev/null`. Killed by P-g's pass 2 once the working files are in the postconditions' directory (`K2PRE zzy:[a symlink to a tree holding a git dir]`, rc 1 in 3–5 s on both shells, `…-pr527-r15.md` §Q). It survives with the working files back in `$_FW_DIR`: the draft-16 prototype gives rc 0, PASSED on both shells. C9 deletes the in-loop search, so the record plants the pass-2 list alone (round 15's `lnblind` insert, verbatim with its source in `…-pr527-r15.md` §Q, which the draft-16 prototype reds only through that search: `zzy:[a symlink to a tree holding a git dir: a/HEAD]`, bash 5.3). That it survives on the implementation with the working files back in `$_FW_DIR` is expected, not measured; X3's survive check measures it | P-g | **fixtures** | 60 / 155 |
| C9 | **re-anchor**: the P-f record "`env -0`'s output gains a last record with no NUL after it" (`mutations.sh:360` at `8413a4db`) and the P-g record "the missing-lines `grep` reads a missing pattern file" (line 364) name `$_FW_DIR/env0` and `$_FW_DIR/pgcur`, which C9 moves into the postconditions' directory (`$_pq`); both expressions name the new path | P-f, P-g | harness | — |
| C10 | **add**: `clean` gets an `objects/info/alternates` naming an object store outside the fixture root. It survives with the `alternate:` clause removed | P-k | **fixtures** | 61 / 156 |
| C10 | **add**: the liveness probe loses its `alternates` file, so it prints no `alternate:` line: NOT EXERCISED, red. It survives with the liveness check removed. With no machine-limitation arm, no machine excuses it | P-k liveness | harness | 62 / 157 |
| C10 | **add**: `count-objects -v` replaced by a command that fails with no output. A failed count is red, and the record survives with the status check removed (the empty output then has no `alternate:` line, so it reads green) | P-k | harness | 63 / 158 |
| C10 | **add**: `clean` gets an `alternates` naming a store that does not exist: rc 0, no `alternate:` line, an error on stderr. It survives with the stderr clause removed | P-k | **fixtures** | 64 / 159 |
| C11 | **no change** for the move: the parent's working files move out of `$CTL` (`…-landing.md` §9.1), and no record anchors their paths: over the `MUTANTS` here-document (`mutations.sh` lines 230–387 on the draft-18 prototype), `/usr/bin/grep -n` with the alternatives `-e 'CTL/\.' -e 'CTL\\/\.' -e fsmhook -e fsmonitor_ran -e control_out -e fifoprobe` finds one line, the `.gd-cachedir` record, a fixture's path. `xbare` and `xanchor` are X16 cells, not records: each needs a tree edit (the ratchet at 0, a stale record) besides its insert, and a record is one expression. Draft 19 put the empty-generated-set check (D17-D) in the opt-in run, where no record could reach it (a trial runs the wire with `env -u WEBREF_WIRE_MUTANTS`); draft 20 makes it always on, and the next row pins it | — | — | 64 / 159 |
| C11 | **add** (D18-B): `mutgen:s/^_mut_regex_mutants() {$/_mut_regex_mutants() { return 0/`, a `mutgen` target (`_MUT_TARGETS` gains it), labelled `the boundary-mutant generator derives a non-empty set from the wire's regexes`. The generator then emits nothing, and the always-on `_mut_gen_floor` reds under that label, rc 1 on both shells (`floorkill`); it survives with the `_mut_gen_floor` call removed from `_mut_correspondence`, rc 0, PASSED (`floorsurv`, bash 5.3; `…-pr527-r15.md` §Q). Draft 21's run-time layer (`_mut_gen_n` against `_mut_gen_floor_n` in `_mut_run`, D19-A) adds **no record**: it runs only in the opt-in run, which no trial reaches; X3's floor cell pins it | generator floor | mutgen | 65 / 160 |
| C12 | **add**: `exec 2>/dev/null` just before the fixtures' `built` line. Killed by W ("the fixtures file left the window's stderr redirected"): on the draft-18 prototype this insert alone gave rc 2 (bash 5.3), and with `: $((1/0))` after it (`xexecarith`) rc 2 on both shells (`…-pr527-r15.md` §Q). It survives with the canary check removed: without the check (the draft-16 prototype) the insert alone gave rc 0, PASSED (bash 3.2), and `xexecarith` rc 0, PASSED on both shells on the draft-17 prototype (round 16) | W | **fixtures** | 66 / 161 |

- **Unchanged, re-attributed:** the records "`.git/config` replaced by a symlink" and "`.git/HEAD` is a
  FIFO" keep their text and target. After C9 they die by the census's shape pass rather than by the
  deleted case split and two-name guard.
- **Totals after C12:** **19 labels** (P-k and its liveness label and the generator floor's label
  added; P-j and W2 renamed), **66 records of this PR, 161 in all**: `_MUT_RECORDS_MIN=161`, and
  `_MUT_UNRECORDED_MAX` stays at 21.
- **Not records, by the caller's `PATH`:** the X14 outcomes. A record's run uses the caller's `PATH`, so
  no record can pose them.
- **Cost (X3).** No new record runs to a bound. The FIFO records end in seconds, because the census
  stops them before git runs (`…-landing.md` X16). Only X3's survive check runs a FIFO record with its
  clause removed; there it waits, by the FIFO procedure above.
- **Retired with C6:** the "resolving relative `PATH` entries" bullet under "What is not a record"
  above describes the normaliser C6 deletes.
