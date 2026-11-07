# VM stack-safety guard — P0 plan (resource-limit Err contract, Err-consumer census)

Status: **plan (pre-implementation)**, written for PM 502; `/elidex-plan-review` required before code
(edge-dense, §2). Owner: VM P4. Parent: [umbrella](2026-11-vm-stack-safety-guard.md). Lands **before**
[PR-1](2026-11-vm-stack-safety-guard-pr1.md) and [PR-1b](2026-11-vm-stack-safety-guard-pr1b.md).
Base: `ce603c74` (origin/main `69fbe61f` merged); every code anchor below is at that commit.
Provenance root `H` is the umbrella's (`/Users/kazuaki/repos/send.sh/elidex-pm-trial/handoff/stackguard-census`).
The census prototype is `H/scripts/err_consumer_census_proto.py` (+ `H/scripts/err_census_classify.py`,
`H/scripts/err_write_before_call_proto.py`); its logs are `H/logs/err-census-p0.log`,
`H/logs/err-census-p0-shell.log`, `H/logs/err-census-p0-classified.tsv`, `H/logs/err-write-before-call-p0.log`.
The prototype is not the deliverable: P0 ships the tracked check of §8, and the prototype's numbers are
re-derived by that check at P0's base.

**Relation to U / PR-1 / PR-1b.** This memo is the single home of the rules below. The fold revision of
the three sibling memos replaces their I-3 text, the CE-upgrade bullet, the async-function bullet and the
placement mutants with pointers here; that revision is not part of this memo.

## §0. Charter

### §0.1 Why (PM 502)

Guard plan-review R1 and R2 each found, one site at a time, a consumer that takes an `Err` from a
probe-bearing call and turns it into a definitive fact: custom-element upgrade marks the element Failed
(R1); caller-side state mutated before `with_call_mode` (R1); the lazy event-handler compile maps every
`Err` to `None` and its caller removes the listener (R2 S-1); no mutant pins probe placement (R2 S-2). The
common root is the absence of a contract for the guard's error and of a census of its consumers. P0 states
the contract once (§4), gives the error an identity that consumers can test (§5), enumerates every consumer
by a property (§6), fixes every consumer that violates the contract (§7), keeps the census honest with a
tracked check whose unknown case is red (§8), and pins each fix and each choke-point placement with a cell
and a mutant (§9, §10).

### §0.2 What P0 owns, and what it does not

P0 owns: the contract C-1…C-5 (§4); the error identity types and the escape rule (§5.1, §5.2); the
**placement** of the four choke-point checks CP-1, CP-2, CP-3a, CP-3b, i.e. the VM half of umbrella I-3
(§5.3); the Err-consumer census and its tracked check (§6, §8); the fix of every definitive consumer (§7);
their cells and mutants (§9, §10).

P0 does not own: the primitive `crate::stack_guard` (NEW: PR-1), the reserve, the per-thread bound, the walker probes,
the compile-time probes, the caps, the regexp front door (PR-1); `MAX_WASM_STACK` and the CP-3 ask (PR-1b);
the frame cap (PR-2). Those PRs put their probe **inside** P0's check (§5.3) and produce the error P0 defines.

### §0.3 Order and why P0 is first

PR-1's probes are the first production producers of a resource-limit `Err`; the definitive consumers of
§6.4 become live failures the moment a producer exists. So every definitive consumer is fixed before
PR-1 lands. At P0's landing no production code produces the error: the cells produce it through the
test-only injection of §5.4, and nothing in production depends on a step that is not yet verified. The
one rule whose input does not exist before PR-1 (the compile-channel aggregation, §5.1 item 4) is stated
here and implemented with its first producer in PR-1, with PR-1's cell.

### §0.4 Scope decisions asked by PM 502

- **CE upgrade moves to P0.** It is a definitive consumer (§6.4 rows D-3/D-4). PR-1 §2's CE bullet and
  PR-1's cell `ce_upgrade_probe_failure_takes_threw_branch` are replaced by P0's rule and cells; the
  expected outcome changes from "failed" to "reverted, retryable" (§7.2).
- **The pre-`with_call_mode` checks move to P0.** For VM state they are the choke-point placement
  (§5.3); for consumer state they are the W population of the census (§6.1) and the D-5 fix.
- **The placement-mutant gap (R2 S-2) belongs to P0.** Placement is where the check sits relative to the
  first state mutation; P0 places the check, PR-1 / PR-1b only fill its body, so the placement cells and
  mutants are P0's (§9, §10).
- **Split.** By breadth P0 is SPLIT-DEFAULT (§3). Proposed: **P0a** (VM core: §5, the tracked check §8
  with the D rows in its pending section, the VM-owned fixes D-13, B-1 and B-2, the placement and identity
  cells) and **P0b** (the consumer fixes D-1…D-12 and their cells, one PR). PM decision requested (see
  report).

### §0.5 Decisions taken here

- **D-P0-1 Identity by kind, not by message.** A resource-limit error is a distinct `VmErrorKind`
  (§5.1). A message compare or a `RangeError` kind cannot be told apart from a script's own
  `RangeError` (`new Array(-1)`).
- **D-P0-2 "Did not start" semantics with an escape rule.** The kind means the guarded step never
  began. Once the error has been thrown into a started script frame it is that script's exception (§5.2).
  No hidden marker on the error object: a marker would make a constructor that ran halfway "retryable",
  and re-running a half-run constructor is worse than the spec's threw branch.
- **D-P0-3 One choke-point check, placed by P0, filled by PR-1 / PR-1b** (§5.3).
- **D-P0-4 Re-entry subjects revert; other subjects take the spec's exception branch** (C-4, §4).
- **D-P0-5 The census is a tracked check in the `tools` job**, with the population derived from the code
  and an unknown member red (§8). Not `trip-wires`: the check needs an interpreter (CLAUDE.md §CI job
  boundary).
- **D-P0-6 Caps are not resource limits.** `MAX_JSON_DEPTH`, `MAX_NESTING_DEPTH`, `MAX_REGEXP_DEPTH`,
  `DENSE_ARRAY_LEN_LIMIT` and the like are answers determined by the input (umbrella I-4); the same input
  fails the same way at any depth. The contract covers only the probe-originated error.

## §1. Scope

**In.** `crates/script/elidex-js/src/vm/error.rs`, `compiler/mod.rs` (`CompileError` kind),
`vm/resource_limit.rs` (NEW), `vm/interpreter.rs` (CP-1 hook, `Vm::eval` mapping), `vm/natives_generator.rs`
(CP-2 hook), `vm/host/wasm/exported_func.rs` and `vm/host/wasm/instance.rs` (CP-3a / CP-3b hooks and the
named conversion), `vm/ops.rs` (escape rule), `vm/dispatch.rs` and `vm/dispatch_objects.rs` (B-1, B-2 routing); the consumer
files of §6.4: `vm/host/event_handler_attrs.rs`, `vm/host/dispatch_target.rs`,
`crates/script/elidex-script-session/src/event_handler_reconcile.rs`,
`crates/script/elidex-script-session/src/event_listener.rs`, `vm/host/custom_elements/upgrade.rs`,
`crates/dom/elidex-custom-elements/src/upgrade.rs`, `vm/host/event_target_dispatch.rs`,
`vm/host/structured_serialize.rs`, `vm/host/history.rs`, `vm/host/indexeddb/value.rs` and its callers,
`vm/host/service_worker/deliver.rs`, `vm/host/worker_scope.rs`, `vm/host/subtle_crypto/ops.rs`; the tracked
check (§8); cells in `crates/script/elidex-js/src/vm/tests/tests_resource_limit.rs` (NEW).

**Out.** Everything of §0.2's second paragraph; the pre-existing deviations recorded in §6.6 (each with
its home); the HTML "report an exception" event (`#11-embedder-diagnostic-channel` keeps the stderr
stand-in).

## §2. Coupled invariants

P0 is edge-dense: five invariants of its own intersect each other and two of PR-1's (I-1 coverage, whose
probes are the producers; I-6 one error shape per channel, umbrella §2 / PR-1 §2).

- **C-1 Identity.** A resource-limit error is `VmErrorKind::ResourceLimit` (NEW) on the runtime channel
  and `CompileErrorKind::ResourceLimit` (NEW) at the compile boundary, and it means "the guarded step has
  not started". The escape rule keeps that meaning true across activations.
