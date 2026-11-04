# NativeError program (umbrella plan-memo)

**Slot**: `#11-vm-native-error-prototypes` (registered 2026-11-05, PM 363; re-scoped into three axes and two PRs,
PM 364). Also closed here: `#11-error-object-builtintag-dual-representation` (by E1). **Owner**: VM P4.
**Home**: VM P4 slice-plan umbrella 9, builtin surface (`docs/plans/2026-07-vm-p4-slice-plan.md`); minting the
row there is a landing obligation of this memo (§5), not done in this commit. **Baseline**: origin/main
`145927c9`. Paths are relative to `crates/script/elidex-js/src/` unless they start with `crates/` or `docs/`.
Spec numbers come from `.claude/tools/webref` output only. Per-PR plan for E1:
`docs/plans/2026-11-vm-native-error-e1-errordata.md` (this commit). This umbrella states the program; it does
not plan E2 in detail.

**Rev 1** (2026-11-05) — first draft, with E1's per-PR plan, for one `/elidex-plan-review`.

## §0. Frame, premises, decisions

### §0.1 The defect

Three coupled defects in the Error family, measured at `145927c9` (probes `isctor/impl/m10-probe3.log` and
`isctor/impl/e1-probe.log` in VM P4 session `0fe71d24`'s scratchpad):

- **A — object structure.** `register_error_constructors` (`vm/globals_errors.rs:23`) gives `Error`,
  `TypeError`, `ReferenceError`, `RangeError`, `SyntaxError` and `URIError` one shared `.prototype`
  (`:66-89`; its docstring `:24-32` admits it). The 5×5 prototype identity matrix is all `===`,
  `TypeError.prototype.name` is `"Error"`, `TypeError.prototype.constructor !== TypeError`, `EvalError` is not
  defined, and `Error.prototype` has no `constructor` (`:34-63` define `toString`, `name`, `message` only).
  ECMA-262 §20.5.6 NativeError Object Structure: each NativeError has its own prototype (§20.5.6.3) whose
  `[[Prototype]]` is %Error.prototype%, with own `constructor`, `message`, `name` (§20.5.6.3.1-3).
- **B — construction sites choose the prototype, and the constructors' `[[Prototype]]`.** The throw path
  (`vm_error_to_thrown`, `vm/ops.rs:171`, alloc `:201-206`) always uses `error_prototype`; call-mode
  `error_entry` (`vm/natives.rs:487`) does too; `clone_error` (`vm/host/structured_clone.rs:638`) keeps the
  source's prototype with an `error_prototype` fallback (`:656`). Every error constructor is allocated with
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
engine-thrown TypeError. So A and B are one PR, terminal by necessity. **C is correct alone**: it changes no
prototype, and it is the precondition for E2 (E2's construction sites must make the kind, and E2's clone
prototype rule needs C's classifier). The design inside each PR is its per-PR plan's.

### §0.3 Premise errors found while planning E1 (reported, not edited into the program)

Full statements in the E1 memo §0.3. The ones that touch the program:

- **The creation census is wider than the brief's list** (E1 P-2): the three `WebAssembly.*Error`
  constructors share `error_ctor_impl`, the Wasm bridge (`wasm_error_to_js_value`, `vm/host/wasm/errors.rs:179`)
  and `Promise.any` (`build_aggregate_error`, `vm/natives_promise_combinator.rs:258`) create objects the spec
  gives `[[ErrorData]]`. E1 includes them (E1 OD-3). E2's construction-site census must include them too.
- **E1 is not "regresses nothing" on its own** (E1 P-5, P-6). One consumer branches on
  `ObjectKind::Ordinary` (JSON.parse's reviver walk, `vm/natives_json/parse.rs:444` / `:469`), and the clone's
  prototype rule is the source's, so two clone rows change. Both are PM decisions (E1 OD-1, OD-2); OD-2 (a)
  adds the row E0 to this program (§1).
- **Map / Set do not exist in the VM** (E1 P-3), so they are no precedent; the precedent is `Date`'s
  `create_date` / `promote_to_date` (E1 §1.2).

## §1. Shape and verdict

The program crosses three invariant axes (A, B, C) with two independently live groups, so it is an umbrella
(CLAUDE.md "Edge-dense work"): one per-PR plan per row, each through `/elidex-plan-review` before
implementation. A per-PR plan that passes plan-review under this umbrella is a terminal unit.

| PR | Scope | Depends on | Closes | Plan-review |
|---|---|---|---|---|
| E1-pre | Pure-move split of `vm/host/structured_clone.rs` (1077 lines; E1 touches `classify` and `clone_error`) into `vm/host/structured_clone/` (NEW): `mod.rs` (entry, recursive walk, binding), `classify.rs` (`CloneKind` + `classify`, the `:208-456` section), `cloners.rs` (the per-kind cloners, the `:457-904` section). CLAUDE.md "1000-line debt", touch-time; the section dividers at `:59`, `:84`, `:208`, `:457`, `:905`, `:916` are the seams | — | — | not required (pure move; `git diff --color-moved`) |
| E0 (**proposed, PM OD-2**) | JSON.parse's InternalizeJSONProperty (ECMA-262 §25.5.2.4) step 5.c walks every non-array object; elidex walks only `Ordinary` / `Arguments` (`vm/natives_json/parse.rs:444`, `_ => {}` `:469`). Silent-wrong today for Date, RegExp, wrappers, thrown errors; E1 would add constructed errors to that set. Proposed slot `#11-vm-json-internalize-non-array-objects` | — | proposed slot | required (small, but its acceptance is E1's T-JSON) |
| E1 | Axis C: every `[[ErrorData]]` creator makes `ObjectKind::Error` (payload dropped); every classifier keys on it (structured clone, IndexedDB; toString already does) | E1-pre; E0 under OD-2 (a) | `#11-error-object-builtintag-dual-representation`; axis C of `#11-vm-native-error-prototypes` | required — `docs/plans/2026-11-vm-native-error-e1-errordata.md` |
| E2 | Axes A + B (§4): own NativeError prototypes and EvalError, every construction site's prototype, constructor `[[Prototype]]` = %Error% | E1 | `#11-vm-native-error-prototypes` | required — **its own per-PR plan and its own `/elidex-plan-review` before E2 is implemented**; not planned here |

**Order**: E1-pre and E0 in either order (no code dependency between them) → E1 → E2.

## §2. Edges and invariants

- **E1 → E2 (the PM's edge).** E2's construction sites (§4) write the kind that E1 introduces, and E2's
  prototype choice for clones is keyed on the serialized name of an object E1 already routes by kind. If E2
  landed first, every new own prototype would be one more prototype that the identity classifiers
  (`is_error_like_proto`, `is_error_like`) do not know, so a `new RangeError()` would clone as `{}` and store
  in IndexedDB silently — the regression PM 364 named.
- **E1-pre → E1.** Mechanical: E1's diff lands in the split files.
- **E0 → E1** (OD-2 (a) only). E1 moves constructed errors out of `ObjectKind::Ordinary`; E0 removes the one
  consumer that treats that kind as "walkable object". Without E0, E1's DoD cannot keep T-JSON's
  constructed-error row green.
- **The program invariant at E2's landing**: an object is an error iff it was created with `[[ErrorData]]`
  (one kind, one predicate); its prototype is the intrinsic its creator names (one rule per site, no shared
  prototype); and no site recognises an error by prototype identity.

## §3. Spec coverage map

Program-level: the clauses each PR owns, so the per-PR maps can be checked against it. Numbers from
`.claude/tools/webref heading ecma262 20.5`, `.claude/tools/webref aoid ecma262 <AO>` and
`.claude/tools/webref dfn html <AO>`. E1's step-level map is its memo's §3.

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| ECMA-262 §20.5.4 Properties of Error Instances | — | instances have [[ErrorData]]; prototypes do not | E1: `vm/object_kind.rs:135` + E1 memo §1.1's eight creators | ✓ | yes |
| ECMA-262 §20.1.3.6 Object.prototype.toString ( ) | 8 | [[ErrorData]] → "Error" | E1: `vm/natives_symbol.rs:284` (unchanged arm, now reached) | ✓ | yes |
| WHATWG HTML §2.7.4 StructuredSerializeInternal ( value, forStorage [ , memory ] ) | 18 | [[ErrorData]] and not a platform object | E1: `vm/host/structured_clone.rs:133`, `:242`; the name normalisation (18.1-18.2) is OD-1 / SR-1 | ✓ | yes |
| WHATWG HTML §2.7.7 StructuredDeserialize ( serialized, targetRealm [ , memory ] ) | 22 | prototype by serialized name (22.1-22.7); object with [[ErrorData]] (22.9) | 22.9: E1 (`clone_error`); 22.1-22.7: E2, or E1 under OD-1 (b) | ✓ | yes |
| ECMA-262 §20.5.1.1 Error ( message [ , options ] ) | 1-2 | OrdinaryCreateFromConstructor « [[ErrorData]] » (E1); intrinsicDefaultProto %Error.prototype% (E2) | `vm/natives.rs:487` (`error_entry`) | ✓ | yes |
| ECMA-262 §20.5.6.1.1 NativeError ( message [ , options ] ) | 1-2 | same with %NativeError.prototype% — E1 the slot, E2 the prototype | `vm/natives.rs:507-545` | ✓ | yes |
| ECMA-262 §20.5.5.1 EvalError | — | the constructor and prototype exist; no engine path throws one (`VmErrorKind` has no variant, `vm/error.rs:20-46`) | E2: `vm/globals_errors.rs` | ✓ | no |
| ECMA-262 §20.5.6.2 Properties of the NativeError Constructors | — | [[Prototype]] is %Error% | E2: `vm/shape_ops.rs:281-288` callers in `vm/globals_errors.rs:75-89` | ✓ | no |
| ECMA-262 §20.5.6.3 Properties of the NativeError Prototype Objects | — | own object, [[Prototype]] %Error.prototype%, own `constructor` / `message` / `name` | E2: `vm/globals_errors.rs:66-89` | ✓ | no |
| ECMA-262 §20.5.3.1 Error.prototype.constructor | — | own `constructor` on %Error.prototype% | E2: `vm/globals_errors.rs:34-63` | ✓ | no |
| ECMA-262 §20.5.7.2 Properties of the AggregateError Constructor | — | [[Prototype]] is %Error% | E2: `vm/globals_errors.rs:114-115` | ✓ | no |
| ECMA-262 §27.5.4.3.1 PerformPromiseAny ( iteratorRecord, ctor, resultCapability, promiseResolve ) | 5.b.ii.1, 5.e.vii.1 | a newly created AggregateError object | E1: `vm/natives_promise_combinator.rs:258` | ✓ | yes |
| ECMA-262 §25.5.2.4 InternalizeJSONProperty ( holder, name, reviver, parseRecord ) | 5.c | every non-array object is walked | E0: `vm/natives_json/parse.rs:444` | ✓ | yes — the reviver |

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
  `145927c9`, `git grep -n 'EvalError' 145927c9 -- crates/script/elidex-js/src` → no output, so measure,
  do not add by symmetry); call-mode `error_entry`; `clone_error` (by serialized name — unless E1 took it
  under OD-1 (b), in which case E2 only changes what the name lookup returns); `build_aggregate_error` and
  `wasm_error_to_js_value` keep theirs. Every site listed in E1 memo §1.1 is in E2's census.
- **The instance-level own `.name`** that `error_ctor_impl` (`vm/natives.rs:455-461`), the throw path
  (`vm/ops.rs:209-214`) and `build_aggregate_error` define is a workaround for the shared prototype; with
  own prototypes the spec's inherited `name` is the source of truth. E2 decides its removal (E1 memo §1.2).
- **Not E2**: `do_new`'s GetPrototypeFromConstructor fallback (E1 memo SD-5) is engine-wide; E2 meets it when
  `ctor.prototype` is replaced and must say whether it consumes or defers it.

## §5. Out of scope, stop-and-report items, and slots

- **Stop-and-report** (found while planning E1; not taken, per the brief's rule):
  - **SR-1** — clone fidelity beyond the prototype (HTML §2.7.4 step 18 / §2.7.7 step 22): `clone_error`
    copies every own data property, the spec copies `message` only. Proposed slot
    `#11-structured-clone-error-record-fidelity`.
  - **SR-2** — DOMException's `[[ErrorData]]` (Web IDL §3.14.1): a predicate that admits a platform object,
    kept apart from step 18. No consumer in elidex until `Error.isError`. E1 does not change DOMException's
    recognition (E1 memo §1.5).
  - **E0** — the JSON internalize axis (OD-2), above.
- **Siblings, proposed slots** (E1 memo §5): `#11-vm-error-install-error-cause` (InstallErrorCause, §20.5.9.1,
  silent), `#11-vm-error-iserror` (Error.isError, §20.5.2.1, loud), `#11-vm-aggregate-error-step-order`
  (silent), `#11-domexception-serializable` (silent), `#11-vm-get-prototype-from-constructor-fallback`
  (silent, engine-wide). None is registered by this commit; the PM registers or rejects each.
- **Landing obligations**: mint the program's row under VM P4 slice-plan umbrella 9
  (`docs/plans/2026-07-vm-p4-slice-plan.md`) — that edit touches the VM P4 family, so it runs
  `python3 .claude/tools/plan-memo-umbrella-check.py docs/plans/2026-07-vm-p4-es-language-completeness.md`
  (CLAUDE.md "VM-P4 umbrella memo の checker 実行"); close `#11-error-object-builtintag-dual-representation`
  at E1's landing and `#11-vm-native-error-prototypes` at E2's.

## §6. Definition of done (program)

- Each PR: its per-PR plan through `/elidex-plan-review`; `cargo test -p elidex-js --all-features`, then
  `mise run ci`; every silent-wrong defect it closes has a test red on the pre-fix head and green after, the
  red run recorded in the PR body; mutants run on the final head as its plan lists.
- At E2's landing: the probes `isctor/impl/m10-probe-test.rs` and `isctor/impl/e1-probe-test.rs` give the
  spec answer for every row — identity matrix diagonal only, `Object.getPrototypeOf(TypeError) === Error`,
  `EvalError` defined, every constructed error `"[object Error]"`, the forged prototype cloning as `{}`.
- Coverage: engine-independent VM tests (`vm/tests/tests_error_data.rs` (NEW) and the files the per-PR plans
  name); no WPT subset is claimed.
