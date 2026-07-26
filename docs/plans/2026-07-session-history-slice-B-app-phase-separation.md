# Plan-memo — Slice B: app-mode (legacy inline) phase-separation

> Status: **pre-implementation DESIGN plan-memo**. Goes through `/elidex-plan-review` before any
> implementation. Anchored on the first-principles spec-faithful ideal, not a surgical patch.
> **This is edge-dense work** (≥3 intersecting invariant axes — the umbrella §2 matrix, restated
> app-specific in §2 here).
>
> **Umbrella (READ FIRST, not restated here):**
> `docs/plans/2026-07-session-history-task-queue-model.md` — §0 (SLICING CORRECTION), §1 (the decisive
> task-timing invariant), §2 (the 5-axis edge matrix a–e), §4 (queue design + §4.5 I1/I2/I3), §7 (resolved
> Q-OWNER / Q-SCHED / Q-VM-MODEL / Q-SYNC-FINALIZE / Q-PEEK / Q-FENCE).
>
> **Slice A (the MIRROR this slice follows):**
> `docs/plans/2026-07-session-history-slice-A-content-phase-separation.md` — the content-mode co-design.
> Slice B is the app-mode leg of the **same** shared primitive. It adds **no new substrate** — the
> Resolution A/B/D/E machinery (nav-vs-traversal supersede · pending-traversal default-suppression ·
> deferred-`SyncUpdate` cancel · no-op peek-classify) already lives in `elidex-navigation` (Slice A shipped
> it). Slice B **only** implements `DrainHost` for the app-mode shell and drives the coordinator's
> **same-turn** entry point. If anything in this memo proposes a substrate change, it is a mistake — flag it.
>
> **Substrate this drives (DO NOT modify):**
> `crates/shell/elidex-navigation/src/traversal_queue.rs` — `TraversalDelta` / `UserInvolvement` /
> `PendingTraversal` / `PendingHistoryStep` / `TraversalQueue` / `DrainOutcome` / `DrainHost` /
> `DrainCoordinator`. The `DrainCoordinator::drain_same_turn` method is **already documented as "the
> app-mode-degenerate / atomic same-turn drain"** (on `DrainCoordinator::drain_same_turn` itself) — Slice B is its first real
> consumer.

---

## §0 Decision + scope

**Thesis (the ideal — anchor the whole memo on this).** After Slice A, elidex runs **two divergent
navigation drains** over one shared `NavigationController`: content-mode drives the shared `DrainCoordinator`
(`content/drain_host.rs`), while app-mode still runs a **hand-rolled synchronous single-drain**
(`app/navigation.rs::process_pending_navigation`, with a traversal-supersede `return true` mid-FIFO). That
fork is the exact strangler middle-state One-issue-one-way forbids (umbrella §2 axis c). **Slice B closes it:
`impl DrainHost for` the app-mode shell and drive `DrainCoordinator::drain_same_turn` at end-of-input-handler.
Both shells then drive the *identical* shared queue primitive; what remains different is the SCHEDULE, most
visibly WHEN Phase 2 is pumped** — content-mode on a later async-pump turn (`run_deferred_traversals`), app-mode
back-to-back inside the input handler (`drain_same_turn`). The two schedules are *not* mirror images: content-mode
drives the coordinator from FIVE sites — three on its async pump (`run_deferred_traversals` →
`drain_synchronous_updates` → the bottom `drain_synchronous_phase`) plus one INSIDE each input handler
(`content/event_handlers.rs`) — while app-mode has only the in-handler one. So what app-mode lacks is the
PUMP, and with it any counterpart to content's post-Phase-2
synchronous settle (§4.2, slot `#11-app-mode-turn-completion-drain`). **That uniformity of the primitive IS the
design contribution.** The `process_pending_navigation`
hand-rolled body — its window-open drop, its history FIFO, its two `return`s — dissolves into the
coordinator's phase partition; the app-specific *bodies* (pipeline rebuild, same-document step, frame render)
stay **behind the `DrainHost` trait**, never crossing into `elidex-navigation`.

**Q-SCHED is RATIFIED — do not re-open (umbrella §7).** App-mode has **no async pump** (`app/events.rs::handle_click` /
`::handle_keyboard` call `process_pending_navigation` synchronously right after event dispatch; there is no
`run_event_loop` equivalent). The ratified resolution is **option (i): app-mode drains its Phase-2 traversal
queue at the END of the same input handler, strictly AFTER Phase-1 synchronous updates complete** — a
*degenerate* two-phase. What §7.4.6.1 *apply the history step* step 12 needs for a **single top-level
traversable** is the **ordering** guarantee ("synchronous navigations processed before documents unload"),
NOT real task deferral; `drain_same_turn` gives exactly that ordering (Phase 1 body fully completes, then
Phase 2 drains). **Documented re-eval trigger (NOT a defect to fix now):** the B1 multi-navigable fan-out —
once multiple navigables must be sequenced across the step-12 unload boundary, an end-of-handler drain that is
not a *real* later task may violate §7.4.6.1 step-12 unload sequencing. That is the **fidelity boundary** this
slice consciously accepts (umbrella §7 Q-SCHED "re-eval at the B1 multi-navigable-fan-out landing"), fenced to
`#11-session-history-task-queue-model` alongside the rest of the B1 fan-out.

**Scope fence (plan-review verifies each leg).**
- **IN:** `impl DrainHost` for the app-mode shell + drive `DrainCoordinator::drain_same_turn` from the two
  input handlers + refine the `handle_click` / `handle_keyboard` default-suppression consumers + remove the
  `app/navigation.rs:73` traversal-supersede `return` + reshape the `traverse_to` peek-then-commit body into
  the delta-keyed `apply_traversal` seam. **No substrate change; no new reentrancy machinery** (I3 = vector
  dead by construction, §4.4).
- **OUT → Slice 4 (fenced):** the canonical §7.3.1.1 running-nested-apply guard **wiring** + `commit_index`
  `debug_assert` retirement + peek-then-commit reentrancy-workaround-framing retirement. App-mode does NOT
  need the interim SW-wait buffer guard content-mode carries (`dispatch_or_buffer_reentrant`) because its
  reentrancy vector is **absent by construction** (§4.4) — so app-mode adds **nothing** to Slice 4's canonical
  work. Slice 4 stays a separate later PR.
- **OUT → `#11-session-history-task-queue-model` (fenced):** the **two DIRECT user-input traversal entry
  points**, both of which bypass the `DrainCoordinator` by peeking `nav_controller` and calling `traverse_to`
  directly:
  1. **chrome-traverse** — `app/navigation.rs::handle_chrome_action` (its `Back`/`Forward` peek→`traverse_to`
     arm, plus address-bar Navigate and Reload);
  2. **Alt+←/→** — `app/inline.rs::handle_keyboard_inline`, which peeks `peek_back`/`peek_forward` and calls
     `traverse_to` **then `return`s before `handle_keyboard` runs**, so that turn never drains the coordinator
     at all.

  Slice B does **NOT** route either through the queue — both collapse into Slice 4's canonical DIRECT-nav
  serialization when M4-10 async-SW-fetch lands (the same fence content-mode's chrome-direct traversal sits
  behind, `content/drain_host.rs:64`–`:70`). Pulling them into the queue now is the failure mode to avoid.
- **OUT → `#11-sync-navigation-steps-queue-tagging` (fenced):** reentrant `SyncUpdate` straddle /
  jump-the-queue (§7.4.1.3) full reconciliation, inherited unchanged from Slice A (Resolution D ships the
  bounded *cancel*, not the call-time-entry jump-the-queue). App-mode inherits this bounded behavior verbatim
  from the shared coordinator — no app-specific work.
- **OUT → B1:** multi-navigable fan-out (§7.4.6.1 steps 3/4/6/7 + per-navigable global-task of 8/12);
  `changingNavigables` is always `{top-level}` (umbrella §0 fence). The Q-SCHED re-eval trigger above lands
  here.

