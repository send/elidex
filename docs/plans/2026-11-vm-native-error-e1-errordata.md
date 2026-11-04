# PR-E1 — `[[ErrorData]]` on every error object, one way to recognise an error (per-PR plan-memo)

**Slot**: `#11-vm-native-error-prototypes` (registered 2026-11-05, PM 363; re-scoped PM 364), axis C. E1 also
closes `#11-error-object-builtintag-dual-representation` (registered at #474, the candidate slot that
`vm/tests/tests_object_constructor.rs:105-112` names): that slot is axis C as seen from
Object.prototype.toString alone. **Owner**: VM P4. **Umbrella**: `docs/plans/2026-11-vm-native-error-program.md`
(this commit); program E1 → E2, E2 gets its own plan and its own `/elidex-plan-review`. **Baseline**:
origin/main `f9cf8c40` (Rev 1-2: `145927c9`; Rev 3 re-ran every §6 command at `f9cf8c40`: every hit count
is unchanged, and the only moved cites are `vm/natives.rs`, +1 line from `:4` on, and one comment moved from
`vm/globals.rs` to `vm/globals_fundamentals.rs` by #592). Every `file:line` is at `f9cf8c40`; §6 re-derives
each one. The structured-clone sites are cited by function, "(after Slice F)" — VM P4 Slice F's row names
no post-split file paths, so this memo names none — never by line. Paths are relative to
`crates/script/elidex-js/src/` unless they start with `crates/` or `docs/`. Spec numbers come from
`.claude/tools/webref` output only (§3 states the command per row).

**Rev 2** (2026-11-05) — PM 365 folded in: E0 and E1-pre are standalone prereqs; Rev 1's OD-1 is closed into
E1 (§1.6); Rev 1's OD-2 is E0; the proposed slots are registered. Nothing else in the design changes.
**Rev 3** (2026-11-05) — plan-review R1, PM 370 rulings: E1-pre is VM P4 Slice F; `promote_to_error` is
fail-safe (promotes only an `Ordinary` receiver; §1.2), with cells for the two non-`Ordinary` paths and the
receiver-admission class left to its registered slot; the step-18.1/18.2 name read is stated as B-clone's
(§2); the mid-walk staleness is the record-fidelity slot's, pinned by a cell (§1.6, §4.1); clerical fixes;
rebased onto `f9cf8c40`.

**Prereqs (umbrella §1, order E0 → F → E1, PM 365 / PM 370)**: **E0** (`#11-vm-json-internalize-non-array-objects`,
registered) — JSON.parse's reviver walk keyed on `ObjectKind::Ordinary` (§0.3 P-5; umbrella §1.1).
**Slice F** (VM P4, `#11-vm-structured-clone-file-split`) — the split of `vm/host/structured_clone.rs` (1077
lines at `f9cf8c40`, touched here; CLAUDE.md "1000-line debt"), per F's row in
`docs/plans/2026-07-vm-p4-slice-plan.md` §5, with F's seam (the clone algorithm vs `// Global registration`
plus the WebIDL binding) and F's own process (umbrella §1.2). Every structured-clone site E1 touches
(`clone_recursive`, `classify`, `clone_ordinary`, `clone_error`, `is_error_like_proto`) is in F's
clone-algorithm half. E1 rebases onto both.

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
- **The clone's prototype (PM 365)**: `clone_error` picks the clone's prototype from the serialized name (HTML
  §2.7.4 StructuredSerializeInternal steps 18.1-18.2, §2.7.7 StructuredDeserialize steps 22.1-22.7), through
  `error_intrinsic_prototype(name)` (NEW), never from the source (§1.6).

**Not in scope** (each stated as a defect with an owner in §5):

- Own NativeError prototypes, `EvalError`, constructor `[[Prototype]]` = %Error%, every other construction
  site that chooses a prototype (`vm_error_to_thrown`'s `error_prototype`, call-mode `error_entry`), and the
  per-name entries of `error_intrinsic_prototype`'s table: **E2**. The only prototype E1 changes is a clone's
  (§1.6), and through a table that returns `error_prototype` for every name until E2.
- The own `.name` property that every elidex error constructor and the throw path define on the
  **instance** (`vm/natives.rs:456-462` in `error_ctor_impl`, `vm/ops.rs:209-214`). The spec defines `name`
  on the prototypes only (§20.5.3.3, §20.5.6.3.3). It is a workaround for the shared prototype; whether and how
  it is removed is E2's decision, stated once in umbrella §4. E1 keeps it.
- `InstallErrorCause`, `Error.isError`, AggregateError's step order, DOMException's `[[ErrorData]]` and its
  serialization, and the rest of HTML step 18 / step 22 fidelity (§5).

### §0.2 Premises re-measured

| Premise (brief) | At `f9cf8c40` (Rev 1-2: `145927c9`; same result) | Evidence |
|---|---|---|
| `register_error_constructors` gives the six constructors one `.prototype` | holds (`vm/globals_errors.rs:66-89`; docstring `:24-32` admits it) | read; probe `m10-probe3.log` "proto identity matrix: 11111 ×5" |
| User errors are `ObjectKind::Ordinary`; toString → `"[object Object]"` for `new Error`, `new TypeError`, `TypeError('x')`, `new RangeError`, `new AggregateError`, `class E extends TypeError` | holds | probe `e1-probe.log` rows "toString user errors", "toString subclass" |
| Engine-thrown errors are `ObjectKind::Error { name }`, toString → `"[object Error]"` | holds (`vm/ops.rs:171`, alloc `:201-206`) | probe "toString thrown" |
| `natives_symbol.rs` ~`:284` keys on the kind | holds (`native_object_prototype_to_string`, `vm/natives_symbol.rs:240`, arm `:284`) | read |
| `do_new` / `construct_synchronous` pre-allocate an `Ordinary` from newTarget.prototype, and `ensure_instance_or_alloc` returns `this` in construct mode | holds (`vm/ops.rs:722-729`; `vm/inner.rs:152`, `:158-161`); the super path passes the derived frame's `this_value`, which is `do_new`'s instance (`vm/dispatch_class.rs:138-148`) | read |
| Classifiers by prototype identity at `structured_clone.rs` ~`:124` and ~`:699` (`is_error_proto`) and `indexeddb/value.rs` ~`:365` | holds, with the names corrected (§0.3 P-1): the guard arm in `clone_recursive`, predicate `is_error_like_proto` (both after Slice F); `is_error_like` `vm/host/indexeddb/value.rs:363-371` | `git grep -n 'error_prototype' f9cf8c40 -- crates/script/elidex-js/src` |
| `structuredClone(Object.create(Error.prototype))` clones as an Error | holds: `instanceof Error` true; the clone's kind is `Ordinary` (toString `"[object Object]"`), because `clone_error` copies the source kind (its `match` on the source's `kind`) | probe "structuredClone fake error" → `cloned:[object Object]:true` |

### §0.3 Premise errors

- **P-1 — names.** The structured-clone predicate is `is_error_like_proto` (after Slice F),
  not `is_error_proto`. The IndexedDB predicate is `is_error_like` (`vm/host/indexeddb/value.rs:363`). The
  two are **different predicates**: the first compares the immediate prototype for equality, the second
  walks the chain. That is already two ways (§1.3).
- **P-2 — the creation list is incomplete.** The brief names the Error / NativeError / AggregateError
  constructors. The census by concept (§1.1) finds three more creators of objects the spec gives
  `[[ErrorData]]`: the three `WebAssembly.*Error` constructors, which share `error_ctor_impl`
  (`vm/natives.rs:554`, `:569`, `:584`); the Wasm bridge's thrown errors (`wasm_error_to_js_value`,
  `vm/host/wasm/errors.rs:179`, alloc `:201-206`); and `Promise.any`'s rejection (`build_aggregate_error`,
  `vm/natives_promise_combinator.rs:258`, alloc `:259-264`). Wasm JS API §5.10 Error Objects creates each
  Wasm error class as "a new object, implementing the NativeError Object Structure"
  (`.claude/tools/webref body wasm-js-api-2 error-objects`), so their instances have the slot.
  PerformPromiseAny (ECMA-262 §27.5.4.3.1) steps 5.b.ii.1 and 5.e.vii.1 reject with "a newly created
  AggregateError object". Leaving any of them `Ordinary` keeps two representations, so E1 includes all
  three (§1.1). **This is a census correction, not a further axis**: same invariant, same mechanism. Flagged
  for the driver because it reaches the Wasm surface the brief asked about (§0.4).
