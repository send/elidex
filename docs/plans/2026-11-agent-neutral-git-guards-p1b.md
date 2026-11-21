# Plan — P1b: the merge wrapper

## §0 Status

**Pair (PM 908)**: this memo is the **P1b** slice of the P1 plan. The pairing rule — what the **P1a memo**,
`docs/plans/2026-11-agent-neutral-git-guards-p1a.md`, holds for both slices, how section numbers and ids work across
the memos — is stated once, in the P1a memo §0 ("Pair").

## §1 Goal and non-goals

The goal and non-goals: P1a memo §1. P1b's files:

| slice | file | what it does |
|---|---|---|
| P1b | `.claude/tools/agent/merge-pr` | `merge-pr [--dry-run] <PR>`: refuses unless run from the `main` worktree's copy, byte-identical to `origin/main`'s; PM go token, a dry Codex verdict on the head with 0 unresolved Codex threads, `mergeStateStatus`, every CI check complete and green, no merge queue; then `gh pr merge --squash --match-head-commit`, then the existing post-merge sync and cleanup scripts. `--dry-run` runs every check and prints the merge command instead of running it |
| P1b | `.claude/tools/agent/test_merge_pr.py` | control + mutant tests for the wrapper |

## §2 Coupled invariants

The axes, the slice tags and the server layer: P1a memo §2. The P1b-owned items follow.

**III — merge decision order and TOCTOU** [P1b]. `merge-pr [--dry-run] <PR>` does, in this order, reading the PR
head `H` exactly once:
1. Usage: optionally `--dry-run`, then exactly one argument, all digits. Else exit 2 (the cleanup script needs a
   number, and a URL or branch argument would need a second resolution of the PR).
2. Invocation site (§4.4): the worktree holding the running file (`git -C "$(dirname "$0")" rev-parse
   --show-toplevel`) must be the worktree that has `refs/heads/main` checked out (`git worktree list
   --porcelain`). Else refuse, naming both. A PR worktree is never the site.
3. Self check: `git fetch origin main`, then the running file must be byte-identical to
   `git show origin/main:.claude/tools/agent/merge-pr`. Else refuse with the remedy
   `git -C <main worktree> merge --ff-only origin/main` (a PR must not be able to weaken its own merge gate;
   decided Q2). A failed fetch or a missing blob refuses too.
4. PM dir: `ELIDEX_PM_DIR` must be set and name an existing directory. Else refuse (decided Q1; who sets it: §4.4).
5. `gh pr view <PR> -R <repo> --json number,state,isDraft,baseRefName,headRefOid,mergeStateStatus,statusCheckRollup`
   → `H` (40 hex). `state` must be `OPEN`.
6. Merge queue: `gh api graphql` for `repository.mergeQueue(branch: <baseRefName>)`; non-null → refuse. Assumption
   (not re-fetched): `main` has no merge queue today; with one, `gh pr merge` would enqueue instead of merging, and
   the pin and step 13 would mean something else.
7. Go token: the file `$ELIDEX_PM_DIR/handoff/codex-lanes/go/<PR>-<H>` must exist, with `H` the full 40-hex sha
   (PLAN §6 decision 6; Claude drivers use it too). A token naming another sha of the same PR is reported by name
   ("go is for `<sha>`, head is `H`"). No prefix matching.
