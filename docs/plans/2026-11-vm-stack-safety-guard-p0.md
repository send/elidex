# VM stack-safety guard — P0 umbrella (resource-limit Err contract, Err-consumer census)

Status: **plan (pre-implementation), P0 umbrella**, split per PM 509; each per-PR slice takes its own
`/elidex-plan-review` before code. Owner: VM P4. Parent: [umbrella U](2026-11-vm-stack-safety-guard.md).
Lands **before** [PR-1](2026-11-vm-stack-safety-guard-pr1.md) and
[PR-1b](2026-11-vm-stack-safety-guard-pr1b.md). Slices: [P0a-1](2026-11-vm-stack-safety-guard-p0a1.md),
[P0a-2a](2026-11-vm-stack-safety-guard-p0a2a.md) and [P0a-2b](2026-11-vm-stack-safety-guard-p0a2b.md) (PM 544
re-sliced P0a into P0a-1 / P0a-2; PM 609 split P0a-2 into P0a-2a / P0a-2b),
[P0b-VM-1](2026-11-vm-stack-safety-guard-p0b-vm1.md), [P0b-VM-2](2026-11-vm-stack-safety-guard-p0b-vm2.md) (PM 513 (a)), P0b-L3 (L3 writes its plan; P0's hand-off is
`H/p0b-l3-handoff.md`). Base: `ce603c74` (origin/main `69fbe61f` merged); every code anchor in the P0
memos is at that commit (`a02777c9` and later commits on this branch touch `docs/plans/` only).
Provenance root `H` is the umbrella's (`/Users/kazuaki/repos/send.sh/elidex-pm-trial/handoff/stackguard-census`).

**What lives here.** The contract (§4, the one canonical text), the census definition (§5), the coupled
invariants of the contract (§2), the slice table, edges and schedule (§1), the pre-existing deviations
the census found and their homes (§6), and the program-level landing obligations (§7). Everything a
single PR implements lives in its slice memo; this memo points to it.

**Relation to U / PR-1 / PR-1b.** The P0 memos are the single home of the rules below. U, PR-1 and PR-1b
point here and to P0a-1 for umbrella I-3, the CE-upgrade and async-function cases and the placement mutants
(fold revision, PM 502). This memo is also the home of the P0 / L3 schedule (§1) and of the P0 program's
landing list (§7, PM 513 (d)).

## §0. Charter

### §0.1 Why (PM 502)

Guard plan-review R1 and R2 each found, one site at a time, a consumer that takes an `Err` from a
probe-bearing call and turns it into a definitive fact: custom-element upgrade marks the element Failed
(R1); caller-side state mutated before `with_call_mode` (R1); the lazy event-handler compile maps every
`Err` to `None` and its caller removes the listener (R2 S-1); no mutant pins probe placement (R2 S-2). The
common root is the absence of a contract for the guard's error and of a census of its consumers. P0 states
the contract once (§4), gives the error an identity that consumers can test (P0a-1 §4), enumerates every
consumer by a property (§5; the check, P0a-2a §5), fixes every consumer that violates the contract (P0a-1 §5,
P0b-VM-1, P0b-VM-2, P0b-L3), keeps the census honest with a tracked check whose unknown case is red, and pins each fix
and each choke-point placement with a cell and a mutant (in each slice).

R3 found one more consumer of the same kind, at a crate boundary: the session crate's UA dispatch loop
consumes a `once` listener before `ScriptEngine::call_listener`, which returns `()` (P0a-1 §4.5, D-14). R1, R2
and R3 each found one; PM 533's diagnosis is that the census population was defined by a type vocabulary
(`Err` types inside `elidex-js`) instead of by the property under test. §5.1 now defines it by the
property: every place where a fallible result loses its kind, including at a boundary.

R4 (at `d7ec4761`) found the fourth: the `unhandledrejection` loop consumes a `once` listener before a call
whose result is dropped (`natives_promise.rs:565` / `:591`), and the census had admitted the site by hand
("nothing written, top-only"; both false). R4 also showed that "did not start" was inferred on the error
side (inside `throw_error`, which only a script frame reaches), so a native callback that had run could still
come out as not started. R3's and R4's terminators fired on the same mechanism; PM 544 re-sliced P0a along the
two roots: **P0a-1** decides "did it start" with a per-entry entered mark owned by the entry (P0a-1 §4.2) and
fixes every site of D-14's shape (D-14, D-15); **P0a-2** (since PM 609 P0a-2a / P0a-2b) checks every admission reason mechanically, so a row
the checker cannot verify is a D row (P0a-2a §5 rule 12).

R5 (the P0a-1 plan-review at `34a6ae8f`, Axis 2) found two effects outside P0a-1's hand-enumerated list of
"first effect" sites — the async start (no frame is pushed) and the wasm adapter's argument coercion (user
`valueOf` before the adapter's own check) — each leaving an entry unentered, i.e. `NotStarted`, i.e. revert
and re-run. The same class as R4's IMP-1. PM 557: the mark is set **by structure**, right after the entry's
own check, inside the one wrapper every entry is written with; `NotStarted` means exactly "this entry's own
check failed", everything after the check is `Threw` (P0a-1 §4.2; the C-4 reading is narrowed to match,
§0.4).