- **P-3 — "Map / Set / … use OrdinaryCreateFromConstructor with a slot list".** Map, Set and WeakMap do not
  exist in the VM (`git grep -n 'map_constructor' f9cf8c40 -- crates/script/elidex-js/src` → no output). The
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
  the prereq **E0** (`#11-vm-json-internalize-non-array-objects`, registered; PM 365 decision 1; umbrella
  §1.1). The census that found it is §1.4.
- **P-6 — the narrow E1 changes two spec-correct observables in structured clone.** Because E1 routes every
  `[[ErrorData]]` object to `clone_error`, and `clone_error` keeps the **source** prototype
  (`clone_error` allocates on `src_proto.or(vm.error_prototype)`), two rows that clone as `{}` today
  would clone as errors with the source's prototype: `structuredClone(new (class E extends TypeError {})())
  instanceof E` and `structuredClone(new WebAssembly.CompileError()) instanceof WebAssembly.CompileError`
  go from `false` (spec-correct, by accident of the `{}`) to `true` (spec: `false`). HTML StructuredDeserialize
  (§2.7.7) step 22 chooses the prototype **by the serialized name**, from the seven names step 18.2 keeps,
  never from the source. The rows are read, not probed. **Decided by PM 365 (decision 3)**: E1 takes the
  by-name rule (§1.6), so both rows stay `false`.

### §0.4 Decisions

Rev 1's open decisions, closed by PM 365 (one line each, where it went):

- **OD-1 (the clone's prototype rule, P-6)** → closed: E1 picks the clone's prototype by serialized name
  through `error_intrinsic_prototype(name)` (§1.6). This is the content Rev 1 listed as option (b).
- **OD-2 (E0, P-5)** → closed: a standalone prereq PR, program row E0 (umbrella §1, §1.1), slot
  `#11-vm-json-internalize-non-array-objects` (registered). Not folded into E1.

Still standing (a designer decision, not a PM one):

- **OD-3 — Wasm in E1 (P-2).** The census puts the Wasm constructors and the Wasm bridge in E1. The
  alternative — leaving both Wasm paths `Ordinary` — keeps a second representation for objects that have the
  slot. **Designer decision: include**; listed so the driver can overrule it.

## §1. Design

**Layering.** Structured clone, the IndexedDB value check and the Wasm error bridge are algorithms over the VM
object graph (`&mut VmInner`, `ObjectKind`, prototypes) with no engine-independent counterpart to route
through (the layering question of the clone algorithm itself is `#11-vm-structured-clone-layering-seam`'s);
E1 changes only how they classify and allocate, and adds no algorithm under `vm/host/`.

### §1.1 Creation census

Found by concept (who allocates an error instance), not by the brief's list:
`git grep -n 'error_prototype' f9cf8c40 -- crates/script/elidex-js/src ':!crates/script/elidex-js/src/vm/tests'`,
`git grep -n 'ObjectKind::Error' f9cf8c40 -- crates/`,
`git grep -n 'error_ctor_impl\|wasm_error_to_js_value\|build_aggregate_error' f9cf8c40 -- crates/script/elidex-js/src`.

| Creator | Site | Kind today | E1 |
|---|---|---|---|
| `Error`, `TypeError`, `ReferenceError`, `RangeError`, `SyntaxError`, `URIError` — construct and call mode | `error_entry` `vm/natives.rs:488` → `ensure_instance_or_alloc` `:494-496` | `Ordinary` | `error_receiver` (NEW) |
| subclass `class E extends TypeError` — `super(...)` reaches the native with the derived frame's `this` | `vm/dispatch_class.rs:138-148` → `construct_synchronous` → `error_entry` | `Ordinary` (E.prototype preserved) | same path, promoted in place, prototype untouched |
| `AggregateError` — construct and call mode | `vm/natives.rs:602`, receiver `:638-640` | `Ordinary` | `error_receiver` (NEW) |
| `WebAssembly.CompileError` / `LinkError` / `RuntimeError` | `vm/natives.rs:554`, `:569`, `:584` (engine-gated) | `Ordinary` | `error_receiver` (NEW) |
| engine throw — `VmError` → JS value | `vm_error_to_thrown` `vm/ops.rs:171`, alloc `:201-206` | `Error { name }` | `create_error_object` (NEW); payload dropped |
| `Promise.any` rejection | `build_aggregate_error` `vm/natives_promise_combinator.rs:258`, alloc `:259-264` | `Ordinary` | `create_error_object` (NEW) |
| Wasm bridge — engine-thrown Wasm errors | `wasm_error_to_js_value` `vm/host/wasm/errors.rs:179`, alloc `:201-206` | `Ordinary` | `create_error_object` (NEW) |
| structured clone of an error | `clone_error` (after Slice F), its `match` on the source kind | copies the source kind, on the source prototype | always `create_error_object` (NEW) on `error_intrinsic_prototype(name)` (§1.6); the `Ordinary` arm goes (unreachable after §1.3) |

**Not creators of error instances** (checked, no change): `Error.prototype`, `AggregateError.prototype` and
the Wasm prototypes are ordinary per spec (`vm/globals_errors.rs:34`, `:93`; `vm/host/wasm/errors.rs:103`);
DOMException (`vm/host/dom_exception.rs:215`, `:309`) is a platform object, §1.5.

### §1.2 Mechanism — how `[[ErrorData]]` reaches a pre-allocated receiver

**What the existing built-ins with internal slots do.** `do_new` allocates the receiver `Ordinary` from
`ctor.prototype` before the native body runs (`vm/ops.rs:722-729`), and every native constructor with a slot
**promotes that receiver's kind in place**, leaving its prototype alone (so a subclass keeps
newTarget.prototype). Census: `git grep -n '\.kind = ObjectKind::' f9cf8c40 -- crates/script/elidex-js/src ':!crates/script/elidex-js/src/vm/tests'`
→ 41 hits, five of them inside `vm/inner.rs`'s helpers. One mechanism, two spellings:

- **Named `VmInner` helpers**, the ES-core built-ins: `promote_to_string_wrapper` (`vm/inner.rs:213`),
  `promote_to_number_wrapper` (`:234`), `promote_to_date` (`:252`), `promote_to_boolean_wrapper` (`:267`),
  `promote_to_array` (`:274`), each beside a fresh-allocation sibling (`create_date` `:240`,
  `create_number_wrapper`, …) for the paths that have no receiver. Callers: `natives_number.rs:47`,
  `natives_boolean.rs:38`, `natives_string.rs:759`, `natives_date/mod.rs:140`, `natives.rs:715`.
- **Inline `get_object_mut(id).kind = …`**, in the WebIDL constructors under `vm/host/` (ArrayBuffer
  `vm/host/array_buffer.rs:507`, Blob, DataView, the observers, the Wasm objects, …) and the Promise
  constructor (`vm/natives_promise.rs:731`).

Not two mechanisms: both promote the same `do_new` receiver in place. **Canonical form for E1 = the named
helper**, because the Error family is ES core and its nearest relative is `Date` (`create_date` +
`promote_to_date`). The inline spelling is a spelling, not a second mechanism; E1 does not touch it (§5,
hygiene note).

**The design:**

