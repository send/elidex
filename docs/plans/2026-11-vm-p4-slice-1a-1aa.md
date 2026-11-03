# VM P4 slice 1aa — the argument-layout and inline-cache contract (per-PR plan, rev 1)

**Parent.** Umbrella **1a** (*Call-spread VM infrastructure*), row `1a` of the §5 slice table in
`docs/plans/2026-07-vm-p4-slice-plan.md`. 1a's derivation ran in the same commit as this memo and is
recorded in §6.6 of `docs/plans/2026-07-vm-p4-slice-1a-1b-call-spread-detail.md`; it minted three
terminal children, **1aa** (this memo), **1ab** (remove `op_array_spread`'s non-spec `return()`) and
**1ac** (root the four call-entry argument windows). This memo is 1aa's mandatory
`/elidex-plan-review` input; it is outside the `2026-07-vm-p4-*` family glob, so family memos cite it
by path in backticks, never by a relative link.

**Base.** `origin/main` = `eaa741b5`. Every `file:line` below is at that revision; crate paths are
relative to `crates/script/elidex-js/src/` unless written in full. Line numbers are evidence — §6
lists the command that re-derives each one.

**Kind.** A terminal **prerequisite** row in §5's sense: it lands before the children of umbrella 1b
that make its `None` IC arm and its layout helper reachable from new opcodes, so its acceptance is
bounded to what its own artifact buys when it lands, and the part with no JS-reachable holder yet is
asserted from Rust (§5, "Prerequisite row").

---

## §0. Scope

### §0.1 What this PR implements

The (a) mechanism of umbrella 1a's derivation — the in-VM calling contract that turns an args array
into a call's argument window — and nothing else:

- **D1 — `lay_out_call_args` (NEW).** One stack-layout operation: the args array at the top of
  `vm.stack` is replaced, in place, by its elements, and the element count is returned. It is the
  single place an args array becomes an argument window (§1.1).
- **D2 — `Empty` normalisation, shared with `collect_array_like`'s dense fast path (dec. 8).** One
  primitive maps an `ObjectKind::Array`'s `elements` to argument values (`JsValue::Empty` →
  `Undefined`); D1 and `Function.prototype.apply`'s dense branch both read through it (§1.2).
- **D3 — `super(...)` has one dispatch path.** `Op::SuperCallSpread` becomes "lay out, then
  `op_super_call`", so the spread and flat `super` calls share argument hand-off, error routing and
  `[[Construct]]` dispatch; `op_super_call` takes `argc: usize` so a spread window wider than 255
  is not truncated (§1.3).
- **D4 — the falsified docstring is corrected.** `op_super_call_spread`'s claim that the compiler
  emits only `CreateArray; ArraySpread x; SuperCallSpread` is false: the synthesized default derived
  constructor (`compiler/expr_class.rs:145-152`) emits `GetLocal 0; SuperCallSpread` over a
  rest-packed array. That producer is spec-required (ECMA-262 §15.7.14 step 14.a.iv.1 NOTE) and is
  **not** folded (§1.4).
- **D5 — `ic_call` / `ic_call_method` take `call_ic_idx: Option<usize>` (dec. 11).** `None` skips
  the IC lookup and the IC population, and otherwise runs the same three-way body, so the spread
  handlers 1b's children add reach the one call-dispatch implementation (§1.5).

No opcode is added, no emit site changes, and no call shape changes behaviour. `f(...xs)`,
`o.m(...xs)` and `new F(...xs)` are still compiled by `compile_arguments`'s spread-as-one-argument
arm after this PR; fixing them is umbrella 1b's (§5).

### §0.2 Premises re-verified at `eaa741b5`

| Premise | Verified by | Result |
|---|---|---|
| `compile_arguments` compiles `Argument::Spread(e)` as one ordinary argument | `sed -n 53,79p compiler/expr_member.rs` | true — the arm at `:70-75` |
| `super(...args)` already lowers spread to `CreateArray`/`ArrayPush`/`ArraySpread` → `Op::SuperCallSpread` | `sed -n 106,134p compiler/expr_member.rs` | true — `:113-129`; the flat branch is `:130-133` |
| `new F(...args)` has the same silent-wrong shape | `sed -n 88,93p compiler/expr.rs` | true — `compile_arguments` then `emit_u8(Op::New, argc)`; `Op::NewSpread` is a `pop; pop; push undefined` stub at `vm/dispatch.rs:879-883` with no emit site |
| `ic_call` / `ic_call_method` have exactly two callers, both in `vm/dispatch.rs` | `git grep -n 'ic_call(\|ic_call_method(' -- crates/script/elidex-js/src` | 2 call sites — `vm/dispatch.rs:705`, `:716` (the other two hits are the definitions, `vm/dispatch_ic.rs:222`, `:268`) |
| `op_super_call_spread` clones `elements` without normalising `Empty` | `sed -n 72,96p vm/dispatch_class.rs` | true — `elements.clone()` at `:76` |
| `collect_array_like`'s dense branch normalises with `.or_undefined()` | `sed -n 272,278p vm/natives_function.rs` | true — `:276-277`; its one caller is `Function.prototype.apply` at `:76` |
| Natives run with `gc_enabled = false` | `git grep -n 'gc_enabled = false' -- crates/script/elidex-js/src/vm/interpreter.rs` | true — the native-dispatch sites `interpreter.rs:364`, `:412`, `:423`, `:502` (the other hits are comments and a test-only panic probe at `:1277`) |
| No test262 harness is wired in this repository | `git ls-files \| /usr/bin/grep -ci test262` | 0 |

