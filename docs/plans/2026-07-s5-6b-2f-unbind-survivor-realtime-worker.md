# S5-6b Stage 2f (keystone) — Unbind survivor set: realtime + worker persistence across per-turn unbind

Per-PR-slice plan-memo under the S5 umbrella (`docs/plans/2026-06-s5-flip-boa-deletion-umbrella.md`)
and the S5-6 flip memo (`docs/plans/2026-07-s5-6-flip-boa-deletion.md` — §3.4 rows **B14** (`:375`) /
**B15** (`:376`), §4.1 the batch-bind bracket model + the E2 cross-batch wrapper-identity edge
(`:461-465`), §8 acceptance-gate item 5 (`:922`)). This memo does **not** re-derive the umbrella's
bracket model (§4.1) or the E4 strangler rule (§8) — it references them and scopes only the
**unbind-teardown / realtime-pump / worker-teardown** surface that rows B14+B15 depend on.

> **Gate**: `/elidex-plan-review` (5-agent) BEFORE any impl. This is a **keystone** — it crosses
> ≥3 intersecting invariant axes (per-turn-bind lifetime × cross-DOM-aliasing safety × broker/worker
> resource lifetime × WebIDL identity), and there is no off-the-shelf canonical algorithm for
> "which unbind actions are per-turn vs per-document". CLAUDE.md "Edge-dense work = multi-PR program
> + 実装前 plan-review 必須". Like the bound-safe-dispatch keystone, it is solved **inside** the flip
> (it is a precondition for a supported surface — WS/SSE/Workers — to keep working), not carved out.
> §6 carries the open design questions reviewers must scrutinise (§6.1 is CRITICAL: self-contained vs
> agent-scoped-EcsDom-gated); §7 is the sub-commit split; §8 is the carve list.

All cites grep-verified against worktree `s5-6b-flip` HEAD **`7dc865ac`** (2026-07-12). NOTE: the
S5-6 flip memo's own B15 row cites (`event_loop.rs:276-277`, `vm_api.rs:422-450`) are **stale** —
the branch has advanced through stages 2a–2f and those lines are now `event_loop.rs:324-325` and
`vm_api.rs:472-495` (+ `host_data/mod.rs:1869`); §4 below carries the refreshed cites. Spec anchors
webref-verified 2026-07-12 (`.claude/tools/webref`).

---

## Decision on record — SPLIT `unbind` into a per-TURN re-establishment and a per-DOCUMENT teardown; PROMOTE realtime + worker state into the unbind-survivor set

The flip's batch-bind model opens **many** `with_bound` brackets per event-loop turn (one per
per-turn deliver helper — `PipelineResult::dispatch_event` / `drain_timers` /
`deliver_records_and_drain` / `deliver_layout_observations` / `drain_worker_messages` /
`deliver_history_step_events` / `deliver_media_query_changes` / …, `crates/shell/elidex-shell/src/lib.rs:272-468`).
Each bracket's RAII `UnbindGuard` calls `unbind()` at bracket-end
(`crates/script/elidex-js/src/engine.rs:357` `with_bound`; guard `:363-368`, `drop → self.0.unbind()`
at `:366`). Today `Vm::unbind` (`crates/script/elidex-js/src/vm/vm_api.rs:454`) **force-closes every
WebSocket/EventSource connection** (`:472-495`, driving `HostData::drain_realtime_for_unbind`
`crates/script/elidex-js/src/vm/host_data/mod.rs:1869`) **and terminates every dedicated worker**
(`:501` → `teardown_workers` `crates/script/elidex-js/src/vm/host/worker.rs:206`). Under the flip
that is **destruction of live connections and workers on every turn** — a **loss-of-function**
regression on a supported surface, strictly worse than the E2 identity delta (§4.1).

The **first-principles ideal** (CLAUDE.md "Ideal over pragmatic"): a per-turn unbind is **not** a
document-teardown event — it is the *re-establishment boundary* of a bound view over **persistent
session resources** that outlive the turn. The clean model names two distinct operations:

- **`Vm::unbind` (per-turn, keeps firing every bracket)** = *un-bind the pointers + drop only what is
  genuinely cross-DOM-aliasing-unsafe to carry into a possible rebind to a different `EcsDom`*
  (non-Node wrapper caches, live collections, IDB txn rollback, dispatcher teardown). It does **not**
  touch realtime connections or workers.
- **`Vm::teardown_document` (NEW, per-document, fires only at pipeline replacement / engine drop)** =
  *release the browsing-context-scoped resources* — force-close WS/SSE conns + terminate workers +
  uncache their wrappers. This is the WHATWG HTML §10.2.4 "terminate a worker" moment (document
  unloading), not a per-turn event.

