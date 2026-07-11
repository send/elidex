# S5-6b Stage 2e — Service Worker cutover (boa→VM)

Per-PR-slice plan-memo under the S5 umbrella (`docs/plans/2026-06-s5-flip-boa-deletion-umbrella.md`)
and the S5-6 flip memo (`docs/plans/2026-07-s5-6-flip-boa-deletion.md` — §3.4 rows B7/B11/B16/B17,
§4.3.6 SW thread, §4.3.2 cross-context drains). This memo does **not** re-derive the umbrella's
bracket model (§4.1) or the E4 strangler rule (§8) — it references them and scopes only the
SW-thread + SW-client-request + client-id surface (memo rows **B7, B11, B17**; B16 iframe
parent-messaging is 2f, B13 pending-focus already landed in stage 2d).

> **Gate**: `/elidex-plan-review` (5-agent) BEFORE any impl (CLAUDE.md "Edge-dense work =
> multi-PR program + 実装前 plan-review 必須" — this stage crosses SW-thread-spawn × storage-broker
> ownership × promise-settle IPC × client-identity coherence). §6 carries the open design questions
> reviewers must scrutinize; §7 is the sub-commit split.

## Decision on record — OPTION A: full-wire all 4 `SwClientRequest` arms

The PM chose **full-wire** over a settle-stub. Rationale, code-grounded:

- The VM's SW **client-request surface is already COMPLETE** — the four `SwClientRequest` arms are
  all queued (`container.rs:158` Register, `registration.rs:182` Update, `registration.rs:205`
  Unregister, `worker.rs:97` PostMessage), and the settle machinery exists on the VM side:
  Register/Update push a promise into `pending_registration_promises` (`container.rs:153-157`,
  `registration.rs:177`), Unregister into `pending_unregister_promises` (`registration.rs:200`),
  both settled by `deliver_sw_client_update` (`deliver.rs:38` — `Registered` at `:40` settles
  register+update, `Unregistered` at `:52` settles unregister). Only the **shell IPC wiring** is
  missing.
- Dropping Update/Unregister would leave their promises **permanently pending** (a `Vec` of
  rooted promises the VM never settles) — strictly **worse than boa**, which never exposed
  `update()`/`unregister()`/`ServiceWorker.postMessage` on this back-channel at all (boa's bridge
  had only `queue_sw_register`; see §8). A hung promise is an observable regression; a missing
  method is not. Full-wire is the only choice that is ≥ boa on every arm.
- Edge-dense → this memo + plan-review precede authoring; the impl splits into sub-commits (§7).

**What "full-wire" means (reconciled with §6.3).** "Full-wire" = **settle-wire all four arms so no
promise hangs** — every awaiting arm reaches a reply that resolves/rejects its rooted promise. It
does **not** pre-decide the Update arm's re-fetch *depth*: whether Update does a minimal
re-validate+settle or the full `#update` re-fetch+install is the §6.3 open question, and the full
*Update* algorithm is carved to §9 (`#11-sw-update-full-algorithm`). The settle-path is in scope for
all four arms; the algorithm depth of one arm is the sole open axis.

**Oracle scope.** The new SW settle-path tests (§7 2e-b) sit **within** the flip's
equivalence-with-boa oracle: they assert the VM's *already-queued* promises settle — i.e. they
complete the cutover of surface the VM already exposes — not new product behavior. This is unlike the
§0.2 A2c peel (which had no VM machinery standing behind it to hang); here the machinery exists and
merely needs its reply wired, so settling it is cutover-completion, not feature growth.

All cites grep-verified against worktree `s5-6b-flip` HEAD `98fdf07e` (2026-07-11). Service
Workers spec anchors webref-verified 2026-07-10 (`.claude/tools/webref`, source `service-workers`).

---

## §1 Scope — what 2e delivers (three parts)

`PipelineResult.runtime` is already swapped boa→VM (`ElidexJsEngine`), and the *SW* `.bridge()`
surface is exactly the two errors below (`event_loop.rs:123`, `navigation.rs:141`) — but ~11 non-SW
`.bridge()` sites remain live/non-compiling (B5/B14/B15/B18/B19), owned by later stages (2f).
Stages 2a–2d already converged the storage-change, IDB-versionchange, pending-focus, and
worker-message drains (`event_loop.rs:97,107,157,182`). The **two remaining SW compile errors** are:

