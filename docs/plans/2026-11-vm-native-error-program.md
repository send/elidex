# NativeError program (umbrella plan-memo)

**Slot**: `#11-vm-native-error-prototypes` (registered 2026-11-05, PM 363; re-scoped into three axes and two PRs,
PM 364; two prereq PRs added and the clone's prototype rule moved into E1, PM 365). Also closed here:
`#11-error-object-builtintag-dual-representation` (by E1) and `#11-vm-json-internalize-non-array-objects` (by
E0). **Owner**: VM P4.
**Home**: this memo, the program's own umbrella (PM 370). VM P4 slice-plan umbrella 9 (builtin surface,
`docs/plans/2026-07-vm-p4-slice-plan.md`) gets a **pointer row for axes A + B only** (the Error intrinsic
surface); axis C and E0 are not umbrella-9 surface. The pointer row is a landing obligation of this memo (§5),
not done in this commit. **Baseline**: origin/main
`f9cf8c40` (Rev 1-2 were measured at `145927c9`; Rev 3 re-derived every census at `f9cf8c40`, E1 memo §6). Paths are relative to `crates/script/elidex-js/src/` unless they start with `crates/` or `docs/`.
Spec numbers come from `.claude/tools/webref` output only. Per-PR plan for E1:
`docs/plans/2026-11-vm-native-error-e1-errordata.md` (this commit). This umbrella states the program; it does
not plan E2 in detail.

**Rev 1** (2026-11-05) — first draft, with E1's per-PR plan, for one `/elidex-plan-review`.
**Rev 2** (2026-11-05) — PM 365 folded in: E0 and E1-pre are standalone prereq PRs (order E0 → E1-pre → E1 →
E2); the clone's by-name prototype (Rev 1's E1 OD-1) is E1's; Rev 1's proposed slots are registered. This is
the program's second widening (PM 364 was the first); no further axis is opened by it.
**Rev 3** (2026-11-05) — plan-review R1, PM 370 rulings: rebased onto `f9cf8c40`; E1-pre **is** VM P4
Slice F (`#11-vm-structured-clone-file-split`), with F's seam and F's own process, so this memo no longer
draws a seam of its own (§1.2); the home is this memo, umbrella 9 gets a pointer row for A + B (§5); E2's
split verdict is left to E2's own plan-review (§0.2, §4); E1's receiver promotion is fail-safe (E1 memo §1.2),
the receiver-admission class staying with its registered slot (§5).
**Rev 4** (2026-11-05) — plan-review R2, PM 374 rulings: E1's second-`super()` cell is dropped (base behaviour
there is a debug panic, owned by two registered slots; E1 memo §1.2); the umbrella-9 pointer row is specified
in both its forms (§5); E1's order against the layering-seam derivation is stated once (§1.2); the clauses
E1's map owns are added to §3; clerical fixes.

## §0. Frame, premises, decisions

### §0.1 The defect