**Deployment-shell note (why the fork is bounded, not a runtime bug — mirror Slice A §0).** Content and
app-mode are **distinct deployment shells, never both active at runtime**. The app-mode `InteractiveState`
drive runs **only** on the legacy-inline path (`App::new_interactive_with_url`, `#[allow(dead_code)]` /
test-support today — the unused `new_interactive` sibling was deleted with this slice); production entry points (`new_threaded*`) use only the content
path. So the Slice-A→Slice-B window where content drives the queue and app still forks is a **bounded
code-duplication strangler confined to legacy/test-only code, NOT a live production runtime fork** (Slice A
§0 E3). Slice B lands in close succession per the umbrella §5 landing-proximity constraint (axis c leg-1);
the leg-2 "gate the supersede-removal" option is already retired (Slice A §0 F2 — no single-user runtime fork
to gate). This memo neither reopens nor relaxes that.

---

## §1 The app-mode-specific decisive invariant — no async pump ⇒ degenerate two-phase (ordering, not deferral)

The umbrella §1 states the spec's task-timing model (a same-turn §7.4.4 synchronous *update* must land before
a same-turn §7.4.3 → §7.4.6.1 *traversal* observes the entry list). Content-mode realizes the task boundary
with a **real** later task (the async pump exposes the deferred apply only on a subsequent turn — I1
structural-by-construction, umbrella §4.5). **App-mode has no such turn.** Its restructure realizes the same
*ordering* invariant **degenerately**:

> **I1 (app-mode leg) — a call-ordering sequencing contract the shell honors.** Within a single input handler,
> `DrainCoordinator::drain_same_turn` runs the full Phase-1 body (`run_synchronous_phase_body`: window-opens →
> §7.4.4 sync updates in-task → §7.4.2 last-wins navigation) to completion, THEN `drain_traversal_queue`
> (Phase 2). Every Phase-1 write to `NavigationController.entries`/`index` lands before any Phase-2 traversal
> apply reads it — **strictly, sequentially, in the same handler.** There is no task boundary, so this leg is
> a **shell-enforced sequencing invariant** (a single method call whose body orders the two phases), NOT a
> by-construction property of the primitive the way content-mode's async-pump leg is. This is the **F1
> residual the Q-SCHED resolution explicitly accepts** (umbrella §4.5 I1 app-mode leg). Because the shared
> primitive gives both shells one drain-then-apply shape, the app-mode contract is a **one-line invariant**
> (call `drain_same_turn`, whose documented body sequences Phase 1 before Phase 2),
> not a per-shell re-derivation.

The decisive spec anchor (webref-verified, §3): §7.4.6.1 *apply the history step* step-12 note — *"This set of
steps are split into two parts to allow synchronous navigations to be processed before documents unload."* For
one top-level traversable there is no *cross-navigable* unload to sequence against, so what the split buys is
purely the **within-traversable ordering** (sync update lands, then traverse). `drain_same_turn` delivers that
ordering without a real deferral — which is *why* option (i) is spec-adequate for the single-traversable scope
and *why* the B1 multi-navigable landing is the documented re-eval trigger (multiple navigables reintroduce a
real cross-navigable unload sequence that a synchronous end-of-handler collapse cannot honor).

**How today's app-mode violates it (the removal target).**

> **Reading note — the `app/navigation.rs:NN` pointers in this subsection and in §4.1 name the
> PRE-Slice-B file** (`origin/main`, i.e. `git show origin/main:crates/shell/elidex-shell/src/app/navigation.rs`).
> They describe the body this slice DELETED, so they deliberately do not resolve against the working tree;
> every pointer to code that still exists was rewritten to a symbol name instead, so it cannot rot.

`process_pending_navigation` (`app/navigation.rs:34`)
drains everything in one pass: window-opens dropped (`:50`) → history FIFO (`:59`–`:75`) with a **traversal
`return true` supersede at `:73`** (a traversal in the FIFO stops replaying trailing intents AND short-circuits
the navigation phase) → last-wins navigation (`:89`–`:95`). The `:73` supersede is the **collapsed
§7.4.3-vs-§7.4.4 boundary** — the direct app-mode mirror of the content `:593` supersede Slice A removed
(umbrella §1 lists both at `content/navigation.rs:583–592` / `app/navigation.rs:73`). A same-turn
`pushState('/a'); history.back()` either supersedes on `back()` (discarding the trailing intent, #259) or — if
reordered — traverses against a half-mutated list, with no phase boundary distinguishing "sync landed, then
traverse" from "traverse, then sync." That is the E7 residual handed here.

---

## §2 Coupled invariants (the app-mode edge matrix — plan-review checks each axis independently)

Mirrors the umbrella's five axes, restated for the app-mode leg. The **≥3 intersecting axes** that force
plan-review are **(a) sync/traversal ordering × (b) app-no-pump-liveness × (c) the fork this slice closes**.

- **(a) Sync/traversal ordering (task-boundary phase-separation).** Phase 1 (window-opens + §7.4.4 sync
  updates + §7.4.2 last-wins nav) completes before Phase 2 (§7.4.3 → §7.4.6.1 traversal apply). *App-mode
  realization:* the sequential body of `drain_same_turn` (I1 app-leg, §1). *Failure mode:* a same-turn
  `pushState` lost to a `back()` supersede (the `:73` bug), or a traversal reading a half-updated list.
- **(b) App-no-pump-liveness (reentrancy / bounded-snapshot).** Content-mode's Phase-2 drains a **bounded
  snapshot** (steps present at drain-start) and relies on the **every-turn async pump** for liveness: a step
  serialized mid-apply drains on the *next* pump turn. **App-mode has no pump**, so a mid-apply-serialized
  step would strand indefinitely (app-mode pumps only when an input turn reaches the drive site). **This axis is the crux — §4.4 proves the reentrancy vector is
  DEAD by construction in app-mode, so the bounded snapshot drains completely every turn and nothing strands.**
  *Failure mode:* blindly adopting content's bounded drain *without* establishing the vector is absent, then a
  stranded step never applying.
- **(c) The content-queued / app-synchronous FORK this slice CLOSES.** Before Slice B: content drives the
  shared `DrainCoordinator`; app runs its own hand-rolled drain (`process_pending_navigation:34`). **Slice B
  makes app drive the SAME coordinator (`drain_same_turn`)** — the One-issue-one-way close. This is the axis
  Slice B *specifically exists to close* (the others it inherits from the shared primitive). *Failure mode:*
  leaving the fork open, or "fixing" app-mode with a parallel bespoke queue instead of the shared one.
- **(d) Single-FIFO issue-order (VM-staging partition).** App-mode's `take_pending_history` yields the VM
  `pending_history` FIFO in issue order; the coordinator's I2 partition preserves it (sync prefix in-task; from
  the first in-range traversal onward, every step defers onto the one tagged queue in issue order). Q-VM-MODEL
  = shell-drain-only: the VM staging is **unchanged** (identical to content). *Failure mode:* reordering a sync
  update ahead of a traversal issued before it.
- **(e) No-op-vs-in-range (peek-classify / cursor atomicity).** Resolution E: app-mode's `classify_traversal`
  peeks `nav_controller` — an **in-range** traversal is a partition barrier; a **no-op** (`go(999)`
  out-of-range → `peek_delta` returns `None`) falls through (no barrier, no default-suppression). Peek→commit
  atomicity (a failed load never moves the cursor — `traverse_to` commits only on load success)
  survives unchanged. *Failure mode:* a no-op traversal wrongly deferring trailing sync or suppressing a link
  default.

**Pairwise intersections (each cell → where its invariant is pinned in this memo):**

| × | (b) app-no-pump | (c) fork-close | (d) VM-staging | (e) peek/atomicity |
|---|---|---|---|---|
| **(a) phase-sep** | I1 app-leg drains Phase 2 after Phase 1 in-handler; bounded snapshot complete because (b) is dead (§4.4) | both shells drive `drain_same_turn` vs `run_deferred_traversals` — one primitive (§0/§4) | I1 ordering ⇒ apply reads a committed list; I2 partition unchanged (§4.2) | I1 ⇒ apply reads a committed cursor; peek→commit atomic (§4.3) |
| **(b) app-no-pump** | — | the fork's app leg is *why* (b) matters — app has no pump to lean on (§4.4) | reentrant re-partition of the FIFO mid-apply is the only queue-growth vector; absent in app-mode (§4.4) | no mid-apply cursor mutation ⇒ peek→commit trivially atomic (§4.4) |
| **(c) fork-close** | — | — | both shells drain the same VM staging (Q-VM-MODEL) | both shells share `peek_*`/`commit_index` |
| **(d) VM-staging** | — | — | — | partition preserves single-FIFO ordering SoT (I2) |

