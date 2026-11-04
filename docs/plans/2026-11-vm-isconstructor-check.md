# `#11-vm-construct-isconstructor-check` — IsConstructor at every `[[Construct]]` entry (per-PR plan-memo)

**Slot**: `#11-vm-construct-isconstructor-check` (registered in the live ledger 2026-11-03, PM 310; family
§8 row in `docs/plans/2026-07-vm-p4-slot-ledger.md`). **Owner**: VM P4. **Order**: first in the VM P4
lane, ahead of 1aa (PM 310). **Umbrella**: the approved VM P4 umbrella (#506) — this slot owns
EvaluateNew step 5 at `do_new`, which the umbrella's §3 row 17 and slice 1b's module cell had assigned to
1b and now assign here (§2). **Prereq — landed**: the standalone pure-move PR #588 (`4fd85f4e`, PM 322)
moved `is_constructor` unchanged, gate included, from `vm/object_kind.rs` into
`vm/construct_admission.rs`; this PR rebases onto it. **Baseline**: `115a667c`. Every
`file:line` below is at `115a667c` unless it names another revision; §6 re-derives each one. The cites of
`is_constructor` at `115a667c` are `vm/object_kind.rs` gate `:1672`, fn `:1673`, JS-function arm
`:1684-1690`, end `:1700`; at `4fd85f4e` the same items are `vm/construct_admission.rs:35`, `:36`,
`:47-53`, `:63`. Between `115a667c` and origin/main `4fd85f4e` the crate changes in `vm/tests/mod.rs`
(+1 line, 201 → 202), the `tests_range.rs` split, and #588's move (`vm/construct_admission.rs` new,
`vm/object_kind.rs` −55, `vm/mod.rs` +1 at `:12`, and the call-path edits at `vm/dispatch.rs:998`,
`vm/host/custom_elements/define.rs:49` and `vm/host/typed_array_static.rs:146`, `:287`); the driver
re-derives the cites in those files after the rebase (§6). Paths are relative to
`crates/script/elidex-js/src/` unless they start with `crates/` or `docs/`.

**The defect, stated as a defect.** Neither construct entry point applies ECMA-262 IsConstructor to
its callee. `new` constructs generators, async functions, async generators and every kind of method;
`super(...)` (written or in the default derived constructor) constructs anything, and when the super
constructor is a function whose call returns without pushing a frame (generator, async, async
generator, async arrow, generator method, async method, or a bound wrapper of one) the re-entrant
`run()` executes the **caller's** frame, so the enclosing function returns early with the constructed
object as its value. That is silent in a release build and a `debug_assert_eq!` panic in a debug
build.

## §0. Scope, premises, decisions

### §0.1 Scope

**In scope — exactly the two `[[Construct]]` entry points and the one predicate they share:**

- `do_new` (`vm/ops.rs:681`, reached from `Op::New` at `vm/dispatch.rs:728-730`). Today it rejects a
  non-Object callee (`:685-687`), an arrow (`:709-714`) and the two non-constructable native shapes
  (`:716-752`), and nothing else. Every `new` expression reaches it, spread arguments included:
  `ExprKind::New` always emits `Op::New` (`compiler/expr.rs:88-93`) and `compile_arguments` compiles a
  spread as one plain argument (`compiler/expr_member.rs:70-75`).
- `construct_synchronous` (`vm/dispatch_class.rs:232`). Callers: `dispatch_super_inner`
  (`vm/dispatch_class.rs:139`; both `Op::SuperCall` and `Op::SuperCallSpread`, so both the written
  `super(...)` and the synthesized default derived constructor) and the custom-element upgrade
  (`vm/host/custom_elements/upgrade.rs:315`). Today its only test is "is an `ObjectKind::Function`,
  or a native whose shape can construct" (`:265-281`).
- The predicate `is_constructor` (`vm/object_kind.rs:1673` at `115a667c`; `vm/construct_admission.rs`
  at `4fd85f4e`, #588) and the compile-time fact it must read (§1.3). Its other consumers — the
  class-heritage check (`vm/dispatch.rs:998`, ClassDefinitionEvaluation step 8.g), `customElements.define`
  (`vm/host/custom_elements/define.rs:49`) and the TypedArray `IsConstructor` gates
  (`vm/host/typed_array_static.rs:146`, `:287`) — change only through the predicate, and §4 pins the
  heritage and `customElements.define` ones.

**Not in scope:**

- Slice S's D-S1 (ordering) stays on S. This slot is S's former D-S2 limb and nothing else of S.
- Missing built-ins (`Object.hasOwn`, `Reflect.construct`, `Proxy`) are not this. Neither `Reflect` nor
  `Proxy` exists in the VM (§1.1), so neither is an entry point to fix; both are
  `#11-vm-proxy-reflect`'s (`Reflect.construct` in slice-plan row **10a**, Proxy `[[Construct]]` in row
  **10b**), and §5 records what they consume from this slot.