- `content/event_loop.rs:123` — `.bridge().drain_sw_register_requests()` (Part 2, B7).
- `content/navigation.rs:141` — `.bridge().client_id()` (Part 3, B17).

Plus the not-yet-invoked VM SW-thread entry (Part 1, B11) — `vm/sw_thread.rs:92` exists but is
called nowhere; the coordinator still spawns boa (`sw_coordinator.rs:192`).

| Part | Surface | Memo row |
|---|---|---|
| 1 | SW thread spawn swap (boa 4-param → VM 6-param) | B11 / §4.3.6 |
| 2 | `drain_sw_register_requests` → `drain_sw_client_requests` (4-arm) + IPC + coordinator handlers + settle-deliver | B7 / §4.3.2 |
| 3 | `client_id()` → shell-owned page client UUID | B17 |

---

## §2 Coupled invariants

- **SW-thread-spawn × storage-broker ownership** — the `cache_conn: Arc<Mutex<SqliteConnection>>`
  handed across the spawn boundary must originate from a single `OriginStorageManager` living on the
  least-authority side. Intersection: *who constructs the OSM (App vs coordinator, §6.5) and what
  capability crosses the boundary* — App-owns hands the coordinator only the per-origin
  `cache_connection(&OriginKey::from_url(scope))` handle, never the whole manager.
- **promise-settle × IPC-arm type** — each awaiting `SwClientRequest` arm settles a specific rooted
  promise via a specific reply→`SwClientUpdate` mapping. Intersection: *arm → pending list → reply
  message → `SwClientUpdate` variant* — Register/Update → `pending_registration_promises` →
  `SwRegistered` → `Registered`; Unregister → `pending_unregister_promises` → `SwUnregistered` (NEW)
  → `Unregistered`; PostMessage → (no promise) → `ContentToSw::PostMessage` → (no reply). This is
  the §5 mapping table; the coupling is that a wrong list/variant pairing hangs or mis-settles the
  promise.
- **client-identity coherence × registration wire** — the shell-minted page client UUID (B17) must
  be the ONE id used everywhere. Intersection: *`SwFetchRequest.client_id` =
  `ContentToSw::PostMessage.client_id` = (future) coordinator `ClientState.id`* — all read one
  `ContentState.client_id` field (NEW) (§6.4); coherent-by-construction even though the coordinator
  registration path is currently dead code (§6.1).
- **bracket lifetime × settle-deliver** — `deliver_sw_client_update` fires JS (promise resolution +
  `statechange`/`controllerchange` listeners) so it is assume-bound and must run inside exactly one
  `with_bound` bracket with a trailing `drain_reactions`, never nested. Intersection: *settle-deliver
  is an engine-driving call identical in shape to the stage-2d-1 bracketed deliver helpers*
  (`lib.rs:373-424`), placed on the content-thread SW-reply handler, never inside another open
  bracket.

**ECS-native side-store check (CLAUDE.md "Side-store→component 判定ルール").** The two new pieces of
session state 2e introduces — the `OriginStorageManager` home (§6.5) and the NEW `ContentState.client_id`
field (§6.4) — are **browsing-context/session-level cross-cutting resources** (CLAUDE.md exception (b)
for the OSM = shared per-origin storage broker, exception (a)-class per-context identity for the page
client id), **not per-entity DOM state** → correctly modelled as a side-store / content-thread field,
**NOT an ECS component**. Naming-collision caveat: the VM-side `client_states: HashMap<ObjectId,
ClientSnapshot>` (`vm/mod.rs:2372`) is a *different, live* per-VM side-store — do **not** conflate it
with the coordinator's dead-code `client_states` (§6.1).

Convergence for this class = each open question in §6 moving from design-tension to a concrete
mechanism (which list / key / message / bracket); expect 2+ review passes.

