# VM stack-safety guard — P0a-1 plan (contract semantics: error identity, the entered mark, escape rule, choke-point placement, the engine boundary)

Status: **plan (pre-implementation)**, per-PR slice of the [P0 umbrella](2026-11-vm-stack-safety-guard-p0.md)
(PM 509; re-sliced from P0a by PM 544). `/elidex-plan-review` required before code. Owner: VM P4. Base:
`ce603c74`; every code anchor below is at that commit (the branch head `d7ec4761` has identical code:
`git diff --stat ce603c74 d7ec4761 -- crates` is empty). Provenance root `H` is the P0 umbrella's. The
sibling slice [P0a-2](2026-11-vm-stack-safety-guard-p0a2.md) is the census checker; it is written against
this memo's contract and lands after it (P0 umbrella §1).

The contract C-1…C-5 is the P0 umbrella §4 (canonical); this memo does not restate it. Edges, schedule
and the other slices: P0 umbrella §1. The census rules, the allowlist and `# pending`: P0a-2 §5–§7.

## §0. Charter

P0a-1 gives the P0 contract its semantics in the VM: the error identity (§4.1), the **entered mark**
that decides C-1's "did not start" at every entry and the escape rule restated on it (§4.2, PM 544),
the placement of the four choke-point checks (§4.3), the test-only injection the consumer slices use
(§4.4), the engine boundary's failure kind `ScriptFailure` and the trait changes that carry it (§4.5,
PM 533 (a)), and the fixes D-13, B-1, B-2, D-14, D-15 and BC-1…BC-3 (§5). D-15 is the third site of
D-14's shape (a `once` removal followed by a dropped call result), found by R4 Axis 2 IMP-2 (PM 544).

