# VM stack-safety guard — P0a plan (VM core: error identity, escape rule, choke-point placement, census check)

Status: **plan (pre-implementation)**, per-PR slice of the [P0 umbrella](2026-11-vm-stack-safety-guard-p0.md)
(PM 509); `/elidex-plan-review` required before code. Owner: VM P4. Base: `ce603c74`; every code anchor
below is at that commit. Provenance root `H` and the census prototype are as in the P0 umbrella header;
the prototype is `H/scripts/err_consumer_census_proto.py` (+ `H/scripts/err_census_classify.py`,
`H/scripts/err_write_before_call_proto.py`, `H/scripts/err_boundary_census_proto.py`), its logs
`H/logs/err-census-p0.log`, `H/logs/err-census-p0-shell.log`, `H/logs/err-census-p0-classified.tsv`,
`H/logs/err-write-before-call-p0.log`, `H/logs/err-boundary-census-p0.log`. The prototype is not the deliverable: P0a ships the tracked check
of §5, and the prototype's numbers are re-derived by that check at P0a's base.

The contract C-1…C-5 is the P0 umbrella §4 (canonical); this memo does not restate it. Edges, schedule
and the other slices: P0 umbrella §1.

## §0. Charter

P0a implements the VM half of the P0 contract: the error identity (§4.1), the escape rule (§4.2), the
placement of the four choke-point checks (§4.3), the test-only injection the consumer slices use (§4.4),
the engine boundary's failure kind (§4.5, PM 533 (a)), the tracked census check and its allowlist with
the `# pending` section (§5), and the fixes D-13, B-1, B-2, D-14 and BC-1…BC-3 (§6). Each later P0b slice only removes its rows from `# pending` (P0 umbrella §0.4).

## §1. Scope

**In.** `crates/script/elidex-js/src/vm/error.rs`, `compiler/mod.rs` (`CompileError` kind),
`vm/resource_limit.rs` (NEW), `vm/interpreter.rs` (CP-1 hook, `Vm::eval` mapping),
`vm/natives_generator.rs` (CP-2 hook), `vm/host/wasm/exported_func.rs` and `vm/host/wasm/instance.rs`
(CP-3a / CP-3b hooks and the named conversion), `vm/ops.rs` (escape rule), `vm/dispatch.rs` and
`vm/dispatch_objects.rs` (B-1, B-2 routing); the injection hooks in `compile_script`, `parse_json_str`,
`stringify_for_structured_shortcut` and `dispatch_vm_simple_event` (§4.4), with the removal of L3's
`fail_next_abort_dispatch` seam and the move of its cell onto that hook (§4.4; L3's files, site list via PM,
P0 umbrella §7 L-12); the tracked check (§5.1); cells in
`crates/script/elidex-js/src/vm/tests/tests_resource_limit.rs` (NEW). For §4.5: in
`crates/script/elidex-script-session/src/`, `outcome.rs` (NEW), `engine.rs` (`ScriptEngine`, `EvalResult`),
`event_dispatch/mod.rs` (`script_dispatch_event_core`), `event_listener.rs` (`remove_with_position`,
`reinsert`), `lib.rs` (re-export); `crates/script/elidex-js/src/engine.rs` (`eval`, `call_listener`,
`drain_timers`); `crates/dom/elidex-dom-api/src/traversal.rs` (`FilterError`) and its `elidex-js` users
(`vm/host/node_filter_dispatch.rs`, `tree_walker_proto.rs`, `node_iterator_proto.rs`); the shell's
`EvalResult` readers (`lib.rs`, `pipeline.rs`, `content/event_loop.rs`, `content/iframe/render.rs`,
`content/iframe/thread.rs`) and the tests that read `EvalResult`'s `success` field or call `call_listener`. The
session-crate and `elidex-dom-api` files are not VM P4's; the PM routes the notice (P0 umbrella §7 L-14).

**Out.** The consumer fixes D-1…D-12 (P0b-VM-1, P0b-VM-2, P0b-L3; P0 umbrella §1); `native_import_scripts`'s kind
mapping (D-11, P0b-VM-1) although §4.1 item 3 states the rule; the deviations of P0 umbrella §6.

## §2. Coupled invariants

