# VM stack-safety guard — P0a-2b plan (the census allowlist and `# pending`, from the classified population)

Status: **plan (pre-implementation)**, per-PR slice of the [P0 umbrella](2026-11-vm-stack-safety-guard-p0.md)
(PM 509; re-sliced from P0a by PM 544; P0a-2 split into [P0a-2a](2026-11-vm-stack-safety-guard-p0a2a.md)
and P0a-2b by PM 609). `/elidex-plan-review` required before code, and it starts only when the
classification of §6 is done. Owner: VM P4. Base: `ce603c74`; every code anchor below is at that commit
until §6's re-measurement replaces it. Provenance root `H` and the prototypes: P0a-2a header.

The contract C-1…C-5 is the P0 umbrella §4; the census definition is the P0 umbrella §5; the check, its
rules, its reason codes and its row format are [P0a-2a](2026-11-vm-stack-safety-guard-p0a2a.md) §4–§5; the
measured population is P0a-2a §7. This memo does not restate them; an unqualified rule number names a
rule of P0a-2a §5. Why the slice is split, and the order
substrate → classification → ratification of the allowlist (PM 608, PM 609): P0a-2a header "The split".
Edges, schedule and the other slices: P0 umbrella §1.

## §0. Charter

P0a-2b ships the allowlist `.claude/tools/err-consumer-census.tsv` with its `# pending` section, switches on
the check's gate mode on the tree (P0a-2a §0 "Two modes"), and makes the legibility rewrites the rows need.
Every row is admitted through a reason code whose predicate P0a-2a's check runs (PM 544); this memo
chooses rows, never reasons. The rows come from the classification of §6, not from this memo's prose:
where §7 names a row and the classification disagrees, the classification is the input and the
disagreement is reported in the PR body.

## §1. Scope

**In.** `.claude/tools/err-consumer-census.tsv` (NEW); the gate run on the tree added to
`.claude/tools/test_err_consumer_census.py` (rc 0 with the allowlist); legibility rewrites in
`crates/script/elidex-js/src/` where a row's propagation has no shape the check follows and the code can
be written in one it does (§7.4; no behaviour change, each named in the PR body); the PR body's counts
(§10).

**Out.** The check, its rules, controls and mutants (P0a-2a); the classification itself (§6, between the
two slices); the contract's semantics and every consumer fix (P0a-1, P0b-VM-1, P0b-VM-2, P0b-L3).

## §2. Coupled invariants

