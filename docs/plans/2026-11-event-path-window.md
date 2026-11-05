# `#11-event-path-window-missing` — the event path reaches the Window: one DOM *get the parent* (per-PR plan memo)

Per-PR plan for the slot `#11-event-path-window-missing` (ledger row registered 2026-11-05, PM 377; direction PM 378:
one DOM *get the parent* function whose Document arm returns the Document's relevant global object except for `load`,
not a Window special case beside the walk). Owner L3, the unit after RC-2a (#596 `17b2a84d`). Written against
`origin/main` = `17b2a84d`; every coordinate below is from that commit.

**Defect.** `build_propagation_path` (`crates/script/elidex-script-session/src/event_dispatch/mod.rs:203`) walks
`EcsDom::get_parent` to the tree root, and the Window entity has no `TreeRelation`
(`crates/core/elidex-ecs/src/dom/mod.rs:491`), so the Window is never on a node-targeted event's path. Runtime
probe (Layout, `f9cf8c40`, VM `dispatchEvent`): capture and bubble `click` listeners on window, document and body fire
`d-cap b-cap b-bub d-bub` (DOM §2.9 order: `w-cap d-cap b-cap b-bub d-bub w-bub`); `composedPath()` during dispatch
is `[body, html, document]`. Every `window.addEventListener` for a node-targeted event never fires — `click`,
`keydown`, `submit`, `input`, `DOMContentLoaded`, and so on.

**Scope.** Two PRs under this one memo (D-4): **PR-A** gives the Document→Window association one ECS home and one
reader (D-2, D-3; behaviour-preserving for the bound document); **PR-B** replaces the tree walk with DOM *get the
parent* (D-1) and builds the path once per dispatch (D-5). Not in scope: the *legacy target override flag*, closed-tree
hiding in the VM's `composedPath()`, `relatedTarget` / touch retargeting, slot assignment on the path (CV P2) — §7.

**Changelog.** *Rev 1* (2026-11-05): first draft.

## §0. Frame, premises, decisions

### §0.1 Coordinate frame

Coordinates are `17b2a84d`. At the implementation base (§9 item 1) every coordinate is re-measured with the commands in
§0.2; the counts are re-derived, not carried. The ledger row's own coordinates (`pipeline.rs:470/516`,
`content/mod.rs:738/762`) are re-stated in §0.3 where they drifted.

### §0.2 Premises (re-measured at `17b2a84d`)

