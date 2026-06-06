# /external-review project overlay — elidex

Loaded by `~/.claude/skills/external-review/SKILL.md` (single-pass triage) when invoked from this repo. Provides project-specific calibration (reviewer identity / fetch / classify / layer-fit / fix-discipline). The full multi-round loop variant's calibration (round-count `failure_modes`, `wakeup_median`, TERMINAL accounting — Copilot-only) lives in `copilot-converge/project.md`.

## repo

`send/elidex`

## build_verify

`cargo fmt --all && mise run ci`

Per CLAUDE.md "Push 前: mise run ci". cargo task は `--all-features` で gate されているので feature-gated code (`#![cfg(feature = "engine")]` 等) も含めて回る (CLAUDE.md "Workflow" 参照)。

## layering

Reference: CLAUDE.md § "Layering mandate (2026-05-04 incident 由来)". Used by SKILL.md Step 4 (downward-drift screen).

### paths

- `crates/script/elidex-js/src/vm/host/`

### api_names

Beyond marshalling use of these APIs triggers downward drift signal:

- `EcsDom::traverse_descendants`
- `EcsDom::find_by_id`
- `EcsDom::with_attribute`

Acceptable marshalling use: prototype install / brand check / `JsValue` ↔ `Entity` marshalling / 単純 attribute read / wrapper 生成。NOT acceptable: DOM mutation / selector matching / form validation / live-collection walker / label association / constraint validation algorithm bodies inside `vm/host/`.

### incident_memo

`m4-12-architectural-drift-incident.md` (PR #151, 2026-05-04 — drift detected late; lesson #145). In single-pass mode the 3-rounds-same-file signal is gone, so the *static* triggers (new loop/walker/state-machine in `paths`, non-marshalling `api_names` call) carry the whole downward-drift screen — apply them on the one pass.

## fix_discipline

Applied at SKILL.md Step 5.1 (fix planning) — per-fix lens. SSoT: `<repo>/.claude/skills/elidex-review/workflow.md` (Step 3.5 + Step 4.5).

**Step 5.1 (per-fix)**: apply Step 3.5 "Philosophy alignment" — symptom vs root through CLAUDE.md "ideal over pragmatic" + "設計優先" + "ECS-native first". The reviewer's obvious patch is usually the smallest symptom-fix (sort / guard / cast); prefer a structural fix where the invariant holds by construction. Polish-domination smell → re-derive.

Precedent (PR #213, 2026-05-20): R2 flagged nondeterministic `HashMap` callback order; reactive patch was per-site `sort_by_key`. Philosophy-ideal was `BTreeMap` keyed by monotonic observer id — registration-order delivery as a *structural* invariant.

**Step 5.5 fix-delta re-verify**: external-review findings are frequently *symptom-shaped* ("add a guard", "handle this case"), and the fix delta never re-enters the pre-push `/elidex-review` philosophy gate. So when any fix this pass is symptom-shaped OR touches `layering.paths`, run one `/elidex-review` over the fix delta before the merge proposal — **Trigger B is the acute external-review-specific risk** (see workflow.md Step 4.5). New findings → resolve before merge.

## classification_calibration

Past elidex incidents that calibrate Step 2 severity (these survive single-pass; the round-count lessons moved to `copilot-converge/project.md`):

- PR #154 (2026-05-05) — ~50% of IMP were miscalibrated polish → apply the one-sentence "what concretely breaks?" test strictly; doc imprecision that doesn't misdirect is MINOR.
- Layer-confusion FPs (broker-register-ack, lessons #135-141; PR #151, lesson #145) → Step 4 upward/downward drift screen, both directions.

## reviewer

elidex's non-Claude diversity reviewer (Step 1 fetch filter / head-staleness / re-trigger):

- `bot_login`: `chatgpt-codex-connector[bot]`
- `name`: Codex (genuine OpenAI Codex Cloud, ChatGPT **Plus**-billed — *not* GitHub Copilot credits)
- `trigger`: `@codex review` (or Codex auto-review, enabled at chatgpt.com/codex)

**Identity caveat**: only the `chatgpt-codex-connector[bot]` review is the genuine Plus-billed Codex. A bare `@codex[agent]` mention instead routes to a GitHub-Copilot SWE agent (runs on `api.individual.githubcopilot.com`, billed as Copilot credits) — do **not** use it. The elidex review lenses reach Codex via `AGENTS.md` (`## Review guidelines` → `axes.md`). Quality validated on #295 (P1 GC-rooting + side-store/unbind panic + spec-accurate WebCrypto findings, catching gate-misses). Background → `memory/project_ai-review-setup.md`.
