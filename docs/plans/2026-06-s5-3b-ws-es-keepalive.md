# S5-3b — WebSocket / EventSource keepalive arm (the state-tiered network predicate)

Per-PR plan-memo under the S5 umbrella (`docs/plans/2026-06-s5-flip-boa-deletion-umbrella.md`)
and the S5-3 program memo (`docs/plans/2026-06-s5-3-eventtarget-listener-keepalive-rooting.md`,
§7 split decision: **S5-3b = WS/ES**). **Anchor = the ideal end-state**, not an incremental patch
(`feedback_plan-memo-anchor-on-ideal-not-incremental`).

S5-3a (#430 `3345949e`) landed the keepalive-**predicate seam** (`crates/script/elidex-js/src/vm/gc/keepalive.rs`)
+ the `MediaQueryList` arm + the `AbortSignal.timeout` membership root. S5-3b is the **next arm**:
extend that seam with **`WebSocket` + `EventSource`** predicates so a *listener-held open connection
survives GC and keeps delivering*, while the genuine-orphan (no-listener) connection keeps the
GC-close. This is a **behavior change to established, behavior-bearing code** (the network
force-close path) — hence edge-dense, hence this plan-review (CLAUDE.md "Edge-dense work = multi-PR
program + 実装前 plan-review 必須"; the S5-3 §7 split carves S5-3b as its own plan-reviewed slice).

> **⚠ DESIGN inheritance (read with the parent):** the parent S5-3 memo's `world_id` framing is
> **SUPERSEDED by the agent-scoped `EcsDom` World program** (PR #434 `deb6eaf6`,
> `docs/plans/2026-06-agent-scoped-ecsdom-world.md`). Throughout this memo, the keepalive
> component-on-entity migration (`#11-eventtarget-keepalive-component-migration`) is **B1-gated**
> (1-agent = 1-World makes per-entity identity stable without a discriminator), **not** world_id-gated.
> Do not reintroduce world_id framing.

> **Gate**: `/elidex-plan-review` (5-agent) BEFORE impl. This memo maps the edge matrix (§8) +
> coupled invariants (§2.4) so plan-review can pre-empt the review tail, settles the spec reading
> (§2, webref-verified 2026-07-01), and answers the scope question (§7).

All file:line cites grep-verified against `main` HEAD `3345949e` (2026-07-01). Every spec § prose
webref-verified 2026-07-01 (sources: `websockets`, `html` server-sent-events, `dom`).

---

## §0.0 Premise revision (Codex external-review Round 1)

**This memo was revised after Codex external-review Round 1 refuted the three delivery-model premises
the original draft rested on.** The original assumed (a) elidex GC runs only at event-loop turn
boundaries, (b) inbound SSE/WS events are dispatched inline with no cross-GC buffer window, and (c) an
outbound WS send could be dropped on GC — and from those it concluded WS `data-queued` was the
implementable no-listener clause while ES `task-queued` was vacuous. **All three premises are wrong**
(verified this session — §2.0): GC is **allocation-triggered and fires mid-turn / mid-task**; inbound
events are **buffered in the NetworkHandle between `drain_fetch_responses_only` and `drain_events`**, and
a mid-turn GC in that gap **silently drops** a buffered event whose wrapper was collected; outbound WS
sends are **broker-owned FIFO once emitted**, transmitted regardless of wrapper survival. The design
consequence: **§2.3's no-listener-clause mapping is SWAPPED** — WS `data-queued` is **VACUOUS in elidex
→ OMIT** (and keying keepalive on `buffered_amount` would *over-root* a listener-less CLOSING socket into
an indefinite leak — Codex F1), while ES `task-queued` is **MEANINGFUL → INCLUDE** (it is exactly the
missing GC root for the inbound buffer window — Codex F3). A third finding (F2) records a real but
observably-inert §7 "readyState as of event-loop step 1" letter-gap → documented + slotted, not fixed
here. The rest of the memo (seam, layering, tests, edge matrix) is updated to this corrected substrate.

**Implementation status:** the committed impl on branch `s5-3b-ws-es-keepalive` (PR #440, `4fda1cda`)
reflects the **OLD pre-R1 design** (WS `has_queued_data` present, ES `has_queued_task` absent, docstrings
claiming the mappings the reverse way). It will be **rewritten to this corrected design after this
re-plan-review passes** — so any "plan says X / code does Y" gap for that commit is expected, not a plan
defect.

---

## §0 Read-first (scope + the central reframe, inherited)

### §0.1 What S5-3b is
A **FLIP-precondition** (umbrella §5 type-(a): land BEFORE the S5-6 boa→VM flip), **VM-internal**, boa
stays live, **no external dependency**. The deliverable is **two new arms on the existing keepalive
seam** (`KeepaliveClass::WebSocket` / `::EventSource`) whose spec-faithful **state-tiered** predicate
keeps a listener-held open WS/ES alive across a GC — fixing the latent bug that today a listener-only
`new WebSocket(url); ws.onmessage = cb` (no retained reference) is **GC-swept and force-closed**, so
the connection dies and stops delivering the moment the page drops its last explicit reference (§1).

It is **inert today** (boa is the live engine; the VM WS/ES message pumps are dormant), but it **gates**
the flip: once the VM drives the shell (S5-6), the gap becomes a live "real-time site silently loses its
socket on the next GC" regression. WS/ES force-close is **pre-existing** behavior (the
`#11-net-ws-sse` / D-12 sweep) — S5-3b **migrates** it onto the seam; it does **not** introduce
divergence (S5-3 §0.3 strangler-safety).

### §0.2 The central reframe (inherited, non-negotiable) — state-tiered, NOT any-listener
The seam is a **per-registrant keepalive PREDICATE**, never an "any-listener roots the target" rule
(DOM §2.8 "Observing event listeners": listener presence must not be observable; there is no general
listener-keepalive rule — S5-3 §2). For WS/ES the spec keepalive is explicitly **state-tiered**: a
`<readyState> × <type-restricted listener subset>` test, **plus** a no-listener in-flight-work clause.
A naive "OPEN connection with any listener stays alive", or worse "OPEN connection stays alive", would
be **over-rooting** (a leaked network thread for a socket nobody listens to — a §2.8 violation in the
leak direction, and a real resource leak). §2 pins the exact tiers from spec prose; §2–§5 must not
regress to "any listener" or "any OPEN".

### §0.3 Strangler-safety (inherited)
S5-3b is **bounded in-program staging under the hard pre-flip gate**
`#11-eventtarget-keepalive-registrant-coverage` (S5-3 §10): **all of S5-3a/b/c MUST land before S5-6**.
The seam + remaining-legacy coexistence is bounded to the pre-flip dev window and **force-resolved by
the flip gate** — the sanctioned staged delivery, NOT the forbidden indefinite strangler. S5-3b is
flip-MANDATORY (a registrant left off the seam at the flip = exactly the forbidden form) but is
flip-*order*-independent relative to S5-3c.

---

## §1 The gap — precise GC mechanics (cited, `3345949e`)

The WS/ES wrappers are non-Node `EventTarget`s with the **same root gap** the seam was built to close,
but with an **extra teardown**: a swept wrapper also **force-closes the network connection**.

1. **The wrapper is not a root and is not traced.** A `WebSocket` / `EventSource` instance's
   out-of-band state lives in `HostData::websocket_states` / `event_source_states`
   (`HashMap<ObjectId, WebSocketState>` / `…<…, EventSourceState>`, `host_data.rs:466` / `:488`), keyed
   by the wrapper's own `ObjectId`. Its listeners (incl. the `onopen`/`onmessage`/`onerror`/`onclose`
   IDL handlers) live in the unified `VmInner::vm_event_listeners` home since
   `#11-realtime-event-listeners` (the `WebSocketState` / `EventSourceState` doc-comments say so,
   `host_data.rs:608-612` / `:700-706`). The **callbacks** are rooted via `listener_store`, but the
   **wrapper** is not a `GcRoots` member and is never marked.

2. **The sweep prunes the state row by the wrapper's mark bit AND force-closes the conn.** At
   `collect.rs:1891-1935`: `dead_ws_conns` / `dead_sse_conns` collect every `conn_id` whose wrapper
   `ObjectId` mark bit is clear (`bit_get(marks, obj_id.0)` false), then
   `websocket_states.retain(|id, _| bit_get(marks, id.0))` prunes the row, and a
   `RendererToNetwork::WebSocketClose(conn_id)` / `EventSourceClose(conn_id)` is emitted to the broker
   per swept conn (`:1924-1932`). The existing comment (`:1875-1886`) states the design intent: the GC
   sweep **is** the explicit close for orphaned wrappers (no implicit cleanup — CLAUDE.md
   "後方互換性は維持しない").

3. **Headline failure.** A listener-only OPEN WS/ES (`new WebSocket(u); ws.onmessage = cb;` with no
   retained reference) has no anchor but its listener (callback rooted, wrapper not). On any GC it is
   swept, its state row pruned, **and its connection force-closed** — so subsequent server frames are
   never delivered. This is **worse than the MQL bug**: the MQL silently stopped delivering; the WS/ES
   *also tears down live network I/O*. Today this is masked only because boa is the live engine; at the
   flip it becomes a live regression for every real-time site that doesn't pin its socket.

**Why the existing force-close is otherwise correct (the boundary S5-3b must preserve):** for a
**genuine orphan** (no listener, no in-flight work) the spec *mandates* the GC-close — WebSockets §7
("If a WebSocket object is garbage collected while its connection is still open, the user agent must
start the WebSocket closing handshake"), HTML §9.2.9 ("If an EventSource object is garbage collected
while its connection is still open, the user agent must abort … the fetch"). So S5-3b must **keep the
force-close for the orphan** and only **suppress it for the listener-held / in-flight case** (§4).

---

## §2 Why spec-faithful = a state-tiered predicate (webref-verified prose)

### §2.0 Corrected delivery model (verified — the coupled-invariant substrate)
The no-listener clauses (§2.3) do not map onto elidex by spec text alone — they map onto elidex's actual
**GC timing + inbound-buffer + outbound-FIFO** mechanics. These five facts (verified this session against
`main` `3345949e`) are the substrate the predicates now rest on; the §2.3 mapping is derived from them,
not from the spec prose in isolation.

1. **GC is allocation-triggered and runs MID-TURN / mid-task, NOT only at event-loop turn boundaries.**
   `alloc_object` calls `collect_garbage()` when the byte threshold is crossed
   (`crates/script/elidex-js/src/vm/inner.rs:92`). So a GC can fire *during* event-handler dispatch and
   *during* `tick_network` — any allocation is a potential collection point. This invalidates the
   original "GC only between turns, so no cross-GC in-flight window" premise.

2. **Inbound network events (SSE `EventSourceEvent`, WS message) are BUFFERED in the NetworkHandle
   between arrival and dispatch.** `tick_network` (`crates/script/elidex-js/src/vm/host/fetch_tick.rs`)
   calls `drain_fetch_responses_only()` (~`:58`) FIRST — which settles fetches AND **allocates**
   (`create_response_from_net`) — leaving WS/SSE events in the handle buffer
   (`crates/net/elidex-net/src/broker/buffered.rs:140` re-buffers non-fetch events), THEN calls
   `drain_events()` (~`:78`) to dispatch them. **A GC triggered by the fetch-settle allocations runs in
   the gap WHILE SSE events sit buffered.** This is the cross-GC inbound window the original memo denied
   existed.

3. **Inbound dispatch routes via a `conn_id → ObjectId` reverse-map; a MISS silently drops the event.**
   `fetch_tick.rs:169-176`: `sse_conn_to_object.get(&conn_id)` → `else { return; }`. If the GC sweep
   collected the wrapper (which prunes both `event_source_states` AND `sse_conn_to_object`,
   `collect.rs:1919-1932`), the later-drained buffered event finds no target and is **silently lost** —
   no error, no delivery.

4. **The dispatch target is rooted ONLY during active handler execution, via the `this` call-frame
   binding; during the BUFFER WINDOW it is rooted ONLY by the keepalive predicate.** There is no
   independent "pending-event / dispatching-target" GC root — `gc/roots.rs` roots the Event *object*,
   not the dispatch *target*. So between buffer and drain, the keepalive predicate is the **only** thing
   that can keep the target wrapper alive to be routed to.

5. **Outbound WS sends are broker-owned once emitted, FIFO-ahead of any force-close.** `native_ws_send`
   (`crates/script/elidex-js/src/vm/host/websocket.rs:836`) increments `buffered_amount`
   **UNCONDITIONALLY** (incl. CLOSING/CLOSED), but only emits `RendererToNetwork::WebSocketSend` to the
   broker when OPEN (`:847`; comment `:843-846`: CLOSING/CLOSED keep the increment but do NOT transmit,
   per WebSockets §3.1 `send(data)` steps, `#dom-websocket-send` — CLOSING/CLOSED: increase
   `bufferedAmount`, do not transmit). The renderer→broker channel is a SINGLE FIFO `crossbeam_channel::unbounded`
   (`crates/net/elidex-net/src/broker/handle.rs:832`), drained in enqueue order
   (`broker/dispatch/mod.rs:69-75`); `WebSocketSend` and `WebSocketClose` both go to the same per-conn
   `command_tx` (`dispatch/mod.rs:218-230`). So a `WebSocketSend` enqueued at `send()`-time is **ALWAYS**
   processed before a later GC-emitted `WebSocketClose` → emitted bytes transmit **regardless** of
   whether the wrapper is kept or collected. The wrapper holds **no** outbound queue.

**Net consequence (drives §2.3):** the two directions are asymmetric. *Outbound* (WS `data-queued`) is
already durable via the broker FIFO — the wrapper protects nothing, so its no-listener clause is vacuous.
*Inbound* (ES `task-queued`) needs the wrapper alive **at dispatch time** to route via the reverse-map —
so its no-listener clause is exactly the GC root the buffer window requires. Fact 5 additionally makes
`buffered_amount`-keyed keepalive an over-root hazard (unconditional increment → CLOSING leak, Codex F1).

### §2.1 WebSockets §7 — Garbage collection (`websockets#garbage-collection`, webref 2026-07-01)
Verbatim prose:

> - A WebSocket object whose ready state was set to **CONNECTING** … must not be garbage collected if
>   there are any event listeners registered for **open, message, error, or close** events.
> - A WebSocket object whose ready state was set to **OPEN** … must not be garbage collected if there
>   are any event listeners registered for **message, error, or close** events.
> - A WebSocket object whose ready state was set to **CLOSING** … must not be garbage collected if there
>   are any event listeners registered for **error or close** events.
> - A WebSocket object with an **established connection that has data queued to be transmitted** to the
>   network must not be garbage collected.
> - If a WebSocket object is garbage collected while its connection is still open, the user agent must
>   start the WebSocket closing handshake, with no status code.

So the keepalive condition is `state ∈ {CONNECTING, OPEN, CLOSING}` with the **tiered** listener subset,
**OR** the no-listener clause `established ∧ data-queued`. **CLOSED ⇒ never kept.** The data-queued
clause proves keepalive is *not purely a listener test* — it is an **in-flight-work** test.

### §2.2 HTML §9.2.9 — EventSource Garbage collection (`html#garbage-collection`, webref 2026-07-01)
Verbatim prose:

> - While an EventSource object's readyState is **CONNECTING**, and the object has one or more event
>   listeners registered for **open, message, or error** events, there must be a strong reference …
> - While an EventSource object's readyState is **OPEN**, and the object has one or more event listeners
>   registered for **message or error** events, there must be a strong reference …
> - While there is a **task queued by an EventSource object on the remote event task source**, there
>   must be a strong reference …
> - If an EventSource object is garbage collected while its connection is still open, the user agent
>   must abort any instance of the fetch algorithm opened by this EventSource.

So `state ∈ {CONNECTING, OPEN}` with the tiered listener subset, **OR** the no-listener clause
`task-queued-on-remote-event-task-source`. **CLOSED ⇒ never kept.**

### §2.3 The two no-listener clauses map ASYMMETRICALLY onto elidex state — WS VACUOUS, ES MEANINGFUL
This is the design substance unique to S5-3b (the WS/ES rows the parent §5 deferred to here). **Codex R1
established that the directionality of the mapping is the OPPOSITE of the original draft** (§0.0): the
mapping is driven by §2.0's delivery mechanics, not the spec prose in isolation.

- **WS `established ∧ data-queued` (§7, an OUTBOUND clause) → VACUOUS in elidex → OMIT the clause.**
  §7's no-listener clause covers "an established connection that has **data queued to be transmitted to
  the network**" — i.e. **outbound** bytes the wrapper is responsible for flushing. In elidex (§2.0
  fact 5) `send()` emits **synchronously** to the broker's FIFO channel; the wrapper holds **no outbound
  queue**; emitted bytes are FIFO-ahead of any GC-emitted `WebSocketClose` so they **always transmit**
  whether the wrapper survives or is collected. Keeping the wrapper alive on `buffered_amount > 0`
  therefore **protects nothing**. **WORSE (Codex Finding F1):** `buffered_amount` is incremented
  **UNCONDITIONALLY** — including for CLOSING/CLOSED sends that never transmit and never receive a
  `BytesSent` to clear them (`websocket.rs:836` add is unconditional; `:847` transmit is OPEN-only). So
  keying keepalive on `buffered_amount > 0` would **OVER-ROOT** a listener-less CLOSING socket into an
  **INDEFINITE LEAK** — the side-table row + connection would live forever if the peer never completes
  the close, because nothing ever drives `buffered_amount` back to 0. **Fix:** `ws_keepalive` becomes a
  **PURE readyState-tier check** — **drop** the `has_queued_data` parameter and the `buffered_amount`
  marshalling entirely.

- **ES `task-queued-on-remote-event-task-source` (§9.2.9, an INBOUND clause) → MEANINGFUL in elidex →
  INCLUDE the clause.** §9.2.9's no-listener clause keeps the wrapper alive "while there is a **task
  queued by an EventSource object on the remote event task source**" — i.e. an **inbound** event that has
  arrived but not yet been dispatched to script. In elidex (§2.0 facts 1-4) this window is **real**: an
  inbound SSE event buffers between `drain_fetch_responses_only` and `drain_events`; a **mid-turn GC** in
  that window can collect a wrapper whose **only** listener is a **NAMED** event
  (`addEventListener('foo', …)` — a type NOT in the readyState tier `{message, error}`); the sweep prunes
  `event_source_states` **and** the `sse_conn_to_object` reverse-map; and the later-drained buffered
  `'foo'` event then hits a reverse-map MISS and is **SILENTLY DROPPED** (Codex Finding F3). The §9.2.9
  task-queued clause is **exactly the missing GC root** for this buffer window — without it, a real-time
  SSE consumer using named events loses events on any allocation-triggered GC. **Fix:** `es_keepalive`
  gains a `has_queued_task: bool` parameter; if `true` it keeps the wrapper alive **regardless** of
  readyState-tier listeners. The seam marshals `has_queued_task` by querying the NetworkHandle buffer for
  a pending `EventSourceEvent(conn_id)` for this ES's `conn_id` — a **new engine-side peek method on
  NetworkHandle**, e.g. `has_pending_event_for_conn(conn_id) -> bool`, that **settles the channel into
  the buffer (same `process_response` routing as a drain) then scans** — covering the full inbound
  pipeline (channel + buffer), not just already-buffered events, so a GC between arrival and the first
  drain doesn't miss a channel-pending event (Codex R2a) (conn_id read from
  `event_source_states[target].conn_id`).