Realtime side-tables + the worker registry are **promoted into the unbind-survivor set** — the same
class as `window_entity` (`host_data/mod.rs:132`, retained across unbind: `vm_api.rs:575`) and the
primary Node wrapper (`vm_api.rs:622-623` `wrapper_store.retain(kind == Node)`, invalidated-not-
dropped via `bind_epoch`). Per-turn unbind leaves them intact; next turn's `tick_network` re-delivers
against the **same** `conn_id`s / worker entities; the broker I/O thread (which outlives a bind cycle
while the `NetworkHandle` is held) never sees a Close.

This is rejected-alternative-explicit: a **flag-hack** ("`unbind(teardown: bool)`" that special-cases
the teardown inside the per-turn path) is NOT chosen — it moves the decision surface into every
caller ("which bool do I pass?") rather than eliminating it, violating "One issue, one way". A clean
method split gives each caller exactly one correct call by construction.

---

## §1 Scope — what 2f-keystone delivers

| Part | Surface | Memo row |
|---|---|---|
| A | Split `unbind` → per-turn `unbind` (realtime/worker teardown REMOVED) + new per-document `teardown_document` | B15 |
| B | Promote `websocket_states` / `ws_conn_to_object` / `event_source_states` / `sse_conn_to_object` / the two conn-id counters (`HostData`) **+** `worker_entities` / `worker_registry` (`VmInner`) into the unbind-survivor set | B15 / B14 |
| C | B15 convergence: the boa `shutdown_all_realtime()` / `shutdown_all_workers()` explicit calls converge onto `teardown_document` at the per-document boundaries + an engine-Drop backstop | B15 |
| D | B14 follow-on (shell consumer): wire VM `tick_network` bracketed per-turn (new `PipelineResult::tick_network` (NEW) helper mirroring the other deliver helpers), delete the boa realtime pump | B14 |

**Explicitly out of scope** (carved, §8): the general cross-batch `[SameObject]` wrapper-identity fix
for getter-attributes (E2 / agent-scoped EcsDom); the full WAAPI surface (B22 / S5-7); any change to
the same-DOM-safe cross-DOM clears that per-turn unbind legitimately performs.

**Layering-check (CLAUDE.md Layering mandate).** The moved surface (`teardown_workers` /
`drain_realtime_for_unbind`) and the new `teardown_document` / `tick_network` wiring are all
**VM-lifecycle + marshalling** — `ObjectId↔conn_id` routing, `WorkerHandle` / `NetworkHandle` resource
teardown, `bind`/`unbind` bracket management — with **no** engine-independent-crate-eligible
DOM/form/selector/CSSOM algorithm in the moved or added code. So the usual crate-mapping table is
intentionally **N/A** for this keystone (nothing to route to `elidex-dom-api` / `elidex-form` /
`elidex-css`).