- `VmInner::promote_to_error(obj_id)` (NEW, `vm/inner.rs`, beside `promote_to_date`): sets
  `ObjectKind::Error` **only if the receiver's kind is `Ordinary`**; any other kind is left as it is (no
  overwrite, no assert). Fail-safe direction (PM 370): an unknown receiver falls to "not promoted", which is
  today's behaviour, never to "kind overwritten". Rev 2 asserted `Ordinary` on the premise that no other
  receiver reaches an error constructor; that premise is false, by two paths (both read, not probed):
  - **(a) custom-element upgrade.** The class chain is validated only at `define`, so after
    `Object.setPrototypeOf(X, TypeError)` the upgrade's `construct_synchronous` call
    (`vm/host/custom_elements/upgrade.rs:318-326`, `this` = the element wrapper) reaches `error_entry` through
    `super()` with the element's `HostObject` wrapper as the receiver. Overwriting that kind would orphan the
    element's HostData identity. E1 keeps base behaviour: the wrapper gets `name` / `message` as today and is
    not promoted. Cell T-PROMO-UPGRADE.
  - **(b) a second `super()`** in `class E extends TypeError`: there is no BindThisValue check
    (`#11-vm-derived-constructor-this-binding`, whose spec answer is a ReferenceError at BindThisValue), so the
    receiver is the already-promoted `Error`. E1 keeps base behaviour: the existing Error is re-initialised.
    Cell T-PROMO-SUPER2.

  The class — every in-place promotion site (`promote_to_date` / `_number_wrapper` / `_string_wrapper` /
  `_boolean_wrapper` / `_array`, the inline host promotions, and `promote_to_error`) reached with a receiver
  that is not a `do_new`-pre-allocated `Ordinary` — is pre-existing and is
  `#11-vm-native-ctor-receiver-promotion-admission` (registered, PM 370). E1 does not take it (§2).
- `VmInner::create_error_object(prototype)` (NEW, `vm/inner.rs`, beside `create_date`): allocates an
  `ObjectKind::Error` — the four engine creators of §1.1 (`vm_error_to_thrown`, `build_aggregate_error`,
  `wasm_error_to_js_value`, `clone_error`).
- `error_receiver(ctx, this, prototype) -> ObjectId` (NEW, `vm/natives.rs`): `ensure_instance_or_alloc`
  then `promote_to_error`. The one receiver step for all ten constructors (`error_entry` ×6, AggregateError,
  the three Wasm constructors). `error_ctor_impl` stays the init step its docstring says it is
  (`vm/natives.rs:439-444`). In call mode `ensure_instance_or_alloc` allocates a fresh object with the
  prototype the caller passed, which is ECMA-262's "If NewTarget is undefined, let newTarget be the active
  function object" (§20.5.1.1 step 1, §20.5.6.1.1 step 1, §20.5.7.1.1 step 1) as far as E1 can follow it;
  which prototype that is per constructor is E2's.

**Order against the spec.** OrdinaryCreateFromConstructor (§10.1.13) with « [[ErrorData]] » is step 2 of each
constructor, before step 3's `? ToString(message)`. `error_receiver` runs before `error_ctor_impl`, so the
promotion precedes the ToString. Not observable either way (the receiver is unreachable from the message's
`toString`), but the code follows the step order. AggregateError's receiver comes after its iterator walk
today (`vm/natives.rs:614-622`, receiver `:638-640`); that order is SD-3 (§5), unchanged here.

**Subclass.** `class E extends TypeError {}`: `new E('m')` → `do_new` allocates an `Ordinary` with
`E.prototype` → the derived constructor's `super(...)` passes that object as `receiver`
(`vm/dispatch_class.rs:138-148`) → `error_entry` → `error_receiver` returns it and promotes it. The kind
changes; `E.prototype` stays. T-SUB-PROTO pins both halves.

**The payload (question 2).** `ObjectKind::Error { name: StringId }` (`vm/object_kind.rs:135`). Every site
that names the variant, by `git grep -n 'ObjectKind::Error' f9cf8c40 -- crates/`:

| Site | Reads `name`? |
|---|---|
| `vm/ops.rs:202` (write, the throw path) | writes it |
| `clone_error` (after Slice F) | reads it, only to copy it into the clone's kind |
| `classify` (after Slice F) | no (`{ .. }`) |
| `vm/host/indexeddb/value.rs:298` | no (`{ .. }`) |
| `vm/natives_symbol.rs:284` | no (`{ .. }`) |
| `vm/gc/trace.rs:556` | no (`{ .. }`; a `StringId`, no GC edge) |

No site reads the value for behaviour: the only reader copies it. The spec slot carries no value (§20.5.4:
"whose value is undefined"). Giving constructed errors the payload would mean choosing a value (the
constructor's name? the subclass's? what `.name` says after a script writes it?), and any choice is a second
source of truth beside the `name` property, which is what `Error.prototype.toString` (§20.5.3.4) and HTML
step 18.1 actually read. **Decision: the variant becomes payload-free `ObjectKind::Error`**, its six sites
respelled. It is E1's (the kind is the recognition token). The own `.name` **property** on instances is
the E2 half of the same duplication (§0.1; E2 decides, umbrella §4), stated, not fixed here.

### §1.3 Recognition census (question 3)

By concept — anything that decides "is this an error" by prototype identity, by an `error_prototype` /
`aggregate_error_prototype` comparison, or by the kind. Commands: the two in §1.1, plus
`git grep -n -i 'error_like\|is_error' f9cf8c40 -- crates/script/elidex-js/src ':!crates/script/elidex-js/src/vm/tests'`.

