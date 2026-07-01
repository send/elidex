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

### §2.3 The two no-listener clauses map ASYMMETRICALLY onto elidex state
This is the design substance unique to S5-3b (the WS/ES rows the parent §5 deferred to here):

- **WS `established ∧ data-queued` → IMPLEMENTABLE, include it.** `WebSocketState.buffered_amount`
  (`host_data.rs:601`) is a **faithful "data queued to be transmitted" signal**: incremented on JS
  `send()` (`websocket.rs:836`, `saturating_add(byte_len)`), decremented on broker `WsEvent::BytesSent`
  (`websocket_dispatch.rs:229`, `saturating_sub(n)`). "Established connection" = the connection exists
  and is not closed = `state ∈ {OPEN, CLOSING}` (CONNECTING is "not yet established"; CLOSED is gone).
  So the elidex clause = `state ∈ {OPEN, CLOSING} ∧ buffered_amount > 0`. **Include** (ideal over
  pragmatic: the state already exists, dropping buffered data on GC would violate §7).

- **ES `task-queued-on-remote-event-task-source` → VACUOUS in elidex's delivery model → correctly
  OMITTED (not a gap).** elidex has **no per-instance queued-task state** for SSE: `EventSourceState`
  (`host_data.rs:673-707`) tracks only `ready_state` / `url` / `origin_sid` / `with_credentials` /
  `last_event_id` / `conn_id` — **no pending-task counter**. Incoming server messages are **drained
  from the broker and dispatched INLINE** during the network tick (`event_source_dispatch.rs`
  `dispatch_sse_event`: `last_event_id` update then `fire_vm_message_event`, synchronously in the tick
  loop) — there is **no "task queued but not yet run" window** that could span a GC. The spec clause
  exists to keep a target alive across that window; elidex's inline drain has no such window, so the
  clause has **no elidex analogue**. A no-listener OPEN ES is therefore collectible (and force-closed) —
  which is **spec-correct** ("GC while open ⇒ abort fetch") because there is never a "no-listener but
  task-queued" state in elidex. **This is a plan-review confirmation point (Q3), not a deferred slot**
  (the clause is vacuous by construction, not unfinished). Should the SSE delivery model ever become a
  deferred-task queue, the clause would gain an analogue and be revisited — but that is speculative, so
  no slot is carved (defer-slot eligibility audit: no concrete trigger).

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
- **in-flight-work** — WS `buffered_amount > 0` (real); ES task-queued (vacuous).
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
- **per-class-predicate × active-state** — WS tier = readyState ∧ tiered-listener-subset ∨
  (established ∧ data-queued); ES tier = readyState ∧ tiered-listener-subset.
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
| WebSockets §7 GC | established ∧ data queued | no-listener clause | `ws_keepalive`: `state ∈ {OPEN,CLOSING} ∧ buffered_amount > 0` | ✓ | yes (`send()`) |
| WebSockets §7 GC | swept (orphan / CLOSED) | force-close handshake | `collect.rs:1891-1935` else-branch (**unchanged**) | ✓ | no |
| HTML §9.2.9 EventSource GC | CONNECTING | listener ∈ {open,message,error} | `es_keepalive` over `event_source_states` | ✓ | yes |
| HTML §9.2.9 EventSource GC | OPEN | listener ∈ {message,error} | `es_keepalive` | ✓ | yes |
| HTML §9.2.9 EventSource GC | CLOSED | never kept | `es_keepalive` returns false → swept | ✓ | no |
| HTML §9.2.9 EventSource GC | task queued on remote event task source | no-listener clause | **VACUOUS** — elidex inline-drain has no queued-task window (§2.3); no analogue, correctly omitted | ✓ | no |
| HTML §9.2.9 EventSource GC | swept (orphan / CLOSED) | force-close abort fetch | `collect.rs:1891-1935` else-branch (**unchanged**) | ✓ | no |
| DOM §2.8 Observing event listeners | general default | — | seam does NOT root on bare-listener / bare-OPEN presence | ✓ | yes |

