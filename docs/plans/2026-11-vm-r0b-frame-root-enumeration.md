# R-0b — one GC frame-root enumeration (per-PR plan memo, native-construct umbrella §4.1)

## §0. Frame

**Umbrella**: the native-construct protocol (R) umbrella, `docs/plans/2026-11-vm-native-construct-contract.md`
(**not ratified, not on main**; frozen text read at `ae51abd7` on branch `vm-p4-native-construct-contract`). This memo
is the per-PR plan of its §4.1 **R-0b** bullet ("a short plan memo and `/elidex-plan-review`", because an omitted field
in a root enumeration is a use-after-free). Lane: VM P4. Date: **11-24** (umbrella §9.1 step 3, PM 688), before R-JS's
implementation (11-28), which changes `CallFrame`'s field set.

**Base**: `origin/main` at **`84fee8fa`**. Every `file:line` below is at `84fee8fa` unless it names another sha. Paths
are under `crates/script/elidex-js/src/` unless absolute.

### §0.1 Premises re-verified

- The live-frame roots are `mark_roots`'s step (b), `vm/gc/roots.rs:278-316` (`// (b) Call frame roots` at `:278`,
  loop `:279-316`). The suspended-frame roots are the `ObjectKind::Generator` arm of `trace_work_list`,
  `vm/gc/trace.rs:330-374`, frame part `:334-369`, `stack_slice` `:370-372`.
- Both enumerate `this_value`, `upvalue_ids`, `local_upvalue_ids`, `actual_args`, `new_instance`, `mode`,
  `home_class`, `pending_completion`; only `generator` differs (live `:301-303`, suspended none). Measured in §2.
- `CallFrame` lives in exactly two places: `VmInner::frames` (`vm/mod.rs:192`, read by `GcRoots`'s `frames`,
  `vm/gc/roots.rs:24`) and `SuspendedFrame`'s `frame` (`vm/coroutine_types.rs:80`). Command:
  `git grep -n -w CallFrame 84fee8fa -- crates/script/elidex-js/src` (every other hit is an import, a struct literal or
  `CallFrame::tdz_init`). So two enumeration sites are the whole population.
- `vm/gc/trace.rs` is 1256 lines; `trace_work_list`'s `match &obj.kind` spans `:246-1224` (a flat case table).

### §0.2 Premise errors and drift

- The umbrella cites `vm/gc/roots.rs:277-313` and `vm/gc/trace.rs:335-372`, and the `home_class` sites `:297` / `:357`.
  These were right at `4b723d4b`; #628 (`c0578223`) shifted both files by one line. At `84fee8fa`: `:278-316`,
  `:334-373`, `home_class` at `roots.rs:298` and `trace.rs:358`. `docs/plans/2026-07-vm-p4-slice-plan.md:951` carries
  the same stale pair (`roots.rs:297`, `trace.rs:357`). §5 says who fixes which.
- The brief's "only `generator` differs" is correct as text; whether the difference is a defect is §2 F-1 (it is not).

## §1. Scope

**In**: one free function, `mark_frame` (NEW), that marks every GC handle a `CallFrame` holds; `mark_roots` (b) and
the `Generator` arm of `trace_work_list` call it. New neutrality cells (§6). The pointer edit of §5.1.

