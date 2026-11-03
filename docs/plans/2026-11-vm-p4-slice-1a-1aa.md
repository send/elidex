# VM P4 slice 1aa — the argument-layout contract (per-PR plan, rev 2)

**Parent.** Umbrella **1a** (*Call-spread VM infrastructure*), row `1a` of the §5 slice table in
`docs/plans/2026-07-vm-p4-slice-plan.md`. 1a's derivation is recorded in §6.6 of
`docs/plans/2026-07-vm-p4-slice-1a-1b-call-spread-detail.md`. It minted three terminal children in
rev 1, **1aa** (this memo), **1ab** (remove `op_array_spread`'s non-spec `return()`) and **1ac**
(root the four call-entry argument windows). The plan-review of rev 1 re-applied the terminality
measurement to 1aa and split the inline-cache entry point out as a fourth child, **1ad** (§2). This
memo is 1aa's mandatory `/elidex-plan-review` input. It is outside the `2026-07-vm-p4-*` family glob,
so family memos cite it by path in backticks, never by a relative link.

**Base.** `origin/main` = `eaa741b5`. Every `file:line` below is at that revision; crate paths are
relative to `crates/script/elidex-js/src/` unless written in full. Line numbers are evidence — §6
lists the command that re-derives each one. Every probe in this memo ran in a throwaway
`git worktree add --detach … eaa741b5` with a temporary test module, removed afterwards, under
`rustc 1.99.1 (58cf8f9c3 2026-10-13)`.

**Kind.** A terminal row (§2 records the measurement and the one open point). It lands before the
children of umbrella 1b that give its layout helper new callers, so the part with no JS-reachable
holder yet (the helper's error contract and its no-allocation window) is asserted from Rust.

**Rev 2 (plan-review R1).** D5 (the `Option<usize>` IC entry) moved to **1ad**; its design and its
tests, corrected for the dry-run's G2–G4, are handed over in §8. `Function.prototype.apply`'s
holey-array defect is fixed here (D6). The two pre-existing `super` defects of rev 1's §3.1 have a
home on Slice **S** (§3.1). Internal-error disposition, the `vm/dispatch.rs` arm shape and edge 16
were decided by the driver (§1.1, §1.3, §7).

**Overlap with a parked branch.** The local branch `vm-p4-rooting-carved` (`37a97d49`, merge-base
`6b33854d`) rewrites `vm/dispatch.rs` (386 lines changed) and `vm/dispatch_helpers.rs` (+535), and
carries a `raise` method on `VmInner` (`dispatch_helpers.rs:313` on that branch, absent from `main`), the remedy that
`#11-vm-internal-error-hard-exit` names. A revival of that branch has to account for the
`lay_out_call_args` producer this PR adds to `dispatch_helpers.rs` (§1.1).

---

## §0. Scope

### §0.1 What this PR implements

The (a) mechanism of umbrella 1a's derivation without its IC half: the in-VM contract that turns an
args array into a call's argument window, plus the one `apply` defect that shares its primitive.

- **D1 — `lay_out_call_args` (NEW).** One stack-layout operation. The args array at the top of
  `vm.stack` is replaced in place by its elements, and the element count is returned. It is the
  single place an args array becomes an argument window (§1.1).
- **D2 — `Empty` normalisation, shared with `collect_array_like`'s dense fast path (dec. 8).** One
  primitive maps an `ObjectKind::Array`'s `elements` to argument values (`JsValue::Empty` →
  `Undefined`). D1 and `Function.prototype.apply`'s dense branch both read through it (§1.2).
- **D3 — `super(...)` has one dispatch path.** `op_super_call_spread` becomes "lay out, then
  `op_super_call`", so the spread and flat `super` calls share argument hand-off and `[[Construct]]`
  dispatch. `op_super_call` takes `argc: usize`, so a spread window wider than 255 is not
  truncated (§1.3).
- **D4 — the falsified docstring is corrected.** `op_super_call_spread`'s claim that the compiler
  emits only `CreateArray; ArraySpread x; SuperCallSpread` is false. The synthesized default derived
  constructor (`compiler/expr_class.rs:145-152`) emits `GetLocal 0; SuperCallSpread` over a
  rest-packed array. That producer is spec-required (ECMA-262 §15.7.14 step 14.a.iv.1 NOTE) and is
  **not** folded (§1.4).
- **D6 — `Function.prototype.apply` takes the dense path only for a hole-free array.** A hole now
  goes through the per-index `Get` path, as CreateListFromArrayLike (§7.3.19 step 6.b) requires
  (§1.5; dec. 8 correction note, 2026-11-03).

No opcode is added, no emit site changes, and no call shape changes behaviour except `apply` over
a holey array. `f(...xs)`, `o.m(...xs)` and `new F(...xs)` are still compiled by
`compile_arguments`'s spread-as-one-argument arm after this PR; fixing them is umbrella 1b's (§5).

### §0.2 Premises re-verified at `eaa741b5`

| Premise | Verified by | Result |
|---|---|---|
| `compile_arguments` compiles `Argument::Spread(e)` as one ordinary argument | `sed -n 53,79p compiler/expr_member.rs` | true — the arm at `:70-75` |
| `super(...args)` already lowers spread to `CreateArray`/`ArrayPush`/`ArraySpread` → `Op::SuperCallSpread` | `sed -n 106,134p compiler/expr_member.rs` | true — `:113-129`; the flat branch is `:130-133` |
| `new F(...args)` has the same silent-wrong shape | `sed -n 88,93p compiler/expr.rs` | true — `compile_arguments` then `emit_u8(Op::New, argc)`; `Op::NewSpread` is a `pop; pop; push undefined` stub at `vm/dispatch.rs:879-883` with no emit site |
| `op_super_call_spread` clones `elements` without normalising `Empty` | `sed -n 72,96p vm/dispatch_class.rs` | true — `elements.clone()` at `:76` |
| `collect_array_like`'s dense branch normalises with `.or_undefined()` and takes every `ObjectKind::Array` | `sed -n 273,296p vm/natives_function.rs` | true — `:278-280` (rev 1 cited `:276-277`, which is the signature's tail); its one caller is `Function.prototype.apply` at `:76` |
| Natives run with `gc_enabled = false` | `git grep -n 'gc_enabled = false' -- crates/script/elidex-js/src/vm/interpreter.rs` | true — the native-dispatch sites `interpreter.rs:364`, `:412`, `:423`, `:502` (the other hits are comments and a test-only panic probe at `:1277`) |
| The allocation tripwire fires only while GC is enabled | `sed -n 100,108p vm/inner.rs` | true — `if self.gc_enabled && self.force_gc_before_next_alloc` at `:105` |
| No test262 harness is wired in this repository | `git ls-files \| /usr/bin/grep -ci test262` | 0 |

### §0.3 Premise errors (reported, not acted on)

1. **"Slice 1a (call-argument spread)", "the narrowest call-argument emit path first, per
   §6.2a-3".** 1a is the *VM infrastructure* half and has **no `compiler/` file** (its §5 row:
   "**no `compiler/` file** (the fold is withdrawn; edge 32 is a test)"). Every emit path, including
   `f(...xs)`, `o.m(...xs)` and `new F(...xs)`, is umbrella **1b**'s (§6.3's `[1b]` tag: "the
   compiler helper, `emit_call` aggregation, `CallMethodSpread`, the three handlers, and arity-based
   form selection — i.e. everything that changes observable behaviour"). §6.2a-3 partitions Slice
   P's `IteratorClose` sites and says nothing about the order of 1a's children.
2. **The regression list** — `Math.max(...[1,2,3])`, the `o.m(...xs)` receiver, `f(a, ...xs, b)`
   ordering, non-iterable spread → TypeError, an iterator that throws mid-way, >255 arguments after
   spread — describes 1b's emit paths. 1aa cannot pass or fail any of them through `Call` /
   `CallMethod`, because no call shape changes here. What 1aa owns of that list is its `super(...)`
   spelling, which is already reachable (§4.2). The mid-drain-throw case's `return()` half is
   **1ab**'s (edge 21).