The re-review of that revision (`b64546fc`, Axis 2 IMP-1) found that the revert was exact per entry but
applied per step: a step whose first entry ran user code and whose second entry failed its own check was
still reverted and re-run, and the entry-to-step mapping was a hand list (it missed the custom-element
upgrade's `prototype` read). PM 590 (the user's choice, option β): **no revert anywhere** — a probe failure
is a throw completion of the step it interrupts, every consumer takes the spec's exception branch, and the
revert API, `NotStarted`, the entered mark and the escape rule are gone (§4 C-4; P0a-1 §4.2 "What β
removed"). The same review's IMP-2 (internal promise reactions drop a failed handler call) is fixed by P0a-1
as D-16 under any option (PM 590).

### §0.2 What P0 owns, and what it does not

P0 owns: the contract C-1…C-5 (§4); the error identity types and the one check site; the
engine boundary's failure type `ScriptFailure` (NEW: P0a-1) and the trait changes that carry it (P0a-1 §4.5);
the **placement** of the four choke-point checks CP-1, CP-2, CP-3a, CP-3b, i.e. the VM half of umbrella I-3;
the Err-consumer census and its tracked check (P0a-2a / P0a-2b); the fix of every definitive consumer; their cells and
mutants. The split of these among the slices is §1.

P0 does not own: the primitive `crate::stack_guard` (NEW: PR-1), the reserve, the per-thread bound, the walker probes,
the compile-time probes, the caps, the regexp front door (PR-1); `MAX_WASM_STACK` and the CP-3 ask (PR-1b);
the frame cap (PR-2). Those PRs put their probe **inside** P0a-1's check (P0a-1 §4.3) and produce the error
P0 defines.

### §0.3 Order and why P0 is first

PR-1's probes are the first production producers of a resource-limit `Err`; the definitive consumers
become live failures the moment a producer exists. So every definitive consumer is fixed before PR-1
lands. At P0's landing no production code produces the error: the cells produce it through P0a-1's
test-only injection (P0a-1 §4.4), and nothing in production depends on a step that is not yet verified.
The one rule whose input does not exist before PR-1 (the compile-channel aggregation, P0a-1 §4.1 item 4)
is stated in P0a-1 and implemented with its first producer in PR-1, with PR-1's cell.

### §0.4 PM rulings

- **Split (PM 509 item 1).** This memo is the P0 umbrella; the slices are §1's. Each P0b PR is a narrow
  slice that only removes rows from the census `# pending` section; under this umbrella each is a base
  case (c) of the edge-dense rule (CLAUDE.md, Design discipline).
- **Owners (PM 509 item 2).** D-1…D-5 → L3 (P0b-L3); D-6…D-12 → VM P4 (P0b-VM-1: D-6, D-7, D-10, D-11;
  P0b-VM-2: D-8, D-9, D-12, PM 513 (a)); D-13, B-1, B-2 → VM P4 (P0a, now P0a-1).
- **Step 3.7's not-parsable arm (PM 509 item 3)** is in P0b-L3's scope (§6 item 4).
- **Slots (PM 509 item 4).** §6 records each home; the ledger writes are rows of §7.
- **C-4 reading ratified (PM 509 item 5).** Spec-retried (re-entry) subjects are reverted; the rest take
  the spec's exception branch with the `RangeError`. Narrowed by PM 557 (below); superseded by PM 590
  (below): no subject is reverted (§4 C-4, D-P0-4).
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
  **Ratified driver decisions**: the census rule "a `?` inside a loop body is a member" (P0a-2a §5 rule 8;
  retired by PM 598, replaced by property: P0a-2a §5 rule 13) and the single test-failure seam (L3's `fail_next_abort_dispatch` moved onto P0a-1's injection, P0a-1 §4.4).
- **PM 533 (R3 Axis 1 IMP-1 / IMP-2).** (a) **Fix**: the `ScriptEngine` boundary, and every other
  engine-facing boundary, keeps the failure kind: a method that returns `()` / `Option` / a value with the
  failure collapsed returns a result that carries it, and the engine-independent caller applies the
  contract (for the `once` case, C-4: put the entry back; superseded by PM 590 (below): nothing is put
  back, the removal stands, §4 C-4). The type is engine-independent; the trait change
  is in P0a-1's scope (P0a-1 §4.5: `ScriptFailure`, D-14, BC-1…BC-3). (b) **Rejected**: moving the `once`
  removal after the call (DOM §2.9 *inner invoke* removes before the call, step 2.5 before 2.13). (c)
  **Rejected**: a top-only N row for `call_listener` (a collapsing boundary is the defect itself; watching it
  leaves the defect in place). (2) **Census population by property**: every place where a fallible result
  loses its kind at a boundary (a trait method, a callback, an API returning `Option` / `()` / a
  non-kind-bearing value) is a member; this replaces "Err types inside `elidex-js`" (§5.1, P0a-2a §5 rules
  9–11, re-derived at `02386ec9`, P0a-2a §7.1). PM 533's diagnosis — R1, R2 and R3 each found one more consumer
  because the population was a type vocabulary, not the property (§0.1) — is recorded with (2). (i)
  **Procedure** (corrected per PM 540: (i) is a decision about how to proceed, not the diagnosis): the
  remaining four axes of R3 are not run now (they would report on text that is about to change); the
  designer revises P0 and P0a, then R4 runs all five axes on P0 and P0a, one axis at a time; R4 stops if one
  more structural IMP of the same mechanism appears (then the PM re-draws the slices). (iii)
  IMP-2 (the HTML §8.1.8.1 step-3 write decisions stay engine-independent in the session crate, not in
  `vm/host`) is P0b-L3's; the reviewer's completion API is passed on as a proposal and L3 decides its shape
  in P0b-L3's plan (`H/p0b-l3-handoff.md` §4); its failure half is P0a-1's `ScriptFailure` (one type).
- **PM 540 (R3/R4 revision at `d7ec4761`).** (1) The landing table (§7) is re-registered after R4
  converges, from the head's §7 with `<path>:<lines> @ <sha>` cells. (2) **Order**: P0a-1 lands before
  `#11-event-path-window-missing`'s PR-A; `call_listener`'s type is the boundary's contract, so the later PR
  adapts; PR-A's mock `ScriptEngine` is written against P0a-1's signature; if P0a-1 is late, the PM decides
  (§1). (3) The 12 drains kept as `()` R rows must each name the consumer of the inner failure (its report
  path); a row that cannot is a D row (P0a-2a §5 rule 12 `drain-consumed`; P0a-1 §4.5 moves
  `set_history_state` out of the drains, R4 Axis 1 MIN-3). (4) The session-crate / `elidex-dom-api` site
  list goes to L3 via the PM (§7 L-14). (5) Slot `#11-event-dispatch-inner-invoke-dual-implementation`
  registered (owner VM P4, re-eval 2026-12-15; out of P0a's scope): the *inner invoke* loop is implemented
  more than once; P0a-1 §4.5 cites it and counts three loops (the slot text was corrected to three, PM 550).
- **PM 544 (R4 at `d7ec4761`: Axis 1 CRIT 0 / IMP 0 / MIN 3; Axis 2 CRIT 0 / IMP 3 (one uncertain) / MIN 3;
  the R3 terminator fired on Axis 2).** IMP-2: `natives_promise.rs:565` / `:591` is D-14's shape, admitted by
  hand as "S: nothing written / top-only", both false (D-15). IMP-1: `NotStarted` did not guarantee "nothing
  ran" (the kind rewrite lived only in `throw_error`, so a nested failure after a native callback had run
  was `NotStarted`, and the once-reinsert would re-run a half-run step). IMP-3 (uncertain): rule 10's cut
  was name × name, fail-unsafe. **Ruling**: re-slice P0a into **P0a-1** (contract semantics: C-1 decided
  by a per-entry entered mark, `ScriptFailure`, the boundary traits, the escape rule, every site of D-14's
  shape) and **P0a-2** (the census checker: every admission reason checked mechanically; an unverifiable
  row is D), each with its own plan-review, in that order; P0b and later are unchanged. Receivers: IMP-1
  → P0a-1; IMP-2's site fix → P0a-1, its admission mechanism → P0a-2; IMP-3 → P0a-2; the R4 MINs are
  assigned in each memo. Grounds: Security by structure ("did it run" is a fact of the side that received
  the entry, not an inference from the error path); the fail-safe direction
  (`memory/feedback_fail-safe-direction-before-list-completeness.md`). The designer of the split memos is
  raised to Fable (global rule 5: two consecutive terminators on one mechanism).
- **CE bundle order (PM, 11-08).** Where P0b-L3 overlaps `custom_elements/upgrade.rs`, P0b-L3 stays limited to
  the P0 contract; L3's CE bundle (TypeError, read-once, `#11-ce-upgrade-reaction-enqueue-order`) rebases
  onto it afterwards.
- **PM 557 (R5: the P0a-1 plan-review at `34a6ae8f`; Axis 1 CRIT 0 / IMP 0 / MIN 3; Axis 2 CRIT 0 / IMP 2 /
  MIN 2, terminator on IMP-1). Superseded by PM 590 (below): the mark, the entry wrapper and `NotStarted`
  are gone, nothing is reverted (§4 C-4; P0a-1 §4.2 "What β removed"); kept as history.** IMP-1: the async start consumes a once listener whose step never ran
  (`interpreter.rs:630-660` pushes no frame; `natives_generator.rs:520-522` rejects the wrapper and the
  call returns `Ok`). IMP-2: the wasm adapter runs user `valueOf` in its argument coercion before its own
  check (`exported_func.rs:284-289`), so a later failure was `NotStarted` → revert → the listener runs
  twice. Root: the mark was set at a hand-enumerated list of first-effect sites; an effect not in the list
  fell to `NotStarted`, the unsafe side. **Ruling, option (A)**: (1) the mark is set by structure, right
  after the entry's own check, inside the one entry wrapper (Rev 2's wrapper `enter`, never implemented and
  deleted by PM 590);
  `NotStarted` means exactly "this entry's own check failed"; every other failure after the check is
  `Threw`, including a nested entry's check failing before any effect of the outer entry. (2) Lens: the
  spec is silent on resource limits → elidex's principles decide: fail-safe direction (an unknown effect
  falls to a visible `Threw`, not to a silent double execution) and Security by structure (the mark's
  position is decided by structure, not by a list's completeness). (B) enumerate more sites and (C) a
  local fix at the async start are rejected: each adds "the next site", the shape R3 / R4 / R5 took.
  (3) PM 509 item 5's C-4 reading is narrowed: "spec-retried subjects are reverted" applies only when
  the entry's own check failed (§4 C-4). (4) An entry may opt in to a finer `NotStarted` only with a
  rule-12 reason code P0a-2's (now P0a-2a's) checker verifies (P0a-1 §4.2; none added). (5) Everything that rested on the
  old mark is re-derived in P0a-1 Rev 2 (the once revert, D-P0-2, M-D14b, §4.2's table, §4.6, I-5, the
  async start and resume paths, the wasm coercion). The async start under (A): the call's check passed, the
  nested CP-2 failure rejects the wrapper, the call returns the rejected wrapper (ECMA-262
  EvaluateAsyncFunctionBody step 5; AsyncBlockStart step 1.h), the once listener is consumed and the
  `RangeError` reaches *notify about rejected promises* — §6 item 5's R row covers the start path too.
