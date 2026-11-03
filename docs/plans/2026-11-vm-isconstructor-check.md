# `#11-vm-construct-isconstructor-check` — IsConstructor at every `[[Construct]]` entry (per-PR plan-memo)

**Slot**: `#11-vm-construct-isconstructor-check` (registered in the live ledger 2026-11-03, PM 310; family
§8 row in `docs/plans/2026-07-vm-p4-slot-ledger.md`). **Owner**: VM P4. **Order**: first in the VM P4
lane, ahead of 1aa (PM 310). **Baseline**: `115a667c`. Every `file:line` below is at `115a667c` unless it
names another revision; §6 re-derives each one. Paths are relative to
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
  (`:716-752`), and nothing else.
- `construct_synchronous` (`vm/dispatch_class.rs:232`). Callers: `dispatch_super_inner`
  (`vm/dispatch_class.rs:139`; both `Op::SuperCall` and `Op::SuperCallSpread`, so both the written
  `super(...)` and the synthesized default derived constructor) and the custom-element upgrade
  (`vm/host/custom_elements/upgrade.rs:315`). Today its only test is "is an `ObjectKind::Function`,
  or a native whose shape can construct" (`:265-281`).
- The predicate `is_constructor` (`vm/object_kind.rs:1673`) and the compile-time fact it must read
  (§1.3). Its other consumers — the class-heritage check (`vm/dispatch.rs:998`, ClassDefinitionEvaluation
  step 8.g), `customElements.define` (`vm/host/custom_elements/define.rs:49`) and the TypedArray
  `IsConstructor` gates (`vm/host/typed_array_static.rs:146`, `:287`) — change only through the
  predicate, and §4 pins the heritage one.

**Not in scope:**

- Slice S's D-S1 (ordering) stays on S. This slot is S's former D-S2 limb and nothing else of S.
- Missing built-ins (`Object.hasOwn`, `Reflect.construct`, `Proxy`) are not this. Neither `Reflect` nor
  `Proxy` exists in the VM (§1.1), so neither is an entry point to fix.
- `new X(...args)` is not constructed at all: `Op::NewSpread` pushes `undefined` (`vm/dispatch.rs:879-883`),
  owned by `#11-vm-call-spread-arguments` (Slices 1a/1b). Recorded as a hand-off in §5, not fixed here.
- The sibling defects §5 lists (own `prototype` on methods and async functions; the native
  `IllegalConstructor` classification, OD-1).

### §0.2 Premises re-measured

| Premise (brief / live ledger) | At `115a667c` | Command |
|---|---|---|
| `do_new` at `ops.rs:681`, arrow rejection at `:709-713` | holds (comment `:709`, test `:710-714`) | `git grep -n 'fn do_new' 115a667c -- crates/script/elidex-js/src/vm/ops.rs`; `sed -n 709,713p` |
| `construct_synchronous` at `dispatch_class.rs:232` | holds | `git grep -n 'fn construct_synchronous' 115a667c -- crates/script/elidex-js` |
| `is_constructor` at `object_kind.rs:1673` | holds | `git grep -n 'fn is_constructor' 115a667c -- crates/script/elidex-js` |
| debug panic at `interpreter.rs:210` | holds (the `debug_assert_eq!` on `frames.len()` in `with_call_mode`) | `sed -n 208,216p crates/script/elidex-js/src/vm/interpreter.rs` |
| line numbers unchanged since `eaa741b5` | holds: no commit in `eaa741b5..115a667c` touches the crate | `git diff --stat eaa741b5 115a667c -- crates/script/elidex-js` prints nothing |
| `new (function*(){})()`, `new ({m(){this.z=1}}).m()`, `new (async function(){})()` construct | holds (probe P1, §6) | — |
| super path over-pops with X = generator | holds, and is wider (§0.3 P-2) | — |

### §0.3 Premise errors

