# VM stack-safety guard — P0 umbrella (resource-limit Err contract, Err-consumer census)

Status: **plan (pre-implementation), P0 umbrella**, split per PM 509; each per-PR slice takes its own
`/elidex-plan-review` before code. Owner: VM P4. Parent: [umbrella U](2026-11-vm-stack-safety-guard.md).
Lands **before** [PR-1](2026-11-vm-stack-safety-guard-pr1.md) and
[PR-1b](2026-11-vm-stack-safety-guard-pr1b.md). Slices: [P0a](2026-11-vm-stack-safety-guard-p0a.md),
[P0b-VM-1](2026-11-vm-stack-safety-guard-p0b-vm1.md), [P0b-VM-2](2026-11-vm-stack-safety-guard-p0b-vm2.md) (PM 513 (a)), P0b-L3 (L3 writes its plan; P0's hand-off is
`H/p0b-l3-handoff.md`). Base: `ce603c74` (origin/main `69fbe61f` merged); every code anchor in the P0
memos is at that commit (`a02777c9` and later commits on this branch touch `docs/plans/` only).
Provenance root `H` is the umbrella's (`/Users/kazuaki/repos/send.sh/elidex-pm-trial/handoff/stackguard-census`).

**What lives here.** The contract (§4, the one canonical text), the census definition (§5), the coupled
invariants of the contract (§2), the slice table, edges and schedule (§1), the pre-existing deviations
the census found and their homes (§6), and the program-level landing obligations (§7). Everything a
single PR implements lives in its slice memo; this memo points to it.

**Relation to U / PR-1 / PR-1b.** The P0 memos are the single home of the rules below. U, PR-1 and PR-1b
point here and to P0a for umbrella I-3, the CE-upgrade and async-function cases and the placement mutants
(fold revision, PM 502). This memo is also the home of the P0 / L3 schedule (§1) and of the P0 program's
landing list (§7, PM 513 (d)).

## §0. Charter

### §0.1 Why (PM 502)

Guard plan-review R1 and R2 each found, one site at a time, a consumer that takes an `Err` from a
probe-bearing call and turns it into a definitive fact: custom-element upgrade marks the element Failed
(R1); caller-side state mutated before `with_call_mode` (R1); the lazy event-handler compile maps every
`Err` to `None` and its caller removes the listener (R2 S-1); no mutant pins probe placement (R2 S-2). The
common root is the absence of a contract for the guard's error and of a census of its consumers. P0 states
the contract once (§4), gives the error an identity that consumers can test (P0a §4), enumerates every
consumer by a property (§5; the check, P0a §5), fixes every consumer that violates the contract (P0a §6,
P0b-VM-1, P0b-VM-2, P0b-L3), keeps the census honest with a tracked check whose unknown case is red, and pins each fix
and each choke-point placement with a cell and a mutant (in each slice).

### §0.2 What P0 owns, and what it does not

P0 owns: the contract C-1…C-5 (§4); the error identity types and the escape rule; the **placement** of
the four choke-point checks CP-1, CP-2, CP-3a, CP-3b, i.e. the VM half of umbrella I-3; the Err-consumer
census and its tracked check; the fix of every definitive consumer; their cells and mutants. The split of
these among the slices is §1.

P0 does not own: the primitive `crate::stack_guard` (NEW: PR-1), the reserve, the per-thread bound, the walker probes,
the compile-time probes, the caps, the regexp front door (PR-1); `MAX_WASM_STACK` and the CP-3 ask (PR-1b);
the frame cap (PR-2). Those PRs put their probe **inside** P0a's check (P0a §4.3) and produce the error
P0 defines.

### §0.3 Order and why P0 is first

PR-1's probes are the first production producers of a resource-limit `Err`; the definitive consumers
become live failures the moment a producer exists. So every definitive consumer is fixed before PR-1
lands. At P0's landing no production code produces the error: the cells produce it through P0a's
test-only injection (P0a §4.4), and nothing in production depends on a step that is not yet verified.
The one rule whose input does not exist before PR-1 (the compile-channel aggregation, P0a §4.1 item 4)
is stated in P0a and implemented with its first producer in PR-1, with PR-1's cell.

