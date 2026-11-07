# `AbortSignal.any` — dependent-signal flattening, spec-shaped signal abort, the DOM §3 citation family (program + PR-A per-PR plan-memo)

Slot: `#11-abort-signal-any-dependent-flattening` (registered PM 454, from the VM stack-safety guard census row
W-11; ledger text PM 458; owner L3). Base: `origin/main` `69fbe61f`. Every code fact below is measured at `69fbe61f`
unless marked; line numbers move, the commands do not. Owner lane: L3 DOM/form.

**What binds this memo.** CLAUDE.md "Design philosophy" (spec → ECS principles → evidence; the side-store→component
rule and its exception (a)), "Layering mandate" (`vm/host/` marshals; algorithms live in an engine-independent crate),
"Edge-dense work" (§1 here), "Spec citation" (every number ↔ title pair below is from `.claude/tools/webref`, run
2026-11-08). The VM stack-safety guard memos on the unlanded branch `vm-stack-safety-plan` at `d6ce4e97`
(`git branch -a --contains d6ce4e97` → that branch only; not on `origin/main`) name this slot as PR-1's one hard edge
(umbrella `docs/plans/2026-11-vm-stack-safety-guard.md` §4 row W-11 and the "Edges" paragraph; PR-1 memo
`docs/plans/2026-11-vm-stack-safety-guard-pr1.md` §2 A-1, §8 cell `abort_any_chain_depth_one_precondition`, §12 L-7).
What they require is listed in §9 and met in §4.

**Rev 1** (2026-11-08) — first draft.

## §0. Frame, premises, decisions

### §0.1 Frame

`AbortSignal.any(signals)` is specified as "create a dependent abort signal" (DOM §3.2): the result records the
**non-dependent** sources of its inputs (a dependent input contributes its own source signals, not itself), so no
dependent signal ever has dependent signals of its own and "signal abort" fans out exactly one level. elidex instead
links each input to the composite in a per-VM map (`VmInner::any_composite_map`) and propagates by recursion, so a
chain of `any()` calls is a chain of Rust frames (process abort) and a chain of GC-collectable intermediates (a lost
abort). The fix is the spec's data model and algorithm, placed in an engine-independent crate with the VM marshalling
to it; the same touch restructures `abort_signal` into the spec's "signal abort" / "run the abort steps" shape (which
also fixes an ordering defect found while measuring, §0.2 item 6) and re-derives the DOM §3 citation family that cites
`AbortSignal` under `§3.1.3` (a section that does not exist).

### §0.2 Premises re-measured at `69fbe61f`