- **P-1 — "no compiler change" (live-ledger entry) is false.** `is_constructor` decides a JS function
  from `this_mode`, `is_async` and `is_generator` only (`vm/object_kind.rs:1680-1687`), and nothing in
  `CompiledFunction` (`bytecode/compiled.rs:42-81`) or `FunctionObject` (`vm/value.rs:588-599`)
  distinguishes a method, getter or setter from a function declaration. Probe P1 measures the
  consequence: `class B extends ({m(){}}).m {}` is accepted (heritage uses the predicate and the
  predicate says *constructor*). A method is not a constructor because DefineMethod never calls
  MakeConstructor, and only the compiler knows which creation path a function took, so the predicate
  needs a compile-time fact (§1.3). The ordering decision does not move (still one PR, §2), but the PR
  touches `compiler/` and `bytecode/compiled.rs`, which the ledger entry said it would not.
- **P-2 — the over-pop is not generator-only.** Release probe P1 returns `"x|[object Object]|2"` for
  every callee whose call short-circuits in `push_js_call_frame` without pushing a frame
  (`vm/interpreter.rs:866-906` async, `:912-` generator): generator, async function, async generator,
  async arrow, generator method, async method, and a bound generator. A non-async arrow and a plain
  method push a frame, so they construct silently but do not over-pop.
- **P-3 — `"x|10,20,object|2"` is the control value, not the post-fix value of the brief's program.**
  With the check in place, `new B()` throws inside `inner`, so the literal program ends in an uncaught
  TypeError. The regression cell catches inside `inner` (§4 T-POP) so the caller observes a value:
  `"x|10,20,TE|2"` after the fix, `"x|[object Object]|2"` at base. `"x|10,20,object|2"` is what the
  same cell returns for a real constructor, and §4 keeps it as that cell's control.
- **P-4 — `is_constructor` is `#[cfg(feature = "engine")]`** (`vm/object_kind.rs:1672`) while `do_new`,
  `construct_synchronous` and the heritage call site are not gated. Inside the workspace build the gate
  is invisible because feature unification turns `engine` on; a standalone
  `cargo check -p elidex-js --lib` (default features) fails today with 16 errors, one of them E0425 for
  `is_constructor` at `vm/dispatch.rs:998`. The predicate is ECMAScript core and depends on nothing
  engine-side, so §1.3 removes the gate. The other 15 errors are not this slot's.

### §0.4 Open decision (not settled by the brief, not chosen here)

**OD-1 — is a WebIDL interface object without a constructor operation a constructor?** The VM models
`CallShape::IllegalConstructor` as having neither `[[Call]]` nor `[[Construct]]`
(`vm/value.rs:700-712`), so `is_constructor` returns false and `class X extends Crypto {}` throws at
class definition. WebIDL §3.7.1 creates every interface object with CreateBuiltinFunction and says
such an object "will throw when called, both as a function and as a constructor" (read with
`.claude/tools/webref body webidl interface-object`), which reads as IsConstructor **true** with a
`[[Construct]]` that throws. The two readings differ only at the predicate's consumers (heritage,
`customElements.define`, the TypedArray gates) and in the super-path message; `new Crypto()` is a
TypeError either way. **This memo keeps today's native classification unchanged** — the predicate's
native arm stays `nf.shape.can_construct()` — and records the question for the driver.
**Recommendation**: a separate slot owned by whoever owns `CallShape` (it reverses a documented design
decision in `vm/value.rs`, and its blast radius is the WebIDL surface, not the ES construct path).

## §1. Design

### §1.1 Construct entry points (census)

Found by `git grep -n 'CallMode::Construct {' 115a667c -- crates/script/elidex-js/src ':!crates/script/elidex-js/src/vm/tests'`
(5 hits: the four creation sites below plus the `NativeContext::new_construct` constructor at
`vm/value.rs:839`) and `git grep -n 'construct_synchronous(\|call_construct_native(\|\.do_new(' 115a667c -- crates/script/elidex-js/src ':!crates/script/elidex-js/src/vm/tests'`.