### §0.4 What the other children and umbrella 1b own

- **1ab** — removing `op_array_spread`'s non-spec `return()` (dec. 13a), with edge 21. 1aa does not
  touch `vm/dispatch_iter.rs`.
- **1ac** — rooting `stack_slice` and `actual_args` in the four call-entry windows
  (`vm/interpreter.rs:630`, `:665`, `:867`, `:914`), with edges 27/28, and the touch-time split
  verdict on `vm/interpreter.rs` (1366 lines) that the split record assigns to umbrella 1a through
  its children (carve-out §6.6). 1aa does not touch `vm/interpreter.rs`.
- **1ad** — `ic_call` / `ic_call_method` → `call_ic_idx: Option<usize>` (dec. 11) with the direct
  `None` unit tests. Its own per-PR plan memo is owed; §8 hands over this memo's rev-1 design and
  tests, corrected. 1aa does not touch `vm/dispatch_ic.rs`, nor the `Op::Call` / `Op::CallMethod`
  arms.
- **Umbrella 1b's children** — `compile_call_arguments` / `ArgsForm`, `emit_call`, `CallSpread` /
  `CallMethodSpread` / `NewSpread` handlers (the first new consumers of D1 and the first `None`
  producers of 1ad), arity-based form selection (`f(a1…a300)`), the stack-depth bound (dec. 12), the
  I-3 tagged-template input contract, and every edge of §6.4 the carve-out does not place on 1a's
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
- **No allocation in the window, by construction.** The body borrows the two fields it needs
  disjointly, `let VmInner { stack, objects, .. } = self;` (`vm/mod.rs:182`, `:186`). Between reading
  `elements` and the last push there is then no `&mut VmInner` through which `alloc_object`
  (`vm/inner.rs:89`) could be reached. Because of the borrow, the body indexes
  `objects[id.0 as usize].as_ref()` itself instead of calling `get_object`. Edge 16 is discharged by
  this borrow plus the tripwire test T2 (§4.1), **not** by a `debug_assert!` in the helper. The
  driver ratified that in R1 (§7). The 1a row, the 1aa row and carve-out §6.6 say the same.
- **Error contract.** A top of stack that is not an `ObjectKind::Array` object is a compiler bug,
  not user error. That covers an empty stack, a primitive, a non-Array object and a freed slot
  (`objects[id]` = `None`). Each returns `Err(VmError::internal(…))` with the stack unchanged.