- **PM 590 (P0a-1 Rev 2 re-review at `b64546fc`; terminator on Axis 2 IMP-1; the user chose β).** No revert anywhere: every probe failure is `Threw`, consumed and reported as a `RangeError` (§4 C-4); IMP-2 is mandatory under any option (P0a-1 §5.5, D-16).
- **PM 598 / PM 603 (P0a-1 Rev 3b re-review).** Rule 8 `loop-no-write-no-drain` is deleted and replaced
  by property: RL-only propagation (β′) is P0a-2a §5 rule 13; for report-and-continue loops (α), which are
  not specific to the resource limit, P0a-2 proposes a separate slot (P0a-2a §9; approved by PM 608). The Err population's reach is closed over every
  function, not only carrier-typed ones (§5.1); the five `dispatch_vm_simple_event` drops are its positive
  control. The D-16 sink reports when the owner is already final (P0a-1 §5.5). Durable facts derived from
  the error are C-4's, under one name (§4).
- **PM 608 (P0a-1 Rev 3c re-review).** (1) The census population is P0a-2's, not P0a-1's: P0a-1's
  ratification does not wait on it (P0a-1 §1). The 574 members only PM 603's reach admits are classified
  before the allowlist's review, measured again at the then-current main, with the resolved call graph
  built as the first part of the checker itself; until then the unclassified rows count as pending
  (P0a-2b §6). (2) The (α) slot `#11-report-and-continue-loop-census` is approved (owner VM P4; its
  Re-eval: §7 L-15, the one home); P0a-1's PR adds the row to the VM-P4 slot ledger; the PMO registers it
  (§7 L-15). (3) The drops of the
  `dispatch_vm_simple_event` shape go to P0b-VM-2 (P0b-VM-2 §0 gives the reason). (4) D-6's script-throw
  half stays `#11-history-state-structured-serialize-fidelity`'s.
- **PM 609 (after Axis 3 on Rev 3c).** P0a-2 is split into **P0a-2a** (the rules, the resolved call
  graph, rule 13, the controls and mutants, a measured population report) and **P0a-2b** (the allowlist
  and `# pending`, built from the classification of the population), each with its own plan-review; this
  replaces PM 544's single P0a-2 slice. Reason (P0a-2a header "The split"): PM 608 fixed the order
  substrate → classification → ratification of the allowlist; the split maps it onto slices and takes the
  data-dependent part out of the substrate's review (the edge-dense multi-PR rule). The classification runs
  after P0a-2a lands, with P0a-2a's checker, and is P0a-2b's review precondition. The (α) slot triggers on
  P0a-2a's landing (P0a-2a §9). P0b-VM-2 owns the whole UA-fire drop class (narrowed by PM 615, below: two members are P0b-VM-1's), with its conditions (P0b-VM-2 §0:
  `indexeddb/dispatch.rs:194` first; a P0b-VM-3 cut proposed to the PM past 15 files or one review round).
- **PM 615 (after Axes 4 and 5 on Rev 3d).** (1) **File overlap.** The two UA-fire drop class members in
  P0b-VM-1's files (`history_events.rs:187`, `worker_scope.rs:333`) are fixed by the slice that owns the
  file, P0b-VM-1 (P0b-VM-1 §5.3). The class's fix shape is defined once, in §4 C-3 ("The UA-fire drop
  class"), as contract prose, not in P0b-VM-2: a shape P0b-VM-2 introduced as code would make P0b-VM-1
  depend on it, the landing-order edge PM 615 excludes. Each slice implements it at its own sites, so no
  shared helper and no edge exists (§1). P0b-VM-2 §10 item 4 is closed. (2) The new slot's Re-eval ("at
  registration" in the repo row, the date at §7 L-15 only) is accepted; the repo row and the slice memos
  point to L-15 (Axis 5 MIN-3). (3) **P0b-L3's date**: the plan
  starts after P0a-2b lands, target P0a-2b's landing + 7 days, a relative date (§1 Schedule; "around 11-12"
  withdrawn).

### §0.5 Decisions

- **D-P0-1 Identity by kind, not by message.** A resource-limit error is a distinct `VmErrorKind`
  (P0a-1 §4.1). A message compare or a `RangeError` kind cannot be told apart from a script's own
  `RangeError` (`new Array(-1)`).
- **D-P0-2 A started step is never re-run (ratified, PM 509 item 7; under β, PM 590, it holds for every
  step).** A custom-element constructor that started and then ran out of stack takes the threw branch,
  because its side effects have already happened. Under β no step is reverted or re-run by the contract,
  whether or not anything of it ran: the error keeps its kind (P0a-1 §4.2) and the consumer takes the
  step's exception branch. No hidden marker on the error object distinguishes "did not start".
- **D-P0-3 One choke-point check, placed by P0a-1, filled by PR-1 / PR-1b** (P0a-1 §4.3).
- **D-P0-4 Every subject takes the spec's exception branch; nothing is reverted** (C-4, §4; PM 590,
  superseding PM 509 item 5's revert of re-entry subjects).
- **D-P0-5 The census is a tracked check in the `tools` job**, with the population derived from the code,
  an unknown member red and every admission reason checked by a predicate (P0a-2a §5; PM 544). Not
  `trip-wires`: the check needs an interpreter (CLAUDE.md §CI job boundary).
- **D-P0-6 Caps are not resource limits.** `MAX_JSON_DEPTH`, `MAX_NESTING_DEPTH`, `MAX_REGEXP_DEPTH`,
  `DENSE_ARRAY_LEN_LIMIT` and the like are answers determined by the input (umbrella I-4); the same input
  fails the same way at any depth. The contract covers only the probe-originated error.

## §1. Slices, edges and schedule (canonical)

| Slice | Content | Owner | Memo | Census `# pending` effect |
|---|---|---|---|---|
| P0a-1 | Contract semantics (VM core and the engine boundary): error kinds, `STACK_EXHAUSTED_MSG`, the one check site `resource_limit::check_entry` (NEW: P0a-1) and CP placement, B-1, B-2, D-13, the engine boundary's `ScriptFailure` with D-14 and BC-1…BC-3 (PM 533), D-15 (the `unhandledrejection` once loop, PM 544), D-16 (the internal-reaction sink, PM 590), test-injection seams (including L3's abort-dispatch seam, moved), placement / identity / sink cells and mutants | VM P4 | [P0a-1](2026-11-vm-stack-safety-guard-p0a1.md) | none (its fixes never enter the section) |
| P0a-2a | The census checker: the shared resolved call graph, the rules with every admission reason checked by a predicate (rule 12), rule 13, controls, mutants, the measured population report (PM 609) | VM P4 | [P0a-2a](2026-11-vm-stack-safety-guard-p0a2a.md) | none (it ships the rules over the section, not the section) |
| *(classification)* | Every member of the population that P0a-1's and P0a-2a's rows do not admit gets a class and a reason code, with P0a-2a's checker at the then-current main (PM 608, PM 609); not a PR | VM P4 | [P0a-2b](2026-11-vm-stack-safety-guard-p0a2b.md) §6 | decides the D rows the section will hold |
| P0a-2b | The allowlist with `# pending` (D-1, D-2, D-5…D-12, plus the D rows the classification finds; D-3 / D-4 leave under β as P rows), the gate on the tree, legibility rewrites (PM 609) | VM P4 | [P0a-2b](2026-11-vm-stack-safety-guard-p0a2b.md) | creates the section (P0a-2b §7.3) |
| P0b-VM-1 | D-6, D-7 (history; L4 files, no driver), D-10, D-11 (worker), and the two UA-fire drop class members in its files (PM 615); WHATWG HTML only | VM P4 | [P0b-VM-1](2026-11-vm-stack-safety-guard-p0b-vm1.md) | removes its 4 rows |
| P0b-VM-2 | D-8 (IndexedDB), D-9 (SW `startMessages()`, made asynchronous), D-12 (WebCrypto), the UA-fire drop class except its two members in P0b-VM-1's files (PM 608, PM 609, PM 615) | VM P4 | [P0b-VM-2](2026-11-vm-stack-safety-guard-p0b-vm2.md) | removes its 4 rows |
| P0b-L3 | D-1, D-2, D-5 and HTML §8.1.8.1 step 3.7's not-parsable arm (D-3 / D-4 are no longer definitive under β, P0a-2b §7.3) | L3 | L3's own plan; P0's input is `H/p0b-l3-handoff.md` | removes its 3 D rows |