### §0.3 Premise errors (reported, not acted on)

1. **"Slice 1a (call-argument spread)", "the narrowest call-argument emit path first, per
   §6.2a-3".** 1a is the *VM infrastructure* half and has **no `compiler/` file** (its §5 row:
   "**no `compiler/` file** (the fold is withdrawn; edge 32 is a test)"); every emit path, including
   `f(...xs)`, `o.m(...xs)` and `new F(...xs)`, is umbrella **1b**'s (§6.3's `[1b]` tag: "the
   compiler helper, `emit_call` aggregation, `CallMethodSpread`, the three handlers, and arity-based
   form selection — i.e. everything that changes observable behaviour"). §6.2a-3 partitions Slice
   P's `IteratorClose` sites and says nothing about the order of 1a's children. None of 1a's
   children consumes another's artifact (carve-out §6.6), so "first" is the id order, and 1aa is
   the child every consumer in 1b reaches.
2. **The regression list** — `Math.max(...[1,2,3])`, the `o.m(...xs)` receiver, `f(a, ...xs, b)`
   ordering, non-iterable spread → TypeError, an iterator that throws mid-way, >255 arguments after
   spread — describes 1b's emit paths. 1aa cannot pass or fail any of them through `Call` /
   `CallMethod`, because no call shape changes here. What 1aa owns of that list is its `super(...)`
   spelling, which is already reachable (§4.2); the mid-drain-throw case's `return()` half is
   **1ab**'s (edge 21).
3. **Not a premise of the brief, found while verifying it:** `Function.prototype.apply`'s dense
   branch is not CreateListFromArrayLike-correct for a holey array whose prototype chain has an
   indexed property (§3.1). §9 dec. 17 assigns this defect class to `Reflect.apply` only.

### §0.4 What the other children and umbrella 1b own

- **1ab** — removing `op_array_spread`'s non-spec `return()` (dec. 13a), with edge 21. 1aa does not
  touch `vm/dispatch_iter.rs`.
- **1ac** — rooting `stack_slice` and `actual_args` in the four call-entry windows
  (`vm/interpreter.rs:630`, `:665`, `:867`, `:914`), with edges 27/28. 1aa does not touch
  `vm/interpreter.rs`.
- **Umbrella 1b's children** — `compile_call_arguments` / `ArgsForm`, `emit_call`, `CallSpread` /
  `CallMethodSpread` / `NewSpread` handlers (the first `None` producers and the first new consumers
  of D1), arity-based form selection (`f(a1…a300)`), the stack-depth bound (dec. 12), the I-3
  tagged-template input contract, and every edge of §6.4 the carve-out does not place on 1a's
  children.

---

## §1. Design

### §1.1 `lay_out_call_args` — the args array becomes the argument window

```rust
/// [C19] ECMA-262 §13.3.8.1 ArgumentListEvaluation, consumer side: the List an
/// args array carries becomes the call's argument window.
/// Stack: [… args_array] → [… arg0 … argN-1]; returns N.
pub(super) fn lay_out_call_args(&mut self) -> Result<usize, VmError>  // (NEW)
```

- **In place, not pop-then-push.** The array is read where it sits (`self.stack.last()`), and the
  slot it occupies is replaced by its elements. No `ObjectId` of the array or of an element is held
  in a Rust local across anything that can allocate.
- **No allocation in the window, by construction rather than by assertion.** The body borrows the
  two fields it needs disjointly — `let VmInner { stack, objects, .. } = self;` (`vm/mod.rs:182`,
  `:186`) — so between reading `elements` and the last push there is no `&mut VmInner` through
  which `alloc_object` (`vm/inner.rs:89`) could be reached. Edge 16 asks for a debug assertion that
  no allocation event occurs in this window; the borrow makes the event unrepresentable, and the
  edge's runtime pin becomes the tripwire test of §4.1 (T2), which fails for any rewrite that
  reintroduces an allocation. ⚠ This is a strengthening of what edge 16 prescribes, and the
  plan-review is asked to ratify it (§7 Q1).
