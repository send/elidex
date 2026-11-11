# VM stack-safety guard — P0a-1 plan (contract semantics: error identity, the check site, choke-point placement, the engine boundary, the internal-reaction sink)

Status: **plan (pre-implementation)**, per-PR slice of the [P0 umbrella](2026-11-vm-stack-safety-guard-p0.md)
(PM 509; re-sliced from P0a by PM 544; Rev 3: option β, PM 590). `/elidex-plan-review` required before
code. Owner: VM P4. Base: `ce603c74` for every code anchor below except those in `vm/interpreter.rs` /
`vm/interpreter_call.rs`, which are read after the `interpreter.rs` prereq split (discharged by #615) on this
branch's merge of main, and the §5.5 anchors, re-verified on the same merge. Provenance root `H` is the P0
umbrella's. The sibling slices [P0a-2a](2026-11-vm-stack-safety-guard-p0a2a.md) (the census checker) and
[P0a-2b](2026-11-vm-stack-safety-guard-p0a2b.md) (its allowlist and `# pending`) are written against this
memo's contract and land after it (P0 umbrella §1).

The contract C-1…C-5 is the P0 umbrella §4 (canonical); this memo does not restate it. Edges, schedule
and the other slices: P0 umbrella §1. The census rules and the measured population: P0a-2a §5–§7; the allowlist and `# pending`: P0a-2b §6–§7.

## §0. Charter

P0a-1 gives the P0 contract its semantics in the VM: the error identity (§4.1), the one check site and the
placement of the four choke-point checks (§4.2, §4.3), the test-only injection the consumer slices use
(§4.4), the engine boundary's failure type `ScriptFailure` and the trait changes that carry it (§4.5,
PM 533 (a)), and the fixes D-13, B-1, B-2, D-14, D-15, D-16 and BC-1…BC-3 (§5).

