# VM stack-safety guard — P0b-VM-2 plan (IndexedDB, service-worker messages, WebCrypto consumers, the UA-fire drop class)

Status: **plan (pre-implementation)**, per-PR slice of the [P0 umbrella](2026-11-vm-stack-safety-guard-p0.md)
(PM 509; split from the former P0b-VM by PM 513 (a), reason in
[P0b-VM-1](2026-11-vm-stack-safety-guard-p0b-vm1.md)'s header); `/elidex-plan-review` required before code.
Owner: VM P4 (PM 509 item 2). Base: `ce603c74`; every code anchor below is at that commit. Requires
[P0a-1](2026-11-vm-stack-safety-guard-p0a1.md), [P0a-2a](2026-11-vm-stack-safety-guard-p0a2a.md) (the check) and
[P0a-2b](2026-11-vm-stack-safety-guard-p0a2b.md) (the allowlist and `# pending`); the slice starts from
P0a-2b (P0 umbrella §1). Sibling P0b-VM-1; independence, the file overlap (two UA-fire drop class members
in P0b-VM-1's files, fixed there, PM 615), edges and schedule: P0 umbrella §1.

The contract C-1…C-5 is the P0 umbrella §4 (canonical); each fix of D-8, D-9 and D-12 applies it at one
site with `is_resource_limit()` (P0a-1 §4.1) as the only test, and the drop-class fixes apply the C-3 class
shape, which tests no predicate. This slice is a base case (c) under the P0 umbrella.

## §0. Charter

P0b-VM-2 fixes the definitive consumers D-8, D-9 and D-12 of P0a-2b §7.3, and the UA-fire drop class
except its two members in P0b-VM-1's files (§1; PM 608 item 3, PM 609 item 2, PM 615). D-9's fix also makes
`startMessages()`'s flush asynchronous, as the Service Workers spec has it (PM 509 item 6), through one
delivery queue `PendingTask::SwMessage` (NEW); PM 513 (c) records that queue on
`#11-sw-client-postmessage-inbound`, whose inbound work builds on it.

**Why the drop class is here (PM 608).** The defect is specific to the resource limit (at the base no `Err`
flows on these dispatch paths; P0's error is what would make one flow), so it belongs inside the P0
program. Every member has the shape of `deliver.rs:341`, a file already in this slice, so one fix shape
covers all of them; that shape is defined once, in the P0 umbrella §4 C-3 ("The UA-fire drop class"), and
each slice implements it at the members in its own files (PM 615). It is a different problem from L3's dispatch path (D-5, the listener call's own error),
and keeping the class in one lane keeps one decision surface.

**Priority.** First: `indexeddb/dispatch.rs:194`. It turns the dispatch's error into `threw: true` and
reports nothing, the silent-drop shape that PM 590 made mandatory to fix (a converted error with no
report); the transaction then aborts with no trace of the cause. Then the rest of the class, then D-8,
D-9, D-12.

**If the slice stops fitting one PR (PM 609 item 2).** When this slice touches more than 15 files, or is
not expected to finish in one review round, the driver proposes to the PM a cut of P0b-VM-3 by class (the
drop class on one side, D-8 / D-9 / D-12 on the other). The count is taken when the classification
(P0a-2b §6) has fixed the class's members. The two members in P0b-VM-1's files (PM 615) are not this
slice's and do not count toward the 15 files.

## §1. Scope

**In.** `crates/script/elidex-js/src/vm/host/indexeddb/value.rs` and its callers in `indexeddb/cursor.rs`,
`index.rs`, `object_store.rs`; `vm/host/service_worker/deliver.rs`, `vm/host/pending_tasks.rs` (one task
variant), `vm/gc/roots.rs` (its exhaustive arm); `vm/host/subtle_crypto/ops.rs`; the files of the UA-fire
drop class below; the census allowlist rows of §6; cells in
`crates/script/elidex-js/src/vm/tests/tests_resource_limit_host.rs` (NEW).

**The UA-fire drop class.** A UA-fired event's dispatch whose `Result` is dropped (`let _ =`, an unused
binding) or converted without a report. Its members are reached by P0's error only under PM 603's reach
(P0a-2a §5 rule 1); at the base `dispatch_vm_event` returns only `Ok`, so nothing flows yet. The fix is
the class shape of the P0 umbrella §4 C-3, implemented at each member below. Members of this slice (paths
under `crates/script/elidex-js/src/`):

- **The seven named drops (PM 603; lines at `ce603c74`):** `vm/host/service_worker/deliver.rs:341`
  (`fire_simple`); `vm/host/event_source_dispatch.rs` 82 (`dispatch_sse_connected`) / 187
  (`fire_sse_error`); `vm/host/websocket_dispatch.rs` 79 (`dispatch_ws_connected`) / 114
  (`fire_vm_close_event`) / 177 (`fire_vm_message_event`) / 212 (`dispatch_ws_error`).
- **The sample's members (P0a-1 Rev 3c re-review, Axis 3; lines at `01d2f535`):**
  `vm/host/indexeddb/dispatch.rs:194` (`dispatch_vm_event`'s `Err(_)` becomes `threw: true`, nothing is
  reported: first, above); `vm/host/worker.rs:278`; `vm/sw_thread.rs:376`; `vm/host/media_query.rs:539` (inside a drain);
  `vm/host/visual_viewport.rs:350`; `vm/host/dom_selection_proto.rs:740`; `vm/host/html_slot_proto.rs:427`;
  `vm/host/file_reader.rs:660`; `vm/host/pending_tasks.rs:461`. `vm/sw_thread.rs:339` is probably an N
  `thread-entry` row, not a member; the classification pass confirms it. The sample's two other members,
  `vm/host/history_events.rs:187` and `vm/host/worker_scope.rs:333`, are P0b-VM-1's (PM 615; P0b-VM-1
  §5.3) and are listed there only.