Elements: **ID** (error kinds, predicates, `Vm::eval` boundary; §4.1), **ER** (escape rule; §4.2),
**B** (B-1 / B-2 routing; §6.2), **CP** (choke-point placement; §4.3), **INJ** (test-only injection;
§4.4), **CEN** (census rules; §5.2), **PEND** (`# pending` section and planned-producer list; §5.3, §5.4),
**OUT** (the boundary's `ScriptFailure` and the D-14 revert; §4.5).

| Pair | Intersection | Pinned by | Basis |
|---|---|---|---|
| ID × ER | ER rewrites the kind to `ThrowValue` with the same message, so `is_resource_limit()` is false after a started frame | `rl_escape_rule_materializes_in_started_frame` | cell |
| ID × CP | each check returns `VmError::resource_limit()`; CP-3b alone materialises it (`vm_error_to_thrown`), at a C-3 channel | `rl_cp3b_instantiate_entry_failure`, `rl_cp3a_wasm_call_entry_failure_retry` | cell |
| ID × INJ | every injection point delivers exactly C-1's kind (`VmError::resource_limit()` / `CompileErrorKind::ResourceLimit`) | `rl_identity_predicates`, `rl_vm_eval_compile_resource_limit_kind` | cell |
| ID × CEN | producer derivation keys on calls to `check_entry` and on constructions of the two kinds | control `census_planned_producer_already_derived` | check |
| ID × PEND | D-13 is fixed here, so it is a P row at landing, never pending | §5.4 | structural |
| ER × B | ER is sound only if every runtime-channel error leaving a started frame passes `throw_error`; B-1 / B-2 are the two that do not | `rl_regexp_literal_error_is_catchable`, `rl_computed_accessor_key_error_is_catchable`; M-B1, M-B2 | cell |
| ER × CP | a CP-1 failure before any frame stays `ResourceLimit` (n = 1); one inside a started frame comes out `ThrowValue` (n = 2) | `rl_escape_rule_materializes_in_started_frame` | cell |
| ER × CEN | rule 6 (`?` on a carrier in `run()`) is the census-side guard of ER | control `census_run_question_mark_on_carrier` | check |
| B × CEN | at base, B-1 / B-2 are rule 6 members; after the fix they are pure throw arms (auto-recognised), leaving only the "throw-routed" P rows | §5.2 rule 6 | check |
| CP × INJ | at P0a the body of `check_entry` is the injection only; `resource_limit::count` counts checks per kind, from which every placement cell takes its `n` | `rl_cp1_*`, `rl_cp2_*`, `rl_cp3*` | cell |
| CP × CEN | the four enclosing functions of CP-1…CP-3b are the derived producers at P0a; carriers are closed over them | §5.3 | check |
| INJ × CEN | the boundary hooks in `compile_script` / `parse_json_str` / `stringify_for_structured_shortcut` are `#[cfg(test)]` statements; the lexer excludes `#[cfg(test)]`-attributed items **and statements**, so a hook does not make its function a derived producer (else `compile_script`, a planned producer, would be red "remove it" before PR-1) | control `census_cfg_test_hook_not_producer` | check |
| CEN × PEND | a D row is red outside `# pending`; a pending row outside the frozen ceiling is red, so the section only shrinks; rule 8's 120 members are all P, so the ceiling is unchanged | controls `census_rejects_definitive_row`, `census_pending_row_outside_ceiling` | check |
| CEN (rule 8) × ER | rule 8 sees a `?` that is correct for the trigger but skips the loop's other subjects; ER is unaffected (the `?` propagates the kind) | control `census_loop_question_mark_member` | check |
| OUT × ID | `NotStarted` ⇔ `is_resource_limit()`; one mapping function, so no boundary can classify on its own | `rl_eval_result_outcome_kind` | cell |
| OUT × ER | a `NotStarted` never passed a started frame (the escape rule rewrites any later one to `ThrowValue` → `Threw`), so the revert is exact: nothing ran | `rl_ua_dispatch_once_listener_survives_entry_failure` (the listener's body has not run) | cell |
| OUT × INJ | the UA dispatch path is reached through `script_dispatch_event` with `Entry(Call, n)`; no new injection point | same cell | cell |
| OUT × CEN | rule 11 makes `ScriptFailure`-bearing types kind-bearing, so the fixed boundaries leave the population; rule 7 keeps D-14's write a W row | controls `census_kind_bearing_type_without_failure`, `census_w_across_boundary` | check |
| OUT × PEND | D-14 is fixed here, so it never enters `# pending`; the ceiling stays 15 | §5.4 | structural |
| INJ × L3 seam | one test-failure mechanism: `SimpleEventDispatch(n)` replaces L3's `fail_next_abort_dispatch`, and as a `#[cfg(test)]` statement it does not make `dispatch_vm_simple_event` a producer | cell (o) after the move; control `census_cfg_test_hook_not_producer` | cell + check |

## §3. Spec coverage map

No spec defines a resource limit (P0 umbrella §3). The rows are the clauses this slice's code answers to.

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| ECMA-262 §17 Error Handling and Language Extensions | paragraph 1 (errors reported when the construct is evaluated), paragraph 3 (no other kinds of errors as early errors) | a resource limit is neither an early error nor a specified error; it surfaces where the construct is evaluated; B-1 / B-2 errors become catchable where evaluated | `VmErrorKind::ResourceLimit` (NEW), `CompileErrorKind::ResourceLimit` (NEW), `Op::PushConst`, `op_define_computed_accessor` | ✓ | yes |
| WHATWG HTML §8.1.4.3 Creating scripts | *create a classic script* steps 10–11 (ParseScript; a list of errors becomes the error to rethrow) | a compile-time resource limit is not a parse error | `Vm::eval` mapping (`interpreter.rs:36`); `EvalResult::outcome` (BC-1) | ✓ | yes |
| WHATWG DOM §2.9 Dispatching events | *inner invoke* step 2 (listeners whose removed is false), 2.5 (once: remove before the call), 2.13 (call; on a throw, report and continue) | `NotStarted`: revert 2.5 (re-entry subject, C-4); `Threw`: 2.5 stands, 2.13.1 report | `script_dispatch_event_core` (D-14), `call_listener` | ✓ | yes |
| WHATWG DOM §6 Traversal | *filter* step 6 (on a throw: unset the active flag, rethrow) | every failure propagates, nothing written; the kind crosses as `FilterError::Script` (NEW: P0a) | `FilterAction::accept` (BC-3) | ✓ | yes |

**Breadth**: K=3 specs (ECMA-262, WHATWG HTML, WHATWG DOM), M=4 entries (verified 2026-11-08 by `python3 .claude/skills/elidex-plan-review/preflight.py` on this memo: rc 0, K=3, M=4, "ok (single PR scope)"). Below the split thresholds.

### §3.1 User-input touch audit

Every placement is reachable from script: the caller's stack depth decides whether the check fails.
Adjacent pre-existing code: `Op::PushConst` propagates `load_constant`'s error with `?` out of `run()`, so
an invalid regexp literal's error is not catchable by the script that evaluates it (B-1) — exposure rises
when PR-1's front door adds a `RangeError` to that site, so P0a routes it. The computed accessor key of
`Op::DefineComputedGetter` / `Op::DefineComputedSetter` is the same shape (B-2): a key whose `toString`
throws escapes `run()` today.

## §4. Types and seams

### §4.1 Error identity

1. `VmErrorKind::ResourceLimit` (NEW), built by `VmError::resource_limit` (NEW) with message
   `STACK_EXHAUSTED_MSG` (NEW, `vm/error.rs`). `vm_error_to_thrown` materialises it as a `RangeError`
   object (`name` "RangeError", `message` the constant); `Display` prints `RangeError: <message>`.
   `VmError::is_resource_limit` (NEW) is true for that kind only.
2. `CompileError` gains `kind: CompileErrorKind` (NEW) with variants `Syntax` and `ResourceLimit`;
   `CompileError::is_resource_limit` (NEW). Every current construction site is `Syntax`.
3. Boundaries keep the kind: `Vm::eval` (`interpreter.rs:36`) maps `CompileErrorKind::ResourceLimit` to
   `VmError::resource_limit()` and `Syntax` to today's `VmErrorKind::CompileError` (D-13, §6.1). The same
   rule binds `native_import_scripts` (`worker_scope.rs:481`), implemented by P0b-VM-1 as D-11.
4. **Compile-channel aggregation (rule here, implementation in PR-1).** `compile_script` reports
   `CompileErrorKind::ResourceLimit` when any parse or scope error is a probe failure, whatever else the
   lists hold: a parse that hit the probe is not a verdict about the source. PR-1 adds
   `JsParseErrorKind::StackExhausted` (NEW: PR-1) and the one arm in `compile_script`, with its cells
   (PR-1 §8 `frontend_*`, `parser_*`).

The compile-time channel has one consumer of the front end's error lists: `compile_script`
(`compiler/mod.rs:142-155`) is the only production caller of `parse_script` / `analyze_scopes`
(`git grep -nE '(parse_script|analyze_scopes)\(' -- crates` minus tests: `compiler/mod.rs` lines
143/149 and the definitions `lib.rs:91/104`). `compile_script` itself has 3 production callers
(`git grep -n 'compile_script(' -- crates` minus tests: `interpreter.rs:36`,
`event_handler_attrs.rs:677`, `worker_scope.rs:481`), all three in the census (D-13 here; D-1 P0b-L3;
D-11 P0b-VM-1).

### §4.2 The escape rule

`throw_error` on `VmInner` (`vm/ops.rs:155`) materialises the error and searches for a handler. Today, when
none is found inside the activation, it returns the **original** error (`Err(error)`, `ops.rs:165`). For
`VmErrorKind::ResourceLimit` it returns instead `VmError { kind: ThrowValue(thrown), message }` with the
same message. Consequence: an error that is still `ResourceLimit` when a consumer sees it never passed a
started script frame, so "did not start" (C-1) is exact; an error raised inside a constructor, a getter or
a callback body that ran reaches the consumer as that body's exception and C-4's spec branch applies
(D-P0-2, ratified PM 509 item 7).

The rule needs every runtime-channel error that leaves a started frame to pass `throw_error`. The census
checks this for the dispatch loop (§5.2 rule 6). Two sites do not today: `Op::PushConst`
(`dispatch.rs:91`) propagates `load_constant`'s error with `?` (B-1), and `op_define_computed_accessor`
(`dispatch_objects.rs:227`, called with `?` at `dispatch.rs:842/845`) propagates `make_property_key`'s
ToPropertyKey error, which can come from script (`to_string` → ToPrimitive), with `?` (B-2). P0a routes
both through `throw_error` (§6.2), which also makes today's errors at those sites catchable.

### §4.3 The choke-point check: placement (I-3, VM half)

`crate::vm::resource_limit::check_entry` (NEW) takes an `EntryKind` (NEW: `Call`, `Resume`, `WasmCall`,
`WasmInstantiate`) and returns `Result<(), VmError>`. P0a places it; PR-1 adds `ensure_stack(ask)` for
`Call` / `Resume` inside it, PR-1b for `WasmCall` / `WasmInstantiate`. The function is the one place where
the ask per kind is chosen. At P0a its body is the `#[cfg(test)]` injection of §4.4 only.

| Id | Site | Placement | What must not have happened yet | Why it holds |
|---|---|---|---|---|
| CP-1 | `VmInner::with_call_mode` (`interpreter.rs:160`) | first statement | the callee's first effect | `with_call_mode` restores frames, value stack, `gc_enabled`, `active_bound_key` and `completion_value` on every exit (`interpreter.rs:166-276`), so any position before `f(self, mode)` is equivalent; the check must precede `f` |
| CP-2 | `VmInner::resume_generator` (`natives_generator.rs:114`) | first statement | the generator taken and marked Running | a Running generator rejects the next resume ("already running") |
| CP-3a | `call_wasm_exported_function` (`host/wasm/exported_func.rs:268`) | before the `HostState` bind and `set_fuel` | store / instance mutation | PR-1b §2 |
| CP-3b | `instantiate_module` (`host/wasm/instance.rs:151`) | first statement, before the lazy `vm_wasm_runtime()` init, `Store::new` and the start function | runtime init, store creation, start-function effects | PR-1b §2 |

VM entry functions: `call`, `call_construct_native`, `run_function`, `construct_synchronous` and
`dispatch_super_inner` mutate nothing before `with_call_mode` (R1 Axis 2, verified at `47b6c934`; re-checked
at P0a's base by `rl_cp1_entry_failure_no_callee_effect`, whose body enters through each of the first four).

**CP-3 conversions (named).** `call_wasm_exported_function` returns `Result<_, VmError>` and propagates the
check with `?`. `instantiate_module` returns `Result<ObjectId, JsValue>`; the check's error converts by
`.map_err(|e| ctx.vm.vm_error_to_thrown(&e))`, the in-file precedent (`instance.rs:162-163`, `coerce_import_object`), never through
`wasm_error_to_js_value` (which would make it `WebAssembly.RuntimeError`). Its two consumers propagate
(`instance.rs:69`) or reject (`namespace.rs:578`); both are C-3 channels, so the materialised form is
admissible there.

### §4.4 Test-only injection

`resource_limit::inject` (NEW: test-only) arms a thread-local: `InjectAt::Entry(EntryKind, n)` makes
the n-th `check_entry` of that kind on this thread return `VmError::resource_limit()`;
`InjectAt::Compile(n)`, `InjectAt::JsonParse(n)`, `InjectAt::JsonStringify(n)` and
`InjectAt::SimpleEventDispatch(n)` do the same at the entry of `compile_script`, `parse_json_str`,
`stringify_for_structured_shortcut` and `dispatch_vm_simple_event` (`event_target_dispatch_vm.rs:653`). `resource_limit::count`
(NEW: test-only) records how many checks of a kind a body performs, so a cell derives its `n` from a
dry run instead of guessing. The four boundary points are not production probe sites: they let a cell
deliver C-1's kind to a consumer through the consumer's real call path before PR-1's walker and front-end
probes exist, and they stay after PR-1 so the consumer cells do not depend on stack sizes. Each boundary
hook is a `#[cfg(test)]`-attributed statement (§2 INJ × CEN). They are the only injection points; a
consumer cell in any slice that needs another is a design change to this section. The P0b slices use them
as they are.

**One test-failure seam (one issue, one way; ratified PM 513; canonical home of the seam's order, PM
538).** L3's `#11-abort-signal-any-dependent-flattening` PR (PR-A, around 11-10; its plan Rev 2.1 §6 (o) and
§9 Edges) ships a test-only field `VmInner::fail_next_abort_dispatch: bool` for its cell (o): when set, the
next abort dispatch returns `Err` in place of the `dispatch_vm_simple_event` call and the field is cleared,
so the cell can show that every dependent's abort steps still run. That is a second test-failure mechanism
beside this one, temporary by plan. **Expected order (PR-A first, then P0a rebased on it):** P0a deletes
`fail_next_abort_dispatch` and rewrites cell (o) to arm `InjectAt::SimpleEventDispatch(n)` (the general form,
at the same call) with `n` from `resource_limit::count`, same assertion; its error is C-1's kind, which is
also the only `Err` that can reach this call once P0's error exists (§5.4). **Inverted order (P0a first):**
there is no seam to move; PR-A arms `InjectAt::SimpleEventDispatch(n)` from the start and adds no field.
Either way L3's contract is unchanged, and after both PRs exactly one seam exists. P0 umbrella §1 points
here.

### §4.5 The engine boundary keeps the kind (PM 533 (a))

**Defect (R3 Axis 1 IMP-1, verified at `02386ec9`).** `script_dispatch_event_core`
(`crates/script/elidex-script-session/src/event_dispatch/mod.rs:493-518`) removes a `once` entry from the
`EventListeners` component (`:499`) and then calls `ScriptEngine::call_listener` (`:503`), which returns
`()`; the VM's implementation drops the call's result (`crates/script/elidex-js/src/engine.rs:275`,
`let _ = g.call(..)`). The engine-independent crate holds the subject but cannot see the kind, so it cannot
apply C-4: a once listener whose call never started is consumed (C-2 (c)). This is **D-14** (§6.3).

**The type.** `elidex_script_session::ScriptFailure` (NEW: P0a, `crates/script/elidex-script-session/src/outcome.rs`,
re-exported at the crate root):

| Variant | Meaning (engine-independent) | Produced when (engine side) |
|---|---|---|
| `Threw` | the step started and ended with an exception (including a parse error reported by the step) | the engine's error is not `is_resource_limit()` |
| `NotStarted` | the step did not start; nothing of it ran (C-1) | `VmError::is_resource_limit()` / `CompileError::is_resource_limit()` |

It is the failure half only: each boundary keeps its own success type (`()`, `NodeFilterResult`, a compile
verdict) and returns `Result<T, ScriptFailure>`. It carries no value: a JS value cannot cross into an
engine-independent crate, so the engine surfaces the error itself (C-3: report, throw or rejection) and the
crate across the boundary uses only the kind, to apply C-2 / C-4 to the subjects it holds. **Home**:
`elidex-script-session`, the sole Script↔ECS boundary (CLAUDE.md, Design philosophy); every crate that
names it already depends on it (`elidex-dom-api` → `elidex-script-session`; the shell; `elidex-js`).
Not `elidex-plugin` (no plugin trait carries a script outcome) and not a VM type (the Layering mandate keeps
the caller engine-independent). The mapping `VmError` → `ScriptFailure` is one function on the engine side
(`VmError::script_failure` (NEW: P0a), `vm/error.rs`), keyed on `is_resource_limit()`, the only test (P0
umbrella §4 "Applying it").

**Enumeration (every method of a trait declared outside `elidex-js` and implemented in it; census B1, §5.4).**

| Boundary (declaration; impl) | Today | P0a |
|---|---|---|
| `ScriptEngine::call_listener` (session `engine.rs:78`; impl `elidex-js/src/engine.rs:203`) | `()`; the impl drops the result (`:275`); the caller writes before the call | `-> Result<(), ScriptFailure>`, `#[must_use]`; D-14 (§6.3) |
| `ScriptEngine::eval` (`:59`; impl `:176`) | `EvalResult { success: bool, error: Option<String> }`: the kind becomes a string | `EvalResult { outcome: Result<(), ScriptFailure>, error: Option<String> }`; `success` is removed, no shim; BC-1 |
| `ScriptEngine::drain_timers` (`:101`; impl `:332`) | `Vec<EvalResult>` | each `EvalResult` carries the outcome (BC-1's type); BC-2 |
| `FilterAction::accept` (`crates/dom/elidex-dom-api/src/traversal.rs:401`; impl `vm/host/node_filter_dispatch.rs:51`) | `Err(FilterError::Throw)`; the kind travels out of band in `JsFilter`'s `pending_error` field | `FilterError::Throw` → `FilterError::Script(ScriptFailure)` (NEW: P0a); the value stays in `pending_error`; BC-3 |
| `ScriptEngine`: `run_microtasks`, `drain_reactions`; `HostDriver`: `deliver_mutation_records`, `deliver_resize_observations`, `deliver_intersection_observations`, `tick_network`, `deliver_sw_client_update`, `deliver_idb_versionchange`, `drain_worker_messages`, `deliver_history_step_events`, `deliver_media_query_changes`, `deliver_visual_viewport_events`, `set_history_state` (13) | `()` | unchanged; class R "drain" (§5.4): the method has no step of its own, each item's failure is consumed on that item's own channel inside the engine at a site that is its own Err-population row, and no caller holds a subject across the call (rule 7, W across) |
| `HostDriver::bind`, `unbind`, `teardown_document`, `seed_sw_client`, `take_pending_storage_changes`, `sw_controller_scope`, `storage_origin_key`, `install_network_handle`, `install_idb_backend` (9) | `()` / a value | unchanged; class N "name-graph collision" (rule 10): no script runs; the name graph reaches a producer only through a colliding name (`new`, `to_string`, `bind`) |
| `HostDriver::with_bound` | `R`, the closure's value | unchanged; class P "pass-through": it returns the closure's own value |
| `ScriptEngine`'s `remove_listener`, `bound_dom_mut`; the other 32 `HostDriver` methods; `MutationDispatcher::dispatch` (`Send + Sync`, no VM) | — | not members (no producer) |

No other implementor exists in non-test code (`git grep -nE 'impl(<[^>]*>)? (\w+::)*(ScriptEngine|HostDriver) for' -- crates`
→ `elidex-js/src/engine.rs:175`, `:373` only, verified 2026-11-08). Test implementors and test callers adapt
in P0a's PR (the tests of `elidex-js` that read `EvalResult`'s `success` field or ignore `call_listener`; the mock
`ScriptEngine` that `#11-event-path-window-missing` plans in `event_dispatch/test_support.rs`, whichever of
the two PRs lands second, §10 item 5).

**The engine side.** `call_listener` matches the call's result: `Ok` → `Ok(())`; an error → it is reported
(the stderr stand-in of P0 umbrella §4 C-3, as `observer_common.rs` does; DOM §2.9 *inner invoke* step
2.13.1 for a throw) and `Err(e.script_failure())` is returned. A listener id that no longer resolves stays
`Ok(())` (step 2, "whose removed is false"). `eval` and `drain_timers` build `outcome` from the same mapping.
After P0b-L3, a failure of `ensure_event_handler_current` (`engine.rs:234`) returns its kind the same way
without calling the listener (hand-off §4).

**The engine-independent side (C-4, the once answer).** A `once` listener whose call returned
`Err(ScriptFailure::NotStarted)` is a re-entry subject (P0 umbrella §4 C-4: a listener still in the list,
DOM §2.9 *inner invoke* step 2 "whose removed is false"; a later dispatch is the spec's re-attempt). C-4
requires the subject as it was before the trigger, so `script_dispatch_event_core`:

1. takes the entry out with `EventListeners::remove_with_position` (NEW: P0a; it returns the entry and its
   index) at step 2.5, as today's removal;
2. on `NotStarted`, puts it back with `EventListeners::reinsert` (NEW: P0a) **at that index**, with the
   same options and `removed` false, **before** the step's microtask checkpoint (`run_microtasks`, `:513`),
   the first point after the failed call where script can run, so no script observes the removal;
3. skips the engine-side `remove_listener` (`:517`), so the callable and its `AbortSignal` back-ref stay;
4. does not invoke it again in this dispatch (the contract retries nothing, P0 umbrella §4) and continues
   with the next listener, as step 2.13 does after a reported exception.

On `Threw`, step 2.5 stands (the step started): the entry stays removed and `remove_listener` runs, as
today. A non-`once` listener has nothing to revert. Nothing ran, so the index is exact (no script mutated
the list between removal and reinsertion; a debug assertion checks the entry is absent at reinsertion).
The two methods are the one revert API of the program: P0b-L3's D-5 (`invoke_listeners_shared`, the VM
dispatch loop) uses them unchanged (hand-off §4).

**Rejected (PM 533).** (b) Removing the `once` entry after the call: DOM §2.9 *inner invoke* removes before
the call (step 2.5 before step 2.13), and a listener that re-dispatches the same event during its own call
must not run twice. (c) A top-only N row for `call_listener`: the collapsing boundary is the defect itself;
a row that watches it leaves the defect in place and would go silently live on the first nested path into
the session dispatch.

## §5. The tracked check

### §5.1 The check's files (all NEW)

- `.claude/tools/_rust_callgraph.py`: the shared lexer (comments, strings and raw strings stripped with
  line numbers kept; `#[cfg(test)] mod x { }` blocks, `#[cfg(test)] mod x;` files and other
  `#[cfg(test)]`-attributed items and statements excluded), function extraction with signature and return
  type, name-based call edges, carrier closure. PR-1's `recursion_cutcheck.py` builds on it (one call-graph
  substrate for both checks).
- `.claude/tools/err_consumer_census.py`: the census over P0 umbrella §5.1's populations.
- `.claude/tools/err-consumer-census.tsv`: the allowlist; one row per non-auto member: path, enclosing
  function, callee, shape, class, reason. Keyed by (path, function, callee, shape, occurrence), not by
  line. A `# pending` section holds the D rows (§5.4).
- `.claude/tools/test_err_consumer_census.py`: tracked `test_*.py`, collected by `scripts/python-suites.sh`
  into the `tools` job; runs the check on the tree (rc 0) and the controls of §5.5.

### §5.2 Rules

1. Populations per P0 umbrella §5.1 (Err, boundary, W), recomputed from the code on every run; test files
   and `#[cfg(test)]` code are excluded by the lexer (§5.1), not by a list. The crates scanned are derived:
   `elidex-js`, every workspace crate whose Cargo.toml depends on it (the shell), and every crate whose
   non-test source names a kind-bearing type (rule 11) or declares a trait that `elidex-js` implements.
2. Auto-recognised shapes are narrow: `?`; `.map_err(f)?` where `f` is `vm_error_to_thrown` or a `From` /
   `Into` conversion; `return` / tail in a carrier function; an arm (or an `if let Err(e) = ..`
   block) whose whole body is `self.throw_error(e, ..)?` or `return Err(e)` or `Err(e)`. Anything else is a member.
3. A member without a row is red; a row without a member is red (the allowlist cannot drift either way).
4. Admissible classes: P, T, R, S, E, N, X (P0 umbrella §5.2). A D row is red, except in the `# pending`
   section, whose rows name their fixing slice (`P0b-VM-1` / `P0b-VM-2` / `P0b-L3`) and lie inside the frozen ceiling
   (§5.4); PR-1's precondition is an empty section (P0 umbrella §7 L-1).
5. A row of class N that claims top-only is red if the reverse call graph reaches it from a native
   function (signature `(ctx: &mut NativeContext<'_>, this: JsValue, args: &[JsValue])`); edges into a
   function whose body is a thread entry (`std::thread::Builder::spawn` / `std::thread::spawn` closure)
   are cut. The other N reasons (argument class, unreachable by construction) are reviewed.
6. Dispatch loop: in `run()`, a `?` on a carrier is a member, not propagation, unless the callee is a
   throw-routed helper listed as such (class P, reason "throw-routed"); a new one is red. At `ce603c74`
   the `?` uses on a carrier in `run()` are `dispatch.rs` 91 (B-1), 842 / 845 (B-2), and 674 / 677 / 885 /
   941, whose callees route their own errors through `throw_error` (`dispatch_objects.rs:40`, `:72`;
   `dispatch_class.rs:47`, `:73`) and are rows of class P "throw-routed" (verified 2026-11-08 by the
   prototype filtered to `run` and by reading each callee).
7. W population: every function of P0 umbrella §5.1's W population has a row (class D pending, or
   admissible with a reason). The callee set is the carriers **and the boundary members** (rule 9), and the
   function may sit in any scanned crate (W across the boundary): `script_dispatch_event_core`'s write at
   `:499` before `call_listener` is how D-14 is seen.
