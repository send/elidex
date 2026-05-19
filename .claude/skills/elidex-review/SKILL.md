---
name: elidex-review
description: elidex project-specific pre-push design review (5-agent parallel). Checks Layering mandate / ECS-native lens (with component data-flow integrity) / pragmatic shortcut / spec citation / project-context beyond what generic /simplify + /review cover. Run BEFORE `git push` to compress Copilot R-loop.
user-invocable: true
---

# elidex-review — pre-push diff review

`/simplify` (code reuse / quality / efficiency) と `/review` (一般 PR review) **の上に重ねる** elidex 専門 design review。Pre-push triple-pass の 3 段目で Copilot R-loop の flag を圧縮する。

- **Axis SSoT**: `./axes.md` (5 axis 定義、`elidex-plan-review` と共有)
- **Workflow SSoT**: `./workflow.md` (Step 1.5 / 3 / 3.5 / 4 + anti-patterns + change log)

本 SKILL.md = thin lifecycle wrapper for input = `git diff main..HEAD`。

## When to invoke

- **Pre-push 必須段 (順序固定)**: `cargo fmt` → `mise run ci` → `/simplify` → `/review` → **本 skill (`/elidex-review`)** で全 PR 実施推奨。本 skill は 5 段目 = 最終 design gate
- generic `/review` だけでは elidex-specific design 原則違反は漏れる (Layering mandate / ideal-over-pragmatic 等)

## Skip OK

- doc-only PR (no src changes) → 各 agent が trivially 0 finding、明示 skip 可
- diff < 30 LoC かつ既存 pattern minor extension のみ → judgment skip 可、landing memo に理由明示

## Workflow

### Step 1 — Collect diff + resolve repo root

```bash
# Explicit ..HEAD range avoids working-tree contamination from
# unstaged changes (matches description "git diff main..HEAD")
git diff main..HEAD > /tmp/elidex-review.diff
git diff main..HEAD --stat > /tmp/elidex-review.stat
wc -l /tmp/elidex-review.diff

# Resolve repo root for Step 2 agent prompts (axes.md absolute path placeholder)
REPO_ROOT=$(git rev-parse --show-toplevel)
echo "$REPO_ROOT/.claude/skills/elidex-review/axes.md"  # verify accessible
```

Diff size > 5000 行なら user 確認 (5-agent token cost 過大)。

### Step 1.5 — Test mental dry-run

`workflow.md` § "Step 1.5" を適用、output を `/tmp/elidex-review.dry-run.md` に。Step 2 Agent 2 prompt に hand off。

### Step 2 — Launch 5 agents in parallel

**同一 message 内 5 並列 Agent tool call** (sequential / inline self-review NG、`workflow.md` § "Anti-patterns" 参照)。

各 agent prompt:

```
Agent <N> — Axis <name> review (diff).

Read ${REPO_ROOT}/.claude/skills/elidex-review/axes.md Axis <N> section.
(${REPO_ROOT} resolved in Step 1 via `git rev-parse --show-toplevel`; substitute the concrete absolute path before dispatching the agent.)

Apply Axis <N> Detect entries tagged [diff] or [both] to /tmp/elidex-review.diff (full diff vs main..HEAD; stat at /tmp/elidex-review.stat).

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
