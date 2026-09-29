# K2 fixture git — the commits, the land order and the exit criteria

This file holds §9 (the commit plan, the land order and the ledger text; §9.2 the two ledger texts,
moved later from the design memo's §5.2) and §11 (the exit criteria)
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

- **Where the record goes.** The final-head sequence's step 2 (§9.1) rewrites every local-derivation
  passage of the trip-wires comments in `.github/workflows/ci.yml`, so no local method survives beside
  the runner method. At `8413a4db` they are: in the block above the job, the sentence "Derive it if you
  need it (`/usr/bin/time -p bash scripts/trip-wires.sh`)" and its "re-derived when a wire lands"
  (lines 146–149); in the job's comment, "Derive it instead:" with the same command (lines 174–175),
  the rule "a wire that adds fixture self-tests re-derives this line in the same PR" (lines 176–179),
  and "THE METHOD" with the verdict (lines 180–185); and the job comment's opening claim "RE-DERIVED WHEN
  THE K2 WIRE LANDED, not inherited" (line 167), which claims the local re-derivation this step
  retires. When step 2 runs, no runner verdict exists yet to take its place, so the step drops the claim
  and keeps that comment's reason (the K2 wire's fixture self-tests are the first thing in the job whose
  cost is a large multiple of the scan it guards). The line numbers are `git show
  8413a4db:.github/workflows/ci.yml | /usr/bin/grep -n -e 'Derive it' -e 'THE RULE' -e 'THE METHOD' -e
  'Verdict at' -e 'RE-DERIVED WHEN'`. In their place the comment states the runner method, the `Layering trip-wires` job
  itself measured from a run, no local host and no scaling, with #510's command
  `gh run view <run-id> --json jobs --jq '.jobs[]|select(.name=="Layering trip-wires")|[.startedAt,.completedAt]'`,
  and the rule that a PR adding fixture self-tests re-measures the job that way. The comment carries
  **no figure**, then or later: as #510's block at `9df03f52` does ("NO FIGURE IS KEPT HERE: the
  measurement, its history and the growth driver have one home, the plan's §8 entry"), the figure's one
  home is a plan record, here `…-pr527-r15.md` §Q, written on #501's branch after X9 (next bullet).
