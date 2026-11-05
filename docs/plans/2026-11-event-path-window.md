# `#11-event-path-window-missing` — the event path reaches the Window: one DOM *get the parent* (per-PR plan memo)

Per-PR plan for the slot `#11-event-path-window-missing` (ledger row registered 2026-11-05, PM 377; direction PM 378:
one DOM *get the parent* function whose Document arm returns the Document's relevant global object except for `load`,
not a Window special case beside the walk). Owner L3, the unit after RC-2a (#596 `17b2a84d`). Written against
`origin/main` = `e8b6a24d` (#599, the `vm/host/document.rs` split); every coordinate below is from that commit.
**PM decision 132 is kept** (Layout T3 D-T3: the active document and its Window are browsing-context state, marshalled
per call, not a component; their component form is B1's, `#11-browsing-context-state-ecs-components`). This memo adds
no component and no store.

**Defect.** `build_propagation_path` (`crates/script/elidex-script-session/src/event_dispatch/mod.rs:203`) walks
`EcsDom::get_parent` to the tree root, and the Window entity has no `TreeRelation`
(`crates/core/elidex-ecs/src/dom/mod.rs:491`), so the Window is never on a node-targeted event's path. Runtime
probe (Layout, `f9cf8c40`, VM `dispatchEvent`): capture and bubble `click` listeners on window, document and body fire
`d-cap b-cap b-bub d-bub` (DOM §2.9 order: `w-cap d-cap b-cap b-bub d-bub w-bub`); `composedPath()` during dispatch
is `[body, html, document]`. Every `window.addEventListener` for a node-targeted event never fires — `click`,
`keydown`, `submit`, `input`, `DOMContentLoaded`, and so on.

**Scope.** One PR, the path only: `get_the_parent` (D-1) reading the one existing browsing-context fact through a
resolver argument (D-2), the path built once per dispatch (D-3), the dead `composed_path_for_js` deleted (D-6). Not in
scope: handler-target determination (T3's, D-8), any Document↔Window component or store (PM 132), the *legacy target
override flag*, closed-tree hiding in `composedPath()` (P2's, D11), `relatedTarget` / touch retargeting, slot
assignment on the path (P2), the at-target two-pass order — §7.

**Changelog.**
- *Rev 2* (2026-11-06): structural rewrite after `/elidex-plan-review` R1 (five reviewers, 2026-11-05; dispositions in
  the PM handoff `l3-evpath-plan-r1-dispositions.md`). Ax5 CRIT (D-2 `AssociatedWindow` reversed PM 132) → D-2 dropped,
  no component, no store. Ax5 CRIT (D-3 entered T3's scope) and Ax4 CRIT (D-3 kept the handler on the element where
  HTML §8.1.8.1 step 3 returns null) → D-3 dropped; `resolve_handler_target`, the body WEH accessors,
  `EcsDom`'s `window_entity`, the `ScriptEngine` trait item and the shell lookups are untouched; the boundary is recorded
  at D-8. Ax2 IMP ×3 (two homes, rebind asserts, detached-body regression) → gone with D-2 / D-3. Ax1 IMP (two copies of
  target resolution) → gone. Ax3 IMP (PR-A not a base-case slice) → one PR (§1). Ax5 IMP (D-11 split bundled) → D-11
  dropped (984 lines, `dom/mod.rs` is not touched). Ax4 T-4 → at-target items run in the bubble pass regardless of
  `bubbles` (DOM §2.9 step 6.14.1), expectation corrected; step 6.9.7 (was "6.7"); `composedPath()` hiding = steps
  7–16 (was "10–15"); *retarget* steps 1 and 2 cited by step; HTML §13.2.7 `load` step is 11.5 (was "9.5", a drift).
  Ax3 MINs → every slot carries owner / trigger / acceptance (§7); the override-flag deferral re-measured (the firing
  site has `document`; the reason is now "target semantics, not path", not "needs the inverse lookup"); dead
  `composed_path_for_js` deleted (D-6); the shell's pre-bind fallback accounted for (D-2, §3.1). Ax5 edges with C0b /
  C0c / T3 / C1-pre / CV P2 D11 → §8. H-14 widened to every local and remote branch and to every file this PR touches or
  reads (H-14). A-* tests, rebind asserts, the PR-A / PR-B split, Q-1 / Q-2 removed. §2, §3, §6, §8, §9 re-derived.
  Coordinates re-pinned at `e8b6a24d` (no cited line moved with #599: the split touched `vm/host/document.rs` only).
- *Rev 1* (2026-11-05): first draft.

## §0. Frame, premises, decisions

### §0.1 Coordinate frame

Coordinates are `e8b6a24d`. At the implementation base (§9 item 1) every coordinate is re-measured with the commands in
§0.2; the counts are re-derived, not carried. The ledger row's own coordinates (`pipeline.rs:470/516`,
`content/mod.rs:738/762`) are re-stated in §0.3 where they drifted. Sibling plans not on main are cited at the commits
named in §8 and were read from their branches (`git show <branch>:docs/plans/<memo>`).

### §0.2 Premises (re-measured at `e8b6a24d`)

| id | claim | evidence (command → result) |
|---|---|---|
| H-1 | `build_propagation_path` is the only builder of a **node**-targeted path | `git grep -n 'build_propagation_path' -- crates docs` → 13 lines outside this memo: the definition `event_dispatch/mod.rs:203`; production calls `:262` (`build_dispatch_plan`), `:373` (`dispatch_event`), `:456` (`script_dispatch_event_core`), `crates/script/elidex-js/src/vm/host/event_target_dispatch.rs:120` (the VM `dispatchEvent` walk; its import `:20`); tests `event_dispatch/tests.rs:38/51/60`; the re-export `crates/script/elidex-script-session/src/lib.rs:34`; comments only at `crates/dom/elidex-dom-api/src/focus/sot.rs:78` and `crates/shell/elidex-shell/src/app/events.rs:195`; `docs/architecture/script.md:11` |
| H-2 | A second *get the parent* provider exists for non-entity targets | `git grep -n 'fn vm_event_parent\|fn build_vm_dispatch_plan' -- crates` → `crates/script/elidex-js/src/vm/host/event_target_dispatch_vm.rs:51` / `:155` (`IDBRequest → IDBTransaction → IDBDatabase`, else null; `ObjectId`-keyed, no entity). Not a path for any node; see D-1 (c) |
| H-3 | The Window entity is a non-Node with no `TreeRelation`, created once per `Vm` at the first bind | `crates/core/elidex-ecs/src/dom/mod.rs:491` (`create_window_root`: `spawn((NodeKind::Window,))`); `crates/script/elidex-js/src/vm/vm_api.rs:221-253` (first bind allocates, later binds reuse `HostData::window_entity`, and the bind caches `window_entity → global_object` in the wrapper cache) |
| H-4 | The Window has three readers today; **none is touched by this PR** | `git grep -n 'window_entity' -- crates` (excluding `vm/tests/`) → (i) `EcsDom`'s `window_entity` (`dom/mod.rs:504`, a linear scan for *the* `NodeKind::Window`), read by `crates/script/elidex-script-session/src/event_handler_processing/mod.rs:251` (`resolve_handler_target`) and its tests `tests_insertion_and_write.rs:60/101/235` — T3 deletes it (D-8); (ii) `HostData::window_entity` (`crates/script/elidex-js/src/vm/host_data/engine_feature/bind.rs:492`; the field `host_data/mod.rs:105`), read by `vm_api.rs:232`, `crates/script/elidex-js/src/vm/globals.rs:60` (comment), `crates/script/elidex-js/src/vm/host/event_handler_attrs.rs:448/471`, `crates/script/elidex-js/src/vm/host/history_events.rs:76/224`; (iii) the `ScriptEngine` trait's `window_entity` (`crates/script/elidex-script-session/src/engine.rs:565`, impl `crates/script/elidex-js/src/engine.rs:694` = `HostData::window_entity`), read by the shell at `crates/shell/elidex-shell/src/pipeline.rs:470/516`, `crates/shell/elidex-shell/src/content/mod.rs:738/762`, `crates/shell/elidex-shell/src/content/message_dispatch.rs:267`. (ii) is the one home of the fact; (iii) is its session-side marshalling, kept by T3 (its DoD item 4: "`HostData::window_entity` and the `ScriptEngine` trait method stay") |
| H-5 | The bound document is one per `Vm`, set at bind and cleared at unbind; one `EcsDom` can hold several Document entities and only the bound one has a browsing context | `bind.rs:115` (`document_entity = Some(document)`), `:121` (`None` at unbind), `:465` (`document()`), `:474` (`document_entity_opt()`: `None` when unbound). `git grep -n 'create_document_node()' -- crates \| grep -v tests` → `dom/mod.rs:453` (the root), `dom/tree/teardown.rs:302` (the fragment throwaway), `crates/dom/elidex-dom-api/src/element/document_base.rs:508`, `crates/dom/elidex-form/src/inert_document.rs:119` (DOMParser), `crates/script/elidex-script-session/src/scripting.rs:140`. C1-pre D-4 states the same fact as the VM's one predicate (`document_has_browsing_context(ctx, doc)` = "`doc` is the bound document", `HostData::document`); T3 D-T3 states it as `ActiveDocument { document, window }` (§8) |
| H-6 | Every shell UA dispatch is bracketed and reaches the session builder with the bound document as `ctx.document` | `git grep -n 'pipeline\.dispatch_event(\|dispatch_event_bracketed(\|result\.dispatch_event(' -- crates/shell/elidex-shell/src \| grep -v '_tests.rs\|fn dispatch_event_bracketed'` → 35 sites; all go through `PipelineResult::dispatch_event` (`crates/shell/elidex-shell/src/lib.rs:293`) or `dispatch_event_bracketed` (`pipeline.rs:121`); both build `ScriptContext::new(session, dom, document)` once and run `script_dispatch_event` under `with_bound(&mut ctx, …)`; the bracket's `bind` passes `ctx.document` to `Vm::bind` (`crates/script/elidex-js/src/engine.rs:377-398`), so while bound `ctx.document == HostData::document` by construction |
| H-7 | The VM-side UA fires share the VM walk | `git grep -n 'dispatch_script_event(' -- crates/script/elidex-js/src \| grep -v 'fn dispatch_script_event\|//'` → 11 call sites: `event_target.rs:727` (`dispatchEvent`), `event_target_dispatch.rs:623/736/821/898`, `history_events.rs:187` (at the Window), `pending_tasks.rs:461` (`postMessage`), `service_worker/event.rs:211`, `worker.rs:278`, `worker_scope.rs:333`, `dom_selection_proto.rs:740` (`selectionchange` at the Document) — all under `crates/script/elidex-js/src/vm/host/`. The walk builds its plan through `build_dispatch_plan` (`event_target_dispatch.rs:119`) and reads `HostData` through `ctx.host()` (`:118`) |
| H-8 | Readers of the built path | `git grep -n '\.composed_path\b' -- crates \| grep -v 'tests\|//'` → 9 lines: `event_target_dispatch.rs:120/142` (seeds the `composedPath()` Array), `crates/script/elidex-js/src/vm/host/events.rs:187/189` (same, UA path), `crates/script/elidex-js/src/vm/natives_promise.rs:524` (hand-set `[document]`, no builder), `mod.rs:161/166` (`composed_path_for_js`), `mod.rs:373/459` (writes) |
| H-9 | `composed_path_for_js` (`mod.rs:156`) has no caller outside its own tests | `git grep -n 'composed_path_for_js' -- crates` → the definition `:156`, two doc-comment mentions `:67/:80`, the `lib.rs:34` re-export and `event_dispatch/tests.rs:433/461`. The VM's `composedPath()` returns the seeded Array as is (`crates/script/elidex-js/src/vm/natives_event.rs:143`), so DOM §2.2 `composedPath()` steps 7–16 (closed-tree hiding) are not applied — P2's (§7) |
| H-10 | Window listeners live in the ECS `EventListeners` component on the Window entity and are collected by the same `collect_listeners` as every node's | `crates/script/elidex-js/src/vm/host/dispatch_target.rs` module doc ("`Node` — the per-entity ECS `EventListeners` component (every DOM node, `Window`, `Document`, …)"); `mod.rs:306` reads `EventListeners` on any entity |
| H-11 | `currentTarget` / `composedPath()` entries are wrappers from the identity cache; the Window's cache entry is the global object | `crates/script/elidex-js/src/vm/host/elements.rs:74` (`create_element_wrapper`: cache hit first); `vm_api.rs:239-253` (bind caches `window_entity → global_object`); `crates/script/elidex-js/src/engine.rs:203` (`call_listener` wraps `event.target` / `current_target` through it); `event_target_dispatch.rs:139-149` (the `composedPath()` Array), `:320-326` (`walk_phase` writes `currentTarget` per item) |
| H-12 | The tree predicates are total on a non-tree entity | `crates/core/elidex-ecs/src/dom/tree/mod.rs:63` (`read_rel` → `Default` when no `TreeRelation`), `:39` (`is_ancestor` walks `get_parent`, so a Window descendant query is `false`); `crates/core/elidex-ecs/src/dom/tree/walkers.rs:395` (`find_tree_root` of the Window is the Window) |
| H-13 | The three dispatch loops gate at-target and bubble on the stop-propagation flags and run the target's listeners in one pass | `mod.rs:364` (`dispatch_event`), `:434` (`script_dispatch_event_core`), `event_target_dispatch.rs:42` (`dispatch_script_event`): capture / at-target / bubble each test `propagation_stopped \|\| immediate_propagation_stopped`; the at-target entries are `collect_listeners(…, None)` (`mod.rs:285`) = every listener on the target in registration order (`crates/script/elidex-script-session/src/event_listener.rs:318` `iter_matching`), capture and non-capture mixed; the host items of a shadow-internal target are capture / bubble entries (phase CAPTURING / BUBBLING). DOM §2.9 steps 6.13–6.14 run the target's capture listeners in the capture pass and its non-capture listeners in the bubble pass, both at AT_TARGET, and every item with a non-null shadow-adjusted target the same way — pre-existing, §7 |
| H-14 | No unlanded branch, local or remote, changes a file this PR touches or reads, other than comment lines | `for b in $(git branch -a --format='%(refname:short)' \| grep -v HEAD); do git log --oneline $b --not origin/main --stat -- crates/script/elidex-script-session/src/event_dispatch crates/script/elidex-script-session/src/lib.rs crates/script/elidex-script-session/src/engine.rs crates/script/elidex-script-session/src/event_handler_processing/mod.rs crates/core/elidex-ecs/src/dom/mod.rs crates/script/elidex-js/src/vm/host/event_target_dispatch.rs crates/script/elidex-js/src/vm/host/events.rs crates/script/elidex-js/src/vm/natives_event.rs crates/shell/elidex-shell/src/pipeline.rs; done` over 74 branches → 7 branches with commits: `origin/rc2a-clone-created` (4, #596's pre-squash history, landed) and `origin/vm-construct-isconstructor-check` (1, #591's, landed); `k2-wire-fgit-scrub-complement` and `vm-p4-slice-1a-plan` (a merge of `origin/main` each, no own change); `domform-submittable-category` (`409bf0d2`: two doc-comment lines in `event_target_dispatch.rs:829-830`, a citation sweep); `layout-split-ecs-dom-factory-seam-abandoned` (2 commits moving factories out of `dom/mod.rs`; abandoned by name, and `dom/mod.rs` is not touched here); `layout-focus-fixup-canonical` (`5382fa78`: −4 lines in `pipeline.rs`, not near `:432-478`; `pipeline.rs` is not touched here). Plans on unlanded branches that name this slot: T3 (H-T27, §8), C1-pre (`:123`, a passing mention) |
| H-15 | `relatedTarget` is marshalled raw, never retargeted | `crates/core/elidex-plugin/src/event_types.rs:119` (`related_target: Option<u64>` entity bits); `crates/script/elidex-js/src/vm/host/event_shapes.rs:450` (bits → wrapper, no `retarget`). Touch events do not exist. §7 |
| H-16 | The session's callback-form loop `dispatch_event` has test callers only | `git grep -n -E '(^\|[^_a-z.])dispatch_event\(' -- crates \| grep -v 'MutationEvent\|self\.dispatch_event(&event)\|pipeline\.dispatch_event\|result\.dispatch_event'` → the definition `mod.rs:364`, its doc mention `:57`, 17 calls in `event_dispatch/tests.rs` (`:75`–`:418`), and the shell's unrelated `PipelineResult::dispatch_event` (`lib.rs:293`). It is the engine-independent form of the loop (U-tests, §6.2); its duplication of `script_dispatch_event_core` is pre-existing and recorded in §7 |

### §0.3 Premise errors in the ledger row / briefs

- "`build_propagation_path` is the only path builder": true for node targets (H-1), but a second *get the parent*
  provider exists for `VmObject` targets (H-2), and `natives_promise.rs:524` hand-sets a one-item path (H-8). D-1 (c)
  says what this slice does with each.
- "the shell's Window-targeted dispatches (`pipeline.rs:470/516`, `content/mod.rs:738/762`)": the lines are the
  `window_entity()` lookups; the dispatches are `pipeline.rs:474/521/533` and `content/mod.rs:749/772`
  (`message_dispatch.rs:273/320` are two more — `resize`, and the iframe→parent `message`). None puts the Window on a
  node's path, as the row says; after D-1 they are unchanged (the Window's parent is null).
- "Shell input events use the same builder (code read)": confirmed transitively (H-6), not directly — no shell site
  calls `build_propagation_path`; they call `PipelineResult::dispatch_event` / `dispatch_event_bracketed`.
- The Rev 2 brief's "`HostData::window_entity` plus C1-pre D-4 `document_has_browsing_context`" names the fact
  correctly but not its session-side carrier: D-4 is a VM-host predicate (`vm/host/document_accessors.rs`, C1-pre, not
  on main) that the session crate cannot call, and the `ScriptEngine` trait item carries only the Window. The pair
  "bound document + its Window" reaches the session loop as `(ctx.document, engine.window_entity())` (H-6) and the VM
  walk as `(HostData::document_entity_opt(), HostData::window_entity())` — D-2.
- The Rev 2 brief's "decide whether the 'load → None' Document-arm exception is implementable now": that exception is
  in scope and was in Rev 1 (now D-5). What R1 asked to re-measure is the deferral of the *legacy target override
  flag*; its Rev 1 reason ("needs the Window→Document direction") was false, since the firing site has `document` —
  D-5 and §7 give the corrected reason.
- The brief's "(uses at ~:262 / :373 / :456)" and "`create_window_root` sets no `TreeRelation`" are exact at
  `e8b6a24d` (H-1, H-3).

### §0.4 Decisions

- **D-1 — one *get the parent*.** `event_dispatch::get_the_parent(dom, node, event, active) -> Option<Entity>` (NEW)
  in `crates/script/elidex-script-session/src/event_dispatch/mod.rs` implements DOM §2.7's default and the three
  overrides as four arms keyed on the entity's components: (a) **Document** (DOM §4.5): `None` if
  `event.event_type == "load"`; else `Some(active.window)` when `active.document == node` (the Document has a
  browsing context, and its relevant global object is that Window); else `None` (no browsing context: the DOMParser,
  inert, throwaway and any other unbound Document, H-5); (b) **ShadowRoot** (DOM §4.8): `None` if `!event.composed`
  and `node` is the root of the original target (`dom.find_tree_root(event.target)`), else `host`; (c) **Node** (DOM
  §4.4): `dom.get_parent(node)` — the *assigned slot* arm is CV P2's (`docs/plans/2026-10-content-visibility.md:473`,
  `:627-628`), and the function is shaped so that P2 adds it as one `if let` in this arm; (d) **default** (Window,
  Worker, MessagePort, Selection, any other non-Node entity): `None`. `build_propagation_path(dom, event, active)` is
  its only production caller and takes the `&DispatchEvent` (the ShadowRoot arm needs `target` and `composed`, the
  Document arm `event_type`); it keeps the `MAX_ANCESTOR_DEPTH` bound. The `VmObject` provider `vm_event_parent` (H-2)
  stays where it is: its targets are per-VM `ObjectId` handles (CLAUDE.md side-store exception (a)), it never meets a
  node, and folding an `ObjectId` chain into an `Entity` function would need the engine type in the session crate — §7
  records the two-provider state once, with its reason. The rejection loop (`natives_promise.rs:524`) is not a
  consumer and keeps its hand-set path; its comment ("matching what a regular dispatch … would produce") is rewritten,
  since a regular Document dispatch now carries the Window item (its target moves under
  `#11-unhandledrejection-dispatch-target`, §7).
- **D-2 — the browsing-context fact is read, not stored: a resolver argument, one home.** The Document arm's input is
  `active: Option<ActiveDocument>` where `pub struct ActiveDocument { pub document: Entity, pub window: Entity }` is
  T3 D-T3's `Copy` value (the active document of the engine's single browsing context and its relevant global object;
  `None` = no bound document). The type is defined once, in `crates/script/elidex-script-session/src/event_handler_processing/mod.rs`
  (T3's placement) and re-exported from `lib.rs`; whichever of T3 and this PR lands first defines it with the same
  shape and doc, the second reuses it (§8). **Home**: the fact has one home, `HostData` (`document_entity`,
  `window_entity`; H-4 (ii), H-5), the same home C1-pre D-4 reads; no second store, no component (PM 132). **Two
  marshalling points, one per entry**: the session loop `script_dispatch_event_core` builds
  `engine.window_entity().map(|window| ActiveDocument { document: ctx.document, window })` — the existing trait item
  (H-4 (iii)) plus the bracket's bound document (H-6); the VM walk `dispatch_script_event` builds it through one host
  helper `HostData::active_document() -> Option<ActiveDocument>` (NEW, `bind.rs`, beside `document_entity_opt`:
  `None` when unbound), which is also T3's "one host helper" for its accessors — T3 names it if it lands first (§8).
  *Why an argument and not the trait item alone*: the builder has three callers and two of them have no
  `ScriptEngine` — the VM walk (`event_target_dispatch.rs:119`) and the callback-form loop (H-16); a trait read inside
  `build_propagation_path` is impossible without making the session's path builder engine-bound. *Why not a
  closure*: the fact is a named value T3 already defines, and a `Copy` pair is what the tests build. *Why not
  `EcsDom`'s `window_entity`*: a world query answers "is there a Window" for every Document in the world (wrong for the
  DOMParser document, H-5, T-9) — the registry idiom T3 deletes. **Pre-bind**: `window_entity()` is `None` before
  the first bind, so `active` is `None` and the Document arm returns `None` — the shell's fallback dispatches
  (`unwrap_or(document)`, §0.3) and any pre-bind Document dispatch keep today's path `[document]`. **B1**: when
  `#11-browsing-context-state-ecs-components` moves `HostData`'s document/window pair to components (B1 §5 req 5,
  `docs/plans/2026-06-agent-scoped-ecsdom-world.md:367`), the two marshalling points become two component reads and
  `ActiveDocument` becomes a read of the Document entity (or is deleted in favour of `dom.get::<…>(node)` in the
  arm); the arm, the signature and the tests do not change — one place, the same as T3's.
- **D-3 — the path is built once per dispatch, as the first commit.** `DispatchPlan` (`mod.rs:248`) gains
  `path: Vec<Entity>` (NEW field); `build_dispatch_plan` builds it once and buckets from it; the three re-builds
  (`mod.rs:373`, `:456`, `event_target_dispatch.rs:120`) read `plan.path` instead. `build_propagation_path` stays
  `pub` (tests) with one production caller. This is a behaviour-preserving change (the same walk, run once instead of
  twice per dispatch) and lands as commit 1 of this PR rather than as a prerequisite PR: D-1 changes the builder's
  signature at every one of the same four call sites, so a separate PR would touch the same lines twice and buy a
  second full gate for ~20 lines; the edge-dense rule's base case is this memo's slice, and commit 1 is reviewable on
  its own (`cargo test` green with no test change).
- **D-4 — the event path stays `Vec<Entity>`.** DOM §2.2's *event path item* fields are, at `e8b6a24d`, functions of
  (invocation target, original target, event): *shadow-adjusted target* = `apply_retarget` per item (`mod.rs:625`;
  the Window item's is null by DOM §2.9 step 6.9.6.2, and `retarget(target, Window)` yields the outermost light-tree
  host because a Window is not a node — DOM §4.8 *retarget* step 1 third bullet is false for a non-node B, so step 2
  repeats to the host; H-12); *invocation-target-in-shadow-tree* and *root-of-closed-tree* are readable off the
  entity; *slot-in-closed-tree* is false until P2; *relatedTarget* and *touch target list* have no producer (H-15). A
  per-item struct buys nothing for the Window and is the first feature's to introduce that needs a stored field (§7).
- **D-5 — `load`.** The Document arm's `"load"` comparison is the whole exception and is in scope. The shell keeps
  firing the Document's `load` at the Window (`pipeline.rs:470-475`; HTML §13.2.7 step 11.5) — path `[Window]`,
  unchanged. A `load` dispatched at a node (`img`, `script`, `link`, `iframe`) stops at the Document: Document capture
  listeners run, Window listeners do not. The *legacy target override flag* (DOM §2.9 step 2: `event.target` reads the
  Window's associated Document) is not implemented at HEAD and stays out. Re-measured: it is implementable now
  without any Window→Document lookup — the firing site holds `document` (`pipeline.rs:436`, a parameter of
  `dispatch_lifecycle_events`), so the shape is a `DispatchEvent` field carrying the override target that the first
  path item's shadow-adjusted target reads, plus both walkers' `target` slot seeding. It stays out because it is
  target semantics (what `e.target` reads at the Window item), not the path, and this slice is the path only — §7.
- **D-6 — retargeting and `composedPath()` need no change; the dead filter is deleted.** `retarget` (`mod.rs:583`)
  and `apply_retarget` (`:625`) already compute the spec result for a Window invocation target (D-4, H-12). The Window
  reaches `composedPath()` through the wrapper cache (H-11), so `e.composedPath().at(-1) === window` and
  `e.currentTarget === window` hold in both walkers without a new marshalling site. `composed_path_for_js`
  (`mod.rs:150-192`), its two tests (`tests.rs:427-461`), its re-export (`lib.rs:34`) and its two doc-comment
  mentions (`mod.rs:67/:80`) are deleted (dead-code rule, H-9): it computes a per-`currentTarget` filtered copy of a
  Rust `Vec<Entity>`, a shape the VM never reads (the VM seeds one Array per dispatch and returns it as is,
  `natives_event.rs:143`), so "connecting" it is not a wiring but the design of the closed-tree surface, which is P2's
  (CV D11, §8). The `dispatch_flag` doc at `mod.rs:75-81` is rewritten to say what still holds (the retained
  `composed_path` is never read after dispatch: the VM clears its Array slot, `event_target_dispatch.rs:243`).
- **D-7 — phases, stop flags, Window-targeted events, the depth cap.** `eventPhase` is unchanged (the Window is a
  capture / bubble item, never at-target for a node event; a Window-targeted event stays `AT_TARGET` with path
  `[Window]`). `stopPropagation()` at the Window's capture item skips at-target and bubble by H-13.
  Window-targeted dispatches (`resize`, `load`, `beforeunload`, `unload`, `message`, `storage`, `popstate`,
  `hashchange`) get path `[Window]` from arm (d) — no doubled Window. The `MAX_ANCESTOR_DEPTH` bound stays in
  `build_propagation_path` (one item deeper now); it is a sibling instance of
  `#11-capped-walker-deep-element-processing`, §7. The at-target one-pass order (H-13) is pre-existing and stays
  (§7); T-4 is written so that it does not depend on it.
- **D-8 — handler-target determination is T3's; the boundary.** HTML §8.1.8.1 *determine the target of an event
  handler* step 3 ("If eventTarget's node document is not an active document, then return null") and step 4 (the node
  document's relevant global object) are implemented by T3's `determine_event_handler_target` (T3 D-T1, D-T3; slot
  `#11-event-handler-target-determination`), replacing `resolve_handler_target` and deleting `EcsDom`'s `window_entity`.
  This PR does not touch `resolve_handler_target` (`event_handler_processing/mod.rs:251`), `native_body_weh_get` /
  `_set` (`event_handler_attrs.rs:448/471`), `EcsDom`'s `window_entity` (`dom/mod.rs:504`), the `ScriptEngine` trait item
  (`engine.rs:565`) or the shell lookups (H-4 (iii)). The two algorithms read the same fact (D-2's `ActiveDocument`)
  and agree by construction once both have landed: a `body` whose node document is not the active document gets no
  Window handler (T3 step 3), and an event dispatched at a node of that document never reaches the Window (D-1 arm
  (a)). *Rejected (R1 Ax4 CRIT)*: Rev 1's D-3 kept the handler on the element where step 3 returns null.
- **D-9 — tests.** A new VM end-to-end file, a new session unit file, three shell tests; the red-before-fix controls
  are named in §6.

## §1. Shape and verdict

**Edge-dense: yes.** The slice binds six intersecting invariant axes (§2): (A1) the path algorithm, (A2) the
browsing-context fact's marshalling at two entry points, (A3) wrapper identity of the Window as `currentTarget` /
`composedPath()` entry, (A4) retargeting with a non-node invocation target, (A5) three loops consuming one plan,
(A6) worlds with several Documents. Three or more intersect at the Document arm alone (A1 × A2 × A6). Under CLAUDE.md
(*Edge-dense work*), `/elidex-plan-review` is mandatory before implementation (R1 done; R2 on this revision). **One
PR**: the work is one narrowly-scoped slice under this reviewed memo — the terminal unit of the rule's base case (c):
it has one design decision (D-2, which reads an existing fact through an argument) and one behaviour change (the Window
on the path); it carries no data-home move, no split and no handler-target change (R1 Ax3 IMP, Ax5 IMP).

**Breadth** (§3): K=2, M=21 → preflight says SPLIT-RECOMMENDED (M ≥ 20), not SPLIT-DEFAULT. Single PR, stated:
of the 21 rows, 6 are hand-offs or records with no code here (the two relatedTarget rows, step 2, `composedPath()`
7–16, §8.1.8.1, §7.2.2) and the remaining 15 all land in one function and one plan field (§4), two of them with a
pre-existing ✗ half recorded in §7 (6.13, 6.14); a split would cut one function in two. Rev 1's M=18 covered two PRs;
the three rows added here (6.13 split from 6.14, the target-item steps split from the parent-append steps, the
relatedTarget steps) are the step-level records R1 asked for, not new code.

## §2. Coupled invariants

- **I-1 path**: the path is DOM §2.9 step 6's item sequence — target, then each *get the parent* until null.
- **I-2 fact**: "this Document has a browsing context, and its Window is W" has one home (`HostData`) and reaches the
  builder as a value; no arm queries the world for a Window.
- **I-3 identity**: the Window as an invocation target marshals to the global object (SameObject with `window`).
- **I-4 retarget**: at a non-node invocation target, `event.target` is the outermost light-tree host.
- **I-5 walkers**: the three loops consume one plan; none re-derives the path.
- **I-6 documents**: a Document that is not the bound document has no event parent.
- **I-7 load**: the Document arm returns null for `load`; Window-targeted `load` is unaffected.

Pairs (every pair of the seven; "—" = independent, stated once):
- I-1 × I-2 = the Document arm is `active.document == node`, never a world query (a query is what breaks I-6; T-9).
- I-1 × I-3 — (identity is the cache's, H-11; the path holds entities).
- I-1 × I-4 = the Window item's *shadow-adjusted target* is null (step 6.9.6.2) and the lazy `apply_retarget` at that
  item walks `retarget(target, Window)` to the outermost host (DOM §4.8 step 1, third bullet false for a non-node B;
  U-5, T-6).
- I-1 × I-5 = `DispatchPlan::path` is built once; `plan.capture` / `plan.bubble` are bucketed from it; all three loops
  read `plan.path` for `composed_path` (D-3; U-9).
- I-1 × I-6 = the DOMParser / inert / throwaway Document is a path root (arm (a) `None`); its subtree's events never
  reach the Window (T-9, U-1).
- I-1 × I-7 = the `"load"` comparison is in the Document arm, so a `load` at a node stops at the Document and a `load`
  at the Window (path `[Window]`) never reaches the arm (T-5).
- I-2 × I-3 = `HostData::window_entity` is the entity the bind cached as `globalThis` (`vm_api.rs:239-253`), so the
  Window the arm returns is the one whose wrapper is `window` (T-3).
- I-2 × I-4 — (retargeting reads the tree, not the fact).
- I-2 × I-5 = both marshalling points read the same `HostData` fields (D-2): the session loop through the trait item +
  `ctx.document` (H-6), the VM walk through `HostData::active_document()`; a mismatch is impossible while bound because
  `ctx.document` is what the bracket bound (T-1 and S-1 cover one entry each).
- I-2 × I-6 = `active.document` is one entity per `Vm`; every other Document compares unequal (H-5).
- I-2 × I-7 — (the `load` test precedes the fact read).
- I-3 × I-4 = `e.target` at the Window listener is the host's wrapper, `e.currentTarget` the global object (T-6).
- I-3 × I-5 = all three loops resolve `current_target` through `create_element_wrapper` (H-11); no walker-specific
  Window marshalling.
- I-3 × I-6, I-3 × I-7 — (identity does not depend on which Document or which type).
- I-4 × I-5 — (`apply_retarget` is shared, `mod.rs:625`).
- I-4 × I-6 = the Window's own tree queries are total (H-12), so no arm needs an "is this a node" pre-check beyond the
  component match.
- I-4 × I-7, I-5 × I-7 — (`load` changes the path's end, not retargeting or the loops).
- I-5 × I-6 = the rejection loop's hand-set `[document]` path is not a consumer (H-8); its Window move is another slot.
- I-6 × I-7 = both are `None` arms of the same function; order inside the arm is `load` first (U-1 covers each alone).

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG DOM §2.7 Interface EventTarget | *get the parent*, default | non-Node entity (Window, Worker, MessagePort, Selection) → null | `get_the_parent` arm (d) (NEW) | ✓ | yes |
| WHATWG DOM §4.4 Interface Node | *get the parent* | (i) assigned → assigned slot (CV P2, `content-visibility.md:473`); (ii) otherwise parent | arm (c): (ii) now, (i) P2's | ✗ (P2) | yes |
| WHATWG DOM §4.5 Interface Document | *get the parent* | (i) type is `load` → null; (ii) no browsing context → null; (iii) relevant global object | arm (a): (i) string compare, (ii) `active.document != node` or `active` is `None`, (iii) `active.window` | ✓ | yes |
| WHATWG DOM §4.8 Interface ShadowRoot | *get the parent* | (i) composed unset and root of path[0]'s invocation target → null; (ii) host | arm (b) (HEAD stops at any shadow root when non-composed; equal until P2) | ✓ | yes |
| WHATWG DOM §4.8 Interface ShadowRoot | *retarget* steps 1–2 | step 1: (i) A not a node; (ii) A's root not a shadow root; (iii) B a node and A's root a shadow-including inclusive ancestor of B → return A; step 2: A ← A's root's host, repeat | `retarget` `mod.rs:583` — B = Window makes (iii) false (H-12) | ✓ | yes |
| WHATWG DOM §2.9 Dispatching events | steps 6.3, 6.8, 6.9.9 (the target item; *get the parent* calls) | target appended first; each parent from the previous item's *get the parent* | `build_propagation_path` (D-1, D-3) | ✓ | yes |
| WHATWG DOM §2.9 Dispatching events | steps 6.9.6 / 6.9.8 (append a parent) | Window parent, or a node in the target's root → 6.9.6.2 append with null shadow-adjusted target; otherwise 6.9.8 with shadow-adjusted target = parent | `build_propagation_path` + lazy `apply_retarget` (D-4) | ✓ | yes |
| WHATWG DOM §2.9 Dispatching events | steps 4, 6.9.3, 6.9.7 (relatedTarget retargeting; parent is relatedTarget → null) | no producer (H-15) | absent → `#11-related-target-retargeting` (§7) | ✗ | no |
| WHATWG DOM §2.9 Dispatching events | step 6.13 (capture pass; 6.13.1 AT_TARGET for items with a non-null shadow-adjusted target) | Window item: CAPTURING, capture listeners | the three loops' capture phase (H-13); ✗ host items of a shadow-internal target read CAPTURING, spec AT_TARGET (pre-existing, §7) | ✗ | yes |
| WHATWG DOM §2.9 Dispatching events | step 6.14 (bubble pass; 6.14.1 AT_TARGET items run regardless of `bubbles`; 6.14.2.1 others continue when `bubbles` is false) | Window item: BUBBLING iff `bubbles`; the target's non-capture listeners always run (T-4) | the three loops' at-target + bubble phases (H-13); ✗ the target's capture and non-capture listeners run in one registration-ordered pass (pre-existing, §7) | ✗ | yes |
| WHATWG DOM §2.9 Dispatching events | *invoke* steps 2–3 (shadow-adjusted target look-back) | null at the Window item → preceding non-null | lazy `apply_retarget` `mod.rs:625` (D-4) | ✓ | yes |
| WHATWG DOM §2.9 Dispatching events | *invoke* step 6 (stop propagation flag) | set at the Window's capture item → at-target and bubble skipped | the three loops' gates (H-13) | ✓ | yes |
| WHATWG DOM §2.9 Dispatching events | step 2 (*targetOverride*, legacy target override flag) | Window `load`: target reads the associated Document | absent at HEAD → `#11-window-load-legacy-target-override` (§7) | ✗ | no |
| WHATWG DOM §2.2 Interface Event | `composedPath()` steps 1–6 (path non-empty; currentTarget appended) | Window entry marshals through the wrapper cache (H-11) | `event_target_dispatch.rs:139-149`, `events.rs:187` | ✓ | yes |
| WHATWG DOM §2.2 Interface Event | `composedPath()` steps 7–16 (closed-tree hiding), 17 (return) | not applied by the VM (H-9) | `composed_path_for_js` deleted (D-6) → P2, `#11-shadow-slot-assignment-flat-tree` (CV D11, §8) | ✗ | yes |
| WHATWG DOM §2.2 Interface Event | *event path item* fields | shadow-adjusted target / in-shadow-tree / root-of-closed-tree derived; relatedTarget / touch target list have no producer (H-15) | `Vec<Entity>` (D-4) | ✗ (relatedTarget, touch) | no |
| WHATWG HTML §7.3.2 Browsing contexts | a Document's browsing context (null for inert Documents) | non-null iff the Document is the bound document (C1-pre D-4's reading, H-5) | `active.document == node` (D-2) | ✓ | no |
| WHATWG HTML §8.1.3.3.4 Relevant | relevant global object of a Document | the bound document's Window | `active.window` (D-2) | ✓ | no |
| WHATWG HTML §7.2.2 The Window object | *associated Document* | the inverse of the pair; not read here | `HostData` (`document_entity`, `window_entity`), unchanged | ✓ | no |
| WHATWG HTML §13.2.7 The end | step 8.2 (`DOMContentLoaded` at the Document, bubbles); step 11.2 (browsing context null → abort), 11.3 (window = relevant global), 11.5 (fire `load` at window, legacy target override flag) | 8.2: path `[Window, Document]` now (T-10, S-3); 11.3 / 11.5 target: `pipeline.rs:470` unchanged; 11.2: the shell's `document` fallback predates this memo; the flag: ✗ (§7) | `dispatch_lifecycle_events` `pipeline.rs:432-475` | ✗ (the flag) | no |
| WHATWG HTML §8.1.8.1 Event handlers | *determine the target of an event handler* step 3 (node document not active → null), step 4 (relevant global object) | T3's `determine_event_handler_target`; not touched here (D-8) | boundary record only | ✗ (T3) | yes |

**Breadth**: K=2 (dom, html), M=21 → SPLIT-RECOMMENDED by preflight; single PR justified in §1.

### §3.1 User-input touch audit

- Event type strings from script (`new Event(type)`; `dispatchEvent`): arm (a)'s `"load"` compare is exact and
  case-sensitive, as DOM §4.5 reads it; `"Load"` reaches the Window (spec).
- The `composed` flag from script: arm (b) reads it; a non-composed event at a node inside a shadow tree still stops
  at that root (unchanged).
- Documents created by script (`DOMParser`) and by the engine (fragment throwaway, inert form document): arm (a)
  returns null for all of them because none is `active.document` — fail-safe direction = no Window; an unknown
  Document kind falls to the same side.
- Pre-bind (`window_entity() == None`): `active` is `None`; every Document arm returns null; the shell's
  `unwrap_or(document)` fallbacks (§0.3) and the pre-bind `DOMContentLoaded` / `readystatechange` dispatches keep
  path `[document]` (D-2).
- `<body on*>` content attributes: not read by this PR (D-8).
- Adjacent pre-existing, exposure unchanged: the `MAX_ANCESTOR_DEPTH` truncation of a deep path (silent;
  `#11-capped-walker-deep-element-processing` is the family); the VM's `composedPath()` exposing closed-tree entries
  (P2, §8); the at-target one-pass order (§7).
- No new network, storage or permission surface.

## §4. Mechanism (one PR, two commits)

**Commit 1 — `DispatchPlan::path` (D-3, behaviour-preserving).** `build_dispatch_plan` keeps the path it builds in
`plan.path`; `dispatch_event` (`mod.rs:373`), `script_dispatch_event_core` (`:456`) and `dispatch_script_event`
(`event_target_dispatch.rs:120`) set `composed_path` from it (`clone()` in the two session loops where the plan is
borrowed; a move in the VM walk, which owns `local`). `cargo test -p elidex-script-session -p elidex-js
--all-features` green with no test change.

**Commit 2 — `get_the_parent` (D-1, D-2, D-5, D-6, D-7) and the tests (D-9).**

```rust
/// WHATWG DOM §2.7 *get the parent*, with the §4.4 / §4.5 / §4.8 overrides.
/// Called at path-build time only (`debug_assert!(event.original_target.is_none())`).
/// `active`: the bound document and its Window (HTML §7.3.2 / §8.1.3.3.4), `None` when unbound.
pub fn get_the_parent(
    dom: &EcsDom, node: Entity, event: &DispatchEvent, active: Option<ActiveDocument>,
) -> Option<Entity> {
    let world = dom.world();
    if let Ok(sr) = world.get::<&ShadowRoot>(node) {                       // §4.8
        let stops = !event.composed && dom.find_tree_root(event.target) == node;
        return (!stops).then_some(sr.host);
    }
    match dom.node_kind(node) {
        Some(NodeKind::Document) => {                                       // §4.5
            if event.event_type == "load" { return None; }
            active.filter(|a| a.document == node).map(|a| a.window)
        }
        Some(kind) if kind.is_node() => dom.get_parent(node),               // §4.4 (P2: assigned slot first)
        _ => None,                                                          // §2.7 default
    }
}
```

`build_propagation_path(dom, event, active)` pushes `event.target`, then `get_the_parent` until `None` or the depth
bound, and reverses; `build_dispatch_plan(dom, event, active)` stores it in `plan.path`. `script_dispatch_event_core`
passes `engine.window_entity().map(|window| ActiveDocument { document: ctx.document, window })`; `dispatch_event`
(the callback form) takes `active` as a parameter (its callers are tests, H-16); `dispatch_script_event` passes
`ctx.host().active_document()`. The `NodeKind::is_node` predicate (`crates/core/elidex-ecs/src/components.rs:538`) is
false for Window / Worker / OffscreenCanvas, so arm (c) never reads a `TreeRelation` off a non-Node; `dom.node_kind`
is `crates/core/elidex-ecs/src/dom/classify.rs:97`. `composed_path_for_js` and its tests go (D-6). Nothing else in
the three loops changes (H-13). `docs/architecture/script.md:11` is rewritten to the new signature and the Window
item; the `natives_promise.rs:510-516` comment is rewritten (D-1).

## §5. Populations

`wc -l` at `e8b6a24d` (re-run at the base). The 1000-line audit: no touched file is above 1000 or crosses it;
`event_target_dispatch.rs` 902 shrinks by one line (D-3) and grows by the `active` read; `natives_promise.rs` 961 is a
comment-only touch; `dom/mod.rs` (984) and `pipeline.rs` (975) are not touched.

| file | lines | change |
|---|---|---|
| `crates/script/elidex-script-session/src/event_dispatch/mod.rs` | 654 | `get_the_parent` (NEW), `build_propagation_path` / `build_dispatch_plan` / `dispatch_event` signatures, `DispatchPlan::path` (NEW), −2 re-builds, −`composed_path_for_js` (`:150-192`), doc at `:57-81` rewritten |
| `crates/script/elidex-script-session/src/event_dispatch/tests.rs` | 481 | `:38`, `:51`, `:60` and the 17 `dispatch_event` calls take `active` (`None`); −`:427-461` (two tests) |
| `crates/script/elidex-script-session/src/event_dispatch/tests_get_the_parent.rs` (NEW) | — | U-1..U-9; declared beside `mod tests;` (`mod.rs:653-654`) |
| `crates/script/elidex-script-session/src/event_handler_processing/mod.rs` | 410 | `ActiveDocument` (NEW, ~10 lines) unless T3 landed first (§8) |
| `crates/script/elidex-script-session/src/lib.rs` | 68 | `:34`: −`composed_path_for_js`, +`get_the_parent`; +`ActiveDocument` re-export |
| `crates/script/elidex-js/src/vm/host_data/engine_feature/bind.rs` | 573 | `HostData::active_document()` (NEW, beside `:474`) unless T3 landed first (§8) |
| `crates/script/elidex-js/src/vm/host/event_target_dispatch.rs` | 902 | `:117-122`: `build_dispatch_plan(dom, &local, active)`, `local.composed_path = plan.path` |
| `crates/script/elidex-js/src/vm/natives_promise.rs` | 961 | `:510-516` comment (D-1) |
| `crates/script/elidex-js/src/vm/tests/tests_event_path_window.rs` (NEW) | — | T-1..T-10; declared in `crates/script/elidex-js/src/vm/tests/mod.rs` (beside `:51` `tests_dispatch_event`) |
| `crates/shell/elidex-shell/src/content_eh_attrs_bind_tests.rs` | 390 | S-1..S-3 (its `click` `:96` / `eval_ok` `:104` helpers; declared by `#[path]` at `crates/shell/elidex-shell/src/content/mod.rs:799-800`; a sibling file declared the same way if the scenario seam is clearer) |
| `docs/architecture/script.md` | 50 | `:11` |

Not touched: `event_target_dispatch_vm.rs` (673; the `VmObject` provider, D-1 (c)), `history_events.rs` (242;
dispatches at `HostData::window_entity`), `dom/mod.rs`, `components.rs`, `vm_api.rs`, `engine.rs` (both crates),
`event_handler_attrs.rs`, `event_handler_processing/mod.rs:251`, `pipeline.rs`, `content/mod.rs`,
`message_dispatch.rs` (D-8, H-4).

## §6. Tests

Each test names its kind: **red** = fails at `e8b6a24d` and passes after; **guard** = passes before and after, and
names the mutant it kills.

### §6.1 VM end-to-end (`tests_event_path_window.rs`, the `tests_dispatch_event.rs` harness: `bind_vm`, `build_simple_tree`)

- **T-1** (red; the probe): capture + bubble `click` listeners on `window`, `document`, `body`;
  `body.dispatchEvent(new Event('click', {bubbles: true}))` → `"w-cap d-cap b-cap b-bub d-bub w-bub"`.
- **T-2** (red): inside the body listener, `e.composedPath()` is `[body, html, document, window]` by `===`.
- **T-3** (red): at the window listeners, `e.currentTarget === window`, `e.target === body`, `e.eventPhase` is `1`
  then `3`.
- **T-4** (red): a non-bubbling event at `body`, listeners registered `b-cap` before `b-bub` → `"w-cap d-cap b-cap
  b-bub"`: the Window and Document items run in the capture pass only (DOM §2.9 step 6.14.2.1), the target's
  non-capture listener runs in the bubble pass regardless of `bubbles` (step 6.14.1). The registration order makes
  HEAD's one-pass at-target order equal the spec's two-pass order (H-13), so the test is about the path, not the order.
- **T-5** (guard; kills a Window arm without the `load` compare): `window.addEventListener('load', f, true)` +
  `el.dispatchEvent(new Event('load'))` → `f` not called; `document.addEventListener('load', g, true)` → `g` called;
  `window.dispatchEvent(new Event('load'))` → the window listener called once with `e.composedPath()` = `[window]`.
- **T-6** (red): open shadow root on `el`, target inside; the window listener sees `e.target === el` and
  `e.composedPath()` ending `…, el, body, html, document, window` with the inner node and shadow root present (open).
- **T-7** (red): `stopPropagation()` in the window capture listener → no document / body listener runs;
  `stopImmediatePropagation()` there → the second window capture listener does not run.
- **T-8** (guard; kills a doubled Window): `window.dispatchEvent(new Event('resize'))` → one call, `e.eventPhase === 2`,
  `e.composedPath()` = `[window]`.
- **T-9** (guard; kills a world-query arm): a `DOMParser` document's body `dispatchEvent(new Event('click',
  {bubbles: true}))` (the `tests_dom_parser.rs` shape) → the window listener is not called; that document's own
  listener is; `e.composedPath()` ends at that document.
- **T-10** (red): `document.dispatchEvent(new Event('DOMContentLoaded', {bubbles: true}))` → the window bubble
  listener runs (the shell's `pipeline.rs:450` dispatch has this shape; S-3 is the shell-side run).

### §6.2 Engine-independent (`tests_get_the_parent.rs`, the `event_dispatch/tests.rs` helpers `elem`, `setup_shadow_dom`, `add_listener`)

`active` is built as `Some(ActiveDocument { document: doc, window: w })` with `w = dom.create_window_root()`.

- **U-1** (red): `get_the_parent(window, click, active)` = `None`; `(document, click, active)` = `Some(window)`;
  `(document, load, active)` = `None`; `(document, click, None)` = `None`; a second Document (`create_document_node`)
  with the same `active` → `None`.
- **U-2** (red): `build_propagation_path` for `doc > html > body > p` with `active` = `[window, doc, html, body, p]`;
  `build_dispatch_plan` has the Window first in `capture` and last in `bubble`; `plan.path` equals the built path.
- **U-3** (guard): the same tree with `active = None` → `[doc, html, body, p]` (today's shape).
- **U-4** (guard): ShadowRoot arm: non-composed event targeted inside the shadow tree → `None` at that root; composed
  → `host`; a non-composed event targeted in the light tree whose walk meets a shadow root cannot occur before P2 —
  stated, not tested.
- **U-5** (red): `apply_retarget` at the Window item for a shadow-internal target → `event.target == host`,
  `original_target == Some(inner)`.
- **U-6** (red): `dispatch_event` (the callback form, `active` given) invokes the Window's capture listener first and
  bubble listener last, through the `EventListeners` component on the Window (`add_listener(window, …)`).
- **U-7** (guard): the depth bound still terminates: a `doc > …` chain of `MAX_ANCESTOR_DEPTH + 2` elements with
  `active` → the path has `MAX_ANCESTOR_DEPTH + 1` items and the Window is absent (the truncation is the pre-existing
  silent cap, §7; the test pins its shape so a later uncapping is a visible change).
- **U-8** (guard; kills an arm order swap): a Document never carries `ShadowRoot`, and a ShadowRoot is `NodeKind::
  DocumentFragment` — `get_the_parent` on each with both arms present returns the §4.8 / §4.5 answer (a mutant that
  tests `node_kind` first would send a ShadowRoot to arm (c) and return its parent, `None`).
- **U-9** (red): `script_dispatch_event_core` sets `event.composed_path` to the plan's path and the Window is on it:
  a mock `ScriptEngine` whose `window_entity()` returns `Some(w)` and `ctx.document = doc` → `event.composed_path ==
  [w, doc, …]` observed inside `call_listener`; the same mock with `window_entity() == None` → `[doc, …]`.

### §6.3 Shell (`content_eh_attrs_bind_tests.rs` harness)

- **S-1** (red): a page whose inline script runs `window.addEventListener('click', e => document.body.setAttribute(
  'data-w', e.currentTarget === window))`; `click(pipeline, "t")` → `data-w="true"`.
- **S-2** (guard): `dispatch_lifecycle_events`' `load` reaches a `window.addEventListener('load')` listener with
  `e.composedPath().length === 1`.
- **S-3** (red): a page whose inline script runs `window.addEventListener('DOMContentLoaded', …)` records the call
  (HTML §13.2.7 step 8.2 through `pipeline.rs:450`; the `build_main_document` harness runs the lifecycle,
  `content_test_support.rs:377`).

## §7. Out of scope and slots

Each item states the defect at HEAD, then the slot with its **owner** / **trigger** / **acceptance**. Registration is
the PM's at landing; the names are proposals unless the slot exists.

- **Two *get the parent* providers.** `vm_event_parent` (`event_target_dispatch_vm.rs:51`) chains `ObjectId` targets;
  `get_the_parent` chains entities. Reason: the `VmObject` targets are per-VM identity handles (side-store exception
  (a)); one function would need the engine's `ObjectId` in the session crate. Not a defect; recorded here once so no
  later reader re-opens it. Re-eval if a `VmObject` ever needs a node parent.
- **Window `load` target** → `#11-window-load-legacy-target-override` (proposed). *Defect*: `pipeline.rs:474` fires the
  Document's `load` at the Window without the *legacy target override flag*: `e.target === window`, spec `document`
  (DOM §2.9 step 2; HTML §13.2.7 step 11.5). *Shape* (D-5): a `DispatchEvent` field set by the firing site, which has
  `document`; the first item's shadow-adjusted target reads it; both walkers seed the `target` slot from it. No
  Window→Document lookup is needed. **Owner**: L3 (the dispatch surface). **Trigger**: the first consumer that reads
  `e.target` in a Window `load` listener (a site's `e.target.readyState` / `e.target.body`), or T3's `<body onload>`
  cells if they observe `e.target`. **Acceptance**: `window.addEventListener('load', e => e.target === document)` is
  `true` from the shell's lifecycle dispatch and `e.currentTarget === window`.
- **`composedPath()` closed-tree hiding** → P2, `#11-shadow-slot-assignment-flat-tree` (exists; CV D11 names the event
  path's *get the parent* / `composedPath` as P2's own surfaces). *Defect*: the VM returns the whole path from every
  listener (H-9); DOM §2.2 `composedPath()` steps 7–16 hide a closed shadow tree's entries from listeners outside it.
  `composed_path_for_js` is deleted here (D-6); P2 designs the surface against the VM's one-Array-per-dispatch
  representation (a per-item Array, or a per-call compute in `native_event_composed_path`). **Owner**: Layout (P2).
  **Trigger**: P2's plan. **Acceptance**: a listener on the host of a closed tree sees no entry below the host in
  `e.composedPath()`; a listener inside sees them.
- **`relatedTarget` retargeting** → `#11-related-target-retargeting` (proposed). *Defect*: `MouseEvent` / `FocusEvent`
  `relatedTarget` is marshalled raw (H-15); DOM §2.9 steps 4 and 6.9.3 retarget it per item, and step 6.9.7 ends the
  path when a parent is the relatedTarget. The first consumer introduces the per-item struct D-4 defers. **Owner**: L3.
  **Trigger**: the first `relatedTarget` producer that crosses a shadow boundary (focus / mouse enter-leave in a shadow
  tree). **Acceptance**: a `focusout` whose `relatedTarget` is inside a closed tree reads the host from outside, and
  the path ends at the parent that is the relatedTarget.
- **Slot assignment on the path** → P2 (`#11-shadow-slot-assignment-flat-tree`): CV `content-visibility.md:473`,
  `:627-628` — the Node arm's assigned-slot branch, `slotInClosedTree`, and the ShadowRoot arm's path-root test
  becoming load-bearing. **Owner / trigger / acceptance**: P2's.
- **At-target order and the shadow-host items** → `#11-dispatch-at-target-two-pass` (proposed). *Defect* (H-13): the
  target's capture and non-capture listeners run in one registration-ordered pass; DOM §2.9 steps 6.13 / 6.14 run the
  capture ones first (capture pass, AT_TARGET) then the non-capture ones (bubble pass, AT_TARGET) — observable as
  `body.addEventListener('click', bub); body.addEventListener('click', cap, true)` firing `bub, cap` (spec `cap,
  bub`); and the host items of a shadow-internal target (non-null shadow-adjusted target, step 6.9.8) read
  `eventPhase` CAPTURING / BUBBLING where the spec reads AT_TARGET, and their non-capture listeners are skipped for a
  non-bubbling event where the spec runs them (6.14.1). **Owner**: L3. **Trigger**: a WPT or site that orders at-target
  listeners by capture, or P2's closed-tree work (which touches the same items). **Acceptance**: the two-pass order
  above, and `e.eventPhase === 2` at a shadow host listener for an event targeted inside its tree.
- **Three copies of the dispatch loop.** `dispatch_event` (`mod.rs:364`, callback form, test callers only, H-16),
  `script_dispatch_event_core` (`:434`) and `dispatch_script_event` (`event_target_dispatch.rs:42`) each run DOM §2.9
  steps 6.13–6.14 over one plan. *Defect*: three sites for one algorithm (one-issue-one-way); the callback form is
  kept here as the engine-independent exercise of the loop (U-6, U-7), not deleted, because its deletion would move the
  session's own loop tests onto a mock engine. Folded into `#11-dispatch-at-target-two-pass` (the two-pass change is
  the natural moment to collapse the loops: one shared phase runner, three invokers). No separate slot.
- **`unhandledrejection` at the Window** → `#11-unhandledrejection-dispatch-target` (exists; owner the core VM area,
  trigger "a PR touching … the dispatch target of global events"). The rejection loop targets the Document with a
  hand-built path (H-8); when the target moves it should go through the shared walker, and the Window's path is then
  `[Window]`. This PR rewrites its comment only (D-1). **Acceptance** (added for the existing row): the listener runs
  with `e.currentTarget === window` and `e.composedPath()` = `[window]`.
- **The depth bound** → `#11-capped-walker-deep-element-processing` (exists; owner L3, registered PM 368). The path
  builder's bound is one more instance (D-7; U-7 pins its shape). **Acceptance** (added for the existing row's list):
  the path of a node at depth `MAX_ANCESTOR_DEPTH + 2` reaches the Window.
- **The browsing-context fact as components** → B1, `#11-browsing-context-state-ecs-components` (exists): `ActiveDocument`
  is read-side marshalling of `HostData`'s pair (D-2); B1 §5 req 5 moves the pair; the two marshalling points become
  component reads (D-2). **Owner / trigger**: B1's. **Acceptance** (for this memo's part): `git grep -n
  'ActiveDocument {' -- crates` → only the component-backed constructors (or none, if the arm reads the component).

## §8. Edges and hand-offs

- **T3 (`docs/plans/2026-10-eh-attrs-t3-target-determination.md`, rev 4.4 `d15a141c`, branch
  `layout-eh-target-determination-plan`, not on main; plan-review closed)** — D-T3 defines `ActiveDocument {document,
  window}` in `event_handler_processing/mod.rs` and builds it in the VM through one host helper; D-T1 deletes
  `resolve_handler_target`; D-T5 deletes `EcsDom`'s `window_entity`; its DoD item 4 keeps `HostData::window_entity` and
  the `ScriptEngine` trait item; its H-T27 and DoD item 1 already provide for this slot landing first ("T3's element
  dispatches are non-bubbling with non-capture handlers, so a Window on the path changes none of them" — still true
  under the T-4 reading: the Window item runs capture listeners only for a non-bubbling event). **Order-free**: if T3
  lands first, this PR imports `ActiveDocument` and T3's host helper (reusing its name) and adds nothing to
  `event_handler_processing/mod.rs` or `bind.rs`; if this PR lands first, T3 finds both defined with its shape and doc
  and re-pins (DoD item 1). The two algorithms then read one fact (D-8). This memo makes no edit to T3's memo.
- **C1-pre (`docs/plans/2026-11-eh-attrs-c1-pre-document-wrapper.md`, rev 2.3 `69700252`, branch
  `layout-c1-pre-document-wrapper-plan`, not on main; plan-review closed)** — D-4 `document_has_browsing_context(ctx,
  doc)` is the VM-host predicate over the same `HostData::document` (H-5); its `:123` names event paths among the
  places a Document entity is reachable. No shared symbol, no ordering constraint; the fact's reading ("non-null iff
  the bound document") is cited from it in §3.
- **C0b / C0c (`docs/plans/2026-11-eh-attrs-c0b-node-document-fact.md`, `…-c0c-adopt-notification-adoptnode.md`,
  branch `layout-dom-c0-insert-adopt-plan`, not on main)** — C0b renames the node-document reader (`owner_document` →
  `node_document`); C0c adds `adoptNode`. This PR reads neither (the Document arm compares the Document entity itself,
  not a node's document; Rev 1's D-3 readers of `owner_document` are gone). A node adopted by C0c into a non-bound
  Document dispatches to a path ending at that Document (arm (a) `None`) — consistent with T3's step 3 for the same
  node. No ordering constraint.
- **CV P2 (Layout; `docs/plans/2026-10-content-visibility.md:204-222` D11, `:473`, `:627-628`, `:762`; slot
  `#11-shadow-slot-assignment-flat-tree`)** — P2 adds the assigned-slot `if let` to arm (c) and nothing else to the
  function; the ShadowRoot arm's path-root test (b) is already the spec's form, so P2 does not revisit it.
  `composedPath()`'s closed-tree hiding is P2's own surface (D11): this memo deletes the dead filter (D-6) and hands the
  surface over (§7). Hand-off: this memo's `get_the_parent` is the one function P2 extends; P2's memo cites it by
  symbol.
- **EH-attrs slice-a (`docs/plans/2026-10-eh-attrs-slice-a.md:597`, landed #563)** — its session fixtures call
  `create_window_root()`; the signature is unchanged by this PR (Rev 1's change is gone).
- **IBP handler gates (`docs/plans/2026-10-ibp-sandbox-handler-gates.md:905`, landed #580)** — its C8e row reads a
  handler through the VM dispatch walk; unchanged here (the walk's plan gains the Window item; the handler lookup is
  per listener).
- **Unlanded branches** — none changes a file this PR touches except comment lines (H-14).
- **Memory / ledger** — the slot row's "`build_propagation_path` is the only path builder" is corrected per §0.3 at
  landing; `docs/architecture/script.md:11` is rewritten in this PR.

## §9. Definition of done

1. Base re-measure: every command in §0.2 re-run at the base; counts and lines updated in this memo (rev bump).
2. Commit 1 (D-3) green on its own: `cargo test -p elidex-script-session -p elidex-js --all-features` with no test
   change.
3. `cargo test -p elidex-script-session -p elidex-js -p elidex-shell --all-features` green at the head;
   `mise run ci` before push; `/elidex-review`; `/external-review` after push.
4. `git grep -n 'composed_path_for_js' -- crates docs` → 0 (this memo's changelog excepted);
   `git grep -n 'build_propagation_path(' -- crates` → the definition, one production call (in `build_dispatch_plan`),
   tests; `git grep -n 'get_the_parent' -- crates` → the definition, its one caller, tests;
   `git grep -n 'struct ActiveDocument' -- crates` → exactly one; `git grep -n 'fn active_document' -- crates` →
   exactly one.
5. Untouched by grep: `git grep -n 'window_entity' -- crates` lists the same lines as H-4 (no deletion, no new
   reader); `git diff --stat origin/main -- crates/core/elidex-ecs crates/shell/elidex-shell/src/pipeline.rs
   crates/script/elidex-js/src/vm/host/event_handler_attrs.rs crates/script/elidex-script-session/src/event_handler_processing/mod.rs`
   → empty except the `ActiveDocument` definition (or fully empty if T3 landed first).
6. T-1..T-10, U-1..U-9, S-1..S-3 present and green; the red set (T-1, T-2, T-3, T-4, T-6, T-7, T-10, U-1, U-2, U-5,
   U-6, U-9, S-1, S-3) shown red on the base (run from the test commit against the pre-fix source commit; no stash).
7. The probe of the ledger row re-run on the head prints `w-cap d-cap b-cap b-bub d-bub w-bub` and
   `BODY > HTML > D > W` (`composedPath()` during dispatch).
8. `wc -l` of every touched file ≤ 1000 (§5).
9. `docs/architecture/script.md:11` describes `build_propagation_path(dom, event, active)` and the Window item.
10. The landing memo corrects the ledger row (§8) and hands §7's texts to the PM: three proposals
    (`#11-window-load-legacy-target-override`, `#11-related-target-retargeting`, `#11-dispatch-at-target-two-pass`)
    and the acceptance additions to three existing rows.
11. **Checker** (run; do not trust a stored verdict): `python3 .claude/skills/elidex-plan-review/preflight.py
    docs/plans/2026-11-event-path-window.md` exits 0.

## §10. Open questions

- **Q-1** `ActiveDocument`'s module: this memo keeps T3's placement (`event_handler_processing/mod.rs`) so neither memo
  edits the other; a crate-level home (`engine.rs`, beside `ScriptContext`, which already carries the bound document)
  would read better for a type two modules share. A pure move either lane can make at the second landing; for the PM
  to say if it wants it now.
- **Q-2** Should the callback-form `dispatch_event` take `active` as a parameter (chosen: its callers are the 17 test
  calls of H-16, verified 2026-11-06, and the Window case needs it, U-6) or be deleted with its tests moved onto a mock engine (then the loop count drops to
  two)? Chosen for this slice: parameter; the loop collapse is `#11-dispatch-at-target-two-pass`'s (§7).