**Option β (PM 590, the user's choice).** A probe failure is a throw completion of the step it interrupts.
Every consumer takes the spec's exception branch for that step: the error is consumed as an exception and
reported as a `RangeError` on the trigger's channel. Nothing is reverted and nothing is retried by the
contract. The canonical statement and its authority chain are the P0 umbrella §4 C-4; this memo points
there. What β removed from Rev 2, and why each piece is gone, is §4.2 "What β removed".

**IMP-2 (PM 590: mandatory under any option).** `run_reaction`'s internal subscribers drop a failed handler
call: the async function's wrapper never settles and nothing is reported. D-16 (§5.5) routes the failure
to the owner of the subscription.

### §0.1 Revision history

| Rev | Ruling | Change |
|---|---|---|
| Rev 1 | PM 544 | P0a re-sliced; "did it start" decided by an entered mark set at a list of first-effect sites |
| Rev 2 | PM 557 | the mark set by structure, inside one entry wrapper `enter`; `NotStarted` = the entry's own check failed |
| Rev 3 | PM 590 | option β: no revert anywhere, every probe failure is `Threw`; `NotStarted`, the mark, `enter`, the escape rule and the revert API deleted; IMP-2 fixed as D-16 (§5.5) |
| Rev 3c | PM 603 | the D-16 sink reports when the owner is already final (decided by the gate's result); `fail_step` keeps the stream-entry guard; where the kind ends (a user handler); two L3 intersection notes |
| Rev 3d | PM 608, PM 609 | the census population is P0a-2a's / P0a-2b's and P0a-1's ratification does not wait on it (§1); the P0a-2 split into P0a-2a / P0a-2b repointed; four SINK pair rows (§2); the thenable-assimilation note on `settle_promise` (§5.5) |
| Rev 3e | PM 615 | D-16's spec grounding per owner: where the spec has a destination and where the principle decides (§3, §3.2, §5.5, Axis 4 IMP-1 / IMP-2); NewPromiseReactionJob cited in absolute step numbers; the spec counterparts of the new items (§5.6); a §3 row for *report an exception* |

## §1. Scope

**In.** `crates/script/elidex-js/src/vm/error.rs`, `compiler/mod.rs` (`CompileError` kind),
`vm/resource_limit.rs` (NEW: `check_entry`, the injection), `vm/interpreter.rs` (CP-1: the check first in
`with_call_mode`; `Vm::eval` mapping), `vm/natives_generator.rs` (CP-2: the check first in
`resume_generator`), `vm/host/wasm/exported_func.rs` and `vm/host/wasm/instance.rs` (CP-3a / CP-3b, the
named conversion), `vm/dispatch.rs` and `vm/dispatch_objects.rs` (B-1, B-2 routing); `vm/natives_promise.rs`
(D-15, D-16), `vm/coroutine_types.rs`, `vm/gc/roots.rs`, `vm/gc/trace.rs`, `vm/natives_promise_combinator.rs`,
`vm/host/readable_stream/controller.rs` and `vm/host/readable_stream/mod.rs` (D-16, §5.5); the injection
hooks in `compile_script`, `parse_json_str`, `stringify_for_structured_shortcut` and
`dispatch_vm_simple_event` (§4.4), with the removal of L3's `fail_next_abort_dispatch` seam and the move of
its cell onto that hook (§4.4; L3's files, site list via PM, P0 umbrella §7 L-12); cells in
`crates/script/elidex-js/src/vm/tests/tests_resource_limit.rs` (NEW). For §4.5: in
`crates/script/elidex-script-session/src/`, `outcome.rs` (NEW), `engine.rs` (`ScriptEngine`, `EvalResult`),
`event_dispatch/mod.rs` (`script_dispatch_event_core`: adapts to `call_listener`'s result, no other change),
`lib.rs` (re-export); `crates/script/elidex-js/src/engine.rs` (`eval`, `call_listener`, `drain_timers`);
`crates/dom/elidex-dom-api/src/traversal.rs` (`FilterError`) and its `elidex-js` users
(`vm/host/node_filter_dispatch.rs`, `tree_walker_proto.rs`, `node_iterator_proto.rs`); the shell's
`EvalResult` readers (`lib.rs`, `pipeline.rs`, `content/event_loop.rs`, `content/iframe/render.rs`,
`content/iframe/thread.rs`) and the tests that read `EvalResult`'s `success` field or call `call_listener`.
The session-crate and `elidex-dom-api` files are not VM P4's; the PM routes the notice (P0 umbrella §7 L-14).

**Out.** The census checker with its controls and mutants (P0a-2a); its allowlist and `# pending`
(P0a-2b); the consumer fixes D-1, D-2, D-5…D-12 and the UA-fire drop class (P0b-VM-1, P0b-VM-2, P0b-L3;
P0 umbrella §1); `native_import_scripts`'s kind mapping (D-11, P0b-VM-1) although §4.1 item 3 states the
rule; the deviations of P0 umbrella §6.

**The census population is not this slice's (PM 608).** P0a-1's scope is the contract and the entries;
the population those entries reach is measured by P0a-2a and classified before P0a-2b's review (P0a-2b
§6). P0a-1's ratification does not wait on that classification.

## §2. Coupled invariants

Elements: **ID** (error kinds, predicates, `Vm::eval` boundary; §4.1), **CHK** (the one check site and
the kind's propagation; §4.2), **CP** (choke-point placement; §4.3), **INJ** (test-only injection; §4.4),
**OUT** (the boundary's `ScriptFailure`; §4.5), **B** (B-1 / B-2 routing; §5.2), **SINK** (the
internal-reaction sink; §5.5). The census-side pairs (ID × CEN, CP × CEN, INJ × CEN, OUT × CEN,
SINK × CEN) are P0a-2a §2.

| Pair | Intersection | Pinned by | Basis |
|---|---|---|---|
| ID × CHK | `check_entry` is the one check site for the four entries; the other producers are the `Vm::eval` conversion (§4.1 item 3) and, from PR-1, the direct `ensure_stack` probes (P0 umbrella §4 C-1, P0a-2a §6); nothing rewrites the error, so `is_resource_limit()` holds wherever the error travels until a consumer surfaces it (C-3) — also when user code of the step ran before the failure | `rl_nested_failure_keeps_kind`; M-REWRITE | cell |
| CHK × CP | the check is the entry function's first statement; a failed check returns before anything of the entry ran, so the VM state the entry guards (frames, the generator's Running state, the wasm store) is untouched | `rl_cp1_entry_failure_no_callee_effect`, `rl_cp2_generator_resumable_after_entry_failure`, `rl_cp3*`; M-P-CP1, M-P-CP2, M-P-CP3a/b | cell |
| CHK × I-5 | the check is one call per entry; the `Ok` path gains nothing else (no wrapper, no rewrite) | PR-1 §10's I-5 bench (umbrella I-5) | measured (PR-1) |
| ID × CP | each check returns `VmError::resource_limit()`; CP-3b alone materialises it (`vm_error_to_thrown`), at a C-3 channel | `rl_cp3b_instantiate_entry_failure`, `rl_cp3a_wasm_call_entry_failure_retry` | cell |
| ID × INJ | every injection point delivers exactly C-1's kind (`VmError::resource_limit()` / `CompileErrorKind::ResourceLimit`) | `rl_identity_predicates`, `rl_vm_eval_compile_resource_limit_kind` | cell |
| ID × B | B-1 / B-2 route the error through `throw_error`, which materialises it where the construct is evaluated (ECMA-262 §17 paragraph 1) | `rl_regexp_literal_error_is_catchable`, `rl_computed_accessor_key_error_is_catchable`; M-B1, M-B2 | cell |
| CP × INJ | at P0a-1 the body of `check_entry` is the injection only; `resource_limit::count` counts checks per kind, from which every placement cell takes its `n` | `rl_cp1_*`, `rl_cp2_*`, `rl_cp3*` | cell |
| OUT × ID | a failed step crosses an engine-facing boundary as `Err(ScriptFailure)` after the engine surfaced it; no boundary returns `()` / `bool` / `Option` for a failed step | `rl_eval_result_outcome_kind`, `rl_ua_dispatch_once_listener_entry_failure_consumed` | cell |
| OUT × C-4 | a once listener is removed before the call (DOM *inner invoke* step 2.5) and stays removed on every failure, the probe's included; the error is reported (step 2.13.1) | `rl_ua_dispatch_once_listener_entry_failure_consumed`, `rl_unhandledrejection_once_listener_entry_failure_consumed`, `rl_bound_native_once_listener_consumed_not_rerun` | cell |
| OUT × INJ | the UA dispatch path is reached through `script_dispatch_event` with `Entry(Call, n)`; no new injection point | same cells | cell |
| OUT × D-15 | the `unhandledrejection` loop reports the listener's error like the session loop's engine half | `rl_unhandledrejection_once_listener_entry_failure_consumed`, `rl_unhandledrejection_once_listener_threw_is_removed` | cell |
| SINK × C-3 | an internal subscription names its owner by type; a failed handler call (the step's CP-1, or any other `Err`) settles the owner on its own channel; when the owner's gate reports it already final, the error goes to the report instead (§5.5 "An owner already final"); no arm drops it | `rl_async_step_call_entry_failure_rejects_wrapper`, `rl_combinator_step_entry_failure_rejects_result`, `rl_stream_pull_step_entry_failure_errors_stream`, `rl_combinator_step_failure_after_result_settled_reports`, `rl_stream_step_failure_after_close_reports`; M-SINK, M-SINK-FINAL | cell + structural |
| SINK × CP-2 | an async step fails either at the step's CP-1 (D-16's sink) or at the resume's CP-2 (`drive_async_coroutine`'s `Err` arm); both call one function, so the wrapper's outcome is the same | `rl_async_step_call_entry_failure_rejects_wrapper`, `rl_async_step_entry_failure_rejects_wrapper` | cell |
| SINK × C-4 | the sink takes the exception branch on the owner's channel and leaves the durable facts that branch leaves: the async wrapper rejected and the generator left suspended (its `finally` does not run, P0 umbrella §6 item 5); the stream errored with `pull_in_flight` cleared; the cancel / combinator promise rejected. Nothing is reverted (β) | `rl_async_step_call_entry_failure_rejects_wrapper`, `rl_stream_pull_step_entry_failure_errors_stream`, `rl_combinator_step_entry_failure_rejects_result` | cell (measured by the cells' assertions on the owner's state) |
| SINK × GC trace | the ids in `ReactionTarget` / `ReactionOwner` are traced (`gc/roots.rs`, `gc/trace.rs`, §5.5) | the trace's `match` on the two enums is exhaustive, so a new variant does not compile without an arm | structural for presence; **not pinned** for content: every owner id is also held by its handler object (`AsyncDriverStep { gen }`, the `ReadableStream*Step`s, the combinator's steps and resolving functions; §5.5 "Facts"), so a missing trace of the target's id is masked by that redundant root and no cell can observe it |
| SINK × cfg gate | `ReactionOwner::Stream`, its `fail_reaction_owner` arm and its trace arm are `#[cfg(feature = "engine")]`, like the `readable_stream` module they call (§5.5 "cfg gate") | a mismatch does not compile in a profile without `engine`; the per-feature-profile build matrix (`docs/plans/2026-11-per-profile-build-matrix-umbrella.md`, U0) builds such a profile once it lands | structural; **not pinned in CI** until U0's matrix lands |
| SINK × I-5 | the reaction record carries `ReactionTarget` in place of `Option<ObjectId>`, and the gated `settle_promise` core returns `Settle` on the microtask path; the `Ok` path of `run_reaction` reads the target only in its `Err` arm, and `Settle` is a fieldless two-variant enum returned by value (no allocation) | — | structural; **not pinned**: PR-1 §10's I-5 bench measures the probes, not the reaction record's size or the microtask path, so the record's possible growth (an enum discriminant beside the id) is unmeasured |
| INJ × L3 seam | one test-failure mechanism: `SimpleEventDispatch(n)` replaces L3's `fail_next_abort_dispatch`, and as a `#[cfg(test)]` statement it does not make `dispatch_vm_simple_event` a producer (P0a-2a rule 1) | cell (o) after the move; P0a-2a control `census_cfg_test_hook_not_producer` | cell + check |

## §3. Spec coverage map

No spec defines a resource limit (P0 umbrella §3). The rows are the clauses this slice's code answers to.

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| ECMA-262 §17 Error Handling and Language Extensions | paragraph 1 (errors reported when the construct is evaluated), paragraph 3 (no other kinds of errors as early errors) | a resource limit is neither an early error nor a specified error; it surfaces where the construct is evaluated; B-1 / B-2 errors become catchable where evaluated | `VmErrorKind::ResourceLimit` (NEW), `CompileErrorKind::ResourceLimit` (NEW), `Op::PushConst`, `op_define_computed_accessor` | ✓ | yes |
| WHATWG HTML §8.1.4.3 Creating scripts | *create a classic script* steps 10–11 (ParseScript; a list of errors becomes the error to rethrow) | a compile-time resource limit is not a parse error | `Vm::eval` mapping (`interpreter.rs:36`); `EvalResult::outcome` (BC-1) | ✓ | yes |
| WHATWG DOM §2.9 Dispatching events | *inner invoke* step 2 (listeners whose removed is false), 2.5 (once: remove before the call), 2.13 (call; on a throw, report and continue) | every failure: 2.5 stands, 2.13.1 report | `call_listener` (D-14); `dispatch_unhandled_rejection_event` (D-15) | ✓ | yes |
| WHATWG HTML §8.1.4.7 Unhandled promise rejections | *notify about rejected promises* step 4.1.2 (fire `unhandledrejection` at the global) | the firing is a DOM §2.9 dispatch; D-15's loop is its elidex form (target: the Document, `#11-unhandledrejection-dispatch-target`) | `dispatch_unhandled_rejection_event` (`natives_promise.rs:565`, `:591`) | ✓ | yes |
| WHATWG DOM §6 Traversal | *filter* step 6 (on a throw: unset the active flag, rethrow) | every failure propagates, nothing written | `FilterAction::accept` (BC-3) | ✓ | yes |
| ECMA-262 §27.5.2.1 NewPromiseReactionJob | step 1.e.i (call the handler); 1.f (no capability: 1.f.i asserts the handler's completion is not abrupt); 1.h.i (a capability: an abrupt completion rejects it) | the async driver's reaction has no capability in the spec (Await step 7), so 1.f is spec-exact and a resource limit breaks its assert; the cancel reaction has one in the spec, the promise `cancel()` returns (ReadableStreamCancel step 8, WebIDL *react* step 8), so 1.h.i rejects it, spec-exact; the stream start / pull and combinator reactions have one in the spec (a derived promise elidex omits), so the spec's branch is 1.h.i on a discarded promise, and where each of those failures goes is the principle's choice per owner (§5.5 "The spec's position"): D-16 | `run_reaction` (`natives_promise.rs:665`, call `:689`, `Err` arm `:695-703`) | ✓ | yes |
| WHATWG HTML §8.1.4.6 Runtime script errors | *report an exception* | the report channel (stderr stand-in, P0 umbrella §4 C-3) of `call_listener` (D-14), the `unhandledrejection` loop (D-15) and `fail_reaction_owner`'s `AlreadyFinal` arm (D-16, §5.5 "An owner already final") | `call_listener` (`engine.rs:275`), `dispatch_unhandled_rejection_event` (`natives_promise.rs:591`), `fail_reaction_owner` (NEW) | ✓ | no |

**Breadth**: K=3 specs (ECMA-262, WHATWG HTML, WHATWG DOM), M=7 entries (verified 2026-11-10, Rev 3e, by `python3
.claude/skills/elidex-plan-review/preflight.py` on this memo: rc 0, K=3, M=7; rc and K / M as printed in the
PR body, P0 umbrella §7 L-13). Below the split thresholds.

### §3.1 User-input touch audit

Every placement is reachable from script: the caller's stack depth decides whether the check fails.
Adjacent pre-existing code: `Op::PushConst` propagates `load_constant`'s error with `?` out of `run()`, so
an invalid regexp literal's error is not catchable by the script that evaluates it (B-1) — exposure rises
when PR-1's front door adds a `RangeError` to that site, so P0a-1 routes it. The computed accessor key of
`Op::DefineComputedGetter` / `Op::DefineComputedSetter` is the same shape (B-2): a key whose `toString`
throws escapes `run()` today. D-15: `node.dispatchEvent(e)` (native) → `invoke_listeners_shared` with
`drain_after_each` → `drain_microtasks` → `process_pending_rejections` (`natives_promise.rs:291`) →
`dispatch_unhandled_rejection_event`, so the `unhandledrejection` loop runs at a script-controlled depth
(R4 Axis 2 IMP-2; the reentrancy guard does not stop it, `microtask_drain_depth` is 0 under `vm.eval`).
D-16: the same `drain_microtasks` path runs `run_reaction` for an async function's await step, so the
step's CP-1 meets a script-controlled depth (Rev 2 Axis 2 IMP-2).

### §3.2 Cited, not touched

The D-16 sinks deliver to algorithms that already exist; their steps are not changed. Per owner, where
the spec sends a failed reaction step, and where the spec stops and the elidex principle decides (§5.5
"The spec's position"):

- **Async driver.** ECMA-262 Await step 7 calls PerformPromiseThen with no capability, so
  NewPromiseReactionJob step 1.f applies and its assert (1.f.i) says the step never fails: the spec has no
  destination. AsyncBlockStart step 1.h (a throw completion rejects the async function's promise) is the
  existing channel the sink uses.
- **Cancel.** WHATWG Streams §4.9.2 *ReadableStreamCancel* step 8 returns the result of WebIDL §3.2.24.1
  *react* to the source's cancel promise, which is that reaction's capability (react steps 6–8): a failed
  step rejects the promise `cancel()` returned. Spec-exact.
- **Start and pull.** Streams §4.9.4 *SetUpReadableStreamDefaultController* steps 11–12 and
  *ReadableStreamDefaultControllerCallPullIfNeeded* steps 7–8 react to the start / pull promise; the
  capability *react* creates (steps 6–8) is discarded (react's Note allows omitting it, as elidex does), so
  in the spec a failed step rejects a promise nobody reads. Erroring the stream is the principle's choice
  (C-3, fail-safe), through the existing *ReadableStreamDefaultControllerError* (step 2: a stream that is
  not "readable" is left as it is).
- **Combinators.** ECMA-262 PerformPromiseAll step 5.k (PerformPromiseAllSettled 5.p, PerformPromiseAny
  5.k, PerformPromiseRace 1.d) Invokes `then` on each element, and Promise.prototype.then step 4 creates
  the derived capability the element's reaction carries; elidex's combinator `subscribe` omits it. In the
  spec a failed element step rejects that derived promise (NewPromiseReactionJob step 1.h.i) and the
  result promise stays pending. Rejecting `result` is the principle's choice (P0 umbrella §4 C-4
  authority (2), fail-safe), through the result capability's existing reject channel (`Promise.all` step
  8.b's IfAbruptRejectPromise is that channel; it is not the spec's branch for this failure).

## §4. Types and seams

### §4.1 Error identity

1. `VmErrorKind::ResourceLimit` (NEW), built by `VmError::resource_limit` (NEW) with message
   `STACK_EXHAUSTED_MSG` (NEW, `vm/error.rs`). `vm_error_to_thrown` materialises it as a `RangeError`
   object (`name` "RangeError", `message` the constant); `Display` prints `RangeError: <message>`.
   `VmError::is_resource_limit` (NEW) is true for that kind only.
2. `CompileError` gains `kind: CompileErrorKind` (NEW) with variants `Syntax` and `ResourceLimit`;
   `CompileError::is_resource_limit` (NEW). Every current construction site is `Syntax`.
3. Boundaries keep the kind: `Vm::eval` (`interpreter.rs:36`) maps `CompileErrorKind::ResourceLimit` to
   `VmError::resource_limit()` and `Syntax` to today's `VmErrorKind::CompileError` (D-13, §5.1). The same
   rule binds `native_import_scripts` (`worker_scope.rs:481`), implemented by P0b-VM-1 as D-11.
   L3's brand-check program also changes this native: whichever lands first, the later PR adapts to the
   earlier one's form (L3 brand-check Rev 4).
4. **Compile-channel aggregation (rule here, implementation in PR-1).** `compile_script` reports
   `CompileErrorKind::ResourceLimit` when any parse or scope error is a probe failure, whatever else the
   lists hold: a parse that hit the probe is not a verdict about the source. PR-1 adds
   `JsParseErrorKind::StackExhausted` (NEW: PR-1) and the one arm in `compile_script`, with its cells
   (PR-1 §8 `frontend_*`, `parser_*`). **Existing caps are not probe failures (Rev 2 A1 MIN-4 / A2 MIN-2).**
   The parser already has `JsParseErrorKind::ResourceLimit` (`error.rs:58`: the 4 GiB source limit,
   `parser/mod.rs:120`; the AST node limit, `:149`) and `JsParseErrorKind::NestingTooDeep` (`error.rs:56`;
   `MAX_NESTING_DEPTH`, `parser/mod.rs:475`). Each is a cap determined by the input (D-P0-6): the same
   source fails the same way at any depth. They map to `CompileErrorKind::Syntax`, as every construction
   does today; only `StackExhausted` maps to `CompileErrorKind::ResourceLimit`. The shared word in
   `JsParseErrorKind::ResourceLimit` is a name, not the kind of C-1.

The compile-time channel has one consumer of the front end's error lists: `compile_script`
(`compiler/mod.rs:142-155`) is the only production caller of `parse_script` / `analyze_scopes`
(`git grep -nE '(parse_script|analyze_scopes)\(' -- crates` minus tests: `compiler/mod.rs` lines
143/149 and the definitions `lib.rs:91/104`). `compile_script` itself has 3 production callers
(`git grep -n 'compile_script(' -- crates` minus tests: `interpreter.rs:36`,
`event_handler_attrs.rs:677`, `worker_scope.rs:481`), all three in the census (D-13 here; D-1 P0b-L3;
D-11 P0b-VM-1).

### §4.2 The check site, and the kind's propagation (Rev 3, PM 590)

**The check site.** `crate::vm::resource_limit::check_entry(kind: EntryKind) -> Result<(), VmError>`
(NEW, `pub(crate)`) is the one check site for the four entries: the injection of §4.4 at P0a-1, the probe
that PR-1 / PR-1b add, and the one place where the ask per kind is chosen. It is not the only producer: the
`Vm::eval` conversion (§4.1 item 3) and PR-1's direct `ensure_stack` probes also produce the kind (P0
umbrella §4 C-1, P0a-2a §6). Each of the four entries of §4.3 calls it as its first
statement and propagates its error with `?` (CP-3b converts it, §4.3). There is no wrapper around the
entry and no rewrite of the entry's result.

**The kind propagates unrewritten.** A `ResourceLimit` keeps its kind until a consumer surfaces it (C-3):
`throw_error` materialises it for the script's handlers where the construct is evaluated (`vm/ops.rs:155`,
unchanged), a rejection materialises it with `vm_error_to_thrown`, and an engine boundary reports it
(§4.5). This holds whether or not user code of the interrupted step ran before the failure: a nested
entry's failure inside a native body (a bound `forEach`'s second callback, a wasm argument's `valueOf`, a
getter reached from `Object.assign`) reaches the native's caller as `ResourceLimit`. So a C-2 consumer
(P0b) recognises every probe failure by `is_resource_limit()` and never mistakes one for a script's own
`RangeError` (D-P0-1).

