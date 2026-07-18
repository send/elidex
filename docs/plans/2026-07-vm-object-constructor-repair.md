# VM `Object` constructor repair — re-diagnosis of `#11-vm-native-fn-generic-invocation`

**Status**: plan-memo, pre-implementation. Branch `vm-object-constructor` (worktree `elidex-wt-object-ctor`), base `origin/main` `3e252148` (#467).
**Lane**: core VM (post-#467 Date-builtin cluster). This memo ships with the impl PR.

---

## 0. TL;DR

The `#467` converge carved `#11-vm-native-fn-generic-invocation` with the premise
*"generic-invoking a native fn throws for ALL receivers — a native-fn CALL DISPATCH
bug whose blast-radius = all native fns × all receivers, edge-dense, plan-review
required."* **That premise is wrong.** Probe-testing the live VM (evidence in §1)
shows the real defect is narrow and unrelated to call dispatch:

> **Global `Object` is registered as a *namespace object* (like `Math`), not a
> constructor.** It is non-callable, and its `prototype` forward-link and
> `Object.prototype.constructor` back-link are absent. Every sibling constructor
> (`Array`/`String`/`Number`/`Boolean`/`Date`/`Error`) is a proper constructor
> wired via the existing `wire_constructor_global` helper. `Object` alone skips it.

The fix makes `register_object_global` produce a proper §20.1.1 `Object`
constructor through the **same helper the six siblings already use**. It is not
edge-dense (§8). It closes `#11-vm-native-fn-generic-invocation` and — by making
`Object.prototype.toString.call(x)` newly JS-observable — must ride with the
`#11-object-prototype-tostring-builtin-tag` wrapper arms to avoid shipping a
newly-observable spec violation (§3).

---

## 1. Re-diagnosis: evidence (live VM probe, `Vm::eval`, engine feature)

| spelling | actual | spec-correct |
|---|---|---|
| `typeof Object` | **`"object"`** | `"function"` |
| `new Object()` | **throw `not a constructor`** | fresh object |
| `Object(5)` / `Object()` | **throw `not a function`** | Number wrapper / fresh object |
| `Object.prototype` | **`undefined`** | %Object.prototype% |
| `Object.prototype === undefined` | **`true`** | `false` |
| `Object.prototype.constructor` | **`undefined`** | `=== Object` |
| `Object.name` / `Object.length` | **`undefined`** | `"Object"` / `1` |
| `Object.getPrototypeOf({}).constructor` | **`undefined`** | `=== Object` |
| `Object.keys/values/assign/...` (statics) | **all work** | ✓ |
| `({}).toString()` | **works** | ✓ |
| `Array.prototype.slice.call([1,2,3])` | **works** | ✓ |
| `Array.prototype.hasOwnProperty` (chain→%Object.prototype%) | **works** | ✓ |
| `Object.prototype.toString.call(new Date())` | **throw `Cannot convert undefined or null to object`** | `"[object Date]"` |
| `Array/String/Number/Boolean/Date.prototype.constructor === Ctor` | **all `true`** | ✓ |

**Why the slot's error text misled the diagnosis**: `Object.prototype` evaluates
to `undefined`; `typeof undefined-slot` is masked because the *member* get
`Object.prototype.toString` then runs `ToObject(undefined)` → the §7.1.19 error
*"Cannot convert undefined or null to object"* (`coerce.rs:713/728`; §7.1.19 =
ToObject, webref-verified — §7.1.18 = ToString). The throw is
**in the property-get of a `undefined` base**, not in `Object.prototype.toString`
(which handles every `this` variant incl. undefined/null, `natives_symbol.rs:246`)
and not in `Function.prototype.call` (which forwards `this_arg` correctly,
`natives_function.rs:52-54`). The interpreter's inherited-method fast path
(`({}).toString()`) reaches the *real* `%Object.prototype%` through `{}`'s
`[[Prototype]]`, which is why ordinary usage works and only the global
`Object.prototype.*` access is broken.

**Root site** — `globals.rs:1366` `register_object_global`:

```rust
fn register_object_global(&mut self) {
    let obj_id = self.create_object_with_methods(&[ /* keys, values, ... */ ]); // ObjectKind::Ordinary — NON-callable
    let name = self.strings.intern("Object");
    self.globals.insert(name, JsValue::Object(obj_id));   // no .prototype, no .constructor back-link
}
```

`create_object_with_methods` (`globals.rs:1024`) allocates `ObjectKind::Ordinary`
with `[[Prototype]] = %Object.prototype%` and installs methods — the correct tool
for `Math`/`JSON` namespaces and for building *prototype* objects, but **not** for
a constructor. Contrast `register_array_global` (`globals.rs:1398`) and the
`String`/`Number`/`Boolean` registrations (`globals_primitives.rs:89/128/191`),
which build a **constructable** function and call
`wire_constructor_global(name, ctor, proto)` (`globals.rs:1044`) to install
`Ctor.prototype = proto` (BUILTIN), `proto.constructor = Ctor` (METHOD), and the
global. `%Object.prototype%` itself already exists (`register_prototypes`,
`globals.rs:1179`, called at `:157` — **before** `register_object_global` at
`:163`, so the ordering for wiring is satisfied).

---

## 2. Fix (sibling-uniform)

Rewrite `register_object_global` to mirror the six siblings:

1. **`native_object_constructor`** (new, `natives_object/…`) implementing
   ECMA-262 **§20.1.1.1 `Object ( value )`** (webref-verified):
   1. If NewTarget is neither `undefined` nor the active function object (i.e. a
      **subclass** `new`): return the `do_new`-provided instance (elidex resolves
      the subclass prototype in `do_new` and passes it as `this`, exactly as
      `Date`/`Number` do — those ctors never resolve the prototype themselves,
      `natives_date/mod.rs:138`).
   2. If `value` is `undefined`/`null`: return a fresh ordinary object with
      `%Object.prototype%` (construct form: `do_new`'s `this`; call form: a fresh
      alloc).
   3. Else: return `ToObject(value)` (`super::coerce::to_object`, already boxes
      primitives — precedent `natives_symbol.rs:318`).
   The construct/call split is a single `is_construct()` `if`/`else`, per the
   native-ctor-guard discipline (`natives_date/mod.rs:129-132`).
2. Build it via `create_constructable_function("Object", native_object_constructor)`.
3. Install the existing static-method list on the ctor.
4. `self.wire_constructor_global("Object", ctor_id, self.object_prototype.expect(...))`
   — installs `Object.prototype` (BUILTIN), `Object.prototype.constructor`
   (METHOD), and the `Object` global.
5. `Object.name = "Object"` is free (`create_native_function_keyed` installs `name`
   per §20.2.4.2). `Object.length = 1` is **intentionally NOT installed**:
   `create_native_function_keyed` installs `name` only, so **no** built-in ctor in
   elidex has a `.length` (verified at impl) — adding it only to `Object` would be a
   strangler (One-issue-one-way). The uniform ctor-`.length` gap is recorded as a
   finding (§7); the §6 test asserts `Object.name` but not `.length`.

**§1-step-1-vs-3 disambiguation** (spec-axis): distinguishing `new Object(5)`
(NewTarget === %Object% → step 3 `ToObject` → Number wrapper) from `new Subclass(5)`
(NewTarget = Subclass → step 1 → the subclass instance, value ignored) requires the
native ctor to compare `ctx.new_target()` against the `%Object%` intrinsic. **This
plan commits to the ideal**: add an `object_constructor: Option<ObjectId>` intrinsic
slot to `VmInner` — directly precedented by the existing `html_element_constructor`
slot (`vm/mod.rs:279`), sibling to `object_prototype`/`array_prototype` — set at
`register_object_global`, and compare `ctx.new_target()` (already available,
`native_context.rs:152`) against it. No approximation, no divergence test: `new
Object(5)` returns the spec-correct Number wrapper, `new Subclass(5)` the subclass
instance. (The earlier draft weighed a `this.[[Prototype]] === %Object.prototype%`
heuristic; **rejected** — plan-review Axis 3 F1 — it misclassifies
`Reflect.construct(Object,[v],newTarget)` when `newTarget.prototype === %Object.prototype%`,
and the ideal costs only one well-precedented field, so per *ideal over pragmatic* a
trivial field is not a design constraint that licenses shipping a spec divergence.)

---

## 3. Cluster split (this PR is **separable**, not an umbrella)

The #467 converge grouped four "VM-core coercion/dispatch" slots. They share only
a common *origin* (surfaced by the same converge), **not a design axis**. Splitting:

- **PR 1 = THIS**: Object-constructor repair **+ completing the §20.1.3.6
  builtin-tag match** (`native_object_prototype_to_string`, `natives_symbol.rs:273`):
  the three primitive-wrapper arms (`[[BooleanData]]`/`[[NumberData]]`/`[[StringData]]`,
  steps 9-11) **and** the `[[ParameterMap]]` Arguments arm (step 6). Closes
  **`#11-vm-native-fn-generic-invocation`** AND **`#11-object-prototype-tostring-builtin-tag`**.
  - *Why the builtin-tag arms ride along*: the Object-ctor repair makes
    `Object.prototype.toString.call(x)` JS-observable through the global-`Object`
    spelling; closing the slot means bringing the **whole** §20.1.3.6 match to spec —
    fixing only some arms = strangler (One-issue-one-way). Current gaps, all falling
    to the step-14 `"Object"` default: `.call(new Number(5))` → `"[object Object]"`
    (spec `"[object Number]"`; + String, Boolean), and `.call(arguments)` →
    `"[object Object]"` (spec `"[object Arguments]"`).
  - *Arguments included* (plan-review Axis 3/4 F2, was excluded on a code+spec-
    contradicted hedge): `ObjectKind::Arguments` exists (`object_kind.rs`), and though
    elidex is strict-only, webref confirms the unmapped (strict) arguments object
    still carries a `[[ParameterMap]]` internal slot (§10.4.4.6 steps 2-3:
    `OrdinaryObjectCreate(%Object.prototype%, « [[ParameterMap]] »)` then set to
    undefined), so §20.1.3.6 step 6 tags it "Arguments". Four match arms total
    (3 wrappers + Arguments); including them makes `.call(x)` correct for *all* x
    ([[feedback_defer-accumulation-signals-mis-drawn-slice]] — don't ship a
    partially-closed match).
- **PR 2 (separate)**: `#11-vm-symbol-operand-coercion-throws` — opcode-level
  coercion dispatch (`` `${Symbol()}` `` etc. return a string instead of
  TypeError). Different subsystem (operator opcodes, not globals). Unblocked for a
  JS-observable test by PR 1 but fixed independently.
- **PR 3 (separate, plan-review REQUIRED)**: `#11-vm-wrapper-coercion-override-bypass`
  — `coerce.rs` `to_number`/`to_string` `NumberWrapper` arms + `natives_json`
  read internal slots, bypassing overridden `valueOf`/`toString`. The slot itself
  mandates plan-review (4 axes: ToPrimitive/ToNumber/ToString/JSON-serialize +
  hot-path perf). Different subsystem (coerce wrappers). Unrelated to Object global.

Bundling all four = a false umbrella that mixes a trivial globals fix with an
edge-dense coerce refactor (violates One-issue-one-way + review-cost-tracks-blast-radius).

---

## 4. Scope boundaries

- **IN**: `Object` callability (§20.1.1.1), `Object.prototype` fwd + `.constructor`
  back-link, `Object.name`/`.length`, §20.1.3.6 wrapper builtin-tag arms.
- **OUT — recorded, not fixed here**:
  - **Global `Function` is absent** (`typeof Function` → `"undefined"`; probe). Same
    *class* of gap (a constructor not wired to a global) but `Function`'s constructor
    is §20.2.1 The Function Constructor — dynamic creation via §20.2.1.1.1
    CreateDynamicFunction (`new Function(src)` = runtime codegen), entangled with the
    core-strict-only stance ([[reference_elidex-js-core-strict-only]]).
    Distinct, larger decision → record as a finding / candidate slot, do **not**
    scope into this PR.
  - `#11-vm-symbol-operand-coercion-throws`, `#11-vm-wrapper-coercion-override-bypass`
    → PR 2 / PR 3 (§3).

---

## §5. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| ECMA-262 §20.1.1.1 Object ( value ) | step 1 | NewTarget = subclass (≠ %Object%) | `native_object_constructor` (NEW) → do_new subclass instance (`this`) | ✓ | yes |
| ECMA-262 §20.1.1.1 Object ( value ) | step 2 | value undefined/null | `native_object_constructor` (NEW) → OrdinaryObjectCreate(%Object.prototype%) | ✓ | yes |
| ECMA-262 §20.1.1.1 Object ( value ) | step 3 | value non-nullish, NewTarget = %Object% | `native_object_constructor` (NEW) → `coerce::to_object(value)` | ✓ | yes |
| ECMA-262 §20.1.3.6 Object.prototype.toString ( ) | step 6 | `[[ParameterMap]]` (Arguments, incl. strict/unmapped) | `native_object_prototype_to_string` NEW arm → "Arguments" | ✓ | yes |
| ECMA-262 §20.1.3.6 Object.prototype.toString ( ) | step 9 | `[[BooleanData]]` wrapper | `native_object_prototype_to_string` NEW arm → "Boolean" | ✓ | yes |
| ECMA-262 §20.1.3.6 Object.prototype.toString ( ) | step 10 | `[[NumberData]]` wrapper | `native_object_prototype_to_string` NEW arm → "Number" | ✓ | yes |
| ECMA-262 §20.1.3.6 Object.prototype.toString ( ) | step 11 | `[[StringData]]` wrapper | `native_object_prototype_to_string` NEW arm → "String" | ✓ | yes |
| ECMA-262 §20.1.3.6 Object.prototype.toString ( ) | steps 5/7-8/12-14 | Array/Function/Error/Date/RegExp/default | existing arms — unchanged | ✓ | no |

**Breadth**: K=1 spec (ecma262), M=8 entries → single-PR scope. (§10.4.4.6 is the
`[[ParameterMap]]`-slot basis for the step-6 arm; cited as an anchor, not a touch row.)

**Non-touched adjacent (§20.1.3.6)**: the `@@toStringTag` override — **steps 15-16**
(step 15 `Get(obj, %Symbol.toStringTag%)`, step 16 fall back to builtinTag if the
tag is not a String) — is **not** in this PR: elidex's builtin-tag arm precedes it
and no wrapper/Arguments kind sets `@@toStringTag`. The existing
`ObjectKind::Promise → "Promise"` arm is retained (behavior-preserving; Promise's
`"[object Promise]"` derives from `@@toStringTag` per spec, the arm is a compatible
shortcut — not touched here). *(Step 6 Arguments is now IN this PR — see the table
row above; the earlier "out of scope" framing is removed.)*

### §5.1 User-input touch audit

- `native_object_constructor` `value` arg — user-controllable (`Object(x)` /
  `new Object(x)`). Flows to `coerce::to_object` (step 3), which handles every
  non-nullish `JsValue` (primitives box; step 2 pre-empts nullish so step 3 never
  reaches the ToObject throw path). Full enum ✓.
- `native_object_prototype_to_string` `this` receiver — user-controllable
  (`Object.prototype.toString.call(x)`, now reachable generically post-repair).
  Every primitive `this` variant already handled (`natives_symbol.rs:246-291`); the
  NEW arms extend only the object-kind match (Boolean/Number/String wrappers +
  Arguments), not the primitive-`this` match. Full enum ✓.
- Adjacent pre-existing, reused verbatim, no new exposure: `wire_constructor_global`
  (`globals.rs:1044`), `create_constructable_function` (`shape_ops.rs:278`),
  `coerce::to_object` (`coerce.rs`). Exposure delta: only the intended `Object`
  callability + generic `Object.prototype.toString`.

**Spec anchors** (webref-verified `ecma262`; re-verify §-number/title at impl —
AO names are more stable than section numbers): §20.1.1.1 `Object ( value )` (3
steps) · §7.1.19 ToObject · §20.1.3.6 `Object.prototype.toString` (builtinTag step 6
Arguments + steps 9-11 wrappers; step 12 Date already #467 R1) · §10.4.4.6
CreateUnmappedArgumentsObject (strict arguments `[[ParameterMap]]` slot) · §20.1.2
statics unchanged (already installed).

---

## 6. Test plan (`tests_*` under `elidex-js/vm/tests`, engine feature)

JS-observable assertions the slot explicitly deferred (`tests_date_api.rs:366`):

- `Object.prototype.toString.call(new Date()) === "[object Date]"`.
- `Object.prototype.toString.call(new Number(5)) === "[object Number]"` (+ String,
  Boolean) — the newly-added wrapper arms.
- `Object.prototype.toString.call((function(){return arguments;})()) === "[object Arguments]"`
  (step-6 arm; strict-only VM but the `[[ParameterMap]]` slot is present, §10.4.4.6).
- `Object.prototype.toString.call([]) === "[object Array]"` (regression guard for
  the object-base case now reachable generically).
- Callability: `typeof Object === "function"`; `new Object() instanceof Object`;
  `typeof Object(5) === "object"`; `Object()`/`Object(null)` → fresh object;
  `new Object(5)` → Number wrapper (`(new Object(5)) instanceof Number`; Option A,
  §20.1.1.1 step 3, NewTarget = %Object%). Subclass-`new` step-1 tested:
  `class X extends Object {}; new X(5)` returns the X instance ignoring the arg
  (native-ctor subclassing works; the Option-A id compare distinguishes it from
  `new Object(5)`).
- Links: `Object.prototype.constructor === Object`;
  `Object.getPrototypeOf({}).constructor === Object`;
  `Object.prototype === Object.getPrototypeOf({})`; `Object.name === "Object"`
  (`.length` intentionally omitted — uniform ctor gap, §7).
- **Regression sweep**: `cargo test -p elidex-js --all-features` — confirm no
  existing test encodes the *broken* `Object()`/`Object.prototype===undefined`
  behavior. (Object is a foundational global: run the full crate suite, not a
  subset, before push.)

---

## 7. Deferred-slot reconciliation (at landing, PM)

- **CLOSE** `#11-vm-native-fn-generic-invocation` — re-diagnosed; the "call
  dispatch" framing is retired, real fix = Object-ctor wiring. Record the
  re-diagnosis so the slot's misleading premise doesn't resurface.
- **CLOSE** `#11-object-prototype-tostring-builtin-tag` — wrapper arms shipped with
  PR 1 (rides along, §3).
- Note in `[[project_open-defer-slots]]` #467-cluster that PR 1 unblocks the
  JS-observable tests for the remaining two slots (PR 2 / PR 3).
- New finding: `Function` global absence (§4) → candidate slot, per-PR-cap audit
  (add Why / re-eval trigger / re-eval date at registration per the #467-lane
  slot-metadata discipline).
- **MEMORY.md Active-state reconcile** (plan-review Axis 5 F9): the live core-VM-lane
  line (MEMORY.md:53) still frames the cluster as "edge-dense → plan-review 要"; at
  landing, drop the "edge-dense" adjective for this keystone (the re-diagnosis
  overturns it, §8) — "keystone" (it unblocks PR2/PR3 tests) and "plan-review ran"
  survive.
- **New finding — Error-object builtinTag gap** (surfaced at impl): a
  user-constructed `new Error()` is an ordinary object on `Error.prototype`, not
  `ObjectKind::Error` (only the throw path at `ops.rs:202` produces that kind), so
  `Object.prototype.toString.call(new Error())` yields `"[object Object]"`, not
  `"[object Error]"` (§20.1.3.6 step 8). The builtin-tag *arm* is correct; the gap
  is the Error-object dual representation — out of this slice's scope (the slot
  residual was the wrapper arms, steps 9-11; the test pins the current
  `"[object Object]"` so the gap is visible). → candidate slot (Why: Error dual
  representation; trigger: an Error-object unification OR a WPT observing
  `toString.call(error)`; re-eval: 2026-10-31).
- **New finding — uniform ctor `.length` gap**: `create_native_function_keyed`
  installs `name` but not `length`, so no built-in constructor exposes `.length`
  (§20.2.4.1 requires it, e.g. `Object.length === 1`). Uniform across all ctors →
  a per-ctor `.length` pass, not an Object-only fix (installing only on `Object`
  would be a strangler). → candidate slot (Why: uniform gap; trigger: a
  `.length`-observing WPT OR a ctor-metadata pass; re-eval: 2026-10-31).

---

## 8. Layering / ECS / edge-dense self-assessment

- **Layering**: `globals.rs`/`natives_object` = VM-core ES-intrinsic setup,
  engine-bound (prototype install + brand marshalling). No engine-independent
  algorithm belongs elsewhere; no `EcsDom` touch. Compliant.
- **ECS-native**: N/A — pure VM globals, no per-entity state.
- **Edge-dense?** No. The fix routes `Object` through the *existing*
  `wire_constructor_global` path six siblings already use; §20.1.1.1 is a 3-step
  canonical algorithm; §20.1.3.6 is a flat builtin-tag match. The one concern the
  siblings lack — the §2 NewTarget step-1-vs-3 disambiguation (Date/Number always
  return `this`; Object must sometimes return `ToObject(value) ≠ this`) — is, under
  the committed **Option A**, a single `new_target == %Object%` compare against one
  new intrinsic field, **not** an intersecting invariant (plan-review Axis 3 F4:
  named explicitly here to reconcile with §2 — the earlier Option-B divergence that
  made it look coupled is dropped). The four concerns (callability / prototype link /
  builtin-tag arms / NewTarget compare) are separable, so this does **not** meet the
  CLAUDE.md edge-dense trigger (≥3 intersecting invariant axes / no canonical
  algorithm). Plan-review is run here because (a) the re-diagnosis overturns the
  slot premise and (b) `Object` is a foundational global with real blast-radius —
  cheapest-stage verification of the split decision + spec fidelity — **not**
  because the fix is edge-dense.
- **1000-line touch discipline** (plan-review Axis 5 F8): `globals.rs` is 1507 lines
  (grew from the 1453 noted at #376 A1). The substantive new code
  (`native_object_constructor`) lands in `natives_object/`, so the `globals.rs` delta
  is the in-place `register_object_global` rewrite (<50 LoC, below the Axis-5
  >50-LoC-add backstop). Per CLAUDE.md the touch-discipline is *any-size* but judged
  by **cohesion, not line-count**: `globals.rs` is a **flat `register_*_global`
  table** (each intrinsic's registration is an independent, non-cohesive entry; the
  cohesive seams — primitives / async / errors — are *already* extracted to
  `globals_primitives.rs` / `globals_async.rs` / `globals_errors.rs`), and a flat
  case-table is an explicit CLAUDE.md exemption from split-on-touch. The residual
  1507-line debt is owned by the existing slot
  **`cleanup-vm-globals-document-file-size`** (#376 A1 Codex R13), not this PR — no
  prereq split.