---

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| SERVICE WORKERS §3.4.3 register(scriptURL, options) (`#navigator-service-worker-register`) | queue client `Register` → coordinator register | `update_via_cache` carried through to `SwRegistration` | `SwClientRequest::Register` consumer (`event_loop.rs:123`) + `SwCoordinator::register` | ✗ (marshal-scale wire) | yes — page-controlled scriptURL/scope, already VM-validated |
| SERVICE WORKERS §3.2.7 updateViaCache (`#service-worker-registration-updateviacache`) | carry token onto registration | imports / all / none | `SwRegister.update_via_cache` field | ✓ (imports/all/none) | yes |
| SERVICE WORKERS §3.2.8 update() (`#service-worker-registration-update`) | `Update` arm → settle via `SwRegistered` | minimal-settle vs full re-fetch+install = §6.3 OPEN | `SwClientRequest::Update` → coordinator Update handler → `SwRegistered` settle | ✗ (depth OPEN) | no — scope only |
| SERVICE WORKERS §3.2.9 unregister() (`#navigator-service-worker-unregister`) | `Unregister` arm → settle via `SwUnregistered` | removal-reply, no teardown ack (§6.6) | `SwClientRequest::Unregister` → coordinator unregister → `SwUnregistered` (NEW) | ✗ | no — scope only |
| SERVICE WORKERS §3.1.4 ServiceWorker.postMessage(message) (`#service-worker-postmessage`) | `PostMessage` arm → route to SW channel | fire-and-forget, no reply | `SwClientRequest::PostMessage` → `ContentToSw::PostMessage` | ✗ | yes — message data, structured-serialized |
| SERVICE WORKERS §4.6.3 FetchEvent.clientId (`#fetch-event-clientid`) | fill `client_id` on fetch request | shell-minted page UUID | `SwFetchRequest.client_id` shell-minted (`navigation.rs:141`) | ✗ | no |
| SERVICE WORKERS §4.3.2 matchAll(options) (`#clients-matchall`) | seed `initial_clients` at spawn | empty today (dead-code population path, §6.1) | `initial_clients` seed | ✗ | no |

**Breadth**: K=1 spec (service-workers), M=7 entries → single-PR OK per the split rule (K≥4/M≥20 =
split-recommended); the sub-commit split (§7) is intra-PR sequencing, not a multi-PR umbrella.

### §3.1 User-input touch audit

- **register scriptURL/scope** — page-controlled, but VM-pre-validated against the document base
  (`container.rs:130-149`) before it reaches the `SwClientRequest::Register` arm.
- **postMessage data** — page-controlled, structured-clone-serialized on the VM side before it
  crosses as `ContentToSw::PostMessage.data`.
- **updateViaCache token** — page-controlled enum (imports/all/none), carried verbatim onto the
  `SwRegistration`.
- **Adjacent pre-existing surface exposure change = none** — the coordinator's `validate_registration`
  security gate (`sw_coordinator.rs:159`) is unchanged; 2e adds no new trust boundary and relaxes no
  existing one.

---

## §4 Per-part design rulings

### Part 1 — SW thread swap (B11)

**Signatures (verified).** boa: `sw_thread_main(script_url, scope, channel, network_handle)` — 4
params (`elidex-js-boa/src/sw_thread.rs:33-38`). VM: `sw_thread_main(script_url, scope, channel,
network_handle, cache_conn: Arc<Mutex<SqliteConnection>>, initial_clients: Vec<ClientSnapshot>)` —
6 params (`elidex-js/src/vm/sw_thread.rs:92-99`); hard-derives `EngineMode::BrowserCompat`
(`sw_thread.rs:79-91,127`, F10 — deliberate, no embedder mode selection until
`#11-async-core-storage-cookiestore`). Call site to rewrite: `sw_coordinator.rs:186-193` (inside
`SwCoordinator::register`).

**Ruling — `cache_conn` (the DR-A shared Cache API connection).**
`OriginStorageManager::cache_connection(&OriginKey) -> Result<Arc<Mutex<SqliteConnection>>, _>`
(`elidex-storage-core/origin_manager.rs:186-191`) returns the `Arc<Mutex>` ready — no wrapping.
Construct **one** `OriginStorageManager::new(profile_dir)` (`origin_manager.rs:123`; it keys
connections internally by `(OriginKey, StorageType)`, `:113`, so one instance serves all origins),
and at spawn call `cache_connection(&OriginKey::from_url(scope))`. The `profile_dir` today is
`dirs_next_data_dir().join("elidex")`, computed locally in `App::init_browser_db`
(`app/mod.rs:343`) and **NOT retained**. 2e must thread it so the coordinator can construct the OSM.
`OriginKey::from_url` returns `Option` — a spawn whose scope has no tuple origin (should not happen
for a validated SW scope, which is HTTPS/localhost per `validate_registration`) fails the
`cache_connection` open; ruling: on `None`/`Err`, the coordinator **first replies
`SwRegistered{ success: false, error: <cache-open error> }`** so the register promise
**rejects/settles** (it must never be left permanently pending — the exact hung-promise failure mode
the Decision-on-record + §8 condemn), **then** skips the spawn (a SW with no cache is non-functional;
do not spawn a half-wired thread). Skip ≠ silence: the promise is always settled before the spawn is
abandoned.

