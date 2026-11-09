# VM stack-safety guard — P0b-VM-2 plan (IndexedDB, service-worker messages, WebCrypto consumers)

Status: **plan (pre-implementation)**, per-PR slice of the [P0 umbrella](2026-11-vm-stack-safety-guard-p0.md)
(PM 509; split from the former P0b-VM by PM 513 (a), reason in
[P0b-VM-1](2026-11-vm-stack-safety-guard-p0b-vm1.md)'s header); `/elidex-plan-review` required before code.
Owner: VM P4 (PM 509 item 2). Base: `ce603c74`; every code anchor below is at that commit. Requires
[P0a-1](2026-11-vm-stack-safety-guard-p0a1.md) and [P0a-2](2026-11-vm-stack-safety-guard-p0a2.md) (the slice starts from
P0a-2, whose prerequisite is P0a-1; P0 umbrella §1). Sibling P0b-VM-1 is independent (disjoint files, disjoint
`# pending` rows). Edges and schedule: P0 umbrella §1.

The contract C-1…C-5 is the P0 umbrella §4 (canonical); each fix below applies it at one site with
`is_resource_limit()` (P0a-1 §4.1) as the only test. This slice is a base case (c) under the P0 umbrella.

## §0. Charter

P0b-VM-2 fixes the definitive consumers D-8, D-9 and D-12 of P0a-2 §7.3. D-9's fix also makes
`startMessages()`'s flush asynchronous, as the Service Workers spec has it (PM 509 item 6), through one
delivery queue `PendingTask::SwMessage` (NEW); PM 513 (c) records that queue on
`#11-sw-client-postmessage-inbound`, whose inbound work builds on it.

## §1. Scope

**In.** `crates/script/elidex-js/src/vm/host/indexeddb/value.rs` and its callers in `indexeddb/cursor.rs`,
`index.rs`, `object_store.rs`; `vm/host/service_worker/deliver.rs`, `vm/host/pending_tasks.rs` (one task
variant), `vm/gc/roots.rs` (its exhaustive arm); `vm/host/subtle_crypto/ops.rs`; the census allowlist rows
of §6; cells in `crates/script/elidex-js/src/vm/tests/tests_resource_limit_host.rs` (NEW).

**Out.** Every non-resource failure at these sites keeps its present behaviour, owned by the slots named in
§5; D-6, D-7, D-10, D-11 (P0b-VM-1).

## §2. Coupled invariants

Elements: **IDB** (D-8), **SW** (D-9 and the task queue), **WC** (WebCrypto D-12), **ID** (P0a-1's predicate
and injection points, consumed here), **PEND** (this slice's rows of `# pending`).

| Pair | Intersection | Pinned by | Basis |
|---|---|---|---|
| IDB × request | each of `json_to_js`'s seven callers turns a C-1 error into `DeferredOutcome::Error` with the materialised `RangeError`; the request fails on its own channel | `rl_idb_get_resource_limit_errors_request`; M-IDB | cell |
| IDB × non-resource | a non-resource parse failure keeps today's `Undefined` (pre-existing, `#11-idb-structured-clone-storage`), so `json_to_js` itself stays a member (P) | §6 | check |
| SW × top-only | after the flush is queued, `fire_message` runs only from the task drain; its `unwrap_or(Undefined)` is then an N row (top-only), which rule 5 checks | `rl_sw_start_messages_is_async`; M-SW | cell + check |
| SW × order | the flush and `deliver_message`'s enabled branch both queue the same task kind, so buffered messages stay ahead of a later one (FIFO on one queue) | `rl_sw_message_order_preserved`; M-SW2 | cell |
| WC × spec fallback | a C-1 error from the primary normalisation returns without running step 3's fallback | `rl_wrap_key_resource_limit_skips_fallback`; M-WC | cell |
| IDB × SW, IDB × WC, SW × WC | disjoint files and carriers; no shared state | — | structural |
| ID × IDB, ID × WC | every fix tests only `is_resource_limit()`, and every cell uses only P0a-1's injection points | P0a-1 M-ID kills the IDB and WebCrypto cells | cell |
| ID × SW | the SW change is a scheduling change (no predicate); its cells do not depend on the injection | M-SW, M-SW2 | cell |
| PEND × ID | this PR deletes exactly its 4 pending rows (D-12 is 2 rows) and adds no pending row (P0a-2 §5 rule 4 ceiling) | P0a-2 control `census_pending_row_outside_ceiling` | check |

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| IndexedDB-3 §6.2 Object store retrieval operations | *retrieve a value from an object store* step 4 (`! StructuredDeserialize`) | stated deviation at a `!`: the request fails with the `RangeError` | `json_to_js` (`indexeddb/value.rs:463`) and its seven callers | ✓ | yes |
| Service Workers §3.4.6 startMessages() | enable the client message queue; §3.4: an enabled queue is a task source | buffered messages are delivered as tasks, not synchronously | `enable_sw_messages`, `deliver_message`, `fire_message` (`deliver.rs:287`, `:263`, `:359`) | ✓ | yes |
| Web Cryptography API §14.3.11 The wrapKey method | steps 2–4 (normalize for "wrapKey"; on error normalize for "encrypt"; on error reject) | step 3 is not taken; rejects with the `RangeError` | `normalize_wrap_with_fallback` (`subtle_crypto/ops.rs:66`, caller `:655`) | ✓ | yes |
| Web Cryptography API §14.3.12 The unwrapKey method | steps 2–4 (same, "unwrapKey" / "decrypt") | same | same helper (`ops.rs:720` caller) | ✓ | yes |

**Breadth**: K=3 specs (IndexedDB-3, Service Workers, Web Cryptography API), M=4 entries
(`python3 .claude/skills/elidex-plan-review/preflight.py` on this memo; `IndexedDB-3` and `Service Workers`
are labels preflight does not map, so their two rows were looked up by hand, §10). Below the split
thresholds.

### §3.1 User-input touch audit

Every touched consumer is reachable from script through a native whose input is script-controlled: the
caller's stack depth and the data (the stored IndexedDB value, the algorithm dictionary). D-9 is
caller-depth reachable: `native_container_start_messages` → `enable_sw_messages` → `fire_message`
(`container.rs:366`, `deliver.rs:287-297`).

## §4. Rows fixed

| Id | Site | Carrier | What the error becomes today | Class | Governing step |
|---|---|---|---|---|---|
| D-8 | `vm/host/indexeddb/value.rs:463` `json_to_js` | `parse_json_str` | `unwrap_or(Undefined)`: `get` / `getAll` / index reads / cursor value succeed with `undefined` | D-sub | IndexedDB-3 §6.2 step 4 |
| D-9 | `vm/host/service_worker/deliver.rs:359` `fire_message` (reached synchronously from `startMessages()`, `deliver.rs:287-297`) | `parse_json_str` | `unwrap_or(Undefined)`: the message is consumed with `data` undefined | D-sub | Service Workers §3.4.6, §3.4 |
| D-12 | `vm/host/subtle_crypto/ops.rs:66` (+ `:68`, same site) | `marshal_normalize` | the step-3 fallback runs; may answer NotSupportedError or a different normalisation | D-cls | Web Cryptography API §14.3.11 / §14.3.12 steps 2–4 |

## §5. Fixes

### §5.1 D-8: IndexedDB reads

`json_to_js` returns `Result<JsValue, VmError>`: a C-1 error propagates; a non-resource parse failure keeps
`Ok(Undefined)` (pre-existing, `#11-idb-structured-clone-storage`). Its seven callers
(`git grep -n 'json_to_js(' -- crates/script/elidex-js/src` minus the definition: `indexeddb/cursor.rs:145`,
`index.rs` 246 / 262 / 333, `object_store.rs` 365 / 373 / 477; verified 2026-11-08) turn a C-1 error into
`DeferredOutcome::Error` with the materialised `RangeError`, so the request fails on its own channel
(IndexedDB-3 §6.2 step 4 is a `!`: a stated resource-limit deviation, like step 3's NotReadableError
result).

### §5.2 D-9: service-worker messages

`PendingTask::SwMessage { data, source_scope }` (NEW, `pending_tasks.rs`; its `gc/roots.rs` arm holds no GC
reference) is the one delivery path of a client message once the queue is enabled: `enable_sw_messages`
queues each buffered message as that task instead of calling `fire_message`, and `deliver_message`'s
enabled branch queues it too, so the order is FIFO on one queue (Service Workers §3.4: an enabled client
message queue is a task source). `fire_message` then runs only from the task drain (top-only, N); its
`unwrap_or(Undefined)` for other deserialise failures is the pre-existing substitution of the SW message
path (spec: `messageerror`), recorded on `#11-sw-client-postmessage-inbound` (P0 umbrella §7 L-10).

### §5.3 D-12

`normalize_wrap_with_fallback` returns a C-1 error from the primary normalisation without running the
fallback (step 3 is for errors the spec defines).

## §6. Census rows after the fix

Deleted from `# pending`: D-8, D-9, D-12 (2) — 4 rows. Added as admissible (keys as in P0a-2 §7.3):

Codes are P0a-2 §5 rule 12's closed table; a row with no fitting code is not given an invented one.

| Site | Class · code | Reason |
|---|---|---|
| `json_to_js` → `parse_json_str` | P · — (no code of rule 12 fits; open) | resource-limit arm propagates; other failures `Undefined` (`#11-idb-structured-clone-storage`) |
| the seven `json_to_js` callers | R · `channel` | request error (`DeferredOutcome::Error`); channel function: the `DeferredOutcome::Error` construction |
| `fire_message` → `parse_json_str` | N · `top-only` | top-only (task drain), rule 5 |
| `normalize_wrap_with_fallback` → `marshal_normalize` (2) | P · — (no code of rule 12 fits; open) | resource-limit arm propagates; fallback only for other errors |

## §7. Cells (written first)

New file `crates/script/elidex-js/src/vm/tests/tests_resource_limit_host.rs` (NEW); P0a-1 §4.4's injection,
`n` from `resource_limit::count` on a dry run.

| Cell | Body | Expect |
|---|---|---|
| `rl_idb_get_resource_limit_errors_request` | a stored record; inject `JsonParse(1)`; `store.get(k)`, `getAll`, an index `get`, a cursor | each request errors with `RangeError` (no success with `undefined`); un-injected reads return the value |
| `rl_sw_start_messages_is_async` | buffered client messages; `navigator.serviceWorker.startMessages()` | no `message` event during the call; events fire after the task drain |
| `rl_sw_message_order_preserved` | messages A, B buffered; `startMessages()`; message C delivered before the drain | events in the order A, B, C |
| `rl_wrap_key_resource_limit_skips_fallback` | `wrapKey` / `unwrapKey` with `{ get name() { k++; return 'AES-KW' } }` and `Entry(Call, 1)` | rejects `RangeError`; `k === 1` |

## §8. Mutants (run on the final head, debug)

| Mutant | Change | Dies |
|---|---|---|
| M-IDB | `unwrap_or(Undefined)` back | `rl_idb_get_resource_limit_errors_request` |
| M-SW | synchronous flush back | `rl_sw_start_messages_is_async` |
| M-SW2 | `deliver_message`'s enabled branch fires synchronously | `rl_sw_message_order_preserved` |
| M-WC | fallback runs on a resource limit | `rl_wrap_key_resource_limit_skips_fallback` |
| M-ID (P0a-1) | `is_resource_limit` always false | the IDB and WebCrypto cells |

## §9. Landing obligations

The ledger rows this slice triggers are rows of the P0 program's landing list, P0 umbrella §7 (PM 513 (d)):
**L-9** (`#11-idb-structured-clone-storage`), **L-10** (`#11-sw-client-postmessage-inbound`). Slice-local:

- **L-v2-1. Counts**: the PR body records the census rc 0 and the class counts at its head, and the
  preflight rc / K / M of this memo (P0 umbrella §7 L-13).

## §10. Unverified facts (each decided at implementation, before its cell is written)

1. Whether the SW cells run in the in-process VM harness or need their own module.
2. That the SW task drain and `deliver_message`'s caller (`deliver.rs:50`) are not reachable from a native
   (P0a-2 §5 rule 5 decides; if reachable, the N row is wrong and this is a design change back to review).
3. Every exhaustive `match` on `PendingTask` besides `gc/roots.rs:557` and `pending_tasks.rs:287`.

webref lookups behind §3 (2026-11-08): `dfn IndexedDB-3 'retrieve a value from an object store'` → §6.2;
`dfn service-workers 'startMessages()'` → §3.4.6, `body service-workers dfn-client-message-queue`;
`dfn webcrypto 'wrapKey()'` → §14.3.11, `dfn webcrypto-2 'unwrapKey()'` → §14.3.12.
