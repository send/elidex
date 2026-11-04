# PR-E1 — `[[ErrorData]]` on every error object, one way to recognise an error (per-PR plan-memo)

**Slot**: `#11-vm-native-error-prototypes` (registered 2026-11-05, PM 363; re-scoped PM 364), axis C. E1 also
closes `#11-error-object-builtintag-dual-representation` (registered at #474, the candidate slot that
`vm/tests/tests_object_constructor.rs:105-112` names): that slot is axis C as seen from
Object.prototype.toString alone. **Owner**: VM P4. **Umbrella**: `docs/plans/2026-11-vm-native-error-program.md`
(this commit); program E1 → E2, E2 gets its own plan and its own `/elidex-plan-review`. **Baseline**:
origin/main `145927c9`. Every `file:line` is at `145927c9`; §6 re-derives each one. Paths are relative to
`crates/script/elidex-js/src/` unless they start with `crates/` or `docs/`. Spec numbers come from
`.claude/tools/webref` output only (§3 states the command per row).

**Prereqs (umbrella §1)**: E1-pre, a pure-move split of `vm/host/structured_clone.rs` (1077 lines, touched
here; CLAUDE.md "1000-line debt"). E0 — proposed, **PM decision OD-2** — JSON.parse's reviver walk keyed on
`ObjectKind::Ordinary` (§0.3 P-5). E1 rebases onto both, and the driver re-derives the `structured_clone`
cites after the rebase (§6).

**The defect, stated as a defect.** An error object made by a constructor (`new TypeError('x')`,
`TypeError('x')`, `new AggregateError([])`, `class E extends TypeError`, `new WebAssembly.CompileError()`),
by `Promise.any`, or by the Wasm bridge is an `ObjectKind::Ordinary`. Only the engine throw path
(`vm/ops.rs:202`) makes an `ObjectKind::Error`. So the VM has two representations of one thing, and
everything that asks "is this an error" answers by one or the other. `Object.prototype.toString` answers by
the kind and is wrong for every constructed error (`"[object Object]"`). Structured clone and IndexedDB
answer by **prototype identity**, so `structuredClone(Object.create(Error.prototype))` clones as an error
although that object has no `[[ErrorData]]`, and `structuredClone(new (class extends TypeError {})('m'))`
clones as `{}` (message lost) although it has one.

## §0. Scope, premises, decisions

### §0.1 Scope

**In scope — one invariant, stated from both sides:**

- **Creation (C1)**: every object that the spec creates with an `[[ErrorData]]` slot is an
  `ObjectKind::Error`, and nothing else is. Error instances have the slot (ECMA-262 §20.5.4 Properties of
  Error Instances: "have an [[ErrorData]] internal slot whose value is undefined"); `Error.prototype` does not
  (§20.5.3: "is not an Error instance and does not have an [[ErrorData]] internal slot"), and neither do the
  NativeError, AggregateError and Wasm prototypes. The creation census is §1.1.
- **Recognition (C2)**: every site that decides "is this an error" keys on that kind. The classifier census is
  §1.3. HTML's structured-serialize Error branch keys on the slot by name (§2.7.4 StructuredSerializeInternal
  step 18: "if value has an [[ErrorData]] internal slot and value is not a platform object"), as does
  Object.prototype.toString (§20.1.3.6 step 8).
- **The kind's payload**: `ObjectKind::Error { name: StringId }` becomes a payload-free `ObjectKind::Error`
  (§1.2).

**Not in scope** (each stated as a defect with an owner in §5):

- Own NativeError prototypes, `EvalError`, constructor `[[Prototype]]` = %Error%, and every construction
  site that chooses a prototype (`vm_error_to_thrown`'s `error_prototype`, call-mode `error_entry`,
  `clone_error`'s prototype rule): **E2**. E1 changes no prototype anywhere, unless the PM takes **OD-1 (b)**
  (§0.4).
- The own `.name` property that every elidex error constructor and the throw path define on the
  **instance** (`vm/natives.rs:455-461` in `error_ctor_impl`, `vm/ops.rs:209-214`). The spec defines `name`
  on the prototypes only (§20.5.3.3, §20.5.6.3.3). It is a workaround for the shared prototype and goes with
  E2's own prototypes. E1 keeps it.
- `InstallErrorCause`, `Error.isError`, AggregateError's step order, DOMException's `[[ErrorData]]` and its
  serialization, and the rest of HTML step 18 / step 22 fidelity (§5).

### §0.2 Premises re-measured

| Premise (brief) | At `145927c9` | Evidence |
|---|---|---|
| `register_error_constructors` gives the six constructors one `.prototype` | holds (`vm/globals_errors.rs:66-89`; docstring `:24-32` admits it) | read; probe `m10-probe3.log` "proto identity matrix: 11111 ×5" |
| User errors are `ObjectKind::Ordinary`; toString → `"[object Object]"` for `new Error`, `new TypeError`, `TypeError('x')`, `new RangeError`, `new AggregateError`, `class E extends TypeError` | holds | probe `e1-probe.log` rows "toString user errors", "toString subclass" |
| Engine-thrown errors are `ObjectKind::Error { name }`, toString → `"[object Error]"` | holds (`vm/ops.rs:171`, alloc `:201-206`) | probe "toString thrown" |
| `natives_symbol.rs` ~`:284` keys on the kind | holds (`native_object_prototype_to_string`, `vm/natives_symbol.rs:240`, arm `:284`) | read |
| `do_new` / `construct_synchronous` pre-allocate an `Ordinary` from newTarget.prototype, and `ensure_instance_or_alloc` returns `this` in construct mode | holds (`vm/ops.rs:722-729`; `vm/inner.rs:152`, `:158-161`); the super path passes the derived frame's `this_value`, which is `do_new`'s instance (`vm/dispatch_class.rs:138-148`) | read |
| Classifiers by prototype identity at `structured_clone.rs` ~`:124` and ~`:699` (`is_error_proto`) and `indexeddb/value.rs` ~`:365` | holds, with the names corrected (§0.3 P-1): guard `vm/host/structured_clone.rs:133`, predicate `is_error_like_proto` `:697-702`; `is_error_like` `vm/host/indexeddb/value.rs:363-371` | `git grep -n 'error_prototype' 145927c9 -- crates/script/elidex-js/src` |
| `structuredClone(Object.create(Error.prototype))` clones as an Error | holds: `instanceof Error` true; the clone's kind is `Ordinary` (toString `"[object Object]"`), because `clone_error` copies the source kind (`:648-652`) | probe "structuredClone fake error" → `cloned:[object Object]:true` |

### §0.3 Premise errors

- **P-1 — names.** The structured-clone predicate is `is_error_like_proto` (`vm/host/structured_clone.rs:697`),
  not `is_error_proto`. The IndexedDB predicate is `is_error_like` (`vm/host/indexeddb/value.rs:363`). The
  two are **different predicates**: the first compares the immediate prototype for equality, the second
  walks the chain. That is already two ways (§1.3).
- **P-2 — the creation list is incomplete.** The brief names the Error / NativeError / AggregateError
  constructors. The census by concept (§1.1) finds three more creators of objects the spec gives
  `[[ErrorData]]`: the three `WebAssembly.*Error` constructors, which share `error_ctor_impl`
  (`vm/natives.rs:553`, `:568`, `:583`); the Wasm bridge's thrown errors (`wasm_error_to_js_value`,
  `vm/host/wasm/errors.rs:179`, alloc `:201-206`); and `Promise.any`'s rejection (`build_aggregate_error`,
  `vm/natives_promise_combinator.rs:258`, alloc `:259-264`). Wasm JS API §5.10 Error Objects creates each
  Wasm error class as "a new object, implementing the NativeError Object Structure"
  (`.claude/tools/webref body wasm-js-api-2 error-objects`), so their instances have the slot.
  PerformPromiseAny (ECMA-262 §27.5.4.3.1) steps 5.b.ii.1 and 5.e.vii.1 reject with "a newly created
  AggregateError object". Leaving any of them `Ordinary` keeps two representations, so E1 includes all
  three (§1.1). **This is a census correction, not a further axis**: same invariant, same mechanism. Flagged
  for the driver because it reaches the Wasm surface the brief asked about (§0.4).
