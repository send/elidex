# Plan-memo — Slice A: traversal-queue substrate co-design + content-mode phase-separation

> Status: **pre-implementation DESIGN plan-memo**. Goes through `/elidex-plan-review` before any
> implementation. Anchored on the first-principles spec-faithful ideal, not a surgical patch.
> **This is edge-dense work** (≥3 intersecting invariant axes — the umbrella §2 matrix).
>
> **Umbrella (READ FIRST, not restated here):**
> `docs/plans/2026-07-session-history-task-queue-model.md` — §0 (SLICING CORRECTION), §1 (the decisive
> task-timing invariant), §2 (the 5-axis edge matrix a–e), §4 (queue design + §4.5 I1/I2/I3), §5 (slice
> ordering, now superseded for the 1/2 split), §7 (resolved Q-OWNER / Q-SCHED / Q-VM-MODEL / Q-SYNC-FINALIZE /
> Q-PEEK / Q-FENCE). This memo covers **only** the co-designed content-mode slice and references the umbrella
> for all shared background.
>
> **Substrate this co-designs ON (EXTEND, do not rewrite):**
> `crates/shell/elidex-navigation/src/traversal_queue.rs` + `traversal_queue_tests.rs` — the 5-round-reviewed
> `TraversalDelta` / `UserInvolvement` / `PendingTraversal` / `PendingHistoryStep` / `TraversalQueue` /
> `DrainOutcome` / `DrainHost` / `DrainCoordinator`. Currently wired into **no shell**.

---

## §0 Decision + scope

**Thesis.** The umbrella's original Slice 1 — an *inert, independently-correct* `DrainCoordinator` substrate
that Slices 2/3 merely "adopt" — was **REFUTED** by implementation + a 5-round Codex review (umbrella §0
SLICING CORRECTION, user-ratified 2026-07-13). Five of the coordinator's correctness properties are
**entangled with real shell state** (document identity, `NavigationController` entry-list peek, the
`pending_navigation` cross-channel) — none expressible in an engine-agnostic no-shell layer — so an inert
substrate ships **latent wrong-when-wired defects**. **Resolution:** fold old Slice 1 (substrate) ⊕ old
Slice 2 (content-mode wiring) into **one co-designed slice** — "traversal-queue substrate + content-mode
phase-separation" — where each of the five entangled properties is designed **correct against the real
consumer**. The 5-round-reviewed substrate re-homes as this slice's base; this memo **extends** it (five
seam/field additions, §2) and **wires it into content mode** (§4).

**The five co-designed resolutions (the plan's spine, §1):** A nav-vs-traversal supersede · B
default-suppression for a *pending* deferred traversal · D deferred-`SyncUpdate` document binding · E no-op
traversal peek-classify · loop-bound (inert-this-slice, fenced to Slice 4). A/B/D/E each need real shell
state the inert substrate lacked; the loop-bound is a conscious fence.

**Scope fence (state clearly, plan-review verifies each leg):**
- **IN:** substrate CO-DESIGN (extend for A/B/D/E) + content-mode WIRING only.
- **OUT → Slice B (old Slice 3): app-mode.** App has **no async pump** (`app/events.rs:89`/`:131` drive
  `process_pending_navigation` synchronously right after event dispatch) — the Q-SCHED end-of-input-handler
  drain is a distinct hard decision. ⚠ **Bounded-drain liveness is content-specific:** Phase-2's bounded
  snapshot (Codex PR#469 R3 T1) relies on content's every-turn async pump to drain a mid-apply-serialized
  step on the *next* turn. App-mode's `drain_same_turn` has no such pump, so Slice B must NOT adopt the
  bounded drain without re-adding an end-of-handler re-check **or** keeping the reentrancy vector dead (see
  the "Resolution (loop-bound)" section + umbrella §4.5 I3). Content and app are **distinct deployment shells, never both active at
  runtime**, so removing content's `:593` supersede while app keeps `app/navigation.rs:73` is a **bounded
  code-duplication strangler, NOT a runtime conflict**; Slice B lands in close succession (umbrella §5
  landing-proximity constraint, axis c). Do **not** wire app-mode here. **Leg-2 of the axis-c constraint
  deliberately RETIRED (F2):** the umbrella §5 gave a two-leg constraint — "land in close succession **OR**
  gate the observable supersede-removal behind Slice B." Leg-2 (the gate) is retired here with justification:
  because content and app are distinct deployment shells never co-active, there is **no single-user runtime
  fork** to gate against, so a gate mechanism would be over-engineering for a non-existent fork. Only leg-1
  (close-succession, bounding the code-duplication strangler window) applies. **Code-level bound (E3):** the
  strangler code-tax during the Slice-A→Slice-B window is confined to legacy/test-only code — app-mode's
  inline path (`new_interactive*`) is `#[allow(dead_code)]` / test-support only, and the production entry
  points (`lib.rs` `new_threaded*`) use only the content path — so "two ways to drain" is never a live
  production fork (complements, does not replace, the runtime-fork argument; leg-1 close-succession remains the
  load-bearing scheduling backstop). This is a deliberate retirement, not a silent drop (confirm in §6
  Q-fence).
- **OUT → Slice 4:** reentrancy-guard *wiring* + `commit_index` `debug_assert` retirement. The
  `running_nested_apply_history_step` bracket exists in the substrate but its only real re-enqueue vector
  (the SW-fetch reentrant pump) is **DEAD/unreachable** in content today (`content/navigation.rs:775`
  "the SW controller path is dead"); wiring it + the termination guard stays Slice 4.
- **OUT → B1:** multi-navigable fan-out (§7.4.6.1 steps 3/4/6/7 + per-navigable global-task of 8/12).
  `changingNavigables` is always `{top-level}` under the single-traversable reality (umbrella §0 fence).

---

## §1 The five co-design resolutions

Each: the defect (current file:line) · spec basis (webref-pinned §+anchor+verbatim) · the decision · the
substrate/coordinator/shell change · (A/D) the bounded divergence + named fence + conformance test.

### Resolution A — nav-vs-traversal supersede (the `location.*` / `pending_navigation` channel)

**Defect.** The coordinator runs Phase-1c `handle_navigation` (`pending_navigation` last-wins) **and** then
applies a deferred traversal in Phase 2 — `DrainOutcome::shipped` only gates the coordinator's *final*
`ship_frame`, NOT the two apply bodies (`traversal_queue.rs:418` runs `handle_navigation` unconditionally;
`:550` runs `apply_traversal`). So a same-turn `history.back(); location.href='/b'` runs BOTH: Phase-1c
loads `/b` over the network and paints it, THEN Phase 2 repaints the back target — a **wasted network load +
visible flash**, and it lands on the *traversal* target even though `location.href` was issued **later**
(issue-order last-wins would land on `/b`). The old shells avoided the fall-through with the `:593`/`:73`
`return true` supersede this slice removes; the substrate faithfully implements the plan's "run both" phase
model (umbrella §4.2), so this is a **phase-model gap for the nav-vs-traversal channel pair**, not a
substrate bug (umbrella §5 Slice-2 CARRY + CARRY-EXT (A)).

**Spec basis** (webref-pinned): `navigate` = **§7.4.2.2 Beginning navigation** (`#navigate`).
- **Step 19:** *"If navigable's ongoing navigation is 'traversal': … Return."* — note verbatim: *"Any
  attempts to navigate a navigable that is currently traversing are ignored."*
- **Step 20:** *"Set the ongoing navigation for navigable to navigationId."* — note verbatim: *"This will
  have the effect of aborting other ongoing **navigations** of navigable…"* (aborts other *navigations*,
  NOT a traversal).

So the issue-order-**earlier** traversal wins over a later same-turn `location.*`: a navigation is ignored
while a traversal is ongoing (step 19), and a later navigation aborts other navigations but not a traversal
(step 20). **Caveat honestly stated:** elidex's VM staging keeps `pending_navigation` (single-slot last-wins,
`vm/host/navigation.rs:143`) as a **separate channel** from `pending_history` FIFO (`:159`); the cross-channel
*issue order* between a `location.href=` and a `history.back()` is **DISCARDED by staging** (the shell always
drains history-FIFO-then-navigation, so it cannot tell which was issued first). Exact spec ordering would
need VM staging to preserve cross-channel issue order (reopening Q-VM-MODEL).

