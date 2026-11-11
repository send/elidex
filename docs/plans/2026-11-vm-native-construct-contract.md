# Native construct protocol (R) — umbrella

Status: **plan (pre-implementation), first draft**, for `/elidex-plan-review` (edge-dense: §2). Owner: VM P4
lane. The R-NAT host chapter (§4.4) is L3's. The per-PR plans (R-JS; R-NAT core / ECMA-262 chapter; R-NAT
host chapter) are written after this umbrella is ratified and each goes through `/elidex-plan-review`
before code.

**Base.** Code anchors are at `origin/main` = `4b723d4b` unless a row says otherwise. Facts about the
in-flight value.rs split (#625, branch `vm-value-frame-split`) are at its head `ab71cb43`.

**Provenance root.** `H` = `/Users/kazuaki/repos/send.sh/elidex-pm-trial/handoff/native-construct-umbrella`
(outside the repo, durable): `H/scripts/` holds the census scripts, `H/logs/` their outputs at `4b723d4b`.
Inputs read as claims: VM P4's assessment `…/handoff/vm-p4-pra1/native-construct-assessment.md`, L3's survey
`…/handoff/vm-p4-pra1/l3-R-survey/` (`brandcheck-R-survey.md`, `census2.py`, `callees.py`, `fam.txt`), and
the PR-0A review `…/handoff/vm-p4-pra1/pr0a-review-063344da.md`. Every number below names its command (§10).

## §0. Charter

Today a native constructor does not create its object. `do_new` allocates an `Ordinary` from
`ctor.prototype` before any callee runs, for JS callees and natives alike (`vm/ops.rs:713-729`), and passes
it as `this`; a native then rewrites that object's kind in place (41 `.kind =` lines) or adopts it
(`ensure_instance_or_alloc`, `vm/inner.rs:152`). A derived class's `this` is that same pre-allocated object,
forwarded into `super()` as a receiver (`dispatch_super_inner`, `vm/dispatch_class.rs:140-158`). The
custom-element upgrade passes the element's wrapper as the receiver (`invoke_upgrade`, the
`construct_synchronous` call). The defects that follow are recorded in the slots of §5: a pre-`super()`
`this` read succeeds, a second `super()` re-initialises a live object (a debug panic for the Error family),
an upgrade can promote an element's wrapper into another kind, and an accessor or non-Object `prototype`
is mishandled.

The specification has one shape, and it has no receiver anywhere:

- ECMA-262 §10.2.2 `[[Construct]]` creates `thisArg` only for a base constructor (step 3.a,
  OrdinaryCreateFromConstructor); a derived constructor's `this` is unbound until SuperCall step 9
  BindThisValue, after step 6 `? Construct(superCtor, argList, newTarget)`.
- ECMA-262 §10.3.2 (built-in `[[Construct]]`) calls BuiltinCallOrConstruct with `thisArg` = *uninitialized*;
  every built-in constructor creates its own object (OrdinaryCreateFromConstructor or an XCreate AO) at its
  own step.
- Web IDL *create an interface object* step 8 internally creates the object from NewTarget, then step 9
  runs the constructor steps "with object as this".
- HTML *upgrade an element* step 11.3 constructs C with no arguments and no receiver; step 11.4 is the
  SameValue test. *HTML element constructors* return the construction-stack element (steps 12-16).
- Proxy `[[Construct]]` and `Reflect.construct` forward `Construct(target, argList, newTarget)`.

The program makes elidex's construct protocol that shape, in two PRs cut by callee kind (§4), so that the
object's kind (and, for a host object, its brand) is written once, at allocation.

### §0.1 Ratified rulings (decided; not re-opened here)