**Edges.** P0a-1 → P0a-2a (the checker is written against P0a-1's contract); P0a-2a → classification →
P0a-2b (the classification runs P0a-2a's landed checker, and P0a-2b's review waits for it, PM 609;
P0a-2b's allowlist records P0a-1's fixes); P0a-2b → P0b-VM-1; P0a-2b → P0b-VM-2; P0a-2b → P0b-L3 (each P0b
slice edits the allowlist and needs P0a-1's API through P0a-2b: P0b-L3 uses `ScriptFailure` and the
injection of P0a-1 §4.4; P0b-VM-1 / P0b-VM-2 use the predicates and the injection); P0b-VM-1 → PR-1;
P0b-VM-2 → PR-1; P0b-L3 → PR-1. The three P0b slices are independent of each other: each touches only its
own rows of `# pending`, and their files are disjoint, except `history_events.rs` and `worker_scope.rs`,
where P0b-VM-1 fixes the two UA-fire drop class members as the file's owner (PM 615); the class's fix shape
is contract prose (§4 C-3), not a shared helper, so no edge between P0b-VM-1 and P0b-VM-2 exists and the
two land in either order. This is the one statement of the overlap; the slice memos point here. L3's `#11-abort-signal-any-dependent-flattening` PR → P0a-1 (the test
seam) and → P0a-2b (the allowlist row of its abort loop) and → P0b-L3: P0a-1 is rebased on it. What each order means is stated once, in P0a-1 §4.4
(the seam) and P0a-2b §7.3 (the row); this memo does not restate it (PM 538). P0b-L3 → L3's CE bundle (§0.4,
CE bundle order).
P0a-1 → `#11-event-path-window-missing` PR-A (PM 540 (2)): PR-A's mock `ScriptEngine`
(`event_dispatch/test_support.rs`) and its edits to `script_dispatch_event_core` are written against
`call_listener -> Result<(), ScriptFailure>` (P0a-1 §4.5; no revert under β); if P0a-1 is late for
PR-A's schedule, the PM decides (P0a-1 §9 item 5).
**PR-1's precondition** is an empty `# pending` section (§7 L-1). Every other memo of the program points
here for edges and dates and does not restate them.

**Schedule (the one home of the P0 / L3 dates; L3 estimates via PM, 11-08).**

- L3's `#11-abort-signal-any-dependent-flattening` PR: a separate small PR, not part of L3's walker
  umbrella; L3 starts it right after #607 Rev 4 (plan and plan-review 11-09, its plan Rev 2.1 on 11-09;
  implementation and pre-push 11-09 to 11-10; PR around 11-10; landing expected 11-10 to 11-11 after Codex
  review). Surfaces: `abort_statics.rs` and a new engine-independent DOM §3.2 "create a dependent abort
  signal". It is also the hard edge W-11 slot → PR-1 (umbrella §4).
- P0a-1: first of the P0 slices, after that PR; then P0a-2a, the classification, P0a-2b (each slice with
  its own plan-review, PM 544, PM 609).
  P0b-VM-1 and P0b-VM-2: after P0a-2b, in either order.
- P0b-L3: its plan starts after P0a-2b lands (and after that PR); target: P0a-2b's landing + 7 days, a
  relative date (PM 615); with a per-PR plan-review.
