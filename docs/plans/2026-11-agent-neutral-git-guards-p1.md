# Plan — P1: agent-neutral git guards and the merge wrapper

## §0 Status

**Program**: the Codex-lane program. Its driver memo lives **outside this repository**:
`/Users/kazuaki/repos/send.sh/elidex-pm-trial/handoff/codex-lanes/PLAN.md` (PM 872, status "draft, not ratified").
This memo is the per-PR plan for its phase **P1** (PLAN §5 P1, rows 1, 2, 3, 5, 6, 7, 8 and 17 of PLAN §1).
**Scope source**: PM 873 / 874. The scope is not re-opened here; §8 holds only the choices that scope leaves open.
**Nature**: repo tooling. New tracked files under `.claude/tools/githooks/` and `.claude/tools/agent/`, their tests,
one `mise` task. Zero `crates/**` diff. Nothing under `~/.claude` is edited, and nothing is installed into any
`.git/hooks`: installing the shim and removing the matching Claude hooks is the **cut-over**, done by the PM after
merge (§7).
**Edge-dense**: yes — five intersecting axes (§2). So `/elidex-plan-review` is mandatory before implementation
(CLAUDE.md Design discipline, "Edge-dense work"). This PR is meant as the terminal per-PR slice of the program.
⚠ The base case of that rule asks for an *approved* umbrella; PLAN.md is still a draft. Whether PM 873/874 counts
as that approval is the PM's call (§8 Q0).

## §1 Goal and non-goals

**Goal.** The guards that today exist only as Claude Code PreToolUse / PostToolUse hooks, and that decide on the
*text* of a Bash command, move to a layer every agent and every human goes through: git's own hooks (which see what
git is about to do) and one merge wrapper (which is the merge). A Codex driver then gets the same guard as a Claude
driver, from one implementation (CLAUDE.md "One issue, one way").

Files this PR adds (all (NEW)):

| file | what it does |
|---|---|
| `.claude/tools/githooks/pre-push` | per pushed ref: reject non-fast-forward (force or delete), reject a target branch that is not the current branch (detached HEAD exempt), reject any push to `refs/heads/main`; then run the ECS-mission ratchet once |
| `.claude/tools/githooks/pre-commit` | rustfmt check of the staged `.rs` blobs only; opt-out by env var |
| `.claude/tools/githooks/shim.sh` | the body the installer writes into the hooks directory, once per hook name; runs the tracked script of the worktree git is running in, passes when that script is absent |
| `.claude/tools/githooks/install.sh` | the installer the `mise` task runs |
| `.claude/tools/agent/merge-pr` | `merge-pr <PR>`: PM go token, Codex assessed head == PR head, `mergeStateStatus`, then `gh pr merge --squash --match-head-commit`, then the existing post-merge sync and cleanup scripts |
| `.claude/tools/githooks/test_githooks.py` | control + mutant tests for both hooks, the shim and the installer |
| `.claude/tools/agent/test_merge_pr.py` | control + mutant tests for the wrapper |
| `mise.toml` `[tasks.install-githooks]` | runs `bash .claude/tools/githooks/install.sh`; not part of `[tasks.ci]` |

**Non-goals.**
- Installing the shim into the shared repository, or removing any Claude hook (cut-over, §7, PM after merge).
- The Codex PreToolUse / Stop adapters (PLAN rows 4, 9, 11 = P2) and the repo docs that name the new procedure
  (`drive/codex.md`, `/pm` §4, CLAUDE.md Workflow line on the push guard = P3).
- Stopping `git commit --amend` itself. Amend is harmless until the amended commit is pushed over its old self,
  and that push is a non-fast-forward which pre-push rejects (PLAN row 1).
- Defeating a deliberate bypass. A git hook cannot stop `--no-verify` (git §HOOKS, §3 row 8); CI (`check` for
  fmt, `tools` for the ratchet) is behind it, and for Claude sessions the existing `git-hook-bypass-guard.py`.

## §2 Coupled invariants

Five axes plus two cross-cutting ones. Each invariant has an id; the pair table below gives each pair's
intersection in one line.

**I — one hooks directory serves every linked worktree and every branch.**
- I1. The shared repository has `core.hooksPath` set in its local config to its own common hooks dir
  (`git config --show-origin --get core.hooksPath` → `file:…/elidex/.git/config  …/elidex/.git/hooks`). One shim
  file per hook name there runs for every linked worktree, every session (Claude, Codex, human) and every branch.
- I2. The shim runs the tracked script of the worktree git invoked it in. git sets the hook's cwd to the root of
  that worktree (§3 row 2, measured from a linked worktree), so `git rev-parse --show-toplevel` in the shim is the
  pushing / committing worktree, and so is the checkout of the script.
- I3. A checkout without the tracked script (a branch cut before this PR) passes, with one stderr line saying it is
  unguarded. PM scope; `main` is protected by the server ruleset and every branch by CI.
- I4. One worktree can route around the shim: `extensions.worktreeConfig` is `true` in the shared repository, and
  `git config --worktree core.hooksPath <dir>` makes `git rev-parse --git-path hooks` answer `<dir>` for that
  worktree only (measured in a scratch clone). The installer reports such worktrees; it cannot prevent them.
- I5. A branch can weaken its own copy of the scripts (I2). That edit is in its PR diff, under review.

