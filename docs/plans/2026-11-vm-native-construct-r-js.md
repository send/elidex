# R-JS — ECMAScript `[[Construct]]`, the this-binding and SuperCall (per-PR plan, native-construct umbrella §4.2)
Status: **plan (pre-implementation), Rev 3.3** (re-check at `2dcce2e2` folded per PM 899, §12; Rev 3.2.1 moved §13's tables to a tracked companion, §13; re-check at `b8029ab9` folded per PM 892 with an additions ledger, §13; re-check at `58b666a6` folded per PM 886, §13; plan-review R1 at `bee45988` folded, §13; PM 883 D1 / D2 / IMPs; PM 880 in Rev 2),
for `/elidex-plan-review` (edge-dense: §2). Owner: VM P4. Umbrella:
`docs/plans/2026-11-vm-native-construct-contract.md` (ratified at #643 `b46a030c`, PM 811 / PM 818). This memo is the
derivation the repo ledger row `#11-vm-derived-constructor-this-binding` (`docs/plans/2026-07-vm-p4-slot-ledger.md`,
the row keyed by that id) asks for (umbrella §4.2: "R-JS's per-PR plan is that derivation"). It yields three children
(§1.2); each is planned here, and this memo's plan-review is each child's plan-review (§1.2, last bullet).

**Base.** Code anchors were measured at `7af70b68` (#650), re-checked at `378633f6` (#653 landed:
`#11-vm-call-internal-callee-this-rooting` closed) and at **`3cbd5b88`** (origin/main, #654 R-0c landed) unless a row
says otherwise; line numbers appear only as "at `<sha>`". `git diff --stat 7af70b68 378633f6` touches, of this memo's
files, only `vm/interpreter_call.rs` and `vm/natives_generator.rs`; `git diff --stat 378633f6 3cbd5b88` touches none
of them (the `data_transfer` split, `events_modern/mod.rs`, `tests_events_modern.rs` and the umbrella); §6.1 and the
umbrella's §8.1 rows are read at `3cbd5b88`; every M-2 count is identical at all three shas (§12 M-2). One in-flight branch is read as a claim: R-0b's memo (branch
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
  gives it its own slot, **`#11-vm-do-new-instance-rooting` (registered 2026-11-21 at #653's landing, Re-eval 12-05;
  `grep -n -F` on `project_open-defer-slots.md` → its open row)**, the slot R-JS's DoD closes (R-JS-3, §4.3 DoD). Its
  second item, the custom-element upgrade path, is measured here (§4.3.5). The row's clause "umbrella §8.3 stands,
  PM 875" predates PM 880's correction (§0.3 RS-1; restated in §9).
- **PM 851 / PM 875 (2) / PM 880** — Rust-side `[[Call]]` is rooted at `call_dispatch`'s entry (#653, landed
  `378633f6`: `call_dispatch` is a rooting wrapper, the arms are in `call_dispatch_rooted`; `call_internal` is
  `pub(super)` with the precondition "reached only from `call_dispatch` / `run_function`"). R-JS enters Rust-side calls
  through `call_dispatch` / `VmInner::call`. PM 875 (2)'s premise ("umbrella §8.3 stands") is **corrected by the PM
  (PM 880)**: §8.3 stands for R-NAT, R-JS-2 and R-JS-3; R-JS-1 changes one expression in the Function arm (§0.2 C-1),
  with the PM 642 (c) per-arm table (§6.1); the umbrella erratum is written in this PR (§0.3 RS-1).
- **#651** (`5550c0b6`) — `VmStackScope` / `VmTempRoot` unwind the frames above them on panic and assert a clean exit
  has none. Consequence used in §4.3.2: **no stack guard is held across an inline frame push** (the frame outlives the
  guard on the `Op::New` path).
- **#650** (`7af70b68`) — `VmInner::create_coroutine` is the one generator / async creation path. Constructors never
  reach it (admission refuses generator and async kinds); R-JS-1 renames a frame field it moves (§6.1).

### §0.2 Premise corrections (brief and umbrella claims checked)
| # | Claim (source) | Measured at `7af70b68`, re-checked at `378633f6` |
|---|---|---|
| C-1 | "R-JS and R-NAT do not touch `call_dispatch`" (umbrella §8.3, §1.2 "Unchanged"; restated by PM 875 (2)) | **False for R-JS-1.** The umbrella's own §1.2 lists `vm/interpreter_call.rs:39` (at `4b723d4b`) among the `captured_this` readers R-JS changes; that line is inside `call_dispatch`'s `ObjectKind::Function` arm (`Self::compute_this_for_call(fo.this_mode, effective_this, fo.captured_this)`, read with `git show 4b723d4b:…/interpreter_call.rs`). Removing the by-value snapshot (§1.3) changes that expression. `call_internal` takes no callee `ObjectId`, so the change cannot move below the arm. One expression, one arm; §6.1 gives the per-arm table. **Decided (PM 880, Q-2):** allowed with that table; at `3cbd5b88` the wrapper `call_dispatch` is `:29`, `call_dispatch_rooted` `:46`, its Function arm `:64` and the expression `:68` (`git grep -n 'compute_this_for_call(' 3cbd5b88 -- crates/script/elidex-js/src/vm/interpreter_call.rs` → 1 line); umbrella erratum §0.3 RS-1 |
| C-2 | "§10.4.1.2 … newTarget remapped when it is the bound function — `admit_construct` (unchanged)" (umbrella §3) | `admit_construct` returns `(target, prepended)` and never sees newTarget; the remap exists only implicitly in `do_new` (`CallMode::Construct { new_target: ctor_id }` after unwrapping, right because `new` passes newTarget = callee). The entry, which takes any newTarget, applies the bound function exotic object [[Construct]] (§10.4.1.2, `webref aoid`/`heading ecma262 10.4.1.2`) step 5 itself, "If SameValue(func, newTarget) is true, set newTarget to target" (`webref body ecma262 sec-bound-function-exotic-objects-construct-argumentslist-newtarget`), iterated over the chain (§4.3.1): a newTarget equal to any link ends as the target. Mechanism: the one chain walk, its visitor also given each link (§4.3.1). Approval: PM 883 D2 ("PM 880 item 7" was the driver's brief, not a PM decision; R1 A5 IMP-2); umbrella erratum §0.3 RS-2 |
| C-3 | `class X extends null {}` has a base default constructor (`compiler/expr_class.rs` `ClassHeritage` doc; test `extends_null_default_ctor_is_base` in `vm/tests/tests_class_machinery.rs`, D-17b R4 G4-2) | ClassDefinitionEvaluation (§15.7.14, `webref aoid ecma262 ClassDefinitionEvaluation`) step 18: "If ClassHeritage is present, set ctorFunc.[[ConstructorKind]] to derived"; step 14.a.4 runs the derived default branch, so `new X()` throws a TypeError (`%Function.prototype%` is not a constructor). Today the literal-`null` path is base and the `(null)` expression path is derived: two answers for one ClassHeritage. R-JS-2 makes the kind derived for both and re-expects that test (§4.2). **The test whose expectation reverses:** `extends_null_default_ctor_is_base` (`vm/tests/tests_class_machinery.rs`; `git grep -n 'extends null' -- crates/script/elidex-js/src/vm/tests` → its source line among 7 hits) asserts `new X() instanceof X`; its new expectation, a TypeError, comes from the spec (step 18 makes the kind derived; step 14.a.4's derived branch, "If IsConstructor(func) is false, throw a TypeError exception", with func = `%Function.prototype%`), not from any engine's behaviour. Its siblings `extends_null_prototype_proto_is_null`, `extends_null_ctor_proto_is_function_prototype`, `extends_null_user_super_call_throws` keep theirs. Spec rung 1 (PM 880, Q-3); §0.3 RS-4 |
| C-4 | the return substitution has two copies (umbrella E-3, A1×A4) | Two copies, and the sync copy reads the **caller's** pre-allocation: `construct_synchronous`'s tail (`vm/dispatch_class.rs` `VmInner::construct_synchronous`, called by `dispatch_super_inner` and by `invoke_upgrade` in `vm/host/custom_elements/upgrade.rs`) substitutes `pre_alloc_instance`, which `dispatch_super_inner` takes from the derived frame before the call. So in `class A { constructor(){ return {tag:1} } } class B extends A {} class C extends B {}`, B's frame is rebound to `{tag:1}` by its `super()`, B returns `undefined`, and the tail returns C's pre-allocated object: `new C().tag` reads `undefined` where the spec gives `1`: B's default constructor is built-in (ClassDefinitionEvaluation step 14.b, CreateBuiltinFunction), so its [[Construct]] is §10.3.2's and its derived branch returns `? Construct(func, args, NewTarget)`'s result (steps 14.a.4.iv, 14.a.7), A's `{tag:1}`; with an explicit `constructor() { super() }` in B, §10.2.2 step 15 gives B's this-binding, the same object. **Measured** (driver, a temporary test on origin/main `378633f6`, rustc 1.99.1, removed after; PM 880 item 6): `class A { constructor() { return {tag: 1} } } class B extends A {} class C extends B {} new C().tag` → `undefined`; the same with an explicit `constructor() { super() }` in B and C → `undefined`; the two-level `new B().tag` → `1` (cell C2-6). A defect the umbrella does not list; R-JS-2's (§4.2) |
| C-5 | #653 "may or may not have landed" (brief) | **Landed** `378633f6` (11-21); `#11-vm-call-internal-callee-this-rooting` closed (PM ledger row, PM 880). Every child's base contains it (§8) |
| C-6 | `[[ConstructorKind]]` is on the frame or the function (umbrella §4.2 "JS base / JS derived") | Nowhere. `CompiledFunction::is_class_ctor` covers base and derived alike (`bytecode/compiled.rs`); `push_js_call_frame` cannot tell them apart. R-JS-2 adds it (§4.2) |
| C-7 | umbrella S-3's `data_transfer.rs:332-341` | #654 (`3cbd5b88`) split the file and re-cited S-3 by symbol itself (`native_data_transfer_constructor`, `vm/host/events_modern/data_transfer/mod.rs`; `git diff 378633f6 3cbd5b88 -- docs/plans/2026-11-vm-native-construct-contract.md` → that one line). The umbrella's other `data_transfer.rs` mentions describe R-0c at its base and stay; Rev 1.7 (5) marks R-0c landed (§0.3 RS-5). This memo cites the site by symbol only (C1-2) |

### §0.3 Ratified-surface changes in this PR (reviewed here)
Each row changes a ratified surface, so it is a design change under this memo's `/elidex-plan-review`, not a clerical
edit (PM 880). The reviewer checks the row's text against the code at `3cbd5b88` and the spec cites named. **Approval
scope:** PM 880's go named the umbrella's §1.2 / §8.3 only; **PM 883 D2 widens it** to RS-2 and §8.1's four rows (RS-1);
D1 adds RS-6 … RS-8; PM 886 adds RS-9 (the sweep); PM 892 folds Rev 3.2's corrections into RS-3 and RS-9 (no new surface).

| # | Surface (file, heading) | Old → new | Grounds |
|---|---|---|---|
| RS-1 | umbrella §1.2 "Unchanged", §8.1 rows `call_dispatch`, `call_internal`, `push_js_call_frame`, `construct_synchronous`, §8.3; history §0.2 Rev 1.7 (1), (3) | "R-JS and R-NAT do not touch `call_dispatch`" / "`call_dispatch`'s arms (`vm/interpreter_call.rs:17`)" → R-NAT, R-JS-2, R-JS-3 do not; R-JS-1 changes one expression of `call_dispatch_rooted`'s Function arm (`:46` / `:64` / `:68` at `3cbd5b88`; wrapper `:29`). §8.1: `call_internal` "its three `new_instance: None` frame literals (`:385`, `:420`, `:469`)" → R-JS-1 the `this` type and `this_value` (`:399`), R-JS-2 the one `new_instance: None` (`:415`; the coroutine frame is a struct update); `push_js_call_frame` "frame literals (`:619`, `:671`, `:718`)" → one literal (`:569`), parameter `:480`, field `:581`, and R-JS-3's frame-first push; `construct_synchronous` "R-JS drops `receiver` / `pre_alloc_instance`" → R-JS-2 the tail, R-JS-3 **deletes** it; `call_dispatch`'s "`:241-246`" → the `WasmExportedFunction` arm `:267` | C-1; PM 880 Q-2; PM 883 D2; §6.1 |
| RS-2 | umbrella §3, row "ECMA-262 §10.4.1.2 [[Construct]]"; §0.2 Rev 1.7 (2) | "`admit_construct` (unchanged)" → `admit_construct`'s one chain walk (steps 1-4 and step 5's test; its visitor is also given each link, newTarget passed in) + the entry (uses the remapped newTarget) | C-2; PM 883 D2; §4.3.1 |
| RS-3 | entry memo `docs/plans/2026-07-vm-p4-es-language-completeness.md`, cited-sections table, [C11] / [C13] "Used by"; umbrella §4.2 split bullets; Rev 1.7 (4) | entry memo [C11] "R-JS (…): steps 3, 6.a, 9-17" → "steps 2-6.a, 9-17, the step-6.b call point and its step-6.c exit"; [C13] "SuperCall steps 6-9, the step-12 call point" (kept). Umbrella §4.2 "[C11] steps 3, 6.a and 12-17" → "steps 2-6.a, 6.c and 9-17"; "Stays with 2ab: InitializeInstanceElements" → its body (R-JS places the call points) | §3 (step 2 = R-JS-2's kind, step 4 = R-JS-3's push, 6.c = R-JS-3's removal of the frame when the step-6.b call is abrupt, step 9 = R-JS-2's frame removal, steps 10-11 its completion handling; 6.c and the step-9 / 10-11 split are Rev 3.2, re-check A4, `webref body` §12 M-1); checker and family test (§12 M-6). **Net against main (`3cbd5b88`):** the entry memo's two R-JS entries are an **addition** (the quoted "old" [C11] text was introduced by this PR's `bee45988` and is not on main); the umbrella's [C11] range and "Stays with 2ab" are on main and are replaced |
| RS-4 | the reviewed test expectation `extends_null_default_ctor_is_base` (D-17b R4 G4-2) | `new X() instanceof X` → TypeError | C-3; spec rung 1, ClassDefinitionEvaluation steps 18 / 14.a.4 (PM 880 Q-3) |
| RS-5 | umbrella §0.2 history order; §4.2 "may yield children"; §9.1 step 6; §7 `data_transfer.rs` row; §9.1 step 8 | rows 1.5, 1.7, 1.6 → 1.5, 1.6, 1.7; "it may yield children along those axes" → three children, one plan-review; step 6 names R-JS = R-JS-3's landing; R-0c "targets 11-21" → landed #654 `3cbd5b88` | R1 A5 / A3 MINs; §8. **Net against main (`3cbd5b88`):** the history order is an **addition** (row 1.7 does not exist on main; this PR's `bee45988` placed it before 1.6, Rev 3 after it); the other three are on main and are replaced |
| RS-6 | slice-plan `docs/plans/2026-07-vm-p4-slice-plan.md` row **C** (`:961` at `bee45988`), its complement classification | "the child of `#11-vm-derived-constructor-this-binding` that owns the `[[ThisBindingStatus]]` representation delivers it on the **call frame**, not a binding cell" → that child (R-JS-1) delivers **row C's first record kind**: a Function Environment Record is a declarative Environment Record (§9.1.1.3), so `ObjectKind::ThisEnvironment` is the kind C's derivation extends, never a second capture mechanism beside it; the old text kept as a ⚠ note | D1 (i), §1.3.4; made in this PR |
| RS-7 | repo ledger `docs/plans/2026-07-vm-p4-slot-ledger.md` row `#11-vm-derived-constructor-this-binding` (`:293`) | "The status on the call frame is the artifact" → "The status on the function's this-environment record is the artifact: held inline by the call frame, a shared `ObjectKind::ThisEnvironment` record once an arrow observes it — the first record kind of umbrella C's representation (R-JS plan §1.3, PM 883 D1)"; the row's other clauses likewise — three more "on the frame" phrasings, the [C11] range, 2ab's share, the kind's home and `construct_synchronous`'s fate (landing list A1 items 2-8, §9) | D1 (i); applied by the recorder at R-JS's plan landing |
| RS-9 | the sweep of the old facts (PM 886): (a) the this-binding status / representation is "on the call frame"; (b) `construct_synchronous` survives R-JS ("loses both parameters", "becomes a caller of the single entry", "R-JS changes D-4's arguments"); (c) R-JS edits `call_internal` / `push_js_call_frame` "for `new_instance` only". Sites: slice-plan rows **S** (two sites), **1a**, **2a**, **2ab**, **9a** and the 2ab ordering paragraph; umbrella §1.2 (In list, `construct_synchronous` row), §1.4 P-11, §2.1 A6×A7, §2.2 E-9, §4.2 (One entry; the representation sentence; umbrella 3's split bullet; 1aa), §8.1 `invoke_upgrade`, §8.4, history 1.7 (6); repo ledger `:293` / `:333` (landing list A1 / A5); PM ledger `:248`, `:254`, `:257` (landing list C2, at R-JS-3's landing: they describe today's tree). Rev 3.2 (re-check A5): row S's wording agrees with row C ("inline on the frame until an arrow observes it, then a shared cell"); 1a's stack-layout evidence and 2ab's E-10 cite are re-pointed; history 1.7 (6) names fact (a)'s sites and PM 886 | Old: as quoted at each site at `58b666a6` (e.g. umbrella §1.2 "loses both parameters; becomes a caller of the single entry"; §4.2 "R-JS edits the same two functions (`call_internal`, `push_js_call_frame`; §8.1) for `new_instance` only"; slice-plan S "the `[[ThisBindingStatus]]` representation on the call frame") → new: a pointer to the full statement or a one-line correction in place. **The full statement lives once:** (a) in RS-6 / RS-7 and §1.3; (b) and (c) in umbrella §8.1's rows and RS-1. Umbrella 3's overlap reading is kept: `home_class`, its `None` in `call_internal` and its rooting stay as they are | PM 886 (one sweep, pointer-or-one-line); re-check A5 N1 / N2 |
| RS-8 | S / N canonical rows: slice-plan rows **3** (`:951`), **C** (`:961`, two sites), **N** (`:971`); repo ledger `:286` (N), `:287` (S) | slice-plan C: "put a field on `FunctionObject` beside the `captured_this` that already exists" → add a field to the record reached through `FunctionObject::this_env`; the I-7 sentence's "a field beside the `captured_this`" → R-JS-1's kind / `this_env`. Rows 3 / N and ledger `:286` / `:287` describe today's tree ("no … beside `captured_this`") and stay true until R-JS-1: each gains a forward pointer, not a rewrite (slice-plan rows here; ledger rows in the landing list) | D1; §1.3.2 |

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
| **R-JS-1** this-environment | 1 | the this-environment record and its one capture route for arrows (§1.3); `captured_this` deleted; `dispatch_super_inner`'s rebind writes through the record (pre-allocation unchanged); wire #5 born with its record half | E-4 for every parent that returns an object other than the pre-allocation (C1-1, C1-2, C1-10); the representation the S / N rows wait on |
| **R-JS-2** return step | 3 | `[[ConstructorKind]]` on `CompiledFunction`; one §10.2.2 steps 9-17 implementation for both drivers, reading the callee frame's binding; `CallFrame::new_instance` and `construct_synchronous`'s `pre_alloc_instance` deleted; wire #5's `new_instance` / `pre_alloc_instance` check | E-3's fourth regression and its base control; C-3; C-4 |
| **R-JS-3** construct entry | 2 (+ the entry) | the single `Construct` entry; derived `this` uninitialized; GetThisBinding / BindThisValue; JS base allocation callee-side; frame-first frame push; the native arm centralised; the upgrade without a receiver, S-2, step 11.4's TypeError; the E-10 seam; wire #5 extended | E-1 (JS facet), E-2, E-3 regressions 1-3, E-6, E-9; `#11-vm-do-new-instance-rooting` (registered); the umbrella row `#11-vm-derived-constructor-this-binding` retires at R-JS-3's landing |

- **Order R-JS-1 → R-JS-2 → R-JS-3**, each from main after the previous lands. R-JS-3 needs R-JS-1 (without the
  record, derived `this` = uninitialized would break E-4's conforming case) and R-JS-2's kind and return step (§10.2.2
  step 15 is the third regression's throw site). R-JS-2 reads the binding through R-JS-1's accessor.
- **No strangler between them.** Each child replaces its axis's mechanism outright: R-JS-1 deletes the snapshot, R-JS-2
  deletes both return copies and `new_instance`, R-JS-3 deletes the receiver and the pre-allocation for JS callees.
  What each leaves for the next is the *contract* the next changes (pre-allocation), never a second mechanism for its
  own axis (CLAUDE.md "One issue, one way").
- **Why split.** M = 22 rows over K = 3 specs (§3; `webref coverage-map` prints "K≥4 or M≥20 → 分割推奨"), and the
  axes fail independently (§1.1). The skill's base case (`.claude/skills/elidex-plan-review/SKILL.md`,
  "When to invoke", Base case) reads: "承認済 umbrella 配下で plan-review を通った narrowly-scoped per-PR slice は
  **terminal 単位** = 許容される単一 PR". R-JS-1 and R-JS-2 each touch one axis of §1.1; R-JS-3 carries B2, B4, B5 and
  B6 (§2) and stays one PR because the umbrella's ratified §4.2 makes them one change: deleting the receiver and the
  pre-allocation for both drivers at once is what leaves no strangler; its pairs are mapped in §2.1 and its edges in §2.2.
- **Plan-review.** This memo is the one plan for the three; its `/elidex-plan-review` is each child's. A child whose
  implementation needs a decision this memo does not make returns to plan-review for that delta (Step 4.5). Each child
  PR runs `/pre-push` and a single-pass Codex.
- **Docstrings, every child.** Each new or rewritten symbol that implements an AO step (R-JS-1: `ThisBinding`,
  `ThisEnvironment`, `FrameThisEnv` with `::bound` / `::lexical`, `this_binding`, `this_env_for_call`, `Op::reads_this_environment`;
  R-JS-2: `ConstructorKind`, `complete_construct`; R-JS-3: the entry, the helper, `initialize_instance_elements`,
  `bind_this`, `ThisBinding::Uninitialized` / `FrameThisEnv::uninitialized` with GetThisBinding's step 2 in `this_binding`) names
  the AO by name and its number from `webref aoid` / `heading` at the child's base (CLAUDE.md "Spec citation").

### §1.3 The representation (axis 1)
The record GetThisEnvironment lands on, in ECMAScript terms, is the this-part of a Function Environment Record
([[ThisBindingStatus]] ∈ {lexical, initialized, uninitialized}, [[ThisValue]]; NewFunctionEnvironment §9.1.2.4 step 4
sets uninitialized for every non-lexical function, OrdinaryCallBindThis §10.2.1.2 step 9 binds it for a base call at
entry). Its readers are the three §9.4.3 consumers: ResolveThisBinding (`this`), GetSuperConstructor /
MakeSuperPropertyReference (`super`, §13.3.7.2 step 1, §13.3.7.3 step 1, §13.3.7.1 SuperProperty step 1), GetNewTarget (`new.target`,
§9.4.5 step 1). An arrow's own environment is lexical (OrdinaryFunctionCreate §10.2.3 step 9), so its consumers reach
the enclosing non-arrow function's record. Two further callers: SuperCall step 7 (`thisER`, which step 9 binds; its op
is already `Reads` through steps 1 and 3, §1.3.3), and PerformEval (§19.2.1.1 step 11.a, direct eval), which is
unreachable today (core exposes no JS-level `eval`, `vm/frame.rs` `CallFrame::kind`'s doc); when a strict direct eval
lands, the op that performs it is classified as reading the record (§1.3.3), so an arrow containing one captures.

**Candidates, weighed in VM terms** (umbrella §4.2 names the first):

| | (U) a hidden `this` binding on the upvalue machinery | (E) a this-environment record, one handle per arrow |
|---|---|---|
| Shape | `this` becomes a local slot of the function; arrows capture it as an upvalue; uninitialized = a TDZ hole | the record is a VM cell; a non-arrow frame holds it inline until an arrow observes it, then the frame and every observing arrow share one cell |
| Spec fit | the this-part is not a named binding: BindThisValue *throws* when already initialized (step 2); InitializeBinding (§9.1.1.1.4 step 1) has the same precondition but only *asserts* it, and only SetMutableBinding (§9.1.1.1.5 step 3, throws when *uninitialized*) is the reverse — so (U) needs a checked-initialize op the `let` machinery does not have | one-to-one: status enum = [[ThisBindingStatus]] (non-lexical part), accessor pair = GetThisBinding / BindThisValue |
| S / N / umbrella 3 | three hidden bindings (`this`, `new.target`, home object), each captured separately: one §9.4.3 record split into three captures | one handle; S, N and umbrella 3 add fields to the same record (§1.3.2) — no second capture mechanism |
| Edge to C / `#11-vm-upvalue-tdz-check` | needs the uninitialized upvalue state umbrella C's derivation mints, and the ledger row warns that C may *replace* the capture mechanism (the row's "two-mechanism outcome"); R-JS would either wait for an undated derivation or mint C's state (two owners) | the record is **C's first record kind** (§1.3.4): R-JS-1 mints it with the this-part only and no upvalue state; C extends it. No wait, no second owner |
| Cost | no new kind; compiler and emit changes for every `this` reader | one internal object kind; one allocation per frame whose function creates an observing arrow |

**Decision: (E)** (kept, PM 883 D1). Grounds, in order: the spec's unit (rung 1: the record and its AOs, not a
binding), the S / N / 3 requirement of one capture route (umbrella §4.2 "Two edges to umbrella 3" (2)), and one
mechanism with C: (E) is the first record kind of C's representation, which C extends (§1.3.4), where (U) would
either block on C's undated derivation or pre-empt it (One issue, one way). The S row already expects an
"environment link" and N's row says N "consumes S's environment link" (repo ledger rows for S and N): (E) is that link.

#### §1.3.1 Storage
- **The cell is an object of the VM's object arena, kind `ObjectKind::ThisEnvironment(ThisEnvironment)` (NEW).**
  Precedent for an internal, never-script-visible shared record in the object arena:
  `ObjectKind::PromiseCombinatorState` (shared by every combinator step). Reasons over a new arena: tracing and sweep
  come from the existing object GC (one `trace_work_list` arm), no new mark bitmap threaded through `mark_frame` /
  `mark_value`, and no touch of `vm/gc/collect.rs` (2064 lines) or `VmInner`'s field list. Not lesson #276: the state
  shape is unique (no other kind holds a this-binding). **Invariant — by type where Rust can state it, by a
  standing wire where it cannot** (Rev 3.2, re-check A3; Rev 3.3, re-check A3 MIN-2 / A2 MIN-2): the handle is `ThisEnvId(ObjectId)` (NEW, `vm/frame.rs`),
  whose field is private to `vm/frame.rs`, so a `ThisEnvId` can be made only there. The record is made only there
  too: `ThisEnvironment`'s field is private to `vm/frame.rs` and it has no `Clone`, `Default` or constructor outside
  it (`ObjectKind` derives nothing, so none is needed), so `ObjectKind::ThisEnvironment(_)` can wrap only a record
  `vm/frame.rs` made — moving one out of an existing object's `kind` creates none and is a `kind` write, umbrella
  §4.6's property. That module holds the one `impl VmInner` block that touches the record: the boxing step
  `box_this_environment` (NEW, the one allocation of the kind and the only expression that builds a `ThisEnvId`,
  called by `create_closure`, §1.3.3) and the accessors (§1.3.3). The trace arm reads the value through
  `ThisEnvironment::traced_value() -> Option<JsValue>` (NEW, read only). The one way out of the handle,
  `ThisEnvId::object_id()` (`pub(in crate::vm)`), exists for the collector (`vm/frame.rs` reads the field itself): its callers are `vm/gc/` only. Rust
  cannot narrow that (`pub(in path)` admits only an ancestor module, and `vm::frame` and `vm::gc` share only
  `crate::vm`), so that one fact is **wire #5's record half, born in R-JS-1** (§4.1; R-JS-2 adds the
  `new_instance` half, §4.2). Holders (all NEW, §4.1): `CallFrame::this_env`, `FunctionObject::this_env`,
  `JsCalleeInfo::this_env`, `CallIC::this_env`; a `JsValue` comes from a handle only through `object_id()`.
- **Types** (NEW, `vm/frame.rs`, beside `UpvalueState`):
  - `ThisBinding { Initialized(JsValue) }` (private to `vm/frame.rs`) — the non-lexical [[ThisBindingStatus]] with [[ThisValue]]. R-JS-1 and
    R-JS-2 produce only `initialized` (the pre-allocation contract binds every construct frame at entry), so
    `Uninitialized` lands in R-JS-3 with its first constructor, `FrameThisEnv::uninitialized()`, called by the derived frame push (§4.3.1, §4.3.3): a variant no code
    constructs fails `mise run ci` (`mise.toml:19`, `cargo clippy --workspace -- -D warnings`, lib targets: a test
    that constructs it does not count; CLAUDE.md "dead code は接続するか削除", no `#[allow(dead_code)]`). Every
    `match` over `ThisBinding` is exhaustive and in `vm/frame.rs`, so the compiler lists the arms R-JS-3 adds.
  - `ThisEnvironment { binding: ThisBinding }` (field private) — the record. R-JS defines one field; §1.3.2 is the room left.
  - `FrameThisEnv` — `CallFrame::this_env` (NEW; replaces `this_value`): a struct over a representation private to
    `vm/frame.rs`, `{ Inline(ThisEnvironment), Cell(ThisEnvId), Unobserved }`, so **outside `vm/frame.rs` no code can
    build or match it** (re-check A2 MIN-2). Constructors: `FrameThisEnv::bound(v)` (`Inline(Initialized(v))`) and
    `FrameThisEnv::lexical(handle: Option<ThisEnvId>)` (`Cell` or `Unobserved`), R-JS-1; `::uninitialized()`, R-JS-3.
    The collector's classifier `FrameThisEnv::gc_edge() -> ThisEnvEdge { Value(JsValue), Cell(ThisEnvId), Nothing }`
    (NEW) is the one view out (`mark_frame`, §4.1). A non-arrow frame starts `Inline`; it becomes `Cell` when an arrow
    that observes it is created in the frame (§1.3.3); an arrow frame is `Cell(<its closure's this_env>)`, or
    `Unobserved` when the arrow's body has no §9.4.3 consumer (by §1.3.3's emission rule no op of its body reads the
    record; a read is a VM invariant violation, §1.3.3).
- **Why not an ECS component or a HostData side-store** (CLAUDE.md "Side-store→component 判定ルール"; PM 880 Q-4,
  checked in plan-review): the rule's subject is *per-entity* state kept in an entity-keyed side-store. The record is
  neither: it is keyed by no entity (it belongs to a call frame and the arrows that close over it — a JS binding, like
  an upvalue), and its content is a `JsValue` that may be an `ObjectId`, i.e. a per-VM identity handle, the class the
  rule's exception (a) keeps out of components. With no entity there is no component to attach (and no despawn to
  clean it up); with no host-side fact there is nothing for `HostData` to hold. It is per-VM object-arena state, so it
  lives in the object arena and is collected by the object GC, as upvalues and `PromiseCombinatorState` are.
