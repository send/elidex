# Plan-memo — `#11-session-history-task-queue-model` (the "#396 root")

> Status: **pre-implementation DESIGN plan-memo**. Goes through `/elidex-plan-review` before any
> implementation. Anchored on the first-principles spec-faithful ideal, not a surgical patch.
> Lineage: reframed and scoped by `docs/plans/2026-07-s5-5c-history-state-traversal.md` §4.3 as
> "the dedicated S5-5d task-boundary restructure" (D5), subsuming #259 / #283 / #448-issue + E7 +
> chrome-button traversal atomicity. 5c shipped "complete-and-shippable on the collapsed synchronous
> model"; this memo restructures the *timing* that 5c deliberately left collapsed.

---

## §0 Decision + scope (fence up front)

**Decision.** Restructure the single synchronous per-turn navigation drain into the spec's two task-timing
classes: (a) **synchronous, in-task** URL/history *updates* (`pushState` / `replaceState` / fragment nav /
`location.*` — WHATWG HTML §7.4.4 *URL and history update steps*, run in the current task) versus (b) a
**deferred, task-queued traversal** (`history.back()` / `forward()` / `go(delta)` — §7.4.3 *traverse the
history by a delta* step 4 *appends* the traversal onto the traversable, applied as a separate scheduled
task via §7.4.6.1 *apply the history step*). Model the traversable's **session history traversal queue**
(§7.3.1.1) as a first-class deferred queue in the **engine-agnostic** `elidex-navigation` layer so **both
shells share one restructured drain primitive** rather than the two divergent synchronous drains they run
today. Carry the queue's **"running nested apply history step" boolean** (§7.3.1.1) as the reentrancy guard,
which by construction retires the `commit_index` peek-then-commit `debug_assert` workaround.