8. **Codex on `H`** (PM 893). The reviewer values — `bot_login`, `trigger`, `assessed_commit_marker`,
   `dry_verdict_match`, `error_verdict_match`, `status_comment_match`, `body_finding_match` — are read from
   `git show origin/main:.claude/skills/external-converge/project.md` § reviewer by the overlay rule of §4.4 (a
   missing or empty key refuses). All three channels are read in full (REST `--paginate --slurp` flattened with
   `add`; GraphQL `reviewThreads` page by page). **Every bot item is admitted** — each formal review and each issue
   comment by `bot_login`, whatever its body; nothing is filtered by the marker, because a findings comment can carry
   none (the "four issue-comment shapes" of `~/.claude/skills/external-review/SKILL.md`: a heading plus
   `/blob/<sha>/…#L` links, no marker). Passes only if all of:
   - a. **0 unresolved Codex review threads**, on any head: threads whose first comment's author is the bot
     (GraphQL login without `[bot]`) and `isResolved == false`. Ground: the dry verdict covers two of the three
     channels — the overlay's `dry_verdict_match` bullet records `#494` R2, `dry_verdict: true` beside three
     unresolved threads carrying real findings.
   - b. `T` = `created_at` of the latest issue comment by an author other than the bot whose body **contains** the
     `trigger` text (literal containment, not equality and not a regex — the global resolver's rule; the bot's own
     boilerplate quotes the trigger, hence the author condition); none → the epoch.
   - c. **Each item's assessed sha, derived as the global resolver derives it**: the `assessed_commit_marker`
     followed by a backticked 7–40-hex sha; else the first `/blob/<sha>/` link in the body; else, for a formal
     review, its `commit_id`. **The body-finding test comes first**, as in the global resolver (its
     `has_body_finding` is computed for every item, and error / status excuse only a comment without one): an item
     whose body matches the body-finding detector of d is never set aside; only an item with no body finding that
     matches `error_verdict_match` or `status_comment_match` is set aside (neither verdict nor finding). The round =
     the items newer than `T` whose sha names `H` (compared on the shorter length, at least 7). **An item newer
     than `T` with no derivable sha is a finding item** (fail-safe, driver decision for round 2): it cannot be
     placed on a head, so it cannot be shown not to be about `H`. The global
     resolver drops such an item from the pass, which is safe for a loop that polls again but would be a fail-open
     for a merge. Remedy: read it, then a new trigger opens a new round.
   - d. The round holds at least one dry verdict (an issue comment matching `dry_verdict_match`) and **no finding
     item**: no item (review body or issue comment) matching the body-finding detector (the generic
     `/blob/<sha>/…#L<n>` pattern ∪ `body_finding_match`), no bot issue comment in the round that is neither dry,
     error nor status, and no item of c's no-sha kind. Dry vs findings is decided across every item, not by the
     latest one (#492).

   The review-summary comment's `Completed` row is **not** an assessment: "a status widget reading completed is not
   a verdict" (`~/.claude/skills/external-review/SKILL.md`, the reviewer-neutral rules; the overlay lists the row
   under `non_verdict_signals`). **This departs from the head guard's lexime #332 branch**, which counts that row
   so a merge after a post-review push (e.g. a merge from `main`) does not read as stale and train the operator
   onto `# merge-stale-ok`. Here every merged head needs its own trigger and dry verdict. Why: the canonical rule
   says the widget carries no verdict — it cannot report findings, and the overlay's `non_verdict_signals` names the
   `Completed` row with its canonical paragraph ("No comment and no review on head means not reviewed", `#525`, the
   `#524` auto-review that ended at `Completed` with no comment); the merge cannot be undone and nothing is behind
   it; and `merge-pr` itself has no override to train onto (D30, D32). That last reason holds for the whole merge
   path **only after the `:36` approval** (P1a memo §7.3): until then a Claude driver can still run a raw `gh pr merge`,
   whose head guard honours `# merge-stale-ok` (IV5, the stated gap). The cost is one `@codex review` per merged
   head, which the overlay's "Never wait on auto-review — always fire one trigger" already makes the procedure.
9. `mergeStateStatus == CLEAN`. Anything else, including values not known today, refuses.
10. CI: every entry of `statusCheckRollup` is complete and green, and there is at least one entry. Both shapes are
    read: a `CheckRun` is green when `status == COMPLETED` and `conclusion` ∈ {`SUCCESS`, `NEUTRAL`, `SKIPPED`}; a
    `StatusContext` is green when `state == SUCCESS`. A pending, failed, cancelled or unrecognised entry, or an
    unknown `__typename`, refuses (decided Q3: CLAUDE.md Workflow "CI 全 pass を目視確認してから squash merge", made
    mechanical). ⚠ Unmeasured assumption: that pending or failed non-required checks also make `mergeStateStatus`
    `UNSTABLE`. Step 10 does not rely on it — §6 feeds `CLEAN` together with a pending check and expects a refusal.
11. `--dry-run`: print the step 12 command (with `H`) and "would merge", exit 0, stop here (decided Q4).
12. `gh pr merge <PR> -R <repo> --squash --match-head-commit <H> --delete-branch=false`. Never `--auto`, `--admin`,
    `-d`. `--match-head-commit` is the server-side pin: if the head moved after step 5, GitHub refuses the merge.
13. `gh pr view <PR> -R <repo> --json state,headRefOid` → `MERGED` and head `H`. Only then step 14. If step 12 exited
    non-zero but step 13 shows `MERGED` at `H` (a lost response after a server-side merge), step 14 runs and a
    warning is printed.
14. Run `~/.claude/hooks/post-merge-sync-main.sh`, then `~/.claude/hooks/post-merge-cleanup-branch.sh`, each once,
    with stdin `{"tool_input":{"command":"gh pr merge <PR>"},"cwd":"<caller's pwd -P>"}`, the caller's cwd as the
    process cwd, and `CLAUDE_PROJECT_DIR` passed through unchanged when the caller has it. Both scripts locate the
    main worktree with `git worktree list` from the process cwd; the cleanup script keeps the caller's own worktree
    using `.cwd`, `CLAUDE_PROJECT_DIR` and `lsof` — so a PR branch held by the live session that ran `merge-pr` is
    left in place. Print each script's `additionalContext`. A missing script prints a note; a script's non-zero exit
    prints a note; neither changes the exit code. The sync fast-forwards the worktree that holds the running
    `merge-pr`, which may rewrite that file: measured, git's fast-forward writes a new inode and a running `bash`
    keeps reading the old one (P1a memo §3.2); independently of that, the script's body is one function called on its last
    line (`main "$@"; exit $?`), so `bash` has parsed all of it before step 14 runs.

