# S5-3 — EventTarget listener-keepalive rooting (the keepalive-predicate seam)

Per-PR plan-memo under the S5 umbrella (`docs/plans/2026-06-s5-flip-boa-deletion-umbrella.md`,
§5 row "S5-3 EventTarget listener-keepalive rooting", slot-table line 222). **Anchor = the ideal
end-state**, not an incremental patch (`feedback_plan-memo-anchor-on-ideal-not-incremental`). This memo
**deliberately reframes** the umbrella's S5-3 shorthand ("GENERIC EventTarget alive while listenered,
unifying AbortSignal/observers/MQL") — that shorthand, taken literally, is **spec-WRONG (over-rooting)**;
the spec-faithful mechanism is a **per-registrant keepalive predicate**, NOT an any-listener root (§2).

> **Gate**: `/elidex-plan-review` (5-agent design review) BEFORE impl, per CLAUDE.md
> "Edge-dense work = multi-PR program + 実装前 plan-review 必須" — and per the umbrella's own
> S5-3 row (`plan-review? = yes`, "edge-dense: ≥3 unification axes"). This memo's job is to map the
> edge matrix (§8) so plan-review can pre-empt the review tail, and to settle the spec-correctness
> reframe (§2) and the scope-split question (§7) **before** a line is written.

All file:line cites re-verified against `main` HEAD `d09829a5` (2026-06-29) — *note the live tree is
`d09829a5`, newer than the `5a7dbc58` named at pickup; cites below are against `d09829a5`*. Every spec §
webref-verified 2026-06-29 (sources: `dom`, `xhr`, `websockets`, `html`, `cssom-view-1`, `FileAPI`).

---

## §0 Read-first (scope + the central reframe)

### §0.1 What S5-3 is
A **FLIP-precondition** (umbrella §5 type-(a): land BEFORE the S5-6 boa→VM flip). **VM-internal**, boa
stays live, **no external dependency**. The deliverable is one **GC mark-roots pass** that keeps a VM
non-Node `EventTarget` alive while a **per-interface keepalive predicate** says it must be — fixing the
headline bug that a listener-only `matchMedia(q).addEventListener('change', cb)` MediaQueryList (MQL) is
GC-collected before `deliver_media_query_changes` can deliver its `change` event (§1).

It is **inert today** (boa is the live engine, the VM media path is dormant), but it **gates** the flip:
once the VM drives the shell (S5-6), the gap becomes a live "responsive site silently stops getting
`change`" regression. The in-code KNOWN-GAP statement + carve token is already planted at
`crates/script/elidex-js/src/vm/host/media_query.rs:336-351`.

### §0.2 The central reframe — this is **NOT** the naive "any-listener root"
The umbrella's shorthand "GENERIC EventTarget alive while listenered" (line 222) and the in-code gap
comment's "generic 'EventTarget kept alive while it has listeners'" (`media_query.rs:343-344`) are, taken
literally, **spec-wrong**: DOM §2.8 ("Observing event listeners", webref-verified) states that listener
presence must NOT be observable, and there is **no** general "a listener keeps the target alive" rule —
keepalive is a **per-interface opt-in exception**, and every real GC-note gates on `<active/in-flight
state> AND <type-restricted listener subset>`, never "any listener of any type" (§2). An any-listener
root would be **over-rooting** (a spec violation in the leak direction), and it would also bake in the
over-rooting the observers already have (§5). So:

> **Ideal mechanism = a keepalive-PREDICATE SEAM**: ONE GC mark-roots pass that consults a per-registrant
> keepalive predicate; each EventTarget class registers its own **spec-faithful** predicate. The shared
> piece = the predicate-consulting root pass. The per-class piece = the predicate. This unifies today's
> divergent dedicated-roots into ONE seam (One-issue-one-way) **and** fills the gaps — without
> over-rooting.

This is the spine of the memo; §2–§5 must not regress to "any-listener root".

### §0.3 The scope-split question (flagged for plan-review, §7)
The ideal is "seam + MQL precondition + migrate the existing divergent roots ONTO the seam in one PR"
(One-issue-one-way). But the breadth (GC root pass × ≥4 object classes × an **observer behavior change**
× a **WS/ES close-semantics change**) may warrant a 2–3 PR mini-program. §7 gives a recommendation and
marks it a **plan-review decision**.

**Strangler distinction (load-bearing — this is what makes the split safe).** The CLAUDE.md prohibition
is on the ***indefinite* strangler**: shipping "new seam + N legacy divergent roots" as **unbounded
permanent coexistence**, where "is this root on the seam or not?" becomes a recurring decision tax with
no forcing function to resolve it. That is **forbidden**. It is **distinct** from **bounded in-program
staging under a hard completion gate**: here, **all of S5-3a/b/c MUST land before S5-6 (the flip)** —
the *same* flip-precondition gate — so the seam+legacy coexistence is **bounded to the pre-flip dev
window and FORCE-RESOLVED by the flip gate** (the umbrella base-case model: a staged delivery whose
completion is hard-gated, not an open-ended migration). This **sanctioned staged delivery** is what §7
recommends.

Two facts make this concrete: (1) the WS/ES force-close and the observer over-root are **pre-existing**
behaviors — S5-3a does **not** *create* divergence, it adds the seam for the flip-critical cases (MQL +
already-correct AbortSignal) and the pre-existing roots are **migrated onto the seam by b/c before the
flip**; (2) the hard gate is registered in §10 as `#11-eventtarget-keepalive-registrant-coverage` =
**S5-3b/c MUST complete before S5-6**, exactly as C3 is a hard pre-flip gate for S5-6.

---

## §1 The bug — precise GC mechanics (cited, `d09829a5`)

The VM GC roots listener **callbacks** unconditionally but never the **target**:

1. **Callbacks are for-life roots.** `HostData::gc_root_object_ids()`
   (`crates/script/elidex-js/src/vm/host_data.rs:1697-1717`) yields every `listener_store` value (the
   callback `ObjectId`s) — seeded into the mark work-list at `crates/script/elidex-js/src/vm/gc/roots.rs:349-352`
   (`for id in hd.gc_root_object_ids() { mark_object(id, …) }`). So the **callback** survives.

2. **The target is NOT a root and is NOT traced.** A non-Node `EventTarget`'s listener metadata lives in
   `VmInner::vm_event_listeners` (a `HashMap<ObjectId, EventListeners>` keyed by the **target's** own
   `ObjectId`, `crates/script/elidex-js/src/vm/mod.rs:1338-1354`). This map is **not** a member of the
   `GcRoots` snapshot (`roots.rs:21-202`) and is **never** marked. Its own doc-comment states the
   contract: *"sweep prunes dead `ObjectId` keys"* (`mod.rs:1349-1352`) — i.e. the listener metadata is
   **retain-pruned by the target's OWN mark bit**, it does not confer reachability.