**Decision.** Ship a **bounded approximation** restoring the pre-existing `:593` supersede semantics,
phase-separated: **when an in-range traversal is enqueued this turn (Resolution E peek-classify), Phase-1c
`handle_navigation` is SUPPRESSED** — the same-turn `location.*` nav is dropped, the traversal applies in
Phase 2. This eliminates the double-apply defect (no `/b` network load + flash, land on the traversal
target). **Target-landing is correct in BOTH same-turn orders (E2):** for `back(); location.href='/b'` and
for `location.href='/b'; back()` alike, the spec lands on the **traversal** target — step 19 ignores a
navigation issued while a traversal is ongoing, and step 20 has a later traversal abort the earlier
navigation. So elidex's "an in-range traversal wins regardless of cross-channel issue order" is **already
spec-correct for the landing** in both orders; the plan **over-fences in the safe direction**. The genuine
**bounded divergence** fenced to `#11-sync-navigation-steps-queue-tagging` is the *fuller* reconciliation
(deferred `SyncUpdate` interleaving, multi-nav sequences, involvement threading) — NOT the target-landing,
which is correct both orders. The fence is conservative/honest, not a landing bug.

**Change (drain-and-DISCARD, not skip — F1).** When an in-range traversal is pending after Phase-1b
(predicate = "the queue holds a `Traversal` step" — Resolution E guarantees no-ops never enqueue, so a no-op
`go(999)` does not suppress the nav), Phase-1c must still **drain the `pending_navigation` slot and discard
it** — NOT skip the call. The VM `pending_navigation` slot's ONLY drain is `take_pending_navigation()` inside
`handle_navigation`; skipping the call would strand the slot so the suppressed `location.*` nav fires **a turn
late** (a spurious deferred nav). Matching §7.4.2.2 step-19 "ignored" (= discarded, NOT deferred), the seam
takes-and-drops without applying. Recommend either `DrainHost::discard_pending_navigation()` (take-and-drop)
OR `handle_navigation(suppress: bool)` that still calls `take_pending_navigation()` but does not apply when
`suppress`. Exact seam-shape choice → §6 Q-shape. **Cross-turn variant:** a Turn-1 queued traversal seeds
`seen_traversal` in Turn-2 (`traversal_queue.rs:388`) and would strand Turn-2's nav identically — the
drain-and-discard MUST apply there too (the predicate is "queue holds a `Traversal` step", which is true
across turns until Phase 2 drains it).

**Bounded divergence + fence + test.** Pinned-not-silent (supported-surface testing): the exact cross-channel
issue-order straddle is **NAMED-FENCED** to slot **`#11-sync-navigation-steps-queue-tagging`**. **Fence-slot
charter dependency (F7):** the slot's charter is the §7.4.1.3 tagged-queue reconciliation; closing A's
cross-channel `location.*` residual through it **additionally requires reopening Q-VM-MODEL** (full
issue-order integration of the nav channel needs VM staging to preserve cross-channel issue order, which
Q-VM-MODEL currently fixes as shell-drain-only) — so A's residual is transparently co-homed on that slot +
a Q-VM-MODEL reopen, not silently orphaned. **Conformance test** documents whichever supersede behavior ships
(both same-turn orders).

### Resolution E — no-op traversal peek-classify at enqueue (the partition-barrier bug)

