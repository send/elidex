# VM stack-safety guard — P0a-2a plan (the census checker: rules, resolved call graph, rule 13, controls, the measured population report)

Status: **plan (pre-implementation)**, per-PR slice of the [P0 umbrella](2026-11-vm-stack-safety-guard-p0.md)
(PM 509; re-sliced from P0a by PM 544; P0a-2 split into P0a-2a and [P0a-2b](2026-11-vm-stack-safety-guard-p0a2b.md)
by PM 609). `/elidex-plan-review` required before code. Owner: VM P4. Base:
`ce603c74`; every code anchor below is at that commit (the branch head `d7ec4761` has identical code).
Provenance root `H` and the census prototypes are as in the P0 umbrella header; the prototypes are
`H/scripts/err_consumer_census_proto.py` (+ `err_census_classify.py`, `err_write_before_call_proto.py`,
`err_boundary_census_proto.py`, `err_loop_question_proto.py`) and, for this revision,
`H/scripts/err_reason_check_proto.py` (the checked-reason re-run, PM 544); their logs are under `H/logs/`
(`err-census-p0.log`, `err-census-p0-shell.log`, `err-census-p0-classified.tsv`,
`err-write-before-call-p0.log`, `err-boundary-census-p0.log`, `err-loop-question-p0a.log`,
`err-loop-question-p0a-classified.tsv`, `err-reason-check-p0a2.log`, `err-reason-check-p0a2-nostdskip.log`,
and, for PM 603, `err-census-p0-allfn.log`, `err-census-p0-newpop.tsv`, `err-census-p0-newpop-consumed.log`
from `H/scripts/err_consumer_census_allfn_proto.py` and `err_census_consumed_beneath_proto.py`).
The prototypes are not the deliverable: P0a-2a ships the tracked check of §4–§5, and every number below is
re-derived by that check at P0a-2a's head (§7, §10).

The contract C-1…C-5 is the P0 umbrella §4; the census definition (populations, classes) is the P0
umbrella §5; the semantics the check tests against (the one check site, `ScriptFailure`, the
failure-carrying boundaries) are [P0a-1](2026-11-vm-stack-safety-guard-p0a1.md) §4. This memo does not restate them.
**Option β (PM 590)** retired the W population and its reason code `reverted-on-not-started`,
renamed S's code to `dropped-top-only`, added the codes `surfaced-across-boundary` and the `channel`
sink of D-16, and took D-3 / D-4 and the two W rows out of `# pending` (P0a-2b §7.3 "D-1…D-12 under β").
PM 595 deleted rule 7 (the W markers) with it; the number stays retired (§5). PM 598 deleted rule 8
(`loop-no-write-no-drain`) the same way and replaced it by property: RL-only propagation is rule 13, and
report-and-continue loops go to the slot `#11-report-and-continue-loop-census` (§5 "(α)", §9; PM 608). PM 603 redefined the Err population's
reach: a producer reached through **any** function makes a carrier's use a member (rule 1), so the
population is "can receive a `ResourceLimit` once PR-1's probes are in", not "returns `Err` today" (§7.1).
Edges, schedule and the other slices: P0 umbrella §1.

**The split (PM 609).** PM 608 fixed the order substrate → classification → ratification of the allowlist.
The split maps that order onto slices: **P0a-2a** (this memo) ships the substrate, the rules, the controls,
the mutants and a measured population report; the **classification** of the population runs after P0a-2a
lands, with P0a-2a's checker; **P0a-2b** ships the allowlist and `# pending` built from that
classification, and its review waits for it (P0a-2b §6). The split takes the data-dependent part out of the
substrate's review, and it is the edge-dense multi-PR rule applied (CLAUDE.md "Edge-dense work"): each part
has its own `/elidex-plan-review`. P0a-2a is also the classification's tool, so nothing is built twice.

**The W retirement (PM 595).** Two statements, kept apart. (1) *The violation W detected does not exist
under β.* Under β a resource limit is the same `Err` as an ordinary throw, a throw completion of the step
(P0 umbrella §4 C-4). A durable write made before the call is the spec's own order (DOM *inner invoke*
step 2.5 removes a `once` listener before step 2.13 calls it) and stands on every outcome, so "a write
before a call that can fail" is no longer a defect the census can find. (2) *The need for cleanup goes
through the same path as an ordinary `Err`.* A write whose cleanup the call needs (a flag set before the
call and cleared after it) is implementation bookkeeping, and an ordinary exception takes the same `Err`
path through it. Whether that path cleans up is the code's general error handling, not a property of the
resource limit, so no detector specific to the resource limit is needed. The Err population (rule 2) and
the boundary population (rule 9) still see every use of the error itself.

## §0. Charter

P0a-2a ships the census check (`.claude/tools/err_consumer_census.py`), its shared call-graph
substrate, the controls and the mutants, and the measured population report (§7). The allowlist and its
`# pending` section are P0a-2b's. **The rule of the check (PM 544):** every admission reason is checked
mechanically. A row carries a reason code; the check
has one predicate per code; a row whose predicate fails, and a row whose code has no predicate, is a **D
row** (red outside `# pending`). The unknown falls to the checked side (fail-safe direction,
`memory/feedback_fail-safe-direction-before-list-completeness.md`); a reason the check cannot see is not an
admission (`memory/feedback_checks-must-not-be-defined-by-the-symptom-vocabulary.md`).

**Two modes, one check.** `--report` prints every member of the populations (rule 1, rule 9, rule 13)
with the candidate reason code each predicate admits, or none, and no allowlist is read: this is the
population report and the classification's instrument. The gate mode compares the members with the
allowlist (rules 3, 4) and is the tracked tree run; it is switched on by P0a-2b, which ships the allowlist.
At P0a-2a's head the tracked test runs the controls and mutants (§8) and `--report` on the tree (rc 0
unless the check itself fails); it asserts no allowlist on the tree.

