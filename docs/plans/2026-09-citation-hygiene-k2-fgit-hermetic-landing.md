# K2 fixture git — the commits, the land order and the exit criteria

This file holds §9 (the commit plan, the land order and the ledger text) and §11 (the exit criteria)
of `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md` ("the design memo"). Both were moved here
unchanged as a touch-time split, before draft 16's edits would have taken the design memo past 1000
lines; the design memo keeps a pointer at each section. The section numbers are kept, so "§9", "§9.1"
and "§11" resolve here. Every other section reference without a file name is to the design memo.

---

## §0 Spec coverage map

**No spec surface** — this file holds the commit plan and exit criteria of a shell-harness slice. It has
the same by-design shape as the design memo's §2.5: `preflight.py` exits 1 here because no table follows
this heading, and the repo-wide preflight command runs over every tracked plan memo.

---

## §9 Commit plan and land order (base `e8f78896`)

| # | commit | kind | verified by |
|---|---|---|---|
| C0 | this memo, the companion, the umbrella rows | docs | the Step-4.5 focused check (E2 and E3) |
| — | **plan-review gate** | | |
| C0b | parent: supersession banner (eleven sites) | docs | the `…-reviews.md` §8.2 command, re-read |
| C1 | controls → controls + fixtures; one parts list; sibling guard in `_mut_run`; references qualified | prereq split | X1, X4, X4b |
| C2 | mutations → mutations + mutgen; references qualified | prereq split | X1, X3, X4, X4b |
| C3 | record comments | infra (§0.2) | X3 |
| C4 | the `harness:` and `fixtures:` record targets with resolver and restore; the harness's `_shq` comment, which said the mutation set "has nothing to aim at", rewritten because C4 makes it false (`…-reviews.md` §13) | infra, required | X3 |
| C5 | the window (§3), with the fixtures calling `git` and ending with the `built` line; `notcommitted`'s `mkdir`; the incomplete-window exit with its named causes; W2 as `_control`'s first statement and after the controls; W3; the per-option pins and the options re-check; §4's postconditions including P-g's census; the 20 records (50 at head; how they came to be: `…-pr527.md`); §3's two comment texts; the ratchet population; `…-reviews.md` §8.1's in-file rewrites; the `ci.yml` line re-derived by its own rule | feature | X1–X3, X5, X6, X8, X11 |