- **P-3 — "Map / Set / … use OrdinaryCreateFromConstructor with a slot list".** Map, Set and WeakMap do not
  exist in the VM (`git grep -n 'map_constructor' 145927c9 -- crates/script/elidex-js/src` → no output). The
  built-ins that do exist are §1.2's census.
- **P-4 — "AggregateError is correct".** True of `AggregateError.prototype` (own object, chained to
  `Error.prototype`, `vm/globals_errors.rs:93-112`). False of the constructor: `create_constructable_function`
  (`vm/shape_ops.rs:281-288`) allocates every native with `prototype: self.function_prototype`
  (`vm/shape_ops.rs:511`), so `Object.getPrototypeOf(AggregateError)` is `Function.prototype`, not %Error%
  (§20.5.7.2 Properties of the AggregateError Constructor: "has a [[Prototype]] internal slot whose value is
  %Error%"). Read, not probed. Same for the three Wasm constructors (NativeError Object Structure,
  §20.5.6.2). Axis B, so **E2** (umbrella §4). Also missing: `Error.prototype.constructor`
  (§20.5.3.1) — `register_error_constructors` defines `toString`, `name` and `message` on `error_proto` and no
  `constructor` (`vm/globals_errors.rs:34-63`). Axis A, **E2**.
- **P-5 — "correct alone, regresses nothing" does not hold for the narrow E1 without E0.** One consumer
  branches on `ObjectKind::Ordinary` and so sees constructed errors move out of its arm: JSON.parse's
  reviver walk `internalize` (`vm/natives_json/parse.rs:403`) walks the keys of an `Ordinary` or `Arguments`
  object (`:444`) and skips every other kind (`:469`, `_ => {}`). InternalizeJSONProperty (ECMA-262
  §25.5.2.4) step 5.c walks EnumerableOwnProperties of **every** non-array object. Today a constructed error
  is walked and an engine-thrown one is not; after the narrow E1 neither is. The defect is pre-existing (it
  also skips Date, RegExp, wrappers, thrown errors) and independent of errors. Measured by §2's criterion it
  is a separate axis (its mutant fails one cell while every other passes), so it is **not** folded in: it is
  the proposed prereq **E0** and PM decision **OD-2** (§0.4). The census that found it is §1.4.
- **P-6 — the narrow E1 changes two spec-correct observables in structured clone.** Because E1 routes every
  `[[ErrorData]]` object to `clone_error`, and `clone_error` keeps the **source** prototype
  (`vm/host/structured_clone.rs:656`, `src_proto.or(vm.error_prototype)`), two rows that clone as `{}` today
  would clone as errors with the source's prototype: `structuredClone(new (class E extends TypeError {})())
  instanceof E` and `structuredClone(new WebAssembly.CompileError()) instanceof WebAssembly.CompileError`
  go from `false` (spec-correct, by accident of the `{}`) to `true` (spec: `false`). HTML StructuredDeserialize
  (§2.7.7) step 22 chooses the prototype **by the serialized name**, from the seven names step 18.2 keeps,
  never from the source. The rows are read, not probed. **PM decision OD-1** (§0.4).

### §0.4 Open decisions — for the PM

- **OD-1 — the clone's prototype rule (P-6).** (a) E1 keeps `clone_error`'s source-prototype rule; the two
  rows change as P-6 says and E2 fixes them. (b) E1 moves HTML §2.7.7 step 22's prototype choice into
  `clone_error`: the prototype is a function of the serialized name (step 18.1-18.2: `? Get(value, "name")`,
  any name outside the seven → "Error"), through one lookup `error_intrinsic_prototype(name)` (NEW, under
  (b) only) that in E1 returns `error_prototype` for all seven names — which is what %TypeError.prototype%
  *is* in elidex until E2 — and that E2 changes to the own prototypes. Under (b) E1 regresses nothing, the
  AggregateError clone stops being `instanceof AggregateError` (spec: name "AggregateError" is not one of the
  seven), and E2 inherits a seam instead of a construction site. **Designer recommendation: (b).** It moves
  one site the umbrella assigned to E2 ("`clone_error` fallback") into E1, which is why it is the PM's call
  and not taken here. Under (b) the `clone_error` row of §1.1 gains the lookup and §4 gains T-CLONE-SUB's (b)
  form; nothing else in this memo changes.
- **OD-2 — E0 (P-5).** (a) A standalone prereq PR, proposed slot `#11-vm-json-internalize-non-array-objects`
  (silent-wrong today for Date / RegExp / wrappers / thrown errors; correct alone), landing before E1.
  (b) Folded into E1, which then has three measured axes (§2). **Designer recommendation: (a)** — the
  defect is not about errors, and the brief's rule is to report a further axis rather than widen.