Three coupled defects in the Error family, probed at `145927c9` (probes `isctor/impl/m10-probe3.log` and
`isctor/impl/e1-probe.log` in VM P4 session `0fe71d24`'s scratchpad; `f9cf8c40` changes none of the probed
files' behaviour, #592 being a pure move of `vm/globals.rs`), line cites at `f9cf8c40`:

- **A — object structure.** `register_error_constructors` (`vm/globals_errors.rs:23`) gives `Error`,
  `TypeError`, `ReferenceError`, `RangeError`, `SyntaxError` and `URIError` one shared `.prototype`
  (`:66-89`; its docstring `:24-32` admits it). The 5×5 prototype identity matrix is all `===`,
  `TypeError.prototype.name` is `"Error"`, `TypeError.prototype.constructor !== TypeError`, `EvalError` is not
  defined, and `Error.prototype` has no `constructor` (`:34-63` define `toString`, `name`, `message` only).
  ECMA-262 §20.5.6 NativeError Object Structure: each NativeError has its own prototype (§20.5.6.3) whose
  `[[Prototype]]` is %Error.prototype%, with own `constructor`, `message`, `name` (§20.5.6.3.1-3).
- **B — construction sites choose the prototype, and the constructors' `[[Prototype]]`.** The throw path
  (`vm_error_to_thrown`, `vm/ops.rs:171`, alloc `:201-206`) always uses `error_prototype`; call-mode
  `error_entry` (`vm/natives.rs:488`) does too; `clone_error` (`vm/host/structured_clone.rs:638`) keeps the
  source's prototype with an `error_prototype` fallback (`:656`), where HTML StructuredDeserialize picks it by
  the serialized name (that one site is E1's, PM 365; §1). Every error constructor is allocated with
  `function_prototype` (`create_constructable_function`, `vm/shape_ops.rs:281-288` → `:511`), where
  §20.5.6.2 and §20.5.7.2 require %Error% — AggregateError and the three Wasm classes included (E1 memo
  §0.3 P-4: the brief's "AggregateError is correct" holds for its prototype object only).
- **C — two representations of an error.** Constructed errors are `ObjectKind::Ordinary`; only the throw path
  makes `ObjectKind::Error` (`vm/ops.rs:202`). So `Object.prototype.toString` (keyed on the kind) is wrong for
  every constructed error, and structured clone and IndexedDB (keyed on prototype identity, by two different
  predicates) clone a forged `Object.create(Error.prototype)` as an error and a subclass instance as `{}`.

### §0.2 The program decision (PM 364)

E1 = C, E2 = A + B, edge E1 → E2. **A alone regresses B**: give the NativeErrors their own prototypes while
the throw path still allocates on `error_prototype`, and `e instanceof TypeError` becomes false for every
engine-thrown TypeError. So A and B are one PR. **C is correct alone**: the only prototype it changes is a
structured clone's, and that change is toward the spec (by serialized name, decision 3 below; Rev 1-2 said
"it changes no prototype", which decision 3 made stale), and it is the precondition for E2 (E2's construction
sites must make the kind, and E2's prototypes reach clones only through C's by-name table). The design inside
each PR is its per-PR plan's.

**E2's split verdict is E2's own plan-review's** (PM 370). "A and B are one PR" says only that A cannot land
without B; whether E2 is itself one terminal unit or splits further is measured by E2's per-PR plan under
§2's criterion, not declared here. One axis that review must measure, not assume: the constructors'
`[[Prototype]]` chain (%Error% for the NativeErrors, AggregateError and the Wasm classes, §4), which has
no construction site in common with the instances' prototypes.

**Revised by PM 365** (three decisions, final):

1. **E0** (`#11-vm-json-internalize-non-array-objects`, registered PM 365) is a standalone prereq PR before E1
   (§1, edge E0 → E1).
2. **E1-pre** is a standalone pure-move split of `vm/host/structured_clone.rs` before E1. **Refined by PM
   370**: E1-pre **is** VM P4 Slice F (`#11-vm-structured-clone-file-split`); what that means for this
   program is stated once, in §1.2.
3. **The clone's prototype by serialized name is E1's**: `clone_error` chooses it per HTML StructuredDeserialize
   step 22 through `error_intrinsic_prototype(name)`, which returns `error_prototype` for every name until E2,
   and E2 only fills that table (E1 memo §1.6). The "`clone_error` fallback" site of axis B leaves E2's scope
   (§4).

### §0.3 Premise errors found while planning E1 (Rev 1; dispositions by PM 365)

Full statements in the E1 memo §0.3. The ones that touch the program:

- **The creation census is wider than the brief's list** (E1 P-2): the three `WebAssembly.*Error`
  constructors share `error_ctor_impl`, the Wasm bridge (`wasm_error_to_js_value`, `vm/host/wasm/errors.rs:179`)
  and `Promise.any` (`build_aggregate_error`, `vm/natives_promise_combinator.rs:258`) create objects the spec
  gives `[[ErrorData]]`. E1 includes them (E1 OD-3). E2's construction-site census must include them too.
- **E1 is not "regresses nothing" on its own** (E1 P-5, P-6). One consumer branches on
  `ObjectKind::Ordinary` (JSON.parse's reviver walk, `vm/natives_json/parse.rs:444` / `:469`), and the clone's
  prototype rule is the source's, so two clone rows change. Both were decided by PM 365: the JSON consumer is
  E0 (decision 1), the clone's prototype rule is E1's (decision 3).
- **Map / Set do not exist in the VM** (E1 P-3), so they are no precedent; the precedent is `Date`'s
  `create_date` / `promote_to_date` (E1 §1.2).

## §1. Shape and verdict

The program crosses three invariant axes (A, B, C) with two independently live groups, so it is an umbrella
(CLAUDE.md "Edge-dense work"): one per-PR plan per row, each through `/elidex-plan-review` before
implementation. A per-PR plan that passes plan-review under this umbrella is a terminal unit.

| PR | Scope | Depends on | Closes | Plan-review |
|---|---|---|---|---|
| E0 | JSON.parse's reviver walk visits every non-array object (§1.1) | — | `#11-vm-json-internalize-non-array-objects` (registered, PM 365) | required — on the short per-PR plan section in E0's own PR (single axis, no memo); that section passes `/elidex-plan-review` before E0's code is written |
| F (VM P4) | prereq = VM P4 Slice F, per its row in `docs/plans/2026-07-vm-p4-slice-plan.md` §5 (§1.2); not this program's row | — | `#11-vm-structured-clone-file-split` (F's) | F's own, per its row (§1.2) |
| E1 | Axis C: every `[[ErrorData]]` creator makes `ObjectKind::Error` (payload dropped); every classifier keys on it (structured clone, IndexedDB; toString already does); and the clone's prototype by serialized name through `error_intrinsic_prototype(name)` (PM 365) | E0, F | `#11-error-object-builtintag-dual-representation`; axis C of `#11-vm-native-error-prototypes` | required — `docs/plans/2026-11-vm-native-error-e1-errordata.md` |
| E2 | Axes A + B (§4): own NativeError prototypes and EvalError, every remaining construction site's prototype, constructor `[[Prototype]]` = %Error%, and the contents of `error_intrinsic_prototype`'s table | E1 | `#11-vm-native-error-prototypes` | required — **its own per-PR plan and its own `/elidex-plan-review` before E2 is implemented**, which also gives E2's split verdict (§0.2); not planned here |

**Order**: E0 → F → E1 → E2 (PM 365; F in E1-pre's place, PM 370), with E1's place after F stated in §1.2
(PM 374). E0 and F have no code dependency on each other; the order between them is the PM's, so it is a
sequence, not a choice.

### §1.1 E0 — JSON.parse's reviver walk (`#11-vm-json-internalize-non-array-objects`)

- **Defect.** `internalize` (`vm/natives_json/parse.rs`) recurses into an object's keys only when its kind is
  `ObjectKind::Ordinary | ObjectKind::Arguments { .. }`, and its `_ => {}` arm returns every other non-array
  kind unwalked. So a reviver never sees the own enumerable properties of a Date, a RegExp, a primitive
  wrapper or an engine-thrown error placed into the parsed graph, and after E1 it would not see those of a
  constructed error either. Silent-wrong.
- **Spec.** ECMA-262 InternalizeJSONProperty (§25.5.2.4, `.claude/tools/webref aoid ecma262
  InternalizeJSONProperty`), step 5: if value is an Object, step 5.a `? IsArray(value)`; step 5.b walks an
  array by index; step 5.c (else) walks `? EnumerableOwnProperties(value, key)` — every non-array object, by
  no other test of its kind.
- **Scope.** The non-array arm of `internalize` admits every object that is not an array (the `_ => {}`
  fallthrough goes). Nothing else in `internalize` changes; anything else E0's per-PR section finds there is
  reported, not taken.
- **Acceptance.** A reviver that places an object of each of these kinds — Date, RegExp, a Number wrapper, a
  caught engine-thrown error, a constructed `new Error('x')` — with an own enumerable property `k` into a later
  key of the parsed graph has `k` visited (red at E0's base for every row but the constructed error, which is
  `Ordinary` until E1 — read from the `internalize` arms, not probed; the constructed-error row is the one
  E1's DoD keeps green). Every existing `JSON.parse` reviver test
  stays green. One mutant: the `_ => {}` arm restored → the non-`Ordinary` rows fail.

### §1.2 E1-pre = VM P4 Slice F (`#11-vm-structured-clone-file-split`)

**Prereq = Slice F (VM P4), per its row** (`docs/plans/2026-07-vm-p4-slice-plan.md` §5, row **F**; PM 370).
This is the one statement of Slice F in this program; every other mention in this memo and in the E1 memo
refers here. This memo draws no seam and states no acceptance of its own for the split (Rev 1-2's own seam and
"plan-review not required" are withdrawn); what follows only points at what F's row already says
(`git grep -n '^| \*\*F\*\* |' -- docs/plans/2026-07-vm-p4-slice-plan.md`), plus E1's place after it:

- **E1's order (PM 374)**: E1 comes after F's plan-review and after any `#11-vm-structured-clone-layering-seam`
  child that review orders before F's consumers; E1's cites follow wherever the clone-algorithm half lands.
  The PM recorded the edge F → E1 on both rows of the live ledger
  (`/Users/kazuaki/.claude/projects/-Users-kazuaki-repos-send-sh-elidex/memory/project_open-defer-slots.md`,
  the pointer rows of `#11-vm-structured-clone-file-split` and `#11-vm-structured-clone-layering-seam`).

- **Seam**: the file's own banners — the clone algorithm (`clone_value` through `data_clone_error`) vs
  `// Global registration` plus the WebIDL binding layer (`register_structured_clone_global`,
  `native_structured_clone`, `validate_transfer`, `ensure_empty_transfer_list`). F's row names no post-split
  file paths, so E1 cites the structured-clone sites by function, "(after Slice F)", and names no
  post-split path (pre-split lines at `f9cf8c40` appear only as locators, E1 memo §1.8).
- **Process**: F is a §5 row with a mandatory `/elidex-plan-review` of its own; its acceptance is F's (the
  suite passes unchanged, the two halves compile as separate modules, the diff is hunk- and
  line-conserving with the hunk count recorded), and so is its declared green guard (zero content delta on
  the StructuredSerialize-vs-StructuredDeserialize classification comments).
- **Not F's, and not this program's**: the layering question the algorithm half's size invites is
  `#11-vm-structured-clone-layering-seam`'s, whose trigger is F's plan-review (its ledger row in
  `docs/plans/2026-07-vm-p4-slot-ledger.md`).

## §2. Edges and invariants

- **E1 → E2 (the PM's edge).** E2's construction sites (§4) write the kind that E1 introduces, and E2's
  own prototypes reach clones only through the `error_intrinsic_prototype` table that E1's `clone_error`
  already reads by serialized name (PM 365). If E2
  landed first, every new own prototype would be one more prototype that the identity classifiers
  (`is_error_like_proto`, `is_error_like`) do not know, so a `new RangeError()` would clone as `{}` and store
  in IndexedDB silently — the regression PM 364 named.
- **F → E1** (F in E1-pre's place, PM 370; the order against the layering-seam derivation, PM 374): §1.2.
- **E0 → E1** (PM 365). E1 moves constructed errors out of `ObjectKind::Ordinary`; E0 removes the one
  consumer that treats that kind as "walkable object". Without E0, E1's DoD cannot keep E0's
  constructed-error acceptance row green.
- **The program invariant at E2's landing**: an object is an error iff it was created with `[[ErrorData]]`
  (one kind, one predicate); its prototype is the intrinsic its creator names (one rule per site, no shared
  prototype); and no site recognises an error by prototype identity.

## §3. Spec coverage map

Program-level: the clauses each PR owns, so the per-PR maps can be checked against it. Numbers from
`.claude/tools/webref heading ecma262 20.5`, `.claude/tools/webref heading ecma262 5.2.4`,
`.claude/tools/webref aoid ecma262 <AO>`, `.claude/tools/webref dfn html <AO>`,
`.claude/tools/webref dfn IndexedDB-3 clone`, `.claude/tools/webref heading wasm-js-api-2 5.10` and
`.claude/tools/webref heading webidl 3.14.1`. E1's step-level map is its memo's §3; every clause it owns has a
row here.
The plan-review preflight over this table
(`python3 .claude/skills/elidex-plan-review/preflight.py docs/plans/2026-11-vm-native-error-program.md`) returns
"SPLIT-RECOMMENDED (K>=4 or M>=20)" with M = 20 and K = 5 (ecma262, html, webidl, IndexedDB, Wasm JS API). The
recommendation is already met: the program is split into Slice F, E0, E1 and E2 (§1), each under its own
plan-review, and K comes from consumer specs (IndexedDB consumes HTML's algorithm, the Wasm JS API row says which
objects the ECMA-262 rows cover, and the Web IDL row touches nothing; E1 memo §3), not from added subsystems.

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| ECMA-262 §20.5.4 Properties of Error Instances | — | instances have [[ErrorData]]; prototypes do not | E1: `vm/object_kind.rs:135` + E1 memo §1.1's eight creators | ✓ | yes |
| ECMA-262 §20.1.3.6 Object.prototype.toString ( ) | 8 | [[ErrorData]] → "Error" | E1: `vm/natives_symbol.rs:284` (unchanged arm, now reached) | ✓ | yes |
| WHATWG HTML §2.7.4 StructuredSerializeInternal ( value, forStorage [ , memory ] ) | 18 | [[ErrorData]] and not a platform object; the name (18.1-18.2) | E1: `clone_recursive` guard, `classify`, and the name read in `clone_error` (after Slice F), its step-18.2 comparison `serialized_error_name` in `vm/globals_errors.rs`; the message (18.3-18.4) and the rest of the record: `#11-structured-clone-error-record-fidelity` | ✓ | yes |
| WHATWG HTML §2.7.7 StructuredDeserialize ( serialized, targetRealm [ , memory ] ) | 22 | prototype by serialized name (22.1-22.7); object with [[ErrorData]] (22.9) | E1 (`clone_error` → `error_intrinsic_prototype`, PM 365); E2 fills the table's per-name prototypes | ✓ | yes |
| ECMA-262 §20.5.1.1 Error ( message [ , options ] ) | 1-2 | OrdinaryCreateFromConstructor « [[ErrorData]] » (E1); intrinsicDefaultProto %Error.prototype% (E2) | `vm/natives.rs:488` (`error_entry`) | ✓ | yes |
| ECMA-262 §20.5.6.1.1 NativeError ( message [ , options ] ) | 1-2 | same with %NativeError.prototype% — E1 the slot, E2 the prototype | `vm/natives.rs:508-546` | ✓ | yes |
| ECMA-262 §20.5.5.1 EvalError | — | the constructor and prototype exist; no engine path throws one (`VmErrorKind` has no variant, `vm/error.rs:20-46`) | E2: `vm/globals_errors.rs` | ✓ | no |
| ECMA-262 §20.5.6.2 Properties of the NativeError Constructors | — | [[Prototype]] is %Error% | E2: `vm/shape_ops.rs:281-288` callers in `vm/globals_errors.rs:75-89` | ✓ | no |
| ECMA-262 §20.5.6.3 Properties of the NativeError Prototype Objects | — | own object, [[Prototype]] %Error.prototype%, own `constructor` / `message` / `name` | E2: `vm/globals_errors.rs:66-89` | ✓ | no |
| ECMA-262 §20.5.3.1 Error.prototype.constructor | — | own `constructor` on %Error.prototype% | E2: `vm/globals_errors.rs:34-63` | ✓ | no |
| ECMA-262 §20.5.7.2 Properties of the AggregateError Constructor | — | [[Prototype]] is %Error% | E2: `vm/globals_errors.rs:114-115` | ✓ | no |
| ECMA-262 §27.5.4.3.1 PerformPromiseAny ( iteratorRecord, ctor, resultCapability, promiseResolve ) | 5.b.ii.1, 5.e.vii.1 | a newly created AggregateError object | E1: `vm/natives_promise_combinator.rs:258` | ✓ | yes |
| ECMA-262 §25.5.2.4 InternalizeJSONProperty ( holder, name, reviver, parseRecord ) | 5.c | every non-array object is walked | E0: `vm/natives_json/parse.rs:444` | ✓ | yes — the reviver |
| ECMA-262 §5.2.4.2 Throw | — | "throw a TypeError exception" ≡ ThrowCompletion(a newly created TypeError object) — the engine throw creates an [[ErrorData]] object | E1: `vm_error_to_thrown` `vm/ops.rs:171` → `create_error_object` (NEW) | ✓ | yes |
| ECMA-262 §10.1.13 OrdinaryCreateFromConstructor ( ctor, intrinsicDefaultProto [ , internalSlotsList ] ) | 2-5 | the slot list « [[ErrorData]] » on the newTarget-derived object | E1: `do_new` pre-allocation `vm/ops.rs:722-729` + `promote_to_error` (NEW) | ✓ | yes |
| ECMA-262 §20.5.7.1.1 AggregateError ( errors, message [ , options ] ) | 1-2 | OrdinaryCreateFromConstructor « [[ErrorData]] » (E1); its intrinsicDefaultProto and the constructor's [[Prototype]] (E2) | E1: `vm/natives.rs:602` → `error_receiver` (NEW) | ✓ | yes |
| WHATWG HTML §2.7.6 StructuredSerializeForStorage ( value ) | — | IndexedDB serializes through StructuredSerializeInternal, so through step 18 | E1: `vm/host/indexeddb/value.rs:298` (kept), `:318-323` + `:363-371` (deleted) | ✓ | yes |
| IndexedDB §5.11 Clone a value | 3-4 | one classifier, structured clone's | E1: `value_to_json` → `clone_value` `vm/host/indexeddb/value.rs:382` | ✓ | yes |
| Wasm JS API §5.10 Error Objects | 2.1 | each class implements the NativeError Object Structure → instances have [[ErrorData]] (E1); the constructors' [[Prototype]] (E2) | E1: `vm/natives.rs:554`, `:569`, `:584`; `vm/host/wasm/errors.rs:201-206` | ✓ | yes |
| Web IDL §3.14.1 DOMException custom bindings | — | that DOMException *instances* have [[ErrorData]] is a reading (E1 memo §1.5); a platform object, never step 18 | none — unchanged by E1 | ✓ (no touch) | yes |

## §4. E2's scope (stated, not planned)

E2's per-PR plan derives its design and its own coverage map; this list is the census it starts from and the
facts it must not lose.

- **A**: six own prototypes (EvalError included) chained to %Error.prototype%, each with own `constructor`,
  `message` (`""`) and `name`; `Error.prototype.constructor`; the `register_error_constructors` docstring's
  admission removed with the shortcut. The Wasm classes already have own prototypes
  (`install_wasm_error_class`, `vm/host/wasm/errors.rs:97`) and are the in-tree shape to match.
- **B, constructors**: `[[Prototype]]` = %Error% for the six NativeErrors, AggregateError and the three Wasm
  classes (Wasm JS API §5.10: "implementing the NativeError Object Structure").
- **B, construction sites** — each chooses its prototype: `vm_error_to_thrown` (`VmErrorKind` → intrinsic;
  whether `VmErrorKind` needs an EvalError variant depends on whether any engine path throws one — none does at
  `f9cf8c40`, `git grep -n 'EvalError' f9cf8c40 -- crates/script/elidex-js/src` → no output, so measure,
  do not add by symmetry); call-mode `error_entry`; `build_aggregate_error` and `wasm_error_to_js_value`
  keep theirs. Every site listed in E1 memo §1.1 but `clone_error` is in E2's census. **Not a construction site of E2's**:
  `clone_error` — it chooses by serialized name in E1 (PM 365); E2 fills `error_intrinsic_prototype`'s table
  with the own prototypes (E1 memo §1.6), and changes nothing in `clone_error`.
- **The instance-level own `.name`** that `error_ctor_impl` (`vm/natives.rs:456-462`), the throw path
  (`vm/ops.rs:209-214`) and `build_aggregate_error` define is a workaround for the shared prototype; with
  own prototypes the spec's inherited `name` is the source of truth. **E2 decides whether and how it is
  removed** — this is the one statement of that decision; E1 keeps the property and references this line.
  One E1 cell depends on it: T-PROMO-UPGRADE's `el.name === 'TypeError'` row (E1 memo §4.1) pins the
  instance-level `name` that `error_ctor_impl` defines on the upgraded wrapper, so E2's decision changes that
  row, and E2's plan states the row's new value with the decision.
- **The table fill lands with the own prototypes**: E2 changes `error_intrinsic_prototype`'s arms in the same
  PR as axis A, never after it — a table still returning `error_prototype` once the NativeErrors have their
  own prototypes regresses `structuredClone(new RangeError()) instanceof RangeError` (true today through the
  shared prototype).
- **The by-name rule's cells** (E1 memo §4.2 M17): E1 kills M17 at unit level, on `serialized_error_name`'s
  `ErrorName` return (E1 memo §4.1 T-NAME-UNIT). What E1 cannot observe is the table's per-name *values*,
  because every arm returns `error_prototype` until E2; E2's per-name cells observe them —
  `structuredClone(new X()) instanceof X` for each of the six NativeErrors, and a script-set
  `e.name = 'RangeError'` on a TypeError choosing %RangeError.prototype% — with a mutant on the table (an arm
  returning `error_prototype`) in E2's mutant list.
- **E2's split verdict** (§0.2): E2's own plan-review measures it; the constructors' `[[Prototype]]` chain
  is an axis that review must measure, not assume folded into A or B's instance sites.
- **Not E2**: `do_new`'s GetPrototypeFromConstructor fallback (E1 memo SD-5) is engine-wide; E2 meets it when
  `ctor.prototype` is replaced and must say whether it consumes or defers it.

## §5. Out of scope and slots

- **Rev 1's stop-and-report items, all settled by PM 365** (one line each, where it went):
  - SR-1 (clone fidelity beyond the prototype) → registered `#11-structured-clone-error-record-fidelity`,
    whose decision surface is `#11-vm-structured-clone-layering-seam`'s umbrella forum (not a second one), and
    which also owns the mid-walk `Get(name)` snapshot staleness (E1 memo §1.6; PM 370); the prototype half is
    E1's (E1 memo §1.6).
  - SR-2 (DOMException's `[[ErrorData]]`, a predicate admitting a platform object) → chartered in
    `#11-vm-error-iserror` (its consumer, `Error.isError`; PM 370), not in `#11-domexception-serializable`,
    which owns DOMException's structured clone (E1 memo SD-4). E1 still does not change DOMException's
    recognition (E1 memo §1.5).
  - E0 (the JSON internalize axis) → program row E0 (§1, §1.1), registered
    `#11-vm-json-internalize-non-array-objects`.
- **Siblings, registered by PM 365** (E1 memo §5): `#11-vm-error-install-error-cause` (InstallErrorCause,
  §20.5.9.1, silent), `#11-vm-error-iserror` (Error.isError, §20.5.2.1, loud),
  `#11-vm-aggregate-error-step-order` (silent; its detail is E1 memo §5 SD-3), `#11-domexception-serializable`
  (silent, owner DOM), `#11-vm-get-prototype-from-constructor-fallback` (silent, engine-wide). None is taken
  by this program.
- **Registered by PM 370**: `#11-vm-native-ctor-receiver-promotion-admission` — the pre-existing class of
  in-place receiver promotions (every `promote_to_*` site, E1's `promote_to_error` included) reached with a
  receiver that is not a `do_new`-pre-allocated `Ordinary`; E1 takes its fail-safe stance (E1 memo §1.2) and
  not the class. Its second path's spec answer is `#11-vm-derived-constructor-this-binding`'s (VM P4 ledger).
  The double-`super()` debug panic (PM 374; E1 memo §1.2) is carried by that slot's **live-ledger pointer
  row** ("**Severity raised (PM 374, VM P4 probe at `f9cf8c40`)**";
  `/usr/bin/grep -n 'derived-constructor-this-binding. → docs/plans' <the live ledger, path in §1.2>` → `:1243`
  on 2026-11-05). The canonical row in `docs/plans/2026-07-vm-p4-slot-ledger.md` (`:293`) does not carry it
  yet (`sed -n 293p docs/plans/2026-07-vm-p4-slot-ledger.md | /usr/bin/grep -c 'shape_add_transition\|panic'`
  → 0 at `f9cf8c40`), and updating that row is the PM's. **Its trigger fires on E1** ("a PR touching any promote site or the upgrade construct path": E1 adds `promote_to_error`), so its
  disposition is recorded at E1's landing: re-evaluated there, its trigger restated for the promote sites that
  remain after E1 (E1 memo §5).
- **Ledger agreement** (PM 370; the ledger edits are the PM's): `#11-error-object-builtintag-dual-representation`
  closes at E1's landing; `#11-idb-structured-clone-storage` covers errors (after E1, IndexedDB rejects subclass
  and Wasm errors; the storage-format redesign is that slot's); the AggregateError and Wasm constructors'
  `[[Prototype]]` = %Error% is E2's (§4).
- **Landing obligations**:
  - **The umbrella-9 pointer, in both its forms, plus its §8 pointer row** (PM 370; forms fixed by PM 374),
    for axes **A + B only** (C and E0 are not umbrella-9 surface), all in one edit of
    `docs/plans/2026-07-vm-p4-slice-plan.md` and `docs/plans/2026-07-vm-p4-slot-ledger.md`:
    - **(a) a §5 row** (`| # | Slice | Primary module(s) | Slot | Tier | Deps |`), in the form of rows **R**
      and **M** (`git grep -n '^| \*\*R\*\* |\|^| \*\*M\*\* |' -- docs/plans/2026-07-vm-p4-slice-plan.md`),
      placed immediately after row **M** (`:992` at `f9cf8c40`), before the id-less `Function`/`eval` row:
      - `#`: **NE** (unused in the family: `cat docs/plans/2026-07-vm-p4-*.md | /usr/bin/grep -c '\*\*NE\*\*'`
        → 0 at `f9cf8c40`).
      - `Slice`: opens, in bold as R and M carry it, with "NativeError object structure (axes A + B of
        `#11-vm-native-error-prototypes`) — PROMOTED TO ITS OWN UMBRELLA, not a terminal unit, outside this
        plan's slice sequence" (the kind literal is R's and M's, character for character), then two sentences:
        the program, its PRs and their order are `docs/plans/2026-11-vm-native-error-program.md`'s, and this
        row is umbrella 9's surface carved out of its derivation (form (b)); its Tier is its own worst
        defect's, not umbrella 9's.
      - `Primary module(s)`: **—**, as R and M carry it (the modules are the program memo's §4 census).
      - `Slot`: `` `#11-vm-native-error-prototypes` `` (registered PM 363, so no **new**).
      - `Tier`: **T1**, from NE's own worst defect: silent-wrong (shared prototypes, so
        `new RangeError() instanceof TypeError` is true; the live ledger's "Silently wrong"), and the Tier
        column is the severity taxonomy of `docs/plans/2026-07-vm-p4-es-language-completeness.md` §2.4, not a
        value inherited from umbrella 9, as R (T2, loud: a parse error) and M (T1) each carry their own.
      - `Deps`: **—** (an umbrella carries no ordering; the program's order is its own memo's §1).
    - **(b) a mention in umbrella 9's row** (`git grep -n '^| \*\*9\*\* |' -- docs/plans/2026-07-vm-p4-slice-plan.md`),
      in the rule's **first** subtraction clause, "**minus any surface another slot already owns**", and not
      in the later §19-seed sentence: §20.5 is not a §19 seed, and the first clause is the one the whole
      rule carries. The phrase form is the §19 sentence's `Atomics` / `Reflect` precedents: the Error
      intrinsics' object structure (ECMA-262 §20.5.3.1, §20.5.5, §20.5.6.2-§20.5.6.3, §20.5.7.2; numbers
      from `.claude/tools/webref heading ecma262 20.5`) "to the child umbrella
      `#11-vm-native-error-prototypes`'s derivation mints for it". The same subtraction names the
      constructor clauses' intrinsicDefaultProto argument, step 2 of §20.5.1.1, §20.5.6.1.1 and §20.5.7.1.1
      (B's half; step numbers from `.claude/tools/webref body ecma262 sec-error-message` /
      `sec-nativeerror` / `sec-aggregate-error`), because §3 assigns that argument to E2, so no step is
      owned by both umbrella 9 and this program. In the same sentence, `Error.isError` (§20.5.2.1,
      `#11-vm-error-iserror`) and InstallErrorCause (§20.5.9.1, `#11-vm-error-install-error-cause`) are
      stated as **not** subtracted. Both are registered with owner "VM P4 (umbrella 9)" on the live ledger,
      and the mention is what makes them visible in the row.
    - **(c) a §8 pointer row** in `docs/plans/2026-07-vm-p4-slot-ledger.md` (`| Slot | Why deferred | Trigger |
      Re-eval |`), in the form of R's pointer slot (`git grep -n '^| .#11-vm-explicit-resource-management. |' --
      docs/plans/2026-07-vm-p4-slot-ledger.md`, `:301` at `f9cf8c40`), placed after `#11-vm-dynamic-import`
      (`:304`), so it is clear of the in-flight branch's tail row (below):
      - `Slot`: `` `#11-vm-native-error-prototypes` ``.
      - `Why deferred`: "**Pointer into §5's row NE**, which declares its own kind and is promoted out of this
        plan's sequence like Slice M; this cell restates nothing of it (§5: a pointer slot carries no marker
        of its own)", then: registered on the live ledger (PM 363), and the program, its PRs and their order
        are `docs/plans/2026-11-vm-native-error-program.md`'s. No marker, so the §8 half does not move.
      - `Trigger`: none of its own. The program memo's §1 order schedules it, and it closes at E2's landing
        (below).
      - `Re-eval`: *at registration: landing + 30 d*. The slot-ledger's Re-eval rule counts a row as
        registered only when its live-ledger entry has a Re-eval of its own, and this slot's entry has none
        (`/usr/bin/grep 'vm-native-error-prototypes. — \*\*open' <the live ledger, path in this memo's §1.2> | /usr/bin/grep -ci 're-eval'`
        → 0).
    - **Population figures the landing re-derives, never carries** (slice-plan census passage, lines at
      `f9cf8c40`). Each figure names the program that yields it. Before → after values are from the landing
      simulation (forms (a), (b) and (c) applied to a `git archive f9cf8c40 docs/plans .claude/tools` copy):
      - `:222` the triple `48 33 15` (total / §5 half / §8 half) → **`49 34 15`**: the checker's
        `[UMBRELLA-MARK]` line (`python3 .claude/tools/plan-memo-umbrella-check.py
        docs/plans/2026-07-vm-p4-es-language-completeness.md`, "48 rows … (slice=33, slot=15)" → "49 rows …
        (slice=34, slot=15)"). **NE**'s `Slice` cell carries the umbrella literal and enters the §5 half. The
        §8 pointer row carries none, so the §8 half stays 15.
      - `:222` "The reading returns **54**" → **49**. The reading it names is the same declaring-field reading
        whose total the triple reports, so today it returns 48 (the `[UMBRELLA-MARK]` total), not 54. The 54
        is the one-liner's total at `2d56fa40` (`54 37 17`, when §5 and §8 were one file), written at that
        commit and not re-derived when the triple moved to 48 at `50328183`.
      - `:222` "move at exactly **four** points" → five: the landing appends its own commit as the fifth point,
        `49 34 15` (row **NE** entering the §5 half).
      - `:222` "all **64** §5 data rows" and `:224` "**64 at this revision**" → **65** (a row is added).
      - `:223` the §8 slot-row figure "**52** in `docs/plans/2026-07-vm-p4-slot-ledger.md`" → **53**
        (`grep -cE '^[|] .#11-' docs/plans/2026-07-vm-p4-slot-ledger.md`, 52 → 53; a pointer row, so neither
        kind half moves). The landing extends that clause's history chain with the step.
      - `:239` "the split is **33** §5 rows and **15** §8 slots" → **34** and **15**, and its §5-side history
        chain ("… and **33** from the one that disproved row **2a**'s") gains "**34** from the revision that
        added row **NE**"; its §8-side chain ("**15** ever since") stays true.
      - Observation only, not this program's to fix, recorded as a fact for whoever re-derives: the passage's
        one-liner, run on the slice plan alone as the passage words it (`<this file>`), prints `33 33 0` at
        `f9cf8c40` (→ `34 34 0` in the simulation), because §8's table moved to the slot ledger. It
        reproduces the checker's triple only when it is given both files.
      - Simulation, for the record: checker rc 0 before and after (`[CENSUS]` 53 → 54, `[UMBRELLA-CELL]`
        37 → 38, 0 mechanical findings, seeds 51 → 51, reported naming sites 726 → 730), and `docs/plans/test_vm_p4_family_layout.py` OK
        (2 tests) before and after.
    - **The layout guard**: the rows and the mention name this memo as a backticked path only. No relative
      link goes from a family memo to a memo outside the family (`docs/plans/test_vm_p4_family_layout.py`).
    - **The checker**: `python3 .claude/tools/plan-memo-umbrella-check.py docs/plans/2026-07-vm-p4-es-language-completeness.md`
      (CLAUDE.md "VM-P4 umbrella memo の checker 実行"), rc 0 expected.
    - ⚠ **Overlap**: the in-flight `vm-p4-slice-1a-plan` branch (`d1e9186a`, merge base `145927c9`, whose
      slice plan and slot ledger equal `f9cf8c40`'s) edits the same passage and the same tables.
      `git diff f9cf8c40 vm-p4-slice-1a-plan -- docs/plans/2026-07-vm-p4-slice-plan.md` has four hunks:
      `@@ -219,9 +219,9 @@` rewrites the census lines in place (§5 rows 64 → 69, the §8 slot-row figure
      52 → 53 for its slot-ledger row `#11-vm-array-index-property-single-store`); `@@ -940,8 +940,13 @@`
      adds rows **1aa**-**1ae** after **1a** (and edits 1a and 1b), which shifts every later row by 5, so
      `:970` (R), `:981` (9) and `:992` (M) become `:975` / `:986` / `:997`; `@@ -958,7 +963,7 @@` and
      `@@ -969,7 +974,7 @@` edit rows **C** and **S** in place. Its slot-ledger diff edits `:287`, `:306-307`
      and `:325` in place and inserts its row after `:332` (`@@ -332,0 +333 @@` under `-U0`). After both land, the §5 population is **70**, the
      §8 slot-row count **54**, and the triple is unchanged by the 1a rows (terminal, so `49 34 15`; that
      branch alone prints `48 33 15`). The `:222` locators keep their line numbers but not their figures.
      Whichever lands second re-derives every figure above on top of the other, by the named programs and
      not by adding the two deltas.
  - **`#11-vm-native-ctor-receiver-promotion-admission`'s disposition at E1's landing** (its trigger fires on
    E1, above).
  - Close `#11-vm-json-internalize-non-array-objects` at E0's landing,
    `#11-error-object-builtintag-dual-representation` at E1's and `#11-vm-native-error-prototypes` at E2's.

## §6. Definition of done (program)

- Each PR: its per-PR plan through `/elidex-plan-review` (Slice F's under its own row, §1.2); `cargo test -p elidex-js --all-features`, then
  `mise run ci`; every silent-wrong defect it closes has a test red on the pre-fix head and green after, the
  red run recorded in the PR body; mutants run on the final head as its plan lists.
- At E2's landing: the probes `isctor/impl/m10-probe-test.rs` and `isctor/impl/e1-probe-test.rs` give the
  spec answer for every row — identity matrix diagonal only, `Object.getPrototypeOf(TypeError) === Error`,
  `EvalError` defined, every constructed error `"[object Error]"`, the forged prototype cloning as `{}`.
- Coverage: engine-independent VM tests (`vm/tests/tests_error_data.rs` (NEW) and the files the per-PR plans
  name); no WPT subset is claimed.
