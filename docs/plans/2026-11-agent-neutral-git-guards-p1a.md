# Plan — P1a: agent-neutral git hooks (and what P1a and P1b share)

## §0 Status

**Pair (PM 908)**: the P1 plan is two memos, one per PR. This one is **P1a**, and it is also the one home of
everything the two slices share (the program and scope of §0, the goal and non-goals of §1, the axes of §2, the
sources, breadth and measurements of §3, the rung of §5, the E2E sites and setup of §7 / §7.5, §7.3, the records
of §8, §9, §10 and §11). The P1b slice is `docs/plans/2026-11-agent-neutral-git-guards-p1b.md` (the **P1b memo**), which refers
here instead of restating. Section numbers are those of the single memo the pair was split from, so a section or
table row absent here is in the P1b memo, and a reference into it names that memo. Ids (the items of the axes
I–VI, F and T, D1–D42, Q0–Q5, E1–E24, M1–M7, the steps A0–A3 and B0–B3) are unique across the pair; an id not
defined in one memo is defined in the other. A retired id is not reused: PM 950 retired D41, V6, A4 and E20–E23.
§4.8 (the bypass-guard patch) is a third file since PM 916,
`docs/plans/2026-11-agent-neutral-git-guards-p1a-bypass-guard.md` (the **bypass-guard memo**); it keeps its section
number, and a reference to it here names that memo.