- **Error contract.** A top of stack that is not an `ObjectKind::Array` object is a compiler bug,
  not user error: `Err(VmError::internal(…))` with the stack unchanged, which is what
  `op_super_call_spread`'s two defensive branches return today (`vm/dispatch_class.rs:78-93`). The
  caller routes the error (D3 routes it through `throw_error`, exactly as now).
- **Unbounded by design here.** The helper returns `usize` and imposes no width: the Array path must
  accept `f(...arrOf1000)` (§6.3). The stack-depth bound (dec. 12) is owned by the child umbrella 1b
  mints for the argument-emit helper; because the helper already returns `Result`, that child can
  put the check here without a signature change (§7 Q3).
- **Home.** `vm/dispatch_helpers.rs` (391 lines; zero `feature = "engine"` occurrences, so the
  core-only predicate of the touch-time split record holds for the whole file). The §5 module
  column names it; nothing found here argues against it.

### §1.2 `Empty` normalisation, shared with `collect_array_like` (dec. 8)

`Op::ArrayHole` and indexed writes put `JsValue::Empty` into `elements`. The arrays D1 receives are
hole-free by construction — `CreateArray` + `ArrayPush` + `ArraySpread` (whose drain pushes
`IteratorStepValue` results) and rest-parameter packing never write `Empty` — so normalisation is
the documented invariant's backstop, not a behaviour. It still has exactly one implementation:

```rust
/// The argument values a dense Array's `elements` denote: holes read as `undefined`.
/// Exact for hole-free arrays. For a holey array it is ArgumentListEvaluation-exact but NOT
/// CreateListFromArrayLike-exact (§7.3.19 step 6.b performs a real `Get`), so a caller that
/// implements CreateListFromArrayLike owns that difference (dec. 17).
pub(super) fn array_argument_values(elements: &[JsValue]) -> impl Iterator<Item = JsValue> + '_  // (NEW)
```

D1 extends the stack from it; `collect_array_like`'s dense branch (`vm/natives_function.rs:276-277`)
collects from it. That second use is behaviour-preserving: the defect it carries (§3.1) is
pre-existing, is not created or widened here, and the docstring above names it instead of hiding it.

### §1.3 One `super(...)` path

Today the two `super` opcodes reach `dispatch_super` by different hand-offs: `Op::SuperCall` drains
`argc` stack values (`vm/dispatch_class.rs:46-60`), `Op::SuperCallSpread` pops the array and clones
`elements` (`:72-96`). After this PR:

```rust
Op::SuperCallSpread => match self.lay_out_call_args() {
    Ok(argc) => self.op_super_call(argc, entry_frame_depth)?,
    Err(e) => { self.throw_error(e, entry_frame_depth)?; }
},
```