- **CLAUDE.md's trip-wires paragraph (D17-C).** Line 94 of `CLAUDE.md` is #501's branch content, not
  main's: `git diff origin/main...e8f78896 -- CLAUDE.md` shows #501 rewriting that paragraph, and its
  text carries a local method, "必要なら導出する (`/usr/bin/time -p bash scripts/trip-wires.sh`)", and
  the rule "`timeout-minutes` は wire が増えた PR で再導出する", which main never had (`git show
  origin/main:CLAUDE.md | /usr/bin/grep -c 'time -p bash scripts/trip-wires.sh'` → 0). #501's squash
  would put that local method on `main` beside the runner method. #527 lands into #501, so **#527
  fixes it in #501's branch**, in the same step-2 commit: the local-method sentence is removed, and the
  closing parenthesis says the job's comment is the one home of the budget's method, so the paragraph
  states no method of its own. It does **not** go back to main's text, which still says "(~1s)" (`git
  show origin/main:CLAUDE.md | /usr/bin/grep -c '(~1s)'` → 1): #501 removed that figure because it was
  false, and restoring it would put a figure where the job comment says none belongs. Step 2 then
  changes a file outside `.claude/tools`, `scripts` and the job's non-comment lines, so T still does not
  move.
- **Where the runner figure comes from, and who writes it.** `ci.yml` runs only for pull requests into
  `main`, and #527 targets `webref-cite-audit-tool`, so #527 has no runner run of its own. X9 is **route
  (b), decided by the user when #527 was opened** (2026-09-28). The decision is recorded in the
  citation-hygiene lane's memory of that PR's creation, not in the PR; PR #527's description records
  the route itself: "after this squashes into #501, #501's `Layering trip-wires` job is the GNU
  evidence; if it reds, a fix lands before #501 merges". So the figure comes from #501's CI after the squash, and the citation-hygiene
  lane, which owns #501, writes the verdict, the run and its job time, into `…-pr527-r15.md` §Q as a
  commit on #501's branch (land order step 7).
- **The threshold.** If the runner-measured job time is half the budget in force or more, **STOP and
  escalate to the user**: 150 s against this PR's base `timeout-minutes: 5`, or 300 s if #510's
  `timeout-minutes: 10` is on `main` by then. The line is this memo's own choice. #510's rule, "When it
  nears 2x, split the wire or re-derive the timeout deliberately" (its job comment: line 174 at
  `106387e4`, line 168 at `9df03f52`), names the headroom but no line, and a STOP has to be decidable from one figure; half is the 2x
  headroom that rule protects, taken as the line.
- **X8 is a sanity record only.** Its local runs at T (three per side, alternated, on the branch and on
  base `e8f78896`) go to `…-pr527-r15.md` §Q. They show whether the window changed the local cost
  grossly; they decide nothing, and no figure from them goes to `ci.yml`. Draft 16's threshold, the
  slowest local run times 4.3 against 150 s, is withdrawn: its source was #510's local-to-runner pair,
  which #510 has since retired (below).
- **Resolvable commits.** After a squash the PR branch's commits are unreachable from `main`, so a
  verdict naming one names it as a ref that stays fetchable, as #510's comment does: `git fetch origin
  pull/527/head` for #527's commits, and `git fetch origin pull/501/head` for #501's, among them the
  run's commit and the verdict commit of step 7, then the SHA.
- **Interaction with #510.** #510 rewrites the same job, and its head moves: read it at the time with
  `gh pr view 510 --json headRefOid`, and its `ci.yml` with `gh api
  'repos/send/elidex/contents/.github/workflows/ci.yml?ref=<sha>' --jq .content | base64 -d`. At
  `106387e4` (read 2026-09-30) its budget block keeps **one** figure, "the job itself on the runner --
  no local host, no scaling", says "Re-measure from a run, not from this line", moves its local figures
  to history because they "had different subjects", and asks for the job to be re-measured on the runner
  when `claim-provenance-trip-wire.sh` lands. It sets `timeout-minutes: 10` and carries no K2 rule; the
  4.3× pair draft 16 used is no longer in it (`/usr/bin/grep -c '4\.3'` → 0). At `9df03f52` (read
  2026-09-30, the same day) the block keeps **no** figure: "NO FIGURE IS KEPT HERE", the measurement's one
  home is its plan's §8 entry, and "Measure the job on the runner itself" with the same `gh run view`
  command; `timeout-minutes: 10` and no K2 rule, as before. If #510 lands first, its
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
and this PR targets `webref-cite-audit-tool` (#519 ran zero checks). X9 is **route (b)**, decided by the
user (above): #501's own CI after the squash is the GNU evidence and the only runner run of this PR's
tool code, so the budget verdict comes from it too. **Stop condition:** if that CI goes red on anything
this slice touched, or its job time reaches the threshold above, a fix (or, for the time, the user's
decision) lands before #501 merges (land order step 7).

**Land order:**
1. `/pre-push` over `e8f78896...k2-wire-fgit-hermetic`.
2. User approval to push and open the PR, stacked on `webref-cite-audit-tool`.
3. `/external-converge`, to TERMINAL.
4. The final-head sequence (§9.1), which ends in the user's approval to squash.
5. Resolve #501's Codex P2 thread.
6. Update the umbrella's single status cell — done inside #527 (it names the PR, which stays true after the squash; PR #527 Codex R19).
7. **X9 on #501**, after #527 is squashed into #501:
   1. #501's CI runs the `Layering trip-wires` job on GitHub's runner.
   2. The lane reads the job's time with the `gh run view … --jq` command above and writes the verdict
      commit on #501's branch: the run, the commit it ran and the time, against the budget in force, in
      `…-pr527-r15.md` §Q (no figure in `ci.yml`).
   3. The threshold: at half the budget or more, STOP and hand it to the user.
   4. **When T moves on #501's branch after the squash, by a fix or by a merge** (a merge of `main`, or
      7.5's conflict resolution, that changes the tool code or the job), §9.1 step 1's local checks run at
      the new T on #501's branch, because they measure the tool code at T whatever moved it, and the
      sequence **returns to 7.1** (D17-B): the job runs again on the runner at the new T, and the new
      run's verdict is written in `…-pr527-r15.md` §Q, beside the old one, which it supersedes. Step 8 is
      reached only when X9's run at the current T is SUCCESS and under the threshold, or, over the
      threshold, the user has decided at 7.3 for that T (one predicate with X9's row, §11: a red run for
      an unrelated wire stops the driver early and times short, so a time alone is not a verdict). The
      verdict commit changes a plan
      file only, so it moves no T, but it moves #501's head, and #501 gets a fresh Codex round on it (the
      rule of §9.1 step 3). **Terminator:** Codex is dry on the verdict commit of a run at the current T;
      a finding whose fix moves T returns to 7.1, and any other fix is docs-only and returns to the Codex
      round.
   5. If #510 has landed first, #501's `ci.yml` conflicts with `main` in the trip-wires comments. The
      merge resolution comes first, and #510's convention governs it; a pull request with a conflict is
      expected to get no `pull_request` run (not measured here), so the re-measure follows the
      resolution.
8. Hand the #501 merge decision to the user.
9. After #501 lands, update the ledger entries with #501's merge commit on `main`. This PR's squash SHA
   is unreachable from `main`.

**Ledger text.** The ledger convention is registration **before** merge. At PR creation the
orchestrating session wrote the two removals, the section deletion, an amend and a −2 header banner
(`project_open-defer-slots.md`, the 2026-09-28 banner). Drafts 11–18 change that text, so the ledger is
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
- **Amend** `#11-k2-wire-exit-trap-masks-set-u-abort` with §9.2's ledger text, verbatim, **replacing**
  in place both the entry's "(its memo U5 …)" sentence and the amendment written at PR creation.