**Touch-time-split note (CLAUDE.md 1000-line debt).** Both touched files are >1000 LoC — `vm_api.rs`
(≈1255) and `host_data/mod.rs` (≈1997). The discipline is *considered* and does **not fire**: this
touch is net-neutral — Part A is a **verbatim MOVE** of two blocks within `vm_api.rs` (no new
algorithm), and Part B is **removal-of-clears** — so there is no substantive >50 LoC growth (the Axis-5
review backstop trigger). The standing `host_data/mod.rs` decomposition debt is already tracked by the
existing carve `#11-host-data-full-decomposition` (prereq split PR #455 note); this keystone does not
enlarge it.

---

## §2 Coupled invariants

The keystone sits at the intersection of four invariant axes. Each pairwise intersection is named so
plan-review can check them independently.

- **per-turn-bind lifetime × broker/worker resource lifetime** — the broker per-conn I/O thread and
  the worker OS threads are **session-scoped**, not turn-scoped. Their liveness is bounded by the
  `NetworkHandle` / `WorkerHandle` Drop, which the VM holds across a bind cycle. The current unbind
  eagerly bounds them to the *bind cycle* (the `vm_api.rs:461-467` comment's stated rationale). Under
  the flip the bind cycle is **one turn**, so eager bounding = per-turn destruction. The correct
  bound is the **document lifetime**; `teardown_document` restores the eager-bounding intent at the
  right granularity (and engine-Drop is the RAII backstop).

- **cross-DOM-aliasing safety × per-turn vs per-document** — the aggressive clears in `unbind`
  (non-Node `wrapper_store.retain` `vm_api.rs:622`, `live_collection_states.clear` `:595`, DnD/touch
  `:637-639`, IDB `:651-664`, Cache/SW `:671-696`) exist because **two `EcsDom::new()` worlds share
  entity-index space** (lesson #195) — a retained wrapper from doc1 would alias a live entity in
  doc2. **That hazard only materialises at a document boundary** (navigation / rebuild spawns a fresh
  `EcsDom`). A per-turn unbind re-binds the **same** `session`/`dom`/`document` (it is a same-DOM
  re-establishment). So the cross-DOM clears are *conservatively* correct today but only *load-bearing*
  at per-document boundaries. The keystone does **not** move those clears (that is the E2/agent-scoped
  work, §6.1) — it moves **only** the realtime+worker teardown, which is loss-of-function, not
  identity-delta. Framing this axis prevents scope-creep into E2.

- **assume-bound deliver × survivor-set state** — `tick_network` (`fetch_tick.rs:73` `VmInner`,
  `vm_api.rs:1049` `Vm` wrapper; trait decl `crates/script/elidex-script-session/src/engine.rs:221`)
  is an **assume-bound** trait method: it reads the bound world and delivers WS/SSE events at the
  wrappers keyed in the (now-surviving) side-tables. For B14's bracketed per-turn wiring to deliver
  correctly, the side-tables MUST survive the *previous* turn's unbind. Axis intersection: survivor
  promotion (Part B) is the **precondition** for B14's bracket to be sound; B14 without Part B
  delivers into cleared tables (silent drop).

- **WebIDL identity × resource survival** — WS/SSE/Worker JS objects are **constructor results**
  (`new WebSocket()` / `new EventSource()` / `new Worker()`) held **directly** by JS, NOT
  `[SameObject]` getter-attributes re-resolved per access. Their identity is preserved by the VM
  object heap (which persists across per-turn unbind — unbind clears *pointers + caches*, it does not
  drop the VM or its heap) plus the GC keepalive that roots a live-connection wrapper. This axis is
  what makes the keystone **self-contained** (§6.1) — identity rides the heap, not the wrapper_cache,
  so the E2 cross-batch getter problem does not bind here.

**ECS-native side-store check (CLAUDE.md "Side-store→component 判定ルール").** Do the promoted
side-tables belong on ECS components instead? No — they hit both documented exceptions:
- `websocket_states` / `ws_conn_to_object` / `event_source_states` / `sse_conn_to_object` value/key on
  **`ObjectId`** (`host_data/mod.rs:502,508,524,528`) — a **per-VM identity handle** (exception (a));
  and `WebSocketState` / `EventSourceState` carry the per-VM wrapper identity + broker routing, not a
  per-entity fact. `worker_entities: HashMap<WorkerId, Entity>` (`vm/mod.rs:2707`) is a routing index
  keyed by the broker-side `WorkerId`, and `worker_registry` (`vm/mod.rs:2697`) owns the
  **`WorkerHandle`** (an OS-thread + channel handle — **shared cross-cutting session resource**,
  exception (b)), not a `Send+Sync` per-entity value.
- The **unbind-survivor set** is the *correct* ECS-native mechanism here, and it already exists:
  `window_entity` + its `EventListeners` component survive (`vm_api.rs:216-227`), the primary Node
  wrapper survives via `bind_epoch` invalidation (`host_data/mod.rs:153,1266`; retain at
  `vm_api.rs:622-623`). Promoting realtime+worker state is *joining an existing survivor category*,
  not inventing a new side-store — it is the ECS-native "this state's lifetime is the document, not
  the turn" statement.

---

## §3 Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §10.2.4 "Processing model" — "terminate a worker" algorithm (#terminate-a-worker) | Run at document unload ("when a worker stops being actively needed"), NOT per event-loop turn | per-document `teardown_document` (NOT per-turn `unbind`) | `Vm::teardown_document` (NEW) / `teardown_workers` `worker.rs:206` (MOVED off `Vm::unbind` `vm_api.rs:501`) | ✓ (2 branches: still-needed worker → keep alive across turn; document unload → terminate) | no |
| WHATWG WebSockets Standard §4 "Feedback from the protocol" (#eventdef-websocket-close) — closest normative anchor; connection-teardown-on-Document-destruction is not separately §-numbered, so cite the close-event anchor + note | Force-close on Document destruction / `.close()` / peer close — never on a turn boundary | per-document `teardown_document` force-close (per-turn `unbind` sends NO Close) | `drain_realtime_for_unbind` `host_data/mod.rs:1869` (MOVED off `Vm::unbind` `vm_api.rs:472-495`) | ✓ (2 branches: live conn → survive turn; document unload → `WebSocketClose`) | no |
| WHATWG HTML §9.2.2 "The EventSource interface" (#eventsource) — closest verified anchor; the remote-event-task-source close-on-destroy is not separately §-numbered, so cite the interface anchor + note | Connection persists until `.close()` / GC / document unload — never on a turn boundary | per-document `teardown_document` force-close (same shape as WebSocket) | `drain_realtime_for_unbind` `host_data/mod.rs:1869` (SSE half, MOVED off `Vm::unbind` `vm_api.rs:472-495`) | ✓ (2 branches: live conn → survive turn; document unload → `EventSourceClose`) | no |
| WHATWG HTML §8.1.4.4 "Calling scripts" — "clean up after running script" (#clean-up-after-running-script) | The bracket is a script-task checkpoint boundary, NOT a document boundary — resource teardown does not belong at bracket-end | per-turn `unbind` boundary (every `with_bound` bracket-end) | `with_bound` `engine.rs:357` (RAII `UnbindGuard::drop → unbind()` `:366`) | ✓ (per-turn boundary fires the checkpoint + pointer-unbind, never document teardown) | no |

**Spec takeaway**: all three realtime/worker lifetimes are **document-scoped**; none is turn-scoped.
The current per-turn teardown is not merely a performance issue — it is a spec-granularity defect that
the flip's bracket multiplication turns from latent (boa's cheap per-call swaps never tore down) into
active. The split restores spec-correct granularity.

---

## §4 Per-part design rulings

### Part A — split `unbind` into per-turn + per-document (B15)

**Ruling.** Remove the realtime teardown (`vm_api.rs:472-495`) and `teardown_workers()` call
(`:501`) from `Vm::unbind`. Move them verbatim into a new `Vm::teardown_document(&mut self)`. The
per-turn `unbind` retains **everything else** it does today (classified below). `teardown_document`
runs the realtime-close + worker-terminate **while still bound** (both need the live `NetworkHandle` /
worker registry + wrapper access), then calls `unbind()` itself as its final step — so
`teardown_document` = "close document resources, then unbind".

**Classification of `unbind`'s current actions (per-turn = STAY, per-document = MOVE).** Read the
full body `vm_api.rs:454-708`:

| Action | Site | Disposition | Why |
|---|---|---|---|
| Realtime close (WS/SSE `Close` sends + side-table clear + counter reset) | `:472-495` + `drain_realtime_for_unbind` `host_data/mod.rs:1869-1878` | **MOVE** → `teardown_document` | loss-of-function if per-turn; document-scoped lifetime |
| `teardown_workers()` (terminate_all + wrapper uncache) | `:501` → `worker.rs:206-217` | **MOVE** → `teardown_document` | §10.2.4; document-scoped |
| Dispatcher teardown (`clear_mutation_dispatcher`, transient-observer scrub, live-range/node-iterator/tree-walker/selection clears) | `:517-554` | **STAY** (per-turn) | paired with per-bracket `bind`'s dispatcher install (`vm_api.rs:314`); bind/unbind dispatcher install is a **strict per-bracket pair** — must fire every bracket or the next `bind`'s `debug_assert!(displaced.is_none())` (`vm_api.rs:315-319`) trips |
| `entity_bits = 0` reset on globalThis HostObject | `:580-585` | **STAY** | post-unbind null-safety for `entity_from_this` consumers |
| `live_collection_states.clear` | `:595` | **STAY** | cross-DOM aliasing (safe within same-DOM turn, but harmless to keep clearing; NOT in scope to change — §6.1) |
| non-Node `wrapper_store.retain(kind==Node)` | `:622-623` | **STAY** | cross-DOM aliasing; the E2 surface (§6.1) — untouched here |
| DnD / touch state clears | `:637-639` | **STAY** | cross-DOM aliasing |
| IDB txn rollback + state clears | `:651-664` | **STAY** | txn rollback is per-bracket-correct: the backend `IdbTransaction` has no Drop-rollback, so leaving it open blocks the next bind's ops (`:645-648` comment). Genuinely per-turn. |
| Cache / SW realm state clears | `:671-696` | **STAY** | per-dispatch-transient + cross-DOM aliasing |

Only the **two realtime/worker blocks MOVE**; everything else is genuinely per-turn (dispatcher
pairing) or cross-DOM-aliasing (same-DOM-safe-to-keep-clearing, but out of scope). This table is the
load-bearing claim plan-review must verify against the full body.

> **Note — the `is_bound()` gate is not the fix.** The current `if hd.is_bound()` gate on the
> realtime drain (`vm_api.rs:475`, `is_bound` at `host_data/mod.rs:1287`) only skips a **never-bound**
> VM (pure-test path). On any real bracket `is_bound()` is true (pointer-based), so teardown runs. The
> gate cannot distinguish per-turn from per-document — only the method split can.

### Part B — promote realtime + worker state into the survivor set (B15/B14)

**Ruling.** After Part A, the six realtime fields (`host_data/mod.rs:502,508,516,524,528,531`) and the
two worker fields (`vm/mod.rs:2697,2707`) are **no longer cleared by per-turn `unbind`** — they simply
persist (nothing clears them). No new field or counter is introduced: promotion = *removal of the
clear*, plus a regression test pinning survival (§7). The conn-id counters (`ws_next_conn_id` /
`sse_next_conn_id`) must persist too (a reset to 0 would collide a re-delivered event against a
recycled id — the same failure class the `bind_epoch` `next_id` comment guards against at
`vm_api.rs:265-273`).

**Asymmetry to note for reviewers**: realtime tables live on **`HostData`**, worker tables on
**`VmInner`** — different structs, same survivor principle. The `HostData` realtime fields survive
because `Vm::unbind` (`vm_api.rs:476`, inside the `#[cfg(feature = "engine")]` block) will no longer
call `HostData::drain_realtime_for_unbind` — the drain is a `Vm::unbind` action, NOT a
`HostData::unbind` action (`HostData::unbind` `host_data/mod.rs:1258-1267` only nulls the
session/dom/document pointers + bumps `bind_epoch`). The `VmInner` worker fields survive because
`Vm::unbind` will no longer call `teardown_workers` (`vm_api.rs:501`). Two edits, one concept.

**Worker `event.target === myWorker` identity — no wrinkle; the review's F1 premise is corrected by
source.** Worker message delivery resolves the worker `Entity → Worker` wrapper via
`hd.get_cached_wrapper(entity)` to seed `event.target` (`worker.rs:133-147`); on a cache miss it
`continue`s and the message is **dropped entirely** (not merely re-wrapped) — so wrapper survival is
load-bearing for *delivery*, not only identity. **Crucially, that wrapper is cached under
`WrapperKind::Node`**: `cache_wrapper(entity, obj)` (worker construction site `worker.rs:398`) and
`get_cached_wrapper(entity)` both key on `WrapperKey::entity(entity, WrapperKind::Node)`
(`host_data/mod.rs:1717-1734`), and `teardown_workers` removes it via the same Node key
(`remove_wrapper` `worker.rs:210-213` → `host_data/mod.rs:1752-1755`). Therefore the general per-turn
retain `wrapper_store.retain(|k, _| k.kind == WrapperKind::Node)` (`vm_api.rs:622-623`) **RETAINS the
Worker wrapper** — it does NOT evict it (the review's F1 premise that `:622` "evicts the non-Node
`Worker` wrapper" is factually incorrect: the Worker wrapper *is* Node-kind). So once Part A moves
`teardown_workers` off per-turn `unbind`, **nothing per-turn removes the Worker wrapper** and
`event.target === myWorker` holds by construction — the Worker wrapper rides the *existing* primary-
Node-wrapper survival mechanism (the same `:622` retain + `bind_epoch` invalidation that keeps the
Window/element wrappers; the Worker HostObject does not snapshot `bind_epoch`, and the entity is
same-DOM-stable under 1-Vm-per-World, so it stays valid).

Consequently **no new `worker_entity → ObjectId` map is introduced** (the review's preferred fix) and
**no widened retain predicate** (the fallback): the Node-kind wrapper cache *already is* the surviving
`entity → ObjectId` map, and adding a parallel worker map would duplicate it — a One-issue-one-way
violation. The surviving-state promotion list for workers is therefore: `worker_entities` +
`worker_registry` (`VmInner`) + the worker entities' *already-Node-kind* `wrapper_store` entries
(retained by `:622`, no code change beyond the Part A teardown move). WS/SSE are symmetric in spirit:
their delivery resolves the target via `ws_conn_to_object[conn_id] → ObjectId` directly, with no
wrapper_store round-trip. The multi-turn `event.target === myWorker` invariant is pinned by a test
(§7 2f-k-b) regardless, so any future change to the Node-kind-caching assumption is caught.

### Part C — B15 convergence (boa explicit calls → `teardown_document`)

**Ruling.** The boa `shutdown_all_realtime()` / `shutdown_all_workers()` explicit calls
(`crates/shell/elidex-shell/src/content/event_loop.rs:324-325` Shutdown arm;
`crates/shell/elidex-shell/src/content/navigation.rs:197` pre-rebuild) converge onto a single
`PipelineResult::teardown_document` (NEW) (bracketless — it manages its own bind/unbind) invoked at exactly
those per-document moments. Plus an **engine-Drop backstop**: `impl Drop for ElidexJsEngine` (or the
owning `PipelineResult`) calls `teardown_document` if it has not already run, so a dropped pipeline
that skipped the explicit call (panic-unwind, iframe teardown path that forgets) still releases its
resources. Idempotent: after the first `teardown_document` the tables are empty, so a second call
(explicit-then-Drop) is a no-op — no double-close (the snapshot lists are empty).

### Part D — B14 follow-on (shell consumer, tick_network wiring)

**Ruling** (sequenced AFTER A+B land — see §7). Once the side-tables survive, add
`PipelineResult::tick_network(&mut self)` mirroring the other bracketed deliver helpers
(`lib.rs:420` `drain_worker_messages` is the exact template — `with_bound` → `engine.tick_network()`
→ `engine.drain_reactions(ctx)`), call it once per event-loop turn in `event_loop.rs`, and **delete
the boa realtime pump** (`event_loop.rs:198-216` `drain_realtime_events` + `dispatch_realtime_events`;
boa defs `crates/script/elidex-js-boa/src/bridge/realtime.rs:222,255`, `runtime/realtime.rs:17`).
`needs_render` comes from the §4.3.8 inclusive-descendants version-delta already wired at
`event_loop.rs:227-236` (stage 2d-2), NOT a per-call bool — the boa pump's `has_js_events` return
(`event_loop.rs:200-214`) dies with it.

---

## §5 Per-document teardown entry-point enumeration

`teardown_document` must fire **exactly once** per document destruction — no leak (a skipped call
leaves the broker I/O thread + worker OS threads alive until process exit), no double-close (a second
call must be a safe no-op). The complete boundary set:

| # | Boundary | Site (HEAD `7dc865ac`) | Wiring |
|---|---|---|---|
| 1 | Content-thread **Shutdown** | `content/event_loop.rs:313-327` (Shutdown arm; boa calls at `:324-325`) | explicit `teardown_document` (after `dispatch_unload_events`, before thread exit) |
| 2 | Address-bar **cross-document Navigate** rebuild | `content/navigation.rs:194-197` (boa `shutdown_all_realtime` at `:197`, pre-`load_document` pipeline replacement) | explicit `teardown_document` on the **outgoing** pipeline before it is replaced |
| 3 | **History-traversal** rebuild (cross-document back/forward) | history-step rebuild path (same replacement shape as #2; audit `content/navigation.rs` traversal callers) | explicit — same "before replacing outgoing pipeline" rule as #2 |
| 4 | **iframe** pipeline replacement / iframe teardown | `content/iframe/*` lifecycle (per-iframe pipeline drop) | explicit at iframe pipeline drop; Drop-backstop covers the forget path |
| 5 | **Engine Drop** (any pipeline drop, incl. panic-unwind) | `impl Drop` on `ElidexJsEngine` / `PipelineResult` | RAII backstop — runs `teardown_document` iff not-already-run |

**Is Drop alone sufficient (pure RAII)?** No — the current unbind comment's rationale
(`vm_api.rs:461-467`) is that closing **eagerly bounds the broker I/O thread's lifetime** rather than
waiting for the `NetworkHandle` Drop, which "can be much later if the embedder keeps the handle around
for a subsequent bind". Under the flip the `NetworkHandle` may be reused across documents (it is
retained on the shell side), so relying on `NetworkHandle` Drop alone could leak the thread across a
navigation. Hence **explicit calls at #1–#4 + Drop as backstop** (#5), not Drop-only. This is the same
eager-bounding intent the current code has, relocated to the document granularity.

**Idempotency / once-ness argument** (plan-review must verify): explicit call at #1–#4 empties the
tables; the Drop backstop (#5) re-invokes but finds empty tables → no-op. The only double-fire risk is
#2/#3 explicit + #5 Drop on the SAME pipeline — covered by the empty-table no-op. The only leak risk
is a boundary NOT in the table above — reviewers should adversarially search for a pipeline-drop path
that bypasses #1–#4 AND somehow also bypasses Drop (should be impossible in safe Rust).

---

## §6 Open design questions (for plan-review)

### §6.1 (CRITICAL) Is the keystone SELF-CONTAINED, or gated on agent-scoped EcsDom (E2)?

**Investigation finding: SELF-CONTAINED. It is landable within S5-6b and is NOT blocked on the
deferred cross-batch-wrapper-identity / agent-scoped-EcsDom work (`[[project_world-id-cross-dom-migration]]`,
flip-memo E2 `:461-465`).** Evidence:

1. **WS/SSE/Worker JS objects are constructor results, not `[SameObject]` getters.** `const ws = new
   WebSocket(url)` / `new EventSource(url)` / `new Worker(url)` bind an `ObjectId` that JS holds
   **directly**. There is no per-access getter that re-resolves the object from a wrapper_cache (unlike
   `el.classList`, the E2 surface). Their identity (`ws === ws`) is preserved by the **VM object heap**,
   which persists across per-turn unbind — `unbind` clears *pointers + wrapper caches + side-tables*,
   it never drops the VM or its GC heap. So identity rides the heap regardless of any wrapper_cache
   clearing.

2. **Connection STATE is keyed by that same `ObjectId`.** `websocket_states: HashMap<ObjectId,
   WebSocketState>` (`host_data/mod.rs:502`), `event_source_states: HashMap<ObjectId,
   EventSourceState>` (`:524`); the reverse routing map `ws_conn_to_object: HashMap<u64, ObjectId>`
   (`:508`) / `sse_conn_to_object` (`:528`) maps broker `conn_id → ObjectId`. Because the `ObjectId`
   remains valid on the heap across unbind (GC-kept while the connection is live — the S5-3 keepalive
   root), simply **not clearing** these tables keeps state reachable and correctly keyed. Delivery
   (`tick_network`) resolves target via `ws_conn_to_object[conn_id] → ObjectId` **directly** — no
   wrapper_cache round-trip, so E2 cannot touch it.

3. **Per-turn unbind is a SAME-DOM re-establishment**, not a cross-DOM rebind. Under the already-current
   model (B1: 1 agent = 1 World = 1 Vm), the per-turn bracket re-binds the SAME `session`/`dom`/
   `document`. The cross-DOM entity-index-aliasing hazard that motivates the aggressive clears exists
   ONLY at a document boundary (fresh `EcsDom`). So keeping the connection/worker state across a
   *same-DOM* turn is aliasing-safe by construction — it does not need agent-scoped EcsDom to be safe;
   it is safe under today's model.

**Worker `event.target` identity — resolved by source, no wrinkle (see §4 Part B for the full
derivation):** worker message delivery resolves the worker `Entity → Worker` wrapper via
`get_cached_wrapper(entity)` to seed `event.target` (`worker.rs:133-147`). That wrapper is cached
under `WrapperKind::Node` (`host_data/mod.rs:1717-1734`), so the per-turn retain
`wrapper_store.retain(kind == Node)` (`vm_api.rs:622-623`) **retains** it. Once Part A moves
`teardown_workers` off per-turn `unbind`, nothing per-turn removes it and `event.target === myWorker`
holds by construction (same-DOM-stable under 1-Vm-per-World; the Worker HostObject does not snapshot
`bind_epoch`). No new map and no widened retain predicate are required — the Node-kind wrapper cache
already serves as the surviving `entity → ObjectId` map. Plan-review need only confirm the Node-kind-
caching assumption (pinned by the §7 2f-k-b identity test); the general E2 getter surface is untouched.

### §6.2 Per-document teardown entry points — is the §5 set complete and once-firing?

Enumerate EVERY document-destruction path (§5 lists 5) and confirm each invokes `teardown_document`
exactly once with no double-close/leak. Specifically: (a) is the history-traversal rebuild (#3) a
distinct site from the address-bar Navigate (#2), or do they share one replacement chokepoint? (b)
does the iframe teardown path (#4) drop the iframe pipeline in a way that reaches the Drop backstop,
or can an iframe pipeline leak? (c) is `dispatch_unload_events` (event_loop.rs:314) the right
*ordering* anchor — teardown AFTER unload handlers run (so `beforeunload`/`unload` can still use the
connection) but BEFORE thread exit?

### §6.3 Which of unbind's current actions are genuinely per-turn vs per-document?

§4-Part-A gives the classification table; plan-review should independently re-derive it from the full
`unbind` body (`vm_api.rs:454-708`) and challenge any STAY/MOVE call. The load-bearing ones: IDB txn
rollback (claimed per-turn — is it? a long-lived IDB txn spanning turns would be aborted every turn,
which may itself be a latent bug orthogonal to this keystone — flag if so), and the dispatcher pairing
(claimed strictly per-bracket via the `displaced.is_none()` assert).

### §6.4 Test strategy — the new load-bearing coverage inverts two existing tests

The NEW coverage = a **multi-turn survival test**: establish WS + SSE + Worker → run ≥2 bind/unbind
turns (per-turn unbind fires) → assert all three still alive + deliverable (no `Close` sent, worker
thread not terminated, next `tick_network` / `drain_worker_messages` delivers). **Two existing tests
assert the OPPOSITE and must be RE-HOMED onto the per-document teardown path**, not deleted:
- `crates/script/elidex-js/src/vm/tests/tests_realtime_keepalive.rs:566`
  `unbind_force_closes_even_listener_held_connection` — asserts `outgoing WebSocketClose == 1` after
  `unbind` (`:604-608`). Re-home: the force-close assertion moves to a `teardown_document`-driven test
  (`unbind` alone must now assert **zero** `Close`; `teardown_document` asserts the `Close`).
- `crates/script/elidex-js/src/vm/tests/tests_worker.rs:741` `worker_wrappers_uncached_on_unbind` —
  asserts `get_cached_wrapper(entity).is_none()` after `unbind` (`:765`). Re-home: the uncache
  assertion moves to a `teardown_document` test; the per-turn `unbind` test must now assert the worker
  wrapper **survives** and `event.target === myWorker` holds (the §4-Part-B Node-kind-retention
  finding — no wrinkle; the wrapper is retained by the `:622` `kind==Node` predicate).

The S5-3 keepalive tests (GC-scoped WITHIN a bind cycle) are unaffected — they already do not exercise
the across-unbind boundary. Plan-review: confirm the re-homing preserves the *intent* (teardown DOES
close, just at the document boundary) rather than dropping coverage.

---

## §7 Sub-commit split + acceptance

The keystone is a base-case terminal slice under the approved S5 umbrella + this plan-review (CLAUDE.md
"base case = narrowly-scoped per-PR slice"), so it is **one PR** — but internally sequenced so each
sub-commit is independently reviewable and the survivor promotion lands **before** its shell consumer.

| Sub-commit | Content | Acceptance |
|---|---|---|
| **2f-k-a** | Split `unbind` → per-turn `unbind` + `Vm::teardown_document` (Part A). Realtime/worker teardown blocks MOVE verbatim. No behaviour change yet at the call sites (unbind no longer tears down, but nothing calls `teardown_document` — so this commit ALONE would leak; it is completed by 2f-k-b/c in the same PR). | Compiles; `unbind` body matches the §4 STAY table; `teardown_document` = the two MOVED blocks + `unbind()`. |
| **2f-k-b** | Promote survivor set (Part B): remove the clears (already gone with the MOVE); keep the conn-id counters. No new worker map / retain-predicate change — the worker wrapper is already Node-kind-retained by `:622` once Part A moves teardown off per-turn (§4 Part B). **Multi-turn survival test** + **multi-turn `event.target === myWorker` identity test** (§6.4). | Survival test green: WS/SSE/Worker survive ≥2 per-turn unbinds, deliverable. Identity test green: after ≥2 per-turn unbinds a worker `message` event has `event.target === myWorker` (the same JS-held `Worker` object), pinning the Node-kind-caching assumption. |
| **2f-k-c** | B15 convergence (Part C): wire `teardown_document` at §5 boundaries #1–#4 + engine-Drop backstop #5; delete boa `shutdown_all_realtime`/`shutdown_all_workers`. **Re-home** the two teardown-asserting tests (§6.4). | Re-homed tests green on `teardown_document`; `unbind`-alone asserts zero Close; no boa `shutdown_all_*` call sites remain. |
| **2f-k-d** | B14 follow-on (Part D): `PipelineResult::tick_network` bracketed helper + per-turn call; delete boa realtime pump (`event_loop.rs:198-216` + boa `realtime.rs` defs). | Realtime E2E: WS echo + SSE stream deliver across multiple turns through the VM path; boa `drain_realtime_events`/`dispatch_realtime_events` grep-clean. |

**Acceptance gate (flip-memo §8 item 5, `:922`).** Workers + realtime are a **supported surface**;
the flip's acceptance gate requires every `deliver_*` to have a live bracketed shell producer that
does not regress. This keystone is the precondition for `tick_network` (B14) + `drain_worker_messages`
(B24, already landed) to be non-destructive. After 2f-k-d: WS/SSE/Worker survive the full per-turn
bracket storm AND are correctly torn down at document boundaries — the gate item is satisfiable.

**Push gate**: `mise run ci` + `/pre-push` (6-stage) + `/external-converge` (Codex) — this is an
edge-dense keystone, so the external pass is the full convergence loop, not a single shot.

---

## §8 Carve list

- **`#11-cross-batch-wrapper-identity`** (E2, pre-existing, flip-memo `:461-465`,`:473`,`:1172`) — the
  general `[SameObject]` getter-attribute survival across same-DOM turns (classList/dataset/style/…).
  This keystone deliberately does NOT touch it: it fixes loss-of-function (connections/workers), not
  the identity-delta, and the connection objects are constructor-results not getters (§6.1). Remains
  gated on agent-scoped EcsDom. (Pre-existing registered slot — no new carve.)
- **`#11-idb-transaction-turn-spanning`** (NEW carve, if §6.3 confirms the per-turn IDB txn rollback
  aborts a legitimately turn-spanning transaction) — orthogonal latent issue surfaced by this audit;
  carve if real, do not fix here.
  - *Why deferred*: out of this keystone's scope (realtime/worker lifetime, not IDB txn granularity);
    unconfirmed until §6.3 is adjudicated.
  - *Re-evaluation trigger*: §6.3 plan-review verdict (is the per-turn IDB txn rollback a real bug?).
  - *Re-evaluation date*: trigger-gated, not calendar — **at S5-6b landing** (the §6.3 verdict lands
    with this PR's plan-review; if confirmed real, the slot is registered then).
- **`#11-web-animations-element-animate`** (B22 / WAAPI, S5-7; flip-memo B22 `:383`,
  `project_open-defer-slots.md:65`) — the one known VM<boa surface at flip; not this keystone's
  concern, already umbrella-scheduled (S5-7 depends on S5-6). (Pre-existing registered slot — no new
  carve.)
- **`#11-teardown-document-shared-worker-scope`** (NEW carve, if SharedWorker enters scope) —
  `teardown_document` as specced here covers dedicated workers (§10.2.4 driven from the owning
  document); SharedWorker lifetime is ref-counted across documents and would need a distinct teardown
  trigger. Out of scope (no SharedWorker surface yet).
  - *Why deferred*: no SharedWorker surface exists in the engine today; adding a ref-counted teardown
    trigger with no consumer would be speculative abstraction.
  - *Re-evaluation trigger*: a SharedWorker surface landing (constructor + registry).
  - *Re-evaluation date*: trigger-gated, not calendar — **when the SharedWorker surface lands**
    (no scheduled phase; event-gated on that feature's kickoff).
