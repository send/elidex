# /external-converge project overlay — elidex

Loaded by `~/.claude/skills/external-converge/SKILL.md` (the full multi-round convergence loop) when invoked from this repo. Provides project-specific calibration — `reviewer`, `wakeup_median`, `layering`, `fix_discipline`, historical `failure_modes`. Reviewer = **OpenAI Codex on ChatGPT Pro** (loop-affordable, no per-credit cost). For the routine single-pass, see `external-review/project.md` (same reviewer).

## repo

`send/elidex`

## build_verify

`cargo fmt --all && mise run ci`

Per CLAUDE.md "Push 前: mise run ci". cargo task は `--all-features` で gate されているので feature-gated code (`#![cfg(feature = "engine")]` 等) も含めて回る (CLAUDE.md "Workflow" 参照)。

## layering

Reference: CLAUDE.md § "Layering mandate (2026-05-04 incident 由来)".

### paths

- `crates/script/elidex-js/src/vm/host/`

### api_names

Beyond marshalling use of these APIs triggers downward drift signal:

- `EcsDom::traverse_descendants`
- `EcsDom::find_by_id`
- `EcsDom::with_attribute`

Acceptable marshalling use: prototype install / brand check / `JsValue` ↔ `Entity` marshalling / 単純 attribute read / wrapper 生成。NOT acceptable: DOM mutation / selector matching / form validation / live-collection walker / label association / constraint validation algorithm bodies inside `vm/host/`.

### incident_memo

`m4-12-architectural-drift-incident.md` (PR #151, 2026-05-04 — 4R × 17 IMP findings before downward drift detected; lesson #145).

## fix_discipline

Applied at SKILL.md Step 5.1 (fix planning) — per-fix lens. SSoT for both: `<repo>/.claude/skills/elidex-review/workflow.md` (Step 3.5 + Step 4.5).

**Step 5.1 (per-fix)**: apply Step 3.5 "Philosophy alignment" — symptom vs root through CLAUDE.md "ideal over pragmatic" + "設計優先" + "ECS-native first". The reviewer's obvious patch is usually the smallest symptom-fix (sort / guard / cast); prefer a structural fix where the invariant holds by construction. Polish-domination smell → re-derive. **Convergence rule**: real → fully fix (incl. real MINs — don't defer as "edge"); FP → reject. Stop on real-gap exhaustion, not round-count → `memory/feedback_review-loop-convergence-merit-not-fatigue.md`.

Precedent (PR #213, 2026-05-20): R2 flagged nondeterministic `HashMap` callback order; reactive patch was per-site `sort_by_key`. Philosophy-ideal was `BTreeMap` keyed by monotonic observer id — registration-order delivery as a *structural* invariant. Reactive patch shipped through TERMINAL and needed follow-up.

**TERMINAL fix-delta pass (Step 5.5, before surfacing merge proposal)**: apply workflow.md Step 4.5 over the cumulative R-loop delta (`git diff <first-R-loop-commit>^..HEAD`). External-review findings are frequently *symptom-shaped* ("add a guard", "handle this case") so **Trigger B is the acute external-review-specific risk** — Step 5.1's lens is self-applied (convergence-biased) and the R-loop delta never re-enters the pre-push `/elidex-review` philosophy gate. If either trigger fires for any R1..Rn fix, run one cumulative `/elidex-review` pass; new findings → resolve before merge. Code-stage delta is batched at merge (workflow.md "Placement" — code fixes are more independent than plan fixes; irreversible merge is the natural gate).

## failure_modes

Historical **Copilot** R-loop incidents that calibrated the loop's defensive rules (now inherited by `external-converge`; the Codex pitfall-gate is simpler — OpenAI cloud, no workflow-log autofind / no `requestReviews` staleness). Each line: incident → operative rule.

- broker-register-ack (slot #10.6c, lessons #135-141) — 8R on layer-confused goal → **Step 3.5 (1) upward drift**.
- PR #151 (lesson #145, `m4-12-architectural-drift-incident.md`) — 4R × 17 IMP before downward drift detected → **Step 3.5 (2) downward drift**.
- PR #154 R1-R9 (2026-05-05) — ~50% IMP miscalibrated as polish, 2 false scope-creep alerts → **Step 2 severity calibration**.
- PR #163 R1-R17 (2026-05-08) — 5k LoC budget upper-bound exceeded by 2× without scope creep → **Step 4 trigger #4** (LoC-scaled).
- PR #163 R29 (workflow-log misread), R30 (`first: 100` page-2 truncation), R31 (post-TERMINAL over-loop), 2026-05-08 — **Step 1 pitfall gate + Step 4 TERMINAL stop**.
- PR #201 R9 (2026-05-17) — pre-request review counted as fresh round, real R10 with IMP arrived later → **Step 1 request-staleness gate**.
- PR #213 R2 (2026-05-20) — reactive `HashMap`+per-site `sort` patch shipped through TERMINAL; philosophy-ideal was `BTreeMap` (structural delivery order) → **Step 5.1 design-philosophy lens** (`fix_discipline` overlay).

## wakeup_median

`3:00` — Codex (OpenAI cloud) review latency from the `@codex review` trigger to `chatgpt-codex-connector[bot]` posting. Observed ~2:00 (#288). **Note**: #295's ~30–90 min round gaps were *fix-time between rounds* (Claude fixing), NOT review latency — don't recalibrate off them. The skill's 90s polling fallback covers variance.

## reviewer

- `bot_login`: `chatgpt-codex-connector[bot]`
- `name`: Codex (OpenAI Codex Cloud, ChatGPT **Pro** — loop-affordable, no per-credit cost)
- `trigger`: `@codex review` (posted as a PR comment to re-trigger each round)

Lenses reach Codex via `AGENTS.md` (`## Review guidelines` → `axes.md`). The genuine Pro Codex is `chatgpt-codex-connector[bot]`; a bare `@codex[agent]` mention is a Copilot-billed impostor — do **not** use. Background → `memory/project_ai-review-setup.md`.
