# Tree model `tsel` — Selection operations check the spec's tree preconditions (per-PR plan-memo)

Slice **`tsel`** of the umbrella
[2026-11-tree-model-shadow-root-is-a-root-umbrella.md](2026-11-tree-model-shadow-root-is-a-root-umbrella.md)
(order tp → **tsel** → tq → ta, PM 884; scope PM 885, widened by PM 890). Slot
`#11-selection-boundary-setter-skips-root-check` (registered 2026-11-21, PM 884; owner L3 DOM/form); the older slot
`#11-selection-document-membership` (registered 2026-11-05) is folded into it and closes with it (PM 890). Base:
`origin/main` `3cbd5b88`; every code fact below was measured there on 2026-11-21. Line numbers move, the commands
do not. Owner lane: L3 DOM/form. Security-relevant: closes the silent production paths by which the document's
Selection mutates shadow-tree content through `deleteFromDocument` (probes 1e, 4a).

Stated in the umbrella and only referred to here: the tree model (§1), the census (§6), the `tq` slice and its walk
(§1.6, §5 `tq`). This memo adds no rule to the umbrella; its umbrella edits are the two lines of §0.4.

## §0. Scope

### §0.1 In scope

The engine-independent Selection state machine, `SelectionState` in `crates/dom/elidex-dom-api/src/selection.rs`,
and the marshalling points in the VM that carry its result (§4.2). Facets, each with its own cells (§7):

1. **The root check.** `extend()` step 5 and `setBaseAndExtent()` step 5 (through DOM *set the start or end of a
   range to a boundary point* step 4) collapse a cross-root pair. Today `extend` (`selection.rs:329`, raw writes at
   `:357-361`) and `set_base_and_extent` (`:381`, raw writes at `:408-412`) write `Range::set_start` / `set_end`
   (`range/boundary.rs:35`, `:42`), which skip step 4. They take the existing `Range::set_start_to_boundary` /
   `set_end_to_boundary` (`range/boundary.rs:52`, `:67`; one issue, one way), and `extend` gains its explicit
   step 5 branch (§4.1: the setters alone collapse a backward cross-root extend at the wrong point, probe 1d).
   `collapse` (`:260`, raw at `:278-279`) also "set the start"s per its step 6, so it takes the same setters (no
   observable change: one point, one root).
2. **"Root is the document."** `selectAllChildren()` step 2 ("If node's root is not the document associated with
   this, abort these steps") and its sibling on the same seam, `addRange()` step 1 ("If the root of the range's
   boundary points are not the document associated with this, abort these steps"). Today `select_all_children`
   (`:424`) checks the node document only and `add_range` (`:455`) compares `Range::owner_document` only, so a node
   or a Range inside a shadow tree, or detached, is accepted (probes 2a-2f). `addRange` is not in PM 885's list; it
   is added here because it is a boundary setter with the same precondition (§0.5, §11 P-10).
3. **The shadow-including document check, with its step order and its abort.** `collapse()` step 4, `extend()`
   step 1, `setBaseAndExtent()` step 2: "If the document associated with this is not a shadow-including inclusive
   ancestor of node, abort these steps." Today `same_document` (`:613-618`) compares the node document, so a
   detached node is accepted (probes 3a, 3c, 3e), and a node of another document is rejected with
   `SelectionError::WrongDocument` (`:125`), which the native throws as "WrongDocumentError" (probe 3h). The spec
   aborts: no exception, no state change, no `selectionchange`. The check also moves to its spec position in each
   method's step order (§4.1), which changes which exception wins (probes 3f, 3h, D1, D3, D5). This facet is the
   defect of the folded slot `#11-selection-document-membership` (§8 L-5).
4. **`deleteFromDocument()`'s document-tree precondition** (PM 890): "The method must invoke deleteContents() on
   this's range if this is not empty and both focus and anchor are in the document tree. Otherwise the method must
   do nothing." Today `SelectionState::delete_from_document` (`:513-546`) checks emptiness only (`:518`) and its
   native (`delete_selection_contents`, `dom_selection_mutation.rs:121-149`) checks nothing and always schedules
   `selectionchange` (`:147`), so a selection lying wholly inside a shadow tree, which facet (3) keeps legal,
   deletes shadow-tree content (probe 4a: `st` "SHADOW" → "DOW"; spec: nothing).

### §0.2 Out of scope (and whose it is)

- **The Range walk.** `next_in_preorder_global` (`range/mod.rs`), `Range::to_string` and `Range::delete_contents`
  (`range/mutation.rs`) are not touched: they are `tq`'s (umbrella §5 `tq`). `tq`'s cross-root Selection
  `toString` cell expects `""` on `tsel`'s base (§2 TQ × SEL).
- **`containsNode()`'s root clause** ("return false if this is empty or if node's root is not the document
  associated with this"). The same predicate as facet (2) on a read path. Defect at base: with `L` selected,
  `containsNode(st)` is `true` for a Text inside a closed shadow tree under `L` (probe N2; spec `false`). Q-2 asks
  whether it joins this slice; until then `same_document` keeps this one caller (§4.1).
- **`getRangeAt()`'s document-tree clause** ("throw an IndexSizeError exception if index is not 0, or if this is
  empty or either focus or anchor is not in the document tree"). The same predicate as facet (4), on a read path
  not named by PM 890. Defect at base: `getRangeAt(0)` returns a range lying in a shadow tree (probe N5; spec
  "IndexSizeError"). §12 Q-3.
- **Selection direction.** `selectAllChildren()` step 7 sets *forwards*; `select_all_children` sets
  `Directionless` (probe N4: `"directionless"`, spec `"forward"`). The `direction` getter returns `"none"` only if
  the selection is empty or *directionless*; `current_direction` (`:204`) returns `Directionless` for every
  collapsed range. Not a precondition; §12 Q-3. The existing cell `select_all_children_makes_range` pins today's
  value and is left as it is (§7.2).
- **`selectAllChildren()`'s end offset.** Step 3 takes "the number of children of node"; `select_all_children`
  takes `node_length` (`:437`), which is a Text node's data length (read from the code, not probed). §12 Q-3.
- **`collapse(null)`.** Step 1 behaves as `removeAllRanges()`; the native's `arg_node_required`
  (`dom_selection_proto.rs:385`) throws "TypeError" (probe N1). WebIDL nullable conversion in the VM, not a
  precondition of `SelectionState`. §12 Q-3.
- **`selectionchange` for a `deleteFromDocument` that runs `deleteContents()` on a collapsed range** (DOM step 1
  returns, so no boundary point is mutated): today the native schedules the event; `tsel` keeps that for the
  branch that does invoke `deleteContents()` and drops it only for the "do nothing" branch (§4.2). §12 Q-3.
- **Cross-root order.** `compare_points` (`range/mod.rs:508`) answers a cross-root query in shadow-including order
  (it climbs through `is_ancestor_or_self` and `tree_order_cmp`); its relation is `tr`'s and `tc`'s (umbrella §5).
  `tsel` adds no cross-root order query; the two it inherits are Q-1.
- **`insertNode`'s transient Range** (`range_proto_mutation.rs:158-160`, raw writes on one node): one point, one
  root, not a Selection write.

### §0.3 The declared behaviour change

One: **Selection operations check the spec's tree preconditions** (PM 890; PM 885's "Selection's boundary setters
apply the spec's preconditions" plus `deleteFromDocument()`'s). Observable results, all from the Selection API §3
and DOM text quoted in §3:

1. A cross-root `setBaseAndExtent` or `extend` leaves a collapsed selection: `toString()` is `""`, `isCollapsed`
   is `true`, and `deleteFromDocument()` removes nothing. After `extend` the caret is at the new focus (step 5).
2. `selectAllChildren(n)` and `addRange(r)` with `n` / `r` inside a shadow tree or detached do nothing.
3. `collapse` / `setPosition`, `extend` and `setBaseAndExtent` with a node that is not in the document,
   shadow-including (detached, or another document's), do nothing and throw nothing. The exceptions that remain
   come in the spec's step order (§4.1).
4. `deleteFromDocument()` does nothing unless both anchor and focus are in the document tree (a node tree whose
   root is a document): a selection inside a shadow tree is kept and nothing is deleted.
5. An operation that does nothing schedules no `selectionchange` (the event is scheduled when "the selection is
   dissociated with its range, associated with a new range, or the associated range's boundary point is mutated";
   an abort does none of these).

### §0.4 Ratified-surface items (reviewed in this plan's plan-review)

The plan PR edits the umbrella in two lines and nowhere else. Both are reviewed here as ratified-surface edits.

- §5.1, after the `tq → ta` bullet (verbatim):

  > - tsel → tq (erratum, tsel plan PR, PM 884): the order is tp → tsel → tq → ta; `tsel` ([2026-11-tree-model-tsel-selection-boundary-preconditions.md](2026-11-tree-model-tsel-selection-boundary-preconditions.md), slot `#11-selection-boundary-setter-skips-root-check`) makes Selection operations check the spec's tree preconditions (PM 890), and `tq`'s cross-root Selection `toString` cell expects `""` on that base.

- A new `**Changelog.**` line directly after the Status block (verbatim; the umbrella had no changelog line, its
  revision record lives in the Status block, which is unchanged, §11 P-6):

  > **Changelog.** erratum §5 (slice order: tsel before tq), no rev bump, PM 884.

Not edited, and superseded by the erratum line: §5.1's bullet "`tq` first among the behaviour slices" and the §5
table's `tq` row, Deps column ("— (it lands before `ta`, PM 820; no code dependency on `tp`)"). `tsel` has no row in
the §5 table; the erratum line is its record there.

### §0.5 Lens block (basis order: spec → elidex principle → evidence)

The choice not fixed by PM 884 / 885 / 890: whether `addRange()` step 1 rides in this slice.

| Principle | Include `addRange` | Leave it |
|---|---|---|
| Spec | both are spec text on one seam | — |
| One issue, one way | **include**: the "root is the document" predicate gets one home (`root_is_document`, NEW); leaving `add_range` on `owner_document` keeps a second predicate for the same spec phrase | rejects |
| Semantic siblings in one commit (PM 885's own ground) | **include** | rejects |
| One declared change | **include**: `addRange` is a Selection operation with a tree precondition | — |
| Ideal over pragmatic | **include** | rejects |
| Security by structure | **include**: a shadow-tree Range added to the document's Selection is the input facet (4) refuses | — |

Every row selects "include", so it is included and reported (§11 P-10). `containsNode` and `getRangeAt` are reads
outside PM 890's list, so the one-declared-change row splits for them; that is Q-2 / Q-3.

## §1. Shape and verdict

One PR. `elidex-dom-api` `selection.rs`: four setters reordered and rewritten onto the spec's setters and two
predicates (NEW, private), `add_range` reads the range's root, `delete_from_document` checks the document tree and
says whether it ran, the `WrongDocument` variant is deleted. `elidex-js`: `mutate_selection` and
`delete_selection_contents` schedule `selectionchange` only when the operation did something, `addRange`'s native
passes the registry instead of reading `owner_document`, `map_selection_error` loses its `WrongDocument` arm. No
algorithm moves into `vm/host/`. Tests: new cells in `selection_tests.rs` and a new VM test file; the VM selection
fixture attaches its tree to the document (§7.2). The slice meets the Edge-dense trigger (≥3 intersecting axes,
§2), so it has this per-PR plan and a full `/elidex-plan-review` before code (§9).

## §2. Coupled invariants (edge-matrix)

Axes: **SEL** (Selection state: `active_range_id`, `direction`), **LRR** (`LiveRangeRegistry`: registration, the
displaced-id unregister in `mutate_selection`, live-range adjustment), **ROOT** (tree root: `find_tree_root`,
`walkers.rs:408`, stops at the `ShadowRoot` marker), **DOC** (document association: shadow-including root,
`find_tree_root_composed`, `walkers.rs:432`; node document, `owner_document`, `dom/mod.rs:881`), **ERR** (exception
vs abort, the native's mapping, `selectionchange`), **MUT** (`deleteFromDocument` → `Range::delete_contents` and
its `MutationRecord`s), **ORD** (boundary-point order across roots), **TQ** (`tq`'s walk).

| Pair | Invariant after `tsel` | Cells |
|---|---|---|
| SEL × ROOT | every range the four setters and `addRange` give the Selection has one root (start's root = end's root) | E1, E2, J1, J2 |
| SEL × DOC | after `collapse` / `extend` / `setBaseAndExtent` the range's shadow-including root is the associated document (a shadow tree of the document is allowed: the spec's note, "anchor and focus … could be in a shadow tree of the same document"); after `selectAllChildren` / `addRange` its root is the document itself | E3, E4, E5, E6, J4, J5, J6, J9 |
| DOC × MUT | `deleteFromDocument` reaches `delete_contents` only when the anchor's and the focus's root is the document; so a Selection never deletes inside a shadow tree, whichever setter made it. With SEL × ROOT, "both" reduces to the start container's root, but the check is written on both points as the spec says | E8, J10 |
| SEL × ERR | an abort returns `Ok(false)` and leaves `active_range_id` and `direction` bit-identical; an exception does too (unchanged rule, `collapse`'s doc comment) | E5, E7, J6 |
| ERR × event | an operation that does nothing schedules no `selectionchange`; one that sets a range, or runs `deleteContents()`, schedules one, as today | J7, J10 |
| LRR × ERR | an abort registers no Range and unregisters none (`prev_id == new_id` in `mutate_selection`) | E5 (registry untouched), J6 |
| LRR × ROOT | live-range adjustment cannot make a one-root range cross-root: DOM's removing steps move a boundary inside the removed node to (parent, index), in the same tree, and inserting steps move offsets within the same parent. So the one-root invariant set at write time holds until the next write. Reasoning, not a cell; `extend` reads "this's range's root" from the start container on that basis | — |
| LRR × DOC (facet 4) | a selection that was in the document tree can leave it only with its tree: removing the subtree that holds a boundary moves the boundary to the removed node's parent, which stays in the document. So facet (4)'s "do nothing" arises from a selection set inside a shadow tree, not from a later removal | E8 |
| DOC × ROOT | the two predicates are storage-neutral: `find_tree_root_composed` reads the stored parent first and the `.host` link second, and at base a shadow root's stored parent is its host, so both reach the host; after `ts` the `.host` branch does it. `find_tree_root` stops at the `ShadowRoot` marker at every head | E6, J9 (they hold unchanged through `ts`) |
| ROOT × ORD | DOM's boundary-point position asserts one root (§3); `setBaseAndExtent()` step 5 and `extend()` step 9 ask "before" of a pair that can be cross-root. `tsel` asks no new cross-root order question: it keeps `direction_between` (`:624`) as the branch selector, so a cross-root `setBaseAndExtent` collapses at whichever point the branch writes last. Not asserted by any cell (Q-1) | — |
| TQ × SEL | `tsel` touches no file of `tq`'s. After `tsel`, a cross-root Selection is collapsed, so its `toString` is `""` with or without `tq`'s walk; that is the value `tq`'s cell expects (umbrella §5.1 erratum). `tq`'s other Selection cells use `addRange` of a Range built in `body` (the `tq` plan §6), whose root is the document, so `addRange`'s new check and facet (4) accept them | J1 (on both bases) |
| ta × tsel | `tsel` adds callers of two relations `ta` handles: `find_tree_root` (kept, census C-6 "K") and `find_tree_root_composed` (renamed "RN"; 2 callers, verified 2026-11-21 at `3cbd5b88`: `git grep -n -P '\bfind_tree_root_composed\(' -- crates ':!*tests*'` → `consumer.rs:408`, `node_proto.rs:455`). `ta`'s rename is compiler-driven, so the new caller follows; the census counts change (§8 L-3) | — |

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| SELECTION API §3 Selection interface | `collapse()` steps 2-6: "If node is a DocumentType, throw an InvalidNodeTypeError exception" / "throw an IndexSizeError exception if offset is longer than node's length" / "If document associated with this is not a shadow-including inclusive ancestor of node, abort these steps." / "Set the start the start and the end of newRange to (node, offset)." | DocumentType → throw; offset > length → throw; not in the document (shadow-including) → abort; else set | `SelectionState::collapse` (`selection.rs:260`) | ✗ (step 1, node null → `removeAllRanges()`: VM nullable conversion, §0.2) | yes |
| SELECTION API §3 Selection interface | `extend()` steps 1, 2, 5-7: "If the document associated with this is not a shadow-including inclusive ancestor of node, abort these steps." / "If this is empty, throw an InvalidStateError exception" / "If node's root is not the same as the this's range's root, set the start newRange's start and end to newFocus." / "Otherwise, if oldAnchor is before or equal to newFocus, set the start newRange's start to oldAnchor, then set its end to newFocus." / "Otherwise, set the start newRange's start to newFocus, then set its end to oldAnchor." | not in the document → abort; empty → throw; cross-root → collapse at newFocus; same root × forward / backward | `SelectionState::extend` (`selection.rs:329`) | ✗ (step 9, direction: §0.2 and Q-1) | yes |
| SELECTION API §3 Selection interface | `setBaseAndExtent()` steps 1, 2, 5: "If anchorOffset is longer than anchorNode's length or if focusOffset is longer than focusNode's length, throw an IndexSizeError exception" / "If document associated with this is not a shadow-including inclusive ancestor of anchorNode or focusNode, abort these steps." / "If anchor is before focus, set the start the newRange's start to anchor and its end to focus. Otherwise, set the start them to focus and anchor respectively." | offset → throw; either node not in the document → abort; branch × set the start / end (DocumentType throws there, DOM step 1) | `SelectionState::set_base_and_extent` (`selection.rs:381`) | ✗ (step 7, direction: §0.2; step 5's "before" across roots: Q-1) | yes |
| SELECTION API §3 Selection interface | `selectAllChildren()` steps 1, 2, 4, 5: "If node is a DocumentType, throw an InvalidNodeTypeError exception" / "If node's root is not the document associated with this, abort these steps." / "Set newRange's start to (node, 0)." / "Set newRange's end to (node, childCount)." | DocumentType → throw; root not the document → abort; else set (plain set, not "set the start") | `SelectionState::select_all_children` (`selection.rs:424`) | ✗ (step 3's childCount and step 7's direction: §0.2) | yes |
| SELECTION API §3 Selection interface | `addRange()` steps 1-3: "If the root of the range's boundary points are not the document associated with this, abort these steps." / "If rangeCount is not 0, abort these steps." / "Set this's range to range by a strong reference" | root not the document → abort; rangeCount ≠ 0 → abort; else set | `SelectionState::add_range` (`selection.rs:455`); native `native_selection_add_range` (`dom_selection_mutation.rs:155`) | ✓ | yes |
| SELECTION API §3 Selection interface | `deleteFromDocument()`: "The method must invoke deleteContents() on this's range if this is not empty and both focus and anchor are in the document tree. Otherwise the method must do nothing." | empty → nothing; anchor or focus not in the document tree → nothing; else `deleteContents()` | `SelectionState::delete_from_document` (`selection.rs:513`); native `delete_selection_contents` (`dom_selection_mutation.rs:121`) | ✓ | yes |
| SELECTION API §2 Definition | *empty*: "When there is no range associated with the selection, the selection is empty"; *anchor* / *focus* note: "anchor and focus of selection need not to be in the document tree. It could be in a shadow tree of the same document." | empty / not empty; shadow tree of the document allowed | `SelectionState::is_empty`, `anchor`, `focus` (`selection.rs:171`, `:223`, `:241`) | ✓ | no |
| SELECTION API §3 Selection interface | `selectionchange` event: "When the selection is dissociated with its range, associated with a new range, or the associated range's boundary point is mutated either by the user or the content script, the user agent must schedule a selectionchange event on document." | nothing done → no event; set / deleted → event | `mutate_selection` (`dom_selection_mutation.rs:55`, the pending flag at `:111`); `delete_selection_contents` (`:147`) | ✗ (input / textarea selection: not this slice; the collapsed-`deleteContents` branch: §0.2) | yes |
| WHATWG DOM §5.6 Interface Range | *set the start or end of a range to a boundary point* steps 1, 2, 4: "If node is a doctype, then throw an "InvalidNodeTypeError" DOMException." / "If offset is greater than node's length, then throw an "IndexSizeError" DOMException." / set the start: "If range's root is not node's root or boundaryPoint is after range's end, then set range's end to boundaryPoint." / set the end: "If range's root is not node's root or boundaryPoint is before range's start, then set range's start to boundaryPoint." | doctype / offset → throw; cross-root or crossing → collapse onto the new point | `Range::set_start_to_boundary`, `Range::set_end_to_boundary` (`range/boundary.rs:52`, `:67`; steps 1-2 are run by the caller, as their doc comment says) | ✓ | yes |
| WHATWG DOM §5.2 Boundary points | *position* step 1: "Assert: nodeA and nodeB have the same root." | same root → before / equal / after; cross-root → not defined | `compare_points` (`range/mod.rs:508`) through `direction_between` (`selection.rs:624`) | ✗ (the cross-root branch is a spec gap: Q-1) | yes |
| WHATWG DOM §5.6 Interface Range | `deleteContents()` step 1: "If this is collapsed, then return." | collapsed → nothing | `Range::delete_contents` (`range/mutation.rs`, reached from `delete_from_document`, `selection.rs:533`) | ✗ (only the collapsed branch is a `tsel` cell; the rest is `tq`'s) | yes |
| WHATWG DOM §4.2.1 Document tree | *document tree*: "A document tree is a node tree whose root is a document." | a boundary node's root is the document → in the document tree; a shadow root or a detached root → not | `root_is_document` (NEW) in `delete_from_document` | ✓ | yes |
| WHATWG DOM §4.8 Interface ShadowRoot | *shadow-including inclusive ancestor* (with *shadow-including root*): an object that is the node or one of its shadow-including ancestors (parent, or a shadow root's host) | the document is one ⇔ the node's shadow-including root is the document (a document has no parent and no host) | `in_document_shadow_including` (NEW) over `EcsDom::find_tree_root_composed` (`walkers.rs:432`) | ✓ | yes |
| WHATWG DOM §1.1 Trees | *root*: "The root of an object is itself, if its parent is null, or else it is the root of its parent." | the root of a shadow-tree node is its shadow root, never the document | `root_is_document` (NEW) over `EcsDom::find_tree_root` (`walkers.rs:408`) | ✓ | yes |

Lookups (2026-11-21, `.claude/tools/webref`): `dfn selection-api "collapse()"` / `"extend()"` /
`"setBaseAndExtent()"` / `"selectAllChildren()"` / `"addRange()"` / `"containsNode()"` / `"deleteFromDocument()"`
→ §3 Selection interface (`#dom-selection-collapse`, `#dom-selection-extend`, `#dom-selection-setbaseandextent`,
`#dom-selection-selectallchildren`, `#dom-selection-addrange`, `#dom-selection-containsnode`,
`#dom-selection-deletefromdocument`); `dfn selection-api empty` → §2 Definition `#dfn-empty`; `dfn selection-api
direction` → §2 `#dfn-direction` and §3 `#dom-selection-direction`; `heading selection-api 3` → §3 Selection
interface. `dfn dom "set the start"` → §5.6 Interface Range `#concept-range-bp-set`, prose from `body dom
concept-range-bp-set`; `dfn dom before` (for boundary point) → §5.2 `#concept-range-bp-before`, prose from `body
dom concept-range-bp-position`; `body dom dom-range-deletecontents` → §5.6; `dfn dom "document tree"` → §4.2.1
`#concept-document-tree`, prose from `body dom concept-document-tree`; `dfn dom "shadow-including inclusive
ancestor"` → §4.8 `#concept-shadow-including-inclusive-ancestor`; `dfn dom "shadow-including root"` → §4.8; `dfn
dom root` (for tree) → §1.1 `#concept-tree-root`. `coverage-map dom 5.6 dom 4.8 dom 5.2 selection-api 3
selection-api 2` produced the starter rows. **The Selection API quotes are not from `webref body`**: `webref body
selection-api` cannot render the ReSpec source, so they are quoted from webref's cache file
`~/.cache/elidex-webref/c0d97c3dfdf1cb8a59fa73f678d007b67d8cef81` (the ReSpec HTML; `<dfn>collapse()</dfn>` at its
line 458, `extend()` 530, `setBaseAndExtent()` 585, `selectAllChildren()` 624, `addRange()` 334,
`deleteFromDocument()` 758, the definitions 95-180, `selectionchange` 1040). No WebFetch.

**Breadth**: K=2 specs (DOM, Selection API), M=14 rows. The Selection API rows are "unmapped-label" for preflight
(its label table has no Selection API entry); their citations are the `dfn selection-api` lookups above.
**Split decision**: single PR (a terminal slice of the umbrella).

### §3.1 User-input touch audit

All inputs are author-controlled: the node and offset arguments of the four setters and the Range given to
`addRange` (built with `setStart` / `setEnd` / `selectNode`, any tree a script can make, closed shadow trees
included). Exposure decreases: after `tsel` no Selection range spans two trees, `deleteFromDocument` deletes only
when both anchor and focus are in the document tree, so the document's Selection can no longer reach shadow-tree
content (probes 1e and 4a: today `st` "SHADOW" → "DOW"), and a detached or foreign node can no longer become the
document's selection. What stays exposed is §0.2's: `getRangeAt(0)` still hands out a range lying in a shadow
tree (Q-3), through which a script that already holds the shadow tree's nodes can mutate it by Range methods. The
VM natives keep their argument conversions (`arg_node_required`, `arg_offset_*`, `arg_range`) unchanged; no new
input path.

## §4. Mechanism

### §4.1 `elidex-dom-api` — `selection.rs`

Two private predicates (NEW), each with its spec sentence as its doc comment:

```rust
/// DOM §4.8: the associated document is a shadow-including inclusive ancestor of `node`
/// ⇔ `node`'s shadow-including root is that document.
fn in_document_shadow_including(node: Entity, owner_document: Entity, dom: &EcsDom) -> bool {
    dom.find_tree_root_composed(node) == owner_document
}

/// DOM §1.1 / §4.2.1: `node`'s root is the associated document (`node` is in its document tree).
fn root_is_document(node: Entity, owner_document: Entity, dom: &EcsDom) -> bool {
    dom.find_tree_root(node) == owner_document
}
```

The four setters return `Result<bool, SelectionError>`: `Ok(true)` set a range, `Ok(false)` aborted (the
`add_range` convention, which returns `bool` for the same reason: the native decides on `selectionchange`). Step
order, each check before any state change:

| Method | New order (spec) | Base order |
|---|---|---|
| `collapse` | DocumentType → throw; offset → throw; `in_document_shadow_including` → abort; `set_start_to_boundary` + `set_end_to_boundary` (step 6) | DocumentType; `same_document` → `WrongDocument`; offset; raw writes |
| `extend` | `in_document_shadow_including` → abort; empty → `InvalidState`; DocumentType → throw; offset → throw (DOM set-the-start steps 1-2, run before step 5's writes); if `find_tree_root(node) != find_tree_root(range start)` → both points to newFocus (step 5); else the forward / backward branch through `set_*_to_boundary` | empty; DocumentType; `same_document`; offset; raw writes |
| `set_base_and_extent` | offsets → `IndexSize` (step 1); `in_document_shadow_including` for both → abort (step 2); DocumentType for either → throw (DOM step 1, inside step 5); branch on `direction_between` through `set_*_to_boundary` (step 5) | DocumentType; `same_document`; offsets; raw writes |
| `select_all_children` | DocumentType → throw; `root_is_document` → abort; plain writes (steps 4-5 say "Set", not "set the start") | DocumentType; `same_document`; raw writes |
| `add_range` | `root_is_document(start container of the registered range)` → abort; rangeCount ≠ 0 → abort; set | rangeCount; `owner_document` equality |
| `delete_from_document` | empty → nothing; `root_is_document` of the anchor's and the focus's node (`self.anchor`, `self.focus`) → else nothing; `delete_contents` and the write-back as today. Returns `Option<Vec<MutationRecord>>`: `None` = did nothing, `Some(records)` = ran `deleteContents()` | empty → `Vec::new()`; else delete |

`delete_from_document` gains the owner document as a parameter (it has none today). Why `extend` needs its own
step 5 branch: `set_*_to_boundary` collapse onto the point written last. In the forward branch that is newFocus,
which is step 5's answer; in the backward branch it is oldAnchor (probe 1d's shape, `collapse(post, 2); extend(st,
3)`, is backward because `compare_points` orders `st` before `post`). So the setters alone are not step 5; the
explicit root comparison is. `set_base_and_extent` has no such step: its cross-root branch is a spec gap (Q-1).

`SelectionError::WrongDocument` (`:125`) has no producer left and is deleted (CLAUDE.md: dead code is deleted).
`same_document` keeps one caller, `contains_node` (`:564`), until Q-2 is answered; if Q-2 folds `containsNode` in,
`same_document` is deleted and `contains_node` calls `root_is_document`. The module doc comment's "validity gates"
paragraph and the doc comments of `collapse`, `set_base_and_extent` and `delete_from_document` are rewritten to the
new order. `direction_between` and `node_fully_contained` keep their raw writes: they build temporary comparison
ranges, not Selection ranges.

### §4.2 `elidex-js` — marshalling only

- `mutate_selection` (`dom_selection_mutation.rs:55`): the closure returns `Result<bool, SelectionError>`; the
  function returns `Result<(), VmError>`; `selectionchange_pending` is set only on `Ok(true)` (`:111`). Its nine
  callers (`git grep -n "mutate_selection(ctx" crates/script/elidex-js/src/vm/host/dom_selection_mutation.rs`:
  `:196` removeRange, `:208` removeAllRanges, `:221` empty, `:237` collapse, `:249` collapseToStart, `:261`
  collapseToEnd, `:275` extend, `:293` setBaseAndExtent, `:306` selectAllChildren) pass the setter's `bool`, or
  `Ok(true)` where the method has no abort (behaviour unchanged for those five).
- `delete_selection_contents` (`:121`): passes the document to `delete_from_document`; on `None` it neither commits
  records nor sets `selectionchange_pending` (`:146-147`); on `Some(records)` it does both, as today.
- `native_selection_add_range` (`:155`): stops reading `r.owner_document` (`:171-172`) and calls `add_range` with
  the registry, the DOM and the document; it still maps an unregistered id to the same "TypeError".
- `map_selection_error` (`dom_selection_proto.rs:304`): the `WrongDocument` arm (`:314`) is deleted with the
  variant. No other arm changes.

### §4.3 ECS-native / layering check

The predicates and step orders live in `elidex-dom-api` (Layering mandate); the VM changes are the "did something"
flags and two parameter lists. `SelectionState` stays a `RangeId` plus a direction (no new component, no
side-store). The predicates read existing `EcsDom` relations; no new tree accessor, so the umbrella's vocabulary
(§1) is unchanged.

## §5. Populations

### §5.1 Callers of each `SelectionState` method

Commands (at `3cbd5b88`; the definitions in `selection.rs` are the control hit of each):

```
git grep -n -w SelectionState -- crates ':!crates/dom/elidex-dom-api/src/selection.rs' ':!crates/dom/elidex-dom-api/src/selection_tests.rs'
for m in set_base_and_extent select_all_children add_range contains_node delete_from_document collapse_to_start collapse_to_end; do git grep -n -P "\b$m\(" -- crates; done
git grep -n -P "\.(collapse|extend)\((reg|registry|&mut reg)" -- crates
git grep -n -P "Selection(State|Direction|Type|Error)\b" -- crates ':!crates/dom/elidex-dom-api/src/selection*' ':!crates/script/elidex-js/src/vm'
```

| Method | Production caller (one each) | Test callers |
|---|---|---|
| `collapse` | `native_selection_collapse` (`dom_selection_mutation.rs:238`; `setPosition` is the same native) | `selection_tests.rs` (11 calls) |
| `extend` | `native_selection_extend` (`:276`) | `selection_tests.rs` (3) |
| `set_base_and_extent` | `native_selection_set_base_and_extent` (`:294`) | `selection_tests.rs` (2) |
| `select_all_children` | `native_selection_select_all_children` (`:307`) | `selection_tests.rs` (2) |
| `add_range` | `native_selection_add_range` (`:182`) | `selection_tests.rs` (3) |
| `delete_from_document` | `delete_selection_contents` (`:140`), from `native_selection_delete_from_document` (`:312`) | none in `selection_tests.rs`; VM: `tests_mutation_observer/range_ops.rs` (2) |
| `contains_node` (Q-2) | `dom_selection_proto.rs:638` | `selection_tests.rs` (4) |
| `collapse_to_start`, `collapse_to_end` (unchanged) | `dom_selection_mutation.rs:250`, `:262` | — |

**Non-script callers: none.** The only `SelectionState` users outside `selection.rs` are the VM's Selection natives,
its HostData slot (`host_data/mod.rs:425`, `bind.rs` borrow splits, `lifecycle.rs:62` reset) and the GC root walk
(`object_roots.rs:156`, `current_range_id` only). The fourth command finds only `lib.rs:120` (the re-export) and
`elidex-form-core`'s own `SelectionDirection` (the input / textarea selection, a different type). No editing, shell
or a11y code reads or writes the document Selection, so nothing depends on today's lax behaviour.

### §5.2 Files touched (`wc -l` at `3cbd5b88`)

`crates/dom/elidex-dom-api/src/selection.rs` 677; `crates/dom/elidex-dom-api/src/selection_tests.rs` 367;
`crates/script/elidex-js/src/vm/host/dom_selection_mutation.rs` 320;
`crates/script/elidex-js/src/vm/host/dom_selection_proto.rs` 748;
`crates/script/elidex-js/src/vm/tests/tests_selection.rs` 787 (fixture only); `crates/script/elidex-js/src/vm/tests/mod.rs`
215 (one `mod` line); `tests_selection_preconditions.rs` (NEW, in `crates/script/elidex-js/src/vm/tests/`).

## §6. Base probes (run at `3cbd5b88`, 2026-11-21)

Two temporary tests, both removed afterwards (`git status --short` empty): a VM test
`tests_tsel_probe.rs` (NEW: removed) in `crates/script/elidex-js/src/vm/tests/`, registered in that directory's
`mod.rs`, and a dom-api test appended to `selection_tests.rs` (reverted). Command, per run:

```
unset RUSTUP_TOOLCHAIN; rustc --version && cargo test -p elidex-js --all-features --lib tsel_probe -- --nocapture
unset RUSTUP_TOOLCHAIN; rustc --version && cargo test -p elidex-dom-api --all-features --lib tsel_probe -- --nocapture
# rustc 1.99.1 (58cf8f9c3 2026-10-13); one cargo job at a time
```

VM tree, fresh document per probe: `html` (document element) > `L` (div) > [`pre` Text "pre", `H` (div), `post` Text
"postTAIL"]; `sr = H.attachShadow({mode: "closed"})` > `se` (span) > `st` Text "SHADOW"; `det` (p, detached) > `dt`
Text "detached"; a foreign node is a `DOMParser` document's `<p>` and its Text ("foreign"). A `selectionchange`
listener counts events. Each row: the script, what base does, the spec value (§3).

| # | Script | Base | Spec |
|---|---|---|---|
| 1a | `setBaseAndExtent(pre,1, st,3)` | not collapsed, `toString` `"reSHA"`, event 1 | collapsed (DOM set-the-end step 4), `""` |
| 1b | `setBaseAndExtent(st,3, pre,1)` | not collapsed, `"reSHA"`, direction backward | collapsed, `""` |
| 1c | `collapse(pre,1); extend(st,3)` | not collapsed, `"reSHA"` | collapsed at (`st`, 3) (step 5), `""` |
| 1d | `collapse(post,2); extend(st,3)` | not collapsed, direction backward, `"DOWpo"` | collapsed at (`st`, 3), `""` |
| 1e | 1a, then `deleteFromDocument()` | `pre` → "p", `st` → "DOW" (shadow content deleted) | nothing removed |
| 1f | 1c, then `deleteFromDocument()` | `pre` → "p", `st` → "DOW" | nothing removed |
| 1g | `setBaseAndExtent(pre,1, post,2)` (control, one root) | set, `"reSHADOWpo"` (the walk overshoot, `tq`'s) | set (the string is `tq`'s) |
| 2a | `selectAllChildren(se)` | set, `"SHADOW"` | abort (root is `sr`) |
| 2b | `selectAllChildren(sr)` | set, `"SHADOW"` | abort |
| 2c | `selectAllChildren(det)` | set, `"detached"` | abort |
| 2d | `selectAllChildren(L)` (control) | set, `(L,0)-(L,3)` | set |
| 2e | `addRange(r)`, `r` = (`st`,0)-(`st`,2) | set, rangeCount 1 | abort |
| 2f | `addRange(r)`, `r` = (`dt`,0)-(`dt`,2) | set | abort |
| 3a | `collapse(dt,1)` | set, event 1 | abort, no event |
| 3b | `collapse(st,1)` (control) | set | set (shadow tree of the document) |
| 3c | `setBaseAndExtent(dt,0, dt,3)` | set, `"det"` | abort |
| 3d | `extend(dt,1)` on empty | "InvalidStateError" | abort (step 1 before step 2), no throw |
| 3e | `collapse(pre,1); extend(dt,1)` | set, `"reSHADOWpostTAILd"` | abort, caret stays at (`pre`,1) |
| 3f | `collapse(dt,99)` | "IndexSizeError" | "IndexSizeError" (step 3) |
| 3g | `setBaseAndExtent(dt,99, pre,0)` | "IndexSizeError" | "IndexSizeError" (step 1) |
| 3h | foreign node: `collapse(ft,1)`, `collapse(ft,99)`, `setBaseAndExtent(ft,1, pre,0)`, `selectAllChildren(fe)`, `extend(ft,1)` | every one "WrongDocumentError" | `(ft,99)`: "IndexSizeError" (step 3 before step 4); the other four abort without throwing |
| 3k | `collapse(sr,0)`; `collapse(document,0)` (control) | both set | both set |
| 4a | `setBaseAndExtent(st,0, st,3); deleteFromDocument()` (the selection lies wholly in the shadow tree) | `st` → "DOW" | nothing: anchor and focus are not in the document tree |
| N1 | `collapse(null)` | "TypeError" | `removeAllRanges()` (§0.2) |
| N2 | `selectAllChildren(L); containsNode(st)` | `true` | `false` (§0.2, Q-2) |
| N4 | `selectAllChildren(L); direction` | `"directionless"` | `"forward"` (§0.2) |
| N5 | `setBaseAndExtent(st,0, st,3); getRangeAt(0)` | returns | "IndexSizeError" (§0.2) |

dom-api probes (`fresh()` document `doc` > `p`; `own_dt` a doctype inserted in `doc`; `fdt` a doctype child of a
second document):

| # | Call | Base | Spec |
|---|---|---|---|
| D1 | `set_base_and_extent(p,0, fdt,0)` | `Err(InvalidNodeType)` | abort (step 2 before the DOM step 1 inside step 5) |
| D2 | `set_base_and_extent(p,0, own_dt,0)` | `Err(InvalidNodeType)` | `InvalidNodeType` |
| D3 | `set_base_and_extent(p,0, own_dt,5)` | `Err(InvalidNodeType)` | `IndexSize` (step 1) |
| D4 | `select_all_children(fdt)` | `Err(InvalidNodeType)` | `InvalidNodeType` (step 1) |
| D5 | `extend(fdt, 0)` on empty | `Err(InvalidState)` | abort (step 1) |
| D6 | `collapse(fdt, 0)` | `Err(InvalidNodeType)` | `InvalidNodeType` (step 2) |

The probe sources are kept outside the repository for the record (VM round 2:
`/private/tmp/claude-501/-Users-kazuaki-orca-workspaces-elidex-L3/dd0852cc-51ac-4464-8e63-719a384a6780/scratchpad/tests_tsel_probe_round2.rs`;
dom-api: `.../scratchpad/tsel_probe_domapi.diff`); the round-1 cases (1a-4a, N1-N5) are the rows above, on the same
fixture.

## §7. Cells and mutants

Every expected value is the spec's (§3, §6 "Spec" column). None is any engine's.

**Unit, `crates/dom/elidex-dom-api/src/selection_tests.rs`** (fixtures: `fresh()`; a host in `doc` with an
attached shadow root holding a Text; a detached element; a second document):

- E1 `set_base_and_extent_cross_root_collapses` (NEW): (light Text, 1, shadow Text, 3) and the reverse order →
  `Ok(true)`, `is_collapsed`, start root = end root. The collapse point is not asserted (Q-1).
- E2 `extend_cross_root_collapses_at_new_focus` (NEW): forward shape (caret before the host, extend into the shadow
  Text) and backward shape (caret after the host, probe 1d) → `anchor` = `focus` = (shadow Text, 3).
- E3 `select_all_children_outside_document_tree_aborts` (NEW): an element of the shadow tree, the shadow root, a
  detached element → `Ok(false)`; a prior collapsed selection is unchanged (`current_range_id`, anchor).
- E4 `add_range_not_rooted_at_document_aborts` (NEW): a registered Range in the shadow tree, a detached one, one in
  the second document → `false`, state unchanged; a Range in `doc` → `true` (control).
- E5 `setters_outside_document_abort` (NEW): `collapse`, `extend` (with a prior caret) and `set_base_and_extent`
  with a detached node and with a node of the second document → `Ok(false)`; `active_range_id` and the
  registry's entry for it unchanged.
- E6 `shadow_tree_boundary_is_accepted` (NEW): `collapse(shadow Text, 1)` → `Ok(true)`; `set_base_and_extent`
  within the shadow Text → `Ok(true)`, not collapsed (the spec's note, §3 row §2).
- E7 `setter_step_order` (NEW): D1 → `Ok(false)`; D2 → `Err(InvalidNodeType)`; D3 → `Err(IndexSize)`; D5 →
  `Ok(false)`; `collapse(foreign Text, 99)` → `Err(IndexSize)`; `extend(node in doc)` on empty →
  `Err(InvalidState)`.
- E8 `delete_from_document_outside_document_tree_does_nothing` (NEW): a selection (shadow Text, 0)-(shadow Text, 3)
  → `None`, the Text's data unchanged, the selection's range unchanged; control: the same over a light Text in
  `doc` → `Some(records)` with one characterData record, data spliced; empty selection → `None`.

**VM end-to-end, `crates/script/elidex-js/src/vm/tests/tests_selection_preconditions.rs` (NEW)** (§6's tree):

- J1 `cross_root_set_base_and_extent_is_collapsed_and_empty` (NEW): 1a and 1b → `isCollapsed`, `type` "Caret",
  `toString()` `""`. Green on `tsel`'s base and after `tq` (the value `tq`'s cell expects).
- J2 `cross_root_extend_collapses_at_focus` (NEW): 1c and 1d → `anchorNode === st && anchorOffset === 3 &&
  focusNode === st && focusOffset === 3`, `toString()` `""`.
- J3 `cross_root_delete_from_document_removes_nothing` (NEW): 1e and 1f → `pre.data === "pre"`, `st.data ===
  "SHADOW"`, `se.isConnected`.
- J4 `select_all_children_outside_document_tree_is_noop` (NEW): 2a, 2b, 2c → no throw, `rangeCount` 0.
- J5 `add_range_outside_document_tree_is_noop` (NEW): 2e, 2f → no throw, `rangeCount` 0.
- J6 `setters_outside_document_abort_without_throw` (NEW): 3a, 3c, 3e (caret stays at (`pre`, 1)), 3d (no throw),
  and 3h's four abort cases → no throw, selection unchanged.
- J7 `aborted_setter_schedules_no_selectionchange` (NEW): one aborted call per method (3a, 3c, 3e, 2a, 2e) → the
  event count stays 0 across the drain; control: `collapse(pre, 1)` → 1.
- J8 `setter_step_order_exceptions` (NEW): `collapse(ft, 99)` → "IndexSizeError" (base: "WrongDocumentError");
  3f, 3g → "IndexSizeError"; `extend(pre, 1)` on empty → "InvalidStateError".
- J9 `shadow_tree_selection_is_accepted` (NEW): 3b → `anchorNode === st`; `setBaseAndExtent(st,0, st,3)` →
  `toString()` `"SHA"`.
- J10 `delete_from_document_inside_shadow_tree_does_nothing` (NEW): 4a → `st.data === "SHADOW"`, the selection
  still (`st`, 0)-(`st`, 3), no `MutationRecord` for a `MutationObserver` on `sr` with `{characterData: true,
  subtree: true}`, and no `selectionchange` from the `deleteFromDocument()` call (counted across a drain that
  follows the setter's own event).

**Mutants** (each run once against the final head, ≤2 concurrent cargo jobs; each must turn at least the listed
cells red):

| # | Mutant | Red |
|---|---|---|
| M1 | `extend`: raw `set_start` / `set_end` back, step 5 branch removed | E2, J2 |
| M2 | `extend`: step 5 branch removed, `set_*_to_boundary` kept | E2 (backward), J2 (1d) |
| M3 | `set_base_and_extent`: raw writes back | E1, J1, J3 |
| M4 | `in_document_shadow_including` → node-document equality (base `same_document`) | E5 (detached), J6 (3a, 3c, 3e) |
| M5 | `in_document_shadow_including` → `find_tree_root` equality | E6, J9 |
| M6 | `root_is_document` → `find_tree_root_composed` equality | E3 (shadow), E4 (shadow), E8, J4 (2a, 2b), J5 (2e), J10 |
| M7 | the abort returns `Err(InvalidState)` instead of `Ok(false)` | E5, J6 |
| M8 | `mutate_selection` sets `selectionchange_pending` on `Ok(false)` | J7 |
| M9 | `collapse`: the document check before the offset check | E7, J8 (`collapse(ft, 99)`) |
| M10 | `extend`: the empty check before the document check | E7 (D5), J6 (3d) |
| M11 | `set_base_and_extent`: DocumentType check before the offset check | E7 (D3) |
| M12 | `delete_from_document`: the document-tree check removed | E8, J10 |
| M13 | `delete_selection_contents` sets `selectionchange_pending` on `None` | J10 (event) |

`add_range` with its two aborts swapped is an equivalent mutant (both abort, nothing observable differs); no cell
is claimed for it. A mutant that checks only the anchor (not the focus) in `delete_from_document` is equivalent
under SEL × ROOT (both points share one root after `tsel`); no cell is claimed for it.

### §7.1 1000-line re-count at `3cbd5b88`

`selection_tests.rs` 367 (+≈290 → ≈660); `tests_selection.rs` 787: the VM cells (≈330 lines) would take it past
1000, so they go in the new file (CLAUDE.md *1000-line debt*); it gets only the fixture edit (+1 line).
`selection.rs` 677 (+≈50); `dom_selection_mutation.rs` 320; `dom_selection_proto.rs` 748; `vm/tests/mod.rs` 215.
No file crosses 1000; no prerequisite split.

### §7.2 Existing tests that pin today's behaviour (expectations re-derived from the spec)

- **The VM fixture.** `TREE_SETUP` (`tests_selection.rs:35-42`) builds a detached `root`, and its comment says
  Selection "works equally on detached subtrees because `same_document` checks `find_tree_root`" (already untrue:
  it checks the node document). 28 of its tests call a setter on that tree (`collapse_sets_caret_directionless`,
  `set_position_is_alias_of_collapse`, `collapse_offset_defaults_to_zero`, `collapse_index_size_throws`,
  `collapse_to_start_drops_to_caret`, `collapse_to_end_drops_to_caret_at_end`, `select_all_children_covers_full_node`,
  `extend_forward_sets_forward_direction`, `extend_backward_sets_backward_direction`,
  `extend_collapsed_back_to_anchor_is_directionless`, `extend_throws_without_initial_range`,
  `set_base_and_extent_forward`, `set_base_and_extent_backward_when_anchor_after_focus`,
  `set_base_and_extent_collapsed_is_directionless`, `add_range_sets_when_empty`, `add_range_noop_when_already_set`,
  `get_range_at_zero_returns_current_range`, `remove_all_ranges_empties`, `empty_is_alias_of_remove_all_ranges`,
  `remove_range_with_current_range_clears`, `remove_range_with_non_current_throws`,
  `contains_node_full_contain_default`, `contains_node_partial_overlap_with_flag`,
  `previous_range_survives_after_collapse`, `selectionchange_fires_after_mutation`,
  `collapse_loop_without_wrapper_does_not_leak_registry`, `previous_range_survives_collapse_when_wrapper_held`,
  `selectionchange_coalesces_multiple_mutations`; counted by a script that splits the file at `#[test]` and
  reports, per test, whether it uses `TREE_SETUP` and which setters it calls). On a detached tree every one of
  them aborts after `tsel`. The fixture gains `document.appendChild(root);` and its comment is rewritten; with
  `root` as the document element every node of the tree passes every precondition (§3), so each test's
  expectation is the spec's value unchanged.
- **dom-api.** `collapse_rejects_cross_document` pins `Err(WrongDocument)`; the spec aborts, so it becomes
  `collapse_cross_document_aborts` (`Ok(false)`, state empty). `add_range_no_op_when_range_set`,
  `add_range_rejected_cross_document` and `add_range_sets_when_empty` change call shape (registry, DOM, document);
  their expectations hold (the third range is in `doc`; the second's is in another document: abort). The other
  setter cells build their nodes under `doc` and keep their values. `select_all_children_makes_range` asserts
  `Directionless`, which is out of scope (§0.2, N4) and is left as it is.
- **`deleteFromDocument`.** `tests_mutation_observer/range_ops.rs`'s two Selection cells
  (`selection_delete_from_document_delivers_removal_records`, `:167`;
  `selection_delete_from_document_intra_text_delivers_character_data_record`, `:558`) build under `body` through
  `build_root_tree`, so both points are in the document tree: `deleteContents()` still runs and their records are
  the spec's. Unchanged.

## §8. Landing items (handed to the PM with the slice; each is "old → new")

- **L-1** `#11-selection-boundary-setter-skips-root-check`: "**open (registered 2026-11-21, PM 884; scope widened
  by PM 885; L3 DOM/form)**" → "**closed** by `tsel` (PR #<n>, `<sha>`, <date>)", with: "Closed in
  `crates/dom/elidex-dom-api/src/selection.rs`: `SelectionState::extend` (step 5 branch) and
  `SelectionState::set_base_and_extent` write through `Range::set_start_to_boundary` / `set_end_to_boundary`;
  `SelectionState::select_all_children` and `SelectionState::add_range` abort unless the root is the document
  (`root_is_document`); `collapse` / `extend` / `set_base_and_extent` abort unless the document is a
  shadow-including inclusive ancestor (`in_document_shadow_including`), in the spec's step order;
  `SelectionState::delete_from_document` does nothing unless anchor and focus are in the document tree (PM 890);
  `SelectionError::WrongDocument` deleted; `mutate_selection` and `delete_selection_contents`
  (`crates/script/elidex-js/src/vm/host/dom_selection_mutation.rs`) schedule `selectionchange` only when the
  operation did something." The row's "no VM change" is corrected to the marshalling edits (§11 P-2), and its
  "scope widened by PM 885" gains "and PM 890 (`deleteFromDocument`)".
- **L-2** the umbrella's §5.1 order (already in the plan PR, §0.4): nothing to apply at landing; the `tq` row's
  Deps gains "tsel" at `tq`'s own landing list if the PM wants the table to carry it.
- **L-3** to `ta`'s plan (hand-off, not a ledger edit): census C-6 row `find_tree_root_composed` "2 callers
  (verified 2026-11-18)" → 3 (`selection.rs`, `in_document_shadow_including`); row `find_tree_root` "24 … lines"
  → 25 (`root_is_document`, plus `extend`'s step 5 if written as a direct call). `ta` re-measures at its base.
- **L-4** if Q-2 folds `containsNode` in: `same_document` deleted, `contains_node` on `root_is_document`; cells
  `contains_node_root_not_document_is_false` (NEW) in `selection_tests.rs` and in the new VM file (N2's shape →
  `false`), mutant "restore `same_document`".
- **L-5** `#11-selection-document-membership` (folded, PM 890): "**open (registered 2026-11-05 at C0a's landing
  #595 `4893df0f`, C0 program R-6 (c); was a pointer row to #583 §8)**" → "**closed**: merged into
  `#11-selection-boundary-setter-skips-root-check`, fixed by `tsel` (<sha>, <date>)". Its defect ("Selection
  methods accept a detached node whose node document is the selection's document") is facet (3); its open question
  ("the Selection API's check is unverified against its step text") is answered by §3's quotes from webref's cache.
  Nothing in the row lies outside `tsel`'s scope. Cites of the old id on `origin/main` (`git grep -n
  'selection-document-membership' origin/main -- docs`, 2026-11-21, 7 hits):
  `docs/plans/2026-10-eh-attrs-c0-insertion-adoption.md:671`, `:720`, `:731`, `:797`;
  `docs/plans/2026-11-range-contents-program.md:334`, `:1911`, `:2263`. The closed row's pointer keeps each cite
  valid, so no memo is edited. Several call the id "proposed" / "a proposal, not a registered slot" (`:671`,
  `:720`, `:797`; `range-contents-program.md:334`, `:1911`, `:2263`), which is stale (registered 2026-11-05); this
  plan PR touches neither memo, so their wording is left for the next PR that does.

## §9. Gate

The slice intersects SEL, ROOT, DOC, ERR, MUT and LRR (§2: ≥3 axes), so the *Edge-dense* rule applies: this per-PR
plan passes `/elidex-plan-review` in full, including the two ratified-surface lines of §0.4, before any code. Then
one PR, through `/pre-push`, cut after this plan's landing. It lands before `tq` (umbrella §5.1 erratum); `tq`'s
P-5 is a precondition on this landing.

## §10. Definition of done

1. §4.1 and §4.2 are implemented; `SelectionError::WrongDocument` and its mapping arm are gone; no algorithm in
   `vm/host/`.
2. E1-E8 and J1-J10 are green; the §7.2 re-derivations are applied; every mutant of §7 turns its cells red (run
   once on the final head).
3. `cargo test -p elidex-dom-api --all-features` and `cargo test -p elidex-js --all-features` pass; `mise run ci`
   passes before push.
4. No file over 1000 lines (§7.1 re-counted at the PR's head).
5. The landing items of §8 are handed to the PM.

## §11. Premises found wrong or not foreseen

- **P-1** The slot draft's path `handoff/pending-at-landing/selection-boundary-slot.md` no longer exists; it was
  applied and renamed `selection-boundary-slot.DONE.md`. The ledger row (`project_open-defer-slots.md`) holds the
  registered text, which also lists `collapse` and `select_all_children` raw writes as sites.
- **P-2** "No VM change" (slot draft, *Home of the fix*; ledger row): false. The abort must not throw and an
  operation that does nothing must not schedule `selectionchange`, so `mutate_selection`,
  `delete_selection_contents` and `map_selection_error` change, and `addRange`'s native stops reading
  `owner_document` (§4.2). Marshalling only; no algorithm moves.
- **P-3** "Not measured: whether the native throws on it" (slot draft (ii)): measured. A foreign-document node
  throws "WrongDocumentError" on all five calls (3h); a detached node is accepted and becomes the selection (3a,
  3c, 3e).
- **P-4** "the Selection setters take the existing `set_*_to_boundary`" (slot draft, *Home of the fix*): necessary
  for `setBaseAndExtent`, not sufficient for `extend`. In the backward branch the setters collapse at oldAnchor;
  step 5 collapses at newFocus, so `extend` needs its explicit root branch (§4.1, cell E2, mutant M2).
- **P-5** The spec value "the range collapses at (`st`, 3)" for `setBaseAndExtent(pre, 1, st, 3)` (the `tq` plan
  §11 P-5 at `b88504fd`): the collapse is determinate, the point is not. DOM's position algorithm asserts one root,
  so step 5's "If anchor is before focus" has no spec answer for a cross-root pair (Q-1). `""` and "nothing
  removed" hold on either branch, so `tq`'s cells are not affected.
- **P-6** "find where the umbrella keeps its changelog" (the driver's addition): the umbrella has no changelog
  line; its revision record is inside the Status block. The line was added as a new `**Changelog.**` line after
  the Status block, which is unchanged (§0.4).
- **P-7** `select_all_children`'s raw writes (`:439-440`, ledger row *Site*) are not a defect: steps 4-5 say "Set
  newRange's start", not "set the start". The facet there is step 2 only.
- **P-8** The VM selection fixture (`TREE_SETUP`) is detached, and its comment's reason is stale (§7.2): 28 cells
  depend on today's lax check. Not in the brief or the slot draft.
- **P-9** The brief calls the umbrella "ratified"; its Status block reads "umbrella, draft 3.5 (pre-implementation)"
  and records no ratification. The ratification is the PM's record, not checked here.
- **P-10** PM 885's list of methods omits `addRange()` step 1, the same "root is the document" precondition on a
  boundary setter (probes 2e, 2f). Included by the lens of §0.5; the PM confirms at plan-review.
- **P-11** PM 890 (verified): `SelectionState::delete_from_document` (`:513-546`) has no document-tree check, only
  the emptiness check at `:518`; its native has none either and schedules `selectionchange` unconditionally
  (`dom_selection_mutation.rs:147`). The function also takes no document today, so it gains one (§4.1).

## §12. Open questions

- **Q-1** (spec gap; not asserted by any cell) `setBaseAndExtent()` step 5 and steps 7 / 9 of
  `setBaseAndExtent()` / `extend()` ask "before" of a pair that can be cross-root, and DOM §5.2 asserts one root.
  The plan keeps `direction_between` as the branch selector (no new order rule), so a cross-root
  `setBaseAndExtent` collapses at the point the chosen branch writes last (with base `compare_points`: the later
  of the two in shadow-including order). The cells assert only what the spec fixes (collapsed, `""`, nothing
  removed). An issue against the Selection API is the PM's call.
- **Q-2** (PM, at plan-review) Does `containsNode()`'s root clause join this slice? For: one predicate, `same_document`
  deleted (one issue, one way), and N2 is a wrong `true` today. Against: it is a read outside PM 890's list. The
  cells are ready (§8 L-4).
- **Q-3** (PM; report only) Candidate slots for §0.2's defects, none decided here: `getRangeAt()`'s document-tree
  clause (N5; the predicate is facet (4)'s `root_is_document`, so folding it in would be one more call); Selection
  direction (N4, and `current_direction` on a collapsed range); `selectAllChildren`'s child count;
  `collapse(null)` (N1); `selectionchange` after a collapsed `deleteContents()`.