- **OD-3 — Wasm in E1 (P-2).** The census puts the Wasm constructors and the Wasm bridge in E1. The
  alternative — leaving both Wasm paths `Ordinary` — keeps a second representation for objects that have the
  slot. **Designer decision: include**; listed so the driver can overrule it.

## §1. Design

### §1.1 Creation census

Found by concept (who allocates an error instance), not by the brief's list:
`git grep -n 'error_prototype' 145927c9 -- crates/script/elidex-js/src ':!crates/script/elidex-js/src/vm/tests'`,
`git grep -n 'ObjectKind::Error' 145927c9 -- crates/`,
`git grep -n 'error_ctor_impl\|wasm_error_to_js_value\|build_aggregate_error' 145927c9 -- crates/script/elidex-js/src`.

| Creator | Site | Kind today | E1 |
|---|---|---|---|
| `Error`, `TypeError`, `ReferenceError`, `RangeError`, `SyntaxError`, `URIError` — construct and call mode | `error_entry` `vm/natives.rs:487` → `ensure_instance_or_alloc` `:493-495` | `Ordinary` | `error_receiver` (NEW) |
| subclass `class E extends TypeError` — `super(...)` reaches the native with the derived frame's `this` | `vm/dispatch_class.rs:138-148` → `construct_synchronous` → `error_entry` | `Ordinary` (E.prototype preserved) | same path, promoted in place, prototype untouched |
| `AggregateError` — construct and call mode | `vm/natives.rs:601`, receiver `:637-639` | `Ordinary` | `error_receiver` (NEW) |
| `WebAssembly.CompileError` / `LinkError` / `RuntimeError` | `vm/natives.rs:553`, `:568`, `:583` (engine-gated) | `Ordinary` | `error_receiver` (NEW) |
| engine throw — `VmError` → JS value | `vm_error_to_thrown` `vm/ops.rs:171`, alloc `:201-206` | `Error { name }` | `create_error_object` (NEW); payload dropped |
| `Promise.any` rejection | `build_aggregate_error` `vm/natives_promise_combinator.rs:258`, alloc `:259-264` | `Ordinary` | `create_error_object` (NEW) |
| Wasm bridge — engine-thrown Wasm errors | `wasm_error_to_js_value` `vm/host/wasm/errors.rs:179`, alloc `:201-206` | `Ordinary` | `create_error_object` (NEW) |
| structured clone of an error | `clone_error` `vm/host/structured_clone.rs:638`, kind `:648-652` | copies the source kind | always `create_error_object` (NEW); the `Ordinary` arm goes (unreachable after §1.3) |

**Not creators of error instances** (checked, no change): `Error.prototype`, `AggregateError.prototype` and
the Wasm prototypes are ordinary per spec (`vm/globals_errors.rs:34`, `:93`; `vm/host/wasm/errors.rs:103`);
DOMException (`vm/host/dom_exception.rs:215`, `:309`) is a platform object, §1.5.

### §1.2 Mechanism — how `[[ErrorData]]` reaches a pre-allocated receiver

**What the existing built-ins with internal slots do.** `do_new` allocates the receiver `Ordinary` from
`ctor.prototype` before the native body runs (`vm/ops.rs:722-729`), and every native constructor with a slot
**promotes that receiver's kind in place**, leaving its prototype alone (so a subclass keeps
newTarget.prototype). Census: `git grep -n '\.kind = ObjectKind::' 145927c9 -- crates/script/elidex-js/src ':!crates/script/elidex-js/src/vm/tests'`
→ 41 hits, five of them inside `vm/inner.rs`'s helpers. One mechanism, two spellings:

- **Named `VmInner` helpers**, the ES-core built-ins: `promote_to_string_wrapper` (`vm/inner.rs:213`),
  `promote_to_number_wrapper` (`:234`), `promote_to_date` (`:252`), `promote_to_boolean_wrapper` (`:267`),
  `promote_to_array` (`:274`), each beside a fresh-allocation sibling (`create_date` `:240`,
  `create_number_wrapper`, …) for the paths that have no receiver. Callers: `natives_number.rs:47`,
  `natives_boolean.rs:38`, `natives_string.rs:759`, `natives_date/mod.rs:140`, `natives.rs:714`.
- **Inline `get_object_mut(id).kind = …`**, in the WebIDL constructors under `vm/host/` (ArrayBuffer
  `vm/host/array_buffer.rs:507`, Blob, DataView, the observers, the Wasm objects, …) and the Promise
  constructor (`vm/natives_promise.rs:731`).

Not two mechanisms: both promote the same `do_new` receiver in place. **Canonical form for E1 = the named
helper**, because the Error family is ES core and its nearest relative is `Date` (`create_date` +
`promote_to_date`). The inline spelling is a spelling, not a second mechanism; E1 does not touch it (§5,
hygiene note).

**The design:**

- `VmInner::promote_to_error(obj_id)` (NEW, `vm/inner.rs`, beside `promote_to_date`): sets
  `ObjectKind::Error`. It `debug_assert!`s that the kind was `Ordinary`: the only non-`Ordinary` receiver a
  native constructor can see is a custom-element upgrade's existing element
  (`vm/host/custom_elements/upgrade.rs:315`), and that path requires an HTMLElement chain, so an error
  constructor never sees it; the assert pins that.
- `VmInner::create_error_object(prototype)` (NEW, `vm/inner.rs`, beside `create_date`): allocates an
  `ObjectKind::Error` — the four engine creators of §1.1 (`vm_error_to_thrown`, `build_aggregate_error`,
  `wasm_error_to_js_value`, `clone_error`).
- `error_receiver(ctx, this, prototype) -> ObjectId` (NEW, `vm/natives.rs`): `ensure_instance_or_alloc`
  then `promote_to_error`. The one receiver step for all ten constructors (`error_entry` ×6, AggregateError,
  the three Wasm constructors). `error_ctor_impl` stays the init step its docstring says it is
  (`vm/natives.rs:438-443`). In call mode `ensure_instance_or_alloc` allocates a fresh object with the
  prototype the caller passed, which is ECMA-262's "If NewTarget is undefined, let newTarget be the active
  function object" (§20.5.1.1 step 1, §20.5.6.1.1 step 1, §20.5.7.1.1 step 1) as far as E1 can follow it;
  which prototype that is per constructor is E2's.