| id | claim | evidence (command → result) |
|---|---|---|
| H-1 | `build_propagation_path` is the only builder of a **node**-targeted path | `git grep -n 'build_propagation_path' -- crates docs` → 13 lines: the definition `event_dispatch/mod.rs:203`; production calls `:262` (`build_dispatch_plan`), `:373` (`dispatch_event`), `:456` (`script_dispatch_event_core`), `crates/script/elidex-js/src/vm/host/event_target_dispatch.rs:120` (the VM `dispatchEvent` walk; its import `:20`); tests `event_dispatch/tests.rs:38/51/60`; the re-export `crates/script/elidex-script-session/src/lib.rs:34`; comments only at `crates/dom/elidex-dom-api/src/focus/sot.rs:78` and `crates/shell/elidex-shell/src/app/events.rs:195`; `docs/architecture/script.md:11` |
| H-2 | A second *get the parent* provider exists for non-entity targets | `git grep -n 'fn vm_event_parent\|fn build_vm_dispatch_plan' -- crates` → both in `crates/script/elidex-js/src/vm/host/event_target_dispatch_vm.rs` (`IDBRequest → IDBTransaction → IDBDatabase`, else null; `ObjectId`-keyed, no entity). Not a path for any node; see D-1 (c) |
| H-3 | The Window entity is a non-Node with no `TreeRelation`, created once per `Vm` at the first bind | `crates/core/elidex-ecs/src/dom/mod.rs:491` (`create_window_root`: `spawn((NodeKind::Window,))`); `crates/script/elidex-js/src/vm/vm_api.rs:227-254` (first bind allocates, later binds reuse `HostData::window_entity`, and the bind caches `window_entity → global_object` in the wrapper cache) |
| H-4 | Three readers answer "which Window belongs to this Document" today | `git grep -n 'window_entity' -- crates` → (i) `EcsDom`'s `window_entity` (`dom/mod.rs:504`, a linear scan for *the* `NodeKind::Window`), read by `crates/script/elidex-script-session/src/event_handler_processing/mod.rs:251` and its tests `tests_insertion_and_write.rs:60/101/235`; (ii) `HostData::window_entity` (`crates/script/elidex-js/src/vm/host_data/engine_feature/bind.rs:492`), read by `vm_api.rs:232`, `crates/script/elidex-js/src/vm/globals.rs:60`, `crates/script/elidex-js/src/vm/host/event_handler_attrs.rs:448/471`, `crates/script/elidex-js/src/vm/host/history_events.rs:187`; (iii) the `ScriptEngine` trait's `window_entity` (`crates/script/elidex-script-session/src/engine.rs:565`, impl `crates/script/elidex-js/src/engine.rs:694`), read by the shell at `crates/shell/elidex-shell/src/pipeline.rs:470/516`, `crates/shell/elidex-shell/src/content/mod.rs:738/762`, `crates/shell/elidex-shell/src/content/message_dispatch.rs:267` |
| H-5 | One `EcsDom` can hold several Document entities; only the bound one has a browsing context | `git grep -n 'create_document_node()' -- crates \| grep -v tests` → `dom/mod.rs:453` (the root), `dom/tree/teardown.rs:302`, `crates/dom/elidex-dom-api/src/element/document_base.rs:508`, `crates/dom/elidex-form/src/inert_document.rs:119`, `crates/script/elidex-script-session/src/scripting.rs:140`. One `ElidexJsEngine` per pipeline (`pipeline.rs:226`), one `EcsDom` per pipeline, an OOP iframe owns its own (`crates/shell/elidex-shell/src/content/iframe/thread.rs:144`) |
| H-6 | Every shell UA dispatch reaches the session builder | `git grep -n 'pipeline\.dispatch_event(\|dispatch_event_bracketed(\|result\.dispatch_event(' -- crates/shell/elidex-shell/src \| grep -v '_tests.rs\|fn dispatch_event_bracketed'` → 35 sites; all go through `PipelineResult::dispatch_event` (`crates/shell/elidex-shell/src/lib.rs:293` → `script_dispatch_event`) or `dispatch_event_bracketed` (`pipeline.rs:121` → `script_dispatch_event`), both → `script_dispatch_event_core` (`mod.rs:434`) → `build_dispatch_plan` + `build_propagation_path` |
| H-7 | The VM-side UA fires share the VM walk | `git grep -n 'dispatch_script_event(' -- crates/script/elidex-js/src \| grep -v 'fn dispatch_script_event\|//'` → 11 call sites: `event_target.rs:727` (`dispatchEvent`), `event_target_dispatch.rs:623/736/821/898`, `history_events.rs:187` (at the Window), `pending_tasks.rs:461` (`postMessage`), `service_worker/event.rs:211`, `worker.rs:278`, `worker_scope.rs:333`, `dom_selection_proto.rs:740` (`selectionchange` at the Document) — all under `crates/script/elidex-js/src/vm/host/`. The walk builds its plan through `build_dispatch_plan` (`event_target_dispatch.rs:119`) |
| H-8 | Readers of the built path | `git grep -n '\.composed_path\b' -- crates \| grep -v 'tests\|//'` → 9 lines: `event_target_dispatch.rs:120/142` (seeds the `composedPath()` Array), `crates/script/elidex-js/src/vm/host/events.rs:187/189` (same, UA path), `crates/script/elidex-js/src/vm/natives_promise.rs:524` (hand-set `[document]`, no builder), `mod.rs:161/166` (`composed_path_for_js`), `mod.rs:373/459` (writes) |
| H-9 | `composed_path_for_js` (`mod.rs:156`) has no production caller | `git grep -n 'composed_path_for_js' -- crates` → the definition, two doc comments, the `lib.rs:34` re-export and `event_dispatch/tests.rs:433/461`. The VM's `composedPath()` returns the seeded Array as is (`crates/script/elidex-js/src/vm/natives_event.rs:143`), so closed-tree hiding (DOM §2.2 `composedPath()` steps 10–15) is not applied — §7 |
| H-10 | Window listeners live in the ECS `EventListeners` component on the Window entity and are collected by the same `collect_listeners` as every node's | `crates/script/elidex-js/src/vm/host/dispatch_target.rs` module doc ("`Node` — the per-entity ECS `EventListeners` component (every DOM node, `Window`, `Document`, …)"); `mod.rs:306` reads `EventListeners` on any entity |
| H-11 | `currentTarget` / `composedPath()` entries are wrappers from the identity cache; the Window's cache entry is the global object | `crates/script/elidex-js/src/vm/host/elements.rs:74` (`create_element_wrapper`: cache hit first); `vm_api.rs:241-254` (bind caches `window_entity → global_object`); `crates/script/elidex-js/src/engine.rs:203` (`call_listener` wraps `event.target` / `current_target` through it); `event_target_dispatch.rs:139-149` (the `composedPath()` Array) |
| H-12 | The tree predicates are total on a non-tree entity | `crates/core/elidex-ecs/src/dom/tree/mod.rs:63` (`read_rel` → `Default` when no `TreeRelation`), `:39` (`is_ancestor` walks `get_parent`, so a Window descendant query is `false`); `crates/core/elidex-ecs/src/dom/tree/walkers.rs:395` (`find_tree_root` of the Window is the Window) |
| H-13 | The three dispatch loops already gate at-target and bubble on the stop-propagation flags | `mod.rs:364` (`dispatch_event`), `:434` (`script_dispatch_event_core`), `event_target_dispatch.rs:42` (`dispatch_script_event`): capture / at-target / bubble each test `propagation_stopped \|\| immediate_propagation_stopped` |
| H-14 | No open branch touches the dispatch module | `for b in $(git branch -r …); do git log --oneline $b --not origin/main -- crates/script/elidex-script-session/src/event_dispatch crates/core/elidex-ecs/src/dom/mod.rs crates/script/elidex-js/src/vm/host/event_target_dispatch.rs; done` → only `origin/rc2a-clone-created` (4 commits = #596's pre-squash history, landed as `17b2a84d`) |
| H-15 | `relatedTarget` is marshalled raw, never retargeted | `crates/core/elidex-plugin/src/event_types.rs:119` (`related_target: Option<u64>` entity bits); `crates/script/elidex-js/src/vm/host/event_shapes.rs:450` (bits → wrapper, no `retarget`). Touch events do not exist. §7 |

### §0.3 Premise errors in the ledger row / brief

- "`build_propagation_path` is the only path builder": true for node targets (H-1), but a second *get the parent*
  provider exists for `VmObject` targets (H-2), and `natives_promise.rs:524` hand-sets a one-item path (H-8). D-1 (c)
  says what this slice does with each.
- "the shell's Window-targeted dispatches (`pipeline.rs:470/516`, `content/mod.rs:738/762`)": the lines are the
  `window_entity()` lookups; the dispatches are `pipeline.rs:474/521/533` and `content/mod.rs:749/772`
  (`message_dispatch.rs:273/320` are two more — `resize`, and the iframe→parent `message`). None puts the Window on a
  node's path, as the row says; after D-1 they are unchanged (the Window's parent is null).
- "Shell input events use the same builder (code read)": confirmed transitively (H-6), not directly — no shell site
  calls `build_propagation_path`; they call `PipelineResult::dispatch_event` / `dispatch_event_bracketed`.
- The brief's "(uses at ~:262 / :373 / :456)" are exact at `17b2a84d`.
- The brief's "`create_window_root` sets no `TreeRelation`" is exact (H-3), and the fix does not add one (D-2).

### §0.4 Decisions