- **C-2 No conversion.** No consumer turns it into (a) a success value (substitution), (b) another error
  class (reclassification), or (c) a durable fact about a subject that outlives the trigger.
- **C-3 Surfacing.** It leaves every consumer by the trigger's own error channel: propagation, a throw
  into the running script, rejection of the trigger's promise or request, or a report.
- **C-4 Spec branches.** At a subject with a state-keyed re-entry, a spec "if X threw" branch is not
  taken for it: the subject is left as if the trigger had not happened. At a subject without one, the
  branch is the error channel of C-3 and is taken with the `RangeError`.
- **C-5 Census honesty.** Every consumer a resource-limit error can reach is in the census; the population
  is derived from the code; an unknown member, a stale row and a definitive row are each red.

| Pair | Intersection | Pinned by | Basis |
|---|---|---|---|
| C-1 × C-2 | C-2's predicate is `is_resource_limit()`; a consumer cannot apply C-2 to an error it cannot recognise, so C-1 must hold at every boundary the error crosses (`Vm::eval`, `native_import_scripts`, the CP-3 conversions) | `rl_vm_eval_compile_resource_limit_kind`, `rl_cp3b_instantiate_entry_failure` | cell |
| C-1 × C-3 | Surfacing materialises the error (a thrown or rejected `RangeError`); after materialisation it is no longer C-1's kind, so C-3 is the last place C-1 applies | escape rule §5.2; `rl_escape_rule_materializes_in_started_frame` | cell |
| C-1 × C-4 | "Not started" is what makes a revert correct: nothing of the trigger ran, so leaving the subject as before is exact. A half-run step arrives as a thrown value and takes the spec branch | `rl_ce_upgrade_after_constructor_started_takes_threw_branch` | cell |
| C-1 × C-5 | The census population is "can receive C-1's kind"; its producers are derived from the code (fns that call the check) plus a planned list that only shrinks (§8.3) | tracked check controls | check |
| C-2 × C-3 | C-2 forbids, C-3 prescribes; together every consumer has exactly one admissible shape per subject class (§6.3 classes P/T/R; D is the forbidden one) | §6.3 class table | structural |
| C-2 × C-4 | A spec exception branch that writes a durable fact is C-2(c) at a re-entry subject and C-3 elsewhere; C-4 is the rule that tells them apart | D-3/D-4 vs the stream rows (§6.3 R) | structural |
| C-2 × C-5 | A D row is not admissible in the allowlist (after P0b; before it only in the pending section) | control `census_rejects_definitive_row` | check |
| C-3 × C-4 | Reverting a subject must still surface the error on the trigger's channel, or a revert is a silent drop | each D-state cell asserts the report / throw | cell |
| C-3 × C-5 | The census classifies the channel each row uses; a row that drops the error with no state written is class S, admissible but recorded (§6.6) | §6.3 S rows | check |
| C-4 × C-5 | Re-entry is a property of the subject, not of the site; the W population (writes before a carrier call) finds the subjects whose state is written before the call | W rows §6.3 | check |
| C-1 × I-1 | PR-1 / PR-1b's probes are C-1's producers: the choke-point probes sit inside P0's check, and every walker / front-end / front-door probe converts through `From<StackExhausted>` to `VmError::resource_limit` (NEW) or `CompileErrorKind::ResourceLimit`, so every probe failure has C-1's kind by construction | §5.3; PR-1 §4.1 | structural |
| C-2 × I-1 | Before a producer exists a definitive consumer is latent; I-1 makes it live, which is the order of §0.3 | PR-1 precondition L-1 | ledger |
| C-3 × I-1 | Surfacing runs inside the reserve: building a `RangeError` and reporting it must fit in `WALKER_RESERVE` (PR-1 §4.3) | PR-1 `stack_guard_reserve_margin_ratchet` | PR-1 cell |
| C-4 × I-1 | A revert writes ECS state at the floor; a revert is a few component writes, bounded, no recursion | §7.2, §7.1 | structural |
| C-5 × I-1 | Each PR-1 probe site that lands removes its name from the planned-producer list (§8.3); the derived producer set then covers it | control `census_planned_producer_already_derived` | check |
| C-1 × I-6 | I-6's runtime message is `STACK_EXHAUSTED_MSG` (NEW here, PM 421 (a)); the kind is P0's, the script-visible class stays `RangeError` | `rl_identity_predicates` | cell |
| C-2 × I-6 | Reclassification (C-2 (b)) is the consumer-side breach of I-6; the D-cls rows are exactly the sites that would break I-6 | D-10…D-13 cells | cell |
| C-3 × I-6 | The channel changes the form (throw / rejection / report), never the class | rejection rows keep `vm_error_to_thrown` | structural |
| C-4 × I-6 | Taking a spec branch with the `RangeError` (non-re-entry subjects) keeps the class: a stream errored with it, a promise rejected with it | §6.3 R rows | structural |
| C-5 × I-6 | The census does not check classes; the D-cls cells do | — | cell |

## §3. Spec coverage map

No spec defines a resource limit. The rows are the algorithms whose exception branch or error channel a
consumer uses; the "Branch" cell says what a resource-limit error does there. Every number is from
`.claude/tools/webref` (`heading` / `dfn` / `body`, ECMA-262 by `body` of the clause and `aoid`); the
commands are listed at the end of §12.

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| ECMA-262 §17 Error Handling and Language Extensions | paragraph 1 (errors reported when the construct is evaluated), paragraph 3 (no other kinds of errors as early errors) | a resource limit is neither an early error nor a specified error; it surfaces where the construct is evaluated | `VmErrorKind::ResourceLimit` (NEW), `CompileErrorKind::ResourceLimit` (NEW) | ✓ | yes |
| WHATWG HTML §8.1.4.3 Creating scripts | *create a classic script* steps 10–11 (ParseScript; a list of errors becomes the error to rethrow) | a compile-time resource limit is not a parse error | `Vm::eval` mapping (`interpreter.rs:36`) | ✓ | yes |
| WHATWG HTML §10.3.1 Importing scripts and libraries | *import scripts into worker global scope* step 6.2 (run the classic script with rethrow errors) | rethrown as `RangeError`, not `TypeError` | `native_import_scripts` (`worker_scope.rs:481`) | ✓ | yes |
| WHATWG HTML §8.1.8.1 Event handlers | *get the current value of the event handler* step 3.7 (not parsable / early error: value null, report SyntaxError) and its note (value null does not remove the listener) | resource limit: no write, surfaces on the trigger | `lazy_compile_handler`, `ensure_event_handler_current`, `reconcile_event_handler` | ✓ | yes |
| WHATWG HTML §4.13.5 Upgrades | *upgrade an element* steps 1–9 and step 10 (threw branch: definition null, reaction queue emptied, rethrow) | entry failure at step 10.3 "constructing C": revert to the state before step 2 | `invoke_upgrade` (`upgrade.rs:216`, `:277`) | ✓ | yes |
| WHATWG HTML §4.13.5 Upgrades | *try to upgrade an element* steps 1–2 | the re-entry that retries a reverted element | `route_custom_element_upgrade` (`creation.rs:238`) | ✓ | no |
| WHATWG DOM §2.9 Dispatching events | *inner invoke* step 2.5 (once listener removed before the call), step 2.13 (call; report on throw) | entry failure: the once listener is restored at its position, error reported | `invoke_listeners_shared` (`event_target_dispatch.rs:460`, `:489`) | ✓ | yes |
| WHATWG HTML §7.2.5 The History interface | *shared history push/replace state steps* step 3 (StructuredSerializeForStorage, rethrow) and step 10 (URL and history update steps) | propagates; no entry committed | `structured_serialize_for_storage` (`structured_serialize.rs:78`), `history.rs:188` | ✓ | yes |
| WHATWG HTML §7.4.6.2 Updating the document | *restore the history object state* step 2 (catch, state null), reached from push/replace through *URL and history update steps* step 8 | push/replace: deserialised before commit, propagates; traversal (event-loop top): the spec branch | `structured_deserialize` (`structured_serialize.rs:97`) | ✓ | yes |
| WHATWG HTML §9.4.4 Message ports | *message port post message steps* step 5 (StructuredSerializeWithTransfer, rethrow) | rethrown as `RangeError`, not DataCloneError | `serialize_message` (`worker_scope.rs:386`) | ✓ | yes |
| IndexedDB-3 §6.2 Object store retrieval operations | *retrieve a value from an object store* step 4 (`! StructuredDeserialize`) | stated deviation at a `!`: the request fails with the `RangeError` | `json_to_js` (`indexeddb/value.rs:463`) and its seven callers | ✓ | yes |
| Service Workers §3.4.6 startMessages() | enable the client message queue; §3.4: an enabled queue is a task source | buffered messages are delivered as tasks, not synchronously | `enable_sw_messages`, `fire_message` (`deliver.rs:287`, `:359`) | ✓ | yes |
| Web Cryptography API §14.3.11 The wrapKey method | steps 2–4 (normalize for "wrapKey"; on error normalize for "encrypt"; on error reject) | step 3 is not taken; rejects with the `RangeError` | `normalize_wrap_with_fallback` (`subtle_crypto/ops.rs:66`) | ✓ | yes |
| Web Cryptography API §14.3.12 The unwrapKey method | steps 2–4 (same, "unwrapKey" / "decrypt") | same | same helper (`ops.rs:720` caller) | ✓ | yes |
| WHATWG HTML §8.1.4.6 Runtime script errors | *report an exception* steps 1–7 | the report channel of C-3; elidex's stand-in is stderr (`#11-embedder-diagnostic-channel`) | report sites of §7 | ✓ | no |