**Cost, and `ci.yml`.** The base job comment "a wire that adds fixture self-tests re-derives this line
in the same PR" is an in-file rule (`git show e8f78896:.github/workflows/ci.yml | sed -n
'/^  trip-wires:/,/^  [a-z]/p'`). C5 re-derived the line by that rule's method then (three local runs
per side, alternated) and wrote the method and one verdict line. Draft 17 retires that method (D15-B):
the budget is read from the job's run on GitHub's runner, as #510 now does (below).

- **Where the record goes.** The final-head sequence's step 2 (§9.1) rewrites the job comment's method
  and verdict paragraph. It states the method: the `Layering trip-wires` job itself, measured from a
  runner run, no local host and no scaling, with the command
  `gh run view <run-id> --json jobs --jq '.jobs[]|select(.name=="Layering trip-wires")|[.startedAt,.completedAt]'`
  (#510's). It says the verdict for this PR's tool code is pending that run (X9), and carries **no
  figure**, scaled or local. The verdict, when written, names the run and the commit it ran.
- **Where the runner figure comes from, and who writes it.** `ci.yml` runs only for pull requests into
  `main`, and #527 targets `webref-cite-audit-tool`, so #527 has no runner run of its own. The figure
  comes from X9's run: route (b), #501's CI after #527 is squashed into #501, or route (a), a temporary
  draft PR to `main` at T. The citation-hygiene lane, which owns #501, reads the job time from that run
  and writes the verdict into the job comment: under (b) as a commit on #501's branch before #501
  merges (the same stop condition as X9's), under (a) in step 2's commit.
- **The threshold.** If the runner-measured job time is half the budget in force or more, **STOP and
  escalate to the user**: 150 s against this PR's base `timeout-minutes: 5`, or 300 s if #510's
  `timeout-minutes: 10` is on `main` by then. Half is #510's own rule for this job ("When it nears 2x,
  split the wire or re-derive the timeout deliberately", its job comment at `106387e4`).
- **X8 is a sanity record only.** Its local runs at T (three per side, alternated, on the branch and on
  base `e8f78896`) go to `…-pr527-r15.md` §Q. They show whether the window changed the local cost
  grossly; they decide nothing, and no figure from them goes to `ci.yml`. Draft 16's threshold, the
  slowest local run times 4.3 against 150 s, is withdrawn: its source was #510's local-to-runner pair,
  which #510 has since retired (below).
- **Resolvable commits.** After #501's squash the PR branch's commits are unreachable from `main`, so a
  verdict naming one names it as a ref that stays fetchable, as #510's comment does: `git fetch origin
  pull/527/head`, then the SHA.
- **Interaction with #510.** #510 rewrites the same job, and its head moves: read it at the time with
  `gh pr view 510 --json headRefOid`, and its `ci.yml` with `gh api
  'repos/send/elidex/contents/.github/workflows/ci.yml?ref=<sha>' --jq .content | base64 -d`. At
  `106387e4` (read 2026-09-30) its budget block keeps **one** figure, "the job itself on the runner --
  no local host, no scaling", says "Re-measure from a run, not from this line", moves its local figures
  to history because they "had different subjects", and asks for the job to be re-measured on the runner
  when `claim-provenance-trip-wire.sh` lands. It sets `timeout-minutes: 10` and carries no K2 rule; the
  4.3× pair draft 16 used is no longer in it (`/usr/bin/grep -c '4\.3'` → 0). If #510 lands first, its
  convention governs the block, and #501, carrying #527, is the later lander: the lane re-measures the
  job on the runner under it. If #501 lands first, #510 is the later lander and re-measures on the
  runner by its own rule. Nothing is carried forward textually (the umbrella withdrew "whichever lands
  second is a textual merge").
- **Interaction with the `stale-claim-detector` lane.** Its local branch (`01b645e6`) adds
  `claim-provenance-trip-wire.sh` to `scripts/trip-wires.sh`'s `REQUIRED_WIRES`. Its base does not
  contain `e8f78896` (`git merge-base --is-ancestor e8f78896 stale-claim-detector` fails), so its list
  lacks the K2 wire; whichever lands later merges the list and re-measures the job on the runner.
- **Ownership.** The budget itself, the job's `timeout-minutes` across lanes, is unowned (parent §6;
  umbrella, Cross-lane coordination); the verdict for this PR's tool code is the citation-hygiene
  lane's (above).
- **Cost shape.** There is still **one** build. It runs in a child process, with the postconditions and
  P-g added.

**X9 cannot run on this PR as stacked.** `ci.yml` triggers only on `pull_request: branches: [main]`,
and this PR targets `webref-cite-audit-tool` (#519 ran zero checks). The GNU/Linux evidence therefore
needs one of two routes, and **the user decides which at push time**:
- **(a) A temporary draft PR from this branch to `main`**, opened solely to run CI and closed
  afterwards. Its diff would also carry #501's changes, because this branch sits on #501, so it must
  not be reviewed or merged as such.
- **(b) #501's own CI after the squash** serves as the GNU evidence. **Stop condition:** if that CI goes
  red on anything this slice touched, or its job time reaches the threshold above, a fix (or, for the
  time, the user's decision) lands before #501 merges.

Either route's run is also the only runner run of this PR's tool code, so the budget verdict comes from
it ("Where the runner figure comes from", above).

**Land order:**
1. `/pre-push` over `e8f78896...k2-wire-fgit-hermetic`.
2. User approval to push and open the PR, stacked on `webref-cite-audit-tool`.
3. `/external-converge`, to TERMINAL.
4. The final-head sequence (§9.1), which ends in the user's approval to squash.
5. Resolve #501's Codex P2 thread.
6. Update the umbrella's single status cell — done inside #527 (it names the PR, which stays true after the squash; PR #527 Codex R19).
7. Hand the #501 merge decision to the user.
8. After #501 lands, update the ledger entries with #501's merge commit on `main`. This PR's squash SHA
   is unreachable from `main`.

**Ledger text.** The ledger convention is registration **before** merge. At PR creation the
orchestrating session wrote the two removals, the section deletion, an amend and a −2 header banner
(`project_open-defer-slots.md`, the 2026-09-28 banner). Drafts 11–17 change that text, so the ledger is
written **once more, in one step**, in the final-head sequence (§9.1, step 4). That step leaves the ledger holding
exactly this:
- **Remove** `#11-k2-fgit-keepset-depends-on-git-purge-glob`, citing §5.1 (dissolved: no keep-set, no
  `_fgit`).