8. **Loop rule (fail-safe; ratified PM 513).** A carrier call propagated by `?` (or `.map_err(..)?`, or a
   binding later propagated so) whose position lies inside a `for` / `while` / `while let` / `loop` body of
   the enclosing function is a **member**, not auto-propagation: the `?` is correct for the trigger but may
   leave the loop's remaining subjects unprocessed, the shape of P0 umbrella §5.1's C-2 (c) that rule 2
   would otherwise accept unseen. The rule does not try to tell "several subjects" from "elements of one
   operation"; every such `?` is a member, and the row's class says which it is (§5.4). `run()` is
   excluded (rule 6 already makes its `?` uses members).
9. **Boundary population (PM 533 (2); fail-safe).** Members, each needing a row:
   (B1) every method of an `impl Trait for T` in `elidex-js` non-test code whose trait is not declared in
   `elidex-js`, when its body reaches a producer in the name graph closed over **every** function (not only
   carrier-typed ones: a boundary method returns `()`) and its return type is not kind-bearing (rule 11);
   (B2) every `pub` function of `elidex-js` with those two properties that a dependent crate calls by name;
   (B3) every call from `elidex-js` to a function of another crate that takes a closure or a function
   pointer (`dyn Fn*`, `impl Fn*`, `fn(..)`, a generic bounded by `Fn*`): the name graph does not follow a
   closure across the call, so the call is a member whatever its body reaches. Fail-safe: an `impl` header
   the lexer cannot parse, a trait it cannot resolve, a return type that is a bare generic parameter, and a
   B3 callee name it cannot resolve through the file's `use` items are members. A B3 row of class N is red
   when the closure literal at the call reaches a producer, or when the argument is not a literal.