**Program**: the Codex-lane program. Its driver memo lives **outside this repository**:
`/Users/kazuaki/repos/send.sh/elidex-pm-trial/handoff/codex-lanes/PLAN.md` (PM 872). It is the ratified program
memo (its line 3: ratified by the user, PM 873; it points at §10 of this memo for its errata), and PM 874 ordered
this per-PR memo. This memo and the P1b memo are the plan for its phase **P1** (PLAN §5 P1, rows 1, 2, 3, 5, 6, 7, 8 and 17 of
PLAN §1).
**Scope source**: PM 873 / 874, restructured by PM 893; plan-review round 2 added PM 900 (the bypass-guard extension
of bypass-guard memo §4.8, approved by the user) and the PM's A2-scope decision; PM 950 (user, option D) withdrew
the one change of a Claude hook that scope held, for `:24` / `:30`. The scope is not re-opened here; §8 records the
decisions.
**Slices (PM 893, overturning PM 874's "single PR")**: two PRs, one memo each since PM 908, **P1a first**.
- **P1a — git hooks**: `pre-push`, `pre-commit`, the shim, the installer (`install-githooks`, `--check`), the
  ratchet's `--rev` mode, their tests, and the one-row `--check` standing item in the PM skill (D33, §4.9). Cut-over §7.1,
  which also applies the global bypass-guard patch this memo carries (bypass-guard memo §4.8; the PM applies it, it is not in the PR).
- **P1b — merge wrapper**: `merge-pr` and its tests. Cut-over P1b memo §7.2.
- **The seam**: this memo's own II × III intersection is empty — `merge-pr` pushes nothing, so no pre-push
  decision meets a merge decision (§2 pair table). The two slices share only the test-isolation helper (T4), which
  P1a adds and P1b imports. §2 is split by slice tag: each invariant is tagged **[P1a]**, **[P1b]** or **[both]**;
  the [P1a] and [both] items are here, the [P1b] items in the P1b memo §2.
**Nature**: repo tooling. New tracked files under `.claude/tools/githooks/` and `.claude/tools/agent/`, their
tests, one `mise` task; one modified tool (`.claude/tools/ecs_mission_ratchet.py` gains `--rev`, with its test); one
row in the §2 PM-pass table of `.claude/skills/pm/SKILL.md` (§4.9). Zero `crates/**` diff. Nothing under
`~/.claude` is edited by the PRs (bypass-guard memo §4.8 is a patch the PM applies at cut-over), and nothing is
installed into any `.git/hooks`:
installing the shim and applying that patch is the **cut-over**, done by the PM / user after each merge (§7). P1a's
cut-over changes no `settings.json` entry and no other Claude hook (PM 950).
**Edge-dense**: yes — six intersecting axes (§2). So `/elidex-plan-review` is mandatory before implementation
(CLAUDE.md Design discipline, "Edge-dense work"). P1a and P1b are each a terminal per-PR slice under the ratified
program memo (decided Q0 in §8); each gets its own full review.

## §1 Goal and non-goals

**Goal.** The guards that today exist only as Claude Code PreToolUse / PostToolUse hooks, and that decide on the
*text* of a Bash command, get an implementation in a layer every agent and every human goes through: git's own hooks
(which see what git is about to do) and one merge wrapper (which is the merge). A Codex driver then gets the same
guard as a Claude driver, from one implementation (CLAUDE.md "One issue, one way"). The Claude Bash hooks for push
branch and fmt (`settings.json` `:24`, `:30`) keep running beside the git hooks (PM 950; IV5).

Files (all (NEW) unless marked):

| slice | file | what it does |
|---|---|---|
| P1a | `.claude/tools/githooks/pre-push` | per pushed ref: reject non-fast-forward (force or delete), reject a target branch that is not the current branch (detached HEAD exempt), reject any push to `refs/heads/main`; then run the ECS-mission ratchet on each pushed commit (`--rev`) |
| P1a | `.claude/tools/githooks/pre-commit` | rustfmt check of the staged `.rs` blobs, with the staged `.rustfmt.toml`; per-invocation opt-out naming the staged tree |
| P1a | `.claude/tools/githooks/shim.sh` | the body the installer writes into the hooks directory, once per hook name; runs **`origin/main`'s** copy of the hook script; passes with a note while `origin/main` has none (absence is an exit-0 empty answer; an error rejects, I3) |
| P1a | `.claude/tools/githooks/install.sh` | the installer; always run as **`origin/main`'s** copy, read into a variable and run with `bash -c` (§4.7); `--check` reports any worktree the shims do not reach |
| P1a | `.claude/tools/githooks/isolated_git.py` | test helper (not a member): the isolated environment of T4 and its refusal on the real repository; imported by every test below |
| P1a | `.claude/tools/githooks/test_githooks.py` | control + mutant tests for both hooks, the shim and the installer (split by scenario if it nears 1000 lines) |
| P1a | `.claude/tools/ecs_mission_ratchet.py` (MODIFIED) | `--rev <sha>`: materialise `<sha>`'s tree in a temp dir and run **`<sha>`'s own ratchet module** on it; the running copy only orchestrates (§4.5) |
| P1a | `.claude/tools/test_ecs_mission_ratchet.py` (MODIFIED) | control + mutant for `--rev` (695 lines today; the addition keeps it under 1000) |
| P1a | `mise.toml` `[tasks.install-githooks]` (MODIFIED) | runs `origin/main`'s `install.sh` with `bash -c` after a read that must succeed (§4.7); not part of `[tasks.ci]` |
| P1a | `.claude/skills/pm/SKILL.md` (MODIFIED) | one row in the §2 PM-pass table, the text of §4.9: `mise run install-githooks -- --check` in every PM pass from shim install on (D33) — in P1a, not P3b, so the standing item is in the PM procedure before anything is installed |

P1b's files (`merge-pr`, `test_merge_pr.py`): P1b memo §1.

**Non-goals.**
- Installing the shim into the shared repository, or changing any Claude hook (cut-over, §7, PM / user after merge).
- The Codex PreToolUse / Stop adapters (PLAN rows 4, 9, 11 = P2).
- **The repo docs that name the old procedure — one list (this is the only copy; §7 points here). Items 1 and 7 are done by #663 (1c50c3d0); items 2–6 are P3b:**
  1. DONE by #663: `.claude/skills/drive/codex.md` (PLAN §4). Its interim clauses are rewritten or removed by P1a (§7.1 A0) and P1b (P1b memo §7.2 B0, B3).
  2. P3b: `CLAUDE.md` Workflow "Git" line (`CLAUDE.md:54` at 6cbe39e3): "CI 全 pass を目視確認してから squash merge
     (… この目視が最終ゲート)" — the check becomes `merge-pr` step 10; the line names `merge-pr` instead.
  3. P3b: `CLAUDE.md` Workflow worktree line (`CLAUDE.md:55`): names `~/.claude/hooks/git-push-branch-guard.sh` as what
     blocks a branch-mismatch push — names pre-push, which decides for every agent (the Claude hook stays beside it, PM 950).
  4. P3b: `.github/workflows/ci.yml` `trip-wires` job comment (`ci.yml:191-194`) and `tools` job comment
     (`ci.yml:275-278`): "merging still rests on the CLAUDE.md convention of eyeballing CI" — becomes `merge-pr`.
  5. P3b: `.claude/skills/drive/SKILL.md` §6 step 4 (its raw `gh pr merge … --match-head-commit <sha>`) — becomes the
     `merge-pr` invocation of P1b memo §4.4 (with `ELIDEX_PM_DIR`).
  6. P3b: `.claude/skills/pm/SKILL.md` §4: the go token (file name with the full 40-hex sha, P1b memo §7.2), PM-merged PRs
     ("PM が起こした PR も、同じ確認の後に PM が同じコマンドで merge する") through `merge-pr`, and Dependabot PRs
     ("Dependabot の PR も CI/tools lane が実行する (PM 240)") through `merge-pr` after the CI/tools lane posts
     `@codex review` (D35). The `--check` standing item is P1a's row
     (§4.9), not P3b's.
  7. DONE by #663: `.claude/skills/pre-push/SKILL.md`, the one-line reference for Codex drivers (PLAN §5 P3).
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
  - **Prevention, for the direct commands the adapter reads = the agent adapter layer.** Claude: the global `git-hook-bypass-guard.py`
    gains the patch of bypass-guard memo §4.8 (PM 900, approved by the user; scope = the whole write grammar of the key, PM's A2-scope
    decision). Read on 2026-11-21: it already denies `-c core.hooksPath=…` / `-c core.hooksPath …` /
    `-ccore.hooksPath=…`, `--config-env=core.hooksPath=…`, a `GIT_CONFIG_KEY_<n>=core.hooksPath` assignment and
    `--no-verify` (its docstring lines 4–14); it has no rule for `git config … core.hooksPath` nor for
    `GIT_CONFIG_PARAMETERS`, both of which reroute (measured, §3.2). Codex: PLAN row 4's adapter (P2). Both are
    global changes applied by the PM / user, not by this PR (the Claude one at §7.1 step A2). A command
    adapter sees commands only: indirect forms (`include.path` pointing at a file that sets the key, a direct file
    write into `config.worktree`) are not prevented, nor are the shapes whose cwd the Claude guard misreads (bypass-guard
    memo §4.8, "`scan()`'s deny is not widened; its blind spots"). `--check` detects the persistent ones of both kinds.
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
- II2 [P1a]. HEAD attached, its full ref `R` (`git symbolic-ref -q HEAD`, e.g. `refs/heads/B`; not `--short`, which
  answers `heads/B` when a tag `B` exists, §3.2, PM 989), `remote-ref` under `refs/heads/` and `!= R` → reject. Keyed on
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

**III — merge decision order and TOCTOU** [P1b]: P1b memo §2 (steps 1–14, exit codes, III1–III5).

**IV — coexistence with the Claude hooks** [both] (shim installed; the Claude hooks beside it).
- IV1 [P1a]. On a Claude `git push` both the Claude PreToolUse hooks (`settings.json` lines 13 and 24) and pre-push
  decide; the push proceeds only if all of them allow. Where they differ: the Claude branch guard fires only on a
  `HEAD:<branch>` refspec in the command text (`git-push-branch-guard.sh` lines 22–24), pre-push on every ref
  (wider); the Claude force guard denies the *flag* text (`-f`, `--force`, `--force-with-lease`, line 13) even for a
  fast-forward, and does not match a `+<refspec>` force, while pre-push decides on the ref shape. No cut-over step
  changes line 13 or line 24 (PM 950, PM 964; IV5).
- IV2 [P1a]. On a Claude `git commit` the Claude fmt hook (line 30) checks the working-tree file of each path the
  *real* index lists; pre-commit checks the blobs of the index git commits from (VI1). They differ on partial
  staging and on `git commit <path>` / `git commit -a`. A deliberate skip from a Claude session needs both opt-outs
  (`# fmt-ok` and VI3's value).
- IV3, IV4 [P1b]: P1b memo §2.
- IV5 [both]. **Three coexistences this scope does not end** (PM 893 (1), PM 950, PM 964). This is the strangler state "One
  issue, one way" forbids, so the memo names them instead of hiding them.
  - **Push branch and fmt (`:24`, `:30`).** Kept by decision (PM 950, user, option D): both Claude hooks run on every
    command as today, and from A1 pre-push / pre-commit decide too; the stricter answer wins (IV1, IV2). No trigger
    and no end in this scope.
  - **Merge (the stated gap).** Until the `:36` approval (§7.3) a Claude driver can still run a raw
    `gh pr merge`. Its only guard is `gh-pr-merge-head-guard.sh`, which allows a PR with no Codex assessment and
    fails open on any `gh` error (its header, lines 14–16), counts the summary row's `Completed` (its lexime #332
    branch), honours `# merge-stale-ok` (header lines 18–19), and asks for no go token. So on that path `merge-pr` step 8's
    "no override to train onto" and V × F's "a fail-closed layer on merge" do **not** hold; they hold for the whole
    merge path only after `:36`. Owner: the PM, who carries `:36` to the user. Trigger: P1b's merge (from then on
    `merge-pr` exists and the raw path is the second procedure). Not pulled into this scope.
  - **Force push.** The Claude force guard (`:13`) denies the flag text (`-f`, `--force`,
    `--force-with-lease`), also on a fast-forward, and does not see a `+<refspec>`; pre-push II5 decides on the ref
    shape. Both decide, so the stricter answer wins (IV1). Kept by decision (PM 964): no defect against the spec or
    the elidex principles has been shown for `:13` in this scope, so both halves stay everywhere as today, and from
    §7.1 step A1 pre-push II5 decides beside the force half. No end in this scope; a defect found later goes to the PM
    as a slot draft at that time.