**Ruling — `initial_clients` (seeds `clients.matchAll()`, SW §4.3 Clients / §4.1.1 clients).**
Marshal the coordinator's `ClientState` (`sw_coordinator.rs:52`, with LOCAL `ClientType`/`FrameType`/
`VisibilityState` enums at `:27/:36/:46`) → `elidex_api_sw::ClientSnapshot` (`types.rs:81`, with the
API-level enums at `types.rs:50/58/68`). A field-by-field enum-family conversion (both enum families
are structurally identical). Source rows = the SW origin/scope's entries from `client_states`. See
the **critical open question in §6.1** — `client_states` is populated by `register_client`
(`sw_coordinator.rs:413`) which has **zero callers** (`#[allow(dead_code)]` on the whole client
block, `:53/:410-437`); the marshalled vec is empty today, which IS boa parity but must be recorded
honestly.

### Part 2 — B7: 4-arm `drain_sw_client_requests` consumer

**The enum (verified, `elidex-api-sw/types.rs:264`):** `Register { script_url, scope,
update_via_cache }` / `Update { scope }` / `Unregister { scope }` / `PostMessage { scope, data }`.
The trait method already exists: `HostDriver::drain_sw_client_requests() -> Vec<SwClientRequest>`
(`engine.rs:268`); the VM impl drains its per-VM queue (`service_worker/mod.rs:195`). Consumer loop
to rewrite: `event_loop.rs:123-141`.

**Register arm.** The enum carries **pre-resolved** `script_url`/`scope` — the VM already resolved
+ validated them against the document base (`container.rs:130-149`). The shell therefore **DROPS**
its current URL-join / `default_scope` fallback (`event_loop.rs:126-133`) and re-parses the resolved
strings into `url::Url`. It **KEEPS** the `current_url()` read only for the two fields the enum does
not carry: `origin` (serialized) and `page_url` (the coordinator's `validate_registration` security
input, `sw_coordinator.rs:159`). New IPC field: `ContentToBrowser::SwRegister` (`ipc.rs:576`) gains
`update_via_cache: UpdateViaCache`; `SwCoordinator::register` (`sw_coordinator.rs:145`) takes it and
stores it on the `SwRegistration` (currently hardcodes `UpdateViaCache::default()` at `:177`).
Spec: SW §3.4.3 `register(scriptURL, options)` (`#navigator-service-worker-register`); §3.2.7
`updateViaCache` (`#service-worker-registration-updateviacache`).

**Update / Unregister arms (each AWAITS a promise).** New IPC: `ContentToBrowser::SwUpdate { scope }`
(NEW) and `ContentToBrowser::SwUnregister { scope }` (NEW), handled in `app/content_messages.rs` beside the
existing `SwRegister` arm (`:121-136`) → new `SwCoordinator` handlers.
- **Update** — the VM's `update()` promise is in `pending_registration_promises`
  (`registration.rs:177`) and settles via `SwClientUpdate::Registered` (same settle as register).
  So the coordinator's Update handler ends by replying `BrowserToContent::SwRegistered(..)`. Spec:
  SW §3.2.8 `update()` (`#service-worker-registration-update`); algorithm dfns *Update* (`#update`)
  / *Soft Update* (`#soft-update`). **⚠ see §6.3** — the existing `UpdateChecker`
  (`elidex-api-sw/update.rs`) only decides *whether* to update (`should_soft_update`/`record_check`/
  `scripts_differ`/`hash_script`), it does **not** perform the re-fetch+install; the depth of the
  Update handler is an open question.
