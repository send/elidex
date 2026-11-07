# VM stack-safety guard — P0b-VM plan (history, IndexedDB, SW messages, worker, WebCrypto consumers)

Status: **plan (pre-implementation)**, per-PR slice of the [P0 umbrella](2026-11-vm-stack-safety-guard-p0.md)
(PM 509); `/elidex-plan-review` required before code. Owner: VM P4 (PM 509 item 2). Base: `ce603c74`;
every code anchor below is at that commit. Requires [P0a](2026-11-vm-stack-safety-guard-p0a.md) (the
error identity, the injection points and the census check); edges and schedule: P0 umbrella §1.

The contract C-1…C-5 is the P0 umbrella §4 (canonical); each fix below applies it at one site with
`is_resource_limit()` (P0a §4.1) as the only test. This slice is a base case (c) under the P0 umbrella:
in the census it only removes its rows from `# pending` (P0a §5.4) and adds the admissible rows named in
§6.

## §0. Charter

P0b-VM fixes the definitive consumers D-6…D-12 of P0a §5.4 (the D-1…D-5 rows are P0b-L3's). D-9's fix
also makes `startMessages()`'s flush asynchronous, as the Service Workers spec has it (PM 509 item 6).

**L4 sites (L4 has no driver; VM P4 edits them in this PR).** Every L4 site this PR touches:

| File | Function | Line at `ce603c74` | Change |
|---|---|---|---|
| `crates/script/elidex-js/src/vm/host/structured_serialize.rs` | `structured_serialize_for_storage` | 74–86 (match at 78) | returns `Result<Option<Vec<u8>>, VmError>` (D-6) |
| same | `structured_deserialize` | 93–99 (`unwrap_or` at 97) | returns `Result<JsValue, VmError>` (D-7) |
| same | `seed_history_state` (on `VmInner`) | 107–109 | takes the catch-null for a non-resource failure; caller is the embedder only after this PR (§6.1) |
| `crates/script/elidex-js/src/vm/host/history.rs` | `state_mutate` | 147–283 (serialize at 188, commit at 248–249) | propagates D-6; deserialises before the `current_url` write (D-7) |
| `crates/script/elidex-js/src/vm/host/history_events.rs` | `reconstruct_history_state` | 202–207 | applies the catch-null on the `Result` (traversal) |

`history_events.rs` is counted as L4 because it is the history traversal file; PM to confirm (§10 item 5).

## §1. Scope

**In.** The L4 files above; `vm/host/indexeddb/value.rs` and its callers in `indexeddb/cursor.rs`,
`index.rs`, `object_store.rs`; `vm/host/service_worker/deliver.rs`, `vm/host/pending_tasks.rs` (one task
variant), `vm/gc/roots.rs` (its exhaustive arm); `vm/host/worker_scope.rs`; `vm/host/subtle_crypto/ops.rs`;
the census allowlist rows of §6; cells in `crates/script/elidex-js/src/vm/tests/tests_resource_limit_host.rs`
(NEW) (a separate file from P0a's so neither grows past the touch-time split line).

**Out.** Every non-resource failure at these sites keeps its present behaviour, owned by the slots of §9;
the HTML "report an exception" event (`#11-embedder-diagnostic-channel`).

## §2. Coupled invariants

Elements: **HS** (history serialize D-6 and deserialise D-7), **IDB** (D-8), **SW** (D-9 and the task
queue), **WK** (worker D-10, D-11), **WC** (WebCrypto D-12), **ID** (P0a's predicate and injection points,
consumed here), **PEND** (this slice's rows of `# pending`).

| Pair | Intersection | Pinned by | Basis |
|---|---|---|---|
| HS-ser × HS-de | push/replace deserialises the bytes right after step 3 and before any commit (the `current_url` write at `history.rs:248` and step 10), and uses that value at *URL and history update steps* step 8 (no second deserialise); a C-1 error at either propagates with nothing committed | `rl_push_state_resource_limit_throws_no_entry`; M-HS1, M-HS2 | cell |
| HS-de × traversal | `structured_deserialize` returns `Result`; its remaining callers (`seed_history_state` from `engine.rs:609`, `reconstruct_history_state` in the traversal drain) take step 2's catch-null and must be top-only | rows N, P0a §5.2 rule 5 | check |
| HS × PEND | the D-6 / D-7 rows leave `# pending`; D-6 and D-7 come back as P rows, the two callers as N rows | §6.1 | check |
| IDB × request | each of `json_to_js`'s seven callers turns a C-1 error into `DeferredOutcome::Error` with the materialised `RangeError`; the request fails on its own channel | `rl_idb_get_resource_limit_errors_request`; M-IDB | cell |
| IDB × non-resource | a non-resource parse failure keeps today's `Undefined` (pre-existing, `#11-idb-structured-clone-storage`), so `json_to_js` itself stays a member (P) | §6.2 | check |
| SW × N | after the flush is queued, `fire_message` runs only from the task drain; its `unwrap_or(Undefined)` is then an N row (top-only), which rule 5 checks | `rl_sw_start_messages_is_async`; M-SW | cell + check |
| SW × order | the flush and `deliver_message`'s enabled branch both queue the same task kind, so buffered messages stay ahead of a later one (FIFO on one queue) | `rl_sw_message_order_preserved` | cell |
| WK × ThrowValue | D-10's resource-limit arm sits beside the existing `ThrowValue` arm, both before the DataCloneError arm | `rl_worker_post_message_resource_limit_is_range_error`; M-W1 | cell |
| WK × ID | D-11's `map_err` is kind-preserving (P0a §4.1 item 3), before the `TypeError` mapping | `rl_import_scripts_compile_resource_limit_is_range_error`; M-W2 | cell |
| WC × spec fallback | a C-1 error from the primary normalisation returns without running step 3's fallback | `rl_wrap_key_resource_limit_skips_fallback`; M-WC | cell |
| ID × all | every fix tests only `is_resource_limit()`, and every cell uses only P0a's injection points (`Entry(Call, n)`, `Compile`, `JsonParse`, `JsonStringify`) | P0a M-ID kills each cell of §7 | cell |
| PEND × all | this PR deletes exactly its 8 pending rows and adds no pending row (P0a §5.2 rule 4 ceiling) | P0a control `census_pending_row_outside_ceiling` | check |

## §3. Spec coverage map

Every number is from `.claude/tools/webref` (commands at the end of §10).

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §7.2.5 The History interface | *shared history push/replace state steps* step 3 (StructuredSerializeForStorage, rethrow) and step 10 (URL and history update steps) | propagates; no entry committed | `structured_serialize_for_storage` (`structured_serialize.rs:78`), `history.rs:188` | ✓ | yes |
| WHATWG HTML §7.4.6.2 Updating the document | *restore the history object state* step 2 (catch, state null), reached from push/replace through *URL and history update steps* step 8 | push/replace: deserialised before commit, propagates; traversal (event-loop top): the spec branch | `structured_deserialize` (`structured_serialize.rs:97`) | ✓ | yes |
| WHATWG HTML §9.4.4 Message ports | *message port post message steps* step 5 (StructuredSerializeWithTransfer, rethrow) | rethrown as `RangeError`, not DataCloneError | `serialize_message` (`worker_scope.rs:386`) | ✓ | yes |
| WHATWG HTML §10.3.1 Importing scripts and libraries | *import scripts into worker global scope* step 6.2 (run the classic script with rethrow errors) | rethrown as `RangeError`, not `TypeError` | `native_import_scripts` (`worker_scope.rs:481`) | ✓ | yes |
| IndexedDB-3 §6.2 Object store retrieval operations | *retrieve a value from an object store* step 4 (`! StructuredDeserialize`) | stated deviation at a `!`: the request fails with the `RangeError` | `json_to_js` (`indexeddb/value.rs:463`) and its seven callers | ✓ | yes |
| Service Workers §3.4.6 startMessages() | enable the client message queue; §3.4: an enabled queue is a task source | buffered messages are delivered as tasks, not synchronously | `enable_sw_messages`, `deliver_message`, `fire_message` (`deliver.rs:287`, `:263`, `:359`) | ✓ | yes |
| Web Cryptography API §14.3.11 The wrapKey method | steps 2–4 (normalize for "wrapKey"; on error normalize for "encrypt"; on error reject) | step 3 is not taken; rejects with the `RangeError` | `normalize_wrap_with_fallback` (`subtle_crypto/ops.rs:66`, caller `:655`) | ✓ | yes |
| Web Cryptography API §14.3.12 The unwrapKey method | steps 2–4 (same, "unwrapKey" / "decrypt") | same | same helper (`ops.rs:720` caller) | ✓ | yes |

**Breadth**: K=4 specs (WHATWG HTML, IndexedDB-3, Service Workers, Web Cryptography API), M=8 entries
(verified 2026-11-08 by `python3 .claude/skills/elidex-plan-review/preflight.py` on this memo: rc 0, K=4,
M=8, SPLIT-RECOMMENDED; `IndexedDB-3` and `Service Workers` are labels preflight does not map, so their
two rows were looked up by hand, §10).
**Split decision: single-PR appeal.** K=4 meets SPLIT-RECOMMENDED (K≥4) and is below SPLIT-DEFAULT. The
four specs come from four independent consumer sites, not from intersecting invariants: §2's cross-element
pairs are only ID × all and PEND × all, both P0a's mechanisms consumed unchanged; the other pairs stay
inside one element. Each fix is one arm against one predicate, in disjoint files, with one injection
harness. Splitting by spec would give PRs of one or two sites each that restate the same §2 rows. If the
PM prefers the split, the seam is by file family: P0b-VM-1 = D-6, D-7, D-10, D-11 (WHATWG HTML only,
K=1, M=4, the L4 files plus `worker_scope.rs`) and P0b-VM-2 = D-8, D-9, D-12 (K=3, M=4); both stay base
cases (c) and each removes its own pending rows.

### §3.1 User-input touch audit

Every touched consumer is reachable from script through a native whose input is script-controlled: the
caller's stack depth (which decides whether the check fails) and the data (the value passed to
`pushState` / `postMessage`, the stored IndexedDB value, the algorithm dictionary, the imported script).
D-7 and D-9 are caller-depth reachable: `native_history_replace_state` → `state_mutate` →
`seed_history_state` → `structured_deserialize` (`history.rs:248-249`, after `current_url` is already
set), and `native_container_start_messages` → `enable_sw_messages` → `fire_message` (`container.rs:366`,
`deliver.rs:287-297`).

## §4. Rows fixed

| Id | Site | Carrier | What the error becomes today | Class | Governing step |
|---|---|---|---|---|---|
| D-6 | `vm/host/structured_serialize.rs:78` | `stringify_for_structured_shortcut` | `_ => None`: `pushState` commits an entry with no state | D-sub | HTML §7.2.5 step 3 (rethrow) |
| D-7 | `structured_serialize.rs:97` (from push/replace) | `parse_json_str` | `unwrap_or(Null)`: `history.state` null while the entry holds state | D-sub | HTML §7.4.6.2 step 2, via *URL and history update steps* step 8 |
| D-8 | `vm/host/indexeddb/value.rs:463` `json_to_js` | `parse_json_str` | `unwrap_or(Undefined)`: `get` / `getAll` / index reads / cursor value succeed with `undefined` | D-sub | IndexedDB-3 §6.2 step 4 |
| D-9 | `vm/host/service_worker/deliver.rs:359` `fire_message` (reached synchronously from `startMessages()`, `deliver.rs:287-297`) | `parse_json_str` | `unwrap_or(Undefined)`: the message is consumed with `data` undefined | D-sub | Service Workers §3.4.6, §3.4 |
| D-10 | `vm/host/worker_scope.rs:386` `serialize_message` | `stringify_for_structured_shortcut` | a non-`ThrowValue` `Err` → DataCloneError | D-cls | HTML §9.4.4 step 5 |
| D-11 | `worker_scope.rs:481` `native_import_scripts` | `compile_script` | `TypeError` | D-cls | HTML §10.3.1 step 6.2 |
| D-12 | `vm/host/subtle_crypto/ops.rs:66` (+ `:68`, same site) | `marshal_normalize` | the step-3 fallback runs; may answer NotSupportedError or a different normalisation | D-cls | Web Cryptography API §14.3.11 / §14.3.12 steps 2–4 |

## §5. Fixes

### §5.1 D-6, D-7: `pushState` / `replaceState`

`structured_serialize_for_storage` returns `Result<Option<Vec<u8>>, VmError>`: a C-1 error propagates (HTML
§7.2.5 step 3, rethrow; no entry is committed); every other failure keeps today's degrade, owned by
`#11-history-state-structured-serialize-fidelity`. `structured_deserialize` returns `Result<JsValue,
VmError>`: a C-1 error propagates; any other decode failure is `Ok(Null)` (step 2's catch). `state_mutate`
deserialises the serialized bytes right after step 3 with `?`, before the `current_url` write, and writes
the value with `VmInner::set_history_state_value` (NEW) at *URL and history update steps* step 8 (no second
deserialise). After this, `seed_history_state` has one production caller, the embedder's
`set_history_state` (`engine.rs:609`, document construction, event-loop top, N), and
`reconstruct_history_state` (`history_events.rs:205`, traversal drain, N); both take the catch-null.

### §5.2 D-8: IndexedDB reads

`json_to_js` returns `Result<JsValue, VmError>`: a C-1 error propagates; a non-resource parse failure
keeps `Ok(Undefined)` (pre-existing, `#11-idb-structured-clone-storage`). Its seven callers
(`git grep -n 'json_to_js(' -- crates/script/elidex-js/src` minus the definition: `indexeddb/cursor.rs:145`,
`index.rs` 246 / 262 / 333, `object_store.rs` 365 / 373 / 477; verified 2026-11-08) turn a C-1 error into
`DeferredOutcome::Error` with the materialised `RangeError`, so the request fails on its own channel
(IndexedDB-3 §6.2 step 4 is a `!`: a stated resource-limit deviation, like step 3's NotReadableError
result).

### §5.3 D-9: service-worker messages

`PendingTask::SwMessage { data, source_scope }` (NEW, `pending_tasks.rs`; its `gc/roots.rs` arm holds no
GC reference) is the one delivery path of a client message once the queue is enabled: `enable_sw_messages`
queues each buffered message as that task instead of calling `fire_message`, and `deliver_message`'s
enabled branch queues it too, so the order is FIFO on one queue (Service Workers §3.4: an enabled client
message queue is a task source). `fire_message` then runs only from the task drain (top-only, N); its
`unwrap_or(Undefined)` for other deserialise failures is the pre-existing substitution of the SW message
path (spec: `messageerror`), recorded in §9 L-b3.

### §5.4 D-10, D-11, D-12

- D-10: `serialize_message` gains `Err(e) if e.is_resource_limit() => Err(e)` before the DataCloneError
  arm.
- D-11: `native_import_scripts` maps `CompileErrorKind::ResourceLimit` (NEW: P0a) to `VmError::resource_limit()`
  before the `TypeError` mapping (the `TypeError`-for-SyntaxError mapping is pre-existing, not P0's).
- D-12: `normalize_wrap_with_fallback` returns a C-1 error from the primary normalisation without
  running the fallback (step 3 is for errors the spec defines).

## §6. Census rows after the fix

Deleted from `# pending`: D-6, D-7, D-8, D-9, D-10, D-11, D-12 (2) — 8 rows. Added as admissible (keys as
in P0a §5.1):

| Site | Class | Reason |
|---|---|---|
| `structured_serialize_for_storage` → `stringify_for_structured_shortcut` | P | resource-limit arm propagates; other failures degrade (`#11-history-state-structured-serialize-fidelity`) |
| `structured_deserialize` → `parse_json_str` | P | resource-limit arm propagates; other failures catch-null (HTML §7.4.6.2 step 2) |
| `seed_history_state` → `structured_deserialize`; `reconstruct_history_state` → `structured_deserialize` | N | top-only (embedder document construction; traversal drain), rule 5 |
| `json_to_js` → `parse_json_str` | P | resource-limit arm propagates; other failures `Undefined` (`#11-idb-structured-clone-storage`) |
| the seven `json_to_js` callers | R | request error (`DeferredOutcome::Error`) |
| `fire_message` → `parse_json_str` | N | top-only (task drain), rule 5 |
| `serialize_message` → `stringify_for_structured_shortcut` | P | resource-limit and `ThrowValue` arms propagate; others DataCloneError |
| `native_import_scripts` → `compile_script` | P | kind-preserving `map_err` |
| `normalize_wrap_with_fallback` → `marshal_normalize` (2) | P | resource-limit arm propagates; fallback only for other errors |

The PR body records the census rc 0 and the class counts at its head.

## §7. Cells (written first)

New file `crates/script/elidex-js/src/vm/tests/tests_resource_limit_host.rs` (NEW); P0a §4.4's injection,
`n` from `resource_limit::count` on a dry run.

| Cell | Body | Expect |
|---|---|---|
| `rl_push_state_resource_limit_throws_no_entry` | `history.pushState({ get x() { return 1 } }, '', '#a')` with `Entry(Call, 1)`; then `JsonParse(1)`; the same for `replaceState` | each throws `RangeError`; `history.length`, `history.state` and `location.href` unchanged |
| `rl_idb_get_resource_limit_errors_request` | a stored record; inject `JsonParse(1)`; `store.get(k)`, `getAll`, an index `get`, a cursor | each request errors with `RangeError` (no success with `undefined`); un-injected reads return the value |
| `rl_sw_start_messages_is_async` | buffered client messages; `navigator.serviceWorker.startMessages()` | no `message` event during the call; events fire after the task drain |
| `rl_sw_message_order_preserved` | messages A, B buffered; `startMessages()`; message C delivered before the drain | events in the order A, B, C |
| `rl_worker_post_message_resource_limit_is_range_error` | worker scope `postMessage({ get x() { return 1 } })` with `Entry(Call, 1)`; then `JsonStringify(1)` | `RangeError`, not DataCloneError |
| `rl_import_scripts_compile_resource_limit_is_range_error` | `importScripts(url)` with `Compile(1)` (§10 item 1) | `RangeError`, not `TypeError` |
| `rl_wrap_key_resource_limit_skips_fallback` | `wrapKey` / `unwrapKey` with `{ get name() { k++; return 'AES-KW' } }` and `Entry(Call, 1)` | rejects `RangeError`; `k === 1` |

## §8. Mutants (run on the final head, debug)

| Mutant | Change | Dies |
|---|---|---|
| M-HS1 | serialize error → `None` | `rl_push_state_resource_limit_throws_no_entry` |
| M-HS2 | deserialise after the `current_url` write | `rl_push_state_resource_limit_throws_no_entry` (`location.href`) |
| M-IDB | `unwrap_or(Undefined)` back | `rl_idb_get_resource_limit_errors_request` |
| M-SW | synchronous flush back | `rl_sw_start_messages_is_async` |
| M-SW2 | `deliver_message`'s enabled branch fires synchronously | `rl_sw_message_order_preserved` |
| M-W1 / M-W2 / M-WC | DataCloneError first / `TypeError` / fallback on resource limit | the matching cell |
| M-ID (P0a) | `is_resource_limit` always false | every cell of §7 except the two SW cells |

## §9. Landing obligations

What this PR (or, where marked, the PM / PMO at landing) does. This memo edits no ledger.

- **L-b1. `#11-history-state-structured-serialize-fidelity` (PM / PMO)**: the entry gains "resource-limit
  errors propagate from the interim serializer and from the push/replace deserialise (stack-safety P0b-VM
  §5.1); the degrade for other failures remains this slot's".
- **L-b2. `#11-idb-structured-clone-storage` (PM / PMO)**: "`json_to_js` is fallible (P0b-VM §5.2)".
- **L-b3. SW message deserialise substitution (PM / PMO)**: `fire_message`'s `unwrap_or(Undefined)` for a
  non-resource deserialise failure (spec: `messageerror`) is recorded on the SW slot the PM names.
- **L-b4. L4 notice (PM)**: the L4 sites of §0 were edited by VM P4 in this PR.

## §10. Unverified facts (each decided at implementation, before its cell is written)

1. Whether the worker test harness can run `importScripts` against a served script, and whether the
   worker / SW cells run in the in-process VM harness or need their own module.
2. That the SW task drain and `deliver_message`'s caller (`deliver.rs:50`) are not reachable from a native
   (P0a §5.2 rule 5 decides; if reachable, the N row is wrong and this is a design change back to review).
3. Same for `seed_history_state` (after §5.1) and `reconstruct_history_state`.
4. Every exhaustive `match` on `PendingTask` besides `gc/roots.rs:557` and `pending_tasks.rs:287`.
5. `history_events.rs` counted as L4 (PM).

webref lookups behind §3 (2026-11-08): `dfn html 'shared history push/replace state steps'` → §7.2.5;
`dfn html 'restore the history object state'` → §7.4.6.2; `body html url-and-history-update-steps`
(step 8); `dfn html 'message port post message steps'` → §9.4.4; `dfn html 'import scripts into worker
global scope'` → §10.3.1; `dfn IndexedDB-3 'retrieve a value from an object store'` → §6.2;
`dfn service-workers 'startMessages()'` → §3.4.6, `body service-workers dfn-client-message-queue`;
`dfn webcrypto 'wrapKey()'` → §14.3.11, `dfn webcrypto-2 'unwrapKey()'` → §14.3.12.