- **Why inline-until-observed, not a cell per call:** a cell per non-arrow call is one allocation on every call for
  state almost always unshared. The inline and the cell form are one type behind the accessors (§1.3.3), and the
  only site that converts is `box_this_environment`, called by `create_closure` (one site, one direction). It is the record's storage, not a second
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
- **Compiler (R-JS-1): defined by the property at emission, not by a scope visitor's coverage** (PM 883; R1 A2 IMP).
  An arrow *observes the this-environment* iff the bytecode of its own function contains an op that reads the record.
  `FunctionCompiler::observes_this_environment` (NEW, `compiler/function.rs`) is set in one private `push_op` (NEW),
  which every `emit*` helper calls instead of pushing `op.to_byte()` itself, by `Op::reads_this_environment` (NEW,
  `bytecode/opcode.rs`) `-> ThisEnvRead { Reads, No, ByChildConstant }`: an exhaustive `match` with no wildcard arm, so
  a new op cannot be added unclassified, and an op whose reading is uncertain is classified `Reads` (**fail-safe:
  unknown → capture**). `Reads`: the §9.4.3 consumers — `PushThis`, `NewTarget`, `SuperCall`, `SuperCallSpread`, the
  existing super-property stubs `GetSuperProp`, `SetSuperProp`, `GetSuperElem` (`bytecode/opcode.rs`; their dispatch
  arms read nothing today, but each is a §9.4.3 consumer through MakeSuperPropertyReference, §13.3.7.3 step 1
  "Let envRecord be GetThisEnvironment()"), every super-property op umbrella 3's child or S adds, a future
  direct-eval op. `ByChildConstant`: `Closure` only — whether it reads is the flag of the child its u16 constant
  operand names (the child is compiled before its `Closure` is emitted). `push_op(op, operand: Option<u16>)` (every
  `emit*` helper passes its first u16 operand; `emit_u8_u16`'s u16 is an IC index and passes `None`) resolves the
  three values in one exhaustive `match`: `Reads` → set the flag; `No` → nothing; `ByChildConstant` → set the flag iff
  the child function the constant names is an arrow with the flag set — and **set it** when the operand is `None`
  or the constant names no function (unknown → capture, by construction, not by the call site). That rule is the
  outward propagation through arrows; it stops at the nearest non-arrow function because only an arrow child
  propagates. `CompiledFunction::observes_this_environment`
  (NEW) = the flag for arrows, `false` for every non-arrow function.
  - Why emission and not the scope visitor (`scope/visitor.rs`, whose `uses_arguments` walk was Rev 2's model): the
    visitor's population is what it visits, not where the compiler emits. At `3cbd5b88` `visit_class_body` visits no
    member key while `compile_class` (`compiler/expr_class.rs`) compiles computed keys and static field initializers
    in the enclosing frame (`DefineComputed*`, the static `Property` arm), so `() => class { [this.k]() {} }` would leave
    the arrow `Unobserved` and fail at `Op::PushThis` (`VmError::internal`; it works today). Conversely the visitor
    walks through a static block, which the compiler emits as its own non-arrow function (a child `FunctionCompiler`,
    called with the class as `this`), so it would over-mark the arrow (R1 A2 MIN). Defined at emission, both are right
    by construction: every position the compiler emits into is covered, and only emitted reads count.
  - **The funnel holds by type** (Rev 3.2, re-check A2 MIN-2). At `3cbd5b88` `FunctionCompiler::bytecode` is `pub`
    (`compiler/function.rs:19`) and other modules touch it: operand bytes after an emit (`expr_object.rs:40`, `:45`,
    `expr_class.rs:390`, `:409`, `:650`, each the flags byte of `DefineGetter` / `DefineSetter` / a define op), a
    jump patch (`stmt.rs:619-620`) and trailing-op reads (`expr_function.rs:255-256`, `:326-327`, `mod.rs:126-127`);
    `git grep -n -P '\.bytecode\b' 3cbd5b88 -- crates/script/elidex-js/src/compiler ':!*/function.rs'` → these 13
    lines (control: `function.rs:134`, `self.bytecode.push(op.to_byte())`, hits without the exclusion). None writes an op
    byte today, but nothing stops the next one. R-JS-1 makes the field private to `compiler/function.rs`: the flags
    bytes go through the existing `emit_u16_u8` (op, index, flags), the switch patch through a `patch_jump_to(pos,
    target)` beside `patch_jump`, the reads through one `ends_with_return()`. Then every byte of a
    `FunctionCompiler`'s bytecode is written in `compiler/function.rs`, and every op byte there through `push_op`.
  - Bytes outside any `FunctionCompiler`: `synthesize_default_class_ctor`'s literal bytecode (`compiler/expr_class.rs`,
    `Op::SuperCallSpread as u8`) builds non-arrow functions only, whose flag is `false` by definition (§4.1 DoD).
- **`create_closure` (`vm/ops.rs`):** for an arrow with the flag it calls `box_this_environment()` (`vm/frame.rs`,
  §1.3.1): if the current frame's `this_env` is `Inline(r)`, allocate `ObjectKind::ThisEnvironment(r)` and set the
  frame to `Cell(id)`; if it is `Cell(id)`, reuse `id`; `Unobserved` is impossible by the transitive flag
  (`debug_assert!`, `VmError::internal` in release). The arrow's
  `FunctionObject::this_env` (NEW) `= Some(id)`, replacing `captured_this`. Rooting: during the cell's allocation the
  binding's value is still in the frame (`Inline`); between the cell's allocation and the frame write there is none;
  the function object's allocation follows with the cell held by the frame.
- **Call:** `compute_this_for_call` (`vm/dispatch_ic.rs`) becomes `this_env_for_call(this_mode, receiver, this_env) ->
  FrameThisEnv` (`Strict` → `FrameThisEnv::bound(receiver)`; `Lexical` → `FrameThisEnv::lexical(this_env)`). Its five callers
  (`ic_call` ×2, `ic_call_method` ×2, the Function arm, in `call_dispatch_rooted` since #653; `git grep -n 'compute_this_for_call(' 7af70b68`
  → 6 lines: these 5 and the definition) pass the result to `push_js_call_frame` / `call_internal`, whose `this`
  parameter becomes `FrameThisEnv`. The other frame-entry writers pass a bound value without that function:
  `do_new` (`vm/ops.rs`, `push_js_call_frame` with the pre-allocated instance), `construct_synchronous`
  (`vm/dispatch_class.rs`, its `push_js_call_frame` with the receiver), `run_function` (`vm/interpreter.rs`,
  `call_internal` with `FrameKind::Eval`, a non-arrow frame): each passes `FrameThisEnv::bound(v)`.
- **Accessors (`VmInner`, NEW, in `vm/frame.rs`'s block, §1.3.1), each resolving `Inline` / `Cell`.** R-JS-1 has
  two: `this_binding(&self, env: &FrameThisEnv) -> Result<Result<JsValue, VmError>, VmError>` (outer: the VM
  invariant; inner: GetThisBinding's own completion, the value or a throw; by the env, not a frame index, so it also reads a frame already popped: the return step, §4.2) and the
  transitional `overwrite_this_for_pre_allocation(frame_idx, v)` (below). R-JS-3 adds the third, `bind_this(frame_idx,
  v) -> Result<(), VmError>` (BindThisValue: ReferenceError when already initialized), **with its first caller**
  (SuperCall step 9, §4.3.3), and GetThisBinding's step 2 (`Uninitialized` → ReferenceError) in `this_binding`'s
  body, so no caller of `this_binding` changes. `bind_this` is not in R-JS-1: until R-JS-3 nothing calls it, and an
  uncalled `VmInner` method (`pub(crate)`, `vm/mod.rs:190` at `3cbd5b88`) is dead code under §1.3.1's rule (PM 892).
  Every reader of the frame's `this` goes through `this_binding` (`Op::PushThis`, `dispatch_super_inner`, the return
  step); `create_closure` reaches the record only through `box_this_environment`; outside `vm/frame.rs` no code
  matches `FrameThisEnv` (private representation; `mark_frame` matches `gc_edge()`'s `ThisEnvEdge`, §4.1).
- **An `Unobserved` read is a VM invariant violation, never a script exception** (re-check A2 MIN-3). It is
  `this_binding`'s **outer** `Err(VmError::internal)` (after a `debug_assert!`); **`Op::PushThis` propagates it with
  `?`** and routes only the inner (spec) `Err` through `throw_error`, the shape of `Op::CheckTdz` beside `peek()?`.
  Reason: `throw_error` → `vm_error_to_thrown` makes `InternalError` a catchable plain `Error` (`vm/ops.rs`, the `_ =>
  "Error"` arm), so script could catch a broken emission invariant and run on; and the same invariant met in
  `Op::Closure` (`box_this_environment`'s `Unobserved`) already propagates uncatchable (`self.create_closure(…)?` at
  `3cbd5b88`): one invariant, one outcome. `do_new`'s convention (every `Err` through `vm_error_to_thrown`) is not
  copied: it mixes the kinds only because its helpers return one `VmError`. The other callers read frames that are
  never `Unobserved` — `complete_construct` a construct frame (arrows are not constructors), `dispatch_super_inner` a
  `SuperCall`'s frame (`Reads`, so an arrow holding one is `Cell`) — and `?` the outer into their own `Result`.
  Whether `throw_error` should refuse `InternalError` VM-wide is not R-JS's (every helper's); reported to the PM (§9).
- **The transitional writer (R-JS-1 → R-JS-3).** Under R-JS-1 and R-JS-2 the pre-allocation contract stands, so
  `dispatch_super_inner` replaces `Initialized(pre_alloc)` with `Initialized(result)`, which BindThisValue refuses by
  design (step 2; R-JS-3's `bind_this`). That one write is `overwrite_this_for_pre_allocation(frame_idx, v)` (NEW, `VmInner`, doc: "today's swap,
  not BindThisValue; deleted by R-JS-3"), with one caller; R-JS-3 deletes it and wire #5's extension checks it is gone
  (§4.3 DoD). It writes the record, so arrows see the swap (C1-10, C1-11).

**ECS / VM idiom table** (OO → ECS lens; the VM's own idioms are the ECS terms here):
| Spec / OO notion | elidex |
|---|---|
| Function Environment Record (an object per call, linked by [[OuterEnv]]) | the frame's inline `ThisEnvironment`; boxed into a cell only when an arrow observes it |
| GetThisEnvironment's outward walk | resolved once at arrow creation (an arrow in an arrow inherits the handle), so every read is one hop |
| an arrow closing over its environment | `FunctionObject::this_env`, beside `upvalue_ids`; one handle for every §9.4.3 consumer |
| class-owned mutable `this` state | the record; one writer (BindThisValue), read by accessor |

No other engine's structure is a ground for this choice; none is named.

#### §1.3.4 The record and umbrella C (PM 883 D1): shape (i)
**Shape: (i) — `ObjectKind::ThisEnvironment` is the first record kind of umbrella C's representation, and C extends
it** (C = slice-plan row **C**, slot `#11-vm-declarative-environment-and-capture-representation`, repo ledger row by
id; its derivation inherits this kind). Reason (rung 1): §9.1.1.3 says "A Function Environment Record is a Declarative
Environment Record" (`webref body ecma262 sec-function-environment-records`), and the this-binding is that record's
Table 17 state. A this-capture beside C's capture would be two mechanisms for one record (One issue, one way).
- **Why not (ii)** (two distinct mechanisms, with an N=2 reason): an N=2 reason would need the this-part and the
  declarative bindings to differ in lifetime, sharing or capture. They do not: both are per invocation, shared by the
  frame and the closures created in it, captured through the closure, and both have an uninitialized state whose read
  throws a ReferenceError (GetThisBinding step 2; GetBindingValue §9.1.1.1.6 step 2). Only *which* closures capture
  differs (arrows only for the this-part): that is the population of one capture, not a second mechanism. No reason
  that survives C's derivation can be written.
- **What R-JS-1 fixes for C, and what it leaves open.** Fixed: a function environment's this-part is one heap record
  of the object arena, shared by the frame and the closures that capture it, reached through the closure's one handle
  (`FunctionObject::this_env`). Open, C's to derive: the declarative part (named bindings and their states), the
  storage policy (inline-until-observed may become C's rule for every binding, or C replaces it for all), the kind's
  name and field layout (renaming `ThisEnvironment` to a function-environment kind is C's), and whether the handle
  becomes the closure's whole [[Environment]] chain.
- **Constraints on C** (the edge, a landing-list line on C's slot, §9): C does not mint a second record kind for
  function environments nor a second closure-held handle for the this-part; if C's derivation finds the kind's shape
  wrong, C migrates it in its own PR (one way, no coexistence); R-JS-1's cells C1-1, C1-2, C1-3, C1-3c, C1-6, C1-7, C1-8, C1-9, C1-10,
  C1-11, C1-12, C1-13, C1-14, C1-15 are C's regression suite for the this-part, and C1-7 / C1-9 are its I-7 sole-root cells for this kind.
- **R-JS-1 does not wait for C**: it mints no named-binding state and no upvalue state, so nothing C must derive is
  pre-empted; C's derivation starts from the kind this plan fixes. No order between C and R-JS-1 is set here.

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
| B1×B2 | the accessors (§1.3.3; `bind_this` from R-JS-3) resolve `Inline` / `Cell`; BindThisValue writes the shared cell, so an arrow created before `super()` reads the bound value (E-4) |
| B1×B3 | the return step reads the binding through `this_binding(&frame.this_env)` on the popped frame (§4.2); for a frame whose record was boxed it reads the cell, the same value |
| B1×B4 | the entry pushes the frame with `FrameThisEnv::bound(thisArg)` (base) or `::uninitialized()` (derived); nothing else creates a record |
| B1×B5 | the cell is an object traced by one `trace_work_list` arm (its binding's value); frames mark `Cell(id)` and `Inline(Initialized(v))` in `mark_frame` (R-0b); `FunctionObject::this_env` is marked in the Function arm (replacing `captured_this`, `vm/gc/trace.rs`) |
| B1×B6 | an upgrade constructor's arrows observe the record like any other (no CE knowledge in the record) |
| B1×B7 | boxing allocates in `create_closure`, no Rust recursion; no CP |
| B1×B8 | `observes_this_environment`, set at emission (§1.3.3), decides which arrows hold a handle; a non-observing arrow keeps nothing alive (today every arrow keeps its `this` alive through `captured_this`); computed keys and static field initializers emitted into an arrow count (C1-12 … C1-14) |
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
| B6×B7 | P0a-2b's D-3 row (the pre-read) loses its site with S-2; D-4's `Err` arm stays, on `VmInner::construct`'s result once R-JS-3 deletes `construct_synchronous` (umbrella A6×A7; reported to P0a-2b's owner, §4.3 DoD) |
| B6×B8 | `[[ConstructorKind]]` decides the upgrade constructor's `this`: a class `extends HTMLElement` is derived (uninitialized, bound by `super()` to the element, C3-11 / C3-14), a plain function is base (a fresh Ordinary from its `prototype`, which is not the element, so step 11.4's TypeError after it ran, C3-12) |
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
| E-5 GC | C1-7 (arrow's cell after the constructor returns), C1-8 (box allocation under GC), C1-9 (a suspended generator frame's `Cell` as the record's sole root) | R-JS-1 |
| E-6 upgrade | C3-11 (`setPrototypeOf(C, Blob)`: TypeError at 11.4, wrapper kind/brand untouched), A-1 green; L3's A-2 … A-8 re-expected | R-JS-3 |
| E-8 step position | stated: the native arm's pre-allocation stays before argument conversion (umbrella E-8); no cell | R-JS-3 |
| E-9 CP-1 | C3-18 (P0a-1's `rl_cp1_entry_failure_no_callee_effect` through the entry: no callee effect, no allocation) | R-JS-3 |
| E-10 seam | the call points (base: after the frame push; derived: after BindThisValue); acceptance by the two-call-site count (§4.3 DoD) — the body is Slice 2ab's | R-JS-3 |
| R-1 `do_new` window (`#11-vm-do-new-instance-rooting`, registered) | C3-9 (red at k = 1 today, green at k = 1, 2 after) | R-JS-3 |
| R-2 sync return reads the caller's pre-allocation (C-4) | C2-6 (red measured at `378633f6` → green), C2-7 (the error surfaces at the caller's `super()`) | R-JS-2 |
| R-3 `[[ConstructorKind]]` for `extends null` (C-3) | C2-8 (red: instance → green: TypeError), C2-9 (literal / parenthesised parity), C3-3b (`extends null` with an empty user constructor: ReferenceError after R-JS-3) | R-JS-2, R-JS-3 |
| R-4 §10.4.1.2 newTarget remap (C-2) | C3-21 (`new BF()` keeps today's target, control), C3-7b (Rust `Construct(BF, [], X)` keeps X; `Construct(BF, [], BF)` gives the target) | R-JS-3 |
| R-5 return-step errors are the caller's (§10.2.2 step 9 before 14 / 15) | C2-5 (a `try` inside the constructor does not catch step 14's TypeError) | R-JS-2 |
| R-6 upgrade under GC (the slot's second item) | C3-17 | R-JS-3 |
| R-7 frame-first push | C3-22 (ordinary rest call under GC; guard, green both) | R-JS-3 |
| R-8 record sharing | C1-11 (two arrows share one cell after `super()`) | R-JS-1 |
| R-9 observation at emission (§1.3.3) | C1-12 (computed member key), C1-13 (static computed key), C1-14 (field initializers), C1-15 (controls: no mark through a static block or a method; raw-byte default constructors) | R-JS-1 |

## §3. Spec coverage map
| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| ECMA-262 §13.3.5.1.1 EvaluateNew ( constructExpr, argumentsNode ) | 5-6 | IsConstructor (unchanged admission), then Construct with newTarget absent | `do_new` → the entry (R-JS-3) | ✓ | yes |
| ECMA-262 §7.3.14 Construct ( ctor [ , argList [ , newTarget ] ] ) | 1-3 | newTarget absent → ctor | the entry (NEW, R-JS-3) | ✓ | yes |
| ECMA-262 §10.2.2 [[Construct]] ( argList, newTarget ) | 2-17 | base (3, 6) / derived; Object / base / non-`undefined` / `undefined` return (12-15) | entry JS arm (R-JS-3); return step (R-JS-2) | ✓ | yes |
| ECMA-262 §10.2.1.1 PrepareForOrdinaryCall ( func, newTarget ) | 7, 12 | the frame push, after §10.2.2 step 3's creation | `push_js_call_frame` (frame-first, R-JS-3) | ✓ | no |
| ECMA-262 §10.2.1.2 OrdinaryCallBindThis ( func, calleeContext, thisArg ) | 2, 5, 9 | lexical (arrow: no bind) / strict; step 6 (non-strict) is outside core | `this_env_for_call` (R-JS-1); base construct binds at push (R-JS-3) | ✓ | yes |
| ECMA-262 §10.3.2 [[Construct]] ( argList, newTarget ) | 1-2 | R-JS stage: a fresh Ordinary from newTarget as `this` (umbrella §4.2); R-NAT makes it uninitialized | entry native arm (R-JS-3) | ✗ (R-NAT's flip) | yes |
| ECMA-262 §10.4.1.2 [[Construct]] ( argList, newTarget ) | 4-6 | step 5: newTarget is a link of the chain → the target; else unchanged | `admit_construct`'s one chain walk (its visitor given each link) reports it; the entry uses the remapped newTarget (C-2, §4.3.1) | ✓ | yes |
| ECMA-262 §10.1.14 GetPrototypeFromConstructor ( ctor, intrinsicDefaultProto ) | 2-6 | Object / non-Object (single realm: step 5 GetFunctionRealm is the VM's one realm) | helper phase 1 (NEW, R-JS-3) | ✓ | yes |
| ECMA-262 §13.3.7.1 Runtime Semantics: Evaluation | SuperCall 1, 3, 5-9, 12 | step 6 before step 9; step 12 seam; steps 2 / 4 unchanged (1a / 1b) | `dispatch_super_inner` (R-JS-1 rebind, R-JS-3 bind) | ✓ | yes |
| ECMA-262 §13.3.7.2 GetSuperConstructor ( ) | 1-5 | `home_class`'s [[GetPrototypeOf]] (unchanged; umbrella 3 owns the field) | `dispatch_super_inner` | ✓ | yes |
| ECMA-262 §9.1.1.3.1 BindThisValue ( envRecord, value ) | 2-4 | already initialized → ReferenceError | `bind_this` (NEW, R-JS-3, with its one caller, SuperCall) | ✓ | yes |
| ECMA-262 §9.1.1.3.3 GetThisBinding ( ) | 2-3 | uninitialized → ReferenceError | `this_binding` (NEW, R-JS-1; step 2's ReferenceError with `Uninitialized`, R-JS-3); `Op::PushThis`, return step 15 (R-JS-2) | ✓ | yes |
| ECMA-262 §9.1.2.4 NewFunctionEnvironment ( func, newTarget ) | 3-4 | lexical / uninitialized | `FrameThisEnv` at frame push (R-JS-1, R-JS-3) | ✓ | no |
| ECMA-262 §9.4.3 GetThisEnvironment ( ) | 1-2 (2.1-2.5) | resolved at arrow creation, one hop | `create_closure`, `this_env_for_call` (R-JS-1) | ✓ | yes |
| ECMA-262 §9.4.4 ResolveThisBinding ( ) | 1-2 | — | `Op::PushThis` (R-JS-1 accessor, R-JS-3 status) | ✓ | yes |
| ECMA-262 §13.2.1.1 Runtime Semantics: Evaluation | 1 | `this` | `Op::PushThis` | ✓ | yes |
| ECMA-262 §10.2.3 OrdinaryFunctionCreate ( proto, sourceText, paramList, body, thisMode, envRecord, privateEnv ) | 9-10 | lexical (arrow) / strict | `create_closure` | ✓ | no |
| ECMA-262 §15.7.14 Runtime Semantics: ClassDefinitionEvaluation | 14.a.4-5, 14.b, 18 | default ctor derived / base (built-in, 14.b: its [[Construct]] is §10.3.2's, C-4); ClassHeritage present → derived (`null` included) | `compile_class` default ctors, `[[ConstructorKind]]` (R-JS-2) | ✓ | yes |
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
- `vm/frame.rs`: `ThisBinding` (one variant), `ThisEnvironment` (+ `traced_value`), `ThisEnvId` (+ `object_id`),
  `FrameThisEnv` (+ `bound`, `lexical`, `gc_edge`), `ThisEnvEdge` (NEW, §1.3.1; private fields and representation);
  `CallFrame::this_value` → `this_env`; the `impl VmInner` block with `box_this_environment`, `this_binding` and
  `overwrite_this_for_pre_allocation` (§1.3.1, §1.3.3).
- `vm/object_kind.rs`: `ObjectKind::ThisEnvironment` (NEW) variant `ThisEnvironment(ThisEnvironment)` ( doc: internal, never a
  `JsValue`). Every exhaustive `match` over `ObjectKind` gains its arm (population: §10 S-1).
- `vm/value.rs`: `FunctionObject::captured_this` → `this_env: Option<ThisEnvId>`; `JsCalleeInfo::captured_this` →
  `this_env`. `vm/ic.rs`: `CallIC::captured_this` → `this_env`. `vm/ops_property.rs` `collect_call_ic`,
  `vm/dispatch_ic.rs` `extract_js_callee` / `try_call_ic`: field rename.
- `vm/dispatch_ic.rs`: `compute_this_for_call` → `this_env_for_call` (§1.3.3).
- `vm/interpreter_call.rs`: §6.1's table (Function arm expression; `call_internal` / `push_js_call_frame`'s `this`
  parameter type; their frame literals).
- `vm/ops.rs` `create_closure`: the boxing rule (§1.3.3); `captured_this` deleted.
- Frame-entry writers outside `this_env_for_call` (§1.3.3): `do_new` (`vm/ops.rs`), `construct_synchronous`'s
  `push_js_call_frame` (`vm/dispatch_class.rs`), `run_function` (`vm/interpreter.rs`) pass `FrameThisEnv::bound(v)`.
- `vm/dispatch.rs`: `Op::PushThis` pushes `this_binding(&frame.this_env)?`'s value, the inner `Err` through
  `throw_error` (§1.3.3). R-JS-3 changes no arm: GetThisBinding's ReferenceError arrives inside `this_binding` (§4.3.3).
- `vm/dispatch_class.rs` `dispatch_super_inner`: the receiver read and the rebind go through the accessors; under the
  pre-allocation contract the rebind stays today's overwrite, now of the record's binding, through the one
  transitional writer `overwrite_this_for_pre_allocation` (§1.3.3; R-JS-3 replaces it with BindThisValue);
  `new_instance` is still updated (R-JS-2 deletes it).
- `vm/gc/mod.rs` `mark_frame` (R-0b): an exhaustive `match` on `this_env.gc_edge()` (R-0b §4's discipline) —
  `Value(v)` → `mark_value`, `Cell(id)` → `mark_object(id.object_id())`, `Nothing` → nothing; `gc_edge` maps
  `Inline(Initialized(v))` / `Cell` / `Unobserved` (R-JS-3 adds `Inline(Uninitialized)` → `Nothing` with the variant);
  `vm/gc/trace.rs`: the Function arm marks `this_env` instead of `captured_this`; a `ThisEnvironment` arm marks
  `traced_value()`.
- `.claude/tools/native-ctor-guard-trip-wire.sh`: **wire #5 born** (umbrella §4.6), its record half (§1.3.1): no
  `.object_id()` call under `crates/script/elidex-js/src` outside `vm/gc/` (comment lines stripped, as wires #1-#4);
  fail-safe: another type's `object_id()` trips it too (0 hits at `3cbd5b88`); control mutant (a call in
  `vm/dispatch.rs`) in the same commit.
- `vm/natives_generator.rs`, `vm/tests/tests_with_call_mode.rs`, `vm/gc/tests.rs`: struct literals (§10 S-2).
- Compiler (§1.3.3, the emission rule): `bytecode/opcode.rs` `Op::reads_this_environment` (exhaustive, no wildcard;
  precedent `Op::operand_size`, exhaustive at `3cbd5b88`); `compiler/function.rs` `push_op` (every `emit*` helper),
  the flag, its copy in `finish`, `bytecode` private with `patch_jump_to` and `ends_with_return` (§1.3.3; their
  callers in `expr_object.rs`, `expr_class.rs`, `stmt.rs`, `expr_function.rs`, `mod.rs`); `compiler/expr_function.rs` (arrows); `bytecode/compiled.rs` (+ every
  `CompiledFunction {` literal, `git grep -n -P 'CompiledFunction \{' 3cbd5b88` → 11 lines, `bytecode/disasm.rs`
  included). The scope visitor is not touched.
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
  creating frame's `gc_edge()` `Value(_)`; one that has one makes it `Cell(id)` once (two arrows → one `id`, `==`).
- C1-7 (GC) `let f; class A { constructor(){ this.tag = 42; f = () => this } } new A();` then a full collection,
  then `f().tag === 42` — both (a guard for the new trace edge); **mutant**: dropping the Function arm's `this_env`
  mark turns it red (same commit).
- C1-8 (GC) the box allocation: a method on an object reachable only through the call (`mk()` pool pattern of
  `tests_bound_call_gc.rs`) creates an observing arrow; collection at the cell's allocation (k = 1); the arrow returns
  `this.x` — green (new code; no "before").
- C1-9 (GC, sole root, design invariant I-7 in `docs/plans/2026-07-vm-p4-design-invariants.md`) `const it = (function*
  () { void (() => this); yield 1; yield this.tag }).call(mk())`, where `mk()` returns `{ tag: 42 }` and nothing else
  keeps it: the arrow is created (boxing the record) and dropped, no binding holds `this`, so the suspended frame's
  `Cell` is the record's only holder and the record the receiver's. `it.next()`, a full collection, then enough
  allocations to reuse a freed slot (`tests_coroutine_creation_gc.rs`'s pattern), then `it.next().value === 42` —
  both; **mutant**: the frame classifier's `Cell` arm skipped (the one suspended frames go through at the child's
  base, R-0b's `mark_frame`) turns it red. A control asserts the arrow is unreachable (no holder but the dropped temp).
- C1-11 sharing: two arrows created before `super()`, one returns `this` after the constructor returns, under the
  pre-allocation contract they read the same rebound object — green (red under a per-arrow copy, mutant).
- C1-12 computed key (R1 A2): `function o(){ return (() => class { [this.k]() { return 1 } })() }`, `new
  (o.call({k: 'm'}))().m() === 1` — both (today `captured_this`; a visitor-defined flag would make it red).
- C1-13 static computed key: `function p(){ return (() => class { static [this.k]() { return 2 } })() }`,
  `p.call({k: 's'}).s() === 2` — both.
- C1-14 field initializers: an instance field `() => class { x = this }` emits no read in the arrow (instance
  initializers are not compiled today, `compile_class`'s `Property` arm), so the arrow is `Unobserved` (Rust); a static
  field `() => class { static x = this }` is emitted into the arrow, so it captures and completes without
  `VmError::internal`. Its value is today's lexical `this`, not the class (DefineField §7.3.32 step 3.a calls the
  initializer with the class as receiver): that deviation is `#11-step9-class-extras`'s (Slice 2ac, the static
  initializer's receiver), and the cell asserts completion only, re-expected there (landing list A6 / B2 item 4).
- C1-15 controls (Rust): `() => class { static { this } }` and `() => ({ m(){ return this } })` leave the arrow
  `Unobserved` (the read is in a non-arrow child); `() => () => this` marks both arrows; a mutant
  `reads_this_environment(PushThis) = No` turns C1-3c red.

**DoD.** All cells green (pathspec `R=crates/script/elidex-js/src` on every grep, so no doc matches itself);
`git grep -n -w captured_this -- $R` → 0 (18 lines at `3cbd5b88`); `git grep -n -P '\bbox_this_environment\(' --
$R` → its definition (`vm/frame.rs`) and `create_closure` only; the record, the handle and `FrameThisEnv` hold by
type (§1.3.1: `ThisEnvironment`'s, `ThisEnvId`'s and `FrameThisEnv`'s fields have no visibility modifier, so no
other module builds or matches them; reviewed on the diff); wire #5's record half and its mutant (§1.3.1) in CI;
`FunctionCompiler::bytecode` has no visibility modifier, so `git grep -n -P '\.bytecode\b' --
crates/script/elidex-js/src/compiler ':!*/function.rs'` → 0 (control: the 13 lines at `3cbd5b88`, §1.3.3), and `git
grep -n 'to_byte()' -- crates/script/elidex-js/src/compiler` → one write, in `push_op` (reads classified); the remaining raw op bytes in
`compiler/` (`git grep -n -P 'Op::\w+ as u8' -- crates/script/elidex-js/src/compiler`, control: `expr_class.rs`'s
default constructors hit) are the default-constructor literals (non-arrow functions) and reads (`ends_with_return`,
unit-test asserts), classified per hit; `overwrite_this_for_pre_allocation` has one caller; `git grep -n -w
bind_this -- $R` → 0 (it lands with its caller, R-JS-3); `mise run ci` green.

### §4.2 R-JS-2 — the return step and `[[ConstructorKind]]` (axis 3)
**Changes.**
- `bytecode/compiled.rs`: `CompiledFunction::constructor_kind: ConstructorKind { Base, Derived }` (NEW enum);
  `compiler/expr_class.rs` sets `Derived` iff ClassHeritage is present (§15.7.14 step 18), so the literal-`null` arm
  of `classify_heritage` becomes derived and gets the derived default constructor (step 14.a.4); `ClassHeritage`'s
  doc and `extends_null_default_ctor_is_base` are rewritten (the test becomes C2-8). Non-class functions: `Base`.
- `vm/construct.rs` (NEW module; one `mod` line in `vm/mod.rs`): `complete_construct(frame: CallFrame, result) ->
  Result<JsValue, VmError>` (the popped frame by value; it reads the binding with `vm.this_binding(&frame.this_env)`,
  §1.3.3 — no frame index; between the pop and that read nothing allocates, and the step-14 / step-15 errors allocate
  only on paths that discard the binding), §10.2.2 steps 12-17: Object → it; `Base` → the binding's value (step 13 thisArg; the base
  binding is bound once at entry and never rebound); `Derived`: non-`undefined` → TypeError (14), else
  `? this_binding` (15; its ReferenceError arrives with `Uninitialized` in R-JS-3, inside `this_binding`), the value
  (16-17).
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
- `.claude/tools/native-ctor-guard-trip-wire.sh`: wire #5 (born in R-JS-1, §4.1) gains umbrella §4.6's R-JS
  check: `new_instance`, `pre_alloc_instance` → 0 under `crates/script/elidex-js/src`, with a control mutant.

**Cells** (in `vm/tests/tests_construct_return.rs`, NEW):
- C2-1 `class A {} class B extends A { constructor(){ super(); return 1 } }` → TypeError — red (instance) → green.
- C2-2 base `class A { constructor(){ return 1 } } new A() instanceof A`; C2-3 derived `return undefined` → the
  instance; C2-4 derived `return {k: 1}` → that object; plain `function F(){ return 1 }; new F() instanceof F` — both.
- C2-5 `class B extends A { constructor(){ super(); try { return 1 } finally { this.f = 1 } } }`: `new B()` throws
  TypeError at the caller, and a `try`/`catch` around `return 1` inside the constructor catches nothing — red →
  green (R-5).
- C2-6 the C-4 chain, `new C().tag === 1`, with and without explicit `constructor() { super() }` in B and C — red
  (measured `undefined` at `378633f6`, C-4) → green; control: the two-level `new B().tag === 1` — both (measured).
- C2-7 the sync driver: `class C extends B { constructor(){ try { super() } catch (e) { return { te: e instanceof
  TypeError } } } }` with C2-1's B → `new C().te` — red → green.
- C2-8 `class X extends null {}; new X()` → TypeError — red (instance) → green; C2-9 parity: `let h = null; class Y
  extends (h) {}; new Y()` → TypeError (both), and `class Z extends null { constructor(){ return
  Object.create(Z.prototype) } } new Z() instanceof Z` (both).

**DoD.** Cells green; wire #5's `new_instance` / `pre_alloc_instance` check and its mutant; `git grep -n -w -e
new_instance -e pre_alloc_instance -- crates/script/elidex-js/src` → 0; `mise run ci` green.

### §4.3 R-JS-3 — the construct entry (axis 2)
#### §4.3.1 The entry
`vm/construct.rs` (R-JS-2's module) gains the entry; the umbrella §4.3 helper module (NEW, beside
`vm/construct_admission.rs`; name final in the PR) gains phase 1:
`get_prototype_from_constructor(new_target, default_proto) -> Result<ObjectId, VmError>` (§10.1.14: a full `? Get`,
so a getter runs; non-Object → the default). Docstrings name the AO, numbers from `webref` (umbrella §4.3).

- **Admission** (EvaluateNew step 5 / SuperCall step 5 / Web IDL §3.12 step 3): `admit_construct`, outside
  `[[Construct]]`; its IsConstructor classification is unchanged, and it passes newTarget through (next bullet, RS-2).
- **§10.4.1.2 step 5**: if newTarget is the callee as passed or any bound link of its chain, newTarget becomes the
  target; otherwise unchanged (C-2). **Mechanism: the visitor's signature changes, no second walk** (R1 A3 MIN).
  `walk_bound_chain` (`vm/dispatch_class.rs`) is the one depth loop for both forms; its visitor at `3cbd5b88` receives
  each link's `bound_args` only. It becomes `FnMut(ObjectId, &[JsValue])` (the link and its arguments);
  `unwrap_bound_function_chain` takes `new_target: Option<ObjectId>` (absent = the callee, Construct step 1) and
  returns the remapped one beside `(target, prepended)`, testing each link in the same pass; `bound_chain_target`'s
  closure ignores the link. `admit_construct` passes newTarget through (RS-2: it is not "unchanged"; its IsConstructor
  classification is). A second walk would be a second traversal of one chain for one [[Construct]] — two places that
  must agree on depth and order — where the step is a per-link test the existing walk already visits.
- **JS callee** (`ObjectKind::Function`):
  - `Base`: `proto = ? get_prototype_from_constructor(newTarget, %Object.prototype%)` (§10.2.2 step 3), `thisArg =`
    OrdinaryObjectCreate(proto) (in R-JS an `Ordinary` allocation; R-NAT's phase 2 is not needed for it), the frame
    pushed with `FrameThisEnv::bound(thisArg)` (steps 4-6.a), then the InitializeInstanceElements call point (6.b,
    `initialize_instance_elements(thisArg, func)` (NEW), body Slice 2ab's, `#11-step9-class-extras`); if it is
    abrupt, the frame is popped and the error is the caller's (6.c: calleeContext removed, `? initializeResult`).
    **In R-JS-3 the body is empty, and says so**: elidex functions carry no [[Fields]] / [[PrivateMethods]] yet
    (instance field initializers are not compiled, `compile_class`'s `Property` arm), so steps 1-4 iterate nothing
    and the function returns `Ok(())`; its docstring names the AO, the slot and that its parameters (`_obj`,
    `_ctor` until then: an unread parameter is a warning under `-D warnings`) are read by 2ab's body. **Form**
    (re-check A3 MIN-1): a free `pub(super) fn initialize_instance_elements(_obj: ObjectId, _ctor: ObjectId) ->
    Result<(), VmError>` in `vm/construct.rs`, not a `VmInner` method: with no `self` the workspace's pedantic
    `clippy::unused_self` (`Cargo.toml:312`) cannot fire, and the always-`Ok` `Result` is the AO's completion
    (6.c reads it), under the `vm` module's existing `unnecessary_wraps` allow (`lib.rs:37`); no `#[allow]` is added.
    2ab makes it a method when its body first reads the VM (its two call sites gain `self.`). It is the seam umbrella §2.2 E-10 ratifies (two callers, the two-call-site DoD), not an unwired stub;
    6.c is unreachable until 2ab, and is written now because it belongs to the call point. The
  `compile_class` markers this seam makes stale (`compiler/expr_class.rs`, the `Property` arm's "for simplicity we skip
  instance field initializers for now" and "Non-static properties: skip") are retagged to `#11-step9-class-extras` in
  R-JS-3 (that slot's adoption terms ask for the in-code retag).
  - `Derived`: no allocation, no `Get`; the frame pushed with `FrameThisEnv::uninitialized()`.
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
  (the native arm makes it), `dispatch_super_inner`'s receiver read, `overwrite_this_for_pre_allocation` (§1.3.3),
  `invoke_upgrade`'s receiver and S-2's two statements. Wire #5 extended: no construct-entry signature takes a `this`
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
- `ThisBinding::Uninitialized` (NEW here, §1.3.1) with its constructor `FrameThisEnv::uninitialized()` and its caller, the derived frame push (§4.3.1), and
  GetThisBinding's step 2 in `this_binding`'s body (`Uninitialized` → ReferenceError, the inner `Err`); the exhaustive
  matches in `vm/frame.rs` gain their arms (`gc_edge`, `traced_value`, the accessors). `Op::PushThis`: R-JS-1's
  arm, unchanged; the ReferenceError comes out of `this_binding` and is thrown via `throw_error` (catchable in the
  frame); the return step's step 15 likewise.
- `dispatch_super_inner`: `result = construct(superCtor, args, newTarget)` (step 6; errors in the derived frame), then
  `bind_this` (step 9, NEW here with this caller, §1.3.3; ReferenceError in the derived frame), then `initialize_instance_elements(result, func)` (12),
  then push `result`. Nothing before `construct` mutates state (keeps P0a-1's reading, umbrella §8.1).

#### §4.3.4 The upgrade (`vm/host/custom_elements/upgrade.rs` `invoke_upgrade`)
- Phase 2's `prototype` `Get` with its two failure exits and Phase 3's `set_wrapper_prototype` call go (S-2);
  `create_element_wrapper` stays (umbrella §5.3).
- Phase 4 calls `ctx.vm.construct(JsValue::Object(constructor), &[], None)` (Web IDL §3.12 step 10).
- Step 11.4: `SameValue` failure throws `VmError::type_error` (PM 805); `finalize_failure_shim`'s call unchanged.
- **Layering** (CLAUDE.md "VM host/ は engine-bound 責務のみ"): the upgrade's state machine stays in the engine-
  independent crate — `elidex_custom_elements::{prepare_upgrade, enter_constructor, finalize_success,
  finalize_failure}` (called from `invoke_upgrade` at `3cbd5b88`); R-JS-3's host touches are marshalling only (the
  `construct` call, the 11.4 `SameValue` and its TypeError, the comment cites) and the `html_element.rs` docstrings.
  The `data_transfer` cell (C1-2) is a test target, not a host change.
- Comments stating the retracted D-17b §6 invariant are rewritten (umbrella §4.2 "Scope", 13 lines by its command at
  `4b723d4b`, re-run at the child's base), and R-JS's 8 P-11 cites (umbrella §1.4 P-11, re-anchored by symbol).

#### §4.3.5 The upgrade path's rooting (`#11-vm-do-new-instance-rooting`'s second item)
Measured by code read at `7af70b68`, re-read at `3cbd5b88`: the definition's constructor is rooted through
`HostData::ce_constructors` (in `gc_root_object_ids`, `vm/gc/roots.rs:350`); the element wrapper is a `StrongRoot` in
`wrapper_store` (`mark_roots` (e), `:386`). After R-JS-3 the
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
  collection at k = 1, 2, 3 → upgrade succeeds, `el instanceof C`, `el.n === 0` — both expected (code read §4.3.5).
  A red result is a defect R-JS-3 fixes in its own PR (owner VM P4, by 12-05): the slot closes only with C3-17 green;
  a fix that needs a decision this memo does not make returns R-JS-3 to plan-review (§1.2), and the slip rule of §8
  applies — the slot stays open, it is never closed on a red cell.
- C3-18 E-9: P0a-1's `rl_cp1_entry_failure_no_callee_effect` re-pointed to `construct`; adds "no allocation"
  (`objects.len() - free_objects.len()` unchanged) — green.
- C3-21 `function F(){ this.nt = new.target }; new (F.bind(null))().nt === F` — both.
- C3-22 R-7 guard: `(function(...r){ return r.length })(1, 2) === 2` under a collection at k = 1 — both.

**DoD.** Cells green; `#11-vm-do-new-instance-rooting` closed (C3-9; the upgrade item answered by C3-17);
`#11-vm-derived-constructor-this-binding` retired (§7.1); wire #5 extended with its mutant (no construct-entry `this`
parameter; `overwrite_this_for_pre_allocation` → 0); `git grep -n 'initialize_instance_elements('` → the definition
and two calls; `git grep -n -P '\bbind_this\('` → the definition and `dispatch_super_inner`'s call; `git grep -n -P
'\bFrameThisEnv::uninitialized\('` → the derived frame push only (`ThisBinding` is private to `vm/frame.rs`); `git grep -n -w construct_synchronous` → 0;
each grep with `-- crates/script/elidex-js/src`; **one GetPrototypeFromConstructor** (R1 A3 IMP): if
`#11-vm-generator-instance-prototype-from-function` landed first with its own `? Get` of `prototype` in
`vm/natives_generator.rs` (EvaluateGeneratorBody's OrdinaryCreateFromConstructor), R-JS-3 converges it onto the
phase-1 helper, and the census of Rust-side `prototype` reads for instance creation in `vm/` (S-6) lists the helper
and its callers only (`html_element.rs` `resolve_validated_prototype` is R-NAT's, umbrella §5.3 S-1); P0a-2b's D-3
row has no site and D-4 reads `construct`'s `Err` (both reported to its owner); `mise run ci` green.

## §5. Files and 1000-line verdicts
`wc -l` at `7af70b68`, the two files #653 changed re-measured at `378633f6`; #654 changed none of them (`3cbd5b88`,
§12 M-3). Files over 1000:

| File | Lines | Touch | Verdict |
|---|---|---|---|
| `vm/mod.rs` | 2787 | R-JS-2: one `mod construct;` line; R-JS-3: the helper module's `mod` line | no seam for this touch (umbrella §7 row; `#11-vm-inner-substruct-decomposition`) |
| `vm/object_kind.rs` | 1646 | R-JS-1: one variant (+ doc) in the `ObjectKind` enum | **no split**: the enum is a flat case table (CLAUDE.md 1000-line rule's exemption) and the touch is one variant; the umbrella's conditional row resolves here. `HostObject`'s definition untouched (PM 642 (a) not fired) |
| `vm/gc/trace.rs` | 1265 | R-JS-1: the Function arm's `this_env` mark and the `ThisEnvironment` arm | R-0b's seam lands first (§8); the remainder is the exempt `ObjectKind` match (umbrella §7) |
| `vm/dispatch.rs` | 1105 | R-JS-1/3: `Op::PushThis`; R-JS-2: `complete_inline_frame` leaves (net −17) and the three entry-frame return sites call the step | no prereq split: the opcode match is exempt and the file shrinks (umbrella §7) |
| `vm/tests/tests_custom_elements.rs` | 1321 | R-JS-3: comment lines only | no seam for this touch; new cells go to `tests_construct_upgrade.rs` |

Under 1000 and touched: `vm/value.rs` 968 (field renames, net 0), `vm/ops.rs` 939, `bytecode/opcode.rs` 577,
`vm/natives_generator.rs` 671 (at `378633f6`), `compiler/expr_class.rs` 653, `vm/gc/roots.rs` 618, `vm/interpreter_call.rs` 613 (at `378633f6`, after #653),
`vm/ops_property.rs` 572, `compiler/function.rs` 473, `vm/host/custom_elements/upgrade.rs` 461,
`vm/dispatch_class.rs` 391, `vm/tests/tests_class_machinery.rs` 391, `vm/host/custom_elements/html_element.rs` 344
(docstrings), `vm/dispatch_ic.rs` 342, `compiler/expr_function.rs` 338, `vm/interpreter.rs` 311, `bytecode/compiled.rs`
231, `vm/frame.rs` 221 (gains the record types and the one `impl VmInner` block, §1.3.1), `vm/gc/mod.rs` 122,
`vm/ic.rs` 60; Rev 3.2's funnel (§1.3.3, R-JS-1, at `3cbd5b88`): `compiler/stmt.rs` 712, `compiler/mod.rs` 354,
`compiler/expr_object.rs` 189 (one-line call changes each). New: `vm/construct.rs`, the helper module, four test files.
`vm/ops.rs` gains the boxing branch (R-JS-1) and loses `do_new`'s pre-allocation (R-JS-3); it stays under 1000 by the
child's own measure, which each PR re-runs.

## §6. Overlaps
### §6.1 PM 642 (c): `vm/interpreter_call.rs`, per function (per arm for `call_dispatch`)
Read at `3cbd5b88` (#653 and #654 landed; every child's base contains them, §8). Decided: PM 880 Q-2, PM 883 D2;
umbrella erratum §0.3 RS-1 (the umbrella's §8.1 rows carry the same anchors).

| Function | R-JS-1 | R-JS-2 | R-JS-3 |
|---|---|---|---|
| `call_dispatch` (`:29`, #653's rooting wrapper) | none | none | none |
| `call_dispatch_rooted` (`:46`) — `ObjectKind::Function` arm (`:64`) | `compute_this_for_call(…, fo.captured_this)` (`:68`) → `this_env_for_call(…, fo.this_env)`; nothing else | none | none |
| `call_dispatch_rooted` — `NativeFunction`, `BoundFunction`, `PromiseResolver`, `PromiseCombinatorStep`, `PromiseFinallyStep`, `AsyncDriverStep`, `ReadableStream{Start,Pull,Cancel}Step`, `WasmExportedFunction` arms, the fallthrough | none | none | none (the entry passes the native's `this`, umbrella §1.2) |
| `call_internal` (`:305`) | `this: JsValue` → `this_env: FrameThisEnv`; the frame literal's `this_value` (`:399`) → `this_env` (the coroutine frame is a struct update of it) | its one `new_instance: None` (`:415`) deleted | none |
| `push_js_call_frame` (`:474`) | `this` parameter type; frame literal (`:569`) | `new_instance` parameter (`:480`) and field (`:581`) deleted | non-coroutine path: frame pushed before the rest-array allocation (§4.3.2 item 3) |

Callers of `call_internal` stay `call_dispatch_rooted` and `run_function` (#653's precondition holds; R-JS adds no
caller). The `ObjectKind::HostObject` definition is untouched by all three.

### §6.2 Stack-safety, P0a-2b, 1aa / 1ac, umbrella 3, S / N, C
- **P0a-1** (umbrella §9.1 step 5 dates its implementation 11-28; step 6 reads that date as its landing; R-JS-1
  starts the day it lands, R-0b being on main by then, §8 rule (a)): its cell
  `rl_cp1_entry_failure_no_callee_effect` enters through `call_construct_native` / `construct_synchronous`; R-JS-3
  re-points it to `construct` (mechanical, umbrella §8.1). `with_call_mode`: unchanged (R-JS calls it).
  `dispatch_super_inner`: nothing mutates before `construct` (P0a-1's reading kept).
- **PR-1 / PR-1b**: re-read `push_js_call_frame` / `call_internal` anchors at their plan-review (umbrella §8.1).
- **P0a-2b**: D-3 (the pre-read) loses its site; D-4's `Err` arm stays but reads `VmInner::construct`'s result, since
  R-JS-3 deletes `construct_synchronous` (both reported to its owner, §4.3 DoD).
- **1aa / 1ac**: 1aa's SuperCall rows (step 6 on `construct_synchronous(…, receiver, …, pre_alloc)`; steps 7-9 ✗) are
  closed by R-JS-2 / R-JS-3; 1ac's `stack_slice` drains in `call_internal` / `push_js_call_frame` are other statements;
  the later rebases (umbrella §4.2).
- **Umbrella 3**: `home_class` and its `None` in `call_internal` untouched; §1.3.2 leaves the home object room in the
  record. **S / N**: consume the record (§1.3.2). **C**: extends the record kind (§1.3.4, §7.2); **`#11-vm-upvalue-tdz-check`**:
  no edge (the record uses no upvalue state).
- **R-0b**: R-JS-1 classifies `this_env` in `mark_frame`; R-JS-2 removes `new_instance` from it. R-0b's §5.4
  obligation (re-point the umbrella's two-site cells to `mark_frame`) has a trigger and a date (§9).

## §7. Slot homing
### §7.1 Closed
| Slot | Ledger | Closed by | Note |
|---|---|---|---|
| `#11-vm-derived-constructor-this-binding` | repo ledger (row by id); PM pointer row | R-JS-3's landing, after R-JS-1 / R-JS-2 | the derivation is this memo; its four regressions and base controls are placed: C3-1, C3-2, C3-3, C2-1, C2-2, C3-4 |
| `#11-vm-do-new-instance-rooting` (registered 2026-11-21, PM 875 (1); Re-eval 12-05) | PM ledger (open row) | R-JS-3 | C3-9; its upgrade item answered by C3-17 |

### §7.2 Edges changed
| Slot | Change |
|---|---|
| `#11-ce-upgrade-returned-object-typeerror` | R-JS-3 takes the error type and its test (C3-13), per PM 805; the two facets stay L3's (PM 808) |
| `#11-vm-native-ctor-receiver-promotion-admission` | both vectors removed by R-JS-3 (umbrella §4.2); the class closes with R-NAT |
| `#11-vm-get-prototype-from-constructor-fallback` | R-JS-3 discharges the reachable facet (C3-6); R-NAT the rest |
| `#11-vm-arrow-lexical-super` (S), `#11-vm-arrow-lexical-new-target` (N) | the `[[ThisBindingStatus]]` mint S waits on is R-JS-1's record; S / N add fields, not a capture route (§1.3.2) |
| `#11-step9-class-extras` (Slice 2ab) | R-JS-3 places the two InitializeInstanceElements call points |
| umbrella C (`#11-vm-declarative-environment-and-capture-representation`) | **new edge** (PM 883 D1, shape (i), §1.3.4): `ObjectKind::ThisEnvironment` is C's first record kind; C's derivation extends it and mints no second function-environment kind or this-capture. Slice-plan row C's classification is rewritten here (RS-6); the slot row's line is in the landing list (§9) |
| `#11-vm-upvalue-tdz-check` | **no edge**: the record uses no upvalue state (umbrella §5.2's row asked the plan to record which owns the state: the this-binding's uninitialized state is the record's, the named bindings' is C's) |
| `#11-vm-generator-instance-prototype-from-function` (PM 850; trigger met, Re-eval 12-05) | its fix (EvaluateGeneratorBody step 2's OrdinaryCreateFromConstructor) calls R-JS-3's phase-1 helper if it lands after R-JS-3; if it lands first with its own `? Get` in `vm/natives_generator.rs`, **R-JS-3 converges it onto the helper** (§4.3 DoD) — one GetPrototypeFromConstructor, no strangler |
| `#11-vm-ecma262-ao-section-renumber-sweep` | fires on `vm/ops.rs` (2 sites) in R-JS-1 |
| `#11-vm-call-internal-callee-this-rooting` (D, #653) | closed by #653 (`378633f6`); every child's base contains it; R-JS adds no `call_internal` caller |

### §7.3 What must not ship
- Pre-allocation removed for derived callees while arrows still snapshot (`this` uninitialized in a by-value copy).
- A guard before `Construct` for the double `super()` (umbrella E-2).
- A cell per non-arrow call, or an arrow capturing the record when its body has no §9.4.3 consumer.
- An arrow left `Unobserved` while its function emits a reading op (the population is the emission, §1.3.3), or an op
  added to `Op` without a `reads_this_environment` classification.
- A this-capture beside the one C extends (§1.3.4).
- Any stack guard held across an inline frame push (#651).
- The return step applied only to one driver.

## §8. Dates (umbrella §9.1, canonical)
The rule (§9.1 step 6): R-JS's implementation starts at the later of P0a-1's landing (11-28, step 5) and the end of
this plan's review (seven days after the umbrella's ratification); 11-28 holds while the umbrella was ratified by
11-20; each day past 11-20 moves step 6 and its dependants. **The umbrella was ratified on 11-18 (PM 818), so nothing
moves**: this plan 11-21 … 11-23, its plan-review 11-24 … 11-27, implementation 11-28 … 12-04, R-JS (= R-JS-3's
landing) **12-05**. Prerequisites on main before R-JS-1's implementation: R-0b (11-24, step 3), #653 (landed
`378633f6`, 11-21), P0a-1 (11-28, step 5).

**Child dates (PM 880 Q-1: set here, inside 11-28 … 12-04).**

| Child | Implementation | Lands | Base |
|---|---|---|---|
| R-JS-1 | 11-28 … 11-29 | 11-30 | main with R-0b, #653, P0a-1 |
| R-JS-2 | 11-30 … 12-01 | 12-02 | main with R-JS-1 |
| R-JS-3 | 12-02 … 12-04 | **12-05** (= R-JS) | main with R-JS-2 |

Rules the dates follow: **(a)** R-JS-1 starts on the later of P0a-1's and R-0b's landings, since it edits R-0b's
`mark_frame` (§4.1) and §9.1 step 6 starts R-JS at P0a-1's (step 5 dates P0a-1's *implementation* 11-28 and step 6
reads that date as its landing, so 11-28 assumes P0a-1 lands that day; R-0b, step 3, targets 11-24, so the later is
P0a-1's while R-0b holds; a later landing of either moves R-JS-1 to it); **(b)** each child is cut from main on the day its predecessor lands and starts then
(§1.2 order, no stacked branches), so a child's landing day is its successor's start; **(c)** the last child's
landing is R-JS's 12-05 (§9.1 step 6); **(d)** the window is split by size: R-JS-3 (the entry, rooting, the upgrade)
gets three days, R-JS-1 and R-JS-2 two each, and each child's days include its `/pre-push` and single-pass Codex
(§1.2). A child that slips moves its successors day for day; when that would move R-JS-3 past 12-05, the slip is
reported to the PM with the §9.1 step 6 consequences (steps 10-15 and §5.4's R-JS-keyed Re-eval proposals). A slip
of P0a-1 past 11-28, or of R-0b past it, moves all three (rule (a)).

## §9. Ledger / erratum edits
**Made in this PR** (PM 880, PM 883, PM 886, PM 892; each a reviewed item, §0.3):
- **Entry memo**, [C11] / [C13] "Used by" cells (RS-3; [C11] with 6.c, Rev 3.2), with the umbrella §5.4 paragraph
  marked done.
- **Umbrella Rev 1.7** (RS-1, RS-2, RS-3, RS-5, RS-9): §1.2 "Unchanged", §3's §10.4.1.2 row, §4.2 (three children,
  the [C11] range with 6.c, 2ab's body), §8.1's four rows, §8.3, §7's and §9.1's R-0c, §9.1 step 6, the §0.2 history
  row (with (4)'s step split and (6)'s sites, Rev 3.2) and order; the RS-9 sites. Line count held at 999: every
  erratum replaces text on its own line.
- **Slice plan** (RS-6, RS-8): row C's classification, its S / N clause and its I-7 sentence; forward pointers in rows
  3 and N. **RS-9 sweep** (PM 886): slice-plan rows S, 1a, 2a, 2ab, 9a, the 2ab ordering paragraph (Rev 3.2: row S's
  wording, 1a's stack-layout pointer, 2ab's E-10 cite).

**Listed, not made** — the recorder's list is
`/Users/kazuaki/repos/send.sh/elidex-pm-trial/handoff/pending-at-landing/r-js-plan.md` (old → new text per line):
- **Repo ledger** (`docs/plans/2026-07-vm-p4-slot-ledger.md`, rows by id): `#11-vm-derived-constructor-this-binding`
  `:293` (A1 items 1-8, RS-7 / RS-9: the "on the call frame" phrasings, the [C11] range, 2ab's share, the kind's
  home, `construct_synchronous`'s callers; at R-JS-3's landing, retired naming the three child PRs);
  `#11-vm-declarative-environment-and-capture-representation` (A2, the edge, §1.3.4); `#11-vm-arrow-lexical-new-target`
  / `#11-vm-arrow-lexical-super` (A3 / A4, forward pointers, RS-8); `#11-vm-construct-isconstructor-check` `:333`
  (A5, RS-9); `#11-step9-class-extras` table row `:321` (A6: R-JS-3's call points and retag, C1-14's re-expectation).
- **PM ledger**: `#11-vm-do-new-instance-rooting` — Trigger "R-JS-3"; "umbrella §8.3 stands, PM 875" → "§8.3 stands
  except R-JS-1's Function-arm expression (PM 880)"; the upgrade clause "probably rooted via `HostData::wrapper_cache`
  … not measured yet" → measured (§4.3.5), the remaining question is C3-17's. S / N pointer rows: "the record is
  R-JS-1's" (§7.2); C's pointer row: the edge; `#11-vm-generator-instance-prototype-from-function`: the helper edge
  and R-JS-3's convergence if it lands first (§7.2); `#11-step9-class-extras` (B2 item 4, as A6). At R-JS-3's
  landing (C2): rows `:248`, `:254` (`#11-vm-call-depth-guard`), `:257`
  (`#11-vm-native-callback-reentry-single-loop`) re-pointed from `construct_synchronous` to `VmInner::construct`.
- **D-17b R4 G4-2** (`extends_null_default_ctor_is_base`): superseded by C-3 at R-JS-2.

**Umbrella items left for a later revision, each with its trigger** (R1 A3 MIN):
- R-0b §5.4's re-pointing of the umbrella's two-site cells to `mark_frame`: **trigger R-0b's landing** (target 11-24);
  owner VM P4; due before R-JS-1 starts (11-28), since R-JS-1 edits `mark_frame`; a slip is reported to the PM.
  It cannot be made now: the tree it describes does not exist before R-0b lands.
- None other: S-3's cite was re-made by #654 itself (C-7); "R-JS = three children" is made in Rev 1.7 (RS-5).
**Reported to the PM** (Rev 3.3): whether `throw_error` should refuse `InternalError` VM-wide (§1.3.3; not R-JS's).

## §10. Facts to measure before or at each child (scout questions)
- S-1 Every exhaustive `match` over `ObjectKind` without a wildcard (each needs a `ThisEnvironment` arm); count with a
  command and a control hit.
- S-2 Every struct literal of `CallFrame`, `FunctionObject`, `JsCalleeInfo`, `CallIC`, `CompiledFunction` (tests
  included): `git grep -n -P '<Type> \{'` at the child's base (at `3cbd5b88`: 7 / 3 / 3 / 3 / 11 lines, comments and
  type definitions included).
- S-3 E-4's red at base for `HTMLElement`, `DataTransfer`, `Touch` and C1-10; R-1's red (C3-9). (C-4's red, C2-6, is
  measured: C-4.)
- S-4 Whether any reader of `this_value` outside `vm/` exists (`git grep -n -w this_value -- crates`), and every
  `captured_this` reader at the child's base.
- S-5 The upgrade path under forced GC (C3-17) at R-JS-3's base.
- S-6 (R-JS-3) Every Rust-side read of a constructor's `prototype` for instance creation in `crates/script/elidex-js/src/vm`
  (property, not spelling: a `Get` / own-property read of `prototype` followed by an allocation), with a control hit;
  after R-JS-3 the list is the helper, its callers and R-NAT's `resolve_validated_prototype`.
- S-7 (R-JS-1) The ops that are **§9.4.3 consumers** — whose spec operation calls GetThisEnvironment directly or
  through ResolveThisBinding, GetSuperConstructor, MakeSuperPropertyReference or GetNewTarget — by the op's spec
  meaning, not by what its dispatch arm reads today (controls: `Op::PushThis`, and `GetSuperProp` / `SetSuperProp` /
  `GetSuperElem`, whose stub arms read nothing at `3cbd5b88`): the `Reads` side of `Op::reads_this_environment` must
  contain every one. Cross-check: every op whose dispatch arm reads the frame's `this_value`, `mode` (newTarget) or
  `home_class` (`vm/dispatch*.rs`) is in the list, except `Op::Closure`: its arm reads the frame's `this_value` to
  capture it (`create_closure`, `vm/ops.rs:878-879` at `3cbd5b88`), which is the propagation, so it is
  `ByChildConstant` (§1.3.3) — the cross-check's one expected non-`Reads` hit (Rev 3.2, re-check A2 MIN-3).

## §11. PM questions (answered, PM 880; PM 883 on plan-review R1)
- **Q-1 Three children, one plan-review** (§1.2): confirmed; each child is its own PR implementing this plan. Dates set
  in §8 (R-JS-3 = 12-05).
- **Q-2 R-JS-1 changes `call_dispatch_rooted`'s Function arm** (C-1): allowed with §6.1's PM 642 (c) table; the
  umbrella erratum is written in this PR (§0.3 RS-1); PM 875 (2)'s premise recorded as corrected by the PM.
- **Q-3 `extends null` becomes derived** (C-3): spec-decided (ClassDefinitionEvaluation step 18), in R-JS-2;
  `extends_null_default_ctor_is_base` re-expected from the spec (§0.3 RS-4).
- **Q-4 The representation (E) as an internal `ObjectKind::ThisEnvironment`** (§1.3, §1.3.1): confirmed as the plan's
  proposal, to be checked in plan-review; the Side-store→component reasoning is §1.3.1. S / N / umbrella 3 owners are
  told at ratification.
- **Q-5 The entry memo's "Used by" cells**: added in this PR (§0.3 RS-3).
- **PM 883 D1** keep (E); one shape with C — chosen **(i)**, §1.3.4; errata RS-6 / RS-7 / RS-8. **D2** the umbrella's
  §3 row and four §8.1 rows in this PR (RS-1, RS-2), widening PM 880's go. **IMPs** A2 (emission, §1.3.3), A3 (one
  GetPrototypeFromConstructor, §4.3 DoD), A5-3 (C1-9 sole root) as proposed; the 25 MINs: §13's companion.

## §12. Measurements (provenance)
- **M-1 spec.** `.claude/tools/webref aoid ecma262` on EvaluateNew, Construct, PrepareForOrdinaryCall,
  OrdinaryCallBindThis, GetPrototypeFromConstructor, GetSuperConstructor, BindThisValue, NewFunctionEnvironment,
  GetThisEnvironment, ResolveThisBinding, OrdinaryFunctionCreate, ClassDefinitionEvaluation,
  InitializeInstanceElements, MakeConstructor, IsConstructor, GetNewTarget (2026-11-21); `heading ecma262` 10.2.2,
  10.3.2, 10.4.1.2, 13.3.7.1, 9.1.1.3.3, 13.2.1.1; `coverage-map` over §3's 22 rows (K = 3, M = 22); `body` on
  `sec-ecmascript-function-objects-construct-argumentslist-newtarget`, `sec-super-keyword-runtime-semantics-evaluation`,
  BindThisValue, `sec-function-environment-records-getthisbinding`, GetThisEnvironment, OrdinaryCallBindThis,
  NewFunctionEnvironment, ClassDefinitionEvaluation (steps 14, 18), `html concept-upgrade-an-element`, `webidl
  construct-a-callback-function`. Rev 3 (2026-11-21): `aoid` InitializeBinding, SetMutableBinding, PerformEval,
  DefineField, ClassFieldDefinitionEvaluation; `heading ecma262` 9.1.1.3, 9.1.1.1, 9.1.1.1.4, 9.1.1.1.5, 19.2.1.1,
  10.3.2; `body` on `sec-function-environment-records`, `sec-declarative-environment-records-initializebinding-n-v`,
  `…-setmutablebinding-n-v-s`, `…-getbindingvalue-n-s`, `sec-performeval` (step 11.a), `sec-definefield`,
  ClassDefinitionEvaluation (step 14.a-14.b), `sec-ecmascript-function-objects-construct-argumentslist-newtarget`
  (steps 1-17, for RS-3), `sec-super-keyword-runtime-semantics-evaluation` (SuperCall 1-13), and (C-2)
  `sec-bound-function-exotic-objects-construct-argumentslist-newtarget`. Rev 3.1 (2026-11-21): `aoid` MakeSuperPropertyReference
  (§13.3.7.3), GetSuperConstructor (§13.3.7.2), GetThisEnvironment (§9.4.3); `body` MakeSuperPropertyReference (step 1).
  Rev 3.2 (2026-11-21): `body` on `sec-ecmascript-function-objects-construct-argumentslist-newtarget` (step 6.c's
  abrupt exit; step 9 alone removes calleeContext, 10-11 handle the completion) and
  `sec-super-keyword-runtime-semantics-evaluation` (SuperCall steps 1, 3 and 7: GetNewTarget, GetSuperConstructor,
  GetThisEnvironment).
- **M-2 counts** at `7af70b68` under `crates/script/elidex-js/src`: `git grep -n -P '<pattern>' 7af70b68` →
  `CallFrame \{` 7, `CompiledFunction \{` 11, `FunctionObject \{` 3, `captured_this` 18, `new_instance` 19,
  `pre_alloc` 7, `construct_synchronous\(` 3, `complete_inline_frame\(` 5, `push_js_call_frame\(` 7,
  `compute_this_for_call\(` 6. Re-run at `378633f6` and at `3cbd5b88` (2026-11-21): every count identical;
  `JsCalleeInfo \{` 3, `CallIC \{` 3 at `3cbd5b88`.
- **M-3 lines.** `wc -l` in the worktree at `7af70b68` (§5); `vm/interpreter_call.rs` and `vm/natives_generator.rs`
  re-measured at `378633f6` (`git diff --stat 7af70b68 378633f6`: of §5's files, the only ones #653 changed).
- **M-4 roots.** `vm/gc/roots.rs` `mark_roots` (frame loop; (e) host data and `wrapper_store`),
  `vm/host_data/engine_feature/object_roots.rs` `gc_root_object_ids` (`ce_constructors`), read 2026-11-21.
- **M-5 ledgers.** `grep -n -F` on `project_open-defer-slots.md` for each slot id (2026-11-21); PORTFOLIO lines of PM
  805, 808, 832, 837, 845, 851, 875 read 2026-11-21; #653 landed as `378633f6` (`git log origin/main`), its slot's row
  closed and `#11-vm-do-new-instance-rooting`'s row open (`grep -n -F`, 2026-11-21).
- **M-6 family.** After RS-3: `python3 .claude/tools/plan-memo-umbrella-check.py
  docs/plans/2026-07-vm-p4-es-language-completeness.md` → exit 0 (0 mechanical findings);
  `python3 docs/plans/test_vm_p4_family_layout.py` → OK (2 tests).
- **M-7 C2-6.** The driver's temporary test on origin/main `378633f6`, rustc 1.99.1, removed after (PM 880 item 6);
  results in C-4.
- **M-8 anchors at `3cbd5b88`** (`git grep -n` on that sha): `interpreter_call.rs` `call_dispatch` :29,
  `call_dispatch_rooted` :46, Function arm :64, `compute_this_for_call(` :68, `WasmExportedFunction` arm :267,
  `call_internal` :305 (literal :399, `new_instance: None` :415, coroutine branch :432), `push_js_call_frame` :474
  (parameter :480, literal :569, field :581); `dispatch_class.rs` `construct_synchronous` :273, `with_call_mode` :323;
  `gc/roots.rs` :350 / :386.
- **M-9 code reads (Rev 3, at `3cbd5b88`).** `scope/visitor.rs` `visit_class_body` (no member-key visit; static block
  scope); `compiler/expr_class.rs` (`Property` arm: static initializer and computed key in the enclosing frame,
  instance initializers skipped; `StaticBlock` arm: a child non-arrow `FunctionCompiler`; default constructors as raw
  bytes); `compiler/function.rs` emit helpers; `bytecode/opcode.rs` `Op::operand_size` (exhaustive);
  `vm/dispatch_class.rs` `walk_bound_chain` (visitor gets `bound_args` only); `vm/construct_admission.rs`
  `admit_construct`; `vm/frame.rs` `CallFrame::kind` doc (no JS-level `eval`); Rev 3.1: `bytecode/opcode.rs`
  `GetSuperProp` / `SetSuperProp` / `GetSuperElem` and their stub arm (`vm/dispatch.rs`; reads no frame state), the
  `emit*` helpers' operands (`compiler/function.rs`; `Op::Closure` via `emit_u16`), the frame-entry writers
  (`git grep -n -P 'push_js_call_frame\(|call_internal\(' 3cbd5b88`: `do_new`, `construct_synchronous`, `run_function`, the IC
  calls, the Function arm); PM ledger rows
  `#11-vm-do-new-instance-rooting` and `#11-vm-generator-instance-prototype-from-function` (`grep -n -F`). Rev 3.2:
  `mise.toml:19` (`cargo clippy --workspace -- -D warnings`); `vm/mod.rs:190` (`pub(crate) struct VmInner`);
  `compiler/function.rs:19` (`pub bytecode`) and the 13 outside touches (§1.3.3's command); `vm/ops.rs:878-879`
  (`create_closure` reads `this_value`); PM ledger `:248`, `:254`, `:257`, `:289`, `:1740`, `:1752` and repo ledger
  `:23` (`grep -n -F` / `grep -c -F` on each landing-list old text → 1).
- **M-10 Rev 3.2.1 / 3.3.** 3.2.1 (`2dcce2e2`, PM 899): §13's tables moved verbatim to the companion (`b13ea4c6`
  :972-1071), no plan statement changed. 3.3 (re-check at `2dcce2e2`, at `3cbd5b88`): `Cargo.toml:312`, `lib.rs:37`;
  `vm/ops.rs` `throw_error` / `vm_error_to_thrown` (`:187`), `vm/dispatch.rs:745`, `Op::CheckTdz`; `ObjectKind`
  derives nothing (`vm/object_kind.rs:113`); `git grep -c -P '\bobject_id\(\)'` → 0; repo ledger `:321` (A6).

## §13. Plan-review dispositions

The per-finding disposition tables for plan-review R1 (at `bee45988`), the R1-fix re-check (at `58b666a6`) and the Rev 3.1 re-check (at `b8029ab9`), with the additions-ledger summary, are kept in the tracked companion `docs/plans/2026-11-vm-native-construct-r-js-dispositions.md` (verbatim from `b13ea4c6` lines 972-1071; PM 899). Nothing in them is a plan statement; every fix they record is in §0–§12 above.