**Order against the spec.** OrdinaryCreateFromConstructor (§10.1.13) with « [[ErrorData]] » is step 2 of each
constructor, before step 3's `? ToString(message)`. `error_receiver` runs before `error_ctor_impl`, so the
promotion precedes the ToString. Not observable either way (the receiver is unreachable from the message's
`toString`), but the code follows the step order. AggregateError's receiver comes after its iterator walk
today (`vm/natives.rs:613-621`, receiver `:637-639`); that order is SD-3 (§5), unchanged here.

**Subclass.** `class E extends TypeError {}`: `new E('m')` → `do_new` allocates an `Ordinary` with
`E.prototype` → the derived constructor's `super(...)` passes that object as `receiver`
(`vm/dispatch_class.rs:138-148`) → `error_entry` → `error_receiver` returns it and promotes it. The kind
changes; `E.prototype` stays. T-SUB-PROTO pins both halves.

**The payload (question 2).** `ObjectKind::Error { name: StringId }` (`vm/object_kind.rs:135`). Every site
that names the variant, by `git grep -n 'ObjectKind::Error' 145927c9 -- crates/`:

| Site | Reads `name`? |
|---|---|
| `vm/ops.rs:202` (write, the throw path) | writes it |
| `vm/host/structured_clone.rs:649` (`clone_error`) | reads it, only to copy it into the clone's kind |
| `vm/host/structured_clone.rs:242` (`classify`) | no (`{ .. }`) |
| `vm/host/indexeddb/value.rs:298` | no (`{ .. }`) |
| `vm/natives_symbol.rs:284` | no (`{ .. }`) |
| `vm/gc/trace.rs:556` | no (`{ .. }`; a `StringId`, no GC edge) |

No site reads the value for behaviour: the only reader copies it. The spec slot carries no value (§20.5.4:
"whose value is undefined"). Giving constructed errors the payload would mean choosing a value (the
constructor's name? the subclass's? what `.name` says after a script writes it?), and any choice is a second
source of truth beside the `name` property, which is what `Error.prototype.toString` (§20.5.3.4) and HTML
step 18.1 actually read. **Decision: the variant becomes payload-free `ObjectKind::Error`**, its six sites
respelled. It is E1's (the kind is the recognition token). The own `.name` **property** on instances is
the E2 half of the same duplication (§0.1), stated, not fixed here.

### §1.3 Recognition census (question 3)

By concept — anything that decides "is this an error" by prototype identity, by an `error_prototype` /
`aggregate_error_prototype` comparison, or by the kind. Commands: the two in §1.1, plus
`git grep -n -i 'error_like\|is_error' 145927c9 -- crates/script/elidex-js/src ':!crates/script/elidex-js/src/vm/tests'`.

| Site | Today | E1 |
|---|---|---|
| Object.prototype.toString — `native_object_prototype_to_string` `vm/natives_symbol.rs:240`, arm `:284` (§20.1.3.6 step 8) | the kind | unchanged (respelled); now true for every constructed error |
| structured clone — `clone_recursive` guard `vm/host/structured_clone.rs:133` + `is_error_like_proto` `:697-702` | the kind (`classify` `:242`) **or** an `Ordinary` whose immediate prototype `==` `error_prototype` / `aggregate_error_prototype` | the kind only (§2.7.4 step 18); guard and predicate deleted |
| structured clone — `clone_error` kind copy `:648-652` | copies `Error { name }` or `Ordinary` | always the kind (§1.1) |
| IndexedDB — `reject_non_json_storable` `vm/host/indexeddb/value.rs:242`: arm `:298` + `Ordinary` branch `:318-323` → `is_error_like` `:363-371` | the kind **or** an `Ordinary` whose chain reaches either prototype | the kind only; the `Ordinary` error branch and `is_error_like` deleted (question 4) |

No other site compares against either prototype: the remaining `error_prototype` hits are allocations
(§1.1), registration (`vm/globals_errors.rs`, `vm/host/wasm/errors.rs:55-86`, `vm/host/dom_exception.rs:252`)
and GC roots (`vm/gc/collect.rs:74-75`). The remaining `is_error` hit is an unrelated local
(`vm/host/indexeddb/request.rs:131`). `Error.prototype.toString` (`vm/natives.rs:401`) checks only "is an
Object", as §20.5.3.4 step 2 does.

**HTML structured serialize.** WHATWG HTML §2.7.4 StructuredSerializeInternal step 18 is the Error branch,
keyed on "has an [[ErrorData]] internal slot and value is not a platform object"
(`.claude/tools/webref body html structuredserializeinternal`). Anything else that is not a platform object
falls to the ordinary-object branch. So the fake `Object.create(Error.prototype)` is an ordinary object (its
clone is `{}` on `Object.prototype`), and the subclass instance and the Wasm errors are errors.

### §1.4 Consumers of the kind change (the sweep that found P-5)

E1 moves constructed errors out of `ObjectKind::Ordinary`. Every site whose behaviour depends on a value
being `Ordinary` sees them move:
`git grep -n 'ObjectKind::Ordinary' 145927c9 -- crates/script/elidex-js/src ':!crates/script/elidex-js/src/vm/tests' | /usr/bin/grep -v 'kind: ObjectKind::Ordinary'`
→ 10 hits. Five are comments (`vm/globals.rs:1371`, `vm/host/dom_exception.rs:7`, `vm/host/dom_rect.rs:8`, `vm/host/structured_clone.rs:127`, `vm/host/wasm/namespace.rs:41`). `vm/gc/trace.rs:553` shares `Error`'s arm (no change). `vm/host/structured_clone.rs:239`
/ `:650` and `vm/host/indexeddb/value.rs:295` are §1.3's. **`vm/natives_json/parse.rs:444` is P-5** (E0, OD-2).
Kind-agnostic paths (property storage, `JSON.stringify`, which branches only on Array / Date / wrappers at
`vm/natives_json/stringify.rs:63`, `:131`, `:470`, `:547-550`; the JSON shortcut behind worker `postMessage` and
`history.state`, `vm/host/structured_serialize.rs`) do not branch on `Ordinary`, so an `Error` object takes
the same path there as today's constructed error.