- **Every further member the classification pass finds** (P0a-2b §6), added to this list before this
  memo's review.

**Out.** Every non-resource failure at these sites keeps its present behaviour, owned by the slots named in
§5; D-6, D-7, D-10, D-11 (P0b-VM-1).

## §2. Coupled invariants

Elements: **IDB** (D-8), **SW** (D-9 and the task queue), **WC** (WebCrypto D-12), **DROP** (the UA-fire drop
class), **ID** (P0a-1's predicate
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
| PEND × ID | this PR deletes exactly its 4 pending rows (D-12 is 2 rows) and adds no pending row (P0a-2a §5 rule 4 ceiling) | P0a-2a control `census_pending_row_outside_ceiling` | check |
| DROP × report | every member of this slice applies the P0 umbrella §4 C-3 class shape (for `indexeddb/dispatch.rs:194`, the mapping-site clause: report first, keep `threw: true`) | `rl_ua_fire_drop_reports` (per member file); M-DROP | cell |
| DROP × census | the class's rows change from `drain-consumed` to R `channel` (the report); a member added later by the census is red until it gets the same fix | P0a-2a control `census_reach_dispatch_drops` | check |

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| IndexedDB-3 §6.2 Object store retrieval operations | *retrieve a value from an object store* step 4 (`! StructuredDeserialize`) | stated deviation at a `!`: the request fails with the `RangeError` | `json_to_js` (`indexeddb/value.rs:463`) and its seven callers | ✓ | yes |
| Service Workers §3.4.6 startMessages() | enable the client message queue; §3.4: an enabled queue is a task source | buffered messages are delivered as tasks, not synchronously | `enable_sw_messages`, `deliver_message`, `fire_message` (`deliver.rs:287`, `:263`, `:359`) | ✓ | yes |
| Web Cryptography API §14.3.11 The wrapKey method | steps 2–4 (normalize for "wrapKey"; on error normalize for "encrypt"; on error reject) | step 3 is not taken; rejects with the `RangeError` | `normalize_wrap_with_fallback` (`subtle_crypto/ops.rs:66`, caller `:655`) | ✓ | yes |
| Web Cryptography API §14.3.12 The unwrapKey method | steps 2–4 (same, "unwrapKey" / "decrypt") | same | same helper (`ops.rs:720` caller) | ✓ | yes |
| WHATWG HTML §8.1.4.6 Runtime script errors | *report an exception* | a dispatch `Err` of a UA-fired event is reported, not dropped (C-3) | the UA-fire drop class (§1) | ✓ | yes |

**Breadth**: K=4 specs (IndexedDB-3, Service Workers, Web Cryptography API, WHATWG HTML), M=5 entries
(verified 2026-11-10 by `python3 .claude/skills/elidex-plan-review/preflight.py` on this memo; `IndexedDB-3` and `Service Workers`
are labels preflight does not map, so their two rows were looked up by hand, §10). Preflight reports K=4
SPLIT-RECOMMENDED, as it has since Rev 3d; the split is decided in this slice's own plan-review, together
with the §0 condition "If the slice stops fitting one PR (PM 609 item 2)".

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

Deleted from `# pending`: D-8, D-9, D-12 (2) — 4 rows. Added as admissible (keys as in P0a-2b §7.3):

Codes are P0a-2a §5 rule 12's closed table; a row with no fitting code is not given an invented one.

| Site | Class · code | Reason |
|---|---|---|
| `json_to_js` → `parse_json_str` | P · — (no code of rule 12 fits; open) | resource-limit arm propagates; other failures `Undefined` (`#11-idb-structured-clone-storage`) |
| the seven `json_to_js` callers | R · `channel` | request error (`DeferredOutcome::Error`); channel function: the `DeferredOutcome::Error` construction |
| `fire_message` → `parse_json_str` | N · `top-only` | top-only (task drain), rule 5 |
| `normalize_wrap_with_fallback` → `marshal_normalize` (2) | P · — (no code of rule 12 fits; open) | resource-limit arm propagates; fallback only for other errors |
| each UA-fire drop class member (§1) | R · `channel` (was `drain-consumed`) | the dispatch `Err` is reported; channel function: the report stand-in (`report_*`) |

## §7. Cells (written first)

New file `crates/script/elidex-js/src/vm/tests/tests_resource_limit_host.rs` (NEW); P0a-1 §4.4's injection,
`n` from `resource_limit::count` on a dry run.

| Cell | Body | Expect |
|---|---|---|
| `rl_idb_get_resource_limit_errors_request` | a stored record; inject `JsonParse(1)`; `store.get(k)`, `getAll`, an index `get`, a cursor | each request errors with `RangeError` (no success with `undefined`); un-injected reads return the value |
| `rl_sw_start_messages_is_async` | buffered client messages; `navigator.serviceWorker.startMessages()` | no `message` event during the call; events fire after the task drain |
| `rl_sw_message_order_preserved` | messages A, B buffered; `startMessages()`; message C delivered before the drain | events in the order A, B, C |
| `rl_wrap_key_resource_limit_skips_fallback` | `wrapKey` / `unwrapKey` with `{ get name() { k++; return 'AES-KW' } }` and `Entry(Call, 1)` | rejects `RangeError`; `k === 1` |
| `rl_ua_fire_drop_reports` (one per member file) | a listener on the UA-fired event; `InjectAt::SimpleEventDispatch(1)` (P0a-1 §4.4) | the `RangeError` is reported once; the UA algorithm continues as on an ordinary listener throw; for `indexeddb/dispatch.rs:194`, the report precedes the transaction's abort |

## §8. Mutants (run on the final head, debug)

| Mutant | Change | Dies |
|---|---|---|
| M-IDB | `unwrap_or(Undefined)` back | `rl_idb_get_resource_limit_errors_request` |
| M-SW | synchronous flush back | `rl_sw_start_messages_is_async` |
| M-SW2 | `deliver_message`'s enabled branch fires synchronously | `rl_sw_message_order_preserved` |
| M-WC | fallback runs on a resource limit | `rl_wrap_key_resource_limit_skips_fallback` |
| M-DROP | the report removed from one member (each member in turn) | that member's `rl_ua_fire_drop_reports` |
| M-ID (P0a-1) | `is_resource_limit` always false | the IDB and WebCrypto cells |

## §9. Landing obligations

The ledger rows this slice triggers are rows of the P0 program's landing list, P0 umbrella §7 (PM 513 (d)):
**L-9** (`#11-idb-structured-clone-storage`), **L-10** (`#11-sw-client-postmessage-inbound`). Slice-local:

- **L-v2-1. Counts**: the PR body records the census rc 0 and the class counts at its head, and the
  preflight rc / K / M of this memo (P0 umbrella §7 L-13).

## §10. Unverified facts (each decided at implementation, before its cell is written)

1. Whether the SW cells run in the in-process VM harness or need their own module.
2. That the SW task drain and `deliver_message`'s caller (`deliver.rs:50`) are not reachable from a native
   (P0a-2a §5 rule 5 decides; if reachable, the N row is wrong and this is a design change back to review).
3. Every exhaustive `match` on `PendingTask` besides `gc/roots.rs:557` and `pending_tasks.rs:287`.
4. **Closed, PM 615: file overlap with P0b-VM-1.** The two members in P0b-VM-1's files
   (`history_events.rs:187`, `worker_scope.rs:333`) are fixed by P0b-VM-1 (P0b-VM-1 §5.3), with the shape
   of the P0 umbrella §4 C-3; no landing-order edge (P0 umbrella §1). The L4 notice for
   `history_events.rs` is P0b-VM-1's (P0 umbrella §7 L-11).
5. Whether `InjectAt::SimpleEventDispatch(n)` reaches every member's dispatch entry
   (`dispatch_script_event`, `fire_vm_event`, `dispatch_vm_event`, `dispatch_event_at_sw_scope`,
   `fire_vm_progress_event`); a member it does not reach needs a P0a-1 injection point, which is a design
   change back to P0a-1's review.

webref lookups behind §3 (2026-11-08): `dfn IndexedDB-3 'retrieve a value from an object store'` → §6.2;
`dfn service-workers 'startMessages()'` → §3.4.6, `body service-workers dfn-client-message-queue`;
`dfn webcrypto 'wrapKey()'` → §14.3.11, `dfn webcrypto-2 'unwrapKey()'` → §14.3.12; (2026-11-10)
`dfn html 'report an exception'` → §8.1.4.6.
