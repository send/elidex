# /external-converge project overlay — elidex

Loaded by `~/.claude/skills/external-converge/SKILL.md` (the full multi-round convergence loop) when invoked from this repo. Provides project-specific calibration — `reviewer`, `wakeup_poll` / `wakeup_surface_threshold`, `layering`, `fix_discipline`, historical `failure_modes`. Reviewer = **OpenAI Codex on ChatGPT Pro** (loop-affordable, no per-credit cost). For the routine single-pass, see `external-review/project.md` (same reviewer).

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

The global SKILL.md reads these hooks from this section: the disposition SSoT, the per-fix lens, where the design's own ideals live, the early + TERMINAL fix-delta design gate, the altitude check's vocabulary, the escalation-gate statement, and the pre-Ask wording. Each is labelled below.

**Disposition SSoT (SKILL.md § "Disposition discipline")**: the rules + the canonical smell-word trigger-list are `<repo>/.claude/skills/elidex-review/workflow.md` Step 3.5 (git-tracked); the `memory/feedback_*` files are incident records, not the rule store. Loop convergence framing → `memory/feedback_review-loop-convergence-merit-not-fatigue.md`.

**Where the design's own ideals live (SKILL.md Trap 2 + Step 4 own-ideal test)**: the PR's plan-memo and CLAUDE.md "Development Rules" (e.g. *Ideal over pragmatic*, *Concurrency by ownership and phases*). Name the ideal verbatim from there.

**Escalation-gate statement (SKILL.md Step 4 "Escalation options")**: the canonical statement of "report + proceed; ask only a genuinely user-owned fork" is `elidex-review/workflow.md` Step 4.

**Root-check / FP references**: the ≥2-round self-root-check → `memory/feedback_cross-round-structural-root-check.md`; the by-construction FP-reject rule (SKILL.md Step 2 FP) → `memory/feedback_fp-reject-suspect-when-by-construction-cheap.md`.

**Step 5.1 (per-fix)**: apply Step 3.5 "Philosophy alignment" — symptom vs root through CLAUDE.md "ideal over pragmatic" + "設計優先" + "ECS-native first". The reviewer's obvious patch is usually the smallest symptom-fix (sort / guard / cast); prefer a structural fix where the invariant holds by construction. Polish-domination smell → re-derive. **Convergence rule**: real → fully fix (incl. real MINs — don't defer as "edge"); FP → reject. Stop on real-gap exhaustion, not round-count → `memory/feedback_review-loop-convergence-merit-not-fatigue.md`.