≥3 intersecting axes ⇒ **plan-review-gated per-PR slice under the approved umbrella** (CLAUDE.md edge-dense
rule). Slice B is a **terminal single PR** (base-case: a narrowly-scoped per-PR slice under an approved
umbrella that has passed plan-review is a terminal unit — umbrella §0 edge-dense base case).

---

## §3 Spec coverage map (the subset this slice touches — IN vs fenced-OUT)

The subset of the umbrella §3 / Slice-A §3 surface the **app-mode** leg touches (single top-level traversable;
multi-navigable fan-out OUT/B1). Section labels use webref **section titles** (all §↔title pairs + algorithm
anchors webref-verified **2026-07-26** — no drift; anchors: `#navigate-non-frag-sync` /
`#reloading-and-traversing` / `#beginning-navigation` / `#updating-the-traversable` / `#traversable-navigables`
/ `#centralized-modifications-of-session-history` / `#scroll-to-fragid` [dfn `#finalize-a-same-document-navigation`]).
`Full enum?` = ✓ when the row's in-scope branches are exhaustively covered by this slice; `PARTIAL` / `n/a
(B1)` mark fenced or B1-gated rows.

| Spec section (webref title) | Step | Branch | App-mode touch (this slice) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §7.4.4 Non-fragment synchronous "navigations" | 3–11 | *URL and history update steps*: build `newEntry`, on push bump index+length, set URL/active entry | IN — Phase-1b sync updates (`PushState`/`ReplaceState`) applied in-task via the `handle_history_action` seam (app body = `apply_state_change`) | ✓ | yes (pushState/replaceState/`location.*`) |
| WHATWG HTML §7.4.4 Non-fragment synchronous "navigations" | 12–13 | append *synchronous navigation steps* onto the ONE tagged queue | PARTIAL — Resolution D (inherited): a `SyncUpdate` deferred behind ANY same-turn traversal is CANCELED (bounded); full call-time-entry jump-the-queue fenced | ✗ (bounded — `#11-sync-navigation-steps-queue-tagging`) | yes |
| WHATWG HTML §7.4.3 Reloading and traversing | 4 | *"Append the following session history traversal steps to traversable"* | IN — Phase-1b enqueue via `classify_traversal` peek seam | ✓ | yes (back/forward/go) |
| WHATWG HTML §7.4.3 Reloading and traversing | 4.4 | *"If allSteps[targetStepIndex] does not exist, then abort these steps"* (no-op) | IN — Resolution E peek-classify: `peek_delta → None` = no barrier, falls through | ✓ | yes (back/forward/go) |
| WHATWG HTML §7.4.2.2 Beginning navigation | 19 | ongoing navigation is "traversal" ⇒ *"Any attempts to navigate a navigable that is currently traversing are ignored"* | **DIVERGENT (deliberate)** — Resolution A: Phase-1c drains the nav slot and does not apply while a `Traversal` step is **queued**. Step 19's gate is *ongoing navigation* == "traversal", set ONLY by the §7.4.6.1 step-8.4 APPLY, so a nav issued before the apply is never step-19-ignored and elidex's window is a strict **superset** of the spec's (slot `#11-nav-supersede-window-vs-ongoing-navigation`). **App-mode drain-and-HOLD, not discard** (§4.3) narrows the superset back to "a traversal that really moved the cursor": same-turn Phase 2 settles it and reinstates the nav if none did | ✗ (divergence — `#11-nav-supersede-window-vs-ongoing-navigation`) | yes (`location.*`) |
| WHATWG HTML §7.4.2.2 Beginning navigation | 20 | *"aborting other ongoing navigations"* (not a traversal) | IN — Resolution A: traversal wins the same-turn straddle; exact cross-channel issue order fenced | ✗ (bounded — `#11-sync-navigation-steps-queue-tagging`) | yes (`location.*`) |
| WHATWG HTML §7.4.6.1 Updating the traversable | 1 | *apply the history step*: assert running within the traversal queue | IN — the queue-serialized Phase-2 apply (guard observational this slice) | ✓ | no |
| WHATWG HTML §7.4.6.1 Updating the traversable | 3/4/6/7 | initiator sandbox / cross-doc navigable set / `changingNavigables` / nonchanging siblings | OUT (B1) — always `{top-level}` | n/a (B1) | no |
| WHATWG HTML §7.4.6.1 Updating the traversable | 8 | per-navigable: set current entry; queue global task on the navigation-and-traversal task source | IN (top-level only) — the single Phase-2 apply, realized as `drain_same_turn`'s end-of-handler Phase-2 (the **degenerate** task, Q-SCHED (i)) | ✓ | no |
| WHATWG HTML §7.4.6.1 Updating the traversable | 12 | *"split into two parts to allow synchronous navigations to be processed before documents unload"* | IN — the decisive phase-separation (Phase 1 lands, then Phase 2), degenerately for single-traversable | ✓ | no |
| WHATWG HTML §7.4.6.1 Updating the traversable | 14 (note at step 14.1.1) | *"Synchronous navigations … jump the queue … before this traversal potentially unloads their document"* | PARTIAL — Resolution D cancel-behind-any-traversal is the bounded stand-in; call-time-entry jump-the-queue fenced (the tagging that enables the jump is defined in §7.4.1.3) | ✗ (bounded — `#11-sync-navigation-steps-queue-tagging`) | no |
| WHATWG HTML §7.3.1.1 Traversable navigables | queue obj | session history traversal **queue** object | IN — `TraversalQueue` on `NavigationController` (cooperative deferred; app-mode drains it synchronously via `drain_same_turn`) | ✓ | no |
| WHATWG HTML §7.3.1.1 Traversable navigables | queue obj | **"running nested apply history step" boolean** (init false) | PARTIAL — present + observational; canonical wiring is Slice 4. **App-mode's reentrancy vector is absent by construction (§4.4), so the guard is inert here — no interim buffer needed** | ✗ (Slice 4) | no |

**Breadth**: K=1 spec (html), M=13 rows. Split decision: spec-breadth reads single-PR; the umbrella's split
driver is the **edge-dense / canonical-algorithm-absent** rule (§2, ≥3 intersecting axes), so the split is by
**implementation slice under the approved umbrella** — this memo is the terminal app-mode Slice B (umbrella §5,
old Slice 3).

### §3.1 User-input touch audit

- **Synchronous updates (§7.4.4, Phase 1 in-task):** `history.pushState()` / `history.replaceState()` /
  `location.href=` / `location.assign()` / `location.replace()` / `location.reload()` — staged to
  `vm/host/navigation.rs` `pending_history` (push/replace) + `pending_navigation` (`location.*`). Identical
  staging to content (Q-VM-MODEL).
- **Traversals (§7.4.3 → §7.4.6.1, Phase 2 deferred within the handler):** `history.back()` /
  `history.forward()` / `history.go(delta)` — staged to `pending_history` as `Back`/`Forward`/`Go`, classified
  by the new `classify_traversal` peek seam (in-range = barrier; no-op falls through).
- **Chrome-button traversals** (`app/navigation.rs::handle_chrome_action`) reach the same
  `peek_*`/`traverse_to` path with `UserInvolvement::BrowserUi` — **FENCED OUT of Slice B** (§0), collapses
  into Slice 4's canonical DIRECT-nav serialization. NOT routed through the coordinator here.
- **Alt+←/→ keyboard traversals** (`app/inline.rs::handle_keyboard_inline`) — the **second** user-input
  traversal entry point, same `peek_back`/`peek_forward` → `traverse_to` shape with
  `UserInvolvement::BrowserUi`, and it `return`s before `handle_keyboard`, so no coordinator drain runs on that
  turn. **FENCED OUT of Slice B** on the same terms as chrome-traverse (§0) — the fence was always real; this
  audit bullet was the incomplete part.

---

## §4 The design (adopt `DrainHost` for the app-mode shell; drive `drain_same_turn`)

### 4.1 The `DrainHost` impl (mirror `content/drain_host.rs`)

Implement the eight `DrainHost` seams for the app-mode shell, each delegating to the existing app-mode body
(the bodies stay behind the trait — no algorithm crosses into `elidex-navigation`). The **queue field** is a
`traversal_queue: TraversalQueue` homed on `interactive` next to `NavigationController` (Q-OWNER =
engine-agnostic near the controller; the controller lives on `InteractiveState` and survives pipeline rebuild),
reached from the `App` `DrainHost` receiver through `self.interactive.as_mut().expect(...)` (§4.5 invariant).