**Why (PM 544).** R4 Axis 2 IMP-2: `natives_promise.rs:591` was admitted as S "nothing written, top-only"
by hand, and both claims were false on the code (D-15, P0a-1 §5.4); rule 5 checked top-only only on N rows.
R4 Axis 2 IMP-3: rule 10 cut collision edges name × name, which can turn an unknown into trusted. R4 Axis 1
MIN-2: rule 9 skipped std traits by an unstated list. Each is the same defect: a reason that lived in prose.

## §1. Scope

**In.** `.claude/tools/_rust_callgraph.py` (NEW), `.claude/tools/err_consumer_census.py` (NEW),
`.claude/tools/test_err_consumer_census.py` (NEW) (§4); the population report in the PR body (§10).

**Out.** The allowlist `.claude/tools/err-consumer-census.tsv`, `# pending`, the legibility rewrites and
the tree gate (P0a-2b); the classification of the population (between the two slices, P0a-2b §6); the
contract's semantics and every consumer fix (P0a-1, P0b-VM-1, P0b-VM-2, P0b-L3); PR-1's
`recursion_cutcheck.py` (it builds on `_rust_callgraph.py`, PR-1 §5.2).

## §2. Coupled invariants

Elements: **CEN** (rules; §5), **REA** (checked reasons; §5 rule 12), **RES** (the resolved call graph;
§5 rule 0), **PEND** (the rules over `# pending`, rules 3–4, and the planned-producer list, §6; the
section's content is P0a-2b's), and P0a-1's **ID** / **CP** / **INJ** / **OUT** where the check reads them.
The pairs over the allowlist's content (ID × PEND, CLS × PEND) are P0a-2b §2.

| Pair | Intersection | Pinned by | Basis |
|---|---|---|---|
| REA × CEN | a row is admissible only through a reason code with a predicate; an unknown code, a failed predicate and a missing predicate are each red | controls `census_rejects_unknown_reason`, `census_rejects_failed_reason`, `census_reason_without_predicate_is_red` | check |
| REA × RES | every reachability predicate (top-only, collision, drain-consumed, closure-literal) runs on the resolved graph (rule 0); an occurrence the resolver cannot resolve is an edge to every definition of that name (over-approximation stays fail-safe) | control `census_unresolved_call_is_every_definition` | check |
| RES × rule 10 | a collision cut is keyed by (caller definition, call occurrence), never by (name, name); a cut that names an occurrence whose callee is not a colliding name is red | controls `census_collision_cut_by_occurrence`, `census_collision_row_other_path` | check |
| REA × rule 5 | top-only is one predicate for every class that claims it (N, S, boundary), not an N-only check | control `census_top_only_reachable_from_native` (fixtures on an N row and an S row) | check |
| REA × rule 9 | std-trait impls are scanned like every foreign trait; a `Drop` / `Iterator::next` that reaches a producer is a member and needs a row | control `census_boundary_std_trait_member` | check |
| REA × rule 11 | kind-bearing is by containment (`Vec<EvalResult>` qualifies); a listed type whose definition does not name `ScriptFailure` is red | controls `census_boundary_kind_bearing_container`, `census_kind_bearing_type_without_failure` | check |
| RES × rule 1 (reach) | a carrier's use is a member when a producer (derived or planned, §6) runs beneath the call through **any** function, not only through carrier-typed ones; an occurrence the resolver cannot place reaches every definition (rule 0), so its call falls in; the five `dispatch_vm_simple_event` drops, which the carrier-typed closure missed, are the positive control (PM 603) | controls `census_reach_through_non_carrier`, `census_reach_dispatch_drops`; mutant M-REACH | check |
| REA × rule 13 | a propagating use of an RL-only carrier is a member; a carrier is RL-only unless an ordinary witness exists whose every hop is uniquely resolved, so the unknown falls to the member side; admissible only through `drain-consumed` or `rl-only-pinned` | controls `census_rl_only_propagation_member`, `census_rejects_failed_reason` (the `rl-only-pinned` fixture) | check |
| ID × CEN | producer derivation keys on calls to `resource_limit::check_entry` (P0a-1 §4.2, the one check site for the four entries) and `ensure_stack`, and on **every mention of the two kinds that the check cannot classify as a predicate** (a struct literal included); the unknown falls to the producer side (§6) | controls `census_planned_producer_already_derived`, `census_resource_limit_literal_is_producer` | check |
| CP × CEN | the four enclosing functions of CP-1…CP-3b are the derived producers at P0a-2a; carriers are closed over them | §6 | check |
| INJ × CEN | the boundary hooks are `#[cfg(test)]` statements; the lexer excludes `#[cfg(test)]`-attributed items **and statements**, so a hook does not make its function a derived producer | control `census_cfg_test_hook_not_producer` | check |
| OUT × CEN | rule 11 makes `ScriptFailure`-bearing types failure-carrying, so P0a-1's fixed boundaries leave the population; an engine-independent caller's use of a boundary's `Err(ScriptFailure)` is admissible only as `surfaced-across-boundary`, whose predicate checks the engine-side row | controls `census_kind_bearing_type_without_failure`, `census_rejects_failed_reason` (the `surfaced-across-boundary` fixture) | check |
| SINK × CEN | a `channel` row holds only when **every** `Err` arm reaches a channel function; `run_reaction`'s row (`natives_promise.rs:689`) passed the prototype on its `Some` arm while its `None` arm dropped (D-16, P0a-1 §5.5); after P0a-1 both arms reach a channel (`settle_promise`, `fail_reaction_owner`) | control `census_rejects_failed_reason` (a `channel` fixture with one dropping arm) | check |
| CEN × PEND | a D row is red outside `# pending`; a pending row outside the ceiling (the key set P0a-2b ratifies, P0a-2b §6) is red, so the section only shrinks; a row that loses its reason under rule 12 is **not** a pending row (it is an instrument or legibility defect, fixed in this slice), so the ceiling is unchanged | controls `census_rejects_definitive_row`, `census_pending_row_outside_ceiling` | check |
| CEN (rule 6) × B | at base, B-1 / B-2 are rule 6 members; after P0a-1 they are pure throw arms (auto-recognised), leaving only the "throw-routed" P rows; rule 6 guards catchability (P0a-1 §5.2) | §5 rule 6 | check |

