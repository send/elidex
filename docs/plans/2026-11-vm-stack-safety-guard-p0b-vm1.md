# VM stack-safety guard — P0b-VM-1 plan (history and worker consumers, WHATWG HTML)

Status: **plan (pre-implementation)**, per-PR slice of the [P0 umbrella](2026-11-vm-stack-safety-guard-p0.md)
(PM 509; split from the former P0b-VM by PM 513 (a)); `/elidex-plan-review` required before code. Owner:
VM P4 (PM 509 item 2). Base: `ce603c74`; every code anchor below is at that commit. Requires
[P0a-1](2026-11-vm-stack-safety-guard-p0a1.md) (the error identity, the injection points) and
[P0a-2](2026-11-vm-stack-safety-guard-p0a2.md) (the census check); the slice starts from P0a-2, whose
prerequisite is P0a-1 (P0 umbrella §1).
Sibling: [P0b-VM-2](2026-11-vm-stack-safety-guard-p0b-vm2.md) (independent: disjoint files, disjoint
`# pending` rows). Edges and schedule: P0 umbrella §1.

The contract C-1…C-5 is the P0 umbrella §4 (canonical); each fix below applies it at one site with
`is_resource_limit()` (P0a-1 §4.1) as the only test. This slice is a base case (c) under the P0 umbrella: in
the census it only removes its rows from `# pending` (P0a-2 §7.3) and adds the admissible rows of §6.

**Why two slices, not one (PM 513 (a)).** The former single memo appealed for one PR because its four
specs came from four independent consumer sites. PM 513 ruled the other way: four independent sites are a
reason to split, not an argument for a single PR. The seam is the one that memo named: by file family. This
slice holds the WHATWG HTML consumers (the L4 history files and `worker_scope.rs`); P0b-VM-2 holds
IndexedDB, Service Workers and WebCrypto.

## §0. Charter

P0b-VM-1 fixes the definitive consumers D-6, D-7, D-10 and D-11 of P0a-2 §7.3.

**L4 sites (L4 has no driver; VM P4 edits them in this PR).** `history_events.rs` counts as an L4 file and
this site list is sufficient (PM 513 (b)). Every L4 site this PR touches:

| File | Function | Line at `ce603c74` | Change |
|---|---|---|---|
| `crates/script/elidex-js/src/vm/host/structured_serialize.rs` | `structured_serialize_for_storage` | 74–86 (match at 78) | returns `Result<Option<Vec<u8>>, VmError>` (D-6) |
| same | `structured_deserialize` | 93–99 (`unwrap_or` at 97) | returns `Result<JsValue, VmError>` (D-7) |
| same | `seed_history_state` (on `VmInner`) | 107–109 | takes the catch-null for a non-resource failure; caller is the embedder only after this PR (§5.1) |
| `crates/script/elidex-js/src/vm/host/history.rs` | `state_mutate` | 147–283 (serialize at 188, commit at 248–249) | propagates D-6; deserialises before the `current_url` write (D-7) |
| `crates/script/elidex-js/src/vm/host/history_events.rs` | `reconstruct_history_state` | 202–207 | applies the catch-null on the `Result` (traversal) |

## §1. Scope

**In.** The L4 files above; `crates/script/elidex-js/src/vm/host/worker_scope.rs` (D-10, D-11); the census
allowlist rows of §6; cells in `crates/script/elidex-js/src/vm/tests/tests_resource_limit_html.rs` (NEW), a
file of its own so neither this slice nor P0b-VM-2 creates a file the other also creates.

**Out.** Every non-resource failure at these sites keeps its present behaviour, owned by the slots named in
§5; the HTML "report an exception" event (`#11-embedder-diagnostic-channel`); D-8, D-9, D-12 (P0b-VM-2).

## §2. Coupled invariants

