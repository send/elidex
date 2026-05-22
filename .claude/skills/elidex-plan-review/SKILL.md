---
name: elidex-plan-review
description: elidex pre-implementation plan-memo design review (5-agent parallel). Same 5 axes as /elidex-review (Layering / ECS-native incl. component data-flow integrity / pragmatic / spec / project-context) applied to a plan-memo BEFORE implementation starts. Catches architectural drift at the cheapest stage.
user-invocable: true
---

# elidex-plan-review — pre-impl plan-memo review

`/elidex-review` の **pre-impl 対**。Plan-memo writeup 直後 / user approval 前に走らせて、implementation で発覚する architectural drift を plan 段階で潰す。

- **Axis SSoT**: `../elidex-review/axes.md`
- **Workflow SSoT**: `../elidex-review/workflow.md`

本 SKILL.md = thin lifecycle wrapper for input = plan-memo path。

## When to invoke

- Plan-memo writeup 直後 / user approval 前 — `/elidex-review` 対象 PR は plan-memo 必須、両 gate 連続適用
- Scope creep risk (新 ECS infra / 既存 system 変更 / cross-crate coordination / 複雑 test plan) 時は特に推奨
- `/elidex-review` の rehearsal ではなく **independent gate** — implementation 段階の細部は plan に無いため両方必要

## Skip OK

- Plan-memo < 50 行 + 既存 pattern 軽微 extension のみ
- Trivial cleanup / rename PR で plan-memo 不要なケース
- Skip した場合は landing memo に理由明示

## Workflow

### Step 1 — Identify plan-memo + resolve repo root

User が plan-memo absolute path を skill argument で指定する前提 (auto-discovery は per-user CLAUDE memory dir が hardcoded path にならないため不可)。Plan-memo は通常 `~/.claude/projects/<encoded-repo-path>/memory/` 配下の `m4-12-pr-*-plan.md` 等、user が明示 path 渡し。

```bash
# Clear stale dry-run from prior plan-review in the same session —
# the Step 1.5 output path is fixed and Write tool requires a Read
# before overwriting a non-empty file, which trips agents up when a
# previous invocation's dry-run still sits at the same path.
rm -f /tmp/elidex-plan-review.dry-run.md

# Skill arg = plan-memo absolute path (user-supplied)
PLAN_MEMO="$1"  # or substitute the absolute path
wc -l "$PLAN_MEMO"

# Resolve repo root for Step 2 agent prompts (axes.md absolute path placeholder)
REPO_ROOT=$(git rev-parse --show-toplevel)
test -f "$REPO_ROOT/.claude/skills/elidex-review/axes.md" || { echo "axes.md missing at $REPO_ROOT/.claude/skills/elidex-review/axes.md" >&2; exit 1; }
```

Plan-memo size > 1000 行なら user 確認 (通常 ~200-500 行)。

### Step 1.5 — Mental dry-run

`workflow.md` § "Step 1.5" を適用、output を `/tmp/elidex-plan-review.dry-run.md` に。**対象は test 限定ではない** — workflow.md 通り plan-memo §E-N test cases AND plan body / §Implementation で言及される new code path that reads ECS components 両方を simulate、write-path が plan 内予定 OR 既存実装で wired か確認。後者を skip すると Axis 2 sub-check 2b coverage が弱化する (non-test read site の data-flow gap 見逃し)。

### Step 2 — Launch 5 agents in parallel

**同一 message 内 5 並列**。各 agent prompt:

```
Agent <N> — Axis <name> review (plan-memo).

Read ${REPO_ROOT}/.claude/skills/elidex-review/axes.md Axis <N> section.
(${REPO_ROOT} resolved in Step 1 via `git rev-parse --show-toplevel`; substitute the concrete absolute path before dispatching the agent.)

Apply Axis <N> Detect entries tagged [plan] or [both] to <plan-memo path>.

<Agent 2 only>: Also read /tmp/elidex-plan-review.dry-run.md and incorporate gaps into Sub-check 2b findings.

Output per axes.md Axis <N> "Output format". Use plan-memo §section identifiers (e.g. `plan-memo §C-3`) instead of file:line where the finding refers to a plan section. Severity per axes.md common calibration. Acceptable exceptions per axis. Do NOT propose fixes. Report findings count by severity at end.
```

| Agent | Axis |
|-------|------|
| 1 | Axis 1 — Layering mandate |
| 2 | Axis 2 — ECS-native lens (+ dry-run) |
| 3 | Axis 3 — Pragmatic shortcut |
| 4 | Axis 4 — Spec citation |
| 5 | Axis 5 — Project-context |

### Step 3 / 3.5 / 4

`workflow.md` § "Step 3" / "Step 3.5" / "Step 4" 参照。Step 3.5 の `Fix decision F<N>` block の `Concrete action` 欄は plan-memo edit OR prerequisite PR carve-out のいずれか。

### Step 5 (plan-review specific) — Plan-memo edit + re-review

User が Fix decisions を accept した場合:

1. Plan-memo edit (Step 3.5 block の `Concrete action` に従って)
2. **適用した各 fix を `workflow.md` § "Step 4.5 — Fix-delta re-verification" で分類**。plan-stage は blast-radius 最大ゆえ **design-affecting fix は即 focused 再レビュー**（触れた axis × 該当 §section のみ・fix を書いていない fresh agent・detect-only）。plan fix は plan 内で compound する（後続 design 判断が乗る）ので cumulative でなく即時。
3. 変更タイプによる分岐:
   - **clerical のみ** (citation / wording / scope-doc) → re-review skip 可、implementation 着手 OR prereq PR carve-out
   - **design-affecting (localized)** → Step 4.5 focused 再レビュー (clean まで Step 3.5/4 を回す) → 着手
   - **structural (plan §section 構造変化)** → **full re-review 必須**: 修正 plan-memo で Step 1 から再走
4. Prerequisite PR carve-out 決定なら別 plan-memo 立てて `/elidex-plan-review`、元 plan は prereq merge 後再 invoke

⚠️ Step 5 の re-review skip (clerical 以外) は **anti-pattern** (`workflow.md` § "Step 4.5" / "Anti-patterns"、D-29 trial precedent)。

## Recommendation phrasing (skill-specific)

- **CRIT**: fix plan-memo BEFORE implementation start (implementation で必ず後悔する architectural error)
- **IMP**: plan-memo 修正推奨 (implementation 段階発覚は revert / re-plan cost 高、特に Axis 2 sub-check 2b data-flow integrity)
- **MIN**: judgment (plan-memo 修正 OR implementation 段階で監視)
- **FP**: ignore (user 確認後)

**Pre-impl gate**: 0 CRIT + 0 IMP → implementation start 推奨 / 1+ CRIT → plan-memo 修正 mandatory (implementation 着手前) / 1+ IMP → user 判断。