- **Register** `#11-trip-wire-liveness-bound` with §9.2's text, verbatim.
- **Amend** `#11-trip-wire-launch-environment`. Member (3) gains the K2 window: git pinned as the
  caller's shell resolves it from the wire's directory, the other entries verbatim. The fired trigger
  "#519 lands" becomes "the next PR that changes how a required wire, or the K2 fixture window,
  resolves or passes `PATH`". The PM lane stays the owner.
- **Fix the header count.** The net change is −1 (one slot dissolved, one closed, one registered),
  and it **replaces** the −2 banner.

### §9.2 The ledger texts (moved from the design memo §5.2)

Moved here from the design memo's §5.2 (unchanged at the split `d6ccc5b1`), beside the ledger step (§9,
"Ledger text") that writes them, before draft 18's edits would have taken the design memo past 1000
lines. Draft 18 then added C12's stderr cause and record to the second text.
Section references in them are to the design memo.

**R9's slot, `#11-trip-wire-liveness-bound`: the text §9's ledger step writes.**
- **Gap**: the K2 wire's harness bounds only `_control`'s child (30 s; #501 R92, present at base
  `e8f78896`). Every other child that runs over fixture state is unbounded, so a block there gives no
  green, but the gate reaches no verdict: CI reds at the job timeout, and a local run waits.
  - **Pre-existing at `e8f78896`**:
    - the plain `--selftest` calls of the relative-scratch and umask blocks, and the fsmonitor
      block's `git ls-files` and `--selftest`;
    - the fixture build, then run in the parent through `_fgit`;
    - the mutation runner's trials.
  - **New in #527**:
    - the fixture build window, which replaces the parent build;
    - C9's two-pass census (the classification and shape pass, then the per-link `find -L` search,
      which #527's census already ran at `8413a4db`);
    - C10's `count-objects` per repo and its probe.
  - **Measured blocks**: a read that does not complete, which git makes through a reference outside
    every census git dir. Each ran as its own process group and reached its 120 s limit on both
    shells; the inserts and the cell scripts are verbatim in `…-pr527.md`, rounds 13–14:
    - `outroot`: a `commondir` naming a git dir outside the fixture root, its `config` a FIFO;
    - `hdless`: a `commondir` naming a directory with `objects` and `refs` but no `HEAD`, its `config` a
      FIFO;
    - `xinclude`: an `include.path` naming a FIFO in the fixture root;
    - `xinclnk`: an `include.path` naming a link in the fixture root to a FIFO outside it;
    - `ttyinc`: an `include.path` naming `/dev/tty`, under a controlling terminal (without one, git
      exits 128 and the run is red).

    A chained `alternates` whose store's own `alternates` is a FIFO blocks `git count-objects -v`, P-k's
    read: rc 142 at a 10 s alarm on git 2.55.0 (a git-only measurement; `pkmeas.sh`, verbatim in
    `…-pr527.md` round 14).