### §1.5 DOMException and the Wasm errors (question 3)

- **Wasm errors have `[[ErrorData]]`** (Wasm JS API §5.10, quoted in §0.3 P-2). E1 changes their recognition:
  both Wasm creators are in §1.1 (OD-3). After E1, `Object.prototype.toString.call(new
  WebAssembly.CompileError())` is `"[object Error]"`; structured clone routes them to `clone_error` (OD-1
  decides the prototype); IndexedDB rejects them like every other error (`DataCloneError`, the v1 JSON-storage
  stance of `#11-idb-structured-clone-storage`) instead of silently storing `{}`.
- **DOMException has `[[ErrorData]]`** per Web IDL §3.14.1 DOMException custom bindings ("It also has
  [[ErrorData]] and [[Stack]] slots, like all built-in exceptions",
  `.claude/tools/webref body webidl js-DOMException-specialness`), and HTML step 18's "and value is not a
  platform object" exists for exactly that case: a DOMException serializes by its own [Serializable] steps
  (Web IDL §4.4 DOMException), never by step 18. **E1 does not change DOMException's recognition**: it stays
  `Ordinary` with its state in `dom_exception_states` (`vm/host/dom_exception.rs:224`, `:318`), so no E1
  classifier sees it as an error, which is what step 18 requires. Its slot has no consumer in elidex today
  (the only spec consumers are toString step 8, which its `@@toStringTag` overrides, and `Error.isError`, which
  elidex lacks). Modelling it — a "has `[[ErrorData]]`" predicate that admits a platform object, kept apart
  from step 18's "and not a platform object" — **is a further axis, reported (stop-and-report SR-2) and not
  taken**. It belongs with `Error.isError` (SD-2) and DOMException serialization (SD-4).

## §2. Edge density, by measurement

Criterion (the IsConstructor memo's §2, `docs/plans/2026-11-vm-isconstructor-check.md`): an axis is
evidenced only by a failure that occurs while the others pass. Measured on the base tree and derived for the
§4.2 mutants from the §4.1 cell bodies (the post-implementation run records it).

- **C1, creation** — every `[[ErrorData]]` creator makes the kind. Base evidence: the thrown-error toString
  cell passes while every constructed-error cell fails (probe `e1-probe.log`), with the classifier (toString's
  kind arm) correct. Mutants M1-M7, M9.
- **C2, recognition** — every classifier keys on the kind. Evidence: with C1 fixed and the structured-clone
  guard kept (M8), T-CLONE-FAKE and T-IDB-FAKE fail while every T-TAG cell passes. Mutants M8, M10, M11.

| Mutant | Cells it fails | Fails alone? | Axis |
|---|---|---|---|
| M3 promotion keyed on the receiver's prototype being `error_prototype` / `aggregate_error_prototype` | T-TAG subclass rows, T-WASM constructor rows, T-CLONE-SUB, T-IDB-SUB | yes | C1 |
| M8 structured clone keeps `CloneKind::Ordinary if is_error_like_proto` | T-CLONE-FAKE, T-IDB-FAKE | yes | C2 |
| M9 `clone_error` allocates `Ordinary` | T-CLONE-KIND; the existing `err:` row of `add_value_with_cloneable_non_json_type_throws_data_clone_error` (`vm/tests/tests_indexeddb_events.rs:560`), once IDB's prototype branch is gone | yes | C1 (`clone_error` is a creator) |
| M14 (only under OD-2 (b)) `internalize` keeps `ObjectKind::Ordinary \| Arguments` | T-JSON's constructed-error row | yes | **C3** — the consumer axis P-5 names |

The payload removal adds no axis: no site reads the value (§1.2), so no cell can fail on it. The IDB
prototype branch adds none either: it becomes unreachable (§4.2 M12).

**Two measured axes under OD-2 (a); three under OD-2 (b).** Either way E1 is a terminal unit: the base case
of CLAUDE.md "Edge-dense work" (c), a plan-reviewed, narrowly scoped slice under an approved umbrella
(`docs/plans/2026-11-vm-native-error-program.md`). And C1 and C2 are not separable: a PR that does C1 alone
makes the fake-error clone and the IDB rows depend on which predicate the clone takes, and a PR that does C2
alone makes every constructed error unclonable as an error (the kind it keys on does not exist yet) — the
second is a regression of `error_subclass_proto_preserved` (`vm/tests/tests_structured_clone.rs:171`). So C1 +
C2 is terminal by necessity.

**Re-slice trigger**: the post-implementation run finds a mutant that fails alone and breaks an invariant
neither C1 nor C2 owns (C3 counts only under OD-2 (b)).

## §3. Spec coverage map

Section numbers from `.claude/tools/webref aoid ecma262 <AO>` / `.claude/tools/webref heading ecma262 20.5`
/ `.claude/tools/webref dfn html <AO>`; step text from `.claude/tools/webref body <spec> <anchor-or-AO>`.
Skeleton and breadth verdict: `.claude/tools/webref coverage-map ecma262 20.5.1.1 ecma262 20.5.6.1.1 ecma262 20.5.7.1.1 ecma262 20.1.3.6 ecma262 10.1.13 ecma262 PerformPromiseAny html 2.7.4 html 2.7.7`
→ "spec=2 (ecma262, html), step=8 entries — ok → single PR scope" (verified 2026-11-05). The Wasm JS API and
IndexedDB rows are labels the preflight does not resolve (it warns and skips them); their numbers come from
`.claude/tools/webref heading wasm-js-api-2 5.10` and `.claude/tools/webref dfn IndexedDB-3 clone`.
**Breadth.** The preflight counts K = 5 specs and says SPLIT-RECOMMENDED. The three extra specs are not
extra subsystems: the IndexedDB row is the consumer of HTML's algorithm (it touches only the classifier the
HTML row already owns), the Wasm JS API row says which objects the ECMA-262 creation rows cover, and the Web
IDL row touches nothing (it records why DOMException is left alone). The split verdict is §2's, by measured
axes, not by spec count.

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| ECMA-262 §20.5.1.1 Error ( message [ , options ] ) | 1-2 | NewTarget undefined → active function; OrdinaryCreateFromConstructor with « [[ErrorData]] » | `error_entry` `vm/natives.rs:487` → `error_receiver` (NEW) | ✓ both modes | yes — any newTarget, any message |
| ECMA-262 §20.5.6.1.1 NativeError ( message [ , options ] ) | 1-2 | same, ×5 (EvalError absent: E2) | `vm/natives.rs:507-545` → `error_entry` | ✓ the five that exist | yes |
| ECMA-262 §20.5.7.1.1 AggregateError ( errors, message [ , options ] ) | 1-2 | same | `vm/natives.rs:601`, receiver `:637-639` → `error_receiver` (NEW) | ✓ | yes |
| ECMA-262 §10.1.13 OrdinaryCreateFromConstructor ( ctor, intrinsicDefaultProto [ , internalSlotsList ] ) | 1-2 | the slot list « [[ErrorData]] » on the object whose prototype came from newTarget (subclass included) | `do_new` pre-allocation `vm/ops.rs:722-729` + `promote_to_error` (NEW) | ✓ construct, call, super | yes — `class E extends TypeError` |
| ECMA-262 §20.5.4 Properties of Error Instances | — | instances have [[ErrorData]] (value undefined); the prototypes do not (§20.5.3) | `ObjectKind::Error` payload dropped, `vm/object_kind.rs:135` | ✓ | no |
| ECMA-262 §27.5.4.3.1 PerformPromiseAny ( iteratorRecord, ctor, resultCapability, promiseResolve ) | 5.b.ii.1, 5.e.vii.1 | "a newly created AggregateError object" | `build_aggregate_error` `vm/natives_promise_combinator.rs:258` → `create_error_object` (NEW) | ✓ both rejection points share the builder | yes — every input rejects |
| ECMA-262 §20.1.3.6 Object.prototype.toString ( ) | 8 | [[ErrorData]] → "Error" (step 15-16 tag override unchanged) | `vm/natives_symbol.rs:284` | ✓ | yes |
| WHATWG HTML §2.7.4 StructuredSerializeInternal ( value, forStorage [ , memory ] ) | 18 | [[ErrorData]] and not a platform object → Error record; otherwise falls through to the ordinary-object branch | `vm/host/structured_clone.rs:133` (guard deleted), `:242` | ✓ fake error, subclass, Wasm, AggregateError, thrown | yes — `structuredClone(x)` |
| WHATWG HTML §2.7.7 StructuredDeserialize ( serialized, targetRealm [ , memory ] ) | 22.9 | OrdinaryObjectCreate(prototype, « [[ErrorData]], [[Stack]] ») — the clone has the slot (22.1-22.7, the prototype choice: OD-1) | `clone_error` `vm/host/structured_clone.rs:638` → `create_error_object` (NEW) | ✓ | yes |
| WHATWG HTML §2.7.6 StructuredSerializeForStorage ( value ) | — | IndexedDB's clone serializes through the same algorithm; v1 then rejects every Error record (JSON storage) | `vm/host/indexeddb/value.rs:298` (kept), `:318-323` + `:363-371` (deleted) | ✓ | yes — `store.add(x)` |
| IndexedDB §5.11 Clone a value | 3-4 | StructuredSerializeForStorage then StructuredDeserialize — one classifier, structured clone's | `value_to_json` → `clone_value` `vm/host/indexeddb/value.rs:382` | ✓ | yes |
| Wasm JS API §5.10 Error Objects | 2.1 | each class implements the NativeError Object Structure → instances have [[ErrorData]] | `vm/natives.rs:553`, `:568`, `:583`; `vm/host/wasm/errors.rs:201-206` | ✓ three classes, both paths | yes |
| Web IDL §3.14.1 DOMException custom bindings | — | DOMException has [[ErrorData]] but is a platform object: never step 18 — **unchanged by E1** (SR-2) | none | ✓ (no touch) | yes |