### §0.4 PM rulings

- **Split (PM 509 item 1).** This memo is the P0 umbrella; the slices are §1's. Each P0b PR is a narrow
  slice that only removes rows from the census `# pending` section; under this umbrella each is a base
  case (c) of the edge-dense rule (CLAUDE.md, Design discipline).
- **Owners (PM 509 item 2).** D-1…D-5 → L3 (P0b-L3); D-6…D-12 → VM P4 (P0b-VM-1: D-6, D-7, D-10, D-11;
  P0b-VM-2: D-8, D-9, D-12, PM 513 (a)); D-13, B-1, B-2 → VM P4 (P0a).
- **Step 3.7's not-parsable arm (PM 509 item 3)** is in P0b-L3's scope (§6 item 4).
- **Slots (PM 509 item 4).** §6 records each home; the ledger writes are rows of §7.
- **C-4 reading ratified (PM 509 item 5).** Spec-retried (re-entry) subjects are reverted; the rest take
  the spec's exception branch with the `RangeError` (§4 C-4, D-P0-4).
- **D-9 → P0b-VM-2 (PM 509 item 6, PM 513 (a))**, including making `startMessages()`'s flush asynchronous.
- **D-P0-2 ratified (PM 509 item 7).** A custom-element constructor that started and then ran out of
  stack takes the threw branch, because its side effects have already happened.
- **Slot id (PM 510).** The IteratorClose slot is `#11-vm-iteratorclose-precedence-convention`.
- **PM 513.** (a) P0b-VM is split into P0b-VM-1 (D-6, D-7, D-10, D-11; WHATWG HTML only) and P0b-VM-2 (D-8,
  D-9, D-12), along the seam the single memo named; recorded reason: four independent sites are a reason to
  split, not an argument for a single PR. (b) `history_events.rs` counts as an L4 file; P0b-VM-1's site list
  is sufficient. (c) The single SW delivery queue (`PendingTask::SwMessage` (NEW: P0b-VM-2)) is recorded on
  `#11-sw-client-postmessage-inbound` (§7 L-10). (d) The ledger rows of this memo and of the slices form the
  P0 program's landing list (§7), copied mechanically by the PMO from the merge commit's memo text.
  **Ratified driver decisions**: the census rule "a `?` inside a loop body is a member" (P0a §5.2 rule 8)
  and the single test-failure seam (L3's `fail_next_abort_dispatch` moved onto P0a's injection, P0a §4.4).
- **CE bundle order (PM, 11-08).** Where P0b-L3 overlaps `custom_elements/upgrade.rs`, P0b-L3 stays limited to
  the P0 contract; L3's CE bundle (TypeError, read-once, `#11-ce-upgrade-reaction-enqueue-order`) rebases
  onto it afterwards.

### §0.5 Decisions

- **D-P0-1 Identity by kind, not by message.** A resource-limit error is a distinct `VmErrorKind`
  (P0a §4.1). A message compare or a `RangeError` kind cannot be told apart from a script's own
  `RangeError` (`new Array(-1)`).
- **D-P0-2 "Did not start" semantics with an escape rule (ratified, PM 509 item 7).** The kind means the
  guarded step never began. Once the error has been thrown into a started script frame it is that
  script's exception (P0a §4.2). No hidden marker on the error object: a marker would make a constructor
  that ran halfway "retryable", and re-running a half-run constructor is worse than the spec's threw
  branch.
- **D-P0-3 One choke-point check, placed by P0a, filled by PR-1 / PR-1b** (P0a §4.3).
- **D-P0-4 Re-entry subjects revert; other subjects take the spec's exception branch** (C-4, §4;
  ratified, PM 509 item 5).
- **D-P0-5 The census is a tracked check in the `tools` job**, with the population derived from the code
  and an unknown member red (P0a §5). Not `trip-wires`: the check needs an interpreter (CLAUDE.md §CI job
  boundary).
- **D-P0-6 Caps are not resource limits.** `MAX_JSON_DEPTH`, `MAX_NESTING_DEPTH`, `MAX_REGEXP_DEPTH`,
  `DENSE_ARRAY_LEN_LIMIT` and the like are answers determined by the input (umbrella I-4); the same input
  fails the same way at any depth. The contract covers only the probe-originated error.