- The built-ins that should `[[Construct]]` their receiver and do not (`Array.from`, `Array.of`,
  `Promise.resolve`, the TypedArray statics' TypedArrayCreateFromConstructor): each is a missing
  construct site (an admission for `Promise.resolve` and the TypedArray statics, a branch on the
  predicate for `Array.from` / `Array.of`), stated as a defect in §1.1 and homed in §5.
- The **argument values** of `new X(...args)`: a spread is passed as one argument (the array), owned by
  `#11-vm-call-spread-arguments` (umbrella 1b's child for `new F(...xs)`). The admission is not affected —
  the call is `Op::New` → `do_new` — so T-NEW spells the spread form (§4). `Op::NewSpread` is never
  emitted (`git grep -n NewSpread 115a667c -- crates/script/elidex-js/src/compiler` → no output); its arm
  `vm/dispatch.rs:879-883` is dead (umbrella `:518`, "confirmed dead").
- The sibling defects §5 lists, each slotted (`#11-vm-non-constructor-own-prototype`,
  `#11-webidl-interface-object-isconstructor`, `#11-inline-event-handler-not-constructor`).

### §0.2 Premises re-measured

| Premise (brief / live ledger) | At `115a667c` | Command |
|---|---|---|
| `do_new` at `ops.rs:681`, arrow rejection at `:709-713` | holds (comment `:709`, test `:710-714`) | `git grep -n 'fn do_new' 115a667c -- crates/script/elidex-js/src/vm/ops.rs`; `sed -n 709,713p` |
| `construct_synchronous` at `dispatch_class.rs:232` | holds | `git grep -n 'fn construct_synchronous' 115a667c -- crates/script/elidex-js` |
| `is_constructor` at `object_kind.rs:1673` | holds (moved to `vm/construct_admission.rs:36` by #588, `4fd85f4e`) | `git grep -n 'fn is_constructor' 115a667c -- crates/script/elidex-js` |
| debug panic at `interpreter.rs:210` | holds (the `debug_assert_eq!` on `frames.len()` in `with_call_mode`) | `sed -n 208,216p crates/script/elidex-js/src/vm/interpreter.rs` |
| line numbers unchanged since `eaa741b5` | holds: no commit in `eaa741b5..115a667c` touches the crate | `git diff --stat eaa741b5 115a667c -- crates/script/elidex-js` prints nothing |
| `new (function*(){})()`, `new ({m(){this.z=1}}).m()`, `new (async function(){})()` construct | holds (probe P1, §6) | — |
| super path over-pops with X = generator | holds, and is wider (§0.3 P-2) | — |
| `new X(...args)` is not constructed (`Op::NewSpread` pushes `undefined`) | **false** — the compiler never emits `Op::NewSpread`; `new X(...args)` is `Op::New` → `do_new` (§0.1) | `git grep -n NewSpread 115a667c -- crates/script/elidex-js/src/compiler` → no output |

### §0.3 Premise errors

- **P-1 — "no compiler change" (live-ledger entry) was false.** `is_constructor` decides a JS function
  from `this_mode`, `is_async` and `is_generator` only (`vm/object_kind.rs:1684-1690`), and nothing in
  `CompiledFunction` (`bytecode/compiled.rs:42-81`) or `FunctionObject` (`vm/value.rs:588-599`)
  distinguishes a method, getter or setter from a function declaration. Probe P1 measures the
  consequence: `class B extends ({m(){}}).m {}` is accepted (heritage uses the predicate and the
  predicate says *constructor*). A method is not a constructor because DefineMethod and
  MethodDefinitionEvaluation never call MakeConstructor, and only the compiler knows which creation path
  a function took, so the predicate needs a compile-time fact (§1.3). The PR touches `compiler/` and
  `bytecode/compiled.rs`. PM 318 accepted this; the live entry no longer carries the claim.
- **P-2 — the over-pop is not generator-only.** Release probe P1 returns `"x|[object Object]|2"` for
  every callee whose call short-circuits in `push_js_call_frame` without pushing a frame
  (`vm/interpreter.rs:866-906` async, `:912-` generator): generator, async function, async generator,
  async arrow, generator method, async method, and a bound generator. A non-async arrow and a plain
  method push a frame, so they construct silently but do not over-pop.
- **P-3 — `"x|10,20,object|2"` is the control value, not the post-fix value of the brief's program.**
  With the check in place, `new B()` throws inside `inner`, so the literal program ends in an uncaught
  TypeError. The regression cell catches inside `inner` (§4 T-POP) so the caller observes a value:
  `"x|10,20,TE|2"` after the fix. `"x|10,20,object|2"` is what the same cell returns for a real
  constructor, and §4 keeps it as that cell's control.
- **P-4 — `is_constructor` is `#[cfg(feature = "engine")]`** (`vm/object_kind.rs:1672`) while `do_new`,
  `construct_synchronous` and the heritage call site are not gated. Inside the workspace build the gate
  is invisible because feature unification turns `engine` on; a standalone
  `cargo check -p elidex-js --lib` (default features) fails today with 16 errors, one of them E0425 for
  `is_constructor` at `vm/dispatch.rs:998`. The predicate is ECMAScript core and depends on nothing
  engine-side, so §1.3 removes the gate (#588 moved it unchanged, gate included). The other 15
  errors belong to `#11-elidex-js-default-features-build-broken` (owner CI/tools; the profile question
  is `#11-elidex-js-no-default-features-support`). **Landing obligations** — the same three sites §5
  names: (1) in `#11-elidex-js-default-features-build-broken`, drop `dispatch.rs:998` from that slot's
  site list; the PR body states the default-features error count before/after (exactly one fewer, the
  `is_constructor` E0425 gone). The live slot records no count (PM 336), so the obligation is relative,
  not a literal (PM 318); (2) in `#11-elidex-js-no-default-features-support`,
  drop `object_kind::is_constructor` (the slot's wording; `construct_admission::is_constructor` since
  #588) from the errors it names; (3) the family's quote of the same 16 errors
  in `docs/plans/2026-07-vm-p4-slot-ledger.md:310` (`#11-vm-typed-array-family-layering-and-gate`) is
  pinned to `907f3135` by its own text, so it stays true and gets no edit — a note only.
- **P-5 — `Op::NewSpread` is not an entry.** The ledger and the previous revision of this memo treated
  `new X(...args)` as a stub outside `do_new`. It is not (§0.1, §0.2); the spread spelling is in this
  PR's path.

### §0.4 Open decision — homed, not chosen here

**OD-1 — is a WebIDL interface object without a constructor operation a constructor?** Homed as
`#11-webidl-interface-object-isconstructor` (PM 318; loud, not silent). The VM models
`CallShape::IllegalConstructor` as having neither `[[Call]]` nor `[[Construct]]`
(`vm/value.rs:700-712`), so `is_constructor` returns false and `class X extends Crypto {}` throws at
class definition. WebIDL §3.7.1 creates every interface object with CreateBuiltinFunction and says such
an object "will throw when called, both as a function and as a constructor"
(`.claude/tools/webref body webidl interface-object`). ECMA-262 §10.3 Built-in Function Objects states
that a built-in function object "has a `[[Construct]]` internal method if and only if" it is described
as a constructor or an algorithm sets it (`.claude/tools/webref body ecma262 sec-built-in-function-objects`),
while it states `[[Call]]` unconditionally ("must have a `[[Call]]` internal method"). So the `[[Call]]`
half of the VM's model departs from §10.3 on any reading, and the `[[Construct]]` half turns on whether
WebIDL's prose "describes it as a constructor" — the question that slot settles. The two readings differ
only at the predicate's consumers (heritage, `customElements.define`, the TypedArray gates) and in the
super-path message; `new Crypto()` is a TypeError either way. **This PR keeps the native classification
unchanged**: the classifier's native arm is today's `CallShape` split.

## §1. Design

### §1.1 Construct entry points (census)

Found by `git grep -n 'CallMode::Construct {' 115a667c -- crates/script/elidex-js/src ':!crates/script/elidex-js/src/vm/tests'`
(5 hits: the four creation sites below plus the `NativeContext::new_construct` constructor at
`vm/value.rs:839`) and `git grep -n 'construct_synchronous(\|call_construct_native(\|\.do_new(' 115a667c -- crates/script/elidex-js/src ':!crates/script/elidex-js/src/vm/tests'`.

| Entry | Site | Checks IsConstructor today? | In scope? |
|---|---|---|---|
| `new F(...)` and `new F(...xs)` — `do_new` | `vm/ops.rs:681` (from `vm/dispatch.rs:730`; the compiler emits `Op::New` for both spellings, `compiler/expr.rs:88-93`) | partially: arrows and non-constructable natives only | **yes** — same defect |
| `super(...)` and the default derived constructor — `construct_synchronous` via `dispatch_super_inner` | `vm/dispatch_class.rs:139` → `:232` | no (JS function kind is not inspected) | **yes** — same defect |
| custom-element upgrade — `construct_synchronous` | `vm/host/custom_elements/upgrade.rs:315` | indirectly: the constructor was admitted at define time by `is_constructor` (`define.rs:49`) | covered by the `construct_synchronous` fix, no own change |
| native construct — `call_construct_native` | `vm/interpreter.rs:95`, sole caller `vm/ops.rs:808` | reached only after `do_new`'s native check | no own change (behind `do_new`) |
| `Op::NewSpread` | `vm/dispatch.rs:879-883` | n/a: never emitted (`git grep -n NewSpread 115a667c -- crates/script/elidex-js/src/compiler` → no output) | **no** — dead arm, not an entry |
| `Reflect.construct`, Proxy `construct` trap | — | n/a: neither exists (`git grep -c 'Proxy' 115a667c -- crates/script/elidex-js/src/vm/object_kind.rs crates/script/elidex-js/src/vm/value.rs` → no output); owned by `#11-vm-proxy-reflect` (`Reflect.construct`: slice-plan row **10a**; Proxy `[[Construct]]`: row **10b**) | no — hand-off in §5 |
| `Array.from` / `Array.of` receiver | `vm/natives_array_hof.rs:489` / `:556` | **missing branch, silently wrong**: both take `_this` and never read it, so they never test IsConstructor(*C*) and never Construct(*C*), and `class A extends Array {}; A.from([1]) instanceof A` is `false` (read from the signatures at `115a667c`, not probed). These are a **branch, not an admission**: `Array.from` steps 5.a and 9 and `Array.of` step 4 are `If IsConstructor(C) … Construct(C) … Else ArrayCreate`, so a non-constructor receiver is not refused (`Array.from.call({}, [1])` returns a plain Array) | no — slice-plan row **9a** (§5): branches on the predicate |
| `Promise.resolve` receiver | `vm/natives_promise.rs:760` | **missing entry, silently wrong**: takes `_this`, so PromiseResolve(*C*, *x*) (§27.5.4.7.1) and its NewPromiseCapability(*C*) (§27.5.1.5, step 1 IsConstructor, step 6 Construct) are skipped — a subclass receiver gets a plain `%Promise%` and a non-constructor receiver is not refused (read, not probed). **Admission shape**: NewPromiseCapability step 1 throws a TypeError when IsConstructor(*C*) is false | no — slice-plan row **9da** (§5): consumes the admission |
| TypedArray statics' TypedArrayCreateFromConstructor | `vm/host/typed_array_static.rs:116-121` | **missing entry, silently wrong**: TypedArrayCreateFromConstructor (§23.2.4.1) step 1 is `Construct(ctor, argList)`; the statics only *test* with `is_constructor` (`:146`, `:287`) and build the instance directly, so a subclass constructor's body never runs — the "deferred to a follow-up PR" comment names no slot. **Admission shape**: the AO's `ctor` parameter is a constructor and its step 1 is the Construct | no — `#11-vm-typed-array-create-construct` (§5): consumes the admission |

### §1.2 What the predicate must answer, against what it answers

Spec: IsConstructor (§7.2.4) is "has a `[[Construct]]` internal method". An ECMAScript function gets
one only from MakeConstructor (§10.2.5) step 1.c; a built-in function gets one from its step 2.a, or by
being described as a constructor (§10.3). The callers of MakeConstructor that create ECMAScript
functions: InstantiateOrdinaryFunctionObject (§15.2.4, step 5; step 4 for the `export default` form),
InstantiateOrdinaryFunctionExpression (§15.2.5, step 7 anonymous; step 10 named), and
ClassDefinitionEvaluation (§15.7.14) step 17 for the class constructor — the written one created at
step 15.a by DefineMethod (MakeConstructor branch 1.c), the default one created at step 14.b by
CreateBuiltinFunction (branch 2.a). The creation paths that do **not** call it: DefineMethod (§15.4.4,
OrdinaryFunctionCreate + MakeMethod at steps 6-7); MethodDefinitionEvaluation (§15.4.5) for getters
(steps 6-7), setters (5-6), generator methods, async generator methods and async methods (5-6 each);
InstantiateGeneratorFunctionObject / Expression (§15.5.3 / §15.5.4); InstantiateAsyncGeneratorFunctionObject
/ Expression (§15.6.3 / §15.6.4); InstantiateAsyncFunctionObject / Expression (§15.8.2 / §15.8.3);
InstantiateArrowFunctionExpression (§15.3.4); InstantiateAsyncArrowFunctionExpression (§15.9.4). A bound
function has `[[Construct]]` iff its target does (BoundFunctionCreate §10.4.1.3 step 6).

Measured at `115a667c` (probe P1, debug and release, §6). "accepts" = constructs or is accepted as
heritage; "TE" = TypeError.

| Callee kind | Spec | `is_constructor` | `new` | `super` (default or written ctor) | `extends` |
|---|---|---|---|---|---|
| generator / async / async generator | false | false | accepts | accepts + **over-pop** (release), panic (debug) | TE |
| async arrow | false | false | TE | accepts + over-pop / panic | TE |
| arrow | false | false | TE | accepts | TE |
| object-literal method, class method, static method | false | **true** | accepts | accepts | **accepts** |
| generator method, async method | false | false | accepts | accepts + over-pop / panic | TE |
| getter / setter (object literal and class) | false | **true** | accepts | accepts | **accepts** |
| bound generator | false | false | accepts | accepts + over-pop / panic | TE |
| bound method | false | **true** | accepts | accepts | TE (by step 8.h.ii, a bound function having no `prototype`, not by 8.g) |
| function declaration / expression, class | true | true | accepts | accepts | accepts |
| bound function, bound class | true | true | accepts | accepts | TE by step 8.h.ii (spec-correct) |
| `Object`, `Error` (constructor natives) | true | true | accepts | accepts | accepts |
| `Math.max` (`CallShape::CallableOnly`) | false | false | TE | TE | TE |

So the predicate is wrong for exactly the method / getter / setter class, and the entries are wrong
because they do not consult it.

### §1.3 One canonical form

**Home.** Everything this PR adds or rewrites on the VM side lives in one module,
`vm/construct_admission.rs` (created by #588 with `is_constructor` moved into it unchanged):
the `ConstructAdmission` classifier, the rewritten `is_constructor`, and the shared admission helper.
That module rather than `vm/dispatch_class.rs` (347 lines) because #588 already made it the predicate's
home and the classifier, the predicate and the helper are one IsConstructor unit; `dispatch_class.rs`
keeps the construct *dispatch*. `vm/object_kind.rs` lost `is_constructor` in #588 and gains nothing here.
`unwrap_bound_function_chain` stays in `vm/dispatch_class.rs` unchanged; the helper and the predicate
call it. **Gating**: every item in the module is ECMAScript core and has an ungated caller (`do_new`,
`construct_synchronous`, `vm/dispatch.rs:998`), so the module carries no `#[cfg(feature = "engine")]` —
neither the per-item gates #588 moved (`vm/construct_admission.rs:7` on the `ObjectKind` import, `:35`
on `is_constructor`, at `4fd85f4e`) nor a gate on its `mod` line (`vm/mod.rs:12`, ungated as #588 left
it). The module header (`:1-5`), which says the follow-up slot "adds" the classifier and helper, is
rewritten to say what the module holds.

**The compile-time fact.** `CompiledFunction` gains `has_construct: bool` (NEW) — "MakeConstructor
applies to the function object created from this code". `compile_nested_function`
(`compiler/expr_function.rs:95`) replaces its unused `_is_declaration: bool` parameter with a
`FunctionCreation` (NEW) naming the spec creation path, so every caller states which path it is on:

- `FunctionCreation::Ordinary` (NEW) — InstantiateOrdinaryFunctionObject / Expression and their generator,
  async and async-generator siblings: `compiler/expr.rs:211`, `compiler/expr_function.rs:169`,
  `compiler/mod.rs:84`. The bit is `true` only when the function is neither async nor a generator
  (§15.5 / §15.6 / §15.8 never call MakeConstructor).
- `FunctionCreation::Method` (NEW) — DefineMethod / MethodDefinitionEvaluation: `compiler/expr_class.rs:305`
  (class and static methods, accessors, generator and async methods) and `:354` (private methods), and
  the object-literal route below. Bit `false`.
- `FunctionCreation::ClassConstructor` (NEW) — the written class constructor (`compiler/expr_class.rs:121`,
  which already sets `is_class_ctor`): ClassDefinitionEvaluation step 15.a creates it by DefineMethod and
  step 17 applies MakeConstructor. Bit `true`.
- The synthesized class constructor (`synthesize_default_class_ctor`, `compiler/expr_class.rs:69`) is
  not compiled through `compile_nested_function`; it sets `has_construct: true` in its struct literal
  (step 14.b + step 17, MakeConstructor branch 2.a).
- Arrows and async arrows (`compiler/expr.rs:217` → `compile_arrow_function`) keep the default `false`.

**Object-literal methods and accessors are the site that needs a new route**: their value is an
ordinary `ExprKind::Function` compiled through `compile_expr` (`compiler/expr_object.rs:74-75` for
`PropertyKind::Init`, `compile_accessor` at `:14-24` for `Get`/`Set`), so the creation path is lost by the
time `compiler/expr.rs:211` sees it. One helper (NEW, in `compiler/expr.rs` next to the
`ExprKind::Function` arm) compiles a function value for a given `FunctionCreation`: it records the
expression's source-map entry the way `compile_expr` does (`compiler/expr.rs:31-32`), calls
`compile_nested_function`, and emits `Op::Closure`. The `ExprKind::Function` arm calls it with
`Ordinary`; `compile_object_expr` calls it with `Method` for a value whose property carries
`PropertyFlags::method()` and `compile_accessor` calls it with `Method` for `Get`/`Set`. A
`{ f: function(){} }` value stays `Ordinary` (it is a function expression; §4 controls it).

**The default is `false`**, so a creation site that nobody classified yields a loud TypeError on `new`
rather than a silent construct — the fail-safe direction for an admission predicate. **One known
creation site gets the bit and should not, and this PR does not change it**: inline event handlers are
compiled from source as a function expression (`vm/host/event_handler_attrs.rs:675-677`), so they take
the `Ordinary` path, while HTML "getting the current value of the event handler" step 3.9 creates them
with OrdinaryFunctionCreate and no MakeConstructor (`.claude/tools/webref body html getting-the-current-value-of-the-event-handler`).
They construct at base too, so the behaviour is unchanged; the fix is `#11-inline-event-handler-not-constructor`
(owner Layout, trigger = this slot's landing, PM 322).

**The predicate.** One classifier over a non-bound object returning a `ConstructAdmission` (NEW):
`Construct`, `NotConstructor`, `CallableOnlyNative(name)`, `IllegalConstructorNative(name)`. The JS arm
reads `has_construct` and nothing else (the `this_mode` / `is_async` / `is_generator` tests at
`vm/object_kind.rs:1685-1689` at `115a667c` are deleted, their content now being part of the bit); the
native arm is today's `CallShape` split (OD-1 leaves it as is); every other kind is `NotConstructor`.
`is_constructor` keeps its signature and becomes `unwrap + matches!(…, Construct)`; its
`#[cfg(feature = "engine")]` is removed (P-4). **One bound-chain walk**: today `is_constructor` has its
own loop (`vm/object_kind.rs:1682`), which returns `false` past `MAX_BIND_CHAIN_DEPTH`, while the
entries' `unwrap_bound_function_chain` raises a RangeError there — two pieces of code applying one depth
limit with two outcomes. The rewritten `is_constructor` drops its loop and calls
`unwrap_bound_function_chain`, mapping that call's depth error to `false` (a predicate answers, it does
not throw; ClassDefinitionEvaluation step 8.g, `customElements.define` and the TypedArray gates then
raise their own TypeError), so the depth limit is applied by one piece of code and the predicate
callers keep today's outcome. Out of scope and unchanged: the third walker,
`unwrap_bound_chain` in `vm/host/typed_array_static.rs:91-100`, which resolves the TypedArray kind
rather than admitting a construct; it is `#11-vm-typed-array-create-construct`'s to fold (§5).

**The admission.** One helper, used by both entries in place of their own checks. It takes the callee
as a `JsValue` and the entry (`New` / `Super`), and owns IsConstructor step 1 (a non-Object is refused)
for both entries; for an Object it unwraps the bound chain with `unwrap_bound_function_chain`
(`vm/dispatch_class.rs:180`, keeping its `Maximum bind chain depth exceeded` RangeError at `:199-201`),
classifies the target, and maps a refusal to the error. `do_new` loses its own non-Object let-else
(`vm/ops.rs:685-687`), its arrow test (`:709-714`) and its native `match` (`:716-752`), and calls the
helper where it now calls `unwrap_bound_function_chain` (`:695`); `construct_synchronous` replaces its
unwrap (`:256`) and its kind test (`:267-281`) the same way. Checking the unwrapped target is
IsConstructor of the wrapper by BoundFunctionCreate step 6.

**The null super constructor moves into the admission.** Today the superCtor read refuses `null`
(`dispatch_super_inner`'s `.prototype.ok_or_else(… "Super constructor null is not a constructor")`,
`vm/dispatch_class.rs:121-124`) — a refusal outside the admission, for SuperCall step 5's IsConstructor,
which webref places after step 4 (`.claude/tools/webref body ecma262 sec-super-keyword-runtime-semantics-evaluation`).
After this PR the read no longer throws: it hands the resolved superCtor (`JsValue::Null` included) to
`construct_synchronous`, whose callee parameter becomes a `JsValue` (`ctor_id: ObjectId` today, `:234`),
and the admission refuses `null` with the `Super` entry's message. The custom-element upgrade passes
`JsValue::Object(constructor)` (`vm/host/custom_elements/upgrade.rs:315`). So every IsConstructor refusal
at both entries, `null` included, is the admission's. **Which entry each caller passes** (driver's
decision): `construct_synchronous` takes the entry as a parameter. `dispatch_super_inner` passes `Super`.
The custom-element upgrade passes `New`: HTML "upgrade an element" step 10.3 ("Let constructResult be the
result of constructing C, with no arguments", `.claude/tools/webref body html concept-upgrade-an-element`)
performs `Construct(C)` directly, the same shape as `new`; it always passes `JsValue::Object`, so the key
is only ever read by the non-Object arm and changes no behaviour.

**Placement.** Both entries run the admission after the arguments are evaluated and before any
allocation, frame push or stack splice: EvaluateNew (§13.3.5.1.1) step 5 follows step 4's
ArgumentListEvaluation, and SuperCall evaluation (§13.3.7.1) step 5 follows step 4. `do_new` runs with the
arguments already on the stack; `construct_synchronous` receives them as a slice. §4 T-ORD pins both,
with a callee whose push runs its body.

**Frame balance.** After admission no callee whose call is frameless can reach the JS branch of
`construct_synchronous`, so the over-pop is removed at its cause, not guarded. The JS branch states
the invariant it now relies on as a `debug_assert!` that the frame count grew by one before `vm.run()`
(`vm/dispatch_class.rs:311-319`) — a guard against a later frameless kind, not a second check. It is
debug-only, like the `debug_assert_eq!` at `vm/interpreter.rs:210` that the original defect slipped past
in release, and that is deliberate: a release check would cost a branch on every construct, a hot path,
and the structural guarantee is elsewhere — the admission at the cause plus the classifier's fail-safe
default (`has_construct` defaults to `false`, unknown kinds are `NotConstructor`), so a kind nobody
classified is refused before it reaches the push. T-POP is release-observable by its value (§4.1: the
cell reads the caller's locals, so a release over-pop is a wrong value, not a silent pass); CI runs debug
nextest, where the `debug_assert!` fires first; no gate runs it in release.

**Error wording** is the crate's existing set, now produced in one place: a JS non-constructor or
non-callable kind → `"not a constructor"` (`vm/ops.rs:686`, `:712`, `:750`;
`vm/dispatch_class.rs:280`); `CallShape::CallableOnly` → `"{name} is not a constructor"`
(`vm/ops.rs:744-747`, `vm/dispatch_class.rs:278`); `CallShape::IllegalConstructor` →
`VmError::illegal_constructor` (`vm/ops.rs:737-741`). The super path gains the last arm, which it
lacks today (`vm/dispatch_class.rs:265-281` reports an `IllegalConstructor` native as
`"{name} is not a constructor"`): one mapping, both entries. **The non-Object arm is the one arm keyed
by the entry**, so both existing messages are kept: `New` → `"not a constructor"` (`new null()`, today
`vm/ops.rs:686`), `Super` → `"Super constructor null is not a constructor"` (today
`vm/dispatch_class.rs:124`; GetSuperConstructor yields an Object or `null`, so `null` is the only
non-Object the `Super` entry sees). Every other arm ignores the entry. Unchanged: the heritage message
`"Class extends value is not a constructor or null"` (`vm/dispatch.rs:1001`).

**Docstrings and comments.** `construct_synchronous`'s claim that it is invoked from "`do_new`'s JS
branch" (`vm/dispatch_class.rs:220-221`) is false — `do_new` pushes its frame inline (`vm/ops.rs:784-793`)
and the second caller is the custom-element upgrade. Two comments name the `can_construct()` gates this PR
removes and are rewritten to name the admission helper: `vm/interpreter.rs:328-329` and
`vm/host/events.rs:578`. Further implementation obligations, same commit as the code they sit on:

- **`is_constructor`'s doc** (`vm/construct_admission.rs:10-34` at `4fd85f4e`): "has a `[[Construct]]`
  internal slot" → "internal method" (IsConstructor step 2's wording); the paragraph on JS functions,
  which describes the `this_mode` / `is_async` / `is_generator` test, is rewritten to say the JS arm reads
  `has_construct`, and the walk paragraph to say it is `unwrap_bound_function_chain`'s.
- **The module header** (`vm/construct_admission.rs:1-5`): rewritten to describe what the module holds
  (classifier, predicate, admission helper), not what a follow-up adds.
- **ClassDefinitionEvaluation step drift.** The comments cite "step 6 / 6.f / 6.f.i / 6.f.ii / 6.f.iii";
  the current text (`.claude/tools/webref body ecma262 ClassDefinitionEvaluation`) has the heritage in
  step 8: 8.c evaluates it, 8.f is the null branch (8.f.i protoParent null, 8.f.ii ctorParent
  `%Function.prototype%`), 8.g IsConstructor false → TypeError, 8.h the constructor path (8.h.i
  `Get(superclass, "prototype")`, 8.h.ii protoParent neither Object nor null → TypeError). The old
  sub-step letters also disagree with each other (`tests_class_machinery.rs:218` labels protoParent
  `6.f.iii`, `:271` labels it `6.f.i`). This PR changes the heritage check's answer through the predicate (no
  code edit at `vm/dispatch.rs:998`), so it re-cites every site of the concept by AO name plus these step numbers: `vm/dispatch.rs:967`,
  `:982-986`; `vm/tests/tests_class_machinery.rs:199`, `:218`, `:227`, `:252`, `:271`, `:280`, and the
  three D4 did not list, `:319` (8.g), `:345` (8.h.ii), `:357` (8.c); `bytecode/opcode.rs:355`;
  `compiler/expr_class.rs:20`, `:200`, `:217`, `:232-234` (sweep:
  `git grep -n '6\.f\|step 6\b' 4fd85f4e -- crates/script/elidex-js/src`, filtered to §15.7.14 /
  heritage cites — 20 matching lines).
- **`vm/host/typed_array_static.rs:116-121`**: the "deferred to a follow-up PR" sentence is re-pointed to
  `#11-vm-typed-array-create-construct` (PM 332), and its `TypedArrayCreate(C, ⟨len⟩)` is spelled by
  the current AO name, TypedArrayCreateFromConstructor (§23.2.4.1).

The new items cite by AO name, with numbers from `webref aoid`:

| Item | Cites |
|---|---|
| `has_construct` | MakeConstructor (§10.2.5) steps 1.c / 2.a |
| `FunctionCreation` | `Ordinary`: InstantiateOrdinaryFunctionObject (§15.2.4) / Expression (§15.2.5) and the §15.5 / §15.6 / §15.8 instantiations; `Method`: DefineMethod (§15.4.4), MethodDefinitionEvaluation (§15.4.5); `ClassConstructor`: ClassDefinitionEvaluation (§15.7.14) steps 15.a and 17 |
| `ConstructAdmission` + classifier | IsConstructor (§7.2.4); §10.3's built-in `[[Construct]]` "if and only if" sentence for the native arm, with a pointer to `#11-webidl-interface-object-isconstructor` |
| `is_constructor` | IsConstructor (§7.2.4); BoundFunctionCreate (§10.4.1.3) step 6 for the walk |
| admission helper | IsConstructor (§7.2.4) step 1 (non-Object); EvaluateNew (§13.3.5.1.1) step 5; SuperCall Evaluation (§13.3.7.1) step 5; ClassDefinitionEvaluation (§15.7.14) step 14.a.iv.3; Construct (§7.3.14), whose caller guarantees a constructor |

**Layering / ECS.** All of this is ECMAScript core in `vm/` and `compiler/`; nothing moves into
`vm/host/`. Its census at `115a667c`: predicate callers `vm/host/custom_elements/define.rs:49` and
`vm/host/typed_array_static.rs:146`, `:287` (unchanged code, new answer through the predicate); the
entry caller `vm/host/custom_elements/upgrade.rs:315`, which reaches the admission through
`construct_synchronous` (now passing `JsValue::Object`) and writes no check; a comment-only edit at `vm/host/events.rs:578`; and the
comment re-point at `vm/host/typed_array_static.rs:116-121`. No component, side store or session state
is involved.

## §2. Edge density, by measurement

Criterion (slice-plan §5's head): an axis is evidenced only by a failure that occurs while the others
pass; count the axes that intersect. Two measurements: the base tree, whose three columns apply the
predicate to different degrees (`extends` applies the full predicate, `new` an arrow-only fragment,
`super` none), and the §4.2 mutants, each read against the §4.1 cell bodies for the set of cells it
fails (derived here; the post-implementation run in §4.2 records it).

- **A1, classification** (is the callee kind classified per spec, bound arm included). Base evidence:
  in the `extends` column the generator / async / arrow rows pass while the method / getter / setter rows
  fail (§1.2). Mutants M3, M4, M5, M6, M6b, M7, M10.
- **A2, entry coverage** (does every entry consult the predicate). Base evidence: the generator row
  passes in `extends` and fails in `new` and `super`, where the predicate is correct and not consulted.
  Mutants M1, M2, M9.
- **A3, refusal before any callee effect** (the admission precedes every allocation, frame push and
  body execution). Mutants M8, M8b — measured below.

The null super constructor (§1.3) adds no axis: its refusal is the admission's non-Object arm, on the
**same admission line** in `construct_synchronous` that A2 and A3 already decide — A2 because the
`Super` entry must consult that arm (M11, M12), A3 because the line runs after the arguments (the null
half is S's acceptance; in this PR the line cannot move ahead of the arguments).

| Mutant | Cells it fails | Fails alone? | Invariant it breaks | Folded into | Meets the other axes at |
|---|---|---|---|---|---|
| **M8** admission's JS-function refusal after `push_js_call_frame` in `construct_synchronous` (the native arm stays before the branch, §4.2) | T-ORD's super half only: the async X's push runs its body (`vm/interpreter.rs:866-906`), so `log` is `1,2`. Every TypeError cell still gets its TypeError, and T-POP still reads `x\|10,20,TE\|2`, because `with_call_mode`'s Err path pops the frames and truncates the stack back to entry (`vm/interpreter.rs:248-260`) | yes | refusal precedes every callee effect | **A3** | A2: the one admission line in `construct_synchronous`; A1: the kinds whose push runs a body are async kinds that A1 classifies `NotConstructor` |
| **M8b** admission's JS-function refusal after the inline frame push in `do_new` (`vm/ops.rs:784-793`; native arm as M8) | T-ORD's new half only (`log` is `1,2`) | yes | same | **A3** | A2: the one admission line in `do_new`; A1: as M8 |
| **M9** the super path keeps its own native message | T-MSG's `Crypto` half only | yes | each entry consults the one admission, its error mapping included | **A2** (error arm) | owned by A2 |
| **M11** the admission's non-Object arm for the `Super` entry removed | T-ORD's null half only (exact message) | yes | each entry consults the one admission, its non-Object arm included | **A2** (error arm) | owned by A2 |
| **M12** the entry key collapsed (one non-Object message for both entries) | T-MSG's `new null()` cell or T-ORD's null half, whichever message was dropped | yes | same | **A2** (error arm) | owned by A2 |
| **M10** the admission walks the chain with `is_constructor` before unwrapping | T-DEPTH (over-depth becomes a TypeError); also T-MSG / T-NEW, whose native messages the predicate's generic throw replaces | yes | a bound function is classified as its target, with the depth RangeError | **A1** (bound arm) | owned by A1 |

M8 is not a third over-pop route: for a frame-pushing kind the frame it pushed is popped by the same
`with_call_mode` Err path. M8b has no such boundary in `do_new`; what it does to a frame-pushing kind is
not derived here, and T-ORD kills it regardless.

**Frame balance is not a fourth axis.** Every over-pop row is a row that fails A2 in the `super` column;
no measured row over-pops while its TypeError expectation holds, and moving the admission (M8) does not
reintroduce it. T-POP is kept because it is the release-observable form of the corruption, not because
it measures an axis.

**Three measured intersecting axes. The PR is terminal**, on two grounds (PM 326):

1. **The base case of CLAUDE.md "Edge-dense work" (c)**: a plan-reviewed, narrowly scoped slice under an
   approved umbrella is a terminal unit. This slot sits under the approved VM P4 umbrella (#506): it owns
   EvaluateNew step 5 at `do_new`, which the umbrella's §3 row 17
   (`docs/plans/2026-07-vm-p4-es-language-completeness.md`) and slice 1b's module cell
   (`docs/plans/2026-07-vm-p4-slice-plan.md`) assigned to 1b and now transfer here, and it carries a
   family §8 row in `docs/plans/2026-07-vm-p4-slot-ledger.md`.
2. **Terminal by necessity**, the family rule in `#11-vm-operand-rooting-by-construction`
   (`docs/plans/2026-07-vm-p4-slot-ledger.md:306` at `115a667c`: a row that cannot be split without
   defeating its deliverable is terminal by necessity; applied to 1aa by PM 288). A2 and A3 are decided
   by the same admission line in each entry. The only possible split separates the two, and a slice that adds an entry's
   admission without fixing its position ships M8 — a refusal after an async body already ran, which is
   silent-wrong.

**Re-slice trigger**: the post-implementation mutant run (§4.2) finds a mutant that fails alone and
breaks an invariant no counted axis owns.

## §3. Spec coverage map

Section numbers from `.claude/tools/webref aoid ecma262 <AO>`; step text from
`.claude/tools/webref body ecma262 <AO-or-anchor>`. SuperCall's evaluation is a syntax-directed
operation without an aoid; its number and steps come from
`.claude/tools/webref body ecma262 sec-super-keyword-runtime-semantics-evaluation` (heading "13.3.7.1
Runtime Semantics: Evaluation"). Skeleton from `.claude/tools/webref coverage-map`; §6 has the command.
OrdinaryFunctionCreate (§10.2.3) and MakeMethod (§10.2.7), which the coverage-map skeleton lists, have no
row of their own: neither sets `[[Construct]]`, and every path that calls them appears below as a creation
row whose branch is "no MakeConstructor". The breadth this table reaches (more rows than touch sites) is
the creation-path census, not a second subsystem; the single-PR verdict is §2's.

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| ECMA-262 §7.2.4 IsConstructor ( arg ) | 1-3 | non-Object → false; has `[[Construct]]` → true; else false | `is_constructor` + the classifier (NEW), `vm/construct_admission.rs` | ✓ every `ObjectKind` reaches one arm; JS arm reads `has_construct` | yes — any script value |
| ECMA-262 §13.3.5.1.1 EvaluateNew ( constructExpr, argumentsNode ) | 5 | IsConstructor false → TypeError, after step 4's argument evaluation (spread spelling included); a non-Object callee (`new null()`) is IsConstructor step 1, the admission's non-Object arm with the `New` message (`vm/ops.rs:685-687`'s let-else removed) | `vm/ops.rs:681` (`do_new`) | ✓ | yes |
| ECMA-262 §13.3.7.1 Runtime Semantics: Evaluation | SuperCall 5 | IsConstructor(superCtor) false → TypeError, after step 4; a `null` superCtor is the admission's non-Object arm with the `Super` message (moved from the read at `vm/dispatch_class.rs:121-124`) | `vm/dispatch_class.rs:139` → `:232` | ✓ | yes — `Object.setPrototypeOf` on the class |
| ECMA-262 §13.3.7.2 GetSuperConstructor ( ) | 5 | superCtor = the active function's `[[GetPrototypeOf]]()` — the script-writable input SuperCall step 5 tests | read by `dispatch_super_inner` (`vm/dispatch_class.rs:121-124`); no longer throws on `null`, hands the value to the admission | ✓ | yes — `Object.setPrototypeOf` |
| ECMA-262 §15.7.14 Runtime Semantics: ClassDefinitionEvaluation | 14.a.iv.3 | default derived ctor: IsConstructor(func) false → TypeError | synthesized ctor → `Op::SuperCallSpread` → `vm/dispatch_class.rs:232` | ✓ | yes |
| ECMA-262 §15.7.14 Runtime Semantics: ClassDefinitionEvaluation | 8.g | heritage not a constructor → TypeError (consumer; changes through the predicate only). Siblings in step 8: 8.f, a `null` heritage, never consults the predicate (`vm/dispatch.rs:997`'s `JsValue::Null` disjunct); 8.h.ii, a `prototype` that is neither Object nor null → TypeError (the `_` arm of `Op::SetPrototype`, `vm/dispatch.rs:965`). A bound method moves from 8.h.ii (base: the predicate says *constructor*, then `Get(bound, "prototype")` is `undefined`) to 8.g — still a TypeError, and with the same text, since both sites throw `"Class extends value is not a constructor or null"` (`:973`, `:1001`), so the move is not observable by message | `vm/dispatch.rs:998` | ✓ | yes |
| ECMA-262 §15.7.14 Runtime Semantics: ClassDefinitionEvaluation | 17 | MakeConstructor on the class constructor: written (step 15.a, branch 1.c) and default (step 14.b, branch 2.a) | `compiler/expr_class.rs:121` (`FunctionCreation::ClassConstructor` (NEW)), `:69` (struct literal) | ✓ | no |
| ECMA-262 §10.2.5 MakeConstructor ( func [ , writableProto [ , proto ] ] ) | 1.c / 2.a | sets `[[Construct]]`: ECMAScript function (1.c) / built-in, i.e. the default class ctor (2.a) | `has_construct` (NEW) in `bytecode/compiled.rs` | ✓ | no |
| ECMA-262 §15.2.4 Runtime Semantics: InstantiateOrdinaryFunctionObject | 5 (named) / 4 (`default`) | declarations are constructors | `compiler/expr_function.rs:169`, `compiler/mod.rs:84` (`Ordinary`) | ✓ | no |
| ECMA-262 §15.2.5 Runtime Semantics: InstantiateOrdinaryFunctionExpression | 7 (anonymous) / 10 (named) | expressions are constructors | `compiler/expr.rs:211` (`Ordinary`) | ✓ | no |
| ECMA-262 §15.4.4 Runtime Semantics: DefineMethod | 6-7 | OrdinaryFunctionCreate + MakeMethod, no MakeConstructor → `has_construct` false | `compiler/expr_object.rs:74-75` (object literal), `compiler/expr_class.rs:305`, `:354` (`Method`) | ✓ | no |
| ECMA-262 §15.4.5 Runtime Semantics: MethodDefinitionEvaluation | get 6-7 | getter: OrdinaryFunctionCreate + MakeMethod, no MakeConstructor → false | `compiler/expr_object.rs:14-24` (`compile_accessor`), `compiler/expr_class.rs:305` (`Method`) | ✓ | no |
| ECMA-262 §15.4.5 Runtime Semantics: MethodDefinitionEvaluation | set 5-6 | setter: same → false | `compiler/expr_object.rs:14-24`, `compiler/expr_class.rs:305` (`Method`) | ✓ | no |
| ECMA-262 §15.4.5 Runtime Semantics: MethodDefinitionEvaluation | GeneratorMethod 5-6 | no MakeConstructor → false (steps 8-9 install `prototype`: SD-1) | `compiler/expr_object.rs:74-75`, `compiler/expr_class.rs:305`, `:354` (`Method`) | ✓ | no |
| ECMA-262 §15.4.5 Runtime Semantics: MethodDefinitionEvaluation | AsyncGeneratorMethod 5-6 | no MakeConstructor → false (steps 8-9 install `prototype`: SD-1) | same sites (`Method`) | ✓ | no |
| ECMA-262 §15.4.5 Runtime Semantics: MethodDefinitionEvaluation | AsyncMethod 5-6 | no MakeConstructor → false | same sites (`Method`) | ✓ | no |
| ECMA-262 §15.5.3 Runtime Semantics: InstantiateGeneratorFunctionObject | whole AO (and §15.5.4 InstantiateGeneratorFunctionExpression) | no MakeConstructor → false | `Ordinary` sites; bit false because `is_generator` | ✓ | no |
| ECMA-262 §15.6.3 Runtime Semantics: InstantiateAsyncGeneratorFunctionObject | whole AO (and §15.6.4 InstantiateAsyncGeneratorFunctionExpression) | no MakeConstructor → false | `Ordinary` sites; bit false because `is_async` and `is_generator` | ✓ | no |
| ECMA-262 §15.8.2 Runtime Semantics: InstantiateAsyncFunctionObject | whole AO (and §15.8.3 InstantiateAsyncFunctionExpression) | no MakeConstructor → false | `Ordinary` sites; bit false because `is_async` | ✓ | no |
| ECMA-262 §15.3.4 Runtime Semantics: InstantiateArrowFunctionExpression | whole AO | no MakeConstructor → false | `compiler/expr.rs:217` → `compile_arrow_function`; default | ✓ | no |
| ECMA-262 §15.9.4 Runtime Semantics: InstantiateAsyncArrowFunctionExpression | whole AO | no MakeConstructor → false | `compiler/expr.rs:217` → `compile_arrow_function`; default | ✓ | no |
| ECMA-262 §10.4.1.3 BoundFunctionCreate ( targetFunc, boundThis, boundArgs ) | 6 | `[[Construct]]` iff target has one | `vm/dispatch_class.rs:180` (unwrap) + classifier on the target | ✓ | yes |
| ECMA-262 §7.3.14 Construct ( ctor [ , argList [ , newTarget ] ] ) | header precondition; 3 is the call | the caller guarantees ctor is a constructor; step 3 calls `ctor.[[Construct]]` | both entries after admission | ✓ | no |

### §3.1 User-input audit

The callee of `new`, the `[[Prototype]]` of a derived class (writable by `Object.setPrototypeOf` at any
time, read by GetSuperConstructor) and the heritage value are all script-controlled. The over-pop makes
the super path the one with a control-flow consequence: a page can make an enclosing function return
early with an attacker-chosen object, and the frames resynchronize only at the eval boundary. After this
PR every script-controlled value reaches the classifier before any frame or allocation effect, and the
classifier's unknown-kind arm is `NotConstructor`.

## §4. Tests

### §4.1 Cells

New file `crates/script/elidex-js/src/vm/tests/tests_construct_isconstructor.rs` (NEW), registered in
`vm/tests/mod.rs` (201 lines at `115a667c`, 202 after the rebase onto `fab44a1d`). No existing file is
grown: `tests_class_machinery.rs` (389) would fit but is scoped to class lowering, and the
`customElements.define` cell goes in the new file too, with its own bound-document setup through
`vm/test_helpers.rs`'s `bind_vm` (the document builder in `tests_custom_elements.rs`, 1320 lines, is
private to that file and is not reused by growing it). Callee kinds, one table shared by the cells:
generator, async, async generator, async arrow, arrow, object-literal method, generator method, async
method, class method, static method, object getter, object setter, class getter, bound generator,
bound method; controls: function declaration, function expression, a `{ f: function(){} }` property
value, class without a constructor, class with a written constructor, bound function, bound class,
`Object`, `Error`.

The T-POP body, written once and used by T-POP and T-POP-CTL (`X` is substituted per kind):

```js
const X = <kind>; class A {} class B extends A {} Object.setPrototypeOf(B, X);
function inner() { let a = 10, b = 20; let r;
  try { r = typeof new B(); } catch (e) { r = e instanceof TypeError ? 'TE' : 'other'; }
  return [a, b, r].join(','); }
function outer() { let x = 'x'; const s = inner(); return x + '|' + s + '|' + (1 + 1); }
outer()
```

| Cell | Asserts | Base (`115a667c`) | Mutant it kills |
|---|---|---|---|
| T-NEW | `new X()` → TypeError, message `not a constructor`, per non-constructor kind (arrow and async arrow already pass); spread spelling `new X(...[1])` with X = generator → TypeError, and `new F(...[1])` with F = a function declaration constructs (`typeof` is `object`); native non-constructor on the `new` path: `new Math.max()` → TypeError, message `max is not a constructor` (`CallShape::CallableOnly`), so a native hole in `do_new` is seen | red for the other 13; spread: red for the generator (it constructs), the control green; `new Math.max()` green (§1.2's `Math.max` row, `new` column) | M1 (no admission in `do_new`), M3, M4, M5; the `Math.max` cell kills an M8b built with the native arm moved too |
| T-SUPER-DEF | `class B extends A {}` + `Object.setPrototypeOf(B, X)` → `new B()` TypeError, per kind | red for all 15 (debug: panic for the 7 frameless kinds) | M2, M3, M4, M5 |
| T-SUPER-EXP | same with `constructor(){ super(); }` | red for all 15 | M2 |
| T-POP | the body above → `"x\|10,20,TE\|2"`, per frameless kind | red: debug panics at `vm/interpreter.rs:210`; release — probed with the variant body `try { r = new B() } catch (e) { r = 'T' } return [a, b, typeof r].join(',')`, which returned `"x\|[object Object]\|2"`; this exact body is not probed at base | M2, M5 (frameless kinds) |
| T-POP-CTL | the body above with X = each control → `"x\|10,20,object\|2"` | green (stays green) | M6, M6b |
| T-EXTENDS | `class B extends X {}` → TypeError for the method / getter / setter kinds | red for 6 | M3, M4 |
| T-CTL | every control constructs through `new` and through both super forms; `extends` accepts function, both classes, `Object`, `Error` | green | M6, M6b, M7 |
| T-ORD | the three bodies below, each in its own fresh `Vm` (so each half has its own `log`), the new and super halves → `"TE\|1"`, the null half → `"TE:Super constructor null is not a constructor\|1"`. An async callee because its push runs the body (`vm/interpreter.rs:866-906`); a generator's push runs none, so a generator callee would not separate the orders | red: the new half constructs, `"object\|1,2"`; the super half over-pops (release) / panics (debug). The null half is **green at base**: today the read refuses `null` inside the opcode, which runs after the arguments (by reading `vm/dispatch_class.rs:121-124`, not probed) | M8 (super half), M8b (new half), M11, M12 (null half) |
| T-MSG | `Object.setPrototypeOf(B, Crypto)` → `new B()` message equals `new Crypto()`'s (`Illegal constructor`); `Object.setPrototypeOf(B, Math.max)` → `max is not a constructor`; `new null()` → TypeError, message **equals** `not a constructor` | red for the first — not probed; at base the super path builds `{name} is not a constructor` (`vm/dispatch_class.rs:276-278`); the second is green (probed); `new null()` green by reading `vm/ops.rs:685-687` (not probed) | M9; M12 (the `new null()` cell) |
| T-DEPTH | a bound chain one past `MAX_BIND_CHAIN_DEPTH` (`vm/mod.rs:105`, 10 000) → RangeError at both entries: `new deep()`, and `Object.setPrototypeOf(B, deep)` → `new B()` | green by reading `vm/dispatch_class.rs:199-201` (not probed) | M10 |
| T-CE | `customElements.define('x-m', ({m(){}}).m)` and the same with an object getter (name `x-g`) → TypeError whose message **equals** `Failed to execute 'define' on 'CustomElementRegistry': parameter 2 is not a constructor.` (`define.rs:52-53`), and afterwards `customElements.get('x-m') === undefined` / `customElements.get('x-g') === undefined`; control `customElements.define('x-c', class extends HTMLElement {})` defines. ⚠ **The message is load-bearing**: at base a method passes `is_constructor` and is then refused by the brand check `validate_html_element_constructor_chain` (`html_element.rs:301-336`) with a *different* TypeError, so an "any TypeError" assertion is green at base and under M3 | red for both by message (base: the brand check's TypeError); control green | M3 |

The T-ORD bodies, written once; each half runs in a fresh `Vm` (a shared `log` would read `1,1` or
`1,2,1` in a later half):

```js
// new half
const log = []; async function X(){ log.push(2) }
let r; try { r = typeof new X(log.push(1)); } catch (e) { r = e instanceof TypeError ? 'TE' : 'other'; }
r + '|' + log.join()
```

```js
// super half
const log = []; async function X(){ log.push(2) }
class A {} class B extends A { constructor(){ super(log.push(1)); } }
Object.setPrototypeOf(B, X);
let r; try { r = typeof new B(); } catch (e) { r = e instanceof TypeError ? 'TE' : 'other'; }
r + '|' + log.join()
```

```js
// null half
const log = [];
class A {} class B extends A { constructor(){ super(log.push(1)); } }
Object.setPrototypeOf(B, null);
let r; try { r = typeof new B(); } catch (e) { r = e instanceof TypeError ? 'TE:' + e.message : 'other'; }
r + '|' + log.join()
```

**What the null half pins.** It is green at base, so it is not a regression cell for this PR's
defect. It has two jobs. (1) This PR's pin that the `null` refusal goes through the admission with the
`Super` message: M11 and M12 fail it. (2) The acceptance Slice S's D-S1 must keep green: when S moves
the superCtor read ahead of the arguments, the refusal must stay in the admission after them, so `log`
stays `1` (§5's S hand-off). A mutant that keeps the throw in the read is **not** added: until S moves
the read, the read runs after the arguments too, so no cell can kill it here.

### §4.2 Mutants

Run on the final head only, sequentially (≤ 2 parallel builds), each recorded as killed-by-cell:

- **M1** remove the admission from `do_new`. **M2** remove it from `construct_synchronous`.
- **M3** object-literal methods and accessors stay on the `compile_expr` route, so they are compiled
  as `Ordinary` and get the bit. **M4** class methods and accessors pass `Ordinary`.
- **M5** `has_construct` ignores `is_async` / `is_generator`.
- **M6** `has_construct` stays `false` for the synthesized class constructor (kills the
  class-without-constructor control and every `class A {}` super cell).
- **M6b** the written class constructor passes `Method` instead of `ClassConstructor` (kills the
  class-with-written-constructor control).
- **M7** the classifier is applied to the bound wrapper instead of the unwrapped target (bound
  controls fail).
- **M8** the admission's JS-function refusal moved after `push_js_call_frame` in `construct_synchronous`.
- **M8b** the admission's JS-function refusal moved after the inline frame push in `do_new` (`vm/ops.rs:784-793`).
- **What M8 / M8b move** — only the refusal for JS-function targets. The native arm of the
  classification (`CallableOnlyNative` / `IllegalConstructorNative`) stays before the JS/native branch in
  each entry, because the native branch reaches no frame push: moving it too would let `Math.max`
  construct, and M8 would also fail T-MSG's `Math.max` half and no longer fail T-ORD alone (§2's table);
  in `do_new` the `new Math.max()` cell of T-NEW sees the same hole.
- **How M8 / M8b are built** — not a pure move, because the code before each push consumes the helper's
  `(target, prepended)` return (`do_new`'s splice and `extract_js_callee`, `vm/ops.rs:698-707`;
  `construct_synchronous`'s argument combine, `vm/dispatch_class.rs:257-265`). Each mutant keeps, where
  the helper was, the non-Object arm's refusal (the raw `unwrap_bound_function_chain` takes an
  `ObjectId`, so the non-Object callee is refused before it), a raw `unwrap_bound_function_chain` call and
  the native arm's refusal, and calls the helper (its result discarded except for the refusal) in the JS branch after the push. Insertion points: in `do_new`, immediately after the inline
  frame push (`vm/ops.rs:784-793`); in `construct_synchronous`, inside the `with_call_mode` closure's JS
  branch **between `push_js_call_frame` and the new frame-balance `debug_assert!`** — after the assert,
  a debug run panics for every frameless kind in T-SUPER-* / T-POP, and M8 no longer fails T-ORD alone
  (§2's table).
- **M9** the super path keeps its own native message.
- **M10** the admission walks the chain with `is_constructor` before unwrapping (over-depth becomes a
  TypeError; also fails T-MSG / T-NEW, whose native messages the generic throw replaces).
- **M11** the admission's non-Object arm for the `Super` entry removed (killed by T-ORD's null half,
  exact message).
- **M12** the entry key collapsed: one non-Object message for both entries, either one (killed by
  T-MSG's `new null()` message or by T-ORD's null half, whichever message was dropped).

A mutant no cell kills is a missing cell, added before the PR leaves review.

## §5. Non-goals, hand-offs, sibling defects

- **SD-1 — own `prototype` on methods and async functions** → `#11-vm-non-constructor-own-prototype`
  (PM 318; silent-wrong; owner VM P4, right after this slot). Probe P1:
  `Object.prototype.hasOwnProperty.call(({m(){}}).m, 'prototype')` and the same for
  `async function(){}` both return `true`; `create_closure` keys the install on `!is_arrow`
  (`vm/ops.rs:897`). **Its fix key is "MakeConstructor ran, or a generator kind"**, not `has_construct`
  alone: generators, generator methods and async generators install `prototype` without MakeConstructor
  (§15.5.3 steps 5-6 for a named declaration; MethodDefinitionEvaluation GeneratorMethod and
  AsyncGeneratorMethod steps 8-9), so keying on `has_construct` would strip a spec-required property.
  `has_construct` is one input. The previous revision of this memo said otherwise; the live slot carries
  the corrected key (PM 322).
- **OD-1** (§0.4) → `#11-webidl-interface-object-isconstructor` (PM 318; loud).
- **Inline event handlers are constructors** (§1.3) → `#11-inline-event-handler-not-constructor`
  (PM 322; owner Layout; trigger = this slot's landing). It also bears on SD-1 (they should have no own
  `prototype`).
- **`new F(...xs)`'s argument values** — the child umbrella 1b's derivation mints for `new F(...xs)`
  gives `new` a real spread path. Whatever opcode it uses, it consumes this PR's admission helper and
  writes no check of its own; EvaluateNew step 5 at `do_new` is this slot's (umbrella §3 row 17 and
  slice-plan 1b's module cell say so).
- **`Reflect.construct` and a future Proxy `[[Construct]]`** → `#11-vm-proxy-reflect`: `Reflect.construct`
  in slice-plan row **10a** (the 10a umbrella), Proxy `[[Construct]]` in row **10b** (the 10b umbrella). `Reflect.construct(target, args, newTarget)` tests IsConstructor on *both* `target` and
  `newTarget`, and a Proxy's `[[Construct]]` exists iff its target's does. Both consume this slot's
  classifier and admission helper (a Proxy as a new `ObjectKind` arm of the classifier) and write no
  check of their own.
- **Missing construct sites in built-ins (§1.1)**, each silently wrong at `115a667c` and each, when
  implemented, writing no IsConstructor test of its own (PM 332). Two have the admission's shape and
  one is a branch:
  - `Array.from` / `Array.of` ignore their receiver (`vm/natives_array_hof.rs:489`, `:556`), so a
    subclass gets a plain Array → slice-plan row **9a** (which already names `Array.from`). **A branch,
    not an admission**: `Array.from` steps 5.a / 9 and `Array.of` step 4 are `If IsConstructor(C) …
    Construct(C) … Else ArrayCreate` (`.claude/tools/webref body ecma262 sec-array.from`, `sec-array.of`),
    so `Array.from.call({}, [1])` returns a plain Array. The fix branches on the `is_constructor`
    predicate; when true, constructs through `construct_synchronous` (which applies the admission);
    writes no IsConstructor test of its own.
  - `Promise.resolve` ignores its receiver (`vm/natives_promise.rs:760`), skipping PromiseResolve /
    NewPromiseCapability(*C*) → slice-plan row **9da**. **Admission shape**: NewPromiseCapability step 1
    throws a TypeError when IsConstructor(*C*) is false, so the fix consumes the admission helper.
  - The TypedArray statics never Construct their receiver (TypedArrayCreateFromConstructor step 1;
    `vm/host/typed_array_static.rs:116-121`) → `#11-vm-typed-array-create-construct` (new, owner VM P4,
    ordered after this slot; umbrella 9's derivation mints its child row). **Admission shape**: the AO's
    `ctor` parameter is a constructor and step 1 is `Construct(ctor, argList)`, so the fix consumes the
    admission helper. That slot also owns folding the file's own bound-chain walker (`:91-100`) into the
    admission's unwrap.
  This branch adds one defect sentence to each of the 9a and 9da rows in
  `docs/plans/2026-07-vm-p4-slice-plan.md`; §1.3 re-points the TypedArray comment.
- **P-4** → the same three sites §0.3 names: `#11-elidex-js-default-features-build-broken` (drop
  `dispatch.rs:998` from its site list; the PR body states the default-features error count before/after,
  exactly one fewer, the `is_constructor` E0425 gone), `#11-elidex-js-no-default-features-support` (its `is_constructor` mention), and the note on
  `docs/plans/2026-07-vm-p4-slot-ledger.md:310`'s quote, pinned at `907f3135` and not edited.
- **`vm/object_kind.rs` (1700 lines)** — `is_constructor` was moved out by #588 (`4fd85f4e`); this PR
  adds nothing to the file.
- **Touch-time line counts** (§6): every file this PR grows is under 1000 lines at `115a667c`
  (`compiler/expr.rs` 296, `expr_class.rs` 632, `expr_object.rs` 152, `expr_function.rs` 290,
  `bytecode/compiled.rs` 221, `vm/ops.rs` 935 — which this PR shrinks — `vm/dispatch_class.rs` 347), so
  the >1000 / >50-line case does not arise. The two files over 1000 lines touched by comment edits only
  have recorded verdicts, with no prereq split (PM 336):
  - `vm/dispatch.rs` (1103): `docs/plans/2026-07-vm-p4-touch-time-split-record.md` records "Decision: no
    standalone prereq-split PR" with the forward rule "never grow an arm in place" (`git grep -n 'no
    standalone prereq-split PR\|never grow an arm in place' -- docs/plans/2026-07-vm-p4-touch-time-split-record.md`
    → `:60`, `:62`). A comment-only correction grows no arm, so it complies.
  - `vm/interpreter.rs` (1366): the same record's watch list (`#11-d17b-dispatch-expr-file-growth`,
    `git grep -n 'Files the watch list still owns' -- docs/plans/2026-07-vm-p4-touch-time-split-record.md`
    → `:81`) assigns it to umbrellas 1a and 3 through the children they mint, which puts the verdict with
    1ac (PM 336). This PR's comment-only edit does not pre-empt that verdict.
- **`vm/mod.rs` is not touched by this PR** (the module is ungated, §1.3), so its stale
  `#11-options-collection-mutable-surface` comment (`vm/mod.rs:1107-1110` at `4fd85f4e`) stays a
  fix-on-touch item for the next PR that touches `vm/mod.rs`.
- **Slice S keeps D-S1** — superCtor resolution moved ahead of the arguments (`dispatch_super_inner`
  reads GetSuperConstructor at `vm/dispatch_class.rs:121-124`, after them; SuperCall steps 3-4 put it
  before). That change moves *when* superCtor is read, not *where* it is admitted: SuperCall step 5's
  IsConstructor still follows step 4, so S keeps routing the resolved superCtor through this slot's
  admission helper (via `construct_synchronous`) and writes no IsConstructor check of its own. S moves
  the read only: after this PR the read no longer throws, and every IsConstructor refusal, `null`
  included, is the admission's (§1.3). **Acceptance S must keep green**: T-ORD's null half (§4.1,
  `"TE:Super constructor null is not a constructor|1"`). This branch adds that sentence to S's two rows on main — slice-plan row **S** (`:972`) and the slot-ledger
  row `#11-vm-arrow-lexical-super` (`:287`) — on the same pattern as the 1b hand-off (umbrella §3 row 17,
  slice-plan 1b's module cell).
- **Re-points owed on the 1a-plan branch (`vm-p4-slice-1a-plan`, `e493bd57`)**, which the driver makes
  there, not this branch: the copies of S's rows and S's D-S1 text — slice-plan `:977`, slot-ledger
  `:287`, 1aa memo (`docs/plans/2026-11-vm-p4-slice-1a-1aa.md`) `:41`, `:459`, `:474`, `:519-589`,
  `:738`, `:750`, `:801`, `:809` — get the S hand-off above; its `vm/object_kind.rs:1673` cites (1aa memo
  `:533`, `:773`, slot-ledger `:287`) are re-pointed to `vm/construct_admission.rs`; and its copy of the
  edge-26 cite in `docs/plans/2026-07-vm-p4-slice-1a-1b-call-spread-detail.md` (`vm/ops.rs:696-700`)
  gets the re-point below.
- **Edge 26's cite** (`docs/plans/2026-07-vm-p4-slice-1a-1b-call-spread-detail.md:467`, "bound-prefix
  splice (`vm/ops.rs:696-700`)") drifts when this PR rewrites `do_new`; this branch re-points it by name
  to the `prepended` half of the admission helper's `(target, prepended)` return.
- **Collision with the 1a-plan branch.** Both branches add a §8 row to
  `docs/plans/2026-07-vm-p4-slot-ledger.md` after the same predecessor row, so whichever lands second
  re-derives, instead of carrying: the slot-row count in slice-plan `:223` (51 → 52 here), the C row's
  denominator (slice-plan `:961`, 51 → 52), and slot-ledger `:208` (registration figures) and `:359`
  (carve-marker totals).
- **Live-ledger landing item for the PM** — the live entry of this slot agrees with its §8 cell on scope
  (P-1's "no compiler change" withdrawn, PM 318) and, since PM 332, on the measured design (three axes,
  §2; `ClassConstructor`, §1.3) and the owed T-POP value `"x|10,20,TE|2"` (P-3). One difference remains:
  the entry's main text files the heritage check under `construct_synchronous` ("SuperCall / class
  heritage"), where heritage goes through `is_constructor` at `vm/dispatch.rs:998`. Slot-ledger `:208`
  and the §8 row name this one difference and this item.
- **Live-ledger landing item for the PM, `#11-vm-typed-array-create-construct`** — the live entry does
  not mention the third bound-chain walker, `unwrap_bound_chain` in `vm/host/typed_array_static.rs:91-100`
  (§1.3), which that slot owns folding into the admission's unwrap (above); the PM adds it to the entry.

## §6. Re-derivation

Run from the repository root at `115a667c`.

- Probe P1 (logs and source in the session scratchpad `isctor/`: `probe-debug.log`,
  `probe-release.log`, `tests_zz_probe.rs`): a throwaway `git worktree add --detach … 115a667c`,
  `mod tests_zz_probe;` appended to `crates/script/elidex-js/src/vm/tests/mod.rs`,
  `env -u RUSTUP_TOOLCHAIN cargo test [--release] -p elidex-js --lib --all-features tests_zz_probe -- --nocapture --test-threads=1`
  (`rustc 1.99.1`). Each cell runs in a fresh `Vm` under `catch_unwind`, so a debug panic reports as
  `PANIC` without masking the following cells.
- Crate drift to origin/main: `git diff --stat 115a667c fab44a1d -- crates/script/elidex-js` →
  `vm/tests/mod.rs` (+1), `tests_range.rs` / `tests_range_stubs_validation.rs` only.
- `vm/ops.rs:681`, `:685-687`, `:695`, `:698-707`, `:709-714`, `:716-752`, `:784-793`, `:808`, `:897`:
  `sed -n <range>p crates/script/elidex-js/src/vm/ops.rs`; anchor `git grep -n 'fn do_new' -- crates/script/elidex-js`.
- `vm/dispatch_class.rs:124`, `:139`, `:180`, `:199-201`, `:220-221`, `:232`, `:256`, `:267-281`, `:311-319`:
  `sed -n` on that file; anchors `git grep -n 'fn construct_synchronous\|fn unwrap_bound_function_chain\|fn dispatch_super_inner' -- crates/script/elidex-js`.
- `vm/object_kind.rs` gate `:1672`, fn `:1673`, loop `:1682`, JS arm `:1684-1690`, end `:1700` at `115a667c`;
  `vm/construct_admission.rs:1-5`, `:7`, `:10-34`, `:35`, `:36`, `:47-53`, `:63` and `vm/mod.rs:12` at `4fd85f4e`:
  `git grep -n 'fn is_constructor\|cfg(feature = "engine")' 115a667c -- crates/script/elidex-js/src/vm/object_kind.rs` and
  `git show 4fd85f4e:<the vm/construct_admission.rs path> | grep -n 'fn is_constructor\|cfg(feature'` (the file
  exists from #588 on, not in this branch's `115a667c` tree);
  `git grep -n 'mod construct_admission' 4fd85f4e -- crates/script/elidex-js/src/vm/mod.rs`.
- `vm/dispatch.rs:728-730`, `:879-883`, `:998`, `:1001`: `git grep -n 'Op::New =>\|Op::CallSpread | Op::NewSpread\|is_constructor(self' -- crates/script/elidex-js/src/vm/dispatch.rs`.
- `Op::NewSpread` never emitted: `git grep -n NewSpread 115a667c -- crates/script/elidex-js/src/compiler` → no output;
  `compiler/expr.rs:88-93` (`git grep -n 'ExprKind::New {' -- crates/script/elidex-js/src/compiler/expr.rs`),
  `compiler/expr_member.rs:70-75` (`git grep -n 'Argument::Spread(e) =>' -- crates/script/elidex-js/src/compiler/expr_member.rs`).
- `vm/interpreter.rs:95`, `:208-216`, `:248-260`, `:328-329`, `:866-906`, `:912`: `git grep -n 'fn call_construct_native\|Ok return leaked\|while self.frames.len() > entry_frames\|can_construct()\` gate\|if is_async {\|if is_generator {' -- crates/script/elidex-js/src/vm/interpreter.rs`.
- `vm/value.rs:588-599`, `:700-712`, `:839`: `git grep -n 'pub struct FunctionObject\|CallShape::IllegalConstructor\`\] — neither\|mode: CallMode::Construct { new_target }' -- crates/script/elidex-js/src/vm/value.rs`.
- `vm/host/custom_elements/define.rs:49`, `upgrade.rs:315`, `vm/host/typed_array_static.rs:146`, `:287`:
  `git grep -n 'is_constructor(\|construct_synchronous(' -- crates/script/elidex-js/src/vm/host`.
- `vm/host/events.rs:578`: `git grep -n "can_construct() == false" -- crates/script/elidex-js/src/vm/host/events.rs`.
- `vm/host/event_handler_attrs.rs:675-677`: `git grep -n 'fn lazy_compile_handler' -- crates/script/elidex-js/src/vm/host`.
- `bytecode/compiled.rs:42-81`: `git grep -n 'pub struct CompiledFunction' -- crates/script/elidex-js`.
- `compiler/expr_function.rs:95`, `:169`, `compiler/expr.rs:31-32`, `:211`, `:217`, `compiler/mod.rs:84`,
  `compiler/expr_class.rs:69`, `:121`, `:305`, `:354`: `git grep -n 'compile_nested_function(\|fn synthesize_default_class_ctor\|fc.source_map.add(fc.pc(), span)' -- crates/script/elidex-js/src/compiler`.
- `compiler/expr_object.rs:14-24`, `:74-75`: `git grep -n 'fn compile_accessor\|fn compile_object_expr' -- crates/script/elidex-js/src/compiler/expr_object.rs`.
- §1.1 missing construct sites: `git grep -n 'fn native_array_from\|fn native_array_of' 115a667c -- crates/script/elidex-js/src/vm/natives_array_hof.rs` (`:489`, `:556`, each followed by `_this: JsValue`);
  `git grep -n 'fn native_promise_resolve' 115a667c -- crates/script/elidex-js/src/vm/natives_promise.rs` (`:760`, `_this` on the parameter line after);
  `git grep -n 'deferred to a follow-up PR' 115a667c -- crates/script/elidex-js/src/vm/host/typed_array_static.rs` (`:120`, sentence `:116-121`);
  spec: `.claude/tools/webref body ecma262 sec-array.from`, `sec-array.of`, `sec-promise.resolve`;
  `aoid ecma262 PromiseResolve` / `NewPromiseCapability` / `TypedArrayCreateFromConstructor`.
- §1.3 comment obligations: `git grep -n '6\.f\|step 6\b' 4fd85f4e -- crates/script/elidex-js/src` (20 heritage lines);
  `git grep -n 'options-collection-mutable-surface' 4fd85f4e -- crates/script/elidex-js/src/vm/mod.rs` (`:1109`; §5's fix-on-touch note);
  step numbers from `.claude/tools/webref body ecma262 ClassDefinitionEvaluation` (heritage = step 8, sub-steps a-h).
- T-CE: `sed -n 48,56p crates/script/elidex-js/src/vm/host/custom_elements/define.rs` (message `:52-53`);
  `git grep -n 'fn validate_html_element_constructor_chain' -- crates/script/elidex-js/src/vm/host` (`html_element.rs:301`).
- §3 8.g row: `sed -n 963,976p` and `sed -n 995,1002p` on `crates/script/elidex-js/src/vm/dispatch.rs` (both messages).
- File sizes: `wc -l crates/script/elidex-js/src/vm/object_kind.rs crates/script/elidex-js/src/vm/tests/tests_custom_elements.rs crates/script/elidex-js/src/vm/tests/tests_class_machinery.rs crates/script/elidex-js/src/vm/tests/mod.rs` → 1700, 1320, 389, 201.
- P-4: `env -u RUSTUP_TOOLCHAIN cargo check -p elidex-js --lib` → `could not compile … due to 16 previous errors`, including E0425 at `vm/dispatch.rs:998` (re-run at `115a667c` for R2, rustc 1.99.1: 16, same E0425).
- Touch-time line counts at `115a667c` (`git show 115a667c:<path> | wc -l`): `compiler/expr.rs` 296, `compiler/expr_class.rs` 632,
  `compiler/expr_object.rs` 152, `compiler/expr_function.rs` 290, `bytecode/compiled.rs` 221, `vm/ops.rs` 935,
  `vm/dispatch_class.rs` 347; over 1000 and touched by comment edits only: `vm/dispatch.rs` 1103, `vm/interpreter.rs` 1366
  (`vm/mod.rs`, 2782, is not touched).
- §2 ground 2: `sed -n 306p docs/plans/2026-07-vm-p4-slot-ledger.md` at `115a667c` (the `#11-vm-operand-rooting-by-construction` row, "terminal by necessity").
- §3: `.claude/tools/webref coverage-map ecma262 IsConstructor ecma262 EvaluateNew ecma262 ClassDefinitionEvaluation ecma262 Construct ecma262 MakeConstructor ecma262 OrdinaryFunctionCreate ecma262 DefineMethod ecma262 MakeMethod ecma262 GetSuperConstructor`;
  `aoid ecma262` for InstantiateOrdinaryFunctionObject, InstantiateOrdinaryFunctionExpression,
  MethodDefinitionEvaluation, InstantiateGeneratorFunctionObject, InstantiateGeneratorFunctionExpression,
  InstantiateAsyncGeneratorFunctionObject, InstantiateAsyncGeneratorFunctionExpression,
  InstantiateAsyncFunctionObject, InstantiateAsyncFunctionExpression, InstantiateArrowFunctionExpression,
  InstantiateAsyncArrowFunctionExpression, BoundFunctionCreate; step numbers from `body ecma262 <AO>`
  (each instantiation AO's body has no `MakeConstructor`: `body ecma262 <AO> | grep -c MakeConstructor` → 0).
- Preflight's one soft warning (symbol `object_kind::is_constructor` not found) is §0.3 (2)'s quote of the live slot's wording, left as written.
- §1.3 null move: `sed -n 115,125p` and `sed -n 228,240p crates/script/elidex-js/src/vm/dispatch_class.rs` (the read, the `ctor_id: ObjectId` parameter); `sed -n 315,316p crates/script/elidex-js/src/vm/host/custom_elements/upgrade.rs`; `.claude/tools/webref body ecma262 sec-super-keyword-runtime-semantics-evaluation` (SuperCall steps 3-5).
- §0.4: `.claude/tools/webref body ecma262 sec-built-in-function-objects`; `.claude/tools/webref body webidl interface-object`.
- §1.3 event handlers: `.claude/tools/webref body html getting-the-current-value-of-the-event-handler` (step 3.9).