**Defect.** The substrate's `TraversalDelta::from_history_action` (`traversal_queue.rs:84`) classifies
**syntactically** (`Back`/`Forward`/`Go → traversal`), so a no-op `go(999)` at end-of-history flips
`seen_traversal` (`:391`), defers trailing sync updates onto the queue (`:404`), and — with Resolution A —
would wrongly suppress the same-turn nav (Resolution B: would wrongly suppress the default) even though it
resolves to **no target**. The substrate cannot tell in-range from no-op: that needs the
`NavigationController` entry list = shell state (umbrella §5 CARRY-EXT-2 (E)). **Traceability (F9):** this
resolution subsumes the umbrella's peek-classify facet referenced as **"C"** in the CARRY-EXT-2 (E) note (a
dangling ref — C and E are the one peek-classify boundary).

**Spec basis** (webref-pinned): `traverse the history by a delta` = **§7.4.3 Reloading and traversing**
(`#traverse-the-history-by-a-delta`), appended-steps sub-step 4.4 verbatim: *"If allSteps[targetStepIndex]
does not exist, then abort these steps."* Out-of-range = a no-op (no target, no apply). elidex's
`NavigationController::peek_go(999)` returns `None` (out-of-range), `peek_back`/`peek_forward` return `None`
at the ends (`navigation.rs:222`/`:229`/`:237`).

**Decision.** The partition must **peek-classify using shell state** at enqueue time. An **in-range**
traversal IS a partition barrier: it enqueues, flips `seen_traversal`, defers trailing sync (I2), suppresses
the same-turn nav (A), suppresses the default (B). A **no-op** traversal (peek → `None`) is **NOT a barrier**:
it does not enqueue, does not flip `seen_traversal`, does not defer trailing sync, does not suppress
nav/default — it falls through (ships nothing), so subsequent same-turn sync updates + the nav still drain
**in-task**. This preserves the pre-existing "failed/no-op traversal → loop continues → trailing intent
survives" contract that `content_history_drain_tests.rs` pins
(`failed_traversal_load_does_not_drop_trailing_history:351`,
`failed_traversal_does_not_block_same_turn_navigation_drain:447`).

**Change.** Add a `DrainHost` seam — recommended `classify_traversal(&mut self, delta: TraversalDelta)
-> Option<PendingTraversal>` (**named `classify_traversal`, NOT `resolve_traversal`, to avoid colliding with
the existing engine-agnostic `NavigationController::resolve_traversal(target_index) -> TraversalKind`,
`navigation.rs:344`, which the content impl itself calls** — F3) — that consults
`NavigationController::peek_*` and returns `Some` (in-range, with the host-filled `UserInvolvement` — scripted
= `None`, chrome = `BrowserUi`) or `None` (no-op). Phase-1b calls it: `Some` → enqueue + barrier; `None` →
fall through (no barrier). This moves `PendingTraversal` construction from the coordinator (`:393`, which
defaults involvement) to the host, which also lets the host supply real involvement (partially retiring the
R4 involvement-default residual). **Bounded imperfection to flag (§6 Q-E):** peek-classify runs at *enqueue*
against the pre-traversal list, but a *stacked* same-turn traversal (`back(); back()`) peeks the unmoved
cursor twice, so the 2nd may classify in-range yet apply as a no-op in Phase 2 (cursor already moved).
`apply_traversal` still correctly ships nothing for the no-op; the only residual is `deferred_own_context`
(B) possibly over-set for the stacked case. Accepted as **bounded, pinned by a conformance test** (§5), NOT
slotted (an accepted bounded behavior is not a platform gap — §6 Q-E).

### Resolution B — default-suppression for a *pending* deferred traversal

**Defect.** The click consumer `content/event_handlers.rs:174` — `if process_pending_actions(state) { return; }`
— uses the drain's `true` to suppress the `<a href>` default navigation (the link block at `:179`).
Post-phase-separation, a click that queues a *valid in-range* `history.back()` leaves the Phase-1
`own_context_action = false` (the traversal is **deferred, not yet applied**) → the `<a href>` default fires
**before** Phase 2 applies the traversal = bug (umbrella §5 CARRY-EXT (B)). The substrate already exposes
`TraversalQueue::is_empty()`/`is_applying()` but not a "a valid traversal is pending" outcome signal.

**Spec basis.** Same as A (§7.4.2.2 step 19 — a navigation, including a link default, is ignored while a
traversal is **ongoing**); an activation that resolves to a valid traversal must not *also* run the link's
default. Step 19 gates strictly on *ongoing* navigation; the *pending* (deferred/enqueued-but-not-yet-applied)
case elidex handles here is A's **bounded approximation** (a queued in-range traversal treated as
supersede-eligible), not a direct step-19 mandate — cross-reference Resolution A.

**Decision (cross-turn-robust — E1).** The shell's own-context signal becomes **applied OR a traversal is
pending in the queue after Phase 1**. The suppression predicate must read the **queue's Traversal-pending
state (this-turn OR still-queued cross-turn)**, not a this-turn-only enqueue bool. The naive
"`deferred_own_context = a traversal was enqueued THIS turn`" is **insufficient**: a Turn-2 `<a href>` click
whose handler runs `location.href='/c'` while a **Turn-1 traversal is still queued** (no intervening
`run_deferred_traversals` pump yet) sets `own_context_action = false` (F1 drops `/c`) AND
`deferred_own_context = false` (no NEW enqueue this turn) → the link default would fire even though the
still-pending traversal should supersede everything this turn. **Robust predicate:** suppress the default
**iff `own_context_action || <the queue holds a `Traversal` step after `drain_synchronous_phase`>`**. This
closes the gap **by construction** (a queue query robust across turns) rather than by pump-timing
reachability — defensible under §7.4.2.2 step-19 (a nav/default is ignored while a traversal is pending).
Resolution E's peek-classify guarantees a no-op `go(999)` never leaves a `Traversal` step in the queue, so it
does NOT over-suppress a legitimate default. This refines B to be **cross-turn-robust**.