**V — cut-over order** [both] (§7 has the steps and what is safe in each intermediate state).
- V1 [P1a]. Nothing is installed before P1a is merged (an installed shim would pass everywhere: I3).
- V2 [P1a]. No Claude hook stops deciding in P1a's cut-over: `:13`, `:24` and `:30` run on every command as today,
  and the git hooks are added on top (PM 950).
- V3, V4 [P1b]: P1b memo §2.
- V5 [P1a]. The `--check` PM-pass item runs from shim install on (A1), so a persistent reroute (I4) is detected from
  the first installed day; the adapter-layer prevention of I4 (the bypass-guard memo §4.8 patch: prevented for the
  shapes its `scan()` reads, that memo) follows at A2. No later step depends on either.

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

The P1b rows (`merge-pr`): P1b memo §2.

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
| both | I × IV | in a worktree with I4's override the git hooks do not run, so the Claude hooks decide alone there, as before P1a, and `--check` lists the worktree; in other repositories nothing changes (P1a changes no Claude hook but the bypass guard, whose rules are elidex-scoped) |
| P1a | I × V | the installer writes one file per hook name into the shared hooks dir, so installation is all-worktrees-at-once; there is no per-worktree rollout |
| P1a | I × VI | pre-commit, like pre-push, is `origin/main`'s copy; the staged config it reads (VI4) is the branch's, so the rule is shared and the edition is the commit's |
| P1a | I × F | I3 is the one fail-open on the reject-side checks; it is taken only on an absence git reported with exit 0, never on an error |
| both | I × T | tests build their own repositories and bare remotes inside a temp dir; T4 refuses the real repository |
| — | II × III | **empty — the slice seam.** `merge-pr` pushes nothing, so pre-push never sees a merge; the only push-shaped event in a merge is server-side |
| P1a | II × IV | double decisions are conjunctive, so a Claude push needs both layers to allow; IV1 lists where they disagree |
| P1a | II × V | no Claude push guard stops deciding at any step (`:13`, `:24` unchanged, V2), so the II rules are added to them from A1 on |
| P1a | II × VI | the ratchet at push reads the pushed commit (II9) and pre-commit reads the index (VI1): neither reads the working tree, so what was committed is what is judged at push |
| P1a | II × F | every ref rule fails closed; only the ratchet's environment error fails open, and it runs only after every ref rule has passed |
| P1a | II × T | each II case is a real `git push` into a local bare repository, so the stdin the hook reads is git's own, not a fixture |
| both | IV × V | IV is the state from §7.1 step A1 on for push / commit (no end in this scope, IV5), and between P1b memo §7.2 step B3 and the §7.3 approvals for merge |
| both | IV × VI | IV2: the two commit checks differ on partial staging and temporary indexes |
| both | IV × F | in IV the stricter answer wins, so a fail-open on one layer is covered by the other where both look |
| both | IV × T | the tests cannot run the Claude hooks (they live outside the repository), and no E case targets IV: E1–E18 run their push and commit cases from a script file so that the Claude hooks do not see them (§7.5 setup), E19 is `--check`, and E24 is the bypass guard. Under PM 950 IV is the Claude hooks' unchanged behaviour plus the git hooks, observed in real use at §7.1 step A3 (both layers deciding) |
| P1a | V × VI | pre-commit decides from A1 on; `:30` is unchanged at every step (V2) |
| both | V × F | each intermediate state of §7 keeps at least one fail-closed layer on push and on commit; on merge only after the `:36` approval — before it, the raw `gh pr merge` path has the fail-open head guard only (IV5, the stated gap) |
| both | V × T | the PRs' tests pass with nothing installed; installation is checked by the E2E, never by CI |
| P1a | VI × F | the only fail-open in VI is rustfmt not running, behind CI `check`; a missing config fails closed |
| P1a | VI × T | each VI case is a real `git commit` (with a pathspec, `-a`, partial staging) in a scratch repository, with the stub rustfmt |
| both | F × T | each fail direction is a test case: the stub rustfmt and a fake `python3` that cannot start cover the warn-and-pass paths, fake `gh` errors cover refusal |