- **Why the asymmetry (the spec is DIRECTIONAL).** The specs' no-listener clauses point in **opposite
  directions**: §7's is **OUTBOUND** ("data queued to be transmitted to the network"), §9.2.9's is
  **INBOUND** ("task queued on the remote event task source"). In elidex, **outbound** bytes are
  **broker-owned once emitted** (→ the clause is vacuous: the wrapper is not on the critical path), while
  **inbound** events need the wrapper **alive at dispatch time** to route via the reverse-map (→ the
  clause is meaningful: the wrapper *is* the critical path). **The corrected (WS-vacuous / ES-meaningful)
  mapping is the SPEC-FAITHFUL one; the original memo had the directionality backwards.** Note the neat
  consequence: **WS §7 has NO inbound clause**, and all WS inbound event types (message/error/close)
  **ARE** in the readyState tiers — so a buffered WS message necessarily implies an in-tier listener → a
  WS analogue of F3 **cannot arise** (no named-event escape hatch). **ES §9.2.9 HAS the inbound clause
  precisely because named events are NOT in its tier `{message, error}`** — the clause is the spec's own
  acknowledgment that a named-event-only ES must survive its buffer window.

### §2.4 Coupled-invariant enumeration (edge-dense canonical home)
The S5-3b arms must satisfy, **together**:

- **GC-rooting** — the predicate runs in `keepalive_survivors` (called at `collect.rs:1233`, marked at
  `:1237`) and `mark_object`s survivors **before** `trace_work_list` (`:1315`) and **before** the sweep
  (`:1891-1935`).
- **listener-lifecycle (type-restricted, per state tier)** — the per-target listener home
  (`vm_event_listeners`) is read **by type**, counting both `addEventListener(type)` and the `on<type>`
  IDL handler, and **excluding** a cleared `on<type> = null` (via `vm_path_has_listener`, §4.2).
- **per-class-predicate** — WS/ES each register their own spec-faithful rule, **owned by the
  engine-independent `elidex-api-ws`** (§4.4 layering), the seam only marshals.
- **active-state** — the readyState tier (`WsReadyState` / `SseReadyState`).
- **in-flight-work** — **WS `data-queued` = VACUOUS** (outbound bytes broker-owned FIFO once emitted, §2.0
  fact 5; keying on `buffered_amount` over-roots CLOSING = F1) → **removed**; **ES `task-queued` =
  MEANINGFUL** (inbound event buffered between fetch-drain and event-drain, §2.0 facts 1-4; a mid-turn GC
  that collects the wrapper drops the buffered event via reverse-map miss = F3) → **INCLUDED** via
  `has_queued_task` marshalled from the NetworkHandle buffer peek.
- **buffer-window × reverse-map-miss (NEW, the ES-critical coupled invariant)** — an inbound SSE event
  sits buffered in the NetworkHandle while a mid-turn GC runs; the dispatch routes `conn_id → ObjectId`
  and a miss silently `return`s (`fetch_tick.rs:169-176`); the wrapper is rooted in that window **only**
  by the keepalive predicate (§2.0 fact 4). So `es_keepalive` **must** return true whenever a task is
  queued for the conn, or the buffered event is lost. This is the invariant the §9.2.9 clause exists to
  hold.
- **force-close interaction** — a predicate-marked conn must survive the sweep retain **and** emit no
  Close; the un-marked orphan must still force-close (`collect.rs:1891-1935`, **unchanged** — §4.3).
- **unbind-lifecycle** — both state maps are `drain_realtime_for_unbind`-cleared on `Vm::unbind`
  (`host_data.rs:1809-1819`), which **also** emits Close for *every* conn (even listener-held) — the
  spec's "Document goes away ⇒ make disappear / forcibly close" rule (distinct from GC keepalive, §8.4).
- **B1-home** — the rooted thing is a per-VM `ObjectId` (side-store→component exception (a)); component
  migration is B1-gated (§6).

Key pairwise intersections (one line each):
- **GC-rooting × force-close** — marking the wrapper sets its bit, so the sweep's `bit_get(marks, id.0)`
  retains the row and skips the Close emit; **the existing sweep IS the else-branch** (no edit, §4.3).
- **per-class-predicate × active-state** — WS tier = readyState ∧ tiered-listener-subset (PURE — no
  data-queued disjunct, §2.3 F1); ES tier = readyState ∧ tiered-listener-subset **∨ has_queued_task**
  (buffer-window root, §2.3 F3).