- **Defects measured in the two withdrawn designs** (`…-pr527.md`, rounds 10–11; rounds 12–14 then
  measured the blocks above):
  1. nested process groups escape an outer group kill, and `$(…)` then waits on the pipe;
  2. a call-site list misses sites (fsmonitor's `git ls-files`);
  3. a re-run of the wire as a new group becomes a background job on a tty, so `stty tostop` stops
     it: a false red;
  4. with trials nested, the INT/TERM exit path's `kill -9` does not reach the trial groups;
  5. a failing process-group probe re-runs the wire without end;
  6. a join decided by an environment variable alone drops every bound, against the wire's rule
     `git show 8413a4db:.claude/tools/webref-generic-core-trip-wire.sh | sed -n 350,364p` ("ENTERED BY
     ARGUMENT, NEVER BY ENVIRONMENT");
  7. a check for `set -m` by spelling misses `set -o monitor` and `-eum`;
  8. a 0-bound phase, or a top-level cap of 0, leaves call sites unbounded;
  9. a parent that relays a child's rc relays bash 3.2's masked rc 0, which widens
     `#11-k2-wire-exit-trap-masks-set-u-abort`.
- **Why deferred**: the legitimate reason is **L3**. A bound over nested children is a load-bearing
  change of its own: edge-dense, so it needs its own plan and plan-review under CLAUDE.md's rule. L2
  also holds, since #527's memo records two designs measured and withdrawn. The confirming questions:
  1. spec faithfulness: no spec surface;
  2. one issue, one way: the only bounded child, `_control`, predates #527, and #527 adds no second
     mechanism;
  3. anti-justification: no, it is not size or session;
  4. **repeat signal: yes.** Codex R25/R26 and plan-review rounds 10–14 raised it. By the lens that
     means fix-in-PR, and only the **user's explicit carve (2026-09-28)** overrides it. That decision
     is the ground here, not the lens.
- **Trigger**: #501's squash merge into `main`, which carries #527's changes (#527 is squashed into
  #501's branch, not into `main`, and its commits are not reachable from `main`) and opens the
  dedicated slice; or, before that, the next PR that adds
  a child to the K2 harness or the mutation runner, or any report of a K2 run that waited instead of
  reaching a verdict.
- **Owner**: the citation-hygiene lane. **Timing**: a slice of its own, planned and plan-reviewed after
  #501's squash merge. **Re-eval**: 2026-11-30.
- **Accounting**: #527's own deferrals, 1.

**`#11-k2-wire-exit-trap-masks-set-u-abort`: the ledger text.** The design memo's §5.2 (now in
`…-residuals.md`) describes the defect; the slot's ledger trigger is "code sourced after wire:405", and
"such code" in the next sentence means that (code sourced after wire:405).
This PR's harness and controls are exactly such code. **The ledger text — the one text; §9's ledger
step writes exactly this** — **replaces**, in place, the entry's sentence "(its memo U5: state
initialised at harness top level, setup failure a labelled verdict)" and the amendment written at PR
creation. It does not append to them:
- **measured, not argued:** with the wire's nounset off, the clean tree and eight cells (six red) give the
  same exit status and verdict lines as with it on (bash 3.2 and 5.3; the cell script is verbatim in
  `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-pr527.md`, "X13", and X13 re-runs it at T, the last
  commit of this PR that changes its tool code or its CI job, `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-landing.md` §9.1).
  On those runs no parent-side verdict depends on `set -u`; that is a measurement over those runs, not a
  proof over every path;
- an incomplete fixture-build window is reported by the **W verdict alone**, and no control runs:
  `_control` asks `_fw_built_or_w2` first, and so does each of the three blocks that are not
  `_control`s (relative scratch, fsmonitor, umask). That covers a child that refuses a prelude missing
  any one of `errexit`, `nounset` or `pipefail`, a fixtures file that stops before its last line, one
  that switches an option off, and one that leaves the window's stderr redirected. A build that is complete but untrusted (a red postcondition, or W3) ends
  the run after its reports, and no control runs over it either;