The pairs with III (I × III, III × IV, III × V, III × VI, III × F, III × T): P1b memo §2.

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
| git §ENVIRONMENT VARIABLES (git(1); the temporary index measured, not in githooks(5)) | pre-commit index | `GIT_INDEX_FILE` names the index being committed; a temporary index for `commit <path>` / `commit -a` | pre-commit VI1 reads `git diff --cached` / `git show :<path>` through it | measured only: githooks(5) does not describe the temporary index | yes |
| git §HOOKS (githooks(5)) | pre-merge-commit | an automatic merge runs pre-merge-commit; a conflicted merge concluded with `git commit` runs pre-commit (last paragraph) | VI5: automatic merges unchecked (backstop CI `check`); conflict resolutions checked | ✓ | yes |
| git §OPERATION MODES (git-merge-base(1)) | `--is-ancestor` | exit 0 / 1 / other | pre-push II4, II5 | measured: 128 for an unknown or all-zero sha | yes |
| git §OPTIONS (git-rev-parse(1)) | `--git-path hooks`, `--git-common-dir`, `--path-format=absolute` | hooks path honours `core.hooksPath`, including a worktree-level value | `installed(wt)` (§4.1), installer | measured: key unset → `<common dir>/hooks` from the main and a linked worktree; `--worktree` value → that value | no |
| git §ENVIRONMENT VARIABLES (git(1)) | `GIT_ALLOW_PROTOCOL` | colon-separated allow-list of transports | T4 (`file` only) | measured: `ssh` refused with exit 128; local path and `file://` allowed | no |
| git §OPTIONS (git-read-tree(1), git-checkout-index(1)); §OUTPUT FORMAT (git-ls-tree(1)) | `read-tree <sha>` into a temp `GIT_INDEX_FILE`, `checkout-index -a --prefix=<dir>/`; `ls-tree -r -z --name-only <sha>` | materialise `<sha>`'s tree; list its tracked paths | ratchet `--rev` (§4.5) | measured: the real index is untouched, symlinks are written as symlinks, and the path list equals `ls-files` of a checkout of `<sha>` | no |
| git §FIELD NAMES (git-for-each-ref(1)); §OUTPUT FORMAT (git-ls-tree(1)) | `for-each-ref --format=…`, `ls-tree <commit> -- <path>` | an absent ref or path prints nothing | shim absence vs error (I3) | measured: absent ref / path → no output, exit 0; `ls-tree` on an absent ref → exit 128 | no |
| git §COMMANDS, §DEPRECATED MODES, §OPTIONS (git-config(1)) | writes of `core.hooksPath` | `set` / `unset` / `rename-section` / `remove-section` / `edit`, and the legacy `<name> <value>`, `--add`, `--replace-all`, `--unset[-all]`, `--rename-section`, `--remove-section`, `-e`; `--local` / `--worktree` / `--global` / `--system` / `-f` / `--file`; variable names case-insensitive (§CONFIGURATION FILE) | bypass-guard memo §4.8 patch (global, PM) | measured: every form reroutes `--git-path hooks` or puts it back, options also after the positionals | yes |
| git §ENVIRONMENT (git-config(1)) | `GIT_CONFIG_COUNT` / `GIT_CONFIG_KEY_<n>`; `GIT_CONFIG_PARAMETERS` | per-process configuration | bypass-guard memo §4.8 patch (`GIT_CONFIG_PARAMETERS` new; `GIT_CONFIG_KEY_<n>` already denied) | `GIT_CONFIG_PARAMETERS` is in neither git(1) nor git-config(1): measured, `'core.hooksPath'='<d>'` and `'core.hookspath=<d>'` reroute one command | yes |
| git §CONFIGURATION FILE (git-config(1), "Includes") | `include.path` | an included file's `core.hooksPath` applies | I4 detection (`--check`), bypass-guard memo §4.8 blind spot | measured: a `--worktree include.path` and a `-c include.path=` both reroute; `--git-path hooks` follows the include | yes |
| rustfmt §OPTIONS (`rustfmt --help`, 1.10.0) | `--check` | the help says it exits 1 when formatting is required | **not used on stdin**: measured exit 0 with a diff on stdin (§3.2) — the help says the opposite of the stdin behaviour | measured | yes |
| rustfmt §OPTIONS (`rustfmt --help`, 1.10.0) | `--config-path` | "Recursively searches the given path for the rustfmt.toml config file" | VI4: a temp dir holding the staged `.rustfmt.toml` | measured: finds `.rustfmt.toml` in the dir on stdin input | yes |

The gh and GitHub GraphQL rows: P1b memo §3.