- **D-1 — one *get the parent*.** `event_dispatch::get_the_parent(dom, node, event) -> Option<Entity>` (NEW) in
  `crates/script/elidex-script-session/src/event_dispatch/mod.rs` implements DOM §2.7's default and the three
  overrides as four arms keyed on the entity's components: (a) **Document** (DOM §4.5): `None` if
  `event.event_type == "load"`, else `dom.associated_window(node)` (D-2) — `None` when the Document has no Window
  (no browsing context); (b) **ShadowRoot** (DOM §4.8): `None` if `!event.composed` and `node` is the root of the
  original target (`dom.find_tree_root(event.target)`), else `host`; (c) **Node** (DOM §4.4): `dom.get_parent(node)`
  — the *assigned slot* arm is CV P2's (`docs/plans/2026-10-content-visibility.md:473`, `:627-628`), and the function
  is shaped so that P2 adds it as one `if let` in this arm; (d) **default** (Window, Worker, MessagePort, Selection,
  any other non-Node entity): `None`. `build_propagation_path(dom, event)` is its only production caller and takes the
  `&DispatchEvent` (the ShadowRoot arm needs `target` and `composed`, the Document arm `event_type`); it keeps the
  `MAX_ANCESTOR_DEPTH` bound. The `VmObject` provider `vm_event_parent` (H-2) stays where it is: its targets are
  per-VM `ObjectId` handles (CLAUDE.md side-store exception (a)), it never meets a node, and folding an `ObjectId`
  chain into an `Entity` function would need the engine type in the session crate — §7 records the two-provider
  state once, with its reason. The rejection loop (`natives_promise.rs:524`) is not a consumer and stays as it is
  (its target moves under `#11-unhandledrejection-dispatch-target`, §8).
