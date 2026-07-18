# Plan-memo — Slice-A pump-turn drain unification (retire the reentrant-replay 2nd channel)

> Status: **pre-implementation DESIGN plan-memo**. Goes through `/elidex-plan-review` before any
> implementation. Anchored on the first-principles ideal (one message intake + one ordered apply/drain
> structure per turn), not a surgical patch.
> Lineage: PR #469 `/external-converge` R13 root-check (adopted) + a 5-agent `/elidex-plan-review` that
> **refuted the first §4 draft's peek-gate mechanism** (3 IMP). This revision re-derives §4 peek-less.
> **Scope: content-shell-only. Explicitly NOT Slice 4** (§5).

---

## §0 Decision + scope

**Thesis.** `event_loop.rs::replay_deferred_reentrant_messages` is a **second navigation-mutation dispatch
channel** — a mini-event-loop that batch-runs `handle_message` at the top of `pump_turn`
(`event_loop.rs:73`) WITHOUT the pump's per-turn phase brackets (the top `run_deferred_traversals` Phase-2
apply and the bottom `drain_synchronous_phase` Phase-1 drain). It collapses the I1 task boundary (:416) and
races the bottom drain (:73). It is the CLAUDE.md "Concurrency by ownership and phases" **辻褄合わせ
anti-pattern** — a manual message-ordering handshake instead of one structural single-writer intake/apply
order. R3→R13 have paid its ordering off one hand-placed check at a time (**four** `shutdown_requested`
re-checks: `event_loop.rs:55/86/121/372`).

**Decision.** RETIRE the second channel and restructure `pump_turn` so a single **message-held** turn
skeleton resolves shutdown-priority (:49), popstate-intent-drain (:73, fresh AND buffered), and
queued-traversal-before-direct-nav (:416) **by construction, with no state-inspection peek-gate on dispatch
order**. The turn: **intake one message (buffer-first, else one `recv_timeout`) and HOLD it → if it is a
`Shutdown`, tear down before any Phase-2 work → apply the deferred traversal and settle its
synchronously-staged history intent as one atomic unit → THEN dispatch the held message → frame tick → the
R9 bottom drain.** The `is_applying()` buffer guard (`dispatch_or_buffer_reentrant`) is KEPT unchanged; only
the buffer's *drain timing* changes (folded into the one intake, one message per turn).

**Why the first draft was refuted (recorded so the gate sees the correction).** Draft-1 kept Phase-2 at the
literal top and *gated buffered delivery* on `has_pending_history`/`has_pending_navigation` peeks — itself a
hand-placed ordering conditional (IMP-1), it covered only *buffered* (not fresh) nav messages so :73 stayed
severable on a fresh recv (IMP-3), and its `:49` `try_recv` fast-path dropped non-`Shutdown` messages because
crossbeam has no peek/putback (IMP-2). The message-**held** skeleton eliminates all three: one intake reads
at most one message and *holds* it (never drops — IMP-2), the hold makes the fix uniform for fresh and
buffered messages (IMP-3), and the ordering is fixed unconditionally with no `has_pending_*` peek (IMP-1).

**Scope fence (plan-review verifies each leg):**
- **IN:** retire `replay_deferred_reentrant_messages`; single held-message intake (buffer-first / one
  `recv_timeout`); Shutdown-priority via the held message; a top drain that settles the Phase-2 popstate
  intent atomically; keep the R9 bottom drain; **edit the `drain_host.rs::dispatch_or_buffer_reentrant`
  docstring** (retire its `:46-48` / `:66` `replay_deferred_reentrant_messages` references — function body
  unchanged); flip the three `replay_*`-shaped tests; add three R13 regression tests.
- **OUT → Slice 4:** the canonical reentrant-message *serialization* (§7.3.1.1 running-nested-apply guard
  WIRING for a reentrant DIRECT nav; §7.4.1.3 tagged-queue routing; `commit_index` `debug_assert`
  retirement). Untouched (§5).
- **OUT → Slice B (app-mode):** app has no async pump. Content-mode only.
- **OUT → B1:** multi-navigable fan-out.

---

## §1 The root (why :73 and :416 share ONE root; :49 is a distinct facet)