1. **Where the state lives.** `crates/script/elidex-js/src/vm/host/abort.rs` (733 lines): `AbortSignalState`
   `:85-113` = `aborted: bool`, `reason: JsValue`, `bound_listener_removals: HashMap<ListenerId, DispatchTarget>`;
   rows in `VmInner::abort_signal_states: HashMap<ObjectId, AbortSignalState>` (`crates/script/elidex-js/src/vm/mod.rs:1279`,
   doc `:1267-1278`), inserted by `create_abort_signal` (`impl VmInner`, `abort.rs:322-332`). The `any()` links are a
   separate per-VM map `VmInner::any_composite_map: HashMap<ObjectId, Vec<ObjectId>>` (input → composites;
   `mod.rs:1381`, doc `:1365-1380`; constructed `crates/script/elidex-js/src/vm/init.rs:815`). `ObjectKind::AbortSignal`
   is payload-free (`crates/script/elidex-js/src/vm/object_kind.rs:273-292`). No abort state is an ECS component and
   none is in an engine-independent crate: `git grep -nE 'AbortSignal|AbortController|abort_reason|source_signals|dependent_signals' -- 'crates/**/*.rs' ':!crates/script/elidex-js/**'`
   → **no output** (positive control: the same grep without the exclusion → 254 lines for `\bAbortSignal\b` alone,
   `git grep -P '\bAbortSignal\b' -- 'crates/**/*.rs' | wc -l`).
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
7. **GC during the abort path.** Native calls run with `gc_enabled = false`
   (`crates/script/elidex-js/src/vm/dispatch.rs:856-864`), so no collection happens inside `controller.abort()` or
   the listeners it fires; the fetch fan-out nevertheless roots its promise across `reject_promise_sync`
   (`abort.rs:639-650`, "defensive root matches the surrounding codebase's invariant"). D-4 follows that invariant
   for the plan's handles (`VmInner::push_temp_root`, `crates/script/elidex-js/src/vm/temp_root.rs:41`).
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
   objects in APIs. There is **no §3.1.2 and no §3.1.3** (`webref heading dom 3.1` → §3.1 only). The dfns
   (`webref dfn dom …`): 'create a dependent abort signal', 'signal abort' (for=AbortSignal), 'dependent', 'source
   signals', 'dependent signals', 'abort algorithms', 'abort reason' → all §3.2 `#interface-AbortSignal`; 'signal
   abort' (for=AbortController) → §3.1. The drift measured
   (`git grep -nE '§3\.1(\.[0-9])+' -- <5 files>` → 15 lines; `git grep -nE 'DOM §3\.1[^.0-9]|WHATWG §3\.1[^.0-9]' -- crates`
   filtered to AbortSignal subjects → 5 lines; verified 2026-11-08 by those two commands), **20 lines in 9 files**,
   three shapes:
   - `§3.1.3` / `§3.1.3.1` / `§3.1.3.2` / `§3.1.3.3` for the statics and `any()` (should be §3.2): `abort_statics.rs`
     `:2`, `:21`, `:38`, `:45`, `:83`, `:128`, `:161` (the ledger's seven); `abort.rs:201`, `:594`;
     `natives_timer.rs:330`; `crates/script/elidex-js/src/vm/tests/tests_abort_statics.rs:2`, `:246`;
     `crates/script/elidex-js/src/vm/tests/tests_fetch.rs:429` — 13.
   - `§3.1.2 step 1` for "signal abort" step 2's default reason (should be §3.2 "signal abort" step 2): `abort.rs:538`,
     `:661` — 2.
   - `§3.1` for an AbortSignal subject (should be §3.2): `abort.rs:465` (`throwIfAborted`),
     `crates/script/elidex-js/src/vm/globals.rs:719`, `mod.rs:1242` (which also cites a non-existent `§7.2`;
     `webref heading dom 7.2` → none; §7 is Sets), `object_kind.rs:273`,
     `crates/script/elidex-js/src/vm/tests/tests_ui_event_constructor.rs:357` — 5.
   Correct and kept: `abort.rs:1-2`, `object_kind.rs:294` (AbortController, §3.1), `keepalive.rs:20`, `:84`, `:236`
   (§3.2 / §3.2.1), `event_target.rs` (§2.7). Premise correction: the ledger's "seven cites, one file" is the
   `abort_statics.rs` subset of a 20-line / 9-file family; "one family, one touch" (PM 458) is read as the whole
   family (D-8).
10. **Existing coverage.** `tests_abort_statics.rs` (470 lines) `any_*` tests `:215-396`; `any_composite_map_is_weak_bookkeeping`
    `:343-377` asserts the **current** non-spec behaviour on the map (dropped composite → map emptied) and is rewritten
    (§6); `any_chained_composites_propagate_through_chain` `:262-277` stays green; `tests_fetch.rs:427-432`
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
    No dependency on boa or any JS engine"; its `lib.rs` (132 lines) has no abort module (`git grep -n 'abort' -- crates/dom/elidex-dom-api/src`
    → no output; positive control: `git grep -n 'pub mod' -- crates/dom/elidex-dom-api/src/lib.rs` → 20 lines). `elidex-js`
    depends on it under the `engine` feature (`crates/script/elidex-js/Cargo.toml:36`; the `engine` list names it).
    `elidex-script-session` hosts `ListenerId` and the listener store but no abort state. The guard umbrella's
    "Edges" paragraph notes the mismatch "its home (`elidex-dom-api`) is not where the abort state lives today" — D-1
    / D-2 resolve it by a trait boundary, not by moving the state.
13. **1000-line precedent.** `docs/plans/2026-07-s5-6-flip-boa-deletion.md:1130`: `vm/mod.rs` "2749 (already >1000)
    | +16 … | <50, no seam obligation". §7 applies the same reading to the four already-over files this PR edits by
    fewer than 50 lines each.

### §0.3 Premise checks against the brief and the ledger

- "unbounded chain … walk is unbounded (stack)" — **true** (item 3, item 5).
- "with intermediates not retained, GC can collect them so `s.aborted` stays false" — **true** (items 4, 5); the
  retention rule it violates is not §3.2.1 (an unlistened intermediate may be collected under the spec) but the
  linkage: the spec never routes an abort *through* an intermediate.
- "Fix = spec flattening (depth 1 by construction)" — **true**; stated in §4.2 from the quoted steps.
- "algorithm in an engine-independent crate" — the home is decided in D-1; the state stays in the VM (D-2), which the
  umbrella flagged as the design question.
