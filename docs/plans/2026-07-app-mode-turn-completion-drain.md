# App-mode turn completion — run the input-handler turn to quiescence

**Slot**: `#11-app-mode-turn-completion-drain` (carved by Slice B's 5-agent `/elidex-review` design gate,
2026-07-26; enriched by a max-effort `/code-review` the same day; severity escalated by the R2 enrichment —
see §1).
**Umbrella**: `#11-session-history-task-queue-model` — this is a **drive-schedule** change, the app-mode
counterpart of Slice A's Codex-R9 fix, and it does **not** touch the shared coordinator's phase semantics.
**Status**: rewrite v2, pre-`/elidex-plan-review` (full re-review 予定). v1 was REJECTED (2 CRIT / 26 IMP /
13 MIN); the user ratified scope option (i) — turn-granularity completion here, traversal-granularity settle
stays fenced — with the fence re-grounded as *same mechanism, different granularity* (§0, §1).
**Edge-dense ⇒ plan-review is MANDATORY** (CLAUDE.md), own PR. All line numbers verified at
`06e632ae` (base `258b799e`); all §↔title pairs and algorithm steps webref-verified 2026-07-29
(commands recorded inline).

---

## §0 Decision + scope

**Decision.** Replace app-mode's single `DrainCoordinator::drain_same_turn` call with a **bounded
loop-until-quiescent turn completion** at the drive site (`App::process_pending_navigation`,
`app/drain_host.rs:194`), so that a §7.4.4 intent staged *during* Phase 2 — canonically a `pushState` from
the `popstate` handler that a same-document traversal fires synchronously — is applied on the turn that
fired it, instead of sitting on the VM `pending_history` FIFO until an unbounded later drive applies it
against a cursor that has moved (§1 severity).

**IN**: the app-mode drive site and its iteration unit (`drain_same_turn` + the reinstatement tail, §4.2);
the quiescence predicate seam (§4.4); the loop's per-turn outcome accumulation + pipeline-swap boundary
(§4.5); the termination bound with a guaranteed follow-up drive (§4.3); the premise-5 `debug_assert` pair's
contracts (§4.7); the flip set (§6): **both** slot pins plus the straddle pin's intermediate assertions; the
sibling-passage sweep (§5.2); the test-file placement decision (§5.1).

**OUT (fenced, each with its owner)**:
- **The multi-traversal straddle — same mechanism, different granularity, NOT "unrelated".** The spec
  settles staged synchronous navigations via §7.4.6.1 *apply the history step* step 14.1.1 — a bounded
  drain loop **between traversal change-jobs, inside the apply** (bracketed by
  `running nested apply history step`, gated per-navigable by
  `navigablesThatMustWaitBeforeHandlingSyncNavigation`, steps 13 + 14.8). This plan implements the same
  bounded-drain mechanism at **turn granularity** (after the whole two-phase drain, before returning to the
  OS loop); the **traversal-granularity** application — consuming a popstate-staged intent *between* two
  queued traversal applies within one Phase-2 snapshot — is owned by
  `#11-sync-navigation-steps-queue-tagging` (its R16 facet). Concretely: with this plan landed, a
  `back(); forward()` turn still settles the popstate-staged intent only *after* `forward()` has moved the
  cursor, landing it on the wrong entry —
  `app_history_phase_sep_tests::app_multi_traversal_snapshot_lands_popstate_staged_update_on_the_wrong_entry`
  (`app_history_phase_sep_tests.rs:490`) keeps pinning exactly that (its final wrong-entry assertions are
  the fence; its *timing* assertions flip — §6). **This plan must not silently narrow that pin.**