**II — push decisions × push shapes.** pre-push stdin is one line per ref,
`<local-ref> SP <local-sha> SP <remote-ref> SP <remote-sha>` (§3 row 3). Decisions, per line:
- II1. `remote-ref == refs/heads/main` → reject, whatever the shape (new, FF, delete, from detached HEAD).
- II2. HEAD attached on branch `B`, `remote-ref` under `refs/heads/` and `!= refs/heads/B` → reject. Keyed on the
  **remote** ref, for every local-ref shape: the local ref is "supplied as it was originally given" (`HEAD` for
  `HEAD:x`, measured; an object name or `HEAD~` likewise), so a rule keyed on the local ref being `HEAD` — the
  current hook's scope — misses `<sha>:x`, `HEAD~0:x` and `git push origin <other-branch>`. Detached HEAD → exempt
  (the current hook's rule: an isolated worktree mid cherry-pick / revert).
- II3. `remote-sha` all zeros (any length, SHA-1 or SHA-256) → a new ref → allowed by the fast-forward rule.
- II4. `local-sha` all zeros (`local-ref` is `(delete)`) → reject. A delete discards the remote's commits, the same
  harm as a force push; and `git merge-base --is-ancestor <remote> 000…0` exits 128 (measured), so the FF rule
  already rejects it. Remote branch deletion belongs to the repository's `delete_branch_on_merge` setting.
- II5. Otherwise `git merge-base --is-ancestor <remote-sha> <local-sha>`: exit 0 → allow; exit 1 → reject
  (non-fast-forward); any other exit → reject (measured: 128 when the remote commit is not in the local object
  store, which means the remote moved and the push would overwrite commits we do not have).
- II6. Several lines in one push: every line is checked, every reason printed, and one rejected line rejects the
  whole push (a non-zero exit aborts the push "without pushing anything", §3 row 6; measured).
- II7. Tags: II1 and II2 do not apply (they are about `refs/heads/`); II3–II5 apply. Annotated tag objects are
  peeled by `merge-base --is-ancestor` (measured exit 0 between two tag objects on one line of history). A tag
  moved *forward* along history passes II5 (stated limit, §8 Q5).
- II8. Empty stdin (an up-to-date push, measured: the hook runs with no lines) → allow and do not run the ratchet.
- II9. The ratchet runs once per push, after every line has passed, and only if at least one line updates or
  creates a ref. It reads the pushing worktree's **working tree**, not the pushed commit (`ecs_mission_ratchet.py`
  `iter_texts` opens files on disk) — stated limit: an uncommitted edit can make it reject or pass a push it should
  not; the CI `tools` job runs it on the PR head and is the authority.
- II10. `--no-verify` skips the hook entirely (§3 row 8). Not stoppable from inside git.

**III — merge decision order and TOCTOU.** `merge-pr <PR>` does, in this order, reading the PR head `H` exactly once:
1. Usage: exactly one argument, all digits. Else exit 2 (the cleanup script needs a number, and a URL or branch
   argument would need a second resolution of the PR).
2. `gh pr view <PR> -R <repo> --json number,state,isDraft,headRefOid,mergeStateStatus` → `H` (40 hex). `state` must
   be `OPEN`.