**Why the mark (PM 544).** R4 Axis 2 IMP-1: the P0a design inferred "did it start" on the error side,
inside `throw_error`, which only a *script* frame reaches. A native body is a started step too, and it
propagates a nested entry's failure with `?`; at the outermost entry no `run()` sits above it, so the kind
stayed `ResourceLimit` after user code had run, and the once-revert would have re-run a half-run step
(D-P0-2's forbidden case). "Did user code take even one step" is a fact the side that received the entry
holds; inferring it later on the error path creates an exception for every path through a native callback
(Security by structure, CLAUDE.md). So the entry records it.

## §1. Scope

**In.** `crates/script/elidex-js/src/vm/error.rs`, `compiler/mod.rs` (`CompileError` kind),
`vm/resource_limit.rs` (NEW: `check_entry`, `exit`, the injection), `vm/interpreter.rs` (CP-1 hook, the
entered mark in `with_call_mode`, `call_internal`, `push_js_call_frame`, `call_dispatch`; `Vm::eval`
mapping), `vm/natives_generator.rs` (CP-2 hook and the mark at the take), `vm/host/wasm/exported_func.rs`
and `vm/host/wasm/instance.rs` (CP-3a / CP-3b hooks, the mark, the named conversion), `vm/dispatch.rs` and
`vm/dispatch_objects.rs` (B-1, B-2 routing); `vm/natives_promise.rs` (D-15, §5.4); the injection hooks in
`compile_script`, `parse_json_str`, `stringify_for_structured_shortcut` and `dispatch_vm_simple_event`
(§4.4), with the removal of L3's `fail_next_abort_dispatch` seam and the move of its cell onto that hook
(§4.4; L3's files, site list via PM, P0 umbrella §7 L-12); cells in
`crates/script/elidex-js/src/vm/tests/tests_resource_limit.rs` (NEW). For §4.5: in
`crates/script/elidex-script-session/src/`, `outcome.rs` (NEW), `engine.rs` (`ScriptEngine`, `EvalResult`),
`event_dispatch/mod.rs` (`script_dispatch_event_core`), `event_listener.rs` (`remove_with_position`,
`reinsert`), `lib.rs` (re-export); `crates/script/elidex-js/src/engine.rs` (`eval`, `call_listener`,
`drain_timers`); `crates/dom/elidex-dom-api/src/traversal.rs` (`FilterError`) and its `elidex-js` users
(`vm/host/node_filter_dispatch.rs`, `tree_walker_proto.rs`, `node_iterator_proto.rs`); the shell's
`EvalResult` readers (`lib.rs`, `pipeline.rs`, `content/event_loop.rs`, `content/iframe/render.rs`,
`content/iframe/thread.rs`) and the tests that read `EvalResult`'s `success` field or call `call_listener`.
The session-crate and `elidex-dom-api` files are not VM P4's; the PM routes the notice (P0 umbrella §7 L-14).

**Out.** The census checker, its allowlist, `# pending`, the controls and mutants of the check (P0a-2);
the consumer fixes D-1…D-12 (P0b-VM-1, P0b-VM-2, P0b-L3; P0 umbrella §1); `native_import_scripts`'s kind
mapping (D-11, P0b-VM-1) although §4.1 item 3 states the rule; the deviations of P0 umbrella §6.

## §2. Coupled invariants

Elements: **ID** (error kinds, predicates, `Vm::eval` boundary; §4.1), **ENT** (the entered mark; §4.2),
**ER** (escape rule, restated on ENT; §4.2), **B** (B-1 / B-2 routing; §5.2), **CP** (choke-point
placement; §4.3), **INJ** (test-only injection; §4.4), **OUT** (the boundary's `ScriptFailure` and the
once revert; §4.5). The census-side pairs (ID × CEN, CP × CEN, INJ × CEN, OUT × CEN, OUT × PEND) are
P0a-2 §2.

| Pair | Intersection | Pinned by | Basis |
|---|---|---|---|
| ID × ENT | the only producer of `ResourceLimit` inside an entry before its first effect is `check_entry`; everything after the first effect is behind the mark | `rl_not_entered_bound_chain_is_not_started` (bound-chain unwrapping is not entering) | cell |
| ENT × ER | ER is applied at the entry's exit from the mark, not from "passed `throw_error`": an error that leaves an entered entry as `ResourceLimit` is rewritten to `ThrowValue` with the same message | `rl_entered_bound_native_listener_is_threw`, `rl_escape_rule_materializes_in_started_frame` | cell |
| ENT × ENT (nesting) | the mark is per entry: a nested entry saves the outer value, works on its own, restores on every exit (Ok, Err, panic), so a nested entry that did not enter cannot clear the outer's mark and one that entered cannot set it | `rl_entered_mark_survives_non_entering_nested_entry`; M-ENT-R | cell |
| ENT × CP | the check precedes the reset of the mark; a failed check returns before touching it, so the outer entry's mark is unchanged and the failed entry's kind is exact | `rl_cp1_entry_failure_no_callee_effect` (frame and mark unchanged) | cell |
| ENT × I-5 | one `bool` saved and reset per entry, one store at the first effect, one compare at the exit; the store at `push_js_call_frame` is the only addition to the plain JS→JS call path | PR-1 §10's I-5 bench (umbrella I-5) | measured (PR-1) |
| ID × CP | each check returns `VmError::resource_limit()`; CP-3b alone materialises it (`vm_error_to_thrown`), at a C-3 channel | `rl_cp3b_instantiate_entry_failure`, `rl_cp3a_wasm_call_entry_failure_retry` | cell |
| ID × INJ | every injection point delivers exactly C-1's kind (`VmError::resource_limit()` / `CompileErrorKind::ResourceLimit`) | `rl_identity_predicates`, `rl_vm_eval_compile_resource_limit_kind` | cell |
| ER × B | B-1 / B-2 are catchability fixes (ECMA-262 §17 paragraph 1: the error surfaces where the construct is evaluated), no longer ER's soundness condition: a `?` out of `run()` still leaves an entered entry and is rewritten at its exit | `rl_regexp_literal_error_is_catchable`, `rl_computed_accessor_key_error_is_catchable`; M-B1, M-B2 | cell |
| ER × CP | a CP-1 failure before any frame stays `ResourceLimit` (n = 1); one inside a started frame comes out `ThrowValue` (n = 2) | `rl_escape_rule_materializes_in_started_frame` | cell |
| CP × INJ | at P0a-1 the body of `check_entry` is the injection only; `resource_limit::count` counts checks per kind, from which every placement cell takes its `n` | `rl_cp1_*`, `rl_cp2_*`, `rl_cp3*` | cell |
| OUT × ID | `NotStarted` ⇔ `is_resource_limit()` at the boundary; one mapping function, so no boundary can classify on its own | `rl_eval_result_outcome_kind` | cell |
| OUT × ENT | a `NotStarted` at the boundary means the one entry the step performed entered nothing: no frame, no native body, so no script ran, no microtask was queued by the step, no listener list was touched; the revert is exact and the reinsert index is exact | `rl_ua_dispatch_once_listener_survives_entry_failure`, `rl_entered_bound_native_listener_is_threw` (the complement: a started step is `Threw`, the removal stands, the body is not re-run) | cell |
| OUT × INJ | the UA dispatch path is reached through `script_dispatch_event` with `Entry(Call, n)`; no new injection point | same cells | cell |
| OUT × D-15 | the third once loop (`dispatch_unhandled_rejection_event`) reverts with the same two methods and the same mark | `rl_unhandledrejection_once_listener_survives_entry_failure` | cell |
| INJ × L3 seam | one test-failure mechanism: `SimpleEventDispatch(n)` replaces L3's `fail_next_abort_dispatch`, and as a `#[cfg(test)]` statement it does not make `dispatch_vm_simple_event` a producer (P0a-2 rule 1) | cell (o) after the move; P0a-2 control `census_cfg_test_hook_not_producer` | cell + check |

## §3. Spec coverage map

No spec defines a resource limit (P0 umbrella §3). The rows are the clauses this slice's code answers to.

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| ECMA-262 §17 Error Handling and Language Extensions | paragraph 1 (errors reported when the construct is evaluated), paragraph 3 (no other kinds of errors as early errors) | a resource limit is neither an early error nor a specified error; it surfaces where the construct is evaluated; B-1 / B-2 errors become catchable where evaluated | `VmErrorKind::ResourceLimit` (NEW), `CompileErrorKind::ResourceLimit` (NEW), `Op::PushConst`, `op_define_computed_accessor` | ✓ | yes |
| WHATWG HTML §8.1.4.3 Creating scripts | *create a classic script* steps 10–11 (ParseScript; a list of errors becomes the error to rethrow) | a compile-time resource limit is not a parse error | `Vm::eval` mapping (`interpreter.rs:36`); `EvalResult::outcome` (BC-1) | ✓ | yes |
| WHATWG DOM §2.9 Dispatching events | *inner invoke* step 2 (listeners whose removed is false), 2.5 (once: remove before the call), 2.13 (call; on a throw, report and continue) | `NotStarted`: revert 2.5 (re-entry subject, C-4); `Threw`: 2.5 stands, 2.13.1 report | `script_dispatch_event_core` (D-14), `call_listener`; `dispatch_unhandled_rejection_event` (D-15) | ✓ | yes |
| WHATWG HTML §8.1.4.7 Unhandled promise rejections | *notify about rejected promises* step 4.1.2 (fire `unhandledrejection` at the global) | the firing is a DOM §2.9 dispatch; D-15's loop is its elidex form (target: the Document, `#11-unhandledrejection-dispatch-target`) | `dispatch_unhandled_rejection_event` (`natives_promise.rs:565`, `:591`) | ✓ | yes |
| WHATWG DOM §6 Traversal | *filter* step 6 (on a throw: unset the active flag, rethrow) | every failure propagates, nothing written; the kind crosses as `FilterError::Script` (NEW: P0a-1) | `FilterAction::accept` (BC-3) | ✓ | yes |

**Breadth**: K=3 specs (ECMA-262, WHATWG HTML, WHATWG DOM), M=5 entries (verified 2026-11-08 by `python3
.claude/skills/elidex-plan-review/preflight.py` on this memo; rc and K / M as printed in the PR body, P0
umbrella §7 L-13). Below the split thresholds.

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

### §4.2 The entered mark and the escape rule (PM 544)

**The fact.** C-1 says the error means "the step the probe guards did not start". At each of the four
entries (§4.3) that fact is recorded by the entry, not inferred later: `VmInner::entry_entered: bool`
(NEW), the **entered mark** of the innermost entry in progress.

**Where it lives.** One field on `VmInner`, used as a per-entry value by the save / reset / restore
discipline `with_call_mode` already applies to `gc_enabled`, `active_bound_key` and `completion_value`
(`interpreter.rs:166-276`): after the check passes, the entry saves the outer value and resets the field to
`false`; on every exit (Ok, Err, panic) it reads its own value and restores the outer's. A nested entry
therefore sees and changes only its own mark (§2 ENT × ENT). The field is not GC-visible state and needs no
root.

**Who sets it (the first effect of the callee).** The function that performs the callee's first effect
sets `entry_entered = true` as its first statement:

| Site | First effect | Why here |
|---|---|---|
| `call_internal` (`interpreter.rs:533`) | the frame push (`self.frames.push`, after the `is_class_ctor` guard) | every `Function`-kind call from an entry (`call_dispatch` `Function` arm, `interpreter.rs:296`) and from a native (`ctx.call_function` → `Vm::call`, `native_context.rs:85`) builds its frame here |
| `push_js_call_frame` (`interpreter.rs:768`) | the frame push | the construct path (`construct_synchronous`'s closure, `dispatch_class.rs:323`; `do_new`) and every plain JS→JS call build their frame here; inside an already-entered entry the store is idempotent |
| `call_dispatch` `NativeFunction` arm (`interpreter.rs:312`) | before `func(&mut ctx, ..)` | a native body is a started step: it may call back into script, write VM state and propagate a nested failure with `?` |
| `call_dispatch` `PromiseResolver`, `PromiseCombinatorStep`, `PromiseFinallyStep`, `ReadableStreamStartStep` / `PullStep` / `CancelStep` arms | before the arm's body call (`settle_promise`, `step_combinator`, `run_finally_step`, `run_*_step`) | VM-internal native bodies with effects of their own |
| `call_dispatch` `AsyncDriverStep` and `WasmExportedFunction` arms | **not set in the arm**: the body contains its own entry check (`resume_generator` CP-2 via `drive_async_coroutine`; `call_wasm_exported_function` CP-3a), which must still be able to fail as "not started" | the mark is set inside, after that check |
| `resume_generator` (`natives_generator.rs:114`) | the take of the suspended frame (status → Running) | from there the generator's state has changed; a `Return` completion with no `finally` marks Completed directly, also an effect |
| `call_wasm_exported_function` (`host/wasm/exported_func.rs:268`) | after `check_entry(WasmCall)`, before the `HostState` bind / `set_fuel` | store mutation is the first effect (§4.3) |
| `instantiate_module` (`host/wasm/instance.rs:151`) | after `check_entry(WasmInstantiate)`, before the lazy `vm_wasm_runtime()` init / `Store::new` | runtime init is the first effect (§4.3) |

`call_dispatch`'s failures before an arm's body (the `ConstructorOnly` / `IllegalConstructor` TypeErrors,
the bind-chain `RangeError`, "not a function") happen with the mark unset; none of them is C-1's kind, and
unwrapping a `BoundFunction` chain is not entering (§2 ID × ENT).

**How `NotStarted` / `Threw` is computed (the escape rule, restated).** `resource_limit::exit` (NEW) is
called at the exit of each of the four entries, with the entry's own mark and the result:

- mark unset: the result is returned as it is — a `ResourceLimit` error leaves as `ResourceLimit`
  (nothing of the callee ran: `NotStarted` at any boundary that maps it);
- mark set: a `ResourceLimit` error is rewritten to `VmError { kind: ThrowValue(thrown), message }` with the
  same message, `thrown` from `vm_error_to_thrown` (a `RangeError` object): the step started, so the error
  is that step's exception (`Threw`; D-P0-2). Any other result passes.

`throw_error` (`vm/ops.rs:155`) is unchanged: it materialises the error for the handler search inside the
activation and, uncaught, returns the original error (`ops.rs:165`); the rewrite happens once, at the
entry's exit (one rule, one place). Consequences: (i) an error that is still `ResourceLimit` when a
consumer sees it left the consumer's own entry without any frame or native body having started: "did not
start" is exact; (ii) a failure raised inside a constructor, a getter, a callback body **or a native body**
that ran reaches the consumer as that body's exception and C-4's spec branch applies (D-P0-2, PM 509 item
7); (iii) the IMP-1 case: `t.addEventListener('x', [].forEach.bind([1, 2], h), {once: true})` — the entry
unwraps the bound function, enters native `forEach` (mark set), `forEach` calls `h` (a nested entry: `h`
runs, its mark is its own), calls `h` again and that nested check fails (`NotStarted` for the nested entry,
`?` out of `forEach`), and the outer exit sees its mark set and returns `ThrowValue`: `Threw` at
`call_listener`, the removal stands, `h` is not re-run.

**Boundaries with more than one entry per step.** `accept` on `JsFilter` performs two entries (`pick_callable`
may run an `acceptNode` getter, then the call); `call_listener` performs one (`ensure_event_handler_current`
is a compile, not an entry; after P0b-L3 its failure returns `NotStarted` without calling, hand-off §4).
The kind at the boundary is the failing entry's; earlier entries completed. For BC-3 the dom-api consumer
writes nothing, so the reading is harmless; for D-14 / D-15 / D-5 there is one entry per step, so the
revert is exact.

**Cost (I-5).** Per entry: one `bool` save, one reset, one compare at the exit, beside the three fields
`with_call_mode` already saves. One store at each first-effect site; the store in `push_js_call_frame` is
the only addition to the plain JS→JS call path (a store to a `VmInner` field already in cache). PR-1 §10's
I-5 bench covers `with_call_mode` and the call path; no separate measurement.

**Interaction with CP placement (§4.3).** The order inside an entry is fixed: `check_entry(kind)?` first,
then save / reset of the mark, then the body, then `exit`. A failed check returns before the reset, so the
outer entry's mark is untouched (§2 ENT × CP). The mark does not move the checks: CP-1…CP-3b stay where
§4.3 puts them, and the mark's first-effect sites are *after* them by construction (the check is the first
statement; the first effect is inside the body).

### §4.3 The choke-point check: placement (I-3, VM half)

`crate::vm::resource_limit::check_entry` (NEW) takes an `EntryKind` (NEW: `Call`, `Resume`, `WasmCall`,
`WasmInstantiate`) and returns `Result<(), VmError>`. P0a-1 places it; PR-1 adds `ensure_stack(ask)` for
`Call` / `Resume` inside it, PR-1b for `WasmCall` / `WasmInstantiate`. The function is the one place where
the ask per kind is chosen. At P0a-1 its body is the `#[cfg(test)]` injection of §4.4 only.

| Id | Site | Placement | What must not have happened yet | Why it holds |
|---|---|---|---|---|
| CP-1 | `VmInner::with_call_mode` (`interpreter.rs:160`) | first statement | the callee's first effect | `with_call_mode` restores frames, value stack, `gc_enabled`, `active_bound_key` and `completion_value` on every exit (`interpreter.rs:166-276`), so any position before `f(self, mode)` is equivalent; the check must precede `f` |
| CP-2 | `VmInner::resume_generator` (`natives_generator.rs:114`) | first statement | the generator taken and marked Running | a Running generator rejects the next resume ("already running") |
| CP-3a | `call_wasm_exported_function` (`host/wasm/exported_func.rs:268`) | before the `HostState` bind and `set_fuel` | store / instance mutation | PR-1b §2 |
| CP-3b | `instantiate_module` (`host/wasm/instance.rs:151`) | first statement, before the lazy `vm_wasm_runtime()` init, `Store::new` and the start function | runtime init, store creation, start-function effects | PR-1b §2 |

VM entry functions: `call` (`interpreter.rs:76`), `call_construct_native` (`:96`), `run_function`
(`:1004`), `construct_synchronous` (`dispatch_class.rs:273`, `with_call_mode` at `:323`) and
`dispatch_super_inner` mutate nothing before `with_call_mode` (R1 Axis 2, verified at `47b6c934`;
re-checked at P0a-1's base by `rl_cp1_entry_failure_no_callee_effect`, whose body enters through each of the
first four). `with_call_mode` has exactly these production callers (`git grep -n 'with_call_mode(' --
crates/script/elidex-js/src` minus tests: `interpreter.rs` 82 / 102 / 1010, `dispatch_class.rs:323`;
verified 2026-11-08).

**CP-3 conversions (named).** `call_wasm_exported_function` returns `Result<_, VmError>` and propagates the
check with `?`. `instantiate_module` returns `Result<ObjectId, JsValue>`; the check's error converts by
`.map_err(|e| ctx.vm.vm_error_to_thrown(&e))`, the in-file precedent (`instance.rs:162-163`,
`coerce_import_object`), never through `wasm_error_to_js_value` (which would make it
`WebAssembly.RuntimeError`). Its two consumers propagate (`instance.rs:69`) or reject (`namespace.rs:578`);
both are C-3 channels, so the materialised form is admissible there. `exit` runs before the conversion.

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
sizes. Each boundary hook is a `#[cfg(test)]`-attributed statement (P0a-2 rule 1 excludes such statements
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
kind, which is also the only `Err` that can reach this call once P0's error exists (P0a-2 §7). **Inverted
order (P0a-1 first):** there is no seam to move; PR-A arms `InjectAt::SimpleEventDispatch(n)` from the
start and adds no field. Either way L3's contract is unchanged, and after both PRs exactly one seam exists.
P0 umbrella §1 points here.

### §4.5 The engine boundary keeps the kind (PM 533 (a))

**Defect (R3 Axis 1 IMP-1, verified at `02386ec9`).** `script_dispatch_event_core`
(`crates/script/elidex-script-session/src/event_dispatch/mod.rs:493-518`) removes a `once` entry from the
`EventListeners` component (`:499`) and then calls `ScriptEngine::call_listener` (`:503`), which returns
`()`; the VM's implementation drops the call's result (`crates/script/elidex-js/src/engine.rs:275`,
`let _ = g.call(..)`). The engine-independent crate holds the subject but cannot see the kind, so it cannot
apply C-4: a once listener whose call never started is consumed (C-2 (c)). This is **D-14** (§5.3).

**The type.** `elidex_script_session::ScriptFailure` (NEW: P0a-1, `crates/script/elidex-script-session/src/outcome.rs`,
re-exported at the crate root):

| Variant | Meaning (engine-independent) | Produced when (engine side) |
|---|---|---|
| `Threw` | the step started and ended with an exception (including a parse error reported by the step) | the engine's error is not `is_resource_limit()` after the entry's `exit` (§4.2) |
| `NotStarted` | the step did not start; nothing of it ran (C-1) | `VmError::is_resource_limit()` / `CompileError::is_resource_limit()` |

It is the failure half only: each boundary keeps its own success type (`()`, `NodeFilterResult`, a compile
verdict) and returns `Result<T, ScriptFailure>`. It carries no value: a JS value cannot cross into an
engine-independent crate, so the engine surfaces the error itself (C-3: report, throw or rejection) and the
crate across the boundary uses only the kind, to apply C-2 / C-4 to the subjects it holds. **Home**:
`elidex-script-session`, the sole Script↔ECS boundary (CLAUDE.md, Design philosophy); every crate that
names it already depends on it (`elidex-dom-api` → `elidex-script-session`, `Cargo.toml:16`; the shell;
`elidex-js`). Not `elidex-plugin` (no plugin trait carries a script outcome) and not a VM type (the Layering
mandate keeps the caller engine-independent). The mapping `VmError` → `ScriptFailure` is one function on
the engine side (`VmError::script_failure` (NEW: P0a-1), `vm/error.rs`), keyed on `is_resource_limit()`,
the only test (P0 umbrella §4 "Applying it"); its exactness is §4.2's.

**Enumeration (every method of a trait declared outside `elidex-js` and implemented in it; census B1,
P0a-2 §7).**

| Boundary (declaration; impl) | Today | P0a-1 |
|---|---|---|
| `ScriptEngine::call_listener` (session `engine.rs:78`; impl `elidex-js/src/engine.rs:203`) | `()`; the impl drops the result (`:275`); the caller writes before the call | `-> Result<(), ScriptFailure>`, `#[must_use]`; D-14 (§5.3) |
| `ScriptEngine::eval` (`:59`; impl `:176`) | `EvalResult { success: bool, error: Option<String> }`: the kind becomes a string | `EvalResult { outcome: Result<(), ScriptFailure>, error: Option<String> }`; `success` is removed, no shim; BC-1 |
| `ScriptEngine::drain_timers` (`:101`; impl `:332`) | `Vec<EvalResult>` | each `EvalResult` carries the outcome (BC-1's type); the container is kind-bearing by containment (P0a-2 rule 11); BC-2 |
| `FilterAction::accept` (`crates/dom/elidex-dom-api/src/traversal.rs:401`; impl `vm/host/node_filter_dispatch.rs:51`) | `Err(FilterError::Throw)`; the kind travels out of band in `JsFilter`'s `pending_error` field | `FilterError::Throw` → `FilterError::Script(ScriptFailure)` (NEW: P0a-1); the value stays in `pending_error`; BC-3, with the null arms removed (below) |
| `ScriptEngine`: `run_microtasks`, `drain_reactions`; `HostDriver`: `deliver_mutation_records`, `deliver_resize_observations`, `deliver_intersection_observations`, `tick_network`, `deliver_sw_client_update`, `deliver_idb_versionchange`, `drain_worker_messages`, `deliver_history_step_events`, `deliver_media_query_changes`, `deliver_visual_viewport_events` (12) | `()` | unchanged; class R "drain": the method has no step of its own, each item's failure is consumed on that item's own channel inside the engine at a site that is its own Err-population row, and no caller holds a subject across the call; **the consuming rows are named per drain and checked** (PM 540 (3); P0a-2 §5 reason `drain-consumed`) |
| `HostDriver::set_history_state` (`:469`; impl `:605`) | `()` | unchanged; **not a drain** (R4 Axis 1 MIN-3): it has a step of its own (`seed_history_state` → `structured_deserialize` → `parse_json_str`). Class N top-only, the reason P0b-VM-1 §5.1 gives its inner chain: the embedder's document-construction entry, HTML §7.4.6.2 *restore the history object state* step 2's catch-null is the spec branch at a non-re-entry subject; the consuming row the checker names is `structured_deserialize` → `parse_json_str` (D-7 until P0b-VM-1, then the N rows `seed_history_state` → `structured_deserialize`) |
| `HostDriver::bind`, `unbind`, `teardown_document`, `seed_sw_client`, `take_pending_storage_changes`, `sw_controller_scope`, `storage_origin_key`, `install_network_handle`, `install_idb_backend` (9) | `()` / a value | unchanged; class N "name-graph collision": no script runs; the name graph reaches a producer only through a colliding name (`new`, `to_string`, `bind`); the cut is keyed per call occurrence (P0a-2 rule 10) |
| `HostDriver::with_bound` | `R`, the closure's value | unchanged; class P "pass-through": it returns the closure's own value |
| `ScriptEngine`'s `remove_listener`, `bound_dom_mut`; the other 32 `HostDriver` methods; `MutationDispatcher::dispatch` (`Send + Sync`, no VM); the std-trait impls (`Drop`, `Iterator`, `Default`, …) | — | members or not as P0a-2's resolved graph decides (rule 9 scans every foreign trait, std included); none runs script (R4 Axis 1 MIN-2 read the 7 std impls at the base) |

No other implementor exists in non-test code (`git grep -nE 'impl(<[^>]*>)? (\w+::)*(ScriptEngine|HostDriver) for' -- crates`
→ `elidex-js/src/engine.rs:175`, `:373` only, verified 2026-11-08). Test implementors and test callers adapt
in P0a-1's PR (the tests of `elidex-js` that read `EvalResult`'s `success` field or ignore `call_listener`).
**Order with `#11-event-path-window-missing` (PM 540 (2)):** P0a-1 lands first; that umbrella's PR-A (the
mock `ScriptEngine` in `event_dispatch/test_support.rs` and its edits to `script_dispatch_event_core`) is
written against this signature (`call_listener -> Result<(), ScriptFailure>`; the revert of this section
stays per listener). The boundary's type is the contract, so the later PR adapts, not the contract. If
P0a-1 is late for PR-A's schedule, the PM decides (§9 item 5).

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

1. takes the entry out with `EventListeners::remove_with_position` (NEW: P0a-1; it returns the entry and its
   index) at step 2.5, as today's removal;
2. on `NotStarted`, puts it back with `EventListeners::reinsert` (NEW: P0a-1) **at that index**, with the
   same options, **before** the step's microtask checkpoint (`run_microtasks`, `:513`), the first point
   after the failed call where script can run, so no script observes the removal;
3. skips the engine-side `remove_listener` (`:517`), so the callable and its `AbortSignal` back-ref stay;
4. does not invoke it again in this dispatch (the contract retries nothing, P0 umbrella §4) and continues
   with the next listener, as step 2.13 does after a reported exception.

On `Threw`, step 2.5 stands (the step started): the entry stays removed and `remove_listener` runs, as
today. A non-`once` listener has nothing to revert.

**Why the revert is exact (re-derived from the mark, PM 544).** `NotStarted` means the one entry the
step performed entered nothing (§4.2): no frame was pushed and no native body began, so between the removal
(step 2.5) and the reinsert no script ran, no microtask was queued by the step, no `addEventListener` /
`removeEventListener` / abort could touch the list, and the dispatch plan is a snapshot
(`build_dispatch_plan`). Hence: the index from `remove_with_position` is still the entry's position
(**reinsert-index validity**); nothing is re-executed by the later dispatch (D-P0-2 holds: a step that had
started arrives as `Threw`, §4.2 (iii)); and **M-D14b's "equivalent" claim** (reinsert after the checkpoint
finds the same state) holds under the mark — the checkpoint after a `NotStarted` call drains an empty queue
and an empty rejection list (the previous listener's checkpoint drained them, and nothing ran since) — but
only under the mark: under the P0a reading it was false (a native callback could have queued a microtask,
R4 Axis 2 IMP-1 (3)). The order of item 2 stays and the mutant stays recorded (§7). A debug assertion
checks the entry is absent at reinsertion.

**Which "removed" the revert writes (R4 Axis 2 MIN-1).** `ListenerEntry` has no `removed` field; elidex
represents DOM §2.7 *remove an event listener* step 2's flag by absence in two stores: membership of the
`EventListeners` component (the ECS fact, written by the session crate at step 2.5 and the SoT) and the
engine's `listener_store` (the callable, a per-VM identity handle: CLAUDE.md side-store exception (a)),
emptied by `remove_listener` after the call. The revert writes the SoT (`reinsert`) and leaves the store
untouched (item 3). If `remove_with_position` returns `None` (the entry is no longer in the component — an
earlier listener of the same dispatch removed it), the engine finds no callable at step 1 and returns
`Ok(())`, so `None` and `NotStarted` cannot co-occur while the two stores are coherent; a `None` with
`Err(NotStarted)` would mean a callable without a component entry and is a debug assertion (the revert is a
no-op, the failure is reported; nothing is reinserted, because the SoT says removed).

**The one revert API (R4 Axis 1 MIN-1).** `remove_with_position` / `reinsert` are the one revert API of
the program; three loops implement DOM §2.9 *inner invoke* in elidex — the session loop
(`script_dispatch_event_core`, D-14, this memo), the VM dispatch loop (`invoke_listeners_shared`, D-5,
P0b-L3, hand-off §4) and the `unhandledrejection` loop (`dispatch_unhandled_rejection_event`, D-15, §5.4,
this memo) — and each keys its revert on the same mark through the same `ScriptFailure` / `is_resource_limit`
test. The revert *decision* (reinsert at the index before the checkpoint, skip the store prune, no
re-invoke, the debug assertion) is therefore written in three places; that duplication is the registered
deviation `#11-event-dispatch-inner-invoke-dual-implementation` (VM P4, re-eval 2026-12-15; its text says
two loops — the third is a PM item of this revision). Whether the decision becomes one session-crate helper
all three loops call is that slot's decision, not this slice's.

**BC-3's failure → null arms (R4 Axis 2 MIN-2).** At the base `tree_walker_proto.rs:377` and
`node_iterator_proto.rs:299` match `Ok(None) | Err(FilterError::Throw) => Ok(JsValue::Null)`; the arm is
reachable only because every `FilterError::Throw` construction (`node_filter_dispatch.rs:51-98`) sets
`pending_error` first and the proto returns that error before the match (`tree_walker_proto.rs:340-347`).
A failure → null arm that is unreachable by an invariant is the C-2 (a) shape guarded by review, and under
P0a-2's checked-reason rule "unreachable by construction" has no mechanical form (it would be a D row). So
P0a-1 removes the arm: the `Err(FilterError::Script(f))` arm **propagates** — it takes `pending_error` and
returns `Err(value)`; with no pending value, `NotStarted` returns `Err(VmError::resource_limit())` (the
kind is the whole information) and `Threw` returns an internal error (never null; a debug assertion names
the breach). The **`pending_error` invariant**, stated once: `accept` returns `Err(FilterError::Script(_))`
only after `pending_error = Some(value)` for a thrown value; the proto consumes `pending_error` in the
`Script` arm of the traversal result and nowhere else. After this the two sites are auto-recognised
propagation (P0a-2 rule 2) or P rows, not N.

**Rejected (PM 533).** (b) Removing the `once` entry after the call: DOM §2.9 *inner invoke* removes before
the call (step 2.5 before step 2.13), and a listener that re-dispatches the same event during its own call
must not run twice. (c) A top-only N row for `call_listener`: the collapsing boundary is the defect itself;
a row that watches it leaves the defect in place and would go silently live on the first nested path into
the session dispatch. The same (c) is why D-15's S / W rows ("nothing written, top-only") were wrong
(PM 544): both claims were false on the code (§5.4), and a hand-asserted top-only is what P0a-2 now checks
on every class.

### §4.6 Native-callback paths (enumerated by mechanism)

Rust code re-enters script only through the four entries of §4.3; every native body that calls back does so
through `Vm::call` / `call_construct_native` / `construct_synchronous` / `run_function` (CP-1, via
`ctx.call_function` / `call_value` / `get_property_value` → `resolve_property` → `call`, `ops_property.rs:40`,
`shape_ops.rs:601` / `:615`), `resume_generator` (CP-2) or the wasm entries (CP-3). The mark is set at the
mechanism (§4.2), so this list is the **seed for cells**, not an input to correctness: a path missing here
is still marked. Classes, with the cell that pins a representative:

| Class | Representatives (native body that re-enters) | Cell |
|---|---|---|
| bound functions | `call_dispatch`'s `BoundFunction` unwrap (not entering) → the target's arm | `rl_not_entered_bound_chain_is_not_started`, `rl_entered_bound_native_listener_is_threw` |
| array / typed-array HOFs | `forEach`, `map`, `reduce`, `sort` comparator, `Array.from` mapFn, `join` → `toString` (`natives_array_hof.rs`, `typed_array_hof.rs`) | `rl_entered_bound_native_listener_is_threw` |
| JSON callbacks | reviver (`internalize`), replacer / `toJSON` (`serialize_*`) | `rl_entered_reviver_nested_failure_is_threw` |
| accessors reached from native code | getters / setters via `get_property_value` → `resolve_property` → `call`; `Object.assign`, `Object.entries`, spread, `structuredClone`'s own-property reads | `rl_entered_getter_from_native_is_threw` |
| coercions | ToPrimitive (`toString` / `valueOf` / `Symbol.toPrimitive`), ToPropertyKey (B-2's site), `Symbol.hasInstance` | covered by the accessor cell's mechanism (same entry); no separate cell |
| iteration protocol | `iter_next` / `iter_close` (`Symbol.iterator`, `next`, `return`; for-of, spread, destructuring, `yield*`) | `rl_entered_generator_resume_from_native_is_threw` (`Array.from(gen)`) |
| Promise jobs | `run_reaction`, `run_callback` (`queueMicrotask`), the `new Promise` executor, thenable `then`, combinator / finally steps | `rl_async_step_entry_failure_rejects_wrapper` (CP-2 side); the reaction loop is a drain (rows R) |
| host callbacks | the three listener loops (§4.5), observer callbacks (`deliver_to_observer_callbacks`), timers (`fire_current_timer`), CE reactions and upgrade (`invoke_callback`, `invoke_upgrade`), stream `start` / `pull` / `cancel`, worker message events, `importScripts` | `rl_ua_dispatch_once_listener_*`, `rl_unhandledrejection_once_listener_*`; the rest are P0b-L3's / P0b-VM's cells against their rows |
| generator resume from a native | `next` / `return` / `throw` natives → `resume_generator`; `drive_async_coroutine` | `rl_cp2_generator_resumable_after_entry_failure`, `rl_entered_generator_resume_from_native_is_threw` |
| construct from a native | `construct_synchronous` (CE upgrade), `Reflect.construct` / `do_new` → `push_js_call_frame`, `super()` (`dispatch_super_inner`) | `rl_cp1_entry_failure_no_callee_effect` (construct path), P0b-L3's `rl_ce_upgrade_after_constructor_started_takes_threw_branch` |
| not live | `Proxy` traps (no `Proxy` kind in `vm/object_kind.rs`); WebIDL *call a user object's operation*'s getter-before-call for `handleEvent` (the listener store accepts only callables, `event_target.rs:131`) | — (recorded; the mark covers them if they appear) |

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
`run()`. These are catchability fixes; the escape rule's soundness no longer rests on them (§2 ER × B).
P0a-2 rule 6 (a `?` on a carrier in `run()` is a member) stays as the census-side guard of catchability.

### §5.3 D-14 and BC-1…BC-3: the engine boundary (PM 533 (a))

| Id | Site | What the error becomes today | Class | Governing step |
|---|---|---|---|---|
| D-14 | `ScriptEngine::call_listener` (impl `engine.rs:203`, result dropped at `:275`) × its caller `script_dispatch_event_core` (session `event_dispatch/mod.rs:499` removal, `:503` call, `:517` `remove_listener`) | dropped; the once entry removed before the call stays removed, and its callable is dropped | D-state (W across the boundary, P0a-2 rule 7) | DOM §2.9 *inner invoke* steps 2.5, 2.13 |
| BC-1 | `ScriptEngine::eval` (impl `engine.rs:176`) | a string in `EvalResult`'s `error` field; the kind is gone at the boundary | boundary collapse; no consumer needs the kind today (the shell's are event-loop top, N) | HTML §8.1.4.3 steps 10–11 (via D-13) |
| BC-2 | `ScriptEngine::drain_timers` (impl `engine.rs:332`) | same, per timer | same | — |
| BC-3 | `FilterAction::accept` (impl `node_filter_dispatch.rs:51`) | `FilterError::Throw`; the kind is in `pending_error`; the protos' null arms rest on an unstated invariant | same; the traversal algorithms propagate with `?` and write nothing (DOM §6 *filter* step 6: unset the active flag, rethrow) | DOM §6 *filter* step 6 |

Fix: §4.5. D-14 is fixed here (the trait change is P0a-1's, PM 533 (a)), so it is never pending; BC-1…BC-3
become kind-bearing and leave the boundary population. The S row `engine.rs:275` of the census at
`ce603c74` was D-14's engine half, not a drop with nothing written (PM 533 item 3; P0a-2 §7).

### §5.4 D-15: the `unhandledrejection` loop (R4 Axis 2 IMP-2; PM 544)

| Id | Site | What the error becomes today | Class | Governing step |
|---|---|---|---|---|
| D-15 | `vm/natives_promise.rs` `dispatch_unhandled_rejection_event`: the `once` removal from the Document's `EventListeners` at `:565` (`listeners.remove(entry.id)`), the call at `:591` (`let _ = g.call(..)`), the store prune after the call (`remove_listener`, `:617-621`) | dropped, no report; the once entry removed before the call stays removed and its callable is pruned | D-state: the W population (the removal is a listener removal, one of its three kinds) and an S row whose two claims were false — "nothing written" (`:565` writes) and "top-only" (§3.1: reached from `dispatchEvent` at a script-controlled depth) | DOM §2.9 *inner invoke* steps 2.5, 2.13; HTML §8.1.4.7 *notify about rejected promises* step 4.1.2 |

Fix (the same shape as D-14, inside the engine, so no boundary type is involved): the loop takes the entry
with `EventListeners::remove_with_position` on the Document; it matches the call's result; on
`Ok` nothing more; on an error that `is_resource_limit()` (after `exit`, §4.2: `NotStarted`) it reinserts
at the index **before** anything else runs (the next listener's event object, the flag sync), skips the
store prune (`:617-621`) for that entry, reports the error (the stderr stand-in, C-3) and continues with
the next listener; on any other error (`Threw`) the removal and the prune stand and the error is reported
(today: swallowed; P0 umbrella §6 item 2's S row of this site becomes R). The listener is not re-invoked in
this dispatch. The index is exact for the reason of §4.5 (the mark). **Layering note:** this loop is VM
core writing an ECS component directly (the third *inner invoke* implementation); it is the registered
deviation's third site (§4.5, PM item), not a new design here.

**Every site of D-14's shape (PM 544).** Concept sweep, not a string sweep: "a listener entry removed
before a call whose result is not propagated", over every crate, keyed on the writers of the listener
stores (`git grep -nE 'listeners\.remove\(|\.remove\(entry\.id\)|remove_listener_entry|remove_listener_and_prune_back_ref' -- crates`
minus tests, 2026-11-08 at `d7ec4761`: 20 lines). Removals that precede a call in the same loop: the
session loop (`event_dispatch/mod.rs:499`, D-14), the VM dispatch loop (`event_target_dispatch.rs:460` via
`remove_listener_entry`, D-5, P0b-L3) and the `unhandledrejection` loop (`natives_promise.rs:565`, D-15).
The other hits are `removeEventListener` / abort-steps removals (`event_target.rs:608/614`, `abort.rs:724`,
`event_handler_attrs.rs` 552/558/649, the helpers `dispatch_target.rs:164`, `inner.rs:60`, `engine.rs:305`)
and test assertions, none followed by a call of the removed entry. Three loops; D-5 stays P0b-L3's (PM 509
item 2; PM 544: "P0b and everything after are unchanged").

## §6. Cells (written first)

New file `crates/script/elidex-js/src/vm/tests/tests_resource_limit.rs` (NEW). Each cell arms §4.4's
injection with an `n` taken from `resource_limit::count` on a dry run of the same body.

| Cell | Body | Expect |
|---|---|---|
| `rl_identity_predicates` | build each error form | `is_resource_limit` true only for `VmError::resource_limit()`; a `ThrowValue` and a `RangeError` kind are false; `CompileError` kinds likewise; the materialised object is a `RangeError` with `STACK_EXHAUSTED_MSG` |
| `rl_cp1_entry_failure_no_callee_effect` | `let n = 0; function g() { n++ }`; inject `Entry(Call, 1)`; `try { [1].forEach(g) } catch (e) { r = e.name }`; the same through `call`, `call_construct_native`, `run_function`, `construct_synchronous` from Rust | `r === 'RangeError'`, `n === 0`, frame and value-stack depth unchanged, the outer entry's mark unchanged; a second run gives `n === 1` |
| `rl_cp2_generator_resumable_after_entry_failure` | `function* g() { yield 1 } const it = g()`; inject `Entry(Resume, 1)`; `it.next()` throws; then `it.next()` | first: `RangeError`; second: `{ value: 1, done: false }` |
| `rl_async_step_entry_failure_rejects_wrapper` | an async function awaiting a resolved promise; inject `Entry(Resume, n)` for the step | the wrapper rejects with `RangeError`; the body's `finally` did not run (P0 umbrella §6 item 5) |
| `rl_cp3a_wasm_call_entry_failure_retry` | an exported function; inject `Entry(WasmCall, 1)` | `RangeError` (not `WebAssembly.RuntimeError`); the next call returns its value |
| `rl_cp3b_instantiate_entry_failure` | inject `Entry(WasmInstantiate, 1)` for `new WebAssembly.Instance(m)`, then for `WebAssembly.instantiate(m)` | sync: `RangeError`; async: rejects `RangeError`; an un-injected retry instantiates |
| `rl_escape_rule_materializes_in_started_frame` | `function f() { [1].forEach(x => x) }`; from Rust `vm.call(f)` with `Entry(Call, 2)`, then with `Entry(Call, 1)` | `n = 2`: `Err` kind `ThrowValue` (a `RangeError` object), not `ResourceLimit`; `n = 1`: kind `ResourceLimit` |
| `rl_not_entered_bound_chain_is_not_started` | `const b = h.bind(null).bind(null)`; from Rust `vm.call(b)` with `Entry(Call, 1)` | kind `ResourceLimit`; `h` ran 0 times (unwrapping is not entering) |
| `rl_entered_bound_native_listener_is_threw` | `let c = 0; function h() { c++ }`; `t.addEventListener('x', [].forEach.bind([1, 2], h), { once: true })`; inject `Entry(Call, 3)` (entries: the listener, `h`, `h`); dispatch twice | first: `c === 1`, the listener is reported as `Threw` and is gone (step 2.5 stands); second: `c === 1` (not re-run) |
| `rl_entered_reviver_nested_failure_is_threw` | `JSON.parse.bind(null, '[1,2]', r)` as a once listener, where `r` counts calls; inject `Entry(Call, n)` for `r`'s second call | the listener is `Threw` and gone; `r` ran once; the second dispatch does not run it |
| `rl_entered_getter_from_native_is_threw` | from Rust `vm.call(Object.assign.bind(null, {}, { get a() { return [1].forEach(x => x) } }))`; inject `Entry(Call, n)` for the arrow inside the getter | kind `ThrowValue`; a `RangeError` object |
| `rl_entered_generator_resume_from_native_is_threw` | `function* g() { yield [1].forEach(x => x) }`; from Rust `vm.call(Array.from.bind(null, g()))`; inject `Entry(Call, n)` for the arrow | kind `ThrowValue`; the generator is Completed (its body threw) |
| `rl_entered_mark_survives_non_entering_nested_entry` | once listener `A = [].forEach.bind([1, 2], f)` with `f = () => { try { Reflect.apply(1, null, []) } catch {} }`; inject `Entry(Call, n)` for `f`'s second call (after a nested entry that passed its check but entered nothing: `Reflect.apply(1)` → "not a function") | `Threw` (the outer mark survived the non-entering nested entry); under M-ENT-R: `NotStarted` |
| `rl_vm_eval_compile_resource_limit_kind` | inject `Compile(1)`; `vm.eval("1")`; also through the `ScriptEngine` `eval` of `ElidexJsEngine` | kind `ResourceLimit`; the `EvalResult` message starts with `RangeError` |
| `rl_regexp_literal_error_is_catchable` | `try { /a{2,1}/ } catch (e) { r = e.name }` (a pattern `regress` 0.11.1 rejects; §9 item 1) | `r` is set (today the error leaves `run()` uncaught) |
| `rl_ua_dispatch_once_listener_survives_entry_failure` | through `script_dispatch_event` (the session loop): listeners A (`once`), B (`once`), C on one target; inject `Entry(Call, 1)` for A; dispatch three times | first: A not run, B and C run, A's error reported; second: A runs, then C (A in its original position); third: C only |
| `rl_ua_dispatch_once_listener_threw_is_removed` | A (`once`) throws | A is gone after the dispatch and its callable is dropped (DOM §2.9 step 2.5 stands for a started call) |
| `rl_unhandledrejection_once_listener_survives_entry_failure` | `document.addEventListener('unhandledrejection', A, { once: true })`, B (`once`), C; `Promise.reject(1)`; inject `Entry(Call, 1)` for A at the checkpoint's dispatch; then a second `Promise.reject(2)` | first: A not run, B and C run, the error reported; second: A runs, then C |
| `rl_unhandledrejection_once_listener_threw_is_removed` | A (`once`) throws | A is gone; its error is reported (today swallowed) |
| `rl_eval_result_outcome_kind` | through `ScriptEngine::eval`: inject `Compile(1)`; then a script that throws; then `drain_timers` with `Entry(Call, 1)` at the timer callback | `outcome == Err(NotStarted)`; `Err(Threw)`; the timer's `EvalResult` carries `Err(NotStarted)` |
| `rl_tree_walker_filter_entry_failure_kind` | `document.createTreeWalker(root, NodeFilter.SHOW_ELEMENT, { acceptNode() { return 1 } })`; inject `Entry(Call, 1)` at the filter's call; `nextNode()`; then `nextNode()` | first throws `RangeError` and `currentNode` is unchanged; second returns the first element (the active flag was unset) |
| `rl_tree_walker_filter_throw_propagates_not_null` | `{ acceptNode() { throw 1 } }`; `nextNode()` | throws `1`; never `null` (the removed arm of §4.5) |
| `rl_computed_accessor_key_error_is_catchable` | `try { ({ get [{ toString() { throw 1 } }]() {} }) } catch (e) { r = e }`; and with `Entry(Call, 1)` on the `toString` | `r === 1`; with the injection, `r` is a `RangeError` |

## §7. Mutants (run on the final head, debug)

| Mutant | Change | Dies |
|---|---|---|
| M-ID | `is_resource_limit` always false | `rl_identity_predicates`, `rl_vm_eval_compile_resource_limit_kind` (the P0b slices record their own cells against it) |
| M-ESC | `exit`'s rewrite removed | `rl_escape_rule_materializes_in_started_frame`, `rl_entered_*` |
| M-ENT-F | the mark not set at the frame push (`call_internal` / `push_js_call_frame`) | `rl_escape_rule_materializes_in_started_frame` (`n = 2` comes out `ResourceLimit`), `rl_entered_getter_from_native_is_threw` |
| M-ENT-N | the mark not set at the native body entry | `rl_entered_bound_native_listener_is_threw` (`forEach`'s own entry has no frame of its own: without the native-arm store the outer mark stays unset) |
| M-ENT-G | the mark not set at the generator take | `rl_entered_generator_resume_from_native_is_threw` if observable through `Array.from`; else recorded with the reason (§9 item 7) |
| M-ENT-R | save / restore of the mark removed | `rl_entered_mark_survives_non_entering_nested_entry` |
| M-ENT-O | the reset placed before the check | `rl_cp1_entry_failure_no_callee_effect` (the outer entry's mark changes on a failed check) |
| M-B1 | `Op::PushConst` back to `?` | `rl_regexp_literal_error_is_catchable` |
| M-B2 | `op_define_computed_accessor` back to `?` | `rl_computed_accessor_key_error_is_catchable` |
| M-P-CP1 | CP-1 check after `f(self, mode)` | `rl_cp1_entry_failure_no_callee_effect` |
| M-P-CP1s | CP-1 check after the `entry_frames` snapshot | **equivalent** (the exit path restores the snapshot); recorded, answers R2 S-2 for CP-1 |
| M-P-CP2 | CP-2 check after the generator is marked Running | `rl_cp2_generator_resumable_after_entry_failure` |
| M-P-CP3a / M-P-CP3b | CP-3 checks after the `HostState` bind / after `Store::new` and the start function | `rl_cp3a_*` / `rl_cp3b_*` if observable; a survivor is recorded as equivalent with the reason (§9 item 2) |
| M-EV | `Vm::eval` maps to `CompileError` | `rl_vm_eval_compile_resource_limit_kind` |
| M-D14a | no reinsert on `NotStarted` | `rl_ua_dispatch_once_listener_survives_entry_failure` |
| M-D14b | reinsert after `run_microtasks` | **equivalent** under the mark (§4.5 "Why the revert is exact"); recorded, the order of §4.5 item 2 stays |
| M-D14c | reinsert at the end of the list | `rl_ua_dispatch_once_listener_survives_entry_failure` (second dispatch: C before A) |
| M-D14d | `remove_listener` still called on `NotStarted` | same (A's callable is gone; the second dispatch cannot run it) |
| M-D15a / M-D15c / M-D15d | the same three for the `unhandledrejection` loop | `rl_unhandledrejection_once_listener_survives_entry_failure` |
| M-OUT | `script_failure` always `Threw` | `rl_eval_result_outcome_kind`, `rl_ua_dispatch_once_listener_survives_entry_failure` |
| M-BC3 | the null arm restored for `FilterError::Script` | `rl_tree_walker_filter_throw_propagates_not_null` |

## §8. Landing obligations

What P0a-1's PR (or, where marked, the PM / PMO at landing) does. This memo edits no ledger. Program-level
obligations (PR-1 precondition, breadth record, slots) are P0 umbrella §7.

The ledger and notice rows this slice triggers are rows of the P0 program's landing list (P0 umbrella §7,
PM 513 (d)): L-2, L-3 (now also D-15's report), L-5, L-6, L-12 (L3 notice) and L-14 (boundary notice).
Slice-local: none; the census counts are P0a-2's (P0a-2 §10).

## §9. Unverified facts (each decided at implementation, before its cell is written)

1. The `regress` 0.11.1 pattern for `rl_regexp_literal_error_is_catchable`.
2. Whether M-P-CP3a / M-P-CP3b are observable.
3. The post-L3 function and callee names of the `AbortSignal` abort loop and of cell (o), read at P0a-1's
   base once L3's `#11-abort-signal-any-dependent-flattening` PR has landed (its plan Rev 2 is `a8c4179e`,
   Rev 2.1 due 11-09; neither is on this branch); the allowlist row is P0a-2's (P0a-2 §7).
4. Whether the stderr report of a `Threw` listener (§4.5, engine side; D-15 likewise) is observable in a
   cell; if not, it is recorded as not pinned (the report channel is `#11-embedder-diagnostic-channel`'s).
5. `#11-event-path-window-missing` PR-A's schedule against P0a-1's (PM 540 (2): P0a-1 first; if P0a-1 is
   late, the PM decides).
6. How a `ThrowValue` produced by `exit` is kept alive across the caller's native frames until it is
   re-thrown or materialised: the existing discipline (`gc_enabled = false` inside native arms,
   `interpreter.rs:312-335`; `ThrowValue` errors from script `throw` already traverse natives this way) is
   expected to cover it; confirmed by reading `gc/roots.rs` before `rl_entered_*` are written.
7. Whether `Array.from(gen)` makes the generator take observable for M-ENT-G, or another native that
   resumes a generator is needed (`yield*` from a native-called body; `drive_async_coroutine`).
8. Whether `Reflect.apply(1, ..)` reaches `with_call_mode` before its "not a function" (the
   non-entering nested entry of `rl_entered_mark_survives_non_entering_nested_entry`); if `Reflect.apply`
   type-checks first, `Function.prototype.call.call(1)` or a direct Rust `vm.call(non_callable)` is used.

webref lookups behind §3 (2026-11-08): `body ecma262 sec-error-handling-and-language-extensions`;
`dfn html 'create a classic script'` → §8.1.4.3; `dfn dom 'inner invoke'` → §2.9 (steps read by `body dom
concept-event-listener-inner-invoke`); `dfn dom 'remove an event listener'` → §2.7 (step 2 sets removed);
`dfn dom 'filter'` → §6 `#concept-node-filter` (step 6 read by `body dom concept-node-filter`); `dfn html
'notify about rejected promises'` → §8.1.4.7 (step 4.1.2 read by `body html notify-about-rejected-promises`);
`dfn html 'restore the history object state'` → §7.4.6.2.