**Breadth**: K=6 specs (ECMA-262, WHATWG HTML, WHATWG DOM, IndexedDB-3, Service Workers, Web Cryptography
API), M=15 entries (verified 2026-11-08 by `python3 .claude/skills/elidex-plan-review/preflight.py` on this memo:
M=15, K=6, with `IndexedDB-3` and `Service Workers` reported as unmapped labels whose rows are not
webref-verified by it; both were looked up by hand, §12; re-recorded at landing, §11 L-9).
**Split decision**: K≥6 → SPLIT-DEFAULT. The breadth comes from many independent consumer sites, each
fixed by the same pattern against one predicate; the intersecting invariants are §2's, which sit in P0a.
Proposed: P0a (ECMA-262 §17 and HTML §8.1.4.3 rows: K=2, M=2) and P0b (the other thirteen rows: K=6,
M=13, appealed as one PR because every row applies C-2 / C-3 at one site with one predicate and one
injection harness; splitting P0b per spec would restate the contract per PR). PM decision requested (see
report).

### §3.1 User-input touch audit

Every touched consumer is reachable from script through a native whose input is script-controlled: the
caller's stack depth (which decides whether the check fails) and the data (handler source, the value
passed to `pushState` / `postMessage`, the stored IndexedDB value, the algorithm dictionary). Adjacent
pre-existing code: `Op::PushConst` propagates `load_constant`'s error with `?` out of `run()`, so an
invalid regexp literal's error is not catchable by the script that evaluates it (B-1, §6.4) — exposure
rises when PR-1's front door adds a `RangeError` to that site, so P0 routes it. The computed accessor key
of `Op::DefineComputedGetter` / `Op::DefineComputedSetter` is the same shape (B-2): a key whose `toString`
throws escapes `run()` today.

### §3.2 Cited, not touched