and `op_super_call_spread` is deleted (its only caller is the arm above; `git grep -n
'op_super_call_spread' -- crates/script/elidex-js/src` → the definition and `vm/dispatch.rs:885`).
`op_super_call` changes `argc: u8` → `argc: usize`; the `Op::SuperCall` arm (`vm/dispatch.rs:939-942`)
widens its `read_u8_op()` with `as usize`. Without the widening, `super(...arrOf300)` would be
truncated to 44 arguments — today it is not, because the spread path never passed a count. The
`dispatch.rs` arms change in place and do not grow (touch-time split record: "never grow an arm in
place").

Error routing is unchanged: both opcodes already end in `throw_error` through `dispatch_super`
(`:104-113`), and the internal error for a non-Array top of stack keeps its route.

### §1.4 The docstring correction (D4)

`op_super_call_spread`'s docstring goes with the function; its content moves to the
`Op::SuperCallSpread` arm and to `lay_out_call_args`, stated over **both** producers: the
`super(...)` lowering (`compiler/expr_member.rs:113-129`, whose array is built by the
`ArraySpread` drain and so performs GetIterator) and the synthesized default derived constructor
(`compiler/expr_class.rs:145-152`, whose array is rest-packed at frame entry and so performs no
iteration — ECMA-262 §15.7.14 step 14.a.iv.1 NOTE). The module docstring of `vm/dispatch_class.rs`
(`:13-14`, "`Op::SuperCallSpread` consumes a pre-built args array") is corrected with it. The
invariant both producers share, and which D1 relies on, is the one stated: the operand is a
hole-free `ObjectKind::Array` that no user code can reach between its construction and the opcode.

### §1.5 The IC entry point (D5, dec. 11)

`ic_call(caller_func_id, argc, call_ic_idx: Option<usize>)` and the same for `ic_call_method`.
`try_call_ic` and `populate_call_ic` keep `usize` and are called only under `Some`:
`call_ic_idx.and_then(|i| self.try_call_ic(caller_func_id, i, callee_id))` on entry, and
`if let Some(i) = call_ic_idx { self.populate_call_ic(…, i, …) }` on the two population points.
The IC-miss JS branch, the native branch and the non-callable branch are otherwise one body for
both arms — the one-SoT reading dec. 11 ratified against duplicating ~35 lines per opcode. The two
callers pass `Some(call_ic_idx)`. The `None` arm has no live producer until 1b's children land,
which is why its direct unit tests are an acceptance item of this PR rather than of theirs (dec. 11:
"without it that child ships exactly the dead third state I-4 rejects").

### §1.6 Lens checks

- **Layering.** All of it is engine-bound VM dispatch (`vm/dispatch_*.rs`); no algorithm moves into
  `vm/host/`, and no engine-independent crate has an argument-layout algorithm to reuse.
- **ECS-native.** No per-entity state is introduced; nothing here is a side-store.
- **One issue, one way.** After this PR there is one args-array→window operation (D1), one
  holes-as-arguments primitive (D2), one `super` dispatch path (D3) and one call-dispatch body for IC
  and non-IC callers (D5). The only remaining second mechanism is the default derived
  constructor's rest-packed producer, and it is the spec-required I-3 carve-out (§6.3).
- **1000-line debt.** Touched files over 1000 lines: `vm/dispatch.rs` (1103) — two arms re-signed
  in place, no growth, per the touch-time split record's ruling that the dispatch match is a flat
  case table. The others are under 1000: `dispatch_helpers.rs` 391, `dispatch_class.rs` 347,
  `dispatch_ic.rs` 342, `natives_function.rs` 297, `tests/tests_class_machinery.rs` 389.

### §1.7 Change list by file

| File | Change |
|---|---|
| `vm/dispatch_helpers.rs` | `lay_out_call_args` (NEW), `array_argument_values` (NEW) |
| `vm/dispatch_class.rs` | `op_super_call` takes `usize`; `op_super_call_spread` deleted; module docstring `:13-14` corrected |
| `vm/dispatch.rs` | `Op::SuperCallSpread` arm (`:884-886`) per §1.3; `Op::SuperCall` arm (`:939-942`) widens; `Op::Call` / `Op::CallMethod` arms (`:705`, `:716`) pass `Some(call_ic_idx)` |
| `vm/dispatch_ic.rs` | `ic_call` / `ic_call_method` take `Option<usize>` (§1.5) |
| `vm/natives_function.rs` | `collect_array_like`'s dense branch reads through `array_argument_values` |
| `vm/tests/tests_call_args_layout.rs` (NEW) | §4.1 |
| `vm/tests/tests_class_machinery.rs` | §4.2 |
| `vm/tests/mod.rs` | registers the new module |

### §1.8 Commit sequence (pins first)

There is no honest red-first set at the base: the Rust tests (T1–T7) name symbols and a signature
that do not exist yet, so they cannot be committed ahead of them, and the JS tests (C1–C7) pin
behaviour that is already correct. So the order is pins first, then each mechanism with its tests:

1. C1–C6 (§4.2), green at the base — committed first, so each later commit is measured against them.
2. D1 + D2 with T1–T3.
3. D3 + D4. The `usize` widening lands in the same commit as the conversion, so C6 is never red.
4. D5 with T4–T7.

The mutation proof each test names (§4) is run once, on the final head, by reverting the one line the
mutant names and confirming the named test goes red.

---

## §2. Coupled invariants

**Terminality, re-applied at this slice's own HEAD (§5: a verdict is evidence until the slice's
plan-review re-applies it).** The criterion's first disjunct asks for ≥3 *intersecting* invariant
axes, an axis being evidenced by failing while the others pass. This PR carries two axes — the
args-array→window layout contract (D1–D4) and the IC entry signature (D5) — and they do **not**
intersect inside it: the `None` arm and the layout helper meet only in 1b's handlers. The second
disjunct fails too: the canonical form exists in-tree (`construct_synchronous`'s callee-then-args
layout, `vm/dispatch_class.rs:304-307`, "matching `Op::Call`'s shape"). So the row is not
edge-dense and this section is an enumeration for the reviewer, not a §2 the trigger requires.

Invariants the design holds simultaneously:

- **I-L (layout).** After D1 the stack holds exactly the array's elements, in index order, where the
  array was; the returned count equals the number pushed.
