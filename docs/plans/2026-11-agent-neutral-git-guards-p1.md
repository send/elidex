# Plan — P1: agent-neutral git guards and the merge wrapper

## §0 Status

**Program**: the Codex-lane program. Its driver memo lives **outside this repository**:
`/Users/kazuaki/repos/send.sh/elidex-pm-trial/handoff/codex-lanes/PLAN.md` (PM 872). It is the ratified program
memo (its line 3: ratified by the user, PM 873; it points at §10 of this memo for its errata), and PM 874 ordered
this per-PR memo. This memo is the plan for its phase **P1** (PLAN §5 P1, rows 1, 2, 3, 5, 6, 7, 8 and 17 of
PLAN §1).
**Scope source**: PM 873 / 874, restructured by PM 893; plan-review round 2 added PM 900 (the bypass-guard extension
of §4.8, approved by the user) and the PM's A2-scope decision. The scope is not re-opened here; §8 records the
decisions.
**Slices (PM 893, overturning PM 874's "single PR")**: two PRs under this one memo, **P1a first**.
- **P1a — git hooks**: `pre-push`, `pre-commit`, the shim, the installer (`install-githooks`, `--check`), the
  ratchet's `--rev` mode, their tests, and the one-line `--check` standing item in the PM skill (D33). Cut-over §7.1,
  which also applies the global bypass-guard patch this memo carries (§4.8; the PM applies it, it is not in the PR).
- **P1b — merge wrapper**: `merge-pr` and its tests. Cut-over §7.2.
- **The seam**: this memo's own II × III intersection is empty — `merge-pr` pushes nothing, so no pre-push
  decision meets a merge decision (§2 pair table). The two slices share only the test-isolation helper (T4), which
  P1a adds and P1b imports. §2 stays one section; each invariant is tagged **[P1a]**, **[P1b]** or **[both]**.
**Nature**: repo tooling. New tracked files under `.claude/tools/githooks/` and `.claude/tools/agent/`, their
tests, one `mise` task; one modified tool (`.claude/tools/ecs_mission_ratchet.py` gains `--rev`, with its test); one
line in `.claude/skills/pm/SKILL.md`. Zero `crates/**` diff. Nothing under `~/.claude` is edited by the PRs (§4.8 is
a patch the PM applies at cut-over), and nothing is installed into any `.git/hooks`:
installing the shim and changing the matching Claude hooks is the **cut-over**, done by the PM / user after each
merge (§7).
**Edge-dense**: yes — six intersecting axes (§2). So `/elidex-plan-review` is mandatory before implementation
(CLAUDE.md Design discipline, "Edge-dense work"). P1a and P1b are each a terminal per-PR slice under the ratified
program memo (decided Q0 in §8); each gets its own full review.

## §1 Goal and non-goals

**Goal.** The guards that today exist only as Claude Code PreToolUse / PostToolUse hooks, and that decide on the
*text* of a Bash command, move to a layer every agent and every human goes through: git's own hooks (which see what
git is about to do) and one merge wrapper (which is the merge). A Codex driver then gets the same guard as a Claude
driver, from one implementation (CLAUDE.md "One issue, one way").

Files (all (NEW) unless marked):

| slice | file | what it does |
|---|---|---|
| P1a | `.claude/tools/githooks/pre-push` | per pushed ref: reject non-fast-forward (force or delete), reject a target branch that is not the current branch (detached HEAD exempt), reject any push to `refs/heads/main`; then run the ECS-mission ratchet on each pushed commit (`--rev`) |
| P1a | `.claude/tools/githooks/pre-commit` | rustfmt check of the staged `.rs` blobs, with the staged `.rustfmt.toml`; per-invocation opt-out naming the staged tree |
| P1a | `.claude/tools/githooks/shim.sh` | the body the installer writes into the hooks directory, once per hook name; runs **`origin/main`'s** copy of the hook script; passes with a note while `origin/main` has none (absence is an exit-0 empty answer; an error rejects, I3) |
| P1a | `.claude/tools/githooks/install.sh` | the installer; always run as **`origin/main`'s** copy, read into a variable and run with `bash -c` (§4.7) — by the `mise` task and by the §7.1 pass-through alike; `--check` reports any worktree the shims do not reach |
| P1a | `.claude/tools/githooks/isolated_git.py` | test helper (not a member): the isolated environment of T4 and its refusal on the real repository; imported by every test below |
| P1a | `.claude/tools/githooks/test_githooks.py` | control + mutant tests for both hooks, the shim and the installer (split by scenario if it nears 1000 lines) |
| P1a | `.claude/tools/ecs_mission_ratchet.py` (MODIFIED) | `--rev <sha>`: materialise `<sha>`'s tree in a temp dir and run **`<sha>`'s own ratchet module** on it; the running copy only orchestrates (§4.5) |
| P1a | `.claude/tools/test_ecs_mission_ratchet.py` (MODIFIED) | control + mutant for `--rev` (695 lines today; the addition keeps it under 1000) |
| P1a | `mise.toml` `[tasks.install-githooks]` (MODIFIED) | runs `origin/main`'s `install.sh` with `bash -c` after a read that must succeed (§4.7); not part of `[tasks.ci]` |
| P1a | `.claude/skills/pm/SKILL.md` (MODIFIED) | one line in §4: `mise run install-githooks -- --check` in every PM pass from shim install on (D33) — in P1a, not P3, so the standing item is in the PM procedure before anything is installed |
| P1b | `.claude/tools/agent/merge-pr` | `merge-pr [--dry-run] <PR>`: refuses unless run from the `main` worktree's copy, byte-identical to `origin/main`'s; PM go token, a dry Codex verdict on the head with 0 unresolved Codex threads, `mergeStateStatus`, every CI check complete and green, no merge queue; then `gh pr merge --squash --match-head-commit`, then the existing post-merge sync and cleanup scripts. `--dry-run` runs every check and prints the merge command instead of running it |
| P1b | `.claude/tools/agent/test_merge_pr.py` | control + mutant tests for the wrapper |

**Non-goals.**
- Installing the shim into the shared repository, or changing any Claude hook (cut-over, §7, PM / user after merge).
- The Codex PreToolUse / Stop adapters (PLAN rows 4, 9, 11 = P2).
- **The repo docs that name the old procedure — P3, one list (this is the only copy; §7 points here):**
  1. `.claude/skills/drive/codex.md` (new, PLAN §4).
  2. `CLAUDE.md` Workflow "Git" line (`CLAUDE.md:54` at 6cbe39e3): "CI 全 pass を目視確認してから squash merge
     (… この目視が最終ゲート)" — the check becomes `merge-pr` step 10; the line names `merge-pr` instead.
  3. `CLAUDE.md` Workflow worktree line (`CLAUDE.md:55`): names `~/.claude/hooks/git-push-branch-guard.sh` as what
     blocks a branch-mismatch push — becomes pre-push.
  4. `.github/workflows/ci.yml` `trip-wires` job comment (`ci.yml:191-194`) and `tools` job comment
     (`ci.yml:275-278`): "merging still rests on the CLAUDE.md convention of eyeballing CI" — becomes `merge-pr`.
  5. `.claude/skills/drive/SKILL.md` §6 step 4 (`drive:127`): raw `gh pr merge … --match-head-commit` — becomes the
     `merge-pr` invocation of §4.4 (with `ELIDEX_PM_DIR`).
  6. `.claude/skills/pm/SKILL.md` §4: the go token (file name with the full 40-hex sha, §7.2), PM-merged PRs
     (`pm:224`) through `merge-pr`, and Dependabot PRs (`pm:223`: the CI/tools lane runs them, PM 240) through
     `merge-pr` after the CI/tools lane posts `@codex review` (D35). The `--check` standing item is P1a's line, not
     P3's.
  7. `.claude/skills/pre-push/SKILL.md`: the one-line reference for Codex drivers (PLAN §5 P3).
- Stopping `git commit --amend` itself. Amend is harmless until the amended commit is pushed over its old self,
  and that push is a non-fast-forward which pre-push rejects (PLAN row 1).
- Tags. Tags are not part of the PR / merge flow; pre-push applies the fast-forward rule to them (II7) and no
  more, so a tag moved *forward* along history passes (stated limit, decided Q5 in §8).
- Defeating a deliberate bypass. A git hook cannot stop `--no-verify` (git-push(1) OPTIONS, git-commit(1); §3); CI
  (`check` for fmt, `tools` for the ratchet) is behind it, and for Claude sessions the existing
  `git-hook-bypass-guard.py`.

## §2 Coupled invariants

Six axes plus two cross-cutting ones. Each invariant has an id and a slice tag; the pair table below gives each
pair's intersection in one line.

**The server layer, stated once** (referenced from I4 and F): the `main-protection` ruleset covers `main` only
(deletion, non-fast-forward, pull request); its admin bypass was removed on 11-21 (PM). It has no
required-status-check rule. Nothing on the server stops a force push to, or a mismatched push into, a feature
branch.

**I — one hooks directory serves every linked worktree and every branch.**
- I1 [P1a]. The shared repository has `core.hooksPath` set in its local config to its own common hooks dir
  (`git config --show-origin --get core.hooksPath` → `file:…/elidex/.git/config  …/elidex/.git/hooks`). One shim
  file per hook name there runs for every linked worktree, every session (Claude, Codex, human) and every branch.
- I2 [P1a]. The shim runs **`origin/main`'s** copy of the hook script (the blob
  `.claude/tools/githooks/<name>` at the commit `refs/remotes/origin/main` names, read with `git cat-file blob` and
  passed to `bash -c`; §4.1), never the working-tree file. So an
  uncommitted edit of the script does nothing, and a branch cut before P1a is guarded like any other (PM 893).
  git sets the hook's cwd to the root of the worktree it runs in (§3, measured from a linked worktree); the
  scripts find their repository with `git rev-parse` from there and locate nothing relative to `$0`.
  `refs/remotes/origin/main` is the local remote-tracking ref: a hook change takes effect in a repository at its
  first fetch after the merge. Tests run the in-tree script directly (T5).
- I3 [P1a]. While `origin/main` has no copy of the script (before P1a's merge, or in a clone without
  `refs/remotes/origin/main`), the shim passes with one stderr line saying the hook is not on `origin/main` yet.
  Absence is read only from an answer that succeeded and was empty: `git for-each-ref` printing no line for the
  ref, or `git ls-tree <commit> -- <path>` printing nothing, both with exit 0 (measured, §3.2). A non-zero exit of
  either (an I/O error, a corrupt ref) rejects, so an error is never read as absence. It ends at P1a's merge plus a
  fetch; V1 keeps the shim uninstalled until then. Moving the ref by hand (`git update-ref refs/remotes/origin/main
  <a commit without the scripts>`) or deleting it is a deliberate bypass, like `--no-verify` (§1 non-goals).
- I4 [P1a]. One worktree can route around the shim: `extensions.worktreeConfig` is `true` in the shared repository,
  and `git config --worktree core.hooksPath <dir>` makes `git rev-parse --git-path hooks` answer `<dir>` for that
  worktree only (measured in a scratch clone). A git hook cannot close this: once the hooks path points elsewhere,
  the shim never runs. Which layer closes it (driver decision):
  - **Prevention, for direct commands = the agent adapter layer.** Claude: the global `git-hook-bypass-guard.py`
    gains the patch of §4.8 (PM 900, approved by the user; scope = the whole write grammar of the key, PM's A2-scope
    decision). Read on 2026-11-21: it already denies `-c core.hooksPath=…` / `-c core.hooksPath …` /
    `-ccore.hooksPath=…`, `--config-env=core.hooksPath=…`, a `GIT_CONFIG_KEY_<n>=core.hooksPath` assignment and
    `--no-verify` (its docstring lines 4–14); it has no rule for `git config … core.hooksPath` nor for
    `GIT_CONFIG_PARAMETERS`, both of which reroute (measured, §3.2). Codex: PLAN row 4's adapter (P2). Both are
    global changes applied by the PM / user, not by this PR — cut-over prerequisites (§7.1 step A2). A command
    adapter sees commands only: indirect forms (`include.path` pointing at a file that sets the key, a direct file
    write into `config.worktree`) are not prevented.
  - **The Codex sandbox caveat.** PLAN §6 item 5 (as revised by PM 877) gives the Codex driver's workspace-write
    sandbox the `.git` common dir. A session that can write there can rewrite a shim in `<common dir>/hooks` or a
    worktree's `config.worktree` by a plain file write, which no command adapter sees. Such a write is detected,
    not prevented: `installed(wt)` compares the shim bytes and the resolved hooks path (§4.1), so `--check` lists
    the worktree.
  - **Detection = this PR.** The predicate `installed(wt)` of §4.1 (the one definition); `install.sh --check` lists
    every worktree for which it is false and exits non-zero if there is one. It sees every persistent reroute,
    direct or indirect, because `git rev-parse --git-path hooks` follows includes (measured, §3.2); it does not see a
    one-command reroute (`-c …`, `GIT_CONFIG_PARAMETERS`), which leaves nothing behind. The PM runs
    `mise run install-githooks -- --check` in **every PM pass, as a standing item** (D33): a person can run git
    by hand after P2 too.
  - **Residual accepted gap: a person running git by hand** (no agent adapter sees it). Behind it: the server layer
    (above) and CI (`check` for fmt, `tools` for the ratchet). Neither covers **branch-mismatch pushes** or **force
    pushes to a feature branch**: for those the git hook is the only layer, and a by-hand reroute of the hooks path
    leaves them open until the next `--check` reports the worktree.