**Change.** Keep `DrainOutcome.deferred_own_context` if it reads cleanest, but its VALUE for the suppression
read must reflect the **queue's Traversal-pending state** (`traversal_queue` holds a `Traversal` step after
`drain_synchronous_phase`), not just this turn's enqueue — `run_synchronous_phase_body` sets it from the
post-partition queue state. Content wiring reads `own_context_action || deferred_own_context` (=
queue-Traversal-pending) at the suppression site. Alternatively expose a `DrainHost::traversal_queue`
`has_pending_traversal()` query the shell consults directly — §6 Q-shape.

### Resolution D — deferred `SyncUpdate` document binding

**Defect.** A `SyncUpdate` deferred behind a same-turn **document-changing** traversal (`back();
pushState('/x')`) is replayed in Phase 2 as a bare `HistoryAction` via `handle_history_action`
(`traversal_queue.rs:565`); the content `apply_push_replace_state` path (`content/navigation.rs`) reads/writes
`state.pipeline.url` + the active runtime = the **active** pipeline. After a document-changing (Rebuild)
traversal rebuilt `state.pipeline`, the OLD document's `pushState` mutates the **NEW** document's identity =
bug (umbrella §5 CARRY-EXT-2 (D)).

**Spec basis** (webref-pinned): the full reconciliation is `finalize a same-document navigation` =
**§7.4.2.3.3 Fragment navigations** (`#finalize-a-same-document-navigation`) + the "jump the queue" ordering
inside `apply the history step` = **§7.4.6.1 Updating the traversable** (`#apply-the-history-step`), verbatim:
*"Synchronous navigations that are intended to take place before this traversal jump the queue at this point,
so they can be added to the correct place in traversable's session history entries **before this traversal
potentially unloads their document**."* The joint-history append ops both live in **§7.4.1.3 Centralized
modifications of session history** (`#centralized-modifications-of-session-history`). That
document-identity-preserving jump-the-queue reconciliation is exactly what slot
`#11-sync-navigation-steps-queue-tagging` fences.

