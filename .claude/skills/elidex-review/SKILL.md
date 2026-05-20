---
name: elidex-review
description: elidex project-specific pre-push design review (5-agent parallel). Checks Layering mandate / ECS-native lens (with component data-flow integrity) / pragmatic shortcut / spec citation / project-context beyond what generic /simplify + /review cover. Run BEFORE `git push` to compress Copilot R-loop.
user-invocable: true
---

# elidex-review — pre-push diff review

`/simplify` (code reuse / quality / efficiency) と `/review` (一般 PR review) **の上に重ねる** elidex 専門 design review。Pre-push triple-pass の 3 段目で Copilot R-loop の flag を圧縮する。

- **Axis SSoT**: `./axes.md` (5 axis 定義、`elidex-plan-review` と共有)
- **Workflow SSoT**: `./workflow.md` (Step 1.5 / 3 / 3.5 / 4 + anti-patterns + change log)

本 SKILL.md = thin lifecycle wrapper for input = `git diff $BASE...HEAD` (3-dot / merge-base vs the freshened base — `origin/main` after a best-effort fetch, falling back to local `main`; the branch's own changes, matching the GitHub PR diff)。

## When to invoke

- **Pre-push 必須段 (順序固定)**: `cargo fmt` → `mise run ci` → `/simplify` → `/review` → **本 skill (`/elidex-review`)** で全 PR 実施推奨。本 skill は 5 段目 = 最終 design gate
- generic `/review` だけでは elidex-specific design 原則違反は漏れる (Layering mandate / ideal-over-pragmatic 等)

## Skip OK

- doc-only PR (no src changes) → 各 agent が trivially 0 finding、明示 skip 可
- diff < 30 LoC かつ既存 pattern minor extension のみ → judgment skip 可、landing memo に理由明示

## Workflow

### Step 1 — Collect diff + resolve repo root

```bash
# Clear stale dry-run from prior PR review in the same session —
# the Step 1.5 output path is fixed and Write tool requires a Read
# before overwriting a non-empty file, which trips agents up when a
# previous invocation's dry-run still sits at the same path.
rm -f /tmp/elidex-review.dry-run.md

# Resolve the base ref against CURRENT remote main, not a possibly-stale local
# `main`. The GitHub PR diffs against origin/main; a local `main` that hasn't
# been fetched can be behind it, making both the diff and the staleness check
# silently wrong. Best-effort refresh — warn (don't silently swallow) on
# failure so a stale base is visible rather than masquerading as current.
git fetch --quiet origin main 2>/dev/null \
  || echo "⚠ could not fetch origin/main (offline?) — base may be stale; using last-known refs." >&2
# Prefer the freshened origin/main; else a verified local main; bail clearly if
# neither ref exists (non-main default branch / partial checkout) rather than
# letting the diff fail cryptically.
if git rev-parse --verify --quiet origin/main >/dev/null; then BASE=origin/main
elif git rev-parse --verify --quiet main >/dev/null; then BASE=main
else echo "no origin/main or local main to diff against — fetch the base branch first" >&2; exit 1; fi

# 3-dot `$BASE...HEAD` (NOT 2-dot `$BASE..HEAD`) so the range is
# `merge-base($BASE, HEAD)..HEAD` — exactly this branch's own commits since it
# forked, matching the diff the GitHub PR will show. Like 2-dot it is
# committed-only (no working-tree contamination), but it ALSO avoids the
# failure mode where the base has advanced past the branch's fork point (e.g. a
# sibling PR merged mid-session): plain 2-dot then reports those base-only
# commits as phantom "deleted" files, contaminating the review. (2026-05-20
# incident: an event-handler PR merged to main while an observer branch —
# forked from the previous main — was under review; 2-dot surfaced 13 unrelated
# files as deletions, wasting a 5-agent run.)
git diff "$BASE"...HEAD > /tmp/elidex-review.diff
git diff "$BASE"...HEAD --stat > /tmp/elidex-review.stat
wc -l /tmp/elidex-review.diff

# Staleness preflight: if $BASE is NOT an ancestor of HEAD, the (freshened)
# base has advanced past the branch's merge-base. The 3-dot diff above is still
# correct (it is the branch's own changes), but the review then runs against
# the OLD fork point — semantic drift against newer base code (e.g. an API this
# branch calls was renamed/changed upstream) is invisible. Surface it so the
# author can rebase for full fidelity + a clean PR before pushing. Non-blocking.
if ! git merge-base --is-ancestor "$BASE" HEAD; then
  behind=$(git rev-list --count "HEAD..$BASE")
  echo "⚠ branch is ${behind} commit(s) behind ${BASE} (base advanced past the fork point)." >&2
  echo "  Diff is correct (3-dot merge-base), but consider 'git rebase ${BASE}' so the" >&2
  echo "  review reflects the real merge result. Re-run Step 1 after rebasing." >&2
fi

# Resolve repo root for Step 2 agent prompts (axes.md absolute path placeholder)
REPO_ROOT=$(git rev-parse --show-toplevel)
test -f "$REPO_ROOT/.claude/skills/elidex-review/axes.md" || { echo "axes.md missing at $REPO_ROOT/.claude/skills/elidex-review/axes.md" >&2; exit 1; }
```

Diff size > 5000 行なら user 確認 (5-agent token cost 過大)。

### Step 1.5 — Mental dry-run

`workflow.md` § "Step 1.5" を適用、output を `/tmp/elidex-review.dry-run.md` に。**対象は test 限定ではない** — workflow.md 通り「新規/変更 test case AND new code path that reads ECS components」両方 (refactor PR の new caller / new system query 等 non-test も含む) を simulate、Sub-check 2b coverage を担保。Step 2 Agent 2 prompt に hand off。

### Step 2 — Launch 5 agents in parallel

**同一 message 内 5 並列 Agent tool call** (sequential / inline self-review NG、`workflow.md` § "Anti-patterns" 参照)。

各 agent prompt:

```
Agent <N> — Axis <name> review (diff).

Read ${REPO_ROOT}/.claude/skills/elidex-review/axes.md Axis <N> section.
(${REPO_ROOT} resolved in Step 1 via `git rev-parse --show-toplevel`; substitute the concrete absolute path before dispatching the agent.)

Apply Axis <N> Detect entries tagged [diff] or [both] to /tmp/elidex-review.diff (the branch's own changes vs the resolved base, 3-dot `$BASE...HEAD` where `$BASE` = current `origin/main`; stat at /tmp/elidex-review.stat).

<Agent 2 only>: Also read /tmp/elidex-review.dry-run.md and incorporate gaps into Sub-check 2b findings.

Output per axes.md Axis <N> "Output format". Severity per axes.md common calibration. Acceptable exceptions per axis. Do NOT propose fixes — list raw suggestion only. Report total findings count by severity at end.
```

| Agent | Axis |
|-------|------|
| 1 | Axis 1 — Layering mandate |
| 2 | Axis 2 — ECS-native lens (+ dry-run) |
| 3 | Axis 3 — Pragmatic shortcut |
| 4 | Axis 4 — Spec citation |
| 5 | Axis 5 — Project-context |

### Step 3 / 3.5 / 4

`workflow.md` § "Step 3" / "Step 3.5" / "Step 4" 参照。

## Recommendation phrasing (skill-specific)

- **CRIT**: fix BEFORE push (Copilot R で必ず flag される)
- **IMP**: push 前 fix 推奨 (Copilot R で 80% flag 確率)
- **MIN**: judgment (defer 可、landing memo で justify)
- **FP**: ignore (user 確認後)

**Pre-push gate**: 0 CRIT + 0 IMP → push 推奨 / 1+ CRIT → push 前 fix mandatory / 1+ IMP → user 判断 (fix or defer with justification)。