### §3.1 Breadth + split verdict
**K = 2** specs (WebSockets, HTML/SSE); **M = 11** rows. This is the **WS/ES row-subset** the parent
S5-3 §3 table (K=6 / M=13) carved into its own slice — so the breadth here is bounded to two interfaces
sharing one mechanism (state-tier + listener-subset). S5-3b is a **single PR** (the narrow WS/ES slice
under the approved umbrella — base-case, §7), not a re-split.

### §3.2 User-input touch audit
User-controllable inputs: the page's `addEventListener(type, …)` + `on<type>` assignments
(`onopen`/`onmessage`/`onerror`/`onclose`), `send()` (drives `buffered_amount`), and WS/ES construction.
The predicate reads the **listener type** (page-supplied) but tests it against a **fixed spec enum** per
state-tier (WS: {open,message,error,close}; ES: {open,message,error}); an unrecognized type simply fails
the test — **no injection surface**. The keepalive is a pure
`(readyState, buffered_amount, listener-type-set) → bool` over per-VM state the page already produced,
so it opens **no new trust boundary** (cf. umbrella §3.1).

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
(NEW). Each **marshals** (reads the VM state map for `readyState` + `buffered_amount`, builds a
listener-presence closure) and **delegates the spec rule** to `elidex-api-ws` (§4.4). Sketch (impl owns
exact form):

```rust
// keepalive.rs — KeepaliveClass enum gains WebSocket, EventSource
fn keepalive(self, vm: &VmInner, target: ObjectId) -> bool {
    match self {
        KeepaliveClass::MediaQueryList => vm_path_has_listener(vm, target, "change", false),
        KeepaliveClass::WebSocket => {
            let Some(st) = vm.host_data.as_deref().and_then(|hd| hd.websocket_states.get(&target))
                else { return false };
            elidex_api_ws::ws_keepalive(
                st.ready_state,
                st.buffered_amount > 0,                    // established∧data-queued marshalled to bool
                |t| vm_path_has_listener(vm, target, t, false),
            )
        }
        KeepaliveClass::EventSource => {
            let Some(st) = vm.host_data.as_deref().and_then(|hd| hd.event_source_states.get(&target))
                else { return false };
            elidex_api_ws::es_keepalive(
                st.ready_state,
                |t| vm_path_has_listener(vm, target, t, false),
            )
        }
    }
}
```

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
engine-independent crate を確認": the `(readyState, in-flight, listener-presence) → bool` tier table is
a **spec/domain algorithm**, not engine-bound marshalling, so it lives in **`elidex-api-ws`** (which
already owns `WsReadyState` / `SseReadyState` and the spec helpers `validate_ws_url` / `normalize_ws_url`
/ `is_mixed_content`; already a dep of `elidex-js`). New engine-indep fns (no `&VmInner`, fully
unit-testable):

```rust
// elidex-api-ws/src/websocket.rs
pub fn ws_keepalive(state: WsReadyState, has_queued_data: bool,
                    has_listener: impl Fn(&str) -> bool) -> bool {
    use WsReadyState::*;
    if matches!(state, Open | Closing) && has_queued_data { return true; } // §7 no-listener clause
    match state {
        Connecting => ["open", "message", "error", "close"].iter().any(|t| has_listener(t)),
        Open       => ["message", "error", "close"].iter().any(|t| has_listener(t)),
        Closing    => ["error", "close"].iter().any(|t| has_listener(t)),
        Closed     => false,
    }
}

// elidex-api-ws/src/event_source.rs
pub fn es_keepalive(state: SseReadyState, has_listener: impl Fn(&str) -> bool) -> bool {
    use SseReadyState::*;
    match state {                       // no in-flight clause — task-queued is vacuous (§2.3)
        Connecting => ["open", "message", "error"].iter().any(|t| has_listener(t)),
        Open       => ["message", "error"].iter().any(|t| has_listener(t)),
        Closed     => false,
    }
}
```

