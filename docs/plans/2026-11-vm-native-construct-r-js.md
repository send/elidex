# R-JS — ECMAScript `[[Construct]]`, the this-binding and SuperCall (per-PR plan, native-construct umbrella §4.2)
Status: **plan (pre-implementation), Rev 1**, for `/elidex-plan-review` (edge-dense: §2). Owner: VM P4. Umbrella:
`docs/plans/2026-11-vm-native-construct-contract.md` (ratified at #643 `b46a030c`, PM 811 / PM 818). This memo is the
derivation the repo ledger row `#11-vm-derived-constructor-this-binding` (`docs/plans/2026-07-vm-p4-slot-ledger.md`,
the row keyed by that id) asks for (umbrella §4.2: "R-JS's per-PR plan is that derivation"). It yields three children
(§1.2); each is planned here, and this memo's plan-review is each child's plan-review (§1.2, last bullet).

**Base.** Code anchors are at `7af70b68` (origin/main, #650) unless a row says otherwise; line numbers appear only as
"at `<sha>`". Two in-flight branches are read as claims: #653 (D, `#11-vm-call-internal-callee-this-rooting`, branch
`vm-bound-call-fallback-rooting`, head `f1f6d42e`, **open, not landed** at this writing) and R-0b's memo (branch
`vm-r0b-frame-roots`, head `c2e3dd5f`, Rev 2.1, `docs/plans/2026-11-vm-r0b-frame-root-enumeration.md`, not pushed).
Each child re-reads its anchors at its own base; where a child's base must contain one of them, the child says so (§8).

**Spec numbers.** Every ECMA-262 number below is printed by `.claude/tools/webref aoid ecma262 <AO>` or `heading
ecma262 <n>` (§12 M-1), step prose by `webref body`. Rows are keyed by AO name.

## §0. Frame

### §0.1 What is carried in (ratified; not re-opened)
- **PM 675** — direction (R), R-JS before R-NAT, cut by callee kind; never per-native migration. **Umbrella §4.2** —
  one `Construct(ctor, args, newTarget)` entry; JS base allocates callee-side through phase 1; JS derived `this`
  uninitialized, `Op::PushThis` = GetThisBinding, SuperCall = Construct then BindThisValue then the
  InitializeInstanceElements seam (E-10); one return step; the native arm keeps today's contract, centralised (E-8,
  E-9); the upgrade loses its receiver and S-2's two statements; the representation is decided once, with S
  (`#11-vm-arrow-lexical-super`) and N (`#11-vm-arrow-lexical-new-target`).
- **PM 805 / PM 808** — R-JS throws upgrade step 11.4's TypeError in `invoke_upgrade` and carries the slot's test
  (`#11-ce-upgrade-returned-object-typeerror` keeps two facets in `elidex-custom-elements`, L3's: the 11 crate cites
  and the state after a failed upgrade). Verified: PORTFOLIO `PM 805` / `PM 808` lines; umbrella §4.2 "Upgrade".
- **PM 832 / PM 837 (4) / PM 845 (2) / PM 875 (1)** — the `do_new` rest-constructor window is R-JS's DoD; PM 875 (1)
  gives it its own slot, drafted as **`#11-vm-do-new-instance-rooting` (proposed; registered at D's landing, not yet
  in the PM ledger: `grep -c -F '#11-vm-do-new-instance-rooting'` on `project_open-defer-slots.md` → 0)**, draft
  `/Users/kazuaki/repos/send.sh/elidex-pm-trial/handoff/pending-at-landing/call-internal-callee-this-rooting.md` §1. Its
  second item, the custom-element upgrade path, is measured here (§4.3.5).
- **PM 851 / PM 875 (2)** — Rust-side `[[Call]]` is rooted at `call_dispatch`'s entry (#653: `call_dispatch` becomes
  a rooting wrapper, the arms move to `call_dispatch_rooted`; `call_internal` becomes `pub(super)` with the precondition
  "reached only from `call_dispatch` / `run_function`"). R-JS enters Rust-side calls through `call_dispatch` /
  `VmInner::call`. **Umbrella §8.3 stands for R-JS-2 and R-JS-3; R-JS-1 must change one expression in the Function arm
  (§0.2 C-1)** — the per-arm table is §6.1 (PM 642 (c), PM 875: the `call_dispatch_rooted` split counts as a
  `call_dispatch` change).
- **#651** (`5550c0b6`) — `VmStackScope` / `VmTempRoot` unwind the frames above them on panic and assert a clean exit
  has none. Consequence used in §4.3.2: **no stack guard is held across an inline frame push** (the frame outlives the
  guard on the `Op::New` path).
- **#650** (`7af70b68`) — `VmInner::create_coroutine` is the one generator / async creation path. Constructors never
  reach it (admission refuses generator and async kinds); R-JS-1 renames a frame field it moves (§6.1).

### §0.2 Premise corrections (brief and umbrella claims checked)
| # | Claim (source) | Measured at `7af70b68` |
|---|---|---|
| C-1 | "R-JS and R-NAT do not touch `call_dispatch`" (umbrella §8.3, §1.2 "Unchanged"; restated by PM 875 (2)) | **False for R-JS-1.** The umbrella's own §1.2 lists `vm/interpreter_call.rs:39` (at `4b723d4b`) among the `captured_this` readers R-JS changes; that line is inside `call_dispatch`'s `ObjectKind::Function` arm (`Self::compute_this_for_call(fo.this_mode, effective_this, fo.captured_this)`, read with `git show 4b723d4b:…/interpreter_call.rs`). Removing the by-value snapshot (§1.3) changes that expression. `call_internal` takes no callee `ObjectId`, so the change cannot move below the arm. One expression, one arm; §6.1 gives the per-arm table. For the PM (§11 Q-2) |
| C-2 | "§10.4.1.2 … newTarget remapped when it is the bound function — `admit_construct` (unchanged)" (umbrella §3) | `admit_construct` returns `(target, prepended)` and never sees newTarget; the remap exists only implicitly in `do_new` (`CallMode::Construct { new_target: ctor_id }` after unwrapping, right because `new` passes newTarget = callee). The entry, which takes any newTarget, applies §10.4.1.2 step 5 itself (§4.3.1) |
| C-3 | `class X extends null {}` has a base default constructor (`compiler/expr_class.rs` `ClassHeritage` doc; test `extends_null_default_ctor_is_base` in `vm/tests/tests_class_machinery.rs`, D-17b R4 G4-2) | ClassDefinitionEvaluation (§15.7.14) step 18: "If ClassHeritage is present, set ctorFunc.[[ConstructorKind]] to derived"; step 14.a.4 runs the derived default branch, so `new X()` throws a TypeError (`%Function.prototype%` is not a constructor). Today the literal-`null` path is base and the `(null)` expression path is derived: two answers for one ClassHeritage. R-JS-2 makes the kind derived for both and re-expects that test (§4.2). Spec rung 1, so not a question; reported (§11 Q-3) because it reverses a reviewed test |
| C-4 | the return substitution has two copies (umbrella E-3, A1×A4) | Two copies, and the sync copy reads the **caller's** pre-allocation: `construct_synchronous`'s tail substitutes `pre_alloc_instance`, which `dispatch_super_inner` takes from the derived frame before the call. So in `class A { constructor(){ return {tag:1} } } class B extends A {} class C extends B {}`, B's frame is rebound to `{tag:1}` by its `super()`, B returns `undefined`, and the tail returns C's pre-allocated object: `new C().tag` reads `undefined` where §10.2.2 step 15 gives B's this-binding, `1` (code read; red-before expected, cell C2-6). A defect the umbrella does not list; R-JS-2's (§4.2) |
| C-5 | #653 "may or may not have landed" (brief) | Open at `f1f6d42e`; its slot's hard edge is "lands before R-JS's implementation (11-28)" (PM ledger row). R-JS-1's base must contain it (§8) |
| C-6 | `[[ConstructorKind]]` is on the frame or the function (umbrella §4.2 "JS base / JS derived") | Nowhere. `CompiledFunction::is_class_ctor` covers base and derived alike (`bytecode/compiled.rs`); `push_js_call_frame` cannot tell them apart. R-JS-2 adds it (§4.2) |
| C-7 | umbrella S-3's `data_transfer.rs:332-341` | L3's #654 splits the file; the site is `native_data_transfer_constructor` in `vm/host/events_modern/data_transfer/mod.rs` after the split (driver note). This memo cites it by symbol only (E-4 cell C1-2) |

## §1. Derivation

### §1.1 The three axes (the ledger row's) and why they fail independently
1. **Representation** — where `[[ThisBindingStatus]]` / `[[ThisValue]]` live, and how an arrow reaches them
   (GetThisEnvironment, §9.4.3). Today: `CallFrame::this_value: JsValue` (no status) and
   `FunctionObject::captured_this: Option<JsValue>`, a by-value copy taken by `create_closure`.
2. **Bind / get** — BindThisValue (§9.1.1.3.1) in SuperCall step 9 and GetThisBinding (§9.1.1.3.3) at `Op::PushThis`
   (via ResolveThisBinding §9.4.4, `this` evaluation §13.2.1.1 step 1) and at §10.2.2 step 15. Today `Op::PushThis`
   pushes `this_value` with no status test, and `dispatch_super_inner` overwrites `this_value` (a swap, not a bind).