- **Remove** `#11-k2-fixture-git-invocation-convention`, citing §0.3 and §5.1: its residual is class
  (c), declared out of scope and owned by code review; the create-time audit finds no owed work and no
  trigger.
- **With both gone, delete their section too:** the heading "Citation-hygiene — K2 wire `_fgit` scrub
  fix: 2 own slots" and its Source paragraph hold nothing else, so they are removed with the two entries
  rather than left orphaned.
- **Amend** `#11-k2-wire-exit-trap-masks-set-u-abort` with §5.2's ledger text, verbatim, **replacing**
  in place both the entry's "(its memo U5 …)" sentence and the amendment written at PR creation.
- **Register** `#11-trip-wire-liveness-bound` with §5.2's text, verbatim.
- **Amend** `#11-trip-wire-launch-environment`. Member (3) gains the K2 window: git pinned as the
  caller's shell resolves it from the wire's directory, the other entries verbatim. The fired trigger
  "#519 lands" becomes "the next PR that changes how a required wire, or the K2 fixture window,
  resolves or passes `PATH`". The PM lane stays the owner.
- **Fix the header count.** The net change is −1 (one slot dissolved, one closed, one registered),
  and it **replaces** the −2 banner.

### §9.1 Drafts 11–17: the commits planned on top of `8413a4db`