Precedent (PR #213, 2026-05-20): R2 flagged nondeterministic `HashMap` callback order; reactive patch was per-site `sort_by_key`. Philosophy-ideal was `BTreeMap` keyed by monotonic observer id — registration-order delivery as a *structural* invariant. Reactive patch shipped through TERMINAL and needed follow-up.

**Fix-delta design gate — TERMINAL (SKILL.md Step 5.5, before surfacing merge proposal)**: the gate is `/elidex-review`. Apply workflow.md Step 4.5 over the cumulative R-loop delta (`git diff <first-R-loop-commit>^..HEAD`). External-review findings are frequently *symptom-shaped* ("add a guard", "handle this case") so **Trigger B is the acute external-review-specific risk** — Step 5.1's lens is self-applied (convergence-biased) and the R-loop delta never re-enters the pre-push `/elidex-review` philosophy gate. If either trigger fires for any R1..Rn fix, run one cumulative `/elidex-review` pass; new findings → resolve before merge. Code-stage delta is batched at merge (workflow.md "Placement" — code fixes are more independent than plan fixes; irreversible merge is the natural gate). **Skipping this TERMINAL pass** is justified only when every axis has expected yield 0 — argue it per-axis per the expected-yield=0 table in `elidex-review/SKILL.md` "Skip OK" (single home; not restated here).

**Fix-delta design gate — early (SKILL.md Step 4 "Fire the design re-gate HERE")**: **Also fire this pass EARLY on persistence — not only at TERMINAL (#396, `feedback_external-review-symptom-patch-accumulates-ungated`)**: a *divergent* loop never reaches TERMINAL, so a TERMINAL-only design re-gate never runs while an ad-hoc edifice accumulates round over round. When the SKILL.md Step-4 ≥2-round self-root-check fires (the *same mechanism* patched across rounds — SKILL.md "Fire the design re-gate HERE, not only at TERMINAL"), run this cumulative fix-delta `/elidex-review` pass **then**, and test the accumulated mechanism against the plan-memo's / CLAUDE.md's OWN stated ideal (e.g. "construction **input**, not reconciled after the fact" / *Concurrency by ownership* = ownership-transfer / single-writer / latest-value), not just abstraction-coverage. PR #396: an 8-finding / 5-round `drain/replay` *was* the reconcile-after-the-fact the plan-memo §2 forbade — a TERMINAL-only re-gate would never have fired (the loop was diverging) and the per-fix lens kept answering "the mechanism is necessary." Distinct from the #374 generator-layer check below (that is *altitude* — B1 detail in a B0 doc; this is a fix *mechanism* that is itself the anti-pattern).

**Altitude check vocabulary (SKILL.md Step 4 "Generator-layer / altitude check" — #374 incident)**: the global rule (corner-hopping ≥3 rounds ⇒ written layer question before patching or proposing a scope-boundary merge) applies unchanged; this paragraph is elidex's layer vocabulary and residual grep. When the same finding-*shape* recurs across ≥3 rounds AND **hops between sub-areas** of the artifact (corner A fixed → corner B surfaces the *same shape* → corner C …, e.g. move-semantics → direct-delivery → boa-delivery → textContent), do **NOT** keep patching corner-by-corner AND do **NOT** surface a scope-boundary/merge proposal yet. First answer in writing: **"is a whole *layer / altitude* of this artifact generating these findings — and should that layer exist here at all?"** For a B0 *design-audit* doc the generating layer is usually **detailed per-instance behavioral characterization** (per-op event-order tables, per-API coalescing specifics, per-parent/per-arity classification, gap-tables) = **B1-altitude detail that does not belong in B0**. The fix is to **remove that layer** (collapse to named invariants + grep-diff methodology + B1 hand-off), NOT to keep correcting it and NOT to scope-boundary-merge with it intact — removal is *altitude correction* (the finding / two-mechanism model / named invariants / design-questions / methodology survive), not scope-cut. A self-grep for residual behavioral claims (`record|fire|callback|suppress|childList|same-parent|cross-parent|deliver|remove-then|insert-then` asserting per-instance behavior) must return **zero** before re-trigger. Skipping straight to "scope-boundary merge, residual is B1-precision" while the generator layer is still in the doc is the #374 failure (≈10 rounds of corner-hopping before the layer was removed). → `memory/feedback_design-audit-doc-convergence-scope-boundary.md` + `memory/feedback_review-fix-philosophy-first.md` (場当たり禁止 applied to *doc structure*).

**Pre-Ask wording (SKILL.md § "Disposition discipline" written-lens attestation — #374 incident, over-asked 2×)**: the global rule applies; elidex's wording and precedent: before ANY `AskUserQuestion` inside the loop, write one line: *"design-lens conclusion = X; does it converge to a single answer? Y/N"*. If it **converges**, do NOT ask — execute X (state it + act). Only ask when the lens genuinely does not converge (a real fork the user owns: scope/charter/merge-readiness). This mirrors the Step-4 TERMINAL written attestation — the forced written check is what makes the rule fire; the passive `memory/feedback_decide-via-philosophy-before-asking.md` note alone did not (#374 asked move-disposition twice when the lens had already converged on "remove the generator layer" / "stop prescribing the subtle rule"). Merge approval is still the user's, but reach it by *proposing + executing the lens result*, not by laundering the decision into a question.

## classification_calibration

Used by SKILL.md Step 2 (severity calibration). Codex badges: **P1 ≈ CRITICAL/IMPORTANT, P2 ≈ IMPORTANT/MINOR** — re-derive via the breaks-test anyway; don't trust the badge. A finding-free Codex formal-review body is ≈621 chars of boilerplate (the P-badge pattern, not body length, is what `body_finding_match` keys on). elidex-flavoured IMPORTANT examples beyond the generic list: **side-store/unbind panic** (alongside GC-rooting / use-after-free). Past miscalibration incidents: § failure_modes (PR #154).

## failure_modes

Historical **Copilot** R-loop incidents that calibrated the loop's defensive rules (now inherited by `external-converge`; the Codex pitfall-gate is simpler — OpenAI cloud, no workflow-log autofind / no `requestReviews` staleness). Each line: incident → operative rule.

- broker-register-ack (slot #10.6c, lessons #135-141) — 8R on layer-confused goal → **Step 3.5 (1) upward drift**.
- PR #151 (lesson #145, `m4-12-architectural-drift-incident.md`) — 4R × 17 IMP before downward drift detected → **Step 3.5 (2) downward drift**.
- PR #154 R1-R9 (2026-05-05) — ~50% IMP miscalibrated as polish, 2 false scope-creep alerts → **Step 2 severity calibration**.
- PR #163 R1-R17 (2026-05-08) — 5k LoC budget upper-bound exceeded by 2× without scope creep → **Step 4 trigger #4** (LoC-scaled).
- PR #163 R29 (workflow-log misread), R30 (`first: 100` page-2 truncation), R31 (post-TERMINAL over-loop), 2026-05-08 — **Step 1 pitfall gate + Step 4 TERMINAL stop**.
- PR #201 R9 (2026-05-17) — pre-request review counted as fresh round, real R10 with IMP arrived later → **Step 1 request-staleness gate**.
- PR #213 R2 (2026-05-20) — reactive `HashMap`+per-site `sort` patch shipped through TERMINAL; philosophy-ideal was `BTreeMap` (structural delivery order) → **Step 5.1 design-philosophy lens** (`fix_discipline` overlay).
- PR #367 + #374 (2026-06-20) — B0 design-audit doc (ScriptSession mutation-path), Codex ~14R total. Three failures + their new gates: **(a)** ad-hoc patched a finding-*generating* detailed-characterization layer with corner-hopping same-shape findings for ~10 rounds before questioning whether the layer belongs in B0 → **`fix_discipline` generator-layer/altitude check**. **(b)** `AskUserQuestion`'d 2× after the design-lens had converged → **`fix_discipline` pre-Ask written-lens attestation**. **(c)** merge-on-stale: #367 merged while Codex's latest review was on a pre-head commit → 8 real post-merge findings → follow-up PR → now a **mechanical block**: `~/.claude/hooks/gh-pr-merge-head-guard.sh` (PreToolUse Bash `gh pr merge`, denies unless Codex `assessed_commit` == PR head; override `# merge-stale-ok`) + the existing `reviewer.assessed_commit_marker` head-check. Net positive of the loop: it caught a spec-wrong move-record invariant (DOM §4.2.3 `suppressObservers` vs HTML §4.13.6 CE lifecycle). → `memory/feedback_design-audit-doc-convergence-scope-boundary.md`.


Codex-era incidents behind the global SKILL.md rules (the global states the mechanism; the evidence lives here). Each line: incident → operative rule.

- PR #295 (Codex, ~4 rounds / ~19 findings, converged — productive loop, real gate-misses caught: a P1 GC-rooting bug, a side-store/unbind panic, spec-accurate WebCrypto findings). Its misstep = **Trap 1 (fatigue/cost bias)**: treating a finding's disposition as a process decision → **SKILL.md "Disposition discipline"**.
- PR #396 — 8 findings / 5 rounds on a `drain/replay` mechanism that *was* the "reconcile after the fact" the plan-memo §2 forbade; the user had to ask "isn't this 場当たり?" after I had concluded "necessary, converging, continue" at R5 → **Trap 2 + Step 4 own-ideal test + early fix-delta gate** (`memory/feedback_external-review-symptom-patch-accumulates-ungated.md`).
- PR #491 — 17 h of zero activity on a fresh PR because the auto-review never fired → **SKILL.md first-round bootstrap: trigger yourself** (and § "Never wait on auto-review").
- `#316` / `#337` — GraphQL bare login vs REST `[bot]` → **Step 1 login-suffix quirk**.
- `#322` / `#337` — dry verdict is an issue-comment; a reviews-only head check false-stalled every dry round → **Step 1 dry-verdict-is-an-issue-comment**.
- `#339` — a real P2 clamp finding hid in a review *body* for many rounds until a maintainer audit → **Step 1 review-BODY findings**.
- `#488` — 9 rounds driven by hand, round arrival missed 3× (each channel alone a partial scan) → **Step 1 "one Bash call, all three channels"** + the `converge-skill-first-guard` hook.
- `#492` R1 — `## Review finding [IMP]` at `16:28:40Z`, then `Didn't find any major issues` at `16:28:48Z`; the finding survived only because a human read the earlier body → **Step 1 "latest wins answers ONLY the head question"**.
- `#494` R2 — resolver returned `dry_verdict: true` with three unresolved threads carrying real findings (the round's formal body was boilerplate; the only marker-bearing comment was the previous round's dry verdict) → **Step 1 dry_verdict covers two of three channels** + **group by invocation, not commit**.
- `#506` R88 — a marker-less, badge-bearing findings comment carrying 5 findings against `974476b4` went unseen ~10 h (`gh api --paginate repos/send/elidex/issues/506/comments` showed 5 marker-bearing comments + that 1) → **Step 1 "the marker is NOT a findings filter"**.
- `#506` R91 — every trigger posted within ~30 s of a push errored (18:07Z, 18:17Z, 18:35Z — 3/3) while a trigger with no push in the preceding minutes ran (18:23Z → review 18:30Z) → **Step 1 error shape: let the head settle before triggering**.
- `#510` R12 (2026-09-07) — the status table appeared: created 09:14:03Z (13 s after the push), edited to `Running since 09:17:25Z` after the 09:17:12Z trigger → **Step 1 status-widget shape**.
- `#510` R31 — the PR crossed 100 threads; an un-paginated `reviewThreads` call returned `unresolved = 0` and the round read as the first dry one; paging found FIVE live P2s (four of that round's, one from the round before, disposed of with a third of its findings unseen) → **Step 1 channel 1 paginated in code**.
- An earlier standalone body scan hard-coded `pulls/359` (a PR number copied from its incident) and scanned the wrong PR → **Step 1 channel 2 reads the same paginated fetch**.
- `#331` — a late review landed on the unchanged head during the TERMINAL→approval gap → **Step 5.5 merge-time re-poll** (`memory/feedback_external-review-merge-on-head-not-stale.md` #331 追補). Also 3 silent trigger-fails in one loop, each recovered by re-posting → **silent-window recovery**.
- `#492` (counter) — three review rounds burned on the reset / retry-admission / threshold holes of a codified re-post counter before it was deleted → **silent-window recovery is judgement, not a counted state machine**.

## wakeup_poll

`300s` — poll cadence while waiting for Codex's review to land, **NOT a latency prediction**.

## wakeup_surface_threshold

`~45min` — how long one `@codex review` trigger is given before you treat it as dropped (SKILL.md Step 5.5 (2)). It counts **from the trigger comment**, never from PR creation or push; see § "Never wait on auto-review" below for what to do when it expires. Deliberately set **well above** Codex's typical review latency so a normal-but-slow review is never cut off mid-flight, and it overrides the generic SKILL.md `~15 min` default (far too aggressive for this reviewer).

### Never wait on auto-review — always fire one trigger

> **CANONICAL SITE.** The single home of the elidex trigger policy for both review workflows. `external-review/project.md` references it instead of restating it. Both global `SKILL.md` files carry only the repo-independent rule (trigger yourself; zero assessment on head ⇒ post the trigger; a reaction is not a verdict) and read this section through `reviewer.auto_review_trusted` / `reviewer.non_verdict_signals`.

**Do not trust auto-review. Post `@codex review` yourself for the head you want reviewed, and time everything from that comment.** PR creation and pushes are not triggers. If a trigger for the current head already exists — posted by an earlier session or by anyone else — that one counts; adopt its timestamp rather than posting a second.

Auto-review is not guaranteed to arm and gives no signal when it doesn't: `#333` sat at zero ~50 min; `#491` sat at zero for **17 hours** on a fresh PR while Codex reviewed a sibling PR in the same repo 1 min after the cap. There is no way to distinguish "unarmed" from "slow" by observation, so waiting to find out is a pure loss.

The duplicate-trigger cost is negligible and empirically harmless — `#488`/`#489` both received manual triggers on top of already-completed auto-reviews with no ill effect. The asymmetry is decisive: a redundant review costs one wasted Codex run, an unarmed review costs unbounded wall-clock. Always trigger.

This is *One issue, one way* (CLAUDE.md): the rule removes the "is it slow or unarmed?" branch entirely rather than moving the decision to a threshold.

**No comment and no review on head means not reviewed, whatever the reactions or the status table show.** Codex's status-table legend reads: 👀 while any review is running, 👍 once all reviews finish with no findings. An auto-review can therefore end with only a 👍, and Step 1 then reports `on_head: false`. Seen on `#524` at 2026-09-27T20:02Z (auto-review on PR open: the status row read `Completed`, a bot 👍 was present, no comment). A later re-trigger overwrote that state on the PR, so the details are recorded in memory `project_codex-silent-dry-resolver`. `#523` is consistent with it: its 👍 at 12:50:31Z comes about 6 min after the last dry comment, and 2 s after a status-table update, with no comment after it. Do not read such a 👍 as the verdict. Trigger as above. In every case checked, a manual `@codex review` posted the explicit dry comment, which the resolver reads. The cases are `#508`, `#519`, `#523`, and `#524`'s re-trigger (dry comment at 2026-09-27T20:29:42Z, under 2 min). If a manual trigger ever ends with only a 👍, that is a new shape. The only thing that tells it apart from a dropped trigger is a bot 👍 dated after the trigger. When you see one, surface it to the user instead of re-posting under the `wakeup_surface_threshold` bullet below.

**Observed latency (user-confirmed 2026-07-18): a Codex review commonly takes ~30 minutes to land — and the loop was giving up around ~15 min far too often.** Range ≈ ~15–30 min, sometimes longer; the earlier "~15 min normal" [#390, 2026-06-21] was the *low* end (one review), the prior "~2 min" [#288] a *manual-trigger one-off*, and #295's ~30–90 min round gaps were *fix-time between rounds* [Claude fixing], not review latency. So:

- **~30 min is NORMAL, not stuck** — *once you have triggered*, do **not** re-trigger or "surface as slow" anywhere in the ~15–30 min window after that trigger. The review is almost certainly still running, and a second trigger interrupts/duplicates a live review rather than recovering a stuck one (#390 re-triggered at ~14 min into a still-running review — premature; this early-give-up is the failure this window protects against). This window is about the **second** trigger; it never justifies withholding the **first** one.
- **Poll patiently** — 300s cadence is fine (not-spamming the reviewer matters more than cache warmth here; a ~30 min wait needs only a handful of polls, so widen toward 600s if preferred). `ScheduleWakeup` IS the poll.
- **`wakeup_surface_threshold` (~45 min) counts from the trigger, and a trigger that draws nothing by then has most likely been dropped.** Silent trigger-fail is a known Codex failure mode and its recovery is simply to post the trigger again (`#331`: 3 in a row inside one loop, every one recovered by re-posting). So re-post and keep polling; if repeated triggers keep drawing nothing, say so to the user and let them decide.

  **This is deliberately guidance, not a counted state machine.** No evidence distinguishes stopping at 3 re-posts from 5 from "keep going", so a codified counter would be inventing precision the observations do not support — and the earlier attempt to codify one (exact threshold, reset rules, an admission condition for retry triggers) is what generated 6 of this PR's 9 review findings. Judge it in the moment and keep the user informed.

**⚠ The ~30 min figure above is `@codex review`-to-verdict latency measured with auto-review in the mix. Manually-triggered rounds measured on 2026-07-28 were far faster — 2m25s, 3m31s, 2m23s (#491 R1/R2, #492 R1), i.e. every sample under 4 min.** Three samples is thin and the ~30 min number is user-confirmed, so the threshold is deliberately left at ~45 min rather than retuned here; but if further manual-trigger rounds stay in the minutes range, the window is ~10× the real latency and shortening it (which also speeds the re-post recovery above) is worth raising with the user. Do not silently retune it on this data alone.

→ `memory/feedback_external-converge-codex-latency.md` (latency) + `memory/feedback_codex-dry-verdict-is-issue-comment.md` §追補 #331/#333 (unarmed auto-review + silent trigger-fail; mechanical re-post is the recovery).

## reviewer

- `bot_login`: `chatgpt-codex-connector[bot]` (REST form). **GraphQL `reviewThreads` author.login is the BARE `chatgpt-codex-connector`** (no `[bot]`) — the Step-1 fetch must normalize (strip `[bot]`) for GraphQL comparisons or it false-negatives every inline finding (`#316`/`#337`).
- `name`: Codex (OpenAI Codex Cloud, ChatGPT **Pro** — loop-affordable, no per-credit cost)
- `trigger`: `@codex review` (posted as a PR comment to re-trigger each round)
- `error_verdict_match`: `Codex Review: Something went wrong` — anchors on the comment's opening, so a finding that merely quotes the phrase is not counted as an error. Codex's error comment (`Codex Review: Something went wrong. Try again later by commenting "@codex review"`): no marker, no badge, no blob link. The trigger was consumed and failed; re-post at once (SKILL.md Step 1 error shape; evidence § failure_modes `#506` R91).
- `status_comment_match`: `codex-pull-request-review-summary` — the HTML-comment tag of Codex's "Codex Review Summary" status table (`Review | Status | Commit | Review trigger`, rows like `🔄 Running since <ts>` / `Manual request`), created on the push and edited in place (SKILL.md Step 1 4th shape; evidence § failure_modes `#510` R12).
- `body_finding_match`: `shields\.io/badge/P[0-9]` (regex: escapes the dot, and does not match other badges such as `badge/PR-…`) — Codex renders each line-anchored body finding with a `![P-badge](shields.io/badge/P1…)` (see § classification_calibration). The global ORs this with its always-on generic `/blob/<sha>/…#L…` detector, so it can only add matches. Measured 2026-10-31: of `#510`'s 75 Codex review bodies (`gh api --paginate repos/send/elidex/pulls/510/reviews`, filtered to the bot) neither pattern matched any (0/75 and 0/75), so on that PR the union adds no false findings.
- `non_verdict_signals`: the 👍 reaction and the status table's `Completed` row. **Canonical rule: § "Never wait on auto-review" → the "No comment and no review on head" paragraph** (#525). Surface-to-user shape: a manual trigger for the current head got no response until `wakeup_surface_threshold`, and only a bot 👍 appeared after that (no comment). An ordinary auto-review 👍 after a fix push is not this shape and does not stop the loop.
- `auto_review_trusted`: `false` — never wait on Codex's auto-review (§ "Never wait on auto-review").
- Cost model: ChatGPT **Pro** — loop-affordable, no per-credit cost, so cost is not even a choose-time argument against `/external-converge` here (*not* GitHub Copilot credits).
- `assessed_commit_marker`: `Reviewed commit:` — appears in BOTH formal-review bodies AND Codex's dry-verdict issue-comment, followed by `` `<sha>` ``. Step 1 reads the reviewer's latest assessed commit from this marker across reviews + issue-comments (NOT the reviews API alone).
- `dry_verdict_match`: `Didn't find any major issues` — Codex's no-findings verdict, posted as a **plain PR issue-comment** (`Codex Review: Didn't find any major issues`), **not** a formal review. Keying head-staleness on `pulls/{n}/reviews` alone false-stalls every dry round (`#322`/`#337`). **CANONICAL SITE for how to read this verdict** (`external-review/project.md` references this bullet rather than restating it):
  - ⚠ **It is never decisive on its own.** Codex can post a findings item and this verdict seconds apart (`#492` R1 — `## Review finding [IMP]` at `16:28:40Z`, the dry verdict at `16:28:48Z`), so trusting the verdict discards the finding.
  - ⚠ **Group by review invocation, not by commit.** A re-trigger on an unchanged head shares its head with the previous round; keying on the commit alone keeps re-counting that round's findings, so a later genuinely-clean verdict can never register and the converge loop cannot terminate (`#494` R2). Admit only items newer than the trigger that opened the round.
  - ⚠ **It covers two of three channels.** The resolver reads reviews + issue-comments; inline threads are fetched separately. `dry_verdict == true` is necessary, never sufficient — union it with `reviewThreads` and the P-badge body scan (`#494` R2 returned `dry_verdict: true` with three unresolved threads carrying real findings).
  - Mechanism: `external-{review,converge}/SKILL.md` Step 1 — `assessed_commit` is latest-wins, `dry_verdict` is not. ⚠ Those SKILL files are **user-level and untracked**, so this repo cannot guarantee an environment has the corrected resolver; see `memory/project_untracked-global-skill-distribution.md`. (`memory/feedback_codex-dry-verdict-is-issue-comment.md`.)

Lenses reach Codex via `AGENTS.md` (`## Review guidelines` → `axes.md`). The genuine Pro Codex is `chatgpt-codex-connector[bot]`; a bare `@codex[agent]` mention is a Copilot-billed impostor — do **not** use. Background → `memory/project_ai-review-setup.md`.
