---
name: elidex-review
description: elidex project-specific pre-push design review (5-agent parallel). Checks Layering mandate / ECS-native lens (with component data-flow integrity) / pragmatic shortcut / spec citation / project-context beyond what generic /simplify + /review cover. Run BEFORE `git push` to compress Copilot R-loop.
user-invocable: true
---

# elidex-review — pre-push diff review

`/simplify` (code reuse / quality / efficiency) と `/review` (一般 PR review) **の上に重ねる** elidex 専門 design review。Pre-push 5 段 gate の最終段 (`/pre-push` Stage 5) で Copilot R-loop の flag を圧縮する。

- **Axis SSoT**: `./axes.md` (5 axis 定義、`elidex-plan-review` と共有)
- **Workflow SSoT**: `./workflow.md` (Step 1.5 / 2 agent prompt template / 3 / 3.5 / 4 / 4.5 + anti-patterns)

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
# Clear stale dry-run from prior PR review in the same session (Write tool's
# "Read first" guard otherwise trips agents on stale residue).
rm -f /tmp/elidex-review.dry-run.md

# Resolve $BASE against CURRENT remote main (the GitHub PR diff's base), not a
# possibly-stale local main. Best-effort fetch — warn on failure rather than
# silently using a stale ref.
git fetch --quiet origin main 2>/dev/null \
  || echo "⚠ could not fetch origin/main (offline?) — base may be stale; using last-known refs." >&2
if git rev-parse --verify --quiet origin/main >/dev/null; then BASE=origin/main
elif git rev-parse --verify --quiet main >/dev/null; then BASE=main
else echo "no origin/main or local main to diff against — fetch the base branch first" >&2; exit 1; fi

# 3-dot `$BASE...HEAD` = merge-base..HEAD (the branch's own commits, matching
# the GitHub PR diff). Avoids the 2-dot phantom-deletion failure mode when the
# base has advanced past the branch's fork point.
git diff "$BASE"...HEAD > /tmp/elidex-review.diff
git diff "$BASE"...HEAD --stat > /tmp/elidex-review.stat
wc -l /tmp/elidex-review.diff

# Staleness preflight: warn if $BASE has advanced past merge-base (review still
# correct, but semantic drift against newer base code is invisible). Non-blocking.
if ! git merge-base --is-ancestor "$BASE" HEAD; then
  behind=$(git rev-list --count "HEAD..$BASE")
  echo "⚠ branch is ${behind} commit(s) behind ${BASE} — consider 'git rebase ${BASE}' before pushing." >&2
fi

# Resolve repo root for Step 2 agent prompts (axes.md absolute path placeholder)
REPO_ROOT=$(git rev-parse --show-toplevel)
test -f "$REPO_ROOT/.claude/skills/elidex-review/axes.md" || { echo "axes.md missing at $REPO_ROOT/.claude/skills/elidex-review/axes.md" >&2; exit 1; }
```

Diff size > 5000 行なら user 確認 (5-agent token cost 過大)。

### Step 1.5 — Mental dry-run

`workflow.md` § "Step 1.5" を適用、output を `/tmp/elidex-review.dry-run.md` に。**対象は test 限定ではない** — workflow.md 通り「新規/変更 test case AND new code path that reads ECS components」両方 (refactor PR の new caller / new system query 等 non-test も含む) を simulate、Sub-check 2b coverage を担保。Step 2 Agent 2 prompt に hand off。

### Step 2 — Launch 5 agents in parallel

`workflow.md` § "Step 2" の prompt template + variable table を使う。本 skill の変数:

| Variable | Value |
|---|---|
| `<INPUT_TAG>` | `[diff]` |
| `<INPUT_PATH>` | `/tmp/elidex-review.diff` (stat at `/tmp/elidex-review.stat`) |
| `<INPUT_CONTEXT>` | `the branch's own changes vs the resolved base, 3-dot \`$BASE...HEAD\` where \`$BASE\` = current \`origin/main\`` |
| `<DRYRUN_PATH>` | `/tmp/elidex-review.dry-run.md` |
| `<LOC_RULE>` | `file:line` |

### Step 3 / 3.5 / 4 / 4.5

`workflow.md` § "Step 3" / "Step 3.5" / "Step 4" / "Step 4.5" 参照。

Diff-stage 特記事項のみ: Step 4.5 (fix-delta re-verification) の placement は **push 直前に cumulative 一括** (workflow.md "Placement" 節参照、code fix は plan fix より独立なので即時でなく batch)。

## Recommendation phrasing (skill-specific)

- **CRIT**: fix BEFORE push (Copilot R で必ず flag される)
- **IMP**: push 前 fix 推奨 (Copilot R で 80% flag 確率)
- **MIN**: judgment (defer 可、landing memo で justify)
- **FP**: ignore (user 確認後)

**Pre-push gate**: 0 CRIT + 0 IMP → push 推奨 / 1+ CRIT → push 前 fix mandatory / 1+ IMP → user 判断 (fix or defer with justification)。
