# C0 `EH-attrs-at-creation-C0` — every insertion adopts the node into the parent's node document (per-PR plan-memo)

Nested PR **C0** of the standalone program `EH-attrs-at-creation` (umbrella
`docs/plans/2026-10-event-handler-attrs-at-creation.md`, approved by #551 `b9122157` at rev 8, carried to rev 8.3 by
#570; "the umbrella"). It closes slot `#11-cross-document-adopt-on-insert` (carved by S5-4a #444) and, with it, slot
`#11-custom-elements-adopted-callback` (D-17 #227), whose trigger is "cross-document adoption infra landing" (§0.2
H-12). Siblings cited by ID: IBP-sandbox N1 (`docs/plans/2026-10-ibp-sandbox-handler-gates.md`, rev 4.5, merged as a
plan by #570, implementation in flight; "the N1 memo"), its node memo `docs/plans/2026-09-ibp-sandbox.md` ("the node
memo"), C1 (`docs/plans/2026-10-eh-attrs-c1-template-owner-document.md`, rev 1 draft on branch
`layout-eh-c1-template-owner-plan`, `bc52d1b9`, not on main; "the C1 draft"), T3
(`docs/plans/2026-10-eh-attrs-t3-target-determination.md`, rev 2 draft on branch `layout-eh-attrs-t3-plan`,
`94ebfce5`, not on main; "the T3 draft"). Draft IDs are cited as draft IDs.

**Placement (evidence).** PM decision 143 (2026-10-31; ledger `project_open-defer-slots.md`, slot
`#11-cross-document-adopt-on-insert`, sub-bullet "Owner assigned 2026-10-31"): "Layout lane, as C0 of the EH-attrs
program. C0 is a standalone PR before C1 … Edge-dense, so plan-review is mandatory. L3's P1 umbrella treats C0 as
landing first (the insertion-steps boundary goes in C0's plan §2). Requirements: C1 plan memo §4.6 C0-1..C0-4". PM
decision 153 (2026-10-31, user decision, relayed by the driver): **the program order is N1 implementation → C0 → C1 →
T3 → slice b** (T3 moved after C1). So at C0's base N1's implementation has landed and T3 has not: HEAD's
`resolve_handler_target` (`crates/script/elidex-script-session/src/event_handler_processing/mod.rs:249`) is the target
rule, and T3's `determine_event_handler_target` / `effective_node_document` do not exist (§8 hand-off H-T).

**Scope.** One fact — the node document, stored as `AssociatedDocument` — kept true by one mechanism, DOM *adopt*,
run at the one tree-linking chokepoint:
1. `EcsDom::node_document` (NEW): DOM §4.4 *node document* (a Document is its own); `owner_document` becomes the
   `ownerDocument` getter's "null for a Document" wrapper over it, and its two tree-position fallbacks are deleted (D-1).
2. `EcsDom::adopt` (NEW): DOM §4.5 *adopt* steps 1–3 (3.1 set node document, 3.3.3 `adoptedCallback`, 3.4 adopting
   steps); `adopt_subtree` stays the step-3 walk (D-2, D-3).
3. Insertion adoption at the three linking primitives (`append_child`, `insert_before`, `replace_child`), placed at
   DOM *insert* step 7.1 / *replace* step 6, and at the session's replace-with-fragment arm (D-4, D-5).
4. `MutationEvent::Adopt` (NEW), the adopting-steps notification, consumed by the custom-element consumer
   (`adoptedCallback`) (D-6).
5. Every node-document derivation reads `node_document`: N1's effective-node-document proxy in clause (b) is deleted,
   and the "Document → itself" idioms are collapsed (D-7).

**Not in scope.** Template contents' owner document (C1: under C2 the template adopting steps are the identity, D-3);
`document.adoptNode` / `importNode` (absent at HEAD; O-2); ownerless node creation (§7); the Selection API's
document-membership check (§7); custom-element registry steps 3.2 / 3.3.1–3.3.2 (§7); `moveBefore` (not implemented).

**Changelog.** *Rev 1* (2026-10-31): first draft, pre-plan-review; folds PM decision 153 (order N1 → C0 → C1 → T3 →
slice b; R-B of the T3 draft closed by C0, §9 item 9).

## §0. Frame, premises, decisions

### §0.1 Coordinate frame

Every `file:line` is a **`1ed126b4`** coordinate (`origin/main`, the branch cut; `git rev-parse HEAD` in this
worktree before this memo's commit). C0's implementation is cut after N1's implementation lands, so every coordinate
in `crates/script/elidex-script-session/src/scripting.rs` and the handler path is **re-measured at that base** (§9
item 1); N1 narrows clause (b)'s arguments to `Entity` (N1 memo D-N2) but keeps the proxy's body. Code references come
from `/usr/bin/grep -n` / `sed -n` over the `1ed126b4` tree, run 2026-10-31. Every § number ↔ title pair and every
algorithm step comes from `.claude/tools/webref` (`heading`, `dfn`, `body`), run 2026-10-31; the commands are in §3.

### §0.2 Premises re-measured

| id | claim | evidence (`1ed126b4`) |
|---|---|---|
| H-1 | No insertion adopts | `/usr/bin/grep -rnF 'adopt_subtree(' crates --include='*.rs'` → definition `crates/core/elidex-ecs/src/dom/tree/teardown.rs:251`, prod caller `finish_detached_fragment` `:337`, test `crates/core/elidex-ecs/src/dom/tests/destroy.rs:324`. `append_child` (`crates/core/elidex-ecs/src/dom/tree/mutation.rs:22`) runs `detach_with_hook` (`:40`) → `link_node` (`:43`) → `fire_after_insert` (`:47`) and never writes `AssociatedDocument` |
| H-2 | Tree linking has one chokepoint | `/usr/bin/grep -rn 'link_node(' crates --include='*.rs'` → definition `mutation.rs:210` (`pub(super)`) and three calls, `:43` (`append_child`), `:119` (`insert_before`), `:197` (`replace_child`); `/usr/bin/grep -rn 'rel.parent = ' crates --include='*.rs'` → `mutation.rs:218` (link) and `crates/core/elidex-ecs/src/dom/tree/mod.rs:85` (clear); `TreeRelation`'s fields are `pub(crate)` (`crates/core/elidex-ecs/src/components.rs:150-158`). Every insertion path in every crate (parsers, `attach_shadow`'s host link, the session's `apply_*`, `Range`) reaches one of the three primitives |
| H-3 | `owner_document` is stamp-first with two tree-position fallbacks | `crates/core/elidex-ecs/src/dom/mod.rs:821-860`: explicit `AssociatedDocument` if it is a live Document (`:828-837`); else the tree root if it is a Document (`:839-841`); else a `DocumentFragment` root's stamp (`:852-856`); else `None`. A Document → `None` (`:825-827`) |
| H-4 | Which nodes carry a stamp at HEAD | stamped: `create_*_with_owner` callers (`createElement` and kin), clones (`crates/core/elidex-ecs/src/dom/tree_clone.rs:221`, `:417`), every node a fragment parse returns (`finish_detached_fragment` adopts into the context's document, `teardown.rs:336-337`) — so every `innerHTML` node and every DOMParser node (DOMParser parses through `apply_set_inner_html` into a stamped `<html>`, `crates/dom/elidex-form/src/inert_document.rs:119-121`). Unstamped: nodes the main-document parse creates (strict `create_element`, tolerant `convert_node` with `None`; C1 draft H-C3), shadow roots (`crates/core/elidex-ecs/src/dom/shadow.rs` writes no `AssociatedDocument`), and engine-internal ownerless creations (§5.1 P-8) |
| H-5 | N1's proxy (clause (b)) | `scripting_disabled_for_platform_object` `crates/script/elidex-script-session/src/scripting.rs:83`; Document arm `entity != document` (`:96`); node arm: composed tree root if a Document, else `owner_document` (`:104-110`); docstring records the slot and the facets (`:35-72`), and that "when it lands … this rule collapses to a plain `owner_document(entity) != bound_document` compare" (`:51-53`) |
| H-6 | "Document → itself" node-document idioms | `owner_doc` `crates/dom/elidex-dom-api/src/element/document_base.rs:173-178`; `owner_document(entity).unwrap_or(entity)` `crates/dom/elidex-dom-api/src/element/href_accessor.rs:50`, `crates/dom/elidex-dom-api/src/char_data/document_props.rs:75`; `same_document` `crates/dom/elidex-dom-api/src/selection.rs:614-619` |
| H-7 | The session record layer already models adopt step 2 | `apply_append_child` / `apply_insert_before` / `apply_replace_child` build the "§4.5 adopt source-parent removal" record (`crates/script/elidex-script-session/src/mutation/mod.rs:304-386`, `:475-521`); `expand_fragment`'s comment says "step 7: per-node adopt + insert" (`:441`), which HEAD does not do (P-2) |
| H-8 | Replace-with-fragment bypasses `EcsDom::replace_child` | `apply_replace_child`'s fragment arm (`mutation/mod.rs:689-743`): `remove_child(parent, old_child)` (`:713`), then `expand_fragment` with `insert_before` / `append_child` per child (`:719-722`). The fragment entity itself is never re-homed |
| H-9 | `MutationEvent` has no adoption variant | `crates/core/elidex-ecs/src/dom/mutation_event.rs:107` (variants `Insert` `:131`, `Remove` `:172`, `TextChange`, `ReplaceData`, `SplitText`, `NormalizeMerge`, `AttributeChange`); the VM fan-out calls seven consumers in field order (`crates/script/elidex-js/src/vm/consumer_dispatcher.rs:148-158`) |
| H-10 | The fire order inside a primitive | `append_child`: `is_connected` capture (`:39`), `detach_with_hook` (`:40`; fires `Remove` for the old parent, `:442-466`), `link_node` (`:43`), `rev_version` (`:44`), `fire_after_insert` (`:47`; `Insert`, suppressed when `node` or `parent` is a shadow root, `:282-299`). `replace_child`: `detach_with_hook(new_child)` (`:164`), `detach(old_child)` (`:186`), `fire_after_remove(old_child)` (`:189`), `link_node` (`:197`), `fire_after_insert` (`:201`) |
| H-11 | Fragment builds run with the dispatcher taken | `begin_detached_fragment` takes it (`teardown.rs:293-304`); the caller restores it after `finish_detached_fragment` (`:330-343`); `parse_into_inert_document` likewise (`inert_document.rs:112`) |
| H-12 | `adoptedCallback` has a reserved slot and no fire path | `CustomElementReaction::Adopted { entity, old_document, new_document }` `crates/dom/elidex-custom-elements/src/reaction.rs:21-25`; the VM flush no-ops it ("reserved by `#11-custom-elements-adopted-callback`", `crates/script/elidex-js/src/vm/host/custom_elements/flush.rs:133-138`); the consumer matches `Insert` / `Remove` / `AttributeChange` only (`crates/dom/elidex-custom-elements/src/consumer.rs:79-98`). Ledger: `m4-12-platform-gap-roadmap.md:1498` (trigger "Multi-document / iframe landing fires Adopted reaction path naturally") |
| H-13 | `is_connected` counts any Document root | `crates/core/elidex-ecs/src/dom/tree/walkers.rs:143-158`: true iff an ancestor is a `NodeKind::Document`; a node in a DOMParser document's tree is connected (spec: DOM §4.2.2 *connected*, shadow-including root is a document) |
| H-14 | HEAD's target rule ignores the node document | `resolve_handler_target` `event_handler_processing/mod.rs:249-255`: Window iff `HandlerScope::Window` and the element's tag is `body` |
| H-15 | `document.adoptNode` / `importNode` do not exist | `/usr/bin/grep -rn 'adoptNode\|importNode\|adopt_node' crates --include='*.rs'` → no hit |

### §0.3 Premise errors (reported)

- **P-1 (brief).** "N1's effective-node-document proxy in scripting.rs:104-109": at `1ed126b4` N1's implementation has
  not landed, so the proxy is HEAD's clause (b) (`:104-110`, H-5). N1 keeps its body and narrows its arguments; C0's
  coordinates are re-measured at N1's landed base (§0.1).
- **P-2 (code comment).** `expand_fragment`'s "step 7: per-node adopt + insert" (`mutation/mod.rs:441`) and
  `fire_after_remove`'s "DOM `insert` adopts the node" (`mutation.rs:330-339`) describe an adoption HEAD does not
  perform (H-1). C0 makes them true; neither needs a rewrite beyond the citation check of §4.7.
- **P-3 (replace order).** `apply_replace_child`'s non-fragment arm cites the replace algorithm as "spec removes
  old_child at step 11, before new_child's adopt at step 13" and steps over `old_child` when building the source-removal
  record's siblings (`mutation/mod.rs:768-785`). Current *replace* (`webref body dom concept-node-replace`,
  2026-10-31): step 6 "Adopt node into parent's node document" **before** step 7 "If child's parent is non-null … remove
  child". Under the current order, adopt's removal of `node` (adopt step 2) sees `child` still in place, so for `P =
  [A, B]; P.replaceChild(B, A)` the source-removal record has `previousSibling = A`; elidex reports `null`. The
  primitive's order (`detach_with_hook(new_child)` before `detach(old_child)`, H-10) already matches the current text;
  only the record layer's sibling capture and its step numbers drift. Not C0's defect (it is MutationObserver record
  content, not the node document); §7 row and O-4.
- **P-4 (`owner_document`'s docstring).** "falls back to the tree-root walk so that legacy entities created without the
  component still resolve" (`mod.rs:815-819`): the unstamped population is not legacy — it is every node of every
  main-document parse and every shadow root (H-4). C0 deletes the fallback by making the stamp universal for every
  node whose tree has a document (D-1, I-U).
- **P-5 (N1 memo NR3).** NR3's premise ("any DOMParser document's node … keeps the DOMParser document as its
  `owner_document`") holds because DOMParser nodes are stamped by the fragment parse's finishing adopt (H-4). The
  *reverse* facets are creation-path dependent: an unstamped main-document node moved into a DOMParser tree resolves
  to the DOMParser document by H-3's tree-root fallback while connected and to the bound document again once removed
  (cell T3). The ledger's facet list is complete in kind but not in this distinction; §6 covers both paths.
- **P-6 (VM `isConnected`).** `native_node_get_is_connected` compares the composed root with the **bound** document
  (`crates/script/elidex-js/src/vm/host/node_proto.rs:440-461`), so a node in a DOMParser document's tree reports
  `false`; `EcsDom::is_connected` (H-13) and the spec say `true`. Not a node-document reader and not C0's; reported for
  the PM (§7, O-5).
- **P-7 (slot text).** The ledger lists `selection.rs` among the `owner_document`-compare consumers. Its check is the
  Selection API's document-membership test, which is a tree relation, not a node-document read (§7, Q-S6); after C0
  its outcome becomes the spec's for connected nodes and stays HEAD's for detached ones.

### §0.4 Decisions (this PR)

- **D-1 — the node document is the stored fact, read by one function.** `EcsDom::node_document(&self, node: Entity)
  -> Option<Entity>` (NEW), in `crates/core/elidex-ecs/src/dom/mod.rs` beside `owner_document`: a `NodeKind::Document`
  → `Some(node)` (DOM §4.4: "The node document of a document is that document itself"); any other node → its
  `AssociatedDocument` if that is a live Document, else `None`. `owner_document` keeps its name and contract for the
  `ownerDocument` getter ("return null, if this is a document; otherwise this's node document"): `None` for a Document,
  else `node_document`. **Both tree-position fallbacks of H-3 are deleted** (`mod.rs:839-856`): after D-4 every node
  linked under a parent whose node document is known carries the stamp (invariant I-U, §2.1), so a tree-root walk can
  only answer for a node whose stamp is missing or stale, and for those it answers by position, which is the proxy C0
  removes. *Side-store rule* (CLAUDE.md): already a component (`AssociatedDocument`, `components.rs:595`), `Send +
  Sync`, holds an `Entity`; no side-store is added. *Idiom table:* OO engines' `Node::m_document` pointer updated by
  `adoptNode` / `insertBefore` → the `AssociatedDocument` component written by the one adopt function; the
  document's own `ownerDocument() == nullptr` special case → `owner_document` over `node_document`.
- **D-2 — `EcsDom::adopt(&mut self, node: Entity, document: Entity)` (NEW) is DOM §4.5 *adopt*.** Step 1 oldDocument
  = `node_document(node)`; step 2 if `node` has a parent, remove it through the same removal path the primitives use
  (`detach_with_hook`, which fires `MutationEvent::Remove`); step 3 if `document` ≠ oldDocument, re-home the
  shadow-including inclusive descendants (`adopt_subtree`, D-3) and dispatch `MutationEvent::Adopt` (D-6). A `None`
  oldDocument (an unstamped node, §2.1 A8) re-homes **silently** — no `Adopt` event — because it is the deferred
  creation-time stamp of an engine-created node, not a move between documents (no spec state corresponds to a node
  without a node document). Home: `crates/core/elidex-ecs/src/dom/tree/mutation.rs` (the removal path is
  `pub(super)` there). Its callers: the three primitives (D-4) and the session's replace-with-fragment arm (D-5); and,
  if O-2 (a), `document.adoptNode`.
- **D-3 — `adopt_subtree` stays the step-3 walk.** `adopt_subtree(root, document)` (`teardown.rs:251`) keeps its
  walk — `for_each_uncapped_shadow_inclusive(root, TemplateContentsWalk::Include, …)` (`crates/core/elidex-ecs/src/dom/tree/walkers.rs:99`),
  uncapped, iterative — and sets every visited node's `AssociatedDocument` (step 3.1). Under C2 the template adopting
  steps (HTML §4.12.3: "Let document be node's node document's appropriate template contents owner document"; adopt
  contents into it) are the identity, because C2's appropriate owner is the template's own document; so re-homing
  contents to `document` is exactly today's walk. C1 replaces that one edge with `f(document)` (C1 draft D-C5). Steps
  3.2 and 3.3.1–3.3.2 (custom-element registries, attribute nodes) are §7 rows; step 3.3.3 is D-6. It moves from
  `teardown.rs` to `mutation.rs` beside `adopt` (one home for the adopt algorithm); `finish_detached_fragment` keeps
  calling it (D-8).
- **D-4 — the primitives adopt at the spec's position.** Each of `append_child`, `insert_before`, `replace_child`
  computes `target = node_document(parent)` and, in place of its bare `detach_with_hook(child, parent)`, runs D-2's
  steps: the removal (as today, firing `Remove` for the old parent), then — if `target` is `Some` and differs from the
  child's node document — the re-home and the `Adopt` event, then `link_node` and `fire_after_insert` as today. If
  `target` is `None` (an ownerless parent, §2.1 A8) the child keeps its stamp. For `replace_child` the adopt of
  `new_child` precedes `old_child`'s removal (*replace* step 6 before step 7), which is the primitive's existing order
  (H-10). §2.0 states the position against the spec's step list.
- **D-5 — replace with a DocumentFragment adopts the fragment.** *Replace* step 6 adopts `node` itself, the fragment
  included (its shadow-including inclusive descendants: the fragment and its children); *insert* step 7.1 then adopts
  each child (a no-op). The session's fragment arm (H-8) calls `EcsDom::adopt(fragment, node_document(parent))` before
  `remove_child(parent, old_child)`. Pre-insert of a fragment (`append` / `insertBefore`) adopts only the children
  (insert step 7.1; the fragment keeps its node document), which the per-child primitives already give.
- **D-6 — `MutationEvent::Adopt { node, old_document, new_document }` (NEW) is the adopting-steps notification.** One
  event per adopted root, dispatched by `adopt` after the re-home and before the root is linked (D-4), only when
  oldDocument is `Some` and differs. Consumers: the custom-element consumer adds an `Adopt` arm that walks the root's
  shadow-including inclusive descendants in shadow-including tree order and enqueues
  `CustomElementReaction::Adopted { entity, old_document, new_document }` for each element whose state is
  `CEState::Custom` (adopt step 3.3.3); the VM flush invokes `adoptedCallback` with the two Document wrappers (`[old,
  new]`, HTML §4.13.6 *enqueue a custom element callback reaction*, args « oldDocument, document »). Batch equivalence:
  the spec interleaves "set node document" and "enqueue" per descendant, but enqueuing runs no script (reactions run at
  the next `[CEReactions]` boundary), so re-homing the whole subtree first and enqueuing after is unobservable. Other
  consumers ignore the variant (`_ => {}`; §5.1 P-6). *Idiom table:* OO engines' `Node::didMoveToNewDocument` virtual
  override per node class → one event on the mutation stream + a consumer arm per system that cares; the state itself
  stays the component (D-1).
- **D-7 — every node-document derivation reads `node_document`.** (a) N1's proxy (clause (b), H-5) is deleted: the
  Document arm and the node arm become one arm, `dom.node_document(entity).is_some_and(|d| d != document)`, and the
  docstring's slot paragraph (`scripting.rs:35-72`) is rewritten (the facets no longer exist); the fail-open on `None`
  is unchanged (it is N1's / the node memo's direction rule, D8). (b) `document_base.rs`'s `owner_doc` is deleted and
  its three callers read `node_document`. (c) `owner_document(x).unwrap_or(x)` at `href_accessor.rs:50` and
  `document_props.rs:75` becomes `node_document(x).unwrap_or(x)` (identical values; one function for "Document →
  itself"). (d) `selection.rs` is **not** changed (P-7, §7). The node memo's planned promotion of the effective
  node-document rule into `EcsDom::effective_node_document` (node memo D8, §4 "Node document" row, edge n3) is
  subsumed: the rule is replaced by the stored fact and the function is `EcsDom::node_document` (O-1: the PM ratifies
  the in-place amendment, §8 R-4).
- **D-8 — the fragment parse keeps its early re-home.** `finish_detached_fragment` calls `adopt_subtree(root, doc)`
  (not `adopt`): it models the adoption that the spec's caller performs when it inserts the returned nodes, done early
  because the throwaway document is destroyed before the caller inserts (`teardown.rs:318-343`). It runs with the
  dispatcher taken (H-11), so no `Adopt` event fires — matching the spec, where nodes created in the fragment parser's
  document are not custom at that adoption (the parser does not upgrade them there). The insertion that follows sees
  oldDocument = the context's document and does nothing (D-2 step 3 guard).
- **D-9 — the umbrella's §1.2 row is C0's to add (PM decision 144).** The PR that lands this memo on `main` (a plan
  PR, as #570 was for N1; or C0's implementation PR if the PM ships plan and code together) carries, in place, the
  umbrella's C0 row in §1.2, an edge row in §1.3 and the order line (§8 R-3 gives the exact texts). Reason: C0 lands
  before C1, so if C1's PR added both rows the umbrella would omit a landed member between the two landings; C1's
  draft R-1 then adds only its own row (§8 H-C1).

## §1. Shape and verdict

**Axes (§2): 8** — A1 the stored fact (D-1), A2 the insertion position (D-4, D-5), A3 the walk (D-3), A4 adoption
notification and hooks (D-6), A5 fragment and DOMParser builds (D-8), A6 node-document consumers (D-7), A7 the EH
program's contract (N1, slice a, T3 after C0, C1), A8 ownerless nodes. ≥ 3 intersecting axes, so this is edge-dense:
PM decision 143's classification holds and plan-review is mandatory (CLAUDE.md *Edge-dense work* (b)). As a per-PR
slice of the approved umbrella it is a terminal unit once its plan-review passes (base case (c)).

**Split decision: one PR.** The candidates and why each split is worse:
- *Adoption first, consumer collapse later.* Between the two PRs `AssociatedDocument` would be correct and N1's
  proxy would still derive the document by position: two derivations of one fact, the strangler state CLAUDE.md
  *One issue, one way* forbids, and every facet the proxy has would persist with a correct fact beside it.
- *Consumer collapse first.* Without adoption the stored fact is wrong for every moved node (H-1), so deleting the
  proxy or the fallbacks would make N1's gate and the getter worse than HEAD (cells T1–T3).
- *`adoptedCallback` later.* `MutationEvent::Adopt` would ship with no consumer (dead code, CLAUDE.md *設計優先*), or
  not ship, leaving adopt step 3.3.3 unimplemented on the first path that reaches it (H-12).
So the fact, its maintenance, its single reader and its one notification land together. The prerequisite 1000-line
splits are none (§5.3); the session's `mutation/mod.rs` is re-measured at the base (O-4 (b)).

**K-0 lens (umbrella K-0: no step worse than HEAD).** Every observable change of §5.4 moves an outcome from HEAD's to
the spec's. Two of them are more restrictive than HEAD (R-3: a bound node moved into a DOMParser subtree stops
compiling raw handlers; R-6: the `ownerDocument` of a moved node changes), both the spec's.

### §1.1 Browser measurement

None. Every outcome in §6 is defined by DOM §4.2.3 / §4.5 and HTML §4.13.6; the one elidex state with no spec
counterpart (a node without a node document, A8) is decided by D-2's and D-4's direction rules, not by a measurement.

## §2. Coupled invariants

### §2.0 Where adopt sits inside *insert* (the boundary for L3's P1)

DOM §4.2.3 *insert* (`webref body dom concept-node-insert`, 2026-10-31), for each node in nodes, in tree order: **7.1
"Adopt node into parent's node document"**; 7.2 / 7.3 append or insert into parent's children; 7.4–7.6 slot
assignment; 7.7 for each shadow-including inclusive descendant: 7.7.1 the insertion steps, 7.7.3 `connectedCallback`
or upgrade; then **8** queue the tree mutation record (unless suppressObservers); **9** the children changed steps;
**10–12** the post-connection steps over a static list. Before step 7: step 4.1 removes a fragment's children (with
suppressObservers) and 4.2 queues the fragment's record. DOM §4.5 *adopt* (`webref body dom concept-node-adopt`):
step 2 removes node from its old parent (the *remove* algorithm, with its removing steps and an unsuppressed record),
step 3 re-homes, 3.3.3 enqueues `adoptedCallback`, 3.4 runs the adopting steps. *Replace* (`webref body dom
concept-node-replace`): **step 6 adopt node** precedes step 7 (remove child, suppressed) and step 9 (insert, which
adopts again as a no-op).

elidex after C0, per primitive (H-10 for HEAD's order):

| elidex step | spec step | event / record |
|---|---|---|
| `is_connected` capture | (input to 7.7 and the CE gates) | — |
| removal from the old parent (`detach_with_hook`) | adopt step 2 (*remove*) | `MutationEvent::Remove` (old parent); session source-removal record (H-7) |
| re-home (`adopt_subtree`) | adopt step 3.1 (and 3.4's template step, identity under C2) | — |
| `MutationEvent::Adopt` (NEW) | adopt step 3.3.3 (`adoptedCallback` enqueue) and the adopting-steps hook | `Adopt` iff oldDocument is `Some` and ≠ document |
| `replace_child` only: `old_child` removal | replace step 7 | `Remove` (old child) |
| `link_node` | insert 7.2 / 7.3 | — |
| `rev_version` | (elidex cache invalidation) | — |
| `fire_after_insert` | insert 7.4–7.7, 9, 12 (consumers), 8 (session record layer) | `MutationEvent::Insert` |

**The contract P1 inherits:** adoption is complete (stamp written, `Adopt` dispatched) before `link_node`, and therefore
before every insertion-steps, children-changed, post-connection and connectedCallback consumer and before the
destination record; it follows the old parent's removal (`Remove`, removing steps, source record). P1 may restructure
`fire_after_insert` into explicit hook phases; it must not move the adopt step after any of them, and the
adopting-steps hook is `MutationEvent::Adopt` (one per adopted root), not a new per-descendant hook. If P1's umbrella
wants a per-descendant adopting-steps hook (DOM's "Run the adopting steps with inclusiveDescendant"), it replaces
D-6's consumer walk, not D-4's position.

### §2.1 Invariant per axis

| axis | invariant |
|---|---|
| A1 fact | **I-U (uniformity):** for every node n with a parent or host p, if `node_document(p)` is `Some(d)` then `node_document(n) == Some(d)`. `node_document` reads only the stamp (or a Document's identity); no reader derives a node document from tree position |
| A2 position | At every link, the child's adoption completes after its old-parent removal and before `link_node`; in `replace_child`, before `old_child`'s removal; a replaced-in fragment is adopted itself; a pre-inserted fragment keeps its node document |
| A3 walk | Adoption re-homes the root's shadow-including inclusive descendants, uncapped, including template contents (to the same document under C2) |
| A4 notification | `Adopt` fires exactly once per adopted root whose oldDocument is a live Document different from the target, before that root's `Insert`; every custom element in its shadow-including subtree gets exactly one `Adopted` reaction per such adoption, enqueued before its `Connected` reaction |
| A5 builds | Dispatcher-suppressed builds (fragment parse, DOMParser, strict error rollback) fire no `Adopt`; every node a build returns carries the receiving document's stamp |
| A6 consumers | Every node-document read in `crates/` goes through `node_document` / `owner_document`; no `find_tree_root_composed`-based or "Document → itself" local derivation remains (§5.1 P-4, P-5) |
| A7 EH contract | N1's compile gate and (after C0) T3's step 3 read the node document D-1 defines; slice a's Insert arm (T-a2) sees the adopted document; K-0 holds |
| A8 ownerless | A node with no node document (an engine-created node never linked under a documented parent) keeps `None` until its first adoption; linking a stamped node under an ownerless parent keeps its stamp; neither fires `Adopt` |

### §2.2 All pairs (28), with intersection

| pair | intersect? | intersection |
|---|---|---|
| A1×A2 | yes | I-U is established by D-4 at every link (H-2's chokepoint); a primitive that linked before adopting would let an `Insert` consumer read the old stamp (U4) |
| A1×A3 | yes | I-U across the host edge and the contents edge needs the walk to be shadow-including and to include contents; a light-only walk leaves shadow content on the old document (U1, T4) |
| A1×A4 | yes | `Adopt`'s `old_document` is `node_document(root)` before the re-home; `new_document` the target; both are stamps, so the consumer reads no tree position |
| A1×A5 | yes | Builds create nodes unstamped or stamped with the throwaway document; the build's own appends stamp them (A8's silent path); the finishing re-home gives the context's document. The strict error path despawns, so no stale stamp survives (`despawn_subtree`, #568) |
| A1×A6 | yes | Deleting the fallbacks is sound only if I-U holds for every connected node; U8 scans every node of a parsed document (both tiers) for the stamp, which is the guard that the deletion removed only dead code |
| A1×A7 | yes | N1's node clause becomes `node_document(entity) != bound`; for a connected node this equals the proxy's composed-root answer (I-U across host edges), for a detached node it is the stamp the last adoption wrote (T8–T10) |
| A1×A8 | yes | `None` from `node_document` now means "never linked under a documented parent" (or a destroyed document); before C0 it also meant "unstamped and detached", which included every removed main-document parse node (T3) |
| A2×A3 | yes | The walk runs between removal and link, on a detached subtree, so it never visits the new parent's tree |
| A2×A4 | yes | `Adopt` precedes `Insert` for the same root (D-4); in `replace_child`, `Adopt(new_child)` precedes `Remove(old_child)` (replace step 6 before 7), so `adoptedCallback` for the new child is enqueued before `disconnectedCallback` for the old one (T7 variant) |
| A2×A5 | yes | During a build, each append adopts from `None` silently; the finishing re-home replaces the throwaway stamp; the caller's later insertion is a no-op (D-8; U9) |
| A2×A6 | yes | The base URL maintainer's `Insert` arm reads `node_document(node)` after the re-home, so a moved `<base>` updates the receiving document; its `Remove` arm reads the old parent's document before (T12) |
| A2×A7 | yes | Slice a's T-a2 (the Insert arm of the content-attribute consumer) processes a node's raw handlers at its insertion, after D-4's adoption, so N1's gate sees the adopted document there (T8) |
| A2×A8 | yes | Linking under an ownerless parent: removal happens, the re-home does not (U3) |
| A3×A4 | yes | The consumer's walk visits the same shadow-including subtree as the re-home, in shadow-including tree order; template contents are visited (elements in contents can be custom only if upgraded there; C1 makes that impossible by giving contents the inert document) |
| A3×A5 | yes | The finishing re-home uses the same walk (D-8); a build that creates a template under the throwaway document gets its contents re-homed with it (C2) |
| A3×A6 | yes | Every consumer reads the descendant's own stamp; none walks up (D-7) |
| A3×A7 | yes | A handler on a shadow-tree element of a moved host reads the new document through its own stamp (the walk reached it), not through a composed root (T4 with handler variant) |
| A3×A8 | no | The walk writes the target document wherever it reaches; ownerless status is a property of the start, A2×A8 |
| A4×A5 | yes | Suppressed builds fire no `Adopt` (H-11), so fragment-parse and DOMParser nodes never produce `adoptedCallback` from the build (U9) |
| A4×A6 | no | `Adopt` has one consumer (custom elements); the node-document consumers read the stamp, not the event |
| A4×A7 | yes | `adoptedCallback` runs script at the next `[CEReactions]` boundary, after the insertion completes; slice a's T-a2 processing happens inside the `Insert` dispatch, before it (T6 order cell) |
| A4×A8 | yes | Silent re-home of an unstamped root (D-2): an engine-created custom element under an ownerless root would miss `adoptedCallback`; Q-S2 measures whether such a node is script-reachable |
| A5×A6 | yes | A DOMParser document's nodes read the DOMParser document through their stamp (unchanged); after a move they read the receiving document (T1) |
| A5×A7 | yes | N1's x4 hand-off (raw values at DOMParser-node creation read null) is unchanged: those nodes stay in the DOMParser document until moved |
| A5×A8 | yes | `finish_detached_fragment` skips its re-home when the context has no node document (`teardown.rs:336`); the returned nodes keep the destroyed throwaway's stamp, which `node_document` reads as `None` (dangling guard), the same as HEAD |
| A6×A7 | yes | N1's clause (b) and the node memo's D8 rule are one function after C0; T3 (landing after C0 and C1) reads `node_document` directly, so its D-T4 extraction is not needed (§8 H-T) |
| A6×A8 | yes | The `ownerDocument` getter keeps its `document_root()` fallback for `None` (`crates/dom/elidex-dom-api/src/node_methods/core.rs:279`): ownerless creation is §7's defect, not a node-document derivation |
| A7×A8 | yes | `None` keeps N1's fail-open and HEAD's target rule (unchanged by C0) |

Count: 28 pairs; 26 intersect, 2 do not (A3×A8, A4×A6). Re-count:
`awk '/^### §2.2/{f=1} /^## §3/{f=0} f && /^\| A[0-9]×A[0-9]/' <memo> | awk -F'|' '{print $3}' | sort | uniq -c`.

## §3. Spec coverage map

Lookups (2026-10-31): `.claude/tools/webref coverage-map dom 4.2.3 dom 4.5 dom 4.4 dom 4.2.1 html 4.12.3 html 4.13.6
html 13.4 html 2.4.3 html 8.1.8.1` (verdict "ok → single PR scope"), then `heading dom 4.8`, `heading dom 4.3.2`,
`heading dom 4.2.2`, `heading html 8.5.1`; `dfn dom 'adopt'` → §4.5 `#concept-node-adopt`; `dfn dom 'adopting steps'`
→ §4.5 `#concept-node-adopt-ext`; `dfn dom 'insert'` → §4.2.3 `#concept-node-insert`; `dfn dom 'replace'` → §4.2.3
`#concept-node-replace`; `dfn dom 'pre-insert'` → §4.2.3; `dfn dom 'replace all'` → §4.2.3; `dfn dom 'node document'`
→ §4.4 `#concept-node-document`; `dfn dom 'shadow-including inclusive descendant'` → §4.8; `dfn dom 'insertion steps'`,
`'children changed steps'`, `'post-connection steps'` → §4.2.3; `dfn dom 'queue a tree mutation record'` → §4.3.2;
`dfn dom 'connected'` → §4.2.2; `dfn html 'enqueue a custom element callback reaction'` → §4.13.6; `dfn html
'determine the target of an event handler'` → §8.1.8.1; `dfn html 'parse HTML from a string'` → §8.5.1; step text from
`body dom concept-node-insert`, `body dom concept-node-adopt`, `body dom concept-node-replace`, `body dom
concept-node-replace-all`, `body dom concept-node-pre-insert`, `body dom concept-node-document`, `body dom
dom-node-ownerdocument`, `body html the-template-element`.

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG DOM §4.2.3 Mutation algorithms | *insert* step 7.1 | adopt each node of nodes into parent's node document, before 7.2 | `append_child` `crates/core/elidex-ecs/src/dom/tree/mutation.rs:22`, `insert_before` `:95` (D-4) | ✓ | yes |
| WHATWG DOM §4.2.3 Mutation algorithms | *insert* steps 4, 7.2–7.7, 8, 9, 10–12 | position of adopt relative to each (§2.0) | `fire_after_insert` `mutation.rs:282` (unchanged); session records `crates/script/elidex-script-session/src/mutation/mod.rs:429` | ✓ | yes |
| WHATWG DOM §4.2.3 Mutation algorithms | *replace* steps 6, 7, 9, 10 | 6 adopt node (a fragment itself included) before 7 removing child | `replace_child` `mutation.rs:138` (D-4); fragment arm `mutation/mod.rs:689` (D-5); record siblings (P-3, §7) | ✗ (record siblings) | yes |
| WHATWG DOM §4.2.3 Mutation algorithms | *pre-insert*, *replace all* | delegate to *insert* | `apply_append_child` `mutation/mod.rs:480`, `apply_insert_before` `:545`, `apply_replace_all` `:813` (unchanged; reach D-4) | ✓ | yes |
| WHATWG DOM §4.5 Interface Document | *adopt* steps 1, 2, 3.1, 3.4 | 1 oldDocument; 2 remove from parent; 3 document ≠ oldDocument → per shadow-including inclusive descendant: 3.1 set node document, 3.4 adopting steps | `EcsDom::adopt` (NEW) and `adopt_subtree` `crates/core/elidex-ecs/src/dom/tree/teardown.rs:251` (moves to `mutation.rs`) (D-2, D-3) | ✓ | yes |
| WHATWG DOM §4.5 Interface Document | *adopt* step 3.3.3 | custom → enqueue `adoptedCallback` « oldDocument, document » | `MutationEvent::Adopt` (NEW); consumer arm `crates/dom/elidex-custom-elements/src/consumer.rs:79`; flush `crates/script/elidex-js/src/vm/host/custom_elements/flush.rs:133` (D-6) | ✓ | yes |
| WHATWG DOM §4.5 Interface Document | *adopt* steps 3.2, 3.3.1, 3.3.2 | shadow-root / element registry; attribute nodes' node document | not modelled (§7, Q-S3, Q-S4) | ✗ | yes |
| WHATWG DOM §4.5 Interface Document | `adoptNode(node)` steps 1–4 | Document → NotSupportedError; ShadowRoot → HierarchyRequestError; adopt; return | absent (H-15; O-2) | ✗ | yes |
| WHATWG DOM §4.4 Interface Node | *node document*; `ownerDocument` getter | every node has one; a document's is itself; getter null for a document | `node_document` (NEW), `owner_document` `crates/core/elidex-ecs/src/dom/mod.rs:821` (D-1); getter `crates/dom/elidex-dom-api/src/node_methods/core.rs:279` | ✗ (ownerless, §7) | no |
| WHATWG DOM §4.2.1 Document tree | (connectedness vs node document) | a node's document tree vs its node document; equal for connected nodes under I-U | `EcsDom::is_connected` `crates/core/elidex-ecs/src/dom/tree/walkers.rs:143` (unchanged) | ✓ | no |
| WHATWG DOM §4.8 Interface ShadowRoot | *shadow-including inclusive descendant* | the walk's order | `for_each_uncapped_shadow_inclusive` `walkers.rs:99` (unchanged) | ✓ | no |
| WHATWG DOM §4.3.2 Queuing a mutation record | tree mutation record for adopt's removal | the source-removal record precedes the destination record | `source_removal_record` `mutation/mod.rs:341` (unchanged) | ✓ | yes |
| WHATWG DOM §4.2.2 Shadow tree | *connected* | shadow-including root is a document | `EcsDom::is_connected` (H-13; unchanged) | ✓ | no |
| WHATWG HTML §4.12.3 The template element | template *adopting steps* steps 1–2 | contents to the appropriate template contents owner document (C2: the template's document) | `adopt_subtree` walk with `TemplateContentsWalk::Include` (D-3; C1 replaces the target) | ✗ (C1) | yes |
| WHATWG HTML §4.13.6 Custom element reactions | *enqueue a custom element callback reaction* | `adoptedCallback` with « oldDocument, document » | consumer arm + flush (D-6) | ✓ | yes |
| WHATWG HTML §13.4 Parsing HTML fragments | the throwaway document; returned nodes | nodes are created in the throwaway document; the caller's insertion adopts them | `finish_detached_fragment` `teardown.rs:330` (D-8, unchanged) | ✓ | yes |
| WHATWG HTML §8.5.1 The DOMParser interface | *parse HTML from a string* | the new document owns its nodes | `parse_into_inert_document` `crates/dom/elidex-form/src/inert_document.rs:110` (unchanged) | ✓ | yes |
| WHATWG HTML §2.4.3 Document base URLs | *document base URL* | per node document | `BaseUrlMaintainer` `crates/dom/elidex-dom-api/src/element/document_base.rs:173-178` (D-7 (b)); `href_accessor.rs:50`, `document_props.rs:75` (D-7 (c)) | ✓ | yes |
| WHATWG HTML §8.1.8.1 Event handlers | *getting the current value of the event handler* step 3.2 (N1's compile gate); *determine the target of an event handler* step 3 (T3, after C0) | node document of the element | clause (b) `crates/script/elidex-script-session/src/scripting.rs:83-114` (D-7 (a)); `resolve_handler_target` `crates/script/elidex-script-session/src/event_handler_processing/mod.rs:249` (unchanged at C0, H-14) | ✓ | yes |

### §3.1 User-input touch audit

Rows marked *yes* carry author markup or script-built trees. C0 parses nothing and stores no string; the `Adopt` event
carries entities only. What changes is which Document an author node is attributed to after a move, and with it
three reads: `ownerDocument` (and the Document methods reached through it), URL resolution through the document base
URL, and N1's compile gate. One new script entry point runs author code: `adoptedCallback`, through the existing
reaction flush (same error routing as `connectedCallback`, `flush.rs:144-`). No path attributes author content to a
document it was not inserted into.

## §4. Mechanism

### §4.1 Home / kind

- `node_document`, `adopt`, `adopt_subtree` and `MutationEvent::Adopt` live in **`elidex-ecs`** (engine-independent;
  every crate that links nodes depends on it). The session's fragment arm (D-5) and the custom-element consumer
  (D-6) are engine-independent crates. The VM change is the flush arm's callback invocation (marshalling two Document
  wrappers), within the Layering mandate.
- No component is added (D-1). The event is transient (an occurrence, not state), so it is a `MutationEvent` variant,
  not a marker.

### §4.2 Reconcile — HEAD → C0

| site | HEAD | C0 |
|---|---|---|
| `owner_document` `mod.rs:821-860` | stamp, else tree-root Document, else fragment root's stamp | `None` for a Document, else `node_document` (stamp only) |
| `append_child` / `insert_before` `mutation.rs:40`, `:114` | `detach_with_hook` then link | removal, re-home + `Adopt` (D-4), then link |
| `replace_child` `mutation.rs:164-197` | `detach_with_hook(new_child)`, `detach(old_child)`, link | removal of `new_child`, its re-home + `Adopt`, `old_child`'s removal, link |
| `adopt_subtree` `teardown.rs:251-259` | walk + stamp | same walk, moved to `mutation.rs`; called by `adopt` and `finish_detached_fragment` |
| session fragment replace `mutation/mod.rs:713` | `remove_child(old_child)` first | `adopt(fragment, node_document(parent))`, then `remove_child(old_child)` |
| clause (b) `scripting.rs:95-113` | Document arm + composed-root proxy | one arm over `node_document` |
| `document_base.rs:173-178` | local `owner_doc` | deleted; callers use `node_document` |
| `href_accessor.rs:50`, `document_props.rs:75` | `owner_document(x).unwrap_or(x)` | `node_document(x).unwrap_or(x)` |
| CE consumer `consumer.rs:79-98` | no `Adopt` arm | `Adopt` arm (D-6) |
| CE flush `flush.rs:133-138` | no-op | invoke `adoptedCallback(old, new)` |

### §4.3 Cost

Every node of a main-document parse acquires `AssociatedDocument` at its append (H-4's unstamped population): one
`insert_one` per node, an archetype move in hecs. The walk per append is O(subtree), which for a parser append is 1.
Measured in the PR (§9 item 7, R-7): if the parse benchmark regresses beyond noise, the remedy is to stamp at creation
in the parsers (the spec's "set upon creation"), not to restore the fallback.

### §4.4 What C0 provides to C1 (C1 draft §4.6, C0-1..C0-4)

| C1 requirement | C0's answer |
|---|---|
| **C0-1** every path *insert* covers (and `replaceChild`, the ParentNode / ChildNode mixins, `innerHTML` / `outerHTML` / `setHTMLUnsafe` placement, `Range` insertion) adopts each inserted node into the parent's node document through the adopt function | D-4 at the three primitives, which are the only linking sites (H-2, §5.1 P-1); the mixins, `innerHTML` placement and `Range` reach them (H-7; Q-S5 confirms `setHTMLUnsafe` / `outerHTML`); `adopt` → `adopt_subtree` is the one function C1's D-C5 edits |
| **C0-2** the parent's node document of a template contents fragment is the fragment's node document | `node_document(fragment)` = the fragment's stamp (D-1); a contents fragment is created stamped (`attach_template_contents`, `mod.rs:561`), so a node appended into contents is re-homed to it (U1 contents variant) |
| **C0-3** adoption happens before the node's `MutationEvent::Insert` | §2.0 / D-4; U4 pins the order |
| **C0-4** decide the effective-node-document proxy's collapse and its interaction with T3's `effective_node_document` and N4's promotion | Collapsed: clause (b) reads `node_document` (D-7 (a)); T3 lands after C0 and uses `node_document` (no extraction, §8 H-T); N4's promotion is subsumed (D-7, O-1, §8 R-4) |

### §4.5 ECS-native and Layering checks

- One fact (the component), one writer (the adopt function), one reader (`node_document`), one notification (the
  event). No side-store, no observer registry; `adoptedCallback` is a consumer arm on the mutation stream, as
  `connectedCallback` already is.
- No algorithm in `vm/host/`: the VM marshals the callback's arguments only.

### §4.6 Docstring citation table (Axis 4)

| symbol | citation |
|---|---|
| `node_document` (NEW) | DOM §4.4 *node document* ("The node document of a document is that document itself") |
| `owner_document` (doc rewrite) | DOM §4.4 `ownerDocument` getter |
| `adopt` (NEW) | DOM §4.5 *adopt* steps 1–3 (3.2, 3.3.1, 3.3.2 not modelled, named) |
| `adopt_subtree` (doc rewrite) | DOM §4.5 *adopt* steps 3.1 and 3.4; HTML §4.12.3 template *adopting steps* (C2: identity) |
| `append_child` / `insert_before` (doc) | DOM §4.2.3 *insert* step 7.1 |
| `replace_child` (doc) | DOM §4.2.3 *replace* steps 6–7 |
| `MutationEvent::Adopt` (NEW) | DOM §4.5 *adopt* step 3.3.3 and the *adopting steps* hook; HTML §4.13.6 |
| clause (b) docstring (rewrite) | HTML §8.1.8.1 step 3.2 node clause; DOM §4.4 *node document* |

### §4.7 Comment sites made true or false by C0

`/usr/bin/grep -rn 'cross-document-adopt-on-insert' crates` → `scripting.rs:51` (rewritten, D-7 (a)); P-2's two
comments (become true; citation numbers checked against §3); `owner_document`'s docstring `mod.rs:806-820` (P-4,
rewritten); `split_text.rs:142-147` ("legacy entities relying on the tree-root Document fallback", rewritten: the
fallback no longer exists); `adopt_subtree`'s docstring (`teardown.rs:245-250`, "Used by the HTML §13.4 fragment
parser", rewritten for two callers).

## §5. Populations, touched files, named risks

### §5.1 Populations (re-runnable)

- **P-1 linking sites** (H-2): `/usr/bin/grep -rn 'link_node(' crates --include='*.rs'` → definition + 3 calls;
  `/usr/bin/grep -rn 'rel.parent = ' crates --include='*.rs'` → 2 (link, clear). Any new hit is a linking path D-4 does
  not cover.
- **P-2 adopt callers**: `/usr/bin/grep -rnF 'adopt_subtree(' crates --include='*.rs'` → at `1ed126b4` def + 1 prod +
  1 test; at C0's head def + `adopt` + `finish_detached_fragment` + tests. `/usr/bin/grep -rnF '.adopt(' crates
  --include='*.rs'` → the three primitives' shared path and the session fragment arm (and `adoptNode` if O-2 (a)).
- **P-3 node-document readers**: `/usr/bin/grep -rn 'owner_document(' crates --include='*.rs' | /usr/bin/grep -v
  '/tests/\|tests\.rs\|/tests_\|test_helpers' | /usr/bin/grep -v 'fn owner_document\|validate_owner_document'` → at
  `1ed126b4`: `teardown.rs:336`, `tree_clone.rs:208`, `scripting.rs:109`, `cssom_sheet.rs:213`, `href_accessor.rs:50`,
  `element/tree.rs:430`, `select.rs:326`, `document_base.rs:177`, `document_props.rs:75`, `split_text.rs:148`,
  `focus/delegate.rs:85`, `node_methods/core.rs:279`, `text_content.rs:119`, `clone.rs:242`, `selection.rs:618`,
  `elidex-html-parser/src/lib.rs:161`, `insert.rs:114` (plus doc-comment mentions). Classification: **code changes**
  (D-7): `scripting.rs:109`, `document_base.rs:177`, `href_accessor.rs:50`, `document_props.rs:75`. **Values change,
  code does not** (they now read the adopted document): `cssom_sheet.rs:213`, `element/tree.rs:430`, `select.rs:326`,
  `split_text.rs:148`, `text_content.rs:119`, `focus/delegate.rs:85`, `core.rs:279`, `clone.rs:242`, `tree_clone.rs:208`,
  the parsers' context reads, `teardown.rs:336`. **Not a node-document consumer by spec** (P-7): `selection.rs:618`.
- **P-4 position-based proxies**: `/usr/bin/grep -rn 'find_tree_root_composed' crates --include='*.rs' | /usr/bin/grep
  -v '/tests/\|tests\.rs\|/tests_'` → `scripting.rs:104` (**deleted**, D-7 (a)), `node_proto.rs:455` (`isConnected`,
  connectedness, not a node document; P-6), the definition `walkers.rs:342`. The fallbacks of `owner_document`
  (`mod.rs:839-856`) are **deleted** (D-1). After C0 the first command returns the definition and `node_proto.rs`
  only.
- **P-5 "Document → itself" idioms**: `/usr/bin/grep -rn 'owner_document(.*)\.unwrap_or(' crates --include='*.rs'` → 2
  (D-7 (c)); `/usr/bin/grep -rn 'fn owner_doc\b' crates --include='*.rs'` → 1 (D-7 (b)). After C0: 0 and 0.
- **P-6 `MutationEvent` consumers**: `/usr/bin/grep -rln 'MutationEvent::Insert' crates --include='*.rs' |
  /usr/bin/grep -v '/tests/\|tests\.rs\|/tests_'` → 11 files at `1ed126b4` (verified 2026-10-31); each `match` must compile with the new
  variant (exhaustive matches, if any, gain an explicit no-op arm; Q-S1). Only the custom-element consumer acts on it.
- **P-7 Document-destroying paths** (stale stamps): `finish_detached_fragment` (`teardown.rs:341`), the strict error
  path (`despawn_subtree(document)`, #568); DOMParser documents' lifetime is Q-S7.
- **P-8 ownerless creations** (A8): `create_element(` / `create_text(` / `create_comment(` / `create_document_fragment(`
  calls without an owner, production code only; the census is Q-S2 (the raw grep counts inline test modules).

### §5.2 Existing tests and docstrings whose expectation changes

Any test asserting that a cross-document insert keeps the source document (or that a removed parser-built node
reports the bound document) turns red by design; candidates: `crates/core/elidex-ecs/src/dom/tests/associated_document.rs`
(187 lines), the `scripting.rs` inline tests (`:117-`, the two-document fixture `:125-131`), the N1 cells that pin the
proxy facets (N1 memo C8 family: the PR re-derives them on N1's landed base), the base-URL maintainer tests in
`document_base.rs`. The complete list at C0's base is Q-S8. Docstrings: §4.7.

### §5.3 Touched files

| file | lines at `1ed126b4` | change | 1000-line decision |
|---|---|---|---|
| `crates/core/elidex-ecs/src/dom/mod.rs` | 930 | `node_document` (NEW); `owner_document` body and doc; fallbacks deleted | net ≈ −10; stays < 1000 |
| `crates/core/elidex-ecs/src/dom/tree/mutation.rs` | 467 | `adopt` (NEW), `adopt_subtree` (moved in), the three primitives | < 1000 |
| `crates/core/elidex-ecs/src/dom/tree/teardown.rs` | 344 | `adopt_subtree` moved out; `finish_detached_fragment` doc | < 1000 |
| `crates/core/elidex-ecs/src/dom/mutation_event.rs` | 352 | `Adopt` variant (NEW) | < 1000 |
| `crates/script/elidex-script-session/src/mutation/mod.rs` | 994 | fragment-replace adopt (D-5), ≈ +4 lines | **re-measure at the base**: if the edit reaches 1000, split first as a standalone pure-move PR (seam: the `apply_replace_*` family vs the record helpers; O-4 (b)) |
| `crates/script/elidex-script-session/src/scripting.rs` | 293 | clause (b) and docstring (D-7 (a)) | < 1000 |
| `crates/dom/elidex-dom-api/src/element/document_base.rs` | 555 | D-7 (b) | < 1000 |
| `crates/dom/elidex-dom-api/src/element/href_accessor.rs` | 567 | D-7 (c) | < 1000 |
| `crates/dom/elidex-dom-api/src/char_data/document_props.rs` | 403 | D-7 (c) | < 1000 |
| `crates/dom/elidex-dom-api/src/char_data/split_text.rs` | 447 | doc (§4.7) | < 1000 |
| `crates/dom/elidex-custom-elements/src/consumer.rs` | 329 | `Adopt` arm | < 1000 |
| `crates/script/elidex-js/src/vm/host/custom_elements/flush.rs` | 237 | `Adopted` invocation | < 1000 |
| `crates/dom/elidex-dom-api/src/selection.rs` | 1036 | **not touched** (P-7) | not triggered |
| `crates/core/elidex-ecs/src/dom/tests/adopt_on_insert.rs` (NEW) | — | U-cells | new file |
| `crates/script/elidex-js/src/vm/tests/tests_insertion_adoption.rs` (NEW) | — | T-cells | new file |

### §5.4 Named risks (and the observable change each is)

- **R-1 `ownerDocument` after a cross-document move** → the receiving document (spec). Scripts that relied on a
  DOMParser node keeping its document after `append` see the page document.
- **R-2 a removed main-document parse node** → keeps the page document (spec); HEAD reported it through the getter's
  `document_root()` fallback, so the page-document value is unchanged, but a parse node removed from a DOMParser tree
  now reports the DOMParser document instead of the page (T3).
- **R-3 handlers** → N1's compile gate follows the adopted document: a page node moved into a detached DOMParser
  subtree stops compiling raw handlers (more restrictive than HEAD, the spec's), a DOMParser node moved into the page
  (connected or detached) compiles them (the spec's; NR3's facet closes).
- **R-4 base URL** → a `<base>` moved into another document updates that document's base URL, not the source's (T12).
- **R-5 `adoptedCallback` runs author code** where HEAD ran none (spec).
- **R-6 replace with a fragment** → the fragment's `ownerDocument` becomes the parent's document (spec; T11).
- **R-7 parse cost** (§4.3).
- **R-8 P1 collision** → §2.0 is the contract; the PM tells L3.

## §6. Test plan

Columns: *HEAD/base* = `1ed126b4` and C0's base (N1 landed), equal unless stated; *C0 = spec* (under C2 for template
rows). **Red-first**: every *red* cell is run on the base before the fix with its control green, and the failure is
recorded in the PR description. Cross-tier: cells marked (×2) run under both parser tiers. `dp` below is `new
DOMParser().parseFromString('<div id=d><span id=s></span></div>', 'text/html')`.

### §6.1 VM integration (`tests_insertion_adoption.rs`, NEW)

| cell | kind | script | HEAD/base | C0 = spec |
|---|---|---|---|---|
| T1 | red | `d = dp.getElementById('d'); s = d.firstChild; document.body.append(d); [d.ownerDocument === document, s.ownerDocument === document]` | `[false, false]` (stamped `dp`, H-4) | `[true, true]` |
| T2 | red | `x = document.createElement('div'); dp.body.append(x); x.ownerDocument === dp` | `false` | `true` |
| T3 | red (×2) | initial HTML `<p id=p></p>`: `dp.body.append(p); a = p.ownerDocument === dp; p.remove(); [a, p.ownerDocument === dp]` | `[true, false]` (tree-root fallback while connected; `document_root()` after removal) | `[true, true]` |
| T4 | red | `h = dp.createElement('div'); sr = h.attachShadow({mode:'open'}); sr.innerHTML = '<b></b>'; document.body.append(h); [sr.ownerDocument === document, sr.firstChild.ownerDocument === document]` | `[?, false]` (shadow root unstamped → tree root; Q-S9) | `[true, true]` |
| T5 | red (C2) | `dp2 = parseFromString('<template id=t><b></b></template>','text/html'); t = dp2.getElementById('t'); document.body.append(t); [t.content.ownerDocument === document, t.content.firstChild.ownerDocument === document]` | `[false, false]` | `[true, true]` under C2; C1 turns both into the inert document |
| T6 | red | `log = []; customElements.define('x-a', class extends HTMLElement { adoptedCallback(o, n) { log.push(['a', o === document, n === dp]) } connectedCallback() { log.push(['c', this.ownerDocument === dp]) } }); e = document.createElement('x-a'); dp.body.append(e); log` | `[['c', false]]` (dp's tree is connected, H-13) | `[['a', true, true], ['c', true]]` |
| T7 | red | continue T6: `log = []; document.body.append(e); log` plus a `disconnectedCallback` pushing `'d'`; variant `document.body.replaceChild(e2, old)` with `old` custom | `[d, c]` | `[d, ['a', false, true]…, c]` in that order (adopt precedes link; in the replace variant `Adopted(e2)` precedes `Disconnected(old)`) |
| T8 | red (NR3 facet; needs N1) | `d2 = dp3.getElementById('q')` from `'<div id=q onclick="window.hit=1"></div>'`; `x = document.createElement('div'); x.append(d2); typeof d2.onclick` | `'object'` (null: owner `dp3`) | `'function'` |
| T9 | red (adopt-then-remove; needs N1) | `document.body.append(d2); d2.remove(); typeof d2.onclick` | `'object'` | `'function'` |
| T10 | red (under-suppress reverse; needs N1) | `y = document.createElement('div'); y.setAttribute('onclick','window.hit=1'); h = dp.createElement('div'); h.append(y); typeof y.onclick` | `'function'` | `'object'` (null: node document `dp`) |
| T11 | red | `f = dp.createDocumentFragment(); f.append(dp.createElement('i')); c = document.createElement('u'); document.body.append(c); document.body.replaceChild(f, c); f.ownerDocument === document`; variant `document.body.append(f2)` (pre-insert) → `f2.ownerDocument === dp` | `[false, true]` | `[true, true]` (replace adopts the fragment; pre-insert does not) |
| T12 | red | `b = dp.createElement('base'); b.setAttribute('href','https://ex.test/'); x = document.createElement('base'); x.setAttribute('href','https://other.test/'); dp.head.append(x); a = dp.createElement('a'); a.setAttribute('href','y'); dp.body.append(a); a.href` | not `https://other.test/y` (the maintainer updates the page document, `x` being stamped `document`; Q-S10 gives the exact value) | `'https://other.test/y'` |
| T13 | guard (R-B hand-off, PM decision 153) | T3 draft's T-18 shape: `p = document.createElement('div'); dd = parseFromString('<body></body>','text/html'); b = dd.body; p.append(b); [b.ownerDocument === document]`; and X2: `b2 = document.createElement('body'); dd.documentElement.append(b2); b2.remove(); [b2.ownerDocument === dd]` | `[false]`, `[false]` | `[true]`, `[true]` — the node-document facts under which T3's step 3 gives the spec's XW-9 / XW-10 / EW-15 / EW-16 outcomes (§9 item 9) |
| T14 | guard (K-0) | same-document move: `q = document.createElement('x-a'); document.body.append(q); log = []; document.body.prepend(q); log` | `[d, c]` | `[d, c]` (no `Adopted`) |

### §6.2 Unit (`crates/core/elidex-ecs/src/dom/tests/adopt_on_insert.rs`, NEW)

| cell | kind | shape | expected |
|---|---|---|---|
| U1 | red | `append_child(D_parent, n)` with `n` stamped `X`, a child, a shadow root with content, a template with contents; variants `insert_before`, `replace_child`, and an append into a contents fragment stamped `D` | every visited node stamped `D` |
| U2 | red | ownerless `n` appended under `D` | stamped `D`; event recorder sees no `Adopt` |
| U3 | red | `n` stamped `D` appended under an ownerless parent | keeps `D`; no `Adopt` |
| U4 | red | event recorder (a `MutationDispatcher` test double) over `append_child` of an `X`-stamped parented node, and over `replace_child` | `Remove(old parent)`, `Adopt`, `Insert`; for replace: `Remove(new_child's old parent)`, `Adopt`, `Remove(old_child)`, `Insert` |
| U5 | guard | same-document move | no `Adopt`, stamps unchanged |
| U6 | guard | adoption past the depth and breadth caps (as `destroy.rs:381`'s shape) | every node re-homed |
| U7 | red | `node_document(D) == Some(D)`; `owner_document(D) == None`; a node whose stamp's document was destroyed → `None` (no tree-root fallback) | as stated |
| U8 | guard (fallback deletion) | parse a document with templates, a declarative shadow root and foreign content (×2); every node under it except the Document carries `AssociatedDocument == D` | all stamped |
| U9 | guard | fragment parse into a context in `D` with a dispatcher installed around it | no `Adopt` during the build; returned nodes stamped `D` |
| U10 | red | `EcsDom::adopt(fragment, D)` on a parentless fragment with children | fragment and children stamped `D`, one `Adopt` |

### §6.3 Mutation guards

| mutation | killed by |
|---|---|
| no adoption in the primitives (HEAD) | T1, T2, U1 |
| adopt after `link_node` | U4, T7 |
| light-tree-only walk | T4, U1 (shadow variant) |
| `Adopt` fired for an ownerless root | U2 |
| `Adopt` fired on a same-document move | T14, U5 |
| fallbacks kept (dead code) | U7 (stale stamp → `None`); U8 makes the deletion safe |
| replace-with-fragment does not adopt the fragment | T11, U10 |
| pre-insert adopts the fragment | T11 variant |
| CE arm missing or `Adopted` after `Connected` | T6, T7 |
| clause (b) keeps the proxy | T9, T10 |

### §6.4 Kept green

`crates/core/elidex-ecs/src/dom/tests/destroy.rs` (fragment teardown, #568), `associated_document.rs` (rewritten where
§5.2 says), the #380 template suite, slice a's E1–E8 and U8, N1's cells not listed in §5.2. Commands: `cargo test -p
elidex-ecs --all-features`, `-p elidex-dom-api`, `-p elidex-custom-elements`, `-p elidex-html-parser`, `-p
elidex-html-parser-strict`, `-p elidex-script-session`, `-p elidex-js`, then `mise run ci`.

## §7. Out of scope and slots

| item | where | defect (stated, not the remedy) |
|---|---|---|
| template contents' owner document | C1 (`#11-template-contents-owner-document`) | under C2 the contents' node document is the template's (D-3) |
| ownerless node creation | proposed slot (§8 R-6 (a)) | engine paths create nodes with no node document (P-8); the `ownerDocument` getter reports the bound document for them (`core.rs:279`), and an ownerless root's first adoption fires no `adoptedCallback` (A4×A8) |
| `document.adoptNode` / `importNode` | O-2 | absent (H-15): `document.adoptNode(n)` is a TypeError |
| adopt steps 3.2, 3.3.2 (custom-element registries) | Q-S3; `#11-custom-elements-*` family (PM) | an element's registry is not re-pointed at adoption |
| adopt step 3.3.1 (attribute nodes' node document) | Q-S4 | if `Attr` entities exist apart from their element, their node document is not re-homed |
| replace's source-removal record siblings | O-4 (a) / proposed slot (§8 R-6 (b)) | P-3: `previousSibling` / `nextSibling` of the source-removal record follow an older replace order |
| Selection API document-membership check | Q-S6 / proposed slot (§8 R-6 (c)) | `same_document` accepts a detached node whose node document is the selection's document |
| VM `isConnected` vs bound document | O-5 / proposed slot (§8 R-6 (d)) | P-6 |
| HTML `img` adopting steps (relevant mutations) | not tracked here (elidex has no *update the image data*: `/usr/bin/grep -rln 'relevant mutation\|update_image_data' crates` → no hit) | an `img` adopted into another document does not re-run image selection; `MutationEvent::Adopt` is where it would hook |

Own deferrals: **0**. Every row above is pre-existing (A96 class) and is not changed by C0, except that C0 adds the
hook the last row names.

## §8. Edges, hand-offs and records

**Edges.** c0a: N1 implementation → C0 (clause (b) is edited on N1's landed body; T8–T10 need N1's compile gate).
c0b: C0 → C1 (semantic precondition, C1 draft §1). c0c: C0 → T3 (T3 reads `node_document`; H-T). c0d: C0 → L3's
P1 (boundary §2.0).

**Hand-offs.**
- **H-C1 to C1**: §4.4's table; C1's D-C5 edits `adopt_subtree`'s template edge only; C1's R-1 adds its own §1.2 row
  (C0's row is D-9's); C1's R-4 text (ledger "Trigger hit … PM decision requested") is superseded by R-1 below.
- **H-T to T3 (rev 3, lands after C1)**: D-T4's extraction of `effective_node_document` is not needed — step 3 reads
  `EcsDom::node_document` (the composed-root proxy no longer exists, D-7 (a)); T3's residual R-B (XW-9, EW-15, EW-16,
  and XW-10's HEAD verdict) is re-derived on C0's facts (T13): XW-9 → the Window (spec), XW-10 → nothing (spec),
  EW-15 → the Window with the getter reading the attribute (spec), EW-16 → nothing (spec); pin T-18 becomes a spec
  cell. T3's §8.3 item 3 (a consumer entry on this slot) is moot: the slot is closed.
- **H-P1 to L3's P1 umbrella** (via the PM): §2.0's contract.
- **H-N4 to N4**: edge n3's "promotes the effective-node-document rule into `EcsDom::effective_node_document`" is
  done by C0 as `EcsDom::node_document` (O-1, R-4).

**Records (for the PM; exact texts).**
- **R-1 ledger `#11-cross-document-adopt-on-insert` — at C0's landing: close**: "Closed by C0 (#<PR>, `<squash>`):
  every insertion adopts (DOM §4.2.3 *insert* step 7.1, *replace* step 6) through `EcsDom::adopt` at the three
  linking primitives; `AssociatedDocument` is the node document, read only by `EcsDom::node_document`; the
  composed-tree-root proxy in N1's compile gate and `owner_document`'s tree-root fallbacks are deleted. The listed
  facets (adopt-then-remove, live node into a detached foreign subtree, their reverses) and the consumer list
  (selection.rs → not a node-document reader, see `<slot R-6 (c)>`; document_base.rs base URL; ownerDocument getter;
  href/baseURI; N1's compile gate; the getter facet NR3) read the adopted document. Residuals homed: `<R-6 slots>`."
- **R-2 ledger / roadmap `#11-custom-elements-adopted-callback` — at C0's landing: close**: "Closed by C0 (#<PR>,
  `<squash>`): `MutationEvent::Adopt` → `CustomElementReaction::Adopted` → `adoptedCallback(oldDocument, document)`."
- **R-3 umbrella, in place (D-9; rev 8.4, carried by the PR that lands this memo)**: §1.2 row "| **C0**
  `EH-attrs-at-creation-C0` (owner: Layout lane, this program; PM decision 143) | slot
  `#11-cross-document-adopt-on-insert`: insertion adoption (DOM §4.2.3 *insert* step 7.1, §4.5 *adopt*); the node
  document is the stored fact, read by `EcsDom::node_document` | N1's implementation landed |"; §1.3 edge row "| x7 |
  C0 → C1 | semantic precondition | without insertion adoption, C1's inert-document contents keep the inert document
  after a clone is appended into the document (C1 memo §1) |"; the order line becomes "approval PR → slice a → IBP-sandbox
  N1 → C0 → C1 → T3 → slice b (PM decision 153)"; a changelog entry.
- **R-4 node memo, in place (if O-1 (a))**: D8's "Promoted unchanged into `EcsDom` as `effective_node_document`" →
  "Replaced by C0 (EH-attrs program) with the stored node document, `EcsDom::node_document`: a Document → itself, a node
  → its `AssociatedDocument`; unresolvable → the node clause is false (unchanged)"; edge n3's "promotes the
  effective-node-document rule into `EcsDom::effective_node_document`" → "reads `EcsDom::node_document` (C0)".
- **R-5 N1 memo**: none in place (it is landed); NR3's facet closes with R-1.
- **R-6 proposed slot texts (defects only; PM registers)**: (a) `#11-ownerless-node-creation` — "Engine paths create
  DOM nodes without a node document (`create_element` / `create_text` / … without an owner); the `ownerDocument`
  getter reports the bound document for them (`node_methods/core.rs:279`), and their first adoption is silent (no
  `adoptedCallback`). Spec: every node has a node document set upon creation (DOM §4.4)." (b)
  `#11-replace-source-removal-record-siblings` (unless O-4 (a)) — P-3's text. (c)
  `#11-selection-document-membership` — "Selection methods accept a node whose node document is the selection's
  document even when it is detached (`selection.rs:614-619`); the Selection API's check is a tree relation." (d)
  `#11-vm-isconnected-bound-document` — P-6's text.

## §9. Definition of done

1. Coordinates re-measured at C0's base (N1's implementation landed); §5.1 populations re-run, changes recorded in
   the PR.
2. `node_document`, `adopt`, `MutationEvent::Adopt` exist with §4.6's citations; `owner_document`'s fallbacks are gone
   (P-4's first command returns the definition and `node_proto.rs` only; P-5's two commands return 0).
3. D-4 to D-8 implemented; P-1's commands unchanged (no new linking path).
4. §6.1 and §6.2 cells present; every red cell run red on the base with its control green, recorded in the PR.
5. §5.2 rewrites done; §4.7's comment sites rewritten.
6. `cargo test` for the crates of §6.4 and `mise run ci` green.
7. Parse cost measured (§4.3, R-7) and recorded in the PR.
8. `python3 .claude/skills/elidex-plan-review/preflight.py docs/plans/2026-10-eh-attrs-c0-insertion-adoption.md`
   exits 0 (run; do not trust a stored verdict).
9. **T3's residual R-B is closed by C0** (PM decision 153): its cause, a stale node document after a cross-document
   move followed by a detached state, no longer exists; T13 pins the node-document facts of both X1 and X2 shapes at
   C0's head, and H-T hands T3 rev 3 the re-derived verdicts of XW-9, XW-10, EW-15, EW-16 and pin T-18 (T3 lands after
   C0 and C1 and owns those target cells; at C0's head HEAD's `resolve_handler_target`, H-14, does not read the node
   document, so no target cell exists to flip here).
10. The PM has applied R-1 to R-4 and registered or declined R-6 (a)–(d).

## §10. Open questions

**For the PM / user.**
- **O-1 N4's promotion.** (a) C0's `EcsDom::node_document` subsumes the node memo's planned
  `EcsDom::effective_node_document`, and C0's PR amends D8 and edge n3 in place (R-4; recommended: one function for one
  fact); (b) C0 keeps a second name for N4 to add (rejected: two names, one concept).
- **O-2 `document.adoptNode`.** (a) C0 adds it (steps 1–4 over `EcsDom::adopt`; VM marshalling only; cells for
  NotSupportedError / HierarchyRequestError / move) — recommended: it is the adopt algorithm's own entry and is
  reachable today with DOMParser; `importNode` (clone-based) stays out; (b) out of C0, slot registered (a B1 note
  already plans "wire `adoptNode` onto `adopt_subtree`", `project_world-id-deferral-contract-next.md:33`).
- **O-3 Vehicle for R-3 (D-9).** The PR that lands this memo carries the umbrella rows (N1's precedent, PM decision
  93), or the PM records them.
- **O-4 (a) P-3's record drift**: fold into C0 (same algorithm, one statement of replace's order) or slot R-6 (b);
  **(b)** if `mutation/mod.rs` reaches 1000 lines at the base, a standalone pure-move split first.
- **O-5 P-6**: register R-6 (d) or hand it to the owner of the VM `Node` surface.

**For plan-review.**
- **Q-1** `MutationEvent::Adopt` per adopted root (D-6) versus per descendant (DOM's "Run the adopting steps with
  inclusiveDescendant"): the root form needs every consumer to walk; the descendant form fires N events per move.
- **Q-2** Whether `adopt_subtree` should stay a separate `pub` function once `adopt` exists (D-3 / D-8 keep it for
  the fragment parse's early re-home).

**Scout questions** (facts this memo needs; not read here).
- **Q-S1** Are any `match` statements over `MutationEvent` exhaustive (no `_` arm), in P-6's 11 files and in tests?
- **Q-S2** Production ownerless creations (P-8) that a script can reach (a returned node, a wrapper): list them; in
  particular, can a script obtain a custom element whose node document is `None` (A4×A8)?
- **Q-S3** Does elidex model a per-element or per-shadow-root custom-element registry (adopt steps 3.2 / 3.3.2)?
- **Q-S4** Do `Attr` entities (`create_attribute`, `crates/core/elidex-ecs/src/dom/mod.rs:638`) exist apart from
  their element, and what does `attr.ownerDocument` return today?
- **Q-S5** Do `outerHTML` and `setHTMLUnsafe` place nodes through the three primitives (C0-1)?
- **Q-S6** The Selection API's exact document check for `collapse` / `setBaseAndExtent` / `extend` / `selectAllChildren`
  / `containsNode`: `webref body selection-api …` fails (anchors not in the rendered chapter); obtain the step text.
- **Q-S7** Is a DOMParser document ever destroyed (stale stamps on nodes moved out of it are re-homed by C0, but nodes
  left detached would read `None`)?
- **Q-S8** At C0's base, every test asserting non-adoption (§5.2).
- **Q-S9** What `owner_document` returns for a shadow root at HEAD (T4's first HEAD value).
- **Q-S10** The exact HEAD value of T12's `a.href` (the DOMParser document's base URL source).