The seam owns only: read the state map, derive `has_queued_data` from `buffered_amount`, build the
`has_listener` closure, call the rule, `mark_object` survivors. **No SPEC-RULE branching in the seam.**
These rules get their **own engine-indep unit tests** in `elidex-api-ws` (every tier branch + the
data-queued clause + CLOSED-never), independent of the VM (§9).

---

## §5 Per-class predicate detail (spec § + elidex predicate + wiring site)

| Class | Spec § (webref) | elidex keepalive predicate | Wiring site | Replaces / fixes |
|---|---|---|---|---|
| **WebSocket** | WebSockets §7 (`websockets#garbage-collection`) | `state ∈ {CONNECTING,OPEN,CLOSING}` with tiered subset {CONNECTING:open/message/error/close; OPEN:message/error/close; CLOSING:error/close} **OR** `state ∈ {OPEN,CLOSING} ∧ buffered_amount > 0` | **rule** `elidex_api_ws::ws_keepalive` (NEW) in `websocket.rs`; **seam** `KeepaliveClass::WebSocket` reads `websocket_states` (`host_data.rs:466`, holds `ready_state`+`buffered_amount`+`conn_id`), builds `has_listener` over `vm_path_has_listener`, calls the rule | **suppresses the force-close** (`collect.rs:1891-1935`) for listener-held / data-queued conns; orphan still force-closes |
| **EventSource** | HTML §9.2.9 (`html#garbage-collection`) | `state ∈ {CONNECTING,OPEN}` with tiered subset {CONNECTING:open/message/error; OPEN:message/error}; **task-queued clause VACUOUS** (§2.3) | **rule** `elidex_api_ws::es_keepalive` (NEW) in `event_source.rs`; **seam** `KeepaliveClass::EventSource` reads `event_source_states` (`host_data.rs:488`), builds the closure, calls the rule | same: suppresses force-close for listener-held conns; orphan still force-closes |

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
   **else-branch** of the keepalive predicate (orphan / CLOSED only); listener-held / data-queued conns
   survive via `keepalive_survivors`.

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

---

## §8 Edge matrix (review-tail pre-empt)

| Invariant axis | WebSocket | EventSource |
|---|---|---|
| **GC-rooting (seam mark)** | ✔ `ws_keepalive` marks survivor in `keepalive_survivors` | ✔ `es_keepalive` marks survivor |
| **listener-lifecycle (type-restricted)** | tiered subset per state (open/message/error/close) | tiered subset per state (open/message/error) |
| **per-class predicate (engine-indep)** | `elidex_api_ws::ws_keepalive` | `elidex_api_ws::es_keepalive` |
| **active-state** | `WsReadyState ∈ {CONNECTING,OPEN,CLOSING}` | `SseReadyState ∈ {CONNECTING,OPEN}` |
| **in-flight-work (no-listener clause)** | `buffered_amount > 0` ∧ established (real) | task-queued = **vacuous** (inline drain, §2.3) |
| **force-close interaction** | marked ⇒ survives sweep, no Close; orphan/CLOSED ⇒ force-close (unchanged) | same |
| **unbind-lifecycle (per-VM)** | `websocket_states` drained + Close emitted for ALL conns on unbind | `event_source_states` drained + Close emitted |
| **behavior-change** | **YES** — listener-held / data-queued open conn must NOT force-close | **YES** — listener-held open conn must NOT force-close |
| **B1-home (component defer)** | exception (a) per-VM now → component after B1 | exception (a) per-VM now → component after B1 |

**Cross-cutting edges plan-review must scrutinize:**
1. **seam × force-close boundary (densest).** The predicate must mark a listener-held / data-queued conn
   ALIVE **and** the (post-flip) message pump must keep delivering — but the no-listener orphan must
   still force-close. Getting it wrong either **leaks network threads** (over-keep: a no-listener OPEN
   socket kept forever) or **drops live deliveries** (under-keep: a listener-held socket closed). The
   tier table + `buffered_amount` clause is the exact boundary; §9 tests both directions.
