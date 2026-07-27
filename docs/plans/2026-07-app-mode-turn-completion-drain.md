# App-mode turn completion — run the input-handler turn to quiescence

**Slot**: `#11-app-mode-turn-completion-drain` (carved by Slice B's 5-agent `/elidex-review` design gate,
2026-07-26; enriched by a max-effort `/code-review` the same day).
**Umbrella**: `#11-session-history-task-queue-model` — this is a **drive-schedule** change, the app-mode
counterpart of Slice A's Codex-R9 fix, and it does **not** touch the shared coordinator's semantics.
**Status**: plan-memo draft, pre-`/elidex-plan-review`. **Edge-dense ⇒ plan-review is MANDATORY** (CLAUDE.md),
own PR.

---

## §0 Decision + scope

**Decision.** Replace app-mode's single `DrainCoordinator::drain_same_turn` call with a **bounded
loop-until-quiescent turn completion**, so that a §7.4.4 intent staged *during* Phase 2 — canonically a
`pushState` from the `popstate` handler that a same-document traversal fires synchronously — is applied on the
turn that fired it, instead of waiting for an unbounded number of input events.

**IN**: the app-mode drive site (`App::process_pending_navigation`, `app/drain_host.rs`); whatever seam the
loop needs to decide "is this turn finished?"; the two premise-5 `debug_assert`s and their contracts; the
`app_popstate_staged_action_defers_to_the_next_drain_not_the_current_queue` pin (flips).

**OUT (fenced, each with its owner)**:
- The **multi-traversal straddle** — a popstate-staged intent from traversal-1 still settles after traversal-2
  when both are in one Phase-2 snapshot. Owned by `#11-sync-navigation-steps-queue-tagging` (R16 facet);
  `app_history_phase_sep_tests::app_multi_traversal_snapshot_lands_popstate_staged_update_on_the_wrong_entry`
  stays as-is. **This plan must not silently narrow that pin.**
- **Content-mode's schedule.** Content already settles this via `drain_synchronous_updates` after
  `run_deferred_traversals` (`content/event_loop.rs`) and its task boundary is deliberate; nothing here changes
  it. If the loop's quiescence machinery lands in `elidex-navigation`, content must be able to **not** use it.
- The §7.4.2.2-step-19 suppression divergence (`#11-nav-supersede-window-vs-ongoing-navigation`) and the
  applied/shipped conflation (`#11-nav-applied-shipped-decouple`) — both pre-existing, both untouched.

**Non-goal**: making app-mode a mirror of content-mode. App-mode has **no async pump**, and that is a property
of the inline shell, not a defect to erase. The goal is *quiescence before returning to the OS event loop*, which
is what content gets from its pump and app-mode currently gets from nothing.

---

## §1 The decisive fact — what is actually broken (verified against `258b799e`, not inherited from the slot)

`App::process_pending_navigation` (`app/drain_host.rs`) is, in order:

1. `debug_assert!(!drain_in_progress)` — premise-5 **entry** guard (re-drive from inside a seam body);
2. `drain_in_progress = true`;
3. **one** `DrainCoordinator::drain_same_turn(self)` — Phase 1 (window-opens → §7.4.4 updates → §7.4.2 nav) →
   Phase 2 (§7.4.6.1 applies) → ship once;
4. the **reinstatement tail** — `deferred_navigation.take()` → `self.navigate(...)`, narrowing the enqueue-time
   suppression superset back down within the turn;
5. `drain_in_progress = false`;
6. `debug_assert!(traversal_queue().is_empty())` — premise-5 **exit** guard (residual step).

Nothing between step 3 and step 6 re-drains the VM `pending_history` FIFO. So an intent staged **during** step 3's
Phase 2 is left on the VM channel, and the next drive is **not** next-input-bounded: `app/events.rs::handle_click`
returns early at four sites (`:22`, `:25`, `:30`, `:35`) and `handle_keyboard` at two (`:168`, `:175`) — all
**before** the drive (`:101`, `:188`). A user clicking blank space never drains it. Residual latency is
**unbounded**, not next-input-bounded.

The existing pin measures the *best* case only: it drives `process_pending_navigation` directly, so it observes
one-turn latency, not a guarantee.

**Spec frame.** WHATWG HTML §8.1.7.3 *Processing model* runs a task to completion — including everything its
synchronously-fired handlers stage — before the event loop moves on. Content-mode approximates that with its
per-turn pump; app-mode currently returns to winit with work still staged. This plan is about restoring
"the turn is finished when the handler's own effects are finished".

---

## §2 Coupled invariants (the edge matrix — plan-review checks each axis independently)

**Six axes intersect here** (≥3 ⇒ plan-review mandatory). (a) is new; (b)–(f) are the shared coordinator's
existing invariants that a repeated drive can break.