10. **Collision rows.** A row of class N whose reason is "name-graph collision" names the colliding edge
    (caller, callee name); the check removes that edge and recomputes reachability, and the row is red if
    the member still reaches a producer without it (another path exists); the row is stale (rule 3) when
    the member reaches no producer even with the edge.
11. **Kind-bearing types.** A return type is kind-bearing when it is `Result<_, E>` with `E` a carrier
    error type (P0 umbrella §5.1), or a type in the check's `KIND_BEARING` list (`ScriptFailure`,
    `EvalResult`, `FilterError` after §4.5). A listed type whose definition does not name `ScriptFailure`
    is red, so the list cannot bless a type that dropped the kind.

### §5.3 Producers and the planned list

Producers are derived: functions that call `check_entry`, `ensure_stack` (NEW: PR-1), or construct
`VmError::resource_limit` / `CompileErrorKind::ResourceLimit`. Until PR-1 / PR-1b land, the check also
reads `PLANNED_PRODUCERS`, a constant listing PR-1 §5.2's walker functions, the compile front end's entry
(`compile_script`), the regexp compile sites of PR-1 §6.1 (`load_constant`, `native_string_match`,
`native_string_search`) and PR-1b's `call_wasm_exported_function` / `instantiate_module`. A planned name
that is also derived is red ("remove it"), so the list only shrinks and is empty after PR-1b. At P0a's
landing `call_wasm_exported_function` and `instantiate_module` are already derived (they call
`check_entry`, §4.3), so they are not in the list P0a ships.