| Site | Today | E1 |
|---|---|---|
| Object.prototype.toString — `native_object_prototype_to_string` `vm/natives_symbol.rs:240`, arm `:284` (§20.1.3.6 step 8) | the kind | unchanged (respelled); now true for every constructed error |
| structured clone — the `CloneKind::Ordinary if is_error_like_proto` guard arm in `clone_recursive` + `is_error_like_proto` (after Slice F) | the kind (`classify`'s `ObjectKind::Error` arm) **or** an `Ordinary` whose immediate prototype `==` `error_prototype` / `aggregate_error_prototype` | the kind only (§2.7.4 step 18); guard and predicate deleted |
| structured clone — `clone_error`'s kind copy | copies `Error { name }` or `Ordinary` | always the kind (§1.1) |
| IndexedDB — `reject_non_json_storable` `vm/host/indexeddb/value.rs:242`: arm `:298` + `Ordinary` branch `:318-323` → `is_error_like` `:363-371` | the kind **or** an `Ordinary` whose chain reaches either prototype | the kind only; the `Ordinary` error branch and `is_error_like` deleted (question 4) |

No other site compares against either prototype: the remaining `error_prototype` hits are allocations
(§1.1), registration (`vm/globals_errors.rs`, `vm/host/wasm/errors.rs:55-86`, `vm/host/dom_exception.rs:252`)
and GC roots (`vm/gc/collect.rs:74-75`). The remaining `is_error` hit is an unrelated local
(`vm/host/indexeddb/request.rs:131`). `Error.prototype.toString` (`vm/natives.rs:402`) checks only "is an
Object", as §20.5.3.4 step 2 does.

**HTML structured serialize.** WHATWG HTML §2.7.4 StructuredSerializeInternal step 18 is the Error branch,
keyed on "has an [[ErrorData]] internal slot and value is not a platform object"
(`.claude/tools/webref body html structuredserializeinternal`). Anything else that is not a platform object
falls to the ordinary-object branch. So the fake `Object.create(Error.prototype)` is an ordinary object (its
clone is `{}` on `Object.prototype`), and the subclass instance and the Wasm errors are errors.

### §1.4 Consumers of the kind change (the sweep that found P-5)

E1 moves constructed errors out of `ObjectKind::Ordinary`. Every site whose behaviour depends on a value
being `Ordinary` sees them move:
`git grep -n 'ObjectKind::Ordinary' f9cf8c40 -- crates/script/elidex-js/src ':!crates/script/elidex-js/src/vm/tests' | /usr/bin/grep -v 'kind: ObjectKind::Ordinary'`
→ 10 hits. Five are comments (`vm/globals_fundamentals.rs:262`, moved from `vm/globals.rs:1371` by #592; `vm/host/dom_exception.rs:7`, `vm/host/dom_rect.rs:8`, the guard comment in `clone_recursive`, `vm/host/wasm/namespace.rs:41`). `vm/gc/trace.rs:553` shares `Error`'s arm (no change). `classify`'s
`Ordinary` arm, `clone_error`'s `Ordinary` arm and `vm/host/indexeddb/value.rs:295` are §1.3's. **`vm/natives_json/parse.rs:444` is P-5** (E0, landed before E1).
Kind-agnostic paths (property storage, `JSON.stringify`, which branches only on Array / Date / wrappers at
`vm/natives_json/stringify.rs:63`, `:131`, `:470`, `:547-550`; the JSON shortcut behind worker `postMessage` and
`history.state`, `vm/host/structured_serialize.rs`) do not branch on `Ordinary`, so an `Error` object takes
the same path there as today's constructed error.

**Consumers of `clone_value`** (`git grep -n 'clone_value(' f9cf8c40 -- crates/script/elidex-js/src ':!crates/script/elidex-js/src/vm/tests'`
→ the definition and three callers): `structuredClone` (`native_structured_clone`), IndexedDB
(`value_to_json`, `vm/host/indexeddb/value.rs:382`) and **`window.postMessage`**
(`native_window_post_message`, `vm/host/pending_tasks.rs:592`). All three see §1.3's classifier change, §1.6's
prototype rule and §1.6's `name` getter run identically, since they share the one function.

### §1.5 DOMException and the Wasm errors (question 3)

- **Wasm errors have `[[ErrorData]]`** (Wasm JS API §5.10, quoted in §0.3 P-2). E1 changes their recognition:
  both Wasm creators are in §1.1 (OD-3). After E1, `Object.prototype.toString.call(new
  WebAssembly.CompileError())` is `"[object Error]"`; structured clone routes them to `clone_error`, whose
  prototype is `error_intrinsic_prototype("Error")` (their names are not among step 18.2's seven, §1.6); IndexedDB rejects them like every other error (`DataCloneError`, the v1 JSON-storage
  stance of `#11-idb-structured-clone-storage`) instead of silently storing `{}`.
- **DOMException has `[[ErrorData]]`** — a *reading* of Web IDL §3.14.1 DOMException custom bindings
  (`.claude/tools/webref body webidl js-DOMException-specialness`), whose sentence "It also has [[ErrorData]]
  and [[Stack]] slots, like all built-in exceptions" has as its literal subject **the interface prototype
  object** for DOMException (the preceding sentence's subject), not DOMException instances. That instances
  carry the slot is the reading; E1 changes nothing that depends on it, and the predicate's owner
  (`#11-vm-error-iserror`) decides it. HTML step 18's "and value is not a
  platform object" exists for exactly that case: a DOMException serializes by its own [Serializable] steps
  (Web IDL §4.4 DOMException), never by step 18. **E1 does not change DOMException's recognition**: it stays
  `Ordinary` with its state in `dom_exception_states` (`vm/host/dom_exception.rs:224`, `:318`), so no E1
  classifier sees it as an error, which is what step 18 requires. Its slot has no consumer in elidex today
  (the only spec consumers are toString step 8, which its `@@toStringTag` overrides, and `Error.isError`, which
  elidex lacks). Modelling it — a "has `[[ErrorData]]`" predicate that admits a platform object, kept apart
  from step 18's "and not a platform object" — **is a further axis, not taken**. Rev 1 reported it (SR-2);
  it is chartered in `#11-vm-error-iserror` (PM 370), its consumer (SD-2, §5);
  `#11-domexception-serializable` owns DOMException's clone (SD-4), not SR-2.

### §1.6 The clone's prototype, by serialized name (PM 365, Rev 1's OD-1)

**Why it is E1's.** §1.3 routes every `[[ErrorData]]` object to `clone_error`, and `clone_error` keeps the
**source** prototype (`src_proto.or(vm.error_prototype)`). So without this section E1 turns two spec-`false`
rows true (§0.3 P-6). HTML decides the clone's prototype from the serialized name and from nothing else:

- StructuredSerializeInternal (§2.7.4) step 18.1 `Let name be ? Get(value, "name")`; step 18.2 "If name is
  not one of "Error", "EvalError", "RangeError", "ReferenceError", "SyntaxError", "TypeError", or "URIError",
  then set name to "Error"" (`.claude/tools/webref body html structuredserializeinternal`).
- StructuredDeserialize (§2.7.7) step 22 (`serialized.[[Type]]` is "Error"): 22.1 prototype = %Error.prototype%;
  22.2-22.7 one step per remaining name; 22.9 `OrdinaryObjectCreate(prototype, « [[ErrorData]], [[Stack]] »)`
  (`.claude/tools/webref body html structureddeserialize`).

**The design.**

- `ErrorName` (NEW, `vm/globals_errors.rs`): a closed enum of the seven names, which are exactly the seven
  intrinsics step 22.1-22.7 can name (%Error% and the six NativeErrors), so the enum sits with the intrinsics'
  owner. No existing type has this set: `VmErrorKind` (`vm/error.rs`) has no `Error` / `EvalError` and carries
  non-error variants.
- `serialized_error_name(vm, name: JsValue) -> ErrorName` (NEW, the clone side: `clone_error`'s half, after
  Slice F) implements step 18.2: a `JsValue::String` whose content is one of the seven gives that name; any
  other string and any non-String value give `ErrorName::Error` (step 18.2 compares the value, it does not
  `ToString` it). **Placement (designer call, Rev 3; Rev 1-2 had it as an associated function of
  `ErrorName` in `vm/globals_errors.rs`)**: the comparison and its fallback to "Error" are a StructuredSerializeInternal step,
  not a property of the intrinsic set, so they sit with the serialization they belong to; `globals_errors.rs`
  keeps only the set and the table.
- `VmInner::error_intrinsic_prototype(name: ErrorName) -> Option<ObjectId>` (NEW, `vm/globals_errors.rs`,
  beside `register_error_constructors`, which owns the intrinsics): an exhaustive `match`, one arm per name
  (steps 22.1-22.7). **In E1 every arm returns `self.error_prototype`**, which is what %TypeError.prototype%
  and its siblings *are* in elidex until E2 (and `EvalError` has no intrinsic yet). E2 changes the arms'
  values and nothing else; the exhaustive `match` makes a forgotten name a compile error. **Realm**:
  StructuredDeserialize creates the value "in targetRealm", so the spec's %Error.prototype% here is
  targetRealm's; reading it from a `VmInner` field is the existing form for every intrinsic (one realm per
  `VmInner` today), and this method is one more site of the B1 multi-realm rewrite, not a new shape.
- `clone_error` (after Slice F): (1) `name = vm.get_property_value(src, "name")?`
  (`get_property_value`, `VmInner`'s method in `vm/shape_ops.rs`, is `Get` §7.3.2, `.claude/tools/webref aoid
  ecma262 Get`: an accessor's getter runs, an abrupt completion propagates out of `structuredClone`); (2)
  `serialized_error_name(vm, name)`; (3)
  `create_error_object(vm.error_intrinsic_prototype(name))` (step 22.9, §1.2); then the memo insertion and
  the own-property copy as today. The `src_proto` parameter goes, and so does its argument at the
  `clone_recursive` call site. The name read precedes the allocation, as step 18 precedes step 22.
- **Not here**: steps 18.3-18.4 (the message by `[[GetOwnProperty]]` + `ToString`), the record's other
  fields, and the own-property copy that carries `name`, `cause`, `errors` and script-added properties onto
  the clone — `#11-structured-clone-error-record-fidelity` (registered, PM 365; its decision surface is
  `#11-vm-structured-clone-layering-seam`'s forum, PM 370; §5). So the clone of a
  `new AggregateError([], 'm')` keeps its own `name` data property `"AggregateError"` while its prototype is
  %Error.prototype%; that mismatch is the record-fidelity slot's, not E1's.
- **The snapshot premise, and the staleness it leaves (record-fidelity's, PM 370).** `clone_ordinary`
  snapshots an object's `(key, value)` entries before recursing into any of them (its own comment: the
  recursion takes `&mut vm`), where StructuredSerializeInternal step 28.4 re-checks `! HasOwnProperty(value,
  key)` and reads `? value.[[Get]](key, value)` per key, after the previous key's serialization. Until E1 no
  script runs during a clone of an error, so the difference is unobservable through errors; E1's step-18.1
  `Get(name)` makes it observable: a `name` getter on an error that mutates (or deletes) a later key of an
  ancestor already snapshotted is not seen by the clone. E1 does not change the snapshot; T-CLONE-NAME-STALE
  pins today's behaviour as a known deviation owned by `#11-structured-clone-error-record-fidelity`.

**Observables E1 changes here** (all toward the spec): a subclass instance's clone is not `instanceof` the
subclass; a `WebAssembly.*Error`'s clone is not `instanceof` its class; an `AggregateError`'s clone is not
`instanceof AggregateError`; a `name` getter on the source runs once during the clone (and its throw
propagates). IndexedDB and `window.postMessage` clone through the same function (§1.4), so the getter also
runs on `store.add` / `put` of an error, before the v1 rejection — as StructuredSerializeForStorage requires —
and on `postMessage` of an error, whose received clone has the same kind and by-name prototype.

### §1.7 Touch-time line counts and the in-flight overlap

Sizes at `f9cf8c40` (`git show f9cf8c40:<path> | wc -l`, paths under `crates/script/elidex-js/src/`):
`vm/natives.rs` 879, `vm/ops.rs` 893, `vm/inner.rs` 315, `vm/globals_errors.rs` 132,
`vm/natives_promise_combinator.rs` 570, `vm/host/wasm/errors.rs` 223, `vm/host/indexeddb/value.rs` 464,
`vm/natives_symbol.rs` 434, `vm/shape_ops.rs` 619 (read only), `vm/tests/mod.rs` 203,
`vm/tests/tests_indexeddb_value.rs` 120, `vm/tests/tests_object_constructor.rs` 154; `vm/tests/tests_error_data.rs`
(NEW). `vm/host/structured_clone.rs` is 1077, and E1 edits it only after Slice F (§0 prereqs), in F's
clone-algorithm half (~914 lines per F's row), where E1's net change shrinks the half (the guard arm,
`is_error_like_proto` and `clone_error`'s `Ordinary` arm go; the name read and `serialized_error_name` come).
Over 1000 and touched, with their verdicts (per `docs/plans/2026-07-vm-p4-touch-time-split-record.md`,
`git grep -n 'Not on any watch list and >1000' -- docs/plans/2026-07-vm-p4-touch-time-split-record.md` →
`:82`, the assignments at `:83-84`):

- **`vm/object_kind.rs` (1645)**: the record assigns its split to **umbrellas 6, 7 and 10**, through the
  children each mints. E1's edit is the `Error { name: StringId }` → `Error` respelling of one variant
  (`vm/object_kind.rs:135`), which shrinks the file and grows no arm, so it complies and does not pre-empt that verdict.
- **`vm/gc/trace.rs` (1255)**: the record assigns it to **umbrellas 7a and 7b**. E1's edit is the
  `ObjectKind::Error { .. }` → `ObjectKind::Error` respelling in the arm it already shares with `Ordinary`
  (§1.2's table), net delta 0, no arm grown; it does not pre-empt that verdict.
- **`vm/tests/tests_custom_elements.rs` (1321)**: not touched; T-PROMO-UPGRADE's harness is written in the
  new file instead (§4.1).

**Overlap with in-flight 1aa** (`docs/plans/2026-11-vm-p4-slice-1a-1aa.md` on branch `vm-p4-slice-1a-plan`,
`git show vm-p4-slice-1a-plan:docs/plans/2026-11-vm-p4-slice-1a-1aa.md`): its change list (§1.7 there) edits
`op_super_call` / `op_super_call_spread` in `vm/dispatch_class.rs` and registers modules in
`vm/tests/mod.rs`, and leaves `[[Construct]]` (`construct_synchronous`, marked "unchanged" in its §3 rows) and
`vm/inner.rs` (read for premises only) untouched; E1 edits neither `dispatch_class.rs` nor
`construct_synchronous` (it only cites `dispatch_class.rs:138-148`), so the overlap is textual (`vm/tests/mod.rs`
and moved cites), not semantic; whichever lands second re-derives the cites.

## §2. Edge density, derived from the cell bodies

Criterion (the IsConstructor memo's §2, `docs/plans/2026-11-vm-isconstructor-check.md`): an axis is
evidenced only by a failure that occurs while the others pass. The base-tree evidence is from the probes
(*probe* rows of §4.1); the "fails alone" column is **derived from the §4.1 cell bodies**, not run — the
post-implementation mutant run records the measured result.

- **C1, creation** — every `[[ErrorData]]` creator makes the kind. Base evidence: the thrown-error toString
  cell passes while every constructed-error cell fails (probe `e1-probe.log`), with the classifier (toString's
  kind arm) correct. Mutants M1-M7, M9, M19.
- **C2, recognition** — every classifier keys on the kind. Evidence: with C1 fixed and the structured-clone
  guard kept (M8), T-CLONE-FAKE and T-IDB-FAKE fail while every T-TAG cell passes. Mutants M8, M10, M11.
- **B-clone, the clone's prototype by serialized name** (§1.6; axis B's clone site, moved from E2 by PM 365).
  Evidence: with C1 and C2 fixed and `clone_error` keeping `src_proto` (M13), the prototype rows of
  T-CLONE-SUB, T-CLONE-AGG and T-CLONE-WASM fail while every T-TAG, T-CLONE-KIND and T-CLONE-FAKE cell
  passes. Mutants M13, M15, M16. **B-clone includes the step-18.1 / 18.2 name read** (M15 `Get` vs own data
  property, M16 no `ToString`), PM 370: the name is the input of the by-name rule, so it is not a fourth axis.
- **The C1 × B-clone pair.** The two meet at one call inside `clone_error`: `create_error_object` (C1: the
  clone has the kind) on `error_intrinsic_prototype(name)` (B-clone: the prototype by name). Separated by
  M9 (allocates `Ordinary`, prototype rule kept → T-CLONE-KIND fails, the prototype rows pass) vs M13 (keeps
  `src_proto`, kind kept → the prototype rows fail, T-CLONE-KIND passes).
- **Receiver admission — a fourth axis, not absorbed** (PM 370). M18 (promote unconditionally) fails
  T-PROMO-UPGRADE while every other cell passes, so it is an axis of its own. E1 does not take it: E1's stance
  is fail-safe (`Ordinary` only, base behaviour on both non-`Ordinary` paths, §1.2), and T-PROMO-UPGRADE /
  T-PROMO-SUPER2 pin that stance; the class is `#11-vm-native-ctor-receiver-promotion-admission`'s.

| Mutant | Cells it fails (derived from the cell bodies) | Fails alone? (derived; the run records it) | Axis |
|---|---|---|---|
| M3 promotion keyed on the receiver's prototype being `error_prototype` / `aggregate_error_prototype` | T-TAG subclass rows, T-WASM constructor rows, T-CLONE-SUB, T-IDB-SUB | yes | C1 |
| M19 `vm_error_to_thrown` allocates `Ordinary` | T-TAG-CTL thrown row, T-CLONE-THROWN | yes | C1 (the throw path is a creator) |
| M8 structured clone keeps `CloneKind::Ordinary if is_error_like_proto` | T-CLONE-FAKE, T-IDB-FAKE | yes | C2 |
| M9 `clone_error` allocates `Ordinary` | T-CLONE-KIND; the existing `err:` row of `add_value_with_cloneable_non_json_type_throws_data_clone_error` (`vm/tests/tests_indexeddb_events.rs:560`), once IDB's prototype branch is gone | yes | C1 (`clone_error` is a creator) |
| M13 `clone_error` keeps `src_proto` | the prototype rows of T-CLONE-SUB, T-CLONE-AGG, T-CLONE-WASM | yes | B-clone |
| M18 `promote_to_error` promotes every receiver kind | T-PROMO-UPGRADE | yes | receiver admission (not E1's; pinned) |

The payload removal adds no axis: no site reads the value (§1.2), so no cell can fail on it. The IDB
prototype branch adds none either: it becomes unreachable (§4.2 M12).

**Three axes E1 takes (C1, C2, B-clone), and one it pins but does not take (receiver admission).** Rev 1's third candidate, the JSON consumer (P-5), is E0's, not
E1's (PM 365). The third axis here is the one PM 365 placed in E1; it is not a further axis beyond that
decision. E1 is a terminal unit: the base case
of CLAUDE.md "Edge-dense work" (c), a plan-reviewed, narrowly scoped slice under an approved umbrella
(`docs/plans/2026-11-vm-native-error-program.md`). And C1 and C2 are not separable: a PR that does C1 alone
makes the fake-error clone and the IDB rows depend on which predicate the clone takes, and a PR that does C2
alone makes every constructed error unclonable as an error (the kind it keys on does not exist yet) — the
second is a regression of `error_subclass_proto_preserved` (`vm/tests/tests_structured_clone.rs:171`). So C1 +
C2 is terminal by necessity. B-clone is not separable from C2 either: C2 without it turns the two P-6 rows
spec-wrong (and the AggregateError row stays so), and B-clone without C2 never sees the subclass and Wasm
instances, which still clone through `clone_ordinary`.

**Re-slice trigger**: the post-implementation run finds a mutant that fails alone and breaks an invariant
neither C1, C2 nor B-clone owns, other than receiver admission (M18, pinned and slotted above).

## §3. Spec coverage map

Section numbers from `.claude/tools/webref aoid ecma262 <AO>` / `.claude/tools/webref heading ecma262 20.5`
/ `.claude/tools/webref dfn html <AO>`; step text from `.claude/tools/webref body <spec> <anchor-or-AO>`.
Skeleton and breadth verdict: `.claude/tools/webref coverage-map ecma262 20.5.1.1 ecma262 20.5.6.1.1 ecma262 20.5.7.1.1 ecma262 20.1.3.6 ecma262 10.1.13 ecma262 PerformPromiseAny html 2.7.4 html 2.7.7`
→ (verdict 1, the requested skeleton) "spec=2 (ecma262, html), step=8 entries … ok → single PR scope"
(verified 2026-11-05, re-run at `f9cf8c40`). The Wasm JS API and
IndexedDB rows are labels the preflight does not resolve (it warns and skips them); their numbers come from
`.claude/tools/webref heading wasm-js-api-2 5.10` and `.claude/tools/webref dfn IndexedDB-3 clone`.
**Breadth.** Verdict 2, the plan-review preflight over this table:
`python3 .claude/skills/elidex-plan-review/preflight.py docs/plans/2026-11-vm-native-error-e1-errordata.md`
→ "unique specs (K): 5 (ecma262, html, webidl, <IndexedDB>, <Wasm JS API>)", "split decision:
SPLIT-RECOMMENDED (K>=4 or M>=20)". The two verdicts differ because verdict 1 counts only the requested
clauses and verdict 2 counts every row's spec. The three extra specs are not
extra subsystems: the IndexedDB row is the consumer of HTML's algorithm (it touches only the classifier the
HTML row already owns), the Wasm JS API row says which objects the ECMA-262 creation rows cover, and the Web
IDL row touches nothing (it records why DOMException is left alone). The split verdict is §2's, by measured
axes, not by spec count.

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| ECMA-262 §20.5.1.1 Error ( message [ , options ] ) | 1-2 | NewTarget undefined → active function; OrdinaryCreateFromConstructor with « [[ErrorData]] » | `error_entry` `vm/natives.rs:488` → `error_receiver` (NEW) | ✓ both modes | yes — any newTarget, any message |
| ECMA-262 §20.5.6.1.1 NativeError ( message [ , options ] ) | 1-2 | same, ×5 (EvalError absent: E2) | `vm/natives.rs:508-546` → `error_entry` | ✓ the five that exist | yes |
| ECMA-262 §20.5.7.1.1 AggregateError ( errors, message [ , options ] ) | 1-2 | same | `vm/natives.rs:602`, receiver `:638-640` → `error_receiver` (NEW) | ✓ | yes |
| ECMA-262 §10.1.13 OrdinaryCreateFromConstructor ( ctor, intrinsicDefaultProto [ , internalSlotsList ] ) | 2 (prototype, do_new's pre-allocation), 3-5 (slots, promote_to_error) | the slot list « [[ErrorData]] » on the object whose prototype came from newTarget (subclass included) | `do_new` pre-allocation `vm/ops.rs:722-729` (step 2) + `promote_to_error` (NEW; steps 3-5) | ✓ construct, call, super | yes — `class E extends TypeError` |
| ECMA-262 §20.5.4 Properties of Error Instances | — | instances have [[ErrorData]] (value undefined); the prototypes do not (§20.5.3) | `ObjectKind::Error` payload dropped, `vm/object_kind.rs:135` | ✓ | no |
| ECMA-262 §27.5.4.3.1 PerformPromiseAny ( iteratorRecord, ctor, resultCapability, promiseResolve ) | 5.b.ii.1, 5.e.vii.1 | "a newly created AggregateError object" | `build_aggregate_error` `vm/natives_promise_combinator.rs:258` → `create_error_object` (NEW) | ✓ both rejection points share the builder | yes — every input rejects |
| ECMA-262 §20.1.3.6 Object.prototype.toString ( ) | 8 | [[ErrorData]] → "Error" (step 15-16 tag override unchanged) | `vm/natives_symbol.rs:284` | ✓ | yes |
| WHATWG HTML §2.7.4 StructuredSerializeInternal ( value, forStorage [ , memory ] ) | 18, 18.1-18.2 | [[ErrorData]] and not a platform object → Error record; otherwise falls through to the ordinary-object branch. 18.1 `? Get(value, "name")`; 18.2 any value outside the seven names → "Error" (18.3-18.7: `#11-structured-clone-error-record-fidelity`) | `clone_recursive` guard arm deleted, `classify`; the name read and `serialized_error_name` (NEW) in `clone_error` (all after Slice F) | ✓ fake error, subclass, Wasm, AggregateError, thrown; the seven names, any other string, a non-String, a getter, a throwing getter | yes — `structuredClone(x)`, the source's `name` |
| WHATWG HTML §2.7.7 StructuredDeserialize ( serialized, targetRealm [ , memory ] ) | 22.1-22.7, 22.9 | prototype by serialized name (one step per name, %Error.prototype% otherwise); OrdinaryObjectCreate(prototype, « [[ErrorData]], [[Stack]] ») — the clone has the slot | `clone_error` → `error_intrinsic_prototype` (NEW, `vm/globals_errors.rs`; every arm `error_prototype` until E2) → `create_error_object` (NEW) | ✓ seven arms, exhaustive `match` | yes |
| WHATWG HTML §2.7.6 StructuredSerializeForStorage ( value ) | — | IndexedDB's clone serializes through the same algorithm; v1 then rejects every Error record (JSON storage) | `vm/host/indexeddb/value.rs:298` (kept), `:318-323` + `:363-371` (deleted) | ✓ | yes — `store.add(x)` |
| IndexedDB §5.11 Clone a value | 3-4 | StructuredSerializeForStorage then StructuredDeserialize — one classifier, structured clone's | `value_to_json` → `clone_value` `vm/host/indexeddb/value.rs:382` | ✓ | yes |
| Wasm JS API §5.10 Error Objects | 2.1 | each class implements the NativeError Object Structure → instances have [[ErrorData]] | `vm/natives.rs:554`, `:569`, `:584`; `vm/host/wasm/errors.rs:201-206` | ✓ three classes, both paths | yes |
| Web IDL §3.14.1 DOMException custom bindings | — | DOMException has [[ErrorData]] but is a platform object: never step 18 — **unchanged by E1** (§1.5, §5) | none | ✓ (no touch) | yes |

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
rest are read, not probed. T-PROMO-UPGRADE needs a bound document: it uses a local harness of the
shape `vm/tests/tests_custom_elements.rs`'s `build_doc` / `run` (`test_helpers::bind_vm`), written in
`tests_error_data.rs`, not added to that file (1321 lines at `f9cf8c40`; §1.7).

| Cell | Asserts | Base (`f9cf8c40`; *probe* values from `145927c9`) | Mutants it kills |
|---|---|---|---|
| T-TAG | `Object.prototype.toString.call(x)` is `"[object Error]"` for `new Error('x')`, `new TypeError('x')`, `new ReferenceError('x')`, `new RangeError('x')`, `new SyntaxError('x')`, `new URIError('x')`, `Error('x')`, `TypeError('x')`, `new AggregateError([], 'x')`, `AggregateError([])`, `new (class E extends TypeError {})('x')`, `new (class F extends Error {})()` | red for all (*probe* for five constructed, call-mode `TypeError`, the subclass) | M1, M2 (call rows), M3 (subclass rows), M4 (AggregateError rows), M11 |
| T-TAG-CTL | `Object.create(Error.prototype)` → `"[object Object]"`; `Error.prototype`, `TypeError.prototype`, `AggregateError.prototype` → `"[object Object]"`; a caught `null.x` → `"[object Error]"`; `e[Symbol.toStringTag] = 'X'` on a `new Error()` → `"[object X]"` | green (fake and thrown: *probe*); stays green | a promotion that over-reaches (prototypes); M19 (thrown row) |
| T-SUB-PROTO | `class E extends TypeError {}; const e = new E('m');` → `Object.getPrototypeOf(e) === E.prototype && e instanceof TypeError && e.message === 'm'` | green; stays green | a promotion that re-allocates or resets the prototype |
| T-ANY | `Promise.any([Promise.reject(1)]).catch(e => { globalThis.out = Object.prototype.toString.call(e); })` → `"[object Error]"` | red | M6 |
| T-WASM | toString of `new WebAssembly.CompileError('m')`, `new WebAssembly.LinkError('m')`, `new WebAssembly.RuntimeError('m')`, and of the error caught from `new WebAssembly.Module(new Uint8Array([0xff]).buffer)` (the bridge path, `tests_wasm_compile.rs:121-127`'s body) → `"[object Error]"` | red | M5 (constructor rows), M7 (bridge row), M3 |
| T-CLONE-KIND | `c = structuredClone(new TypeError('m'))` → `Object.prototype.toString.call(c) === '[object Error]'`, `c instanceof TypeError`, `c.name === 'TypeError'`, `c.message === 'm'` | red on the toString half (*probe* `true,true,TypeError,m,[object Object]`) | M9, M1 |
| T-CLONE-FAKE | `c = structuredClone(Object.create(Error.prototype))` → `c instanceof Error === false`, `Object.getPrototypeOf(c) === Object.prototype`, toString `"[object Object]"` | red (*probe* `cloned:[object Object]:true`) | M8 |
| T-CLONE-THROWN | a caught `null.x` cloned → `instanceof TypeError`, `name === 'TypeError'`, `"[object Error]"` | green (*probe*); stays green | M19 |
| T-CLONE-SUB | `class E extends TypeError {}; c = structuredClone(new E('m'))` → `"[object Error]"`, `c instanceof TypeError`, `c.message === 'm'`, `Object.getPrototypeOf(c) === TypeError.prototype`, and `c instanceof E === false` (§1.6) | red (base clones `{}`: `clone_ordinary` drops the non-enumerable `name` / `message`) | M1, M3, M13 |
| T-CLONE-WASM | `c = structuredClone(new WebAssembly.CompileError('m'))` → `c instanceof WebAssembly.CompileError === false`, `Object.getPrototypeOf(c) === Error.prototype`, `"[object Error]"`; the same for `LinkError` / `RuntimeError` | red on the prototype and toString halves (base: `{}` on `Object.prototype`); the `instanceof` half green by accident of the `{}` | M5, M13 |
| T-CLONE-AGG | `c = structuredClone(new AggregateError([], 'm'))` → `c instanceof AggregateError === false`, `Object.getPrototypeOf(c) === Error.prototype`, `c.message === 'm'`, `"[object Error]"` | red (base: `is_error_like_proto` admits `aggregate_error_prototype`, `clone_error` keeps it) | M4, M13 |
| T-CLONE-NAME-GET | `e = new TypeError('m'); let n = 0; Object.defineProperty(e, 'name', { get() { n++; return 'TypeError'; } }); structuredClone(e); n === 1`; and a getter that throws a sentinel makes `structuredClone(e)` throw that sentinel | red (base: `clone_error` skips accessors, no `Get`; `n === 0`, no throw) | M15 |
| T-CLONE-NAME-NONSTRING | `e = new Error('m'); e.name = Object.create({ toString() { throw 1; } }); c = structuredClone(e)` does not throw and `Object.getPrototypeOf(c) === Error.prototype` (the `toString` sits on the name object's prototype, so the own-property copy clones the name object as `{}` without meeting a function) | green (base copies data properties, no `ToString`); stays green | M16 |
| T-IDB-FAKE | `store.add({ wrapped: Object.create(Error.prototype) }, 1)` does not throw | red (base: `clone_error` keeps `error_prototype` and the `Ordinary` kind, then `is_error_like` rejects with `DataCloneError`) | M8 |
| T-IDB-SUB | `store.add({ wrapped: new (class extends TypeError {})('x') }, 1)` → `DataCloneError` | red (base: the clone is `{}`, stored) | M1, M3 |
| T-CLONE-NAME-STALE | `const e = new Error('m'); const o = { e, k: 1 }; Object.defineProperty(e, 'name', { get() { o.k = 2; return 'Error'; } }); c = structuredClone(o)` → `c.k === 1` — **pins a known deviation** (spec: `2`, step 28.4's per-key `[[Get]]`), owned by `#11-structured-clone-error-record-fidelity` (§1.6); the cell's comment names the slot and the spec answer | green (base: the getter does not run, `c.k === 1`); stays green after E1 (the getter runs, the snapshot keeps `1`) | — (pin) |
| T-PROMO-UPGRADE | path (a) of §1.2: `const el = document.createElement('x-e'); class X extends HTMLElement {} ; customElements.define('x-e', X); Object.setPrototypeOf(X, TypeError); customElements.upgrade(el);` → no throw, `el.tagName === 'X-E'` (a brand-checked `HostObject` accessor), and `el.name === 'TypeError'` (base behaviour: `error_ctor_impl` defines it on the wrapper) — pins E1's fail-safe stance; the cell's comment names `#11-vm-native-ctor-receiver-promotion-admission` | green (read, not probed); stays green | M18 |
| T-PROMO-SUPER2 | path (b) of §1.2: `class E extends TypeError { constructor() { super('a'); super('b'); } }; const e = new E();` → no throw, `e.message === 'b'`, `Object.prototype.toString.call(e) === '[object Error]'`, `e instanceof E` — pins base behaviour (the existing Error re-initialised) as a known deviation; spec: a ReferenceError at BindThisValue, `#11-vm-derived-constructor-this-binding`, named in the cell's comment | green on all but the toString half (red at base: `"[object Object]"`; the receiver is `Ordinary` until E1) | — (pin; a promotion that asserts `Ordinary`, Rev 2's design, panics here in debug builds) |

**Existing tests that change or must stay green:**

- E0's acceptance cells (umbrella §1.1; landed before E1): the reviver walks a constructed error's own
  enumerable properties. Green at E1's base (E0's walk is kind-independent); E1's DoD keeps them green.
- `clone_error_breaks_cycles_via_memo` and `clone_error_recurses_into_cause_value`
  (`vm/tests/tests_structured_clone.rs`): green, stay green (name `"TypeError"` / `"Error"` → `error_prototype`;
  the own-property copy is unchanged).

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
  `Object.prototype` (`clone_ordinary`, after Slice F) or on its own type's prototype. So
  the branch is dead code and is deleted (CLAUDE.md "dead code は接続するか削除"); the PR body states this
  argument instead of a kill.
- **M13** `clone_error` keeps `src_proto` (allocates on the source's prototype) → T-CLONE-SUB, T-CLONE-WASM,
  T-CLONE-AGG prototype rows.
- **M15** `clone_error` reads `name` as an own data property (no `Get`, a getter does not run) →
  T-CLONE-NAME-GET.
- **M16** `serialized_error_name` applies `ToString` to a non-String before the comparison →
  T-CLONE-NAME-NONSTRING.
- **M17 — equivalent in E1, no cell.** `serialized_error_name` returns `ErrorName::Error` (NEW) for every value (the
  name is ignored). In E1 every arm of `error_intrinsic_prototype` returns `error_prototype`, so no E1 cell
  can tell the names apart. E2's per-PR plan owns its kill, with the per-name cells that the own prototypes
  make observable (`structuredClone(new RangeError()) instanceof RangeError`, and a script-set
  `e.name = 'RangeError'` on a TypeError choosing %RangeError.prototype%). The PR body states this.
- **M18** `promote_to_error` promotes every receiver kind (the `Ordinary` test removed) → T-PROMO-UPGRADE
  (the wrapper's kind becomes `Error`, so its brand-checked `tagName` accessor throws). T-PROMO-SUPER2 does
  not kill it (overwriting `Error` with `Error` is a no-op), which is why the upgrade cell exists.
- **M19** `vm_error_to_thrown` allocates `Ordinary` (Rev 2 listed the throw path as a creator but gave it no
  mutant) → T-TAG-CTL's thrown row, T-CLONE-THROWN (the clone guard is gone, so an `Ordinary` thrown error
  clones as `{}`).
- Rev 1's M14 (`internalize` keeps the `Ordinary | Arguments` arm) is E0's mutant now (umbrella §1.1).

The payload removal has no mutant: the compiler checks every pattern site, and no site reads the value
(§1.2's table is the evidence, re-derived by §6's command).

## §5. Non-goals, hand-offs, sibling defects

- **E2** (umbrella §4): own NativeError prototypes and EvalError (axis A), the prototype each remaining
  construction site chooses, the per-name values of `error_intrinsic_prototype` (§1.6), the constructors' `[[Prototype]]` = %Error% (AggregateError and the Wasm classes included,
  P-4), `Error.prototype.constructor`, and the instance-level own `.name` (E2 decides, umbrella §4).
- **Clone fidelity beyond the prototype** (Rev 1's SR-1; HTML §2.7.4 steps 18.3-18.7 and §2.7.7 steps 22.8,
  22.10-22.13): `clone_error` copies every own data property (`name`, `cause`, `errors`, script-added ones;
  its own-property loop), where the spec carries `message` only (and an implementation-defined stack). The
  prototype half is E1's (§1.6); the rest → `#11-structured-clone-error-record-fidelity` (registered, PM 365),
  decided under `#11-vm-structured-clone-layering-seam`'s umbrella forum rather than a second one, and also
  owning the mid-walk `Get(name)` snapshot staleness (§1.6, T-CLONE-NAME-STALE; PM 370).
- **IndexedDB storage of errors**: after E1, IndexedDB rejects every `[[ErrorData]]` object, subclass and
  Wasm errors included, with `DataCloneError` (§1.5); storing them faithfully needs the storage-format
  redesign, which `#11-idb-structured-clone-storage` covers (extended to errors, PM 370).
- **DOMException's `[[ErrorData]]`** (Rev 1's SR-2; §1.5) → chartered in `#11-vm-error-iserror` (PM 370), its
  only consumer `Error.isError` (SD-2). Not taken by E1. (`#11-domexception-serializable` is SD-4's.)
- **Receiver admission of the in-place promotions** (§1.2, §2) → `#11-vm-native-ctor-receiver-promotion-admission`
  (registered, PM 370; pre-existing class, all promote sites); path (b)'s spec answer →
  `#11-vm-derived-constructor-this-binding` (VM P4 ledger). E1 pins its fail-safe stance (T-PROMO-UPGRADE,
  T-PROMO-SUPER2) and takes neither.
- **SD-1 — `InstallErrorCause` missing** (ECMA-262 §20.5.9.1, step 4 of every constructor in §3):
  `error_ctor_impl` ignores `options` (`vm/natives.rs:445-481`), so `new Error('m', { cause: 1 }).cause`
  is `undefined`. Silent-wrong. Slot `#11-vm-error-install-error-cause` (registered, PM 365).
- **SD-2 — `Error.isError` missing** (ECMA-262 §20.5.2.1; `git grep -n 'isError' f9cf8c40 -- crates/script/elidex-js/src`
  → no output). Loud. After E1 its body is the kind check plus the DOMException answer above. Slot
  `#11-vm-error-iserror` (registered, PM 365).
- **SD-3 — AggregateError's step order**: elidex walks the `errors` iterable (`vm/natives.rs:614-622`)
  before ToString(message), where §20.5.7.1.1 does step 3 (message) before step 5 (IteratorToList);
  observable through side effects. Silent-wrong. Slot `#11-vm-aggregate-error-step-order` (registered, PM 365);
  this bullet is the slot's detail (PM 370).
- **SD-4 — DOMException structured clone**: DOMException is `[Serializable]` (Web IDL §4.4), but elidex's
  `classify` sends its `Ordinary` to `clone_ordinary`, which copies enumerable own properties (none; the
  state is in `dom_exception_states`) onto `Object.prototype` — read, not probed. Silent-wrong; unchanged by
  E1. Slot `#11-domexception-serializable` (registered, PM 365; owner DOM).
- **SD-5 — `do_new`'s GetPrototypeFromConstructor fallback** (ECMA-262 §10.1.14): when `ctor.prototype` is
  not an Object, or is an accessor, `do_new` uses `None` (`vm/ops.rs:714-717` keeps only a
  `PropertyResult::Data(JsValue::Object)`), where the spec reads with `? Get` (getters run) and falls back to
  the realm's intrinsicDefaultProto. Engine-wide, not error-specific; E2 meets it when it gives each error
  constructor its own intrinsic default. Slot `#11-vm-get-prototype-from-constructor-fallback` (registered,
  PM 365).
- **SuppressedError** (§20.5.8) is not implemented; when it lands it is a §1.1 creator. The obligation to create
  it through `error_receiver` / `ObjectKind::Error` is `#11-vm-explicit-resource-management`'s (VM P4 Slice R,
  which owns the surface SuppressedError belongs to).
- **Hygiene note, no slot**: the promotion mechanism has two spellings (§1.2). E1 uses the named one.

## §6. Re-derivation

Run at `f9cf8c40` (each command is the one that produced the cite; line numbers move, the commands do not).
The structured-clone functions are cited by name; at `f9cf8c40` they are all in `vm/host/structured_clone.rs`
(the first structured-clone command); after Slice F they are wherever F's clone-algorithm half puts them (the
second command, run on E1's base, searches `vm/host/` rather than naming a path F's row does not name).

```sh
B=f9cf8c40; S=crates/script/elidex-js/src
git grep -n 'fn register_error_constructors\|let ctors: &\|for &(name, func) in ctors\|let agg_proto = ' $B -- $S/vm/globals_errors.rs
git grep -n 'fn error_ctor_impl\|fn error_entry\|fn native_aggregate_error_constructor\|fn native_wasm_\|fn native_error_to_string' $B -- $S/vm/natives.rs
git grep -n 'fn ensure_instance_or_alloc\|fn promote_to\|pub(crate) fn create_date' $B -- $S/vm/inner.rs
git grep -n 'fn vm_error_to_thrown\|kind: ObjectKind::Error' $B -- $S/vm/ops.rs
git grep -n 'ObjectKind::Error' $B -- crates/
git grep -n 'error_prototype' $B -- $S ':!crates/script/elidex-js/src/vm/tests'
git grep -n 'fn clone_recursive\|fn classify\|fn clone_error\|fn clone_ordinary\|fn is_error_like_proto\|CloneKind::Ordinary if' $B -- $S/vm/host/structured_clone.rs
git grep -n 'fn clone_recursive\|fn classify\|fn clone_error\|fn clone_ordinary\|fn is_error_like_proto\|CloneKind::Ordinary if' HEAD -- $S/vm/host/   # on E1's base, after Slice F
git grep -n 'fn get_property_value' $B -- $S/vm/shape_ops.rs
git grep -n 'fn reject_non_json_storable\|fn is_error_like\|if is_error_like\|clone_value' $B -- $S/vm/host/indexeddb/value.rs
git grep -n 'fn build_aggregate_error' $B -- $S/vm/natives_promise_combinator.rs
git grep -n 'fn wasm_error_to_js_value\|let instance = ctx.vm.alloc_object' $B -- $S/vm/host/wasm/errors.rs
git grep -n 'ObjectKind::Ordinary' $B -- $S ':!crates/script/elidex-js/src/vm/tests' | /usr/bin/grep -v 'kind: ObjectKind::Ordinary'
git grep -n '\.kind = ObjectKind::' $B -- $S ':!crates/script/elidex-js/src/vm/tests'
git grep -n 'fn internalize\|_ => {}' $B -- $S/vm/natives_json/parse.rs
git grep -n 'NB: `new Error' $B -- $S/vm/tests/tests_object_constructor.rs
git grep -n 'isError\|map_constructor' $B -- $S
git grep -n 'clone_value(' $B -- $S ':!crates/script/elidex-js/src/vm/tests'
git grep -n 'construct_synchronous(' $B -- $S/vm/host/custom_elements/upgrade.rs
for f in vm/natives.rs vm/ops.rs vm/inner.rs vm/object_kind.rs vm/gc/trace.rs vm/host/structured_clone.rs; do git show $B:$S/$f | wc -l; done
.claude/tools/webref heading ecma262 20.5
.claude/tools/webref aoid ecma262 OrdinaryCreateFromConstructor
.claude/tools/webref aoid ecma262 Get
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