- **(a) Turn completion (NEW).** The turn ends only when the handler's staged work is settled. *Failure mode:*
  an unbounded loop on the single-writer renderer thread — an adversarial or merely enthusiastic `popstate`
  handler that re-stages every iteration.
- **(b) I1 phase ordering.** Phase 1 completes before Phase 2, per iteration. *Failure mode:* a loop that
  re-enters Phase 2 without a fresh Phase 1, or interleaves them.
- **(c) I2 issue-order partition.** The single VM FIFO is the ordering SoT; from the first in-range traversal
  onward every step defers in issue order. *Failure mode:* iteration N+1 applying something that was issued
  *before* something iteration N deferred.
- **(d) I3 Phase-2 bounded snapshot + §4.4 premise 5.** `drain_traversal_queue` processes `pending_len()` steps
  captured at drain-start; app-mode's whole-queue completeness rests on "no app-mode body drives the Phase-1
  partition". *Failure mode:* the loop being mistaken for — or enabling — a body-driven re-entry, which
  interleaves partitions and silently voids I2 and the Resolution-D latch.
- **(e) Resolution-E classification freshness.** A traversal must be peek-classified in the same iteration whose
  Phase 2 applies it, so an in-range decision is never frozen across a window in which a **non-drain** cursor
  mover can run (chrome Back/Forward and Alt+←/→ call `traverse_to`; an `<a href>` default calls `navigate`).
  *Failure mode:* a resident `Traversal` step acting as a full barrier — seeding `seen_traversal` at Phase-1
  entry and latching `suppress_default` at exit — for a traversal that has since gone out of range. **This axis
  is what kills the obvious fix; see §4.1.**
- **(f) Resolution-D `traversal_applied` (per-drain latch).** A `SyncUpdate` deferred behind a cursor-moving
  traversal is cancelled *within that drain*. *Failure mode:* looping resets the latch, so a step that should
  have been cancelled is applied by a later iteration — or, inversely, a legitimately fresh intent is cancelled.

**Pairwise intersections** (cell → where this memo pins it):

| × | (b) I1 | (c) I2 | (d) I3/premise-5 | (e) Res-E freshness | (f) Res-D latch |
|---|---|---|---|---|---|
| **(a) turn completion** | each iteration is a whole `drain_same_turn`, never a partial phase (§4.2) | iteration N+1 handles only intents issued *during* N, so FIFO order across iterations is issue order (§4.3) | the loop is **site-driven**, not body-driven — the distinction premise 5 actually guards (§4.6) | every traversal is classified and applied in the same iteration (§4.2) — the property the trailing drain loses | a `[Traversal, SyncUpdate]` pair cannot split across iterations (§4.5) |
| **(b) I1** | — | partition runs per iteration, on a FIFO that is empty of prior-iteration steps | Phase 2 bounded per iteration | classify → apply within one iteration | latch scoped to one iteration's Phase 2 |
| **(c) I2** | — | — | the queue is empty at each iteration's end (exit assert holds) | no resident step to freeze | cancel decisions are made on one iteration's steps |
| **(d) I3/premise-5** | — | — | — | premise 5 keeps partitions non-interleaved | latch integrity depends on non-interleaving |
| **(e) Res-E** | — | — | — | — | a cursor-moving apply is what arms the latch |

---

## §3 Spec coverage map

| Spec | Section (webref-verified) | This plan |
|---|---|---|
| HTML | §8.1.7.3 *Processing model* (`#event-loop-processing-model`) | **Frame** — a task runs to completion including synchronously-staged follow-on work |
| HTML | §7.4.4 *Non-fragment synchronous "navigations"* | The staged intent that currently strands |
| HTML | §7.4.3 *Reloading and traversing* | Traversals a `popstate` handler may stage during Phase 2 |
| HTML | §7.4.6.1 *Updating the traversable* | The Phase-2 apply that fires `popstate` |
| HTML | §7.4.6.1 step 14 note | **Fenced OUT** — sync navs jumping the queue between traversals is `#11-sync-navigation-steps-queue-tagging` |
| HTML | §7.3.1.1 *Traversable navigables* | The queue + nested-apply guard the loop must leave empty |

*(All §↔title pairs re-verified with `.claude/tools/webref` at authoring time; §8.1.7.3 = "Processing model",
§8.1.7.1 = "Definitions", §8.1.7.2 = "Queuing tasks".)*

---

## §4 The design

### 4.1 The obvious fix is WRONG, not merely insufficient (falsification first)

A trailing `DrainCoordinator::drain_synchronous_updates` after `drain_same_turn` — the literal transcription of
content-mode's R9 fix — **must not be adopted**. It settles a popstate-staged `pushState`, but a popstate-staged
`back()` is peek-classified by that trailing Phase 1b and left **resident on the `TraversalQueue` across the turn
boundary**, because the trailing drain has no Phase 2 behind it.