- I5 [P1a]. A branch cannot weaken the hook that judges its own pushes (I2: `origin/main`'s copy runs). A PR that
  changes a hook script takes effect only after it merges, under review. **Stated limit — the ratchet's capture set
  is the branch's own**: II9 runs `<sha>`'s ratchet module with `<sha>`'s classifier, allowlist and `SELF_FILES`
  (§4.5), so a branch that adds an allowlist row or narrows the classifier passes its own push. That is the same
  limit as the CI `tools` job, which also runs the branch's ratchet; the ratchet's docstring names these edits as
  invisible to it and hands them to review (`elidex-review` axes.md Axis 2 sub-check 2d).

**II — push decisions × push shapes** [P1a]. pre-push stdin is one line per ref,
`<local-ref> SP <local-sha> SP <remote-ref> SP <remote-sha>` (§3). Decisions, per line:
- II1 [P1a]. `remote-ref == refs/heads/main` → reject, whatever the shape (new, FF, delete, from detached HEAD).
- II2 [P1a]. HEAD attached on branch `B`, `remote-ref` under `refs/heads/` and `!= refs/heads/B` → reject. Keyed on
  the **remote** ref, for every local-ref shape: the local ref is "supplied as it was originally given" (`HEAD` for
  `HEAD:x`, measured; an object name or `HEAD~` likewise), so a rule keyed on the local ref being `HEAD` — the
  current hook's scope — misses `<sha>:refs/heads/x`, `HEAD~0:refs/heads/x` and `git push origin <other-branch>`
  (git itself refuses `<sha>:x` and `HEAD~0:x` before the hook, because an unqualified destination cannot be
  guessed from an object name). Detached HEAD → exempt
  (the current hook's rule: an isolated worktree mid cherry-pick / revert). Backstop for the exemption: none
  mechanical — II5 still applies, so a detached push can only fast-forward a branch, never overwrite it; a
  fast-forward into another session's branch shows up as foreign commits in that PR's diff (F).
- II3 [P1a]. `remote-sha` all zeros (any length, SHA-1 or SHA-256) → a new ref → allowed by the fast-forward rule.
- II4 [P1a]. `local-sha` all zeros (`local-ref` is `(delete)`) → reject. A delete discards the remote's commits, the
  same harm as a force push; and `git merge-base --is-ancestor <remote> 000…0` exits 128 (measured), so the FF rule
  already rejects it. Remote branch deletion belongs to the repository's `delete_branch_on_merge` setting.
- II5 [P1a]. Otherwise `git merge-base --is-ancestor <remote-sha> <local-sha>`: exit 0 → allow; exit 1 → reject
  (non-fast-forward); any other exit → reject (measured: 128 when the remote commit is not in the local object
  store, which means the remote moved and the push would overwrite commits we do not have).
- II6 [P1a]. Several lines in one push: every line is checked, every reason printed, and one rejected line rejects
  the whole push (a non-zero exit aborts the push "without pushing anything", §3; measured).
- II7 [P1a]. Tags: II1 and II2 do not apply (they are about `refs/heads/`); II3–II5 apply. Annotated tag objects are
  peeled by `merge-base --is-ancestor` (measured exit 0 between two tag objects on one line of history). A tag
  moved *forward* along history passes II5 (stated limit, §1 non-goals, decided Q5).
- II8 [P1a]. Empty stdin (an up-to-date push, measured: the hook runs with no lines) → allow and do not run the
  ratchet.
- II9 [P1a]. **The ratchet reads the pushed tree** (PM 893). After every line has passed, once per distinct
  `local-sha` among the lines that create or update a ref, the hook reads `origin/main`'s
  `.claude/tools/ecs_mission_ratchet.py` blob into a variable (a failed read rejects, F) and only then runs it as
  `python3 - --rev <local-sha>` with that text on stdin — a pipe straight from `git show` would hand `python3` an
  empty program on a failed read, and an empty program exits 0. **Every verdict input comes from `<local-sha>`** (§4.5 is the one statement): the running copy (`origin/main`'s)
  only orchestrates. So the verdict is the one `<local-sha>`'s ratchet gives in a checkout of `<local-sha>`, and an
  uncommitted edit cannot change it. A `<local-sha>` without the ratchet is told apart by history: cut before the
  ratchet existed → pass with a note; a descendant of the commit that added it (the branch deleted it) → reject
  (§4.5, F). The cost is one ratchet run per distinct pushed commit, plus materialising its tree (§3.2: 8.6 s for one
  working-tree run; `--rev` unmeasured — the PR measures it).
- II10 [P1a]. `--no-verify` skips the hook entirely (git-push(1) OPTIONS, §3). Not stoppable from inside git.

**VI — commit decisions** [P1a]. pre-commit, per `git commit`:
- VI1 [P1a]. **Index blob, not working-tree file.** The checked content is `git show ":<path>"` read through the
  inherited `GIT_INDEX_FILE`. For `git commit <path>` and `git commit -a` git commits from a temporary index
  (measured: `GIT_INDEX_FILE` = `next-index-*.lock` / `index.lock`; githooks(5) does not say so), and only the
  inherited variable reads the index git is about to commit. For a plain `git commit` the inherited value is the
  relative `.git/index` (measured by plan-review round 2), so the hook makes it absolute against its starting cwd
  before anything else and never changes cwd before reading the index.
- VI2 [P1a]. **ACMR.** `git diff --cached --name-only -z --diff-filter=ACMR -- '*.rs'`: R added to the current
  hook's ACM — a renamed file with changed content is a staged `.rs` too. Deleted files are not checked.
- VI3 [P1a]. **`ELIDEX_FMT_OK` is per invocation** (decided, item E22 of PM 893): the hook honours it only when its
  value equals the staged tree (`git write-tree` through the inherited index). Any other value — an exported `1`,
  a tree from an earlier staging — is refused with a note naming both trees, and the check runs. So an opt-out
  covers exactly the commit it was written for. **Stated limit**: for `git commit -a` and `git commit <path>` git
  builds the temporary index only after the command starts, so no value can be written for it beforehand; to skip
  such a commit, stage first (`git add …`) and commit from the real index with `ELIDEX_FMT_OK=$(git write-tree)`.
- VI4 [P1a]. **Config and edition from the index.** rustfmt is given `--config-path` = a temp dir holding the
  staged `.rustfmt.toml` blob (`git show :.rustfmt.toml`), so the edition that formats the commit is the commit's
  (measured: `--config-path <dir>` on stdin finds `.rustfmt.toml` there; without it, `async fn` fails to parse).
  The hook writes no edition — the edition has one home. No staged `.rustfmt.toml` → reject (the commit removes the
  edition's home; F). `.rustfmt.toml` and the workspace `Cargo.toml` both say `edition = "2021"` today; if they
  ever differ, CI `check` (`cargo fmt --all -- --check`) is the authority and the hook is a pre-filter.
- VI5 [P1a]. **Merge commits.** An automatic merge runs pre-merge-commit, not pre-commit, and is not checked
  (this PR installs no pre-merge-commit shim). A conflicted merge concluded with `git commit` runs pre-commit
  (githooks(5) pre-merge-commit, last paragraph), so its staged blobs are checked. Backstop for automatic merges:
  CI `check` (`cargo fmt --all -- --check`).
- VI6 [P1a]. `--no-verify` / `-n` skips the hook (git-commit(1)); not stoppable from inside git.

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
     review, its `commit_id`. Items matching `error_verdict_match` or `status_comment_match` are set aside first
     (neither verdict nor finding). The round = the items newer than `T` whose sha names `H` (compared on the shorter
     length, at least 7). **An item newer than `T` with no derivable sha is a finding item** (fail-safe, driver
     decision for round 2): it cannot be placed on a head, so it cannot be shown not to be about `H`. The global
     resolver drops such an item from the pass, which is safe for a loop that polls again but would be a fail-open
     for a merge. Remedy: read it, then a new trigger opens a new round.
   - d. The round holds at least one dry verdict (an issue comment matching `dry_verdict_match`) and **no finding
     item**: no review body matching the body-finding detector (the generic `/blob/<sha>/…#L<n>` pattern ∪
     `body_finding_match`), no bot issue comment in the round that is neither dry, error nor status, and no item of
     c's no-sha kind. Dry vs findings is decided across every item, not by the latest one (#492).

   The review-summary comment's `Completed` row is **not** an assessment: "a status widget reading completed is not
   a verdict" (`~/.claude/skills/external-review/SKILL.md`, the reviewer-neutral rules; the overlay lists the row
   under `non_verdict_signals`). **This departs from the head guard's lexime #332 branch**, which counts that row
   so a merge after a post-review push (e.g. a merge from `main`) does not read as stale and train the operator
   onto `# merge-stale-ok`. Here every merged head needs its own trigger and dry verdict. Why: the canonical rule
   says the widget carries no verdict — it cannot report findings, and the overlay's `non_verdict_signals` names the
   `Completed` row with its canonical paragraph ("No comment and no review on head means not reviewed", `#525`, the
   `#524` auto-review that ended at `Completed` with no comment); the merge cannot be undone and nothing is behind
   it; and `merge-pr` itself has no override to train onto (D30, D32). That last reason holds for the whole merge
   path **only after the `:36` approval** (§7.3): until then a Claude driver can still run a raw `gh pr merge`,
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
    keeps reading the old one (§3.2); independently of that, the script's body is one function called on its last
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
- III5 [P1b]. **No override flag.** Break-glass is a raw merge by the PM, recorded (§7.4); this also covers a broken
  `merge-pr` on `origin/main`, whose fix PR is merged that way.

**IV — coexistence with the Claude hooks** [both] (shim installed; the Claude hooks not yet changed, or changed
only for elidex).
- IV1 [P1a]. On a Claude `git push` both the Claude PreToolUse hooks (`settings.json` lines 13 and 24) and pre-push
  decide; the push proceeds only if all of them allow. Where they differ: the Claude branch guard fires only on a
  `HEAD:<branch>` refspec in the command text (`git-push-branch-guard.sh` lines 22–24), pre-push on every ref
  (wider); the Claude force guard denies the *flag* text (`-f`, `--force`, `--force-with-lease`, line 13) even for a
  fast-forward, and does not match a `+<refspec>` force, while pre-push decides on the ref shape. After §7.1
  step A4, line 24 passes through in an elidex worktree where `installed(wt)` holds, and only pre-push decides
  there; line 13 is unchanged (§7.3).
- IV2 [P1a]. On a Claude `git commit` the Claude fmt hook (line 30) checks the working-tree file of each path the
  *real* index lists; pre-commit checks the blobs of the index git commits from (VI1). They differ on partial
  staging and on `git commit <path>` / `git commit -a`. Until step A4 a deliberate skip needs both opt-outs
  (`# fmt-ok` and VI3's value); after it, in an installed elidex worktree, only VI3's.
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
- IV5 [both]. **Two coexistences the approved scope cannot end** (PM 893 (1)); each has an owner, a trigger and an
  end. This is the strangler state "One issue, one way" forbids, so the memo names them instead of hiding them.
  - **Merge (the stated gap).** Until the `:36` approval (§7.3) a Claude driver can still run a raw
    `gh pr merge`. Its only guard is `gh-pr-merge-head-guard.sh`, which allows a PR with no Codex assessment and
    fails open on any `gh` error (its header, lines 14–16), counts the summary row's `Completed` (its lexime #332
    branch), honours `# merge-stale-ok` (header lines 18–19), and asks for no go token. So on that path step 8's
    "no override to train onto" and V × F's "a fail-closed layer on merge" do **not** hold; they hold for the whole
    merge path only after `:36`. Owner: the PM, who carries `:36` to the user. Trigger: P1b's merge (from then on
    `merge-pr` exists and the raw path is the second procedure). Not pulled into this scope.
  - **Force push.** Until the `:13` approval (§7.3) the Claude force guard denies the flag text (`-f`, `--force`,
    `--force-with-lease`), also on a fast-forward, and does not see a `+<refspec>`; pre-push II5 decides on the ref
    shape. Both decide, so the stricter answer wins (IV1). Owner: the PM. Trigger: §7.1 step A4 (from then on `:13`
    is the one Claude push guard that still decides on text in an installed worktree). End: the `:13` approval,
    under which its force half passes through like `:24` and the amend half stays.

**V — cut-over order** [both] (§7 has the steps and what is safe in each intermediate state).
- V1 [P1a]. Nothing is installed before P1a is merged (an installed shim would pass everywhere: I3).
- V2 [P1a]. The shim goes in before any Claude hook stops deciding. For `:24` / `:30` this holds by construction:
  their pass-through defers only where `installed(wt)` is true (§4.1, §7.1 step A4).
- V3 [P1b]. Raw `gh pr merge` is denied or adapted (Claude side, `:36`) before the PostToolUse merge entries
  (`:136`, `:142`) change; the reverse order leaves raw merges without sync / cleanup. Both are §7.3 items.
- V4 [P1b]. The post-merge scripts stay in `~/.claude/hooks/` whatever happens to their PostToolUse entries:
  `merge-pr` calls them.
- V5 [P1a]. The adapter-layer prevention of I4 (the §4.8 patch: prevented for direct commands, detected by
  `--check`) is in place before `:24` / `:30` pass through; the `--check` PM-pass item runs from shim install on.
  Without both, the pass-through makes I4 an unwatched hole for agents.
- V6 [P1a]. `:30` passes through only after pre-commit is installed — by construction, as V2.

**F — fail direction per check** [both] (rule: decide by where an unknown falls, not by how complete a list is).

| slice | check | an unknown (tool missing, API error, unrecognised value) falls to | why |
|---|---|---|---|
| P1a | II1 main, II2 branch, II3–II5 FF / delete | **reject** | no server backstop off `main` (§2 server layer); a mismatched push puts another session's commits in a PR |
| P1a | II2 detached-HEAD exemption | pass (by design) | no mechanical backstop; II5 bounds it to fast-forwards, and foreign commits show in the target PR's diff |
| P1a | II9 ratchet: interpreter cannot start (`python3 -c ''` fails) | **pass with a warning** | the CI `tools` job is the authority; in a checkout mise does not trust, the `python3` shim exits 1 (measured), the same code as a finding, so the hook runs `python3 -c ''` first to tell them apart |
| P1a | II9 ratchet exit 1: findings by `<sha>`'s own rules | reject | a finding on the pushed tree |
| P1a | II9 ratchet exit 2: usage | reject | pre-push and the ratchet both come from `origin/main`, so a usage error is an error between them, not a state |
| P1a | II9 ratchet exit 3: an error while judging — materialising the tree, loading `<sha>`'s module, a missing name of the §4.5 interface, any exception inside `<sha>`'s code (a traceback), or a return value other than 0 / 1 | reject, with the reason's last line | the orchestrator catches every exception, so a traceback never reads as a finding (exit 1) or as ok; the cause is the pushed tree or the machine, and an unjudged push is not a clean one |
| P1a | II9 ratchet: `origin/main` has the hook but not the ratchet, or `git show` of it fails | reject | cannot happen by checkout once the hook is on `origin/main` (both land in P1a), only by an error |
| P1a | II9 ratchet: no ratchet at `<sha>`, and `<sha>` does not contain the commit that added it | pass with a note (exit 0) | decided by history: no rule existed at that commit |
| P1a | II9 ratchet: no ratchet at `<sha>`, and `<sha>` contains that commit | reject (exit 1) | the branch deleted the ratchet; passing would let a branch switch off the rule that judges it. A PR that really removes or renames it goes through break-glass (§7.4) |
| P1a | II9 ratchet: ancestry of `<sha>` unknown (`merge-base --is-ancestor` exit 128) | reject (exit 3) | an error, not a state |
| P1a | VI pre-commit: a staged blob rustfmt reformats or cannot parse | reject | measured |
| P1a | VI4 no staged `.rustfmt.toml` | reject | the commit removes the edition's home; formatting with a default edition would decide on the wrong grammar |
| P1a | VI pre-commit: `rustfmt --version` fails | **pass with a warning** | the CI `check` job's `cargo fmt --all -- --check` is the authority; in an untrusted checkout the `rustfmt` shim exits 1 (measured). Rejecting there would train operators onto the opt-out — the failure mode recorded in `gh-pr-merge-head-guard.sh` for #487 |
| P1a | VI3 `ELIDEX_FMT_OK` not equal to the staged tree | check runs (note) | an opt-out is for one commit only |
| P1a | VI pre-commit: `git rev-parse --show-toplevel` fails | reject | `git commit` always has a work tree, so a failure is an error, not a state |
| P1a | shim: `for-each-ref` prints no line for `refs/remotes/origin/main`, or `ls-tree` prints no entry for the hook (both exit 0) | pass with a note | I3: an absence git reported, not an error; ends at P1a's merge plus a fetch. Deleting or moving the ref on purpose is a deliberate bypass (§1 non-goals) |
| P1a | shim: `for-each-ref`, `ls-tree` or `cat-file` exits non-zero | reject | an error, not a state (I3) |
| P1a | shim: `--show-toplevel` | not called | the shim reads `origin/main`'s blob, which needs no work tree; pre-push needs none either (refs and `--rev` only), so a bare repository is judged too |
| P1a | installer: the invoking worktree's hooks path is not `<common dir>/hooks`, or a non-shim hook is already there | refuse | writing elsewhere would act on other repositories (a global `core.hooksPath`) or on one worktree only, or replace someone's hook |
| P1a | `--check`: a worktree whose hooks path cannot be resolved, or `origin/main`'s `shim.sh` cannot be read | listed, exit 1 | `installed(wt)` is false when it cannot be shown true |
| P1b | merge-pr: every `gh` / `jq` / fetch failure, a wrong invocation site, a running file that differs from `origin/main`'s, a reviewer key missing in the overlay, no dry Codex verdict on `H` after the last trigger, an unresolved Codex thread, a merge queue, a `mergeStateStatus` other than `CLEAN`, a check not complete and green, `ELIDEX_PM_DIR` unset or missing, a token for another sha | **refuse** | the wrapper *is* the merge, which cannot be undone. The current head guard fails open (its header, lines 14–16) because it sits in front of a merge a human still decides; here nothing is behind |
| P1b | merge-pr: post-merge script missing or failing | note, exit 0 | the merge already happened |

**T — the test harness** [both] (CI `tools` job, `scripts/python-suites.sh`).
- T1 [both]. Members are tracked `test_*.py`, found by glob; every test must run and pass (a skip is red).
- T2 [both]. The offline regime drops every `GIT_*` variable and blocks sockets except `AF_UNIX` — for Python
  processes. Its stated limit (`scripts/python-suites.sh` header, "THE STATED LIMIT"): a child that "is not
  Python" is outside the guard. Every `git`, `bash` and `gh` these tests start is such a child; T4 closes that
  class for these suites by construction.
- T3 [P1a]. The job declares Python only. A check whose result depends on another runtime's version belongs to a
  job that declares that runtime (the `tools` job comment). So the pre-commit tests use a **stub rustfmt**, built to
  the real CLI shape measured in §3.2 (stdin in, formatted text out; exit 1 on a parse error; `--check` on stdin
  prints a diff and exits 0; `--config-path <dir>` reads the edition from `<dir>/.rustfmt.toml`). The real rustfmt
  is exercised by the PM's E2E (§7.5).
- T4 [both]. **Isolation by construction** (`isolated_git.py`, PM 893). Every test process runs with `HOME` = a temp
  dir, `GIT_CONFIG_GLOBAL=/dev/null`, `GIT_CONFIG_NOSYSTEM=1`, `GIT_ALLOW_PROTOCOL=file` (git refuses every
  non-file transport: measured `fatal: transport 'ssh' not allowed`, exit 128; local-path and `file://` pushes
  work), and a `PATH` whose `gh`, `ssh` and `rustfmt` are stubs ahead of everything else. Remotes are local bare
  repositories only. The helper **refuses to run** (the test fails, not skips) if any repository it is handed has
  a common dir equal to the real repository's (`git rev-parse --path-format=absolute --git-common-dir` of the test
  file's own checkout, computed with the `GIT_*` variables removed), or a common dir outside the test's temp dir.
- T5 [P1a]. Tests run the in-tree hook scripts directly (the shim's `origin/main` lookup is itself tested by giving
  the scratch repository an `origin/main` that holds a chosen script).

### Pair intersections

| slice | pair | intersection |
|---|---|---|
| P1a | I × II | every push from every worktree and branch is judged by `origin/main`'s ref rules (I2), the same for every branch, old ones included; the ratchet's capture set is the pushed commit's own (I5, stated limit) |
| — | I × III | `merge-pr` is not a git hook; its analogue of I5 (a PR weakening the copy that merges it) is closed by step 2 (site) and step 3 (`origin/main`'s bytes) |
| both | I × IV | in a worktree with I4's override, `installed(wt)` is false, so the Claude hooks keep deciding there (no pass-through); in other repositories nothing changes |
| P1a | I × V | the installer writes one file per hook name into the shared hooks dir, so installation is all-worktrees-at-once; there is no per-worktree rollout |
| P1a | I × VI | pre-commit, like pre-push, is `origin/main`'s copy; the staged config it reads (VI4) is the branch's, so the rule is shared and the edition is the commit's |
| P1a | I × F | I3 is the one fail-open on the reject-side checks; it is taken only on an absence git reported with exit 0, never on an error |
| both | I × T | tests build their own repositories and bare remotes inside a temp dir; T4 refuses the real repository |
| — | II × III | **empty — the slice seam.** `merge-pr` pushes nothing, so pre-push never sees a merge; the only push-shaped event in a merge is server-side |
| P1a | II × IV | double decisions are conjunctive, so until step A4 a push needs both layers to allow; IV1 lists where they disagree |
| P1a | II × V | `:24` passes through only where the shim is installed, so II2 is in force wherever the Claude branch guard stops deciding; `:13` (force) is not changed (§7.3) |
| P1a | II × VI | the ratchet at push reads the pushed commit (II9) and pre-commit reads the index (VI1): neither reads the working tree, so what was committed is what is judged at push |
| P1a | II × F | every ref rule fails closed; only the ratchet's environment error fails open, and it runs only after every ref rule has passed |
| P1a | II × T | each II case is a real `git push` into a local bare repository, so the stdin the hook reads is git's own, not a fixture |
| P1b | III × IV | IV3: the wrapper's command text is outside the Claude merge hooks' trigger; IV4: the only double run is harmless |
| P1b | III × V | the go token protocol starts when the PM starts writing tokens (§7.2 step B3); until then `merge-pr` refuses every PR |
| — | III × VI | none: `merge-pr` makes no local commit (the squash is server-side) |
| P1b | III × F | every step before step 12 fails closed; after step 12 nothing refuses, because a merge cannot be refused afterwards |
| P1b | III × T | the fake `gh` serves recorded response shapes (the head review on page 2 as in #487; a `Completed` summary row as in lexime #332, which must now refuse; a finding and a dry verdict seconds apart as in #492; a marker-less findings comment as in #506 R88; both rollup shapes, the `StatusContext` one built from the GraphQL reference, §4.6) and acts as the server for `--match-head-commit`: it refuses when its head moved |
| both | IV × V | IV is the state between §7.1 step A1 and step A4 for push / commit, and between §7.2 step B3 and the §7.3 approvals for merge |
| both | IV × VI | IV2: the two commit checks differ on partial staging and temporary indexes until step A4 |
| both | IV × F | in IV the stricter answer wins, so a fail-open on one layer is covered by the other where both look |
| both | IV × T | the tests cannot run the Claude hooks (they live outside the repository); IV is checked by the PM's E2E only |
| P1a | V × VI | V6: `:30` defers only where pre-commit is installed |
| both | V × F | each intermediate state of §7 keeps at least one fail-closed layer on push and on commit; on merge only after the `:36` approval — before it, the raw `gh pr merge` path has the fail-open head guard only (IV5, the stated gap) |
| both | V × T | the PRs' tests pass with nothing installed; installation is checked by the E2E, never by CI |
| P1a | VI × F | the only fail-open in VI is rustfmt not running, behind CI `check`; a missing config fails closed |
| P1a | VI × T | each VI case is a real `git commit` (with a pathspec, `-a`, partial staging) in a scratch repository, with the stub rustfmt |
| both | F × T | each fail direction is a test case: the stub rustfmt and a fake `python3` that cannot start cover the warn-and-pass paths, fake `gh` errors cover refusal |

## §3. Spec coverage map

There is no web spec here. The table lists the git, gh, rustfmt and GitHub behaviours the scripts rely on, so the
preflight's per-row check still means something: each row names its source document and section, and a
measurement where the documents say nothing (or say otherwise). Sources: `git help githooks`, `git help push`,
`git help commit`, `git help merge-base`, `git help rev-parse`, `git help git`, `git help config`, `git help
read-tree`, `git help checkout-index`, `git help ls-tree`, `git help for-each-ref` (git 2.56.1), `gh pr merge --help` /
`gh api --help` (gh 2.105.0), `rustfmt --help` (rustfmt 1.10.0), GitHub GraphQL reference (not re-fetched for this
memo).

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| git §DESCRIPTION (githooks(5)) | hooks directory | `$GIT_DIR/hooks`, or `core.hooksPath`; non-executable hooks are ignored | `install.sh` target; the shim must be executable; the hook scripts are run with `bash -c` and need no exec bit | ✓ + measured: per-worktree override (I4) | yes (any worktree's config) |
| git §DESCRIPTION (githooks(5)) | working directory and environment | cwd = root of the working tree; `GIT_DIR` etc. exported | hook scripts' `git rev-parse`; ratchet strips repo-locating vars itself (`git_env`) | ✓ (measured from a linked worktree: cwd = that worktree, `GIT_DIR` = its gitdir) | no |
| git §HOOKS (githooks(5)) | pre-push stdin | new ref: remote sha all zeros | pre-push II3 | ✓ | yes |
| git §HOOKS (githooks(5)) | pre-push stdin | delete: local ref `(delete)`, local sha all zeros | pre-push II4 | ✓ (measured line) | yes |
| git §HOOKS (githooks(5)) | pre-push stdin | local ref "supplied as it was originally given" | pre-push II2 keys on the remote ref | ✓ (measured: `HEAD`) | yes |
| git §HOOKS (githooks(5)) | pre-push exit | non-zero aborts the whole push | pre-push II6 | ✓ (measured: nothing reached the remote) | yes |
| git §HOOKS (githooks(5)) | pre-push stdin | up-to-date push: no lines | pre-push II8 | doc silent; measured | no |
| git §OPTIONS (git-push(1)) | `--verify`, `--no-verify` | toggles the pre-push hook | stated limit II10 | ✓ | yes |
| git §OPTIONS (git-commit(1)) | `-n`, `--no-verify` | bypasses pre-commit | stated limit VI6 | ✓ | yes |
| git §HOOKS (githooks(5)) | pre-commit | no parameters; non-zero aborts | pre-commit exit | ✓ | yes |
| git §ENVIRONMENT (measured, not githooks(5)) | pre-commit index | `GIT_INDEX_FILE` names the index being committed; a temporary index for `commit <path>` / `commit -a` | pre-commit VI1 reads `git diff --cached` / `git show :<path>` through it | measured only: githooks(5) does not describe the temporary index | yes |
| git §HOOKS (githooks(5)) | pre-merge-commit | an automatic merge runs pre-merge-commit; a conflicted merge concluded with `git commit` runs pre-commit (last paragraph) | VI5: automatic merges unchecked (backstop CI `check`); conflict resolutions checked | ✓ | yes |
| git §OPERATION MODES (git-merge-base(1)) | `--is-ancestor` | exit 0 / 1 / other | pre-push II4, II5 | measured: 128 for an unknown or all-zero sha | yes |
| git §OPTIONS (git-rev-parse(1)) | `--git-path hooks`, `--git-common-dir`, `--path-format=absolute` | hooks path honours `core.hooksPath`, including a worktree-level value | `installed(wt)` (§4.1), installer | measured: key unset → `<common dir>/hooks` from the main and a linked worktree; `--worktree` value → that value | no |
| git §ENVIRONMENT VARIABLES (git(1)) | `GIT_ALLOW_PROTOCOL` | colon-separated allow-list of transports | T4 (`file` only) | measured: `ssh` refused with exit 128; local path and `file://` allowed | no |
| git §OPTIONS (git-read-tree(1), git-checkout-index(1)); §OUTPUT FORMAT (git-ls-tree(1)) | `read-tree <sha>` into a temp `GIT_INDEX_FILE`, `checkout-index -a --prefix=<dir>/`; `ls-tree -r -z --name-only <sha>` | materialise `<sha>`'s tree; list its tracked paths | ratchet `--rev` (§4.5) | measured: the real index is untouched, symlinks are written as symlinks, and the path list equals `ls-files` of a checkout of `<sha>` | no |
| git §FIELD NAMES (git-for-each-ref(1)); §OUTPUT FORMAT (git-ls-tree(1)) | `for-each-ref --format=…`, `ls-tree <commit> -- <path>` | an absent ref or path prints nothing | shim absence vs error (I3) | measured: absent ref / path → no output, exit 0; `ls-tree` on an absent ref → exit 128 | no |
| git §COMMANDS, §DEPRECATED MODES, §OPTIONS (git-config(1)) | writes of `core.hooksPath` | `set` / `unset` / `rename-section` / `remove-section` / `edit`, and the legacy `<name> <value>`, `--add`, `--replace-all`, `--unset[-all]`, `--rename-section`, `--remove-section`, `-e`; `--local` / `--worktree` / `--global` / `--system` / `-f` / `--file`; variable names case-insensitive (§CONFIGURATION FILE) | §4.8 patch (global, PM) | measured: every form reroutes `--git-path hooks` or puts it back, options also after the positionals | yes |
| git §ENVIRONMENT (git-config(1)) | `GIT_CONFIG_COUNT` / `GIT_CONFIG_KEY_<n>`; `GIT_CONFIG_PARAMETERS` | per-process configuration | §4.8 patch (`GIT_CONFIG_PARAMETERS` new; `GIT_CONFIG_KEY_<n>` already denied) | `GIT_CONFIG_PARAMETERS` is in neither git(1) nor git-config(1): measured, `'core.hooksPath'='<d>'` and `'core.hookspath=<d>'` reroute one command | yes |
| git §CONFIGURATION FILE (git-config(1), "Includes") | `include.path` | an included file's `core.hooksPath` applies | I4 detection (`--check`), §4.8 blind spot | measured: a `--worktree include.path` and a `-c include.path=` both reroute; `--git-path hooks` follows the include | yes |
| rustfmt §OPTIONS (`rustfmt --help`, 1.10.0) | `--check` | the help says it exits 1 when formatting is required | **not used on stdin**: measured exit 0 with a diff on stdin (§3.2) — the help says the opposite of the stdin behaviour | measured | yes |
| rustfmt §OPTIONS (`rustfmt --help`, 1.10.0) | `--config-path` | "Recursively searches the given path for the rustfmt.toml config file" | VI4: a temp dir holding the staged `.rustfmt.toml` | measured: finds `.rustfmt.toml` in the dir on stdin input | yes |
| gh §FLAGS (gh pr merge) | `--match-head-commit SHA` | head must match | `merge-pr` step 12 | ✓ | yes (PR number) |
| gh §FLAGS (gh pr merge) | `--squash`, `--delete-branch=false` | no `--auto`, `--admin`, `-d` | `merge-pr` step 12 argv | ✓ | yes |
| gh §FLAGS (gh api) | `--paginate --slurp` | pages as an array of arrays | `merge-pr` step 8 | ✓ | no |
| GitHub GraphQL §OBJECTS (PullRequest) | `mergeStateStatus` | `CLEAN` only | `merge-pr` step 9 | allow-list, so completeness of the enum is not relied on | no |
| GitHub GraphQL §OBJECTS (PullRequest) | `headRefOid`, `state`, `isDraft`, `baseRefName` | `OPEN` before, `MERGED` after | `merge-pr` steps 5, 13 | ✓ | no |
| GitHub GraphQL §OBJECTS (PullRequest) | `statusCheckRollup` | `CheckRun` (`status`, `conclusion`) and `StatusContext` (`state`); green = `COMPLETED` + `SUCCESS` / `NEUTRAL` / `SKIPPED`, or `SUCCESS`; at least one entry | `merge-pr` step 10 | allow-list, so the enums' completeness is not relied on; an unknown `__typename` refuses | no |
| GitHub GraphQL §OBJECTS (PullRequestReviewThread) | `isResolved`, first comment's `author.login` | unresolved bot thread count | `merge-pr` step 8a | ✓ (login without `[bot]` in GraphQL, per the overlay) | no |
| GitHub GraphQL §OBJECTS (Repository) | `mergeQueue(branch:)` | null when the branch has no merge queue | `merge-pr` step 6 | assumption: `main` has none today | no |

**Breadth**: K=4 (git, gh, rustfmt, GitHub GraphQL), M=30. The preflight's SPLIT-DEFAULT (K≥6 or M≥30; reached by
M=30 since round 2's rows) is answered by the two slices and the separate global patch — no single PR carries more
than 19 rows: P1a holds 17 git and 2 rustfmt rows (K=2, M=19), the §4.8
patch (applied by the PM, in neither PR's diff) the 3 `git-config(1)` rows (K=1, M=3), P1b the 3 gh and 5 GitHub
GraphQL rows (K=2, M=8).

### §3.1 User-input touch audit

No web-content input. The inputs are what an agent or a person types: refspecs and flags of `git push`, the index
of `git commit`, the PR number given to `merge-pr`, and files under the PM dir. Each one reaches a decision only
through git's own report (pre-push stdin, the index) or a `gh` response, never by parsing the command text — that is
the point of the move (IV1, IV2).

### §3.2 Measured behaviours the documents do not state

All measured on 2026-11-21 in scratch repositories (deleted afterwards), git 2.56.1, rustfmt 1.10.0 through
`~/.cargo/bin/rustfmt`:
- `rustfmt --edition 2021 --check < bad.rs` prints a diff and **exits 0**; `rustfmt --check bad.rs` exits 1.
  `rustfmt --help` says `--check` "Exits with 1 and prints a diff if formatting is required" — true for a file,
  the opposite for stdin. So pre-commit does not use `--check`: it pipes the staged blob into rustfmt and compares
  the output with the blob (equal = clean; rustfmt non-zero = cannot parse = reject).
- `printf 'async fn f() {}\n' | rustfmt --config-path <dir>` with `<dir>/.rustfmt.toml` = `edition = "2021"`
  exits 0; the same with no config exits 1 (edition 2015 cannot parse `async fn`).
- In a checkout mise does not trust, the mise shims for `rustfmt` and `python3` print a trust error and exit 1.
- One local ratchet run on `origin/main` 3cbd5b88 took 8.6 s wall clock (`/usr/bin/time -p`, this machine, one
  run, working-tree mode).
- `GIT_ALLOW_PROTOCOL=file`: `git push ssh://…` → `fatal: transport 'ssh' not allowed`, exit 128; pushes to a local
  path and to `file://` succeed.
- Fresh repository with `core.hooksPath` unset: `git rev-parse --path-format=absolute --git-path hooks` answers
  `<common dir>/hooks` from the main and from a linked worktree; with the key set to that path, the same.
- `echo '…' | python3 - <args>`: `__file__` is `'<stdin>'` (so `--rev` must not depend on `__file__`); under
  `python3 -c` it is undefined.

Added in plan-review round 2 (2026-11-21, scratch repositories under the session scratchpad, deleted afterwards,
git 2.56.1, `HOME` / `GIT_CONFIG_GLOBAL=/dev/null` / `GIT_CONFIG_NOSYSTEM=1` isolated):
- Each of these moved `git rev-parse --path-format=absolute --git-path hooks` of a linked worktree away from
  `<common dir>/hooks` (with `extensions.worktreeConfig=true`): `git config core.hooksPath <d>`; the same with
  `--worktree`, `--local`; `git config core.hooksPath <d> --worktree` (option after the positionals);
  `core.HOOKSPATH`; `--add`; `--replace-all`; `config set --worktree`, `config set --worktree --all`; `--file` /
  `-f` / `--file=` naming the worktree's `config.worktree`; `--type=path`; `--comment x`; `--worktree --`. Reads
  (`--worktree --get`, `config get`) did not. With the key set locally, each of `--unset`, `--unset-all`,
  `config unset`, `config unset --all`, `--remove-section core`, `config remove-section core`,
  `--rename-section core zz`, `config rename-section core zz` put the hooks path back — they write the key too.
- `GIT_CONFIG_PARAMETERS="'core.hooksPath'='/alt'"` and `"'core.hookspath=/alt'"` reroute that one command; so does
  `git -c include.path=<file>` with `<file>` setting the key, and `git config --worktree include.path <file>`
  reroutes persistently (and `--git-path hooks` shows it).
- `git for-each-ref --format='%(objectname)' refs/remotes/origin/main` with no such ref: no output, exit 0.
  `git ls-tree HEAD -- no/such`: no output, exit 0. `git ls-tree <absent ref> -- x`: exit 128.
  `git cat-file -e HEAD:no/such`: exit 128 (so `-e` cannot tell an absent path from an error).
- `git merge --ff-only` that changes a tracked script gives the file a new inode (390470020 → 390470029); a `bash`
  that was running the old file and ran that merge kept executing the old text.
- `GIT_INDEX_FILE=<tmp> git read-tree HEAD` then `GIT_INDEX_FILE=<tmp> git checkout-index -a --prefix=<dir>/`
  wrote the tree under `<dir>` (a relative and an absolute symlink as symlinks), left the real index unchanged
  (`git diff --cached` empty), and `git ls-tree -r -z --name-only HEAD` listed the same paths as `git ls-files -z`.

## §4 What each file does

### §4.1 Shim, installer and `installed(wt)` [P1a]

**`installed(wt)` — the one definition** (I4, `--check`, §6 and the §7.1 pass-through all refer here). Let
`D` = `$(git rev-parse --path-format=absolute --git-common-dir)/hooks`, resolved with `cd … && pwd -P`.
`installed(wt)` is true iff `git -C <wt> rev-parse --path-format=absolute --git-path hooks`, resolved the same way,
equals `D`, and `D/pre-push` and `D/pre-commit` exist, are executable, and are **byte-identical to `origin/main`'s
`shim.sh`** (`git cat-file blob <origin/main commit>:.claude/tools/githooks/shim.sh`, compared with `cmp`). The
bytes decide, not a marker line: a shim edited in place, an older shim body, or a foreign file with a copied marker
is not installed. There is no earlier shim version to accept (nothing has been installed before P1a; no backward
compatibility). A hooks path that cannot be resolved, or a `shim.sh` that cannot be read from `origin/main`, makes it
false. A fresh repository with `core.hooksPath` unset and the shims in `.git/hooks` satisfies it (§3.2).
**Which copy evaluates it**: always `origin/main`'s `install.sh`, run in the §4.7 form — by the `mise` task (§4.7),
by `--check`, and by the §7.1 pass-through. No caller runs a working-tree `install.sh`, so there is one predicate,
not one per checkout.

The shim (body in `.claude/tools/githooks/shim.sh`, written by the installer as `pre-push` and `pre-commit`):

```sh
#!/bin/sh
# elidex-githook-shim (installed by `mise run install-githooks`; source .claude/tools/githooks/shim.sh)
n=$(basename "$0"); ref=refs/remotes/origin/main; p=.claude/tools/githooks/$n
r=$(git for-each-ref --format='%(refname) %(objectname)' "$ref") || { echo "elidex githooks: cannot read $ref" >&2; exit 1; }
c=$(printf '%s\n' "$r" | sed -n "s|^$ref ||p")
e=; [ -z "$c" ] || e=$(git ls-tree "$c" -- "$p") || { echo "elidex githooks: cannot read $c:$p" >&2; exit 1; }
if [ -z "$e" ]; then
  echo "elidex githooks: $p is not on origin/main yet; $n is unguarded here" >&2
  exit 0
fi
body=$(git cat-file blob "$c:$p") || { echo "elidex githooks: cannot read $c:$p" >&2; exit 1; }
exec bash -c "$body" "$n" "$@"
```

stdin passes through `exec` to the script (`bash -c` reads the program from its argument, not from stdin). The
script's `$0` is the hook name. Absence is only an empty answer with exit 0 (I3); `for-each-ref` matches a pattern
by prefix up to a slash, so the line is selected by its exact refname; `c` pins one commit for both reads, so a
concurrent fetch cannot pair the entry of one commit with the blob of another.

`install.sh`, run from any worktree of the target repository:
1. Compute `D`; refuse unless the invoking worktree's hooks path equals `D` (a global or worktree-level
   `core.hooksPath` would make the installer write somewhere else).
2. For each of `pre-push`, `pre-commit`: if a file exists in `D` and its second line does not begin with
   `# elidex-githook-shim`, refuse and name it (someone else's hook); else write `origin/main`'s `shim.sh` bytes
   and `chmod 755`. The marker only decides whether the installer may overwrite; `installed(wt)` decides on bytes.
   Re-running is a no-op apart from rewriting the same bytes. `origin/main` without `shim.sh` → refuse (V1).
3. For each worktree in `git worktree list --porcelain`, print any for which `installed(wt)` is false; any such
   worktree makes the exit non-zero.
4. Print what was written. Exit 0 / 1.

`install.sh --check` computes `D` and does step 3, and writes nothing. `mise run install-githooks -- --check` passes
the flag through; this invocation is the PM's standing pass item and the one the negative control uses (§7.5 E19).
`install.sh --installed` evaluates `installed(wt)` for the worktree of its cwd only: exit 0 when true, 1 otherwise,
writes nothing, prints nothing on 0. In every mode `install.sh` reads nothing relative to `$0` (it runs as `bash -c`
from a variable) and takes the shim bytes from `origin/main`, not from the working tree. `--installed` is the predicate
as a command, so the §7.1 pass-through calls it instead of re-implementing it; its interface (exit 0 / 1, silent on
0, runnable in the §4.7 form from any worktree of the repository) is a contract with that global hook (§9).

### §4.2 pre-push [P1a]

As §2 II1–II9, in that order per line; all reasons are collected and printed to stderr with the rule's name and the
remedy (FF: "merge, do not rebase or force"; branch: "push from the worktree that has the branch checked out";
main: "open a PR"). Current branch: `git symbolic-ref --short -q HEAD` (empty = detached). The ratchet is run as in
II9, after `python3 -c ''` succeeds; its exit code is mapped as the F table's II9 rows (0 pass, 1 / 2 / 3 reject,
each with its own message: finding, usage, error).

### §4.3 pre-commit [P1a]

1. `GIT_INDEX_FILE`, when set and relative, is made absolute against the hook's starting cwd (VI1); the hook never
   changes cwd before step 6. `top=$(git rev-parse --show-toplevel)` (failure rejects, F).
2. `git diff --cached --name-only -z --diff-filter=ACMR -- '*.rs'` (VI2). None → exit 0.
3. `ELIDEX_FMT_OK` set: equal to `git write-tree` → exit 0 with a note; otherwise a note naming both trees, and go on
   (VI3).
4. `rustfmt --version` fails → warning, exit 0 (F).
5. `git show :.rustfmt.toml` into a temp dir; absent → reject (VI4).
6. Each file: `git show ":<path>" | rustfmt --config-path <temp dir>`, compared byte for byte with the blob.
   rustfmt non-zero or a difference → that path is listed. Any listed → exit 1 with the list and "run
   `cargo fmt --all`, re-stage". The temp dir is removed on every exit.

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
| `dry_verdict_match`, `error_verdict_match`, `status_comment_match` | regex searched in the body (jq `test`) |
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
environment (or on the line). The E2E setup sets it (§7.5).

Because of step 3, a PR that changes `merge-pr` is merged with `origin/main`'s copy; P1b itself, before which
`origin/main` has no copy, is merged by hand — the last raw merge outside break-glass. The Codex-head logic is
written once, here; the copy in the global `gh-pr-merge-head-guard.sh` stays until the §7.3 `:36` approval
(IV5).

### §4.5 Ratchet `--rev` [P1a]

`ecs_mission_ratchet.py --rev <sha>`, run with its cwd inside the repository (any worktree, or a bare repository);
check only, no other flag combines with it. **Every verdict input comes from `<sha>`: the running copy only
orchestrates.** This is the one statement of the rule (II9, I5 and §6 refer here). The running copy (under pre-push,
`origin/main`'s) does exactly this:
1. Locate the repository from the cwd (`git rev-parse --absolute-git-dir`, with `git_env()`; nothing relative to
   `__file__`, which is `'<stdin>'` under `python3 -`, §3.2). Resolve `<sha>` with `rev-parse --verify
   <sha>^{commit}`; failure → exit 3.
2. **No ratchet at `<sha>`** (`git ls-tree <sha> -- .claude/tools/ecs_mission_ratchet.py` prints nothing, exit 0):
   `A` = the oldest commit on `refs/remotes/origin/main` that added the file (`git log --reverse --diff-filter=A
   --format=%H refs/remotes/origin/main -- <path>`, first line). `git merge-base --is-ancestor A <sha>`: exit 1 →
   cut before the ratchet existed, print "no ratchet at `<sha>` (before `<A>`)", exit 0; exit 0 → the branch
   deleted it, print that, exit 1; anything else, or no `A` → exit 3.
3. **Materialise** `<sha>`'s tree in a temp dir `T`: `git read-tree <sha>` into a temp `GIT_INDEX_FILE`, then
   `git checkout-index -a --prefix=T/` with the same variable (`git_env()` plus that one), so `T` holds what a
   checkout of `<sha>` writes — symlinks as symlinks, attributes applied — and the real index is not touched
   (measured, §3.2).
4. **Load `<sha>`'s module**: exec `T/.claude/tools/ecs_mission_ratchet.py` into a fresh namespace with
   `__name__ = "ecs_ratchet_rev"` (its `main` does not run on load) and `__file__` = that path (so its
   `DEFAULT_ALLOWLIST` is `T`'s, i.e. `<sha>`'s allowlist).
5. **Set exactly two locators** in that namespace: `repo_root_of` → returns `T`; `tracked_files` → returns `<sha>`'s
   paths (`git ls-tree -r -z --name-only <sha>`, decoded and sorted as the module's own `tracked_files` does). A
   checkout of `<sha>` has that root and an index equal to `<sha>`'s tree, so these are the answers `<sha>`'s own
   `git rev-parse --show-toplevel` and `git ls-files` would give there (§3.2: the path lists are equal).
6. **Run `<sha>`'s `main([])`** and exit with its return value if it is 0 or 1. Everything that decides is `<sha>`'s:
   the file filter (regular file via `os.path.isfile`, which follows symlinks; the NUL rule; U+FFFD decoding), the
   classifier `denied`, `SELF_FILES`, the allowlist path and parser, `malformed` and its row rules (which call
   `<sha>`'s `denied`), `errors`, the output lines. Any other return value, any exception (the orchestrator catches
   all of them), or a missing interface name → print the reason's last line, exit 3.
7. `T` and the temp index are removed on every exit.

- **Interface contract**: `main(argv)`, `repo_root_of(path)`, `tracked_files(repo_root)` and a module-level
  `DEFAULT_ALLOWLIST` derived from `__file__`. Both committed versions have them (69fbe61f #609 and 1b727d18 #652:
  `git show <c>:.claude/tools/ecs_mission_ratchet.py`, the three `def`s and the `DEFAULT_ALLOWLIST` line).
  `test_ecs_mission_ratchet.py` pins them on the in-tree module, so a PR that renames one is red in CI before any
  `<sha>` carries the rename.
- **Symlinks**: the working-tree mode follows them (`os.path.isfile`), and `--rev` does too, because `T` holds them
  as a checkout would. There is no second rule to keep in step. (Round 1's "a symlink is skipped, as the
  working-tree mode skips non-regular files" was wrong: the working-tree mode reads through a symlink to a file.)
- **Exit codes**: 0 ok, or no ratchet before its introduction; 1 findings, or the ratchet deleted on the branch;
  2 usage (the orchestrator's own); 3 error. The working-tree mode keeps 0 / 1 / 2.
- **Cost**: materialising the tree at fca1f6cd is 1957 blobs, 42.9 MB (`git ls-tree -r -l`); the run time of
  `--rev` is unmeasured, and the PR measures it on one push.
- The module docstring's "reads every tracked file" paragraph gains the `--rev` paragraph.

### §4.6 Tests

- `test_githooks.py` [P1a]: builds, per test, a scratch repository + bare remote + a linked worktree in a temp dir
  (T4), gives it an `origin/main` holding the in-tree scripts (T5), installs the shim with `install.sh`, and runs
  real `git push` / `git commit`. The stub `rustfmt` (T3) and a stub ratchet (exit code chosen per test, records
  that it ran and its `--rev` argument) are reached through `PATH` and the scratch `origin/main`; a fake `python3`
  that exits 1 covers the interpreter-cannot-start path.
- `test_ecs_mission_ratchet.py` [P1a]: the `--rev` cases of §6.
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
- `isolated_git.py` [P1a, imported by P1b]: T4; loaded by path (`importlib.util.spec_from_file_location`, as
  `test_ecs_mission_ratchet.py` does).
- Mutants: each mutant is a one-line text replacement in a temp copy of the script. The harness asserts the
  replaced text occurs exactly once before applying it (so a later edit of the script cannot turn a mutant into a
  no-op), runs the case named in §6 against the mutated copy, and asserts the case's verdict flips.
- Each PR re-derives the `tools` job's `timeout-minutes` from its own run, per the rule in that job's comment.

### §4.7 mise task [P1a]

```toml
[tasks.install-githooks]
description = "Install the githook shims into this repository's hooks directory (all worktrees). Not part of ci."
run = 'b=$(git show refs/remotes/origin/main:.claude/tools/githooks/install.sh) && bash -c "$b" install.sh'
```

`mise run install-githooks -- --check` appends `--check`, which reaches the script as `$1`. The `&&` makes a failed
read fail the task: a pipe into `bash -s` would run an empty program on a failed read, and an empty program exits 0.
The same form is the one the §7.1 pass-through runs.

### §4.8 The bypass-guard patch (global; the PM applies it at §7.1 A2) [P1a cut-over]

PM 900: the user approved extending `~/.claude/hooks/git-hook-bypass-guard.py`; the PM's A2-scope decision sets the
scope to the whole write grammar of `core.hooksPath` — the legacy options and the `config set` / `unset`
subcommands, `--add` / `--replace-all` / `--file`, a case-insensitive key, and `GIT_CONFIG_PARAMETERS`. The patch is
written here, against the file as read on 2026-11-21 (`shasum -a 256` =
`9902fe77ac1ebbf9912315265b4f93e24b70a7603d91d5945bcca7469e1a0438`; if the file differs when the PM applies it, the
patch is re-derived, not forced). It follows the file's own structure: one more case in `check_git` (by
subcommand), the env rule widened inside `env_hooks_assign`, the same `in_scope` (no copy of its roots), the same
fail-open on an internal error, and tests in the existing `git-hook-bypass-guard.test.py` lists.

**No silent overlap with the existing rules.** `-c core.hooksPath=…` (and `-c core.hooksPath …`,
`-ccore.hooksPath=…`), `--config-env=core.hooksPath=…` and a `GIT_CONFIG_KEY_<n>=core.hooksPath` assignment are
already denied — they are git's global options and environment, read before the subcommand. The new rule reads only
what follows the `config` subcommand, plus one more variable name in the env rule; a command carrying both kinds is
denied once, with the first hit's name.

**Scope of the new rule.** A write is denied when the git segment's effective cwd is in scope (as every rule of the
file), or when `--file` names a path in scope (an elidex `.git/config` written from outside). `--global` /
`--system` from outside the scope stay allowed: the shared repository's local `core.hooksPath` (I1) outranks them.
The docstring names what the rule cannot see (indirect `include.path`, a swapped `GIT_CONFIG_GLOBAL` / `HOME`, a
direct file write, `--git-dir` from outside); the persistent ones are detected by `--check` (I4).

```diff
--- a/git-hook-bypass-guard.py
+++ b/git-hook-bypass-guard.py
@@ -6,13 +6,23 @@
     `--config-env=core.hooksPath=...` (キー名は大文字小文字を区別しない。`git -C dir` 付きも)
   - commit / push / merge / pull / rebase / cherry-pick / am / revert の `--no-verify` (--no-ve... の省略形も)、
     `git commit -n` (-an / -nm のような束ね形も)
+  - `git config` で core.hooksPath を書く形すべて: 旧形式 (`git config [--local|--worktree|--global|--system|
+    -f <file>|--file[=]<file>] core.hooksPath <dir>`、option は位置引数の後ろでもよい)、`--add` / `--replace-all` /
+    `--unset` / `--unset-all`、subcommand `set` / `unset`、section ごとの `--remove-section` / `--rename-section` /
+    `remove-section` / `rename-section` (section `core`)、editor で開く `-e` / `--edit` / `edit` (どの key も書ける)。
+    読む形 (`--get` 系、`-l` / `--list`、`get` / `list`、key だけの旧形式 `git config core.hooksPath`) は通す。
+    `--file` の先が elidex の中なら、cwd が外でも deny
 対象は実効 cwd (hook JSON の cwd、先行する `cd`、`git -C <dir>`) が elidex の checkout / worktree の中にある時だけ。
 範囲の考え方は ecs-mission-guard.py の in_scope と同じ (SCOPE_ROOTS / SCOPE_PARENT)。外では何も出さず exit 0。
 
 shell の字句は shlex で quote を考慮して分け、`&& ; | ( )` と改行で区切った各 command を見る。
 `bash -c '...'` / `sh -c` / `eval` の引数は再帰して見る。引数の文字列の中身 (grep の pattern、commit message) は見ない。
-`GIT_CONFIG_KEY_<n>=core.hooksPath` の env 代入も deny (command 先頭の `VAR=x`、`env VAR=x`、同じ command 内で先行する `export VAR=x`)。
-見えない所 (blind spot): `$(...)` / バッククォート内、変数展開で組み立てた git、git alias。
+`GIT_CONFIG_KEY_<n>=core.hooksPath` と、値に core.hooksPath を含む `GIT_CONFIG_PARAMETERS=...` の env 代入も deny
+(command 先頭の `VAR=x`、`env VAR=x`、同じ command 内で先行する `export VAR=x`)。
+見えない所 (blind spot): `$(...)` / バッククォート内、変数展開で組み立てた git、git alias。core.hooksPath を間接に
+変える形 (`include.path` / `includeIf.*.path` の書き込み、`-c include.path=...`、`GIT_CONFIG_GLOBAL` / `HOME` を
+差し替えた file)、`.git/config` / `config.worktree` への直接の file 書き込み、cwd の外から `--git-dir` で elidex を
+指す形。永続する迂回は `mise run install-githooks -- --check` (elidex の P1 plan memo の installed(wt)) が見つける。
 
 Fails OPEN: どんな内部エラーでも no output, exit 0。
 Env overrides (tests): GIT_BYPASS_GUARD_STATE_DIR, GIT_BYPASS_GUARD_EXTRA_SCOPE (os.pathsep 区切り)。
@@ -35,6 +45,15 @@
 COMMIT_VALUE_SHORT = set("mFCctSu")
 COMMIT_VALUE_LONG = {"--message", "--file", "--reuse-message", "--reedit-message", "--author", "--date",
                      "--template", "--fixup", "--squash", "--cleanup", "--trailer", "--gpg-sign"}
+# git config (git-config(1) COMMANDS / DEPRECATED MODES / OPTIONS, git 2.56.1)
+CONFIG_SUBCMDS = {"list", "get", "set", "unset", "rename-section", "remove-section", "edit"}
+CONFIG_READ_SUBCMDS = {"list", "get"}
+CONFIG_READ_ACTIONS = {"--get", "--get-all", "--get-regexp", "--get-urlmatch", "--get-color", "--get-colorbool",
+                       "-l", "--list"}
+CONFIG_WRITE_ACTIONS = {"--add", "--replace-all", "--unset", "--unset-all", "--rename-section", "--remove-section",
+                        "-e", "--edit"}
+# long options that take a separate value (`--opt=value` is one token); `-f` is handled with the short clusters
+CONFIG_VALUE_LONG = {"--file", "--blob", "--type", "--default", "--comment", "--value", "--url"}
 
 
 def state_dir():
@@ -95,9 +114,16 @@
 
 
 def env_hooks_assign(tok):
+    """env 代入 1 つ -> 拾った形の名前、または ""。"""
     k, eq, v = tok.partition("=")
-    return bool(eq) and k.upper().startswith("GIT_CONFIG_KEY_") and k[len("GIT_CONFIG_KEY_"):].isdigit() \
-        and v.strip().lower() == "core.hookspath"
+    if not eq:
+        return ""
+    if k.upper().startswith("GIT_CONFIG_KEY_") and k[len("GIT_CONFIG_KEY_"):].isdigit() \
+            and v.strip().lower() == "core.hookspath":
+        return "GIT_CONFIG_KEY_n=core.hooksPath"
+    if k.upper() == "GIT_CONFIG_PARAMETERS" and "core.hookspath" in v.lower():
+        return "GIT_CONFIG_PARAMETERS=core.hooksPath"
+    return ""
 
 
 def hookspath_key(s):
@@ -138,6 +164,58 @@
             if ch in COMMIT_VALUE_SHORT:
                 break
     return False
+
+
+def config_hookspath_write(args, cwd):
+    """`git config` の後ろの token -> (violation or None, `--file` の先 or None)。
+    位置引数は option とその値を除いた全部 (git は option を位置引数の後ろにも置ける)。未知の option が値を取って
+    位置がずれても、core.hooksPath が位置引数のどこかにあり読む形でなければ書く形として拾う (未知は deny 側)。"""
+    pos, acts, target, i = [], set(), None, 0
+    while i < len(args):
+        a = args[i]
+        if a == "--":
+            pos.extend(args[i + 1:])
+            break
+        if a in CONFIG_VALUE_LONG and i + 1 < len(args):
+            if a == "--file":
+                target = args[i + 1]
+            i += 2
+            continue
+        if a.startswith("--file="):
+            target = a[len("--file="):]
+        elif a.startswith("--"):
+            acts.add(a.split("=", 1)[0])
+        elif a.startswith("-") and len(a) > 1:
+            for k, ch in enumerate(a[1:], 1):
+                if ch == "f":
+                    if k < len(a) - 1:
+                        target = a[k + 1:]
+                    elif i + 1 < len(args):
+                        target = args[i + 1]
+                        i += 1
+                    break
+                acts.add("-" + ch)
+        else:
+            pos.append(a)
+        i += 1
+    if target is not None:
+        target = os.path.join(cwd, os.path.expanduser(target))
+    sub = pos[0] if pos and pos[0] in CONFIG_SUBCMDS else None
+    names = pos[1:] if sub else pos
+    if sub == "edit" or acts & {"-e", "--edit"}:
+        return "git config --edit", target
+    if sub in ("rename-section", "remove-section") or acts & {"--rename-section", "--remove-section"}:
+        if names and names[0].lower() == "core":
+            return "git config remove/rename-section core", target
+        return None, target
+    if not any(n.lower() == "core.hookspath" for n in names):
+        return None, target
+    reads = sub in CONFIG_READ_SUBCMDS or bool(acts & CONFIG_READ_ACTIONS)
+    if reads and not acts & CONFIG_WRITE_ACTIONS:
+        return None, target
+    if sub is None and len(names) == 1 and not acts & CONFIG_WRITE_ACTIONS:
+        return None, target  # `git config core.hooksPath` = 旧形式の読み
+    return "git config core.hooksPath", target
 
 
 def check_git(args, cwd):
@@ -173,6 +251,12 @@
             break
     sub = args[i] if i < len(args) else None
     rest = args[i + 1:] if sub else []
+    if sub == "config":
+        v, target = config_hookspath_write(rest, eff)
+        if v:
+            hits.append(v)
+            if target and not in_scope(eff) and in_scope(target):
+                eff = target
     if sub in HOOKED_SUBCMDS:
         skip = commit_value_skips(rest) if sub == "commit" else set()
         for k, r in enumerate(rest):
@@ -190,10 +274,10 @@
     """-> (violation, effective_cwd_of_the_violating_git) or (None, None)。cd を追跡する。"""
     if depth > 3:
         return None, None
-    exported = False  # `export GIT_CONFIG_KEY_n=core.hooksPath` seen earlier in this command
+    exported = ""  # `export GIT_CONFIG_KEY_n=core.hooksPath` / `GIT_CONFIG_PARAMETERS=...` seen earlier in this command
     for seg in segments(cmd):
         j = 0
-        seg_env = any(env_hooks_assign(t) for t in seg[:next((n for n, t in enumerate(seg) if not (is_assign(t) or os.path.basename(t) in WRAPPERS)), len(seg))])
+        seg_env = next(filter(None, map(env_hooks_assign, seg[:next((n for n, t in enumerate(seg) if not (is_assign(t) or os.path.basename(t) in WRAPPERS)), len(seg))])), "")
         while j < len(seg) and (is_assign(seg[j]) or os.path.basename(seg[j]) in WRAPPERS):
             j += 1
         if j >= len(seg):
@@ -204,11 +288,11 @@
             if d:
                 cwd = os.path.join(cwd, os.path.expanduser(d))
         elif prog == "export":
-            exported = exported or any(env_hooks_assign(t) for t in rest)
+            exported = exported or next(filter(None, map(env_hooks_assign, rest)), "")
         elif prog == "git":
             v, eff = check_git(rest, cwd)
             if not v and (seg_env or exported):
-                v = "GIT_CONFIG_KEY_n=core.hooksPath"
+                v = seg_env or exported
             if v and in_scope(eff):
                 return v, eff
         elif prog in SHELLS or prog == "eval":
```

**Tests** (added to the `DENY` / `ALLOW` lists and as two methods of `git-hook-bypass-guard.test.py`; written and
run against a scratch copy of the patched file on 2026-11-21: 14 tests OK; the same tests against the unpatched
file fail, 39 subtests). Each positive form was measured to reroute, or to write, the key (§3.2):

| kind | commands (cwd in scope unless stated) | expected |
|---|---|---|
| legacy write | `git config core.hooksPath /alt`; with `--local`, `--worktree`, `--global`, `--system`; `git config core.hooksPath /alt --worktree`; `--worktree core.HOOKSPATH`, `--worktree CORE.hookspath`; `--add --worktree`; `--replace-all`; `--type=path --worktree`, `--type path --worktree`; `--comment x`; `--worktree -- core.hooksPath /alt` | deny |
| legacy unset / section | `--unset`, `--unset-all core.hooksPath`; `--remove-section core`; `--rename-section core zz` | deny |
| subcommands | `config set --worktree`, `set --all`, `set --append`; `config unset`, `unset --all`; `remove-section core`; `rename-section Core zz` | deny |
| file target | `--file .git/config.worktree`, `-f …`, `-f.git/…`, `--file=…`; from outside: `--file <elidex>/.git/config core.hooksPath /alt`, `-f <elidex>/.git/config --unset core.hooksPath` | deny |
| editor | `git config --edit --worktree`, `-e`, `config edit` | deny (an editor can write any key) |
| reach | `git -C <elidex> config --worktree …`; `cd <elidex> && git config --worktree …` from outside; `bash -c '…'`; `git status && git config …` | deny |
| env | `GIT_CONFIG_PARAMETERS="'core.hooksPath'='/alt'" git commit -m x`; `env GIT_CONFIG_PARAMETERS="'core.hookspath=/alt'" git push origin HEAD`; `export GIT_CONFIG_PARAMETERS="'CORE.HOOKSPATH'='/alt'"; git commit -m x` | deny |
| reads (negative) | `git config core.hooksPath`, `--get`, `--get-all`, `--show-origin --get`, `--worktree core.hooksPath`, `--get-regexp core.hooks`, `--list`, `-l --show-scope`, `config get`, `config get --all`, `config list` | allow |
| other keys (negative) | `--worktree user.name a`; `config set core.editor vim`; `--unset user.email`; `--remove-section alias`; `--worktree core.hooks /alt`; `--worktree core.hooksPathX /alt`; `GIT_CONFIG_PARAMETERS="'user.name'='a'" git commit -m x` | allow |
| text, not a command (negative) | `git commit -m 'git config core.hooksPath /alt'`; `git log --grep 'git config --worktree core.hooksPath'` | allow |
| out of scope (negative) | every `DENY` entry from `/private/tmp` (the existing loop); `git config --file /private/tmp/x core.hooksPath /alt` from `/private/tmp` | allow |

**Mutants** (each a one-line replacement in a temp copy, run through the file's own `GIT_BYPASS_GUARD_SCRIPT`
override; all nine were killed by the tests above on 2026-11-21):

| replaced line | → | killed by |
|---|---|---|
| `if not any(n.lower() == "core.hookspath" for n in names):` | exact-case compare | `core.HOOKSPATH` forms |
| `if k.upper() == "GIT_CONFIG_PARAMETERS" and` | `if False and` | the env rows |
| `if target and not in_scope(eff) and in_scope(target):` | `if False:` | the from-outside `--file` rows |
| `if sub is None and len(names) == 1 and not acts & CONFIG_WRITE_ACTIONS:` | drop `len(names) == 1` | every legacy write |
| `CONFIG_READ_SUBCMDS = {"list", "get"}` | add `"set"` | the `set` rows |
| `    if sub == "config":` | another name | every config row |
| `        if names and names[0].lower() == "core":` | exact-case compare | `rename-section Core zz` |
| `    if sub == "edit" or acts & {"-e", "--edit"}:` | `if sub == "edit":` | `--edit`, `-e` |
| `            pos.extend(args[i + 1:])` | `pass` | `--worktree -- core.hooksPath /alt` |

**The PM's application and E2E line** (global: it acts on every session on this machine at once, other
repositories' sessions included, `~/.claude` CLAUDE.md "変更の blast radius"): back up both files
(`cp -p ~/.claude/hooks/git-hook-bypass-guard{,.test}.py <backup dir>/`, the path into the PM's log), apply the
diff and the test additions, then `python3 ~/.claude/hooks/git-hook-bypass-guard.test.py` → OK, then the nine
mutants above → each killed. E2E, from a Claude session whose cwd is an elidex worktree: the Bash call
`git config --worktree core.hooksPath /tmp/x` → denied with "git config core.hooksPath" in the reason, and
`git rev-parse --git-path hooks` unchanged afterwards; `git config --get core.hooksPath` → runs. If the hook files
are dotfiles-managed, the edit goes through the dotfiles session.

## §5 Decisions and their ground

Rung = which level decided it (CLAUDE.md order: spec → design principle → measurement; here "spec" means the git /
gh / rustfmt documents of §3, and "incident" means a recorded incident in an existing hook's header).

| # | slice | decision | rung |
|---|---|---|---|
| D1 | P1a | FF rule on every ref; delete and unknown remote sha reject (II3–II5) | PM scope; git docs (§3 pre-push and `--is-ancestor` rows); principle Security by structure (no server backstop off `main`); measured exit codes |
| D2 | P1a | branch rule keyed on the remote ref, every `refs/heads/` line, attached HEAD only (II2) | PM scope; incident #285 (`git-push-branch-guard.sh` header); git docs (local ref is not a name). ⚠ wider than the current hook, which only sees `HEAD:<x>` |
| D3 | P1a | any `refs/heads/main` target rejects (II1) | CLAUDE.md Workflow "main 直接 push 禁止"; PLAN row 17 |
| D4 | P1a | check every line, print every reason, one bad line aborts the push (II6) | git docs (pre-push exit row) |
| D5 | P1a | ratchet after the ref rules, skipped on empty / delete-only pushes; warn-and-pass on an interpreter that cannot start (II8, II9, F) | principle: unknowns fall to the side with a backstop (CI `tools`); measured shim exit 1 |
| D6 | P1a | pre-commit reads index blobs, not working-tree files (VI1) | measured temporary index; the current hook's header admits the staged-vs-working-tree edge |
| D7 | P1a | format-and-compare, not `--check`, on stdin | measured (§3.2); `rustfmt --help` says the opposite |
| D8 | P1a | rustfmt config from the staged `.rustfmt.toml`, no edition in the hook (VI4) | One issue, one way (the edition has one home); measured `--config-path` on stdin |
| D9 | P1a | opt-out `ELIDEX_FMT_OK` (a git hook cannot see a `# fmt-ok` comment) | PM scope |
| D10 | P1a | rustfmt cannot run → warn and pass | principle as D5 (CI `check` fmt step); incident #487's "trains around the guard" |
| D11 | P1a | shim runs `origin/main`'s copy with `bash -c`; passes while `origin/main` has none, read only from an exit-0 empty answer; an error rejects (I2, I3) | PM 893 (C5): removes the uncommitted-edit and old-branch fail-open; git docs (exec bit); principle: unknowns by fail-safe direction (round 2, A3 MIN); measured absence answers (§3.2) |
| D12 | P1a | installer refuses a hooks path other than `<common dir>/hooks` and a foreign hook; reports worktrees where `installed(wt)` is false; `installed(wt)` compares bytes with `origin/main`'s `shim.sh`, no earlier version accepted; every caller runs `origin/main`'s `install.sh` | Security by structure; measured `core.hooksPath` and `extensions.worktreeConfig`; CLAUDE.md "後方互換性は維持しない" (no earlier shim accepted); One issue, one way (one predicate, round 2 A2-2 / A3 MIN) |
| D13 | P1b | merge-pr order of §2 III; `H` read once | drive skill §6 steps 2–4 (`merge at head <sha>`, re-check head and `mergeStateStatus`, `--match-head-commit`); PM scope |
| D14 | P1b | no Codex assessment → refuse (the current hook allows) | principle: unknown falls to refuse where nothing is behind; incident #367 |
| D15 | P1b | every `gh` / `jq` error → refuse | as D14 |
| D16 | P1b | `mergeStateStatus` allow-list `CLEAN` | as D14 (an enum value added later refuses) |
| D17 | P1b | `--delete-branch=false` written out | incident lexime #354 (`gh-pr-merge-delete-branch-worktree-guard.sh` header) |
| D18 | P1b | post-merge: the existing scripts, once, with the JSON, the caller's cwd and its `CLAUDE_PROJECT_DIR` | PM scope; PM 893 (D15); the scripts' own interface (sync lines 18–28, cleanup lines 32–47, 70–79) |
| D19 | P1b | the Codex-head logic lives once, in `merge-pr`; the global copy goes with the §7.3 `:36` approval | One issue, one way |
| D20 | both | tests: real git, stub rustfmt, fake gh, isolated by construction (T4) | the `tools` job boundary comment (T3); the user rule that fixtures take the real shape; PM 893 (C11) |
| D21 | both | PLAN.md is the ratified program memo (Q0) | PM 873 (user approved the whole plan), PM 874 (ordered this memo); PLAN.md:3 |
| D22 | P1b | `ELIDEX_PM_DIR` required; unset or missing → refuse (Q1); set by the PM's kickoff | principle: the merge is irreversible, so an unknown falls to refuse; PM 893 (D14) |
| D23 | P1b | `merge-pr` refuses unless byte-identical to `origin/main`'s copy, after a fetch (Q2) | Security by structure: a PR must not be able to weaken its own merge gate |
| D24 | P1b | every CI check complete and green, both rollup shapes, besides `CLEAN` (Q3) | CLAUDE.md Workflow "CI 全 pass を目視確認してから squash merge", made mechanical; PM 893 (C7) |
| D25 | P1b | `--dry-run` runs every check and prints the merge command (Q4) | the PM's E2E needs the good path on a real PR without an irreversible act |
| D26 | P1a | a tag moved forward passes (Q5) | tags are not part of the PR / merge flow; stated limit |
| D27 | both | two slices, P1a first; the seam is the empty II × III | PM 893 (A1) |
| D28 | P1a | the ratchet reads the pushed tree (`--rev`), once per distinct pushed sha; every verdict input is `<sha>`'s (its whole module on its materialised tree), the running copy only orchestrates; a ratchet deleted on the branch rejects | PM 893 (C6); round 2 A2-1 (driver decision: one source for every verdict input); fail-safe direction (A3 MIN) |
| D29 | P1a | `ELIDEX_FMT_OK` must equal the staged tree | PM 893 (E22): an opt-out covers one commit |
| D30 | P1b | Codex: dry verdict on `H` after the last trigger, no finding in any channel, 0 unresolved Codex threads; the `Completed` row is not an assessment; every bot item admitted, its sha derived as the global resolver (marker → first `/blob/<sha>/` → `commit_id`), a round item with no sha is a finding; the trigger by literal containment from a non-bot author | PM 893 (C7); canonical rule in the global external-review SKILL; #492 (latest item does not win), #494 R2 (threads), #525 (`Completed`); round 2 A5-1 (driver decision: no-sha item = finding) |
| D31 | P1b | reviewer values read from `origin/main`'s `external-converge/project.md` § reviewer, not copied into the script | One issue, one way (the overlay is their canonical site) |
| D32 | P1b | no override flag; break-glass = a recorded raw merge by the PM (§7.4); its post-merge sync / cleanup is the PM's, by where the merge ran | PM 893 (C8); round 2 A3 / A2 MIN |
| D33 | P1a | `--check` is a standing PM-pass item, through the PM's own invocation; its line in `.claude/skills/pm/SKILL.md` lands in P1a | PM 893 (C9): hand-run git stays possible after P2; round 2 A5 MIN (in the PM procedure before P3) |
| D34 | P1b | invocation site = the `main` worktree's copy, run from the caller's cwd | PM 893 (D13); step 2 refuses a PR worktree |
| D35 | P1b | Dependabot PRs get a `@codex review` from the CI/tools lane, which runs them (`pm:223`, PM 240), then `merge-pr` | PM 893 (E29); no Dependabot exemption, no break-glass for them; round 2 A5 MIN (the lane, not the PM) |
| D36 | P1b | the go token's trust model is accidents + sandboxed agents | PM 893 (E28); stated limit III4 |
| D37 | P1b | a merge queue on the base refuses | PM 893 (E24): main has none today |
| D38 | P1a | the §4.8 bypass-guard patch: the whole write grammar of `core.hooksPath` and `GIT_CONFIG_PARAMETERS`; `--file` in scope denies from anywhere | PM 900 (user approved); the PM's A2-scope decision; measured forms (§3.2) |
| D39 | P1a | P1a break-glass: the PM, in a terminal, `--no-verify`, recorded; the fix PR then takes the normal gate (§7.4) | round 2 A3-2 (driver decision); D32's pattern (no override, a recorded hand action) |
| D40 | both | the raw-merge gap until `:36` is stated with owner (PM) and trigger (P1b's merge), not closed here | PM 893 (1) (approved scope); round 2 A3-1 (driver decision) |
| D41 | P1a | the pass-through defers only when every git segment's effective cwd is in scope and installed; it imports the bypass guard's derivation and `in_scope` | round 2 A2-3 (driver decision); One issue, one way (no second derivation, no copy of the roots) |
| D42 | P1b | the go token path stays `$ELIDEX_PM_DIR/handoff/codex-lanes/go/<PR>-<H>` | PLAN §6 decision 6 fixes `$PMD/handoff/codex-lanes/go/<PR>-<sha>` (ratified, PM 873); round 2 Axis 1 MIN answered by it |

## §6 Control / mutant table

Each row: a control that must pass, a bad input that must be rejected, and the one-line mutant the bad (or control)
case must kill.

| slice | decision | control (passes) | bad input (rejected) | mutant the test kills |
|---|---|---|---|---|
| P1a | II3 new ref | push a new branch (remote sha 40 zeros); a 64-zero line fed to the script | — | zero test written as the 40-character literal → the 64-zero control fails |
| P1a | II5 FF | FF push of the current branch | push after `reset --hard HEAD~1` with `+<refspec>` | `--is-ancestor … \|\| reject` → `\|\| true` |
| P1a | II5 unknown remote | — | remote advanced by another clone, local never fetched (exit 128) | reject only on exit 1 |
| P1a | II4 delete | — | `git push origin --delete <current>` | skip lines whose local ref is `(delete)` |
| P1a | II2 branch | push `refs/heads/<current>` | `HEAD:other`, and `git push origin <other-local-branch>` | compare only when the local ref is `HEAD` (the Claude hook's scope) → the second bad case passes |
| P1a | II2 detached | detached HEAD pushes `HEAD:refs/heads/x` | detached HEAD force-pushes over `x` (II5 still rejects) | drop the detached exemption (control fails); exempt detached HEAD from II5 too (bad case passes) |
| P1a | II1 main | push `refs/heads/domain` | push to `refs/heads/main` from branch `main` (FF, branch matches), and from detached HEAD | drop the main rule; or match `*main` (the `domain` control fails) |
| P1a | II6 several refs | — | one good line + one bad line; the remote is unchanged afterwards | reject flag set inside a piped `while read` subshell (last line wins) |
| P1a | II7 tags | push a new lightweight and a new annotated tag | move a tag back with `+refs/tags/t` | exempt `refs/tags/` from II5 |
| P1a | II8 empty push | up-to-date push; the stub ratchet did not run | — | run the ratchet unconditionally |
| P1a | II9 ratchet | stub ratchet exit 0; `git push origin HEAD~1:refs/heads/<current>` (new ref) → the stub received `--rev` with `HEAD~1`'s sha, once per distinct sha | stub ratchet exit 1 | ratchet call `\|\| true`; pass `HEAD` instead of the line's local sha (the recorded argument differs) |
| P1a | II9 interpreter | fake `python3` that cannot start → pass with the warning | `origin/main` holds pre-push but not the ratchet; the read of `origin/main`'s ratchet blob fails (fake `git` for that call) | treat the start failure as a finding (control fails); `ratchet absent \|\| exit 0`; pipe `git show` straight into `python3 -` (the failed read runs an empty program, exit 0) |
| P1a | II9 exit mapping | stub ratchet exit 0 → pass | stub exit 2 → rejected "usage"; stub exit 3 → rejected "error" | reject only on exit 1 (exit 2 / 3 pass) |
| P1a | `--rev` tree | committed tree clean, an untagged browser mention added only in the working tree → `--rev HEAD` ok | the mention committed, then removed from the working tree only → `--rev HEAD` reports it | read the files from disk (`iter_texts(root)`) instead of `<sha>`'s blobs (both verdicts flip) |
| P1a | `--rev` allowlist / classifier | `<sha>` whose committed classifier denies an extra word and whose committed allowlist rows cover it → ok | — | set the running module's `denied` into `<sha>`'s namespace (scan misses the word: stale rows); set only the running module's `malformed` (it calls the running `denied`: the rows read "not a normalised deny line"); point `DEFAULT_ALLOWLIST` at the running copy's file (the rows are missing) |
| P1a | `--rev` SELF_FILES | `<sha>` whose committed `SELF_FILES` lists an extra fixture file full of engine names → ok | — | set the running module's `SELF_FILES` (the fixture's lines become findings) |
| P1a | `--rev` no ratchet | `<sha>` cut before the ratchet existed → "no ratchet (before `<A>`)", exit 0 | a branch commit that deleted the ratchet → exit 1 | exit 0 whenever the file is absent (the deletion passes); exit 1 whenever it is absent (the control fails) |
| P1a | `--rev` error | — | `<sha>`'s ratchet has a committed syntax error → exit 3, and pre-push rejects with "error", not "finding"; `<sha>`'s `main` returns 2 → exit 3 | let the exception propagate (Python exits 1: a traceback reads as a finding); pass any return value through |
| P1a | `--rev` symlink | a tracked symlink to a tracked file holding an untagged mention → both modes report it (same verdict) | — | run `checkout-index` under `-c core.symlinks=false` (the link becomes a file holding its target's path: `--rev` misses the mention) |
| P1a | `--rev` interface | the in-tree module exposes `main`, `repo_root_of`, `tracked_files`, `DEFAULT_ALLOWLIST` (pin) | `<sha>` whose module lacks `tracked_files` → exit 3 | treat a missing name as "no ratchet" (exit 0) |
| P1a | I2 which script | `origin/main` holds a pre-push that passes; the worktree's own copy is edited to reject everything (uncommitted) → the push passes | `origin/main` holds a rejecting pre-push; the worktree's copy is edited to `exit 0` → the push is rejected | shim runs `$top/.claude/tools/githooks/<name>` (the working-tree file) |
| P1a | I2 old branch | — | a branch cut before the scripts existed force-pushes → rejected (`origin/main`'s rules) | shim passes when the checkout lacks the script |
| P1a | I3 absent | `origin/main` without the script → pass, stderr note | — | shim `exit 1` when absent |
| P1a | shim stdin | — | bad push through the installed shim | shim runs the script with `</dev/null` |
| P1a | shim read error | — | a fake `git` wrapper on `PATH` makes one call fail: `for-each-ref`, then `ls-tree`, then `cat-file` → exit 1 each time | `\|\| exit 0` on the read; treat a non-zero `ls-tree` as absence (the `cat-file -e` shape) |
| P1a | D12 installer | install into a fresh scratch repo (key unset); re-install is a no-op | existing foreign `pre-push`; invoking worktree with a worktree-level `core.hooksPath` | overwrite unconditionally; drop the hooks-path equality test |
| P1a | I4 `--check` | fresh scratch repo with `core.hooksPath` unset, shims installed → exit 0, nothing written; same with the key set to `D` | one linked worktree with `git config --worktree core.hooksPath <other>` → listed, exit 1; the same through `--worktree include.path` of a file that sets the key → listed; a shim missing → exit 1; a shim with one byte changed and its marker intact → listed; `origin/main` without `shim.sh` → listed | compare `git config --get core.hooksPath` with `hooks` (the fresh-repo control fails); read the key from the main worktree only (misses the worktree-level value); print the divergence but exit 0; test the marker line instead of the bytes (the edited shim passes) |
| P1a | §4.7 read failure | the task's run string under `sh -c` with `origin/main` holding `install.sh` → runs it with `--check` as `$1` | `origin/main` without `install.sh` → non-zero | `git show … \| bash -s --` (the failed read runs an empty program, exit 0) |
| P1a | `--installed` | installed worktree → exit 0, no output | the I4-overridden worktree → exit 1; a shim missing → exit 1 | `exit 0` unconditionally; evaluate the main worktree instead of the cwd's |
| P1a | VI1 index | blob formatted, working-tree copy not | blob not formatted, working-tree copy formatted | read `$top/<path>` instead of `git show :<path>` |
| P1a | VI1 temp index | real index holds an unformatted `b.rs`; `git commit -m x a.rs` (formatted) passes | `git commit -a` with an unformatted tracked change | `unset GIT_INDEX_FILE` |
| P1a | D7 stdin check | — | unformatted blob (stub mirrors rustfmt: `--check` on stdin prints a diff, exits 0) | decide on `rustfmt --check` exit code |
| P1a | VI2 ACMR | deleting an unformatted `.rs` | renamed and reformatted-badly `.rs` | `--diff-filter=ACM` |
| P1a | VI3 opt-out | `ELIDEX_FMT_OK=$(git write-tree)` with a bad blob → committed, note | `ELIDEX_FMT_OK=1` with a bad blob → rejected; a tree sha from before a re-stage → rejected | honour any non-empty value; compare against `HEAD^{tree}` |
| P1a | VI4 config | staged `.rustfmt.toml` with edition 2021 and a staged `async fn` file → committed (stub reads the edition from `--config-path`) | `.rustfmt.toml` deleted in the index → rejected | `--config-path "$top"` (the working-tree config); `--edition 2021` written in the hook (the stub fails on the edition-2018 fixture) |
| P1a | VI5 merge | — | conflicted merge concluded with `git commit`, unformatted resolution staged → rejected | exit 0 when `MERGE_HEAD` exists |
| P1a | D10 env | `rustfmt --version` fails → pass with warning | blob rustfmt cannot parse | treat every rustfmt non-zero as an environment error (bad case passes) |
| both | T4 isolation | a scratch repository under the test's temp dir | the helper is handed the real repository → the test fails | drop the common-dir comparison |
| P1b | step 1 usage | — | `merge-pr abc`, `merge-pr` → exit 2, fake `gh` saw no `pr merge` | accept any argument |
| P1b | step 2 site | the copy in the scratch repository's `main` worktree | the same bytes run from a linked worktree's checkout → exit 1, names both | drop the site check |
| P1b | step 3 self check | running copy equals `origin/main`'s blob | running copy differs by one byte; `origin/main` has no copy; the fetch fails | drop the comparison; compare against the working-tree file instead of `origin/main` |
| P1b | step 4 PM dir | `ELIDEX_PM_DIR` set to an existing dir | unset; set to a missing dir | default to a path; test only "set" |
| P1b | step 6 merge queue | `mergeQueue` null | `mergeQueue` non-null → exit 1 | skip the query |
| P1b | step 7 token | token `<PR>-<H>` present | none; token for an older sha; token named with a 7-char sha | glob `<PR>-*`; prefix compare |
| P1b | step 8 Codex | dry verdict comment on `H` after the last trigger, 0 unresolved Codex threads; the same with the head review only on page 2 (#487 shape); the trigger inside a longer human comment ("fixed X. @codex review") counts | only a `Completed` summary row on `H` (lexime #332 shape); dry verdict + one unresolved Codex thread; dry verdict older than the last trigger; a finding comment and a dry verdict seconds apart (#492); a marker-less findings comment (heading + `/blob/<H>/…#L` link) beside a dry verdict (#506 R88 shape); a bot comment newer than the trigger with no marker and no blob link that is neither error nor status; a bot comment quoting the trigger is not a trigger (the dry verdict before the human trigger stays stale); verdict on an older sha; overlay key missing; `gh api` exits non-zero | accept the summary row; ignore threads; latest item wins (the #492 case passes); filter comments by the marker (the #506 case passes); drop items with no sha (the no-sha case passes); trigger by equality (the longer-comment control fails); trigger from any author (the bot's quote resets `T`); drop the trigger cut-off; drop `--paginate` (fake serves page 1 only without it); empty assessment → allow |
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

The repository documents that name the old procedure change in P3; their one list is §1 non-goals ("The repo docs
that name the old procedure"), not restated here. **Where each E2E case runs, stated once**: E1–E18 and E20–E23 in
a scratch `git clone --local` under `$S` (§7.5 setup); E19 and E24 in the shared repository, after A1 (E19) and
after A2 (E24); M1–M6 against a real open PR (they stop before the merge); M7 is the first real merge after B3.

### §7.1 P1a cut-over

| step | action | state after it | safe because |
|---|---|---|---|
| A0 | P1a merges | scripts on `origin/main`, nothing installed | nothing runs them; Claude hooks unchanged |
| A1 | E2E E1–E18 in a scratch clone (§7.5), then user approval (PLAN §6 item 2), then `mise run install-githooks` from the main worktree; it must exit 0; then E19 in the shared repository | git hooks + Claude hooks both decide (IV) | conjunctive: each push / commit needs both to allow; every branch is judged by `origin/main`'s rules (I2), so no list of unguarded branches is needed |
| A2 | prerequisites for A4, global, PM / user: apply the §4.8 patch to `~/.claude/hooks/git-hook-bypass-guard.py` with its tests and mutants, then E24; the PM runs `mise run install-githooks -- --check` in every PM pass from here on (standing, D33; the line is in the PM skill since P1a) | same | I4 is prevented for direct commands by Claude agents, detected by `--check` for everyone |
| A3 | one good push and one good commit in a real lane worktree | same | real use, both layers deciding |
| A4 | after A2; user approval of PLAN §6 item 3 (its text: 「git hook 移行後に Claude hook (`set:24,30`) を撤去するか (global)」), then **`:24` and `:30` only**: an elidex-scoped pass-through (below) instead of removal — narrower than that text (PM 893 (2)) | in an installed elidex worktree only the git hooks decide push-branch and fmt; everywhere else unchanged | V2 / V5 / V6 |

**The pass-through (A4).** `git-push-branch-guard.sh` (`:24`) and `cargo-fmt-precommit.sh` (`:30`) gain one early
exit, taken only when **every git segment** of the command passes both tests below; one segment that fails either
(or a command in which no git segment is found) and the hook runs unchanged:
- **Its effective cwd**, derived per segment exactly as `~/.claude/hooks/git-hook-bypass-guard.py` derives it (the
  hook JSON's `cwd`, a preceding `cd`, `git -C <dir>`; `segments()` and `check_git()` of that file), is in scope by
  that file's `in_scope`. The pass-through imports that file by path and calls its functions; it copies neither
  the derivation nor `SCOPE_ROOTS` / `SCOPE_PARENT`. (Two copies of the roots exist today — that file and
  `ecs-mission-guard.py`; the pass-through adds none. The roots themselves are read in that file, not here.)
- **`installed(wt)` holds** for that cwd: the hook runs the §4.7 form there —
  `b=$(git -C "$cwd" show refs/remotes/origin/main:.claude/tools/githooks/install.sh) && bash -c "$b" install.sh
  --installed`, with that cwd as the process cwd — and only exit 0 counts. A failed read stops at `&&` (a pipe into
  `bash -s` would run an empty program, which exits 0 and would read as installed).

Then exit 0 and let the git hook decide. The `settings.json` entries stay.
**Blast radius**: these hooks run for every repository on this machine and protect lexime and others (incidents
lexime #332 / #350 / #354); outside elidex nothing changes, and inside elidex a worktree the shims do not reach keeps
the Claude hook. The edit is global and the PM's / user's (`~/.claude` CLAUDE.md "変更の blast radius": state it,
back up first); if those files are dotfiles-managed, it goes through the dotfiles session.

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

### §7.3 Needs a further user approval (not in the approved scope)

PM 893 (1) limits the approved change to `:24` and `:30`. The others, each with what it would become; the PM takes
them to the user:

| line | today | would become |
|---|---|---|
| `:13` | amend / force deny on flag text (inline jq) | the force half: the same elidex-scoped pass-through (pre-push II5 covers `-f` and `+<refspec>`); **the amend half stays everywhere** |
| `:36` | `gh-pr-merge-head-guard.sh` | as PLAN §1 row 6: in elidex, an adapter onto the same core — a raw `gh pr merge <N>` runs `merge-pr --dry-run <N>`'s checks and is denied when they refuse; other repos unchanged. **Proposal (for the PM to take to the user)**: in elidex, deny raw `gh pr merge` outright, naming `merge-pr`. Reason: with the adapter two merge procedures remain (IV5) and the raw one still needs `:42` / `:136` / `:142`; with the deny, `merge-pr` is the one way. If adopted, break-glass (§7.4) runs outside the Claude Bash tool, so no PostToolUse entry fires for it: its sync and cleanup are the PM's, by hand, as §7.4 says |
| `:42` | delete-branch worktree guard | in elidex, inert once raw merges are denied (`merge-pr` writes `--delete-branch=false`); kept as is under the adapter form |
| `:136`, `:142` | PostToolUse sync / cleanup | unchanged entries; the scripts stay (V4). Under the deny form they never fire for elidex merges; under the adapter form they keep serving raw merges. Changed only after `:36` (V3) |

### §7.4 Break-glass

Both kinds are the PM's hand action with a record; neither is an override flag, and neither is run by an agent.

**Merge (P1b, D32).** There is no override in `merge-pr`. When a merge must happen that `merge-pr` refuses —
including a broken `merge-pr` on `origin/main`, whose fix PR is merged this way — the PM runs, by hand,
`gh pr merge <N> -R send/elidex --squash --match-head-commit <H> --delete-branch=false`, and records it in
PORTFOLIO with a decision number and the reason. **The post-merge sync and cleanup depend on where that command
runs**, and are the PM's either way: run as a Claude Bash tool call, the PostToolUse entries (`:136`, `:142`) run
them as today; run in a terminal (or under the §7.3 deny form, which forces that), nothing runs them, and the PM
runs both scripts by hand from its own cwd with step 14's stdin:
`printf '{"tool_input":{"command":"gh pr merge <N>"},"cwd":"%s"}' "$(pwd -P)" | ~/.claude/hooks/post-merge-sync-main.sh`,
then the same into `post-merge-cleanup-branch.sh`.

**Push / commit (P1a, D39).** A broken pre-push, pre-commit or ratchet on `origin/main` rejects the push (or the
commit) of its own fix branch, and `--no-verify` is denied to agents (`git-hook-bypass-guard.py`). Then the PM, by
hand in a terminal (not through an agent), commits with `git commit --no-verify` if pre-commit is the broken part,
pushes the fix branch with `git push --no-verify origin <branch>`, and records it in PORTFOLIO with a decision
number and the reason. The fix PR then goes through the normal gate (review, CI, and `merge-pr` once P1b is live)
— only the hook run is skipped, nothing after it. The same applies to a PR that really removes or renames the
ratchet (F, "the branch deleted the ratchet").

### §7.5 E2E cases

Unsafe orders: A4 before A1 (no push guard — prevented anyway by `installed(wt)`); changing `:136` / `:142` before
`:36` (raw merges lose sync and cleanup); installing before A0 (the shim passes everywhere).

**Setup, once (P1a cases).** Every command below runs with `GIT_ALLOW_PROTOCOL=file` exported (git then refuses
every non-file transport, §3.2), so no case can reach GitHub. `git clone --local <shared repo> $S/repo`; check
`git -C $S/repo rev-parse --git-common-dir` is under `$S`. A `--local` clone's `origin/main` is the shared
repository's **local `main` branch**, so assert `git -C $S/repo rev-parse refs/remotes/origin/main` equals
`git -C <shared repo> rev-parse refs/remotes/origin/main` (else fast-forward the shared `main` first, or stop).
Point its `origin` at a fresh `git init --bare $S/remote.git` (the Claude shared-state guard asks for its sentinel
on this command even in a clone), then assert `git -C $S/repo remote get-url origin` is under `$S`. Push the clone's
`refs/remotes/origin/main` (P1a merged) to `$S/remote.git` as `main` and fetch, so `origin/main` holds the scripts.
`git -C $S/repo config extensions.worktreeConfig true` (a clone does not copy the shared repository's local config,
and E18 needs `--worktree`). `git worktree add -b wtb $S/wt`. Install with the §4.7 form from `$S/repo` —
`b=$(git show refs/remotes/origin/main:.claude/tools/githooks/install.sh) && bash -c "$b" install.sh` — not
`mise run` (the scratch clone is not mise-trusted, and must not be trusted from an unisolated mise state). Put a
real `rustfmt` and `python3` first on `PATH` (`~/.cargo/bin`, and the directory of `mise which python3` taken in a
trusted checkout). Write the push and
commit cases into a script file and run it with `bash`: from a Claude session, the Claude text hooks (lines 13, 24,
30) would otherwise deny the canonical bad commands before git runs, and the E2E would measure them, not the git
hooks. Run cases one at a time; `$S` is deleted at the end.

| # | slice | command (in `$S/wt` on branch `wtb` unless stated) | expected |
|---|---|---|---|
| E1 | P1a | `git push origin wtb` (new) | pushed |
| E2 | P1a | new commit, `git push origin wtb` (FF) | pushed; ratchet "ok" line on stderr |
| E3 | P1a | `git reset --hard HEAD~1 && git push origin +wtb` | rejected, FF rule; remote unchanged |
| E4 | P1a | `git push origin --delete wtb` | rejected, delete |
| E5 | P1a | `git push origin HEAD:other` | rejected, branch mismatch |
| E6 | P1a | `git branch side && git push origin side` | rejected, branch mismatch (the Claude hook would have allowed this) |
| E7 | P1a | `git checkout --detach && git commit --allow-empty -m d && git push origin HEAD:refs/heads/det` | pushed |
| E8 | P1a | in `$S/repo` on `main`: new commit, `git push origin main` | rejected, main |
| E9 | P1a | `git push origin wtb HEAD:other` (one good, one bad) | rejected; `wtb` on the remote unchanged |
| E10 | P1a | an untagged engine-name line in the working tree only, push | pushed (the pushed tree is clean); committed, push → rejected by the ratchet; with the tag added, pushed |
| E11 | P1a | on a branch cut from a commit before P1a: `git push origin +<that branch>` over an advanced remote | rejected, FF rule (`origin/main`'s rules judge it) |
| E12 | P1a | edit the worktree's `.claude/tools/githooks/pre-push` to `exit 0` (uncommitted), repeat E3 | still rejected |
| E13 | P1a | stage an unformatted `.rs`, `git commit -m x` | rejected, path listed |
| E14 | P1a | stage the formatted file, leave an unformatted working copy, commit | committed |
| E15 | P1a | real index holds an unformatted `b.rs`; `git commit -m x a.rs` with `a.rs` formatted | committed |
| E16 | P1a | E13 with `ELIDEX_FMT_OK=$(git write-tree)`; then with `ELIDEX_FMT_OK=1` | committed, note; then rejected |
| E17 | P1a | E13 with `PATH` not holding a working `rustfmt` (the untrusted mise shim) | committed, warning |
| E18 | P1a | in the scratch clone: `git -C $S/wt config --worktree core.hooksPath $S/alt`, then the §4.7 form with `--check` from `$S/repo` | lists `$S/wt`, exits 1; after `--unset`, exits 0 |
| E19 | P1a | after A1, in the shared repository, through the PM's own invocation: `mise run install-githooks -- --check` → exit 0. Negative control through the same invocation, from a script file that installs its cleanup first — `trap 'git worktree remove --force $S/neg' EXIT` — then `git worktree add --detach $S/neg origin/main`, `git -C $S/neg config --worktree core.hooksPath $S/alt`, `mise run install-githooks -- --check; rc=$?` from the main worktree → `rc` is 1 and the output names `$S/neg`; then the trap's removal and a re-run → exit 0; the script exits with an explicit status built from both captured codes | as stated; the negative control is the one sanctioned write of the key, undone by the trap on every exit |
| E20 | P1a | after A4, with `GIT_BYPASS_GUARD_EXTRA_SCOPE=$S` in the hook's environment (the bypass guard's test override, so `$S` counts as elidex): feed `{"tool_input":{"command":"git push origin HEAD:other"},"cwd":"$S/repo"}` to `git-push-branch-guard.sh`, and a staged-unformatted commit command to `cargo-fmt-precommit.sh`, in the installed scratch clone | no output from either (passed through; the git hook decides) |
| E21 | P1a | E20 with `cwd` = `$S/wt` after `git -C $S/wt config --worktree core.hooksPath $S/alt` (I4); and a compound `cd $S/repo && git push origin HEAD:x; cd $S/wt && git push origin HEAD:y` from `$S/repo` | deny from the Claude hook (as before A4) in both: one segment's worktree is not installed |
| E22 | P1a | E20 without `GIT_BYPASS_GUARD_EXTRA_SCOPE`, from a repository outside the scope (a scratch `git init` under `$S`, or a lexime worktree, read only) | deny, as before A4 |
| E23 | P1a | E20 after `git -C $S/repo update-ref refs/remotes/origin/main <a commit before P1a>` (the read of `install.sh` fails) | deny, as before A4 |
| E24 | P1a | after A2, in the shared repository: the §4.8 E2E line | as §4.8 states |
| M1 | P1b | `merge-pr <open PR>` with no token (`ELIDEX_PM_DIR=$S/pm`) | exit 1, "no go token"; PR still open |
| M2 | P1b | token for an older sha of that PR (in `$S/pm`) | exit 1, names both shas |
| M3 | P1b | token for head (in `$S/pm`), no dry Codex verdict on head after the last trigger | exit 1, names the Codex condition |
| M4 | P1b | token for head, `mergeStateStatus` not `CLEAN`, or a check pending | exit 1, names the state or the check |
| M5 | P1b | a copy of `merge-pr` saved under `$S` and run from there | exit 1, invocation site |
| M6 | P1b | `merge-pr --dry-run <PR>` on a PR the PM has approved (`ELIDEX_PM_DIR` = the real PM dir, token written) | exit 0, prints the `gh pr merge … --match-head-commit <H> --delete-branch=false` command; the PR is still open |
| M7 | P1b | the first real merge the PM approves after B3, run by the driver from its own PR worktree (cwd = the worktree its live session holds) | merged; sync and cleanup notes printed once; the PR's worktree is **left in place** and the cleanup note says so; no PostToolUse notes in the Claude transcript for that command |

M1–M6 stop before step 12 of §2 III, so they are safe on a real open PR. The head-moves race (III1) is not
reproducible by hand; it is covered by the test with the fake server.

## §8 Decided (formerly open questions)

No question is open. Each former question was decided by the driver (lens converged) or the PM; ground in one line.

- **Q0** PLAN.md is the ratified program memo (driver memo) — PM 873 (the user approved the whole Codex-lane plan;
  PM confirmed), PM 874 (ordered this per-PR memo); PLAN.md:3 records it. PLAN §6 decision 5 (Codex driver
  sandbox) was revised by PM 877/878; P1 does not depend on it except §7.2 step B1.
- **Q1** `ELIDEX_PM_DIR` is required; unset or a missing directory refuses (§2 III step 4). Ground: the merge is
  irreversible, so an unknown falls to refuse.
- **Q2** `merge-pr` refuses unless the running file is byte-identical to `origin/main`'s copy, after a fetch (step
  3). Ground: Security by structure — a PR must not be able to weaken its own merge gate.
- **Q3** Every CI check must be complete and green, besides `mergeStateStatus == CLEAN` (step 10). Ground: CLAUDE.md
  Workflow "CI 全 pass を目視確認してから squash merge", made mechanical. Unmeasured assumption kept: that pending
  or failed non-required checks give `UNSTABLE`; step 10 does not rely on it and §6 tests `CLEAN` + pending.
- **Q4** `--dry-run` runs every check and prints the merge command without running it (step 11); the PM's E2E uses
  it on a real PR (§7.5 M6). Ground: the good path needs a check that is not an irreversible act.
- **Q5** A tag moved forward passes II5: accepted as a stated limit, named in §1 non-goals. Ground: tags are not
  part of the PR / merge flow.
- **PM 893** (plan-review round 1; labels such as "(C5)" or "(E22)" name the items of that round's decision list,
  sections A–E, items 1–29): two slices, P1a first (D27); pre-commit as its own axis VI; cut-over limited to
  `:24` / `:30` as an elidex-scoped pass-through, the rest in §7.3; shim runs `origin/main`'s copy (D11); ratchet
  `--rev` (D28); the Codex requirement of step 8 and the `Completed` row departure (D30); no override, break-glass
  recorded (D32); `--check` standing (D33); the Codex execpolicy prerequisite (§7.2 B1); tests isolated by
  construction (T4, D20); invocation site (D34); `ELIDEX_PM_DIR` set by the PM's kickoff (D22); `CLAUDE_PROJECT_DIR`
  forwarded (D18); the full-sha token told to the PM (§7.2); both rollup shapes (D24); `ELIDEX_FMT_OK` per
  invocation (D29); token trust model (D36); Dependabot through `@codex review` (D35); merge queue refuses (D37).
- **PM 900 and the PM's A2-scope decision** (plan-review round 2): the user approved the bypass-guard extension; its
  scope is the whole write grammar of `core.hooksPath` plus `GIT_CONFIG_PARAMETERS`, with tests per form, negative
  controls and a mutant; the memo notes the existing `-c` / `--config-env` / `GIT_CONFIG_KEY_<n>` rules so the new
  rule does not overlap them silently; the PM applies it at P1a cut-over with the shim (§4.8, D38).
- **Round 2 driver decisions** (the open choices of the round-2 findings): A3-1 the raw-merge gap until `:36` is
  stated with owner and trigger, `:36` not pulled into scope (IV5, D40); A3-2 the P1a break-glass (§7.4, D39); A5-1
  the assessed-sha derivation and the no-sha item as a finding (step 8c, D30); A2-1 every `--rev` verdict input
  from `<sha>` (§4.5, D28); A2-2 `installed(wt)` by bytes, no earlier version, and the Codex sandbox caveat (§4.1,
  I4, D12); A2-3 the pass-through per git segment (§7.1, D41); the go token path kept, PLAN §6 decision 6 being its
  ground (D42).

## §9 Layering and what is not touched

No engine code. The scripts live under `.claude/tools/` next to the ratchet they call, and enter CI only through the
`tools` job's member glob. `~/.claude/**`, `~/.codex/**`, `.github/workflows/ci.yml` (apart from the
`timeout-minutes` re-derivation in §4.6) and every `.git/hooks` are untouched by these PRs (§4.8 is applied by the
PM, outside them). The one modified tool is `.claude/tools/ecs_mission_ratchet.py` (P1a, `--rev`; its working-tree
mode is unchanged).

**Two runtime dependencies cross the repository boundary**; each has a contract and an owner:
- **Tracked → global.** `merge-pr` (tracked) runs `~/.claude/hooks/post-merge-sync-main.sh` and
  `post-merge-cleanup-branch.sh` (global) with a hand-built PostToolUse payload (step 14: `.tool_input.command`,
  `.cwd`, the process cwd, `CLAUDE_PROJECT_DIR`). The contract is that interface (sync lines 18–28, cleanup lines
  32–47 and 70–79); its owner is the owner of `~/.claude/hooks` (PM / user, through the dotfiles session if those
  files are dotfiles-managed), who keeps it or changes `merge-pr` with it. `test_merge_pr.py` pins the payload
  `merge-pr` sends; a missing or failing script is a note, never a refusal (F).
- **Global → tracked.** After §7.1 A4 the global `:24` / `:30` hooks run `origin/main`'s `install.sh --installed`.
  The contract is §4.1's: exit 0 / 1, silent on 0, runnable in the §4.7 form from any worktree; its owner is this
  repository (the CI/tools lane), and `test_githooks.py` pins it. A change to it is a change to a global hook's
  input and is reviewed as such.

## §10 For the PM: PLAN.md corrections

PLAN.md is the PM's file; this section is addressed to the PM, who applies (or rejects) each item there. PLAN.md:3
already points here as the canonical errata; the items after the first eight are new since, so its count
"誤り 8 件" becomes fourteen.

Every cited line in PLAN §0, §1 rows 1, 2, 3, 5, 6, 7, 8, 17, §3, §5 P1, §6, §7 that points into this repository or
`~/.claude` was re-read at `origin/main` 3cbd5b88 (PLAN cites 1b727d18; `git diff --stat 1b727d18 3cbd5b88 --
CLAUDE.md .claude/skills/drive/SKILL.md .github/workflows/ci.yml` is empty, and `CL:27`, `CL:33`, `CL:57`,
`ci:271-279`, `drive:99-101`, `drive:127`, `set:13,24,30,36,42,136,142` and the cited hook line ranges say what
PLAN says). Re-checked at 6cbe39e3: `git diff --stat 3cbd5b88 6cbe39e3 -- CLAUDE.md .claude/skills/drive/SKILL.md
.github/workflows/ci.yml .claude/skills/pm/SKILL.md .claude/tools/ecs_mission_ratchet.py` is empty. Claims that are
wrong or incomplete:
- **§5 P1 completion** ("Claude session から既知の悪い push … が拒否され"): from a Claude session the Claude text
  hooks deny `-f` / `--force` (line 13) and `HEAD:<branch>` (line 24) before git runs, so that E2E would measure the
  Claude hooks. §7.5 runs the cases from a script file.
- **§1 row 2** ("元 script の規則を移す"): the Claude rule fires only on a `HEAD:<branch>` refspec in the command
  text (`git-push-branch-guard.sh` lines 22–24). pre-push sees every ref, so the rule becomes wider (E6), not moved.
- **§1 row 3** ("元 script と同じ範囲"): the file set is the same, the content is not. The Claude hook checks the
  working-tree file (`cargo-fmt-precommit.sh` lines 46–48) of the paths in the real index; for `git commit <path>`
  and `git commit -a` git commits from a temporary index (measured), which only a git hook sees. Its
  `--diff-filter=ACM` also misses renames.
- **§1 row 8** ("1 実装・2 呼び出し元", JSON on stdin): the JSON is not the whole interface. Both scripts find the
  main worktree with `git worktree list` from the *process* cwd (sync line 25, cleanup line 39); cleanup also reads
  `CLAUDE_PROJECT_DIR` (line 71) and `lsof` (line 83).
- **§1 row 1**: the existing force guard (line 13) matches flag text only; a `+<refspec>` force push passes it.
  Not wrong in PLAN, but it is a gap of today's layer that the git hook closes.
- **§1 preamble / §6 item 2** ("全 worktree … に効く"): a worktree can redirect its hooks path with
  `git config --worktree core.hooksPath` (`extensions.worktreeConfig` is `true` in the shared repository); in the
  probe, the Claude bypass guard denied `-c core.hooksPath` but not that command. The installer reports it (I4).
- **§1 row 5**: the ratchet as it stands reads the working tree, not the pushed commit; P1a adds `--rev` so pre-push
  judges the pushed commit (II9).
- `$PMD` is used throughout PLAN but not defined in it; a tracked script needs it from the environment (Q1); the PM
  sets it in each kickoff (§4.4).
- **§5 P1** ("1 PR"): two slices, P1a (git hooks) then P1b (merge wrapper) (PM 893).
- **§1 preamble** ("shim は script が無い古い branch では通す"): the shim runs `origin/main`'s copy, so old branches
  are guarded too (PM 893; I2).
- **§0.1** ("admin は bypass する"): the ruleset's admin bypass was removed on 11-21 (PM).
- **§5 P1 completion and §6 item 3** ("`set:24,30` の Claude hook 撤去"): an elidex-scoped pass-through instead of
  removal (PM 893; §7.1 A4); `set:13,36,42,136,142` need a further user approval (§7.3).
- **§1 row 6** (wrapper: "Codex 評価 head == head"): a dry verdict on the head after the last trigger with 0
  unresolved Codex threads; the summary row's `Completed` is not an assessment (PM 893; §2 III step 8).
- **§6 item 5** (Codex sandbox: add-dir の `.git` common dir): a session that can write the common dir can rewrite a
  shim or a worktree's `config.worktree` by a file write, which no command adapter sees; it is detected by
  `--check` (bytes and hooks path, §4.1), not prevented (I4).

## §11 Provenance

Commands behind the facts above (run 2026-11-21; probes in scratch repositories — a `git clone --local` with origin
re-pointed to a scratch bare repository, and `git init` repositories with `HOME`, `GIT_CONFIG_GLOBAL=/dev/null`,
`GIT_CONFIG_NOSYSTEM=1` — deleted afterwards):
- `git --version` → 2.56.1; `gh --version` → 2.105.0; `~/.cargo/bin/rustfmt --version` → 1.10.0.
- `git config --show-origin --show-scope --get-all core.hooksPath`; `git config --get extensions.worktreeConfig`.
- pre-push / pre-commit recording hooks (print args, cwd, `GIT_*`, HEAD, stdin / `git diff --cached --name-only`)
  over: new branch, up-to-date, FF, `HEAD:other`, two refs, `--delete`, two tags, detached `HEAD:refs/heads/det`, a
  hook exiting 1; commits with a pathspec and with `-a`.
- `git merge-base --is-ancestor` on ancestor / non-ancestor / unknown sha / all-zero sha / two tag objects.
- `git config --worktree core.hooksPath <dir>` then `git rev-parse --git-path hooks` in that worktree and in another.
- `git rev-parse --path-format=absolute --git-path hooks` in a fresh `git init` repository and its linked worktree,
  with `core.hooksPath` unset and set to `<common dir>/hooks`.
- `GIT_ALLOW_PROTOCOL=file GIT_SSH_COMMAND=false git push` to `ssh://127.0.0.1/x`, to a local path and to `file://`.
- rustfmt `--check` on stdin vs on a file, and format-and-compare, on a bad, a good and an unparsable input;
  `rustfmt --config-path <dir>` and `<dir>/.rustfmt.toml` on `async fn f() {}` from stdin, and without a config;
  `rustfmt --help`.
- `python3 -c ''` and `rustfmt --version` in an untrusted scratch checkout; `__file__` under `python3 -` and
  `python3 -c`.
- `/usr/bin/time -p python3 .claude/tools/ecs_mission_ratchet.py` in this worktree.
- `git help githooks` (pre-merge-commit), `git help push` (`--no-verify`), `git help merge-base` (OPERATION MODES).
- `gh pr view --json` field list (no merge-queue field: hence the GraphQL query of step 6).
- Round 2 (2026-11-21): the probe scripts of §3.2's round-2 list, in `git init` repositories under the session
  scratchpad (`HOME`, `GIT_CONFIG_GLOBAL=/dev/null`, `GIT_CONFIG_NOSYSTEM=1`; deleted afterwards): each `git config`
  form followed by `git -C <linked wt> rev-parse --path-format=absolute --git-path hooks`; `GIT_CONFIG_PARAMETERS`
  and `-c include.path=` on `rev-parse --git-path hooks`; `for-each-ref` / `ls-tree` / `cat-file -e` on an absent
  ref and path; a script that runs `git merge --ff-only` over itself, with `os.stat().st_ino` before and after;
  `read-tree` + `checkout-index -a --prefix=` into a temp index, then `git diff --cached` and the two path lists;
  the memo's shim body extracted and run with and without `origin/main` holding a hook.
- `git help -m config` / `read-tree` / `checkout-index` / `ls-tree` / `for-each-ref` / `git` section headings
  (`col -b`, `^[A-Z][A-Z -]+$`).
- `git log --follow -- .claude/tools/ecs_mission_ratchet.py` (69fbe61f, 1b727d18) and, per commit, the `def main`,
  `def repo_root_of`, `def tracked_files` and `DEFAULT_ALLOWLIST` lines; `git ls-tree -r -l HEAD` (1957 blobs,
  42.9 MB at fca1f6cd).
- §4.8: the hook and its test copied into the scratchpad, patched, `python3 git-hook-bypass-guard.test.py` (patched:
  OK, 14 tests; with `GIT_BYPASS_GUARD_SCRIPT` = the unpatched file: 39 subtest failures), and the nine mutants
  through `GIT_BYPASS_GUARD_SCRIPT` (all killed); `diff -u` of the two hook files is the block in §4.8.
- `~/.claude/hooks/gh-pr-merge-head-guard.sh` header lines 1–19; `.claude/skills/external-converge/project.md`
  § reviewer and `external-review/project.md` § reviewer; `~/.claude/skills/external-review/SKILL.md` Step 1
  resolver; `.claude/skills/pm/SKILL.md:222-224`; `~/.codex/rules/default.rules` (`prefix_rule(pattern=[…])`);
  `.github/workflows/ci.yml:191-194`, `:275-278` at `origin/main` 6cbe39e3; PLAN.md §6 item 6.