| Entry | Site | Checks IsConstructor today? | In scope? |
|---|---|---|---|
| `new F(...)` — `do_new` | `vm/ops.rs:681` (from `vm/dispatch.rs:730`) | partially: arrows and non-constructable natives only | **yes** — same defect |
| `super(...)` and the default derived constructor — `construct_synchronous` via `dispatch_super_inner` | `vm/dispatch_class.rs:139` → `:232` | no (JS function kind is not inspected) | **yes** — same defect |
| custom-element upgrade — `construct_synchronous` | `vm/host/custom_elements/upgrade.rs:315` | indirectly: the constructor was admitted at define time by `is_constructor` (`define.rs:49`) | covered by the `construct_synchronous` fix, no own change |
| native construct — `call_construct_native` | `vm/interpreter.rs:95`, sole caller `vm/ops.rs:808` | reached only after `do_new`'s native check | no own change (behind `do_new`) |
| `new X(...args)` — `Op::NewSpread` | `vm/dispatch.rs:879-883` | n/a: a stub that pushes `undefined` | **no** — `#11-vm-call-spread-arguments`; hand-off §5 |
| `Reflect.construct`, Proxy `construct` trap | — | n/a: neither exists (`git grep -c 'Proxy' 115a667c -- crates/script/elidex-js/src/vm/object_kind.rs crates/script/elidex-js/src/vm/value.rs` → no output) | no |
| species / `Array.from` / `Array.of` / Promise subclassing | — | `Array.from` ignores its receiver; TypedArray statics only *test* with `is_constructor` and never call a user constructor | no |

### §1.2 What the predicate must answer, against what it answers

Spec: IsConstructor (§7.2.4) is "has a `[[Construct]]` internal method". An ECMAScript function gets
one only from MakeConstructor (§10.2.5): InstantiateOrdinaryFunctionObject (§15.2.4) and
InstantiateOrdinaryFunctionExpression (§15.2.5) call it, ClassDefinitionEvaluation (§15.7.14) calls it
at step 17 for the class constructor; DefineMethod (§15.4.4) uses OrdinaryFunctionCreate + MakeMethod
and does not; the generator, async and async-generator instantiations (§15.5.3, §15.8.2, §15.6.3) and
arrows (§15.3.4) do not. A bound function has `[[Construct]]` iff its target does (BoundFunctionCreate
§10.4.1.3 step 6).

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

**The compile-time fact.** `CompiledFunction` gains `has_construct: bool` (NEW) — "MakeConstructor
applies to the function object created from this code". The compiler sets it at the sites where the
spec calls MakeConstructor, and nowhere else:

- function declarations and expressions that are neither async nor generator —
  `compile_nested_function` (`compiler/expr_function.rs:95`), whose unused `_is_declaration: bool`
  parameter is replaced by the creation path (a `FunctionCreation` (NEW) of `Ordinary` or `Method`),
  so every caller states which path it is on: `compiler/expr.rs:211`, `compiler/expr_function.rs:169`,
  `compiler/mod.rs:84` pass `Ordinary`; `compiler/expr_class.rs:305` and `:354` pass `Method`;
- class constructors — the written one (`compiler/expr_class.rs:121`, which already sets
  `is_class_ctor`) and the synthesized one (`synthesize_default_class_ctor`, `compiler/expr_class.rs:69`);
- **object-literal methods and accessors are the site that needs a new route**: their value is an
  ordinary `ExprKind::Function` compiled through `compile_expr` (`compiler/expr_object.rs:74-75` for
  `PropertyKind::Init`, `compile_accessor` at `:14-24` for `Get`/`Set`), so the creation path is lost
  by the time `compiler/expr.rs:211` sees it. `compile_object_expr` compiles a value whose property
  carries `PropertyFlags::method()` or a `Get`/`Set` kind through `compile_nested_function(…, Method)`
  directly. A `{ f: function(){} }` value stays `Ordinary` (it is a function expression; §4 controls it).