Elements: **PEND** (`# pending` and its ceiling; §6, §7.3), **CLS** (the classification; §6), **ROW** (the
allowlist's admissible rows; §7), and P0a-1's **ID** and P0a-2a's **CEN** where the rows rest on them.

| Pair | Intersection | Pinned by | Basis |
|---|---|---|---|
| ID × PEND | D-13…D-16 are fixed in P0a-1, so they are admissible rows at P0a-2b's landing, never pending | §7.3 | structural |
| CLS × PEND | until the classification is done, every unclassified row counts as pending (the unknown on the unsafe side); the ceiling is fixed by this memo's ratification and only shrinks after it | §6; P0a-2a control `census_pending_row_outside_ceiling` | check |
| CLS × ROW | a row admitted on this memo's prose and not on the classification's output is a disagreement, reported, never silently kept | §0; the gate run (a row whose predicate fails is red) | check |
| ROW × CEN | every row's reason code has a predicate in P0a-2a §5 rule 12; a row this memo needs with no code is a design change to P0a-2a (the code table is closed) | P0a-2a control `census_reason_without_predicate_is_red` | check |
| PEND × P0b | each P0b slice deletes exactly its own pending rows; PR-1's precondition is an empty section | P0 umbrella §7 L-1 | check |

## §3. Spec coverage map

The allowlist implements no algorithm; the rows are the clauses this memo's row judgements rest on (the
check's own clauses are P0a-2a §3).

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG DOM §2.9 Dispatching events | *inner invoke* step 2.5 (once: remove before the call), 2.13.1 (report on a throw) | the write stands on every outcome (β); the call's error must be reported, not dropped | the D-5 / D-14 / D-15 rows (§7.3) | ✓ | yes |
| WHATWG DOM §3.2 Interface AbortSignal | *signal abort* (for AbortSignal) steps 4–6 | every dependent's abort steps run; the first `Err` surfaces after the loop | the `AbortSignal` abort row (§7.3) | ✓ | yes |
| WHATWG HTML §4.13.5 Upgrades | *upgrade an element* step 3 (state "failed" before the construct), step 11 (exception branch rethrows) | under β every failure takes the exception branch; nothing is reverted | D-3 / D-4 leave `# pending` as P rows (§7.3) | ✓ | yes |

**Breadth**: K=2 specs (WHATWG DOM, WHATWG HTML), M=3 entries (verified 2026-11-10 by `python3
.claude/skills/elidex-plan-review/preflight.py` on this memo; rc and K / M as printed in the PR body, P0
umbrella §7 L-13). Below the split thresholds.

## §4. The allowlist file

`.claude/tools/err-consumer-census.tsv`: the row format is P0a-2a §4's. A `# pending` section holds the D
rows (§7.3), each naming its fixing slice. The gate run on the tree is added to
`test_err_consumer_census.py` in this PR (rc 0), so from P0a-2b's landing every unknown member, stale row
and D row outside `# pending` is red in the `tools` job.

## §5. Rules

P0a-2a §5. This slice adds no rule and no reason code.

## §6. Precondition for review: the classification (PM 608, PM 609)

**The state at this revision.** `# pending` holds **11 classified pending keys + 574 unclassified rows**;
until the classification is done, unclassified rows count as pending. An unknown is counted on the unsafe
side (`memory/feedback_fail-safe-direction-before-list-completeness.md`). The 574 are P0a-2a §7.1's members
that only PM 603's reach admits, measured at `ce603c74`.

**The precondition.** P0a-2b's `/elidex-plan-review` starts only after the classification pass has
classified every member of the Err population that P0a-1's and P0a-2a's rows do not already admit,
under two conditions:

- (a) It measures again at the then-current main (today `b8b4d6f5`, which includes #615's split of
  `vm/interpreter.rs`). The `ce603c74` numbers are history; §7's lines are re-derived from that run.
- (b) It runs P0a-2a's landed checker (`--report`), whose resolved call graph is the first part of that
  checker (`_rust_callgraph.py`), not a throwaway script. So the pass runs after P0a-2a lands.

Its inputs are P0a-2a §7.1's sample (a hint scoped to its 46 rows) and callee breakdown (413 `expect`
collisions, 60 Ok-only `settle_promise`, about 101 to check by hand). Its output is one class and reason
code per member, the members of the UA-fire drop class (owners: P0 umbrella §1; members: [P0b-VM-2](2026-11-vm-stack-safety-guard-p0b-vm2.md) §1, P0b-VM-1 §5.3), and
every member that fails every code. A member that fails every code and is a consumer defect goes to the PM
with a proposed owner (PM 557 (4)) and, with the PM's owner, becomes a `# pending` key.

**The ceiling.** This memo's `# pending` key set when its review ratifies it is the ceiling of P0a-2a §5
rule 4: the 11 keys of §7.3 plus the keys the classification adds. From that ratification on no slice adds
a pending row; the section only shrinks. P0a-1's ratification does not wait on this (P0a-1 §1).

## §7. The allowlist at P0a-2b's landing

### §7.1 Inputs

The classification's output (§6) and P0a-2a §7's report. The rows below are the base's prototype rows,
kept as the expectation the classification is compared with (§0).

### §7.2 The drains (PM 540 (3))