2. **predicate × `on<type>` handler** (`event_target_dispatch_vm.rs:93`): the type test must count
   `ws.onmessage = cb` / `es.onmessage = cb` with no `addEventListener` (else an on-handler-only page's
   socket is wrongly collected). `vm_path_has_listener` does (verified); test the handler-only path.
3. **CLOSED / CLOSING tier correctness.** A CLOSED WS/ES is **never** kept (else immortal closed
   wrappers leak); a CLOSING WS is kept **only** with an {error,close} listener **or** buffered data.
   Test: an OPEN WS with **only** an `open` listener (not in the OPEN tier {message,error,close}) is
   **collected** — proving the predicate is tiered, not any-listener.
4. **unbind force-close vs GC keepalive** (`vm_api.rs:420-461` / `host_data.rs:1809-1819`): on `unbind`,
   `drain_realtime_for_unbind` force-closes **every** conn (even listener-held). This is **correct and
   unchanged** — it is the spec's "Document object goes away ⇒ make disappear (WS) / forcibly close
   (ES)" rule (WebSockets §7 1001 close; HTML §9.2.9 abort fetch + set CLOSED), a **distinct** rule from
   GC keepalive. Confirm plan-review does not read unbind-closing a listener-held conn as a keepalive
   regression.
5. **`buffered_amount` faithfulness** (`websocket.rs:836` add / `websocket_dispatch.rs:229` sub): the
   data-queued clause is only as accurate as `buffered_amount`. It is existing behavior S5-3b relies on
   (not modified); flag that a `BytesSent` that never arrives keeps the socket immortal-until-unbind —
   which is **spec-mandated** ("must not be GC'd while data queued"), bounded by unbind.
6. **ES task-queued vacuity** (§2.3): confirm omitting the ES no-listener task-queued clause is
   spec-faithful given elidex's inline broker-drain (no queued-task window). A no-listener OPEN ES is
   collectible + force-closed — spec-correct because the "no-listener but task-queued" state never
   arises.

---

## §9 Test strategy (VM-test oracle — boa is the live engine)

S5-3b is exercised by **VM tests** (`elidex-js` `engine`-feature suite) + **engine-indep unit tests**
(`elidex-api-ws`). Test infra (from S5-3a `tests_match_media_keepalive.rs` + existing
`tests_websocket.rs` / `tests_event_source.rs`): `with_bound_vm(|vm| …)`, `vm.inner.collect_garbage()`
to force GC, `inject_ws_event_and_tick` / SSE inject helpers to drive readyState (Connected ⇒ OPEN) and
deliver messages.

**Engine-indep unit tests (`elidex-api-ws`, pure):**
- `ws_keepalive` every branch: each state × in/out-of-tier listener; CLOSED-never; data-queued clause
  (OPEN/CLOSING ∧ queued ⇒ true even with no listener; CONNECTING ∧ queued ⇒ false — not established).
- `es_keepalive` every branch: each state × in/out-of-tier listener; CLOSED-never; no data-queued axis.

**VM tests (the decisive behavior):**
- **WS keepalive (headline, positive):** `new WebSocket(u); ws.onmessage = cb;` drive to OPEN, **drop
  the reference**, force GC → assert the wrapper survives (`websocket_states` row retained) **and no
  `WebSocketClose` was emitted** to the broker, then inject a `message` → assert `cb` fired.
- **WS negative control (no over-rooting):** OPEN WS with **no listener** and no buffered data, no
  reference → GC → assert row pruned **and** `WebSocketClose` emitted (genuine orphan still force-closed).
- **WS tier (not any-listener):** OPEN WS with **only** an `open` listener (out of the OPEN tier) → GC →
  assert collected + closed (proves tier, not any-listener).
- **WS data-queued clause:** OPEN WS, no listener, `send()` to push `buffered_amount > 0`, no reference →
  GC → assert survives (no Close); then `BytesSent` to drop `buffered_amount` to 0 → GC → assert
  collected + closed.