## §1. Slices, edges and schedule (canonical)

| Slice | Content | Owner | Memo | Census `# pending` effect |
|---|---|---|---|---|
| P0a | VM core: error kinds, `STACK_EXHAUSTED_MSG`, `resource_limit::check_entry` (NEW: P0a) and CP placement, escape rule, B-1, B-2, D-13, test-injection seams (including L3's abort-dispatch seam, moved), the tracked census check and its allowlist (D-1…D-12 and the two D-state W functions in `# pending`), placement / identity cells and mutants | VM P4 | [P0a](2026-11-vm-stack-safety-guard-p0a.md) | creates the section (P0a §5.4) |
| P0b-VM-1 | D-6, D-7 (history; L4 files, no driver), D-10, D-11 (worker); WHATWG HTML only | VM P4 | [P0b-VM-1](2026-11-vm-stack-safety-guard-p0b-vm1.md) | removes its 4 rows |
| P0b-VM-2 | D-8 (IndexedDB), D-9 (SW `startMessages()`, made asynchronous), D-12 (WebCrypto) | VM P4 | [P0b-VM-2](2026-11-vm-stack-safety-guard-p0b-vm2.md) | removes its 4 rows |
| P0b-L3 | D-1…D-5 and HTML §8.1.8.1 step 3.7's not-parsable arm | L3 | L3's own plan; P0's input is `H/p0b-l3-handoff.md` | removes its 5 D rows and 2 W rows |

**Edges.** P0a → P0b-VM-1; P0a → P0b-VM-2; P0a → P0b-L3; P0b-VM-1 → PR-1; P0b-VM-2 → PR-1; P0b-L3 → PR-1.
The three P0b slices are independent of each other (disjoint files; each touches only its own rows of
`# pending`). L3's `#11-abort-signal-any-dependent-flattening` PR → P0a and → P0b-L3: P0a's allowlist
carries the row of the flattened `AbortSignal` abort loop and P0a moves that PR's test seam onto its own
injection (P0a §4.4, §5.4), so P0a is rebased on L3's PR; if the order inverts, L3's PR writes that row
instead (same place) and P0a still moves the seam. P0b-L3 → L3's CE bundle (§0.4, CE bundle order).
**PR-1's precondition** is an empty `# pending` section (§7 L-1). Every other memo of the program points
here for edges and dates and does not restate them.

**Schedule (the one home of the P0 / L3 dates; L3 estimates via PM, 11-08).**

- L3's `#11-abort-signal-any-dependent-flattening` PR: a separate small PR, not part of L3's walker
  umbrella; L3 starts it right after #607 Rev 4 (plan and plan-review 11-09, its plan Rev 2.1 on 11-09;
  implementation and pre-push 11-09 to 11-10; PR around 11-10; landing expected 11-10 to 11-11 after Codex
  review). Surfaces: `abort_statics.rs` and a new engine-independent DOM §3.2 "create a dependent abort
  signal". It is also the hard edge W-11 slot → PR-1 (umbrella §4).
