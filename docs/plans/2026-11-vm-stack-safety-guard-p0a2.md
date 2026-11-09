# VM stack-safety guard — P0a-2 plan (the census checker: checked admission reasons, resolved call graph, allowlist, `# pending`)

Status: **plan (pre-implementation)**, per-PR slice of the [P0 umbrella](2026-11-vm-stack-safety-guard-p0.md)
(PM 509; re-sliced from P0a by PM 544). `/elidex-plan-review` required before code. Owner: VM P4. Base:
`ce603c74`; every code anchor below is at that commit (the branch head `d7ec4761` has identical code).
Provenance root `H` and the census prototypes are as in the P0 umbrella header; the prototypes are
`H/scripts/err_consumer_census_proto.py` (+ `err_census_classify.py`, `err_write_before_call_proto.py`,
`err_boundary_census_proto.py`, `err_loop_question_proto.py`) and, for this revision,
`H/scripts/err_reason_check_proto.py` (the checked-reason re-run, PM 544); their logs are under `H/logs/`
(`err-census-p0.log`, `err-census-p0-shell.log`, `err-census-p0-classified.tsv`,
`err-write-before-call-p0.log`, `err-boundary-census-p0.log`, `err-loop-question-p0a.log`,
`err-loop-question-p0a-classified.tsv`, `err-reason-check-p0a2.log`, `err-reason-check-p0a2-nostdskip.log`).
The prototypes are not the deliverable: P0a-2 ships the tracked check of §4–§5, and every number below is
re-derived by that check at P0a-2's base.

The contract C-1…C-5 is the P0 umbrella §4; the census definition (populations, classes) is the P0
umbrella §5; the semantics the check tests against (the entered mark, `ScriptFailure`, the kind-bearing
boundaries) are [P0a-1](2026-11-vm-stack-safety-guard-p0a1.md) §4. This memo does not restate them.
Edges, schedule and the other slices: P0 umbrella §1. P0a-2 lands after P0a-1 and before every P0b slice.

## §0. Charter

P0a-2 ships the tracked census check (`.claude/tools/err_consumer_census.py`), its shared call-graph
substrate, the allowlist with its `# pending` section, the controls and the mutants. **The rule of this
slice (PM 544):** every admission reason is checked mechanically. A row carries a reason code; the check
has one predicate per code; a row whose predicate fails, and a row whose code has no predicate, is a **D
row** (red outside `# pending`). The unknown falls to the checked side (fail-safe direction,
`memory/feedback_fail-safe-direction-before-list-completeness.md`); a reason the check cannot see is not an
admission (`memory/feedback_checks-must-not-be-defined-by-the-symptom-vocabulary.md`).

**Why (PM 544).** R4 Axis 2 IMP-2: `natives_promise.rs:591` was admitted as S "nothing written, top-only"
by hand, and both claims were false on the code (D-15, P0a-1 §5.4); rule 5 checked top-only only on N rows.
R4 Axis 2 IMP-3: rule 10 cut collision edges name × name, which can turn an unknown into trusted. R4 Axis 1
MIN-2: rule 9 skipped std traits by an unstated list. Each is the same defect: a reason that lived in prose.

## §1. Scope

**In.** `.claude/tools/_rust_callgraph.py` (NEW), `.claude/tools/err_consumer_census.py` (NEW),
`.claude/tools/err-consumer-census.tsv` (NEW), `.claude/tools/test_err_consumer_census.py` (NEW) (§4);
legibility rewrites in `crates/script/elidex-js/src/` where a row's propagation has no shape the check
follows and the code can be written in one it does (§7.4; no behaviour change, each named in the PR body);
the PR body's counts (§10).

**Out.** The contract's semantics and every consumer fix (P0a-1, P0b-VM-1, P0b-VM-2, P0b-L3); PR-1's
`recursion_cutcheck.py` (it builds on `_rust_callgraph.py`, PR-1 §5.2).

## §2. Coupled invariants

Elements: **CEN** (rules; §5), **REA** (checked reasons; §5 rule 12), **RES** (the resolved call graph;
§5 rule 0), **PEND** (`# pending` and the planned-producer list; §6, §7), and P0a-1's **ID** / **CP** /
**INJ** / **OUT** where the check reads them.