Today `pump_turn` has four nav-mutation opportunities per turn, in order: (1) top Phase-2 apply
`run_deferred_traversals` (`:49` in file) — a same-document traversal fires `popstate` **synchronously**,
whose handler may stage `pushState`/`location.*` in the VM `pending_history`/`pending_navigation` buffers;
(2) the replay batch `:73`; (3) recv `handle_message` `:113`; (4) the sole staged-intent drain
`drain_synchronous_phase` `:363`.

- **:416 (I1 collapse).** The replay batch dispatches `[A queues history.back()→Traversal, B=direct Navigate]`
  back-to-back with **no Phase-2 apply between them**, so the queued traversal is unapplied when B rebuilds →
  B overtakes it. The normal pump lands A and B on *separate* turns and applies the traversal at the top of
  B's turn first.
- **:73 (bottom-drain race).** Step 1's popstate stages a `pushState`; a nav-mutating message (batch **or**
  fresh recv — IMP-3: the fresh `:113` recv already sits between step 1 and the `:363` drain) rebuilds the VM
  before step 4 reads the old VM's buffers → intent lost.
- **Shared root:** any nav-mutating message dispatched between the Phase-2 popstate-staging and the
  staged-intent drain severs the intent, and the replay batch additionally packs multiple such dispatches
  into one turn with no Phase-2 between them. Both vanish if the (single, held) message is dispatched **after**
  Phase-2 and after the popstate intent is drained.
- **:49 is a DISTINCT facet (teardown-priority).** A `Shutdown` already queued at turn start is not the
  replay root — it is the ordering of the normal channel's pending `Shutdown` vs the top Phase-2 apply.

---

## §2 Coupled-invariant enumeration (edge-dense plan — REQUIRED)

Four axes the restructure simultaneously satisfies:
- **(a) shutdown/teardown-priority** — a queued teardown runs before further script/network on a closing tab;
  a torn-down pipeline is never mutated.
- **(b) phase ordering** — **I1** (a deferred apply is exposed only on a *later* turn) + **I2** (one ordered
  intake + one ordered FIFO drain SoT; no second message-dispatch channel).
- **(c) reentrancy-buffer window-closure** — the `is_applying()` peek→commit window stays closed (interim
  guard KEPT).
- **(d) input-vs-popstate-nav ordering (the plan-review MISS, fixed by the drain-split).** An
  already-pending held input (a `MouseClick`/`KeyDown` — an *older task*) must be dispatched against the
  document that was current when it was queued, NOT against a document a **same-turn** popstate-staged
  CROSS-document navigation (`location.assign`, a *later task*) would rebuild to. Draft-2 ran the WHOLE
  Phase-1 body at the top drain (incl. Phase 1c `handle_navigation`), so a popstate-staged cross-document nav
  blocking-loaded + rebuilt `state.pipeline` at step 3, BEFORE step 4's held input → the input hit the wrong
  document (spec-divergent; the OLD pre-restructure code drained `pending_navigation` only at the bottom,
  after the message, and was spec-aligned).

Pairwise intersections:
- **a × b:** the held-message check `if msg == Shutdown → teardown → Break` sits BEFORE Phase-2, so teardown
  wins over the deferred-apply — using the same `Break` the phase model already uses; no extra ordering
  conditional (the held message is already in hand, so no channel peek).
- **a × c:** `dispatch_or_buffer_reentrant` still short-circuits `Shutdown` immediately (never buffers it), so
  the buffer is provably Shutdown-free; a re-delivered non-`Shutdown` message flows through the ONE
  `handle_message` intake and inherits the normal `Shutdown`→`Break` contract → the retired replay's bespoke
  `ControlFlow` exit-propagation is unnecessary.
- **b × c:** the buffer becomes a *deferral queue feeding the one intake* (one message per turn), not a
  *parallel drain* — one writer, one ordered intake; the single VM FIFO stays the drain SoT.