**Where the kind ends (Axis 2 MIN-1 on Rev 3b).** Entering a user handler is the C-3 surfacing. When
`throw_error` hands the materialised `RangeError` to a `catch` or a `finally`, the script has seen it, and
the kind ends there. A `catch` that rethrows, or a `finally` that completes and re-raises the pending
exception (`Op::Throw`'s uncaught path and `Op::EndFinally`'s `ThrowUncaught` in `vm/dispatch.rs`), raises
a `ThrowValue` of that `RangeError`, not a `ResourceLimit`. This is the script's own exception from then on,
and every consumer takes its script-throw branch. `rl_nested_failure_keeps_kind` deliberately has no
`finally`, so it pins the kind up to the first handler and no further. No consumer's **spec** branch for
a script throw is catch-to-default. Measured on the census's converting rows (`awk -F'\t' '$1 ~ /^(D|E|S)/'
H/logs/err-census-p0-classified.tsv`, 28 rows), the callees that can run user code are:
`get_property_value` / `construct_synchronous` (D-3, D-4: the upgrade's exception branch rethrows);
`call_value` (D-5: report); `run_script` (D-2: report); `marshal_normalize` (D-12: WebCrypto §14.3.11
*wrapKey* step 3 normalises again with op "encrypt", and step 4 rejects on a second error, so it retries and
does not default); and `stringify_for_structured_shortcut` (D-6, D-10: its getters and `toJSON`). The E rows
keep the original completion (IteratorClose), and the S rows drop without converting. One **code** branch
does default a script throw: D-6 (`structured_serialize.rs:78-85`) maps a user exception to `None`, where
HTML §7.2.5 *shared history push/replace state steps* step 3 rethrows. That is D-6's existing deviation
(P0b-VM-1 gives it the resource-limit arm; the script-throw half is
`#11-history-state-structured-serialize-fidelity`'s), not a consequence of the kind ending.

**What β removed (each with what read it).**

- **`ScriptFailure::NotStarted`.** Its one reader was the revert (§4.5 Rev 2: `reinsert` on `NotStarted`,
  D-14, D-15, D-5; `abandon_upgrade`, D-4). Under β no consumer distinguishes "the entry's own check
  failed" from "the step threw", so the variant is dead. `ScriptFailure` stays as the failure half of the
  engine boundary, without variants (§4.5, with what reads it).
- **The entered mark.** It existed to make `NotStarted` exact. No reader remains.
- **`enter(cx, kind, body)`, `EntryError` and the exit rewrite (the escape rule).** The rewrite turned a
  body's `ResourceLimit` into a thrown `RangeError` so that only the entry's own check could read as
  `NotStarted`. With no `NotStarted`, the rewrite only hid the kind from C-2 consumers, which is the unsafe
  side (a probe failure after user code ran would have been taken as the script's own exception, e.g. by a
  catch-to-null branch). `check_entry` is the single check site and needs no wrapper: the placement
  (§4.3) is a statement order, pinned by the placement cells and mutants (§6, §7). The `JsValue` conversion
  that `EntryError` carried is CP-3b's named conversion at its call (§4.3).
- **The "more than one entry per step" analysis** (Rev 2 A2 IMP-1). It mapped entries to steps for the
  revert. No per-step mapping remains: every failure takes the step's exception branch, whichever entry
  failed and however many the step performed (`accept`'s getter, call and `valueOf`; the CE upgrade's
  `prototype` read and construct).
- **The opt-in to a finer `NotStarted`** (Rev 2, PM 557 item 4). There is nothing to refine.
- **The native-callback path table** (Rev 2 §4.6). It seeded cells that pinned the mark on each class of
  native re-entry. Every class now has one outcome, pinned once by `rl_nested_failure_keeps_kind` and
  `rl_bound_native_once_listener_consumed_not_rerun`. The Promise-job facts it held are §5.5's.

**Where an exact `ResourceLimit` is observed (re-derived under β).**

- CP-1: the caller of `call` / `call_construct_native` / `run_function` / `construct_synchronous`. An engine
  boundary reports it and returns `Err(ScriptFailure)` (§4.5); `run_reaction` hands it to the reaction's
  target (§5.5); a native body propagates it with `?` to its own caller; `run()` routes it through
  `throw_error`, which materialises it for a handler and, when none catches it, returns the original `Err`
  with its kind (`vm/ops.rs:155-165`), so the top of an uncaught evaluation reads `is_resource_limit()`.