- P0a: first of the P0 slices, after that PR. P0b-VM-1 and P0b-VM-2: after P0a, in either order.
- P0b-L3: after P0a and after that PR; PR around 11-12, with a per-PR plan-review.
- L3's `#11-dom-walkers-iterative-over-ecs-links` umbrella (with the capped-walker slot): start ≈11-11,
  slice 1 (`elidex-ecs` `tree/walkers.rs` + the capped walker's 8 callers) landing ≈11-14, the whole
  umbrella ≈end of November, i.e. after PR-1 (the soft edge PR-1 → that slot, umbrella §1.2).

## §2. Coupled invariants

P0 is edge-dense: the contract's five invariants intersect each other and two of PR-1's (I-1 coverage,
whose probes are the producers; I-6 one error shape per channel, umbrella §2 / PR-1 §2). Each slice memo
enumerates the pairs of its own elements in its §2.

| Pair | Intersection | Pinned by | Basis |
|---|---|---|---|
| C-1 × C-2 | C-2's predicate is `is_resource_limit()`; a consumer cannot apply C-2 to an error it cannot recognise, so C-1 must hold at every boundary the error crosses (`Vm::eval`, `native_import_scripts`, the CP-3 conversions) | P0a `rl_vm_eval_compile_resource_limit_kind`, `rl_cp3b_instantiate_entry_failure`; P0b-VM-1 `rl_import_scripts_compile_resource_limit_is_range_error` | cell |
| C-1 × C-3 | Surfacing materialises the error (a thrown or rejected `RangeError`); after materialisation it is no longer C-1's kind, so C-3 is the last place C-1 applies | P0a escape rule §4.2; `rl_escape_rule_materializes_in_started_frame` | cell |
| C-1 × C-4 | "Not started" is what makes a revert correct: nothing of the trigger ran, so leaving the subject as before is exact. A half-run step arrives as a thrown value and takes the spec branch | P0a `rl_escape_rule_materializes_in_started_frame`; P0b-L3 `rl_ce_upgrade_after_constructor_started_takes_threw_branch` | cell |
| C-1 × C-5 | The census population is "can receive C-1's kind"; its producers are derived from the code (fns that call the check) plus a planned list that only shrinks (P0a §5.3) | P0a census controls | check |
| C-2 × C-3 | C-2 forbids, C-3 prescribes; together every consumer has exactly one admissible shape per subject class (§5.2 classes P/T/R; D is the forbidden one) | §5.2 class table | structural |
| C-2 × C-4 | A spec exception branch that writes a durable fact is C-2(c) at a re-entry subject and C-3 elsewhere; C-4 is the rule that tells them apart | D-3/D-4 (P0b-L3) vs the stream rows (P0a §5.2, class R) | structural |
| C-2 × C-5 | A D row is not admissible in the allowlist (after the P0b slices; before them only in `# pending`) | P0a control `census_rejects_definitive_row` | check |
| C-3 × C-4 | Reverting a subject must still surface the error on the trigger's channel, or a revert is a silent drop | each D-state cell asserts the report / throw (P0b-L3) | cell |
| C-3 × C-5 | The census classifies the channel each row uses; a row that drops the error with no state written is class S, admissible but recorded (§6 item 2) | P0a §5.2 S rows | check |
| C-4 × C-5 | Re-entry is a property of the subject, not of the site; the W population (writes before a carrier call) finds the subjects whose state is written before the call | P0a §5.2 W rows | check |
| C-1 × I-1 | PR-1 / PR-1b's probes are C-1's producers: the choke-point probes sit inside P0a's check, and every walker / front-end / front-door probe converts through `From<StackExhausted>` to `VmError::resource_limit` (NEW: P0a) or `CompileErrorKind::ResourceLimit`, so every probe failure has C-1's kind by construction | P0a §4.3; PR-1 §4.1 | structural |
| C-2 × I-1 | Before a producer exists a definitive consumer is latent; I-1 makes it live, which is the order of §0.3 | §7 L-1 | ledger |
| C-3 × I-1 | Surfacing runs inside the reserve: building a `RangeError` and reporting it must fit in `WALKER_RESERVE` (PR-1 §4.3) | PR-1 `stack_guard_reserve_margin_ratchet` | PR-1 cell |
| C-4 × I-1 | A revert writes ECS state at the floor; a revert is a few component writes, bounded, no recursion | P0b-L3 (hand-off, D-1…D-5) | structural |
| C-5 × I-1 | Each PR-1 probe site that lands removes its name from the planned-producer list (P0a §5.3); the derived producer set then covers it | P0a control `census_planned_producer_already_derived` | check |
| C-1 × I-6 | I-6's runtime message is `STACK_EXHAUSTED_MSG` (NEW: P0a, PM 421 (a)); the kind is P0's, the script-visible class stays `RangeError` | P0a `rl_identity_predicates` | cell |
| C-2 × I-6 | Reclassification (C-2 (b)) is the consumer-side breach of I-6; the D-cls rows are exactly the sites that would break I-6 | P0b-VM-1 D-10 / D-11 and P0b-VM-2 D-12 cells; P0a D-13 cell | cell |
| C-3 × I-6 | The channel changes the form (throw / rejection / report), never the class | rejection rows keep `vm_error_to_thrown` | structural |
| C-4 × I-6 | Taking a spec branch with the `RangeError` (non-re-entry subjects) keeps the class: a stream errored with it, a promise rejected with it | P0a §5.2 R rows | structural |
| C-5 × I-6 | The census does not check classes; the D-cls cells do | — | cell |

## §3. Spec coverage map

No spec defines a resource limit. This memo implements no algorithm; the rows are the clauses the
contract itself rests on (§4 Grounds and C-4's re-entry subjects). The algorithms a fix touches are in
each slice's §3. Every number is from `.claude/tools/webref` (commands at the end of §6).

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| ECMA-262 §17 Error Handling and Language Extensions | paragraph 1 (errors reported when the construct is evaluated), paragraph 3 (no other kinds of errors as early errors) | a resource limit is neither an early error nor a specified error; it surfaces where the construct is evaluated | contract C-1, C-4 (no code here) | ✓ | yes |
| WHATWG HTML §8.1.8.1 Event handlers | *get the current value of the event handler* step 3 (internal raw uncompiled handler) | re-entry subject of C-4 | contract C-4 (no code here) | ✓ | yes |
| WHATWG HTML §4.13.5 Upgrades | *try to upgrade an element* steps 1–2 | re-entry subject of C-4 | contract C-4 (no code here) | ✓ | no |
| WHATWG DOM §2.9 Dispatching events | *inner invoke* step 2 (each listener still in the list) | re-entry subject of C-4 | contract C-4 (no code here) | ✓ | yes |
| WHATWG HTML §8.1.4.6 Runtime script errors | *report an exception* steps 1–7 | the report channel of C-3; elidex's stand-in is stderr (`#11-embedder-diagnostic-channel`) | contract C-3 (no code here) | ✓ | no |

**Breadth**: K=3 specs (ECMA-262, WHATWG HTML, WHATWG DOM), M=5 entries (verified 2026-11-08 by `python3 .claude/skills/elidex-plan-review/preflight.py` on this memo: rc 0, K=3, M=5, "ok (single PR scope)"); per-slice breadth is each
slice's §3 (§7 L-13 records them).

### §3.1 Cited, not touched

- ECMA-262 §7.4.11 IteratorClose step 5 (a throw completion wins over the `return` method's throw): the
  E rows of P0a §5.2 implement the opposite precedence (§6 item 1).
- WHATWG HTML §10.2.4 *run a worker*: a top-level exception is reported and the worker keeps running;
  `worker_thread.rs:200` closes the worker (§6 item 3).
- WHATWG Streams: an exception from `pull` / `start` errors the stream (P0a §5.2 R rows, C-4 non-re-entry).

## §4. The contract (canonical)

**C-1 Identity.** The guard's error is `VmErrorKind::ResourceLimit` (NEW: P0a). It is produced only by
the choke-point check (P0a §4.3) and, after PR-1, by the walker and front-end probes and the regexp front
door (PR-1 §5–§6). It means: the step the probe guards did not start. At the compile boundary it is
`CompileErrorKind::ResourceLimit` (NEW: P0a). Every boundary that converts the error keeps the kind
(`Vm::eval`, `native_import_scripts`, the CP-3 conversions of P0a §4.3). Once the error has been thrown
into a started script frame it becomes that frame's exception (escape rule, P0a §4.2) and leaves the
contract.

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
script-visible class is `RangeError` with `STACK_EXHAUSTED_MSG` (NEW: P0a; "Maximum call stack size
exceeded", PM 421 (a)).

**C-4 Spec exception branches (ratified, PM 509 item 5).** A *re-entry subject* is one whose spec
algorithm re-attempts the failed step on a later trigger, keyed on the subject's state: an event handler
whose value is an internal raw uncompiled handler (HTML §8.1.8.1 step 3), an element whose custom element
state is "undefined" (HTML §4.13.5 *try to upgrade an element*), a listener still in the listener list
(DOM §2.9), a buffered message not yet delivered. At a re-entry subject the consumer leaves the subject as
it was before the trigger and surfaces the error (C-3); the spec branch for an exception is not taken,
because in the spec's model the step ran (no spec step yields a resource limit, ECMA-262 §17 paragraphs 1
and 3). At a subject without a re-entry (a promise, a stream, an IndexedDB request or transaction, a
worker's or service worker's top-level evaluation) the spec branch is the error channel and is taken with
the `RangeError`.

**C-5 Census.** Every consumer a resource-limit error can reach is in the census; the population is
derived from the code (§5.1); an unknown member, a stale row and a definitive row are each red. The check
that enforces it is P0a §5.

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

**Applying it.** Each fix applies C-2 / C-3 / C-4 at one site, with `is_resource_limit()` as the only
test.

## §5. The census definition

### §5.1 Population (by property)

**Err population.** Every production use, in a crate that can hold the error, of a call to a *carrier*
where the error is not propagated. A carrier is a function whose return type carries the error
(`Result<_, E>` with `E` ∈ {`VmError`, `CompileError`, `JsValue`, `StackExhausted` (NEW: PR-1),
`RegExpCompileError` (NEW: PR-1)}, or `EvalResult`) **and** that reaches a producer in the call graph.
Producers: functions that call `check_entry` (derived), plus the planned list of PR-1 / PR-1b probe
functions until they land (P0a §5.3). A use is outside the population only when it propagates in a shape
the check recognises (P0a §5.2 rule 2: `?`, `.map_err(<kind-preserving>)?`, `return` / tail in a carrier
function, a pure throw or `Err(e)` arm); every other use is a member and is classified (§5.2), and a
member may still propagate (class P: a match-arm tail, `return Some(Err(e))`, close-then-return). Crates:
`crates/script/elidex-js/src`; the `EvalResult` consumers in `crates/shell/elidex-shell/src` and
`crates/script/elidex-script-session/src` (the error leaves `elidex-js` only as `EvalResult`,
`engine.rs:176-201` / `:332-357`, or as `()` from the `ScriptEngine` methods).

**W population.** Functions in which a durable write (ECS component write, custom-element state
transition, listener removal) precedes a carrier call in the same body. This is the pre-call mutation
class of R1.

### §5.2 Classes

| Class | Meaning | Admissible |
|---|---|---|
| P | propagates in a shape the check does not auto-recognise (match-arm tail, `return Some(Err(e))`, close-then-return, a kind-preserving `map_err` closure) | yes, row with the reason |
| T | thrown into the running script (`throw_error`) | yes; the check auto-recognises the pure arm (P0a §5.2 rule 2), so these need no row |
| R | the trigger's own channel: promise / request rejection, stream error, report (stderr), `EvalResult` | yes, row |
| S | dropped, nothing written; the report the spec asks for is missing | yes, row, with §6 home |
| E | replaced by another error, both surfaced | yes, row, with §6 home |
| N | not reachable by a script-controlled depth (argument class, a fresh thread's top, an embedder entry), or unreachable by construction | yes, row; top-only is checked (P0a §5.2 rule 5) |
| X | test support (panics by design) | yes, row |
| D | definitive: D-state (C-2 c), D-sub (C-2 a), D-cls (C-2 b) | **no** (only in `# pending` until its P0b slice, P0a §5.2 rule 4) |

The rows, the counts and the `# pending` composition are P0a §5 (the allowlist is P0a's deliverable).

## §6. Pre-existing deviations seen, and their homes

1. **IteratorClose precedence.** ECMA-262 §7.4.11 step 5 returns the original throw completion before
   step 6 considers the `return` method's throw; the ten E rows (P0a §5.2) let the `return` method's
   error win (comments: `dispatch_iter.rs:54-55`, `typed_array_static.rs` at 743, `webidl_sequence.rs` at
   141). Not a C-2 breach in practice; a spec deviation of its own. Home (PM 509 item 4): the existing
   `#11-vm-iteratorclose-precedence-convention`, to which the ten E rows are appended (§7 L-4).
2. **Unreported listener and callback errors.** The four S rows (P0a §5.2) drop the error with no
   report; the stderr stand-in is used elsewhere (`flush.rs`, `observer_common.rs`). Home:
   `#11-embedder-diagnostic-channel` (facet, §7 L-3).
3. **A worker's top-level exception closes the worker** (`worker_thread.rs:200-204`); HTML §10.2.4
   reports it and keeps running. Home (PM 509 item 4): `#11-worker-toplevel-error-closes-worker`, owner
   VM P4.
4. **Not-parsable inline handler removes its listener** (`event_handler_attrs.rs:548-552`); HTML §8.1.8.1
   step 3.7's note says the value becomes null and the listener stays. In scope of P0b-L3 (PM 509 item 3):
   value null, listener kept, SyntaxError reported (`H/p0b-l3-handoff.md`).
5. **A failed `AsyncDriverStep` resume** (R2 A5-IMP4): CP-2 fails, the wrapper is rejected
   (`natives_generator.rs:520-525`), the generator stays suspended, a `finally` in the body does not run.
   The row is R (the wrapper is the trigger's channel); P0a pins the outcome
   (`rl_async_step_entry_failure_rejects_wrapper`). Home (PM 502): `#11-vm-async-resume-failure-skips-finally`.
   This item is the program's one statement of the case (PR-1 §2 points here).
6. **Synchronous `startMessages()` flush** (Service Workers §3.4: an enabled client message queue is a task
   source). Removed by P0b-VM-2 as D-9's fix (PM 509 item 6); the one queue is recorded on
   `#11-sw-client-postmessage-inbound` (PM 513 (c), §7 L-10).

Out of scope, not P0's: `#11-ce-upgrade-reaction-enqueue-order` (L3's CE bundle, §0.4).

webref lookups behind §3 and this section (2026-11-08): `body ecma262 sec-error-handling-and-language-extensions`;
`body ecma262 IteratorClose` (`aoid ecma262 IteratorClose` → §7.4.11); `dfn html 'get the current value of
the event handler'` → §8.1.8.1; `dfn html 'try to upgrade an element'` → §4.13.5; `dfn dom 'inner
invoke'` → §2.9; `dfn html 'report an exception'` → §8.1.4.6; `dfn html 'run a worker'` → §10.2.4.

## §7. Landing list of the P0 program (canonical; PM 513 (d))

This section is the P0 program's landing list. It holds every ledger and repo-text row of the P0 memos (the
slices' landing sections point here) and is copied mechanically by the PMO from this memo's text **on the
merge commit** of the PR named in each row: one row = target, exact sentence, source. The P0 PRs edit no
ledger. Rows marked *PR* are done by that PR itself; rows marked *PM* are notices.

| Row | When | Executor | Target (kind) | Exact sentence to append | Source |
|---|---|---|---|---|---|
| L-1 | PR-1's PR | PR | PR-1's PR body | "P0a, P0b-VM-1, P0b-VM-2 and P0b-L3 landed (`git merge-base --is-ancestor <sha> HEAD` → 0 for each); the census `# pending` section is empty." | `docs/plans/2026-11-vm-stack-safety-guard-p0.md:119-120` |
| L-2 | P0a's merge | PM | L3 lane notice | "P0b-L3's input is `H/p0b-l3-handoff.md`; the P0 contract is P0 umbrella §4 on P0a's merge commit." | `docs/plans/2026-11-vm-stack-safety-guard-p0.md:111` |
| L-3 | P0a's merge | PMO | `#11-embedder-diagnostic-channel` (pointer row → `memory/m4-12-pr-d22-observer-pair-landing.md:71`; append there) | "Facet (stack-safety P0, P0a §5.4 S rows): listener / callback errors are dropped without a report at `engine.rs:275`, `custom_elements/flush.rs:70`, `natives_promise.rs:591` and `coerce.rs:246` (paths under `crates/script/elidex-js/src/`, lines at `ce603c74`)." | `docs/plans/2026-11-vm-stack-safety-guard-p0a.md:265-268` |
| L-4 | P0a's merge | PMO | `#11-vm-iteratorclose-precedence-convention` (live entry) | "Further sites of the same inversion (stack-safety P0a §5.4 E rows, lines at `ce603c74`, under `crates/script/elidex-js/src/`): `vm/dispatch_iter.rs` 52 / 309; `vm/ops.rs:60`; `vm/host/typed_array_static.rs` 743 / 801; `vm/natives_array_hof.rs:485`; `vm/webidl_sequence.rs` 141 / 149; `vm/host/url_search_params.rs:318`; `vm/host/structured_clone.rs:1064`. No new slot." | `docs/plans/2026-11-vm-stack-safety-guard-p0a.md:269-273` |
| L-5 | P0a's merge | PMO | `#11-worker-toplevel-error-closes-worker` (live entry) | "Found by the stack-safety P0 census (P0 umbrella §6 item 3): `worker_thread.rs:200-204` closes the worker where HTML §10.2.4 reports and keeps running." | `docs/plans/2026-11-vm-stack-safety-guard-p0.md:292-294` |
| L-6 | P0a's merge | PMO | `#11-vm-async-resume-failure-skips-finally` (live entry) | "Outcome pinned by stack-safety P0a cell `rl_async_step_entry_failure_rejects_wrapper` (the wrapper rejects with `RangeError`; the body's `finally` does not run)." | `docs/plans/2026-11-vm-stack-safety-guard-p0.md:298-302` |
| L-7 | P0a's merge | PMO | `#11-vm-native-callback-reentry-single-loop` (live entry) | "Stated limits of the stack-safety P0 Err-consumer census (P0a §5.6), re-evaluated with PR-1's cut-check limits when this slot is designed: calls through function pointers, trait objects and closures passed across functions; writes inside a callee before the carrier call; macros; a straight-line `?` that returns before a fan-out; durable writes to a side table. Re-eval-date: 2026-12-31." | `docs/plans/2026-11-vm-stack-safety-guard-p0a.md:355-362` |
| L-8 | P0b-VM-1's PR | PR | `#11-history-state-structured-serialize-fidelity` (pointer row → repo memo; append to `docs/plans/2026-07-s5-5c-history-state-traversal.md:868`, the D1 bullet) | "Resource-limit errors propagate from the interim serializer and from the push/replace deserialise (stack-safety P0b-VM-1 §5.1); the degrade for other failures remains this slot's." | `docs/plans/2026-11-vm-stack-safety-guard-p0b-vm1.md:96-104` |
| L-9 | P0b-VM-2's merge | PMO | `#11-idb-structured-clone-storage` (live entry) | "`json_to_js` is fallible: a resource-limit error fails the request; other parse failures keep `undefined` (stack-safety P0b-VM-2 §5.1)." | `docs/plans/2026-11-vm-stack-safety-guard-p0b-vm2.md:80-86` |
| L-10 | P0b-VM-2's merge | PMO | `#11-sw-client-postmessage-inbound` (live entry; the PM 513 (c) queue note is already on it) | "`fire_message`'s `unwrap_or(Undefined)` for a non-resource deserialise failure is a pre-existing substitution (spec: `messageerror`), left to this slot (stack-safety P0b-VM-2 §5.2)." | `docs/plans/2026-11-vm-stack-safety-guard-p0b-vm2.md:90-96` |
| L-11 | P0b-VM-1's merge | PM | L4 notice | "VM P4 edited the L4 sites listed in stack-safety P0b-VM-1 §0 (`structured_serialize.rs`, `history.rs`, `history_events.rs`) in P0b-VM-1's PR." | `docs/plans/2026-11-vm-stack-safety-guard-p0b-vm1.md:27-33` |
| L-12 | P0a's merge | PM | L3 notice (site list via PM, PM 502) | "P0a edited L3's AbortSignal files after rebasing on `#11-abort-signal-any-dependent-flattening`: the `#[cfg(test)]` seam `fail_next_abort_dispatch` is removed and its cell uses `InjectAt::SimpleEventDispatch` (NEW: P0a) (P0a §4.4); the allowlist row of the abort loop is P0a's (P0a §5.4)." | `docs/plans/2026-11-vm-stack-safety-guard-p0a.md:165-173` |
| L-13 | each P0 PR | PR | that PR's body | "`python3 .claude/skills/elidex-plan-review/preflight.py` on this umbrella and on the slice memo at the PR's head: rc and K / M as printed." | `docs/plans/2026-11-vm-stack-safety-guard-p0.md:178-179` |