## §3. Spec coverage map

No spec defines the census. The check implements no algorithm; the rows are the two clauses its admission
predicates rest on (the rest are P0 umbrella §3 and P0a-1 §3; the allowlist's rows are P0a-2b §3).

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| ECMA-262 §17 Error Handling and Language Extensions | paragraph 3 (no other kinds of errors as early errors) | no spec step yields the error, so every admission reason is a property of the code, checkable by the tool, never a spec branch | rule 12 (§5) | ✓ | no |
| WHATWG DOM §2.9 Dispatching events | *inner invoke* step 2.5 (once: remove before the call), 2.13.1 (report on a throw) | the write stands on every outcome (P0 umbrella §4 C-4, β); what the check verifies is that the call's error is reported, not dropped | rule 12 `channel` / `dropped-top-only` predicates; their fixtures in §8 | ✓ | yes |

**Breadth**: K=2 specs (ECMA-262, WHATWG DOM), M=2 entries (verified 2026-11-10 by `python3
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
- `.claude/tools/test_err_consumer_census.py`: tracked `test_*.py`, collected by `scripts/python-suites.sh`
  into the `tools` job; runs the controls and mutants of §8 and `--report` on the tree (§0 "Two modes").
  P0a-2b adds the gate run on the tree (rc 0) with the allowlist.

The allowlist's row format (the key, the class, the reason code and its arguments) is the check's input
format, so it is specified here and the fixtures of §8 use it: one row per non-auto member: path, enclosing
function, callee, shape, class, **reason code**, reason arguments (the channel function for `channel`, the
cut occurrences for `collision`, the consuming rows for `drain-consumed`), free text. Keyed by (path,
function, callee, shape, occurrence), not by line. A `# pending` section holds the D rows. The file
`.claude/tools/err-consumer-census.tsv` and its content are P0a-2b's.

## §5. Rules

0. **Resolution (NEW; the answer to R4 Axis 2 IMP-3).** A call occurrence is resolved to a set of
   definitions, not to a name: `recv.m(` → the methods named `m` (definitions with a `self` parameter);
   `path::f(` → the definitions of `f` in the module `path` names (a lower-case segment) or, for a type
   segment, its associated functions; bare `f(` → the definitions of `f` in the same file, else the free
   functions of that name. An occurrence that resolves to nothing known keeps an edge to **every**
   definition of the name (the over-approximation is kept on the fail-safe side; it costs rows, never
   silence). Edges are (caller definition → callee definition); every reachability predicate below runs on
   this graph. Rule 10's cut is an occurrence of this graph.
1. Populations per P0 umbrella §5.1 (Err, boundary), recomputed from the code on every run; test files
   and `#[cfg(test)]` code are excluded by the lexer (§4), not by a list. The crates scanned are derived:
   `elidex-js`, every workspace crate whose Cargo.toml depends on it (the shell), and every crate whose
   non-test source names a kind-bearing type (rule 11) or declares a trait that `elidex-js` implements.
   **Reach (PM 603).** A carrier reaches a producer when the resolved graph (rule 0), closed over **every**
   function, has a path from it to a derived producer or a `PLANNED_PRODUCERS` name (§6). The closure is
   not restricted to carrier-typed functions. Whether an intermediate function absorbs the error is a
   property of its body, so the next edit can change it without turning anything red
   (`memory/feedback_derived-populations-shrink-in-silence.md`); the absorbing use is a member at its own
   site anyway. The prototype's carrier-typed closure is how the five `dispatch_vm_simple_event` drops fell
   outside (measured: §7.1). The over-approximation costs rows, never silence: `drain-consumed` (rule 12)
   admits a use whose error is consumed beneath it. Rule 3 makes the allowlist the ratchet: a member that
   vanishes leaves a stale row, which is red.
2. Auto-recognised shapes are narrow: `?`; `.map_err(f)?` where `f` is `vm_error_to_thrown` or a `From` /
   `Into` conversion; `return` / tail in a carrier function; an arm (or an `if let Err(e) = ..`
   block) whose whole body is `self.throw_error(e, ..)?` or `return Err(e)` or `Err(e)`. Anything else is a member.
3. A member without a row is red; a row without a member is red (the allowlist cannot drift either way).
4. Admissible classes: P, T, R, S, E, N, X (P0 umbrella §5.2), **each only through a reason code of rule
   12**. A D row is red, except in the `# pending` section, whose rows name their fixing slice
   (`P0b-VM-1` / `P0b-VM-2` / `P0b-L3`) and lie inside the ceiling (P0a-2b §6); PR-1's precondition is an
   empty section (P0 umbrella §7 L-1).
5. **Top-only (generalised, PM 544).** Any row of any class whose reason is or includes top-only (reason
   codes `top-only`, `thread-entry`; the `dropped-top-only` code includes it) is red if the reverse
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
7. *(Retired, PM 595: the W markers. The number is not reused. Why: the W retirement paragraph at the
   top of this memo. D-14 and D-15 are seen by their drops (rule 5 on the S claim), not by their writes.)*
8. *(Retired, PM 598: the loop rule and its code `loop-no-write-no-drain`. The number is not reused.
   Why: its admission was the symptom's vocabulary, a write or a drain before the `?`
   (`memory/feedback_checks-must-not-be-defined-by-the-symptom-vocabulary.md`). Under β neither half
   detected a contract violation. A write before the call is the spec's order or bookkeeping (the W
   retirement paragraph). A take before the call is lost the same way on an ordinary exception, except
   where the carrier has no ordinary exception, which is rule 13's property. The property the rule was
   meant to protect splits in two, each defined by the property: (β′) RL-only propagation, rule 13, in
   this slice; (α) a report-and-continue loop that ends on one iteration's `Err`, which is not specific to
   the resource limit (§5 "(α)", slot `#11-report-and-continue-loop-census`, §9). A `?` inside a loop is rule 2's like any other `?`; the
   120 members leave the allowlist (§7.1). The "loop write" definition went with the rule, because nothing
   else read it.)*
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
| P · `rl-only-pinned` (rule 13) | the row names a `#[test]` function in a tracked test file of `elidex-js` whose raw text carries the marker `// census: <path>::<function>` of the member's key; a missing function or marker is red | the cell |
| T · `throw-arm` | the Err path calls `throw_error(` or `vm_error_to_thrown(` and `handle_exception(`; or the function returns `VmError` and yields the error as a value for its caller's throw | — |
| R · `channel` | **every** arm of the Err path (each `Err` arm, each branch inside it), or the function the binding is passed to (one level), calls a named channel function with the error or its materialisation; an arm that reaches none is red (the D-16 shape); the channel set is the C-3 list (promise / request rejection, `settle_promise`, the report stand-in `eprintln!` / `report_*`, stream error, `EvalResult` construction, the internal-reaction sink `fail_reaction_owner` (NEW: P0a-1 §5.5)); a channel not on the list is red | the channel function(s) |
| R · `surfaced-across-boundary` | the call is to a B1 boundary method (rule 9) whose return type carries `ScriptFailure` (rule 11), and the implementing method's own use of its carrier passes `channel` (the engine reported before returning `Err`) | the implementing method |
| R · `drain-consumed` (B1; Err rows and rule 13 members, PM 603) | with every consuming row's edge cut ((function, callee) of the R / T / S / E / N / X rows and the `# pending` rows), the method (for an Err row, the callee) reaches no producer; the row lists the consuming rows its paths hit (PM 540 (3): the report path) | the consuming rows |
| S · `dropped-top-only` | `top-only` holds (rule 5); under β a write before the call does not bear on the drop (the W retirement paragraph at the top of this memo) | — |
| E · `replaced-both-surfaced` | the use is `.err()` or `.is_err()`-tested and, after it, the original or the replacement leaves as `Err(..)` or as the value of a `-> VmError` function | — |
| N · `top-only` | rule 5 | — |
| N · `thread-entry` | the function is the body, or a callee of the body, of a `spawn*` closure (rule 5's cut) | — |
| N · `collision` | rule 10 | the cut occurrences |
| N · `closure-literal` (B3) | the argument is a closure literal whose body reaches no producer; a parameter or a non-literal argument is red unless the row names the enclosing function's own closure parameter and every call site of that function passes a literal that passes this predicate (transitive, one level) | — |
| X · `test-support` | the function has no production caller on the resolved graph (a file declared `#[cfg(test)] mod x;` is excluded by rule 1 and has no row) | — |
| *(no code)* `argument-class`, `unreachable-by-construction`, `reviewed` | **no predicate**: not admissible. The code is made legible instead (P0a-2b §7.4): call the non-carrier conversion; remove the unreachable arm (P0a-1 §4.5 BC-3); pass a closure literal | — |

The predicate texts above are the specification of the check; the check's docstring quotes this table and
`test_err_consumer_census.py` has one red control per code (§8).

13. **RL-only propagation (β′; PM 598, PM 603; NEW).** A carrier is **RL-only** when it reaches a
    producer (rule 1's reach) and has no **ordinary witness**. An ordinary witness is a path along *Err-flow*
    edges (a call whose use is auto-recognised propagation, rule 2, or a P `propagates` row) from the
    carrier to an *ordinary source*, every hop of which is an occurrence that rule 0 resolves to exactly one
    definition. An ordinary source yields an `Err` of another kind: a `VmError` constructor whose
    definition names one kind other than `ResourceLimit` (derived from the constructors' bodies, as §6
    derives the producers), a `VmError { kind: <other kind>, .. }` literal, `CompileErrorKind::Syntax` (NEW: P0a-1), or
    the interpreter's `Op::Throw` / `Op::EndFinally` re-raise (a script's own throw). Fail-safe: an unknown
    kind, a constructor that names more than one kind, a hop through an unresolved or colliding occurrence
    and a hop the check cannot classify each count as no witness, so the carrier is RL-only. **Every use of
    an RL-only carrier is a member, the propagating ones included.** Rule 2's auto-recognition rests on
    "an early exit leaves the state an ordinary exception leaves on the same path". On an RL-only path no
    ordinary exception exists, so whatever the exit abandons is new under P0: a fan-out's remaining
    subjects, items taken from a store, a latch already set (Axis 2 IMP-2 on Rev 3b: `abort_signal`
    `:592` / `:610`). Its admissible codes are `drain-consumed` (nothing flows: the error is consumed
    beneath) and `rl-only-pinned` (a cell injects the failure at this use and asserts the state the
    spec's exception branch leaves, C-4). A non-propagating use keeps its rule-2 class. **Derivability.**
    The RL side is §6's derivation, total since Rev 3b MIN-4 (every non-predicate mention of the kind).
    The ordinary side needs no totality: a missed ordinary source only makes more carriers RL-only, which
    costs rows and never hides one. Both are computed on the same resolved graph. At the base every derived
    and planned producer is also an ordinary source on its own path (a call runs user code; the JSON,
    clone and regexp walkers have their own `SyntaxError` / `DataCloneError` / `TypeError`). So RL-only
    carriers arise in two places: where the ordinary error is consumed on the path and the resource limit
    is not (the `is_resource_limit()` arms that P0a-1 and the P0b slices add), and where nothing returns
    `Err` today (`dispatch_vm_simple_event`, whose only return is `Ok`, P0a-2b §7.3). The count is the tracked
    check's (§11 item 3).

**(α) Report-and-continue loops (PM 598 items 2–3).** The property: where the spec's algorithm reports an
iteration's exception and continues (DOM *inner invoke* step 2.13.1 inside the listener loop; HTML *report
an exception* in the custom-element reaction, microtask and observer loops), one iteration's `Err` must not
end the loop (P0 umbrella §4 C-4, "what the branch does not skip"). **Form: (ii), a per-row C-4 cell, not
(i).** Whether the governing algorithm continues past a failure is a fact of the spec, not of the code. A
loop of `get_property_value` calls in `JSON.stringify` and a loop of listener calls have the same Rust
shape and opposite spec branches (ReturnIfAbrupt; report and continue). A predicate on the code could only
key on the callback's name (`call_listener`, `invoke_callback`, …) or on the loop's report calls, and both
are the symptom's vocabulary: a new report-and-continue loop that calls through another name would fall
outside. It could not be made fail-safe without admitting by a hand-written spec class, which PM 544
forbids. The population can still be derived by property: a loop whose body reaches a script entry (CP-1
/ CP-2) on the resolved graph. Each member is then pinned by a cell that fails iteration 1 and asserts
that iteration 2 ran and that iteration 1's error was reported. **Scope: a separate slot, not P0a-2a / P0a-2b.**
Under β a resource limit is an ordinary `Err` (C-4), so a `?` that ends such a loop is as wrong for a
script's own throw as for a resource limit. That is a spec-conformance defect of the loop, and P0's census
is the census of what P0's error makes newly reachable. The part that is new under P0, an iteration whose
carrier never failed before, is rule 13's (β′) and stays here. (α) and (β′) do not fit one check: (β′) is
a property of the code, (α) a property of the spec. The slot: §9.

## §6. Producers and the planned list

Producers are derived: functions that call `resource_limit::check_entry` (the one check site for the four
entries, P0a-1 §4.2), `ensure_stack` (NEW: PR-1), or construct `VmError::resource_limit` /
`CompileErrorKind::ResourceLimit`. **Direct construction (fail-safe; Axis 1 MIN-4).** `VmError`'s fields
are `pub` (`vm/error.rs:13-14`), so a struct literal `VmError { kind: VmErrorKind::ResourceLimit, .. }`
produces the kind without any named constructor. So the derivation also keys on the variant itself:
every non-test mention of `VmErrorKind::ResourceLimit` (NEW: P0a-1) or `CompileErrorKind::ResourceLimit` makes its
enclosing function a producer, unless the check classifies the mention as a predicate. A predicate is
a `matches!(.., <variant>)` argument, the body of `is_resource_limit`, a match or `if let` pattern, or an
`==` / `!=` operand. The enum declaration is the variant's definition, not a mention. A mention the check
cannot classify is a producer (the unknown falls to the checked side, §0). `VmError`'s fields stay `pub`:
making them private is a 48-site refactor outside this slice. Until PR-1 / PR-1b land, the check also
reads `PLANNED_PRODUCERS`, a constant listing PR-1 §5.2's walker functions, the compile front end's entry
(`compile_script`), the regexp compile sites of PR-1 §6.1 (`load_constant`, `native_string_match`,
`native_string_search`) and PR-1b's `call_wasm_exported_function` / `instantiate_module`. A planned name
that is also derived is red ("remove it"), so the list only shrinks and is empty after PR-1b. At P0a-2a's
landing `call_wasm_exported_function` and `instantiate_module` are already derived (they call
`resource_limit::check_entry`, P0a-1 §4.2–§4.3), so they are not in the list P0a-2a ships.

## §7. The measured population report

The report is P0a-2a's deliverable for the population (PM 609): the tracked check's `--report` at P0a-2a's
head records every number of this section again (§10 L-a2a). The numbers below are the prototypes' at the
base; they are history once the tracked check reproduces them, and the classification measures again at the
then-current main (P0a-2b §6).

### §7.1 The census at the base (unchanged numbers)

Prototype at `ce603c74` (verified 2026-11-08 via
`python3 -I H/scripts/err_consumer_census_proto.py <wt> crates/script/elidex-js`, log
`H/logs/err-census-p0.log`): 4307 function bodies, 2236 carriers by type, 1042 carriers that reach a
producer; 1575 uses propagate by `?` and 188 by return / tail, auto-recognised; 196 non-propagating uses in
`elidex-js` and 6 in the shell (`H/logs/err-census-p0-shell.log`, seeds `eval drain_timers eval_script`),
202 rows, each hand-classified (`H/scripts/err_census_classify.py`, output
`H/logs/err-census-p0-classified.tsv`: P 80, T 44, R 32, S 4, E 10, N 8 + 6 shell, X 4, D 14 rows at 13
sites). **Rule 8's 120 members leave with the rule (PM 598).** They were verified 2026-11-08 via `LOOPQ=1
python3 -I H/scripts/err_loop_question_proto.py <wt> crates/script/elidex-js` (log
`H/logs/err-loop-question-p0a.log`: 99 `for`, 9 `while let`, 5 `loop`, 4 `for` inside `while`, 3 `while`) and
hand-classified P (`H/logs/err-loop-question-p0a-classified.tsv`: 82 *loop-spec-abrupt*, 17
*loop-conversion*, 21 *loop-graph*). Each is a `?`, auto-recognised by rule 2 now, unless rule 13 makes its
callee RL-only (§11 item 3). Total at the base under the old reach: 202 Err rows (was 322). W
population (retired by PM 590): 5 functions (`python3 -I H/scripts/err_write_before_call_proto.py <wt>
crates/script/elidex-js`, log `H/logs/err-write-before-call-p0.log`).

**The population under PM 603's reach.** `ALLFN=1 python3 -I H/scripts/err_consumer_census_allfn_proto.py
<tree> crates/script/elidex-js` (the census prototype with its closure over every function; `<tree>` is
`git archive ce603c74`, committed into a scratch repository), log `H/logs/err-census-p0-allfn.log`. Without
`ALLFN` the same script reproduces §7.1's 196 rows. The closure reaches 3244 names (1042 before) and 2135 of
the 2236 carrier-typed functions (1042 before); 3115 uses propagate by `?`. **770 non-propagating uses in
`elidex-js` (196 before), 574 of them only under the new reach** (`H/logs/err-census-p0-newpop.tsv`; no row
leaves). The shell is unchanged: its 3 carrier names reach under both closures, 6 rows. Shapes of the 770:
binding with another use 313, `let _ =` 128, argument / arm / other 108, `if let` 62, `match` 58, the rest
≤ 16 each. The five `dispatch_vm_simple_event` drops are among the 574, with two more of the same shape
(`websocket_dispatch.rs:114` `fire_vm_close_event`, `:177` `fire_vm_message_event`). The 574 are not
hand-classified. `drain-consumed` decides most of them, and it needs rule 0's resolved graph: on the name
graph, with every consuming row's edge cut, all 574 callees still reach a producer through std-method names
(`H/scripts/err_census_consumed_beneath_proto.py`, log `H/logs/err-census-p0-newpop-consumed.log`). So
their classes, and what they add to `# pending`, are the classification's (P0a-2b §6), run with this
slice's checker. Total at the base under PM 603's reach: 776 Err rows (770 + 6).

**A measured hint, scoped to its sample (P0a-1 Rev 3c re-review, Axis 3).** A hand sample of **46** of the
574 rows found **0** consumer defects: **35** admissible and **11** latent. The latent ones are all UA-fire
`let _` drops of the shape of the seven `dispatch_vm_simple_event` drops (the UA-fire drop class, owned by
P0b-VM-2, PM 608 / PM 609; its members are listed in [P0b-VM-2](2026-11-vm-stack-safety-guard-p0b-vm2.md)
§1). The sample says nothing about the other 528 rows. **Callee breakdown of the 574, an input to the
classification and not a classification:** 413 rows whose callee is an `expect` name collision, 60 uses of
the Ok-only `settle_promise` (no `Err` flows at the base; P0a-1 §5.5 names what would change that), and
about 101 left to check by hand. The report at P0a-2a's head prints the same breakdown from the resolved
graph.

**Boundary population (rule 9; PM 533 (2)).** Prototype at `02386ec9` (code identical to `ce603c74`),
`python3 -I H/scripts/err_boundary_census_proto.py <wt>`, log `H/logs/err-boundary-census-p0.log`, name
graph (pre-rule-0): the producer closure over every function reaches 3244 of 4307 names. Hand-classified:

| Section | Scanned | Members | By class |
|---|---|---|---|
| B1 foreign-trait methods (`ScriptEngine` 7, `HostDriver` 53, `FilterAction` 1, `MutationDispatcher` 1; std traits skipped by the prototype) | 62 | 27 | **D 1** (D-14, `call_listener`); **boundary collapse 3** (BC-1 `eval`, BC-2 `drain_timers`, BC-3 `FilterAction::accept`), all fixed in P0a-1; R 13 (12 drains + `set_history_state`, P0a-1 §4.5); N 9 (collision); P 1 (`with_bound`) |
| B2 `pub` `elidex-js` fns called by the shell (the only dependent crate) | 8 calls | 2 new | 6 are the shell's calls of B1 `HostDriver` methods by name (no new row); N 2: `finish` (collision with `finish` in `compiler/function.rs`, `app/render.rs:296`), `sw_thread_main` (`app/sw_coordinator.rs:214`, a fresh thread's top, rule 5's thread-entry cut) |
| B3 closure / fn-pointer parameters called from `elidex-js` | 21 calls | 21 | N 21: Rust-only closures (`with_range` 7, `with_range_mut` 3, `reconcile_event_handler`'s scripting gate 1, `for_each_shadow_inclusive_descendant` 1 in `custom_elements/lookup.rs:159`, `ws_keepalive` / `es_keepalive` 2, the WebCrypto RNG of `sign` / `generate_key` 2), and 5 name collisions with `sign` (the BigInt `.sign()` in `coerce.rs:23`, `coerce_ops.rs` 58 / 64 / 70, `pools.rs:142`); `range_proto.rs:361` passes a parameter, not a literal |
| W across the boundary (Rev 2's W rule; the population retired by PM 590, the rule deleted by PM 595) | 80 calls, from another crate, of a B1 / B2 member's name (8 of them are `finish` collisions) | 3 with a write before the call | **D 1** (`script_dispatch_event_core` → `call_listener`, the write at `:499`: D-14); P 1 (the same write before `run_microtasks` at `:513`); N 1 (`layout_inline_context_fragmented` → `finish`, the B2 collision) |

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
| rule 8 (120; retired, PM 598) | — | the rows leave the allowlist (rule 2's `?`) | the 2 RED (`natives_json/stringify.rs:192`, `:204`) leave with them | — | — | 0 |
| B1 (62; std skipped) | 62 | 31 not members on the resolved graph; 4 BC / D-14 (kind-bearing after P0a-1); 13 `drain-consumed` (12 drains + `set_history_state`, each with its consuming rows listed in the log); 1 `pass-through` | 13 `collision` (the 9 of §7.1 plus `remove_listener`, `sync_dirty_canvases`, `has_pending_session_history_work`, `MutationDispatcher::dispatch`, which the wider scan makes reach a producer through std-method names): no occurrences named yet | 0 | 0 | 0 |
| B1 std traits (rule 9 as written, `-nostdskip`) | +45 | +31 not members | +14 `collision` (`Drop::drop` ×4: `engine.rs:427`, `:800`, `temp_root.rs:101`, `:204`; `next` of two `Iterator` impls: `typed_array_hof.rs:85`, `value.rs:250`; `Default::default` ×7; `Display::fmt` 1 — none runs script, R4 Axis 1 MIN-2) | 0 | 0 | 0 |
| B3 (12 detected by this prototype; §7.1's detector counted 21, §11 item 1) | 12 | 10 `closure-literal` | 2 (`range_proto.rs:361` passes a parameter; `dom_bridge.rs:492` passes a non-literal) | 0 | 0 | 0 |
| W across (retired with the W population, PM 590 / PM 595; the re-run's verdicts are kept as history) | 3 | 2 (`call_listener`, `run_microtasks`, as admissible under Rev 2's revert); the `finish` N row is stale on the resolved graph (the resolved `finish` is not the VM's) | 0 | 0 | 0 | 0 |

**Rows that lose their reason, and who fixes them.**

- **One consumer defect**: D-15 (`natives_promise.rs:565` / `:591`), the S row's "nothing written" is false
  (a listener removal precedes the call) and its "top-only" is false (rule 5 generalised finds the
  `dispatchEvent` path, P0a-1 §3.1). Fixed in **P0a-1** (§5.4), like D-14; never pending.
- **A second consumer defect, found after the re-run** (P0a-1 Rev 2 re-review, Axis 2 IMP-2): D-16,
  `run_reaction` → `call` (`natives_promise.rs:689`). The prototype gave it R `channel` OK on its
  `Some(capability)` arm (`settle_promise`); its `None` arm, taken by every internal subscriber, calls no
  channel. The `channel` predicate now requires every arm (rule 12). Fixed in **P0a-1** (§5.5); never
  pending.
- **Instrument debt (this slice)**: 37 UNFOLLOWED + 1 RED (T) = 38 Err rows (rule 8's 2 RED left with it, PM 598), 13 (+14 std)
  B1 collision rows, 2 B3 rows. None is a consumer defect; each is a shape the tracked check must follow,
  a cut the row must name, or a predicate that must be typed. P0a-2a's acceptance is a report with **no
  UNFOLLOWED verdict on these rows**: the check follows the shapes the prototype did not (`let x = match …` with a throw arm; `if let
  Some(r) = f() { return r }` fall-through on `Option<Result>`; an arm value of a bound match; a closure
  argument to an iteration helper (`validate_pair_entry` ×2); a binding passed to a function whose body is
  the channel (`finish` ×9, `settle` inside `make_async_coroutine_and_drive`); a `.map(closure)` chain), and
  the report gives each collision member the occurrences a `collision` row would name. A shape that
  resists is named in the report and becomes a legibility rewrite of P0a-2b (P0a-2b §7.4, which also holds
  the three rewrites this re-run names).
- **`# pending`**: no row of this re-run is a pending candidate: D-14 and D-15 are fixed in P0a-1, and
  instrument debt is not a consumer defect. The section's key set is P0a-2b's (P0a-2b §6).

## §8. Controls and mutants

Controls (tempfile fixtures), each red: `census_rejects_unknown_member` (an added `.ok()` on a carrier
call); `census_rejects_stale_row`; `census_rejects_definitive_row` (a D row outside `# pending`);
`census_pending_row_outside_ceiling`; `census_planned_producer_already_derived`;
`census_resource_limit_literal_is_producer` (a fixture function whose only producer edge is a struct
literal `VmError { kind: VmErrorKind::ResourceLimit, .. }`, and a carrier caller that drops its result
with `.ok()` and has no row → red; a function that only tests the variant with `matches!` derives
nothing; its mutant removes the variant-mention key of §6, so nothing derives the literal and the
control turns green);
`census_top_only_reachable_from_native` (fixtures: an N row and an S row claiming top-only, each
reached from a native); `census_run_question_mark_on_carrier` (a new `?` on a carrier in `run()`);
`census_cfg_test_hook_not_producer` (a fixture whose only producer edge is
a `#[cfg(test)]` statement must not derive a producer; red when it does); for the reach (rule 1, PM 603):
`census_reach_through_non_carrier` (a fixture `fn g() { let _ = producer(); }` returning `()`, `fn f() ->
Result<(), VmError> { g(); Ok(()) }`, and a caller `let _ = f();` with no row → red: the reach passes
through `g`, which is not carrier-typed), `census_reach_dispatch_drops` (on the tree: the seven named
members of the UA-fire drop class (P0b-VM-2 §1), the five PM 603 names and two of the same shape, keyed by (path, function, callee), are members
whatever their rows; a fix that removes a call updates
the control in the same commit), and mutant **M-REACH** (the closure restricted to carrier-typed functions,
the prototype's: both controls fail because the members vanish); for rule 13:
`census_rl_only_propagation_member` (a carrier `f` whose only `Err` source is a producer, and `g` with
`f()?` inside a `for` and no row → red; the same with `f` also returning `Err(VmError::type_error(..))`
through a uniquely resolved hop → auto; that hop through a name with two definitions → red again; with an
`rl-only-pinned` row naming a `#[test]` function that carries the marker → green), with the mutants "rule 13
disabled" and "the witness's uniqueness requirement dropped" (the colliding-hop fixture turns green); for
the boundary (rules 9–11): `census_boundary_unit_return_member` (a fixture `impl ForeignTrait for T` whose method returns `()`
and calls a producer, no row → red); `census_boundary_std_trait_member` (the same for `impl Iterator for T`
whose `next` calls a producer → red; a `Drop` likewise); `census_boundary_kind_bearing_auto` (the same method
returning `Result<(), ScriptFailure>` → not a member); `census_boundary_kind_bearing_container`
(`Vec<ScriptFailureBearing>` → not a member; `Vec<u8>` → member); `census_boundary_unparsed_impl_member` (an
`impl` header the lexer cannot parse → member); `census_boundary_closure_param_member` (a call passing a
closure to another crate's `impl FnMut` parameter, no row → red; with a `closure-literal` row and a closure
literal that calls a producer → red; with a non-literal argument → red); `census_collision_row_other_path` (a `collision` row whose member still reaches a producer once the
named occurrences are cut → red); `census_collision_cut_by_occurrence` (a `collision` row naming an
occurrence of a name with one definition → red; a cut that would remove another caller's edge of the same
name must not make a red member green); `census_unresolved_call_is_every_definition` (an occurrence the
resolver cannot place keeps an edge to every definition: a fixture where the only producer path is through
such a call stays red); `census_kind_bearing_type_without_failure` (a `KIND_BEARING` entry whose definition
does not name `ScriptFailure` → red); for rule 12: `census_rejects_unknown_reason` (a row with a code not in
the table → red), `census_rejects_failed_reason` (one fixture per code whose predicate fails: a `propagates`
row whose Err arm is `_ => None`; a `throw-arm` row with no throw; a `channel` row naming a function the Err
path does not call, one naming a function outside the channel list, and one whose second `Err` arm calls
no channel (`if let Some(cap) = .. { settle } ` with no `else`: the D-16 shape); a `dropped-top-only` row
reached from a native; a `surfaced-across-boundary` row whose implementing method's row is not `channel`;
a `replaced-both-surfaced` row with no `Err` after the use; a
`drain-consumed` row with a producer path through no row; an `rl-only-pinned` row naming a test function
without the marker; a `closure-literal` row with a parameter
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
std-method name shared by several types stays a collision until a row cuts its occurrences); macros;
whether a loop's or a fan-out's governing algorithm continues past a failing subject (α, §5: a `?` that
ends it is rule 2's unless rule 13 makes it a member). Owner VM P4; re-evaluated with PR-1's cut-check limits when
`#11-vm-native-callback-reentry-single-loop` is designed; ledger row P0 umbrella §7 L-7.

**(α) slot: `#11-report-and-continue-loop-census` (approved, PM 608; registered in the VM-P4 slot
ledger `docs/plans/2026-07-vm-p4-slot-ledger.md` by P0a-1's PR, P0 umbrella §7 L-15).** *Defect:* a loop
whose governing algorithm reports an iteration's exception and continues can end on the first failing
iteration through `?` or `return Err`, leaving the remaining callbacks unrun. Since rule 8's retirement
(PM 598) no check enumerates such loops, and rule 2 auto-recognises the `?`. The defect holds for a
script's own throw as much as for a resource limit. No instance was found under the old reach (the
carrier-typed closure; 1575 `?` uses at the base, §7.1); the `?` sites inside loops that PM 603's reach
adds (3115 `?` uses in all) have not been examined. *Why deferred:* it is a spec-conformance defect of the
loop, not one P0's error makes newly reachable (§5 "(α)"), and each member needs a per-loop C-4 cell
because whether the loop continues is a fact of the spec, not of the code. *Owner:* VM P4 (the census
substrate); each loop's fix goes to its file's owner. *Trigger:* **P0a-2a's landing** (PM 609 asked for a
choice between 2a and 2b). The slot's population is "a loop whose body reaches a script entry on the
resolved graph" (§5 "(α)"); that needs only the resolved call graph, which P0a-2a ships. No (α) member is an
Err-census row, so the allowlist and `# pending` of P0a-2b neither derive it nor gate it, and waiting for
P0a-2b would delay the derivation by the classification pass for nothing. *Re-eval:* the ledger row's (one
home; PM 608 approved 2026-12-15, carried by P0 umbrella §7 L-15). *Form:* (ii) of §5 "(α)".

## §10. Landing obligations

What P0a-2a's PR (or, where marked, the PM / PMO at landing) does. This memo edits no ledger. Program-level
obligations (PR-1 precondition, breadth record, slots) are P0 umbrella §7.

The ledger and notice rows this slice triggers are rows of the P0 program's landing list (P0 umbrella §7,
PM 513 (d)): L-7 (census limits). Slice-local:

- **L-a2a. The population report**: P0a-2a's PR body records the tracked check's `--report` at its head:
  the members per population (rule 13's; the Err population under PM 603's reach, with the members that
  only it admits listed; the boundary population by section; and the std-trait members separately), the
  candidate reason code per member and the members with none, the callee breakdown of §7.1 re-derived, and
  any shape the report names as unfollowed (§7.2).

## §11. Unverified facts (each decided at implementation, before its control is written)

1. The B3 population: `err_boundary_census_proto.py` counted 21 calls, `err_reason_check_proto.py` 12
   (the two detectors differ on which other-crate functions take a closure); the tracked check's rule-9 B3
   derivation decides, and the PR body records the count (L-a2a).
2. Whether `dispatch.rs:430`'s `let old = match …` throw arm and the other UNFOLLOWED shapes of §7.2 are
   followed by the check's statement analysis or named in the report for a rewrite (P0a-2b §7.4).
3. Rule 13's population at the base: the RL-only carriers and their uses. Expected small at the base
   (§5 rule 13 "Derivability"); it grows with the `is_resource_limit()` arms of P0a-1 / P0b and with
   PR-1's probes, each of which re-runs the check.

(The former P0a-2 §11 items: 1, 4 and 7 are P0a-2b §11 / §6; 3 was dissolved by PM 590; 8 was settled by
PM 608 / PM 609, the UA-fire drop class is P0b-VM-2's.)