- a mode restriction a fixture sealed and the window could not apply, or refused, is red (W4);
- pinned by the W records (one per prelude option, one for the early return, one for an abort, one for
  an option switched off by the fixtures file, one for its stderr left redirected), the three W2 records and the W4 record. The three block gates
  are pinned by the traced `w2rec` cell only (§3's declared gap).

### §9.1 Drafts 11–20: the commits planned on top of `8413a4db`

Drafts 11–20 are to be implemented as six commits on the PR branch; none has been made. C7 (draft
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
  2. One commit records the results in `…-pr527-r15.md` §Q and rewrites every local-derivation passage
     of the trip-wires comments to the runner method, with no figure, and CLAUDE.md's trip-wires paragraph
     to carry no method of its own (§9, "Where the record goes" and "CLAUDE.md's trip-wires paragraph").
     After the rewrite it runs a complement check and records its output in the same commit's §Q
     entry: `/usr/bin/grep -c 'time -p bash scripts/trip-wires.sh' CLAUDE.md .github/workflows/ci.yml`
     gives 0 for each file, and `/usr/bin/grep -n -i -e 're-derive' -e 'rederive' CLAUDE.md
     .github/workflows/ci.yml` lists no line that asks for a local derivation (each remaining hit is
     named in §Q with why it is not one). It changes comment lines, `CLAUDE.md` and the `…-pr527-r15.md`
     §Q record only: `git diff
     --name-only <T> HEAD -- .claude/tools scripts` is empty and the job's non-comment lines are equal
     at T and HEAD, so T does not move and it re-runs nothing.
  3. One fresh Codex round on the new head, because the head moved: `/external-converge` counts only a
     dry verdict on the head, and the global hook `~/.claude/hooks/gh-pr-merge-head-guard.sh` refuses
     `gh pr merge` while Codex's latest assessed commit is not the head. Dry: step 4. A finding: fix it.
     - A fix that changes the tool code or the job (its `timeout-minutes` included) moves T: back to
       step 1.
     - A docs-only fix that changes a check's definition (a row of §11, or corpus §6.1) runs the changed
       check at T and records the result in the fix commit itself. T does not move: back to step 3. A
       red result is itself a finding and is handled as one: a fix that moves T goes back to step 1. Two
       rows are exceptions: a change to X9's row is run by X9's route, on #501 after the squash (land
       order step 7), not at T; and X8 is a record that cannot fail, so its re-run is recorded, not
       judged.
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
- **X9** runs on GitHub's ubuntu runner, which a stacked PR does not reach. It runs by route (b), on #501
  after the squash (land order step 7), not at the final head.

The history goes to `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-pr527-r15.md` §Q, not to the
provenance companion, which is at 972 lines (`wc -l`), nor to `…-pr527.md`, which is closed at round 14.

| # | commit | what | this PR's / all records |
|---|---|---|---|
| C6 | M-PATH | delete the normaliser (the `_fp_*` loop and its comment block); `_FGIT_PATH="$_FGIT_BIN:$PATH"`; P-j keeps its first-entry and `git` clauses and loses the per-entry one; P-j's label renamed (§4); the harness comments that cite "memo §0.1" and "memo §1" without the file name go with the block (X4b is **not** empty on them at `8413a4db`); the harness comment that cites the design memo's §5.2 (line 156 at `8413a4db`) names `…-residuals.md` §5.2 | 49 / 144 |
| C8 | untrusted build | `_fgit_window_verdict_exit` (renamed from `_fgit_window_incomplete_exit`) reports the window's verdict in one place, with W3's report moved into it; it is the one writer of `_fw_trusted`, and `_fw_built_or_w2` its one reader; the run ends unless the build is complete and trusted; W2's label renamed, and W2's existing record re-anchored to the new name; the record `s/^export LC_ALL=C$/export LC_ALL=C.UTF-8/` relabelled to P-h's label, and one record added for the scan's locale (corpus §6.1, D18-B's C8 rows) | 52 / 147 |
| C9 | M-SHAPE | the two-pass census at the start of the postconditions, its verdict `_pg_census` written once after both passes (classification and shape, one-link regular files included; then the per-link search over a clean pass 1); a red census returns before any other postcondition, so no git runs; the postconditions' directory, a fresh `mktemp -d` beside `$_FW_DIR` made at the start of the postconditions, holding every file a postcondition writes and reads back (the census lists, the per-link results, `env -0`'s output, P-g's population and listings; D15-A) and, after a clean census, the references (P-a's and P-d's probe repos by `mkdir` without `-p`, P-g's reference listings) (design memo §4, "The references and the working files"); the P-f and P-g records anchored on `$_FW_DIR/env0` and `$_FW_DIR/pgcur` re-anchored; the non-directory-link arm inside a `.git`, the in-loop per-link search and the two-name `HEAD`/`config` guard deleted. The census comment's "There is no time bound" sentence is rewritten to name `#11-trip-wire-liveness-bound`, and Codex R25's "nothing here has a watchdog" comment is rewritten for the census | 60 / 155 |
| C10 | P-k | `count-objects -v` per compared `.git` over a clean census, red on any `alternate:` line, a non-zero exit or any stderr; its liveness probe (C-quoted path), a third repo in the postconditions' directory, with no machine-limitation arm; two labels | 64 / 159 |
| C11 | the parent's verifier directory | right after `_fgit_window` returns, `_VFY="$(mktemp -d "$SCRATCH/verifyXXXXXX")"`, checked, exit 2 on failure; every file the parent writes and reads back under `$CTL` moves there (`fsmhook`, `.fsmonitor_ran`, `.fsm_out`, `.umask_out`, `.control_out`, `.mutants`, `.anchor`, `.bare`, `.genmutants`, `.genequiv`, `.genseen`), and the source-time `.fifoprobe` moves to `$SCRATCH`; after it the design memo §4 checker lists one line (D16-B). The mutation set's entry guard (`for _n in CTL …`, `mutations.sh` line 90 at `8413a4db`) and the "WHAT IT CONSUMES" headers of `mutations.sh` (line 29) and `mutgen.sh` (line 12) name `$_VFY` in place of `$CTL`. An empty generated set is red **always on** (D17-D, D18-B): `_mut_gen_floor` in `mutgen.sh`, called by `_mut_correspondence`, counts `_mut_regex_mutants`' output over the running wire's `$K2RE` and `$K2RE_PATH` and reports under a new label, `the boundary-mutant generator derives a non-empty set from the wire's regexes` (`_mg_lbl`, in the controls file); `mutgen` joins `_MUT_TARGETS`, and one record pins the check (corpus §6.1). No record anchors the moved paths. The directory is out of the fixtures' reach except through the declared class (c) of the design memo §0.3 (an executable written into a caller-`PATH` directory, `staleshim`, D18-A) | 65 / 160 |
| C12 | the window's stderr | the child writes a fixed canary to fd 2 after the options re-check; the parent requires it in the captured `stderr` before it counts the window complete (W, with its own cause sentence) and takes it out of the replay (design memo §3; D16-D) | 66 / 161 |

C8 comes before C9, so an untrusted build already stops the run when C9's records plant a FIFO.

After C12 the land order resumes at **step 1**: `/pre-push` over the new commits, `mise run ci`
included. The PR is stacked on `webref-cite-audit-tool`, so no CI runs on it, and the local gate is the
only gate before step 3 (`/external-converge`). Step 2, the push confirmation, may be skipped, because
push confirmation is required only when a PR is opened or merged. After TERMINAL comes the final-head
sequence above (step 4). Draft 20 extends C8 (the `LC_ALL` record relabelled, one record added) and
C11 (the always-on generator floor, its label, the `mutgen` target and its record), step 2's complement
check and land order step 7.4, so plan-review round 19, a focused re-check of those changes, runs
before C6 (design memo §12).

## §11 Exit criteria

These run on both shells, and on both gits wherever the corpus has a column.

| id | command | expected |
|---|---|---|
| X1 | the X1 block below | `rc=0`, `0`, `1` |
| X2 | the corpus §6 G cells (its recipe, on the implementing head as `p6/`) | all PASS with P equal, 4 configs |
| X3 | at T only (§9.1; it is one control pass per record): `WEBREF_WIRE_MUTANTS=1 $SH $W`, then `/usr/bin/grep -F -e 'entr(ies), 0 not killed as named' -e ', 0 neither killed nor argued equivalent' -e 'trip-wire PASSED'`; then corpus §6.1's survive check, by hand: each §6.1 record run with the clause it names removed; then one opt-in cell, the **plant cell** (C11): the same mutation run with `ln -s /dev/null "$CTL/.genmutants"` inserted just before the fixtures' `built` line. Each of these runs on bash 5.3 **and** 3.2 at T | three hits in every column. C1–C3 are byte-identical to base. Each §6.1 record survives with its clause removed; a FIFO record reaches the alarm instead (§6.1's procedure). The plant cell: at `8413a4db` (base) the generator reads no mutant, and the run reports `0 mutant(s)` and PASSED, the blind; after C11 the plant has no effect, and the run reports its normal non-zero `N mutant(s)` and PASSED. The draft-19 floor cell is withdrawn: the empty-set check is always on since draft 20 (D18-B), and its record's kill and survive check are §6.1's like any record's. Measured once each on bash 5.3 only (`…-pr527-r15.md` §Q, 29–42 min per run, two runs at a time): base rc 0, PASSED, `0 mutant(s)`; the draft-19 prototype with the plant, `50 mutant(s)`, `0 neither killed nor argued equivalent`. At T both run on both shells, and the run must also be PASSED (the draft-19 prototype's hand set had the `LC_ALL=C.UTF-8` record killed for the wrong reason; draft 20 relabels it, corpus §6.1). The bash-3.2 runs are estimated at about 1.3 times the 5.3 time, 40–55 min each, from the clean-run ratio (25 s against 19 s on the draft-20 prototype); that is an estimate, not a measurement |
| X4 | C1/C2: X1's log at the parent commit and at the split commit, with scratch paths normalised by one `sed`, then `diff` | empty |
| X4b | the X4b block below, over each file that C1, C2, C6, C8, C9, C10, C11 or C12 edits (the harness is edited by C6, C8, C9, C10, C11 and C12) | empty (at `8413a4db` the harness gives 2 lines, 71 and 93, which C6 deletes) |
| X5 | the corpus §6 R, RLOUD, RES and P cells, same recipe; the draft-10 cells (companion §E.7) on the implementing head as `p11/`; and drafts 11–20's cells (corpus §6.1) | R and RES green with P equal; RLOUD red; P PASS; every AFTER row PASS; each draft-11–20 cell as corpus §6.1 says |
| X6 | two by-hand edits to `…trip-wire.fixtures.sh`, each just before its `: > "$_FW_DIR/built"` line: `mkdir -p "$_FGIT_VOID" && printf 'ref: refs/heads/x\n' > "$_FGIT_VOID/HEAD" && : > "$_FGIT_VOID/config"`, and `git -C "$CTL/clean" config include.path /nonexistent-k2`; one run each | red: the template through **P-c**, the include through **P-g**. Not P-d: P-d compares two inits that both use the void as their template, so a template planted there is on both sides. Not P-a: P-a reads one probe repo, not the fixtures |
| X8 | at T (§9.1), a sanity record only (§9, "X8 is a sanity record only"): `/usr/bin/time -p bash scripts/trip-wires.sh`, three runs on the branch and three on base `e8f78896`, alternated | the six times recorded in `…-pr527-r15.md` §Q; nothing decided and nothing written to `ci.yml`. The budget verdict is X9's |
| X9 | `Layering trip-wires` on ubuntu (GNU) by route (b), decided by the user: #501's CI after #527 is squashed into it (land order step 7); the job's time from that run, by §9's `gh run view` command | SUCCESS, and the job's time under half the budget in force (§9, "The threshold"; at or above it, STOP), the verdict written on #501's branch as §9 says. The run is also GNU evidence for `env -i`, `env -0`, the window, the prelude, and the census's `find` expressions under GNU find: `find "$CTL" \( -iname .git -print0 \) -o \( -iname HEAD -print0 \)` (pass 1's list), `find <.git> \( \( -type f -links +1 \) -o \( ! -type f ! -type d \) \) -print` (pass 1's shape), `find "$CTL" -type l -print0` (pass 2's link list) and `find -L <link> \( -iname HEAD -o -iname .git \) -print` (pass 2's search). If a record's sed expression reads differently under GNU sed, the always-on anchor check in `_mut_correspondence` is the first thing to fail, on the ordinary run |
| X10 | `$SH -n` over every `.claude/tools/webref-generic-core-trip-wire*.sh`, and `wc -l` over the parts this PR edits (controls, fixtures, harness, mutations, mutgen; C11 edits mutgen) | clean; each edited part below 1000 lines. The wire itself (1259 lines at `8413a4db`) is not edited by this PR, so the rule does not reach it |
| X11 | in a clone, add to `…trip-wire.controls.sh`, after the other `_lbl` definitions, `_x_lbl="an unrecorded probe"` and `echo "$_x_lbl" >/dev/null` | red, and the ratchet lists it |
| X12 | withdrawn with draft 12's time bound (draft 13); the number is not reused | — |
| X16 | at T, by hand, with the cell script `cell15.sh` verbatim in `…-pr527.md` (round 13): the four Codex R26③ FIFO cells of the table "R26③, measured" (`commondir`, `HEAD`, `config`, `config.worktree` in `clean/.git`), round 13's cells `xcommon`, `bareh`, `outsidein`, `hard`, `xlink`, `xincreg` and `clean`, round 14's `xrefpoison` and `xrefctl`, round 15's `lnblind2` (its insert verbatim in `…-pr527-r15.md` §Q), round 16's `xbare`, `xanchor` and `xexecarith` (C11, C12; each insert and tree edit verbatim in `…-pr527-r15.md` §Q), round 17's `shimprepend`, `shimafter` and `shimhash` (inserts verbatim in `…-pr527-r15.md` §Q), and round 18's `stalectl` and `staleshim` (below), one run per cell and shell | each red cell: rc 1, P-g, the untrusted-build stop runs no control, and the run ends in under 40 s; `clean` PASSED. The 40 s is five times the slowest red cell measured on the draft-15 prototype (`xincreg`, 8 s on bash 3.2; on draft 16's, also 8 s). The waiting cells (`hdless`, `xinclude`, `xinclnk`, `outroot`, `ttyinc` under a terminal) are not exit criteria: they wait, R9. Round 16's cells expect their own verdicts instead: `xbare` rc 1 with the ratchet's `… labels have no mutation record, against a ratchet of 0.`, `xanchor` rc 1 with `… its anchor is stale`, `xexecarith` rc 2 with W's sentence "the fixtures file left the window's stderr redirected". Round 17's: `shimprepend` rc 1 with P-j's first-entry report (not P-g), the untrusted-build stop running no control; `shimafter` and `shimhash` rc 0, PASSED, the declared class-(c) boundary (design memo §0.3): a red there means the boundary moved, and the memo must say how. Round 18's `stalectl` and `staleshim` run by `cellw.sh` (`cell15.sh` with a caller-`PATH` directory `cpath` put first, verbatim in `…-pr527-r15.md` §Q) on a tree with one never-matching record added, `s/K2NEVERMATCHES/x/` labelled `the fixture build window completed`: `stalectl` (no insert) rc 1 with `… its anchor is stale`; `staleshim` (its insert verbatim in §Q; it writes `cpath/mktemp`, which the parent's `_VFY` then resolves to) rc 0, PASSED, the declared class-(c) boundary (D18-A), with `cpath/mktemp` and the prepared directory removed between runs |
| X13 | the `crc.sh` script, verbatim in `…-pr527.md`, at T: the clean tree and eight cells (six red; env0 and reftable green; `garbagehead` is an untrusted build), with the parent's nounset off (`NOU=1`) and on, on bash 3.2 and 5.3 | the same exit status, NE/CF counts and first four `!!` lines both ways, on both shells |
| X14 | at T, each run by `cell15.sh`'s method (`…-pr527.md`, round 13: its own process group, `perl -e 'setpgrp; alarm 120; exec @ARGV'`, then `kill -9 -<pgid>` of that group only): PX1's caller `PATH` (`~+/bin` first, a logging `git` in the repository's `bin/`); Codex R23's `tools/bin` wrapper with its interpreter beside it; Codex R24's `~/bin` wrapper with a helper | each ends within 120 s, PASSED or red with a named cause; reaching the alarm fails the criterion. 120 s is about five times the slowest clean run measured on the draft-15 prototype (24 s, bash 3.2). For PX1 the expected result is PASSED with the wrapper as the fixture git (§0.1). The outcomes are recorded in `…-pr527-r15.md` §Q |
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