- **listener-lifecycle × on-handler** — the type test must count `ws.onmessage = cb` with no
  `addEventListener` (else an on-handler-only page's socket is wrongly collected); `vm_path_has_listener`
  already does (§4.2).
- **unbind-lifecycle × force-close** — unbind force-closes even a listener-held conn (correct: document
  teardown is the spec's *forcible* close, not GC) — confirm this is not mistaken for a keepalive bug.

---

## §3 Spec coverage map (keepalive RULES × condition-tiers)

Each row is a `<spec GC-note> × <state/listener condition-branch>`; the **seam arm (§4)** implements
every row. "Touch" names the predicate site from §5.

| Spec section | Step / condition | Branch | Touch (predicate site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WebSockets §7 GC | CONNECTING | listener ∈ {open,message,error,close} | `ws_keepalive` over `websocket_states` | ✓ | yes (page listeners) |
| WebSockets §7 GC | OPEN | listener ∈ {message,error,close} | `ws_keepalive` | ✓ | yes |
| WebSockets §7 GC | CLOSING | listener ∈ {error,close} | `ws_keepalive` | ✓ | yes |
| WebSockets §7 GC | CLOSED | never kept | `ws_keepalive` returns false → swept | ✓ | no |
| WebSockets §7 GC | established ∧ data queued | no-listener clause (OUTBOUND) | **OMITTED / VACUOUS** — outbound bytes broker-owned FIFO once emitted (§2.0 fact 5), so wrapper protects nothing; keying on `buffered_amount` over-roots CLOSING = **F1 leak**. `ws_keepalive` is a PURE tier check | ✓ | — |
| WebSockets §7 GC | swept (orphan / CLOSED) | force-close handshake | `collect.rs:1891-1935` else-branch (**unchanged**) | ✓ | no |
| HTML §9.2.9 EventSource GC | CONNECTING | listener ∈ {open,message,error} | `es_keepalive` over `event_source_states` | ✓ | yes |
| HTML §9.2.9 EventSource GC | OPEN | listener ∈ {message,error} | `es_keepalive` | ✓ | yes |
| HTML §9.2.9 EventSource GC | CLOSED | never kept | `es_keepalive` returns false → swept | ✓ | no |
| HTML §9.2.9 EventSource GC | task queued on remote event task source | no-listener clause (INBOUND) | **INCLUDED / MEANINGFUL** — inbound event buffered between `drain_fetch_responses_only` and `drain_events` (§2.0 facts 1-4); mid-turn GC collecting the wrapper drops it via reverse-map miss = **F3**. `es_keepalive(has_queued_task=true) ⇒ true` regardless of tier | ✓ | yes (server push) |
| HTML §9.2.9 EventSource GC | swept (orphan / CLOSED) | force-close abort fetch | `collect.rs:1891-1935` else-branch (**unchanged**) | ✓ | no |
| DOM §2.8 Observing event listeners | general default | — | seam does NOT root on bare-listener / bare-OPEN presence | ✓ | yes |

### §3.1 Breadth + split verdict
**K = 2** specs (WebSockets, HTML/SSE); **M = 11** rows. This is the **WS/ES row-subset** the parent
S5-3 §3 table (K=6 / M=13) carved into its own slice — so the breadth here is bounded to two interfaces
sharing one mechanism (state-tier + listener-subset). S5-3b is a **single PR** (the narrow WS/ES slice
under the approved umbrella — base-case, §7), not a re-split.

### §3.2 User-input touch audit
User-controllable inputs: the page's `addEventListener(type, …)` + `on<type>` assignments
(`onopen`/`onmessage`/`onerror`/`onclose`) and WS/ES construction; for ES, an **inbound** server push
buffered for the conn (drives `has_queued_task`). The predicate reads the **listener type** (page-supplied)
but tests it against a **fixed spec enum** per state-tier (WS: {open,message,error,close}; ES:
{open,message,error}); an unrecognized type simply fails the test — **no injection surface**. WS keepalive
is a pure `(readyState, listener-type-set) → bool` (no `buffered_amount` — data-queued clause dropped,
§2.3 F1); ES keepalive is `(readyState, has_queued_task, listener-type-set) → bool` where `has_queued_task`
is a buffer-peek boolean, not page-controlled content — so it opens **no new trust boundary** (cf.
umbrella §3.1).

---

## §4 The ideal — extend the seam (mechanism design)

### §4.1 Where it hooks (same seam as S5-3a — `&VmInner`, NOT `GcRoots`)
S5-3a's actual implementation reads `&VmInner` directly in `keepalive_survivors`
(`keepalive.rs:141`), reusing `vm_path_has_listener` rather than threading state maps into the `GcRoots`
snapshot. **This SUPERSEDES the parent S5-3 §4.1/§6 "borrow the maps into `GcRoots`" framing** — the
landed seam took the cleaner `&VmInner` route, and S5-3b follows it. `websocket_states` /
`event_source_states` are `pub(crate)` fields on `HostData` (`host_data.rs:466`/`:488`), reachable from
`gc/keepalive.rs` as `vm.host_data.as_deref().map(|hd| &hd.websocket_states)` — the **same `&VmInner`
borrow** the MQL arm already uses for `document_entity_opt`. **No `GcRoots` change, no new side-store.**

### §4.2 The marshalling layer (seam MARSHALS, engine-indep crate RULES)
Two new `KeepaliveClass` arms — `KeepaliveClass::WebSocket` (NEW) and `KeepaliveClass::EventSource`
(NEW). Each **marshals** and **delegates the spec rule** to `elidex-api-ws` (§4.4). Post-Codex-R1
(§2.3): **WS marshals readyState ONLY** (no `buffered_amount` — the data-queued clause is dropped as
vacuous/F1); **ES marshals readyState + `has_queued_task`** (peeked from the NetworkHandle buffer for
this ES's `conn_id`). Sketch (impl owns exact form):

```rust
// keepalive.rs — KeepaliveClass enum gains WebSocket, EventSource
fn keepalive(self, vm: &VmInner, target: ObjectId) -> bool {
    match self {
        KeepaliveClass::MediaQueryList => vm_path_has_listener(vm, target, "change", false),
        KeepaliveClass::WebSocket => {
            let Some(st) = vm.host_data.as_deref().and_then(|hd| hd.websocket_states.get(&target))
                else { return false };
            // PURE tier check — no data-queued disjunct (§2.3 F1: buffered_amount over-roots CLOSING)
            elidex_api_ws::ws_keepalive(
                st.ready_state,
                |t| vm_path_has_listener(vm, target, t, false),
            )
        }
        KeepaliveClass::EventSource => {
            let Some(hd) = vm.host_data.as_deref() else { return false };
            let Some(st) = hd.event_source_states.get(&target) else { return false };
            // has_queued_task: is an inbound SSE event buffered for this conn awaiting drain? (§2.3 F3)
            // NetworkHandle peek scans the `buffered` Vec WITHOUT draining (new elidex-net method).
            let has_queued_task = hd
                .network_handle_ref()                       // engine-side marshalling only
                .has_pending_event_for_conn(st.conn_id);
            elidex_api_ws::es_keepalive(
                st.ready_state,
                has_queued_task,
                |t| vm_path_has_listener(vm, target, t, false),
            )
        }
    }
}
```

**`has_pending_event_for_conn(conn_id) -> bool` is a NEW `elidex-net` deliverable** (a NetworkHandle peek
that **settles the channel into the buffer — same routing as a drain — then scans** for a pending
`EventSourceEvent(conn_id)`, covering the full inbound pipeline (channel + buffer) so a GC before the
first drain doesn't miss a channel-pending event (Codex R2a) — `conn_id`
read from `event_source_states[target].conn_id`). The buffer-peek is **engine-side marshalling** in the
seam (it reads VM/net-handle state); the **tier rule** stays in `elidex-api-ws` (§4.4). This preserves
the layering point: the seam decides *whether a task is queued for this conn* (a marshalled bool), the
engine-indep rule decides *what that bool means for keepalive* (short-circuit true, else tier).

**Two impl-guarantees the seam relies on (plan-review clarity):**
1. **Row existence** — the per-id `keepalive` calls read `event_source_states[target]` (and
   `websocket_states[target]`), which are guaranteed present: `keepalive_survivors` iterates the map's
   own keys and marks survivors **before** the sweep (`collect.rs:1233-1238` before `:1891-1935`) that
   prunes rows — so every `target` in the loop still has its row at read time (no `None`-branch reachable
   in practice; the `else { return false }` is a belt-and-suspenders guard).
2. **Buffer-peek borrow** — `has_pending_event_for_conn` takes an **immutable** borrow of the NetworkHandle
   `buffered` cell (`RefCell::borrow`, read-only scan, no drain). GC runs synchronously on the single VM
   thread with no concurrent buffer mutation in flight, so the immutable borrow cannot conflict; the peek
   is a pure `.iter().any(matches conn_id)` — no panic risk, no state change.

And the registrant loops in `keepalive_survivors` (collect keys first to keep the `host_data` borrow
disjoint from the per-id `keepalive` calls — both immutable, but collecting avoids any
iterator-vs-closure borrow friction):

```rust
if let Some(hd) = vm.host_data.as_deref() {
    let ws_ids: Vec<ObjectId> = hd.websocket_states.keys().copied().collect();
    keep.extend(ws_ids.into_iter().filter(|&id| KeepaliveClass::WebSocket.keepalive(vm, id)));
    let es_ids: Vec<ObjectId> = hd.event_source_states.keys().copied().collect();
    keep.extend(es_ids.into_iter().filter(|&id| KeepaliveClass::EventSource.keepalive(vm, id)));
}
```

**The listener test** reuses `vm_path_has_listener(vm, target, type, false)`
(`event_target_dispatch_vm.rs:79`) — the dispatch-time SSoT, so *kept-alive ⇔ would-actually-fire*: it
counts a typed `addEventListener` **or** a live `on<type>` handler, and EXCLUDES a cleared
`on<type> = null` (whose callable is retired from `listener_store`). The `bubbles` arg is `false` (the
WS/ES is the target; only its own listeners matter — the depth-0 `is_target` branch counts them
regardless of bubbling). The engine-indep rule consumes the closure as `has_listener: impl Fn(&str) ->
bool` — it owns *which* types to test per state-tier, not *how* listeners are stored.

### §4.3 collect.rs needs NO edit — the sweep IS the else-branch
**The force-close path (`collect.rs:1891-1935`) is unchanged.** The keepalive mark (`:1233-1238`) runs
**before** the sweep, and the sweep already computes `dead_ws_conns` / `dead_sse_conns` purely from the
mark bit (`bit_get(marks, obj_id.0)`). So a predicate-marked wrapper has its bit set ⇒ it is **not** in
the dead set ⇒ its row is **retained** and **no Close is emitted**. The un-marked orphan (predicate
false) ⇒ swept ⇒ force-closed, exactly as today. The behavior change is achieved **entirely by adding
the keepalive marks**; the existing unconditional-force-close becomes the *de-facto* else-branch via the
mark bit. (Verify at impl: confirm no other path force-closes independent of the mark bit.)

### §4.4 Layering — `elidex-api-ws` owns the spec rule
Per CLAUDE.md "VM host/ は engine-bound 責務のみ" / "新規 algorithm を host/ に書く前に
engine-independent crate を確認": the tier table — WS `(readyState, listener-presence) → bool`, ES
`(readyState, has_queued_task, listener-presence) → bool` — is a **spec/domain algorithm**, not
engine-bound marshalling, so it lives in **`elidex-api-ws`** (which
already owns `WsReadyState` / `SseReadyState` and the spec helpers `validate_ws_url` / `normalize_ws_url`
/ `is_mixed_content`; already a dep of `elidex-js`). New engine-indep fns (no `&VmInner`, fully
unit-testable):

```rust
// elidex-api-ws/src/websocket.rs
// PURE tier check — the §7 OUTBOUND "data queued" clause is OMITTED (vacuous in elidex, §2.3 F1):
// outbound bytes are broker-owned FIFO once emitted, and buffered_amount over-roots CLOSING.
pub fn ws_keepalive(state: WsReadyState, has_listener: impl Fn(&str) -> bool) -> bool {
    use WsReadyState::*;
    match state {
        Connecting => ["open", "message", "error", "close"].iter().any(|t| has_listener(t)),
        Open       => ["message", "error", "close"].iter().any(|t| has_listener(t)),
        Closing    => ["error", "close"].iter().any(|t| has_listener(t)),
        Closed     => false,
    }
}

// elidex-api-ws/src/event_source.rs
// The §9.2.9 INBOUND "task queued on the remote event task source" clause IS included (meaningful in
// elidex, §2.3 F3): a buffered inbound event needs the wrapper alive at dispatch time to route.
pub fn es_keepalive(state: SseReadyState, has_queued_task: bool,
                    has_listener: impl Fn(&str) -> bool) -> bool {
    use SseReadyState::*;
    if has_queued_task { return true; }         // §9.2.9 no-listener clause — buffer-window root
    match state {
        Connecting => ["open", "message", "error"].iter().any(|t| has_listener(t)),
        Open       => ["message", "error"].iter().any(|t| has_listener(t)),
        Closed     => false,
    }
}
```

The seam owns only: read the state map, **peek the NetworkHandle buffer** to derive `has_queued_task`
(ES) via `has_pending_event_for_conn` (a new `elidex-net` method — §4.2), build the `has_listener`
closure, call the rule, `mark_object` survivors. **No SPEC-RULE branching in the seam.** These rules get
their **own engine-indep unit tests** in `elidex-api-ws` (every tier branch + CLOSED-never for both; the
`has_queued_task ⇒ true` short-circuit for ES), independent of the VM (§9). Note `ws_keepalive` **loses**
the `has_queued_data` parameter and its clause; `es_keepalive` **gains** the `has_queued_task` parameter
and its short-circuit — this is the §2.3 SWAP realized in the signatures.

---

## §5 Per-class predicate detail (spec § + elidex predicate + wiring site)

| Class | Spec § (webref) | elidex keepalive predicate | Wiring site | Replaces / fixes |
|---|---|---|---|---|
| **WebSocket** | WebSockets §7 (`websockets#garbage-collection`) | **tier ONLY**: `state ∈ {CONNECTING,OPEN,CLOSING}` with tiered subset {CONNECTING:open/message/error/close; OPEN:message/error/close; CLOSING:error/close}. **data-queued clause OMITTED** (vacuous, §2.3 F1) | **rule** `elidex_api_ws::ws_keepalive(state, has_listener)` (NEW, no `has_queued_data`) in `websocket.rs`; **seam** `KeepaliveClass::WebSocket` reads `websocket_states` (`host_data.rs:466`) for `ready_state` only, builds `has_listener` over `vm_path_has_listener`, calls the rule | **suppresses the force-close** (`collect.rs:1891-1935`) for listener-held conns; orphan / listener-less CLOSING still force-closes (F1 guard) |
| **EventSource** | HTML §9.2.9 (`html#garbage-collection`) | **tier OR has_queued_task**: `state ∈ {CONNECTING,OPEN}` with tiered subset {CONNECTING:open/message/error; OPEN:message/error} **OR** an inbound event is buffered for this conn (§2.3 F3) | **rule** `elidex_api_ws::es_keepalive(state, has_queued_task, has_listener)` (NEW) in `event_source.rs`; **seam** `KeepaliveClass::EventSource` reads `event_source_states` (`host_data.rs:488`) for `ready_state`+`conn_id`, peeks the NetworkHandle buffer via `has_pending_event_for_conn(conn_id)` (NEW `elidex-net` method) for `has_queued_task`, builds the closure, calls the rule | suppresses force-close for listener-held **and buffer-window** conns; genuine orphan still force-closes |

Event-type strings (verified): WS handlers `["onopen","onmessage","onerror","onclose"]` →
`"open"/"message"/"error"/"close"` (`websocket.rs:221`; `ws_open_event_type => "open"`
`well_known.rs:1411`); ES handlers `["onopen","onmessage","onerror"]` → `"open"/"message"/"error"`
(`event_source.rs:154`). Both register into `vm_event_listeners` (the home `vm_path_has_listener` reads).

### §5.1 Stale-comment refresh (an S5-3b deliverable, not a side-effect)
The seam doc-comments already forward-reference the WS/ES arm as *future*; S5-3b's deliverables include
flipping them to *landed*:
1. `keepalive.rs:38-41` ("a future `WebSocket`/`EventSource` arm marshals VM state and delegates its
   tier rule to `elidex-api-ws` … (S5-3b/c)") → reframe as landed.
2. `keepalive.rs:63-69` (the `KeepaliveClass` doc: "The remaining non-Node EventTargets migrate … before
   the S5-6 flip … `WebSocket` / `EventSource` (state-tiered listener subset, S5-3b …)") → mark WS/ES
   done; observer (S5-3c) remains.
3. The `collect.rs:1875-1886` force-close comment → note that the unconditional close is now the
   **else-branch** of the keepalive predicate (orphan / CLOSED only); listener-held conns (WS/ES) and
   buffer-window ES conns (`has_queued_task`) survive via `keepalive_survivors`.

---

## §6 ECS-native lens + B1 home constraint

The rooted thing is a per-VM `ObjectId` (the WS/ES wrapper). Under CLAUDE.md's side-store→component
rule it is the **per-VM-identity-handle exception (a)**: the value is `Send` (`ObjectId(u32)`) but its
meaning is per-VM, and both state maps are `unbind`-cleared (`drain_realtime_for_unbind`,
`host_data.rs:1809-1819`) — the canonical exception-(a) lifecycle. The ideal ECS-native form (a
keepalive marker-**component** on the watched entity) does **not even apply** here: WS/ES are **non-Node**
EventTargets with **no entity**, only an `ObjectId`. So S5-3b lands the **per-VM HostData/registry +
predicate form** (the same per-VM-now / component-later pattern S5-2 and S5-3a used).

The component-migration ideal is tracked by the **existing** slot
`#11-eventtarget-keepalive-component-migration` (S5-3 §10), now **B1-gated** (agent-scoped `EcsDom`
World, PR #434 — `world_id` SUPERSEDED): under 1-agent = 1-World per-entity identity is stable, so the
marker-component becomes safe without a discriminator. S5-3b adds WS/ES as new registrants under that
*same* deferred slot — **no new component owed pre-B1, no new slot for the home question**.

**ECS axis confirmation for plan-review**: the predicate reads per-VM EventTarget state (a per-VM /
browsing-context-level fact, exception (a)), not a per-entity DOM fact mis-stored in a side-store.

---

## §7 Scope (single PR, base-case — plan-review confirm)

S5-3b is a **single PR**: the narrow WS/ES arm under the approved S5 umbrella + the S5-3 §7 split, having
passed `/elidex-plan-review` = a **terminal base-case** (CLAUDE.md base-case rule: a narrowly-scoped
per-PR slice under an approved umbrella + plan-review is an allowed single PR; the slice touching the
same subsystem is **not** a re-split trigger). It is edge-dense (touches the behavior-bearing
force-close path) — which is why it gets this plan-review, **not** why it must split further. No prereq
split is owed: `keepalive.rs` (176 LoC), `elidex-api-ws/websocket.rs` (227), `event_source.rs` (tiny)
are all well under the 1000-line touch-time threshold; `collect.rs` is large but **untouched** (§4.3).
The Codex-R1 correction adds **one small `elidex-net` deliverable** — the NetworkHandle
`has_pending_event_for_conn(conn_id) -> bool` buffer-peek (§4.2, settles the channel into the buffer —
same routing as a drain — then scans, covering the full inbound pipeline (Codex R2a)) — a bounded
additive method that does not change the single-PR base-case verdict (confirm the touch
site is under the touch-time threshold at impl).

---

## §8 Edge matrix (review-tail pre-empt)

| Invariant axis | WebSocket | EventSource |
|---|---|---|
| **GC-rooting (seam mark)** | ✔ `ws_keepalive` marks survivor in `keepalive_survivors` | ✔ `es_keepalive` marks survivor |
| **listener-lifecycle (type-restricted)** | tiered subset per state (open/message/error/close) | tiered subset per state (open/message/error) |
| **per-class predicate (engine-indep)** | `elidex_api_ws::ws_keepalive` | `elidex_api_ws::es_keepalive` |
| **active-state** | `WsReadyState ∈ {CONNECTING,OPEN,CLOSING}` | `SseReadyState ∈ {CONNECTING,OPEN}` |
| **in-flight-work (no-listener clause)** | data-queued = **VACUOUS** → **OMITTED** (outbound broker-FIFO; `buffered_amount` over-roots CLOSING = F1, §2.3) | task-queued = **MEANINGFUL** → **INCLUDED** via `has_queued_task` (inbound buffer window, reverse-map-miss drop = F3, §2.3) |
| **force-close interaction** | marked ⇒ survives sweep, no Close; orphan/CLOSED/listener-less-CLOSING ⇒ force-close (unchanged) | marked (listener OR queued task) ⇒ survives; orphan ⇒ force-close (unchanged) |
| **unbind-lifecycle (per-VM)** | `websocket_states` drained + Close emitted for ALL conns on unbind | `event_source_states` drained + Close emitted |
| **behavior-change** | **YES** — listener-held open conn must NOT force-close (data-queued NOT a keepalive input, §2.3 F1) | **YES** — listener-held OR buffer-window (`has_queued_task`) open conn must NOT force-close |
| **B1-home (component defer)** | exception (a) per-VM now → component after B1 | exception (a) per-VM now → component after B1 |

**Cross-cutting edges plan-review must scrutinize:**
1. **seam × force-close boundary (densest).** The predicate must mark a listener-held conn (WS/ES) or a
   buffer-window ES conn (`has_queued_task`) ALIVE **and** the (post-flip) message pump must keep
   delivering — but the no-listener / no-queued-task orphan must still force-close. Getting it wrong
   either **leaks network threads** (over-keep: a no-listener OPEN socket kept forever — or, per F1, a
   listener-less CLOSING socket rooted by a stale `buffered_amount` if the dropped clause were reinstated)
   or **drops live deliveries** (under-keep: a listener-held socket closed, or an ES with a buffered
   inbound event collected → F3 silent drop). The tier table (+ ES `has_queued_task` short-circuit) is
   the exact boundary; §9 tests both directions.
2. **predicate × `on<type>` handler** (`event_target_dispatch_vm.rs:93`): the type test must count
   `ws.onmessage = cb` / `es.onmessage = cb` with no `addEventListener` (else an on-handler-only page's
   socket is wrongly collected). `vm_path_has_listener` does (verified); test the handler-only path.
3. **CLOSED / CLOSING tier correctness.** A CLOSED WS/ES is **never** kept (else immortal closed
   wrappers leak); a CLOSING WS is kept **only** with an {error,close} listener — **NOT** on buffered
   data (the data-queued clause is dropped, §2.3 F1; a listener-less CLOSING WS with `buffered_amount>0`
   is **collected**, the F1 guard). Test: an OPEN WS with **only** an `open` listener (not in the OPEN
   tier {message,error,close}) is **collected** — proving the predicate is tiered, not any-listener.
4. **unbind force-close vs GC keepalive** (`vm_api.rs:420-461` / `host_data.rs:1809-1819`): on `unbind`,
   `drain_realtime_for_unbind` force-closes **every** conn (even listener-held). This is **correct and
   unchanged** — it is the spec's "Document object goes away ⇒ make disappear (WS) / forcibly close
   (ES)" rule (WebSockets §7 1001 close; HTML §9.2.9 abort fetch + set CLOSED), a **distinct** rule from
   GC keepalive. Confirm plan-review does not read unbind-closing a listener-held conn as a keepalive
   regression.
5. **WS outbound FIFO makes data-queued VACUOUS; `buffered_amount` over-roots CLOSING = F1** (§2.0 fact 5
   / §2.3): outbound bytes are broker-owned FIFO once `send()` emits (`websocket.rs:847`), transmitted
   ahead of any GC-emitted `WebSocketClose` regardless of wrapper survival — so keeping the wrapper alive
   on queued data protects nothing. WORSE, `buffered_amount` is incremented **unconditionally**
   (`websocket.rs:836`, incl. CLOSING/CLOSED sends that never transmit and never clear via `BytesSent`),
   so a `buffered_amount`-keyed keepalive would root a **listener-less CLOSING socket forever** (indefinite
   leak, **Codex F1**). Confirm the clause is **OMITTED** and `ws_keepalive` is a pure tier check.
6. **ES task-queued MEANINGFUL; buffer-window silent-drop = F3, must keep alive** (§2.0 facts 1-4 / §2.3):
   an inbound SSE event buffers between `drain_fetch_responses_only` and `drain_events`
   (`fetch_tick.rs:~58/~78`); a mid-turn GC in that window that collects a wrapper whose only listener is
   a **named** event (not in tier `{message,error}`) prunes `event_source_states` + `sse_conn_to_object`
   (`collect.rs:1919-1932`), and the later-drained buffered event hits a reverse-map miss
   (`fetch_tick.rs:169-176`, `else { return; }`) and is **silently dropped** (**Codex F3**). Confirm the
   `has_queued_task` clause is **INCLUDED** and roots the wrapper across the buffer window.
7. **buffer-window × reverse-map core (the ES-critical invariant)**: the dispatch target is rooted in the
   buffer window **only** by the keepalive predicate — `gc/roots.rs` roots the Event object, not the
   target (§2.0 fact 4). So `es_keepalive(has_queued_task=true)` is the sole GC root standing between a
   buffered inbound event and a silent-drop. Getting this wrong = data loss for every named-event SSE
   consumer on any allocation-triggered GC (which fires mid-`tick_network`, §2.0 fact 1).
8. **WS §7 "readyState as of event-loop step 1" letter-gap = F2** (`keepalive.rs:141`): WS §7 keys the
   listener tiers to the readyState "as of the last time the event loop reached step 1"; elidex reads the
   **LIVE current** `ready_state` (`WebSocketState` has only the live field — no per-turn snapshot). Given
   mid-turn GC (§2.0 fact 1) this letter-divergence is **real and constructible**. But it has **no
   demonstrated observable consequence**: (a) the dispatch target is `this`-rooted during its own handler
   (§2.0 fact 4) so a mid-handler GC cannot collect the socket being dispatched to; (b) after the `open`
   handler returns, an `open`-only-listener OPEN socket has no deliverable events (open fires once; no
   message/error/close listener) so collecting it loses nothing script-observable. **Disposition:
   documented accepted letter-gap + low-priority slot `#11-keepalive-event-loop-step1-snapshot`** (a
   cross-cutting per-turn readyState-snapshot mechanism affecting ALL keepalive arms, not just WS) IF
   elidex later wants strict §7 letter-conformance — **NOT a this-PR fix**. Plan-review: confirm the
   no-observable-break reasoning.

---

## §9 Test strategy (VM-test oracle — boa is the live engine)

S5-3b is exercised by **VM tests** (`elidex-js` `engine`-feature suite) + **engine-indep unit tests**
(`elidex-api-ws`). Test infra (from S5-3a `tests_match_media_keepalive.rs` + existing
`tests_websocket.rs` / `tests_event_source.rs`): `with_bound_vm(|vm| …)`, `vm.inner.collect_garbage()`
to force GC, `inject_ws_event_and_tick` / SSE inject helpers to drive readyState (Connected ⇒ OPEN) and
deliver messages.

**Engine-indep unit tests (`elidex-api-ws`, pure):**
- `ws_keepalive` every branch: each state × in/out-of-tier listener; CLOSED-never. **No data-queued
  cases** — the parameter is removed (§2.3 F1); a listener-less CLOSING WS ⇒ `false` (collected), which
  is the F1 guard at the unit level.
- `es_keepalive` every branch: each state × in/out-of-tier listener; CLOSED-never. **New `has_queued_task`
  cases**: `has_queued_task=true` ⇒ `true` for a **non-CLOSED** state incl. a no-tier-listener
  OPEN/CONNECTING (the buffer-window short-circuit, §2.3 F3), but **CLOSED is never kept even with a
  queued task** (Codex R2b-A: a CLOSED source's buffered events are dropped by dispatch, so rooting it is
  a pure leak); `has_queued_task=false` ⇒ falls through to the tier check.

**VM tests (the decisive behavior):**
- **WS keepalive (headline, positive):** `new WebSocket(u); ws.onmessage = cb;` drive to OPEN, **drop
  the reference**, force GC → assert the wrapper survives (`websocket_states` row retained) **and no
  `WebSocketClose` was emitted** to the broker, then inject a `message` → assert `cb` fired.
- **WS negative control (no over-rooting):** OPEN WS with **no listener** and no buffered data, no
  reference → GC → assert row pruned **and** `WebSocketClose` emitted (genuine orphan still force-closed).
- **WS tier (not any-listener):** OPEN WS with **only** an `open` listener (out of the OPEN tier) → GC →
  assert collected + closed (proves tier, not any-listener).
- **WS CLOSING-no-leak (F1 regression guard):** a **CLOSING** WS with **no** listener, then a post-close
  `send()` that bumps `buffered_amount > 0` (unconditional increment, never transmitted, never cleared),
  no reference → GC → assert **COLLECTED + closed** (i.e. NOT rooted by the now-removed data-queued
  clause). This is the guard that the F1 indefinite-leak cannot recur.
- **WS CLOSED never kept:** a CLOSED WS (even with a `close` listener) → GC → assert collected.
- **WS `on*`-only path:** handler-only `ws.onmessage = cb` (no `addEventListener`) survives + delivers.
- **ES task-queued (F3 regression, headline for ES):** an OPEN ES whose **only** listener is a NAMED
  event (`es.addEventListener('foo', cb)` — NOT in the ES tier `{message,error}`), buffer a `'foo'`
  `EventSourceEvent` for its conn in the NetworkHandle, then **force GC in the buffer window** (before the
  event is drained) → assert the ES **SURVIVES** (row retained, no `EventSourceClose`) **and** the
  buffered `'foo'` event is subsequently **DELIVERED** to `cb`, NOT silently dropped. (Drives
  `has_pending_event_for_conn` → `has_queued_task=true` → keepalive true.)
- **ES no-queued-task control:** the same named-event-only OPEN ES with **no** buffered event
  (`has_queued_task=false`) → GC → assert **collected + `EventSourceClose` emitted** (the tier does not
  cover `'foo'`, and no task is queued, so it is a genuine orphan).
- **ES tier / mirror set:** listener-held (in-tier `message`/`error`) OPEN ES survives + keeps
  delivering; no-listener OPEN ES collected + `EventSourceClose` emitted; OPEN ES with only `open`
  listener collected (out of ES OPEN tier {message,error}); CLOSED ES collected.
- **unbind force-close (regression guard):** a listener-held OPEN WS/ES is force-closed on `Vm::unbind`
  (document-teardown rule, §8.4) — assert Close emitted on unbind even though GC would keep it.

(Asserting "no Close emitted" requires capturing `RendererToNetwork` messages — the existing WS/ES
dispatch tests already use a fake broker via `inject_ws_event_and_tick`; confirm the harness exposes the
sent-message log, else extend it.)

**Out of S5-3b (rides S5-6):** nothing — S5-3b is pure VM capability + tests. The message pumps it
protects are VM-resident; S5-3b only ensures their targets survive to be delivered to.

---

## §10 Deferred slots + open questions (per-PR cap ≤3)

### Slots (two parent-carved to REGISTER + one NEW low-priority from Codex F2 — see reconciliation)
The two parent-carved slots below were **carved as NEW by the parent S5-3 §10** but **never landed in the
canonical registry** `memory/project_open-defer-slots.md` — S5-3a (#430) registered only the *predecessor*
`#11-eventtarget-listener-keepalive-rooting`. So the registry currently has the predecessor (still
carrying the **refuted** "GENERIC EventTarget alive while listenered" any-listener framing, §2) and
**not** these two. They are program-level slots (not new S5-3b scope); registering them is a ledger
catch-up. S5-3b additionally carves **one NEW low-priority slot** from Codex F2
(`#11-keepalive-event-loop-step1-snapshot`, the per-turn readyState-snapshot letter-gap, §8 edge 8) —
a single new defer, within the ≤3 per-PR cap. **S5-3b owns the reconciliation** (deliverable below).

- **`#11-eventtarget-keepalive-component-migration`** (carved by parent §10, **registry-absent**,
  B1-gated) — S5-3b adds WS/ES as new per-VM HostData registrants; the component-on-entity ideal stays
  deferred to the B1 program (§6).
- **`#11-eventtarget-keepalive-registrant-coverage`** (carved by parent §10, **registry-absent**, HARD
  pre-flip gate) — S5-3b **satisfies the WS/ES portion**. After S5-3b, only **S5-3c (observers)**
  remains off the seam; the gate stays open until S5-3c lands (before S5-6).
- **`#11-keepalive-event-loop-step1-snapshot`** (NEW, low-priority, carved by Codex F2 / §8 edge 8) — WS
  §7 (and analogously all keepalive arms) key the listener tiers to "readyState as of the last time the
  event loop reached step 1"; elidex reads the LIVE current readyState (no per-turn snapshot). Given
  mid-turn GC (§2.0 fact 1) this is a **real but observably-inert letter-gap** (§8 edge 8). The
  cross-cutting per-turn readyState-snapshot mechanism (which would affect **all** keepalive arms, not
  just WS) is deferred to this slot IF elidex later wants strict §7 letter-conformance. **NOT a this-PR
  fix** — documented accepted gap. **Defer triplet** — *Why deferred*: no observable break (target is
  `this`-rooted during its handler; an `open`-only OPEN socket delivers nothing post-open, §8 edge 8), and
  the mechanism is cross-cutting (all arms) so out of this WS/ES slice; *Re-evaluation trigger*: a
  production/WPT case where a cross-GC readyState change is script-observable, OR the S5-6 flip's
  spec-conformance pass electing strict §7 letter-fidelity; *Re-evaluation date*: **at the S5-6 flip
  conformance review** (the first point the VM keepalive path is live and letter-conformance is testable).
- **ES task-queued clause** — **NOT deferred; INCLUDED in S5-3b** (§2.3 SWAP). Codex R1 refuted the
  original "vacuous under inline drain" premise: the inbound event **is** buffered across a mid-turn GC
  window (§2.0 facts 1-4), so omitting the clause would silently drop named-event SSE deliveries (F3).
  The clause is therefore **implemented** via `es_keepalive(has_queued_task, …)` + the NetworkHandle peek
  — no slot, because the work is done in this PR, not postponed.

### Defer-ledger reconciliation (an S5-3b landing deliverable, not a side-effect)
At S5-3b landing (in the landing-memo / `project_open-defer-slots.md` update — the slot-registration
convention point, per `feedback_defer-ledger-philosophy-lens`):
1. **Register** `#11-eventtarget-keepalive-registrant-coverage` (active HARD pre-flip gate, now tracking
   **S5-3c** — observers must land before S5-6), `#11-eventtarget-keepalive-component-migration`
   (B1-gated), and **`#11-keepalive-event-loop-step1-snapshot`** (NEW, low-priority, Codex F2 — per-turn
   readyState-snapshot for strict §7 letter-conformance across all keepalive arms) in
   `project_open-defer-slots.md`.
2. **Reframe + retire** the predecessor `#11-eventtarget-listener-keepalive-rooting` slot text — its
   "GENERIC 'EventTarget alive while listenered'" framing is **refuted** by §2 (the parent §12 named
   this an S5-3a deliverable; S5-3a left it undone). Reframe to the **keepalive-predicate seam**
   (per-registrant spec-faithful predicate), and mark it **superseded by the S5-3a/b/c program** (MQL +
   AbortSignal.timeout in S5-3a, WS/ES in S5-3b, observers in S5-3c).

### Open questions for `/elidex-plan-review`
- **Q1 (the tiers — the spine):** Confirm the WS tier {CONNECTING:open/message/error/close;
  OPEN:message/error/close; CLOSING:error/close; CLOSED:none} (**data-queued clause OMITTED**, §2.3 F1),
  and the ES tier {CONNECTING:open/message/error; OPEN:message/error; CLOSED:none} **OR has_queued_task**
  (§2.3 F3), are the spec-faithful rules (§2.1/§2.2 webref prose + §2.0 delivery mechanics). Lean:
  **yes** (verbatim from §7 / §9.2.9, with the corrected no-listener-clause directionality).
- **Q2 (WS data-queued clause OMITTED — vacuous + F1):** **DECIDED post-Codex-R1** (§2.3): the §7
  OUTBOUND data-queued clause is **omitted** because (a) outbound bytes are broker-owned FIFO once
  emitted so the wrapper protects nothing (§2.0 fact 5), and (b) `buffered_amount` is incremented
  unconditionally (incl. CLOSING/CLOSED never-transmitted sends) so keying keepalive on it would
  **over-root a listener-less CLOSING socket into an indefinite leak** (Codex F1). `ws_keepalive` is a
  pure tier check with no `has_queued_data` parameter. Plan-review validates the vacuity + F1 reasoning.
- **Q3 (ES task-queued clause INCLUDED — meaningful, F3):** **DECIDED post-Codex-R1** (§2.3): the §9.2.9
  INBOUND task-queued clause is **included** because an inbound SSE event is **buffered across a mid-turn
  GC window** (§2.0 facts 1-4); omitting it would let a GC collect a named-event-only wrapper, prune the
  reverse-map, and **silently drop** the later-drained event (Codex F3). Implemented via
  `es_keepalive(has_queued_task, …)` + the NetworkHandle peek. Plan-review validates the buffer-window
  premise + that the peek correctly detects a pending `EventSourceEvent(conn_id)`.
- **Q4 (collect.rs no-edit):** Confirm the keepalive mark (`collect.rs:1233-1238`, before the sweep at
  `:1891-1935`) makes the existing unconditional force-close the **de-facto else-branch** with **no
  edit** to the sweep — and that no other path force-closes independent of the mark bit. Lean: **yes**.
- **Q5 (layering home):** Confirm `ws_keepalive` / `es_keepalive` belong in engine-indep `elidex-api-ws`
  (with `WsReadyState` / `SseReadyState` + the spec helpers), the seam only marshalling. Lean: **yes**
  (CLAUDE.md layering mandate; the rule is a spec/domain algorithm).
- **Q5b (NetworkHandle peek layering):** Confirm the **new `has_pending_event_for_conn(conn_id) -> bool`
  peek method** belongs in `elidex-net` (NetworkHandle), and that invoking it in the seam to marshal
  `has_queued_task` is **engine-side marshalling** (reads net-handle buffer state), NOT a spec-rule leak
  into `host/` — the tier/short-circuit rule stays in `elidex-api-ws`, the seam only supplies the
  marshalled bool (§4.2). Lean: **yes** (the peek is a net-transport query; the keepalive semantics of
  the bool live in the engine-indep rule).
- **Q6 (unbind force-close not a regression):** Confirm unbind force-closing **even a listener-held**
  conn (`drain_realtime_for_unbind`) is the spec's document-teardown forcible-close (distinct from GC
  keepalive), correct and unchanged. Lean: **yes**.
- **Q7 (F2 letter-gap disposition):** Confirm the WS §7 "readyState as of event-loop step 1" letter-gap
  (elidex reads live readyState, no per-turn snapshot — §8 edge 8) has **no observable break** given (a)
  the dispatch target is `this`-rooted during its own handler (§2.0 fact 4) and (b) an `open`-only OPEN
  socket has no post-`open` deliverable events. Disposition: **documented accepted gap** + low-priority
  slot `#11-keepalive-event-loop-step1-snapshot` (cross-cutting per-turn snapshot for ALL arms). Plan-
  review confirms the no-observable-break reasoning is sound and that this is correctly NOT a this-PR fix.

---

## §11 Verified-cites note (read before plan-review)

Spec prose webref-verified 2026-07-01: WebSockets §7 (`websockets#garbage-collection`), HTML §9.2.9
(`html#garbage-collection`, server-sent-events), DOM §2.8 (`dom#observing-event-listeners`). Code cites
grep-verified against `main` `3345949e`: `keepalive.rs:38-41`/`:63-69`/`:141`/`:162-167`,
`collect.rs:1233-1238`/`:1315`/`:1891-1935`, `host_data.rs:466`/`:488`/`:601`/`:608-612`/`:673-707`/
`:700-706`/`:1809-1819`, `websocket.rs:184-185`/`:221`/`:419`/`:630`/`:836`,
`websocket_dispatch.rs:229`, `event_source.rs:154`, `event_source_dispatch.rs` (`dispatch_sse_event` —
runs inline **once drained**, but the event is buffered BEFORE the drain, §2.0 facts 2-3),
`event_target_dispatch_vm.rs:79-94`, `vm_api.rs:420-461`, `well_known.rs:1411`,
`elidex-api-ws/{lib,websocket,event_source}.rs`.

**Corrected delivery-model cites (§2.0, verified this session, Codex R1 substrate):**
`crates/script/elidex-js/src/vm/inner.rs:92` (allocation-triggered `collect_garbage()` — mid-turn GC,
fact 1); `crates/script/elidex-js/src/vm/host/fetch_tick.rs:~58` (`drain_fetch_responses_only`, allocates
via `create_response_from_net`) / `:~78` (`drain_events`) / `:169-176` (`sse_conn_to_object.get` →
`else { return; }` silent drop, facts 2-3); `crates/net/elidex-net/src/broker/buffered.rs:140` (non-fetch
events re-buffered, fact 2); `crates/script/elidex-js/src/vm/gc/collect.rs:1919-1932` (sweep prunes
`event_source_states` + `sse_conn_to_object`, fact 3); `crates/script/elidex-js/src/vm/gc/roots.rs`
(roots the Event object, NOT the dispatch target, fact 4); `crates/script/elidex-js/src/vm/host/
websocket.rs:836` (unconditional `buffered_amount` increment) / `:847` (OPEN-only `WebSocketSend` emit) /
`:843-846` (CLOSING/CLOSED keep-increment-no-transmit comment, fact 5 / F1);
`crates/net/elidex-net/src/broker/handle.rs:832` (single FIFO `crossbeam_channel::unbounded`);
`crates/net/elidex-net/src/broker/dispatch/mod.rs:69-75` (drained in enqueue order) / `:218-230`
(`WebSocketSend` + `WebSocketClose` → same per-conn `command_tx`, fact 5). New `elidex-net` deliverable:
`has_pending_event_for_conn(conn_id) -> bool` NetworkHandle peek (§4.2).

**One framing correction vs the parent S5-3 memo:** the parent §4.1/§6 proposed threading the state
maps into the `GcRoots` snapshot. The **landed** S5-3a seam instead reads `&VmInner` directly
(`keepalive.rs:141`), so S5-3b reads `host_data.websocket_states` / `event_source_states` via the same
`&VmInner` borrow — **no `GcRoots` change** (§4.1). The parent's GcRoots framing is superseded by the
landed mechanism.

---

## §12 Workflow

plan-verify grep against `3345949e` (done) → **`/elidex-plan-review` (this memo) BEFORE impl** → impl in
this worktree (`elidex-api-ws` rules + unit tests → seam arms → stale-comment refresh → VM tests) →
`/pre-push` (6-stage) → `/external-converge` (Codex) → squash merge. boa untouched (VM-internal). B1
component migration stays out (`#11-eventtarget-keepalive-component-migration`, deferred to the post-S5
B1 cohort). After S5-3b: only **S5-3c (observers)** remains for the
`#11-eventtarget-keepalive-registrant-coverage` hard pre-flip gate.
