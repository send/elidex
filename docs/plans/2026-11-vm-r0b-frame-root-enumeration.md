# R-0b — one GC frame-root enumeration (per-PR plan memo, native-construct umbrella §4.1)

## §0. Frame

**Umbrella**: the native-construct protocol (R) umbrella, `docs/plans/2026-11-vm-native-construct-contract.md`
(ratified; on main since #643 `b46a030c`, 11-18; Rev 1.7 at `f9acca16`; its own code anchors are at its base
`4b723d4b`). This memo is the per-PR plan of its §4.1 **R-0b** bullet ("R-0b: one GC frame-root enumeration"), which,
in §4.1's plan-review paragraph, asks for "a short plan memo and `/elidex-plan-review`" because an omitted field in a
root enumeration is a use-after-free. Lane: VM P4. (Rev 1–2.2 read the umbrella at Rev 1.6, `414dd70d` on branch
`vm-native-construct-plan`.)

**Dates** (umbrella §9.1 step 3, PM 688; §4.1's plan-review paragraph, PM 784 (3)): target **11-24**, hard edge
**12-04**; a date that would pass 11-26 is reported to the PM. **Ratification condition** (§9.1 step 3): R-0b is
implemented against the reviewed umbrella. Met: the umbrella was ratified and landed at #643 (11-18), so this memo's
review is the remaining gate before R-0b's implementation, which stays ahead of R-JS's (§9.1 step 6, 11-28 at the
earliest), which changes `CallFrame`'s field set. (Until Rev 2.2 this paragraph made 11-24 conditional on a
ratification by 11-23.)

**Base**: `origin/main` at **`f9acca16`** (Rev 2.2; Rev 1–2.1 were written at `84fee8fa`). Every `file:line` below is at
`f9acca16` unless it names another sha. A cite marked `84fee8fa` states a fact of that tree which a later PR changed; it is
kept as written, not refitted to the new tree (Rev 2.2, §0.4). Paths are under `crates/script/elidex-js/src/` unless
absolute.

### §0.1 Premises re-verified