- "all seven `§3.1.3`-family cites in `abort_statics.rs`" — **the family is 20 lines in 9 files** (item 9, verified
  2026-11-08).
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
  `DependentLinks<H>` (NEW) = `{ dependent: bool, source_signals: Vec<H>, dependent_signals: Vec<H> }` (ordered,
  deduplicated — an Infra set) and a trait `AbortSignalGraph` (NEW) with associated `Handle: Copy + Eq + Hash` and
  `Reason: Copy`, methods `abort_reason(h) -> Option<Reason>` (`None` = not aborted; the spec's "aborted ⇔ reason not
  undefined"), `set_abort_reason(h, r)`, `links(h) -> Option<&DependentLinks<H>>`, `links_mut(h)`. `VmInner`
  implements it over `abort_signal_states` (`Handle = ObjectId`, `Reason = JsValue`); `AbortSignalState` gains a
  `links: DependentLinks<ObjectId>` field; **`any_composite_map` is deleted** (field, constructor, GC prune, docs,
  tests: `git grep -l 'any_composite_map' -- crates` → 8 files, 21 lines at `69fbe61f`). Side-store→component rule:
  the state is keyed by a VM `ObjectId` and the sets hold `ObjectId`s — per-VM identity handles, exception (a); there
  is no entity to carry a component (an `AbortSignal` is a VM object by the EventTarget dispatch core's `VmObject`
  home, `abort.rs:27-44`), so the per-VM row is the correct home today, as `keepalive.rs:29-33` records for the
  same class of object.
- **D-3 `aborted: bool` + `reason: JsValue` → `reason: Option<JsValue>`; `aborted()` is derived.** Grounds: the spec
  defines aborted as "abort reason is not undefined", and step 2 of both "signal abort" and the static `abort()`
  materialises `undefined` into an `AbortError` `DOMException`, so an aborted signal's reason is never `undefined`;
  two fields carried one fact under a convention. Every reader is in this PR's edit set
  (`git grep -nE '\.aborted\b|\.reason\b' -- abort.rs abort_statics.rs event_target.rs fetch/mod.rs gc/trace.rs tests_dispatch_integration.rs`
  → 6 non-comment lines at `69fbe61f`; the `reason` getter maps `None` → `Undefined`).
- **D-4 `abort_signal` becomes the spec's two algorithms, iterative.** `elidex_dom_api::abort::signal_abort` (NEW) `(graph, signal, reason) -> Option<AbortPlan<H>>`
  performs steps 1-4 (returns `None` when already aborted; otherwise sets the signal's and each non-aborted
  dependent's reason and returns `AbortPlan { signal, dependents }` (NEW), the ordered list of steps 5-6). The VM's
  `abort_signal` then: materialises the default reason first (step 2's "otherwise" branch is engine-bound — a
  `DOMException` object), calls `signal_abort`, roots every handle of the plan with `push_temp_root` for the rest of
  the call (item 7; a recycled `ObjectId` must never receive another signal's abort), and runs
  `run_abort_steps(ctx, h)` (NEW) for the signal and then for each dependent — a loop, no recursion. `run_abort_steps`
  is "run the abort steps": the two algorithm stores first (detach bound listeners; reject and cancel the fetches),
  then fire `'abort'`. Consequences: (i) depth 1 by construction (§4.2); (ii) `d.aborted` is `true` inside the
  source's listeners (item 5, third repro); (iii) the fetch rejection moves from after the fire to before it — the
  spec's order; script cannot observe the difference (promise reactions are microtasks, `cancel_fetch` is a broker
  message), the internal order is stated and tested on the plan, not on timing. `internal_abort_signal` and
  `native_abort_controller_abort` keep their signatures (callers unchanged).
- **D-5 The reason-only paths set the reason only.** "create a dependent abort signal" step 2 and the static
  `abort(reason)` step 2 set the result's reason and return; today both call `abort_signal` on the fresh signal
  (`abort_statics.rs:39`, `:159`), which fires `'abort'` at an object that cannot yet have listeners or algorithms
  (unobservable). PR-A sets the reason through the graph and does not run abort steps; the result of step 2 is **not**
  dependent (`dependent` stays false, `source_signals` empty — step 3 is not reached), which §3.2.1 (PR-B) relies on.
- **D-6 Sets are ordered and deduplicated; a signal's `dependent_signals` is emptied after planning.** "Append" to an
  Infra set is a no-op for a present member, so `any([a, a])` records one source and one dependent (today the map
  holds two entries and the latch hides it, `tests_abort_statics.rs:304-321`). After `signal_abort` has planned a
  signal's dependents they are all aborted (step 4) and the set is never read again (step 1 returns on the next
  call), so it is emptied — the weak set has no observable content, and this is today's `remove(&signal_id)`
  (`abort.rs:607`) in the new shape. `source_signals` is left as the spec has it (PR-B's predicate reads it).
- **D-7 Program of two PRs; this memo is the program table and PR-A's plan.** §1 verdict: edge-dense. **PR-A** (this
  memo): D-1 – D-6, D-8, D-9, the tests of §6. **PR-B** (own per-PR memo, after PR-A lands): the §3.2.1 keepalive arm
  `KeepaliveClass::AbortSignalDependent` (NEW) in PR-B on the existing seam (item 11), its registrant loop over
  `abort_signal_states` rows with `links.dependent && !links.source_signals.is_empty()`, the listener half through
  `vm_path_has_listener(vm, id, "abort", false)` (`crates/script/elidex-js/src/vm/host/event_target_dispatch_vm.rs:79-84`)
  and the "abort algorithms is non-empty" half over the two stores; its engine-independent predicate function in the
  same `abort.rs` module (the `elidex_api_ws::ws_keepalive(state, has_listener)` shape,
  `crates/api/elidex-api-ws/src/websocket.rs:162`). PR-B is a behaviour change in the leak direction (DOM §2.8:
  listener presence is not a general keepalive; the arm must gate on `dependent ∧ sources non-empty`), which is why it
  is its own reviewed unit and why PR-A does not partially implement it. The slot closes at PR-A's landing (the
  ledger's defects are PR-A's; PR-1's hard edge is on PR-A); PR-B is proposed as its own slot (§8, PM registers).
  Open to the PM whether the two program rows warrant a separate umbrella file (§10 Q-1); the content of an umbrella
  for two PRs is §1's table and §9's order, both here.
- **D-8 The whole citation family, in PR-A.** All 20 lines of item 9 are re-derived: `§3.1.3*` → `§3.2 Interface
  AbortSignal` with the algorithm named ("static any(signals) method steps", "create a dependent abort signal step N",
  "static abort(reason) method steps step 2", "static timeout(milliseconds) method steps step 3.1"); `§3.1.2 step 1` →
  `§3.2 "signal abort" step 2`; `§3.1` on an AbortSignal subject → `§3.2`; `mod.rs:1242`'s `§7.2` is dropped (the
  "EventTarget but not a Node" fact is the IDL `interface AbortSignal : EventTarget`, §3.2). PR-1's L-7 check
  (`git grep -n '§3\.1\.3' -- crates/script/elidex-js/src/vm/host/abort_statics.rs` → 0 lines) is a subset of DoD 6.
  Hygiene in the same touch: `abort.rs:512` "matching browsers" is rewritten on spec grounds [spec] (the spec removes
  no listener at abort and fires once because step 1 returns on a second call; no browser is a ground) — the
  repository's ratchet (`python3 .claude/tools/ecs_mission_ratchet.py`, 2026-11-08: `ok (501 allowlisted, 0 new, 0
  stale)`) counts that line today and must count one fewer after.
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
| **PR-A** (this memo) | D-1 – D-6, D-8, D-9: the module, the trait, flattening, the iterative spec-shaped `abort_signal`, `any_composite_map` deleted, the 20 citations, hygiene; tests of §6 | — | VM stack-safety guard PR-1 (hard edge, §9) |
| **PR-B** (own memo) | DOM §3.2.1: `KeepaliveClass::AbortSignalDependent` (NEW) in PR-B + the engine-independent predicate; the GC cell "listenered unreferenced dependent survives and fires" | PR-A (`dependent`, `source_signals`) | — (no consumer waits) |

PR-A is the terminal unit of this program (base case: a narrowly-scoped per-PR slice under the program table, through
`/elidex-plan-review` before implementation). Coverage breadth K = 1 spec, M = 17 rows (§3): single-PR scope for PR-A.

Not in PR-A: §3.2.1 (PR-B); the sequence conversion of `any()`'s argument; the unification of the two abort-algorithm
stores; any ECS component (no entity exists for a signal, D-2).

## §2. Coupled invariants

Axes: **A** handle identity (an `ObjectId` is recycled by the free list after a sweep; a set or plan holding a dead
handle must never be read as the new occupant), **B** GC weak sets (source / dependent signals root nothing; dead
members are pruned at the sweep; nothing new is traced), **C** propagation order and re-entry (steps 3-6 in the spec's
order; listeners run JS that may abort other controllers, add listeners, call `any()`), **D** the crate boundary (the
algorithm reads and writes only reasons and links through the trait; `DOMException` creation, listener detach, fetch
rejection and the event fire stay in `vm/host/`).

- **A**: links hold handles only; the sweep tail prunes dead handles from every surviving row's two sets in the same
  pass that prunes dead rows (`collect.rs:1442-1443` today prunes rows; the set prune replaces `:1507-1522`), so no
  set holds a dead handle after a sweep; a plan's handles are temp-rooted for the duration of the abort call (D-4).
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
| A × C | the plan's handles are rooted before `run_abort_steps` runs JS; with `gc_enabled = false` inside natives (§0.2 item 7) the root is a belt on a buckle, as `abort.rs:639-650`'s is |
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
| WHATWG DOM §3.2.1 Garbage collection | the GC note (non-aborted ∧ dependent ∧ sources non-empty ∧ (abort listener ∨ algorithms non-empty) → must not be collected) | listenered unreferenced dependent | **PR-B**: `KeepaliveClass::AbortSignalDependent` (NEW) in PR-B; PR-A only supplies `dependent` / `source_signals` | ✗ (PR-B) | no |
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

pub struct DependentLinks<H> { pub dependent: bool, source_signals: Vec<H>, dependent_signals: Vec<H> }
impl<H: Copy + Eq> DependentLinks<H> {
    pub fn source_signals(&self) -> &[H];           pub fn dependent_signals(&self) -> &[H];
    fn append_source(&mut self, h: H);              fn append_dependent(&mut self, h: H);   // set semantics
    pub fn retain_live(&mut self, alive: impl Fn(H) -> bool);                                // sweep prune (B)
    fn take_dependents(&mut self) -> Vec<H>;                                                  // D-6
}
pub trait AbortSignalGraph {
    type Handle: Copy + Eq + core::hash::Hash;
    type Reason: Copy;
    fn abort_reason(&self, h: Self::Handle) -> Option<Self::Reason>;   // None ⇔ not aborted (or no such row)
    fn set_abort_reason(&mut self, h: Self::Handle, reason: Self::Reason);
    fn links(&self, h: Self::Handle) -> Option<&DependentLinks<Self::Handle>>;
    fn links_mut(&mut self, h: Self::Handle) -> Option<&mut DependentLinks<Self::Handle>>;
}
pub enum DependentCreation<R> { AlreadyAborted(R), Linked }
pub fn create_dependent_abort_signal<G: AbortSignalGraph>(g: &mut G, result: G::Handle, signals: &[G::Handle]) -> DependentCreation<G::Reason>;
pub struct AbortPlan<H> { pub signal: H, pub dependents: Vec<H> }
pub fn signal_abort<G: AbortSignalGraph>(g: &mut G, signal: G::Handle, reason: G::Reason) -> Option<AbortPlan<G::Handle>>;
```