Exit codes: 0 merged, or `--dry-run` with every check passed; 1 refused at steps 2–10 (nothing merged); 2 usage;
3 step 12 failed and the PR is not merged.
- III1 [P1b]. TOCTOU between step 5 and step 12 on the head: closed by the pin. Every check (token, Codex, state,
  CI) is about the same `H`, and the server merges only `H`.
- III2 [P1b]. TOCTOU on `mergeStateStatus` between step 9 and step 12 (base moved, a check run flipped): **not**
  pinned by the server. If the base moved and the PR still merges cleanly, the squash includes a base nobody tested
  together with `H` — the same exposure as today's manual merge. Stated limit.
- III3 [P1b]. A new Codex item posted between step 8 and step 12 is about `H` or a later head; a later head is
  caught by the pin. A finding posted on `H` in that window is not seen (the same window as a manual merge).
- III4 [P1b]. **Trust model of the go token**: it guards against accidents and sandboxed agents. A session that can
  write `$ELIDEX_PM_DIR` — an unsandboxed Claude session can — can write its own token. Stated limit (D36).
- III5 [P1b]. **No override flag.** Break-glass is a raw merge run by the user from the commands and record line the
  PM prepares (who acts: P1a memo §7.4), recorded (§7.4); this also covers a broken
  `merge-pr` on `origin/main`, whose fix PR is merged that way.

**IV — coexistence with the Claude hooks** (the axis, IV1, IV2 and IV5: P1a memo §2).
- IV3 [P1b]. `merge-pr` is invisible to the Claude merge hooks: its invocation text (`… merge-pr 655`) does not match
  their trigger `gh[[:space:]]+pr[[:space:]]+merge`, and the `gh pr merge` it runs is a child process, not a tool
  call. So the Claude PreToolUse head / delete-branch guards (lines 36, 42) do not double-decide, and the PostToolUse
  sync / cleanup (lines 136, 142) do not run a second time.
- IV4 [P1b]. The PostToolUse entries carry `"if": "Bash(gh *)"` (`settings.json:137`, `:143`), so they run only for a
  command Claude Code's matcher reads as a `gh` command. If one command line both calls `merge-pr` and holds such a
  `gh pr merge <N>` command, the scripts run a second time after the wrapper's run. Sync is silent on that run
  **only if the first run fast-forwarded** local `main` (it exits when local `main` already equals `origin/main`,
  sync line 35); if the first run refused (a dirty tree), the second prints the same note again. Cleanup is silent
  **only if the first run removed the worktree**: it then finds no worktree holding the branch, `branch -D` fails silently, and it prints
  nothing (cleanup lines 59–62, 116–120); if the first run left the worktree in place (a live session holds it), the
  second run prints the same note again. Harmless either way.

**V — cut-over order** (the axis, V1, V2, V5 and V6: P1a memo §2).
- V3 [P1b]. Raw `gh pr merge` is denied or adapted (Claude side, `:36`) before the PostToolUse merge entries
  (`:136`, `:142`) change; the reverse order leaves raw merges without sync / cleanup. Both are P1a memo §7.3 items.
- V4 [P1b]. The post-merge scripts stay in `~/.claude/hooks/` whatever happens to their PostToolUse entries:
  `merge-pr` calls them.

**F — fail direction per check** (the rule and the P1a rows: P1a memo §2).

| slice | check | an unknown (tool missing, API error, unrecognised value) falls to | why |
|---|---|---|---|
| P1b | merge-pr: every `gh` / `jq` / fetch failure, a wrong invocation site, a running file that differs from `origin/main`'s, a reviewer key missing in the overlay, no dry Codex verdict on `H` after the last trigger, an unresolved Codex thread, a merge queue, a `mergeStateStatus` other than `CLEAN`, a check not complete and green, `ELIDEX_PM_DIR` unset or missing, a token for another sha | **refuse** | the wrapper *is* the merge, which cannot be undone. The current head guard fails open (its header, lines 14–16) because it sits in front of a merge a human still decides; here nothing is behind |
| P1b | merge-pr: post-merge script missing or failing | note, exit 0 | the merge already happened |

### Pair intersections

The pairs without III: P1a memo §2.