- **PM 675.** Direction (R) as above; PR-0A (L3's receiver admission) withdrawn; order **R-JS first, then
  R-NAT**; the cut is by callee kind; R-NAT lands in one go as a joint PR (VM P4 core + ECMA-262, L3 host);
  never per-native migration, never PR-0A landed and then deleted; edge-dense, so umbrella + per-PR plans,
  each plan-reviewed. How R-NAT is assembled is this plan's to propose (§4.5).
- **PM 676.** The value.rs re-export shims become a slot (`#11-vm-value-reexport-shim-retirement`, PM
  ledger); this umbrella proposes where its PR lands (§9.2).
- **PM 642 (a)-(c), PM 654 E-9.** A plan touching `ObjectKind::HostObject` or `call_dispatch` sends its
  hunks to the PM at plan time; `interpreter_call.rs` hunks are written per function (per arm for
  `call_dispatch`); whichever of two overlapping PRs lands later rebases (§8.3).

## §1. Scope

### §1.1 The population (one definition)

**Definition (by property).** The population P is **every native function object that has a
`[[Construct]]` internal method** — in elidex, every `ObjectKind::NativeFunction` whose `CallShape` is
`Ordinary` or `ConstructorOnly`, the two shapes `classify_construct_target` admits
(`vm/construct_admission.rs:84`) — counted by distinct `func`. This is exactly the set whose construct goes
through the native arm of `[[Construct]]` (§10.3.2), i.e. the set whose `this` R-NAT changes to
*uninitialized*. Membership does not depend on how a body treats `this` today.

**Measured: P = 98** (`python3 -I H/scripts/ctor_census.py <repo> 4b723d4b`, log
`H/logs/ctor_census-4b723d4b.log`):

| Part | Count | How |
|---|---|---|
| `fn native_*_(constructor\|ctor)` definitions | 89 | regex over non-test sources |
| − installed `CallableOnly` (no `[[Construct]]`): `Symbol`, `BigInt` | −2 | `create_native_function` at `vm/globals_primitives.rs:235`, `:308` |
| = installed constructable | 87 | 51 at a literal install site, 36 through a parameter site (the error table `vm/globals_errors.rs:66-76`, `register_event_subclass` `vm/host/events_extras.rs:264`, `register_descendant` `vm/host/events_ui.rs:552`, `wire_rect_constructor` `vm/host/dom_rect.rs:647`, the Wasm error installer `vm/host/wasm/errors.rs:127`) |
| + `typed_array_ctor_wrapper!` constructors | +11 | `vm/host/typed_array_install.rs:537-547`, installed at `:357` |
| **P** | **98** | |

**P_create = 97**: every member except `%TypedArray%` (`native_abstract_typed_array_ctor`,
`vm/host/typed_array_install.rs:559`), which always throws (§23.2.1.1 step 1).

**Chapters.** ECMA-262: **28** — `Object`, `Array`, `String`, `Number`, `Boolean`, `Date`, `Error` and the
five NativeErrors present, `AggregateError`, `Promise` (14, ungated), plus `ArrayBuffer`, `DataView`,
`%TypedArray%` and the 11 TypedArrays (14, engine-gated: `vm/host/mod.rs:74-75`, `:101-102`, `:342-349`).
Host: **70** — 62 Web IDL constructors and the 8 WebAssembly JS API ones.

**Reconciliation with the two input counts** (both claims, both re-run at `4b723d4b`):

| Input | Its count | Its definition | Sites not in P | Sites of P it misses | → P |
|---|---|---|---|---|---|
| VM P4 assessment §1.2-1.3 | 98 | ctor names + install sites | none | none | 98 |
| L3 survey §2 (`census2.py` + 11 macros) | ~101 | ctor-shaped name, takes `NativeContext`, referenced | `native_symbol_constructor`, `native_bigint_constructor` (CallableOnly), `require_subclass_ctor` (`vm/host/typed_array_static.rs:129`, a helper, not a function object) | none | 101 − 3 = 98 |

`census2.py` re-run on a flat copy of the sources at `4b723d4b` gives the same 90 rows as `fam.txt`
(`H/logs/l3-census2-4b723d4b.log`); its 90 = the 89 names + `require_subclass_ctor`. L3's families map onto
P as: F1 32 (of which `ArrayBuffer`, `DataView`, `Promise` are ECMA-262), F1b 5, typed arrays 11, F2 43 (of
which `Object`, `Error` ×6, `AggregateError` are ECMA-262), F3 2, F5 8 − 3 = 5 (`%TypedArray%`,
`HTMLElement`, `ImageData`, the two queuing strategies). Host 70 = F1 29 + F2 35 + F3 2 + F5 4.

**The "77".** L3's earlier figure is the members whose creation reaches one of the 41 `.kind =` lines
(`git grep -n -P '\.kind\s*=[^=]' 4b723d4b -- crates/script/elidex-js/src ':!*tests*'` → 41). It is a
symptom subset, P − 21, where the 21 are `Object`, `Error` ×6, `AggregateError`, `%TypedArray%`,
`HTMLElement`, `DOMException`, `DOMRect`, `DOMRectReadOnly`, the 3 Wasm errors, `ImageData`, the two
queuing strategies, `DataTransfer` and `Touch` (each read at its site; derived, not re-measured per
member). It is not the program's population.

### §1.2 In

The construct entries and the native arm (`do_new`, `construct_synchronous`, `dispatch_super_inner`,
`call_construct_native`, `invoke_upgrade`'s call), the frame's this-binding state, `Op::PushThis`, arrow
`captured_this`, frame completion, the GC frame roots, one creation helper module (NEW), all 97 creating
members, the deletion of `ensure_instance_or_alloc` / `promote_to_*` / `CallFrame::new_instance`, the
trip-wire extension, and the cells of §2.

| Item (`4b723d4b`) | What changes | PR |
|---|---|---|
| `VmInner::do_new` (`vm/ops.rs:681`; prototype read `:713-717`, allocation `:722-730`, JS path `:732-752`, native path `:753-776`) | becomes `EvaluateNew` → the single `[[Construct]]` entry; no pre-allocation for derived JS callees; base JS allocates callee-side | R-JS |
| `construct_synchronous` (`vm/dispatch_class.rs:273`; `receiver` / `pre_alloc_instance` parameters, substitution `:378-389`) | loses both parameters; becomes a caller of the single entry | R-JS |
| `dispatch_super_inner` (`vm/dispatch_class.rs:116`; receiver read `:140-141`, rebind `:152-159`) | SuperCall: Construct (step 6), then BindThisValue (step 9) | R-JS |
| `CallFrame::new_instance` (`vm/value.rs:1014`; `vm/frame.rs` after #625) and its 24 `git grep` hits | replaced by the this-binding state (§4.2) | R-JS |
| `Op::PushThis` (`vm/dispatch.rs:738-741`) | GetThisBinding (ReferenceError while uninitialized) | R-JS |
| `create_closure` arrow `captured_this` (`vm/ops.rs:832`) and its readers (`vm/dispatch_ic.rs:176-306`, `vm/ic.rs:59`, `vm/ops_property.rs:290`, `vm/interpreter_call.rs:39`, `vm/gc/trace.rs:255`) | an arrow reads the enclosing this-binding, not a snapshot (E-4) | R-JS |
| `complete_inline_frame` (`vm/dispatch.rs:1087`) and `construct_synchronous`'s tail (`:378-389`) | one §10.2.2 steps 12-17 implementation | R-JS |
| `call_construct_native` (`vm/interpreter.rs:95`) | its `this` parameter goes | R-JS |
| `ensure_instance_or_alloc` (`vm/inner.rs:152`, 14 calls) and `promote_to_*` (`vm/inner.rs:213-277`, 6 calls) | deleted; Date's `store` setter (`vm/natives_date/mod.rs:115`) becomes a payload write | R-NAT |
| the 97 creating members | create through the helper at their creation step | R-NAT |
| `invoke_upgrade`'s pre-construct `prototype` read (`vm/host/custom_elements/upgrade.rs:216-222`) | removed (§5.3 S-2) | R-JS |
| `resolve_validated_prototype` (`vm/host/custom_elements/html_element.rs:276`) | folded onto the helper's phase 1 (§5.3 S-1) | R-NAT (host) |

**Unchanged (re-verified):** `with_call_mode` (`vm/interpreter.rs:160`); `NativeContext::new_target` /
`is_construct` (`vm/native_context.rs:152` / `:138`); `NativeContext::new_construct` (`vm/value.rs:825`);
`CallMode` (`vm/value.rs:648-657`); the `NativeFunction` signature; `call_dispatch`'s arms
(`vm/interpreter_call.rs:17`): the entry passes the native's `this`, so no arm changes (§8.3).

### §1.3 Out

| Item | Owner |
|---|---|
| Absent constructors (`Function`, `RegExp`, `Map` / `Set` / `WeakMap` / `WeakSet`, `WeakRef`, `FinalizationRegistry`, `Proxy`, `Reflect`, `EvalError`, `SharedArrayBuffer`, `Float16Array`) | their slots (§5.2); each is born on (R) |
| The error type of upgrade step 11.4 (NotSupportedError today) | `#11-ce-upgrade-returned-object-typeerror` (L3) |
| Per-NativeError prototypes | `#11-vm-native-error-prototypes` (E1 / E2) |
| Instance fields (SuperCall step 12, §10.2.2 step 6.b) | Slice 2ab of `#11-step9-class-extras` (R-JS leaves the seam, E-10) |
| Lexical `super` / `new.target` through arrows | `#11-vm-arrow-lexical-super` (S), `#11-vm-arrow-lexical-new-target` (N); R-JS decides the representation with them (E-4) |

### §1.4 Premise corrections

| # | Premise (source) | Correction (measured at `4b723d4b`) |
|---|---|---|
| P-1 | "77 constructors" (L3 brand-check memo; PM 664's "39") | P = 98, P_create = 97 (§1.1); 77 is the `.kind =` symptom subset |
| P-2 | `do_new` pre-allocates for native callees | it pre-allocates for **every** callee: the instance (`vm/ops.rs:724`) goes to JS frames too (`:742-751`, `this` and `new_instance`), base and derived |
| P-3 | NewTarget must be plumbed to natives | already present: `call_dispatch` builds `NativeContext::new_construct(self, new_target)` (`vm/interpreter_call.rs:110-115`), read by `NativeContext::new_target` (`vm/native_context.rs:152`) |
| P-4 | only HTMLElement returns the construction-stack element | in HTML it is every interface with `[HTMLConstructor]` (`webref dfn html HTMLConstructor` → §3.2.3); elidex implements only `HTMLElement` because `define` rejects customized built-ins |
| P-5 | the brief list implies Map / Set / RegExp / Function / Proxy / EvalError constructors | none exists (§1.3) |
| P-6 | PR-0A's "wire #4 (NEW)" | `.claude/tools/native-ctor-guard-trip-wire.sh` already has wire #4 (`:86`, the "Illegal constructor" body throw); the new wire is #5 (§4.6) |
| P-7 | "`new_instance`: 20 hits" (assessment) | 24 (`git grep -n -P 'new_instance' 4b723d4b -- crates/script/elidex-js/src`), comments included |
| P-8 | "`ensure_instance_or_alloc`: 15 callers" (assessment; L3: "15 lines in 11 files") | 14 calls in 10 files plus the definition (15 lines, 11 files) |
| P-9 | the touched files over 1000 lines are `vm/mod.rs`, `gc/trace.rs`, `dispatch.rs`, `websocket.rs` | also `vm/host/events_modern/data_transfer.rs` (1226; `DataTransfer`'s constructor is in P) and, conditionally, `vm/object_kind.rs` (1645) (§7) |
| P-10 | "201 files import through `value::`" (PM 676) | 201 is the single-line spelling; the property ("names a re-exported type through a `value::` path", brace groups across lines included) is **309** (§9.2) |
| P-11 | E1 §1.2 and `construct_admission.rs` cite upgrade "step 10.3 / 10.4" | today's HTML numbers them 11.3 / 11.4 (`webref body html concept-upgrade-an-element`) |

## §2. Coupled invariants

Ten axes. Every pair is listed with what meets there; "—" means the pair shares no site or state (stated,
not omitted). The edges E-n are the cells' subjects (§2.2).

| Axis | Content |
|---|---|
| A1 entry | one `[[Construct]]` entry for `new`, SuperCall, the upgrade and every future caller (Reflect.construct, Proxy, TypedArrayCreate, Array.from) |
| A2 proto | GetPrototypeFromConstructor (§10.1.14) / Web IDL §3.8 steps 2-3: one resolver |
| A3 this | the this-binding: base bound at entry, derived uninitialized until `super()`, arrow reads the binding, native gets *uninitialized* |
| A4 return | §10.2.2 steps 12-17 for JS; §10.3.2 step 2 (an Object) for natives |
| A5 GC | rooting of the fresh object and of newTarget across creation |
| A6 CE | the construction stack, `HTMLElement`, upgrade step 11.4 |
| A7 stack | stack-safety's check sites (CP-1..CP-3b), PR-1's probes, the P0a-2a census rows |
| A8 gate | the feature rows (the `default` row of PR-A1) |
| A9 split | the 1000-line touch-time splits (§7) |
| A10 step | the creation step's position per constructor and the host's ECS side effects |

### §2.1 Pairs (45)

| Pair | Intersection |
|---|---|
| A1×A2 | the entry resolves the prototype only for a JS base callee (§10.2.2 step 3.a) and, in R-JS, for the centralised native pre-allocation; after R-NAT it resolves nothing for natives (E-1, E-11) |
| A1×A3 | the entry decides the initial binding: bound (base), uninitialized (derived), *uninitialized* passed to a native (R-NAT) or a fresh `Ordinary` (R-JS stage) |
| A1×A4 | one return step per callee kind, instead of today's two copies (`complete_inline_frame`, `construct_synchronous` tail) (E-3) |
| A1×A5 | base allocation happens callee-side after the prototype `[[Get]]`; the object is rooted by the frame's binding immediately (E-5) |
| A1×A6 | the upgrade calls the entry with no receiver; the SameValue at step 11.4 is unchanged (E-6) |
| A1×A7 | the native arm's allocation (R-JS) stays inside `with_call_mode`'s closure, after CP-1; `new` of a JS function stays inline (no new Rust recursion, so no new CP or probe) (E-9) |
| A1×A8 | the entry and the helper are core and ungated; both have `default`-row callers (JS base, the 14 ungated ECMA-262 members) |
| A1×A9 | the entry is a new core module: one `mod` line in `vm/mod.rs`; `complete_inline_frame` leaves `vm/dispatch.rs` (§7) |
| A1×A10 | the entry never runs a creation step for a native; in the R-JS stage the native arm's pre-allocation keeps today's position (stated, E-8) |
| A2×A3 | a derived constructor resolves no prototype (§10.2.2 step 3 is base-only); today `do_new` resolves one for it |
| A2×A4 | the prototype `[[Get]]` precedes the body, so a getter's effects stay observable when the body returns another Object |
| A2×A5 | the `[[Get]]` may run user code before allocation; only newTarget is live and it is rooted (value stack / caller frame) |
| A2×A6 | `resolve_validated_prototype` and the upgrade pre-read are two more resolvers; both fold or go (§5.3 S-1, S-2) |
| A2×A7 | the helper's fallible `? Get` is a new producer path under P0a-2a rule 1 (reach); every member's `?` on it is a carrier use (§9.1) |
| A2×A8 | the default prototype is an argument, so the core helper reads no engine-gated `VmInner` field |
| A2×A9 | — |
| A2×A10 | the `[[Get]]` happens at each constructor's creation step: Web IDL after overload resolution (step 7 → 8), Error before `ToString` (§20.5.1.1 step 2), Date after `ToNumber` (§21.4.2.1 step 6) (E-8) |
| A3×A4 | derived: `undefined` return → GetThisBinding (ReferenceError if `super()` never ran), other primitive → TypeError; base: `thisArg` (E-3) |
| A3×A5 | the binding's representation is traced wherever it lives (frame field, or a captured cell an arrow keeps alive) (E-5) |
| A3×A6 | `class C extends HTMLElement`: `super()` binds the element `HTMLElement` returns; the wrapper is no longer `this` before `super()` (E-6) |
| A3×A7 | GetThisBinding's ReferenceError is thrown in the run loop and caught by `handle_exception`; no CP, no Rust recursion |
| A3×A8 | — (core only) |
| A3×A9 | `vm/dispatch.rs` (`PushThis` arm, exempt match), `vm/frame.rs` (967 + 221 after #625, both under 1000), `vm/gc/trace.rs` (prereq split R-0b) |
| A3×A10 | SuperCall step 6 before step 9: a second `super()` runs the parent's creation and side effects (an ECS spawn included) before the ReferenceError (E-2) |
| A4×A5 | an object discarded by an Object return is garbage; nothing re-roots it |
| A4×A6 | `HTMLElement` returns the stack element (an Object) and it wins; step 11.4 compares that |
| A4×A7 | — |
| A4×A8 | — |
| A4×A9 | the unified return step lives in the entry module, not `vm/dispatch.rs` |
| A4×A10 | every native returns an Object in Construct mode (§10.3.2 step 2): a `debug_assert` at the entry's native arm |
| A5×A6 | the element wrapper stays rooted by the wrapper cache; it is no longer handed to a constructor |
| A5×A7 | CP-1 precedes every allocation the entry makes, so `rl_cp1_entry_failure_no_callee_effect` holds with no allocation (E-9) |
| A5×A8 | — |
| A5×A9 | the live-frame roots (`vm/gc/roots.rs:277-313`) and the suspended-frame roots (`vm/gc/trace.rs:335-372`) are two copies of one enumeration; R-JS edits both (R-0b) |
| A5×A10 | a host constructor spawns its entity before allocating `HostObject { entity, … }`; a throw between them must leave no orphan (E-7) |
| A6×A7 | P0a-2b reads `invoke_upgrade`'s `construct_synchronous` `Err` path (`p0a2b.md:175-176`, `:225`); R-JS changes the arguments, not the flow |
| A6×A8 | custom elements are engine-gated; the core entry has no CE knowledge |
| A6×A9 | `vm/tests/tests_custom_elements.rs` (1321): new cells go to a new file |
| A6×A10 | `HTMLElement`'s step 9.1 creates through the wrapper identity map (`create_element_wrapper`), not the helper's allocation; only phase 1 is shared |
| A7×A8 | CP-3a / CP-3b are engine-gated (Wasm); the entry's CP-1 is core |
| A7×A9 | `vm/interpreter.rs` (330) and `vm/interpreter_call.rs` (727): no split |
| A7×A10 | CP-3b is `instantiate_module`'s first statement (`vm/host/wasm/instance.rs:151`); R-NAT edits its `.kind =` (`:188`) and its receiver parameter |
| A8×A9 | — |
| A8×A10 | the step-position table (§4.3, §4.4) carries each member's feature row |
| A9×A10 | the `data_transfer.rs` split (R-0c) precedes R-NAT's host hunk there |

### §2.2 Edges (each maps to cells in its per-PR plan)

- **E-1 prototype resolution.** `function F(){}; F.prototype = 3; Object.getPrototypeOf(new F()) ===
  Object.prototype` (today `null`, because `do_new` falls back to `None`). Accessor case: unreachable for
  built-ins and classes (non-writable, non-configurable `prototype`); reachable once Reflect / Proxy exist,
  so the helper takes `? Get` from the start.
- **E-2 double `super()`.** `class E extends TypeError { constructor(){ super('a'); super('b') } }`: after
  R-JS the second call constructs a fresh object (no panic), then throws ReferenceError. The parent's
  effects precede the throw; a pre-Construct guard is wrong (slot `#11-vm-derived-constructor-this-binding`).
- **E-3 return.** The four regressions of that slot's row (pre-`super()` read, double `super()`, missing
  `super()`, `return 1` in a derived constructor) and its base controls (`return 1` in a base constructor
  yields the instance; `this.x = 1` works).
- **E-4 arrow before `super()`.** `constructor(){ const f = () => this; super(); return f() }` is conforming.
  Today it works only when the parent promotes the pre-allocated object; it fails already with
  `HTMLElement` (`new X()`), `DataTransfer` and `Touch`. R-JS makes the arrow read the binding. The
  representation must be reachable from an arrow and is decided **once**, with S and N (§4.2). This edge is
  why R-JS lands first (PM 675).
- **E-5 GC.** A forced collection between creation and the first use: base `this`, an arrow's captured
  binding after the constructor returns, a suspended generator frame (which never holds a construct state).
- **E-6 upgrade.** `Object.setPrototypeOf(C, Blob)` then upgrade: after R-JS the native arm gives Blob a
  fresh object, `super()` binds it, step 11.4 fails, the wrapper's kind and brand are untouched. Cell A-1
  (an ordinary `HTMLElement` upgrade) stays green. L3's A-2 … A-8 are re-expected under this edge.
- **E-7 host ECS order.** Worker (`vm/host/worker.rs:321`) and OffscreenCanvas
  (`vm/host/offscreen_canvas/mod.rs:346`): prototype `[[Get]]` → ECS side effects → allocation with the
  final `HostObject`. Web IDL allocates at step 8 and runs the side effects at step 9; elidex defers the
  unobservable MakeBasicObject past them so the kind is written once. A throw after the spawn and before
  the allocation is enumerated per member in §4.4.
- **E-8 step position.** The creation step differs per constructor (E-n of A2×A10). Unobservable until
  Proxy / Reflect.construct exist; the step-position tables (§4.3, §4.4) pin it so `#11-vm-proxy-reflect`
  inherits the right order. In the R-JS stage the native pre-allocation stays where `do_new` has it today
  (before argument conversion), a stated pre-existing deviation that R-NAT removes.
- **E-9 CP-1.** P0a-1's `rl_cp1_entry_failure_no_callee_effect` enters through `call_construct_native` and
  `construct_synchronous`; it stays green, with the stronger reading "no allocation either".
- **E-10 instance-field seam.** SuperCall step 12 / §10.2.2 step 6.b InitializeInstanceElements: R-JS puts
  one call point after BindThisValue (derived) and after OrdinaryCallBindThis (base), empty until Slice 2ab.
- **E-11 Call mode of dual-mode constructors.** `Date()`, `String(x)`, `Array(3)`, `Error('m')`,
  `Object()`: NewTarget undefined → the active function (each constructor's step 1); the helper's Call-mode
  branch uses the default prototype. L3's A-9 is reused.

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| ECMA-262 §13.3.5.1.1 EvaluateNew ( constructExpr, argumentsNode ) | 5-6 | IsConstructor, then Construct | `do_new` → the single entry (NEW) | ✓ | yes |
| ECMA-262 §7.3.14 Construct ( ctor [ , argList [ , newTarget ] ] ) | 1-2 | newTarget defaults to ctor | the single entry (NEW) | ✓ | yes |
| ECMA-262 §10.2.2 [[Construct]] ( argList, newTarget ) | 3, 6, 12-17 | base / derived; Object / undefined / other return | the entry's JS arm; `complete_inline_frame` | ✓ | yes |
| ECMA-262 §10.2.1.2 OrdinaryCallBindThis ( func, calleeContext, thisArg ) | 5-7 | base constructor binds at entry | the entry's JS base arm | ✓ | yes |
| ECMA-262 §10.3.2 [[Construct]] ( argList, newTarget ) | 1-2 | thisArg uninitialized; result is an Object | the entry's native arm | ✓ | yes |
| ECMA-262 §10.3.3 BuiltinCallOrConstruct ( func, thisArg, argList, newTarget ) | 11 | "If thisArg is uninitialized, the this value is uninitialized" | `call_construct_native`, the native arm | ✓ | yes |
| ECMA-262 §10.4.1.2 [[Construct]] ( argList, newTarget ) | 4-6 | bound args prepended; newTarget remapped when it is the bound function | `admit_construct` (unchanged) | ✓ | yes |
| ECMA-262 §10.1.14 GetPrototypeFromConstructor ( ctor, intrinsicDefaultProto ) | 2-6 | Object / non-Object (realm intrinsic) | helper phase 1 (NEW) | ✓ | yes |
| ECMA-262 §10.1.13 OrdinaryCreateFromConstructor ( ctor, intrinsicDefaultProto [ , internalSlotsList ] ) | 2-5 | — | helper phases 1-2 (NEW) | ✓ | yes |
| ECMA-262 §10.1.12 OrdinaryObjectCreate ( proto [ , additionalInternalSlotsList ] ) | 1-4 | — | helper phase 2 (NEW) | ✓ | no |
| ECMA-262 §7.3.1 MakeBasicObject ( internalSlotsList ) | 1-7 | the kind is the slot list, written once | helper phase 2 (NEW) | ✓ | no |
| ECMA-262 §13.3.7.1 Runtime Semantics: Evaluation | SuperCall 1-13 | step 6 Construct before step 9 BindThisValue; step 12 seam | `dispatch_super_inner` | ✓ | yes |
| ECMA-262 §9.1.1.3.1 BindThisValue ( envRecord, value ) | 2-4 | already initialized → ReferenceError | `dispatch_super_inner` | ✓ | yes |
| ECMA-262 §9.1.1.3.3 GetThisBinding ( ) | 2-3 | uninitialized → ReferenceError | `Op::PushThis`; the entry's derived return | ✓ | yes |
| ECMA-262 §9.4.3 GetThisEnvironment ( ) | 1-4 | arrows skip to the enclosing function environment | arrow `captured_this` readers (E-4) | ✓ | yes |
| ECMA-262 §7.3.33 InitializeInstanceElements ( obj, ctor ) | — | seam only (Slice 2ab) | the entry; `dispatch_super_inner` | ✗ | yes |
| ECMA-262 §10.5.13 [[Construct]] ( argList, newTarget ) | 7-11 | no receiver anywhere | future `#11-vm-proxy-reflect` caller of the entry | ✗ | yes |
| ECMA-262 §28.1.2 Reflect.construct ( target, args [ , newTarget ] ) | 1-5 | newTarget ≠ target | future caller of the entry | ✗ | yes |
| ECMA-262 §20.5.1.1 Error ( message [ , options ] ) | 1-2 | creation before ToString | `error_entry` | ✓ | yes |
| ECMA-262 §20.5.6.1.1 NativeError ( message [ , options ] ) | 1-2 | same | `error_entry` | ✓ | yes |
| ECMA-262 §21.4.2.1 Date ( ...values ) | 1, 6-7 | Call mode returns a string; creation after ToNumber | `native_date_constructor` | ✓ | yes |
| Web IDL §3.7.1 Interface object | 1.2, 7-9 | NewTarget undefined → TypeError (unchanged); create at step 8 after overload resolution at step 7 | the 62 Web IDL members | ✓ | yes |
| Web IDL §3.8 Platform objects implementing interfaces | internally create 2-9 | newTarget undefined / non-Object prototype → interface prototype; [[PrimaryInterface]] at step 8 | helper phases 1-2 (NEW) | ✓ | yes |
| Web IDL §3.12 Invoking callback functions | construct 1-11 | no receiver | `invoke_upgrade` → the entry | ✓ | yes |
| WHATWG HTML §3.2.3 HTML element constructors | 9-16 | empty construction stack creates from NewTarget; else returns the stack element; step 11 fallback | `native_html_element_ctor`, `resolve_validated_prototype` | ✓ | yes |
| WHATWG HTML §4.13.5 Upgrades | upgrade 11.3-11.4 | constructResult ≠ element → TypeError | `invoke_upgrade` | ✓ | yes |
| WHATWG HTML §4.13.4 The CustomElementRegistry interface | define: prototype Get / TypeError | the only `define`-time prototype check (S-2) | `invoke_upgrade` pre-read (removed) | ✓ | yes |

Numbers: `webref heading` / `aoid` at this draft (§10 M-6). ECMA-262 rows are keyed by AO name where one
exists. `✗` rows are seams for future callers, not this program's behaviour.

### §3.1 User-input touch audit

Every row marked "yes" is reachable from script: `new`, `super()`, a user getter on `prototype` (once
reachable, E-1), a custom-element definition and upgrade, and the constructor arguments. No row takes
input across a process boundary.

## §4. Per-PR split

### §4.1 Prerequisites (standalone, behaviour-neutral)

- **R-0a: the value.rs split** (#625, in flight; `vm/value.rs` 1176 → 967, `CallFrame` → `vm/frame.rs`).
  R-JS then edits `vm/frame.rs`.
- **R-0b: one GC frame-root enumeration** (NEW, VM P4). The live-frame block of `mark_roots`
  (`vm/gc/roots.rs:277-313`) and the suspended-frame block of `trace_work_list` (`vm/gc/trace.rs:335-372`)
  enumerate the same `CallFrame` fields (`this_value`, upvalues, `actual_args`, `new_instance`, `mode`,
  `home_class`, `pending_completion`); only `generator` differs. Extract one frame-root function used by
  both, before R-JS changes the field set. After #625 (it reads `vm/frame.rs`).
- **R-0c: split `vm/host/events_modern/data_transfer.rs`** (1226, L3) at its three interfaces:
  `DataTransfer` (`:1-803`), `DataTransferItem` (`:804-984`), `DataTransferItemList` (`:985-1226`) (§7).
- **R-0d: `vm/host/websocket.rs`** (1001): verdict in the host chapter (§7).

### §4.2 R-JS (VM P4): ECMAScript `[[Construct]]` and SuperCall

The derivation of `#11-vm-derived-constructor-this-binding` (an umbrella row with three axes:
representation, BindThisValue / GetThisBinding, return substitution). R-JS's per-PR plan is that
derivation; it may yield children along those axes, and each child is plan-reviewed.

- **One entry.** `Construct(ctor, args, newTarget)` after `admit_construct` (the one IsConstructor unit).
  `do_new`, `construct_synchronous` (SuperCall and the upgrade) and every future entry call it; the
  duplicated arms (bound splice, prototype read, Object-return-wins) collapse.
- **JS base.** OrdinaryCreateFromConstructor on the callee side through the helper's phase 1 (§10.2.2
  step 3.a), then OrdinaryCallBindThis.
- **JS derived.** `this` uninitialized. `Op::PushThis` = GetThisBinding. SuperCall = Construct, then
  BindThisValue (ReferenceError on the second), then the InitializeInstanceElements seam (E-10).
- **Return.** §10.2.2 steps 12-17, one implementation for the inline frame and the nested `run()`.
- **The this-binding representation** must be readable by an arrow created before `super()` (E-4) and
  must not be a by-value snapshot. The candidate the derivation weighs first is the existing upvalue
  machinery: the derived constructor's `this` as a binding slot with the `Op::CheckTdz` discipline,
  captured by arrows like any `let` (uninitialized = TDZ hole). The alternative is a frame status plus an
  environment link. The choice is made once, with S (`#11-vm-arrow-lexical-super`) and N
  (`#11-vm-arrow-lexical-new-target`), whose GetThisEnvironment reads the same thing.
- **Native arm: today's contract, centralised.** The entry's native arm allocates an `Ordinary` from
  GetPrototypeFromConstructor(newTarget) and passes it as `this` — what natives receive today, moved from
  `do_new` into one place, inside `with_call_mode`'s closure (E-9). Fallback: unreachable in this stage
  (newTarget for a native is a built-in or a class, both with an Object `prototype`); it uses the
  `%Object.prototype%` default with a `debug_assert`, and R-NAT gives each member its own default.
- **Upgrade.** `invoke_upgrade` loses `receiver` and `pre_alloc`; the pre-construct prototype read goes
  (S-2).
- **Already closed by R-JS** (not waiting for R-NAT): both receiver vectors of
  `#11-vm-native-ctor-receiver-promotion-admission`. A native now only ever receives a fresh `Ordinary`
  minted by the native arm: a second `super()` gets a new one (then ReferenceError), and the upgrade no
  longer passes the wrapper. The double-`super()` debug panic goes with them.
- **Wire #5, first half** (§4.6).

### §4.3 R-NAT core and ECMA-262 chapter (VM P4)

- **Helper module** (NEW, core, ungated, beside `vm/construct_admission.rs`):
  - phase 1, `get_prototype_from_constructor(new_target, default_proto) -> Result<ObjectId, VmError>`
    (born in R-JS, §4.2): ECMA-262 §10.1.14 and Web IDL §3.8 steps 2-3, which have the same shape;
  - phase 2, `alloc_with_kind(proto, kind) -> ObjectId`: OrdinaryObjectCreate / MakeBasicObject; the kind,
    and for a `HostObject` its brand (L3 brand-check PR-1, Web IDL §3.8 step 8), written once;
  - `NativeContext::create_from_new_target(default_proto, kind)`: both, with the Call-mode branch (E-11).
    Not spelled `if !ctx.is_construct()` nor as `let Some(..) = ctx.new_target() else` with the "new
    operator" message, so trip-wires #1 / #3 stay silent.
- **The flip.** The entry's native arm passes *uninitialized* (`undefined`); the last pre-allocation goes.
- **ECMA-262 members (28)**: each creates at its creation step. Step-position table, one row per member
  (constructor, spec step of creation, today's site, feature row). Inputs for its rows: L3 F1 / F1b /
  typed arrays / F2 (ECMA part), and PR-0A's placement table (its §4.5.6 DoD item 2) as a cross-check.
  Rows of note: `Array` (ArrayCreate, not OrdinaryCreateFromConstructor), `String` (StringCreate),
  TypedArrays (`construct_typed_array`; AllocateTypedArray before the iteration), `Promise` (creation
  before the executor), Error family (`error_entry` / `error_ctor_impl`, AggregateError's SD-3 order
  unchanged), `Date` (after ToNumber; the `store` setter becomes a payload write).
- **Deleted.** `ensure_instance_or_alloc`, `promote_to_*`, the 41 `.kind =` lines; wire #5 complete.

### §4.4 R-NAT host chapter (L3) — stub

Written by L3 after this umbrella is ratified (the PM tells L3). It must contain:

1. **The 70 rows**, by family, each with: the interface, Web IDL (or WebAssembly JS API) creation step,
   today's site, the helper call, the feature row, and whether the member spawns ECS state. Population:
   §1.1's host 70 (F1 29, F2 35, F3 2, F5 4); re-run `H/scripts/ctor_census.py` at the chapter's base.
2. **Argument conversion before creation** (Web IDL step 7 → 8) per member, so no temporary rooting is
   needed while user `valueOf` runs.
3. **E-7 for every ECS-spawning member** (Worker, OffscreenCanvas, observers, WebSocket, EventSource, …):
   the order prototype `[[Get]]` → side effects → allocation, and what a throw between them leaves.
4. **Shared helpers**: `create_fresh_event_object` (`vm/host/events.rs:299`; its Call-mode branch serves the
   4 VM-internal dispatchers), `build_ui_event_instance`, `build_event_subclass_instance`, `construct_rect`
   (`DOMRect`'s side store insert after allocation; PR-0A review IMP-4), `build_response_instance`, and
   `instantiate_module`, which also serves the non-constructor `WebAssembly.instantiate`
   (`vm/host/wasm/namespace.rs:578`) and holds CP-3b as its first statement (§8).
5. **`HTMLElement`** (unchanged creation through the wrapper identity map; `resolve_validated_prototype`
   folded onto phase 1, S-1) and `%TypedArray%`'s body (unchanged).
6. **`DataTransfer` / `Touch`** (S-3): the subclass prototype, with cells.
7. **`HostObject` hunks** for the PM at plan time (PM 642 (a)): the Worker / OffscreenCanvas allocations
   and their brand.
8. **Touch-time verdicts** for its files over 1000 lines: R-0c, R-0d (§7).
9. **L3's cells A-1 … A-9** re-expected under E-2 / E-6 / E-11, and the brand-check §6.2 cell that PM 664
   re-points (§5.1).

### §4.5 How R-NAT is assembled (proposal)

- **One squash, one PR.** Stacked branches merged back-to-back are rejected: between the two merges main
  would hold two native contracts (the strangler PM 675 forbids), and "no release in between" cannot be
  enforced.
- **Branches.** `rnat-core` (VM P4) from main after R-JS: helper phase 2 and the ECMA-262 chapter.
  `rnat-host` (L3) stacked on `rnat-core`: the host chapter. The flip commit (VM P4) on top of `rnat-host`.
  The PR is opened from the top branch. No two lanes push to one branch.
- **Commit order.** (1) helper phase 2; (2) ECMA-262 members; (3) host members (L3, one commit per family);
  (4) the flip, the deletions and wire #5. Commits 2 and 3 are green on their own: a member that creates
  its own object is valid under the pre-allocation contract (an Object return wins), so every commit
  builds and passes; only the squash reaches main.
- **Review.** One `/elidex-review` and one Codex pass on the whole PR; L3 reviews the host commits and VM
  P4 the rest. A rebase of `rnat-core` is followed by a rebase of `rnat-host` before the flip commit is
  re-applied.

### §4.6 Wire #5 (`.claude/tools/native-ctor-guard-trip-wire.sh`)

Property: **an object's kind variant is written only in the `Object { kind: … }` literal at allocation**,
and no construct path carries a receiver. Born in R-JS (`pre_alloc_instance`, `new_instance` → 0),
completed in R-NAT (`ensure_instance_or_alloc`, `promote_to_\w+\(`, and assignments of an `ObjectKind::`
value through `.kind =` or a `&mut` deref → 0, PR-0A review MIN-2), with a control mutant in the same
commit.

### §4.7 What must not ship

- Natives migrated one at a time while the native arm still pre-allocates ("host first, ECMA-262 later").
- Any receiver-admission gate (`AdmittedReceiver`-style), landed or planned to be deleted.
- R-NAT before R-JS: it would spread E-4's arrow breakage from three parents to every native parent until
  R-JS lands (PM 675).

## §5. Slot homing

Slot ids are exact matches in the repo ledger (`docs/plans/2026-07-vm-p4-slot-ledger.md`) or the PM ledger
(`project_open-defer-slots.md`), as each row says.

### §5.1 Closed

| Slot | Ledger | Closed by | Note |
|---|---|---|---|
| `#11-vm-native-ctor-receiver-promotion-admission` | PM `:270` | R-NAT (the class) | R-JS already removes both vectors (§4.2). The PM 660 / 664 facets (`:271`; the 39 / 37 counts) concern a guard that is no longer written. |
| `#11-vm-get-prototype-from-constructor-fallback` | PM `:268` | R-NAT | R-JS discharges the reachable facet (JS base, E-1); R-NAT gives every native its default. |
| `#11-vm-derived-constructor-this-binding` | repo `:293` (PM pointer `:1301`) | R-JS (the derivation and its children) | the four regressions and base controls are R-JS's acceptance. PM 660 facet (`:1302`): the Worker / OffscreenCanvas guard is not written, because R-JS gives natives fresh objects. PM 664 facet: the brand-check §6.2 cell expects ReferenceError (`super()` path, R-JS) and TypeError (upgrade path, step 11.4). |

### §5.2 Edges changed

| Slot | Ledger | Change |
|---|---|---|
| `#11-ce-upgrade-returned-object-typeerror` | PM `:1029` | not closed; its reachable set grows (every re-prototyped upgrade reaches step 11.4); its trigger fires on R-JS (`upgrade.rs`); its text's "step 10.4" is 11.4 today (P-11) |
| `#11-vm-arrow-lexical-super` (S), `#11-vm-arrow-lexical-new-target` (N) | repo `:287`, `:286` | S already waits on the derived-this derivation's `[[ThisBindingStatus]]` mint; R-JS mints it, decided with S and N (§4.2) |
| `#11-step9-class-extras` (Slice 2ab) | repo `:321` | R-JS creates the InitializeInstanceElements seam (E-10); 2ab fills it |
| `#11-vm-proxy-reflect` (slice-plan row 10a) | PM `:330` | unblocked: a caller of the single entry, no receiver |
| `#11-vm-typed-array-create-construct` | PM `:249` | unblocked: TypedArrayCreateFromConstructor calls the entry |
| Array.from / Array.of (row 9a), PromiseResolve(C) (row 9da) | PM `:248` | unblocked the same way |
| `#11-vm-function-constructor-global`, `#11-vm-map-set-collections`, `#11-vm-regexp-constructor-and-flags`, `#11-vm-float16array-global`, `#11-vm-weakref-finalization-registry` | repo / PM | each future constructor is born on (R): it creates through the helper |
| `#11-vm-native-error-prototypes` | PM `:250` | E1 rebases onto R-NAT (§6) |
| `#11-error-object-builtintag-dual-representation` | PM `:228` | unchanged owner (E1); E1 writes `ObjectKind::Error` through the helper |
| `#11-vm-value-reexport-shim-retirement` | PM `:1490` | placement §9.2 |
| `#11-vm-operand-rooting-by-construction` | repo `:306` | no new instance: the helper's `? Get` runs before allocation (A2×A5) |
| `#11-bound-dispatch-soundness-by-contract` | PM | no effect (script-session trait surface) |

### §5.3 Sibling defects homed here (no slot; each is a named deliverable with cells)

- **S-1 `resolve_validated_prototype` throws.** On a non-Object `prototype` it throws TypeError
  (`vm/host/custom_elements/html_element.rs:282-285`), where HTML §3.2.3 step 11 (and Web IDL §3.8 step
  3.3 on the sync path) fall back to the interface prototype of NewTarget's realm. Latent today (no
  Reflect.construct). Home: R-NAT host chapter, by folding it onto phase 1.
- **S-2 the upgrade's pre-construct `prototype` read.** `invoke_upgrade` reads `constructor.prototype`
  before constructing and fails with TypeError on a non-Object (`upgrade.rs:216-222`). HTML checks this
  only at `define` (§4.13.4); a plain function's `prototype` stays writable, so `F.prototype = 1` after
  `define` makes elidex skip the constructor (its side effects) where the spec runs it and then fails at
  step 11.4. The read exists to pre-publish the wrapper's prototype for the receiver design (D-17b §6),
  which R-JS removes. Home: R-JS.
- **S-3 `DataTransfer` / `Touch` ignore the subclass prototype.** Both allocate with the fixed interface
  prototype (`vm/host/events_modern/data_transfer.rs:332-341`, `vm/host/events_modern/touch.rs:327-335`),
  so `class T extends Touch {}; new T(init) instanceof T` is false. Home: R-NAT host chapter.

### §5.4 Ledger edits at ratification (for the PMO; listed, not made here)

- PM `:270`: owner "native construct protocol program" already reads R-NAT (PM 675); add "R-JS removes
  both vectors". PM `:268`: add "R-JS: reachable facet".
- Repo `:293`: the PM 660 / 664 facets are restated as above, replacing the PR-0A wording of PM 665's
  facet 1.
- PM `:1029`: "step 10.4" → "step 11.4".
- PM `:1298` points `#11-vm-class-instance-field-init` at the repo ledger, where `:386` records it as
  withdrawn; re-point to `#11-step9-class-extras` (Slice 2ab).

## §6. E1 (NativeError `[[ErrorData]]`)

E1 (`docs/plans/2026-11-vm-native-error-e1-errordata.md`, not landed: `git grep -n
'promote_to_error\|error_receiver' 4b723d4b -- crates` → 0) lands **after R-NAT**, rebased onto the helper.
M18 is withdrawn (PM 675). Its ratified surface changes as follows; E1 gets a focused plan-review on these
lines when it resumes.

| E1 site | Today's text | After R-NAT |
|---|---|---|
| §1.1 creation census, the ten constructor rows (`:198-201`) | `error_receiver` (NEW) | the helper with `ObjectKind::Error { name }` and the default prototype; `error_receiver` is not written |
| §1.2 mechanism (`:211-300`) | `promote_to_error` promotes an `Ordinary` receiver in place; the census paragraph "every native constructor with a slot promotes that receiver's kind in place" | not written; the paragraph becomes history; the four engine creators keep `create_error_object`, which is the helper's phase 2 with the default prototype |
| §1.2 (a) upgrade path | a known deviation pinned by T-PROMO-UPGRADE | the spec outcome: step 11.4 fails (TypeError, NotSupportedError until `#11-ce-upgrade-returned-object-typeerror`), `el.name === undefined` |
| §1.2 (b) double `super()` and its slot text ("E1 does not change", "takes neither", no cell because it panics) | false after R-JS | a cell: fresh object, then ReferenceError (`class E extends TypeError`, second `super()`), PR-0A review IMP-3 |
| §1.2 "Subclass" paragraph (`:288-292`) | the derived frame's receiver is promoted | the native creates from NewTarget; `E.prototype` comes from phase 1 |
| §2 the receiver-admission axis (`:518-522`, `:534-540`) | a fourth axis, M18 | removed |
| §3 OrdinaryCreateFromConstructor row (`:596`) | "do_new's pre-allocation, promote_to_error" | the helper at step 2 |
| §4 T-PROMO-UPGRADE (`:662`) | pins base behaviour | expects the spec outcome above |
| §4.2 M1 / M2 / M4 / M5 / M18 (`:690-716`) | mutants of `error_receiver` / `promote_to_error` | M18 withdrawn; the rest re-expressed on the helper call (wrong kind, wrong default prototype, bypassing the helper); new mutant "the constructor reuses `this`", which turns T-PROMO-UPGRADE and the double-`super()` cell red |
| §5 hand-off bullets (`:744-758`) | receiver admission → its slot; trigger restated with `promote_to_error` | closed by R-NAT; nothing to restate |
| §0.3 / §1.2 upgrade step numbers | 10.3 / 10.4 | 11.3 / 11.4 (P-11) |

## §7. 1000-line touch-time verdicts

`wc -l` at `4b723d4b` of every file R-JS or R-NAT touches (`H/logs/r-touched-files-4b723d4b.txt`, 73
files); the files over 1000:

| File | Lines | Touch | Verdict |
|---|---|---|---|
| `vm/mod.rs` | 2797 | one `mod` line (the entry / helper module) | **no seam for this touch.** The file is the `VmInner` struct (`:188-2790`, 351 fields); its decomposition is a design program with its own home (memory `vm-inner-substruct-deferral`), not a touch-time split. A one-line declaration does not fire it (PM 654 Q-14 reasoning). |
| `vm/gc/trace.rs` | 1255 | R-JS: the suspended-frame `new_instance` (`:350`) and arrow `captured_this` (`:255`) | **real seam: R-0b** (one frame-root enumeration shared with `vm/gc/roots.rs`), a standalone prereq. The rest is `trace_work_list`'s match over `ObjectKind` (a flat case table, exempt); the file stays over 1000 after R-0b and that remainder is the exempt table. |
| `vm/host/events_modern/data_transfer.rs` | 1226 | R-NAT: the `DataTransfer` constructor (S-3) | **real seam: R-0c** (three interfaces, three files), a standalone prereq before R-NAT's host commits. PM 654 Q-14 ruled a one-`..` mechanical ripple out of scope; R-NAT's hunk is a content change to the constructor, so that ruling does not cover it. |
| `vm/value.rs` | 1176 | R-JS: `FunctionObject::captured_this` (`:598`) | **discharged by R-0a** (967 after #625). |
| `vm/dispatch.rs` | 1104 | R-JS: `Op::PushThis` (`:738`), `complete_inline_frame` (`:1087`) | **no prereq split.** The file is `run()`'s opcode match (exempt, per the touch-time record) plus `complete_inline_frame` (17 lines), which R-JS rewrites and moves into the entry module (net −17). |
| `vm/host/websocket.rs` | 1001 | R-NAT: the constructor (`:329`) | **seam exists** (construction and protocol validation, `:329-559`, as `websocket/construct.rs`); the verdict is the host chapter's, proposed: split before R-NAT, because R-NAT's hunk is a content change to that unit. |
| `vm/object_kind.rs` | 1645 | only if R-JS changes `FunctionObject`'s home or the HostObject variant changes | conditional; if touched, the per-PR plan gives the verdict (PM 642 (a): a split is a standalone prereq ordered by the PM) |
| `vm/tests/tests_custom_elements.rs` | 1321 | none: new cells go to a new test file | — |

Under 1000 and touched: `vm/ops.rs` 893, `vm/natives_promise.rs` 961, `vm/host/range_proto.rs` 906,
`vm/interpreter_call.rs` 727, `vm/dispatch_class.rs` 391, `vm/inner.rs` 315, `vm/interpreter.rs` 330.

## §8. Overlap with stack-safety (P0a-1, PR-1, PR-1b)

### §8.1 Statement-level list

| Function (`4b723d4b`) | Stack-safety statement | This program's statement | First | Second rebases |
|---|---|---|---|---|
| `with_call_mode` (`vm/interpreter.rs:160`) | P0a-1 CP-1: `check_entry` as the first statement; PR-1 fills it with the probe | unchanged | P0a-1 | — |
| `call_construct_native` (`vm/interpreter.rs:95`) | entered by P0a-1's cell `rl_cp1_entry_failure_no_callee_effect` from Rust | R-JS drops `this` (or folds the function into the entry) | P0a-1 | R-JS rewrites the cell's Rust call |
| `construct_synchronous` (`vm/dispatch_class.rs:273`; `with_call_mode` at `:323`) | the same cell enters through it | R-JS drops `receiver` / `pre_alloc_instance` | P0a-1 | R-JS rewrites the cell's call |
| `dispatch_super_inner` (`vm/dispatch_class.rs:116`) | P0a-1 cites it as "mutates nothing before `with_call_mode`" | R-JS: Construct, then BindThisValue after the call | P0a-1 | R-JS keeps the claim true (nothing before the entry) |
| `do_new` (`vm/ops.rs:681`) | none (JS `new` is inline; the native path's CP-1 is inside `call_construct_native`) | R-JS: the entry | — | — |
| `call_dispatch` (`vm/interpreter_call.rs:17`) | PR-1b: the `WasmExportedFunction` arm (`:241-246`, CP-3a's caller) | none (§1.2) | — | — |
| `push_js_call_frame` (`vm/interpreter_call.rs:505`) | PR-1 cites it (probe placement, `pr1.md:352-353`) | R-JS: the `new_instance` parameter (`:511`) and its literals (`:385`, `:420`, `:469`) | R-JS | PR-1 re-reads its anchors at its plan-review |
| `instantiate_module` (`vm/host/wasm/instance.rs:151`) | P0a-1 CP-3b: `check_entry` as the first statement | R-NAT: the receiver parameter and `.kind =` (`:188`) | P0a-1 | R-NAT |
| `invoke_upgrade` (`upgrade.rs:129`) | P0a-2b reads the `construct_synchronous` `Err` arm (`p0a2b.md:175-176`, `:225`) | R-JS: the call's arguments and S-2's pre-read | R-JS | P0a-2b re-checks the two anchors |
| `run_reaction` (`vm/natives_promise.rs:665`) | P0a-1 D-16: `ReactionTarget` in the reaction record, the `Err` arm | none; R-NAT edits `native_promise_constructor` (`:712`) in the same file | P0a-1 | — (different function) |
| `mark_roots` / `trace_work_list` (`vm/gc/roots.rs`, `vm/gc/trace.rs`) | P0a-1 D-16: the reaction-target tracing (`roots.rs:416-428`, `trace.rs:284`) | R-0b / R-JS: the frame-root blocks (`roots.rs:277-313`, `trace.rs:335-372`) | either | the later one (different statements) |
| `coroutine_types.rs` `Reaction` | P0a-1 D-16 changes the record | the shim retirement rewrites `value::Reaction` imports (§9.2) | P0a-1 | the retirement (script-regenerated) |

### §8.2 The census (P0a-2a)

P0a-2a rule 1 makes a carrier's use a member whenever a producer runs beneath the call through any function
(`p0a2a.md:27-29`). R-JS rewrites the construct entries (members appear and disappear there); R-NAT adds a
fallible, re-entrant call (`? Get` on `prototype`) to every creating member. So the classification must
measure a main that already has both, or R-JS / R-NAT must carry their own rows. §9.1 orders it so the
classification runs once.

### §8.3 The PM 642 rule

R-JS and R-NAT do not touch `call_dispatch` (the entry passes the native's `this`) and do not change the
`ObjectKind::HostObject` definition. R-NAT writes two `HostObject { … }` allocation literals (Worker,
OffscreenCanvas); per PM 642 (a) their hunks go to the PM at the host chapter's plan time, and L3's
brand-check PR-1 (after R-NAT) adds the brand at exactly those two literals. Under PM 642 (c) and PM 654
E-9, every `interpreter_call.rs` hunk is written per function in the per-PR plans, and the later of two
overlapping PRs rebases.

### §8.4 The CE reaction drain and the microtask checkpoint (for PM 684)

L3's ShadowRoot PR may touch this area. Every function in it that R-JS or R-NAT touches (at `4b723d4b`):

- `invoke_upgrade` (`vm/host/custom_elements/upgrade.rs:129`), R-JS: its `construct_synchronous` call loses
  `JsValue::Object(wrapper_id)` / `Some(wrapper_id)` (the call becomes a receiver-less call of the single
  entry); the pre-construct `prototype` read (`:216-222`) is removed (S-2); the step 11.4 SameValue test
  and the `Err` arm are unchanged.
- `native_html_element_ctor` (`vm/host/custom_elements/html_element.rs:107`), R-NAT host chapter: the
  construction-stack return (steps 12-16) and the sync path's wrapper creation are unchanged; its two
  `resolve_validated_prototype` calls (`:204`, `:249`) go through the helper's phase 1 (S-1).
- `resolve_validated_prototype` (`html_element.rs:276`), R-NAT host chapter: folded onto phase 1 (S-1).
- Untouched: `invoke_upgrade`'s callers `invoke_one` (`vm/host/custom_elements/flush.rs:100`), the
  creation path (`creation.rs:238`) and the `upgrade()` walk (`lookup.rs:183`); `flush_ce_reactions`
  (`flush.rs:39`); `invoke_callback` (`flush.rs:156`; lifecycle callbacks are calls, not constructs);
  `drain_reactions` (`crates/script/elidex-js/src/engine.rs:314`); `drain_microtasks` and
  `drain_queue_pass` (`vm/natives_promise.rs:274`, `:311`). R-NAT edits `native_promise_constructor` in the
  same file (`:712`), a different function.

## §9. Placement and dates

### §9.1 R-JS and R-NAT (proposal)

Order, with the reason for each edge (dependency or regression):

1. value.rs split (#625) — **11-18** (unchanged). R-0a; hard edge for R-JS.
2. R-0b frame-root split — **11-20**. After #625 (it reads `vm/frame.rs`); before R-JS (both copies change).
3. PR-A1 — **11-21** (unchanged). No shared function (assessment Q3, FP-3 of the PR-0A review).
4. default-row rustdoc PR — **11-25** (unchanged). Shares only the `vm/inner.rs:14` doc link.
5. P0a-1 implementation — **11-28** (proposed; its date is the stack-safety program's). Before R-JS:
   P0a-1's ratified plan cites `call_construct_native` / `construct_synchronous` as they are today, and
   R-JS first would change a ratified surface (a re-review); P0a-1 first leaves R-JS a mechanical rebase of
   one cell.
6. **R-JS — lands 12-05** (plan written 11-17 … 11-19, plan-review 11-20 … 11-24, implementation 11-28 …
   12-04). After P0a-1 (above); before R-NAT (PM 675, E-4); before P0a-2a's PR, so P0a-2a's measured report
   sees the final construct entries.
7. PR-D — **12-04** (unchanged; doc links only; whichever lands second rebases trivially).
8. P0a-2a PR — after R-JS (**12-09**, proposed).
9. **R-NAT — lands 12-15** (core chapter and host chapter written 11-26 … 12-02, plan-review 12-03 …
   12-05, implementation 12-08 … 12-12, the two lanes in parallel). After R-JS; independent of P0a-2a's
   PR.
10. Shim retirement — **12-16** (§9.2).
11. The classification → P0a-2b — the classification starts after step 10, at that main, so it measures
    R-JS's entries, R-NAT's helper rows and the retirement's import lines once (§8.2). Its dates are the
    stack-safety program's.
12. Slice F — unchanged relative order (after P0a-2b); it shares no function with this program
    (`vm/host/structured_clone.rs` has no `.kind =` line and no constructor).
13. E1 — after R-NAT **and** Slice F (§6).

Effect on the current queue: P0a-2a's PR moves behind R-JS by about a week, and the classification behind
R-NAT and the retirement. Nothing regresses in the window: R-JS keeps the native contract (centralised),
and R-NAT is atomic. If the PM keeps P0a-2a's classification ahead of R, the alternative is that R-JS and
R-NAT each classify their own rows in their PRs (no `# pending` left, which is PR-1's precondition).

### §9.2 `#11-vm-value-reexport-shim-retirement` (PM 676)

**Measured.** `vm/value.rs` re-exports `coroutine_types::*`, `VmError` / `VmErrorKind`, `ObjectKind` and
`ObserverKind` (`:12`, `:17`, `:22`, `:24`). Files naming one of them through a `value::` path:

- single-line spelling: `git grep -lP "value::(\{[^}]*\b)?(<names>)\b" 4b723d4b -- crates/script/elidex-js/src`
  → **201** (also 201 at #625's `ab71cb43` and at `654fcb3a`); outside `crates/script/elidex-js/` → 0;
- by the property (brace groups across lines included): `python3 -I H/scripts/shim_census.py <repo>
  4b723d4b` → **309** (`H/logs/shim_census-4b723d4b.log`). Example missed by the single-line spelling:
  `vm/dispatch_iter.rs` (`value::{` on one line, `ObjectKind` on the next).

`<names>` = `ObjectKind|ObserverKind|VmError|VmErrorKind` and the 11 public types of
`vm/coroutine_types.rs`. 71 of the 73 files R touches are in the 309.

**Proposal: immediately after R-NAT, as its own script-generated PR** (step 10 of §9.1).

- **Conflict.** Before or during R, the retirement would force R-JS, `rnat-core` and `rnat-host` (two lanes
  in parallel) to rebase over 309 files, 71 of them their own. After R-NAT no R branch is in flight, and
  the next users of those files (L3's brand-check PR-1, E1) start after R-NAT, so they start on the
  retired base and rebase nothing.
- **Decision surface.** Bundled into R-NAT, a joint semantic PR would carry import-only hunks in 309 files
  beside the contract hunks in its own files (71 of them shared); every reviewer of either chapter would have to sort each hunk
  into "contract" or "import". As its own PR, the retirement has one reviewable artifact: the script, run
  at the PR's base (regenerated at rebase, never hand-merged).
- **One rule for R's own lines:** code R-JS / R-NAT write imports `VmError`, `ObjectKind` etc. from their
  defining modules, so the retirement never revisits R's lines.
- The retirement also precedes the classification (§9.1 step 11), so the import-line shift happens before
  the census is measured.

## §10. Measurements (provenance)

All at `4b723d4b` unless stated.

- **M-1 P.** `python3 -I H/scripts/ctor_census.py /Users/kazuaki/repos/send.sh/elidex-wt-nativecons
  4b723d4b` → 59 constructable install sites, 89 ctor-named functions (2 CallableOnly), 51 literal + 36
  parameter-site + 11 macro = **98** (`H/logs/ctor_census-4b723d4b.log`).
- **M-2 L3's census.** `census2.py` (unmodified) over a flat copy of `crates/script/elidex-js/src` at
  `4b723d4b` → 90 rows (`H/logs/l3-census2-4b723d4b.log`); the set differences against M-1's 89 names:
  L3-only `require_subclass_ctor`; M-1-only none.
- **M-3 kind writes.** `git grep -n -P '\.kind\s*=[^=]' 4b723d4b -- crates/script/elidex-js/src
  ':!*tests*'` → 41 (35 files; `vm/inner.rs` 5).
- **M-4 adoption calls.** `git grep -n -P 'ensure_instance_or_alloc\(' 4b723d4b -- crates/script/elidex-js/src
  ':!*tests*'` → 15 lines (14 calls + the definition) in 11 files; `promote_to_\w+\(` → 6 calls.
- **M-5 shims.** §9.2 (201 / 309).
- **M-6 spec numbers.** `.claude/tools/webref coverage-map ecma262 10.2.2 ecma262 10.3.2 ecma262
  BuiltinCallOrConstruct ecma262 OrdinaryCreateFromConstructor ecma262 GetPrototypeFromConstructor …
  webidl 3.8 webidl 3.7.1 webidl 3.12 html 3.2.3 html 4.13.5`; step text from `webref body` on each anchor.
- **M-7 line counts.** `git show 4b723d4b:<path> | wc -l` over `H/logs/r-touched-files-4b723d4b.txt`;
  `vm/value.rs` at `ab71cb43` → 967.