- **Content-mode's schedule.** Content already settles this per pump turn via `drain_synchronous_updates`
  after `run_deferred_traversals` (`content/event_loop.rs:205-206`) and its task boundary is deliberate;
  nothing here changes it. If any loop machinery lands in `elidex-navigation`, content must be able to
  **not** use it (§4.4 option (C)'s risk).
- The §7.4.2.2-step-19 suppression divergence (`#11-nav-supersede-window-vs-ongoing-navigation`) and the
  applied/shipped conflation (`#11-nav-applied-shipped-decouple`) — both pre-existing, both untouched.
- **Non-drain cursor-mover routing.** Whether chrome Back/Forward, Alt+←/→, the address bar, and Reload
  should route through the coordinator (or drain staged residue before moving the cursor) is the umbrella's
  Slice-4 DIRECT-nav serialization question (`#11-session-history-task-queue-model`), not this plan's — see
  §7 Q3 for the bounded residual this leaves.

**Non-goal**: making app-mode a mirror of content-mode. App-mode has **no async pump**, and that is a
property of the inline shell, not a defect to erase. The goal is *quiescence before returning to the OS
event loop* — the settle content gets from its per-turn pump, expressed as a drive-site loop because
app-mode has no later turn to defer to (except the bound-degradation follow-up, §4.3).

---

## §1 The decisive fact — what is actually broken (verified against `06e632ae`, not inherited from the slot)

`App::process_pending_navigation` (`app/drain_host.rs:194`) is, in order:

1. `debug_assert!(!drain_in_progress)` (`:221`) — premise-5 **entry** guard (re-drive from inside a seam
   body);
2. `drain_in_progress = true` (`:229`);
3. **one** `DrainCoordinator::drain_same_turn(self)` (`:230`) — Phase 1 (window-opens → §7.4.4 updates →
   §7.4.2 nav) → Phase 2 (§7.4.6.1 applies) → ship once;
4. the **reinstatement tail** (`:277-286`) — `deferred_navigation.take()` → `self.navigate(...)`, narrowing
   the enqueue-time suppression superset back down within the turn;
5. `drain_in_progress = false` (`:290`);
6. `debug_assert!(traversal_queue().is_empty())` (`:301`) — premise-5 **exit** guard (residual step).

Nothing between step 3 and step 6 re-drains the VM `pending_history` FIFO. An intent staged **during** step
3's Phase 2 is left on the VM channel, and the next drive is **not** next-input-bounded:
`app/events.rs::handle_click` returns early at four sites (`:22`, `:25`, `:30`, `:35`) and
`handle_keyboard` at two (`:168`, `:175`) — all **before** the drive (`:101`, `:188`). A user clicking
blank space never drains it.

**The harm is wrong-entry mutation, not latency** (the slot's R2 enrichment; the destructive pin below).
While the staged intent sits on the FIFO, the **non-drain cursor movers** run without draining it first
(full command-derived enumeration in §4.1): chrome toolbar Back/Forward (`app/navigation.rs:505` →
`traverse_to`), Alt+←/→ (`app/inline.rs:258` → `traverse_to`), the `<a href>` default
(`app/events.rs:160` → `navigate`), the address bar (`app/navigation.rs:484` → `navigate`, including its
same-document arm), and chrome Reload (`app/navigation.rs:519` → `navigate`, restamping the document
identity the classification reads). The eventual drive then applies the staged update against the
**post-traversal** cursor: the replace arm overwrites the wrong current entry, and the push arm reaches
`push_entry`'s `entries.truncate(current_index + 1)`, **destroying live forward entries** the user just
traversed away from. Pinned by
`app_popstate_staged_push_destroys_forward_entries_after_an_interleaved_chrome_traversal`
(`app_history_phase_sep_tests.rs:691`); the latency facet is pinned by
`app_popstate_staged_action_defers_to_the_next_drain_not_the_current_queue` (`:604`). Their shared
docstring is explicit: *"Both flip when the fix lands."* (`:577`, verified by
`grep -rn "Both flip when the fix lands" crates/`).

**Spec frame — what the spec does and does not promise.** WHATWG HTML §8.1.7.3 *Processing model*
(webref: `heading html 8.1.7.3` → `#event-loop-processing-model`; prose via
`body html event-loop-processing-model`) runs one task (step 2.6 "Perform oldestTask's steps") and then
only a **microtask checkpoint** (step 2.8) before returning to the queue — it does **not** promise that
work a handler *staged elsewhere* completes within the turn. And §7.4.4 *URL and history update steps*
step 13 (`body html navigate-non-frag-sync`) stages exactly such work: it **appends** the synchronous
navigation steps (the `finalize a same-document navigation` that performs the real entries-list mutation)
to the **traversable** — i.e. onto the §7.3.1.1 *session history traversal queue*, a parallel queue — not
into the current task. So §8.1.7.3 gives this plan no quiescence warrant.

The spec's actual settle point for those staged steps is **§7.4.6.1 *apply the history step* step 14.1.1**
(`body html updating-the-traversable`): while change jobs remain and
`running nested apply history step` is false, repeatedly take a staged synchronous-navigation-steps item
whose target navigable is not in `navigablesThatMustWaitBeforeHandlingSyncNavigation`, bracket it in the
nested-apply boolean, and run it — a **bounded drain loop of staged sync-nav work, between traversal
change-jobs**. The spec's note on it: synchronous navigations *"jump the queue at this point, so they can
be added to the correct place in traversable's session history entries before this traversal potentially
unloads their document."*

**This plan's mapping**: the inline shell has no pump and no parallel queue, so the settle the spec
expresses as "traversal queue + step-14.1.1 drain inside the apply" is realized as a **quiescence loop at
the drive site, at turn granularity** — settle everything the turn's handlers staged before returning to
winit. The traversal-granularity component (14.1.1's *between change-jobs* placement, which is what makes
the staged intent land on the entry whose handler issued it even when more traversals follow in the same
snapshot) is precisely what `#11-sync-navigation-steps-queue-tagging` owns. Same mechanism — a bounded
drain of staged sync-nav — at two granularities, split across two slots.

---

## §2 Coupled invariants (the edge matrix — plan-review checks each axis independently)

**Seven axes intersect here** (≥3 ⇒ plan-review mandatory). (a) and (g) are new; (b)–(f) are the shared
coordinator's existing invariants that a repeated drive can break.

- **(a) Turn completion (NEW).** The turn ends only when the handlers' staged work is settled. *Failure
  mode:* an unbounded loop on the single-writer renderer thread — a `popstate` handler that re-stages every
  iteration.
- **(b) I1 phase ordering.** Phase 1 completes before Phase 2, per iteration. *Failure mode:* a loop that
  re-enters Phase 2 without a fresh Phase 1, or interleaves them.
- **(c) I2 issue-order partition.** The single VM FIFO is the ordering SoT; from the first in-range
  traversal onward every step defers in issue order. *Failure mode:* iteration N+1 applying something
  issued *before* something iteration N deferred.
- **(d) I3 Phase-2 bounded snapshot + §4.4 premise 5.** `drain_traversal_queue` processes `pending_len()`
  steps captured at drain-start (`traversal_queue/coordinator.rs:430`); app-mode's whole-queue completeness
  rests on "no app-mode body drives the Phase-1 partition". *Failure mode:* the loop being mistaken for —
  or enabling — a body-driven re-entry, which interleaves partitions and silently voids I2 and the
  Resolution-D latch.
- **(e) Resolution-E classification freshness.** A traversal must be peek-classified in the same iteration
  whose Phase 2 applies it, so an in-range decision is never frozen across a window in which a **non-drain**
  cursor mover can run (the five movers enumerated in §4.1). *Failure mode:* a resident `Traversal` step
  acting as a full barrier — seeding `seen_traversal` at Phase-1 entry and latching `suppress_default` at
  exit — for a traversal that has since gone out of range. **This axis is what kills the obvious fix; see
  §4.1.**
- **(f) Resolution-D `traversal_applied` (per-drain latch).** A `SyncUpdate` deferred behind a
  cursor-moving traversal is cancelled *within that drain* (`coordinator.rs:493`). *Failure mode:* looping
  resets the latch, so a step that should have been cancelled is applied by a later iteration — or a
  legitimately fresh intent is cancelled.
- **(g) Outcome accumulation across iterations (NEW).** The turn returns ONE `DrainOutcome` to
  `handle_click` (`events.rs:101`) and holds ONE `deferred_navigation` slot and ONE pipeline. *Failure
  modes:* iteration 2's all-false outcome **clearing** iteration 1's `suppress_default` (breaking the
  single-home contract and firing an `<a href>` default that must stay suppressed); a held navigation
  leaking across iterations; the loop continuing across a **pipeline swap** and running a new document's
  staged intents inside the old document's input turn (§4.5).

**Pairwise intersections** (cell → where this memo pins it):

| × | (b) I1 | (c) I2 | (d) I3/premise-5 | (e) Res-E freshness | (f) Res-D latch | (g) accumulation |
|---|---|---|---|---|---|---|
| **(a) turn completion** | each iteration is a whole `drain_same_turn` + tail, never a partial phase (§4.2) | iteration N+1 handles only intents issued *during* N, so FIFO order across iterations is issue order (§4.2) | the loop is **site-driven**, not body-driven — the distinction premise 5 guards (§4.7) | every traversal is classified and applied in the same iteration (§4.2) — the property the trailing drain loses | a `[Traversal, SyncUpdate]` pair cannot split across iterations (§4.6) | quiescent ⇒ nothing left to mis-accumulate; bound-hit residue is flagged + re-driven (§4.3) |
| **(b) I1** | — | partition runs per iteration, on a FIFO empty of prior-iteration steps | Phase 2 bounded per iteration | classify → apply within one iteration | latch scoped to one iteration's Phase 2 | outcome merged only at iteration boundaries |
| **(c) I2** | — | — | the queue is empty at each iteration's end (exit assert holds) | no resident step to freeze | cancel decisions are made on one iteration's steps | the reinstatement tail runs *inside* the iteration, before the next Phase 1 (§4.2) |
| **(d) I3/premise-5** | — | — | — | premise 5 keeps partitions non-interleaved | latch integrity depends on non-interleaving | `drain_in_progress` brackets the whole loop (§4.7) |
| **(e) Res-E** | — | — | — | — | a cursor-moving apply is what arms the latch | a pipeline swap ends the loop before stale classification is possible (§4.5) |
| **(f) Res-D** | — | — | — | — | — | per-iteration latch + OR-latched outcome never re-order a cancel |

---

## §3 Spec coverage map

**Derivation (honest split of tool output vs manual expansion).** The skeleton + breadth verdict came from
`.claude/tools/webref coverage-map html 8.1.7.3 html 7.4.4 html 7.4.3 html 7.4.6.1 html 7.4.6.2 html
7.3.1.1 html 7.4.2.2`, whose output is **section-granular**: `Breadth (requested): spec=1 (html), step=7
entries / Split decision: ok → single PR scope` (7 = one row per requested section). The per-step rows
below are a **manual expansion** of those 7 sections into algorithm-step branches; M counts the manual
rows. Step numbers were verified against prose fetched via `body html <anchor>` (anchors as reported by
the tool: `#event-loop-processing-model`, `#navigate-non-frag-sync`, `#reloading-and-traversing`,
`#updating-the-traversable`, `#updating-the-document`, `#traversable-navigables`, `#beginning-navigation`).

| Spec section | Algorithm + step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §8.1.7.3 Processing model | *event loop processing model* steps 2.6 (run task) / 2.8 (microtask checkpoint) | what the spec does **NOT** promise: staged parallel-queue work is outside the turn | `App::process_pending_navigation` — the loop's frame of reference (§1) | ✓ | yes (every input turn) |
| WHATWG HTML §7.4.4 Non-fragment synchronous "navigations" | *URL and history update steps* step 13 (append sync-nav steps to the traversable) | (i) staged BEFORE Phase 2 — applied in-task today | `DrainHost::handle_history_action` via Phase 1b | ✓ | yes (`pushState`/`replaceState`) |
| WHATWG HTML §7.4.4 Non-fragment synchronous "navigations" | *URL and history update steps* step 13 | (ii) staged DURING Phase 2 by a `popstate` handler — **the defect** | next loop iteration's Phase 1b | ✓ | yes |
| WHATWG HTML §7.4.3 Reloading and traversing | *traverse the history by a delta* step 4 (append traversal steps) | (i) in-range → barrier + enqueue | `DrainHost::classify_traversal` per iteration | ✓ | yes (`back`/`forward`/`go`) |
| WHATWG HTML §7.4.3 Reloading and traversing | *traverse the history by a delta* step 4, sub-step 4.4 ("does not exist ⇒ abort") | (ii) out-of-range → no-op, no barrier (Resolution E) | same; **must stay per-iteration** — §4.1 | ✓ | yes |
| WHATWG HTML §7.4.6.1 Updating the traversable | *apply the history step* step 12 (two-part split, "synchronous navigations … before documents unload") | Phase 2 apply, once per iteration | `DrainCoordinator::drain_traversal_queue` | ✓ | yes |
| WHATWG HTML §7.4.6.1 Updating the traversable | *apply the history step* steps 13 + 14.8 (`navigablesThatMustWaitBeforeHandlingSyncNavigation` — init + per-navigable append) | the machinery that decides which staged sync-navs settle before vs after a given change-job | **FENCED OUT** — `#11-sync-navigation-steps-queue-tagging` (single-traversable elidex has one navigable; the per-navigable gate is the tagged-queue work) | ✗ (deliberate) | yes |
| WHATWG HTML §7.4.6.1 Updating the traversable | *apply the history step* step 14.1.1 (nested bounded drain of staged sync-nav steps, bracketed by `running nested apply history step`) | the spec's settle point — **traversal granularity**; this plan is its **turn-granularity** counterpart (§1) | this plan's loop (turn granularity); the between-change-jobs placement is FENCED to `#11-sync-navigation-steps-queue-tagging` | ✗ (deliberate — granularity split, §0) | yes |
| WHATWG HTML §7.4.6.1 Updating the traversable | *apply the history step* step 14.12.4 (targetEntry's document == displayedDocument ⇒ perform `updateDocument` synchronously; 14.12.5 queues a task otherwise) | the same-document apply is synchronous — which is why `popstate` fires *inside* Phase 2 | `apply_traversal_delta` → `traverse_to` → `same_document_step`; the 14.12.5 cross-document task is why a pipeline swap ends the loop (§4.5) | ✓ | yes |
| WHATWG HTML §7.4.6.2 Updating the document | *update document for history step application* step 6.4.3 (fire `popstate`) | the staging vector: the handler that runs synchronously inside the Phase-2 apply | `same_document_step`'s popstate dispatch (`app/navigation.rs:129`) | ✓ | yes |
| WHATWG HTML §7.3.1.1 Traversable navigables | *session history traversal queue* + *running nested apply history step* (initially false) | guard bracket + queue emptiness at turn exit | `TraversalQueue` (`traversal_queue/queue.rs`), exit `debug_assert` | ✓ | no |
| WHATWG HTML §7.4.2.2 Beginning navigation | *navigate* step 19 (ongoing navigation == "traversal") | **FENCED OUT** — enqueue-time suppression divergence | `#11-nav-supersede-window-vs-ongoing-navigation` | ✗ (pre-existing) | yes (`location.*`) |

**Breadth**: K=1 spec; tool section count = 7; **M = 12 manually-expanded data rows** (the table directly
above) → single-PR scope (below the K≥4 / M≥20 split-recommended threshold; the tool's own verdict on the
section-granular input was also "single PR scope").

**In-tree citation drift found while verifying (goes into the §5.2 sweep)**:
`app_history_phase_sep_tests.rs:88` cites "§7.4.6.2 step 6.3 fires popstate in place"; the fire step is
**6.4.3** (step 6.3 is "Restore the history object state"). This PR corrects it while flipping the pins in
that file.

### §3.1 User-input touch audit

Every row above is reachable from ordinary user input, because the drive site *is* the input handler:

- `App::process_pending_navigation` is called from `app/events.rs::handle_click` (`:101`) and
  `handle_keyboard` (`:188`) — any script the page runs on click/keydown reaches every row.
- The page-controlled surface is the `popstate` handler body: it may call `pushState`/`replaceState`
  (§7.4.4 row ii), `back`/`forward`/`go` (§7.4.3 rows), `location.*` (§7.4.2.2 row), or `window.open` —
  and it may do so **unconditionally**, which is exactly the non-termination vector §4.3 bounds.
- Exposure delta: the loop adds no new entry point on the input path; the same script surface runs up to
  N (= the §4.3 cap) times per turn instead of once. The bound-degradation follow-up (§4.3) adds one
  **drive call from the inline redraw path**, gated on a residue flag — a new call site of an existing
  drive, not a new script surface.

---

## §4 The design

### 4.1 The obvious fix is WRONG, not merely insufficient (falsification first)

A trailing `DrainCoordinator::drain_synchronous_updates` after `drain_same_turn` — the literal
transcription of content-mode's R9 fix — **must not be adopted**. It settles a popstate-staged `pushState`,
but a popstate-staged `back()` is peek-classified by that trailing Phase 1b and left **resident on the
`TraversalQueue` across the turn boundary**, because the trailing drain has no Phase 2 behind it.

That resident step is *not* stranded — turn N+1 seeds `seen_traversal` from `has_pending_traversal()`
(`coordinator.rs:119-120`) and drains it — so the damage is not latency. The damage is that it **freezes
the in-range classification a turn early** (axis (e)): between turns, the non-drain cursor movers run, so
by turn N+1 the step may be a no-op while still acting as a **full barrier** — deferring every fresh
`pushState` behind it and latching `suppress_default` true, killing an unrelated `<a href>` default. That
voids the queue's own contract that Resolution E "leaves no `Traversal` step for a no-op, so it does not
over-suppress" (`traversal_queue/queue.rs:122` `has_pending_traversal` doc).

**The non-drain cursor movers, enumerated** (derived by
`grep -rn '\.traverse_to(\|::traverse_to' crates/shell/elidex-shell/src` and
`grep -rn 'self\.navigate(\|app\.navigate(' crates/shell/elidex-shell/src`, non-test, non-drain sites —
the #487-corrected full set):

| Mover | Site | Primitive | Cursor/SoT effect |
|---|---|---|---|
| Chrome toolbar Back/Forward | `app/navigation.rs:505` (`handle_chrome_action`) | `traverse_to` | moves cursor |
| Alt+←/→ | `app/inline.rs:258` | `traverse_to` | moves cursor |
| `<a href>` click default | `app/events.rs:160` | `navigate(Push)` | pushes (truncates forward) or same-document commit |
| Address bar `ChromeAction::Navigate` | `app/navigation.rs:484` | `navigate(Push)` | pushes — **including a same-document arm** (`navigate` `:53-67` takes `same_document_step` for a SameDocument-classified URL, committing the cursor with no rebuild) |
| Chrome `ChromeAction::Reload` | `app/navigation.rs:519` | `navigate(Reload)` | no cursor move, but `restamp_current_document` re-stamps the document identity that same-document classification reads |

None of the five routes through `process_pending_navigation`, so none drains staged residue first — this
is what turns latency into wrong-entry mutation (§1) and what makes a frozen resident classification
stale (this section).

The trailing drain also **contradicts the exit `debug_assert` by construction** (the queue would be
deliberately non-empty at drain exit), which is the tell that it is the wrong shape rather than an
incomplete one.

### 4.2 The ideal — iterate whole drains, not partial phases

Repeat the **entire iteration unit** — `drain_same_turn` (Phase 1 → Phase 2 → ship) **plus the
reinstatement tail** — until the turn is quiescent:

```
// inside the drain_in_progress bracket (§4.7)
let mut outcome = DrainOutcome::default();
for round in 0..MAX_TURN_COMPLETION_ROUNDS {
    let doc_marker = current_document_marker();            // §4.5 (c)
    let iter = DrainCoordinator::drain_same_turn(self);
    let iter = self.reinstate_deferred_navigation(iter);   // the tail, now per-iteration
    outcome.merge(iter);                                   // field-wise OR — §4.5 (a)
    if current_document_marker() != doc_marker { break }   // pipeline swap ends the turn — §4.5 (c)
    if !self.staged_work_pending() { break }               // §4.4 predicate
    if round == MAX_TURN_COMPLETION_ROUNDS - 1 { /* §4.3 degrade */ }
}
```

Every property the trailing drain loses is preserved *because the unit of iteration is a whole cycle*:
each traversal is classified and applied in the same iteration (axis (e)); each iteration's Phase 2
empties the queue, so the exit assert stays true (axis (d)); and each iteration's partition sees a FIFO
containing only intents issued during the previous iteration, which is exactly issue order (axis (c)).

**The reinstatement tail moves inside the iteration** (axis (c) × (g)): a navigation held by iteration
N's Phase 1c and refuted by its Phase 2 must apply **before** iteration N+1's Phase 1 partitions fresh
intents — otherwise a held `location.*` (issued in iteration N) would apply after intents issued in
iteration N+1, inverting issue order. The tail's own contract is untouched: the held request still lives
for at most one iteration and never crosses one (`InteractiveState::deferred_navigation` doc,
`app/mod.rs:252-269`).

**Ship-once across iterations — resolved structurally, not open.** `drain_same_turn` ships at most one
frame per call, so N iterations issue ≤N `ship_frame`s. App-mode's `ship_frame` is
`window.request_redraw()` (`app/drain_host.rs:598-602`), and its own doc records that *"winit coalesces
concurrent requests into one `RedrawRequested`"* — so ≤N requests still produce one frame, exactly as the
dispatch-layer redraw already coexists with the seam today. No accumulate-and-suppress machinery is
needed; the merged `outcome.shipped` (OR) keeps the caller's semantics.

### 4.3 Termination — a constant cap with a guaranteed follow-up drive

A handler that unconditionally re-stages (`onpopstate = () => history.pushState(…)` plus a traversal)
makes the fixpoint unreachable. This runs on the single-writer renderer thread, so an unbounded loop is a
hang.

The in-tree bound idioms, both command-verified (`grep -rn 'MAX_[A-Z_]*' crates/shell/...`):

- **(α) drain-start snapshot bound** — `drain_traversal_queue` captures `pending_len()` once and processes
  only those steps (`coordinator.rs:430`; `queue.rs:137` `pending_len` doc). **Not applicable here**: the
  snapshot idiom terminates a drain of *pre-existing* work by excluding work created during it — but this
  loop's entire purpose is to consume work created during the previous iteration, so a start-snapshot of
  the loop is the degenerate "one iteration", i.e. today's defect.
- **(β) constant cap + next-frame guarantee** — `App::MAX_DRAIN_PER_TAB = 1000`
  (`app/content_messages.rs:23`: *"Any remaining messages will be drained on the next frame"*) and
  `MAX_CE_STABILIZATION_ROUNDS = 8` (`lib.rs:13`, loop at `lib.rs:582-595`: warn via `eprintln!` on the
  final round, *"some mutations may be deferred to next frame"*). **Adopted.**

**Design** (all three parameters concrete; §7 Q2 ratifies the values):

- **Unit**: whole iterations of the §4.2 unit per turn (not steps applied — a single iteration's step count
  is already bounded by the Phase-2 snapshot).
- **Value**: `MAX_TURN_COMPLETION_ROUNDS = 8` — the same order as the CE stabilization cap, and far above
  any legitimate depth (each round requires the page to have staged *new* work from inside the previous
  round's handlers).
- **Observability**: `eprintln!` warning on hitting the cap, mirroring the CE loop — **not** a
  `debug_assert`: an adversarial-but-legal page must not panic a debug build.
- **Degradation with a guaranteed next drive** (the part "no worse than today" cannot supply, since today
  *is* wrong-entry mutation): on hitting the cap, set a `turn_completion_residue: bool` on
  `InteractiveState`, call `request_redraw()`, and have the **inline redraw path** drive
  `process_pending_navigation` when the flag is set (clearing it first; the drive re-runs the loop with a
  fresh cap). winit's `RedrawRequested` is a guaranteed later turn, so the residue is **frame-bounded**,
  not input-bounded — the exact `MAX_DRAIN_PER_TAB` shape ("drained on the next frame"). An adversarial
  re-stager thus degrades to one capped loop per frame: bounded per-frame work, no hang, no unbounded
  residue window for the §1 movers. This adds one call site of the existing drive (premise-5 impact:
  §4.7); it does not add a script surface (§3.1).

Truncation-without-a-scheduled-next-drive — v1's shape — is rejected: it leaves the §1 wrong-entry window
open for exactly the adversarial case, and "never worse than today" is not an acceptable invariant when
today is destructive.

### 4.4 The quiescence predicate — two orthogonal decisions, with measured costs

The staged-work SoT is the **`HostDriver`** staging channels — the engine-side trait the VM stages into
and the shells drain from (`elidex-script-session/src/engine.rs`): `take_pending_history` (`:375`),
`take_pending_navigation` (`:369`), `take_pending_window_opens` (`:387`) — **all consuming**. The drive
site cannot ask "is anything staged?" through them without consuming it. But the trait already has a
**non-consuming peek precedent**: `has_pending_scroll` (`:541`) — *"Non-consuming peek … **Peek, don't
consume** — the render pass remains the single drain point"* (verified by
`grep -rn "Peek, don't consume" crates/`), implemented by the VM engine at
`elidex-js/src/engine.rs:709` over `vm_api_viewport.rs:31`.

**Implementor costs, command-counted** (`grep -rn 'impl HostDriver\|impl DrainHost' crates/`):

- `HostDriver`: **1 production implementor** — `ElidexJsEngine` (`elidex-js/src/engine.rs:374`).
- `DrainHost`: **3 implementors** — `App` (`app/drain_host.rs:364`), `ContentState`
  (`content/drain_host.rs:176`), and `MockHost` (`elidex-navigation/src/traversal_queue_tests.rs:150`).

v1 posed one question ("where does the predicate live?") over options (A)/(B)/(C); those conflate **two
orthogonal decisions**:

**Decision 1 — which layer owns the predicate:**
- **(A) Derive from `DrainOutcome`** ("loop while the last iteration did something"). **Rejected
  explicitly**: `own_context_action` deliberately excludes window-opens (they act on other browsing
  contexts — `coordinator.rs:30-34`), so a window-open-only iteration reads as no-progress and strands the
  opens on the old pipeline's runtime; and "did something" ≠ "something remains" — it always runs one
  useless trailing iteration and still cannot see work staged by the *last* useful one's Phase 2… unless
  it loops one extra time per round, which is (A)'s only honest form and is strictly worse than a peek.
- **(B) A new `DrainHost` method** (`fn has_pending_work(&self) -> bool`). Honest, but costed at the wrong
  layer: **3 implementors** (both shells + the mock), and content's impl would exist only to satisfy the
  trait — the coordinator never calls it (the loop is app-only, §0).
- **(D) A new `HostDriver` non-consuming peek** — the `has_pending_scroll` shape, e.g.
  `fn has_pending_session_history_work(&self) -> bool` returning "history FIFO non-empty ∨ navigation slot
  occupied ∨ window-opens non-empty" (window-opens **included**: app-mode's settle for them is
  drain-and-drop, and excluding them strands opens staged during Phase 2). **1 implementor**, the layer
  that owns the channels, an existing idiom, and a single home for "what counts as staged work". Whether
  it is one composed method or three per-channel peeks is a naming-level choice for plan-review; the lean
  is one method (one decision surface).

**Decision 2 — which layer owns the loop:**
- **Drive site** (`App::process_pending_navigation`). The policy (loop, cap, degrade-to-redraw, residue
  flag) is shell schedule policy; the coordinator stays a stateless phase driver; content-mode is
  untouched by construction.
- **Coordinator** (`drain_to_quiescence`). One-issue-one-way *if* quiescence were a shared concept — but
  content must **not** use it (its task boundary is the point), so this mints a second drive shape with
  exactly one consumer, and it would need the `HostDriver` peek plumbed through `DrainHost` anyway
  (the coordinator cannot see the engine). Rejected unless plan-review finds a second consumer.

**Author's lean: (D) × drive-site.** This is the decision plan-review should own — §7 Q1.

### 4.5 Loop × per-turn state (the `accumulate` contract — no placeholders)

- **(a) `suppress_default` is an OR-latch.** It is the ONE shared default-suppression signal with a
  "single home" (`coordinator.rs:39-55`), consumed by `handle_click` as an early return (`events.rs:101`).
  It describes the **turn**, not the last iteration: if iteration 1 suppressed (an own-context effect or a
  pending traversal) and iteration 2 is a quiet settle returning all-false, the `<a href>` default must
  stay dropped. Therefore `outcome.merge(iter)` is **field-wise OR for all three fields**
  (`own_context_action`, `shipped`, `suppress_default`) — monotone, never cleared within a turn. (Also
  load-bearing for the `hit_entity` staleness invariant in `events.rs:107-149`, which reasons "every
  rebuild path also latched `suppress_default`": OR-latching keeps that reasoning valid across
  iterations.)
- **(b) `deferred_navigation` never crosses an iteration.** The tail runs per-iteration (§4.2), so the
  single slot's existing one-drive lifetime contract (`app/mod.rs:260-268`: cleared by a cursor-moving
  apply, else reinstated-and-taken before the drive returns) becomes a one-**iteration** lifetime. There
  is no overwrite case to define: the slot is provably `None` at each iteration boundary.
- **(c) A pipeline swap ends the loop.** `self.navigate(...)` on its cross-document path replaces
  `interactive.pipeline` wholesale (`load_url_into_pipeline`), and with it
  `pipeline.runtime` — the `HostDriver` whose channels the predicate reads. Two reasons the loop must
  stop, not continue:
  1. **FIFO identity**: the predicate would silently switch to reading the *new* document's runtime;
     "work staged by this turn's handlers" and "the new document's initial staging" become
     indistinguishable.
  2. **Spec task mapping**: §7.4.6.1 step 14.12.5 queues `updateDocument` as a **global task** when the
     target document is not the displayed one — cross-document work belongs to a later task, and a fresh
     document's initial scripts staging history intents are that later task's business (content-mode's
     pump picks them up on a later pump turn; app-mode's next drive does the same). Settling them inside
     the old input turn would run the new document's task inside the old one.
  Detection: compare a pre-iteration document marker; the concrete marker is the current entry's
  `document_sequence` (stamped fresh by every rebuild path — `push`/`replace`/`restamp_current_document`,
  `app/navigation.rs:79-90`), read via `nav_controller`; exact seam to plan-review. Same-document applies
  (fragment nav, same-document traversal) do not restamp, so they do not end the loop — correct, since
  their staged follow-ups are precisely this turn's work.
  The swap-exit also composes with the §4.3 cap: a handler chain that keeps *navigating* terminates at
  the first rebuild, independent of the cap.

### 4.6 Resolution-D across iterations

`traversal_applied` is a **per-drain local** (`coordinator.rs:431`), so each iteration resets it. This is
correct *provided* a `[Traversal, SyncUpdate]` pair can never split across iterations — and it cannot:
Phase 1b enqueues both in the same iteration, and that iteration's Phase-2 `pending_len()` snapshot counts
**all** pending steps, so it captures the pair whole and cancels the straddle. A `SyncUpdate` staged
*afterwards*, by the popstate handler, belongs to the next iteration's Phase 1 and **should** be applied
in-task — that is the point of the fix, and the same outcome content-mode's `drain_synchronous_updates`
produces. To be re-derived under plan-review, not assumed: whether any interleaving exists in which a
cursor-moving traversal in iteration N should have cancelled an intent that iteration N+1 applies (the
turn-granularity residue of exactly this shape is the §0 fence).

### 4.7 Premise 5, restated for a site-driven loop

The entry `debug_assert!(!drain_in_progress)` (`drain_host.rs:221`) guards **body-driven re-entry**: a
`DrainHost` seam body, an apply body, or the reinstatement tail calling back into the drive, which
interleaves two partitions and silently voids I2 and the Resolution-D latch. A **site-driven** loop is
categorically different: iterations are strictly sequential, each starting only after the previous one's
bodies have all returned. The loop therefore lives **inside** the `drain_in_progress` window and the
entry assert is unchanged in force — but the premise-5 prose ("no app-mode apply body drives the Phase-1
partition", module doc `drain_host.rs:76-96`) must be restated to name the distinction (sequential
site-driven iteration = legal; nested body-driven re-entry = the bug), or the next maintainer will "fix"
the assert.

**The exit assert (`:301`) is UNCHANGED — in text, force, and meaning.** It reads only
`traversal_queue().is_empty()`, which each iteration's Phase 2 re-establishes; and on the §4.3
bound-degradation path the residue lives on the **VM FIFO**, which the assert never inspects — so there is
no exit-assert-vs-residue tension to reconcile (v1's Q4 was a phantom conflict and is retired). What DOES
change is the assert's **message** (`:303-311`), which currently explains the residual's unbounded
lifetime via "app-mode pumps only on input, `#11-app-mode-turn-completion-drain`" — after this PR the
slot is closed and the lifetime is frame-bounded; the message joins the §5.2 sweep.

The **"SOLE site" language** (`drain_host.rs:131-134`) gains one caller: the residue-flagged redraw drive
(§4.3). The property that matters — every drive goes through `process_pending_navigation`'s guard pair —
is preserved; the doc is updated to name both callers.

### 4.8 OO→ECS / layer map (umbrella plan §4.5 style — reviewer verification without re-deriving §4)

Spec *"the task runs, then its staged sync-nav settles"* → **drive-site schedule policy** on elidex's
single-writer event loop (CLAUDE.md Concurrency-by-ownership: the settle happens inside the input-handler
phase window; no new thread, no shared-lock reconciliation). Spec *session history traversal queue* +
*running nested apply history step* → unchanged (`elidex-navigation`, Slice-B shapes). The **quiescence
predicate** → an engine-boundary **channel peek** (`HostDriver`, §4.4 (D)) — the ECS-native idiom here is
the existing staged-channel + drain-point pattern (`has_pending_scroll`), NOT an OO observer/callback
("notify me when work is staged") which would invert ownership and re-enter the VM from the shell. The
**per-turn accumulation state** (`outcome`, doc marker, residue flag) → turn-scoped locals plus one
`bool` on `InteractiveState` — browsing-context/session-level state under CLAUDE.md side-store exception
(b), same as `traversal_queue`/`deferred_navigation` beside it; nothing here is per-entity, so no ECS
component is warranted and no side-store registry is minted. Crate placement: the loop + cap + residue
flag = `elidex-shell` (app only); the peek = `elidex-script-session` trait + `elidex-js` impl;
`elidex-navigation` gains **no** code, only doc-passage updates (§5.2).

---

## §5 Decomposition

Single PR under the approved umbrella (edge-dense base case: a narrowly-scoped per-PR slice that has
passed plan-review is a terminal unit). Source files are bounded (`app/drain_host.rs` 651 lines,
`app/events.rs` 230, `elidex-script-session/src/engine.rs` gains one trait method + one impl in
`elidex-js`). If plan-review overturns §4.4 toward the coordinator-owned loop, that becomes an
`elidex-navigation` behavior change touching both shells and must be re-sliced as its own prereq PR.

### §5.1 Test-file audit (touch-time split discipline — the file this PR grows)

`app_history_phase_sep_tests.rs` is **756 lines** (`wc -l`, verified at `06e632ae`). §8 adds three new
tests and flips/rewrites three existing ones; landing all of that in place approaches the 1000-line
guideline — the exact debt PR #490 just discharged for this suite. The file's own module doc names its
scenario seams (partition ordering / Resolution E / Resolution B consumer / liveness + the two
slot pins co-located at `:15-23`), so the seam is real:

**Decision: the turn-completion scenario family moves to a NEW sibling file,
`app_turn_completion_tests.rs`, in this PR** (new-file placement while writing, per
touch-time-split-means-while-writing — not a prereq split PR, since nothing is over the line today):
- moves there: the two flipped slot pins (`app_popstate_staged_action_…`, `:604`;
  `app_popstate_staged_push_destroys_…`, `:691`) — after the flip they *are* turn-completion conformance,
  not phase-separation pins — plus the three new §8 tests.
- stays in `app_history_phase_sep_tests.rs`: the straddle pin (`:490`, it pins
  `#11-sync-navigation-steps-queue-tagging`, a partition-granularity fence) with its §6 assertion
  restructure; everything else untouched.
- both files' module docs updated: the phase-sep doc's "Both `#11-…-turn-completion-drain` pins live
  here, together" (`:15-23`) moves with the pins; the co-location rationale (two facets of one slot)
  survives in the new file; the content-side third-pin cross-check note (`:25-32`) moves with them.

Net: phase-sep shrinks to ≈530 lines, the new file starts ≈400 — both bounded, seams honest.

### §5.2 Sibling sweep — every passage this PR must update (command-derived)

Derived by `grep -rn '11-app-mode-turn-completion-drain' crates/ --include='*.rs'`,
`grep -n 'app-mode\|App-mode' crates/shell/elidex-navigation/src/traversal_queue/*.rs`, and
`grep -rn 'no post-Phase-2\|ONE .drain_same_turn' crates/shell --include='*.rs'`:

**`elidex-shell` (the slot's own fences — flip from "gap" to "how the loop works")**:
1. `app/drain_host.rs:36-46` — module-doc "no post-Phase-2 synchronous settle … fenced to
   `#11-app-mode-turn-completion-drain`".
2. `app/drain_host.rs:145-193` — the `process_pending_navigation` ⚠ block (the gap statement + the
   trailing-drain falsification; the falsification text survives, re-anchored as design rationale).
3. `app/drain_host.rs:301-312` — the exit-assert message's "nothing bounds WHEN that turn arrives
   (app-mode pumps only on input, `#11-app-mode-turn-completion-drain`)" (§4.7).
4. `app/mod.rs:244-250` — `InteractiveState::traversal_queue` doc ("no async pump, so its Phase 2 drains
   at the END of the same input handler") — gains the loop.
5. `app_history_phase_sep_tests.rs:15-23, :25-32, :49-55` — module-doc pin co-location + "no post-Phase-2
   synchronous settle" + "has no app-mode twin" (moves/updates with §5.1).
6. `app_history_phase_sep_tests.rs:88` — the §7.4.6.2 popstate step-number drift (§3).

**`elidex-navigation` (doc-only; no code change — passages that state the app-mode schedule)**:
7. `traversal_queue/mod.rs:56-59` — "App-mode calls none of those three … drains Phase 1 and Phase 2
   back-to-back inside the input handler through `drain_same_turn`" — still true per-iteration; gains
   "repeated to quiescence by the drive site".
8. `traversal_queue/coordinator.rs:329-342` — `drain_same_turn` doc ("ONE frame … end-of-input-handler
   drain") — per-iteration framing + the ship coalescing note (§4.2).
9. `traversal_queue/coordinator.rs:413-429` — the R18-latch carry passage ("unreachable in app-mode
   Slice-B — BY CONSTRUCTION … no apply body re-enters `run_synchronous_phase_body` mid-drain"): the
   argument SURVIVES (premise 5 is untouched by a site-driven loop, §4.7) but its wording assumes one
   drain per turn; re-audit + reword against the loop.
10. `traversal_queue/host.rs:129-138` — the classify-hoist soundness note's app-mode leg ("app-mode:
    structurally void, no reentrant Phase 1") — survives; re-audit the "needs a step to survive a drain"
    sentence against per-iteration drains (each iteration still empties the queue, so it holds).
11. `traversal_queue/queue.rs:20-24` — "app-mode is structurally reentrancy-free" — survives; reword to
    name the loop as site-driven.

(`coordinator.rs:460-480` — the Phase-2 apply-site KNOWN-DIVERGENCE comment and its app-mode pin
reference — stays: it documents the queue-tagging fence, whose pin this PR restructures but does not
flip, §6.)

---

## §6 What this closes / does not close / flips

**Closes**: `#11-app-mode-turn-completion-drain`.

**Flips — BOTH slot pins** (their shared docstring: *"Both flip when the fix lands."*,
`app_history_phase_sep_tests.rs:577`):
- `app_popstate_staged_action_defers_to_the_next_drain_not_the_current_queue` (`:604`) → the content shape:
  the popstate-staged `pushState` settles within the same `process_pending_navigation` call (iteration 2);
  the second drive in the test becomes a no-op and is removed.
- `app_popstate_staged_push_destroys_forward_entries_after_an_interleaved_chrome_traversal` (`:691`) → the
  staged push now settles at the end of the SAME turn, against `/a` — the entry whose handler issued it —
  yielding `[base, /a, /from-popstate]` (forward-`/b` truncation is §7.4.4 push semantics, not the
  divergence); the interleaved chrome Back then arrives *after* the turn completed and traverses normally.
  The destruction scenario becomes unreachable on this path (residually reachable only via §4.3
  bound-degradation residue — §7 Q3).

**Restructured but NOT flipped — the straddle pin** (`app_multi_traversal_snapshot_lands_popstate_staged_update_on_the_wrong_entry`, `:490`,
owned by `#11-sync-navigation-steps-queue-tagging`). Read against the test body:
- `:504-511` (both traversals drained in one snapshot; cursor nets onto `/a`) — **unchanged**.
- `:513-518` — asserts the staged replace is "NOT settled by the drain that fired it (it is still on the
  VM FIFO)" — **flips**: with turn completion it IS settled by the same drive, in iteration 2, against the
  post-`forward()` cursor.
- `:521` (the second `process_pending_navigation`) — becomes a no-op; removed.
- `:523-534` — the final wrong-entry assertions (the replace lands on `/a`, destroying it; `base`, the
  entry whose handler issued it, is untouched) — **the fence, unchanged in substance**, now asserted after
  the FIRST drive. The docstring's deferral sentence is rewritten from "next drive" to "later iteration of
  the same turn — the traversal-granularity placement (§7.4.6.1 step 14.1.1 between change-jobs) is the
  fenced work".

**Does NOT close, and must not appear to**: `#11-sync-navigation-steps-queue-tagging` (the
traversal-granularity settle, per §0), `#11-nav-supersede-window-vs-ongoing-navigation`,
`#11-nav-applied-shipped-decouple`.

---

## §7 Open questions for `/elidex-plan-review` (decision-level; v1's resolved/phantom Qs removed —
Q3 ship-once resolved in §4.2, Q4 exit-assert retired in §4.7, Q5 subsumed by §6, Q6 withdrawn: all six
early-return sites precede `dispatch_event` (`events.rs:76`, `:181`), so no script runs and nothing is
staged on those turns — the early returns delay *draining*, which the §4.3 follow-up drive now bounds)

- **Q1 — predicate layer × loop owner** (§4.4): author leans (D) `HostDriver` non-consuming peek ×
  drive-site loop. (A) is refuted, not merely disfavored; (B) costs 3 implementors to (D)'s 1; coordinator
  ownership mints a one-consumer drive shape. Ratify or overturn (overturning to coordinator ⇒ re-slice,
  §5).
- **Q2 — bound parameters** (§4.3): ratify cap value (8) and the degrade shape (residue flag +
  `request_redraw` + flag-gated redraw drive). The *structure* (constant cap, `eprintln` observability,
  frame-bounded residue) is design, not open.
- **Q3 — the bound-degradation residual window**: on the degraded path, a non-drain cursor mover (§4.1
  table) can still run between the capped turn and its redraw follow-up, reproducing §1's wrong-entry
  shape for adversarial pages only. Accept as the bounded residual of this slice (lean — it is
  frame-narrow, adversarial-only, and mover routing is Slice 4's question), or require the five movers to
  drive the drain first (touches `handle_chrome_action`/`inline.rs`, pre-empting Slice 4)?

---

## §8 Test strategy

All new/flipped turn-completion tests live in `app_turn_completion_tests.rs` (§5.1).

- **Flip** both slot pins per §6 (new file); **restructure** the straddle pin per §6 (stays in
  `app_history_phase_sep_tests.rs`; its final wrong-entry fence assertions must still fail-if-fixed the
  same way — guard against silently narrowing the queue-tagging divergence).
- **New**: a popstate handler staging a `pushState` settles within the same
  `process_pending_navigation` call (the direct content-shape twin of
  `pump_drains_popstate_staged_pushstate_this_turn`).
- **New**: a popstate handler staging a `back()` is applied within the same turn — the case the
  trailing-drain alternative gets wrong: assert the traversal queue is empty at drive exit, the cursor
  moved, and a subsequent click's `<a href>` default is NOT suppressed by any surviving latch (axis (e)).
- **New (termination + degrade)**: a handler that re-stages unconditionally terminates at the cap with
  the residue still staged and `turn_completion_residue` set; a subsequent redraw-path drive picks it up
  (frame-bounded, not input-bounded). Assert per-turn iteration count never exceeds the cap.
- **Accumulation (axis (g))**: a turn whose iteration 1 latches `suppress_default` and whose iteration 2
  is a quiet settle still suppresses the `<a href>` default (OR-latch; through the real click path).
- **Pipeline-swap boundary**: covered structurally by the disconnected harness (a successful mid-loop
  rebuild is unreachable — same limitation the `hit_entity` invariant records, `events.rs:125-126`);
  pinned instead at the unit the marker uses (`document_sequence` restamp on rebuild,
  `elidex-navigation/src/navigation_tests.rs` already covers the restamp). If plan-review wants a live
  pin, it needs the threaded harness and is out of this slice's reach — name it, don't fake it.
- **Unchanged**: `app_trailing_syncupdate_canceled_behind_cursor_moving_traversal`,
  `app_drain_same_turn_leaves_no_residual_…`, all content-mode tests, all `elidex-navigation` isolation
  tests (no coordinator behavior change under §4.4 lean).

---

## §9 Defer ledger

Own-deferral budget: expected **0–1**. The §7 Q3 residual, if accepted, is NOT a new slot — it is the
bound-degradation footnote of this slot's close, recorded in the closing note and subsumed by Slice 4's
mover routing. Anything else discovered mid-implementation that is not the turn-completion loop belongs to
an existing slot (§6) or a new one with the 3-element audit (`Why deferred` / `Re-evaluation trigger` /
`Re-evaluation date`).