3. **The sweep prunes the listener row by the target's mark bit.** At
   `crates/script/elidex-js/src/vm/gc/collect.rs:1718-1739`: `self.vm_event_listeners.retain(|id, _|
   bit_get(marks, id.0))` — and it then **retires the dead target's `ListenerId`s from `listener_store`**
   (the root) so the callbacks release too. So a target anchored **only** by its listeners is swept, and
   its out-of-band state row is mark-bit-pruned with it.

4. **Headline failure (MQL).** A listener-only MQL has no retained JS reference; its only anchor is the
   `change` listener (whose callback is rooted, but the MQL itself is not). On any GC it is collected, and
   its registry row in `VmInner::media_query_list_registry` (`mod.rs:1257`) is pruned. But
   `deliver_media_query_changes` iterates **`self.media_query_list_registry` directly**
   (`crates/script/elidex-js/src/vm/host/media_query.rs:384`, `for (&id, entry) in
   &self.media_query_list_registry`) — so the collected MQL is silently absent and the `change` event is
   never fired. The in-code KNOWN-GAP comment (`media_query.rs:336-351`) documents exactly this and carves
   `#11-eventtarget-listener-keepalive-rooting`.

**Why S5-2's VV/Screen are NOT in scope of this bug** (stated so plan-review confirms the boundary):
`visual_viewport_instance` / `screen_instance` are **permanent proto-roots** (marked unconditionally at
`crates/script/elidex-js/src/vm/gc/collect.rs:457-469`), so S5-2's `deliver_visual_viewport_events` is
safe *only because* its singleton is never listener-only-rooted. The same class of producer **breaks**
for any transient (non-singleton) listener-only target — MQL is the first instance.

---

## §2 Why the naive any-listener root is spec-WRONG (the predicate argument)

### §2.1 DOM §2.8 — listener presence must not be observable
DOM §2.8 "Observing event listeners" (`dom#observing-event-listeners`, webref-verified):

> *"In general, developers do not expect the presence of an event listener to be observable. … Ideally,
> any new event APIs are defined such that they do not need this property."*

There is **no general rule** that "an EventTarget with a listener stays alive". Keepalive is a per-interface
**exception**, and where it exists it is **type-restricted** (a subset of event types) **AND**
**state-gated** (an active/in-flight condition) — making a *no-op* listener of an *unrelated* type
**non-keepalive**, exactly as §2.8 demands. An "any listener of any type roots the target" rule would make
the presence of an empty listener observable through GC behavior — a §2.8 violation in the over-rooting
direction.

### §2.2 The real per-interface GC-notes (webref-verified table)

| Interface | Spec § (webref) | Keepalive condition (exact prose) | No-listener clause |
|---|---|---|---|
| **XMLHttpRequest** | XHR §3.2 Garbage collection (`xhr#garbage-collection`) | state ∈ {opened-with-send, headers-received, loading} **AND** ≥1 listener of type ∈ {`readystatechange`,`progress`,`abort`,`error`,`load`,`timeout`,`loadend`} | (GC-while-open ⇒ terminate fetch controller) |
| **WebSocket** | WebSockets §7 Garbage collection (`websockets#garbage-collection`) | **state-tiered**: CONNECTING ⇒ listener ∈ {open,message,error,close}; OPEN ⇒ {message,error,close}; CLOSING ⇒ {error,close} | **"established connection with data queued to be transmitted must not be garbage collected"** (no-listener clause) |
| **EventSource** | HTML §9.2.9 Garbage collection (`html#garbage-collection`, server-sent-events) | **state-tiered**: CONNECTING ⇒ listener ∈ {open,message,error}; OPEN ⇒ {message,error} | "while there is a task queued … on the remote event task source, … strong reference" (no-listener clause) |
| **AbortSignal** (dependent) | DOM §3.2.1 AbortSignal GC (`dom#abort-signal-garbage-collection`) | non-aborted dependent signal: **source signals non-empty** AND (≥1 `abort` listener **OR** abort-algorithms non-empty) | — |
| **AbortSignal.timeout** | DOM `dom-abortsignal-timeout` step note | for the timeout's duration, if signal has any `abort` listener ⇒ strong ref from global | (the timer registration itself is the anchor — see `pending_timeout_signals`, §5) |

Each is `<state> AND <type-restricted subset>`, **never** "any listener". The WS/ES no-listener clauses
("data queued" / "task queued") prove keepalive is **not** a listener test at all in those cases — it is an
**in-flight-work** test.

### §2.3 The registry-membership interfaces (NO GC-note → predicate is "registry membership")
Three relevant classes have **no** GC-note; they are kept alive by **registry membership**, not a listener
test (webref-verified absences):

- **MediaQueryList** — CSSOM-View §4.2 (`cssom-view-1#the-mediaquerylist-interface`) defines "*A
  MediaQueryList object has an associated … document set on creation*" and the "evaluate media queries and
  report changes for a Document" routine iterates "*each MediaQueryList object target that has doc as its
  document*". There is **no** GC-note in CSSOM-View (`dfn 'garbage collection' cssom-view-1` → no hit). The
  MQL is kept alive by the **document's MQL set**; a listener-less unreferenced MQL is **correctly
  collectible** (it can never deliver a useful `change` anyway). In elidex the registry-membership analogue
  is `media_query_list_registry`, but elidex's registry is a plain side-store that confers **no GC
  reachability** today — which is precisely the bug. The spec-faithful elidex predicate is **not** raw
  registry membership (that would re-introduce the over-rooting the document-set has only because the
  document is itself rooted); it is "**has ≥1 `change` listener**" (§5).
- **MutationObserver** — DOM has no MutationObserver GC dfn; kept alive by the node's **registered observer
  list** (`dom#registered-observer-list`, §4.3). Membership = an **active observation**.
- **FileReader** — FileAPI has no GC-note (`dfn 'garbage collection' FileAPI` → no hit); an in-flight read
  is anchored by the **File Reading Task Source** (FileAPI §6.1, `FileAPI#blobreader-task-source`). An
  **idle** FileReader with a listener is **correctly collectible** → **not a gap** (§5, out of scope).

**Conclusion**: the only correct generalization is a **predicate seam** — one root pass that asks each
registrant *its own* spec-faithful question. "Any listener roots the target" fails §2.8 and contradicts
every per-interface note above.