**Breadth**: K=4 (git, gh, rustfmt, GitHub GraphQL), M=30. The preflight's SPLIT-DEFAULT (K≥6 or M≥30; reached by
M=30 since round 2's rows) is answered by the two slices and the separate global patch — no single PR carries more
than 19 rows: P1a holds 17 git and 2 rustfmt rows (K=2, M=19), the bypass-guard memo §4.8
patch (applied by the PM, in neither PR's diff) the 3 `git-config(1)` rows (K=1, M=3), P1b the 3 gh and 5 GitHub
GraphQL rows (K=2, M=8).

### §3.1 User-input touch audit

No web-content input. The inputs are what an agent or a person types: refspecs and flags of `git push`, the index
of `git commit`, the PR number given to `merge-pr`, and files under the PM dir. Each one reaches a decision only
through git's own report (pre-push stdin, the index) or a `gh` response, never by parsing the command text — that is
the point of the git-hook layer (IV1, IV2).

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
- (Codex #677 round 3, 2026-11-23, git 2.56.1) On branch `topic` with a tag `topic`: `git symbolic-ref --short -q
  HEAD` → `heads/topic`, `git symbolic-ref -q HEAD` → `refs/heads/topic` (detached: empty, exit 1); `git push origin
  HEAD` feeds pre-push `HEAD <sha> refs/heads/topic <zeros>`, and `git push origin topic` fails ("matches more than one").
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

**`installed(wt)` — the one definition** (I4, `--check` and §6 refer here). Let
`D` = `$(git rev-parse --path-format=absolute --git-common-dir)/hooks`, resolved with `cd … && pwd -P`.
`installed(wt)` is true iff `git -C <wt> rev-parse --path-format=absolute --git-path hooks`, resolved the same way,
equals `D`, and `D/pre-push` and `D/pre-commit` exist, are executable, and are **byte-identical to `origin/main`'s
`shim.sh`** (`git cat-file blob <origin/main commit>:.claude/tools/githooks/shim.sh`, compared with `cmp`). The
bytes decide, not a marker line: a shim edited in place, an older shim body, or a foreign file with a copied marker
is not installed. There is no earlier shim version to accept (nothing has been installed before P1a; no backward
compatibility). A hooks path that cannot be resolved, or a `shim.sh` that cannot be read from `origin/main`, makes it
false. A fresh repository with `core.hooksPath` unset and the shims in `.git/hooks` satisfies it (§3.2).
**Which copy evaluates it**: always `origin/main`'s `install.sh`, run in the §4.7 form — by the `mise` task (§4.7)
and by `--check`. No caller runs a working-tree `install.sh`, so there is one predicate, not one per checkout.
**Re-install when the shim changes**: a PR that changes `.claude/tools/githooks/shim.sh` writes a re-run of
`mise run install-githooks` into its own cut-over; the PM confirms it landed (PM 921).

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
In both modes `install.sh` reads nothing relative to `$0` (it runs as `bash -c` from a variable) and takes the shim
bytes from `origin/main`, not from the working tree.

### §4.2 pre-push [P1a]

As §2 II1–II9, in that order per line; all reasons are collected and printed to stderr with the rule's name and the
remedy (FF: "merge, do not rebase or force"; branch: "push from the worktree that has the branch checked out";
main: "open a PR"). Current branch: `git symbolic-ref -q HEAD`, the full ref compared as is with the remote ref
(empty = detached; never `--short`, II2). The ratchet is run as in
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

P1b memo §4.4.

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
- `test_merge_pr.py` [P1b]: P1b memo §4.6.
- `isolated_git.py` [P1a, imported by P1b]: T4; loaded by path (`importlib.util.spec_from_file_location`, as
  `test_ecs_mission_ratchet.py` does).
- Mutants: each mutant is a one-line text replacement in a temp copy of the script. The harness asserts the
  replaced text occurs exactly once before applying it (so a later edit of the script cannot turn a mutant into a
  no-op), runs the case named in §6 (or P1b memo §6) against the mutated copy, and asserts the case's verdict flips.
- Each PR re-derives the `tools` job's `timeout-minutes` from its own run, per the rule in that job's comment.

### §4.7 mise task [P1a]

```toml
[tasks.install-githooks]
description = "Install the githook shims into this repository's hooks directory (all worktrees). Not part of ci."
run = 'b=$(git show refs/remotes/origin/main:.claude/tools/githooks/install.sh) && bash -c "$b" install.sh'
```

`mise run install-githooks -- --check` appends `--check`, which reaches the script as `$1`. The `&&` makes a failed
read fail the task: a pipe into `bash -s` would run an empty program on a failed read, and an empty program exits 0.

### §4.8 The bypass-guard patch (global; the PM applies it at §7.1 A2) [P1a cut-over]

Bypass-guard memo §4.8 (PM 916); this memo's §7.1 step A2 applies it.

### §4.9 The PM-skill row [P1a]

P1a's diff adds one row to the PM-pass table of `.claude/skills/pm/SKILL.md` §2 ("PM pass (毎回この順、出力は表)"),
after its last row: the standing item of D33 (PM 908: it is in P1a's diff, not P3b's). It goes in §2, not in §4
("Merge 検証"), because §4 runs once per merge and the item runs in every pass. Before A1 the row has a value, not a
`skipped:` (the skill's skip limit would otherwise escalate it every pass until A1). The row, with `<n>` = the next
row number:

```md
| <n> | **githooks の到達** (常設、D33) | A1 (shim の install) 以降: `cd /Users/kazuaki/repos/send.sh/elidex && mise run install-githooks -- --check`。exit 0 → 値 `githooks: installed in every worktree`。exit 1 → 出力が名指す worktree を表に写し、その worktree の lane に理由 (hooks path の迂回か shim の書き換え、`docs/plans/2026-11-agent-neutral-git-guards-p1a.md` §4.1 の `installed(wt)`) を聞く。worktree を名指さない失敗は `githooks: ERROR` (0 と読まない)。A1 より前は値 `githooks: not installed yet (A1 前)` |
```

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
| D20 | both | tests: real git, stub rustfmt, fake gh, isolated by construction (T4) | the `tools` job boundary comment (T3); the user rule that fixtures take the real shape; PM 893 (C11) |
| D21 | both | PLAN.md is the ratified program memo (Q0) | PM 873 (user approved the whole plan), PM 874 (ordered this memo); PLAN.md:3 |
| D26 | P1a | a tag moved forward passes (Q5) | tags are not part of the PR / merge flow; stated limit |
| D27 | both | two slices, P1a first; the seam is the empty II × III | PM 893 (A1) |
| D28 | P1a | the ratchet reads the pushed tree (`--rev`), once per distinct pushed sha; every verdict input is `<sha>`'s (its whole module on its materialised tree), the running copy only orchestrates; a ratchet deleted on the branch rejects | PM 893 (C6); round 2 A2-1 (driver decision: one source for every verdict input); fail-safe direction (A3 MIN) |
| D29 | P1a | `ELIDEX_FMT_OK` must equal the staged tree | PM 893 (E22): an opt-out covers one commit |
| D33 | P1a | `--check` is a standing PM-pass item, through the PM's own invocation; its row in `.claude/skills/pm/SKILL.md` §2 (§4.9) lands in P1a's diff | PM 893 (C9): hand-run git stays possible after P2; round 2 A5 MIN (in the PM procedure before P3); PM 908 (in P1a's diff) |
| D38 | P1a | the bypass-guard patch (bypass-guard memo §4.8): the whole write grammar of `core.hooksPath` and `GIT_CONFIG_PARAMETERS`; `--file` in scope denies from anywhere | PM 900 (user approved); the PM's A2-scope decision; measured forms (§3.2) |
| D39 | P1a | P1a break-glass: `--no-verify`, run by the user from the commands and record line the PM prepares (who acts: §7.4); recorded; the fix PR then takes the normal gate (§7.4) | round 2 A3-2 (driver decision); D32's pattern (no override, a recorded hand action); PM 908 (the PM is a Claude session, and the bypass guard denies it `--no-verify`) |
| D40 | both | the raw-merge gap until `:36` is stated with owner (PM) and trigger (P1b's merge), not closed here | PM 893 (1) (approved scope); round 2 A3-1 (driver decision) |

The P1b rows (D13–D19, D22–D25, D30–D32, D34–D37, D42): P1b memo §5.

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
| P1a | II2 same-name tag | with a tag named as the branch: `git push origin HEAD` | the same, `HEAD:other` | compare `refs/heads/` + `--short` (control fails: `refs/heads/heads/<b>`) |
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
| P1a | VI1 index | blob formatted, working-tree copy not | blob not formatted, working-tree copy formatted | read `$top/<path>` instead of `git show :<path>` |
| P1a | VI1 temp index | real index holds an unformatted `b.rs`; `git commit -m x a.rs` (formatted) passes | `git commit -a` with an unformatted tracked change | `unset GIT_INDEX_FILE` |
| P1a | D7 stdin check | — | unformatted blob (stub mirrors rustfmt: `--check` on stdin prints a diff, exits 0) | decide on `rustfmt --check` exit code |
| P1a | VI2 ACMR | deleting an unformatted `.rs` | renamed and reformatted-badly `.rs` | `--diff-filter=ACM` |
| P1a | VI3 opt-out | `ELIDEX_FMT_OK=$(git write-tree)` with a bad blob → committed, note | `ELIDEX_FMT_OK=1` with a bad blob → rejected; a tree sha from before a re-stage → rejected | honour any non-empty value; compare against `HEAD^{tree}` |
| P1a | VI4 config | staged `.rustfmt.toml` with edition 2021 and a staged `async fn` file → committed (stub reads the edition from `--config-path`) | `.rustfmt.toml` deleted in the index → rejected | `--config-path "$top"` (the working-tree config); `--edition 2021` written in the hook (the stub fails on the edition-2018 fixture) |
| P1a | VI5 merge | — | conflicted merge concluded with `git commit`, unformatted resolution staged → rejected | exit 0 when `MERGE_HEAD` exists |
| P1a | D10 env | `rustfmt --version` fails → pass with warning | blob rustfmt cannot parse | treat every rustfmt non-zero as an environment error (bad case passes) |
| both | T4 isolation | a scratch repository under the test's temp dir | the helper is handed the real repository → the test fails | drop the common-dir comparison |

The P1b rows (`merge-pr` steps 1–14, the overlay rule, III1, IV3): P1b memo §6.

## §7 Cut-over (PM / user, after each merge) and the E2E cases

The repository documents that name the old procedure change in #663 (done) and P3b; their one list is §1 non-goals ("The repo docs
that name the old procedure"), not restated here. **Where each E2E case runs, stated once**: E1–E18 in
a scratch `git clone --local` under `$S` (§7.5 setup); E19 and E24 in the shared repository, after A1 (E19) and
after A2 (E24); M1–M6 against a real open PR (they stop before the merge); M7 is the first real merge after B3.

### §7.1 P1a cut-over

| step | action | state after it | safe because |
|---|---|---|---|
| A0 | P1a merges. Its PR also edits `.claude/skills/drive/codex.md` on `main` (there since #663, 1c50c3d0; PM 935, PM 936), clause by clause, removing only what P1a replaces (the one statement of the rule; the P1b memo §7.2 B0 edit follows it). Row "Claude の PreToolUse / Stop の guard (`~/.claude/settings.json`)": the "P1a 着地まで" label goes, because its clauses stay — `hooks.json` calling Claude's PreToolUse scripts, the Stop adapter (P2), the cut-out amend / force-push script (those scripts remain, §7.3); "P1a の git hook (`pre-push` / `pre-commit`) が入ったらそちらが正" becomes: from A1 (the shim install) pre-push and pre-commit decide too, for every agent. Row "ECS-mission guard (Claude の編集 hook)": the label goes; the PR-body ratchet line (`/drive` §4) and the CI `tools` job stay; "P1a の `pre-push` が入ったらそれも加わる" becomes: from A1, pre-push runs the ratchet on each pushed commit (`--rev`) too. The provenance line ("名前の出所", pinned to this branch's sha) is re-pointed to the memos on `main`; its last sentence, 「下の「P1a / P1b 着地まで」の行は P1a / P1b の PR が外す (PM 935)」, becomes what the PRs do (PM 938): 「下の merge の行 (`/drive` §6 の 4) の暫定の節は P1b の PR が「B3 まで」と書き換え (P1b memo §7.2 B0)、B3 で外す (同 B3)」. That re-point and a check of these edits are items of the CI/tools landing list (PM 935 Q-H) | scripts on `origin/main`, nothing installed; the §4.9 PM-pass row is in the PM skill and reads `not installed yet` | nothing runs them; Claude hooks unchanged |
| A1 | E2E E1–E18 in a scratch clone (§7.5), then user approval (PLAN §6 item 2), then `mise run install-githooks` from the main worktree; it must exit 0; then E19 in the shared repository; from here on the §4.9 row runs `--check` in every PM pass | git hooks + Claude hooks both decide (IV) | conjunctive: each push / commit needs both to allow; every branch is judged by `origin/main`'s rules (I2), so no list of unguarded branches is needed |
| A2 | global, PM / user: apply the patch of bypass-guard memo §4.8 to `~/.claude/hooks/git-hook-bypass-guard.py` with its tests and mutants — the diff includes the `git_segments()` refactor of `scan()` (PM 922), and is shown to the user before it is applied (that section's application line) — then E24; the §4.9 PM-pass row keeps running `mise run install-githooks -- --check` (standing since A1, D33) | same | I4 is prevented for the direct commands the bypass guard reads (its blind spots listed in its docstring), detected by `--check` for everyone |
| A3 | one good push and one good commit in a real lane worktree | same | real use, both layers deciding |

**For the user, from A1** (PM 908). The shims are installed at A1, so from then on the push / commit break-glass
can be needed; before A2 and after it, the bypass guard denies `--no-verify` to every Claude session, the PM's
included. So when a broken hook on `origin/main` blocks its own fix (the push / commit
break-glass, §7.4), you run the bypass — in your own terminal, or as `! <command>` in the PM session — from the exact
commands and record line the PM gives you; no agent can run it. Before A2, check the `! <command>` form once
(§7.4, the unverified premise and its one-command check); until it passes, use your own terminal.

**No `settings.json` step (PM 950).** The cut-over ends at A3. It changes no `settings.json` entry and neither
`git-push-branch-guard.sh` (`:24`) nor `cargo-fmt-precommit.sh` (`:30`): both keep running on every command, in every
repository, as today, and the git hooks decide beside them (IV5). The one global change is A2's patch; its blast
radius and application are stated in the bypass-guard memo §4.8.

### §7.2 P1b cut-over

P1b memo §7.2.

### §7.3 Needs a further user approval (not in the approved scope)

PM 893 (1) limited the approved change to `:24` and `:30`, and PM 950 withdrew that one: both stay as they are
(IV5), and by PM 964 `:13` stays as it is too (IV5). The others, each with what it would become; the PM takes them
to the user:

| line | today | would become |
|---|---|---|
| `:36` | `gh-pr-merge-head-guard.sh` | as PLAN §1 row 6: in elidex, an adapter onto the same core — a raw `gh pr merge <N>` runs `merge-pr --dry-run <N>`'s checks and is denied when they refuse; other repos unchanged. **Proposal (for the PM to take to the user)**: in elidex, deny raw `gh pr merge` outright, naming `merge-pr`. Reason: with the adapter two merge procedures remain (IV5) and the raw one still needs `:42` / `:136` / `:142`; with the deny, `merge-pr` is the one way. Break-glass is not changed by either form (P1b memo §7.4) |
| `:42` | delete-branch worktree guard | in elidex, inert once raw merges are denied (`merge-pr` writes `--delete-branch=false`); kept as is under the adapter form |
| `:136`, `:142` | PostToolUse sync / cleanup | unchanged entries; the scripts stay (V4). Under the deny form they never fire for elidex merges; under the adapter form they keep serving raw merges. Changed only after `:36` (V3) |

### §7.4 Break-glass

Neither kind is an override flag, and neither is run by a lane driver; each is a hand action with a record.

**Who acts, for both kinds (PM 908, PM 916; the one statement, D32 and D39 refer here).** **The actor is the
user** — decided by PM 908 for the push / commit kind and by PM 916 for the merge kind, one actor rule for both. The
ground is those decisions, not what a guard would do to an attempt by the PM, and the guards differ by kind.
Push / commit: `git-hook-bypass-guard.py` (the file bypass-guard memo §4.8 patches) denies a Claude session's
`--no-verify` in elidex (it fails open only on an internal error), so a PM attempt is denied. Merge:
`gh-pr-merge-head-guard.sh` (`:36`) is not an enforcement against a raw merge. It allows a PR that Codex never
assessed, fails open on any `gh` error, and honours `# merge-stale-ok` / `# codex-head-override` (its header, lines
14–19), so a raw `gh pr merge` by a Claude session, the PM's included, often passes it. The user runs the commands
in their own terminal, or as `! <command>` in the PM session. The PM prepares the exact commands (per kind: below, and P1b memo §7.4) and the record line
(PORTFOLIO, with a decision number and the reason), and gives both to the user.
**The `! <command>` form rests on a premise: that a command typed as `! <command>` in the PM session does not pass
through Claude's PreToolUse hooks. Unverified.** Before the A2 cut-over, the user checks it with a harmless command:
type `! git -C /Users/kazuaki/repos/send.sh/elidex -c core.hooksPath=/dev/null --version` in the PM session and
confirm it prints the git version with no deny (control: the PM runs the same command as a Bash tool call and it is
denied, `-c core.hooksPath` in scope; a control that is not denied makes the check void). Until the check passes,
the user's own terminal is the only form.
**What the check shows, and what it does not.** It proves only the bypass guard's PreToolUse decision: no deny for a
`! <command>`. That guard has no `if` filter (`settings.json:113`) and the control shows it is live for a Bash tool
call; whether, for `! <command>`, the hook is not run or runs and reaches no deny, the check cannot tell apart ("no
deny" is also what a fail-open guard yields), and the bypass needs only the outcome. It says nothing about the
entries carrying an `"if"` filter: `:24` / `:30` (`"Bash(git *)"`, `settings.json:25`, `:31`) and `:36` / `:136` / `:142`
(`"Bash(gh *)"`, `settings.json:37`, `:137`, `:143`). Neither break-glass kind relies on them: the merge commands run
the post-merge scripts themselves (P1b memo §7.4).

**Merge (P1b, D32).** P1b memo §7.4.

**Push / commit (P1a, D39).** A broken pre-push, pre-commit or ratchet on `origin/main` rejects the push (or the
commit) of its own fix branch, and `--no-verify` is denied to every Claude session. The PM prepares, in the fix
branch's worktree, `git commit --no-verify` if pre-commit is the broken part, then
`git push --no-verify origin <branch>`. The fix PR then goes through the normal gate (review, CI, and `merge-pr`
once P1b is live)
— only the hook run is skipped, nothing after it. The same applies to a PR that really removes or renames the
ratchet (F, "the branch deleted the ratchet").

### §7.5 E2E cases

Unsafe orders: changing `:136` / `:142` before `:36` (raw merges lose sync and cleanup); installing before A0 (the
shim passes everywhere).

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
| E5 | P1a | `git push origin HEAD:other`; then `git tag wtb`, `git push origin HEAD`, `HEAD:other`, `git tag -d wtb` | rejected, branch mismatch; with the tag: pushed, rejected |
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
| E19 | P1a | after A1, in the shared repository, through the PM's own invocation: `mise run install-githooks -- --check` → exit 0. Negative control through the same invocation, from a script file that installs a backstop first — `trap 'git worktree remove --force $S/neg 2>/dev/null' EXIT`, for an early exit only — then `git worktree add --detach $S/neg origin/main`, `git -C $S/neg config --worktree core.hooksPath $S/alt`, `mise run install-githooks -- --check; rc1=$?` from the main worktree → `rc1` is 1 and the output names `$S/neg`; then, explicitly, `git worktree remove --force $S/neg` and `mise run install-githooks -- --check; rc2=$?` → `rc2` is 0; the script ends with `[ "$rc1" = 1 ] && [ "$rc2" = 0 ]` as its status | as stated; the negative control is the one sanctioned write of the key, undone by the explicit removal (the trap only covers an exit before it) |
| E24 | P1a | after A2 (the diff, refactor included, applied after the user saw it), in the shared repository: the bypass-guard memo §4.8 E2E line | as that section states |

M1–M7 [P1b]: P1b memo §7.5.

## §8 Decided (formerly open questions)

No question is open. Each former question was decided by the driver (lens converged) or the PM; ground in one line.

- **Q0** PLAN.md is the ratified program memo (driver memo) — PM 873 (the user approved the whole Codex-lane plan;
  PM confirmed), PM 874 (ordered this per-PR memo); PLAN.md:3 records it. PLAN §6 decision 5 (Codex driver
  sandbox) was revised by PM 877/878; P1 does not depend on it except P1b memo §7.2 step B1.
- **Q1–Q4** (`merge-pr`): P1b memo §8.
- **Q5** A tag moved forward passes II5: accepted as a stated limit, named in §1 non-goals. Ground: tags are not
  part of the PR / merge flow.
- **PM 893** (plan-review round 1; labels such as "(C5)" or "(E22)" name the items of that round's decision list,
  sections A–E, items 1–29): two slices, P1a first (D27); pre-commit as its own axis VI; the Claude lines other than
  `:24` / `:30` in §7.3; shim runs `origin/main`'s copy (D11); ratchet
  `--rev` (D28); the Codex requirement of `merge-pr` step 8 and the `Completed` row departure (D30); no override, break-glass
  recorded (D32); `--check` standing (D33); the Codex execpolicy prerequisite (P1b memo §7.2 B1); tests isolated by
  construction (T4, D20); invocation site (D34); `ELIDEX_PM_DIR` set by the PM's kickoff (D22); `CLAUDE_PROJECT_DIR`
  forwarded (D18); the full-sha token told to the PM (P1b memo §7.2); both rollup shapes (D24); `ELIDEX_FMT_OK` per
  invocation (D29); token trust model (D36); Dependabot through `@codex review` (D35); merge queue refuses (D37).
- **PM 900 and the PM's A2-scope decision** (plan-review round 2): the user approved the bypass-guard extension; its
  scope is the whole write grammar of `core.hooksPath` plus `GIT_CONFIG_PARAMETERS`, with tests per form, negative
  controls and a mutant; the memo notes the existing `-c` / `--config-env` / `GIT_CONFIG_KEY_<n>` rules so the new
  rule does not overlap them silently; the PM applies it at P1a cut-over with the shim (bypass-guard memo §4.8, D38).
- **Round 2 driver decisions** (the open choices of the round-2 findings): A3-1 the raw-merge gap until `:36` is
  stated with owner and trigger, `:36` not pulled into scope (IV5, D40); A3-2 the P1a break-glass (§7.4, D39); A5-1
  the assessed-sha derivation and the no-sha item as a finding (`merge-pr` step 8c, D30); A2-1 every `--rev` verdict input
  from `<sha>` (§4.5, D28); A2-2 `installed(wt)` by bytes, no earlier version, and the Codex sandbox caveat (§4.1,
  I4, D12); the go token path kept, PLAN §6 decision 6 being its
  ground (D42).
- **PM 908** (after plan-review round 2): the plan is split into this memo (P1a, and everything both slices share)
  and the P1b memo (§0 Pair); the PM-skill row is in P1a's diff (§4.9, D33, §7.1 A0–A2); the P1a break-glass actor
  is the user, with the commands and record line prepared by the PM (§7.4, D39, the A1 note in §7.1).
- **PM 916**: §4.8 is its own file, the bypass-guard memo, applied by this memo's cut-over by reference (§0, §7.1
  A2); the merge break-glass actor is the user too, by this decision, with one actor rule for both kinds (§7.4;
  P1b memo D32, III5) — the ground is the decision, not `gh-pr-merge-head-guard`, which does not stop a Claude
  session's raw merge (§7.4: it allows an unassessed PR, fails open on `gh` errors, honours its overrides); the
  `! <command>` premise is unverified, and the user checks it before A2 (§7.4, the A1 note in §7.1).
- **PM 922** (user approved option A): the effective-cwd derivation is one iterator, `git_segments()`, in the bypass
  guard, and `scan()` is built on it (bypass-guard memo §4.8).
- **PM 930** (the part that stands): the break-glass actor rule rests on PM 908 / PM 916 themselves, and `:36` is
  described by what it does, not as an enforcement (§7.4; P1b memo D32).
- **PM 930 / 936** (the part that stands): `scan()`'s deny is not widened; its blind spots, zsh's included, are
  listed in the docstring (bypass-guard memo §4.8).
- **PM 936, PM 938** (the part that stands): the codex.md edits are per clause and timed by A1 / B3 (§7.1 A0, P1b
  memo §7.2 B0); codex.md line 9 in A0, the "B3 まで" clauses at B3.
- [history] Superseded by PM 950, content not kept: PM 893's change of `:24` / `:30`, round 2's A2-3, the second
  caller PM 922 gave the iterator, and the pass-through parts of PM 930, PM 936 and PM 938.
- **PM 950** (user, option D): no pass-through. The Claude Bash hooks at `:24` / `:30` keep running on every command,
  exactly as today, and P1a's git hooks are added on top (IV5, V2, §7.1). Kept from the approved scope:
  `git_segments()` as the iterator `scan()` is built on (PM 922) and the deny of `core.hooksPath` writes (PM 900,
  D38). This answers PLAN §6 item 3 (§10). The ids it retired: §0 Pair.
- **PM 964**: no new `:13` design. No defect against the spec or the elidex principles has been shown for `:13` in
  this scope, so the force-push coexistence is kept by decision, with no end in this scope; a defect found later goes
  to the PM as a slot draft at that time (IV5).

## §9 Layering and what is not touched

No engine code. The scripts live under `.claude/tools/` next to the ratchet they call, and enter CI only through the
`tools` job's member glob. `~/.claude/**`, `~/.codex/**`, `.github/workflows/ci.yml` (apart from the
`timeout-minutes` re-derivation in §4.6) and every `.git/hooks` are untouched by these PRs (bypass-guard memo §4.8 is applied by the
PM, outside them). The one modified tool is `.claude/tools/ecs_mission_ratchet.py` (P1a, `--rev`; its working-tree
mode is unchanged).