### §3.1 User-input audit

Every input is script-controlled: the constructor's newTarget (any subclass), the message, the value handed
to `structuredClone` and to `IDBObjectStore.add` / `put` (any object, any prototype, including one forged
from `Object.create(Error.prototype)`), and the inputs whose rejection `Promise.any` aggregates. After E1 the
error/not-error decision depends only on how the object was created, never on a prototype the script can
choose, so a forged prototype can no longer make a plain object clone or store as an error, and a subclass
can no longer make an error clone as `{}`.

## §4. Tests

### §4.1 Cells

New file `crates/script/elidex-js/src/vm/tests/tests_error_data.rs` (NEW), `#![cfg(feature = "engine")]`
like `tests_object_constructor.rs` (Object.prototype.toString needs the `engine` globals), registered in
`vm/tests/mod.rs` (203 lines). Helpers: `super::helpers::{eval_bool, eval_string}` and
`super::eval_global_string` (the microtask-drained global read `tests_promise.rs` uses). The IDB cells go in
`vm/tests/tests_indexeddb_value.rs` (120 lines; its charter is "which JS values may cross into the
backend"), on its `with_vm` harness. Base values marked *probe* are from `e1-probe.log` at `145927c9`; the
rest are read, not probed.

| Cell | Asserts | Base (`145927c9`) | Mutants it kills |
|---|---|---|---|
| T-TAG | `Object.prototype.toString.call(x)` is `"[object Error]"` for `new Error('x')`, `new TypeError('x')`, `new ReferenceError('x')`, `new RangeError('x')`, `new SyntaxError('x')`, `new URIError('x')`, `Error('x')`, `TypeError('x')`, `new AggregateError([], 'x')`, `AggregateError([])`, `new (class E extends TypeError {})('x')`, `new (class F extends Error {})()` | red for all (*probe* for five constructed, call-mode `TypeError`, the subclass) | M1, M2 (call rows), M3 (subclass rows), M4 (AggregateError rows), M11 |
| T-TAG-CTL | `Object.create(Error.prototype)` → `"[object Object]"`; `Error.prototype`, `TypeError.prototype`, `AggregateError.prototype` → `"[object Object]"`; a caught `null.x` → `"[object Error]"`; `e[Symbol.toStringTag] = 'X'` on a `new Error()` → `"[object X]"` | green (fake and thrown: *probe*); stays green | a promotion that over-reaches (prototypes) |
| T-SUB-PROTO | `class E extends TypeError {}; const e = new E('m');` → `Object.getPrototypeOf(e) === E.prototype && e instanceof TypeError && e.message === 'm'` | green; stays green | a promotion that re-allocates or resets the prototype |
| T-ANY | `Promise.any([Promise.reject(1)]).catch(e => { globalThis.out = Object.prototype.toString.call(e); })` → `"[object Error]"` | red | M6 |
| T-WASM | toString of `new WebAssembly.CompileError('m')`, `new WebAssembly.LinkError('m')`, `new WebAssembly.RuntimeError('m')`, and of the error caught from `new WebAssembly.Module(new Uint8Array([0xff]).buffer)` (the bridge path, `tests_wasm_compile.rs:121-127`'s body) → `"[object Error]"` | red | M5 (constructor rows), M7 (bridge row), M3 |
| T-CLONE-KIND | `c = structuredClone(new TypeError('m'))` → `Object.prototype.toString.call(c) === '[object Error]'`, `c instanceof TypeError`, `c.name === 'TypeError'`, `c.message === 'm'` | red on the toString half (*probe* `true,true,TypeError,m,[object Object]`) | M9, M1 |
| T-CLONE-FAKE | `c = structuredClone(Object.create(Error.prototype))` → `c instanceof Error === false`, `Object.getPrototypeOf(c) === Object.prototype`, toString `"[object Object]"` | red (*probe* `cloned:[object Object]:true`) | M8 |
| T-CLONE-THROWN | a caught `null.x` cloned → `instanceof TypeError`, `name === 'TypeError'`, `"[object Error]"` | green (*probe*); stays green | — (control) |
| T-CLONE-SUB | `class E extends TypeError {}; c = structuredClone(new E('m'))` → `"[object Error]"`, `c instanceof TypeError`, `c.message === 'm'`; and `c instanceof E` is `true` under OD-1 (a), `false` under OD-1 (b) | red (base clones `{}`: `clone_ordinary` drops the non-enumerable `name` / `message`, `vm/host/structured_clone.rs:465-505`) | M1, M3, M13 under (b) |
| T-IDB-FAKE | `store.add({ wrapped: Object.create(Error.prototype) }, 1)` does not throw | red (base: `clone_error` keeps `error_prototype` and the `Ordinary` kind, then `is_error_like` rejects with `DataCloneError`) | M8 |
| T-IDB-SUB | `store.add({ wrapped: new (class extends TypeError {})('x') }, 1)` → `DataCloneError` | red (base: the clone is `{}`, stored) | M1, M3 |
| T-JSON (OD-2) | a constructed error with an own enumerable `k`, placed by the reviver into a later key, has `k` visited: `const e = new Error('x'); e.k = 1; let seen = false; JSON.parse('{"a":0,"b":0}', function (k, v) { if (k === 'a') this.b = e; if (k === 'k') seen = true; return v; }); seen` → `true`; the same with a caught thrown error → `true` | constructed row green, thrown row red | M14; under OD-2 (a) this cell is E0's, and E1's DoD keeps it green |

**Existing tests that change or must stay green:**

- `generic_object_prototype_tostring_builtin_tags` (`vm/tests/tests_object_constructor.rs:67`): its
  `new Error('e')` assertion at `:111-114` **pins the defect** (`"[object Object]"`) with a comment
  (`:105-112`) naming the candidate slot. E1 flips it to `"[object Error]"` and replaces the comment with
  the §20.1.3.6 step 8 cite. A red-at-base cell that already exists, inverted.
- `error_subclass_proto_preserved` (`vm/tests/tests_structured_clone.rs:171`): green, stays green.
- `add_value_with_cloneable_non_json_type_throws_data_clone_error` (`vm/tests/tests_indexeddb_events.rs:560`),
  its `err: new Error('x')` row (`:579`): green, stays green — after E1 it is rejected by the kind arm
  (`value.rs:298`) instead of by `is_error_like`.
- `promise_any_aggregate_error_has_own_name_matching_ctor_path` (`vm/tests/tests_promise.rs:514`) and the
  Wasm `instanceof` cells (`tests_wasm.rs:557`, `:588`; `tests_wasm_compile.rs:125`): green, stay green.

### §4.2 Mutants

Run on the final head only, sequentially (≤ 2 parallel builds), each recorded as killed-by-cell. A mutant no
cell kills is a missing cell, added before the PR leaves review.

- **M1** `error_receiver` without `promote_to_error`. **M2** promotion only when `ctx.mode.is_construct()`
  (call-mode allocations stay `Ordinary`). **M3** promotion keyed on prototype identity (only when the
  receiver's `[[Prototype]]` is `error_prototype` / `aggregate_error_prototype`).
- **M4** the AggregateError constructor bypasses `error_receiver`. **M5** the three Wasm constructors bypass
  it.
- **M6** `build_aggregate_error` allocates `Ordinary`. **M7** `wasm_error_to_js_value` allocates `Ordinary`.
- **M8** `clone_recursive` keeps the `CloneKind::Ordinary if is_error_like_proto` arm.
- **M9** `clone_error` allocates `Ordinary`.
- **M10** IndexedDB's `ObjectKind::Error` arm (`value.rs:298`) removed → the existing `err:` row.
- **M11** toString's `ObjectKind::Error` arm removed → T-TAG, and T-TAG-CTL's thrown row.
- **M12 — equivalent, no cell.** Keeping IndexedDB's `is_error_like` branch. After E1 it cannot fire: the walk
  runs on the clone (`value.rs:382`), a clone of an `[[ErrorData]]` object is always `ObjectKind::Error`
  (caught by the arm before it), and every other clone with an object-ish kind is allocated on
  `Object.prototype` (`clone_ordinary`, `structured_clone.rs:471-474`) or on its own type's prototype. So
  the branch is dead code and is deleted (CLAUDE.md "dead code は接続するか削除"); the PR body states this
  argument instead of a kill.
- **M13** (OD-1 (b) only) `clone_error` keeps `src_proto`. **M14** (OD-2 (b) only) `internalize` keeps the
  `Ordinary | Arguments` arm.

The payload removal has no mutant: the compiler checks every pattern site, and no site reads the value
(§1.2's table is the evidence, re-derived by §6's command).

## §5. Non-goals, hand-offs, sibling defects

- **E2** (umbrella §4): own NativeError prototypes and EvalError (axis A), the prototype each construction
  site chooses, the constructors' `[[Prototype]]` = %Error% (AggregateError and the Wasm classes included,
  P-4), `Error.prototype.constructor`, and dropping the instance-level own `.name` once the prototypes carry
  it.
- **SR-1 — clone fidelity beyond the prototype** (HTML §2.7.4 step 18 and §2.7.7 step 22): today
  `clone_error` copies every own data property (`name`, `cause`, `errors`, script-added ones;
  `structured_clone.rs:663-690`), where the spec carries `message` only (and an implementation-defined
  stack), and does not normalise the name. Prototype choice is OD-1; the rest has no owner. **Reported, not
  homed** (proposed slot `#11-structured-clone-error-record-fidelity`).
- **SR-2 — DOMException's `[[ErrorData]]`** (§1.5). Reported; a further axis, not taken.
- **SD-1 — `InstallErrorCause` missing** (ECMA-262 §20.5.9.1, step 4 of every constructor in §3):
  `error_ctor_impl` ignores `options` (`vm/natives.rs:444-480`), so `new Error('m', { cause: 1 }).cause`
  is `undefined`. Silent-wrong. Proposed slot `#11-vm-error-install-error-cause`.
- **SD-2 — `Error.isError` missing** (ECMA-262 §20.5.2.1; `git grep -n 'isError' 145927c9 -- crates/script/elidex-js/src`
  → no output). Loud. After E1 its body is the kind check plus SR-2's answer. Proposed slot
  `#11-vm-error-iserror`.
- **SD-3 — AggregateError's step order**: elidex walks the `errors` iterable (`vm/natives.rs:613-621`)
  before ToString(message), where §20.5.7.1.1 does step 3 (message) before step 5 (IteratorToList);
  observable through side effects. Silent-wrong. Proposed slot `#11-vm-aggregate-error-step-order`.
- **SD-4 — DOMException structured clone**: DOMException is `[Serializable]` (Web IDL §4.4), but elidex's
  `classify` sends its `Ordinary` to `clone_ordinary`, which copies enumerable own properties (none; the
  state is in `dom_exception_states`) onto `Object.prototype` — read, not probed. Silent-wrong; unchanged by
  E1. Proposed slot `#11-domexception-serializable`.
- **SD-5 — `do_new`'s GetPrototypeFromConstructor fallback** (ECMA-262 §10.1.14): when `ctor.prototype` is
  not an Object, or is an accessor, `do_new` uses `None` (`vm/ops.rs:714-717` keeps only a
  `PropertyResult::Data(JsValue::Object)`), where the spec reads with `? Get` (getters run) and falls back to
  the realm's intrinsicDefaultProto. Engine-wide, not error-specific; E2 meets it when it gives each error
  constructor its own intrinsic default. Proposed slot `#11-vm-get-prototype-from-constructor-fallback`.
- **SuppressedError** (§20.5.8) is not implemented; when it lands it is a §1.1 creator. Note only.
- **Hygiene note, no slot**: the promotion mechanism has two spellings (§1.2). E1 uses the named one.

## §6. Re-derivation

Run at `145927c9` (each command is the one that produced the cite; line numbers move, the commands do not).
After the E1-pre rebase the `vm/host/structured_clone.rs` cites move into its split files; re-derive them with
the same commands over `vm/host/structured_clone/`.

```sh
B=145927c9; S=crates/script/elidex-js/src
git grep -n 'fn register_error_constructors\|let ctors: &\|for &(name, func) in ctors\|let agg_proto = ' $B -- $S/vm/globals_errors.rs
git grep -n 'fn error_ctor_impl\|fn error_entry\|fn native_aggregate_error_constructor\|fn native_wasm_\|fn native_error_to_string' $B -- $S/vm/natives.rs
git grep -n 'fn ensure_instance_or_alloc\|fn promote_to\|pub(crate) fn create_date' $B -- $S/vm/inner.rs
git grep -n 'fn vm_error_to_thrown\|kind: ObjectKind::Error' $B -- $S/vm/ops.rs
git grep -n 'ObjectKind::Error' $B -- crates/
git grep -n 'error_prototype' $B -- $S ':!crates/script/elidex-js/src/vm/tests'
git grep -n 'fn classify\|fn clone_error\|fn is_error_like_proto\|CloneKind::Ordinary if' $B -- $S/vm/host/structured_clone.rs
git grep -n 'fn reject_non_json_storable\|fn is_error_like\|if is_error_like\|clone_value' $B -- $S/vm/host/indexeddb/value.rs
git grep -n 'fn build_aggregate_error' $B -- $S/vm/natives_promise_combinator.rs
git grep -n 'fn wasm_error_to_js_value\|let instance = ctx.vm.alloc_object' $B -- $S/vm/host/wasm/errors.rs
git grep -n 'ObjectKind::Ordinary' $B -- $S ':!crates/script/elidex-js/src/vm/tests' | /usr/bin/grep -v 'kind: ObjectKind::Ordinary'
git grep -n '\.kind = ObjectKind::' $B -- $S ':!crates/script/elidex-js/src/vm/tests'
git grep -n 'fn internalize\|_ => {}' $B -- $S/vm/natives_json/parse.rs
git grep -n 'NB: `new Error' $B -- $S/vm/tests/tests_object_constructor.rs
git grep -n 'isError\|map_constructor' $B -- $S
.claude/tools/webref heading ecma262 20.5
.claude/tools/webref aoid ecma262 OrdinaryCreateFromConstructor
.claude/tools/webref aoid ecma262 PerformPromiseAny
.claude/tools/webref aoid ecma262 InternalizeJSONProperty
.claude/tools/webref body ecma262 sec-object.prototype.tostring
.claude/tools/webref body html structuredserializeinternal
.claude/tools/webref body html structureddeserialize
.claude/tools/webref body webidl js-DOMException-specialness
.claude/tools/webref body wasm-js-api-2 error-objects
.claude/tools/webref body IndexedDB-3 clone
```

Probes (scratchpad, VM P4 session `0fe71d24`): `isctor/impl/e1-probe-test.rs` + `e1-probe.log`,
`isctor/impl/m10-probe-test.rs` + `m10-probe3.log`, both at `145927c9`. Every row not marked *probe* in §4.1
is read from the code, not run.