- L3's `#11-dom-walkers-iterative-over-ecs-links` umbrella (with the capped-walker slot): start ≈11-11,
  slice 1 (`elidex-ecs` `tree/walkers.rs` + the capped walker's 8 callers) landing ≈11-14, the whole
  umbrella ≈end of November, i.e. after PR-1 (the soft edge PR-1 → that slot, umbrella §1.2).

## §2. Coupled invariants

P0 is edge-dense: the contract's five invariants intersect each other and two of PR-1's (I-1 coverage,
whose probes are the producers; I-6 one error shape per channel, umbrella §2 / PR-1 §2). Each slice memo
enumerates the pairs of its own elements in its §2.

| Pair | Intersection | Pinned by | Basis |
|---|---|---|---|
| C-1 × C-2 | C-2's predicate is `is_resource_limit()`; a consumer cannot apply C-2 to an error it cannot recognise, so C-1 must hold at every boundary the error crosses inside the engine (`Vm::eval`, `native_import_scripts`, the CP-3 conversions) and nothing rewrites it on the way (P0a-1 §4.2); the engine-facing traits carry the failure as `ScriptFailure` after the engine surfaced it (P0a-1 §4.5) | P0a-1 `rl_vm_eval_compile_resource_limit_kind`, `rl_cp3b_instantiate_entry_failure`, `rl_nested_failure_keeps_kind`, `rl_eval_result_outcome_kind`; P0b-VM-1 `rl_import_scripts_compile_resource_limit_is_range_error` | cell |
| C-1 × C-3 | Surfacing materialises the error (a thrown or rejected `RangeError`); after materialisation it is no longer C-1's kind, so C-3 is the last place C-1 applies | P0a-1 `rl_nested_failure_keeps_kind`, `rl_cp1_entry_failure_no_callee_effect` | cell |
| C-1 × C-4 | Under β the kind decides no revert: every consumer takes the step's exception branch whether or not anything of the step ran, so no "did it start" fact exists to get wrong; a half-run step is not re-run (D-P0-2) | P0a-1 `rl_bound_native_once_listener_consumed_not_rerun`, `rl_ua_dispatch_once_listener_entry_failure_consumed`; P0b-L3 `rl_ce_upgrade_after_constructor_started_takes_threw_branch` | cell |
| C-1 × C-5 | The census population is "can receive C-1's kind"; its producers are derived from the code (fns that call the check) plus a planned list that only shrinks (P0a-2a §6) | P0a-2a census controls | check |
| C-2 × C-3 | C-2 forbids, C-3 prescribes; together every consumer has exactly one admissible shape per subject class (§5.2 classes P/T/R; D is the forbidden one), and each admissible shape is a checked reason (P0a-2a §5 rule 12) | §5.2 class table; P0a-2a rule 12 | structural |
| C-2 × C-4 | C-2 forbids converting the error (a success value, another class); C-4 says which writes stand (the spec's own writes for the step's exception branch) and forbids every other durable fact derived from the error. A write the spec makes before the step is not derived from the error | D-3 / D-4 (P0a-2b §7.3: the threw branch, no longer D) vs D-1 / D-2 (P0b-L3: a listener removal the spec never makes) | structural |
| C-2 × C-5 | A D row is not admissible in the allowlist (after the P0b slices; before them only in `# pending`); a row admitted under a reason the check cannot verify is a D row (PM 544) | P0a-2a controls `census_rejects_definitive_row`, `census_rejects_failed_reason` | check |
| C-3 × C-4 | Taking the exception branch must surface the error on the trigger's channel, or the branch is a silent drop | P0a-1 D-14 / D-15 / D-16 cells; P0b-L3 D-5's report | cell |
| C-3 × C-5 | The census classifies the channel each row uses; a row that drops the error is class S, admissible but recorded (§6 item 2) only when it is top-only, and top-only is checked, not asserted (D-14, D-15, D-16 failed it) | P0a-2b §7.3 S rows; rule 12 `dropped-top-only` | check |
| C-4 × C-5 | Under β a write before the call is the spec's own and stands on every outcome, so the census has no W population (removed, PM 590); what C-4 needs from the census is that the error is not dropped, which the Err and boundary populations check, and, on an RL-only path, that what an early exit abandons is pinned by a cell | P0a-2a "The W retirement" (rule 7 deleted, PM 595), rules 12 and 13 (rule 8 deleted, PM 598) | check |
| C-1 × I-1 | PR-1 / PR-1b's probes are C-1's producers: the choke-point probes sit inside P0a-1's check, and every walker / front-end / front-door probe converts through `From<StackExhausted>` to `VmError::resource_limit` (NEW: P0a-1) or `CompileErrorKind::ResourceLimit`, so every probe failure has C-1's kind by construction | P0a-1 §4.3; PR-1 §4.1 | structural |
| C-2 × I-1 | Before a producer exists a definitive consumer is latent; I-1 makes it live, which is the order of §0.3 | §7 L-1 | ledger |
| C-3 × I-1 | Surfacing runs inside the reserve: building a `RangeError` and reporting it must fit in `WALKER_RESERVE` (PR-1 §4.3) | PR-1 `stack_guard_reserve_margin_ratchet` | PR-1 cell |
| C-4 × I-1 | Taking an exception branch at the floor runs no recursion of its own (a report, a rejection, a state set) | P0a-1 §5.5 (the sinks run no script); PR-1 `stack_guard_reserve_margin_ratchet` | structural |
| C-5 × I-1 | Each PR-1 probe site that lands removes its name from the planned-producer list (P0a-2a §6); the derived producer set then covers it | P0a-2a control `census_planned_producer_already_derived` | check |
| C-1 × I-6 | I-6's runtime message is `STACK_EXHAUSTED_MSG` (NEW: P0a-1, PM 421 (a)); the kind is P0's, the script-visible class stays `RangeError` | P0a-1 `rl_identity_predicates` | cell |
| C-2 × I-6 | Reclassification (C-2 (b)) is the consumer-side breach of I-6; the D-cls rows are exactly the sites that would break I-6 | P0b-VM-1 D-10 / D-11 and P0b-VM-2 D-12 cells; P0a-1 D-13 cell | cell |
| C-3 × I-6 | The channel changes the form (throw / rejection / report), never the class | rejection rows keep `vm_error_to_thrown` | structural |
| C-4 × I-6 | Taking a spec branch with the `RangeError` keeps the class: a stream errored with it, a promise rejected with it, a listener's report | P0a-2b §7.3 R rows | structural |
| C-5 × I-6 | The census does not check classes; the D-cls cells do | — | cell |

## §3. Spec coverage map

No spec defines a resource limit. This memo implements no algorithm; the rows are the clauses the
contract itself rests on (§4 Grounds and C-4's exception branches). The algorithms a fix touches are in
each slice's §3. Every number is from `.claude/tools/webref` (commands at the end of §6).

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| ECMA-262 §17 Error Handling and Language Extensions | paragraph 1 (errors reported when the construct is evaluated), paragraph 3 (no other kinds of errors as early errors) | a resource limit is neither an early error nor a specified error; it surfaces where the construct is evaluated | contract C-1, C-4 (no code here) | ✓ | yes |
| WHATWG HTML §8.1.8.1 Event handlers | *get the current value of the event handler* step 3 (internal raw uncompiled handler; written only at step 3.7.1 for a parse error and at step 3.12 on success) | C-4: the spec writes nothing for a failed compile that is not a parse error, so nothing is written | contract C-4 (no code here) | ✓ | yes |
| WHATWG HTML §4.13.5 Upgrades | *upgrade an element* step 3 (state "failed" before the construct) and step 11's exception branch | C-4: the state set at step 3 stands; the threw branch is taken | contract C-4 (no code here) | ✓ | no |
| WHATWG DOM §2.9 Dispatching events | *inner invoke* step 2 (each listener whose removed is false), 2.5 (once: removed before the call), 2.13 (call; report on a throw) | C-4: the removal of 2.5 stands; 2.13.1 reports | contract C-4 (no code here) | ✓ | yes |
| WHATWG HTML §8.1.4.6 Runtime script errors | *report an exception* steps 1–7 | the report channel of C-3; elidex's stand-in is stderr (`#11-embedder-diagnostic-channel`) | contract C-3 (no code here) | ✓ | no |

**Breadth**: K=3 specs (ECMA-262, WHATWG HTML, WHATWG DOM), M=5 entries (verified 2026-11-08 by `python3 .claude/skills/elidex-plan-review/preflight.py` on this memo: rc 0, K=3, M=5, "ok (single PR scope)"); per-slice breadth is each
slice's §3 (§7 L-13 records them).

### §3.1 Cited, not touched

- ECMA-262 §7.4.11 IteratorClose step 5 (a throw completion wins over the `return` method's throw): the
  E rows of P0a-2b §7.3 implement the opposite precedence (§6 item 1).
- WHATWG HTML §10.2.4 *run a worker*: a top-level exception is reported and the worker keeps running;
  `worker_thread.rs:200` closes the worker (§6 item 3).
- WHATWG Streams: an exception from `pull` / `start` errors the stream (P0a-2b §7.3 R rows, C-4 non-re-entry).

## §4. The contract (canonical)

**C-1 Identity.** The guard's error is `VmErrorKind::ResourceLimit` (NEW: P0a-1). It is produced only by
the choke-point check (P0a-1 §4.3) and, after PR-1, by the walker and front-end probes and the regexp front
door (PR-1 §5–§6). It means: a probe found too little stack for the step it guards, and the step was
interrupted there (a throw completion, C-4). It says nothing about how much of the step ran, and nothing
rewrites it: the kind travels until a consumer surfaces it (C-3; P0a-1 §4.2). At the compile
boundary it is `CompileErrorKind::ResourceLimit` (NEW: P0a-1). Every boundary that converts the error keeps
the kind (`Vm::eval`, `native_import_scripts`, the CP-3 conversions of P0a-1 §4.3). At an engine-facing
boundary into an engine-independent crate (the `ScriptEngine` methods, `FilterAction::accept`) the engine
surfaces the error first and the boundary returns `Err(ScriptFailure)` (NEW: P0a-1; P0a-1 §4.5): no such
boundary returns `()`, `Option` or a value in which the failure is collapsed.

**C-2 No conversion.** No consumer turns a C-1 error into
(a) a success value: no `.ok()`, `unwrap_or(..)`, `_ => None` / default, no fallback that answers in its
place;
(b) another error class: no `DataCloneError`, `TypeError`, `NotSupportedError`, `CompileError` string in its
place.
(Durable facts derived from the error are C-4's, "what the branch does not write", PM 598.)

**C-3 Surfacing.** The error leaves the consumer by the trigger's own channel: `?`; a throw into the
running script (`throw_error`); rejection of the promise or request the trigger returned; or a report
(HTML *report an exception*, whose stand-in is stderr until `#11-embedder-diagnostic-channel`). The
script-visible class is `RangeError` with `STACK_EXHAUSTED_MSG` (NEW: P0a-1; "Maximum call stack size
exceeded", PM 421 (a)).

*The UA-fire drop class (PM 608, PM 615; the one definition of its fix shape).* A UA-fired event's
dispatch whose `Result` is dropped (`let _ =`, an unused binding) or converted without a report is fixed
by this shape: the site that receives the `Result` reports an `Err` once, through the report channel above
(the stderr stand-in, as `observer_common.rs` and P0a-1's `call_listener` use it), and the UA algorithm then continues exactly as after an
ordinary listener throw; a site that maps the `Err` to a fact the UA algorithm reads (`threw: true` at
`indexeddb/dispatch.rs:194`) keeps the mapping and reports first. The shape is contract prose, not a shared
helper: each slice implements it at the members in its own files (P0b-VM-2 §1; the two in P0b-VM-1's
files, P0b-VM-1 §5.3), so the slices share no code and need no edge (§1).

**C-4 Spec exception branches (option β, PM 590; supersedes PM 509 item 5 and PM 557's narrowing).** A
probe failure is a throw completion of the step it interrupts. Every consumer takes the spec's exception
branch for that step with the `RangeError` and surfaces it on the trigger's channel (C-3): a once listener
removed before its call (DOM §2.9 *inner invoke* step 2.5) stays removed and the error is reported (step
2.13.1); a custom element whose state was set to "failed" before its construct (HTML §4.13.5 *upgrade an
element* step 3) takes step 11's exception branch; an async function's promise is rejected (AsyncBlockStart
step 1.h); a promise, a stream, an IndexedDB request or transaction, a worker's or service worker's
top-level evaluation take their error channel. **Nothing is reverted, whether or not anything of the step
ran, and the contract retries nothing.** Where the spec writes nothing for the step — a handler's compile
that is not a parse error (HTML §8.1.8.1 *get the current value of the event handler* writes the value
only at step 3.7.1 and step 3.12) — nothing is written, and the spec's own later trigger re-attempts it;
that is the absence of a write, not a revert. This is the one statement of the rule; §0.4 and the slice
memos point here.

*What the branch does not write.* No consumer writes a durable fact about a subject,
derived from the error, that the spec's exception branch for the step does not write: no listener or
handler removal, no null value, no negative cache, no stored record or session-history state derived from
it. *What the branch does not skip.* Where the spec's algorithm reports an iteration's exception and
continues, one iteration's failure does not end the loop: the remaining subjects are processed. The census
checks the second half only where the loss is new under P0 (RL-only propagation, P0a-2a §5 rule 13); a
loop that ends early on any exception is not specific to the resource limit (P0a-2a §5 "(α)").

*Authority (PM 590).* (1) The spec is silent on resource limits: ECMA-262 §17's fourth paragraph permits
two kinds of extension (syntax; additional types, values, objects, properties and functions), neither a
resource limit, and §9.4 gives the execution context stack no bound. (2) So the elidex principles decide:
Security by structure (no fact "did the step start" whose exactness depends on where a mark is set or on a
list of entries per step — R3, R4, R5 and the Rev 2 re-review each found the next such site) and the
fail-safe direction (an unforeseen case falls to a visible, reported failure, never to a silent second
execution of a half-run step). (3) Consistency: a resource limit rides ECMA-262's Completion Record model
(§6.2.4) as a throw completion; DOM *inner invoke* removes a `once` listener before calling it (step
2.5), so the spec itself consumes it on any throw; AsyncBlockStart step 1.h rejects the async function's
promise on a throw completion, the outcome P0a-1 already gives the async start and resume. Accepted cost:
a script that drives another's handler to the stack limit consumes that handler's `once` listener and
fails its custom-element upgrade (Grounds (3) below now covers handlers and caches, not these).

**C-5 Census.** Every consumer a resource-limit error can reach, and every place where its kind can be
lost, is in the census; the population is derived from the code by that property (§5.1); an unknown
member, a stale row, a definitive row and a row whose admission reason the check cannot verify are each
red (PM 544). The check that enforces it is P0a-2a §5.

**Grounds.** (0) C-4's authority chain (above). (1) Spec: §17 paragraph 1 reports an error where its construct is evaluated; paragraph 3
forbids treating a non-early error as an early error, so a compile-time resource limit is not a
SyntaxError and HTML §8.1.8.1 step 3.7 (not parsable) does not apply to it. No spec step produces the
error, so every spec branch that would consume it was written for a different condition (umbrella §3,
PM 424). (2) ECS-native: a component write is a durable fact; reconcile a component only from a
definitive outcome (R2 A2's alternative), never from a transient condition of the Rust stack. (3) Security
by structure: the stack depth at which a page's handler first compiles is under the control of any script
on the page; without C-4's "what the branch does not write" one script can permanently disable another's handlers (a custom element's
upgrade and a `once` listener are consumed as the spec consumes them on any throw: C-4, accepted cost). (4) Fail-safe direction: an unknown consumer is red (C-5), not trusted.

**What the contract does not say.** It does not make a resource limit invisible: the script sees a
`RangeError` at the trigger. It does not retry or revert anything. It does not cover caps (D-P0-6).

**Applying it.** Each fix applies C-2 / C-3 / C-4 at one site, with `is_resource_limit()` as the only
test.

## §5. The census definition

### §5.1 Population (by property; PM 533 (2))

**The property.** A member is every place where a fallible result can lose its kind: a use that does not
propagate it (Err population) and a boundary whose type cannot carry it (boundary population). (The W
population of durable writes before a call existed to find the subjects C-4 reverted; under β nothing is
reverted, a write before the call stands on every outcome, and the population is retired, PM 590; its rule
is deleted by PM 595, for the two reasons in P0a-2a's paragraph "The W retirement".) The type vocabulary
below (carriers, kind-bearing types) is the check's instrument for that property, not its definition;
where the instrument cannot decide, the place is a member (fail-safe, C-5).

**Err population.** Every production use, in a crate that can hold the error, of a call to a *carrier*
where the error is not propagated. A carrier is a function whose return type carries the error
(`Result<_, E>` with `E` ∈ {`VmError`, `CompileError`, `JsValue`, `StackExhausted` (NEW: PR-1),
`RegExpCompileError` (NEW: PR-1), `ScriptFailure` (NEW: P0a-1)}, or a type that carries `ScriptFailure`:
`EvalResult`, `FilterError` after P0a-1 §4.5) **and** that reaches a producer in the call graph, closed
over **every** function, not only carrier-typed ones (PM 603): the population is "can receive a
`ResourceLimit` once PR-1's probes are in", not "returns `Err` today". An edge the graph cannot resolve
goes to every definition, so its call falls in, never out (P0a-2a §5 rules 0 and 1).
Producers: derived from the code (P0a-2a §6: calls of `resource_limit::check_entry` and `ensure_stack`, and
every mention of the kind that is not a predicate), plus the planned list of PR-1 / PR-1b probe
functions until they land (P0a-2a §6). A use is outside the population only when it propagates in a shape
the check recognises (P0a-2a §5 rule 2: `?`, `.map_err(<kind-preserving>)?`, `return` / tail in a carrier
function, a pure throw or `Err(e)` arm); every other use is a member and is classified (§5.2), and a
member may still propagate (class P: a match-arm tail, `return Some(Err(e))`, close-then-return). Crates
(derived, P0a-2a §5 rule 1): `crates/script/elidex-js/src`; the crates that depend on it (the shell); every
crate that names a kind-bearing type or declares a trait `elidex-js` implements
(`elidex-script-session`, `elidex-dom-api`, `elidex-ecs` at `02386ec9`). The call graph is resolved by
definition, not by name (P0a-2a §5 rule 0). A propagating use of a carrier with no ordinary error on its
path (RL-only) is also a member: rule 2's "propagates" assumes an ordinary exception leaves the same state,
and on that path there is none (P0a-2a §5 rule 13).

**Boundary population.** Every place where the kind crosses out of `elidex-js` in a type that cannot carry
it: (B1) a method of a trait declared outside `elidex-js` and implemented in it whose body reaches a
producer and whose return type is not kind-bearing; (B2) a `pub` function of `elidex-js` with those two
properties that another crate calls; (B3) a call from `elidex-js` that passes a closure or a function
pointer to another crate (opaque to the call graph, so always a member). At `ce603c74` the error leaves
`elidex-js` as `EvalResult` (a string), as `()` from `call_listener` and the drains, and as
`FilterError::Throw` (P0a-1 §4.5 enumerates them; P0a-2a §7 counts them).

### §5.2 Classes

| Class | Meaning | Admissible |
|---|---|---|
| P | propagates in a shape the check does not auto-recognise (match-arm tail, `return Some(Err(e))`, close-then-return, a kind-preserving `map_err` closure) | yes, row with the reason |
| T | thrown into the running script (`throw_error`) | yes; the check auto-recognises the pure arm (P0a-2a §5 rule 2), so these need no row |
| R | the trigger's own channel: promise / request rejection, stream error, report (stderr), `EvalResult` | yes, row |
| S | dropped at a depth no script controls (top-only); the report the spec asks for is missing | yes, row, with §6 home |
| E | replaced by another error, both surfaced | yes, row, with §6 home |
| N | not reachable by a script-controlled depth (a fresh thread's top, an embedder entry), or a name-graph collision | yes, row; top-only and the collision cut are checked (P0a-2a §5 rules 5, 10); "argument class" and "unreachable by construction" have no predicate and are not admissible (the code is made legible instead, P0a-2b §7.4) |
| X | test support (panics by design) | yes, row |
| D | definitive: D-state (C-4), D-sub (C-2 a), D-cls (C-2 b); a drop whose admission (S) does not hold (D-14, D-15, D-16); **a row admitted under a reason the check cannot verify** (PM 544) | **no** (only in `# pending` until its P0b slice, P0a-2a §5 rule 4; an unverifiable reason is never pending: it is an instrument or legibility defect fixed in P0a-2b) |

Every admissible row carries a reason code with a predicate (P0a-2a §5 rule 12). The rows, the counts and
the `# pending` composition are P0a-2a §7 (measured) and P0a-2b §6–§7 (classified; the allowlist is P0a-2b's
deliverable).

## §6. Pre-existing deviations seen, and their homes

**Scope.** This section lists the pre-existing spec deviations the census found that P0 does not fix,
each with its home. It is not the list of every slot the program names: the (α) slot
`#11-report-and-continue-loop-census` is P0a-2a §9's (a class the census does not enumerate, §7 L-15), and
`#11-event-dispatch-inner-invoke-dual-implementation` is a layering deviation P0a-1 §5.4 cites (PM 540
(5)); the umbrella's slot table lists them all (umbrella §5, the P0 program's row).

1. **IteratorClose precedence.** ECMA-262 §7.4.11 step 5 returns the original throw completion before
   step 6 considers the `return` method's throw; the ten E rows (P0a-2b §7.3) let the `return` method's
   error win (comments: `dispatch_iter.rs:54-55`, `typed_array_static.rs` at 743, `webidl_sequence.rs` at
   141). Not a C-2 breach in practice; a spec deviation of its own. Home (PM 509 item 4): the existing
   `#11-vm-iteratorclose-precedence-convention`, to which the ten E rows are appended (§7 L-4).
2. **Unreported listener and callback errors.** The two S rows (P0a-2b §7.3: `coerce.rs:246`,
   `flush.rs:70`) drop the error with no report; the stderr stand-in is used elsewhere (`flush.rs`,
   `observer_common.rs`). The other two S rows of the census at `ce603c74`, `engine.rs:275` and
   `natives_promise.rs:591`, were D-14's and D-15's engine halves (PM 533 item 3; PM 544); P0a-1 reports
   there. The VM dispatch loop's listener error (`event_target_dispatch.rs:489`, D-5) is reported by
   P0b-L3. Home: `#11-embedder-diagnostic-channel` (facet, §7 L-3).
3. **A worker's top-level exception closes the worker** (`worker_thread.rs:200-204`); HTML §10.2.4
   reports it and keeps running. Home (PM 509 item 4): `#11-worker-toplevel-error-closes-worker`, owner
   VM P4.
4. **Not-parsable inline handler removes its listener** (`event_handler_attrs.rs:548-552`); HTML §8.1.8.1
   step 3.7's note says the value becomes null and the listener stays. In scope of P0b-L3 (PM 509 item 3):
   value null, listener kept, SyntaxError reported (`H/p0b-l3-handoff.md`).
5. **A failed `AsyncDriverStep` resume** (R2 A5-IMP4): CP-2 fails, the wrapper is rejected
   (`natives_generator.rs:520-525`), the generator stays suspended, a `finally` in the body does not run.
   The row is R (the wrapper is the trigger's channel); P0a-1 pins the outcome
   (`rl_async_step_entry_failure_rejects_wrapper`). Home (PM 502): `#11-vm-async-resume-failure-skips-finally`.
   This item is the program's one statement of the case (PR-1 §2 points here). **The async start (PM 557)**
   meets CP-2 at the same site through `make_async_coroutine_and_drive`: the wrapper is rejected and handed
   out, the once listener that called it is consumed (P0a-1 §4.2 "The async start"). That is the spec's own
   shape (EvaluateAsyncFunctionBody step 5, AsyncBlockStart step 1.h), not a deviation — no `finally` can
   have been entered at the start — so the item's deviation and slot are unchanged; the same R row covers
   both paths, and P0a-1 pins the start (`rl_async_start_entry_failure_rejects_wrapper_once_consumed`).
   **The step's CP-1 (PM 590, D-16)**: a later step can also fail at the reaction job's call of the
   `AsyncDriverStep`, before CP-2; P0a-1 §5.5 routes it to the same wrapper rejection, so the item covers
   it with the same outcome (`rl_async_step_call_entry_failure_rejects_wrapper`).
6. **Synchronous `startMessages()` flush** (Service Workers §3.4: an enabled client message queue is a task
   source). Removed by P0b-VM-2 as D-9's fix (PM 509 item 6); the one queue is recorded on
   `#11-sw-client-postmessage-inbound` (PM 513 (c), §7 L-10).

Out of scope, not P0's: `#11-ce-upgrade-reaction-enqueue-order` (L3's CE bundle, §0.4).

webref lookups behind §3 and this section (2026-11-08): `body ecma262 sec-error-handling-and-language-extensions`;
`body ecma262 IteratorClose` (`aoid ecma262 IteratorClose` → §7.4.11); `dfn html 'get the current value of
the event handler'` → §8.1.8.1; `dfn html 'try to upgrade an element'` → §4.13.5; `dfn dom 'inner
invoke'` → §2.9 (steps 2, 2.5, 2.13 read by `body dom concept-event-listener-inner-invoke`); `dfn html
'report an exception'` → §8.1.4.6; `dfn html 'run a worker'` → §10.2.4.

## §7. Landing list of the P0 program (canonical; PM 513 (d))

This section is the P0 program's landing list. It holds every ledger and repo-text row of the P0 memos (the
slices' landing sections point here) and is copied mechanically by the PMO from this memo's text **on the
merge commit** of the PR named in each row: one row = target, exact sentence, source. The P0 PRs edit no
ledger outside this repo; the one repo ledger they edit is the VM-P4 slot ledger, by row L-15 (PM 608). Rows
marked *PR* are done by that PR itself; rows marked *PM* are notices.

This table "at `b7f03821`" is registered in the landing list `vm-p0-program.md` (PM 517). The PM 533 revision changed L-3 and L-7 and added L-14; the PM 544 revision moved every P0a source to P0a-1 / P0a-2 and changed L-3's sentence; the PM 557 revision (P0a-1 Rev 2) changed no row's sentence and re-derived every `<path>:<lines>` cell; the PM 590 revision (β) changed L-6's and L-14's sentences and re-derived every `<path>:<lines>` cell of this memo and of P0a-1 / P0a-2. The PM 609 revision (Rev 3d) moved the P0a-2 sources to P0a-2a / P0a-2b, added L-15, changed L-1's, L-3's, L-4's and L-12's sentences to the split names, and re-derived every `<path>:<lines>` cell. The PM 615 revision (Rev 3e) changed L-15's sentence (the PMO's registration and the Re-eval flip; the date's one home) and re-derived every `<path>:<lines>` cell. The `<path>:<lines>` source cells are exact at this revision's commit; the registration is refreshed after R4 converges, from the head's table with `<path>:<lines> @ <sha>` cells (PM 540 (1)).

| Row | When | Executor | Target (kind) | Exact sentence to append | Source |
|---|---|---|---|---|---|
| L-1 | PR-1's PR | PR | PR-1's PR body | "P0a-1, P0a-2a, P0a-2b, P0b-VM-1, P0b-VM-2 and P0b-L3 landed (`git merge-base --is-ancestor <sha> HEAD` → 0 for each); the census `# pending` section is empty." | `docs/plans/2026-11-vm-stack-safety-guard-p0.md:272-273` |
| L-2 | P0a-1's merge | PM | L3 lane notice | "P0b-L3's input is `H/p0b-l3-handoff.md`; the P0 contract is P0 umbrella §4 on P0a-1's merge commit." | `docs/plans/2026-11-vm-stack-safety-guard-p0.md:253` |
| L-3 | P0a-1's merge | PMO | `#11-embedder-diagnostic-channel` (pointer row → `memory/m4-12-pr-d22-observer-pair-landing.md:71`; append there) | "Facet (stack-safety P0, P0a-2b §7.3 S rows): listener / callback errors are dropped without a report at `custom_elements/flush.rs:70` and `coerce.rs:246` (paths under `crates/script/elidex-js/src/`, lines at `ce603c74`); `engine.rs:275` (`call_listener`) and `natives_promise.rs:591` (`dispatch_unhandled_rejection_event`) report from P0a-1 on (P0a-1 §4.5, §5.4)." | `docs/plans/2026-11-vm-stack-safety-guard-p0a2b.md:152-154` |
| L-4 | P0a-2b's merge | PMO | `#11-vm-iteratorclose-precedence-convention` (live entry) | "Further sites of the same inversion (stack-safety P0a-2b §7.3 E rows, lines at `ce603c74`, under `crates/script/elidex-js/src/`): `vm/dispatch_iter.rs` 52 / 309; `vm/ops.rs:60`; `vm/host/typed_array_static.rs` 743 / 801; `vm/natives_array_hof.rs:485`; `vm/webidl_sequence.rs` 141 / 149; `vm/host/url_search_params.rs:318`; `vm/host/structured_clone.rs:1064`. No new slot." | `docs/plans/2026-11-vm-stack-safety-guard-p0a2b.md:155-159` |
| L-5 | P0a-1's merge | PMO | `#11-worker-toplevel-error-closes-worker` (live entry) | "Found by the stack-safety P0 census (P0 umbrella §6 item 3): `worker_thread.rs:200-204` closes the worker where HTML §10.2.4 reports and keeps running." | `docs/plans/2026-11-vm-stack-safety-guard-p0.md:513-515` |
| L-6 | P0a-1's merge | PMO | `#11-vm-async-resume-failure-skips-finally` (live entry) | "Outcome pinned by stack-safety P0a-1 cells `rl_async_step_entry_failure_rejects_wrapper` (the resume's check) and `rl_async_step_call_entry_failure_rejects_wrapper` (the reaction job's call of the step, D-16): the wrapper rejects with `RangeError`; the body's `finally` does not run." | `docs/plans/2026-11-vm-stack-safety-guard-p0.md:519-531` |
| L-7 | P0a-2a's merge | PMO | `#11-vm-native-callback-reentry-single-loop` (live entry) | "Stated limits of the stack-safety P0 Err-consumer census (P0a-2a §9), re-evaluated with PR-1's cut-check limits when this slot is designed: calls through function pointers, trait objects and closures passed across functions inside one crate (across crates they are the census's boundary population, PM 533); receiver types (the resolver keys on `self`-ness, module and file); writes inside a callee before the carrier call; macros; a straight-line `?` that returns before a fan-out; durable writes to a side table. Re-eval-date: 2026-12-31." | `docs/plans/2026-11-vm-stack-safety-guard-p0a2a.md:477-485` |
| L-8 | P0b-VM-1's PR | PR | `#11-history-state-structured-serialize-fidelity` (pointer row → repo memo; append to `docs/plans/2026-07-s5-5c-history-state-traversal.md:868`, the D1 bullet) | "Resource-limit errors propagate from the interim serializer and from the push/replace deserialise (stack-safety P0b-VM-1 §5.1); the degrade for other failures remains this slot's." | `docs/plans/2026-11-vm-stack-safety-guard-p0b-vm1.md:109-117` |
| L-9 | P0b-VM-2's merge | PMO | `#11-idb-structured-clone-storage` (live entry) | "`json_to_js` is fallible: a resource-limit error fails the request; other parse failures keep `undefined` (stack-safety P0b-VM-2 §5.1)." | `docs/plans/2026-11-vm-stack-safety-guard-p0b-vm2.md:128-134` |
| L-10 | P0b-VM-2's merge | PMO | `#11-sw-client-postmessage-inbound` (live entry; the PM 513 (c) queue note is already on it) | "`fire_message`'s `unwrap_or(Undefined)` for a non-resource deserialise failure is a pre-existing substitution (spec: `messageerror`), left to this slot (stack-safety P0b-VM-2 §5.2)." | `docs/plans/2026-11-vm-stack-safety-guard-p0b-vm2.md:138-144` |
| L-11 | P0b-VM-1's merge | PM | L4 notice | "VM P4 edited the L4 sites listed in stack-safety P0b-VM-1 §0 (`structured_serialize.rs`, `history.rs`, `history_events.rs`) in P0b-VM-1's PR." | `docs/plans/2026-11-vm-stack-safety-guard-p0b-vm1.md:32-39` |
| L-12 | P0a-1's merge | PM | L3 notice (site list via PM, PM 502) | "P0a-1 edited L3's AbortSignal files after rebasing on `#11-abort-signal-any-dependent-flattening`: the `#[cfg(test)]` seam `fail_next_abort_dispatch` is removed and its cell uses `InjectAt::SimpleEventDispatch` (NEW: P0a-1) (P0a-1 §4.4); the allowlist row of the abort loop is P0a-2b's (P0a-2b §7.3)." | `docs/plans/2026-11-vm-stack-safety-guard-p0a1.md:338-349` |
| L-13 | each P0 PR | PR | that PR's body | "`python3 .claude/skills/elidex-plan-review/preflight.py` on this umbrella and on the slice memo at the PR's head: rc and K / M as printed." | `docs/plans/2026-11-vm-stack-safety-guard-p0.md:334-335` |
| L-14 | P0a-1's merge | PM | notice to the owners of `elidex-script-session` and `elidex-dom-api` (site list via PM) | "P0a-1 changed the engine boundary (stack-safety P0a-1 §4.5, PM 533): `ScriptFailure` (NEW, `elidex-script-session/src/outcome.rs`); `ScriptEngine::call_listener` returns `Result<(), ScriptFailure>`; `EvalResult` carries `outcome` in place of `success`; `script_dispatch_event_core` consumes `call_listener`'s result (no revert: a `once` listener stays removed on every failure, PM 590); `FilterError::Throw` became `FilterError::Script(ScriptFailure)` and the traversal protos' failure → null arms are gone." | `docs/plans/2026-11-vm-stack-safety-guard-p0a1.md:45-63` |
| L-15 | P0a-1's PR (done on this plan branch at Rev 3d) and P0a-1's merge | PR, then PMO | the repo's VM-P4 slot ledger `docs/plans/2026-07-vm-p4-slot-ledger.md` §8 table: P0a-1's PR adds the row `#11-report-and-continue-loop-census`; then the PMO registers it in the live ledger `project_open-defer-slots.md` at P0a-1's merge, and flips the repo row's Re-eval cell to *per live ledger* | the repo row as committed (owner VM P4; Trigger: P0a-2a's landing; Re-eval *at registration: per the stack-safety P0 umbrella §7 L-15*). The PMO's registration line: "`#11-report-and-continue-loop-census` → `docs/plans/2026-07-vm-p4-slot-ledger.md` (VM-P4 slot ledger §8); Re-eval-date: 2026-12-15 (PM 608)." This row is the date's one home: the ledger's Re-eval rule admits no absolute date in a repo row (PM 615 (2)); P0 §0.4 PM 608 and P0a-2a §9 point here. Until the PMO registers it, the slot-homing check (`slot_homing_check.py`, which reads the PM-side ledger) prints this id as unhomed; that is expected. | `docs/plans/2026-11-vm-stack-safety-guard-p0a2a.md:487-502` |