**The default is `false`**, so a creation site that nobody classified yields a loud TypeError on `new`
rather than a silent construct — the fail-safe direction for an admission predicate.

**The predicate.** One classifier over a non-bound object, in `vm/object_kind.rs` next to
`is_constructor`, returning a `ConstructAdmission` (NEW): `Construct`, `NotConstructor`,
`CallableOnlyNative(name)`, `IllegalConstructorNative(name)`. The JS arm reads `has_construct` and
nothing else (the `this_mode` / `is_async` / `is_generator` tests at `:1680-1687` are deleted, their
content now being part of the bit); the native arm is today's `CallShape` split (OD-1 leaves it as is);
every other kind is `NotConstructor`. `is_constructor` keeps its signature and its bound-chain walk
and becomes `walk + matches!(…, Construct)`; its `#[cfg(feature = "engine")]` is removed (P-4).

**The admission.** One helper in `vm/dispatch_class.rs`, beside `unwrap_bound_function_chain` (`:180`),
used by both entries in place of their own checks: unwrap the bound chain (keeping today's
`Maximum bind chain depth exceeded` RangeError at `:199-201`), classify the target, and map a refusal
to the error. `do_new` loses its arrow test (`vm/ops.rs:709-714`) and its native `match`
(`:716-752`) and calls the helper where it now calls `unwrap_bound_function_chain` (`:696`);
`construct_synchronous` replaces its unwrap (`:256`) and its kind test (`:267-281`) the same way. Checking the unwrapped target is IsConstructor
of the wrapper by BoundFunctionCreate step 6.

**Placement.** Both entries run after the arguments are evaluated and before any allocation, frame
push or stack splice: EvaluateNew (§13.3.5.1.1) step 5 follows step 4's ArgumentListEvaluation, and
SuperCall evaluation (§13.3.7.1) step 5 follows step 4. `do_new` runs with the arguments already on
the stack; `construct_synchronous` receives them as a slice. §4 T-ORD pins both.

**Frame balance.** After admission no callee whose call is frameless can reach the JS branch of
`construct_synchronous`, so the over-pop is removed at its cause, not guarded. The JS branch states
the invariant it now relies on as a `debug_assert!` that the frame count grew by one before `vm.run()`
(`vm/dispatch_class.rs:311-319`) — a guard against a later frameless kind, not a second check.

**Error wording** is the crate's existing set, now produced in one place: a JS non-constructor or
non-callable kind → `"not a constructor"` (`vm/ops.rs:686`, `:712`, `:750`;
`vm/dispatch_class.rs:280`); `CallShape::CallableOnly` → `"{name} is not a constructor"`
(`vm/ops.rs:744-747`, `vm/dispatch_class.rs:278`); `CallShape::IllegalConstructor` →
`VmError::illegal_constructor` (`vm/ops.rs:737-741`). The super path gains the last arm, which it
lacks today (`vm/dispatch_class.rs:265-281` reports an `IllegalConstructor` native as
`"{name} is not a constructor"`): one mapping, both entries. Unchanged: the heritage message
`"Class extends value is not a constructor or null"` (`vm/dispatch.rs:1001`) and the null-super
message (`vm/dispatch_class.rs:124`).

**Docstring.** `construct_synchronous`'s claim that it is invoked from "`do_new`'s JS branch"
(`vm/dispatch_class.rs:220-221`) is false — `do_new` pushes its frame inline (`vm/ops.rs:784-793`) and
the second caller is the custom-element upgrade. The PR touches the function and corrects it.

**Layering / ECS.** All of this is ECMAScript core in `vm/` and `compiler/`; nothing moves into
`vm/host/`, whose two consumers only call the predicate. No component, side store or session state is
involved.

## §2. Edge density, by measurement