- **Unregister** — reply `BrowserToContent::SwUnregistered { scope, success }` (NEW variant), mapped
  to `SwClientUpdate::Unregistered` which settles `pending_unregister_promises` (`deliver.rs:52,297`).
  The coordinator's existing `unregister(&mut self, scope)` (`sw_coordinator.rs:367`) already removes
  from `store` + drops the `handles` entry (which sends `Shutdown`); 2e wraps it to send the reply.
  Spec: SW §3.2.9 `unregister()` (`#navigator-service-worker-unregister`); algorithm dfn *Unregister*
  (`#unregister`).

**PostMessage arm (fire-and-forget).** New IPC `ContentToBrowser::SwPostMessage { scope, data }` (NEW) →
coordinator routes to the target SW handle's channel via the existing
`ContentToSw::PostMessage { data, origin, client_id }` (`types.rs:118-123`) — `handle.send(..)`
(`handle.rs:67`). No reply. `origin`/`client_id` on that message = the sender page's origin +
its client id (§6.4 coherence). Spec: SW §3.1.4 `ServiceWorker.postMessage(message)`
(`#service-worker-postmessage`) — the `data`-only overload this arm wires (the §3.1.5
`postMessage(message, options)` overload has a distinct anchor `#service-worker-postmessage-options`
and is not wired here).

### Part 3 — B17: client_id shell-owned

`content/navigation.rs:141` reads `.bridge().client_id()` (boa per-runtime UUIDv4) to fill
`SwFetchRequest.client_id` (feeds `FetchEvent.clientId`, SW §4.6.3 `#fetch-event-clientid`). The VM
has **no window-side client id** (grep confirms: the VM takes client ids as params on the SW side,
never mints a window-side one). Ruling (memo B17): the **shell mints** the page's client UUID at
pipeline construction — a `String` field on `ContentState` (or `PipelineResult`), set once via
`uuid::Uuid::new_v4()` (the `resulting_client_id` at `navigation.rs:149` already uses this crate),
read at `:141`. **Coherence ("ONE generator", §6.4):** the same id must be the id the coordinator
tracks as this page's `ClientState.id` so `clients.matchAll()`/`initial_clients` and
`FetchEvent.clientId` agree — but that path is presently un-wired (dead code), so the coherence is
today **vacuous**; §6.4 rules on how to structure it so it becomes coherent by construction when
client-registration lands, without 2e being forced to wire that path.

---

## §5 Shared infra — the settle-deliver path (currently a NO-OP)

Even Register's promise never settles under the VM today: `BrowserToContent::SwRegistered(_)` is a
**dropped no-op arm** at `event_loop.rs:575` (grouped with `SwControllerSet`/`SwStateChanged`,
`:576-577`). The browser side already SENDS the reply — `SwCoordinator::register` sends
`SwRegistered{ success, error }` on both the failure (`:161`) and success (`:206`) paths, and
`tick()` stages `SwClientBroadcast::StateChanged`/`ControllerSet` drained + fanned out as
`SwStateChanged`/`SwControllerSet` (`content_messages.rs:285-309`). The content thread just discards
them.

**Ruling — wire a bracketed settle helper.** On each SW reply message, the content thread calls a
new `PipelineResult::deliver_sw_client_update(SwClientUpdate)` (NEW) helper mirroring the stage-2d-1
bracketed helpers (`lib.rs:373-424` — build `ScriptContext` once → `with_bound { engine.<deliver>;
engine.drain_reactions(ctx) }`; the trait method is `engine.rs:229`). Message → `SwClientUpdate`
mapping:

| `BrowserToContent` | → `SwClientUpdate` | settles |
|---|---|---|
| `SwRegistered(SwRegisteredData)` | `Registered { scope, success, error, worker, update_via_cache }` | register + update promises |
| `SwUnregistered { scope, success }` (NEW) | `Unregistered { scope, success }` | unregister promises |
| `SwStateChanged { scope, state }` | `StateChanged { scope, state }` (see §6.2) | `.state`/onstatechange/onupdatefound |
| `SwControllerSet { scope }` | `ControllerSet { scope: Some(scope) }` (see §6.2) | controller + oncontrollerchange |

