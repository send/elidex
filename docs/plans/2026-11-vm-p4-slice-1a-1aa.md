# VM P4 slice 1aa — the argument-layout contract (per-PR plan, rev 4.1)

**Parent.** Umbrella **1a** (*Call-spread VM infrastructure*), row `1a` of the §5 slice table in
`docs/plans/2026-07-vm-p4-slice-plan.md`. 1a's derivation is recorded in §6.6 of
`docs/plans/2026-07-vm-p4-slice-1a-1b-call-spread-detail.md`. It minted three terminal children in
rev 1, **1aa** (this memo), **1ab** (remove `op_array_spread`'s non-spec `return()`) and **1ac**
(root the four call-entry argument windows). The plan-review of rev 1 re-applied the terminality
measurement to 1aa and split the inline-cache entry point out as a fourth child, **1ad** (§2). The
PM's ruling on that review (PM 288) split `Function.prototype.apply`'s holey-array fix out as a fifth,
**1ae** (§9). This memo is 1aa's mandatory `/elidex-plan-review` input. It is outside the `2026-07-vm-p4-*` family glob,
so family memos cite it by path in backticks, never by a relative link.

**Base.** `origin/main` = `eaa741b5`. Every `file:line` below is at that revision; crate paths are
relative to `crates/script/elidex-js/src/` unless written in full. Line numbers are evidence — §6
lists the command that re-derives each one. Every probe in this memo ran in a throwaway
`git worktree add --detach … eaa741b5` with a temporary test module, removed afterwards, under
`rustc 1.99.1 (58cf8f9c3 2026-10-13)`.

**Kind.** A terminal row, terminal by necessity (PM 288; §2 records the measurement and the ruling). It lands before the
children of umbrella 1b that give its layout helper new callers, so the part with no JS-reachable
holder yet (the helper's error contract and its no-allocation window) is asserted from Rust.

**Rev 2 (plan-review R1).** D5 (the `Option<usize>` IC entry) moved to **1ad**; its design and its
tests, corrected for the dry-run's G2–G4, are handed over in §8. `Function.prototype.apply`'s
holey-array defect was fixed here (D6) in rev 2; it is 1ae's from rev 3. The two pre-existing `super` defects of rev 1's §3.1 have a
home on Slice **S** (§3.1). Internal-error disposition, the `vm/dispatch.rs` arm shape and edge 16
were decided by the driver (§1.1, §1.3, §7).

**Rev 3 (PM 288).** D6 and its tests A1/A2 moved to **1ae**, a new terminal child of umbrella 1a whose
`Deps` names 1aa; its design is handed over in §9. 1aa still makes `collect_array_like`'s dense branch
read through the shared primitive (D2), with its behaviour unchanged. 1aa is ruled terminal by
necessity (§2), which closes rev 2's §7 Q1; S's kind is re-applied at S's own derivation and
plan-review, which closes Q2. The own-accessor defect of §1.5 has a home,
`#11-vm-array-index-property-single-store` (registered as `#11-vm-array-index-own-accessor-invisible`;
renamed and widened at PM 296, rev 4).

**Rev 4 (plan-review R2).** Test spellings that did not compile or could be vacuous are fixed (T3,
§8 T6; T3's freed-slot order; the I-L mutant). "Uncatchable" is scoped to the `run()` that raises the
error (§1.1, §1.3). The two slot triggers 1aa fires are recorded, with a disposition, on their
ledger rows, which closes §7 Q3/Q4. The stack-depth bound is placed in `lay_out_call_args`, owned
by 1b's argument-emit child (§1.1). The release-build outcome of D-S2 is measured (§3.1). The spec
map is completed and re-cited (§3, §5.1).

**Rev 4.1 (R2 focused re-review; read at `d1e9186a`, this branch's merge of origin/main `145927c9`).**
D-S2 was discharged by #591 (`#11-vm-construct-isconstructor-check`); §3.1 keeps a discharge pointer
only, and its `vm/object_kind.rs` cite is re-cited by name to `vm/construct_admission.rs`. D-CL3 is
folded into 1ae (PM 310; §3.1, §9). §3's map gains the rows rev 4's sentence claimed, and its breadth
paragraph says what "changes" means. §2's I-G mutant now breaks I-G. Every other `file:line` stays
at `eaa741b5` (Base); the cites this revision adds name the revision they were read at.

**Overlap with a parked branch.** The local branch `vm-p4-rooting-carved` (`37a97d49`, merge-base
`6b33854d`) rewrites `vm/dispatch.rs` (386 lines changed) and `vm/dispatch_helpers.rs` (+535), and
carries a `raise` method on `VmInner` (`dispatch_helpers.rs:313` on that branch, absent from `main`), the remedy that
`#11-vm-internal-error-hard-exit` names. A revival of that branch has to account for the
`lay_out_call_args` producer this PR adds to `dispatch_helpers.rs` (§1.1). From rev 4 that
constraint is also written on `#11-vm-operand-rooting-by-construction`'s ledger row, the row that
cites the branch (`2026-07-vm-p4-slot-ledger.md`).

---

## §0. Scope

### §0.1 What this PR implements

The (a) mechanism of umbrella 1a's derivation without its IC half: the in-VM contract that turns an
args array into a call's argument window.

- **D1 — `lay_out_call_args` (NEW).** One stack-layout operation. The args array at the top of
  `vm.stack` is replaced in place by its elements, and the element count is returned. It is the
  single place an args array becomes an argument window (§1.1).
- **D2 — `Empty` normalisation, shared with `collect_array_like`'s dense fast path (dec. 8).** One
  primitive maps an `ObjectKind::Array`'s `elements` to argument values (`JsValue::Empty` →
  `Undefined`). D1 and `Function.prototype.apply`'s dense branch both read through it (§1.2). The
  dense branch's behaviour does not change here; the hole-free precondition in front of it is
  **1ae**'s (§9).
- **D3 — `super(...)` has one dispatch path.** `op_super_call_spread` becomes "lay out, then
  `op_super_call`", so the spread and flat `super` calls share argument hand-off and `[[Construct]]`
  dispatch. `op_super_call` takes `argc: usize`, so a spread window wider than 255 is not
  truncated (§1.3).
- **D4 — the falsified docstring is corrected.** `op_super_call_spread`'s claim that the compiler
  emits only `CreateArray; ArraySpread x; SuperCallSpread` is false. The synthesized default derived
  constructor (`compiler/expr_class.rs:145-152`) emits `GetLocal 0; SuperCallSpread` over a
  rest-packed array. That producer is spec-required (ECMA-262 §15.7.14 step 14.a.iv.1 NOTE) and is
  **not** folded (§1.4).
- **D6** — moved to **1ae** at rev 3 (PM 288); §9 holds its design and tests.

No opcode is added, no emit site changes, and no observable behaviour changes. `f(...xs)`, `o.m(...xs)` and `new F(...xs)` are still compiled by
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
- **1ae** — `Function.prototype.apply` takes `collect_array_like`'s dense branch only for a
  hole-free array (dec. 8's correction note), with its red test and dense control. Its `Deps` names
  1aa, because the precondition guards the call into 1aa's `array_argument_values`. Its own per-PR
  plan memo is owed; §9 hands over this memo's rev-2 design and tests. 1aa does not add the
  precondition. Since PM 310 it also owns D-CL3, the generic path's length clamp (§3.1, §9).
- **Umbrella 1b's children** — `compile_call_arguments` / `ArgsForm`, `emit_call`, `CallSpread` /
  `CallMethodSpread` / `NewSpread` handlers (the first new consumers of D1 and the first `None`
  producers of 1ad), arity-based form selection (`f(a1…a300)`), the stack-depth bound (dec. 12), the
  I-3 tagged-template input contract, and every edge of §6.4 the carve-out does not place on 1a's
  children. EvaluateNew step 5 at `do_new` is not theirs: its one owner statement is umbrella §3
  row 17 (`#11-vm-construct-isconstructor-check`, landed as #591).

---

## §1. Design

### §1.1 `lay_out_call_args` — the args array becomes the argument window

```rust
/// [C19] ECMA-262 §13.3.8.1 ArgumentListEvaluation, consumer side: the List an
/// args array carries becomes the call's argument window — [C13] §13.3.7.1
/// `SuperCall` step 4's argList, consumed at step 6, and §15.7.14
/// ClassDefinitionEvaluation step 14.a.i's `args`, consumed at step 14.a.iv.4.
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
- **Error disposition: uncatchable within the `run()` that raised it.** Every caller propagates
  that `Err` with `?`, never through `throw_error`. This is the disposition `op_super_call`'s own
  underflow already uses (`vm/dispatch_class.rs:55`, `return Err(VmError::internal("super call:
  stack underflow"))`, propagated by the `Op::SuperCall` arm's `?`). So no `try`/`catch` in the
  frame that raised it, or anywhere else in the same `run()`, can swallow it. The guarantee stops at
  the nearest boundary that turns an `Err` into a thrown value: a nested `run()` (the JS branch of
  `construct_synchronous`, `vm/dispatch_class.rs:319`, whose `raw_result?` at `:334` returns through
  `dispatch_super` → `throw_error`, `:109-112`), or a native that called back into JS (`Op::Call` →
  `vm_error_to_thrown` + `handle_exception`, `vm/dispatch.rs:705-710`). Across either boundary the
  error is catchable; that is §1.3 (iii), and `#11-vm-internal-error-hard-exit` closes it. §1.3
  states what this changes in `op_super_call_spread`.
- **The stack-depth bound lives here, and 1aa does not write it.** The helper returns `usize` and
  imposes no width in this PR: the Array path must accept `f(...arrOf1000)` (§6.3). Dec. 12 requires
  a bound and gives it to the child umbrella 1b mints for the argument-emit helper. That bound
  belongs in `lay_out_call_args`, not in the emit helper. `lay_out_call_args` is the one place an
  args array becomes a stack window, and `super(...)` and the rest-packed default derived constructor
  reach the stack only through it, never through the emit helper. The owner stays dec. 12's (1b's
  argument-emit child), and so does an edge-29-style acceptance on the super path:
  `super(...arrOf(bound + 1))` raises a RangeError, and `super(...arrOf(bound))` succeeds
  (carve-out §6.6, the 1b row). 1aa leaves the hook point: the helper already returns `Result`, so
  that child adds the check here without a signature change. **Read at `eaa741b5` (a code read,
  not a probe), `Function.prototype.apply` has no such push.** `native_function_apply` collects into a Rust
  `Vec` (`vm/natives_function.rs:76`) and calls `ctx.call_function` → `Vm::call` →
  `call_internal`, which grows `vm.stack` by the callee's `local_count` (`vm/interpreter.rs:599`)
  and copies `min(args.len(), param_count)` values into it (`:602-603`). The excess goes to the
  heap (the rest-parameter Array and `actual_args`), and a native callee receives the slice. So the
  bound has no second site there. The super path, by contrast, puts N values on `vm.stack` twice in
  sequence on the JS branch: the layout, then, after `op_super_call`'s drain, `construct_synchronous`'s
  re-push (`vm/dispatch_class.rs:304-307`).
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
/// Exact for the compiler-built arrays `lay_out_call_args` receives, whose values are
/// §13.3.7.1 `SuperCall` step 4's argList and §15.7.14 step 14.a.i's `args`.
/// NOT CreateListFromArrayLike-exact for a holey array (§7.3.19 step 6.b performs a real
/// `Get`), so a CreateListFromArrayLike caller reads through it only for a hole-free array
/// (dec. 8's caller-side precondition). Assumes the representation invariant "a non-`Empty`
/// element is a default data property", which does not hold for user-reachable Arrays until
/// `#11-vm-array-index-property-single-store` lands. ⚠ `collect_array_like`'s dense branch
/// does not test the precondition yet and reads a holey Array through this function (1ae, slot
/// `#11-vm-call-spread-arguments`, owns the precondition and its test).
pub(super) fn array_argument_values(elements: &[JsValue]) -> impl Iterator<Item = JsValue> + '_  // (NEW)
```

D1 extends the stack from it. `collect_array_like`'s dense branch collects from it, for every
`ObjectKind::Array` as today, so its behaviour is unchanged. The hole-free precondition that makes
that exact for a holey array is **1ae**'s (§9); the docstring states the defect until 1ae removes
the ⚠ line with the fix. The representation-invariant line stays until
`#11-vm-array-index-property-single-store` lands (§9).

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
- **Error routing, stated per error.** (i) The internal errors this path raises are uncatchable
  **within the `run()` that raises them**: `lay_out_call_args`'s `Err` (§1.1) and `op_super_call`'s
  underflow both propagate with `?` out of that `run()`. Today `op_super_call_spread`'s two
  defensive branches (`:78-93`) route the same compiler-invariant break through `throw_error`, where
  a user `try`/`catch` in the same frame can catch it. They are deleted with the clone, so **1aa adds
  no internal error catchable in the raising `run()`, and removes two.** No JS program can reach
  either deleted branch (the operand is compiler-built), so this changes no observable behaviour.
  When the raising `run()` is itself nested (a derived constructor run by `construct_synchronous`'s
  `vm.run()`, `:319`), its `Err` reaches the outer `run()` through `raw_result?` (`:334`) →
  `dispatch_super` → `throw_error` (`:109-112`), and there it is catchable. That is case (iii), and
  it applies to these errors too, not only to the ones listed there. (ii) User-facing errors
  from `dispatch_super_inner` (the `home_class` SyntaxError, the null-super TypeError,
  `construct_synchronous`'s "not a constructor") keep their route: `dispatch_super` → `throw_error`
  (`:104-113`), catchable, unchanged. (iii) **Not unified here:** `construct_synchronous`'s own
  internal error (`:321-323`, "ObjectKind::Function lost between probe and dispatch"), and any
  internal `Err` a nested `run()` returns (the errors of (i) included, once they cross that
  boundary), reach `throw_error` through `dispatch_super` and stay catchable. So does any internal
  `Err` that crosses a native boundary, where `Op::Call` hands it to `vm_error_to_thrown` +
  `handle_exception` (`vm/dispatch.rs:705-710`). Making them uncatchable means a kind test. The remedy `#11-vm-internal-error-hard-exit`
  (`2026-07-vm-p4-slot-ledger.md`) prescribes is "the kind test occurs at **exactly one** site,
  `throw_error` itself", and its cell rejects per-site kind tests ("adding the kind test at each of
  the … call sites individually satisfies them while leaving the next extracted helper to re-open
  the decision"). **1aa does not take that slot.** Its remedy changes the disposition of every
  `throw_error` caller crate-wide, and that is not this row's charter. The slot's trigger ("with
  `#11-vm-operand-rooting-by-construction`, or the next dispatch-loop PR") matches 1aa's wording, so
  1aa fires it: the slot becomes due. It does not transfer the deliverable. The firing and its
  disposition (it lands with `#11-vm-operand-rooting-by-construction`, i.e. before 1ac) are written on
  the slot's row in `2026-07-vm-p4-slot-ledger.md` (rev 4). The (iii) errors are inside that slot's
  single-site remedy and need no new evidence row.

### §1.4 The docstring correction (D4)

`op_super_call_spread`'s docstring is rewritten with its body. It states the contract over **both**
producers: the `super(...)` lowering (`compiler/expr_member.rs:113-129`), whose array is built by
the `ArraySpread` drain and so performs GetIterator, and the synthesized default derived constructor
(`compiler/expr_class.rs:145-152`), whose array is rest-packed at frame entry and so performs no
iteration (ECMA-262 §15.7.14 step 14.a.iv.1 NOTE). The module docstring of `vm/dispatch_class.rs`
(`:13-14`, "`Op::SuperCallSpread` consumes a pre-built args array") is corrected with it. D1 relies
on the invariant both producers share: the operand is a hole-free `ObjectKind::Array` that no user
code can reach between its construction and the opcode.

### §1.5 `apply` over a holey array — moved to 1ae (rev 3)

D6, its probe and the adjacent own-accessor defect it surfaced are in §9, the hand-off to **1ae**.
The own-accessor defect is homed on `#11-vm-array-index-property-single-store`
(`2026-07-vm-p4-slot-ledger.md`; registered as `#11-vm-array-index-own-accessor-invisible` and
widened to the whole root at PM 296); it was routed to the driver for an owner in rev 2.

### §1.6 Lens checks

- **Layering.** All of it is engine-bound VM dispatch (`vm/dispatch_*.rs`) and one VM native
  (`vm/natives_function.rs`). No algorithm moves into `vm/host/`, and no engine-independent crate
  has an argument-layout algorithm to reuse.
- **ECS-native.** No per-entity state is introduced; nothing here is a side-store.
- **One issue, one way.** After this PR there is one args-array→window operation (D1), one
  holes-as-arguments primitive (D2) with one stated precondition for CreateListFromArrayLike callers
  (which 1ae adds in front of its one such caller, §9), and one `super` hand-off (D3). The only remaining second mechanism is the default derived
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
| `vm/natives_function.rs` | `collect_array_like`'s dense branch reads through `array_argument_values` (D2); no precondition (1ae's, §9) |
| `vm/tests/tests_call_args_layout.rs` (NEW) | §4.1 |
| `vm/tests/tests_class_machinery.rs` | §4.2 |
| `vm/tests/mod.rs` | registers the new modules |

### §1.8 Commit sequence (pins first)

There is no honest red-first set for D1–D4 at the base. The Rust tests (T1–T3) name symbols that do
not exist yet, and the JS tests (C1–C7) pin behaviour that is already correct (probed, §4.2). (D6's
red-first test moved to 1ae with it, §9.) So:

1. C1–C6 (§4.2), green at the base (probed) — committed first, so each later commit is measured
   against them.
2. D1 + D2 with T1–T3.
3. D3 + D4. The `usize` widening lands in the same commit as the conversion, so C6 is never red.

The mutation proof each test names (§4) is run once, on the final head. Each mutant is a compiling
edit, named in §4. A revert that does not compile is not a proof.

---

## §2. Coupled invariants, and terminality re-applied

**The measurement.** §5's head: "the only evidence that an axis is an axis is that it **fails while
the others pass**", and the criterion's first disjunct asks for ≥3 such axes that intersect. This is
the measurement that split 1a. Applied literally at this slice's HEAD, over rev 1's scope (D1–D5)
plus D6 (the table is the rev-2 measurement; A1 and A2 are 1ae's tests from rev 3, §9):

| Invariant | Mutant that breaks it alone (compiles) | Red | Green under that mutant |
|---|---|---|---|
| **I-L (layout)** — the stack holds exactly the array's elements, in index order, where the array was; the count equals the number pushed | push `elements` reversed, spelled `let v: Vec<_> = array_argument_values(elements).collect(); stack.extend(v.into_iter().rev())` (`array_argument_values` returns `impl Iterator`, not `DoubleEndedIterator`, so `.rev()` on it does not compile) | T1, C3, C5 (`"[0,2,1]"`), C6 (`arguments[299] === 0`) | T2, T3, C1, C2, A1, A2 |
| **I-H (holes)** — no `JsValue::Empty` reaches an argument window | drop `.or_undefined()` from `array_argument_values` | T1 | T2, T3, C1–C6 (no compiler-built array has a hole), A1, A2 |
| **I-G (GC)** — no `ObjectId` the args array or its elements denote is held only in a Rust local across an allocation **in D1's window** | after the error-contract validation, give up the disjoint borrow and spell the body as `let Some(JsValue::Object(id)) = self.stack.pop() else { unreachable!() }; let ObjectKind::Array { elements } = &self.get_object(id).kind else { unreachable!() }; let v: Vec<JsValue> = array_argument_values(elements).collect(); let _ = self.create_array_object(Vec::new()); self.stack.extend(v);` (the shared borrow of `elements` ends at the `collect`, so the `&mut self` call compiles): the array is popped, so its id is held only in the local `id`, and its element values only in the Rust `Vec` `v`, across `create_array_object` (`vm/inner.rs:173`, an allocation while GC is enabled). ⚠ *Rev 4's mutant, "allocate a temporary JS array from the helper", left the array on the stack, where it stays rooted, so it did not break I-G; and with the disjoint borrow in place it did not compile.* Placed after validation for the reason rev 4 gave: placed before it, the allocation takes T3's freed slot (`alloc_object` reuses freed slots first, `vm/inner.rs:121-123`) and T3's freed-slot case turns into an `Ok(0)` on a live object | T2 — by its stronger property, not by observing a freed object: T2 fails on **any** allocation in the window (its armed one-shot is consumed), and this mutant allocates; no test observes the dangling `id` itself | T1, T3, C1–C6 |
| **I-O (observability)** — laying out performs no property lookup and no iteration (§15.7.14 step 14.a.iv.1 NOTE; edge 32) | lay out through `%Array.prototype%[%Symbol.iterator%]`, leaving the array on the stack | C1 | T1, T3, C2–C6; T2 too, because the iterator objects are allocated inside natives under `gc_enabled = false` (`vm/interpreter.rs:364`; dry-run G1) |
| **I-W (width)** — no argument count passes through a `u8` on the array path | `self.op_super_call(argc as u8 as usize, …)` in `op_super_call_spread` | C6 | everything else |
| **I-A (array-like)** — `apply` over a holey Array performs §7.3.19 step 6.b's `Get` (**1ae** from rev 3) | drop D6's hole-free precondition | A1 | everything else |
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

**Ruled (PM 288): the remainder is terminal by necessity.** Without D5 the remainder still carried
≥3 axes that fail alone and intersect at `lay_out_call_args`, so read literally the measurement
still said edge-dense, and rev 2 left that open (its §7 Q1). The ruling rests on three facts and two
in-tree rules:
- The five invariants (I-L order, I-H holes, I-G no allocation in the window, I-O no iteration, I-W
  width) are properties of one ~20-line loop, `lay_out_call_args`, and of its return type. They are
  not separable mechanisms.
- Cutting that loop defeats the deliverable: every partition lands it missing one of its properties,
  or lands it dead.
- `super(...)` already runs the loop on a live path (D3), so a partial loop would ship as a
  live, partial contract.
- The family's own rule, in `#11-vm-operand-rooting-by-construction`: "a row that cannot be split
  without defeating its deliverable is terminal by necessity".
- CLAUDE.md's *Edge-dense work* base case (c): a narrowly scoped per-PR slice under an approved
  umbrella that has passed plan-review is a terminal unit, "per-PR slice が同 subsystem を触ること自体は
  再分割 trigger にしない". 1aa is a plan-reviewed slice under the approved #506 umbrella.

So 1aa is terminal. The §5 row and carve-out §6.6 say the same.

**I-A (D6) is cut out as 1ae (PM 288).** It fails alone and touches only `collect_array_like`, so it
was the one part separable without cutting `lay_out_call_args`. It meets I-H at the shared primitive,
and that meeting is now the sibling edge 1ae → 1aa (§9), not an intersection inside this PR.

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
  D6's precondition is what makes sharing it with `collect_array_like` exact. From rev 3 this pair
  crosses a PR boundary: 1aa ships the primitive and its docstring's ⚠ line, and 1ae ships the
  precondition (the 1ae → 1aa edge, §9).

**What T2 observes.** I-G is defined "across an allocation", and T2 observes allocation events
only: its armed one-shot is consumed by the next `alloc_object`. A collection with no allocation in
the window, for example a direct `collect_garbage()` call from the helper, is not observed by T2.
No such call exists or is planned, and the disjoint borrow (§1.1) makes it unrepresentable in the
window anyway (`collect_garbage` takes `&mut self`), so the claim is scoped to what T2 can see and no
coverage is claimed beyond it.

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
is not changed here. It belongs to `#11-vm-operand-rooting-by-construction`, and from rev 4 the
question is written on that slot's ledger row (§7 Q3, closed).

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
D5 to 1ad (§8). Rev 4 re-read the steps with `webref body ecma262 GetSuperConstructor` (steps 1-6),
`body ecma262 ArgumentListEvaluation` (all eight productions), `body ecma262 Construct` (steps 1-3),
`body ecma262 ClassDefinitionEvaluation` (step 14.a.i-vii), `body ecma262 sec-function.prototype.apply`
(steps 1-6) and `body ecma262 CreateListFromArrayLike` (steps 1-7), and completed the rows those
bodies have and the map lacked (plan-review R2). ⚠ *That overclaimed: rev 4.1 added the rows it still
lacked — `apply` steps 1 and 3.a / 5, CreateListFromArrayLike steps 1 and 4 / 5 / 6.a / 6.d / 6.e / 7,
and the abrupt `precedingArgs` (step 1) of `ArgumentList , ... AssignmentExpression`.*

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| ECMA-262 §13.3.8.1 Runtime Semantics: ArgumentListEvaluation | `Arguments : ( )` step 1 | empty List | flat `super()` → `Op::SuperCall 0` — unchanged producer; consumer `op_super_call(0)` | ✓ | no |
| ECMA-262 §13.3.8.1 Runtime Semantics: ArgumentListEvaluation | `ArgumentList : AssignmentExpression` steps 1-3; `ArgumentList : ArgumentList , AssignmentExpression` steps 1-4 | non-spread elements, normal completion, in order | flat `super(a, b)`: compiled in order, `Op::SuperCall argc`; mixed `super(a, ...xs, b)`: `ArrayPush` (`compiler/expr_member.rs:116-127`) — producer unchanged; consumer `lay_out_call_args` preserves index order (I-L, C3) | ✓ | yes |
| ECMA-262 §13.3.8.1 Runtime Semantics: ArgumentListEvaluation | `ArgumentList : AssignmentExpression` steps 1-2; `ArgumentList : ArgumentList , AssignmentExpression` steps 1-3 | abrupt `? Evaluation` / `? GetValue` (and an abrupt `precedingArgs`, step 1) → the completion propagates | producer: the compiled argument expressions (unchanged); the throw unwinds before the opcode runs, so 1aa's consumer is never reached | ✓ (probed, §3.2) | yes |
| ECMA-262 §13.3.8.1 Runtime Semantics: ArgumentListEvaluation | `ArgumentList : ... AssignmentExpression` steps 1-5; `ArgumentList : ArgumentList , ... AssignmentExpression` steps 1-4 | normal completion: the List is the drain order, `precedingArgs` first | producer `ArraySpread` → `op_array_spread` (unchanged); consumer `lay_out_call_args` preserves index order (I-L) | ✓ | yes |
| ECMA-262 §13.3.8.1 Runtime Semantics: ArgumentListEvaluation | `ArgumentList : ... AssignmentExpression` steps 2-3; `ArgumentList : ArgumentList , ... AssignmentExpression` steps 2-3 | abrupt `? Evaluation` / `? GetValue` of the spread operand → the completion propagates | producer (unchanged); unwinds before `ArraySpread`, not on 1aa's consumer side | ✓ (probed, §3.2) | yes |
| ECMA-262 §13.3.8.1 Runtime Semantics: ArgumentListEvaluation | `ArgumentList : ArgumentList , ... AssignmentExpression` step 1 | abrupt `precedingArgs` → the completion propagates | producer: the preceding arguments, compiled before the spread operand (`compiler/expr_member.rs:116-127`, unchanged); the throw unwinds before `ArraySpread` and the opcode, so 1aa's consumer is never reached | ✓ (code read, not probed: the probed `super(1, ...(throw))` covers step 2, not step 1) | yes |
| ECMA-262 §13.3.8.1 Runtime Semantics: ArgumentListEvaluation | `... AssignmentExpression` step 4 / `ArgumentList , ... AssignmentExpression` step 3 | `? GetIterator(…, sync)` throws: non-iterable spread operand → TypeError | producer `op_array_spread` → `resolve_iterator` (unchanged; not on 1aa's consumer side) | ✓ (probed, §3.2) | yes |
| ECMA-262 §13.3.8.1 Runtime Semantics: ArgumentListEvaluation | `... AssignmentExpression` step 5.a / `ArgumentList , ... AssignmentExpression` step 4.a | `? IteratorStepValue` throws → the error propagates | producer `op_array_spread`'s drain (unchanged): propagation ✓ (probed, §3.2); the extra `return()` it calls on that path is ✗ and is **1ab**'s (edge 21) | ✗ (`return()` half; 1ab) | yes |
| ECMA-262 §13.3.8.1 Runtime Semantics: ArgumentListEvaluation | `... AssignmentExpression` step 5.b | the iterator is done at once: the List is empty | `super(...[])` → `lay_out_call_args` returns `Ok(0)` → window of 0 (C2) | ✓ | yes |
| ECMA-262 §13.3.8.1 Runtime Semantics: ArgumentListEvaluation | `TemplateLiteral : NoSubstitutionTemplate` steps 1-3; `TemplateLiteral : SubstitutionTemplate` steps 1-4; `SubstitutionTemplate : TemplateHead Expression TemplateSpans` steps 1-5 | tagged-template argument lists | — not touched | n/a (these productions are the argument list of a tagged template, never of `super`'s `Arguments`; the tagged-call lowering is umbrella 4a's child's, and the I-3 input contract is 1b's) | yes |
| ECMA-262 §13.3.7.1 Runtime Semantics: Evaluation | `SuperCall` steps 1-2 | GetNewTarget; Assert constructor | `dispatch_super_inner` reads the frame's `CallMode::Construct { new_target }` (`vm/dispatch_class.rs:132-135`) — unchanged | ✓ | no |
| ECMA-262 §13.3.7.1 Runtime Semantics: Evaluation | `SuperCall` step 3 → step 4 order | superCtor is fixed **before** the arguments are evaluated | — `dispatch_super_inner` resolves `home.prototype` (`:121-124`) when the opcode runs, after the arguments | ✗ (pre-existing; probed; homed on Slice S, §3.1) | yes |
| ECMA-262 §13.3.7.1 Runtime Semantics: Evaluation | `SuperCall` step 4 | `argList` from ArgumentListEvaluation | flat: `Op::SuperCall` (`u8` operand) → `op_super_call(usize)`; spread: `op_super_call_spread` → `lay_out_call_args` (NEW) → `op_super_call` | ✓ | yes |
| ECMA-262 §13.3.7.1 Runtime Semantics: Evaluation | `SuperCall` step 5 | IsConstructor(superCtor) false → TypeError | `construct_synchronous` → `admit_construct` (`vm/construct_admission.rs`, from #591) — not touched by 1aa; at the base it accepted every `ObjectKind::Function` | ✓ from #591 (✗ at `eaa741b5`: D-S2, discharged by `#11-vm-construct-isconstructor-check`, §3.1) | yes |
| ECMA-262 §13.3.7.1 Runtime Semantics: Evaluation | `SuperCall` step 6 | `? Construct(superCtor, argList, newTarget)` | `construct_synchronous(super_class, receiver, args, CallMode::Construct { new_target }, pre_alloc)` (`:139-145`) — unchanged | ✓ | yes |
| ECMA-262 §13.3.7.1 Runtime Semantics: Evaluation | `SuperCall` steps 7-9 | GetThisEnvironment; `? BindThisValue(thisER, result)` | `dispatch_super_inner` *assigns* `this_value` / `new_instance` (`:150-154`), with no `[[ThisBindingStatus]]` | ✗ (pre-existing; owner `#11-vm-derived-constructor-this-binding`, and Slice S for the arrow spelling) | yes |
| ECMA-262 §13.3.7.1 Runtime Semantics: Evaluation | `SuperCall` steps 10-12 | `? InitializeInstanceElements(result, funcObj)` | — not touched | n/a (instance-element initialisation is umbrella 2a's — row 2ab) | yes |
| ECMA-262 §13.3.7.1 Runtime Semantics: Evaluation | `SuperCall` step 13 | return result | `self.stack.push(result)` (`:156`) — unchanged | ✓ | no |
| ECMA-262 §13.3.7.2 GetSuperConstructor ( ) | steps 1-6 | `GetThisEnvironment()` (1) → Assert Function Environment Record (2) → `[[FunctionObject]]` (3) → Assert ECMAScript function object (4) → `! activeFunc.[[GetPrototypeOf]]()` (5) → return superCtor (6) | `home_class` → `.prototype` (`vm/dispatch_class.rs:116-125`) — unchanged; correct for a constructor frame. Through an arrow it is ✗ and is Slice S's | ✓ (constructor frame) | yes |
| ECMA-262 §7.3.14 Construct ( ctor [ , argList [ , newTarget ] ] ) | step 1 | `newTarget` defaults to `ctor` | — every `super` caller passes `newTarget` | n/a (never absent on this path) | no |
| ECMA-262 §7.3.14 Construct ( ctor [ , argList [ , newTarget ] ] ) | step 2 | `argList` defaults to a new empty List | — every `super` caller passes `argList` (possibly empty) | n/a (never absent on this path) | no |
| ECMA-262 §7.3.14 Construct ( ctor [ , argList [ , newTarget ] ] ) | step 3 | `? ctor.[[Construct]](argList, newTarget)`; `ctor` is a constructor by the parameter's type, so Construct has no IsConstructor step of its own — the test is its callers' (`SuperCall` step 5, 14.a.iv.3) | `construct_synchronous`, reached from SuperCall step 6 / ClassDefinitionEvaluation 14.a.iv.4 with `newTarget` explicit — unchanged | ✓ (the callers' test is the step-5 / 14.a.iv.3 rows, applied by `admit_construct` from #591) | yes |
| ECMA-262 §15.7.14 Runtime Semantics: ClassDefinitionEvaluation | step 14.a.i | `args` = the List passed by [[Call]] / [[Construct]] | rest-param packing into local 0 (`compiler/expr_class.rs:145-152`, unchanged) → `lay_out_call_args` | ✓ | yes |
| ECMA-262 §15.7.14 Runtime Semantics: ClassDefinitionEvaluation | step 14.a.ii | NewTarget undefined → TypeError | the class-constructor call-mode guard on the call path — not touched | ✓ (probed, §3.2) | yes |
| ECMA-262 §15.7.14 Runtime Semantics: ClassDefinitionEvaluation | step 14.a.iii | `ctorFunc` = the active function object | — no separate read: the default constructor's frame `home_class` stands for it in 14.a.iv.2, unchanged | n/a (no site of its own) | no |
| ECMA-262 §15.7.14 Runtime Semantics: ClassDefinitionEvaluation | step 14.a.iv (the test) | `ctorFunc.[[ConstructorKind]]` is derived | decided at compile time: the derived default constructor is synthesized only when `ClassHeritage` is present (`compiler/expr_class.rs:129`), which is when step 18 sets `[[ConstructorKind]]` to derived — unchanged | n/a (no runtime test; compile-time selection, unchanged) | no |
| ECMA-262 §15.7.14 Runtime Semantics: ClassDefinitionEvaluation | step 14.a.iv.1 NOTE | must **not** call `%Array.prototype%[%Symbol.iterator%]` | `lay_out_call_args` reads `elements` directly (I-O); edge 32 (C1) | ✓ | yes |
| ECMA-262 §15.7.14 Runtime Semantics: ClassDefinitionEvaluation | step 14.a.iv.2 | func = `! ctorFunc.[[GetPrototypeOf]]()` | `home_class` of the default ctor frame = the class → `.prototype` — unchanged | ✓ | yes |
| ECMA-262 §15.7.14 Runtime Semantics: ClassDefinitionEvaluation | step 14.a.iv.3 | IsConstructor(func) false → TypeError | the same `construct_synchronous` → `admit_construct` admission as SuperCall step 5 — not touched by 1aa | ✓ from #591 (✗ at `eaa741b5`: D-S2, discharged, §3.1) | yes |
| ECMA-262 §15.7.14 Runtime Semantics: ClassDefinitionEvaluation | step 14.a.iv.4 | `? Construct(func, args, NewTarget)` | `construct_synchronous` — unchanged | ✓ | yes |
| ECMA-262 §15.7.14 Runtime Semantics: ClassDefinitionEvaluation | step 14.a.v | base branch: `? OrdinaryCreateFromConstructor(NewTarget, "%Object.prototype%")` | — a base default constructor emits no super call and never reaches `lay_out_call_args` | n/a (not on this path) | no |
| ECMA-262 §15.7.14 Runtime Semantics: ClassDefinitionEvaluation | step 14.a.vi | `? InitializeInstanceElements(result, ctorFunc)` | — not touched | n/a (umbrella 2a's — row 2ab) | yes |
| ECMA-262 §15.7.14 Runtime Semantics: ClassDefinitionEvaluation | step 14.a.vii | return `NormalCompletion(result)` | the synthesized body's `Pop; ReturnUndefined`, after which `construct_synchronous` substitutes the constructed instance (`compiler/expr_class.rs:145-152`) — unchanged | n/a (not a site this PR touches) | no |
| ECMA-262 §20.2.3.1 Function.prototype.apply ( thisArg, argArray ) | step 1 | `func` = the this value | the native's `this` argument, read by `require_callable_this` (`vm/natives_function.rs:63`) — unchanged | ✓ | yes |
| ECMA-262 §20.2.3.1 Function.prototype.apply ( thisArg, argArray ) | step 2 | IsCallable(func) false → TypeError | `require_callable_this` (`vm/natives_function.rs:63`) — unchanged | ✓ | yes |
| ECMA-262 §20.2.3.1 Function.prototype.apply ( thisArg, argArray ) | step 3 | `argArray` undefined or null → `? Call(func, thisArg)` | `ctx.call_function(callee_id, this_arg, &[])` (`:67-69`) — unchanged | ✓ | yes |
| ECMA-262 §20.2.3.1 Function.prototype.apply ( thisArg, argArray ) | steps 3.a, 5 | `PrepareForTailCall()` (§15.10.3) | — | n/a (no tail calls in elidex, as the umbrella's EvaluateCall step 6 row records) | no |
| ECMA-262 §20.2.3.1 Function.prototype.apply ( thisArg, argArray ) | step 4 | `? CreateListFromArrayLike(argArray)` | `collect_array_like` reads its dense branch through `array_argument_values` (D2), behaviour unchanged | ✗ for a holey Array (pre-existing; **1ae**, §9) | yes |
| ECMA-262 §20.2.3.1 Function.prototype.apply ( thisArg, argArray ) | step 6 | `? Call(func, thisArg, argList)` | `ctx.call_function(callee_id, this_arg, &call_args)` (`:77`) — unchanged | ✓ | yes |
| ECMA-262 §7.3.19 CreateListFromArrayLike ( obj [ , validElementTypes ] ) | step 1 | `validElementTypes` absent → any | — `apply` passes none | n/a (always any on this path; see step 6.c) | no |
| ECMA-262 §7.3.19 CreateListFromArrayLike ( obj [ , validElementTypes ] ) | step 2 | `obj` not an Object → TypeError | done by the caller, `native_function_apply` (`:71-75`), before `collect_array_like` is called — unchanged | ✓ | yes |
| ECMA-262 §7.3.19 CreateListFromArrayLike ( obj [ , validElementTypes ] ) | step 3 | `length` = `? LengthOfArrayLike(obj)` | generic path: `get_property_value(length)` → `to_number` → `trunc`, NaN or negative → 0 (as ToLength), then `.min(DENSE_ARRAY_LEN_LIMIT)` (`1 << 27`, `vm/ops.rs:24`; `natives_function.rs:288`) — unchanged | ✗ (pre-existing: a length above 2^27 is silently truncated, where ToLength admits up to 2^53 − 1; observable only past 2^27 indices; D-CL3, owner **1ae** since PM 310, §3.1, §9) | yes |
| ECMA-262 §7.3.19 CreateListFromArrayLike ( obj [ , validElementTypes ] ) | step 6.b | `next` = `? Get(obj, indexName)` — consults the prototype chain for a hole | unchanged here: the dense branch takes every Array, holey ones included; the per-index `get_property_value` loop is taken only for a non-Array | ✗ (pre-existing; red at the base, A1; **1ae**, §9) | yes |
| ECMA-262 §7.3.19 CreateListFromArrayLike ( obj [ , validElementTypes ] ) | steps 4, 5, 6.a, 6.d, 6.e, 7 | the List is built index by index from 0, `indexName` = `! ToString(𝔽(index))`, each `next` appended, and returned | generic path: `Vec::with_capacity`, `ctx.intern(&i.to_string())`, `result.push(val)` over `0..len` (`:289-296`) — unchanged; the dense branch builds its List through `array_argument_values` (D2), behaviour unchanged | ✓ (generic path); the dense branch is equal to steps 3-7 for a hole-free Array only (**1ae**, §9) | yes |
| ECMA-262 §7.3.19 CreateListFromArrayLike ( obj [ , validElementTypes ] ) | step 6.c | `validElementTypes` is property-key and `next` is not a property key → TypeError | — `apply` passes no `validElementTypes` (step 1 makes it any) | n/a (never property-key on this path) | no |

**Breadth**: K=1 (ECMA-262), M=45. The preflight prints SPLIT-DEFAULT (K ≥ 6 or M ≥ 30), which asks
a single-PR memo to justify itself in this narrative; rev 1 had M=16 ("ok"), rev 2 and rev 3 had
M=25 (SPLIT-RECOMMENDED), and rev 4 had M=40. The justification is the paragraph's last two sentences: of the 45 rows,
35 name sites this PR does not change (verified 2026-11-05, by reading the touch column of every row), and the split criterion is §2's measurement, under which 1aa
is terminal by necessity (PM 288). The rows added in rev 2 are the R1 completeness rows (abrupt branches, non-spread
productions, SuperCall steps 1-2 and 7-13, ClassDefinitionEvaluation 14.a.ii / 14.a.vi, Construct
step 1). The rows added in rev 4 are the R2 completeness rows: the abrupt `? Evaluation` / `? GetValue`
branches of both production kinds, the spread production's step 5.b (C2's row; rev 3 put C2 on
`Arguments : ( )`, which is the flat `super()`), the TemplateLiteral productions (n/a),
GetSuperConstructor steps 5-6, Construct step 2, ClassDefinitionEvaluation 14.a.iii / 14.a.iv's test
/ 14.a.v / 14.a.vii (n/a), `apply` steps 2 / 3 / 6, and CreateListFromArrayLike steps 2 / 3 / 6.c. The rows added in rev 4.1 are the ones rev 4's sentence above claimed (n/a, ✓ and one code-read ✓).
Most of them are sites this PR does not touch ("unchanged", n/a, or a pre-existing ✗ with an
owner). The rows whose touch column names a site this PR changes are 10: `Arguments : ( )` (rev 4
counted 8 and left it out, though its consumer `op_super_call(0)` is the function 1aa re-signs `u8` →
`usize`), the three other ArgumentListEvaluation consumer rows (non-spread and spread normal
completion — the flat half of the non-spread row reaches the same re-signed `op_super_call` — and step
5.b), `SuperCall` step 4, 14.a.i, 14.a.iv.1 NOTE, §20.2.3.1 step 4, §7.3.19 step 6.b and §7.3.19's
list-building row (the dense branch's read-through). **"Changes" means the site's code is edited — a
signature or a body — not that its behaviour changes**: none of the 10 changes observable behaviour
(§0.1). (From rev 3 the last three are touched by D2's read-through only; their ✗ is 1ae's.) Breadth is not the split criterion. §2's measurement is, and §2 records its verdict and the
ruling.

### §3.1 User-input touch audit, and the pre-existing defects

Sites this PR touches that user-controlled values flow through:

- `op_super_call_spread` / `lay_out_call_args`: the element values are user values (spread operand
  or the arguments of `new B(…)`), and their count is user-controlled (`super(...arrOf1e6)`). The
  array itself is compiler-built and unreachable from user code. Exposure change: **none**. The same
  values reach `dispatch_super` in the same order, and the count was already unbounded (the current
  `elements.clone()` + `construct_synchronous`'s re-push).
- `collect_array_like`: a user array via `Function.prototype.apply`. Exposure change: **none** in
  1aa (the dense branch reads through the shared primitive with the same normalisation). The change
  that makes a holey Array run user getters on the prototype chain is 1ae's (§9).

Pre-existing defects found while auditing, probed at the base. None is created or widened here.
D-S1's home is judged on the criterion "which existing row's change already rewrites the superCtor
resolution site" (rev 1-4 judged D-S2 onto the same home; #591 has since discharged it):

- **D-S1 — `super(...)` resolves the super constructor after its arguments.** §13.3.7.1 `SuperCall`
  fixes `superCtor` at step 3, before ArgumentListEvaluation at step 4. `dispatch_super_inner` reads
  `home.prototype` (`vm/dispatch_class.rs:121-124`) when the opcode runs. Probe:
  `class A { constructor(){ this.k = 'A' } } class C { constructor(){ this.k = 'C' } } class B extends A { constructor() { super(Object.setPrototypeOf(B, C)) } } new B().k`
  → `"C"` at the base (spec: `"A"`). The spread spelling `super(...[Object.setPrototypeOf(B, C)])`
  also gives `"C"`. Both `super` opcodes share it. A fix resolves before the arguments, which is an
  emit change and outside 1a's charter.
- **D-S2 — the super constructor's IsConstructor test — discharged by #591.** SuperCall step 5 and
  ClassDefinitionEvaluation 14.a.iv.3 were not applied to a JS super constructor at the base
  (`construct_synchronous` admitted every `ObjectKind::Function`). `#11-vm-construct-isconstructor-check`
  landed the fix as #591 (squash `145927c9`): `do_new` and `construct_synchronous` both call
  `admit_construct` (`vm/construct_admission.rs`, beside the predicate `is_constructor`, which #588
  `4fd85f4e` moved there from `vm/object_kind.rs`) before any frame push. Its measured matrix,
  including the release-build frame over-pop rev 4 measured here, is that slot's
  (`docs/plans/2026-11-vm-isconstructor-check.md`); rev 4's probes stay in this memo's history at
  `e493bd57`. 1aa neither owns nor changes it, and what Slice S keeps from it is acceptance, stated
  once on S's §5 row.

**The home: Slice S (`#11-vm-arrow-lexical-super`), judged (ii).** S's deliverable replaces the
superCtor resolution site: "that link carried on the closure rather than read off the current frame".
That frame read is `dispatch_super_inner`'s `self.frames[frame_idx].home_class` (`:116-119`), the
first line of GetSuperConstructor. The other candidates are weaker on that criterion.
(i) `#11-vm-derived-constructor-this-binding` rewrites the receiver read and the substitution
(`:136`, `:150-154`, SuperCall steps 7-9), not the resolution. (iii) A new row would be a second home
for the resolution site, which S rewrites anyway. Rev 1-4 sent D-S2 (SuperCall step 5's test) to the
same home, on the reasoning that the test reads the value step 3 resolves; #591 then discharged it on
its own slot (D-S2 above). D-S1, its repro, owner and trigger are written in S's ledger row
(`2026-07-vm-p4-slot-ledger.md`) and pointed to from S's §5 row, and both rows carry D-S2's
discharge pointer. That row's Re-eval stays *at
registration: landing + 30 d* (§7 Q5).

**⚠ What that home does to S's own kind (§7 Q2, closed by PM 288).** S's §5 row reads "one axis". By §2's
measurement, the lexical link, the step-3 ordering and the step-5 test each fail while the other two
pass, and all three meet at the superCtor resolution site: three axes that intersect. The S row now
says that its terminality has to be re-applied before it is scheduled, and **does not decide it**:
PM 288 places the re-application at S's own derivation and plan-review. ⚠ *(rev 4.1) The step-5
test is no longer S's (D-S2 discharged by #591), but S's moved read must keep that admission's T-ORD
null half green, so the count S's plan-review re-takes may be two or three; S's row says so, and this
memo does not take it.*

Measured at HEAD `41a45a43` before this revision:
`/usr/bin/grep -c 'GetSuperConstructor' docs/plans/2026-07-vm-p4-*.md` is 1 in each of four
files, so **4** lines: three about `super` through an arrow, and one that is carve-out §6.6's seed
list. Rev 1 said 3, which was measured before §6.6 was written. None of the four owned D-S1 or D-S2.

**D-CL3 — CreateListFromArrayLike step 3 is truncated at `DENSE_ARRAY_LEN_LIMIT`.**
`collect_array_like`'s generic path clamps `length` with `.min(DENSE_ARRAY_LEN_LIMIT)` (`1 << 27`,
`vm/ops.rs:24`; `vm/natives_function.rs:288`), where §7.3.19 step 3's LengthOfArrayLike → ToLength
admits up to 2^53 − 1. An array-like whose `length` is above 2^27 is read as if its length were
2^27, with no error. This is pre-existing, not created or widened here (1aa does not touch the
generic path), and observable only for an array-like with more than 2^27 indices. ⚠ *This read "No
row of the family owns it, so it is reported here, with no owner chosen in this memo".* **Owner: 1ae**
(PM 310 folded it there: the same function, `collect_array_like`, and the same AO). The spec-correct
outcome and the acceptance are stated once, on 1ae's §5 row; §9 hands the design input over.

### §3.2 Probes behind the cells marked "probed, §3.2"

These rows concern sites 1aa does not touch. Each was probed at the base, so the mark is a measurement:

- ArgumentListEvaluation `? GetIterator` on a non-iterable operand:
  `class A {} class B extends A { constructor(){ super(...1) } } try { new B(); 'no-throw' } catch (e) { 'threw ' + (e instanceof TypeError) }`
  → `"threw true"`.
- `? IteratorStepValue` propagation:
  `const it = { [Symbol.iterator](){ return { next(){ throw new RangeError('boom') } } } }; class A {} class B extends A { constructor(){ super(...it) } }`,
  then `new B()` inside `try` → `"threw true boom"` (`e instanceof RangeError`, message kept).
- The abrupt `? Evaluation` / `? GetValue` branches (rev 4), each with
  `class A { constructor(...a){ this.n = a.length } }`, a derived `B` whose constructor is the
  `super(…)` shown, and `try { new B(); 'no-throw' } catch (e) { 'threw ' + (e instanceof X) … }`:
  `super(1, (()=>{ throw new RangeError('e1') })())` → `"threw true e1"`; `super(1, undeclaredQ)` →
  `"threw true"` (ReferenceError); `super(...(()=>{ throw new RangeError('e2') })())` →
  `"threw true e2"`; `super(...undeclaredR)` → `"threw true"`; `super(1, ...(()=>{ throw new
  RangeError('e3') })())` → `"threw true e3"`; `super(1, ...undeclaredS)` → `"threw true"`; an abrupt
  first argument, `super((()=>{ throw new RangeError('e4') })(), 2)` → `"threw true e4"`. Control:
  `super(1, ...[2, 3])` → `n` is `3`.
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
  `Empty != Undefined` is observable). Kills: reversed order (I-L, the mutant spelled in §2's I-L
  row: collect `array_argument_values(elements)` into a `Vec`, then extend the stack from
  `v.into_iter().rev()`), dropped normalisation (I-H), an off-by-one count, the array left on the
  stack.
- **T2 edge 16 — no VM-side allocation in the window while GC is enabled.** Precondition, asserted
  first: `gc_enabled == true` (the tripwire fires only under it, `vm/inner.rs:105`). Same setup as
  T1. Arm `force_gc_before_next_alloc` (`vm/mod.rs:274`) after setup, call `lay_out_call_args()`, and
  assert the one-shot is still armed. Paired control in the same test: the same arming followed by
  one `alloc_object` consumes it, so the probe can observe an allocation. Kills: an implementation
  that makes a **VM-side allocation while GC is enabled** between reading `elements` and the last
  push, e.g. `self.create_array_object(…)` from the helper. It does **not** kill a layout through the
  iterator protocol, whose ArrayIterator and `{value, done}` objects are allocated inside natives
  under `gc_enabled = false` (dry-run G1). C1 kills that mutant.
- **T3 error contract.** Four inputs, each `Err` with `matches!(e.kind, VmErrorKind::InternalError)`
  (`VmErrorKind` derives only `Debug`, `vm/error.rs:19-20`, so `==` does not compile; `matches!` is
  the crate's idiom, `vm/host/dom_bridge.rs:567`) and the stack unchanged (compared before and
  after): an empty stack; a primitive on top; an ordinary non-Array object on top; an `Object(id)`
  whose slot is freed (`objects[id]` = `None`, set up by allocating, dropping every root, and
  collecting). **The freed-slot case is built last**, after the other three inputs are set up, and
  nothing is allocated between `collect_garbage()` and the call: `alloc_object` reuses freed slots
  first (`free_objects.pop()`, `vm/inner.rs:121-123`), so an allocation in between would make `id`
  alias a live object and silently turn the case into another one. Immediately before the call the
  test asserts `vm.inner.objects[id.0 as usize].is_none()`. Kills: a helper that pops before
  validating, and an indexing slip that panics on the freed slot (e.g. `get_object`'s
  `expect("object already freed")`, `vm/inner.rs:299-303`).

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

### §4.3 `apply` over a holey array — moved to 1ae (rev 3)

A1 and A2 are 1ae's, in §9.

Not here, and why: edges 21 and 27/28 are 1ab's and 1ac's (carve-out §6.6). The `None` IC tests are
1ad's (§8). A1/A2 are 1ae's (§9). The brief's call-shape regressions belong to 1b's children
(§0.3 item 2). The existing suite passing is necessary and not sufficient: it does not cover I-O,
which is why T2 and C1 exist.

---

## §5. Non-goals, and what the next slices own

- **`f(...xs)` / `o.m(...xs)` / `new F(...xs)` / optional-call spread** — umbrella 1b's children.
  `compiler/expr_member.rs:70-75` and `compiler/expr.rs:88-92` are not touched, so `Math.max(...[1,2,3])`
  is still `NaN` after this PR. `Op::CallSpread` / `Op::NewSpread` stay stubs
  (`vm/dispatch.rs:879-883`) until those children connect them to D1 + 1ad (and `do_new`).
- **The IC `Option<usize>` entry** — 1ad (§8).
- **`Function.prototype.apply` over a holey array, and D-CL3's length clamp** — 1ae (§9).
- **An Array index's own properties (an own accessor, any non-default descriptor, and the own-property
  statics over Array indices)** — `#11-vm-array-index-property-single-store` (§9).
- **`op_array_spread`'s `return()`** — 1ab.
- **The four argument windows in `vm/interpreter.rs`**, and that file's touch-time split verdict — 1ac.
- **Stack-depth bound** — dec. 12 places it on the child umbrella 1b mints for the argument-emit
  helper. Its site is `lay_out_call_args` (§1.1), so the super path is bounded by the same check,
  with its own `super(...arrOf(bound + 1))` acceptance on that child. 1aa leaves the hook point.
- **D-S1** — Slice S (§3.1). **D-S2** — discharged by #591 (`#11-vm-construct-isconstructor-check`,
  §3.1); no next slice owns it.
- **Catchable internal errors elsewhere on the `super` path** — `#11-vm-internal-error-hard-exit` (§1.3).
- **Folding the default derived constructor onto `ArraySpread`** — forbidden (§15.7.14 step
  14.a.iv.1 NOTE); C1 pins it.

### §5.1 Per-function citation table (the docstrings this PR writes)

| Function | Citation in its docstring | Steps it implements |
|---|---|---|
| `lay_out_call_args` (NEW) | [C19] ECMA-262 §13.3.8.1 ArgumentListEvaluation (consumer side: the List it lays out is [C13] §13.3.7.1 `SuperCall` step 4's argList, consumed at step 6, and §15.7.14 ClassDefinitionEvaluation step 14.a.i's `args`, consumed at step 14.a.iv.4); §15.7.14 step 14.a.iv.1 NOTE (no iteration) | none of ArgumentListEvaluation's own steps: it lays out a List already evaluated, as the hand-off between `SuperCall` steps 4 and 6 and between 14.a.i and 14.a.iv.4 |
| `array_argument_values` (NEW) | the same consuming steps (`SuperCall` step 4 → 6; §15.7.14 14.a.i → 14.a.iv.4), for which holes cannot occur; §7.3.19 CreateListFromArrayLike step 6.b, as the precondition it does **not** meet for a holey array; the representation invariant it assumes (`#11-vm-array-index-property-single-store`) | — |
| `op_super_call_spread` (rewritten) | [C13] ECMA-262 §13.3.7.1 `SuperCall : super Arguments` steps 3-6 (step 4's argList laid out here, steps 3, 5 and 6 via `op_super_call`); §15.7.14 steps 14.a.i and 14.a.iv.1-4, including 14.a.iv.1's NOTE (the second producer) | `SuperCall` step 4's hand-off and 14.a.i → 14.a.iv.4's; steps 3 (✗ D-S1 ordering), 5 (`construct_synchronous`'s `admit_construct`, #591), 6, and 14.a.iv.2-4 (14.a.iv.3 likewise), via `op_super_call` → `dispatch_super` |
| `op_super_call` (re-signed) | [C13] §13.3.7.1 `SuperCall` steps 3-6 (unchanged text; the D-S1 ordering divergence named) | step 4's hand-off (the drain); 3 (✗ D-S1: resolved after the arguments), 5 (`admit_construct`, #591), 6 |
| `collect_array_like` | ECMA-262 §20.2.3.1 Function.prototype.apply step 4 → §7.3.19 CreateListFromArrayLike steps 3-7 (step 2 is done by the caller, `native_function_apply`, `vm/natives_function.rs:71-75`; step 6.c is n/a, since `apply` passes no `validElementTypes`) | the generic loop: 3 (✗ above `DENSE_ARRAY_LEN_LIMIT`, §3; D-CL3, 1ae's), 4-5, 6.a, 6.b, 6.d, 6.e, 7; the dense branch: reads through `array_argument_values`, equal to steps 3-7 for a hole-free Array only (the precondition is 1ae's, §9) |
| `ic_call` / `ic_call_method` | 1ad's: §13.3.6.2 EvaluateCall steps 1-2 (`thisValue`: `ic_call_method`'s receiver; `ic_call`'s `undefined` through `compute_this_for_call`), 4-5 and 7; step 3 (ArgumentListEvaluation) is not theirs (it is the argument-emit helper's, 1b) — §8 | — |

---

## §6. Structural references — re-derivation commands

All at `eaa741b5`, from `crates/script/elidex-js/src/`:

- `git grep -n 'fn compile_arguments\|Argument::Spread' -- compiler/expr_member.rs` → `:53`, `:70`,
  and the super-spread arm's `:123`.
- `git grep -n 'ExprKind::New' -- compiler/expr.rs` → `:88`.
- `git grep -n 'Op::SuperCallSpread as u8' -- compiler/expr_class.rs` → `:149` (inside `:145-152`).
- `git grep -n 'fn op_super_call\|fn dispatch_super\|fn construct_synchronous\|vm.stack.push(JsValue::Object(ctor_id))' -- vm/dispatch_class.rs`
  → `:46`, `:72`, `:104`, `:115`, `:232`, `:304`.
- `git grep -n 'stack underflow\|VmError::internal(' -- vm/dispatch_class.rs`
  → `:55`, `:79`, `:88`, `:321`.
- `git grep -n 'Op::SuperCall =>\|Op::SuperCallSpread =>\|Op::CallSpread | Op::NewSpread' -- vm/dispatch.rs`
  → `:939`, `:884`, `:879`.
- `git grep -n 'fn collect_array_like\|collect_array_like(ctx\|ObjectKind::Array { elements } = &obj.kind\|let length_key' -- vm/natives_function.rs`
  → `:273`, `:76` (1 caller), `:278`, `:124`, `:155`, `:281` (the `:124` / `:155` hits are other natives).
- (rev 4.1) D-S2's line cites (`vm/ops.rs`'s arrow test, `vm/dispatch_class.rs`'s two admission
  tests, `vm/object_kind.rs`'s `is_constructor`) are retired with D-S2 (§3.1). Its discharge is cited
  by name: `git grep -n 'pub(crate) fn is_constructor\|pub(crate) fn admit_construct' -- vm/construct_admission.rs`
  at `d1e9186a` finds both.
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
worktree removed afterwards). The JS of each probe is quoted where its result is cited: §9 (A1/A2,
the own-accessor case and its reach probes), §3.1 (D-S1; rev 4's D-S2 probes are in history at
`e493bd57`), §3.2, §4.2 (C1–C6).

---

## §7. Questions for the plan-review (rev 4)

R1's questions are settled: **Q1** (edge 16): the structural approach is ratified, with the
disjoint borrow and the T2 tripwire and no `debug_assert!` (§1.1). **Q2** (the wrapper): kept, as
"lay out, then `op_super_call`" in `vm/dispatch_class.rs`, so `vm/dispatch.rs` does not grow (§1.3).
**Q3** (stack-depth bound): dec. 12 keeps it on 1b's argument-emit child. **Q4/Q5** (owners): D-S1
and D-S2 go to Slice S (D-S2 since discharged by #591, §3.1), and the `apply` hole was fixed here in rev 2 (it is 1ae's from rev 3, §9).
Rev 2's questions follow. Q1 and Q2 are closed by PM 288, Q3 and Q4 are closed in rev 4 by writing
them onto their ledger rows (R2), and Q5 stands:

- **Q1 — is the remainder terminal? Closed (PM 288): terminal by necessity**, on the facts and
  rules §2 records. The one part separable without cutting `lay_out_call_args`, D6, was split out
  as **1ae** (§9), so the answer is both: 1aa is terminal, and D6 is no longer in it.
- **Q2 — S's kind. Closed (PM 288): re-applied at S's own derivation and plan-review, not now.** S's
  §5 row keeps its ⚠ (three intersecting axes once D-S1/D-S2 are homed there; from rev 4.1, D-S2
  discharged by #591, a count of two or three to be re-taken there) and now names PM 288 as the
  ruling on when it is re-applied.
- **Q3 — the native-constructor re-entry window** (§2, I-G's scope): a native constructor that calls
  user code (e.g. an executor) while `op_super_call`'s `call_args` holds the only reference to the
  arguments. It is pre-existing and not changed here. **Closed (rev 4): handed off, not measured.**
  The question is written on `#11-vm-operand-rooting-by-construction`'s ledger row
  (`2026-07-vm-p4-slot-ledger.md`), with its evidence (`vm/interpreter.rs:136-150`, `:364`), for that
  slot's plan-review.
- **Q4 — `#11-vm-operand-rooting-by-construction`'s trigger** ("now — the next VM dispatch-loop PR,
  or …", ledger). 1aa edits a dispatch-loop arm, so by the wording it fires the trigger: the slot is
  due. That was already its priority ("now"). 1aa does not take the deliverable. The slot is a
  type-level guard over ~20 arms whose cell forbids partitioning it, and 1aa has no artifact edge on
  it, because I-G is held by D1's borrow (no hold the guard would express: no user code runs in D1's
  window). 1ac's `Deps` on it is unchanged. **Closed (rev 4, R2):** the firing and its disposition
  ("scheduled — must land before 1ac starts", 1ac's `Deps` edge) are written on the slot's ledger
  row, and so is `#11-vm-internal-error-hard-exit`'s firing (it lands with this slot, so before 1ac;
  §1.3).
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
from the IC-miss JS branch (`vm/dispatch_ic.rs:253`, `:308`) and the native/non-callable branch
(`:263`, `:325`), but `collect_call_ic` (`vm/ops_property.rs:282-294`) returns `None` for any callee
that is not an `ObjectKind::Function`. So at the base only the JS branch's populate writes a slot;
under the design only `Some` reaches it (I-C).

**Spec rows (moved from rev 1 §3):** §13.3.6.2 EvaluateCall steps 1-2 (thisValueRef → thisValue:
`ic_call_method`'s receiver at `stack[args_start-2]`, `ic_call`'s `compute_this_for_call`), steps 4-5
(not an Object / not callable → TypeError, the non-Object `return Err` at `vm/dispatch_ic.rs:231-233` before any
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
  `None` arm. Assert `matches!(e.kind, VmErrorKind::TypeError)` (`ic_call` returns a `VmError`, not
  a thrown value; `VmErrorKind` has no `PartialEq`, `vm/error.rs:19-20`). Kills: a `None` arm that returns `Ok(undefined)`.
- **T7 `ic_call_method(.., None)`** over `[o, o.m, 4]` with `m(x) { return [this === o, x] }` →
  `[true, 4]`. Same slot precondition, order and comparison as T4, **plus its own `Some` control
  observing IC population** (G4; the 1ad row's acceptance requires one for each entry point). The
  native and non-callable branches as in T5 / T6. Kills: the receiver dropped on the `None` path.

---

## §9. Handed to 1ae — input for its per-PR plan (not 1aa scope)

1ae's plan memo is owed and is not written in this round. What rev 2 designed for D6 is recorded
here so it is not lost. 1ae's plan-review owns it from here.

**Row.** 1ae, a terminal child of umbrella 1a, split from 1aa by the PM's ruling on 1aa's
plan-review (PM 288). Its `Deps` names **1aa**. Judged on the criterion "does the fix live in the
shared primitive's caller-side precondition": it does. The fix is a guard in front of
`collect_array_like`'s call into `array_argument_values`, and that function's docstring (§1.2)
states the guard as the precondition a CreateListFromArrayLike caller owes. Compilation does not
force the edge, because the guard could be written in front of today's inline `.or_undefined()` map.
It is taken anyway. Landing 1ae first would put the guard in front of a map that 1aa then deletes,
so two PRs would place the same line, and A1's mutant ("drop the precondition") would be defined
against a call about to disappear. 1ae also removes the ⚠ line that 1aa's docstring carries for
`collect_array_like`.

**Design (rev 2 §1.5, unchanged).** `collect_array_like` (`vm/natives_function.rs:273-296` at
`eaa741b5`) takes its dense branch (`:278-280`) for **every** `ObjectKind::Array` and maps a hole to
`undefined` with `.or_undefined()`. ECMA-262 §20.2.3.1 step 4 is `? CreateListFromArrayLike(argArray)`,
and §7.3.19 step 6.b is `? Get(obj, indexName)`, which consults the prototype chain for a hole. Probe
at the base:

```
Object.defineProperty(Array.prototype, 0, { get(){ return 'G' }, configurable: true });
function f(a, b){ return '' + a + ',' + b }
f.apply(null, [ , 1])          // elidex: "undefined,1"   spec: "G,1"
```

The fix takes the dense branch only when no element is `Empty`, and then collects through
`array_argument_values` (an identity on a hole-free array). A holey Array falls through to the
existing per-index `get_property_value` loop (`:281-295`), which performs the `Get`. This is dec. 8's
caller-side precondition, which dec. 8 says is where the dense/array-like distinction lives. Dec. 8's
premise that "`collect_array_like` already has the correct `Empty` normalisation" is false for this
caller: dec. 17's reasoning for `Reflect.apply` applies to `Function.prototype.apply` too. The
correction is recorded in dec. 8 (2026-11-03) and points at 1ae. The dense branch stays for the
common hole-free case. Spec rows: §20.2.3.1 step 4 and §7.3.19 step 6.b of §3's map. Citation:
§5.1's `collect_array_like` row, whose dense-branch cell becomes "a hole-free fast path equal to
steps 3-7".

**D-CL3, folded in (PM 310).** `collect_array_like`'s generic loop clamps CreateListFromArrayLike
step 3's length with `.min(DENSE_ARRAY_LEN_LIMIT)` (`vm/natives_function.rs:288`, `vm/ops.rs:24` at
`eaa741b5`; §3.1). It is the same function and the same AO as D6, so 1ae owns it. The spec-correct
outcome (every index below the ToLength result read with `? Get`, and an abrupt `RangeError`, never a
silently shortened List, where the List exceeds what elidex can hold) and its acceptance
(`f.apply(null, { length: 2 ** 27 + 1 })`) are stated on 1ae's §5 row and not restated here. Open for
1ae's plan-review: where that error is raised relative to the observable index `Get`s, and whether
the two limbs intersect (a code read says no, §5 row).

**Tests (rev 2 §4.3, unchanged).** The test module is the implementer's choice: a new
`vm/tests/tests_function_apply_holes.rs`, or the existing `Function.prototype` test module if one
exists.
- **A1** (red at the base, probe `"undefined,1"`). Define a getter on `Array.prototype[0]`
  (`configurable: true`, removed at the end); `f.apply(null, [ , 1])` → `f` sees `"G,1"`. Kills:
  dropping the precondition.
- **A2** dense control (green at the base, probe `"7,1"`): `f.apply(null, [7, 1])` → `"7,1"`, with the
  same getter installed, so an own element is shown to win over the prototype chain. Kills: a fix
  that sends every Array through the generic path and reads the getter for an own element too.
- Red-first: A1 lands with the fix in one commit; A2 is green throughout.
- No own-accessor test, for the reason below.

**The adjacent defect, now homed: `#11-vm-array-index-property-single-store`** (registered by the
PM as `#11-vm-array-index-own-accessor-invisible`, PM 288, and renamed and widened to the whole root
at PM 296; its row is in `2026-07-vm-p4-slot-ledger.md`; the live ledger's Re-eval is 2026-12-01).
An own accessor defined on an Array index is invisible to elidex's own `[[Get]]`. At the base,
`const a = [1, 2]; Object.defineProperty(a, 0, { get(){ return 'G' } })` succeeds, and
`typeof Object.getOwnPropertyDescriptor(a, 0).get` is `"function"`. But `a[0]` is `1`, `[...a]` is
`1,2`, `f.apply(null, a)` sees `1,2`, and `super(...a)` passes `1,2` (spec: `G` each time). The
element vector keeps the stale value beside the accessor. 1ae's precondition is therefore exact
*relative to elidex's `[[Get]]`*, and no 1ae test can be green for this case until the slot lands.
The reach probes run for that row at PM 288 (same probe method as §6, `eaa741b5`) found the defect
wider than its first id: it also hides non-default data descriptors (`value`, `writable: false`,
`enumerable: false`, `configurable: false`), leaves `length` unchanged for a define past the end,
and, with no `defineProperty` at all, `Object.keys([1,2])` is empty,
`Object.prototype.hasOwnProperty.call([1,2], 0)` is `false` and
`Object.getOwnPropertyDescriptor([1,2], 0)` is `undefined`. The PM ruled at PM 296 that the slot owns
that whole root: Array index keys under `[[GetOwnProperty]]`, `[[DefineOwnProperty]]` and
`[[OwnPropertyKeys]]`, with the invariant "a non-`Empty` element is a default data property". The
row records each probe, the code site behind it, the assertions 1ae dropped (own accessor seen by
`f.apply(null, a)`, `super(...a)` and `[...a]`) as acceptance items for its plan-review, and its
order (right after 1b's three call-shape children).
