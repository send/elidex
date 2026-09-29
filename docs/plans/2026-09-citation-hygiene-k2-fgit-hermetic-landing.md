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
'/^  trip-wires:/,/^  [a-z]/p'`). C5 re-derives the line by that rule's own method, and records the
method and the verdict without figures. If the value would change, **STOP and escalate to the user**.

- **Where the record goes.** C5 replaced the job comment's re-derivation paragraphs with the method
  (three runs per side, alternated) and one verdict line. That line is in a commit, so it cannot name
  its own commit. It names what X8 measured: base `e8f78896` and **the tool code at T**, where T is the
  last commit on the PR branch that changes `.claude/tools/**` or `scripts/**` (`git log -1 --format=%H
  e8f78896..HEAD -- .claude/tools scripts`). The job's time is the wires' own: the job comment says it
  "tracks the number of controls, not the size of the repo", so a commit that changes neither path
  leaves the line true. After C6–C10 the line is stale until X8 measures T and rewrites it (§9.1).
- **The threshold.** Take the slowest of the three head runs and multiply it by 4.3. That factor is
  the one runner-to-local pair this repository records for a trip-wires run, not a proven worst case.
  It comes from PR #510's trip-wires comment: that PR's python self-test wire in mutant mode ran 49 s
  locally and 211 s on Codex's runner (`git show 400086ee:.github/workflows/ci.yml`, the comment above
  `trip-wires:`). If the product is 150 s or more, **STOP**. 150 s is half of the budget this PR's base
  sets, `timeout-minutes: 5`. Equivalently, STOP when the slowest head run is 150 / 4.3 = 34.883… s or
  more. If #510 has landed first with its 10 minutes, the later lander re-derives against the budget
  then on `main` (the next bullet).
- **Resolvable commits.** The verdict line names `e8f78896` and T, and after #501's squash the PR
  branch's commits are unreachable from `main`. X8 names them as refs that stay fetchable, as #510's
  comment does: `git fetch origin pull/527/head`, then the SHA.
- **Interaction with #510.** #510 rewrites the same job. Its head moves, so read it at the time with
  `gh pr view 510 --json headRefOid`. At `5fb94705` (read 2026-09-29; its `ci.yml` lines 128–140, the
  4.3× source, are unchanged from `400086ee`) it sets `timeout-minutes: 10` and carries no K2 rule. The umbrella
  withdrew "whichever lands second is a textual merge", so the later lander **re-derives** the line
  under both rules. It does not carry a paragraph forward.
- **Interaction with the `stale-claim-detector` lane.** Its local branch (`01b645e6`) adds
  `claim-provenance-trip-wire.sh` to `scripts/trip-wires.sh`'s `REQUIRED_WIRES`. Its base does not
  contain `e8f78896` (`git merge-base --is-ancestor e8f78896 stale-claim-detector` fails), so its list
  lacks the K2 wire; whichever lands later merges the list and re-derives the job's line by the rule.
- **Ownership.** The budget half is unowned (parent §6; umbrella, Cross-lane coordination).
- **Cost shape.** There is still **one** build. It runs in a child process, with the postconditions and
  P-g added.

**X9 cannot run on this PR as stacked.** `ci.yml` triggers only on `pull_request: branches: [main]`,
and this PR targets `webref-cite-audit-tool` (#519 ran zero checks). The GNU/Linux evidence therefore
needs one of two routes, and **the user decides which at push time**:
- **(a) A temporary draft PR from this branch to `main`**, opened solely to run CI and closed
  afterwards. Its diff would also carry #501's changes, because this branch sits on #501, so it must
  not be reviewed or merged as such.
- **(b) #501's own CI after the squash** serves as the GNU evidence. **Stop condition:** if that CI goes
  red on anything this slice touched, a fix lands before #501 merges.

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
(`project_open-defer-slots.md`, the 2026-09-28 banner). Drafts 11–16 change that text, so the ledger is
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

### §9.1 Drafts 11–16: the commits planned on top of `8413a4db`

Drafts 11–16 are to be implemented as four commits on the PR branch; none has been made. C7 (draft
12's time bound) and X12 (draft 12's watchdog cells) are withdrawn, and neither number is reused. Each
commit is to be green on both shells and to set `_MUT_RECORDS_MIN` to its own record count; the record
changes are in corpus §6.1.
- **Per commit**, the light checks: X1, X4b and X10.
- **The final-head sequence** (land order step 4) runs after `/external-converge` reaches TERMINAL. The
  final head is the head at squash time. Every check below measures tool code, so its subject is
  **T**, the last commit that changes `.claude/tools/**` or `scripts/**` (§9, "Where the record goes"),
  and it re-runs only when T moves:
  1. At T, unless already recorded for this T: X2, X3 (with corpus §6.1's survive check), X5, X6, X8,
     X11, X13, X14, X15 and X16.
  2. One commit records the results in `…-pr527.md` §P and rewrites the `ci.yml` verdict line to name
     `e8f78896` and T. It changes no tool code (`git diff --name-only <T> HEAD -- .claude/tools
     scripts` is empty), so it re-runs nothing.
  3. One fresh Codex round on the new head, because the head moved: `/external-converge` counts only a
     dry verdict on the head, and the global hook `~/.claude/hooks/gh-pr-merge-head-guard.sh` refuses
     `gh pr merge` while Codex's latest assessed commit is not the head. Dry: step 4. A finding: fix it;
     if the fix changes tool code, T moves and the sequence returns to step 1, otherwise to step 3.
  4. The one ledger step (§9, "Ledger text"). It writes no commit, so the head does not move.
  5. The user's squash approval.

  **Terminator:** step 3 is dry on a head whose tool code is T's and whose `ci.yml` line names T. Only
  a Codex finding whose fix changes tool code returns the sequence to step 1; the sequence's own
  commit (step 2) never does.
- **X9** runs on GitHub's ubuntu runner, which a stacked PR does not reach, so it runs when §9's route
  (a) or (b) happens, not at the final head.

The history goes to `…-pr527.md`, not to the provenance companion, which is at 972 lines
(`wc -l docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-provenance.md`).

| # | commit | what | this PR's / all records |
|---|---|---|---|
| C6 | M-PATH | delete the normaliser (the `_fp_*` loop and its comment block); `_FGIT_PATH="$_FGIT_BIN:$PATH"`; P-j keeps its first-entry and `git` clauses and loses the per-entry one; P-j's label renamed (§4); the harness comments that cite "memo §0.1" and "memo §1" without the file name go with the block (X4b is **not** empty on them at `8413a4db`) | 49 / 144 |
| C8 | untrusted build | `_fgit_window_verdict_exit` (renamed from `_fgit_window_incomplete_exit`) reports the window's verdict in one place, with W3's report moved into it; it is the one writer of `_fw_trusted`, and `_fw_built_or_w2` its one reader; the run ends unless the build is complete and trusted; W2's label renamed, and W2's existing record re-anchored to the new name | 51 / 146 |
| C9 | M-SHAPE | the two-pass census at the start of the postconditions, its verdict `_pg_census` written once after both passes (classification and shape, one-link regular files included; then the per-link search over a clean pass 1); a red census returns before any other postcondition, so no git runs; the references (P-a's and P-d's probe repos, P-g's reference listings) made after a clean census in a fresh `mktemp -d` beside `$_FW_DIR`, the repos by `mkdir` without `-p` (design memo §4, "The references"); the non-directory-link arm inside a `.git`, the in-loop per-link search and the two-name `HEAD`/`config` guard deleted. The census comment's "There is no time bound" sentence is rewritten to name `#11-trip-wire-liveness-bound`, and Codex R25's "nothing here has a watchdog" comment is rewritten for the census | 58 / 153 |
| C10 | P-k | `count-objects -v` per compared `.git` over a clean census, red on any `alternate:` line, a non-zero exit or any stderr; its liveness probe (C-quoted path), a third repo in the references' directory, with no machine-limitation arm; two labels | 62 / 157 |

C8 comes before C9, so an untrusted build already stops the run when C9's records plant a FIFO.

After C10 the land order resumes at **step 1**: `/pre-push` over the new commits, `mise run ci`
included. The PR is stacked on `webref-cite-audit-tool`, so no CI runs on it, and the local gate is the
only gate before step 3 (`/external-converge`). Step 2, the push confirmation, may be skipped, because
push confirmation is required only when a PR is opened or merged. After TERMINAL comes the final-head
sequence above (step 4). Draft 16 changes the references' location and the final-head sequence, so
plan-review round 15, a focused re-check of the draft-16 delta, runs before C6 (§12).

## §11 Exit criteria

These run on both shells, and on both gits wherever the corpus has a column.

| id | command | expected |
|---|---|---|
| X1 | the X1 block below | `rc=0`, `0`, `1` |
| X2 | the corpus §6 G cells (its recipe, on the implementing head as `p6/`) | all PASS with P equal, 4 configs |
| X3 | at T only (§9.1; it is one control pass per record): `WEBREF_WIRE_MUTANTS=1 $SH $W`, then `/usr/bin/grep -F -e 'entr(ies), 0 not killed as named' -e ', 0 neither killed nor argued equivalent' -e 'trip-wire PASSED'`; then corpus §6.1's survive check, by hand: each §6.1 record run with the clause it names removed | three hits in every column. C1–C3 are byte-identical to base. Each §6.1 record survives with its clause removed; a FIFO record reaches the alarm instead (§6.1's procedure) |
| X4 | C1/C2: X1's log at the parent commit and at the split commit, with scratch paths normalised by one `sed`, then `diff` | empty |
| X4b | the X4b block below, over each file that C1, C2, C6, C8, C9 or C10 edits (the harness is edited by C6, C8, C9 and C10) | empty (at `8413a4db` the harness gives 2 lines, 71 and 93, which C6 deletes) |
| X5 | the corpus §6 R, RLOUD, RES and P cells, same recipe; the draft-10 cells (companion §E.7) on the implementing head as `p11/`; and drafts 11–16's cells (corpus §6.1) | R and RES green with P equal; RLOUD red; P PASS; every AFTER row PASS; each draft-11–16 cell as corpus §6.1 says |
| X6 | two by-hand edits to `…trip-wire.fixtures.sh`, each just before its `: > "$_FW_DIR/built"` line: `mkdir -p "$_FGIT_VOID" && printf 'ref: refs/heads/x\n' > "$_FGIT_VOID/HEAD" && : > "$_FGIT_VOID/config"`, and `git -C "$CTL/clean" config include.path /nonexistent-k2`; one run each | red: the template through **P-c**, the include through **P-g**. Not P-d: P-d compares two inits that both use the void as their template, so a template planted there is on both sides. Not P-a: P-a reads one probe repo, not the fixtures |
| X8 | the ci.yml rule's derivation (§9), measured at T (§9.1) | method and verdict recorded, the line naming `e8f78896` and T; STOP on change |
| X9 | `Layering trip-wires` on ubuntu (GNU), via route (a) or (b) of §9, chosen by the user at push time | SUCCESS. This is GNU evidence for `env -i`, `env -0`, the window, the prelude, and the census's `find` expressions under GNU find: `find "$CTL" \( -iname .git -print0 \) -o \( -iname HEAD -print0 \)` (pass 1's list), `find <.git> \( \( -type f -links +1 \) -o \( ! -type f ! -type d \) \) -print` (pass 1's shape), `find "$CTL" -type l -print0` (pass 2's link list) and `find -L <link> \( -iname HEAD -o -iname .git \) -print` (pass 2's search). If a record's sed expression reads differently under GNU sed, the always-on anchor check in `_mut_correspondence` is the first thing to fail, on the ordinary run |
| X10 | `$SH -n` over every `.claude/tools/webref-generic-core-trip-wire*.sh`, and `wc -l` over the parts this PR edits (controls, fixtures, harness, mutations, mutgen) | clean; each edited part below 1000 lines. The wire itself (1259 lines at `8413a4db`) is not edited by this PR, so the rule does not reach it |
| X11 | in a clone, add to `…trip-wire.controls.sh`, after the other `_lbl` definitions, `_x_lbl="an unrecorded probe"` and `echo "$_x_lbl" >/dev/null` | red, and the ratchet lists it |
| X12 | withdrawn with draft 12's time bound (draft 13); the number is not reused | — |
| X16 | at T, by hand, with the cell script `cell15.sh` verbatim in `…-pr527.md` (round 13): the four Codex R26③ FIFO cells of the table "R26③, measured" (`commondir`, `HEAD`, `config`, `config.worktree` in `clean/.git`), round 13's cells `xcommon`, `bareh`, `outsidein`, `hard`, `xlink`, `xincreg` and `clean`, and round 14's `xrefpoison` and `xrefctl`, one run per cell and shell | each red cell: rc 1, P-g, the untrusted-build stop runs no control, and the run ends in under 40 s; `clean` PASSED. The 40 s is five times the slowest red cell measured on the draft-15 prototype (`xincreg`, 8 s on bash 3.2; on draft 16's, also 8 s). The waiting cells (`hdless`, `xinclude`, `xinclnk`, `outroot`, `ttyinc` under a terminal) are not exit criteria: they wait, R9 |
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