**⚠ Mapping gap (design point, not open — flag for review confirmation).** The current
`SwRegisteredData` (`ipc.rs:239`) carries only `{ scope, success, error: Option<String> }`, but
`SwClientUpdate::Registered` needs `worker: Option<SwWorkerSnapshot>` (to seed `.installing`/
`.waiting`/`.active` at resolve — `deliver.rs:92-112`) and `update_via_cache: UpdateViaCache` (seeds
`registration.updateViaCache`), and its `error` is a typed `Option<SwRegisterError>` (mapped 1:1 to a
`DOMException` via `map_sw_register_error`, `deliver.rs:82`) not a `String`. Ruling: **extend
`SwRegisteredData`** to carry `worker: Option<SwWorkerSnapshot>` + `update_via_cache: UpdateViaCache`
+ `error: Option<SwRegisterError>` (typed), and populate them in `SwCoordinator::register`/the Update
handler from the `SwRegistration` it just created (`sw_coordinator.rs:171-178` has scope, script_url,
state → build the `SwWorkerSnapshot { script_url, state }`, `types.rs:188`). Without this the
register promise resolves with an empty registration (`.active === null`), a behavior delta from a
correct implementation.

`SwControllerSet`/`SwStateChanged` scoping decision → §6.2.

---

## §6 Open design questions (for plan-review)

### §6.1 (CRITICAL) `client_states` population path — is it wired at all?