- **b × d (the drain-split):** the top drain is the NEW `drain_synchronous_updates` (Phase 1a window-opens +
  1b same-document `pending_history`), the bottom stays the full `drain_synchronous_phase` (1a + 1b + 1c). So
  the top settles ONLY the same-document sync intent :73 protects; a popstate-staged CROSS-document
  `pending_navigation` is drained ONLY at the bottom (step 6), AFTER the step-4 held input — the input hits
  the pre-nav document by construction (b: phase ordering realizes d).
- **c × d / a × d:** orthogonal — the cross-document nav (Phase 1c) is neither a reentrancy vector (c) nor a
  teardown (a); the split touches only which drain runs 1c, leaving the `is_applying()` guard and the
  shutdown short-circuit unchanged.

**Accretion ledger (honest before/after).**

| | current | reworked |
|---|---|---|
| nav **message**-dispatch channels | 2 (replay batch + recv) | **1** (single held-message intake) |
| `shutdown_requested` flag re-checks | 4 pump-level (`:55/:86/:121/:372`) + 1 in-replay | **3** (after Phase-2+top-drain / after held-msg / after bottom-drain) |
| retired symbols | — | `replay_deferred_reentrant_messages` + its `ControlFlow` exit-propagation doc |
| drain-primitive call sites / turn | input-handler in-task + bottom (R9) | input-handler in-task + **top (popstate)** + bottom (R9) |

The one *addition* is the top drain — but it is the single coordinator drain of the single VM FIFO invoked
at a phase boundary (draining the Phase-2 popstate intent), **not** a second message-dispatch channel. Net:
one fewer message channel, one fewer flag check, one function retired; the drain primitive gains one
phase-boundary call. No canonical algorithm exists in-tree for this pump ordering (its correctness is the
R3→R13 tail), so per CLAUDE.md this plan is `/elidex-plan-review`-gated despite the one-file blast radius.

---

## §3. Spec coverage map

Thin surface — a shell event-loop scheduling restructure, not a new normative algorithm. All §↔title pairs
webref-verified (`webref heading html 8.1.7`; anchors `#event-loop-processing-model` / `#generic-task-sources`).
`Touch` = compile/dispatch site. `Full enum?` = ✓ when in-scope branches are exhaustively covered.

