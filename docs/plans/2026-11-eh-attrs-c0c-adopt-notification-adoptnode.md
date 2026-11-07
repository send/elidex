# C0c `EH-attrs-at-creation-C0c` — the adoption notification (`adoptedCallback`) and `document.adoptNode` (per-PR plan memo)

Per-PR slice **C0c** of the C0 program (program memo `docs/plans/2026-10-eh-attrs-c0-insertion-adoption.md`, rev 3.8;
"the program memo"; its revs 3.3–3.5 do not touch C0c's decisions), under the umbrella `docs/plans/2026-10-event-handler-attrs-at-creation.md`. It carries program
decisions **D-3 (c), D-6, D-13, D-19**, after its prerequisite pure-move split **D-14 (b)**, on C0b's head. It closes
slot `#11-custom-elements-adopted-callback` (D-17 #227) and, as the program's last PR, slot
`#11-cross-document-adopt-on-insert` (program R-1, R-2). Program IDs are cited as the program memo's; C0b's as the C0b
memo's (`docs/plans/2026-11-eh-attrs-c0b-node-document-fact.md`, "the C0b memo").

**Placement.** PM 158 (`adoptNode` in the C0 program); PM 244 (C0c = D-3 (c), D-6 with its consumer arms and the VM
flush, and D-13, after D-14 (b); any time after C0b and before C1); PM 311 (after #583, edge c0h). The `Adopt` variant ships with its consumer (no
consumer-less event).

**Scope.**
1. The adopt walk's contents segments and skipped contents fragments (D-3 (c)).
2. `MutationEvent::Adopt` (NEW) with `AdoptedContents` (NEW), dispatched once per adopt call after the walk and before
   the link (D-6).
3. The custom-element consumer's `Adopt` arm and the VM flush's `adoptedCallback` invocation (D-6); explicit
   arms in the exhaustive matches (a no-op in `range/live.rs`, a recording arm in the test recorder `mutation_hook.rs`;
   *Rev 3.7.5*).
4. `TemplateContentsWalk::Include` in the spec's order (D-19).
5. `document.adoptNode(node)` (D-13): VM native → dom-api `AdoptNode` → session `apply_adopt_node` → `EcsDom::adopt`.

**Not in scope.** `importNode` (proposed `#11-import-node`); registry steps 3.2 / 3.3.2 — scoped half `#11-shadow-scoped-custom-element-registry`, non-scoped half (3.3.2: a null-registry document's element adopted into a registry-bearing one) L3's proposed `#11-document-null-registry-adopt-reset`, which hooks this PR's `Adopt` (§7); the `img` adopting steps
(proposed `#11-img-adopting-steps-relevant-mutations`, whose hook this PR provides); the engine-wide reaction drain
point (`#11-ce-reactions-method-scope-drain`, L3's proposal); C1's `f` values of the segments (C1 D-C5, T25).

**Changelog.** *Rev 3.7.5* (2026-11-08, implementation errata; measured at the base `062337aa` and at C0c's head):
D-6's lookup trigger names the consumer's second case (a reached contents fragment with no segment, i.e. a skipped one)
and the walk's (a `None` segment, for templates); the lookup is DOM §4.8's *shadow-including root*, computed by the
existing `EcsDom::find_tree_root_composed` on both sides, and §8's stated defect is restated for it; H-c4 / D-13 step 3:
handlers push records with `session.push_notify_record` loops (no public `push_records`); §5 P-c1 adds
`elidex-form/src/clone.rs:67` and states each caller's order-sensitivity; §5 P-c2 is 1 hit at the base and is listed
by class at the head; §0.1's factories range and where the coordinates now live; `MutationEvent` has eight variants at
the base (H-c1, D-6, §4.2): `live.rs`'s `Adopt` joins the shared no-op arm, the test recorder records `Adopt`; §6.1's
cells use `globalThis.<name>`; D-6 Flush: the lifecycle callback is read at reaction time (a pre-existing deviation).
*Rev 3.7.4* (2026-11-03, PM 319): D-6's lookup runs only when needed and its depth cap is a stated
defect (§8); D-3 (c) gives the None-under-None rationale; H-c6 / D-13 receiver corrected to `document_receiver`; T15 (iv).
*Rev 3.7.3* (2026-11-03, focused re-check after PM 315; with the program memo's rev 3.9): the skip covers
the Attr re-home (D-3 (c), U19, §6.3); U19's legs get distinct documents and an unambiguous order; §2 A1×A4 says what
tree position chooses. *Rev 3.7.2* (2026-11-03, PM 317 / 318; with the program memo's rev 3.8): #583 landed (`12ea5dce`):
DoD 8 lists its four `dom/mod.rs` statements on main, edge c0h; the `live.rs` / `mutation_hook.rs` splits are L3
RC-0a's (PM 318): header, D-14, §5, c0-pre-b, DoD 1. *Rev 3.7.1* (2026-11-03): sweep after PM 315 — scope item 1, §2 A3×A4, §3 and §4.2 point at D-3 (c) /
D-6 instead of implying a segment per fragment; U19's third leg and §6.3's row restore the pin of a nested segment
under a `None` (silent re-home) segment, dropped by rev 3.7's reshaping of the second leg. *Rev 3.7* (2026-11-03, PM 315): a contents fragment whose `old` equals `new` is skipped by both the
stamps and the segments, and a skipped fragment reads `None` in D-6's lookup (D-3 (c), D-6, I-A4, U19, §6.3); C0c's
PR corrects #583's `dom/mod.rs` statements (DoD 8; §8's PM line deleted); T15r's spec column is derived, not measured.
*Rev 3.6* (2026-11-03, with the program memo's rev 3.6): plan-review at `76af676a` (0 CRIT / 8 IMP /
22 MIN) and PM 306 / 311; dispositions §10. Reactions read in their own eval (§6.1); red-first base table (§6.5);
`adoptNode` registered as an own property in `document.rs`; the segment lookup (D-6) and the skipped-segment rule
(D-3 (c)); the null-registry regression recorded (§1, §8, T15r); edge c0h; R-6 (b) / (f) in DoD 7. *Rev 3.2* (2026-11-03, with the program memo's rev 3.2; PM 273): DoD 8 — the C0 program PR that lands
after #583 compares the shared points' wording (program §9 item 6). *Rev 3.1* (2026-11-03, with the program memo's rev 3.1): §1 states why `adoptNode` stays in this PR (R2
Ax3 MIN-7); D-14 (e)'s L3 side is RC-0a (L3 #583 `55d35a16`). *Rev 1* (2026-11-03): first draft, carved from the C0 memo rev 2.1 (`d262ca8c`) D-3 (c) / D-6 / D-13 /
T6, T7, T14–T16, T18 / U13 per plan-review R1's SPLIT and PM 244; D-19 decided (R1 Ax1 MIN-2); §4.7's layering table
added (Ax1 MIN-1); `[CEReactions]` drain point stated (Ax4 MIN).

## §0. Frame, premises, decisions

### §0.1 Coordinate frame

Coordinates are `27b50dfe` (program §0.1). C0c's base is C0b's head plus D-14 (b), which moves
`crates/script/elidex-js/src/vm/host/document.rs`'s node factories (`:278-421`) to `vm/host/document_factories.rs`
(NEW) and its accessors (`:423-938`) to `vm/host/document_accessors.rs` (NEW). *Rev 3.7.5:* the factories range was
inconsistent from the start: it overlaps the installer (`:285`) and the method table (`:379-398`), which stay in
`document.rs`. D-14 (b) landed as #599; at the base `062337aa`, `document_factories.rs` is 162 lines,
`document_accessors.rs` 590 and `document.rs` 499, and the moved coordinates are read there. L3 RC-2a may have landed (edge c0e);
its files that C0c also edits (`crates/core/elidex-ecs/src/dom/mutation_event.rs`, `dom/tree/walkers.rs`, `dom/mod.rs`,
`crates/core/elidex-ecs/src/lib.rs`, `crates/dom/elidex-custom-elements/src/consumer.rs`,
`crates/dom/elidex-dom-api/src/range/live.rs`) are in the re-measure scope (§9 item 1).

### §0.2 Premises

Program H-9, H-12, H-15, H-16 bind this PR. Added:

| id | claim | evidence (`27b50dfe`) |
|---|---|---|
| H-c1 | The CE consumer's dispatch entry | `CustomElementReactionConsumer::handle` (`crates/dom/elidex-custom-elements/src/consumer.rs:75-96`): `Insert`, `Remove`, `AttributeChange`, `_ => {}`; `handle_insert` `:98`. At the base `062337aa` (*Rev 3.7.5*): `handle` `:89-113`, arms `Insert`, `Remove`, `AttributeChange`, `CloneCreated` (RC-2a), `_ => {}` |
| H-c2 | The flush's reaction loop and its callback helper | `flush_ce_reactions` (`crates/script/elidex-js/src/vm/host/custom_elements/flush.rs:39`); the `AttributeChanged` arm calls `invoke_callback(&mut ctx, entity, cb_sid, &args)` (`:131`); the `Adopted` arm is a no-op (`:133-138`) |
| H-c3 | Where reactions drain | `flush_ce_reactions` is called at script-execution and event checkpoints: `crates/script/elidex-js/src/vm/interpreter.rs:54`, `crates/script/elidex-js/src/engine.rs:165`, `crates/script/elidex-js/src/vm/natives_timer.rs:281`, `crates/script/elidex-js/src/vm/host/custom_elements/define.rs:204` (`git grep -n 'flush_ce_reactions()' -- crates/script`); not at a `[CEReactions]` method's return (L3 memo Rev 5, E2 measurement) |
| H-c4 | The VM → engine-independent dispatch | `invoke_dom_api` (`crates/script/elidex-js/src/vm/host/dom_bridge.rs:462`) calls a registered dom-api handler, maps a `DomApiError` to its DOMException and allocates the returned node's wrapper (`:516-523`); handlers push session records in a `session.push_notify_record(record)` loop (e.g. `crates/dom/elidex-dom-api/src/element/tree.rs:294-296`; *Rev 3.7.5*: `push_records` is not there, and the only `push_records`, `child_node/mutations.rs:52`, is private to that module) |
| H-c5 | Custom elements inside template contents exist | P-4 (C1 memo P-C14): an element upgraded in a document and appended into `tpl.content` stays custom (adoption does not un-upgrade), so per-node pairs are load-bearing |
| H-c6 | Document methods are own properties | no `Document.prototype`: `install_document_methods_for_entity` (`crates/script/elidex-js/src/vm/host/document.rs:285`) installs `DOCUMENT_METHODS_POST_LIVE_COLLECTION` (`:379-398`) on each Document wrapper, install order = own-property order (`:306-323`); callers `:269` (bind), `dom_parser.rs:305`, `node_methods_extras.rs:263`. Unbound VM or non-HostObject `this` → `null`; a non-Document HostObject → `TypeError` "Illegal invocation" (`document_receiver` `:83-88` → `require_receiver`, `event_target.rs:331-363`) |

### §0.3 Premise errors

- **P-4 (program, restated).** "Elements in contents can be custom only if upgraded there" (C0 rev 1) was wrong (H-c5);
  folded as per-segment pairs (D-3 (c), D-6).
- **P-c1 (rev 2 Q-1, now decided).** One event per contents segment would follow the spec's structure (each template's
  adopting steps are a nested *adopt*), but a consumer would then emit contents callbacks after every outer one, out of
  spec order; one event per adopt call carrying all segments lets the consumer walk once in spec order (D-6).

### §0.4 Decisions (this PR; full statements in the program memo)

- **D-3 (c) — segments.** The walk (C0b's private helper in `tree/mutation.rs`) collects, for each contents fragment it
  reaches and does not skip (below), `AdoptedContents { fragment, documents }` with `old` = the fragment's node document read before the walk
  writes it and `new` = the target it writes (C2: `document`; C1: `f(document)`), in walk order; `documents = None` when
  `old` is `None`, or when the template lies in a `None` segment (a design choice: that state is reachable only from
  fixtures, A4×A8, so no behaviour depends on it). A fragment is **skipped** when its `old` equals `new`
  (the nested adopt's step 3 guard, "If document is not oldDocument", `#concept-node-adopt`) or its template's innermost
  contents fragment (D-6's lookup) is skipped: the walk writes no stamp to it or to a node whose innermost contents
  fragment is skipped, re-homes none of such a node's Attrs (the guard gates the whole loop, 3.1 and 3.3.1), and
  records no segment for it. C0c is the first place this is reachable (`adoptNode` can put
  contents in another document than their template's).
- **D-6 — the event and its consumers.** `MutationEvent::Adopt { node, documents: Option<(Entity, Entity)>, contents:
  &'a [AdoptedContents] }`; `adopt` dispatches it after the walk, before returning to the primitive (so before
  `link_node` and `Insert`), iff `documents` is `Some` (*adopt* step 3's guard; every segment under a `None` pair is
  `None`, D-3 (c)). **Consumer:**
  `CustomElementReactionConsumer::handle` gains `MutationEvent::Adopt { .. } => self.handle_adopt(..)` (its parameter
  names `&[AdoptedContents]`, re-exported beside `MutationEvent`, §5), which walks
  `node`'s subtree with `for_each_uncapped_shadow_inclusive(node, TemplateContentsWalk::Include, …)` (spec order after
  D-19) and enqueues `CustomElementReaction::Adopted { entity, old_document, new_document }`
  for each element in `CEState::Custom` whose pair is `Some` (*adopt* 3.3.3: "enqueue a custom element callback
  reaction with inclusiveDescendant, callback name "adoptedCallback", and « oldDocument, document »"). *An element's
  pair* is its innermost segment's: `r` = `e`'s *shadow-including root* (DOM §4.8, `#concept-shadow-including-root`),
  computed by the existing `EcsDom::find_tree_root_composed` (`walkers.rs`) on both sides, the walk and the consumer
  (*Rev 3.7.5*; one issue, one way: C0c's first slice, `553e3b3e`, had two hand-written copies of the climb, one per
  side); a contents fragment has no parent and is not a shadow root, so the climb stops at the nearest one; if `r` is a
  segment's `fragment`, that segment's `documents`; if `r` is `node`, the root's; else `None` (a skipped fragment has no
  segment, D-3 (c)). The nearest contents fragment is the innermost segment, so leaving a segment needs no signal and the
  walker keeps its one visit callback. *Rejected:* an exit callback on the walker (a second shape of the one walk, which
  D-19 rejects) and a consumer-side segment stack (popping it needs the same ancestor test). The lookup runs only when needed (*Rev 3.7.5*): in the walk,
  once a fragment is skipped (for every node) or a `None` segment is recorded, the root's included (for templates, so
  a nested segment reads `None`); in the consumer, from the start when a segment's pair differs from the root's, and
  otherwise from the point where its walk reaches a contents fragment that has no segment (a skipped one; without this
  its custom elements would take the root's pair, and U19's second leg would give `Adopted(D0, target)` where it gives
  none). Otherwise every node takes
  the root's pair. Its depth cap: §8. **Flush:** the
  `Adopted` arm (`flush.rs:133-138`) resolves the two Document wrappers and calls `invoke_callback(&mut ctx, entity,
  adopted_callback_sid, &[old, new])` (H-c2), with the same error routing as `connectedCallback`. *Rev 3.7.5:* like
  `connectedCallback` and the other callbacks, `invoke_callback` reads `adoptedCallback` from `constructor.prototype`
  when the reaction runs (the definition stores no lifecycle callbacks; `well_known.adopted_callback` was already
  interned). This is a pre-existing deviation, not fixed here: HTML `define()` (`#dom-customelementregistry-define`)
  reads the lifecycle callbacks once, at definition time (steps 14.3–14.4), and stores them in the definition (step
  15). **Other consumers:**
  `range/live.rs:113-163` (the one exhaustive production match) gains an explicit no-op `Adopt` arm (live ranges are
  updated by adopt step 2's `Remove`); the test match `crates/core/elidex-ecs/src/dom/tests/mutation_hook.rs:68-148`
  likewise; the others have `_ => {}` (program P-6). *Rev 3.7.5:* at the base `MutationEvent` has eight variants
  (RC-2a's `CloneCreated`); `live.rs:113-164`'s no-op arm is shared by `AttributeChange` / `CloneCreated`, and clippy
  (`match_same_arms`, pedantic, denied in CI) rejects a second identical arm, so `Adopt` joins that arm; the test
  recorder's match stays in `mutation_hook.rs` after RC-0a's split (`:72-158` at the base) and its `Adopt` arm records
  the event (`MockEvent::Adopt`) as its other arms do, not a no-op. *Batch equivalence:* the spec interleaves "set node document" and
  "enqueue" per descendant; enqueueing runs no script (reactions run at the next drain, H-c3), so re-homing first and
  enqueueing after, in the same order, is unobservable. *Idiom table:* OO engines' per-node-class "moved to new
  document" virtual override (`adoptedCallback` dispatch inside the adopt loop) → one event on the mutation stream and
  one consumer arm per system that cares, as `connectedCallback` already is.
- **D-19 — the walker's order.** In `for_each_uncapped_shadow_inclusive` (`crates/core/elidex-ecs/src/dom/tree/walkers.rs:99`),
  `Include` pushes the contents fragment **after** the light children and the shadow root (so it pops first, right
  after the template node), instead of before them (`:112-118`); this revises slice a §4.3
  (`docs/plans/2026-10-eh-attrs-slice-a.md:350`, #563), which pushed the fragment first. Its docstring (`:86-91`) changes
  from "visited after the template's own subtree" to "visited right after the template (DOM §4.5 *adopt* step 3.4 runs
  the template's adopting steps, whose step 2 adopts the contents, HTML §4.12.3, inside the per-descendant loop)".
  `despawn_subtree` (`teardown.rs:197`) and the adopt walk are
  order-insensitive (H-16), so only the consumer observes it (U13, U18).
- **D-13 — `document.adoptNode(node)`.** *VM* (marshalling): `native_document_adopt_node` (NEW) in
  `vm/host/document.rs` (where the installer and the method tables stay after D-14 (b)), installed as an own property
  by an `("adoptNode", …)` entry appended to `DOCUMENT_METHODS_POST_LIVE_COLLECTION` (H-c6; appending keeps the
  existing names' order). It brand-checks `this` with `document_receiver`, as its siblings (H-c6), unwraps
  `node` (non-Node → `TypeError` per WebIDL) and calls `invoke_dom_api("adoptNode", …)`, which maps errors and
  returns node's wrapper (H-c4). *dom-api*: `AdoptNode` (NEW) in `crates/dom/elidex-dom-api/src/document.rs`,
  registered in `registry.rs`: step 1 a Document → `NotSupportedError`; step 2 a ShadowRoot → `HierarchyRequestError`;
  step 3 pushes each record of `apply_adopt_node(dom, node, this)` with `session.push_notify_record` (a loop, as the
  other handlers; *Rev 3.7.5*, H-c4); step 4 return node. A host-linked DocumentFragment
  (template contents) is adoptable: DOM's `adoptNode` has no exclusion (`webref body dom dom-document-adoptnode`, run
  2026-11-03). *Session*: `apply_adopt_node(dom, node, document) -> Vec<MutationRecord>` (NEW) in
  `mutation/child_list.rs`: if `node` has a parent, capture `source_removal_record(dom, node)` (H-7) pre-write; call
  `dom.adopt(node, document)`; return the record (adopt step 2's *remove* queues it, DOM §4.2.3 *remove* step 16;
  adoption itself queues none).
  *`[CEReactions]`* (`webref idl dom Document`: `[CEReactions] Node adoptNode(Node node)`): the reactions it enqueues
  (`Adopted`, and `Disconnected` from step 2's removal) drain at the next checkpoint like every other `[CEReactions]`
  method's (H-c3); the method-scope drain is the engine-wide gap L3 proposes as `#11-ce-reactions-method-scope-drain`
  (L3 memo §9, PM 224), not this PR's. An element whose `Upgrade` reaction is still pending when it is adopted in the
  same script loses `adoptedCallback` (not delayed: `handle_adopt` reads `CEState::Custom`); this is recorded on
  `#11-ce-reactions-method-scope-drain` (proposed in L3 #583; the PM adds the line at registration, PM 306); C0c adds
  no workaround. An adopt applied by the shell's unbound flush reaches no consumer (dispatcher cleared at `unbind`,
  `vm/host/custom_elements/records.rs:22-30`), and the record leg converts only child-list and attribute records
  (`:74-138`), so its `adoptedCallback` is lost too (same slot).
- **D-14 (b) — prerequisite.** As program D-14: (b) `vm/host/document.rs` (1197), its own PR, a pure move. The splits
  of `range/live.rs` (1021) and `dom/tests/mutation_hook.rs` (1143), program D-14 (d) / (e), are L3 RC-0a's (PM 318);
  when C0c's base includes RC-0a, C0c is written on the split files.

## §1. Shape and verdict

**Axes (program §2): A4 notification, with A1, A2, A3, A5, A7, A8, A9 at the 8 pairs marked C0c.** Edge-dense as a
slice of the program; plan-review mandatory; a terminal unit once passed (base case (c)). Why `adoptNode` travels with
the notification (program §1; the coupling is ordering, not mechanism): before the notification it would ship a new
`[CEReactions]` method without its callback (T15); after it, as its own PR, it would carry no mechanism of its own (one
session function over C0b's `adopt` plus marshalling, D-13) and one pair of its own, A1×A9 (T16, the removal record);
its pair with the notification, A4×A9, is pinned by T15, which needs both halves; PM 158 put it in the program as the
adopt algorithm's own entry. Splitting it buys no smaller decision surface for one more per-PR memo and review.

**K-0 lens.** Adds `adoptedCallback` and `adoptNode` where HEAD has none (R-5). Same-document moves enqueue nothing
(T14). One recorded regression (2026-11-03, PM 311; §8): a custom element that elidex wrongly upgrades inside a
null-registry document (e.g. DOMParser) gets an extra `adoptedCallback(dp, document)` when adopted; HEAD fires none
(T15r). Its window closes with `#11-document-null-registry-adopt-reset` (trigger: C0c lands).

## §2. Coupled invariants (program §2.2 pairs with PR = C0c)

| pair | intersection | pinned by |
|---|---|---|
| A1×A4 | each pair's `old` is a stamp read before the walk, `new` the stamp written; tree position only chooses which pair applies (D-6), no node document is derived from it | U13, T18 |
| A1×A9 | `adoptNode`'s record is the removal's; the stamp change is no record | T16 |
| A2×A4 | `Adopt` precedes `Insert`; in `replace_child`, `Adopt(new_child)` precedes `Remove(old_child)` | U4 (C0c part), T7 |
| A3×A4 | the consumer visits the walk's set in spec order (D-19), each node with its pair (D-6's lookup) | U13, U18, T18 |
| A4×A5 | builds adopt nothing (C0b D-10) and run with the dispatcher taken, so no `adoptedCallback` from a build | C0b's U9 walk counter (`Adopt` is dispatched only from a walk) |
| A4×A7 | `adoptedCallback` runs at the next drain, after the insertion completes; slice a's processing is inside `Insert` | T6 |
| A4×A8 | after C0b every script-reachable custom element has a node document, so the silent re-home cannot swallow a callback | U2 (C0b) re-run: no `Adopt` for an ownerless root |
| A4×A9 | `adoptNode` fires `Adopt` like an insertion's adoption; removal record, then `Adopted` | T15, T16 |

**I-A4 (axis invariant).** `Adopt` fires once per adopt call iff the root's pair is a (live, different) document pair,
before that root's `Insert`; every custom element of the adopted subtree whose pair is `Some` gets exactly one `Adopted`
reaction with its own pair, in spec order (contents right after their template), enqueued before its `Connected`; an
element inside a `None` or skipped segment gets none (D-3 (c)).

## §3. Spec coverage map

Lookups (2026-11-03): `heading dom 4.5 / 4.8 / 4.3.2 / 4.2.3`; `heading html 4.12.3 / 4.13.6`; `body dom concept-node-adopt`
(3, 3.3.3, 3.4), `dom-document-adoptnode` (1–4), `concept-node-remove` (16); `body html the-template-element` (adopting
steps 1–2); `dfn html 'appropriate template contents owner document'` → §4.12.3; `dfn html 'enqueue a custom element callback reaction'` → §4.13.6;
`dfn html CEReactions` → §4.13.6 `#cereactions`; `dfn dom 'shadow-including tree order'` → §4.8; `idl dom Document`.

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG DOM §4.5 Interface Document | *adopt* 3.3.3 | custom → enqueue `adoptedCallback` « oldDocument, document »: one event per adopt call, each node its own pair (D-6's lookup) | `MutationEvent::Adopt` (NEW) in `crates/core/elidex-ecs/src/dom/mutation_event.rs:107`; consumer `crates/dom/elidex-custom-elements/src/consumer.rs:75` | ✓ | yes |
| WHATWG DOM §4.5 Interface Document | *adopt* 3.4 (template adopting steps) | each contents fragment's (old, new) recorded where the walk writes it | the walk's contents edge (C0b's private helper in `crates/core/elidex-ecs/src/dom/tree/mutation.rs`) | ✓ | yes |
| WHATWG DOM §4.5 Interface Document | `adoptNode` 1–4 | Document → NotSupportedError; ShadowRoot → HierarchyRequestError; adopt; return node | dom-api `AdoptNode` (NEW) in `crates/dom/elidex-dom-api/src/document.rs`; session `apply_adopt_node` (NEW); VM native (NEW) | ✓ | yes |
| WHATWG DOM §4.2.3 Mutation algorithms | *remove* step 16 (queue a tree mutation record, §4.3.2), run by adopt step 2 | the source record of `adoptNode` on a parented node | `source_removal_record` `crates/script/elidex-script-session/src/mutation/mod.rs:341` | ✓ | yes |
| WHATWG DOM §4.8 Interface ShadowRoot | *shadow-including tree order* | the walk's order over the shadow-including tree; contents, outside it, are reached by adopt 3.4 (HTML §4.12.3 adopting step 2) right after their template | `for_each_uncapped_shadow_inclusive` `crates/core/elidex-ecs/src/dom/tree/walkers.rs:99` (D-19) | ✓ | no |
| WHATWG HTML §4.13.6 Custom element reactions | *enqueue a custom element callback reaction*; `[CEReactions]` | `adoptedCallback` with « oldDocument, document »; drain point | consumer arm; flush `crates/script/elidex-js/src/vm/host/custom_elements/flush.rs:133` | ✗ (method-scope drain: L3's proposed slot) | yes |
| WHATWG HTML §4.12.3 The template element | *adopting steps* 1–2 | the contents' pair: `old` = the contents' own node document, `new` = step 1's document (C2: the template's new node document; C1: `f` of it) | segment recording (D-3 (c)) | ✗ (C1's `f`) | yes |

### §3.1 User-input touch audit

Two new script entry points: `adoptedCallback` (author code, through the existing flush and error routing) and
`document.adoptNode` (author argument, marshalled into the engine-independent path). `Adopt` carries entities only.

## §4. Mechanism

### §4.2 Reconcile — C0b's head → C0c

| site | C0b | C0c |
|---|---|---|
| `adopt` (`tree/mutation.rs`) | walk, no event | walk + skips + segments (D-3 (c)); `Adopt` iff the root's pair |
| `mutation_event.rs:107` | seven variants (eight at the base `062337aa`, RC-2a's `CloneCreated`; *Rev 3.7.5*) | + `Adopt`, `AdoptedContents` |
| `dom/mod.rs:32`, `lib.rs:44-47` | `MutationEvent` re-exported | + `AdoptedContents` |
| `walkers.rs:112-118` | contents after the subtree | contents right after the template |
| `consumer.rs:75-96` | no `Adopt` arm | `handle_adopt` |
| `range/live.rs:113-163`; `mutation_hook.rs:68-148` | exhaustive without `Adopt` | `live.rs`: `Adopt` joins the shared no-op arm; the recorder: an arm that records it (*Rev 3.7.5*, D-6) |
| `flush.rs:133-138` | no-op | `adoptedCallback(old, new)` |
| `vm/host/document.rs` (after D-14 (b)) | — | `adoptNode` native and its table entry |
| dom-api `document.rs`, `registry.rs` | — | `AdoptNode` |
| session `mutation/child_list.rs` | — | `apply_adopt_node` |

### §4.7 Layering: VM site → engine-independent API

| VM site (marshalling only) | engine-independent API | engine call |
|---|---|---|
| `adoptNode` native (NEW, `vm/host/document.rs`) | `invoke_dom_api("adoptNode")` (`dom_bridge.rs:462`) → dom-api `AdoptNode` (NEW) | session `apply_adopt_node` (NEW) → `EcsDom::adopt` |
| flush `Adopted` arm (`flush.rs:133-138`) | `invoke_callback` (`flush.rs:131`'s helper) with two Document wrappers | — |

No algorithm in `vm/host/`. The consumer arm lives in `elidex-custom-elements` (engine-independent), beside
`handle_insert`.

### §4.8 Docstring citation table

| symbol | citation |
|---|---|
| `MutationEvent::Adopt`, `AdoptedContents` (NEW) | DOM §4.5 *adopt* steps 3, 3.3.3, 3.4 (`#concept-node-adopt`); HTML §4.12.3 template *adopting steps* 1–2 (`#the-template-element`; step 1's `#appropriate-template-contents-owner-document`) |
| `handle_adopt` (NEW) | DOM §4.5 *adopt* 3.3.3; HTML §4.13.6 *enqueue a custom element callback reaction* (`#enqueue-a-custom-element-callback-reaction`); DOM §4.8 *shadow-including tree order* |
| flush `Adopted` arm (doc) | HTML §4.13.6 (`adoptedCallback` « oldDocument, document ») |
| `TemplateContentsWalk::Include` (doc) | DOM §4.8 *shadow-including tree order* (`#concept-shadow-including-tree-order`) for the tree; DOM §4.5 *adopt* 3.4 and HTML §4.12.3 template *adopting steps* 2 (`#the-template-element`) for the contents' position |
| `AdoptNode`, `apply_adopt_node`, the native (NEW) | DOM §4.5 `adoptNode(node)` steps 1–4 (`#dom-document-adoptnode`); the record: DOM §4.2.3 *remove* step 16 (`#concept-node-remove`); `[CEReactions]` HTML §4.13.6 (`#cereactions`) |
| `range/live.rs` `Adopt` arm | DOM §4.5 *adopt* step 2 (the removal already updated live ranges) |

### §4.9 Comment sites

The flush's "v1 single-document VM never enqueues Adopted — the slot is reserved by
`#11-custom-elements-adopted-callback`" (`flush.rs:134-137`, false after C0c); the consumer's "other variants are
no-ops" (`consumer.rs:73-74`); `walkers.rs:86-91` (D-19) and `:112-113` ("Pushed first, so popped last"); the
`CustomElementReaction::Adopted` docstring (`reaction.rs:21-25`) if it names the slot.

## §5. Populations, touched files, risks

- **P-6** (program §5), by property: (i) every `match` on `MutationEvent` without a wildcard arm, each gaining a no-op
  arm — the compiler lists them: add the variant, then `cargo check --workspace --all-targets --all-features`, one
  `E0004` per member (tests included); (ii) the consumers (program P-6's command, 11 files at `27b50dfe`), each
  judged for a non-no-op arm. Re-run at C0c's base; if RC-2a has landed, its `CloneCreated` arms are in the same
  matches (c0e).
- **P-c1 Include callers**: `git grep -n 'TemplateContentsWalk::Include' -- 'crates/*.rs'` → `teardown.rs:197`, the
  adopt walk, and (C0c) the consumer; each caller's order-sensitivity stated. *Rev 3.7.5* (at the base `062337aa`):
  the production callers are `teardown.rs:199`, `adopt_walk` (`tree/mutation.rs`) and
  `crates/dom/elidex-form/src/clone.rs:67` (missing above); the other hits are the walker's own branch, tests and doc
  comments. Teardown: insensitive (it destroys the collected set in reverse, and a fragment still follows its
  template). Adopt walk: insensitive for the stamps; its segments need only that a fragment follow its template.
  Form clone: insensitive (it collects the marked entities and copies each one independently). The new
  `handle_adopt` is the only order-sensitive caller.
- **P-c2 adoptNode reach**: `/usr/bin/grep -rn 'adoptNode\|adopt_node' crates --include='*.rs'` → 0 at `27b50dfe`
  (H-15); at C0c's head the native, the handler, the session function, tests. *Rev 3.7.5:* 1 hit at the base
  `062337aa`, not 0: C0b's doc comment `crates/core/elidex-ecs/src/dom/tree/mutation.rs:243`. At C0c's head (the same command, `| wc -l`, verified 2026-11-08), 43 hits in
  8 files, by class: native (`vm/host/document.rs`, 9: the fn, its doc and the table entry); handler + registration
  (dom-api `document.rs`, 9; `registry.rs:79`); session fn + re-exports (`mutation/child_list.rs`, 3;
  `mutation/mod.rs:15`; `elidex-script-session/src/lib.rs:55`); tests (`tests_insertion_adoption.rs`, 17;
  `registry.rs:547`, the registry test's list); doc comments (C0b's `tree/mutation.rs:249`).

| file | lines at `27b50dfe` | change | 1000-line decision |
|---|---|---|---|
| `crates/core/elidex-ecs/src/dom/mutation_event.rs` | 352 | `Adopt`, `AdoptedContents` (NEW) | < 1000 |
| `crates/core/elidex-ecs/src/dom/tree/mutation.rs` | 467 (≈ 560 at C0b) | segments, dispatch | < 1000 |
| `crates/core/elidex-ecs/src/dom/tree/walkers.rs` | 392 | D-19 (RC-2a also edits it, c0e) | < 1000 |
| `crates/core/elidex-ecs/src/dom/mod.rs` 926, `crates/core/elidex-ecs/src/lib.rs` 51 | as listed | re-export `AdoptedContents` (`:32`, `:44-47`; c0e) | < 1000 (one line each) |
| `crates/dom/elidex-custom-elements/src/consumer.rs` | 329 | `handle_adopt` | < 1000 |
| `crates/dom/elidex-dom-api/src/range/live.rs` | 1021 | no-op arm | split by L3 RC-0a (D-14) |
| `crates/core/elidex-ecs/src/dom/tests/mutation_hook.rs` | 1143 | no-op arm in the recorder | split by L3 RC-0a (D-14) |
| `crates/script/elidex-js/src/vm/host/custom_elements/flush.rs` | 237 | the arm | < 1000 |
| `crates/script/elidex-js/src/vm/host/document.rs` | 1197 | **split first (D-14 (b))**; then the native and its `DOCUMENT_METHODS_POST_LIVE_COLLECTION` entry (H-c6) in the remaining file (1197 − 144 − 516 = 537 lines, + ≈ 25) | prereq PR; < 1000 after |
| `crates/dom/elidex-dom-api/src/document.rs` 665, `registry.rs` 785 | as listed | `AdoptNode` | < 1000 |
| `crates/script/elidex-script-session/src/mutation/child_list.rs` (NEW: D-14 (a)) | `mutation/mod.rs:296-895` = 600 lines, + its `use` header | `apply_adopt_node` (≈ 25) | < 1000 (C0a / C0b edit it in place); re-measured at the base (DoD 1) |
| `crates/core/elidex-ecs/src/dom/tests/adopt_on_insert.rs` (NEW: C0b), `crates/script/elidex-js/src/vm/tests/tests_insertion_adoption.rs` (NEW: C0b) | — (NEW in C0b) | cells | measured at the base (DoD 1): if C0c's cells would take a file to ≥ 1000, they go to a sibling file for the notification (a scenario seam) |

**Risks.** R-5 `adoptedCallback` and `adoptNode` run / reach author code where HEAD ran none (spec). R-10 the
`MutationEvent` census with RC-2a (c0e).

## §6. Test plan

**Red-first** as the C0b memo §6, in the base formulations of §6.5. `dp`, `dp2` as there.

### §6.1 VM integration (`tests_insertion_adoption.rs`)

`‖` separates evals. *Rev 3.7.5:* the cells create their names as `globalThis.<name>` (the logic is unchanged): in
this VM a top-level `var` does not persist across separate evals (`tests_base_href.rs:463`), and a bare assignment
to an undeclared name throws `ReferenceError` (strict-only core, `Op::SetGlobal`). Reactions drain at the end of each eval (`interpreter.rs:54`, H-c3), so a cell reads `log` in an
eval after the one that enqueues. The cells share T6's definition of `x-a` (one `define` per test, HTML `define()` step
3 rejects a second): `log.push` entries `['a', o === document, n === dp]` (adopted), `['c', this.ownerDocument === dp]`
(connected), `['d', this.id]` (disconnected; read at the drain, so by id).

| cell | kind | script | base (C0b's head) | C0c = spec |
|---|---|---|---|---|
| T6 | red | `log = []; customElements.define('x-a', class extends HTMLElement { adoptedCallback(o, n) { log.push(['a', o === document, n === dp]) } connectedCallback() { log.push(['c', this.ownerDocument === dp]) } disconnectedCallback() { log.push(['d', this.id]) } }); e = document.createElement('x-a'); dp.body.append(e)` ‖ `log` | `[['c', true]]` (C0b adopts; no callback) | `[['a', true, true], ['c', true]]` |
| T7 | red | after T6: `log = []; document.body.append(e)` ‖ `log`; variant: `old = document.createElement('x-a'); old.id = 'o'; document.body.append(old); e2 = document.createElement('x-a'); e2.id = 'n'; dp.body.append(e2)` ‖ `log = []; document.body.replaceChild(e2, old)` ‖ `log` | `[['d', ''], ['c', false]]`; variant `[['d', 'n'], ['d', 'o'], ['c', false]]` | `[['d', ''], ['a', false, false], ['c', false]]`; variant `[['d', 'n'], ['a', false, false], ['d', 'o'], ['c', false]]` (`Adopted(e2)` before `Disconnected(old)`) |
| T14 | guard (K-0) | same-document move: `q = document.createElement('x-a'); document.body.append(q)` ‖ `log = []; document.body.prepend(q)` ‖ `log` | `[['d', ''], ['c', false]]` | the same (no `Adopted`) |
| T15 | red | (i) `n = dp.getElementById('d'); r = document.adoptNode(n); [r === n, n.ownerDocument === document, n.parentNode]`; (ii) `document.adoptNode(dp)`; `h = document.createElement('div'); h.attachShadow({mode: 'open'}); document.adoptNode(h.shadowRoot)`; (iii) `c = document.createElement('x-a'); c.id = 'c'; dp.body.append(c)` ‖ `log = []; document.adoptNode(c)` ‖ `log` (`c` is created in a registry-bearing document); (iv) `x = document.createElement('div'); document.body.append(x); r = dp.adoptNode(x); [r === x, x.ownerDocument === dp, x.parentNode]` (a non-bound Document's wrapper, `dom_parser.rs:305`) | `TypeError` (H-15) | (i) `[true, true, null]`; (ii) `NotSupportedError`, `HierarchyRequestError`; (iii) `[['d', 'c'], ['a', false, false]]`; (iv) `[true, true, null]` |
| T15r | known defect (recorded regression, §1, §8; not the spec column) | `k = dp.createElement('x-a'); k.id = 'k'; dp.body.append(k)` ‖ `log = []; document.body.append(k)` ‖ `log` | `[['d', 'k'], ['c', false]]` (`k` wrongly upgraded in `dp`: the `createElement` native upgrades whatever the document, `vm/host/document_factories.rs:89`) | C0c: `[['d', 'k'], ['a', false, false], ['c', false]]`. Spec (derived from the spec text; DOMParser's null-registry behaviour is not measured in a browser): `k` is not custom in `dp`, so `[['c', false]]` after its upgrade at insertion; `#11-document-null-registry-adopt-reset` changes this cell to that |
| T16 | red | `MutationObserver` on `d`'s old parent in `dp`, `{childList: true}`: `document.adoptNode(d); mo.takeRecords()` | `TypeError` | one record: `removedNodes [d]`, target the old parent, siblings pre-removal |
| T18 | red (per-node pairs; P-4) | `t = dp.createElement('template'); dp.body.append(t); e = document.createElement('x-a'); t.content.append(e); document.adoptNode(t.content)` ‖ `log = []; dp2.body.append(t)` ‖ `log` | `TypeError` | `[['a', true, false]]`: `e`'s pair is its segment's (`old = document`, the contents' node document; `new = dp2` under C2), not the root's (`dp`, `dp2`), which would give `['a', false, false]`; C1 turns `new` into `dp2`'s inert document |

### §6.2 Unit

| cell | kind | shape | expected |
|---|---|---|---|
| U4 | red (`adopt_on_insert.rs`) | C0b's U4 recorder | `Remove(old parent)`, `Adopt`, `Insert`; replace: `Remove(new_child's old parent)`, `Adopt`, `Remove(old_child)`, `Insert` |
| U5 | guard | same-document move | no `Adopt` |
| U2′ | guard | C0b's U2 (ownerless root under `D`) | no `Adopt` (silent re-home) |
| U10 | red | `EcsDom::adopt(fragment, D)` on a parentless fragment with children | one `Adopt`, `documents = Some((X, D))` |
| U13 | red (order; `elidex-custom-elements`) | a template with a custom light child `L` and a custom element `C` in its contents, adopted | reactions `C` then `L`; `C` with its segment's pair |
| U18 | red (D-19; `elidex-ecs`) | `for_each_uncapped_shadow_inclusive(root, Include, …)` over a template with contents and light children, and a shadow host | template, its contents subtree, then its light children; host, its shadow tree, its light children |
| U19 | guard | nested contents: a template inside contents of a template, a custom element in the inner contents; first leg: the outer template in `D0`, the outer contents in `Y`, the inner contents in `X` (three distinct documents); second leg: the outer contents adopted into the target, then the inner contents (with the element and its materialized `a = el.getAttributeNode('id')`) into `X`; third leg (fixture): the outer contents ownerless (silent re-home), the inner contents in `X` ≠ the target | first leg: the inner element gets `Adopted(X, target)` (the innermost segment's pair); second leg: no segment for either contents, the inner contents, the element and `a` still read `X`, no reaction (D-3 (c) skip); third leg: both segments recorded with `documents = None`, both contents and the element stamped with the target (not skipped), no reaction (its own pair would give `Adopted(X, target)`) |

### §6.3 Mutation guards

| mutation | killed by |
|---|---|
| CE arm missing | T6, U13 |
| `Adopted` after `Connected` / `Adopt` after `Insert` | T6, T7, U4 |
| root pair used for contents | T18, U13, U19 |
| the walk stamps or records inside a fragment whose `old` equals `new` | U19 (second leg) |
| the skip applied to the stamps but not to the Attr re-home | U19 (second leg) |
| a segment under a `None` segment keeps its own pair | U19 (third leg) |
| contents visited after the template's subtree | U13, U18 |
| `Adopt` on a same-document move / for an ownerless root | T14, U5, U2′ |
| `adoptNode` without the removal record | T16 |
| `adoptNode` accepts a Document or ShadowRoot | T15 |

### §6.4 Kept green

C0b's §6 cells; `cargo test --all-features -p elidex-ecs -p elidex-custom-elements -p elidex-dom-api -p
elidex-script-session -p elidex-js`; `mise run ci`.

### §6.5 Base formulations (red-first; Ax3 IMP-1)

A red cell is run on the base (C0b's head) through the base's API; a compile failure is not counted as red. Where the
cell's subject is new at C0c, it is red by absence, and the row says so.

| cell | NEW API in the cell | base formulation → expected base result | control (green on the base) |
|---|---|---|---|
| T6, T7 | none | as written → the *base* column (no `'a'` entry) | `e.ownerDocument === dp` after the append (C0b's adoption ran) |
| T15, T16 | `document.adoptNode` (the subject) | red by absence (new subject): `typeof document.adoptNode` → `'undefined'` | `typeof document.createElement` → `'function'` |
| T18 | `document.adoptNode` (set-up only) | red by absence: a segment whose pair differs from the root's is reachable only through `adoptNode` (otherwise under C2 the contents' node document is the template's); observed on the base: `typeof document.adoptNode` → `'undefined'`, and `log = []; t.content.append(e)` ‖ `log` → `[]` (no `Adopted` at all) | `e.ownerDocument === dp` after `t.content.append(e)` |
| U4 | `MutationEvent::Adopt` | compile failure, not counted; HEAD proxy: C0b's recorder → `Remove(old parent)`, `Insert`, nothing between | the moved node's `node_document` is the target at `Insert` |
| U10 | `MutationEvent::Adopt` | compile failure, not counted; HEAD proxy: the recorder sees no event from `EcsDom::adopt(fragment, D)` | the children read `D` afterwards |
| U13 | none (`EcsDom::adopt` is C0b's) | as written, with the CE consumer installed → no `Adopted` in the queue | `C` and `L` read `CEState::Custom` |
| U18 | none | as written → template, its light children, then its contents subtree (`walkers.rs:112-118`) | the host's shadow tree right after the host (not changed by D-19) |

Guards (T14, U2′, U5, U19) and the known-defect cell T15r are not run on the base.

## §7. Out of scope

Program §7: `importNode`, the scoped registries, the `img` adopting steps (this PR provides its hook: the `Adopt` event),
the method-scope reaction drain.

| item | where | defect |
|---|---|---|
| adopt step 3.3.2, non-scoped half | L3's proposed `#11-document-null-registry-adopt-reset` (L3 Rev 6.1 H8, PM 249); it hooks `MutationEvent::Adopt` (D-6), so its consumer arm lands after this PR | the slot's (L3 #583 §9): elidex has no null-registry document and no adopt-time registry reset; for C0c's part see §1's regression |

## §8. Edges and records

| # | edge | kind | note |
|---|---|---|---|
| c0-pre-b | D-14 (b) → C0c | code predecessor | pure move |
| c0-bc | C0b → C0c | code + semantic | `adopt`, the walk, the stamps the pairs read |
| c0b | C0c → C1 | semantic + code | C1's per-segment pair values (D-C5, T25) need the segments and the consumer; C1's edge rows are C1's |
| c0e | C0c ↔ L3 RC-2a | census collision + file overlap | `CloneCreated` and `Adopt` share the enum, `handle`, the exhaustive matches; whichever lands second re-runs P-6 and adds its arm to every exhaustive match. Files both edit: `walkers.rs` (D-19, `:86-118`; RC-2a's `for_each_clone_created`, #583 §7) and the re-exports `dom/mod.rs:32`, `lib.rs:44-47` (§5) |
| c0h | #583 → C0c | ordering (PM 311); #583 landed (`12ea5dce`) | C0c lands after #583: the slots it points to are registered at #583's landing, and DoD 8 checks #583's text |

**Recorded regression (2026-11-03, PM 311).** A custom element that elidex wrongly upgrades inside a null-registry
document (e.g. DOMParser) gets an extra `adoptedCallback(dp, document)` when adopted; HEAD fires none (T15r). Bounded:
`#11-document-null-registry-adopt-reset` (L3, proposed in #583) hooks `MutationEvent::Adopt`, has the trigger "C0c
lands", and is L3's next item after C0c. C0c adds no guard that skips `Adopted` for such elements (it would duplicate
that slot's fix).

**Stated defect (PM 319; restated *Rev 3.7.5*).** D-6's lookup is `find_tree_root_composed`, which stops after
`MAX_ANCESTOR_DEPTH` steps (10 000; `walkers.rs:425-441`, `dom/mod.rs:48`, at C0c's head) and then returns the interior
node it reached. That node matches no segment and reads `None`, so below that depth the element's `adoptedCallback` is
lost (consumer), and a node inside a skipped fragment is stamped by mistake (walk: it reads as a live `None` segment). Same root as `#11-live-range-bridge-wide-sibling-index` (engine-wide
`MAX_ANCESTOR_DEPTH` caps).

**Records carried by C0c's PR.** The umbrella row's "C0c: planned." → "C0c: landed (#<PR>)." with a changelog line
(program R-3). **For the PM:**
- program R-1 (close `#11-cross-document-adopt-on-insert`), R-2 (close `#11-custom-elements-adopted-callback`), and
  register R-6 (b) `#11-import-node` and (f) `#11-img-adopting-steps-relevant-mutations` at this landing (program §9
  item 4);
- the PM adds the recorded regression above to `#11-document-null-registry-adopt-reset`'s defect text at registration
  (PM 311), and D-13's pending-upgrade line to `#11-ce-reactions-method-scope-drain`'s (PM 306).

## §9. Definition of done

1. D-14 (b) landed; coordinates re-measured at the base, RC-2a's files included; P-6, P-c1, P-c2 re-run and recorded.
2. D-3 (c), D-6, D-13, D-19 implemented with §4.8's citations.
3. §6 cells present; every red cell red on the base (C0b's head) with its control green, recorded in the PR.
4. P-c2 at the head lists exactly the native, the handler, the session function and tests (with their re-exports, the
   registration and doc comments, by class as §5 records it; *Rev 3.7.5*).
5. §6.4 green; `mise run ci` green.
6. `python3 .claude/skills/elidex-plan-review/preflight.py docs/plans/2026-11-eh-attrs-c0c-adopt-notification-adoptnode.md`
   and on the program memo exit 0 (run).
7. The umbrella row's C0c clause updated; the PM has applied R-1 and R-2 and registered `#11-import-node` and
   `#11-img-adopting-steps-relevant-mutations` (R-6 (b), (f)).
8. Program §9 item 6's points, C0c's included, compared with #583 as on main (C0c lands after it, edge c0h; PM 273).
   C0c's PR corrects #583's four statements that C0c does not edit `dom/mod.rs` to the facts at that point (C0c
   re-exports `AdoptedContents` there, §5). As of main `12ea5dce` (line numbers a hint; re-located by content at
   implementation): the Rev 6.4 changelog entry ("`dom/mod.rs` is RC-2a's and C0b's, not C0c's", `:116`); §7's
   `dom/mod.rs` row ("C0c does not edit it", `:1888`); §9 ("is edited by RC-2a and C0b, not by C0c", `:2004-2005`);
   H6 ("RC-2a's and C0b's, not C0c's", `:2277-2278`).

## §10. Review dispositions (plan-review at `76af676a`; 0 CRIT / 8 IMP / 22 MIN; PM 306 / 311)

| finding | disposition | where |
|---|---|---|
| Ax2 IMP-1 (pending upgrade) | lost, not delayed; recorded on the drain slot (PM 306); no workaround | D-13, §8 |
| Ax2 IMP-2, Ax4 MIN-5 (drain timing; redefinition) | evals split at `‖`; one class with `disconnectedCallback`; logs in T6's shape | §6.1 |
| Ax3 IMP-1 (red-first) | base-formulation table | §6.5 |
| Ax5 IMP-1 (`Document.prototype`) | own property via `DOCUMENT_METHODS_POST_LIVE_COLLECTION`, native in `document.rs` | H-c6, D-13, §4, §5 |
| Ax5 IMP-2 (RC-2a overlap) | `walkers.rs`, re-exports added; #583 `:1888` line (C0c's PR from rev 3.7) | §0.1, §5, c0e, §9 |
| Ax5 IMP-3 (cross-check) | program §9 item 6 extended; DoD 8 | program §9; §9 |
| Ax5 IMP-4 (order vs #583) | edge c0h (PM 311) | Placement, c0h; program §1, §8 |
| Ax5 IMP-5 (null-registry) | recorded regression, bounded by L3's slot; T15 creates in `document`; T15r (PM 311) | §1, §6.1, §8 |
| Ax1 MIN (error mapping) | the native brand-checks, unwraps, calls | D-13, H-c4 |
| Ax2 MIN (skipped segment) | nested segment under `None` is `None`; Adopt iff the root's pair | D-3 (c), D-6, I-A4, U19 |
| Ax2 MIN (U9′ vacuous) | deleted; C0b's walk counter pins A4×A5 | §2, §6.2 |
| Ax2 MIN (unbound flush) | stated, same slot | D-13 |
| Ax3 MIN (D-6 exit) | nearest-fragment lookup, no walker API | D-6 |
| Ax3 MIN (drain slot unregistered) | registered at #583's landing (PM 306), edge c0h | c0h |
| Ax3 MIN (R-6 (b), (f) in DoD) | DoD 7 (PM 311) | §8, §9 |
| Ax3 MIN (§1 reason) | A1×A9 is `adoptNode`'s own | §1; program §1 |
| Ax3 MIN (receiver) | `document_receiver`, as siblings (corrected PM 319) | D-13, H-c6, T15 |
| Ax3 MIN (three files) | decided | §5 |
| Ax3 MIN (iframe side) | R-2 names `#11-windowproxy-browsing-context` | program R-2 |
| Ax4 MIN-1–4 | *remove* step 16; contents via adopt 3.4 + adopting steps; pair wording; anchors | D-13, D-19, §3, §4.8 |
| Ax5 MIN (header rev) | rev 3.6; 3.3–3.5 do not touch C0c | header |
| Ax5 MIN (slice a §4.3) | cited | D-19 |
| Ax5 MIN (H-C1) | (vi) D-19 | program H-C1 |
| Ax5 MIN (census population) | by property, compiler command | §5 P-6 |
| Ax5 MIN (R-6 timing) | C0c, as program §9 item 4 (PM 311) | §8, §9 |
| Ax5 MIN (native in factories) | moved to `document.rs` | D-13 |