Drafts 11–17 are to be implemented as four commits on the PR branch; none has been made. C7 (draft
12's time bound) and X12 (draft 12's watchdog cells) are withdrawn, and neither number is reused. Each
commit is to be green on both shells and to set `_MUT_RECORDS_MIN` to its own record count; the record
changes are in corpus §6.1.
- **Per commit**, the light checks: X1, X4b and X10.
- **The final-head sequence** (land order step 4) runs after `/external-converge` reaches TERMINAL. The
  final head is the head at squash time. Every check below measures the tool code or the job that runs
  it, so its subject is **T**: the last commit that changes `.claude/tools/**` or `scripts/**`, **or the
  `trip-wires` job of `.github/workflows/ci.yml` outside its comment lines** (its `timeout-minutes`,
  `runs-on`, steps). The command is below. The checks re-run only when T moves:
  1. At T, unless already recorded for this T: X2, X3 (with corpus §6.1's survive check), X5, X6, X8,
     X11, X13, X14, X15 and X16.
  2. One commit records the results in `…-pr527-r15.md` §Q and rewrites the job comment's method and
     verdict paragraph (§9, "Where the record goes"). It changes comment lines only: `git diff
     --name-only <T> HEAD -- .claude/tools scripts` is empty and the job's non-comment lines are equal
     at T and HEAD, so T does not move and it re-runs nothing.
  3. One fresh Codex round on the new head, because the head moved: `/external-converge` counts only a
     dry verdict on the head, and the global hook `~/.claude/hooks/gh-pr-merge-head-guard.sh` refuses
     `gh pr merge` while Codex's latest assessed commit is not the head. Dry: step 4. A finding: fix it.
     - A fix that changes the tool code or the job (its `timeout-minutes` included) moves T: back to
       step 1.
     - A docs-only fix that changes a check's definition (a row of §11, or corpus §6.1) runs the changed
       check at T and records the result in the fix commit itself. T does not move: back to step 3.
     - Any other fix: back to step 3.
  4. The one ledger step (§9, "Ledger text"). It writes no commit, so the head does not move.
  5. The user's squash approval.

  **Terminator:** step 3 is dry on a head whose T is unchanged since step 1 last ran. Only a Codex
  finding whose fix moves T returns the sequence to step 1; the sequence's own commit (step 2) never
  does, and neither does a docs-only fix, which carries its own re-run.

  ```sh
  # T: the last commit after e8f78896 that changes the tool code, or the trip-wires job outside comments
  _job() { git show "$1:.github/workflows/ci.yml" | sed -n '/^  trip-wires:/,/^  [a-z]/p' | /usr/bin/grep -v '^ *#'; }
  for c in $(git log --format=%H e8f78896..HEAD); do
    [ -n "$(git diff --name-only "$c^" "$c" -- .claude/tools scripts)" ] && { echo "$c"; break; }
    [ "$(_job "$c^")" = "$(_job "$c")" ] || { echo "$c"; break; }
  done
  ```
- **X9** runs on GitHub's ubuntu runner, which a stacked PR does not reach, so it runs when §9's route
  (a) or (b) happens, not at the final head.

The history goes to `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-pr527-r15.md` §Q, not to the
provenance companion, which is at 972 lines (`wc -l`), nor to `…-pr527.md`, which is closed at round 14.

| # | commit | what | this PR's / all records |
|---|---|---|---|
| C6 | M-PATH | delete the normaliser (the `_fp_*` loop and its comment block); `_FGIT_PATH="$_FGIT_BIN:$PATH"`; P-j keeps its first-entry and `git` clauses and loses the per-entry one; P-j's label renamed (§4); the harness comments that cite "memo §0.1" and "memo §1" without the file name go with the block (X4b is **not** empty on them at `8413a4db`) | 49 / 144 |
| C8 | untrusted build | `_fgit_window_verdict_exit` (renamed from `_fgit_window_incomplete_exit`) reports the window's verdict in one place, with W3's report moved into it; it is the one writer of `_fw_trusted`, and `_fw_built_or_w2` its one reader; the run ends unless the build is complete and trusted; W2's label renamed, and W2's existing record re-anchored to the new name | 51 / 146 |
| C9 | M-SHAPE | the two-pass census at the start of the postconditions, its verdict `_pg_census` written once after both passes (classification and shape, one-link regular files included; then the per-link search over a clean pass 1); a red census returns before any other postcondition, so no git runs; the postconditions' directory, a fresh `mktemp -d` beside `$_FW_DIR` made at the start of the postconditions, holding every file a postcondition writes and reads back (the census lists, the per-link results, `env -0`'s output, P-g's population and listings; D15-A) and, after a clean census, the references (P-a's and P-d's probe repos by `mkdir` without `-p`, P-g's reference listings) (design memo §4, "The references and the working files"); the P-f and P-g records anchored on `$_FW_DIR/env0` and `$_FW_DIR/pgcur` re-anchored; the non-directory-link arm inside a `.git`, the in-loop per-link search and the two-name `HEAD`/`config` guard deleted. The census comment's "There is no time bound" sentence is rewritten to name `#11-trip-wire-liveness-bound`, and Codex R25's "nothing here has a watchdog" comment is rewritten for the census | 59 / 154 |
| C10 | P-k | `count-objects -v` per compared `.git` over a clean census, red on any `alternate:` line, a non-zero exit or any stderr; its liveness probe (C-quoted path), a third repo in the postconditions' directory, with no machine-limitation arm; two labels | 63 / 158 |

C8 comes before C9, so an untrusted build already stops the run when C9's records plant a FIFO.

After C10 the land order resumes at **step 1**: `/pre-push` over the new commits, `mise run ci`
included. The PR is stacked on `webref-cite-audit-tool`, so no CI runs on it, and the local gate is the
only gate before step 3 (`/external-converge`). Step 2, the push confirmation, may be skipped, because
push confirmation is required only when a PR is opened or merged. After TERMINAL comes the final-head
sequence above (step 4). Draft 17 moves the working files, reads the budget from a runner run and
widens T to the job, so plan-review round 16, a focused re-check of the draft-17 delta, runs before C6
(design memo §12).

