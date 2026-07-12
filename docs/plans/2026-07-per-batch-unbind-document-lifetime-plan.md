# Per-batch-unbind document-lifetime state — survivor-set promotion for the Custom-Elements registry + `navigator.serviceWorker` client

Per-PR-slice plan-memo. Closes the registered slot **`#11-per-batch-unbind-document-lifetime-state`**
(the S5-6b `/external-converge` R4 P1 cluster — R4-#4 CE registry + R4-#5 SW client, one root). It is
the **direct extension of the Stage-2f keystone** (`docs/plans/2026-07-s5-6b-2f-unbind-survivor-realtime-worker.md`,
merged inside the flip PR #457): 2f split `Vm::unbind` into a per-turn `unbind` + a per-document
`teardown_document` and promoted the **realtime/worker** resources into the unbind-survivor set. This
memo promotes the **next two field groups in the same class** — the Custom-Elements registry and the
page's `navigator.serviceWorker` client state — which 2f did not scope.

> **Gate**: `/elidex-plan-review` (5-agent) BEFORE any impl. Edge-dense per CLAUDE.md ("≥3 intersecting
> invariant axes … no off-the-shelf canonical algorithm for which unbind actions are per-turn vs
> per-document"): per-turn-bind lifetime × cross-DOM-aliasing safety × CE upgrade/reaction lifetime × SW
> client registration lifetime. Like 2f, it is solved **inside the survivor-set model** (a supported
> surface — custom elements + service-worker registration during load — must keep working), not carved
> out. **§6 carries the decision reviewers must adjudicate (§6.1 is CRITICAL: SELF-CONTAINED vs
> agent-scoped-EcsDom/B1-gated — this memo argues SELF-CONTAINED and must survive scrutiny).**

All cites grep-verified against worktree `per-batch-unbind-doc-lifetime` base `origin/main` = **`315ba316`**
(post-boa-deletion #458; the VM is the sole engine). Spec anchors webref-verified 2026-07-12
(`html`, `service-workers`).

---

## Decision on record — the document-lifetime CE + SW clears are MISFILED on the per-turn `unbind`; MOVE them to the per-document `teardown_document` (survivor-set promotion), and REJECT the "interim until B1" framing the in-tree comments currently assert

### The bug (R4-#4 / R4-#5, one root)

The flip's batch-bind model opens **one `with_bound` bracket per batch** — per script-eval, per UA-event,
per timer/record/observer deliver (`crates/shell/elidex-shell/src/pipeline.rs:353` initial-scripts loop;
`crates/shell/elidex-shell/src/lib.rs:299,312,323,344,…` the ~10 per-turn deliver helpers). Each bracket's
RAII `UnbindGuard::drop` runs a full `Vm::unbind` at bracket-end
(`crates/script/elidex-js/src/engine.rs:424-443`). `Vm::unbind`
(`crates/script/elidex-js/src/vm/vm_api.rs:454`) **clears the authoritative Custom-Elements registry**
(`:728-739`) **and the `navigator.serviceWorker` client state** (`:631-648`) on **every** bracket. So:

- **CE (R4-#4)**: a `customElements.define('my-el', …)` in one batch is **gone** before the next batch. The
  initial-scripts set is ONE bracket (`pipeline.rs:339-346` comment: "ONE bracket around the per-script
  eval loop + timer drain — NOT per-script"), so `define()` in script A is visible to script B *during
  load* — but the bracket closes (`unbind`) before the `flush_with_ce_reactions` / lifecycle-dispatch
  phase and before every later `eval_script`/`dispatch_event`/timer batch. A definition used by a later
  task, an upgrade of a parser-created element, or a `whenDefined()` awaited after load resolves against an
  **empty** registry — a **loss of function on the common custom-element page** (§4.13.5 "upgrade" never
  fires; §4.13.4 `whenDefined` never settles).
- **SW (R4-#5)**: `navigator.serviceWorker.register(url)` stages a `SwClientRequest` on
  `sw_client_outgoing` (`vm_api.rs:648`), which the content event loop drains **outside** any bracket
  (`crates/shell/elidex-shell/src/content/event_loop.rs:134-135` `drain_sw_client_requests`). The staging
  bracket's `unbind` clears `sw_client_outgoing` **before** the loop drains it → the `register()` never
  reaches the coordinator. The whole client registry (`sw_registrations`, `sw_ready_promise`,
  `sw_controller_scope`, the pending-promise maps) is wiped identically each batch.

### The first-principles ideal (CLAUDE.md "Ideal over pragmatic")

**A per-turn unbind is not a document-teardown event.** Stage 2f already named the two distinct
operations and built the split. This memo applies the *same* clean model to two more field groups that 2f
left on the per-turn path:

- **`Vm::unbind` (per-turn, fires every bracket)** = re-establishment boundary of a bound view over
  persistent session state. Drops only what is genuinely cross-DOM-aliasing-unsafe to carry into a
  *possible* rebind to a different `EcsDom` (non-Node wrapper caches, live collections, IDB txn rollback,
  dispatcher teardown, per-dispatch SW *worker-side* event state). It must **not** touch the CE registry or
  the `navigator.serviceWorker` **client** state.
- **`Vm::teardown_document` (existing, per-document, fires only at pipeline replacement / engine drop)** =
  release the browsing-context-scoped resources. Its trailing `unbind()` is preceded by the document
  clears (realtime close + worker terminate today; **+ the CE registry + SW client clears** after this
  memo). This is the §4.13 custom-element-registry / §3.4 `ServiceWorkerContainer` lifetime moment
  (document destruction), not a per-turn event.

CE + SW client state are **promoted into the unbind-survivor set** — the same class as `window_entity`
(retained: `vm_api.rs` bind `:224`), the primary Node wrapper (`wrapper_store.retain(kind == Node)`,
`vm_api.rs:573-576`), and the 2f-promoted realtime/worker tables. Per-turn unbind leaves them intact; the
next batch's `bind` re-shares the (now-**populated**) `Arc<Mutex<CustomElementRegistry>>` and the SW client
maps persist; `teardown_document` clears them at the real document boundary.

**Rejected alternative — the flag-hack** (`unbind(teardown: bool)` special-casing document teardown inside
the per-turn path) is NOT chosen, identical to 2f's reasoning: it moves the decision surface into every
caller ("which bool?") instead of eliminating it (One-issue-one-way). The method split already exists (2f);
this memo just files the CE/SW clears on the correct side of it.

### The framing correction this memo makes on record

The in-tree comments at `vm_api.rs:702-704,724-727` assert these clears are the cross-DOM-aliasing scrub
whose "interim form [is] unchanged **until B1**" (agent-scoped EcsDom). **That framing is wrong for the
document-lifetime subset** and this memo supersedes it (§6.1): the cross-DOM-rebind hazard the scrub guards
**does not occur in production** — a live `Vm` only ever re-binds to the **same** `EcsDom` for its whole
life; navigation allocates a **new** `Vm` (the named flip invariant, stated in-code at
`crates/script/elidex-js/src/vm/host/media_query.rs:184-190`: "*Under B1 a `Vm` never rebinds across
worlds — navigation allocates a NEW `Vm` … Reachable only via the unbound-rebind unit harness today
(non-production)*"). So preserving CE/SW state across a *same-DOM* per-batch unbind is **aliasing-safe by
construction today**, exactly as 2f established for realtime/worker (2f §6.1). B1 is the *grain*
generalization (CE/SW state → per-realm/per-document-root components, agent-scoped-EcsDom §5 req 5/6), a
**future refinement**, not a **precondition** for closing this functional gap. This memo delivers the fix
now; B1 later moves the same state onto components.

---

## §1 Scope — what this slice delivers

| Part | Surface | Row |
|---|---|---|
| A | Split: MOVE the CE-registry clears (`vm_api.rs:728-739`) + the `navigator.serviceWorker` **client** clears (the document-lifetime subset of `:631-648`) from per-turn `Vm::unbind` into `Vm::teardown_document` (before its trailing `unbind()`). Per-turn `unbind` retains **everything else** (classified §4). | R4-#4 / R4-#5 |
| B | Promote the **document-lifetime** CE fields (`ce_registry` / `ce_constructors` / `ce_constructor_to_id` / `ce_when_defined_promises` / `ce_next_constructor_id`) + the SW client fields (§4 table) into the survivor set = **removal of the per-turn clear** (they simply persist). No new field/counter. **`ce_reaction_queue` is NOT promoted** — it is a per-turn transient drained-each-checkpoint queue holding `Entity` refs, so it STAYS on the per-turn Entity-scrub set (§4, §6.3). | R4-#4 / R4-#5 |
| C | Update the misfiled in-tree comments — BOTH the `unbind`-body comments (`vm_api.rs:702-727` CE, `:633-653` SW) AND the authoritative **field doc-comments** on `host_data/mod.rs` / `VmInner` that assert the OLD lifetime: `ce_registry` (`host_data/mod.rs:540-545` "Cleared on `Vm::unbind` … neither survives an unbind crossing"), `ce_constructors`/`ce_constructor_to_id` (`:561-563` "Cleared on unbind."), + the SW client fields' docs (`VmInner`, e.g. the `sw_*` block). Rewrite per §6.1: these are document-lifetime clears moved to `teardown_document` (self-contained); the genuine cross-DOM scrubs are the wrapper/collection/observer-Entity ones (incl. `ce_reaction_queue`) that STAY. | this memo §6.1 / §6.2 |

**Explicitly out of scope** (carved / cross-referenced, §8): the SW **worker-side** per-dispatch event
state (`fetch_event_states` / `extendable_event_states` / `client_states` / `sw_clients` / `sw_outgoing`,
`:628-632` — genuinely per-dispatch-transient, STAY); the `idb_database_states` cross-turn versionchange
survival (R14, `#11-idb-connection-queue` — **same class**, its delivery machinery is a separate feature);
the eventual CE/SW **component migration** (agent-scoped EcsDom §5 req 5/6); any change to the genuine
cross-DOM scrubs that STAY (`wrapper_store` non-Node retain, `live_collection_states`, IntersectionObserver
`root: Entity`, DnD/touch, IDB txn rollback).

**Layering-check (CLAUDE.md Layering mandate).** The moved surface is **VM-lifecycle only** — clearing
per-realm `ce_*` handles + `VmInner` SW client side-tables at the correct `bind`/`unbind` boundary. No
engine-independent DOM/form/selector/CSSOM algorithm is moved or added, so the crate-mapping table is N/A
(same as 2f, which was VM-lifecycle + marshalling).

**Touch-time-split note (CLAUDE.md 1000-line debt).** `vm_api.rs` (≈1273 L) and `host_data/mod.rs`
(≈1997 L) are both >1000. The discipline is *considered* and does **not fire**: this touch is
**net-negative** — Part A is a verbatim MOVE of two clear-blocks within `vm_api.rs` (no new algorithm) and
Part B is removal-of-clears. No substantive >50 LoC growth (the Axis-5 review backstop trigger). The
standing `host_data/mod.rs` decomposition debt is tracked by `#11-host-data-full-decomposition`; this slice
does not enlarge it.

---

## §2 Coupled invariants

Four axes intersect; each pairwise intersection is named so plan-review can check them independently.

- **per-turn-bind lifetime × CE upgrade/reaction lifetime** — the CE registry is populated by
  `customElements.define()` and consumed by *later* tasks: parser-created element upgrades (§4.13.5 "upgrade
  an element"), scripted `document.createElement('my-el')`, `whenDefined()` promise resolution (§4.13.4
  when-defined promise map). These consumers span **many** brackets; a per-turn clear bounds the registry
  to **one** bracket. The correct bound is the **document lifetime** (the CustomElementRegistry is owned by
  the global/realm, §4.13.4).

- **per-turn-bind lifetime × SW client registration lifetime** — `register()` stages an outbound request
  drained by the event loop **one bracket later** (`event_loop.rs:134`, outside any bracket); the whole
  `ServiceWorkerContainer` client state (§3.4: `controller` §3.4.1, `ready` §3.4.2, registrations) is
  document-scoped. A per-turn clear both (a) loses the staged request before the drain and (b) resets the
  client registry a page reads across turns.

- **cross-DOM-aliasing safety × per-turn vs per-document** — the aggressive clears in `unbind` exist
  because two `EcsDom::new()` worlds share entity-index space (lesson #195). **That hazard materialises
  ONLY at a document boundary** (a fresh `EcsDom`), and — the load-bearing fact — **no live `Vm` ever
  rebinds to a different `EcsDom` in production** (§6.1; `media_query.rs:184-190`). A per-turn unbind is a
  **same-DOM re-establishment**. The CE `ce_constructors`/`ce_when_defined_promises` are per-VM `ObjectId`s
  that **ride the VM heap** across per-turn unbind (unbind clears pointers/caches, never the heap); the SW
  client maps are `ObjectId`/`String`-keyed likewise. So keeping them across a same-DOM turn is
  aliasing-safe by construction (identical to 2f's WS/SSE `ObjectId`-keyed state). This axis is what makes
  the slice **self-contained** (§6.1), NOT B1-gated.

- **survivor set is purely `ObjectId`/`String`-keyed × NO `Entity`-keyed exception** — the promoted CE
  fields value/key on per-VM `ObjectId` (`ce_constructors` / `ce_constructor_to_id` /
  `ce_when_defined_promises`) or are name-keyed realm state (`ce_registry`), and the SW client maps on
  `ObjectId`/`String` — **all ride the VM heap / are index-internal, none holds an `Entity`**. The **one
  `Entity`-holder in the CE block — `ce_reaction_queue`** (`vm_api.rs:717` comment "every variant holds an
  `Entity`") — is deliberately **NOT promoted**: it is a **transient processing queue drained at each
  script-execution / microtask checkpoint** by `flush_ce_reactions` (`host_data/mod.rs:550-552` doc:
  "drained at script-execution / event-dispatch / microtask checkpoints"; `interpreter.rs:54`,
  `natives_timer.rs:281`, `define.rs:204`, `engine.rs:159`), NOT document-lifetime authoritative state, so
  it **STAYS** on the per-turn Entity-scrub set (alongside `mutation_observers.clear_pending_records()` at
  `:706` — the same "per-DOM `Entity` refs, drop each turn" class). Keeping it per-turn is a **no-op in the
  well-behaved (drained-empty) case** and preserves the current Entity-scrub posture with **zero change to
  the reaction queue's behaviour** (§6.3). This is what makes the survivor set self-contained with **no
  `Entity`-keyed exception to reason about** (§6.1).

**ECS-native side-store check (CLAUDE.md "Side-store→component 判定ルール").** Do the promoted fields
belong on ECS components? **Under today's model, no — they hit exception (a)** (`ce_constructors` /
`ce_when_defined_promises` value on per-VM `ObjectId`; the SW client maps on `ObjectId`/`String`); the
**unbind-survivor set is the correct ECS-native mechanism** (this state's lifetime is the document, not the
turn — the same statement 2f made for realtime/worker). **Under B1 (agent-scoped EcsDom) they migrate to
per-realm/per-document-root components** (agent-scoped-EcsDom **§5 req 5** = the per-context state grain
rule; **req 7** = the multi-realm axis that makes "the CustomElementRegistry is per-realm, one per
same-agent Window" expressible) — that is the future grain refinement this slice does not block and does
not pre-empt (it would be a lone-outlier migration before the realm axis exists — One-issue-one-way).

---

## §3 Spec coverage map

| Spec section | Step / concept | Branch (2 per row: same-doc turn vs document destruction) | Touch (clear site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §4.13.4 "The CustomElementRegistry interface" (`#customelementregistry`; `define` `#dom-customelementregistry-define`) | the per-global custom element **registry** (name→definition) populated by `define()` | (i) same-doc turn → **survive** (`define()` from an earlier batch must persist); (ii) document destroy → **clear** | `ce_registry` clear `vm_api.rs:728-731` (MOVE→`teardown_document`) | ✓ | yes (`define()`) |
| WHATWG HTML §4.13.3 "Core concepts" (custom element **definition** `#custom-element-definition`) | `CustomElementDefinition.constructor` = per-VM `ObjectId`; `constructor_id` indexes `ce_constructors` | (i) same-doc turn → survive (ObjectId rides heap); (ii) doc destroy → clear | `ce_constructors` / `ce_constructor_to_id` / `ce_next_constructor_id` `vm_api.rs:736-739` (MOVE) | ✓ | no (structure) |
| WHATWG HTML §4.13.4 "The CustomElementRegistry interface" (when-defined promise map `#when-defined-promise-map`) | per-registry map name→Promise, settled on later `define()` | (i) same-doc turn → survive (else `whenDefined()` never settles); (ii) doc destroy → clear | `ce_when_defined_promises` `vm_api.rs:738` (MOVE) | ✓ | yes (`whenDefined()`) |
| WHATWG HTML §4.13.5 "Upgrades" ("upgrade an element" `#concept-upgrade-an-element`, "try to upgrade" `#concept-try-upgrade`) | later upgrade of a created/parsed element reads the registry (a downstream **consumer**, spans many tasks) | (i) upgrade in a later batch → reads the **surviving** registry; (ii) — (no clear site; motivates survival) | consumer of the above (no clear site) | ✓ | yes (parser/createElement) |
| WHATWG HTML §8.1.4.4 "Calling scripts" — "clean up after running script" (`#clean-up-after-running-script`) | the per-turn bracket = a script-task checkpoint, NOT a document boundary | (i) every bracket-end → per-turn `unbind` fires (checkpoint + pointer-unbind); (ii) never tears down document state | `with_bound` `engine.rs:424` (RAII `unbind`) | ✓ | no |
| W3C Service Workers §3.4 "ServiceWorkerContainer" (`controller` §3.4.1, `ready` §3.4.2, `register` §3.4.3 `#dom-serviceworkercontainer-register`, `getRegistration(s)` §3.4.4/§3.4.5) | the page's SW **client** state (controller, ready promise, registrations, pending register/unregister promises, outbound request queue) | (i) same-doc turn → survive (incl. `sw_client_outgoing` for the out-of-bracket drain); (ii) doc destroy → clear. **Worker-side `:628-632` = per-dispatch, STAY (separate branch)** | SW client clears `vm_api.rs:639-648` (MOVE the document-lifetime subset) | ✓ | yes (`register()` etc.) |

**Spec takeaway**: the CE registry and the `ServiceWorkerContainer` client are **document-scoped** by
spec; none is script-task-scoped. The per-turn clear is a spec-granularity defect the flip's bracket
multiplication turned from latent (boa's cheap per-call swaps never cleared authoritative state this way)
into active — the same shape 2f found for realtime/worker.

**Trust boundary (user-input audit).** No parse/eval/marshal path changes. `define()` / `register()` /
`whenDefined()` are already-consumed surfaces; this slice changes only *when their state is cleared*
(document boundary, not turn). No new external input.

---

## §4 Per-field classification (per-turn STAY vs per-document MOVE) — the load-bearing table

Extends the 2f §4 classification. Read the full `unbind` body `vm_api.rs:454-895`. **Only the two
document-lifetime blocks MOVE**; everything else is genuinely per-turn or a genuine cross-DOM scrub that
STAYS.

### CE block (`vm_api.rs:728-739`)

| Field | Type | Disposition | Why |
|---|---|---|---|
| `ce_registry` | `Arc<Mutex<CustomElementRegistry>>` | **MOVE** | §4.13.4 authoritative name→definition registry; document/realm lifetime; `define()` from an earlier batch must survive |
| `ce_constructors` / `ce_constructor_to_id` | `HashMap<u64,ObjectId>` / `HashMap<ObjectId,u64>` | **MOVE** | per-VM ctor `ObjectId` rides the heap; `new.target`→id SoT; document lifetime |
| `ce_when_defined_promises` | `HashMap<String,ObjectId>` | **MOVE** | §4.13.4 when-defined promise map; document lifetime |
| `ce_next_constructor_id` | `u64` | **MOVE** (do NOT reset to 0 per-turn) | a per-turn reset to 0 recycles ctor ids against surviving `ce_constructors` entries — the id-collision class the `bind_epoch` `next_id` comment guards (`vm_api.rs:265-273`) |
| `ce_reaction_queue` | `Arc<Mutex<VecDeque<CustomElementReaction>>>` | **STAY** (per-turn) | NOT document-lifetime state: a transient queue drained each checkpoint by `flush_ce_reactions` (empty at bracket-end in the well-behaved case) + the **one `Entity`-holder** in the CE block; leave it on the per-turn Entity-scrub (like `mutation_observers.clear_pending_records()`) — behaviour-unchanged, no-op when empty (§2 axis 4, §6.3) |

### SW `navigator.serviceWorker` **client** block (`vm_api.rs:639-648`) — all MOVE (survive)

**MOVE** (document-lifetime, survive per-turn unbind): `pending_registration_promises` /
`pending_unregister_promises` (`HashMap<String,Vec<ObjectId>>`, GC-force-marked) · `sw_ready_promise`
(`Option<ObjectId>`, GC-force-marked) · `sw_registrations` (`HashMap<String,SwRegistrationEntry>`, pure
data) · `sw_registration_states` / `service_worker_states` (`HashMap<ObjectId,String>`, wrapper-brand —
GC-sweep-pruned) · `sw_controller_scope` (`Option<String>`) · `sw_messages_enabled` (`bool`) ·
`sw_message_buffer` (`Vec<(String,String)>`) · `sw_client_outgoing` (`Vec<SwClientRequest>`, plain IPC data
— the R4-#5 queue the event loop drains a bracket later). The §3.4 `ServiceWorkerContainer` client state is
the document's.

> **⚠ REVIEW ROUND-TRIP (`/code-review` finding [1] → Codex #459 R1 P1)** — the wrapper-brand maps
> `sw_registration_states` / `service_worker_states` are keyed by the `ServiceWorkerRegistration` /
> `ServiceWorker` wrapper `ObjectId` (non-Node, `WrapperKey::scope`, dropped every unbind by
> `wrapper_store.retain(kind == Node)`). `/code-review` flagged that surviving them could leave a stale
> entry mis-branding a recycled `ObjectId`, and they were briefly reclassified **STAY**. **Codex R1 P1
> reverted that**: clearing the brand per-turn makes a JS-**retained** wrapper an **illegal receiver**
> (`require_registration_scope` fails) after the first unbind. The correct behaviour is **MOVE (survive)**:
> the **GC sweep already prunes a brand entry when its wrapper `ObjectId` is collected**
> (`gc/collect.rs:1757` `.retain(marked)`; the `sw_registrations` registry-walk keeps live ones marked) —
> so a retained (reachable) wrapper keeps its brand while a dropped-and-collected one leaves no stale entry.
> The `/code-review` stale-entry concern was a false positive (it missed the GC-sweep prune). Pinned by
> `retained_sw_registration_wrapper_survives_per_turn_unbind`. (Cross-batch registration wrapper *identity*
> / SameObject across a `wrapper_store` drop = the deferred `#11-cross-batch-wrapper-identity`, unchanged.)

> **⚠ AS-BUILT (Codex #459 R3 — the wrapper is part of the identity unit, at BOTH ends)** — R3 surfaced the
> two remaining faces of the same root, both real (P2 → IMPORTANT), both a coherent completion (NOT a new
> mechanism; the ≥2-round self-root-check held — see the resume-state memo):
> - **R3-1 (CE registry WRAPPER survives)** — `custom_element_registry_instance` was initially classified a
>   per-turn STAY ("re-minted on next bind"). That is **WRONG for the same reason R2's SW wrapper was**:
>   `globalThis.customElements` is an eager data property installed ONCE (`register_globals` at `Vm::new`,
>   never re-run per bind), so after the surviving registry data + a dropped wrapper slot, the next access
>   mints a SECOND wrapper and `convert_custom_element_registry_member` classifies the page's own
>   `customElements` as `Foreign` (`createElement(x, { customElementRegistry: customElements })` throws
>   NotSupportedError). Reclassified **MOVE (survive per-turn, cleared at `teardown_document`)**, in lockstep
>   with the CE data. Cross-DOM-safe by construction (same-DOM rebind only). Pinned by
>   `custom_element_registry_wrapper_identity_survives_per_turn_unbind`.
> - **R3-2 (teardown DROPS the surviving SW wrappers)** — R2 made `unbind` RETAIN the Scope-keyed
>   `ServiceWorkerRegistration` / `ServiceWorker` `wrapper_store` entries but `teardown_document` cleared only
>   the data + brand rows, leaving stale wrapper entries. A later same-`Vm` re-`register()` of the same scope
>   would hit `intern_wrapper`'s cached `ObjectId`, SKIP the alloc closure that repopulates
>   `sw_registration_states` / `service_worker_states`, and return a dead-brand receiver. Teardown now
>   `remove_wrapper_keyed`s the surviving Registration/Worker entries in lockstep with the data/brand clear.
>   Pinned by `teardown_document_drops_sw_registration_wrapper_so_reregister_is_valid`.
>
> Net invariant: a **document-lifetime wrapped identity = {data, brand, wrapper}** — `unbind` RETAINS all
> three, `teardown_document` CLEARS all three, uniformly for the SW registration/worker unit AND the CE
> registry. The by-construction form (per-document-root component, despawn auto-clears the unit) stays
> deferred to agent-scoped `EcsDom` (B1, §5 req5+req7) — genuinely unavailable now, so uniform manual
> completion is the correct interim, self-contained per §6.1.

### STAYS (per-turn or genuine cross-DOM scrub — this slice does NOT touch)

| Field(s) | Site | Why STAY |
|---|---|---|
| SW **worker-side** per-dispatch: `fetch_event_states` / `extendable_event_states` / `client_states` / `sw_clients` / `sw_outgoing` | `:628-632` | genuinely per-dispatch-transient (a service-worker Vm's event handling); the comment's "retained `Client` wrapper must not read the prior snapshot" is a per-dispatch concern, not client-registry survival |
| Dispatcher teardown / live-range / node-iterator / tree-walker / selection clears | `:485-500,654-660` | strict per-bracket pair with `bind`'s dispatcher install (2f §4; the `displaced.is_none()` assert `bind`-side) |
| global `HostObject.entity_bits = 0` | `:531-537` | post-unbind null-safety for `entity_from_this` |
| `live_collection_states` / `wrapper_store.retain(kind==Node)` / DnD / touch / IntersectionObserver `root: Entity` scrub | `:547,573-576,589-591,688-698` | genuine cross-DOM aliasing (Entity/recycled-ObjectId); same-DOM-safe to keep clearing; out of scope (E2 / `#11-cross-batch-wrapper-identity`) |
| IDB txn rollback + `idb_*_states` clears | `:603-616` | txn rollback is per-bracket-correct (the backend `IdbTransaction` has no Drop-rollback; leaving it open blocks the next bind) — 2f §4. **`idb_database_states` cross-turn survival is the sibling R14 case → `#11-idb-connection-queue`, §8** |
| `mutation_observers.clear_pending_records()` / stale slot signals / `NotifyMutationObservers` microtask strip | `:706,758-765` | per-DOM Entity refs; per-turn |

This table is the load-bearing claim plan-review must re-derive from the full body and challenge each
MOVE (esp. `ce_reaction_queue` §6.3 and the SW worker-side/client-side split).

---

## §5 Per-document teardown entry-point — reuses the 2f-wired set (no new sites)

`Vm::teardown_document` (`vm_api.rs:913-961`) already fires **exactly once** per document destruction at
the boundaries 2f enumerated + verified idempotent, and this slice adds the CE/SW clears **before its
trailing `unbind()`** (they need no live handle — pure map clears — so ordering vs the realtime/worker
close is free; place them adjacent to `teardown_workers()` at `:957`). The wired boundaries:

| # | Boundary | Site (`315ba316`) |
|---|---|---|
| 1 | Top-level cross-document nav / traversal rebuild | `app/navigation.rs:387` (`teardown_document()` before `build_pipeline_from_loaded`) |
| 2 | Content-thread nav chokepoint | `content/navigation.rs:201` |
| 3 | Event-loop teardown (Shutdown) | `content/event_loop.rs:340` |
| 4 | Single-iframe document destruction | `content/iframe/lifecycle.rs:177` |
| 5 | Engine Drop backstop (any pipeline drop, incl. panic-unwind) | `engine.rs:793-814` (`impl Drop`, runs `teardown_document` iff not-already-run) |

**Idempotency**: after the first `teardown_document` the CE/SW maps are empty → the Drop backstop (#5)
re-invoking finds them empty = no-op, matching the realtime/worker snapshot-empty argument (2f §5). No new
leak/double-clear surface: the CE/SW clears are pure `.clear()` (no external resource, unlike the WS Close /
worker terminate), so a double-fire is trivially safe. **Plan-review need only confirm** the CE/SW clears
inherit the *same* once-firing set 2f verified (no CE/SW-specific extra boundary).

---

## §6 Open design questions (for plan-review)

### §6.1 (CRITICAL) SELF-CONTAINED, or gated on agent-scoped EcsDom (B1)?

**Investigation finding: SELF-CONTAINED. Landable now; NOT blocked on agent-scoped EcsDom / world_id /
`#11-cross-batch-wrapper-identity`.** This memo asserts it and asks plan-review to attack it. Evidence
(mirrors 2f §6.1, adapted):

1. **No production cross-DOM rebind.** A live `Vm` re-binds only to the **same** `EcsDom` for its whole
   life; navigation allocates a **new** `Vm` (pipeline owns `dom` + `runtime` together — `lib.rs:203,209`;
   navigation replaces the whole pipeline after `teardown_document` — `app/navigation.rs:387-405`,
   `content/navigation.rs:201`). No temporary/throwaway rebind exists: DOMParser/inert-document parsing
   builds a **second Document entity in the same `EcsDom`** (`vm/host/dom_parser.rs:292-297` via
   `elidex-form`, no second `bind`); `teardown_document`'s internal `bind` re-binds the **same** `ctx.dom`;
   `bind_worker` is a **separate** worker Vm on its own `EcsDom`, not a rebind. In-code:
   `media_query.rs:184-190` ("cross-`EcsDom` rebind does not occur in production … Reachable only via the
   unbound-rebind unit harness today (non-production), where it is inert").
2. **CE ctor / SW promise identity rides the heap.** `ce_constructors` / `ce_when_defined_promises` value on
   per-VM `ObjectId`s that persist on the VM object heap across per-turn unbind (unbind clears
   *pointers + caches + side-tables*, never the VM/heap). So preserving the maps keeps the ids valid and
   correctly keyed — no wrapper_cache round-trip (the E2 getter surface), so E2 cannot bind here.
3. **Same-DOM re-establishment.** Per-turn unbind re-binds the SAME `session`/`dom`/`document`; the
   cross-DOM entity-index-aliasing hazard motivating the aggressive clears exists ONLY at a document
   boundary. Keeping CE/SW state across a same-DOM turn is aliasing-safe by construction — it does not need
   agent-scoped EcsDom.

**The counterexample plan-review MUST weigh — R14 `idb_database_states`.** The S5-6b R14 finding deferred
`idb_database_states` cross-turn survival to agent-scoped EcsDom (flip-kickoff; `#11-idb-connection-queue`).
Why is CE/SW self-contained but that deferred? Two distinct reasons, both to be confirmed by plan-review:
(a) `idb_database_states` survival is coupled to the **IDB versionchange *delivery* machinery** (fire on
same-VM other connections, cross-tab, block/wait — a **feature** beyond survivor promotion), whereas CE/SW
survival is pure loss-of-function recovery with existing consumers (`flush_ce_reactions`,
`drain_sw_client_requests`); (b) the R14 note predates the crystallized 2f §6.1 self-contained argument and
was conservative. **If plan-review concludes the survivor-promotion *alone* (not the delivery machinery) is
self-contained for `idb_database_states` too**, that is a finding for `#11-idb-connection-queue` (its
survivor-promotion could ride the same argument), **not** a reason to gate CE/SW. The adversarial question:
is there ANY per-VM `Entity`-keyed (not `ObjectId`-keyed) CE/SW field whose survival would alias on the
non-production rebind path such that a future harness/test breaks? (`ce_reaction_queue` is the only
`Entity`-holder — §6.3.)

### §6.2 Comment-rewrite scope (Part C) — how much to say

The misfiled comments (`vm_api.rs:702-727` CE, `:633-653` SW) **and the field doc-comments**
(`host_data/mod.rs:540-545` "neither survives an unbind crossing", `:561-563` "Cleared on unbind.", the SW
`VmInner` field docs) currently frame the clears as the cross-DOM-aliasing scrub "interim until B1" / assert
the state does not survive. Part C rewrites them to: *these are document-lifetime clears, moved to
`teardown_document`; the state now SURVIVES per-turn unbind; the genuine cross-DOM scrubs are the
wrapper/collection/observer-Entity ones (incl. `ce_reaction_queue`) that STAY (self-contained per §6.1, not
B1-gated)*. Question: does plan-review want the `⚠ SUPERSEDED → agent-scoped World` forward-pointer
**removed** for these blocks (the fix lands, not defers), or **retained-and-annotated** ("the *grain*
migration to per-realm/per-document-root components still rides B1 §5 req 5 / req 7; the *survival* is fixed
here")? Recommend the latter — the survival is fixed now, the component-grain refinement is still B1's.
⚠ The field doc-comments are the **canonical lifetime SoT** a reader consults; leaving them saying "does
not survive" after the fix ships the opposite of the behaviour (plan-review Agent-2 2b finding).

### §6.3 `ce_reaction_queue` — STAY (per-turn) is the right classification (resolved; confirm)

§4 classifies `ce_reaction_queue` **STAY** (per-turn), NOT promoted — it is a transient queue drained each
checkpoint by `flush_ce_reactions` and the one `Entity`-holder in the CE block, so leaving it on the
per-turn Entity-scrub is behaviour-unchanged (a no-op when drained-empty). This deliberately **avoids** the
earlier draft's tension (a MOVE would have rested on either an unconfirmed "always empty at bracket-end"
premise or the comment-invariant same-DOM `Entity` validity — the one place the "interim until B1" framing
retains force, Codex plan-review R-analog IMP). By keeping it STAY, the survivor set is purely
`ObjectId`/`String`-keyed with **no `Entity`-keyed field to reason about**, so §6.1's self-contained claim
holds without exception. **Plan-review CONFIRMED (focused re-check on the fix-delta, fresh agent)**: (a) **every** bracket type that
can enqueue a CE reaction drains it *inside the same `with_bound`* before that bracket's `unbind` — `eval`
(`interpreter.rs:54`), `drain_timers` (`natives_timer.rs:281`), `dispatch_event` (`engine.rs:327`
`settle_tasks_and_reactions`→`flush_ce_reactions`), and **all ~10 `deliver_*` brackets** which wrap
`deliver_*`+`engine.drain_reactions(ctx)` in one bracket (`lib.rs:344-531`; load-bearing because
`deliver_mutation_records` `mutation_observer.rs:440-450` enqueues-only and defers the drain to the
post-deliver `drain_reactions` *in the same bracket*). So the queue is **drained-empty at `unbind`** and the
per-turn clear is a genuine no-op; (b) no consumer requires a reaction to survive a bracket boundary
(enqueue→drain is always intra-bracket); (c) registry survival *newly enables* later-batch reactions
(previously suppressed by the empty registry) — each is drained in its own bracket, so it **fires
correctly** (the intended fix), no leak. **One pre-existing residual** the re-check surfaced: the
`MAX_CE_DRAIN_ITERATIONS = 16` overflow path (`custom_elements/flush.rs:33,92`) can leave a non-empty tail
whose comment claims it "defers to the next checkpoint" — the per-turn clear drops that tail. This is
**pre-existing and unchanged by this slice** (the queue is cleared per-turn today too; registry survival
does not alter the tail's fate) → out of scope; flag as a separate pre-existing CE-reaction-overflow
concern only if a real repro appears (do not mint a speculative slot).

### §6.4 Test strategy — new coverage inverts any per-turn-clear assertion

NEW coverage = **multi-batch survival**:
- **CE**: `define('my-el', C)` in batch A → in batch B (fresh bracket) `document.createElement('my-el')` /
  parser-upgrade / `customElements.get('my-el') === C` / a `whenDefined('my-el')` awaited across the
  bracket resolves. Assert the registry survived ≥2 per-turn unbinds.
- **SW**: `navigator.serviceWorker.register(url)` staged in a script batch → assert the `SwClientRequest`
  survives the bracket's `unbind` and is present for `drain_sw_client_requests` (the event-loop drain);
  `navigator.serviceWorker.ready` / `controller` read across ≥2 turns is stable.
- **Teardown still clears**: after `teardown_document`, the CE registry + SW client maps are empty. **Re-home
  discipline (2f §6.4)**: any existing test asserting per-turn `unbind` clears CE/SW must move its clear
  assertion onto the `teardown_document` path (preserving intent — teardown DOES clear, at the document
  boundary), while the per-turn `unbind` test flips to assert **survival**. Candidate re-home targets
  (grep-located, confirm each at impl): `crates/script/elidex-js/src/vm/tests/tests_custom_elements.rs`
  (9 `vm.unbind()` sites incl. `:39/:53/:77/:538/:729/:773/:1124/:1236/:1319`),
  `tests/tests_service_worker.rs:750`, `tests/tests_service_worker_client.rs`. Not every `unbind()` call
  asserts a CE/SW *clear* — the impl must inspect each to distinguish clear-assertions (re-home) from
  incidental teardown (leave).

---

## §7 Sub-commit split + acceptance

Base-case terminal slice under the approved S5 umbrella + this plan-review (CLAUDE.md "base case =
narrowly-scoped per-PR slice") → **one PR**, internally sequenced so survival lands with its regression
proof. **CE-vs-SW split considered + rejected**: although CE (§4.13, `enqueue_upgrade_walk`) and SW (§3.4,
`drain_sw_client_requests`) have independent delivery machinery and spec surfaces, they share **one root**
(the per-turn `unbind` misfiling document-lifetime clears) and **one mechanism** (the 2f survivor-set
move), so a single plan-review + single PR is correct; the sub-commit split (a = move, b = tests) already
isolates the two field groups for independent review.

| Sub-commit | Content | Acceptance |
|---|---|---|
| **a** | Part A: MOVE the CE clears + the SW client document-lifetime clears from `unbind` into `teardown_document` (adjacent to `teardown_workers()`, before the trailing `unbind()`). `unbind` body then matches the §4 STAY table. | Compiles; `unbind` no longer clears `ce_*` / the SW client subset; `teardown_document` clears them + still force-closes realtime/workers. |
| **b** | Part B + tests: the clears are already gone with the MOVE (no separate removal); keep `ce_next_constructor_id` / conn-id counters unreset. **Multi-batch CE survival test + multi-batch SW client survival test** (§6.4); re-home any per-turn-clear assertion onto `teardown_document`. | Survival tests green: CE registry + SW client survive ≥2 per-turn unbinds and are consumable; `teardown_document` still empties them. |
| **c** | Part C: rewrite the misfiled `vm_api.rs` comments (§6.2 disposition) — document-lifetime clears, self-contained, grain-migration still B1's. | Comments match the shipped classification; no `crates/` logic change. |

**Push gate**: `mise run ci` + `/pre-push` (6-stage) + `/external-converge` (Codex) — edge-dense, so the
external pass is the full convergence loop (the R4 finding originated in exactly this loop).

---

## §8 Carve list

- **`#11-idb-connection-queue`** (EXISTING — sibling, requirements cross-referenced) — the load-bearing
  reason `idb_database_states`' clear **cannot ride this slice** is that it is **entangled with the
  per-bracket IDB txn rollback** (`vm_api.rs:603-616`): the backend `IdbTransaction` has no Drop-rollback, so
  the txn state must be aborted every turn (genuinely per-turn, STAYS §4), and `idb_database_states` clearing
  sits inside that same per-bracket block — un-MOVing just the connection registry from it is a larger
  restructure than a clean clear-block move. Its cross-turn survival *also* needs the versionchange
  *delivery* machinery (fire on same-VM other connections + cross-tab + block/wait), a separate unbuilt
  feature. **Finding to record at that slot**: per §6.1, `idb_database_states`' *survivor-promotion alone*
  (untangled from the txn rollback) is plausibly self-contained — re-eval the R14 "only safe under
  agent-scoped EcsDom" claim when that slot lands. Not fixed here.
- **`#11-cross-batch-wrapper-identity`** (E2, EXISTING) — the `[SameObject]` getter-attribute survival
  (`classList`/`dataset`/`style`) across same-DOM turns. Untouched: CE/SW state is constructor-result /
  authoritative-registry, not per-access getters (§6.1). Remains its own slot.
- **CE/SW → per-realm/per-document-root component migration** (agent-scoped EcsDom **§5 req 5** [grain rule]
  **+ req 7** [multi-realm axis], NO new slot — folded into that program) — under B1 the CustomElementRegistry
  is per-realm (one per same-agent Window) and the SW client per-document-root; the survivor-set fields
  migrate to components then. This slice delivers the survival now; B1 refines the grain. Tracked in
  `docs/plans/2026-06-agent-scoped-ecsdom-world.md` §5.
- **`#11-sw-controller-seed-on-navigation`** (EXISTING, S5-6b R4-#3) — distinct: `seed_sw_client` not called
  on rebuild (`controller===null` after a controlled reload). Orthogonal to survival across per-turn unbind;
  not this slice.

**Landing bookkeeping (record at merge).** This slice **closes** `#11-per-batch-unbind-document-lifetime-state`
**self-contained** — so the ledger slot's disposition must be reconciled: `project_open-defer-slots.md`
currently records it `Trigger: B1 (agent-scoped EcsDom) impl` / `Date: B1-gated` (and MEMORY.md / the
flip-kickoff R14 note frame it as B1-gated). At landing, update the ledger to record it CLOSED
self-contained-now, with the **grain refinement** (CE/SW → per-realm/per-document-root components) noted as
the residual that rides agent-scoped EcsDom §5 req 5 / req 7 (a fold into that program, not a surviving
`#11-` slot). cap −1.