Each of the 12 drains and `set_history_state` names its consuming rows (the
`drain-consumed` arguments; full lists per method in `H/logs/err-reason-check-p0a2.log`). First-hop
consumers: the microtask-checkpoint drains (`run_microtasks`, `tick_network`, `deliver_idb_versionchange`,
`deliver_sw_client_update`, `deliver_history_step_events`, `deliver_media_query_changes`,
`deliver_visual_viewport_events`) → `run_callback` → `call` (R), `run_reaction` → `call` (R),
`dispatch_unhandled_rejection_event` → `call` (R after P0a-1, D-15), `flush_ce_reactions` → `invoke_one`
(S; home P0 umbrella §6 item 2); `drain_reactions` → `flush_ce_reactions` → `invoke_one` (S), `invoke_callback` → `call` (R);
`deliver_mutation_records` / `deliver_resize_observations` / `deliver_intersection_observations` →
`deliver_to_observer_callbacks` → `call` (R); `deliver_sw_client_update` → also `fire_message` →
`parse_json_str` (D-9, pending → P0b-VM-2); `drain_worker_messages` → `dispatch_message_event_at` →
`parse_json_str` (N); `deliver_history_step_events` → also `reconstruct_history_state` →
`structured_deserialize` (D-7, pending → P0b-VM-1's N row); `set_history_state` → `structured_deserialize`
→ `parse_json_str` (D-7, pending → P0b-VM-1's N row `seed_history_state` → `structured_deserialize`). A
drain whose paths hit only `# pending` rows is admissible now and stays so when the pending row becomes
its slice's admissible row; a drain that reaches a producer through no row is red.

### §7.3 Composition at landing

Boundary side (after P0a-1): BC-1…BC-3 and `call_listener` are no longer B1 members (their return types
carry `ScriptFailure`, rule 11). Err side, the deltas P0a-1 causes: `engine.rs:275` is R `channel:report`;
`natives_promise.rs:591` is R `channel:report`; `natives_promise.rs:689` is R `channel` on both arms
(`settle_promise`, `fail_reaction_owner`; D-16); `script_dispatch_event_core`'s use of `call_listener`'s
result is R `surfaced-across-boundary` (implementing method `call_listener`, whose `:275` row is R
`channel:report`); the two traversal protos' uses of the `FilterError::Script` (NEW: P0a-1) arm are
auto-recognised or P `propagates`; D-13's row (`vm/interpreter.rs` `eval` → `compile_script`) is P
`propagates` (kind-preserving `map_err`). The exact counts are the tracked check's at P0a-2b's head (§10),
not these.

Admissible rows at the base (paths under `crates/script/elidex-js/src/` unless stated; lines at `ce603c74`;
each with its rule-12 code at landing; the classification re-derives them, §6):

- **R (32):** `engine.rs` 191 / 344 (`channel:EvalResult`); `vm/host/body_mixin.rs:289`;
  `vm/host/cache/natives.rs` 96 / 121 / 145 / 198 / 252 / 287 / 340 / 382 / 420 (`channel:finish`, the
  binding passed to `finish`, which rejects); `vm/host/custom_elements/creation.rs:238`, `flush.rs` 104 /
  183 / 202 / 234, `lookup.rs:183` (`channel:report`); `vm/host/fetch/mod.rs` 183 / 204;
  `vm/host/observer_common.rs:295` (`channel:report`); `vm/host/readable_stream/controller.rs:135`,
  `mod.rs` 410 / 639 (stream error / cancel rejection, C-4 non-re-entry); `vm/host/service_worker/container.rs`
  109 / 218; `vm/host/wasm/namespace.rs:578`; `vm/natives_generator.rs:472` (wrapper rejection; residual P0
  umbrella §6 item 5); `vm/natives_promise.rs` 648 / 689 / 747; `vm/natives_timer.rs:370`. After P0a-1:
  `engine.rs:275` and `vm/natives_promise.rs:591` (`channel:report`), and `vm/interpreter_call.rs:631`
  (`push_js_call_frame` → `make_async_coroutine_and_drive`, the wrapper is settled inside the callee:
  `channel`, moved from N).
- **S (2):** `vm/coerce.rs:246` (`to_display_string`, console display); `vm/host/custom_elements/flush.rs:70`
  (`invoke_one` reports inside). Home: P0 umbrella §6 item 2. `engine.rs:275` (D-14's engine half, PM 533
  item 3) and `vm/natives_promise.rs:591` (D-15, PM 544) are no longer S.
- **E (10):** `vm/dispatch_iter.rs` 52 / 309; `vm/ops.rs:60`; `vm/host/typed_array_static.rs` 743 / 801;
  `vm/natives_array_hof.rs:485`; `vm/webidl_sequence.rs` 141 / 149; `vm/host/url_search_params.rs:318`;
  `vm/host/structured_clone.rs:1064`. A `return` method called at the same depth as the failed step fails
  at its own entry the same way, so the C-1 error is what normally survives; the precedence itself is
  P0 umbrella §6 item 1.
- **N (7 + 6):** `vm/host/worker_scope.rs:277` (`top-only`: worker loop and the embedder's
  `drain_worker_messages`); `vm/sw_thread.rs` 217 / 474 and `vm/worker_thread.rs` 200 / 227 / 259
  (`thread-entry`); shell: `content/event_loop.rs:284`, `content/iframe/render.rs:93`,
  `content/iframe/thread.rs:100`, `lib.rs:310`, `pipeline.rs` 357 / 359 (`top-only`, `EvalResult.outcome`
  discarded at the event-loop top). `vm/ops_element.rs:134` leaves the population by the legibility rewrite
  (§7.4); `vm/interpreter_call.rs:631` moves to R.
- **X (2):** `engine.rs` 63 / 78 (embedder-test helpers, `test-support`). `vm/host/subtle_crypto/differential.rs`
  32 / 36 are in a `#[cfg(test)] mod differential;` file (`subtle_crypto/mod.rs:62-63`), excluded by rule 1:
  not members.
- **P (80) and T (44):** listed in `H/logs/err-census-p0-classified.tsv` (rule 8's 120 left, PM 598); T = `vm/dispatch.rs` (40), `dispatch_objects.rs` 48 / 80,
  `dispatch_class.rs:110`, `dispatch_helpers.rs:383` (`throw-arm`; `:383` by the `-> VmError` clause).
- **W: none (the population is retired, PM 590).** The five functions the prototype listed
  (`custom_elements/creation.rs:238`, `html_input_value.rs:353`, `natives_promise.rs:565`,
  `script_dispatch_event_core` `:499` / `:513`) have no W row; their calls' uses are Err rows like any
  other. `html_input_value.rs:353`'s legibility rewrite (§7.4) stays.
- **P, from `# pending` (D-3, D-4; PM 590):** `vm/host/custom_elements/upgrade.rs` `invoke_upgrade` →
  `get_property_value` (`:216`) and → `construct_synchronous` (`:277`, via the binding `result`): each `Err`
  arm runs `finalize_failure_shim` and returns `Err`, class P `propagates` (below).

- **`AbortSignal` abort steps (PM, 11-08; L3's `#11-abort-signal-any-dependent-flattening`; corrected per
  L3's attestation N-6, verified against the code).** *At `ce603c74`:* `abort_signal`
  (`vm/host/abort.rs:522-658`) has two `?`: on `dispatch_vm_simple_event` at `:592`, and on its own
  recursion over the composites at `:610`, inside the `for` loop. Neither can fail today:
  `dispatch_vm_simple_event` → `fire_vm_event` → `fire_vm_event_unchecked` → `dispatch_vm_event`
  (`event_target_dispatch_vm.rs:271-347`, no `?` in its body), whose only return is `Ok` (listener exceptions
  are absorbed inside dispatch), and those two `?` are `abort_signal`'s only error sources, so `:610` can
  fail only if `:592` can. Under the carrier-typed closure `abort_signal` reached no producer. Under PM 603's
  reach it does (through the listener calls beneath `dispatch_vm_simple_event`), and
  `dispatch_vm_simple_event` has no ordinary witness (its only return is `Ok`), so the `:592` / `:610` `?`
  are rule 13 members. The shape is **latent definitive**: once P0's error can
  surface through this dispatch (a check or probe on the dispatch path, e.g. a PR-1 walker probe in event
  construction), an `Err` at `:592` would return after the latch and before the composite and fetch
  fan-outs, and one at `:610` after `any_composite_map.remove` (`:607`), so the remaining dependents would
  never abort (C-4). No input reaches it today. *After L3's PR:* DOM §3.2 *signal abort* (`dfn dom
  'signal abort'`, for AbortSignal) step 4 fixes every dependent's reason and steps 5–6 run for all of
  them; the loop runs `run_abort_steps` for every entry, keeps the first `Err`, and returns it after the
  loop, with no `?` inside the loop (L3's contract, unchanged). If the call is then in the population, it is
  a member (its result is stored, not propagated in place) of class **P**, code `propagates` (the stored
  first `Err` is returned after the loop: the binding's sinking use), reason "deferred propagation: the
  first `Err` is returned after every dependent's abort steps ran (DOM *signal abort* steps 4–6); later
  `Err`s of the same trigger are not surfaced separately, one surfacing per trigger suffices (C-3)". While the
  callee stays RL-only, rule 13 makes the row's code `rl-only-pinned` (the cell that fails one dependent's
  abort steps and asserts every other dependent aborted, through P0a-1's `SimpleEventDispatch(n)` injection)
  or `drain-consumed`, and it is the fail-safe for the class: a `?` put back inside that loop is a member. **Which PR writes the row:** L3's PR is expected first (edge, P0 umbrella §1); P0a-1 is
  rebased on it (the seam, P0a-1 §4.4) and P0a-2b's allowlist carries the post-L3 row if the tracked check
  lists the call, keyed on the names as they stand at P0a-2b's base (§11 item 1). If L3's PR lands after
  P0a-2b, P0a-2b ships no row and L3's PR adds it if the check asks for it.

- **The UA-fire drop class (Axis 2 on Rev 3b; PM 603; owners: P0 umbrella §1, PM 608 / PM 609 / PM 615).** A UA-fired
  event's dispatch whose result is dropped with `let _ =`, or converted without a report. Its members are
  P0b-VM-2 §1's list and P0b-VM-1 §5.3's two: the seven named drops (the five `dispatch_vm_simple_event` drops and
  `websocket_dispatch.rs` 114 / 177), the sites P0a-2a §7.1's sample found, and every member the
  classification adds (§6). They are members only under PM 603's reach. At the base `dispatch_vm_event` has
  no `?` and returns only `Ok` (above), so nothing flows. A row's code is `drain-consumed` if the tracked
  check finds every producer path consumed (the listener calls' rows; D-5 is pending → P0b-L3); a member the
  classification finds to have another code (a `thread-entry` N row, for one) keeps it. They turn red the
  moment a probe or a `?` makes the dispatch path carry an `Err`, and each then needs a report (C-3).
  P0b-VM-2 makes the reports, one mechanism for the class, before PR-1's probes (edge P0b-VM-2 → PR-1).

**D-1…D-12 under β (PM 590).** Each judgement re-made on the contract as β states it (P0 umbrella §4):

- **D-1, D-2 — unchanged, D-state.** The listener removal after a failed compile (`ensure_event_handler_current`,
  `event_handler_attrs.rs:548-552`) and the source taken before it (`take_uncompiled`) are writes the spec
  does not make for that failure (it writes the value only at HTML §8.1.8.1 step 3.7.1 or 3.12): C-4.
- **D-3, D-4 — no longer definitive; they leave `# pending`.** On a failure of the `prototype` read or of
  the construct, `invoke_upgrade` runs `finalize_failure_shim` (state Failed) and returns the error. HTML
  §4.13.5 *upgrade an element* sets the state to "failed" at step 3, before the construct, and step 11's
  exception branch keeps a non-"custom" state and rethrows: under β (C-4) that is the branch every failure
  takes, the error surfaces through the callers' reports (class R), and nothing is reverted. Their rows
  become P `propagates` at P0a-2b's landing. Where elidex's Failed differs from the spec's
  "precustomized" (a failure after step 11.2), that is `#11-ce-upgrade-returned-object-typeerror` (L3's CE
  bundle), not P0's.
- **D-5 — stays D, with a different defect.** Under β the `once` removal at `:460` and the prune after the
  call stand (DOM *inner invoke* steps 2.5, 2.13). What remains wrong is that the error is dropped:
  `.is_err()` sets `threw` and nothing reports it (step 2.13.1), at a script-controlled depth, so no S
  admission holds. P0b-L3 reports it (hand-off §4).
- **D-6…D-12 — unchanged.** Each converts the error into a success value or another class (C-2 (a) /
  (b)); none involved a revert. Under β the kind also reaches them when user code of the step ran before
  the failure (no rewrite, P0a-1 §4.2), which widens what their `is_resource_limit()` arm propagates and
  changes no fix.

**`# pending`: 11 classified keys** (+ the unclassified rows until the classification is done, §6; the
ceiling of P0a-2a §5 rule 4 is fixed at this memo's ratification. Restated after PM 544: unchanged,
because D-14 and D-15 are fixed in P0a-1 like D-13, and instrument debt is not pending; restated after PM
590: D-3, D-4 and the two W rows leave, D-16 is fixed in P0a-1.) Keys (path, function, callee, shape):

| Rows | Key | Fixing slice |
|---|---|---|
| D-1 | `vm/host/event_handler_attrs.rs`, `lazy_compile_handler`, `compile_script`, `.ok` | P0b-L3 |
| D-2 | `vm/host/event_handler_attrs.rs`, `lazy_compile_handler`, `run_script`, match | P0b-L3 |
| D-5 | `vm/host/event_target_dispatch.rs`, `invoke_listeners_shared`, `call_value`, `.is_err` | P0b-L3 |
| D-6 | `vm/host/structured_serialize.rs`, `structured_serialize_for_storage`, `stringify_for_structured_shortcut`, match | P0b-VM-1 |
| D-7 | `vm/host/structured_serialize.rs`, `structured_deserialize`, `parse_json_str`, `.unwrap_or` | P0b-VM-1 |
| D-8 | `vm/host/indexeddb/value.rs`, `json_to_js`, `parse_json_str`, `.unwrap_or` | P0b-VM-2 |
| D-9 | `vm/host/service_worker/deliver.rs`, `fire_message`, `parse_json_str`, `.unwrap_or` | P0b-VM-2 |
| D-10 | `vm/host/worker_scope.rs`, `serialize_message`, `stringify_for_structured_shortcut`, match | P0b-VM-1 |
| D-11 | `vm/host/worker_scope.rs`, `native_import_scripts`, `compile_script`, `map_err` | P0b-VM-1 |
| D-12 (2) | `vm/host/subtle_crypto/ops.rs`, `normalize_wrap_with_fallback`, `marshal_normalize`, match (`:66`) and other (`:68`) | P0b-VM-2 |

Each P0b slice deletes its rows here and, where its site stays a member after the fix, adds the
admissible row its memo names **with its rule-12 code**; no slice adds a pending row (the ceiling). A P0b
memo's row whose reason has no code of rule 12 is a design change to P0a-2a (the code table is closed).

### §7.4 Legibility rewrites

When the check cannot follow a row's propagation and the predicate is right, the code is written in a shape
the check follows (one issue, one way: the allowlist does not grow prose to cover analyser gaps). Each
rewrite is behaviour-preserving, named in the PR body with its row, and reviewed as part of this PR.

- **The three P0a-2a §7.2's re-run names (no behaviour change)**: `ops_element.rs:134`
  (`classify_typed_array_string_key`: call the String → Number conversion directly, not the generic
  `to_number`; the row leaves the population); `range_proto.rs:361` and `dom_bridge.rs:492` (pass the closure
  literal at the call, or name the enclosing function's closure parameter per P0a-2a §5 rule 12's
  transitive clause); the BC-3 null arms are already removed by P0a-1.
- Every shape P0a-2a's report names as unfollowed (P0a-2a §11 item 2), and the others the gate run on the
  tree names (§11 item 2).

## §8. Controls

The controls and mutants are P0a-2a §8. This slice adds the gate run on the tree (§4). A red gate on the
tree is fixed by a row, a rewrite or a consumer fix, never by a change to a control.

## §10. Landing obligations

What P0a-2b's PR (or, where marked, the PM / PMO at landing) does. This memo edits no ledger.
Program-level obligations are P0 umbrella §7; the rows this slice triggers there are L-3 (S rows) and L-4
(E rows). Slice-local:

- **L-a2b. Counts and ceiling**: P0a-2b's PR body records the gate's counts per class and per rule-12 code
  at its head; the classification's run (its main commit and command, §6 (a)) and its disagreements with
  §7 (§0); the legibility rewrites of §7.4 with their rows; and the `# pending` keys of §7.3 with the keys
  the classification added.

## §11. Unverified facts (each decided at implementation, before its control is written)

1. The post-L3 function and callee names of the `AbortSignal` abort loop (§7.3), read at P0a-2b's base
   once L3's `#11-abort-signal-any-dependent-flattening` PR has landed, and whether the tracked check lists
   the call.
2. Which rows beyond §7.4's three need a legibility rewrite once the gate runs on the tree.