| slice | pair | intersection |
|---|---|---|
| — | I × III | `merge-pr` is not a git hook; its analogue of I5 (a PR weakening the copy that merges it) is closed by step 2 (site) and step 3 (`origin/main`'s bytes) |
| P1b | III × IV | IV3: the wrapper's command text is outside the Claude merge hooks' trigger; IV4: the only double run is harmless |
| P1b | III × V | the go token protocol starts when the PM starts writing tokens (§7.2 step B3); until then `merge-pr` refuses every PR |
| — | III × VI | none: `merge-pr` makes no local commit (the squash is server-side) |
| P1b | III × F | every step before step 12 fails closed; after step 12 nothing refuses, because a merge cannot be refused afterwards |
| P1b | III × T | the fake `gh` serves recorded response shapes (the head review on page 2 as in #487; a `Completed` summary row as in lexime #332, which must now refuse; a finding and a dry verdict seconds apart as in #492; a marker-less findings comment as in #506 R88; both rollup shapes, the `StatusContext` one built from the GraphQL reference, §4.6) and acts as the server for `--match-head-commit`: it refuses when its head moved |

## §3. Spec coverage map

The sources, the other rows, the breadth, §3.1 and §3.2: P1a memo §3. The gh and GitHub GraphQL rows:

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| gh §FLAGS (gh pr merge) | `--match-head-commit SHA` | head must match | `merge-pr` step 12 | ✓ | yes (PR number) |
| gh §FLAGS (gh pr merge) | `--squash`, `--delete-branch=false` | no `--auto`, `--admin`, `-d` | `merge-pr` step 12 argv | ✓ | yes |
| gh §FLAGS (gh api) | `--paginate --slurp` | pages as an array of arrays | `merge-pr` step 8 | ✓ | no |
| GitHub GraphQL §OBJECTS (PullRequest) | `mergeStateStatus` | `CLEAN` only | `merge-pr` step 9 | allow-list, so completeness of the enum is not relied on | no |
| GitHub GraphQL §OBJECTS (PullRequest) | `headRefOid`, `state`, `isDraft`, `baseRefName` | `OPEN` before, `MERGED` after | `merge-pr` steps 5, 13 | ✓ | no |
| GitHub GraphQL §OBJECTS (PullRequest) | `statusCheckRollup` | `CheckRun` (`status`, `conclusion`) and `StatusContext` (`state`); green = `COMPLETED` + `SUCCESS` / `NEUTRAL` / `SKIPPED`, or `SUCCESS`; at least one entry | `merge-pr` step 10 | allow-list, so the enums' completeness is not relied on; an unknown `__typename` refuses | no |
| GitHub GraphQL §OBJECTS (PullRequestReviewThread) | `isResolved`, first comment's `author.login` | unresolved bot thread count | `merge-pr` step 8a | ✓ (login without `[bot]` in GraphQL, per the overlay) | no |
| GitHub GraphQL §OBJECTS (Repository) | `mergeQueue(branch:)` | null when the branch has no merge queue | `merge-pr` step 6 | assumption: `main` has none today | no |

## §4 What each file does

### §4.4 merge-pr [P1b]

As §2 III, steps 1–14. The repository for `-R` is `gh repo view --json nameWithOwner` in the caller's cwd, read once.
The script's body is one function, called on the last line (`main "$@"; exit $?`), so a rewrite of the file during
step 14's sync cannot change what runs (§2 III step 14).

**The overlay rule (step 8), stated once.** The § reviewer section of `origin/main`'s
`.claude/skills/external-converge/project.md` is the lines from `## reviewer` to the next `## ` heading. A key's
bullet is a line starting `` - `<key>`: ``; its value is the **first backtick span after that colon** on the same
line (a key with no such bullet, two bullets, or an empty span refuses). How each value is used — the global
resolver's use, written once in the script:

| key | used as |
|---|---|
| `bot_login` | literal; REST compares it whole, GraphQL compares it with a trailing `[bot]` removed |
| `trigger` | literal containment in an issue comment's body (step 8b) |
| `assessed_commit_marker` | regex prefix, followed by `` [^`]*`([0-9a-f]{7,40}) `` (step 8c) |
| `dry_verdict_match`, `error_verdict_match`, `status_comment_match` | regex searched in the body (jq `test`); error / status set an item aside only when the body-finding test (next row) does not match it (step 8c) |
| `body_finding_match` | regex, OR-ed with the always-on generic `/blob/[0-9a-f]{7,40}/[^)[:space:]]*#L[0-9]+` |

**Where the values are canonical.** `external-review/project.md` § reviewer also writes `bot_login`, `trigger`,
`assessed_commit_marker` and `dry_verdict_match`; it defers to `external-converge/project.md` for the trigger
policy, for how to read the dry verdict, and for the values of the other keys. `merge-pr` reads
`external-converge/project.md` only — the file that declares itself canonical — and `test_merge_pr.py` asserts that
the four values written in both files are equal, so the second copy cannot drift silently.

**Codex drivers' invocation form.** The Codex execpolicy matches an argv prefix (`prefix_rule(pattern=[…])` in
`~/.codex/rules/default.rules`), which a `$(…)` in the command cannot satisfy. So a Codex driver runs the literal
form `bash /Users/kazuaki/repos/send.sh/elidex/.claude/tools/agent/merge-pr <PR>` with `ELIDEX_PM_DIR` from its
launch environment, and §7.2 B1's allow rule names exactly `["bash",
"/Users/kazuaki/repos/send.sh/elidex/.claude/tools/agent/merge-pr"]`. If the `main` worktree ever moves, step 2
refuses the old path (it is no longer the `main` worktree) and the rule is updated with the move.

**Invocation site and how it is found** (step 2). The running copy is the one in the worktree that has `main`
checked out — today `/Users/kazuaki/repos/send.sh/elidex` — run from the caller's own cwd:

```sh
ELIDEX_PM_DIR=<pm dir> bash "$(git worktree list --porcelain | awk '/^worktree /{w=substr($0,10)} /^branch refs\/heads\/main$/{print w; exit}')/.claude/tools/agent/merge-pr" <PR>
```

Not a `mise` task: `mise run` resolves the task in the caller's checkout, which for a driver is its PR worktree —
the site step 2 refuses. The caller's cwd stays the driver's own worktree on purpose: step 14 hands it to the
cleanup script, which keeps the worktree of the live session (item D15 of PM 893).
**Who keeps the site current**: every merge through `merge-pr` ends with `post-merge-sync-main.sh` (step 14), which
fast-forwards that worktree to `origin/main`; if it is behind anyway (a merge made elsewhere, a dirty tree the sync
refused), step 3 refuses with the `merge --ff-only` remedy. Step 3 is the reconciliation: the site check says
*which* copy runs, the self check says that copy is `origin/main`'s.
**Who sets `ELIDEX_PM_DIR`**: the PM, in the kickoff brief of every driver, Claude and Codex. A Claude Bash call does
not keep exports, so Claude drivers write it on the invocation line as above; Codex drivers have it in the launch
environment (or on the line). The E2E cases set it on their own lines (§7.5: `$S/pm` for M1–M4, the real PM dir for
M6; M7 runs under the PM's kickoff).

Because of step 3, a PR that changes `merge-pr` is merged with `origin/main`'s copy; P1b itself, before which
`origin/main` has no copy, is merged by hand — the last raw merge outside break-glass. The Codex-head logic is
written once, here; the copy in the global `gh-pr-merge-head-guard.sh` stays until the P1a memo §7.3 `:36` approval
(IV5).

### §4.6 Tests

The P1a files, `isolated_git.py`, the mutant harness and the `timeout-minutes` rule: P1a memo §4.6.

- `test_merge_pr.py` [P1b]: a fake `gh` on `PATH` serving fixture responses by argv and recording every argv; a fake
  `HOME` with stub post-merge scripts that record their stdin, cwd and `CLAUDE_PROJECT_DIR`; a temp PM dir for
  tokens; a scratch repository whose `origin` is a local bare repository holding the script and the overlay file on
  `main`, with `main` checked out in its main worktree, so steps 2–3 run locally. The scratch `origin`'s overlay is
  the **tracked** `.claude/skills/external-converge/project.md` of the test's own checkout, committed into the scratch
  `main` at test time (not a copy kept in the test), and one test runs the §4.4 overlay rule on that tracked file
  and asserts every key yields a non-empty value of its kind (a literal, or a regex that compiles), and that the
  four keys written in both overlays are equal (§4.4) — so an edit that breaks the format is red in CI. Fixture
  sources: the recorded Codex shapes named in the III × T row; the `StatusContext` entry is built from the GitHub
  GraphQL reference (`StatusContext`: `__typename`, `context`, `state`), because no rollup recorded in this
  repository holds one — the PR names the reference page and its fetch date in the fixture file.

## §5 Decisions and their ground

The rung: P1a memo §5. The P1b rows:

| # | slice | decision | rung |
|---|---|---|---|
| D13 | P1b | merge-pr order of §2 III; `H` read once | drive skill §6 steps 2–4 (`merge at head <sha>`, re-check head and `mergeStateStatus`, `--match-head-commit`); PM scope |
| D14 | P1b | no Codex assessment → refuse (the current hook allows) | principle: unknown falls to refuse where nothing is behind; incident #367 |
| D15 | P1b | every `gh` / `jq` error → refuse | as D14 |
| D16 | P1b | `mergeStateStatus` allow-list `CLEAN` | as D14 (an enum value added later refuses) |
| D17 | P1b | `--delete-branch=false` written out | incident lexime #354 (`gh-pr-merge-delete-branch-worktree-guard.sh` header) |
| D18 | P1b | post-merge: the existing scripts, once, with the JSON, the caller's cwd and its `CLAUDE_PROJECT_DIR` | PM scope; PM 893 (D15); the scripts' own interface (sync lines 18–28, cleanup lines 32–47, 70–79) |
| D19 | P1b | the Codex-head logic lives once, in `merge-pr`; the global copy goes with the P1a memo §7.3 `:36` approval | One issue, one way |
| D22 | P1b | `ELIDEX_PM_DIR` required; unset or missing → refuse (Q1); set by the PM's kickoff | principle: the merge is irreversible, so an unknown falls to refuse; PM 893 (D14) |
| D23 | P1b | `merge-pr` refuses unless byte-identical to `origin/main`'s copy, after a fetch (Q2) | Security by structure: a PR must not be able to weaken its own merge gate |
| D24 | P1b | every CI check complete and green, both rollup shapes, besides `CLEAN` (Q3) | CLAUDE.md Workflow "CI 全 pass を目視確認してから squash merge", made mechanical; PM 893 (C7) |
| D25 | P1b | `--dry-run` runs every check and prints the merge command (Q4) | the PM's E2E needs the good path on a real PR without an irreversible act |
| D30 | P1b | Codex: dry verdict on `H` after the last trigger, no finding in any channel, 0 unresolved Codex threads; the `Completed` row is not an assessment; every bot item admitted, its sha derived as the global resolver (marker → first `/blob/<sha>/` → `commit_id`), a round item with no sha is a finding; the trigger by literal containment from a non-bot author | PM 893 (C7); canonical rule in the global external-review SKILL; #492 (latest item does not win), #494 R2 (threads), #525 (`Completed`); round 2 A5-1 (driver decision: no-sha item = finding) |
| D31 | P1b | reviewer values read from `origin/main`'s `external-converge/project.md` § reviewer, not copied into the script | One issue, one way (the overlay is their canonical site) |
| D32 | P1b | no override flag; break-glass = a raw merge and its post-merge sync / cleanup, run by the user from the commands and record line the PM prepares (who acts: P1a memo §7.4); recorded (§7.4) | PM 893 (C8); round 2 A3 / A2 MIN; PM 916 (the PM is a Claude session, and `gh-pr-merge-head-guard` applies to it too) |
| D34 | P1b | invocation site = the `main` worktree's copy, run from the caller's cwd | PM 893 (D13); step 2 refuses a PR worktree |
| D35 | P1b | Dependabot PRs get a `@codex review` from the CI/tools lane, which runs them (`pm:223`, PM 240), then `merge-pr` | PM 893 (E29); no Dependabot exemption, no break-glass for them; round 2 A5 MIN (the lane, not the PM) |
| D36 | P1b | the go token's trust model is accidents + sandboxed agents | PM 893 (E28); stated limit III4 |
| D37 | P1b | a merge queue on the base refuses | PM 893 (E24): main has none today |
| D42 | P1b | the go token path stays `$ELIDEX_PM_DIR/handoff/codex-lanes/go/<PR>-<H>` | PLAN §6 decision 6 fixes `$PMD/handoff/codex-lanes/go/<PR>-<sha>` (ratified, PM 873); round 2 Axis 1 MIN answered by it |

## §6 Control / mutant table

The row shape: P1a memo §6. The P1b rows:

| slice | decision | control (passes) | bad input (rejected) | mutant the test kills |
|---|---|---|---|---|
| P1b | step 1 usage | — | `merge-pr abc`, `merge-pr` → exit 2, fake `gh` saw no `pr merge` | accept any argument |
| P1b | step 2 site | the copy in the scratch repository's `main` worktree | the same bytes run from a linked worktree's checkout → exit 1, names both | drop the site check |
| P1b | step 3 self check | running copy equals `origin/main`'s blob | running copy differs by one byte; `origin/main` has no copy; the fetch fails | drop the comparison; compare against the working-tree file instead of `origin/main` |
| P1b | step 4 PM dir | `ELIDEX_PM_DIR` set to an existing dir | unset; set to a missing dir | default to a path; test only "set" |
| P1b | step 6 merge queue | `mergeQueue` null | `mergeQueue` non-null → exit 1 | skip the query |
| P1b | step 7 token | token `<PR>-<H>` present | none; token for an older sha; token named with a 7-char sha | glob `<PR>-*`; prefix compare |
| P1b | step 8 Codex | dry verdict comment on `H` after the last trigger, 0 unresolved Codex threads; the same with the head review only on page 2 (#487 shape); the trigger inside a longer human comment ("fixed X. @codex review") counts | only a `Completed` summary row on `H` (lexime #332 shape); dry verdict + one unresolved Codex thread; dry verdict older than the last trigger; a finding comment and a dry verdict seconds apart (#492); a marker-less findings comment (heading + `/blob/<H>/…#L` link) beside a dry verdict (#506 R88 shape); a findings comment on `H` (heading + `/blob/<H>/…#L12` link) that quotes `Codex Review: Something went wrong`, beside a dry verdict; a bot comment newer than the trigger with no marker and no blob link that is neither error nor status; a bot comment quoting the trigger is not a trigger (the dry verdict before the human trigger stays stale); verdict on an older sha; overlay key missing; `gh api` exits non-zero | accept the summary row; ignore threads; latest item wins (the #492 case passes); filter comments by the marker (the #506 case passes); set error / status items aside before the body-finding test (the quoting findings comment is set aside and the merge proceeds); drop items with no sha (the no-sha case passes); trigger by equality (the longer-comment control fails); trigger from any author (the bot's quote resets `T`); drop the trigger cut-off; drop `--paginate` (fake serves page 1 only without it); empty assessment → allow |
| P1b | §4.4 overlay rule | the tracked `external-converge/project.md` yields every key, each value of its kind; the four keys shared with `external-review/project.md` are equal | a fixture overlay with a key's bullet missing, doubled, or with an empty span → exit 1, names the key | take the line's first backtick span (it is the key's own name); accept the last of two bullets |
| P1b | step 9 state | `CLEAN` | `UNSTABLE`, `BLOCKED`, `DIRTY`, `BEHIND`, `DRAFT`, `UNKNOWN`, `HAS_HOOKS`, `FOO` | deny-list `DIRTY\|BLOCKED` |
| P1b | step 10 CI | `CheckRun` entries `COMPLETED` + `SUCCESS` / `NEUTRAL` / `SKIPPED`; a `StatusContext` `SUCCESS` | `CLEAN` with a pending `CheckRun`; a `FAILURE`; a `StatusContext` `PENDING`; empty rollup; unrecognised conclusion; unknown `__typename` | check only failed entries (pending passes); read only `conclusion` (the `StatusContext` cases pass); allow an empty rollup |
| P1b | step 11 dry run | `--dry-run` with every check passing prints the step 12 argv, exit 0, fake `gh` saw no `pr merge`, no post-merge run | `--dry-run` with a failing check → exit 1 | `--dry-run` handled before the checks (prints "would merge" with a failing check) |
| P1b | step 12 argv | merge argv exactly `pr merge <PR> -R <repo> --squash --match-head-commit <H> --delete-branch=false` | — | drop `--match-head-commit` |
| P1b | III1 TOCTOU | — | fake moves the head between view and merge → fake refuses the pinned merge → exit 3, no post-merge run | drop `--match-head-commit` (fake then merges) |
| P1b | step 13 lost response | merge exits non-zero, then view says `MERGED` at `H` → post-merge runs, warning | merge exits non-zero, view says `OPEN` → exit 3 | run post-merge whenever step 12 ran |
| P1b | step 14 post-merge | sync then cleanup, once each; recorded stdin's command matches `gh[[:space:]]+pr[[:space:]]+merge[[:space:]]+[0-9]+` (cleanup's own trigger), `.cwd` is the caller's `pwd -P`, `CLAUDE_PROJECT_DIR` arrives unchanged | — | command text `gh pr merge` without the number; scripts run in `$HOME` instead of the caller's cwd; `env -u CLAUDE_PROJECT_DIR` |
| P1b | step 14 missing | scripts absent → exit 0 with a note | — | exit non-zero when a script is absent |
| P1b | IV3 | `merge-pr 655` (the documented invocation) does not match the PostToolUse trigger | — | (pin on the usage string; a rename that adds `gh pr merge` to it fails) |

## §7 Cut-over (PM / user, after each merge) and the E2E cases

Where each E2E case runs (M1–M7 included) and the unsafe orders: P1a memo §7 and §7.5.

### §7.2 P1b cut-over

| step | action | state after it | safe because |
|---|---|---|---|
| B0 | P1b merges (by hand, through the Claude hooks as today — the last raw merge outside break-glass) | `merge-pr` on `origin/main` | nothing calls it yet |
| B1 | prerequisite, P2-owned (PM / user): the Codex execpolicy allow list (PM 877 / 878, `~/.codex/rules`) must not include `gh pr merge`; it allows exactly the argv prefix `["bash", "/Users/kazuaki/repos/send.sh/elidex/.claude/tools/agent/merge-pr"]`, the literal form of §4.4 (a `prefix_rule` cannot match the `$(…)` form) | Codex drivers cannot raw-merge without a prompt | the wrapper is the only merge path Codex is allowed |
| B2 | E2E M1–M6 (§7.5) | unchanged | M1–M6 stop before step 12 |
| B3 | **only after B1**: the PM starts writing go tokens and puts `ELIDEX_PM_DIR` in every kickoff (§4.4); drivers merge with `merge-pr` (M7) | wrapper path live; raw `gh pr merge` still possible for Claude drivers (IV5, the stated gap) | the raw path keeps its Claude guards; a Codex driver has no raw path (B1) |

**For the PM, the token's writer**: the file name is `<PR>-<H>` with `H` the **full 40-hex** head sha
(`gh pr view <PR> --json headRefOid -q .headRefOid`), under `$ELIDEX_PM_DIR/handoff/codex-lanes/go/`. A 7-character
or any other prefix is refused (step 7). The file's content is not read.

### §7.4 Break-glass

What both kinds share (who acts, and the unverified `! <command>` premise) and the push / commit kind: P1a memo
§7.4.

**Merge (P1b, D32).** There is no override in `merge-pr`. When a merge must happen that `merge-pr` refuses —
including a broken `merge-pr` on `origin/main`, whose fix PR is merged this way — the user runs it (who acts: P1a
memo §7.4). The PM prepares the commands, all run from one cwd in this order:
`gh pr merge <N> -R send/elidex --squash --match-head-commit <H> --delete-branch=false`, then step 14's two scripts
with its stdin,
`printf '{"tool_input":{"command":"gh pr merge <N>"},"cwd":"%s"}' "$(pwd -P)" | ~/.claude/hooks/post-merge-sync-main.sh`,
then the same into `post-merge-cleanup-branch.sh`; and the record line. **The post-merge sync and cleanup are in
those commands**, because the merge runs outside the Claude Bash tool and no PostToolUse entry (`:136`, `:142`)
runs them; if one ever did, the second run is harmless (IV4).

### §7.5 E2E cases

The setup and E1–E24: P1a memo §7.5.

`$S/pm` is an empty directory made for M1–M4 (`mkdir -p $S/pm/handoff/codex-lanes/go`) and removed with `$S`.

| # | slice | command (the §4.4 invocation from a worktree of the shared repository, against a real open PR, unless stated) | expected |
|---|---|---|---|
| M1 | P1b | `merge-pr <open PR>` with no token (`ELIDEX_PM_DIR=$S/pm`) | exit 1, "no go token"; PR still open |
| M2 | P1b | token for an older sha of that PR (in `$S/pm`) | exit 1, names both shas |
| M3 | P1b | token for head (in `$S/pm`), no dry Codex verdict on head after the last trigger | exit 1, names the Codex condition |
| M4 | P1b | token for head (in `$S/pm`), `mergeStateStatus` not `CLEAN`, or a check pending | exit 1, names the state or the check |
| M5 | P1b | a copy of `merge-pr` saved under `$S` and run from there | exit 1, invocation site |
| M6 | P1b | `merge-pr --dry-run <PR>` on a PR the PM has approved (`ELIDEX_PM_DIR` = the real PM dir, token written) | exit 0, prints the `gh pr merge … --match-head-commit <H> --delete-branch=false` command; the PR is still open |
| M7 | P1b | the first real merge the PM approves after B3, run by the driver from its own PR worktree (cwd = the worktree its live session holds) | merged; sync and cleanup notes printed once; the PR's worktree is **left in place** and the cleanup note says so; no PostToolUse notes in the Claude transcript for that command |

M1–M6 stop before step 12 of §2 III, so they are safe on a real open PR. The head-moves race (III1) is not
reproducible by hand; it is covered by the test with the fake server.

## §8 Decided (formerly open questions)

Q0, Q5 and the round records: P1a memo §8.

- **Q1** `ELIDEX_PM_DIR` is required; unset or a missing directory refuses (§2 III step 4). Ground: the merge is
  irreversible, so an unknown falls to refuse.
- **Q2** `merge-pr` refuses unless the running file is byte-identical to `origin/main`'s copy, after a fetch (step
  3). Ground: Security by structure — a PR must not be able to weaken its own merge gate.