That resident step is *not* stranded — turn N+1 seeds `seen_traversal` from `has_pending_traversal()` and drains
it — so the damage is not latency. The damage is that it **freezes the in-range classification a turn early**
(axis (e)): between turns, the non-drain cursor movers run, so by turn N+1 the step may be a no-op while still
acting as a **full barrier** — deferring every fresh `pushState` behind it and latching `suppress_default` true,
killing an unrelated `<a href>` default. That voids the queue's own contract that Resolution E "leaves no
`Traversal` step for a no-op, so it does not over-suppress".

It also **contradicts the exit `debug_assert` by construction** (the queue would be deliberately non-empty at
drain exit), which is the tell that it is the wrong shape rather than an incomplete one.

### 4.2 The ideal — iterate whole drains, not partial phases

Repeat the **entire** `drain_same_turn` (Phase 1 → Phase 2 → ship) until the turn is quiescent:

```
loop {
    let outcome_n = DrainCoordinator::drain_same_turn(self);   // whole cycle, never a partial phase
    accumulate(outcome_n);
    if turn_is_quiescent() { break }
}
```

Every property the trailing drain loses is preserved *because the unit of iteration is a whole cycle*: each
traversal is classified and applied in the same iteration (axis (e)); each iteration's Phase 2 empties the queue,
so the exit assert stays true (axis (d)); and each iteration's partition sees a FIFO containing only intents
issued during the previous iteration, which is exactly issue order (axis (c)).

**Ship-once is the open wrinkle**: `drain_same_turn` ships at most one frame per call, so N iterations could ship
N frames. Options — accumulate and suppress all but the last; or let the coordinator's existing `shipped`
bookkeeping ride and accept ≤N ships for a turn that genuinely did N rounds of work. §7 Q3.

### 4.3 Termination — the bound is a liveness backstop, not a correctness boundary

A handler that unconditionally re-stages (`onpopstate = () => history.pushState(…)` plus a traversal) makes the
fixpoint unreachable. Since this runs on the single-writer renderer thread, an unbounded loop is a hang, which is
strictly worse than today's late-settle.

**Proposal**: bound the loop, and on hitting the bound **degrade to exactly today's behavior** — leave the
residue staged for the next drive. The bound is then a backstop, and the invariant is *"never worse than today,
quiescent in every non-adversarial case"*. This deliberately mirrors the Phase-2 bounded snapshot's own design
(bounded, terminates by construction, residue deferred).

Open: the bound's *value* and its *unit* (iterations? total steps applied?), and whether hitting it should be
observable (a `debug_assert`? a counter? silence?). §7 Q2.

### 4.4 The quiescence predicate — the central design decision

The drive site cannot ask "is anything staged?" without consuming it: `take_pending_history` drains. So the loop
needs a signal. Candidates, in increasing order of blast radius:

- **(A) Derive from `DrainOutcome`.** Loop while the last iteration "did something". Problem: `own_context_action`
  deliberately **excludes** window-opens (they act on other browsing contexts), so a window-open-only iteration
  reads as no-progress; and "did something" ≠ "something remains".
- **(B) A new `DrainHost` observer seam** — e.g. `fn has_pending_work(&self) -> bool`, implemented by each shell
  over its own staging channels. Honest and precise; adds a trait method **both** shells must implement, and
  content's impl would exist only to satisfy the trait.
- **(C) Put the loop in the coordinator** (`drain_to_quiescence`). One-issue-one-way if quiescence is a general
  concept — but content-mode must **not** use it (its task boundary is the point), so this risks minting a second
  drive shape nobody outside app-mode wants.

**Author's lean: (B)**, because the predicate is genuinely shell state (which channels exist and what "empty"
means is shell-specific), and because it keeps the *policy* (loop, bound, degrade) in the shell that needs it
while the coordinator stays a stateless phase driver. But this is exactly the decision plan-review should own —
see §7 Q1. **(A) should be rejected explicitly rather than by omission**, since it is the cheapest and the most
likely to be reached for later.

### 4.5 Resolution-D across iterations

`traversal_applied` is a **per-drain local**, so each iteration resets it. This is correct *provided* a
`[Traversal, SyncUpdate]` pair can never split across iterations — and it cannot: Phase 1b enqueues both in the
same iteration, and that iteration's Phase-2 `pending_len()` snapshot counts **all** pending steps, so it captures
the pair whole and cancels the straddle. A `SyncUpdate` staged *afterwards*, by the popstate handler, belongs to
the next iteration's Phase 1 and **should** be applied in-task — that is the entire point of the fix, and it is
the same outcome content-mode's `drain_synchronous_updates` produces.