- **WS CLOSED never kept:** a CLOSED WS (even with a `close` listener) → GC → assert collected.
- **WS `on*`-only path:** handler-only `ws.onmessage = cb` (no `addEventListener`) survives + delivers.
- **ES mirror set:** listener-held OPEN ES survives + keeps delivering; no-listener OPEN ES collected +
  `EventSourceClose` emitted; OPEN ES with only `open` listener collected (out of ES OPEN tier
  {message,error}); CLOSED ES collected.
- **unbind force-close (regression guard):** a listener-held OPEN WS/ES is force-closed on `Vm::unbind`
  (document-teardown rule, §8.4) — assert Close emitted on unbind even though GC would keep it.

(Asserting "no Close emitted" requires capturing `RendererToNetwork` messages — the existing WS/ES
dispatch tests already use a fake broker via `inject_ws_event_and_tick`; confirm the harness exposes the
sent-message log, else extend it.)

**Out of S5-3b (rides S5-6):** nothing — S5-3b is pure VM capability + tests. The message pumps it
protects are VM-resident; S5-3b only ensures their targets survive to be delivered to.

---

## §10 Deferred slots + open questions (per-PR cap ≤3)

### Slots (S5-3b creates no NEW defer concept — but must REGISTER two parent-carved slots, see reconciliation)
Both slots below were **carved as NEW by the parent S5-3 §10** but **never landed in the canonical
registry** `memory/project_open-defer-slots.md` — S5-3a (#430) registered only the *predecessor*
`#11-eventtarget-listener-keepalive-rooting`. So the registry currently has the predecessor (still
carrying the **refuted** "GENERIC EventTarget alive while listenered" any-listener framing, §2) and
**not** these two. They are program-level slots (not new S5-3b scope); registering them is a ledger
catch-up, not a new defer (defer-cap ≤3 unaffected). **S5-3b owns the reconciliation** (deliverable
below).

- **`#11-eventtarget-keepalive-component-migration`** (carved by parent §10, **registry-absent**,
  B1-gated) — S5-3b adds WS/ES as new per-VM HostData registrants; the component-on-entity ideal stays
  deferred to the B1 program (§6).
- **`#11-eventtarget-keepalive-registrant-coverage`** (carved by parent §10, **registry-absent**, HARD
  pre-flip gate) — S5-3b **satisfies the WS/ES portion**. After S5-3b, only **S5-3c (observers)**
  remains off the seam; the gate stays open until S5-3c lands (before S5-6).
- **ES task-queued clause** — **NOT a defer slot** (§2.3). Create-time 4-question eligibility audit
  (`feedback_defer-slot-eligibility-audit-at-create`): spec-core-deferred ✗ (the clause is *vacuous*
  under inline drain, not a postponed feature) / one-issue-one-way ✗ / pragmatic-shortcut ✗ (the ideal
  *is* omission) / repeat-signal ✗ → **0/4**. Downgraded to an in-memo + in-code maintenance note: *if
  the SSE delivery model ever becomes a deferred-task queue, re-architect the ES arm at that point* (no
  slot tracking owed).

### Defer-ledger reconciliation (an S5-3b landing deliverable, not a side-effect)
At S5-3b landing (in the landing-memo / `project_open-defer-slots.md` update — the slot-registration
convention point, per `feedback_defer-ledger-philosophy-lens`):
1. **Register** `#11-eventtarget-keepalive-registrant-coverage` (active HARD pre-flip gate, now tracking
   **S5-3c** — observers must land before S5-6) and `#11-eventtarget-keepalive-component-migration`
   (B1-gated) in `project_open-defer-slots.md`.
2. **Reframe + retire** the predecessor `#11-eventtarget-listener-keepalive-rooting` slot text — its
   "GENERIC 'EventTarget alive while listenered'" framing is **refuted** by §2 (the parent §12 named
   this an S5-3a deliverable; S5-3a left it undone). Reframe to the **keepalive-predicate seam**
   (per-registrant spec-faithful predicate), and mark it **superseded by the S5-3a/b/c program** (MQL +
   AbortSignal.timeout in S5-3a, WS/ES in S5-3b, observers in S5-3c).

### Open questions for `/elidex-plan-review`
- **Q1 (the tiers — the spine):** Confirm the WS tier {CONNECTING:open/message/error/close;
  OPEN:message/error/close; CLOSING:error/close; CLOSED:none} + the `established ∧ data-queued`
  no-listener clause, and the ES tier {CONNECTING:open/message/error; OPEN:message/error; CLOSED:none},
  are the spec-faithful rules (§2.1/§2.2 webref prose). Lean: **yes** (verbatim from §7 / §9.2.9).
- **Q2 (WS data-queued = `state ∈ {OPEN,CLOSING} ∧ buffered_amount > 0`):** **DECIDED** (spec + ideal
  lens converge, not an open choice): "established connection" = {OPEN, CLOSING} — CONNECTING is
  not-yet-established (and `send()` before OPEN throws `InvalidStateError`, so `buffered_amount > 0` is
  unreachable while CONNECTING), CLOSED is gone, CLOSING is still established (data buffered before
  `close()` keeps flushing). `buffered_amount > 0` is the faithful "data queued to transmit" signal
  (`websocket.rs:836` add / `websocket_dispatch.rs:229` sub). Plan-review validates the spec reading.
- **Q3 (ES task-queued vacuity):** **DECIDED** (spec + project-model lens converge): the ES no-listener
  task-queued clause is omitted because elidex's **inline** broker-drain has no queued-but-unrun task
  window (§2.3) — clause 3 (HTML §9.2.9) only adds keepalive for the *no-listener* case, which has no
  observable dependence under inline drain, so a no-listener OPEN ES is collectible + force-closed
  (spec-correct). No slot (no concrete future trigger). Plan-review validates the delivery-model premise.
- **Q4 (collect.rs no-edit):** Confirm the keepalive mark (`collect.rs:1233-1238`, before the sweep at
  `:1891-1935`) makes the existing unconditional force-close the **de-facto else-branch** with **no
  edit** to the sweep — and that no other path force-closes independent of the mark bit. Lean: **yes**.
- **Q5 (layering home):** Confirm `ws_keepalive` / `es_keepalive` belong in engine-indep `elidex-api-ws`
  (with `WsReadyState` / `SseReadyState` + the spec helpers), the seam only marshalling. Lean: **yes**
  (CLAUDE.md layering mandate; the rule is a spec/domain algorithm).
- **Q6 (unbind force-close not a regression):** Confirm unbind force-closing **even a listener-held**
  conn (`drain_realtime_for_unbind`) is the spec's document-teardown forcible-close (distinct from GC
  keepalive), correct and unchanged. Lean: **yes**.

---

## §11 Verified-cites note (read before plan-review)

Spec prose webref-verified 2026-07-01: WebSockets §7 (`websockets#garbage-collection`), HTML §9.2.9
(`html#garbage-collection`, server-sent-events), DOM §2.8 (`dom#observing-event-listeners`). Code cites
grep-verified against `main` `3345949e`: `keepalive.rs:38-41`/`:63-69`/`:141`/`:162-167`,
`collect.rs:1233-1238`/`:1315`/`:1891-1935`, `host_data.rs:466`/`:488`/`:601`/`:608-612`/`:673-707`/
`:700-706`/`:1809-1819`, `websocket.rs:184-185`/`:221`/`:419`/`:630`/`:836`,
`websocket_dispatch.rs:229`, `event_source.rs:154`, `event_source_dispatch.rs` (`dispatch_sse_event`
inline drain), `event_target_dispatch_vm.rs:79-94`, `vm_api.rs:420-461`, `well_known.rs:1411`,
`elidex-api-ws/{lib,websocket,event_source}.rs`.

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