- **Error disposition: uncatchable.** Every caller propagates that `Err` with `?`, never through
  `throw_error`. This is the disposition `op_super_call`'s own underflow already uses
  (`vm/dispatch_class.rs:55`, `return Err(VmError::internal("super call: stack underflow"))`,
  propagated by the `Op::SuperCall` arm's `?`). A broken VM invariant must not be swallowable by user
  `try`/`catch`. §1.3 states what that changes in `op_super_call_spread`.
- **Unbounded by design here.** The helper returns `usize` and imposes no width: the Array path must
  accept `f(...arrOf1000)` (§6.3). The stack-depth bound belongs to the child umbrella 1b mints for
  the argument-emit helper (dec. 12). The helper already returns `Result`, so that child can put the
  check here without a signature change.
- **Home.** `vm/dispatch_helpers.rs` (391 lines; zero `feature = "engine"` occurrences, so the
  core-only predicate of the touch-time split record holds for the whole file). The §5 module
  column names it.

### §1.2 `Empty` normalisation, shared with `collect_array_like` (dec. 8)

`Op::ArrayHole` and indexed writes put `JsValue::Empty` into `elements`. The arrays D1 receives are
hole-free by construction: `CreateArray` + `ArrayPush` + `ArraySpread` (whose drain pushes
`IteratorStepValue` results) and rest-parameter packing never write `Empty`. So for D1 the
normalisation backstops the documented invariant rather than changing behaviour. It still has
exactly one implementation:

```rust
/// The argument values a dense Array's `elements` denote: holes read as `undefined`.
/// ArgumentListEvaluation-exact for the compiler-built arrays `lay_out_call_args` receives.
/// NOT CreateListFromArrayLike-exact for a holey array (§7.3.19 step 6.b performs a real
/// `Get`), so a CreateListFromArrayLike caller reads through it only for a hole-free array
/// (dec. 8's caller-side precondition; `collect_array_like`).
pub(super) fn array_argument_values(elements: &[JsValue]) -> impl Iterator<Item = JsValue> + '_  // (NEW)
```

D1 extends the stack from it. `collect_array_like`'s dense branch collects from it under D6's
precondition.

### §1.3 One `super(...)` path

Today the two `super` opcodes reach `dispatch_super` by different hand-offs. `Op::SuperCall` drains
`argc` stack values (`vm/dispatch_class.rs:46-60`). `Op::SuperCallSpread` pops the array and clones
`elements` (`:72-96`). After this PR, `op_super_call_spread` stays the arm's callee and is rewritten
in `vm/dispatch_class.rs` as:

```rust
pub(super) fn op_super_call_spread(&mut self, entry_frame_depth: usize) -> Result<(), VmError> {
    let argc = self.lay_out_call_args()?;
    self.op_super_call(argc, entry_frame_depth)
}
```

- **`vm/dispatch.rs` does not grow.** The touch-time split record (`2026-07-vm-p4-touch-time-split-record.md:61-75`)
  forbids growing an arm in place. Rev 1 inlined the `match` into the `Op::SuperCallSpread` arm and
  claimed the arms "do not grow". That was false: the arm went from 3 lines to 4. Now the arm
  (`vm/dispatch.rs:884-886`) is untouched. The only other `dispatch.rs` touch is the `Op::SuperCall`
  arm (`:939-942`), whose `self.op_super_call(argc, …)` becomes `self.op_super_call(argc as usize,
  …)` on the same line. **Net `dispatch.rs` line delta: 0.** (Rev 1's `Op::Call` / `Op::CallMethod`
  touch is 1ad's now.)
- **`op_super_call` changes `argc: u8` → `argc: usize`.** Without the widening,
  `super(...arrOf300)` would be truncated to 44 arguments. Today it is not, because the spread path
  never passed a count.
- **Error routing, stated per error.** (i) The internal errors on this path are uncatchable:
  `lay_out_call_args`'s `Err` (§1.1) and `op_super_call`'s underflow both propagate with `?`. Today
  `op_super_call_spread`'s two defensive branches (`:78-93`) route the same compiler-invariant break
  through `throw_error`, where a user `try`/`catch` can catch it. They are deleted with the clone, so
  **1aa adds no catchable internal error and removes two.** No JS program can reach either branch
  (the operand is compiler-built), so this changes no observable behaviour. (ii) User-facing errors
  from `dispatch_super_inner` (the `home_class` SyntaxError, the null-super TypeError,
  `construct_synchronous`'s "not a constructor") keep their route: `dispatch_super` → `throw_error`
  (`:104-113`), catchable, unchanged. (iii) **Not unified here:** `construct_synchronous`'s own
  internal error (`:321-323`, "ObjectKind::Function lost between probe and dispatch"), and any
  internal `Err` a nested `run()` returns, reach `throw_error` through `dispatch_super` and stay
  catchable. Making them uncatchable means a kind test. The remedy `#11-vm-internal-error-hard-exit`
  (`2026-07-vm-p4-slot-ledger.md`) prescribes is "the kind test occurs at **exactly one** site,
  `throw_error` itself", and its cell rejects per-site kind tests ("adding the kind test at each of
  the … call sites individually satisfies them while leaving the next extracted helper to re-open
  the decision"). **1aa does not take that slot.** Its remedy changes the disposition of every
  `throw_error` caller crate-wide, and that is not this row's charter. The slot's trigger ("with
  `#11-vm-operand-rooting-by-construction`, or the next dispatch-loop PR") matches 1aa's wording, so
  1aa fires it: the slot becomes due. It does not transfer the deliverable. The (iii) errors are
  inside that slot's single-site remedy and need no new evidence row.

### §1.4 The docstring correction (D4)

`op_super_call_spread`'s docstring is rewritten with its body. It states the contract over **both**
producers: the `super(...)` lowering (`compiler/expr_member.rs:113-129`), whose array is built by
the `ArraySpread` drain and so performs GetIterator, and the synthesized default derived constructor
(`compiler/expr_class.rs:145-152`), whose array is rest-packed at frame entry and so performs no
iteration (ECMA-262 §15.7.14 step 14.a.iv.1 NOTE). The module docstring of `vm/dispatch_class.rs`
(`:13-14`, "`Op::SuperCallSpread` consumes a pre-built args array") is corrected with it. D1 relies
on the invariant both producers share: the operand is a hole-free `ObjectKind::Array` that no user
code can reach between its construction and the opcode.

### §1.5 `apply` over a holey array (D6, dec. 8 correction)

`collect_array_like` (`vm/natives_function.rs:273-296`) takes its dense branch (`:278-280`) for
**every** `ObjectKind::Array` and maps a hole to `undefined` with `.or_undefined()`. ECMA-262
§20.2.3.1 step 4 is `? CreateListFromArrayLike(argArray)`, and §7.3.19 step 6.b is
`? Get(obj, indexName)`, which consults the prototype chain for a hole. Probe at the base:

```
Object.defineProperty(Array.prototype, 0, { get(){ return 'G' }, configurable: true });
function f(a, b){ return '' + a + ',' + b }
f.apply(null, [ , 1])          // elidex: "undefined,1"   spec: "G,1"
```

After this PR, the dense branch is taken only when no element is `Empty`, and then it collects
through `array_argument_values` (an identity on a hole-free array). A holey Array falls through to
the existing per-index `get_property_value` loop (`:281-295`), which performs the `Get`. This is
dec. 8's caller-side precondition, which dec. 8 says is where the dense/array-like distinction
lives. Dec. 8's premise that "`collect_array_like` already has the correct `Empty` normalisation"
is false for this caller: dec. 17's reasoning for `Reflect.apply` applies to `Function.prototype.apply`
too. The correction is recorded in dec. 8 (2026-11-03). The dense branch stays for the common
hole-free case.

**An adjacent defect, found while probing the precondition (reported, not homed here).** An own
accessor defined on an Array index is invisible to elidex's own `[[Get]]`. At the base,
`const a = [1, 2]; Object.defineProperty(a, 0, { get(){ return 'G' } })` succeeds, and
`typeof Object.getOwnPropertyDescriptor(a, 0).get` is `"function"`. But `a[0]` is `1`, `[...a]` is
`1,2`, `f.apply(null, a)` sees `1,2`, and `super(...a)` passes `1,2` (spec: `G` each time). The
element vector keeps the stale value beside the accessor. D6's precondition is therefore exact
*relative to elidex's `[[Get]]`*, and no 1aa test can be green for this case until that is fixed.
It sits in the property-storage layer (`ObjectKind::Array` elements against `defineProperty`), so
it is neither 1aa's nor 1a's. It is routed to the driver for an owner.

### §1.6 Lens checks

- **Layering.** All of it is engine-bound VM dispatch (`vm/dispatch_*.rs`) and one VM native
  (`vm/natives_function.rs`). No algorithm moves into `vm/host/`, and no engine-independent crate
  has an argument-layout algorithm to reuse.
- **ECS-native.** No per-entity state is introduced; nothing here is a side-store.
- **One issue, one way.** After this PR there is one args-array→window operation (D1), one
  holes-as-arguments primitive (D2) with one stated precondition for CreateListFromArrayLike callers
  (D6), and one `super` hand-off (D3). The only remaining second mechanism is the default derived
  constructor's rest-packed producer, and that is the spec-required I-3 carve-out (§6.3).
- **1000-line debt.** Touched files over 1000 lines: `vm/dispatch.rs` (1103). One arm changes on one
  line, net delta 0 (§1.3). Under 1000: `dispatch_helpers.rs` 391, `dispatch_class.rs` 347,
  `natives_function.rs` 297, `tests/tests_class_machinery.rs` 389.

### §1.7 Change list by file

| File | Change |
|---|---|
| `vm/dispatch_helpers.rs` | `lay_out_call_args` (NEW), `array_argument_values` (NEW) |
| `vm/dispatch_class.rs` | `op_super_call` takes `usize`; `op_super_call_spread` rewritten as lay out → `op_super_call` (§1.3), docstring rewritten; module docstring `:13-14` corrected |
| `vm/dispatch.rs` | `Op::SuperCall` arm (`:939-942`) widens `argc` in place; nothing else |
| `vm/natives_function.rs` | `collect_array_like`'s dense branch: hole-free precondition, reads through `array_argument_values` (D6) |
| `vm/tests/tests_call_args_layout.rs` (NEW) | §4.1 |
| `vm/tests/tests_class_machinery.rs` | §4.2 |
| `vm/tests/tests_function_apply_holes.rs` (NEW) — or the existing `Function.prototype` test module if one exists, chosen by the implementer | §4.3 |
| `vm/tests/mod.rs` | registers the new modules |

### §1.8 Commit sequence (pins first)

There is no honest red-first set for D1–D4 at the base. The Rust tests (T1–T3) name symbols that do
not exist yet, and the JS tests (C1–C7) pin behaviour that is already correct (probed, §4.2). D6 has
a real red-first test. So:

1. C1–C6 (§4.2), green at the base (probed) — committed first, so each later commit is measured
   against them.
2. A1 (§4.3), red at the base (probed), with the D6 fix in the same commit; A2 green throughout.
3. D1 + D2 with T1–T3.
4. D3 + D4. The `usize` widening lands in the same commit as the conversion, so C6 is never red.

The mutation proof each test names (§4) is run once, on the final head. Each mutant is a compiling
edit, named in §4. A revert that does not compile is not a proof.

---

## §2. Coupled invariants, and terminality re-applied

**The measurement.** §5's head: "the only evidence that an axis is an axis is that it **fails while
the others pass**", and the criterion's first disjunct asks for ≥3 such axes that intersect. This is
the measurement that split 1a. Applied literally at this slice's HEAD, over rev 1's scope (D1–D5)
plus D6:

| Invariant | Mutant that breaks it alone (compiles) | Red | Green under that mutant |
|---|---|---|---|
| **I-L (layout)** — the stack holds exactly the array's elements, in index order, where the array was; the count equals the number pushed | push `elements` reversed | T1, C3 | T2, T3, C1, C2, A1, A2 |
| **I-H (holes)** — no `JsValue::Empty` reaches an argument window | drop `.or_undefined()` from `array_argument_values` | T1 | T2, T3, C1–C6 (no compiler-built array has a hole), A1, A2 |
| **I-G (GC)** — no `ObjectId` the args array or its elements denote is held only in a Rust local across an allocation **in D1's window** | allocate a temporary JS array from the helper while GC is enabled | T2 | T1, T3, C1–C6 |
| **I-O (observability)** — laying out performs no property lookup and no iteration (§15.7.14 step 14.a.iv.1 NOTE; edge 32) | lay out through `%Array.prototype%[%Symbol.iterator%]`, leaving the array on the stack | C1 | T1, T3, C2–C6; T2 too, because the iterator objects are allocated inside natives under `gc_enabled = false` (`vm/interpreter.rs:364`; dry-run G1) |
| **I-W (width)** — no argument count passes through a `u8` on the array path | `self.op_super_call(argc as u8 as usize, …)` in `op_super_call_spread` | C6 | everything else |
| **I-A (array-like)** — `apply` over a holey Array performs §7.3.19 step 6.b's `Get` | drop D6's hole-free precondition | A1 | everything else |
| **I-C (IC)** — `None` never reads or writes `call_ic_slots`; `Some(i)` behaves as today | `None` treated as slot 0 | 1ad's T4/T7 (§8) | everything else |

**Where they intersect.** I-L, I-H, I-G and I-O are properties of one loop in one body,
`lay_out_call_args`. I-W is a property of that body's return type and of its one consumer. I-H and
I-A meet at `array_argument_values` and the precondition in front of it. I-C meets none of them
inside the PR: the `None` arm and the layout helper first meet in 1b's handlers.

**Verdict, by the measurement.** Five axes (I-L, I-H, I-G, I-O, I-W) fail alone and intersect at
`lay_out_call_args`, so the first disjunct is **met**: rev 1's scope was edge-dense. Rev 1's "two
axes" counted mechanisms (D1–D4 versus D5) where the criterion asks for axes, and its own pairwise
list (I-L×I-G, I-L×I-H, I-L×I-O, I-L×I-W, I-H×I-O) already showed the intersection. Per the
driver's ruling (PM 285), **D5 is split out as 1ad**, its own terminal child of 1a. I-C intersects
nothing, so 1ad is one axis. It gets its own per-PR plan memo, with §8 as its input.

**⚠ Open — the measurement does not end at the split.** Without D5, the remainder (D1–D4 + D6)
still carries ≥3 axes that fail alone and intersect at `lay_out_call_args`. Read literally, the
measurement still says edge-dense. The driver's ruling settles D5; it does not settle the
remainder. **Recommendation (not applied as a decision; §7 Q1): the remainder is terminal by
necessity**, on two in-tree rules:
- The family's own precedent, `#11-vm-operand-rooting-by-construction`: "a row that cannot be split
  without defeating its deliverable is terminal by necessity". Here I-L, I-H, I-G and I-O are four
  properties of one ~20-line loop, and I-W is its signature. Every partition would land that loop
  missing one of its properties while D3 makes it live (`super(...)` is reachable today), or would
  land it dead.
- CLAUDE.md's base case (c): a narrowly scoped per-PR slice under an approved umbrella that has
  passed plan-review is a terminal unit, "per-PR slice が同 subsystem を触ること自体は再分割 trigger
  にしない".

I-A (D6) could be cut out as a sixth child: it fails alone and touches only `collect_array_like`.
It intersects I-H at the shared primitive, and the driver placed it on 1aa. That placement stands
unless Q1 is answered by re-splitting.

**Pairwise intersections that are real in this PR** (the reviewer's check list):

- **I-L × I-G** — the window in which elements move from the heap to the stack is the one with no
  `&mut VmInner`. Reordering the body so the array is popped before its elements are read would
  reopen it.
- **I-L × I-H** — the push and the normalisation are one iterator (`array_argument_values`), so the
  stack cannot receive an un-normalised element.
- **I-L × I-O** — the layout reads `elements` directly. An implementation through the iterator
  protocol satisfies I-L and I-H and breaks I-O (edge 32's mutant).
- **I-L × I-W** — the count D1 returns is `usize`, and its only consumer in this PR (`op_super_call`)
  takes `usize`.
- **I-H × I-A** — the normalisation is ArgumentListEvaluation-exact, not CreateListFromArrayLike-exact.
  D6's precondition is what makes sharing it with `collect_array_like` exact.

**I-G's scope.** I-G covers D1's window only. A second, pre-existing window sits downstream:
`op_super_call` drains the laid-out values into `call_args: Vec<JsValue>` (`vm/dispatch_class.rs:58`),
and they are held only there until `construct_synchronous` re-pushes them (`:304-306`) for the JS
branch, or for the whole native constructor body on the native branch (`call_dispatch`, `:330`).
The flat `Op::SuperCall` path has always had this window, and D3 now routes spread elements through it
too, as `elements.clone()` did before. Evidence that nothing allocates in it on the JS branch (dry
run, read at `eaa741b5`): `unwrap_bound_function_chain`, the `combined_args` `Vec`,
`extract_js_callee`, `with_call_mode` and the `VmError` constructors call no `alloc_object`. On the
native branch the native body runs with `gc_enabled = false` (`vm/interpreter.rs:364`). Whether a
native constructor that re-enters user code keeps GC disabled for that code was not measured, and it
is not changed here. It belongs to `#11-vm-operand-rooting-by-construction` (§7 Q3).

---

## §3. Spec coverage map

Skeleton generated with `.claude/tools/webref coverage-map ecma262 ArgumentListEvaluation ecma262
ClassDefinitionEvaluation ecma262 Construct ecma262 CreateListFromArrayLike ecma262
GetSuperConstructor` and `.claude/tools/webref coverage-map ecma262 13.3.7.1 ecma262 20.2.3.1`
(those two anchors are not AO names, so they are passed as § numbers). Every § number below is
`webref aoid` / `webref heading` output, and every step is `webref body` output (2026-11-03):
`aoid` → ArgumentListEvaluation §13.3.8.1, EvaluateCall §13.3.6.2, CreateListFromArrayLike §7.3.19,
ClassDefinitionEvaluation §15.7.14, GetSuperConstructor §13.3.7.2, ArrayAccumulation §13.2.4.1,
IsConstructor §7.2.4, Call §7.3.13, Construct §7.3.14; `heading 13.3.7` → §13.3.7.1 Runtime
Semantics: Evaluation; `heading 20.2.3.1` → Function.prototype.apply. EvaluateCall's rows moved with
D5 to 1ad (§8).

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| ECMA-262 §13.3.8.1 Runtime Semantics: ArgumentListEvaluation | `Arguments : ( )` step 1 | empty List | `super(...[])` → window of 0 (C2); flat `super()` → `Op::SuperCall 0` | ✓ | yes |
| ECMA-262 §13.3.8.1 Runtime Semantics: ArgumentListEvaluation | `ArgumentList : AssignmentExpression` steps 1-3; `ArgumentList : ArgumentList , AssignmentExpression` steps 1-4 | non-spread elements, in order | flat `super(a, b)`: compiled in order, `Op::SuperCall argc`; mixed `super(a, ...xs, b)`: `ArrayPush` (`compiler/expr_member.rs:116-127`) — producer unchanged; consumer `lay_out_call_args` preserves index order (I-L, C3) | ✓ | yes |
| ECMA-262 §13.3.8.1 Runtime Semantics: ArgumentListEvaluation | `ArgumentList : ... AssignmentExpression` steps 1-5; `ArgumentList : ArgumentList , ... AssignmentExpression` steps 1-4 | normal completion: the List is the drain order, `precedingArgs` first | producer `ArraySpread` → `op_array_spread` (unchanged); consumer `lay_out_call_args` preserves index order (I-L) | ✓ | yes |
| ECMA-262 §13.3.8.1 Runtime Semantics: ArgumentListEvaluation | `... AssignmentExpression` step 4 / `ArgumentList , ... AssignmentExpression` step 3 | `? GetIterator(…, sync)` throws: non-iterable spread operand → TypeError | producer `op_array_spread` → `resolve_iterator` (unchanged; not on 1aa's consumer side) | ✓ (probed, §3.2) | yes |
| ECMA-262 §13.3.8.1 Runtime Semantics: ArgumentListEvaluation | `... AssignmentExpression` step 5.a / `ArgumentList , ... AssignmentExpression` step 4.a | `? IteratorStepValue` throws → the error propagates | producer `op_array_spread`'s drain (unchanged): propagation ✓ (probed, §3.2); the extra `return()` it calls on that path is ✗ and is **1ab**'s (edge 21) | ✗ (`return()` half; 1ab) | yes |
| ECMA-262 §13.3.7.1 Runtime Semantics: Evaluation | `SuperCall` steps 1-2 | GetNewTarget; Assert constructor | `dispatch_super_inner` reads the frame's `CallMode::Construct { new_target }` (`vm/dispatch_class.rs:132-135`) — unchanged | ✓ | no |
| ECMA-262 §13.3.7.1 Runtime Semantics: Evaluation | `SuperCall` step 3 → step 4 order | superCtor is fixed **before** the arguments are evaluated | — `dispatch_super_inner` resolves `home.prototype` (`:121-124`) when the opcode runs, after the arguments | ✗ (pre-existing; probed; homed on Slice S, §3.1) | yes |
| ECMA-262 §13.3.7.1 Runtime Semantics: Evaluation | `SuperCall` step 4 | `argList` from ArgumentListEvaluation | flat: `Op::SuperCall` (`u8` operand) → `op_super_call(usize)`; spread: `op_super_call_spread` → `lay_out_call_args` (NEW) → `op_super_call` | ✓ | yes |
| ECMA-262 §13.3.7.1 Runtime Semantics: Evaluation | `SuperCall` step 5 | IsConstructor(superCtor) false → TypeError | `construct_synchronous` accepts every `ObjectKind::Function` (`:267`, `:272`), with no arrow / method / generator / async test | ✗ (pre-existing; probed; homed on Slice S, §3.1) | yes |
| ECMA-262 §13.3.7.1 Runtime Semantics: Evaluation | `SuperCall` step 6 | `? Construct(superCtor, argList, newTarget)` | `construct_synchronous(super_class, receiver, args, CallMode::Construct { new_target }, pre_alloc)` (`:139-145`) — unchanged | ✓ | yes |
| ECMA-262 §13.3.7.1 Runtime Semantics: Evaluation | `SuperCall` steps 7-9 | GetThisEnvironment; `? BindThisValue(thisER, result)` | `dispatch_super_inner` *assigns* `this_value` / `new_instance` (`:150-154`), with no `[[ThisBindingStatus]]` | ✗ (pre-existing; owner `#11-vm-derived-constructor-this-binding`, and Slice S for the arrow spelling) | yes |
| ECMA-262 §13.3.7.1 Runtime Semantics: Evaluation | `SuperCall` steps 10-12 | `? InitializeInstanceElements(result, funcObj)` | — not touched | n/a (instance-element initialisation is umbrella 2a's — row 2ab) | yes |
| ECMA-262 §13.3.7.1 Runtime Semantics: Evaluation | `SuperCall` step 13 | return result | `self.stack.push(result)` (`:156`) — unchanged | ✓ | no |
| ECMA-262 §13.3.7.2 GetSuperConstructor ( ) | steps 1-4 | `GetThisEnvironment()` → `[[FunctionObject]]` → `[[GetPrototypeOf]]()` | `home_class` → `.prototype` (`vm/dispatch_class.rs:116-125`) — unchanged; correct for a constructor frame. Through an arrow it is ✗ and is Slice S's | ✓ (constructor frame) | yes |
| ECMA-262 §7.3.14 Construct ( ctor [ , argList [ , newTarget ] ] ) | step 1 | `newTarget` defaults to `ctor` | — every `super` caller passes `newTarget` | n/a (never absent on this path) | no |
| ECMA-262 §7.3.14 Construct ( ctor [ , argList [ , newTarget ] ] ) | steps 2-3 | (Assert IsConstructor) `ctor.[[Construct]](argList, newTarget)` | `construct_synchronous`, reached from SuperCall step 6 / ClassDefinitionEvaluation 14.a.iv.4 with `newTarget` explicit — unchanged. Its JS-branch gate is the step-5 ✗ above | ✓ dispatch; ✗ gate (row above) | yes |
| ECMA-262 §15.7.14 Runtime Semantics: ClassDefinitionEvaluation | step 14.a.i | `args` = the List passed by [[Call]] / [[Construct]] | rest-param packing into local 0 (`compiler/expr_class.rs:145-152`, unchanged) → `lay_out_call_args` | ✓ | yes |
| ECMA-262 §15.7.14 Runtime Semantics: ClassDefinitionEvaluation | step 14.a.ii | NewTarget undefined → TypeError | the class-constructor call-mode guard on the call path — not touched | ✓ (probed, §3.2) | yes |
| ECMA-262 §15.7.14 Runtime Semantics: ClassDefinitionEvaluation | step 14.a.iv.1 NOTE | must **not** call `%Array.prototype%[%Symbol.iterator%]` | `lay_out_call_args` reads `elements` directly (I-O); edge 32 (C1) | ✓ | yes |
| ECMA-262 §15.7.14 Runtime Semantics: ClassDefinitionEvaluation | step 14.a.iv.2 | func = `! ctorFunc.[[GetPrototypeOf]]()` | `home_class` of the default ctor frame = the class → `.prototype` — unchanged | ✓ | yes |
| ECMA-262 §15.7.14 Runtime Semantics: ClassDefinitionEvaluation | step 14.a.iv.3 | IsConstructor(func) false → TypeError | the same `construct_synchronous` gate as SuperCall step 5 | ✗ (pre-existing; probed; homed on Slice S, §3.1) | yes |
| ECMA-262 §15.7.14 Runtime Semantics: ClassDefinitionEvaluation | step 14.a.iv.4 | `? Construct(func, args, NewTarget)` | `construct_synchronous` — unchanged | ✓ | yes |
| ECMA-262 §15.7.14 Runtime Semantics: ClassDefinitionEvaluation | step 14.a.vi | `? InitializeInstanceElements(result, ctorFunc)` | — not touched | n/a (umbrella 2a's — row 2ab) | yes |
| ECMA-262 §20.2.3.1 Function.prototype.apply ( thisArg, argArray ) | step 4 | `? CreateListFromArrayLike(argArray)` | `collect_array_like` (D6) | ✓ (after D6) | yes |
| ECMA-262 §7.3.19 CreateListFromArrayLike ( obj [ , validElementTypes ] ) | step 6.b | `next` = `? Get(obj, indexName)` — consults the prototype chain for a hole | dense branch only for a hole-free Array (D6); otherwise the per-index `get_property_value` loop | ✓ (after D6; red at the base, A1) | yes |

**Breadth**: K=1 (ECMA-262), M=25. The preflight prints SPLIT-RECOMMENDED (M ≥ 20); rev 1 had M=16
("ok"). The rows added in rev 2 are the R1 completeness rows (abrupt branches, non-spread
productions, SuperCall steps 1-2 and 7-13, ClassDefinitionEvaluation 14.a.ii / 14.a.vi, Construct
step 1). Most of them are sites this PR does not touch ("unchanged", n/a, or a pre-existing ✗ with an
owner). The rows whose touch column names a site this PR changes are 8: the three ArgumentListEvaluation
consumer rows, `SuperCall` step 4, 14.a.i, 14.a.iv.1 NOTE, §20.2.3.1 step 4 and §7.3.19 step 6.b.
Breadth is not the split criterion. §2's measurement is, and §2 records its verdict and the open
point.

### §3.1 User-input touch audit, and the pre-existing defects

Sites this PR touches that user-controlled values flow through:

- `op_super_call_spread` / `lay_out_call_args`: the element values are user values (spread operand
  or the arguments of `new B(…)`), and their count is user-controlled (`super(...arrOf1e6)`). The
  array itself is compiler-built and unreachable from user code. Exposure change: **none**. The same
  values reach `dispatch_super` in the same order, and the count was already unbounded (the current
  `elements.clone()` + `construct_synchronous`'s re-push).
- `collect_array_like`: a user array via `Function.prototype.apply`. Exposure change: a holey Array
  now runs user getters on the prototype chain, which is what the spec requires (D6).

Pre-existing defects found while auditing, probed at the base. Neither is created or widened here.
Both have the same home, judged on the criterion "which existing row's change already rewrites the
superCtor resolution site":

- **D-S1 — `super(...)` resolves the super constructor after its arguments.** §13.3.7.1 `SuperCall`
  fixes `superCtor` at step 3, before ArgumentListEvaluation at step 4. `dispatch_super_inner` reads
  `home.prototype` (`vm/dispatch_class.rs:121-124`) when the opcode runs. Probe:
  `class A { constructor(){ this.k = 'A' } } class C { constructor(){ this.k = 'C' } } class B extends A { constructor() { super(Object.setPrototypeOf(B, C)) } } new B().k`
  → `"C"` at the base (spec: `"A"`). The spread spelling `super(...[Object.setPrototypeOf(B, C)])`
  also gives `"C"`. Both `super` opcodes share it. A fix resolves before the arguments, which is an
  emit change and outside 1a's charter.
- **D-S2 — the super constructor's IsConstructor test is missing for JS functions.** SuperCall step 5
  and ClassDefinitionEvaluation 14.a.iv.3 throw a TypeError when the super constructor is not a
  constructor. `construct_synchronous` (`:267`, `:272`) admits every `ObjectKind::Function`, unlike
  `do_new` (`vm/ops.rs:709-713`, which rejects `ThisMode::Lexical`) and unlike the crate's
  `is_constructor` (`vm/object_kind.rs:1673`, which also rejects async and generator functions, and
  is `#[cfg(feature = "engine")]`). Probes, each
  `class A{}; class B extends A{}; Object.setPrototypeOf(B, X); try { new B(); 'no-throw' } catch (e) { 'threw ' + (e instanceof TypeError) }`:
  `X = ()=>{}` → `"no-throw"`; `X = ({m(){}}).m` → `"no-throw"`; `X = function*(){}` → the test build
  **panics** on `debug_assert_eq!` "Ok return leaked 0 frame(s) above entry 2"
  (`vm/interpreter.rs:210`). The hand-written `constructor(...a){ super(...a) }` and the flat
  `constructor(){ super(1) }` spellings with `X = ()=>{}` also give `"no-throw"`. Control:
  `new (()=>{})()` → `"threw true"`. ⚠ Whether `do_new` admits a method or a generator was not probed.
  Its check reads only `this_mode`, and the slot's plan-review has to measure that before choosing
  one predicate for both paths (*One issue, one way*).

**The home: Slice S (`#11-vm-arrow-lexical-super`), judged (ii).** S's deliverable replaces the
superCtor resolution site: "that link carried on the closure rather than read off the current frame".
That frame read is `dispatch_super_inner`'s `self.frames[frame_idx].home_class` (`:116-119`), the
first line of GetSuperConstructor. The other candidates are weaker on that criterion.
(i) `#11-vm-derived-constructor-this-binding` rewrites the receiver read and the substitution
(`:136`, `:150-154`, SuperCall steps 7-9), not the resolution. (iii) A new row would be a second home
for the resolution site, which S rewrites anyway. The IsConstructor test (D-S2) is SuperCall step 5,
applied to the value step 3 resolved. Once the ordering moves resolution before the arguments, the
test has to read the resolved value from wherever that move puts it. So D-S2 goes with D-S1, and
one home only. The two defects, their repros, owner and trigger are written in S's ledger row
(`2026-07-vm-p4-slot-ledger.md`) and pointed to from S's §5 row. That row's Re-eval stays *at
registration: landing + 30 d* (§7 Q5).

**⚠ What that home does to S's own kind (open, §7 Q2).** S's §5 row reads "one axis". By §2's
measurement, the lexical link, the step-3 ordering and the step-5 test each fail while the other two
pass, and all three meet at the superCtor resolution site: three axes that intersect. The S row now
says that its terminality has to be re-applied before it is scheduled, and **does not decide it**.

Measured at HEAD `41a45a43` before this revision:
`/usr/bin/grep -c 'GetSuperConstructor' docs/plans/2026-07-vm-p4-*.md` is 1 in each of four
files, so **4** lines: three about `super` through an arrow, and one that is carve-out §6.6's seed
list. Rev 1 said 3, which was measured before §6.6 was written. None of the four owned D-S1 or D-S2.

### §3.2 Probes behind the cells marked "probed, §3.2"

These rows concern sites 1aa does not touch. Each was probed at the base, so the mark is a measurement:

- ArgumentListEvaluation `? GetIterator` on a non-iterable operand:
  `class A {} class B extends A { constructor(){ super(...1) } } try { new B(); 'no-throw' } catch (e) { 'threw ' + (e instanceof TypeError) }`
  → `"threw true"`.
- `? IteratorStepValue` propagation:
  `const it = { [Symbol.iterator](){ return { next(){ throw new RangeError('boom') } } } }; class A {} class B extends A { constructor(){ super(...it) } }`,
  then `new B()` inside `try` → `"threw true boom"` (`e instanceof RangeError`, message kept).
- ClassDefinitionEvaluation 14.a.ii:
  `class A {} class B extends A {} try { B(); 'no-throw' } catch (e) { 'threw ' + (e instanceof TypeError) }`
  → `"threw true"`.

---

## §4. Tests

Supported surface: elidex-js unit and VM integration tests (`cargo test -p elidex-js
--all-features`). There is no test262 subset in this repository (§0.2). Each test names a mutant
that compiles and that the test kills, so none is satisfiable by an implementation that does not do
the thing.

### §4.1 `vm/tests/tests_call_args_layout.rs` (NEW) — from Rust

- **T1 layout.** Push a sentinel. Allocate `o1` and push it too, so it is rooted before anything
  else allocates. Then allocate an Array `[o1, Empty, 3]` and push it. The stack is
  `[sentinel, o1, array]`. `lay_out_call_args()` → `Ok(3)`, and the stack is
  `[sentinel, o1, o1, undefined, 3]`. Compare with `==` (`JsValue: PartialEq`, so
  `Empty != Undefined` is observable). Kills: reversed order (I-L), dropped normalisation (I-H), an
  off-by-one count, the array left on the stack.
- **T2 edge 16 — no VM-side allocation in the window while GC is enabled.** Precondition, asserted
  first: `gc_enabled == true` (the tripwire fires only under it, `vm/inner.rs:105`). Same setup as
  T1. Arm `force_gc_before_next_alloc` (`vm/mod.rs:274`) after setup, call `lay_out_call_args()`, and
  assert the one-shot is still armed. Paired control in the same test: the same arming followed by
  one `alloc_object` consumes it, so the probe can observe an allocation. Kills: an implementation
  that makes a **VM-side allocation while GC is enabled** between reading `elements` and the last
  push, e.g. `self.create_array_object(…)` from the helper. It does **not** kill a layout through the
  iterator protocol, whose ArrayIterator and `{value, done}` objects are allocated inside natives
  under `gc_enabled = false` (dry-run G1). C1 kills that mutant.
- **T3 error contract.** Four inputs, each `Err` with `e.kind == VmErrorKind::InternalError` and the
  stack unchanged (compared before and after): an empty stack; a primitive on top; an ordinary
  non-Array object on top; an `Object(id)` whose slot is freed (`objects[id]` = `None`, set up by
  allocating, dropping every root, and collecting). Kills: a helper that pops before validating, and
  an indexing slip that panics on the freed slot.

### §4.2 `vm/tests/tests_class_machinery.rs` — from JS (no-regression; probed green at the base)

All of C1–C6 were run at `eaa741b5` (§6, "Probes") and are green there.

- **C1 edge 32.** Replace `%Array.prototype%[Symbol.iterator]` with a counting wrapper of the
  original. Count per construction: reset or take a delta, so the order does not matter.
  `class A { constructor(a, b) { this.s = a + b } } class B extends A {}`: `new B(1, 2).s === 3`
  **and** the count is 0. **Positive control in the same test**: `class C extends A {
  constructor(...args) { super(...args) } }`; `new C(1, 2).s === 3` and the count is 1. Probe at
  the base: `"3 0 3 1"`. Kills: a layout through the iterator protocol (I-O). The control alone
  cannot tell that mutant from a broken spread path.
- **C2** `super(...[])` → `A` sees `arguments.length === 0` (probe: `0`).
- **C3** `super(1, ...xs, 4)` with `xs = [2, 3]` → `A` sees `[1,2,3,4]` in order (probe:
  `"[1,2,3,4]"`).
- **C4** native base. `class B extends Array {}`: `new B(1, 2, 3)` has `length === 3`, is
  `instanceof B`, and `Array.isArray` holds (probe: `"3 true true"`). The hand-written
  `constructor(...xs){ super(...xs) }` spelling over `Array` gives `length === 3`, `instanceof B`
  (probe: `"3 true"`). Covers the native branch of `construct_synchronous` through the converted
  handler. Green at the base, so no native-base subclassing gap needs a home.
- **C5** bound base.
  `function A(...a){ this.s = JSON.stringify(a) } const BA = A.bind(null, 0); BA.prototype = A.prototype; class B extends BA { constructor(...a) { super(...a) } } new B(1, 2).s`
  → `"[0,1,2]"`, the bound-prefix splice composed with the laid-out window. Rev 1's spelling,
  `class B extends A.bind(null, 0)`, throws at class definition, because a bound function has no
  `prototype` (`vm/dispatch.rs:960-977`), and the existing test
  `extends_bound_constructor_throws_on_undefined_prototype` (`:340`) pins that throw. Assigning
  `BA.prototype` works at the base (probe: `BA.prototype === A.prototype` → `true`), and so does the
  default-ctor spelling `class B extends BA {}` (probe: `"[0,1,2]"`). Both spellings go in the test.
- **C6 width.** `super(...xs)` with 300 elements → `A` sees `arguments.length === 300` and
  `arguments[299] === 299` (probe: `"300 299"`). Kills: "truncate `argc` to `u8` in
  `op_super_call_spread`", i.e. `self.op_super_call(argc as u8 as usize, …)`. That mutant compiles,
  and C6 turns red under it (I-W). The flat `Op::SuperCall` arm's `as usize` widening cannot be
  killed by any test, by design: its operand is a `u8` (`read_u8_op()`), so no flat spelling can
  carry more than 255. That is stated rather than implied as coverage.
- **C7** the existing `super_call_spread_propagates_argv` (`:124`) is kept unchanged.

### §4.3 `apply` over a holey array (D6)

- **A1** (red at the base, probe `"undefined,1"`). Define a getter on `Array.prototype[0]`
  (`configurable: true`, removed at the end); `f.apply(null, [ , 1])` → `f` sees `"G,1"`. Kills:
  dropping D6's precondition.
- **A2** dense control (green at the base, probe `"7,1"`): `f.apply(null, [7, 1])` → `"7,1"`, with the
  same getter installed, so an own element is shown to win over the prototype chain. Kills: a fix
  that sends every Array through the generic path and reads the getter for an own element too.
- No own-accessor test: it is red at the base for a reason outside this PR (§1.5).

Not here, and why: edges 21 and 27/28 are 1ab's and 1ac's (carve-out §6.6). The `None` IC tests are
1ad's (§8). The brief's call-shape regressions belong to 1b's children (§0.3 item 2). The existing
suite passing is necessary and not sufficient: it covers neither I-O nor D6, which is why T2, C1 and
A1 exist.

---

## §5. Non-goals, and what the next slices own

- **`f(...xs)` / `o.m(...xs)` / `new F(...xs)` / optional-call spread** — umbrella 1b's children.
  `compiler/expr_member.rs:70-75` and `compiler/expr.rs:88-92` are not touched, so `Math.max(...[1,2,3])`
  is still `NaN` after this PR. `Op::CallSpread` / `Op::NewSpread` stay stubs
  (`vm/dispatch.rs:879-883`) until those children connect them to D1 + 1ad (and `do_new`).
- **The IC `Option<usize>` entry** — 1ad (§8).
- **`op_array_spread`'s `return()`** — 1ab.
- **The four argument windows in `vm/interpreter.rs`**, and that file's touch-time split verdict — 1ac.
- **Stack-depth bound** — dec. 12 places it on the child umbrella 1b mints for the argument-emit
  helper.
- **D-S1 / D-S2** — Slice S (§3.1).
- **Catchable internal errors elsewhere on the `super` path** — `#11-vm-internal-error-hard-exit` (§1.3).
- **Folding the default derived constructor onto `ArraySpread`** — forbidden (§15.7.14 step
  14.a.iv.1 NOTE); C1 pins it.

### §5.1 Per-function citation table (the docstrings this PR writes)

| Function | Citation in its docstring | Steps it implements |
|---|---|---|
| `lay_out_call_args` (NEW) | [C19] ECMA-262 §13.3.8.1 ArgumentListEvaluation (consumer side); §15.7.14 ClassDefinitionEvaluation step 14.a.iv.1 NOTE (no iteration) | none of ArgumentListEvaluation's own steps: it lays out a List already evaluated |
| `array_argument_values` (NEW) | §13.3.8.1 (holes as `undefined`); §7.3.19 CreateListFromArrayLike step 6.b, as the precondition it does **not** meet for a holey array | — |
| `op_super_call_spread` (rewritten) | [C13] ECMA-262 §13.3.7.1 `SuperCall : super Arguments` step 4 (argList); §15.7.14 step 14.a.iv.1 NOTE (the second producer) | step 4's hand-off; steps 3, 5, 6 via `dispatch_super` |
| `op_super_call` (re-signed) | [C13] §13.3.7.1 `SuperCall` steps 3-6 (unchanged text; the D-S1 ordering divergence named) | 3, 5 (✗ D-S2), 6 |
| `collect_array_like` | ECMA-262 §20.2.3.1 Function.prototype.apply step 4 → §7.3.19 CreateListFromArrayLike steps 2-7 | the generic loop: 3, 6.a, 6.b, 6.d; the dense branch: a hole-free fast path equal to steps 3-7 |
| `ic_call` / `ic_call_method` | 1ad's: §13.3.6.2 EvaluateCall steps 4-5 and 7 (and steps 1-2 for `ic_call_method`'s receiver) — §8 | — |

---

## §6. Structural references — re-derivation commands

All at `eaa741b5`, from `crates/script/elidex-js/src/`:

- `git grep -n 'fn compile_arguments\|Argument::Spread' -- compiler/expr_member.rs` → `:53`, `:70`,
  and the super-spread arm's `:123`.
- `git grep -n 'ExprKind::New' -- compiler/expr.rs` → `:88`.
- `git grep -n 'Op::SuperCallSpread as u8' -- compiler/expr_class.rs` → `:149` (inside `:145-152`).
- `git grep -n 'fn op_super_call\|fn dispatch_super\|fn construct_synchronous\|vm.stack.push(JsValue::Object(ctor_id))' -- vm/dispatch_class.rs`
  → `:46`, `:72`, `:104`, `:115`, `:232`, `:304`.
- `git grep -n 'stack underflow\|VmError::internal(\|let is_js = \|if !is_js && !is_native_ctor' -- vm/dispatch_class.rs`
  → `:55`, `:79`, `:88`, `:267`, `:272`, `:321`.
- `git grep -n 'Op::SuperCall =>\|Op::SuperCallSpread =>\|Op::CallSpread | Op::NewSpread' -- vm/dispatch.rs`
  → `:939`, `:884`, `:879`.
- `git grep -n 'fn collect_array_like\|collect_array_like(ctx\|ObjectKind::Array { elements } = &obj.kind\|let length_key' -- vm/natives_function.rs`
  → `:273`, `:76` (1 caller), `:278`, `:124`, `:155`, `:281` (the `:124` / `:155` hits are other natives).
- `git grep -n 'Arrow functions are not constructors' -- vm/ops.rs` → `:709`.
- `git grep -n 'pub(crate) fn is_constructor' -- vm/object_kind.rs` → `:1673`.
- `git grep -n 'pub(crate) fn alloc_object\|force_gc_before_next_alloc' -- vm/inner.rs` → `:89`, `:105`, `:106` (the one-shot is `#[cfg(test)]`, `vm/mod.rs:273`).
- `git grep -n 'pub(crate) stack:\|pub(crate) objects:\|pub(crate) force_gc_before_next_alloc' -- vm/mod.rs`
  → `:182`, `:186`, `:274`.
- `git grep -n 'pub fn or_undefined' -- vm/value.rs` → `:141`.
- `git grep -n 'fn super_call_spread_propagates_argv\|fn extends_bound_constructor_throws_on_undefined_prototype' -- vm/tests/tests_class_machinery.rs`
  → `:124`, `:340`.
- `wc -l vm/dispatch.rs vm/dispatch_helpers.rs vm/dispatch_class.rs vm/natives_function.rs vm/tests/tests_class_machinery.rs`
  → 1103 / 391 / 347 / 297 / 389.

New symbols: `lay_out_call_args` (NEW), `array_argument_values` (NEW),
`tests_call_args_layout` (NEW).

**Probes** (each in a throwaway `git worktree add --detach … eaa741b5` with a temporary
`vm/tests/tests_zz_probe.rs` that calls `super::eval` / `eval_string` / `eval_number` /
`eval_bool`, run with
`env -u RUSTUP_TOOLCHAIN cargo test -p elidex-js --all-features --lib tests_zz_probe -- --nocapture --test-threads=1`,
worktree removed afterwards). The JS of each probe is quoted where its result is cited: §1.5 (A1/A2),
§1.5 (the own-accessor case), §3.1 (D-S1, D-S2), §3.2, §4.2 (C1–C6).

---

## §7. Questions for the plan-review (rev 2)

R1's questions are settled: **Q1** (edge 16): the structural approach is ratified, with the
disjoint borrow and the T2 tripwire and no `debug_assert!` (§1.1). **Q2** (the wrapper): kept, as
"lay out, then `op_super_call`" in `vm/dispatch_class.rs`, so `vm/dispatch.rs` does not grow (§1.3).
**Q3** (stack-depth bound): dec. 12 keeps it on 1b's argument-emit child. **Q4/Q5** (owners): D-S1
and D-S2 go to Slice S, and the `apply` hole is fixed here. Open:

- **Q1 — is the remainder terminal?** §2: by the literal measurement, D1–D4 + D6 still carries five
  intersecting axes after D5's split. The recommendation is "terminal by necessity" (the
  operand-rooting precedent and CLAUDE.md's base case (c)); the alternative is a further split,
  with D6 the only part separable without cutting `lay_out_call_args`. A ruling is needed before
  implementation.
- **Q2 — S's kind.** Homing D-S1/D-S2 on S gives S three intersecting axes (§3.1). S's row now says
  its terminality has to be re-applied before it is scheduled. Who re-applies it, and when: S's own
  plan-review, or the driver now?
- **Q3 — the native-constructor re-entry window** (§2, I-G's scope): a native constructor that calls
  user code (e.g. an executor) while `op_super_call`'s `call_args` holds the only reference to the
  arguments. It is pre-existing and not changed here. It is recorded as a question for
  `#11-vm-operand-rooting-by-construction`'s plan-review rather than measured.
- **Q4 — `#11-vm-operand-rooting-by-construction`'s trigger** ("now — the next VM dispatch-loop PR,
  or …", ledger). 1aa edits a dispatch-loop arm, so by the wording it fires the trigger: the slot is
  due. That was already its priority ("now"). 1aa does not take the deliverable. The slot is a
  type-level guard over ~20 arms whose cell forbids partitioning it, and 1aa has no artifact edge on
  it, because I-G is held by D1's borrow (no hold the guard would express: no user code runs in D1's
  window). 1ac's `Deps` on it is unchanged. Ratify.
- **Q5 — Re-eval of the S home.** The brief asked for "Re-eval 2026-12-01". The ledger's Re-eval rule
  (`2026-07-vm-p4-slot-ledger.md`, "no row of this table carries an absolute date, except the one
  row marked *to retire*") forbids it, and the home is an existing row whose Re-eval is *at
  registration: landing + 30 d*. So no date was written.

---

## §8. Handed to 1ad — input for its per-PR plan (not 1aa scope)

1ad's plan memo is owed and is not written in this round. What rev 1 designed for D5, corrected for
the dry-run's G2–G4 and the R1 notes, is recorded here so it is not lost. 1ad's plan-review owns it
from here.

**Design (rev 1 §1.5, unchanged).** `ic_call(caller_func_id, argc, call_ic_idx: Option<usize>)`, and
the same for `ic_call_method`. `try_call_ic` and `populate_call_ic` keep `usize` and are called only
under `Some`: `call_ic_idx.and_then(|i| self.try_call_ic(caller_func_id, i, callee_id))` on entry,
and `if let Some(i) = call_ic_idx { self.populate_call_ic(…, i, …) }` at the population call sites.
The IC-miss JS branch, the native branch and the non-callable branch stay one body (dec. 11). The two
callers (`vm/dispatch.rs:705`, `:716`) pass `Some(call_ic_idx)` on the same line (net 0).
**Wording correction (G3):** rev 1 said "the two population points". `populate_call_ic` is called
from the IC-miss JS branch (`vm/dispatch_ic.rs:251`, `:310`) and the native branch (`:259`, `:323`),
but `collect_call_ic` (`vm/ops_property.rs:282-294`) returns `None` for any callee that is not an
`ObjectKind::Function`. So only the JS branch's call **writes** a slot, under `Some` or `None`.

**Spec rows (moved from rev 1 §3):** §13.3.6.2 EvaluateCall steps 1-2 (thisValueRef → thisValue:
`ic_call_method`'s receiver at `stack[args_start-2]`, `ic_call`'s `compute_this_for_call`), steps 4-5
(not an Object / not callable → TypeError, the non-Object route at `vm/dispatch_ic.rs:229` before any
IC code, the non-callable Object through `self.call`), step 6 n/a (no tail calls), step 7
(`Call(func, thisValue, argList)`); §7.3.13 Call steps 2-3.

**Tests, corrected:**
- **T4 `ic_call(.., None)`, JS callee.** The caller must have ≥1 call IC slot. Either its source
  contains a real call site (e.g. a top-level `f(0, 0)`; slot count = `call_ic_slot_count`,
  `compiler/function.rs:410`), or a slot is pushed by hand as `vm/gc/tests.rs:273` does. A script
  that only defines `f` has zero slots, and then every assertion is vacuous (G2.1). **Order:**
  snapshot slot `i` while it is **empty**, run the `None` call, assert slot `i` is still empty, and
  only **after** that run the `Some(i)` control and assert it populated slot `i` (or run the control
  on a different slot or a fresh VM). Running `Some` first leaves slot `i` full, and then a "None
  uses slot 0" mutant takes the IC-hit path, which writes nothing, and survives (G2.2).
  **Comparison:** `CallIC` derives `Clone, Debug` only (`vm/ic.rs:53`), so compare with
  `is_none()` per slot (or a `Debug`-string compare), not "byte-identical" (G2.3). Kills: `None`
  treated as slot 0. A sentinel such as `usize::MAX` is an **equivalent** mutant (`get` / `get_mut` →
  `None`) and is not claimed (G2.4).
- **T5 native callee** (`Math.max`, args `[1, 3, 2]`): `3` pushed and the stack shape `[…, 3]`.
  Kills: a `None` arm that skips the native branch. "No IC slot written" is **not** a kill for a
  native (it holds at the base under any mutant) and is either dropped or labelled a declared
  non-discriminating guard (G3).
- **T6 non-callable callee.** The required input is a non-callable **Object** (`{}`), which reaches
  the three-way body. A primitive returns `type_error` before any IC code and does not exercise the
  `None` arm. Assert `e.kind == VmErrorKind::TypeError` (`ic_call` returns a `VmError`, not a thrown
  value). Kills: a `None` arm that returns `Ok(undefined)`.
- **T7 `ic_call_method(.., None)`** over `[o, o.m, 4]` with `m(x) { return [this === o, x] }` →
  `[true, 4]`. Same slot precondition, order and comparison as T4, **plus its own `Some` control
  observing IC population** (G4; the 1ad row's acceptance requires one for each entry point). The
  native and non-callable branches as in T5 / T6. Kills: the receiver dropped on the `None` path.