**Finding (investigated).** The coordinator's entire client-tracking block is dead code:
`ClientState` (`sw_coordinator.rs:52`), `register_client` (`:413`), `unregister_client` (`:418`),
`all_clients` (`:423`), `get_client` (`:428`), `has_foreground_client` (`:433`) are all under
`#[allow(dead_code)]` (`:53`), and grep across `crates/shell/elidex-shell/src` finds **no caller** of
`register_client`. Therefore `client_states` is **always empty**, and `clients.matchAll()` under both
boa and the VM seeds empty. Seeding empty `initial_clients` in 2e **is boa parity**. **Open
question:** is the page→coordinator client-registration wire in-scope for 2e, or is it a separate
pre-existing gap 2e should NOT absorb (justify-don't-absorb)? Recommendation (lean, not a decision):
**out of 2e** — it is a pre-existing hole under both engines, not a flip regression; carve a slot
(§9). Reviewers: confirm, or rule that the flip's SW correctness demands it now.

### §6.2 Do `SwControllerSet` / `SwStateChanged` feed `deliver_sw_client_update`, or defer?

These two are also no-op arms today (`event_loop.rs:576-577`), and the VM has the deliver machinery
(`SwClientUpdate::StateChanged`/`ControllerSet`, `deliver.rs:47-48`). Wiring them makes
`controllerchange` and `statechange`/`updatefound` fire on the window realm. **Open question:** are
they in 2e's settle-deliver group (natural — same helper, same bracket, machinery present), or a
separate fidelity slice? Lean: **in scope** — they ride the identical bracketed helper and the
coordinator already broadcasts them (`content_messages.rs:296-306`), so omitting them would leave the
broadcast dangling into the same dropped arm. Reviewers: confirm the grouping.

### §6.3 Update handler depth — full re-fetch+install, or minimal settle?

The `UpdateChecker` (`elidex-api-sw/update.rs`) decides *whether* (`should_soft_update`,
`scripts_differ`, `hash_script`), not *how* — it does not re-fetch the script or spawn a replacement
worker. A spec-faithful *Update* (`#update`) re-fetches the script, byte-compares
(`scripts_differ`), and on change installs a new worker (new lifecycle). **Open question:** does 2e
implement the full re-fetch+install Update, or a minimal form (re-validate + reply `SwRegistered`
resolving the promise against the existing registration, deferring genuine script-replacement)? This
governs whether the Update arm is marshal-scale (minimal) or algorithm-scale (full). Do NOT
pre-decide — reviewers weigh it against the umbrella's "Ideal over pragmatic" (which argues full) vs.
the flip's equivalence-oracle scope (which argues the flip should not grow new algorithms). Note:
boa had **no** update path at all, so any settling form is ≥ boa.

### §6.4 client-id coherence structuring (B17 × §6.1)

Given §6.1 (client registration un-wired), the "ONE generator" coherence is vacuous now. **Open
question:** where does the minted client UUID live so it is coherent-by-construction *when*
registration lands — on `ContentState` (content-thread-owned, the natural home for `client_id` fed to
both `SwFetchRequest` and a future `register_client`), passed into the coordinator via the same
`SwRegister`/client-registration IPC? Confirm the field home and that the `PostMessage` arm's
`ContentToSw::PostMessage.client_id` (§2 Part 2) reads this same field.

### §6.5 OSM ownership — coordinator-owns vs App-owns-and-passes (storage-broker boundary)

The `OriginStorageManager` must be constructed once and live somewhere. **Open question:** does the
`SwCoordinator` own the OSM (constructed in `SwCoordinator::new`/`with_persistence`, needing
`profile_dir` threaded into the constructor — 3 call sites: `app/mod.rs:322,453,493`), or does `App`
own it (beside `browser_db`, `app/mod.rs:356`) and pass `cache_conn` per-spawn into `register`?
CLAUDE.md "Security by structure" wants renderer/coordinator storage access through a broker
boundary; both keep storage off the content thread, but App-owns keeps the OSM adjacent to the other
browser-owned storage state (`browser_db`, cookie jar) and hands the coordinator only the specific
`Arc<Mutex<SqliteConnection>>` it needs (narrower capability). Lean: **App-owns, passes cache_conn**
(least-authority). Reviewers rule.

### §6.6 D2 connection-close coupling — does unregister need an ack round-trip?

Memo §4.3.2 H10 flags that IDB versionchange delivery may force the `IdbConnectionsClosed`-style ack
sequencing (D2's fire condition). **Open question:** does SW `unregister()` / worker shutdown need a
similar "worker fully torn down" ack before the promise settles, or is the synchronous
`handle` drop (`sw_coordinator.rs:369` sends `Shutdown`) + immediate `SwUnregistered` reply
sufficient? Note: the VM's `unregister()` promise resolves per SW §3.2.9 once the registration is
*removed* (not after teardown completes), so an immediate reply is spec-aligned — but the SW thread's
in-flight `respondWith`/`waitUntil` pumps outlive the `Shutdown` send. Lean: **no ack needed** (spec
settles on removal, not teardown); reviewers confirm no D2 fire.

---

## §7 Sub-commit split + acceptance

The impl follows this split (dependencies noted; §6.4/§6.1 findings may reorder 2e-b/2e-c).

- **2e-a — SW-thread spawn swap (B11).** OSM construction + `profile_dir` threading (§6.5 ownership)
  + `ClientState`→`ClientSnapshot` marshalling (empty today, §6.1) + `sw_coordinator.rs:192` boa→VM
  6-param spawn. No window-side behavior change (boa `drain_sw_register_requests` stays live until
  2e-b). **Acceptance:** VM SW tests stay green (`vm/tests/tests_service_worker.rs`,
  `tests_service_worker_client.rs`, `tests_cache.rs`); `elidex-api-sw` crate tests green; a shell
  test that a registered SW spawns the VM thread + `clients.matchAll()` returns `[]` (parity).
- **2e-b — B7 4-arm consumer + IPC + coordinator handlers + settle-deliver (§5).**
  `event_loop.rs:123` → `drain_sw_client_requests`; new `ContentToBrowser::SwUpdate`/`SwUnregister`/
  `SwPostMessage` + `SwRegister.update_via_cache`; new `BrowserToContent::SwUnregistered`; extend
  `SwRegisteredData` (§5 mapping gap); bracketed `PipelineResult::deliver_sw_client_update`; wire the
  four dropped no-op arms (`event_loop.rs:575-577`). **Acceptance:** shell integration tests that
  `register()`/`update()`/`unregister()` promises **settle** (the load-bearing new coverage — none
  exists today, §8) and `ServiceWorker.postMessage` reaches the SW; `cargo check -p elidex-shell`
  drops the `event_loop.rs:123` error.
- **2e-c — B17 client_id shell-owned (§4 Part 3, §6.4).** Mint on `ContentState`; read at
  `navigation.rs:141`. **Acceptance:** `cargo check -p elidex-shell` drops the `navigation.rs:141`
  error; SW navigation-intercept test carries the shell-minted `FetchEvent.clientId`.

**Whole-stage gate:** `cargo check -p elidex-shell` clean of the two SW errors; no test imports
`elidex_js_boa` for SW; `mise run test -p elidex-shell -p elidex-js -p elidex-api-sw` green.
Dependency: 2e-b's settle-deliver and 2e-c's client-id are independent of 2e-a's spawn; 2e-b's
Register arm depends on 2e-a's coordinator OSM only if the OSM lives on the coordinator (§6.5).
Sequencing pin: 2e-b's **PostMessage arm** sends `ContentToSw::PostMessage.client_id`, which reads
the NEW `ContentState.client_id` field MINTED in 2e-c — so **2e-c lands before 2e-b's PostMessage
arm** (otherwise the PostMessage `client_id` has no populated source). The write-path is wired by
2e-c; the rest of 2e-b (Register/Update/Unregister settle-deliver) has no such ordering constraint.

### §7.1 Why one atomic PR, not multi-PR

The S5-6b flip is a **single compiler-driven big-bang PR**: the crate does not compile until the full
boa→VM cutover AND the boa-crate deletion land together (flip memo
`2026-07-s5-6-flip-boa-deletion.md` Q1 charter — "flip+deletion remain atomic within S5-6b",
`:26-30`). No sub-slice compiles standalone, so a multi-PR *merge-to-main* split is architecturally
impossible. The CLAUDE.md edge-dense rule's **intent** — avoid a giant review tail on ≥3
intersecting-invariant work — is satisfied structurally, not by fragmenting the merge: (a) per-stage
`/elidex-plan-review` for the edge-dense stages (this 2e review), (b) bounded, individually-reviewable
WIP **sub-commits** (2a…2f on branch `s5-6b-flip`; git log shows stage1…stage2d-3 already landed as
sub-commits on this one branch), and (c) the terminal whole-flip `/external-converge` as the atomic
PR's adversarial design gate. The §3.2 K/M spec-breadth metric is **not** the split authority here —
it measures spec surface, not invariant-axis density; the flip's compiler-driven atomicity is what
forecloses the multi-PR split. Affirm explicitly: **2e is a sub-commit slice of the atomic S5-6b PR,
not an independent merge-to-main.**

---

## §8 Parity note — this is beyond-boa-parity, and justified

boa's window-side SW back-channel was **register-only**: its bridge exposed `queue_sw_register` (the
sole `drain_sw_register_requests` source, `event_loop.rs:123`) — no `update()`, no `unregister()`, no
`ServiceWorker.postMessage`. The VM surface added all four arms (`SwClientRequest`, `types.rs:264`).
Full-wiring them is **beyond** boa parity, but it is the correct call (§ decision-on-record): the VM
already queues + awaits all four, so a settle-stub would hang three of them — a regression *created*
by the flip. The full wire spends only shell IPC plumbing (marshal-scale) to make the
already-complete VM surface reachable. The one genuinely-new-algorithm risk is the Update arm's
re-fetch depth (§6.3), explicitly surfaced for review rather than silently absorbed.

---

## §9 Carve list

- **`#11-sw-client-registration-wire`** (NEW, if §6.1 rules client-registration out of 2e) — wire the
  page→coordinator `register_client` path so `clients.matchAll()` returns real clients; the dead-code
  block (`sw_coordinator.rs:410-437`) becomes live. Pre-existing gap under both engines, not a flip
  regression. **Re-evaluate** at S5-6b landing / when SW multi-client support lands (whichever first).
  **Register** in the defer-slots ledger ([[project_open-defer-slots]]) at S5-6b landing-memo time.
- **`#11-sw-update-full-algorithm`** (NEW, if §6.3 rules the minimal-settle form) — full SW *Update*
  (`#update`): re-fetch → byte-compare → install replacement worker + new lifecycle. **Re-evaluate**
  at S5-6b landing / when SW update-on-navigation fidelity is next touched (whichever first).
  **Register** in the defer-slots ledger ([[project_open-defer-slots]]) at S5-6b landing-memo time.
- **Existing, unaffected:** quota wiring (`sw_coordinator.rs:386` `TODO(M4-8.5)` — the OSM this stage
  constructs is the state that TODO waited for, but connecting `navigator.storage.estimate()` to the
  real `QuotaManager` stays M4-8.5-owned, umbrella §1.4); SW navigation-interception response body
  (`navigation.rs:171` `TODO` — the interception protocol is untouched, M4-10 family);
  `IdbConnectionsClosed` D2 coupling (§6.6, memo §4.3.2 H10).