| Pair | Intersection | Pinned by | Basis |
|---|---|---|---|
| REA × CEN | a row is admissible only through a reason code with a predicate; an unknown code, a failed predicate and a missing predicate are each red | controls `census_rejects_unknown_reason`, `census_rejects_failed_reason`, `census_reason_without_predicate_is_red` | check |
| REA × RES | every reachability predicate (top-only, collision, drain-consumed, closure-literal) runs on the resolved graph (rule 0); an occurrence the resolver cannot resolve is an edge to every definition of that name (over-approximation stays fail-safe) | control `census_unresolved_call_is_every_definition` | check |
| RES × rule 10 | a collision cut is keyed by (caller definition, call occurrence), never by (name, name); a cut that names an occurrence whose callee is not a colliding name is red | controls `census_collision_cut_by_occurrence`, `census_collision_row_other_path` | check |
| REA × rule 5 | top-only is one predicate for every class that claims it (N, S, W, boundary), not an N-only check | control `census_top_only_reachable_from_native` (fixtures on an S row and a W row) | check |
| REA × rule 9 | std-trait impls are scanned like every foreign trait; a `Drop` / `Iterator::next` that reaches a producer is a member and needs a row | control `census_boundary_std_trait_member` | check |
| REA × rule 11 | kind-bearing is by containment (`Vec<EvalResult>` qualifies); a listed type whose definition does not name `ScriptFailure` is red | controls `census_boundary_kind_bearing_container`, `census_kind_bearing_type_without_failure` | check |
| REA × rule 8 | the loop rule's admissible reason is a predicate (no durable write and no durable-store drain before the `?`), not the spec class the prototype named by hand | control `census_loop_question_mark_member` (and its admissible fixture) | check |
| ID × CEN | producer derivation keys on calls to `resource_limit::enter` (P0a-1 §4.2; `check_entry` is private inside it) and on constructions of the two kinds | control `census_planned_producer_already_derived` | check |
| CP × CEN | the four enclosing functions of CP-1…CP-3b are the derived producers at P0a-2; carriers are closed over them | §6 | check |
| INJ × CEN | the boundary hooks are `#[cfg(test)]` statements; the lexer excludes `#[cfg(test)]`-attributed items **and statements**, so a hook does not make its function a derived producer | control `census_cfg_test_hook_not_producer` | check |
| OUT × CEN | rule 11 makes `ScriptFailure`-bearing types kind-bearing, so P0a-1's fixed boundaries leave the population; rule 7 keeps D-14's and D-15's writes W rows (admissible: reverted on `NotStarted`) | controls `census_kind_bearing_type_without_failure`, `census_w_across_boundary` | check |
| CEN × PEND | a D row is red outside `# pending`; a pending row outside the frozen ceiling is red, so the section only shrinks; a row that loses its reason under rule 12 is **not** a pending row (it is an instrument or legibility defect, fixed in this slice), so the ceiling is unchanged | controls `census_rejects_definitive_row`, `census_pending_row_outside_ceiling` | check |
| ID × PEND | D-13, D-14, D-15 are fixed in P0a-1, so they are admissible rows at P0a-2's landing, never pending | §7 | structural |
| CEN (rule 6) × B | at base, B-1 / B-2 are rule 6 members; after P0a-1 they are pure throw arms (auto-recognised), leaving only the "throw-routed" P rows; rule 6 guards catchability (P0a-1 §5.2) | §5 rule 6 | check |

## §3. Spec coverage map

No spec defines the census. The check implements no algorithm; the rows are the two clauses the admissible
classes rest on (the rest are P0 umbrella §3 and P0a-1 §3).

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| ECMA-262 §17 Error Handling and Language Extensions | paragraph 3 (no other kinds of errors as early errors) | no spec step yields the error, so every admission reason is a property of the code, checkable by the tool, never a spec branch | rule 12 (§5) | ✓ | no |
| WHATWG DOM §2.9 Dispatching events | *inner invoke* step 2.5 (once: remove before the call) | the canonical durable write the W population detects (rule 7); its admissible reason is "reverted on `NotStarted`" (P0a-1 §4.5) | rule 7, the W rows of §7 | ✓ | yes |

**Breadth**: K=2 specs (ECMA-262, WHATWG DOM), M=2 entries (verified 2026-11-08 by `python3
.claude/skills/elidex-plan-review/preflight.py` on this memo; rc and K / M as printed in the PR body, P0
umbrella §7 L-13). Below the split thresholds.

## §4. The check's files (all NEW)

- `.claude/tools/_rust_callgraph.py`: the shared lexer (comments, strings and raw strings stripped with
  line numbers kept; `#[cfg(test)] mod x { }` blocks, `#[cfg(test)] mod x;` files and other
  `#[cfg(test)]`-attributed items and statements excluded), function extraction with signature, `self`-ness,
  parameters and return type, **resolved** call edges (rule 0), carrier closure. PR-1's
  `recursion_cutcheck.py` builds on it (one call-graph substrate for both checks).
- `.claude/tools/err_consumer_census.py`: the census over P0 umbrella §5.1's populations, with rule 12's
  predicates.
- `.claude/tools/err-consumer-census.tsv`: the allowlist; one row per non-auto member: path, enclosing
  function, callee, shape, class, **reason code**, reason arguments (the channel function for `channel`, the
  cut occurrences for `collision`, the consuming rows for `drain-consumed`), free text. Keyed by (path,
  function, callee, shape, occurrence), not by line. A `# pending` section holds the D rows (§7).
- `.claude/tools/test_err_consumer_census.py`: tracked `test_*.py`, collected by `scripts/python-suites.sh`
  into the `tools` job; runs the check on the tree (rc 0) and the controls of §8.

## §5. Rules

0. **Resolution (NEW; the answer to R4 Axis 2 IMP-3).** A call occurrence is resolved to a set of
   definitions, not to a name: `recv.m(` → the methods named `m` (definitions with a `self` parameter);
   `path::f(` → the definitions of `f` in the module `path` names (a lower-case segment) or, for a type
   segment, its associated functions; bare `f(` → the definitions of `f` in the same file, else the free
   functions of that name. An occurrence that resolves to nothing known keeps an edge to **every**
   definition of the name (the over-approximation is kept on the fail-safe side; it costs rows, never
   silence). Edges are (caller definition → callee definition); every reachability predicate below runs on
   this graph. Rule 10's cut is an occurrence of this graph.
1. Populations per P0 umbrella §5.1 (Err, boundary, W), recomputed from the code on every run; test files
   and `#[cfg(test)]` code are excluded by the lexer (§4), not by a list. The crates scanned are derived:
   `elidex-js`, every workspace crate whose Cargo.toml depends on it (the shell), and every crate whose
   non-test source names a kind-bearing type (rule 11) or declares a trait that `elidex-js` implements.
2. Auto-recognised shapes are narrow: `?`; `.map_err(f)?` where `f` is `vm_error_to_thrown` or a `From` /
   `Into` conversion; `return` / tail in a carrier function; an arm (or an `if let Err(e) = ..`
   block) whose whole body is `self.throw_error(e, ..)?` or `return Err(e)` or `Err(e)`. Anything else is a member.