- **D-2 — the Document→Window association is an ECS component on the Document.** `AssociatedWindow(pub Entity)`
  (NEW, `crates/core/elidex-ecs/src/components.rs`, beside `AssociatedDocument` at `:595`): on a Document entity, the
  Window whose *associated Document* (HTML §7.2.2) that Document is; presence is "the Document has a browsing
  context" (HTML §7.3.2), and the value is the Document's relevant global object (HTML §8.1.3.3.4). **Single writer**:
  `EcsDom::create_window_root(document)` (signature change at `dom/mod.rs:491`) spawns the Window and inserts the
  component on `document` (debug-asserting `NodeKind::Document`). **Single reader**: `EcsDom::associated_window(doc)
  -> Option<Entity>` (NEW). Justification under the side-store rule: a per-entity fact of the Document, the value is
  an `Entity` (`Send + Sync`) in the *same* world as the Document, not a per-VM `ObjectId` — exception (a) does not
  apply (no cross-world aliasing: the Window is spawned in the Document's world, and a despawned Document takes the
  component with it); exception (b) does not apply (it is a single entity's fact, not session state). The linear scan
  `EcsDom`'s `window_entity` (`dom/mod.rs:504`) is the registry idiom the rule forbids, and it is wrong on two real
  shapes: a world with several Documents (H-5: an inert / DOMParser / throwaway Document resolves to the Window it
  does not have) and, after B1's agent-scoped world, a world with several Windows. No `TreeRelation` is added to the
  Window: the tree parent (`get_parent`) and the event parent (`get_the_parent`) stay two concepts.
- **D-3 — one reader for "this Document's Window".** `EcsDom`'s `window_entity` and the `ScriptEngine` trait's `window_entity`
  (trait `crates/script/elidex-script-session/src/engine.rs:565`, impl `crates/script/elidex-js/src/engine.rs:694`)
  are deleted. Their readers resolve through D-2: the shell's five lookups (H-4 (iii)) become
  `dom.associated_window(document).unwrap_or(document)` (the pre-bind fallback is unchanged: no Window, no
  component); `resolve_handler_target` (`event_handler_processing/mod.rs:251`) becomes
  `dom.owner_document(origin).and_then(|d| dom.associated_window(d)).unwrap_or(origin)` (HTML §8.1.8.2: the body
  WEH accessors are exposed on `body` / `frameset` elements *owned by that Window object's associated Document*); the
  VM's body WEH accessors (`event_handler_attrs.rs:448/471`) resolve the same way, so the IDL accessor and the
  inline-handler detector agree on a body in a document that has no Window (both keep it on the element).
  `HostData::window_entity` stays, with one meaning: the entity behind `globalThis` (`vm_api.rs:232`,
  `globals.rs:60`, `history_events.rs:187` dispatching *at* the Window). The bind's reuse branch
  (`vm_api.rs:232`) `debug_assert_eq!(dom.associated_window(document), Some(we))` — the single-world,
  single-document invariant `bind.rs:492` already documents, now checked.
- **D-4 — two PRs, one memo.** PR-A = D-2 + D-3 + D-11 (the data home; behaviour-preserving for the bound document,
  behaviour-changing only for a body in a Window-less document, where the new behaviour is the spec's). PR-B = D-1 +
  D-5 + D-6..D-10 (the path). PR-A first: it is a pure data-home move whose review is Axis 2, and PR-B's review is then
  only the algorithm. Both PRs are per-PR slices of this memo; `/elidex-plan-review` runs once on it (§1).
- **D-5 — the path is built once per dispatch.** `DispatchPlan` (`mod.rs:248`) gains `path: Vec<Entity>` (NEW
  field); `build_dispatch_plan` builds it once and buckets from it; the three re-builds (`mod.rs:373`, `:456`,
  `event_target_dispatch.rs:120`) read `plan.path` instead. `build_propagation_path` stays `pub` (tests) with one
  production caller.
- **D-6 — the event path stays `Vec<Entity>`.** DOM §2.2's *event path item* fields are, at `17b2a84d`, functions of
  (invocation target, original target, event): *shadow-adjusted target* = `apply_retarget` per item (`mod.rs:625`;
  the Window item's is null by DOM §2.9 step 6.9.6.2, and `retarget(target, Window)` yields the outermost light-tree
  host because a Window is not a node — DOM §4.8 *retarget* step 1, H-12); *invocation-target-in-shadow-tree* and
  *root-of-closed-tree* are readable off the entity; *slot-in-closed-tree* is false until P2; *relatedTarget* and
  *touch target list* have no producer (H-15). A per-item struct buys nothing for the Window and is the first
  feature's to introduce that needs a stored field (§7).
- **D-7 — `load`.** The Document arm's `"load"` comparison is the whole exception. The shell keeps firing the
  Document's `load` at the Window (`pipeline.rs:474`; HTML §13.2.7 step 9.5) — path `[Window]`, unchanged. A `load`
  dispatched at a node (`img`, `script`, `link`, `iframe`) stops at the Document: Document capture listeners run,
  Window listeners do not. The *legacy target override flag* (DOM §2.9 step 2; `event.target` reads the Document for
  the Window `load`) is not implemented at HEAD and stays out — §7.
- **D-8 — retargeting and `composedPath()` need no change.** `retarget` (`mod.rs:583`) and `apply_retarget`
  (`:625`) already compute the spec result for a Window invocation target (D-6, H-12). The Window reaches
  `composedPath()` through the wrapper cache (H-11), so `e.composedPath().at(-1) === window` and
  `e.currentTarget === window` hold in both walkers without a new marshalling site. The missing closed-tree hiding
  (H-9) is pre-existing and stays out — §7.
- **D-9 — phases, stop flags, Window-targeted events, the depth cap.** `eventPhase` is unchanged (the Window is a
  capture / bubble item, never at-target for a node event; a Window-targeted event stays `AT_TARGET` with path
  `[Window]`). `stopPropagation()` at the Window's capture item skips at-target and bubble by H-13.
  Window-targeted dispatches (`resize`, `load`, `beforeunload`, `unload`, `message`, `storage`, `popstate`,
  `hashchange`) get path `[Window]` from arm (d) — no doubled Window. The `MAX_ANCESTOR_DEPTH` bound stays in
  `build_propagation_path` (one item deeper now); it is a sibling instance of
  `#11-capped-walker-deep-element-processing`, §8.
- **D-10 — tests.** A new VM end-to-end file, a new session unit file, one shell bind test, PR-A's ECS and consumer
  tests; the red-before-fix controls are named in §6.
- **D-11 — touch-time split in `dom/mod.rs`.** `crates/core/elidex-ecs/src/dom/mod.rs` is 984 lines and PR-A adds
  to it. The non-Node global-scope roots (`create_window_root`, `window_entity`, `create_worker_global_scope_root`,
  `worker_scope_entity`, `dom/mod.rs:491-544`) are one cohesion seam — EventTargets that are not Nodes. PR-A's first
  commit moves them to `crates/core/elidex-ecs/src/dom/global_scope.rs` (NEW) as a pure move (`impl EcsDom` block,
  no behaviour change); the second commit adds D-2 there. `pipeline.rs` (975) is touched by two one-line replacements
  (net 0); no other touched file is near 1000 (§5).

## §1. Shape and verdict

**Edge-dense: yes.** The slice binds six intersecting invariant axes (§2): (A1) the path algorithm, (A2) the
Document→Window data home and its bind lifecycle, (A3) wrapper identity of the Window as `currentTarget` /
`composedPath()` entry, (A4) retargeting with a non-node invocation target, (A5) two walkers consuming one plan,
(A6) worlds with several Documents. Three or more intersect at the Document arm alone (A1 × A2 × A6). Under CLAUDE.md
(*Edge-dense work*), `/elidex-plan-review` is mandatory before implementation and the work is split per PR: D-4's
PR-A / PR-B, each the terminal unit (a narrowly-scoped slice under this reviewed memo). Both PRs are reviewed by the
one plan-review of this memo; the memo is their umbrella and their per-PR plan, because PR-A is a prerequisite move
of the H-4 readers that has no design of its own beyond D-2 / D-3.

**Breadth** (§3): K=2, M=18 → single-PR by preflight's breadth rule; the split is the edge-dense rule's, not breadth's.

## §2. Coupled invariants

- **I-1 path**: the path is DOM §2.9 step 6's item sequence — target, then each *get the parent* until null.
- **I-2 home**: a Document's Window is one component, written once at Window creation, read by one function.
- **I-3 identity**: the Window as an invocation target marshals to the global object (SameObject with `window`).
- **I-4 retarget**: at a non-node invocation target, `event.target` is the outermost light-tree host.
- **I-5 walkers**: the session core loop and the VM walk consume one plan; neither re-derives the path.
- **I-6 documents**: a Document without a Window has no event parent.
- **I-7 load**: the Document arm returns null for `load`; Window-targeted `load` is unaffected.

Pairs:
- I-1 × I-2 = the Document arm is a component read, never a world query (a query is what breaks I-6).
- I-1 × I-4 = the Window item's *shadow-adjusted target* is null (step 6.9.6.2) and the lazy `apply_retarget` at that
  item walks `retarget(target, Window)` to the outermost host (DOM §4.8 step 1, "B is a node" false).
- I-1 × I-5 = `DispatchPlan::path` is built once; `plan.capture` / `plan.bubble` are bucketed from it; both walkers
  read `plan.path` for `composed_path` (D-5).
- I-1 × I-7 = the `"load"` comparison is in the Document arm, so a `load` at a node stops at the Document and a `load`
  at the Window (path `[Window]`) never reaches the arm.
- I-2 × I-3 = `HostData::window_entity` and `AssociatedWindow` name the same entity; the bind asserts it (D-3) and
  the wrapper-cache entry is written in the same bind before any dispatch (H-3, H-11).
- I-2 × I-6 = presence per Document, not per world: the throwaway, inert and DOMParser Documents (H-5) carry no
  component; the bound Document does.
- I-3 × I-5 = both walkers resolve `current_target` through `create_element_wrapper` (H-11); no walker-specific
  Window marshalling.
- I-4 × I-6 = the Window's own tree queries are total (H-12), so no arm needs a "is this a node" pre-check beyond the
  component match.
- I-5 × I-6 = the rejection loop's hand-set `[document]` path is not a consumer (H-8); its Window move is another slot.

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG DOM §2.7 Interface EventTarget | *get the parent*, default | non-Node entity (Window, Worker, MessagePort, Selection) → null | `get_the_parent` arm (d) (NEW) | ✓ | yes |
| WHATWG DOM §4.4 Interface Node | *get the parent* | (i) assigned → assigned slot (CV P2, `content-visibility.md:473`); (ii) otherwise parent | arm (c): (ii) now, (i) P2's | ✗ (P2) | yes |
| WHATWG DOM §4.5 Interface Document | *get the parent* | (i) type is `load` → null; (ii) no browsing context → null; (iii) relevant global object | arm (a): (i) string compare, (ii) component absent, (iii) component value | ✓ | yes |
| WHATWG DOM §4.8 Interface ShadowRoot | *get the parent* | (i) composed unset and root of path[0]'s invocation target → null; (ii) host | arm (b) (HEAD stops at any shadow root when non-composed; equal until P2) | ✓ | yes |
| WHATWG DOM §4.8 Interface ShadowRoot | *retarget* step 1 | (i) A not a node; (ii) A's root not a shadow root; (iii) B a node and A's root a shadow-including inclusive ancestor of B → A; else host, repeat | `retarget` `mod.rs:583` — B = Window falls to "else" (H-12) | ✓ | yes |
| WHATWG DOM §2.9 Dispatching events | step 6.8–6.9.9 (parent loop) | Window parent: 6.9.6 (append with null shadow-adjusted target); other parents 6.9.6 / 6.9.8 | `build_propagation_path` (D-1, D-5) | ✓ | yes |
| WHATWG DOM §2.9 Dispatching events | step 6.13 / 6.14 (capture then bubble over items) | Window item: capture always; bubble iff `bubbles` | `build_dispatch_plan` buckets, three loops (H-13) | ✓ | yes |
| WHATWG DOM §2.9 Dispatching events | *invoke* steps 2–3 (shadow-adjusted target look-back) | null at the Window item → preceding non-null | lazy `apply_retarget` `mod.rs:625` (D-6) | ✓ | yes |
| WHATWG DOM §2.9 Dispatching events | *invoke* step 6 (stop propagation flag) | set at the Window's capture item → at-target and bubble skipped | the three loops' gates (H-13) | ✓ | yes |
| WHATWG DOM §2.9 Dispatching events | step 2 (*targetOverride*, legacy target override flag) | Window `load`: target reads the associated Document | absent at HEAD → `#11-window-load-legacy-target-override` (§7) | ✗ | no |
| WHATWG DOM §2.2 Interface Event | `composedPath()` steps 1–3, 6 (path non-empty; currentTarget appended) | Window entry marshals through the wrapper cache (H-11) | `event_target_dispatch.rs:139-149`, `events.rs:187` | ✓ | yes |
| WHATWG DOM §2.2 Interface Event | `composedPath()` steps 10–15 (closed-tree hiding) | not applied by the VM (H-9) | `composed_path_for_js` unwired → `#11-composedpath-closed-tree-visibility` (§7) | ✗ | yes |
| WHATWG DOM §2.2 Interface Event | *event path item* fields | shadow-adjusted target / in-shadow-tree / root-of-closed-tree derived; relatedTarget / touch target list have no producer (H-15) | `Vec<Entity>` (D-6) | ✗ (relatedTarget, touch) | no |
| WHATWG HTML §8.1.3.3.4 Relevant | relevant global object of a Document | the Window the component names | `AssociatedWindow` value (D-2) | ✓ | no |
| WHATWG HTML §7.2.2 The Window object | *associated Document* | the inverse of `AssociatedWindow`; inverse lookups are a query (none needed here) | `create_window_root(document)` (D-2) | ✓ | no |
| WHATWG HTML §7.3.2 Browsing contexts | a Document's browsing context (null for inert Documents) | component presence (D-2) | `associated_window` (NEW) | ✓ | no |
| WHATWG HTML §13.2.7 The end | step 9.2 (browsing context null → abort), 9.3 (window = relevant global), 9.5 (fire `load` at window, legacy target override) | 9.3 / 9.5 target: `pipeline.rs:470` via D-3; 9.2: the shell's `document` fallback predates this memo; the flag: ✗ (§7) | `dispatch_lifecycle_events` `pipeline.rs:432` | ✗ (the flag) | no |
| WHATWG HTML §8.1.8.2 Event handlers on elements, Document objects, and Window objects | body / frameset WEH exposed on elements *owned by that Window object's associated Document* | a body whose node document has no Window keeps its handler | `resolve_handler_target`, `native_body_weh_get` / `_set` (D-3) | ✓ | yes |

**Breadth**: K=2 (dom, html), M=18 → single PR by breadth; split per §1.

### §3.1 User-input touch audit

- Event type strings from script (`new Event(type)`; `dispatchEvent`): arm (a)'s `"load"` compare is exact and
  case-sensitive, as DOM §4.5 reads it; `"Load"` reaches the Window (spec).
- The `composed` flag from script: arm (b) reads it; a non-composed event at a node inside a shadow tree still stops
  at that root (unchanged).
- Documents created by script (`DOMParser`, `createHTMLDocument`) and by the engine (fragment throwaway, inert form
  document): arm (a) returns null for all of them by component absence — fail-safe direction = no Window.
- `<body on*>` content attributes (parser input, author strings): D-3 routes them by the body's node document's
  component; a body in a Window-less document keeps its own entry (HTML §8.1.8.2 wording).
- Adjacent pre-existing, exposure unchanged: the `MAX_ANCESTOR_DEPTH` truncation of a deep path (silent; `#11-capped-walker-deep-element-processing` is the family); the VM's `composedPath()` exposing closed-tree entries (§7).
- No new network, storage or permission surface.

## §4. Mechanism

### §4.1 PR-A — the data home (D-2, D-3, D-11)

1. **Pure move** (commit 1): `crates/core/elidex-ecs/src/dom/global_scope.rs` (NEW) receives, verbatim, the
   `impl EcsDom` items at `dom/mod.rs:491-544` (`create_window_root`, `window_entity`,
   `create_worker_global_scope_root`, `worker_scope_entity`); `dom/mod.rs` declares the module. `cargo test -p
   elidex-ecs` green with no test change.
2. **Component** (commit 2): `AssociatedWindow(pub Entity)` in `components.rs` with a doc that states the three
   spec facts it carries (HTML §7.2.2 inverse, §7.3.2 presence, §8.1.3.3.4 value). `create_window_root(&mut self,
   document: Entity) -> Entity` spawns `(NodeKind::Window,)` and `insert_one(document, AssociatedWindow(window))`
   (`debug_assert!(matches!(self.node_kind(document), Some(NodeKind::Document)))`). `associated_window(&self, document)
   -> Option<Entity>` reads it and returns `None` for a dangling value (`!self.contains(w)`), the shape of
   `owner_document`'s guard (`dom/mod.rs:875`). `window_entity()` is deleted.
3. **Writers and readers**: `vm_api.rs:235` passes `document`; the reuse branch asserts (D-3). `resolve_handler_target`
   and `native_body_weh_get` / `_set` resolve through `owner_document` → `associated_window` (before C0b lands; C0b
   renames the node-document reader, §8). The five shell sites take `dom.associated_window(document)`; the
   the `ScriptEngine` trait's `window_entity` item and its impl are deleted. Tests that called `create_window_root()` /
   `window_entity()` (`node_kind.rs:119`, `clone.rs:453`, `tests_insertion_and_write.rs:27/60/101/193/235`,
   `tests_window_global.rs:72/123/130/361`) take the returned entity or read the component.

### §4.2 PR-B — the path (D-1, D-5..D-10)

```rust
/// WHATWG DOM §2.7 *get the parent*, with the §4.4 / §4.5 / §4.8 overrides.
/// Called at path-build time only (`debug_assert!(event.original_target.is_none())`).
pub fn get_the_parent(dom: &EcsDom, node: Entity, event: &DispatchEvent) -> Option<Entity> {
    let world = dom.world();
    if let Ok(sr) = world.get::<&ShadowRoot>(node) {                       // §4.8
        let stops = !event.composed && dom.find_tree_root(event.target) == node;
        return (!stops).then_some(sr.host);
    }
    match dom.node_kind(node) {
        Some(NodeKind::Document) => {                                       // §4.5
            if event.event_type == "load" { None } else { dom.associated_window(node) }
        }
        Some(kind) if kind.is_node() => dom.get_parent(node),               // §4.4 (P2: assigned slot first)
        _ => None,                                                          // §2.7 default
    }
}
```

`build_propagation_path(dom, event)` pushes `event.target`, then `get_the_parent` until `None` or the depth bound,
and reverses. `build_dispatch_plan` stores it in `DispatchPlan::path`; `dispatch_event`, `script_dispatch_event_core`
and `dispatch_script_event` set `event.composed_path = plan.path.clone()` (the VM walk owns `local`, so a move). The
`NodeKind::is_node` predicate (`crates/core/elidex-ecs/src/components.rs:538`) is false for Window / Worker /
OffscreenCanvas, so arm (c) never reads a `TreeRelation` off a non-Node. Nothing else in the three loops changes (H-13).
`docs/architecture/script.md:11` is rewritten to the new signature and the Window item.

## §5. Populations

`wc -l` at `17b2a84d` (re-run at the base). The 1000-line audit: `dom/mod.rs` 984 → D-11's split; `pipeline.rs` 975
is touched net 0; `event_target_dispatch.rs` 902 shrinks (D-5); no other file crosses 1000.

| PR | file | lines | change |
|---|---|---|---|
| A | `crates/core/elidex-ecs/src/dom/mod.rs` | 984 | −50 (move), + module decl |
| A | `crates/core/elidex-ecs/src/dom/global_scope.rs` (NEW) | — | the moved block + D-2's writer / reader |
| A | `crates/core/elidex-ecs/src/components.rs` | 822 | +~15 (`AssociatedWindow`) |
| A | `crates/core/elidex-ecs/src/dom/tests/node_kind.rs` | 204 | fixture: `create_window_root(doc)`; A-1 / A-2 |
| A | `crates/core/elidex-ecs/src/dom/tests/associated_document.rs` | 187 | A-1 / A-2 may live here instead (same component family) |
| A | `crates/script/elidex-js/src/vm/vm_api.rs` | 772 | `:235` argument; reuse-branch assert |
| A | `crates/script/elidex-js/src/vm/host/event_handler_attrs.rs` | 683 | `:448`, `:471` resolve via the component |
| A | `crates/script/elidex-js/src/engine.rs` | 829 | −`window_entity` impl (`:694`) |
| A | `crates/script/elidex-script-session/src/engine.rs` | 722 | −trait item (`:565`) |
| A | `crates/script/elidex-script-session/src/event_handler_processing/mod.rs` | 410 | `:251` |
| A | `crates/script/elidex-script-session/src/event_handler_processing/tests_insertion_and_write.rs` | 344 | fixtures `:27`, `:193`; reads `:60`, `:101`, `:235` |
| A | `crates/shell/elidex-shell/src/pipeline.rs` | 975 | `:470`, `:516` (net 0) |
| A | `crates/shell/elidex-shell/src/content/mod.rs` | 865 | `:738`, `:762` |
| A | `crates/shell/elidex-shell/src/content/message_dispatch.rs` | 484 | `:267` |
| A | `crates/dom/elidex-dom-api/src/node_methods/tests/clone.rs` | 720 | `:453` fixture |
| A | `crates/script/elidex-js/src/vm/tests/tests_window_global.rs` | 374 | `:72`, `:123`, `:130`, `:361`; A-5 |
| B | `crates/script/elidex-script-session/src/event_dispatch/mod.rs` | 654 | `get_the_parent` (NEW), `build_propagation_path` signature, `DispatchPlan::path` (NEW), −2 re-builds |
| B | `crates/script/elidex-script-session/src/event_dispatch/tests.rs` | 481 | `:38`, `:51`, `:60` take an event |
| B | `crates/script/elidex-script-session/src/event_dispatch/tests_get_the_parent.rs` (NEW) | — | U-1..U-9 |
| B | `crates/script/elidex-js/src/vm/host/event_target_dispatch.rs` | 902 | `:120` → `plan.path` |
| B | `crates/script/elidex-js/src/vm/tests/tests_event_path_window.rs` (NEW) | — | T-1..T-10 |
| B | `crates/shell/elidex-shell/src/content_eh_attrs_bind_tests.rs` | 390 | S-1 (or a sibling file if a new harness is needed) |
| B | `crates/script/elidex-script-session/src/lib.rs` | 68 | re-export unchanged in names |
| B | `docs/architecture/script.md` | 50 | `:11` |

Not touched: `natives_promise.rs` (961; the rejection loop, §8), `event_target_dispatch_vm.rs` (673; the `VmObject`
provider, D-1 (c)), `history_events.rs` (242; dispatches at `HostData::window_entity`, which stays).

## §6. Tests

Each test names its kind: **red** = fails at `17b2a84d` and passes after; **guard** = passes before and after, and
names the mutant it kills.

### §6.1 PR-A

- **A-1** (red, elidex-ecs): `create_window_root(doc)` → `associated_window(doc) == Some(w)`; `node_kind(w) ==
  Some(NodeKind::Window)`; no `TreeRelation` on `w`.
- **A-2** (guard; kills a world-query reader): two Documents in one world, one Window → `associated_window(doc2) ==
  None`.
- **A-3** (guard): despawning the Window → `associated_window(doc) == None` (dangling guard).
- **A-4** (red, elidex-script-session): `resolve_handler_target(body_in_inert_doc, HandlerScope::Window, &dom) ==
  body_in_inert_doc` while the live body still resolves to the Window (`tests_insertion_and_write.rs` fixture plus an
  inert Document built with `create_document_node`).
- **A-5** (red, elidex-js): `new DOMParser().parseFromString('<body onhashchange="x()">', 'text/html').body
  .onhashchange` is `null` and setting it does not write the Window's entry; the live `document.body.onhashchange`
  round-trips through the Window as today (`tests_event_handler_attrs_window_delegating.rs` is the control).
- **A-6** (guard, elidex-shell): `resize` / `message` / `storage` / `load` still target the Window after the shell
  reads the component — the existing shell tests for these dispatches are the controls; S-2 adds the `[Window]` path
  assertion.

### §6.2 PR-B — VM end-to-end (`tests_event_path_window.rs`, the `tests_dispatch_event.rs` harness)

- **T-1** (red; the probe): capture + bubble `click` listeners on `window`, `document`, `body`;
  `body.dispatchEvent(new Event('click', {bubbles: true}))` → `"w-cap d-cap b-cap b-bub d-bub w-bub"`.
- **T-2** (red): inside the body listener, `e.composedPath()` is `[body, html, document, window]` by `===`.
- **T-3** (red): at the window listeners, `e.currentTarget === window`, `e.target === body`, `e.eventPhase` is `1`
  then `3`.
- **T-4** (red): a non-bubbling event at `body` → `"w-cap d-cap b-cap"` only.
- **T-5** (guard; kills a Window arm without the `load` compare): `window.addEventListener('load', f, true)` +
  `el.dispatchEvent(new Event('load'))` → `f` not called; `document.addEventListener('load', g, true)` → `g` called;
  `window.dispatchEvent(new Event('load'))` → the window listener called once with `e.composedPath()` = `[window]`.
- **T-6** (red): open shadow root on `el`, target inside; the window listener sees `e.target === el` and
  `e.composedPath()` ending `…, el, body, html, document, window` with the inner node and shadow root present (open).
- **T-7** (red): `stopPropagation()` in the window capture listener → no document / body listener runs;
  `stopImmediatePropagation()` there → the second window capture listener does not run.
- **T-8** (guard; kills a doubled Window): `window.dispatchEvent(new Event('resize'))` → one call, `e.eventPhase === 2`,
  `e.composedPath()` = `[window]`.
- **T-9** (guard; kills the linear-scan reader): a `DOMParser` document's body `dispatchEvent(new Event('click',
  {bubbles: true}))` → the window listener is not called; that document's own listener is.
- **T-10** (red): `document.dispatchEvent(new Event('DOMContentLoaded', {bubbles: true}))` → the window bubble
  listener runs (the shell's `pipeline.rs:450` dispatch has this shape).

### §6.3 PR-B — engine-independent (`tests_get_the_parent.rs`, the `event_dispatch/tests.rs` helpers)

- **U-1** (red): `get_the_parent(window, click)` = `None`; `(document, click)` = `Some(window)`; `(document, load)` =
  `None`; a Document with no component → `None`.
- **U-2** (red): `build_propagation_path` for `doc > html > body > p` with a Window = `[window, doc, html, body, p]`;
  `build_dispatch_plan` has the Window first in `capture` and last in `bubble`; `plan.path` equals the built path.
- **U-3** (guard): the same tree with no Window → `[doc, html, body, p]` (today's shape).
- **U-4** (guard): ShadowRoot arm: non-composed event targeted inside the shadow tree → `None` at that root; composed
  → `host`; a non-composed event targeted in the light tree whose walk meets a shadow root cannot occur before P2 —
  stated, not tested.
- **U-5** (red): `apply_retarget` at the Window item for a shadow-internal target → `event.target == host`,
  `original_target == Some(inner)`.
- **U-6** (guard): `composed_path_for_js` with `current_target = window` hides closed-tree entries (the existing
  `:442` shape plus a Window).
- **U-7** (guard): `dispatch_event` (the callback form) invokes the Window's capture listener first and bubble listener
  last, through the `EventListeners` component on the Window.
- **U-8** (guard): the depth bound still terminates a cyclic `TreeRelation` fixture with the Window present.
- **U-9** (red): `script_dispatch_event_core` sets `event.composed_path` to the plan's path (one build): a test engine
  counting `get_parent` reads is not available; the assertion is `event.composed_path == plan.path` on a mock
  `ScriptEngine`.

### §6.4 PR-B — shell (`content_eh_attrs_bind_tests.rs` harness)

- **S-1** (red): a page whose inline script runs `window.addEventListener('click', e => document.body.setAttribute(
  'data-w', e.currentTarget === window))`; `click(pipeline, "t")` → `data-w="true"`.
- **S-2** (guard): `dispatch_lifecycle_events`' `load` reaches a `window.addEventListener('load')` listener with
  `e.composedPath().length === 1`.

## §7. Out of scope and slots

Each item states the defect at HEAD, then the slot. Registration is the PM's at landing; the names are proposals.

- **Two *get the parent* providers.** `vm_event_parent` (`event_target_dispatch_vm.rs`) chains `ObjectId` targets;
  `get_the_parent` chains entities. Reason: the `VmObject` targets are per-VM identity handles (side-store exception
  (a)); one function would need the engine's `ObjectId` in the session crate. Not a defect; recorded here once so no
  later reader re-opens it. Re-eval if a `VmObject` ever needs a node parent.
- **Window `load` target.** `pipeline.rs:474` fires the Document's `load` at the Window without the *legacy target
  override flag*: `event.target === window`, spec `document` (DOM §2.9 step 2; HTML §13.2.7 step 9.5). Needs the
  Window→Document direction (a query over `AssociatedWindow`, or the Window-side inverse) → `#11-window-load-legacy-
  target-override`.
- **`composedPath()` closed-tree hiding.** The VM returns the whole path from every listener (H-9); DOM §2.2 steps
  10–15 hide a closed shadow tree's entries from listeners outside it. `composed_path_for_js` implements a per-
  `currentTarget` filter that nothing calls; wiring it means a per-item Array in the VM walk, or deleting it
  (dead-code discipline). → `#11-composedpath-closed-tree-visibility`.
- **`relatedTarget` retargeting.** `MouseEvent` / `FocusEvent` `relatedTarget` is marshalled raw (H-15); DOM §2.9
  steps 4 and 6.9.3 retarget it per item, and step 6.7 ends the path when a parent is the relatedTarget. The first
  consumer introduces the per-item struct D-6 defers → `#11-related-target-retargeting`.
- **Slot assignment on the path.** CV P2 (`content-visibility.md:473`, `:627-628`): the Node arm's assigned-slot
  branch, `slotInClosedTree`, and the ShadowRoot arm's path-root test becoming load-bearing.
- **`unhandledrejection` at the Window.** `#11-unhandledrejection-dispatch-target` (open): the rejection loop targets
  the Document with a hand-built path; when the target moves it should go through the shared walker, and the Window's
  path is then `[Window]`.
- **The depth bound.** `#11-capped-walker-deep-element-processing` (open) is the family; the path builder's bound is
  one more instance (D-9).
- **B1 multi-Window worlds.** `#11-browsing-context-state-ecs-components` / B1: `AssociatedWindow` is the first
  per-Document browsing-context fact on the entity; B1 decides whether the Window creates it (as here) or the
  navigable does.

## §8. Edges and hand-offs

- **CV P2 (Layout)** — `docs/plans/2026-10-content-visibility.md:473` ("Node's *get the parent* (DOM §4.4 Interface
  Node) P2's (§5.3)"), `:627-628` (P2-R3: the *assigned slot* never clears; "the event path's *get the parent* /
  `composedPath`"), `:762`. P2 adds the assigned-slot `if let` to arm (c) and nothing else to the function; the
  ShadowRoot arm's path-root test (b) is already the spec's form, so P2 does not revisit it. Hand-off: this memo's
  `get_the_parent` is the one function P2 extends; P2's memo cites it by symbol.
- **C0b (`docs/plans/2026-11-eh-attrs-c0b-node-document-fact.md`, unlanded)** — renames the node-document reader
  (`owner_document` → `node_document`). D-3's two consumers read `owner_document` until C0b lands; C0b's rename sweep
  includes them (two sites, named in §4.1 step 3). If C0b lands first, PR-A reads `node_document`.
- **EH-attrs slice-a (`docs/plans/2026-10-eh-attrs-slice-a.md:597`)** — its session fixtures call
  `create_window_root` with no argument; PR-A changes the signature, so that memo's fixture line is read as
  `create_window_root(doc)` (one wording edit at its next revision; the landed tests are PR-A's population).
- **IBP handler gates (`docs/plans/2026-10-ibp-sandbox-handler-gates.md:905`, landed)** — NR5 notes the rejection
  loop's Document target; unchanged here.
- **No open branch** touches `event_dispatch/`, `dom/mod.rs` or `event_target_dispatch.rs` (H-14).
- **Memory / ledger** — the slot row's "`build_propagation_path` is the only path builder" is corrected per §0.3 at
  landing; `docs/architecture/script.md:11` is rewritten in PR-B.

## §9. Definition of done

1. Base re-measure: every command in §0.2 re-run at the base; counts and lines updated in this memo (rev bump).
2. PR-A: `cargo test -p elidex-ecs -p elidex-script-session -p elidex-js -p elidex-shell -p elidex-dom-api
   --all-features` green; `git grep -n 'window_entity' -- crates` lists only `HostData::window_entity` and its four
   readers (`vm_api.rs`, `globals.rs`, `history_events.rs`, tests) — neither `EcsDom`'s nor the `ScriptEngine` trait's
   `window_entity` exists; `git grep -n 'fn window_entity' -- crates` → exactly one definition.
3. PR-A: A-1..A-6 present and green; A-1, A-4, A-5 shown red on the base (`git stash`-free: run them from the test
   commit against the pre-fix source commit).
4. PR-B: `git grep -n 'build_propagation_path(' -- crates` → the definition, one production call (in
   `build_dispatch_plan`), tests; `git grep -n 'get_the_parent' -- crates` → the definition and its one caller.
5. PR-B: T-1..T-10, U-1..U-9, S-1..S-2 present and green; the red set (T-1..T-4, T-6, T-7, T-10, U-1, U-2, U-5, U-9,
   S-1) shown red on PR-A's head.
6. The probe of the ledger row re-run on PR-B's head prints `w-cap d-cap b-cap b-bub d-bub w-bub` and
   `BODY > HTML > D > W` (`composedPath()` during dispatch).
7. `wc -l` of every touched file ≤ 1000; `dom/mod.rs` after D-11 < 984.
8. `mise run ci` green on each PR; `/elidex-review` on each; `/external-review` after push.
9. `docs/architecture/script.md:11` describes `build_propagation_path(dom, event)` and the Window item.
10. The landing memo corrects the ledger row (§8) and registers §7's proposed slots or records why not.

## §10. Open questions

- **Q-1** Does B1 (agent-scoped `EcsDom` world) want the Window to write `AssociatedWindow` at creation (D-2), or the
  navigable's creation steps to? D-2 picks the Window's creation because that is where the pair exists today
  (`vm_api.rs:235`); B1's memo decides whether it moves.
- **Q-2** `create_window_root(document)` vs a separate `set_associated_window`: one entry is chosen (the spec creates
  the Window with its associated Document). If a reviewer wants a rebind to a different Document to re-link rather
  than assert (D-3), that is the second writer this memo avoids — decide at review.
- **Q-3** Should PR-B delete `composed_path_for_js` (dead in production, H-9) or leave it for the closed-tree slot?
  This memo leaves it (U-6 extends its test); the slot decides.