- **Q3** Every CI check must be complete and green, besides `mergeStateStatus == CLEAN` (step 10). Ground: CLAUDE.md
  Workflow "CI 全 pass を目視確認してから squash merge", made mechanical. Unmeasured assumption kept: that pending
  or failed non-required checks give `UNSTABLE`; step 10 does not rely on it and §6 tests `CLEAN` + pending.
- **Q4** `--dry-run` runs every check and prints the merge command without running it (step 11); the PM's E2E uses
  it on a real PR (§7.5 M6). Ground: the good path needs a check that is not an irreversible act.

## §9 Layering and what is not touched

Layering, what is not touched, and the global → tracked dependency: P1a memo §9. The cross-boundary
dependency P1b owns:

- **Tracked → global.** `merge-pr` (tracked) runs `~/.claude/hooks/post-merge-sync-main.sh` and
  `post-merge-cleanup-branch.sh` (global) with a hand-built PostToolUse payload (step 14: `.tool_input.command`,
  `.cwd`, the process cwd, `CLAUDE_PROJECT_DIR`). The contract is that interface (sync lines 18–28, cleanup lines
  32–47 and 70–79); its owner is the owner of `~/.claude/hooks` (PM / user, through the dotfiles session if those
  files are dotfiles-managed), who keeps it or changes `merge-pr` with it. `test_merge_pr.py` pins the payload
  `merge-pr` sends; a missing or failing script is a note, never a refusal (F).