- **I-H (holes).** No `JsValue::Empty` reaches an argument window (D2).
- **I-G (GC).** No `ObjectId` that the args array or its elements denote is held only in a Rust local
  across an allocation (D1's disjoint borrow).
- **I-O (observability).** Laying out an args array performs no property lookup and no iteration;
  in particular the default derived constructor never calls `%Array.prototype%[%Symbol.iterator%]`
  (§15.7.14 step 14.a.iv.1 NOTE; edge 32).
- **I-W (width).** No argument count passes through a `u8` on the array path (D3's widening).
- **I-C (IC).** `None` never reads or writes `call_ic_slots`; `Some(i)` behaves exactly as today.

Pairwise intersections that are real in this PR:

- **I-L × I-G** — the window in which elements move from the heap to the stack is the one with no
  `&mut VmInner`; reordering the body so the array is popped before its elements are read would
  reopen it.
- **I-L × I-H** — the push and the normalisation are one iterator (`array_argument_values`), so the
  stack cannot receive an un-normalised element.
- **I-L × I-O** — the layout reads `elements` directly; an implementation through the iterator
  protocol satisfies I-L and I-H and breaks I-O (edge 32's mutant).
- **I-L × I-W** — the count D1 returns is `usize` and its only consumer in this PR (`op_super_call`)
  takes `usize`.
- **I-H × I-O** — the normalisation is ArgumentListEvaluation-exact, not CreateListFromArrayLike-exact;
  sharing it with `collect_array_like` is behaviour-preserving there, and the difference is named at
  the primitive (§1.2, §3.1).
- **I-C × the rest** — none in this PR (stated so the reviewer can check the claim of
  non-intersection rather than infer it).

---

## §3. Spec coverage map

Skeleton generated with `.claude/tools/webref coverage-map ecma262 ArgumentListEvaluation ecma262
EvaluateCall ecma262 ClassDefinitionEvaluation ecma262 Call ecma262 Construct ecma262
CreateListFromArrayLike ecma262 GetSuperConstructor` and `.claude/tools/webref coverage-map ecma262
13.3.7.1 ecma262 20.2.3.1` (the two anchors `sec-super-keyword-runtime-semantics-evaluation` and
`sec-function.prototype.apply` are not AO names, so they are passed as § numbers; verdict "ok →
single PR scope", 2026-11-03). Every § number is `webref aoid` / `webref heading` output; steps are
`webref body` output.

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| ECMA-262 §13.3.7.1 Runtime Semantics: Evaluation | `SuperCall : super Arguments` step 4 | `argList` from the spread productions of ArgumentListEvaluation | `Op::SuperCallSpread` arm → `lay_out_call_args` (NEW) → `op_super_call` | ✓ | yes |
| ECMA-262 §13.3.7.1 Runtime Semantics: Evaluation | `SuperCall` steps 1, 3, 5, 6 | GetNewTarget; GetSuperConstructor; IsConstructor false → TypeError; Construct(superCtor, argList, newTarget) | `dispatch_super_inner` / `construct_synchronous` (`vm/dispatch_class.rs:116-157`, `:232`) — unchanged | ✓ | yes |
| ECMA-262 §13.3.7.1 Runtime Semantics: Evaluation | `SuperCall` steps 3 → 4 order | superCtor is fixed **before** the arguments are evaluated | — (pre-existing divergence, §3.1; not this PR's) | ✗ | yes |
| ECMA-262 §13.3.7.2 GetSuperConstructor ( ) | steps 3, 5 | `activeFunc.[[GetPrototypeOf]]()` | `home_class` → `.prototype` (`vm/dispatch_class.rs:118-125`) — unchanged | ✓ | yes |
| ECMA-262 §13.3.8.1 Runtime Semantics: ArgumentListEvaluation | `ArgumentList : ... AssignmentExpression` steps 1-5; `ArgumentList : ArgumentList , ... AssignmentExpression` steps 1-4 | the List's order is the drain order; `precedingArgs` first | producer `compiler/expr_member.rs:113-129` (unchanged); consumer `lay_out_call_args` preserves index order (I-L) | ✓ | yes |
| ECMA-262 §13.3.8.1 Runtime Semantics: ArgumentListEvaluation | `Arguments : ( )` step 1 | empty List | `super(...[])` → window of 0 | ✓ | yes |
| ECMA-262 §15.7.14 Runtime Semantics: ClassDefinitionEvaluation | step 14.a.i | `args` = the List passed by [[Call]] / [[Construct]] | rest-param packing into local 0 (`compiler/expr_class.rs:145-152`, unchanged) → `lay_out_call_args` | ✓ | yes |
| ECMA-262 §15.7.14 Runtime Semantics: ClassDefinitionEvaluation | step 14.a.iv.1 NOTE | must **not** call `%Array.prototype%[%Symbol.iterator%]` | `lay_out_call_args` reads `elements` directly (I-O); edge 32 | ✓ | yes |
| ECMA-262 §15.7.14 Runtime Semantics: ClassDefinitionEvaluation | step 14.a.iv.2-4 | func = ctorFunc.[[GetPrototypeOf]](); IsConstructor false → TypeError; Construct(func, args, NewTarget) | `dispatch_super_inner` — unchanged | ✓ | yes |
| ECMA-262 §13.3.6.2 EvaluateCall ( func, thisValueRef, argumentListNode, tailPosition ) | steps 4-5 | func not an Object / not callable → TypeError | `ic_call` / `ic_call_method` non-callable branch, under `Some` and `None` | ✓ | yes |
| ECMA-262 §13.3.6.2 EvaluateCall ( func, thisValueRef, argumentListNode, tailPosition ) | step 7 | `Call(func, thisValue, argList)` | `ic_call` (`this` from `compute_this_for_call`) / `ic_call_method` (receiver), JS and native branches, under `Some` and `None` | ✓ | yes |
| ECMA-262 §13.3.6.2 EvaluateCall ( func, thisValueRef, argumentListNode, tailPosition ) | step 6 | `PrepareForTailCall()` | — | n/a (no tail calls in elidex) | no |
| ECMA-262 §7.3.13 Call ( func, thisValue [ , argList ] ) | steps 2-3 | IsCallable false → TypeError; `func.[[Call]]` | native branch → `self.call` (unchanged) | ✓ | yes |
| ECMA-262 §7.3.14 Construct ( ctor [ , argList [ , newTarget ] ] ) | steps 1-3 | `newTarget` defaults to `ctor` | `do_new` — not touched | n/a (not touched) | no |
| ECMA-262 §20.2.3.1 Function.prototype.apply ( thisArg, argArray ) | step 4 | `CreateListFromArrayLike(argArray)` | `collect_array_like` dense branch → `array_argument_values` (behaviour-preserving) | ✗ (holey-array branch diverges, §3.1) | yes |
| ECMA-262 §7.3.19 CreateListFromArrayLike ( obj [ , validElementTypes ] ) | step 6.b | `next` = `? Get(obj, indexName)` — consults the prototype chain for a hole | `collect_array_like` dense branch maps a hole to `undefined` without the `Get` | ✗ (pre-existing, §3.1) | yes |

**Breadth**: K=1 (ECMA-262), M=16 → single PR.

### §3.1 User-input touch audit

Sites this PR touches that user-controlled values flow through:

- `Op::SuperCallSpread` / `lay_out_call_args`: the element values are user values (spread operand
  or the arguments of `new B(…)`), and their count is user-controlled (`super(...arrOf1e6)`); the
  array itself is compiler-built and unreachable from user code. Exposure change: **none** — the
  same values reach `dispatch_super` in the same order; the count was already unbounded (the current
  `elements.clone()` + `construct_synchronous`'s re-push).
- `ic_call` / `ic_call_method`: callee, receiver and arguments are user values. Exposure change:
  **none** for `Some`; `None` is unreachable from JS in this PR.
- `collect_array_like`: user array via `Function.prototype.apply`. Exposure change: **none**.

Adjacent pre-existing defects found while auditing (reported; neither is created or widened here):

- **`super(...)` resolves the super constructor after its arguments** (§13.3.7.1 `SuperCall` fixes
  `superCtor` at step 3, before ArgumentListEvaluation at step 4; `dispatch_super_inner` reads
  `home.prototype` when the opcode runs). Observable when an argument changes the active class's
  prototype, e.g. `class B extends A { constructor() { super(Object.setPrototypeOf(B, C)) } }`
  constructs `A` per spec and `C` here. Both `super` opcodes share it; a fix needs an emit change
  (resolve before the arguments), so it is not 1aa's. **No owner in the family memos**
  (`/usr/bin/grep -n 'GetSuperConstructor' docs/plans/2026-07-vm-p4-*.md` → 3 hits, all about
  `super` through an arrow). Routed to the driver for an owner (§7 Q4).
- **`Function.prototype.apply` on a holey array** skips CreateListFromArrayLike's `Get` for a hole
  (§7.3.19 step 6.b): with `Array.prototype[0]` defined as a getter,
  `f.apply(null, [ , 1])` must run it and does not. §9 dec. 17 names this defect for
  `Reflect.apply` only. Routed to the driver (§7 Q5); the remedy is a caller-side precondition in
  `collect_array_like` (take the dense branch only when no element is `Empty`), which dec. 8 says
  is where that distinction lives.

---

## §4. Tests

Supported surface: elidex-js unit and VM integration tests (`cargo test -p elidex-js
--all-features`); there is no test262 subset in this repository (§0.2). Each test names the mutant
it kills, so none is satisfiable by an implementation that does not do the thing.

### §4.1 `vm/tests/tests_call_args_layout.rs` (NEW) — from Rust

- **T1 layout.** An Array `[o1, Empty, 3]` (objects allocated in setup) at the top of the stack
  above a sentinel; `lay_out_call_args()` → `Ok(3)` and the stack is `[sentinel, o1, undefined, 3]`.
  Kills: reversed order, dropped normalisation, off-by-one count, array left on the stack.
- **T2 edge 16 — no allocation in the window.** Same setup; arm `force_gc_before_next_alloc`
  (`vm/mod.rs:274`) after setup; call `lay_out_call_args()`; assert the one-shot is still armed.
  Paired control in the same test: the same arming followed by one `alloc_object` consumes it, so
  the probe can observe an allocation. Kills: any implementation that allocates (a temporary JS
  array, a boxed iterator object) between reading `elements` and the last push.
- **T3 error contract.** Top of stack a non-Array object, then a primitive: `Err` of the internal
  kind, stack unchanged. Kills: a helper that pops before validating.
- **T4 `ic_call(.., None)`, JS callee.** Compile a script defining `function f(a, b) { return
  [this, a + b] }`; lay out `[f, 1, 2]` on the stack; `ic_call(caller, 2, None)` then `run()`; the
  result is `[undefined, 3]`, and the caller's `call_ic_slots` are byte-identical before and after.
  **Positive control in the same test**: the same call with `Some(i)` (an in-range slot) populates
  slot `i`, so the "unchanged" assertion can fail. Kills: a `None` that silently uses slot 0 or any
  sentinel.
- **T5 `ic_call(.., None)`, native callee** (`Math.max`, args `[1, 3, 2]`) → `3` pushed, stack shape
  `[…, 3]`, no IC slot written. Kills: a `None` arm that skips the native branch.
- **T6 `ic_call(.., None)`, non-callable callee** → `Err` whose thrown value is a `TypeError`. Kills:
  a `None` arm that returns `Ok` with `undefined`.
- **T7 `ic_call_method(.., None)`** over `[o, o.m, 4]` with `m(x) { return [this === o, x] }` →
  `[true, 4]`, no IC slot written; plus the native and non-callable branches as in T5/T6. Kills: a
  receiver dropped on the `None` path.

### §4.2 `vm/tests/tests_class_machinery.rs` — from JS (no-regression; green at the base and after)

- **C1 edge 32.** Replace `%Array.prototype%[Symbol.iterator]` with a counting wrapper of the
  original; `class A { constructor(a, b) { this.s = a + b } } class B extends A {}`; `new B(1, 2).s
  === 3` **and** the count is 0. **Positive control in the same test**: `class C extends A {
  constructor(...args) { super(...args) } }`; `new C(1, 2).s === 3` and the count is 1. Kills: a
  layout through the iterator protocol (I-O), which the control alone cannot distinguish from a
  broken spread path.
- **C2** `super(...[])` → `A` sees `arguments.length === 0`.
- **C3** `super(a, ...xs, b)` → `A` sees `[a, ...xs, b]` in order.
- **C4** default derived constructor over a native base: `class B extends Array {}`;
  `new B(1, 2, 3).length === 3`; and the hand-written `super(...xs)` spelling over `Array`. Covers
  the native branch of `construct_synchronous` through the converted arm. ⚠ Not probed at the base:
  if `class B extends Array {}` is already red there, that is a pre-existing gap in native-base
  subclassing, not this PR's — the implementer substitutes another constructable native base that is
  green at the base and reports the gap, rather than landing a red pin.
- **C5** bound base: `class B extends A.bind(null, 0) { constructor(...a) { super(...a) } }` → `A`
  sees `[0, ...a]` (the bound-prefix splice composed with the laid-out window).
- **C6 width.** `super(...arrOf300)` → `A` sees `arguments.length === 300` and the last value.
  Kills: `op_super_call(argc: u8)` surviving the conversion (I-W).
- **C7** the existing `super_call_spread_propagates_argv` (`:124`) is kept unchanged.

Not here, and why: edges 21 and 27/28 are 1ab's and 1ac's (carve-out §6.6); the brief's call-shape
regressions are 1b's children's (§0.3 item 2). The existing suite passing is necessary and not
sufficient — it covers neither I-O nor the `None` arm, which is why T2, T4-T7 and C1 exist.

---

## §5. Non-goals, and what the next slices own

- **`f(...xs)` / `o.m(...xs)` / `new F(...xs)` / optional-call spread** — umbrella 1b's children:
  `compiler/expr_member.rs:70-75` and `compiler/expr.rs:88-92` are not touched, so `Math.max(...[1,2,3])`
  is still `NaN` after this PR. `Op::CallSpread` / `Op::NewSpread` stay stubs
  (`vm/dispatch.rs:879-883`) until those children connect them to D1 + D5 (and `do_new`).
- **`op_array_spread`'s `return()`** — 1ab.
- **The four argument windows in `vm/interpreter.rs`** — 1ac.
- **Stack-depth bound** — dec. 12's owner (§7 Q3).
- **The two pre-existing defects of §3.1** — routed, not fixed (§7 Q4, Q5).
- **Folding the default derived constructor onto `ArraySpread`** — forbidden (§15.7.14 step
  14.a.iv.1 NOTE); C1 pins it.

---

## §6. Structural references — re-derivation commands

All at `eaa741b5`, from `crates/script/elidex-js/src/`:

- `git grep -n 'fn compile_arguments\|Argument::Spread' -- compiler/expr_member.rs` → `:53`, `:70`,
  and the super-spread arm's `:123`.
- `git grep -n 'ExprKind::New' -- compiler/expr.rs` → `:88`.
- `git grep -n 'Op::SuperCallSpread as u8' -- compiler/expr_class.rs` → `:149` (inside `:145-152`).
- `git grep -n 'ic_call(\|ic_call_method(' -- vm` → `dispatch.rs:705`, `:716`, `dispatch_ic.rs:222`,
  `:268` (2 call sites + 2 definitions).
- `git grep -n 'fn op_super_call\|fn dispatch_super\|fn construct_synchronous\|vm.stack.push(JsValue::Object(ctor_id))' -- vm/dispatch_class.rs`
  → `:46`, `:72`, `:104`, `:115`, `:232`, `:304`.
- `git grep -n 'Op::SuperCall =>\|Op::SuperCallSpread =>\|Op::CallSpread | Op::NewSpread' -- vm/dispatch.rs`
  → `:939`, `:884`, `:879`.
- `git grep -n 'fn try_call_ic\|fn populate_call_ic' -- vm/dispatch_ic.rs` → `:200`, `:330`.
- `git grep -n 'fn collect_array_like\|collect_array_like(ctx' -- vm/natives_function.rs` → `:273`,
  `:76` (1 caller).
- `git grep -n 'pub(crate) fn alloc_object\|force_gc_before_next_alloc' -- vm/inner.rs` → `:89`, `:105`, `:106` (the one-shot is `#[cfg(test)]`, `vm/mod.rs:273`).
- `git grep -n 'pub(crate) stack:\|pub(crate) objects:\|pub(crate) force_gc_before_next_alloc' -- vm/mod.rs`
  → `:182`, `:186`, `:274`.
- `git grep -n 'pub fn or_undefined' -- vm/value.rs` → `:141`.
- `git grep -n 'fn super_call_spread_propagates_argv' -- vm/tests/tests_class_machinery.rs` → `:124`.
- `wc -l vm/dispatch.rs vm/dispatch_helpers.rs vm/dispatch_class.rs vm/dispatch_ic.rs vm/natives_function.rs vm/tests/tests_class_machinery.rs`
  → 1103 / 391 / 347 / 342 / 297 / 389.

New symbols: `lay_out_call_args` (NEW), `array_argument_values` (NEW),
`tests_call_args_layout` (NEW).

---

## §7. Questions for the plan-review

- **Q1 — edge 16 by construction.** The carve-out asks for a debug assertion (an allocation epoch or
  an unconsumed tripwire). This plan makes the allocation unrepresentable (disjoint field borrow)
  and keeps the tripwire as a test (T2) rather than a `debug_assert!` in the helper. Ratify, or
  require the in-helper assertion as well.
- **Q2 — deleting `op_super_call_spread`.** The carve-out says "convert `op_super_call_spread` to
  consume it". Converting leaves a two-line function whose body is the arm; deleting it is the
  same mechanism with one fewer name. Ratify the deletion, or keep the wrapper for symmetry with
  1b's three spread handlers.
- **Q3 — where the stack-depth bound lives.** Dec. 12 puts it on the child umbrella 1b mints for the
  argument-emit helper. The push it bounds is D1's. Placing the check in `lay_out_call_args` would
  bound `super(...)` too, which is reachable today. If the plan-review moves the owner, that changes
  an owner and so updates §5 in the same PR (§5: "a plan-review result that changes the split, the
  ordering, or an owner updates this table in the same PR").
- **Q4 — owner for `super(...)`'s GetSuperConstructor ordering** (§3.1): candidates are the children
  umbrella 3 mints (it owns the super-property emit shape) or a new §8 slot. Not decided here.
- **Q5 — owner for `Function.prototype.apply`'s hole divergence** (§3.1): extend dec. 17's
  `Reflect.apply` child to cover it (same AO, same helper), or a new slot. Not decided here.
