# `AbortSignal.any` — dependent-signal flattening, spec-shaped signal abort, the DOM citation family (umbrella + PR-A per-PR plan-memo)

Slot: `#11-abort-signal-any-dependent-flattening` (registered PM 454, from the VM stack-safety guard census row
W-11; ledger text PM 458; owner L3). Base: `origin/main` `69fbe61f`. Every code fact below is measured at `69fbe61f`
unless marked; line numbers move, the commands do not. Owner lane: L3 DOM/form. **This memo is both the umbrella of the
two-PR program (§1's table, §9's order) and PR-A's per-PR plan** (PM 504, Q-1 of Rev 1).

**What binds this memo.** CLAUDE.md "Design philosophy" (spec → ECS principles → evidence; the side-store→component
rule and its exception (a)), "Layering mandate" (`vm/host/` marshals; algorithms live in an engine-independent crate),
"Edge-dense work" (§1 here), "Spec citation" (every number ↔ title pair below is from `.claude/tools/webref`, run
2026-11-08). The VM stack-safety guard memos on the unlanded branch `vm-stack-safety-plan` — tip `ce603c74` (a merge
of `origin/main` `69fbe61f` into `4557c99b`; `git diff --stat 4557c99b ce603c74 -- docs/plans/2026-11-vm-stack-safety-guard.md docs/plans/2026-11-vm-stack-safety-guard-pr1.md`
→ empty, so `4557c99b` is the memos' commit; `git merge-base --is-ancestor 4557c99b origin/main` → no) — name this
slot as PR-1's one hard edge (umbrella `docs/plans/2026-11-vm-stack-safety-guard.md` §4 row W-11 at `:245` and the
"Edges" paragraph; PR-1 memo `docs/plans/2026-11-vm-stack-safety-guard-pr1.md` §2 A-1 at `:104`, §8 cell
`abort_any_chain_depth_one_precondition` at `:829`, §12 L-7 at `:968`). Rev 1 read them at `d6ce4e97`; the A-1, L-7,
W-11 and precondition-cell text is unchanged between the two commits. The command (re-run 2026-11-08; `-P`, not
`\b` — the `-E` flavour of `\b` is the silent-zero trap of memory `reference_git-grep-E-word-boundary-silent-zero`):
`R='abort_any_chain_depth_one_precondition|abort-signal-any|L-7|(?<![\w-])A-1(?![\w-])'; for P in docs/plans/2026-11-vm-stack-safety-guard.md docs/plans/2026-11-vm-stack-safety-guard-pr1.md; do diff <(git show d6ce4e97:$P | grep -P "$R") <(git show 4557c99b:$P | grep -P "$R"); done`
(positive control: `git show 4557c99b:docs/plans/2026-11-vm-stack-safety-guard-pr1.md | grep -cP "$R"` → 21; the
umbrella → 5) → only the umbrella's Edges prose and a PR-1 §1 sentence differ, neither a requirement. The same
`diff` from `4557c99b` to `b7f03821` (the guard branch's commit this memo cites for P0, below; branch tip `d7ec4761`)
leaves PR-A's requirements unchanged, at new lines: `b7f03821` A-1 is PR-1 `:82`, the precondition cell `:828`, L-7
`:952` (re-worded, same check: `git grep -n '§3\.1\.3' -- crates/script/elidex-js/src/vm/host/abort_statics.rs` → 0
lines), umbrella W-11 `:254`. What they require is listed in §9 and met in §4. PM 502 put a new slice **P0** (the
resource-limit `Err` contract + an `Err`-consumer census, owner VM P4) before PR-1; P0 is now a program — umbrella
`docs/plans/2026-11-vm-stack-safety-guard-p0.md` and its VM-core slice `docs/plans/2026-11-vm-stack-safety-guard-p0a.md`,
cited at **`b7f03821`** (`git show b7f03821:<path>`; on `vm-stack-safety-plan`, unlanded; below, `p0.md:N` /
`p0a.md:N` = line N of these two files at `b7f03821`). §4.3's `Err` contract for
`abort_signal` is written to it (D-4), and P0a edits this PR's files after it lands (§9 Edges).

**Rev 1** (2026-11-08) — first draft.
**Rev 2** (2026-11-08) — `/elidex-plan-review` R1 fixes (CRIT 0 / IMP 7 / MIN 22) and the PM 504 rulings; changelog
at the end.
**Rev 2.1** (2026-11-08) — Rev 2 attestation fixes (IMP N-1, N-2, N-3, N-4, N-7; MIN N-5, N-6, N-8, F1, F4, F5, F6,
F7, M10, M11), the driver's scope ruling on the citation family (PR-A keeps 23 lines, hands 26 to the DOM-events
renumber sweep, §9.2), the PR body's defect statement (§9.1); changelog at the end.
**Rev 2.2** (2026-11-08) — Rev 2.1 attestation fixes (PM 538: deletion or replacement only) and their attestation's
MINs; changelog at the end.
**Rev 2.3** (2026-11-09) — Codex P2: each set keeps hashed membership beside its order (D-2, D-6, §4.1).

## §0. Frame, premises, decisions

### §0.1 Frame

`AbortSignal.any(signals)` is specified as "create a dependent abort signal" (DOM §3.2): the result records the
**non-dependent** sources of its inputs (a dependent input contributes its own source signals, not itself), so no
dependent signal ever has dependent signals of its own and "signal abort" fans out exactly one level. elidex instead
links each input to the composite in a per-VM map (`VmInner::any_composite_map`) and propagates by recursion, so a
chain of `any()` calls is a chain of Rust frames (process abort) and a chain of GC-collectable intermediates (a lost
abort). The fix is the spec's data model and algorithm, placed in an engine-independent crate with the VM marshalling
to it; the same touch restructures `abort_signal` into the spec's "signal abort" / "run the abort steps" shape (which
also fixes an ordering defect found while measuring, §0.2 item 5) and re-derives the DOM citation family of the files
it edits: the `AbortSignal` lines that cite `§3.1.x` (sections that do not exist) and the two `event_target.rs`
lines that cite `§2.6.3` for the signal option's already-aborted short-circuit ("add an event listener" step 2,
§2.7). The other `EventTarget` citations of the same drift are handed to the DOM-events renumber sweep (§9.2).

### §0.2 Premises re-measured at `69fbe61f`

1. **Where the state lives.** `crates/script/elidex-js/src/vm/host/abort.rs` (733 lines): `AbortSignalState`
   `:85-113` = `aborted: bool`, `reason: JsValue`, `bound_listener_removals: HashMap<ListenerId, DispatchTarget>`;
   rows in `VmInner::abort_signal_states: HashMap<ObjectId, AbortSignalState>` (`crates/script/elidex-js/src/vm/mod.rs:1279`,
   doc `:1267-1278`), inserted by `create_abort_signal` (`impl VmInner`, `abort.rs:322-332`). The `any()` links are a
   separate per-VM map `VmInner::any_composite_map: HashMap<ObjectId, Vec<ObjectId>>` (input → composites;
   `mod.rs:1381`, doc `:1365-1380`; constructed `crates/script/elidex-js/src/vm/init.rs:815`). `ObjectKind::AbortSignal`
   is payload-free (`crates/script/elidex-js/src/vm/object_kind.rs:273-292`). No abort state is an ECS component and
   none is in an engine-independent crate: `git grep -nP 'AbortSignal|AbortController|abort_reason|source_signals|dependent_signals' -- 'crates/**/*.rs' ':!crates/script/elidex-js/**'`
   → **no output** (exit 1; positive control of the same shape without the exclusion:
   `git grep -P '\bAbortSignal\b' -- 'crates/**/*.rs' | wc -l` → 254).
2. **How `any()` links.** `native_abort_signal_static_any` (`crates/script/elidex-js/src/vm/host/abort_statics.rs:96-181`):
   reads an `ObjectKind::Array` directly (`:113-120`; the sequence conversion is the registered slots' concern, §8),
   validates every element and captures the first already-aborted reason (`:129-150`), allocates the composite
   (`:153`), then either aborts it synchronously through `abort_signal` (`:154-159`) or pushes the composite under
   **each input** in `any_composite_map` (`:169-178`) — the input itself, whether or not it is a composite. So
   `any([any([a])])` is a two-link chain; nothing flattens.
3. **How abort propagates.** `abort_signal` (`abort.rs:522-658`): latch + reason `:528-561`, back-ref prune
   `:569-571`, detach bound listeners `:580`, fire `'abort'` `:591-592`, then the composite fan-out `:607-612` with
   the recursive call at `:610` (`git grep -n 'abort_signal(ctx, composite_id, reason)' -- crates` → that one line),
   then the fetch fan-out `:627-655`. Depth = chain length; the Rust stack is the only bound.
4. **What GC keeps.** `any_composite_map` is a deliberate non-root (`crates/script/elidex-js/src/vm/gc/roots.rs:257-261`
   and note (k) `:600-618`; `crates/script/elidex-js/src/vm/gc/keepalive.rs:83-92`): the sweep tail prunes dead
   composites from each value list and drops empty keys (`crates/script/elidex-js/src/vm/gc/collect.rs:1507-1522`);
   the trace arm for a reachable signal marks only its `reason` (`crates/script/elidex-js/src/vm/gc/trace.rs:385-398`).
   An intermediate composite that script does not hold and that has no `'abort'` listener is collected, its map entry
   goes with it, and the chain is cut.
5. **Reproduced (scratch tests appended to `tests_abort_statics.rs`, run, then reverted with `git checkout`; the
   worktree is clean; `cargo test -p elidex-js --all-features --lib -- <name> --test-threads=1`, debug, 2026-11-08):**
   - `scratch_any_chain_kept_intermediates_deep` — 3000 nested `AbortSignal.any([s])` with every intermediate kept in an
     array, then `c.abort()` → **`has overflowed its stack` / SIGABRT** (the process, not the test). PR-1 memo §11
     M-AB measured the same shape: kept N = 10 / 100 / 1000 → true / true / abort (debug, 2 MiB).
   - `scratch_any_chain_dropped_intermediates_gc` — 50 nested `any()` inside an IIFE with only the last kept
     (`globalThis.s`), `vm.inner.collect_garbage()`, then `c.abort()` → **`s.aborted === false`** (half-propagated
     abort).
   - `scratch_dependent_aborted_visible_in_source_listener` — `d = AbortSignal.any([c.signal])`; inside `c.signal`'s
     `'abort'` listener read `d.aborted` → **`false`**. Spec "signal abort" step 4 sets every dependent's reason
     **before** step 5 fires the source's event (§0.4 D-4). Not in the ledger; found by reading `:591-612`.
6. **The other readers and writers of signal state** (`git grep -nE 'abort_signal_states|any_composite_map|abort_listener_back_refs|pending_timeout_signals|fetch_abort_observers|fetch_signal_back_refs' -- 'crates/**/*.rs'`,
   non-test lines, 2026-11-08): `abort.rs` (above); `abort_statics.rs:143` (aborted read), `:69` (timeout
   registration), `:174` (the map push); `crates/script/elidex-js/src/vm/host/event_target.rs:172-181` (already-aborted
   short-circuit, DOM §2.7 "add an event listener" step 2) and `:215-222` (the listener-removal abort step, step 6,
   written into `bound_listener_removals` + the reverse index); `crates/script/elidex-js/src/vm/host/fetch/mod.rs:233-240`
   (the fetch abort step, into `fetch_abort_observers` + reverse index) and `:298-305` (`pre_flight_abort_reason`);
   `crates/script/elidex-js/src/vm/host/fetch_tick.rs:343-347`, `:390-394` (settle-time removal);
   `crates/script/elidex-js/src/vm/inner.rs:67-68` (`removeEventListener` scrub); `crates/script/elidex-js/src/vm/natives_timer.rs:324-340`
   (timer fire → `internal_abort_signal`); GC: `collect.rs:1112`, `:1235`, `:1345` (trace pass-through and the
   keepalive mark), `:1442-1445` (row + back-ref prune), `:1507-1522` (the map), `:1788-1792`, `:1800-1801`, `:1813`;
   `roots.rs:100-101`; `trace.rs:32-35`, `:385-398`; `keepalive.rs:272-275` (timeout membership root). The spec's
   "abort algorithms" set is therefore split across two stores (`bound_listener_removals` per row;
   `fetch_abort_observers` per VM) and run at two points of `abort_signal` (`:580` before the fire, `:627-655`
   after it). Unifying the two stores is not this slot's (§8); their **run order** is (§0.4 D-4).
7. **GC during the abort path — per entry point.** `abort_signal` has three entry points and they do not share a GC
   regime:
   - `controller.abort()`, `AbortSignal.abort()`, `AbortSignal.any()` enter through the native-call gate, which pins
     `gc_enabled = false` for the native's duration (`crates/script/elidex-js/src/vm/interpreter.rs:360-365`, the
     `saved_gc` / `self.gc_enabled = false` pair around the native dispatch; the sibling sites `:413-418`, `:424-429`,
     `:503-514`; `git grep -n 'gc_enabled = ' -- crates/script/elidex-js/src | grep -v tests` lists every write).
     No collection runs inside them or the listeners they fire. (Rev 1 cited `dispatch.rs:856-864`; that is the
     `Arguments`-object allocation guard, not the native gate.)
   - `AbortSignal.timeout(ms)` fires from the timer drain: `VmInner::drain_timers` (`crates/script/elidex-js/src/vm/natives_timer.rs:298`)
     → `:324-345` → `internal_abort_signal` (`abort.rs:681-688`) → `abort_signal`. The drain runs at an eval boundary
     with **`gc_enabled = true`**: `init.rs:855` sets it true at construction, nothing on the drain path clears it,
     and `crates/script/elidex-js/src/vm/host/pending_tasks.rs:366` records the same fact for the sibling
     `drain_tasks` ("runs at eval boundaries with `gc_enabled = true` (unlike inside native dispatch …)"). A listener
     fired on this path allocates, so a collection can run **between two plan entries**.
   So on the timer path the plan's handles are the only thing that keeps a planned dependent alive while an earlier
   entry's listeners run: an unreferenced dependent that is planned (it was live at step 4) and then swept would have
   its `ObjectId` recycled by the listener's allocation, and `run_abort_steps` on that handle would fire `'abort'` at
   the new occupant. **Rooting the plan on the VM stack (one `VmInner::push_stack_scope` frame,
   `crates/script/elidex-js/src/vm/temp_root.rs:155`, holding every plan handle — D-4) is therefore the correctness
   guarantee of D-4, not a defensive duplicate of the native gate**; the fetch fan-out's promise root
   (`abort.rs:639-650`, a single `push_temp_root`) is the same guarantee for the same reason. The test seam exists:
   `force_gc_before_next_alloc` (`crates/script/elidex-js/src/vm/inner.rs:105`; field `mod.rs:277`) runs a collection
   at the next allocation while `gc_enabled` is true — exactly the timer-path window — and §6 cell (n) arms it from
   inside the listener, after planning.
8. **The spec.** `webref body dom interface-AbortSignal` (DOM §3.2 Interface AbortSignal, 2026-11-08), quoted where
   relied on:
   - "An AbortSignal object has a **dependent** (a boolean), which is initially false." / "associated **source
     signals** (a weak set of AbortSignal objects that the object is dependent on for its aborted state)" /
     "associated **dependent signals** (a weak set of AbortSignal objects that are dependent on the object for their
     aborted state)".
   - "The static any(signals) method steps are to return the result of creating a dependent abort signal from signals
     using AbortSignal and the current realm."
   - **create a dependent abort signal**: "1. Let resultSignal be a new object implementing signalInterface using
     realm. 2. For each signal of signals: if signal is aborted, then set resultSignal's abort reason to signal's
     abort reason and return resultSignal. 3. Set resultSignal's dependent to true. 4. For each signal of signals:
     4.1 If signal's dependent is false: 4.1.1 Append signal to resultSignal's source signals. 4.1.2 Append
     resultSignal to signal's dependent signals. 4.2 Otherwise, for each sourceSignal of signal's source signals:
     4.2.1 Assert: sourceSignal is not aborted and not dependent. 4.2.2 Append sourceSignal to resultSignal's source
     signals. 4.2.3 Append resultSignal to sourceSignal's dependent signals. 5. Return resultSignal."
   - **signal abort** (for=AbortSignal): "1. If signal is aborted, then return. 2. Set signal's abort reason to reason
     if it is given; otherwise to a new "AbortError" DOMException. 3. Let dependentSignalsToAbort be a new list.
     4. For each dependentSignal of signal's dependent signals: 4.1 If dependentSignal is not aborted: 4.1.1 Set
     dependentSignal's abort reason to signal's abort reason. 4.1.2 Append dependentSignal to
     dependentSignalsToAbort. 5. Run the abort steps for signal. 6. For each dependentSignal of
     dependentSignalsToAbort, run the abort steps for dependentSignal."
   - **run the abort steps**: "1. For each algorithm of signal's abort algorithms: run algorithm. 2. Empty signal's
     abort algorithms. 3. Fire an event named abort at signal."
   - "An AbortSignal object is **aborted** when its abort reason is not undefined."
   - "The static abort(reason) method steps are: 1. Let signal be a new AbortSignal object. 2. Set signal's abort
     reason to reason if it is given; otherwise to a new "AbortError" DOMException. 3. Return signal."
   - §3.2.1 Garbage collection (`webref body dom abort-signal-garbage-collection`): "A non-aborted dependent
     AbortSignal object must not be garbage collected while its source signals is non-empty and it has registered
     event listeners for its abort event or its abort algorithms is non-empty."
   - §3.1 Interface AbortController (`webref body dom interface-abortcontroller`): "To signal abort on an
     AbortController controller with an optional reason, signal abort on controller's signal with reason if it is
     given."