- CP-2: the three generator natives (the script sees a catchable `RangeError`; the generator stays
  suspended and the next `.next()` resumes it, `rl_cp2_generator_resumable_after_entry_failure`), and
  `drive_async_coroutine` (`natives_generator.rs:520-522`: it materialises the error and rejects the
  wrapper — the async function's own channel, C-3; P0 umbrella §6 item 5). The initial drive
  (`make_async_coroutine_and_drive`, the async start) and a later step both reach this arm when the resume's
  CP-2 fails. **A later step meets CP-1 first** (Rev 2 A2 IMP-2): the reaction job calls the
  `AsyncDriverStep` handler through `vm.call` (`natives_promise.rs:689`), and `call_dispatch`'s
  `AsyncDriverStep` arm (`interpreter_call.rs:183-190`) runs only after that check passed. A failure of
  that CP-1 goes to the subscription's owner, the same wrapper rejection (§5.5).
- CP-3a / CP-3b: `call_dispatch`'s `WasmExportedFunction` arm (`interpreter_call.rs:248`) inside an
  enclosing CP-1 entry or `run()` (the script sees a `RangeError`), and the two `instantiate_module`
  consumers (the materialised `RangeError` is thrown or becomes a rejection, §4.3).

**The async start.** The call's own CP-1 check passes; `call_internal`'s async branch
(`interpreter_call.rs:367-397`) builds the initial frame and the wrapper promise without pushing a frame;
`make_async_coroutine_and_drive` → `drive_async_coroutine` → `resume_generator`, whose check fails (CP-2).
`drive_async_coroutine` rejects the wrapper with the materialised `RangeError` and returns `Ok`; the call
returns `Ok(wrapper)`. This is the spec's shape for an async function whose body completes abruptly:
ECMA-262 EvaluateAsyncFunctionBody step 5 returns the promise in every case, AsyncBlockStart step 1.h
rejects it on a throw completion, and even a failure of FunctionDeclarationInstantiation rejects it
(EvaluateAsyncFunctionBody step 3). At `call_listener` the outcome is `Ok(())`, the once removal stands,
and the `RangeError` reaches HTML *notify about rejected promises* (D-15's loop, §5.4), then the stderr
stand-in if unhandled. The inline dispatch path (`interpreter_call.rs:631`) has the same `Ok` from
`make_async_coroutine_and_drive`; its `Err(_e)` arm stays unreachable (`drive_async_coroutine` returns
`Err` only for a generator without a wrapper, which that path never creates) and is a row of the census allowlist
(P0a-2b §7.3).

**Cost (I-5).** One call per entry, no field, no store, no rewrite. PR-1 §10's I-5 bench covers
`with_call_mode` and the call path; no separate measurement.

### §4.3 The choke-point check: placement (I-3, VM half)

`check_entry` (§4.2) takes an `EntryKind` (NEW: `Call`, `Resume`, `WasmCall`, `WasmInstantiate`). P0a-1
places it; PR-1 adds `ensure_stack(ask)` for `Call` / `Resume` inside it, PR-1b for `WasmCall` /
`WasmInstantiate`. At P0a-1 its body is the `#[cfg(test)]` injection of §4.4 only.

| Id | Site | Placement | What must not have happened yet | Why it holds |
|---|---|---|---|---|
| CP-1 | `VmInner::with_call_mode` (`interpreter.rs:160`) | first statement, before the snapshot and `f(self, mode)` | the callee's first effect | `with_call_mode` restores frames, value stack, `gc_enabled`, `active_bound_key` and `completion_value` on every exit (`interpreter.rs:166-276`), so any position before `f(self, mode)` is equivalent; the check must precede `f` |
| CP-2 | `VmInner::resume_generator` (`natives_generator.rs:114`) | first statement | the generator taken and marked Running | a Running generator rejects the next resume ("already running") |
| CP-3a | `call_wasm_exported_function` (`host/wasm/exported_func.rs:268`) | after the storage lookup and the argument coercions (`:284-289`, nested CP-1 entries, each checked), before the `HostState` bind and `set_fuel` | store / instance mutation | PR-1b §2 |
| CP-3b | `instantiate_module` (`host/wasm/instance.rs:151`) | first statement, before the lazy `vm_wasm_runtime()` init, `Store::new` and the start function | runtime init, store creation, start-function effects | PR-1b §2 |

Entry functions (production callers of `with_call_mode`, `git grep -n 'with_call_mode(' --
crates/script/elidex-js/src` minus tests: `interpreter.rs` 82 / 102 / 298, `dispatch_class.rs:323`; the
`with_call_mode` tests moved to `vm/tests/tests_with_call_mode.rs` with the split): `call`
(`interpreter.rs:76`), `call_construct_native` (`:95`), `run_function` (`:292`), `construct_synchronous`
(`dispatch_class.rs:273`, `with_call_mode` at `:323`) and `dispatch_super_inner` mutate nothing before
`with_call_mode` (R1 Axis 2, verified at `47b6c934`; re-checked at P0a-1's base by
`rl_cp1_entry_failure_no_callee_effect`, whose body enters through each of the first four).

**CP-3 conversions (named).** `call_wasm_exported_function` returns `Result<_, VmError>` and propagates the
check with `?`. `instantiate_module` returns `Result<ObjectId, JsValue>`; the check's error converts by
`.map_err(|e| ctx.vm.vm_error_to_thrown(&e))?`, the in-file precedent (`instance.rs:162-163`,
`coerce_import_object`), never through `wasm_error_to_js_value` (which would make it
`WebAssembly.RuntimeError`). Its two consumers propagate (`instance.rs:69`) or reject (`namespace.rs:578`);
both are C-3 channels, so the materialised form is admissible there.

### §4.4 Test-only injection

`resource_limit::inject` (NEW: test-only) arms a thread-local: `InjectAt::Entry(EntryKind, n)` makes
the n-th `check_entry` of that kind on this thread return `VmError::resource_limit()`;
`InjectAt::Compile(n)`, `InjectAt::JsonParse(n)`, `InjectAt::JsonStringify(n)` and
`InjectAt::SimpleEventDispatch(n)` do the same at the entry of `compile_script`, `parse_json_str`,
`stringify_for_structured_shortcut` and `dispatch_vm_simple_event` (`event_target_dispatch_vm.rs:653`).
`resource_limit::count` (NEW: test-only) records how many checks of a kind a body performs, so a cell
derives its `n` from a dry run instead of guessing. The four boundary points are not production probe
sites: they let a cell deliver C-1's kind to a consumer through the consumer's real call path before PR-1's
walker and front-end probes exist, and they stay after PR-1 so the consumer cells do not depend on stack
sizes. Each boundary hook is a `#[cfg(test)]`-attributed statement (P0a-2a rule 1 excludes such statements
from producer derivation). They are the only injection points; a consumer cell in any slice that needs
another is a design change to this section. The P0b slices use them as they are.

**One test-failure seam (one issue, one way; ratified PM 513; canonical home of the seam's order, PM
538).** L3's `#11-abort-signal-any-dependent-flattening` PR (PR-A, around 11-10; its plan Rev 2.1 §6 (o) and
§9 Edges) ships a test-only field `VmInner::fail_next_abort_dispatch: bool` for its cell (o): when set, the
next abort dispatch returns `Err` in place of the `dispatch_vm_simple_event` call and the field is cleared,
so the cell can show that every dependent's abort steps still run. That is a second test-failure mechanism
beside this one, temporary by plan. **Expected order (PR-A first, then P0a-1 rebased on it):** P0a-1
deletes `fail_next_abort_dispatch` and rewrites cell (o) to arm `InjectAt::SimpleEventDispatch(n)` (the
general form, at the same call) with `n` from `resource_limit::count`, same assertion; its error is C-1's
kind, which is also the only `Err` that can reach this call once P0's error exists (P0a-2b §7.3). **Inverted
order (P0a-1 first):** there is no seam to move; PR-A arms `InjectAt::SimpleEventDispatch(n)` from the
start and adds no field. Either way L3's contract is unchanged, and after both PRs exactly one seam exists.
P0 umbrella §1 points here.

### §4.5 The engine boundary carries the failure (PM 533 (a); re-derived under β)

**Defect (R3 Axis 1 IMP-1, verified at `02386ec9`).** `ScriptEngine::call_listener` returns `()`; the VM's
implementation drops the call's result (`crates/script/elidex-js/src/engine.rs:275`, `let _ = g.call(..)`),
so a listener's error — a probe failure included — is neither reported nor visible to the caller across the
boundary. This is **D-14** (§5.3). Under Rev 2 the caller also needed the kind, to revert a `once`
removal; under β it does not (C-4), and the defect is the drop.

**The type.** `elidex_script_session::ScriptFailure` (NEW: P0a-1, `crates/script/elidex-script-session/src/outcome.rs`,
re-exported at the crate root) is a unit struct: *the step failed, and the engine has already surfaced the
error on the trigger's channel (C-3).* It is the failure half only: each boundary keeps its own success
type (`()`, `NodeFilterResult`, a compile verdict) and returns `Result<T, ScriptFailure>`. It carries no
value (a JS value cannot cross into an engine-independent crate) and, under β, no kind: no consumer across
the boundary reads one. **What reads it** (the reason it stays without the revert): (1) the shell's
`EvalResult` readers, which read whether the script failed (today's `success`); (2) the traversal protos,
whose `FilterError::Script` arm takes `pending_error` and propagates it (BC-3 below); (3) P0b-L3's
session-side handler completion (PM 533 (iii), hand-off §4), which chooses the no-write arm on `Err`; (4)
P0a-2a's rule 11, which recognises a boundary that carries the failure, so a collapsing boundary stays a
census member. `call_listener`'s caller reads it as "surfaced; step 2.5 stands". **Home**:
`elidex-script-session`, the sole Script↔ECS boundary (CLAUDE.md, Design philosophy); every crate that names
it already depends on it (`elidex-dom-api` → `elidex-script-session`, `Cargo.toml:16`; the shell;
`elidex-js`). The engine side builds it from any error; there is no mapping function and no predicate on
the boundary.

**Enumeration (every method of a trait declared outside `elidex-js` and implemented in it; census B1,
P0a-2a §7).**

| Boundary (declaration; impl) | Today | P0a-1 |
|---|---|---|
| `ScriptEngine::call_listener` (session `engine.rs:78`; impl `elidex-js/src/engine.rs:203`) | `()`; the impl drops the result (`:275`) | `-> Result<(), ScriptFailure>`, `#[must_use]`; the impl reports the error; D-14 (§5.3) |
| `ScriptEngine::eval` (`:59`; impl `:176`) | `EvalResult { success: bool, error: Option<String> }` | `EvalResult { outcome: Result<(), ScriptFailure>, error: Option<String> }`; `success` is removed, no shim; BC-1 |
| `ScriptEngine::drain_timers` (`:101`; impl `:332`) | `Vec<EvalResult>` | each `EvalResult` carries the outcome (BC-1's type); the container carries the failure by containment (P0a-2a rule 11); BC-2 |
| `FilterAction::accept` (`crates/dom/elidex-dom-api/src/traversal.rs:401`; impl `vm/host/node_filter_dispatch.rs:51`) | `Err(FilterError::Throw)`; the value travels out of band in `JsFilter`'s `pending_error` field | `FilterError::Throw` → `FilterError::Script(ScriptFailure)` (NEW: P0a-1); the value stays in `pending_error`; BC-3, with the null arms removed (below) |
| `ScriptEngine`: `run_microtasks`, `drain_reactions`; `HostDriver`: `deliver_mutation_records`, `deliver_resize_observations`, `deliver_intersection_observations`, `tick_network`, `deliver_sw_client_update`, `deliver_idb_versionchange`, `drain_worker_messages`, `deliver_history_step_events`, `deliver_media_query_changes`, `deliver_visual_viewport_events` (12) | `()` | unchanged; class R "drain": the method has no step of its own, each item's failure is consumed on that item's own channel inside the engine at a site that is its own Err-population row, and no caller holds a subject across the call; **the consuming rows are named per drain and checked** (PM 540 (3); P0a-2a §5 reason `drain-consumed`) |
| `HostDriver::set_history_state` (`:469`; impl `:605`) | `()` | unchanged; **not a drain** (R4 Axis 1 MIN-3): it has a step of its own (`seed_history_state` → `structured_deserialize` → `parse_json_str`). Class N top-only, the reason P0b-VM-1 §5.1 gives its inner chain: the embedder's document-construction entry, HTML §7.4.6.2 *restore the history object state* step 2's catch-null is the spec branch; the consuming row the checker names is `structured_deserialize` → `parse_json_str` (D-7 until P0b-VM-1, then the N rows `seed_history_state` → `structured_deserialize`) |
| `HostDriver::bind`, `unbind`, `teardown_document`, `seed_sw_client`, `take_pending_storage_changes`, `sw_controller_scope`, `storage_origin_key`, `install_network_handle`, `install_idb_backend` (9) | `()` / a value | unchanged; class N "name-graph collision": no script runs; the name graph reaches a producer only through a colliding name (`new`, `to_string`, `bind`); the cut is keyed per call occurrence (P0a-2a rule 10) |
| `HostDriver::with_bound` | `R`, the closure's value | unchanged; class P "pass-through": it returns the closure's own value |
| `ScriptEngine`'s `remove_listener`, `bound_dom_mut`; the other 32 `HostDriver` methods; `MutationDispatcher::dispatch` (`Send + Sync`, no VM); the std-trait impls (`Drop`, `Iterator`, `Default`, …) | — | members or not as P0a-2a's resolved graph decides (rule 9 scans every foreign trait, std included); none runs script (R4 Axis 1 MIN-2 read the 7 std impls at the base) |

No other implementor exists in non-test code (`git grep -nE 'impl(<[^>]*>)? (\w+::)*(ScriptEngine|HostDriver) for' -- crates`
→ `elidex-js/src/engine.rs:175`, `:373` only, verified 2026-11-08). Test implementors and test callers adapt
in P0a-1's PR (the tests of `elidex-js` that read `EvalResult`'s `success` field or ignore `call_listener`).
**Order with `#11-event-path-window-missing` (PM 540 (2)):** P0a-1 lands first; that umbrella's PR-A (the
mock `ScriptEngine` in `event_dispatch/test_support.rs` and its edits to `script_dispatch_event_core`) is
written against this signature (`call_listener -> Result<(), ScriptFailure>`); under β the session loop
does nothing on `Err` beyond what it does on `Ok`, so PR-A's edits carry no revert. The boundary's type is
the contract, so the later PR adapts, not the contract. If P0a-1 is late for PR-A's schedule, the PM
decides (§9 item 5).

**The engine side.** `call_listener` matches the call's result: `Ok` → `Ok(())`; an error → it is reported
(the stderr stand-in of P0 umbrella §4 C-3, as `observer_common.rs` does; DOM §2.9 *inner invoke* step
2.13.1) and `Err(ScriptFailure)` is returned. A listener id that no longer resolves stays `Ok(())` (step 2,
"whose removed is false"). `eval` and `drain_timers` build `outcome` the same way. After P0b-L3, a failure
of `ensure_event_handler_current` (`engine.rs:234`) is reported and returns `Err(ScriptFailure)` without
calling the listener (hand-off §4).

**The engine-independent side (C-4 under β).** `script_dispatch_event_core`
(`crates/script/elidex-script-session/src/event_dispatch/mod.rs:493-518`) removes a `once` entry at step 2.5
(`:499`), calls `call_listener` (`:503`), runs the microtask checkpoint (`:513`) and the engine-side
`remove_listener` (`:517`) whatever the result. That is the spec's order: the removal precedes the call,
and a call that throws — for any reason, a probe failure included — leaves the listener removed and is
reported (DOM §2.9 *inner invoke* steps 2.5, 2.13). The loop's only change is consuming the `#[must_use]`
result. Nothing is reinserted and the listener is not invoked again by the contract (P0 umbrella §4 C-4).

**BC-3's failure → null arms (R4 Axis 2 MIN-2).** At the base `tree_walker_proto.rs:377` and
`node_iterator_proto.rs:299` match `Ok(None) | Err(FilterError::Throw) => Ok(JsValue::Null)`; the arm is
reachable only because every `FilterError::Throw` construction (`node_filter_dispatch.rs:51-98`) sets
`pending_error` first and the proto returns that error before the match (`tree_walker_proto.rs:340-347`).
A failure → null arm that is unreachable by an invariant is the C-2 (a) shape guarded by review, and under
P0a-2a's checked-reason rule "unreachable by construction" has no mechanical form (it would be a D row). So
P0a-1 removes the arm: the `Err(FilterError::Script(ScriptFailure))` arm **propagates** — it takes
`pending_error` and returns `Err(value)`; with no pending value it returns an internal error (never null; a
debug assertion names the breach). No error is rebuilt from the boundary type (Rev 2 A1 MIN-3: the second
inverse mapping is gone with `NotStarted`). The **`pending_error` invariant**, stated once: `accept`
returns `Err(FilterError::Script(_))` only after `pending_error = Some(value)`; the proto consumes
`pending_error` in the `Script` arm of the traversal result and nowhere else. After this the two sites are
auto-recognised propagation (P0a-2a rule 2) or P rows, not N. `pending_error` holds the `VmError`
(R1 Axis 2), so the kind reaches the script's `throw_error` unchanged.

**Rejected (PM 533).** (b) Removing the `once` entry after the call: DOM §2.9 *inner invoke* removes before
the call (step 2.5 before step 2.13). (c) A top-only N row for `call_listener`: the dropping boundary is the
defect itself; a row that watches it leaves the drop in place.

## §5. Fixes

### §5.1 D-13: `Vm::eval`

| Id | Site | Carrier | What the error becomes today | Class | Governing step |
|---|---|---|---|---|---|
| D-13 | `vm/interpreter.rs:36` `Vm::eval` | `compile_script` | `VmErrorKind::CompileError` (event-loop top; identity only) | D-cls | HTML §8.1.4.3 steps 10–11 |

Fix: §4.1 item 3.

### §5.2 B-1, B-2: errors that leave `run()` without `throw_error`

| Id | Site | Carrier | Today | Governing step |
|---|---|---|---|---|
| B-1 | `vm/dispatch.rs:91` `Op::PushConst` | `load_constant` | `?` out of `run()`: not catchable by the script that evaluates the literal | ECMA-262 §17 paragraph 1 |
| B-2 | `vm/dispatch_objects.rs:227` `op_define_computed_accessor` (`dispatch.rs:842/845`) | `make_property_key` | same | ECMA-262 §17 paragraph 1 |

Fix: `Op::PushConst` matches `load_constant`'s result, and `op_define_computed_accessor` matches
`make_property_key`'s, and each sends an error through `throw_error`, like every other fallible op in
`run()`. These are catchability fixes. P0a-2a rule 6 (a `?` on a carrier in `run()` is a member) stays as
the census-side guard of catchability.

**B-1's test vehicle (R5 Axis 1 MIN-3).** [spec] A regexp literal whose pattern is invalid is an early
error (ECMA-262 §13.2.7.1 Static Semantics: Early Errors, via §22.2.1.1); its evaluation never throws
(§13.2.7.3 step 3 is `! RegExpCreate`). elidex compiles regexp literals lazily and surfaces the failure at
run time as a `TypeError` (`dispatch_helpers.rs:168-171`): the registered deviation
`#11-vm-regexp-syntax-error-surface` (VM P4), not this slice's. Before PR-1 no spec-sanctioned run-time
error exists at `Op::PushConst` — the only error `load_constant` can raise there is that deviation's — so
the cell `rl_regexp_literal_error_is_catchable` keeps the vehicle `/a{2,1}/` and asserts **only that the
error is caught** (B-1's subject); it does not assert the error's class, so it pins nothing of the
deviation. PR-1's front door adds the first spec-admissible run-time error at the site (its `RangeError`),
and PR-1's regexp cells pin that class there.

### §5.3 D-14 and BC-1…BC-3: the engine boundary (PM 533 (a))

| Id | Site | What the error becomes today | Class | Governing step |
|---|---|---|---|---|
| D-14 | `ScriptEngine::call_listener` (impl `engine.rs:203`, result dropped at `:275`) | dropped, no report; the boundary returns `()` | D (an S row whose top-only does not hold: the session dispatch is reachable at a nested depth, PM 533 (c)) | DOM §2.9 *inner invoke* step 2.13.1 |
| BC-1 | `ScriptEngine::eval` (impl `engine.rs:176`) | a string in `EvalResult`'s `error` field and a `bool` | boundary collapse; the shell's readers are event-loop top, N | HTML §8.1.4.3 steps 10–11 (via D-13) |
| BC-2 | `ScriptEngine::drain_timers` (impl `engine.rs:332`) | same, per timer | same | — |
| BC-3 | `FilterAction::accept` (impl `node_filter_dispatch.rs:51`) | `FilterError::Throw`; the value is in `pending_error`; the protos' null arms rest on an unstated invariant | same; the traversal algorithms propagate with `?` and write nothing (DOM §6 *filter* step 6: unset the active flag, rethrow) | DOM §6 *filter* step 6 |

Fix: §4.5. D-14 is fixed here (the trait change is P0a-1's, PM 533 (a)), so it is never pending; BC-1…BC-3
carry the failure and leave the boundary population. The S row `engine.rs:275` of the census at
`ce603c74` was D-14's engine half (PM 533 item 3; P0a-2b §7.3).

### §5.4 D-15: the `unhandledrejection` loop (R4 Axis 2 IMP-2; PM 544)

| Id | Site | What the error becomes today | Class | Governing step |
|---|---|---|---|---|
| D-15 | `vm/natives_promise.rs` `dispatch_unhandled_rejection_event`: the `once` removal from the Document's `EventListeners` at `:565` (`listeners.remove(entry.id)`), the call at `:591` (`let _ = g.call(..)`), the store prune after the call (`remove_listener`, `:617-621`) | dropped, no report | D: an S row whose top-only was false (§3.1: reached from `dispatchEvent` at a script-controlled depth) | DOM §2.9 *inner invoke* steps 2.5, 2.13; HTML §8.1.4.7 *notify about rejected promises* step 4.1.2 |

Fix: the loop matches the call's result; on an error it reports it (the stderr stand-in, C-3) and continues
with the next listener; the removal at `:565` and the prune at `:617-621` stand on every outcome (the
spec's order, as in §4.5). P0 umbrella §6 item 2's S row of this site becomes R. **Layering note:** this
loop is VM core writing an ECS component directly (the third *inner invoke* implementation); it is the
registered deviation `#11-event-dispatch-inner-invoke-dual-implementation` (VM P4, re-eval 2026-12-15; the
slot counts three loops, PM 550), not a new design here. Under β the three loops share no revert decision;
what they share is the spec's order and the report.

**Every site of D-14's shape (PM 544).** Concept sweep, keyed on the writers of the listener stores
(`git grep -nE 'listeners\.remove\(|\.remove\(entry\.id\)|remove_listener_entry|remove_listener_and_prune_back_ref' -- crates`
minus tests, 2026-11-08 at `d7ec4761`: 20 lines). Removals that precede a call in the same loop: the
session loop (`event_dispatch/mod.rs:499`, D-14), the VM dispatch loop (`event_target_dispatch.rs:460` via
`remove_listener_entry`, D-5, P0b-L3) and the `unhandledrejection` loop (`natives_promise.rs:565`, D-15).
Under β the shape's defect is the unreported error after the call, not the removal: D-14 and D-15 report
here; D-5's report is P0b-L3's (hand-off §4). The other hits are `removeEventListener` / abort-steps
removals and test assertions, none followed by a call of the removed entry. L3's brand-check program
changes how `remove_listener_entry` receives its `DispatchTarget`: whichever lands first, the later PR
adapts to the earlier one's form (L3 brand-check Rev 4).

### §5.5 D-16: internal promise reactions drop a failed handler call (Rev 2 Axis 2 IMP-2; PM 590)

| Id | Site | What the error becomes today | Class | Governing step |
|---|---|---|---|---|
| D-16 | `vm/natives_promise.rs` `run_reaction` (`:665`): `vm.call(handler, …)` at `:689` (CP-1); the `Err` arm (`:695-703`) settles only `if let Some(cap) = capability` | with `capability == None` the error is dropped: no report, the owner never settles | D (C-3: dropped at a script-controlled depth, §3.1; P0a-2a §7.2's re-run had it as R on the `Some` arm only) | ECMA-262 NewPromiseReactionJob step 1.f (async driver) and 1.h.i on the promise `cancel()` returns (cancel, ReadableStreamCancel step 8, spec-exact) and on the omitted derived capability (stream start / pull, combinators); per owner, §3.2 |

The id D-16 is the P0 census series (D-1…D-16); the code comment `D-16 #11-wasm-vm` in
`interpreter_call.rs` is another series.

**Facts (re-verified on this branch's merge of main).** `run_reaction` takes `capability: Option<ObjectId>`
(`:669`); the reaction record carries it (`Reaction`, `coroutine_types.rs:183-187`; the microtask
`PromiseReaction`, `natives_promise.rs:39-44`; traced by `gc/roots.rs:416-428` and `gc/trace.rs:284`).
`subscribe_then` (`natives_promise.rs:883-892`) always passes `None`. `None` subscribers (`git grep -n
'subscribe_then(\|then_impl_internal(' -- crates/script/elidex-js/src` minus definitions and tests):
- the async driver, `natives_generator.rs:514` (`AsyncDriverStep` fulfil / reject steps, owner: the
  generator `gen`, whose `wrapper` is the async function's promise, `coroutine_types.rs:38`);
- `readable_stream/mod.rs:466` (`ReadableStreamStartStep { stream_id, .. }`, owner: the stream);
- `readable_stream/controller.rs:170` (`ReadableStreamPullStep { stream_id, .. }`, owner: the stream);
- `readable_stream/mod.rs:667` (`ReadableStreamCancelStep { promise, .. }`, owner: the cancel promise `p`);
- **the combinator per-item subscription**, `natives_promise_combinator.rs:446` (`subscribe`,
  `then_impl_internal(.., None)`; callers `:359` for `race`, whose handlers are the result promise's
  resolving functions, and `:412` for `all` / `allSettled` / `any`, whose handlers are
  `PromiseCombinatorStep`s; owner: the combinator's result promise `result`). The brief's list named four
  subscribers; this fifth is `then_impl_internal` called directly (Rev 2's facts file left the combinator
  claim unverified).

**Path.** An awaited promise settles; the reaction job runs `run_reaction`, whose `vm.call` is a CP-1 entry.
If that check fails (or the internal handler returns any other `Err`), the `None` arm drops it.
`call_dispatch`'s `AsyncDriverStep` arm (`interpreter_call.rs:183-190`) is never reached, so the generator
stays suspended, the wrapper never settles and nothing is reported. The stream and combinator owners stay
pending the same way.

**The spec's position, per owner (§3.2 has the steps).** *Async driver*: Await step 7 subscribes with no
capability, so NewPromiseReactionJob step 1.f applies, and its assert (1.f.i) says the handler's completion
is not abrupt: Await's closures always return normally. A resource limit at the handler's call breaks that
assert, and the spec is silent on what then happens (P0 umbrella §4 C-4, authority chain item 1).
*Cancel*: the reaction's capability is the promise `cancel()` returned (ReadableStreamCancel step 8, WebIDL
*react* steps 6–8), so rejecting it is spec-exact. *Start / pull and combinators*: the spec's reaction has a
capability, a derived promise that nobody reads (WebIDL *react*'s discarded `newCapability`; the promise
`then` returns inside PerformPromiseAll step 5.k and its siblings); elidex omits it. The spec's branch,
step 1.h.i, rejects that derived promise and leaves the stream readable and the result promise pending.
**Where the spec stops**: at the async driver's assert, and at the discarded
derived promise for start / pull and combinators. From there the elidex principles decide: the failure
must surface on the owner's channel (C-3; fail-safe: an unknown falls to a visible failure), and the place
it surfaces must be decided by structure, not by a list of handler kinds (Security by structure). Erroring
the stream and rejecting `result` are those choices, not spec branches; only cancel's rejection is the
spec's own.

**Recommended design: the subscription names its owner, by type.** `Reaction::capability: Option<ObjectId>`
becomes `target: ReactionTarget` (NEW):

```rust
enum ReactionTarget { Capability(ObjectId), Internal(ReactionOwner) }
enum ReactionOwner { AsyncFunction { gen: ObjectId }, Promise(ObjectId), Stream(ObjectId) }
```

There is no "no target" value: every subscription states where a failed handler call goes, and a new
internal subscriber does not compile until it names its owner. `subscribe_then` and the combinator's
`subscribe` take a `ReactionOwner`; `then_impl_internal` takes a `ReactionTarget`; the GC traces the ids in
both. `run_reaction`'s `Err` arm: `Capability(cap)` → reject `cap` with the materialised value (today's
`Some` arm); `Internal(owner)` → `fail_reaction_owner(vm, owner, &e)` (NEW), which materialises the error
once (`vm_error_to_thrown`) and settles the owner on its own channel, running no script:

| Owner | Settles | Same as | After |
|---|---|---|---|
| `AsyncFunction { gen }` (the async driver) | rejects `gen`'s wrapper with the `RangeError` | `drive_async_coroutine`'s `Err` arm (`natives_generator.rs:520-523`), factored into one function both call; the spec has no destination (Await step 7, NewPromiseReactionJob 1.f.i), and the wrapper is AsyncBlockStart step 1.h's channel: the principle's choice (§3.2) | the generator stays suspended and is never resumed; a `finally` in the body does not run: P0 umbrella §6 item 5, whose R row and slot `#11-vm-async-resume-failure-skips-finally` now cover the step's CP-1 as well as its CP-2 |
| `Stream(stream_id)` (start, pull) | `readable_stream::fail_step` (NEW: P0a-1, below): errors the stream with the `RangeError` (`error_stream`) and clears `pull_in_flight` | the `Err` arm after the pull call (`controller.rs:179-183`); the channel is the one Streams *SetUpReadableStreamDefaultController* step 12 / *CallPullIfNeeded* step 8 use for a rejected start / pull promise, but the spec does not take it for a failed reaction step: the principle's choice (§3.2) | the stream is errored; readers reject (§3.2) |
| `Promise(p)` (cancel; combinator result) | rejects `p` with the `RangeError` | the `Err` arm after the cancel call (`readable_stream/mod.rs:671-674`). Cancel: spec-exact (ReadableStreamCancel step 8, §3.2). Combinator: the result capability's existing reject channel; the spec would reject the omitted derived promise (NewPromiseReactionJob step 1.h.i), so rejecting `result` is the principle's choice (P0 umbrella §4 C-4 authority (2), fail-safe; §3.2) | `cancel()`'s promise rejects; `Promise.all` / `allSettled` / `any` / `race` reject. Later element settlements are ignored by the existing already-resolved gate (`settle_promise`); a later step's own failure is reported (below) |

**The stream arm lives in the stream module (Axis 1 MIN-2, MIN-3).** `error_stream`
(`controller.rs:294`) is `pub(super)` and `pull_in_flight` is host-private stream state. So the arm goes
through a new host entry, `vm/host/readable_stream::fail_step` (NEW: P0a-1; `pub(crate)`, signature
`(vm, stream_id, reason)`), beside `run_start_step` / `run_pull_step` / `run_cancel_step` (`mod.rs:473`, `:498`,
`:538`). It owns both writes: `error_stream`, and the `pull_in_flight` clear. Like `run_*_step`, it
first checks `vm.readable_stream_states.contains_key(&stream_id)` (`mod.rs:479`, `:504`), because
`error_stream` expects the entry (`stream_state_mut`); a missing entry is the gate's "already final"
answer (below). Core `fail_reaction_owner`
calls it the way `call_dispatch` calls the `run_*_step` functions. The Streams algorithm stays in its
module (Layering mandate), and core does not widen `error_stream`. **No start / pull discriminator.** The
two existing reject branches are the same call (`run_start_step` and `run_pull_step` each run
`error_stream(vm, stream_id, value)` on `is_reject`). The clear is a no-op for a start failure: pull never
fires before `start_called` (`pull_should_fire`, `controller.rs:77-83`), so `pull_in_flight` is false
then. Cancel needs no stream target either: `ReadableStreamCancelStep` carries the cancel promise, not the
stream (`interpreter_call.rs` `run_cancel_step(self, promise, ..)`), so its owner is `Promise(p)` (the
table's third row). **cfg gate.** `ReactionOwner::Stream` (NEW: P0a-1), its arm in `fail_reaction_owner` and its GC trace arm are
`#[cfg(feature = "engine")]`, like the `ObjectKind::ReadableStream*Step` variants (`object_kind.rs:872-891`) and their trace
arms (`gc/trace.rs:969-977`); the `readable_stream` module itself is `#[cfg(feature = "engine")]`
(`host/mod.rs:312-313`). **Inbound host call (`#11-vm-inbound-host-call-membership-test`,
`docs/plans/2026-07-vm-p4-design-invariants.md:120`).** It rides the `run_*_step` mechanism (the same cfg
gate and the same call shape: a core free-function call into `host::readable_stream` with the stream id),
so it is not a new kind of core → host call; that slot's audit counts it with the three `run_*_step`
calls (`interpreter_call.rs:191-227`).

**An owner already final (Axis 2 IMP-1 on Rev 3b; PM 603).** Each owner's channel has a gate that makes
the settle a no-op when the owner is already final: `settle_promise`'s AlreadyResolved check
(`natives_promise.rs:129-139`; ECMA-262 §27.5.1.3 CreateResolvingFunctions, the resolve function's step
2.a and the reject function's step 4.a, "If promiseOrEmpty.[[Value]] is empty, return undefined" — the code
comment's "[[AlreadyResolved]]" is the older spec's name for that record), `reject_promise` on a
non-Pending promise (`:204-206`), `error_stream` on a non-Readable stream (`controller.rs:295-297`; Streams
§4.9.4 *ReadableStreamDefaultControllerError* step 2), and `fail_step`'s missing-entry guard. A combinator element's
step can fail after the result has settled, and a stream's start or pull step can fail after the stream
closed, errored or was cancelled. The state is then correct, but the `RangeError` would be lost without a
report. PM 590's rule is that a failure is never dropped silently, so this is a silent failure. So every
owner sink goes through its gate and returns the gate's answer, `Settle { Applied, AlreadyFinal }` (NEW:
P0a-1): `settle_promise` is split into the gated core, which returns it, and today's native-facing wrapper;
`error_stream` returns it; `fail_step` returns `AlreadyFinal` on a missing entry. The promise owners
(`AsyncFunction`'s wrapper and `Promise(p)`) are settled through `settle_promise`, never through
`reject_promise` directly, so the resolver's AlreadyResolved record is honoured. On `AlreadyFinal`,
`fail_reaction_owner` reports the materialised error through HTML *report an exception*, whose stand-in is
stderr (P0 umbrella §4 C-3; the same report `call_listener` uses, §4.5). The branch is decided by the gate's
result, not by a list of owner states, so a new owner kind or a new final state needs no new arm. The
async wrapper and the cancel promise are always Pending when their step runs, so for them the fallback is
structural and unobserved.

**`settle_promise` stays Ok-only today; thenable assimilation would change that (Axis 3 MIN-4 on Rev 3c).**
`settle_promise`'s doc comment says arbitrary thenables are not yet assimilated (`natives_promise.rs:121` at
`658cc302`, and the module doc above it). That gap's home is VM P4 slice-plan row **9da**, under umbrella
**9d** (`docs/plans/2026-07-vm-p4-slice-plan.md`; both rows cite slot `#11-vm-builtin-prototype-static-sweep`);
row 9da measures the same `:121` comment. When assimilation lands, `settle_promise` performs
`Get(resolution, "then")` (ECMA-262 §27.5.1.3 CreateResolvingFunctions, the resolve function's step 2.f;
an abrupt `Get` rejects the promise at step 2.g), which runs a getter, so a producer is reached beneath it. The 60 uses of `settle_promise` that are
Ok-only at the base (P0a-2a §7.1) then become members of the census. The ratchet catches it: P0a-2a §5
rule 1 recomputes the reach on every run, and a member without a row is red (rule 3), so the PR that adds
assimilation must classify them.

The rejection reaches *notify about rejected promises* if unhandled (D-15's loop). With the report for a
final owner, the `RangeError` is surfaced in every case. The async driver's answer to the brief's two questions: the `RangeError` goes to
the wrapper's rejection; the generator's state stays suspended (one outcome for the step's CP-1 and its
CP-2, so a cell cannot tell which check failed).

**Rejected.** (a) *Report to stderr in the `None` arm.* The error is surfaced, but the owner stays pending
forever: the async function's awaiting code never continues and its promise never settles — the trigger's
own channel is the owner, and C-3 asks for that channel. (b) *Derive the owner from the handler's
`ObjectKind`* (`AsyncDriverStep { gen }`, `ReadableStream*Step`, `PromiseCombinatorStep`, the resolving
functions). It needs a fallback arm for any other kind, i.e. a list whose completeness decides where an
unknown goes — the shape PM 557 rejected for the entered mark. (c) *Call internal handlers without CP-1*
(dispatch the step natives directly from the job). It is a second call path beside `vm.call` (one issue,
one way), removes a re-entry probe from I-3's set, and leaves the `None` arm dropping every other `Err` an
internal native returns. (d) *Deliver the failure to the step's own reject branch* (e.g. resume the async
function with a throw at the `await`). Each owner's reject branch means something different
(`allSettled`'s reject step records a result; the async driver's resumes the coroutine, a CP-2 entry at
the same depth), so it is not one rule, and for the async driver it re-enters at the depth that just
failed.

**Does it fork?** No. (a)–(d) each lose to one principle stated above; the recommendation follows from C-3
and Security by structure without a second reading.

### §5.6 Spec counterparts of the new functions and types (Axis 4 MIN-2 on Rev 3d)

| New item | Spec algorithm it stands in for |
|---|---|
| `resource_limit::check_entry` (NEW: P0a-1, §4.2) | elidex-internal, no spec counterpart: no spec defines a resource limit (ECMA-262 §17's fourth paragraph permits no such extension and §9.4 bounds no stack; P0 umbrella §4 C-4 authority (1)) |
| `ReactionTarget` (NEW: P0a-1) | `Capability` is the PromiseReaction Record's [[Capability]] (NewPromiseReactionJob step 1.a, rejected at 1.h.i); `Internal` has no spec counterpart: it stands where the spec has no capability (Await step 7, NewPromiseReactionJob 1.f) or a derived one elidex omits (§3.2) |
| `ReactionOwner` (NEW: P0a-1) | elidex-internal, no spec counterpart: the owner a failed internal step goes to is the principle's choice per owner (§5.5 "The spec's position") |
| `fail_reaction_owner` (NEW: P0a-1) | elidex-internal routing; each arm calls an existing channel: AsyncBlockStart step 1.h's rejection, `fail_step` (below), or the capability's reject (ReadableStreamCancel step 8 for cancel, spec-exact; the result capability for a combinator, the principle's choice); its `AlreadyFinal` arm is HTML §8.1.4.6 *report an exception* (§3) |
| `readable_stream::fail_step` (NEW: P0a-1) | Streams §4.9.4 *ReadableStreamDefaultControllerError* (its `error_stream` call; the missing-entry guard answers like that algorithm's step 2 for a stream that is gone); the `pull_in_flight` clear is elidex-internal state with no spec step (CallPullIfNeeded step 8 leaves [[pulling]] set and errors the stream) |
| `Settle { Applied, AlreadyFinal }` (NEW: P0a-1) | elidex-internal: it returns as a value the early return of the gates it reports — CreateResolvingFunctions' resolve step 2.a / reject step 4.a (`promiseOrEmpty` empty) and *ReadableStreamDefaultControllerError* step 2 (state not "readable") |

## §6. Cells (written first)

New file `crates/script/elidex-js/src/vm/tests/tests_resource_limit.rs` (NEW). Each cell arms §4.4's
injection with an `n` taken from `resource_limit::count` on a dry run of the same body.

| Cell | Body | Expect |
|---|---|---|
| `rl_identity_predicates` | build each error form | `is_resource_limit` true only for `VmError::resource_limit()`; a `ThrowValue` and a `RangeError` kind are false; `CompileError` kinds likewise; the materialised object is a `RangeError` with `STACK_EXHAUSTED_MSG` |
| `rl_cp1_entry_failure_no_callee_effect` | `let n = 0; function g() { n++ }`; inject `Entry(Call, 1)`; `try { [1].forEach(g) } catch (e) { r = e.name }`; the same through `call`, `call_construct_native`, `run_function`, `construct_synchronous` from Rust | `r === 'RangeError'`, `n === 0`, frame and value-stack depth unchanged; a second run gives `n === 1` |
| `rl_cp2_generator_resumable_after_entry_failure` | `function* g() { yield 1 } const it = g()`; inject `Entry(Resume, 1)`; `it.next()` throws; then `it.next()` | first: `RangeError`; second: `{ value: 1, done: false }` |
| `rl_async_step_entry_failure_rejects_wrapper` | an async function awaiting a resolved promise; inject `Entry(Resume, n)` for the step's resume | the wrapper rejects with `RangeError`; the body's `finally` did not run (P0 umbrella §6 item 5) |
| `rl_async_step_call_entry_failure_rejects_wrapper` | the same async function; inject `Entry(Call, n)` for the reaction job's call of the `AsyncDriverStep` (D-16) | the same outcome as the cell above: the wrapper rejects with `RangeError`, `finally` did not run; nothing on stderr from the reaction job |
| `rl_combinator_step_entry_failure_rejects_result` | `Promise.all([p1, p2])` with `p1`, `p2` resolved; inject `Entry(Call, n)` for `p1`'s element step; then the same for `Promise.race` | each result promise rejects with `RangeError` (D-16, owner `Promise`) |
| `rl_combinator_step_failure_after_result_settled_reports` | `Promise.race([p1, p2])` with `p1` resolved before `p2`; inject `Entry(Call, n)` for `p2`'s element step, after the result has settled | the result keeps `p1`'s value; the `RangeError` is reported (§9 item 4), not dropped |
| `rl_stream_step_failure_after_close_reports` | a `ReadableStream` whose `start` calls `controller.close()` and returns a resolved promise; inject `Entry(Call, n)` for the start step's call | the stream stays closed (reads resolve `done`); the `RangeError` is reported (§9 item 4) |
| `rl_stream_pull_step_entry_failure_errors_stream` | a `ReadableStream` whose `pull` returns a resolved promise; inject `Entry(Call, n)` for the pull step's call; `reader.read()`; also the start step and `cancel()`'s step | the stream is errored with `RangeError` and the read rejects; `cancel()`'s promise rejects with `RangeError` (D-16, owners `Stream` / `Promise`) |
| `rl_async_start_entry_failure_rejects_wrapper_once_consumed` | `let c = 0; async function A() { c++ }`; `t.addEventListener('x', A, { once: true })`; `document.addEventListener('unhandledrejection', e => { r = e.reason.name })`; inject `Entry(Resume, 1)` (the initial drive); dispatch through the session loop, run the microtask checkpoint, dispatch again | first: `call_listener` returned `Ok(())`, A is gone, `c === 0`, `r === 'RangeError'`; second: `c === 0`, A not run. §4.2 "The async start" |
| `rl_nested_failure_keeps_kind` | `function f() { [1].forEach(x => x) }`; from Rust `vm.call(f)` with `Entry(Call, 2)`, then with `Entry(Call, 1)`; also `vm.call(Object.assign.bind(null, {}, { get a() { return [1].forEach(x => x) } }))` with `n` for the arrow | every `Err` is kind `ResourceLimit` (`is_resource_limit()`), whether the failing check was the outer call's own or a nested one after user code ran |
| `rl_bound_native_once_listener_consumed_not_rerun` | `let c = 0; function h() { c++ }`; `t.addEventListener('x', [].forEach.bind([1, 2], h), { once: true })`; inject `Entry(Call, 3)` (entries: the listener, `h`, `h`); dispatch twice | first: `c === 1`, the error is reported and the listener is gone (step 2.5 stands); second: `c === 1` (not re-run: D-P0-2) |
| `rl_cp3a_wasm_call_entry_failure_retry` | an exported function; inject `Entry(WasmCall, 1)` | `RangeError` (not `WebAssembly.RuntimeError`); the next call returns its value |
| `rl_cp3b_instantiate_entry_failure` | inject `Entry(WasmInstantiate, 1)` for `new WebAssembly.Instance(m)`, then for `WebAssembly.instantiate(m)` | sync: `RangeError`; async: rejects `RangeError`; an un-injected retry instantiates |
| `rl_vm_eval_compile_resource_limit_kind` | inject `Compile(1)`; `vm.eval("1")`; also through the `ScriptEngine` `eval` of `ElidexJsEngine` | kind `ResourceLimit`; the `EvalResult` message starts with `RangeError` |
| `rl_regexp_literal_error_is_catchable` | `let r; try { /a{2,1}/ } catch (e) { r = e }` (a pattern `regress` 0.11.1 rejects; §9 item 1) | `r !== undefined` (today the error leaves `run()` uncaught); the error's class is **not** asserted (§5.2 "B-1's test vehicle") |
| `rl_ua_dispatch_once_listener_entry_failure_consumed` | through `script_dispatch_event` (the session loop): listeners A (`once`), B (`once`), C on one target; inject `Entry(Call, 1)` for A; dispatch twice | first: A not run, B and C run, `call_listener` returned `Err(ScriptFailure)` for A and A's `RangeError` is reported; second: C only (A and B consumed) |
| `rl_ua_dispatch_once_listener_threw_is_removed` | A (`once`) throws | A is gone after the dispatch and its callable is dropped; the error is reported (today dropped) |
| `rl_unhandledrejection_once_listener_entry_failure_consumed` | `document.addEventListener('unhandledrejection', A, { once: true })`, B (`once`), C; `Promise.reject(1)`; inject `Entry(Call, 1)` for A at the checkpoint's dispatch; then a second `Promise.reject(2)` | first: A not run, B and C run, the error reported; second: C only |
| `rl_unhandledrejection_once_listener_threw_is_removed` | A (`once`) throws | A is gone; its error is reported (today swallowed) |
| `rl_eval_result_outcome_kind` | through `ScriptEngine::eval`: inject `Compile(1)`; then a script that throws; then `drain_timers` with `Entry(Call, 1)` at the timer callback | each `outcome == Err(ScriptFailure)` and each `error` starts with the class (`RangeError`, the thrown value's) |
| `rl_tree_walker_filter_entry_failure_kind` | `document.createTreeWalker(root, NodeFilter.SHOW_ELEMENT, { acceptNode() { return 1 } })`; inject `Entry(Call, 1)` at the filter's call; `nextNode()`; then `nextNode()` | first throws `RangeError` and `currentNode` is unchanged; second returns the first element (the active flag was unset) |
| `rl_tree_walker_filter_throw_propagates_not_null` | `{ acceptNode() { throw 1 } }`; `nextNode()` | throws `1`; never `null` (the removed arm of §4.5) |
| `rl_computed_accessor_key_error_is_catchable` | `try { ({ get [{ toString() { throw 1 } }]() {} }) } catch (e) { r = e }`; and with `Entry(Call, 1)` on the `toString` | `r === 1`; with the injection, `r` is a `RangeError` |

## §7. Mutants (run on the final head, debug)

| Mutant | Change | Dies |
|---|---|---|
| M-ID | `is_resource_limit` always false | `rl_identity_predicates`, `rl_vm_eval_compile_resource_limit_kind`, `rl_nested_failure_keeps_kind` (the P0b slices record their own cells against it) |
| M-REWRITE | an exit rewrite added in `with_call_mode` (`ResourceLimit` → a thrown `RangeError`, Rev 2's escape rule) | `rl_nested_failure_keeps_kind` (`ThrowValue` instead of `ResourceLimit` for `n = 2`) |
| M-B1 | `Op::PushConst` back to `?` | `rl_regexp_literal_error_is_catchable` |
| M-B2 | `op_define_computed_accessor` back to `?` | `rl_computed_accessor_key_error_is_catchable` |
| M-P-CP1 | `check_entry` after `f(self, mode)` | `rl_cp1_entry_failure_no_callee_effect` |
| M-P-CP1s | `check_entry` after the `entry_frames` snapshot | **equivalent** (the exit path restores the snapshot); recorded, answers R2 S-2 for CP-1 |
| M-P-CP2 | the check after the generator take (marked Running before CP-2) | `rl_cp2_generator_resumable_after_entry_failure` |
| M-P-CP3a / M-P-CP3b | CP-3 checks after the `HostState` bind / after `Store::new` and the start function | `rl_cp3a_*` / `rl_cp3b_*` if observable; a survivor is recorded as equivalent with the reason (§9 item 2) |
| M-EV | `Vm::eval` maps to `CompileError` | `rl_vm_eval_compile_resource_limit_kind` |
| M-D14 | `call_listener` drops the error again (`Ok(())`, no report) | `rl_ua_dispatch_once_listener_entry_failure_consumed` (`Ok` for A); the report half is §9 item 4's |
| M-D15 | the `unhandledrejection` loop drops the error again | `rl_unhandledrejection_once_listener_threw_is_removed` if the report is observable (§9 item 4); otherwise recorded as not pinned |
| M-SINK | `fail_reaction_owner` does nothing (the old `None` arm) | `rl_async_step_call_entry_failure_rejects_wrapper`, `rl_combinator_step_entry_failure_rejects_result`, `rl_stream_pull_step_entry_failure_errors_stream` |
| M-SINK-FINAL | `fail_reaction_owner` ignores `AlreadyFinal` (no report) | `rl_combinator_step_failure_after_result_settled_reports`, `rl_stream_step_failure_after_close_reports` if the report is observable (§9 item 4); otherwise recorded as not pinned |
| M-SINK2 | the async driver's owner sink resumes the generator with a throw instead of rejecting the wrapper (rejected option (d)) | `rl_async_step_call_entry_failure_rejects_wrapper` (`finally` runs) |
| M-OUT | `outcome` always `Ok` | `rl_eval_result_outcome_kind` |
| M-BC3 | the null arm restored for `FilterError::Script` | `rl_tree_walker_filter_throw_propagates_not_null` |

## §8. Landing obligations

What P0a-1's PR (or, where marked, the PM / PMO at landing) does. This memo edits no ledger. Program-level
obligations (PR-1 precondition, breadth record, slots) are P0 umbrella §7.

The ledger and notice rows this slice triggers are rows of the P0 program's landing list (P0 umbrella §7,
PM 513 (d)): L-2, L-3 (now also D-15's report), L-5, L-6 (now also D-16's cell), L-12 (L3 notice) and L-14
(boundary notice). Slice-local: none; the census counts are P0a-2a's and P0a-2b's (P0a-2a §10, P0a-2b §10).

## §9. Unverified facts (each decided at implementation, before its cell is written)

1. The `regress` 0.11.1 pattern for `rl_regexp_literal_error_is_catchable`.
2. Whether M-P-CP3a / M-P-CP3b are observable.
3. The post-L3 function and callee names of the `AbortSignal` abort loop and of cell (o), read at P0a-1's
   base once L3's `#11-abort-signal-any-dependent-flattening` PR has landed; the allowlist row is P0a-2b's
   (P0a-2b §7.3).
4. Whether the stderr report of a listener error (§4.5, engine side; D-15 likewise) is observable in a
   cell; if not, it is recorded as not pinned (the report channel is `#11-embedder-diagnostic-channel`'s).
5. `#11-event-path-window-missing` PR-A's schedule against P0a-1's (PM 540 (2): P0a-1 first; if P0a-1 is
   late, the PM decides).
6. Whether the `unhandledrejection` dispatch of `rl_async_start_entry_failure_rejects_wrapper_once_consumed`
   runs inside the cell's microtask checkpoint (the `r === 'RangeError'` read); if the harness drains
   rejections elsewhere, the cell reads the wrapper's state directly (rejected, reason a `RangeError`) and
   the report channel is item 4's.
7. D-16: how the combinator's step reaches its result promise (`PromiseCombinatorStep`'s state →
   `result`; the subscribing call already holds `result` at `natives_promise_combinator.rs:359` / `:412`, so
   the owner is passed there), and that every `ReactionOwner` id is traced by the GC from the queued
   reaction until it runs.

webref lookups behind §4.2 "Where the kind ends" (2026-11-10): `body webcrypto SubtleCrypto-method-wrapKey`
→ §14.3.11 (steps 3–4); `dfn html 'shared history push/replace state steps'` → §7.2.5 (step 3 read by
`body html shared-history-push/replace-state-steps`).

webref lookups behind §4.2 and §5.2 (2026-11-09): `aoid ecma262 EvaluateAsyncFunctionBody` → §15.8.4
(steps 3, 5 read by `body`); `aoid ecma262 AsyncBlockStart` → §27.10.5.2 (step 1.h read by `body`);
`aoid ecma262 FunctionDeclarationInstantiation` → §10.2.11; `body ecma262
sec-primary-expression-regular-expression-literals-static-semantics-early-errors` → §13.2.7.1; `body
ecma262 sec-regular-expression-literals-runtime-semantics-evaluation` → §13.2.7.3; `body ecma262
sec-patterns-static-semantics-early-errors` → §22.2.1.1.

webref lookups behind §3 and §5.5 (2026-11-08; the D-16 rows 2026-11-10): `body ecma262
sec-error-handling-and-language-extensions`; `dfn html 'create a classic script'` → §8.1.4.3; `dfn dom
'inner invoke'` → §2.9 (steps read by `body dom concept-event-listener-inner-invoke`); `dfn dom 'filter'`
→ §6 `#concept-node-filter` (step 6 read by `body dom concept-node-filter`); `dfn html 'notify about
rejected promises'` → §8.1.4.7 (step 4.1.2 read by `body html notify-about-rejected-promises`); `dfn html
'restore the history object state'` → §7.4.6.2; `aoid ecma262 NewPromiseReactionJob` → §27.5.2.1 (steps
1.a, 1.e.i, 1.f, 1.f.i, 1.h.i read by `body`, 2026-11-10); `aoid ecma262 Await` → §27.10.5.3 (step 7 and its
closures read by `body`); `aoid ecma262 PerformPromiseAll` → §27.5.4.1.2 (step 5.k read by `body`),
`PerformPromiseAllSettled` → §27.5.4.2.1 (5.p), `PerformPromiseAny` → §27.5.4.3.1 (5.k),
`PerformPromiseRace` → §27.5.4.5.1 (1.d); `body ecma262 sec-promise.prototype.then` → §27.5.5.4 (step 4);
`body ecma262 sec-promise.all` → §27.5.4.1 (step 8.b); `aoid ecma262 CreateResolvingFunctions` → §27.5.1.3
(steps 2.a, 2.f, 2.g, 4.a read by `body`); `dfn webidl react` → §3.2.24.1 (steps 6–8 and the Note read by
`body webidl dfn-perform-steps-once-promise-is-settled`); `dfn streams ReadableStreamDefaultControllerError`
→ §4.9.4 (step 2); `dfn html 'report an exception'` → §8.1.4.6; `dfn streams
SetUpReadableStreamDefaultController` → §4.9.4 (steps 11–12 read by `body streams
set-up-readable-stream-default-controller`); `dfn streams ReadableStreamDefaultControllerCallPullIfNeeded`
→ §4.9.4 (steps 7–8); `dfn streams ReadableStreamCancel` → §4.9.2 (step 8).