| Spec section | Step / concept | Branch | Touch (dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §8.1.7.3 Processing model | 1–2 | select ONE task per loop iteration; run to completion | IN — one held message + one Phase-2 apply per `pump_turn` (`event_loop.rs:35`); a buffered reentrant message re-delivered as a LATER task | ✓ | yes (any inbound) |
| WHATWG HTML §8.1.7.4 Generic task sources | — | **navigation and traversal task source** | IN (top-level) — the deferred Phase-2 apply `run_deferred_traversals` (`event_loop.rs:49`); a held `Navigate`/`GoBack` is ordered AFTER it | ✓ | yes (back/forward/`location.*`) |
| WHATWG HTML §8.1.7.4 Generic task sources | — | **user interaction task source** | IN — a buffered `MouseClick`/`KeyDown` re-delivered one-per-turn via the single `handle_message` intake (`event_loop.rs:113`) | ✓ | yes (click/key) |
| WHATWG HTML §8.1.7.3 Processing model | 2.1 | task-**QUEUE** selection is **implementation-defined** (§8.1.7.3 step 2.1) | IN — teardown-priority (a queued `Shutdown` handled before the Phase-2 apply) is an **elidex implementation POLICY permitted by the implementation-defined selection**, NOT a spec-mandated rule | ✓ (policy) | yes (tab close) |
| WHATWG HTML §7.4.6.1 Updating the traversable | 12 (+12.4) | the apply-WORK task — "split into two parts … before documents unload"; 12.4 update-only when `displayedEntry is targetEntry` and reload-pending is false | IN (unchanged) — the Phase-2 apply this memo re-orders the held message *around*, not the apply body | ✓ | no |
| WHATWG HTML §7.3.1.1 Traversable navigables | queue obj | "running nested apply history step" boolean | OUT (Slice 4) — observational; its canonical WIRING is fenced; the shell `is_applying()` interim guard is kept | n/a (Slice 4) | no |

**Breadth**: K=1 spec (html), M=6 rows.

**Split decision**: single-PR. K/M is small AND — unlike the umbrella (5 intersecting invariants, canonical
algorithm absent → multi-slice) — this is a bounded restructure (one shell file + one small engine-agnostic
coordinator seam) whose canonical replacement (Slice 4) is explicitly out. The §2 edge-density (four axes)
mandates the `/elidex-plan-review` gate, but the axes are satisfied *within one held-message intake + one
ordered, source-partitioned apply/drain structure*; the only cross-crate surface is the clean
`drain_synchronous_updates` coordinator seam (a behavior-preserving factoring of the existing Phase-1 body,
axis d), which is a local addition, not a re-slice trigger. Terminal per-PR unit under the Slice-A/umbrella
program.

---

## §4 The reworked pump turn (peek-less, message-held)

**First-principles ideal.** A turn has exactly ONE message intake and a fixed, unconditional phase order.
Nothing inspects VM pending-intent state to *decide dispatch order* (that was draft-1's refuted peek-gate).
The order alone makes the three defects unreachable.

**Turn skeleton** (all in `event_loop.rs::pump_turn`; existing sites cited by current line):

```
1. INTAKE ONE MESSAGE, held as `msg: Option<BrowserToContent>`:
     if !deferred_reentrant_messages.is_empty()  → msg = Some(pop_front())   // buffer-first, one/turn
     else                                         → msg = channel.recv_timeout(timeout).ok()
   (timeout computed as today from animations/timers, PLUS 0 when a deferred
    traversal is pending — see "Liveness" below. `Disconnected` ⇒ Break.)
2. TEARDOWN-PRIORITY [:49]:  if msg == Some(Shutdown) → handle_message(Shutdown) → return Break
                             // runs BEFORE any Phase-2 work: no popstate/load on a closing tab
3. PHASE-2 APPLY + POPSTATE SAME-DOCUMENT SETTLE (before the held message can rebuild):
     run_deferred_traversals(state)              // applies a PRIOR turn's queued traversal [:416]
     drain_synchronous_updates(state)  // TOP drain — 1a window-opens + 1b same-doc pushState ONLY [:73],
                                        //            NOT 1c cross-doc nav (deferred to step 6 — axis d)
     if shutdown_requested → Break               // (Phase-2/drain can reach handle_navigate's SW-wait)
4. DISPATCH THE HELD MESSAGE (after Phase-2 + same-doc settle):
     if let Some(m) = msg (non-Shutdown) → handle_message(m)   // may rebuild VM; same-doc intent already safe,
                                                               // popstate cross-doc nav still pending → input
                                                               // hits the pre-nav document (axis d)
     if shutdown_requested → Break
5. FRAME TICK (timers / fetch / worker / iframe / render) — may stage navs
6. R9 BOTTOM DRAIN:  drain_synchronous_phase(state)   // FULL 1a+1b+1c — runs Phase 1c (cross-doc nav);
                                                       // + frame-tick + step-4 non-input staged navs
     if shutdown_requested → Break
7. Continue
```

**IMP-1 (no peek-gate) — resolved by construction.** Dispatch order (1→2→3→4→5→6) is unconditional; the only
VM **pending-intent** read that influences dispatch **order** is NONE. Two non-ordering state reads remain in
step 1, neither of which gates dispatch order: `traversal_queue.is_empty()` is folded into the recv
**timeout** (wait-**duration** only — the SAME category as the existing `animations_running` /
`next_timer_deadline` timeout inputs, `event_loop.rs:90-106`), and `deferred_reentrant_messages.is_empty()`
picks the intake **source** (buffer-first `pop_front` vs `recv_timeout`). One sets how long step 1 waits, the
other where step 1's single message comes from; both feed the SAME one held-message intake and leave the 1→6
order fixed. `TraversalQueue::is_empty()` (`traversal_queue.rs:226`) already exists on `ContentState`; no new
accessor.

**IMP-2 (no message loss) — resolved by construction.** Step 1 is the SOLE intake: one `recv_timeout` (or one
buffer `pop_front`) reads **at most one** message and **holds** it. There is no separate `try_recv`
Shutdown-probe — teardown-priority is decided on the already-held `msg` (step 2), so nothing is ever read to
check-and-discarded. crossbeam's no-peek/no-putback is irrelevant because we never need to put a message back.

**IMP-3 (:73 uniform for fresh AND buffered) — resolved by construction.** Whether `msg` came from the buffer
or a fresh `recv_timeout`, it is the SAME held value dispatched at step 4 — after step 3's Phase-2 apply +
same-document settle. So a fresh cross-document `Navigate`/`Reload` can no longer rebuild between the popstate
staging (step 3, `run_deferred_traversals`) and its settle (step 3, top `drain_synchronous_updates`); the
same-document intent lands in `NavigationController` (which survives pipeline rebuild) before step 4. The
half-coverage of draft-1 is gone.

**:416 — resolved by construction.** Phase-2 (step 3) applies a queued traversal BEFORE the held message
dispatches (step 4), every turn, unconditionally — a direct `Navigate` can never overtake a queued traversal.
Buffered input+direct-nav pairs are delivered one-per-turn (buffer-first, step 1), each getting its own step-3
apply, so the traversal from turn T applies at step 3 of turn T+1 before T+1's held message.

**Why two drain sites, and the ASYMMETRIC split (draft-2 correction).** Draft-2 ran the *same* whole-body
`drain_synchronous_phase` at both the top (step 3) and bottom (step 6), partitioned only TEMPORALLY. `/code-review`
(verified by an independent adversarial pass) found that over-drains: the whole body includes Phase 1c
(`handle_navigation`), so a popstate handler calling a CROSS-document `location.assign` during the step-3 apply
staged `pending_navigation`, which the top drain then applied → a blocking document load rebuilt `state.pipeline`
to `/other` BEFORE step 4's held `MouseClick`/`KeyDown` dispatched → the input hit the wrong document
(spec-divergent — the OLD code drained `pending_navigation` only at the bottom, after the message, so an
already-pending input processed against the pre-nav document; `location.assign` completes in a LATER task).
`placement_seq` staleness is viewport-only and does NOT cover a document rebuild. **The fix is the drain-split**
(faithful to §4's own stated "settle the *sync* intent"): a new coordinator seam
`DrainCoordinator::drain_synchronous_updates` runs Phase 1a (window-opens) + 1b (same-document `pending_history`)
but NOT Phase 1c; the top drain (step 3) uses it, the bottom drain (step 6) keeps the full
`drain_synchronous_phase` (1a+1b+1c). So the partition is now **ASYMMETRIC by SOURCE, not merely temporal**:
- **top (step 3)** = same-document `pushState`/`replaceState` (the :73 intent) + window-opens — committed to
  `NavigationController` before the held message, so it survives a held-Navigate rebuild.
- **bottom (step 6)** = the FULL body — runs Phase 1c (`handle_navigation`, the CROSS-document
  `pending_navigation` drain). A popstate-staged cross-document nav is NEVER drained at the top; it is drained
  at step 4's input handler (in-task Phase 1c, after the event dispatched) or here at step 6 — both AFTER the
  step-4 held input → the input hits the pre-nav document, the cross-doc nav applies as a later task.
Both seams share the same 1a/1b core (`run_synchronous_updates_body`) — the full body is `updates_body` + 1c,
no copy-paste; the FIFO-partition logic stays in `elidex-navigation`. The single VM FIFO stays the ordering SoT
(I2): the same-document `pending_history` is drained by whichever of top/bottom first takes it (take-consumed,
no double-apply); a step-3-enqueued traversal is still pending at step 6, so Resolution A/B suppression there
still fires. Input-handler in-task draining is UNCHANGED and orthogonal.

### §4.1 Validation against the mandated constraints

- **I1 (deferred apply on a LATER turn than the enqueuing turn).** Enqueue sites (a JS `history.back()`
  reaching `enqueue_traversal` via a drain) are step 3-top, step 4 (input handler in-task), or step 6 — all
  **at or after** step 3 of the same turn T. The apply is step 3 (`run_deferred_traversals`) of turn **T+1**.
  Step 3 of turn T already ran before any turn-T enqueue, so a traversal enqueued in T is applied in T+1 —
  distinct turns, by construction. (Phase-2 does not itself enqueue; only the drains do.)
- **Liveness (no traversal starvation).** Phase-2 runs at step 3 **every turn, unconditionally** — this memo
  does NOT adopt "Phase-2-only-when-idle" (evaluated and REJECTED: it starves :416 and adds a poll-interval
  latency). So under sustained message load a queued traversal still applies on the very next turn's step 3.
  The only latency nuance is step 1's blocking wait when the channel is idle AND a traversal is queued: fold
  `!traversal_queue.is_empty()` into the timeout as a `0` (like a due timer), so the idle turn returns
  immediately and step 3 applies without a poll-interval delay. This is a wait-duration input, homogeneous
  with the existing animation/timer timeout shortening — not an ordering gate. Buffer liveness: the buffer
  drains one-per-turn (buffer-first) and only ever fills during a single `is_applying()` SW-wait window
  (bounded, small); a channel `Shutdown` waiting behind it is delayed by at most (buffer length) fast turns —
  and the buffer is provably `Shutdown`-free (guard invariant), so no teardown signal is ever stuck in it.
- **R9 (bottom-drain coverage of callback-staged navs) preserved.** R9 (commit `94feacda`) moved
  `drain_synchronous_phase` to the bottom to drain timer/fetch/worker (and popstate) callback-staged navs.
  Here the bottom drain (step 6) STAYS the FULL `drain_synchronous_phase` after the frame tick and still
  covers timer/fetch/worker AND any non-input held-message handler that stages a nav (e.g. a
  `resize`/`visibilitychange` handler calling `location.*`) — AND it runs Phase 1c (cross-document
  `pending_navigation`, axis d). Only the *popstate SAME-document sync* intent settles at the top drain
  (step 3, `drain_synchronous_updates`), where it must be to precede the held-message rebuild; a popstate
  CROSS-document nav is NEVER drained at the top (that was the draft-2 regression) — it defers to step 4's
  input handler (in-task, after the event dispatched) or this step-6 bottom drain, both after the held input. No R9 source is dropped; the split is asymmetric by source
  (top 1a+1b = same-doc sync + window-opens · in-task = input handlers · bottom 1a+1b+1c = frame-tick +
  non-input message handlers + the sole cross-doc nav).

---

## §5 Explicit NOT-Slice-4 fence

Untouched, verify each leg at plan-review:
- **`TraversalQueue` / `running_nested_apply_history_step` wiring** — the shell `is_applying()` interim guard
  (`dispatch_or_buffer_reentrant`) stays as-is; only where its buffer is *drained* changes.
- **§7.4.1.3 tagged-queue routing** — NOT adopted. The R13 finding's "route the buffered messages through the
  traversal queue" phrasing IS the Slice-4 canonical and is deliberately NOT taken; "one held-message intake
  through the normal message path" achieves :73/:416 in the shell now.
- **`commit_index` `debug_assert` retirement** — untouched; still backstopped by the interim guard.

The Slice-4 canonical reentrant-message serialization remains a conscious fence (Slice-A §0; task-queue-model
§4.4). This memo makes the *interim* correct without pre-empting the canonical.

---

## §6 Test strategy

Pin the three R13 scenarios as conformance tests (content-mode `pump_turn` harness, driven turn-at-a-time, as
`content_history_phase_sep_tests.rs` interim-guard tests do):

- **(:416) queued-traversal-before-direct-nav.** Buffer `[input msg whose handler queues `back()`, direct
  `Navigate`]`; drive turns; assert the traversal applies (step 3) **before** the `Navigate` rebuild — the
  history list the traversal resolves against is the pre-`Navigate` list.
- **(:73) popstate-staged `pushState` survives, fresh AND buffered.** Queue a same-document traversal (step-3
  popstate → handler `pushState`s) and drive a turn whose held message is a cross-document `Navigate` —
  once with `Navigate` as a **fresh** channel message, once as a **buffered** reentrant message; assert in
  BOTH that the `pushState` intent is applied to the entry list (the top drain settled it before the rebuild).
- **(:49) teardown-priority.** Pre-queue a `Shutdown` on the channel AND a prior-turn queued traversal; drive
  ONE `pump_turn`; assert teardown ran (`shutdown_requested`, pump returns `Break`) and **no** popstate /
  document load fired first (e.g. zero `SwFetchRequest`, no rebuild observed).

Disposition of the three `replay_*`-shaped tests (they call the retired symbol directly): two are
**flipped** (preserving the observable contract on the new mechanism), one (`:783`) is **deleted/folded**
into its canonical sibling per One-issue-one-way:
- `interim_guard_buffered_shutdown_breaks_the_pump` (`content_history_phase_sep_tests.rs:783`) — currently
  calls `replay_deferred_reentrant_messages` with a buffered `Shutdown`. Its premise (a `Shutdown` *inside*
  the buffer) is now provably impossible (the guard never buffers `Shutdown`), and the invariant it would be
  re-purposed to assert (a `Shutdown` at `dispatch_or_buffer_reentrant` is handled immediately, never enters
  `deferred_reentrant_messages`) is ALREADY the canonical assertion of the KEPT sibling `:839`
  (`interim_guard_shutdown_handled_immediately_not_buffered`). Re-purposing `:783` would therefore just
  DUPLICATE `:839`. Per One-issue-one-way, **DELETE `:783`** (fold it into `:839` as the single canonical
  assertion site) — the break-on-nested-`Shutdown` coverage it also touched is preserved by the flipped
  `:898` below, so no coverage is lost.
- `interim_guard_shutdown_handled_immediately_not_buffered` (`:839`) — **unchanged** (asserts the KEPT
  `dispatch_or_buffer_reentrant` short-circuit); verify it still holds.
- `interim_guard_replay_stops_and_pump_breaks_on_nested_shutdown` (`:898`) — currently pins the batch stopping
  mid-replay. No batch exists now; **flip** to the equivalent one-drain invariant: a held nav-msg whose nested
  SW-wait consumes a re-dispatched `Shutdown` makes `pump_turn` `Break` at step 4's `shutdown_requested` check
  before the next buffered message is intaken (the torn-down pipeline is never dispatched against).

**Preserve unchanged:** `pump_turn_applies_enqueued_traversal_on_a_later_turn` (`:431`, the I1 task-boundary
test — must still pass), and R8/R9 shutdown/drain checks not tied to the replay symbol.

**Touch-time 1000-line split (impl-time obligation, NOT a note).** `content_history_phase_sep_tests.rs` is
**978 LoC** today; the three NEW R13 conformance tests add ~+150–240 LoC (net of the `:783` deletion),
crossing ~1100–1200. Per CLAUDE.md "1000-line debt = touch-time split" (which applies to test files),
**measure the file post-add at implementation** and, if it is ≥1000 LoC, carve — **as part of THIS PR**
(touch-time, single/standalone commit) — a cohesion seam into a sibling test module: preferred seam = the
three R13 conformance scenarios (the `:416` / `:73` / `:49` tests) into
`content_history_pump_turn_tests.rs`; if the pre-existing interim-guard tests form the cleaner seam, split
those instead. This is a required impl-time step, not a deferrable TODO.

---

## §7 Files + grep-verified symbols

Existing symbols grep-verified against `crates/shell/elidex-shell/src`; NEW annotated.

- `crates/shell/elidex-shell/src/content/event_loop.rs` — `pump_turn` (restructure), `replay_deferred_reentrant_messages` (RETIRE), `run_deferred_traversals` (step 3, unchanged body), `drain_synchronous_updates` (step-3 top drain — **NEW seam**, see below), `drain_synchronous_phase` (step-6 bottom, unchanged full body), `handle_message` (single intake, reused), `recv_timeout` + timeout compute (`:90-108`, +`traversal_queue.is_empty()` input).
- `crates/shell/elidex-shell/src/content/drain_host.rs` — `dispatch_or_buffer_reentrant` **function body KEPT
  unchanged**, but its **docstring is EDITED**: `:46-48` ("the event loop replays / it at the top of a later
  `pump_turn` … see `event_loop::replay_deferred_reentrant_messages`") and `:66` ("R5's
  `replay_deferred_reentrant_messages -> ControlFlow` exit-propagation stays as the defensive path …") both
  reference the RETIRED symbol / the retired "replays at the top of `pump_turn`" mechanism → dead/stale after
  retirement. Rewrite those spans to the one-intake buffer-drain (the buffer is drained one-per-turn via the
  single held-message intake, not batch-replayed; the `Shutdown`-never-buffered invariant is unchanged and no
  longer relies on a `ControlFlow` replay exit).
- `crates/shell/elidex-shell/src/content/mod.rs` — `deferred_reentrant_messages` field (KEPT; drained via the single intake), `shutdown_requested` (KEPT).
- `crates/shell/elidex-navigation/src/traversal_queue.rs` — `TraversalQueue::is_empty` (`:226`, EXISTING; read
  by the timeout). **NEW public seam `DrainCoordinator::drain_synchronous_updates`** (draft-2 correction, axis
  d) = the shared 1a/1b body (`run_synchronous_updates_body`, factored OUT of `run_synchronous_phase_body`
  which now composes `updates_body` + Phase 1c) + `ship_if_needed`. The full `drain_synchronous_phase` is
  unchanged in behavior. No copy-paste (the difference is only whether 1c runs); the FIFO-partition logic
  stays engine-agnostic in `elidex-navigation`.
- `crates/shell/elidex-shell/src/content_history_pump_turn_tests.rs` — three NEW R13 conformance tests +
  the NEW `popstate_cross_document_navigation_deferred_below_held_input` regression (axis d), which uses
  `drain_synchronous_updates` / `drain_synchronous_phase` + `runtime.take_pending_navigation` (EXISTING
  `HostDriver` method) to pin the deferral structurally.
- `crates/shell/elidex-shell/src/content_history_phase_sep_tests.rs` — interim-guard tests (flip/delete per §6).

**Crate surface.** No NEW `HostDriver` peek (draft-1's `has_pending_*` is eliminated by the peek-less rework),
so **no `elidex-script-session` / `elidex-js` trait surface**. There IS a **NEW `elidex-navigation` surface**
(the drain-split correction): the public `DrainCoordinator::drain_synchronous_updates` seam (axis d). It stays
a clean engine-agnostic coordinator method — the shell just calls the right one of the two drains at each site
(top = updates-only, bottom = full). Blast radius = `content/event_loop.rs` + `elidex-navigation`'s coordinator
seam + the tests.

---

## §8 Landing-time doc-sync (sibling memos describe the RETIRED replay as live)

**✅ RECONCILED at Codex PR#469 R17 (2026-07-18)** — all three spans below rewritten to the one-per-turn held-message intake wording: content-phase-separation.md `:72-73` + `:314` (retired reentrant-replay-at-the-top → buffer-first one-per-turn intake), task-queue-model.md I3 (`:313` "top-of-loop `run_deferred_traversals`" → "step-3 per-turn"). Kept below as the obligation record.

This slice retires the "buffered … and replayed at the top of a later `pump_turn`" mechanism that two sibling
memos currently describe as live interim behavior. **On landing, update these spans** (grep-verified line
refs; re-grep at landing since siblings may shift) to the new one-per-turn intake wording — e.g. "the buffer
is drained one-per-turn via the single held-message intake (step 1), not batch-replayed at the top of
`pump_turn`":

- `docs/plans/2026-07-session-history-slice-A-content-phase-separation.md:72-73` — "the message is buffered
  onto `ContentState::deferred_reentrant_messages` and replayed / at the top of a later `pump_turn`".
- `docs/plans/2026-07-session-history-slice-A-content-phase-separation.md:314` — "replayed on a later
  `pump_turn`, never applied under the held peek".
- `docs/plans/2026-07-session-history-task-queue-model.md` §4.5 I3 (`:313`) — the liveness citation "**Liveness
  is preserved by content-mode's async pump running Phase-2 every turn** (`event_loop.rs` top-of-loop
  `run_deferred_traversals`)": Phase-2 still runs every turn (step 3), but the buffer-drain liveness is now the
  one-per-turn held-message intake, not a top-of-loop replay pass — reword the interim-drain prose accordingly
  (the every-turn Phase-2 apply claim itself stays true).