**Scope fence (critical — cite this, it is the load-bearing boundary).** This slot implements the
**single-traversable (top-level) task-boundary phase-separation ONLY**. The **multi-navigable fan-out** of
§7.4.6.1 — *apply the history step* step 4 (*"get all navigables that might experience a cross-document
traversal"*), step 6 *changingNavigables* across an iframe subtree, and the per-navigable "queue a global
task on the navigation-and-traversal task source of navigable's active window" of steps 8/12 — is **out of
scope, deferred to the friendly-iframe layer** (B1). Iframes do not share a `Vm` and the friendly-iframe
navigable tree is not built yet: `docs/plans/2026-06-agent-scoped-ecsdom-world.md` §6.3. Under the current
single-top-level-traversable reality, *changingNavigables* is always the one-element set `{top-level}`, so
the fan-out collapses to a no-op and modelling it would be speculative abstraction for a navigable tree that
does not exist (Phase-design-is-not-a-shortcut-license: the fan-out gets a named seam, not an implementation).

**Edge-density flag.** This is **edge-dense work** (≥3 intersecting invariant axes — see §2) touching a
subsystem with no canonical algorithm already in the tree. Per CLAUDE.md it **must not ride a single PR**:
it lands as an **umbrella + per-PR-plan-reviewed slices** (§5), each slice its own `/elidex-plan-review`.

---

## §1 The decisive invariant — the spec's task-timing model vs elidex's single synchronous drain

**Spec model (webref-verified 2026-07-13, all §↔title pairs looked up — see report; note the section
*titles* differ from the algorithm *dfn* names).**

The HTML LS separates a history operation's **state update** from its **traversal** by *task timing*:

1. **§7.4.4 "Non-fragment synchronous 'navigations'"**, algorithm *URL and history update steps*
   (`#url-and-history-update-steps`). Runs **synchronously in the current task**. Steps 1–11 mutate the
   session-history entry directly: allocate `newEntry`, on `"push"` **increment the history object's index
   and set length to index+1** (step 6, annotated *"temporary best-guess values for immediate synchronous
   access"*), set the document URL (step 8), set the navigable's active session history entry (step 10).
   Then step 13 **appends** *"the following session history synchronous navigation steps involving
   navigable"* to the traversable (a *finalize a same-document navigation* + WebDriver-BiDi tail). So even the "synchronous" path **appends onto the traversable's ONE session history traversal queue** (§7.4.1.3 *Centralized modifications of session history* defines both append ops), as *synchronous navigation steps* tagged to "jump the queue" — NOT a separate queue. The timing model is finer than a binary sync/async but is still **one tagged queue** (see §3, and Q-SYNC-FINALIZE in §7).

2. **§7.4.3 "Reloading and traversing"**, algorithm *traverse the history by a delta*
   (`#traverse-the-history-by-a-delta`). `history.back()/forward()/go(delta)` land here. Step 4 is the
   decisive verb: **"Append the following session history traversal steps to traversable"** — the traversal
   is **queued** on the traversable, **not run synchronously**. The queued steps resolve `targetStepIndex`
   from `allSteps` and *apply the traverse history step* → §7.4.6.1.

3. **§7.4.6.1 "Updating the traversable"**, algorithm *apply the history step* (`#apply-the-history-step`).
   Step 1 **asserts "This is running within traversable's session history traversal queue"**. It computes
   *changingNavigables* and, for each, **"Queue a global task on the navigation and traversal task source of
   navigable's active window"** (steps 8 and 12). **Step 12's design note is the whole point of this slot:**
   *"This set of steps are split into two parts to allow synchronous navigations to be processed before
   documents unload."* Step 12.4 makes it concrete — if `displayedEntry is targetEntry` (a synchronous
   navigation *already updated* the active entry) the continuation is marked **update-only** and aborts the
   unload path. That "sync-already-landed" branch is only reachable **because the sync updates run in an
   earlier task than the traversal apply**.

4. **§7.3.1.1 "Traversable navigables"**, concept *session history traversal queue*
   (`#tn-session-history-traversal-queue`, in `document-sequences.html`). A per-traversable **session
   history traversal parallel queue** carrying a **"running nested apply history step" boolean, initially
   false** — the reentrancy guard that serializes re-entrant applies.

**The invariant, stated:** *a same-turn synchronous update (§7.4.4) must be fully applied before a same-turn
traversal (§7.4.3 → §7.4.6.1) observes the entry list.* The spec guarantees this by running the update in
the current task and the traversal in a **later** queued task on the navigation-and-traversal task source.

**How elidex's single synchronous drain violates it.** Both shells drain *everything* in one synchronous
pass, ordered window-opens → history-FIFO → navigation-last-wins, and **collapse the §7.4.3 queued-task
boundary onto a synchronous early-return**:

- `crates/shell/elidex-shell/src/content/navigation.rs` `process_pending_actions` (`:529`): history FIFO
  loop `:570–595`, where a traversal `handle_history_action → true` triggers an **immediate `return true`**
  (`:593`) that supersedes the document *in the same pass*. The collapsed boundary is called out verbatim in
  the comment at `:583–592` ("The single-navigable 'traversal = a later task' boundary the spec models
  (§7.4.3 queued task) is collapsed onto this synchronous return … the reframed deferred slot
  `#11-session-history-task-queue-model`"). Navigation-last-wins follows at `:609–628`.
- `crates/shell/elidex-shell/src/app/navigation.rs` `process_pending_navigation` (`:34`): the *same* 3-phase
  order, history `:59–75` with the identical `return true` traversal-supersede (`:73`), navigation `:89–95`.

Because the traversal runs *in the same synchronous pass as, and immediately after, the history FIFO of the
same turn*, elidex cannot honor §7.4.6.1 step-12's split: a `history.back(); pushState('/x')` turn either
supersedes on the `back()` (discarding the trailing `pushState`) or — if the trailing intents were reordered
— applies the traversal against an entry list the same-turn sync update already mutated, with no task
boundary distinguishing "sync landed, then traverse" from "traverse, then sync." That is the **E7 residual**
5c documented and handed here.

---

## §2 Coupled invariants (the edge matrix — plan-review checks each axis independently)

This work binds **five** intersecting invariant axes. Naming them so `/elidex-plan-review` can verify each
without re-deriving the whole:

- **(a) Task-boundary phase-separation.** The synchronous drain processes window-opens + **synchronous**
  nav/history *updates* (§7.4.4) in the current task; **traversals** (§7.4.3) are appended to the traversal
  queue and applied as a **separate scheduled task**, so the sync updates have already landed when the
  traversal applies (§7.4.6.1 step 12). *Failure mode:* a same-turn `pushState` lost to a `back()` supersede,
  or a traversal seeing a half-updated entry list.
- **(b) Traversal-queue reentrancy / serialization.** The §7.3.1.1 "running nested apply history step"
  boolean serializes a re-entrant nav-mutating message that arrives *while a traversal step applies* — the
  SW-fetch synchronous message pump (`content/navigation.rs:770–776`) is the named vector. *Failure mode:* a
  reentrant drain stales a held cursor (the `commit_index` `debug_assert` case, `:255–261`).
- **(c) Two-shell mirror unification.** Content mode (`content/navigation.rs:529`) and app mode
  (`app/navigation.rs:34`) run **two divergent synchronous drains** driving one engine-agnostic
  `NavigationController`. One-issue-one-way forbids fixing content-mode and leaving app-mode a fork; the
  restructured drain primitive must be **shared** (its home is the engine-agnostic layer — §4/§7 Q-OWNER).
  *Failure mode:* a strangler middle-state with a queued content drain and a synchronous app fork.
- **(d) VM-staging "turn" model.** The VM defines a *turn's* actions in
  `crates/script/elidex-js/src/vm/host/navigation.rs`: `pending_history: VecDeque<HistoryAction>` FIFO
  (`:159`, push/replace synchronous, traversal enqueue-only) + `pending_navigation: Option<NavigationRequest>`
  last-wins (`:143`), drained once per turn via `take_pending_history`/`take_pending_navigation`
  (`engine.rs:574/:570`). The phase-separation must decide whether this staging model changes or **only the
  shell drain does** (§7 Q-VM-MODEL). *Failure mode:* duplicating the queue in two layers.
- **(e) Cursor atomicity (peek / commit).** `NavigationController` peeks a target without moving the cursor
  (`peek_back/forward/go :222/:229/:237`) and commits only on load success (`commit_index :255`); the
  `debug_assert :256–261` **explicitly names the reentrant-drain case deferred to this slot**. The traversal
  queue must serialize applies so peek→commit is atomic *by construction*, retiring the assert. *Failure
  mode:* the assert becoming reachable (an `entries` mutation between peek and commit).

**Axes (a) × (b) × (c) intersect nontrivially** (a queued traversal that also serializes reentrancy and is
shared across two shells), plus (d)/(e) are the data-model and cursor invariants the restructure must not
break. ≥3 intersecting axes ⇒ **umbrella + per-PR plan-review**, per CLAUDE.md edge-dense rule.

**Pairwise intersections** (each cell → where its invariant is pinned; (a)–(e) per the bullets above):

| × | (b) reentrancy | (c) two-shell | (d) VM-staging | (e) cursor atomicity |
|---|---|---|---|---|
| **(a) phase-sep** | queued apply that also serializes reentrancy (§4.4 / I3) | shared drain primitive, both shells (§4.1 / §4.3) | **I1 ordering + I2 partition** (§4.5) — the FIFO-split reconciliation | **I1** ordering ⇒ apply reads a committed list; **I3** keeps peek→commit atomic (§4.5) |
| **(b) reentrancy** | — | guard lives in the shared primitive (§4.1) | **I3** serializes the reentrant SW-pump message onto the **I2** single-FIFO-fed queue (§4.4 / §4.5) | **I3 bracket** covers peek→commit (§4.5) |
| **(c) two-shell** | — | — | both shells drain the same VM staging (§4.2) | both shells share peek/commit (§4.1) |
| **(d) VM-staging** | — | — | — | partition preserves single-FIFO ordering SoT (I2) |

(d) and (e) ARE intersecting axes — not merely "constraints" — via I1/I2/I3 (§4.5); the earlier "must not break" phrasing understated them.

---

## §3 Spec coverage map (supported-surface step lists — IN vs B1-gated OUT)

Breadth target: the **single top-level traversable** task-boundary. Steps whose only effect is the
multi-navigable fan-out are OUT (B1-gated). Prose pulled via `.claude/tools/webref body html <anchor>`.

Canonical single-table schema (preflight); section labels use the webref **section titles** (verified 2026-07-13; titles differ from the algorithm *dfn* names — see §1). `Full enum?` = ✓ when the row's in-scope branches are exhaustively covered; `n/a (B1)` / `✗` mark the B1-gated or open-question rows. `User-input flow` = a JS/history-API caller reaches it.

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §7.4.4 Non-fragment synchronous "navigations" | 1–3 | *URL and history update steps*: resolve navigable/activeEntry, build `newEntry` | IN — `state_mutate` (`vm/host/history.rs:147`) | ✓ | yes (pushState/replaceState/`location.*`) |
| WHATWG HTML §7.4.4 Non-fragment synchronous "navigations" | 4 | `is initial about:blank` ⇒ force `"replace"` | OUT — shared `is_initial_about_blank` flag deferred (5c §4.4) | ✓ | yes |
| WHATWG HTML §7.4.4 Non-fragment synchronous "navigations" | 5–6 | `entryToReplace`; on push increment index + best-guess length | IN — `NavigationState.current_index`/`history_length` (`vm/host/navigation.rs:114`/`125`) | ✓ | yes |
| WHATWG HTML §7.4.4 Non-fragment synchronous "navigations" | 7–11 | restore state, set URL, set active entry, update nav-API entries | IN — synchronous in-task class | ✓ | yes |
| WHATWG HTML §7.4.4 Non-fragment synchronous "navigations" | 12–13 | append *session history synchronous navigation steps* (finalize + BiDi) to traversable | PARTIAL — Q-SYNC-FINALIZE: sync-nav steps tagged on the ONE queue (§7.4.1.3), fenced to B1 (§7) | ✗ (open Q) | yes |
| WHATWG HTML §7.4.3 Reloading and traversing | 1–3 | *traverse the history by a delta*: snapshot source params, userInvolvement | IN — single-navigable (initiator = top-level) | ✓ | yes (back/forward/go) |
| WHATWG HTML §7.4.3 Reloading and traversing | 4 | **append traversal steps to traversable** → resolve `targetStepIndex`, apply | IN — the queue-boundary this slot builds | ✓ | yes |
| WHATWG HTML §7.4.6.1 Updating the traversable | 1 | *apply the history step*: assert running within traversal queue | IN — the reentrancy-guard invariant | ✓ | no |
| WHATWG HTML §7.4.6.1 Updating the traversable | 2 | `targetStep` = getting the used step | IN — maps to `peek_go`/index resolution | ✓ | no |
| WHATWG HTML §7.4.6.1 Updating the traversable | 3 | initiator sandbox check across changing navigables | OUT (B1) — single navigable | n/a (B1) | no |
| WHATWG HTML §7.4.6.1 Updating the traversable | 4 | get all navigables that might cross-document traverse | OUT (B1) — always `{top-level}`, the fan-out fence | n/a (B1) | no |
| WHATWG HTML §7.4.6.1 Updating the traversable | 5 | checkForCancelation / unloading canceled | PARTIAL — mechanism IN, cross-doc unload set OUT (B1) | ✗ (B1) | no |
| WHATWG HTML §7.4.6.1 Updating the traversable | 6 | `changingNavigables` | OUT (B1) — one-element `{top-level}` set; classification stays `resolve_traversal` | n/a (B1) | no |
| WHATWG HTML §7.4.6.1 Updating the traversable | 7 | nonchanging navigables needing length/index update | OUT (B1) — no sibling navigables | n/a (B1) | no |
| WHATWG HTML §7.4.6.1 Updating the traversable | 8 | per-navigable: set current entry; **queue global task on navigation-and-traversal task source** | IN (top-level only) — the single queued apply task | ✓ | no |
| WHATWG HTML §7.4.6.1 Updating the traversable | 9–11 | totalChangeJobs / continuations queue | PARTIAL — two-part split IN conceptually, multi-job bookkeeping OUT (one job) | ✗ (one job) | no |
| WHATWG HTML §7.4.6.1 Updating the traversable | 12 | **two-part split** ("processed before documents unload"); **12.4 update-only** when `displayedEntry is targetEntry` | IN — the decisive phase-separation + fast path | ✓ | no |
| WHATWG HTML §7.4.6.1 Updating the traversable | 13–18 | populate/unload/URL update per navigable | PARTIAL — top-level update IN (via `traverse_to`/`handle_navigate`), cross-navigable OUT (B1) | ✗ (B1) | no |
| WHATWG HTML §7.3.1.1 Traversable navigables | queue obj | session history traversal **parallel queue** | IN — cooperative deferred queue on the event loop (§4) | ✓ | no |
| WHATWG HTML §7.3.1.1 Traversable navigables | queue obj | **"running nested apply history step" boolean** (init false) | IN — axis (b) reentrancy guard | ✓ | no |
| WHATWG HTML §7.3.1.1 Traversable navigables | queue obj | system visibility / is-created-by-web-content | OUT — page-visibility + popup provenance, unrelated | n/a | no |

**Breadth**: K=1 spec (html), M=21 entries (21 table rows, verified 2026-07-13). **Split decision by spec-breadth = single-PR**, BUT the *design* is edge-dense (§2, 5 intersecting axes) → the split is by **implementation slice** (§5 umbrella), not spec breadth — spec-breadth K/M under-counts design edge-density here (the canonical-algorithm-absent-subsystem MANDATORY trigger, not the K≥4/M≥20 trigger, is what forces the umbrella).

**Breadth verdict.** The supported surface is the **single-top-level-traversable phase-separation +
reentrancy serialization**: §7.4.3 step 4's queue-append, §7.4.6.1 step 1/8/12 (the one-navigable apply +
two-part split + update-only fast path), and the §7.3.1.1 queue object with its nested-apply boolean. The
**multi-navigable fan-out** (§7.4.6.1 steps 3/4/6/7 and the per-navigable global-task fan-out of 8/12) is
**out of scope and B1-gated**. The §7.4.4 step-13 *synchronous navigation steps* are tagged step-sets on the ONE traversal queue (§7.4.1.3), resolved (Q-SYNC-FINALIZE, §7) as **fenced to B1** (Q-SYNC-FINALIZE): whether elidex models it as a distinct third queue class now, or keeps folding
its finalize into the synchronous drain (defensible under single-traversable, where there is no cross-doc
unload to sequence against). No new spec surface is *removed*; the restructure re-times the existing surface.

---

## §4 The design (skeleton items 1–4, fully worked)

### 4.1 The queue abstraction + its owner (skeleton 1)

Introduce a **traversable session-history-traversal queue** as a first-class object in the engine-agnostic
`elidex-navigation` crate, **at/near `NavigationController`** — `NavigationController` is elidex's de-facto
traversable proxy (it owns `entries` + the scalar cursor `index`, and is owned by `ContentState` /
`InteractiveState` so it *survives pipeline rebuild*, `navigation.rs:67`). Placing the queue here means
**both shells share one primitive** (axis c) and the classification/cursor invariants (axes d/e) stay in the
engine-independent layer they already live in (`resolve_traversal :344`, `TraversalKind :451`).

Queue shape (decision-altitude — exact fields handed to implementation):
- an **ordered queue of pending traversal applies** (each carrying the resolved delta/target and the
  source-snapshot/userInvolvement inputs §7.4.3 steps 1–3 captured), and
- the **`running_nested_apply_history_step: bool`** guard (§7.3.1.1), initially false.

This is a **cooperative single-threaded queue**, not an OS "parallel queue" thread: elidex's renderer main
thread is the sole owner/writer of the DOM + navigation state (CLAUDE.md "Concurrency by ownership and
phases"). The spec's *parallel queue* is realized as a **deferred task on elidex's existing event loop**
(§4.3), preserving the single-writer discipline — modelling it as a real thread would violate the ownership
mandate and is not what the two-part split needs (the split needs *ordering*, not parallelism).

### 4.2 The drain restructure for both shells (skeleton 2)

Split the single `process_pending_actions` / `process_pending_navigation` pass into two phases:

- **Phase 1 — synchronous, in-task (unchanged timing):** window-opens (`content/navigation.rs:542`) →
  synchronous history *updates* only (the `PushState`/`ReplaceState` FIFO entries — §7.4.4) →
  synchronous navigation (`location.*` last-wins; §7.4.2.2 *navigate*, fragment = §7.4.2.3.3 *Fragment navigations*). These already mutate
  `NavigationController` / rebuild the pipeline in the current task and must keep doing so.
- **Phase 2 — deferred traversal apply (new task boundary):** a `Back`/`Forward`/`Go` `HistoryAction`
  drained in Phase 1 is **not** applied inline; it is **appended to the traversal queue** and applied by a
  **separately scheduled task** that runs *after* Phase 1's synchronous updates have landed. The apply task
  is the existing `handle_history_action`/`traverse_to` body, now invoked from the queue rather than from the
  FIFO loop, guarded by the nested-apply boolean.

The FIFO's current **`return true` on traversal-supersede** (`:593` / `:73`) is **removed**: the loop no
longer stops replaying trailing intents. Trailing *synchronous* intents belong to Phase 1 and the deferred
traversal applies afterward — realizing §7.4.6.1 step 12's "synchronous navigations processed before
documents unload" + §12.4's update-only fast path. ⚠ **Same-turn straddle caveat (I2, §4.5):** where a
`pushState` is issued *after* a same-turn `back()`, elidex's **tagged one-queue, issue-order-preserving**
model (Slice-1 as-built, I2/§4.5) defers the trailing `pushState` onto the one tagged queue **after** the
`back()` (never hoisted ahead of it); the residual bounded divergence from the spec's exact queue-reconciliation
(§7.4.2.3.3 finalize / §7.4.1.3 jump-the-queue) is pinned by Slice-1's conformance test. Do NOT read this paragraph as
"trailing sync always lands first = spec"; that over-claim is corrected in I2.

### 4.3 The task-source / scheduling mechanism (open-question-flagged)

The spec queues the apply "on the navigation-and-traversal task source." elidex realizes a *"separate
scheduled task"* differently in the two shells today — this is the **central open question** (Q-SCHED, §7):

- **Content mode has an async pump.** `content/event_loop.rs` `run_event_loop` (the async pump, ~`:180+`)
  already routes window-opens, `tick_network`, `drain_worker_messages` on each async turn, *distinct from*
  the input-driven synchronous `process_pending_actions` call in `content/event_handlers.rs:174/:389`. The
  comment at `content/navigation.rs:668` names "the two pumps (`process_pending_actions` and the async
  `run_event_loop`)". **The deferred traversal apply is naturally scheduled as a task the async pump drains**
  — a clean realization of the navigation-and-traversal task source.
- **App mode has NO async pump.** `process_pending_navigation` is called **synchronously** right after event
  dispatch in `app/events.rs:89/:131` (`handle_click`/`handle_keyboard`); legacy inline app mode is purely
  input-driven with no `run_event_loop` equivalent. **There is no existing task source to queue the deferred
  apply onto.** This is the **app-mode-has-no-async-pump tension** — flagged as Q-SCHED. Candidate
  resolutions for plan-review (do NOT pre-decide): (i) app-mode drains its own traversal queue at the *end*
  of the same input handler, after Phase 1 completes — a *degenerate* two-phase (still correct for
  single-traversable because the ordering guarantee, not real deferral, is what §12 needs); (ii) give app
  mode a minimal post-input queue-drain step; (iii) accept a documented app-mode fidelity gap and fence it.
  The philosophy lens (Ideal over pragmatic + One-issue-one-way) leans (i): **the shared queue primitive
  enforces the Phase-1-before-Phase-2 *ordering* in both shells; the shells differ only in *when* Phase 2's
  drain is pumped** (async turn vs end-of-input-handler), not in the primitive. Ratify in plan-review.

### 4.4 The reentrancy guard + what gets retired (skeleton 3 + 4)

While a traversal apply runs, set `running_nested_apply_history_step = true`; a reentrant nav-mutating
message (the SW-fetch synchronous message pump, `content/navigation.rs:770–776`) that arrives mid-apply is
**serialized onto the queue** rather than mutating `entries`/the cursor under the held peek. This makes
peek→commit atomic **by construction**, so:

- **Retire the `commit_index` `debug_assert`** (`navigation.rs:256–261`) — the reentrant-drain case it
  backstops is now structurally impossible (the queue serializes the reentrant mutation). `commit_index` may
  keep a plain in-range check, but the "entries mutated between peek and commit" assertion documenting *this
  deferred slot* is removed.
- **Retire the peek-then-commit split as a reentrancy workaround** where it existed *only* to defend against
  the reentrant window. Peek-then-commit is *also* the atomic-on-load-failure mechanism (a failed load must
  not move the cursor — `handle_history_action` doc `:764–769`); **that** role stays. Plan-review must
  separate the two roles (Q-PEEK, §7): retire the reentrancy-workaround framing, keep the load-atomicity
  behavior (possibly re-expressed now that the queue guarantees no concurrent mutation).

### 4.5 Invariants of the traversal-queue primitive (the load-bearing contract both shells enforce)

The phase-separation's correctness rests on three invariants the shared queue primitive (§4.1) enforces **structurally**, not by per-shell assertion. Named so implementation cannot silently violate them, and so §2's (a)/(b)/(d)/(e) pairwise cells have a concrete home:

- **I1 — reconciliation-point ordering (axis a × d).** All Phase-1 synchronous in-task writes to `NavigationController.entries` + `index` (`pushState`/`replaceState`/`location.*`) complete **before** any Phase-2 traversal apply reads them. **Content mode:** structural by construction — the async pump exposes the deferred apply only on a *later* turn. **App mode (option i):** enforced by a **call-ordering discipline** — the shell drains Phase 2 at end-of-input-handler, *strictly after* Phase 1 in the same handler; there is no task boundary, so this leg is a **sequencing contract the shell must honor**, not a by-construction property of the primitive (the F1 residual the Q-SCHED resolution accepts). The shared primitive gives both shells a single drain-then-apply shape, so the app-mode contract is a one-line sequencing invariant, not a per-shell re-derivation.
- **I2 — partition rule (axis a × d).** The single issue-ordered VM `pending_history` FIFO (mixing sync updates + traversals) is partitioned "sync-in-task / traversal-deferred" — but the entry-list state a deferred traversal resolves `targetStepIndex` against is a **documented design decision, not a naive "all sync first."** ⚠ **Spec note (webref-verified §7.4.2.3.3 / §7.4.1.3, Step-4.5 correction):** `pushState`/`replaceState`/`location.*` do **NOT** synchronously mutate the traversable's entries/current-step SoT — the synchronous step only bumps the History-object best-guess index + active entry, then *appends* the real entries/step mutation as a **`finalize a same-document navigation`** step (with a race-guard, §7.4.2.3.3 step 2) that runs **on** the traversal queue (step 1 assert). So the spec model is a **queue reconciliation with "jump the queue" tagging** (§7.4.1.3), **not** a synchronous Phase-1 that fully lands first — the earlier draft's "`back()` resolves against the post-`/b` list = the spec ordering" was **spec-incorrect** and is withdrawn. **elidex's decision (Slice-1 as-built):** ship a **tagged one-queue, issue-order-preserving** model (§7.4.1.3): the sync *prefix* before the first same-turn traversal applies in Phase 1; from the first traversal onward **every** step — including a *trailing* sync — defers onto the one tagged queue (`PendingHistoryStep::{Traversal, SyncUpdate}`) in **issue order**, so a trailing `pushState` is **never hoisted ahead of** an earlier same-turn `back()` (matching the spec-robust point that `back()` does not resolve against a later-issued entry). Spec-equivalent for the common case (a `pushState` and a `back()` in **separate** turns) *and* issue-order-preserving for the straddle; the **residual bounded divergence** is only the *exact* finalize race-guard / jump-the-queue reconciliation semantics (still simplified vs §7.4.2.3.3), which Slice-1's conformance test pins. The **exact straddle outcome is NOT asserted here** (spec-intricate); Slice 1/2 **webref-pins it and adds a conformance test documenting whichever behavior ships** (supported-surface testing — the divergence is *pinned, not silent*). The full queue-reconciliation is the same §7.4.1.3 tagged-queue machinery fenced to B1 (`#11-sync-navigation-steps-queue-tagging`). Design rule the partition MUST honor: **preserve issue order — never reorder a sync update ahead of a traversal issued before it**; the single FIFO stays the sole ordering SoT (axis d).
- **I3 — guard bracket (axis b × e).** `running_nested_apply_history_step` is set **before the peek** (`peek_back`/`peek_forward`/`peek_go`) and cleared **after the commit** (`commit_index`), covering the *entire* peek→commit window. A reentrant nav-mutating message (SW-pump) arriving inside the bracket is serialized onto the queue, not applied under the held peek — making peek→commit atomic **by construction** (retiring the `commit_index` `debug_assert`). A guard set after the peek or cleared before the commit leaves the race reachable and is a bug. **Liveness (F3 residual):** a message serialized *during* an apply must be **eventually drained** — the drain loop MUST re-check the queue for items enqueued mid-apply before returning (in app-mode's end-of-handler drain there is no later async turn, so a stranded item would otherwise wait until the next input event). Slice 4 pins the re-check-until-empty invariant.

**OO→ECS / layer map** (reviewer verification without re-deriving §4): spec *session history traversal queue* → cooperative deferred task on elidex's single-writer event loop (NOT an OS thread — CLAUDE.md Concurrency-by-ownership); spec *running nested apply history step* boolean → I3 serialization guard; spec *traversable* → browsing-context/session-level navigation state (side-store exception (b), not a per-entity ECS component — `#11-browsing-context-state-ecs-components` owns any wholesale migration). Crate placement: `elidex-navigation` (queue + guard + `NavigationController` + `resolve_traversal`/`TraversalKind`/`peek_*`/`commit_index`/`handle_history_action`/`traverse_to`) = engine-agnostic; the shells (`content/navigation.rs`, `app/navigation.rs`) own the Phase-1/Phase-2 drain; `vm/host/navigation.rs` staging (`pending_history`/`pending_navigation`) **unchanged** (Q-VM-MODEL = shell-drain-only).

---

## §5 Decomposition — umbrella + slices (each its own `/elidex-plan-review` + PR)

Per the edge-dense rule this lands as an **umbrella plan + narrowly-scoped slices**, NOT one PR. Proposed
ordering (each slice terminal under the approved umbrella once its own plan-review passes):

- **Slice 0 — SKIPPED (assessed 2026-07-13, PM decision).** The two-shell drains (`content/navigation.rs:529`, `app/navigation.rs:34`) were assessed for a standalone prereq unification and it was **declined**. Recorded reason: *the extractable seam is ~15 lines of 3-phase orchestration whose pivot (the supersede `return true`, content `:593` / app `:73`) is **deleted** by the phase-separation; the substantive bodies (pipeline rebuild + frame shipping) are irreducibly shell-specific across `ContentState`/`InteractiveState` and cannot cross into `elidex-navigation`; and the cited 1000-line pressure is **misattributed** — the drain is a small fraction of the 904 LoC, so real relief is an orthogonal `handle_navigate`/`same_document_step` carve, not this split.* The two-shell unification is **folded into Slice 1** (the shared drain-coordinator is born in its final phase-separated shape for both shells to adopt in Slice 2/3 — One-issue-one-way satisfied without a throwaway behavior-neutral extraction of about-to-be-deleted control flow). ⚠ **Orthogonal 1000-carve note:** if Slice 2's additions push `content/navigation.rs` past 1000 LoC, the touch-time split is the `handle_navigate`/`same_document_step`→sibling-module carve (a *separate* standalone prereq at that point), NOT the drain-unification (which does not relieve it).
- **Slice 1 — the shared drain-coordinator + traversal queue + nested-apply boolean in `elidex-navigation`** (born phase-separated; **no shell wired yet**): introduce the traversal queue on/near `NavigationController` AND the shared drain-coordinator (phase-partition + queue + the §4.5 I1/I2/I3 invariants) in its **final** shape — a **host-trait-parameterized** primitive (window-open / history-action / navigation / ship-frame hooks) both shells will adopt, so `ContentState`/`InteractiveState`/pipeline stay *behind the trait* and never cross the crate boundary. Unit-pinned in isolation. Retires nothing yet; pure additive substrate that Slices 2/3 make each shell drive.
- **Slice 2 — content-mode phase-separation:** move `Back/Forward/Go` out of the synchronous FIFO into the
  queue, schedule the deferred apply on the async `run_event_loop` pump, remove the `:593` supersede-return.
  Flip the `content_history_drain_tests.rs` cases that pin synchronous supersede (§8).
  ⚠ **CARRY (Slice-1 `/code-review` high, 2026-07-13) — traversal-vs-navigation supersede is under-analyzed.**
  The coordinator runs Phase-1c `location.*` navigation (`handle_navigation`) *unconditionally*, then applies
  the deferred traversal in Phase 2 — so a same-turn `history.back(); location.href='/b'` **loads `/b` over the
  network + ships its frame, then rebuilds to the back target**, a wasted load + visible flash, and lands on the
  *traversal* target even though `location.href` was issued **later** (issue-order last-wins would land on `/b`).
  This is the §6-E7 "nav in task N, traversal in task N+1" position, but E7 did not walk the `location.*`-channel
  supersede specifically (I2 only partitions the `pending_history` FIFO; `pending_navigation` is a separate
  channel Phase-1c always drains first). **Slice-2 plan-review MUST resolve**: does a same-turn deferred traversal
  suppress a *later-issued* Phase-1c navigation (or vice-versa) — webref-pin `location.*` navigate (§7.4.2.2) vs
  traverse (§7.4.3) same-turn ordering + add the conformance test (§4.5 I2 already fences the exact straddle
  outcome as Slice-1/2 conformance territory). NOT a Slice-1 substrate defect (substrate faithfully implements
  the plan's phase model; the *phase model for the nav-vs-traversal case* is what needs ratifying).
- **Slice 3 — app-mode phase-separation:** resolve Q-SCHED (§4.3) — apply the ratified app-mode scheduling
  (likely end-of-input-handler queue drain), remove the `:73` supersede-return. One-issue-one-way close:
  both shells now drive the shared queue. **Landing-proximity constraint (axis c):** Slice 2 and Slice 3 SHOULD land in close succession — or Slice 2's observable supersede-removal be gated behind Slice 3 — so the content-queued / app-synchronous fork (§2 axis (c) failure mode) is not left open across unrelated PRs.
- **Slice 4 — reentrancy guard + retirements:** wire `running_nested_apply_history_step`, serialize the
  SW-pump reentrant vector, retire the `commit_index` `debug_assert` and the peek-then-commit
  reentrancy-workaround framing (keep load-atomicity). Close #448-issue.
  ⚠ **CARRY (Slice-1 `/code-review` high, 2026-07-13) — bound the re-check-until-empty drain loop.**
  `drain_traversal_queue`'s `while let Some(step) = pop_next()` (the §4.5-I3 eventual-drain) is **unbounded**: a
  wired host whose `apply_traversal` re-enqueues a traversal on *every* call never terminates → the single-writer
  renderer main thread hangs. Inert in Slice 1 (no wired re-enqueue source; the unit test bounds via a one-shot
  `take()`), so NOT a Slice-1 defect — but Slice 4 owns the guard semantics and **MUST make termination
  structural**: the spec's "running nested apply history step" boolean should *gate* re-entrant applies (serialize
  the finite set of externally-arrived messages), not merely be observational (`is_applying`) over an unbounded
  loop. Pin a liveness/termination invariant (a real re-enqueue source produces a *finite* per-turn set; a
  drain that could loop forever on a pathological source is a bug) with a test.

Do **not** bundle. Slice ordering keeps each PR shippable (Slice 1 is inert substrate; Slices 2/3 each leave
a working shell; Slice 4 removes the scaffolding once the queue guarantees hold).

---

## §6 What it subsumes / closes

| Item | Code site | How the new model closes it |
|---|---|---|
| **#396 root** — sync drain conflates §7.4.4 in-task update with §7.4.3 queued traversal | `content/navigation.rs:583–592` collapsed-boundary comment | Phase-separation (§4.2) gives the traversal its own task after sync updates land — the split §7.4.6.1 step 12 mandates |
| **#259** — multi-action FIFO in one turn | `vm/host/navigation.rs:159` FIFO + `content/navigation.rs:570–595` | Phase 1 replays *all* synchronous updates (no supersede-return truncation); the traversal defers, so a `pushState; pushState; back()` turn keeps both pushes |
| **#283** — fall-through onto freshly-rebuilt runtime | `content/navigation.rs:576–582` (the `return true` guarding the fresh-runtime `location.*` drain) | With the traversal deferred to a later task, the fresh page's own `pending_navigation` is drained on *its* turn, not stranded/mis-read on the traversing turn — the supersede-return workaround is removed, not reinforced |
| **#448-issue** — SW-pump held-peek reentrancy | `content/navigation.rs:770–776` + `commit_index` assert `:255–261` | The nested-apply boolean serializes the reentrant SW-fetch message; peek→commit atomic by construction (§4.4) |
| **chrome-button traversal atomicity** | `app/navigation.rs handle_chrome_action :596`, `traverse_to :514`; content chrome path | Toolbar Back/Forward already routes through the shared `traverse_to`/peek-then-commit (`app/navigation.rs:608–624`); with the queue, a chrome traversal and a same-turn JS sync update phase-separate identically (One-issue-one-way with the JS API) |
| **E7** — traversal + nav same-turn race | 5c §4.3 (`:463–465`) residual | The task boundary *is* the resolution: sync update in task N, traversal apply in task N+1; §12.4 update-only handles the "sync already moved the active entry" case |

---

## §7 Open questions for `/elidex-plan-review` (decision-level)

**RESOLVED by `/elidex-plan-review` 2026-07-13** (rationale = the numbered questions below): **Q-OWNER** = engine-agnostic (`elidex-navigation`, near `NavigationController`) — all 5 axes concurred. **Q-SCHED** = option (i) end-of-input-handler drain (ordering-not-parallelism; shared primitive; One-issue-one-way) — **re-eval trigger: revisit app-mode scheduling at the B1 multi-navigable-fan-out landing** (an end-of-handler drain that is not a real later task may violate §7.4.6.1 step-12 unload sequencing once multi-navigable lands). **Q-VM-MODEL** = shell-drain-only (VM staging unchanged). **Q-SYNC-FINALIZE** = **one traversal queue with tagged step-sets** (§7.4.1.3 *Centralized modifications of session history* defines both *append … traversal steps* and *append … synchronous navigation steps* onto the SAME queue's algorithm set; sync-nav steps are tagged to "jump the queue") — the **cross-navigable finalize ordering** is observationally inert for a single top-level traversable (no sibling navigable to sequence against) → **fence to B1**, tracked as new slot **`#11-sync-navigation-steps-queue-tagging`**. ⚠ **Corrected (Step 4.5):** the tagging is NOT fully inert single-traversable — the same-turn **straddle** (`pushState; back(); pushState`, I2) IS observable; elidex ships the **tagged one-queue, issue-order-preserving model** in-scope (Slice-1 as-built — trailing sync deferred after the traversal, never hoisted; residual divergence = the exact finalize race-guard / jump-the-queue semantics) with a **Slice-1 conformance test** pinning the outcome (the slot covers the full spec reconciliation — cross-navigable ordering AND the exact single-traversable straddle semantics). (Why-deferred: full §7.4.1.3 queue-reconciliation = B1 tagged-queue machinery; single-traversable divergence is bounded + Slice-1-pinned; Trigger: B1 multi-navigable landing OR a straddle-fidelity WPT/site; Re-eval date 2026-10-31.) **Q-PEEK** = retire the reentrancy-workaround framing (I3 makes peek→commit atomic by construction), keep the load-atomicity role (a failed load must not move the cursor); `commit_index` keeps a plain in-range check. **Q-FENCE** = ratified (legitimate cross-PR boundary, B1-gated).

1. **Q-OWNER — queue owner placement.** Engine-agnostic (`elidex-navigation`, near `NavigationController`)
   vs shell. *Lean: engine-agnostic* (both shells share it; classification/cursor invariants already live
   there). Ratify that `NavigationController` is the right traversable-proxy home, or whether a distinct
   `Traversable` wrapper should own both the controller and the queue.
2. **Q-SCHED — app-mode scheduling mechanism.** App mode has no async pump (`app/events.rs:89/:131`,
   synchronous post-dispatch). Ratify (i) end-of-input-handler queue drain [lean], (ii) a new minimal
   app-mode pump, or (iii) a fenced app-mode fidelity gap. This is the single biggest design decision.
3. **Q-VM-MODEL — does the VM staging model change, or only the shell drain?** `pending_history` FIFO +
   `pending_navigation` last-wins (`vm/host/navigation.rs:143/:159`) already separate synchronous updates
   from traversals *as data*; the re-timing may be **shell-drain-only** (VM staging unchanged). *Lean:
   shell-only* — the VM correctly stages a turn's intents; the phase-separation is about *when the shell
   applies them*, not how the VM buffers them. Confirm no VM change is needed (avoids duplicating the queue
   in two layers, axis d).
4. **Q-SYNC-FINALIZE — model §7.4.4 step-13 *synchronous navigation steps* as a distinct third queue class?**
   The finalize-a-same-document-navigation tail is *also* queued onto the traversable. For a single
   traversable with no cross-doc unload to sequence against, folding it into the synchronous drain is
   defensible. Decide: model now (full fidelity) vs fence to B1 (when cross-navigable finalize ordering
   actually matters).
5. **Q-PEEK — separate the two roles of peek-then-commit.** Retire the *reentrancy-workaround* framing (the
   queue now guarantees no concurrent mutation) while keeping the *load-atomicity* behavior (a failed load
   must not move the cursor). Confirm the split and whether `commit_index` keeps a plain in-range check.
6. **Q-FENCE — ratify the scope fence.** Confirm the single-top-level-traversable scope and the B1 deferral
   of §7.4.6.1 steps 3/4/6/7 + per-navigable global-task fan-out
   (`docs/plans/2026-06-agent-scoped-ecsdom-world.md` §6.3). Confirm no speculative multi-navigable
   abstraction is built now.

---

## §8 Test strategy

**What pins the phase-separation.** New tests must assert *ordering across a task boundary*, which the
current suite cannot express (it drives one synchronous `process_pending_actions` and asserts the collapsed
result).

- **Existing suite `crates/shell/elidex-shell/src/content_history_drain_tests.rs` (534 LoC, 12 tests) pins
  the *current synchronous* behavior** and its module doc (`:3`) states it "pins the `process_pending_actions`
  drain reorder (WHATWG HTML §7.4.4)." The tests that assert **traversal-supersede-in-one-pass** (a `back()`
  discarding a trailing same-turn intent — the `:593` `return true` behavior, e.g. around `:446–:461`) will
  **flip**: after phase-separation a trailing *synchronous* `pushState` is preserved (Phase 1) and the
  traversal applies in Phase 2. Each flipped test must be re-anchored to the new task-boundary expectation
  with a §7.4.6.1-step-12 cite, not merely deleted (Supported-surface testing: the regression it guards
  changes shape, it does not disappear).
- **New phase-separation tests (per shell):** `pushState('/a'); history.back()` in one turn ⇒ `/a` committed
  to `NavigationController` *and* the traversal applies against the updated entry list (asserting Phase 1
  landed before Phase 2). `history.back(); pushState('/x')` ⇒ both observed in the spec order. `go(0)` reload
  still `Rebuild`. These pin axis (a).
- **Reentrancy test (Slice 4):** drive a reentrant nav-mutating message during a traversal apply and assert
  the cursor is not staled (the `commit_index` assert no longer reachable) — pins axis (b). If the SW-pump
  vector remains unreachable in test today (the SW controller path is "dead" per `:775`), assert the
  serialization at the queue level directly rather than through the SW path.
- **Two-shell parity:** the same scenario table runs against both `content_history_drain_tests.rs` and the
  app-mode equivalent (`app_fragment_nav_tests.rs` neighborhood), pinning axis (c) — One-issue-one-way is
  test-enforced, not just asserted.
- **Cursor atomicity regression (axis e):** keep the failed-load-does-not-move-cursor tests (peek-then-commit
  load-atomicity role survives Q-PEEK); only the reentrancy-workaround assertion is retired.