3. A member without a row is red; a row without a member is red (the allowlist cannot drift either way).
4. Admissible classes: P, T, R, S, E, N, X (P0 umbrella §5.2), **each only through a reason code of rule
   12**. A D row is red, except in the `# pending` section, whose rows name their fixing slice
   (`P0b-VM-1` / `P0b-VM-2` / `P0b-L3`) and lie inside the frozen ceiling (§7); PR-1's precondition is an
   empty section (P0 umbrella §7 L-1).
5. **Top-only (generalised, PM 544).** Any row of any class whose reason is or includes top-only (reason
   codes `top-only`, `thread-entry`; the `dropped-nothing-written` code includes it) is red if the reverse
   graph (rule 0) reaches it from a native function (signature `(ctx: &mut NativeContext<'_>, ..)`); edges
   into a function that is the body, or a callee of the body, of a `std::thread::Builder::spawn` /
   `std::thread::spawn` / `spawn_worker` closure are cut. At the base this check is what PM 533 (c) rejected
   for `call_listener` and what D-15's S row failed (P0a-1 §5.4).
6. Dispatch loop: in `run()`, a `?` on a carrier is a member, not propagation, unless the callee is a
   throw-routed helper listed as such (class P, reason `throw-routed`: the callee's own Err path passes the
   `throw-arm` predicate); a new one is red. At `ce603c74` the `?` uses on a carrier in `run()` are
   `dispatch.rs` 91 (B-1), 842 / 845 (B-2), and 674 / 677 / 885 / 941, whose callees route their own errors
   through `throw_error` (`dispatch_objects.rs:40`, `:72`; `dispatch_class.rs:47`, `:73`) (verified
   2026-11-08 by the prototype filtered to `run` and by reading each callee).
7. W population: every function of P0 umbrella §5.1's W population has a row (class D pending, or
   admissible with a reason of rule 12: `reverted-on-not-started`, or `top-only` with `no-durable-write`
   failing — see rule 12). The callee set is the carriers **and the boundary members** (rule 9), and the
   function may sit in any scanned crate (W across the boundary): `script_dispatch_event_core`'s write at
   `:499` before `call_listener` is how D-14 is seen; `dispatch_unhandled_rejection_event`'s at `:565`
   before `call` is how D-15 is seen.
8. **Loop rule (fail-safe; ratified PM 513).** A carrier call propagated by `?` (or `.map_err(..)?`, or a
   binding later propagated so) whose position lies inside a `for` / `while` / `while let` / `loop` body of
   the enclosing function is a **member**, not auto-propagation: the `?` is correct for the trigger but may
   leave the loop's remaining subjects unprocessed, the shape of P0 umbrella §5.1's C-2 (c) that rule 2
   would otherwise accept unseen. The rule does not try to tell "several subjects" from "elements of one
   operation"; every such `?` is a member, and **its one admissible reason is rule 12's
   `loop-no-write-no-drain`** (the prototype's hand classes *loop-spec-abrupt* / *loop-conversion* /
   *loop-graph* are commentary, not reasons). `run()` is excluded (rule 6 already makes its `?` uses members).
9. **Boundary population (PM 533 (2); fail-safe).** Members, each needing a row:
   (B1) every method of an `impl Trait for T` in `elidex-js` non-test code whose trait is not declared in
   `elidex-js` — **std / core / alloc traits included** (`Drop`, `Iterator`, `Default`, `Display`, …; R4
   Axis 1 MIN-2: the `next` of an `Iterator` impl that reaches a producer collapses the kind into `Option`, a `Drop` that
   runs script loses it entirely, so the property applies and no trait list is exempt) — when its body
   reaches a producer in the resolved graph closed over **every** function (not only carrier-typed ones: a
   boundary method returns `()`) and its return type is not kind-bearing (rule 11);
   (B2) every `pub` function of `elidex-js` with those two properties that a dependent crate calls by name;
   (B3) every call from `elidex-js` to a function of another crate that takes a closure or a function
   pointer (`dyn Fn*`, `impl Fn*`, `fn(..)`, a generic bounded by `Fn*`): the graph does not follow a closure
   across the call, so the call is a member whatever its body reaches. Fail-safe: an `impl` header the
   lexer cannot parse, a trait it cannot resolve, a return type that is a bare generic parameter, and a B3
   callee name it cannot resolve through the file's `use` items are members.
10. **Collision rows (rewritten; R4 Axis 2 IMP-3).** A row of class N whose reason code is `collision`
    names the colliding **call occurrences** (file:line, callee name) in the member's own body or along its
    witness path; the check cuts exactly those occurrences' edges in the resolved graph (rule 0) and
    recomputes reachability: the row is red if the member still reaches a producer, red if a named
    occurrence is not a call to a name with more than one definition (then it is not a collision), and
    stale (rule 3) when the member reaches no producer even without the cuts. A cut never removes an edge
    from a different caller definition or a different occurrence of the same name in the same body.
11. **Kind-bearing types (containment; R4 Axis 2 MIN-3).** A return type is kind-bearing when it is
    `Result<_, E>` with `E` a carrier error type (P0 umbrella §5.1), or when its outermost type or any of
    its generic arguments, recursively, is a type in the check's `KIND_BEARING` list (`ScriptFailure`,
    `EvalResult`, `FilterError` carrying `ScriptFailure` after P0a-1 §4.5): `Vec<EvalResult>` (`drain_timers`, BC-2) qualifies. A
    listed type whose definition does not name `ScriptFailure` is red, so the list cannot bless a type that
    dropped the kind.
12. **Checked reasons (NEW; PM 544).** Every admitted row names one reason code; each code has one
    predicate; a failed predicate, an unknown code and a code without a predicate are red (the row is D). The
    codes are closed (adding one is a change to this table, with its control and mutant):