3. Go token: the file `<PM dir>/handoff/codex-lanes/go/<PR>-<H>` must exist, with `H` the full 40-hex sha (PLAN §6
   decision 6; Claude drivers use it too). A token naming another sha of the same PR is reported by name ("go is for
   `<sha>`, head is `H`"). No prefix matching.
4. Codex assessed head == `H`: the paginated logic of `gh-pr-merge-head-guard.sh` (formal reviews ∪ `Reviewed
   commit:` comments ∪ `Completed` rows of the review-summary comment; `--paginate --slurp`, flattened with `add`).
   Compared on the shorter of the two lengths, at least 7.
5. `mergeStateStatus == CLEAN`. Anything else, including values not known today, refuses.
6. `gh pr merge <PR> -R <repo> --squash --match-head-commit <H> --delete-branch=false`. Never `--auto`, `--admin`,
   `-d`. `--match-head-commit` is the server-side pin: if the head moved after step 2, GitHub refuses the merge.
7. `gh pr view <PR> --json state,headRefOid` → `MERGED` and head `H`. Only then step 8. If step 6 exited non-zero
   but step 7 shows `MERGED` at `H` (a lost response after a server-side merge), step 8 runs and a warning is printed.
8. Run `~/.claude/hooks/post-merge-sync-main.sh`, then `~/.claude/hooks/post-merge-cleanup-branch.sh`, each once,
   with stdin `{"tool_input":{"command":"gh pr merge <PR>"},"cwd":"<caller's pwd -P>"}` and the caller's cwd as the
   process cwd (both scripts locate the main worktree with `git worktree list` from the process cwd; the cleanup
   script also reads `.cwd`, `CLAUDE_PROJECT_DIR` and `lsof` to keep the caller's own worktree). Print each script's
   `additionalContext`. A missing script prints a note; a script's non-zero exit prints a note; neither changes the
   exit code.

Exit codes: 0 merged; 1 refused at steps 2–5 (nothing merged); 2 usage; 3 step 6 failed and the PR is not merged.
- III1. TOCTOU between step 2 and step 6 on the head: closed by the pin. Every check (token, Codex, state) is about
  the same `H`, and the server merges only `H`.
- III2. TOCTOU on `mergeStateStatus` between step 5 and step 6 (base moved, a check run flipped): **not** pinned by
  the server. If the base moved and the PR still merges cleanly, the squash includes a base nobody tested together
  with `H` — the same exposure as today's manual merge. Stated limit.
- III3. A new Codex review posted between step 4 and step 6 is about `H` or a later head; a later head is caught by
  the pin.

**IV — coexistence with the Claude hooks during migration** (shim installed, Claude hooks not yet removed).
- IV1. On a Claude `git push` both the Claude PreToolUse hooks (`settings.json` lines 13 and 24) and pre-push
  decide; the push proceeds only if all of them allow. Where they differ: the Claude branch guard fires only on a
  `HEAD:<branch>` refspec in the command text (`git-push-branch-guard.sh` lines 22–24), pre-push on every ref
  (wider); the Claude force guard denies the *flag* text (`-f`, `--force`, `--force-with-lease`, line 13) even for a
  fast-forward, and does not match a `+<refspec>` force, while pre-push decides on the ref shape.
- IV2. On a Claude `git commit` the Claude fmt hook (line 30) checks the working-tree file of each path the *real*
  index lists; pre-commit checks the blobs of the index git commits from. They differ on partial staging and on
  `git commit <path>` / `git commit -a`, where git uses a temporary index (measured: `GIT_INDEX_FILE` =
  `next-index-*.lock` / `index.lock`). A deliberate skip needs both opt-outs (`# fmt-ok` and the env var).
- IV3. `merge-pr` is invisible to the Claude merge hooks: its invocation text (`… merge-pr 655`) does not match their
  trigger `gh[[:space:]]+pr[[:space:]]+merge`, and the `gh pr merge` it runs is a child process, not a tool call. So
  the Claude PreToolUse head / delete-branch guards (lines 36, 42) do not double-decide, and the PostToolUse sync /
  cleanup (lines 136, 142) do not run a second time.
- IV4. If a command line both calls `merge-pr` and contains the literal text `gh pr merge <N>` (in an echo or a
  comment), the PostToolUse scripts run a second time after the wrapper's run. Both are idempotent on that second
  run: sync exits when local `main` already equals `origin/main` (sync line 35); cleanup finds no worktree holding
  the branch, `branch -D` fails silently, and it prints nothing (cleanup lines 59–62, 116–120).
- IV5. Until cut-over, a raw `gh pr merge` is still possible and still guarded by the Claude hooks: two merge
  procedures coexist. This is the strangler state "One issue, one way" forbids, bounded by §7 step 5.

**V — cut-over order** (§7 has the steps and what is safe in each intermediate state).
- V1. Nothing is installed before this PR is merged (an installed shim would pass everywhere: I3).
- V2. The shim goes in before any Claude hook comes out; the reverse order leaves a window with no guard at all.
- V3. Raw `gh pr merge` is denied (Claude side) before the Claude PostToolUse merge hooks are removed; the reverse
  order leaves raw merges without sync / cleanup.
- V4. The post-merge scripts stay in `~/.claude/hooks/` after their PostToolUse entries are removed: `merge-pr`
  calls them.

**F — fail direction per check** (rule: decide by where an unknown falls, not by how complete a list is).

| check | an unknown (tool missing, API error, unrecognised value) falls to | why |
|---|---|---|
| II1 main, II2 branch, II3–II5 FF / delete | **reject** | no backstop: the server stops non-FF only on `main` and an admin token may bypass that (PLAN §0.1, a claim not re-measured here); a mismatched push puts another session's commits in a PR |
| II9 ratchet: findings (exit 1 after a working interpreter) | reject | measured finding |
| II9 ratchet: interpreter cannot start, ratchet file absent while pre-push is present | interpreter: **pass with a warning**; file absent: reject | the CI `tools` job is the authority; in a checkout mise does not trust, the `python3` shim exits 1 (measured), the same code as a finding, so the hook first runs `python3 -c ''` to tell them apart. A pre-push without its ratchet cannot happen by checkout, only by edit, so it rejects |
| pre-commit: a staged blob rustfmt reformats or cannot parse | reject | measured |
| pre-commit: `rustfmt --version` fails | **pass with a warning** | the CI `check` job's `cargo fmt --all -- --check` is the authority; in an untrusted checkout the `rustfmt` shim exits 1 (measured). Rejecting there would train operators onto the opt-out — the failure mode recorded in `gh-pr-merge-head-guard.sh` for #487 |
| shim: tracked script absent | pass with a note | PM scope (I3) |
| installer: hooks path outside the repository's common dir, or a non-shim hook already there | refuse | writing there would act on other repositories, or replace someone's hook |
| merge-pr: every `gh` / `jq` failure, no Codex assessment at all, a `mergeStateStatus` other than `CLEAN`, no PM dir, a token for another sha | **refuse** | the wrapper *is* the merge, which cannot be undone. The current head guard fails open (its header, lines 14–16) because it sits in front of a merge a human still decides; here nothing is behind |
| merge-pr: post-merge script missing or failing | note, exit 0 | the merge already happened |

**T — the test harness** (CI `tools` job, `scripts/python-suites.sh`).
- T1. Members are tracked `test_*.py`, found by glob; every test must run and pass (a skip is red).
- T2. The offline regime drops every `GIT_*` variable and blocks sockets except `AF_UNIX`. Pushes in tests go to a
  local bare repository (local transport, no network); `gh` is a fake on `PATH`.
- T3. The job declares Python only. A check whose result depends on another runtime's version belongs to a job that
  declares that runtime (the `tools` job comment). So the pre-commit tests use a **stub rustfmt**, built to the real
  CLI shape measured in §3.2 (stdin in, formatted text out; exit 1 on a parse error; `--check` on stdin prints a
  diff and exits 0). The real rustfmt is exercised by the PM's E2E (§7).

### Pair intersections

| pair | intersection |
|---|---|
| I × II | every push from every worktree is judged by that worktree's copy of the rules (I2), so II's rules are per branch until the branch merges `main`; old branches get none (I3) |
| I × III | none structural: `merge-pr` is not a git hook and runs from whatever checkout calls it — its own I5 analogue (a PR branch can weaken the copy that merges it) is §8 Q2 |
| I × IV | in a worktree with I4's override, only the Claude hooks decide during migration, and nothing after cut-over |
| I × V | the installer writes one file per hook name into the shared hooks dir, so installation is all-worktrees-at-once; there is no per-worktree rollout |
| I × F | "script absent → pass" (I3) is the one fail-open on the reject-side checks, and it is decided by checkout, not by error |
| I × T | tests build their own repositories with linked worktrees and install the shim there; nothing in a test touches the real repository |
| II × III | `merge-pr` pushes nothing, so pre-push never sees a merge; the only push-shaped event in a merge is server-side |
| II × IV | double decisions are conjunctive, so during migration a push needs both layers to allow; IV1 lists where they disagree |
| II × V | removing the Claude force guard (line 13) is safe only once pre-push is installed, because II5 is what then stops `-f` and `+<refspec>` |
| II × F | every ref rule fails closed; only the ratchet's environment error fails open, and it runs only after every ref rule has passed |
| II × T | each II case is a real `git push` into a bare repository, so the stdin the hook reads is git's own, not a fixture |
| III × IV | IV3: the wrapper's command text is outside the Claude merge hooks' trigger; IV4: the only double run is idempotent |
| III × V | the go token protocol starts when the PM starts writing tokens (§7 step 4); until then `merge-pr` refuses every PR |
| III × F | every step before step 6 fails closed; after step 6 nothing refuses, because a merge cannot be refused afterwards |
| III × T | the fake `gh` serves recorded response shapes (page 2 holds the head review as in #487; the summary-row `Completed` shape as in the lexime #332 header comment) and acts as the server for `--match-head-commit`: it refuses when its head moved |
| IV × V | IV is exactly the state between §7 step 3 and step 5; its disagreements end when the Claude side is removed |
| IV × F | in IV the stricter answer wins, so a fail-open on one layer is covered by the other where both look |
| IV × T | the tests cannot run the Claude hooks (they live outside the repository); IV is checked by the PM's E2E only |
| V × F | each intermediate state of §7 keeps at least one fail-closed layer on push and on merge |
| V × T | the PR's tests pass with nothing installed; installation is checked by the E2E, never by CI |
| F × T | each fail direction is a test case: the stub rustfmt and a fake `python3` that cannot start cover the warn-and-pass paths, fake `gh` errors cover refusal |

## §3. Spec coverage map

There is no web spec here. The table lists the git, gh and GitHub behaviours the scripts rely on, so the
preflight's per-row check still means something: each row names its source document and section, and a
measurement where the documents say nothing. Sources: `git help githooks` (git 2.56.1), `git help merge-base`,
`git help rev-parse`, `gh pr merge --help` / `gh api --help` (gh 2.105.0), GitHub GraphQL reference (not
re-fetched for this memo).

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| git §DESCRIPTION (githooks(5)) | hooks directory | `$GIT_DIR/hooks`, or `core.hooksPath`; non-executable hooks are ignored | `install.sh` target; the shim must be executable, the tracked scripts are run with `bash` and need no exec bit | ✓ + measured: per-worktree override (I4) | yes (any worktree's config) |
| git §DESCRIPTION (githooks(5)) | working directory and environment | cwd = root of the working tree; `GIT_DIR` etc. exported | shim's `--show-toplevel`; ratchet strips repo-locating vars itself (`git_env`) | ✓ (measured from a linked worktree: cwd = that worktree, `GIT_DIR` = its gitdir) | no |
| git §HOOKS (githooks(5)) | pre-push stdin | new ref: remote sha all zeros | pre-push II3 | ✓ | yes |
| git §HOOKS (githooks(5)) | pre-push stdin | delete: local ref `(delete)`, local sha all zeros | pre-push II4 | ✓ (measured line) | yes |
| git §HOOKS (githooks(5)) | pre-push stdin | local ref "supplied as it was originally given" | pre-push II2 keys on the remote ref | ✓ (measured: `HEAD`) | yes |
| git §HOOKS (githooks(5)) | pre-push exit | non-zero aborts the whole push | pre-push II6 | ✓ (measured: nothing reached the remote) | yes |
| git §HOOKS (githooks(5)) | pre-push stdin | up-to-date push: no lines | pre-push II8 | doc silent; measured | no |
| git §HOOKS (githooks(5)) | pre-commit / pre-push | bypassed by `--no-verify` | stated limit (§1 non-goals) | ✓ | yes |
| git §HOOKS (githooks(5)) | pre-commit | no parameters; non-zero aborts; the index is `GIT_INDEX_FILE` | pre-commit reads `git diff --cached` / `git show :<path>` with the inherited `GIT_INDEX_FILE` | measured: temporary index for `commit <path>` and `commit -a` | yes |
| git §HOOKS (githooks(5)) | pre-merge-commit | merge commits run this hook, not pre-commit | not hooked: merge commits are not fmt-checked (same coverage as the Claude hook, which matches only `git commit`) | ✓ | yes |
| git §OPTIONS (git-merge-base(1)) | `--is-ancestor` | exit 0 / 1 / other | pre-push II4, II5 | measured: 128 for an unknown or all-zero sha | yes |
| git §OPTIONS (git-rev-parse(1)) | `--git-path hooks`, `--show-toplevel`, `--git-common-dir` | hooks path honours `core.hooksPath` | shim, `install.sh` | measured (I4) | no |
| gh §FLAGS (gh pr merge) | `--match-head-commit SHA` | head must match | `merge-pr` step 6 | ✓ | yes (PR number) |
| gh §FLAGS (gh pr merge) | `--squash`, `--delete-branch=false` | no `--auto`, `--admin`, `-d` | `merge-pr` step 6 argv | ✓ | yes |
| gh §FLAGS (gh api) | `--paginate --slurp` | pages as an array of arrays | `merge-pr` step 4 | ✓ | no |
| GitHub GraphQL §OBJECTS (PullRequest) | `mergeStateStatus` | `CLEAN` only | `merge-pr` step 5 | allow-list, so completeness of the enum is not relied on | no |
| GitHub GraphQL §OBJECTS (PullRequest) | `headRefOid`, `state`, `isDraft` | `OPEN` before, `MERGED` after | `merge-pr` steps 2, 7 | ✓ | no |

**Breadth**: K=3 (git, gh, GitHub GraphQL), M=17. Single PR.

### §3.1 User-input touch audit

No web-content input. The inputs are what an agent or a person types: refspecs and flags of `git push`, the index
of `git commit`, the PR number given to `merge-pr`, and files under the PM dir. Each one reaches a decision only
through git's own report (pre-push stdin, the index) or a `gh` response, never by parsing the command text — that is
the point of the move (IV1, IV2).

### §3.2 Measured behaviours the documents do not state

All measured on 2026-11-21 in a scratch `git clone --local` (deleted afterwards), git 2.56.1, rustfmt 1.10.0
through `~/.cargo/bin/rustfmt`:
- `rustfmt --edition 2021 --check < bad.rs` prints a diff and **exits 0**; `rustfmt --check bad.rs` exits 1. So
  pre-commit must not use `--check` on stdin: it pipes the staged blob into rustfmt and compares the output with
  the blob (equal = clean; rustfmt non-zero = cannot parse = reject).
- In a checkout mise does not trust, the mise shims for `rustfmt` and `python3` print a trust error and exit 1.
- One local ratchet run on `origin/main` 3cbd5b88 took 8.6 s wall clock (`/usr/bin/time -p`, this machine, one
  run); that cost is added to every push that updates a ref.

## §4 What each file does

### §4.1 Shim and installer

The shim (body in `.claude/tools/githooks/shim.sh`, written by the installer as `pre-push` and `pre-commit`):

```sh
#!/bin/sh
# elidex-githook-shim v1 (installed by `mise run install-githooks`; source .claude/tools/githooks/shim.sh)
top=$(git rev-parse --show-toplevel 2>/dev/null) || exit 0
s="$top/.claude/tools/githooks/$(basename "$0")"
if [ ! -f "$s" ]; then
  echo "elidex githooks: $s not in this checkout; $(basename "$0") is unguarded here" >&2
  exit 0
fi
exec bash "$s" "$@"
```

stdin passes through `exec`. A bare repository has no top level and passes.

`install.sh`, run from any worktree of the target repository:
1. `hooks=$(git rev-parse --path-format=absolute --git-path hooks)`; refuse unless it is inside
   `git rev-parse --path-format=absolute --git-common-dir` (a global `core.hooksPath` would act on every
   repository).
2. For each of `pre-push`, `pre-commit`: if a file exists there and its second line is not the shim marker, refuse
   and name it; else write the shim and `chmod 755`. Re-running is a no-op apart from rewriting the same bytes.
3. For each worktree in `git worktree list --porcelain`, print any whose `git -C <wt> rev-parse --git-path hooks`
   differs from `hooks` (I4).
4. Print what was written. Exit 0 / 1.

### §4.2 pre-push

As §2 II1–II9, in that order per line; all reasons are collected and printed to stderr with the rule's name and the
remedy (FF: "merge, do not rebase or force"; branch: "push from the worktree that has the branch checked out";
main: "open a PR"). Current branch: `git symbolic-ref --short -q HEAD` (empty = detached). The ratchet is
`python3 "$top/.claude/tools/ecs_mission_ratchet.py"` with no arguments, after `python3 -c ''` succeeds.

### §4.3 pre-commit

1. `ELIDEX_FMT_OK` set and non-empty → exit 0 with a note.
2. `git diff --cached --name-only -z --diff-filter=ACMR -- '*.rs'` (R added to the current hook's ACM: a renamed
   file with changed content is a staged `.rs` too). None → exit 0.
3. `rustfmt --version` fails → warning, exit 0 (F).
4. Each file: `git show ":<path>" | rustfmt --config-path "$top"` (the worktree's `.rustfmt.toml`; no edition
   written in the hook — the edition has one home), compared byte for byte with the blob. rustfmt non-zero or a
   difference → that path is listed. Any listed → exit 1 with the list and "run `cargo fmt --all`, re-stage".

### §4.4 merge-pr

As §2 III, steps 1–8. The repository for `-R` is `gh repo view --json nameWithOwner` in the caller's cwd, read once.
The PM dir comes from the environment (§8 Q1). The Codex-head logic is written once, here; the copy in the global
`gh-pr-merge-head-guard.sh` goes away at cut-over (§7 step 5), so after cut-over there is one implementation.

### §4.5 Tests

Two members (T1), both `unittest` modules, each kept under 1000 lines (split by scenario when they approach it):
- `test_githooks.py`: builds, per test, a scratch repository + bare remote + a linked worktree in a temp dir,
  installs the shim there with `install.sh`, and runs real `git push` / `git commit`. A stub `rustfmt` and a stub
  ratchet (exit code chosen per test, records that it ran) are put on `PATH` / at the ratchet's path in the scratch
  checkout; a fake `python3` that exits 1 covers the interpreter-cannot-start path.
- `test_merge_pr.py`: a fake `gh` on `PATH` serving fixture responses by argv and recording every argv; a fake
  `HOME` with stub post-merge scripts that record their stdin and cwd; a temp PM dir for tokens.
- Mutants: each mutant is a one-line text replacement in a temp copy of the script. The harness asserts the
  replaced text occurs exactly once before applying it (so a later edit of the script cannot turn a mutant into a
  no-op), runs the case named in §6 against the mutated copy, and asserts the case's verdict flips.
- The PR re-derives the `tools` job's `timeout-minutes` from its own run, per the rule in that job's comment.

### §4.6 mise task

```toml
[tasks.install-githooks]
description = "Install the githook shims into this repository's hooks directory (all worktrees). Not part of ci."
run = "bash .claude/tools/githooks/install.sh"
```

## §5 Decisions and their ground

Rung = which level decided it (CLAUDE.md order: spec → design principle → measurement; here "spec" means the git /
gh documents of §3, and "incident" means a recorded incident in an existing hook's header).

| # | decision | rung |
|---|---|---|
| D1 | FF rule on every ref; delete and unknown remote sha reject (II3–II5) | PM scope; git docs (§3 rows 3, 4, 11); principle Security by structure (no server backstop off `main`); measured exit codes |
| D2 | branch rule keyed on the remote ref, every `refs/heads/` line, attached HEAD only (II2) | PM scope; incident #285 (`git-push-branch-guard.sh` header); git docs row 5 (local ref is not a name). ⚠ wider than the current hook, which only sees `HEAD:<x>` |
| D3 | any `refs/heads/main` target rejects (II1) | CLAUDE.md Workflow "main 直接 push 禁止"; PLAN row 17 |
| D4 | check every line, print every reason, one bad line aborts the push (II6) | git docs row 6 |
| D5 | ratchet once, after the ref rules, skipped on empty / delete-only pushes; warn-and-pass on an interpreter that cannot start (II8, II9, F) | principle: unknowns fall to the side with a backstop (CI `tools`); measured shim exit 1 |
| D6 | pre-commit reads index blobs, not working-tree files | git docs row 9; measured temporary index; the current hook's header admits the staged-vs-working-tree edge |
| D7 | format-and-compare, not `--check`, on stdin | measured (§3.2) |
| D8 | rustfmt config from the worktree's `.rustfmt.toml`, no edition in the hook | One issue, one way (the edition has one home) |
| D9 | opt-out `ELIDEX_FMT_OK` | PM scope (a git hook cannot see a `# fmt-ok` comment) |
| D10 | rustfmt cannot run → warn and pass | principle as D5 (CI `check` fmt step); incident #487's "trains around the guard" |
| D11 | shim passes when the script is absent, runs the script with `bash` | PM scope; git docs row 1 (exec bit) |
| D12 | installer refuses an out-of-repo hooks path and a foreign hook; reports I4 worktrees | Security by structure; measured `core.hooksPath` and `extensions.worktreeConfig` |
| D13 | merge-pr order of §2 III; `H` read once | drive skill §6 steps 2–4 (`merge at head <sha>`, re-check head and `mergeStateStatus`, `--match-head-commit`); PM scope |
| D14 | no Codex assessment → refuse (the current hook allows) | principle: unknown falls to refuse where nothing is behind; incident #367 |
| D15 | every `gh` / `jq` error → refuse | as D14 |
| D16 | `mergeStateStatus` allow-list `CLEAN` | as D14 (an enum value added later refuses) |
| D17 | `--delete-branch=false` written out | incident lexime #354 (`gh-pr-merge-delete-branch-worktree-guard.sh` header) |
| D18 | post-merge: the existing scripts, once, with the JSON and the caller's cwd | PM scope; the scripts' own interface (sync lines 18–28, cleanup lines 32–47, 70–79) |
| D19 | the Codex-head logic lives once, in `merge-pr`; the global copy goes at cut-over | One issue, one way |
| D20 | tests: real git, stub rustfmt, fake gh | the `tools` job boundary comment (T3); the user rule that fixtures take the real shape (the stubs copy the measured CLI shape and the recorded response shapes) |

## §6 Control / mutant table

Each row: a control that must pass, a bad input that must be rejected, and the one-line mutant the bad (or control)
case must kill.

| decision | control (passes) | bad input (rejected) | mutant the test kills |
|---|---|---|---|
| II3 new ref | push a new branch (remote sha 40 zeros); a 64-zero line fed to the script | — | zero test written as the 40-character literal → the 64-zero control fails |
| II5 FF | FF push of the current branch | push after `reset --hard HEAD~1` with `+<refspec>` | `--is-ancestor … \|\| reject` → `\|\| true` |
| II5 unknown remote | — | remote advanced by another clone, local never fetched (exit 128) | reject only on exit 1 |
| II4 delete | — | `git push origin --delete <current>` | skip lines whose local ref is `(delete)` |
| II2 branch | push `refs/heads/<current>` | `HEAD:other`, and `git push origin <other-local-branch>` | compare only when the local ref is `HEAD` (the Claude hook's scope) → the second bad case passes |
| II2 detached | detached HEAD pushes `HEAD:refs/heads/x` | — | drop the detached exemption |
| II1 main | push `refs/heads/domain` | push to `refs/heads/main` from branch `main` (FF, branch matches), and from detached HEAD | drop the main rule; or match `*main` (the `domain` control fails) |
| II6 several refs | — | one good line + one bad line; the remote is unchanged afterwards | reject flag set inside a piped `while read` subshell (last line wins) |
| II7 tags | push a new lightweight and a new annotated tag | move a tag back with `+refs/tags/t` | exempt `refs/tags/` from II5 |
| II8 empty push | up-to-date push; the stub ratchet did not run | — | run the ratchet unconditionally |
| II9 ratchet | stub ratchet exit 0 | stub ratchet exit 1 | ratchet call `\|\| true` |
| II9 interpreter | fake `python3` that cannot start → pass with the warning | ratchet file deleted from the checkout | treat the start failure as a finding (control fails); `[ -f ratchet ] \|\| exit 0` |
| I2 which script | two worktrees, one whose tracked pre-push rejects everything; the other's push passes | push from the rejecting one | shim resolves the script from `--git-common-dir/..` |
| I3 absent | checkout without the script → pass, stderr note | — | shim `exit 1` when absent |
| shim stdin | — | bad push through the installed shim | shim runs the script with `</dev/null` |
| shim exec bit | tracked script with mode 644 still runs | — | `exec "$s"` instead of `exec bash "$s"` |
| D12 installer | install into a fresh scratch repo; re-install is a no-op | existing foreign `pre-push`; hooks path outside the common dir | overwrite unconditionally; drop the inside-common-dir test |
| D6 index | blob formatted, working-tree copy not | blob not formatted, working-tree copy formatted | read `$top/<path>` instead of `git show :<path>` |
| D6 temp index | real index holds an unformatted `b.rs`; `git commit -m x a.rs` (formatted) passes | — | `unset GIT_INDEX_FILE` |
| D7 stdin check | — | unformatted blob (stub mirrors rustfmt: `--check` on stdin prints a diff, exits 0) | decide on `rustfmt --check` exit code |
| ACMR | deleting an unformatted `.rs` | renamed and reformatted-badly `.rs` | `--diff-filter=ACM` |
| D9 opt-out | `ELIDEX_FMT_OK=1` with a bad blob | — | opt-out ignored |
| D10 env | `rustfmt --version` fails → pass with warning | blob rustfmt cannot parse | treat every rustfmt non-zero as an environment error (bad case passes) |
| III1 usage | — | `merge-pr abc`, `merge-pr` → exit 2, fake `gh` saw no `pr merge` | accept any argument |
| III3 token | token `<PR>-<H>` present | none; token for an older sha; token named with a 7-char sha | glob `<PR>-*`; prefix compare |
| Q1 PM dir | — | PM dir variable unset | default to a path |
| III4 Codex | head review only on page 2 (#487 shape); clean auto-review only as a `Completed` summary row (lexime #332 shape) | latest assessment on an older sha; no Codex assessment at all; `gh api` exits non-zero | drop `--paginate` (fake serves page 1 only without it); drop the summary-row source; empty assessment → allow |
| III5 state | `CLEAN` | `UNSTABLE`, `BLOCKED`, `DIRTY`, `BEHIND`, `DRAFT`, `UNKNOWN`, `HAS_HOOKS`, `FOO` | deny-list `DIRTY\|BLOCKED` |
| III6 argv | merge argv exactly `pr merge <PR> -R <repo> --squash --match-head-commit <H> --delete-branch=false` | — | drop `--match-head-commit` |
| III1 TOCTOU | — | fake moves the head between view and merge → fake refuses the pinned merge → exit 3, no post-merge run | drop `--match-head-commit` (fake then merges) |
| III7 lost response | merge exits non-zero, then view says `MERGED` at `H` → post-merge runs, warning | merge exits non-zero, view says `OPEN` → exit 3 | run post-merge whenever step 6 ran |
| III8 post-merge | sync then cleanup, once each; recorded stdin's command matches `gh[[:space:]]+pr[[:space:]]+merge[[:space:]]+[0-9]+` (cleanup's own trigger) and `.cwd` is the caller's `pwd -P` | — | command text `gh pr merge` without the number; scripts run in `$HOME` instead of the caller's cwd |
| III8 missing | scripts absent → exit 0 with a note | — | exit non-zero when a script is absent |
| IV3 | `merge-pr 655` (the documented invocation) does not match the PostToolUse trigger | — | (pin on the usage string; a rename that adds `gh pr merge` to it fails) |

## §7 Cut-over (PM, after merge) and the E2E cases

**Order and what is safe at each state.**

| step | action | state after it | safe because |
|---|---|---|---|
| 0 | this PR merges | scripts on `main`, nothing installed | nothing runs them; Claude hooks unchanged |
| 1 | E2E in a scratch clone (cases below) | unchanged | scratch only |
| 2 | user approval (PLAN §6 item 2), then `mise run install-githooks` from the main worktree; check its I4 report is empty | git hooks + Claude hooks both decide (IV) | conjunctive: each push / commit needs both to allow |
| 3 | one good push and one good commit in a real lane worktree on a branch that has the scripts; list branches without them: `for b in $(git for-each-ref --format='%(refname:short)' refs/heads); do git cat-file -e "$b:.claude/tools/githooks/pre-push" 2>/dev/null \|\| echo "$b"; done` | same | listed branches are I3 (unguarded by git hooks); they keep the Claude hooks until step 5 |
| 4 | the PM starts writing go tokens; drivers merge with `merge-pr` | wrapper path live; raw `gh pr merge` still possible (IV5) | the raw path keeps its Claude guards |
| 5 | user approval (PLAN §6 item 3), then in `~/.claude/settings.json`: replace the head guard (line 36) with a deny of raw `gh pr merge` that names `merge-pr`; then remove lines 24, 30, 13 (or its force half), 42, 136, 142 | one way per guard | V2: git hooks were installed at step 2; V3: raw merges are denied before their post-merge hooks go; V4: the scripts stay |
| 6 | P3 updates the repo docs that name the old guards (CLAUDE.md Workflow names `git-push-branch-guard.sh`; drive skill §6 step 4 names raw `gh pr merge`) | docs match | — |

Unsafe orders: step 5 before step 2 (no push guard); removing lines 136 / 142 before raw merges are denied (raw
merges lose sync and cleanup); installing before step 0 (the shim passes everywhere).

**E2E cases.** Setup, once: `git clone --local <shared repo> $S/repo`, point its `origin` at a fresh `git init --bare
$S/remote.git` (the Claude shared-state guard asks for its sentinel on this command even in a clone — check
`git rev-parse --git-common-dir` is under `$S` first), `git worktree add -b wtb $S/wt`, run `bash
.claude/tools/githooks/install.sh` from `$S/repo` (not `mise run`: the scratch clone is not mise-trusted, and must not
be trusted from an unisolated mise state), and put a real `rustfmt` and `python3` first on `PATH`
(`~/.cargo/bin`, and the directory of `mise which python3` taken in a trusted checkout). Write the push and commit
cases into a script file and run it with `bash`: from a Claude session, the Claude text hooks (lines 13, 24, 30)
would otherwise deny the canonical bad commands before git runs, and the E2E would measure them, not the git
hooks. Run cases one at a time; `$S` is deleted at the end.

| # | command (in `$S/wt` on branch `wtb` unless stated) | expected |
|---|---|---|
| E1 | `git push origin wtb` (new) | pushed |
| E2 | new commit, `git push origin wtb` (FF) | pushed; ratchet "ok" line on stderr |
| E3 | `git reset --hard HEAD~1 && git push origin +wtb` | rejected, FF rule; remote unchanged |
| E4 | `git push origin --delete wtb` | rejected, delete |
| E5 | `git push origin HEAD:other` | rejected, branch mismatch |
| E6 | `git branch side && git push origin side` | rejected, branch mismatch (the Claude hook would have allowed this) |
| E7 | `git checkout --detach && git commit --allow-empty -m d && git push origin HEAD:refs/heads/det` | pushed |
| E8 | in `$S/repo` on `main`: new commit, `git push origin main` | rejected, main |
| E9 | `git push origin wtb HEAD:other` (one good, one bad) | rejected; `wtb` on the remote unchanged |
| E10 | commit a tracked line naming an engine without a tag, push | rejected by the ratchet; with the tag added, pushed |
| E11 | on a branch cut from a commit before this PR: push | pushed, stderr note "unguarded here" |
| E12 | stage an unformatted `.rs`, `git commit -m x` | rejected, path listed |
| E13 | stage the formatted file, leave an unformatted working copy, commit | committed |
| E14 | real index holds an unformatted `b.rs`; `git commit -m x a.rs` with `a.rs` formatted | committed |
| E15 | E12 with `ELIDEX_FMT_OK=1` | committed, note |
| E16 | E12 with `PATH` not holding a working `rustfmt` (the untrusted mise shim) | committed, warning |
| E17 | after step 2, in the shared repo: `git worktree list --porcelain` × `git -C <wt> rev-parse --git-path hooks` | every worktree answers the shared path |
| M1 | `merge-pr <open PR>` with no token | exit 1, "no go token"; PR still open |
| M2 | token for an older sha of that PR | exit 1, names both shas |
| M3 | token for head, Codex latest on an older sha | exit 1, Codex stale |
| M4 | token for head, `mergeStateStatus` not `CLEAN` | exit 1, names the state |
| M5 | the first real merge the PM approves after step 4 | merged; sync and cleanup notes printed once; no PostToolUse notes in the Claude transcript for that command |

M1–M4 refuse before step 6 of §2 III, so they are safe on a real open PR. The head-moves race (III1) is not
reproducible by hand; it is covered by the test with the fake server.

## §8 Open questions

- **Q0 (PM)** Does PM 873/874 count as the approved umbrella for the edge-dense base case, while PLAN.md is a draft?
  Recommendation: yes, recorded by the PM's decision number in this memo's §0 before plan-review.
- **Q1** Where `merge-pr` finds the PM dir. Recommendation: environment variable `ELIDEX_PM_DIR`, required; unset →
  refuse (a tracked file must not carry a user path, and a default would fail open).
- **Q2** A PR branch can edit the `merge-pr` that merges it (I5's analogue). Recommendation: refuse unless the
  running file's blob equals `git show origin/main:.claude/tools/agent/merge-pr` (after a fetch). This PR itself is
  then merged by hand, the last raw merge.
- **Q3** `CLEAN` is not known here to imply "every check finished green" (the ruleset has no required status check;
  whether pending or failed non-required checks give `UNSTABLE` was not measured). Recommendation: also require every
  entry of `statusCheckRollup` to be completed with `SUCCESS`, `NEUTRAL` or `SKIPPED`, which turns CLAUDE.md's "CI
  全 pass を目視確認" into a check. The PM's visual check stays.
- **Q4** A `--dry-run` flag (steps 1–5, print "would merge `<H>`", exit 0) for E2E on a real PR without merging.
  Recommendation: add it; it is the only way to run the good path of steps 2–5 without an irreversible act.
- **Q5** Tags moved forward pass II5. Recommendation: accept as a stated limit; the lane workflow does not push tags.

## §9 Layering and what is not touched

No engine code. The scripts live under `.claude/tools/` next to the ratchet they call, and enter CI only through the
`tools` job's member glob. `~/.claude/**`, `~/.codex/**`, `.github/workflows/ci.yml` (apart from the
`timeout-minutes` re-derivation in §4.5) and every `.git/hooks` are untouched by this PR.

## §10 PLAN.md claims re-checked

Every cited line in PLAN §0, §1 rows 1, 2, 3, 5, 6, 7, 8, 17, §3, §5 P1, §6, §7 that points into this repository or
`~/.claude` was re-read at `origin/main` 3cbd5b88 (PLAN cites 1b727d18; `git diff --stat 1b727d18 3cbd5b88 --
CLAUDE.md .claude/skills/drive/SKILL.md .github/workflows/ci.yml` is empty, and `CL:27`, `CL:33`, `CL:57`,
`ci:271-279`, `drive:99-101`, `drive:127`, `set:13,24,30,36,42,136,142` and the cited hook line ranges say what
PLAN says). Claims that are wrong or incomplete:
- **§5 P1 completion** ("Claude session から既知の悪い push … が拒否され"): from a Claude session the Claude text
  hooks deny `-f` / `--force` (line 13) and `HEAD:<branch>` (line 24) before git runs, so that E2E would measure the
  Claude hooks. §7 runs the cases from a script file.
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
- **§1 row 5**: the ratchet reads the working tree, not the pushed commit (II9).
- `$PMD` is used throughout PLAN but not defined in it; a tracked script needs it from the environment (Q1).

## §11 Provenance

Commands behind the facts above (run 2026-11-21; probes in a scratch `git clone --local`, origin re-pointed to a
scratch bare repository, deleted afterwards):
- `git --version` → 2.56.1; `gh --version` → 2.105.0; `~/.cargo/bin/rustfmt --version` → 1.10.0.
- `git config --show-origin --show-scope --get-all core.hooksPath`; `git config --get extensions.worktreeConfig`.
- pre-push / pre-commit recording hooks (print args, cwd, `GIT_*`, HEAD, stdin / `git diff --cached --name-only`)
  over: new branch, up-to-date, FF, `HEAD:other`, two refs, `--delete`, two tags, detached `HEAD:refs/heads/det`, a
  hook exiting 1; commits with a pathspec and with `-a`.
- `git merge-base --is-ancestor` on ancestor / non-ancestor / unknown sha / all-zero sha / two tag objects.
- `git config --worktree core.hooksPath <dir>` then `git rev-parse --git-path hooks` in that worktree and in another.
- rustfmt `--check` on stdin vs on a file, and format-and-compare, on a bad, a good and an unparsable input.
- `python3 -c ''` and `rustfmt --version` in an untrusted scratch checkout.
- `/usr/bin/time -p python3 .claude/tools/ecs_mission_ratchet.py` in this worktree.
