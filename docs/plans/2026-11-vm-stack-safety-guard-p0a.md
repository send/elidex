# VM stack-safety guard — P0a plan (VM core: error identity, escape rule, choke-point placement, census check)

Status: **plan (pre-implementation)**, per-PR slice of the [P0 umbrella](2026-11-vm-stack-safety-guard-p0.md)
(PM 509); `/elidex-plan-review` required before code. Owner: VM P4. Base: `ce603c74`; every code anchor
below is at that commit. Provenance root `H` and the census prototype are as in the P0 umbrella header;
the prototype is `H/scripts/err_consumer_census_proto.py` (+ `H/scripts/err_census_classify.py`,
`H/scripts/err_write_before_call_proto.py`), its logs `H/logs/err-census-p0.log`,
`H/logs/err-census-p0-shell.log`, `H/logs/err-census-p0-classified.tsv`,
`H/logs/err-write-before-call-p0.log`. The prototype is not the deliverable: P0a ships the tracked check
of §5, and the prototype's numbers are re-derived by that check at P0a's base.

The contract C-1…C-5 is the P0 umbrella §4 (canonical); this memo does not restate it. Edges, schedule
and the other slices: P0 umbrella §1.

## §0. Charter

P0a implements the VM half of the P0 contract: the error identity (§4.1), the escape rule (§4.2), the
placement of the four choke-point checks (§4.3), the test-only injection the consumer slices use (§4.4),
the tracked census check and its allowlist with the `# pending` section (§5), and the VM-owned fixes
D-13, B-1, B-2 (§6). Each later P0b slice only removes its rows from `# pending` (P0 umbrella §0.4).

## §1. Scope

**In.** `crates/script/elidex-js/src/vm/error.rs`, `compiler/mod.rs` (`CompileError` kind),
`vm/resource_limit.rs` (NEW), `vm/interpreter.rs` (CP-1 hook, `Vm::eval` mapping),
`vm/natives_generator.rs` (CP-2 hook), `vm/host/wasm/exported_func.rs` and `vm/host/wasm/instance.rs`
(CP-3a / CP-3b hooks and the named conversion), `vm/ops.rs` (escape rule), `vm/dispatch.rs` and
`vm/dispatch_objects.rs` (B-1, B-2 routing); the injection hooks in `compile_script`, `parse_json_str`,
`stringify_for_structured_shortcut` and `dispatch_vm_simple_event` (§4.4), with the removal of L3's
`fail_next_abort_dispatch` seam and the move of its cell onto that hook (§4.4; L3's files, site list via PM,
P0 umbrella §7 L-12); the tracked check (§5.1); cells in
`crates/script/elidex-js/src/vm/tests/tests_resource_limit.rs` (NEW).

**Out.** The consumer fixes D-1…D-12 (P0b-VM-1, P0b-VM-2, P0b-L3; P0 umbrella §1); `native_import_scripts`'s kind
mapping (D-11, P0b-VM-1) although §4.1 item 3 states the rule; the deviations of P0 umbrella §6.

## §2. Coupled invariants

Elements: **ID** (error kinds, predicates, `Vm::eval` boundary; §4.1), **ER** (escape rule; §4.2),
**B** (B-1 / B-2 routing; §6.2), **CP** (choke-point placement; §4.3), **INJ** (test-only injection;
§4.4), **CEN** (census rules; §5.2), **PEND** (`# pending` section and planned-producer list; §5.3, §5.4).

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
| INJ × L3 seam | one test-failure mechanism: `SimpleEventDispatch(n)` replaces L3's `fail_next_abort_dispatch`, and as a `#[cfg(test)]` statement it does not make `dispatch_vm_simple_event` a producer | cell (o) after the move; control `census_cfg_test_hook_not_producer` | cell + check |

## §3. Spec coverage map

No spec defines a resource limit (P0 umbrella §3). The rows are the clauses this slice's code answers to.

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| ECMA-262 §17 Error Handling and Language Extensions | paragraph 1 (errors reported when the construct is evaluated), paragraph 3 (no other kinds of errors as early errors) | a resource limit is neither an early error nor a specified error; it surfaces where the construct is evaluated; B-1 / B-2 errors become catchable where evaluated | `VmErrorKind::ResourceLimit` (NEW), `CompileErrorKind::ResourceLimit` (NEW), `Op::PushConst`, `op_define_computed_accessor` | ✓ | yes |
| WHATWG HTML §8.1.4.3 Creating scripts | *create a classic script* steps 10–11 (ParseScript; a list of errors becomes the error to rethrow) | a compile-time resource limit is not a parse error | `Vm::eval` mapping (`interpreter.rs:36`) | ✓ | yes |

**Breadth**: K=2 specs (ECMA-262, WHATWG HTML), M=2 entries (verified 2026-11-08 by `python3 .claude/skills/elidex-plan-review/preflight.py` on this memo: rc 0, K=2, M=2, "ok (single PR scope)"). Below the split thresholds.

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

