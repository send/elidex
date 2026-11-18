# Native construct protocol (R) — umbrella

Status: **plan (pre-implementation), Rev 1.3** (§0.2), for `/elidex-plan-review` (edge-dense: §2). Owner: VM P4
lane. The R-NAT host chapter (§4.4) is L3's. The per-PR plans (R-JS; R-NAT core / ECMA-262 chapter; R-NAT
host chapter) are written after this umbrella is ratified (the PM's go; PM 737) and each goes through
`/elidex-plan-review` before code.

**Base.** Code anchors are at `4b723d4b` unless a row says otherwise (the anchors Rev 1.2 and Rev 1.3
added included). Facts about the value.rs split (#625, landed 11-11 as `439fa47c`) were measured at its PR
head `ab71cb43`.

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
- Web IDL *create an interface object* step 1.8 internally creates the object from NewTarget, then step
  1.9 runs the constructor steps "with object as this" (overload resolution is step 1.7).
- HTML *upgrade an element* step 11.3 constructs C with no arguments and no receiver; step 11.4 is the
  SameValue test. *HTML element constructors* return the construction-stack element (steps 12-16).
- `Reflect.construct` calls `Construct(target, argList, newTarget)`, and Proxy `[[Construct]]` forwards the
  same call when the handler has no `construct` trap (§10.5.13 step 7). Neither has a receiver.

The program makes elidex's construct protocol that shape, in two PRs cut by callee kind (§4), so that the
object's kind (and, for a host object, its brand) is written once, at allocation. "Written once" covers
those two facts on the object. Mutable per-instance state that a kind keeps outside the object (an
`ObjectId`-keyed side table, say) is not the kind: it stays mutable and is not moved here (E-7).

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

### §0.2 History

| Rev | Change | Rulings |
|---|---|---|
| 1 | First draft (`6b62e89e`). | PM 675, PM 676 |
| 1.1 | §9: R-0b moved to 11-24 (after this umbrella's plan-review); §9's dates are canonical; P0a-1's implementation fixed at 11-28; PR-A1 recorded as landed (#628 `c0578223`, 11-13). §4.5 / §4.5.1: R-NAT branch ownership, pull-in by merge, per-commit green record. No other design content changed. | PM 687, PM 688 |
| 1.2 | R1 plan-review at `d11c4df7` (CRIT 0 / IMP 9 / MIN 21 after dedup). **F1** WebSocket protocol-list validation goes to `elidex-api-ws`; R-0d re-derived (§4.1, §7). **F2** the outside-write population is defined by property, four classes, one no-orphan rule (§4.4 item 3; E-7 and A5×A10 refer to it). **F3** `HTMLElement`'s sync path runs phase 1 before the spawn (§4.4 item 5, §8.4). **F4** per-constructor Call-mode table; the active-function branch is its own helper entry (§4.3, E-11, §3). **F5** P-11 enumerates 13 stale upgrade cites and homes them. **F6** R-0c / R-0d are §9.1 steps 8 / 9 (the later steps renumbered). **F7** repo `:293` is VM P4's, in the rustdoc PR (§5.4, §9.1 step 4). **F8** PM `:271` / `:1327` added to §5.4. **F9** VM-P4 family homes named and split (§4.2, §1.3). MIN: Web IDL §3.7.1 steps are 1.7 / 1.8 / 1.9; OrdinaryCallBindThis, GetThisEnvironment, MakeBasicObject, OrdinaryObjectCreate step ranges; WebAssembly JS API rows (§3); docstring rule for NEW functions; M-6's full command; PM-ledger line numbers re-measured; Re-eval dates (§5.4); A6×A7 and the `invoke_upgrade` row state S-2's flow change; `call_internal` row (§8.1); plan-review skip for R-0b / R-0c / R-0d (§4.1); §4.4 row schema gains layering and state-shape columns; R-0c's seam choice recorded; the scope of "written once" (§0); the R-JS-stage fallback's edge to `#11-vm-proxy-reflect`; `EvalError` / `SharedArrayBuffer` homes (§1.3); wires #1 / #3 exempt the helper module by property (§4.6); `vm/mod.rs` verdict cites its slot (§7). **PM 737**: the R-JS plan window is triggered by the umbrella's ratification; no date moves at the estimate (§9.1). **PM 745**: the host-chapter window is PM-confirmed. | PM 737, PM 743, PM 745 |
| 1.3 | R2 plan-review at `e47adadc` (CRIT 0 / IMP 9 / MIN 18 after dedup). **Altitude correction (PM 754; precedent #374):** §4.4's class table, per-class write orders, windows and cells were host-chapter content at umbrella altitude and kept producing findings (G2, G3, four A2 MINs). They are removed; the umbrella keeps one invariant, stated once (E-7), and §4.4 lists what L3's chapter must contain. **G1** R-0d mints no token predicate; the predicate's home is for the PM (§4.1). **G4** `invoke_upgrade`'s Phase 3 under S-2: `:244` stays; `:245` is D-17b §6's and is not retracted here (O-1, for the PM; §5.3). **G5** Rust-driven `Construct` cells required of R-NAT's plan (§4.3). **G6** P-11 by property over `crates/`: 19 lines (§1.4, M-8). **G7** the upvalue candidate's dependency on `#11-vm-upvalue-tdz-check` / umbrella C (§4.2, §5.2). **G8** R-JS against the parked Slice 1aa / 1ac (§4.2). **G9** L3's capacity for 12-08 … 12-12 is an open condition (§9.1 step 11). MIN: OrdinaryCallBindThis step 6; per-prerequisite plan-review (§4.1); wire #5's complement by property (§4.6); `HTMLElement`'s Call mode at HTML §3.2.3 step 5; Proxy forwards only without a trap (§0, §3); `define` steps 14.1-14.2; phase 1 = Web IDL §3.8 step 3 (3.2-3.3); row → PR ownership (§3); the Layering column's disposition and the State-shape values (§4.4 item 1); `#11-error-object-builtintag-dual-representation` in the Re-eval sweep (§5.4); the `push_js_call_frame` row's `pr1.md` reference (§8.1); "at `4b723d4b`"; the family-layout pin (§4.2). Vanished with the removed layer: the two definitions of "window", EventSource's unnamed pattern, the `HTMLElement` class-(a) row, S-1's missing reference. **PM 750**: R-0c targets 11-21, R-0d 11-24, hard edge 12-04. **PM 752**: `#11-data-transfer-drag-data-store-layering` cited; `SharedArrayBuffer` is `#11-wasm-threads`'s (§1.3). | PM 750, PM 752, PM 754 |
| 1.4 | PM 784 on Rev 1.3's open items. **(1) O-1** is with the user, not decided: every S-2 site stays conditional, D-17b §6 unchanged (§5.3). **(2) G1** option (A): `elidex-api-ws` depends on `elidex-api-fetch` and R-0d's validator calls `is_valid_header_name`; (B), a `token` predicate in `elidex-plugin`, rejected (Plugin-first); the host header copy and the `elidex-net` matcher go to a slot the PM registers (§4.1, §7, M-10). **(3)** R-0b / R-0d: a short memo and `/elidex-plan-review` each, targets 11-24, hard edge 12-04, a date past 11-26 reported to the PM (§4.1, §9.1 steps 3 / 9). **(4) G8** R-JS before 1aa (§4.2). **(5) G6** all 11 `elidex-custom-elements` cites on `#11-ce-upgrade-returned-object-typeerror` (L3) (§1.4 P-11, §5.2, §5.4, M-8). **(6)** R-0d's open condition closed by (A) (§9.1 step 9). **(7) G9** stays open; L3 re-derives its dates after its tree-model R2 (§9.1 step 11). No other design change. | PM 784 |

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
| `invoke_upgrade`'s pre-construct `prototype` read (`vm/host/custom_elements/upgrade.rs:216-222`) | S-2's subject (§5.3); kept as today until open item O-1 is ruled | R-JS |
| `resolve_validated_prototype` (`vm/host/custom_elements/html_element.rs:276`) | folded onto the helper's phase 1 (§5.3 S-1) | R-NAT (host) |

**Unchanged (re-verified):** `with_call_mode` (`vm/interpreter.rs:160`); `NativeContext::new_target` /
`is_construct` (`vm/native_context.rs:152` / `:138`); `NativeContext::new_construct` (`vm/value.rs:825`);
`CallMode` (`vm/value.rs:648-657`); the `NativeFunction` signature; `call_dispatch`'s arms
(`vm/interpreter_call.rs:17`): the entry passes the native's `this`, so no arm changes (§8.3).

### §1.3 Out

| Item | Owner |
|---|---|
| Absent constructors (`Function`, `RegExp`, `Map` / `Set` / `WeakMap` / `WeakSet`, `WeakRef`, `FinalizationRegistry`, `Proxy`, `Reflect`, `EvalError`, `SharedArrayBuffer`, `Float16Array`) | their slots (§5.2); each is born on (R). `EvalError` is `#11-vm-native-error-prototypes`'s (PM ledger). `SharedArrayBuffer` is `#11-wasm-threads`'s (`vm/host/array_buffer.rs:32`, the module doc; PM ledger; outside the supported surface, PM 752), and it also gates `#11-vm-atomics-global`'s `wait` half (repo `:316`). Rev 1.2's "no slot of its own" was wrong |
| `[[HomeObject]]`: the frame field `home_class`, its `None` in `call_internal`, the closure representation; the super-property emit shape | slice-plan umbrella 3 (§4.2, family homes) |
| SuperCall's argument evaluation (step 4) and `op_super_call_spread`'s layout and inline cache | umbrellas 1a / 1b ([C13]; §4.2, family homes) |
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
| P-11 | E1 §1.2 and `construct_admission.rs` cite upgrade "step 10.3 / 10.4" | today's *upgrade an element* (`webref body html concept-upgrade-an-element`) has 13 steps: the attribute and connected enqueues are 4-5, the stack push is 6, the catching block is 11 (11.2 "precustomized", 11.3 construct, 11.4 SameValue; the cleanup and the exception branch are its unnumbered tail), "custom" is 13. **Population, by property: every line under `crates/` that cites a step of that algorithm by a number that is not today's** (a cite split over two comment lines included). **19 lines** (§10 M-8). `crates/script/elidex-js`, 8 → **R-JS**, which rewrites each site's subject (`invoke_upgrade`'s call, the entry's callers, `construct_synchronous`'s docstring, the return substitution): `vm/construct_admission.rs:51`, `vm/dispatch_class.rs:266` (the cite spans `:265-266`), `vm/host/custom_elements/upgrade.rs:275`, `:293`, `:297`, `html_element.rs:195`, `compiler/expr_class.rs:149`, `vm/tests/tests_custom_elements.rs:474` (a comment line; §7). Crate `elidex-custom-elements`, 11 → **L3**, with `#11-ce-upgrade-returned-object-typeerror` (PM 749 homed the first 8, PM 784 (5) all 11: one pass over the same files, One issue, one way; its fix rewrites `finalize_failure`, whose docstring is `:208-217`): `upgrade.rs:163`, `:208`, `:210`, `:211`, `registry.rs:233`, `:282`, `:294`, `tests.rs:194`, and three the property adds to Rev 1.2's spelling-bounded list, `reaction.rs:52` ("step 8", the exception tail), `registry.rs:40` ("step 12", today 4), `upgrade.rs:168` ("12", today 13) — the same home (PM 784 (5)). R does not touch that crate (§8.4). E1's memo: §6 |

## §2. Coupled invariants

Ten axes. Every pair is listed with what meets there; "—" means the pair shares no site or state (stated,
not omitted). The edges E-n are the cells' subjects (§2.2).

| Axis | Content |
|---|---|
| A1 entry | one `[[Construct]]` entry for `new`, SuperCall, the upgrade and every future caller (Reflect.construct, Proxy, TypedArrayCreate, Array.from) |
| A2 proto | GetPrototypeFromConstructor (§10.1.14) / Web IDL §3.8 step 3 (3.2-3.3): one resolver |
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
| A2×A6 | `resolve_validated_prototype` and the upgrade pre-read are two more resolvers; the first folds onto phase 1 (§5.3 S-1), the second is S-2's, subject to O-1 |
| A2×A7 | the helper's fallible `? Get` is a new producer path under P0a-2a rule 1 (reach); every member's `?` on it is a carrier use (§9.1) |
| A2×A8 | the default prototype is an argument, so the core helper reads no engine-gated `VmInner` field |
| A2×A9 | — |
| A2×A10 | the `[[Get]]` happens at each constructor's creation step: Web IDL after overload resolution (step 1.7 → 1.8), Error before `ToString` (§20.5.1.1 step 2), Date after `ToNumber` (§21.4.2.1 step 6) (E-8) |
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
| A5×A10 | a creation step that writes state outside the object (an entity, a side-table row, a reverse-map entry, a sent message) has a window between that write and the object's allocation or link; the invariant for it is E-7 |
| A6×A7 | P0a-2b reads two `Err` paths of `invoke_upgrade` (`p0a2b.md:175-176`, `:225`): D-3, the `prototype` read (`upgrade.rs:216`), and D-4, the `construct_synchronous` call (`:277`). R-JS changes D-4's arguments only. If O-1 (§5.3) rules that the read goes, D-3's site goes with its two failure exits (S-2, a flow change: P0a-2b's D-3 row has no site afterwards) |
| A6×A8 | custom elements are engine-gated; the core entry has no CE knowledge |
| A6×A9 | `vm/tests/tests_custom_elements.rs` (1321): new cells go to a new file |
| A6×A10 | `HTMLElement`'s step 9.1 creates through the wrapper identity map (`create_element_wrapper`), not the helper's allocation; only phase 1 is shared, and its position on the sync path is E-7 applied (§4.4 item 5) |
| A7×A8 | CP-3a / CP-3b are engine-gated (Wasm); the entry's CP-1 is core |
| A7×A9 | `vm/interpreter.rs` (330) and `vm/interpreter_call.rs` (727): no split |
| A7×A10 | CP-3b is `instantiate_module`'s first statement (`vm/host/wasm/instance.rs:151`); R-NAT edits its `.kind =` (`:188`) and its receiver parameter |
| A8×A9 | — |
| A8×A10 | the step-position table (§4.3, §4.4) carries each member's feature row |
| A9×A10 | the `data_transfer.rs` split (R-0c) precedes R-NAT's host hunk there |

### §2.2 Edges (each maps to cells in its per-PR plan)

- **E-1 prototype resolution.** `function F(){}; F.prototype = 3; Object.getPrototypeOf(new F()) ===
  Object.prototype` (today `null`, because `do_new` falls back to `None`). Accessor case: unreachable for
  built-ins and classes (non-writable, non-configurable `prototype`); reachable from script once Reflect /
  Proxy exist, so the helper takes `? Get` from the start. From Rust both cases are reachable now (the
  entry takes any constructor as newTarget): §4.3's Rust-driven cells.
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
- **E-7 the creation-order invariant** (stated here once; A5×A10, A6×A10, §4.3's Rust-driven cells, §4.4
  and §8.4 refer to it). For a Web IDL member the observable order is the specification's: argument
  conversion (Web IDL §3.7.1 step 1.7, overload resolution) → internally create (step 1.8; §3.8, whose
  step 3.2 is the prototype `? Get`, the helper's phase 1) → the constructor steps (step 1.9, the
  member's own validation included). Phase 2 (§3.8 steps 4-9, MakeBasicObject and its slot writes) is
  unobservable, so a member whose kind carries what its constructor steps produce runs it after them,
  and the kind is still written once. **No orphan:** a constructor that throws leaves no state outside
  the object (an entity, a side-table row, a reverse-map entry, a sent message) without its object.
  What gives it: **nothing fallible, and no user code, between a member's first outside write and the
  write that links the object to that state**; its guarantor is phase 2's signature (`-> ObjectId`,
  infallible). Which members write outside the object, each one's order and window against this
  invariant, and the cells are the host chapter's (§4.4).
- **E-8 step position.** The creation step differs per constructor (E-n of A2×A10). Unobservable until
  Proxy / Reflect.construct exist; the step-position tables (§4.3, §4.4) pin it so `#11-vm-proxy-reflect`
  inherits the right order. In the R-JS stage the native pre-allocation stays where `do_new` has it today
  (before argument conversion), a stated pre-existing deviation that R-NAT removes.
- **E-9 CP-1.** P0a-1's `rl_cp1_entry_failure_no_callee_effect` enters through `call_construct_native` and
  `construct_synchronous`; it stays green, with the stronger reading "no allocation either".
- **E-10 instance-field seam.** SuperCall step 12 / §10.2.2 step 6.b InitializeInstanceElements: R-JS puts
  one call point after BindThisValue (derived) and after OrdinaryCallBindThis (base), empty until Slice 2ab.
- **E-11 Call mode (NewTarget undefined).** The specification has four behaviours, per constructor (table
  in §4.3): `Array(3)`, `Error('m')` and the rest of the Error family substitute the active function and
  create; `Date()`, `String(x)`, `Number(x)`, `Boolean(x)` return a primitive and create nothing;
  `Object()` has its own steps 1-3; the others throw a TypeError. Only the first group reaches the creation
  helper in Call mode. L3's A-9 is reused, re-expected per group.

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| ECMA-262 §13.3.5.1.1 EvaluateNew ( constructExpr, argumentsNode ) | 5-6 | IsConstructor, then Construct | `do_new` → the single entry (NEW) | ✓ | yes |
| ECMA-262 §7.3.14 Construct ( ctor [ , argList [ , newTarget ] ] ) | 1-2 | newTarget defaults to ctor | the single entry (NEW) | ✓ | yes |
| ECMA-262 §10.2.2 [[Construct]] ( argList, newTarget ) | 3, 6, 12-17 | base / derived; Object / undefined / other return | the entry's JS arm; `complete_inline_frame` | ✓ | yes |
| ECMA-262 §10.2.1.2 OrdinaryCallBindThis ( func, calleeContext, thisArg ) | 5, 7-9 | base constructor binds at entry. Step 6 is the non-strict branch: outside core (LegacySemantics), and every core function is strict, so `[[Construct]]` never reaches it | the entry's JS base arm | ✓ | yes |
| ECMA-262 §10.3.2 [[Construct]] ( argList, newTarget ) | 1-2 | thisArg uninitialized; result is an Object | the entry's native arm | ✓ | yes |
| ECMA-262 §10.3.3 BuiltinCallOrConstruct ( func, thisArg, argList, newTarget ) | 11 | "If thisArg is uninitialized, the this value is uninitialized" | `call_construct_native`, the native arm | ✓ | yes |
| ECMA-262 §10.4.1.2 [[Construct]] ( argList, newTarget ) | 4-6 | bound args prepended; newTarget remapped when it is the bound function | `admit_construct` (unchanged) | ✓ | yes |
| ECMA-262 §10.1.14 GetPrototypeFromConstructor ( ctor, intrinsicDefaultProto ) | 2-6 | Object / non-Object (realm intrinsic) | helper phase 1 (NEW) | ✓ | yes |
| ECMA-262 §10.1.13 OrdinaryCreateFromConstructor ( ctor, intrinsicDefaultProto [ , internalSlotsList ] ) | 2-5 | — | helper phases 1-2 (NEW) | ✓ | yes |
| ECMA-262 §10.1.12 OrdinaryObjectCreate ( proto [ , additionalInternalSlotsList ] ) | 1-5 | — | helper phase 2 (NEW) | ✓ | no |
| ECMA-262 §7.3.1 MakeBasicObject ( internalSlotsList ) | 1-9 | the kind is the slot list, written once | helper phase 2 (NEW) | ✓ | no |
| ECMA-262 §13.3.7.1 Runtime Semantics: Evaluation | SuperCall 1-13 | step 6 Construct before step 9 BindThisValue; step 12 seam | `dispatch_super_inner` | ✓ | yes |
| ECMA-262 §9.1.1.3.1 BindThisValue ( envRecord, value ) | 2-4 | already initialized → ReferenceError | `dispatch_super_inner` | ✓ | yes |
| ECMA-262 §9.1.1.3.3 GetThisBinding ( ) | 2-3 | uninitialized → ReferenceError | `Op::PushThis`; the entry's derived return | ✓ | yes |
| ECMA-262 §9.4.3 GetThisEnvironment ( ) | 1-2 (2.1-2.5) | arrows skip to the enclosing function environment | arrow `captured_this` readers (E-4) | ✓ | yes |
| ECMA-262 §7.3.33 InitializeInstanceElements ( obj, ctor ) | — | seam only (Slice 2ab) | the entry; `dispatch_super_inner` | ✗ | yes |
| ECMA-262 §10.5.13 [[Construct]] ( argList, newTarget ) | 7-11 | trap undefined → `Construct(target, argList, newTarget)` (7.1); otherwise the trap is called (9); no receiver in either | future `#11-vm-proxy-reflect` caller of the entry | ✗ | yes |
| ECMA-262 §28.1.2 Reflect.construct ( target, args [ , newTarget ] ) | 1-5 | newTarget ≠ target | future caller of the entry | ✗ | yes |
| ECMA-262 §20.5.1.1 Error ( message [ , options ] ) | 1-2 | NewTarget undefined → the active function; creation before ToString | `error_entry` | ✓ | yes |
| ECMA-262 §20.5.6.1.1 NativeError ( message [ , options ] ) | 1-2 | same | `error_entry` | ✓ | yes |
| ECMA-262 §20.5.7.1.1 AggregateError ( errors, message [ , options ] ) | 1-2 | same | `error_ctor_impl` | ✓ | yes |
| ECMA-262 §23.1.1.1 Array ( ...values ) | 1-2 | NewTarget undefined → the active function; prototype `? Get` at step 2, then ArrayCreate | the `Array` constructor | ✓ | yes |
| ECMA-262 §20.1.1.1 Object ( value ) | 1-3 | creates from NewTarget only when it is neither undefined nor the active function | the `Object` constructor | ✓ | yes |
| ECMA-262 §22.1.1.1 String ( value ) | 2.a, 3-4 | Call mode returns a String; creation at step 4 after ToString | the `String` constructor | ✓ | yes |
| ECMA-262 §21.1.1.1 Number ( value ) | 3-4 | Call mode returns a Number; creation at step 4 after ToNumeric | the `Number` constructor | ✓ | yes |
| ECMA-262 §20.3.1.1 Boolean ( value ) | 2-3 | Call mode returns a Boolean; creation at step 3 | the `Boolean` constructor | ✓ | yes |
| ECMA-262 §21.4.2.1 Date ( ...values ) | 1, 6-7 | Call mode returns a string; creation after ToNumber | `native_date_constructor` | ✓ | yes |
| Web IDL §3.7.1 Interface object | 1.2, 1.7-1.9 | NewTarget undefined → TypeError (unchanged); create at step 1.8 after overload resolution at step 1.7; constructor steps at 1.9 (E-7); overridden constructor steps replace all of these (`HTMLElement`: the HTML §3.2.3 row) | the 62 Web IDL members and the 5 WebAssembly interfaces (`Module`, `Instance`, `Memory`, `Table`, `Global`: Web IDL interfaces under `[LegacyNamespace=WebAssembly]`) | ✓ | yes |
| Web IDL §3.8 Platform objects implementing interfaces | internally create 2-9 | newTarget undefined (step 2) / `? Get` and the non-Object fallback to the interface prototype of newTarget's realm (step 3: 3.2-3.3, phase 1); [[PrimaryInterface]] at step 8 | helper phases 1-2 (NEW) | ✓ | yes |
| WASM JS API 2 §5.10 Error Objects | 2.1 | `CompileError`, `LinkError`, `RuntimeError` implement the NativeError Object Structure, so they create as §20.5.6.1.1 steps 1-2 (active function in Call mode) | the Wasm error installer (`vm/host/wasm/errors.rs:127`) | ✓ | yes |
| Web IDL §3.12 Invoking callback functions | construct 1-11 | no receiver | `invoke_upgrade` → the entry | ✓ | yes |
| WHATWG HTML §3.2.3 HTML element constructors | 5, 9-16 | no definition for NewTarget → TypeError (step 5, which is also where Call mode throws: NewTarget undefined matches no definition); empty construction stack creates from NewTarget; else returns the stack element; step 11 fallback | `native_html_element_ctor`, `resolve_validated_prototype` | ✓ | yes |
| WHATWG HTML §4.13.5 Upgrades | upgrade 11.3-11.4 | constructResult ≠ element → TypeError | `invoke_upgrade` | ✓ | yes |
| WHATWG HTML §4.13.4 The CustomElementRegistry interface | define 14.1-14.2 | `? Get(constructor, "prototype")`; non-Object → TypeError: the only prototype check HTML has for a definition (S-2) | `invoke_upgrade` pre-read (S-2, O-1) | ✓ | yes |

Numbers: `webref heading` / `aoid` at this draft (§10 M-6). ECMA-262 rows are keyed by AO name where one
exists. `✗` rows are seams for future callers, not this program's behaviour. Breadth K = 4, M = 34, the
split-default range: the program is split (§4), and each per-PR plan carries its own map over its rows.
**Row → PR.** R-JS: the rows touching the entry and its callers, the this-binding sites,
`invoke_upgrade`, and helper phase 1. R-NAT core: helper phase 2, the *uninitialized* `thisArg` (the
flip), the nine ECMA-262 constructor rows. R-NAT host: the Web IDL, WebAssembly and HTML §3.2.3 rows.
Why R-NAT is one PR over two groups, and how it is assembled and reviewed: §4.5 (PM 675). The
`WASM JS API 2` label is the one `webref coverage-map` prints; `preflight.py` does not map it (a soft
warning), so that row's number was checked by `webref heading wasm-js-api-2 5` and its step by `webref body
wasm-js-api-2 error-objects`.

### §3.1 User-input touch audit

Every row marked "yes" is reachable from script: `new`, `super()`, a user getter on `prototype` (once
reachable, E-1), a custom-element definition and upgrade, and the constructor arguments. No row takes
input across a process boundary.

## §4. Per-PR split

### §4.1 Prerequisites (standalone, behaviour-neutral)

- **R-0a: the value.rs split** (#625, landed 11-11 as `439fa47c`; `vm/value.rs` 1176 → 967, `CallFrame` → `vm/frame.rs`).
  R-JS then edits `vm/frame.rs`.
- **R-0b: one GC frame-root enumeration** (NEW, VM P4). The live-frame block of `mark_roots`
  (`vm/gc/roots.rs:277-313`) and the suspended-frame block of `trace_work_list` (`vm/gc/trace.rs:335-372`)
  enumerate the same `CallFrame` fields (`this_value`, upvalues, `actual_args`, `new_instance`, `mode`,
  `home_class`, `pending_completion`); only `generator` differs. Extract one frame-root function used by
  both, before R-JS changes the field set. After #625 (it reads `vm/frame.rs`).
- **R-0c: split `vm/host/events_modern/data_transfer.rs`** (1226, L3) at its three interfaces:
  `DataTransfer` (`:1-803`), `DataTransferItem` (`:804-984`), `DataTransferItemList` (`:985-1226`) (§7):
  a pure move, no body changed (PM 750). The other candidate seam, moving the drag-data-store algorithms
  (HTML §6.11.2) out of `host/`, is a content move with a state-shape decision (`DataTransferState`), not
  a behaviour-neutral split: it is `#11-data-transfer-drag-data-store-layering`'s (owner L3, PM 752).
- **R-0d: move WebSocket protocol-list validation to `elidex-api-ws`, onto the one token predicate**
  (L3; §7). The seam of `vm/host/websocket.rs` is a layering one: the checks of WebSockets §3.1's
  constructor step 9 (no value twice; each value an element of a `Sec-WebSocket-Protocol` field, a
  `token` by RFC 6455 §4.1) are an engine-independent algorithm inside `host/` (`:496-523`; the
  predicate `is_valid_protocol_token_char`, `:527-558`). They move to
  `crates/api/elidex-api-ws/src/websocket.rs` as a protocol-list validator (NEW, `&[String] ->
  Result<(), String>`) beside `validate_ws_url` (`:80`); `parse_protocols` keeps the marshalling and maps
  the `Err` to SyntaxError, as the constructor does for `validate_ws_url`. No new `host/` file.
  **The validator mints no token predicate** (One issue, one way). The tree spells RFC 7230 §3.2.6
  `token` four times (M-10): `elidex-api-fetch`'s `is_valid_header_name` (`src/headers.rs:69`; public,
  engine-independent; a header name is the same production, Fetch §2.2.2), a host copy
  (`vm/host/headers/validation.rs:20`, `:43`), `is_valid_protocol_token_char` (deleted by R-0d) and
  `validate_method`'s matcher (`elidex-net`, `transport.rs:300-306`). **`elidex-api-ws` gains a
  dependency on `elidex-api-fetch`, and the validator calls `elidex_api_fetch::is_valid_header_name`**
  (PM 784 (2)): no new function (design first); the edge is acyclic (`elidex-api-fetch` has no
  dependency, M-10) and follows the spec relation (the constructor's step 12.1 connects through Fetch).
  A `token` predicate in `elidex-plugin` is rejected: it gives the crate of extension-point types a new
  responsibility, an HTTP grammar (Plugin-first). The host header copy and the `elidex-net` matcher are
  not R-0d's: they go to a slot the PM registers (PM 784 (2)).
- **Plan-review of the prerequisites**, each against the skill's "Skip OK" clauses
  (`.claude/skills/elidex-plan-review/SKILL.md`; "the mandatory trigger is not met" is not one). **R-0c:
  skipped** as a "trivial cleanup / rename PR" (a pure move); the reason goes in its landing memo.
  **R-0b** and **R-0d: a short plan memo and `/elidex-plan-review` each.** R-0b rewrites the root
  enumeration of every frame, where an omitted field is a use-after-free; R-0d is a cross-crate move
  with the dependency decision above and a SyntaxError surface that must not change. All three pass
  `/pre-push`. R-0b and R-0d keep their 11-24 targets and the 12-04 hard edge with the memo and the
  review ahead (PM 784 (3)); if a date would pass 11-26, it is reported to the PM (§9.1).

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
  captured by arrows like any `let` (uninitialized = TDZ hole). That candidate has a dependency the
  weighing counts: a captured binding has no TDZ today (`UpvalueState` is `Open` / `Closed`,
  `vm/value.rs:941-946`; a captured read emits no `Op::CheckTdz`: `#11-vm-upvalue-tdz-check`), and the
  uninitialized state is minted by slice-plan umbrella C's derivation, which that slot is ordered after.
  So it waits for C's child or mints the state itself (§5.2). The alternative, a frame status plus an
  environment link, has no such edge. The choice is made once, with S (`#11-vm-arrow-lexical-super`)
  and N (`#11-vm-arrow-lexical-new-target`), whose GetThisEnvironment reads the same thing.
- **Native arm: today's contract, centralised.** The entry's native arm allocates an `Ordinary` from
  GetPrototypeFromConstructor(newTarget) and passes it as `this` — what natives receive today, moved from
  `do_new` into one place, inside `with_call_mode`'s closure (E-9). Fallback: unreachable in this stage
  (newTarget for a native is a built-in or a class, both with an Object `prototype`); it uses the
  `%Object.prototype%` default with a `debug_assert`, and R-NAT gives each member its own default. The
  edge that would make it reachable is `#11-vm-proxy-reflect` (a newTarget whose `prototype` is not an
  Object reaching a native): it does not land between R-JS and R-NAT, where the default would be
  `%Object.prototype%` instead of the member's own (§9.1 step 11). R-NAT's flip deletes this allocation and
  its `debug_assert`.
- **Upgrade.** `invoke_upgrade` loses `receiver` and `pre_alloc`. Its Phase 2 read and Phase 3
  (`create_element_wrapper`, `set_wrapper_prototype`) stay as today until O-1 is ruled (S-2).
- **Already closed by R-JS** (not waiting for R-NAT): both receiver vectors of
  `#11-vm-native-ctor-receiver-promotion-admission`. A native now only ever receives a fresh `Ordinary`
  minted by the native arm: a second `super()` gets a new one (then ReferenceError), and the upgrade no
  longer passes the wrapper. The double-`super()` debug panic goes with them.
- **Wire #5, first half** (§4.6).
- **Family homes (the VM-P4 family).** The entry memo (`docs/plans/2026-07-vm-p4-es-language-completeness.md`)
  homes [C11] (§10.2.2 `[[Construct]]`) on a child of umbrella 2ab (the base-path field-initialisation
  timing) and a child of umbrella 3 (both call entry points), and [C13] (§13.3.7.1 SuperCall) on children
  of umbrellas 1a / 1b (argument layout, inline cache, the emit) and of umbrella 3 (the super-property emit
  shape). Slice-plan umbrella 3 (`docs/plans/2026-07-vm-p4-slice-plan.md`, row **3**) owns `[[HomeObject]]`:
  the frame field `home_class`, its move to the closure, and its threading through both call entry points.
  The split:
  - **R-JS takes** the this-binding axis of both clauses, the carve repo `:293` records ("carved rather
    than folded into Slice 2ab"): [C11] steps 3, 6.a and 12-17; [C13] steps 6-9; the seam after each bind.
  - **Stays with 2ab:** InitializeInstanceElements ([C11] step 6.b, [C13] step 12) (E-10).
  - **Stays with 1a / 1b:** [C13] step 4 (ArgumentListEvaluation) and `op_super_call_spread`'s argument
    layout and inline cache. R-JS changes what `dispatch_super_inner` does once the arguments are
    evaluated, not how they are laid out.
  - **Stays with umbrella 3:** the super-property emit shape, `[[HomeObject]]` on the closure, and
    `home_class` through both entries. R-JS edits the same two functions (`call_internal`,
    `push_js_call_frame`; §8.1) for `new_instance` only; `home_class`, its `None` in `call_internal` and
    its rooting stay as they are.
  - **Two edges to umbrella 3.** (1) R-0b folds the two frame-marking sites of `home_class`
    (`vm/gc/roots.rs:297`, `vm/gc/trace.rs:357`) into the one frame-root function, so umbrella 3's
    obligation to retire "the two frame-marking sites" reads one site afterwards. (2) The this-binding and
    umbrella 3's closure-carried home object are both reached through GetThisEnvironment's walk (S's AO).
    R-JS's plan decides only the this-binding, and shows that its representation leaves room for a home
    object beside it without a second capture mechanism; whether and how the home object moves stays
    umbrella 3's derivation.
  - R-JS's plan PR adds R-JS to the "Used by" cells of [C11] and [C13] in the entry memo (a VM-P4 family
    edit: the umbrella checker runs on it, CLAUDE.md "VM-P4 family"). The cells name this memo by a
    backticked path, not a link: a link to a memo outside the family turns the family-layout pin red
    (`docs/plans/test_vm_p4_family_layout.py`, (i)).
- **Slice 1aa / 1ac (children of umbrella 1a).** 1aa's plan is parked and not ratified (rev 4.1 on
  `vm-p4-slice-1a-plan`, `7777faef`: `docs/plans/2026-11-vm-p4-slice-1a-1aa.md`, base `eaa741b5`; a
  base bump is owed). **Order: R-JS first** (PM 784 (4)): R-JS is dated and on R-NAT's critical path, 1aa
  is parked and unratified, and 1aa's owed base bump absorbs R-JS. In 1aa's surface R-JS changes: its §3
  SuperCall rows (step 6, "unchanged" on `construct_synchronous(super_class, receiver, …, pre_alloc)`,
  loses both parameters; steps 7-9, ✗ with owner `#11-vm-derived-constructor-this-binding`, are closed);
  its anchors in `dispatch_super_inner` and `construct_synchronous`; and the [C13] "Used by" cell, which
  both edit (the later one merges). 1ac roots the `stack_slice` drains of `call_internal` and
  `push_js_call_frame` (`vm/interpreter_call.rs:368`, `:403`, `:605`, `:652`), in the frame literals R-JS
  takes `new_instance` out of (§8.1): other fields; the later one rebases.

### §4.3 R-NAT core and ECMA-262 chapter (VM P4)

- **Helper module** (NEW, core, ungated, beside `vm/construct_admission.rs`):
  - phase 1, `get_prototype_from_constructor(new_target, default_proto) -> Result<ObjectId, VmError>`
    (born in R-JS, §4.2): ECMA-262 §10.1.14 and Web IDL §3.8 step 3 (3.2-3.3), which have the same shape;
  - phase 2, `alloc_with_kind(proto, kind) -> ObjectId`: OrdinaryObjectCreate / MakeBasicObject; the kind,
    and for a `HostObject` its brand (L3 brand-check PR-1, Web IDL §3.8 step 8), written once;
  - `NativeContext::create_from_new_target(default_proto, kind)`: both phases, for Construct mode only.
    NewTarget is present there: by the dispatch gate for a `ConstructorOnly` member, by the member's own
    Call-mode return for `Date` / `String` / `Number` / `Boolean`. It has no Call-mode branch;
  - the active-function entry (NEW; its name is the per-PR plan's): for the constructors whose step 1
    substitutes the active function (table below), and only for them. Phase 1 runs on NewTarget when
    present. In Call mode the active function is the built-in itself, whose `prototype` is a non-writable,
    non-configurable data property holding the default (§23.1.2.5, §20.5.2.2, §20.5.6.2.1, §20.5.7.2.1), so
    the `? Get` is unobservable and the entry takes the default directly;
  - every NEW function of the program (the entry, these helper functions, R-0b's frame-root function,
    R-0d's validator) carries a docstring naming the AO or algorithm it implements: ECMA-262 by AO name,
    numbers from `webref`;
  - wires #1 / #3 and this module: §4.6.
- **Call mode per constructor** (NewTarget undefined; the ECMA-262 28, steps from `webref body`):

  | Constructors | Step | Call-mode behaviour | Helper in Call mode |
  |---|---|---|---|
  | `Array` (§23.1.1.1) | 1 | newTarget = the active function; creates (ArrayCreate) | the active-function entry |
  | `Error` (§20.5.1.1), the 5 NativeErrors (§20.5.6.1.1), `AggregateError` (§20.5.7.1.1) | 1 | newTarget = the active function; creates at step 2 | the active-function entry |
  | `Object` (§20.1.1.1) | 1-3 | creates from NewTarget only when it is neither undefined nor the active function (step 1.a); otherwise OrdinaryObjectCreate(%Object.prototype%) or ToObject(value) | never reached: phase 1 runs only under step 1.a (NewTarget is another constructor, so Construct mode) |
  | `Date` (§21.4.2.1) | 1 | returns ToDateString(now); no object | never reached |
  | `String` (§22.1.1.1) | 2.a, 3 | returns a String (SymbolDescriptiveString for a Symbol); no object | never reached |
  | `Number` (§21.1.1.1) | 3 | returns the Number; no object | never reached |
  | `Boolean` (§20.3.1.1) | 2 | returns the Boolean; no object | never reached |
  | `Promise` (§27.5.3.1), `ArrayBuffer` (§25.1.4.1), `DataView` (§25.3.2.1), the 11 TypedArrays (§23.2.5.1) | 1 | TypeError | never reached |
  | `%TypedArray%` (§23.2.1.1) | 1 | TypeError in both modes | never reached |

  1 + 7 + 1 + 4 + 14 + 1 = 28. The host side has one more active-function group, the three WebAssembly
  error classes (NativeError Object Structure, §3); every Web IDL constructor throws at step 1.2, except
  `HTMLElement`, whose overridden constructor steps throw at HTML §3.2.3 step 5 (§3).
- **The flip.** The entry's native arm passes *uninitialized* (`undefined`); the last pre-allocation goes.
- **ECMA-262 members (28)**: each creates at its creation step. Step-position table, one row per member
  (constructor, spec step of creation, today's site, feature row). Inputs for its rows: L3 F1 / F1b /
  typed arrays / F2 (ECMA part), and PR-0A's placement table (its §4.5.6 DoD item 2) as a cross-check.
  Rows of note: `Array` (ArrayCreate, not OrdinaryCreateFromConstructor), `String` (StringCreate),
  TypedArrays (`construct_typed_array`; AllocateTypedArray before the iteration), `Promise` (creation
  before the executor), Error family (`error_entry` / `error_ctor_impl`, AggregateError's SD-3 order
  unchanged), `Date` (after ToNumber; the `store` setter becomes a payload write).
- **Deleted.** `ensure_instance_or_alloc`, `promote_to_*`, the 41 `.kind =` lines; wire #5 complete.
- **Cells driven from Rust (required of R-NAT's per-PR plan).** Three behaviours need a newTarget that
  script cannot pass before Reflect.construct exists. Each gets a cell that calls the entry,
  `Construct(ctor, args, newTarget)`, from Rust (the route P0a-1's planned cell
  `rl_cp1_entry_failure_no_callee_effect` takes, E-9); a code read is not their acceptance. (i) A
  newTarget whose `prototype` is not an Object, and one whose `prototype` `Get` throws: the member's
  own default; the throw, with nothing created. It closes
  `#11-vm-get-prototype-from-constructor-fallback`. (ii) S-1. (iii) The `HTMLElement` sync path: a
  throwing `Get` spawns no entity (E-7). The cells on `#11-vm-proxy-reflect` (§5.2) are additional.

### §4.4 R-NAT host chapter (L3): required contents

Written by L3 after this umbrella is ratified (the PM tells L3). This section lists what the chapter must
contain, not its content. Rev 1.2's item 3 was such content (a class table with write orders and
windows); R2 found it wrong (G2, G3) and it is removed (`e47adadc` has it, as a claim).

1. **The 70 rows**, by family, each with: the interface, Web IDL (or WebAssembly JS API) creation step,
   today's site, the helper call, the feature row, and three more columns. **Outside writes** (item 3),
   or none. **Layering**: the engine-independent crate that owns the member's algorithm (argument
   validation, parsing), or "marshalling only". An algorithm found in `host/` (CLAUDE.md, Layering
   mandate) gets a disposition in its row: a prerequisite move (as R-0d; it needs a §9.1 row, so it goes
   to the PM before the chapter's plan-review) or a slot candidate reported to the PM. **State shape**:
   none; an ECS component on its entity; an entity-keyed side store (CLAUDE.md's side-store rule:
   exception (a) or (b), else a slot candidate); a table keyed by the object's `ObjectId` for a member
   with no entity (outside that rule, which is about per-entity state: recorded, no verdict); shared
   state (exception (b)). R-NAT's hunks move no algorithm and no state. Population: §1.1's host 70
   (the 8 WebAssembly members: §3); re-run `H/scripts/ctor_census.py` at the chapter's base.
2. **Each member's order against E-7**: where its argument conversion, phase 1, the validation of its
   constructor steps, each outside write, phase 2 and the link sit. Conversion before phase 1 also means
   no temporary rooting is needed while user `valueOf` runs.
3. **The outside-write census (E-7).** Population, by property: every member whose creation step writes
   per-instance state outside the object it returns, taken at the chapter's base by reading each of the
   70 bodies; no spelling is the population (`_states.insert(` is a lower bound, M-9). Per member: every
   outside write, the throw window (what can throw, or run user code, between the first outside write
   and the link), and a cell or a code-read verdict that the window is empty. A member that does not
   meet E-7 today is fixed in R-NAT and gets a cell.
4. **Shared helpers**: `create_fresh_event_object` (`vm/host/events.rs:299`; its Call-mode branch serves the
   4 VM-internal dispatchers), `build_ui_event_instance`, `build_event_subclass_instance`, `construct_rect`
   (`DOMRect`'s side store insert after allocation; PR-0A review IMP-4), `build_response_instance`, and
   `instantiate_module`, which also serves the non-constructor `WebAssembly.instantiate`
   (`vm/host/wasm/namespace.rs:578`) and holds CP-3b as its first statement (§8).
5. **`HTMLElement`.** Creation stays through the wrapper identity map, and `resolve_validated_prototype`
   folds onto phase 1 (S-1). The chapter states the sync path's order under E-7 (today the entity spawn,
   `html_element.rs:236`, and the wrapper, `:248`, precede the prototype `? Get`, `:249`; HTML §3.2.3
   step 9.1 has that `? Get` first), with the Rust-driven cell of §4.3. `%TypedArray%`'s body is unchanged.
6. **`DataTransfer` / `Touch`** (S-3): the subclass prototype, with cells.
7. **`HostObject` hunks** for the PM at plan time (PM 642 (a)): the Worker / OffscreenCanvas allocations
   and their brand.
8. **Touch-time verdicts** for its files over 1000 lines: R-0c, R-0d (§7), re-measured at the chapter's
   base.
9. **L3's cells A-1 … A-9** re-expected under E-2 / E-6 / E-11, and the brand-check §6.2 cell that PM 664
   re-points (§5.1).

### §4.5 How R-NAT is assembled (proposal)

- **One squash, one PR.** Stacked branches merged back-to-back are rejected: between the two merges main
  would hold two native contracts (the strangler PM 675 forbids), and "no release in between" cannot be
  enforced.
- **Branches.** `rnat-core` (VM P4) from main after R-JS: helper phase 2 and the ECMA-262 chapter.
  `rnat-host` (L3) based on `rnat-core`: the host chapter. The flip commit (VM P4) after the host commits.
  The PR is opened from `rnat-core`. No two lanes push to one branch. Ownership, pull-in and the
  per-commit record: §4.5.1.
- **Commit order.** (1) helper phase 2; (2) ECMA-262 members; (3) host members (L3, one commit per family);
  (4) the flip, the deletions and wire #5. Commits 2 and 3 are green on their own: a member that creates
  its own object is valid under the pre-allocation contract (an Object return wins), so every commit
  builds and passes; only the squash reaches main.
- **Review.** One `/elidex-review` and one Codex pass on the whole PR; L3 reviews the host commits and VM
  P4 the rest. When `rnat-core` moves, nothing is rebased (§4.5.1).

### §4.5.1 Branch ownership (PM 687 (3))

- **Owner and merger.** VM P4 owns the R-NAT PR: it opens it from `rnat-core` and squash-merges it after
  the PM's "merge at head <sha>" for that head. L3 neither opens nor merges it.
- **One writer per branch.** `rnat-core`: VM P4 only. `rnat-host`: L3 only, created from `rnat-core`.
  Neither lane pushes to the other's branch; each works in its own worktree (CLAUDE.md, parallel sessions
  and worktree isolation).
- **Pull-in: merge, not cherry-pick.** VM P4 brings L3's commits in on `rnat-core` with
  `git merge --no-ff origin/rnat-host`, at a sha L3 names. Reason: the hooks forbid amend and force-push,
  so no branch of R-NAT is ever rewritten, and the pull-in must keep L3's shas. A cherry-pick copies L3's
  commits under new shas: `rnat-host` and `rnat-core` then carry the same change twice, and L3's next sync
  from `rnat-core` re-applies or conflicts with its own copies; and the per-commit record below, keyed by
  sha, would leave the PR's copies unrecorded. Merge commits never reach main (the PR is squashed).
- **Pull-in order.** `rnat-core`'s own commits (helper phase 2, ECMA-262 members) → the merge of
  `rnat-host` (L3's host commits, one per family) → the flip commit (the flip, the deletions, wire #5),
  which VM P4 makes only after L3 reports `rnat-host` complete at a named sha. Host commits L3 adds later
  (review fixes) come in by a further merge; the record covers them at their own shas and at the merge.
- **When `rnat-core` moves after L3 has branched** (a core fix, or main): nobody rebases. Main reaches
  `rnat-core` by VM P4 merging `origin/main` into it; `rnat-core` reaches `rnat-host` by L3 running
  `git merge origin/rnat-core` on `rnat-host`, before its next host commit or when VM P4 asks. The lane
  that makes a merge resolves its conflicts; a conflict in the other lane's chapter is resolved toward that
  lane's side and reported to it, and that lane re-checks it in the record.
- **Ordering edges to L3's own work.** R-0c (the `data_transfer.rs` split) and R-0d (the WebSocket
  validator move), both L3's, land on main before `rnat-host` is created (§9.1 steps 8 / 9), and
  `rnat-host` is created from an `rnat-core` that contains them (if `rnat-core`
  was cut earlier, VM P4 first merges `origin/main` into it). The host chapter (§4.4) stays L3's to author;
  VM P4 owns the PR, not the chapter.
- **Intermediate green.** Every commit the squash covers — L3's, VM P4's and the merge commits — is checked
  at its own sha in a scratch worktree `S`, by a scripted run (not `git rebase -x`, which rewrites the
  history it checks and drops merges):

  ```sh
  S=<scratch dir>; git fetch origin && git worktree add --detach "$S" origin/rnat-core
  for c in $(git rev-list --reverse --topo-order origin/main..origin/rnat-core); do
    git -C "$S" checkout -q --detach "$c" && ( cd "$S" &&
      env -u RUSTUP_TOOLCHAIN cargo clippy -p elidex-js -- -D warnings &&
      env -u RUSTUP_TOOLCHAIN cargo clippy -p elidex-js --features engine -- -D warnings &&
      env -u RUSTUP_TOOLCHAIN cargo clippy -p elidex-js --all-targets --features engine,compat-webapi -- -D warnings &&
      env -u RUSTUP_TOOLCHAIN cargo test -p elidex-js --features engine,compat-webapi )
    echo "PER-COMMIT $c rc=$?"
  done 2>&1 | tee "H/logs/rnat-per-commit-$(git rev-parse --short=12 origin/rnat-core).log"
  ```

  Rows: clippy (which includes `cargo check`) on the default row (lib only, as CI's default-features step),
  on `--features engine`, and on `--features engine,compat-webapi` with `--all-targets`; tests on
  `--features engine,compat-webapi` (elidex-js's `--all-features`, the row CI tests). Verdict: the number
  of `PER-COMMIT` lines equals `git rev-list --count origin/main..origin/rnat-core`, and none has a
  non-zero `rc`. **Recorded** in the PR body: one table row per sha (sha, the three rows, rc) and the log
  path. Each new head re-runs the loop over the whole range; because nothing is rewritten, the shas already
in the record are the same commits (only new shas add rows). The
  PM's "merge at head <sha>" is given against a record whose last row is that head.

### §4.6 Wire #5 (`.claude/tools/native-ctor-guard-trip-wire.sh`)

Property: **an object's kind variant is written only in the `Object { kind: … }` literal at allocation**,
and no construct path carries a receiver. Born in R-JS (`pre_alloc_instance`, `new_instance` → 0),
completed in R-NAT (`ensure_instance_or_alloc`, `promote_to_\w+\(`, and assignments of an `ObjectKind::`
value through `.kind =` or a `&mut` deref → 0, PR-0A review MIN-2), with a control mutant in the same
commit.

**Wires #1 / #3 and the helper module.** Their property is "no constructor body rejects a bare call
itself" (the dispatch gate does, `CallShape::ConstructorOnly`). The helper's active-function entry reads
the call mode for a different purpose: it substitutes, and never throws for the mode. So it is outside
that property whatever its spelling, and the wires say so instead of relying on the spelling: R-NAT's wire
commit excludes the helper module's file from #1 and #3 by path, with that reason in the script (the
shape of wire #4's `vm/error.rs` exclusion), and wire #5 adds the complement inside that file, by
property: no `Err` whose condition reads the call mode, whatever builds it (a `VmError::type_error`
literal, or a builder call such as `VmError::illegal_constructor(`, `vm/error.rs:108`). A message
spelling is not the test: the gate's own message is "Please use the 'new' operator"
(`vm/interpreter_call.rs:80`). A control mutant for each direction lands in the same commit.

### §4.7 What must not ship

- Natives migrated one at a time while the native arm still pre-allocates ("host first, ECMA-262 later").
- Any receiver-admission gate (`AdmittedReceiver`-style), landed or planned to be deleted.
- R-NAT before R-JS: it would spread E-4's arrow breakage from three parents to every native parent until
  R-JS lands (PM 675).

## §5. Slot homing

Slot ids are exact matches in the repo ledger (`docs/plans/2026-07-vm-p4-slot-ledger.md`) or the PM ledger
(`project_open-defer-slots.md`), as each row says. PM-ledger line numbers were re-measured live on
2026-11-15 (`grep -n` on the slot id); the ledger moves, so the PMO matches on the id, not the number.

### §5.1 Closed

| Slot | Ledger | Closed by | Note |
|---|---|---|---|
| `#11-vm-native-ctor-receiver-promotion-admission` | PM `:270` | R-NAT (the class) | R-JS already removes both vectors (§4.2). The PM 660 / 664 facets (`:271`; the 39 / 37 counts) concern a guard that is no longer written. |
| `#11-vm-get-prototype-from-constructor-fallback` | PM `:268` | R-NAT | R-JS discharges the reachable facet (JS base, E-1); R-NAT gives every native its default, accepted by the Rust-driven cell (i) of §4.3. |
| `#11-vm-derived-constructor-this-binding` | repo `:293` (PM pointer `:1326`) | R-JS (the derivation and its children) | the four regressions and base controls are R-JS's acceptance. PM 660 facet (PM `:1327`): the Worker / OffscreenCanvas guard is not written, because R-JS gives natives fresh objects. PM 664 facet: the brand-check §6.2 cell expects ReferenceError (`super()` path, R-JS) and TypeError (upgrade path, step 11.4). |

### §5.2 Edges changed

| Slot | Ledger | Change |
|---|---|---|
| `#11-ce-upgrade-returned-object-typeerror` | PM `:1045` | not closed; its reachable set grows (every re-prototyped upgrade reaches step 11.4); its trigger fires on R-JS (`upgrade.rs`); its text's "step 10.4" is 11.4 today, and P-11's 11 crate cites ride on its fix (L3; 8 by PM 749, 3 by PM 784 (5)) |
| `#11-vm-arrow-lexical-super` (S), `#11-vm-arrow-lexical-new-target` (N) | repo `:287`, `:286` | S already waits on the derived-this derivation's `[[ThisBindingStatus]]` mint; R-JS mints it, decided with S and N (§4.2) |
| `#11-step9-class-extras` (Slice 2ab; umbrella 3 adopts the same slot) | repo `:321` | R-JS creates the InitializeInstanceElements seam (E-10); 2ab fills it. Umbrella 3 is untouched, with two edges (§4.2, family homes) |
| `#11-vm-proxy-reflect` (slice-plan row 10a) | PM `:330` | unblocked by R-NAT, and not before it (§4.2, the R-JS-stage fallback): a caller of the single entry, no receiver. It adds script-level cells for the three behaviours §4.3's Rust-driven cells accept (the non-Object and throwing `prototype`, S-1, the `HTMLElement` sync path); R-NAT's acceptance does not wait for them |
| `#11-vm-typed-array-create-construct` | PM `:249` | unblocked: TypedArrayCreateFromConstructor calls the entry |
| Array.from / Array.of (row 9a), PromiseResolve(C) (row 9da) | PM `:248` | unblocked the same way |
| `#11-vm-function-constructor-global`, `#11-vm-map-set-collections`, `#11-vm-regexp-constructor-and-flags`, `#11-vm-float16array-global`, `#11-vm-weakref-finalization-registry` | repo `:318` / PM `:328` / PM `:329` / repo `:315` / repo `:317` | each future constructor is born on (R): it creates through the helper |
| `#11-vm-native-error-prototypes` | PM `:250` | E1 rebases onto R-NAT (§6) |
| `#11-error-object-builtintag-dual-representation` | PM `:228` | unchanged owner (E1); E1 writes `ObjectKind::Error` through the helper |
| `#11-vm-value-reexport-shim-retirement` | PM `:1544` | placement §9.2 |
| `#11-vm-upvalue-tdz-check`; slice-plan umbrella **C** | repo ledger; `2026-07-vm-p4-slice-plan.md` row C | a new edge, if R-JS's plan picks the upvalue representation for the this-binding: that candidate needs the uninitialized upvalue state C's derivation mints and this slot is ordered after (§4.2). The R-JS plan records which of the two owns the state, or that the edge does not arise |
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
  step 11.4. The read is host-side marshalling (`vm/host/custom_elements/upgrade.rs`); the crate
  `elidex-custom-elements` is not touched (§8.4). Home: R-JS, subject to O-1: the read is the only
  input of Phase 3's `set_wrapper_prototype(…, proto_id)` (`:245`), D-17b §6's write. **Rev 1.3 keeps
  `:245`, and so the read, as they are today** (PM 754); R-JS removes only the receiver, and every S-2
  site of this memo is conditional on O-1. Phase 3's `create_element_wrapper(entity)` (`:244`) stays
  under any ruling: `native_html_element_ctor`'s upgrade branch (`get_cached_wrapper(entity)`,
  `html_element.rs:199`) and the step 11.4 SameValue test (`upgrade.rs:321`) need the wrapper.
  - **O-1 (open; with the user (PM 784 (1)); the PM's recommendation is to retract (HTML's upgrade reads
    no prototype; the prototype changes at §3.2.3 step 14 inside `super()`)): S-2 against D-17b §6.** (1)
    D-17b §6, the "pre-publication invariant": the wrapper's `[[Prototype]]` is `constructor.prototype`
    before the construction stack is pushed, so a synchronous observer between that point and the
    `HTMLElement` constructor's own write (a DOM walker, a construction-stack peek) sees it. Recorded in
    D-17b's plan memo (PM memory `m4-12-pr-d17b-html-element-constructor-base-plan.md` §5 / §6; #232
    `e30722f4`; its R4 removed the write, R5 restored it), in comments (`upgrade.rs:210`, `:233-243`;
    `html_element.rs:22`, `:189-190`), and named by two tests (`vm/tests/tests_custom_elements.rs:904`,
    `:919`) that read the prototype after the upgrade, not in that window. (2) HTML has no such write:
    *upgrade an element* reads no `prototype`; the element's prototype changes at §3.2.3 step 14, inside
    `super()`. (3) S-2 is fixed only when the read's two failure exits are gone, and `:245` needs a
    `proto_id`: removing the read retracts §6; keeping §6 keeps a pre-construct `Get`. Rev 1.3 decides
    neither.
- **S-3 `DataTransfer` / `Touch` ignore the subclass prototype.** Both allocate with the fixed interface
  prototype (`vm/host/events_modern/data_transfer.rs:332-341`, `vm/host/events_modern/touch.rs:327-335`),
  so `class T extends Touch {}; new T(init) instanceof T` is false. Home: R-NAT host chapter.

### §5.4 Ledger edits (listed, not made here)

**For the PMO, at ratification** (PM ledger):

- PM `:270`: owner "native construct protocol program" already reads R-NAT (PM 675); add "R-JS removes
  both vectors". PM `:268`: add "R-JS: reachable facet".
- PM `:271` and PM `:1327` (the PM 660 / 664 facets of the admission slot and of the derived-this pointer)
  still say that brand-check PR-1 closes Worker / OffscreenCanvas with a guard ("`this` must be an
  unminted Ordinary, else TypeError"). §5.1 retracts that guard, so both are restated: "no guard is
  written; R-JS gives a native a fresh object and R-NAT creates from NewTarget", keeping PM 664's cell
  expectation (ReferenceError on the `super()` path, TypeError at upgrade step 11.4).
- PM `:1045`: "step 10.4" → "step 11.4"; add P-11's 11 crate cites to the fix (L3, PM 784 (5)).
- PM `:1320` points `#11-vm-class-instance-field-init` at the repo ledger, where `:386` records it as
  withdrawn; re-point to `#11-step9-class-extras` (Slice 2ab).
- PM `:330`: add "after R-NAT" and the inherited cells (§5.2).
- **Re-eval dates that §9.1 passes** (the slot's date is earlier than the day the schedule acts on it;
  the new date is proposed, the PM sets it): PM `:1326` (derived-this), `:1311` / `:1312` (N / S), `:1045`
  and `:289` (`#11-step9-class-extras`) carry 2026-12-01, and R-JS lands 12-05 → 12-05; PM `:250`
  (`#11-vm-native-error-prototypes`) carries 12-05, and `#11-error-object-builtintag-dual-representation`
  carries 12-01 (PM 489; it closes on E1's landing), and E1 follows R-NAT (12-15) and Slice F → Slice F's
  date for both; PM `:1544` (shim retirement) carries 12-15, and its PR is 12-16 → 12-16.

**For VM P4, in this umbrella's plan PR (PM 787; it ratifies the R-JS / R-NAT terms, so the row is written once, where its terms land):** repo `:293`. The row today holds
the carve, the four regressions with their base controls, the three axes, and the PM 374 severity
paragraph (the double-`super()` panic, and why a guard before Construct is wrong). Its trigger cell reads
"with umbrella 2ab's derivation … or the next call-frame PR" (`call_internal` / `push_js_call_frame`). It
has no PM 660 / 664 facet: PM 665's facet-1 wording (the PR-0A admission helper) was never written
there, and PR-0A is withdrawn (PM 675). The PR writes, in R-JS / R-NAT terms: closed by R-JS (this
umbrella); the PM 660 facet as "no guard is written" (as above); the PM 664 facet as the brand-check
§6.2 cell's expectation; the trigger's "next call-frame PR" resolved to R-JS (PM 665 / 687 had put it in the default-row rustdoc PR, before this umbrella; PM 787 moves it here). It is a VM-P4 family edit
(the umbrella checker runs on the entry memo). The repo ledger has no row for
`#11-vm-native-ctor-receiver-promotion-admission` (`grep -c` → 0), so PM 665's owner edit has no repo site.

**For VM P4, in R-JS's plan PR:** the "Used by" cells of [C11] / [C13] (§4.2, family homes).

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
| §0.3 / §1.2 / §5 upgrade step numbers (`:245`, `:247`, `:755`) | 10.3 / 10.4 | 11.3 / 11.4 (P-11) |

## §7. 1000-line touch-time verdicts

`wc -l` at `4b723d4b` of every file R-JS or R-NAT touches (`H/logs/r-touched-files-4b723d4b.txt`, 73
files); the files over 1000:

| File | Lines | Touch | Verdict |
|---|---|---|---|
| `vm/mod.rs` | 2797 | one `mod` line (the entry / helper module) | **no seam for this touch.** The file is the `VmInner` struct (`:188-2790`, 351 fields); its decomposition is a design program with its own slot, `#11-vm-inner-substruct-decomposition` (PM `:1441`, PM 517), not a touch-time split. A one-line declaration does not fire it (PM 654 Q-14 reasoning). |
| `vm/gc/trace.rs` | 1255 | R-JS: the suspended-frame `new_instance` (`:350`) and arrow `captured_this` (`:255`) | **real seam: R-0b** (one frame-root enumeration shared with `vm/gc/roots.rs`), a standalone prereq. The rest is `trace_work_list`'s match over `ObjectKind` (a flat case table, exempt); the file stays over 1000 after R-0b and that remainder is the exempt table. |
| `vm/host/events_modern/data_transfer.rs` | 1226 | R-NAT: the `DataTransfer` constructor (S-3) | **real seam: R-0c** (three interfaces, three files), a standalone prereq before R-NAT's host commits. PM 654 Q-14 ruled a one-`..` mechanical ripple out of scope; R-NAT's hunk is a content change to the constructor, so that ruling does not cover it. |
| `vm/value.rs` | 1176 | R-JS: `FunctionObject::captured_this` (`:598`) | **discharged by R-0a** (967 after #625). |
| `vm/dispatch.rs` | 1104 | R-JS: `Op::PushThis` (`:738`), `complete_inline_frame` (`:1087`) | **no prereq split.** The file is `run()`'s opcode match (exempt, per the touch-time record) plus `complete_inline_frame` (17 lines), which R-JS rewrites and moves into the entry module (net −17). |
| `vm/host/websocket.rs` | 1001 | R-NAT: the constructor (`:329`) | **real seam, and it is a layering one: R-0d.** The protocol-list validation (`:496-523`, 28 lines) and its token predicate (`:527-558` and the blank after it, 33 lines) are an engine-independent algorithm; the validation moves to `elidex-api-ws` and the predicate is deleted for `elidex_api_fetch::is_valid_header_name`, a new `elidex-api-ws` → `elidex-api-fetch` edge (§4.1, PM 784 (2)), not moved to a new `host/` file (a `websocket/construct.rs` would keep the algorithm in `host/`, against the Layering mandate). Measured: 1001 − 61 + the call that replaces them (about 4 lines) ≈ 944, under 1000, so no host-side split follows. R-0d's PR re-measures; if the file is over 1000 at the host chapter's base, the chapter gives a new verdict. PM 667's Q-16 (no split for a mechanical one-hunk change) was for PR-0A; R-NAT's hunk is a content change to the constructor. |
| `vm/object_kind.rs` | 1645 | only if R-JS changes `FunctionObject`'s home or the HostObject variant changes | conditional; if touched, the per-PR plan gives the verdict (PM 642 (a): a split is a standalone prereq ordered by the PM) |
| `vm/tests/tests_custom_elements.rs` | 1321 | R-JS: one comment line (`:474`, P-11); new cells go to a new test file | **no seam for this touch**: a one-line cite fix, the reasoning of the `vm/mod.rs` row |

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
| `call_internal` (`vm/interpreter_call.rs:270`) | P0a-1 cites its async branch (`p0a1.md:278`, `interpreter_call.rs:367-397`); PR-1 cites it (`pr1.md:353`) | R-JS: its three `new_instance: None` frame literals (`:385`, `:420`, `:469`); `home_class` stays (§4.2, family homes) | P0a-1 | R-JS. `:385` is inside the async branch P0a-1 cites: one field of its frame literal, not its control flow, so P0a-1's reading ("builds the initial frame … without pushing a frame") stays true. PR-1 re-reads its anchors at its plan-review |
| `push_js_call_frame` (`vm/interpreter_call.rs:505`) | PR-1 names it only in its split verdict for `vm/interpreter.rs` (`pr1.md:353`, the call-op dispatch half); it places no probe in it | R-JS: the `new_instance` parameter (`:511`) and its frame literals (`:619`, `:671`, `:718`) | R-JS | PR-1 re-reads its anchors at its plan-review |
| `instantiate_module` (`vm/host/wasm/instance.rs:151`) | P0a-1 CP-3b: `check_entry` as the first statement | R-NAT: the receiver parameter and `.kind =` (`:188`) | P0a-1 | R-NAT |
| `invoke_upgrade` (`upgrade.rs:129`) | P0a-2b reads two `Err` arms (`p0a2b.md:175-176`, `:225`): D-3, the `prototype` read (`:216`), and D-4, `construct_synchronous` (`:277`) | R-JS: the call's arguments change (D-4's `Err` arm stays). S-2: if O-1 rules that the read goes, D-3's site goes with its two failure exits | R-JS | P0a-2b re-checks D-4's anchor, and drops D-3's row only under that ruling |
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
  entry); `create_element_wrapper` (`:244`), the step 11.4 SameValue test and the construct's `Err` arm
  are unchanged; the pre-construct `prototype` read (`:216-222`) and `set_wrapper_prototype` (`:245`)
  are S-2's, unchanged until O-1 is ruled; its stale step cites are fixed (P-11).
- `native_html_element_ctor` (`vm/host/custom_elements/html_element.rs:107`), R-NAT host chapter: the
  construction-stack return (steps 12-16) is unchanged; its two `resolve_validated_prototype` calls
  (`:204`, `:249`) go through the helper's phase 1 (S-1); the sync path's order is §4.4 item 5's. A
  Layering finding in this function is dispositioned by §4.4 item 1, never as a hunk of R-NAT.
- `resolve_validated_prototype` (`html_element.rs:276`), R-NAT host chapter: folded onto phase 1 (S-1).
- The engine-independent crate `elidex-custom-elements` is not touched by R-JS or R-NAT: `prepare_upgrade`,
  `enter_constructor`, `finalize_success`, `finalize_failure`, the construction stack and
  `spawn_custom_element_entity` keep their bodies and signatures (the host changes only when it calls the
  last one). The upgrade algorithm stays there; S-2's read is host-side. Its stale cites: L3 (P-11).
- Untouched: `invoke_upgrade`'s callers `invoke_one` (`vm/host/custom_elements/flush.rs:100`), the
  creation path (`creation.rs:238`) and the `upgrade()` walk (`lookup.rs:183`); `flush_ce_reactions`
  (`flush.rs:39`); `invoke_callback` (`flush.rs:156`; lifecycle callbacks are calls, not constructs);
  `drain_reactions` (`crates/script/elidex-js/src/engine.rs:314`); `drain_microtasks` and
  `drain_queue_pass` (`vm/natives_promise.rs:274`, `:311`). R-NAT edits `native_promise_constructor` in the
  same file (`:712`), a different function.

## §9. Placement and dates

The dates in §9 are canonical for this program (PM 687 (1)): any other mention of a date (in this memo, a
per-PR plan or a lane SSoT) defers to them.

### §9.1 R-JS and R-NAT (proposal)

Order, with the reason for each edge (dependency or regression):

1. value.rs split (#625) — **landed 11-11** (`439fa47c`). R-0a; hard edge for R-JS.
2. PR-A1 — **landed 11-13** (#628 `c0578223`). No shared function (assessment Q3, FP-3 of the PR-0A
   review).
3. R-0b frame-root split — **11-24** (PM 688). After this umbrella's plan-review, so it is implemented
   against the reviewed umbrella; after #625 (it reads `vm/frame.rs`); before R-JS's implementation (both
   copies change), which starts 11-28. 11-24 holds while the umbrella is ratified by 11-23; if it is
   later, R-0b is the day after the ratification, still ahead of R-JS's implementation (step 6).
   A short plan memo and `/elidex-plan-review` first; dates as §4.1 (PM 784 (3)).
4. default-row rustdoc PR — **11-25** (unchanged). Shares only the `vm/inner.rs:14` doc link. Repo-ledger
   `:293` is not its: this umbrella's plan PR writes it (PM 787; §5.4).
5. P0a-1 implementation — **11-28** (PM 687; the stack-safety program's date). Before R-JS:
   P0a-1's ratified plan cites `call_construct_native` / `construct_synchronous` as they are today, and
   R-JS first would change a ratified surface (a re-review); P0a-1 first leaves R-JS a mechanical rebase of
   one cell.
6. **R-JS — lands 12-05 (estimate).** Its per-PR plan window opens when this umbrella is ratified (the
   PM's go): a condition, not a date (PM 737). Estimate, for a ratification by 11-17: plan written 11-17 …
   11-19, plan-review 11-20 … 11-24, implementation 11-28 … 12-04. The implementation starts at the later
   of P0a-1's landing (11-28, step 5) and the end of the R-JS plan's own review (seven days after the
   umbrella's ratification), so 11-28 holds while the umbrella is ratified by 11-20. Each day past 11-20
   moves this row and steps 8-12 by a day; no date moves at the estimate. After P0a-1 (above); before
   R-NAT (PM 675, E-4); before P0a-2a's PR, so P0a-2a's measured report sees the final construct entries.
7. PR-D — **12-04** (unchanged; doc links only; whichever lands second rebases trivially).
8. **R-0c** (the `data_transfer.rs` split, L3; a pure move) — **targets 11-21, hard edge 12-04 (PM
   750).** The edge: on main before `rnat-host` is created (§4.5.1; 12-08). By 11-21 it precedes the
   host chapter (from 11-26), which then cites post-split anchors. It shares no file with R-JS.
9. **R-0d** (the WebSocket validator move, L3) — **targets 11-24, hard edge 12-04 (PM 750).** After R-0c,
   one at a time. Its open condition is closed by (A) (§4.1, PM 784 (2), (6)). Memo, review and dates as
   step 3 (PM 784 (3)).
10. P0a-2a PR — after R-JS (**12-09**, proposed).
11. **R-NAT — lands 12-15** (core chapter and host chapter written 11-26 … 12-02, plan-review 12-03 …
   12-05, implementation 12-08 … 12-12, the two lanes in parallel). The host chapter's window is
   PM-confirmed (PM 745): L3 writes it in parallel with its tree-model PR-1. **Open condition: L3's
   capacity for the implementation window 12-08 … 12-12, to be confirmed by the PM (PM 754).** L3
   re-derives its dates after its tree-model R2; the PM decides on that answer (PM 784 (7)). After R-JS;
   independent of P0a-2a's PR. `#11-vm-proxy-reflect` and the other new callers of the entry (§5.2) start
   after this step (§4.2, the R-JS-stage fallback).
12. Shim retirement — **12-16** (§9.2).
13. The classification → P0a-2b — the classification starts after step 12, at that main, so it measures
    R-JS's entries, R-NAT's helper rows and the retirement's import lines once (§8.2). Its dates are the
    stack-safety program's.
14. Slice F — unchanged relative order (after P0a-2b); it shares no function with this program
    (`vm/host/structured_clone.rs` has no `.kind =` line and no constructor).
15. E1 — after R-NAT **and** Slice F (§6).

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

**Proposal: immediately after R-NAT, as its own script-generated PR** (step 12 of §9.1).

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
- The retirement also precedes the classification (§9.1 step 13), so the import-line shift happens before
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
- **M-6 spec numbers.** `.claude/tools/webref coverage-map ecma262 EvaluateNew ecma262 Construct ecma262
  10.2.2 ecma262 OrdinaryCallBindThis ecma262 10.3.2 ecma262 BuiltinCallOrConstruct ecma262 10.4.1.2
  ecma262 GetPrototypeFromConstructor ecma262 OrdinaryCreateFromConstructor ecma262 OrdinaryObjectCreate
  ecma262 MakeBasicObject ecma262 13.3.7.1 ecma262 BindThisValue ecma262 9.1.1.3.3 ecma262
  GetThisEnvironment ecma262 InitializeInstanceElements ecma262 10.5.13 ecma262 28.1.2 ecma262 20.5.1.1
  ecma262 20.5.6.1.1 ecma262 20.5.7.1.1 ecma262 23.1.1.1 ecma262 20.1.1.1 ecma262 22.1.1.1 ecma262
  21.1.1.1 ecma262 20.3.1.1 ecma262 21.4.2.1 webidl 3.7.1 webidl 3.8 wasm-js-api-2 5.10 webidl 3.12 html
  3.2.3 html 4.13.5 html 4.13.4` (34 rows, re-run 2026-11-15); step text from `webref body` on each anchor
  or AO name. §4.3's Call-mode table adds `webref heading ecma262` on 27.5.3.1, 25.1.4.1, 25.3.2.1,
  23.2.5.1, 23.2.1.1 and the four `.prototype` sections it cites.
- **M-7 line counts.** `git show 4b723d4b:<path> | wc -l` over `H/logs/r-touched-files-4b723d4b.txt`;
  `vm/value.rs` at `ab71cb43` → 967.
- **M-8 stale upgrade cites (P-11).** Frame `F`, the files under `crates/` that name the algorithm: `git
  grep -l -P '(?i)upgrade an element|4\.13\.5|C4\\?\]' 4b723d4b -- crates` → 20. Candidates: `git grep -n
  -P '(?i)steps? \d|^\s*//[/!]? *\d+\.\d' 4b723d4b -- F` → 158 lines (the second alternative catches a
  number that starts the next comment line). Read: 25 cite a step of *upgrade an element*, 6 with today's
  number (step 6) and **19** stale (8 → R-JS, 11 → L3 by PM 784 (5)). At `3e4a03a2`: 20, 158, the same
  19. Limit: a file citing an upgrade step without one of `F`'s three spellings is outside the frame.
- **M-9 outside writes (§4.4 item 3; a lower bound, not the population).** `git grep -c -P
  '_states\s*\.insert\(' 4b723d4b -- crates/script/elidex-js/src ':!*tests*'` → 53 lines in 37 files.
- **M-10 R-0d.** `git show 4b723d4b:crates/script/elidex-js/src/vm/host/websocket.rs | wc -l` → 1001; the
  moved ranges `:496-523` (28) and `:527-559` (33) by `grep -n` on their first and last lines. `token`
  spellings: `git grep -n -P 'fn is_valid_header_name|fn is_tchar|fn is_valid_protocol_token_char|HTTP
  token:' 4b723d4b -- crates` → 5 lines, 4 files. Manifests: `git grep -n 'elidex-api-fetch\|elidex_api_fetch'
  4b723d4b -- . ':!Cargo.lock' ':!docs'` → the workspace manifest and its own (no dependent); `git grep -c
  'elidex-api-\|elidex-cache-api' 4b723d4b -- 'crates/api/*/Cargo.toml'` → 1 each, its own name (no edge).
  R-0d's edge (PM 784 (2)): `elidex-api-fetch`'s `[dependencies]` is empty, so it closes no cycle.
- **M-11 ledgers.** PM ledger: `grep -n -- '<slot id>'` on the live file, 2026-11-15. Repo ledger: `sed -n
  293p docs/plans/2026-07-vm-p4-slot-ledger.md` at `d11c4df7` (no "PM 660", "PM 664" or "PR-0A" in it).