Criterion (slice-plan §5's head): an axis is evidenced only by a failure that occurs while the others
pass; count the axes that intersect. The base tree is itself the measurement, because its three
columns apply the predicate to different degrees — `extends` applies the full predicate, `new` applies
an arrow-only fragment of it, `super` applies none.

- **A1, classification** (is the callee kind classified per spec). Evidence: in the `extends` column
  the generator / async / arrow rows pass while the method / getter / setter rows fail (§1.2). So
  classification fails independently of placement.
- **A2, placement** (does every entry consult the predicate). Evidence: the generator row passes in
  `extends` and fails in `new` and `super`, where the predicate is correct and not consulted. So
  placement fails independently of classification.
- **Frame balance is not a third axis.** Every over-pop row is a row that fails A2 in the `super`
  column; no measured row over-pops while its TypeError expectation holds. In mutant terms: the only
  ways to reintroduce the over-pop are to drop the `construct_synchronous` admission (an A2 mutant) or to
  classify a frameless kind as a constructor (an A1 mutant), and both are killed first by the TypeError
  cells. T-POP is kept because it is the release-observable form of the corruption, not because it
  measures an axis.
- **Error mapping** shares A2's site (the admission helper) and fails with it.

**Two intersecting axes (< 3).** The second disjunct is disproved by citation: IsConstructor and
MakeConstructor are canonical algorithms (§3). **Single PR.** The post-implementation mutant run
(§4.2) re-measures this; if it shows a mutant surviving every cell, the slot comes back for re-slicing.

## §3. Spec coverage map

Section numbers from `.claude/tools/webref aoid ecma262 <AO>`; step text from
`.claude/tools/webref body ecma262 <AO-or-anchor>`. SuperCall's evaluation is a syntax-directed
operation without an aoid; its number and steps come from
`.claude/tools/webref body ecma262 sec-super-keyword-runtime-semantics-evaluation` (heading "13.3.7.1
Runtime Semantics: Evaluation"). Skeleton from `.claude/tools/webref coverage-map`; §6 has the command.

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| ECMA-262 §7.2.4 IsConstructor ( arg ) | 1-3 | non-Object → false; has `[[Construct]]` → true; else false | `vm/object_kind.rs:1673` (`is_constructor`) + the classifier (NEW) | ✓ every `ObjectKind` reaches one arm; JS arm reads `has_construct` | yes — any script value |
| ECMA-262 §13.3.5.1.1 EvaluateNew ( constructExpr, argumentsNode ) | 5 | IsConstructor false → TypeError, after step 4's argument evaluation | `vm/ops.rs:681` (`do_new`) | ✓ | yes |
| ECMA-262 §13.3.7.1 Runtime Semantics: Evaluation | SuperCall 5 | IsConstructor(superCtor) false → TypeError, after step 4 | `vm/dispatch_class.rs:139` → `:232` | ✓ | yes — `Object.setPrototypeOf` on the class |
| ECMA-262 §15.7.14 Runtime Semantics: ClassDefinitionEvaluation | 14.a.iv.3 | default derived ctor: IsConstructor(func) false → TypeError | synthesized ctor → `Op::SuperCallSpread` → `vm/dispatch_class.rs:232` | ✓ | yes |
| ECMA-262 §15.7.14 Runtime Semantics: ClassDefinitionEvaluation | 8.g | heritage not a constructor → TypeError (consumer; changes through the predicate only) | `vm/dispatch.rs:998` | ✓ | yes |
| ECMA-262 §15.7.14 Runtime Semantics: ClassDefinitionEvaluation | 17 | MakeConstructor on the class constructor | `compiler/expr_class.rs:121`, `:69` | ✓ | no |
| ECMA-262 §10.2.5 MakeConstructor ( func [ , writableProto [ , proto ] ] ) | 1.c | sets `[[Construct]]` on an ECMAScript function | `has_construct` (NEW) in `bytecode/compiled.rs` | ✓ | no |
| ECMA-262 §15.2.4 Runtime Semantics: InstantiateOrdinaryFunctionObject | MakeConstructor step | declarations are constructors | `compiler/expr_function.rs:169`, `compiler/mod.rs:84` | ✓ | no |
| ECMA-262 §15.2.5 Runtime Semantics: InstantiateOrdinaryFunctionExpression | MakeConstructor step | expressions are constructors | `compiler/expr.rs:211` | ✓ | no |
| ECMA-262 §15.4.4 Runtime Semantics: DefineMethod | 6-7 | OrdinaryFunctionCreate + MakeMethod, no MakeConstructor | `compiler/expr_object.rs:46`, `compiler/expr_class.rs:305`, `:354` | ✓ | no |
| ECMA-262 §10.4.1.3 BoundFunctionCreate ( targetFunc, boundThis, boundArgs ) | 6 | `[[Construct]]` iff target has one | `vm/dispatch_class.rs:180` (unwrap) + classifier on the target | ✓ | yes |
| ECMA-262 §7.3.14 Construct ( ctor [ , argList [ , newTarget ] ] ) | 3 | caller guarantees ctor is a constructor | both entries after admission | ✓ | no |

### §3.1 User-input audit

The callee of `new`, the `[[Prototype]]` of a derived class (writable by `Object.setPrototypeOf` at any
time) and the heritage value are all script-controlled. The over-pop makes the super path the one
with a control-flow consequence: a page can make an enclosing function return early with an
attacker-chosen object, and the frames resynchronize only at the eval boundary. After this PR every
script-controlled value reaches the classifier before any frame or allocation effect, and the
classifier's unknown-kind arm is `NotConstructor`.

## §4. Tests

### §4.1 Cells

New file `crates/script/elidex-js/src/vm/tests/tests_construct_isconstructor.rs` (NEW), registered in
`vm/tests/mod.rs` (201 lines). No existing file is grown: `tests_class_machinery.rs` (389) would fit
but is scoped to class lowering, and `tests_custom_elements.rs` is 1320 lines (over the 1000-line
touch-time rule), which is why no `customElements.define` cell is added (§5). Callee kinds, one table
shared by the cells: generator, async, async generator, async arrow, arrow, object-literal method,
generator method, async method, class method, static method, object getter, object setter, class
getter, bound generator, bound method; controls: function declaration, function expression, a
`{ f: function(){} }` property value, class, bound function, bound class, `Object`, `Error`.

| Cell | Asserts | Base (`115a667c`) | Mutant it kills |
|---|---|---|---|
| T-NEW | `new X()` → TypeError, message `not a constructor`, per non-constructor kind (arrow and async arrow already pass) | red for the other 13 | M1 (no admission in `do_new`), M3, M4, M5 |
| T-SUPER-DEF | `class B extends A {}` + `Object.setPrototypeOf(B, X)` → `new B()` TypeError, per kind | red for all 15 (debug: panic for the 7 frameless kinds) | M2, M3, M4, M5 |
| T-SUPER-EXP | same with `constructor(){ super(); }` | red for all 15 | M2 |
| T-POP | `function inner(){ let a=10,b=20; let r; try { r = new B() } catch (e) { r = e instanceof TypeError ? 'TE' : 'other' } return [a,b,r].join(',') }` called from `outer` → `"x|10,20,TE|2"`, per frameless kind | release `"x|[object Object]|2"`, debug panic at `vm/interpreter.rs:210` | M2, M5 (frameless kinds) |
| T-POP-CTL | the same cell with X = each control → `"x|10,20,object|2"` | green (stays green) | M6 |
| T-EXTENDS | `class B extends X {}` → TypeError for the method / getter / setter kinds | red for 6 | M3, M4 |
| T-CTL | every control constructs through `new` and through both super forms; `extends` accepts function, class, `Object`, `Error` | green | M6, M7 |
| T-ORD | `new X(log.push(1))` and `super(log.push(1))` with a generator X → TypeError and `log.length === 1` | red on the TypeError half at both (`log.length` is already 1); debug panics for `super` | M8 |
| T-MSG | `Object.setPrototypeOf(B, Crypto)` → `new B()` message equals `new Crypto()`'s (`Illegal constructor`); `Object.setPrototypeOf(B, Math.max)` → `max is not a constructor` | red for the first — not probed; at base the super path builds `{name} is not a constructor` (`vm/dispatch_class.rs:276-278`); the second is green (probed) | M9 |
| T-DEPTH | a bound chain one past `MAX_BIND_CHAIN_DEPTH` (`vm/mod.rs:105`, 10 000) → RangeError at both entries | green by reading `vm/dispatch_class.rs:199-201` (not probed) | M10 |

### §4.2 Mutants

Run on the final head only, sequentially (≤ 2 parallel builds), each recorded as killed-by-cell:

- **M1** remove the admission from `do_new`. **M2** remove it from `construct_synchronous`.
- **M3** object-literal methods and accessors stay on the `compile_expr` route, so they are compiled
  as `Ordinary` and get the bit. **M4** class methods and accessors pass `Ordinary`.
- **M5** `has_construct` ignores `is_async` / `is_generator`.
- **M6** default `has_construct` stays `false` for the synthesized class constructor (kills the
  class-without-constructor control and every `class A {}` super cell).
- **M7** the classifier is applied to the bound wrapper instead of the unwrapped target (bound
  controls fail).
- **M8** the admission moved after `push_js_call_frame` in `construct_synchronous`.
- **M9** the super path keeps its own native message.
- **M10** the admission walks the chain with `is_constructor` before unwrapping (over-depth becomes a
  TypeError).

A mutant no cell kills is a missing cell, added before the PR leaves review.

## §5. Non-goals, hand-offs, sibling defects

- **SD-1 — own `prototype` on methods and async functions.** Probe P1:
  `Object.prototype.hasOwnProperty.call(({m(){}}).m, 'prototype')` and the same for
  `async function(){}` both return `true`; per §10.2.5 the property is installed by MakeConstructor,
  which neither path calls. `create_closure` keys the install on `!is_arrow` (`vm/ops.rs:897`).
  `has_construct` is the natural key for that fix, but it changes observable properties, not
  construct admission, and the brief bounds this slot to the two entry points. **No slot exists**
  (`grep -c 'MakeConstructor'` over the live ledger → 0 on 2026-11-03, and `git grep -c MakeConstructor 115a667c -- docs/plans` → no output; this memo is now a hit, so re-run against `115a667c`, not HEAD) — a slot is owed and
  the PM mints it.
- **OD-1** (§0.4) — the native `IllegalConstructor` classification.
- **`Op::NewSpread`** — when `#11-vm-call-spread-arguments` replaces the stub, `new X(...args)` must
  route through the same admission helper; this PR lands first, so that slice consumes the helper
  rather than writing a third check.
- **`customElements.define` with a method** becomes a TypeError through the predicate; no cell here,
  because its test file is over 1000 lines and the define gate is unchanged code.
- **`vm/object_kind.rs` is 1700 lines.** This PR edits `is_constructor`'s body and adds the classifier
  beside it. Whether the file has a real cohesion seam (the touch-time rule) is a question for the
  driver before implementation; if it does, the split is a standalone prereq PR.
- Slice S keeps D-S1; this branch edits neither S's row nor the 1aa memo.

## §6. Re-derivation

Run from the repository root at `115a667c`.

- Probe P1 (logs and source in the session scratchpad `isctor/`: `probe-debug.log`,
  `probe-release.log`, `tests_zz_probe.rs`): a throwaway `git worktree add --detach … 115a667c`,
  `mod tests_zz_probe;` appended to `crates/script/elidex-js/src/vm/tests/mod.rs`,
  `env -u RUSTUP_TOOLCHAIN cargo test [--release] -p elidex-js --lib --all-features tests_zz_probe -- --nocapture --test-threads=1`
  (`rustc 1.99.1`). Each cell runs in a fresh `Vm` under `catch_unwind`, so a debug panic reports as
  `PANIC` without masking the following cells.
- `vm/ops.rs:681`, `:685-687`, `:696`, `:709-714`, `:716-752`, `:784-793`, `:808`, `:897`:
  `sed -n <range>p crates/script/elidex-js/src/vm/ops.rs`; anchor `git grep -n 'fn do_new' -- crates/script/elidex-js`.
- `vm/dispatch_class.rs:124`, `:139`, `:180`, `:199-201`, `:220-221`, `:232`, `:256`, `:267-281`, `:311-319`:
  `sed -n` on that file; anchors `git grep -n 'fn construct_synchronous\|fn unwrap_bound_function_chain\|fn dispatch_super_inner' -- crates/script/elidex-js`.
- `vm/object_kind.rs:1672-1687`: `git grep -n 'fn is_constructor' -- crates/script/elidex-js`.
- `vm/dispatch.rs:728-730`, `:879-883`, `:998`, `:1001`: `git grep -n 'Op::New =>\|Op::CallSpread | Op::NewSpread\|is_constructor(self' -- crates/script/elidex-js/src/vm/dispatch.rs`.
- `vm/interpreter.rs:95`, `:208-216`, `:866-906`, `:912`: `git grep -n 'fn call_construct_native\|Ok return leaked\|if is_async {\|if is_generator {' -- crates/script/elidex-js/src/vm/interpreter.rs`.
- `vm/value.rs:588-599`, `:700-712`, `:839`: `git grep -n 'pub struct FunctionObject\|CallShape::IllegalConstructor\`\] — neither\|mode: CallMode::Construct { new_target }' -- crates/script/elidex-js/src/vm/value.rs`.
- `vm/host/custom_elements/define.rs:49`, `upgrade.rs:315`, `vm/host/typed_array_static.rs:146`, `:287`:
  `git grep -n 'is_constructor(\|construct_synchronous(' -- crates/script/elidex-js/src/vm/host`.
- `bytecode/compiled.rs:42-81`: `git grep -n 'pub struct CompiledFunction' -- crates/script/elidex-js`.
- `compiler/expr_function.rs:95`, `:169`, `compiler/expr.rs:211`, `compiler/mod.rs:84`,
  `compiler/expr_class.rs:69`, `:121`, `:305`, `:354`: `git grep -n 'compile_nested_function(\|fn synthesize_default_class_ctor' -- crates/script/elidex-js/src/compiler`.
- `compiler/expr_object.rs:14-24`, `:46`, `:74-75`: `git grep -n 'fn compile_accessor\|fn compile_object_expr' -- crates/script/elidex-js/src/compiler/expr_object.rs`.
- File sizes: `wc -l crates/script/elidex-js/src/vm/object_kind.rs crates/script/elidex-js/src/vm/tests/tests_custom_elements.rs crates/script/elidex-js/src/vm/tests/tests_class_machinery.rs crates/script/elidex-js/src/vm/tests/mod.rs` → 1700, 1320, 389, 201.
- P-4: `env -u RUSTUP_TOOLCHAIN cargo check -p elidex-js --lib` → `could not compile … due to 16 previous errors`, including E0425 at `vm/dispatch.rs:998`.
- §3: `.claude/tools/webref coverage-map ecma262 IsConstructor ecma262 EvaluateNew ecma262 ClassDefinitionEvaluation ecma262 Construct ecma262 MakeConstructor ecma262 OrdinaryFunctionCreate ecma262 DefineMethod ecma262 MakeMethod ecma262 GetSuperConstructor`
  (split decision "ok → single PR scope"); `aoid ecma262` for InstantiateOrdinaryFunctionObject,
  InstantiateOrdinaryFunctionExpression, InstantiateGeneratorFunctionObject,
  InstantiateAsyncFunctionObject, InstantiateAsyncGeneratorFunctionObject,
  InstantiateArrowFunctionExpression, BoundFunctionCreate.