### §5.4 The allowlist at P0a's landing

Prototype at `ce603c74` (verified 2026-11-08 via
`python3 -I H/scripts/err_consumer_census_proto.py <wt> crates/script/elidex-js`, log
`H/logs/err-census-p0.log`): 4307 function bodies, 2236 carriers by type, 1042 carriers that reach a
producer; 1575 uses propagate by `?` and 188 by return / tail, auto-recognised; 196 non-propagating uses in
`elidex-js` and 6 in the shell (`H/logs/err-census-p0-shell.log`, seeds `eval drain_timers eval_script`),
202 rows, each hand-classified (`H/scripts/err_census_classify.py`, output
`H/logs/err-census-p0-classified.tsv`: P 80, T 44, R 32, S 4, E 10, N 8 + 6 shell, X 4, D 14 rows at 13
sites). **Rule 8 adds 120 members** (verified 2026-11-08 via `LOOPQ=1 python3 -I
H/scripts/err_loop_question_proto.py <wt> crates/script/elidex-js`, the prototype with rule 8 added, log
`H/logs/err-loop-question-p0a.log`: 99 `for`, 9 `while let`, 5 `loop`, 4 `for` inside `while`, 3 `while`),
all hand-classified **P** (`H/logs/err-loop-question-p0a-classified.tsv`), **none D**, so the `# pending`
ceiling is unchanged. Three reasons: *loop-spec-abrupt* (82: the governing algorithm itself stops at the
first abrupt completion inside the iteration, e.g. the Array / TypedArray HOFs, `Object.assign`,
`Math.max`, `importScripts` step 6), *loop-conversion* (17: argument or init conversion before any durable
write, e.g. `classList.add`, `slot.assign`, the ChildNode mixins, the `*Init` parsers), *loop-graph* (21:
one trigger, one result, recursion over one value graph such as `clone_*`, `internalize`, `flat_into`, or
the bound-callee loop of `call_dispatch`). Total at P0a's base: 322 rows. W population: 5 functions (verified 2026-11-08 via
`python3 -I H/scripts/err_write_before_call_proto.py <wt> crates/script/elidex-js`, log
`H/logs/err-write-before-call-p0.log`). P0a's PR body records the tracked check's own counts at its head.

**Boundary population (rule 9; PM 533 (2)).** Prototype at `02386ec9` (code identical to `ce603c74`:
`git diff --stat ce603c74 02386ec9 -- crates` is empty), verified 2026-11-08 via
`python3 -I H/scripts/err_boundary_census_proto.py <wt>`, log `H/logs/err-boundary-census-p0.log`. The
producer closure over every function reaches 3244 of 4307 names (the name graph over-approximates through
colliding names; rule 10 is the answer). By section, hand-classified:

| Section | Scanned | Members | By class |
|---|---|---|---|
| B1 foreign-trait methods (`ScriptEngine` 7, `HostDriver` 53, `FilterAction` 1, `MutationDispatcher` 1) | 62 | 27 | **D 1** (D-14, `call_listener`); **boundary collapse 3** (BC-1 `eval`, BC-2 `drain_timers`, BC-3 `FilterAction::accept`), all fixed here; R 13 (drains, §4.5); N 9 (collision); P 1 (`with_bound`) |
| B2 `pub` `elidex-js` fns called by the shell (the only dependent crate) | 8 calls | 2 new | 6 are the shell's calls of B1 `HostDriver` methods by name (no new row); N 2: `finish` (collision with `finish` in `compiler/function.rs`, `app/render.rs:296`), `sw_thread_main` (`app/sw_coordinator.rs:214`, a fresh thread's top, rule 5's thread-entry cut) |
| B3 closure / fn-pointer parameters called from `elidex-js` | 21 calls | 21 | N 21: Rust-only closures (`with_range` 7, `with_range_mut` 3, `reconcile_event_handler`'s scripting gate 1, `for_each_shadow_inclusive_descendant` 1 in `custom_elements/lookup.rs:159`, `ws_keepalive` / `es_keepalive` 2, the WebCrypto RNG of `sign` / `generate_key` 2), and 5 name collisions with `sign` (the BigInt `.sign()` in `coerce.rs:23`, `coerce_ops.rs` 58 / 64 / 70, `pools.rs:142`); `range_proto.rs:361` passes a parameter, not a literal, and is reviewed (it borrows the range registry and cannot reach the VM) |
| W across the boundary (rule 7) | 80 calls, from another crate, of a B1 / B2 member's name (8 of them are `finish` collisions) | 3 with a write before the call | **D 1** (`script_dispatch_event_core` → `call_listener`, the write at `:499`: D-14); P 1 (the same write before `run_microtasks` at `:513`: after §4.5 it is the spec's step-2.5 removal of a started call, or already reverted); N 1 (`layout_inline_context_fragmented` → `finish`, the B2 collision) |

New definitive sites: **D-14 only**, fixed in this slice. The boundary collapses BC-1…BC-3 are fixed in this
slice. No new site goes to `# pending`.