- The live-frame roots are `mark_roots`'s step (b), `vm/gc/roots.rs:277-315` at `f9acca16` (`// (b) Call frame
  roots` at `:277`, loop `:278-315`). The suspended-frame roots are the `ObjectKind::Generator` arm of `trace_work_list`,
  `vm/gc/trace.rs:330-374` at `f9acca16` (unmoved since `84fee8fa`), frame part `:334-369`, `stack_slice` `:370-372`.
- Both enumerate `this_value`, `upvalue_ids`, `local_upvalue_ids`, `actual_args`, `new_instance`, `mode`,
  `home_class`, `pending_completion`; only `generator` differs (live `roots.rs:300-302`, suspended none). Measured in §2.
- `CallFrame` is **stored** in exactly two places: `VmInner::frames` (`vm/mod.rs`, read by `GcRoots`'s `frames`,
  `vm/gc/roots.rs`) and `SuspendedFrame`'s `frame` (`vm/coroutine_types.rs`). Command, at `f9acca16`:
  `git grep -n -w CallFrame f9acca16 -- crates/script/elidex-js/src` prints 41 lines, classified completely: the two
  storage residencies, 2 (`VmInner::frames`, `SuspendedFrame::frame`); the residency read, 1 (`GcRoots`'s `frames`
  field, `roots.rs`); the struct definition, 1 (`frame.rs`); the impl, 1 (`frame.rs`); imports, 5 (`coroutine_types.rs`,
  `gc/roots.rs`, `interpreter_call.rs`, `mod.rs`, `natives_generator.rs`); struct literals, 4 (three in
  `interpreter_call.rs`, one in `forged_frame`, `vm/tests/tests_with_call_mode.rs`); `CallFrame::tdz_init` calls, 2
  (`interpreter_call.rs`); `create_coroutine`'s by-value parameter, 1 (next bullet); `forged_frame`'s return type, 1
  (`vm/tests/tests_with_call_mode.rs`); doc and comment mentions, 23. Sum 2+1+1+1+5+4+2+1+1+23 = 41. Positive control:
  the same command must print `vm/frame.rs:70:pub struct CallFrame {` (the definition) and
  `vm/mod.rs:192:    pub(crate) frames: Vec<CallFrame>,` (a residency); it does. So the two blocks are the whole
  population of the frame's **storage residencies**, and R-0b's one function serves both.
- A frame being created for a generator or async call is in neither residency until it is installed, and needs no
  third one. Since #650 (`7af70b68`, `#11-vm-coroutine-creation-frame-rooting`) every generator and async call ends in
  `VmInner::create_coroutine` (`vm/natives_generator.rs`; callers `call_internal` and `push_js_call_frame`,
  `vm/interpreter_call.rs`), which takes the frame **by value** and (1) allocates the `Generator` shell
  (`suspended: None`) while the frame's locals, and on the `push_js_call_frame` entry its callee and receiver, are still
  on the VM stack (on the `call_internal` entry `call_dispatch` roots them); (2) roots `actual_args` and the async
  wrapper across that allocation with `push_stack_scope`; (3) drains the locals, sets `frame.generator` and stores the
  `SuspendedFrame`, with no allocation in between. The frame in flight is only that parameter; its handles are rooted on
  the stack, never by a frame enumeration. So R-0b's population stays the two storage residencies, with nothing to
  fold and no ordering against the slot (§0.3). (At Rev 2.1, `84fee8fa`, this window was an open use-after-free.)
- `vm/gc/trace.rs` is 1265 lines at `f9acca16` (1256 at `84fee8fa`); `trace_work_list`'s `match &obj.kind` spans
  `:246-1233` (a flat case table).

### §0.2 Premise errors and drift

- The umbrella cites `vm/gc/roots.rs:277-313` and `vm/gc/trace.rs:335-372`, and the `home_class` sites `:297` / `:357`.
  These were right at `4b723d4b`; #628 (`c0578223`) shifted both files by one line. At `84fee8fa`: `:278-316`,
  `:334-373`, `home_class` at `roots.rs:298` and `trace.rs:358`. `docs/plans/2026-07-vm-p4-slice-plan.md:951` carries
  the same stale pair (`roots.rs:297`, `trace.rs:357`). §5 says who fixes which. At `f9acca16` (#644 `4d7b41dd` removed
  one line of `roots.rs` above the block): `:277-315`, `:334-373`, `home_class` at `roots.rs:297` and `trace.rs:358`.
- The brief's "only `generator` differs" is correct as text; whether the difference is a defect is §2 F-1 (it is not).
- Rev 1 of this memo said "two enumeration sites are the whole population" (§0.1; §3 and §8 read the same way). They
  are the whole population of storage residencies only; a frame in flight during creation is outside both and rooted on
  the stack (§0.1, fourth bullet).

### §0.3 `#11-vm-coroutine-creation-frame-rooting` (closed by #650)

Resolved on the stack-rooting branch: #650 (`7af70b68`, 11-21) keeps the frame-in-flight's handles rooted on the VM
stack and in `create_coroutine`'s stack scope until the install (§0.1, fourth bullet), and adds no GC-visible residency.
So there is no third storage site, nothing for R-0b to fold and no ordering question; R-0b's base `f9acca16` contains
#650. (Was open at Rev 2.1 (`84fee8fa`), with a coupling conditional on the fix's design; closed by #650.)

### §0.4 Revisions

| Rev | Change | Decision |
|---|---|---|
| 1 | First draft at `84fee8fa`. | — |
| 2 | Plan-review R1 at `06a16913` (CRIT 0 / IMP 3 / MIN 12 / FP 1). **K1** population restated as storage residencies; the creation window cited to its slot (§0.1, §0.2, §0.3, §3, §8). **K2** `FrameKind` matched and `HandlerEntry` destructured exhaustively inside `mark_frame` (§4). **K3** design-invariants I-7 added to the coupled sites (§5.2). MIN: D-2's rule applied to `upvalue_slots` (§1.2, §4); why D-3 does not narrow the type (§1.3); the pre-refactor run recorded, the owner-`generator` cell a smoke check (§6); lasting homes for the umbrella and P0a-1 anchor shifts and the other line-shift sites (§5.3-§5.5); §3 restated over §9.9.1 / §9.9.2; umbrella branch, dates and ratification condition (§0); the §4.3 docstring obligation and the acceptance grep's test exclusion (§8). | PM 806 |
| 2.1 | Plan-review R2 Axis 2 re-check (IMP 1, MIN 4). **IMP-1** a field bound as `field: _` could be retyped to a handle type and still compile: every untraced field of `CallFrame` and `HandlerEntry` is now bound to a name and its type pinned at the binding site; the residual narrowed to what a pinned type means to the GC and new handle-carrying variants of shared primitives (§4); a mechanical pin listing and a local retype control added (§8). MIN: the narrowing slot's draft name and its ID cited once registered (§1.3); the pre-existing VM-P4 family anchor drift homed by a proposed slot (§5.5); `:81` and `:768` added to the abort-signal memo's moved cites (§5.5); the suspended `upvalue_slots` cell's observable named (§6). | PM 811 |
| 2.2 | Cites re-anchored to `f9acca16` (drift findings reported to the driver). Code cites whose line only locates a symbol now name the symbol; ranges that are the claim (the two blocks, §2's census, §8's control) are re-measured at `f9acca16`; the §2 census re-run gives the same 17 rows. Cites of facts that #650 (`7af70b68`) or #644 (`4d7b41dd`) changed are marked `84fee8fa` and kept as written (§0.1's creation-window sites and `CallFrame` command, §1.2's four store sites, §5.5's L3 branch and abort-signal memo lines). | — |
| 2.3 | Stale tree statements folded (#650 / #643 / #644 landed), no plan decision changed. #650: the coroutine-creation window is closed on the stack-rooting branch, no third residency (§0.1, §0.2, §0.3, §1 Out, §3, §8); `SuspendedFrame` store sites 4 → 2, D-2 and the `upvalue_slots` subset re-checked at both (§1.2). #643: the umbrella is ratified and on main at Rev 1.7, its cites re-pointed by § (§0, §5.4, §8), and R-JS plan's cite of this memo re-pointed when this memo lands (§5.4). #644: L3's branch landed, the rebase coupling removed, the abort memo's cite lines re-measured at `f9acca16` (§5.5). | — |
| 2.4 | Focused re-check of Rev 2.2–2.3 (CRIT 0 / IMP 0 / MIN 5 + 1 pre-existing) folded; no plan decision changed. §0.1's `CallFrame` census classified completely with per-class counts and a positive control; §1.1's module-doc cite located; §5.4's R-JS cite paragraph put in the past tense; §5.5's abort-signal memo cites read against its `69fbe61f` pin, the PR-body listing dropped; §2's `CallMode` range `:628-641`. | — |
| 2.5 | Post-landing: the D-3 narrowing slot cited by its registered ID (§1); §5.4's obligation marked discharged by the umbrella's Rev 1.8. No plan decision changed. | — |

## §1. Scope

**In**: one free function, `mark_frame` (NEW), that marks every GC handle a `CallFrame` holds; `mark_roots` (b) and
the `Generator` arm of `trace_work_list` call it. New neutrality cells (§6). The memo edits of §5.1 and §5.2.

**Out**: no field of `CallFrame` added, removed or retyped; no change to what is rooted (§1.2 and §1.3 show the three
forms below are no-ops at run time). `stack_slice` and `upvalue_slots` stay in the `Generator` arm (they are
`SuspendedFrame`'s, not the frame's). A frame in flight during creation is not stored: it is `create_coroutine`'s
by-value parameter, rooted on the VM stack by #650 (§0.1, §0.3). `ObjectKind::Function`'s `captured_this` (its arm of `trace_work_list`, `vm/gc/trace.rs`) is R-JS's, not a frame field. P0a-1's
reaction-target arms are not touched (§5.3). `vm/gc/collect.rs` (2064 lines at `f9acca16`) is not touched.

### §1.1 D-1: location — `vm/gc/mod.rs`, beside the mark primitives

`vm/gc/mod.rs` already homes, by its own stated rule, the code "both halves of the mark phase consume"
(`vm/gc/mod.rs`'s module doc, the `## Module layout` section, last paragraph: `mark_value`, `mark_object`, `mark_upvalue`). `mark_frame` is exactly that: a
composite primitive called by the root walker (`roots.rs`) and the work-list tracer (`trace.rs`). It is a free function
taking split borrows like its siblings (`vm/gc/mod.rs`'s module doc, "split borrow — mark bits are `&mut` while
objects/upvalues/stack/frames are `&`"):

`pub(super) fn mark_frame(frame: &CallFrame, upvalues: &[Upvalue], uv_marks: &mut [u64], obj_marks: &mut [u64], work: &mut Vec<u32>)`

Rejected: **`vm/frame.rs`** (an `impl CallFrame` method). `vm/frame.rs` is frame *state* (its module doc); putting mark logic
there makes the frame module depend on the GC's mark bits and work list, against the GC's one-phase-per-file layout
(`vm/gc/mod.rs`'s module doc, "Module layout"). **A new `vm/gc/frame.rs`**: a ~45-line file for one function that `mod.rs`'s existing rule
already homes. **Either caller file**: the other half would then import from a sibling phase file.

Size: `vm/gc/mod.rs` 122 → ~195 (§7).

### §1.2 D-2: `generator` — marked in `mark_frame`, for both callers (no parameter)

`mark_frame` marks `generator` like every other handle. For a suspended frame that call is a no-op:

1. `susp.frame.generator` is `None` or the generator that owns `susp`. At `f9acca16` a `SuspendedFrame` has two store
   sites (`git grep -n -P 'suspended = Some\(|suspended: Some\(' f9acca16 -- crates/script/elidex-js/src`, 2 hits;
   positive control: the same command at `84fee8fa` gives the 4 sites of Rev 2.1, which #650 merged into
   `create_coroutine`): `vm/natives_generator.rs:173` (`create_coroutine`) allocates the generator shell at `:148`, sets
   `frame.generator = Some(gen_id)` to that same object at `:169` and stores the frame into it at `:173`;
   `vm/natives_generator.rs:78` (`op_yield_suspend`) stores the frame into the generator read from that frame's own
   `generator` (`:45-49`, `:72-78`; `frame_idx` is the top frame, `vm/dispatch.rs:42`, the one `frames.pop()` takes).
2. The arm runs on an object popped from the work list, and an object is pushed only after its mark bit is set
   (`mark_object`, `vm/gc/mod.rs`). So `mark_object(owner)` finds the bit set and returns.

Rejected: **the caller keeps `generator`** — the shared function would still have to name the field (§4's exhaustive
pattern) as `generator: _`, an intentional omission whose soundness lives in a comment, and the live caller would carry
one field outside the enumeration. **A `bool` / enum parameter** — a mode flag whose only effect is skipping a no-op;
it would also be the first place a future frame-site difference gets bolted on, re-splitting the enumeration. One
enumeration with no mode is the One-issue-one-way form. If the invariant in step 1 were ever broken, D-2 roots the
foreign generator rather than leaving it unrooted: the fail-safe direction.

**The same rule for `SuspendedFrame`'s `upvalue_slots`** (Rev 2). Today neither block traces it (O-2). The `Generator`
arm marks each of its ids with `mark_upvalue`, beside `stack_slice` (both are `SuspendedFrame`'s, not the frame's,
so they stay in the arm, not in `mark_frame`). That call is a no-op for the same two-step reason: (1) every id in
`upvalue_slots` is also in `susp.frame.local_upvalue_ids` (it is pushed only while iterating that list,
in `op_yield_suspend`, `vm/natives_generator.rs`, and the list is preserved verbatim across resume, `resume_generator`'s
reopen-loop comment; the other store site, `create_coroutine`, stores it empty), which `mark_frame` marks; (2) `mark_upvalue` returns at once on a set bit (its bit test,
`vm/gc/mod.rs`), and either order marks the same
closed value. Binding it as `upvalue_slots: _` would rest its soundness on that comment, the form D-2 rejects for
`generator`; marking it keeps one rule for every handle and falls on "rooted" if the subset invariant ever breaks.

### §1.3 D-3: `pending_completion` — mark the value of every `FrameCompletion` variant

Both blocks today mark `Return(v)` / `Throw(v)` and skip `Normal(_)` (`roots.rs:306-314`, `trace.rs:361-369`).
`Normal(JsValue)` is never stored there: the only write is inside `route_to_next_finally` (`vm/ops.rs`), and its
two callers pass `FrameCompletion::Return(v)` (`op_end_finally`, `vm/ops.rs`; `resume_generator`'s `Return` arm,
`vm/natives_generator.rs`); the type's doc says "as a pending completion it is unused" (`FrameCompletion`,
`vm/coroutine_types.rs`). `mark_frame` marks the value of all three
variants with one exhaustive or-pattern (a new variant fails to compile). At run time this is a no-op (no writer
produces `Normal`); it removes the one place where the type admits an unrooted `JsValue` in a frame. The alternative
(keep skipping `Normal`) is equally neutral; D-3 is chosen by the fail-safe direction (an unknown case falls on
"rooted"). **For plan-review**: if the reviewers read D-3 as outside "behaviour-neutral", keep today's arms.

**Why D-3 does not narrow the type instead** (Rev 2). The cleaner fix is a pending-completion type with only `Return`
and `Throw`, so `Normal` cannot be stored at all. That retypes a `CallFrame` field (`CallFrame::pending_completion`, `vm/frame.rs`), which §1's Out
forbids, and its writers and readers are outside `vm/gc/` (`route_to_next_finally`'s parameter, `vm/ops.rs`; its
callers `op_end_finally` and `resume_generator`; `op_end_finally`'s `None | Some(Normal(_))` arm, `vm/ops.rs`,
which is dead for the same reason). `FrameCompletion` itself stays three-variant: it is also the resumption kind, where
`Normal(v)` is the `.next(v)` argument (`FrameCompletion`'s doc, `vm/coroutine_types.rs`). The narrowing is its own change, the slot
**`#11-vm-pending-completion-type-narrowing`** (its scope includes deleting the dead `None | Some(Normal(_))` arm in
`op_end_finally`). When it lands, D-3's `Normal` alternative disappears with the variant: the
or-pattern stops compiling until it is removed, so the two cannot drift.

## §2. Field census (at `f9acca16`; the same result at `84fee8fa`)

**Method**: list the fields from the struct (`vm/frame.rs:70-152`), then count, per field, the hits of `frame.<f>` in
the live block and of `susp.frame.<f>` in the suspended block:

```sh
S=f9acca16; D=crates/script/elidex-js/src/vm
for f in $(git show $S:$D/frame.rs | sed -n '70,152p' | /usr/bin/grep -oE '^    pub [a-z_]+:' | sed -E 's/^    pub ([a-z_]+):/\1/'); do
  l=$(git show $S:$D/gc/roots.rs | sed -n '278,315p' | /usr/bin/grep -c -w "frame\.$f")
  s=$(git show $S:$D/gc/trace.rs | sed -n '334,373p' | /usr/bin/grep -c -w "susp\.frame\.$f")
  printf '%-20s live=%s susp=%s\n' $f $l $s; done
```

17 fields (at `84fee8fa` the `roots.rs` range was `279,316`; same output). **Control**: `this_value` → live=1 susp=1
(known-marked in both, `roots.rs:279`, `trace.rs:339`); `generator` → live=1 susp=0 (known asymmetry, `roots.rs:300`), so the method separates the cases. GC relevance is read
from the field's **type**, not from the blocks: the GC collects `Object`s and `Upvalue`s only; strings, symbols,
shapes and compiled functions are permanent (`vm/gc/mod.rs`'s module doc). `ObjectId` indexes `Vm::objects`, `UpvalueId` indexes `Vm::upvalues`,
`FuncId` indexes `Vm::compiled_functions` (each type's doc, `vm/value.rs`).

| # | Field (`vm/frame.rs`) | Type | GC handle? | Live (`roots.rs`) | Suspended (`trace.rs`) |
|---|---|---|---|---|---|
| 1 | `func_id` `:72` | `FuncId` | no (compiled functions are permanent) | — | — |
| 2 | `ip` `:74` | `usize` | no | — | — |
| 3 | `base` `:76` | `usize` | no | — | — |
| 4 | `upvalue_ids` `:78` | `Arc<[UpvalueId]>` | **yes** | `:280-282` | `:340-342` |
| 5 | `local_upvalue_ids` `:80` | `Vec<UpvalueId>` | **yes** | `:283-285` | `:343-345` |
| 6 | `this_value` `:82` | `JsValue` | **yes** | `:279` | `:339` |
| 7 | `exception_handlers` `:84` | `Vec<HandlerEntry>` | no (`Option<u32>` ×2, `usize`; `:214-221`) | — | — |
| 8 | `tdz_bits` `:87` | `u64` | no | — | — |
| 9 | `tdz_overflow` `:90` | `Box<[u64]>` | no | — | — |
| 10 | `actual_args` `:93` | `Option<Vec<JsValue>>` | **yes** | `:286-290` | `:346-350` |
| 11 | `cleanup_base` `:96` | `usize` | no | — | — |
| 12 | `new_instance` `:100` | `Option<ObjectId>` | **yes** | `:291-293` | `:351-353` |
| 13 | `mode` `:111` | `CallMode` (`Construct { new_target: ObjectId }`, `vm/value.rs:628-641`) | **yes** | `:294-296` | `:354-357` |
| 14 | `home_class` `:121` | `Option<ObjectId>` | **yes** | `:297-299` | `:358-360` |
| 15 | `kind` `:134` | `FrameKind` (fieldless, `:58-67`) | no | — | — |
| 16 | `generator` `:138` | `Option<ObjectId>` | **yes** | `:300-302` | **none** (F-1) |
| 17 | `pending_completion` `:151` | `Option<Box<FrameCompletion>>` (`Normal`/`Return`/`Throw`, each a `JsValue`; `vm/coroutine_types.rs:66-70`) | **yes** | `:306-314` (`Return`, `Throw`) | `:361-369` (`Return`, `Throw`) |

Ten GC-relevant fields; nine marked by both blocks, one by the live block only.

### §2.1 Findings

- **F-1 (`generator`, live only). Not a defect.** The suspended frame's `generator` is `None` or its owner, which is
  already marked when its arm runs (§1.2 steps 1-2). No use-after-free; no slot. D-2 folds it into the one enumeration.
- **No field is GC-relevant and marked by neither block.**

### §2.2 Observations (sub-field level, not findings)

- **O-1** `FrameCompletion::Normal(v)` in `pending_completion` is marked by neither block; it has no writer (§1.3).
  Handled by D-3.
- **O-2** `SuspendedFrame`'s `upvalue_slots` (`vm/coroutine_types.rs`, `Vec<(UpvalueId, u16)>`) is not traced. It is a
  subset of `frame.local_upvalue_ids` (built from it in `op_yield_suspend`, `vm/natives_generator.rs`; the list is preserved verbatim
  across resume, `resume_generator`), which `mark_frame` marks. Sound today; Rev 2 marks it anyway, by D-2's rule (§1.2).
- **O-3** The block comment at `trace.rs:335-338` (at `f9acca16`) lists four of the frame's handle fields and is already incomplete; it
  is deleted with the block.

## §3. Spec coverage map

No algorithm is implemented or changed: the GC is engine-internal. The row is the clause this slice's code answers
to, as in P0a-1's §3 (a resource limit) and PR-A2's §3. ECMA-262 §9.9.2 Liveness (`webref body ecma262
sec-liveness`) defines when a set of objects is live (two conditions: membership in a `[[KeptAlive]]` List, or a valid
future WeakRef-oblivious execution that observes an element's identity); its Note 4, informative, glosses the second as
identity "observable via non-WeakRef means". §9.9.1 Objectives (`webref body ecma262 sec-weakref-invariants`) says
that objects which are not live "may be released". The spec does not require a field to be marked: Note 5 says a value's
presence in a field does not make it live, and Note 6 that liveness is undecidable and engines use conservative
approximations such as reachability. Marking every handle field of a frame is that approximation (reachable ⊇ live).
**The hard requirement is the engine's own soundness** (§0): a handle the frame later reads and the collector did not
mark is swept into the free list (`sweep_objects`, `vm/gc/sweep.rs`) and its `ObjectId` reused, a use-after-free. R-0b's acceptance
(§6) is that every GC-relevant field of §2 stays marked, live and suspended. The repo rules that shape the change
(設計優先, One issue one way, 1000-line debt) are CLAUDE.md's, not a spec's.

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| ECMA-262 §9.9.2 Liveness (with §9.9.1 Objectives) | Note 6: reachability as the conservative approximation of liveness; R-0b keeps every handle a stored frame holds reachable | a frame field as the only reference, in either storage residency (live frame, suspended frame); a frame in flight during creation is rooted on the VM stack by `create_coroutine` (#650, §0.1), not here | `mark_frame` (NEW, `vm/gc/mod.rs`); callers `mark_roots` (`vm/gc/roots.rs:277-315`), `trace_work_list` `Generator` arm (`vm/gc/trace.rs:330-374`) | ✓ (exhaustive `CallFrame` pattern, §4) | yes (script decides what each field holds) |

**Breadth**: K=1, M=1.

### §3.1 User-input touch audit

No new site reachable by web content. Script reaches both callers only through allocation-triggered collection; the
rooted set is unchanged (§1.2, §1.3).

## §4. Completeness mechanism

`mark_frame` opens with an **exhaustive destructuring of `CallFrame` without `..`**:

```rust
let CallFrame {
    upvalue_ids, local_upvalue_ids, this_value, actual_args, new_instance,
    mode, home_class, generator, pending_completion,
    exception_handlers, kind,
    func_id, ip, base, tdz_bits, tdz_overflow, cleanup_base,
} = frame;
// Untraced: no GC handle in these types (permanent / plain data). Each type is pinned here.
let _: &FuncId = func_id;
let _: &usize = ip;
let _: &usize = base;
let _: &u64 = tdz_bits;
let _: &Box<[u64]> = tdz_overflow;
let _: &usize = cleanup_base;
let _: &FrameKind = kind;
match kind { FrameKind::Eval | FrameKind::Function => {} }
for HandlerEntry { catch_ip, finally_ip, stack_depth } in exception_handlers {
    let _: &Option<u32> = catch_ip;
    let _: &Option<u32> = finally_ip;
    let _: &usize = stack_depth;
}
```

Adding, removing or renaming a `CallFrame` field fails to compile here until the author classifies it: traced (bound
and marked) or not traced (bound and type-pinned in the commented group). Deliberately untraced fields are therefore
named, not absent: the omission is a reviewed line. The nested types are matched exhaustively too: `mode` by
`match mode { CallMode::Call => {}, CallMode::Construct { new_target } => … }` (today's `if let` in both blocks is
not exhaustive); `pending_completion` by the or-pattern of D-3. The `Generator` arm destructures `SuspendedFrame`
the same way (`let SuspendedFrame { frame, stack_slice, upvalue_slots } = &**susp`; all three are marked, §1.2).

**Type pins** (Rev 2.1). A field bound as `field: _` matches any type, so retyping it (say `cleanup_base: usize` →
`ObjectId`, or `HandlerEntry`'s `catch_ip` → a handle type) would still compile and leave the new handle unmarked. Each
untraced field is therefore bound to a name and ascribed its type at the binding site, one `let _: &T = field;` line
per field: a retype fails to compile on that line until the author re-classifies the field. The pinned set at
`f9acca16`, unchanged since `84fee8fa` (`CallFrame` and `HandlerEntry`, `vm/frame.rs`): `CallFrame`'s `func_id: FuncId`, `ip: usize`, `base: usize`,
`tdz_bits: u64`, `tdz_overflow: Box<[u64]>`, `cleanup_base: usize`, `kind: FrameKind`; `HandlerEntry`'s
`catch_ip: Option<u32>`, `finally_ip: Option<u32>`, `stack_depth: usize`. (`kind`'s pin is redundant with its
wildcard-free match, which already rejects another type; it is kept so that every untraced field has one.) A pin is a
coercion site, so a retype to a type that `Deref`s to the pinned type would pass it; no GC handle type implements
`Deref` (at `f9acca16`, as at `84fee8fa`, the crate's `Deref` impls are `VmTempRoot` and `VmStackScope`, `vm/temp_root.rs`,
and a test guard), and a frame field of either would be a design change of its own. Traced fields need no pin: each is passed
to a typed mark primitive (`mark_value`, `mark_object`, `mark_upvalue`), so a retype fails there. If clippy's
`borrowed_box` lint fires on the `&Box<[u64]>` pin, it takes a local `#[allow]` with that reason; it is not rewritten
as `&[u64]`, which would accept any type that derefs to a slice.

**The frame's own composite types** (Rev 2, closing Rev 1's stated residual). `exception_handlers` and `kind` are
bound, not `_`, and each is classified at its own level:

- **`HandlerEntry`** (`vm/frame.rs`; `catch_ip`, `finally_ip`: `Option<u32>`; `stack_depth`: `usize`) is
  destructured per element without `..`, each field type-pinned. A field added to it fails to compile at this loop,
  which is also where a handle-typed field would be bound and marked. The body holds only type ascriptions, so at most
  it costs one pass over the frame's handler list per collection (the optimiser may drop it).
- **`FrameKind`** (`vm/frame.rs`, fieldless: `Eval`, `Function`) is matched without a wildcard. A new variant
  fails to compile; so does a field added to `Eval` or `Function`, since a unit pattern does not match a tuple or
  struct variant.

**What is still not covered, precisely** (Rev 2.1): two things, neither a change to a frame type's shape.
(1) What a pinned type *means* to the GC: the pins fix `FuncId`, but `FuncId`'s permanence (an index into
`compiled_functions`, permanent by the GC's rule, `vm/gc/mod.rs`'s module doc) is that rule's, which every arm of the collector
reads; making compiled functions collectable changes the rule, not the frame. (2) A new handle-carrying variant of a
shared primitive type (`JsValue`, marked through `mark_value`): it is those primitives' concern, shared with every arm,
not R-0b's.

The compiler, not a test, is the guard; the §6 cells prove today's rooted set, not future completeness.

## §5. Coupled sites

### §5.1 Umbrella 3 (`docs/plans/2026-07-vm-p4-slice-plan.md:951`, VM-P4 family)

Its obligation reads "the retirement of the two frame-marking sites" of `home_class`, citing `vm/gc/roots.rs:297` and
`vm/gc/trace.rs:357` (one line stale at `84fee8fa`, §0.2; at `f9acca16` `roots.rs:297` is the `home_class` line
again and `trace.rs:357` is still one line off, `:358`). After R-0b there is one site, inside `mark_frame`. **R-0b's
implementation PR edits** the two cites into `mark_frame` (`vm/gc/mod.rs`) and "the two frame-marking sites" into "the
frame-marking site"; nothing else in the obligation changes (it still owes a tracing edge for the closure-carried home
object). Then: `python3 .claude/tools/plan-memo-umbrella-check.py docs/plans/2026-07-vm-p4-es-language-completeness.md`
→ exit 0 (baseline at `f9acca16`: exit 0). No link to a memo outside the family is added
(`docs/plans/test_vm_p4_family_layout.py`).

### §5.2 Design invariant I-7 (`docs/plans/2026-07-vm-p4-design-invariants.md:245-249`, VM-P4 family)

I-7 ("A representation change is a collector change") states the frame marking of `new_target` and `home_class` as
"**4 sites, 2 fields**": `mark_roots` (`vm/gc/roots.rs:294-297`) and the suspended-generator arm (`vm/gc/trace.rs:353-357`),
"the whole of" `grep -rn 'home_class\|new_target' crates/script/elidex-js/src/vm/gc/` less its two doc-comment hits.
At `f9acca16` the four sites are `roots.rs:294-295`, `:297-299`, `trace.rs:354-357`, `:358-360`, and the doc-comment
hits `roots.rs:58`, `:63` (`git grep -n -E 'home_class|new_target' f9acca16 -- crates/script/elidex-js/src/vm/gc/`,
8 hits; at `84fee8fa` the two `roots.rs` sites were one line later). After R-0b both fields are marked at **one** site, `mark_frame`, so the sentence goes false when R-0b lands.
**R-0b's implementation PR rewrites it**: one site (`mark_frame`, `vm/gc/mod.rs`), 2 fields; the population command
restated so that it runs from the repository root (I-7's own ⚠ on bare paths) and excludes `vm/gc/tests.rs`, whose §6
cells name both fields; the hit count measured at the PR's head. Nothing else in I-7 changes (the sole-root rule and
the "net loss of a strong edge" consequence still hold, for one site). It is a VM-P4 family memo: the same PR runs
§5.1's entry-memo checker (exit 0) and adds no out-of-family link.

### §5.3 P0a-1 D-16 (`docs/plans/2026-11-vm-stack-safety-guard-p0a1.md`)

P0a-1 traces `ReactionTarget` / `ReactionOwner` ids at `vm/gc/roots.rs:416-428` and `vm/gc/trace.rs:284`, and cites
the `ReadableStream*Step` trace arms at `trace.rs:969-977` (memo `:523`, `:598` at `f9acca16`; `:516`, `:591` at `84fee8fa`). Different statements; either order
(umbrella §8.1). R-0b edits nothing in P0a-1. Line drift only: removing the frame blocks shifts `trace.rs` lines after
`:334` (so `:969-977`, not `:284`) and `roots.rs` lines after `:277` (so `:416-428`). P0a-1 is ratified and lands
11-28; R-0b does not edit a ratified memo. **Home for the shifted anchors**: if R-0b lands first (the planned order),
P0a-1's implementation PR (VM P4) carries them in its PR body, as a note directly under the body's pointer to its plan
memo: "anchors moved by R-0b (#n, sha): memo `:523` `gc/roots.rs:416-428` → `:a-b`; memo `:598` `gc/trace.rs:969-977`
→ `:c-d`; `gc/trace.rs:284` unchanged", the numbers measured at that PR's base. R-0b's own PR body lists the same
anchors for P0a-1's implementer. If P0a-1 lands first, the anchors are history and nothing is owed.

### §5.4 R-JS and the umbrella (umbrella §4.2)

R-JS changes the frame's field set (`new_instance`). After R-0b it edits one function, and §4's pattern fails to
compile until it does. The umbrella (on main, ratified, Rev 1.7 at `f9acca16`, §0) writes its cells that name the two
blocks or two sites at its base `4b723d4b`; they stay true of that tree, and after R-0b they describe a tree that no
longer exists. In the main file: §2.1 A3×A9 (the `vm/gc/trace.rs` prereq split) and A5×A9 ("two copies of one
enumeration; R-JS edits both"); §4.1's R-0b bullet; §4.2's first edge to umbrella 3 ("Two edges to umbrella 3", item
(1), `roots.rs:297` / `trace.rs:357`); §7's `vm/gc/trace.rs` row (the suspended-frame `new_instance` at `:350`); §8.1's
`mark_roots` / `trace_work_list` row; §9.1 step 3 ("both copies change"). **Obligation**: the umbrella's first revision
after R-0b lands (owner VM P4, the umbrella's owner) re-points these cells to `mark_frame` (one site). The R-JS plan
on main records the same item with its trigger and due date (`docs/plans/2026-11-vm-native-construct-r-js.md` §9,
"Umbrella items left for a later revision"). R-0b's PR body and its landing report to the PM name this obligation;
this section is its durable statement. Discharged by the umbrella's Rev 1.8.

**The R-JS plan's cite of this memo.** The R-JS plan (`docs/plans/2026-11-vm-native-construct-r-js.md`, #657) read
this memo as an in-flight claim at Rev 2.1 (`c2e3dd5f`, branch `vm-r0b-frame-roots`, not on main), with its landing
order: this plan lands before R-JS-1 starts, and the cite is re-pointed to its main path when it lands (PM 899 / PM 904:
no repo cite to a local path). This plan PR re-pointed that cite (R-JS memo `:14-16`) to
`docs/plans/2026-11-vm-r0b-frame-root-enumeration.md`, still recording that it was read as a claim at Rev 2.1.

### §5.5 Other line-shift sites (not R-0b's to edit)

Removing the two blocks shifts every later line of `vm/gc/roots.rs` (after `:277`) and `vm/gc/trace.rs` (after
`:334`).

- **`docs/plans/2026-11-abort-signal-any-flattening.md`** (L3, on main; its PR-A landed as #644 `4d7b41dd`, so no L3
  branch is in flight over these files) cites, at `f9acca16`, lines after the blocks: `trace.rs:385-398` (`:89`,
  `:781`, `:818`, `:966`), `trace.rs:387` (`:320`, `:529`), `trace.rs:540` (`:189`), note (k) `roots.rs:600-618`
  (`:87`, `:776`, `:817`) and the line counts `roots.rs` 619 (`:817`; 618 at `f9acca16`) and `trace.rs` 1255 (`:818`,
  `:966`; 1265 at `f9acca16`), both changed again by R-0b (§7). That memo pins every code fact at `69fbe61f` ("line numbers move, the
  commands do not", its header at `:4-5` on main), so R-0b makes none of its cites false: each stays true at its pinned sha. They are
  L3's memo, and R-0b does not edit it. L3's next PR in that program re-measures them at its own base, the owner Rev
  2.1 named (L3's implementation). Its `roots.rs:257-261` (`:86`, `:776`, `:817`), `roots.rs:100-101` and
  `trace.rs:32-35` (`:113`) precede the blocks and do not move. Rev 2.1's obligation that R-0b's PR body list the moved
  lines is dropped: the pin makes it moot, since no cite becomes false, and lines measured at R-0b's head would be stale
  again at L3's base, where the re-measurement belongs.
- **VM-P4 family memos whose frame-block cites are already stale at `84fee8fa`** (R-0b did not make them stale):
  `docs/plans/2026-07-vm-p4-slot-ledger.md:286` (`roots.rs:294-295`, `trace.rs:353-355`), `2026-07-vm-p4-slice-plan.md:943`
  and `2026-07-vm-p4-slice-1a-1b-call-spread-detail.md:470` (`trace.rs:345-370`). They are pre-existing and
  outside R-0b. No slot homes VM-P4 family code-anchor sweeps today; the driver proposes one to the PM as their home (a
  slot, not a report). (§5.1's `slice-plan.md:951` and §5.2's I-7 are edited by R-0b because R-0b makes their *statement* false, not only their line numbers.)

## §6. Tests

**Existing coverage** (none isolates a single frame field as the only reference): `gc_correctness_under_stress` and
`gc_heap_bounded_in_loop` (`vm/gc/tests.rs`; low `gc_threshold`, live frames under collection);
`saved_completion_stack_roots_displaced_object_through_gc` (`vm/tests/tests_frame_completion_disentanglement.rs`);
generator yield/resume cycles without a forced collection (`generator_closure_survives_many_yield_resume_cycles`,
`vm/tests/tests_generator.rs`).

**New cells** (`vm/gc/tests.rs`, 407 lines; hand-built `CallFrame`s, as `forged_frame` in `vm/tests/tests_with_call_mode.rs` does).
Each allocates a fresh ordinary object reachable **only** through one handle, calls `vm.inner.collect_garbage()`, and
asserts the object is still allocated:

| Handle | Live (frame pushed on `vm.inner.frames`) | Suspended (frame in a `Generator`'s `SuspendedFrame`, generator rooted via a global) |
|---|---|---|
| `this_value` | ✓ | ✓ |
| `upvalue_ids` (closed upvalue holding the object) | ✓ | ✓ |
| `local_upvalue_ids` (same) | ✓ | ✓ |
| `actual_args` | ✓ | ✓ |
| `new_instance` | ✓ | ✓ |
| `mode` = `Construct { new_target }` | ✓ | ✓ |
| `home_class` | ✓ | ✓ |
| `generator` | ✓ (a separate generator object) | **smoke** (= the owner; asserts the owner survives and the cell does not panic) |
| `pending_completion` `Return` / `Throw` | ✓ | ✓ |
| `pending_completion` `Normal` (D-3) | ✓ | ✓ |
| `generator` = a foreign generator (D-2's fail-safe) | — | ✓ |
| `upvalue_slots` holding an id outside `local_upvalue_ids` (D-2's rule, §1.2); observable: the object the upvalue's `Closed` value holds stays allocated | — | ✓ |

Plus a **negative control** per column: the same frame with the handle cleared → the object is collected (the cells
can see a collection). **Exempt**: the suspended owner-`generator` cell is a smoke check, not a neutrality cell. Its
object is the generator rooted through the global, so it survives whether or not the frame's field is marked: it
cannot fail, has no negative control, and proves only that marking the owner from inside its own arm does not panic
or loop. The foreign-generator row is the cell that can see `generator` marked in a suspended frame.
The `upvalue_slots` cell asserts on the closed value's object, like every other row, not on the upvalue: an unmarked
upvalue is reset by `sweep_upvalues` to `Closed(Undefined)` (`vm/gc/sweep.rs`), which would also be observable,
but the object form keeps one observable for the whole table and its negative control (id removed from
`upvalue_slots` → the object is collected) is the same shape as the others.

**Commits and the recorded neutrality run.**

1. **First commit** (cells only; every row except the last three, `Normal`, foreign `generator`, `upvalue_slots`):
   green on the pre-refactor code. Run, at that commit,
   `cargo test -p elidex-js --all-features -- vm::gc::tests` and record in the PR body that commit's sha, the
   command and its `test result:` line (passed count, 0 failed). This is the neutrality evidence, measured, not argued.
2. **Refactor commit** (`mark_frame`, the two callers, the last three rows): the same command green, recorded the same
   way. The first commit's cells stay green: that is the neutrality proof. The last three rows land here because each
   builds a state no writer produces (§1.2, §1.3) and is red before the refactor by design; they document D-2's and
   D-3's fail-safe effects.

Before push: `cargo fmt --all`; `cargo test -p elidex-js --all-features`; `mise run ci`; `/pre-push`.

## §7. 1000-line verdicts (`wc -l` at `f9acca16`; after = estimate)

| File | Now | After | Verdict |
|---|---|---|---|
| `vm/gc/trace.rs` | 1265 | ~1234 | R-0b is the real seam (umbrella §7). The remainder is `trace_work_list`'s `match` over `ObjectKind` (`:246-1233`), a flat case table: exempt. Stays over 1000. |
| `vm/gc/roots.rs` | 618 | ~583 | under 1000 |
| `vm/gc/mod.rs` | 122 | ~195 | under 1000 |
| `vm/gc/tests.rs` | 407 | ~650 | under 1000 |

## §8. Acceptance

- `mark_frame` is the only code that reads a stored `CallFrame`'s handles for the GC (both storage residencies, §0.1;
  a frame in flight during creation is `create_coroutine`'s by-value parameter, rooted on the VM stack by #650, and is
  outside this criterion).
  `git grep -n -P 'frame\.(this_value|home_class|new_instance)' -- crates/script/elidex-js/src/vm/gc ':!crates/script/elidex-js/src/vm/gc/tests.rs'`
  returns no hit (`mark_frame` reads the handles through §4's bindings). `vm/gc/tests.rs` is excluded because §6's
  cells build and clear frames there (`frame.this_value = …` in a negative control is a test write, not a GC read).
  Control: the same command at `f9acca16` returns six hits, `roots.rs:279`, `:291`, `:297`, `trace.rs:339`, `:351`,
  `:358` (with or without the exclusion: `vm/gc/tests.rs` has none at `f9acca16`; at `84fee8fa` the `roots.rs` hits
  were one line later).
- §4's `CallFrame` and `SuspendedFrame` patterns and the `HandlerEntry` pattern have no `..`; the `FrameKind`, `CallMode`
  and `FrameCompletion` matches have no wildcard. §6's cells green, both runs recorded in the PR body (§6 steps 1-2);
  the negative controls collect.
- **Type pins** (§4, Rev 2.1). (a) Every field `mark_frame`'s `CallFrame` and `HandlerEntry` patterns bind and do not
  mark has a type pin. The listing below, run from the repository root at the PR's head, prints each name the two
  patterns bind as `pinned` (a `let _: &T = <name>;` line exists) or `unpinned`, and exits 0 only if no binding is a
  `_`-equivalent (`f: _`, `f: _x`, `..`), the pinned set is exactly §4's ten fields and the unpinned set exactly the ten
  bound for marking (the nine traced fields of §2 and `exception_handlers`, whose elements are destructured); a PR that
  changes either type's fields updates both sets with its classification:
  ```sh
  python3 - crates/script/elidex-js/src/vm/gc/mod.rs <<'PY'
  import re, sys
  body = re.search(r'fn mark_frame\b.*?\n\}\n', open(sys.argv[1]).read(), re.S).group(0)
  pats = re.findall(r'(?:let CallFrame|for HandlerEntry)\s*\{(.*?)\}', body, re.S)
  bound = [b.strip() for p in pats for b in re.sub(r'//[^\n]*', '', p).split(',') if b.strip()]
  bad = [b for b in bound if not re.fullmatch(r'[a-z_][a-z0-9_]*', b)]
  pinned = {b for b in bound if re.search(r'let _: &[^=;]+= %s;' % re.escape(b), body)}
  for b in bound: print(b, 'NOT A NAME' if b in bad else 'pinned' if b in pinned else 'unpinned')
  pin = {'func_id', 'ip', 'base', 'tdz_bits', 'tdz_overflow', 'cleanup_base', 'kind', 'catch_ip', 'finally_ip', 'stack_depth'}
  mark = {'upvalue_ids', 'local_upvalue_ids', 'this_value', 'actual_args', 'new_instance', 'mode', 'home_class',
          'generator', 'pending_completion', 'exception_handlers'}
  sys.exit(1 if bad or len(pats) != 2 or pinned != pin or set(bound) - pinned != mark else 0)
  PY
  ```
  Control (run on a fixture of §4's code block, not committed): exit 0 as written; exit 1 with `cleanup_base` bound
  as `cleanup_base: _` and its pin removed; exit 1 with only `stack_depth`'s pin removed.
  (b) A **local control, not committed**: retype one pinned field (`CallFrame`'s `cleanup_base` to `ObjectId`), run
  `cargo check -p elidex-js --all-features`, and show the build fails at its pin in `mark_frame`; record the error line
  (`error[E0308]: mismatched types` with its `vm/gc/mod.rs:<line>`) in the PR body, then revert.
- **Docstring** (umbrella §4.3, the docstring bullet: every NEW function of the program carries a docstring naming the AO or
  algorithm it implements). `mark_frame` implements no AO; its docstring names the clause it answers to, ECMA-262
  §9.9.2 Liveness, and states that marking every handle field is the reachability approximation Note 6 allows (§3),
  over a frame in either storage residency.
- §5.1's and §5.2's edits made; entry-memo checker exit 0; `python3 .claude/tools/ecs_mission_ratchet.py` 0 new.
- §5.3's and §5.5's anchor lists and §5.4's obligation are in the PR body.