**One runtime dependency crosses the repository boundary**, with a contract and an owner — tracked → global
(`merge-pr` → the post-merge scripts): P1b memo §9.

## §10 For the PM: PLAN.md corrections

PLAN.md is the PM's file; this section is addressed to the PM, who applies (or rejects) each item there. PLAN.md:3
already points here as the canonical errata; the items after the first eight are new since, so its count
"誤り 8 件" becomes fourteen.

**PLAN.md:3 itself** (not counted in the fourteen): it names "the P1 plan memo (branch ci-agent-neutral-guards-p1)
の errata 節". Since PM 908 the plan is two memos (§0 Pair); the errata are this memo's §10,
`docs/plans/2026-11-agent-neutral-git-guards-p1a.md` §10, and the pointer should name it.

Every cited line in PLAN §0, §1 rows 1, 2, 3, 5, 6, 7, 8, 17, §3, §5 P1, §6, §7 that points into this repository or
`~/.claude` was re-read at `origin/main` 3cbd5b88 (PLAN cites 1b727d18; `git diff --stat 1b727d18 3cbd5b88 --
CLAUDE.md .claude/skills/drive/SKILL.md .github/workflows/ci.yml` is empty, and `CL:27`, `CL:33`, `CL:57`,
`ci:271-279`, `drive:99-101`, `drive:127`, `set:13,24,30,36,42,136,142` and the cited hook line ranges say what
PLAN says; at 1c50c3d0 #663 has moved the drive lines: `drive:99-101` is `/drive` §4's ratchet bullet, `drive:127` §6 step 4). Re-checked at 6cbe39e3: `git diff --stat 3cbd5b88 6cbe39e3 -- CLAUDE.md .claude/skills/drive/SKILL.md
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
  sets it in each kickoff (P1b memo §4.4).
- **§5 P1** ("1 PR"): two slices, P1a (git hooks) then P1b (merge wrapper) (PM 893).
- **§1 preamble** ("shim は script が無い古い branch では通す"): the shim runs `origin/main`'s copy, so old branches
  are guarded too (PM 893; I2).
- **§0.1** ("admin は bypass する"): the ruleset's admin bypass was removed on 11-21 (PM).
- **§5 P1 completion and §6 item 3** ("`set:24,30` の Claude hook 撤去"): not removed and not changed — both keep
  running on every command, and the git hooks are added on top (PM 950, user; §7.1); `set:13,36,42,136,142` need a
  further user approval (§7.3).
- **§1 row 6** (wrapper: "Codex 評価 head == head"): a dry verdict on the head after the last trigger with 0
  unresolved Codex threads; the summary row's `Completed` is not an assessment (PM 893; P1b memo §2 III step 8).
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
- `gh pr view --json` field list (no merge-queue field: hence the GraphQL query of `merge-pr` step 6).
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
- bypass-guard memo §4.8 (PM 900, 922, 950): the live guard (`9902fe77…0438`) and its test copied, patched, run and
  mutated in the scratchpad; how, and every count and probe result, only in that memo's Tests and Mutants paragraphs.
- `~/.claude/hooks/gh-pr-merge-head-guard.sh` header lines 1–19; `.claude/skills/external-converge/project.md`
  § reviewer and `external-review/project.md` § reviewer; `~/.claude/skills/external-review/SKILL.md` Step 1
  resolver; `.claude/skills/pm/SKILL.md` §4's merge bullets; `~/.codex/rules/default.rules` (`prefix_rule(pattern=[…])`);
  `.github/workflows/ci.yml:191-194`, `:275-278` at `origin/main` 6cbe39e3; PLAN.md §6 item 6.