## §11 Exit criteria

These run on both shells, and on both gits wherever the corpus has a column.

| id | command | expected |
|---|---|---|
| X1 | the X1 block below | `rc=0`, `0`, `1` |
| X2 | the corpus §6 G cells (its recipe, on the implementing head as `p6/`) | all PASS with P equal, 4 configs |
| X3 | at T only (§9.1; it is one control pass per record): `WEBREF_WIRE_MUTANTS=1 $SH $W`, then `/usr/bin/grep -F -e 'entr(ies), 0 not killed as named' -e ', 0 neither killed nor argued equivalent' -e 'trip-wire PASSED'`; then corpus §6.1's survive check, by hand: each §6.1 record run with the clause it names removed | three hits in every column. C1–C3 are byte-identical to base. Each §6.1 record survives with its clause removed; a FIFO record reaches the alarm instead (§6.1's procedure) |
| X4 | C1/C2: X1's log at the parent commit and at the split commit, with scratch paths normalised by one `sed`, then `diff` | empty |
| X4b | the X4b block below, over each file that C1, C2, C6, C8, C9 or C10 edits (the harness is edited by C6, C8, C9 and C10) | empty (at `8413a4db` the harness gives 2 lines, 71 and 93, which C6 deletes) |
| X5 | the corpus §6 R, RLOUD, RES and P cells, same recipe; the draft-10 cells (companion §E.7) on the implementing head as `p11/`; and drafts 11–17's cells (corpus §6.1) | R and RES green with P equal; RLOUD red; P PASS; every AFTER row PASS; each draft-11–17 cell as corpus §6.1 says |
| X6 | two by-hand edits to `…trip-wire.fixtures.sh`, each just before its `: > "$_FW_DIR/built"` line: `mkdir -p "$_FGIT_VOID" && printf 'ref: refs/heads/x\n' > "$_FGIT_VOID/HEAD" && : > "$_FGIT_VOID/config"`, and `git -C "$CTL/clean" config include.path /nonexistent-k2`; one run each | red: the template through **P-c**, the include through **P-g**. Not P-d: P-d compares two inits that both use the void as their template, so a template planted there is on both sides. Not P-a: P-a reads one probe repo, not the fixtures |
| X8 | at T (§9.1), a sanity record only (§9, "X8 is a sanity record only"): `/usr/bin/time -p bash scripts/trip-wires.sh`, three runs on the branch and three on base `e8f78896`, alternated | the six times recorded in `…-pr527-r15.md` §Q; nothing decided and nothing written to `ci.yml`. The budget verdict is X9's |
| X9 | `Layering trip-wires` on ubuntu (GNU), via route (a) or (b) of §9, chosen by the user at push time; the job's time from that run, by §9's `gh run view` command | SUCCESS, and the job's time under half the budget in force (§9, "The threshold"; at or above it, STOP), the verdict written as §9 says. The run is also GNU evidence for `env -i`, `env -0`, the window, the prelude, and the census's `find` expressions under GNU find: `find "$CTL" \( -iname .git -print0 \) -o \( -iname HEAD -print0 \)` (pass 1's list), `find <.git> \( \( -type f -links +1 \) -o \( ! -type f ! -type d \) \) -print` (pass 1's shape), `find "$CTL" -type l -print0` (pass 2's link list) and `find -L <link> \( -iname HEAD -o -iname .git \) -print` (pass 2's search). If a record's sed expression reads differently under GNU sed, the always-on anchor check in `_mut_correspondence` is the first thing to fail, on the ordinary run |
| X10 | `$SH -n` over every `.claude/tools/webref-generic-core-trip-wire*.sh`, and `wc -l` over the parts this PR edits (controls, fixtures, harness, mutations, mutgen) | clean; each edited part below 1000 lines. The wire itself (1259 lines at `8413a4db`) is not edited by this PR, so the rule does not reach it |
| X11 | in a clone, add to `…trip-wire.controls.sh`, after the other `_lbl` definitions, `_x_lbl="an unrecorded probe"` and `echo "$_x_lbl" >/dev/null` | red, and the ratchet lists it |
| X12 | withdrawn with draft 12's time bound (draft 13); the number is not reused | — |
| X16 | at T, by hand, with the cell script `cell15.sh` verbatim in `…-pr527.md` (round 13): the four Codex R26③ FIFO cells of the table "R26③, measured" (`commondir`, `HEAD`, `config`, `config.worktree` in `clean/.git`), round 13's cells `xcommon`, `bareh`, `outsidein`, `hard`, `xlink`, `xincreg` and `clean`, round 14's `xrefpoison` and `xrefctl`, and round 15's `lnblind2` (its insert verbatim in `…-pr527-r15.md` §Q), one run per cell and shell | each red cell: rc 1, P-g, the untrusted-build stop runs no control, and the run ends in under 40 s; `clean` PASSED. The 40 s is five times the slowest red cell measured on the draft-15 prototype (`xincreg`, 8 s on bash 3.2; on draft 16's, also 8 s). The waiting cells (`hdless`, `xinclude`, `xinclnk`, `outroot`, `ttyinc` under a terminal) are not exit criteria: they wait, R9 |
| X13 | the `crc.sh` script, verbatim in `…-pr527.md`, at T: the clean tree and eight cells (six red; env0 and reftable green; `garbagehead` is an untrusted build), with the parent's nounset off (`NOU=1`) and on, on bash 3.2 and 5.3 | the same exit status, NE/CF counts and first four `!!` lines both ways, on both shells |
| X14 | at T, each run by `cell15.sh`'s method (`…-pr527.md`, round 13: its own process group, `perl -e 'setpgrp; alarm 120; exec @ARGV'`, then `kill -9 -<pgid>` of that group only): PX1's caller `PATH` (`~+/bin` first, a logging `git` in the repository's `bin/`); Codex R23's `tools/bin` wrapper with its interpreter beside it; Codex R24's `~/bin` wrapper with a helper | each ends within 120 s, PASSED or red with a named cause; reaching the alarm fails the criterion. 120 s is about five times the slowest clean run measured on the draft-15 prototype (24 s, bash 3.2). For PX1 the expected result is PASSED with the wrapper as the fixture git (§0.1). The outcomes are recorded in `…-pr527.md` |
| X15 | by hand, before `built`: `git init -q "$CTL/zzx" && mkdir -p "$_FW_DIR/zzt/a" && : > "$_FW_DIR/zzt/a/HEAD" && ln -s "$_FW_DIR/zzt" "$CTL/zzx/.git/k2link"` | rc 1, P-g, whose message names `zzx` by the shape rule only; no "symlink to a tree holding a git dir" entry for `k2link`, so the search did not run behind it |

Commands containing `|` are kept out of table cells, because `\|` in a markdown cell is read one way
raw and another way rendered, and that made one of X1/X4b vacuous. They use `-e` alternatives instead.
Each was re-run to show it discriminates (companion §A.10).

```sh
# X1 — a green log gives 0 and 1; a red one gives 1 and 0 (measured on c7 f0011 and f0013)
( $SH $W ) > $S/l 2>&1; echo "rc=$?"
/usr/bin/grep -c -e 'CONTROL NOT EXERCISED' -e 'CONTROL FAILED' $S/l
/usr/bin/grep -c 'trip-wire PASSED' $S/l

# X4b — base mutations.sh gives 2 lines (15, 20); the same file with both qualified gives 0.
# The old in-table form '§[0-9]\|plan memo\|the memo' (raw, no -i) gave 0 on base, i.e. vacuous.
/usr/bin/grep -n -i -E -e '§[0-9]' -e 'plan memo' -e 'the memo' <file> | /usr/bin/grep -v 'citation-hygiene-[A-Za-z0-9-]*\.md'
```