### §2.4 Coupled-invariant enumeration (the canonical home; §8's matrix is the expanded form)
This is an edge-dense plan; the simultaneously-satisfied invariants live here (per the plan-review
Pre-condition #3), with §8's matrix as the per-class expansion. The seam must satisfy, **together**:

- **GC-rooting** — the keepalive predicate runs in the `mark_roots` pass and `mark_object`s survivors.
- **listener-lifecycle** — the per-target listener home (`vm_event_listeners`) is read by type, including
  the `on*` handler-attribute path.
- **per-class-predicate** — each class registers its own spec-faithful rule (the §5 table).
- **active-state** — the in-flight/active condition (readyState, non-aborted, has-observation).
- **unbind-lifecycle** — every per-VM registrant map is `unbind`-cleared (no cross-DOM survivor).
- **world_id-home** — per-VM `ObjectId` now (exception (a)); component-on-entity after world_id.

Key pairwise intersections (one line each):
- **GC-rooting × listener-lifecycle** — the mark pass reads the per-target listener home
  (`vm_event_listeners`) to decide survival.
- **per-class-predicate × active-state** — WS/ES tier = readyState ∧ listener-subset; AbortSignal =
  non-aborted ∧ (listener ∨ algorithms).
- **AbortSignal active-state × `any()` composite** — a dependent `any()` composite is predicate-rooted
  while non-aborted ∧ source-signals-non-empty ∧ listenered, **NOT** by mere existence (this preserves
  the (k) non-root for the no-anchor case: a composite with no listener and no live source signals stays
  a non-root).
- **unbind-lifecycle × world_id-home** — the per-VM registrant maps are `unbind`-cleared now;
  component-on-entity migration is deferred to the world_id cohort (§6).

---

## §3. Spec coverage map

This is a **GC-keepalive** plan, so the rows are keepalive **RULES × condition-tiers** (not algorithm
steps): each row is a `<spec GC-note> × <state/listener condition-branch>`, and the **keepalive-predicate
seam (§4)** implements every row (one mark-roots pass consulting a per-registrant predicate; the per-class
predicate sites are the §5 table). The "Touch" column names the predicate / mark-pass site from §5.

| Spec section | Step / condition | Branch | Touch (predicate / mark-pass site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| DOM §2.8 Observing event listeners | general default (no listener-keepalive) | — | seam does NOT root on bare-listener presence | ✓ | yes (page listeners) |
| DOM §3.2.1 AbortSignal GC | dependent signal, non-aborted | source-signals non-empty ∧ (≥1 `abort` listener ∨ abort-algorithms non-empty) | AbortSignal predicate; routes controller-trace + `pending_timeout_signals` + `any()` | ✓ | yes |
| WebSockets §7 GC | CONNECTING | listener ∈ {open,message,error,close} | WS predicate over `websocket_states` | ✓ | yes |
| WebSockets §7 GC | OPEN | listener ∈ {message,error,close} | WS predicate | ✓ | yes |
| WebSockets §7 GC | CLOSING | listener ∈ {error,close} | WS predicate | ✓ | yes |
| WebSockets §7 GC | established conn, data queued | no-listener clause | WS predicate else-branch (keeps genuine-orphan force-close) | ✓ | no |
| HTML §9.2.9 EventSource GC | CONNECTING | listener ∈ {open,message,error} | ES predicate over `event_source_states` | ✓ | yes |
| HTML §9.2.9 EventSource GC | OPEN | listener ∈ {message,error} | ES predicate | ✓ | yes |
| HTML §9.2.9 EventSource GC | queued remote-event task | no-listener clause | ES predicate else-branch | ✓ | no |
| CSSOM-View §4.2 MediaQueryList | document-set membership *(spec: listener-independent)* → elidex narrows to ≥1 `change` listener/`onchange` (a listener-less unreferenced MQL delivers nothing, so collectible) — **pragmatic-faithful** | has ≥1 `change` listener OR `onchange` handler | MQL predicate over `media_query_list_registry` | ✓ | yes |
| DOM §4.3 registered-observer-list | observer keepalive (no GC-dfn) | has ≥1 active observation (Option α) | observer predicate replaces construct-root | ✓ | yes |
| XHR §3.2 GC | (XHR not implemented in VM) | reference-only | n/a — no XHR ObjectKind; born into seam when it lands | n/a | n/a (future) |
| FileAPI §6.1 file-reading task source | in-flight read | task-rooted (`PendingTask::FileRead`); idle = collectible | already correct; out of scope | ✓ | no |

### §3.1 Breadth + split verdict
**K = 6** distinct specs (DOM, WebSockets, HTML, CSSOM-View, XHR, FileAPI); **M = 13** entries (rows). The
breadth is genuinely high (K=6 / M=13) because the keepalive seam unifies six independent GC-notes — but
§7 already proposes the **S5-3a / S5-3b / S5-3c split** (precondition / WS-ES close-semantics / observer
over-root) precisely to bound each behavior-bearing axis into its own plan-reviewed slice. So the breadth
here **aligns with SPLIT-RECOMMENDED**: the per-PR slices each touch a narrow row-subset of this table
(S5-3a = MQL + AbortSignal rows; S5-3b = WS/ES rows; S5-3c = observer row), not all six specs at once.

### §3.2 User-input touch audit
The user-controllable inputs are the page's `addEventListener(type, …)` calls + `on*` handler assignments
(`onchange`/`onmessage`/`onopen`/`onerror`/`onclose`/`onabort`) + the construction of WS / ES /
AbortSignal targets. The predicate reads the **listener type** (untrusted, page-supplied), but the
type-set it tests against is a **fixed spec enum** per interface (MQL:{change}; WS-tiered:{open,message,
error,close}; ES-tiered:{open,message,error}; AbortSignal:{abort}) — there is **no injection surface**: an
unrecognized type simply fails the keepalive test. The keepalive is a pure
`(active-state, listener-type-set) → bool`, reading only per-VM state the page already produced, so it
opens **no new trust boundary** (cf. umbrella §3.1: the flip is trust-boundary-neutral).

---

## §4 The ideal — the keepalive-predicate seam (mechanism design)

### §4.1 Where it hooks
`GcRoots<'a>` (`crates/script/elidex-js/src/vm/gc/roots.rs:21-202`) is an immutable snapshot of every root
origin, constructed from `VmInner` at `crates/script/elidex-js/src/vm/gc/collect.rs:52`, and walked by
`mark_roots` (`roots.rs:269`, invoked at `collect.rs:1145`). The seam is a **new mark-roots pass** in
`mark_roots`, modeled exactly on the existing per-instance root passes already there — e.g.
`pending_timeout_signals` (`roots.rs:483-492`, label "(j)") and the `any_composite_map` *non*-root note
(`roots.rs:600-618`, label "(k)").

The pass needs read access to the **registrant maps** (which today are NOT in `GcRoots`): at minimum
`media_query_list_registry` (`mod.rs:1257`), plus `vm_event_listeners` (`mod.rs:1354`, the listener home
the predicate queries by type), `websocket_states` / `event_source_states`
(`crates/script/elidex-js/src/vm/host_data.rs:451`/`:473`, for the state-tiered predicates), and the
observer-binding maps (`host_data.rs:1700-1708`). These are added to `GcRoots` as borrowed refs (the same
pattern as `abort_signal_states` at `roots.rs:96` and the ~30 sibling state maps) and the pass marks each
registrant `ObjectId` whose predicate returns `true` via `mark_object(id, obj_marks, work)`.

### §4.2 What a registrant looks like (the dispatch shape — seam MARSHALS, engine-indep crate RULES)
The per-class predicate **decomposes into two layers** (the layering mandate — see §4.4): the **pure
spec rule** `(active-state, listener-presence) → bool` lives in the engine-independent crate that owns
the interface's domain (`elidex-api-ws` for WS/ES, `elidex-api-observers` for observers); the `vm/gc/`
**seam only marshals** — it supplies the VM-side state (`websocket_states`/`event_source_states` value,
the observer-id) and a **listener-presence closure** over `vm_event_listeners`, then CALLS the
engine-indep rule. The seam never re-derives the SPEC-RULE logic locally. Two candidate dispatch shapes
for the marshalling layer — **plan-review picks**:

- **(shape A) static enum dispatch** — a `KeepaliveClass` enum **(NEW)** (`Mql`, `WebSocket`,
  `EventSource`, `AbortSignalDependent`, `Observer`) with a `fn keepalive(&self, target: ObjectId, roots: &GcRoots) ->
  bool` match arm per class. The arm *marshals* (reads the VM state map + builds the `has_listener`
  closure) and *delegates the rule* to the engine-indep fn (e.g. `KeepaliveClass::WebSocket` reads
  `websocket_states` for the `WsReadyState` and calls `elidex_api_ws::ws_keepalive(state, has_listener)`
  — see §5). Hot-path, no allocation, mirrors how `WrapperKind::mark_agent` (`roots.rs:358`) already
  dispatches mark behavior by a per-kind enum. **Recommended** — it is the elidex-plugin "static enum
  dispatch for built-ins" canonical form (CLAUDE.md Plugin-first).
- **(shape B) trait object registry** — a `dyn KeepalivePredicate` list. Rejected: these are all
  **built-in** EventTargets (no runtime/user extension), so dynamic dispatch is the wrong tier
  (CLAUDE.md: "Hot path built-in is static dispatch").

The **type-restricted listener test** (the `has_listener` closure the seam hands the engine-indep rule)
is implementable today: `EventListeners`
(`crates/script/elidex-script-session/src/event_listener.rs:131`) exposes `matching_all(event_type)`
(`:329`), `matching_all_ids(event_type)` (`:338`), and `find_event_handler(event_type)` (`:194`) — so
"has ≥1 `change` listener (or `onchange` handler)" = `vm_event_listeners.get(&target).is_some_and(|l|
!l.matching_all("change").is_empty() || l.find_event_handler("change").is_some())`. (The `on*` IDL handler
attribute registers via `add_event_handler`, `event_listener.rs:172` — so a predicate that only checked
`matching_all` would miss an `onchange = …` page; the closure must count both, the §5 wiring notes
this.) The engine-indep rule consumes this as an opaque `has_listener: impl Fn(&str) -> bool` — it owns
*which* types to test (the spec enum) but not *how* listeners are stored (the VM concern).

### §4.3 Per-VM HostData home (NOT a component — see §6)
The registrant maps stay where they are (`VmInner` for `media_query_list_registry`/`vm_event_listeners`,
`HostData` for `websocket_states`/observer bindings) — **per-VM** state, `unbind`-cleared. The seam adds
**no new side-store**; it only adds the *predicate-consulting mark pass*. The component-on-entity ideal is
world_id-gated (§6).

### §4.4 Layering note — engine-indep crate owns the SPEC-RULE, the seam owns only the marshalling
The per-class keepalive logic **splits along the layering mandate** (CLAUDE.md "VM host/ は engine-bound
責務のみ" / "新規 algorithm を host/ に書く前に engine-independent crate を確認"):

- **The engine-indep crate owns the pure spec rule.** The `(active-state, listener-presence) → bool` tier
  table for WS/ES belongs in **`elidex-api-ws`** (which already carries the spec helpers `validate_ws_url`
  / `normalize_ws_url` / `is_mixed_content` and the `WsReadyState` / `SseReadyState` enums,
  `crates/api/elidex-api-ws/src/websocket.rs` + `event_source.rs`). The "is this observer currently
  observing anything" membership query belongs as a method on the engine-indep observer registries in
  **`elidex-api-observers`** (`crates/api/elidex-api-observers/src/mutation/mod.rs` + `intersection.rs` +
  `resize.rs`). These are spec/domain algorithms, not engine-bound marshalling — they must NOT live as
  bespoke logic in `vm/gc/`.
- **The `vm/gc/` seam owns only VM-state marshalling + the mark pass.** It reads the per-VM state maps,
  builds the listener-presence closure, calls the engine-indep rule, and `mark_object`s the survivors.
  No SPEC-RULE branching in the seam.
- **Two predicates legitimately stay VM-side** (no engine-indep analogue exists, so they are genuine VM
  GC bookkeeping, not engine-bound algorithm mis-homed): **(1) the AbortSignal dependent-signal
  predicate** — AbortSignal has no engine-indep crate (its source-signal graph + abort-algorithm set are
  VM `ObjectKind`/trace state), so its keepalive is VM GC bookkeeping; **(2) the MQL `change`-listener
  test** — it uses `EventListeners::matching_all` / `find_event_handler`, which already live in the
  engine-indep `elidex-script-session` (`event_listener.rs`), so the rule is already engine-indep; the
  seam just calls it (no new crate move owed).

---

## §5 Per-class predicate table (each: spec § + elidex predicate + wiring site)

| Class | Spec § (webref) | elidex keepalive predicate | Wiring site | Replaces / fixes |
|---|---|---|---|---|
| **MediaQueryList** | CSSOM-View §4.2 (no GC-note; document-set membership *(spec: listener-independent)*) | **has ≥1 `change` listener OR `onchange` handler** (no active-state axis) — **pragmatic-faithful**: narrows the listener-independent document-set to a listener test (a listener-less unreferenced MQL delivers nothing, so collecting it is GC-observably sound) | predicate over `media_query_list_registry` (`mod.rs:1257`) keys ∩ `vm_event_listeners` `change` test | THE flip-precondition (`media_query.rs:336-351`) |
| **WebSocket** | WebSockets §7 (`websockets#garbage-collection`) | `state ∈ {CONNECTING,OPEN,CLOSING}` with the **tiered** listener subset {CONNECTING:open/message/error/close; OPEN:message/error/close; CLOSING:error/close} **OR** "established connection with data queued to transmit" | **rule** = `elidex_api_ws::ws_keepalive(state: WsReadyState, has_listener) -> bool` **(NEW)** in `elidex-api-ws/src/websocket.rs`; **seam** = `KeepaliveClass::WebSocket` reads `websocket_states` (`host_data.rs:451`, holds `readyState`+`conn_id`) for the state, supplies a `has_listener` closure over `vm_event_listeners`, calls the rule | **REPLACES the force-close** at `collect.rs:1869-1896` for listener-held open conns |
| **EventSource** | HTML §9.2.9 (`html#garbage-collection`) | `state ∈ {CONNECTING,OPEN}` with tiered subset {CONNECTING:open/message/error; OPEN:message/error} **OR** queued remote-event task | **rule** = `elidex_api_ws::es_keepalive(state: SseReadyState, has_listener) -> bool` **(NEW)** in `elidex-api-ws/src/event_source.rs` (SSE analogue, same crate); **seam** = `KeepaliveClass::EventSource` reads `event_source_states` (`host_data.rs:473`), supplies the closure, calls the rule | same: REPLACES force-close at `collect.rs:1883-1896` for listener-held open conns |
| **AbortSignal (dependent)** | DOM §3.2.1 (`dom#abort-signal-garbage-collection`) | non-aborted dependent: **source signals non-empty** AND (≥1 `abort` listener OR abort-algorithms non-empty) | route existing roots through the seam; address `any()` composite (`roots.rs:600-618`, today NOT marked) under this predicate | unifies the AbortController trace + timeout root + `any()` non-root into one rule |
| **AbortSignal.timeout** | DOM `dom-abortsignal-timeout` step note | timer pending ⇒ root (already correct: `pending_timeout_signals`, `roots.rs:483-492`) | route the existing `pending_timeout_signals` root through the seam (mechanism only) | mechanism unification (behavior unchanged) |
| **Observers (Mutation/Resize/Intersection)** | DOM §4.3 registered-observer-list (membership = active observation) | **has ≥1 active observation** (registry membership) | **rule** = `is_observing(observer_id) -> bool` **(NEW)** on each engine-indep registry (`elidex-api-observers/src/mutation/mod.rs` + `intersection.rs` + `resize.rs`), maintained **incrementally** O(1) (see §5.2); **seam** = `KeepaliveClass::Observer` calls it; **Option α/β** below | **FIXES over-rooting** (construct-time root never released, see below) |
| **FileReader** | FileAPI §6.1 task source (no GC-note) | in-flight ⇒ task-rooted (already correct: `PendingTask::FileRead`, `roots.rs:515-524`); **idle-with-listener = collectible** | — | **NOT a gap; OUT OF SCOPE** (idle FileReader has no spec keepalive) |
| **VisualViewport / Screen** | (S5-2 singletons) | permanent proto-roots (`collect.rs:457-469`) | — | **unaffected**; could migrate onto the seam later for uniformity but **need not** (not a gap) |

### §5.1 AbortSignal sub-cases (precise cites)
- **AbortController internal signal**: today the controller traces to its signal via the
  `ObjectKind::AbortController { signal_id }` arm (`object_kind.rs:311` field; `gc/trace.rs:391-392`
  `mark_object(*signal_id)`) — so the signal survives while the controller is referenced. **Correct
  already**; route through the seam for uniformity (mechanism, not behavior).
- **`AbortSignal.timeout`**: rooted via `pending_timeout_signals` (`roots.rs:483-492`). Correct; route
  through seam.
- **`AbortSignal.any()` composite**: deliberately **NOT** marked today (`roots.rs:600-618`, label "(k)":
  "weak bookkeeping only — NOT GC roots"). The §3.2.1 dependent-signal predicate ("source signals non-empty
  AND has abort listeners/algorithms") is **exactly** what should gate it — so the seam **fills** the
  currently-deliberate non-root: a composite with an installed `abort` listener and live source signals
  must survive (it does indirectly today only if a JS ref holds it; §3.2.1 says it must survive on the
  predicate). This is a **behavior change to verify** at plan-review.

### §5.2 Observers — the over-rooting reconciliation (explicit plan-review question)
Today the `(callback, instance)` binding is inserted at **construction**
(`crates/script/elidex-js/src/vm/host/mutation_observer.rs:197-203`) and rooted **for life** via
`gc_root_object_ids` (`host_data.rs:1700-1708`, the `mutation_observer_bindings` /
`resize_observer_bindings` / `intersection_observer_bindings` flat-map). **`disconnect()` does NOT release
it**: `native_mutation_observer_disconnect` (`mutation_observer.rs:249-261`) calls `observers.disconnect`
(clears the *observation*) but **never touches `mutation_observer_bindings`** — so a
constructed-then-disconnected observer is **immortal until `unbind`** (a leak, and an over-root: a
no-observation observer with no JS reference must be collectible). The spec-faithful predicate is "**has ≥1
active observation**" (registry membership = the node's registered-observer list, DOM §4.3).

**The membership query is an engine-indep `is_observing(observer_id) -> bool` (NEW), maintained
incrementally — NOT a per-GC scan.** Today the observation set is the **per-entity `MutationObservedBy`
component** (`mutation/mod.rs:114`, a `Vec<MutationObservation>` on each *observed target entity*; the
mutation crate's own doc-comment notes the registered-observer list "lives as a `MutationObservedBy`
component on the observed target", and there is **No observer→nodes reverse index** — the only directions
are entity→observers and the observer-id allocator). So the literal "has ≥1 active observation" framing,
implemented naively, would force the GC predicate to **scan every `*ObservedBy` component on every entity**
to find one carrying this `observer_id` — an **O(observers × observed-entities) per-GC full-entity scan**
(repeated for the Intersection/Resize registries' analogous per-entity component lists). The ECS-native
fix is to maintain an **active-observation count per observer on the registry side**, incremented/decremented
as observations are added/removed at their existing chokepoints:

- **increment**: `observe` (`mutation/mod.rs:256`) and `add_transient_observers` (`:186`) — and the
  `IntersectionObserverRegistry::observe` / `ResizeObserverRegistry::observe` analogues
  (`intersection.rs:150` / `resize.rs:133`);
- **decrement**: `unobserve` / `disconnect` (`mutation/mod.rs:339`, via `retain_observations`; the
  intersection/resize `unobserve`/`disconnect` `retain`s at `intersection.rs:177`/`:189`,
  `resize.rs:167`/`:179`) and `retain_observations`-family clears (`mutation/mod.rs:561`).

`is_observing` then reads this count **O(1) per registrant** — so the GC predicate cost is O(observers),
not O(observers × observed-entities). **This subsumes F2** (the O(N²) `*ObservedBy` full-entity scan the
"≥1 active observation" framing would otherwise require is removed by the observer-side count index).

> **Plan-review decision (recommend α)**:
> - **(α)** Route observers through the seam with the **active-observations** predicate, **fixing** the
>   over-rooting (a constructed-never-observed or disconnected observer with no JS ref becomes collectible).
>   *Ideal / spec-faithful, but a behavior change* — a page relying on `new MutationObserver(cb)` staying
>   alive across a GC **without** `observe()` and without a stored reference would change (such a page is
>   already spec-non-conformant; the callback is rooted only while an observation exists).
> - **(β)** Keep the construct/disconnect rooting, only unify the *mechanism* (move the binding root behind
>   the seam without changing when it roots). No behavior change; leaves the over-root.
>
> **Recommendation: α** (ideal over pragmatic; spec-faithful; removes the leak). Surfaced because α is a
> behavior change that plan-review must accept.

---

## §6 ECS-native lens + world_id home constraint + deferred component migration

The rooted thing is a per-VM `ObjectId` (the registrant's wrapper). Under CLAUDE.md's side-store→component
rule it is the **per-VM-identity-handle exception (a)**: the value is `Send` (`ObjectId(u32)`) but its
meaning is **per-VM** — `EcsDom` shares the entity-index/`ObjectId` space across VMs and rebinds it, so
component-izing the keepalive marker would create **cross-DOM aliasing** (a prior DOM's wrapper picked up
after rebind). `Vm::unbind` already documents this exact hazard for the sibling per-VM handles:
`crates/script/elidex-js/src/vm/vm_api.rs:608-609` ("*cross-DOM-aliasing per the side-store→component rule
exception (a)), so they must be cleared on unbind*").

The **IDEAL ECS-native form** is a keepalive **marker-component on the watched entity** — the
`MutationObservedBy` precedent from `#213` (the genuine side-store→component outlier per
`memory/ecs-native-side-store-audit-2026-05-21.md`: observer registry `Vec<ObserverState>` →
per-target-entity `*ObservedBy` component). But:

- the keepalive registrants here are **non-Node** EventTargets (MQL / WebSocket / AbortSignal) — they have
  **no entity**, only an `ObjectId` — so the `*ObservedBy`-component shape does not even apply to most of
  them today, and
- the ones that *could* be component-homed are world_id-gated (the cross-DOM discriminator
  `#11-wrapper-cache-cross-dom-discriminator`), and **world_id lands strictly AFTER S5** (umbrella §0).

> **Therefore S5-3 lands the per-VM HostData registry/predicate form** (`unbind`-cleared, consistent with
> how `vm_event_listeners` / `listener_store` / `websocket_states` already live), and the
> component-on-entity migration is **deferred to the world_id cohort**. This is the **same per-VM-now /
> component-later pattern** S5-2 used for its singletons (`localStorage`-style per-bind caching, cleared on
> unbind). **Carve `#11-eventtarget-keepalive-component-migration`** (NEW, §10) for the world_id-gated move.

**ECS axis confirmation for plan-review**: the keepalive predicate reads per-VM EventTarget state (a
browsing-context-level/per-VM fact, exception (a)), not a per-entity DOM fact mis-stored in a side-store.
No new component is owed pre-world_id.

---

## §7 Scope / PR-split recommendation (a plan-review decision)

**Ideal (One-issue-one-way)**: one PR = the keepalive-predicate seam + MQL wiring (the actual
flip-precondition) + **migrate the existing divergent roots onto the seam** (AbortSignal
controller/timeout/`any()`, observers, WS/ES) in the same PR.

**Strangler-safety of the split (§0.3).** The split below is **bounded in-program staging under a hard
pre-flip gate**, NOT the forbidden *indefinite* strangler. The forbidden form is **unbounded permanent
coexistence** ("new seam + N legacy divergent roots" with no forcing function — the decision tax "is this
root on the seam or not?" reappearing on every later GC PR indefinitely). What §7 recommends is instead a
staged delivery whose completion is **hard-gated by the flip**: **all of S5-3a/b/c MUST land before S5-6**
(§0.3 / §10's `#11-eventtarget-keepalive-registrant-coverage` hard gate), so the seam+legacy coexistence
is bounded to the pre-flip dev window and FORCE-RESOLVED at the flip. Note also that the WS/ES force-close
and observer over-root are **pre-existing** behaviors — S5-3a does not *introduce* divergence; it adds the
seam for the flip-critical cases and b/c migrate the pre-existing roots onto it before the flip.

**But the breadth is genuinely edge-dense**: GC root pass × **4 object classes** × an **observer
behavior-change** (§5.2 α) × a **WS/ES close-semantics change** (`collect.rs:1869-1896` force-close must
become "stay alive + keep delivering" for listener-held open conns). That is ≥3 intersecting invariant
axes touching **established, behavior-bearing** code (the network force-close path, the observer root) — a
classic edge-dense bundle.

**Recommendation (mark as plan-review decision)**: a **2–3 PR mini-program** under the umbrella:
- **S5-3a** = the **seam + MQL predicate + MQL wiring** (the pure flip-precondition; touches no
  behavior-bearing established code — MQL keepalive is net-new) + route the **already-correct** AbortSignal
  roots (controller trace / timeout) through the seam as the seam's first non-MQL clients (mechanism
  unification, **no behavior change**). This is the narrow base-case slice the flip actually needs.
- **S5-3b** = **WS/ES**: replace the force-close-on-sweep with the state-tiered keepalive predicate (the
  behavior change "listener-held open connection must STAY ALIVE and keep delivering, not be closed"),
  keeping the genuine-orphan GC-close for the no-listener case (`collect.rs:1869-1896` becomes the *else*
  branch).
- **S5-3c** = **observer reconciliation** (§5.2 α): move the construct-time binding root to the
  active-observations predicate (the over-rooting fix / leak fix). Independently shippable; not a flip
  blocker.

This keeps the **flip-precondition** (S5-3a) small and behavior-neutral while isolating the two behavior
changes (WS/ES close-semantics; observer over-root) into their own plan-reviewed slices — each a terminal
base-case under the approved umbrella (CLAUDE.md base-case rule). **Alternative**: one PR if plan-review
judges the migration cohesive enough to land atomically (the One-issue-one-way purity argument). **Lean:
split into S5-3a (precondition) + S5-3b (network) + S5-3c (observers)** — where **S5-3a is the narrow
true precondition** (the seam itself), but **all three (a/b/c) are pre-flip-mandatory: they MUST all land
before S5-6** (the §10 `#11-eventtarget-keepalive-registrant-coverage` hard gate), so the split is
strangler-safe (bounded staging force-resolved by the flip, not unbounded coexistence). b/c fix
pre-existing latent bugs and are not individually flip-*order*-critical relative to each other, but the
hard gate forbids reaching S5-6 with any of them still off the seam.

---

## §8 Edge matrix (review-tail pre-empt)

Invariant axes × the touched concerns. Cells name the per-class behavior at each intersection.

| Invariant axis | MQL | WebSocket | EventSource | AbortSignal | Observers |
|---|---|---|---|---|---|
| **GC-rooting (the seam)** | ✔ predicate marks survivor | ✔ predicate marks survivor | ✔ predicate marks survivor | ✔ route existing roots through seam | ✔ predicate replaces construct-root |
| **listener-lifecycle (type-restricted)** | `change` only | tiered subset (per state) | tiered subset (per state) | `abort` only | — (observation, not listener) |
| **per-class predicate** | listener test | state ∧ tiered listener ∨ data-queued | state ∧ tiered listener ∨ task-queued | source-signals ∧ (listener ∨ algorithms) | active-observation count |
| **active-state axis** | none | readyState ∈ active-set | readyState ∈ active-set | non-aborted | has observation |
| **unbind-lifecycle (per-VM)** | registry cleared on unbind | `websocket_states` drained on unbind (`websocket.rs:41`) | `event_source_states` drained on unbind | per-VM states cleared | bindings cleared on unbind |
| **behavior-change** | net-new (no change) | **YES** — listener-held open conn must NOT force-close (`collect.rs:1869-1896`) | **YES** — same | **maybe** — `any()` composite now predicate-rooted (`roots.rs:600-618`) | **YES (α)** — disconnected/never-observed becomes collectible |
| **world_id-home (component defer)** | exception (a) per-VM now | exception (a) per-VM now | exception (a) per-VM now | exception (a) per-VM now | exception (a) per-VM now → `#213` component AFTER world_id |

**Cross-cutting edges plan-review must scrutinize:**
1. **seam × WS/ES close-semantics** (densest): the predicate must mark a listener-held open conn ALIVE
   **and** the producer must keep delivering — but the no-listener orphan must still force-close
   (`collect.rs:1869-1896` becomes the predicate's `false` branch). Getting the boundary wrong either leaks
   network threads (over-keep) or drops live deliveries (under-keep).
2. **predicate × `on*` handler attribute** (`event_listener.rs:172`): the type test must count both
   `addEventListener('change')` AND `onchange = …` (else an `onchange`-only page's MQL is wrongly
   collected). Same for WS/ES `onmessage`/`onopen`/etc. and AbortSignal `onabort`.
3. **seam × sweep-prune ordering** (`collect.rs:1718-1739` / `:1869-1896`): the keepalive **mark** runs in
   `mark_roots` (before sweep); the existing **retain-prune** runs in sweep. A target the predicate marked
   must survive the `vm_event_listeners.retain(…)` (it will, since its bit is now set) — verify the mark
   pass runs *before* every retain-prune that keys on the mark bit.
4. **observer α × callback root** (`host_data.rs:1700-1708`): moving the root from construct-time to
   active-observation must keep the **callback** rooted for the observation's duration (the binding's
   `callback` ObjectId) — the predicate marks `[callback, instance]` while ≥1 observation, releases both
   when the last observation ends.
5. **AbortSignal seam × `any()` non-root** (`roots.rs:600-618`): the deliberate non-root becomes a
   predicate-gated root — confirm this does not re-introduce the unbounded-composite-accumulation the "(k)"
   comment warns about (it does not, because the predicate requires live source-signals AND listeners, not
   mere existence).
6. **world_id-home × unbind scrub**: every per-VM registrant map must be `unbind`-cleared (it already is
   for `vm_event_listeners` / `websocket_states` / observer bindings; confirm `media_query_list_registry`
   participates — it survives unbind by design today, `media_query.rs:362-363`, document-filtered, so its
   keepalive predicate must be document-scoped to avoid rooting a prior-document MQL after rebind).

---

## §9 Test strategy (VM-test oracle — boa is the live engine)

Because boa is the live engine until S5-6, the keepalive is exercised **only by VM tests** (the
`elidex-js` `engine`-feature test suite). The decisive tests:

- **MQL keepalive (the headline, positive)**: `let cb = …; matchMedia('(min-width: 600px)')
  .addEventListener('change', cb);` with **no retained MQL reference** → force a GC → change the
  `ViewportState` width across the breakpoint → `deliver_media_query_changes()` → **assert `cb` fired**.
  (Today this silently fails — the MQL is collected. This is the test that flips green.)
- **MQL negative control (no over-rooting)**: a `matchMedia('(min-width: 600px)')` with **no listener** and
  **no retained reference** → GC → assert its registry row is **pruned** (the listener-less unreferenced
  target IS collected — proving the predicate is not a registry-membership over-root).
- **`onchange` handler counts**: `let m = matchMedia(q); m.onchange = cb; m = null;` → GC → flip → assert
  `cb` fired (the handler-attr path, `event_listener.rs:172`, must satisfy the predicate).
- **WS/ES (S5-3b)**: a listener-held OPEN connection survives a GC and **keeps delivering** a subsequent
  `message` (assert no `WebSocketClose` emitted, no force-close); a **no-listener** orphan OPEN connection
  is force-closed on GC (the genuine-orphan path preserved).
- **AbortSignal `any()` (S5-3a)**: a composite with an installed `abort` listener and live source signals
  survives a GC (predicate-rooted); a composite with no listener and no JS ref is collected (the "(k)"
  non-root behavior preserved for the no-anchor case).
- **Observer α (S5-3c)**: `new MutationObserver(cb); /* no observe, no ref */` → GC → assert collected (the
  over-root/leak fix); an observer with a live `observe()` survives across GC and its `cb` still fires.

**Out of S5-3 (rides S5-6)**: nothing — S5-3 is pure VM capability + VM tests. The producers it protects
(`deliver_media_query_changes`, and at the flip the WS/ES message pumps) are already VM-resident; S5-3 only
ensures their targets survive to be delivered to.

---

## §10 Deferred slots + open questions (per-PR cap ≤3)

### Slots
- **`#11-eventtarget-keepalive-component-migration`** (NEW) —
  - **Why deferred**: the keepalive marker is a per-VM `ObjectId` (exception (a),
    `vm_api.rs:608-609`); the ideal ECS-native form (a keepalive marker-component on the watched entity,
    the `#213` `*ObservedBy` precedent) requires the cross-DOM discriminator
    (`#11-wrapper-cache-cross-dom-discriminator`), which lands **strictly after S5** (umbrella §0). S5-3
    lands the per-VM HostData/registry form (the same per-VM-now / component-later pattern S5-2 used).
  - **Re-evaluation trigger**: world_id `world_id` discriminator lands (the wrapper-identity-component
    migration cohort).
  - **Re-evaluation date**: demand-gated (post-S5, with the world_id program).
- **`#11-eventtarget-keepalive-registrant-coverage`** (NEW, *only if the §7 split lands as 2–3 PRs*) — a
  **HARD pre-flip gate** (not a soft "in-program continuation"):
  - **Why deferred**: if S5-3a ships seam + MQL + AbortSignal-mechanism only, the WS/ES close-semantics
    change (S5-3b) and the observer over-root fix (S5-3c) are the seam's remaining clients — but they are
    **flip-precondition-mandatory**: leaving them off the seam at the flip = exactly the forbidden
    indefinite strangler (seam + N legacy roots persisting past the dev window). So this slot is a **HARD
    completion gate**: **S5-3b/c MUST complete before S5-6 (the flip)**, in the same sense C3 is a hard
    pre-flip gate for S5-6. It is a *bounded* defer (resolved at the flip), not an open-ended one.
  - **Re-evaluation trigger**: **S5-3a lands → S5-3b/c become pre-flip-mandatory (hard gate)** — they
    must be picked up and landed *before* S5-6 is allowed to proceed (not "whenever convenient").
  - **Re-evaluation date**: hard-gated by S5-6 (must complete before the flip); within that window,
    immediately after S5-3a as in-program continuation.
  - *(If plan-review chooses the single-PR option, this slot is not created — the single PR already
    lands all registrants on the seam before the flip.)*
- **`#11-eventtarget-listener-keepalive-rooting`** (EXISTING, the carve at `media_query.rs:336-351`) — this
  is the slot S5-3 **retires**: the seam + MQL wiring is the hard part; the WS/ES + observer migration are
  the seam's remaining clients (folded under the program above).

### Open questions for /elidex-plan-review
- **Q1 (the reframe — the spine)**: Confirm the mechanism is a **keepalive-predicate seam**, NOT the
  umbrella/in-code "generic any-listener root" shorthand (which is §2.8-violating over-rooting). Lean:
  **predicate seam** (the only form consistent with §2.8 + every per-interface GC-note).
- **Q2 (scope split)**: One PR, or the **S5-3a (seam+MQL+AbortSignal-mechanism) / S5-3b (WS/ES
  close-semantics) / S5-3c (observer over-root fix)** mini-program (§7)? Lean: **split**, with **only
  S5-3a as the flip-precondition** (b/c fix pre-existing latent bugs, flip-order-independent). Confirm the
  base-case exemption applies to each slice.
- **Q3 (observer α vs β)**: §5.2 — route observers through the active-observations predicate (α, fixes the
  over-root/leak, a behavior change) or keep construct/disconnect rooting and unify mechanism only (β)?
  Lean: **α** (ideal/spec-faithful; removes the immortal-disconnected-observer leak).
- **Q4 (WS/ES close-semantics)**: Confirm the spec reading that a **listener-held open connection must stay
  alive and keep delivering** (WS §7 / ES §9.2.9 state-tiers) — so the sweep force-close
  (`collect.rs:1869-1896`) becomes the predicate's `false` (no-listener-orphan) branch, NOT the
  unconditional close it is today. Lean: **yes** (the §7/§9.2.9 tiers are explicit; the current
  unconditional force-close is the bug).
- **Q5 (dispatch shape)**: §4.2 — static enum `KeepaliveClass` dispatch (shape A, recommended) vs trait
  registry (shape B)? Lean: **A** (built-in EventTargets → static dispatch, CLAUDE.md Plugin-first).
- **Q6 (predicate state-source location)**: the predicate reads `vm_event_listeners` (VmInner) AND
  `websocket_states`/`event_source_states` (HostData) — confirm threading both into `GcRoots` (modeling
  `pending_timeout_signals` + `abort_signal_states`) is the right home vs a consolidated keepalive
  side-table. Lean: **borrow the existing maps into `GcRoots`** (no new side-store; the seam is a pass,
  not storage).
- **Q7 (FileReader / VV / Screen out-of-scope)**: Confirm idle FileReader (FileAPI no GC-note,
  task-source-anchored only) and the proto-rooted VV/Screen are **correctly out of scope** (not gaps), so
  reviewers don't flag their absence from the predicate set. Lean: **out of scope** (no spec keepalive for
  an idle FileReader; VV/Screen are permanent proto-roots).

---

## §11 Verified-cites note (read before plan-review)

Two cites I was handed at pickup did **not** resolve as given and are **corrected** above (so plan-review
isn't anchored on drift):

1. **AbortController "internal `signal_id` slot" root** — the prompt framed it as a `roots.rs` root. There
   is **no** AbortController root in `roots.rs`; the controller→signal keepalive is a **trace** arm
   (`ObjectKind::AbortController { signal_id }` field `object_kind.rs:311`; trace `gc/trace.rs:391-392`).
   Corrected in §5.1. (`pending_timeout_signals` at `roots.rs:483-492` and the `any()` non-root at
   `roots.rs:600-618` **did** resolve exactly as given.)
2. **MQL seed at `media_query.rs:241`** — line 241 is the `onchange` event-handler accessor install, not
   the flip-prior seed. The seed (`last_matches = evaluate(…)` at construction) is in
   `create_media_query_list` at **`media_query.rs:255`** (registry insert at `:274-282`). Not load-bearing
   for S5-3 (it's an S5-2/MQL-creation detail), but corrected for accuracy.

One **comment-vs-reality discrepancy** to flag (not a blocker, but plan-review should know): the
`PendingTask::FileRead` root comment claims the FileReader wrapper "*is rooted via
`HostData::gc_root_object_ids` for its lifetime*" with "(Phase 4 of `#11-file-api` will hook up rooting)"
(`roots.rs:516-518`) — but a grep of `gc_root_object_ids` (`host_data.rs:1697-1717`) shows **no** FileReader
rooting (the "will hook up" is unfinished). So today an idle FileReader **is** collectible — which *agrees*
with the §5 "idle FileReader = collectible = not a gap, out of scope" position (the in-flight read is
task-rooted at `roots.rs:515-524`; only the idle case is collectible, and that is spec-correct).

Everything else resolved exactly: DOM §2.8 / §3.2.1, XHR §3.2, WebSockets §7, HTML §9.2.9, CSSOM-View §4.2,
FileAPI §6.1 (all prose-matched via webref); and the code cites `host_data.rs:1697-1717`/`:1700-1708`,
`roots.rs:349-352`/`:483-492`/`:515-524`/`:600-618`, `collect.rs:457-469`/`:1718-1739`/`:1869-1896`,
`mod.rs:1257`/`:1338-1354`, `media_query.rs:336-351`/`:384`, `mutation_observer.rs:197-203`/`:249-261`,
`vm_api.rs:608-609`, `object_kind.rs:311`/`:1618`, `event_listener.rs:131`/`:172`/`:329`/`:338`/`:194`,
`event_target_dispatch_vm.rs:392`, and the seam shape `roots.rs:21-202` / `collect.rs:52`/`:1145`.

---

## §12 Workflow

plan-verify grep against `d09829a5` → **`/elidex-plan-review` (this memo) BEFORE impl** → (per §7 split
decision) impl S5-3a in an isolated worktree (seam + MQL + AbortSignal-mechanism) → `/pre-push` (6-stage) →
`/external-converge` (Codex) → squash merge; then S5-3b (WS/ES) and S5-3c (observers) as their own
plan-reviewed base-case slices. boa untouched (VM-internal). world_id component migration stays out
(`#11-eventtarget-keepalive-component-migration`, deferred to the post-S5 world_id cohort).

**Stale-artifact rewrite (an S5-3a deliverable, not a side-effect).** The umbrella/in-code "any-listener
root" shorthand is **refuted** by this memo's §2 reframe; S5-3a's deliverables explicitly include
**rewriting that shorthand at its two live homes** so the codebase does not retain the refuted framing:
1. **the in-code KNOWN-GAP carve comment** at `crates/script/elidex-js/src/vm/host/media_query.rs:336-351`
   — the "generic 'EventTarget kept alive while it has listeners'" text → reframe to the **keepalive-
   predicate seam** (per-registrant spec-faithful predicate, NOT an any-listener root);
2. **the `#11-eventtarget-listener-keepalive-rooting` slot text** in `project_open-defer-slots.md` — the
   "GENERIC 'EventTarget alive while listenered'" framing → reframe to the predicate-seam (and mark the
   slot retired-by-S5-3, per §10).