Rows at P0a's landing, boundary side: 49 (B1: R 13, N 9, P 1; B2: N 2; B3: N 21; W across: P 2, N 1).
BC-1…BC-3 and `call_listener` are no longer B1 members (their return types are kind-bearing, rule 11);
D-14's W row stays, now admissible: P "the step-2.5 removal is reverted on `NotStarted` (C-4, §4.5)". Err side, the deltas
that §4.5 causes: `engine.rs:275` moves from S to R (it reports and returns the kind);
`script_dispatch_event_core`'s use of `call_listener`'s result becomes a member of class R ("reported by
the engine; C-4 revert here", §4.5); the `elidex-js` uses of the traversal results that now carry
`FilterError::Script` become members where they are not `?`. The exact counts are the tracked check's at
P0a's head (L-a2), not these.

Admissible rows (paths under `crates/script/elidex-js/src/` unless stated; lines at `ce603c74`):

- **R (32):** `engine.rs` 191 / 344; `vm/host/body_mixin.rs:289`; `vm/host/cache/natives.rs` 96 / 121 /
  145 / 198 / 252 / 287 / 340 / 382 / 420 (`finish` rejects); `vm/host/custom_elements/creation.rs:238`,
  `flush.rs` 104 / 183 / 202 / 234, `lookup.rs:183` (report); `vm/host/fetch/mod.rs` 183 / 204;
  `vm/host/observer_common.rs:295` (report; records are delivered from the embedder's entry, top-only);
  `vm/host/readable_stream/controller.rs:135`, `mod.rs` 410 / 639 (stream error / cancel rejection, C-4
  non-re-entry); `vm/host/service_worker/container.rs` 109 / 218; `vm/host/wasm/namespace.rs:578`;
  `vm/natives_generator.rs:472` (wrapper rejection; residual P0 umbrella §6 item 5); `vm/natives_promise.rs` 648 /
  689 / 747; `vm/natives_timer.rs:370`.
- **S (3):** `vm/coerce.rs:246` (`to_display_string`, console display); `vm/host/custom_elements/flush.rs:70`
  (`invoke_one` reports inside); `vm/natives_promise.rs:591` (unhandled-rejection listener, top-only).
  Home: P0 umbrella §6 item 2. **Correction (PM 533 item 3):** the census at `ce603c74` listed
  `engine.rs:275` (`call_listener`'s `let _ =`) here as "dropped, nothing written, top-only". That was
  false: the session crate removed the `once` entry before the call (`event_dispatch/mod.rs:499`), so the
  site is D-14's engine half (§6.3), and "top-only" is the rejected argument (c) of §4.5. After §4.5 the
  site is a match that reports and returns the kind: class R.
- **E (10):** `vm/dispatch_iter.rs` 52 / 309; `vm/ops.rs:60`; `vm/host/typed_array_static.rs` 743 / 801;
  `vm/natives_array_hof.rs:485`; `vm/webidl_sequence.rs` 141 / 149; `vm/host/url_search_params.rs:318`;
  `vm/host/structured_clone.rs:1064`. A `return` method called at the same depth as the failed step fails
  at its own entry the same way, so the C-1 error is what normally survives; the precedence itself is
  P0 umbrella §6 item 1.
- **N (8 + 6):** `vm/ops_element.rs:134` (`to_number` of a String calls no script);
  `vm/interpreter.rs:894` (`drive_async_coroutine` turns every error into a rejection,
  `natives_generator.rs:520-525`); `vm/host/worker_scope.rs:277` (worker loop and the embedder's
  `drain_worker_messages`); `vm/sw_thread.rs` 217 / 474 and `vm/worker_thread.rs` 200 / 227 / 259 (top of a
  fresh thread); shell: `content/event_loop.rs:284`, `content/iframe/render.rs:93`,
  `content/iframe/thread.rs:100`, `lib.rs:310`, `pipeline.rs` 357 / 359 (event-loop top, `EvalResult`
  discarded).
- **X (4):** `engine.rs` 63 / 78 (embedder-test helpers); `vm/host/subtle_crypto/differential.rs` 32 / 36
  (`#[cfg(test)] mod differential;`, `subtle_crypto/mod.rs:62-63`; the tracked check excludes such files,
  §5.2 rule 1).
- **P (80 + 120 by rule 8) and T (44):** listed in `H/logs/err-census-p0-classified.tsv` and
  `H/logs/err-loop-question-p0a-classified.tsv`; T = `vm/dispatch.rs` (40),
  `dispatch_objects.rs` 48 / 80, `dispatch_class.rs:110`, `dispatch_helpers.rs:383`. D-13's row
  (`vm/interpreter.rs` `eval` → `compile_script`) is P "kind-preserving `map_err`" after §6.1.
- **W, admissible (3):** `custom_elements/creation.rs:238` (the `is_value` reset is derived from the
  registry, not from the upgrade's outcome); `html_input_value.rs:353` (a Number to string, no script);
  `natives_promise.rs:591` (top-only).

- **`AbortSignal` abort steps (PM, 11-08; L3's `#11-abort-signal-any-dependent-flattening`; corrected per
  L3's attestation N-6, verified against the code).** *At `ce603c74`:* `abort_signal`
  (`vm/host/abort.rs:522-658`) has two `?`: on `dispatch_vm_simple_event` at `:592`, and on its own
  recursion over the composites at `:610`, inside the `for` loop. Neither can fail today:
  `dispatch_vm_simple_event` → `fire_vm_event` → `fire_vm_event_unchecked` → `dispatch_vm_event`
  (`event_target_dispatch_vm.rs:271-347`, no `?` in its body), whose only return is `Ok` (listener exceptions are absorbed
  inside dispatch), and those two `?` are `abort_signal`'s only error sources, so `:610` can fail only if
  `:592` can. The name graph agrees: `abort_signal` reaches no producer, so it is not in the population and
  rule 8 does not fire on `:610` (it is not among the 120). The shape is **latent definitive**: once P0's
  error can surface through this dispatch (a check or probe on the dispatch path, e.g. a PR-1 walker probe
  in event construction), an `Err` at `:592` would return after the latch and before the composite and
  fetch fan-outs, and one at `:610` after `any_composite_map.remove` (`:607`), so the remaining dependents
  would never abort (C-2 (c)). No input reaches it today. *After L3's PR:* DOM §3.2 *signal abort*
  (`dfn dom 'signal abort'`, for AbortSignal) step 4 fixes every dependent's reason and steps 5–6 run for all
  of them; the loop runs `run_abort_steps` for every entry, keeps the first `Err`, and returns it after the
  loop, with no `?` inside the loop (L3's contract, unchanged). If the call is then in the population, it is
  a member (its result is stored, not propagated in place) of class **P**, reason "deferred propagation:
  the first `Err` is returned after every dependent's abort steps ran (DOM *signal abort* steps 4–6); later
  `Err`s of the same trigger are not surfaced separately, one surfacing per trigger suffices (C-3)". Rule 8
  stays the fail-safe for the class: a `?` put back inside that loop is a member the moment the call joins
  the population. **Which PR writes the row:** L3's PR is expected first (edge, P0 umbrella §1); P0a is
  rebased on it and its allowlist carries the post-L3 row if the tracked check lists the call, keyed on the
  names as they stand at P0a's base (§10 item 4). If L3's PR lands after P0a, P0a ships no row and L3's PR
  adds it if the check asks for it.

**`# pending` (15 rows; this key set is the frozen ceiling of §5.2 rule 4; restated after PM 533 (2):
unchanged, because the one new D, D-14, is fixed in this slice like D-13).** Keys (path, function,
callee, shape):

| Rows | Key | Fixing slice |
|---|---|---|
| D-1 | `vm/host/event_handler_attrs.rs`, `lazy_compile_handler`, `compile_script`, `.ok` | P0b-L3 |
| D-2 | `vm/host/event_handler_attrs.rs`, `lazy_compile_handler`, `run_script`, match | P0b-L3 |
| D-3 | `vm/host/custom_elements/upgrade.rs`, `invoke_upgrade`, `get_property_value`, match | P0b-L3 |
| D-4 | `vm/host/custom_elements/upgrade.rs`, `invoke_upgrade`, `construct_synchronous`, match via binding | P0b-L3 |
| D-5 | `vm/host/event_target_dispatch.rs`, `invoke_listeners_shared`, `call_value`, `.is_err` | P0b-L3 |
| W | `vm/host/custom_elements/upgrade.rs`, `invoke_upgrade` (W population) | P0b-L3 |
| W | `vm/host/event_target_dispatch.rs`, `invoke_listeners_shared` (W population) | P0b-L3 |
| D-6 | `vm/host/structured_serialize.rs`, `structured_serialize_for_storage`, `stringify_for_structured_shortcut`, match | P0b-VM-1 |
| D-7 | `vm/host/structured_serialize.rs`, `structured_deserialize`, `parse_json_str`, `.unwrap_or` | P0b-VM-1 |
| D-8 | `vm/host/indexeddb/value.rs`, `json_to_js`, `parse_json_str`, `.unwrap_or` | P0b-VM-2 |
| D-9 | `vm/host/service_worker/deliver.rs`, `fire_message`, `parse_json_str`, `.unwrap_or` | P0b-VM-2 |
| D-10 | `vm/host/worker_scope.rs`, `serialize_message`, `stringify_for_structured_shortcut`, match | P0b-VM-1 |
| D-11 | `vm/host/worker_scope.rs`, `native_import_scripts`, `compile_script`, `map_err` | P0b-VM-1 |
| D-12 (2) | `vm/host/subtle_crypto/ops.rs`, `normalize_wrap_with_fallback`, `marshal_normalize`, match (`:66`) and other (`:68`) | P0b-VM-2 |

Each P0b slice deletes its rows here and, where its site stays a member after the fix, adds the
admissible row its memo names; no slice adds a pending row (the ceiling).

### §5.5 Controls and mutants

Controls (tempfile fixtures), each red: `census_rejects_unknown_member` (an added `.ok()` on a carrier
call); `census_rejects_stale_row`; `census_rejects_definitive_row` (a D row outside `# pending`);
`census_pending_row_outside_ceiling`; `census_planned_producer_already_derived`;
`census_top_only_reachable_from_native`; `census_run_question_mark_on_carrier` (a new `?` on a carrier in
`run()`); `census_w_function_without_row`; `census_cfg_test_hook_not_producer` (a fixture whose only
producer edge is a `#[cfg(test)]` statement must not derive a producer; red when it does);
`census_loop_question_mark_member` (a fixture with `for x in xs { carrier(x)?; }` and no row → red; the same
`?` outside the loop → auto); for the boundary (rules 7, 9–11): `census_boundary_unit_return_member` (a
fixture `impl ForeignTrait for T` whose method returns `()` and calls a producer, no row → red);
`census_boundary_kind_bearing_auto` (the same method returning `Result<(), ScriptFailure>` → not a member);
`census_boundary_unparsed_impl_member` (an `impl` header the lexer cannot parse → member);
`census_boundary_closure_param_member` (a call passing a closure to another crate's `impl FnMut` parameter,
no row → red; with a class-N row and a closure literal that calls a producer → red);
`census_w_across_boundary` (a fixture in another crate that writes `get::<&mut EventListeners>` and then
calls a boundary member, no row → red); `census_collision_row_other_path` (an N-collision row whose member
still reaches a producer once the named edge is cut → red); `census_kind_bearing_type_without_failure` (a
`KIND_BEARING` entry whose definition does not name `ScriptFailure` → red). `--mutants`
runs the check with each rule disabled and asserts the matching control turns green, so a rule that
stops working is seen (the plan-memo-umbrella-check precedent).

### §5.6 Stated limits and their home

The check does not see: calls through function pointers, trait objects and closures passed across
functions **inside one crate** (the `NativeFn` table, `fire_vm_message_event`'s closure argument is seen only
because its body is in the caller); across crates, foreign-trait impls and closure parameters are the
boundary population (rule 9, PM 533 (2)), and a trait object of a foreign trait is covered through its
impls; writes performed inside a callee before the carrier call (D-1's `take_uncompiled` sat in
`reconcile_event_handler`, another crate); macros; a straight-line `?` that returns before a fan-out over
other subjects (the `:592` shape of §5.4; rule 8 covers a `?` inside the fan-out's loop, not one before
it); durable writes to a side table (the `abort_signal_states` latch) rather than an ECS component, which
the W population's three kinds do not include. Owner VM P4; re-evaluated with PR-1's cut-check limits when
`#11-vm-native-callback-reentry-single-loop` is designed; ledger row P0 umbrella §7 L-7.

## §6. Fixes

### §6.1 D-13: `Vm::eval`

| Id | Site | Carrier | What the error becomes today | Class | Governing step |
|---|---|---|---|---|---|
| D-13 | `vm/interpreter.rs:36` `Vm::eval` | `compile_script` | `VmErrorKind::CompileError` (event-loop top; identity only) | D-cls | HTML §8.1.4.3 steps 10–11 |

Fix: §4.1 item 3.

### §6.2 B-1, B-2: errors that leave `run()` without `throw_error`

| Id | Site | Carrier | Today | Governing step |
|---|---|---|---|---|
| B-1 | `vm/dispatch.rs:91` `Op::PushConst` | `load_constant` | `?` out of `run()`: not catchable, bypasses the escape rule | ECMA-262 §17 paragraph 1 |
| B-2 | `vm/dispatch_objects.rs:227` `op_define_computed_accessor` (`dispatch.rs:842/845`) | `make_property_key` | same | ECMA-262 §17 paragraph 1 |

Fix: `Op::PushConst` matches `load_constant`'s result, and `op_define_computed_accessor` matches
`make_property_key`'s, and each sends an error through `throw_error`, like every other fallible op in
`run()`.

### §6.3 D-14 and BC-1…BC-3: the engine boundary (PM 533 (a))

| Id | Site | What the error becomes today | Class | Governing step |
|---|---|---|---|---|
| D-14 | `ScriptEngine::call_listener` (impl `engine.rs:203`, result dropped at `:275`) × its caller `script_dispatch_event_core` (session `event_dispatch/mod.rs:499` removal, `:503` call, `:517` `remove_listener`) | dropped; the once entry removed before the call stays removed, and its callable is dropped | D-state (W across the boundary, rule 7) | DOM §2.9 *inner invoke* steps 2.5, 2.13 |
| BC-1 | `ScriptEngine::eval` (impl `engine.rs:176`) | a string in `EvalResult`'s `error` field; the kind is gone at the boundary | boundary collapse; no consumer needs the kind today (the shell's are event-loop top, N) | HTML §8.1.4.3 steps 10–11 (via D-13) |
| BC-2 | `ScriptEngine::drain_timers` (impl `engine.rs:332`) | same, per timer | same | — |
| BC-3 | `FilterAction::accept` (impl `node_filter_dispatch.rs:51`) | `FilterError::Throw`; the kind is in `pending_error` | same; the traversal algorithms propagate with `?` and write nothing (DOM §6 *filter* step 6: unset the active flag, rethrow) | DOM §6 *filter* step 6 |

Fix: §4.5. D-14 is fixed here (the trait change is P0a's, PM 533 (a)), so it is never pending; BC-1…BC-3
become kind-bearing and leave the boundary population. The S row `engine.rs:275` of the census at
`ce603c74` was D-14's engine half, not a drop with nothing written (§5.4 correction).

## §7. Cells (written first)

New file `crates/script/elidex-js/src/vm/tests/tests_resource_limit.rs` (NEW). Each cell arms §4.4's
injection with an `n` taken from `resource_limit::count` on a dry run of the same body.

| Cell | Body | Expect |
|---|---|---|
| `rl_identity_predicates` | build each error form | `is_resource_limit` true only for `VmError::resource_limit()`; a `ThrowValue` and a `RangeError` kind are false; `CompileError` kinds likewise; the materialised object is a `RangeError` with `STACK_EXHAUSTED_MSG` |
| `rl_cp1_entry_failure_no_callee_effect` | `let n = 0; function g() { n++ }`; inject `Entry(Call, 1)`; `try { [1].forEach(g) } catch (e) { r = e.name }`; the same through `call`, `call_construct_native`, `run_function`, `construct_synchronous` from Rust | `r === 'RangeError'`, `n === 0`, frame and value-stack depth unchanged; a second run gives `n === 1` |
| `rl_cp2_generator_resumable_after_entry_failure` | `function* g() { yield 1 } const it = g()`; inject `Entry(Resume, 1)`; `it.next()` throws; then `it.next()` | first: `RangeError`; second: `{ value: 1, done: false }` |
| `rl_async_step_entry_failure_rejects_wrapper` | an async function awaiting a resolved promise; inject `Entry(Resume, n)` for the step | the wrapper rejects with `RangeError`; the body's `finally` did not run (P0 umbrella §6 item 5) |
| `rl_cp3a_wasm_call_entry_failure_retry` | an exported function; inject `Entry(WasmCall, 1)` | `RangeError` (not `WebAssembly.RuntimeError`); the next call returns its value |
| `rl_cp3b_instantiate_entry_failure` | inject `Entry(WasmInstantiate, 1)` for `new WebAssembly.Instance(m)`, then for `WebAssembly.instantiate(m)` | sync: `RangeError`; async: rejects `RangeError`; an un-injected retry instantiates |
| `rl_escape_rule_materializes_in_started_frame` | `function f() { [1].forEach(x => x) }`; from Rust `vm.call(f)` with `Entry(Call, 2)`, then with `Entry(Call, 1)` | `n = 2`: `Err` kind `ThrowValue` (a `RangeError` object), not `ResourceLimit`; `n = 1`: kind `ResourceLimit` |
| `rl_vm_eval_compile_resource_limit_kind` | inject `Compile(1)`; `vm.eval("1")`; also through the `ScriptEngine` `eval` of `ElidexJsEngine` | kind `ResourceLimit`; the `EvalResult` message starts with `RangeError` |
| `rl_regexp_literal_error_is_catchable` | `try { /a{2,1}/ } catch (e) { r = e.name }` (a pattern `regress` 0.11.1 rejects; §10 item 1) | `r` is set (today the error leaves `run()` uncaught) |
| `rl_ua_dispatch_once_listener_survives_entry_failure` | through `script_dispatch_event` (the session loop): listeners A (`once`), B (`once`), C on one target; inject `Entry(Call, 1)` for A; dispatch three times | first: A not run, B and C run, A's error reported; second: A runs, then C (A in its original position); third: C only |
| `rl_ua_dispatch_once_listener_threw_is_removed` | A (`once`) throws | A is gone after the dispatch and its callable is dropped (DOM §2.9 step 2.5 stands for a started call) |
| `rl_eval_result_outcome_kind` | through `ScriptEngine::eval`: inject `Compile(1)`; then a script that throws; then `drain_timers` with `Entry(Call, 1)` at the timer callback | `outcome == Err(NotStarted)`; `Err(Threw)`; the timer's `EvalResult` carries `Err(NotStarted)` |
| `rl_tree_walker_filter_entry_failure_kind` | `document.createTreeWalker(root, NodeFilter.SHOW_ELEMENT, { acceptNode() { return 1 } })`; inject `Entry(Call, 1)` at the filter's call; `nextNode()`; then `nextNode()` | first throws `RangeError` and `currentNode` is unchanged; second returns the first element (the active flag was unset) |
| `rl_computed_accessor_key_error_is_catchable` | `try { ({ get [{ toString() { throw 1 } }]() {} }) } catch (e) { r = e }`; and with `Entry(Call, 1)` on the `toString` | `r === 1`; with the injection, `r` is a `RangeError` |

## §8. Mutants (run on the final head, debug)

| Mutant | Change | Dies |
|---|---|---|
| M-ID | `is_resource_limit` always false | `rl_identity_predicates`, `rl_vm_eval_compile_resource_limit_kind` (the P0b slices record their own cells against it) |
| M-ESC | escape rule removed | `rl_escape_rule_materializes_in_started_frame` |
| M-B1 | `Op::PushConst` back to `?` | `rl_regexp_literal_error_is_catchable` |
| M-B2 | `op_define_computed_accessor` back to `?` | `rl_computed_accessor_key_error_is_catchable` |
| M-P-CP1 | CP-1 check after `f(self, mode)` | `rl_cp1_entry_failure_no_callee_effect` |
| M-P-CP1s | CP-1 check after the `entry_frames` snapshot | **equivalent** (the exit path restores the snapshot); recorded, answers R2 S-2 for CP-1 |
| M-P-CP2 | CP-2 check after the generator is marked Running | `rl_cp2_generator_resumable_after_entry_failure` |
| M-P-CP3a / M-P-CP3b | CP-3 checks after the `HostState` bind / after `Store::new` and the start function | `rl_cp3a_*` / `rl_cp3b_*` if observable; a survivor is recorded as equivalent with the reason (§10 item 2) |
| M-EV | `Vm::eval` maps to `CompileError` | `rl_vm_eval_compile_resource_limit_kind` |
| M-D14a | no reinsert on `NotStarted` | `rl_ua_dispatch_once_listener_survives_entry_failure` |
| M-D14b | reinsert after `run_microtasks` | **equivalent** today: the checkpoint after a call that never started finds an empty queue (the previous listener's checkpoint drained it, and nothing ran since); recorded, the order of §4.5 item 2 stays |
| M-D14c | reinsert at the end of the list | `rl_ua_dispatch_once_listener_survives_entry_failure` (second dispatch: C before A) |
| M-D14d | `remove_listener` still called on `NotStarted` | same (A's callable is gone; the second dispatch cannot run it) |
| M-OUT | `script_failure` always `Threw` | `rl_eval_result_outcome_kind`, `rl_ua_dispatch_once_listener_survives_entry_failure` |
| M-CC | each §5.2 rule (and the ceiling, the `#[cfg(test)]` statement exclusion) disabled | the matching control (§5.5) |

## §9. Landing obligations

What P0a's PR (or, where marked, the PM / PMO at landing) does. This memo edits no ledger. Program-level
obligations (PR-1 precondition, breadth record, slots) are P0 umbrella §7.

The ledger and notice rows this slice triggers are rows of the P0 program's landing list (P0 umbrella §7,
PM 513 (d)): L-2, L-3, L-4, L-5, L-6, L-7 (census limits, formerly L-a1) and L-12 (L3 notice). Slice-local:

- **L-a2. Counts and ceiling**: P0a's PR body records the tracked check's counts per class at its head
  (rule 8's members and the boundary population, rule 9, separately) and the 15 `# pending` keys of §5.4.

## §10. Unverified facts (each decided at implementation, before its cell is written)

1. The `regress` 0.11.1 pattern for `rl_regexp_literal_error_is_catchable`.
2. Whether M-P-CP3a / M-P-CP3b are observable.
3. Every N row claiming top-only is re-checked by §5.2 rule 5 at P0a's base.
4. The post-L3 function and callee names of the `AbortSignal` abort loop (§5.4) and of cell (o), read at
   P0a's base once L3's `#11-abort-signal-any-dependent-flattening` PR has landed (its plan Rev 2 is
   `a8c4179e`, Rev 2.1 due 11-09; neither is on this branch), and whether the tracked check lists the call.
5. The test-only mock `ScriptEngine` that `#11-event-path-window-missing` plans
   (`docs/plans/2026-11-event-path-window.md` on main, `event_dispatch/test_support.rs`, NEW there) and that
   umbrella's edits to `script_dispatch_event_core`: whichever PR lands second adapts to the other (the
   mock implements `call_listener -> Result<(), ScriptFailure>`; the revert of §4.5 stays per listener).
   P0a's PR body records which order happened (P0 umbrella §1).
6. Whether the stderr report of a `Threw` listener (§4.5, engine side) is observable in a cell; if not, it
   is recorded as not pinned (the report channel is `#11-embedder-diagnostic-channel`'s).

webref lookups behind §3 (2026-11-08): `body ecma262 sec-error-handling-and-language-extensions`;
`dfn html 'create a classic script'` → §8.1.4.3; `dfn dom 'inner invoke'` → §2.9 (steps read by `body dom
concept-event-listener-inner-invoke`); `dfn dom 'remove an event listener'` → §2.7 (step 2 sets removed);
`dfn dom 'filter'` → §6 `#concept-node-filter` (step 6 read by `body dom concept-node-filter`).
