# /copilot-review project overlay — elidex

Loaded by `~/.claude/skills/copilot-review/SKILL.md` when invoked from this repo. Provides project-specific calibration for the generic skill.

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

Applied at SKILL.md Step 5.1 (fix planning), before settling a finding's patch.

Reuse `/elidex-review` **Step 3.5 "Philosophy alignment"** (SSoT: `<repo>/.claude/skills/elidex-review/workflow.md` § "Step 3.5"): for each fix, re-evaluate **symptom vs root** through CLAUDE.md "ideal over pragmatic" + "設計優先 (場当たり的 reactive fix 禁止)" + "ECS-native first". A Copilot finding's obvious patch is usually the smallest symptom-fix (add a sort / guard / attribute / cast); prefer a structural fix that makes the invariant hold **by construction** (existing abstraction / data-structure choice / restructure) over a per-site reactive patch. Polish-domination smell: if every option is symptom-level, suspect the framing and re-derive.

Precedent (PR #213, 2026-05-20): R2 flagged nondeterministic `HashMap` callback order; the reactive patch was "add `sort_by_key` at each enumeration site". The philosophy-ideal was a `BTreeMap` keyed by the monotonic observer id — registration-order delivery becomes a *structural* invariant (no sort to forget, the exact omission Copilot flagged). The reactive patch shipped through TERMINAL and needed a follow-up to reach the ideal; this lens at Step 5.1 would have reached `BTreeMap` at R2.

**TERMINAL fix-delta pass (Step 4.5 applied at the R-loop end).** Step 5.1's lens is self-applied by the convergence-biased orchestrator, so it is weak; the R-loop's accumulated fixes also never re-enter the pre-push `/elidex-review` philosophy gate (that gate ran on the *pre-Copilot* diff). So: at **TERMINAL (SKILL.md Step 5.5), before surfacing the merge proposal**, classify every R-loop fix per `<repo>/.claude/skills/elidex-review/workflow.md` § "Step 4.5". If ANY fix across R1..Rn was **design-affecting** (type / data structure / invariant / algorithm / scope / premise), run **one cumulative focused `/elidex-review` pass over the R-loop delta** (`git diff <first-R-loop-commit>^..HEAD`) — touched axes only, fresh detect-only agent. A new finding → resolve before proposing merge (counts as a normal round). **clerical-only R-loops** (citation / wording / cfg-gate / comment — e.g. PR #222 R3+R4) **skip this pass.** Code-stage delta is batched at merge (not per-round) because code fixes are more independent than plan fixes; the irreversible merge is the natural gate.

## failure_modes

Each line: incident → operative rule that landed in `~/.claude/skills/copilot-review/SKILL.md`.

- broker-register-ack (slot #10.6c, lessons #135-141) — 8R on layer-confused goal → **Step 3.5 (1) upward drift**.
- PR #151 (lesson #145, `m4-12-architectural-drift-incident.md`) — 4R × 17 IMP before downward drift detected → **Step 3.5 (2) downward drift**.
- PR #154 R1-R9 (2026-05-05) — ~50% IMP miscalibrated as polish, 2 false scope-creep alerts → **Step 2 severity calibration**.
- PR #163 R1-R17 (2026-05-08) — 5k LoC budget upper-bound exceeded by 2× without scope creep → **Step 4 trigger #4** (LoC-scaled).
- PR #163 R29 (workflow-log misread), R30 (`first: 100` page-2 truncation), R31 (post-TERMINAL over-loop), 2026-05-08 — **Step 1 pitfall gate + Step 4 TERMINAL stop**.
- PR #201 R9 (2026-05-17) — pre-request review counted as fresh round, real R10 with IMP arrived later → **Step 1 request-staleness gate**.
- PR #213 R2 (2026-05-20) — reactive `HashMap`+per-site `sort` patch shipped through TERMINAL; philosophy-ideal was `BTreeMap` (structural delivery order) → **Step 5.1 design-philosophy lens** (`fix_discipline` overlay).

## wakeup_median

`3:30` — default, no override applied.

Calibration data (4 rounds observed 2026-05-19/20):

| Round | request → reviewed |
|---|---|
| PR #208 R2 | 5:09 |
| PR #208 R3 | 5:59 |
| PR #209 R1 | 2:03 |
| PR #209 R2 | 2:20 |

Range ~2:00–6:00, bimodal (small PRs ~2:00 / larger PRs ~5:30). The 3:30 mean approximates well; the formula's 90s polling fallback covers the variance without needing a project-specific override.

## copilot_bot_id

`BOT_kgDOCnlnWA` — verified for `send/elidex`. The SKILL.md default `BOT_kgDOCnlnWA` matches.