- ECMA-262 §7.4.11 IteratorClose step 5 (a throw completion wins over the `return` method's throw): the
  E rows of §6.3 implement the opposite precedence (§6.6 item 1).
- WHATWG HTML §10.2.4 *run a worker*: a top-level exception is reported and the worker keeps running;
  `worker_thread.rs:200` closes the worker (§6.6 item 3).
- WHATWG Streams: an exception from `pull` / `start` errors the stream (§6.3 R rows, C-4 non-re-entry).

## §4. The contract (canonical)

**C-1 Identity.** The guard's error is `VmErrorKind::ResourceLimit` (NEW). It is produced only by the
choke-point check of §5.3 and, after PR-1, by the walker and front-end probes and the regexp front door
(PR-1 §5–§6). It means: the step the probe guards did not start. At the compile boundary it is
`CompileErrorKind::ResourceLimit` (NEW). Every boundary that converts the error keeps the kind
(`Vm::eval`, `native_import_scripts`, the CP-3 conversions of §5.3). Once the error has been thrown into a
started script frame it becomes that frame's exception (escape rule, §5.2) and leaves the contract.

**C-2 No conversion.** No consumer turns a C-1 error into
(a) a success value: no `.ok()`, `unwrap_or(..)`, `_ => None` / default, no fallback that answers in its
place;
(b) another error class: no `DataCloneError`, `TypeError`, `NotSupportedError`, `CompileError` string in its
place;
(c) a durable fact about a subject that outlives the trigger: no "failed" mark, no listener or handler
removal, no null value, no negative cache, no stored record or session-history state derived from it, no
consumption of a once listener or a buffered message whose step never ran.

**C-3 Surfacing.** The error leaves the consumer by the trigger's own channel: `?`; a throw into the
running script (`throw_error`); rejection of the promise or request the trigger returned; or a report
(HTML *report an exception*, whose stand-in is stderr until `#11-embedder-diagnostic-channel`). The
script-visible class is `RangeError` with `STACK_EXHAUSTED_MSG` (NEW; "Maximum call stack size exceeded",
PM 421 (a)).

**C-4 Spec exception branches.** A *re-entry subject* is one whose spec algorithm re-attempts the failed
step on a later trigger, keyed on the subject's state: an event handler whose value is an internal raw
uncompiled handler (HTML §8.1.8.1 step 3), an element whose custom element state is "undefined" (HTML
§4.13.5 *try to upgrade an element*), a listener still in the listener list (DOM §2.9), a buffered
message not yet delivered. At a re-entry subject the consumer leaves the subject as it was before the
trigger and surfaces the error (C-3); the spec branch for an exception is not taken, because in the spec's
model the step ran (no spec step yields a resource limit, ECMA-262 §17 paragraphs 1 and 3). At a subject
without a re-entry (a promise, a stream, an IndexedDB request or transaction, a worker's or service
worker's top-level evaluation) the spec branch is the error channel and is taken with the `RangeError`.

**C-5 Census.** §8.

**Grounds.** (1) Spec: §17 paragraph 1 reports an error where its construct is evaluated; paragraph 3
forbids treating a non-early error as an early error, so a compile-time resource limit is not a
SyntaxError and HTML §8.1.8.1 step 3.7 (not parsable) does not apply to it. No spec step produces the
error, so every spec branch that would consume it was written for a different condition (umbrella §3,
PM 424). (2) ECS-native: a component write is a durable fact; reconcile a component only from a
definitive outcome (R2 A2's alternative), never from a transient condition of the Rust stack. (3) Security
by structure: the stack depth at which a page's handler first compiles or its element first upgrades is
under the control of any script on the page; without C-2 (c) one script can permanently disable another's
handlers and custom elements. (4) Fail-safe direction: an unknown consumer is red (C-5), not trusted.

**What the contract does not say.** It does not make a resource limit invisible: the script sees a
`RangeError` at the trigger. It does not retry anything itself: a reverted subject is retried by the
spec's own re-entry. It does not cover caps (D-P0-6).

## §5. Types and seams

### §5.1 Error identity

1. `VmErrorKind::ResourceLimit` (NEW), built by `VmError::resource_limit` (NEW) with message
   `STACK_EXHAUSTED_MSG` (NEW, `vm/error.rs`). `vm_error_to_thrown` materialises it as a `RangeError`
   object (`name` "RangeError", `message` the constant); `Display` prints `RangeError: <message>`.
   `VmError::is_resource_limit` (NEW) is true for that kind only.
2. `CompileError` gains `kind: CompileErrorKind` (NEW) with variants `Syntax` and `ResourceLimit`;
   `CompileError::is_resource_limit` (NEW). Every current construction site is `Syntax`.
3. Boundaries keep the kind: `Vm::eval` (`interpreter.rs:36`) maps `CompileErrorKind::ResourceLimit` to
   `VmError::resource_limit()` and `Syntax` to today's `VmErrorKind::CompileError`; `native_import_scripts`
   (`worker_scope.rs:481`) does the same before its `TypeError` mapping (D-11, D-13).
4. **Compile-channel aggregation (rule here, implementation in PR-1).** `compile_script` reports
   `CompileErrorKind::ResourceLimit` when any parse or scope error is a probe failure, whatever else the
   lists hold: a parse that hit the probe is not a verdict about the source. PR-1 adds
   `JsParseErrorKind::StackExhausted` (NEW: PR-1) and the one arm in `compile_script`, with its cells
   (PR-1 §8 `frontend_*`, `parser_*`).

### §5.2 The escape rule

`throw_error` on `VmInner` (`vm/ops.rs:155`) materialises the error and searches for a handler. Today, when
none is found inside the activation, it returns the **original** error (`Err(error)`, `ops.rs:165`). For
`VmErrorKind::ResourceLimit` it returns instead `VmError { kind: ThrowValue(thrown), message }` with the
same message. Consequence: an error that is still `ResourceLimit` when a consumer sees it never passed a
started script frame, so "did not start" (C-1) is exact; an error raised inside a constructor, a getter or
a callback body that ran reaches the consumer as that body's exception and C-4's spec branch applies.

The rule needs every runtime-channel error that leaves a started frame to pass `throw_error`. The census
checks this for the dispatch loop (§8.2 rule 6). Two sites do not today (§6.4): `Op::PushConst`
(`dispatch.rs:91`) propagates `load_constant`'s error with `?` (B-1), and `op_define_computed_accessor`
(`dispatch_objects.rs:227`, called with `?` at `dispatch.rs:842/845`) propagates `make_property_key`'s
ToPropertyKey error, which can come from script (`to_string` → ToPrimitive), with `?` (B-2). P0 routes both
through `throw_error`, which also makes today's errors at those sites catchable.

### §5.3 The choke-point check: placement (I-3, VM half)

`crate::vm::resource_limit::check_entry` (NEW) takes an `EntryKind` (NEW: `Call`, `Resume`, `WasmCall`,
`WasmInstantiate`) and returns `Result<(), VmError>`. P0 places it; PR-1 adds `ensure_stack(ask)` for
`Call` / `Resume` inside it, PR-1b for `WasmCall` / `WasmInstantiate`. The function is the one place where
the ask per kind is chosen. At P0 its body is the `#[cfg(test)]` injection of §5.4 only.

| Id | Site | Placement | What must not have happened yet | Why it holds |
|---|---|---|---|---|
| CP-1 | `VmInner::with_call_mode` (`interpreter.rs:160`) | first statement | the callee's first effect | `with_call_mode` restores frames, value stack, `gc_enabled`, `active_bound_key` and `completion_value` on every exit (`interpreter.rs:166-276`), so any position before `f(self, mode)` is equivalent; the check must precede `f` |
| CP-2 | `VmInner::resume_generator` (`natives_generator.rs:114`) | first statement | the generator taken and marked Running | a Running generator rejects the next resume ("already running") |
| CP-3a | `call_wasm_exported_function` (`host/wasm/exported_func.rs:268`) | before the `HostState` bind and `set_fuel` | store / instance mutation | PR-1b §2 |
| CP-3b | `instantiate_module` (`host/wasm/instance.rs:151`) | first statement, before the lazy `vm_wasm_runtime()` init, `Store::new` and the start function | runtime init, store creation, start-function effects | PR-1b §2 |

VM entry functions: `call`, `call_construct_native`, `run_function`, `construct_synchronous` and
`dispatch_super_inner` mutate nothing before `with_call_mode` (R1 Axis 2, verified at `47b6c934`; re-checked
at P0's base by `rl_cp1_entry_failure_no_callee_effect`, whose body enters through each of the first four).

**CP-3 conversions (named).** `call_wasm_exported_function` returns `Result<_, VmError>` and propagates the
check with `?`. `instantiate_module` returns `Result<ObjectId, JsValue>`; the check's error converts by
`.map_err(|e| ctx.vm.vm_error_to_thrown(&e))`, the in-file precedent (`instance.rs:162-163`, `coerce_import_object`), never through
`wasm_error_to_js_value` (which would make it `WebAssembly.RuntimeError`). Its two consumers propagate
(`instance.rs:69`) or reject (`namespace.rs:578`); both are C-3 channels, so the materialised form is
admissible there.

### §5.4 Test-only injection

`resource_limit::inject` (NEW: test-only) arms a thread-local: `InjectAt::Entry(EntryKind, n)` makes
the n-th `check_entry` of that kind on this thread return `VmError::resource_limit()`;
`InjectAt::Compile(n)`, `InjectAt::JsonParse(n)` and `InjectAt::JsonStringify(n)` do the same at the entry
of `compile_script`, `parse_json_str` and `stringify_for_structured_shortcut`. `resource_limit::count`
(NEW: test-only) records how many checks of a kind a body performs, so a cell derives its `n` from a
dry run instead of guessing. The three boundary points are not production probe sites: they let a cell
deliver C-1's kind to a consumer through the consumer's real call path before PR-1's walker and front-end
probes exist, and they stay after PR-1 so the consumer cells do not depend on stack sizes. They are the
only injection points; a consumer cell that needs another is a design change to this section.

## §6. The census

### §6.1 Population (by property)

**Err population.** Every production use, in a crate that can hold the error, of a call to a *carrier*
where the error is not propagated. A carrier is a function whose return type carries the error
(`Result<_, E>` with `E` ∈ {`VmError`, `CompileError`, `JsValue`, `StackExhausted` (NEW: PR-1),
`RegExpCompileError` (NEW: PR-1)}, or `EvalResult`) **and** that reaches a producer in the call graph.
Producers: functions that call `check_entry` (derived), plus the planned list of PR-1 / PR-1b probe
functions until they land (§8.3). A use is outside the population only when it propagates in a shape the
check recognises (§8.2 rule 2: `?`, `.map_err(<kind-preserving>)?`, `return` / tail in a carrier
function, a pure throw or `Err(e)` arm); every other use is a member and is classified (§6.3), and a member
may still propagate (class P: a match-arm tail, `return Some(Err(e))`, close-then-return). Crates: `crates/script/elidex-js/src`;
the `EvalResult` consumers in `crates/shell/elidex-shell/src` and `crates/script/elidex-script-session/src`
(the error leaves `elidex-js` only as `EvalResult`, `engine.rs:176-201` / `:332-357`, or as `()` from the
`ScriptEngine` methods).

**W population.** Functions in which a durable write (ECS component write, custom-element state
transition, listener removal) precedes a carrier call in the same body. This is the pre-call mutation
class of R1.

### §6.2 Method and counts

Prototype at `ce603c74` (verified 2026-11-08 via
`python3 -I H/scripts/err_consumer_census_proto.py <wt> crates/script/elidex-js`, log
`H/logs/err-census-p0.log`): 4307 function bodies, 2236 carriers by type, 1042 carriers that reach a
producer; 1575 uses propagate by `?` and 188 by return / tail, auto-recognised; 196 non-propagating uses in
`elidex-js` and 6 in the shell (`H/logs/err-census-p0-shell.log`, seeds `eval drain_timers eval_script`),
202 rows, each hand-classified (`H/scripts/err_census_classify.py`, output
`H/logs/err-census-p0-classified.tsv`). W population: 5 functions (verified 2026-11-08 via
`python3 -I H/scripts/err_write_before_call_proto.py <wt> crates/script/elidex-js`, log
`H/logs/err-write-before-call-p0.log`).

The compile-time channel has one consumer of the front end's error lists: `compile_script`
(`compiler/mod.rs:142-155`) is the only production caller of `parse_script` / `analyze_scopes`
(`git grep -nE '(parse_script|analyze_scopes)\(' -- crates` minus tests: `compiler/mod.rs` lines
143/149 and the definitions `lib.rs:91/104`). `compile_script` itself has 3 production callers
(`git grep -n 'compile_script(' -- crates` minus tests: `interpreter.rs:36`,
`event_handler_attrs.rs:677`, `worker_scope.rs:481`), all three in the census.

### §6.3 Classes

| Class | Meaning | Admissible | Rows |
|---|---|---|---|
| P | propagates in a shape the check does not auto-recognise (match-arm tail, `return Some(Err(e))`, close-then-return) | yes, row with the reason | 80 |
| T | thrown into the running script (`throw_error`) | yes; the check auto-recognises the pure arm (§8.2 rule 2), so these need no row (the prototype lists them) | 44 |
| R | the trigger's own channel: promise / request rejection, stream error, report (stderr), `EvalResult` | yes, row | 32 |
| S | dropped, nothing written; the report the spec asks for is missing | yes, row, with §6.6 home | 4 |
| E | replaced by another error, both surfaced | yes, row, with §6.6 home | 10 |
| N | not reachable by a script-controlled depth (argument class, a fresh thread's top, an embedder entry), or unreachable by construction | yes, row; top-only is checked (§8.2 rule 5) | 8 + 6 shell |
| X | test support (panics by design) | yes, row | 4 |
| D | definitive: D-state (C-2 c), D-sub (C-2 a), D-cls (C-2 b) | **no** (pending section until P0b, §8.2 rule 4) | 14 rows, 13 sites (§6.4 D-1…D-13; verified 2026-11-08, `H/logs/err-census-p0-classified.tsv`) |

Row lists (paths under `crates/script/elidex-js/src/` unless stated; lines at `ce603c74`):

- **R (32):** `engine.rs` 191 / 344; `vm/host/body_mixin.rs:289`; `vm/host/cache/natives.rs` 96 / 121 /
  145 / 198 / 252 / 287 / 340 / 382 / 420 (`finish` rejects); `vm/host/custom_elements/creation.rs:238`,
  `flush.rs` 104 / 183 / 202 / 234, `lookup.rs:183` (report); `vm/host/fetch/mod.rs` 183 / 204;
  `vm/host/observer_common.rs:295` (report; records are delivered from the embedder's entry, top-only);
  `vm/host/readable_stream/controller.rs:135`, `mod.rs` 410 / 639 (stream error / cancel rejection, C-4
  non-re-entry); `vm/host/service_worker/container.rs` 109 / 218; `vm/host/wasm/namespace.rs:578`;
  `vm/natives_generator.rs:472` (wrapper rejection; residual §6.6 item 5); `vm/natives_promise.rs` 648 /
  689 / 747; `vm/natives_timer.rs:370`.
- **S (4):** `engine.rs:275` (`call_listener`'s `let _ =`, the shell's UA dispatch, top-only);
  `vm/coerce.rs:246` (`to_display_string`, console display); `vm/host/custom_elements/flush.rs:70`
  (`invoke_one` reports inside); `vm/natives_promise.rs:591` (unhandled-rejection listener, top-only).
- **E (10):** `vm/dispatch_iter.rs` 52 / 309; `vm/ops.rs:60`; `vm/host/typed_array_static.rs` 743 / 801;
  `vm/natives_array_hof.rs:485`; `vm/webidl_sequence.rs` 141 / 149; `vm/host/url_search_params.rs:318`;
  `vm/host/structured_clone.rs:1064`. A `return` method called at the same depth as the failed step fails
  at its own entry the same way, so the C-1 error is what normally survives; the precedence itself is
  §6.6 item 1.
- **N (8 + 6):** `vm/ops_element.rs:134` (`to_number` of a String calls no script);
  `vm/interpreter.rs:894` (`drive_async_coroutine` turns every error into a rejection,
  `natives_generator.rs:520-525`); `vm/host/worker_scope.rs:277` (worker loop and the embedder's
  `drain_worker_messages`); `vm/sw_thread.rs` 217 / 474 and `vm/worker_thread.rs` 200 / 227 / 259 (top of a
  fresh thread); shell: `content/event_loop.rs:284`, `content/iframe/render.rs:93`,
  `content/iframe/thread.rs:100`, `lib.rs:310`, `pipeline.rs` 357 / 359 (event-loop top, `EvalResult`
  discarded).
- **X (4):** `engine.rs` 63 / 78 (embedder-test helpers); `vm/host/subtle_crypto/differential.rs` 32 / 36
  (`#[cfg(test)] mod differential;`, `subtle_crypto/mod.rs:62-63`; the tracked check excludes such files,
  §8.2 rule 1).
- **P (80) and T (44):** listed in `H/logs/err-census-p0-classified.tsv`; T = `vm/dispatch.rs` (40),
  `dispatch_objects.rs` 48 / 80, `dispatch_class.rs:110`, `dispatch_helpers.rs:383`.

W population (5): `custom_elements/upgrade.rs:216` (D-3/D-4); `event_target_dispatch.rs:489` (D-5);
`custom_elements/creation.rs:238` (the `is_value` reset is derived from the registry, not from the
upgrade's outcome: admissible); `html_input_value.rs:353` (a Number to string, no script);
`natives_promise.rs:591` (top-only).

### §6.4 Definitive rows and the two bypasses

| Id | Site | Carrier | What the error becomes today | Class | Governing step | Lane |
|---|---|---|---|---|---|---|
| D-1 | `vm/host/event_handler_attrs.rs:677` `lazy_compile_handler` | `compile_script` | `.ok()?` → `None` → `ensure_event_handler_current` (`:548-552`) removes the listener; the source was already taken by `reconcile_event_handler` (`event_handler_reconcile.rs:67-95`, `take_uncompiled`) | D-state | HTML §8.1.8.1 step 3.7 (does not apply) | L3 |
| D-2 | `event_handler_attrs.rs:678` | `run_script` (CP-1 through `run_function`) | `_ => None` → same | D-state | same | L3 |
| D-3 | `vm/host/custom_elements/upgrade.rs:216` (Phase 2, `prototype` read) | `get_property_value` (CP-1 through an accessor) | `finalize_failure_shim` → Failed | D-state | HTML §4.13.5 *upgrade an element* | L3 |
| D-4 | `upgrade.rs:277` (construct) | `construct_synchronous` (CP-1) | `finalize_failure_shim` → Failed, after `enter_constructor` (`:249`, precustomized) and the wrapper prototype splice (`:244`) | D-state | HTML §4.13.5 step 10 | L3 |
| D-5 | `vm/host/event_target_dispatch.rs:489` | `call_value` (CP-1) | `.is_err()` → `threw`; the once listener was removed at `:460` and stays removed though it never ran | D-state | DOM §2.9 *inner invoke* steps 2.5, 2.13 | L3 |
| D-6 | `vm/host/structured_serialize.rs:78` | `stringify_for_structured_shortcut` | `_ => None`: `pushState` commits an entry with no state | D-sub | HTML §7.2.5 step 3 (rethrow) | L4 (no driver) |
| D-7 | `structured_serialize.rs:97` (from push/replace) | `parse_json_str` | `unwrap_or(Null)`: `history.state` null while the entry holds state | D-sub | HTML §7.4.6.2 step 2, via *URL and history update steps* step 8 | L4 (no driver) |
| D-8 | `vm/host/indexeddb/value.rs:463` `json_to_js` | `parse_json_str` | `unwrap_or(Undefined)`: `get` / `getAll` / index reads / cursor value succeed with `undefined` | D-sub | IndexedDB-3 §6.2 step 4 | unassigned |
| D-9 | `vm/host/service_worker/deliver.rs:359` `fire_message` (reached synchronously from `startMessages()`, `deliver.rs:287-297`) | `parse_json_str` | `unwrap_or(Undefined)`: the message is consumed with `data` undefined | D-sub | Service Workers §3.4.6, §3.4 | unassigned |
| D-10 | `vm/host/worker_scope.rs:386` `serialize_message` | `stringify_for_structured_shortcut` | `Err(_)` → DataCloneError | D-cls | HTML §9.4.4 step 5 | unassigned (brief: L3) |
| D-11 | `worker_scope.rs:481` `native_import_scripts` | `compile_script` | `TypeError` | D-cls | HTML §10.3.1 step 6.2 | unassigned (brief: L3) |
| D-12 | `vm/host/subtle_crypto/ops.rs:66` (+ `:68`, same site) | `marshal_normalize` | the step-3 fallback runs; may answer NotSupportedError or a different normalisation | D-cls | Web Cryptography API §14.3.11 / §14.3.12 steps 2–4 | unassigned |
| D-13 | `vm/interpreter.rs:36` `Vm::eval` | `compile_script` | `VmErrorKind::CompileError` (event-loop top; identity only) | D-cls | HTML §8.1.4.3 steps 10–11 | VM P4 |
| B-1 | `vm/dispatch.rs:91` `Op::PushConst` | `load_constant` | `?` out of `run()`: not catchable, bypasses the escape rule | bypass | ECMA-262 §17 paragraph 1 | VM P4 |
| B-2 | `vm/dispatch_objects.rs:227` `op_define_computed_accessor` (`dispatch.rs:842/845`) | `make_property_key` | same | bypass | ECMA-262 §17 paragraph 1 | VM P4 |

D-3's reachability depends on a constructor whose `prototype` lookup reaches an accessor (§12 item 2).
D-7 and D-9 are caller-depth reachable: `native_history_replace_state` → `state_mutate` →
`seed_history_state` → `structured_deserialize` (`history.rs:248-249`, after `current_url` is already set), and `native_container_start_messages` →
`enable_sw_messages` → `fire_message` (name-based reverse path, prototype; confirmed by reading
`deliver.rs:287-297`).

### §6.5 Sites per lane

- **L3 (D-1…D-5, five sites, verified 2026-11-08 against §6.4; files of L3's charter: event handlers, custom elements, DOM dispatch):** D-1, D-2
  (`vm/host/event_handler_attrs.rs`, with `crates/script/elidex-script-session/src/event_handler_reconcile.rs`
  and `event_listener.rs`), D-3, D-4 (`vm/host/custom_elements/upgrade.rs`, with
  `crates/dom/elidex-custom-elements/src/upgrade.rs`), D-5 (`vm/host/event_target_dispatch.rs`,
  `vm/host/dispatch_target.rs`).
- **L4, no driver (2):** D-6, D-7 (`vm/host/structured_serialize.rs`, `vm/host/history.rs`).
- **Unassigned (D-8…D-12, five sites and six census rows, verified 2026-11-08 against §6.4):** D-8 (IndexedDB), D-9 (service worker), D-10, D-11 (worker; the brief
  lists worker under L3), D-12 (Web Crypto).
- **VM P4 (3):** D-13, B-1, B-2.

PM decision requested (see report): the owner of each unassigned site, and who edits the L3 / L4 files
in P0b.

### §6.6 Pre-existing deviations seen, not changed by P0

1. **IteratorClose precedence.** ECMA-262 §7.4.11 step 5 returns the original throw completion before
   step 6 considers the `return` method's throw; the ten E rows let the `return` method's error win
   (comments: `dispatch_iter.rs:54-55`, `typed_array_static.rs` at 743, `webidl_sequence.rs` at 141).
   Not a C-2 breach in practice (§6.3 E); a spec deviation of its own. Slot requested (see report).
2. **Unreported listener and callback errors.** The four S rows drop the error with no report; the
   stderr stand-in is used elsewhere (`flush.rs`, `observer_common.rs`). Home:
   `#11-embedder-diagnostic-channel` (facet added at landing, §11 L-5).
3. **A worker's top-level exception closes the worker** (`worker_thread.rs:200-204`); HTML §10.2.4
   reports it and keeps running. Slot requested (see report).
4. **Not-parsable inline handler removes its listener** (`event_handler_attrs.rs:548-552`); HTML §8.1.8.1
   step 3.7's note says the value becomes null and the listener stays. D-1's fix rewrites this branch;
   whether P0b also implements the not-parsable arm per step 3.7 (value null via the cleared state, listener
   kept, SyntaxError reported) is PM decision requested (see report). Recommendation: yes, since the
   rewritten branch would otherwise be written knowingly against the note.
5. **A failed `AsyncDriverStep` resume** (R2 A5-IMP4): CP-2 fails, the wrapper is rejected
   (`natives_generator.rs:520-525`), the generator stays suspended, a `finally` in the body does not run.
   The row is R (the wrapper is the trigger's channel); running `finally` needs the stack the step does not
   have, and re-queueing the step would loop inside one drain. Stated, with a cell pinning the outcome
   (`rl_async_step_entry_failure_rejects_wrapper`). Slot or acceptance: PM decision requested (see report).
6. **Synchronous `startMessages()` flush** (Service Workers §3.4: an enabled client message queue is a task
   source). P0b removes it as D-9's fix (§7.7); recorded here because the fix is spec conformance, not only C-2.

## §7. Fixes

Each fix applies C-2 / C-3 / C-4 at one site, with `is_resource_limit()` as the only test.

### §7.1 D-1, D-2: lazy event-handler compile

`reconcile_event_handler` stops taking the source before the compile: it returns
`HandlerReconcile::Compile(source)` from a read (`uncompiled_source`), and the component is written only on
a definitive outcome. `lazy_compile_handler` returns `Result<ObjectId, HandlerCompileError>` (NEW:
`NotParsable(CompileError)`, `ResourceLimit(VmError)`). `ensure_event_handler_current` returns
`Result<(), VmError>`:

- `Ok(callable)` → `set_handler_callable`, then `clear_uncompiled` on the `EventListeners` component;
- `NotParsable` → today's branch, or the step-3.7 arm if PM accepts §6.6 item 4;
- `ResourceLimit` → no write; `Err(VmError::resource_limit())`.

Callers: `dispatch_target.rs:180` (VM dispatch: report, `threw = true`, skip the invocation),
`engine.rs:234` (UA dispatch, event-loop top: report, skip), `natives_promise.rs:543` (unhandled-rejection
handler, top: report, skip), and the IDL getter path (throws; §12 item 1). Engine-independent half:
`elidex-script-session` (`event_handler_reconcile.rs`, `event_listener.rs`).

### §7.2 D-3, D-4: custom-element upgrade

`elidex_custom_elements::abandon_upgrade` (NEW) (`crates/dom/elidex-custom-elements/src/upgrade.rs`) sets the
state back to "undefined" (the state before *upgrade an element* step 2) and enqueues nothing. In
`invoke_upgrade`: Phase 2 (`:216`) on a C-1 error returns it without a state write; Phase 4 (`:277`) on a C-1
error restores the wrapper's previous `[[Prototype]]` (recorded before the splice at `:244`), calls
`abandon_upgrade`, and returns the error. The construction stack is popped by the existing RAII guard. A
`ThrowValue` error keeps today's branch (`finalize_failure_shim`). Callers already report (§6.3 R). The
retry is the spec's: insertion (*try to upgrade an element*, `creation.rs` `route_custom_element_upgrade`)
or `customElements.upgrade()`.

### §7.3 D-5: once listener on a never-started call

`EventListeners::remove_with_position` (NEW) returns the removed entry and its index at `:460`; on a C-1
error from `call_value` (`:489`), `EventListeners::reinsert` (NEW) puts it back at that index, the
post-call `remove_listener_and_prune_back_ref` for `once` (`:508-510`) is skipped, the error is reported
and `threw` is set. Nothing ran, so no re-entrant dispatch can have observed the removal.

### §7.4 D-6, D-7: `pushState` / `replaceState`

`structured_serialize_for_storage` returns `Result<Option<Vec<u8>>, VmError>`: a C-1 error propagates (HTML
§7.2.5 step 3, rethrow; no entry is committed); every other failure keeps today's degrade, owned by
`#11-history-state-structured-serialize-fidelity`. The push/replace path deserialises the serialized bytes
right after step 3 and before step 10 commits, and uses that value at *URL and history update steps* step 8
(no second deserialise); a C-1 error there propagates before any commit. `structured_deserialize` returns
`Result`; the traversal callers (`seed_history_state` at document construction, `history_events.rs:205`)
apply step 2's catch-null explicitly (event-loop top, N).

### §7.5 D-8: IndexedDB reads

`json_to_js` returns `Result<JsValue, VmError>`. Its seven callers (`git grep -n 'json_to_js(' -- crates/script/elidex-js/src`
minus the definition: `indexeddb/cursor.rs:145`, `index.rs` 246 / 262 / 333, `object_store.rs` 365 / 373 / 477;
verified 2026-11-08) turn a C-1 error into `DeferredOutcome::Error` with the materialised `RangeError`, so
the request fails on its own channel (IndexedDB-3 §6.2 step 4 is a `!`: a stated resource-limit deviation,
like step 3's NotReadableError result).

### §7.6 D-10, D-11, D-12, D-13

- D-10: `serialize_message` gains `Err(e) if e.is_resource_limit() => Err(e)` before the DataCloneError
  arm.
- D-11: `native_import_scripts` maps `CompileErrorKind::ResourceLimit` to `VmError::resource_limit()`
  before the `TypeError` mapping (the `TypeError`-for-SyntaxError mapping is pre-existing, not P0's).
- D-12: `normalize_wrap_with_fallback` returns a C-1 error from the primary normalisation without
  running the fallback (step 3 is for errors the spec defines).
- D-13: §5.1 item 3.

### §7.7 D-9: service-worker messages

`enable_sw_messages` queues each buffered message as a task on the client's pending-task queue instead of
calling `fire_message` synchronously (Service Workers §3.4). `fire_message` then runs only from the event
loop (top-only, N); its `unwrap_or(Undefined)` for other deserialise failures is the pre-existing
substitution of the SW message path (spec: `messageerror`), recorded in §11 L-6.

### §7.8 B-1, B-2: errors that leave `run()` without `throw_error`

`Op::PushConst` matches `load_constant`'s result, and `op_define_computed_accessor` matches
`make_property_key`'s, and each sends an error through `throw_error`, like every other fallible op in
`run()`.

## §8. The tracked check

### §8.1 The check's files (all NEW)

- `.claude/tools/_rust_callgraph.py`: the shared lexer (comments, strings and raw strings stripped with
  line numbers kept; `#[cfg(test)] mod x { }` blocks and `#[cfg(test)] mod x;` files excluded), function
  extraction with signature and return type, name-based call edges, carrier closure. PR-1's
  `recursion_cutcheck.py` builds on it (one call-graph substrate for both checks).
- `.claude/tools/err_consumer_census.py`: the census over §6.1's populations.
- `.claude/tools/err-consumer-census.tsv`: one row per non-auto member: path, enclosing function, callee,
  shape, class, reason. Keyed by (path, function, callee, shape, occurrence), not by line.
- `.claude/tools/test_err_consumer_census.py`: tracked `test_*.py`, collected by `scripts/python-suites.sh`
  into the `tools` job; runs the check on the tree (rc 0) and the controls of §8.4.

### §8.2 Rules

1. Population per §6.1, recomputed from the code on every run; test files and `#[cfg(test)]`-declared
   module files are excluded by the lexer, not by a list.
2. Auto-recognised shapes are narrow: `?`; `.map_err(f)?` where `f` is `vm_error_to_thrown` or a `From` /
   `Into` conversion; `return` / tail in a carrier function; an arm (or an `if let Err(e) = ..`
   block) whose whole body is `self.throw_error(e, ..)?` or `return Err(e)` or `Err(e)`. Anything else is a member.
3. A member without a row is red; a row without a member is red (the allowlist cannot drift either way).
4. Admissible classes: P, T, R, S, E, N, X. A D row is red, except in the `# pending` section, whose rows
   name their fixing PR; the section may only shrink, and PR-1's precondition is an empty section (L-1).
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
7. W population: every function of §6.1's W population has a row (class D pending, or admissible with a
   reason).

### §8.3 Producers and the planned list

Producers are derived: functions that call `check_entry`, `ensure_stack` (NEW: PR-1), or construct
`VmError::resource_limit` / `CompileErrorKind::ResourceLimit`. Until PR-1 / PR-1b land, the check also
reads `PLANNED_PRODUCERS`, a constant listing PR-1 §5.2's walker functions, the compile front end's entry
(`compile_script`), the regexp compile sites of PR-1 §6.1 (`load_constant`, `native_string_match`,
`native_string_search`) and PR-1b's `call_wasm_exported_function` / `instantiate_module`. A planned name
that is also derived is red ("remove it"), so the list only shrinks and is empty after PR-1b.

### §8.4 Controls and mutants

Controls (tempfile fixtures), each red: `census_rejects_unknown_member` (an added `.ok()` on a carrier
call); `census_rejects_stale_row`; `census_rejects_definitive_row` (a D row outside `# pending`);
`census_planned_producer_already_derived`; `census_top_only_reachable_from_native`;
`census_run_question_mark_on_carrier` (a new `?` on a carrier in `run()`); `census_w_function_without_row`. `--mutants` runs the
check with each rule disabled and asserts the matching control turns green, so a rule that stops working
is seen (the plan-memo-umbrella-check precedent).

### §8.5 Stated limits and their home

The check does not see: calls through function pointers, trait objects and closures passed across
functions (the `NativeFn` table, `fire_vm_message_event`'s closure argument is seen only because its body is
in the caller); writes performed inside a callee before the carrier call (D-1's `take_uncompiled` sat in
`reconcile_event_handler`, another crate); macros. Owner VM P4; re-evaluated with PR-1's cut-check limits
when `#11-vm-native-callback-reentry-single-loop` is designed; ledger home §11 L-7.

## §9. Cells (written first)

New file `crates/script/elidex-js/src/vm/tests/tests_resource_limit.rs` (NEW). Each cell arms §5.4's
injection with an `n` taken from `resource_limit::count` on a dry run of the same body.

| Cell | Body | Expect |
|---|---|---|
| `rl_identity_predicates` | build each error form | `is_resource_limit` true only for `VmError::resource_limit()`; a `ThrowValue` and a `RangeError` kind are false; `CompileError` kinds likewise; the materialised object is a `RangeError` with `STACK_EXHAUSTED_MSG` |
| `rl_cp1_entry_failure_no_callee_effect` | `let n = 0; function g() { n++ }`; inject `Entry(Call, 1)`; `try { [1].forEach(g) } catch (e) { r = e.name }`; the same through `call`, `call_construct_native`, `run_function`, `construct_synchronous` from Rust | `r === 'RangeError'`, `n === 0`, frame and value-stack depth unchanged; a second run gives `n === 1` |
| `rl_cp2_generator_resumable_after_entry_failure` | `function* g() { yield 1 } const it = g()`; inject `Entry(Resume, 1)`; `it.next()` throws; then `it.next()` | first: `RangeError`; second: `{ value: 1, done: false }` |
| `rl_async_step_entry_failure_rejects_wrapper` | an async function awaiting a resolved promise; inject `Entry(Resume, n)` for the step | the wrapper rejects with `RangeError`; the body's `finally` did not run (§6.6 item 5) |
| `rl_cp3a_wasm_call_entry_failure_retry` | an exported function; inject `Entry(WasmCall, 1)` | `RangeError` (not `WebAssembly.RuntimeError`); the next call returns its value |
| `rl_cp3b_instantiate_entry_failure` | inject `Entry(WasmInstantiate, 1)` for `new WebAssembly.Instance(m)`, then for `WebAssembly.instantiate(m)` | sync: `RangeError`; async: rejects `RangeError`; an un-injected retry instantiates |
| `rl_escape_rule_materializes_in_started_frame` | `function f() { [1].forEach(x => x) }`; from Rust `vm.call(f)` with `Entry(Call, 2)`, then with `Entry(Call, 1)` | `n = 2`: `Err` kind `ThrowValue` (a `RangeError` object), not `ResourceLimit`; `n = 1`: kind `ResourceLimit` |
| `rl_vm_eval_compile_resource_limit_kind` | inject `Compile(1)`; `vm.eval("1")`; also through the `ScriptEngine` `eval` of `ElidexJsEngine` | kind `ResourceLimit`; the `EvalResult` message starts with `RangeError` |
| `rl_regexp_literal_error_is_catchable` | `try { /a{2,1}/ } catch (e) { r = e.name }` (a pattern `regress` 0.11.1 rejects; §12 item 4) | `r` is set (today the error leaves `run()` uncaught) |
| `rl_computed_accessor_key_error_is_catchable` | `try { ({ get [{ toString() { throw 1 } }]() {} }) } catch (e) { r = e }`; and with `Entry(Call, 1)` on the `toString` | `r === 1`; with the injection, `r` is a `RangeError` |
| `rl_handler_compile_failure_keeps_source` | `<div onclick="c++">` with an `addEventListener` before and after it; inject `Compile(1)`; `div.dispatchEvent(new Event('click'))` twice | first dispatch: handler not run, the other two run, error reported; second: handler runs once, in its original position |
| `rl_handler_closure_creation_failure_keeps_source` | same, inject `Entry(Call, n)` at `run_script`'s entry inside the lazy compile | same |
| `rl_handler_getter_compile_failure_throws` | inject `Compile(1)`; `div.onclick`; then `div.onclick` | first throws `RangeError`; second is a function (§12 item 1 decides the path) |
| `rl_ce_upgrade_construct_entry_failure_reverts` | an `x-a` element in the document; `customElements.define('x-a', C)` with `Entry(Call, n)` at the construct | `C` ran 0 times; `el.matches(':defined') === false`; `Object.getPrototypeOf(el) === HTMLElement.prototype`; error reported; then `customElements.upgrade(el)` → `C` ran once, `el instanceof C` |
| `rl_ce_upgrade_after_constructor_started_takes_threw_branch` | `C`'s constructor calls `[1].forEach(x => x)`; inject at that callback's entry | today's threw branch (state Failed), `C` ran once: the escape rule's boundary |
| `rl_ce_upgrade_prototype_read_entry_failure` | a constructor whose `prototype` lookup reaches an accessor (§12 item 2) | no state write; retry upgrades |
| `rl_dispatch_once_listener_survives_entry_failure` | listeners A (once), B (once), C on one target; inject `Entry(Call, 1)` for A; dispatch twice | first: A not run, B and C run, error reported; second: A runs, then C; A and B are gone afterwards |
| `rl_push_state_resource_limit_throws_no_entry` | `history.pushState({ get x() { return 1 } }, '')` with `Entry(Call, 1)`; then `JsonParse(1)` | both throw `RangeError`; `history.length` and `history.state` unchanged |
| `rl_idb_get_resource_limit_errors_request` | a stored record; inject `JsonParse(1)`; `store.get(k)`, `getAll`, an index `get`, a cursor | each request errors with `RangeError` (no success with `undefined`); un-injected reads return the value |
| `rl_sw_start_messages_is_async` | buffered client messages; `navigator.serviceWorker.startMessages()` | no `message` event during the call; events fire after the task drain |
| `rl_worker_post_message_resource_limit_is_range_error` | worker scope `postMessage({ get x() { return 1 } })` with `Entry(Call, 1)`; then `JsonStringify(1)` | `RangeError`, not DataCloneError |
| `rl_import_scripts_compile_resource_limit_is_range_error` | `importScripts(url)` with `Compile(1)` (§12 item 3) | `RangeError`, not `TypeError` |
| `rl_wrap_key_resource_limit_skips_fallback` | `wrapKey` / `unwrapKey` with `{ get name() { k++; return 'AES-KW' } }` and `Entry(Call, 1)` | rejects `RangeError`; `k === 1` |

## §10. Mutants (run on the final head, debug)

| Mutant | Change | Dies |
|---|---|---|
| M-ID | `is_resource_limit` always false | every consumer cell of §9 |
| M-ESC | escape rule removed | `rl_escape_rule_materializes_in_started_frame`, `rl_ce_upgrade_after_constructor_started_takes_threw_branch` |
| M-B1 | `Op::PushConst` back to `?` | `rl_regexp_literal_error_is_catchable` |
| M-B2 | `op_define_computed_accessor` back to `?` | `rl_computed_accessor_key_error_is_catchable` |
| M-P-CP1 | CP-1 check after `f(self, mode)` | `rl_cp1_entry_failure_no_callee_effect` |
| M-P-CP1s | CP-1 check after the `entry_frames` snapshot | **equivalent** (the exit path restores the snapshot); recorded, answers R2 S-2 for CP-1 |
| M-P-CP2 | CP-2 check after the generator is marked Running | `rl_cp2_generator_resumable_after_entry_failure` |
| M-P-CP3a / M-P-CP3b | CP-3 checks after the `HostState` bind / after `Store::new` and the start function | `rl_cp3a_*` / `rl_cp3b_*` if observable; a survivor is recorded as equivalent with the reason (§12 item 5) |
| M-EV | `Vm::eval` maps to `CompileError` | `rl_vm_eval_compile_resource_limit_kind` |
| M-H1 / M-H2 | removal on resource limit / `take_uncompiled` before compile | `rl_handler_*` |
| M-CE1 / M-CE2 | `finalize_failure` instead of `abandon_upgrade` / no prototype restore | `rl_ce_upgrade_construct_entry_failure_reverts` |
| M-D1 / M-D2 | no reinsert / reinsert at the end | `rl_dispatch_once_listener_survives_entry_failure` |
| M-HS1 / M-HS2 | serialize error → `None` / deserialise after commit | `rl_push_state_resource_limit_throws_no_entry` |
| M-IDB | `unwrap_or(Undefined)` back | `rl_idb_get_resource_limit_errors_request` |
| M-SW | synchronous flush back | `rl_sw_start_messages_is_async` |
| M-W1 / M-W2 / M-WC | DataCloneError first / `TypeError` / fallback on resource limit | the matching cell |
| M-CC | each §8.2 rule disabled | the matching control (§8.4) |

## §11. Landing obligations

What P0's PRs (or, where marked, the PM / PMO at landing) do. This memo edits no ledger.

- **L-1. PR-1 precondition**: P0 landed and the census `# pending` section empty; PR-1's body records
  `git merge-base --is-ancestor <P0 sha> HEAD` → 0.
- **L-2. Lane notices (PM)**: the L3 sites of §6.5 to L3; D-6 / D-7 (L4, no driver) and the unassigned
  sites to their owners once named.
- **L-3. `#11-history-state-structured-serialize-fidelity` (PM / PMO)**: the entry gains "resource-limit
  errors propagate from the interim serializer and from the push/replace deserialise (stack-safety P0
  §7.4); the degrade for other failures remains this slot's".
- **L-4. `#11-idb-structured-clone-storage` (PM / PMO)**: "`json_to_js` is fallible (P0 §7.5)".
- **L-5. `#11-embedder-diagnostic-channel` (PM / PMO)**: facet "listener / callback errors dropped
  without a report: `engine.rs:275`, `custom_elements/flush.rs:70`, `natives_promise.rs:591`,
  `coerce.rs:246` (P0 §6.6 item 2)".
- **L-6. SW message deserialise substitution (PM / PMO)**: `fire_message`'s `unwrap_or(Undefined)` for a
  non-resource deserialise failure (spec: `messageerror`) is recorded on the SW slot the PM names.
- **L-7. Census limits (PM / PMO)**: §8.5's limits join PR-1 §5.3's on the
  `#11-vm-native-callback-reentry-single-loop` entry, with re-eval 2026-12-31.
- **L-8. Slots requested (PM)**: §6.6 items 1, 3, 5 (and 4 if not taken into P0b).
- **L-9. Breadth record**: `python3 .claude/skills/elidex-plan-review/preflight.py` on this memo at each
  PR's head; rc and K / M as printed.

## §12. Unverified facts (each decided at implementation, before its cell is written)

1. The IDL getter path (`div.onclick`) to `lazy_compile_handler`: `ensure_event_handler_current` has three
   production callers (`git grep -n 'ensure_event_handler_current(' -- crates/script/elidex-js/src` minus the
   definition: `engine.rs:234`, `dispatch_target.rs:180`, `natives_promise.rs:543`; verified 2026-11-08);
   which of them, if any, the getter goes through is not traced.
2. Whether a custom-element constructor's `prototype` lookup can reach an accessor in elidex (no `Proxy`
   kind in `vm/object_kind.rs`; a bound constructor with an accessor on `Function.prototype` is the
   candidate). If not, D-3 is N (unreachable) and its cell is dropped with the reason; the shared fix stays.
3. Whether the worker test harness can run `importScripts` against a served script.
4. The `regress` 0.11.1 pattern for `rl_regexp_literal_error_is_catchable`.
5. Whether M-P-CP3a / M-P-CP3b are observable.
6. Every N row claiming top-only is re-checked by §8.2 rule 5 at P0's base.

webref lookups behind §3 (2026-11-08): `body ecma262 sec-error-handling-and-language-extensions`;
`body ecma262 IteratorClose` (`aoid ecma262 IteratorClose` → §7.4.11); `dfn html 'get the current value of
the event handler'` → §8.1.8.1; `dfn html 'upgrade an element'` / `'try to upgrade an element'` → §4.13.5;
`dfn dom 'inner invoke'` → §2.9; `dfn html 'shared history push/replace state steps'` → §7.2.5;
`dfn html 'restore the history object state'` → §7.4.6.2; `body html url-and-history-update-steps` (step 8);
`dfn html 'message port post message steps'` → §9.4.4; `dfn html 'import scripts into worker global scope'`
→ §10.3.1; `dfn html 'create a classic script'` → §8.1.4.3; `dfn html 'report an exception'` → §8.1.4.6;
`dfn html 'run a worker'` → §10.2.4; `dfn IndexedDB-3 'retrieve a value from an object store'` → §6.2;
`dfn service-workers 'startMessages()'` → §3.4.6, `body service-workers dfn-client-message-queue`;
`dfn webcrypto 'wrapKey()'` → §14.3.11, `dfn webcrypto-2 'unwrapKey()'` → §14.3.12.