Elements: **HS** (history serialize D-6 and deserialise D-7), **WK** (worker D-10, D-11), **ID** (P0a-1's
predicate and injection points, consumed here), **PEND** (this slice's rows of `# pending`).

| Pair | Intersection | Pinned by | Basis |
|---|---|---|---|
| HS-ser × HS-de | push/replace deserialises the bytes right after step 3 and before any commit (the `current_url` write at `history.rs:248` and step 10), and uses that value at *URL and history update steps* step 8 (no second deserialise); a C-1 error at either propagates with nothing committed | `rl_push_state_resource_limit_throws_no_entry`; M-HS1, M-HS2 | cell |
| HS-de × traversal | `structured_deserialize` returns `Result`; its remaining callers (`seed_history_state` from `engine.rs:609`, `reconstruct_history_state` in the traversal drain) take step 2's catch-null and must be top-only | rows N, P0a-2 §5 rule 5 | check |
| HS × WK | disjoint files; `stringify_for_structured_shortcut` is a carrier both reach, but each consumer applies its own arm | — | structural |
| HS × PEND | the D-6 / D-7 rows leave `# pending`; D-6 and D-7 come back as P rows, the two callers as N rows | §6 | check |
| WK × ThrowValue | D-10's resource-limit arm sits beside the existing `ThrowValue` arm, both before the DataCloneError arm | `rl_worker_post_message_resource_limit_is_range_error`; M-W1 | cell |
| WK × ID | D-11's `map_err` is kind-preserving (P0a-1 §4.1 item 3), before the `TypeError` mapping | `rl_import_scripts_compile_resource_limit_is_range_error`; M-W2 | cell |
| WK × PEND | D-10 / D-11 leave `# pending` and come back as P rows | §6 | check |
| ID × HS, ID × WK | every fix tests only `is_resource_limit()`, and every cell uses only P0a-1's injection points (`Entry(Call, n)`, `Compile`, `JsonParse`, `JsonStringify`) | P0a-1 M-ID kills each cell of §7 | cell |
| PEND × ID | this PR deletes exactly its 4 pending rows and adds no pending row (P0a-2 §5 rule 4 ceiling) | P0a-2 control `census_pending_row_outside_ceiling` | check |

## §3. Spec coverage map

Every number is from `.claude/tools/webref` (commands at the end of §10).

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §7.2.5 The History interface | *shared history push/replace state steps* step 3 (StructuredSerializeForStorage, rethrow) and step 10 (URL and history update steps) | propagates; no entry committed | `structured_serialize_for_storage` (`structured_serialize.rs:78`), `history.rs:188` | ✓ | yes |
| WHATWG HTML §7.4.6.2 Updating the document | *restore the history object state* step 2 (catch, state null), reached from push/replace through *URL and history update steps* step 8 | push/replace: deserialised before commit, propagates; traversal (event-loop top): the spec branch | `structured_deserialize` (`structured_serialize.rs:97`) | ✓ | yes |
| WHATWG HTML §9.4.4 Message ports | *message port post message steps* step 5 (StructuredSerializeWithTransfer, rethrow) | rethrown as `RangeError`, not DataCloneError | `serialize_message` (`worker_scope.rs:386`) | ✓ | yes |
| WHATWG HTML §10.3.1 Importing scripts and libraries | *import scripts into worker global scope* step 6.2 (run the classic script with rethrow errors) | rethrown as `RangeError`, not `TypeError` | `native_import_scripts` (`worker_scope.rs:481`) | ✓ | yes |

**Breadth**: K=1 spec (WHATWG HTML), M=4 entries (`python3 .claude/skills/elidex-plan-review/preflight.py`
on this memo, 2026-11-08: see the fold commit's message for rc and figures). Below the split thresholds.

### §3.1 User-input touch audit

Every touched consumer is reachable from script through a native whose input is script-controlled: the
caller's stack depth (which decides whether the check fails) and the data (the value passed to `pushState`
/ `postMessage`, the imported script). D-7 is caller-depth reachable: `native_history_replace_state` →
`state_mutate` → `seed_history_state` → `structured_deserialize` (`history.rs:248-249`, after
`current_url` is already set).

## §4. Rows fixed

| Id | Site | Carrier | What the error becomes today | Class | Governing step |
|---|---|---|---|---|---|
| D-6 | `vm/host/structured_serialize.rs:78` | `stringify_for_structured_shortcut` | `_ => None`: `pushState` commits an entry with no state | D-sub | HTML §7.2.5 step 3 (rethrow) |
| D-7 | `structured_serialize.rs:97` (from push/replace) | `parse_json_str` | `unwrap_or(Null)`: `history.state` null while the entry holds state | D-sub | HTML §7.4.6.2 step 2, via *URL and history update steps* step 8 |
| D-10 | `vm/host/worker_scope.rs:386` `serialize_message` | `stringify_for_structured_shortcut` | a non-`ThrowValue` `Err` → DataCloneError | D-cls | HTML §9.4.4 step 5 |
| D-11 | `worker_scope.rs:481` `native_import_scripts` | `compile_script` | `TypeError` | D-cls | HTML §10.3.1 step 6.2 |

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

### §5.2 D-10, D-11

- D-10: `serialize_message` gains `Err(e) if e.is_resource_limit() => Err(e)` before the DataCloneError
  arm.
- D-11: `native_import_scripts` maps `CompileErrorKind::ResourceLimit` (NEW: P0a-1) to
  `VmError::resource_limit()` before the `TypeError` mapping (the `TypeError`-for-SyntaxError mapping is
  pre-existing, not P0's).

## §6. Census rows after the fix

Deleted from `# pending`: D-6, D-7, D-10, D-11 — 4 rows. Added as admissible (keys as in P0a-2 §7.3):

Codes are P0a-2 §5 rule 12's closed table; a row with no fitting code is not given an invented one.

| Site | Class · code | Reason |
|---|---|---|
| `structured_serialize_for_storage` → `stringify_for_structured_shortcut` | P · — (no code of rule 12 fits; open) | resource-limit arm propagates; other failures degrade (`#11-history-state-structured-serialize-fidelity`) |
| `structured_deserialize` → `parse_json_str` | P · — (no code of rule 12 fits; open) | resource-limit arm propagates; other failures catch-null (HTML §7.4.6.2 step 2) |
| `seed_history_state` → `structured_deserialize`; `reconstruct_history_state` → `structured_deserialize` (2 rows) | N · `top-only` | top-only (embedder document construction; traversal drain), rule 5 |
| `serialize_message` → `stringify_for_structured_shortcut` | P · — (no code of rule 12 fits; open) | resource-limit and `ThrowValue` arms propagate; others DataCloneError |
| `native_import_scripts` → `compile_script` | P · `propagates` | kind-preserving `map_err` |

## §7. Cells (written first)

New file `crates/script/elidex-js/src/vm/tests/tests_resource_limit_html.rs` (NEW); P0a-1 §4.4's injection,
`n` from `resource_limit::count` on a dry run.

| Cell | Body | Expect |
|---|---|---|
| `rl_push_state_resource_limit_throws_no_entry` | `history.pushState({ get x() { return 1 } }, '', '#a')` with `Entry(Call, 1)`; then `JsonParse(1)`; the same for `replaceState` | each throws `RangeError`; `history.length`, `history.state` and `location.href` unchanged |
| `rl_worker_post_message_resource_limit_is_range_error` | worker scope `postMessage({ get x() { return 1 } })` with `Entry(Call, 1)`; then `JsonStringify(1)` | `RangeError`, not DataCloneError |
| `rl_import_scripts_compile_resource_limit_is_range_error` | `importScripts(url)` with `Compile(1)` (§10 item 1) | `RangeError`, not `TypeError` |

## §8. Mutants (run on the final head, debug)

| Mutant | Change | Dies |
|---|---|---|
| M-HS1 | serialize error → `None` | `rl_push_state_resource_limit_throws_no_entry` |
| M-HS2 | deserialise after the `current_url` write | `rl_push_state_resource_limit_throws_no_entry` (`location.href`) |
| M-W1 | DataCloneError arm first | `rl_worker_post_message_resource_limit_is_range_error` |
| M-W2 | `TypeError` for the resource-limit kind | `rl_import_scripts_compile_resource_limit_is_range_error` |
| M-ID (P0a-1) | `is_resource_limit` always false | every cell of §7 |

## §9. Landing obligations

The ledger and repo-text rows this slice triggers are rows of the P0 program's landing list, P0 umbrella §7
(PM 513 (d)): **L-8** (`#11-history-state-structured-serialize-fidelity`, done by this PR in the repo memo) and **L-11** (L4 notice).
Slice-local:

- **L-v1-1. Counts**: the PR body records the census rc 0 and the class counts at its head, and the
  preflight rc / K / M of this memo (P0 umbrella §7 L-13).

## §10. Unverified facts (each decided at implementation, before its cell is written)

1. Whether the worker test harness can run `importScripts` against a served script, and whether the worker
   cells run in the in-process VM harness or need their own module.
2. That `seed_history_state` (after §5.1) and `reconstruct_history_state` are not reachable from a native
   (P0a-2 §5 rule 5 decides; if reachable, the N row is wrong and this is a design change back to review).

webref lookups behind §3 (2026-11-08): `dfn html 'shared history push/replace state steps'` → §7.2.5;
`dfn html 'restore the history object state'` → §7.4.6.2; `body html url-and-history-update-steps`
(step 8); `dfn html 'message port post message steps'` → §9.4.4; `dfn html 'import scripts into worker
global scope'` → §10.3.1.