3. **Return** — §10.2.2 steps 9-17, today two copies: `complete_inline_frame` (inline `new`) and
   `construct_synchronous`'s tail (SuperCall, upgrade), neither reading `[[ConstructorKind]]`.

Each fails without the others: axis 1 breaks with pre-allocation still in place (an arrow created before `super()` keeps
a stale snapshot when the parent returns another object: E-4, cell C1-10); axis 3 breaks with axes 1-2 untouched
(`return 1` in a derived constructor, C-4); axis 2 needs axis 1 (a status an arrow cannot see would turn E-4's
conforming case into a ReferenceError) and is what removes pre-allocation (axis 2 is the entry's change).

### §1.2 Children (the derivation's output)
| Child | Axis | Delivers | Closes / discharges |
|---|---|---|---|
| **R-JS-1** this-environment | 1 | the this-environment record and its one capture route for arrows (§1.3); `captured_this` deleted; `dispatch_super_inner`'s rebind writes through the record (pre-allocation unchanged) | E-4 for every parent that returns an object other than the pre-allocation (C1-1, C1-2, C1-10); the representation the S / N rows wait on |
| **R-JS-2** return step | 3 | `[[ConstructorKind]]` on `CompiledFunction`; one §10.2.2 steps 9-17 implementation for both drivers, reading the callee frame's binding; `CallFrame::new_instance` and `construct_synchronous`'s `pre_alloc_instance` deleted; wire #5 born | E-3's fourth regression and its base control; C-3; C-4 |
| **R-JS-3** construct entry | 2 (+ the entry) | the single `Construct` entry; derived `this` uninitialized; GetThisBinding / BindThisValue; JS base allocation callee-side; frame-first frame push; the native arm centralised; the upgrade without a receiver, S-2, step 11.4's TypeError; the E-10 seam; wire #5 extended | E-1 (JS facet), E-2, E-3 regressions 1-3, E-6, E-9; `#11-vm-do-new-instance-rooting` (proposed); the umbrella row `#11-vm-derived-constructor-this-binding` retires at R-JS-3's landing |

- **Order R-JS-1 → R-JS-2 → R-JS-3**, each from main after the previous lands. R-JS-3 needs R-JS-1 (without the
  record, derived `this` = uninitialized would break E-4's conforming case) and R-JS-2's kind and return step (§10.2.2
  step 15 is the third regression's throw site). R-JS-2 reads the binding through R-JS-1's accessor.
- **No strangler between them.** Each child replaces its axis's mechanism outright: R-JS-1 deletes the snapshot, R-JS-2
  deletes both return copies and `new_instance`, R-JS-3 deletes the receiver and the pre-allocation for JS callees.
  What each leaves for the next is the *contract* the next changes (pre-allocation), never a second mechanism for its
  own axis (CLAUDE.md "One issue, one way").
- **Why split.** M = 22 rows over K = 3 specs (§3; `webref coverage-map` prints "K≥4 or M≥20 → 分割推奨"), and the
  axes fail independently (§1.1); each child is a single-axis terminal unit under this ratified umbrella
  (`.claude/skills/elidex-plan-review/SKILL.md`, "Base case").
- **Plan-review.** This memo is the one plan for the three; its `/elidex-plan-review` is each child's. A child whose
  implementation needs a decision this memo does not make returns to plan-review for that delta (Step 4.5). Each child
  PR runs `/pre-push` and a single-pass Codex.

### §1.3 The representation (axis 1)
The record GetThisEnvironment lands on, in ECMAScript terms, is the this-part of a Function Environment Record
([[ThisBindingStatus]] ∈ {lexical, initialized, uninitialized}, [[ThisValue]]; NewFunctionEnvironment §9.1.2.4 step 4
sets uninitialized for every non-lexical function, OrdinaryCallBindThis §10.2.1.2 step 9 binds it for a base call at
entry). Its readers are the three §9.4.3 consumers: ResolveThisBinding (`this`), GetSuperConstructor /
MakeSuperPropertyReference (`super`, §13.3.7.2 step 1, §13.3.7.1 SuperProperty step 1), GetNewTarget (`new.target`,
§9.4.5 step 1). An arrow's own environment is lexical (OrdinaryFunctionCreate §10.2.3 step 9), so its consumers reach
the enclosing non-arrow function's record.

**Candidates, weighed in VM terms** (umbrella §4.2 names the first):

| | (U) a hidden `this` binding on the upvalue machinery | (E) a this-environment record, one handle per arrow |
|---|---|---|
| Shape | `this` becomes a local slot of the function; arrows capture it as an upvalue; uninitialized = a TDZ hole | the record is a VM cell; a non-arrow frame holds it inline until an arrow observes it, then the frame and every observing arrow share one cell |
| Spec fit | `this` is not a declarative binding: BindThisValue *refuses* when initialized (step 2), the reverse of InitializeBinding / SetMutableBinding (§9.1.1.1.5 step 3 throws when *uninitialized*), so (U) needs a write op the `let` machinery does not have | one-to-one: status enum = [[ThisBindingStatus]] (non-lexical part), accessor pair = GetThisBinding / BindThisValue |
| S / N / umbrella 3 | three hidden bindings (`this`, `new.target`, home object), each captured separately: one §9.4.3 record split into three captures | one handle; S, N and umbrella 3 add fields to the same record (§1.3.2) — no second capture mechanism |
| Edge to C / `#11-vm-upvalue-tdz-check` | needs the uninitialized upvalue state umbrella C's derivation mints, and the ledger row warns that C may *replace* the capture mechanism (the row's "two-mechanism outcome"); R-JS would either wait for an undated derivation or mint C's state (two owners) | none: the record does not use upvalues (§7.2 row "C") |
| Cost | no new kind; compiler and emit changes for every `this` reader | one internal object kind; one allocation per frame whose function creates an observing arrow |

**Decision: (E).** Grounds, in order: the spec's unit (rung 1: the record and its AOs, not a binding), the S / N / 3
requirement of one capture route (umbrella §4.2 "Two edges to umbrella 3" (2)), and independence from C's undated
derivation, which (U) would either block on or pre-empt (One issue, one way). The S row already expects an
"environment link" and N's row says N "consumes S's environment link" (repo ledger rows for S and N): (E) is that link.

#### §1.3.1 Storage
- **The cell is an object of the VM's object arena, kind `ObjectKind::ThisEnvironment(ThisEnvironment)` (NEW).**
  Precedent for an internal, never-script-visible shared record in the object arena:
  `ObjectKind::PromiseCombinatorState` (shared by every combinator step). Reasons over a new arena: tracing and sweep
  come from the existing object GC (one `trace_work_list` arm), no new mark bitmap threaded through `mark_frame` /
  `mark_value`, and no touch of `vm/gc/collect.rs` (2064 lines) or `VmInner`'s field list. Not lesson #276: the state
  shape is unique (no other kind holds a this-binding). **Invariant:** a `ThisEnvironment` `ObjectId` is held only by
  `CallFrame::this_env` (`Cell`), `FunctionObject::this_env` and the call IC (§4.1); it never becomes a `JsValue`, so
  no script operation reaches it (checked in R-JS-1 by grep over the writes of the variant, §4.1 DoD).
- **Types** (NEW, `vm/frame.rs`, beside `UpvalueState`):
  - `ThisBinding { Uninitialized, Initialized(JsValue) }` — the non-lexical [[ThisBindingStatus]] with [[ThisValue]].
  - `ThisEnvironment { binding: ThisBinding }` — the record. R-JS defines one field; §1.3.2 is the room left.
  - `FrameThisEnv { Inline(ThisEnvironment), Cell(ObjectId), Unobserved }` — `CallFrame::this_env` (NEW; replaces
    `this_value`). A non-arrow frame starts `Inline`; it becomes `Cell` when an arrow that observes it is created in the
    frame (§1.3.3); an arrow frame is `Cell(<its closure's this_env>)`, or `Unobserved` when the arrow's body has no
    §9.4.3 consumer (the compiler guarantees no reader then; a read is `VmError::internal`).
- **Why inline-until-observed, not a cell per call:** a cell per non-arrow call is one allocation on every call for
  state almost always unshared. The inline and the cell form are one type behind one accessor pair (§1.3.3), and the
  only site that converts is `create_closure` (one site, one direction). It is the record's storage, not a second
  representation.

#### §1.3.2 Room for S, N and umbrella 3 (stated, not built)
- **S** (GetSuperConstructor through an arrow) reads [[FunctionObject]]; **umbrella 3** owns the home object and its
  move to the closure. Both are fields of the same record; whether `CallFrame::home_class` moves into it is umbrella
  3's derivation (umbrella §4.2), not R-JS's.
- **N** reads [[NewTarget]]. `CallFrame::mode` already holds `new_target`; whether it moves into the record or the
  record points back is N's derivation. R-JS makes the arrow capture route cover all three consumers (§1.3.3), so
  N and S add a field and an accessor, no compiler capture change.
- **SuperCall through an arrow (S)** binds the shared cell (BindThisValue on the enclosing record): the cell is shared
  by the frame and every observing arrow, so a `super()` from one arrow is seen by another, also after the constructor
  has returned. A per-arrow copy would diverge there; this is why the record is a shared cell (cell C1-11 pins
  sharing between two arrows).

#### §1.3.3 Capture and access
- **Compiler (R-JS-1):** `Scope::observes_this_environment` (NEW) in `scope/mod.rs`, set by the scope visitor
  (`scope/visitor.rs`) on an arrow scope whose body contains `this`, `super` or `new.target` (`ExprKind::This`,
  `ExprKind::Super`, `ExprKind::MetaProperty` new.target), and on every arrow scope between it and the enclosing
  non-arrow function scope — the walk `uses_arguments` already does for `arguments` (`scope/visitor.rs`, the
  `ScopeKind::Function if !…is_arrow` arm). Population by property (a §9.4.3 consumer), not by the `this` token.
  Carried to `CompiledFunction::observes_this_environment` (NEW) for arrows; false for every non-arrow function.
- **`create_closure` (`vm/ops.rs`):** for an arrow with the flag: if the current frame's `this_env` is `Inline(r)`,
  allocate `ObjectKind::ThisEnvironment(r)` and set the frame to `Cell(id)`; if it is `Cell(id)`, reuse `id`;
  `Unobserved` is impossible by the transitive flag (`debug_assert!`, `VmError::internal` in release). The arrow's
  `FunctionObject::this_env` (NEW) `= Some(id)`, replacing `captured_this`. Rooting: during the cell's allocation the
  binding's value is still in the frame (`Inline`); between the cell's allocation and the frame write there is none;
  the function object's allocation follows with the cell held by the frame.
- **Call:** `compute_this_for_call` (`vm/dispatch_ic.rs`) becomes `this_env_for_call(this_mode, receiver, this_env) ->
  FrameThisEnv` (`Strict` → `Inline(Initialized(receiver))`; `Lexical` → `Cell(id)` or `Unobserved`). Its five callers
  (`ic_call` ×2, `ic_call_method` ×2, `call_dispatch`'s Function arm; `git grep -n 'compute_this_for_call(' 7af70b68`
  → 6 lines: these 5 and the definition) pass the result to `push_js_call_frame` / `call_internal`, whose `this`
  parameter becomes `FrameThisEnv`.
- **Accessors (`VmInner`, NEW):** `this_binding(frame_idx) -> ThisBinding` and `bind_this(frame_idx, v) ->
  Result<(), VmError>` (BindThisValue: ReferenceError when already `Initialized`), both resolving `Inline` / `Cell`.
  Every reader of the frame's `this` goes through them (`Op::PushThis`, `dispatch_super_inner`, the return step,
  `create_closure`); no reader matches `FrameThisEnv` itself.

**ECS / VM idiom table** (OO → ECS lens; the VM's own idioms are the ECS terms here):
| Spec / OO notion | elidex |
|---|---|
| Function Environment Record (an object per call, linked by [[OuterEnv]]) | the frame's inline `ThisEnvironment`; boxed into a cell only when an arrow observes it |
| GetThisEnvironment's outward walk | resolved once at arrow creation (an arrow in an arrow inherits the handle), so every read is one hop |
| an arrow closing over its environment | `FunctionObject::this_env`, beside `upvalue_ids`; one handle for every §9.4.3 consumer |
| class-owned mutable `this` state | the record; one writer (BindThisValue), read by accessor |

No other engine's structure is a ground for this choice; none is named.

## §2. Coupled invariants
Eight axes (the umbrella's A1-A10 restricted to R-JS, plus the two the derivation adds, B7 and B8):

| Axis | Content |
|---|---|
| B1 record | the this-environment record, its inline / cell storage, the arrow handle (§1.3) |
| B2 bind/get | GetThisBinding at `Op::PushThis` and at §10.2.2 step 15; BindThisValue at SuperCall step 9 |
| B3 return | §10.2.2 steps 9-17, one implementation, two drivers (inline frame, nested `run()`) |
| B4 entry | `Construct(ctor, args, newTarget)`: admission, §10.4.1.2 remap, kind dispatch, base allocation, frame push |
| B5 GC | every handle the entry, the frame push and the record create is rooted before the next allocation |
| B6 CE | `invoke_upgrade` without a receiver; S-2; step 11.4's TypeError |
| B7 stack | P0a-1's CP-1 at `with_call_mode`, its cell; no new Rust recursion; #651's guard rule |
| B8 compiler | `[[ConstructorKind]]` and `observes_this_environment` on `CompiledFunction`; the default constructors |

### §2.1 Pairs (28)
| Pair | Intersection |
|---|---|
| B1×B2 | both accessors resolve `Inline` / `Cell`; BindThisValue writes the shared cell, so an arrow created before `super()` reads the bound value (E-4) |
| B1×B3 | the return step reads the binding through the accessor; for a frame whose record was boxed it reads the cell, the same value |
| B1×B4 | the entry pushes the frame with `Inline(Initialized(thisArg))` (base) or `Inline(Uninitialized)` (derived); nothing else creates a record |
| B1×B5 | the cell is an object traced by one `trace_work_list` arm (its binding's value); frames mark `Cell(id)` and `Inline(Initialized(v))` in `mark_frame` (R-0b); `FunctionObject::this_env` is marked in the Function arm (replacing `captured_this`, `vm/gc/trace.rs`) |
| B1×B6 | an upgrade constructor's arrows observe the record like any other (no CE knowledge in the record) |
| B1×B7 | boxing allocates in `create_closure`, no Rust recursion; no CP |
| B1×B8 | `observes_this_environment` decides which arrows hold a handle; a non-observing arrow keeps nothing alive (today every arrow keeps its `this` alive through `captured_this`) |
| B2×B3 | §10.2.2 step 15 is GetThisBinding on the callee's record: a derived constructor that never ran `super()` and returns `undefined` throws ReferenceError after its frame is removed (step 9), so in the caller |
| B2×B4 | SuperCall = step 6 `? Construct` through the entry, then step 9 BindThisValue: the parent's effects precede the second `super()`'s ReferenceError (E-2) |
| B2×B5 | BindThisValue stores a value already rooted by the SuperCall result on the VM stack |
| B2×B6 | `class C extends HTMLElement`: `super()` binds what `HTMLElement` returns (the construction-stack element); the wrapper is never `this` before it (E-6) |
| B2×B7 | GetThisBinding's ReferenceError is thrown in the run loop (`throw_error`, like `Op::CheckTdz`); no CP |
| B2×B8 | the status starts `Uninitialized` iff `[[ConstructorKind]]` is derived and the call is a construct |
| B3×B4 | the entry attaches nothing to the frame for the return step: it reads `mode` (Construct), the callee's kind and the binding |
| B3×B5 | the result is on the VM stack (inline) or in `run()`'s return value until pushed; a discarded `thisArg` is garbage |
| B3×B6 | step 11.4 compares what the return step produced; a derived upgrade constructor returning `undefined` after `super()` yields the element |
| B3×B7 | the sync driver's return step runs at `run()`'s entry-frame return, inside the caller's `with_call_mode` |
| B3×B8 | step 13 vs 14 is the kind; `extends null` is derived (C-3) |
| B4×B5 | §4.3.2: operands on the VM stack from entry to frame push; `proto` under a stack scope until `thisArg` exists; `thisArg` crosses no allocation before the frame owns it (frame-first push) |
| B4×B6 | the upgrade calls the entry with `Construct(C, [])` (Web IDL §3.12 step 10): no receiver, newTarget = C |
| B4×B7 | sync callers: admission outside `with_call_mode`, everything of `[[Construct]]` inside it after CP-1 (E-9); inline `new` stays inline |
| B4×B8 | the entry dispatches on `CompiledFunction`'s kind; a non-class function is base |
| B5×B6 | the wrapper (`wrapper_store`, `StrongRoot`) and the definition's constructor (`HostData::ce_constructors` via `gc_root_object_ids`) are rooted by host state (code read, `vm/gc/roots.rs` (e), `host_data/engine_feature/object_roots.rs`); the entry roots its own operands anyway (§4.3.5) |
| B5×B7 | #651: a guard held across an inline frame push would truncate the frame; the entry holds none across it |
| B5×B8 | — |
| B6×B7 | P0a-2b's D-3 row (the pre-read) loses its site with S-2; D-4 (`construct_synchronous`'s `Err`) keeps its arm (umbrella A6×A7) |
| B6×B8 | — |
| B7×B8 | — |

### §2.2 Edges → cells
Each umbrella edge R-JS owns, and the edges this derivation adds (R-n), mapped to cells (§4.x). "Red" = expected at
the child's base, "green" = after the child. Code-read expectations are confirmed by a red-cell commit first in each
child (the #650 practice).

| Edge | Cells | Child |
|---|---|---|
| E-1 prototype fallback (JS facet) | C3-6 (red: `null` proto → green: `%Object.prototype%`), C3-7 (Rust-driven newTarget ≠ ctor; a bound newTarget with no own `prototype` → default), C3-8 helper cells | R-JS-3 |
| E-2 double `super()` | C3-2 (fresh object, parent effects counted, ReferenceError, no debug panic for `TypeError`), C3-2b (caught inside the constructor; first binding kept) | R-JS-3 |
| E-3 four regressions + base controls | C3-1, C3-2, C3-3 (regressions 1-3), C2-1 (4th), C2-2 / C3-4 (base controls), C2-3 / C2-4 (`undefined` / Object return) | R-JS-2, R-JS-3 |
| E-4 arrow before `super()` | C1-1 (`HTMLElement`), C1-2 (`DataTransfer`, `native_data_transfer_constructor`), C1-3 (`Touch`), C1-10 (pure JS: parent returns another object); C3-5 (arrow called before `super()` throws ReferenceError, after it returns the bound value) | R-JS-1, R-JS-3 |
| E-5 GC | C1-7 (arrow's cell after the constructor returns), C1-8 (box allocation under GC), C1-9 (suspended generator frame holding a `Cell`) | R-JS-1 |
| E-6 upgrade | C3-11 (`setPrototypeOf(C, Blob)`: TypeError at 11.4, wrapper kind/brand untouched), A-1 green; L3's A-2 … A-8 re-expected | R-JS-3 |
| E-8 step position | stated: the native arm's pre-allocation stays before argument conversion (umbrella E-8); no cell | R-JS-3 |
| E-9 CP-1 | C3-18 (P0a-1's `rl_cp1_entry_failure_no_callee_effect` through the entry: no callee effect, no allocation) | R-JS-3 |
| E-10 seam | the call points (base: after the frame push; derived: after BindThisValue); acceptance by the two-call-site count (§4.3 DoD) — the body is Slice 2ab's | R-JS-3 |
| R-1 `do_new` window (`#11-vm-do-new-instance-rooting`, proposed) | C3-9 (red at k = 1 today, green at k = 1, 2 after) | R-JS-3 |
| R-2 sync return reads the caller's pre-allocation (C-4) | C2-6 (red → green), C2-7 (the error surfaces at the caller's `super()`) | R-JS-2 |
| R-3 `[[ConstructorKind]]` for `extends null` (C-3) | C2-8 (red: instance → green: TypeError), C2-9 (literal / parenthesised parity), C3-3b (`extends null` with an empty user constructor: ReferenceError after R-JS-3) | R-JS-2, R-JS-3 |
| R-4 §10.4.1.2 newTarget remap (C-2) | C3-21 (`new BF()` keeps today's target, control), C3-7b (Rust `Construct(BF, [], X)` keeps X; `Construct(BF, [], BF)` gives the target) | R-JS-3 |
| R-5 return-step errors are the caller's (§10.2.2 step 9 before 14 / 15) | C2-5 (a `try` inside the constructor does not catch step 14's TypeError) | R-JS-2 |
| R-6 upgrade under GC (the proposed slot's second item) | C3-17 | R-JS-3 |
| R-7 frame-first push | C3-22 (ordinary rest call under GC; guard, green both) | R-JS-3 |
| R-8 record sharing | C1-11 (two arrows share one cell after `super()`) | R-JS-1 |

## §3. Spec coverage map
| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| ECMA-262 §13.3.5.1.1 EvaluateNew ( constructExpr, argumentsNode ) | 5-6 | IsConstructor (unchanged admission), then Construct with newTarget absent | `do_new` → the entry (R-JS-3) | ✓ | yes |
| ECMA-262 §7.3.14 Construct ( ctor [ , argList [ , newTarget ] ] ) | 1-3 | newTarget absent → ctor | the entry (NEW, R-JS-3) | ✓ | yes |
| ECMA-262 §10.2.2 [[Construct]] ( argList, newTarget ) | 2-17 | base (3, 6) / derived; Object / base / non-`undefined` / `undefined` return (12-15) | entry JS arm (R-JS-3); return step (R-JS-2) | ✓ | yes |
| ECMA-262 §10.2.1.1 PrepareForOrdinaryCall ( func, newTarget ) | 7, 12 | the frame push, after step 3's creation | `push_js_call_frame` (frame-first, R-JS-3) | ✓ | no |
| ECMA-262 §10.2.1.2 OrdinaryCallBindThis ( func, calleeContext, thisArg ) | 2, 5, 9 | lexical (arrow: no bind) / strict; step 6 (non-strict) is outside core | `this_env_for_call` (R-JS-1); base construct binds at push (R-JS-3) | ✓ | yes |
| ECMA-262 §10.3.2 [[Construct]] ( argList, newTarget ) | 1-2 | R-JS stage: a fresh Ordinary from newTarget as `this` (umbrella §4.2); R-NAT makes it uninitialized | entry native arm (R-JS-3) | ✗ (R-NAT's flip) | yes |
| ECMA-262 §10.4.1.2 [[Construct]] ( argList, newTarget ) | 4-6 | step 5: newTarget is a link of the chain → the target; else unchanged | `admit_construct` (unchanged) + the entry's remap (C-2) | ✓ | yes |
| ECMA-262 §10.1.14 GetPrototypeFromConstructor ( ctor, intrinsicDefaultProto ) | 2-6 | Object / non-Object (single realm: step 5 GetFunctionRealm is the VM's one realm) | helper phase 1 (NEW, R-JS-3) | ✓ | yes |
| ECMA-262 §13.3.7.1 Runtime Semantics: Evaluation | SuperCall 1, 3, 5-9, 12 | step 6 before step 9; step 12 seam; steps 2 / 4 unchanged (1a / 1b) | `dispatch_super_inner` (R-JS-1 rebind, R-JS-3 bind) | ✓ | yes |
| ECMA-262 §13.3.7.2 GetSuperConstructor ( ) | 1-5 | `home_class`'s [[GetPrototypeOf]] (unchanged; umbrella 3 owns the field) | `dispatch_super_inner` | ✓ | yes |
| ECMA-262 §9.1.1.3.1 BindThisValue ( envRecord, value ) | 2-4 | already initialized → ReferenceError | `bind_this` (NEW, R-JS-1; called by SuperCall in R-JS-3) | ✓ | yes |
| ECMA-262 §9.1.1.3.3 GetThisBinding ( ) | 2-3 | uninitialized → ReferenceError | `this_binding` (NEW, R-JS-1); `Op::PushThis`, return step 15 (R-JS-2 / R-JS-3) | ✓ | yes |
| ECMA-262 §9.1.2.4 NewFunctionEnvironment ( func, newTarget ) | 3-4 | lexical / uninitialized | `FrameThisEnv` at frame push (R-JS-1, R-JS-3) | ✓ | no |
| ECMA-262 §9.4.3 GetThisEnvironment ( ) | 1-2 (2.1-2.5) | resolved at arrow creation, one hop | `create_closure`, `this_env_for_call` (R-JS-1) | ✓ | yes |
| ECMA-262 §9.4.4 ResolveThisBinding ( ) | 1-2 | — | `Op::PushThis` (R-JS-1 accessor, R-JS-3 status) | ✓ | yes |
| ECMA-262 §13.2.1.1 Runtime Semantics: Evaluation | 1 | `this` | `Op::PushThis` | ✓ | yes |
| ECMA-262 §10.2.3 OrdinaryFunctionCreate ( proto, sourceText, paramList, body, thisMode, envRecord, privateEnv ) | 9-10 | lexical (arrow) / strict | `create_closure` | ✓ | no |
| ECMA-262 §15.7.14 Runtime Semantics: ClassDefinitionEvaluation | 14.a.4-5, 18 | default ctor derived / base; ClassHeritage present → derived (`null` included) | `compile_class` default ctors, `[[ConstructorKind]]` (R-JS-2) | ✓ | yes |
| ECMA-262 §7.3.33 InitializeInstanceElements ( obj, ctor ) | — | call points only (Slice 2ab) | the entry (base), `dispatch_super_inner` (derived) | ✗ | yes |
| Web IDL §3.12 Invoking callback functions | construct 3, 10 | `Construct(F, jsArgs)`, newTarget absent | `invoke_upgrade` → the entry (R-JS-3) | ✓ | yes |
| WHATWG HTML §4.13.5 Upgrades | upgrade 11.3-11.4 | constructResult ≠ element → TypeError | `invoke_upgrade` (R-JS-3) | ✓ | yes |
| WHATWG HTML §4.13.4 The CustomElementRegistry interface | define 14.1-14.2 | the only `prototype` check; the upgrade-time re-read is deleted (S-2) | `invoke_upgrade` Phase 2 / 3 (R-JS-3) | ✓ | yes |

Numbers: `webref coverage-map` over these 22 rows (§12 M-1). Breadth K = 3, M = 22 → split recommended; split into
three children (§1.2). `✗` rows: the §10.3.2 row is R-NAT's flip (R-JS keeps today's contract there), the §7.3.33 row
is Slice 2ab's body.

### §3.1 User-input touch audit
Every "yes" row is reachable from script: `new`, `super()`, `this`, arrows, `extends`, the constructor arguments, a
custom-element definition and upgrade, and an assignment to a plain function's `prototype` (E-1, S-2). The
accessor-`prototype` case of the phase-1 `? Get` is not reachable from script before `#11-vm-proxy-reflect`: a
function's `prototype` is a non-configurable data property, and a class cannot declare a static `prototype` member;
Rust reaches it (C3-8). Adjacent pre-existing lax surface: `Op::NewSpread` is a dead stub (umbrella §5.2,
`#11-vm-call-spread-arguments`); untouched, exposure unchanged.

## §4. The children

### §4.1 R-JS-1 — the this-environment record (axis 1)
**Changes.**
- `vm/frame.rs`: `ThisBinding`, `ThisEnvironment`, `FrameThisEnv` (NEW); `CallFrame::this_value` → `this_env`.
- `vm/object_kind.rs`: `ObjectKind::ThisEnvironment` (NEW) variant `ThisEnvironment(ThisEnvironment)` ( doc: internal, never a
  `JsValue`). Every exhaustive `match` over `ObjectKind` gains its arm (population: §10 S-1).
- `vm/value.rs`: `FunctionObject::captured_this` → `this_env: Option<ObjectId>`; `JsCalleeInfo::captured_this` →
  `this_env`. `vm/ic.rs`: `CallIC::captured_this` → `this_env`. `vm/ops_property.rs` `collect_call_ic`,
  `vm/dispatch_ic.rs` `extract_js_callee` / `try_call_ic`: field rename.
- `vm/dispatch_ic.rs`: `compute_this_for_call` → `this_env_for_call` (§1.3.3).
- `vm/interpreter_call.rs`: §6.1's table (Function arm expression; `call_internal` / `push_js_call_frame`'s `this`
  parameter type; their frame literals).
- `vm/ops.rs` `create_closure`: the boxing rule (§1.3.3); `captured_this` deleted.
- `vm/dispatch.rs`: `Op::PushThis` reads `this_binding(frame_idx)`; `Initialized(v)` → push `v`. (`Uninitialized`
  cannot occur before R-JS-3; the arm already returns ReferenceError for it, so R-JS-3 changes no arm.)
- `vm/dispatch_class.rs` `dispatch_super_inner`: the receiver read and the rebind go through the accessors; under the
  pre-allocation contract the rebind stays today's overwrite, now of the record's binding (R-JS-3 replaces it with
  BindThisValue); `new_instance` is still updated (R-JS-2 deletes it).
- `vm/gc/mod.rs` `mark_frame` (R-0b): `this_env` classified — `Inline(Initialized(v))` → `mark_value`, `Cell(id)` →
  `mark_object`, `Inline(Uninitialized)` / `Unobserved` → nothing (an exhaustive `match`, R-0b §4's discipline);
  `vm/gc/trace.rs`: the Function arm marks `this_env` instead of `captured_this`; a `ThisEnvironment` arm marks
  `Initialized(v)`.
- `vm/natives_generator.rs`, `vm/tests/tests_with_call_mode.rs`, `vm/gc/tests.rs`: struct literals (§10 S-2).
- Compiler: `scope/mod.rs`, `scope/visitor.rs` (the flag), `compiler/function.rs`, `compiler/expr_function.rs`,
  `bytecode/compiled.rs` (+ every `CompiledFunction {` literal, `git grep -n -P 'CompiledFunction \{' 7af70b68` → 11
  lines, `bytecode/disasm.rs` included).
- `vm/ops.rs`'s two `#11-vm-ecma262-ao-section-renumber-sweep` sites (the slot's list at `1146380f`: `vm/ops.rs:264`
  OrdinaryToPrimitive, `:419` ToPropertyKey) are fixed in passing (its trigger).

**Cells** (new file `vm/tests/tests_this_environment.rs`; red / green as above, "both" = green before and after):
- C1-1 `class X extends HTMLElement { constructor(){ const f = () => this; super(); this.same = f() === this } }`
  defined, `new X().same` — red → green (umbrella E-4).
- C1-2 the same with `DataTransfer` (`native_data_transfer_constructor`) and C1-3 with `Touch` (a minimal valid
  init) — red → green (umbrella E-4; re-measured at the child's base).
- C1-10 `class A { constructor(){ return {tag: 1} } } class B extends A { constructor(){ const f = () => this; super();
  this.r = f() } } const b = new B(); b.tag === 1 && b.r === b` — red → green.
- C1-3c controls, both: base `class A { constructor(){ const f = () => this; this.ok = f() === this } }`; nested
  arrows `class A { m(){ return () => () => this } } const a = new A(); a.m()()() === a`; **negative control** —
  an ordinary nested function has its own this-environment: `let g; function C(){ g = function(){ return this } }
  new C(); g() === undefined`.
- C1-6 (Rust) an arrow whose body has no §9.4.3 consumer leaves its `FunctionObject::this_env` `None` and the
  creating frame `Inline`; one that has one makes it `Cell` once (two arrows → one cell).
- C1-7 (GC) `let f; class A { constructor(){ this.tag = 42; f = () => this } } new A();` then a full collection,
  then `f().tag === 42` — both (a guard for the new trace edge); **mutant**: dropping the Function arm's `this_env`
  mark turns it red (same commit).
- C1-8 (GC) the box allocation: a method on an object reachable only through the call (`mk()` pool pattern of
  `tests_bound_call_gc.rs`) creates an observing arrow; collection at the cell's allocation (k = 1); the arrow returns
  `this.x` — green (new code; no "before").
- C1-9 (GC) `const o = { *g(){ const f = () => this; yield 1; yield f() === o } }; const it = o.g(); it.next();`
  collection, `it.next().value` — both; **mutant**: `mark_frame` skipping `Cell` turns it red.
- C1-11 sharing: two arrows created before `super()`, one returns `this` after the constructor returns, under the
  pre-allocation contract they read the same rebound object — green (red under a per-arrow copy, mutant).

**DoD.** All cells green; `git grep -n -w captured_this` → 0 (the field and its readers, `git grep -n -w
captured_this 7af70b68 -- crates/script/elidex-js/src` → 18 lines today); every write of
`ObjectKind::ThisEnvironment` is in `create_closure` (`git grep -n 'ObjectKind::ThisEnvironment('` → the definition
site, `create_closure`, the trace arm and the tests); `mise run ci` green.

### §4.2 R-JS-2 — the return step and `[[ConstructorKind]]` (axis 3)
**Changes.**
- `bytecode/compiled.rs`: `CompiledFunction::constructor_kind: ConstructorKind { Base, Derived }` (NEW enum);
  `compiler/expr_class.rs` sets `Derived` iff ClassHeritage is present (§15.7.14 step 18), so the literal-`null` arm
  of `classify_heritage` becomes derived and gets the derived default constructor (step 14.a.4); `ClassHeritage`'s
  doc and `extends_null_default_ctor_is_base` are rewritten (the test becomes C2-8). Non-class functions: `Base`.
- `vm/construct.rs` (NEW module; one `mod` line in `vm/mod.rs`): `complete_construct(frame, result) ->
  Result<JsValue, VmError>`, §10.2.2 steps 12-17: Object → it; `Base` → the binding's value (step 13 thisArg; the base
  binding is bound once at entry and never rebound); `Derived`: non-`undefined` → TypeError (14), else
  `this_binding` → ReferenceError if `Uninitialized` (15; unreachable until R-JS-3), else the value (16-17).
- **Two drivers, one step.** (a) Inline: `complete_inline_frame` (`vm/dispatch.rs`) moves to `vm/construct.rs`,
  becomes fallible, pops the frame and truncates to `cleanup_base` first (step 9), then applies the step when the
  frame's `mode` is `Construct`; its five call sites (`git grep -n 'complete_inline_frame(' 7af70b68` → 5 lines: 4
  calls + the definition) route an `Err` through `throw_error` at the caller frame (R-5). (b) Nested `run()`: the
  three entry-frame return sites of `run()` (`Op::Return`, `Op::ReturnUndefined`, the implicit fall-through) apply
  the same step before returning when the entry frame's `mode` is `Construct`.
- `vm/dispatch_class.rs` `construct_synchronous`: the tail substitution and `pre_alloc_instance` go; the native
  branch's Object-wins-else substitution uses its `receiver` (today's contract; `receiver` goes in R-JS-3);
  `dispatch_super_inner` drops `pre_alloc`. `CallFrame::new_instance` is deleted with its marks (`mark_frame`) and its
  frame literals; `push_js_call_frame` loses the parameter.
- `vm/ops.rs` `do_new`: the native branch's substitution stays (today's contract) until R-JS-3 moves it.
- `.claude/tools/native-ctor-guard-trip-wire.sh`: **wire #5 born** (umbrella §4.6): `new_instance`,
  `pre_alloc_instance` → 0 under `crates/script/elidex-js/src`, with a control mutant in the same commit.

**Cells** (in `vm/tests/tests_construct_return.rs`, NEW):
- C2-1 `class A {} class B extends A { constructor(){ super(); return 1 } }` → TypeError — red (instance) → green.
- C2-2 base `class A { constructor(){ return 1 } } new A() instanceof A`; C2-3 derived `return undefined` → the
  instance; C2-4 derived `return {k: 1}` → that object; plain `function F(){ return 1 }; new F() instanceof F` — both.
- C2-5 `class B extends A { constructor(){ super(); try { return 1 } finally { this.f = 1 } } }`: `new B()` throws
  TypeError at the caller, and a `try`/`catch` around `return 1` inside the constructor catches nothing — red →
  green (R-5).
- C2-6 the C-4 chain, `new C().tag === 1` — red → green.
- C2-7 the sync driver: `class C extends B { constructor(){ try { super() } catch (e) { return { te: e instanceof
  TypeError } } } }` with C2-1's B → `new C().te` — red → green.
- C2-8 `class X extends null {}; new X()` → TypeError — red (instance) → green; C2-9 parity: `let h = null; class Y
  extends (h) {}; new Y()` → TypeError (both), and `class Z extends null { constructor(){ return
  Object.create(Z.prototype) } } new Z() instanceof Z` (both).

**DoD.** Cells green; wire #5 (first half) and its mutant; `git grep -n -w -e new_instance -e pre_alloc_instance` → 0
outside the wire; `mise run ci` green.

### §4.3 R-JS-3 — the construct entry (axis 2)
#### §4.3.1 The entry
`vm/construct.rs` (R-JS-2's module) gains the entry; the umbrella §4.3 helper module (NEW, beside
`vm/construct_admission.rs`; name final in the PR) gains phase 1:
`get_prototype_from_constructor(new_target, default_proto) -> Result<ObjectId, VmError>` (§10.1.14: a full `? Get`,
so a getter runs; non-Object → the default). Docstrings name the AO, numbers from `webref` (umbrella §4.3).

- **Admission** (EvaluateNew step 5 / SuperCall step 5 / Web IDL §3.12 step 3): `admit_construct`, unchanged, outside
  `[[Construct]]`.
- **§10.4.1.2 step 5**: if newTarget is the callee as passed or any bound link of its chain, newTarget becomes the
  target; otherwise unchanged (C-2). The chain walk reports links (`walk_bound_chain`'s visitor, `vm/dispatch_class.rs`).
- **JS callee** (`ObjectKind::Function`):
  - `Base`: `proto = ? get_prototype_from_constructor(newTarget, %Object.prototype%)` (§10.2.2 step 3), `thisArg =`
    OrdinaryObjectCreate(proto) (in R-JS an `Ordinary` allocation; R-NAT's phase 2 is not needed for it), the frame
    pushed with `Inline(Initialized(thisArg))` (steps 4-6.a), then the InitializeInstanceElements call point (6.b,
    `initialize_instance_elements(thisArg, func)` (NEW), body Slice 2ab's, `#11-step9-class-extras`).
  - `Derived`: no allocation, no `Get`; the frame pushed with `Inline(Uninitialized)`.
- **Native callee**: today's contract centralised (umbrella §4.2): inside `with_call_mode`'s closure,
  `proto = ? get_prototype_from_constructor(newTarget, %Object.prototype%)` with the umbrella's `debug_assert` on the
  fallback, an `Ordinary` from it, `call_dispatch(ctor, this, args, Construct { newTarget })`, Object-wins-else-`this`.
  R-NAT deletes this allocation and the substitution (its flip). E-8's deviation (allocation before argument
  conversion) is stated, not changed.
- **Drivers.** Inline (`do_new`, EvaluateNew step 6): a JS frame is left for the dispatch loop (no Rust recursion); a
  native completes through `call_construct_native`. Sync (`VmInner::construct` (NEW), `(ctor, args, new_target) ->
  Result<JsValue, VmError>`, for SuperCall, the upgrade and future callers): admission, then `with_call_mode`
  (CP-1 first, E-9), then the same core, then `run()` for a JS frame (R-JS-2's step at its return).
- **Deleted**: `construct_synchronous` (its callers call `construct`), `call_construct_native`'s `this` parameter
  (the native arm makes it), `dispatch_super_inner`'s receiver read, `invoke_upgrade`'s receiver and S-2's two
  statements. Wire #5 extended: no construct-entry signature takes a `this`
  (a check on `construct`'s and the inline core's parameter lists; control mutant).

#### §4.3.2 Rooting by construction (B5; PM 832 / 875)
The rule: **no `ObjectId` the entry holds crosses an allocation outside a root.**
1. **Operands** (`ctor`, `newTarget`, `args`) are on the VM stack from entry to the frame push: inline, `Op::New`'s
   `[ctor, args…]` (newTarget = ctor); sync, `construct` lays out `[newTarget, ctor, args…]` (cleanup offset 2). On
   the native path the operands stay on the stack until the native returns (today `do_new` truncates first, safe only
   because no allocation follows; R-JS-3 allocates there).
2. **`proto`**, from its `Get` to `thisArg`'s allocation, is held by a `push_stack_scope` that ends before any frame
   is pushed (#651: no guard spans an inline frame push).
3. **`thisArg`** crosses no allocation before the frame owns it: **`push_js_call_frame` pushes the frame before its
   first allocation** (the rest array; the frame owns the stack slot the array is written to, and
   `create_array_object` roots its elements, census `handoff/boundroot/census-2026-11-20.md`). This reorders its
   non-coroutine path; the coroutine path is `create_coroutine`'s (#650), unreachable for constructors
   (`debug_assert`).
4. After the push, the frame roots `thisArg` (`this_env`), newTarget (`mode`), the callee (cleanup region) and the
   arguments (locals, `actual_args`).
This closes R-1: today `do_new` allocates the instance with GC off and then `push_js_call_frame` allocates the rest
array while the instance is only in Rust locals (`this`, `new_instance`).

#### §4.3.3 GetThisBinding / BindThisValue
- `Op::PushThis`: R-JS-1's arm, unchanged; `Uninitialized` becomes reachable here and throws ReferenceError via
  `throw_error` (catchable in the frame).
- `dispatch_super_inner`: `result = construct(superCtor, args, newTarget)` (step 6; errors in the derived frame), then
  `bind_this` (step 9; ReferenceError in the derived frame), then `initialize_instance_elements(result, func)` (12),
  then push `result`. Nothing before `construct` mutates state (keeps P0a-1's reading, umbrella §8.1).

#### §4.3.4 The upgrade (`vm/host/custom_elements/upgrade.rs` `invoke_upgrade`)
- Phase 2's `prototype` `Get` with its two failure exits and Phase 3's `set_wrapper_prototype` call go (S-2);
  `create_element_wrapper` stays (umbrella §5.3).
- Phase 4 calls `ctx.vm.construct(JsValue::Object(constructor), &[], None)` (Web IDL §3.12 step 10).
- Step 11.4: `SameValue` failure throws `VmError::type_error` (PM 805); `finalize_failure_shim`'s call unchanged.
- Comments stating the retracted D-17b §6 invariant are rewritten (umbrella §4.2 "Scope", 13 lines by its command at
  `4b723d4b`, re-run at the child's base), and R-JS's 8 P-11 cites (umbrella §1.4 P-11, re-anchored by symbol).

#### §4.3.5 The upgrade path's rooting (the proposed slot's second item)
Measured by code read at `7af70b68`: the definition's constructor is rooted through `HostData::ce_constructors` (in
`gc_root_object_ids`); the element wrapper is a `StrongRoot` in `wrapper_store` (`mark_roots` (e)). After R-JS-3 the
entry roots its operands itself (§4.3.2), so the remaining question is the wrapper across the constructor body, which
step 11.4 compares; C3-17 measures it.

**Cells** (`vm/tests/tests_construct_entry.rs`, NEW; upgrade cells in `vm/tests/tests_construct_upgrade.rs`, NEW, not
in the 1321-line `tests_custom_elements.rs`):
- C3-1 `class B extends A { constructor(){ this.x = 1; super() } }` → ReferenceError — red → green.
- C3-2 `let n = 0; class A { constructor(){ n++ } } class B extends A { constructor(){ super(); super() } }` →
  ReferenceError and `n === 2`; and `class E extends TypeError { constructor(){ super('a'); super('b') } }` →
  ReferenceError, no debug panic — red (panic / no throw) → green. C3-2b `constructor(){ super(); try { super() }
  catch (e) { this.re = e instanceof ReferenceError } }` → `new B().re` and the first binding kept — red → green.
- C3-3 `class B extends A { constructor(){} }` → ReferenceError at the caller — red → green; C3-3b `class X extends
  null { constructor(){} }` → ReferenceError — red → green.
- C3-4 base control `class A { constructor(){ this.x = 1 } } new A().x === 1` — both.
- C3-5 `constructor(){ const f = () => this; let t = false; try { f() } catch (e) { t = e instanceof ReferenceError }
  super(); this.t = t && f() === this }` → true — red → green.
- C3-6 `function F(){}; F.prototype = 3; Object.getPrototypeOf(new F()) === Object.prototype` — red (`null`) → green.
- C3-7 (Rust, `VmInner::construct`) a base JS callee with newTarget X (`X.prototype` an object): the result's
  prototype is X's — new code; C3-7b bound: `Construct(BF, [], X)` keeps X, `Construct(BF, [], BF)` gives the target
  (`new.target` recorded by the body); newTarget a bound function (no own `prototype`) → `%Object.prototype%`.
- C3-8 (Rust, the helper) `get_prototype_from_constructor` on an object whose `prototype` getter throws → `Err`, the
  getter ran once; returns a primitive → the default; returns an object → it.
- C3-9 R-1: `function F(...r){ this.r = r }`, `const o = new F(1, 2); o !== o.r && o.r.length === 2`, and the same with
  `class A { constructor(...r){ this.r = r } }`; collection at k = 1 and k = 2 (`tests_coroutine_creation_gc.rs`'s
  `arm_gc_at_alloc`) — red at k = 1 today (the freed instance's slot is reused by the rest array, so `o === o.r`, code
  read) → green at both.
- C3-10 controls: `class A {} class B extends A {}; Object.getPrototypeOf(new B()) === B.prototype`; `class A {
  constructor(){ this.nt = new.target } } class B extends A {}; new B().nt === B`; `class E extends Error {};
  Object.getPrototypeOf(new E()) === E.prototype`; `class D extends Date {}; new D(0) instanceof D` — both.
- C3-11 E-6: `Object.setPrototypeOf(C, Blob)` then upgrade → TypeError at 11.4, the element's wrapper keeps its kind
  and prototype — green after (today the wrapper is promoted, umbrella §0); cell A-1 green.
- C3-12 PM 805 window: a plain function `F` defined, `F.prototype = 1`, upgrade: `F` runs (side effect observed),
  the upgrade fails with TypeError — red (TypeError before `F` runs) → green.
- C3-13 the slot's test: a constructor returning another object fails the upgrade with TypeError (sync
  `customElements.upgrade`) — red (NotSupportedError) → green.
- C3-14 the retraction: a constructor reading `Object.getPrototypeOf(el)` before `super()` sees the pre-upgrade
  prototype — red → green. C3-15 failure paths: a throw before `super()` leaves the pre-upgrade prototype; after
  `super()`, `C.prototype` (HTML §3.2.3 step 14 ran) — red / both.
- C3-16 the two tests citing D-17b §6 in `tests_custom_elements.rs` keep their assertions — both.
- C3-17 R-6: `class C extends HTMLElement { constructor(...r){ super(); this.n = r.length } }` upgraded with a
  collection at k = 1, 2, 3 → upgrade succeeds, `el instanceof C`, `el.n === 0` — both expected (code read §4.3.5);
  a red result reopens the slot's item.
- C3-18 E-9: P0a-1's `rl_cp1_entry_failure_no_callee_effect` re-pointed to `construct`; adds "no allocation"
  (`objects.len() - free_objects.len()` unchanged) — green.
- C3-21 `function F(){ this.nt = new.target }; new (F.bind(null))().nt === F` — both.
- C3-22 R-7 guard: `(function(...r){ return r.length })(1, 2) === 2` under a collection at k = 1 — both.

**DoD.** Cells green; `#11-vm-do-new-instance-rooting` closed (C3-9; the upgrade item answered by C3-17);
`#11-vm-derived-constructor-this-binding` retired (§7.1); wire #5 extended with its mutant; `git grep -n
'initialize_instance_elements('` → the definition and two calls; `git grep -n -w construct_synchronous` → 0; P0a-2b's
D-3 row has no site (reported to its owner); `mise run ci` green.

## §5. Files and 1000-line verdicts
`wc -l` at `7af70b68` (§12 M-3). Files over 1000:

| File | Lines | Touch | Verdict |
|---|---|---|---|
| `vm/mod.rs` | 2787 | R-JS-2: one `mod construct;` line; R-JS-3: the helper module's `mod` line | no seam for this touch (umbrella §7 row; `#11-vm-inner-substruct-decomposition`) |
| `vm/object_kind.rs` | 1646 | R-JS-1: one variant (+ doc) in the `ObjectKind` enum | **no split**: the enum is a flat case table (CLAUDE.md 1000-line rule's exemption) and the touch is one variant; the umbrella's conditional row resolves here. `HostObject`'s definition untouched (PM 642 (a) not fired) |
| `vm/gc/trace.rs` | 1265 | R-JS-1: the Function arm's `this_env` mark and the `ThisEnvironment` arm | R-0b's seam lands first (§8); the remainder is the exempt `ObjectKind` match (umbrella §7) |
| `vm/dispatch.rs` | 1105 | R-JS-1/3: `Op::PushThis`; R-JS-2: `complete_inline_frame` leaves (net −17) and the three entry-frame return sites call the step | no prereq split: the opcode match is exempt and the file shrinks (umbrella §7) |
| `vm/tests/tests_custom_elements.rs` | 1321 | R-JS-3: comment lines only | no seam for this touch; new cells go to `tests_construct_upgrade.rs` |

Under 1000 and touched: `vm/value.rs` 968 (field renames, net 0), `vm/ops.rs` 939, `scope/visitor.rs` 675,
`vm/natives_generator.rs` 674, `compiler/expr_class.rs` 653, `vm/gc/roots.rs` 618, `vm/interpreter_call.rs` 580,
`vm/ops_property.rs` 572, `compiler/function.rs` 473, `vm/host/custom_elements/upgrade.rs` 461, `scope/mod.rs` 431,
`vm/dispatch_class.rs` 391, `vm/tests/tests_class_machinery.rs` 391, `vm/host/custom_elements/html_element.rs` 344
(docstrings), `vm/dispatch_ic.rs` 342, `compiler/expr_function.rs` 338, `vm/interpreter.rs` 311, `bytecode/compiled.rs`
231, `vm/frame.rs` 221, `vm/gc/mod.rs` 122, `vm/ic.rs` 60. New: `vm/construct.rs`, the helper module, four test files.
`vm/ops.rs` gains the boxing branch (R-JS-1) and loses `do_new`'s pre-allocation (R-JS-3); it stays under 1000 by the
child's own measure, which each PR re-runs.

## §6. Overlaps
### §6.1 PM 642 (c): `vm/interpreter_call.rs`, per function (per arm for `call_dispatch`)
Read against #653's head `f1f6d42e` (R-JS-1's base must contain #653, §8).

| Function | R-JS-1 | R-JS-2 | R-JS-3 |
|---|---|---|---|
| `call_dispatch` (#653's rooting wrapper) | none | none | none |
| `call_dispatch_rooted` — `ObjectKind::Function` arm | `compute_this_for_call(…, fo.captured_this)` → `this_env_for_call(…, fo.this_env)`; nothing else | none | none |
| `call_dispatch_rooted` — `NativeFunction`, `BoundFunction`, `PromiseResolver`, `PromiseCombinatorStep`, `PromiseFinallyStep`, `AsyncDriverStep`, `ReadableStream{Start,Pull,Cancel}Step`, `WasmExportedFunction` arms, the fallthrough | none | none | none (the entry passes the native's `this`, umbrella §1.2) |
| `call_internal` | `this: JsValue` → `this_env: FrameThisEnv`; its frame literals' `this_value` → `this_env` | its `new_instance: None` literals deleted | none |
| `push_js_call_frame` | `this` parameter type; frame literal | `new_instance` parameter and field deleted | non-coroutine path: frame pushed before the rest-array allocation (§4.3.2 item 3) |

Callers of `call_internal` stay `call_dispatch_rooted` and `run_function` (#653's precondition holds; R-JS adds no
caller). The `ObjectKind::HostObject` definition is untouched by all three.

### §6.2 Stack-safety, P0a-2b, 1aa / 1ac, umbrella 3, S / N, C
- **P0a-1** (implementation 11-28, lands before R-JS-1's implementation starts, umbrella §9.1 step 5): its cell
  `rl_cp1_entry_failure_no_callee_effect` enters through `call_construct_native` / `construct_synchronous`; R-JS-3
  re-points it to `construct` (mechanical, umbrella §8.1). `with_call_mode`: unchanged (R-JS calls it).
  `dispatch_super_inner`: nothing mutates before `construct` (P0a-1's reading kept).
- **PR-1 / PR-1b**: re-read `push_js_call_frame` / `call_internal` anchors at their plan-review (umbrella §8.1).
- **P0a-2b**: D-3 (the pre-read) loses its site; D-4 stays (R-JS-3).
- **1aa / 1ac**: 1aa's SuperCall rows (step 6 on `construct_synchronous(…, receiver, …, pre_alloc)`; steps 7-9 ✗) are
  closed by R-JS-2 / R-JS-3; 1ac's `stack_slice` drains in `call_internal` / `push_js_call_frame` are other statements;
  the later rebases (umbrella §4.2).
- **Umbrella 3**: `home_class` and its `None` in `call_internal` untouched; §1.3.2 leaves the home object room in the
  record. **S / N**: consume the record (§1.3.2). **C / `#11-vm-upvalue-tdz-check`**: the edge does not arise (§7.2).
- **R-0b**: R-JS-1 classifies `this_env` in `mark_frame`; R-JS-2 removes `new_instance` from it. R-0b's §5.4
  obligation (re-point the umbrella's two-site cells to `mark_frame`) is listed in §9.

## §7. Slot homing
### §7.1 Closed
| Slot | Ledger | Closed by | Note |
|---|---|---|---|
| `#11-vm-derived-constructor-this-binding` | repo ledger (row by id); PM pointer row | R-JS-3's landing, after R-JS-1 / R-JS-2 | the derivation is this memo; its four regressions and base controls are placed: C3-1, C3-2, C3-3, C2-1, C2-2, C3-4 |
| `#11-vm-do-new-instance-rooting` (**proposed**, registration at D's landing, PM 875 (1)) | PM (pending) | R-JS-3 | C3-9; its upgrade item answered by C3-17 |

### §7.2 Edges changed
| Slot | Change |
|---|---|
| `#11-ce-upgrade-returned-object-typeerror` | R-JS-3 takes the error type and its test (C3-13), per PM 805; the two facets stay L3's (PM 808) |
| `#11-vm-native-ctor-receiver-promotion-admission` | both vectors removed by R-JS-3 (umbrella §4.2); the class closes with R-NAT |
| `#11-vm-get-prototype-from-constructor-fallback` | R-JS-3 discharges the reachable facet (C3-6); R-NAT the rest |
| `#11-vm-arrow-lexical-super` (S), `#11-vm-arrow-lexical-new-target` (N) | the `[[ThisBindingStatus]]` mint S waits on is R-JS-1's record; S / N add fields, not a capture route (§1.3.2) |
| `#11-step9-class-extras` (Slice 2ab) | R-JS-3 places the two InitializeInstanceElements call points |
| `#11-vm-upvalue-tdz-check`, umbrella C | **the edge does not arise**: (E) is chosen, the record uses no upvalue state (umbrella §5.2's row asked the plan to record this) |
| `#11-vm-generator-instance-prototype-from-function` (PM 850) | its fix (EvaluateGeneratorBody step 2's OrdinaryCreateFromConstructor) calls R-JS-3's phase-1 helper; it lands after R-JS-3 or carries its own `? Get` and rebases |
| `#11-vm-ecma262-ao-section-renumber-sweep` | fires on `vm/ops.rs` (2 sites) in R-JS-1 |
| `#11-vm-call-internal-callee-this-rooting` (D, #653) | R-JS-1's base contains it; R-JS adds no `call_internal` caller |

### §7.3 What must not ship
- Pre-allocation removed for derived callees while arrows still snapshot (`this` uninitialized in a by-value copy).
- A guard before `Construct` for the double `super()` (umbrella E-2).
- A cell per non-arrow call, or an arrow capturing the record when its body has no §9.4.3 consumer.
- Any stack guard held across an inline frame push (#651).
- The return step applied only to one driver.

## §8. Dates (umbrella §9.1, canonical)
The rule (§9.1 step 6): R-JS's implementation starts at the later of P0a-1's landing (11-28, step 5) and the end of
this plan's review (seven days after the umbrella's ratification); 11-28 holds while the umbrella was ratified by
11-20; each day past 11-20 moves step 6 and its dependants. **The umbrella was ratified on 11-18 (PM 818), so nothing
moves**: this plan 11-21 … 11-23, its plan-review 11-24 … 11-27, implementation 11-28 … 12-04, R-JS (= R-JS-3's
landing) **12-05**. Prerequisites on main before R-JS-1's implementation: R-0b (11-24, step 3), #653 (its slot's
hard edge, 11-28), P0a-1 (11-28). The children land in order inside 11-28 … 12-04; their individual dates are the
PM's to set (§11 Q-1). A slip of R-JS-3 past 12-05 is reported with the §9.1 step 6 consequences (steps 10-15 and
§5.4's R-JS-keyed Re-eval proposals).

## §9. Ledger / erratum edits (listed, not made)
- **Entry memo `docs/plans/2026-07-vm-p4-es-language-completeness.md`, [C11] / [C13] "Used by" cells**: umbrella §5.4
  says R-JS's plan PR adds R-JS there (a VM-P4 family edit; checker run on the entry memo; backticked path, no link).
  Proposed cell additions: [C11] "· R-JS (`docs/plans/2026-11-vm-native-construct-r-js.md`): steps 3, 6.a, 9-17";
  [C13] "· R-JS (same): SuperCall steps 6-9, the step-12 call point". Not made: this memo's brief forbids edits
  outside it (§11 Q-5).
- **Umbrella** (next revision, owner VM P4): §8.3 / §1.2 "do not touch `call_dispatch`" → the Function-arm exception
  (C-1); §3 row §10.4.1.2 "(unchanged)" → the entry's remap (C-2); R-0b §5.4's re-pointing of the two-site cells to
  `mark_frame`; S-3's cite → `native_data_transfer_constructor` (C-7); §4.2 / §9.1 "R-JS" = three children (§1.2).
- **Repo ledger row `#11-vm-derived-constructor-this-binding`**: at R-JS-3's landing, retired, naming the three child
  PRs.
- **PM ledger**: register `#11-vm-do-new-instance-rooting` (D's landing, PM 875 (1)); its Trigger becomes "R-JS-3";
  S / N pointer rows: "the record is R-JS-1's" (§7.2); `#11-vm-generator-instance-prototype-from-function`: the
  helper edge (§7.2).
- **D-17b R4 G4-2** (`extends_null_default_ctor_is_base`): superseded by C-3 at R-JS-2.

## §10. Facts to measure before or at each child (scout questions)
- S-1 Every exhaustive `match` over `ObjectKind` without a wildcard (each needs a `ThisEnvironment` arm); count with a
  command and a control hit.
- S-2 Every struct literal of `CallFrame`, `FunctionObject`, `JsCalleeInfo`, `CallIC`, `CompiledFunction` (tests
  included): `git grep -n -P '<Type> \{'` at the child's base (at `7af70b68`: 7 / 3 / — / — / 11 lines).
- S-3 E-4's red at base for `HTMLElement`, `DataTransfer`, `Touch` and C1-10; C-4's red (C2-6); R-1's red (C3-9).
- S-4 Whether any reader of `this_value` outside `vm/` exists (`git grep -n -w this_value -- crates`), and every
  `captured_this` reader at the child's base.
- S-5 The upgrade path under forced GC (C3-17) at R-JS-3's base.

## §11. Open questions for the PM (lean for each)
- **Q-1 Three children, one plan-review** (§1.2). Lean: yes; the PM sets the children's dates inside 11-28 … 12-04,
  R-JS-3 = 12-05.
- **Q-2 R-JS-1 changes `call_dispatch_rooted`'s Function arm** (C-1; umbrella §8.3 / PM 875 (2) read as "no touch").
  Lean: accept the one-expression change with §6.1's table; the umbrella erratum in §9.
- **Q-3 `extends null` becomes derived** (C-3), reversing a reviewed test. Lean: spec (§15.7.14 step 18), in R-JS-2.
- **Q-4 The representation (E) and its storage as an internal `ObjectKind`** (§1.3, §1.3.1), decided "once, with S
  and N". Lean: (E) in the object arena; S / N / umbrella 3 owners told at ratification.
- **Q-5 The entry memo's "Used by" cells** (§9): umbrella §5.4 puts them in R-JS's plan PR. Lean: add them in this
  plan PR before push (the driver authorises the edit).

## §12. Measurements (provenance)
- **M-1 spec.** `.claude/tools/webref aoid ecma262` on EvaluateNew, Construct, PrepareForOrdinaryCall,
  OrdinaryCallBindThis, GetPrototypeFromConstructor, GetSuperConstructor, BindThisValue, NewFunctionEnvironment,
  GetThisEnvironment, ResolveThisBinding, OrdinaryFunctionCreate, ClassDefinitionEvaluation,
  InitializeInstanceElements, MakeConstructor, IsConstructor, GetNewTarget (2026-11-21); `heading ecma262` 10.2.2,
  10.3.2, 10.4.1.2, 13.3.7.1, 9.1.1.3.3, 13.2.1.1; `coverage-map` over §3's 22 rows (K = 3, M = 22); `body` on
  `sec-ecmascript-function-objects-construct-argumentslist-newtarget`, `sec-super-keyword-runtime-semantics-evaluation`,
  BindThisValue, `sec-function-environment-records-getthisbinding`, GetThisEnvironment, OrdinaryCallBindThis,
  NewFunctionEnvironment, ClassDefinitionEvaluation (steps 14, 18), `html concept-upgrade-an-element`, `webidl
  construct-a-callback-function`.
- **M-2 counts** at `7af70b68` under `crates/script/elidex-js/src`: `git grep -n -P '<pattern>' 7af70b68` →
  `CallFrame \{` 7, `CompiledFunction \{` 11, `FunctionObject \{` 3, `captured_this` 18, `new_instance` 19,
  `pre_alloc` 7, `construct_synchronous\(` 3, `complete_inline_frame\(` 5, `push_js_call_frame\(` 7,
  `compute_this_for_call\(` 6.
- **M-3 lines.** `wc -l` in the worktree at `7af70b68` (§5).
- **M-4 roots.** `vm/gc/roots.rs` `mark_roots` (frame loop; (e) host data and `wrapper_store`),
  `vm/host_data/engine_feature/object_roots.rs` `gc_root_object_ids` (`ce_constructors`), read 2026-11-21.
- **M-5 ledgers.** `grep -n -F` on `project_open-defer-slots.md` for each slot id (2026-11-21); PORTFOLIO lines of PM
  805, 808, 832, 837, 845, 851, 875 read 2026-11-21; #653 state by `gh pr view 653` (OPEN, head `f1f6d42e`).