**Out**: no field of `CallFrame` added, removed or retyped; no change to what is rooted (§1.2 and §1.3 show the two
forms below are no-ops at run time). `stack_slice` stays in the `Generator` arm (it is `SuspendedFrame`'s, not the
frame's). `ObjectKind::Function`'s `captured_this` (`vm/gc/trace.rs:256`) is R-JS's, not a frame field. P0a-1's
reaction-target arms are not touched (§5.2). `vm/gc/collect.rs` (2073 lines) is not touched.

### §1.1 D-1: location — `vm/gc/mod.rs`, beside the mark primitives

`vm/gc/mod.rs` already homes, by its own stated rule, the code "both halves of the mark phase consume"
(`vm/gc/mod.rs:29-30`: `mark_value`, `mark_object`, `mark_upvalue`, `:85-122`). `mark_frame` is exactly that: a
composite primitive called by the root walker (`roots.rs`) and the work-list tracer (`trace.rs`). It is a free function
taking split borrows like its siblings (`vm/gc/mod.rs:11-12`, "split borrow — mark bits are `&mut` while
objects/upvalues/stack/frames are `&`"):

`pub(super) fn mark_frame(frame: &CallFrame, upvalues: &[Upvalue], uv_marks: &mut [u64], obj_marks: &mut [u64], work: &mut Vec<u32>)`

Rejected: **`vm/frame.rs`** (an `impl CallFrame` method). `vm/frame.rs` is frame *state* (`:1-7`); putting mark logic
there makes the frame module depend on the GC's mark bits and work list, against the GC's one-phase-per-file layout
(`vm/gc/mod.rs:16-27`). **A new `vm/gc/frame.rs`**: a ~45-line file for one function that `mod.rs`'s existing rule
already homes. **Either caller file**: the other half would then import from a sibling phase file.

Size: `vm/gc/mod.rs` 122 → ~180 (§7).

### §1.2 D-2: `generator` — marked in `mark_frame`, for both callers (no parameter)

`mark_frame` marks `generator` like every other handle. For a suspended frame that call is a no-op:

1. `susp.frame.generator` is `None` or the generator that owns `susp`. The four store sites of a `SuspendedFrame`
   (`git grep -n -P 'suspended = Some\(|suspended: Some\(' 84fee8fa -- crates/script/elidex-js/src`, 4 hits):
   `vm/interpreter_call.rs:435` and `:686` and `vm/natives_generator.rs:555` create the generator and then back-link
   `susp.frame.generator = Some(gen_id)` to that same object (`interpreter_call.rs:446`, `:698`,
   `natives_generator.rs:565`); `vm/natives_generator.rs:78` (yield) stores the frame into the generator read from that
   frame's own `generator` (`:45-49`, `:72-78`).
2. The arm runs on an object popped from the work list, and an object is pushed only after its mark bit is set
   (`mark_object`, `vm/gc/mod.rs:96-101`). So `mark_object(owner)` finds the bit set and returns.

Rejected: **the caller keeps `generator`** — the shared function would still have to name the field (§4's exhaustive
pattern) as `generator: _`, an intentional omission whose soundness lives in a comment, and the live caller would carry
one field outside the enumeration. **A `bool` / enum parameter** — a mode flag whose only effect is skipping a no-op;
it would also be the first place a future frame-site difference gets bolted on, re-splitting the enumeration. One
enumeration with no mode is the One-issue-one-way form. If the invariant in step 1 were ever broken, D-2 roots the
foreign generator rather than leaving it unrooted: the fail-safe direction.

### §1.3 D-3: `pending_completion` — mark the value of every `FrameCompletion` variant

Both blocks today mark `Return(v)` / `Throw(v)` and skip `Normal(_)` (`roots.rs:307-315`, `trace.rs:361-369`).
`Normal(JsValue)` is never stored there: the only write is `vm/ops.rs:561` inside `route_to_next_finally`, and its
two callers pass `FrameCompletion::Return(v)` (`vm/ops.rs:500`, `vm/natives_generator.rs:243`); the type's doc says
"as a pending completion it is unused" (`vm/coroutine_types.rs:59-61`). `mark_frame` marks the value of all three
variants with one exhaustive or-pattern (a new variant fails to compile). At run time this is a no-op (no writer
produces `Normal`); it removes the one place where the type admits an unrooted `JsValue` in a frame. The alternative
(keep skipping `Normal`) is equally neutral; D-3 is chosen by the fail-safe direction (an unknown case falls on
"rooted"). **For plan-review**: if the reviewers read D-3 as outside "behaviour-neutral", keep today's arms.

## §2. Field census (at `84fee8fa`)

**Method**: list the fields from the struct (`vm/frame.rs:70-152`), then count, per field, the hits of `frame.<f>` in
the live block and of `susp.frame.<f>` in the suspended block:

```sh
S=84fee8fa; D=crates/script/elidex-js/src/vm
for f in $(git show $S:$D/frame.rs | sed -n '70,152p' | /usr/bin/grep -oE '^    pub [a-z_]+:' | sed -E 's/^    pub ([a-z_]+):/\1/'); do
  l=$(git show $S:$D/gc/roots.rs | sed -n '279,316p' | /usr/bin/grep -c -w "frame\.$f")
  s=$(git show $S:$D/gc/trace.rs | sed -n '334,373p' | /usr/bin/grep -c -w "susp\.frame\.$f")
  printf '%-20s live=%s susp=%s\n' $f $l $s; done
```

17 fields. **Control**: `this_value` → live=1 susp=1 (known-marked in both, `roots.rs:280`, `trace.rs:339`);
`generator` → live=1 susp=0 (known asymmetry, `roots.rs:301`), so the method separates the cases. GC relevance is read
from the field's **type**, not from the blocks: the GC collects `Object`s and `Upvalue`s only; strings, symbols,
shapes and compiled functions are permanent (`vm/gc/mod.rs:3-4`). `ObjectId` indexes `Vm::objects`
(`vm/value.rs:40-42`), `UpvalueId` indexes `Vm::upvalues` (`:60-62`), `FuncId` indexes `Vm::compiled_functions`
(`:50-52`).

| # | Field (`vm/frame.rs`) | Type | GC handle? | Live (`roots.rs`) | Suspended (`trace.rs`) |
|---|---|---|---|---|---|
| 1 | `func_id` `:72` | `FuncId` | no (compiled functions are permanent) | — | — |
| 2 | `ip` `:74` | `usize` | no | — | — |
| 3 | `base` `:76` | `usize` | no | — | — |
| 4 | `upvalue_ids` `:78` | `Arc<[UpvalueId]>` | **yes** | `:281-283` | `:340-342` |
| 5 | `local_upvalue_ids` `:80` | `Vec<UpvalueId>` | **yes** | `:284-286` | `:343-345` |
| 6 | `this_value` `:82` | `JsValue` | **yes** | `:280` | `:339` |
| 7 | `exception_handlers` `:84` | `Vec<HandlerEntry>` | no (`Option<u32>` ×2, `usize`; `:214-221`) | — | — |
| 8 | `tdz_bits` `:87` | `u64` | no | — | — |
| 9 | `tdz_overflow` `:90` | `Box<[u64]>` | no | — | — |
| 10 | `actual_args` `:93` | `Option<Vec<JsValue>>` | **yes** | `:287-291` | `:346-350` |
| 11 | `cleanup_base` `:96` | `usize` | no | — | — |
| 12 | `new_instance` `:100` | `Option<ObjectId>` | **yes** | `:292-294` | `:351-353` |
| 13 | `mode` `:111` | `CallMode` (`Construct { new_target: ObjectId }`, `vm/value.rs:628-640`) | **yes** | `:295-297` | `:354-357` |
| 14 | `home_class` `:121` | `Option<ObjectId>` | **yes** | `:298-300` | `:358-360` |
| 15 | `kind` `:134` | `FrameKind` (fieldless, `:58-67`) | no | — | — |
| 16 | `generator` `:138` | `Option<ObjectId>` | **yes** | `:301-303` | **none** (F-1) |
| 17 | `pending_completion` `:151` | `Option<Box<FrameCompletion>>` (`Normal`/`Return`/`Throw`, each a `JsValue`; `vm/coroutine_types.rs:66-70`) | **yes** | `:307-315` (`Return`, `Throw`) | `:361-369` (`Return`, `Throw`) |

Ten GC-relevant fields; nine marked by both blocks, one by the live block only.

### §2.1 Findings

- **F-1 (`generator`, live only). Not a defect.** The suspended frame's `generator` is `None` or its owner, which is
  already marked when its arm runs (§1.2 steps 1-2). No use-after-free; no slot. D-2 folds it into the one enumeration.
- **No field is GC-relevant and marked by neither block.**

### §2.2 Observations (sub-field level, not findings)

- **O-1** `FrameCompletion::Normal(v)` in `pending_completion` is marked by neither block; it has no writer (§1.3).
  Handled by D-3.
- **O-2** `SuspendedFrame`'s `upvalue_slots` (`vm/coroutine_types.rs:86`, `Vec<(UpvalueId, u16)>`) is not traced. It is a
  subset of `frame.local_upvalue_ids` (built from it at `vm/natives_generator.rs:53-63`; the list is preserved verbatim
  across resume, `:170-174`), which `mark_frame` marks. Sound; §4 makes the omission explicit.
- **O-3** The block comment at `trace.rs:335-338` lists four of the frame's handle fields and is already incomplete; it
  is deleted with the block.

## §3. Spec coverage map

None. The GC is engine-internal: no ECMAScript, Web IDL or web-platform algorithm is implemented or changed. The
governing sources are the repo's rules (CLAUDE.md Design discipline: 設計優先, One issue one way, 1000-line debt) and
the umbrella's §4.1 / §7 rows.

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| none — engine-internal GC | — | — | `mark_frame` (NEW, `vm/gc/mod.rs`); callers `mark_roots` (`vm/gc/roots.rs:278-316`), `trace_work_list` `Generator` arm (`vm/gc/trace.rs:330-374`) | ✓ (exhaustive `CallFrame` pattern, §4) | no |

**Breadth**: K=0, M=1.

**Preflight**: `python3 .claude/skills/elidex-plan-review/preflight.py` hard-fails this table by construction (exit 1:
"1 of 1 row(s) missing `§<number>` reference"; grep-pass 0 hard). The row has no section because there is no spec; no
citation is invented to satisfy it. Whether a spec-less memo runs the preflight with the row as is or the preflight
gains a "none" form is for the driver.

### §3.1 User-input touch audit

No new site reachable by web content. Script reaches both callers only through allocation-triggered collection; the
rooted set is unchanged (§1.2, §1.3).

## §4. Completeness mechanism

`mark_frame` opens with an **exhaustive destructuring of `CallFrame` without `..`**:

```rust
let CallFrame {
    upvalue_ids, local_upvalue_ids, this_value, actual_args, new_instance,
    mode, home_class, generator, pending_completion,
    // No GC handle in these types (permanent / plain data); see each type.
    func_id: _, ip: _, base: _, exception_handlers: _, tdz_bits: _,
    tdz_overflow: _, cleanup_base: _, kind: _,
} = frame;
```

Adding, removing or renaming a `CallFrame` field fails to compile here until the author classifies it: traced (bound
and marked) or not traced (bound to `_` in the commented group). Deliberately untraced fields are therefore named, not
absent: the omission is a reviewed line. The nested types are matched exhaustively too: `mode` by
`match mode { CallMode::Call => {}, CallMode::Construct { new_target } => … }` (today's `if let`, `roots.rs:295`, is
not exhaustive); `pending_completion` by the or-pattern of D-3. The `Generator` arm destructures `SuspendedFrame`
the same way (`let SuspendedFrame { frame, stack_slice, upvalue_slots: _ } = &**susp`, with O-2's reason on the `_`).

**Residual (stated, not closed)**: the pattern sees `CallFrame`'s own fields, not the contents of a type behind a `_`
binding. A GC handle added inside `HandlerEntry` or `FrameKind` would compile silently. Both are plain data today; the
comment on the `_` group names the types, so a reviewer of such a change sees the frame's classification depends on it.

The compiler, not a test, is the guard; the §6 cells prove today's rooted set, not future completeness.

## §5. Coupled sites

### §5.1 Umbrella 3 (`docs/plans/2026-07-vm-p4-slice-plan.md:951`, VM-P4 family)

Its obligation reads "the retirement of the two frame-marking sites" of `home_class`, citing `vm/gc/roots.rs:297` and
`vm/gc/trace.rs:357` (one line stale at `84fee8fa`, §0.2). After R-0b there is one site, inside `mark_frame`. **R-0b's
implementation PR edits** the two cites into `mark_frame` (`vm/gc/mod.rs`) and "the two frame-marking sites" into "the
frame-marking site"; nothing else in the obligation changes (it still owes a tracing edge for the closure-carried home
object). Then: `python3 .claude/tools/plan-memo-umbrella-check.py docs/plans/2026-07-vm-p4-es-language-completeness.md`
→ exit 0 (baseline at `84fee8fa`: exit 0). No link to a memo outside the family is added
(`docs/plans/test_vm_p4_family_layout.py`).

### §5.2 P0a-1 D-16 (`docs/plans/2026-11-vm-stack-safety-guard-p0a1.md`)

P0a-1 traces `ReactionTarget` / `ReactionOwner` ids at `vm/gc/roots.rs:416-428` and `vm/gc/trace.rs:284`, and cites
the `ReadableStream*Step` trace arms at `trace.rs:969-977` (memo `:516`, `:591`). Different statements; either order
(umbrella §8.1). R-0b edits nothing in P0a-1. Line drift only: removing the frame block shifts `trace.rs` lines after
`:334` (so `:969-977`, not `:284`) and `roots.rs` lines after `:278` (so `:416-428`). P0a-1 is ratified and lands
11-28, after R-0b; R-0b's PR body lists the shifted anchors for P0a-1's implementer instead of editing a ratified memo.

### §5.3 R-JS (umbrella §4.2)

R-JS changes the frame's field set (`new_instance`). After R-0b it edits one function, and §4's pattern fails to
compile until it does. The umbrella's cells that name two blocks (§2.1 A3×A9 and A5×A9, §7 `vm/gc/trace.rs`, §8.1
`mark_roots` / `trace_work_list`) are not on main; they are pointers for the umbrella's next revision, reported to
the driver.

## §6. Tests

**Existing coverage** (none isolates a single frame field as the only reference): `gc_correctness_under_stress` and
`gc_heap_bounded_in_loop` (`vm/gc/tests.rs:384`, `:337`; low `gc_threshold`, live frames under collection);
`saved_completion_stack_roots_displaced_object_through_gc` (`vm/tests/tests_frame_completion_disentanglement.rs:242`);
generator yield/resume cycles without a forced collection (`vm/tests/tests_generator.rs:281`).

**New cells** (`vm/gc/tests.rs`, 407 lines; hand-built `CallFrame`s, as `vm/tests/tests_with_call_mode.rs:116` does).
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
| `generator` | ✓ (a separate generator object) | ✓ (= the owner; asserts the owner survives and the cell does not panic) |
| `pending_completion` `Return` / `Throw` | ✓ | ✓ |
| `pending_completion` `Normal` (D-3) | ✓ | ✓ |

Plus a **negative control** per column: the same frame with the handle cleared → the object is collected (the cells
can see a collection). The cells except the `Normal` row go in a first commit that is green on the pre-refactor code;
the refactor commit keeps them green: that is the neutrality proof. The `Normal` row lands with the refactor (it is
red before D-3, by design, and documents D-3's one-line effect).

Before push: `cargo fmt --all`; `cargo test -p elidex-js --all-features`; `mise run ci`; `/pre-push`.

## §7. 1000-line verdicts (`wc -l` at `84fee8fa`; after = estimate)

| File | Now | After | Verdict |
|---|---|---|---|
| `vm/gc/trace.rs` | 1256 | ~1225 | R-0b is the real seam (umbrella §7). The remainder is `trace_work_list`'s `match` over `ObjectKind` (`:246-1224`), a flat case table: exempt. Stays over 1000. |
| `vm/gc/roots.rs` | 621 | ~586 | under 1000 |
| `vm/gc/mod.rs` | 122 | ~180 | under 1000 |
| `vm/gc/tests.rs` | 407 | ~600 | under 1000 |

## §8. Acceptance

- `mark_frame` is the only code that reads a `CallFrame`'s handles for the GC; `git grep -n -P 'frame\.(this_value|home_class|new_instance)' -- crates/script/elidex-js/src/vm/gc`
  returns no hit (`mark_frame` reads the handles through §4's bindings; control: the same command at `84fee8fa`
  returns six hits, `roots.rs:280`, `:292`, `:298`, `trace.rs:339`, `:351`, `:358`).
- §4's pattern has no `..`. §6's cells green; the negative controls collect.
- §5.1's edit made; checker exit 0; `python3 .claude/tools/ecs_mission_ratchet.py` 0 new.