`create_dependent_abort_signal` is steps 2-5 verbatim (`result` is the host's step-1 object): step 2 returns
`AlreadyAborted(reason)` without touching `result`'s links; step 3 sets `dependent`; step 4 appends through the set
primitives, step 4.2.1 as `debug_assert!(g.abort_reason(s).is_none() && !g.links(s).dependent)`. `signal_abort`
is steps 1-4: `None` if `abort_reason(signal).is_some()`; else set the reason, take the dependents (D-6), set each
non-aborted dependent's reason to the same `reason` value, return the plan. The module's unit tests use a
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
- `abort_signal(ctx, signal_id, reason)`: `if ctx.vm.abort_reason(signal_id).is_some() { return Ok(()) }` (step 1
  before materialising, as today's `:528-535`); materialise; `let Some(plan) = signal_abort(ctx.vm, signal_id, r)`;
  root `plan.signal` and each dependent via `push_temp_root` (guards held to the end); `run_abort_steps(ctx,
  plan.signal)?; for d in plan.dependents { run_abort_steps(ctx, d)?; }`.
- `run_abort_steps(ctx, id)` (NEW, `pub(super)` is not needed — private): step 1 = the two algorithm stores in
  today's order within each (`bound_listener_removals` → `abort_listener_back_refs` prune → `detach_bound_listeners`;
  then the `fetch_abort_observers` block moved up from `:627-655`); step 2 = both emptied (they are drained by
  `take` / `remove` today); step 3 = `dispatch_vm_simple_event(ctx, id, abort, false, false)`. A row missing for `id`
  (impossible after rooting; kept as the `let Some(..) else { return Ok(()) }` today's `:555-557` has) is a no-op.
- `internal_abort_signal` unchanged. Module docs `abort.rs:17-25`, `:43-44`, `:82-113`, `:493-521`, `:594-606` and
  `abort_statics.rs:1-12`, `:82-95` rewritten for the new shape; the 20 citations (D-8).

### §4.4 GC (`gc/collect.rs`, `gc/roots.rs`, `gc/trace.rs`, `gc/keepalive.rs`)

- `collect.rs:1507-1522` (the map) → after the row prune at `:1442-1443`: `for state in abort_signal_states.values_mut()
  { state.links.retain_live(|h| bit_get(marks, h.0)); }`. Same pass, same mark bits (A × B).
- `roots.rs:257-261` and note (k) `:600-618` rewritten: the sets are weak by spec; the §3.2.1 root is PR-B's arm on
  the keepalive seam (pointer). `keepalive.rs:83-92`'s "future arm" sentence is updated to name PR-B and the fields it
  reads (no arm added in PR-A).
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
| `crates/script/elidex-js/src/vm/host/abort.rs` | 733 | `AbortSignalState` (D-2, D-3), `impl AbortSignalGraph for VmInner`, `abort_signal` restructured, `run_abort_steps` (NEW), docs, citations `:201`, `:465`, `:538`, `:594`, `:661`, `:512` hygiene |
| `crates/script/elidex-js/src/vm/host/abort_statics.rs` | 181 | `any` on the algorithm; `abort` reason-only; docs; the seven citations |
| `crates/script/elidex-js/src/vm/host/event_target.rs` | 734 | `:172-181` reads `reason.is_some()` (D-3); no other change |
| `crates/script/elidex-js/src/vm/host/fetch/mod.rs` | 305 | `:298-305` `pre_flight_abort_reason` returns `state.reason` (D-3) |
| `crates/script/elidex-js/src/vm/mod.rs` | 2785 | `any_composite_map` field + doc deleted (`:1365-1381`); `abort_signal_states` doc (`:1267-1278`); citation `:1242` |
| `crates/script/elidex-js/src/vm/init.rs` | 858 | `:815` deleted |
| `crates/script/elidex-js/src/vm/object_kind.rs` | 1645 | docs `:273-292` |
| `crates/script/elidex-js/src/vm/globals.rs` | — (one comment line, `:719`) | citation |
| `crates/script/elidex-js/src/vm/natives_timer.rs` | 440 | citation `:330` |
| `crates/script/elidex-js/src/vm/gc/collect.rs` | 2074 | `:1507-1522` → the set prune (§4.4) |
| `crates/script/elidex-js/src/vm/gc/roots.rs` | 619 | `:257-261`, `:600-618` docs |
| `crates/script/elidex-js/src/vm/gc/trace.rs` | 1255 | `:385-398` (`Option` reason; comment) |
| `crates/script/elidex-js/src/vm/gc/keepalive.rs` | 434 | `:83-92` doc names PR-B |
| `crates/script/elidex-js/src/vm/tests/tests_abort_statics.rs` | 470 | header `:2`, `:246` citations; `any_*` tests rewritten in place (§6) |
| `crates/script/elidex-js/src/vm/tests/tests_abort_dependent.rs` (NEW) | — | §6 cells e-k |
| `crates/script/elidex-js/src/vm/tests/mod.rs` | 207 | `mod tests_abort_dependent;` |
| `crates/script/elidex-js/src/vm/tests/tests_fetch.rs` | — (one comment line, `:429`) | citation |
| `crates/script/elidex-js/src/vm/tests/tests_ui_event_constructor.rs` | — (one comment line, `:357`) | citation |
| `crates/script/elidex-js/src/tests_dispatch_integration.rs` | — (`:492-495` reads a row) | follows D-3 if it reads `.aborted` (measured: it reads `abort_signal_states` presence; re-check at the head) |

Not touched, with the reason: `crates/script/elidex-js/src/vm/tests/tests_abort.rs` (950 — new cells go to the new
file, D-9); `crates/script/elidex-js/src/vm/inner.rs`, `fetch_tick.rs`, `event_target_dispatch_vm.rs` (read the
unchanged stores); `crates/dom/elidex-dom-api/Cargo.toml` (no new dependency: the module uses `core` and `std`
only); `deny.toml`; `scripts/`, `.claude/tools/`. The file list is the union of `git grep -l 'any_composite_map' -- crates`
(8 files, verified 2026-11-08) and the citation files of §0.2 item 9 (9 files, verified 2026-11-08) plus the two NEW
files and the three D-3 reader files; re-run
both greps at the implementation head.

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
- (m) `fetch_rejection_runs_as_abort_step` — in `tests_fetch.rs`'s harness: a fetch bound to `c.signal`; `c.abort()`
  → the promise rejects with the reason (existing behaviour); the internal order is asserted by the plan (the stores
  are empty when the `'abort'` listener runs: `vm.inner.fetch_abort_observers` has no entry for the signal inside the
  listener — observed through a native-side probe, not JS).

Rewritten in place (`tests_abort_statics.rs`): `any_duplicate_input_fires_composite_only_once` (`:304-321`) keeps its
assertion and gains "one source"; `any_composite_map_cleaned_after_input_aborts` (`:324-340`) → asserts
`links(a.signal).dependent_signals` emptied (D-6) and `b.signal`'s intact; `any_composite_map_is_weak_bookkeeping`
(`:343-377`) → (i) above (moved); `any_composite_with_user_ref_survives_gc_and_propagates_abort` (`:379-396`) keeps
its behaviour with the map assertion replaced by a links assertion. Stay green unchanged:
`any_chained_composites_propagate_through_chain` (`:262-277`), `any_two_inputs_*`, `any_three_inputs_*`,
`abort_signal_any_*`, `tests_fetch.rs` `fetch_composite_signal_any_pre_abort_rejects`, `tests_abort_signal_option.rs`,
`tests_dispatch_integration.rs:447-495`.

Guards: DoD 2-4 greps. No WPT subset exists for `dom/abort` in this repository (§0.2 item 10); the cells above are
the supported-surface regression set for `AbortSignal.any`.

## §7. 1000-line audit (CLAUDE.md "1000-line debt")

Files over 1000 that PR-A edits: `vm/mod.rs` (2785, about −17 / +3: a field and its doc deleted, one doc rewritten,
one citation), `gc/collect.rs` (2074, about −16 / +6), `gc/trace.rs` (1255, ±3), `object_kind.rs` (1645, docs only).
Each is well under 50 added lines — the reading `docs/plans/2026-07-s5-6-flip-boa-deletion.md:1130` gives ("<50, no
seam obligation") — and none has a cohesion seam this PR would be the one to cut (`VmInner`'s field block and the sweep
tail are the same monolith every VM PR edits by a few lines). `abort.rs` goes from 733 to about 760; `tests_abort.rs`
(950) is not edited (D-9). No split is a prerequisite of PR-A.

## §8. Out of scope, and slots

- `#11-webidl-sequence-dense-array-fast-path` (open; ledger `:276`) and `#11-vm-webidl-sequence-early-exit-drain`
  (open): the `any()` argument's conversion (today an `ObjectKind::Array` read with no `Symbol.iterator`) is theirs;
  PR-A keeps the read and the comment that names them replaces `abort_statics.rs:102-107`'s "PR5a2" note (that PR,
  #85, landed without it).
- **PR-B** (proposed slot, PM registers; owner L3): DOM §3.2.1 on the keepalive seam (D-7). Defect today: an
  unreferenced dependent with an `'abort'` listener is collected and the listener never fires. Prerequisite: PR-A.
- **Candidate slot, PM decides** (`feedback_defer-slot-eligibility-audit-at-create`): the spec's single "abort
  algorithms" set is two stores (`bound_listener_removals` per row; `fetch_abort_observers` + `fetch_signal_back_refs`
  per VM) run by `run_abort_steps` in sequence. PR-A puts them behind one function; folding them into one
  `AbortAlgorithm` enum on the row is a representation change with its own GC and fetch-tick edges, not this slot's
  defect. Repeat-signal: none yet (one site runs them).
- `#11-eventtarget-keepalive-component-migration` (B1-gated): unchanged; D-2's exception (a) is the same reading
  `keepalive.rs:29-33` records.
- `#11-dom-walkers-iterative-over-ecs-links` (L3): unrelated — no DOM walker is on the abort path; the guard
  umbrella's edge from it to PR-1 is soft (PM 483 correction), and PR-A has no file overlap with it.
- `#11-vm-call-depth-guard` / PR-1: the hard edge (§9); PR-A adds no probe and needs none (§4.2).
- The `AbortSignal.timeout` keepalive membership and the timer path: unchanged except `natives_timer.rs:330`.

## §9. Edges, hand-offs, definition of done

**Edges.**

- **VM stack-safety guard PR-1 (VM P4; memos at `d6ce4e97` on `vm-stack-safety-plan`, unlanded) — hard edge, PR-A
  before PR-1.** What PR-1 requires of this slot and how PR-A meets it: (1) A-1 "a dependent signal's source signals
  are never dependent … the fan-out is depth 1 by construction" — §4.2; (2) PR-1 §5.3 treats `abort_signal` as a cut
  point with no probe — §4.2 (no Rust recursion on the path; JS re-entry is CP-1's); (3) PR-1's cell
  `abort_any_chain_depth_one_precondition` (10 000 kept, 2 MiB thread) must be green after PR-A — §6 (e) is the same
  shape; (4) PR-1 §12 L-7: `git grep -n '§3\.1\.3' -- crates/script/elidex-js/src/vm/host/abort_statics.rs` → 0
  lines — DoD 6 (a superset); (5) PR-1's PR body records `git merge-base --is-ancestor <PR-A squash sha> HEAD` → hand
  the sha to VM P4 at landing. PR-1 does not edit `abort.rs` / `abort_statics.rs` (umbrella §4 row W-11: "not touched
  by PR-1"), so no file overlap. The umbrella's "its own design may take longer than its one-line remedy suggests" is
  answered by D-1 / D-2 (a trait, not a move of the state).
- **L3's other work** (`#11-event-path-window-missing` plan, the walker umbrella): no shared file (those edit
  `HostDriver` / event-path and ECS walkers; PR-A edits none of them). Order per PM 485: #607 → **this slot** →
  stale-entity → walker umbrella.
- **VM P4's open PRs** (S1 string-replace, Slice E/F, guard memos): none edits the files of §5.1 as far as their
  memos state (their memos were not re-read for this; the DoD re-runs `git diff --stat` against the base at the head,
  and any overlap is a rebase, not a design question).
- **PR-B** (this program): after PR-A; its memo cites PR-A's squash sha and re-measures `keepalive.rs`.

**Hand-offs** (for the PM; the memo edits no ledger): at PR-A's landing close `#11-abort-signal-any-dependent-flattening`
with the PR number and squash sha; correct the ledger's "seven cites in `abort_statics.rs`" to the 20-line / 9-file
family (§0.2 item 9) and note that PR-A re-derived all of them; register PR-B as a slot (or accept D-7's program row
as its registration); decide the candidate slot of §8; tell VM P4 the squash sha (PR-1 A-1 ancestry check and L-7) and
that the §3.2.1 arm is PR-B, not in PR-A, so PR-1's `abort_any_chain_depth_one_precondition` depends on PR-A only.

**Definition of done** (every command re-runnable at the PR head):

1. Red-before-fix recorded in the PR body: cells (e), (f), (g) of §6 run at the pre-fix head (the PR's first commit
   adds the tests; `cargo test -p elidex-js --all-features --lib -- tests_abort_dependent --test-threads=1` → (e)
   aborts the process, (f) and (g) fail) and green at the PR head.
2. The map is gone: `git grep -n 'any_composite_map' -- crates` → **no output** (positive control at `69fbe61f`:
   `git grep -n 'any_composite_map' -- crates | wc -l` → 21).
3. One writer: `git grep -nE 'source_signals|dependent_signals' -- crates ':!crates/dom/elidex-dom-api/src/abort.rs'`
   shows only reads (`.source_signals()` / `.dependent_signals()` accessor calls, docs, tests' assertions) — no
   `push` / `retain` / `clear` / `remove` on either set outside the module (checked by reading the hits; the module
   keeps the `Vec`s private so a write outside it does not compile).
4. No recursion: `git grep -nE '^\s*abort_signal\(ctx' -- crates/script/elidex-js/src/vm/host/abort.rs` → **no
   output** inside `abort_signal`'s own body (today `:610`; the remaining callers are `native_abort_controller_abort`,
   `internal_abort_signal`, and `abort_statics.rs`'s `timeout` path — listed in the PR body with their lines).
5. Layering: `git grep -nE 'DependentLinks|create_dependent_abort_signal|signal_abort\(' -- crates/script/elidex-js/src`
   shows calls and the trait impl only (no definition of a `struct` / `fn` with those names in `elidex-js`).
6. Citations: `git grep -nE '§3\.1(\.[0-9])+' -- crates` → **no output** on an AbortSignal subject (positive control
   at `69fbe61f`: 15 lines across the 5 files of §0.2 item 9), and `git grep -nE '(DOM|WHATWG) §3\.1[^.0-9]' -- crates`
   shows only AbortController subjects (today's `abort.rs:1`, `object_kind.rs:294` and the 5 rewritten lines now
   reading §3.2); `git grep -n '§7\.2' -- crates/script/elidex-js/src/vm/mod.rs` → **no output**.
7. Hygiene: `python3 .claude/tools/ecs_mission_ratchet.py` → `ok` with `0 new` and the allowlisted count **one lower**
   than the base's 501 (the `abort.rs:512` line is gone, D-8); the PR body records both numbers.
8. `cargo test -p elidex-dom-api -p elidex-js --all-features` green (the cells of §6 present); `mise run ci` green
   before push; `cargo fmt --all` before each commit.
9. The PR body records the base sha, the guard memos' commit it was checked against (`d6ce4e97`), and the squash sha
   is handed to VM P4 and the PM (hand-offs).
10. `/elidex-plan-review` on this memo before implementation (mandatory, §1); `/elidex-review` before push.

## §10. Open questions

- **Q-1 (PM)** Program shape: D-7 puts the two-row program table in this memo (PR-A's plan). Does the PM want a
  separate umbrella file for a two-PR program whose second PR is one arm on an existing seam, or is §1's table the
  umbrella?
- **Q-2 (PM)** The ledger's citation count: 20 lines / 9 files (§0.2 item 9, verified 2026-11-08), not 7 / 1. D-8
  fixes all in PR-A. If
  the PM prefers the ledger's literal scope, the 13 lines outside `abort_statics.rs` become a follow-up — this memo
  recommends against (one family, one touch).
- **Q-3 (driver / review)** D-3 (`reason: Option<JsValue>`) is a representation change beyond the ledger's defects;
  grounds in D-3. Include or defer?
- **Q-4 (PM)** The candidate slot of §8 (one abort-algorithms set) — register, or leave as a note until a second site
  runs the algorithms?
- No question is open on the spec reading: every step relied on is quoted in §0.2 item 8.