**One test-failure seam (one issue, one way; ratified PM 513).** L3's `#11-abort-signal-any-dependent-flattening`
PR (around 11-10; P0 umbrella §1) adds a `#[cfg(test)]` `fail_next_abort_dispatch` for its cell (o): the
next abort dispatch fails, so the cell can show that every dependent's abort steps still run. That is a
second test-failure mechanism beside this one. P0a rebases on L3's PR (edge, P0 umbrella §1) and moves it
here: `SimpleEventDispatch(n)` is the general form, P0a deletes `fail_next_abort_dispatch` and rewrites cell
(o) to arm `InjectAt::SimpleEventDispatch(n)` with `n` from `resource_limit::count`, same assertion. Its
error is C-1's kind, which is also the only `Err` that can reach this call once P0's error exists (§5.4).
L3's contract is unchanged. If L3's PR lands after P0a, it uses `SimpleEventDispatch` from the start and
adds no seam of its own.

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

1. Population per P0 umbrella §5.1, recomputed from the code on every run; test files and `#[cfg(test)]`
   code are excluded by the lexer (§5.1), not by a list.
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
   admissible with a reason).
8. **Loop rule (fail-safe; ratified PM 513).** A carrier call propagated by `?` (or `.map_err(..)?`, or a
   binding later propagated so) whose position lies inside a `for` / `while` / `while let` / `loop` body of
   the enclosing function is a **member**, not auto-propagation: the `?` is correct for the trigger but may
   leave the loop's remaining subjects unprocessed, the shape of P0 umbrella §5.1's C-2 (c) that rule 2
   would otherwise accept unseen. The rule does not try to tell "several subjects" from "elements of one
   operation"; every such `?` is a member, and the row's class says which it is (§5.4). `run()` is
   excluded (rule 6 already makes its `?` uses members).

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

Admissible rows (paths under `crates/script/elidex-js/src/` unless stated; lines at `ce603c74`):

- **R (32):** `engine.rs` 191 / 344; `vm/host/body_mixin.rs:289`; `vm/host/cache/natives.rs` 96 / 121 /
  145 / 198 / 252 / 287 / 340 / 382 / 420 (`finish` rejects); `vm/host/custom_elements/creation.rs:238`,
  `flush.rs` 104 / 183 / 202 / 234, `lookup.rs:183` (report); `vm/host/fetch/mod.rs` 183 / 204;
  `vm/host/observer_common.rs:295` (report; records are delivered from the embedder's entry, top-only);
  `vm/host/readable_stream/controller.rs:135`, `mod.rs` 410 / 639 (stream error / cancel rejection, C-4
  non-re-entry); `vm/host/service_worker/container.rs` 109 / 218; `vm/host/wasm/namespace.rs:578`;
  `vm/natives_generator.rs:472` (wrapper rejection; residual P0 umbrella §6 item 5); `vm/natives_promise.rs` 648 /
  689 / 747; `vm/natives_timer.rs:370`.
- **S (4):** `engine.rs:275` (`call_listener`'s `let _ =`, the shell's UA dispatch, top-only);
  `vm/coerce.rs:246` (`to_display_string`, console display); `vm/host/custom_elements/flush.rs:70`
  (`invoke_one` reports inside); `vm/natives_promise.rs:591` (unhandled-rejection listener, top-only).
  Home: P0 umbrella §6 item 2.
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

**`# pending` (15 rows; this key set is the frozen ceiling of §5.2 rule 4).** Keys (path, function,
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
`?` outside the loop → auto). `--mutants`
runs the check with each rule disabled and asserts the matching control turns green, so a rule that
stops working is seen (the plan-memo-umbrella-check precedent).

### §5.6 Stated limits and their home

The check does not see: calls through function pointers, trait objects and closures passed across
functions (the `NativeFn` table, `fire_vm_message_event`'s closure argument is seen only because its body is
in the caller); writes performed inside a callee before the carrier call (D-1's `take_uncompiled` sat in
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
| M-CC | each §5.2 rule (and the ceiling, the `#[cfg(test)]` statement exclusion) disabled | the matching control (§5.5) |

## §9. Landing obligations

What P0a's PR (or, where marked, the PM / PMO at landing) does. This memo edits no ledger. Program-level
obligations (PR-1 precondition, breadth record, slots) are P0 umbrella §7.

The ledger and notice rows this slice triggers are rows of the P0 program's landing list (P0 umbrella §7,
PM 513 (d)): L-2, L-3, L-4, L-5, L-6, L-7 (census limits, formerly L-a1) and L-12 (L3 notice). Slice-local:

- **L-a2. Counts and ceiling**: P0a's PR body records the tracked check's counts per class at its head
  (rule 8's members separately) and the 15 `# pending` keys of §5.4.

## §10. Unverified facts (each decided at implementation, before its cell is written)

1. The `regress` 0.11.1 pattern for `rl_regexp_literal_error_is_catchable`.
2. Whether M-P-CP3a / M-P-CP3b are observable.
3. Every N row claiming top-only is re-checked by §5.2 rule 5 at P0a's base.
4. The post-L3 function and callee names of the `AbortSignal` abort loop (§5.4) and of cell (o), read at
   P0a's base once L3's `#11-abort-signal-any-dependent-flattening` PR has landed (its plan Rev 2 is
   `a8c4179e`, Rev 2.1 due 11-09; neither is on this branch), and whether the tracked check lists the call.

webref lookups behind §3 (2026-11-08): `body ecma262 sec-error-handling-and-language-extensions`;
`dfn html 'create a classic script'` → §8.1.4.3.