To be re-derived under plan-review, not assumed: whether any interleaving exists in which a cursor-moving
traversal in iteration N should have cancelled an intent that iteration N+1 applies.

### 4.6 Premise 5, restated for a site-driven loop

The entry `debug_assert!(!drain_in_progress)` guards **body-driven re-entry**: a `DrainHost` seam body, an apply
body, or the reinstatement tail calling back into the drive, which interleaves two partitions and silently voids
I2 and the Resolution-D latch. A **site-driven** loop is categorically different: iterations are strictly
sequential, each starting only after the previous one's bodies have all returned.

The loop therefore lives **inside** the `drain_in_progress` window and the assert is unchanged in force — but its
prose currently says "no app-mode apply body drives the Phase-1 partition", which a reader of the new code will
read as contradicted. The contract must be restated to name the distinction (sequential site-driven iteration =
legal; nested body-driven re-entry = the bug), or the next maintainer will "fix" the assert.

The exit assert is **strengthened**, not weakened: it now asserts quiescence of a completed turn rather than of a
single pass — except on the §4.3 bound-degradation path, where a residue is deliberate. Reconciling those two is
§7 Q4.

---

## §5 Decomposition

Single PR under the approved umbrella (edge-dense base case: a narrowly-scoped per-PR slice that has passed
plan-review is a terminal unit). No prereq split is owed — the drive site's file is well under the line
(`app/drain_host.rs` is comment-dominated but bounded), and the §4.4 decision may add one trait method.

If plan-review picks **(C)**, that becomes an `elidex-navigation` behavior change touching both shells and
should be re-sliced as its own prereq PR.

---

## §6 What this closes / does not close

**Closes**: `#11-app-mode-turn-completion-drain`.
**Flips**: `app_history_phase_sep_tests::app_popstate_staged_action_defers_to_the_next_drain_not_the_current_queue`
— from "defers to the next drain" to the content shape ("settled on the turn that fired it").
**Does NOT close, and must not appear to**: `#11-sync-navigation-steps-queue-tagging` (the multi-traversal
straddle), `#11-nav-supersede-window-vs-ongoing-navigation`, `#11-nav-applied-shipped-decouple`.
**Interaction to check**: the destructive pin
`app_popstate_staged_push_destroys_forward_entries_after_an_interleaved_chrome_traversal` pins a *between-turns*
chrome traversal, so turn completion narrows the window in which it can occur but does not eliminate it (the user
can still traverse between two input turns). Whether that pin's docstring needs re-scoping is §7 Q5.

---

## §7 Open questions for `/elidex-plan-review` (decision-level)

- **Q1 — quiescence predicate**: (A) `DrainOutcome`-derived / (B) new `DrainHost::has_pending_work` / (C)
  coordinator-side `drain_to_quiescence`. Author leans (B); (A) should be explicitly refuted.
- **Q2 — the bound**: value, unit (iterations vs applied steps), and observability on hit.
- **Q3 — ship-once across iterations**: one frame per turn, or one per iteration that did work?
- **Q4 — exit assert vs deliberate bound-degradation residue**: how do both stay true?
- **Q5 — pin re-scoping**: does the destructive pin's docstring change, given the window narrows but persists?
- **Q6 — is app-mode's `handle_keyboard`/`handle_click` early-return set itself part of the defect?** The slot
  frames unbounded latency as *caused* by the early returns. Turn completion fixes the *staged-during-Phase-2*
  case, but an intent staged by a handler that returns early was never drained at all. Is that a second,
  separate gap this plan should name (and fence), rather than leave implicitly fixed-looking?

---

## §8 Test strategy

- **Flip** the latency pin to the content shape; keep its docstring's fence pointer accurate.
- **New**: a popstate handler staging a `pushState` settles within the same `process_pending_navigation` call.
- **New**: a popstate handler staging a `back()` is applied within the same turn — the case the trailing-drain
  alternative gets wrong; assert the queue is empty at drive exit and that no `suppress_default` latch survives.
- **New (termination)**: a handler that re-stages unconditionally terminates at the bound and leaves the residue
  staged, i.e. degrades to today's behavior rather than hanging.
- **Unchanged**: the multi-traversal straddle pin must still fail the same way (guard against silently narrowing
  a fenced divergence).
- **Parity**: content-mode tests untouched; `elidex-navigation` isolation tests untouched unless Q1 picks (C).

---

## §9 Defer ledger

Own-deferral budget: expected **0–1**. Anything discovered mid-implementation that is not the turn-completion
loop belongs to an existing slot (see §6) or a new one with the 3-element audit (`Why deferred` /
`Re-evaluation trigger` / `Re-evaluation date`).