| Seam | App-mode realization (existing site → reshaped) |
|---|---|
| `traversal_queue` | `&mut self.interactive.as_mut().expect(...).traversal_queue` — homed on `interactive` next to `nav_controller`; the `expect` is an unreachable-panic (never-cleared `interactive` invariant, §4.5) |
| `route_window_opens` | the `take_pending_window_opens()` drop (`process_pending_navigation:50`) — legacy inline has no new-tab / iframe, so drain-and-drop (unchanged behavior) |
| `take_pending_history` | `self.interactive.as_mut().expect(...).pipeline.runtime.take_pending_history()` (`:59`) — `App` receiver reaches the VM staging through `interactive` |
| `handle_history_action` | the **sync-update-only** arm — `PushState`/`ReplaceState` → `apply_state_change` (the `Back`/`Forward`/`Go` arms move OUT to `apply_traversal`) |
| `classify_traversal` (peek-gated, first traversal) | `nav_controller.peek_delta(delta).is_some().then(\|\| self.pending_traversal(delta))` — mirrors `content/drain_host.rs:236` exactly |
| `pending_traversal` (no-peek, subsequent) | `PendingTraversal { delta, user_involvement: UserInvolvement::None }` (scripted; chrome `BrowserUi` is the fenced chrome path) |
| `handle_navigation` | the Phase-1c `take_pending_navigation` + `resolve_nav_url` + `self.navigate(...)` block (`:89`–`:95`), all `&mut App` bodies (set_title co-located, §4.5); on `suppress`, take `pending_navigation` and HOLD it on `InteractiveState::deferred_navigation` without applying (Resolution A / F1 — §4.3's hold-then-settle, not a drop) |
| `apply_traversal` | reshape `traverse_to` into a **delta-keyed** `&mut App` body: `peek_delta(delta) → (target_index, url)`, then the existing peek-then-commit `resolve_traversal`/`same_document_step`/`navigate_to_history_url`+`commit_index`, returning `bool` (shipped). The app-mode mirror of content's `apply_traversal_delta:337` |
| `ship_frame` | performs `render_state.window.request_redraw()` **inside the seam** — the App-owned OS-window output, the mirror of content's `send_display_list` (`content/drain_host.rs:305`–`:314`). **`set_title` is NOT in the seam** (as built — §4.5 "Residual impl note"): it stays co-located in the nav / sync-update bodies, which also serve the non-drain callers, and every path that reaches this seam has already run one of those bodies, so a seam-level `set_title` would be redundant. `render_state` is reached because the receiver is `App` (the `App::render_state` field); the coordinator touches the window only BEHIND the trait (§4.5, Q-SHIP resolved) |

**Drain rewiring.** Replace `process_pending_navigation`'s hand-rolled body (`:34`–`:98`) with a single
`DrainCoordinator::drain_same_turn(self)` call (`self` = `App`), keeping the `interactive.is_some()` guard
(`:35`) so the coordinator is only ever driven with `interactive` present — the invariant that makes every
per-seam `.expect()` an unreachable-panic (§4.5). The two `return true`s (`:73` traversal-supersede,
`:93` nav-applied) dissolve:
- **`:73` — DELETED semantics** (the load-bearing removal, mirror of content `:593`). A traversal in the
  history FIFO no longer stops the drain; it **enqueues** (Phase 1b) and **applies in Phase 2**. Trailing
  *synchronous* intents replay via the I2 partition (never truncated — closes #259). The collapsed
  §7.4.3-vs-§7.4.4 boundary is gone.
- **`:93` — RELOCATED** into the `handle_navigation` seam's `bool` return. "A navigation applied" now flows
  through `DrainOutcome.{own_context_action, shipped}`, not an early return from the drain.

### 4.2 Why `drain_same_turn` (the ratified Q-SCHED (i) shape)

The substrate provides `drain_same_turn` **explicitly** as "the app-mode-degenerate / atomic same-turn drain"
(`DrainCoordinator::drain_same_turn`): it runs `run_synchronous_phase_body` (Phase 1) then `drain_traversal_queue`
(Phase 2) back-to-back and ships **exactly once** at the end. That IS Q-SCHED option (i): "drain Phase 2 at
end-of-input-handler, strictly after Phase 1." Content-mode does NOT use this method — it schedules the two
phases across separate turns (`drain_synchronous_phase` in-task + `run_deferred_traversals` on the async pump).
`drain_same_turn` is the degenerate collapse of that schedule for a shell with no task boundary. **This is the
whole point of One-issue-one-way: same coordinator, same phase bodies, same I1/I2/I3 invariants, same
Resolution A/B/D/E machinery — the shells share the primitive and differ in their ENTRY POINTS (which realize
WHEN Phase 2 pumps).**

**The entry-point sets are NOT symmetric, and the asymmetry is a real bounded gap.** Content-mode drives THREE
coordinator calls per pump turn (`content/event_loop.rs`: `run_deferred_traversals` → `drain_synchronous_updates`
→ `drain_synchronous_phase`); app-mode drives ONE. The missing counterpart is content's **post-Phase-2
synchronous settle** — the `drain_synchronous_updates` that runs immediately after `run_deferred_traversals` so a
§7.4.4 intent staged by the `popstate` handler a Phase-2 traversal fired (`pushState`, `location.*`) completes on
the SAME turn. App-mode's `drain_same_turn` returns straight after Phase 2, so such an intent waits for the NEXT
input event. **Bounded, pinned-not-silent** (`app_history_drain_tests::app_popstate_staged_action_defers_to_the_next_drain_not_the_current_queue`),
fenced to `#11-app-mode-turn-completion-drain`. The correct fix is **loop-until-quiescent turn completion**, not
a trailing `drain_synchronous_updates` — that would settle a popstate-staged `pushState` but strand a
popstate-staged `back()` with no Phase 2 behind it — so it is edge-dense and lands as its own plan-reviewed PR.

The I2 partition, the `SyncUpdate` cancel (Resolution D), the no-op peek-classify (Resolution E), and the
nav-suppression (Resolution A) are all **inherited from the shared coordinator** with zero app-specific logic
— app-mode gets them for free by driving `drain_same_turn`. This is the ideal: app-mode adds **implementation
of seams**, not **re-implementation of policy**.

### 4.3 Supersede-return removal + default-suppression consumer refinement

**Supersede removal** — §4.1 above (`:73` deleted, `:93` relocated). Post-removal, a same-turn
`pushState('/a'); back()` keeps `/a` (Phase 1) and applies the traversal (Phase 2), and a
`back(); location.href='/b'` suppresses the later `location.*` nav (Resolution A — the in-range traversal
wins), landing on the back target with no `/b` load/flash.

**The enqueue-time supersede is a deliberate DIVERGENCE from §7.4.2.2 step 19, not an application of it**
(webref-verified 2026-07-26; slot `#11-nav-supersede-window-vs-ongoing-navigation`). Step 19's gate is
*ongoing navigation* == "traversal" (§7.4.2.5), which §7.4.6.1 *Updating the traversable* **step 8.4** sets
only once the apply RUNS — §7.4.3's enqueue sets nothing, and the three null-resets are each annotated *"This
allows new navigations of navigable to start, whereas during the traversal they were blocked."* So the spec's
blocking window is strictly *during the apply*, and a `location.*` issued before it — `history.back();
location.assign('/b')` in one handler — is never step-19-ignored, **however the queued traversal later
resolves**. Resolution A suppresses from **enqueue** time: a strict superset of the spec's window,
engine-wide (shipped in Slice A, PR #469; content-mode carries the same rule) and NOT introduced here.

**Suppression is HOLD-then-settle, not discard (the §7.4.2 leg of Resolution D)** — which narrows the
superset back to "a traversal that really moved the cursor", the closest app-mode gets to the spec window
without a behavior change. App-mode's Phase 2 runs in the same turn and therefore settles it:
`handle_navigation` drains the VM slot (so nothing can re-fire a turn late) but HOLDS
the resolved request on `InteractiveState::deferred_navigation`; `apply_traversal` CANCELS it the moment a
traversal moves the cursor — the same "cursor-moved" predicate the coordinator's `traversal_applied` latch
uses for a deferred `SyncUpdate` — and `process_pending_navigation`'s tail reinstates whatever survives.
This preserves, on the §7.4.2 leg, the contract the retired drain had ("a no-target / failed-load traversal
returns `false`, so the loop CONTINUES and trailing same-turn intents still apply" — Codex R1 P2 / R2), which
the sync-update leg keeps via Resolution D's re-check gate. **Content-mode has no mirror**: its Phase 2 is a
genuinely later task, so reinstating there WOULD be the fire-a-turn-late that draining exists to prevent; its
fix is the tagged queue (`#11-sync-navigation-steps-queue-tagging`).

**Default-suppression consumer refinement (the `handle_click` bool consumer).** Today `handle_click` reads
`if self.process_pending_navigation() { return; }` — the `true` suppresses the `<a href>` default navigation
(`:94`–`:107`). Post-restructure the consumer reads **`DrainCoordinator::drain_same_turn(self).suppress_default`**
— the `DrainOutcome.suppress_default` field the coordinator already computes as
`own_context_action || <queue holds a Traversal step>` (at the end of `run_synchronous_phase_body`), the ONE shared
default-suppression signal (Slice A Resolution B/E1). `handle_keyboard` calls it for effect and ignores the
result (unchanged — no default to suppress there).

**The app-mode subtlety (the "pending-but-not-yet-applied traversal" the task flags).** Content-mode's
consumer genuinely sees a *pending, not-yet-applied* traversal (Phase 2 is deferred to a later pump turn), so
Resolution B's queue-query (`suppress_default` reading `has_pending_traversal`) is what suppresses the default
before the apply. **App-mode's `drain_same_turn` applies the traversal *before returning*** — so by the time
`handle_click` reads the outcome, the traversal is already applied. **But the SAME field is correct in BOTH
shells:** `suppress_default` is computed at the END OF PHASE 1 (inside `run_synchronous_phase_body`, before
`drain_traversal_queue` runs), so at that instant the in-range traversal is enqueued-but-not-yet-applied and
`has_pending_traversal()` is `true` → `suppress_default = true`. App-mode reads the identical field with the
identical semantics; the only difference is that Phase 2 has *also already run* by the time the field is read.
**One-issue-one-way: one field, one consumer rule, both shells.** Resolution E guarantees a no-op `go(999)`
never enqueues a `Traversal` step, so it never over-suppresses a legitimate link default — the app-mode
consumer inherits that correctness. Peek→commit atomicity is likewise inherited — but it does **NOT** reach
`suppress_default`, and the earlier draft of this sentence claimed it did. `suppress_default` is latched at the
END of Phase 1 as `own_context_action || has_pending_traversal() || is_applying()`, **before any Phase-2 apply
runs**, so an in-range traversal whose Phase-2 cross-document load later FAILS still yields
`suppress_default = true`; only `shipped` / `own_context_action` stay `false` (from the apply body's `false`
return). `app_history_drain_tests::app_go_zero_is_an_in_range_barrier_that_rebuilds` pins exactly that shape —
`suppress_default` true, `!shipped`, failing rebuild. The `suppress_default = false` case is the **no-op /
out-of-range** traversal, which the preceding sentence already covers (Resolution E enqueues no `Traversal`
step at all, so nothing is pending to latch on).

### 4.4 THE CRUX — I3 app-mode liveness: **the reentrancy vector is DEAD by construction** (resolution (b))

The umbrella §4.5 I3 caveat: content-mode's bounded snapshot relies on the **every-turn async pump** for
liveness (a step serialized mid-apply drains next turn); app-mode has no pump, so Slice B must NOT blindly
adopt the bounded drain without either **(a)** an end-of-handler re-check, or **(b)** proving the reentrancy
vector is dead by construction. **Resolution: (b) — the vector is structurally absent in app-mode. Option (a)
is rejected as dead code defending against a vector that cannot fire.** The by-construction proof (the
app-mode analog of content's R4/R16 SW-reachability analysis):

**1. The app-mode `DrainHost` drive runs EXCLUSIVELY on the legacy-inline `InteractiveState` path.**
`process_pending_navigation` (→ `drain_same_turn`) is reached only from `handle_click` / `handle_keyboard`
(`app/events.rs`), which are called only from the **inline** dispatch — `app/inline.rs`'s
`handle_mouse_press_inline` and `handle_keyboard_inline`, both reached from
`handle_window_event_inline`. Threaded mode uses a **different** method set
(`handle_keyboard_threaded`, `app/threaded.rs:400`) that *messages the content thread* — which runs its own
content-mode `DrainHost`. So `drain_same_turn` never runs in threaded mode.

**2. The inline `InteractiveState` path has NO service-worker machinery at all.** `App::new_interactive_with_url` sets
`network_process: None` and `origin_storage: None` (`app/mod.rs`, verbatim comments
"Legacy mode — no broker" / "Inline/legacy mode — no SW, no per-origin storage").

**3. The inline navigation body issues a DIRECT blocking fetch with no SW hook.** `load_url_into_pipeline`
(`app/navigation.rs`) → `elidex_navigation::load_document(url, &network_handle, None)` →
`network_handle.fetch_blocking(req)` (`loader.rs:188`). `load_document`'s third parameter is `Option<Request>`
(a request override, **not** an SW hook — `loader.rs:181`–`:185`); there is **no** `sw_controller_scope()`
consultation, **no** SW-fetch wait loop, **no** `BrowserToContent` message re-dispatch. Contrast content-mode's
`handle_navigate` (`content/navigation.rs:116` `if let Some(sw_scope) = ...sw_controller_scope()` → the SW-wait
loop `:159`–`:205` that re-dispatches via `dispatch_or_buffer_reentrant:191`) — **that** blocking SW-wait is
the ENTIRE content-mode reentrancy vector, and app-mode's inline path structurally lacks it.

**4. The app-mode SW facilities are browser-thread / content-thread-only and unreachable from the inline
drive.** `SwCoordinator` (`app/sw_coordinator.rs`) and `SwFetchRelay` (`app/sw_fetch_relay.rs`) are
**browser-thread** facilities operating over **content-thread** channels (`TabId`,
`LocalChannel<BrowserToContent, …>`); they never touch `InteractiveState`. `SwFetchRelay` is moreover
`#[allow(dead_code)]` / **unwired** (the `mod sw_fetch_relay` declaration in `app/mod.rs` carries the attribute; its only call site is a `TODO(M4-10)` in the *threaded*
`content_messages.rs`). Neither can re-dispatch a nav-mutating message during an inline `apply_traversal`.

**5. No other mid-apply queue-growth vector exists.** The only way the `TraversalQueue` grows mid-apply is a
**reentrant re-partition of the VM `pending_history` FIFO** during `drain_traversal_queue` — which requires
re-entering `run_synchronous_phase_body` mid-drain (content's SW-pump-message vector, buffered by its interim
guard). App-mode's `drain_same_turn` is a straight synchronous body with no message-recv loop; nothing
re-enters Phase 1 during Phase 2. A popstate handler or a freshly-rebuilt page's initial script may *stage*
new history actions onto the VM `pending_history`, but those are NOT partitioned into the `TraversalQueue`
until the next `process_pending_navigation` — they do **not** re-enter the current drain's queue.
(That "next" is unbounded, not next-input-bounded: `events::handle_click` returns early on a hit-test miss /
a chrome-band click / an unset `cursor_pos`, and `events::handle_keyboard` on an unfocused document, all
before the drive site — see `#11-app-mode-turn-completion-drain`.)
**Root invariant (premise 5 rests on THIS, not merely on the staging-not-partitioning consequence):** *No
app-mode apply body synchronously drives the coordinator's Phase-1 partition (`run_synchronous_phase_body`); a
reshaped `apply_traversal` MUST preserve this — a body that eagerly re-drained pending nav would re-open the
mid-apply re-enqueue vector.* The staging-not-partitioning above holds only **because** this invariant does,
which hardens the by-construction proof against a future apply-body change.

**As-built trip-wire (machine-guarding premise 5).** The premise has two failure shapes, and
`process_pending_navigation` takes one `debug_assert` each. ENTRY = a **re-drive** from any body the drive
runs; its signal is a host-side `InteractiveState::drain_in_progress` flag set for the whole drive, NOT
`TraversalQueue::is_applying()` — the coordinator's `enter_nested_apply`/`exit_nested_apply` bracket wraps
`DrainHost::apply_traversal` ALONE, so `is_applying()` is blind to a re-drive from a Phase-1 seam body,
including the headline "re-drain at the end of `navigate`" shape (app-mode reaches `navigate` from Phase 1c).
EXIT = a **residual step** left on the queue; it does not strand permanently (the next drive's Phase-1 seed +
bounded snapshot drain it) but its latency is unbounded and it acts as a full partition barrier meanwhile.

**Conclusion.** In app-mode the `TraversalQueue` cannot grow during `drain_traversal_queue`, so the **bounded
snapshot captured at Phase-2 drain-start equals the entire queue** (everything Phase 1 partitioned this turn),
and the drain is **complete-and-terminating by construction** — **nothing strands.** Option (a)'s
end-of-handler re-check would re-drain an *always-empty* residual: it defends against a mid-apply re-enqueue
that has no source in app-mode, so it is **dead code** and is rejected (Ideal-over-pragmatic: do not add a
guard for an unreachable state). Option (b) is by-construction-sound AND the One-issue-one-way choice —
app-mode drives the identical `drain_traversal_queue` bounded snapshot as content, just synchronously via
`drain_same_turn`, and needs **no** app-mode reentrancy machinery (no `deferred_reentrant_messages`, no
`dispatch_or_buffer_reentrant` mirror). The §7.3.1.1 nested-apply guard is present-but-inert in app-mode.

**Reconciliation with the open-defer-slots ledger (R18 facet — `project_open-defer-slots.md` R18).** The ledger
records the app-mode `traversal_applied` per-drain-local case — a `SyncUpdate` serialized behind an in-flight
traversal and left for a LATER drain, applying-not-canceling against the post-traversal cursor — as a
**reentrant-Phase-1-under-apply carry** onto `#11-sync-navigation-steps-queue-tagging`. §4.4 **discharges that
R18 app-mode carry dead-by-construction**: inline app-mode has no SW pump ⇒ no reentrant Phase-1 under apply ⇒
nothing to carry — a **stronger** outcome than the ledger's anticipated "carry" (the carry is not merely
deferred but structurally void here). This discharges **only** the reentrant-Phase-1-under-apply carry: the
tagging-slot canonical proper — the multi-traversal Phase-2 straddle / cross-navigable finalize (R16-F3) —
**stays fenced for app-mode too**, inherited verbatim from the shared coordinator (as §0/§3 already state via
Resolution D).

> **Documented re-eval trigger (not a current residual).** If a future change wires an SW-fetch relay into the
> inline `InteractiveState` navigation path (M4-10 async-SW-fetch), premise 2/3 breaks and the vector becomes
> reachable — at which point app-mode inherits the Slice-4 canonical DIRECT-nav serialization (the same
> canonical work content-mode gets). Until then the vector is absent. This is the app-mode analog of
> content-mode's SW-fetch fence, and it lands on the same `#11-session-history-task-queue-model` slot.

### 4.5 Distinct app-mode wrinkles vs the content mirror (name them so plan-review sees the deltas)

Content-mode's `ContentState` is a **self-contained per-thread actor** — every seam reaches only `self.*`,
including its output channel (`send_display_list`), so `impl DrainHost for ContentState` is clean. The app-mode
receiver that is self-contained the *same* way — owning **everything the drain needs** — is **`App`**, not
`InteractiveState`:

- **`interactive`** (`App::interactive`, `Option<InteractiveState>`) carries the pipeline + `nav_controller` +
  `window_title` + the new `traversal_queue`, but has **no window handle and no output path**, and is itself
  documented legacy/test state (see the `InteractiveState` doc comment).
- **`render_state`** (the winit window — the OUTPUT path) lives on `App` (`App::render_state`), NOT on `InteractiveState`.
- **`web_storage`** (`Arc<WebStorageManager>`, `App::web_storage`) lives on `App`; the drain reads it via
  `load_url_into_pipeline` to re-install the rebuilt pipeline's `localStorage` backend.

Only `App` owns all three, so the One-issue-one-way structural mirror of `impl DrainHost for ContentState` is
**`impl DrainHost for App`** (§7 Q-IMPL-TARGET) — the receiver self-contained the way `ContentState` is.

**The one distinct app-mode wrinkle — a guarded reach-through, provably safe.** Because the queue and controller
are homed on `interactive` (an `Option` on `App`), every per-drain seam reaches
`self.interactive.as_mut().expect(<drive-only-when-Some>)`. This `expect` is an **unreachable-panic across the
entire `drain_same_turn`**: the sole drive site `process_pending_navigation` enters behind
`let Some(interactive) = &mut self.interactive else { return false }` (`navigation.rs:35`), and there is **no
`self.interactive = None` anywhere in the crate** — `navigate` / `navigate_to_history_url` /
`load_url_into_pipeline` replace `interactive.pipeline` **in place**, never the `Option`
(the never-cleared invariant documented on the `App::interactive` field). So the `expect` is a bounded, provably-safe wrinkle, not
an ownership gap — and it is the ONLY cost, because `App` already owns every field the drain touches (no
bolted-on clone, no external output escape hatch).

This memo **RESOLVES** both design decisions (§7) on **design merit (One-issue-one-way), NOT churn**:
- **Q-IMPL-TARGET — RESOLVED: `impl DrainHost for App`.** Three merit reasons converge:
  1. **ship_frame-output symmetry (decisive).** `ContentState::ship_frame` (`content/drain_host.rs:305`–`:314`)
     performs the shell's OUTPUT *inside the seam* via `send_display_list()` — the output mechanism `ContentState`
     owns. The faithful mirror keeps `ship_frame` doing output: app-mode's `ship_frame` performs
     `render_state.window.request_redraw()` **inside the seam**, via the winit window `App` owns (as built, the
     *frame ship* is the seam's alone; `set_title` stays in the nav bodies — Residual impl note below). Under
     `InteractiveState` (no window handle / no output path) the seam **cannot
     ship** — an asymmetry with content at the very seam that defines the pattern; `App` preserves the symmetric
     ship-once realization.
  2. **Self-containment.** `App` owns EVERYTHING the drain needs (`interactive` + `render_state` + `web_storage`),
     the analog of `ContentState`'s self-containment that makes its `impl` clean. `InteractiveState` is **not**
     self-contained: it would need a bolted-on `web_storage` clone (violating CLAUDE.md side-store exception (b) —
     shared cross-cutting browser-level state must not live on per-inline actor state) **plus** an external output
     escape hatch. `App` needs neither.
  3. **The `expect` cost is an acceptable, provably-safe wrinkle** (the never-cleared-`interactive` invariant
     above) — every per-drain seam `.expect()` is an unreachable-panic. `ship_frame` doing the
     `request_redraw` is layering-clean: OS-window I/O inside the app-mode shell's OWN `DrainHost` impl (same
     crate `elidex-shell`), the precise mirror of content's `send_display_list`; the coordinator touches the
     window only BEHIND the trait.

  **Consequence (state as consequence, not reason):** because the nav bodies stay `&mut App` methods, `set_title`
  stays co-located in the bodies (`navigation.rs:174`/`:320`/`:341`/`:499`) serving BOTH the drain and the
  non-drain callers (`<a href>` click `events.rs:105`, Alt+arrow `inline.rs:258`, chrome
  `navigation.rs:604`/`:625`/`:639`) — **NO set_title lift, NO web_storage clone, non-drain callers unaffected.**
- **Q-SHIP — RESOLVED: `ship_frame` performs the frame ship in the seam.** `ship_frame` performs
  `render_state.window.request_redraw()` inside the seam (the App-owned output, the mirror of content's
  `send_display_list`), off the `DrainOutcome.shipped` signal. A pure `pushState` turn changes no layout but DOES
  change the chrome URL bar, so `ship_frame` still issues the `request_redraw` (repaint chrome). **`set_title` is
  deliberately NOT lifted into the seam** (as built, per the Residual impl note below): every path that reaches
  `ship_frame` has already run a nav / sync-update body that wrote `window_title` AND pushed it to the window, so
  a seam-level `set_title` would be redundant — and those bodies must keep it regardless, since they also serve
  the non-drain callers.

  **Residual impl note (a flagged Slice-B impl detail, not a blocker) — request_redraw ship-once consolidation.**
  For the faithful mirror, prefer the drain path's repaint to flow through `ship_frame` + the apply-bodies
  (**ship-once**), leaving `set_title` in the nav bodies (which serve all callers). The non-drain callers (link
  click, Alt+arrow, chrome) keep their existing dispatch-layer `request_redraw` (`inline.rs:201`/`:259`/`:298`);
  the impl MUST ensure the drain path does not ALSO redraw those (preserve ship-once). This symmetric realization
  is **App-only feasible** — flag it so impl chooses it over a degenerate "`ship_frame` just re-renders" form.

**Ownership/layering note.** `NavigationController` + the new `traversal_queue` stay engine-agnostic;
`InteractiveState` / `App` / pipeline / `EcsDom` stay behind the `DrainHost` trait (never cross the crate
boundary). Single-writer discipline is preserved — app-mode Phase 2 is a **synchronous end-of-handler segment**
of the sole renderer thread, not an OS thread. **Side-store exception (b) (mirror Slice A §4 F4):**
`NavigationController.entries`/`index` + `traversal_queue` + the VM `pending_history`/`pending_navigation`
channels are CLAUDE.md **side-store exception (b)** — browsing-context/session-level state of the single
top-level traversable, not per-entity facts, so not ECS components. (`DrainOutcome` is deliberately **not** in
that set: it is a by-value function return, not a store at all, so the side-store question does not arise for
it.)

**No existing slot owns a migration of these, and none should be implied.**
`#11-browsing-context-state-ecs-components` scopes the **VM's per-document state cluster**
(`NavigationState.current_url` + `HostData.{document_origin_override, sandbox_flags, iframe_depth,
fallback_opaque_origin}`), and it **explicitly excludes** `pending_navigation` / `pending_history` as intent
buffers (`pending_navigation`/`pending_history` は intent buffer ゆえ移行対象外); it never scoped the shell-side
`NavigationController` / `TraversalQueue` at all. So exception (b) here is a **standing classification**, not a
deferral parked on that slot.

**B1 re-derivation trigger (recorded here so it is not inherited silently).** Exception (b) holds *because*
`changingNavigables` is always `{top-level}` (§0 fence): one traversable ⇒ one queue ⇒ session-level state. The
B1 multi-navigable fan-out kills that premise — a per-navigable queue is per-**navigable** state, and the
tempting `HashMap<NavigableId, TraversalQueue>` is exactly the entity-keyed side-store the CLAUDE.md rule
forbids (`Send + Sync`, not a per-VM identity handle ⇒ the ECS-native answer is a component on the navigable's
entity, with despawn doing the cleanup). B1 must therefore **re-derive** this classification from first
principles rather than carry this paragraph forward.

---

## §5 Decomposition — terminal single PR + a named prereq

**Slice B is a terminal single PR** under the approved umbrella (edge-dense base case, §2). It bundles nothing
fenced (§0). Two adjacent items are **named, not bundled**:

- **Prereq (standalone split-on-touch PR, lands BEFORE Slice B impl) — content test-helper collapse.**
  `content_fragment_nav_tests.rs` (`base():30`, `drain_browser():39`) and `content_history_drain_tests.rs`
  (`base():42`, `drain_browser():56`) each **redefine** helpers that now duplicate the canonical
  `content_test_support.rs` (`base():205`, `drain_browser():211`). This dedup is **content-internal**
  (both `drain_browser` variants operate on a content-thread `LocalChannel<BrowserToContent, ContentToBrowser>`)
  — a standalone prereq per CLAUDE.md "1000-line debt = touch-time split" split-on-touch discipline, NOT
  bundled into Slice B's PR. It touches content test files only; Slice B does not depend on its outcome beyond
  the tidy base it leaves.
- **App-mode conformance tests → the `app_fragment_nav_tests.rs` neighborhood** (umbrella §8), likely a new
  sibling `app_history_drain_tests.rs` (mirroring `content_history_drain_tests.rs`). See §8.

**The app/content test-support boundary the prereq MUST respect (assessed — a false-unification guard).**
`content_test_support.rs` is **explicitly content-thread-specific**: it spawns content threads over a test
broker (`spawn_test_content`), builds `ContentState` (`build_test_content_state*`), and its `drain_browser`
consumes a **browser IPC channel** an app-mode inline `App` **does not have** (inline mode is synchronous, no
browser channel). App-mode's `app_fragment_nav_tests.rs` builds an `App` via `App::new_interactive_with_url`
(`:43`), drives `app.process_pending_navigation()` / `app.navigate(...)` directly, and defines its own
`base()` (`:19`) as a bare `url::Url::parse` (3 lines). **Assessment: app-mode's `base()` is a FALSE-unification
target** — folding it into the content-thread-specific `content_test_support` would couple app-mode tests to
content-thread scaffolding for a trivial one-liner and drag in `ContentState`/browser-channel machinery
irrelevant to `App`/`InteractiveState`. **App-mode warrants its own thin test-support home** (app-local helpers
in `app_fragment_nav_tests.rs` / a new `app_history_drain_tests.rs`, or a small `app_test_support.rs` if the
app test files proliferate). The prereq collapse is **content-internal only** and MUST NOT force app-mode's
`base()` into `content_test_support`.

**1000-line touch-split assessment for `app/navigation.rs` (737 LoC).** LOW risk of crossing 1000. The natural
home for Slice B's new drain-adapter code is a **new `app/drain_host.rs`** (the direct mirror of
`content/drain_host.rs`, itself carved from `content/navigation.rs` at the drain-adapter cohesion seam — Codex
PR#469 R5). Homing `impl DrainHost` + the delta-keyed `apply_traversal` body there **removes** the
`process_pending_navigation` hand-rolled body (~65 LoC) from `app/navigation.rs` while adding little, so
`app/navigation.rs` stays near its current size. The **orthogonal** `handle_navigate`/`same_document_step` →
sibling-module carve (umbrella §5 Slice-0 note) is the touch-time split **only if** `app/navigation.rs` still
crosses 1000 after the drain-adapter goes to its own file — assessed **unlikely**, so it is **named as a
separate standalone prereq trigger, not planned into Slice B**. (The drain restructure does NOT relieve line
pressure — the carve, if triggered, is orthogonal to it, per CLAUDE.md.)

---

## §6 What it subsumes / closes

| Item | Code site | How Slice B closes it |
|---|---|---|
| **Axis-c fork** (the One-issue-one-way close) | `app/navigation.rs:34` hand-rolled drain vs `content/drain_host.rs` shared coordinator | App-mode drives `DrainCoordinator::drain_same_turn`; both shells now drive the identical primitive (§0/§4.2) |
| **#396 root (app leg)** — sync drain conflates §7.4.4 in-task update with §7.4.3 queued traversal | `app/navigation.rs:73` supersede-`return` | Phase-separation: the traversal gets Phase 2 after sync updates land (degenerately, end-of-handler) |
| **#259 (app leg)** — multi-action FIFO truncated by a traversal supersede | `app/navigation.rs:59`–`:75` | Phase 1 replays ALL synchronous updates (no `:73` truncation); the traversal defers, so `pushState; pushState; back()` keeps both pushes |
| **#283 (app leg)** — fall-through onto a freshly-rebuilt runtime | `app/navigation.rs:73` (the `return true` guarding the fresh-runtime `location.*` drain) | With the traversal deferred to Phase 2 and `:73` removed, a freshly-loaded page's `pending_navigation` drains on the **next** input (app-mode's degenerate later-task), not stranded/mis-read on the traversing turn |
| **E7 (app leg)** — traversal + nav same-turn race | umbrella §1 residual | The Phase-1-before-Phase-2 ordering (I1 app-leg) IS the resolution for a single traversable; §7.4.6.1 step 12.4 update-only inherited via the shared apply |
| **Resolution A/B/D/E (app leg)** — nav-supersede / pending-default-suppression / SyncUpdate-cancel / no-op-peek | inherited from the shared coordinator | App-mode gets all four for free by driving `drain_same_turn` — no app-specific policy code (§4.2) |

**Explicitly NOT closed (fenced, §0):** chrome-traverse atomicity (`handle_chrome_action:596` →
`#11-session-history-task-queue-model`, Slice 4 canonical); the full §7.4.1.3 jump-the-queue reconciliation
(→ `#11-sync-navigation-steps-queue-tagging`); multi-navigable fan-out + the Q-SCHED B1 re-eval (→ B1).

---

## §7 Open questions for `/elidex-plan-review` (decision-level)

- **Q-IMPL-TARGET — RESOLVED: `impl DrainHost for App`** (the One-issue-one-way structural mirror of
  `impl DrainHost for ContentState` — the app-mode receiver that owns EVERYTHING the drain needs, the way
  `ContentState` does). Chosen on **design merit, NOT churn**, on three converging reasons (§4.5):
  1. **ship_frame-output symmetry (decisive).** `ContentState::ship_frame` performs the shell's output *inside
     the seam* (`send_display_list`, `content/drain_host.rs:305`–`:314`); the faithful mirror keeps `ship_frame`
     doing output via the winit window `App` owns (the `App::render_state` field). `InteractiveState` has no
     window handle / no output path (see the `InteractiveState` field list), so under it the seam CANNOT ship — asymmetry at the
     very seam that defines the pattern.
  2. **Self-containment.** `App` owns `interactive` + `render_state` + `web_storage`;
     `InteractiveState` would need a bolted-on `web_storage` clone (violating CLAUDE.md side-store exception (b))
     **plus** an external output escape hatch. (`InteractiveState` is itself documented legacy/test state,
     the `InteractiveState` doc comment; `App` is the real driver object.)
  3. **The `expect` cost is provably safe.** Seams reach `self.interactive.as_mut().expect(...)`; the `expect` is
     an unreachable-panic — the drive site guards
     `let Some(interactive) = &mut self.interactive else { return false }` (`navigation.rs:35`) and there is **no
     `self.interactive = None` anywhere in the crate** (bodies replace `interactive.pipeline` in place,
     `navigation.rs:405`; the `navigation.rs:79`–`:83` never-cleared invariant).

  **Consequence:** the nav bodies stay `&mut App` methods, so `set_title` stays co-located
  (`navigation.rs:174`/`:320`/`:341`/`:499`) serving BOTH drain and non-drain callers (`events.rs:105`,
  `inline.rs:258`, chrome `navigation.rs:604`/`:625`/`:639`) — **no set_title lift, no web_storage clone.**
  Plan-review ratifies the resolved target.
- **Q-SHIP — RESOLVED: `ship_frame` performs the `request_redraw` in the seam** (the App-owned output,
  the mirror of content's `send_display_list`), off the `DrainOutcome.shipped` signal. A pure `pushState` turn
  changes no layout but DOES change the chrome URL bar, so `ship_frame` still issues the `request_redraw` (repaint
  chrome). **`set_title` stays OUT of the seam** (as built — every path reaching `ship_frame` already ran a nav /
  sync-update body that set the title, and those bodies serve the non-drain callers too).
  **Residual impl note (flagged, not a blocker) — request_redraw ship-once:** consolidate the drain
  path's repaint through `ship_frame` + the apply-bodies (ship-once), leaving `set_title` in the nav bodies (they
  serve all callers) and the non-drain callers' existing dispatch-layer `request_redraw`
  (`inline.rs:201`/`:259`/`:298`) untouched — the impl must not double-redraw those. App-only feasible; flag it
  over a degenerate "`ship_frame` just re-renders" form (§4.5).
- **Q-I3 — confirm resolution (b) (reentrancy vector dead by construction).** §4.4 argues the SW-fetch
  reentrancy vector is structurally absent from the inline path (no SW machinery, direct `fetch_blocking`, no
  message pump), so the bounded snapshot drains completely and option (a)'s end-of-handler re-check is dead
  code. **This is presented as RESOLVED, not open** — but it is the axis plan-review will scrutinize hardest,
  so confirm the reachability proof (esp. premise 5: no reentrant FIFO re-partition mid-apply). If plan-review
  finds any inline-path re-enqueue vector I missed, (b) reopens toward (a) or the Slice-4 canonical.
- **Q-DEFAULT — the `handle_click` consumer refinement.** Confirm reading `DrainOutcome.suppress_default`
  (computed at end-of-Phase-1 as `own_context_action || has_pending_traversal`) is correct for app-mode even
  though `drain_same_turn` has *also* applied the traversal by the time the field is read (§4.3) — one field,
  one consumer rule, both shells. Confirm the no-op `go(999)` non-suppression (Resolution E) holds at the
  app-mode consumer.
- **Q-fence — chrome-traverse + landing-proximity.** Confirm chrome-traverse (`handle_chrome_action:596`) stays
  OUT (Slice 4 canonical, NOT routed through the coordinator in Slice B), and that Slice B lands in close
  succession after Slice A (umbrella §5 axis-c leg-1) to bound the code-duplication strangler window.

---

## §8 Test strategy (supported-surface: flip the synchronous-supersede pins, add app conformance parity)

**Flip the app-mode drain tests that pin synchronous supersede.** Any app-mode test asserting the `:73`
traversal-supersede-in-one-pass (a `back()` discarding a trailing same-turn intent) **flips** to the
task-boundary expectation with a §7.4.6.1-step-12 cite (Supported-surface testing: the regression **changes
shape, it does not disappear**). The `app_fragment_nav_tests.rs` fragment/reload cases (which do not exercise
traversal supersede) stay green.

**Add app-mode conformance tests mirroring content's** (the `content_history_drain_tests.rs` scenario table,
run against `App`/`InteractiveState` — pinning axis c "One-issue-one-way is test-enforced, not just asserted"),
in a new `app_history_drain_tests.rs` (§5 boundary — app-local helpers, NOT folded into
`content_test_support`):
- **Phase-sep ordering (axis a / I1 app-leg):** `pushState('/a'); history.back()` in one handler ⇒ `/a`
  committed to `NavigationController` (Phase 1) THEN the traversal applies against the updated list (Phase 2).
  `history.back(); pushState('/x')` ⇒ the trailing sync defers and is **canceled** behind the cursor-moving
  traversal (Resolution D inherited — pins the coherent-state cancel). `go(0)` reload still rebuilds.
- **Nav-vs-traversal supersede (Resolution A):** `history.back(); location.href='/b'` ⇒ Phase-1c drains
  the nav slot and holds it, the applied traversal cancels it, no `/b` load, land on the back target;
  document the reverse cross-channel order (bounded divergence, pinned-not-silent). Its **complement** pins
  §4.3's hold-then-settle: when the `back()`'s cross-document load FAILS the navigable never traversed, so
  the held `location.*` is reinstated in the same turn
  (`app_failed_traversal_reinstates_the_superseded_navigation`).
- **No-op peek-classify (Resolution E):** `go(999); pushState('/x')` at end-of-history ⇒ the no-op does NOT
  defer the trailing push (it applies in-handler) and does NOT suppress a same-turn link default.
- **Default-suppression consumer (Resolution B, app-mode form):** a click whose handler runs a valid
  `history.back()` ⇒ `drain_same_turn(...).suppress_default` is `true`, the `<a href>` default is suppressed;
  a click whose handler runs a no-op `go(999)` ⇒ `suppress_default` is `false`, the default fires (pins the
  §4.3 refinement + Resolution E non-over-suppression).
- **#259 no-truncation:** `pushState; pushState; back()` ⇒ both pushes survive Phase 1, the traversal applies
  Phase 2 (the `:73`-removal regression pin).
- **Liveness-inert (axis b / I3(b)):** assert app-mode's `apply_traversal` does not re-enqueue and the
  `drain_same_turn` bounded snapshot drains the full turn's queue (no residual) — the by-construction
  completeness §4.4 argues. (No reentrant-message test — the vector is absent by construction; the interim
  buffer guard content-mode tests does not exist in app-mode.)
- **Cursor atomicity (axis e):** keep/mirror the failed-load-does-not-move-cursor tests
  (`traverse_to:579`–`:592` peek-then-commit) — a failed cross-document load leaves the cursor + reports
  `shipped = false` (no default over-suppression).

**Two-shell parity (axis c).** The scenario table mirrors content's `content_history_drain_tests.rs` run against
the app-mode shell — One-issue-one-way is test-enforced across the two entry points (`run_deferred_traversals`
vs `drain_same_turn`) **for everything the shared coordinator owns**. Parity is deliberately NOT total: content's
R9 pin (`content_history_phase_sep_tests::pump_drains_popstate_staged_pushstate_this_turn` — the post-Phase-2
synchronous settle) has no app-mode counterpart, because app-mode has no such settle. Its app-mode twin pins the
OPPOSITE behavior (the popstate-staged intent drains on the next drive that is reached — one turn in the
test, unbounded in production), fenced to
`#11-app-mode-turn-completion-drain` (§4.2).