**Decision (GENERALIZED — Codex PR#469 R6; supersedes the earlier document-changing-only scope):** a
`SyncUpdate` that STRADDLES **any** same-turn traversal apply — same-document OR document-changing — is
**CANCELED** (dropped, not applied against the post-traversal cursor). The earlier scope (cancel behind a
*document-changing* traversal only, let a same-document straddle apply) was **spec-wrong on the entry/index**:
a straddle sync applied against the post-traversal cursor lands the update on the **traversal target**,
corrupting the current entry. Example: from `[base, /a]` at `/a`, `history.back(); history.replaceState(null,
'', '/x')` — `back()` (same-document, `document_changed` stays false under the old model) applies moving the
cursor to `base`, then the deferred `ReplaceState` applied against `base` lands `/x`-current with list `[/x,
/a]` instead of leaving `base` current with list `[base, /x]`. **This is a REACHABLE corruption** (needs no
service worker). Canceling drops the straddle update but preserves **coherent state** — correct cursor
(`base`) + correct current entry (`base`), list `[base, /a]`; the only divergence is the lost straddle
`replaceState` (a bounded, documented divergence), NOT a corrupt `/x`-current entry.

**Root (self-root-check, ≥2 rounds on the deferred-SyncUpdate-straddle mechanism — R3 T3 call-time-URL,
R4 :744/:540 cross-turn context, R6 :803 call-time-entry):** the correct behavior is WHATWG HTML §7.4.1.3
"Centralized modifications of session history" **jump-the-queue** — a synchronous navigation step that
straddles a traversal must apply to the **call-time entry BEFORE** the traversal moves the cursor. The R3 T3
fix (capture the deferred `SyncUpdate`'s call-time URL and APPLY it after the traversal) was a **piecemeal
symptom-patch on the apply-after model** — it fixed the URL but not the entry/index, and completing it
piecemeal (index/doc-changed snapshots) is exactly the ad-hoc edifice this plan committed against. That full
tagged-queue reconciliation stays **NAMED-FENCED to `#11-sync-navigation-steps-queue-tagging`** (umbrella §7
Q-SYNC-FINALIZE, edge-dense — `/elidex-plan-review` mandatory).

**Change (interim, coherent bounded divergence).** The Phase-2 `drain_traversal_queue` loop tracks a monotonic
`traversal_applied` latch: once **ANY** traversal step has applied this drain (same-document OR
document-changing), every subsequent `PendingHistoryStep::SyncUpdate` step is **CANCELED** (skipped) instead
of calling `handle_history_action`. This SUPERSEDES the earlier `changed_document`-discriminated cancel: the
`TraversalApplyOutcome { shipped, changed_document }` return collapses back to `apply_traversal -> bool`
(shipped), and T3's `normalize_deferred_sync_update` seam + the content `normalize_deferred_history_url` helper
are **removed** (the straddle `SyncUpdate` is now always canceled, never applied — the URL-binding they existed
to fix is moot). The trailing sync still defers onto the queue in issue order (I2 partition unchanged); the
cancel is the single Phase-2 home, uniform across same-turn and cross-turn straddles.

**Bounded divergence + fence + test.** The full call-time-entry jump-the-queue reconciliation is NAMED-FENCED
to `#11-sync-navigation-steps-queue-tagging`. **Conformance tests** pin the cancel (pinned-not-silent):
`traversal_queue_tests::syncupdate_canceled_after_{document_changing,same_document}_traversal` +
`content_history_phase_sep_tests::deferred_syncupdate_canceled_behind_same_document_traversal` (the re-anchored
former T3 test).

### Resolution (loop-bound) — BOUNDED SNAPSHOT shipped (Codex PR#469 R3 T1); reentrancy-guard wiring stays Slice 4

`drain_traversal_queue` (`traversal_queue.rs` `fn drain_traversal_queue` ~`:719`) drains a **bounded snapshot**,
not a re-check-until-empty loop: it captures `let mut remaining = host.traversal_queue().pending_len();` once and
runs `while remaining > 0 { remaining -= 1; … pop_next() … }` (`:745`), processing **only** the steps present at
drain-start. A step enqueued **during** the drain — a reentrant SW-pump message serialized onto the back of the
queue — is left for the **next** `run_deferred_traversals` turn, so the loop **terminates by construction** even
against a host that re-enqueues on every apply. Liveness is preserved because **content-mode's async pump drains
Phase-2 every turn** (`event_loop.rs` top-of-loop `run_deferred_traversals`), not by draining to exhaustion. In
content mode the only re-enqueue vector is the reentrant SW-fetch message pump (`content/navigation.rs` SW-fetch
relay; the reentrancy-vector doc note records it as **DEAD/unreachable** today — "the SW controller path is
dead"), and content's `apply_traversal` (a Rebuild / same-document apply) does **not** re-enqueue mid-apply, so
even the next-turn deferral is inert here.

**Decision.** The loop **bound is IMPLEMENTED in this slice** (bounded snapshot above). It adds a **test
asserting content's `apply_traversal` does not re-enqueue** (bounded-in-practice) plus a test that a mid-apply
re-enqueue is deferred, not drained to exhaustion. What **stays Slice 4** is only the reentrancy-guard *wiring* —
serializing a reentrant **DIRECT** nav message via `running_nested_apply_history_step` (it does not consult
`is_applying()` today) + the `commit_index` `debug_assert` retirement — NOT the loop bound. Stated explicitly so
plan-review sees the split: **bound = shipped; guard wiring = conscious Slice-4 fence.**

⚠ **App-mode (Slice B) liveness caveat.** This bounded-drain liveness argument is **content-mode-specific** — it
rides the every-turn async pump. App-mode (Slice B) drains via `drain_same_turn` (`app/events.rs:89`/`:131`
synchronous end-of-input-handler) with **no async pump**, so a step serialized mid-apply would strand until the
next input event. **Slice B must NOT adopt the bounded drain as-is** without either (a) re-adding an
end-of-handler re-check that re-drains the residual snapshot, **or** (b) keeping the reentrancy vector dead
(no source that re-enqueues mid-apply). Captured here so the constraint is not lost when Slice B is built (see
also §0 scope fence "OUT → Slice B" and umbrella §4.5 I3 app-mode caveat).

---

## §2 The substrate + coordinator changes (extend, engine-agnostic)

All changes keep the substrate **engine-agnostic** (Layering mandate): the coordinator owns *ordering* + the
§4.5 I1/I2/I3 invariants; the host owns *entry-list resolution* + document identity + frame bodies. No
DOM/selector/form/algorithm logic crosses into `elidex-navigation` — only ordering + host seams.

1. **New `DrainHost` seam — peek-classify (E).** `fn classify_traversal(&mut self, delta: TraversalDelta)
   -> Option<PendingTraversal>` (named `classify_traversal` to avoid colliding with
   `NavigationController::resolve_traversal(target_index) -> TraversalKind`, `navigation.rs:344` — F3). Host
   consults `NavigationController::peek_back`/`peek_forward`/`peek_go`, returns `Some(PendingTraversal { delta,
   user_involvement: <host-supplied> })` in-range, `None` no-op. `PendingTraversal` construction moves out of
   `run_synchronous_phase_body:393` into the host.
2. **Phase-1b partition change — in-range-only barrier (E).** On a `Back`/`Forward`/`Go` action,
   `run_synchronous_phase_body` calls `host.classify_traversal(delta)`: `Some(pt)` → `enqueue_traversal(pt)` +
   `seen_traversal = true` + set `deferred_own_context`; `None` → **fall through** (no enqueue, no barrier,
   subsequent same-turn sync/nav stay in-task). Cross-turn `seen_traversal` seed from a non-empty queue
   (`:388`) is preserved.
3. **Phase-1c nav-suppression (A) — drain-and-DISCARD (F1).** After Phase-1b, when the queue holds a pending
   `Traversal` step, the coordinator does **NOT skip** Phase-1c — it drains the `pending_navigation` slot and
   **discards** it (via `DrainHost::discard_pending_navigation()` or `handle_navigation(suppress: true)` — §6
   Q-shape), so the VM slot never strands and re-fires a turn late. Otherwise `handle_navigation()` applies
   normally. No-ops never enqueue (E), so they never suppress. The discard also covers the cross-turn seed
   (a Turn-1 traversal still queued in Turn-2, `:388`).
4. **`DrainOutcome.deferred_own_context: bool` (B).** New field, distinct from `own_context_action` /
   `shipped`. Set when `classify_traversal` returns `Some`. Consumed by the content default-suppression site.
5. **Phase-2 `SyncUpdate` cancellation behind ANY traversal (D — GENERALIZED, R6).** `drain_traversal_queue`
   tracks a monotonic `traversal_applied` latch and cancels (skips) subsequent `SyncUpdate` steps once **any**
   traversal (same-document OR document-changing) has applied. `apply_traversal` returns a plain `bool`
   (shipped) — the `TraversalApplyOutcome.changed_document` discriminator + T3's `normalize_deferred_sync_update`
   seam are removed (superseded — a straddle sync is always canceled, never applied). Full call-time-entry
   jump-the-queue → `#11-sync-navigation-steps-queue-tagging`.

Unchanged: `TraversalQueue` shape (the `running_nested_apply_history_step` guard stays observational this
slice — its *wiring* is Slice 4), the ship-once `ship_if_needed` funnel, the split entry points
(`drain_synchronous_phase` / `run_deferred_traversals` / `drain`), the I2 issue-order-preserving partition.

---

## §3. Spec coverage map

The **subset of the umbrella §3 surface this slice actually touches** (single top-level traversable;
multi-navigable fan-out OUT/B1). Section labels use the webref **section titles** (anchors:
`#beginning-navigation` / `#reloading-and-traversing` / `#navigate-non-frag-sync` /
`#updating-the-traversable` / `#traversable-navigables`; all §↔title pairs webref-verified — no drift).
`Full enum?` = ✓ when the row's in-scope branches are exhaustively covered by this slice; `PARTIAL` / `n/a
(B1)` mark fenced or B1-gated rows. `User-input flow` = a JS/history-API caller reaches it.

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §7.4.2.2 Beginning navigation | 19 | ongoing navigation is "traversal" ⇒ navigate ignored | IN — Resolution A: Phase-1c nav-suppression when a `Traversal` step is pending (`traversal_queue.rs:418` predicate change) | ✓ | yes (`location.*`) |
| WHATWG HTML §7.4.2.2 Beginning navigation | 20 | later navigation aborts other *navigations* (not a traversal) | IN — Resolution A: traversal wins the same-turn straddle; exact cross-channel issue order fenced | ✗ (bounded — `#11-sync-navigation-steps-queue-tagging`) | yes (`location.*`) |
| WHATWG HTML §7.4.3 Reloading and traversing | 4 | append traversal steps to traversable → resolve `targetStepIndex` | IN — Phase-1b enqueue via new `classify_traversal` seam | ✓ | yes (back/forward/go) |
| WHATWG HTML §7.4.3 Reloading and traversing | 4.4 | `allSteps[targetStepIndex]` does not exist ⇒ abort (no-op) | IN — Resolution E peek-classify: `peek_*` → `None` = no barrier, falls through | ✓ | yes (back/forward/go) |
| WHATWG HTML §7.4.4 Non-fragment synchronous "navigations" | 3–11 | *URL and history update steps*: build `newEntry`, on push bump index+length, set URL/active entry | IN — Phase-1b sync updates (`PushState`/`ReplaceState`) in-task via `handle_history_action` | ✓ | yes (pushState/replaceState/`location.*`) |
| WHATWG HTML §7.4.4 Non-fragment synchronous "navigations" | 12–13 | append *synchronous navigation steps* (finalize + BiDi) to the traversable's ONE tagged queue | PARTIAL — Resolution D (GENERALIZED, R6): a deferred `SyncUpdate` behind ANY same-turn traversal is CANCELED (bounded); full call-time-entry jump-the-queue reconciliation fenced | ✗ (bounded — `#11-sync-navigation-steps-queue-tagging`) | yes |
| WHATWG HTML §7.4.6.1 Updating the traversable | 1 | *apply the history step*: assert running within traversal queue | IN — the queue-serialized Phase-2 apply (guard observational this slice) | ✓ | no |
| WHATWG HTML §7.4.6.1 Updating the traversable | 3/4/6/7 | initiator sandbox check / cross-doc navigable set / `changingNavigables` / nonchanging siblings | OUT (B1) — always `{top-level}`; the fan-out fence | n/a (B1) | no |
| WHATWG HTML §7.4.6.1 Updating the traversable | 8 | per-navigable: set current entry; queue global task on the navigation-and-traversal task source | IN (top-level only) — the single Phase-2 apply task, scheduled on `run_event_loop` (`event_loop.rs:21`) | ✓ | no |
| WHATWG HTML §7.4.6.1 Updating the traversable | 12 + 12.4 | two-part split ("processed before documents unload"); 12.4 update-only when `displayedEntry is targetEntry` | IN — the decisive phase-separation (Phase 1 lands, Phase 2 applies) | ✓ | no |
| WHATWG HTML §7.4.6.1 Updating the traversable | 14.1.1 | *synchronous navigations jump the queue … before this traversal potentially unloads their document* | PARTIAL — Resolution D (GENERALIZED, R6) cancel-behind-any-traversal is the bounded stand-in; call-time-entry jump-the-queue reconciliation fenced | ✗ (bounded — `#11-sync-navigation-steps-queue-tagging`) | no |
| WHATWG HTML §7.3.1.1 Traversable navigables | queue obj | session history traversal **queue** object | IN — `TraversalQueue` on `NavigationController` (cooperative deferred task, not an OS thread) | ✓ | no |
| WHATWG HTML §7.3.1.1 Traversable navigables | queue obj | **"running nested apply history step" boolean** (init false) | PARTIAL — present + observational; reentrancy-guard *wiring* + termination = Slice 4 (loop-bound inert) | ✗ (Slice 4) | no |

**Breadth**: K=1 spec (html), M=13 rows.

**Split decision**: spec-breadth alone reads single-PR, but the umbrella's split driver is the
**canonical-algorithm-absent / edge-dense** rule (umbrella §2 = 5 intersecting invariant axes, §3 breadth
verdict), NOT K/M — K/M under-counts the design edge-density. So the split is by **implementation slice under
the approved umbrella** (this memo = the co-designed Slice A, a terminal per-PR slice), consistent with
umbrella §3/§5.

### §3.1 User-input touch audit

- **Synchronous updates (§7.4.4, Phase 1 in-task):** `history.pushState()` / `history.replaceState()` /
  `location.href=` / `location.assign()` / `location.replace()` / `location.reload()` — staged to
  `vm/host/navigation.rs` `pending_history` (push/replace) + `pending_navigation` (`location.*`).
- **Traversals (§7.4.3 → §7.4.6.1, Phase 2 deferred):** `history.back()` / `history.forward()` /
  `history.go(delta)` — staged to `pending_history` as `Back`/`Forward`/`Go`, classified by the new
  `classify_traversal` peek seam (in-range = barrier; no-op falls through).
- **Chrome-button traversals** (`app/navigation.rs:596` `handle_chrome_action`) reach the same `peek_*` /
  `traverse_to` path with `UserInvolvement::BrowserUi` — but app-mode wiring is **Slice B**, not this slice.

---

## §4 Content-mode wiring

**`impl DrainHost for ContentState`** (seam → content site):

| Seam | Content realization |
|---|---|
| `traversal_queue` | `&mut` a `TraversalQueue` field homed on `NavigationController` (survives pipeline rebuild, `navigation.rs:67`) |
| `route_window_opens` | existing `route_window_opens(state, take_pending_window_opens())` (`navigation.rs:542`) |
| `take_pending_history` | `state.pipeline.runtime.take_pending_history()` (`:568`) |
| `handle_history_action` | the **sync-update-only** arm of the existing `handle_history_action` (`:777`) — `PushState`/`ReplaceState`; the `Back`/`Forward`/`Go` arms move OUT to `apply_traversal` |
| `handle_navigation` | the Phase-1c `take_pending_navigation` + `resolve_nav_url` + `handle_navigate` block (`:609`–`:628`); on suppress, takes-and-drops `pending_navigation` without applying (F1) |
| `classify_traversal` (NEW) | `peek_back`/`peek_forward`/`peek_go` on `nav_controller`; `Some` with `UserInvolvement::None` (scripted), `None` on out-of-range |
| `apply_traversal` (NEW) | the peek-then-commit + `handle_navigate` body currently in `handle_history_action`'s traversal arms, returning `bool` (shipped) — the `changed_document` discriminator was removed when D generalized (R6) |
| `ship_frame` | `state.send_display_list()` |

**Drain rewiring:**
- **Replace `process_pending_actions`** (`navigation.rs:529`). Input handlers call
  `DrainCoordinator::drain_synchronous_phase(state)` **in-task**; `run_event_loop`
  (`content/event_loop.rs:21`, where window-opens already route at `:204`–`:205`) calls
  `DrainCoordinator::run_deferred_traversals(state)` each pump turn (Phase 2 — the
  navigation-and-traversal task source realization, umbrella §4.3 Q-SCHED content resolution).
- **Remove the `:593` supersede-`return true`** (and the now-unreachable fall-through comment `:583`–`:592`).
  Traversals defer to Phase 2; trailing sync updates replay via the I2 partition.
- **Nav-suppression is drain-and-DISCARD, never skip (F1).** When Phase-1b left a `Traversal` step pending,
  Phase-1c still drains `pending_navigation` (the slot's only drain) and drops it — so the suppressed
  `location.*` nav does not strand and re-fire on the next turn.
- **Wire default-suppression consumer** (`event_handlers.rs:174` click, `:389` keyboard): the call sites
  currently branch on the `bool`; rewire to `let out = drain_synchronous_phase(state); if out.own_context_action
  || out.deferred_own_context { return; }` (click) / the inverted form at `:389` (keyboard).

**1000-line touch-time-split obligation (F8).** `content/navigation.rs` is 904 LoC; this wiring may push it
past 1000. If it crosses, the touch-time split is the umbrella §5 Slice-0 `handle_navigate` /
`same_document_step` sibling-module carve as a **separate standalone prereq PR** (NOT bundled into this
slice's PR, NOT the declined drain-unification), per CLAUDE.md "1000-line debt = touch-time split."

**Ownership/layering note.** `NavigationController` + the `TraversalQueue` stay engine-agnostic; `ContentState`
/ pipeline / `EcsDom` stay behind the `DrainHost` trait (never cross the crate boundary). Single-writer
renderer-main-thread discipline is preserved — Phase 2 is a *cooperative deferred task on the existing event
loop*, not an OS thread (umbrella §4.1). **Side-store exception (b) classification (F4):**
`NavigationController.entries`/`index` + `TraversalQueue` + `DrainOutcome` + the VM
`pending_history`/`pending_navigation` channels are CLAUDE.md **side-store exception (b)** —
browsing-context/session-level state, NOT per-entity ECS components — so the non-ECS-component choice is
justified in-plan (`#11-browsing-context-state-ecs-components` owns any wholesale migration).

---

## §5 Test strategy (supported-surface: flip, don't delete)

- **Flip the supersede cases in `content_history_drain_tests.rs`** to the task-boundary expectation with
  §7.4.6.1-step-12 cites (the regression **changes shape, does not disappear**). The success-path complement
  of `failed_traversal_does_not_block_same_turn_navigation_drain:447` flips: a *successful* in-range traversal
  now defers (Phase 2) rather than superseding in one synchronous pass; re-anchor the supersede/order tests to
  the phase-separated order. `failed_traversal_load_does_not_drop_trailing_history:351` stays green under E
  (no-op falls through).
- **NEW phase-sep test:** `pushState('/a'); history.back()` in one turn ⇒ `/a` committed to the
  `NavigationController` in Phase 1, THEN the traversal applies in Phase 2 against the **updated** entry list
  (pins I1 ordering across the task boundary).
- **NEW nav-vs-traversal supersede conformance test (A):** `history.back(); location.href='/b'` ⇒ Phase-1c
  nav suppressed, no `/b` load/flash, land on the back target; document whichever behavior ships for the
  reverse cross-channel order (bounded divergence, pinned-not-silent).
- **NEW no-op peek-classify test (E):** `go(999); pushState('/x')` at end-of-history ⇒ the no-op does NOT
  defer the trailing push (it applies in-task) and does NOT suppress a same-turn nav/default.
- **`SyncUpdate`-cancellation conformance tests (D — GENERALIZED, R6):** `back(); replaceState('/x')` across a
  same-document traversal ⇒ the deferred `/x` update is **canceled** (lands on the back target `base`, list
  `[base, /a]` unchanged — NOT the corrupt `/x`-current); the document-changing variant cancels via the SAME
  code path. Substrate isolation pins both scenarios
  (`syncupdate_canceled_after_{document_changing,same_document}_traversal`); the content-level re-anchored
  former-T3 test (`deferred_syncupdate_canceled_behind_same_document_traversal`) pins the reachable
  same-document corruption. (Earlier plan text let a same-document straddle *apply* — superseded: that was the
  R6-caught corruption.)
- **NEW cross-turn default-suppression conformance test (B cross-turn-robust, E1):** a Turn-1 `history.back()`
  left queued (no intervening Phase-2 pump), then a Turn-2 `<a href>` click whose handler runs
  `location.href='/c'` ⇒ the link default is **suppressed** (the still-pending traversal supersedes; F1 drops
  `/c`) — pins the queue-Traversal-pending predicate across turns (§7.4.2.2 step-19).
- **NEW stacked-traversal conformance test (E divergence, F5):** `back(); back()` ⇒ pins the bounded
  behavior where the 2nd traversal peeks the unmoved cursor and may over-set `deferred_own_context` while its
  Phase-2 apply is a no-op that ships nothing — documenting the accepted divergence (pinned-not-silent, no
  `#11-*` slot — §6 Q-E).
- **NEW loop-inert test (loop-bound):** assert content's `apply_traversal` does not re-enqueue
  (bounded-in-practice; the structural guard is Slice 4).

App-mode parity is **Slice B** (out of scope here); no new app-mode history-drain test file exists yet (only
`app_fragment_nav_tests.rs`).

---

## §6 Open questions for `/elidex-plan-review`

- **Q-D — `SyncUpdate` cancel scope. RESOLVED + GENERALIZED (Codex PR#469 R6).** Recommend was **cancel** for
  the document-changing case; R6 showed the same-document straddle *applying* is a REACHABLE corruption (lands
  the update on the traversal target, corrupting the current entry). Resolution: cancel a deferred `SyncUpdate`
  behind **ANY** same-turn traversal (interim bounded divergence — the straddle update is dropped, coherent
  state preserved). The correct §7.4.1.3 jump-the-queue application to the call-time entry is
  `#11-sync-navigation-steps-queue-tagging`.
- **Q-shape — the exact `DrainOutcome` / seam shapes.** `classify_traversal -> Option<PendingTraversal>` vs a
  bool "in-range?" seam; the nav-discard seam (A/F1) as `discard_pending_navigation()` (take-and-drop) vs
  `handle_navigation(suppress: bool)` (still drains, does not apply); `apply_traversal` return (**resolved
  R6:** plain `bool` shipped + a coordinator-tracked `traversal_applied` latch — the earlier
  `TraversalApplyOutcome { shipped, changed_document }` was superseded when D generalized to cancel behind ANY
  traversal); `deferred_own_context` as a `DrainOutcome` field vs a queue query.
- **Q-A — nav-suppression predicate.** "queue holds a pending `Traversal` step" — does it correctly handle
  **both** same-turn orders (`back(); location.href` and `location.href; back()`) acceptably given staging
  discarded cross-channel issue order? Confirm the bounded divergence is acceptable + fenced.
- **Q-E — stacked-traversal peek-classify imperfection.** `back(); back()` peeks the unmoved cursor twice, so
  the 2nd may classify in-range yet apply as a no-op (over-setting `deferred_own_context`). **Accepted as
  bounded** (the apply still ships nothing) — **pinned by the §5 stacked-traversal conformance test, NOT
  slotted** (an accepted bounded behavior is not a platform gap and fails the slot-fit audit). Confirm accept
  vs require cursor simulation.
- **Q-fence — app-mode-separate + landing-proximity.** Confirm app-mode is Slice B (distinct deployment
  shell, bounded strangler not runtime conflict) and Slice B lands in close succession (umbrella §5 axis c).

---

## §7 What it closes / defers

| | Item | Disposition |
|---|---|---|
| **Closes** | #396-root double-apply (Phase-1c nav + Phase-2 traversal both paint) | Resolution A — nav suppressed when an in-range traversal is pending |
| **Closes** | CARRY (Slice-2) — nav-vs-traversal supersede under-analyzed | Resolution A — bounded supersede, conformance-pinned |
| **Closes** | CARRY-EXT (A) — double apply-body / no supersede | Resolution A |
| **Closes** | CARRY-EXT (B) — default fires over a pending deferred traversal | Resolution B — `deferred_own_context` |
| **Closes** | CARRY-EXT-2 (E) — no-op traversal false partition barrier | Resolution E — peek-classify |
| **Closes** | CARRY-EXT-2 (D) — deferred `SyncUpdate` document binding | Resolution D (GENERALIZED, R6) — cancel a straddle `SyncUpdate` behind ANY same-turn traversal (bounded) |
| **Defers → Slice 4** | loop-bound (unbounded re-check-until-empty) + reentrancy-guard wiring + `commit_index` `debug_assert` retirement | inert this slice (SW pump dead); loop-inert test only |
| **Defers → Slice B** | app-mode phase-separation (Q-SCHED end-of-input-handler drain) | distinct shell; lands in close succession |
| **Defers → `#11-sync-navigation-steps-queue-tagging`** | full issue-order nav-channel integration (A) + document-identity jump-the-queue reconciliation (D) | §7.4.1.3 / §7.4.2.3.3 / §7.4.6.1 tagged-queue machinery; **A's cross-channel leg additionally needs a Q-VM-MODEL reopen** (co-homed, F7) — the slot charter is the tagged-queue reconciliation, and A's `location.*` cross-channel integration needs VM staging to preserve cross-channel issue order |
| **Defers → B1** | multi-navigable fan-out (§7.4.6.1 steps 3/4/6/7 + per-navigable global-task) | `changingNavigables = {top-level}` today |