| Class · code | Predicate (on the resolved graph and the lexed body) | Row arguments |
|---|---|---|
| P · `propagates` | the Err path of the use ends in a propagating sink: every `Err` arm (or `else` branch / `let … else` body) of the match on the call or on its binding ends in `return Err(..)` / `Err(..)` tail / `?` / `Some(Err(..))`; a method chain uses only kind-preserving combinators (`map`, `and_then`, `inspect_err`, kind-preserving `map_err`) and ends in a sink; a binding has a sinking use and no use that collapses it (`.ok()`, `unwrap_or*`, `map_or*`, `expect`, `unwrap`); the call is an arm value or `Some(..)` / `Ok(..)`-wrapped in tail / return position; or it is the argument of a `Result`-taking function whose own result sinks | — |
| P · `throw-routed` (rule 6) | the callee's Err path passes `throw-arm` | — |
| P · `pass-through` (B1) | the return type is the method's own generic parameter | — |
| P · `loop-no-write-no-drain` (rule 8) | no W marker (rule 7's kinds) and no durable-store drain (`take` / `drain` / `pop` / `pop_front` / `remove` on a field of the VM, `HostData` or the world — never on a local) in the body before the `?` | — |
| P · `reverted-on-not-started` (W rows) | the write precedes a boundary or carrier call whose `NotStarted` / `is_resource_limit()` arm calls the reinsert / restore named by the row before the next statement that can run script | the revert call |
| T · `throw-arm` | the Err path calls `throw_error(` or `vm_error_to_thrown(` and `handle_exception(`; or the function returns `VmError` and yields the error as a value for its caller's throw | — |
| R · `channel` | the Err path, or the function the binding is passed to (one level), calls the named channel function with the error or its materialisation; the channel set is the C-3 list (promise / request rejection, `settle_promise`, the report stand-in `eprintln!` / `report_*`, stream error, `EvalResult` construction); a channel not on the list is red | the channel function |
| R · `drain-consumed` (B1) | with every consuming row's edge cut ((function, callee) of the R / T / S / E / N / X rows and the `# pending` rows), the method reaches no producer; the row lists the consuming rows its paths hit (PM 540 (3): the report path) | the consuming rows |
| S · `dropped-nothing-written` | no W marker in the body before the call, no W row for the function in any scanned crate (rule 7), and `top-only` holds (rule 5) | — |
| E · `replaced-both-surfaced` | the use is `.err()` or `.is_err()`-tested and, after it, the original or the replacement leaves as `Err(..)` or as the value of a `-> VmError` function | — |
| N · `top-only` | rule 5 | — |
| N · `thread-entry` | the function is the body, or a callee of the body, of a `spawn*` closure (rule 5's cut) | — |
| N · `collision` | rule 10 | the cut occurrences |
| N · `closure-literal` (B3) | the argument is a closure literal whose body reaches no producer; a parameter or a non-literal argument is red unless the row names the enclosing function's own closure parameter and every call site of that function passes a literal that passes this predicate (transitive, one level) | — |
| X · `test-support` | the function has no production caller on the resolved graph (a file declared `#[cfg(test)] mod x;` is excluded by rule 1 and has no row) | — |
| *(no code)* `argument-class`, `unreachable-by-construction`, `reviewed` | **no predicate**: not admissible. The code is made legible instead (§7.4): call the non-carrier conversion; remove the unreachable arm (P0a-1 §4.5 BC-3); pass a closure literal | — |

The predicate texts above are the specification of the check; the check's docstring quotes this table and
`test_err_consumer_census.py` has one red control per code (§8).

## §6. Producers and the planned list

Producers are derived: functions that call `resource_limit::enter` (the wrapper of the private `check_entry`,
P0a-1 §4.2), `ensure_stack` (NEW: PR-1), or construct
`VmError::resource_limit` / `CompileErrorKind::ResourceLimit`. Until PR-1 / PR-1b land, the check also
reads `PLANNED_PRODUCERS`, a constant listing PR-1 §5.2's walker functions, the compile front end's entry
(`compile_script`), the regexp compile sites of PR-1 §6.1 (`load_constant`, `native_string_match`,
`native_string_search`) and PR-1b's `call_wasm_exported_function` / `instantiate_module`. A planned name
that is also derived is red ("remove it"), so the list only shrinks and is empty after PR-1b. At P0a-2's
landing `call_wasm_exported_function` and `instantiate_module` are already derived (they call
`resource_limit::enter`, P0a-1 §4.2–§4.3), so they are not in the list P0a-2 ships.

## §7. The allowlist at P0a-2's landing

### §7.1 The census at the base (unchanged numbers)

Prototype at `ce603c74` (verified 2026-11-08 via
`python3 -I H/scripts/err_consumer_census_proto.py <wt> crates/script/elidex-js`, log
`H/logs/err-census-p0.log`): 4307 function bodies, 2236 carriers by type, 1042 carriers that reach a
producer; 1575 uses propagate by `?` and 188 by return / tail, auto-recognised; 196 non-propagating uses in
`elidex-js` and 6 in the shell (`H/logs/err-census-p0-shell.log`, seeds `eval drain_timers eval_script`),
202 rows, each hand-classified (`H/scripts/err_census_classify.py`, output
`H/logs/err-census-p0-classified.tsv`: P 80, T 44, R 32, S 4, E 10, N 8 + 6 shell, X 4, D 14 rows at 13
sites). **Rule 8 adds 120 members** (verified 2026-11-08 via `LOOPQ=1 python3 -I
H/scripts/err_loop_question_proto.py <wt> crates/script/elidex-js`, log
`H/logs/err-loop-question-p0a.log`: 99 `for`, 9 `while let`, 5 `loop`, 4 `for` inside `while`, 3 `while`),
hand-classified P (`H/logs/err-loop-question-p0a-classified.tsv`). Total at the base: 322 Err rows. W
population: 5 functions (`python3 -I H/scripts/err_write_before_call_proto.py <wt>
crates/script/elidex-js`, log `H/logs/err-write-before-call-p0.log`).

**Boundary population (rule 9; PM 533 (2)).** Prototype at `02386ec9` (code identical to `ce603c74`),
`python3 -I H/scripts/err_boundary_census_proto.py <wt>`, log `H/logs/err-boundary-census-p0.log`, name
graph (pre-rule-0): the producer closure over every function reaches 3244 of 4307 names. Hand-classified:

| Section | Scanned | Members | By class |
|---|---|---|---|
| B1 foreign-trait methods (`ScriptEngine` 7, `HostDriver` 53, `FilterAction` 1, `MutationDispatcher` 1; std traits skipped by the prototype) | 62 | 27 | **D 1** (D-14, `call_listener`); **boundary collapse 3** (BC-1 `eval`, BC-2 `drain_timers`, BC-3 `FilterAction::accept`), all fixed in P0a-1; R 13 (12 drains + `set_history_state`, P0a-1 §4.5); N 9 (collision); P 1 (`with_bound`) |
| B2 `pub` `elidex-js` fns called by the shell (the only dependent crate) | 8 calls | 2 new | 6 are the shell's calls of B1 `HostDriver` methods by name (no new row); N 2: `finish` (collision with `finish` in `compiler/function.rs`, `app/render.rs:296`), `sw_thread_main` (`app/sw_coordinator.rs:214`, a fresh thread's top, rule 5's thread-entry cut) |
| B3 closure / fn-pointer parameters called from `elidex-js` | 21 calls | 21 | N 21: Rust-only closures (`with_range` 7, `with_range_mut` 3, `reconcile_event_handler`'s scripting gate 1, `for_each_shadow_inclusive_descendant` 1 in `custom_elements/lookup.rs:159`, `ws_keepalive` / `es_keepalive` 2, the WebCrypto RNG of `sign` / `generate_key` 2), and 5 name collisions with `sign` (the BigInt `.sign()` in `coerce.rs:23`, `coerce_ops.rs` 58 / 64 / 70, `pools.rs:142`); `range_proto.rs:361` passes a parameter, not a literal |
| W across the boundary (rule 7) | 80 calls, from another crate, of a B1 / B2 member's name (8 of them are `finish` collisions) | 3 with a write before the call | **D 1** (`script_dispatch_event_core` → `call_listener`, the write at `:499`: D-14); P 1 (the same write before `run_microtasks` at `:513`); N 1 (`layout_inline_context_fragmented` → `finish`, the B2 collision) |

### §7.2 The checked-reason re-run (PM 544; prototype `H/scripts/err_reason_check_proto.py` at `d7ec4761`)

The prototype gives every row of §7.1 its rule-12 code and runs the predicate; logs
`H/logs/err-reason-check-p0a2.log` (std traits skipped, as §7.1's prototype did) and
`H/logs/err-reason-check-p0a2-nostdskip.log` (rule 9 as written). Verdicts: **OK** (predicate holds),
**RED** (the reason is false on the code), **UNFOLLOWED** (the predicate exists but the prototype's analyser
cannot follow this shape), **NOCHECK** (no predicate). Under the rule, RED, UNFOLLOWED and NOCHECK rows are
D. The resolved graph (rule 0, four crates scanned, 5660 definitions): 4525 definitions still reach a
producer — resolution by `self`-ness, module and file removes free-function collisions but not std-method
names (`len`, `expect`, `extend`, `new`, `parse`, `default`, `to_string`), which is why rule 10 keys the cut
by occurrence and why the collision rows below are red until their occurrences are named.

| Population | Rows | OK | RED | UNFOLLOWED | NOCHECK | Pending D |
|---|---|---|---|---|---|---|
| Err (202) | 202 | 148 (E 10, N 6, shell 6, P 56, R 22, S 3, T 43, X 2) | 2 (S `natives_promise.rs:591` = **D-15**; T `dispatch.rs:430`, the analyser's `let x = match` gap) | 37 (P 24, R 10, N 1, X 2 — the X rows are in a `#[cfg(test)] mod differential;` file, so they leave the population) | 1 (N `ops_element.rs:134`, `argument-class`) | 14 |
| rule 8 (120) | 120 | 118 | 2 (`natives_json/stringify.rs:192`, `:204`: the marker `self.stack.pop(` is the serializer's cycle stack, a field, so the prototype's receiver-syntax predicate cannot tell it from a durable store) | 0 | 0 | 0 |
| B1 (62; std skipped) | 62 | 31 not members on the resolved graph; 4 BC / D-14 (kind-bearing after P0a-1); 13 `drain-consumed` (12 drains + `set_history_state`, each with its consuming rows listed in the log); 1 `pass-through` | 13 `collision` (the 9 of §7.1 plus `remove_listener`, `sync_dirty_canvases`, `has_pending_session_history_work`, `MutationDispatcher::dispatch`, which the wider scan makes reach a producer through std-method names): no occurrences named yet | 0 | 0 | 0 |
| B1 std traits (rule 9 as written, `-nostdskip`) | +45 | +31 not members | +14 `collision` (`Drop::drop` ×4: `engine.rs:427`, `:800`, `temp_root.rs:101`, `:204`; `next` of two `Iterator` impls: `typed_array_hof.rs:85`, `value.rs:250`; `Default::default` ×7; `Display::fmt` 1 — none runs script, R4 Axis 1 MIN-2) | 0 | 0 | 0 |
| B3 (12 detected by this prototype; §7.1's detector counted 21, §11 item 2) | 12 | 10 `closure-literal` | 2 (`range_proto.rs:361` passes a parameter; `dom_bridge.rs:492` passes a non-literal) | 0 | 0 | 0 |
| W across | 3 | 2 (`call_listener`: `reverted-on-not-started` after P0a-1; `run_microtasks`: step 2.5 of a started call or already reverted); the `finish` N row is stale on the resolved graph (the resolved `finish` is not the VM's) | 0 | 0 | 0 | 0 |

**Rows that lose their reason, and who fixes them.**

- **One consumer defect**: D-15 (`natives_promise.rs:565` / `:591`), the S row's "nothing written" is false
  (a listener removal precedes the call) and its "top-only" is false (rule 5 generalised finds the
  `dispatchEvent` path, P0a-1 §3.1). Fixed in **P0a-1** (§5.4), like D-14; never pending.
- **Instrument debt (this slice)**: 37 UNFOLLOWED + 1 RED (T) + 2 RED (rule 8) = 40 Err rows, 13 (+14 std)
  B1 collision rows, 2 B3 rows. None is a consumer defect; each is a shape the tracked check must follow,
  a cut the row must name, or a predicate that must be typed. P0a-2's acceptance is **rc 0 with every row
  verified**: the check follows the shapes the prototype did not (`let x = match …` with a throw arm; `if let
  Some(r) = f() { return r }` fall-through on `Option<Result>`; an arm value of a bound match; a closure
  argument to an iteration helper (`validate_pair_entry` ×2); a binding passed to a function whose body is
  the channel (`finish` ×9, `settle` inside `make_async_coroutine_and_drive`); a `.map(closure)` chain), the
  durable-store set of `loop-no-write-no-drain` is defined by type (fields of `VmInner`, `HostData`, the
  world) not by receiver syntax, and each collision row names its occurrences. Where a shape resists, the
  code is rewritten into one the check recognises (§7.4).
- **Legibility rewrites (this slice, no behaviour change)**: `ops_element.rs:134`
  (`classify_typed_array_string_key`: call the String → Number conversion directly, not the generic
  `to_number`; the row leaves the population); `range_proto.rs:361` and `dom_bridge.rs:492` (pass the closure
  literal at the call, or name the enclosing function's closure parameter per rule 12's transitive clause);
  the BC-3 null arms are already removed by P0a-1.
- **`# pending`: unchanged — 15 keys** (§7.3). No row of this re-run enters it: D-14 and D-15 are fixed in
  P0a-1, and instrument debt is not a consumer defect. The ceiling is the same frozen key set.

**PM 540 (3), the drains.** Each of the 12 drains and `set_history_state` names its consuming rows (the
`drain-consumed` arguments; full lists per method in `H/logs/err-reason-check-p0a2.log`). First-hop
consumers: the microtask-checkpoint drains (`run_microtasks`, `tick_network`, `deliver_idb_versionchange`,
`deliver_sw_client_update`, `deliver_history_step_events`, `deliver_media_query_changes`,
`deliver_visual_viewport_events`) → `run_callback` → `call` (R), `run_reaction` → `call` (R),
`dispatch_unhandled_rejection_event` → `call` (R after P0a-1, D-15), `flush_ce_reactions` → `invoke_one`
(S; §9 home); `drain_reactions` → `flush_ce_reactions` → `invoke_one` (S), `invoke_callback` → `call` (R);
`deliver_mutation_records` / `deliver_resize_observations` / `deliver_intersection_observations` →
`deliver_to_observer_callbacks` → `call` (R); `deliver_sw_client_update` → also `fire_message` →
`parse_json_str` (D-9, pending → P0b-VM-2); `drain_worker_messages` → `dispatch_message_event_at` →
`parse_json_str` (N); `deliver_history_step_events` → also `reconstruct_history_state` →
`structured_deserialize` (D-7, pending → P0b-VM-1's N row); `set_history_state` → `structured_deserialize`
→ `parse_json_str` (D-7, pending → P0b-VM-1's N row `seed_history_state` → `structured_deserialize`). A
drain whose paths hit only `# pending` rows is admissible now and stays so when the pending row becomes
its slice's admissible row; a drain that reaches a producer through no row is red.

### §7.3 Composition at landing

Boundary side (after P0a-1): BC-1…BC-3 and `call_listener` are no longer B1 members (their return types are
kind-bearing, rule 11); the W rows of `script_dispatch_event_core` (`:499`) and
`dispatch_unhandled_rejection_event` (`:565`) are admissible P `reverted-on-not-started`. Err side, the
deltas P0a-1 causes: `engine.rs:275` is R `channel:report`; `natives_promise.rs:591` is R `channel:report`
(its revert is the W row's); `script_dispatch_event_core`'s use of `call_listener`'s result is R
(reported by the engine, reverted here); the two traversal protos' uses of the `FilterError::Script` (NEW: P0a-1) arm are
auto-recognised or P `propagates`; D-13's row (`vm/interpreter.rs` `eval` → `compile_script`) is P
`propagates` (kind-preserving `map_err`). The exact counts are the tracked check's at P0a-2's head (§10),
not these.

Admissible rows at the base (paths under `crates/script/elidex-js/src/` unless stated; lines at `ce603c74`;
each with its rule-12 code at landing):

- **R (32):** `engine.rs` 191 / 344 (`channel:EvalResult`); `vm/host/body_mixin.rs:289`;
  `vm/host/cache/natives.rs` 96 / 121 / 145 / 198 / 252 / 287 / 340 / 382 / 420 (`channel:finish`, the
  binding passed to `finish`, which rejects); `vm/host/custom_elements/creation.rs:238`, `flush.rs` 104 /
  183 / 202 / 234, `lookup.rs:183` (`channel:report`); `vm/host/fetch/mod.rs` 183 / 204;
  `vm/host/observer_common.rs:295` (`channel:report`); `vm/host/readable_stream/controller.rs:135`,
  `mod.rs` 410 / 639 (stream error / cancel rejection, C-4 non-re-entry); `vm/host/service_worker/container.rs`
  109 / 218; `vm/host/wasm/namespace.rs:578`; `vm/natives_generator.rs:472` (wrapper rejection; residual P0
  umbrella §6 item 5); `vm/natives_promise.rs` 648 / 689 / 747; `vm/natives_timer.rs:370`. After P0a-1:
  `engine.rs:275` and `vm/natives_promise.rs:591` (`channel:report`), and `vm/interpreter.rs:894`
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
  (§7.2); `vm/interpreter.rs:894` moves to R.
- **X (2):** `engine.rs` 63 / 78 (embedder-test helpers, `test-support`). `vm/host/subtle_crypto/differential.rs`
  32 / 36 are in a `#[cfg(test)] mod differential;` file (`subtle_crypto/mod.rs:62-63`), excluded by rule 1:
  not members.
- **P (80 + 120 by rule 8) and T (44):** listed in `H/logs/err-census-p0-classified.tsv` and
  `H/logs/err-loop-question-p0a-classified.tsv`; T = `vm/dispatch.rs` (40), `dispatch_objects.rs` 48 / 80,
  `dispatch_class.rs:110`, `dispatch_helpers.rs:383` (`throw-arm`; `:383` by the `-> VmError` clause).
- **W, admissible (5 after P0a-1):** `custom_elements/creation.rs:238` (the `is_value` reset is derived
  from the registry, not from the upgrade's outcome: it precedes the call and is not undone by its failure
  — admissible only if rule 12's `reverted-on-not-started` or `top-only` + `no-durable-write` holds; the
  prototype did not decide it, §11 item 3); `html_input_value.rs:353` (the carrier is `to_number` on a
  Number: the legibility rewrite of §7.2 applies, the row leaves); `natives_promise.rs:565`
  (`reverted-on-not-started`, P0a-1 §5.4); across the boundary `script_dispatch_event_core` `:499`
  (`reverted-on-not-started`) and `:513` (step 2.5 of a started call or already reverted).

- **`AbortSignal` abort steps (PM, 11-08; L3's `#11-abort-signal-any-dependent-flattening`; corrected per
  L3's attestation N-6, verified against the code).** *At `ce603c74`:* `abort_signal`
  (`vm/host/abort.rs:522-658`) has two `?`: on `dispatch_vm_simple_event` at `:592`, and on its own
  recursion over the composites at `:610`, inside the `for` loop. Neither can fail today:
  `dispatch_vm_simple_event` → `fire_vm_event` → `fire_vm_event_unchecked` → `dispatch_vm_event`
  (`event_target_dispatch_vm.rs:271-347`, no `?` in its body), whose only return is `Ok` (listener exceptions
  are absorbed inside dispatch), and those two `?` are `abort_signal`'s only error sources, so `:610` can
  fail only if `:592` can. The graph agrees: `abort_signal` reaches no producer, so it is not in the
  population and rule 8 does not fire on `:610`. The shape is **latent definitive**: once P0's error can
  surface through this dispatch (a check or probe on the dispatch path, e.g. a PR-1 walker probe in event
  construction), an `Err` at `:592` would return after the latch and before the composite and fetch
  fan-outs, and one at `:610` after `any_composite_map.remove` (`:607`), so the remaining dependents would
  never abort (C-2 (c)). No input reaches it today. *After L3's PR:* DOM §3.2 *signal abort* (`dfn dom
  'signal abort'`, for AbortSignal) step 4 fixes every dependent's reason and steps 5–6 run for all of
  them; the loop runs `run_abort_steps` for every entry, keeps the first `Err`, and returns it after the
  loop, with no `?` inside the loop (L3's contract, unchanged). If the call is then in the population, it is
  a member (its result is stored, not propagated in place) of class **P**, code `propagates` (the stored
  first `Err` is returned after the loop: the binding's sinking use), reason "deferred propagation: the
  first `Err` is returned after every dependent's abort steps ran (DOM *signal abort* steps 4–6); later
  `Err`s of the same trigger are not surfaced separately, one surfacing per trigger suffices (C-3)". Rule 8
  stays the fail-safe for the class: a `?` put back inside that loop is a member the moment the call joins
  the population. **Which PR writes the row:** L3's PR is expected first (edge, P0 umbrella §1); P0a-1 is
  rebased on it (the seam, P0a-1 §4.4) and P0a-2's allowlist carries the post-L3 row if the tracked check
  lists the call, keyed on the names as they stand at P0a-2's base (§11 item 1). If L3's PR lands after
  P0a-2, P0a-2 ships no row and L3's PR adds it if the check asks for it.

**`# pending` (15 rows; this key set is the frozen ceiling of §5 rule 4; restated after PM 544: unchanged,
because D-14 and D-15 are fixed in P0a-1 like D-13, and instrument debt is not pending).** Keys (path,
function, callee, shape):

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
admissible row its memo names **with its rule-12 code**; no slice adds a pending row (the ceiling). A P0b
memo's row whose reason has no code of rule 12 is a design change to this memo (the code table is closed).

### §7.4 Legibility rewrites

When the check cannot follow a row's propagation and the predicate is right, the code is written in a shape
the check follows (one issue, one way: the allowlist does not grow prose to cover analyser gaps). Each
rewrite is behaviour-preserving, named in the PR body with its row, and reviewed as part of this PR. The
three the re-run names are §7.2's; others are decided at implementation when the tracked check runs on the
tree (§11 item 4).

## §8. Controls and mutants

Controls (tempfile fixtures), each red: `census_rejects_unknown_member` (an added `.ok()` on a carrier
call); `census_rejects_stale_row`; `census_rejects_definitive_row` (a D row outside `# pending`);
`census_pending_row_outside_ceiling`; `census_planned_producer_already_derived`;
`census_top_only_reachable_from_native` (fixtures: an N row, an S row and a W row claiming top-only, each
reached from a native); `census_run_question_mark_on_carrier` (a new `?` on a carrier in `run()`);
`census_w_function_without_row`; `census_cfg_test_hook_not_producer` (a fixture whose only producer edge is
a `#[cfg(test)]` statement must not derive a producer; red when it does); `census_loop_question_mark_member`
(a fixture with `for x in xs { carrier(x)?; }` and no row → red; the same `?` outside the loop → auto; with a
`loop-no-write-no-drain` row and a `self.queue.pop()` before the `?` → red); for the boundary (rules 7,
9–11): `census_boundary_unit_return_member` (a fixture `impl ForeignTrait for T` whose method returns `()`
and calls a producer, no row → red); `census_boundary_std_trait_member` (the same for `impl Iterator for T`
whose `next` calls a producer → red; a `Drop` likewise); `census_boundary_kind_bearing_auto` (the same method
returning `Result<(), ScriptFailure>` → not a member); `census_boundary_kind_bearing_container`
(`Vec<ScriptFailureBearing>` → not a member; `Vec<u8>` → member); `census_boundary_unparsed_impl_member` (an
`impl` header the lexer cannot parse → member); `census_boundary_closure_param_member` (a call passing a
closure to another crate's `impl FnMut` parameter, no row → red; with a `closure-literal` row and a closure
literal that calls a producer → red; with a non-literal argument → red); `census_w_across_boundary` (a
fixture in another crate that writes `get::<&mut EventListeners>` and then calls a boundary member, no row
→ red); `census_collision_row_other_path` (a `collision` row whose member still reaches a producer once the
named occurrences are cut → red); `census_collision_cut_by_occurrence` (a `collision` row naming an
occurrence of a name with one definition → red; a cut that would remove another caller's edge of the same
name must not make a red member green); `census_unresolved_call_is_every_definition` (an occurrence the
resolver cannot place keeps an edge to every definition: a fixture where the only producer path is through
such a call stays red); `census_kind_bearing_type_without_failure` (a `KIND_BEARING` entry whose definition
does not name `ScriptFailure` → red); for rule 12: `census_rejects_unknown_reason` (a row with a code not in
the table → red), `census_rejects_failed_reason` (one fixture per code whose predicate fails: a `propagates`
row whose Err arm is `_ => None`; a `throw-arm` row with no throw; a `channel` row naming a function the Err
path does not call, and one naming a function outside the channel list; a `dropped-nothing-written` row
with a W marker before the call; a `replaced-both-surfaced` row with no `Err` after the use; a
`drain-consumed` row with a producer path through no row; a `closure-literal` row with a parameter
argument; a `test-support` row with a production caller), `census_reason_without_predicate_is_red` (a row
whose code is `argument-class` → red). `--mutants` runs the check with each rule and each predicate
disabled and asserts the matching control turns green, so a rule that stops working is seen (the
plan-memo-umbrella-check precedent).

## §9. Stated limits and their home

The check does not see: calls through function pointers, trait objects and closures passed across
functions **inside one crate** (the `NativeFn` table; `fire_vm_message_event`'s closure argument is seen
only because its body is in the caller); across crates, foreign-trait impls and closure parameters are the
boundary population (rule 9, PM 533 (2)), and a trait object of a foreign trait is covered through its
impls; receiver types (rule 0 resolves by `self`-ness, module and file, not by the receiver's type, so a
std-method name shared by several types stays a collision until a row cuts its occurrences); writes
performed inside a callee before the carrier call (D-1's `take_uncompiled` sat in
`reconcile_event_handler`, another crate); macros; a straight-line `?` that returns before a fan-out over
other subjects (the `:592` shape of §7.3; rule 8 covers a `?` inside the fan-out's loop, not one before
it); durable writes to a side table (the `abort_signal_states` latch) rather than an ECS component, which
the W population's three kinds do not include. Owner VM P4; re-evaluated with PR-1's cut-check limits when
`#11-vm-native-callback-reentry-single-loop` is designed; ledger row P0 umbrella §7 L-7.

## §10. Landing obligations

What P0a-2's PR (or, where marked, the PM / PMO at landing) does. This memo edits no ledger. Program-level
obligations (PR-1 precondition, breadth record, slots) are P0 umbrella §7.

The ledger and notice rows this slice triggers are rows of the P0 program's landing list (P0 umbrella §7,
PM 513 (d)): L-4 (E rows) and L-7 (census limits). Slice-local:

- **L-a2. Counts and ceiling**: P0a-2's PR body records the tracked check's counts per class and per
  rule-12 code at its head (rule 8's members, the boundary population by section, and the std-trait members
  separately), the legibility rewrites of §7.4 with their rows, and the 15 `# pending` keys of §7.3.

## §11. Unverified facts (each decided at implementation, before its control is written)

1. The post-L3 function and callee names of the `AbortSignal` abort loop (§7.3), read at P0a-2's base
   once L3's `#11-abort-signal-any-dependent-flattening` PR has landed, and whether the tracked check lists
   the call.
2. The B3 population: `err_boundary_census_proto.py` counted 21 calls, `err_reason_check_proto.py` 12
   (the two detectors differ on which other-crate functions take a closure); the tracked check's rule-9 B3
   derivation decides, and the PR body records the count (L-a2).
3. `custom_elements/creation.rs:238`'s W row: whether its write (`is_value = None` before `invoke_upgrade`)
   is admissible under `reverted-on-not-started` (P0b-L3's `abandon_upgrade` path) or under `top-only` with
   no durable write of the subject; decided with P0b-L3's hand-off (`H/p0b-l3-handoff.md` §4 D-3 / D-4).
4. Which rows beyond §7.2's three need a legibility rewrite once the tracked check runs on the tree (§7.4).
5. Whether `dispatch.rs:430`'s `let old = match …` throw arm and the other UNFOLLOWED shapes of §7.2 are
   followed by the check's statement analysis or are rewritten (§7.4).