9. **The citation family.** `webref heading dom 3` (2026-11-08): §3 Aborting ongoing activities, §3.1 Interface
   AbortController, §3.2 Interface AbortSignal, §3.2.1 Garbage collection, §3.3 Using AbortController and AbortSignal
   objects in APIs. There is **no §3.1.2 and no §3.1.3** (`webref heading dom 3.1` → §3.1 only; `webref heading dom
   3.1.3` → "no headings"). The dfns (`webref dfn dom …`): 'create a dependent abort signal', 'signal abort'
   (for=AbortSignal), 'dependent', 'source signals', 'dependent signals', 'abort algorithms', 'abort reason' → all §3.2
   `#interface-AbortSignal`; 'signal abort' (for=AbortController) → §3.1. On the EventTarget side: `webref heading dom
   2.6` → §2.6 Defining event interfaces; `2.7` → §2.7 Interface EventTarget (no subsections: `heading dom 2.6.3` and
   `heading dom 2.7.7` → "no headings"); 'add an event listener' / 'remove an event listener' → §2.7.

   **Census, re-run as a sweep rather than a hand-picked pathspec** (all at `69fbe61f`, 2026-11-08; every command
   is re-runnable; `-P`, never `-E` with `\b`):
   - P1 `git grep -nP '§3\.1(\.\d+)*(?![.\d])' -- crates | grep -iP 'abort|signal'` → **23 lines, 9 files**
     (crates-wide `§3.1` / `§3.1.x` lines whose own text names abort or signal).
   - P3 `git grep -nP -B3 '§3\.1(\.\d+)*(?![.\d])' -- crates | awk 'BEGIN{RS="--\n"} /[Aa]bort/ {print}' | grep -P '§3\.1'`
     → **23 lines, the same set** (the context form: a `§3.1` line whose three preceding lines name abort).
   - P2 `git grep -nP '§3\.1(\.\d+)*(?![.\d])' -- $(git grep -lP 'AbortSignal' -- crates)` → 33 lines = P1 + 10
     non-abort subjects (Service Workers §3.1/§3.2/§3.4 in `trace.rs:540`, `structured_clone.rs:309`, `mod.rs:2428`,
     `:2458`, `:2540`, `object_kind.rs:1318`, `:1350`; UI Events §3.1 `mod.rs:1428`; Selection API §3.1
     `well_known.rs:1255`; a memo id `value.rs:672`) — read and excluded.
   - Scope control: the unfiltered `git grep -nP '§3\.1(\.\d+)*(?![.\d])' -- crates | wc -l` → **190** (Fetch,
     IndexedDB, CSS Fragmentation, SW, HTML … — this is why the DoD's "no output" check (DoD 6) is the P1 shape, not
     the bare regex).
   Of the 23: two are correct and kept — `abort.rs:1-2` ("§3.1 Interface AbortController / §3.2 Interface
   AbortSignal") and `object_kind.rs:294` (AbortController, §3.1). **21 wrong lines in 9 files**, three shapes:
   - `§3.1.3` / `§3.1.3.1` / `§3.1.3.2` / `§3.1.3.3` for the statics and `any()` (should be §3.2): `abort_statics.rs`
     `:2`, `:21`, `:38`, `:45`, `:83`, `:128`, `:161` (the ledger's seven); `abort.rs:201`, `:594`;
     `natives_timer.rs:330`; `mod.rs:1360` (inside the `any_composite_map` doc `:1358-1380`, deleted with the field);
     `crates/script/elidex-js/src/vm/tests/tests_abort_statics.rs:2`, `:246`;
     `crates/script/elidex-js/src/vm/tests/tests_fetch.rs:429` — 14.
   - `§3.1.2 step 1` for "signal abort" step 2's default reason (should be §3.2 "signal abort" step 2): `abort.rs:538`,
     `:661` — 2.
   - `§3.1` for an AbortSignal subject (should be §3.2): `abort.rs:465` (`throwIfAborted`),
     `crates/script/elidex-js/src/vm/globals.rs:719`, `mod.rs:1242` (which also cites a non-existent `§7.2`;
     `webref heading dom 7.2` → none; §7 is Sets), `object_kind.rs:273`,
     `crates/script/elidex-js/src/vm/tests/tests_ui_event_constructor.rs:357` — 5.
   **The signal-option lines, in a file PR-A edits.** `event_target.rs` (D-3 edits `:172-181`) carries the same
   non-existent `§2.6.3` for the signal option: `git grep -nP '§2\.6\.3' -- crates` → **2 lines**, `event_target.rs:110`
   and `:168` (the doc of `addEventListener` and the comment that opens D-3's block), both "WHATWG §2.6.3 step 3: an
   already-aborted signal short-circuits" = "add an event listener" step 2 (positive control for the file:
   `git grep -nP '§2\.7(?![.\d])' -- crates/script/elidex-js/src/vm/host/event_target.rs` → 3). Their subject is the
   `AbortSignal` (the listener's signal), so they are PR-A's. **PR-A's family: 23 lines in 10 files** (the 21 above in
   9 files + these 2; each count is the `wc -l` of the grep named beside it, verified 2026-11-08); the per-line
   old → new table is D-8.

   **The handed-off lines (§9.2).** The same file and its siblings carry the DOM-events chapter's
   renumber drift beyond the 2 signal-option lines of the file PR-A edits: `§2.6 step N` for add-an-event-listener /
   addEventListener steps, `§2.7.7` for removeEventListener steps, `§2.6` for the event listener's `once` /
   `passive` fields, and `§2.7.x` sub-sections that do not exist (`webref heading dom 2.7` → no subsections) — 26 lines
   in 7 files, listed with their commands in §9.2. Their owner is the DOM-events renumber sweep (memory
   `project_dom-events-section-renumber-sweep`, whose method is to "sweep ALL DOM §2.x event citations in live
   crates"); they are not PR-A's (driver ruling on the Rev 2 attestation's SCOPE item, which narrows PM 504 Q-2's
   "whole family" to the 23 lines above). Scope controls measured for that list (2026-11-08, at `69fbe61f`):
   the DOM-prefixed `§2.6` shape `git grep -nP '(DOM|WHATWG) §2\.6(?![.\d])|(DOM|WHATWG) §2\.6\.\d' -- crates` → 9
   lines in 3 files; `git grep -nP '§2\.6\.3|§2\.7\.7' -- crates` → 10 lines in 2 files; their union → **17** lines
   in 4 files (`:110` and `:168` are in both); the unprefixed `§2.6` form in the three DOM-subject files
   (`event_target.rs`, `tests_add_event_listener.rs`, `elidex-script-session/src/event_listener.rs`) → 10 + 3 + 2 =
   **15**; the bare shape `git grep -nP '§2\.6(?![.\d])|§2\.6\.\d|§2\.7\.7' -- crates` → 63 lines in **19** files, the
   other **15** files being HTML §2.6.x (form controls, select) and CSS — read and excluded.

   Correct and kept: `abort.rs:1-2`, `object_kind.rs:294`, `keepalive.rs:20`, `:84`, `:236` (§3.2 / §3.2.1),
   `event_target.rs:10`, `:44` (§2.7, no step), `:504` (HTML §8.1.8.1), the §2.9 lines. Not re-derived (section number
   resolves; the step is not an abort subject): `event_target.rs:131` "§2.7 step 8" (handleEvent) — §9.2. Premise
   correction (ledger): "seven cites, one file" is the `abort_statics.rs` subset.
10. **Existing coverage.** `tests_abort_statics.rs` (427 lines) `any_*` tests `:215-396`; `any_composite_map_is_weak_bookkeeping`
    `:343-377` asserts the **current** non-spec behaviour on the map (dropped composite → map emptied) and is rewritten
    (§6); `any_chained_composites_propagate_through_chain` `:262-277` stays green;
    `any_direct_composite_abort_does_not_touch_inputs` `:280-301` stays green (its comment describes the map's
    one direction and is rewritten for the sets, §6); `tests_fetch.rs:427-432`
    (`fetch_composite_signal_any_pre_abort_rejects`) stays green. No WPT suite covers abort:
    `ls crates/tools/elidex-wpt/src/suites/` → `cascade.rs` only; `git ls-files | grep -i wpt | grep -i abort` → no
    output (positive control: `git ls-files | grep -i abort` → the six `abort*` files of §5.1). Coverage is
    engine-independent unit tests in the new module plus VM tests (CLAUDE.md "Supported-surface testing").
11. **The keepalive seam** (`keepalive.rs:1-50`, `:57-92`): per-registrant predicates, static enum dispatch; the
    doc at `:83-92` already names "a future `AbortSignalDependent` arm … under the DOM §3.2.1 dependent-signal
    predicate (non-aborted ∧ source-signals-non-empty ∧ listenered)" as a behaviour change deferred from S5-3a, with
    the rule "adding a client is a new arm here + its registrant loop in `keepalive_survivors`, never a new divergent
    root pass". The predicate needs `dependent` and `source signals`, which exist only after this PR (→ PR-B, D-7).
12. **Crate homes.** `docs/architecture/dom.md:11-13`: `elidex-dom-api` = "Engine-independent DOM API handlers …
    No dependency on boa or any JS engine"; its `lib.rs` (132 lines) has no abort module
    (`git grep -nP 'AbortSignal|abort_signal|mod abort' -- crates/dom/elidex-dom-api/src` → no output, exit 1;
    the looser `git grep -n 'abort' -- crates/dom/elidex-dom-api/src` → 5 lines, all English prose — "aborts the
    walk", "abort the whole call" — in `class_list_tests.rs:929`, `:947`, `selection.rs:451`, `subtree_walk.rs:19`,
    `:57`; positive control: `git grep -n 'pub mod' -- crates/dom/elidex-dom-api/src/lib.rs` → 19 lines). `elidex-js`
    depends on it under the `engine` feature (`crates/script/elidex-js/Cargo.toml:36`; the `engine` list names it).
    `elidex-script-session` hosts `ListenerId` and the listener store but no abort state. The guard umbrella's
    "Edges" paragraph notes the mismatch "its home (`elidex-dom-api`) is not where the abort state lives today" — D-1
    / D-2 resolve it by a trait boundary, not by moving the state.
13. **1000-line precedent and the touch-time record.** `docs/plans/2026-07-s5-6-flip-boa-deletion.md:1130`:
    `vm/mod.rs` "2749 (already >1000) | +16 … | <50, no seam obligation" is the size reading; the cohesion reading
    CLAUDE.md asks for is per file, and `docs/plans/2026-07-vm-p4-touch-time-split-record.md:81-84` assigns the
    verdicts: `vm/object_kind.rs` → "umbrellas 6, 7 and 10, through the children each mints", `vm/gc/collect.rs` →
    "umbrella 7b", `vm/gc/trace.rs` → "umbrellas 7a and 7b"; `vm/mod.rs` is on no list, and PR-1's memo gives it "no
    touch-time seam for the range PR-1 touches" (`:394` at `4557c99b`). §7 gives PR-A's own cohesion verdicts for the
    ranges it touches.

### §0.3 Premise checks against the brief and the ledger

- "unbounded chain … walk is unbounded (stack)" — **true** (item 3, item 5).
- "with intermediates not retained, GC can collect them so `s.aborted` stays false" — **true** (items 4, 5); the
  retention rule it violates is not §3.2.1 (an unlistened intermediate may be collected under the spec) but the
  linkage: the spec never routes an abort *through* an intermediate.
- "Fix = spec flattening (depth 1 by construction)" — **true**; stated in §4.2 from the quoted steps.
- "algorithm in an engine-independent crate" — the home is decided in D-1; the state stays in the VM (D-2), which the
  umbrella flagged as the design question.
- "all seven `§3.1.3`-family cites in `abort_statics.rs`" — **the `§3.1.x` family is 21 wrong lines in 9 files; with
  the two signal-option lines of `event_target.rs` (`§2.6.3`) PR-A's family is 23 lines in 10 files** (item 9, census
  re-run 2026-11-08). The other lines of the same drift (26 lines, 7 files) are handed to the DOM-events
  renumber sweep (§9.2; Rev 2's "44 lines / 13 files, all in PR-A" was narrowed on the Rev 2 attestation).
- "webref places 'create a dependent abort signal' at DOM §3.2" — **true**.
- Not in the brief, found by measurement: the "signal abort" step-4-before-step-5 ordering (item 5, third repro) and
  the abort-algorithms run order (item 6).

### §0.4 Decisions (this program and PR-A)

- **D-1 Home = `crates/dom/elidex-dom-api/src/abort.rs` (NEW), `pub mod abort`.** Grounds: DOM §3 is the DOM
  Standard; `elidex-dom-api` is chartered as the engine-independent DOM-algorithm crate (item 12) and is already an
  `engine`-feature dependency of `elidex-js`; the umbrella (PM 454) named "the `elidex-dom-api` side". Rejected:
  `elidex-script-session` (the Script↔ECS boundary; no entity is involved here) and a new crate (a ~250-line module
  does not justify one; CLAUDE.md "新しい型を足す前に既存の抽象で解決できないか"). The module depends on nothing in
  the crate (no `World`, no `EcsDom`): it is generic over a handle type.
- **D-2 Representation: the spec record, generic over the handle; the VM keeps the rows.** The module defines
  `DependentLinks<H>` (NEW) = `{ dependent: bool, source_signals: OrderedSet<H>, dependent_signals: OrderedSet<H> }` — **all three
  fields private**, read through `is_dependent()` / `source_signals()` / `dependent_signals()`, written only by the
  module's algorithms (so a host cannot set `dependent` or push a handle; DoD 3 checks the reads, the compiler the
  writes) — ordered and deduplicated (an Infra set) — and a trait `AbortSignalGraph` (NEW) with associated
  `Handle: Copy + Eq + Hash` and `Reason: Copy`, methods `abort_reason(h) -> Option<Reason>` (`None` = not aborted;
  the spec's "aborted ⇔ reason not undefined"), `set_abort_reason(h, r)`, `links(h) -> Option<&DependentLinks<H>>`,
  `links_mut(h)`. `links(h) == None` means **no such signal** (contract in §4.1). `VmInner` implements it over
  `abort_signal_states` (`Handle = ObjectId`, `Reason = JsValue`); `AbortSignalState` gains a
  `links: DependentLinks<ObjectId>` field; **`any_composite_map` is deleted** (field, constructor, GC prune, docs,
  tests: `git grep -l 'any_composite_map' -- crates` → 8 files, 21 lines at `69fbe61f`). **Why a trait with one
  production impl rather than a module-owned store.** The alternative — the module owns
  `HashMap<H, (Option<R>, DependentLinks<H>)>` and the VM calls into it — splits one signal's row across two stores
  (reason + links in the module, `bound_listener_removals` in the VM row), gives the sweep two prune sites keyed by
  the same `ObjectId`, and re-creates the `ObjectId`-keyed side-store this decision deletes. The trait keeps one row
  per signal where the row already is, and moves only the algorithm; the cost is four one-line methods. The trait
  has two impls in the tree anyway (the VM, and the module's `HashMap` test graph, §4.1), so the test graph is not a
  mock of a one-impl abstraction but the engine-independent reading of the same contract. Side-store→component rule:
  the state is keyed by a VM `ObjectId` and the sets hold `ObjectId`s — per-VM identity handles, exception (a); there
  is no entity to carry a component (an `AbortSignal` is a VM object by the EventTarget dispatch core's `VmObject`
  home, `abort.rs:27-44`), so the per-VM row is the correct home today, as `keepalive.rs:29-33` records for the
  same class of object.
- **D-3 `aborted: bool` + `reason: JsValue` → `reason: Option<JsValue>`; `aborted()` is derived.** Grounds: the spec
  defines aborted as "abort reason is not undefined", and step 2 of both "signal abort" and the static `abort()`
  materialises `undefined` into an `AbortError` `DOMException`, so an aborted signal's reason is never `undefined`;
  two fields carried one fact under a convention. Every reader of the two fields is in this PR's edit set:
  `git grep -nP '(?<![A-Za-z_])(s|state)\.(aborted|reason)\b' -- crates/script/elidex-js/src` → 20 lines at
  `69fbe61f`, of which 13 are Rust field reads/writes — `gc/trace.rs:387`; `abort.rs:443`, `:457`, `:475-476`,
  `:532`, `:558-559`; `abort_statics.rs:144-145`; `event_target.rs:178`; `fetch/mod.rs:300-301` — 3 are comments
  (`abort.rs:604`, `fetch/mod.rs:295`, `tests_abort_statics.rs:77`) and 4 are JS source text inside test strings
  (`tests_abort.rs:735`, `tests_abort_statics.rs:47`, `:81`, `:84`), not field access. (Rev 1's command used bare pathspecs and `\b`
  under `-E`; run at `69fbe61f` it prints nothing — a silent zero, not a measurement.) The `reason` getter maps
  `None` → `Undefined`.
- **D-4 `abort_signal` becomes the spec's two algorithms, iterative.** `elidex_dom_api::abort::signal_abort` (NEW) `(graph, signal, reason) -> Option<AbortPlan<H>>`
  performs steps 1-4 (returns `None` when already aborted; otherwise sets the signal's and each non-aborted
  dependent's reason and returns `AbortPlan { signal, dependents }` (NEW), the ordered list of steps 5-6). The VM's
  `abort_signal` then: materialises the default reason first (step 2's "otherwise" branch is engine-bound — a
  `DOMException` object), calls `signal_abort`, **roots every handle of the plan on the VM stack for the rest of the
  call** — one `push_stack_scope()` frame (`temp_root.rs:155`) onto whose `stack` it pushes `JsValue::Object(h)` for
  `plan.signal` and each dependent, with the loop running on `NativeContext::new_call(&mut frame)` (the shape of
  `internal_abort_signal`, `abort.rs:681-688`, and of the event-payload root at
  `crates/script/elidex-js/src/vm/host/event_target_dispatch_vm.rs:446`). Not N `push_temp_root` guards: each
  `VmTempRoot` holds the `&mut VmInner` (`temp_root.rs:41`, struct `:76-80`), so a second guard cannot be taken while
  the first lives, and the type's own doc names `push_stack_scope` as the form for "an arbitrary number of values"
  (`temp_root.rs:131-137`). On the timer entry point this rooting is the correctness guarantee (item 7:
  `gc_enabled` is true there, a listener's allocation can sweep an unreferenced planned dependent, and a recycled
  `ObjectId` must never receive another signal's abort steps); on the native entry points the gate already prevents
  the sweep and the root is inert — and runs `run_abort_steps(ctx, h)` (NEW) for the signal and then for each
  dependent — a loop, no recursion. `run_abort_steps` is "run the abort steps": the two algorithm stores first (detach bound listeners;
  reject and cancel the fetches), then fire `'abort'`. Consequences: (i) depth 1 by construction (§4.2); (ii)
  `d.aborted` is `true` inside the source's listeners (item 5, third repro — the third defect, in PR-A per PM 504);
  (iii) the fetch rejection moves from after the fire to before it — the spec's order; script cannot observe the
  difference (promise reactions are microtasks, `cancel_fetch` is a broker message), the internal order is stated
  and tested on the plan, not on timing. `internal_abort_signal` and `native_abort_controller_abort` keep their
  signatures (callers unchanged).
  **The `Err` contract after the plan is committed.** `run_abort_steps` has one `Result`-typed callee:
  `dispatch_vm_simple_event` (`crates/script/elidex-js/src/vm/host/event_target_dispatch_vm.rs:653`,
  `-> Result<bool, VmError>`); `detach_bound_listeners` and `reject_promise_sync` return `()`. **At `69fbe61f` that
  callee never returns `Err`**: it is `fire_vm_event` (`:392`) → `fire_vm_event_unchecked` (`:429`) →
  `dispatch_vm_event` (`:271-347`, no `?` in its body, its one return is `Ok`), and a listener's exception is
  swallowed and recorded, not propagated (`crates/script/elidex-js/src/vm/host/event_target_dispatch.rs:485-497`:
  `call_value(..).is_err()` → `threw = true`; DOM §2.9 inner invoke reports the exception). So the `Err` path is
  **latent**: no input reaches it today, and it becomes reachable when PM 502's slice P0 lets its resource-limit
  error surface through dispatch (a check or probe on the dispatch path — P0a §5.4 at `b7f03821`, the
  "`AbortSignal` abort steps" row, `p0a.md:292-315`, classifies the shape as "latent definitive" on the same
  reading). The contract below is written for that error; it is not a claim that some `Err` reaches the loop
  today. (Rev 2 said a listener-time resource limit or internal fault "reaches `abort_signal` as `Err`"; at
  `69fbe61f` none does.) The spec has no failure path here: "signal abort" step 5 runs the abort steps for the signal and step 6 runs them "for
  each dependentSignal of dependentSignalsToAbort" — unconditionally — and step 4 has already committed every
  dependent's reason, so a dependent whose abort steps do not run is latched aborted for ever (step 1 returns on
  every later call) with its algorithms never run and its event never fired: silent, permanent, and exactly the
  shape M4-12's policy (c) ("本番で黙って誤る slot を 0 に") names. Today's code has the same hole in its own shape,
  latent in the same sense: `:592` would return after the latch and `:610` after `any_composite_map.remove`, and
  the rest of the fan-out would not run (§9.1 states it for the PR body). **Contract (PR-A):
  `abort_signal` runs `run_abort_steps` for every plan entry regardless of earlier results, keeps the first `Err`,
  and returns it after the loop.** Grounds: the spec's steps 5-6 are unconditional; the only fact an `Err` may add
  is "the caller sees the failure", never "some dependents were not aborted"; and P0's contract is honoured in the
  direction P0 states it — an `Err` does not create a divergent committed fact (the committed facts, the reasons, are
  the spec's own step 4 and precede the `Err` by design). The alternative "keep `?`, since no `Err` reaches it
  today" is rejected: P0's purpose is to make a resource-limit error reach exactly this kind of call, `VmError` is
  open-ended, and a `?` inside the loop would turn P0's landing into the silent permanent defect above. P0a §5.4
  (`b7f03821`, `p0a.md:292-315`) already records this contract as L3's ("run all, keep the first, return it after the
  loop, with no `?` inside the loop") and classes the post-PR-A call **P** (deferred propagation) if its tracked
  check lists it. Hand-off: PR-A's PR body gives the post-PR-A names P0a needs (`abort_signal` →
  `run_abort_steps` → `dispatch_vm_simple_event`, and cell (o)'s name — P0a §10 item 4, `p0a.md:434-436`), §9.1.
- **D-5 The reason-only paths set the reason only.** "create a dependent abort signal" step 2 and the static
  `abort(reason)` step 2 set the result's reason and return; today both call `abort_signal` on the fresh signal
  (`abort_statics.rs:39`, `:159`), which fires `'abort'` at an object that cannot yet have listeners or algorithms
  (unobservable). PR-A sets the reason through the graph and does not run abort steps; the result of step 2 is **not**
  dependent (`dependent` stays false, `source_signals` empty — step 3 is not reached), which §3.2.1 (PR-B) relies on.
- **D-6 Sets are ordered and deduplicated; a signal's `dependent_signals` is emptied after planning.** "Append" to an
  Infra set is a no-op for a present member (membership is a hashed lookup, so an append stays O(1) like today's
  `push`), so `any([a, a])` records one source and one dependent (today the map
  holds two entries and the latch hides it, `tests_abort_statics.rs:304-321`). After `signal_abort` has planned a
  signal's dependents they are all aborted (step 4) and the set is never read again (step 1 returns on the next
  call), so it is emptied — the weak set has no observable content, and this is today's `remove(&signal_id)`
  (`abort.rs:607`) in the new shape. `source_signals` is left as the spec has it (PR-B's predicate reads it).
- **D-7 Program of two PRs; this memo is the umbrella and PR-A's plan (PM 504 Q-1).** §1 verdict: edge-dense.
  **PR-A** (this memo): D-1 – D-6, D-8, D-9, the tests of §6. **PR-B** = slot **`#11-abort-signal-dependent-keepalive`**
  (**registered, PM 512**, owner L3 — ledger memory `project_open-defer-slots.md:1360`; §8 has the row text with
  trigger and re-eval date; own per-PR memo, after PR-A lands): the §3.2.1 keepalive arm `KeepaliveClass::AbortSignalDependent` (NEW) in PR-B on the existing
  seam (item 11), its registrant loop over `abort_signal_states` rows, the listener half through
  `vm_path_has_listener(vm, id, "abort", false)` (`crates/script/elidex-js/src/vm/host/event_target_dispatch_vm.rs:79-84`)
  and the "abort algorithms is non-empty" half over the two stores; its engine-independent predicate function in the
  same `abort.rs` module (the `elidex_api_ws::ws_keepalive(state, has_listener)` shape,
  `crates/api/elidex-api-ws/src/websocket.rs:162`). **The predicate is §3.2.1 in full** (`webref body dom
  abort-signal-garbage-collection`: "A non-aborted dependent AbortSignal object must not be garbage collected while
  its source signals is non-empty and it has registered event listeners for its abort event or its abort algorithms
  is non-empty"): keep ⇔ **not aborted ∧ dependent ∧ source signals non-empty ∧ (an `abort` listener is registered ∨
  abort algorithms non-empty)**. PR-B is a behaviour change in the leak direction: the DOM Standard states no
  keepalive requirement for a listenered signal other than §3.2.1 and the `timeout()` note (§3.2), so an arm that
  kept every listenered signal would root what the spec lets go — this is an inference from the absence of such a
  requirement in §2.7 / §2.8, not a citation of one — which is why the arm gates on the full predicate, why it is its
  own reviewed unit, and why PR-A does not partially implement it. **Code docs PR-A writes name the slot id, never
  "PR-B"** (`keepalive.rs:83-92`, `roots.rs` note (k), the §8 pointer). The parent slot
  `#11-abort-signal-any-dependent-flattening` closes at PR-A's landing (its defects are PR-A's; PR-1's hard edge is
  on PR-A); the condition Rev 2 set — "only once `#11-abort-signal-dependent-keepalive` is registered", so the
  §3.2.1 defect is never left without a slot — is met (PM 512).
- **D-8 The `AbortSignal` citation family, in PR-A (PM 504 Q-2, narrowed to the `AbortSignal` subject on the Rev 2
  attestation — §0.2 item 9, §9.2).** All 23 lines of item 9 are re-derived, section **and** step, from `webref` (2026-11-08: `heading dom 2.7`, `heading dom 3`, `heading html 8.1.8.1`, `heading fetch 5`,
  `body dom interface-AbortSignal`, `body dom add-an-event-listener`, `body fetch fetch-method`). Quotes below are
  verbatim from `webref body`. Code comments name the algorithm and the step, not a bare `§3.2 step N`. Lines inside
  blocks PR-A deletes or rewrites are marked; the rest are one-line comment edits.

  *Sub-family 1 — DOM §3 (21 lines, 9 files).* Target section for every AbortSignal subject: **§3.2 Interface
  AbortSignal** (`#interface-AbortSignal`).

  | Line | Old | New (section · algorithm · step) | Step change |
  |---|---|---|---|
  | `abort_statics.rs:2` | `(WHATWG DOM §3.1.3)` | DOM §3.2 — the static `abort(reason)` / `timeout(milliseconds)` / `any(signals)` method steps | — |
  | `abort_statics.rs:21` | `(WHATWG §3.1.3.1)` | §3.2 "The static abort(reason) method steps" | — |
  | `abort_statics.rs:38` | `matching WHATWG §3.1.3.1 step 2` | §3.2 static abort(reason) method steps step 2: "Set signal's abort reason to reason if it is given; otherwise to a new "AbortError" DOMException." | 2 → 2 |
  | `abort_statics.rs:45` | `(WHATWG §3.1.3.2)` | §3.2 "The static timeout(milliseconds) method steps" | — |
  | `abort_statics.rs:83` | `(WHATWG §3.1.3.3)` | §3.2 "The static any(signals) method steps are to return the result of creating a dependent abort signal from signals using AbortSignal and the current realm." (block rewritten, §4.3) | — |
  | `abort_statics.rs:128` | `(WHATWG §3.1.3.3 step 2)` | §3.2 create a dependent abort signal step 2: "For each signal of signals: if signal is aborted, then set resultSignal's abort reason to signal's abort reason and return resultSignal." (block rewritten — the capture moves into the algorithm, D-5) | 2 → 2 |
  | `abort_statics.rs:161` | `Multi-input propagation (WHATWG §3.1.3.3)` | §3.2 create a dependent abort signal step 4 (4.1 "Append signal to resultSignal's source signals. … Append resultSignal to signal's dependent signals."; 4.2 the source-signals form) (block deleted with the map push) | — → 4 |
  | `abort.rs:201` | `(WHATWG §3.1.3)` | DOM §3.2 — the three static method steps | — |
  | `abort.rs:465` | `WHATWG §3.1: if aborted, throw the stored reason` | §3.2 "The throwIfAborted() method steps are to throw this's abort reason, if this is aborted." | — |
  | `abort.rs:538` | `(WHATWG DOM §3.1.2 step 1)` | §3.2 "signal abort" step 2: "Set signal's abort reason to reason if it is given; otherwise to a new "AbortError" DOMException." | **1 → 2** |
  | `abort.rs:594` | `AbortSignal.any(inputs) fan-out (WHATWG §3.1.3.3)` | block deleted (the fan-out is replaced by the plan loop; the new comment cites "signal abort" steps 3-6 and "run the abort steps" 1-3) | — |
  | `abort.rs:661` | `WHATWG DOM §3.1.2 step 1: "set reason to a new 'AbortError' DOMException"` | §3.2 "signal abort" step 2 **and** the static abort(reason) method steps step 2 — the sentence is identical in both: "Set signal's abort reason to reason if it is given; otherwise to a new "AbortError" DOMException." (the docstring names both callers) | **1 → 2** |
  | `natives_timer.rs:330` | `(WHATWG §3.1.3.2)` | §3.2 static timeout(milliseconds) method steps step 3.1: "Queue a global task on the timer task source given global to signal abort given signal and a new "TimeoutError" DOMException." | — → 3.1 |
  | `mod.rs:1242` | `WHATWG DOM §3.1 / §7.2 — AbortSignal is an EventTarget but not a Node` | §3.2 (IDL `interface AbortSignal : EventTarget`); `§7.2` dropped (no such section; §7 is Sets) | — |
  | `mod.rs:1360` | `(WHATWG DOM §3.1.3.3)` in the `any_composite_map` doc | deleted with the field (`:1358-1381`) | — |
  | `object_kind.rs:273` | `(WHATWG DOM §3.1)` | §3.2 Interface AbortSignal | — |
  | `globals.rs:719` | `(WHATWG DOM §3.1)` for "AbortController constructor + AbortSignal global + AbortSignal.prototype" | §3.1 Interface AbortController / §3.2 Interface AbortSignal (mixed subject) | — |
  | `tests_abort_statics.rs:2` | `(WHATWG DOM §3.1.3)` | DOM §3.2 — the static method steps | — |
  | `tests_abort_statics.rs:246` | `WHATWG §3.1.3.3 step 2 already-aborted fast path` | §3.2 create a dependent abort signal step 2 (quote as at `:128`) | 2 → 2 |
  | `tests_fetch.rs:429` | `WHATWG §3.1.3.3 step 3: "if any input is already aborted, return a newly-created signal aborted with its reason"` | §3.2 create a dependent abort signal step 2: "For each signal of signals: if signal is aborted, then set resultSignal's abort reason to signal's abort reason and return resultSignal." (the old line quoted a paraphrase as spec text) | **3 → 2** |
  | `tests_ui_event_constructor.rs:357` | `WHATWG DOM §3.1: AbortSignal is an EventTarget (not a Node)` | §3.2 (IDL `interface AbortSignal : EventTarget`) | — |

  *Sub-family 2 — the signal option in `event_target.rs` (2 lines).* Target: **§2.7 Interface EventTarget**, "add an
  event listener" (`#add-an-event-listener`; `webref body dom add-an-event-listener`) step 2: "If listener's signal is
  non-null and is aborted, then return." (Step 1 is the ServiceWorkerGlobalScope warning, so the short-circuit is
  step 2, not 3.)

  | Line | Old | New | Step change |
  |---|---|---|---|
  | `event_target.rs:110` | `WHATWG §2.6.3 step 3: an already-aborted signal short-circuits` | §2.7 add an event listener step 2: "If listener's signal is non-null and is aborted, then return." | **3 → 2** |
  | `event_target.rs:168` | `WHATWG §2.6.3 step 3` (same as `:110`; the comment that opens D-3's block) | §2.7 add an event listener step 2 (block rewritten, D-3) | **3 → 2** |

  The other EventTarget lines of the same files (21) and five siblings elsewhere are not PR-A's: §9.2 lists them,
  with Rev 2's per-line derivation for the 21 kept there as input to the sweep that owns them.

  *Other citations in the blocks PR-A rewrites* (not § drift of the DOM, found by reading the blocks):
  `abort.rs:615-616` "WHATWG Fetch §5.1 step 13: if abort signal is aborted, set request's done flag" — `webref
  heading fetch 5.1` → §5.1 **Headers class**; the fetch(input, init) method steps are **§5.6 Fetch methods**
  (`#fetch-method`), and the step this block implements is **step 11**: "Add the following abort steps to
  requestObject's signal: … 4. Abort the fetch() call with p, request, responseObject, and requestObject's signal's
  abort reason." (`webref dfn fetch 'Abort the fetch() call'` → §5.6 `#abort-fetch`; the "done flag" wording is not
  in the current text) — rewritten where the block moves into `run_abort_steps`. `abort.rs:499`, `:585` "WHATWG
  §8.1.8.1" (registration-order run of `onabort` + listeners) — the spec is unnamed; `webref heading html 8.1.8.1` →
  **HTML §8.1.8.1 Event handlers**; both lines gain the name (as `event_target.rs:504` already has it).
  PR-1's L-7 check (`git grep -n '§3\.1\.3' -- crates/script/elidex-js/src/vm/host/abort_statics.rs` → 0 lines) is a
  subset of DoD 6.
  **Hygiene in the same touch.** `abort.rs:512`'s closing clause grounds the post-abort visibility of `onabort` in
  an appeal to precedent instead of the spec; the line is rewritten on spec grounds [spec]: the spec removes no
  listener at abort and fires once because "signal abort" step 1 returns on a second call. The repository's ratchet
  does **not** count this line today: `python3 .claude/tools/ecs_mission_ratchet.py` → `ok (501 allowlisted, 0 new,
  0 stale)`, `git grep -n 'host/abort' -- .claude/tools/ecs-mission-allowlist.tsv` → no output (positive control:
  `git grep -c 'host/' -- .claude/tools/ecs-mission-allowlist.tsv` → 122), and the classifier's `GENERIC` phrase list
  (`.claude/tools/ecs_mission_ratchet.py:41-43`, seven phrases) does not contain the phrase at `:512`. So the
  allowlisted count is unchanged by the rewrite (DoD 7 is `0 new / 0 stale`, not "one lower"); the phrase is of the
  class the vocabulary gap slot `#11-ecs-mission-detector-vocabulary` (PM 482) tracks [history], and PR-A removes the
  line on its own grounds, not because a detector fired.
- **D-9 Tests.** The module's own unit tests over a `HashMap`-backed test graph (algorithm cells, no VM); VM cells in
  a new file `crates/script/elidex-js/src/vm/tests/tests_abort_dependent.rs` (NEW) (the graph and GC cells of §6;
  `tests_abort.rs` is at 950 and is not touched, so it does not cross 1000); the existing `any_*` tests in
  `tests_abort_statics.rs` rewritten in place where they assert the map (§6 lists each).
- **D-10 Not PR-A's.** The `any()` input conversion (Array-only read, `abort_statics.rs:102-120`) stays with
  `#11-webidl-sequence-dense-array-fast-path` / `#11-vm-webidl-sequence-early-exit-drain` (§8); the two abort-algorithm
  stores stay two (§8 candidate slot); `AbortSignal.timeout`'s keepalive membership (`keepalive.rs:272-275`) and the
  timer path are unchanged except the citation.

## §1. Shape and verdict

**Edge-dense**, by CLAUDE.md's first disjunct: §2 enumerates four intersecting invariant axes (handle identity, GC
weak sets, propagation order and re-entry, the crate boundary). The second disjunct does not apply — the spec gives
the canonical algorithm, step by step (§0.2 item 8), which is why the propagation axis is mostly settled by quotation.
Per the rule the work is a program, not one PR:

| PR | Content | Depends on | Lands before |
|---|---|---|---|
| **PR-A** (this memo) | D-1 – D-6, D-8, D-9: the module, the trait, flattening, the iterative spec-shaped `abort_signal` (with the step-4-before-5 ordering fix — the third defect, PM 504), `any_composite_map` deleted, the 23-line `AbortSignal` citation family, hygiene; tests of §6 | — | VM stack-safety guard PR-1 (hard edge, §9) |
| **PR-B** = slot `#11-abort-signal-dependent-keepalive` (own memo) | DOM §3.2.1: `KeepaliveClass::AbortSignalDependent` (NEW) in PR-B + the engine-independent predicate; the GC cell "listenered unreferenced dependent survives and fires" | PR-A (`dependent`, `source_signals`) | — (no consumer waits) |

PR-A is the terminal unit of this program (base case: a per-PR slice under the umbrella table, through
`/elidex-plan-review` before implementation). Coverage breadth K = 1 spec, M = 17 rows (§3): single-PR scope for PR-A.

**Why PR-A is not cut further (the base-case claim, reasoned).** The review axis asks that a terminal slice be
"narrowed to a single invariant-axis intersection"; PR-A carries the four axes of §2 and D-3 / D-4 / D-5 / D-8. The
candidate cuts and why each fails:

- **D-8 alone (a citations-only PR first).** 9 of the 23 lines sit inside blocks PR-A deletes or rewrites for a
  structural reason (enumerated by reading each block at `69fbe61f`): `abort.rs:538`, `:594` (the `abort_signal`
  body `:522-658`, D-4); `abort_statics.rs:2` (module doc `:1-12`, rewritten §4.3), `:38` (static `abort()` body
  `:30-41`, D-5), `:83` (the `any()` doc `:82-95`), `:128`, `:161` (the `any()` body `:96-181`); `mod.rs:1360` (the
  `any_composite_map` doc, deleted with the field); `event_target.rs:168` (the comment opening D-3's block
  `:168-181`). A citations PR would write new text on those 9 that the next PR deletes or rewrites again, and the
  other 14 would then be a second touch of the same family, which PM 458 / PM 504 Q-2 ("one family, one touch")
  rule out. (Rev 2 said "21 of the 44"; that count was not an enumeration.)
- **D-3 alone (`reason: Option<JsValue>` first).** Its 13 field readers (D-3's grep) are `abort_signal`'s latch
  (`abort.rs:532`, `:558-559` — rewritten by D-4), `any()`'s first-aborted capture (`abort_statics.rs:144-145` —
  removed by D-5), the getters, `event_target.rs:178`, `fetch/mod.rs:300-301`, `trace.rs:387`. A D-3-only PR
  rewrites the latch and the capture; PR-A′ rewrites both again. Nothing reads `Option` in between, so the cut
  edits the same 5 latch / capture lines (`abort.rs:532`, `:558`, `:559`, `abort_statics.rs:144`, `:145`) twice
  for no reader's benefit. (Rev 2 called this "a strangler in time"; it is not a strangler — no two mechanisms
  coexist — only a double edit.)
- **D-4 alone (the iterative two-algorithm shape first).** The plan's dependents come from `dependent_signals`, which
  does not exist without D-2. An iterative `abort_signal` over `any_composite_map` would be a worklist over the chain
  — a traversal of the data model the spec says must not exist — fixing the stack defect and leaving the GC-cut chain
  (item 5, second repro) and the ordering (third repro, since step 4 needs the dependents set) for a later PR that
  deletes the worklist. That intermediate state is "new seam + the legacy map", the strangler CLAUDE.md's "One
  issue, one way" names.
- **Flattening alone (D-1 / D-2 + `any()` on the algorithm; keep the recursive `abort_signal`).** Depth 1 by
  construction makes the recursion at most one level deep, so the stack defect goes; but `abort_signal` would still
  recurse into the dependents through the set with the fire before the dependents' reasons are set (third defect),
  and the second PR would rewrite the same function. Two rewrites of `abort_signal` for one spec algorithm.
- **The GC prune change alone** has no subject without the sets.

Every cut leaves either a commit that implements the wrong data model or a function rewritten twice; the four axes of
§2 are properties of one algorithm pair ("create a dependent abort signal" / "signal abort") that the spec defines as
a unit, which is why the intersection cannot be narrowed below it. The one cut that *does* separate — §3.2.1's
keepalive arm, a behaviour change in the leak direction with its own predicate — is PR-B.

Not in PR-A: §3.2.1 (PR-B, `#11-abort-signal-dependent-keepalive`); the sequence conversion of `any()`'s argument
(§8); the unification of the two abort-algorithm stores (`#11-abort-algorithms-single-set`, PM 504, §8); any ECS
component (no entity exists for a signal, D-2).

## §2. Coupled invariants

Axes: **A** handle identity (an `ObjectId` is recycled by the free list after a sweep; a set or plan holding a dead
handle must never be read as the new occupant), **B** GC weak sets (source / dependent signals root nothing; dead
members are pruned at the sweep; nothing new is traced), **C** propagation order and re-entry (steps 3-6 in the spec's
order; listeners run JS that may abort other controllers, add listeners, call `any()`), **D** the crate boundary (the
algorithm reads and writes only reasons and links through the trait; `DOMException` creation, listener detach, fetch
rejection and the event fire stay in `vm/host/`).

- **A**: links hold handles only; the sweep tail prunes dead handles from every surviving row's two sets in the same
  pass that prunes dead rows (`collect.rs:1442-1443` today prunes rows; the set prune replaces `:1507-1522`), so no
  set holds a dead handle after a sweep; a plan's handles are rooted on the VM stack (one `push_stack_scope` frame) for the duration of the abort call (D-4).
- **B**: `DependentLinks::retain_live(|h| alive(h))` (NEW) is the one prune rule, called from the sweep for both sets;
  `trace_work_list`'s `AbortSignal` arm marks the reason and nothing in the links (the sets are weak by spec); the
  §3.2.1 root is PR-B's and is the only root that will ever read the links.
- **C**: `signal_abort` sets every reason (the signal's, each non-aborted dependent's) **before** any JS runs; the
  plan is fixed at that point. A listener that aborts another source `b` of a planned dependent `d` finds `d` aborted
  (step 4.1 skips it; `d`'s abort steps run once, from the first plan). A listener that calls `any([signal, …])`
  takes step 2 (the signal is aborted) and creates a non-dependent, already-aborted result — it is not appended to the
  signal's (already emptied, D-6) dependent set. A listener that adds an `'abort'` listener to a planned dependent
  sees it fire in step 6 (the dependent is aborted but its abort steps have not run — the spec's order).
- **D**: the module exports `DependentLinks`, `AbortSignalGraph`, `DependentCreation { AlreadyAborted(R), Linked }`
  (NEW), `create_dependent_abort_signal`, `AbortPlan`, `signal_abort`; it names no `ObjectId`, `JsValue`, `Entity`,
  listener or fetch type. `vm/host/abort.rs` keeps: the `DOMException` materialisation, `run_abort_steps`, the two
  algorithm stores, `dispatch_vm_simple_event`; `abort_statics.rs` keeps the brand checks and the Array read.

| Pair | Intersection |
|---|---|
| A × B | a dead handle in a set lives from the object's death to the same sweep's prune — no read happens in between (collection and prune are one `collect_garbage` call) — so a recycled slot is never seen through a set; the sweep prunes sets **after** the row prune, from the same mark bits |
| A × C | the plan's handles are rooted before `run_abort_steps` runs JS. On the timer entry point (`gc_enabled = true`, §0.2 item 7) this is the guarantee: a listener's allocation can sweep an unreferenced planned dependent, and without the root its recycled `ObjectId` would receive the abort steps (§6 cell (n) is red without the root); on the native entry points the gate makes the root inert, as `abort.rs:639-650`'s promise root is there |
| B × C | a dependent collected between two aborts is simply absent from the planning loop (pruned); a dependent that dies with a listener attached is PR-B's defect, unchanged by PR-A (today's behaviour, §0.2 item 4) |
| A × D | the trait's `Handle` is `Copy + Eq + Hash` and nothing more; recycling is a VM fact the module cannot see — which is why the prune and the rooting are VM-side, named in A and B, and tested in the VM (§6 e, h) |
| C × D | the module returns the plan and sets reasons; it runs no step that can run JS. The line is "the algorithm owns reason and links; the VM owns everything that fires" — the same line the spec draws between "signal abort" steps 1-4 and "run the abort steps" |
| B × D | `retain_live` is engine-independent (a predicate over handles); the predicate it receives is the VM's mark-bit read — the module never learns what "alive" means |

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG DOM §3.2 Interface AbortSignal | static any(signals) method steps (create a dependent abort signal with AbortSignal and the current realm) | sequence argument (Array read today) | `native_abort_signal_static_any` → `create_dependent_abort_signal` (NEW) | ✗ (sequence conversion: `#11-webidl-sequence-dense-array-fast-path`, §8) | yes |
| WHATWG DOM §3.2 Interface AbortSignal | create a dependent abort signal step 1 (new object) | — | `create_abort_signal` on `VmInner` (row with `DependentLinks::default()`) | ✓ | yes |
| WHATWG DOM §3.2 Interface AbortSignal | create a dependent abort signal step 2 (an input is aborted → copy its reason, return; not dependent) | first aborted input / none | `DependentCreation::AlreadyAborted` (NEW) → VM sets the reason only (D-5) | ✓ | yes |
| WHATWG DOM §3.2 Interface AbortSignal | create a dependent abort signal step 3 (dependent ← true) | — | `create_dependent_abort_signal` (NEW) | ✓ | yes |
| WHATWG DOM §3.2 Interface AbortSignal | create a dependent abort signal step 4.1 (non-dependent input: append to sources; append result to its dependents) | non-dependent input; duplicate input (set) | same | ✓ | yes |
| WHATWG DOM §3.2 Interface AbortSignal | create a dependent abort signal step 4.2 (dependent input: for each of its source signals, assert not aborted and not dependent; append both ways) | dependent input; its sources | same; `debug_assert!` for 4.2.1 | ✓ | yes |
| WHATWG DOM §3.2 Interface AbortSignal | signal abort steps 1-2 (aborted → return; reason ← given, otherwise a new "AbortError" DOMException) | reason given / undefined | step 1 = `signal_abort` (NEW) returns `None`; step 2's "otherwise" = `create_default_abort_error` before the call (engine-bound) | ✓ | yes |
| WHATWG DOM §3.2 Interface AbortSignal | signal abort steps 3-4 (dependentSignalsToAbort; for each non-aborted dependent: set its reason, append) | aborted dependent (skipped) / non-aborted | `signal_abort` (NEW) → `AbortPlan` (NEW) | ✓ | yes |
| WHATWG DOM §3.2 Interface AbortSignal | signal abort steps 5-6 (run the abort steps for signal, then for each planned dependent) | — | `abort_signal`'s loop over the plan (iterative; handles rooted, D-4) → `run_abort_steps` (NEW) | ✓ | yes |
| WHATWG DOM §3.2 Interface AbortSignal | run the abort steps 1-3 (run and empty the abort algorithms; fire an event named abort) | listener-removal algorithms; fetch algorithms; the fire | `run_abort_steps` (NEW): `detach_bound_listeners`, the fetch rejection block, `dispatch_vm_simple_event` | ✓ (the two algorithm stores stay two, §8) | yes |
| WHATWG DOM §3.2 Interface AbortSignal | aborted getter / reason getter / throwIfAborted ("aborted when its abort reason is not undefined") | aborted / not | `native_abort_signal_get_aborted`, `native_abort_signal_get_reason`, `native_abort_signal_throw_if_aborted` read `reason.is_some()` (D-3) | ✓ | yes |
| WHATWG DOM §3.2 Interface AbortSignal | source signals / dependent signals are weak sets | dead member | sweep-tail prune via `DependentLinks::retain_live` (NEW); no trace edge | ✓ | no |
| WHATWG DOM §3.2 Interface AbortSignal | static abort(reason) method steps step 2 (set the reason; no abort steps) | reason given / undefined | `native_abort_signal_static_abort` sets the reason only (D-5) | ✓ | yes |
| WHATWG DOM §3.2 Interface AbortSignal | static timeout(milliseconds) method steps step 3.1 (signal abort with a new "TimeoutError" DOMException) | — | unchanged timer path (`internal_abort_signal`) into the restructured `abort_signal`; citation at `natives_timer.rs:330` | ✓ | yes |
| WHATWG DOM §3.2.1 Garbage collection | the normative requirement ("must not be garbage collected": non-aborted ∧ dependent ∧ source signals non-empty ∧ (abort listener registered ∨ abort algorithms non-empty)) | listenered unreferenced dependent | **PR-B** (`#11-abort-signal-dependent-keepalive`): `KeepaliveClass::AbortSignalDependent` (NEW) in PR-B; PR-A only supplies `dependent` / `source_signals` | ✗ (PR-B) | no |
| WHATWG DOM §3.1 Interface AbortController | abort(reason) method steps (signal abort on this's signal) | — | `native_abort_controller_abort` → `abort_signal` (entry unchanged) | ✓ | yes |
| WHATWG DOM §2.7 Interface EventTarget | add an event listener step 2 (signal aborted → return) and step 6 (add abort steps: remove the listener) | — | `event_target.rs:172-181` reads `aborted` (D-3); `:215-222` writes the listener-removal algorithm, which `run_abort_steps` runs as step 1 | ✓ | yes |

Numbers and titles: `webref heading dom 3` (§3 Aborting ongoing activities; §3.1 Interface AbortController; §3.2
Interface AbortSignal; §3.2.1 Garbage collection; §3.3 Using AbortController and AbortSignal objects in APIs),
`webref heading dom 2.7` (Interface EventTarget), `webref heading dom 2.8` (Observing event listeners; cited by
`keepalive.rs`, read for PR-B's bound, not cited by number in code). Dfns: `webref dfn dom 'create a dependent abort
signal'`, `'signal abort'` (two hits: for=AbortController → §3.1 `#abortcontroller-signal-abort`; for=AbortSignal →
§3.2 `#abortsignal-signal-abort`), `'dependent'`, `'source signals'`, `'dependent signals'`, `'abort algorithms'`,
`'abort reason'` (all §3.2), `'add an event listener'` (§2.7 `#add-an-event-listener`). Prose: `webref body dom
interface-AbortSignal`, `abort-signal-garbage-collection`, `interface-abortcontroller`, `add-an-event-listener` (steps 2
and 6). IDL: `webref idl dom AbortSignal` (`[NewObject] static AbortSignal _any(sequence<AbortSignal> signals)`). All
2026-11-08. `webref coverage-map dom 3.1 dom 3.2 dom 3.2.1 dom 2.7` (run 2026-11-08) → spec = 1 (dom), step = 4
entries (verified 2026-11-08: the four distinct sections of the table), "ok → single PR scope". **Breadth: K = 1, M = 17**; single PR (PR-A).

Step numbers are cited as `webref body` shows them today; code comments name the algorithm and the step ("create a
dependent abort signal step 4.2"), not a bare `§3.2 step N`.

### §3.1 User-input touch audit

- `AbortSignal.any` (`native_abort_signal_static_any`): the argument is user script's; today's Array read and brand
  checks are kept (D-10); the new path is the algorithm over validated handles. A dependent input whose source set is
  user-shaped (any length) is iterated once — no recursion, no cap needed.
- `controller.abort(reason)` / `AbortSignal.abort(reason)`: `reason` is any user value; stored as today, now in
  `Option`. The plan length is the number of live dependents of one signal (user-shaped); one loop.
- Adjacent pre-existing: the Array-only argument read (`abort_statics.rs:113-120`) — exposure unchanged, owned by the
  sequence slots (§8).

## §4. Mechanism

### §4.1 The module (`crates/dom/elidex-dom-api/src/abort.rs` (NEW))

```rust
//! DOM §3.2 Interface AbortSignal — the dependent-signal graph and the two
//! algorithms that read and write it ("create a dependent abort signal",
//! "signal abort" steps 1-4). Engine-independent: generic over the host's
//! handle (`H`) and reason (`R`) types; runs no abort step (the host's).

#[derive(Default)] struct OrderedSet<H> { order: Vec<H>, members: HashSet<H> }   // module-private; every write updates both halves
#[derive(Default)]
pub struct DependentLinks<H> { dependent: bool, source_signals: OrderedSet<H>, dependent_signals: OrderedSet<H> }   // all private (M2)
impl<H: Copy + Eq + core::hash::Hash> DependentLinks<H> {
    pub fn is_dependent(&self) -> bool;
    pub fn source_signals(&self) -> &[H];           pub fn dependent_signals(&self) -> &[H];
    fn set_dependent(&mut self);                    // step 3; module-private
    fn append_source(&mut self, h: H);              fn append_dependent(&mut self, h: H);   // set semantics
    pub fn retain_live(&mut self, alive: impl Fn(H) -> bool);                                // sweep prune (B)
    fn take_dependents(&mut self) -> Vec<H>;                                                  // D-6
}
pub trait AbortSignalGraph {
    type Handle: Copy + Eq + core::hash::Hash;
    type Reason: Copy;
    /// Specified only for a handle with a row (`links(h).is_some()`): `None` ⇔ not aborted.
    fn abort_reason(&self, h: Self::Handle) -> Option<Self::Reason>;
    fn set_abort_reason(&mut self, h: Self::Handle, reason: Self::Reason);
    /// `None` ⇔ no such signal (the existence test).
    fn links(&self, h: Self::Handle) -> Option<&DependentLinks<Self::Handle>>;
    fn links_mut(&mut self, h: Self::Handle) -> Option<&mut DependentLinks<Self::Handle>>;
}
pub enum DependentCreation<R> { AlreadyAborted(R), Linked }
pub fn create_dependent_abort_signal<G: AbortSignalGraph>(g: &mut G, result: G::Handle, signals: &[G::Handle]) -> DependentCreation<G::Reason>;
pub struct AbortPlan<H> { pub signal: H, pub dependents: Vec<H> }
pub fn signal_abort<G: AbortSignalGraph>(g: &mut G, signal: G::Handle, reason: G::Reason) -> Option<AbortPlan<G::Handle>>;
```

`create_dependent_abort_signal` is steps 2-5 verbatim (`result` is the host's step-1 object): step 2 returns
`AlreadyAborted(reason)` without touching `result`'s links; step 3 `set_dependent`; step 4 appends through the set
primitives, step 4.2.1 as `debug_assert!(g.abort_reason(s).is_none() && g.links(s).is_some_and(|l| !l.is_dependent()))`.
`signal_abort` is steps 1-4: `None` if `links(signal)` is `None` (no such signal — today's `let Some(state) … else
{ return Ok(()) }` at `abort.rs:555-557`, kept as a contract rather than an accident) or `abort_reason(signal).is_some()`
(step 1); else set the reason, take the dependents (D-6), set each non-aborted dependent's reason to the same
`reason` value, return the plan.

**The `links(h) == None` contract.** In the VM every row is created by the one constructor
`VmInner::create_abort_signal` (`abort.rs:322-332`; `git grep -n 'AbortSignalState::new' -- crates` → that one
site) with `links: DependentLinks::default()`, and a row is removed only when its object dies, in the sweep
(`collect.rs:1442-1443`). So `None` means "no such signal": a handle that was never a signal or whose object has been
collected. Where the module can meet it: (i) `create_dependent_abort_signal`'s `result` and every element of
`signals` — the VM has brand-checked each (`abort_statics.rs:137-141`), and a brand-checked live `AbortSignal` has a
row, so the module treats a `None` here as a caller bug (`debug_assert!`, then skip the handle); (ii) a member of a
`source_signals` / `dependent_signals` set — never `None`: a member's row and its set entries die in the same sweep
(A × B), and no sweep runs between planning and the plan loop on the native paths, while on the timer path the plan
handles are rooted (D-4); (iii) `signal_abort`'s `signal` — `None` returns `None` (contract above). The VM impl of
`abort_reason` returns `None` for a missing row; the module never relies on that (it tests `links` first), so the
`None`-means-two-things ambiguity of the raw method does not reach the algorithm. The module's unit tests use a
`HashMap<u32, (Option<&'static str>, DependentLinks<u32>)>` graph (§6 a-d).

### §4.2 Depth 1 by construction

From step 4: a signal is appended to a `dependent_signals` set only at 4.1.2 (the signal is non-dependent, by 4.1's
test) or 4.2.3 (the signal is a source signal, asserted non-dependent at 4.2.1). So **no dependent signal has
dependent signals**, and "signal abort" step 4 on a non-dependent signal reaches every transitive dependent in one
loop; step 4 on a dependent signal (only via its own abort, which the spec reaches through its sources) finds an empty
set. A chain `any([any([any([a])])])` has every result sourced directly on `a`; `a`'s set holds all three. Nothing in
the VM recurses: `abort_signal` runs `run_abort_steps` over `plan.signal` then `plan.dependents` in a `for`. The only
nested Rust frames on the abort path are JS re-entry from listeners (PR-1's CP-1), which is exactly the census
classification PR-1 §5.3 makes ("`abort_signal` is cut on A-1").

### §4.3 The VM (`crates/script/elidex-js/src/vm/host/abort.rs`, `abort_statics.rs`)

- `AbortSignalState { reason: Option<JsValue>, links: DependentLinks<ObjectId>, bound_listener_removals }` (D-2, D-3);
  `AbortSignalState::new()` → `reason: None`, `links: DependentLinks::default()`.
- `impl AbortSignalGraph for VmInner` (local type, foreign trait — permitted): four one-line methods over
  `abort_signal_states`.
- `native_abort_signal_static_any`: validation as today (brand check, first-aborted capture dropped — the algorithm
  does step 2); allocate; `match create_dependent_abort_signal(ctx.vm, composite, &inputs)` → `AlreadyAborted(r)` sets
  `reason = Some(r)` (D-5), `Linked` returns. The self-reference guard at `:170-172` goes (a fresh handle cannot be
  an input).
- `native_abort_signal_static_abort`: allocate; materialise the default if `undefined`; set the reason (D-5). No
  `abort_signal` call.
- `abort_signal(ctx, signal_id, reason)`: `if ctx.vm.links(signal_id).is_none_or(|_| ctx.vm.abort_reason(signal_id).is_some()) { return Ok(()) }`
  (no row, or step 1 — before materialising, as today's `:528-535` / `:555-557`); materialise; `let Some(plan) =
  signal_abort(ctx.vm, signal_id, r) else { return Ok(()) }`; root the plan in **one stack scope** held to the end
  of the call (the timer-path guarantee, §0.2 item 7; why not `push_temp_root`, D-4):
  `let mut frame = ctx.vm.push_stack_scope(); frame.stack.push(JsValue::Object(plan.signal)); for &d in
  &plan.dependents { frame.stack.push(JsValue::Object(d)); } let mut rooted = NativeContext::new_call(&mut frame);`
  then **run every entry and propagate the first `Err` afterwards** (D-4's contract), on `rooted`:
  `let mut first_err = run_abort_steps(&mut rooted, plan.signal).err(); for d in plan.dependents { if let Err(e) =
  run_abort_steps(&mut rooted, d) { first_err.get_or_insert(e); } } first_err.map_or(Ok(()), Err)`. The scope
  drops at the end of the call and truncates the stack to its entry length (`temp_root.rs:203-229`; its clean-drop
  check is `len >= saved_len` only, so listener re-entry that pushes and pops above the base is the expected shape). No `?` inside the loop; the PR body
  gives P0a the post-PR-A names (§9.1).
- `run_abort_steps(ctx, id)` (NEW, `pub(super)` is not needed — private): step 1 = the two algorithm stores in
  today's order within each (`bound_listener_removals` → `abort_listener_back_refs` prune → `detach_bound_listeners`;
  then the `fetch_abort_observers` block moved up from `:627-655`); step 2 = both emptied (they are drained by
  `take` / `remove` today); step 3 = `dispatch_vm_simple_event(ctx, id, abort, false, false)` (under `#[cfg(test)]`,
  a set `fail_next_abort_dispatch` is cleared and returns `Err` in place of this call — §6 (o); P0a later replaces
  the field with its `InjectAt::SimpleEventDispatch`, §9 Edges). A row missing for `id`
  (impossible after rooting; kept as the `let Some(..) else { return Ok(()) }` today's `:555-557` has) is a no-op.
- `internal_abort_signal` unchanged. Module docs `abort.rs:17-25`, `:43-44`, `:82-113`, `:493-521`, `:594-606` and
  `abort_statics.rs:1-12`, `:82-95` rewritten for the new shape; the citation lines of D-8's tables.

### §4.4 GC (`gc/collect.rs`, `gc/roots.rs`, `gc/trace.rs`, `gc/keepalive.rs`)

- `collect.rs:1507-1522` (the map) → after the row prune at `:1442-1443`: `for state in abort_signal_states.values_mut()
  { state.links.retain_live(|h| bit_get(marks, h.0)); }`. Same pass, same mark bits (A × B).
- `roots.rs:257-261` and note (k) `:600-618` rewritten: the sets are weak by spec; the §3.2.1 root is the arm slot
  `#11-abort-signal-dependent-keepalive` adds on the keepalive seam (pointer by slot id, never "PR-B").
  `keepalive.rs:83-92`'s "future arm" sentence is rewritten to name that slot id, the fields it reads, and the
  **full** §3.2.1 predicate (today's paraphrase at `:84-85` drops `dependent` and the "or its abort algorithms is
  non-empty" disjunct; D-7 has the full form). No arm is added in PR-A.
- `trace.rs:385-398`: `mark_value` on `Some(reason)`; the comment states that `links` is not traced and why.
- `mod.rs:1365-1381` (field + doc) deleted; `:1267-1278` doc rewritten; `init.rs:815` deleted; `object_kind.rs:273-292`
  doc follows D-3 / D-2.

### §4.5 ECS-native and layering checks

- Side-store→component: applied in D-2 — the row's owner is a VM `ObjectId`, not an entity; the sets hold `ObjectId`s
  (exception (a)). No `HashMap<Entity, _>` is introduced; `any_composite_map` (an `ObjectId`-keyed side-store that
  duplicated a fact of the row) is deleted in favour of the row's own field — one fact, one place.
- Layering mandate: `vm/host/abort*.rs` after PR-A contains brand checks, allocation, `DOMException` materialisation,
  listener detach, fetch rejection, event fire, and calls into `elidex_dom_api::abort` (NEW) for the graph algorithms. No
  selector / tree / graph algorithm is written in `host/` (DoD 3 checks that `source_signals` / `dependent_signals`
  are not mutated outside the module).
- One issue, one way: there is one way a signal learns its dependents (the row's set) and one way an abort reaches them
  (the plan); the strangler state "map + sets" does not exist at any commit of PR-A.
- Plugin-first: not engaged (no extension point; `AbortSignal` is a built-in with static dispatch today and after).
- Concurrency: unchanged — all state is VM-thread-owned.

## §5. Populations

### §5.1 What PR-A edits and adds (`wc -l` at `69fbe61f`, run 2026-11-08 in the worktree, whose tree equals `69fbe61f`)

| File | Lines | Change |
|---|---|---|
| `crates/dom/elidex-dom-api/src/abort.rs` (NEW) | — | the module (§4.1) with inline unit tests; about 250 |
| `crates/dom/elidex-dom-api/src/lib.rs` | 132 | `pub mod abort;` |
| `crates/script/elidex-js/src/vm/host/abort.rs` | 733 | `AbortSignalState` (D-2, D-3), `impl AbortSignalGraph for VmInner`, `abort_signal` restructured (D-4, incl. the `Err` contract), `run_abort_steps` (NEW), docs, citations `:201`, `:465`, `:538`, `:594`, `:661`, `:499` / `:585` (HTML named), `:615-616` (Fetch §5.6 step 11), `:512` hygiene |
| `crates/script/elidex-js/src/vm/host/abort_statics.rs` | 181 | `any` on the algorithm; `abort` reason-only; docs; the seven citations |
| `crates/script/elidex-js/src/vm/host/event_target.rs` | 734 | `:172-181` reads `reason.is_some()` (D-3); the 2 `§2.6.3` citation lines `:110`, `:168` (D-8 sub-family 2); no other change (its 14 EventTarget-only citation lines are §9.2's) |
| `crates/script/elidex-js/src/vm/host/fetch/mod.rs` | 305 | `:298-305` `pre_flight_abort_reason` returns `state.reason` (D-3) |
| `crates/script/elidex-js/src/vm/mod.rs` | 2785 | `any_composite_map` field + doc deleted (`:1358-1381`, incl. the `:1360` citation); `abort_signal_states` doc (`:1267-1278`); citation `:1242`; `#[cfg(test)] fail_next_abort_dispatch: bool` (NEW, beside `:276-277`; cell (o); removed by P0a, §9 Edges) |
| `crates/script/elidex-js/src/vm/init.rs` | 858 | `:815` deleted |
| `crates/script/elidex-js/src/vm/object_kind.rs` | 1645 | docs `:273-292` |
| `crates/script/elidex-js/src/vm/globals.rs` | — (one comment line, `:719`) | citation |
| `crates/script/elidex-js/src/vm/natives_timer.rs` | 440 | citation `:330` |
| `crates/script/elidex-js/src/vm/gc/collect.rs` | 2074 | `:1507-1522` → the set prune (§4.4) |
| `crates/script/elidex-js/src/vm/gc/roots.rs` | 619 | `:257-261`, `:600-618` docs |
| `crates/script/elidex-js/src/vm/gc/trace.rs` | 1255 | `:385-398` (`Option` reason; comment) |
| `crates/script/elidex-js/src/vm/gc/keepalive.rs` | 434 | `:83-92` doc names PR-B |
| `crates/script/elidex-js/src/vm/tests/tests_abort_statics.rs` | 427 | header `:2`, `:246` citations; `any_*` tests rewritten in place (§6); `:280-301` comment rewritten |
| `crates/script/elidex-js/src/vm/tests/tests_abort_dependent.rs` (NEW) | — | §6 cells e-l, n, o (with the two test natives of (n)) |
| `crates/script/elidex-js/src/vm/tests/mod.rs` | 207 | `mod tests_abort_dependent;` |
| `crates/script/elidex-js/src/vm/tests/tests_fetch.rs` | 570 | citation `:429`; §6 cell (m) (it needs this file's fetch harness — the broker stub and `fetch_composite_signal_any_pre_abort_rejects`'s fixtures — so it lives here, not in the new file) |
| `crates/script/elidex-js/src/vm/tests/tests_ui_event_constructor.rs` | — (one comment line, `:357`) | citation |

Not touched, with the reason: `crates/script/elidex-js/src/tests_dispatch_integration.rs` (`:487-495` reads
`abort_signal_states.get(..).map_or(.., |s| s.bound_listener_removals.len())` — the store PR-A does not change; no
`.aborted` / `.reason` read: measured, it is absent from D-3's grep); `crates/script/elidex-js/src/vm/tests/tests_abort.rs`
(950 — new cells go to the new file, D-9); `crates/script/elidex-js/src/vm/inner.rs`, `fetch_tick.rs`,
`event_target_dispatch_vm.rs` (read the unchanged stores); `crates/dom/elidex-dom-api/Cargo.toml` (no new
dependency: the module uses `core` and `std` only); `deny.toml`; `scripts/`, `.claude/tools/`. The file list is the
union of `git grep -l 'any_composite_map' -- crates` (8 files, verified 2026-11-08), the citation files of §0.2
item 9 (10 files, verified 2026-11-08), the two NEW files and the D-3 reader files; re-run both greps at the
implementation head.

**Overlaps with open branches (measured 2026-11-08; rebase-only, no design question).** VM stack-safety guard PR-1
(memo at `4557c99b`) touches `vm/mod.rs` (one `mod regexp_compile;` line, memo `:404-405`; `:394` is that file's
touch-time verdict), `vm/object_kind.rs` (the
`RegExp` variant's `compiled` field, `:745-746`) and `vm/tests/mod.rs` (it adds `tests_stack_guard.rs`, `:775`) —
three files PR-A also edits, in disjoint ranges; PR-A lands first (§9), so PR-1 rebases. L2's branch
`layout-dom-c0c-adopt-notification` edits `crates/dom/elidex-dom-api/src/lib.rs` (`git diff --stat origin/main...layout-dom-c0c-adopt-notification`
→ `lib.rs | 2 +-`) — PR-A adds `pub mod abort;` to the same file. That branch has since **landed** (#611
`0a5a77ba`, §9 Edges), so PR-A's branch, cut from the current `origin/main`, starts on it — no rebase remains.
Rev 1's "no file overlap" was stated without this measurement and was false. DoD 11 re-runs the check at the head.

### §5.2 Every mutation of the two sets

After PR-A the only writers of `source_signals` / `dependent_signals` are the module's private `append_*` /
`take_dependents` / `retain_live` (DoD 3). At `69fbe61f` the corresponding fact ("who writes `any_composite_map`") is
`abort_statics.rs:173-177` (push) and `abort.rs:607` (remove) and `collect.rs:1516` (retain) — three sites, all
deleted.

## §6. Test and coverage plan

Module unit tests (`elidex-dom-api`, no VM; a `HashMap` graph with `u32` handles and `&str` reasons):

- (a) `flattening_sources_are_never_dependent`: `d1 = create([a])`, `d2 = create([d1, b])` → `d2.sources == [a, b]`,
  `a.dependents == [d1, d2]`, `d1.dependents == []`, `d2.dependent && d1.dependent`.
- (b) `already_aborted_input_short_circuits`: `create([a_aborted, b])` → `AlreadyAborted("ra")`, result not dependent,
  no links written on `b`.
- (c) `set_semantics`: `create([a, a])` → one source, one dependent on `a`.
- (d) `signal_abort_plans_then_empties`: `abort(a)` with dependents `d1`, `d2` (`d2` pre-aborted) → plan `[d1]`, both
  reasons set (`d1 == a`'s), `a.dependents == []` after, second `abort(a)` → `None`; `retain_live` drops a dead
  handle from both sets.

VM cells (`tests_abort_dependent.rs` (NEW); red at `69fbe61f` where marked):

- (e) **red** `any_chain_depth_one_kept_intermediates_10000` — 10 000 nested `any([s])` with intermediates kept, then
  `c.abort()` → `s.aborted === true`, process alive (today: SIGABRT, §0.2 item 5; this is the same shape as PR-1's
  `abort_any_chain_depth_one_precondition`, kept in both suites on purpose: PR-1's is its precondition check, this is
  the slot's own).
- (f) **red** `any_chain_dropped_intermediates_propagates_after_gc` — 50 nested `any()` with only the last kept;
  `collect_garbage()`; `c.abort()` → `s.aborted === true` and `s.reason === c.signal.reason` (today: `false`).
- (g) **red** `dependents_are_aborted_inside_source_listener` — `d.aborted === true` read from `c.signal`'s listener
  (today: `false`).
- (h) `sets_prune_dead_handles_at_sweep` — `d = any([a.signal, b.signal])` kept; drop `b` and its controller;
  `collect_garbage()`; `links(d).source_signals == [a.signal]`; `links(a.signal).dependent_signals == [d]`.
- (i) `dropped_dependent_is_pruned_from_source` — the rewrite of `any_composite_map_is_weak_bookkeeping`: `c` dropped,
  GC → `links(a.signal).dependent_signals` empty; `a.abort()` is a silent no-op for dependents.
- (j) `reason_identity_across_dependents` — default `AbortError`: `d1.reason === d2.reason === c.signal.reason` (one
  object).
- (k) `already_aborted_any_result_is_not_dependent` and `empty_any_is_dependent_with_no_sources` — the internal flags
  PR-B's predicate reads (D-5; step 3 with an empty list).
- (l) `listener_reentry_abort_other_source_fires_dependent_once` — `d = any([a, b])`; `a`'s listener calls `b.abort()`;
  `d`'s listener count = 1; `b.signal.aborted === true`.
- (m) `fetch_rejection_runs_as_abort_step` — **in `tests_fetch.rs`** (its fetch harness; §5.1): a fetch bound to
  `c.signal`; `c.abort()` → the promise rejects with the reason (existing behaviour); the internal order is asserted
  by the plan (the stores are empty when the `'abort'` listener runs: `vm.inner.fetch_abort_observers` has no entry
  for the signal inside the listener — observed through a native-side probe, not JS).
- (n) **red at `69fbe61f` in the sense that the guarantee it checks does not exist there** (the map has no plan to
  root; the cell is written against PR-A's shape and is the mutation proof of D-4's rooting) —
  `timer_path_plan_handles_survive_gc_inside_listener`. **The collection must run after planning, not before**:
  setting `force_gc_before_next_alloc` natively before `drain_timers` (Rev 2's shape) fires it at the drain's first
  allocation, `build_dom_exception` for the `TimeoutError` reason (`natives_timer.rs:345` →
  `crates/script/elidex-js/src/vm/host/dom_exception.rs:311`), which precedes `signal_abort`, so the dropped
  dependent is swept before it is planned and the cell is green with or without the root (vacuous). So the arming
  is done from inside `t`'s listener by a test native, the shape of `native_disconnect_peer_and_arm_gc`
  (`crates/script/elidex-js/src/vm/tests/tests_observer_keepalive/mid_delivery_gc.rs:105-129`, which sets
  `ctx.vm.force_gc_before_next_alloc = true` and returns). Setup: `globalThis.t = AbortSignal.timeout(0)` (rooted by
  `pending_timeout_signals`, `keepalive.rs:272-275`); `AbortSignal.any([t])` with the result **dropped** (no
  reference, no listener), its `ObjectId` `dep` read natively as the one row `abort_signal_states` gained;
  `t.addEventListener('abort', () => { __armGc(); ({}); globalThis.g = __gcFired();
  globalThis.fresh = new AbortController(); fresh.signal.addEventListener('abort', () => {
  globalThis.misfire = true; }); })`. The native call runs with
  the gate's `gc_enabled = false`; the object literal is the first allocation after it with `gc_enabled = true`
  (the timer path's regime, §0.2 item 7), so the forced collection runs there — after planning, inside the plan
  loop. Advance the clock and `vm.inner.drain_timers(now)`. **Preconditions** (asserted, both builds): the flag was
  consumed inside the listener (`g === true`; test native `__gcFired()` returns `!force_gc_before_next_alloc`; the
  `tests_member_compound_assign_gc.rs:59` style); `dep` was planned (natively, before the drain,
  `links(t).dependent_signals() == [dep]`). **Kill assertion** (independent of where the freed slot goes): after the
  drain, `abort_signal_states` has a row at `dep` whose reason is the same value as `t.reason` (`same_value`) — a
  recycled occupant's row is `None` and a swept row is absent, so the mutant is red whatever the free list does;
  plus `misfire === undefined`, `fresh.signal.aborted === false`, `t.aborted === true`. **Mutant**: drop the pushes
  onto the scope's stack in `abort_signal` → `dep` is swept by the forced collection inside `t`'s listener and
  `run_abort_steps(dep)` runs against a missing row or a new occupant → the kill assertion is red. The mutation
  proof records the listener's allocation order. Variants in the same file: the dependent kept in a global
  (control: green with or without the root), and two dropped dependents of one `t` (the plan has two
  otherwise-unrooted handles).
- (o) `abort_err_does_not_skip_dependents` — the `Err` contract (D-4), driven through `internal_abort_signal`
  directly (`abort.rs:681-688`; the timer path's entry, without the timer): `drain_timers` cannot carry it, since
  it does not return a timeout fire's `Err` — the `AbortSignal.timeout` fire is excluded from its results
  (`natives_timer.rs:291-293`, `:358-359`) and its `Err` only reaches `eprintln!` (`:349-351`). Setup: `c = new
  AbortController()`, `d1 = AbortSignal.any([c.signal])`, `d2 = AbortSignal.any([c.signal])`, both kept, each with
  an `'abort'` listener that records it ran. The first dispatch of the plan (the source's own fire) is made to
  return `Err` through a `#[cfg(test)]` field `VmInner::fail_next_abort_dispatch: bool` (NEW, test-only; the
  `force_gc_before_next_alloc` precedent, `mod.rs:276-277`), checked by `run_abort_steps` at step 3 in place of the
  `dispatch_vm_simple_event` call and cleared when it fires — the position P0a's `InjectAt::SimpleEventDispatch`
  takes (P0a §4.4). An injection seam is the only way to produce this `Err` today: `dispatch_vm_simple_event`
  never returns `Err` at `69fbe61f` (D-4). Call `internal_abort_signal(&mut vm.inner, c_signal_id,
  JsValue::Undefined)` → returns `Err`; `d1.aborted && d2.aborted`; both dependents' listeners ran;
  `c.signal.aborted === true`. Mutant: `?` on the first
  `run_abort_steps` (or inside the loop) → `d1`'s and `d2`'s listeners do not run → red. **This seam is
  temporary by plan**: P0a rebases on PR-A, deletes `fail_next_abort_dispatch` and re-arms this cell with
  `InjectAt::SimpleEventDispatch(n)`, same assertion (P0a §4.4, `p0a.md:165-173`; P0 §7 L-12, `p0.md:334`; inverted
  order: §9 Edges).

Rewritten in place (`tests_abort_statics.rs`): `any_duplicate_input_fires_composite_only_once` (`:304-321`) keeps its
assertion and gains "one source"; `any_composite_map_cleaned_after_input_aborts` (`:324-340`) → asserts
`links(a.signal).dependent_signals` emptied (D-6) and `b.signal`'s intact; `any_composite_map_is_weak_bookkeeping`
(`:343-377`) → (i) above (moved); `any_composite_with_user_ref_survives_gc_and_propagates_abort` (`:379-396`) keeps
its behaviour with the map assertion replaced by a links assertion;
`any_direct_composite_abort_does_not_touch_inputs` (`:280-301`) **stays green with its comment rewritten** (it
describes "only the input → composite direction is wired" in map terms; the sets have the same one direction by
construction — step 4 appends the result to the source's `dependent_signals`, never the source to the result's).
Stay green unchanged: `any_chained_composites_propagate_through_chain` (`:262-277`), `any_two_inputs_*`,
`any_three_inputs_*`, `abort_signal_any_*`, `tests_fetch.rs` `fetch_composite_signal_any_pre_abort_rejects`,
`tests_abort_signal_option.rs`, `tests_dispatch_integration.rs:447-497` (reads `bound_listener_removals` only).

Guards: DoD 2-4 greps. No WPT subset exists for `dom/abort` in this repository (§0.2 item 10); the cells above are
the supported-surface regression set for `AbortSignal.any`.

## §7. 1000-line audit (CLAUDE.md "1000-line debt")

Files over 1000 that PR-A edits, with the touch-time cohesion verdict for the range PR-A touches (CLAUDE.md's
discipline is cohesion, not line count; the size reading `docs/plans/2026-07-s5-6-flip-boa-deletion.md:1130` "<50,
no seam obligation" is the floor, not the test):

| File | Lines | PR-A's range | Owner of the file-level verdict (`docs/plans/2026-07-vm-p4-touch-time-split-record.md:81-84`) | PR-A's verdict for its range |
|---|---|---|---|---|
| `vm/mod.rs` | 2785 | `:1242` (one comment), `:1267-1278` (one doc), `:1358-1381` (field + doc deleted; about −24 / +3) | on no watch list; PR-1's memo `:394` (at `4557c99b`): "no touch-time seam for the range PR-1 touches" | the `VmInner` field block is a flat declaration list; a seam would be a struct split (`VmInner` → sub-structs), which `vm-inner-substruct-deferral` already records as deferred with its reasons — not PR-A's to cut by deleting one field |
| `gc/collect.rs` | 2074 | `:1507-1522` → the set prune (about −16 / +6), beside `:1442-1443` | umbrella 7b | the sweep tail is a sequence of per-store `retain` calls (a flat case table); PR-A removes one and adds one in the same block — no cohesion boundary is created or crossed |
| `gc/trace.rs` | 1255 | `:385-398` (±3) | umbrellas 7a and 7b | one arm of the `ObjectKind` trace `match`; the file's seam, if any, is the match itself (7a/7b's) |
| `object_kind.rs` | 1645 | `:273-292` (docs only) | umbrellas 6, 7 and 10 | a doc comment on one variant |

`abort.rs` goes from 733 to about 760 (the `Err`-collecting loop and `run_abort_steps` add; the fan-out block
goes); `event_target.rs` 734 → 734 (comment edits); `tests_abort.rs` (950) is not edited (D-9); `tests_fetch.rs`
570 → about 610 (cell (m)). No file crosses 1000 and no split is a prerequisite of PR-A.

## §8. Out of scope, and slots

- `#11-webidl-sequence-dense-array-fast-path` (open; ledger `:276`) and `#11-vm-webidl-sequence-early-exit-drain`
  (open): the `any()` argument's conversion (today an `ObjectKind::Array` read with no `Symbol.iterator`) is theirs;
  PR-A keeps the read and the comment that names them replaces `abort_statics.rs:102-107`'s "PR5a2" note (that PR,
  #85, landed without it).
- **`#11-abort-signal-dependent-keepalive`** (= PR-B; **registered, PM 512**, open, owner L3 — ledger memory
  `project_open-defer-slots.md:1360`). Row text (Rev 2's draft; the ledger's registered row is canonical): *Defect*: a non-aborted dependent `AbortSignal` (an `any()` result) that script no longer
  references but that has an `'abort'` listener (or a pending abort algorithm — a `fetch` or an `addEventListener({signal})`)
  is collected, so its listener never fires and its algorithms never run when a source aborts. [spec] DOM §3.2.1
  Garbage collection: "A non-aborted dependent AbortSignal object must not be garbage collected while its source
  signals is non-empty and it has registered event listeners for its abort event or its abort algorithms is
  non-empty." *Shape*: `KeepaliveClass::AbortSignalDependent` (NEW) on the keepalive seam (`keepalive.rs`), its
  registrant loop over `abort_signal_states` rows, the predicate as an engine-independent function in
  `elidex_dom_api::abort` (D-7). *Why deferred*: a behaviour change in the leak direction that needs `dependent` /
  `source_signals` (PR-A's) and its own review; not partially implemented in PR-A. **Prerequisite**: PR-A landed.
  **Trigger**: PR-A's landing (the memo is authored against PR-A's squash sha and re-measures `keepalive.rs`); earlier
  if a consumer reports a lost abort on an unreferenced listenered `any()` result. **Acceptance**: the GC cell
  "listenered unreferenced dependent survives `collect_garbage()` and fires on the source's abort" green, and the
  leak-direction control "unlistenered unreferenced dependent is still collected" green. **Re-eval-date**:
  2026-12-15. The parent slot `#11-abort-signal-any-dependent-flattening` may close at PR-A's landing: this id is
  registered (D-7).
- **`#11-abort-algorithms-single-set`** — **registered (PM 504; ledger memory `project_open-defer-slots.md:1353`), open, owner L3, Re-eval-date 2026-12-15; OUT of
  PR-A** (PM 504: "AbortSignal.any の PR-A に入れるかは L3 が決める" → L3's decision here is *out*). The spec's single
  "abort algorithms" set is two stores (`bound_listener_removals` per row; `fetch_abort_observers` +
  `fetch_signal_back_refs` per VM) run by `run_abort_steps` in sequence. PR-A puts them behind one function, which is
  the shape the slot's fix will replace: folding them into one `AbortAlgorithm` enum on the row is a representation
  change with its own GC and fetch-tick edges (`fetch_tick.rs:343-347`, `:390-394` settle-time removal), and bundling
  it would add a fifth axis to §1's base case. **Trigger**: the next PR that adds a third abort-algorithm store or a
  second site that runs them (today one site runs them, `run_abort_steps`).
- `#11-eventtarget-keepalive-component-migration` (B1-gated): unchanged; D-2's exception (a) is the same reading
  `keepalive.rs:29-33` records.
- `#11-dom-walkers-iterative-over-ecs-links` (L3): unrelated — no DOM walker is on the abort path; the guard
  umbrella's edge from it to PR-1 is soft (PM 483 correction), and PR-A has no file overlap with it.
- `#11-vm-call-depth-guard` / PR-1: the hard edge (§9); PR-A adds no probe and needs none (§4.2).
- The `AbortSignal.timeout` keepalive membership and the timer path: unchanged except `natives_timer.rs:330`.

## §9. Edges, hand-offs, definition of done

**Edges.**

- **VM stack-safety guard PR-1 (VM P4; memos at `4557c99b` on `vm-stack-safety-plan`, tip `ce603c74` when Rev 2 was
  written, now `d7ec4761`; unlanded) — hard edge, PR-A before PR-1.** What PR-1 requires of this slot and how PR-A
  meets it: (1) A-1 (PR-1 §2, `:104`; `:82` at `b7f03821`) "a dependent signal's source signals are never dependent
  … the fan-out is depth 1 by construction" — §4.2; (2) PR-1 §5.3 treats `abort_signal` as a cut point with no
  probe — §4.2 (no Rust recursion on the path; JS re-entry is CP-1's); (3) PR-1's cell
  `abort_any_chain_depth_one_precondition` (§8, `:829`; `:828` at `b7f03821`; 10 000 kept, 2 MiB thread) must be
  green after PR-A — §6 (e) is the same shape; (4) PR-1 §12 L-7 (`:968`; `:952` at `b7f03821`): `git grep -n
  '§3\.1\.3' -- crates/script/elidex-js/src/vm/host/abort_statics.rs` → 0 lines — DoD 6 (a superset); (5) PR-1's PR
  body records `git merge-base --is-ancestor <PR-A squash sha> HEAD` → hand the sha to VM P4 at landing. PR-1 does
  not edit `abort.rs` / `abort_statics.rs` (umbrella §4 row W-11 at `:245`: "not touched by PR-1"); it **does** edit
  `vm/mod.rs`, `vm/object_kind.rs` and `vm/tests/mod.rs` in ranges disjoint from PR-A's (§5.1 "Overlaps") — a rebase
  for PR-1, which lands second. The umbrella's "its own design may take longer than its one-line remedy suggests" is
  answered by D-1 / D-2 (a trait, not a move of the state).
- **The P0 program (PM 502; umbrella `docs/plans/2026-11-vm-stack-safety-guard-p0.md` and slice P0a
  `docs/plans/2026-11-vm-stack-safety-guard-p0a.md`, both cited at `b7f03821` (P0a §4.4's order: `d7ec4761`), owner VM
  P4) — PR-A first, P0a rebases on it.** P0 §1 Edges (`p0.md:113-118`):
  "L3's `#11-abort-signal-any-dependent-flattening` PR → P0a and →
  P0b-L3: P0a's allowlist carries the row of the flattened `AbortSignal` abort loop and P0a moves that PR's test
  seam onto its own injection (P0a §4.4, §5.4), so P0a is rebased on L3's PR". So the order is: **PR-A lands → P0a
  rebases on it, deletes `fail_next_abort_dispatch` and re-arms cell (o) with `InjectAt::SimpleEventDispatch(n)`
  (NEW: P0a), same assertion** (P0a §4.4 "One test-failure seam", `p0a.md:165-173`, ratified PM 513) — P0a edits
  L3's `AbortSignal` files to do it, which P0 §7 L-12 (`p0.md:334`) records as the L3 notice at P0a's merge ("P0a
  edited L3's AbortSignal files after rebasing on `#11-abort-signal-any-dependent-flattening`: the `#[cfg(test)]`
  seam `fail_next_abort_dispatch` is removed and its cell uses `InjectAt::SimpleEventDispatch` …"). PR-A therefore
  adds the field (cell (o) needs it, and no P0a injection exists at PR-A's base) and does not try to anticipate
  P0a's API. P0a also carries the allowlist row for PR-A's abort loop (P0a §5.4, `p0a.md:292-315`), keyed on the
  post-PR-A names it reads at its base (P0a §10 item 4, `p0a.md:434-436`) — the names PR-A's PR body gives (§9.1).
  If the order ever inverts (P0a first), P0a §4.4 / §5.4's inverted-order arms at `d7ec4761` (`:189-190`,
  `:464-465` there) apply to PR-A: seam ("PR-A arms `InjectAt::SimpleEventDispatch(n)` from the start and adds no
  field") and row ("P0a ships no row and L3's PR adds it if the check asks for it"); that is a plan change for
  PR-A, not an implementation choice. P0b-L3 (L3's slice of P0) is after P0a and after PR-A (P0 §1 schedule,
  `p0.md:122-130`) and has its own plan.
- **L3's other work** (`#11-event-path-window-missing` plan, the walker umbrella): no shared file (those edit
  `HostDriver` / event-path and ECS walkers; PR-A edits none of them). Order per PM 485: #607 → **this slot** →
  stale-entity → walker umbrella.
- **L2's C0c** (`layout-dom-c0c-adopt-notification`): **landed** as #611 `0a5a77ba` after Rev 2 was measured
  (`git log --oneline 69fbe61f..origin/main` → `0a5a77ba`, `64e17366`, `a8bf36b4`); its one-line edit of
  `crates/dom/elidex-dom-api/src/lib.rs` (`git diff --stat 69fbe61f origin/main -- crates/dom/elidex-dom-api/src/lib.rs`
  → `lib.rs | 2 +-`; one `--stat` over that file plus `crates/script/elidex-js/src/vm/host/abort.rs`,
  `abort_statics.rs`, `event_target.rs` and `crates/script/elidex-js/src/vm/mod.rs` prints only the `lib.rs` line —
  the `lib.rs` hit is the control that the command reports changes — so origin/main has not moved PR-A's main files) is now on the base PR-A's branch is cut from — no longer
  an overlap, just the base.
- **VM P4's open PRs** (S1 string-replace, Slice E/F): their memos were not re-read; DoD 11 measures the file overlap
  at the head against every open branch, and any overlap is a rebase, not a design question.
- **PR-B** (`#11-abort-signal-dependent-keepalive`, registered PM 512): after PR-A; its memo cites PR-A's squash sha
  and re-measures `keepalive.rs`.
- **The DOM-events renumber sweep** (memory `project_dom-events-section-renumber-sweep`, driver-less tracked note):
  receives the 26 lines of §9.2. No ordering edge: PR-A edits only `event_target.rs:110`, `:168` and `:172-181` of
  the files those lines are in, so the sweep re-runs §9.2's commands at its own base and finds the same lines,
  possibly shifted.

**Hand-offs** (for the PM; the memo edits no ledger): (1) `#11-abort-signal-dependent-keepalive` is registered (PM 512)
— the parent slot `#11-abort-signal-any-dependent-flattening` closes at PR-A's landing (PR number + squash sha)
(D-7); (2) correct the ledger's "seven cites in `abort_statics.rs`" to PR-A's 23-line / 10-file family (§0.2 item 9)
and note that PR-A re-derived all of them; (3) `#11-abort-algorithms-single-set` (PM 504) stays open and out of
PR-A — note L3's decision and the trigger of §8; (4) tell VM P4 the squash sha (PR-1 A-1 ancestry check and L-7),
that the §3.2.1 arm is `#11-abort-signal-dependent-keepalive`, not in PR-A, so PR-1's
`abort_any_chain_depth_one_precondition` depends on PR-A only, and that P0a can rebase (P0 §1 edge): the post-PR-A
names of the abort loop and of cell (o), and the `fail_next_abort_dispatch` field P0a removes, are in PR-A's PR body
(§9.1); (5) **PR-A consumes part of the DOM-events renumber note** (memory
`project_dom-events-section-renumber-sweep`): its two signal-option lines `event_target.rs:110` / `:168`
("§2.6.3 step 3" → §2.7 "add an event listener" step 2) are among that note's "add-listener algorithm" sites and are
fixed in PR-A; the note's remaining sites in this area — the 26 lines of §9.2, and `event_target.rs:131` "§2.7 step
8" (handleEvent; the number resolves, the step is not re-derived here) — stay the note's. Update the note at PR-A's
landing: replace its add-listener site list with §9.2's commands, keeping `event_target.rs:131`.

### §9.1 PR body (what PR-A's description must say)

The PR body carries, besides DoD 1, 7, 9 and 11's records:

- **The defect at the base head** (`69fbe61f`; lines at that sha), in the `Err` shape P0 targets. It is
  **latent** — `dispatch_vm_simple_event` never returns `Err` at the base (D-4; P0a §5.4 at `b7f03821` reads it the
  same way, "latent definitive") — and becomes live when P0's resource-limit error surfaces through dispatch:
  - `abort.rs:592` — `dispatch_vm_simple_event(..)?` runs **after** the latch at `:558-559` has set the signal
    `aborted` with its reason. An `Err` there returns from `abort_signal` with the signal already aborted, so every
    later `abort()` is a no-op (`:528-535`, step 1), and the composite fan-out (`:607-612`) and the fetch fan-out
    (`:627-655`) never run: the dependents are never aborted, the fetches never rejected — permanently.
  - `abort.rs:610` — the recursive `abort_signal(ctx, composite_id, reason)?` runs **after**
    `any_composite_map.remove(&signal_id)` at `:607` has taken the whole composite list. An `Err` for one composite
    returns with the rest of the list dropped (the remaining composites are never aborted — and, the entry being
    removed, no later abort of this signal can reach them) and with this signal's fetch fan-out (`:627-655`) not run,
    so its fetch observers are stranded.
  - **The fix**: "signal abort" steps 4-6 as one plan, and run-all / first-`Err` — `run_abort_steps` for every
    entry, the first `Err` kept and returned after the loop, no `?` inside it (D-4, §4.3).
- **For P0a's census** (P0a §5.4 row and §10 item 4): the post-PR-A call chain `abort_signal` → `run_abort_steps` →
  `dispatch_vm_simple_event`, with file:line at the PR head; the contract "run all, keep the first `Err`, return it
  after the loop"; cell (o)'s name `abort_err_does_not_skip_dependents` and its seam
  `VmInner::fail_next_abort_dispatch` — **P0a moves it onto `InjectAt::SimpleEventDispatch(n)`** (inverted order:
  §9 Edges).
- **The callers of `abort_signal`** at the PR head (DoD 4), with their lines.

### §9.2 Hand-off to the DOM-events renumber sweep (26 lines, not PR-A's)

Owner: memory `project_dom-events-section-renumber-sweep` (its method: "sweep ALL DOM §2.x event citations in live
crates"). Lines at `69fbe61f`; the commands are the hand-off — re-run them at the sweep's base.

**(a) EventTarget-only lines of the files PR-A's census covered (21 lines, 4 files).** Command:
`git grep -nP '§2\.6(?![.\d])|§2\.6\.\d|§2\.7\.7' -- crates/script/elidex-js/src/vm/host/event_target.rs crates/script/elidex-js/src/vm/tests/tests_remove_event_listener.rs crates/script/elidex-js/src/vm/tests/tests_add_event_listener.rs crates/script/elidex-script-session/src/event_listener.rs | grep -vP '§2\.6\.3'`
→ 21 lines (positive control: the same command without the final `grep -v` → 23, and `| grep -cP
'§2\.6\.3'` on it → 2, the two lines PR-A keeps). `event_target.rs` ×14 (`:20`, `:93`, `:101`,
`:113`, `:184`, `:377`, `:445`, `:474`, `:540`, `:550`, `:557`, `:571`, `:592`, `:599`),
`crates/script/elidex-js/src/vm/tests/tests_remove_event_listener.rs` ×2 (`:162`, `:202`),
`crates/script/elidex-js/src/vm/tests/tests_add_event_listener.rs` ×3 (`:122`, `:151`, `:177`),
`crates/script/elidex-script-session/src/event_listener.rs` ×2 (`:90`, `:94`). Rev 2 derived each from `webref`
(2026-11-08) for PR-A; the derivation is kept here as input, not as the sweep's verdict — the sweep re-derives
(`webref body dom add-an-event-listener`, `body dom dom-eventtarget-addeventlistener`, `body dom
dom-eventtarget-removeeventlistener`, `dfn dom 'event listener'` → §2.7 `#concept-event-listener`). Steps as
`webref body` gives them: "add an event listener" — 1 ServiceWorkerGlobalScope warning; 2 "If listener's signal is
non-null and is aborted, then return."; 3 "If listener's callback is null, then return."; 4 default passive; 5 "If
eventTarget's event listener list does not contain an event listener whose type is listener's type, callback is
listener's callback, and capture is listener's capture, then append listener to eventTarget's event listener
list."; 6 the signal's abort steps — addEventListener(type, callback, options) method steps — 1 "Let capture,
passive, once, and signal be the result of flattening more options."; 2 add an event listener —
removeEventListener(type, callback, options) method steps — 1 "Let capture be the result of flattening options.";
2 "If this's event listener list contains an event listener whose type is type, callback is callback, and capture
is capture, then remove an event listener with this and that event listener."

| Line | Old | New (Rev 2's derivation) | Step change |
|---|---|---|---|
| `event_target.rs:20` | `§2.7.7 (type, callback, capture) match` | §2.7 removeEventListener method steps step 2 (the (type, callback, capture) match) | — → 2 |
| `event_target.rs:93` | `WHATWG DOM §2.6 / §2.7.  Behaviour:` | WHATWG DOM §2.7 | — |
| `event_target.rs:101` | `spec §2.6 step 2: "If callback is null, then return."` | §2.7 add an event listener step 3: "If listener's callback is null, then return." | **2 → 3** |
| `event_target.rs:113` | `Duplicate check (§2.6 step 4)` | §2.7 add an event listener step 5 | **4 → 5** |
| `event_target.rs:184` | `WHATWG §2.6 step 4 excludes once / passive` | §2.7 add an event listener step 5 (identity is type, callback, capture) | **4 → 5** |
| `event_target.rs:377` | `WHATWG DOM §2.6 step 1 — extract listener flags from the third argument of add/removeEventListener` | the doc covers **both** methods (`:377-378`): §2.7 addEventListener method steps step 1 ("Let capture, passive, once, and signal be the result of flattening more options.") **and** removeEventListener method steps step 1 ("Let capture be the result of flattening options.") | 1 → 1 (both) |
| `event_target.rs:445` | `§2.7.7's "flatten options" step for removal` | §2.7 removeEventListener method steps step 1: "Let capture be the result of flattening options." | — → 1 |
| `event_target.rs:474` | `§2.6 step 4` | §2.7 add an event listener step 5 | **4 → 5** |
| `event_target.rs:540` | `WHATWG DOM §2.7.7: locate any listener whose (type, callback, …` | §2.7 removeEventListener method steps step 2 | — → 2 |
| `event_target.rs:550` | `§2.7.7 step 2: null / undefined callback → silent no-op` | §2.7 removeEventListener method steps step 2 (a null callback matches no listener) | 2 → 2 |
| `event_target.rs:557` | `per §2.6 step 4` | §2.7 add an event listener step 5 | **4 → 5** |
| `event_target.rs:571` | `(§2.7.7 step 2)` | §2.7 removeEventListener method steps step 2 | 2 → 2 |
| `event_target.rs:592` | `Spec §2.7.7: only capture is read from options for removal` | §2.7 removeEventListener method steps step 1 ("flattening options") | — → 1 |
| `event_target.rs:599` | `WHATWG §2.6 step 4 forbids duplicates` | §2.7 add an event listener step 5 | **4 → 5** |
| `tests_remove_event_listener.rs:162` | `WHATWG DOM §2.7.7 only flattens capture for removal` | §2.7 removeEventListener method steps step 1 | — → 1 |
| `tests_remove_event_listener.rs:202` | `(WHATWG DOM §2.7.7 only flattens capture)` | §2.7 removeEventListener method steps step 1 | — → 1 |
| `tests_add_event_listener.rs:122` | `registration must be discarded per WHATWG DOM §2.6 step 4` | §2.7 add an event listener step 5 | **4 → 5** |
| `tests_add_event_listener.rs:151` | `duplicates per §2.6 step 4` | §2.7 add an event listener step 5 | **4 → 5** |
| `tests_add_event_listener.rs:177` | `§2.6 step 2: "If callback is null, then return."` | §2.7 add an event listener step 3: "If listener's callback is null, then return." | **2 → 3** |
| `event_listener.rs:90` | `WHATWG DOM §2.6: if true, the listener is automatically removed` (`once`) | §2.7 event listener "once (a boolean, initially false)" | — |
| `event_listener.rs:94` | `WHATWG DOM §2.6: if true, preventDefault() inside this listener …` (`passive`) | §2.7 event listener "passive (null or a boolean, initially null)" | — |

**(b) The concept-sweep siblings in other files (5 lines, 3 files)** — found by the Rev 2 attestation (N-3) with a
concept grep over dotted DOM `§2.x.y` citations. Command:
`git grep -nP '(DOM|WHATWG) §2\.\d+\.\d' -- crates | grep -vP 'HTML|§2\.4\.6|§2\.6\.3|§2\.7\.7'` → 5 lines
(positive control: without the final `grep -vP` → 14 lines — these 5, the 6 `§2.6.3` / `§2.7.7` lines of (a) and of
PR-A, and 3 `crates/dom/elidex-dom-compat/src/presentational.rs` lines (`:275`, `:339`, `:719`) citing "WHATWG
§2.4.6" for legacy HTML colour parsing, not DOM events): `crates/script/elidex-js/src/vm/host/event_source.rs:40` ("WHATWG DOM §2.7.2"),
`crates/script/elidex-js/src/vm/tests/tests_event_source.rs:733` ("§2.7.2 step 5"), `:800` ("DOM §2.7.5 step 2"),
`:815` ("§2.7.2 step 2"), `crates/script/elidex-js/src/vm/tests/tests_indexeddb_events.rs:287` ("WHATWG DOM §2.7.3").
`webref heading dom 2.7` → §2.7 with no subsections (`heading dom 2.7.2` → "no headings"), so each is drift of the
same chapter; not derived here. This grep sees only the dotted form; the sweep's own method ("ALL DOM §2.x")
covers the rest.

**(c)** `event_target.rs:131` "§2.7 step 8" (handleEvent) — the number resolves; the step is not re-derived here.

### §9.3 Definition of done

Every command is re-runnable at the PR head.

1. Red-before-fix recorded in the PR body: cells (e), (f), (g) of §6 run at the pre-fix head (the PR's first commit
   adds the tests; `cargo test -p elidex-js --all-features --lib -- tests_abort_dependent --test-threads=1` → (e)
   aborts the process, (f) and (g) fail) and green at the PR head.
2. The map is gone: `git grep -n 'any_composite_map' -- crates` → **no output** (positive control at `69fbe61f`:
   `git grep -n 'any_composite_map' -- crates | wc -l` → 21).
3. One writer: `git grep -nP 'source_signals|dependent_signals|\.dependent\b|is_dependent' -- crates ':!crates/dom/elidex-dom-api/src/abort.rs'`
   shows only reads (`.is_dependent()` / `.source_signals()` / `.dependent_signals()` accessor calls, docs, tests'
   assertions) — no `push` / `retain` / `clear` / `remove` on either set and no assignment to `dependent` outside
   the module (checked by reading the hits; all three fields are private, so a write outside the module does not
   compile — D-2).
4. No recursion — the body of `abort_signal` calls no `abort_signal`:
   `awk 'f{print FILENAME":"NR": "$0} f&&/^}/{f=0} /^[[:space:]]*(pub(\([a-z]+\))?[[:space:]]+)?fn[[:space:]]+(abort_signal|run_abort_steps)[<(]/{f=1}' crates/script/elidex-js/src/vm/host/abort.rs | grep -P '(?<![A-Za-z_])abort_signal\('`
   → **no output** (positive control at `69fbe61f`: `git show 69fbe61f:crates/script/elidex-js/src/vm/host/abort.rs
   | awk 'f{print NR": "$0} f&&/^}/{f=0} /^[[:space:]]*(pub(\([a-z]+\))?[[:space:]]+)?fn[[:space:]]+(abort_signal|run_abort_steps)[<(]/{f=1}' | grep -P '(?<![A-Za-z_])abort_signal\('`
   → 1 line, `610: abort_signal(ctx, composite_id, reason)?;`; the `awk` range is the bodies, `:523-658` there).
   (Rev 2's `git grep -nE '^\s*abort_signal\(ctx'` prints nothing even at `69fbe61f` — `\s` under `-E` — and had no
   function range.) The callers, `git grep -nP '(?<![A-Za-z_])abort_signal\(' -- crates` at `69fbe61f` (5 call
   lines + the definition `abort.rs:522` + a doc mention `fetch/mod.rs:294`): `native_abort_controller_abort`
   (`abort.rs:402`), the recursion (`:610`), `internal_abort_signal` (`:687`), the static `abort()`
   (`abort_statics.rs:39`) and `any()`'s already-aborted path (`abort_statics.rs:159`). At the PR head the last
   three are gone (`:610` by D-4, the two statics by D-5), so the expected callers are `native_abort_controller_abort`
   and `internal_abort_signal` only; `AbortSignal.timeout` reaches `abort_signal` only through
   `internal_abort_signal` (`natives_timer.rs:346-347`), not by a call of its own. The PR body lists the callers
   with their lines.
5. Layering: `git grep -nE 'DependentLinks|create_dependent_abort_signal|signal_abort\(' -- crates/script/elidex-js/src`
   shows calls and the trait impl only (no definition of a `struct` / `fn` with those names in `elidex-js`).
6. Citations (the census shapes of §0.2 item 9, crates-wide; `-P`, never `-E` with `\b` / `\s`):
   `git grep -nP '§3\.1(\.\d+)*(?![.\d])' -- crates | grep -iP 'abort|signal'` → exactly **3 lines**: `abort.rs:1`
   ("§3.1 Interface AbortController"), `object_kind.rs:294` (the `AbortController` variant) and the rewritten
   `globals.rs:719` line, whose mixed subject (AbortController constructor + AbortSignal global) cites "§3.1 Interface
   AbortController / §3.2 Interface AbortSignal" (D-8) — positive control at `69fbe61f`: 23 lines in 9 files. (Rev 2
   said "exactly 2", which its own `globals.rs:719` row contradicts.) The dotted form
   `git grep -nP '§3\.1(\.\d+)+' -- crates | grep -iP 'abort|signal'` → **no output** (control at `69fbe61f`: 16
   lines); `git grep -nP '§3\.1 / §7\.2' -- crates/script/elidex-js/src/vm/mod.rs` → **no output** (control: 1; the
   bare `§7\.2` grep hits 8 lines of other specs in that file and is not the check); the signal-option lines
   `git grep -nP '§2\.6\.3' -- crates` → **no output** (control at `69fbe61f`: 2 lines, `event_target.rs:110`,
   `:168`); `git grep -nP 'Fetch §5\.1' -- crates/script/elidex-js/src/vm/host/abort.rs` → **no output** (control: 1
   — the `step 13` is on the next line, so the check is the section alone);
   `git grep -nP '(?<!HTML )§8\.1\.8\.1' -- crates/script/elidex-js/src/vm/host/abort.rs` → **no output** (control:
   2). The handed-off shapes are **not** checked here — they are §9.2's, and §9.2 (a)'s command still prints
   21 lines at the PR head, which is the expected state.
7. Hygiene: `python3 .claude/tools/ecs_mission_ratchet.py` → `ok (501 allowlisted, 0 new, 0 stale)` — the count is
   unchanged because the line PR-A rewrites is not counted today (D-8); the PR body records the output and quotes
   `abort.rs:512` before and after.
8. `cargo test -p elidex-dom-api -p elidex-js --all-features` green (the cells of §6 present); `mise run ci` green
   before push; `cargo fmt --all` before each commit.
9. The PR body records the base sha, the guard memos' commits it was checked against (PR-1 / umbrella `4557c99b`;
   P0 / P0a `b7f03821`, P0a §4.4's order `d7ec4761`), §9.1's contents, and the squash sha is handed to VM P4 and the
   PM (hand-offs).
10. `/elidex-plan-review` on this memo before implementation (mandatory, §1); `/elidex-review` before push.
11. Overlap check at the head, recorded in the PR body: `for b in $(git branch -a --format='%(refname:short)' | grep
    -v HEAD); do o=$(comm -12 <(git diff --name-only origin/main...HEAD | sort) <(git diff --name-only
    origin/main...$b 2>/dev/null | sort)); [ -n "$o" ] && echo "== $b" && echo "$o"; done` — the expected hit is
    none from `vm-stack-safety-plan` while its PR-1 / P0a code is unwritten (its commits are memos), and none from
    `layout-dom-c0c-adopt-notification` (landed, #611); any hit is a rebase note, not a design question (§5.1
    "Overlaps"). Positive control built into the loop: `git branch -a` lists PR-A's own branch, whose diff
    intersected with itself is every file PR-A touches — that `== <PR-A branch>` block must appear (if it does not,
    the loop measured nothing).

## §10. Open questions

None. Rev 1's four questions were ruled by **PM 504** (2026-11-08) and the rulings are folded in:

- Q-1 → this memo is the umbrella and PR-A's plan (header, D-7; no separate umbrella file).
- Q-2 → the citation family is fixed in PR-A (D-8). Rev 2 read "the whole family" as 44 lines / 13 files; on the
  Rev 2 attestation the driver narrowed it to the `AbortSignal` subject — 23 lines / 10 files in PR-A, 26
  lines handed off to the DOM-events renumber sweep (§0.2 item 9, §9.2). The "ledger's literal scope" branch
  stays removed.
- Q-3 → D-3 (`reason: Option<JsValue>`) is in PR-A. The "include or defer" branch is removed.
- Q-4 → the two abort-algorithm stores are slot `#11-abort-algorithms-single-set` (registered PM 504, owner L3),
  **out of PR-A** (§8). The third defect found by measurement (dependents not aborted inside the source's `'abort'`
  listener, §0.2 item 5) is fixed in PR-A (D-4 (ii), §6 cell (g)).

Nothing awaits registration: PR-B's slot `#11-abort-signal-dependent-keepalive` is registered (PM 512), which D-7
required before the parent slot closes. No question is open on the spec reading: every step relied on is quoted in
§0.2 item 8 and D-8.

## Rev 2 changelog (plan-review R1, 2026-11-08; one line per item)

- F1 — §0.2 item 7 rewritten per entry point (native gate `interpreter.rs:360-365`; timer path `drain_timers` →
  `internal_abort_signal` runs with `gc_enabled = true`, `init.rs:855` / `pending_tasks.rs:366`); plan rooting stated
  as the guarantee in D-4, §2 A × C; VM cell (n) added (`force_gc_before_next_alloc`, `inner.rs:105`) with its mutant.
- F2 — D-8 hygiene and DoD 7: the ratchet does not count `abort.rs:512` (no `host/abort` allowlist row; `GENERIC` lacks
  the phrase); DoD = `0 new / 0 stale`, count unchanged; the vocabulary gap is `#11-ecs-mission-detector-vocabulary`'s.
- F3 — `event_target.rs:110` / `:168` "§2.6.3 step 3" → §2.7 add an event listener step 2; decided: every non-resolving
  DOM §2.x cite in the file joins (§2.6 step N ×8, §2.7.7 ×6), plus the siblings the crates-wide sweep found
  (`tests_remove_event_listener.rs` 2, `tests_add_event_listener.rs` 3, `event_listener.rs` 2).
- F4 — §0.2 item 9 census redone as a crates-wide sweep (P1 / P2 / P3 shapes + scope control 190), `mod.rs:1360` added,
  family = 44 lines / 13 files; DoD 6 uses the census shapes with controls.
- F5 — D-8 per-line old → new table (section and step) for all 44 lines, verbatim `webref body` quotes;
  `tests_fetch.rs:429` step 3 → 2 and its paraphrase replaced.
- F6 — PR-B = proposed slot `#11-abort-signal-dependent-keepalive` (§8 row text: defect, [spec], shape, prerequisite,
  trigger, acceptance, re-eval 2026-12-15); code docs cite the id; parent slot closes only once it is registered (D-7,
  hand-off 1).
- F7 — §1 "Why PR-A is not cut further": D-8 alone, D-3 alone, D-4 alone, flattening alone, GC prune alone — each with
  the failure (same edit set twice / strangler in time / wrong data model / function rewritten twice).
- M1 — §0.2 item 12: identifier grep (`AbortSignal|abort_signal|mod abort` → 0, exit 1), the loose `abort` grep's 5
  prose lines listed, `pub mod` control 19.
- M2 — `DependentLinks` fields all private, `is_dependent()` / `set_dependent()` (§4.1, D-2); DoD 3 extended.
- M3 — §4.1 `links(h) == None` contract (one constructor `abort.rs:322-332`; rows removed only in the sweep
  `collect.rs:1442-1443`); the 4.2.1 `debug_assert!` type-checks against `Option`.
- M4 — D-2: why a trait with one production impl beats a module-owned store (row split, two prune sites, a second
  side-store); the test graph is a second impl.
- M5 — §8: trigger + re-eval date for both slots.
- M6 — §10 Q-1..Q-4 resolved per PM 504; "defer" branches of Q-2 / Q-3 removed; D-7's open question removed.
- M7 — DoD 11 overlap check added; §5.1 "Overlaps" measured now.
- M8 — `tests_dispatch_integration.rs` moved to "Not touched" (reads `bound_listener_removals.len()` only, `:487-495`).
- M9 — `any_direct_composite_abort_does_not_touch_inputs` (`:280-301`) listed: stays green, comment rewritten.
- M10 — §5.1 / §9: PR-1 touches `vm/mod.rs`, `vm/object_kind.rs`, `vm/tests/mod.rs` (memo `:394`, `:745-746`,
  `:775` at `4557c99b`); L2 C0c branch edits `elidex-dom-api/src/lib.rs` — rebase-only, stated.
- M11 — guard-memo anchor `d6ce4e97` → `4557c99b` (tip `ce603c74`), A-1 `:104`, L-7 `:968`, W-11 `:245`, cell `:829`;
  PM 502 P0 named (header, §9, D-4).
- M12 — D-4 / §4.3: `Err` contract "run every plan entry, keep the first `Err`, return it after the loop", grounded in
  "signal abort" steps 5-6 and M4-12 policy (c); related to P0 (census line in the PR body); cell (o) added.
- M13 — D-3 grep → `git grep -nP '(?<![A-Za-z_])(s|state)\.(aborted|reason)\b' -- crates/script/elidex-js/src` (20
  lines: 13 field accesses listed, 2 comments, 5 JS-text); Rev 1's command noted as a silent zero.
- M14 — `tests_abort_statics.rs` 470 → 427 (items 10, §5.1).
- M15 — cell (m) placed in `tests_fetch.rs` (needs its harness); §5.1 row updated.
- M16 — §7 rebuilt as a per-file table with the touch-time record's owners (`:81-84`) and PR-A's range verdicts.
- M17 — `abort.rs:615-616` "Fetch §5.1 step 13" → Fetch §5.6 fetch(input, init) method steps step 11 (D-8).
- M18 — `abort.rs:499` / `:585` "§8.1.8.1" → HTML §8.1.8.1 Event handlers (D-8).
- M19 — `abort.rs:661` docstring names "signal abort" step 2 and the static abort(reason) step 2 (D-8 table).
- M20 — `keepalive.rs:84` paraphrase and D-7's predicate → the full §3.2.1 form (D-7, §4.4).
- M21 — §3 row: §3.2.1 is the normative requirement; D-7's §2.8 "not a general keepalive" reworded as an inference.
- M22 — folded into F1 (anchor `dispatch.rs:856-864` is the `Arguments` guard; `interpreter.rs:360-365` is the gate).
- Premises found wrong while re-measuring (recorded here, not written in): none of the 29 findings was wrong; two were
  incomplete — F3's list omitted the `§2.7.7` shape (6 + 2 lines) and the `§2.6` siblings in two more files, and
  M11's "byte-identical" holds for the A-1 / L-7 / W-11 / precondition text but adjacent Edges prose and line numbers
  moved.

## Rev 2.1 changelog (Rev 2 attestation, 2026-11-08; one line per item)

- N-1 (IMP) — D-4, §4.3, §2 A, §0.2 item 7: the plan is rooted by one `push_stack_scope` frame (`temp_root.rs:155`)
  with every handle pushed on its stack and the loop on `NativeContext::new_call(&mut frame)` (the
  `internal_abort_signal` shape, `abort.rs:681-688`); N `push_temp_root` guards are impossible (each holds
  `&mut VmInner`, `temp_root.rs:41`, `:76-80`; the doc rule `:131-137`).
- N-2 (IMP) — §6 (n): the GC is armed inside `t`'s listener by a test native (the `mid_delivery_gc.rs:105-129`
  shape), after planning — Rev 2's native arming fired at `build_dom_exception` (`natives_timer.rs:345` →
  `dom_exception.rs:311`) before planning; preconditions (flag consumed in the listener, `dep` planned), a kill
  assertion independent of the LIFO free list (`inner.rs:121-123`), and the mutant run's requirement that
  `fresh.signal`'s `ObjectId == dep`. §6 (o): renamed `abort_err_does_not_skip_dependents`, driven through
  `internal_abort_signal` directly — `drain_timers` drops a timeout fire's `Err` (`natives_timer.rs:291-293`,
  `:349-351`, `:358-359`).
- N-3 (IMP) — resolved for PR-A by the SCOPE ruling: the 5 concept-sweep lines are listed in §9.2 (b) with their
  command and control; owner = the DOM-events renumber sweep.
- N-4 (IMP) — DoD 6: P1 → exactly **3** lines (`abort.rs:1`, `object_kind.rs:294`, the rewritten `globals.rs:719`).
- N-7 (IMP) — DoD 4: an `awk` function-range + `grep -P` command with a positive control (`:610` at `69fbe61f`);
  callers re-measured (`abort.rs:402`, `:610`, `:687`, `abort_statics.rs:39`, `:159`; the two statics removed by D-5;
  the timeout path reaches `abort_signal` only via `internal_abort_signal`).
- N-5 (MIN) — the EventTarget scope controls corrected and moved with the lines to §0.2 item 9 / §9.2: union 17 (not
  19), unprefixed in the three DOM-subject files 15 (not 13), bare shape 63 lines in 19 files (not 20).
- N-6 (MIN) — D-4: `dispatch_vm_simple_event` never returns `Err` at `69fbe61f` (`dispatch_vm_event` `:271-347` has
  only `Ok`; listener exceptions are swallowed, `event_target_dispatch.rs:485-497`); the `Err` path is latent, the
  contract stays, the P0 hand-off now points at P0a §5.4 (`b7f03821`), which already records it.
- N-8 (MIN) — header: the requirement-diff command written out with `-P` and a positive control (no `\b`).
- F1 (MIN) — `internal_abort_signal` anchor → `abort.rs:681-688`.
- F4 (MIN) — the bare `§2.6` control: 19 files, the other 15 (§0.2 item 9).
- F5 (MIN) — §9.2 (a): `event_target.rs:377`'s doc covers add **and** remove — both method steps' step 1.
- F6 (MIN) — `#11-abort-signal-dependent-keepalive` is registered (PM 512, ledger `project_open-defer-slots.md:1360`):
  "proposed" removed (D-7, §8, §9 hand-off 1, §10); `#11-abort-algorithms-single-set` gains its ledger pointer (`:1353`).
- F7 (MIN) — §1: the D-8 cut enumerates the 9 of 23 lines inside rewritten blocks (Rev 2's "21 of 44" was not an
  enumeration); the D-3 cut "edits 5 latch / capture lines twice", not "a strangler".
- M10 (MIN) — §5.1 "Overlaps": PR-1's `mod regexp_compile;` line is memo `:404-405` (`:394` is the verdict line).
- M11 (MIN) — P0 umbrella and P0a cited at `b7f03821` (header, D-4, §9 Edges, §9.1); `a02777c9` / `2b9c4dce` not cited.
  The order PR-A → P0a (rebase; `fail_next_abort_dispatch` → `InjectAt::SimpleEventDispatch`) is written in §9 Edges
  from P0 §1 (`p0.md:113-118`), P0 §7 L-12 (`p0.md:334`), P0a §4.4 (`p0a.md:165-173`).
- SCOPE (driver) — PR-A's family = 23 lines (21 `AbortSignal`-subject + `event_target.rs:110`, `:168`); D-8's
  sub-family 2 cut to those two; the 26 others → §9.2 (21 + 5, path:line, commands, Rev 2's derivation kept as input);
  §0.1, §0.3, §1, §5.1 (three rows removed), DoD 6, hand-off 2, §10 Q-2 follow.
- ALSO — §9.1 (NEW) the PR body: the latent defect at `:592` (`Err` after the latch → later `abort()` a no-op,
  dependents never aborted) and `:610` (`Err` after `any_composite_map.remove` → remaining composites and the fetch
  observers stranded), the run-all / first-`Err` fix, and `fail_next_abort_dispatch` kept for P0a to re-home;
  hand-off (5): PR-A consumes part of the DOM-events renumber note.
- Also re-measured while editing: L2's C0c landed (#611 `0a5a77ba`) — §9 Edges and DoD 11 updated; D-3's 20-line
  breakdown corrected (`tests_abort_statics.rs:77` is a Rust comment: 13 / 3 / 4, not 13 / 2 / 5); the PR-1 and
  umbrella requirement lines located at `b7f03821` (A-1 `:82`, cell `:828`, L-7 `:952`, W-11 `:254`).

## Rev 2.2 changelog (Rev 2.1 attestation, 2026-11-08; one line per item)

- **Rev 2.2** (PM 538, Rev 2.1 attestation): IMP-1, MIN-2 to MIN-9 fixed by deletion or replacement only, nothing added; PM 540: P0a §4.4 at `d7ec4761`, seam order; its attestation's M-A to M-G likewise.
